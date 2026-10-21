//! Deterministic image-to-3D sculpting.
//!
//! The sculptor converts a single user image into a closed, textured 3D relief
//! mesh using silhouette inflation:
//!
//! 1. Segment the subject: use the alpha channel when the image has real
//!    transparency, otherwise flood-fill the background from the image border
//!    by color distance.
//! 2. Keep the largest connected subject region and fill interior holes.
//! 3. Compute a chamfer distance transform inside the silhouette and map it
//!    through a circular profile, so thick regions bulge more than thin ones.
//! 4. Emit a front surface, a shallower back surface, and shared boundary
//!    vertices that seal the two into one watertight shell, textured with the
//!    original image.
//!
//! It also clusters the subject's colors and derives a [`CharacterSpec`] so the
//! same image can drive a fully rigged procedural character.
//!
//! This is geometric reconstruction from one view, not a learned model: depth
//! is inferred from silhouette thickness, not from shading or semantics.

use std::{collections::VecDeque, io::Cursor, path::Path};

use image::{imageops::FilterType, ImageFormat, ImageReader, Rgb, RgbImage, Rgba, RgbaImage};
use serde::Serialize;
use serde_json::json;
use thiserror::Error;

use crate::{
    glb::{write_textured_glb, GlbError, MeshData},
    spec::{
        hex_color, BodyType, CharacterSpec, Ears, EyeStyle, HairStyle, Outfit, Palette,
        Proportions, SpecError, Tail, Wings, SPEC_SCHEMA_VERSION,
    },
};

pub const DEFAULT_MAX_IMAGE_BYTES: u64 = 20 * 1024 * 1024;
pub const DEFAULT_MAX_IMAGE_DIMENSION: u32 = 4096;
pub const DEFAULT_MAX_DECODE_ALLOC: u64 = 128 * 1024 * 1024;
/// Longest side of the sculpting grid. 128 keeps the mesh under ~66k triangles.
pub const DEFAULT_GRID_RESOLUTION: u32 = 128;
pub const DEFAULT_TEXTURE_RESOLUTION: u32 = 1024;
pub const PREVIEW_RESOLUTION: u32 = 256;
const MIN_GRID_SIDE: u32 = 8;
/// Fraction of grid pixels that must be translucent before alpha is trusted.
const ALPHA_SEGMENTATION_MIN_FRACTION: f32 = 0.02;
const ALPHA_OPAQUE_THRESHOLD: u8 = 128;
const ALPHA_TRANSLUCENT_THRESHOLD: u8 = 200;
const MIN_SUBJECT_COVERAGE: f32 = 0.03;
const MAX_SUBJECT_COVERAGE: f32 = 0.97;
/// Border color spread (RGB distance, 90th percentile) above which the
/// background is considered too busy to separate reliably.
const MAX_BORDER_SPREAD: f32 = 60.0;
const BORDER_THRESHOLD_MIN: f32 = 20.0;
const BORDER_THRESHOLD_MAX: f32 = 70.0;
/// Front bulge height as a fraction of the inscribed radius.
const FRONT_DEPTH: f32 = 0.6;
/// Back surface depth relative to the front.
const BACK_DEPTH_RATIO: f32 = 0.7;
const HEIGHT_SMOOTHING_PASSES: usize = 2;
const TEXTURE_BLEED_PASSES: usize = 6;
const PALETTE_CLUSTERS: usize = 5;
const PALETTE_ITERATIONS: usize = 12;
const PALETTE_MAX_SAMPLES: usize = 20_000;
const PALETTE_MIN_WEIGHT: f32 = 0.02;
/// Share of the top band a non-skin color needs before it is treated as hair.
const HAIR_MIN_TOP_SHARE: f32 = 0.2;
const DEFAULT_SKIN: [u8; 3] = [0xf2, 0xc9, 0xa8];

#[derive(Debug, Error)]
pub enum SculptError {
    #[error("image file could not be read: {0}")]
    Io(#[from] std::io::Error),
    #[error("image must be a regular file between 1 byte and {0} bytes")]
    FileSize(u64),
    #[error("image could not be decoded: {0}")]
    Decode(String),
    #[error("could not find a subject in the image: {0}")]
    NoSubject(String),
    #[error("generated mesh is invalid: {0}")]
    Mesh(#[from] GlbError),
    #[error("generated character spec is invalid: {0}")]
    Spec(#[from] SpecError),
}

#[derive(Debug, Clone, Copy)]
pub struct ImageSculptLimits {
    pub max_file_bytes: u64,
    pub max_dimension: u32,
    pub max_decode_alloc: u64,
    pub grid_resolution: u32,
    pub texture_resolution: u32,
}

impl Default for ImageSculptLimits {
    fn default() -> Self {
        Self {
            max_file_bytes: DEFAULT_MAX_IMAGE_BYTES,
            max_dimension: DEFAULT_MAX_IMAGE_DIMENSION,
            max_decode_alloc: DEFAULT_MAX_DECODE_ALLOC,
            grid_resolution: DEFAULT_GRID_RESOLUTION,
            texture_resolution: DEFAULT_TEXTURE_RESOLUTION,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Segmentation {
    Alpha,
    BorderColor,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SculptStats {
    pub source_width: u32,
    pub source_height: u32,
    pub grid_width: u32,
    pub grid_height: u32,
    pub vertex_count: usize,
    pub triangle_count: usize,
    pub subject_coverage: f32,
    pub segmentation: Segmentation,
}

#[derive(Debug, Clone)]
pub struct SculptOutput {
    /// Self-contained binary glTF of the relief mesh.
    pub glb: Vec<u8>,
    /// Transparent cut-out preview PNG of the segmented subject.
    pub preview_png: Vec<u8>,
    /// Procedural character derived from the subject's palette.
    pub spec: CharacterSpec,
    pub stats: SculptStats,
}

/// Sculpts `image_path` into a relief mesh and a palette-derived spec.
///
/// `style_id` becomes the id of the derived [`CharacterSpec`].
pub fn sculpt_image(
    image_path: &Path,
    style_id: &str,
    name: &str,
    limits: ImageSculptLimits,
) -> Result<SculptOutput, SculptError> {
    let metadata = std::fs::symlink_metadata(image_path)?;
    if !metadata.file_type().is_file()
        || metadata.len() == 0
        || metadata.len() > limits.max_file_bytes
    {
        return Err(SculptError::FileSize(limits.max_file_bytes));
    }
    let mut reader = ImageReader::open(image_path)?
        .with_guessed_format()
        .map_err(|error| SculptError::Decode(error.to_string()))?;
    let mut decode_limits = image::Limits::default();
    decode_limits.max_image_width = Some(limits.max_dimension);
    decode_limits.max_image_height = Some(limits.max_dimension);
    decode_limits.max_alloc = Some(limits.max_decode_alloc);
    reader.limits(decode_limits);
    let source = reader
        .decode()
        .map_err(|error| SculptError::Decode(error.to_string()))?
        .to_rgba8();
    sculpt_rgba(&source, style_id, name, limits)
}

/// Sculpts an already decoded image. Exposed for callers that decode
/// themselves and for tests.
pub fn sculpt_rgba(
    source: &RgbaImage,
    style_id: &str,
    name: &str,
    limits: ImageSculptLimits,
) -> Result<SculptOutput, SculptError> {
    let (source_width, source_height) = source.dimensions();
    if source_width < MIN_GRID_SIDE || source_height < MIN_GRID_SIDE {
        return Err(SculptError::NoSubject(format!(
            "image must be at least {MIN_GRID_SIDE}x{MIN_GRID_SIDE} pixels"
        )));
    }
    let (grid_width, grid_height) = fit_within(
        source_width,
        source_height,
        limits.grid_resolution.max(MIN_GRID_SIDE),
    );
    let small = image::imageops::resize(source, grid_width, grid_height, FilterType::Triangle);

    let (mut mask, segmentation) = segment(&small)?;
    keep_largest_region(&mut mask, grid_width, grid_height);
    fill_holes(&mut mask, grid_width, grid_height);
    let subject = mask.iter().filter(|&&inside| inside).count();
    let coverage = subject as f32 / mask.len() as f32;
    if !(MIN_SUBJECT_COVERAGE..=MAX_SUBJECT_COVERAGE).contains(&coverage) {
        return Err(SculptError::NoSubject(format!(
            "subject covers {:.0}% of the image; expected {:.0}%-{:.0}%. Use a PNG with a transparent background or a plain backdrop",
            coverage * 100.0,
            MIN_SUBJECT_COVERAGE * 100.0,
            MAX_SUBJECT_COVERAGE * 100.0
        )));
    }

    let relief = Relief::build(&mask, grid_width, grid_height);
    let mesh = relief.mesh();
    let texture = build_texture(
        source,
        &mask,
        grid_width,
        grid_height,
        limits.texture_resolution,
    );
    let texture_png = encode_png_rgb(&texture)?;
    let glb = write_textured_glb(
        &mesh,
        &texture_png,
        name,
        json!({ "agentpet": { "kind": "image-sculpt", "version": 1 } }),
    )?;
    let preview_png = encode_png_rgba(&build_preview(source, &mask, grid_width, grid_height))?;
    let spec = derive_spec(&small, &mask, grid_width, grid_height, style_id, name)?;

    Ok(SculptOutput {
        glb,
        preview_png,
        spec,
        stats: SculptStats {
            source_width,
            source_height,
            grid_width,
            grid_height,
            vertex_count: mesh.positions.len(),
            triangle_count: mesh.triangle_count(),
            subject_coverage: coverage,
            segmentation,
        },
    })
}

fn fit_within(width: u32, height: u32, longest: u32) -> (u32, u32) {
    let scale = longest as f32 / width.max(height) as f32;
    let scale = scale.min(1.0);
    (
        ((width as f32 * scale).round() as u32).max(MIN_GRID_SIDE),
        ((height as f32 * scale).round() as u32).max(MIN_GRID_SIDE),
    )
}

fn color_distance(a: [u8; 3], b: [u8; 3]) -> f32 {
    let dr = a[0] as f32 - b[0] as f32;
    let dg = a[1] as f32 - b[1] as f32;
    let db = a[2] as f32 - b[2] as f32;
    (dr * dr + dg * dg + db * db).sqrt()
}

fn rgb(pixel: &Rgba<u8>) -> [u8; 3] {
    [pixel[0], pixel[1], pixel[2]]
}

fn segment(image: &RgbaImage) -> Result<(Vec<bool>, Segmentation), SculptError> {
    let (width, height) = image.dimensions();
    let translucent = image
        .pixels()
        .filter(|pixel| pixel[3] < ALPHA_TRANSLUCENT_THRESHOLD)
        .count();
    if translucent as f32 / (width * height) as f32 >= ALPHA_SEGMENTATION_MIN_FRACTION {
        let mask = image
            .pixels()
            .map(|pixel| pixel[3] >= ALPHA_OPAQUE_THRESHOLD)
            .collect();
        return Ok((mask, Segmentation::Alpha));
    }

    let mut border = Vec::new();
    for x in 0..width {
        border.push((x, 0));
        border.push((x, height - 1));
    }
    for y in 1..height - 1 {
        border.push((0, y));
        border.push((width - 1, y));
    }
    let colors: Vec<[u8; 3]> = border
        .iter()
        .map(|&(x, y)| rgb(image.get_pixel(x, y)))
        .collect();
    let median = |channel: usize| {
        let mut values: Vec<u8> = colors.iter().map(|color| color[channel]).collect();
        values.sort_unstable();
        values[values.len() / 2]
    };
    let background = [median(0), median(1), median(2)];
    let mut spread: Vec<f32> = colors
        .iter()
        .map(|&color| color_distance(color, background))
        .collect();
    spread.sort_by(f32::total_cmp);
    let p90 = spread[(spread.len() * 9) / 10];
    if p90 > MAX_BORDER_SPREAD {
        return Err(SculptError::NoSubject(
            "the background is too busy to separate; use a PNG with a transparent background or a plain backdrop".into(),
        ));
    }
    let threshold = (p90 * 1.5 + 14.0).clamp(BORDER_THRESHOLD_MIN, BORDER_THRESHOLD_MAX);

    let index = |x: u32, y: u32| (y * width + x) as usize;
    let mut is_background = vec![false; (width * height) as usize];
    let mut queue = VecDeque::new();
    for &(x, y) in &border {
        if !is_background[index(x, y)]
            && color_distance(rgb(image.get_pixel(x, y)), background) < threshold
        {
            is_background[index(x, y)] = true;
            queue.push_back((x, y));
        }
    }
    while let Some((x, y)) = queue.pop_front() {
        for (nx, ny) in neighbors4(x, y, width, height) {
            let i = index(nx, ny);
            if !is_background[i]
                && color_distance(rgb(image.get_pixel(nx, ny)), background) < threshold
            {
                is_background[i] = true;
                queue.push_back((nx, ny));
            }
        }
    }
    Ok((
        is_background.into_iter().map(|bg| !bg).collect(),
        Segmentation::BorderColor,
    ))
}

fn neighbors4(x: u32, y: u32, width: u32, height: u32) -> impl Iterator<Item = (u32, u32)> {
    let candidates = [
        (x.wrapping_sub(1), y),
        (x + 1, y),
        (x, y.wrapping_sub(1)),
        (x, y + 1),
    ];
    candidates
        .into_iter()
        .filter(move |&(nx, ny)| nx < width && ny < height)
}

/// Labels 4-connected regions of `value` and returns (labels, sizes).
fn label_regions(mask: &[bool], value: bool, width: u32, height: u32) -> (Vec<u32>, Vec<usize>) {
    const UNLABELED: u32 = u32::MAX;
    let mut labels = vec![UNLABELED; mask.len()];
    let mut sizes = Vec::new();
    let mut queue = VecDeque::new();
    for start in 0..mask.len() {
        if mask[start] != value || labels[start] != UNLABELED {
            continue;
        }
        let label = sizes.len() as u32;
        let mut size = 0;
        labels[start] = label;
        queue.push_back(start);
        while let Some(i) = queue.pop_front() {
            size += 1;
            let (x, y) = ((i as u32) % width, (i as u32) / width);
            for (nx, ny) in neighbors4(x, y, width, height) {
                let n = (ny * width + nx) as usize;
                if mask[n] == value && labels[n] == UNLABELED {
                    labels[n] = label;
                    queue.push_back(n);
                }
            }
        }
        sizes.push(size);
    }
    (labels, sizes)
}

fn keep_largest_region(mask: &mut [bool], width: u32, height: u32) {
    let (labels, sizes) = label_regions(mask, true, width, height);
    let Some(largest) = sizes
        .iter()
        .enumerate()
        .max_by_key(|&(_, &size)| size)
        .map(|(label, _)| label as u32)
    else {
        return;
    };
    for (inside, label) in mask.iter_mut().zip(labels) {
        if *inside && label != largest {
            *inside = false;
        }
    }
}

fn fill_holes(mask: &mut [bool], width: u32, height: u32) {
    let (labels, sizes) = label_regions(mask, false, width, height);
    let mut touches_border = vec![false; sizes.len()];
    for (i, &label) in labels.iter().enumerate() {
        if mask[i] {
            continue;
        }
        let (x, y) = ((i as u32) % width, (i as u32) / width);
        if x == 0 || y == 0 || x == width - 1 || y == height - 1 {
            touches_border[label as usize] = true;
        }
    }
    for (i, &label) in labels.iter().enumerate() {
        if !mask[i] && !touches_border[label as usize] {
            mask[i] = true;
        }
    }
}

/// Inflated height field on a grid padded by one empty pixel on every side.
struct Relief {
    width: usize,
    height: usize,
    inside: Vec<bool>,
    boundary: Vec<bool>,
    heights: Vec<f32>,
    grid_width: u32,
    grid_height: u32,
}

impl Relief {
    fn build(mask: &[bool], grid_width: u32, grid_height: u32) -> Self {
        let width = grid_width as usize + 2;
        let height = grid_height as usize + 2;
        let mut inside = vec![false; width * height];
        for y in 0..grid_height as usize {
            for x in 0..grid_width as usize {
                inside[(y + 1) * width + x + 1] = mask[y * grid_width as usize + x];
            }
        }

        // Two-pass 3-4 chamfer distance transform (distance to background).
        const ORTHOGONAL: u32 = 3;
        const DIAGONAL: u32 = 4;
        let mut distance: Vec<u32> = inside
            .iter()
            .map(|&value| if value { u32::MAX / 2 } else { 0 })
            .collect();
        for y in 1..height - 1 {
            for x in 1..width - 1 {
                let i = y * width + x;
                if !inside[i] {
                    continue;
                }
                let best = distance[i]
                    .min(distance[i - 1] + ORTHOGONAL)
                    .min(distance[i - width] + ORTHOGONAL)
                    .min(distance[i - width - 1] + DIAGONAL)
                    .min(distance[i - width + 1] + DIAGONAL);
                distance[i] = best;
            }
        }
        for y in (1..height - 1).rev() {
            for x in (1..width - 1).rev() {
                let i = y * width + x;
                if !inside[i] {
                    continue;
                }
                let best = distance[i]
                    .min(distance[i + 1] + ORTHOGONAL)
                    .min(distance[i + width] + ORTHOGONAL)
                    .min(distance[i + width + 1] + DIAGONAL)
                    .min(distance[i + width - 1] + DIAGONAL);
                distance[i] = best;
            }
        }
        let radius = distance
            .iter()
            .copied()
            .max()
            .map_or(1.0, |d| d as f32 / ORTHOGONAL as f32)
            .max(1.0);

        let boundary: Vec<bool> = (0..width * height)
            .map(|i| {
                inside[i]
                    && [i - 1, i + 1, i - width, i + width]
                        .iter()
                        .any(|&n| !inside[n])
            })
            .collect();
        let mut heights: Vec<f32> = (0..width * height)
            .map(|i| {
                if !inside[i] || boundary[i] {
                    return 0.0;
                }
                let t = (distance[i] as f32 / ORTHOGONAL as f32 / radius).clamp(0.0, 1.0);
                let profile = (1.0 - (1.0 - t) * (1.0 - t)).sqrt();
                profile * radius * FRONT_DEPTH
            })
            .collect();
        for _ in 0..HEIGHT_SMOOTHING_PASSES {
            let previous = heights.clone();
            for y in 1..height - 1 {
                for x in 1..width - 1 {
                    let i = y * width + x;
                    if !inside[i] || boundary[i] {
                        continue;
                    }
                    let mut sum = 0.0;
                    for dy in [-1isize, 0, 1] {
                        for dx in [-1isize, 0, 1] {
                            let n = (i as isize + dy * width as isize + dx) as usize;
                            sum += previous[n];
                        }
                    }
                    heights[i] = sum / 9.0;
                }
            }
        }
        Self {
            width,
            height,
            inside,
            boundary,
            heights,
            grid_width,
            grid_height,
        }
    }

    fn mesh(&self) -> MeshData {
        let (width, height) = (self.width, self.height);
        let (mut min_x, mut max_x, mut min_y, mut max_y) = (usize::MAX, 0, usize::MAX, 0);
        for y in 0..height {
            for x in 0..width {
                if self.inside[y * width + x] {
                    min_x = min_x.min(x);
                    max_x = max_x.max(x);
                    min_y = min_y.min(y);
                    max_y = max_y.max(y);
                }
            }
        }
        let scale = 1.0 / ((max_y - min_y).max(1) as f32);
        let center_x = (min_x + max_x) as f32 / 2.0;

        let mut mesh = MeshData::default();
        let mut front = vec![u32::MAX; width * height];
        let mut back = vec![u32::MAX; width * height];
        for y in 0..height {
            for x in 0..width {
                let i = y * width + x;
                if !self.inside[i] {
                    continue;
                }
                let px = (x as f32 - center_x) * scale;
                let py = (max_y - y) as f32 * scale;
                let uv = [
                    (x as f32 - 0.5) / self.grid_width as f32,
                    (y as f32 - 0.5) / self.grid_height as f32,
                ];
                let depth = self.heights[i] * scale;
                front[i] = mesh.positions.len() as u32;
                mesh.positions.push([px, py, depth]);
                mesh.uvs.push(uv);
                if self.boundary[i] {
                    back[i] = front[i];
                } else {
                    back[i] = mesh.positions.len() as u32;
                    mesh.positions.push([px, py, -depth * BACK_DEPTH_RATIO]);
                    mesh.uvs.push(uv);
                }
            }
        }

        for y in 0..height - 1 {
            for x in 0..width - 1 {
                let a = y * width + x;
                let b = a + 1;
                let c = a + width;
                let d = c + 1;
                let corners = [a, b, c, d].map(|i| self.inside[i]);
                // Counter-clockwise when viewed from +Z (image rows grow downward).
                let triangles: &[[usize; 3]] = match corners {
                    [true, true, true, true] => &[[a, c, b], [b, c, d]],
                    [true, true, true, false] => &[[a, c, b]],
                    [false, true, true, true] => &[[b, c, d]],
                    [true, false, true, true] => &[[a, c, d]],
                    [true, true, false, true] => &[[a, d, b]],
                    _ => &[],
                };
                for &[p, q, r] in triangles {
                    mesh.indices
                        .extend_from_slice(&[front[p], front[q], front[r]]);
                    mesh.indices.extend_from_slice(&[back[p], back[r], back[q]]);
                }
            }
        }

        mesh.normals = vec![[0.0; 3]; mesh.positions.len()];
        for triangle in mesh.indices.chunks_exact(3) {
            let [p0, p1, p2] = [0, 1, 2].map(|k| mesh.positions[triangle[k] as usize]);
            let e1 = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
            let e2 = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];
            let normal = [
                e1[1] * e2[2] - e1[2] * e2[1],
                e1[2] * e2[0] - e1[0] * e2[2],
                e1[0] * e2[1] - e1[1] * e2[0],
            ];
            for &vertex in triangle {
                let accumulated = &mut mesh.normals[vertex as usize];
                for axis in 0..3 {
                    accumulated[axis] += normal[axis];
                }
            }
        }
        for normal in &mut mesh.normals {
            let length =
                (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
            *normal = if length > f32::EPSILON {
                [normal[0] / length, normal[1] / length, normal[2] / length]
            } else {
                [0.0, 0.0, 1.0]
            };
        }
        mesh
    }
}

fn grid_mask_at(mask: &[bool], grid_width: u32, grid_height: u32, u: f32, v: f32) -> bool {
    let x = ((u * grid_width as f32) as u32).min(grid_width - 1);
    let y = ((v * grid_height as f32) as u32).min(grid_height - 1);
    mask[(y * grid_width + x) as usize]
}

/// Builds the opaque texture. Colors outside the subject are bled outward
/// from the silhouette so bilinear sampling at the edge never picks up the
/// original background.
fn build_texture(
    source: &RgbaImage,
    mask: &[bool],
    grid_width: u32,
    grid_height: u32,
    resolution: u32,
) -> RgbImage {
    let (width, height) = fit_within(source.width(), source.height(), resolution);
    let resized = image::imageops::resize(source, width, height, FilterType::Triangle);
    let mut known: Vec<bool> = (0..width * height)
        .map(|i| {
            let (x, y) = (i % width, i / width);
            grid_mask_at(
                mask,
                grid_width,
                grid_height,
                (x as f32 + 0.5) / width as f32,
                (y as f32 + 0.5) / height as f32,
            )
        })
        .collect();
    let mut texture = RgbImage::from_fn(width, height, |x, y| {
        let pixel = resized.get_pixel(x, y);
        Rgb([pixel[0], pixel[1], pixel[2]])
    });
    for _ in 0..TEXTURE_BLEED_PASSES {
        let snapshot = texture.clone();
        let previous = known.clone();
        for y in 0..height {
            for x in 0..width {
                let i = (y * width + x) as usize;
                if previous[i] {
                    continue;
                }
                let mut sum = [0u32; 3];
                let mut count = 0;
                for (nx, ny) in neighbors4(x, y, width, height) {
                    if previous[(ny * width + nx) as usize] {
                        let pixel = snapshot.get_pixel(nx, ny);
                        for channel in 0..3 {
                            sum[channel] += pixel[channel] as u32;
                        }
                        count += 1;
                    }
                }
                if count > 0 {
                    texture.put_pixel(x, y, Rgb(sum.map(|value| (value / count) as u8)));
                    known[i] = true;
                }
            }
        }
    }
    texture
}

fn build_preview(
    source: &RgbaImage,
    mask: &[bool],
    grid_width: u32,
    grid_height: u32,
) -> RgbaImage {
    let (width, height) = fit_within(source.width(), source.height(), PREVIEW_RESOLUTION);
    let mut preview = image::imageops::resize(source, width, height, FilterType::Triangle);
    for (x, y, pixel) in preview.enumerate_pixels_mut() {
        let inside = grid_mask_at(
            mask,
            grid_width,
            grid_height,
            (x as f32 + 0.5) / width as f32,
            (y as f32 + 0.5) / height as f32,
        );
        if !inside {
            pixel[3] = 0;
        }
    }
    preview
}

fn encode_png_rgb(image: &RgbImage) -> Result<Vec<u8>, SculptError> {
    let mut bytes = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
        .map_err(|error| SculptError::Decode(error.to_string()))?;
    Ok(bytes)
}

fn encode_png_rgba(image: &RgbaImage) -> Result<Vec<u8>, SculptError> {
    let mut bytes = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
        .map_err(|error| SculptError::Decode(error.to_string()))?;
    Ok(bytes)
}

#[derive(Debug, Clone, Copy)]
struct Cluster {
    color: [f32; 3],
    weight: f32,
}

/// Deterministic k-means with farthest-point initialization.
fn cluster_colors(samples: &[[u8; 3]], k: usize) -> (Vec<Cluster>, Vec<usize>) {
    let to_f = |c: [u8; 3]| [c[0] as f32, c[1] as f32, c[2] as f32];
    let distance2 = |a: [f32; 3], b: [f32; 3]| {
        (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)
    };
    let mean = samples.iter().fold([0.0f32; 3], |acc, &c| {
        let c = to_f(c);
        [acc[0] + c[0], acc[1] + c[1], acc[2] + c[2]]
    });
    let n = samples.len().max(1) as f32;
    let mean = [mean[0] / n, mean[1] / n, mean[2] / n];
    let mut centers = vec![samples
        .iter()
        .map(|&c| to_f(c))
        .min_by(|a, b| distance2(*a, mean).total_cmp(&distance2(*b, mean)))
        .unwrap_or(mean)];
    while centers.len() < k {
        let farthest = samples
            .iter()
            .map(|&c| to_f(c))
            .max_by(|a, b| {
                let da = centers
                    .iter()
                    .map(|&m| distance2(*a, m))
                    .fold(f32::MAX, f32::min);
                let db = centers
                    .iter()
                    .map(|&m| distance2(*b, m))
                    .fold(f32::MAX, f32::min);
                da.total_cmp(&db)
            })
            .unwrap_or(mean);
        centers.push(farthest);
    }
    let mut assignment = vec![0usize; samples.len()];
    for _ in 0..PALETTE_ITERATIONS {
        for (slot, &sample) in assignment.iter_mut().zip(samples) {
            let color = to_f(sample);
            *slot = (0..centers.len())
                .min_by(|&a, &b| {
                    distance2(color, centers[a]).total_cmp(&distance2(color, centers[b]))
                })
                .unwrap_or(0);
        }
        let mut sums = vec![[0.0f32; 4]; centers.len()];
        for (&cluster, &sample) in assignment.iter().zip(samples) {
            let color = to_f(sample);
            let entry = &mut sums[cluster];
            entry[0] += color[0];
            entry[1] += color[1];
            entry[2] += color[2];
            entry[3] += 1.0;
        }
        for (center, sum) in centers.iter_mut().zip(&sums) {
            if sum[3] > 0.0 {
                *center = [sum[0] / sum[3], sum[1] / sum[3], sum[2] / sum[3]];
            }
        }
    }
    let mut counts = vec![0usize; centers.len()];
    for &cluster in &assignment {
        counts[cluster] += 1;
    }
    let clusters = centers
        .into_iter()
        .zip(counts)
        .map(|(color, count)| Cluster {
            color,
            weight: count as f32 / n,
        })
        .collect();
    (clusters, assignment)
}

fn to_rgb8(color: [f32; 3]) -> [u8; 3] {
    color.map(|channel| channel.round().clamp(0.0, 255.0) as u8)
}

/// Returns (hue degrees, saturation, value) for an RGB color.
fn hsv(color: [f32; 3]) -> (f32, f32, f32) {
    let [r, g, b] = color.map(|c| c / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let hue = if delta <= f32::EPSILON {
        0.0
    } else if (max - r).abs() <= f32::EPSILON {
        60.0 * ((g - b) / delta).rem_euclid(6.0)
    } else if (max - g).abs() <= f32::EPSILON {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    let saturation = if max <= f32::EPSILON {
        0.0
    } else {
        delta / max
    };
    (hue, saturation, max)
}

fn derive_spec(
    small: &RgbaImage,
    mask: &[bool],
    width: u32,
    height: u32,
    style_id: &str,
    name: &str,
) -> Result<CharacterSpec, SculptError> {
    let mut rows = Vec::new();
    let mut samples = Vec::new();
    for y in 0..height {
        for x in 0..width {
            if mask[(y * width + x) as usize] {
                rows.push(y);
                samples.push(rgb(small.get_pixel(x, y)));
            }
        }
    }
    let stride = samples.len().div_ceil(PALETTE_MAX_SAMPLES).max(1);
    let rows: Vec<u32> = rows.into_iter().step_by(stride).collect();
    let samples: Vec<[u8; 3]> = samples.into_iter().step_by(stride).collect();
    let (clusters, assignment) = cluster_colors(&samples, PALETTE_CLUSTERS.min(samples.len()));

    let top = rows.iter().copied().min().unwrap_or(0) as f32;
    let bottom = rows.iter().copied().max().unwrap_or(0) as f32;
    let span = (bottom - top).max(1.0);
    // Clusters ranked by pixel count inside a vertical band of the subject.
    let ranked_in = |from: f32, to: f32| -> Vec<(usize, usize)> {
        let mut counts = vec![0usize; clusters.len()];
        for (&row, &cluster) in rows.iter().zip(&assignment) {
            let t = (row as f32 - top) / span;
            if (from..=to).contains(&t) {
                counts[cluster] += 1;
            }
        }
        let mut ranked: Vec<(usize, usize)> = counts.into_iter().enumerate().collect();
        ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        ranked
    };
    let dominant_in =
        |from: f32, to: f32| ranked_in(from, to).first().map_or(0, |&(index, _)| index);
    let significant = || {
        clusters
            .iter()
            .enumerate()
            .filter(|(_, c)| c.weight >= PALETTE_MIN_WEIGHT)
    };
    let skin_index = significant()
        .filter(|(_, cluster)| {
            let (h, s, v) = hsv(cluster.color);
            (5.0..=50.0).contains(&h) && (0.15..=0.65).contains(&s) && v > 0.35
        })
        .max_by(|a, b| a.1.weight.total_cmp(&b.1.weight))
        .map(|(index, _)| index);
    let skin = skin_index.map_or(DEFAULT_SKIN, |index| to_rgb8(clusters[index].color));

    // The top of a portrait is often face as much as hair; prefer the
    // strongest non-skin color there when it covers a meaningful share.
    let top_band = ranked_in(0.0, 0.22);
    let top_total: usize = top_band.iter().map(|&(_, count)| count).sum();
    let hair_index = top_band
        .iter()
        .find(|&&(index, count)| {
            Some(index) != skin_index && count as f32 >= top_total as f32 * HAIR_MIN_TOP_SHARE
        })
        .or_else(|| top_band.first())
        .map_or(0, |&(index, _)| index);
    let hair = clusters[hair_index].color;
    let primary = clusters[dominant_in(0.3, 0.7)].color;
    let secondary = clusters[dominant_in(0.75, 1.0)].color;
    let accent = significant()
        .max_by(|a, b| {
            let (_, sa, va) = hsv(a.1.color);
            let (_, sb, vb) = hsv(b.1.color);
            (sa * va).total_cmp(&(sb * vb))
        })
        .map_or(primary, |(_, cluster)| cluster.color);

    let spec = CharacterSpec {
        schema_version: SPEC_SCHEMA_VERSION,
        id: style_id.to_owned(),
        name: name
            .trim()
            .chars()
            .take(crate::spec::MAX_NAME_CHARS)
            .collect(),
        title: "Styled from your image".into(),
        description: "A rigged character whose palette was sampled from an imported image.".into(),
        body: BodyType::Humanoid,
        ears: Ears::Human,
        tail: Tail::None,
        wings: Wings::None,
        hair: HairStyle::Bob,
        eyes: EyeStyle::Round,
        outfit: Outfit::Hoodie,
        accessories: Vec::new(),
        palette: Palette {
            skin: hex_color(skin),
            hair: hex_color(to_rgb8(hair)),
            eyes: hex_color(to_rgb8(accent)),
            primary: hex_color(to_rgb8(primary)),
            secondary: hex_color(to_rgb8(secondary)),
            accent: hex_color(to_rgb8(accent)),
        },
        proportions: Proportions::default(),
        energy: 1.0,
        tags: vec!["custom".into(), "from-image".into()],
    };
    spec.validate()?;
    Ok(spec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glb::read_glb;

    /// A filled disk of `color` centered on a plain background.
    fn disk_image(size: u32, background: Rgba<u8>, color: Rgba<u8>) -> RgbaImage {
        let center = size as f32 / 2.0;
        let radius = size as f32 * 0.3;
        RgbaImage::from_fn(size, size, |x, y| {
            let dx = x as f32 + 0.5 - center;
            let dy = y as f32 + 0.5 - center;
            if dx * dx + dy * dy <= radius * radius {
                color
            } else {
                background
            }
        })
    }

    #[test]
    fn sculpts_plain_background_subject_into_a_valid_closed_mesh() {
        let image = disk_image(200, Rgba([250, 250, 250, 255]), Rgba([200, 40, 60, 255]));
        let output = sculpt_rgba(
            &image,
            "user.style-test",
            "Disk",
            ImageSculptLimits::default(),
        )
        .expect("sculpts");
        let stats = &output.stats;
        assert_eq!(stats.segmentation, Segmentation::BorderColor);
        let expected = std::f32::consts::PI * 0.3 * 0.3;
        assert!(
            (stats.subject_coverage - expected).abs() < 0.03,
            "coverage {} expected about {expected}",
            stats.subject_coverage
        );
        assert!(stats.triangle_count > 1000 && stats.triangle_count < 100_000);

        let container = read_glb(&output.glb).expect("valid glb");
        let max = &container.document["accessors"][0]["max"];
        let min = &container.document["accessors"][0]["min"];
        let height = max[1].as_f64().unwrap() - min[1].as_f64().unwrap();
        assert!(
            (height - 1.0).abs() < 1e-3,
            "normalized height, got {height}"
        );
        assert!(
            max[2].as_f64().unwrap() > 0.15,
            "front bulges toward the viewer"
        );
        assert!(min[2].as_f64().unwrap() < -0.1, "back bulges away");

        // Every edge in a closed shell is shared by exactly two triangles.
        let indices = indices_from(&container);
        let mut edges = std::collections::HashMap::new();
        for triangle in indices.chunks_exact(3) {
            for (a, b) in [(0, 1), (1, 2), (2, 0)] {
                let key = (triangle[a].min(triangle[b]), triangle[a].max(triangle[b]));
                *edges.entry(key).or_insert(0u32) += 1;
            }
        }
        assert!(
            edges.values().all(|&count| count == 2),
            "mesh is watertight"
        );

        assert_eq!(output.spec.palette.primary, "#c8283c");
        output.spec.validate().expect("derived spec is valid");
        assert_eq!(&output.preview_png[1..4], b"PNG");
    }

    fn indices_from(container: &crate::glb::GlbContainer<'_>) -> Vec<u32> {
        let doc = &container.document;
        let view =
            &doc["bufferViews"][doc["accessors"][3]["bufferView"].as_u64().unwrap() as usize];
        let offset = view["byteOffset"].as_u64().unwrap() as usize;
        let length = view["byteLength"].as_u64().unwrap() as usize;
        container.binary[offset..offset + length]
            .chunks_exact(4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .collect()
    }

    #[test]
    fn uses_alpha_channel_when_present() {
        let image = disk_image(120, Rgba([0, 0, 0, 0]), Rgba([30, 120, 220, 255]));
        let output = sculpt_rgba(
            &image,
            "user.style-alpha",
            "Alpha",
            ImageSculptLimits::default(),
        )
        .expect("sculpts");
        assert_eq!(output.stats.segmentation, Segmentation::Alpha);
    }

    #[test]
    fn rejects_images_without_a_separable_subject() {
        let blank = RgbaImage::from_pixel(64, 64, Rgba([10, 200, 10, 255]));
        assert!(matches!(
            sculpt_rgba(
                &blank,
                "user.style-x",
                "Blank",
                ImageSculptLimits::default()
            ),
            Err(SculptError::NoSubject(_))
        ));
        let noisy = RgbaImage::from_fn(64, 64, |x, y| {
            let v = ((x * 97 + y * 31) % 255) as u8;
            Rgba([v, 255 - v, (v / 2).wrapping_mul(3), 255])
        });
        assert!(matches!(
            sculpt_rgba(
                &noisy,
                "user.style-x",
                "Noise",
                ImageSculptLimits::default()
            ),
            Err(SculptError::NoSubject(_))
        ));
        let tiny = RgbaImage::from_pixel(4, 4, Rgba([0, 0, 0, 255]));
        assert!(sculpt_rgba(&tiny, "user.style-x", "Tiny", ImageSculptLimits::default()).is_err());
    }

    #[test]
    fn decodes_files_and_enforces_size_limits() {
        let directory =
            std::env::temp_dir().join(format!("agentpet-sculpt-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("dir");
        let path = directory.join("disk.png");
        disk_image(96, Rgba([255, 255, 255, 255]), Rgba([20, 20, 20, 255]))
            .save(&path)
            .expect("save");
        sculpt_image(
            &path,
            "user.style-file",
            "File",
            ImageSculptLimits::default(),
        )
        .expect("sculpts file");
        let limits = ImageSculptLimits {
            max_file_bytes: 10,
            ..ImageSculptLimits::default()
        };
        assert!(matches!(
            sculpt_image(&path, "user.style-file", "File", limits),
            Err(SculptError::FileSize(10))
        ));
        std::fs::write(directory.join("bad.png"), b"not an image").expect("write");
        assert!(matches!(
            sculpt_image(
                &directory.join("bad.png"),
                "user.style-file",
                "Bad",
                ImageSculptLimits::default()
            ),
            Err(SculptError::Decode(_))
        ));
        std::fs::remove_dir_all(&directory).expect("cleanup");
    }
}

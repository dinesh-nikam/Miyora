//! Animated image decoding and sprite-sheet generation.
//!
//! Animated GIF and WebP inputs are decoded into individual frames, laid out
//! into a single horizontal sprite sheet, and described with one frame
//! rectangle per decoded frame. This is what turns a user-supplied animated
//! image into a data-only PetPack animation.

use std::{fs::File, io::BufReader, path::Path};

use image::{AnimationDecoder, Frame, ImageFormat, ImageReader, RgbaImage};

use crate::{FrameRect, PetPackError, MAX_IMAGE_DIMENSION};

pub(crate) const MAX_DECODED_FRAMES: usize = 512;
const MAX_DECODE_ALLOCATION: u64 = 64 * 1024 * 1024;

/// One decoded animation frame with its display duration in milliseconds.
#[derive(Debug, Clone)]
pub struct DecodedFrame {
    pub image: RgbaImage,
    pub delay_ms: u32,
}

/// Result of decoding an animated (or static) image into frames.
#[derive(Debug, Clone)]
pub struct DecodedAnimation {
    pub frames: Vec<DecodedFrame>,
    pub animated: bool,
}

fn reader_with_limits(path: &Path) -> Result<ImageReader<BufReader<File>>, PetPackError> {
    let mut reader = ImageReader::open(path)?.with_guessed_format()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_DIMENSION);
    limits.max_image_height = Some(MAX_IMAGE_DIMENSION);
    limits.max_alloc = Some(MAX_DECODE_ALLOCATION);
    reader.limits(limits);
    Ok(reader)
}

fn frame_delay_ms(frame: &Frame) -> u32 {
    let (numer, denom) = frame.delay().numer_denom_ms();
    if denom == 0 || numer == 0 {
        return 100;
    }
    (numer / denom).max(1)
}

/// Decodes every frame of a GIF, animated WebP, or static image.
///
/// Static images produce a single frame with a 100 ms nominal delay.
/// Animated inputs are bounded to [`MAX_DECODED_FRAMES`]; additional frames
/// are dropped rather than rejected so oversized imports still produce a
/// usable companion.
pub fn decode_animation(path: &Path) -> Result<DecodedAnimation, PetPackError> {
    let reader = reader_with_limits(path)?;
    let format = reader.format();
    if matches!(format, Some(ImageFormat::Gif) | Some(ImageFormat::WebP)) {
        let buffered = BufReader::new(File::open(path)?);
        let frames: Vec<Frame> = match format {
            Some(ImageFormat::Gif) => image::codecs::gif::GifDecoder::new(buffered)?
                .into_frames()
                .collect_frames()?,
            Some(ImageFormat::WebP) => image::codecs::webp::WebPDecoder::new(buffered)?
                .into_frames()
                .collect_frames()?,
            _ => unreachable!("guarded above"),
        };
        if frames.len() > 1 {
            let decoded: Vec<DecodedFrame> = frames
                .into_iter()
                .take(MAX_DECODED_FRAMES)
                .map(|frame| {
                    let delay_ms = frame_delay_ms(&frame);
                    DecodedFrame {
                        image: frame.into_buffer(),
                        delay_ms,
                    }
                })
                .collect();
            if !decoded.is_empty() {
                return Ok(DecodedAnimation {
                    frames: decoded,
                    animated: true,
                });
            }
        }
    }
    let decoded = reader.decode()?;
    Ok(DecodedAnimation {
        frames: vec![DecodedFrame {
            image: decoded.to_rgba8(),
            delay_ms: 100,
        }],
        animated: false,
    })
}

/// Lays decoded frames out into a horizontal strip sprite sheet.
///
/// Frames keep their native size; the sheet width grows with the frame count.
/// Returns the RGBA sheet, one frame rectangle per frame, and the rounded
/// frames-per-second derived from the mean frame delay.
pub fn build_sprite_sheet(
    frames: &[DecodedFrame],
) -> Result<(RgbaImage, Vec<FrameRect>, u16), PetPackError> {
    if frames.is_empty() {
        return Err(PetPackError::Invalid(
            "animation contains no decodable frames".into(),
        ));
    }
    let cell_width = frames.iter().map(|f| f.image.width()).max().unwrap_or(0);
    let cell_height = frames.iter().map(|f| f.image.height()).max().unwrap_or(0);
    if cell_width == 0 || cell_height == 0 {
        return Err(PetPackError::Invalid(
            "animation frame dimensions are empty".into(),
        ));
    }
    let columns =
        u32::try_from(frames.len()).map_err(|_| PetPackError::Invalid("too many frames".into()))?;
    let total_width = cell_width
        .checked_mul(columns)
        .ok_or_else(|| PetPackError::Invalid("sprite sheet width overflow".into()))?;
    if total_width > MAX_IMAGE_DIMENSION || cell_height > MAX_IMAGE_DIMENSION {
        return Err(PetPackError::Invalid(
            "sprite sheet exceeds the 4096 pixel dimension limit".into(),
        ));
    }
    let mut sheet = RgbaImage::from_pixel(total_width, cell_height, image::Rgba([0, 0, 0, 0]));
    let mut rects = Vec::with_capacity(frames.len());
    let mut total_delay_ms = 0_u64;
    for (index, frame) in frames.iter().enumerate() {
        let offset_x = cell_width * index as u32;
        for (x, y, pixel) in frame.image.enumerate_pixels() {
            sheet.put_pixel(offset_x + x, y, *pixel);
        }
        rects.push(FrameRect {
            x: offset_x,
            y: 0,
            width: frame.image.width(),
            height: frame.image.height(),
        });
        total_delay_ms = total_delay_ms.saturating_add(u64::from(frame.delay_ms));
    }
    let mean_delay_ms = (total_delay_ms / frames.len() as u64).max(1);
    let fps = (1_000_u64 / mean_delay_ms)
        .clamp(1, 60)
        .try_into()
        .unwrap_or(1);
    Ok((sheet, rects, fps))
}

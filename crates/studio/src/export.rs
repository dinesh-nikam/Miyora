//! Local export: GLB scene -> OBJ (+MTL/texture), binary STL, binary PLY.
//! Runs fully offline. Every written file is re-read and verified before the
//! export is reported as complete.

use std::path::{Path, PathBuf};

use agentpet_avatar::glb::read_glb;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Bounds that keep a hostile file from exhausting memory.
const MAX_VERTICES: usize = 20_000_000;
const MAX_INDICES: usize = 60_000_000;
const MAX_NODE_DEPTH: usize = 64;
const MAX_NODES_VISITED: usize = 100_000;
const STL_HEADER: usize = 80;
const STL_TRIANGLE_BYTES: usize = 50;
const SCALE_RANGE: std::ops::RangeInclusive<f32> = 0.001..=1000.0;

const FLOAT: u64 = 5126;
const UNSIGNED_BYTE: u64 = 5121;
const UNSIGNED_SHORT: u64 = 5123;
const UNSIGNED_INT: u64 = 5125;
const TRIANGLES: u64 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LocalExportFormat {
    /// Byte-for-byte copy of the stored model (GLB or FBX).
    Original,
    Obj,
    Stl,
    Ply,
}

impl LocalExportFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Original => "original",
            Self::Obj => "obj",
            Self::Stl => "stl",
            Self::Ply => "ply",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ExportOptions {
    /// Uniform scale applied to positions (e.g. 1000 for metres -> millimetres).
    pub scale: f32,
    /// Rotate y-up (glTF) to z-up, the usual convention for slicers and CAD.
    pub z_up: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            scale: 1.0,
            z_up: false,
        }
    }
}

impl ExportOptions {
    pub fn validated(self) -> Result<Self, String> {
        if !self.scale.is_finite() || !SCALE_RANGE.contains(&self.scale) {
            return Err("Scale must be between 0.001 and 1000.".into());
        }
        Ok(self)
    }

    fn apply(&self, [x, y, z]: [f32; 3]) -> [f32; 3] {
        let point = if self.z_up { [x, -z, y] } else { [x, y, z] };
        point.map(|value| value * self.scale)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Texture {
    pub bytes: Vec<u8>,
    pub extension: &'static str,
}

/// Triangle soup merged from every mesh node, in world space.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExportMesh {
    pub positions: Vec<[f32; 3]>,
    pub uvs: Option<Vec<[f32; 2]>>,
    pub colors: Option<Vec<[u8; 3]>>,
    pub indices: Vec<u32>,
    pub texture: Option<Texture>,
}

impl ExportMesh {
    pub fn triangles(&self) -> usize {
        self.indices.len() / 3
    }
}

type Matrix = [f32; 16];
const IDENTITY: Matrix = [
    1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
];

/// Column-major 4x4 product `a * b` (glTF convention).
fn multiply(a: &Matrix, b: &Matrix) -> Matrix {
    let mut out = [0.0; 16];
    for column in 0..4 {
        for row in 0..4 {
            out[column * 4 + row] = (0..4).map(|k| a[k * 4 + row] * b[column * 4 + k]).sum();
        }
    }
    out
}

fn transform_point(m: &Matrix, [x, y, z]: [f32; 3]) -> [f32; 3] {
    [
        m[0] * x + m[4] * y + m[8] * z + m[12],
        m[1] * x + m[5] * y + m[9] * z + m[13],
        m[2] * x + m[6] * y + m[10] * z + m[14],
    ]
}

fn floats<const N: usize>(value: Option<&Value>) -> Option<[f32; N]> {
    let items = value?.as_array()?;
    if items.len() != N {
        return None;
    }
    let mut out = [0.0; N];
    for (slot, item) in out.iter_mut().zip(items) {
        *slot = item.as_f64()? as f32;
    }
    Some(out)
}

/// glTF node transform: `matrix`, or translation * rotation * scale.
fn local_matrix(node: &Value) -> Matrix {
    if let Some(matrix) = floats::<16>(node.get("matrix")) {
        return matrix;
    }
    let [tx, ty, tz] = floats::<3>(node.get("translation")).unwrap_or([0.0; 3]);
    let [qx, qy, qz, qw] = floats::<4>(node.get("rotation")).unwrap_or([0.0, 0.0, 0.0, 1.0]);
    let [sx, sy, sz] = floats::<3>(node.get("scale")).unwrap_or([1.0; 3]);
    let (xx, yy, zz) = (qx * qx, qy * qy, qz * qz);
    let (xy, xz, yz) = (qx * qy, qx * qz, qy * qz);
    let (wx, wy, wz) = (qw * qx, qw * qy, qw * qz);
    [
        (1.0 - 2.0 * (yy + zz)) * sx,
        2.0 * (xy + wz) * sx,
        2.0 * (xz - wy) * sx,
        0.0,
        2.0 * (xy - wz) * sy,
        (1.0 - 2.0 * (xx + zz)) * sy,
        2.0 * (yz + wx) * sy,
        0.0,
        2.0 * (xz + wy) * sz,
        2.0 * (yz - wx) * sz,
        (1.0 - 2.0 * (xx + yy)) * sz,
        0.0,
        tx,
        ty,
        tz,
        1.0,
    ]
}

struct Reader<'a> {
    doc: &'a Value,
    bin: &'a [u8],
}

impl Reader<'_> {
    fn item(&self, collection: &str, index: u64) -> Result<&Value, String> {
        self.doc
            .get(collection)
            .and_then(Value::as_array)
            .and_then(|items| items.get(index as usize))
            .ok_or_else(|| format!("Model references a missing {collection} entry."))
    }

    fn view_bytes(&self, view_index: u64) -> Result<(&[u8], Option<usize>), String> {
        let view = self.item("bufferViews", view_index)?;
        if view.get("buffer").and_then(Value::as_u64) != Some(0) {
            return Err(
                "Model uses external buffers; only self-contained GLB files can be converted."
                    .into(),
            );
        }
        let offset = view.get("byteOffset").and_then(Value::as_u64).unwrap_or(0) as usize;
        let length = view
            .get("byteLength")
            .and_then(Value::as_u64)
            .ok_or("Buffer view has no length.")? as usize;
        let end = offset
            .checked_add(length)
            .filter(|end| *end <= self.bin.len())
            .ok_or("Buffer view exceeds the file.")?;
        let stride = view
            .get("byteStride")
            .and_then(Value::as_u64)
            .map(|stride| stride as usize);
        Ok((&self.bin[offset..end], stride))
    }

    /// Reads an accessor as rows of f32 (normalized integers mapped to 0..1).
    fn read(
        &self,
        accessor_index: u64,
        components: usize,
        max: usize,
    ) -> Result<Vec<Vec<f32>>, String> {
        let accessor = self.item("accessors", accessor_index)?;
        if accessor.get("sparse").is_some() {
            return Err("Sparse accessors are not supported.".into());
        }
        let count = accessor
            .get("count")
            .and_then(Value::as_u64)
            .ok_or("Accessor has no count.")? as usize;
        if count > max {
            return Err("Model is too large to convert.".into());
        }
        let component_type = accessor
            .get("componentType")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let normalized = accessor
            .get("normalized")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let size = match component_type {
            FLOAT | UNSIGNED_INT => 4,
            UNSIGNED_SHORT => 2,
            UNSIGNED_BYTE => 1,
            _ => return Err("Model uses an unsupported accessor component type.".into()),
        };
        let view_index = accessor
            .get("bufferView")
            .and_then(Value::as_u64)
            .ok_or("Accessor has no buffer view.")?;
        let (bytes, stride) = self.view_bytes(view_index)?;
        let base = accessor
            .get("byteOffset")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize;
        let element = size * components;
        let stride = stride.unwrap_or(element);
        let mut rows = Vec::with_capacity(count);
        for index in 0..count {
            let start = base + index * stride;
            let chunk = bytes
                .get(start..start + element)
                .ok_or("Accessor data exceeds its buffer view.")?;
            let row = (0..components)
                .map(|component| {
                    let at = &chunk[component * size..(component + 1) * size];
                    match component_type {
                        FLOAT => f32::from_le_bytes([at[0], at[1], at[2], at[3]]),
                        UNSIGNED_INT => u32::from_le_bytes([at[0], at[1], at[2], at[3]]) as f32,
                        UNSIGNED_SHORT => {
                            let value = u16::from_le_bytes([at[0], at[1]]) as f32;
                            if normalized {
                                value / 65535.0
                            } else {
                                value
                            }
                        }
                        _ => {
                            let value = at[0] as f32;
                            if normalized {
                                value / 255.0
                            } else {
                                value
                            }
                        }
                    }
                })
                .collect();
            rows.push(row);
        }
        Ok(rows)
    }

    fn read_indices(&self, accessor_index: u64) -> Result<Vec<u32>, String> {
        let accessor = self.item("accessors", accessor_index)?;
        if accessor.get("componentType").and_then(Value::as_u64) == Some(FLOAT) {
            return Err("Index data must be integers.".into());
        }
        Ok(self
            .read(accessor_index, 1, MAX_INDICES)?
            .into_iter()
            .map(|row| row[0] as u32)
            .collect())
    }

    fn texture(&self, material_index: u64) -> Option<Texture> {
        let texture_index = self
            .item("materials", material_index)
            .ok()?
            .get("pbrMetallicRoughness")?
            .get("baseColorTexture")?
            .get("index")?
            .as_u64()?;
        let image_index = self
            .item("textures", texture_index)
            .ok()?
            .get("source")?
            .as_u64()?;
        let image = self.item("images", image_index).ok()?;
        let extension = match image.get("mimeType")?.as_str()? {
            "image/png" => "png",
            "image/jpeg" => "jpg",
            _ => return None,
        };
        let (bytes, _) = self.view_bytes(image.get("bufferView")?.as_u64()?).ok()?;
        Some(Texture {
            bytes: bytes.to_vec(),
            extension,
        })
    }
}

/// Collects every triangle primitive of the default scene in world space.
pub fn read_mesh(glb: &[u8]) -> Result<ExportMesh, String> {
    let container = read_glb(glb).map_err(|error| format!("Not a valid GLB: {error}"))?;
    let doc = &container.document;
    if let Some(required) = doc
        .get("extensionsRequired")
        .and_then(Value::as_array)
        .filter(|items| !items.is_empty())
    {
        let names: Vec<&str> = required.iter().filter_map(Value::as_str).collect();
        return Err(format!(
            "Model uses compressed or extended data ({}); export the original GLB instead.",
            names.join(", ")
        ));
    }
    let reader = Reader {
        doc,
        bin: container.binary,
    };
    let scene_index = doc.get("scene").and_then(Value::as_u64).unwrap_or(0);
    let scene = doc
        .get("scenes")
        .and_then(Value::as_array)
        .and_then(|scenes| scenes.get(scene_index as usize));
    let roots: Vec<u64> = match scene {
        Some(scene) => scene
            .get("nodes")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_u64)
            .collect(),
        None => (0..doc
            .get("nodes")
            .and_then(Value::as_array)
            .map_or(0, Vec::len) as u64)
            .collect(),
    };

    let mut mesh = ExportMesh::default();
    let mut uvs: Vec<Option<[f32; 2]>> = Vec::new();
    let mut colors: Vec<Option<[u8; 3]>> = Vec::new();
    let mut stack: Vec<(u64, Matrix, usize)> =
        roots.into_iter().map(|node| (node, IDENTITY, 0)).collect();
    let mut visited = 0usize;
    while let Some((node_index, parent, depth)) = stack.pop() {
        visited += 1;
        if depth > MAX_NODE_DEPTH || visited > MAX_NODES_VISITED {
            return Err("Model node hierarchy is too deep or cyclic.".into());
        }
        let node = reader.item("nodes", node_index)?;
        let world = multiply(&parent, &local_matrix(node));
        for child in node
            .get("children")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_u64)
        {
            stack.push((child, world, depth + 1));
        }
        let Some(mesh_index) = node.get("mesh").and_then(Value::as_u64) else {
            continue;
        };
        let primitives = reader
            .item("meshes", mesh_index)?
            .get("primitives")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for primitive in &primitives {
            if primitive
                .get("mode")
                .and_then(Value::as_u64)
                .unwrap_or(TRIANGLES)
                != TRIANGLES
            {
                continue;
            }
            let attributes = primitive
                .get("attributes")
                .ok_or("Primitive has no attributes.")?;
            let position = attributes
                .get("POSITION")
                .and_then(Value::as_u64)
                .ok_or("Primitive has no positions.")?;
            let positions = reader.read(position, 3, MAX_VERTICES)?;
            let base = mesh.positions.len();
            if base + positions.len() > MAX_VERTICES {
                return Err("Model is too large to convert.".into());
            }
            for row in &positions {
                mesh.positions
                    .push(transform_point(&world, [row[0], row[1], row[2]]));
            }
            let primitive_uvs = attributes
                .get("TEXCOORD_0")
                .and_then(Value::as_u64)
                .map(|uv| reader.read(uv, 2, MAX_VERTICES))
                .transpose()?;
            let primitive_colors = match attributes.get("COLOR_0").and_then(Value::as_u64) {
                Some(color) => {
                    let components = match reader
                        .item("accessors", color)?
                        .get("type")
                        .and_then(Value::as_str)
                    {
                        Some("VEC4") => 4,
                        _ => 3,
                    };
                    Some(reader.read(color, components, MAX_VERTICES)?)
                }
                None => None,
            };
            for index in 0..positions.len() {
                uvs.push(
                    primitive_uvs
                        .as_ref()
                        .and_then(|rows| rows.get(index))
                        .map(|row| [row[0], row[1]]),
                );
                colors.push(
                    primitive_colors
                        .as_ref()
                        .and_then(|rows| rows.get(index))
                        .map(|row| {
                            [row[0], row[1], row[2]]
                                .map(|channel| (channel.clamp(0.0, 1.0) * 255.0).round() as u8)
                        }),
                );
            }
            let indices = match primitive.get("indices").and_then(Value::as_u64) {
                Some(accessor) => reader.read_indices(accessor)?,
                None => (0..positions.len() as u32).collect(),
            };
            if indices
                .iter()
                .any(|index| *index as usize >= positions.len())
            {
                return Err("Model has out-of-range triangle indices.".into());
            }
            if mesh.indices.len() + indices.len() > MAX_INDICES {
                return Err("Model is too large to convert.".into());
            }
            mesh.indices.extend(
                indices
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .flatten()
                    .map(|index| index + base as u32),
            );
            if mesh.texture.is_none() {
                mesh.texture = primitive
                    .get("material")
                    .and_then(Value::as_u64)
                    .and_then(|material| reader.texture(material));
            }
        }
    }
    if mesh.indices.is_empty() {
        return Err("Model contains no triangles to export.".into());
    }
    if uvs.iter().any(Option::is_some) {
        mesh.uvs = Some(uvs.into_iter().map(|uv| uv.unwrap_or([0.0, 0.0])).collect());
    } else {
        mesh.texture = None;
    }
    if colors.iter().any(Option::is_some) {
        mesh.colors = Some(
            colors
                .into_iter()
                .map(|color| color.unwrap_or([255, 255, 255]))
                .collect(),
        );
    }
    Ok(mesh)
}

fn face_normal(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let n = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    let length = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    if length > 0.0 {
        n.map(|value| value / length)
    } else {
        [0.0; 3]
    }
}

pub fn write_stl(mesh: &ExportMesh, options: &ExportOptions) -> Vec<u8> {
    let mut out = Vec::with_capacity(STL_HEADER + 4 + mesh.triangles() * STL_TRIANGLE_BYTES);
    let mut header = [0u8; STL_HEADER];
    let label = b"Miyora 3D Studio export";
    header[..label.len()].copy_from_slice(label);
    out.extend_from_slice(&header);
    out.extend_from_slice(&(mesh.triangles() as u32).to_le_bytes());
    for triangle in mesh.indices.as_chunks::<3>().0 {
        let [a, b, c] =
            [0, 1, 2].map(|corner| options.apply(mesh.positions[triangle[corner] as usize]));
        for value in face_normal(a, b, c).into_iter().chain(a).chain(b).chain(c) {
            out.extend_from_slice(&value.to_le_bytes());
        }
        out.extend_from_slice(&0u16.to_le_bytes());
    }
    out
}

pub fn write_ply(mesh: &ExportMesh, options: &ExportOptions) -> Vec<u8> {
    let mut header = format!(
        "ply\nformat binary_little_endian 1.0\ncomment Miyora 3D Studio export\nelement vertex {}\nproperty float x\nproperty float y\nproperty float z\n",
        mesh.positions.len()
    );
    if mesh.colors.is_some() {
        header.push_str("property uchar red\nproperty uchar green\nproperty uchar blue\n");
    }
    header.push_str(&format!(
        "element face {}\nproperty list uchar int vertex_indices\nend_header\n",
        mesh.triangles()
    ));
    let mut out = header.into_bytes();
    for (index, position) in mesh.positions.iter().enumerate() {
        for value in options.apply(*position) {
            out.extend_from_slice(&value.to_le_bytes());
        }
        if let Some(colors) = &mesh.colors {
            out.extend_from_slice(&colors[index]);
        }
    }
    for triangle in mesh.indices.as_chunks::<3>().0 {
        out.push(3);
        for index in triangle {
            out.extend_from_slice(&(*index as i32).to_le_bytes());
        }
    }
    out
}

pub struct ObjFiles {
    pub obj: Vec<u8>,
    /// `(file name, bytes)` for the material library and texture, when present.
    pub sidecars: Vec<(String, Vec<u8>)>,
}

/// OBJ with `vt` UVs and, when present, vertex colours as `v x y z r g b`
/// (read by Blender and MeshLab). A base-colour texture is written next to it.
pub fn write_obj(mesh: &ExportMesh, options: &ExportOptions, stem: &str) -> ObjFiles {
    let mut obj = String::from("# Miyora 3D Studio export\n");
    let mut sidecars = Vec::new();
    if let Some(texture) = &mesh.texture {
        let texture_name = format!("{stem}_texture.{}", texture.extension);
        let mtl = format!("newmtl material0\nKd 1 1 1\nmap_Kd {texture_name}\n");
        obj.push_str(&format!("mtllib {stem}.mtl\nusemtl material0\n"));
        sidecars.push((format!("{stem}.mtl"), mtl.into_bytes()));
        sidecars.push((texture_name, texture.bytes.clone()));
    }
    for (index, position) in mesh.positions.iter().enumerate() {
        let [x, y, z] = options.apply(*position);
        match &mesh.colors {
            Some(colors) => {
                let [r, g, b] = colors[index].map(|channel| channel as f32 / 255.0);
                obj.push_str(&format!("v {x} {y} {z} {r:.4} {g:.4} {b:.4}\n"));
            }
            None => obj.push_str(&format!("v {x} {y} {z}\n")),
        }
    }
    if let Some(uvs) = &mesh.uvs {
        // glTF UV origin is top-left; OBJ is bottom-left.
        for [u, v] in uvs {
            obj.push_str(&format!("vt {u} {}\n", 1.0 - v));
        }
    }
    for triangle in mesh.indices.as_chunks::<3>().0 {
        let [a, b, c] = [triangle[0] + 1, triangle[1] + 1, triangle[2] + 1];
        if mesh.uvs.is_some() {
            obj.push_str(&format!("f {a}/{a} {b}/{b} {c}/{c}\n"));
        } else {
            obj.push_str(&format!("f {a} {b} {c}\n"));
        }
    }
    ObjFiles {
        obj: obj.into_bytes(),
        sidecars,
    }
}

/// Re-reads a written file and checks it holds the expected geometry.
pub fn verify(
    format: LocalExportFormat,
    bytes: &[u8],
    vertices: usize,
    triangles: usize,
) -> Result<(), String> {
    let fail = |what: &str| Err(format!("Export verification failed: {what}."));
    match format {
        LocalExportFormat::Original => Ok(()),
        LocalExportFormat::Stl => {
            let count = bytes
                .get(STL_HEADER..STL_HEADER + 4)
                .map(|word| u32::from_le_bytes([word[0], word[1], word[2], word[3]]) as usize);
            if count != Some(triangles)
                || bytes.len() != STL_HEADER + 4 + triangles * STL_TRIANGLE_BYTES
            {
                return fail("STL triangle count or size mismatch");
            }
            Ok(())
        }
        LocalExportFormat::Ply => {
            let end = bytes
                .windows(11)
                .position(|window| window == b"end_header\n")
                .ok_or("Export verification failed: PLY header missing.")?;
            let header = std::str::from_utf8(&bytes[..end])
                .map_err(|_| "Export verification failed: PLY header unreadable.".to_string())?;
            if !header.contains(&format!("element vertex {vertices}\n"))
                || !header.contains(&format!("element face {triangles}\n"))
            {
                return fail("PLY element counts mismatch");
            }
            Ok(())
        }
        LocalExportFormat::Obj => {
            let text = std::str::from_utf8(bytes)
                .map_err(|_| "Export verification failed: OBJ is not text.".to_string())?;
            let count = |prefix: &str| text.lines().filter(|line| line.starts_with(prefix)).count();
            if count("v ") != vertices || count("f ") != triangles {
                return fail("OBJ vertex or face count mismatch");
            }
            Ok(())
        }
    }
}

const GLB_MAGIC: u32 = 0x4654_6C67;
const GLB_JSON_CHUNK: u32 = 0x4E4F_534A;
const GLB_BIN_CHUNK: u32 = 0x004E_4942;
const ARRAY_BUFFER: u32 = 34962;
const ELEMENT_ARRAY_BUFFER: u32 = 34963;

/// Smooth per-vertex normals from area-weighted face normals.
fn vertex_normals(mesh: &ExportMesh) -> Vec<[f32; 3]> {
    let mut normals = vec![[0.0f32; 3]; mesh.positions.len()];
    for triangle in mesh.indices.as_chunks::<3>().0 {
        let [a, b, c] = triangle.map(|index| mesh.positions[index as usize]);
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let n = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        for index in triangle {
            for axis in 0..3 {
                normals[*index as usize][axis] += n[axis];
            }
        }
    }
    normals
        .into_iter()
        .map(|n| {
            let length = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            if length > 0.0 {
                n.map(|value| value / length)
            } else {
                [0.0, 1.0, 0.0]
            }
        })
        .collect()
}

/// Self-contained GLB 2.0 for a mesh produced by this crate (e.g. after
/// polygon reduction): positions, normals, optional UVs, vertex colours and
/// base-colour texture.
pub fn write_glb(mesh: &ExportMesh) -> Vec<u8> {
    let mut bin: Vec<u8> = Vec::new();
    let mut views = Vec::new();
    let mut push_view = |bytes: &[u8], target: Option<u32>| -> usize {
        while bin.len() % 4 != 0 {
            bin.push(0);
        }
        let mut view =
            serde_json::json!({ "buffer": 0, "byteOffset": bin.len(), "byteLength": bytes.len() });
        if let Some(target) = target {
            view["target"] = target.into();
        }
        bin.extend_from_slice(bytes);
        views.push(view);
        views.len() - 1
    };
    let floats = |values: &mut dyn Iterator<Item = f32>| {
        values.flat_map(f32::to_le_bytes).collect::<Vec<u8>>()
    };
    let count = mesh.positions.len();
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for position in &mesh.positions {
        for axis in 0..3 {
            min[axis] = min[axis].min(position[axis]);
            max[axis] = max[axis].max(position[axis]);
        }
    }
    let mut accessors = Vec::new();
    let mut attributes = serde_json::Map::new();
    let position_view = push_view(
        &floats(&mut mesh.positions.iter().flatten().copied()),
        Some(ARRAY_BUFFER),
    );
    accessors.push(serde_json::json!({ "bufferView": position_view, "componentType": FLOAT, "count": count, "type": "VEC3", "min": min, "max": max }));
    attributes.insert("POSITION".into(), (accessors.len() - 1).into());
    let normals = vertex_normals(mesh);
    let normal_view = push_view(
        &floats(&mut normals.iter().flatten().copied()),
        Some(ARRAY_BUFFER),
    );
    accessors.push(serde_json::json!({ "bufferView": normal_view, "componentType": FLOAT, "count": count, "type": "VEC3" }));
    attributes.insert("NORMAL".into(), (accessors.len() - 1).into());
    if let Some(uvs) = &mesh.uvs {
        let view = push_view(
            &floats(&mut uvs.iter().flatten().copied()),
            Some(ARRAY_BUFFER),
        );
        accessors.push(serde_json::json!({ "bufferView": view, "componentType": FLOAT, "count": count, "type": "VEC2" }));
        attributes.insert("TEXCOORD_0".into(), (accessors.len() - 1).into());
    }
    if let Some(colors) = &mesh.colors {
        let mut channels = colors
            .iter()
            .flatten()
            .map(|channel| f32::from(*channel) / 255.0);
        let view = push_view(&floats(&mut channels), Some(ARRAY_BUFFER));
        accessors.push(serde_json::json!({ "bufferView": view, "componentType": FLOAT, "count": count, "type": "VEC3" }));
        attributes.insert("COLOR_0".into(), (accessors.len() - 1).into());
    }
    let index_bytes: Vec<u8> = mesh
        .indices
        .iter()
        .flat_map(|index| index.to_le_bytes())
        .collect();
    let index_view = push_view(&index_bytes, Some(ELEMENT_ARRAY_BUFFER));
    accessors.push(serde_json::json!({ "bufferView": index_view, "componentType": UNSIGNED_INT, "count": mesh.indices.len(), "type": "SCALAR" }));
    let indices_accessor = accessors.len() - 1;
    let mut document = serde_json::json!({
        "asset": { "version": "2.0", "generator": "Miyora 3D Studio" },
        "scene": 0,
        "scenes": [{ "nodes": [0] }],
        "nodes": [{ "mesh": 0 }],
        "meshes": [{ "primitives": [{ "attributes": attributes, "indices": indices_accessor, "material": 0, "mode": TRIANGLES }] }],
        "materials": [{ "pbrMetallicRoughness": { "metallicFactor": 0.0, "roughnessFactor": 1.0 } }],
    });
    if let (Some(texture), true) = (&mesh.texture, mesh.uvs.is_some()) {
        let image_view = push_view(&texture.bytes, None);
        let mime = if texture.extension == "png" {
            "image/png"
        } else {
            "image/jpeg"
        };
        document["images"] = serde_json::json!([{ "bufferView": image_view, "mimeType": mime }]);
        document["samplers"] = serde_json::json!([{}]);
        document["textures"] = serde_json::json!([{ "source": 0, "sampler": 0 }]);
        document["materials"][0]["pbrMetallicRoughness"]["baseColorTexture"] =
            serde_json::json!({ "index": 0 });
    }
    while bin.len() % 4 != 0 {
        bin.push(0);
    }
    document["accessors"] = Value::Array(accessors);
    document["bufferViews"] = Value::Array(views);
    document["buffers"] = serde_json::json!([{ "byteLength": bin.len() }]);
    let mut json = document.to_string().into_bytes();
    while json.len() % 4 != 0 {
        json.push(b' ');
    }
    let total = 12 + 8 + json.len() + 8 + bin.len();
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(&GLB_MAGIC.to_le_bytes());
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&(json.len() as u32).to_le_bytes());
    out.extend_from_slice(&GLB_JSON_CHUNK.to_le_bytes());
    out.extend_from_slice(&json);
    out.extend_from_slice(&(bin.len() as u32).to_le_bytes());
    out.extend_from_slice(&GLB_BIN_CHUNK.to_le_bytes());
    out.extend_from_slice(&bin);
    out
}

/// Writes through a temporary file in the destination folder, then renames.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut partial = path.as_os_str().to_owned();
    partial.push(".part");
    let partial = PathBuf::from(partial);
    std::fs::write(&partial, bytes)?;
    std::fs::rename(&partial, path).inspect_err(|_| {
        // Best effort: leave no stray temp file when the rename fails.
        let _ = std::fs::remove_file(&partial);
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use agentpet_avatar::glb::{write_textured_glb, MeshData};

    /// A unit square (2 triangles) with UVs and a texture, through the repo's own GLB writer.
    pub(crate) fn textured_glb() -> Vec<u8> {
        let mesh = MeshData {
            positions: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            normals: vec![[0.0, 0.0, 1.0]; 4],
            uvs: vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
            indices: vec![0, 1, 2, 0, 2, 3],
        };
        let png = crate::media::tests::png_bytes(4, 4);
        write_textured_glb(&mesh, &png, "square", serde_json::json!({})).unwrap()
    }

    #[test]
    fn reads_geometry_uvs_and_texture_from_glb() {
        let mesh = read_mesh(&textured_glb()).unwrap();
        assert_eq!((mesh.positions.len(), mesh.triangles()), (4, 2));
        assert_eq!(mesh.uvs.as_ref().unwrap()[2], [1.0, 0.0]);
        assert_eq!(mesh.texture.as_ref().unwrap().extension, "png");
        assert!(mesh.colors.is_none());
    }

    #[test]
    fn writers_produce_verifiable_files() {
        let mesh = read_mesh(&textured_glb()).unwrap();
        let options = ExportOptions::default();
        let stl = write_stl(&mesh, &options);
        verify(LocalExportFormat::Stl, &stl, 4, 2).unwrap();
        assert!(verify(LocalExportFormat::Stl, &stl, 4, 3).is_err());
        let ply = write_ply(&mesh, &options);
        verify(LocalExportFormat::Ply, &ply, 4, 2).unwrap();
        let obj = write_obj(&mesh, &options, "chest");
        verify(LocalExportFormat::Obj, &obj.obj, 4, 2).unwrap();
        let text = String::from_utf8(obj.obj).unwrap();
        assert!(text.contains("mtllib chest.mtl") && text.contains("f 1/1 2/2 3/3"));
        assert!(text.contains("vt 1 1\n"), "UV v flipped for OBJ");
        let names: Vec<&str> = obj.sidecars.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, vec!["chest.mtl", "chest_texture.png"]);
    }

    #[test]
    fn options_scale_and_rotate_to_z_up() {
        let options = ExportOptions {
            scale: 1000.0,
            z_up: true,
        };
        assert_eq!(options.apply([1.0, 2.0, 3.0]), [1000.0, -3000.0, 2000.0]);
        assert!(ExportOptions {
            scale: 0.0,
            z_up: false
        }
        .validated()
        .is_err());
        assert!(ExportOptions {
            scale: f32::NAN,
            z_up: false
        }
        .validated()
        .is_err());
    }

    #[test]
    fn node_transforms_are_applied() {
        let node = serde_json::json!({ "translation": [1.0, 2.0, 3.0], "scale": [2.0, 2.0, 2.0] });
        assert_eq!(
            transform_point(&local_matrix(&node), [1.0, 1.0, 1.0]),
            [3.0, 4.0, 5.0]
        );
        let half = std::f32::consts::FRAC_1_SQRT_2;
        let quarter_turn_y = serde_json::json!({ "rotation": [0.0, half, 0.0, half] });
        let rotated = transform_point(&local_matrix(&quarter_turn_y), [1.0, 0.0, 0.0]);
        assert!(rotated[0].abs() < 1e-5 && (rotated[2] + 1.0).abs() < 1e-5);
        let parent = local_matrix(&serde_json::json!({ "translation": [10.0, 0.0, 0.0] }));
        let child = local_matrix(&node);
        assert_eq!(
            transform_point(&multiply(&parent, &child), [1.0, 1.0, 1.0]),
            [13.0, 4.0, 5.0]
        );
    }

    #[test]
    fn written_glb_round_trips_and_validates() {
        let source = read_mesh(&textured_glb()).unwrap();
        let mut coloured = source.clone();
        coloured.colors = Some(vec![[10, 20, 30]; 4]);
        for mesh in [source, coloured] {
            let glb = write_glb(&mesh);
            let report = crate::media::inspect_model(&glb).unwrap();
            assert_eq!((report.triangles, report.vertices), (Some(2), Some(4)));
            assert_eq!(report.has_vertex_colors, Some(mesh.colors.is_some()));
            assert!(report.pbr.unwrap().base_color, "texture kept");
            let back = read_mesh(&glb).unwrap();
            assert_eq!(back.positions, mesh.positions);
            assert_eq!(back.indices, mesh.indices);
            assert_eq!(back.uvs, mesh.uvs);
            assert_eq!(back.colors, mesh.colors);
            assert_eq!(back.texture, mesh.texture);
        }
    }

    #[test]
    fn rejects_compressed_corrupt_and_empty_models() {
        let compressed = crate::media::tests::glb_bytes(
            r#"{"asset":{"version":"2.0"},"extensionsRequired":["EXT_meshopt_compression"]}"#,
        );
        assert!(read_mesh(&compressed)
            .unwrap_err()
            .contains("EXT_meshopt_compression"));
        assert!(read_mesh(b"not a glb").is_err());
        let empty = crate::media::tests::glb_bytes(r#"{"asset":{"version":"2.0"},"nodes":[]}"#);
        assert!(read_mesh(&empty).unwrap_err().contains("no triangles"));
        let mut truncated = textured_glb();
        truncated.truncate(truncated.len() - 8);
        assert!(read_mesh(&truncated).is_err());
    }
}

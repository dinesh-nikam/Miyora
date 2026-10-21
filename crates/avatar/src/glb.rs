//! Minimal binary glTF 2.0 writer and container reader.
//!
//! The writer emits a single textured triangle mesh with all buffers and the
//! texture embedded, so the result never references external resources.

use serde_json::{json, Value};
use thiserror::Error;

const GLB_MAGIC: u32 = 0x4654_6C67; // "glTF"
const GLB_VERSION: u32 = 2;
const CHUNK_JSON: u32 = 0x4E4F_534A; // "JSON"
const CHUNK_BIN: u32 = 0x004E_4942; // "BIN\0"
const TARGET_ARRAY_BUFFER: u32 = 34962;
const TARGET_ELEMENT_ARRAY_BUFFER: u32 = 34963;
const COMPONENT_FLOAT: u32 = 5126;
const COMPONENT_UNSIGNED_INT: u32 = 5125;
const FILTER_LINEAR: u32 = 9729;
const WRAP_CLAMP_TO_EDGE: u32 = 33071;

#[derive(Debug, Error, PartialEq)]
pub enum GlbError {
    #[error("mesh is empty")]
    EmptyMesh,
    #[error("mesh attribute lengths do not match")]
    AttributeMismatch,
    #[error("mesh index {0} is out of range")]
    IndexOutOfRange(u32),
    #[error("mesh contains a non-finite coordinate")]
    NonFinite,
    #[error("binary glTF container is invalid: {0}")]
    Container(String),
}

#[derive(Debug, Clone, Default)]
pub struct MeshData {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

impl MeshData {
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    fn validate(&self) -> Result<(), GlbError> {
        if self.positions.is_empty() || self.indices.is_empty() || self.indices.len() % 3 != 0 {
            return Err(GlbError::EmptyMesh);
        }
        if self.normals.len() != self.positions.len() || self.uvs.len() != self.positions.len() {
            return Err(GlbError::AttributeMismatch);
        }
        let count = self.positions.len() as u32;
        if let Some(&bad) = self.indices.iter().find(|&&index| index >= count) {
            return Err(GlbError::IndexOutOfRange(bad));
        }
        let finite = self
            .positions
            .iter()
            .chain(self.normals.iter())
            .flatten()
            .chain(self.uvs.iter().flatten())
            .all(|value| value.is_finite());
        if !finite {
            return Err(GlbError::NonFinite);
        }
        Ok(())
    }
}

fn pad_to_four(buffer: &mut Vec<u8>, fill: u8) {
    while buffer.len() % 4 != 0 {
        buffer.push(fill);
    }
}

fn float_bytes<'a>(values: impl Iterator<Item = &'a f32>) -> Vec<u8> {
    values.flat_map(|value| value.to_le_bytes()).collect()
}

/// Writes `mesh` and the PNG `texture_png` into a self-contained GLB file.
pub fn write_textured_glb(
    mesh: &MeshData,
    texture_png: &[u8],
    node_name: &str,
    extras: Value,
) -> Result<Vec<u8>, GlbError> {
    mesh.validate()?;
    let mut bin = Vec::new();
    let mut views = Vec::new();
    let mut push_view = |bytes: &[u8], target: Option<u32>| {
        pad_to_four(&mut bin, 0);
        let offset = bin.len();
        bin.extend_from_slice(bytes);
        let mut view = json!({ "buffer": 0, "byteOffset": offset, "byteLength": bytes.len() });
        if let Some(target) = target {
            view["target"] = json!(target);
        }
        views.push(view);
        views.len() - 1
    };

    let indices: Vec<u8> = mesh.indices.iter().flat_map(|i| i.to_le_bytes()).collect();
    let position_view = push_view(
        &float_bytes(mesh.positions.iter().flatten()),
        Some(TARGET_ARRAY_BUFFER),
    );
    let normal_view = push_view(
        &float_bytes(mesh.normals.iter().flatten()),
        Some(TARGET_ARRAY_BUFFER),
    );
    let uv_view = push_view(
        &float_bytes(mesh.uvs.iter().flatten()),
        Some(TARGET_ARRAY_BUFFER),
    );
    let index_view = push_view(&indices, Some(TARGET_ELEMENT_ARRAY_BUFFER));
    let image_view = push_view(texture_png, None);
    pad_to_four(&mut bin, 0);

    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for position in &mesh.positions {
        for axis in 0..3 {
            min[axis] = min[axis].min(position[axis]);
            max[axis] = max[axis].max(position[axis]);
        }
    }
    let vertex_count = mesh.positions.len();
    let document = json!({
        "asset": { "version": "2.0", "generator": "AgentPet avatar sculptor" },
        "scene": 0,
        "scenes": [{ "nodes": [0] }],
        "nodes": [{ "mesh": 0, "name": node_name }],
        "meshes": [{
            "name": node_name,
            "primitives": [{
                "attributes": { "POSITION": 0, "NORMAL": 1, "TEXCOORD_0": 2 },
                "indices": 3,
                "material": 0
            }]
        }],
        "materials": [{
            "name": "sculpt",
            "pbrMetallicRoughness": {
                "baseColorTexture": { "index": 0 },
                "metallicFactor": 0.0,
                "roughnessFactor": 0.82
            }
        }],
        "textures": [{ "source": 0, "sampler": 0 }],
        "samplers": [{
            "magFilter": FILTER_LINEAR,
            "minFilter": FILTER_LINEAR,
            "wrapS": WRAP_CLAMP_TO_EDGE,
            "wrapT": WRAP_CLAMP_TO_EDGE
        }],
        "images": [{ "bufferView": image_view, "mimeType": "image/png" }],
        "accessors": [
            { "bufferView": position_view, "componentType": COMPONENT_FLOAT, "count": vertex_count, "type": "VEC3", "min": min, "max": max },
            { "bufferView": normal_view, "componentType": COMPONENT_FLOAT, "count": vertex_count, "type": "VEC3" },
            { "bufferView": uv_view, "componentType": COMPONENT_FLOAT, "count": vertex_count, "type": "VEC2" },
            { "bufferView": index_view, "componentType": COMPONENT_UNSIGNED_INT, "count": mesh.indices.len(), "type": "SCALAR" }
        ],
        "bufferViews": views,
        "buffers": [{ "byteLength": bin.len() }],
        "extras": extras
    });
    let mut json_chunk =
        serde_json::to_vec(&document).map_err(|error| GlbError::Container(error.to_string()))?;
    pad_to_four(&mut json_chunk, b' ');

    let total = 12 + 8 + json_chunk.len() + 8 + bin.len();
    let total =
        u32::try_from(total).map_err(|_| GlbError::Container("output exceeds 4 GiB".into()))?;
    let mut out = Vec::with_capacity(total as usize);
    out.extend_from_slice(&GLB_MAGIC.to_le_bytes());
    out.extend_from_slice(&GLB_VERSION.to_le_bytes());
    out.extend_from_slice(&total.to_le_bytes());
    out.extend_from_slice(&(json_chunk.len() as u32).to_le_bytes());
    out.extend_from_slice(&CHUNK_JSON.to_le_bytes());
    out.extend_from_slice(&json_chunk);
    out.extend_from_slice(&(bin.len() as u32).to_le_bytes());
    out.extend_from_slice(&CHUNK_BIN.to_le_bytes());
    out.extend_from_slice(&bin);
    Ok(out)
}

/// Parsed GLB container: the JSON document and the binary chunk.
#[derive(Debug)]
pub struct GlbContainer<'a> {
    pub document: Value,
    pub binary: &'a [u8],
}

/// Validates the GLB header and chunk layout and returns the parsed document.
pub fn read_glb(bytes: &[u8]) -> Result<GlbContainer<'_>, GlbError> {
    let fail = |reason: &str| GlbError::Container(reason.to_owned());
    let word = |offset: usize| -> Result<u32, GlbError> {
        bytes
            .get(offset..offset + 4)
            .map(|slice| u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
            .ok_or_else(|| fail("truncated header"))
    };
    if word(0)? != GLB_MAGIC {
        return Err(fail("missing glTF magic"));
    }
    if word(4)? != GLB_VERSION {
        return Err(fail("unsupported container version"));
    }
    if word(8)? as usize != bytes.len() {
        return Err(fail("declared length does not match file length"));
    }
    let json_length = word(12)? as usize;
    if word(16)? != CHUNK_JSON {
        return Err(fail("first chunk is not JSON"));
    }
    let json_end = 20usize
        .checked_add(json_length)
        .filter(|&end| end <= bytes.len())
        .ok_or_else(|| fail("JSON chunk exceeds file"))?;
    let document: Value = serde_json::from_slice(&bytes[20..json_end])
        .map_err(|error| GlbError::Container(error.to_string()))?;
    let binary = if json_end == bytes.len() {
        &bytes[json_end..]
    } else {
        let bin_length = word(json_end)? as usize;
        if word(json_end + 4)? != CHUNK_BIN {
            return Err(fail("second chunk is not BIN"));
        }
        let start = json_end + 8;
        let end = start
            .checked_add(bin_length)
            .filter(|&end| end <= bytes.len())
            .ok_or_else(|| fail("BIN chunk exceeds file"))?;
        &bytes[start..end]
    };
    Ok(GlbContainer { document, binary })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn triangle() -> MeshData {
        MeshData {
            positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            normals: vec![[0.0, 0.0, 1.0]; 3],
            uvs: vec![[0.0, 1.0], [1.0, 1.0], [0.0, 0.0]],
            indices: vec![0, 1, 2],
        }
    }

    #[test]
    fn writes_a_self_contained_container_that_round_trips() {
        let png = [0x89, b'P', b'N', b'G', 1, 2, 3];
        let glb =
            write_textured_glb(&triangle(), &png, "test", json!({"kind": "test"})).expect("writes");
        assert_eq!(glb.len() % 4, 0);
        let container = read_glb(&glb).expect("reads");
        let doc = &container.document;
        assert_eq!(doc["asset"]["version"], "2.0");
        assert_eq!(doc["accessors"][0]["count"], 3);
        assert_eq!(doc["accessors"][0]["max"][0], 1.0);
        assert!(doc["buffers"][0].get("uri").is_none(), "no external buffer");
        let view_index = doc["images"][0]["bufferView"].as_u64().expect("view") as usize;
        let offset = doc["bufferViews"][view_index]["byteOffset"]
            .as_u64()
            .expect("offset") as usize;
        assert_eq!(&container.binary[offset..offset + png.len()], &png);
        for view in doc["bufferViews"].as_array().expect("views") {
            assert_eq!(view["byteOffset"].as_u64().expect("offset") % 4, 0);
        }
    }

    #[test]
    fn rejects_invalid_meshes() {
        let mut mesh = triangle();
        mesh.indices = vec![0, 1, 5];
        assert_eq!(
            write_textured_glb(&mesh, &[], "x", Value::Null).unwrap_err(),
            GlbError::IndexOutOfRange(5)
        );
        let mut mesh = triangle();
        mesh.positions[0][0] = f32::NAN;
        assert_eq!(
            write_textured_glb(&mesh, &[], "x", Value::Null).unwrap_err(),
            GlbError::NonFinite
        );
        let mut mesh = triangle();
        mesh.uvs.pop();
        assert_eq!(
            write_textured_glb(&mesh, &[], "x", Value::Null).unwrap_err(),
            GlbError::AttributeMismatch
        );
    }

    #[test]
    fn rejects_corrupted_containers() {
        let glb = write_textured_glb(&triangle(), &[1, 2, 3], "t", Value::Null).expect("writes");
        assert!(read_glb(&glb[..glb.len() - 4]).is_err());
        let mut bad = glb.clone();
        bad[0] = 0;
        assert!(read_glb(&bad).is_err());
        assert!(read_glb(&[]).is_err());
    }
}

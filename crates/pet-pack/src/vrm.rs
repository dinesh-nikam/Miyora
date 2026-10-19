//! VRM (glTF 2.0 binary) 3D Avatar Guardrail Validation and Metadata Extraction.

use std::{
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

use serde::{Deserialize, Serialize};

use crate::PetPackError;

pub const DEFAULT_MAX_VRM_FILE_BYTES: u64 = 50 * 1024 * 1024; // 50 MB
pub const DEFAULT_MAX_VRM_TRIANGLES: u64 = 100_000;
pub const DEFAULT_MAX_VRM_TEXTURES: usize = 16;
pub const DEFAULT_MAX_VRM_TEXTURE_DIMENSION: u32 = 2048;

const GLTF_MAGIC: [u8; 4] = [0x67, 0x6C, 0x54, 0x46]; // "glTF"
const CHUNK_TYPE_JSON: u32 = 0x4E4F534A; // "JSON"

/// Resource guardrails enforced during VRM model validation.
#[derive(Debug, Clone, Copy)]
pub struct VrmGuardrails {
    pub max_file_bytes: u64,
    pub max_triangles: u64,
    pub max_textures: usize,
    pub max_texture_dimension: u32,
    pub allow_external_uris: bool,
}

impl Default for VrmGuardrails {
    fn default() -> Self {
        Self {
            max_file_bytes: DEFAULT_MAX_VRM_FILE_BYTES,
            max_triangles: DEFAULT_MAX_VRM_TRIANGLES,
            max_textures: DEFAULT_MAX_VRM_TEXTURES,
            max_texture_dimension: DEFAULT_MAX_VRM_TEXTURE_DIMENSION,
            allow_external_uris: false,
        }
    }
}

/// Extracted VRM metadata (author, title, license).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VrmMetadata {
    pub title: String,
    pub author: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub license_name: Option<String>,
    #[serde(default)]
    pub allowed_user: Option<String>,
    #[serde(default)]
    pub commercial_usage: Option<bool>,
    pub triangle_count: u64,
    pub texture_count: usize,
}

#[derive(Deserialize)]
struct GltfHeaderDoc {
    #[serde(default)]
    buffers: Vec<GltfBufferDoc>,
    #[serde(default)]
    images: Vec<GltfImageDoc>,
    #[serde(default)]
    meshes: Vec<GltfMeshDoc>,
    #[serde(default)]
    accessors: Vec<GltfAccessorDoc>,
    #[serde(default)]
    extensions: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct GltfBufferDoc {
    #[serde(default)]
    uri: Option<String>,
    #[serde(rename = "byteLength")]
    _byte_length: Option<u64>,
}

#[derive(Deserialize)]
struct GltfImageDoc {
    #[serde(default)]
    uri: Option<String>,
    #[serde(rename = "bufferView")]
    _buffer_view: Option<u32>,
    #[serde(rename = "mimeType")]
    _mime_type: Option<String>,
}

#[derive(Deserialize)]
struct GltfMeshDoc {
    #[serde(default)]
    primitives: Vec<GltfPrimitiveDoc>,
}

#[derive(Deserialize)]
struct GltfPrimitiveDoc {
    #[serde(default)]
    indices: Option<usize>,
    #[serde(default)]
    attributes: std::collections::HashMap<String, usize>,
}

#[derive(Deserialize)]
struct GltfAccessorDoc {
    count: u64,
    #[serde(rename = "type")]
    _accessor_type: Option<String>,
}

/// Parses and validates a VRM file against guardrails.
pub fn validate_and_extract_vrm(
    vrm_path: &Path,
    guardrails: VrmGuardrails,
) -> Result<VrmMetadata, PetPackError> {
    let metadata = std::fs::symlink_metadata(vrm_path)?;
    if !metadata.file_type().is_file() {
        return Err(PetPackError::Invalid(
            "VRM path must be a regular file".into(),
        ));
    }
    let file_len = metadata.len();
    if file_len > guardrails.max_file_bytes {
        return Err(PetPackError::VrmBudgetExceeded(format!(
            "file size {file_len} bytes exceeds maximum {} bytes",
            guardrails.max_file_bytes
        )));
    }

    let file = File::open(vrm_path)?;
    let mut reader = BufReader::new(file);

    let mut header = [0_u8; 12];
    reader.read_exact(&mut header)?;
    if header[0..4] != GLTF_MAGIC {
        return Err(PetPackError::Invalid(
            "file is not a valid glTF binary (VRM)".into(),
        ));
    }
    let version = u32::from_le_bytes(header[4..8].try_into().unwrap());
    if version != 2 {
        return Err(PetPackError::Invalid(format!(
            "unsupported glTF binary version {version}"
        )));
    }

    // Read chunk 0 (JSON)
    let mut chunk_header = [0_u8; 8];
    reader.read_exact(&mut chunk_header)?;
    let chunk_len = u32::from_le_bytes(chunk_header[0..4].try_into().unwrap()) as usize;
    let chunk_type = u32::from_le_bytes(chunk_header[4..8].try_into().unwrap());
    if chunk_type != CHUNK_TYPE_JSON {
        return Err(PetPackError::Invalid(
            "expected JSON chunk as first glTF block".into(),
        ));
    }

    let mut json_bytes = vec![0_u8; chunk_len];
    reader.read_exact(&mut json_bytes)?;
    let doc: GltfHeaderDoc = serde_json::from_slice(&json_bytes)?;

    // Check external resource guardrails
    if !guardrails.allow_external_uris {
        for buf in &doc.buffers {
            if let Some(uri) = &buf.uri {
                if !uri.starts_with("data:") {
                    return Err(PetPackError::VrmExternalResource(uri.clone()));
                }
            }
        }
        for img in &doc.images {
            if let Some(uri) = &img.uri {
                return Err(PetPackError::VrmExternalResource(uri.clone()));
            }
        }
    }

    // Check texture count guardrails
    let texture_count = doc.images.len();
    if texture_count > guardrails.max_textures {
        return Err(PetPackError::VrmBudgetExceeded(format!(
            "texture count {texture_count} exceeds maximum {}",
            guardrails.max_textures
        )));
    }

    // Estimate triangle count from primitives
    let mut triangle_count = 0_u64;
    for mesh in &doc.meshes {
        for prim in &mesh.primitives {
            if let Some(indices_idx) = prim.indices {
                if let Some(accessor) = doc.accessors.get(indices_idx) {
                    triangle_count = triangle_count.saturating_add(accessor.count / 3);
                }
            } else if let Some(pos_idx) = prim.attributes.get("POSITION") {
                if let Some(accessor) = doc.accessors.get(*pos_idx) {
                    triangle_count = triangle_count.saturating_add(accessor.count / 3);
                }
            }
        }
    }

    if triangle_count > guardrails.max_triangles {
        return Err(PetPackError::VrmBudgetExceeded(format!(
            "triangle count {triangle_count} exceeds limit {}",
            guardrails.max_triangles
        )));
    }

    // Extract metadata from VRMC_vrm or VRM extension
    let mut title = "Untitled Avatar".to_string();
    let mut author = "Unknown Author".to_string();
    let mut version = None;
    let mut license_name = None;
    let mut allowed_user = None;
    let mut commercial_usage = None;

    if let Some(extensions) = &doc.extensions {
        // Check VRM 1.0 (VRMC_vrm)
        if let Some(vrmc) = extensions.get("VRMC_vrm") {
            if let Some(meta) = vrmc.get("meta") {
                if let Some(t) = meta.get("name").and_then(|v| v.as_str()) {
                    title = t.to_string();
                }
                if let Some(a) = meta.get("authors").and_then(|v| v.as_array()) {
                    let names: Vec<_> = a.iter().filter_map(|x| x.as_str()).collect();
                    if !names.is_empty() {
                        author = names.join(", ");
                    }
                }
                version = meta
                    .get("version")
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string);
                license_name = meta
                    .get("licenseUrl")
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string);
                commercial_usage = meta
                    .get("commercialUsage")
                    .and_then(|v| v.as_str())
                    .map(|s| s != "personalNonProfit");
            }
        }
        // Check VRM 0.x (VRM)
        else if let Some(vrm0) = extensions.get("VRM") {
            if let Some(meta) = vrm0.get("meta") {
                if let Some(t) = meta.get("title").and_then(|v| v.as_str()) {
                    title = t.to_string();
                }
                if let Some(a) = meta.get("author").and_then(|v| v.as_str()) {
                    author = a.to_string();
                }
                version = meta
                    .get("version")
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string);
                license_name = meta
                    .get("licenseName")
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string);
                allowed_user = meta
                    .get("allowedUserName")
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string);
                commercial_usage = meta
                    .get("commercialUsaName")
                    .and_then(|v| v.as_str())
                    .map(|s| s == "Allow");
            }
        }
    }

    Ok(VrmMetadata {
        title,
        author,
        version,
        license_name,
        allowed_user,
        commercial_usage,
        triangle_count,
        texture_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_dummy_glb(json_str: &str) -> Vec<u8> {
        let mut buf = Vec::new();
        let json_bytes = json_str.as_bytes();
        let padded_json_len = (json_bytes.len() + 3) & !3;
        let mut padded_json = json_bytes.to_vec();
        padded_json.resize(padded_json_len, b' ');

        let total_len = 12 + 8 + padded_json_len as u32;

        // Header
        buf.extend_from_slice(&GLTF_MAGIC);
        buf.extend_from_slice(&2_u32.to_le_bytes());
        buf.extend_from_slice(&total_len.to_le_bytes());

        // Chunk 0 (JSON)
        buf.extend_from_slice(&(padded_json_len as u32).to_le_bytes());
        buf.extend_from_slice(&CHUNK_TYPE_JSON.to_le_bytes());
        buf.extend_from_slice(&padded_json);

        buf
    }

    #[test]
    fn validates_clean_vrm_and_extracts_metadata() {
        let json = r#"{
            "asset": {"version": "2.0"},
            "buffers": [{"byteLength": 1024}],
            "images": [{"bufferView": 0, "mimeType": "image/png"}],
            "meshes": [{
                "primitives": [{"indices": 0}]
            }],
            "accessors": [{"count": 900, "type": "SCALAR"}],
            "extensions": {
                "VRM": {
                    "meta": {
                        "title": "Aoi",
                        "author": "AgentPet Team",
                        "version": "1.0",
                        "licenseName": "CC-BY-4.0"
                    }
                }
            }
        }"#;

        let glb_bytes = create_dummy_glb(json);
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(&glb_bytes).unwrap();

        let meta = validate_and_extract_vrm(temp.path(), VrmGuardrails::default()).unwrap();
        assert_eq!(meta.title, "Aoi");
        assert_eq!(meta.author, "AgentPet Team");
        assert_eq!(meta.triangle_count, 300);
        assert_eq!(meta.texture_count, 1);
    }

    #[test]
    fn rejects_external_uri_in_buffers() {
        let json = r#"{
            "asset": {"version": "2.0"},
            "buffers": [{"uri": "https://malicious.site/data.bin", "byteLength": 1024}]
        }"#;

        let glb_bytes = create_dummy_glb(json);
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(&glb_bytes).unwrap();

        let err = validate_and_extract_vrm(temp.path(), VrmGuardrails::default()).unwrap_err();
        assert!(matches!(err, PetPackError::VrmExternalResource(_)));
    }

    #[test]
    fn rejects_excessive_triangle_count() {
        let json = r#"{
            "asset": {"version": "2.0"},
            "meshes": [{
                "primitives": [{"indices": 0}]
            }],
            "accessors": [{"count": 360000, "type": "SCALAR"}]
        }"#;

        let glb_bytes = create_dummy_glb(json);
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(&glb_bytes).unwrap();

        let guardrails = VrmGuardrails {
            max_triangles: 100_000,
            ..Default::default()
        };
        let err = validate_and_extract_vrm(temp.path(), guardrails).unwrap_err();
        assert!(matches!(err, PetPackError::VrmBudgetExceeded(_)));
    }
}

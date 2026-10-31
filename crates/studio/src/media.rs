//! Validation of untrusted media: user input images and provider model outputs.

use std::io::Cursor;

use base64::Engine;
use image::{ImageFormat, ImageReader};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::provider::ExportFormat;

const MAX_IMAGE_DIMENSION: u32 = 8192;
const MAX_IMAGE_DECODE_ALLOC: u64 = 512 * 1024 * 1024;
const RECOMMENDED_MIN_DIMENSION: u32 = 256;
const PREVIEW_EDGE: u32 = 384;
const GLB_MAGIC: u32 = 0x4654_6C67;
const GLB_JSON_CHUNK: u32 = 0x4E4F_534A;
const GLB_MAX_JSON_BYTES: usize = 64 * 1024 * 1024;
const FBX_BINARY_MAGIC: &[u8] = b"Kaydara FBX Binary  \0";
const GLTF_TRIANGLES_MODE: u64 = 4;

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageCheck {
    pub mime: &'static str,
    pub extension: &'static str,
    pub width: u32,
    pub height: u32,
    pub byte_size: u64,
    pub sha256: String,
    pub warnings: Vec<String>,
}

/// Validates format by content (not extension), size, dimensions, and full decode.
pub fn validate_image(bytes: &[u8], max_bytes: u64) -> Result<ImageCheck, String> {
    if bytes.is_empty() {
        return Err("The image file is empty.".into());
    }
    if bytes.len() as u64 > max_bytes {
        return Err(format!(
            "The image is {:.1} MB; the limit is {:.0} MB.",
            bytes.len() as f64 / 1_048_576.0,
            max_bytes as f64 / 1_048_576.0
        ));
    }
    let format = image::guess_format(bytes)
        .map_err(|_| "Unrecognized image format. Use PNG, JPEG or WebP.".to_string())?;
    let (mime, extension) = match format {
        ImageFormat::Png => ("image/png", "png"),
        ImageFormat::Jpeg => ("image/jpeg", "jpg"),
        ImageFormat::WebP => ("image/webp", "webp"),
        other => {
            return Err(format!(
                "{other:?} images are not supported. Use PNG, JPEG or WebP."
            ))
        }
    };
    let decoded = decode(bytes, format)
        .map_err(|error| format!("The image is corrupt or unreadable: {error}"))?;
    let (width, height) = (decoded.width(), decoded.height());
    let mut warnings = Vec::new();
    if width.min(height) < RECOMMENDED_MIN_DIMENSION {
        warnings.push(format!(
            "Image is {width}x{height}; at least {RECOMMENDED_MIN_DIMENSION}x{RECOMMENDED_MIN_DIMENSION} is recommended."
        ));
    }
    Ok(ImageCheck {
        mime,
        extension,
        width,
        height,
        byte_size: bytes.len() as u64,
        sha256: sha256_hex(bytes),
        warnings,
    })
}

fn decode(bytes: &[u8], format: ImageFormat) -> Result<image::DynamicImage, image::ImageError> {
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_DIMENSION);
    limits.max_image_height = Some(MAX_IMAGE_DIMENSION);
    limits.max_alloc = Some(MAX_IMAGE_DECODE_ALLOC);
    reader.limits(limits);
    reader.decode()
}

/// Working copy for upload. The provider's file endpoint documents JPEG/PNG
/// only, so WebP is re-encoded to PNG; the original is never modified.
pub fn upload_copy(
    bytes: &[u8],
    check: &ImageCheck,
) -> Result<(Vec<u8>, &'static str, &'static str), String> {
    if check.mime != "image/webp" {
        return Ok((bytes.to_vec(), check.mime, check.extension));
    }
    let decoded = decode(bytes, ImageFormat::WebP).map_err(|error| error.to_string())?;
    let mut png = Vec::new();
    decoded
        .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
        .map_err(|error| error.to_string())?;
    Ok((png, "image/png", "png"))
}

/// Small PNG data URL for UI previews, so full images never cross IPC.
pub fn preview_data_url(bytes: &[u8]) -> Result<String, String> {
    let format = image::guess_format(bytes).map_err(|error| error.to_string())?;
    let thumbnail = decode(bytes, format)
        .map_err(|error| error.to_string())?
        .thumbnail(PREVIEW_EDGE, PREVIEW_EDGE);
    let mut png = Vec::new();
    thumbnail
        .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
        .map_err(|error| error.to_string())?;
    Ok(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png)
    ))
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PbrChannels {
    pub base_color: bool,
    pub metallic_roughness: bool,
    pub normal: bool,
    pub occlusion: bool,
    pub emissive: bool,
}

/// Facts read from the file itself. `None` = not determinable for this format.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelReport {
    pub format: &'static str,
    pub byte_size: u64,
    pub meshes: Option<u64>,
    pub vertices: Option<u64>,
    pub triangles: Option<u64>,
    pub materials: Option<u64>,
    pub textures: Option<u64>,
    pub images: Option<u64>,
    pub animations: Option<u64>,
    pub skins: Option<u64>,
    pub has_uv: Option<bool>,
    /// Per-vertex colours (`COLOR_0`), used instead of texture maps by some engines.
    pub has_vertex_colors: Option<bool>,
    pub pbr: Option<PbrChannels>,
    pub extensions_required: Vec<String>,
}

pub fn inspect_model(bytes: &[u8]) -> Result<ModelReport, String> {
    if bytes.starts_with(FBX_BINARY_MAGIC) || bytes.starts_with(b"; FBX") {
        return Ok(ModelReport {
            format: "fbx",
            byte_size: bytes.len() as u64,
            ..ModelReport::default()
        });
    }
    inspect_glb(bytes)
}

const ZIP_MAGIC: &[u8] = b"PK\x03\x04";
const ARCHIVE_MAX_ENTRIES: usize = 1000;
const ARCHIVE_LISTED_ENTRIES: usize = 50;
const STL_HEADER_BYTES: usize = 84;
const STL_TRIANGLE_BYTES: u64 = 50;

/// What a converted file turned out to be. Read from content, never from the URL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportCheck {
    pub container: &'static str,
    /// File extension to save with.
    pub extension: &'static str,
    pub byte_size: u64,
    /// Archive member names (first entries only) when the output is an archive.
    pub entries: Vec<String>,
}

/// Validates a provider conversion output against the requested format.
/// Archives are listed from the central directory only; nothing is extracted.
pub fn inspect_export(bytes: &[u8], format: ExportFormat) -> Result<ExportCheck, String> {
    let check = |container, extension, entries| {
        Ok(ExportCheck {
            container,
            extension,
            byte_size: bytes.len() as u64,
            entries,
        })
    };
    if bytes.starts_with(ZIP_MAGIC) {
        let archive = zip::ZipArchive::new(Cursor::new(bytes))
            .map_err(|_| "Converted archive is corrupt.".to_string())?;
        if archive.len() > ARCHIVE_MAX_ENTRIES {
            return Err("Converted archive has too many entries.".into());
        }
        let names: Vec<String> = archive.file_names().map(str::to_owned).collect();
        if names
            .iter()
            .any(|name| name.starts_with('/') || name.split(['/', '\\']).any(|part| part == ".."))
        {
            return Err("Converted archive contains unsafe paths.".into());
        }
        let lower: Vec<String> = names.iter().map(|name| name.to_ascii_lowercase()).collect();
        let has = |suffixes: &[&str]| {
            lower
                .iter()
                .any(|name| suffixes.iter().any(|suffix| name.ends_with(suffix)))
        };
        let listed = names.iter().take(ARCHIVE_LISTED_ENTRIES).cloned().collect();
        return match format {
            ExportFormat::Usdz if has(&[".usd", ".usda", ".usdc"]) => check("usdz", "usdz", listed),
            ExportFormat::ThreeMf if lower.iter().any(|name| name == "3d/3dmodel.model") => {
                check("3mf", "3mf", listed)
            }
            ExportFormat::Gltf if has(&[".gltf", ".glb"]) => check("zip", "zip", listed),
            ExportFormat::Obj if has(&[".obj"]) => check("zip", "zip", listed),
            ExportFormat::Fbx if has(&[".fbx"]) => check("zip", "zip", listed),
            ExportFormat::Stl if has(&[".stl"]) => check("zip", "zip", listed),
            _ => Err(format!(
                "Converted archive contains no {} file.",
                format.as_str()
            )),
        };
    }
    match format {
        ExportFormat::Fbx if bytes.starts_with(FBX_BINARY_MAGIC) || bytes.starts_with(b"; FBX") => {
            check("fbx", "fbx", Vec::new())
        }
        ExportFormat::Gltf if read_u32(bytes, 0) == Some(GLB_MAGIC) => {
            inspect_glb(bytes)?;
            check("glb", "glb", Vec::new())
        }
        ExportFormat::Gltf
            if serde_json::from_slice::<Value>(bytes)
                .is_ok_and(|json| json.get("asset").is_some()) =>
        {
            check("gltf", "gltf", Vec::new())
        }
        ExportFormat::Obj
            if std::str::from_utf8(bytes)
                .is_ok_and(|text| text.lines().any(|line| line.trim_start().starts_with("v "))) =>
        {
            check("obj", "obj", Vec::new())
        }
        ExportFormat::Stl if is_stl(bytes) => check("stl", "stl", Vec::new()),
        _ => Err(format!(
            "Converted file is not a valid {} model.",
            format.as_str()
        )),
    }
}

fn is_stl(bytes: &[u8]) -> bool {
    if bytes.len() >= STL_HEADER_BYTES {
        if let Some(count) = read_u32(bytes, 80) {
            if STL_HEADER_BYTES as u64 + u64::from(count) * STL_TRIANGLE_BYTES == bytes.len() as u64
            {
                return true;
            }
        }
    }
    bytes.starts_with(b"solid")
        && std::str::from_utf8(bytes).is_ok_and(|text| text.contains("endsolid"))
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    bytes
        .get(offset..offset + 4)
        .map(|slice| u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn inspect_glb(bytes: &[u8]) -> Result<ModelReport, String> {
    if read_u32(bytes, 0) != Some(GLB_MAGIC) {
        return Err("Output is neither a GLB nor an FBX model.".into());
    }
    if read_u32(bytes, 4) != Some(2) {
        return Err("Unsupported glTF container version.".into());
    }
    if read_u32(bytes, 8).map(|length| length as usize) != Some(bytes.len()) {
        return Err("GLB is truncated: declared length does not match file size.".into());
    }
    let json_length = read_u32(bytes, 12).ok_or("GLB has no JSON chunk.")? as usize;
    if read_u32(bytes, 16) != Some(GLB_JSON_CHUNK) || json_length > GLB_MAX_JSON_BYTES {
        return Err("GLB JSON chunk is missing or too large.".into());
    }
    let json_bytes = bytes
        .get(20..20 + json_length)
        .ok_or("GLB JSON chunk is truncated.")?;
    let json: Value = serde_json::from_slice(json_bytes)
        .map_err(|_| "GLB JSON chunk is not valid JSON.".to_string())?;

    let array_len = |key: &str| {
        json.get(key)
            .and_then(Value::as_array)
            .map_or(0, |items| items.len() as u64)
    };
    let accessor_count = |index: Option<&Value>| {
        index.and_then(Value::as_u64).and_then(|index| {
            json.get("accessors")?
                .get(index as usize)?
                .get("count")?
                .as_u64()
        })
    };
    let (mut vertices, mut triangles, mut has_uv, mut has_colors) = (0u64, 0u64, false, false);
    for primitive in json
        .get("meshes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|mesh| mesh.get("primitives")?.as_array())
        .flatten()
    {
        let attributes = primitive.get("attributes");
        let positions =
            accessor_count(attributes.and_then(|attributes| attributes.get("POSITION")))
                .unwrap_or(0);
        vertices = vertices.saturating_add(positions);
        has_uv |= attributes
            .and_then(|attributes| attributes.get("TEXCOORD_0"))
            .is_some();
        has_colors |= attributes
            .and_then(|attributes| attributes.get("COLOR_0"))
            .is_some();
        if primitive
            .get("mode")
            .and_then(Value::as_u64)
            .unwrap_or(GLTF_TRIANGLES_MODE)
            == GLTF_TRIANGLES_MODE
        {
            let corners = accessor_count(primitive.get("indices")).unwrap_or(positions);
            triangles = triangles.saturating_add(corners / 3);
        }
    }
    let mut pbr = PbrChannels::default();
    for material in json
        .get("materials")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let metallic = material.get("pbrMetallicRoughness");
        pbr.base_color |= metallic
            .and_then(|value| value.get("baseColorTexture"))
            .is_some();
        pbr.metallic_roughness |= metallic
            .and_then(|value| value.get("metallicRoughnessTexture"))
            .is_some();
        pbr.normal |= material.get("normalTexture").is_some();
        pbr.occlusion |= material.get("occlusionTexture").is_some();
        pbr.emissive |= material.get("emissiveTexture").is_some();
    }
    let extensions_required = json
        .get("extensionsRequired")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    Ok(ModelReport {
        format: "glb",
        byte_size: bytes.len() as u64,
        meshes: Some(array_len("meshes")),
        vertices: Some(vertices),
        triangles: Some(triangles),
        materials: Some(array_len("materials")),
        textures: Some(array_len("textures")),
        images: Some(array_len("images")),
        animations: Some(array_len("animations")),
        skins: Some(array_len("skins")),
        has_uv: Some(has_uv),
        has_vertex_colors: Some(has_colors),
        pbr: Some(pbr),
        extensions_required,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn png_bytes(width: u32, height: u32) -> Vec<u8> {
        let image = image::RgbaImage::from_pixel(width, height, image::Rgba([200, 30, 30, 255]));
        let mut out = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut out), ImageFormat::Png)
            .unwrap();
        out
    }

    pub(crate) fn glb_bytes(json: &str) -> Vec<u8> {
        let mut json = json.as_bytes().to_vec();
        while !json.len().is_multiple_of(4) {
            json.push(b' ');
        }
        let total = 12 + 8 + json.len();
        let mut out = Vec::new();
        out.extend_from_slice(&GLB_MAGIC.to_le_bytes());
        out.extend_from_slice(&2u32.to_le_bytes());
        out.extend_from_slice(&(total as u32).to_le_bytes());
        out.extend_from_slice(&(json.len() as u32).to_le_bytes());
        out.extend_from_slice(&GLB_JSON_CHUNK.to_le_bytes());
        out.extend_from_slice(&json);
        out
    }

    pub(crate) const SAMPLE_GLTF: &str = r#"{"asset":{"version":"2.0"},
        "accessors":[{"count":8},{"count":36},{"count":8}],
        "meshes":[{"primitives":[{"attributes":{"POSITION":0,"TEXCOORD_0":2},"indices":1,"material":0}]}],
        "materials":[{"pbrMetallicRoughness":{"baseColorTexture":{"index":0},"metallicRoughnessTexture":{"index":1}},"normalTexture":{"index":2}}],
        "textures":[{},{},{}],"images":[{},{},{}],
        "extensionsRequired":["EXT_meshopt_compression"]}"#;

    #[test]
    fn accepts_valid_png_and_warns_on_small_images() {
        let check = validate_image(&png_bytes(300, 300), 1 << 20).unwrap();
        assert_eq!(
            (check.mime, check.width, check.height),
            ("image/png", 300, 300)
        );
        assert!(check.warnings.is_empty());
        assert_eq!(check.sha256.len(), 64);
        assert_eq!(
            validate_image(&png_bytes(64, 64), 1 << 20)
                .unwrap()
                .warnings
                .len(),
            1
        );
    }

    #[test]
    fn rejects_empty_oversized_unsupported_and_corrupt_images() {
        assert!(validate_image(&[], 10).is_err());
        assert!(validate_image(&png_bytes(300, 300), 10)
            .unwrap_err()
            .contains("limit"));
        assert!(validate_image(b"GIF89a\x01\x00\x01\x00", 1 << 20)
            .unwrap_err()
            .contains("not supported"));
        let mut corrupt = png_bytes(300, 300);
        corrupt.truncate(corrupt.len() / 2);
        assert!(validate_image(&corrupt, 1 << 20)
            .unwrap_err()
            .contains("corrupt"));
        assert!(validate_image(b"plain text", 1 << 20).is_err());
    }

    #[test]
    fn webp_upload_copy_is_png_and_original_untouched() {
        let mut webp = Vec::new();
        image::RgbaImage::from_pixel(300, 300, image::Rgba([1, 2, 3, 255]))
            .write_to(&mut Cursor::new(&mut webp), ImageFormat::WebP)
            .unwrap();
        let original = webp.clone();
        let check = validate_image(&webp, 1 << 20).unwrap();
        let (copy, mime, extension) = upload_copy(&webp, &check).unwrap();
        assert_eq!((mime, extension), ("image/png", "png"));
        assert_eq!(image::guess_format(&copy).unwrap(), ImageFormat::Png);
        assert_eq!(webp, original);
        assert!(preview_data_url(&webp)
            .unwrap()
            .starts_with("data:image/png;base64,"));
    }

    #[test]
    fn inspects_glb_geometry_materials_and_pbr_from_file_data() {
        let report = inspect_model(&glb_bytes(SAMPLE_GLTF)).unwrap();
        assert_eq!(report.format, "glb");
        assert_eq!(report.vertices, Some(8));
        assert_eq!(report.triangles, Some(12));
        assert_eq!(report.materials, Some(1));
        assert_eq!(report.has_uv, Some(true));
        assert_eq!(report.has_vertex_colors, Some(false));
        let pbr = report.pbr.unwrap();
        assert!(pbr.base_color && pbr.metallic_roughness && pbr.normal);
        assert!(!pbr.occlusion, "absent channels are never claimed");
        assert_eq!(report.extensions_required, vec!["EXT_meshopt_compression"]);
    }

    #[test]
    fn rejects_truncated_or_foreign_model_files() {
        let mut glb = glb_bytes(SAMPLE_GLTF);
        glb.pop();
        assert!(inspect_model(&glb).unwrap_err().contains("truncated"));
        assert!(inspect_model(b"<html>expired</html>").is_err());
        assert!(inspect_model(&glb_bytes("{not json")).is_err());
        assert_eq!(
            inspect_model(b"Kaydara FBX Binary  \0rest").unwrap().format,
            "fbx"
        );
    }

    pub(crate) fn zip_bytes(names: &[&str]) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for name in names {
            writer
                .start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            std::io::Write::write_all(&mut writer, b"data").unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    #[test]
    fn export_validation_detects_container_from_content() {
        let usdz =
            inspect_export(&zip_bytes(&["model.usdc", "tex/0.png"]), ExportFormat::Usdz).unwrap();
        assert_eq!((usdz.container, usdz.extension), ("usdz", "usdz"));
        assert_eq!(usdz.entries, vec!["model.usdc", "tex/0.png"]);
        let three_mf =
            inspect_export(&zip_bytes(&["3D/3dmodel.model"]), ExportFormat::ThreeMf).unwrap();
        assert_eq!(three_mf.extension, "3mf");
        let obj_zip = inspect_export(&zip_bytes(&["m.obj", "m.mtl"]), ExportFormat::Obj).unwrap();
        assert_eq!(obj_zip.extension, "zip");
        assert_eq!(
            inspect_export(&glb_bytes(SAMPLE_GLTF), ExportFormat::Gltf)
                .unwrap()
                .extension,
            "glb"
        );
        assert_eq!(
            inspect_export(br#"{"asset":{"version":"2.0"}}"#, ExportFormat::Gltf)
                .unwrap()
                .extension,
            "gltf"
        );
        assert_eq!(
            inspect_export(b"# obj\nv 0 0 0\nf 1 1 1\n", ExportFormat::Obj)
                .unwrap()
                .extension,
            "obj"
        );
        let mut stl = vec![0u8; 80];
        stl.extend_from_slice(&1u32.to_le_bytes());
        stl.extend_from_slice(&[0u8; 50]);
        assert_eq!(
            inspect_export(&stl, ExportFormat::Stl).unwrap().extension,
            "stl"
        );
        assert_eq!(
            inspect_export(b"Kaydara FBX Binary  \0x", ExportFormat::Fbx)
                .unwrap()
                .extension,
            "fbx"
        );
    }

    #[test]
    fn export_validation_rejects_mismatched_corrupt_or_unsafe_outputs() {
        assert!(inspect_export(&zip_bytes(&["m.obj"]), ExportFormat::Usdz).is_err());
        assert!(inspect_export(&zip_bytes(&["../evil.obj"]), ExportFormat::Obj).is_err());
        assert!(inspect_export(b"PK\x03\x04garbage", ExportFormat::Usdz).is_err());
        assert!(inspect_export(b"<html>expired</html>", ExportFormat::Fbx).is_err());
        assert!(inspect_export(&[0u8; 90], ExportFormat::Stl).is_err());
        assert!(inspect_export(b"{\"no\":1}", ExportFormat::Gltf).is_err());
    }
}

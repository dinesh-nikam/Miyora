//! Provider abstraction. UI and services talk to `GenerationProvider`; only
//! provider modules (e.g. `tripo`) know concrete endpoints.

use serde::{Deserialize, Serialize};

use crate::error::ProviderError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Capability {
    TextTo3d,
    ImageTo3d,
    MultiviewTo3d,
    TextToImage,
    ImageToImage,
    Texture,
    Segmentation,
    MeshCompletion,
    Retopology,
    RigCheck,
    Rig,
    Retarget,
    Convert,
    TaskStatus,
    TaskList,
    Upload,
    AccountBalance,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: String,
    pub label: String,
    /// Where this entry came from; Tripo exposes no model-list endpoint.
    pub source: &'static str,
    pub supports_geometry_quality: bool,
    /// Upper face limit for standard geometry quality.
    pub max_faces_standard: u32,
    /// Upper face limit for detailed geometry quality, when supported.
    pub max_faces_detailed: Option<u32>,
}

/// Hard request limits published by the provider.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderLimits {
    pub prompt_max_chars: usize,
    pub negative_prompt_max_chars: usize,
    pub image_max_bytes: u64,
    pub quad_max_faces: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeometryQuality {
    Standard,
    Detailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextureQuality {
    Standard,
    Detailed,
    Extreme,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportOrientation {
    #[serde(rename = "+x")]
    PosX,
    #[serde(rename = "-x")]
    NegX,
    #[serde(rename = "+y")]
    PosY,
    #[serde(rename = "-y")]
    NegY,
}

/// Provider-neutral generation settings. `None` means "provider default".
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GenerationSettings {
    pub model: Option<String>,
    pub model_seed: Option<i64>,
    pub texture_seed: Option<i64>,
    pub image_seed: Option<i64>,
    pub face_limit: Option<u32>,
    pub texture: Option<bool>,
    pub pbr: Option<bool>,
    pub texture_quality: Option<TextureQuality>,
    pub geometry_quality: Option<GeometryQuality>,
    pub auto_size: Option<bool>,
    pub quad: Option<bool>,
    pub smart_low_poly: Option<bool>,
    pub export_uv: Option<bool>,
    pub compress_geometry: Option<bool>,
    pub export_orientation: Option<ExportOrientation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextTo3dRequest {
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub settings: GenerationSettings,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImageTo3dRequest {
    /// Provider file reference returned by `upload_image`.
    pub file_token: String,
    pub settings: GenerationSettings,
}

/// Canonical view order used by the UI, storage roles, and providers.
pub const VIEW_ROLES: [&str; 4] = ["front", "left", "back", "right"];

#[derive(Debug, Clone, PartialEq)]
pub struct MultiviewTo3dRequest {
    /// File tokens in `VIEW_ROLES` order; `None` skips a view. Front is required.
    pub views: [Option<String>; 4],
    pub settings: GenerationSettings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ExportFormat {
    Gltf,
    Fbx,
    Usdz,
    Obj,
    Stl,
    #[serde(rename = "3MF")]
    ThreeMf,
}

impl ExportFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Gltf => "GLTF",
            Self::Fbx => "FBX",
            Self::Usdz => "USDZ",
            Self::Obj => "OBJ",
            Self::Stl => "STL",
            Self::ThreeMf => "3MF",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TextureFormat {
    Jpeg,
    Png,
    Webp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FbxPreset {
    Blender,
    #[serde(rename = "3dsmax")]
    Max3ds,
    Mixamo,
}

/// Provider-neutral conversion options. `None` means "provider default".
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ConvertOptions {
    pub face_limit: Option<u32>,
    pub quad: Option<bool>,
    pub texture_size: Option<u32>,
    pub texture_format: Option<TextureFormat>,
    pub bake: Option<bool>,
    pub pack_uv: Option<bool>,
    pub pivot_to_center_bottom: Option<bool>,
    pub scale_factor: Option<f64>,
    pub with_animation: Option<bool>,
    pub export_orientation: Option<ExportOrientation>,
    pub fbx_preset: Option<FbxPreset>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConvertRequest {
    /// Provider task ID of the generation whose model is converted.
    pub source_task_id: String,
    pub format: ExportFormat,
    pub options: ConvertOptions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderStatus {
    Queued,
    Running,
    Success,
    Failed,
    Banned,
    Expired,
    Cancelled,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputKind {
    Model,
    PreviewImage,
}

impl OutputKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Model => "model",
            Self::PreviewImage => "preview_image",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputRef {
    pub kind: OutputKind,
    /// Provider field name, e.g. `model_url`. Stable per provider; used for idempotent storage.
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProviderTask {
    pub provider_task_id: String,
    pub status: ProviderStatus,
    /// Only set when the provider reports it.
    pub progress: Option<u8>,
    pub outputs: Vec<OutputRef>,
    pub credits_consumed: Option<f64>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountBalance {
    pub available: f64,
    pub frozen: f64,
}

/// Every `GenerationSettings` field name (camelCase), for providers that accept all of them.
pub const ALL_SETTINGS: &[&str] = &[
    "model",
    "modelSeed",
    "textureSeed",
    "imageSeed",
    "faceLimit",
    "texture",
    "pbr",
    "textureQuality",
    "geometryQuality",
    "autoSize",
    "quad",
    "smartLowPoly",
    "exportUv",
    "compressGeometry",
    "exportOrientation",
];

/// Names of settings that are set (non-null) but not in `supported`.
pub fn unsupported_settings(settings: &GenerationSettings, supported: &[&str]) -> Vec<String> {
    match serde_json::to_value(settings) {
        Ok(serde_json::Value::Object(fields)) => fields
            .into_iter()
            .filter(|(name, value)| !value.is_null() && !supported.contains(&name.as_str()))
            .map(|(name, _)| name)
            .collect(),
        _ => Vec::new(),
    }
}

pub trait GenerationProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn label(&self) -> &'static str;
    /// Cloud providers need a key; local engines do not.
    fn requires_api_key(&self) -> bool;
    /// Whether a stored key is used when present (optional keys raise free quotas).
    fn accepts_api_key(&self) -> bool {
        self.requires_api_key()
    }
    /// `Err(reason)` when the provider cannot run here (e.g. engine not installed).
    fn availability(&self) -> Result<(), String>;
    /// `GenerationSettings` fields (camelCase) this provider honours; others are rejected.
    fn supported_settings(&self) -> &'static [&'static str];
    fn capabilities(&self) -> &'static [Capability];
    fn models(&self) -> Vec<ModelInfo>;
    fn default_model(&self) -> &'static str;
    fn limits(&self) -> ProviderLimits;
    fn upload_image(
        &self,
        bytes: Vec<u8>,
        file_name: &str,
        mime: &str,
    ) -> Result<String, ProviderError>;
    fn text_to_3d(&self, request: &TextTo3dRequest) -> Result<String, ProviderError>;
    fn image_to_3d(&self, request: &ImageTo3dRequest) -> Result<String, ProviderError>;
    fn multiview_to_3d(&self, request: &MultiviewTo3dRequest) -> Result<String, ProviderError>;
    fn convert(&self, request: &ConvertRequest) -> Result<String, ProviderError>;
    fn task_status(&self, provider_task_id: &str) -> Result<ProviderTask, ProviderError>;
    fn account_balance(&self) -> Result<AccountBalance, ProviderError>;
    fn download(&self, url: &str, max_bytes: u64) -> Result<Vec<u8>, ProviderError>;
}

/// Provider task IDs are interpolated into URL paths, so restrict them.
pub fn is_valid_task_id(id: &str) -> bool {
    (1..=128).contains(&id.len())
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_ids_reject_path_injection() {
        assert!(is_valid_task_id("1ec04ced-4b87-44f6-a296-beee80777941"));
        assert!(is_valid_task_id("task_abc123"));
        assert!(!is_valid_task_id(""));
        assert!(!is_valid_task_id("../account/balance"));
        assert!(!is_valid_task_id("a?b=c"));
        assert!(!is_valid_task_id(&"a".repeat(129)));
    }

    #[test]
    fn settings_deserialize_from_camel_case_with_defaults() {
        let settings: GenerationSettings =
            serde_json::from_str(r#"{"faceLimit":5000,"exportOrientation":"-y"}"#).unwrap();
        assert_eq!(settings.face_limit, Some(5000));
        assert_eq!(settings.export_orientation, Some(ExportOrientation::NegY));
        assert_eq!(settings.pbr, None);
    }
}

//! Tripo API v3 provider. Contract source: https://developers.tripo3d.ai/en/docs
//! (retrieved 2026-10-29). Only documented endpoints are used.

use std::sync::Arc;

use serde_json::{json, Map, Value};

use crate::error::{ErrorKind, ProviderError};
use crate::provider::{
    is_valid_task_id, AccountBalance, Capability, ConvertOptions, ConvertRequest,
    GenerationProvider, GenerationSettings, GeometryQuality, ImageTo3dRequest, ModelInfo,
    MultiviewTo3dRequest, OutputKind, OutputRef, ProviderLimits, ProviderStatus, ProviderTask,
    TextTo3dRequest, TextureQuality, ALL_SETTINGS, VIEW_ROLES,
};
use crate::transport::{Body, HttpRequest, Method, Transport, TransportError};

pub const PROVIDER_ID: &str = "tripo";
pub const TRIPO_V3_BASE_URL: &str = "https://openapi.tripo3d.ai/v3";
pub const DEFAULT_MODEL: &str = "v3.1-20260211";

const API_RESPONSE_MAX_BYTES: u64 = 2 * 1024 * 1024;
const PROVIDER_MESSAGE_MAX_CHARS: usize = 300;
const FILE_TOKEN_MAX_CHARS: usize = 256;
const LIMITS: ProviderLimits = ProviderLimits {
    prompt_max_chars: 1024,
    negative_prompt_max_chars: 255,
    image_max_bytes: 20 * 1024 * 1024,
    quad_max_faces: 150_000,
};
/// Documented `smart_low_poly` face range (triangle mode).
const SMART_LOW_POLY_FACES: std::ops::RangeInclusive<u32> = 500..=20_000;

const MULTIVIEW_MIN_VIEWS: usize = 2;
/// Bounds this app enforces; the docs give only defaults (4096 px, scale 1).
const CONVERT_TEXTURE_SIZES: std::ops::RangeInclusive<u32> = 256..=8192;
const CONVERT_SCALE_RANGE: std::ops::RangeInclusive<f64> = 0.001..=1000.0;

/// Only capabilities this build actually implements are advertised.
const CAPABILITIES: &[Capability] = &[
    Capability::TextTo3d,
    Capability::ImageTo3d,
    Capability::MultiviewTo3d,
    Capability::Convert,
    Capability::TaskStatus,
    Capability::Upload,
    Capability::AccountBalance,
];

struct DocumentedModel {
    id: &'static str,
    label: &'static str,
    v3_features: bool,
    max_faces_standard: u32,
    max_faces_detailed: Option<u32>,
}

/// From the H-series text-to-model docs "UPPER LIMITS BY MODE" table.
const DOCUMENTED_MODELS: [DocumentedModel; 3] = [
    DocumentedModel {
        id: "v3.1-20260211",
        label: "v3.1 (latest, best quality)",
        v3_features: true,
        max_faces_standard: 1_500_000,
        max_faces_detailed: Some(2_000_000),
    },
    DocumentedModel {
        id: "v3.0-20250812",
        label: "v3.0 (stable)",
        v3_features: true,
        max_faces_standard: 1_000_000,
        max_faces_detailed: Some(2_000_000),
    },
    DocumentedModel {
        id: "v2.5-20250123",
        label: "v2.5 (balanced)",
        v3_features: false,
        max_faces_standard: 500_000,
        max_faces_detailed: None,
    },
];

pub struct TripoProvider {
    transport: Arc<dyn Transport>,
    api_key: String,
}

impl TripoProvider {
    pub fn new(transport: Arc<dyn Transport>, api_key: String) -> Self {
        Self { transport, api_key }
    }

    fn call(&self, method: Method, path: &str, body: Body) -> Result<Value, ProviderError> {
        let request = HttpRequest {
            method,
            url: format!("{TRIPO_V3_BASE_URL}{path}"),
            bearer: Some(self.api_key.clone()),
            body,
            long_running: false,
        };
        let response = self
            .transport
            .send(request, API_RESPONSE_MAX_BYTES)
            .map_err(|error| map_transport_error(error, ErrorKind::UnknownProvider))?;
        parse_envelope(response.status, &response.body, response.retry_after_secs)
    }

    fn submit(&self, path: &str, body: Map<String, Value>) -> Result<String, ProviderError> {
        let data = self.call(Method::Post, path, Body::Json(Value::Object(body)))?;
        data.get("task_id")
            .and_then(Value::as_str)
            .filter(|id| is_valid_task_id(id))
            .map(str::to_owned)
            .ok_or_else(|| {
                ProviderError::new(
                    ErrorKind::UnknownProvider,
                    "Provider accepted the request but returned no valid task ID.",
                )
            })
    }
}

impl GenerationProvider for TripoProvider {
    fn id(&self) -> &'static str {
        PROVIDER_ID
    }

    fn label(&self) -> &'static str {
        "Tripo API (cloud, paid credits)"
    }

    fn requires_api_key(&self) -> bool {
        true
    }

    fn availability(&self) -> Result<(), String> {
        Ok(())
    }

    fn supported_settings(&self) -> &'static [&'static str] {
        ALL_SETTINGS
    }

    fn capabilities(&self) -> &'static [Capability] {
        CAPABILITIES
    }

    fn models(&self) -> Vec<ModelInfo> {
        documented_models()
    }

    fn default_model(&self) -> &'static str {
        DEFAULT_MODEL
    }

    fn limits(&self) -> ProviderLimits {
        LIMITS
    }

    fn upload_image(
        &self,
        bytes: Vec<u8>,
        file_name: &str,
        mime: &str,
    ) -> Result<String, ProviderError> {
        let body = Body::Multipart {
            field: "file",
            file_name: file_name.into(),
            mime: mime.into(),
            bytes,
        };
        let data = self
            .call(Method::Post, "/files", body)
            .map_err(|mut error| {
                if matches!(
                    error.kind,
                    ErrorKind::Validation | ErrorKind::UnknownProvider
                ) {
                    error.kind = ErrorKind::FileUpload;
                }
                error
            })?;
        data.get("file_token")
            .and_then(Value::as_str)
            .filter(|token| is_valid_file_token(token))
            .map(str::to_owned)
            .ok_or_else(|| {
                ProviderError::new(
                    ErrorKind::FileUpload,
                    "Upload returned no valid file token.",
                )
            })
    }

    fn text_to_3d(&self, request: &TextTo3dRequest) -> Result<String, ProviderError> {
        validate_text(&request.prompt, LIMITS.prompt_max_chars, "Prompt", true)?;
        let mut body = settings_body(&request.settings)?;
        body.insert("prompt".into(), json!(request.prompt));
        if let Some(negative) = request
            .negative_prompt
            .as_deref()
            .filter(|text| !text.is_empty())
        {
            validate_text(
                negative,
                LIMITS.negative_prompt_max_chars,
                "Negative prompt",
                false,
            )?;
            body.insert("negative_prompt".into(), json!(negative));
        }
        if let Some(seed) = request.settings.image_seed {
            body.insert("image_seed".into(), json!(seed));
        }
        self.submit("/generation/text-to-model", body)
    }

    fn image_to_3d(&self, request: &ImageTo3dRequest) -> Result<String, ProviderError> {
        if !is_valid_file_token(&request.file_token) {
            return Err(ProviderError::new(
                ErrorKind::Validation,
                "Invalid file token.",
            ));
        }
        let mut body = settings_body(&request.settings)?;
        body.insert("input".into(), json!(request.file_token));
        self.submit("/generation/image-to-model", body)
    }

    /// Uses the documented "view-key" input form: `[{"front": token}, {"left": token}, ...]`.
    fn multiview_to_3d(&self, request: &MultiviewTo3dRequest) -> Result<String, ProviderError> {
        let reject = |message: &str| Err(ProviderError::new(ErrorKind::Validation, message));
        if request.views[0].is_none() {
            return reject("The front view is required.");
        }
        let mut inputs = Vec::new();
        for (role, token) in VIEW_ROLES.iter().zip(&request.views) {
            if let Some(token) = token {
                if !is_valid_file_token(token) {
                    return reject("Invalid file token.");
                }
                inputs.push(json!({ *role: token }));
            }
        }
        if inputs.len() < MULTIVIEW_MIN_VIEWS {
            return reject("Multiview generation needs at least 2 views.");
        }
        let mut body = settings_body(&request.settings)?;
        body.insert("inputs".into(), Value::Array(inputs));
        self.submit("/generation/multiview-to-model", body)
    }

    fn convert(&self, request: &ConvertRequest) -> Result<String, ProviderError> {
        if !is_valid_task_id(&request.source_task_id) {
            return Err(ProviderError::new(
                ErrorKind::Validation,
                "Invalid source task ID.",
            ));
        }
        let mut body = convert_body(&request.options)?;
        body.insert("input".into(), json!(request.source_task_id));
        body.insert("format".into(), json!(request.format.as_str()));
        self.submit("/models/convert", body)
    }

    fn task_status(&self, provider_task_id: &str) -> Result<ProviderTask, ProviderError> {
        if !is_valid_task_id(provider_task_id) {
            return Err(ProviderError::new(
                ErrorKind::Validation,
                "Invalid task ID.",
            ));
        }
        let data = self.call(
            Method::Get,
            &format!("/tasks/{provider_task_id}"),
            Body::Empty,
        )?;
        parse_task(provider_task_id, &data)
    }

    fn account_balance(&self) -> Result<AccountBalance, ProviderError> {
        let data = self.call(Method::Get, "/account/balance", Body::Empty)?;
        let number = |key: &str| data.get(key).and_then(Value::as_f64);
        match (number("balance"), number("frozen")) {
            (Some(available), Some(frozen)) => Ok(AccountBalance { available, frozen }),
            _ => Err(ProviderError::new(
                ErrorKind::UnknownProvider,
                "Balance response was missing fields.",
            )),
        }
    }

    fn download(&self, url: &str, max_bytes: u64) -> Result<Vec<u8>, ProviderError> {
        if !url.starts_with("https://") {
            return Err(ProviderError::new(
                ErrorKind::Conversion,
                "Provider output URL is not HTTPS.",
            ));
        }
        let request = HttpRequest {
            method: Method::Get,
            url: url.into(),
            bearer: None,
            body: Body::Empty,
            long_running: true,
        };
        let response = self
            .transport
            .send(request, max_bytes)
            .map_err(|error| map_transport_error(error, ErrorKind::Conversion))?;
        match response.status {
            200 => Ok(response.body),
            403 | 404 | 410 => Err(ProviderError::new(
                ErrorKind::Conversion,
                format!(
                    "Output file is no longer available (HTTP {}).",
                    response.status
                ),
            )),
            status => Err(ProviderError::new(
                ErrorKind::ProviderUnavailable,
                format!("Output download failed (HTTP {status})."),
            )),
        }
    }
}

pub fn documented_models() -> Vec<ModelInfo> {
    DOCUMENTED_MODELS
        .iter()
        .map(|model| ModelInfo {
            id: model.id.into(),
            label: model.label.into(),
            source: "documentation",
            supports_geometry_quality: model.v3_features,
            max_faces_standard: model.max_faces_standard,
            max_faces_detailed: model.max_faces_detailed,
        })
        .collect()
}

fn is_valid_file_token(token: &str) -> bool {
    (1..=FILE_TOKEN_MAX_CHARS).contains(&token.len())
        && token
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(&byte))
}

fn validate_text(
    text: &str,
    max_chars: usize,
    field: &str,
    required: bool,
) -> Result<(), ProviderError> {
    if required && text.trim().is_empty() {
        return Err(ProviderError::new(
            ErrorKind::Validation,
            format!("{field} is required."),
        ));
    }
    let count = text.chars().count();
    if count > max_chars {
        return Err(ProviderError::new(
            ErrorKind::Validation,
            format!("{field} is {count} characters; the provider limit is {max_chars}."),
        ));
    }
    Ok(())
}

fn validate_model_id(model: &str) -> Result<(), ProviderError> {
    let valid = (1..=64).contains(&model.len())
        && model
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b".-_".contains(&byte));
    if valid {
        Ok(())
    } else {
        Err(ProviderError::new(
            ErrorKind::Validation,
            "Model ID contains unsupported characters.",
        ))
    }
}

/// Maps provider-neutral settings onto documented Tripo fields, rejecting
/// combinations the docs say the API rejects.
fn settings_body(settings: &GenerationSettings) -> Result<Map<String, Value>, ProviderError> {
    let reject = |message: String| Err(ProviderError::new(ErrorKind::Validation, message));
    let model = settings.model.as_deref().unwrap_or(DEFAULT_MODEL);
    validate_model_id(model)?;
    let documented = DOCUMENTED_MODELS.iter().find(|entry| entry.id == model);
    let v3_features = documented.is_none_or(|entry| entry.v3_features);
    let uses_v3_only = settings.texture_quality.is_some()
        || settings.geometry_quality.is_some()
        || settings.auto_size.is_some()
        || settings.quad.is_some()
        || settings.smart_low_poly.is_some()
        || settings.compress_geometry.is_some();
    if !v3_features && uses_v3_only {
        return reject(
            "Quality, auto-size, quad, smart low-poly and compression need model v3.0 or newer."
                .into(),
        );
    }
    if let (Some(faces), Some(entry)) = (settings.face_limit, documented) {
        let max = match settings.geometry_quality {
            Some(GeometryQuality::Detailed) => {
                entry.max_faces_detailed.unwrap_or(entry.max_faces_standard)
            }
            _ => entry.max_faces_standard,
        };
        if faces == 0 || faces > max {
            return reject(format!(
                "Polygon budget must be between 1 and {max} for {model}."
            ));
        }
    }
    if settings.quad == Some(true)
        && settings
            .face_limit
            .is_some_and(|faces| faces > LIMITS.quad_max_faces)
    {
        return reject(format!(
            "Quad output allows at most {} faces.",
            LIMITS.quad_max_faces
        ));
    }
    if settings.smart_low_poly == Some(true)
        && settings
            .face_limit
            .is_some_and(|faces| !SMART_LOW_POLY_FACES.contains(&faces))
    {
        return reject("Smart low-poly needs a polygon budget between 500 and 20,000.".into());
    }

    let mut body = Map::new();
    body.insert("model".into(), json!(model));
    let mut put = |key: &str, value: Option<Value>| {
        if let Some(value) = value {
            body.insert(key.into(), value);
        }
    };
    put("model_seed", settings.model_seed.map(Value::from));
    put("texture_seed", settings.texture_seed.map(Value::from));
    put("face_limit", settings.face_limit.map(Value::from));
    put("texture", settings.texture.map(Value::from));
    put("pbr", settings.pbr.map(Value::from));
    put(
        "texture_quality",
        settings.texture_quality.map(|quality| {
            json!(match quality {
                TextureQuality::Standard => "standard",
                TextureQuality::Detailed => "detailed",
                TextureQuality::Extreme => "extreme",
            })
        }),
    );
    put(
        "geometry_quality",
        settings.geometry_quality.map(|quality| {
            json!(match quality {
                GeometryQuality::Standard => "standard",
                GeometryQuality::Detailed => "detailed",
            })
        }),
    );
    put("auto_size", settings.auto_size.map(Value::from));
    put("quad", settings.quad.map(Value::from));
    put("smart_low_poly", settings.smart_low_poly.map(Value::from));
    put("export_uv", settings.export_uv.map(Value::from));
    put(
        "compress",
        settings
            .compress_geometry
            .filter(|enabled| *enabled)
            .map(|_| json!("geometry")),
    );
    put(
        "export_orientation",
        settings
            .export_orientation
            .and_then(|orientation| serde_json::to_value(orientation).ok()),
    );
    Ok(body)
}

/// Maps conversion options onto documented `/models/convert` fields.
fn convert_body(options: &ConvertOptions) -> Result<Map<String, Value>, ProviderError> {
    let reject = |message: String| Err(ProviderError::new(ErrorKind::Validation, message));
    if options.face_limit == Some(0) {
        return reject("Polygon budget must be positive.".into());
    }
    if options.quad == Some(true)
        && options
            .face_limit
            .is_some_and(|faces| faces > LIMITS.quad_max_faces)
    {
        return reject(format!(
            "Quad output allows at most {} faces.",
            LIMITS.quad_max_faces
        ));
    }
    if options
        .texture_size
        .is_some_and(|size| !CONVERT_TEXTURE_SIZES.contains(&size))
    {
        return reject("Texture size must be between 256 and 8192 pixels.".into());
    }
    if options
        .scale_factor
        .is_some_and(|scale| !scale.is_finite() || !CONVERT_SCALE_RANGE.contains(&scale))
    {
        return reject("Scale factor must be between 0.001 and 1000.".into());
    }
    let to_value = |value: serde_json::Result<Value>| value.ok();
    let mut body = Map::new();
    let mut put = |key: &str, value: Option<Value>| {
        if let Some(value) = value {
            body.insert(key.into(), value);
        }
    };
    put("face_limit", options.face_limit.map(Value::from));
    put("quad", options.quad.map(Value::from));
    put("texture_size", options.texture_size.map(Value::from));
    put(
        "texture_format",
        options
            .texture_format
            .and_then(|format| to_value(serde_json::to_value(format))),
    );
    put("bake", options.bake.map(Value::from));
    put("pack_uv", options.pack_uv.map(Value::from));
    put(
        "pivot_to_center_bottom",
        options.pivot_to_center_bottom.map(Value::from),
    );
    put("scale_factor", options.scale_factor.map(Value::from));
    put("with_animation", options.with_animation.map(Value::from));
    put(
        "export_orientation",
        options
            .export_orientation
            .and_then(|value| to_value(serde_json::to_value(value))),
    );
    put(
        "fbx_preset",
        options
            .fbx_preset
            .and_then(|preset| to_value(serde_json::to_value(preset))),
    );
    Ok(body)
}

fn map_transport_error(error: TransportError, too_large_kind: ErrorKind) -> ProviderError {
    match error {
        TransportError::Timeout => ProviderError::new(
            ErrorKind::ProviderUnavailable,
            "The provider did not respond in time.",
        ),
        TransportError::Unreachable(_) => ProviderError::new(
            ErrorKind::ProviderUnavailable,
            "Could not reach the provider.",
        ),
        TransportError::TooLarge => {
            ProviderError::new(too_large_kind, "Provider response exceeded the size limit.")
        }
        TransportError::InvalidRequest(detail) => ProviderError::new(
            ErrorKind::Validation,
            format!("Request could not be built: {detail}"),
        ),
    }
}

fn truncate(text: &str) -> String {
    text.chars().take(PROVIDER_MESSAGE_MAX_CHARS).collect()
}

/// Separates HTTP success from API success: only `HTTP 200 && code == 0` yields data.
pub(crate) fn parse_envelope(
    status: u16,
    body: &[u8],
    retry_after_secs: Option<u64>,
) -> Result<Value, ProviderError> {
    let Ok(envelope) = serde_json::from_slice::<Value>(body) else {
        let kind = if status >= 500 {
            ErrorKind::ProviderUnavailable
        } else {
            ErrorKind::UnknownProvider
        };
        return Err(ProviderError::new(
            kind,
            format!("Provider returned an unreadable response (HTTP {status})."),
        ));
    };
    let code = envelope.get("code").and_then(Value::as_i64);
    if status == 200 && code == Some(0) {
        return envelope
            .get("data")
            .cloned()
            .filter(|data| !data.is_null())
            .ok_or_else(|| {
                ProviderError::new(ErrorKind::UnknownProvider, "Provider response had no data.")
            });
    }
    let text = |key: &str| envelope.get(key).and_then(Value::as_str).map(truncate);
    let message = match text("message") {
        Some(message) => format!("Provider rejected the request: {message}"),
        None => format!("Provider rejected the request (HTTP {status})."),
    };
    Err(ProviderError {
        kind: error_kind(status, code),
        message,
        provider_code: code,
        request_id: text("request_id"),
        retry_after_secs,
    })
}

/// Error-code table from https://developers.tripo3d.ai/en/docs/error-handling.
fn error_kind(status: u16, code: Option<i64>) -> ErrorKind {
    match code {
        Some(1000 | 1001) => ErrorKind::Authentication,
        Some(1007 | 2000) => ErrorKind::RateLimit,
        Some(2010) => ErrorKind::InsufficientCredits,
        Some(1004 | 2002 | 2003 | 2004 | 2008 | 2015 | 2018) => ErrorKind::Validation,
        _ => match status {
            401 | 403 => ErrorKind::Authentication,
            429 => ErrorKind::RateLimit,
            400 | 404 | 422 => ErrorKind::Validation,
            500..=599 => ErrorKind::ProviderUnavailable,
            _ => ErrorKind::UnknownProvider,
        },
    }
}

pub(crate) fn parse_task(requested_id: &str, data: &Value) -> Result<ProviderTask, ProviderError> {
    if data
        .get("task_id")
        .and_then(Value::as_str)
        .is_some_and(|id| id != requested_id)
    {
        return Err(ProviderError::new(
            ErrorKind::UnknownProvider,
            "Provider returned a different task than requested.",
        ));
    }
    let status = match data.get("status").and_then(Value::as_str) {
        Some("queued") => ProviderStatus::Queued,
        Some("running") => ProviderStatus::Running,
        Some("success") => ProviderStatus::Success,
        Some("failed") => ProviderStatus::Failed,
        Some("banned") => ProviderStatus::Banned,
        Some("expired") => ProviderStatus::Expired,
        Some("cancelled") => ProviderStatus::Cancelled,
        _ => ProviderStatus::Unknown,
    };
    let progress = data
        .get("progress")
        .and_then(Value::as_f64)
        .map(|value| value.clamp(0.0, 100.0).round() as u8);
    let mut outputs: Vec<OutputRef> = data
        .get("output")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter_map(|(key, value)| {
            let url = value.as_str()?;
            if !key.ends_with("_url") || !url.starts_with("https://") {
                return None;
            }
            let kind = if key.contains("image") {
                OutputKind::PreviewImage
            } else {
                OutputKind::Model
            };
            Some(OutputRef {
                kind,
                label: key.clone(),
                url: url.into(),
            })
        })
        .collect();
    outputs.sort_by(|left, right| left.label.cmp(&right.label));
    let error_message = match status {
        ProviderStatus::Banned => Some("The input violates the provider content policy.".into()),
        ProviderStatus::Expired => {
            Some("The task expired; its output files are no longer available.".into())
        }
        ProviderStatus::Failed => Some(
            data.get("error")
                .and_then(Value::as_str)
                .map(truncate)
                .unwrap_or_else(|| "The provider reported the generation as failed.".into()),
        ),
        _ => None,
    };
    Ok(ProviderTask {
        provider_task_id: requested_id.into(),
        status,
        progress,
        outputs,
        credits_consumed: data.get("credits_consumed").and_then(Value::as_f64),
        error_message,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use std::collections::VecDeque;
    use std::sync::Mutex;

    use super::*;
    use crate::provider::ExportOrientation;
    use crate::transport::HttpResponse;

    pub(crate) const CREATE: &str = include_str!("../tests/fixtures/tripo_v3/create_task.json");
    pub(crate) const SUCCESS: &str = include_str!("../tests/fixtures/tripo_v3/task_success.json");
    pub(crate) const UPLOAD: &str = include_str!("../tests/fixtures/tripo_v3/upload.json");
    const BALANCE: &str = include_str!("../tests/fixtures/tripo_v3/balance.json");
    const NO_CREDIT: &str =
        include_str!("../tests/fixtures/tripo_v3/error_insufficient_credits.json");

    pub(crate) struct Recorded {
        pub url: String,
        pub bearer: Option<String>,
        pub json: Option<Value>,
    }

    /// Replays documented responses in order; test-only, never compiled into release code.
    #[derive(Clone, Default)]
    pub(crate) struct ScriptedTransport {
        pub responses: Arc<Mutex<VecDeque<Result<HttpResponse, TransportError>>>>,
        pub requests: Arc<Mutex<Vec<Recorded>>>,
    }

    impl ScriptedTransport {
        pub(crate) fn push(&self, status: u16, body: &[u8]) {
            self.responses.lock().unwrap().push_back(Ok(HttpResponse {
                status,
                body: body.to_vec(),
                retry_after_secs: None,
            }));
        }

        pub(crate) fn push_error(&self, error: TransportError) {
            self.responses.lock().unwrap().push_back(Err(error));
        }
    }

    impl Transport for ScriptedTransport {
        fn send(&self, request: HttpRequest, _max: u64) -> Result<HttpResponse, TransportError> {
            let json = match &request.body {
                Body::Json(value) => Some(value.clone()),
                _ => None,
            };
            self.requests.lock().unwrap().push(Recorded {
                url: request.url,
                bearer: request.bearer,
                json,
            });
            self.responses
                .lock()
                .unwrap()
                .pop_front()
                .expect("unexpected request")
        }
    }

    fn provider(transport: &ScriptedTransport) -> TripoProvider {
        TripoProvider::new(Arc::new(transport.clone()), "tsk_test".into())
    }

    #[test]
    fn text_to_3d_posts_documented_fields_to_dedicated_endpoint() {
        let transport = ScriptedTransport::default();
        transport.push(200, CREATE.as_bytes());
        let request = TextTo3dRequest {
            prompt: "A low-poly wooden chest".into(),
            negative_prompt: Some("blurry".into()),
            settings: GenerationSettings {
                face_limit: Some(80_000),
                geometry_quality: Some(GeometryQuality::Detailed),
                texture_quality: Some(TextureQuality::Detailed),
                compress_geometry: Some(true),
                export_orientation: Some(ExportOrientation::NegY),
                model_seed: Some(7),
                ..GenerationSettings::default()
            },
        };
        assert_eq!(
            provider(&transport).text_to_3d(&request).unwrap(),
            "task_abc123"
        );
        let requests = transport.requests.lock().unwrap();
        assert_eq!(
            requests[0].url,
            "https://openapi.tripo3d.ai/v3/generation/text-to-model"
        );
        assert_eq!(requests[0].bearer.as_deref(), Some("tsk_test"));
        let body = requests[0].json.as_ref().unwrap();
        assert_eq!(body["model"], "v3.1-20260211");
        assert_eq!(body["prompt"], "A low-poly wooden chest");
        assert_eq!(body["negative_prompt"], "blurry");
        assert_eq!(body["face_limit"], 80_000);
        assert_eq!(body["geometry_quality"], "detailed");
        assert_eq!(body["texture_quality"], "detailed");
        assert_eq!(body["compress"], "geometry");
        assert_eq!(body["export_orientation"], "-y");
        assert_eq!(body["model_seed"], 7);
        assert!(
            body.get("type").is_none(),
            "v3 endpoints take no type field"
        );
        assert!(
            body.get("pbr").is_none(),
            "unset options use provider defaults"
        );
    }

    #[test]
    fn image_to_3d_sends_file_token_as_input() {
        let transport = ScriptedTransport::default();
        transport.push(200, CREATE.as_bytes());
        let request = ImageTo3dRequest {
            file_token: "file_abc123".into(),
            settings: GenerationSettings::default(),
        };
        provider(&transport).image_to_3d(&request).unwrap();
        let requests = transport.requests.lock().unwrap();
        assert!(requests[0].url.ends_with("/generation/image-to-model"));
        assert_eq!(requests[0].json.as_ref().unwrap()["input"], "file_abc123");
    }

    #[test]
    fn multiview_uses_view_key_inputs_and_requires_front_plus_one() {
        let transport = ScriptedTransport::default();
        transport.push(200, CREATE.as_bytes());
        let tripo = provider(&transport);
        let request = |views: [Option<&str>; 4]| MultiviewTo3dRequest {
            views: views.map(|view| view.map(str::to_owned)),
            settings: GenerationSettings::default(),
        };
        tripo
            .multiview_to_3d(&request([Some("file_f"), None, Some("file_b"), None]))
            .unwrap();
        {
            let requests = transport.requests.lock().unwrap();
            assert!(requests[0].url.ends_with("/generation/multiview-to-model"));
            assert_eq!(
                requests[0].json.as_ref().unwrap()["inputs"],
                json!([{ "front": "file_f" }, { "back": "file_b" }])
            );
        }
        assert!(tripo
            .multiview_to_3d(&request([None, Some("a"), Some("b"), None]))
            .is_err());
        assert!(tripo
            .multiview_to_3d(&request([Some("a"), None, None, None]))
            .is_err());
        assert!(tripo
            .multiview_to_3d(&request([Some("a"), Some("../x"), None, None]))
            .is_err());
        assert_eq!(transport.requests.lock().unwrap().len(), 1);
    }

    #[test]
    fn convert_posts_documented_fields_and_validates_options() {
        use crate::provider::{ExportFormat, FbxPreset, TextureFormat};
        let transport = ScriptedTransport::default();
        transport.push(200, CREATE.as_bytes());
        let tripo = provider(&transport);
        let options = ConvertOptions {
            texture_size: Some(2048),
            texture_format: Some(TextureFormat::Png),
            pivot_to_center_bottom: Some(true),
            fbx_preset: Some(FbxPreset::Max3ds),
            ..ConvertOptions::default()
        };
        let request = ConvertRequest {
            source_task_id: "task_abc123".into(),
            format: ExportFormat::Fbx,
            options,
        };
        tripo.convert(&request).unwrap();
        {
            let requests = transport.requests.lock().unwrap();
            assert!(requests[0].url.ends_with("/models/convert"));
            assert_eq!(
                requests[0].json.as_ref().unwrap(),
                &json!({
                    "input": "task_abc123",
                    "format": "FBX",
                    "texture_size": 2048,
                    "texture_format": "PNG",
                    "pivot_to_center_bottom": true,
                    "fbx_preset": "3dsmax"
                })
            );
        }
        let bad = |options: ConvertOptions| ConvertRequest {
            options,
            ..request.clone()
        };
        assert!(tripo
            .convert(&bad(ConvertOptions {
                texture_size: Some(100),
                ..Default::default()
            }))
            .is_err());
        assert!(tripo
            .convert(&bad(ConvertOptions {
                scale_factor: Some(f64::NAN),
                ..Default::default()
            }))
            .is_err());
        assert!(tripo
            .convert(&bad(ConvertOptions {
                quad: Some(true),
                face_limit: Some(200_000),
                ..Default::default()
            }))
            .is_err());
        assert!(tripo
            .convert(&ConvertRequest {
                source_task_id: "a/b".into(),
                ..request.clone()
            })
            .is_err());
        assert_eq!(transport.requests.lock().unwrap().len(), 1);
        assert_eq!(serde_json::to_value(ExportFormat::ThreeMf).unwrap(), "3MF");
    }

    #[test]
    fn request_validation_rejects_documented_invalid_combinations() {
        let transport = ScriptedTransport::default();
        let tripo = provider(&transport);
        let text = |prompt: &str, settings: GenerationSettings| {
            tripo.text_to_3d(&TextTo3dRequest {
                prompt: prompt.into(),
                negative_prompt: None,
                settings,
            })
        };
        assert_eq!(
            text(&"x".repeat(1025), GenerationSettings::default())
                .unwrap_err()
                .kind,
            ErrorKind::Validation
        );
        assert_eq!(
            text("  ", GenerationSettings::default()).unwrap_err().kind,
            ErrorKind::Validation
        );
        let v25_quality = GenerationSettings {
            model: Some("v2.5-20250123".into()),
            geometry_quality: Some(GeometryQuality::Detailed),
            ..GenerationSettings::default()
        };
        assert!(text("cat", v25_quality).is_err());
        assert!(text(
            "cat",
            GenerationSettings {
                face_limit: Some(1_500_001),
                ..Default::default()
            }
        )
        .is_err());
        assert!(text(
            "cat",
            GenerationSettings {
                quad: Some(true),
                face_limit: Some(150_001),
                ..Default::default()
            }
        )
        .is_err());
        assert!(text(
            "cat",
            GenerationSettings {
                smart_low_poly: Some(true),
                face_limit: Some(100),
                ..Default::default()
            }
        )
        .is_err());
        assert!(text(
            "cat",
            GenerationSettings {
                model: Some("v3/../x".into()),
                ..Default::default()
            }
        )
        .is_err());
        assert!(
            transport.requests.lock().unwrap().is_empty(),
            "invalid requests must not reach the network"
        );
    }

    #[test]
    fn parses_documented_success_task() {
        let transport = ScriptedTransport::default();
        transport.push(200, SUCCESS.as_bytes());
        let task = provider(&transport).task_status("task_abc123").unwrap();
        assert_eq!(task.status, ProviderStatus::Success);
        assert_eq!(task.progress, Some(100));
        assert_eq!(task.credits_consumed, Some(100.0));
        assert_eq!(task.outputs.len(), 2);
        assert_eq!(task.outputs[0].label, "model_url");
        assert_eq!(task.outputs[0].kind, OutputKind::Model);
        assert_eq!(task.outputs[1].kind, OutputKind::PreviewImage);
        assert!(transport.requests.lock().unwrap()[0]
            .url
            .ends_with("/tasks/task_abc123"));
    }

    #[test]
    fn task_parsing_handles_terminal_and_hostile_values() {
        let banned = parse_task("t1", &json!({"task_id": "t1", "status": "banned"})).unwrap();
        assert_eq!(banned.status, ProviderStatus::Banned);
        assert!(banned.error_message.unwrap().contains("content policy"));
        let odd = parse_task(
            "t1",
            &json!({"status": "weird", "progress": 250, "output": {"model_url": "http://x/a.glb", "note": "x"}}),
        )
        .unwrap();
        assert_eq!(odd.status, ProviderStatus::Unknown);
        assert_eq!(odd.progress, Some(100));
        assert!(odd.outputs.is_empty(), "non-HTTPS outputs are dropped");
        assert!(parse_task("t1", &json!({"task_id": "other", "status": "success"})).is_err());
    }

    #[test]
    fn invalid_task_id_never_reaches_transport() {
        let transport = ScriptedTransport::default();
        assert_eq!(
            provider(&transport)
                .task_status("../account")
                .unwrap_err()
                .kind,
            ErrorKind::Validation
        );
        assert!(transport.requests.lock().unwrap().is_empty());
    }

    #[test]
    fn upload_and_balance_parse_documented_responses() {
        let transport = ScriptedTransport::default();
        transport.push(200, UPLOAD.as_bytes());
        transport.push(200, BALANCE.as_bytes());
        let tripo = provider(&transport);
        assert_eq!(
            tripo
                .upload_image(vec![1, 2], "a.png", "image/png")
                .unwrap(),
            "file_abc123"
        );
        let balance = tripo.account_balance().unwrap();
        assert_eq!(balance.available, 10_000.0);
        assert_eq!(balance.frozen, 200.0);
        let requests = transport.requests.lock().unwrap();
        assert!(requests[0].url.ends_with("/files"));
        assert!(requests[1].url.ends_with("/account/balance"));
    }

    #[test]
    fn maps_documented_errors_to_stable_kinds() {
        let error = parse_envelope(403, NO_CREDIT.as_bytes(), None).unwrap_err();
        assert_eq!(error.kind, ErrorKind::InsufficientCredits);
        assert_eq!(error.provider_code, Some(2010));
        assert_eq!(error.request_id.as_deref(), Some("req_abc123"));
        let cases = [
            (
                401,
                r#"{"code":1000,"message":"bad key"}"#,
                ErrorKind::Authentication,
            ),
            (
                429,
                r#"{"code":2000,"message":"busy"}"#,
                ErrorKind::RateLimit,
            ),
            (429, r#"{"code":1007}"#, ErrorKind::RateLimit),
            (
                400,
                r#"{"code":2008,"message":"policy"}"#,
                ErrorKind::Validation,
            ),
            (400, r#"{"code":2015}"#, ErrorKind::Validation),
            (500, r#"{"code":9999}"#, ErrorKind::ProviderUnavailable),
            (
                502,
                "<html>bad gateway</html>",
                ErrorKind::ProviderUnavailable,
            ),
            (200, "not json", ErrorKind::UnknownProvider),
            (
                200,
                r#"{"code":2002,"message":"bad field"}"#,
                ErrorKind::Validation,
            ),
        ];
        for (status, body, expected) in cases {
            assert_eq!(
                parse_envelope(status, body.as_bytes(), None)
                    .unwrap_err()
                    .kind,
                expected,
                "{status} {body}"
            );
        }
    }

    #[test]
    fn http_success_with_error_code_is_not_success() {
        assert!(parse_envelope(200, br#"{"code":2010,"message":"no credit"}"#, None).is_err());
        assert!(
            parse_envelope(200, br#"{"code":0}"#, None).is_err(),
            "missing data is not success"
        );
    }

    #[test]
    fn transport_failures_map_to_provider_unavailable_and_downloads_require_https() {
        let transport = ScriptedTransport::default();
        transport.push_error(TransportError::Timeout);
        let tripo = provider(&transport);
        assert_eq!(
            tripo.account_balance().unwrap_err().kind,
            ErrorKind::ProviderUnavailable
        );
        assert_eq!(
            tripo.download("http://cdn/x.glb", 10).unwrap_err().kind,
            ErrorKind::Conversion
        );
        transport.push(404, b"");
        assert_eq!(
            tripo.download("https://cdn/x.glb", 10).unwrap_err().kind,
            ErrorKind::Conversion
        );
        let requests = transport.requests.lock().unwrap();
        assert!(
            requests[1].bearer.is_none(),
            "API key is never sent to output CDNs"
        );
    }
}

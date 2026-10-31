//! TRELLIS (MIT, Microsoft) through the public Hugging Face Space
//! `trellis-community/TRELLIS`, driven with Gradio's queue protocol (`sse_v3`),
//! the same protocol the official `gradio_client` uses:
//! `/gradio_api/upload`, `/gradio_api/queue/join`, `/gradio_api/queue/data` (SSE).
//! Endpoint indices are resolved by `api_name` from the Space's public `/config`
//! on every job, so a redeployed Space does not silently break the mapping.
//! Contract inspected 2026-10-29 (Gradio 5.34.2).
//!
//! Free: runs on the Space's ZeroGPU. Anonymous calls get a small daily quota;
//! an optional Hugging Face token (stored in the OS keyring) uses the account's
//! free quota. Jobs run in background threads; status lives in `status.json`.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::error::{ErrorKind, ProviderError};
use crate::local::{read_status, write_failed};
use crate::provider::{
    is_valid_task_id, AccountBalance, Capability, ConvertRequest, GenerationProvider,
    ImageTo3dRequest, ModelInfo, MultiviewTo3dRequest, OutputKind, OutputRef, ProviderLimits,
    ProviderStatus, ProviderTask, TextTo3dRequest,
};
use crate::transport::{Body, HttpRequest, Method, Transport};

pub const PROVIDER_ID: &str = "hf-trellis";
pub const MODEL_ID: &str = "trellis-image-large";
const SPACE_URL: &str = "https://trellis-community-trellis.hf.space";
const SESSION_API: &str = "start_session";
const PREPROCESS_API: &str = "preprocess_image";
const GENERATE_API: &str = "generate_and_extract_glb";
/// image, multi-images, multi-image flag (state), seed, 6 sampler/export settings, texture size.
const GENERATE_INPUTS: usize = 11;
/// Defaults published by the Space's UI configuration.
const SS_GUIDANCE: f64 = 7.5;
const SS_STEPS: u32 = 12;
const SLAT_GUIDANCE: f64 = 3.0;
const SLAT_STEPS: u32 = 12;
const MULTIIMAGE_ALGO: &str = "stochastic";
const MESH_SIMPLIFY: f64 = 0.95;
const TEXTURE_SIZE: u32 = 1024;
/// TRELLIS seeds are int32.
const SEED_MAX: i64 = i32::MAX as i64;
const RESPONSE_MAX_BYTES: u64 = 4 * 1024 * 1024;
const CONFIG_MAX_BYTES: u64 = 8 * 1024 * 1024;
const SERVER_PATH_MAX_CHARS: usize = 1024;
const STATUS_FILE: &str = "status.json";
const LIMITS: ProviderLimits = ProviderLimits {
    prompt_max_chars: 0,
    negative_prompt_max_chars: 0,
    image_max_bytes: 20 * 1024 * 1024,
    quad_max_faces: 0,
};
const CAPABILITIES: &[Capability] = &[
    Capability::ImageTo3d,
    Capability::TaskStatus,
    Capability::Upload,
];
const SUPPORTED_SETTINGS: &[&str] = &["model", "modelSeed"];

/// Shared state for every provider instance the factory creates.
pub struct HfSpaceEngine {
    transport: Arc<dyn Transport>,
    jobs_dir: PathBuf,
    /// Upload token -> server-side file path returned by the Space.
    uploads: Mutex<HashMap<String, String>>,
    /// Jobs running in this process; others still marked running were interrupted.
    live: Mutex<HashSet<String>>,
}

impl HfSpaceEngine {
    pub fn new(transport: Arc<dyn Transport>, jobs_dir: PathBuf) -> Arc<Self> {
        Arc::new(Self {
            transport,
            jobs_dir,
            uploads: Mutex::default(),
            live: Mutex::default(),
        })
    }

    fn job_dir(&self, job_id: &str) -> Result<PathBuf, ProviderError> {
        if !is_valid_task_id(job_id) {
            return Err(ProviderError::new(ErrorKind::Validation, "Invalid job ID."));
        }
        Ok(self.jobs_dir.join(job_id))
    }

    /// Returns the raw response body after mapping HTTP failures to readable messages.
    fn send(
        &self,
        token: Option<&str>,
        method: Method,
        path: &str,
        body: Body,
        long_running: bool,
        max_bytes: u64,
    ) -> Result<Vec<u8>, String> {
        let request = HttpRequest {
            method,
            url: format!("{SPACE_URL}{path}"),
            bearer: token.map(str::to_owned),
            body,
            long_running,
        };
        let response = self
            .transport
            .send(request, max_bytes)
            .map_err(|_| "Could not reach the Hugging Face Space.".to_string())?;
        match response.status {
            200 => Ok(response.body),
            401 | 403 => {
                Err("Hugging Face rejected the token. Check it in Studio settings.".into())
            }
            429 => Err("Hugging Face rate limit reached. Try again later.".into()),
            404 => Err("The Space API changed or is unavailable.".into()),
            status if status >= 500 => Err(format!(
                "The Space is sleeping or busy (HTTP {status}). Try again shortly."
            )),
            status => Err(format!("The Space rejected the request (HTTP {status}).")),
        }
    }

    fn send_json(
        &self,
        token: Option<&str>,
        method: Method,
        path: &str,
        body: Body,
        max: u64,
    ) -> Result<Value, String> {
        let bytes = self.send(token, method, path, body, false, max)?;
        serde_json::from_slice(&bytes).map_err(|_| "Space returned unreadable data.".to_string())
    }

    /// One queued event: join, then read the session's SSE stream until it closes.
    fn queue_call(
        &self,
        token: Option<&str>,
        fn_index: u64,
        data: Value,
        session: &str,
    ) -> Result<Vec<Value>, String> {
        let joined = self.send_json(
            token,
            Method::Post,
            "/gradio_api/queue/join",
            Body::Json(json!({
                "data": data,
                "event_data": null,
                "fn_index": fn_index,
                "trigger_id": null,
                "session_hash": session,
            })),
            RESPONSE_MAX_BYTES,
        )?;
        let event_id = joined
            .get("event_id")
            .and_then(Value::as_str)
            .filter(|id| is_valid_task_id(id))
            .ok_or("Space did not return an event ID.")?
            .to_owned();
        let stream = self.send(
            token,
            Method::Get,
            &format!("/gradio_api/queue/data?session_hash={session}"),
            Body::Empty,
            true,
            RESPONSE_MAX_BYTES,
        )?;
        let text =
            String::from_utf8(stream).map_err(|_| "Space returned unreadable data.".to_string())?;
        parse_queue_stream(&text, &event_id)
    }

    fn run_job(
        &self,
        token: Option<&str>,
        job_id: &str,
        server_path: &str,
        seed: i64,
    ) -> Result<String, String> {
        let config =
            self.send_json(token, Method::Get, "/config", Body::Empty, CONFIG_MAX_BYTES)?;
        let endpoints = resolve_endpoints(&config)?;
        let session = uuid::Uuid::new_v4().simple().to_string()[..12].to_owned();
        self.queue_call(token, endpoints.session, json!([]), &session)?;
        let uploaded = json!({ "path": server_path, "meta": { "_type": "gradio.FileData" } });
        let preprocessed = self
            .queue_call(token, endpoints.preprocess, json!([uploaded]), &session)?
            .into_iter()
            .next()
            .filter(Value::is_object)
            .ok_or("The Space could not prepare the image.")?;
        let data = json!([
            preprocessed,
            [],
            null,
            seed,
            SS_GUIDANCE,
            SS_STEPS,
            SLAT_GUIDANCE,
            SLAT_STEPS,
            MULTIIMAGE_ALGO,
            MESH_SIMPLIFY,
            TEXTURE_SIZE
        ]);
        let outputs = self.queue_call(token, endpoints.generate, data, &session)?;
        glb_url(&outputs).ok_or_else(|| format!("Job {job_id}: the Space returned no GLB file."))
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Endpoints {
    session: u64,
    preprocess: u64,
    generate: u64,
}

/// Maps `api_name` to `fn_index` and checks the generate signature still matches.
fn resolve_endpoints(config: &Value) -> Result<Endpoints, String> {
    let dependencies = config
        .get("dependencies")
        .and_then(Value::as_array)
        .ok_or("Space config has no endpoints.")?;
    let find = |name: &str| {
        dependencies
            .iter()
            .enumerate()
            .find_map(|(index, dependency)| {
                (dependency.get("api_name").and_then(Value::as_str) == Some(name)).then(|| {
                    (
                        dependency
                            .get("id")
                            .and_then(Value::as_u64)
                            .unwrap_or(index as u64),
                        dependency,
                    )
                })
            })
    };
    let missing = |name: &str| format!("The Space no longer offers `{name}`; its API changed.");
    let (session, _) = find(SESSION_API).ok_or_else(|| missing(SESSION_API))?;
    let (preprocess, _) = find(PREPROCESS_API).ok_or_else(|| missing(PREPROCESS_API))?;
    let (generate, dependency) = find(GENERATE_API).ok_or_else(|| missing(GENERATE_API))?;
    let inputs = dependency
        .get("inputs")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    if inputs != GENERATE_INPUTS {
        return Err(format!(
            "The Space changed `{GENERATE_API}` ({inputs} inputs, expected {GENERATE_INPUTS})."
        ));
    }
    Ok(Endpoints {
        session,
        preprocess,
        generate,
    })
}

/// Reads a Gradio `sse_v3` stream and returns the output data of `event_id`.
pub(crate) fn parse_queue_stream(text: &str, event_id: &str) -> Result<Vec<Value>, String> {
    for line in text.lines() {
        let Some(payload) = line.strip_prefix("data:") else {
            continue;
        };
        let Ok(message) = serde_json::from_str::<Value>(payload.trim()) else {
            continue;
        };
        let ours = message
            .get("event_id")
            .and_then(Value::as_str)
            .is_none_or(|id| id == event_id);
        if !ours {
            continue;
        }
        match message.get("msg").and_then(Value::as_str) {
            Some("process_completed") => {
                let output = message.get("output");
                if message.get("success").and_then(Value::as_bool) == Some(true) {
                    return output
                        .and_then(|output| output.get("data"))
                        .and_then(Value::as_array)
                        .cloned()
                        .ok_or_else(|| "Space returned an empty result.".to_string());
                }
                return Err(space_error(output.and_then(|output| output.get("error"))));
            }
            Some("unexpected_error") => return Err(space_error(message.get("message"))),
            _ => {}
        }
    }
    Err("The Space stream ended without a result.".into())
}

fn space_error(detail: Option<&Value>) -> String {
    match detail.and_then(Value::as_str).filter(|message| !message.trim().is_empty()) {
        Some(message) => format!("The Space reported an error: {}", message.chars().take(300).collect::<String>()),
        None => "The Space reported an error (often the free GPU quota is used up; add a Hugging Face token or try later).".into(),
    }
}

/// The GLB is the model-viewer or download output; both are Gradio FileData.
fn glb_url(outputs: &[Value]) -> Option<String> {
    outputs.iter().find_map(|output| {
        let url = output.get("url")?.as_str()?;
        let path = output.get("path").and_then(Value::as_str).unwrap_or(url);
        (is_space_file_url(url) && path.to_ascii_lowercase().ends_with(".glb"))
            .then(|| url.to_owned())
    })
}

fn is_space_file_url(url: &str) -> bool {
    url.starts_with(&format!("{SPACE_URL}/gradio_api/file=")) && !url.contains("..")
}

pub struct HfTrellisProvider {
    engine: Arc<HfSpaceEngine>,
    token: Option<String>,
}

impl HfTrellisProvider {
    /// An empty key means anonymous access.
    pub fn new(engine: Arc<HfSpaceEngine>, api_key: String) -> Self {
        Self {
            engine,
            token: Some(api_key).filter(|key| !key.is_empty()),
        }
    }

    fn unsupported(&self, what: &str) -> ProviderError {
        ProviderError::new(
            ErrorKind::Validation,
            format!("The TRELLIS Space provider does not support {what}."),
        )
    }
}

impl GenerationProvider for HfTrellisProvider {
    fn id(&self) -> &'static str {
        PROVIDER_ID
    }

    fn label(&self) -> &'static str {
        "TRELLIS via Hugging Face (free GPU quota, online)"
    }

    fn requires_api_key(&self) -> bool {
        false
    }

    fn accepts_api_key(&self) -> bool {
        true
    }

    fn availability(&self) -> Result<(), String> {
        Ok(())
    }

    fn supported_settings(&self) -> &'static [&'static str] {
        SUPPORTED_SETTINGS
    }

    fn capabilities(&self) -> &'static [Capability] {
        CAPABILITIES
    }

    fn models(&self) -> Vec<ModelInfo> {
        vec![ModelInfo {
            id: MODEL_ID.into(),
            label: "TRELLIS image-large (Hugging Face Space)".into(),
            source: "space config",
            supports_geometry_quality: false,
            max_faces_standard: 0,
            max_faces_detailed: None,
        }]
    }

    fn default_model(&self) -> &'static str {
        MODEL_ID
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
            field: "files",
            file_name: file_name.into(),
            mime: mime.into(),
            bytes,
        };
        let response = self
            .engine
            .send_json(
                self.token.as_deref(),
                Method::Post,
                "/gradio_api/upload",
                body,
                RESPONSE_MAX_BYTES,
            )
            .map_err(|message| ProviderError::new(ErrorKind::FileUpload, message))?;
        let path = response
            .as_array()
            .and_then(|paths| paths.first())
            .and_then(Value::as_str)
            .filter(|path| {
                path.starts_with('/') && path.len() <= SERVER_PATH_MAX_CHARS && !path.contains("..")
            })
            .ok_or_else(|| {
                ProviderError::new(ErrorKind::FileUpload, "The Space did not accept the image.")
            })?;
        let token = uuid::Uuid::new_v4().simple().to_string();
        self.engine
            .uploads
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(token.clone(), path.to_owned());
        Ok(token)
    }

    fn text_to_3d(&self, _request: &TextTo3dRequest) -> Result<String, ProviderError> {
        Err(self.unsupported("text-to-3D"))
    }

    fn image_to_3d(&self, request: &ImageTo3dRequest) -> Result<String, ProviderError> {
        let server_path = self
            .engine
            .uploads
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(&request.file_token)
            .ok_or_else(|| {
                ProviderError::new(
                    ErrorKind::Validation,
                    "Uploaded image was not found; upload again.",
                )
            })?;
        let seed = request.settings.model_seed.unwrap_or(0).clamp(0, SEED_MAX);
        let job_id = uuid::Uuid::new_v4().simple().to_string();
        let job_dir = self.engine.job_dir(&job_id)?;
        std::fs::create_dir_all(&job_dir)
            .and_then(|()| std::fs::write(job_dir.join(STATUS_FILE), r#"{"state":"running"}"#))
            .map_err(|error| ProviderError::new(ErrorKind::FileUpload, error.to_string()))?;
        self.engine
            .live
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(job_id.clone());
        let engine = Arc::clone(&self.engine);
        let token = self.token.clone();
        let id = job_id.clone();
        std::thread::Builder::new()
            .name("studio-hf-job".into())
            .spawn(move || {
                let dir = engine.jobs_dir.join(&id);
                match engine.run_job(token.as_deref(), &id, &server_path, seed) {
                    Ok(url) => {
                        let payload = json!({ "state": "succeeded", "modelUrl": url }).to_string();
                        let partial = dir.join("status.json.part");
                        let written = std::fs::write(&partial, payload)
                            .and_then(|()| std::fs::rename(&partial, dir.join(STATUS_FILE)));
                        if written.is_err() {
                            write_failed(&dir, "Could not record the result.");
                        }
                    }
                    Err(message) => write_failed(&dir, &message),
                }
                engine
                    .live
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .remove(&id);
            })
            .map_err(|_| {
                ProviderError::new(
                    ErrorKind::ProviderUnavailable,
                    "Could not start the job thread.",
                )
            })?;
        Ok(job_id)
    }

    fn multiview_to_3d(&self, _request: &MultiviewTo3dRequest) -> Result<String, ProviderError> {
        Err(self.unsupported("multiview input yet"))
    }

    fn convert(&self, _request: &ConvertRequest) -> Result<String, ProviderError> {
        Err(self.unsupported("format conversion"))
    }

    fn task_status(&self, job_id: &str) -> Result<ProviderTask, ProviderError> {
        let job_dir = self.engine.job_dir(job_id)?;
        let status = read_status(&job_dir).ok_or_else(|| {
            ProviderError::new(
                ErrorKind::TaskFailed,
                "Job status is missing or unreadable.",
            )
        })?;
        let live = self
            .engine
            .live
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .contains(job_id);
        let mut task = ProviderTask {
            provider_task_id: job_id.into(),
            status: ProviderStatus::Unknown,
            progress: None,
            outputs: Vec::new(),
            credits_consumed: None,
            error_message: status.error.clone(),
        };
        task.status = match status.state.as_str() {
            "running" if live => ProviderStatus::Running,
            "running" => {
                task.error_message =
                    Some("The job was interrupted (the app was closed). Generate again.".into());
                ProviderStatus::Failed
            }
            "succeeded" => {
                let url = std::fs::read_to_string(job_dir.join(STATUS_FILE))
                    .ok()
                    .and_then(|text| serde_json::from_str::<Value>(&text).ok())
                    .and_then(|value| value.get("modelUrl")?.as_str().map(str::to_owned))
                    .filter(|url| is_space_file_url(url))
                    .ok_or_else(|| {
                        ProviderError::new(
                            ErrorKind::UnknownProvider,
                            "Job result is missing its model URL.",
                        )
                    })?;
                task.outputs.push(OutputRef {
                    kind: OutputKind::Model,
                    label: "model".into(),
                    url,
                });
                ProviderStatus::Success
            }
            "failed" => ProviderStatus::Failed,
            _ => ProviderStatus::Unknown,
        };
        Ok(task)
    }

    fn account_balance(&self) -> Result<AccountBalance, ProviderError> {
        Err(ProviderError::new(
            ErrorKind::Validation,
            "Hugging Face ZeroGPU quota is not exposed through the Space API.",
        ))
    }

    fn download(&self, url: &str, max_bytes: u64) -> Result<Vec<u8>, ProviderError> {
        if !is_space_file_url(url) {
            return Err(ProviderError::new(
                ErrorKind::Conversion,
                "Output URL is not from the TRELLIS Space.",
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
            .engine
            .transport
            .send(request, max_bytes)
            .map_err(|_| {
                ProviderError::new(
                    ErrorKind::ProviderUnavailable,
                    "Could not download the model from the Space.",
                )
            })?;
        if response.status != 200 {
            return Err(ProviderError::new(
                ErrorKind::Conversion,
                format!(
                    "The Space no longer serves this file (HTTP {}).",
                    response.status
                ),
            ));
        }
        Ok(response.body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tripo::tests::ScriptedTransport;

    const GLB_URL: &str =
        "https://trellis-community-trellis.hf.space/gradio_api/file=/tmp/gradio/abc/sample.glb";

    fn config() -> Value {
        let inputs: Vec<u64> = (0..GENERATE_INPUTS as u64).collect();
        json!({ "dependencies": [
            { "id": 0, "api_name": "lambda" },
            { "id": 4, "api_name": "start_session", "inputs": [] },
            { "id": 8, "api_name": "preprocess_image", "inputs": [6] },
            { "id": 11, "api_name": "generate_and_extract_glb", "inputs": inputs },
        ]})
    }

    fn completed(event_id: &str, data: Value) -> String {
        let done = json!({ "msg": "process_completed", "event_id": event_id, "success": true, "output": { "data": data } });
        format!(
            "data: {{\"msg\":\"estimation\",\"event_id\":\"{event_id}\",\"rank\":0}}\n\ndata: {done}\n\ndata: {{\"msg\":\"close_stream\"}}\n\n"
        )
    }

    #[test]
    fn endpoints_resolve_by_api_name_and_signature_is_checked() {
        assert_eq!(
            resolve_endpoints(&config()).unwrap(),
            Endpoints {
                session: 4,
                preprocess: 8,
                generate: 11
            }
        );
        let changed = json!({ "dependencies": [
            { "id": 4, "api_name": "start_session" }, { "id": 8, "api_name": "preprocess_image" },
            { "id": 11, "api_name": "generate_and_extract_glb", "inputs": [1, 2] }
        ]});
        assert!(resolve_endpoints(&changed).unwrap_err().contains("changed"));
        assert!(resolve_endpoints(&json!({ "dependencies": [] }))
            .unwrap_err()
            .contains("start_session"));
    }

    #[test]
    fn queue_stream_returns_our_result_or_actionable_error() {
        assert_eq!(
            parse_queue_stream(&completed("e1", json!([1])), "e1").unwrap(),
            vec![json!(1)]
        );
        let other = completed("other", json!([1]));
        assert!(parse_queue_stream(&other, "e1")
            .unwrap_err()
            .contains("without a result"));
        let quota = "data: {\"msg\":\"process_completed\",\"event_id\":\"e1\",\"success\":false,\"output\":{\"error\":null}}\n";
        assert!(parse_queue_stream(quota, "e1")
            .unwrap_err()
            .contains("quota"));
        let named = "data: {\"msg\":\"process_completed\",\"event_id\":\"e1\",\"success\":false,\"output\":{\"error\":\"GPU task aborted\"}}\n";
        assert!(parse_queue_stream(named, "e1")
            .unwrap_err()
            .contains("GPU task aborted"));
        let outputs = [
            json!(null),
            json!({ "path": "x.mp4", "url": "https://trellis-community-trellis.hf.space/gradio_api/file=/x.mp4" }),
            json!({ "path": "/tmp/a/sample.glb", "url": GLB_URL }),
        ];
        assert_eq!(glb_url(&outputs).as_deref(), Some(GLB_URL));
        assert_eq!(
            glb_url(&[json!({ "path": "x.glb", "url": "https://evil.example/x.glb" })]),
            None
        );
    }

    #[test]
    fn job_uses_one_session_for_session_preprocess_and_generate() {
        let dir = tempfile::tempdir().unwrap();
        let transport = ScriptedTransport::default();
        let engine = HfSpaceEngine::new(Arc::new(transport.clone()), dir.path().to_path_buf());
        let processed = json!({ "path": "/tmp/gradio/p/processed.png", "meta": { "_type": "gradio.FileData" } });
        transport.push(200, config().to_string().as_bytes());
        transport.push(200, br#"{"event_id":"e1"}"#);
        transport.push(200, completed("e1", json!([])).as_bytes());
        transport.push(200, br#"{"event_id":"e2"}"#);
        transport.push(200, completed("e2", json!([processed.clone()])).as_bytes());
        transport.push(200, br#"{"event_id":"e3"}"#);
        let outputs = json!([null, { "path": "/tmp/a/sample.mp4", "url": "https://trellis-community-trellis.hf.space/gradio_api/file=/tmp/a/sample.mp4" }, { "path": "/tmp/a/sample.glb", "url": GLB_URL }, { "path": "/tmp/a/sample.glb", "url": GLB_URL }]);
        transport.push(200, completed("e3", outputs).as_bytes());

        let url = engine
            .run_job(Some("hf_token"), "job1", "/tmp/gradio/u/in.png", 42)
            .unwrap();
        assert_eq!(url, GLB_URL);
        let requests = transport.requests.lock().unwrap();
        assert!(requests[0].url.ends_with("/config"));
        let joins: Vec<&Value> = requests
            .iter()
            .filter_map(|request| request.json.as_ref())
            .collect();
        assert_eq!(
            joins
                .iter()
                .map(|join| join["fn_index"].as_u64().unwrap())
                .collect::<Vec<_>>(),
            vec![4, 8, 11]
        );
        let session = joins[0]["session_hash"].clone();
        assert!(
            joins.iter().all(|join| join["session_hash"] == session),
            "one Gradio session per job"
        );
        assert!(requests[2].url.ends_with(&format!(
            "/gradio_api/queue/data?session_hash={}",
            session.as_str().unwrap()
        )));
        assert_eq!(joins[1]["data"][0]["path"], "/tmp/gradio/u/in.png");
        let generate = joins[2]["data"].as_array().unwrap();
        assert_eq!(generate.len(), GENERATE_INPUTS);
        assert_eq!(
            (&generate[0], &generate[2], &generate[3], &generate[10]),
            (&processed, &Value::Null, &json!(42), &json!(1024))
        );
        assert_eq!(requests[5].bearer.as_deref(), Some("hf_token"));
    }

    #[test]
    fn upload_maps_token_and_status_reports_real_state() {
        let dir = tempfile::tempdir().unwrap();
        let transport = ScriptedTransport::default();
        let engine = HfSpaceEngine::new(Arc::new(transport.clone()), dir.path().to_path_buf());
        let provider = HfTrellisProvider::new(Arc::clone(&engine), String::new());
        transport.push(200, br#"["/tmp/gradio/u/in.png"]"#);
        let token = provider
            .upload_image(vec![1], "in.png", "image/png")
            .unwrap();
        assert_eq!(
            engine
                .uploads
                .lock()
                .unwrap()
                .get(&token)
                .map(String::as_str),
            Some("/tmp/gradio/u/in.png")
        );
        assert!(
            transport.requests.lock().unwrap()[0].bearer.is_none(),
            "anonymous without a token"
        );

        let job_dir = dir.path().join("job1");
        std::fs::create_dir_all(&job_dir).unwrap();
        std::fs::write(job_dir.join(STATUS_FILE), r#"{"state":"running"}"#).unwrap();
        assert_eq!(
            provider.task_status("job1").unwrap().status,
            ProviderStatus::Failed,
            "interrupted"
        );
        let ok = json!({ "state": "succeeded", "modelUrl": GLB_URL }).to_string();
        std::fs::write(job_dir.join(STATUS_FILE), ok).unwrap();
        let done = provider.task_status("job1").unwrap();
        assert_eq!(
            (done.status, done.outputs[0].url.as_str()),
            (ProviderStatus::Success, GLB_URL)
        );
        let evil =
            json!({ "state": "succeeded", "modelUrl": "https://evil.example/a.glb" }).to_string();
        std::fs::write(job_dir.join(STATUS_FILE), evil).unwrap();
        assert!(provider.task_status("job1").is_err());
        assert!(provider.download("https://evil.example/a.glb", 10).is_err());
    }

    #[test]
    fn upload_rejects_unexpected_server_paths() {
        let dir = tempfile::tempdir().unwrap();
        let transport = ScriptedTransport::default();
        let engine = HfSpaceEngine::new(Arc::new(transport.clone()), dir.path().into());
        let provider = HfTrellisProvider::new(engine, String::new());
        transport.push(200, br#"["../etc/passwd"]"#);
        assert_eq!(
            provider
                .upload_image(vec![1], "a.png", "image/png")
                .unwrap_err()
                .kind,
            ErrorKind::FileUpload
        );
        transport.push(503, b"");
        assert!(provider
            .upload_image(vec![1], "a.png", "image/png")
            .unwrap_err()
            .message
            .contains("sleeping"));
    }
}

//! Local, offline TripoSR engine (MIT). Generation runs on this machine through
//! a fixed Python runner installed by `engines/triposr/setup_engine.py`.
//!
//! The provider implements the same async-task contract as cloud providers:
//! `image_to_3d` queues a job, a single worker (one GPU) runs it, `task_status`
//! reads the runner's `status.json`, and `download` reads only allowlisted
//! files from the job directory through `local://<job>/<file>` references.

use std::collections::{HashSet, VecDeque};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use serde::Deserialize;

use crate::error::{ErrorKind, ProviderError};
use crate::provider::{
    is_valid_task_id, AccountBalance, Capability, ConvertRequest, GenerationProvider,
    ImageTo3dRequest, ModelInfo, MultiviewTo3dRequest, OutputKind, OutputRef, ProviderLimits,
    ProviderStatus, ProviderTask, TextTo3dRequest,
};

pub const PROVIDER_ID: &str = "local-triposr";
pub const MODEL_ID: &str = "triposr-1.0";
const MANIFEST_FILE: &str = "engine.json";
const LOCAL_SCHEME: &str = "local://";
/// Output files a job may expose; nothing else in a job directory is readable.
const MODEL_FILE: &str = "model.glb";
const PREVIEW_FILE: &str = "processed_input.png";
const STATUS_FILE: &str = "status.json";
const STATUS_MAX_BYTES: u64 = 64 * 1024;
/// Kills a runner that hangs; TripoSR on CPU finishes well inside this.
const JOB_MAX_RUNTIME: Duration = Duration::from_secs(30 * 60);
const WORKER_POLL: Duration = Duration::from_millis(500);
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
/// `texture: true` bakes a UV texture; otherwise the mesh carries vertex colours.
const SUPPORTED_SETTINGS: &[&str] = &["model", "texture"];

#[derive(Debug, Clone, Copy)]
pub struct LocalEngineConfig {
    /// Smaller chunks use less GPU memory; 2048 fits a 4 GB GPU.
    pub chunk_size: u32,
    pub mc_resolution: u32,
    pub texture_resolution: u32,
}

impl Default for LocalEngineConfig {
    fn default() -> Self {
        Self {
            chunk_size: 2048,
            mc_resolution: 256,
            texture_resolution: 1024,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawManifest {
    engine: String,
    python: PathBuf,
    runner: PathBuf,
}

/// Validated engine install: every executable path lives inside the engine root.
#[derive(Debug, Clone)]
pub struct EngineManifest {
    pub root: PathBuf,
    pub python: PathBuf,
    pub runner: PathBuf,
}

impl EngineManifest {
    pub fn load(root: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(root.join(MANIFEST_FILE)).map_err(|_| {
            format!(
                "Local engine is not installed. Run `python engines/triposr/setup_engine.py` (expected {}).",
                root.join(MANIFEST_FILE).display()
            )
        })?;
        let raw: RawManifest = serde_json::from_str(&text)
            .map_err(|_| "Local engine manifest is invalid.".to_string())?;
        if raw.engine != "triposr" {
            return Err("Local engine manifest is for a different engine.".into());
        }
        let root = root.canonicalize().map_err(|error| error.to_string())?;
        let inside = |path: &Path, what: &str| -> Result<PathBuf, String> {
            // Relative paths resolve against the root; absolute ones must still land inside it.
            let resolved = root
                .join(path)
                .canonicalize()
                .map_err(|_| format!("Local engine {what} is missing."))?;
            if resolved.starts_with(&root) && resolved.is_file() {
                Ok(resolved)
            } else {
                Err(format!("Local engine {what} is outside the engine folder."))
            }
        };
        Ok(Self {
            python: inside(&raw.python, "Python")?,
            runner: inside(&raw.runner, "runner")?,
            root,
        })
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct RunnerStatus {
    pub(crate) state: String,
    pub(crate) error: Option<String>,
}

#[derive(Debug, serde::Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JobSettings {
    bake_texture: bool,
}

struct Queue {
    pending: VecDeque<String>,
    /// Jobs queued or running in this process; others still marked running were interrupted.
    live: HashSet<String>,
    worker_started: bool,
}

/// Shared by every provider instance the factory creates.
pub struct LocalEngine {
    engine_root: PathBuf,
    jobs_dir: PathBuf,
    config: LocalEngineConfig,
    queue: Mutex<Queue>,
    wake: Condvar,
}

impl LocalEngine {
    pub fn new(engine_root: PathBuf, jobs_dir: PathBuf, config: LocalEngineConfig) -> Arc<Self> {
        Arc::new(Self {
            engine_root,
            jobs_dir,
            config,
            queue: Mutex::new(Queue {
                pending: VecDeque::new(),
                live: HashSet::new(),
                worker_started: false,
            }),
            wake: Condvar::new(),
        })
    }

    /// `MIYORA_ENGINE_ROOT` overrides the default `%LOCALAPPDATA%/Miyora/engines/triposr`.
    pub fn default_engine_root() -> PathBuf {
        if let Some(root) = std::env::var_os("MIYORA_ENGINE_ROOT").filter(|value| !value.is_empty())
        {
            return PathBuf::from(root);
        }
        let base = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("miyora"));
        base.join("Miyora").join("engines").join("triposr")
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Queue> {
        self.queue
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn job_dir(&self, job_id: &str) -> Result<PathBuf, ProviderError> {
        if !is_valid_task_id(job_id) {
            return Err(ProviderError::new(
                ErrorKind::Validation,
                "Invalid local job ID.",
            ));
        }
        Ok(self.jobs_dir.join(job_id))
    }

    fn enqueue(self: &Arc<Self>, job_id: String) -> Result<(), ProviderError> {
        let manifest = EngineManifest::load(&self.engine_root)
            .map_err(|reason| ProviderError::new(ErrorKind::ProviderUnavailable, reason))?;
        let mut queue = self.lock();
        if !queue.worker_started {
            let engine = Arc::clone(self);
            std::thread::Builder::new()
                .name("studio-local-engine".into())
                .spawn(move || engine.work(manifest))
                .map_err(|_| {
                    ProviderError::new(
                        ErrorKind::ProviderUnavailable,
                        "Could not start the local engine worker.",
                    )
                })?;
            queue.worker_started = true;
        }
        queue.live.insert(job_id.clone());
        queue.pending.push_back(job_id);
        self.wake.notify_one();
        Ok(())
    }

    fn work(&self, manifest: EngineManifest) {
        loop {
            let job_id = {
                let mut queue = self.lock();
                loop {
                    if let Some(job) = queue.pending.pop_front() {
                        break job;
                    }
                    queue = self
                        .wake
                        .wait(queue)
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                }
            };
            let job_dir = self.jobs_dir.join(&job_id);
            if let Err(reason) = self.run_job(&manifest, &job_id, &job_dir) {
                write_failed(&job_dir, &reason);
            }
            self.lock().live.remove(&job_id);
        }
    }

    fn run_job(
        &self,
        manifest: &EngineManifest,
        job_id: &str,
        job_dir: &Path,
    ) -> Result<(), String> {
        let settings: JobSettings = std::fs::read_to_string(job_dir.join("job.json"))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .ok_or("Job settings are missing.")?;
        let log =
            std::fs::File::create(job_dir.join("engine.log")).map_err(|error| error.to_string())?;
        let log_err = log.try_clone().map_err(|error| error.to_string())?;
        let mut command = Command::new(&manifest.python);
        command
            .arg("-I")
            .arg(&manifest.runner)
            .arg("--job-dir")
            .arg(job_dir)
            .args(["--chunk-size", &self.config.chunk_size.to_string()])
            .args(["--mc-resolution", &self.config.mc_resolution.to_string()])
            .current_dir(&manifest.root)
            .env("HF_HOME", manifest.root.join("hf"))
            .env("U2NET_HOME", manifest.root.join("u2net"))
            .stdin(Stdio::null())
            .stdout(log)
            .stderr(log_err);
        if settings.bake_texture {
            command.arg("--bake-texture").args([
                "--texture-resolution",
                &self.config.texture_resolution.to_string(),
            ]);
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
        }
        let mut child = command
            .spawn()
            .map_err(|_| "The local engine could not be started.".to_string())?;
        let started = Instant::now();
        let exit = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if started.elapsed() > JOB_MAX_RUNTIME => {
                    // Best effort: the job is reported failed either way.
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "Local job {job_id} exceeded {} minutes and was stopped.",
                        JOB_MAX_RUNTIME.as_secs() / 60
                    ));
                }
                Ok(None) => std::thread::sleep(WORKER_POLL),
                Err(error) => return Err(error.to_string()),
            }
        };
        let finished = read_status(job_dir)
            .is_some_and(|status| status.state == "succeeded" || status.state == "failed");
        if !finished {
            return Err(format!(
                "The local engine exited unexpectedly (code {:?}). See engine.log.",
                exit.code()
            ));
        }
        Ok(())
    }
}

pub(crate) fn read_status(job_dir: &Path) -> Option<RunnerStatus> {
    let file = std::fs::File::open(job_dir.join(STATUS_FILE)).ok()?;
    let mut text = String::new();
    file.take(STATUS_MAX_BYTES).read_to_string(&mut text).ok()?;
    serde_json::from_str(&text).ok()
}

pub(crate) fn write_failed(job_dir: &Path, reason: &str) {
    let payload = serde_json::json!({ "state": "failed", "error": reason });
    let partial = job_dir.join("status.json.part");
    if std::fs::write(&partial, payload.to_string()).is_ok()
        && std::fs::rename(&partial, job_dir.join(STATUS_FILE)).is_ok()
    {
        return;
    }
    eprintln!(
        "{}",
        serde_json::json!({ "component": "studio", "operation": "local_status_write", "status": "error" })
    );
}

pub struct LocalTriposrProvider {
    engine: Arc<LocalEngine>,
}

impl LocalTriposrProvider {
    pub fn new(engine: Arc<LocalEngine>) -> Self {
        Self { engine }
    }

    fn unsupported(&self, what: &str) -> ProviderError {
        ProviderError::new(
            ErrorKind::Validation,
            format!("The local TripoSR engine does not support {what}."),
        )
    }
}

impl GenerationProvider for LocalTriposrProvider {
    fn id(&self) -> &'static str {
        PROVIDER_ID
    }

    fn label(&self) -> &'static str {
        "Local TripoSR (free, offline)"
    }

    fn requires_api_key(&self) -> bool {
        false
    }

    fn availability(&self) -> Result<(), String> {
        EngineManifest::load(&self.engine.engine_root).map(|_| ())
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
            label: "TripoSR 1.0 (local)".into(),
            source: "local engine",
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

    /// "Upload" stores the image in a fresh job directory; the token is the job ID.
    fn upload_image(
        &self,
        bytes: Vec<u8>,
        _file_name: &str,
        _mime: &str,
    ) -> Result<String, ProviderError> {
        let job_id = uuid::Uuid::new_v4().simple().to_string();
        let job_dir = self.engine.job_dir(&job_id)?;
        std::fs::create_dir_all(&job_dir)
            .and_then(|()| std::fs::write(job_dir.join("input.png"), bytes))
            .map_err(|error| {
                ProviderError::new(
                    ErrorKind::FileUpload,
                    format!("Could not stage the image: {error}"),
                )
            })?;
        Ok(job_id)
    }

    fn text_to_3d(&self, _request: &TextTo3dRequest) -> Result<String, ProviderError> {
        Err(self.unsupported("text-to-3D"))
    }

    fn image_to_3d(&self, request: &ImageTo3dRequest) -> Result<String, ProviderError> {
        let job_dir = self.engine.job_dir(&request.file_token)?;
        if !job_dir.join("input.png").is_file() {
            return Err(ProviderError::new(
                ErrorKind::Validation,
                "Staged image was not found.",
            ));
        }
        let settings = JobSettings {
            bake_texture: request.settings.texture == Some(true),
        };
        let json = serde_json::to_string(&settings)
            .map_err(|error| ProviderError::new(ErrorKind::Validation, error.to_string()))?;
        std::fs::write(job_dir.join("job.json"), json)
            .and_then(|()| std::fs::write(job_dir.join(STATUS_FILE), r#"{"state":"queued"}"#))
            .map_err(|error| ProviderError::new(ErrorKind::FileUpload, error.to_string()))?;
        self.engine.enqueue(request.file_token.clone())?;
        Ok(request.file_token.clone())
    }

    fn multiview_to_3d(&self, _request: &MultiviewTo3dRequest) -> Result<String, ProviderError> {
        Err(self.unsupported("multiview input"))
    }

    fn convert(&self, _request: &ConvertRequest) -> Result<String, ProviderError> {
        Err(self.unsupported("format conversion"))
    }

    fn task_status(&self, job_id: &str) -> Result<ProviderTask, ProviderError> {
        let job_dir = self.engine.job_dir(job_id)?;
        let status = read_status(&job_dir).ok_or_else(|| {
            ProviderError::new(
                ErrorKind::TaskFailed,
                "Local job status is missing or unreadable.",
            )
        })?;
        let live = self.engine.lock().live.contains(job_id);
        let mut task = ProviderTask {
            provider_task_id: job_id.into(),
            status: ProviderStatus::Unknown,
            progress: None,
            outputs: Vec::new(),
            credits_consumed: None,
            error_message: status.error.clone(),
        };
        task.status = match status.state.as_str() {
            "queued" if live => ProviderStatus::Queued,
            "running" if live => ProviderStatus::Running,
            "queued" | "running" => {
                task.error_message = Some(
                    "The local job was interrupted (the app was closed). Generate again.".into(),
                );
                ProviderStatus::Failed
            }
            "succeeded" => {
                task.outputs.push(OutputRef {
                    kind: OutputKind::Model,
                    label: "model".into(),
                    url: format!("{LOCAL_SCHEME}{job_id}/{MODEL_FILE}"),
                });
                if job_dir.join(PREVIEW_FILE).is_file() {
                    task.outputs.push(OutputRef {
                        kind: OutputKind::PreviewImage,
                        label: "processed_input_image".into(),
                        url: format!("{LOCAL_SCHEME}{job_id}/{PREVIEW_FILE}"),
                    });
                }
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
            "The local engine has no account or credits; generation is free.",
        ))
    }

    fn download(&self, url: &str, max_bytes: u64) -> Result<Vec<u8>, ProviderError> {
        let invalid =
            || ProviderError::new(ErrorKind::Conversion, "Invalid local output reference.");
        let (job_id, file) = url
            .strip_prefix(LOCAL_SCHEME)
            .and_then(|rest| rest.split_once('/'))
            .ok_or_else(invalid)?;
        if file != MODEL_FILE && file != PREVIEW_FILE {
            return Err(invalid());
        }
        let path = self.engine.job_dir(job_id)?.join(file);
        let mut bytes = Vec::new();
        std::fs::File::open(&path)
            .and_then(|handle| {
                handle
                    .take(max_bytes.saturating_add(1))
                    .read_to_end(&mut bytes)
            })
            .map_err(|_| {
                ProviderError::new(ErrorKind::Conversion, "Local output file is missing.")
            })?;
        if bytes.len() as u64 > max_bytes {
            return Err(ProviderError::new(
                ErrorKind::Conversion,
                "Local output exceeds the size limit.",
            ));
        }
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine(dir: &Path) -> Arc<LocalEngine> {
        LocalEngine::new(
            dir.join("engine"),
            dir.join("jobs"),
            LocalEngineConfig::default(),
        )
    }

    #[test]
    fn manifest_requires_install_and_contained_paths() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("engine");
        std::fs::create_dir_all(&root).unwrap();
        assert!(EngineManifest::load(&root)
            .unwrap_err()
            .contains("not installed"));

        let outside = dir.path().join("python.exe");
        std::fs::write(&outside, b"").unwrap();
        std::fs::write(root.join("run_triposr.py"), b"").unwrap();
        let manifest = |python: &Path| {
            serde_json::json!({ "engine": "triposr", "python": python, "runner": root.join("run_triposr.py") })
                .to_string()
        };
        std::fs::write(root.join(MANIFEST_FILE), manifest(&outside)).unwrap();
        assert!(EngineManifest::load(&root).unwrap_err().contains("outside"));

        let inside = root.join("python.exe");
        std::fs::write(&inside, b"").unwrap();
        std::fs::write(root.join(MANIFEST_FILE), manifest(&inside)).unwrap();
        assert!(EngineManifest::load(&root).is_ok());
    }

    #[test]
    fn unavailable_engine_rejects_jobs_without_spawning() {
        let dir = tempfile::tempdir().unwrap();
        let provider = LocalTriposrProvider::new(engine(dir.path()));
        assert!(provider.availability().is_err());
        let token = provider
            .upload_image(vec![1, 2, 3], "a.png", "image/png")
            .unwrap();
        let request = ImageTo3dRequest {
            file_token: token,
            settings: Default::default(),
        };
        assert_eq!(
            provider.image_to_3d(&request).unwrap_err().kind,
            ErrorKind::ProviderUnavailable
        );
    }

    #[test]
    fn status_mapping_reports_only_real_runner_state() {
        let dir = tempfile::tempdir().unwrap();
        let engine = engine(dir.path());
        let provider = LocalTriposrProvider::new(Arc::clone(&engine));
        let job = provider
            .upload_image(vec![0], "a.png", "image/png")
            .unwrap();
        let job_dir = engine.job_dir(&job).unwrap();
        let set = |json: &str| std::fs::write(job_dir.join(STATUS_FILE), json).unwrap();

        set(r#"{"state":"running","stage":"inference"}"#);
        let interrupted = provider.task_status(&job).unwrap();
        assert_eq!(
            interrupted.status,
            ProviderStatus::Failed,
            "not live in this process"
        );
        assert!(interrupted.error_message.unwrap().contains("interrupted"));

        engine.lock().live.insert(job.clone());
        assert_eq!(
            provider.task_status(&job).unwrap().status,
            ProviderStatus::Running
        );
        assert_eq!(
            provider.task_status(&job).unwrap().progress,
            None,
            "no invented progress"
        );

        set(r#"{"state":"failed","error":"GPU ran out of memory."}"#);
        let failed = provider.task_status(&job).unwrap();
        assert_eq!(failed.status, ProviderStatus::Failed);
        assert_eq!(
            failed.error_message.as_deref(),
            Some("GPU ran out of memory.")
        );

        set(r#"{"state":"succeeded"}"#);
        std::fs::write(job_dir.join(MODEL_FILE), b"glb").unwrap();
        let done = provider.task_status(&job).unwrap();
        assert_eq!(done.status, ProviderStatus::Success);
        assert_eq!(done.outputs.len(), 1);
        assert_eq!(provider.download(&done.outputs[0].url, 10).unwrap(), b"glb");
        assert!(provider.task_status("../x").is_err());
    }

    #[test]
    fn downloads_are_confined_to_allowlisted_job_files() {
        let dir = tempfile::tempdir().unwrap();
        let provider = LocalTriposrProvider::new(engine(dir.path()));
        let job = provider
            .upload_image(vec![0], "a.png", "image/png")
            .unwrap();
        for url in [
            format!("local://{job}/input.png"),
            format!("local://{job}/job.json"),
            "local://../secrets/model.glb".to_string(),
            "https://example.com/model.glb".to_string(),
            format!("local://{job}/../../model.glb"),
        ] {
            assert!(provider.download(&url, 100).is_err(), "{url}");
        }
    }

    #[test]
    fn unsupported_operations_are_explicit() {
        let dir = tempfile::tempdir().unwrap();
        let provider = LocalTriposrProvider::new(engine(dir.path()));
        assert!(provider.account_balance().is_err());
        let text = TextTo3dRequest {
            prompt: "x".into(),
            negative_prompt: None,
            settings: Default::default(),
        };
        assert_eq!(
            provider.text_to_3d(&text).unwrap_err().kind,
            ErrorKind::Validation
        );
        assert!(!provider.requires_api_key());
    }
}

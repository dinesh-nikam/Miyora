//! Application service: the only entry point the desktop shell uses.
//! Orchestrates credentials, providers, validation, storage and polling.

use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::credentials::{credential_reference, normalize_api_key, CredentialStore};
use crate::decimate::{decimate, DecimateOptions};
use crate::error::{ErrorKind, ProviderError, StudioError};
use crate::export::{self, ExportOptions, LocalExportFormat};
use crate::hf::{self, HfSpaceEngine, HfTrellisProvider};
use crate::local::{self, LocalEngine, LocalTriposrProvider};
use crate::media::{self, ImageCheck};
use crate::prompt::{self, BuiltPrompt, Preset, PromptIntent, PRESETS};
use crate::provider::{
    unsupported_settings, AccountBalance, Capability, GenerationProvider, GenerationSettings,
    ImageTo3dRequest, ModelInfo, OutputKind, ProviderLimits, ProviderStatus, ProviderTask,
    TextTo3dRequest,
};
use crate::store::{
    clean_name, new_id, AssetDetail, AssetSummary, ExportView, NewExport, NewGeneration, NewInput,
    NewOutput, ProjectView, StudioStore, TaskRecord, TaskView,
};
use crate::task::{state_for_provider_status, PollPolicy, TaskState};
use crate::transport::Transport;
use crate::tripo::{self, TripoProvider};

pub const STUDIO_DIR: &str = "studio";
/// Display order; the first available provider is the UI default.
pub const PROVIDER_IDS: [&str; 3] = [local::PROVIDER_ID, hf::PROVIDER_ID, tripo::PROVIDER_ID];
const MAX_MODEL_DOWNLOAD_BYTES: u64 = 512 * 1024 * 1024;
const MAX_PREVIEW_DOWNLOAD_BYTES: u64 = 20 * 1024 * 1024;
const POLL_POLICY_KEY: &str = "poll_policy";
const DEFAULT_TASK_LIST: u32 = 200;
const DEFAULT_ASSET_NAME_CHARS: usize = 48;
const EXPORT_STEM_MAX_CHARS: usize = 60;
pub const RETOPOLOGY_OPERATION: &str = "retopology";
/// Provider id recorded for on-device mesh processing (not a generation engine).
pub const LOCAL_MESH_PROVIDER: &str = "local-mesh";
const RETOPOLOGY_MODEL: &str = "quadric-edge-collapse";
/// Seeds are generated locally when unset so every generation is reproducible.
const SEED_MASK: u32 = 0x7FFF_FFFF;

pub type ProviderFactory =
    Arc<dyn Fn(&str, String) -> Option<Box<dyn GenerationProvider>> + Send + Sync>;

/// Production factory: one shared HTTPS transport and shared engines.
/// `hf_jobs_dir` holds Hugging Face Space job status files.
pub fn provider_factory(
    transport: Arc<dyn Transport>,
    local_engine: Arc<LocalEngine>,
    hf_jobs_dir: PathBuf,
) -> ProviderFactory {
    let hf_engine = HfSpaceEngine::new(transport.clone(), hf_jobs_dir);
    Arc::new(
        move |id: &str, api_key: String| -> Option<Box<dyn GenerationProvider>> {
            match id {
                tripo::PROVIDER_ID => {
                    Some(Box::new(TripoProvider::new(transport.clone(), api_key)))
                }
                local::PROVIDER_ID => {
                    Some(Box::new(LocalTriposrProvider::new(local_engine.clone())))
                }
                hf::PROVIDER_ID => {
                    Some(Box::new(HfTrellisProvider::new(hf_engine.clone(), api_key)))
                }
                _ => None,
            }
        },
    )
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInfo {
    pub id: String,
    pub label: String,
    pub requires_api_key: bool,
    pub accepts_api_key: bool,
    pub has_api_key: bool,
    /// `None` when usable; otherwise why not (e.g. engine not installed).
    pub unavailable_reason: Option<String>,
    pub supported_settings: Vec<String>,
    /// Ready to generate: key stored (cloud) or engine installed (local).
    pub configured: bool,
    pub capabilities: Vec<Capability>,
    pub models: Vec<ModelInfo>,
    pub default_model: String,
    pub limits: ProviderLimits,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudioOverview {
    pub providers: Vec<ProviderInfo>,
    pub presets: Vec<Preset>,
    pub poll_policy: PollPolicy,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextSubmission {
    pub project_id: String,
    pub asset_id: Option<String>,
    pub asset_name: Option<String>,
    pub provider: String,
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub intent: Option<PromptIntent>,
    #[serde(default)]
    pub settings: GenerationSettings,
    pub parent_version_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageSubmission {
    pub project_id: String,
    pub asset_id: Option<String>,
    pub asset_name: Option<String>,
    pub provider: String,
    pub image_path: String,
    #[serde(default)]
    pub settings: GenerationSettings,
    pub parent_version_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetopologyRequest {
    pub generation_id: String,
    pub target_triangles: usize,
    pub preserve_borders: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportPlan {
    pub suggested_file_name: String,
    pub extension: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageInspection {
    pub file_name: String,
    pub check: ImageCheck,
    pub preview_data_url: String,
}

enum PollOutcome {
    Pending,
    Finished,
}

#[derive(Clone)]
pub struct StudioService {
    store: StudioStore,
    root: PathBuf,
    credentials: Arc<dyn CredentialStore>,
    factory: ProviderFactory,
    pollers: Arc<Mutex<HashSet<String>>>,
    /// False only in tests, which drive `run_poller` deterministically.
    background_polling: bool,
}

impl StudioService {
    pub fn open(
        app_data_dir: &Path,
        credentials: Arc<dyn CredentialStore>,
        factory: ProviderFactory,
    ) -> Result<Self, StudioError> {
        let root = app_data_dir.join(STUDIO_DIR);
        let store = StudioStore::open(&root)?;
        if store.list_projects()?.is_empty() {
            store.create_project("My First Project")?;
        }
        Ok(Self {
            store,
            root,
            credentials,
            factory,
            pollers: Arc::default(),
            background_polling: true,
        })
    }

    fn metadata_provider(&self, id: &str) -> Result<Box<dyn GenerationProvider>, StudioError> {
        (self.factory)(id, String::new())
            .ok_or_else(|| StudioError::NotFound(format!("provider {id}")))
    }

    fn provider(&self, id: &str) -> Result<Box<dyn GenerationProvider>, StudioError> {
        let metadata = self.metadata_provider(id)?;
        metadata.availability().map_err(|reason| {
            StudioError::Provider(ProviderError::new(ErrorKind::ProviderUnavailable, reason))
        })?;
        if !metadata.accepts_api_key() {
            return Ok(metadata);
        }
        let key = match self.store.credential_ref(id)? {
            Some(reference) => self
                .credentials
                .get(&reference)
                .map_err(StudioError::Credential)?,
            None => None,
        };
        if key.is_none() && metadata.requires_api_key() {
            return Err(StudioError::MissingCredential(id.into()));
        }
        (self.factory)(id, key.unwrap_or_default())
            .ok_or_else(|| StudioError::NotFound(format!("provider {id}")))
    }

    fn default_model_key(provider: &str) -> String {
        format!("default_model.{provider}")
    }

    pub fn overview(&self) -> Result<StudioOverview, StudioError> {
        let providers = PROVIDER_IDS
            .iter()
            .map(|id| {
                let provider = self.metadata_provider(id)?;
                let unavailable_reason = provider.availability().err();
                let configured = if provider.requires_api_key() {
                    self.store.credential_ref(id)?.is_some()
                } else {
                    unavailable_reason.is_none()
                };
                Ok(ProviderInfo {
                    id: (*id).into(),
                    label: provider.label().into(),
                    requires_api_key: provider.requires_api_key(),
                    accepts_api_key: provider.accepts_api_key(),
                    has_api_key: self.store.credential_ref(id)?.is_some(),
                    unavailable_reason,
                    supported_settings: provider
                        .supported_settings()
                        .iter()
                        .map(|name| (*name).into())
                        .collect(),
                    configured,
                    capabilities: provider.capabilities().to_vec(),
                    models: provider.models(),
                    default_model: self
                        .store
                        .setting(&Self::default_model_key(id))?
                        .unwrap_or_else(|| provider.default_model().into()),
                    limits: provider.limits(),
                })
            })
            .collect::<Result<_, StudioError>>()?;
        Ok(StudioOverview {
            providers,
            presets: PRESETS.to_vec(),
            poll_policy: self.poll_policy()?,
        })
    }

    pub fn poll_policy(&self) -> Result<PollPolicy, StudioError> {
        Ok(self
            .store
            .setting(POLL_POLICY_KEY)?
            .and_then(|json| serde_json::from_str::<PollPolicy>(&json).ok())
            .and_then(|policy| policy.validated().ok())
            .unwrap_or_default())
    }

    pub fn set_poll_policy(&self, policy: PollPolicy) -> Result<PollPolicy, StudioError> {
        let policy = policy.validated().map_err(StudioError::InvalidInput)?;
        let json = serde_json::to_string(&policy)
            .map_err(|error| StudioError::InvalidInput(error.to_string()))?;
        self.store.set_setting(POLL_POLICY_KEY, &json)?;
        Ok(policy)
    }

    pub fn set_default_model(&self, provider: &str, model: &str) -> Result<(), StudioError> {
        let known = self
            .metadata_provider(provider)?
            .models()
            .iter()
            .any(|entry| entry.id == model);
        if !known {
            return Err(StudioError::InvalidInput(format!(
                "Unknown model {model} for {provider}."
            )));
        }
        self.store
            .set_setting(&Self::default_model_key(provider), model)
    }

    pub fn save_api_key(&self, provider: &str, raw_key: &str) -> Result<(), StudioError> {
        if !self.metadata_provider(provider)?.accepts_api_key() {
            return Err(StudioError::InvalidInput(format!(
                "{provider} does not use an API key."
            )));
        }
        let key = normalize_api_key(raw_key).map_err(StudioError::InvalidInput)?;
        let reference = credential_reference(provider);
        self.credentials
            .set(&reference, &key)
            .map_err(StudioError::Credential)?;
        self.store.set_credential_ref(provider, &reference)?;
        log_event("credential_saved", None, provider, "ok", None, None);
        Ok(())
    }

    pub fn delete_api_key(&self, provider: &str) -> Result<(), StudioError> {
        if let Some(reference) = self.store.credential_ref(provider)? {
            self.credentials
                .delete(&reference)
                .map_err(StudioError::Credential)?;
        }
        self.store.remove_credential_ref(provider)?;
        log_event("credential_deleted", None, provider, "ok", None, None);
        Ok(())
    }

    pub fn account_balance(&self, provider: &str) -> Result<AccountBalance, StudioError> {
        let provider = self.provider(provider)?;
        let started = Instant::now();
        let result = provider.account_balance();
        log_result("balance", None, provider.id(), &result, started);
        Ok(result?)
    }

    pub fn build_prompt(
        &self,
        provider: &str,
        intent: &PromptIntent,
        preset_id: Option<&str>,
    ) -> Result<BuiltPrompt, StudioError> {
        let limits = self.metadata_provider(provider)?.limits();
        let preset = match preset_id {
            Some(id) => Some(
                prompt::preset(id).ok_or_else(|| StudioError::NotFound(format!("preset {id}")))?,
            ),
            None => None,
        };
        prompt::build_prompt(intent, preset, &limits).map_err(StudioError::InvalidInput)
    }

    pub fn list_projects(&self) -> Result<Vec<ProjectView>, StudioError> {
        self.store.list_projects()
    }

    pub fn create_project(&self, name: &str) -> Result<ProjectView, StudioError> {
        self.store.create_project(name)
    }

    pub fn list_assets(&self, project_id: &str) -> Result<Vec<AssetSummary>, StudioError> {
        self.store.list_assets(project_id)
    }

    pub fn asset_detail(&self, asset_id: &str) -> Result<AssetDetail, StudioError> {
        self.store.asset_detail(asset_id)
    }

    pub fn list_tasks(&self) -> Result<Vec<TaskView>, StudioError> {
        self.store.list_tasks(DEFAULT_TASK_LIST)
    }

    /// Reads a user-chosen image under the provider's limits and returns a small preview.
    pub fn inspect_image(
        &self,
        provider: &str,
        path: &str,
    ) -> Result<ImageInspection, StudioError> {
        let limits = self.metadata_provider(provider)?.limits();
        let (bytes, file_name) = read_user_file(Path::new(path), limits.image_max_bytes)?;
        let check = media::validate_image(&bytes, limits.image_max_bytes)
            .map_err(StudioError::InvalidInput)?;
        let preview_data_url =
            media::preview_data_url(&bytes).map_err(StudioError::InvalidInput)?;
        Ok(ImageInspection {
            file_name,
            check,
            preview_data_url,
        })
    }

    fn resolve_settings(
        &self,
        provider: &dyn GenerationProvider,
        mut settings: GenerationSettings,
    ) -> Result<GenerationSettings, StudioError> {
        if settings.model.is_none() {
            let stored = self
                .store
                .setting(&Self::default_model_key(provider.id()))?;
            settings.model = Some(stored.unwrap_or_else(|| provider.default_model().into()));
        }
        let supported = provider.supported_settings();
        let unsupported = unsupported_settings(&settings, supported);
        if !unsupported.is_empty() {
            return Err(StudioError::InvalidInput(format!(
                "{} does not support: {}.",
                provider.label(),
                unsupported.join(", ")
            )));
        }
        if supported.contains(&"modelSeed") {
            settings.model_seed.get_or_insert_with(random_seed);
        }
        if supported.contains(&"textureSeed")
            && (settings.texture != Some(false) || settings.pbr == Some(true))
        {
            settings.texture_seed.get_or_insert_with(random_seed);
        }
        Ok(settings)
    }

    pub fn submit_text_to_3d(&self, submission: TextSubmission) -> Result<TaskView, StudioError> {
        let provider = self.provider(&submission.provider)?;
        let mut settings = self.resolve_settings(provider.as_ref(), submission.settings)?;
        if provider.supported_settings().contains(&"imageSeed") {
            settings.image_seed.get_or_insert_with(random_seed);
        }
        let request = TextTo3dRequest {
            prompt: submission.prompt.trim().to_owned(),
            negative_prompt: submission
                .negative_prompt
                .map(|text| text.trim().to_owned())
                .filter(|text| !text.is_empty()),
            settings,
        };
        let started = Instant::now();
        let result = provider.text_to_3d(&request);
        log_result("submit_text_to_3d", None, provider.id(), &result, started);
        let provider_task_id = result?;
        let default_name: String = request
            .prompt
            .chars()
            .take(DEFAULT_ASSET_NAME_CHARS)
            .collect();
        let view = self.store.record_submission(NewGeneration {
            project_id: &submission.project_id,
            asset_id: submission.asset_id.as_deref(),
            asset_name: non_blank(submission.asset_name.as_deref()).unwrap_or(&default_name),
            parent_version_id: submission.parent_version_id.as_deref(),
            operation: "text_to_3d",
            provider: provider.id(),
            model: request
                .settings
                .model
                .as_deref()
                .unwrap_or(provider.default_model()),
            prompt: Some(&request.prompt),
            negative_prompt: request.negative_prompt.as_deref(),
            intent_json: submission
                .intent
                .as_ref()
                .and_then(|intent| serde_json::to_string(intent).ok()),
            settings_json: settings_json(&request.settings)?,
            provider_task_id: &provider_task_id,
            generation_id: new_id(),
            inputs: Vec::new(),
        })?;
        self.spawn_poller(&view.id);
        Ok(view)
    }

    pub fn submit_image_to_3d(&self, submission: ImageSubmission) -> Result<TaskView, StudioError> {
        let provider = self.provider(&submission.provider)?;
        let max_bytes = provider.limits().image_max_bytes;
        let (bytes, file_name) = read_user_file(Path::new(&submission.image_path), max_bytes)?;
        let check = media::validate_image(&bytes, max_bytes).map_err(StudioError::InvalidInput)?;
        let settings = self.resolve_settings(provider.as_ref(), submission.settings)?;
        let generation_id = new_id();
        let input = self.store_input(&generation_id, &bytes, &file_name, &check)?;
        let (upload, mime, extension) =
            media::upload_copy(&bytes, &check).map_err(StudioError::InvalidInput)?;
        let started = Instant::now();
        let result = provider
            .upload_image(upload, &format!("input.{extension}"), mime)
            .and_then(|file_token| {
                provider.image_to_3d(&ImageTo3dRequest {
                    file_token,
                    settings: settings.clone(),
                })
            });
        log_result("submit_image_to_3d", None, provider.id(), &result, started);
        let provider_task_id = result?;
        let stem = Path::new(&file_name)
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Image model".into());
        let view = self.store.record_submission(NewGeneration {
            project_id: &submission.project_id,
            asset_id: submission.asset_id.as_deref(),
            asset_name: non_blank(submission.asset_name.as_deref()).unwrap_or(&stem),
            parent_version_id: submission.parent_version_id.as_deref(),
            operation: "image_to_3d",
            provider: provider.id(),
            model: settings
                .model
                .as_deref()
                .unwrap_or(provider.default_model()),
            prompt: None,
            negative_prompt: None,
            intent_json: None,
            settings_json: settings_json(&settings)?,
            provider_task_id: &provider_task_id,
            generation_id,
            inputs: vec![input],
        })?;
        self.spawn_poller(&view.id);
        Ok(view)
    }

    /// Keeps an untouched copy of the user's image under managed storage.
    fn store_input(
        &self,
        generation_id: &str,
        bytes: &[u8],
        file_name: &str,
        check: &ImageCheck,
    ) -> Result<NewInput, StudioError> {
        let relative = format!("inputs/{generation_id}/source.{}", check.extension);
        write_atomic(&self.managed_path(&relative)?, bytes)?;
        Ok(NewInput {
            role: "source",
            original_name: file_name.into(),
            stored_path: relative,
            sha256: check.sha256.clone(),
            mime: check.mime.into(),
            width: check.width,
            height: check.height,
            byte_size: check.byte_size,
        })
    }

    /// Submits the same request again as a new version (consumes credits).
    pub fn regenerate(&self, generation_id: &str) -> Result<TaskView, StudioError> {
        let generation = self.store.generation(generation_id)?;
        let asset_id = generation
            .task
            .as_ref()
            .map(|task| task.asset_id.clone())
            .ok_or_else(|| StudioError::NotFound(format!("task for generation {generation_id}")))?;
        let project_id = self.store.asset_project(&asset_id)?;
        let stored: GenerationSettings =
            serde_json::from_value(generation.settings.clone()).unwrap_or_default();
        // Fresh seeds give a genuinely new variation; the rest is reproduced.
        let settings = GenerationSettings {
            model_seed: None,
            texture_seed: None,
            image_seed: None,
            ..stored
        };
        let parent_version_id = generation
            .version_id
            .clone()
            .or(generation.parent_version_id.clone());
        match generation.operation.as_str() {
            "text_to_3d" => self.submit_text_to_3d(TextSubmission {
                project_id,
                asset_id: Some(asset_id),
                asset_name: None,
                provider: generation.provider,
                prompt: generation.prompt.unwrap_or_default(),
                negative_prompt: generation.negative_prompt,
                intent: generation
                    .intent
                    .and_then(|value| serde_json::from_value(value).ok()),
                settings,
                parent_version_id,
            }),
            "image_to_3d" => {
                let (_, relative) = self
                    .store
                    .generation_inputs(generation_id)?
                    .into_iter()
                    .next()
                    .ok_or_else(|| StudioError::NotFound("stored source image".into()))?;
                self.submit_image_to_3d(ImageSubmission {
                    project_id,
                    asset_id: Some(asset_id),
                    asset_name: None,
                    provider: generation.provider,
                    image_path: self.managed_path(&relative)?.to_string_lossy().into_owned(),
                    settings,
                    parent_version_id,
                })
            }
            other => Err(StudioError::InvalidInput(format!(
                "Unsupported operation {other}."
            ))),
        }
    }

    pub fn resume_task(&self, task_id: &str) -> Result<TaskView, StudioError> {
        let view = self.store.resume_task(task_id)?;
        self.spawn_poller(task_id);
        Ok(view)
    }

    /// Restarts tracking for unfinished tasks (e.g. after an app restart).
    pub fn resume_active_tasks(&self) -> Result<usize, StudioError> {
        let ids = self.store.active_task_ids()?;
        for id in &ids {
            self.spawn_poller(id);
        }
        Ok(ids.len())
    }

    /// Reduces a version's polygon count on this computer and stores the result
    /// as a new version whose parent is the source version.
    pub fn retopologize(&self, request: RetopologyRequest) -> Result<TaskView, StudioError> {
        let source = self.store.generation(&request.generation_id)?;
        let output = source
            .outputs
            .iter()
            .find(|output| output.kind == OutputKind::Model.as_str())
            .ok_or_else(|| StudioError::NotFound("model output".into()))?;
        if output.format != "glb" {
            return Err(StudioError::InvalidInput(format!(
                "Polygon reduction needs a GLB model; this one is {}.",
                output.format.to_uppercase()
            )));
        }
        let asset_id = source
            .task
            .as_ref()
            .map(|task| task.asset_id.clone())
            .ok_or_else(|| StudioError::NotFound("source task".into()))?;
        let project_id = self.store.asset_project(&asset_id)?;
        let (relative, _) = self.store.output_location(output.id)?;
        let mesh = export::read_mesh(&std::fs::read(self.managed_path(&relative)?)?)
            .map_err(StudioError::InvalidInput)?;
        let started = Instant::now();
        let options = DecimateOptions {
            target_triangles: request.target_triangles,
            preserve_borders: request.preserve_borders,
        };
        let (reduced, report) = decimate(&mesh, options).map_err(StudioError::InvalidInput)?;
        let glb = export::write_glb(&reduced);
        let model_report = media::inspect_model(&glb).map_err(|reason| {
            StudioError::InvalidInput(format!("Reduced model failed validation: {reason}"))
        })?;
        let generation_id = new_id();
        let settings_json = serde_json::json!({
            "algorithm": RETOPOLOGY_MODEL,
            "targetTriangles": request.target_triangles,
            "preserveBorders": request.preserve_borders,
            "sourceGenerationId": request.generation_id,
            "sourceTriangles": report.source_triangles,
            "resultTriangles": report.result_triangles,
            "sourceVertices": report.source_vertices,
            "resultVertices": report.result_vertices,
        })
        .to_string();
        let view = self.store.record_submission(NewGeneration {
            project_id: &project_id,
            asset_id: Some(&asset_id),
            asset_name: "",
            parent_version_id: source.version_id.as_deref(),
            operation: RETOPOLOGY_OPERATION,
            provider: LOCAL_MESH_PROVIDER,
            model: RETOPOLOGY_MODEL,
            prompt: None,
            negative_prompt: None,
            intent_json: None,
            settings_json,
            provider_task_id: &generation_id,
            generation_id: generation_id.clone(),
            inputs: Vec::new(),
        })?;
        self.store
            .update_task(&view.id, TaskState::Running, Some("reducing"), None)?;
        let stored_path = format!("assets/{asset_id}/{generation_id}/model.glb");
        write_atomic(&self.managed_path(&stored_path)?, &glb)?;
        let done = self.store.complete_task(
            &view.id,
            &[NewOutput {
                kind: OutputKind::Model.as_str(),
                label: "model".into(),
                format: "glb".into(),
                stored_path,
                sha256: media::sha256_hex(&glb),
                byte_size: glb.len() as u64,
                report_json: serde_json::to_string(&model_report).ok(),
            }],
            None,
        )?;
        log_event(
            "retopology",
            Some(&view.id),
            LOCAL_MESH_PROVIDER,
            "succeeded",
            None,
            Some(started.elapsed()),
        );
        Ok(done)
    }

    /// File name and extension to suggest in the save dialog.
    pub fn export_plan(
        &self,
        output_id: i64,
        format: LocalExportFormat,
    ) -> Result<ExportPlan, StudioError> {
        let source = self.store.output_source(output_id)?;
        if source.kind != OutputKind::Model.as_str() {
            return Err(StudioError::InvalidInput(
                "Only model outputs can be exported.".into(),
            ));
        }
        if format != LocalExportFormat::Original && source.format != "glb" {
            return Err(StudioError::InvalidInput(format!(
                "Conversion needs a GLB model; this one is {}. Export the original instead.",
                source.format.to_uppercase()
            )));
        }
        let extension = match format {
            LocalExportFormat::Original => source.format.clone(),
            other => other.as_str().to_owned(),
        };
        let stem: String = source
            .asset_name
            .chars()
            .map(|ch| {
                if ch.is_alphanumeric() || ch == '-' || ch == '_' {
                    ch
                } else {
                    '_'
                }
            })
            .take(EXPORT_STEM_MAX_CHARS)
            .collect();
        let stem = if stem.trim_matches('_').is_empty() {
            "model".to_owned()
        } else {
            stem
        };
        Ok(ExportPlan {
            suggested_file_name: format!("{stem}.{extension}"),
            extension,
        })
    }

    /// Writes the model to a user-chosen path, verifies what was written, records it.
    pub fn export_output(
        &self,
        output_id: i64,
        format: LocalExportFormat,
        options: ExportOptions,
        destination: &Path,
    ) -> Result<ExportView, StudioError> {
        let plan = self.export_plan(output_id, format)?;
        let options = options.validated().map_err(StudioError::InvalidInput)?;
        let destination = destination.with_extension(&plan.extension);
        let folder = destination
            .parent()
            .filter(|folder| folder.is_dir())
            .ok_or_else(|| {
                StudioError::InvalidInput("The destination folder does not exist.".into())
            })?;
        let source = self.store.output_source(output_id)?;
        let bytes = std::fs::read(self.managed_path(&source.stored_path)?)?;
        let started = Instant::now();
        let mut files = vec![destination.clone()];
        let (written, triangles) = match format {
            LocalExportFormat::Original => {
                export::write_atomic(&destination, &bytes)?;
                if media::sha256_hex(&std::fs::read(&destination)?) != media::sha256_hex(&bytes) {
                    return Err(StudioError::InvalidInput(
                        "Export verification failed: copy differs.".into(),
                    ));
                }
                (bytes.len() as u64, None)
            }
            converted => {
                let mesh = export::read_mesh(&bytes).map_err(StudioError::InvalidInput)?;
                let main = match converted {
                    LocalExportFormat::Stl => export::write_stl(&mesh, &options),
                    LocalExportFormat::Ply => export::write_ply(&mesh, &options),
                    _ => {
                        let stem = destination
                            .file_stem()
                            .map(|stem| stem.to_string_lossy().into_owned())
                            .unwrap_or_default();
                        let obj = export::write_obj(&mesh, &options, &stem);
                        for (name, data) in &obj.sidecars {
                            let path = folder.join(name);
                            export::write_atomic(&path, data)?;
                            if std::fs::metadata(&path)?.len() != data.len() as u64 {
                                return Err(StudioError::InvalidInput(format!(
                                    "Export verification failed: {name}."
                                )));
                            }
                            files.push(path);
                        }
                        obj.obj
                    }
                };
                export::write_atomic(&destination, &main)?;
                let reread = std::fs::read(&destination)?;
                export::verify(converted, &reread, mesh.positions.len(), mesh.triangles())
                    .map_err(StudioError::InvalidInput)?;
                (reread.len() as u64, Some(mesh.triangles() as u64))
            }
        };
        let view = self.store.record_export(&NewExport {
            generation_id: source.generation_id,
            output_id,
            format: format.as_str().into(),
            options_json: serde_json::to_string(&options).unwrap_or_default(),
            destination: destination.to_string_lossy().into_owned(),
            files: files
                .iter()
                .map(|path| path.to_string_lossy().into_owned())
                .collect(),
            byte_size: written,
            sha256: media::sha256_hex(&std::fs::read(&destination)?),
            triangles,
        })?;
        log_event(
            "export",
            None,
            "local",
            format.as_str(),
            None,
            Some(started.elapsed()),
        );
        Ok(view)
    }

    pub fn read_output(&self, output_id: i64) -> Result<Vec<u8>, StudioError> {
        let (relative, _) = self.store.output_location(output_id)?;
        Ok(std::fs::read(self.managed_path(&relative)?)?)
    }

    pub fn output_path(&self, output_id: i64) -> Result<PathBuf, StudioError> {
        let (relative, _) = self.store.output_location(output_id)?;
        self.managed_path(&relative)
    }

    fn managed_path(&self, relative: &str) -> Result<PathBuf, StudioError> {
        let path = Path::new(relative);
        if !path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
        {
            return Err(StudioError::InvalidInput(
                "Stored path escapes the studio folder.".into(),
            ));
        }
        Ok(self.root.join(path))
    }

    fn spawn_poller(&self, task_id: &str) {
        if !self.background_polling {
            return;
        }
        let claim = |insert: bool| {
            let mut active = self
                .pollers
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if insert {
                active.insert(task_id.to_owned())
            } else {
                active.remove(task_id)
            }
        };
        if !claim(true) {
            return; // already tracked
        }
        let service = self.clone();
        let owned_id = task_id.to_owned();
        let spawned = std::thread::Builder::new()
            .name("studio-task-poller".into())
            .spawn(move || {
                let policy = service.poll_policy().unwrap_or_default();
                service.run_poller(&owned_id, policy, &std::thread::sleep);
                service
                    .pollers
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .remove(&owned_id);
            });
        if spawned.is_err() {
            claim(false);
            log_event("poller_spawn", Some(task_id), "-", "error", None, None);
        }
    }

    /// Bounded polling loop: stops on terminal state, timeout, or error budget.
    pub fn run_poller(&self, task_id: &str, policy: PollPolicy, sleep: &dyn Fn(Duration)) {
        let started = Instant::now();
        let mut consecutive_errors = 0u32;
        loop {
            if started.elapsed() >= Duration::from_secs(policy.timeout_secs) {
                let error = ProviderError::new(
                    ErrorKind::TaskTimeout,
                    format!(
                        "Stopped tracking after {} minutes; the provider may still finish.",
                        policy.timeout_secs / 60
                    ),
                );
                self.record_failure(task_id, &error);
                return;
            }
            match self.poll_once(task_id) {
                Ok(PollOutcome::Finished) => return,
                Ok(PollOutcome::Pending) => {
                    consecutive_errors = 0;
                    sleep(policy.next_delay(0, None));
                }
                Err(StudioError::Provider(error))
                    if matches!(
                        error.kind,
                        ErrorKind::RateLimit | ErrorKind::ProviderUnavailable
                    ) =>
                {
                    consecutive_errors += 1;
                    if consecutive_errors >= policy.max_consecutive_errors {
                        self.record_failure(task_id, &error);
                        return;
                    }
                    if self.store.note_transient_error(task_id, &error).is_err() {
                        log_event(
                            "poll_note",
                            Some(task_id),
                            "-",
                            "store_error",
                            Some(error.kind),
                            None,
                        );
                    }
                    sleep(policy.next_delay(consecutive_errors, error.retry_after_secs));
                }
                Err(StudioError::Provider(error)) => {
                    self.record_failure(task_id, &error);
                    return;
                }
                Err(StudioError::MissingCredential(provider)) => {
                    let error = ProviderError::new(
                        ErrorKind::Authentication,
                        format!("No API key for {provider}; add one in settings."),
                    );
                    self.record_failure(task_id, &error);
                    return;
                }
                Err(other) => {
                    let error = ProviderError::new(
                        ErrorKind::Conversion,
                        format!("Local processing failed: {other}"),
                    );
                    self.record_failure(task_id, &error);
                    return;
                }
            }
        }
    }

    fn record_failure(&self, task_id: &str, error: &ProviderError) {
        let status = match self.store.fail_task(task_id, TaskState::Failed, error) {
            Ok(_) => "recorded",
            Err(_) => "store_error",
        };
        log_event(
            "task_failed",
            Some(task_id),
            "-",
            status,
            Some(error.kind),
            None,
        );
    }

    fn poll_once(&self, task_id: &str) -> Result<PollOutcome, StudioError> {
        let task = self.store.task(task_id)?;
        if task.state.is_terminal() {
            return Ok(PollOutcome::Finished);
        }
        let provider = self.provider(&task.provider)?;
        let remote = provider.task_status(&task.provider_task_id)?;
        match (remote.status, state_for_provider_status(remote.status)) {
            (_, None) => {
                self.store
                    .update_task(task_id, task.state, Some("unknown_status"), None)?;
                Ok(PollOutcome::Pending)
            }
            (ProviderStatus::Success, _) => {
                self.store.update_task(
                    task_id,
                    TaskState::Running,
                    Some("downloading"),
                    remote.progress,
                )?;
                let started = Instant::now();
                let outputs = self.collect_outputs(provider.as_ref(), &task, &remote)?;
                self.store
                    .complete_task(task_id, &outputs, remote.credits_consumed)?;
                log_event(
                    "task_completed",
                    Some(task_id),
                    provider.id(),
                    "succeeded",
                    None,
                    Some(started.elapsed()),
                );
                Ok(PollOutcome::Finished)
            }
            (_, Some(state)) if state.is_terminal() => {
                let kind = if state == TaskState::Expired {
                    ErrorKind::TaskTimeout
                } else {
                    ErrorKind::TaskFailed
                };
                let message = remote
                    .error_message
                    .clone()
                    .unwrap_or_else(|| format!("Provider reported {}.", state.as_str()));
                self.store
                    .fail_task(task_id, state, &ProviderError::new(kind, message))?;
                log_event(
                    "task_terminal",
                    Some(task_id),
                    provider.id(),
                    state.as_str(),
                    Some(kind),
                    None,
                );
                Ok(PollOutcome::Finished)
            }
            (_, Some(state)) => {
                let stage = if state == TaskState::Queued {
                    "queued"
                } else {
                    "generating"
                };
                self.store
                    .update_task(task_id, state, Some(stage), remote.progress)?;
                Ok(PollOutcome::Pending)
            }
        }
    }

    /// Downloads and validates outputs. A task only succeeds with a valid model file.
    fn collect_outputs(
        &self,
        provider: &dyn GenerationProvider,
        task: &TaskRecord,
        remote: &ProviderTask,
    ) -> Result<Vec<NewOutput>, StudioError> {
        let mut outputs = Vec::new();
        for output in &remote.outputs {
            let label: String = output
                .label
                .chars()
                .map(|ch| {
                    if ch.is_ascii_alphanumeric() || ch == '_' {
                        ch.to_ascii_lowercase()
                    } else {
                        '_'
                    }
                })
                .take(64)
                .collect();
            let max_bytes = match output.kind {
                OutputKind::Model => MAX_MODEL_DOWNLOAD_BYTES,
                OutputKind::PreviewImage => MAX_PREVIEW_DOWNLOAD_BYTES,
            };
            let bytes = provider.download(&output.url, max_bytes)?;
            let (format, report_json) = match output.kind {
                OutputKind::Model => {
                    let report = media::inspect_model(&bytes).map_err(|reason| {
                        ProviderError::new(
                            ErrorKind::Conversion,
                            format!("Downloaded model failed validation: {reason}"),
                        )
                    })?;
                    (
                        report.format.to_owned(),
                        serde_json::to_string(&report).ok(),
                    )
                }
                OutputKind::PreviewImage => match media::validate_image(&bytes, max_bytes) {
                    Ok(check) => (check.extension.to_owned(), None),
                    Err(_) => {
                        // A broken preview must not discard a valid model.
                        log_event(
                            "preview_invalid",
                            Some(&task.id),
                            provider.id(),
                            "skipped",
                            Some(ErrorKind::Conversion),
                            None,
                        );
                        continue;
                    }
                },
            };
            let relative = format!(
                "assets/{}/{}/{label}.{format}",
                task.asset_id, task.generation_id
            );
            write_atomic(&self.managed_path(&relative)?, &bytes)?;
            outputs.push(NewOutput {
                kind: output.kind.as_str(),
                label,
                format,
                stored_path: relative,
                sha256: media::sha256_hex(&bytes),
                byte_size: bytes.len() as u64,
                report_json,
            });
        }
        if !outputs
            .iter()
            .any(|output| output.kind == OutputKind::Model.as_str())
        {
            return Err(ProviderError::new(
                ErrorKind::Conversion,
                "Provider reported success but returned no model file.",
            )
            .into());
        }
        Ok(outputs)
    }
}

fn non_blank(value: Option<&str>) -> Option<&str> {
    value.filter(|text| !text.trim().is_empty())
}

fn settings_json(settings: &GenerationSettings) -> Result<String, StudioError> {
    serde_json::to_string(settings).map_err(|error| StudioError::InvalidInput(error.to_string()))
}

fn random_seed() -> i64 {
    let bytes = uuid::Uuid::new_v4().into_bytes();
    i64::from(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) & SEED_MASK)
}

fn read_user_file(path: &Path, max_bytes: u64) -> Result<(Vec<u8>, String), StudioError> {
    let metadata = std::fs::metadata(path)
        .map_err(|_| StudioError::InvalidInput("The selected file could not be opened.".into()))?;
    if !metadata.is_file() {
        return Err(StudioError::InvalidInput(
            "The selection is not a file.".into(),
        ));
    }
    if metadata.len() > max_bytes {
        return Err(StudioError::InvalidInput(format!(
            "The file is {:.1} MB; the limit is {:.0} MB.",
            metadata.len() as f64 / 1_048_576.0,
            max_bytes as f64 / 1_048_576.0
        )));
    }
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok((
        std::fs::read(path)?,
        clean_name(&name).unwrap_or_else(|_| "image".into()),
    ))
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), StudioError> {
    let parent = path
        .parent()
        .ok_or_else(|| StudioError::InvalidInput("Invalid storage path.".into()))?;
    std::fs::create_dir_all(parent)?;
    let partial = path.with_extension("part");
    std::fs::write(&partial, bytes)?;
    std::fs::rename(&partial, path)?;
    Ok(())
}

fn log_result<T>(
    operation: &str,
    task_id: Option<&str>,
    provider: &str,
    result: &Result<T, ProviderError>,
    started: Instant,
) {
    let (status, kind) = match result {
        Ok(_) => ("ok", None),
        Err(error) => ("error", Some(error.kind)),
    };
    log_event(
        operation,
        task_id,
        provider,
        status,
        kind,
        Some(started.elapsed()),
    );
}

/// Structured, privacy-safe log line: no prompts, keys, URLs or file contents.
fn log_event(
    operation: &str,
    task_id: Option<&str>,
    provider: &str,
    status: &str,
    error_kind: Option<ErrorKind>,
    duration: Option<Duration>,
) {
    let line = serde_json::json!({
        "component": "studio",
        "operation": operation,
        "taskId": task_id,
        "provider": provider,
        "status": status,
        "errorKind": error_kind.map(ErrorKind::as_str),
        "durationMs": duration.map(|duration| duration.as_millis() as u64),
    });
    eprintln!("{line}");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::credentials::tests::MemoryCredentials;
    use crate::media::tests::{glb_bytes, png_bytes, SAMPLE_GLTF};
    use crate::transport::TransportError;
    use crate::tripo::tests::{ScriptedTransport, CREATE, SUCCESS, UPLOAD};

    const NO_SLEEP: &dyn Fn(Duration) = &|_| {};

    fn service(dir: &Path, transport: &ScriptedTransport) -> StudioService {
        let transport: Arc<dyn Transport> = Arc::new(transport.clone());
        let local = LocalEngine::new(
            dir.join("engine"),
            dir.join("jobs"),
            crate::local::LocalEngineConfig::default(),
        );
        let mut studio = StudioService::open(
            dir,
            Arc::new(MemoryCredentials::default()),
            provider_factory(transport, local, dir.join("hf-jobs")),
        )
        .unwrap();
        studio.background_polling = false;
        studio
    }

    fn tripo_info(studio: &StudioService) -> ProviderInfo {
        studio
            .overview()
            .unwrap()
            .providers
            .into_iter()
            .find(|provider| provider.id == "tripo")
            .unwrap()
    }

    #[test]
    fn local_engine_reports_unavailable_and_rejects_unsupported_settings() {
        let dir = tempfile::tempdir().unwrap();
        let studio = service(dir.path(), &ScriptedTransport::default());
        let overview = studio.overview().unwrap();
        let local = &overview.providers[0];
        assert_eq!(local.id, crate::local::PROVIDER_ID);
        assert!(!local.requires_api_key && !local.configured);
        assert!(local
            .unavailable_reason
            .as_deref()
            .unwrap()
            .contains("not installed"));
        let project = studio.list_projects().unwrap().remove(0).id;
        let image = dir.path().join("in.png");
        std::fs::write(&image, png_bytes(300, 300)).unwrap();
        let submission = |settings: GenerationSettings| ImageSubmission {
            project_id: project.clone(),
            asset_id: None,
            asset_name: None,
            provider: crate::local::PROVIDER_ID.into(),
            image_path: image.to_string_lossy().into_owned(),
            settings,
            parent_version_id: None,
        };
        let unavailable = studio.submit_image_to_3d(submission(GenerationSettings::default()));
        assert!(
            matches!(unavailable, Err(StudioError::Provider(error)) if error.kind == ErrorKind::ProviderUnavailable)
        );

        // Fake install (files only; nothing is executed before validation fails).
        let engine = dir.path().join("engine");
        std::fs::create_dir_all(&engine).unwrap();
        for file in ["python.exe", "run_triposr.py"] {
            std::fs::write(engine.join(file), b"").unwrap();
        }
        let manifest = serde_json::json!({
            "engine": "triposr", "python": engine.join("python.exe"), "runner": engine.join("run_triposr.py")
        });
        std::fs::write(engine.join("engine.json"), manifest.to_string()).unwrap();
        assert!(studio.overview().unwrap().providers[0].configured);
        let quad = GenerationSettings {
            quad: Some(true),
            face_limit: Some(5000),
            ..Default::default()
        };
        match studio.submit_image_to_3d(submission(quad)) {
            Err(StudioError::InvalidInput(message)) => {
                assert!(message.contains("faceLimit") && message.contains("quad"))
            }
            other => panic!("expected rejection, got {other:?}"),
        }
    }

    fn keyed(dir: &Path, transport: &ScriptedTransport) -> (StudioService, String) {
        let studio = service(dir, transport);
        studio.save_api_key("tripo", "tsk_test_key").unwrap();
        let project = studio.list_projects().unwrap().remove(0).id;
        (studio, project)
    }

    fn text(project_id: &str) -> TextSubmission {
        TextSubmission {
            project_id: project_id.into(),
            asset_id: None,
            asset_name: None,
            provider: "tripo".into(),
            prompt: " A wooden chest ".into(),
            negative_prompt: Some(" ".into()),
            intent: None,
            settings: GenerationSettings::default(),
            parent_version_id: None,
        }
    }

    fn policy() -> PollPolicy {
        PollPolicy {
            max_consecutive_errors: 2,
            ..PollPolicy::default()
        }
    }

    #[test]
    fn submission_without_key_never_reaches_network() {
        let dir = tempfile::tempdir().unwrap();
        let transport = ScriptedTransport::default();
        let studio = service(dir.path(), &transport);
        let project = studio.list_projects().unwrap().remove(0);
        assert!(matches!(
            studio.submit_text_to_3d(text(&project.id)),
            Err(StudioError::MissingCredential(_))
        ));
        assert!(transport.requests.lock().unwrap().is_empty());
    }

    #[test]
    fn api_key_lives_only_in_credential_store() {
        let dir = tempfile::tempdir().unwrap();
        let studio = service(dir.path(), &ScriptedTransport::default());
        studio.save_api_key("tripo", "tsk_SECRETVALUE123").unwrap();
        assert!(tripo_info(&studio).configured);
        let db_root = dir.path().join(STUDIO_DIR);
        for name in ["studio.sqlite", "studio.sqlite-wal"] {
            if let Ok(bytes) = std::fs::read(db_root.join(name)) {
                assert!(
                    !bytes
                        .windows(16)
                        .any(|window| window == b"tsk_SECRETVALUE1"),
                    "{name} leaked the key"
                );
            }
        }
        studio.delete_api_key("tripo").unwrap();
        assert!(!tripo_info(&studio).configured);
    }

    #[test]
    fn text_to_3d_end_to_end_downloads_validates_and_versions() {
        let dir = tempfile::tempdir().unwrap();
        let transport = ScriptedTransport::default();
        let (studio, project) = keyed(dir.path(), &transport);

        transport.push(200, CREATE.as_bytes());
        let task = studio.submit_text_to_3d(text(&project)).unwrap();
        assert_eq!(task.state, TaskState::Queued);
        let body = transport.requests.lock().unwrap()[0].json.clone().unwrap();
        assert_eq!(body["prompt"], "A wooden chest");
        assert!(body.get("negative_prompt").is_none());
        assert!(
            body["model_seed"].is_i64() && body["texture_seed"].is_i64(),
            "seeds recorded for reproducibility"
        );

        transport.push(
            200,
            br#"{"code":0,"data":{"task_id":"task_abc123","status":"running","progress":40}}"#,
        );
        transport.push(200, SUCCESS.as_bytes());
        transport.push(200, &glb_bytes(SAMPLE_GLTF));
        transport.push(200, &png_bytes(64, 64));
        studio.run_poller(&task.id, policy(), NO_SLEEP);

        let done = studio.store.task_view(&task.id).unwrap();
        assert_eq!(done.state, TaskState::Succeeded);
        let detail = studio.asset_detail(&task.asset_id).unwrap();
        assert_eq!(detail.name, "A wooden chest");
        let generation = &detail.generations[0];
        assert_eq!(generation.version_number, Some(1));
        assert_eq!(generation.model, "v3.1-20260211");
        let usage = generation.usage.as_ref().unwrap();
        assert_eq!((usage.credits, usage.source.as_str()), (100.0, "provider"));
        let model = generation
            .outputs
            .iter()
            .find(|output| output.kind == "model")
            .unwrap();
        assert_eq!(model.format, "glb");
        assert_eq!(
            studio.read_output(model.id).unwrap(),
            glb_bytes(SAMPLE_GLTF)
        );
        assert_eq!(model.report.as_ref().unwrap()["triangles"], 12);
        assert!(studio.list_assets(&project).unwrap()[0]
            .thumbnail_output_id
            .is_some());

        // Replaying the poll after completion changes nothing and makes no requests.
        let before = transport.requests.lock().unwrap().len();
        studio.run_poller(&task.id, policy(), NO_SLEEP);
        assert_eq!(transport.requests.lock().unwrap().len(), before);

        // Regenerate creates version 2 with lineage to version 1.
        transport.push(200, br#"{"code":0,"data":{"task_id":"task_def456"}}"#);
        let second = studio.regenerate(&task.generation_id).unwrap();
        assert_eq!(second.asset_id, task.asset_id);
        let lineage = studio.store.generation(&second.generation_id).unwrap();
        assert_eq!(lineage.parent_version_id, done.version_id);
    }

    #[test]
    fn image_to_3d_uploads_then_submits_and_keeps_original() {
        let dir = tempfile::tempdir().unwrap();
        let transport = ScriptedTransport::default();
        let (studio, project) = keyed(dir.path(), &transport);
        let source = dir.path().join("robot.png");
        std::fs::write(&source, png_bytes(300, 300)).unwrap();

        transport.push(200, UPLOAD.as_bytes());
        transport.push(200, CREATE.as_bytes());
        let task = studio
            .submit_image_to_3d(ImageSubmission {
                project_id: project,
                asset_id: None,
                asset_name: None,
                provider: "tripo".into(),
                image_path: source.to_string_lossy().into_owned(),
                settings: GenerationSettings::default(),
                parent_version_id: None,
            })
            .unwrap();
        assert_eq!(task.asset_name, "robot");
        let requests = transport.requests.lock().unwrap();
        assert!(requests[0].url.ends_with("/files"));
        assert_eq!(requests[1].json.as_ref().unwrap()["input"], "file_abc123");
        let inputs = studio.store.generation_inputs(&task.generation_id).unwrap();
        assert_eq!(inputs[0].0.width, 300);
        assert_eq!(
            std::fs::read(studio.managed_path(&inputs[0].1).unwrap()).unwrap(),
            png_bytes(300, 300)
        );
        assert_eq!(
            std::fs::read(&source).unwrap(),
            png_bytes(300, 300),
            "original untouched"
        );
    }

    #[test]
    fn export_writes_verifies_and_records_every_format() {
        let dir = tempfile::tempdir().unwrap();
        let transport = ScriptedTransport::default();
        let (studio, project) = keyed(dir.path(), &transport);
        let task = studio
            .store
            .record_submission(crate::store::tests::submission(&project, "task_abc123"))
            .unwrap();
        let glb = crate::export::tests::textured_glb();
        transport.push(200, SUCCESS.as_bytes());
        transport.push(200, &glb);
        transport.push(200, &png_bytes(64, 64));
        studio.run_poller(&task.id, policy(), NO_SLEEP);
        let detail = studio.asset_detail(&task.asset_id).unwrap();
        let outputs = &detail.generations[0].outputs;
        let model = outputs
            .iter()
            .find(|output| output.kind == "model")
            .unwrap()
            .id;
        let preview = outputs
            .iter()
            .find(|output| output.kind != "model")
            .unwrap()
            .id;

        let out = dir.path().join("exports");
        std::fs::create_dir_all(&out).unwrap();
        let plan = studio.export_plan(model, LocalExportFormat::Stl).unwrap();
        assert_eq!(plan.suggested_file_name, "Chest.stl");
        let defaults = ExportOptions::default();
        let original = studio
            .export_output(
                model,
                LocalExportFormat::Original,
                defaults,
                &out.join("a.bin"),
            )
            .unwrap();
        assert!(
            original.destination.ends_with("a.glb"),
            "extension forced to the real format"
        );
        assert_eq!(std::fs::read(out.join("a.glb")).unwrap(), glb);
        let print = ExportOptions {
            scale: 1000.0,
            z_up: true,
        };
        let stl = studio
            .export_output(model, LocalExportFormat::Stl, print, &out.join("print"))
            .unwrap();
        assert_eq!((stl.triangles, stl.byte_size), (Some(2), 84 + 2 * 50));
        let obj = studio
            .export_output(model, LocalExportFormat::Obj, defaults, &out.join("m.obj"))
            .unwrap();
        assert_eq!(obj.files.len(), 3, "obj + mtl + texture");
        assert!(out.join("m.mtl").is_file() && out.join("m_texture.png").is_file());
        studio
            .export_output(model, LocalExportFormat::Ply, defaults, &out.join("m.ply"))
            .unwrap();
        let history = studio.asset_detail(&task.asset_id).unwrap().generations[0]
            .exports
            .len();
        assert_eq!(history, 4);

        assert!(studio
            .export_output(
                preview,
                LocalExportFormat::Original,
                defaults,
                &out.join("x")
            )
            .is_err());
        let missing = dir.path().join("nope").join("x.stl");
        assert!(studio
            .export_output(model, LocalExportFormat::Stl, defaults, &missing)
            .is_err());
        let bad_scale = ExportOptions {
            scale: 0.0,
            z_up: false,
        };
        assert!(studio
            .export_output(model, LocalExportFormat::Stl, bad_scale, &out.join("y"))
            .is_err());
    }

    #[test]
    fn retopology_creates_reduced_child_version() {
        let dir = tempfile::tempdir().unwrap();
        let transport = ScriptedTransport::default();
        let (studio, project) = keyed(dir.path(), &transport);
        let task = studio
            .store
            .record_submission(crate::store::tests::submission(&project, "task_abc123"))
            .unwrap();
        let mut grid = crate::export::ExportMesh::default();
        let n = 20u32;
        for y in 0..=n {
            for x in 0..=n {
                grid.positions.push([x as f32, y as f32, 0.0]);
            }
        }
        for y in 0..n {
            for x in 0..n {
                let at = |x: u32, y: u32| y * (n + 1) + x;
                grid.indices
                    .extend([at(x, y), at(x + 1, y), at(x + 1, y + 1)]);
                grid.indices
                    .extend([at(x, y), at(x + 1, y + 1), at(x, y + 1)]);
            }
        }
        transport.push(200, SUCCESS.as_bytes());
        transport.push(200, &crate::export::write_glb(&grid));
        transport.push(200, &png_bytes(64, 64));
        studio.run_poller(&task.id, policy(), NO_SLEEP);
        let v1 = studio
            .store
            .task_view(&task.id)
            .unwrap()
            .version_id
            .unwrap();

        let reduced = studio
            .retopologize(RetopologyRequest {
                generation_id: task.generation_id.clone(),
                target_triangles: 100,
                preserve_borders: true,
            })
            .unwrap();
        assert_eq!(reduced.state, TaskState::Succeeded);
        assert_eq!(reduced.operation, RETOPOLOGY_OPERATION);
        let child = studio.store.generation(&reduced.generation_id).unwrap();
        assert_eq!(child.version_number, Some(2));
        assert_eq!(child.parent_version_id.as_deref(), Some(v1.as_str()));
        assert_eq!(child.settings["sourceTriangles"], 800);
        let triangles = child.outputs[0].report.as_ref().unwrap()["triangles"]
            .as_u64()
            .unwrap();
        assert!(triangles < 800 / 3, "reduced to {triangles}");
        assert_eq!(child.settings["resultTriangles"], triangles);

        let too_high = RetopologyRequest {
            generation_id: task.generation_id.clone(),
            target_triangles: 800,
            preserve_borders: true,
        };
        assert!(matches!(
            studio.retopologize(too_high),
            Err(StudioError::InvalidInput(_))
        ));
        let generations = studio
            .asset_detail(&task.asset_id)
            .unwrap()
            .generations
            .len();
        assert_eq!(generations, 2, "failed request records nothing");
    }

    #[test]
    fn invalid_download_fails_as_resumable_conversion_error() {
        let dir = tempfile::tempdir().unwrap();
        let transport = ScriptedTransport::default();
        let (studio, project) = keyed(dir.path(), &transport);
        let task = studio
            .store
            .record_submission(crate::store::tests::submission(&project, "task_abc123"))
            .unwrap();

        transport.push(200, SUCCESS.as_bytes());
        transport.push(200, b"<html>expired link</html>");
        studio.run_poller(&task.id, policy(), NO_SLEEP);
        let failed = studio.store.task_view(&task.id).unwrap();
        assert_eq!(failed.state, TaskState::Failed);
        assert_eq!(failed.error.as_ref().unwrap().kind, "conversion");
        assert!(failed.can_resume);
        assert!(
            failed.version_id.is_none(),
            "no version for an unvalidated output"
        );
    }

    #[test]
    fn offline_polling_backs_off_then_fails_and_can_resume() {
        let dir = tempfile::tempdir().unwrap();
        let transport = ScriptedTransport::default();
        let (studio, project) = keyed(dir.path(), &transport);
        let task = studio
            .store
            .record_submission(crate::store::tests::submission(&project, "task_abc123"))
            .unwrap();

        transport.push_error(TransportError::Unreachable("offline".into()));
        transport.push_error(TransportError::Unreachable("offline".into()));
        let delays = Mutex::new(Vec::new());
        studio.run_poller(&task.id, policy(), &|delay| {
            delays.lock().unwrap().push(delay)
        });
        assert_eq!(
            *delays.lock().unwrap(),
            vec![Duration::from_secs(6)],
            "exponential backoff before giving up"
        );
        let failed = studio.store.task_view(&task.id).unwrap();
        assert_eq!(failed.error.as_ref().unwrap().kind, "provider_unavailable");
        assert!(failed.can_resume);

        studio.resume_task(&task.id).unwrap();
        transport.push(
            200,
            br#"{"code":0,"data":{"task_id":"task_abc123","status":"banned"}}"#,
        );
        studio.run_poller(&task.id, policy(), NO_SLEEP);
        let banned = studio.store.task_view(&task.id).unwrap();
        assert_eq!(banned.state, TaskState::Failed);
        assert_eq!(banned.error.as_ref().unwrap().kind, "task_failed");
        assert!(
            !banned.can_resume,
            "content-policy failures need a new request"
        );
    }

    #[test]
    fn timeout_is_bounded_by_policy() {
        let dir = tempfile::tempdir().unwrap();
        let (studio, project) = keyed(dir.path(), &ScriptedTransport::default());
        let task = studio
            .store
            .record_submission(crate::store::tests::submission(&project, "task_x"))
            .unwrap();
        studio.run_poller(
            &task.id,
            PollPolicy {
                timeout_secs: 0,
                ..PollPolicy::default()
            },
            NO_SLEEP,
        );
        assert_eq!(
            studio
                .store
                .task_view(&task.id)
                .unwrap()
                .error
                .unwrap()
                .kind,
            "task_timeout"
        );
    }

    #[test]
    fn image_inspection_rejects_bad_files() {
        let dir = tempfile::tempdir().unwrap();
        let studio = service(dir.path(), &ScriptedTransport::default());
        let good = dir.path().join("photo.png");
        std::fs::write(&good, png_bytes(300, 300)).unwrap();
        let inspection = studio
            .inspect_image("tripo", good.to_str().unwrap())
            .unwrap();
        assert_eq!(
            (inspection.check.width, inspection.file_name.as_str()),
            (300, "photo.png")
        );
        let bad = dir.path().join("fake.png");
        std::fs::write(&bad, b"not an image").unwrap();
        assert!(studio
            .inspect_image("tripo", bad.to_str().unwrap())
            .is_err());
        assert!(studio
            .inspect_image("tripo", dir.path().to_str().unwrap())
            .is_err());
    }

    #[test]
    fn managed_paths_cannot_escape_root_and_settings_validate() {
        let dir = tempfile::tempdir().unwrap();
        let studio = service(dir.path(), &ScriptedTransport::default());
        assert!(studio.managed_path("../secrets.txt").is_err());
        assert!(studio.managed_path("assets/a/b.glb").is_ok());
        assert!(studio.set_default_model("tripo", "not-a-model").is_err());
        studio.set_default_model("tripo", "v3.0-20250812").unwrap();
        assert_eq!(tripo_info(&studio).default_model, "v3.0-20250812");
        assert!(studio
            .set_poll_policy(PollPolicy {
                interval_secs: 0,
                ..PollPolicy::default()
            })
            .is_err());
    }
}

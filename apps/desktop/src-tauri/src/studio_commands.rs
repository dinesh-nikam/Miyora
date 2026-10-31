//! Thin Tauri adapters over `agentpet_studio::service::StudioService`.
//! Every command runs on a blocking worker so provider HTTP never stalls the UI.
//! Errors cross IPC as a JSON-encoded `ErrorReport` string (what/why/what to do).

use std::path::Path;
use std::sync::Arc;

use agentpet_studio::credentials::KeyringStore;
use agentpet_studio::error::{ErrorReport, StudioError};
use agentpet_studio::export::{ExportOptions, LocalExportFormat};
use agentpet_studio::local::{LocalEngine, LocalEngineConfig};
use agentpet_studio::prompt::{BuiltPrompt, PromptIntent};
use agentpet_studio::provider::AccountBalance;
use agentpet_studio::service::{
    provider_factory, ImageInspection, ImageSubmission, RetopologyRequest, StudioOverview,
    StudioService, TextSubmission, STUDIO_DIR,
};
use agentpet_studio::store::{AssetDetail, AssetSummary, ExportView, ProjectView, TaskView};
use agentpet_studio::task::PollPolicy;
use agentpet_studio::transport::{HttpConfig, ReqwestTransport};
use tauri::State;
use tauri_plugin_dialog::DialogExt;

pub fn open_studio(app_data_dir: &Path) -> Result<StudioService, Box<dyn std::error::Error>> {
    let transport = ReqwestTransport::new(HttpConfig::default())
        .map_err(|error| format!("studio HTTP client unavailable: {error:?}"))?;
    let local_engine = LocalEngine::new(
        LocalEngine::default_engine_root(),
        app_data_dir.join(STUDIO_DIR).join("local-jobs"),
        LocalEngineConfig::default(),
    );
    let studio = StudioService::open(
        app_data_dir,
        Arc::new(KeyringStore),
        provider_factory(
            Arc::new(transport),
            local_engine,
            app_data_dir.join(STUDIO_DIR).join("hf-jobs"),
        ),
    )?;
    // Offline is fine: pollers back off and stop within the configured budget.
    if let Err(error) = studio.resume_active_tasks() {
        eprintln!("Miyora Studio could not resume tasks: {error}");
    }
    Ok(studio)
}

fn report(error: StudioError) -> String {
    serde_json::to_string(&ErrorReport::from(&error)).unwrap_or_else(|_| error.to_string())
}

async fn run<T: Send + 'static>(
    studio: State<'_, StudioService>,
    job: impl FnOnce(StudioService) -> Result<T, StudioError> + Send + 'static,
) -> Result<T, String> {
    let service = studio.inner().clone();
    tauri::async_runtime::spawn_blocking(move || job(service))
        .await
        .map_err(|_| {
            report(StudioError::InvalidInput(
                "Studio worker stopped unexpectedly.".into(),
            ))
        })?
        .map_err(report)
}

#[tauri::command]
pub async fn studio_overview(studio: State<'_, StudioService>) -> Result<StudioOverview, String> {
    run(studio, |service| service.overview()).await
}

#[tauri::command]
pub async fn studio_save_api_key(
    provider: String,
    api_key: String,
    studio: State<'_, StudioService>,
) -> Result<(), String> {
    run(studio, move |service| {
        service.save_api_key(&provider, &api_key)
    })
    .await
}

#[tauri::command]
pub async fn studio_delete_api_key(
    provider: String,
    studio: State<'_, StudioService>,
) -> Result<(), String> {
    run(studio, move |service| service.delete_api_key(&provider)).await
}

#[tauri::command]
pub async fn studio_account_balance(
    provider: String,
    studio: State<'_, StudioService>,
) -> Result<AccountBalance, String> {
    run(studio, move |service| service.account_balance(&provider)).await
}

#[tauri::command]
pub async fn studio_set_default_model(
    provider: String,
    model: String,
    studio: State<'_, StudioService>,
) -> Result<(), String> {
    run(studio, move |service| {
        service.set_default_model(&provider, &model)
    })
    .await
}

#[tauri::command]
pub async fn studio_set_poll_policy(
    policy: PollPolicy,
    studio: State<'_, StudioService>,
) -> Result<PollPolicy, String> {
    run(studio, move |service| service.set_poll_policy(policy)).await
}

#[tauri::command]
pub async fn studio_build_prompt(
    provider: String,
    intent: PromptIntent,
    preset_id: Option<String>,
    studio: State<'_, StudioService>,
) -> Result<BuiltPrompt, String> {
    run(studio, move |service| {
        service.build_prompt(&provider, &intent, preset_id.as_deref())
    })
    .await
}

#[tauri::command]
pub async fn studio_list_projects(
    studio: State<'_, StudioService>,
) -> Result<Vec<ProjectView>, String> {
    run(studio, |service| service.list_projects()).await
}

#[tauri::command]
pub async fn studio_create_project(
    name: String,
    studio: State<'_, StudioService>,
) -> Result<ProjectView, String> {
    run(studio, move |service| service.create_project(&name)).await
}

#[tauri::command]
pub async fn studio_list_assets(
    project_id: String,
    studio: State<'_, StudioService>,
) -> Result<Vec<AssetSummary>, String> {
    run(studio, move |service| service.list_assets(&project_id)).await
}

#[tauri::command]
pub async fn studio_asset_detail(
    asset_id: String,
    studio: State<'_, StudioService>,
) -> Result<AssetDetail, String> {
    run(studio, move |service| service.asset_detail(&asset_id)).await
}

#[tauri::command]
pub async fn studio_list_tasks(studio: State<'_, StudioService>) -> Result<Vec<TaskView>, String> {
    run(studio, |service| service.list_tasks()).await
}

#[tauri::command]
pub async fn studio_inspect_image(
    provider: String,
    path: String,
    studio: State<'_, StudioService>,
) -> Result<ImageInspection, String> {
    run(studio, move |service| {
        service.inspect_image(&provider, &path)
    })
    .await
}

#[tauri::command]
pub async fn studio_submit_text(
    submission: TextSubmission,
    studio: State<'_, StudioService>,
) -> Result<TaskView, String> {
    run(studio, move |service| service.submit_text_to_3d(submission)).await
}

#[tauri::command]
pub async fn studio_submit_image(
    submission: ImageSubmission,
    studio: State<'_, StudioService>,
) -> Result<TaskView, String> {
    run(studio, move |service| {
        service.submit_image_to_3d(submission)
    })
    .await
}

#[tauri::command]
pub async fn studio_regenerate(
    generation_id: String,
    studio: State<'_, StudioService>,
) -> Result<TaskView, String> {
    run(studio, move |service| service.regenerate(&generation_id)).await
}

#[tauri::command]
pub async fn studio_resume_task(
    task_id: String,
    studio: State<'_, StudioService>,
) -> Result<TaskView, String> {
    run(studio, move |service| service.resume_task(&task_id)).await
}

/// Raw bytes (no JSON array encoding) for model files and preview images.
#[tauri::command]
pub async fn studio_read_output(
    output_id: i64,
    studio: State<'_, StudioService>,
) -> Result<tauri::ipc::Response, String> {
    run(studio, move |service| service.read_output(output_id))
        .await
        .map(tauri::ipc::Response::new)
}

/// Asks for a destination with the native Save dialog (opened here, so the
/// webview never supplies a write path), then exports and verifies the file.
/// `None` means the user cancelled.
#[tauri::command]
pub async fn studio_export_output(
    app: tauri::AppHandle,
    output_id: i64,
    format: LocalExportFormat,
    options: ExportOptions,
    studio: State<'_, StudioService>,
) -> Result<Option<ExportView>, String> {
    run(studio, move |service| {
        let plan = service.export_plan(output_id, format)?;
        let picked = app
            .dialog()
            .file()
            .set_file_name(&plan.suggested_file_name)
            .add_filter(plan.extension.to_uppercase(), &[plan.extension.as_str()])
            .blocking_save_file();
        let Some(picked) = picked else {
            return Ok(None);
        };
        let path = picked
            .into_path()
            .map_err(|_| StudioError::InvalidInput("Choose a folder on this computer.".into()))?;
        service
            .export_output(output_id, format, options, &path)
            .map(Some)
    })
    .await
}

/// Polygon reduction on this computer; returns the new version's (completed) task.
#[tauri::command]
pub async fn studio_retopologize(
    request: RetopologyRequest,
    studio: State<'_, StudioService>,
) -> Result<TaskView, String> {
    run(studio, move |service| service.retopologize(request)).await
}

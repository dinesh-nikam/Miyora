use std::{
    fs,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
    time::Duration,
};

use agentpet_avatar::CharacterSpec;
use agentpet_core::{
    AchievementSummary, AgentSessionSummary, AvatarCatalogEntry, AvatarSource, Core,
    CustomAvatarSummary, DesktopPosition, EventType, InstalledPetPackSummary, PetEvent,
    ProgressionSnapshot, ResolvedAvatar, RuntimeSnapshot, XpTransaction,
};
use agentpet_pet_engine::PetState;
use agentpet_pet_pack::PetPackManifest;
use tauri::{Manager, State};

const STATE_TIMER_INTERVAL: Duration = Duration::from_secs(1);
const POSITION_WRITE_DEBOUNCE: Duration = Duration::from_millis(400);
const AVATAR_LIBRARY_DIR: &str = "avatars";
const QUOTA_FRESHNESS_LIMIT_SECS: u64 = 300;
const MAX_COSTS_FILE_BYTES: u64 = 2 * 1024 * 1024;

struct AppRuntime {
    core: Core,
    #[cfg(feature = "dev-simulator")]
    simulated_core: Core,
    #[cfg(feature = "dev-simulator")]
    is_simulated: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pet_library: std::path::PathBuf,
    avatars_root: std::path::PathBuf,
    app_data_dir: std::path::PathBuf,
}

impl AppRuntime {
    fn active_core(&self) -> &Core {
        #[cfg(feature = "dev-simulator")]
        {
            if self.is_simulated.load(std::sync::atomic::Ordering::Relaxed) {
                return &self.simulated_core;
            }
        }
        &self.core
    }
}

#[tauri::command]
fn get_runtime_snapshot(runtime: State<'_, AppRuntime>) -> Result<RuntimeSnapshot, String> {
    #[allow(unused_mut)]
    let mut snapshot = runtime
        .active_core()
        .snapshot()
        .map_err(|error| format!("RUNTIME_READ_FAILED: {error}"))?;
    #[cfg(feature = "dev-simulator")]
    {
        snapshot.is_simulated = runtime
            .is_simulated
            .load(std::sync::atomic::Ordering::Relaxed);
    }
    Ok(snapshot)
}

#[tauri::command]
fn get_progression(
    runtime: State<'_, AppRuntime>,
    window: tauri::WebviewWindow,
) -> Result<ProgressionSnapshot, String> {
    require_main_window(&window)?;
    runtime
        .active_core()
        .progression()
        .map_err(|error| format!("PROGRESSION_READ_FAILED: {error}"))
}

#[tauri::command]
fn get_xp_history(
    runtime: State<'_, AppRuntime>,
    window: tauri::WebviewWindow,
) -> Result<Vec<XpTransaction>, String> {
    require_main_window(&window)?;
    runtime
        .active_core()
        .recent_xp_transactions()
        .map_err(|error| format!("XP_HISTORY_READ_FAILED: {error}"))
}

#[tauri::command]
fn get_achievements(
    runtime: State<'_, AppRuntime>,
    window: tauri::WebviewWindow,
) -> Result<Vec<AchievementSummary>, String> {
    require_main_window(&window)?;
    runtime
        .active_core()
        .achievements()
        .map_err(|error| format!("ACHIEVEMENTS_READ_FAILED: {error}"))
}

#[tauri::command]
fn get_session_history(
    runtime: State<'_, AppRuntime>,
    window: tauri::WebviewWindow,
    limit: usize,
    offset: usize,
) -> Result<Vec<AgentSessionSummary>, String> {
    require_main_window(&window)?;
    runtime
        .active_core()
        .session_history(limit, offset)
        .map_err(|error| format!("SESSION_HISTORY_READ_FAILED: {error}"))
}

fn require_main_window(window: &tauri::WebviewWindow) -> Result<(), String> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err("WINDOW_PERMISSION_DENIED: this action is available only in the main window".into())
    }
}

fn require_overlay_window(window: &tauri::WebviewWindow) -> Result<(), String> {
    if window.label() == "pet-overlay" {
        Ok(())
    } else {
        Err(
            "WINDOW_PERMISSION_DENIED: this action is available only in the companion window"
                .into(),
        )
    }
}

fn require_app_window(window: &tauri::WebviewWindow) -> Result<(), String> {
    if window.label() == "main" || window.label() == "pet-overlay" {
        Ok(())
    } else {
        Err("WINDOW_PERMISSION_DENIED: this action is available only in application windows".into())
    }
}

#[tauri::command]
fn toggle_pet_pause(
    runtime: State<'_, AppRuntime>,
    window: tauri::WebviewWindow,
) -> Result<RuntimeSnapshot, String> {
    require_overlay_window(&window)?;
    let core = runtime.active_core();
    let snapshot = core
        .snapshot()
        .map_err(|error| format!("RUNTIME_READ_FAILED: {error}"))?;
    let event_type = if snapshot.pet_state == PetState::Paused {
        EventType::UserResumed
    } else {
        EventType::UserPaused
    };
    core.accept_event(&PetEvent::new(event_type, "desktop_overlay"))
        .map_err(|error| format!("PET_INTERACTION_FAILED: {error}"))?;
    core.snapshot()
        .map_err(|error| format!("RUNTIME_READ_FAILED: {error}"))
}

#[tauri::command]
fn list_installed_packs(
    runtime: State<'_, AppRuntime>,
    window: tauri::WebviewWindow,
) -> Result<Vec<InstalledPetPackSummary>, String> {
    require_main_window(&window)?;
    runtime
        .core
        .list_installed_packs()
        .map_err(|error| format!("PET_LIBRARY_READ_FAILED: {error}"))
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PetPackCard {
    #[serde(flatten)]
    pack: InstalledPetPackSummary,
    preview_data_url: Option<String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SpriteFrame {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SelectedPetAnimation {
    pet_name: String,
    animation_id: String,
    fps: u16,
    loop_animation: bool,
    frames: Vec<SpriteFrame>,
    image_data_url: String,
}

#[tauri::command]
fn get_selected_pet_animation(
    runtime: State<'_, AppRuntime>,
) -> Result<Option<SelectedPetAnimation>, String> {
    let snapshot = runtime
        .active_core()
        .snapshot()
        .map_err(|error| format!("RUNTIME_READ_FAILED: {error}"))?;
    let Some(selection_id) = snapshot.selected_pet_id else {
        return Ok(None);
    };
    let packs = runtime
        .active_core()
        .list_installed_packs()
        .map_err(|error| format!("PET_LIBRARY_READ_FAILED: {error}"))?;
    let Some(pack) = packs
        .into_iter()
        .find(|pack| pack.selection_id == selection_id)
    else {
        return Ok(None);
    };
    let root = runtime.pet_library.join(&pack.id).join(&pack.version);
    let manifest_bytes = fs::read(root.join("pet.json"))
        .map_err(|error| format!("PET_PACK_READ_FAILED: {error}"))?;
    let manifest = PetPackManifest::parse(&manifest_bytes)
        .map_err(|error| format!("PET_PACK_READ_FAILED: {error}"))?;
    let animation_id = agentpet_pet_engine::select_animation(
        snapshot.pet_state,
        &manifest.state_mappings,
        &manifest.animations,
    );
    let Some(animation_id) = animation_id else {
        return Ok(None);
    };
    let animation = manifest
        .animations
        .get(animation_id)
        .ok_or_else(|| "PET_ANIMATION_MISSING: animation mapping is invalid".to_string())?;
    let relative_asset = manifest
        .assets
        .get(&animation.asset)
        .ok_or_else(|| "PET_ASSET_MISSING: animation asset is missing".to_string())?;
    let root_canonical =
        std::fs::canonicalize(&root).map_err(|error| format!("PET_PACK_READ_FAILED: {error}"))?;
    let asset_path = std::fs::canonicalize(root.join(relative_asset))
        .map_err(|error| format!("PET_ASSET_READ_FAILED: {error}"))?;
    if !asset_path.starts_with(&root_canonical) {
        return Err("PET_ASSET_INVALID: asset path escaped installed pack".into());
    }
    let bytes = fs::read(asset_path).map_err(|error| format!("PET_ASSET_READ_FAILED: {error}"))?;
    let mime = match Path::new(relative_asset)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => return Err("PET_ASSET_INVALID: unsupported animation image format".into()),
    };
    Ok(Some(SelectedPetAnimation {
        pet_name: manifest.name,
        animation_id: animation_id.to_string(),
        fps: animation.fps,
        loop_animation: animation.loop_animation,
        frames: animation
            .frames
            .iter()
            .map(|frame| SpriteFrame {
                x: frame.x,
                y: frame.y,
                width: frame.width,
                height: frame.height,
            })
            .collect(),
        image_data_url: format!(
            "data:{mime};base64,{}",
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, bytes)
        ),
    }))
}

#[tauri::command]
fn list_pet_pack_cards(
    runtime: State<'_, AppRuntime>,
    window: tauri::WebviewWindow,
) -> Result<Vec<PetPackCard>, String> {
    require_app_window(&window)?;
    let packs = runtime
        .core
        .list_installed_packs()
        .map_err(|error| format!("PET_LIBRARY_READ_FAILED: {error}"))?;
    packs
        .into_iter()
        .map(|pack| {
            let preview_path = runtime
                .pet_library
                .join(&pack.id)
                .join(&pack.version)
                .join("generated/preview.png");
            let preview_data_url = match fs::read(preview_path) {
                Ok(bytes) => Some(format!(
                    "data:image/png;base64,{}",
                    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, bytes)
                )),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(error) => return Err(format!("PET_PREVIEW_READ_FAILED: {error}")),
            };
            Ok(PetPackCard {
                pack,
                preview_data_url,
            })
        })
        .collect()
}

#[tauri::command]
fn install_pet_pack(
    source_path: String,
    runtime: State<'_, AppRuntime>,
    window: tauri::WebviewWindow,
) -> Result<InstalledPetPackSummary, String> {
    require_main_window(&window)?;
    if source_path.trim().is_empty() {
        return Err(
            "PET_PACK_SOURCE_REQUIRED: Select an extracted PetPack folder or archive.".into(),
        );
    }
    let path = Path::new(&source_path);
    let manifest = if path.is_file() {
        let signers = runtime
            .core
            .trusted_signers()
            .map_err(|error| format!("PET_PACK_INSTALL_FAILED: {error}"))?;
        runtime
            .core
            .install_archive_pet_pack(path, &runtime.pet_library, &signers, false)
            .map_err(|error| format!("PET_PACK_INSTALL_FAILED: {error}"))?
    } else {
        runtime
            .core
            .install_extracted_pet_pack(path, &runtime.pet_library)
            .map_err(|error| format!("PET_PACK_INSTALL_FAILED: {error}"))?
    };
    Ok(pack_summary(manifest))
}

#[tauri::command]
async fn create_pet_from_image(
    image_path: String,
    name: String,
    runtime: State<'_, AppRuntime>,
    window: tauri::WebviewWindow,
) -> Result<InstalledPetPackSummary, String> {
    require_main_window(&window)?;
    let core = runtime.core.clone();
    let pet_library = runtime.pet_library.clone();
    drop(window);
    let manifest = tauri::async_runtime::spawn_blocking(move || {
        core.create_custom_pet_from_image(Path::new(&image_path), &pet_library, &name)
    })
    .await
    .map_err(|error| format!("PET_IMAGE_WORKER_FAILED: {error}"))?
    .map_err(|error| format!("PET_IMAGE_CREATE_FAILED: {error}"))?;
    Ok(pack_summary(manifest))
}

fn pack_summary(manifest: PetPackManifest) -> InstalledPetPackSummary {
    InstalledPetPackSummary {
        selection_id: format!("{}@{}", manifest.id, manifest.version),
        id: manifest.id,
        version: manifest.version,
        name: manifest.name,
        creator: manifest.creator,
        license: manifest.license,
    }
}

#[tauri::command]
fn select_pet(
    pack_id: Option<String>,
    runtime: State<'_, AppRuntime>,
    window: tauri::WebviewWindow,
) -> Result<(), String> {
    require_app_window(&window)?;
    runtime
        .core
        .set_selected_pet(pack_id.as_deref())
        .map_err(|error| format!("PET_SELECTION_FAILED: {error}"))
}

#[tauri::command]
fn get_avatar(runtime: State<'_, AppRuntime>) -> Result<ResolvedAvatar, String> {
    runtime
        .core
        .resolve_avatar()
        .map_err(|error| format!("AVATAR_READ_FAILED: {error}"))
}

/// Progress data the 3D stage visualizes: level and progress toward the next one.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AvatarHud {
    level: u64,
    total_xp: i64,
    xp_into_level: i64,
    xp_to_next_level: i64,
}

#[tauri::command]
fn get_avatar_hud(runtime: State<'_, AppRuntime>) -> Result<AvatarHud, String> {
    let progression = runtime
        .active_core()
        .progression()
        .map_err(|error| format!("PROGRESSION_READ_FAILED: {error}"))?;
    Ok(AvatarHud {
        level: progression.level,
        total_xp: progression.total_xp,
        xp_into_level: progression.xp_into_level,
        xp_to_next_level: progression.xp_to_next_level,
    })
}

#[tauri::command]
fn get_avatar_model(
    avatar_id: String,
    runtime: State<'_, AppRuntime>,
) -> Result<tauri::ipc::Response, String> {
    runtime
        .core
        .custom_avatar_model(&avatar_id, &runtime.avatars_root)
        .map(tauri::ipc::Response::new)
        .map_err(|error| format!("AVATAR_MODEL_READ_FAILED: {error}"))
}

#[tauri::command]
fn list_avatar_catalog(
    runtime: State<'_, AppRuntime>,
    window: tauri::WebviewWindow,
) -> Result<Vec<AvatarCatalogEntry>, String> {
    require_main_window(&window)?;
    runtime
        .core
        .list_avatar_catalog()
        .map_err(|error| format!("AVATAR_CATALOG_READ_FAILED: {error}"))
}

#[tauri::command]
fn list_custom_avatars(
    runtime: State<'_, AppRuntime>,
    window: tauri::WebviewWindow,
) -> Result<Vec<CustomAvatarSummary>, String> {
    require_main_window(&window)?;
    runtime
        .core
        .list_custom_avatars()
        .map_err(|error| format!("AVATAR_LIBRARY_READ_FAILED: {error}"))
}

#[tauri::command]
fn get_avatar_preview(
    avatar_id: String,
    runtime: State<'_, AppRuntime>,
    window: tauri::WebviewWindow,
) -> Result<Option<String>, String> {
    require_main_window(&window)?;
    let preview = runtime
        .core
        .custom_avatar_preview(&avatar_id, &runtime.avatars_root)
        .map_err(|error| format!("AVATAR_PREVIEW_READ_FAILED: {error}"))?;
    Ok(preview.map(|bytes| {
        format!(
            "data:image/png;base64,{}",
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, bytes)
        )
    }))
}

#[tauri::command]
fn select_avatar(
    source: String,
    avatar_id: String,
    runtime: State<'_, AppRuntime>,
    window: tauri::WebviewWindow,
) -> Result<ResolvedAvatar, String> {
    require_main_window(&window)?;
    let source = AvatarSource::parse(&source)
        .map_err(|error| format!("AVATAR_SELECTION_FAILED: {error}"))?;
    runtime
        .core
        .select_avatar(source, &avatar_id)
        .and_then(|()| runtime.core.resolve_avatar())
        .map_err(|error| format!("AVATAR_SELECTION_FAILED: {error}"))
}

#[tauri::command]
fn save_avatar_customization(
    spec: CharacterSpec,
    runtime: State<'_, AppRuntime>,
    window: tauri::WebviewWindow,
) -> Result<(), String> {
    require_main_window(&window)?;
    runtime
        .core
        .save_avatar_customization(&spec)
        .map_err(|error| format!("AVATAR_SAVE_FAILED: {error}"))
}

#[tauri::command]
fn reset_avatar_customization(
    catalog_id: String,
    runtime: State<'_, AppRuntime>,
    window: tauri::WebviewWindow,
) -> Result<bool, String> {
    require_main_window(&window)?;
    runtime
        .core
        .reset_avatar_customization(&catalog_id)
        .map_err(|error| format!("AVATAR_RESET_FAILED: {error}"))
}

#[tauri::command]
fn create_custom_character(
    spec: CharacterSpec,
    runtime: State<'_, AppRuntime>,
    window: tauri::WebviewWindow,
) -> Result<CustomAvatarSummary, String> {
    require_main_window(&window)?;
    runtime
        .core
        .create_custom_character(&spec)
        .map_err(|error| format!("AVATAR_CREATE_FAILED: {error}"))
}

#[tauri::command]
async fn create_avatar_from_image(
    image_path: String,
    name: String,
    runtime: State<'_, AppRuntime>,
    window: tauri::WebviewWindow,
) -> Result<Vec<CustomAvatarSummary>, String> {
    require_main_window(&window)?;
    let core = runtime.core.clone();
    let avatars_root = runtime.avatars_root.clone();
    drop(window);
    tauri::async_runtime::spawn_blocking(move || {
        core.create_avatars_from_image(Path::new(&image_path), &avatars_root, &name)
    })
    .await
    .map_err(|error| format!("AVATAR_IMAGE_WORKER_FAILED: {error}"))?
    .map_err(|error| format!("AVATAR_IMAGE_IMPORT_FAILED: {error}"))
}

#[tauri::command]
async fn import_vrm_avatar(
    vrm_path: String,
    runtime: State<'_, AppRuntime>,
    window: tauri::WebviewWindow,
) -> Result<CustomAvatarSummary, String> {
    require_main_window(&window)?;
    let core = runtime.core.clone();
    let avatars_root = runtime.avatars_root.clone();
    drop(window);
    tauri::async_runtime::spawn_blocking(move || {
        core.import_vrm_avatar(Path::new(&vrm_path), &avatars_root)
    })
    .await
    .map_err(|error| format!("AVATAR_VRM_WORKER_FAILED: {error}"))?
    .map_err(|error| format!("AVATAR_VRM_IMPORT_FAILED: {error}"))
}

#[tauri::command]
fn delete_custom_avatar(
    avatar_id: String,
    runtime: State<'_, AppRuntime>,
    window: tauri::WebviewWindow,
) -> Result<(), String> {
    require_main_window(&window)?;
    runtime
        .core
        .delete_custom_avatar(&avatar_id, &runtime.avatars_root)
        .map_err(|error| format!("AVATAR_DELETE_FAILED: {error}"))
}

#[tauri::command]
fn show_pet_overlay(app: tauri::AppHandle, window: tauri::WebviewWindow) -> Result<(), String> {
    require_main_window(&window)?;
    let window = app
        .get_webview_window("pet-overlay")
        .ok_or_else(|| "PET_OVERLAY_UNAVAILABLE: companion window is not configured".to_string())?;
    window
        .show()
        .map_err(|error| format!("PET_OVERLAY_SHOW_FAILED: {error}"))
}

#[tauri::command]
fn verify_ledger(
    runtime: State<'_, AppRuntime>,
    window: tauri::WebviewWindow,
) -> Result<agentpet_core::LedgerVerificationResult, String> {
    require_app_window(&window)?;
    runtime
        .core
        .verify_ledger()
        .map_err(|error| format!("LEDGER_VERIFICATION_FAILED: {error}"))
}

#[tauri::command]
fn get_real_events(
    runtime: State<'_, AppRuntime>,
    window: tauri::WebviewWindow,
    limit: Option<usize>,
) -> Result<Vec<agentpet_core::StoredAgentEvent>, String> {
    require_main_window(&window)?;
    runtime
        .active_core()
        .recent_events(limit.unwrap_or(30))
        .map_err(|error| format!("EVENTS_READ_FAILED: {error}"))
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ProviderStatusReport {
    provider_id: String,
    display_name: String,
    detected: bool,
    hook_status: String,
    quota_status: Option<agentpet_adapters::claude_code::ClaudeQuotaReport>,
    quota_provenance: String,
    quota_updated_seconds_ago: Option<u64>,
    cost_usd: Option<f64>,
    cost_provenance: String,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    tokens_provenance: String,
    config_path: Option<String>,
}

#[tauri::command]
fn get_providers_status(
    runtime: State<'_, AppRuntime>,
    window: tauri::WebviewWindow,
) -> Result<Vec<ProviderStatusReport>, String> {
    require_app_window(&window)?;
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from);

    let mut reports = Vec::new();

    // 1. Claude Code
    let claude_dir = home.as_ref().map(|h| h.join(".claude"));
    let claude_detected = claude_dir.as_ref().map(|p| p.is_dir()).unwrap_or(false);
    let settings_path = claude_dir.as_ref().map(|d| d.join("settings.json"));
    let claude_hook_installed = settings_path
        .as_ref()
        .and_then(|p| fs::read_to_string(p).ok())
        .map(|s| s.contains("agentpet hook"))
        .unwrap_or(false);

    let statusline_file = runtime.app_data_dir.join("claude_statusline.json");
    let (quota_report, quota_updated_secs) = if let Ok(bytes) = fs::read(&statusline_file) {
        let meta = fs::metadata(&statusline_file).ok();
        let secs = meta
            .and_then(|m| m.modified().ok())
            .and_then(|m| std::time::SystemTime::now().duration_since(m).ok())
            .map(|d| d.as_secs());
        (
            agentpet_adapters::claude_code::parse_statusline_payload(&bytes).ok(),
            secs,
        )
    } else {
        (None, None)
    };

    let (claude_cost, claude_in_tokens, claude_out_tokens) = claude_dir
        .as_ref()
        .map(|d| d.join("metrics").join("costs.jsonl"))
        .filter(|p| p.is_file())
        .and_then(|p| {
            let meta = fs::metadata(&p).ok()?;
            if meta.len() > MAX_COSTS_FILE_BYTES {
                return None;
            }
            fs::read_to_string(&p).ok()
        })
        .map(|content| {
            let mut cost_sum = 0.0f64;
            let mut in_sum = 0i64;
            let mut out_sum = 0i64;
            for line in content.lines() {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
                    if let Some(c) = val.get("estimated_cost_usd").and_then(|v| v.as_f64()) {
                        cost_sum += c;
                    }
                    if let Some(i) = val.get("input_tokens").and_then(|v| v.as_i64()) {
                        in_sum += i;
                    }
                    if let Some(o) = val.get("output_tokens").and_then(|v| v.as_i64()) {
                        out_sum += o;
                    }
                }
            }
            (Some(cost_sum), Some(in_sum), Some(out_sum))
        })
        .unwrap_or((None, None, None));

    let claude_cost_prov = if claude_cost.is_some() {
        "ESTIMATED".to_string()
    } else {
        "N/A".to_string()
    };
    let claude_tokens_prov = if claude_in_tokens.is_some() {
        "LOCAL".to_string()
    } else {
        "N/A".to_string()
    };
    let claude_quota_prov = if let Some(ref report) = quota_report {
        if quota_updated_secs.map_or(false, |secs| secs > QUOTA_FRESHNESS_LIMIT_SECS) {
            "STALE".to_string()
        } else {
            report.provenance.clone()
        }
    } else {
        "N/A".to_string()
    };

    reports.push(ProviderStatusReport {
        provider_id: "claude_code".into(),
        display_name: "Claude Code".into(),
        detected: claude_detected,
        hook_status: if claude_hook_installed {
            "installed".into()
        } else {
            "not_installed".into()
        },
        quota_status: quota_report,
        quota_provenance: claude_quota_prov,
        quota_updated_seconds_ago: quota_updated_secs,
        cost_usd: claude_cost,
        cost_provenance: claude_cost_prov,
        input_tokens: claude_in_tokens,
        output_tokens: claude_out_tokens,
        tokens_provenance: claude_tokens_prov,
        config_path: settings_path.map(|p| p.to_string_lossy().to_string()),
    });

    // 2. OpenAI Codex CLI
    let codex_hooks_file = agentpet_adapters::codex::default_hooks_path();
    let codex_detected = codex_hooks_file
        .as_ref()
        .and_then(|p| p.parent())
        .is_some_and(|dir| dir.is_dir());
    let codex_hooks_installed = codex_hooks_file
        .as_ref()
        .map(|p| {
            agentpet_adapters::codex::is_installed(p, agentpet_adapters::codex::HOOK_COMMAND)
                .unwrap_or(false)
        })
        .unwrap_or(false);
    let codex_hook_status = if codex_hooks_installed {
        "installed"
    } else if codex_detected {
        "not_installed"
    } else {
        "missing"
    };
    reports.push(ProviderStatusReport {
        provider_id: "codex".into(),
        display_name: "OpenAI Codex CLI".into(),
        detected: codex_detected,
        hook_status: codex_hook_status.into(),
        quota_status: None,
        quota_provenance: "N/A".into(),
        quota_updated_seconds_ago: None,
        cost_usd: None,
        cost_provenance: "N/A".into(),
        input_tokens: None,
        output_tokens: None,
        tokens_provenance: "N/A".into(),
        config_path: codex_hooks_file.map(|p| p.to_string_lossy().to_string()),
    });

    // 3. GitHub Copilot CLI
    let copilot_dir = home.as_ref().map(|h| h.join(".copilot"));
    let copilot_detected = copilot_dir.as_ref().map(|p| p.is_dir()).unwrap_or(false);
    reports.push(ProviderStatusReport {
        provider_id: "copilot".into(),
        display_name: "GitHub Copilot CLI".into(),
        detected: copilot_detected,
        hook_status: if copilot_detected {
            "unsupported".into()
        } else {
            "missing".into()
        },
        quota_status: None,
        quota_provenance: "N/A".into(),
        quota_updated_seconds_ago: None,
        cost_usd: None,
        cost_provenance: "N/A".into(),
        input_tokens: None,
        output_tokens: None,
        tokens_provenance: "N/A".into(),
        config_path: copilot_dir.map(|p| p.to_string_lossy().to_string()),
    });

    // 4. Google Antigravity
    let ag_dir = home.as_ref().map(|h| h.join(".gemini").join("antigravity"));
    let ag_detected = ag_dir.as_ref().map(|p| p.is_dir()).unwrap_or(false);
    reports.push(ProviderStatusReport {
        provider_id: "antigravity".into(),
        display_name: "Google Antigravity".into(),
        detected: ag_detected,
        hook_status: if ag_detected {
            "unsupported".into()
        } else {
            "missing".into()
        },
        quota_status: None,
        quota_provenance: "N/A".into(),
        quota_updated_seconds_ago: None,
        cost_usd: None,
        cost_provenance: "N/A".into(),
        input_tokens: None,
        output_tokens: None,
        tokens_provenance: "N/A".into(),
        config_path: ag_dir.map(|p| p.to_string_lossy().to_string()),
    });

    // 5. Cursor IDE
    let cursor_dir = home.as_ref().map(|h| h.join(".cursor"));
    let cursor_detected = cursor_dir.as_ref().map(|p| p.is_dir()).unwrap_or(false);
    reports.push(ProviderStatusReport {
        provider_id: "cursor".into(),
        display_name: "Cursor IDE".into(),
        detected: cursor_detected,
        hook_status: if cursor_detected {
            "unsupported".into()
        } else {
            "missing".into()
        },
        quota_status: None,
        quota_provenance: "N/A".into(),
        quota_updated_seconds_ago: None,
        cost_usd: None,
        cost_provenance: "N/A".into(),
        input_tokens: None,
        output_tokens: None,
        tokens_provenance: "N/A".into(),
        config_path: cursor_dir.map(|p| p.to_string_lossy().to_string()),
    });

    Ok(reports)
}

#[cfg(debug_assertions)]
#[tauri::command]
fn toggle_demo_mode(
    #[allow(unused_variables)] runtime: State<'_, AppRuntime>,
    window: tauri::WebviewWindow,
) -> Result<bool, String> {
    require_main_window(&window)?;
    #[cfg(feature = "dev-simulator")]
    {
        let current = runtime
            .is_simulated
            .load(std::sync::atomic::Ordering::Relaxed);
        let next = !current;
        runtime
            .is_simulated
            .store(next, std::sync::atomic::Ordering::Relaxed);
        Ok(next)
    }
    #[cfg(not(feature = "dev-simulator"))]
    {
        Err("SIMULATION_DISABLED: Development simulator is disabled in release builds".into())
    }
}

#[cfg(debug_assertions)]
#[tauri::command]
fn simulate_agent_event(
    #[allow(unused_variables)] runtime: State<'_, AppRuntime>,
    window: tauri::WebviewWindow,
    #[allow(unused_variables)] event_type: String,
) -> Result<RuntimeSnapshot, String> {
    require_main_window(&window)?;
    #[cfg(feature = "dev-simulator")]
    {
        let parsed_type = match event_type.as_str() {
            "started" => EventType::AgentStarted,
            "working" => EventType::AgentWorking,
            "thinking" => EventType::AgentThinking,
            "waiting" => EventType::AgentWaiting,
            "needs_input" => EventType::AgentInputRequired,
            "completed" => EventType::AgentCompleted,
            "failed" => EventType::AgentFailed,
            unknown => return Err(format!("UNKNOWN_EVENT_TYPE: {unknown}")),
        };
        let event = PetEvent::new(parsed_type, "demo_simulator");
        runtime
            .is_simulated
            .store(true, std::sync::atomic::Ordering::Relaxed);
        runtime
            .simulated_core
            .accept_event(&event)
            .map_err(|error| format!("SIMULATION_FAILED: {error}"))?;
        let mut snapshot = runtime
            .simulated_core
            .snapshot()
            .map_err(|error| format!("SNAPSHOT_FAILED: {error}"))?;
        snapshot.is_simulated = true;
        Ok(snapshot)
    }
    #[cfg(not(feature = "dev-simulator"))]
    {
        Err("SIMULATION_DISABLED: Development simulator is disabled in release builds".into())
    }
}

fn restore_visible_position(
    window: &tauri::WebviewWindow,
    saved: Option<DesktopPosition>,
) -> Result<(), String> {
    let monitors = window
        .available_monitors()
        .map_err(|error| format!("MONITOR_QUERY_FAILED: {error}"))?;
    let saved_monitor = saved.and_then(|position| {
        monitors
            .iter()
            .find(|monitor| {
                let work = monitor.work_area();
                let right = i64::from(work.position.x) + i64::from(work.size.width);
                let bottom = i64::from(work.position.y) + i64::from(work.size.height);
                i64::from(position.x) >= i64::from(work.position.x)
                    && i64::from(position.x) < right
                    && i64::from(position.y) >= i64::from(work.position.y)
                    && i64::from(position.y) < bottom
            })
            .cloned()
    });
    let primary_monitor = if saved_monitor.is_none() {
        window
            .primary_monitor()
            .map_err(|error| format!("PRIMARY_MONITOR_QUERY_FAILED: {error}"))?
    } else {
        None
    };
    let monitor = saved_monitor
        .or(primary_monitor)
        .or_else(|| monitors.first().cloned());
    let Some(monitor) = monitor else {
        if let Some(position) = saved {
            window
                .set_position(tauri::PhysicalPosition::new(position.x, position.y))
                .map_err(|error| format!("PET_POSITION_RESTORE_FAILED: {error}"))?;
        }
        return Ok(());
    };
    let work = monitor.work_area();
    let size = window
        .outer_size()
        .map_err(|error| format!("PET_WINDOW_SIZE_FAILED: {error}"))?;
    let left = i64::from(work.position.x);
    let top = i64::from(work.position.y);
    let max_x = (left + i64::from(work.size.width) - i64::from(size.width)).max(left);
    let max_y = (top + i64::from(work.size.height) - i64::from(size.height)).max(top);
    let x = saved
        .map_or(max_x - 16, |position| i64::from(position.x))
        .clamp(left, max_x)
        .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
    let y = saved
        .map_or(max_y - 16, |position| i64::from(position.y))
        .clamp(top, max_y)
        .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
    window
        .set_position(tauri::PhysicalPosition::new(x, y))
        .map_err(|error| format!("PET_POSITION_RESTORE_FAILED: {error}"))
}

fn run_position_writer(
    core: Core,
    receiver: Receiver<tauri::PhysicalPosition<i32>>,
) -> std::io::Result<()> {
    std::thread::Builder::new()
        .name("agentpet-position-writer".into())
        .spawn(move || {
            while let Ok(mut latest) = receiver.recv() {
                while let Ok(position) = receiver.recv_timeout(POSITION_WRITE_DEBOUNCE) {
                    latest = position;
                }
                if let Err(error) = core.save_overlay_position(DesktopPosition {
                    x: latest.x,
                    y: latest.y,
                }) {
                    eprintln!("AgentPet could not persist companion position: {error}");
                }
            }
        })?;
    Ok(())
}

fn run_state_timer(
    core: Core,
    #[cfg(feature = "dev-simulator")] simulated_core: Core,
    #[cfg(feature = "dev-simulator")] is_simulated: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> std::io::Result<()> {
    std::thread::Builder::new()
        .name("agentpet-state-timer".into())
        .spawn(move || loop {
            #[cfg(feature = "dev-simulator")]
            let target = if is_simulated.load(std::sync::atomic::Ordering::Relaxed) {
                &simulated_core
            } else {
                &core
            };
            #[cfg(not(feature = "dev-simulator"))]
            let target = &core;

            if let Err(error) = target.advance_state_timers() {
                eprintln!("AgentPet could not advance timed pet state: {error}");
            }
            std::thread::sleep(STATE_TIMER_INTERVAL);
        })?;
    Ok(())
}

fn seed_starter_packs(core: &Core, pet_library: &Path) {
    let Ok(installed) = core.list_installed_packs() else {
        return;
    };
    let search_roots = [
        PathBuf::from("pet-packs"),
        PathBuf::from("../../pet-packs"),
        PathBuf::from("../../../pet-packs"),
    ];

    for candidate_root in &search_roots {
        if candidate_root.is_dir() {
            let Ok(entries) = fs::read_dir(candidate_root) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && path.join("pet.json").is_file() {
                    if let Ok(pre_manifest) = PetPackManifest::load_and_validate(
                        &path,
                        agentpet_pet_pack::ValidationLimits::default(),
                    ) {
                        if installed
                            .iter()
                            .any(|p| p.id == pre_manifest.id && p.version == pre_manifest.version)
                        {
                            continue;
                        }
                    }
                    let _ = core.install_extracted_pet_pack(&path, pet_library);
                }
            }
            break;
        }
    }

    if let Ok(snapshot) = core.snapshot() {
        if snapshot.selected_pet_id.is_none()
            || snapshot.selected_pet_id.as_deref() == Some("agentpet.starter-aoi@1.0.0")
        {
            if let Ok(packs) = core.list_installed_packs() {
                if packs.iter().any(|p| p.id == "agentpet.starter-aoi") {
                    let _ = core.set_selected_pet(Some("agentpet.starter-aoi@1.0.0"));
                } else if let Some(first) = packs.first() {
                    let _ = core.set_selected_pet(Some(&format!("{}@{}", first.id, first.version)));
                }
            }
        }
    }
}

pub fn run() {
    let (position_sender, position_receiver) = mpsc::channel();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let core = Core::open(&data_dir)?;
            #[cfg(feature = "dev-simulator")]
            let simulated_core = Core::open_simulated(&data_dir)?;
            #[cfg(feature = "dev-simulator")]
            let is_simulated = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let pet_library = data_dir.join("pet-library");
            let avatars_root = data_dir.join(AVATAR_LIBRARY_DIR);
            if let Err(error) = core.sweep_avatar_staging(&avatars_root) {
                eprintln!("AgentPet could not clean interrupted avatar imports: {error}");
            }
            seed_starter_packs(&core, &pet_library);
            let overlay_window = app
                .get_webview_window("pet-overlay")
                .ok_or("pet-overlay window is missing")?;
            restore_visible_position(&overlay_window, core.overlay_position()?)
                .map_err(std::io::Error::other)?;
            run_position_writer(core.clone(), position_receiver).map_err(std::io::Error::other)?;
            run_state_timer(
                core.clone(),
                #[cfg(feature = "dev-simulator")]
                simulated_core.clone(),
                #[cfg(feature = "dev-simulator")]
                is_simulated.clone(),
            )?;
            app.manage(AppRuntime {
                core,
                #[cfg(feature = "dev-simulator")]
                simulated_core,
                #[cfg(feature = "dev-simulator")]
                is_simulated,
                pet_library,
                avatars_root,
                app_data_dir: data_dir,
            });
            Ok(())
        })
        .on_window_event(move |window, event| {
            if window.label() == "pet-overlay" {
                if let tauri::WindowEvent::Moved(position) = event {
                    if position_sender.send(*position).is_err() {
                        eprintln!("AgentPet position writer is unavailable");
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_runtime_snapshot,
            get_progression,
            get_xp_history,
            get_achievements,
            get_session_history,
            list_installed_packs,
            list_pet_pack_cards,
            get_selected_pet_animation,
            install_pet_pack,
            create_pet_from_image,
            select_pet,
            show_pet_overlay,
            toggle_pet_pause,
            verify_ledger,
            get_real_events,
            get_providers_status,
            #[cfg(debug_assertions)]
            toggle_demo_mode,
            #[cfg(debug_assertions)]
            simulate_agent_event,
            get_avatar,
            get_avatar_hud,
            get_avatar_model,
            get_avatar_preview,
            list_avatar_catalog,
            list_custom_avatars,
            select_avatar,
            save_avatar_customization,
            reset_avatar_customization,
            create_custom_character,
            create_avatar_from_image,
            import_vrm_avatar,
            delete_custom_avatar
        ])
        .run(tauri::generate_context!())
        .expect("failed to run AgentPet desktop application");
}

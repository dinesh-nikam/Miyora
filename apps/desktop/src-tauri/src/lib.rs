use std::{
    fs,
    path::Path,
    sync::mpsc::{self, Receiver},
    time::Duration,
};

use agentpet_core::{
    AchievementSummary, AgentSessionSummary, Core, DesktopPosition, EventType,
    InstalledPetPackSummary, PetEvent, ProgressionSnapshot, RuntimeSnapshot, XpTransaction,
};
use agentpet_pet_engine::PetState;
use agentpet_pet_pack::PetPackManifest;
use tauri::{Manager, State};

const STATE_TIMER_INTERVAL: Duration = Duration::from_secs(1);
const POSITION_WRITE_DEBOUNCE: Duration = Duration::from_millis(400);

struct AppRuntime {
    core: Core,
    pet_library: std::path::PathBuf,
}

#[tauri::command]
fn get_runtime_snapshot(runtime: State<'_, AppRuntime>) -> Result<RuntimeSnapshot, String> {
    runtime
        .core
        .snapshot()
        .map_err(|error| format!("RUNTIME_READ_FAILED: {error}"))
}

#[tauri::command]
fn get_progression(
    runtime: State<'_, AppRuntime>,
    window: tauri::WebviewWindow,
) -> Result<ProgressionSnapshot, String> {
    require_main_window(&window)?;
    runtime
        .core
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
        .core
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
        .core
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
        .core
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

#[tauri::command]
fn toggle_pet_pause(
    runtime: State<'_, AppRuntime>,
    window: tauri::WebviewWindow,
) -> Result<RuntimeSnapshot, String> {
    require_overlay_window(&window)?;
    let snapshot = runtime
        .core
        .snapshot()
        .map_err(|error| format!("RUNTIME_READ_FAILED: {error}"))?;
    let event_type = if snapshot.pet_state == PetState::Paused {
        EventType::UserResumed
    } else {
        EventType::UserPaused
    };
    runtime
        .core
        .accept_event(&PetEvent::new(event_type, "desktop_overlay"))
        .map_err(|error| format!("PET_INTERACTION_FAILED: {error}"))?;
    runtime
        .core
        .snapshot()
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
        .core
        .snapshot()
        .map_err(|error| format!("RUNTIME_READ_FAILED: {error}"))?;
    let Some(selection_id) = snapshot.selected_pet_id else {
        return Ok(None);
    };
    let packs = runtime
        .core
        .list_installed_packs()
        .map_err(|error| format!("PET_LIBRARY_READ_FAILED: {error}"))?;
    let Some(pack) = packs
        .into_iter()
        .find(|pack| pack.selection_id == selection_id)
    else {
        return Ok(None);
    };
    let root = runtime.pet_library.join(&pack.id).join(&pack.version);
    let manifest =
        PetPackManifest::load_and_validate(&root, agentpet_pet_pack::ValidationLimits::default())
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
    require_main_window(&window)?;
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
        return Err("PET_PACK_SOURCE_REQUIRED: Select an extracted PetPack folder.".into());
    }
    let manifest = runtime
        .core
        .install_extracted_pet_pack(Path::new(&source_path), &runtime.pet_library)
        .map_err(|error| format!("PET_PACK_INSTALL_FAILED: {error}"))?;
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
    require_main_window(&window)?;
    runtime
        .core
        .set_selected_pet(pack_id.as_deref())
        .map_err(|error| format!("PET_SELECTION_FAILED: {error}"))
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

fn run_state_timer(core: Core) -> std::io::Result<()> {
    std::thread::Builder::new()
        .name("agentpet-state-timer".into())
        .spawn(move || loop {
            if let Err(error) = core.advance_state_timers() {
                eprintln!("AgentPet could not advance timed pet state: {error}");
            }
            std::thread::sleep(STATE_TIMER_INTERVAL);
        })?;
    Ok(())
}

pub fn run() {
    let (position_sender, position_receiver) = mpsc::channel();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let core = Core::open(&data_dir)?;
            let overlay_window = app
                .get_webview_window("pet-overlay")
                .ok_or("pet-overlay window is missing")?;
            restore_visible_position(&overlay_window, core.overlay_position()?)
                .map_err(std::io::Error::other)?;
            run_position_writer(core.clone(), position_receiver).map_err(std::io::Error::other)?;
            run_state_timer(core.clone())?;
            app.manage(AppRuntime {
                core,
                pet_library: data_dir.join("pet-library"),
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
            toggle_pet_pause
        ])
        .run(tauri::generate_context!())
        .expect("failed to run AgentPet desktop application");
}

//! Tray menu, global shortcuts, click-through, and reposition mode for the pet
//! overlay. The overlay has no visible controls, so every action here must stay
//! reachable from the tray, a shortcut, or the main window.
//! Policy (bindings, timeouts) lives in `agentpet_core::overlay_control`.

use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

use agentpet_core::{
    normalize_accelerator, Core, EventType, PetEvent, RepositionSession, RuntimeSnapshot,
    ShortcutAction, ShortcutBindings,
};
use agentpet_pet_engine::PetState;
use serde::Serialize;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, State, WebviewWindow,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState};

use crate::{require_main_window, require_overlay_window, AppRuntime};

pub const OVERLAY_LABEL: &str = "pet-overlay";
const MAIN_LABEL: &str = "main";
const OVERLAY_MODE_EVENT: &str = "overlay-mode-changed";
const OPEN_COMMAND_DECK_EVENT: &str = "open-command-deck";
const SHORTCUT_STATUS_EVENT: &str = "shortcut-status-changed";
const REPOSITION_WATCH_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "state")]
pub enum ShortcutStatus {
    Registered {
        accelerator: String,
    },
    Unbound,
    /// Another application owns the shortcut. The action stays unbound: the
    /// app never picks a different shortcut on the user's behalf.
    Conflict {
        accelerator: String,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutStatusEntry {
    action: ShortcutAction,
    status: ShortcutStatus,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
struct OverlayMode {
    repositioning: bool,
}

#[derive(Default)]
pub struct OverlayControl {
    reposition: Mutex<Option<RepositionSession>>,
    registered: Mutex<HashMap<ShortcutAction, Shortcut>>,
    statuses: Mutex<HashMap<ShortcutAction, ShortcutStatus>>,
}

impl OverlayControl {
    pub(crate) fn is_repositioning(&self) -> bool {
        self.reposition
            .lock()
            .map(|session| session.is_some())
            .unwrap_or(true)
    }
}

pub(crate) fn overlay(app: &AppHandle) -> Result<WebviewWindow, String> {
    app.get_webview_window(OVERLAY_LABEL)
        .ok_or_else(|| "PET_OVERLAY_UNAVAILABLE: companion window is not configured".to_string())
}

fn lock<T>(mutex: &Mutex<T>) -> Result<std::sync::MutexGuard<'_, T>, String> {
    mutex
        .lock()
        .map_err(|_| "OVERLAY_CONTROL_LOCK_POISONED".to_string())
}

pub fn dispatch(app: &AppHandle, action: ShortcutAction) -> Result<(), String> {
    match action {
        ShortcutAction::ShowHide => toggle_overlay_visibility(app),
        ShortcutAction::Pause => toggle_pause(app.state::<AppRuntime>().active_core()).map(|_| ()),
        ShortcutAction::Reposition => toggle_reposition(app),
    }
}

pub fn toggle_overlay_visibility(app: &AppHandle) -> Result<(), String> {
    let window = overlay(app)?;
    let visible = window
        .is_visible()
        .map_err(|error| format!("PET_OVERLAY_QUERY_FAILED: {error}"))?;
    if visible {
        exit_reposition(app)?;
        window
            .hide()
            .map_err(|error| format!("PET_OVERLAY_HIDE_FAILED: {error}"))
    } else {
        window
            .show()
            .map_err(|error| format!("PET_OVERLAY_SHOW_FAILED: {error}"))
    }
}

fn toggle_pause(core: &Core) -> Result<RuntimeSnapshot, String> {
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

pub fn open_command_deck(app: &AppHandle) -> Result<(), String> {
    let main = app
        .get_webview_window(MAIN_LABEL)
        .ok_or_else(|| "MAIN_WINDOW_UNAVAILABLE: main window is not configured".to_string())?;
    let shown = main
        .show()
        .and_then(|()| main.unminimize())
        .and_then(|()| main.set_focus());
    shown.map_err(|error| format!("MAIN_WINDOW_SHOW_FAILED: {error}"))?;
    app.emit_to(MAIN_LABEL, OPEN_COMMAND_DECK_EVENT, ())
        .map_err(|error| format!("COMMAND_DECK_OPEN_FAILED: {error}"))
}

fn set_repositioning(app: &AppHandle, repositioning: bool) -> Result<(), String> {
    let window = overlay(app)?;
    // Normal mode is click-through so the pet never blocks the apps beneath it.
    window
        .set_ignore_cursor_events(!repositioning)
        .map_err(|error| format!("PET_OVERLAY_POINTER_MODE_FAILED: {error}"))?;
    if repositioning {
        let focused = window.show().and_then(|()| window.set_focus());
        focused.map_err(|error| format!("PET_OVERLAY_SHOW_FAILED: {error}"))?;
    }
    if let Err(error) = app.emit(OVERLAY_MODE_EVENT, OverlayMode { repositioning }) {
        eprintln!("AgentPet could not broadcast overlay mode: {error}");
    }
    Ok(())
}

pub fn enter_click_through(app: &AppHandle) -> Result<(), String> {
    set_repositioning(app, false)
}

fn toggle_reposition(app: &AppHandle) -> Result<(), String> {
    let control = app.state::<OverlayControl>();
    let active = lock(&control.reposition)?.is_some();
    if active {
        exit_reposition(app)
    } else {
        *lock(&control.reposition)? = Some(RepositionSession::start(Instant::now()));
        set_repositioning(app, true)
    }
}

pub fn exit_reposition(app: &AppHandle) -> Result<(), String> {
    let control = app.state::<OverlayControl>();
    let was_active = lock(&control.reposition)?.take().is_some();
    if was_active {
        set_repositioning(app, false)?;
    }
    Ok(())
}

pub fn record_activity(app: &AppHandle) {
    let control = app.state::<OverlayControl>();
    let Ok(mut session) = control.reposition.lock() else {
        return;
    };
    if let Some(session) = session.as_mut() {
        session.record_activity(Instant::now());
    }
}

pub fn spawn_reposition_watcher(app: AppHandle) -> std::io::Result<()> {
    std::thread::Builder::new()
        .name("agentpet-reposition-watcher".into())
        .spawn(move || loop {
            std::thread::sleep(REPOSITION_WATCH_INTERVAL);
            let control = app.state::<OverlayControl>();
            let expired = control
                .reposition
                .lock()
                .map(|session| session.is_some_and(|s| s.is_expired(Instant::now())))
                .unwrap_or(false);
            if expired {
                if let Err(error) = exit_reposition(&app) {
                    eprintln!("AgentPet could not end reposition mode: {error}");
                }
            }
        })?;
    Ok(())
}

pub fn handle_shortcut(app: &AppHandle, shortcut: &Shortcut, event: ShortcutEvent) {
    if event.state() != ShortcutState::Pressed {
        return;
    }
    let control = app.state::<OverlayControl>();
    let action = control.registered.lock().ok().and_then(|registered| {
        registered
            .iter()
            .find(|(_, bound)| *bound == shortcut)
            .map(|(action, _)| *action)
    });
    if let Some(action) = action {
        if let Err(error) = dispatch(app, action) {
            eprintln!("AgentPet shortcut action failed: {error}");
        }
    }
}

fn register_os_shortcut(app: &AppHandle, accelerator: &str) -> Result<Shortcut, String> {
    let shortcut: Shortcut = accelerator
        .parse()
        .map_err(|error| format!("{accelerator} is not a supported shortcut ({error})"))?;
    app.global_shortcut().register(shortcut).map_err(|error| {
        format!("{accelerator} is already in use by another application ({error})")
    })?;
    Ok(shortcut)
}

fn status_entries(control: &OverlayControl) -> Result<Vec<ShortcutStatusEntry>, String> {
    let statuses = lock(&control.statuses)?;
    Ok(ShortcutAction::ALL
        .into_iter()
        .map(|action| ShortcutStatusEntry {
            action,
            status: statuses
                .get(&action)
                .cloned()
                .unwrap_or(ShortcutStatus::Unbound),
        })
        .collect())
}

fn broadcast_statuses(app: &AppHandle, entries: &[ShortcutStatusEntry]) {
    if let Err(error) = app.emit(SHORTCUT_STATUS_EVENT, entries) {
        eprintln!("AgentPet could not broadcast shortcut status: {error}");
    }
}

/// Registers saved bindings at startup. A binding another app already owns is
/// reported as a conflict and left unbound; the tray still offers the action.
pub fn register_saved_shortcuts(app: &AppHandle, bindings: &ShortcutBindings) {
    let control = app.state::<OverlayControl>();
    let (Ok(mut registered), Ok(mut statuses)) =
        (control.registered.lock(), control.statuses.lock())
    else {
        return;
    };
    for action in ShortcutAction::ALL {
        let status = match bindings.get(action) {
            None => ShortcutStatus::Unbound,
            Some(accelerator) => match register_os_shortcut(app, accelerator) {
                Ok(shortcut) => {
                    registered.insert(action, shortcut);
                    ShortcutStatus::Registered {
                        accelerator: accelerator.to_owned(),
                    }
                }
                Err(message) => ShortcutStatus::Conflict {
                    accelerator: accelerator.to_owned(),
                    message,
                },
            },
        };
        statuses.insert(action, status);
    }
}

pub fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let menu = Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, "toggle-pet", "Show/Hide Pet", true, None::<&str>)?,
            &MenuItem::with_id(app, "open-deck", "Open Command Deck", true, None::<&str>)?,
            &MenuItem::with_id(app, "pause", "Pause/Resume", true, None::<&str>)?,
            &MenuItem::with_id(app, "reposition", "Reposition Mode", true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "quit", "Quit Miyora", true, None::<&str>)?,
        ],
    )?;
    let mut tray = TrayIconBuilder::with_id("miyora")
        .tooltip("Miyora")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| {
            let result = match event.id().as_ref() {
                "toggle-pet" => dispatch(app, ShortcutAction::ShowHide),
                "open-deck" => open_command_deck(app),
                "pause" => dispatch(app, ShortcutAction::Pause),
                "reposition" => dispatch(app, ShortcutAction::Reposition),
                "quit" => {
                    app.exit(0);
                    Ok(())
                }
                _ => Ok(()),
            };
            if let Err(error) = result {
                eprintln!("AgentPet tray action failed: {error}");
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

#[tauri::command]
pub fn get_companion_shortcuts(
    control: State<'_, OverlayControl>,
    window: WebviewWindow,
) -> Result<Vec<ShortcutStatusEntry>, String> {
    require_main_window(&window)?;
    status_entries(&control)
}

/// Rebinds one action. The new shortcut is registered with the OS before the
/// old one is released, so a conflict leaves the previous binding working.
#[tauri::command]
pub fn set_companion_shortcut(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    control: State<'_, OverlayControl>,
    window: WebviewWindow,
    action: ShortcutAction,
    accelerator: Option<String>,
) -> Result<Vec<ShortcutStatusEntry>, String> {
    require_main_window(&window)?;
    let normalized = accelerator
        .as_deref()
        .map(normalize_accelerator)
        .transpose()
        .map_err(|error| format!("SHORTCUT_INVALID: {error}"))?;
    let previous = lock(&control.registered)?.get(&action).copied();
    let replacement = match normalized.as_deref() {
        Some(value) => {
            let parsed: Shortcut = value
                .parse()
                .map_err(|error| format!("SHORTCUT_INVALID: {value} ({error})"))?;
            if Some(parsed) == previous {
                Some(parsed)
            } else {
                Some(
                    register_os_shortcut(&app, value)
                        .map_err(|message| format!("SHORTCUT_CONFLICT: {message}"))?,
                )
            }
        }
        None => None,
    };
    if let Err(error) = runtime
        .core
        .save_shortcut_binding(action, normalized.as_deref())
    {
        if let Some(shortcut) = replacement.filter(|shortcut| Some(*shortcut) != previous) {
            let _ = app.global_shortcut().unregister(shortcut);
        }
        return Err(format!("SHORTCUT_SAVE_FAILED: {error}"));
    }
    if let Some(old) = previous.filter(|old| Some(*old) != replacement) {
        if let Err(error) = app.global_shortcut().unregister(old) {
            eprintln!("AgentPet could not release the previous shortcut: {error}");
        }
    }
    {
        let mut registered = lock(&control.registered)?;
        let mut statuses = lock(&control.statuses)?;
        match (replacement, normalized) {
            (Some(shortcut), Some(value)) => {
                registered.insert(action, shortcut);
                statuses.insert(action, ShortcutStatus::Registered { accelerator: value });
            }
            _ => {
                registered.remove(&action);
                statuses.insert(action, ShortcutStatus::Unbound);
            }
        }
    }
    let entries = status_entries(&control)?;
    broadcast_statuses(&app, &entries);
    Ok(entries)
}

#[tauri::command]
pub fn run_companion_action(
    app: AppHandle,
    window: WebviewWindow,
    action: ShortcutAction,
) -> Result<(), String> {
    require_main_window(&window)?;
    dispatch(&app, action)
}

#[tauri::command]
pub fn report_overlay_activity(app: AppHandle, window: WebviewWindow) -> Result<(), String> {
    require_overlay_window(&window)?;
    record_activity(&app);
    Ok(())
}

#[tauri::command]
pub fn exit_overlay_reposition(app: AppHandle, window: WebviewWindow) -> Result<(), String> {
    require_overlay_window(&window)?;
    exit_reposition(&app)
}

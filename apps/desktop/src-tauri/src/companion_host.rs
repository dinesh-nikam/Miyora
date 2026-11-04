//! Desktop host loops for the pet: deterministic roaming inside the current
//! monitor's work area, and the once-a-day companion moment. The policies
//! live in `agentpet_core` (`Roamer`, `Core::deliver_daily_moment`); this
//! module only feeds them window geometry and applies their results.

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::{Duration, Instant},
};

use agentpet_core::{DailyMoment, LogicalRect, RoamInput, Roamer, ROAM_TICK};
use agentpet_pet_engine::{AnimationClip, PetPresentation, PetState};
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};

use crate::{
    overlay_controls::{overlay, OverlayControl},
    require_main_window, AppRuntime,
};

const DAILY_MOMENT_EVENT: &str = "daily-moment";
const DAILY_CHECK_INTERVAL: Duration = Duration::from_secs(30);
const DAILY_HISTORY_LIMIT: usize = 30;
/// How long the pet plays its celebrate clip after a new daily moment.
const EMOTE_DURATION: Duration = Duration::from_secs(4);

#[derive(Default)]
pub struct CompanionHost {
    walking: AtomicBool,
    emote_until: Mutex<Option<Instant>>,
}

impl CompanionHost {
    /// Rust-resolved presentation: the state clip, roaming locomotion, and a
    /// short celebrate emote right after a daily moment while idle.
    pub fn presentation(&self, state: PetState) -> PetPresentation {
        let mut presentation =
            PetPresentation::procedural(state, self.walking.load(Ordering::Relaxed));
        let emoting = self
            .emote_until
            .lock()
            .map(|until| until.is_some_and(|deadline| Instant::now() < deadline))
            .unwrap_or(false);
        if emoting && state == PetState::Idle {
            presentation.state_clip = AnimationClip::Celebrate;
        }
        presentation
    }
}

/// Runs `tick` forever, logging each distinct error once instead of every tick.
fn run_loop(
    name: &str,
    interval: Duration,
    app: AppHandle,
    mut tick: impl FnMut(&AppHandle) -> Result<(), String> + Send + 'static,
) -> std::io::Result<()> {
    std::thread::Builder::new()
        .name(name.into())
        .spawn(move || {
            let mut last_error: Option<String> = None;
            loop {
                match tick(&app) {
                    Ok(()) => last_error = None,
                    Err(error) => {
                        if last_error.as_deref() != Some(error.as_str()) {
                            eprintln!("AgentPet companion host: {error}");
                            last_error = Some(error);
                        }
                    }
                }
                std::thread::sleep(interval);
            }
        })?;
    Ok(())
}

pub fn spawn_roaming(app: AppHandle) -> std::io::Result<()> {
    let mut roamer: Option<(String, Roamer)> = None;
    run_loop("agentpet-roaming", ROAM_TICK, app, move |app| {
        roam_tick(app, &mut roamer)
    })
}

fn roam_tick(app: &AppHandle, roamer: &mut Option<(String, Roamer)>) -> Result<(), String> {
    let host = app.state::<CompanionHost>();
    let window = overlay(app)?;
    let visible = window
        .is_visible()
        .map_err(|error| format!("PET_OVERLAY_QUERY_FAILED: {error}"))?;
    // Never move the pet while the user is dragging it or a fullscreen app is in front.
    if !visible
        || app.state::<OverlayControl>().is_repositioning()
        || platform::foreground_is_fullscreen(&window)
    {
        host.walking.store(false, Ordering::Relaxed);
        return Ok(());
    }
    let runtime = app.state::<AppRuntime>();
    let state = runtime
        .active_core()
        .snapshot()
        .map_err(|error| format!("RUNTIME_READ_FAILED: {error}"))?
        .pet_state;
    let reduced_motion = runtime
        .core
        .companion_preferences()
        .map_err(|error| format!("COMPANION_PREFERENCES_READ_FAILED: {error}"))?
        .reduced_motion
        || platform::os_reduced_motion();
    let companion = runtime
        .core
        .avatar_selection()
        .map_err(|error| format!("AVATAR_READ_FAILED: {error}"))?
        .avatar_id;
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let key = format!("{companion}|{today}");
    if roamer.as_ref().is_none_or(|(current, _)| *current != key) {
        *roamer = Some((key, Roamer::new(&companion, &today)));
    }
    let Some((_, roamer)) = roamer.as_mut() else {
        return Ok(());
    };

    let (position, allowed, scale) = geometry(&window)?;
    let output = roamer.step(RoamInput {
        state,
        frozen: reduced_motion,
        position,
        allowed,
    });
    host.walking.store(output.walking, Ordering::Relaxed);
    let target = tauri::PhysicalPosition::new(
        (output.position.0 * scale).round() as i32,
        (output.position.1 * scale).round() as i32,
    );
    let current = window
        .outer_position()
        .map_err(|error| format!("PET_POSITION_QUERY_FAILED: {error}"))?;
    if target != current {
        window
            .set_position(target)
            .map_err(|error| format!("PET_POSITION_UPDATE_FAILED: {error}"))?;
    }
    Ok(())
}

/// Window origin and allowed origins in logical DIPs for the monitor the
/// overlay is on (or the primary monitor if it is on none), plus that
/// monitor's scale factor. Recomputed every tick, so DPI changes and
/// monitor removal clamp the pet back inside a work area.
fn geometry(window: &WebviewWindow) -> Result<((f64, f64), LogicalRect, f64), String> {
    let query = |error: tauri::Error| format!("MONITOR_QUERY_FAILED: {error}");
    let monitor = match window.current_monitor().map_err(query)? {
        Some(monitor) => monitor,
        None => window
            .primary_monitor()
            .map_err(query)?
            .ok_or("NO_MONITOR: no display is available")?,
    };
    let scale = monitor.scale_factor();
    let logical = |value: f64| value / scale;
    let work = monitor.work_area();
    let size = window
        .outer_size()
        .map_err(|error| format!("PET_WINDOW_SIZE_FAILED: {error}"))?;
    let origin = window
        .outer_position()
        .map_err(|error| format!("PET_POSITION_QUERY_FAILED: {error}"))?;
    let work_area = LogicalRect {
        x: logical(f64::from(work.position.x)),
        y: logical(f64::from(work.position.y)),
        width: logical(f64::from(work.size.width)),
        height: logical(f64::from(work.size.height)),
    };
    let allowed = LogicalRect::allowed_origins(
        work_area,
        logical(f64::from(size.width)),
        logical(f64::from(size.height)),
    );
    Ok((
        (logical(f64::from(origin.x)), logical(f64::from(origin.y))),
        allowed,
        scale,
    ))
}

pub fn spawn_daily_moments(app: AppHandle) -> std::io::Result<()> {
    run_loop(
        "agentpet-daily-moments",
        DAILY_CHECK_INTERVAL,
        app,
        daily_tick,
    )
}

fn daily_tick(app: &AppHandle) -> Result<(), String> {
    let runtime = app.state::<AppRuntime>();
    let avatar = runtime
        .core
        .resolve_avatar()
        .map_err(|error| format!("AVATAR_READ_FAILED: {error}"))?;
    let category = avatar.spec.as_ref().and_then(|spec| spec.category);
    let now = chrono::Local::now().fixed_offset();
    let Some(moment) = runtime
        .core
        .deliver_daily_moment(&avatar.avatar_id, category, now)
        .map_err(|error| format!("DAILY_MOMENT_FAILED: {error}"))?
    else {
        return Ok(());
    };
    if let Ok(mut until) = app.state::<CompanionHost>().emote_until.lock() {
        *until = Some(Instant::now() + EMOTE_DURATION);
    }
    app.emit(DAILY_MOMENT_EVENT, &moment)
        .map_err(|error| format!("DAILY_MOMENT_BROADCAST_FAILED: {error}"))
}

#[tauri::command]
pub fn get_daily_moments(
    runtime: State<'_, AppRuntime>,
    window: WebviewWindow,
) -> Result<Vec<DailyMoment>, String> {
    require_main_window(&window)?;
    runtime
        .core
        .daily_moments(DAILY_HISTORY_LIMIT)
        .map_err(|error| format!("DAILY_MOMENTS_READ_FAILED: {error}"))
}

#[cfg(windows)]
mod platform {
    use windows_sys::core::BOOL;
    use windows_sys::Win32::{
        Foundation::{HWND, RECT},
        Graphics::Gdi::{
            GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
        },
        UI::WindowsAndMessaging::{
            GetClassNameW, GetForegroundWindow, GetWindowRect, SystemParametersInfoW,
            SPI_GETCLIENTAREAANIMATION,
        },
    };

    /// True when the foreground window (other than the overlay and the
    /// desktop itself) covers its whole monitor: borderless fullscreen.
    /// Exclusive-fullscreen games are not detected reliably.
    pub fn foreground_is_fullscreen(overlay: &tauri::WebviewWindow) -> bool {
        let own = overlay.hwnd().ok().map(|hwnd| hwnd.0 as HWND);
        // SAFETY: plain Win32 queries on a window handle the OS just returned;
        // every out-parameter is a properly sized, initialized local.
        unsafe {
            let foreground = GetForegroundWindow();
            if foreground.is_null() || Some(foreground) == own {
                return false;
            }
            let mut class = [0u16; 64];
            let length = GetClassNameW(foreground, class.as_mut_ptr(), class.len() as i32);
            let name = String::from_utf16_lossy(&class[..usize::try_from(length).unwrap_or(0)]);
            if name == "Progman" || name == "WorkerW" {
                return false;
            }
            let mut rect: RECT = std::mem::zeroed();
            if GetWindowRect(foreground, &mut rect) == 0 {
                return false;
            }
            let monitor = MonitorFromWindow(foreground, MONITOR_DEFAULTTONEAREST);
            let mut info: MONITORINFO = std::mem::zeroed();
            info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
            if GetMonitorInfoW(monitor, &mut info) == 0 {
                return false;
            }
            let bounds = info.rcMonitor;
            rect.left <= bounds.left
                && rect.top <= bounds.top
                && rect.right >= bounds.right
                && rect.bottom >= bounds.bottom
        }
    }

    /// Windows "Show animations" turned off.
    pub fn os_reduced_motion() -> bool {
        let mut animations: BOOL = 1;
        // SAFETY: SPI_GETCLIENTAREAANIMATION writes one BOOL to the pointer.
        let ok = unsafe {
            SystemParametersInfoW(
                SPI_GETCLIENTAREAANIMATION,
                0,
                (&mut animations as *mut BOOL).cast(),
                0,
            )
        };
        ok != 0 && animations == 0
    }
}

#[cfg(not(windows))]
mod platform {
    pub fn foreground_is_fullscreen(_overlay: &tauri::WebviewWindow) -> bool {
        false
    }

    pub fn os_reduced_motion() -> bool {
        false
    }
}

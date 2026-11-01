//! Desktop overlay control policy: persisted global-shortcut bindings and the
//! reposition-mode timeout. The desktop host performs OS registration and
//! window changes; the rules it follows live here so they are testable.

use std::time::{Duration, Instant};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::{Core, CoreError};

/// Reposition mode ends after this long without pointer or key activity.
pub const REPOSITION_IDLE_LIMIT: Duration = Duration::from_secs(30);
/// Reposition mode never lasts longer than this, even with activity.
pub const REPOSITION_HARD_LIMIT: Duration = Duration::from_secs(120);

const MAX_ACCELERATOR_CHARS: usize = 64;
const MODIFIERS: &[&str] = &["Ctrl", "Alt", "Shift", "Super"];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ShortcutAction {
    ShowHide,
    Pause,
    Reposition,
}

impl ShortcutAction {
    pub const ALL: [Self; 3] = [Self::ShowHide, Self::Pause, Self::Reposition];

    fn column(self) -> &'static str {
        match self {
            Self::ShowHide => "shortcut_show_hide",
            Self::Pause => "shortcut_pause",
            Self::Reposition => "shortcut_reposition",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutBindings {
    pub show_hide: Option<String>,
    pub pause: Option<String>,
    pub reposition: Option<String>,
}

impl ShortcutBindings {
    pub fn get(&self, action: ShortcutAction) -> Option<&str> {
        match action {
            ShortcutAction::ShowHide => self.show_hide.as_deref(),
            ShortcutAction::Pause => self.pause.as_deref(),
            ShortcutAction::Reposition => self.reposition.as_deref(),
        }
    }
}

/// Canonicalizes an accelerator such as `ctrl + alt + p` to `Ctrl+Alt+P`.
/// Requires at least one modifier and exactly one non-modifier key so a bare
/// key can never be captured system-wide.
pub fn normalize_accelerator(input: &str) -> Result<String, CoreError> {
    let invalid = |reason: &str| Err(CoreError::InvalidShortcut(reason.into()));
    if input.chars().count() > MAX_ACCELERATOR_CHARS || !input.is_ascii() {
        return invalid("shortcut must be short ASCII text such as Ctrl+Alt+P");
    }
    let mut modifiers: Vec<&'static str> = Vec::new();
    let mut key: Option<String> = None;
    for part in input.split('+').map(str::trim) {
        if part.is_empty() {
            return invalid("shortcut has an empty part");
        }
        let modifier = MODIFIERS.iter().copied().find(|name| {
            name.eq_ignore_ascii_case(part)
                || (*name == "Ctrl" && part.eq_ignore_ascii_case("Control"))
        });
        match modifier {
            Some(name) if modifiers.contains(&name) => {
                return invalid("shortcut repeats a modifier");
            }
            Some(name) => modifiers.push(name),
            None if key.is_some() => return invalid("shortcut must have exactly one key"),
            None => {
                if !part.chars().all(|c| c.is_ascii_alphanumeric()) {
                    return invalid("shortcut key must be a letter, digit, or named key");
                }
                let (first, rest) = part.split_at(1);
                key = Some(first.to_ascii_uppercase() + &rest.to_ascii_lowercase());
            }
        }
    }
    let Some(key) = key else {
        return invalid("shortcut must end with a key");
    };
    if modifiers.is_empty() {
        return invalid("shortcut needs Ctrl, Alt, Shift, or Super");
    }
    modifiers.sort_by_key(|name| MODIFIERS.iter().position(|m| m == name));
    Ok(format!("{}+{key}", modifiers.join("+")))
}

/// Tracks one reposition-mode session against the idle and hard limits.
#[derive(Debug, Clone, Copy)]
pub struct RepositionSession {
    started_at: Instant,
    last_activity_at: Instant,
}

impl RepositionSession {
    pub fn start(now: Instant) -> Self {
        Self {
            started_at: now,
            last_activity_at: now,
        }
    }

    pub fn record_activity(&mut self, now: Instant) {
        self.last_activity_at = self.last_activity_at.max(now);
    }

    pub fn is_expired(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.last_activity_at) >= REPOSITION_IDLE_LIMIT
            || now.saturating_duration_since(self.started_at) >= REPOSITION_HARD_LIMIT
    }
}

impl Core {
    pub fn shortcut_bindings(&self) -> Result<ShortcutBindings, CoreError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| CoreError::LockPoisoned)?;
        Ok(connection.query_row(
            "SELECT shortcut_show_hide, shortcut_pause, shortcut_reposition
             FROM companion_preferences WHERE singleton = 1",
            [],
            |row| {
                Ok(ShortcutBindings {
                    show_hide: row.get(0)?,
                    pause: row.get(1)?,
                    reposition: row.get(2)?,
                })
            },
        )?)
    }

    /// Stores one binding (`None` unbinds it). The accelerator is normalized
    /// and must not duplicate another action's binding.
    pub fn save_shortcut_binding(
        &self,
        action: ShortcutAction,
        accelerator: Option<&str>,
    ) -> Result<ShortcutBindings, CoreError> {
        let normalized = accelerator.map(normalize_accelerator).transpose()?;
        let current = self.shortcut_bindings()?;
        if let Some(value) = normalized.as_deref() {
            let taken = ShortcutAction::ALL
                .into_iter()
                .any(|other| other != action && current.get(other) == Some(value));
            if taken {
                return Err(CoreError::InvalidShortcut(
                    "shortcut is already used by another companion action".into(),
                ));
            }
        }
        {
            let connection = self
                .connection
                .lock()
                .map_err(|_| CoreError::LockPoisoned)?;
            // Column names come from a fixed enum, never from input.
            connection.execute(
                &format!(
                    "UPDATE companion_preferences SET {} = ?1, updated_at = ?2 WHERE singleton = 1",
                    action.column()
                ),
                rusqlite::params![normalized, Utc::now().to_rfc3339()],
            )?;
        }
        self.shortcut_bindings()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accelerators_normalize_and_reject_unsafe_forms() {
        assert_eq!(
            normalize_accelerator("ctrl + alt + p").unwrap(),
            "Ctrl+Alt+P"
        );
        assert_eq!(
            normalize_accelerator("Alt+Control+space").unwrap(),
            "Ctrl+Alt+Space"
        );
        assert_eq!(
            normalize_accelerator("Shift+Super+F10").unwrap(),
            "Shift+Super+F10"
        );
        for bad in [
            "P",
            "Ctrl+",
            "Ctrl+Ctrl+P",
            "Ctrl+P+Q",
            "Ctrl+Alt",
            "Ctrl+Ä",
            "Ctrl+;",
            "",
        ] {
            assert!(
                normalize_accelerator(bad).is_err(),
                "{bad} should be rejected"
            );
        }
    }

    #[test]
    fn default_bindings_exist_and_updates_are_validated() {
        let core = Core::in_memory().expect("database initializes");
        let defaults = core.shortcut_bindings().expect("bindings");
        assert_eq!(defaults.show_hide.as_deref(), Some("Ctrl+Alt+P"));
        assert_eq!(defaults.pause.as_deref(), Some("Ctrl+Alt+Space"));
        assert_eq!(defaults.reposition.as_deref(), Some("Ctrl+Alt+R"));

        let updated = core
            .save_shortcut_binding(ShortcutAction::Pause, Some("ctrl+shift+9"))
            .expect("rebind");
        assert_eq!(updated.pause.as_deref(), Some("Ctrl+Shift+9"));

        assert!(matches!(
            core.save_shortcut_binding(ShortcutAction::Reposition, Some("Ctrl+Alt+P")),
            Err(CoreError::InvalidShortcut(_))
        ));
        let unbound = core
            .save_shortcut_binding(ShortcutAction::ShowHide, None)
            .expect("unbind");
        assert_eq!(unbound.show_hide, None);
        assert_eq!(unbound.reposition.as_deref(), Some("Ctrl+Alt+R"));
    }

    #[test]
    fn reposition_session_expires_on_idle_and_hard_limits() {
        let start = Instant::now();
        let mut session = RepositionSession::start(start);
        assert!(!session.is_expired(start + Duration::from_secs(29)));
        assert!(session.is_expired(start + REPOSITION_IDLE_LIMIT));

        // Activity every 20 s keeps it alive until the 120 s hard limit.
        let mut session_with_activity = RepositionSession::start(start);
        for seconds in (20..=100).step_by(20) {
            session_with_activity.record_activity(start + Duration::from_secs(seconds));
        }
        assert!(!session_with_activity.is_expired(start + Duration::from_secs(119)));
        assert!(session_with_activity.is_expired(start + REPOSITION_HARD_LIMIT));

        // An out-of-order (older) activity timestamp never shortens the session.
        session.record_activity(start + Duration::from_secs(10));
        session.record_activity(start + Duration::from_secs(5));
        assert!(!session.is_expired(start + Duration::from_secs(39)));
    }
}

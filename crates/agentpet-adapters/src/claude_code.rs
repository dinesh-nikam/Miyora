//! Claude Code native hook adapter.
//!
//! Parses the documented Claude Code hook JSON payload (delivered on stdin
//! to a configured hook command), normalizes it into a [`NormalizedEvent`]
//! with a deterministic idempotency key, and installs/removes the hook
//! command in the user's Claude Code settings with backup, verification,
//! and ownership-scoped removal.

use std::{
    fs,
    path::{Path, PathBuf},
};

use uuid::Uuid;

use crate::{AdapterError, HookEventKind, NormalizedEvent};

pub const ADAPTER_ID: &str = "claude_code";
/// Claude Code hook events this adapter consumes.
pub const HOOK_EVENTS: &[&str] = &[
    "SessionStart",
    "UserPromptSubmit",
    "PreToolUse",
    "PostToolUse",
    "Notification",
    "Stop",
];

/// Fixed namespace for deterministic adapter event UUIDs (v5).
const EVENT_NAMESPACE: Uuid = Uuid::from_bytes([
    0x62, 0x6c, 0x91, 0x27, 0x4f, 0x1c, 0x5a, 0x8d, 0x9e, 0x21, 0x37, 0xa4, 0x5b, 0xd0, 0x11, 0xcb,
]);

/// File name of the Claude Code user settings document.
pub const SETTINGS_FILE: &str = "settings.json";
/// Suffix for the safety backup written beside the settings file.
pub const BACKUP_SUFFIX: &str = ".agentpet.bak";

#[derive(Debug, serde::Deserialize)]
struct ClaudeHookPayload {
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    hook_event_name: Option<String>,
    #[serde(default)]
    tool_name: Option<String>,
    #[serde(default)]
    tool_use_id: Option<String>,
    #[serde(default)]
    cwd: Option<String>,
}

/// Parses a Claude Code hook payload and normalizes it.
///
/// The same logical hook firing always yields the same event id so daemon
/// retries and duplicate deliveries dedupe.
pub fn parse_hook_payload(input: &[u8]) -> Result<NormalizedEvent, AdapterError> {
    if input.len() > crate::MAX_HOOK_PAYLOAD_BYTES {
        return Err(AdapterError::PayloadTooLarge);
    }
    let payload: ClaudeHookPayload = serde_json::from_slice(input)?;
    let hook_event = payload
        .hook_event_name
        .as_deref()
        .ok_or(AdapterError::MissingField("hook_event_name"))?;
    let kind = match hook_event {
        "SessionStart" => HookEventKind::AgentStarted,
        "UserPromptSubmit" | "PreToolUse" | "PostToolUse" => HookEventKind::AgentWorking,
        "Notification" => HookEventKind::AgentInputRequired,
        "Stop" => HookEventKind::AgentCompleted,
        other => return Err(AdapterError::UnsupportedHookEvent(other.to_owned())),
    };
    let session_id = payload.session_id.filter(|id| !id.trim().is_empty());
    if matches!(
        kind,
        HookEventKind::AgentStarted | HookEventKind::AgentCompleted
    ) && session_id.is_none()
    {
        return Err(AdapterError::MissingField("session_id"));
    }
    Ok(NormalizedEvent {
        event_id: deterministic_event_id(
            session_id.as_deref(),
            hook_event,
            payload.tool_use_id.as_deref(),
            payload.tool_name.as_deref(),
        ),
        kind,
        agent_id: ADAPTER_ID.to_owned(),
        session_id,
        project_path: payload.cwd.filter(|path| !path.trim().is_empty()),
    })
}

fn deterministic_event_id(
    session_id: Option<&str>,
    hook_event: &str,
    tool_use_id: Option<&str>,
    tool_name: Option<&str>,
) -> String {
    let seed = format!(
        "{}|{}|{}|{}",
        session_id.unwrap_or(""),
        hook_event,
        tool_use_id.unwrap_or(""),
        tool_name.unwrap_or("")
    );
    Uuid::new_v5(&EVENT_NAMESPACE, seed.as_bytes()).to_string()
}

/// Default user settings path (`~/.claude/settings.json`).
pub fn default_settings_path() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".claude").join(SETTINGS_FILE))
}

/// Report of a hook installation.
#[derive(Debug, Clone)]
pub struct InstallReport {
    pub config_path: PathBuf,
    pub backup_path: Option<PathBuf>,
    pub command: String,
}

/// Installs the hook command for every supported Claude Code hook event.
pub fn install_hook(settings_path: &Path, command: &str) -> Result<InstallReport, AdapterError> {
    if command.trim().is_empty() || command.len() > 4_096 {
        return Err(AdapterError::InvalidConfig(
            "hook command must be non-empty and at most 4096 chars".into(),
        ));
    }
    let previous = read_previous(settings_path)?;
    let backup_path = backup_settings(settings_path, previous.as_deref())?;
    let mut document = parse_settings(previous.as_deref().unwrap_or(b""))?;
    let mut changed = false;
    for event in HOOK_EVENTS {
        changed |= ensure_hook_entry(&mut document, event, command)?;
    }
    if changed {
        write_verified(settings_path, &document, previous.as_deref())?;
    }
    Ok(InstallReport {
        config_path: settings_path.to_path_buf(),
        backup_path,
        command: command.to_owned(),
    })
}

/// Removes only AgentPet-owned hook entries invoking `command`.
pub fn uninstall_hook(settings_path: &Path, command: &str) -> Result<bool, AdapterError> {
    let previous = read_previous(settings_path)?;
    let Some(bytes) = previous else {
        return Ok(false);
    };
    let mut document = parse_settings(&bytes)?;
    let mut removed = false;
    if let Some(hooks) = document
        .get_mut("hooks")
        .and_then(serde_json::Value::as_object_mut)
    {
        for event in HOOK_EVENTS {
            if let Some(entries) = hooks.get_mut(*event).and_then(|v| v.as_array_mut()) {
                let before = entries.len();
                entries.retain(|entry| !entry_invokes(entry, command));
                if entries.len() != before {
                    removed = true;
                }
                if entries.is_empty() {
                    hooks.remove(*event);
                }
            }
        }
        if hooks.is_empty() {
            document.as_object_mut().map(|map| map.remove("hooks"));
        }
    }
    if removed {
        write_verified(settings_path, &document, Some(&bytes))?;
    }
    Ok(removed)
}

/// Returns true when `command` is registered for every supported event.
pub fn is_installed(settings_path: &Path, command: &str) -> Result<bool, AdapterError> {
    let previous = read_previous(settings_path)?;
    let Some(bytes) = previous else {
        return Ok(false);
    };
    let document = parse_settings(&bytes)?;
    Ok(HOOK_EVENTS.iter().all(|event| {
        hook_entries(&document, event)
            .iter()
            .any(|e| entry_invokes(e, command))
    }))
}

fn read_previous(settings_path: &Path) -> Result<Option<Vec<u8>>, AdapterError> {
    match fs::read(settings_path) {
        Ok(bytes) => {
            if bytes.len() > 1024 * 1024 {
                return Err(AdapterError::InvalidConfig(
                    "settings file too large".into(),
                ));
            }
            Ok(Some(bytes))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(AdapterError::ConfigRead(e.to_string())),
    }
}

fn parse_settings(bytes: &[u8]) -> Result<serde_json::Value, AdapterError> {
    if bytes.iter().all(|b| b.is_ascii_whitespace()) {
        return Ok(serde_json::json!({}));
    }
    let v: serde_json::Value = serde_json::from_slice(bytes)?;
    if !v.is_object() {
        return Err(AdapterError::InvalidConfig(
            "settings root must be an object".into(),
        ));
    }
    Ok(v)
}

fn backup_settings(
    settings_path: &Path,
    previous: Option<&[u8]>,
) -> Result<Option<PathBuf>, AdapterError> {
    let Some(previous) = previous else {
        return Ok(None);
    };
    let name = format!(
        "{}{}",
        settings_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy(),
        BACKUP_SUFFIX
    );
    let backup_path = settings_path.with_file_name(name);
    if backup_path.exists() {
        return Ok(Some(backup_path));
    }
    fs::write(&backup_path, previous).map_err(|e| AdapterError::ConfigWrite(e.to_string()))?;
    Ok(Some(backup_path))
}

fn ensure_hook_entry(
    document: &mut serde_json::Value,
    event: &str,
    command: &str,
) -> Result<bool, AdapterError> {
    let hooks = document
        .as_object_mut()
        .ok_or_else(|| AdapterError::InvalidConfig("settings root must be an object".into()))?
        .entry("hooks")
        .or_insert_with(|| serde_json::json!({}));
    let hooks_obj = hooks
        .as_object_mut()
        .ok_or_else(|| AdapterError::InvalidConfig("hooks must be an object".into()))?;
    let entries = hooks_obj
        .entry(event)
        .or_insert_with(|| serde_json::json!([]));
    let entries = entries
        .as_array_mut()
        .ok_or_else(|| AdapterError::InvalidConfig("hook entries must be an array".into()))?;
    if entries.iter().any(|e| entry_invokes(e, command)) {
        return Ok(false);
    }
    entries.push(serde_json::json!({"hooks": [{"type": "command", "command": command}]}));
    Ok(true)
}

fn hook_entries<'a>(document: &'a serde_json::Value, event: &str) -> Vec<&'a serde_json::Value> {
    document
        .get("hooks")
        .and_then(|h| h.get(event))
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().collect())
        .unwrap_or_default()
}

fn entry_invokes(entry: &serde_json::Value, command: &str) -> bool {
    entry
        .get("hooks")
        .and_then(|h| h.as_array())
        .map(|arr| {
            arr.iter()
                .any(|h| h.get("command").and_then(|c| c.as_str()) == Some(command))
        })
        .unwrap_or(false)
}

fn write_verified(
    settings_path: &Path,
    document: &serde_json::Value,
    previous: Option<&[u8]>,
) -> Result<(), AdapterError> {
    if let Some(parent) = settings_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| AdapterError::ConfigWrite(e.to_string()))?;
        }
    }
    let serialized = serde_json::to_vec_pretty(document)
        .map_err(|e| AdapterError::ConfigWrite(e.to_string()))?;
    if let Err(e) = fs::write(settings_path, &serialized) {
        restore_previous(settings_path, previous);
        return Err(AdapterError::ConfigWrite(e.to_string()));
    }
    if parse_settings(&serialized).is_err() {
        restore_previous(settings_path, previous);
        return Err(AdapterError::InstallFailed(
            "written settings did not verify".into(),
        ));
    }
    Ok(())
}

fn restore_previous(settings_path: &Path, previous: Option<&[u8]>) {
    match previous {
        Some(bytes) => {
            let _ = fs::write(settings_path, bytes);
        }
        None => {
            let _ = fs::remove_file(settings_path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAX: usize = crate::MAX_HOOK_PAYLOAD_BYTES;

    #[test]
    fn maps_documented_hook_events_and_is_idempotent() {
        let start = parse_hook_payload(
            br#"{"session_id":"abc","hook_event_name":"SessionStart","cwd":"/tmp/proj"}"#,
        )
        .expect("session start parses");
        assert_eq!(start.kind, HookEventKind::AgentStarted);
        assert_eq!(start.session_id.as_deref(), Some("abc"));
        let stop = parse_hook_payload(br#"{"session_id":"abc","hook_event_name":"Stop"}"#)
            .expect("stop parses");
        assert_eq!(stop.kind, HookEventKind::AgentCompleted);
        let tool = parse_hook_payload(
            br#"{"session_id":"abc","hook_event_name":"PreToolUse","tool_name":"Bash","tool_use_id":"t1"}"#,
        )
        .expect("tool parses");
        assert_eq!(tool.kind, HookEventKind::AgentWorking);
        let replay = parse_hook_payload(
            br#"{"session_id":"abc","hook_event_name":"PreToolUse","tool_name":"Bash","tool_use_id":"t1"}"#,
        )
        .expect("replay parses");
        assert_eq!(tool.event_id, replay.event_id);
        let other = parse_hook_payload(
            br#"{"session_id":"abc","hook_event_name":"PreToolUse","tool_name":"Bash","tool_use_id":"t2"}"#,
        )
        .expect("second tool parses");
        assert_ne!(tool.event_id, other.event_id);
    }

    #[test]
    fn rejects_malformed_and_unsupported_payloads() {
        assert!(parse_hook_payload(b"not json").is_err());
        assert!(parse_hook_payload(br#"{"hook_event_name":"SubagentStop"}"#).is_err());
        assert!(parse_hook_payload(br#"{"hook_event_name":"SessionStart"}"#).is_err());
        assert!(parse_hook_payload(&vec![0_u8; MAX + 1]).is_err());
    }

    #[test]
    fn install_is_reversible_and_preserves_user_entries() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(SETTINGS_FILE);
        fs::write(
            &path,
            br#"{"hooks":{"SessionStart":[{"hooks":[{"type":"command","command":"user-cmd"}]}]}}"#,
        )
        .expect("seed settings");
        install_hook(&path, "agentpet hook").expect("install");
        assert!(is_installed(&path, "agentpet hook").expect("check"));
        let removed = uninstall_hook(&path, "agentpet hook").expect("uninstall");
        assert!(removed);
        assert!(!is_installed(&path, "agentpet hook").expect("recheck"));
        let doc: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).expect("read")).expect("parse");
        assert!(hook_entries(&doc, "SessionStart")
            .iter()
            .any(|e| entry_invokes(e, "user-cmd")));
    }
}

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

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{AdapterError, HookEventKind, NormalizedEvent};

pub const ADAPTER_ID: &str = "claude_code";
/// Claude Code hook events this adapter consumes.
pub const HOOK_EVENTS: &[&str] = &[
    "SessionStart",
    "UserPromptSubmit",
    "PreToolUse",
    "PostToolUse",
    "PostToolUseFailure",
    "Notification",
    "Stop",
    "SubagentStop",
    "PreCompact",
    "PostCompact",
];

/// Notification type Claude Code sends when a tool call awaits approval.
const PERMISSION_PROMPT: &str = "permission_prompt";

/// Lifecycle-tracking events required to verify hook installation.
pub const REQUIRED_HOOK_EVENTS: &[&str] = &[
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
    notification_type: Option<String>,
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
    let notification_type = payload
        .notification_type
        .as_deref()
        .filter(|value| !value.trim().is_empty());
    let kind = match hook_event {
        "SessionStart" => HookEventKind::AgentStarted,
        // A failed tool call is routine; the turn continues. AgentFailed
        // would end the session as failed.
        "UserPromptSubmit" | "PreToolUse" | "PostToolUse" | "PostToolUseFailure"
        | "SubagentStop" | "PostCompact" => HookEventKind::AgentWorking,
        "PreCompact" => HookEventKind::AgentThinking,
        // Only a permission prompt blocks the agent. Idle reminders and other
        // notifications mean it is waiting, not that input is required.
        "Notification" if notification_type == Some(PERMISSION_PROMPT) => {
            HookEventKind::AgentInputRequired
        }
        "Notification" => HookEventKind::AgentWaiting,
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
            notification_type,
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
    notification_type: Option<&str>,
) -> String {
    let mut seed = format!(
        "{}|{}|{}|{}",
        session_id.unwrap_or(""),
        hook_event,
        tool_use_id.unwrap_or(""),
        tool_name.unwrap_or("")
    );
    // Appended only when present so ids of other events stay unchanged.
    if let Some(notification_type) = notification_type {
        seed.push('|');
        seed.push_str(notification_type);
    }
    Uuid::new_v5(&EVENT_NAMESPACE, seed.as_bytes()).to_string()
}

/// Default user settings path (`~/.claude/settings.json`).
pub fn default_settings_path() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
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
    install_hook_events(settings_path, command, HOOK_EVENTS)
}

/// Removes only AgentPet-owned hook entries invoking `command`.
pub fn uninstall_hook(settings_path: &Path, command: &str) -> Result<bool, AdapterError> {
    uninstall_hook_events(settings_path, command, HOOK_EVENTS)
}

/// Returns true when `command` is registered for required lifecycle events.
pub fn is_installed(settings_path: &Path, command: &str) -> Result<bool, AdapterError> {
    is_installed_for_events(settings_path, command, REQUIRED_HOOK_EVENTS)
}

/// Installs `command` for `events` in a hooks document shaped like Claude
/// Code settings (`{"hooks": {"<Event>": [{"hooks": [...]}]}}`).
pub(crate) fn install_hook_events(
    settings_path: &Path,
    command: &str,
    events: &[&str],
) -> Result<InstallReport, AdapterError> {
    if command.trim().is_empty() || command.len() > 4_096 {
        return Err(AdapterError::InvalidConfig(
            "hook command must be non-empty and at most 4096 chars".into(),
        ));
    }
    let previous = read_previous(settings_path)?;
    let backup_path = backup_settings(settings_path, previous.as_deref())?;
    let mut document = parse_settings(previous.as_deref().unwrap_or(b""))?;
    let mut changed = false;
    for event in events {
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

/// Removes only entries invoking `command` from `events`.
pub(crate) fn uninstall_hook_events(
    settings_path: &Path,
    command: &str,
    events: &[&str],
) -> Result<bool, AdapterError> {
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
        for event in events {
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

/// Returns true when `command` is registered for every one of `required`.
pub(crate) fn is_installed_for_events(
    settings_path: &Path,
    command: &str,
    required: &[&str],
) -> Result<bool, AdapterError> {
    let previous = read_previous(settings_path)?;
    let Some(bytes) = previous else {
        return Ok(false);
    };
    let document = parse_settings(&bytes)?;
    Ok(required.iter().all(|event| {
        hook_entries(&document, event)
            .iter()
            .any(|e| entry_invokes(e, command))
    }))
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeQuotaWindow {
    pub used_percentage: f64,
    pub resets_at: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeQuotaReport {
    pub five_hour: Option<ClaudeQuotaWindow>,
    pub seven_day: Option<ClaudeQuotaWindow>,
    pub context_window_used_percentage: Option<f64>,
    pub updated_at: String,
    pub provenance: String,
}

fn parse_resets_at(v: &serde_json::Value) -> Option<String> {
    let val = v.get("resets_at")?;
    if let Some(epoch_sec) = val.as_i64() {
        chrono::DateTime::from_timestamp(epoch_sec, 0).map(|dt| dt.to_rfc3339())
    } else if let Some(epoch_sec_f) = val.as_f64() {
        chrono::DateTime::from_timestamp(epoch_sec_f as i64, 0).map(|dt| dt.to_rfc3339())
    } else if let Some(s) = val.as_str() {
        if let Ok(epoch) = s.parse::<i64>() {
            chrono::DateTime::from_timestamp(epoch, 0).map(|dt| dt.to_rfc3339())
        } else {
            Some(s.to_string())
        }
    } else {
        None
    }
}

pub fn parse_statusline_payload(input: &[u8]) -> Result<ClaudeQuotaReport, AdapterError> {
    let root: serde_json::Value = serde_json::from_slice(input)?;
    let rate_limits = root.get("rate_limits");
    let context_window = root.get("context_window");
    let five_hour = rate_limits.and_then(|r| r.get("five_hour")).and_then(|v| {
        let used = v.get("used_percentage")?.as_f64()?;
        let resets = parse_resets_at(v);
        Some(ClaudeQuotaWindow {
            used_percentage: used,
            resets_at: resets,
        })
    });
    let seven_day = rate_limits.and_then(|r| r.get("seven_day")).and_then(|v| {
        let used = v.get("used_percentage")?.as_f64()?;
        let resets = parse_resets_at(v);
        Some(ClaudeQuotaWindow {
            used_percentage: used,
            resets_at: resets,
        })
    });
    let context_window_used = context_window
        .and_then(|c| c.get("used_percentage"))
        .and_then(|v| v.as_f64());
    let provenance = if five_hour.is_some() || seven_day.is_some() {
        "LIVE".to_string()
    } else {
        "N/A".to_string()
    };
    let updated_at = root
        .get("timestamp")
        .or_else(|| root.get("captured_at"))
        .or_else(|| root.get("updated_at"))
        .and_then(|ts| {
            if let Some(s) = ts.as_str() {
                if let Ok(epoch) = s.parse::<i64>() {
                    chrono::DateTime::from_timestamp(epoch, 0).map(|dt| dt.to_rfc3339())
                } else {
                    Some(s.to_string())
                }
            } else if let Some(sec) = ts.as_i64() {
                chrono::DateTime::from_timestamp(sec, 0).map(|dt| dt.to_rfc3339())
            } else if let Some(sec) = ts.as_f64() {
                chrono::DateTime::from_timestamp(sec as i64, 0).map(|dt| dt.to_rfc3339())
            } else {
                None
            }
        })
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
    Ok(ClaudeQuotaReport {
        five_hour,
        seven_day,
        context_window_used_percentage: context_window_used,
        updated_at,
        provenance,
    })
}

pub const CHAIN_FILE: &str = "agentpet_chain.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatuslineChainRecord {
    pub original_command: Option<String>,
    pub installed_at: String,
}

pub fn default_chain_file_path(settings_path: &Path) -> PathBuf {
    settings_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(CHAIN_FILE)
}

pub fn read_chain_record(chain_path: &Path) -> Option<StatuslineChainRecord> {
    let bytes = fs::read(chain_path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

pub fn write_chain_record(
    chain_path: &Path,
    record: &StatuslineChainRecord,
) -> Result<(), AdapterError> {
    let data =
        serde_json::to_vec_pretty(record).map_err(|e| AdapterError::ConfigWrite(e.to_string()))?;
    atomic_write(chain_path, &data)
}

pub fn install_statusline_shim(
    settings_path: &Path,
    shim_command: &str,
) -> Result<Option<String>, AdapterError> {
    let previous = read_previous(settings_path)?;
    let _backup_path = backup_settings(settings_path, previous.as_deref())?;
    let mut document = parse_settings(previous.as_deref().unwrap_or(b""))?;
    let chain_path = default_chain_file_path(settings_path);
    let current_cmd = document
        .get("statusLine")
        .and_then(|status_obj| status_obj.get("command"))
        .and_then(|c| c.as_str());

    let original_command = if let Some(cmd) = current_cmd {
        if !cmd.contains("agentpet statusline") {
            Some(cmd.to_string())
        } else {
            read_chain_record(&chain_path).and_then(|rec| rec.original_command)
        }
    } else {
        read_chain_record(&chain_path).and_then(|rec| rec.original_command)
    };
    // Store original command in chain.json
    let chain_record = StatuslineChainRecord {
        original_command: original_command.clone(),
        installed_at: chrono::Utc::now().to_rfc3339(),
    };
    write_chain_record(&chain_path, &chain_record)?;

    // Set command cleanly to shim_command WITHOUT embedding arguments
    document["statusLine"] = serde_json::json!({
        "type": "command",
        "command": shim_command,
    });
    write_verified(settings_path, &document, previous.as_deref())?;
    Ok(original_command)
}

pub fn uninstall_statusline_shim(settings_path: &Path) -> Result<bool, AdapterError> {
    let previous = read_previous(settings_path)?;
    let Some(bytes) = previous else {
        return Ok(false);
    };
    let mut document = parse_settings(&bytes)?;
    let mut removed = false;
    let chain_path = default_chain_file_path(settings_path);
    let chain_record = read_chain_record(&chain_path);

    if let Some(status_obj) = document.get_mut("statusLine") {
        if let Some(cmd) = status_obj.get("command").and_then(|c| c.as_str()) {
            if cmd.contains("agentpet statusline") {
                if let Some(record) = chain_record {
                    if let Some(orig) = record.original_command {
                        *status_obj = serde_json::json!({
                            "type": "command",
                            "command": orig,
                        });
                    } else {
                        document.as_object_mut().map(|m| m.remove("statusLine"));
                    }
                } else if let Some(chained) = cmd
                    .split("--chain \"")
                    .nth(1)
                    .and_then(|s| s.strip_suffix('"'))
                {
                    *status_obj = serde_json::json!({
                        "type": "command",
                        "command": chained,
                    });
                } else {
                    document.as_object_mut().map(|m| m.remove("statusLine"));
                }
                removed = true;
            }
        }
    }
    if removed {
        write_verified(settings_path, &document, Some(&bytes))?;
        let _ = fs::remove_file(chain_path);
    }
    Ok(removed)
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
    let timestamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
    let name = format!(
        "{}.{}.bak",
        settings_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy(),
        timestamp
    );
    let backup_path = settings_path.with_file_name(name);
    fs::write(&backup_path, previous).map_err(|e| AdapterError::ConfigWrite(e.to_string()))?;

    // Also write standard .agentpet.bak for backwards compatibility
    let std_name = format!(
        "{}{}",
        settings_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy(),
        BACKUP_SUFFIX
    );
    let std_backup = settings_path.with_file_name(std_name);
    let _ = fs::write(&std_backup, previous);

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

pub fn atomic_write(target_path: &Path, data: &[u8]) -> Result<(), AdapterError> {
    let parent = target_path.parent().unwrap_or_else(|| Path::new("."));
    if !parent.as_os_str().is_empty() {
        fs::create_dir_all(parent).map_err(|e| AdapterError::ConfigWrite(e.to_string()))?;
    }
    let tmp_name = format!(
        "{}.tmp.{}",
        target_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy(),
        uuid::Uuid::new_v4().simple()
    );
    let tmp_path = parent.join(tmp_name);
    fs::write(&tmp_path, data).map_err(|e| AdapterError::ConfigWrite(e.to_string()))?;

    #[cfg(windows)]
    {
        if target_path.exists() {
            let old_tmp = parent.join(format!(
                "{}.old.{}",
                target_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy(),
                uuid::Uuid::new_v4().simple()
            ));
            let _ = fs::rename(target_path, &old_tmp);
            if let Err(e) = fs::rename(&tmp_path, target_path) {
                let _ = fs::rename(&old_tmp, target_path);
                let _ = fs::remove_file(&tmp_path);
                return Err(AdapterError::ConfigWrite(e.to_string()));
            }
            let _ = fs::remove_file(&old_tmp);
            return Ok(());
        }
    }
    fs::rename(&tmp_path, target_path).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        AdapterError::ConfigWrite(e.to_string())
    })?;
    Ok(())
}

fn write_verified(
    settings_path: &Path,
    document: &serde_json::Value,
    previous: Option<&[u8]>,
) -> Result<(), AdapterError> {
    let serialized = serde_json::to_vec_pretty(document)
        .map_err(|e| AdapterError::ConfigWrite(e.to_string()))?;
    if let Err(e) = atomic_write(settings_path, &serialized) {
        restore_previous(settings_path, previous);
        return Err(e);
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
            let _ = atomic_write(settings_path, bytes);
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
        let subagent_stop =
            parse_hook_payload(br#"{"session_id":"abc","hook_event_name":"SubagentStop"}"#)
                .expect("subagent stop parses");
        assert_eq!(subagent_stop.kind, HookEventKind::AgentWorking);
    }

    #[test]
    fn rejects_malformed_and_unsupported_payloads() {
        assert!(parse_hook_payload(b"not json").is_err());
        assert!(parse_hook_payload(br#"{"hook_event_name":"UnknownEvent"}"#).is_err());
        assert!(parse_hook_payload(br#"{"hook_event_name":"SessionStart"}"#).is_err());
        assert!(parse_hook_payload(&vec![0_u8; MAX + 1]).is_err());
    }

    #[test]
    fn tool_failure_and_compaction_keep_the_session_working() {
        let failure = parse_hook_payload(
            br#"{"session_id":"abc","hook_event_name":"PostToolUseFailure","tool_name":"Bash","tool_use_id":"t2"}"#,
        )
        .expect("tool failure parses");
        assert_eq!(failure.kind, HookEventKind::AgentWorking);
        let pre_compact =
            parse_hook_payload(br#"{"session_id":"abc","hook_event_name":"PreCompact"}"#)
                .expect("pre compact parses");
        assert_eq!(pre_compact.kind, HookEventKind::AgentThinking);
        let post_compact =
            parse_hook_payload(br#"{"session_id":"abc","hook_event_name":"PostCompact"}"#)
                .expect("post compact parses");
        assert_eq!(post_compact.kind, HookEventKind::AgentWorking);
    }

    #[test]
    fn only_permission_prompts_require_input() {
        let permission = parse_hook_payload(
            br#"{"session_id":"abc","hook_event_name":"Notification","notification_type":"permission_prompt"}"#,
        )
        .expect("permission prompt parses");
        assert_eq!(permission.kind, HookEventKind::AgentInputRequired);
        let idle = parse_hook_payload(
            br#"{"session_id":"abc","hook_event_name":"Notification","notification_type":"idle_prompt"}"#,
        )
        .expect("idle prompt parses");
        assert_eq!(idle.kind, HookEventKind::AgentWaiting);
        assert_ne!(permission.event_id, idle.event_id);
        let untyped =
            parse_hook_payload(br#"{"session_id":"abc","hook_event_name":"Notification"}"#)
                .expect("untyped notification parses");
        assert_eq!(untyped.kind, HookEventKind::AgentWaiting);
    }

    #[test]
    fn install_registers_every_consumed_event() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(SETTINGS_FILE);
        install_hook(&path, "agentpet hook").expect("install");
        let doc: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).expect("read")).expect("parse");
        for event in ["PostToolUseFailure", "PreCompact", "PostCompact"] {
            assert!(
                hook_entries(&doc, event)
                    .iter()
                    .any(|e| entry_invokes(e, "agentpet hook")),
                "{event} hook is registered"
            );
        }
        assert!(uninstall_hook(&path, "agentpet hook").expect("uninstall"));
        assert!(!fs::read_to_string(&path)
            .expect("read")
            .contains("agentpet hook"));
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

    #[test]
    fn resolves_settings_path_on_current_platform() {
        let path = default_settings_path();
        assert!(
            path.is_some(),
            "default_settings_path must resolve on this platform"
        );
        let path = path.unwrap();
        assert!(
            path.ends_with(format!(".claude{}", std::path::MAIN_SEPARATOR).as_str())
                || path.ends_with(SETTINGS_FILE)
        );
    }

    #[test]
    fn statusline_preserves_spaces_in_paths_and_unicode() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("Folder With Spaces 🚀").join(SETTINGS_FILE);
        let orig_cmd = r#"C:\Program Files\My Tools\status.exe --tag "こんにちは世界""#;
        let seed = serde_json::json!({
            "statusLine": {
                "type": "command",
                "command": orig_cmd
            },
            "customUnknownKey": { "nested": "unicode 🎯 value" }
        });
        fs::create_dir_all(path.parent().unwrap()).expect("create parent");
        fs::write(&path, serde_json::to_vec(&seed).unwrap()).expect("write seed");

        let returned_orig = install_statusline_shim(&path, "agentpet statusline").expect("install");
        assert_eq!(returned_orig.as_deref(), Some(orig_cmd));

        // Read modified settings: customUnknownKey preserved, statusLine is just "agentpet statusline"
        let modified: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(
            modified["statusLine"]["command"].as_str().unwrap(),
            "agentpet statusline"
        );
        assert_eq!(
            modified["customUnknownKey"]["nested"].as_str().unwrap(),
            "unicode 🎯 value"
        );

        // Uninstall restores exact original
        let uninstalled = uninstall_statusline_shim(&path).expect("uninstall");
        assert!(uninstalled);
        let restored: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(
            restored["statusLine"]["command"].as_str().unwrap(),
            orig_cmd
        );
        assert_eq!(
            restored["customUnknownKey"]["nested"].as_str().unwrap(),
            "unicode 🎯 value"
        );
    }

    #[test]
    fn statusline_preserves_escaped_quotes_and_flags() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(SETTINGS_FILE);
        let orig_cmd = r#"my_tool --format="json" --filter="name=\"test\"""#;
        let seed = serde_json::json!({
            "statusLine": {
                "type": "command",
                "command": orig_cmd
            }
        });
        fs::write(&path, serde_json::to_vec(&seed).unwrap()).expect("write seed");

        install_statusline_shim(&path, "agentpet statusline").expect("install");
        let uninstalled = uninstall_statusline_shim(&path).expect("uninstall");
        assert!(uninstalled);

        let restored: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(
            restored["statusLine"]["command"].as_str().unwrap(),
            orig_cmd
        );
    }

    #[test]
    fn statusline_handles_missing_settings_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(SETTINGS_FILE);
        assert!(!path.exists());

        let orig = install_statusline_shim(&path, "agentpet statusline").expect("install");
        assert_eq!(orig, None);
        assert!(path.exists());

        let doc: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(
            doc["statusLine"]["command"].as_str().unwrap(),
            "agentpet statusline"
        );

        let uninstalled = uninstall_statusline_shim(&path).expect("uninstall");
        assert!(uninstalled);
        let restored: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert!(restored.get("statusLine").is_none());
    }

    #[test]
    fn statusline_refuses_invalid_json() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(SETTINGS_FILE);
        fs::write(&path, b"INVALID { JSON").expect("write invalid");

        let result = install_statusline_shim(&path, "agentpet statusline");
        assert!(result.is_err());

        // File must not be modified
        let content = fs::read(&path).unwrap();
        assert_eq!(content, b"INVALID { JSON");
    }

    #[test]
    fn statusline_creates_timestamped_backup() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(SETTINGS_FILE);
        fs::write(&path, br#"{"existingKey":"value"}"#).expect("write seed");

        install_hook(&path, "agentpet hook").expect("install hook");

        // Verify that a backup file with .bak exists
        let entries = fs::read_dir(dir.path()).unwrap();
        let backup_exists = entries
            .flatten()
            .any(|e| e.file_name().to_string_lossy().contains(".bak"));
        assert!(backup_exists, "backup file must be created on modification");
    }
}

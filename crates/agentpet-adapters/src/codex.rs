//! OpenAI Codex CLI native hook adapter.
//!
//! Codex runs `command` hooks registered in `hooks.json` (under `CODEX_HOME`,
//! default `~/.codex`) and feeds each one a JSON payload on stdin. The
//! document uses the same `{"hooks": {"<Event>": [{"hooks": [...]}]}}` shape
//! as Claude Code settings, so installation reuses the Claude Code helpers.
//!
//! Codex records trust against a hash of each hook definition and skips new
//! or changed hooks until the user trusts them in `/hooks`. [`HOOK_COMMAND`]
//! must therefore stay byte-stable across releases.

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use uuid::Uuid;

use crate::{
    claude_code::{self, InstallReport},
    AdapterError, HookEventKind, NormalizedEvent,
};

pub const ADAPTER_ID: &str = "codex";
/// Hook command written to `hooks.json`. Changing it forces a Codex re-trust.
pub const HOOK_COMMAND: &str = "agentpet hook --adapter codex";
/// File name of the Codex hooks document.
pub const HOOKS_FILE: &str = "hooks.json";

/// Codex hook events this adapter consumes.
pub const HOOK_EVENTS: &[&str] = &[
    "SessionStart",
    "UserPromptSubmit",
    "PreToolUse",
    "PostToolUse",
    "PermissionRequest",
    "PreCompact",
    "PostCompact",
    "SubagentStart",
    "SubagentStop",
    "Stop",
    "Interrupt",
];

/// Lifecycle-tracking events required to verify hook installation.
pub const REQUIRED_HOOK_EVENTS: &[&str] = &[
    "SessionStart",
    "UserPromptSubmit",
    "PreToolUse",
    "PostToolUse",
    "PermissionRequest",
    "Stop",
];

/// Shown after install: Codex will not run the hooks until they are trusted.
pub const TRUST_NOTICE: &str = "Codex skips new hooks until you trust them: run `/hooks` in Codex \
and trust the AgentPet entries. Hooks must not be disabled with `[features] hooks = false` in \
config.toml.";

/// Fixed namespace for deterministic Codex event UUIDs (v5).
const EVENT_NAMESPACE: Uuid = Uuid::from_bytes([
    0x9a, 0x3e, 0x51, 0x0c, 0x7d, 0x42, 0x5f, 0x16, 0xb3, 0x88, 0x2e, 0x64, 0xc1, 0x0f, 0x7a, 0x95,
]);

#[derive(Debug, serde::Deserialize)]
struct CodexHookPayload {
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    hook_event_name: Option<String>,
    #[serde(default)]
    turn_id: Option<String>,
    #[serde(default)]
    tool_name: Option<String>,
    #[serde(default)]
    tool_use_id: Option<String>,
    #[serde(default)]
    cwd: Option<String>,
}

/// Parses a Codex hook payload and normalizes it.
///
/// Prompts, tool input, tool output, and assistant messages are never
/// deserialized. The event id includes `turn_id`, so each turn's `Stop` is a
/// distinct event while duplicate deliveries of one firing dedupe.
pub fn parse_hook_payload(input: &[u8]) -> Result<NormalizedEvent, AdapterError> {
    if input.len() > crate::MAX_HOOK_PAYLOAD_BYTES {
        return Err(AdapterError::PayloadTooLarge);
    }
    let payload: CodexHookPayload = serde_json::from_slice(input)?;
    let hook_event = payload
        .hook_event_name
        .as_deref()
        .ok_or(AdapterError::MissingField("hook_event_name"))?;
    let kind = match hook_event {
        "SessionStart" => HookEventKind::AgentStarted,
        "UserPromptSubmit" | "PreToolUse" | "PostToolUse" | "PostCompact" | "SubagentStart"
        | "SubagentStop" => HookEventKind::AgentWorking,
        "PreCompact" => HookEventKind::AgentThinking,
        "PermissionRequest" => HookEventKind::AgentInputRequired,
        // An interrupted turn waits for the user. AgentFailed would end the
        // session as failed.
        "Interrupt" => HookEventKind::AgentWaiting,
        "Stop" => HookEventKind::AgentCompleted,
        other => return Err(AdapterError::UnsupportedHookEvent(other.to_owned())),
    };
    let session_id = non_empty(payload.session_id);
    if matches!(
        kind,
        HookEventKind::AgentStarted | HookEventKind::AgentCompleted
    ) && session_id.is_none()
    {
        return Err(AdapterError::MissingField("session_id"));
    }
    let turn_id = non_empty(payload.turn_id);
    let tool_use_id = non_empty(payload.tool_use_id);
    let tool_name = non_empty(payload.tool_name);
    Ok(NormalizedEvent {
        event_id: deterministic_event_id(&[
            session_id.as_deref().unwrap_or(""),
            hook_event,
            turn_id.as_deref().unwrap_or(""),
            tool_use_id.as_deref().unwrap_or(""),
            tool_name.as_deref().unwrap_or(""),
        ]),
        kind,
        agent_id: ADAPTER_ID.to_owned(),
        session_id,
        project_path: non_empty(payload.cwd),
    })
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|value| !value.trim().is_empty())
}

fn deterministic_event_id(parts: &[&str]) -> String {
    Uuid::new_v5(&EVENT_NAMESPACE, parts.join("|").as_bytes()).to_string()
}

/// Default hooks path: `$CODEX_HOME/hooks.json`, else `~/.codex/hooks.json`.
pub fn default_hooks_path() -> Option<PathBuf> {
    hooks_path_from(
        std::env::var_os("CODEX_HOME"),
        std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")),
    )
}

fn hooks_path_from(codex_home: Option<OsString>, home: Option<OsString>) -> Option<PathBuf> {
    let codex_home = codex_home
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| home.map(|home| PathBuf::from(home).join(".codex")))?;
    Some(codex_home.join(HOOKS_FILE))
}

/// Installs the hook command for every supported Codex hook event.
pub fn install_hook(hooks_path: &Path, command: &str) -> Result<InstallReport, AdapterError> {
    claude_code::install_hook_events(hooks_path, command, HOOK_EVENTS)
}

/// Removes only AgentPet-owned hook entries invoking `command`.
pub fn uninstall_hook(hooks_path: &Path, command: &str) -> Result<bool, AdapterError> {
    claude_code::uninstall_hook_events(hooks_path, command, HOOK_EVENTS)
}

/// Returns true when `command` is registered for required lifecycle events.
pub fn is_installed(hooks_path: &Path, command: &str) -> Result<bool, AdapterError> {
    claude_code::is_installed_for_events(hooks_path, command, REQUIRED_HOOK_EVENTS)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn parse(json: &str) -> NormalizedEvent {
        parse_hook_payload(json.as_bytes()).expect("payload parses")
    }

    #[test]
    fn maps_codex_hook_events() {
        let cases = [
            ("SessionStart", HookEventKind::AgentStarted),
            ("UserPromptSubmit", HookEventKind::AgentWorking),
            ("PreToolUse", HookEventKind::AgentWorking),
            ("PostToolUse", HookEventKind::AgentWorking),
            ("PermissionRequest", HookEventKind::AgentInputRequired),
            ("PreCompact", HookEventKind::AgentThinking),
            ("PostCompact", HookEventKind::AgentWorking),
            ("SubagentStart", HookEventKind::AgentWorking),
            ("SubagentStop", HookEventKind::AgentWorking),
            ("Interrupt", HookEventKind::AgentWaiting),
            ("Stop", HookEventKind::AgentCompleted),
        ];
        for (event, kind) in cases {
            let parsed = parse(&format!(
                r#"{{"session_id":"s1","hook_event_name":"{event}","turn_id":"t1","cwd":"/work"}}"#
            ));
            assert_eq!(parsed.kind, kind, "{event}");
            assert_eq!(parsed.agent_id, ADAPTER_ID);
            assert_eq!(parsed.session_id.as_deref(), Some("s1"));
            assert_eq!(parsed.project_path.as_deref(), Some("/work"));
        }
        assert_eq!(cases.len(), HOOK_EVENTS.len());
    }

    #[test]
    fn event_ids_dedupe_replays_but_separate_turns() {
        let first = parse(r#"{"session_id":"s1","hook_event_name":"Stop","turn_id":"t1"}"#);
        let replay = parse(r#"{"session_id":"s1","hook_event_name":"Stop","turn_id":"t1"}"#);
        let next_turn = parse(r#"{"session_id":"s1","hook_event_name":"Stop","turn_id":"t2"}"#);
        assert_eq!(first.event_id, replay.event_id);
        assert_ne!(first.event_id, next_turn.event_id);
        let tool_a = parse(
            r#"{"session_id":"s1","hook_event_name":"PreToolUse","turn_id":"t1","tool_name":"shell","tool_use_id":"a"}"#,
        );
        let tool_b = parse(
            r#"{"session_id":"s1","hook_event_name":"PreToolUse","turn_id":"t1","tool_name":"shell","tool_use_id":"b"}"#,
        );
        assert_ne!(tool_a.event_id, tool_b.event_id);
    }

    #[test]
    fn ignores_content_fields_and_rejects_bad_payloads() {
        let parsed = parse(
            r#"{"session_id":"s1","hook_event_name":"UserPromptSubmit","turn_id":"t1","prompt":"secret","tool_input":{"command":"rm -rf /"},"last_assistant_message":"x"}"#,
        );
        assert_eq!(parsed.kind, HookEventKind::AgentWorking);
        assert!(parse_hook_payload(b"not json").is_err());
        assert!(parse_hook_payload(br#"{"session_id":"s1"}"#).is_err());
        assert!(parse_hook_payload(br#"{"hook_event_name":"SessionEnd"}"#).is_err());
        assert!(parse_hook_payload(br#"{"hook_event_name":"Stop","session_id":" "}"#).is_err());
        assert!(parse_hook_payload(br#"{"hook_event_name":"SessionStart"}"#).is_err());
        assert!(parse_hook_payload(&vec![b' '; crate::MAX_HOOK_PAYLOAD_BYTES + 1]).is_err());
    }

    #[test]
    fn resolves_hooks_path_from_codex_home_then_home() {
        assert_eq!(
            hooks_path_from(Some("/opt/codex".into()), Some("/home/u".into())),
            Some(PathBuf::from("/opt/codex").join(HOOKS_FILE))
        );
        assert_eq!(
            hooks_path_from(Some("".into()), Some("/home/u".into())),
            Some(PathBuf::from("/home/u").join(".codex").join(HOOKS_FILE))
        );
        assert_eq!(hooks_path_from(None, None), None);
    }

    #[test]
    fn install_is_idempotent_reversible_and_preserves_user_hooks() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(HOOKS_FILE);
        fs::write(
            &path,
            br#"{"description":"mine","hooks":{"Stop":[{"hooks":[{"type":"command","command":"user-stop"}]}]}}"#,
        )
        .expect("seed hooks");
        let report = install_hook(&path, HOOK_COMMAND).expect("install");
        assert!(report.backup_path.is_some());
        assert!(is_installed(&path, HOOK_COMMAND).expect("check"));
        let after_first = fs::read(&path).expect("read");
        install_hook(&path, HOOK_COMMAND).expect("reinstall");
        assert_eq!(
            fs::read(&path).expect("read"),
            after_first,
            "reinstall must not change the hook definitions Codex has trusted"
        );
        let doc: serde_json::Value = serde_json::from_slice(&after_first).expect("parse");
        for event in HOOK_EVENTS {
            let entries = doc["hooks"][*event].as_array().expect("event entries");
            assert!(
                entries
                    .iter()
                    .any(|entry| entry["hooks"][0]["command"] == HOOK_COMMAND),
                "{event} is registered"
            );
        }
        assert!(uninstall_hook(&path, HOOK_COMMAND).expect("uninstall"));
        assert!(!is_installed(&path, HOOK_COMMAND).expect("recheck"));
        let doc: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).expect("read")).expect("parse");
        assert_eq!(doc["description"], "mine");
        assert_eq!(doc["hooks"]["Stop"][0]["hooks"][0]["command"], "user-stop");
        assert!(doc["hooks"].get("PreToolUse").is_none());
        assert!(!uninstall_hook(&path, HOOK_COMMAND).expect("second uninstall"));
    }

    #[test]
    fn install_creates_missing_hooks_file_and_refuses_invalid_json() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("codex-home").join(HOOKS_FILE);
        let report = install_hook(&path, HOOK_COMMAND).expect("install into new dir");
        assert!(report.backup_path.is_none());
        assert!(is_installed(&path, HOOK_COMMAND).expect("check"));

        let broken = dir.path().join("broken.json");
        fs::write(&broken, b"{not json").expect("seed broken");
        assert!(install_hook(&broken, HOOK_COMMAND).is_err());
        assert_eq!(fs::read(&broken).expect("read"), b"{not json");
    }
}

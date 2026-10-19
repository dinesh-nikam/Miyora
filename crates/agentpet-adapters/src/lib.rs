//! Agent adapter contracts: normalized events, adapter registry, the Claude
//! Code native hook adapter, and the versioned local hook IPC endpoint.
//!
//! Adapters consume documented hook payloads and emit normalized events. They
//! never fabricate session completion or token usage. The universal wrapper
//! and the hook daemon both persist through `agentpet-core`, which owns the
//! authoritative event and session lifecycle.

pub mod claude_code;
pub mod ipc;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Current hook IPC protocol version. Bumped on breaking envelope changes.
pub const PROTOCOL_VERSION: u16 = 1;
/// Maximum bytes accepted for a single hook payload or IPC frame.
pub const MAX_HOOK_PAYLOAD_BYTES: usize = 256 * 1024;

#[derive(Debug, Error)]
pub enum AdapterError {
    #[error("hook payload is not valid UTF-8")]
    PayloadNotUtf8,
    #[error("hook payload exceeds the {MAX_HOOK_PAYLOAD_BYTES} byte limit")]
    PayloadTooLarge,
    #[error("hook payload JSON is invalid: {0}")]
    InvalidJson(#[from] serde_json::Error),
    #[error("hook payload is missing required field: {0}")]
    MissingField(&'static str),
    #[error("hook payload field is invalid: {0}")]
    InvalidField(&'static str),
    #[error("unsupported hook IPC protocol version {0}")]
    ProtocolVersionUnsupported(u16),
    #[error("unsupported hook event: {0}")]
    UnsupportedHookEvent(String),
    #[error("hook configuration is invalid: {0}")]
    InvalidConfig(String),
    #[error("hook configuration file could not be read: {0}")]
    ConfigRead(String),
    #[error("hook configuration file could not be written: {0}")]
    ConfigWrite(String),
    #[error("hook installation failed after backup; configuration restored: {0}")]
    InstallFailed(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Lifecycle event kinds emitted by adapters. Serialized in snake_case to
/// match the local daemon protocol documented in API.md.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HookEventKind {
    AgentStarted,
    AgentWorking,
    AgentThinking,
    AgentWaiting,
    AgentInputRequired,
    AgentCompleted,
    AgentFailed,
}

/// A normalized, privacy-filtered agent lifecycle event ready for the local
/// daemon. `event_id` is a stable idempotency key (UUID) so replays dedupe.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedEvent {
    pub event_id: String,
    pub kind: HookEventKind,
    pub agent_id: String,
    pub session_id: Option<String>,
    pub project_path: Option<String>,
}

/// Static description of a known agent adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentAdapter {
    pub id: &'static str,
    pub display_name: &'static str,
    pub executable: &'static str,
    /// True when AgentPet can install a native hook for this adapter.
    pub native_hook: bool,
}

/// Known agent adapters. Only adapters with `native_hook` support reversible
/// local hook installation; the rest are observed through the universal
/// wrapper or by emitting normalized events to `agentpet hook`.
pub const ADAPTERS: &[AgentAdapter] = &[
    AgentAdapter {
        id: "claude_code",
        display_name: "Claude Code",
        executable: "claude",
        native_hook: true,
    },
    AgentAdapter {
        id: "codex",
        display_name: "OpenAI Codex CLI",
        executable: "codex",
        native_hook: false,
    },
    AgentAdapter {
        id: "gemini_cli",
        display_name: "Gemini CLI",
        executable: "gemini",
        native_hook: false,
    },
    AgentAdapter {
        id: "aider",
        display_name: "Aider",
        executable: "aider",
        native_hook: false,
    },
    AgentAdapter {
        id: "opencode",
        display_name: "OpenCode",
        executable: "opencode",
        native_hook: false,
    },
];

pub fn adapter_by_id(id: &str) -> Option<&'static AgentAdapter> {
    ADAPTERS.iter().find(|adapter| adapter.id == id)
}

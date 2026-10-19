//! Versioned local hook IPC endpoint.
//!
//! Hooks, the CLI, and the desktop daemon exchange a versioned JSON
//! envelope over a same-user local socket (Unix) or named pipe (Windows).
//! This module owns the envelope schema, validation, dispatch into
//! `agentpet-core`, and a caller-side rate limiter.

use std::collections::VecDeque;
#[cfg(unix)]
use std::path::PathBuf;

use agentpet_core::{Core, CoreError, EventType, HookEvent, HookEventOutcome};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{AdapterError, HookEventKind, NormalizedEvent, PROTOCOL_VERSION};

/// Envelope message kind for submitting a normalized adapter event.
pub const EVENT_SUBMIT_KIND: &str = "event.submit";
const MAX_FUTURE_SKEW_SECS: i64 = 300;
const MAX_ENVELOPE_AGE_SECS: i64 = 3_600;
const MAX_SUBMITS_PER_MINUTE: usize = 120;

/// Versioned hook IPC envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookEnvelope {
    pub protocol_version: u16,
    pub message_id: String,
    pub sent_at: String,
    pub kind: String,
    pub payload: NormalizedEvent,
}

/// Structured acknowledgement returned for every envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookAck {
    pub message_id: String,
    pub accepted: bool,
    pub code: String,
    pub message: String,
    pub outcome: Option<String>,
}

impl HookAck {
    fn ok(message_id: &str, outcome: HookEventOutcome) -> Self {
        Self {
            message_id: message_id.to_owned(),
            accepted: true,
            code: "OK".into(),
            message: "event accepted".into(),
            outcome: Some(outcome_label(outcome).to_owned()),
        }
    }

    fn rejected(message_id: &str, code: &str, message: String) -> Self {
        Self {
            message_id: message_id.to_owned(),
            accepted: false,
            code: code.into(),
            message,
            outcome: None,
        }
    }
}

fn outcome_label(outcome: HookEventOutcome) -> &'static str {
    match outcome {
        HookEventOutcome::Accepted => "accepted",
        HookEventOutcome::Duplicate => "duplicate",
        HookEventOutcome::SessionStarted => "session_started",
        HookEventOutcome::SessionCompleted { .. } => "session_completed",
        HookEventOutcome::SessionFailed => "session_failed",
    }
}

/// Builds an `event.submit` envelope for a normalized adapter event.
pub fn submit_envelope(event: NormalizedEvent) -> HookEnvelope {
    HookEnvelope {
        protocol_version: PROTOCOL_VERSION,
        message_id: uuid::Uuid::new_v4().to_string(),
        sent_at: Utc::now().to_rfc3339(),
        kind: EVENT_SUBMIT_KIND.to_owned(),
        payload: event,
    }
}

/// Parses and validates a raw IPC frame against byte bound, protocol
/// version, message identity, timestamp window, and kind.
pub fn parse_frame(input: &[u8], now: DateTime<Utc>) -> Result<HookEnvelope, AdapterError> {
    if input.len() > crate::MAX_HOOK_PAYLOAD_BYTES {
        return Err(AdapterError::PayloadTooLarge);
    }
    let envelope: HookEnvelope = serde_json::from_slice(input)?;
    validate_envelope(&envelope, now)?;
    Ok(envelope)
}

fn validate_envelope(envelope: &HookEnvelope, now: DateTime<Utc>) -> Result<(), AdapterError> {
    if envelope.protocol_version != PROTOCOL_VERSION {
        return Err(AdapterError::ProtocolVersionUnsupported(
            envelope.protocol_version,
        ));
    }
    if uuid::Uuid::parse_str(&envelope.message_id).is_err() {
        return Err(AdapterError::InvalidField("message_id"));
    }
    if envelope.kind != EVENT_SUBMIT_KIND {
        return Err(AdapterError::InvalidField("kind"));
    }
    let sent_at = DateTime::parse_from_rfc3339(&envelope.sent_at)
        .map_err(|_| AdapterError::InvalidField("sent_at"))?
        .with_timezone(&Utc);
    let age = (now - sent_at).num_seconds();
    if !(-MAX_FUTURE_SKEW_SECS..=MAX_ENVELOPE_AGE_SECS).contains(&age) {
        return Err(AdapterError::InvalidField("sent_at"));
    }
    validate_event(&envelope.payload)?;
    Ok(())
}

fn validate_event(event: &NormalizedEvent) -> Result<(), AdapterError> {
    if uuid::Uuid::parse_str(&event.event_id).is_err() {
        return Err(AdapterError::InvalidField("event_id"));
    }
    if event.agent_id.trim().is_empty() || event.agent_id.len() > 128 {
        return Err(AdapterError::InvalidField("agent_id"));
    }
    if event
        .session_id
        .as_deref()
        .is_some_and(|id| id.trim().is_empty() || id.len() > 128)
    {
        return Err(AdapterError::InvalidField("session_id"));
    }
    if event
        .project_path
        .as_deref()
        .is_some_and(|p| p.len() > 4_096)
    {
        return Err(AdapterError::InvalidField("project_path"));
    }
    Ok(())
}

/// Converts a normalized adapter event into the core hook event shape.
pub fn to_hook_event(event: &NormalizedEvent) -> HookEvent {
    HookEvent {
        id: event.event_id.clone(),
        event_type: match event.kind {
            HookEventKind::AgentStarted => EventType::AgentStarted,
            HookEventKind::AgentWorking => EventType::AgentWorking,
            HookEventKind::AgentThinking => EventType::AgentThinking,
            HookEventKind::AgentWaiting => EventType::AgentWaiting,
            HookEventKind::AgentInputRequired => EventType::AgentInputRequired,
            HookEventKind::AgentCompleted => EventType::AgentCompleted,
            HookEventKind::AgentFailed => EventType::AgentFailed,
        },
        agent_id: event.agent_id.clone(),
        session_id: event.session_id.clone(),
        project_path: event.project_path.clone(),
    }
}

/// Validates a raw IPC frame and persists it through the core.
/// The core dedupes by stable event id, so daemon retries are safe.
pub fn dispatch_frame(core: &Core, input: &[u8], now: DateTime<Utc>) -> HookAck {
    let envelope = match parse_frame(input, now) {
        Ok(envelope) => envelope,
        Err(error) => {
            let (code, message) = ipc_error_code(&error);
            return HookAck::rejected("", &code, message);
        }
    };
    dispatch_envelope(core, &envelope)
}

/// Persists an already-validated envelope through the core.
pub fn dispatch_envelope(core: &Core, envelope: &HookEnvelope) -> HookAck {
    match core.record_hook_event(&to_hook_event(&envelope.payload)) {
        Ok(outcome) => HookAck::ok(&envelope.message_id, outcome),
        Err(error) => {
            let (code, message) = core_error_code(&error);
            HookAck::rejected(&envelope.message_id, &code, message)
        }
    }
}

fn ipc_error_code(error: &AdapterError) -> (String, String) {
    match error {
        AdapterError::ProtocolVersionUnsupported(_) => {
            ("PROTOCOL_VERSION_UNSUPPORTED".into(), error.to_string())
        }
        _ => ("INVALID_ENVELOPE".into(), error.to_string()),
    }
}

fn core_error_code(error: &CoreError) -> (String, String) {
    match error {
        CoreError::InvalidHookEvent(_) => ("INVALID_EVENT".into(), error.to_string()),
        _ => (
            "PERSIST_FAILED".into(),
            "the event could not be persisted".into(),
        ),
    }
}

/// Sliding-window submit rate limiter for one IPC caller.
pub struct SubmitRateLimiter {
    max_per_minute: usize,
    window: VecDeque<DateTime<Utc>>,
}

impl SubmitRateLimiter {
    pub fn new() -> Self {
        Self {
            max_per_minute: MAX_SUBMITS_PER_MINUTE,
            window: VecDeque::new(),
        }
    }

    /// Records a submit at `now`; false means the per-minute budget is
    /// exhausted and the caller should answer `RATE_LIMITED`.
    pub fn check(&mut self, now: DateTime<Utc>) -> bool {
        while self
            .window
            .front()
            .is_some_and(|old| (now - *old).num_seconds() >= 60)
        {
            self.window.pop_front();
        }
        if self.window.len() >= self.max_per_minute {
            return false;
        }
        self.window.push_back(now);
        true
    }
}

impl Default for SubmitRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

/// Local daemon socket path on Unix.
#[cfg(unix)]
pub fn daemon_socket_path() -> Option<PathBuf> {
    agentpet_core::default_data_directory()
        .map(|dir| dir.join("agentpet.sock"))
        .ok()
}

/// Windows named-pipe name for the local daemon endpoint.
#[cfg(windows)]
pub fn daemon_pipe_name() -> String {
    r"\\.\pipe\dev.agentpet.desktop".to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claude_code;

    fn sample_event() -> NormalizedEvent {
        claude_code::parse_hook_payload(
            br#"{"session_id":"s1","hook_event_name":"SessionStart","cwd":"/tmp"}"#,
        )
        .expect("fixture parses")
    }

    fn frame_bytes(envelope: &HookEnvelope) -> Vec<u8> {
        serde_json::to_vec(envelope).expect("envelope serializes")
    }

    #[test]
    fn accepts_valid_envelope_and_persists_idempotently() {
        let core = Core::in_memory().expect("in-memory core");
        let envelope = submit_envelope(sample_event());
        let frame = frame_bytes(&envelope);
        let first = dispatch_frame(&core, &frame, Utc::now());
        assert!(first.accepted);
        assert_eq!(first.code, "OK");
        let replay = dispatch_frame(&core, &frame, Utc::now());
        assert!(replay.accepted, "replays dedupe instead of failing");
    }

    #[test]
    fn rejects_wrong_version_stale_timestamp_and_bad_kind() {
        let mut envelope = submit_envelope(sample_event());
        envelope.protocol_version = PROTOCOL_VERSION + 1;
        assert!(parse_frame(&frame_bytes(&envelope), Utc::now()).is_err());

        let mut envelope = submit_envelope(sample_event());
        envelope.kind = "event.unknown".into();
        assert!(parse_frame(&frame_bytes(&envelope), Utc::now()).is_err());

        let mut envelope = submit_envelope(sample_event());
        envelope.sent_at = (Utc::now() - chrono::Duration::hours(2)).to_rfc3339();
        assert!(parse_frame(&frame_bytes(&envelope), Utc::now()).is_err());

        let mut envelope = submit_envelope(sample_event());
        envelope.message_id = "not-a-uuid".into();
        assert!(parse_frame(&frame_bytes(&envelope), Utc::now()).is_err());
    }

    #[test]
    fn rate_limiter_trips_after_the_minute_budget() {
        let mut limiter = SubmitRateLimiter::new();
        let now = Utc::now();
        for _ in 0..MAX_SUBMITS_PER_MINUTE {
            assert!(limiter.check(now));
        }
        assert!(!limiter.check(now));
        assert!(limiter.check(now + chrono::Duration::seconds(61)));
    }
}

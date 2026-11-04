//! Versioned local hook IPC endpoint.
//!
//! Hooks, the CLI, and the desktop daemon exchange a versioned JSON
//! envelope over a same-user local socket (Unix) or named pipe (Windows).
//! This module owns the envelope schema, validation, dispatch into
//! `agentpet-core`, and a caller-side rate limiter.

use std::collections::VecDeque;
use std::path::Path;
#[cfg(unix)]
use std::path::PathBuf;
use std::time::Duration;

use agentpet_core::{Core, CoreError, EventType, HookEvent, HookEventOutcome};
use agentpet_ipc::{DaemonServer, Endpoint, IpcError, LocalSecret};
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

/// Handle to a running background IPC server.
pub struct IpcServerHandle {
    shutdown_sender: tokio::sync::broadcast::Sender<()>,
}

impl IpcServerHandle {
    /// Signals the IPC server to shut down cleanly.
    pub fn shutdown(&self) {
        let _ = self.shutdown_sender.send(());
    }
}

impl Drop for IpcServerHandle {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Runs the local daemon IPC server until a shutdown signal is received.
pub async fn run_ipc_server(
    core: Core,
    endpoint: Endpoint,
    secret: LocalSecret,
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
) -> Result<(), IpcError> {
    let mut server = DaemonServer::bind(&endpoint).await?;
    let secret_bytes = secret.as_bytes().to_vec();

    loop {
        tokio::select! {
            _ = shutdown_rx.recv() => {
                break;
            }
            accept_res = server.accept() => {
                match accept_res {
                    Ok(mut conn) => {
                        let core = core.clone();
                        let secret_bytes = secret_bytes.clone();
                        tokio::spawn(async move {
                            if let Err(e) = conn.authenticate_server(&secret_bytes).await {
                                eprintln!("agentpet IPC server: client authentication failed: {e}");
                                return;
                            }
                            let mut limiter = SubmitRateLimiter::new();
                            while let Ok(Some(frame)) = conn.read_frame().await {
                                let now = Utc::now();
                                if !limiter.check(now) {
                                    let ack = HookAck::rejected("", "RATE_LIMITED", "rate limit exceeded".to_string());
                                    let _ = conn.write_json(&ack).await;
                                    continue;
                                }
                                let ack = dispatch_frame(&core, &frame, now);
                                if conn.write_json(&ack).await.is_err() {
                                    break;
                                }
                            }
                        });
                    }
                    Err(err) => {
                        eprintln!("agentpet IPC server: accept error: {err}");
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                }
            }
        }
    }
    Ok(())
}

/// Spawns the IPC server in a background task with clean shutdown support.
pub fn spawn_ipc_server(
    core: Core,
    endpoint: Endpoint,
    secret: LocalSecret,
) -> Result<IpcServerHandle, IpcError> {
    let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);
    let handle = IpcServerHandle {
        shutdown_sender: shutdown_tx,
    };

    if let Ok(tokio_handle) = tokio::runtime::Handle::try_current() {
        tokio_handle.spawn(async move {
            if let Err(e) = run_ipc_server(core, endpoint, secret, shutdown_rx).await {
                eprintln!("agentpet IPC server error: {e}");
            }
        });
    } else {
        std::thread::Builder::new()
            .name("agentpet-ipc-server".into())
            .spawn(move || {
                let rt = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(e) => {
                        eprintln!("Failed to create tokio runtime for IPC server: {e}");
                        return;
                    }
                };
                rt.block_on(async move {
                    if let Err(e) = run_ipc_server(core, endpoint, secret, shutdown_rx).await {
                        eprintln!("agentpet IPC server error: {e}");
                    }
                });
            })
            .map_err(|e| IpcError::TransportUnavailable(std::io::Error::other(e)))?;
    }

    Ok(handle)
}

/// Sends a normalized event to the local daemon over IPC asynchronously.
pub async fn send_event_via_ipc(
    endpoint: &Endpoint,
    secret: &[u8],
    event: &NormalizedEvent,
    timeout_duration: Duration,
) -> Result<HookAck, IpcError> {
    let mut conn = tokio::time::timeout(timeout_duration, agentpet_ipc::connect(endpoint, secret))
        .await
        .map_err(|_| {
            IpcError::TransportUnavailable(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "IPC connect timed out",
            ))
        })?
        .map_err(|e| IpcError::TransportUnavailable(std::io::Error::other(e.to_string())))?;

    let envelope = submit_envelope(event.clone());
    let frame = serde_json::to_vec(&envelope).map_err(|_| IpcError::MalformedFrame {
        reason: "serialization failed",
    })?;

    tokio::time::timeout(timeout_duration, conn.write_frame(&frame))
        .await
        .map_err(|_| {
            IpcError::TransportUnavailable(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "IPC write timed out",
            ))
        })?
        .map_err(|e| IpcError::TransportUnavailable(std::io::Error::other(e.to_string())))?;

    let ack: HookAck = tokio::time::timeout(timeout_duration, conn.read_json())
        .await
        .map_err(|_| {
            IpcError::TransportUnavailable(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "IPC read timed out",
            ))
        })?
        .map_err(|e| IpcError::TransportUnavailable(std::io::Error::other(e.to_string())))?;

    Ok(ack)
}

/// Sends a normalized event to the local daemon over IPC synchronously.
pub fn send_event_sync(
    endpoint: &Endpoint,
    secret: &[u8],
    event: &NormalizedEvent,
    timeout_duration: Duration,
) -> Result<HookAck, IpcError> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| IpcError::TransportUnavailable(std::io::Error::other(e)))?;
    rt.block_on(send_event_via_ipc(
        endpoint,
        secret,
        event,
        timeout_duration,
    ))
}

/// Dispatches an event to the daemon over IPC, falling back to direct SQLite when offline.
pub fn dispatch_event_with_fallback(
    data_dir: &Path,
    event: &NormalizedEvent,
) -> Result<HookAck, CoreError> {
    if let Ok(endpoint) = Endpoint::resolve(data_dir) {
        if let Ok(secret) =
            agentpet_ipc::load_or_create_secret(&agentpet_ipc::secret_path(data_dir))
        {
            if let Ok(ack) = send_event_sync(
                &endpoint,
                secret.as_bytes(),
                event,
                Duration::from_millis(500),
            ) {
                if ack.accepted {
                    return Ok(ack);
                }
            }
        }
    }

    let core = Core::open(data_dir)?;
    let hook_event = to_hook_event(event);
    let outcome = core.record_hook_event(&hook_event)?;
    Ok(HookAck::ok(&event.event_id, outcome))
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

    #[test]
    fn hook_delivered_via_ipc_updates_live_pet_state() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let core = Core::open(temp_dir.path()).expect("core");
        #[cfg(windows)]
        let endpoint =
            Endpoint::NamedPipe(format!(r"\\.\pipe\test-pipe-live-{}", uuid::Uuid::new_v4()));
        #[cfg(unix)]
        let endpoint = Endpoint::UnixSocket(temp_dir.path().join("test_live.sock"));

        let secret = LocalSecret::generate().expect("secret");
        let handle = spawn_ipc_server(core.clone(), endpoint.clone(), secret.clone())
            .expect("server spawned");
        std::thread::sleep(Duration::from_millis(50));

        let event = NormalizedEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            kind: HookEventKind::AgentWorking,
            agent_id: "claude_code".into(),
            session_id: Some("session-live-1".into()),
            project_path: None,
        };

        let ack = send_event_sync(&endpoint, secret.as_bytes(), &event, Duration::from_secs(5))
            .expect("send via ipc");
        assert!(ack.accepted);

        let snap = core.snapshot().expect("snapshot");
        assert_eq!(snap.pet_state, agentpet_core::PetState::Working);

        let finish = NormalizedEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            kind: HookEventKind::AgentCompleted,
            agent_id: "claude_code".into(),
            session_id: Some("session-live-1".into()),
            project_path: None,
        };
        let ack2 = send_event_sync(
            &endpoint,
            secret.as_bytes(),
            &finish,
            Duration::from_secs(5),
        )
        .expect("send finish via ipc");
        assert!(ack2.accepted);

        let snap2 = core.snapshot().expect("snapshot");
        assert_eq!(snap2.pet_state, agentpet_core::PetState::Success);

        handle.shutdown();
    }

    #[test]
    fn fallback_works_when_socket_is_absent() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let event = NormalizedEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            kind: HookEventKind::AgentWorking,
            agent_id: "claude_code".into(),
            session_id: Some("session-fallback-1".into()),
            project_path: None,
        };

        // No IPC server running on temp_dir
        let ack = dispatch_event_with_fallback(temp_dir.path(), &event).expect("fallback succeeds");
        assert!(ack.accepted);

        let core = Core::open(temp_dir.path()).expect("core");
        let snap = core.snapshot().expect("snapshot");
        assert_eq!(snap.pet_state, agentpet_core::PetState::Working);
    }

    #[test]
    fn concurrent_cli_and_daemon_writes_dont_corrupt_ledger() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let core = Core::open(temp_dir.path()).expect("core");
        let endpoint_name = format!("test-pipe-conc-{}", uuid::Uuid::new_v4());
        #[cfg(windows)]
        let endpoint = Endpoint::NamedPipe(format!(r"\\.\pipe\{}", endpoint_name));
        #[cfg(unix)]
        let endpoint = Endpoint::UnixSocket(temp_dir.path().join("test_conc.sock"));

        let secret = LocalSecret::generate().expect("secret");
        let handle = spawn_ipc_server(core.clone(), endpoint.clone(), secret.clone())
            .expect("server spawned");
        std::thread::sleep(Duration::from_millis(50));

        let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
        let mut handles = Vec::new();

        // 4 threads writing via IPC
        for i in 0..4 {
            let b = barrier.clone();
            let ep = endpoint.clone();
            let sec = secret.as_bytes().to_vec();
            handles.push(std::thread::spawn(move || {
                b.wait();
                for j in 0..5 {
                    let session_id = format!("ipc-sess-{i}-{j}");
                    let start_event = NormalizedEvent {
                        event_id: uuid::Uuid::new_v4().to_string(),
                        kind: HookEventKind::AgentStarted,
                        agent_id: "test_agent".into(),
                        session_id: Some(session_id.clone()),
                        project_path: None,
                    };
                    let _ = send_event_sync(&ep, &sec, &start_event, Duration::from_secs(5));
                    let complete_event = NormalizedEvent {
                        event_id: uuid::Uuid::new_v4().to_string(),
                        kind: HookEventKind::AgentCompleted,
                        agent_id: "test_agent".into(),
                        session_id: Some(session_id),
                        project_path: None,
                    };
                    let _ = send_event_sync(&ep, &sec, &complete_event, Duration::from_secs(5));
                }
            }));
        }

        // 4 threads writing directly to SQLite (simulating offline CLI)
        for i in 0..4 {
            let b = barrier.clone();
            let path = temp_dir.path().to_path_buf();
            handles.push(std::thread::spawn(move || {
                b.wait();
                let direct_core = Core::open(&path).expect("open direct core");
                for j in 0..5 {
                    let session_id = format!("direct-sess-{i}-{j}");
                    let _ = direct_core.start_session(&session_id, "direct_agent", None);
                    let _ = direct_core.finish_session(
                        &session_id,
                        agentpet_core::SessionOutcome::Completed,
                        Some(0),
                        100,
                    );
                }
            }));
        }

        for h in handles {
            h.join().expect("thread finished");
        }

        handle.shutdown();

        let verification = core.verify_ledger().expect("ledger verifies");
        assert!(
            verification.is_valid,
            "ledger must remain valid with hash chains intact"
        );
        assert!(
            verification.total_entries > 0,
            "transactions must have been written"
        );
    }
}

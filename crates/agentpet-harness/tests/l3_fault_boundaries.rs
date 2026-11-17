//! Layer 3: Fault Injection Boundaries Test Suite
//!
//! Verifies:
//! - MYR-HARN-007: Crash consistency at persistence boundaries (kill before write, mid-transaction rollback).
//! - MYR-HARN-008: Post-commit crash and retry storm invariance (strict idempotency, no duplicate XP).
//! - MYR-HARN-009: SQLite lock contention (`SQLITE_BUSY`) recovery and typed error taxonomy.
//! - MYR-HARN-010: IPC truncated length header rejection.
//! - MYR-HARN-011: IPC oversized frame immediate rejection.
//! - MYR-HARN-012: IPC empty payload rejection.
//! - MYR-HARN-013: IPC truncated payload rejection.
//! - MYR-HARN-014: IPC handshake tampering and version mismatch rejection.

use agentpet_core::{Core, EventType, HookEvent};
use agentpet_harness::fault::{db_locks, ipc_framing, persistence};
use agentpet_ipc::framing::DEFAULT_MAX_FRAME_BYTES;
use tempfile::tempdir;
use uuid::Uuid;

#[test]
fn test_persistence_mid_transaction_crash_leaves_zero_traces() {
    let temp = tempdir().expect("tempdir");
    let session_id = Uuid::new_v4().to_string();
    let event = HookEvent {
        id: Uuid::new_v4().to_string(),
        event_type: EventType::AgentStarted,
        agent_id: "claude_code".into(),
        session_id: Some(session_id),
        project_path: Some("/test/project".into()),
    };

    persistence::simulate_mid_transaction_crash(temp.path(), &event)
        .expect("mid-transaction crash handled cleanly");
}

#[test]
fn test_persistence_post_commit_retry_is_strictly_idempotent() {
    let temp = tempdir().expect("tempdir");
    let core = Core::open(temp.path()).expect("open core");

    let session_id = Uuid::new_v4().to_string();

    // 1. Session start retry
    let start_event = HookEvent {
        id: Uuid::new_v4().to_string(),
        event_type: EventType::AgentStarted,
        agent_id: "claude_code".into(),
        session_id: Some(session_id.clone()),
        project_path: Some("/test/project".into()),
    };
    persistence::simulate_post_commit_retry(&core, &start_event)
        .expect("start retry idempotent");

    // 2. Session completed retry (XP awarding event)
    let complete_event = HookEvent {
        id: Uuid::new_v4().to_string(),
        event_type: EventType::AgentCompleted,
        agent_id: "claude_code".into(),
        session_id: Some(session_id),
        project_path: Some("/test/project".into()),
    };
    persistence::simulate_post_commit_retry(&core, &complete_event)
        .expect("complete retry idempotent");
}

#[test]
fn test_sqlite_locked_db_recovers_gracefully() {
    let temp = tempdir().expect("tempdir");
    let event = HookEvent {
        id: Uuid::new_v4().to_string(),
        event_type: EventType::AgentStarted,
        agent_id: "claude_code".into(),
        session_id: Some(Uuid::new_v4().to_string()),
        project_path: Some("/test/project".into()),
    };

    let outcome = db_locks::simulate_lock_contention_and_recovery(temp.path(), &event)
        .expect("lock contention recovered");
    assert!(
        matches!(outcome, agentpet_core::HookEventOutcome::SessionStarted),
        "Session should be started once lock releases"
    );
}

#[test]
fn test_sqlite_persisted_lock_timeout_returns_typed_busy_error() {
    let temp = tempdir().expect("tempdir");
    let event = HookEvent {
        id: Uuid::new_v4().to_string(),
        event_type: EventType::AgentStarted,
        agent_id: "claude_code".into(),
        session_id: Some(Uuid::new_v4().to_string()),
        project_path: Some("/test/project".into()),
    };

    db_locks::simulate_persisted_lock_timeout(temp.path(), &event)
        .expect("lock timeout handled with typed error");
}

#[tokio::test]
async fn test_ipc_truncated_header_rejects_without_panic() {
    for len in 1..4 {
        let partial_header = vec![0x00; len];
        ipc_framing::assert_truncated_header_rejected(&partial_header).await;
    }
}

#[tokio::test]
async fn test_ipc_oversized_frame_rejects_without_buffering() {
    let oversized = DEFAULT_MAX_FRAME_BYTES + 1024;
    ipc_framing::assert_oversized_frame_rejected(oversized, DEFAULT_MAX_FRAME_BYTES).await;
}

#[tokio::test]
async fn test_ipc_empty_payload_rejects_with_typed_error() {
    ipc_framing::assert_zero_length_payload_rejected().await;
}

#[tokio::test]
async fn test_ipc_truncated_payload_rejects_gracefully() {
    let declared_length = 64_u32;
    let actual_payload = vec![0xAA; 16]; // Only 16 bytes provided
    ipc_framing::assert_truncated_payload_rejected(declared_length, &actual_payload).await;
}

#[test]
fn test_ipc_handshake_tampered_hmac_rejects_auth() {
    let secret = b"my_super_secure_local_ipc_secret";
    ipc_framing::assert_tampered_hmac_rejected(secret);
}

#[test]
fn test_ipc_handshake_unsupported_version_rejects() {
    let secret = b"my_super_secure_local_ipc_secret";
    ipc_framing::assert_unsupported_version_rejected(secret, 999);
}

#[tokio::test]
async fn mutation_sanity_check_fails_if_corrupted_frame_is_accepted() {
    // Sanity check: verify that a valid frame IS accepted, so we know the rejection
    // tests are actually testing corruption rather than a universally rejecting function.
    let payload = b"{\"event\":\"agent_started\"}";
    let frame = agentpet_ipc::framing::encode_frame(payload);
    let mut cursor = std::io::Cursor::new(frame);
    let result = agentpet_ipc::framing::read_frame(&mut cursor, DEFAULT_MAX_FRAME_BYTES).await;

    assert!(result.is_ok(), "Valid frame must decode cleanly");
    assert_eq!(
        result.unwrap(),
        Some(payload.to_vec()),
        "Decoded payload must match original"
    );
}

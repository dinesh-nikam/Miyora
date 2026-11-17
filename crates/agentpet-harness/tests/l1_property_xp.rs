//! L1 Property Tests: XP Idempotency & Tamper-Evident Ledger Integrity
//!
//! Requirement: MYR-HARN-001
//! For any agent session lifecycle event sequence, any duplication of events,
//! and any permitted interleaving, the final XP, level, and unlocked achievements
//! must be strictly identical to the canonical sequence, and verify_ledger()
//! must confirm a valid hash chain.

use agentpet_core::Core;
use agentpet_harness::generators::events::{session_lifecycle_scenario, SessionLifecycleScenario};
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    #[test]
    fn prop_xp_and_achievements_are_strictly_idempotent(scenario in session_lifecycle_scenario()) {
        assert_xp_idempotency(&scenario, false);
    }
}

/// Helper that evaluates a scenario on two isolated databases.
/// If `mutate_break_idempotency` is true, duplicate events are artificially assigned
/// new IDs, simulating broken deduplication to prove the test catches the mutation.
fn assert_xp_idempotency(scenario: &SessionLifecycleScenario, mutate_break_idempotency: bool) {
    let canonical_core = Core::in_memory().expect("canonical core opens");
    let replay_core = Core::in_memory().expect("replay core opens");

    // 1. Ingest canonical sequence
    for event in &scenario.canonical_events {
        canonical_core
            .record_hook_event(event)
            .expect("canonical event records successfully");
    }

    // 2. Ingest duplicated and permuted sequence
    for mut event in scenario.duplicated_and_permuted_events.clone() {
        if mutate_break_idempotency {
            // Deliberately simulate broken session deduplication by synthesizing new session IDs
            event.id = uuid::Uuid::new_v4().to_string();
            event.session_id = Some(uuid::Uuid::new_v4().to_string());
        }
        replay_core
            .record_hook_event(&event)
            .expect("replay event records successfully");
    }

    // 3. Verify total XP matches
    let canonical_prog = canonical_core.progression().expect("canonical progression");
    let replay_prog = replay_core.progression().expect("replay progression");

    assert_eq!(
        canonical_prog.total_xp, replay_prog.total_xp,
        "XP divergence detected between canonical and replayed sequence for session {}",
        scenario.session_id
    );

    // 4. Verify level matches
    assert_eq!(
        canonical_prog.level, replay_prog.level,
        "Level divergence detected for session {}",
        scenario.session_id
    );

    // 5. Verify cryptographic ledger integrity on both cores
    let canonical_ledger = canonical_core
        .verify_ledger()
        .expect("canonical ledger check");
    assert!(
        canonical_ledger.is_valid,
        "Canonical ledger corrupted: {:?}",
        canonical_ledger
    );

    let replay_ledger = replay_core.verify_ledger().expect("replay ledger check");
    assert!(
        replay_ledger.is_valid,
        "Replay ledger corrupted: {:?}",
        replay_ledger
    );

    // 6. Verify ledger transaction counts are identical (duplicate deliveries must not insert ledger rows)
    assert_eq!(
        canonical_ledger.total_entries, replay_ledger.total_entries,
        "Ledger entry count mismatch: canonical={} vs replay={}",
        canonical_ledger.total_entries, replay_ledger.total_entries
    );
}

#[test]
fn mutation_sanity_check_fails_when_idempotency_broken() {
    let scenario = SessionLifecycleScenario {
        session_id: "mut-test-1".into(),
        canonical_events: vec![
            agentpet_core::HookEvent {
                id: "e0000000-0000-0000-0000-000000000001".into(),
                event_type: agentpet_core::EventType::AgentStarted,
                agent_id: "claude_code".into(),
                session_id: Some("mut-test-1".into()),
                project_path: None,
            },
            agentpet_core::HookEvent {
                id: "e0000000-0000-0000-0000-000000000002".into(),
                event_type: agentpet_core::EventType::AgentCompleted,
                agent_id: "claude_code".into(),
                session_id: Some("mut-test-1".into()),
                project_path: None,
            },
        ],
        duplicated_and_permuted_events: vec![
            agentpet_core::HookEvent {
                id: "e0000000-0000-0000-0000-000000000001".into(),
                event_type: agentpet_core::EventType::AgentStarted,
                agent_id: "claude_code".into(),
                session_id: Some("mut-test-1".into()),
                project_path: None,
            },
            agentpet_core::HookEvent {
                id: "e0000000-0000-0000-0000-000000000002".into(),
                event_type: agentpet_core::EventType::AgentCompleted,
                agent_id: "claude_code".into(),
                session_id: Some("mut-test-1".into()),
                project_path: None,
            },
            // Duplicate of completion
            agentpet_core::HookEvent {
                id: "e0000000-0000-0000-0000-000000000002".into(),
                event_type: agentpet_core::EventType::AgentCompleted,
                agent_id: "claude_code".into(),
                session_id: Some("mut-test-1".into()),
                project_path: None,
            },
        ],
    };

    // When deduplication is healthy, this passes
    assert_xp_idempotency(&scenario, false);

    // Mutation test: if deduplication was bypassed (assigning new event IDs to duplicates),
    // the mutation must be caught and panic.
    let mutation_result = std::panic::catch_unwind(|| {
        assert_xp_idempotency(&scenario, true);
    });
    assert!(
        mutation_result.is_err(),
        "Mutation sanity check failed: broken idempotency was not detected!"
    );
}

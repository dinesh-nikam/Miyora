//! L2 Replay Tests: Golden Snapshot Validation & Replay Idempotency
//!
//! Requirements: MYR-HARN-004, MYR-HARN-005
//! - Replaying real event logs against a fresh database matches the committed golden snapshot.
//! - Replaying the same log twice yields an identical database state.
//! - Replaying a log into a half-populated database yields an identical database state.

use agentpet_core::{Core, HookEvent};
use agentpet_harness::replay::normalizer::NormalizedDbSnapshot;
use agentpet_harness::replay::runner::{replay_hook_events, snapshot_from_data_dir};

const GOLDEN_FLOW_RAW: &str = include_str!("../fixtures/golden_session_flow.json");
const GOLDEN_SNAPSHOT_RAW: &str = include_str!("../fixtures/golden_snapshot.json");

fn load_golden_flow() -> Vec<HookEvent> {
    serde_json::from_str(GOLDEN_FLOW_RAW).expect("golden session flow parses")
}

fn load_golden_snapshot() -> NormalizedDbSnapshot {
    serde_json::from_str(GOLDEN_SNAPSHOT_RAW).expect("golden snapshot parses")
}

#[test]
fn test_replay_matches_golden_snapshot() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let core = Core::open(temp_dir.path()).expect("core opens");
    let events = load_golden_flow();
    let expected_snapshot = load_golden_snapshot();

    // 1. Replay full event flow
    replay_hook_events(&core, &events).expect("replay succeeds");

    // 2. Capture normalized snapshot
    let snapshot = snapshot_from_data_dir(temp_dir.path()).expect("snapshot extracted");

    // 3. Assert full database snapshot is identical to golden baseline
    assert_eq!(
        snapshot, expected_snapshot,
        "Normalized database state diverged from golden baseline!"
    );

    // 4. Verify cryptographic ledger head
    let ledger_result = core.verify_ledger().expect("ledger check");
    assert!(ledger_result.is_valid);
    assert_eq!(ledger_result.total_entries, 2);
}

#[test]
fn test_replay_twice_is_strictly_identical() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let core = Core::open(temp_dir.path()).expect("core opens");
    let events = load_golden_flow();

    // First replay
    replay_hook_events(&core, &events).expect("first replay");
    let snapshot_once = snapshot_from_data_dir(temp_dir.path()).expect("snapshot once");

    // Second replay of identical events
    replay_hook_events(&core, &events).expect("second replay");
    let snapshot_twice = snapshot_from_data_dir(temp_dir.path()).expect("snapshot twice");

    assert_eq!(
        snapshot_once, snapshot_twice,
        "Replaying an identical event log twice mutated the database state!"
    );
}

#[test]
fn test_replay_into_half_populated_db_is_strictly_identical() {
    let events = load_golden_flow();

    // Database A: Full single replay
    let dir_a = tempfile::tempdir().expect("dir_a");
    let core_a = Core::open(dir_a.path()).expect("core_a");
    replay_hook_events(&core_a, &events).expect("replay a");
    let snapshot_a = snapshot_from_data_dir(dir_a.path()).expect("snapshot a");

    // Database B: Half-populated first (first 3 events), then entire log replayed
    let dir_b = tempfile::tempdir().expect("dir_b");
    let core_b = Core::open(dir_b.path()).expect("core_b");
    replay_hook_events(&core_b, &events[..3]).expect("partial replay");
    // Replay full log (events 0..6, with 0..2 being duplicates and 3..6 being new)
    replay_hook_events(&core_b, &events).expect("full replay into partial");
    let snapshot_b = snapshot_from_data_dir(dir_b.path()).expect("snapshot b");

    assert_eq!(
        snapshot_a, snapshot_b,
        "Replaying full event stream into a half-populated DB did not match clean single replay!"
    );
}

#[test]
fn mutation_sanity_check_fails_on_snapshot_divergence() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let core = Core::open(temp_dir.path()).expect("core");
    let events = load_golden_flow();
    replay_hook_events(&core, &events).expect("replay");
    let snapshot = snapshot_from_data_dir(temp_dir.path()).expect("snapshot");

    let mut tampered_snapshot = snapshot.clone();
    // Tamper with progression total_xp
    tampered_snapshot.progression[0].total_xp = 9999;

    let result = std::panic::catch_unwind(move || {
        assert_eq!(
            snapshot, tampered_snapshot,
            "Snapshots must diverge on tampered XP!"
        );
    });

    assert!(
        result.is_err(),
        "Mutation sanity check failed: tampered snapshot was not detected!"
    );
}

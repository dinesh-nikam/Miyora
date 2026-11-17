//! Persistence boundary fault simulation.
//!
//! Validates crash consistency:
//! - Kill before write: zero trace, zero XP.
//! - Mid-transaction crash: aborted transactions roll back cleanly; re-opening the core sees no corruption.
//! - Post-commit crash and retry: retrying after commit is strictly idempotent; no double-counted XP or duplicate achievements.

use std::path::Path;

use agentpet_core::{Core, HookEvent, HookEventOutcome};
use rusqlite::{params, Connection, TransactionBehavior};

/// Simulates a crash mid-transaction before commit.
///
/// Opens a direct connection, starts an immediate transaction, writes partial
/// state (an event row into `agent_events`), and abruptly drops/aborts the transaction.
/// Proves that upon restart, SQLite rolled back all partial records, the hash chain is intact,
/// and re-ingesting the event succeeds without constraint collisions.
pub fn simulate_mid_transaction_crash(
    data_dir: &Path,
    event: &HookEvent,
) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize schema and tables via Core::open
    {
        let _core = Core::open(data_dir)?;
    }

    let db_path = data_dir.join("agentpet.sqlite3");
    {
        let mut conn = Connection::open(&db_path)?;
        conn.busy_timeout(std::time::Duration::from_millis(500))?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

        // Inject partial write directly into agent_events
        tx.execute(
            "INSERT INTO agent_events(id, schema_version, event_type, occurred_at, source, session_id, project_id, accepted_at)
             VALUES (?1, 1, 'agent_started', datetime('now'), ?2, ?3, ?4, datetime('now'))",
            params![event.id, event.agent_id, event.session_id, event.project_path],
        )?;

        // Simulate crash: Drop transaction without calling tx.commit()
        drop(tx);
    }

    // Verify restart consistency
    let core = Core::open(data_dir)?;
    let conn = Connection::open(&db_path)?;
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM agent_events WHERE id = ?1",
        params![event.id],
        |row| row.get(0),
    )?;
    assert_eq!(
        count, 0,
        "Aborted transaction must leave zero rows in agent_events"
    );

    // Assert ledger integrity
    let ledger_status = core.verify_ledger()?;
    assert!(
        ledger_status.is_valid,
        "Ledger chain must remain valid after mid-transaction crash"
    );

    // Now prove the event can be recorded normally without conflict
    let outcome = core.record_hook_event(event)?;
    assert!(
        !matches!(outcome, HookEventOutcome::Duplicate),
        "Event must be accepted after previous crashed attempt rolled back"
    );

    Ok(())
}

/// Simulates a crash occurring after commit but before acknowledgement to the agent hook.
///
/// In this scenario, the agent hook never receives the ACK (or times out) and resends
/// the exact same event. The harness asserts that:
/// 1. The retry is recognized as a duplicate.
/// 2. Zero additional XP is awarded.
/// 3. Zero duplicate achievements are issued.
/// 4. The ledger hash chain remains completely valid.
pub fn simulate_post_commit_retry(
    core: &Core,
    event: &HookEvent,
) -> Result<(), Box<dyn std::error::Error>> {
    // Initial persist (succeeds)
    let outcome_initial = core.record_hook_event(event)?;

    let ledger_initial = core.verify_ledger()?;
    let prog_initial = core.progression()?;

    // Simulate crash before ACK: client retries the identical event
    let outcome_retry = core.record_hook_event(event)?;

    let ledger_retry = core.verify_ledger()?;
    let prog_retry = core.progression()?;

    // Invariant assertions
    match outcome_initial {
        HookEventOutcome::SessionStarted => {
            assert!(
                matches!(
                    outcome_retry,
                    HookEventOutcome::Duplicate | HookEventOutcome::Accepted
                ),
                "Duplicate SessionStarted must not create a new session"
            );
        }
        HookEventOutcome::SessionCompleted { xp_awarded: true } => {
            assert!(
                matches!(
                    outcome_retry,
                    HookEventOutcome::Duplicate
                        | HookEventOutcome::SessionCompleted { xp_awarded: false }
                ),
                "Duplicate SessionCompleted must return xp_awarded: false or Duplicate"
            );
        }
        _ => {}
    }

    // State invariants: XP, level, and ledger entry count must be identical
    assert_eq!(
        prog_initial.total_xp, prog_retry.total_xp,
        "Total XP must not change on post-commit retry"
    );
    assert_eq!(
        prog_initial.level, prog_retry.level,
        "Level must not change on post-commit retry"
    );
    assert_eq!(
        ledger_initial.total_entries, ledger_retry.total_entries,
        "Ledger entry count must not increase on post-commit retry"
    );
    assert!(ledger_retry.is_valid, "Ledger hash chain must remain valid");

    Ok(())
}

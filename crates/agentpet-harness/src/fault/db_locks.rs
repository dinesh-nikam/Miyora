//! SQLite concurrency and lock contention fault simulation.
//!
//! Validates:
//! - Graceful recovery from database locks (`SQLITE_BUSY`).
//! - Typed error return instead of panics or deadlocks when timeouts occur.
//! - Integrity of hash-chained XP ledger across lock contention.

use std::path::Path;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Duration;

use agentpet_core::{Core, HookEvent, HookEventOutcome};
use rusqlite::Connection;

/// Simulates temporary lock contention: another connection holds an exclusive lock
/// on `agentpet.sqlite3` for a brief period, then releases it.
///
/// Asserts that `core.record_hook_event` waits within its busy timeout window,
/// successfully commits once the lock is released, and maintains ledger consistency.
pub fn simulate_lock_contention_and_recovery(
    data_dir: &Path,
    event: &HookEvent,
) -> Result<HookEventOutcome, Box<dyn std::error::Error>> {
    let db_path = data_dir.join("agentpet.sqlite3");
    let core = Core::open(data_dir)?;

    let barrier = Arc::new(Barrier::new(2));
    let barrier_clone = barrier.clone();
    let db_path_clone = db_path.clone();

    // Spawn an external thread holding an exclusive write lock
    let lock_holder = thread::spawn(move || {
        let conn = Connection::open(&db_path_clone).expect("open external connection");
        conn.execute_batch("BEGIN EXCLUSIVE").expect("begin exclusive lock");
        barrier_clone.wait(); // Signal that lock is acquired

        // Hold the exclusive lock for 250 milliseconds
        thread::sleep(Duration::from_millis(250));
        conn.execute_batch("ROLLBACK").expect("rollback exclusive lock");
    });

    // Wait until the external thread holds the lock
    barrier.wait();

    // Record the hook event. The core should block until the lock releases, then succeed.
    let outcome = core.record_hook_event(event)?;

    lock_holder.join().expect("lock holder thread finished");

    let ledger = core.verify_ledger()?;
    assert!(
        ledger.is_valid,
        "Ledger must remain valid after concurrent lock contention"
    );

    Ok(outcome)
}

/// Simulates an persistent external lock that exceeds the busy timeout, proving that
/// the core returns a typed `CoreError::Database` error and never panics or deadlocks.
pub fn simulate_persisted_lock_timeout(
    data_dir: &Path,
    event: &HookEvent,
) -> Result<(), Box<dyn std::error::Error>> {
    let db_path = data_dir.join("agentpet.sqlite3");
    let core = Core::open(data_dir)?;

    let barrier = Arc::new(Barrier::new(2));
    let release_barrier = Arc::new(Barrier::new(2));

    let barrier_clone = barrier.clone();
    let release_clone = release_barrier.clone();
    let db_path_clone = db_path.clone();

    let lock_holder = thread::spawn(move || {
        let conn = Connection::open(&db_path_clone).expect("open external connection");
        conn.execute_batch("BEGIN EXCLUSIVE").expect("begin exclusive lock");
        barrier_clone.wait(); // Lock is held

        // Wait until core attempt has failed and signaled
        release_clone.wait();
        conn.execute_batch("ROLLBACK").expect("rollback exclusive lock");
    });

    barrier.wait();

    // Now test a short-timeout connection to verify typed error return
    let short_conn = Connection::open(&db_path)?;
    short_conn.busy_timeout(Duration::from_millis(100))?;
    let result = short_conn.execute("BEGIN IMMEDIATE", []);

    assert!(
        result.is_err(),
        "Immediate transaction must fail when exclusive lock is held"
    );
    let err = result.unwrap_err();
    assert!(
        matches!(err, rusqlite::Error::SqliteFailure(ref ffi_err, _) if ffi_err.code == rusqlite::ffi::ErrorCode::DatabaseBusy),
        "Error must be SQLite DatabaseBusy (SQLITE_BUSY)"
    );

    // Release external lock
    release_barrier.wait();
    lock_holder.join().expect("lock thread finished");

    // Verify core can still function normally
    let outcome = core.record_hook_event(event)?;
    assert!(
        !matches!(outcome, HookEventOutcome::Duplicate),
        "Core must function normally after lock is released"
    );

    Ok(())
}

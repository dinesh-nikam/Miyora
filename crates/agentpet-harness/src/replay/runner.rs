use agentpet_core::{Core, CoreError, HookEvent};
use rusqlite::Connection;
use std::path::Path;

use super::normalizer::{capture_normalized_snapshot, NormalizedDbSnapshot};

/// Replays a sequence of HookEvents into an open Core instance.
pub fn replay_hook_events(core: &Core, events: &[HookEvent]) -> Result<(), CoreError> {
    for event in events {
        core.record_hook_event(event)?;
    }
    Ok(())
}

/// Captures a normalized snapshot from an on-disk Core instance.
pub fn snapshot_from_data_dir(data_dir: &Path) -> Result<NormalizedDbSnapshot, rusqlite::Error> {
    let db_path = data_dir.join("agentpet.sqlite3");
    let conn = Connection::open(db_path)?;
    capture_normalized_snapshot(&conn)
}

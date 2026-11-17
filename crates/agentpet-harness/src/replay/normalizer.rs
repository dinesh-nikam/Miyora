use rusqlite::Connection;
use serde::{Deserialize, Serialize};

/// Canonical, normalized representation of a database state.
/// All non-deterministic timestamps, ephemeral OS paths, and randomized UUIDs
/// are replaced with canonical placeholders so two equivalent databases produce
/// byte-identical JSON serializations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedDbSnapshot {
    pub pet_state: NormalizedPetState,
    pub progression: Vec<NormalizedProgression>,
    pub sessions: Vec<NormalizedSession>,
    pub xp_transactions: Vec<NormalizedXpTx>,
    pub ledger_entries: Vec<NormalizedLedgerEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedPetState {
    pub state: String,
    pub selected_pet_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedProgression {
    pub pet_id: String,
    pub level: i64,
    pub total_xp: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedSession {
    pub session_id: String,
    pub agent_id: String,
    pub status: String,
    pub pet_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedXpTx {
    pub pet_id: String,
    pub source: String,
    pub amount: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedLedgerEntry {
    pub index_id: u64,
    pub provider: String,
    pub amount: i64,
}

/// Extracts and normalizes the full state from a SQLite connection.
pub fn capture_normalized_snapshot(
    conn: &Connection,
) -> Result<NormalizedDbSnapshot, rusqlite::Error> {
    // 1. pet_state
    let pet_state = conn.query_row(
        "SELECT state, selected_pet_id FROM pet_state WHERE singleton = 1",
        [],
        |row| {
            Ok(NormalizedPetState {
                state: row.get(0)?,
                selected_pet_id: row.get(1)?,
            })
        },
    )?;

    // 2. pet_progression
    let mut stmt =
        conn.prepare("SELECT pet_id, level, total_xp FROM pet_progression ORDER BY pet_id ASC")?;
    let progression = stmt
        .query_map([], |row| {
            Ok(NormalizedProgression {
                pet_id: row.get(0)?,
                level: row.get(1)?,
                total_xp: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    // 3. agent_sessions
    let mut stmt = conn.prepare(
        "SELECT session_id, agent_id, status, pet_id FROM agent_sessions ORDER BY session_id ASC",
    )?;
    let sessions = stmt
        .query_map([], |row| {
            Ok(NormalizedSession {
                session_id: row.get(0)?,
                agent_id: row.get(1)?,
                status: row.get(2)?,
                pet_id: row.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    // 4. xp_transactions
    let mut stmt = conn.prepare(
        "SELECT pet_id, source, amount FROM xp_transactions ORDER BY pet_id ASC, amount ASC, source ASC",
    )?;
    let xp_transactions = stmt
        .query_map([], |row| {
            Ok(NormalizedXpTx {
                pet_id: row.get(0)?,
                source: row.get(1)?,
                amount: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    // 5. xp_ledger
    let mut stmt =
        conn.prepare("SELECT index_id, provider, amount FROM xp_ledger ORDER BY index_id ASC")?;
    let ledger_entries = stmt
        .query_map([], |row| {
            Ok(NormalizedLedgerEntry {
                index_id: row.get(0)?,
                provider: row.get(1)?,
                amount: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(NormalizedDbSnapshot {
        pet_state,
        progression,
        sessions,
        xp_transactions,
        ledger_entries,
    })
}

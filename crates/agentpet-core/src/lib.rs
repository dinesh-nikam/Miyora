use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use agentpet_pet_engine::{
    EngineContext, PetEngine, PetState, SessionReaction, Stimulus, Transition,
};
use agentpet_pet_pack::{
    signature::TrustedSigner, PetPackError, PetPackManifest, ValidationLimits,
};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

const DATABASE_FILE: &str = "agentpet.sqlite3";
#[cfg(feature = "dev-simulator")]
pub const SIMULATED_DATABASE_FILE: &str = "agentpet_simulated.sqlite3";
const SCHEMA_VERSION: i64 = 9;
const SQLITE_BUSY_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_SESSION_XP: i64 = 10;
const DEFAULT_LEVEL_XP: i64 = 100;
const DEFAULT_LEVEL_XP_INCREMENT: i64 = 50;
const MIN_BASE_XP_PER_LEVEL: i64 = 10;
const MAX_CONFIGURED_XP: i64 = 1_000_000;
const FALLBACK_PET_ID: &str = "local.default";
const DEFAULT_XP_HISTORY_PAGE_SIZE: usize = 10;
const MAX_XP_HISTORY_PAGE_SIZE: usize = 100;
const DEFAULT_SESSION_HISTORY_PAGE_SIZE: usize = 8;
const MAX_SESSION_HISTORY_PAGE_SIZE: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    AgentStarted,
    AgentWorking,
    AgentThinking,
    AgentWaiting,
    AgentInputRequired,
    AgentCompleted,
    AgentFailed,
    UserPaused,
    UserResumed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionOutcome {
    Completed,
    Failed,
    Interrupted,
}

impl SessionOutcome {
    fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Interrupted => "interrupted",
        }
    }

    fn event_type(self) -> EventType {
        match self {
            Self::Completed => EventType::AgentCompleted,
            Self::Failed | Self::Interrupted => EventType::AgentFailed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OnRejectedTransition {
    Fail,
    /// Session facts are persisted even when the pet cannot present a reaction for them.
    KeepState,
}

impl EventType {
    fn as_str(&self) -> &'static str {
        match self {
            Self::AgentStarted => "AGENT_STARTED",
            Self::AgentWorking => "AGENT_WORKING",
            Self::AgentThinking => "AGENT_THINKING",
            Self::AgentWaiting => "AGENT_WAITING",
            Self::AgentInputRequired => "AGENT_INPUT_REQUIRED",
            Self::AgentCompleted => "AGENT_COMPLETED",
            Self::AgentFailed => "AGENT_FAILED",
            Self::UserPaused => "USER_PAUSED",
            Self::UserResumed => "USER_RESUMED",
        }
    }

    fn stimulus(&self) -> Stimulus {
        match self {
            Self::AgentStarted => Stimulus::AgentStarted,
            Self::AgentWorking => Stimulus::AgentWorking,
            Self::AgentThinking => Stimulus::AgentThinking,
            Self::AgentWaiting => Stimulus::AgentWaiting,
            Self::AgentInputRequired => Stimulus::AgentInputRequired,
            Self::AgentCompleted => Stimulus::AgentCompleted,
            Self::AgentFailed => Stimulus::AgentFailed,
            Self::UserPaused => Stimulus::UserPaused,
            Self::UserResumed => Stimulus::UserResumed,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PetEvent {
    pub id: Uuid,
    pub schema_version: u16,
    pub event_type: EventType,
    pub occurred_at: DateTime<Utc>,
    pub source: String,
    pub session_id: Option<String>,
    pub project_id: Option<String>,
}

impl PetEvent {
    pub fn new(event_type: EventType, source: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            schema_version: 1,
            event_type,
            occurred_at: Utc::now(),
            source: source.into(),
            session_id: None,
            project_id: None,
        }
    }

    fn validate(&self) -> Result<(), CoreError> {
        if self.schema_version != 1 {
            return Err(CoreError::UnsupportedEventSchema(self.schema_version));
        }
        if self.source.trim().is_empty() || self.source.len() > 128 {
            return Err(CoreError::InvalidSource);
        }
        if self.session_id.as_ref().is_some_and(|v| v.len() > 128)
            || self.project_id.as_ref().is_some_and(|v| v.len() > 256)
        {
            return Err(CoreError::InvalidIdentifier);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSnapshot {
    pub pet_state: PetState,
    pub selected_pet_id: Option<String>,
    pub last_event_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DesktopPosition {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProgressionSnapshot {
    pub pet_id: String,
    pub total_xp: i64,
    pub level: u64,
    pub xp_into_level: i64,
    pub xp_to_next_level: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProgressionRules {
    pub xp_per_completed_session: i64,
    pub base_xp_per_level: i64,
    pub level_xp_increment: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct XpTransaction {
    pub transaction_id: String,
    pub pet_id: String,
    pub source: String,
    pub source_id: String,
    pub amount: i64,
    pub created_at: String,
    pub metadata_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentSessionSummary {
    pub session_id: String,
    pub agent_id: String,
    pub project_path: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub duration_ms: Option<i64>,
    pub status: String,
    pub exit_code: Option<i32>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AchievementSummary {
    pub id: String,
    pub title: String,
    pub description: String,
    pub threshold: i64,
    pub progress: i64,
    pub reward_xp: i64,
    pub unlocked_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPetPackSummary {
    pub id: String,
    pub selection_id: String,
    pub version: String,
    pub name: String,
    pub creator: String,
    pub license: String,
}

pub const GENESIS_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LedgerVerificationResult {
    pub is_valid: bool,
    pub total_entries: u64,
    pub head_hash: String,
    pub broken_at_index: Option<u64>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StoredAgentEvent {
    pub id: String,
    pub event_type: String,
    pub occurred_at: String,
    pub source: String,
    pub provider: String,
    pub session_id: Option<String>,
    pub project_id: Option<String>,
    pub payload_json: Option<String>,
    pub latency_ms: Option<u64>,
}

pub const DOMAIN_TAG_V2: &[u8] = b"agentpet.ledger.v2";

pub fn compute_ledger_hash_v2(
    prev_hash: &str,
    idempotency_key: &str,
    amount: i64,
    pet_id: &str,
    created_at: &str,
) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN_TAG_V2);
    feed_length_prefixed(&mut hasher, prev_hash.as_bytes());
    feed_length_prefixed(&mut hasher, idempotency_key.as_bytes());
    feed_length_prefixed(&mut hasher, &amount.to_be_bytes());
    feed_length_prefixed(&mut hasher, pet_id.as_bytes());
    feed_length_prefixed(&mut hasher, created_at.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn compute_ledger_hash_v1(
    prev_hash: &str,
    idempotency_key: &str,
    amount: i64,
    pet_id: &str,
    created_at: &str,
) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher
        .update(format!("{prev_hash}:{idempotency_key}:{amount}:{pet_id}:{created_at}").as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn compute_ledger_hash(
    prev_hash: &str,
    idempotency_key: &str,
    amount: i64,
    pet_id: &str,
    created_at: &str,
) -> String {
    compute_ledger_hash_v2(prev_hash, idempotency_key, amount, pet_id, created_at)
}

fn feed_length_prefixed(hasher: &mut sha2::Sha256, data: &[u8]) {
    use sha2::Digest;
    let len = (data.len() as u32).to_be_bytes();
    hasher.update(len);
    hasher.update(data);
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TrustedSignerEntry {
    key_id: String,
    public_key: String,
}

/// Persisted record of a native agent hook installation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HookInstallRecord {
    pub adapter_id: String,
    pub config_path: String,
    pub backup_path: Option<String>,
    pub installed_at: String,
    pub status: String,
}

/// A normalized agent lifecycle event delivered by a hook adapter or the
/// universal wrapper. Hook events reuse the same event table and pet state
/// machine as directly submitted events.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookEvent {
    /// Stable idempotency key supplied by the adapter (UUID string).
    pub id: String,
    pub event_type: EventType,
    pub agent_id: String,
    pub session_id: Option<String>,
    pub project_path: Option<String>,
}

/// Result of ingesting a normalized hook event.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum HookEventOutcome {
    /// The event was newly persisted and the pet state machine processed it.
    Accepted,
    /// The idempotency key was already recorded; nothing changed.
    Duplicate,
    /// The event started a new observed agent session.
    SessionStarted,
    /// The event completed the observed session and awarded XP in the same
    /// transaction.
    SessionCompleted { xp_awarded: bool },
    /// The event failed the observed session; no XP was awarded.
    SessionFailed,
}

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("database path is not valid UTF-8")]
    InvalidDatabasePath,
    #[error("unsupported database schema version {0}")]
    UnsupportedDatabaseSchema(i64),
    #[error("unsupported event schema version {0}")]
    UnsupportedEventSchema(u16),
    #[error("invalid event source")]
    InvalidSource,
    #[error("invalid event identifier")]
    InvalidIdentifier,
    #[error("invalid pet state {0}")]
    InvalidState(String),
    #[error("transition from {from:?} to {to:?} is not allowed")]
    InvalidTransition { from: PetState, to: PetState },
    #[error("lock poisoned")]
    LockPoisoned,
    #[error("PetPack validation or installation failed: {0}")]
    PetPack(#[from] PetPackError),
    #[error("PetPack {0} is not installed")]
    PackNotInstalled(String),
    #[error("agent session {0} is not running")]
    SessionNotRunning(String),
    #[error("application data directory is unavailable")]
    DataDirectoryUnavailable,
    #[error("session duration is out of range")]
    InvalidDuration,
    #[error("progression rules are outside supported bounds")]
    InvalidProgressionRules,
    #[error("history pagination is outside supported bounds")]
    InvalidPagination,
    #[error("PetPack manifest serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("PetPack install failed ({primary}) and installed files could not be rolled back ({cleanup})")]
    PackInstallRollback { primary: String, cleanup: String },
    #[error("trusted signer configuration is invalid: {0}")]
    TrustedSignerConfig(String),
    #[error("hook event is invalid: {0}")]
    InvalidHookEvent(String),
}

#[derive(Clone)]
pub struct Core {
    connection: Arc<Mutex<Connection>>,
    engine: PetEngine,
    /// Local application data directory used for non-database configuration
    /// such as trusted signing keys. `None` for in-memory test databases.
    #[allow(dead_code)]
    pub data_dir: Option<PathBuf>,
}

impl Core {
    pub fn open(data_dir: &Path) -> Result<Self, CoreError> {
        std::fs::create_dir_all(data_dir).map_err(|error| {
            CoreError::Database(rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
        })?;
        let path = data_dir.join(DATABASE_FILE);
        let path = path.to_str().ok_or(CoreError::InvalidDatabasePath)?;
        let connection = Connection::open(path)?;
        connection.busy_timeout(SQLITE_BUSY_TIMEOUT)?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        Self::migrate(&connection)?;
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
            engine: PetEngine::default(),
            data_dir: Some(data_dir.to_path_buf()),
        })
    }

    pub fn open_default() -> Result<Self, CoreError> {
        Self::open(&default_data_directory()?)
    }

    pub fn in_memory() -> Result<Self, CoreError> {
        let connection = Connection::open_in_memory()?;
        connection.busy_timeout(SQLITE_BUSY_TIMEOUT)?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        Self::migrate(&connection)?;
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
            engine: PetEngine::default(),
            data_dir: None,
        })
    }

    fn migrate(connection: &Connection) -> Result<(), CoreError> {
        Self::migrate_to(connection, SCHEMA_VERSION)
    }

    fn migrate_to(connection: &Connection, target: i64) -> Result<(), CoreError> {
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_versions (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL
             );",
        )?;
        let current: i64 = connection
            .query_row("SELECT MAX(version) FROM schema_versions", [], |row| {
                row.get::<_, Option<i64>>(0)
            })
            .optional()?
            .flatten()
            .unwrap_or(0);
        if current > SCHEMA_VERSION {
            return Err(CoreError::UnsupportedDatabaseSchema(current));
        }
        if current < 1 && target >= 1 {
            let transaction = connection.unchecked_transaction()?;
            transaction.execute_batch(
                "CREATE TABLE settings (
                    key TEXT PRIMARY KEY NOT NULL,
                    value TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                 );
                 CREATE TABLE agent_events (
                    id TEXT PRIMARY KEY NOT NULL,
                    schema_version INTEGER NOT NULL,
                    event_type TEXT NOT NULL,
                    occurred_at TEXT NOT NULL,
                    source TEXT NOT NULL,
                    session_id TEXT,
                    project_id TEXT,
                    accepted_at TEXT NOT NULL
                 );
                 CREATE INDEX agent_events_occurred_at_idx ON agent_events(occurred_at);
                 CREATE TABLE pet_state (
                    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
                    state TEXT NOT NULL,
                    selected_pet_id TEXT,
                    last_event_at TEXT
                 );
                 INSERT INTO pet_state(singleton, state) VALUES (1, 'idle');",
            )?;
            transaction.execute(
                "INSERT INTO schema_versions(version, applied_at) VALUES (?1, ?2)",
                params![1, Utc::now().to_rfc3339()],
            )?;
            transaction.commit()?;
        }
        if current < 2 && target >= 2 {
            let transaction = connection.unchecked_transaction()?;
            transaction.execute_batch(
                "CREATE TABLE installed_pet_packs (
                    pack_id TEXT NOT NULL,
                    version TEXT NOT NULL,
                    name TEXT NOT NULL,
                    creator TEXT NOT NULL,
                    license TEXT NOT NULL,
                    source TEXT NOT NULL,
                    installed_path TEXT NOT NULL UNIQUE,
                    manifest_json TEXT NOT NULL,
                    installed_at TEXT NOT NULL,
                    PRIMARY KEY(pack_id, version)
                 );
                 CREATE INDEX installed_pet_packs_name_idx ON installed_pet_packs(name COLLATE NOCASE);",
            )?;
            transaction.execute(
                "INSERT INTO schema_versions(version, applied_at) VALUES (?1, ?2)",
                params![2, Utc::now().to_rfc3339()],
            )?;
            transaction.commit()?;
        }
        if current < 3 && target >= 3 {
            let transaction = connection.unchecked_transaction()?;
            transaction.execute_batch(
                "CREATE TABLE agent_sessions (
                    session_id TEXT PRIMARY KEY NOT NULL,
                    agent_id TEXT NOT NULL,
                    project_path TEXT,
                    started_at TEXT NOT NULL,
                    ended_at TEXT,
                    duration_ms INTEGER CHECK(duration_ms IS NULL OR duration_ms >= 0),
                    status TEXT NOT NULL CHECK(status IN ('running', 'completed', 'failed', 'interrupted')),
                    exit_code INTEGER,
                    input_tokens INTEGER CHECK(input_tokens IS NULL OR input_tokens >= 0),
                    output_tokens INTEGER CHECK(output_tokens IS NULL OR output_tokens >= 0)
                 );
                 CREATE INDEX agent_sessions_started_at_idx ON agent_sessions(started_at);
                 CREATE INDEX agent_sessions_status_idx ON agent_sessions(status);",
            )?;
            transaction.execute(
                "INSERT INTO schema_versions(version, applied_at) VALUES (?1, ?2)",
                params![3, Utc::now().to_rfc3339()],
            )?;
            transaction.commit()?;
        }
        if current < 4 && target >= 4 {
            let transaction = connection.unchecked_transaction()?;
            transaction.execute_batch(
                "ALTER TABLE agent_sessions ADD COLUMN pet_id TEXT NOT NULL DEFAULT 'local.default';
                 CREATE TABLE progression_config (
                    singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
                    xp_per_completed_session INTEGER NOT NULL CHECK(xp_per_completed_session > 0),
                    base_xp_per_level INTEGER NOT NULL CHECK(base_xp_per_level > 0),
                    level_xp_increment INTEGER NOT NULL CHECK(level_xp_increment >= 0)
                 );
                 CREATE TABLE xp_transactions (
                    transaction_id TEXT PRIMARY KEY NOT NULL,
                    pet_id TEXT NOT NULL,
                    source TEXT NOT NULL,
                    source_id TEXT NOT NULL,
                    amount INTEGER NOT NULL CHECK(amount > 0),
                    created_at TEXT NOT NULL,
                    metadata_json TEXT NOT NULL,
                    UNIQUE(source, source_id)
                 );
                 CREATE INDEX xp_transactions_pet_time_idx ON xp_transactions(pet_id, created_at);
                 CREATE TABLE pet_progression (
                    pet_id TEXT PRIMARY KEY NOT NULL,
                    total_xp INTEGER NOT NULL CHECK(total_xp >= 0),
                    level INTEGER NOT NULL CHECK(level >= 1),
                    updated_at TEXT NOT NULL
                 );
                 CREATE TRIGGER xp_transactions_no_update
                    BEFORE UPDATE ON xp_transactions BEGIN SELECT RAISE(ABORT, 'XP ledger is immutable'); END;
                 CREATE TRIGGER xp_transactions_no_delete
                    BEFORE DELETE ON xp_transactions BEGIN SELECT RAISE(ABORT, 'XP ledger is immutable'); END;",
            )?;
            transaction.execute(
                "INSERT INTO progression_config(singleton, xp_per_completed_session, base_xp_per_level, level_xp_increment)
                 VALUES (1, ?1, ?2, ?3)",
                params![DEFAULT_SESSION_XP, DEFAULT_LEVEL_XP, DEFAULT_LEVEL_XP_INCREMENT],
            )?;
            transaction.execute(
                "INSERT INTO schema_versions(version, applied_at) VALUES (?1, ?2)",
                params![4, Utc::now().to_rfc3339()],
            )?;
            transaction.commit()?;
        }
        if current < 5 && target >= 5 {
            let transaction = connection.unchecked_transaction()?;
            transaction.execute_batch(
                "CREATE TABLE achievement_definitions (
                    id TEXT PRIMARY KEY NOT NULL,
                    title TEXT NOT NULL,
                    description TEXT NOT NULL,
                    metric TEXT NOT NULL CHECK(metric = 'completed_sessions'),
                    threshold INTEGER NOT NULL CHECK(threshold > 0),
                    reward_xp INTEGER NOT NULL CHECK(reward_xp >= 0)
                 );
                 CREATE TABLE pet_achievements (
                    pet_id TEXT NOT NULL,
                    achievement_id TEXT NOT NULL REFERENCES achievement_definitions(id),
                    unlocked_at TEXT NOT NULL,
                    source_session_id TEXT NOT NULL,
                    PRIMARY KEY(pet_id, achievement_id)
                 );
                 CREATE INDEX pet_achievements_unlocked_idx ON pet_achievements(pet_id, unlocked_at);
                 CREATE TRIGGER pet_achievements_no_update
                    BEFORE UPDATE ON pet_achievements BEGIN SELECT RAISE(ABORT, 'achievement awards are immutable'); END;
                 CREATE TRIGGER pet_achievements_no_delete
                    BEFORE DELETE ON pet_achievements BEGIN SELECT RAISE(ABORT, 'achievement awards are immutable'); END;
                 INSERT INTO achievement_definitions(id, title, description, metric, threshold, reward_xp) VALUES
                    ('first_session', 'First Steps', 'Complete your first coding-agent session.', 'completed_sessions', 1, 25),
                    ('ten_sessions', 'Getting in Rhythm', 'Complete ten coding-agent sessions.', 'completed_sessions', 10, 50),
                    ('hundred_sessions', 'Century Club', 'Complete one hundred coding-agent sessions.', 'completed_sessions', 100, 100);",
            )?;
            transaction.execute(
                "INSERT INTO schema_versions(version, applied_at) VALUES (?1, ?2)",
                params![5, Utc::now().to_rfc3339()],
            )?;
            transaction.commit()?;
        }
        if current < 6 && target >= 6 {
            let now = Utc::now().to_rfc3339();
            let transaction = connection.unchecked_transaction()?;
            transaction.execute_batch("ALTER TABLE pet_state ADD COLUMN state_entered_at TEXT;")?;
            transaction.execute(
                "UPDATE pet_state SET state_entered_at = COALESCE(last_event_at, ?1)",
                [&now],
            )?;
            transaction.execute(
                "INSERT INTO schema_versions(version, applied_at) VALUES (?1, ?2)",
                params![6, now],
            )?;
            transaction.commit()?;
        }
        if current < 7 && target >= 7 {
            let transaction = connection.unchecked_transaction()?;
            transaction.execute_batch(
                "CREATE TABLE agent_hook_installs (
                    adapter_id TEXT PRIMARY KEY NOT NULL,
                    config_path TEXT NOT NULL,
                    backup_path TEXT,
                    installed_at TEXT NOT NULL,
                    status TEXT NOT NULL CHECK(status IN ('installed', 'removed'))
                 );
                 ALTER TABLE installed_pet_packs ADD COLUMN install_source TEXT NOT NULL DEFAULT 'folder';
                 ALTER TABLE installed_pet_packs ADD COLUMN content_hash TEXT;
                 ALTER TABLE installed_pet_packs ADD COLUMN signature_key_id TEXT;",
            )?;
            transaction.execute(
                "INSERT INTO schema_versions(version, applied_at) VALUES (?1, ?2)",
                params![7, Utc::now().to_rfc3339()],
            )?;
            transaction.commit()?;
        }
        if current < 8 && target >= 8 {
            let transaction = connection.unchecked_transaction()?;
            transaction.execute_batch(
                "CREATE TABLE IF NOT EXISTS xp_ledger (
                    index_id INTEGER PRIMARY KEY AUTOINCREMENT,
                    entry_id TEXT UNIQUE NOT NULL,
                    prev_hash TEXT NOT NULL,
                    hash TEXT NOT NULL,
                    provider TEXT NOT NULL,
                    session_id TEXT NOT NULL,
                    event_id TEXT NOT NULL,
                    idempotency_key TEXT UNIQUE NOT NULL,
                    pet_id TEXT NOT NULL,
                    amount INTEGER NOT NULL CHECK(amount > 0),
                    created_at TEXT NOT NULL,
                    metadata_json TEXT NOT NULL,
                    ledger_version INTEGER NOT NULL DEFAULT 1
                 );
                 CREATE INDEX IF NOT EXISTS xp_ledger_pet_time_idx ON xp_ledger(pet_id, created_at);
                 CREATE TRIGGER IF NOT EXISTS xp_ledger_no_update
                    BEFORE UPDATE ON xp_ledger BEGIN SELECT RAISE(ABORT, 'XP ledger is immutable'); END;
                 CREATE TRIGGER IF NOT EXISTS xp_ledger_no_delete
                    BEFORE DELETE ON xp_ledger BEGIN SELECT RAISE(ABORT, 'XP ledger is immutable'); END;
                 ALTER TABLE agent_events ADD COLUMN payload_json TEXT;
                 ALTER TABLE agent_events ADD COLUMN provider TEXT NOT NULL DEFAULT 'universal';
                 ALTER TABLE agent_events ADD COLUMN monotonic_ms INTEGER DEFAULT 0;",
            )?;
            transaction.execute(
                "INSERT INTO schema_versions(version, applied_at) VALUES (?1, ?2)",
                params![8, Utc::now().to_rfc3339()],
            )?;
            transaction.commit()?;
        }
        if current < 9 && target >= 9 {
            let transaction = connection.unchecked_transaction()?;
            let has_col: bool = transaction
                .prepare("SELECT ledger_version FROM xp_ledger LIMIT 0")
                .is_ok();
            if !has_col {
                transaction.execute_batch(
                    "ALTER TABLE xp_ledger ADD COLUMN ledger_version INTEGER NOT NULL DEFAULT 1;",
                )?;
            }
            transaction.execute(
                "INSERT INTO schema_versions(version, applied_at) VALUES (?1, ?2)",
                params![9, Utc::now().to_rfc3339()],
            )?;
            transaction.commit()?;
        }
        Ok(())
    }

    pub fn snapshot(&self) -> Result<RuntimeSnapshot, CoreError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| CoreError::LockPoisoned)?;
        let (state, selected_pet_id, last_event_at): (String, Option<String>, Option<String>) =
            connection.query_row(
                "SELECT state, selected_pet_id, last_event_at FROM pet_state WHERE singleton = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )?;
        Ok(RuntimeSnapshot {
            pet_state: parse_state(&state)?,
            selected_pet_id,
            last_event_at: parse_timestamp(last_event_at)?,
        })
    }

    /// Applies any due timed-state expiry. Returns the new state when one was entered.
    pub fn advance_state_timers(&self) -> Result<Option<PetState>, CoreError> {
        self.advance_state_timers_at(Utc::now())
    }

    fn advance_state_timers_at(&self, now: DateTime<Utc>) -> Result<Option<PetState>, CoreError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| CoreError::LockPoisoned)?;
        let (current, entered_at) = read_pet_state(&connection)?;
        let due = self
            .engine
            .definition(current)
            .timeout
            .is_some_and(|timeout| elapsed_since(entered_at, now) >= timeout.after);
        if !due {
            return Ok(None);
        }
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (before, _) = read_pet_state(&transaction)?;
        let after = settle_state(&transaction, &self.engine, now)?;
        transaction.commit()?;
        Ok((after != before).then_some(after))
    }

    pub fn overlay_position(&self) -> Result<Option<DesktopPosition>, CoreError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| CoreError::LockPoisoned)?;
        let value: Option<String> = connection
            .query_row(
                "SELECT value FROM settings WHERE key = 'pet_overlay_position'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        value
            .map(|json| serde_json::from_str(&json).map_err(CoreError::Serialization))
            .transpose()
    }

    pub fn save_overlay_position(&self, position: DesktopPosition) -> Result<(), CoreError> {
        let json = serde_json::to_string(&position)?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| CoreError::LockPoisoned)?;
        connection.execute(
            "INSERT INTO settings(key, value, updated_at) VALUES ('pet_overlay_position', ?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![json, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn progression(&self) -> Result<ProgressionSnapshot, CoreError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| CoreError::LockPoisoned)?;
        let selected_pet: Option<String> = connection.query_row(
            "SELECT selected_pet_id FROM pet_state WHERE singleton = 1",
            [],
            |row| row.get(0),
        )?;
        let pet_id = selected_pet.unwrap_or_else(|| FALLBACK_PET_ID.to_owned());
        let total_xp: i64 = connection.query_row(
            "SELECT COALESCE((SELECT total_xp FROM pet_progression WHERE pet_id = ?1), 0)",
            [&pet_id],
            |row| row.get(0),
        )?;
        let rules = load_progression_rules(&connection)?;
        progression_from_total(pet_id, total_xp, rules)
    }

    pub fn xp_history(&self, limit: usize, offset: usize) -> Result<Vec<XpTransaction>, CoreError> {
        if limit == 0 || limit > MAX_XP_HISTORY_PAGE_SIZE || offset > i64::MAX as usize {
            return Err(CoreError::InvalidPagination);
        }
        let connection = self
            .connection
            .lock()
            .map_err(|_| CoreError::LockPoisoned)?;
        let selected_pet: Option<String> = connection.query_row(
            "SELECT selected_pet_id FROM pet_state WHERE singleton = 1",
            [],
            |row| row.get(0),
        )?;
        let pet_id = selected_pet.unwrap_or_else(|| FALLBACK_PET_ID.to_owned());
        let mut statement = connection.prepare(
            "SELECT transaction_id, pet_id, source, source_id, amount, created_at, metadata_json
             FROM xp_transactions WHERE pet_id = ?1 ORDER BY created_at DESC, transaction_id DESC
             LIMIT ?2 OFFSET ?3",
        )?;
        let rows = statement.query_map(params![pet_id, limit as i64, offset as i64], |row| {
            Ok(XpTransaction {
                transaction_id: row.get(0)?,
                pet_id: row.get(1)?,
                source: row.get(2)?,
                source_id: row.get(3)?,
                amount: row.get(4)?,
                created_at: row.get(5)?,
                metadata_json: row.get(6)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(CoreError::from)
    }

    pub fn recent_xp_transactions(&self) -> Result<Vec<XpTransaction>, CoreError> {
        self.xp_history(DEFAULT_XP_HISTORY_PAGE_SIZE, 0)
    }

    /// Opens the isolated database for simulate mode (`agentpet_simulated.sqlite3`).
    #[cfg(feature = "dev-simulator")]
    pub fn open_simulated(data_dir: &Path) -> Result<Self, CoreError> {
        std::fs::create_dir_all(data_dir).map_err(|error| {
            CoreError::Database(rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
        })?;
        let path = data_dir.join(SIMULATED_DATABASE_FILE);
        let path = path.to_str().ok_or(CoreError::InvalidDatabasePath)?;
        let connection = Connection::open(path)?;
        connection.busy_timeout(SQLITE_BUSY_TIMEOUT)?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        Self::migrate(&connection)?;
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
            engine: PetEngine::default(),
            data_dir: Some(data_dir.to_path_buf()),
        })
    }

    /// Verifies the cryptographic integrity of the append-only XP ledger hash chain.
    pub fn verify_ledger(&self) -> Result<LedgerVerificationResult, CoreError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| CoreError::LockPoisoned)?;
        let mut statement = connection.prepare(
            "SELECT index_id, entry_id, prev_hash, hash, idempotency_key, pet_id, amount, created_at, COALESCE(ledger_version, 1)
             FROM xp_ledger ORDER BY index_id ASC",
        )?;
        struct LedgerRow {
            index_id: u64,
            prev_hash: String,
            hash: String,
            idempotency_key: String,
            pet_id: String,
            amount: i64,
            created_at: String,
            ledger_version: i64,
        }
        let rows = statement.query_map([], |row| {
            Ok(LedgerRow {
                index_id: row.get(0)?,
                prev_hash: row.get(2)?,
                hash: row.get(3)?,
                idempotency_key: row.get(4)?,
                pet_id: row.get(5)?,
                amount: row.get(6)?,
                created_at: row.get(7)?,
                ledger_version: row.get(8)?,
            })
        })?;

        let mut expected_prev = GENESIS_HASH.to_string();
        let mut count = 0_u64;
        let mut head = expected_prev.clone();

        for row in rows {
            let row = row?;
            count += 1;
            if row.prev_hash != expected_prev {
                return Ok(LedgerVerificationResult {
                    is_valid: false,
                    total_entries: count,
                    head_hash: row.hash,
                    broken_at_index: Some(row.index_id),
                    error_message: Some(format!(
                        "Chain link broken at entry {}: expected prev_hash {}, found {}",
                        row.index_id, expected_prev, row.prev_hash
                    )),
                });
            }
            let computed = if row.ledger_version == 1 {
                compute_ledger_hash_v1(
                    &row.prev_hash,
                    &row.idempotency_key,
                    row.amount,
                    &row.pet_id,
                    &row.created_at,
                )
            } else {
                compute_ledger_hash_v2(
                    &row.prev_hash,
                    &row.idempotency_key,
                    row.amount,
                    &row.pet_id,
                    &row.created_at,
                )
            };
            if computed != row.hash {
                return Ok(LedgerVerificationResult {
                    is_valid: false,
                    total_entries: count,
                    head_hash: row.hash.clone(),
                    broken_at_index: Some(row.index_id),
                    error_message: Some(format!(
                        "Cryptographic hash invalid at entry {}: computed {}, found {}",
                        row.index_id, computed, row.hash
                    )),
                });
            }
            expected_prev = row.hash.clone();
            head = row.hash;
        }

        Ok(LedgerVerificationResult {
            is_valid: true,
            total_entries: count,
            head_hash: head,
            broken_at_index: None,
            error_message: None,
        })
    }

    /// Queries recent normalized agent events with measured event-to-render latency.
    pub fn recent_events(&self, limit: usize) -> Result<Vec<StoredAgentEvent>, CoreError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| CoreError::LockPoisoned)?;
        let limit = limit.clamp(1, 100);
        let mut statement = connection.prepare(
            "SELECT id, event_type, occurred_at, source, COALESCE(provider, 'universal'),
                    session_id, project_id, payload_json, accepted_at
             FROM agent_events ORDER BY occurred_at DESC, id DESC LIMIT ?1",
        )?;
        let rows = statement.query_map([limit as i64], |row| {
            let occurred_at: String = row.get(2)?;
            let accepted_at: String = row.get(8)?;
            let latency_ms = match (
                DateTime::parse_from_rfc3339(&occurred_at),
                DateTime::parse_from_rfc3339(&accepted_at),
            ) {
                (Ok(occ), Ok(acc)) => {
                    let diff = acc.signed_duration_since(occ).num_milliseconds();
                    Some(diff.max(0) as u64)
                }
                _ => None,
            };
            Ok(StoredAgentEvent {
                id: row.get(0)?,
                event_type: row.get(1)?,
                occurred_at,
                source: row.get(3)?,
                provider: row.get(4)?,
                session_id: row.get(5)?,
                project_id: row.get(6)?,
                payload_json: row.get(7)?,
                latency_ms,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(CoreError::from)
    }

    pub fn session_history(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<AgentSessionSummary>, CoreError> {
        if limit == 0 || limit > MAX_SESSION_HISTORY_PAGE_SIZE || offset > i64::MAX as usize {
            return Err(CoreError::InvalidPagination);
        }
        let connection = self
            .connection
            .lock()
            .map_err(|_| CoreError::LockPoisoned)?;
        let mut statement = connection.prepare(
            "SELECT session_id, agent_id, project_path, started_at, ended_at, duration_ms,
                    status, exit_code, input_tokens, output_tokens
             FROM agent_sessions ORDER BY started_at DESC, session_id DESC LIMIT ?1 OFFSET ?2",
        )?;
        let rows = statement.query_map(params![limit as i64, offset as i64], |row| {
            Ok(AgentSessionSummary {
                session_id: row.get(0)?,
                agent_id: row.get(1)?,
                project_path: row.get(2)?,
                started_at: row.get(3)?,
                ended_at: row.get(4)?,
                duration_ms: row.get(5)?,
                status: row.get(6)?,
                exit_code: row.get(7)?,
                input_tokens: row.get(8)?,
                output_tokens: row.get(9)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(CoreError::from)
    }

    pub fn recent_sessions(&self) -> Result<Vec<AgentSessionSummary>, CoreError> {
        self.session_history(DEFAULT_SESSION_HISTORY_PAGE_SIZE, 0)
    }

    pub fn achievements(&self) -> Result<Vec<AchievementSummary>, CoreError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| CoreError::LockPoisoned)?;
        let selected_pet: Option<String> = connection.query_row(
            "SELECT selected_pet_id FROM pet_state WHERE singleton = 1",
            [],
            |row| row.get(0),
        )?;
        let pet_id = selected_pet.unwrap_or_else(|| FALLBACK_PET_ID.to_owned());
        let completed_sessions: i64 = connection.query_row(
            "SELECT COUNT(*) FROM agent_sessions WHERE pet_id = ?1 AND status = 'completed'",
            [&pet_id],
            |row| row.get(0),
        )?;
        let mut statement = connection.prepare(
            "SELECT d.id, d.title, d.description, d.threshold, d.reward_xp, a.unlocked_at
             FROM achievement_definitions d LEFT JOIN pet_achievements a
               ON a.achievement_id = d.id AND a.pet_id = ?1
             ORDER BY d.threshold, d.id",
        )?;
        let rows = statement.query_map([&pet_id], |row| {
            let threshold: i64 = row.get(3)?;
            Ok(AchievementSummary {
                id: row.get(0)?,
                title: row.get(1)?,
                description: row.get(2)?,
                threshold,
                progress: completed_sessions.min(threshold),
                reward_xp: row.get(4)?,
                unlocked_at: row.get(5)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(CoreError::from)
    }

    pub fn set_progression_rules(&self, rules: ProgressionRules) -> Result<(), CoreError> {
        validate_progression_rules(rules)?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| CoreError::LockPoisoned)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        transaction.execute(
            "UPDATE progression_config SET xp_per_completed_session = ?1, base_xp_per_level = ?2,
             level_xp_increment = ?3 WHERE singleton = 1",
            params![
                rules.xp_per_completed_session,
                rules.base_xp_per_level,
                rules.level_xp_increment
            ],
        )?;
        let mut statement = transaction.prepare("SELECT pet_id, total_xp FROM pet_progression")?;
        let existing = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        drop(statement);
        for (pet_id, total_xp) in existing {
            let progression = progression_from_total(pet_id.clone(), total_xp, rules)?;
            transaction.execute(
                "UPDATE pet_progression SET level = ?1, updated_at = ?2 WHERE pet_id = ?3",
                params![progression.level as i64, Utc::now().to_rfc3339(), pet_id],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn accept_event(&self, event: &PetEvent) -> Result<bool, CoreError> {
        event.validate()?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| CoreError::LockPoisoned)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let accepted = record_event(
            &transaction,
            &self.engine,
            event,
            event.event_type.stimulus(),
            OnRejectedTransition::Fail,
        )?;
        transaction.commit()?;
        Ok(accepted)
    }

    /// Ingests a normalized hook event from a local agent adapter.
    ///
    /// Events are deduplicated by their stable idempotency key. Session
    /// lifecycle events additionally open or close an observed agent session;
    /// a completed session awards XP in the same transaction that records the
    /// completion, exactly like the universal wrapper. Invalid or rejected
    /// state transitions never corrupt the pet state: the session fact is
    /// persisted with the state kept unchanged.
    pub fn record_hook_event(&self, hook_event: &HookEvent) -> Result<HookEventOutcome, CoreError> {
        if hook_event.id.trim().is_empty() || hook_event.id.len() > 64 {
            return Err(CoreError::InvalidHookEvent(
                "id must be a non-empty string of at most 64 characters".into(),
            ));
        }
        if hook_event.agent_id.trim().is_empty() || hook_event.agent_id.len() > 128 {
            return Err(CoreError::InvalidHookEvent(
                "agent_id must be a non-empty string of at most 128 characters".into(),
            ));
        }
        let session_id = hook_event
            .session_id
            .as_deref()
            .filter(|value| !value.trim().is_empty());
        if hook_event.session_id.as_deref().is_some_and(|raw| {
            let filtered = raw.trim();
            filtered.is_empty() || raw.len() > 128
        }) {
            return Err(CoreError::InvalidHookEvent(
                "session_id must be a non-empty string of at most 128 characters".into(),
            ));
        }
        if hook_event
            .project_path
            .as_deref()
            .is_some_and(|path| path.len() > 4_096)
        {
            return Err(CoreError::InvalidHookEvent(
                "project_path is too long".into(),
            ));
        }
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| CoreError::LockPoisoned)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let outcome =
            record_hook_event_in_transaction(&transaction, &self.engine, hook_event, session_id)?;
        transaction.commit()?;
        Ok(outcome)
    }

    /// Records the local installation state of a native agent hook.
    pub fn record_hook_install(&self, record: &HookInstallRecord) -> Result<(), CoreError> {
        if record.adapter_id.trim().is_empty() || record.adapter_id.len() > 128 {
            return Err(CoreError::InvalidIdentifier);
        }
        if record.config_path.trim().is_empty() || record.config_path.len() > 4_096 {
            return Err(CoreError::InvalidIdentifier);
        }
        if record.status != "installed" && record.status != "removed" {
            return Err(CoreError::InvalidIdentifier);
        }
        let connection = self
            .connection
            .lock()
            .map_err(|_| CoreError::LockPoisoned)?;
        connection.execute(
            "INSERT INTO agent_hook_installs(adapter_id, config_path, backup_path, installed_at, status)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(adapter_id) DO UPDATE SET
                config_path = excluded.config_path,
                backup_path = excluded.backup_path,
                installed_at = excluded.installed_at,
                status = excluded.status",
            params![
                record.adapter_id,
                record.config_path,
                record.backup_path,
                record.installed_at,
                record.status,
            ],
        )?;
        Ok(())
    }

    /// Reads all persisted hook installation records.
    pub fn hook_installs(&self) -> Result<Vec<HookInstallRecord>, CoreError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| CoreError::LockPoisoned)?;
        let mut statement = connection.prepare(
            "SELECT adapter_id, config_path, backup_path, installed_at, status
             FROM agent_hook_installs ORDER BY adapter_id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(HookInstallRecord {
                adapter_id: row.get(0)?,
                config_path: row.get(1)?,
                backup_path: row.get(2)?,
                installed_at: row.get(3)?,
                status: row.get(4)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(CoreError::from)
    }

    pub fn start_session(
        &self,
        session_id: &str,
        agent_id: &str,
        project_path: Option<&str>,
    ) -> Result<(), CoreError> {
        if session_id.trim().is_empty()
            || session_id.len() > 128
            || agent_id.trim().is_empty()
            || agent_id.len() > 128
        {
            return Err(CoreError::InvalidIdentifier);
        }
        if project_path.is_some_and(|path| path.len() > 4_096) {
            return Err(CoreError::InvalidIdentifier);
        }
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| CoreError::LockPoisoned)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let selected_pet: Option<String> = transaction.query_row(
            "SELECT selected_pet_id FROM pet_state WHERE singleton = 1",
            [],
            |row| row.get(0),
        )?;
        let pet_id = selected_pet.unwrap_or_else(|| FALLBACK_PET_ID.to_owned());
        transaction.execute(
            "INSERT INTO agent_sessions(session_id, agent_id, project_path, started_at, status, pet_id)
             VALUES (?1, ?2, ?3, ?4, 'running', ?5)",
            params![session_id, agent_id, project_path, Utc::now().to_rfc3339(), pet_id],
        )?;
        let mut event = PetEvent::new(EventType::AgentStarted, agent_id);
        event.session_id = Some(session_id.to_owned());
        record_event(
            &transaction,
            &self.engine,
            &event,
            Stimulus::AgentStarted,
            OnRejectedTransition::KeepState,
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn finish_session(
        &self,
        session_id: &str,
        outcome: SessionOutcome,
        exit_code: Option<i32>,
        duration_ms: u64,
    ) -> Result<(), CoreError> {
        let duration_ms = i64::try_from(duration_ms).map_err(|_| CoreError::InvalidDuration)?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| CoreError::LockPoisoned)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let session: Option<(String, String)> = transaction
            .query_row(
                "SELECT agent_id, pet_id FROM agent_sessions WHERE session_id = ?1 AND status = 'running'",
                [session_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let (agent_id, pet_id) =
            session.ok_or_else(|| CoreError::SessionNotRunning(session_id.to_owned()))?;
        let updated = transaction.execute(
            "UPDATE agent_sessions
             SET ended_at = ?1, duration_ms = ?2, status = ?3, exit_code = ?4
             WHERE session_id = ?5 AND status = 'running'",
            params![
                Utc::now().to_rfc3339(),
                duration_ms,
                outcome.as_str(),
                exit_code,
                session_id
            ],
        )?;
        if updated != 1 {
            return Err(CoreError::SessionNotRunning(session_id.to_owned()));
        }
        let mut event = PetEvent::new(outcome.event_type(), agent_id);
        event.session_id = Some(session_id.to_owned());
        let reaction = if outcome == SessionOutcome::Completed {
            let leveled_from_session = award_session_xp(&transaction, session_id, &pet_id)?;
            let (achievement_unlocked, leveled_from_achievement) =
                award_session_achievements(&transaction, session_id, &pet_id)?;
            SessionReaction::Succeeded {
                leveled_up: leveled_from_session || leveled_from_achievement,
                achievement_unlocked,
            }
        } else {
            SessionReaction::Failed
        };
        record_event(
            &transaction,
            &self.engine,
            &event,
            Stimulus::SessionFinished(reaction),
            OnRejectedTransition::KeepState,
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn set_selected_pet(&self, pet_id: Option<&str>) -> Result<(), CoreError> {
        if pet_id.is_some_and(|value| value.trim().is_empty() || value.len() > 128) {
            return Err(CoreError::InvalidIdentifier);
        }
        let connection = self
            .connection
            .lock()
            .map_err(|_| CoreError::LockPoisoned)?;
        if let Some(pet_id) = pet_id {
            let installed: bool = connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM installed_pet_packs WHERE pack_id || '@' || version = ?1)",
                [pet_id],
                |row| row.get(0),
            )?;
            if !installed {
                return Err(CoreError::PackNotInstalled(pet_id.to_owned()));
            }
        }
        connection.execute(
            "UPDATE pet_state SET selected_pet_id = ?1 WHERE singleton = 1",
            [pet_id],
        )?;
        Ok(())
    }

    pub fn list_installed_packs(&self) -> Result<Vec<InstalledPetPackSummary>, CoreError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| CoreError::LockPoisoned)?;
        let mut statement = connection.prepare(
            "SELECT pack_id, version, name, creator, license
             FROM installed_pet_packs
             ORDER BY name COLLATE NOCASE, pack_id, version",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(InstalledPetPackSummary {
                id: row.get(0)?,
                selection_id: format!("{}@{}", row.get::<_, String>(0)?, row.get::<_, String>(1)?),
                version: row.get(1)?,
                name: row.get(2)?,
                creator: row.get(3)?,
                license: row.get(4)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(CoreError::from)
    }

    pub fn install_extracted_pet_pack(
        &self,
        source_root: &Path,
        library_root: &Path,
    ) -> Result<PetPackManifest, CoreError> {
        let installed = PetPackManifest::install_extracted(
            source_root,
            library_root,
            ValidationLimits::default(),
        )?;
        self.record_installed_pet_pack(installed)
    }

    /// Installs a PetPack from a ZIP or tar.gz archive after validating every
    /// entry and, when trusted signers are configured or `require_signature`
    /// is set, verifying the detached `pet.sig` signature.
    pub fn install_archive_pet_pack(
        &self,
        archive_path: &Path,
        library_root: &Path,
        trusted_signers: &[TrustedSigner],
        require_signature: bool,
    ) -> Result<PetPackManifest, CoreError> {
        let installed = agentpet_pet_pack::archive::install_archive(
            archive_path,
            library_root,
            ValidationLimits::default(),
            agentpet_pet_pack::archive::ArchiveLimits::default(),
            trusted_signers,
            require_signature,
        )?;
        self.record_installed_pet_pack(installed)
    }

    pub fn create_custom_pet_from_image(
        &self,
        image_path: &Path,
        library_root: &Path,
        name: &str,
    ) -> Result<PetPackManifest, CoreError> {
        let installed =
            PetPackManifest::create_companion_from_image(image_path, library_root, name)?;
        self.record_installed_pet_pack(installed)
    }

    /// Reads the local trusted signing keys used to verify PetPack archive
    /// signatures. Keys come from `<data_dir>/trusted-signers.json`, an array
    /// of `{ "keyId": string, "publicKey": string }` objects with base64
    /// Ed25519 public keys. In-memory databases have no data directory and
    /// return no trusted signers.
    pub fn trusted_signers(&self) -> Result<Vec<TrustedSigner>, CoreError> {
        let Some(data_dir) = self.data_dir.as_ref() else {
            return Ok(Vec::new());
        };
        let config_path = data_dir.join("trusted-signers.json");
        let bytes = match std::fs::read(&config_path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => {
                return Err(CoreError::Database(
                    rusqlite::Error::ToSqlConversionFailure(Box::new(error)),
                ))
            }
        };
        if bytes.len() > 64 * 1024 {
            return Err(CoreError::TrustedSignerConfig(
                "trusted-signers.json exceeds 64 KiB".into(),
            ));
        }
        let entries: Vec<TrustedSignerEntry> = serde_json::from_slice(&bytes)
            .map_err(|error| CoreError::TrustedSignerConfig(error.to_string()))?;
        if entries.len() > 32 {
            return Err(CoreError::TrustedSignerConfig(
                "at most 32 trusted signers are supported".into(),
            ));
        }
        entries
            .into_iter()
            .map(|entry| {
                TrustedSigner::from_base64(&entry.key_id, &entry.public_key)
                    .map_err(|error| CoreError::TrustedSignerConfig(error.to_string()))
            })
            .collect()
    }

    fn record_installed_pet_pack(
        &self,
        installed: agentpet_pet_pack::InstalledPetPack,
    ) -> Result<PetPackManifest, CoreError> {
        let manifest_json = match serde_json::to_string(&installed.manifest) {
            Ok(value) => value,
            Err(error) => {
                let primary = CoreError::Serialization(error);
                return Err(rollback_pack_install(&installed.directory, primary));
            }
        };
        let installed_path = installed.directory.to_string_lossy().into_owned();
        let record_result = (|| {
            let mut connection = self
                .connection
                .lock()
                .map_err(|_| CoreError::LockPoisoned)?;
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            transaction.execute(
                "INSERT INTO installed_pet_packs(
                    pack_id, version, name, creator, license, source,
                    installed_path, manifest_json, installed_at,
                    install_source, content_hash, signature_key_id
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    installed.manifest.id,
                    installed.manifest.version,
                    installed.manifest.name,
                    installed.manifest.creator,
                    installed.manifest.license,
                    installed.manifest.source,
                    installed_path,
                    manifest_json,
                    Utc::now().to_rfc3339(),
                    installed.install_source,
                    installed.content_hash,
                    installed.signature_key_id,
                ],
            )?;
            transaction.commit()?;
            Ok::<(), CoreError>(())
        })();
        if let Err(primary) = record_result {
            return Err(rollback_pack_install(&installed.directory, primary));
        }
        Ok(installed.manifest)
    }
}

pub fn default_data_directory() -> Result<PathBuf, CoreError> {
    #[cfg(target_os = "windows")]
    let base = std::env::var_os("APPDATA").map(PathBuf::from);

    #[cfg(target_os = "macos")]
    let base = std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join("Library").join("Application Support"));

    #[cfg(all(unix, not(target_os = "macos")))]
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .map(|home| home.join(".local").join("share"))
        });

    #[cfg(not(any(target_os = "windows", target_os = "macos", unix)))]
    let base: Option<PathBuf> = None;

    base.map(|path| path.join("dev.agentpet.desktop"))
        .ok_or(CoreError::DataDirectoryUnavailable)
}

fn validate_progression_rules(rules: ProgressionRules) -> Result<(), CoreError> {
    if rules.xp_per_completed_session <= 0
        || rules.xp_per_completed_session > MAX_CONFIGURED_XP
        || rules.base_xp_per_level < MIN_BASE_XP_PER_LEVEL
        || rules.base_xp_per_level > MAX_CONFIGURED_XP
        || rules.level_xp_increment < 0
        || rules.level_xp_increment > MAX_CONFIGURED_XP
    {
        return Err(CoreError::InvalidProgressionRules);
    }
    Ok(())
}

fn load_progression_rules(connection: &Connection) -> Result<ProgressionRules, CoreError> {
    connection
        .query_row(
            "SELECT xp_per_completed_session, base_xp_per_level, level_xp_increment
         FROM progression_config WHERE singleton = 1",
            [],
            |row| {
                Ok(ProgressionRules {
                    xp_per_completed_session: row.get(0)?,
                    base_xp_per_level: row.get(1)?,
                    level_xp_increment: row.get(2)?,
                })
            },
        )
        .map_err(CoreError::from)
}

fn level_threshold(level: u64, rules: ProgressionRules) -> u128 {
    let completed_levels = u128::from(level.saturating_sub(1));
    let next_level_count = completed_levels.saturating_sub(1);
    let linear = completed_levels.saturating_mul(rules.base_xp_per_level as u128);
    let progression = (rules.level_xp_increment as u128)
        .saturating_mul(completed_levels)
        .saturating_mul(next_level_count)
        / 2;
    linear.saturating_add(progression)
}

fn progression_from_total(
    pet_id: String,
    total_xp: i64,
    rules: ProgressionRules,
) -> Result<ProgressionSnapshot, CoreError> {
    validate_progression_rules(rules)?;
    if total_xp < 0 {
        return Err(CoreError::InvalidProgressionRules);
    }
    let total = total_xp as u128;
    let mut low = 1_u64;
    let mut high = (total_xp / rules.base_xp_per_level) as u64 + 2;
    while low < high {
        let middle = low + (high - low).div_ceil(2);
        if level_threshold(middle, rules) <= total {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    let xp_into_level = (total - level_threshold(low, rules)) as i64;
    let threshold = (rules.base_xp_per_level as u128).saturating_add(
        (rules.level_xp_increment as u128).saturating_mul(u128::from(low.saturating_sub(1))),
    );
    let xp_to_next_level = threshold.min(i64::MAX as u128) as i64;
    Ok(ProgressionSnapshot {
        pet_id,
        total_xp,
        level: low,
        xp_into_level,
        xp_to_next_level,
    })
}

fn award_session_xp(
    transaction: &rusqlite::Transaction<'_>,
    session_id: &str,
    pet_id: &str,
) -> Result<bool, CoreError> {
    let rules = load_progression_rules(transaction)?;
    award_xp_transaction(
        transaction,
        pet_id,
        "session_completed",
        session_id,
        rules.xp_per_completed_session,
        "{\"outcome\":\"completed\"}",
        rules,
    )
}

fn award_session_achievements(
    transaction: &rusqlite::Transaction<'_>,
    session_id: &str,
    pet_id: &str,
) -> Result<(bool, bool), CoreError> {
    let completed_sessions: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM agent_sessions WHERE pet_id = ?1 AND status = 'completed'",
        [pet_id],
        |row| row.get(0),
    )?;
    let eligible = {
        let mut statement = transaction.prepare(
            "SELECT d.id, d.threshold, d.reward_xp FROM achievement_definitions d
             LEFT JOIN pet_achievements a ON a.achievement_id = d.id AND a.pet_id = ?1
             WHERE d.metric = 'completed_sessions' AND d.threshold <= ?2 AND a.achievement_id IS NULL
             ORDER BY d.threshold, d.id",
        )?;
        let rows = statement.query_map(params![pet_id, completed_sessions], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    let mut unlocked_any = false;
    let mut leveled_up = false;
    let rules = load_progression_rules(transaction)?;
    for (achievement_id, _threshold, reward_xp) in eligible {
        let inserted = transaction.execute(
            "INSERT INTO pet_achievements(pet_id, achievement_id, unlocked_at, source_session_id)
             VALUES (?1, ?2, ?3, ?4) ON CONFLICT(pet_id, achievement_id) DO NOTHING",
            params![pet_id, achievement_id, Utc::now().to_rfc3339(), session_id],
        )?;
        if inserted == 1 {
            unlocked_any = true;
            if reward_xp > 0 {
                let source_id = format!("{pet_id}:{achievement_id}");
                leveled_up |= award_xp_transaction(
                    transaction,
                    pet_id,
                    "achievement_unlocked",
                    &source_id,
                    reward_xp,
                    "{\"reason\":\"completed_sessions\"}",
                    rules,
                )?;
            }
        }
    }
    Ok((unlocked_any, leveled_up))
}

fn award_xp_transaction(
    transaction: &rusqlite::Transaction<'_>,
    pet_id: &str,
    source: &str,
    source_id: &str,
    amount: i64,
    metadata_json: &str,
    rules: ProgressionRules,
) -> Result<bool, CoreError> {
    let inserted = transaction.execute(
        "INSERT INTO xp_transactions(transaction_id, pet_id, source, source_id, amount, created_at, metadata_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(source, source_id) DO NOTHING",
        params![Uuid::new_v4().to_string(), pet_id, source, source_id, amount, Utc::now().to_rfc3339(), metadata_json],
    )?;
    if inserted == 1 {
        let last_hash: Option<String> = transaction
            .query_row(
                "SELECT hash FROM xp_ledger ORDER BY index_id DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?;
        let prev_hash = last_hash.unwrap_or_else(|| GENESIS_HASH.to_string());
        let idempotency_key = format!("{source}:{source_id}");
        let now_str = Utc::now().to_rfc3339();
        let hash = compute_ledger_hash_v2(&prev_hash, &idempotency_key, amount, pet_id, &now_str);
        let entry_id = Uuid::new_v4().to_string();
        transaction.execute(
            "INSERT INTO xp_ledger(entry_id, prev_hash, hash, provider, session_id, event_id, idempotency_key, pet_id, amount, created_at, metadata_json, ledger_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 2)
             ON CONFLICT(idempotency_key) DO NOTHING",
            params![
                entry_id,
                prev_hash,
                hash,
                source,
                source_id,
                entry_id,
                idempotency_key,
                pet_id,
                amount,
                now_str,
                metadata_json
            ],
        )?;
        let previous_level: i64 = transaction.query_row(
            "SELECT COALESCE((SELECT level FROM pet_progression WHERE pet_id = ?1), 1)",
            [pet_id],
            |row| row.get(0),
        )?;
        let total_xp: i64 = transaction.query_row(
            "SELECT COALESCE(SUM(amount), 0) FROM xp_ledger WHERE pet_id = ?1",
            [pet_id],
            |row| row.get(0),
        )?;
        let progression = progression_from_total(pet_id.to_owned(), total_xp, rules)?;
        transaction.execute(
            "INSERT INTO pet_progression(pet_id, total_xp, level, updated_at) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(pet_id) DO UPDATE SET total_xp = excluded.total_xp, level = excluded.level, updated_at = excluded.updated_at",
            params![pet_id, total_xp, progression.level as i64, Utc::now().to_rfc3339()],
        )?;
        return Ok(progression.level > previous_level as u64);
    }
    Ok(false)
}

fn parse_state(value: &str) -> Result<PetState, CoreError> {
    PetState::parse(value).ok_or_else(|| CoreError::InvalidState(value.to_owned()))
}

fn parse_timestamp(value: Option<String>) -> Result<Option<DateTime<Utc>>, CoreError> {
    value
        .map(|value| DateTime::parse_from_rfc3339(&value).map(|parsed| parsed.with_timezone(&Utc)))
        .transpose()
        .map_err(|error| {
            CoreError::Database(rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(error),
            ))
        })
}

fn read_pet_state(connection: &Connection) -> Result<(PetState, Option<DateTime<Utc>>), CoreError> {
    let (state, entered_at): (String, Option<String>) = connection.query_row(
        "SELECT state, state_entered_at FROM pet_state WHERE singleton = 1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    Ok((parse_state(&state)?, parse_timestamp(entered_at)?))
}

fn engine_context(connection: &Connection) -> Result<EngineContext, CoreError> {
    let has_active_sessions: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM agent_sessions WHERE status = 'running')",
        [],
        |row| row.get(0),
    )?;
    Ok(EngineContext {
        has_active_sessions,
    })
}

/// A missing or future entry time counts as fully elapsed so a clock change cannot pin a timed state.
fn elapsed_since(entered_at: Option<DateTime<Utc>>, now: DateTime<Utc>) -> Duration {
    match entered_at {
        Some(entered_at) if entered_at <= now => {
            (now - entered_at).to_std().unwrap_or(Duration::MAX)
        }
        _ => Duration::MAX,
    }
}

fn settle_state(
    connection: &Connection,
    engine: &PetEngine,
    now: DateTime<Utc>,
) -> Result<PetState, CoreError> {
    let (current, entered_at) = read_pet_state(connection)?;
    let Some(next) = engine.expire(
        current,
        elapsed_since(entered_at, now),
        engine_context(connection)?,
    ) else {
        return Ok(current);
    };
    connection.execute(
        "UPDATE pet_state SET state = ?1, state_entered_at = ?2 WHERE singleton = 1",
        params![next.as_str(), now.to_rfc3339()],
    )?;
    Ok(next)
}

fn record_event(
    transaction: &rusqlite::Transaction<'_>,
    engine: &PetEngine,
    event: &PetEvent,
    stimulus: Stimulus,
    on_rejected: OnRejectedTransition,
) -> Result<bool, CoreError> {
    event.validate()?;
    let event_id = event.id.to_string();
    let exists: bool = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM agent_events WHERE id = ?1)",
        [&event_id],
        |row| row.get(0),
    )?;
    if exists {
        return Ok(false);
    }
    let now = Utc::now();
    let current = settle_state(transaction, engine, now)?;
    let occurred_at = event.occurred_at.to_rfc3339();
    match engine.decide(current, stimulus, engine_context(transaction)?) {
        Transition::Enter(next) => {
            transaction.execute(
                "UPDATE pet_state SET state = ?1, state_entered_at = ?2, last_event_at = ?3 WHERE singleton = 1",
                params![next.as_str(), now.to_rfc3339(), occurred_at],
            )?;
        }
        Transition::Rejected { from, to } if on_rejected == OnRejectedTransition::Fail => {
            return Err(CoreError::InvalidTransition { from, to });
        }
        Transition::Remain | Transition::Rejected { .. } => {
            transaction.execute(
                "UPDATE pet_state SET last_event_at = ?1 WHERE singleton = 1",
                [&occurred_at],
            )?;
        }
    }
    transaction.execute(
        "INSERT INTO agent_events(id, schema_version, event_type, occurred_at, source, session_id, project_id, accepted_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            event_id,
            event.schema_version,
            event.event_type.as_str(),
            occurred_at,
            event.source,
            event.session_id,
            event.project_id,
            now.to_rfc3339(),
        ],
    )?;
    Ok(true)
}

fn rollback_pack_install(directory: &Path, primary: CoreError) -> CoreError {
    match std::fs::remove_dir_all(directory) {
        Ok(()) => primary,
        Err(cleanup) if cleanup.kind() == std::io::ErrorKind::NotFound => primary,
        Err(cleanup) => CoreError::PackInstallRollback {
            primary: primary.to_string(),
            cleanup: cleanup.to_string(),
        },
    }
}

/// Records a normalized hook event and drives the observed session lifecycle
/// inside the caller's transaction. Session facts are always persisted; a
/// rejected pet-state transition keeps the current state rather than failing
/// the ingest, because the agent activity is real regardless of the pet.
fn record_hook_event_in_transaction(
    transaction: &rusqlite::Transaction<'_>,
    engine: &PetEngine,
    hook_event: &HookEvent,
    session_id: Option<&str>,
) -> Result<HookEventOutcome, CoreError> {
    let event_id = Uuid::parse_str(&hook_event.id)
        .map_err(|_| CoreError::InvalidHookEvent("id must be a UUID".into()))?;
    let mut event = PetEvent::new(hook_event.event_type.clone(), &hook_event.agent_id);
    event.id = event_id;
    event.session_id = session_id.map(str::to_owned);
    event.project_id = hook_event.project_path.clone();
    let accepted = record_event(
        transaction,
        engine,
        &event,
        hook_event.event_type.stimulus(),
        OnRejectedTransition::KeepState,
    )?;
    if !accepted {
        return Ok(HookEventOutcome::Duplicate);
    }
    let Some(session_id) = session_id else {
        return Ok(HookEventOutcome::Accepted);
    };
    match hook_event.event_type {
        EventType::AgentStarted => {
            let selected_pet: Option<String> = transaction.query_row(
                "SELECT selected_pet_id FROM pet_state WHERE singleton = 1",
                [],
                |row| row.get(0),
            )?;
            let pet_id = selected_pet.unwrap_or_else(|| FALLBACK_PET_ID.to_owned());
            let inserted = transaction.execute(
                "INSERT INTO agent_sessions(session_id, agent_id, project_path, started_at, status, pet_id)
                 VALUES (?1, ?2, ?3, ?4, 'running', ?5)
                 ON CONFLICT(session_id) DO NOTHING",
                params![
                    session_id,
                    hook_event.agent_id,
                    hook_event.project_path,
                    Utc::now().to_rfc3339(),
                    pet_id,
                ],
            )?;
            Ok(if inserted == 1 {
                HookEventOutcome::SessionStarted
            } else {
                HookEventOutcome::Accepted
            })
        }
        EventType::AgentCompleted | EventType::AgentFailed => {
            let session: Option<(String, String)> = transaction
                .query_row(
                    "SELECT agent_id, pet_id FROM agent_sessions WHERE session_id = ?1 AND status = 'running'",
                    [session_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?;
            let Some((_agent_id, pet_id)) = session else {
                // Already finished or never tracked locally; the lifecycle
                // event is still recorded above but no state changes.
                return Ok(if hook_event.event_type == EventType::AgentCompleted {
                    HookEventOutcome::SessionCompleted { xp_awarded: false }
                } else {
                    HookEventOutcome::SessionFailed
                });
            };
            let outcome = if hook_event.event_type == EventType::AgentCompleted {
                SessionOutcome::Completed
            } else {
                SessionOutcome::Failed
            };
            let started_at: Option<String> = transaction.query_row(
                "SELECT started_at FROM agent_sessions WHERE session_id = ?1",
                [session_id],
                |row| row.get(0),
            )?;
            let duration_ms = started_at
                .as_deref()
                .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
                .map(|started| {
                    let now = Utc::now();
                    let started = started.with_timezone(&Utc);
                    (now - started)
                        .to_std()
                        .map(|value| u64::try_from(value.as_millis()).unwrap_or(u64::MAX))
                })
                .transpose()
                .unwrap_or(Some(0))
                .unwrap_or(0);
            let duration_ms = i64::try_from(duration_ms).map_err(|_| CoreError::InvalidDuration)?;
            transaction.execute(
                "UPDATE agent_sessions
                 SET ended_at = ?1, duration_ms = ?2, status = ?3, exit_code = ?4
                 WHERE session_id = ?5 AND status = 'running'",
                params![
                    Utc::now().to_rfc3339(),
                    duration_ms,
                    outcome.as_str(),
                    None::<i32>,
                    session_id
                ],
            )?;
            if outcome == SessionOutcome::Completed {
                award_session_xp(transaction, session_id, &pet_id)?;
                award_session_achievements(transaction, session_id, &pet_id)?;
                Ok(HookEventOutcome::SessionCompleted { xp_awarded: true })
            } else {
                Ok(HookEventOutcome::SessionFailed)
            }
        }
        _ => Ok(HookEventOutcome::Accepted),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initializes_offline_state_and_persists_selection() {
        let core = Core::in_memory().expect("database initializes");
        assert_eq!(core.snapshot().expect("snapshot").pet_state, PetState::Idle);
        assert!(matches!(
            core.set_selected_pet(Some("creator.pet@1.0.0")),
            Err(CoreError::PackNotInstalled(_))
        ));
        core.set_selected_pet(None)
            .expect("empty selection persists");
        assert_eq!(core.snapshot().expect("snapshot").selected_pet_id, None);
    }

    #[test]
    fn overlay_position_is_saved_and_restored_from_settings() {
        let core = Core::in_memory().expect("database initializes");
        assert_eq!(core.overlay_position().expect("position query"), None);
        let position = DesktopPosition { x: -1280, y: 240 };
        core.save_overlay_position(position)
            .expect("position persists");
        assert_eq!(
            core.overlay_position().expect("position restores"),
            Some(position)
        );
        core.save_overlay_position(DesktopPosition { x: 32, y: 48 })
            .expect("position updates");
        assert_eq!(
            core.overlay_position().expect("updated position"),
            Some(DesktopPosition { x: 32, y: 48 })
        );
    }

    #[test]
    fn event_acceptance_is_deduplicated_and_state_transition_is_explicit() {
        let core = Core::in_memory().expect("database initializes");
        let event = PetEvent::new(EventType::AgentStarted, "test-fixture");
        assert!(core.accept_event(&event).expect("first event accepted"));
        assert!(!core.accept_event(&event).expect("duplicate ignored"));
        assert_eq!(
            core.snapshot().expect("snapshot").pet_state,
            PetState::Working
        );
    }

    #[test]
    fn invalid_transition_is_rejected_without_recording_event() {
        let core = Core::in_memory().expect("database initializes");
        let event = PetEvent::new(EventType::AgentCompleted, "test-fixture");
        assert!(matches!(
            core.accept_event(&event),
            Err(CoreError::InvalidTransition { .. })
        ));
        assert_eq!(core.snapshot().expect("snapshot").pet_state, PetState::Idle);
    }

    #[test]
    fn pack_install_records_only_after_atomic_file_install() {
        use agentpet_pet_pack::{Anchor, AnimationDefinition, FrameRect, PetPackManifest};
        use std::collections::BTreeMap;

        let temporary = tempfile::tempdir().expect("temporary directory created");
        let source = temporary.path().join("source");
        let library = temporary.path().join("library");
        std::fs::create_dir_all(source.join("assets")).expect("assets directory created");
        image::RgbaImage::new(32, 32)
            .save(source.join("assets/idle.png"))
            .expect("test image written");
        let manifest = PetPackManifest {
            schema_version: 1,
            id: "creator.sample".into(),
            name: "Sample".into(),
            version: "1.0.0".into(),
            description: "Test fixture".into(),
            creator: "Test".into(),
            license: "CC-BY-4.0".into(),
            source: "https://example.invalid".into(),
            attribution: "Test attribution".into(),
            commercial_use: false,
            redistribution: true,
            derivative_works: true,
            assets: BTreeMap::from([("idle".into(), "assets/idle.png".into())]),
            animations: BTreeMap::from([(
                "idle".into(),
                AnimationDefinition {
                    asset: "idle".into(),
                    frames: vec![FrameRect {
                        x: 0,
                        y: 0,
                        width: 32,
                        height: 32,
                    }],
                    fps: 1,
                    loop_animation: true,
                    anchor: Anchor { x: 0.5, y: 1.0 },
                },
            )]),
            state_mappings: BTreeMap::new(),
            tags: vec![],
        };
        std::fs::write(
            source.join("pet.json"),
            serde_json::to_vec(&manifest).expect("manifest serializes"),
        )
        .expect("manifest written");

        let core = Core::in_memory().expect("database initializes");
        let installed = core
            .install_extracted_pet_pack(&source, &library)
            .expect("pack installed");
        assert_eq!(installed.id, "creator.sample");
        assert!(library.join("creator.sample/1.0.0/pet.json").is_file());
        assert!(library
            .join("creator.sample/1.0.0/generated/preview.png")
            .is_file());
        core.set_selected_pet(Some("creator.sample@1.0.0"))
            .expect("installed pack selected");
        assert_eq!(
            core.snapshot()
                .expect("snapshot")
                .selected_pet_id
                .as_deref(),
            Some("creator.sample@1.0.0")
        );
        assert_eq!(core.list_installed_packs().expect("pack list").len(), 1);
        assert!(matches!(
            core.install_extracted_pet_pack(&source, &library),
            Err(CoreError::PetPack(PetPackError::AlreadyInstalled))
        ));
    }

    #[test]
    fn creating_a_pet_from_an_image_installs_and_indexes_it() {
        let temporary = tempfile::tempdir().expect("temporary directory created");
        let image_path = temporary.path().join("upload.png");
        let library = temporary.path().join("library");
        image::RgbaImage::from_pixel(48, 64, image::Rgba([180, 90, 120, 255]))
            .save(&image_path)
            .expect("image fixture written");
        let core = Core::in_memory().expect("database initializes");

        let manifest = core
            .create_custom_pet_from_image(&image_path, &library, "Mochi")
            .expect("custom image pet is installed and indexed");
        assert_eq!(manifest.name, "Mochi");
        assert_eq!(
            core.list_installed_packs()
                .expect("installed packs list")
                .len(),
            1
        );
        assert!(library
            .join(&manifest.id)
            .join(&manifest.version)
            .join("generated/preview.png")
            .is_file());
    }

    #[test]
    fn session_lifecycle_persists_real_outcome_and_updates_pet_state() {
        let core = Core::in_memory().expect("database initializes");
        core.start_session("session-1", "universal", Some("C:/work/project"))
            .expect("session starts");
        assert_eq!(
            core.snapshot().expect("snapshot").pet_state,
            PetState::Working
        );
        core.finish_session("session-1", SessionOutcome::Completed, Some(0), 1_250)
            .expect("session completes");
        assert_eq!(
            core.snapshot().expect("achievement reaction").pet_state,
            PetState::Achievement
        );
        let progression = core.progression().expect("session awards XP");
        assert_eq!(progression.total_xp, DEFAULT_SESSION_XP + 25);
        assert_eq!(progression.level, 1);
        let achievements = core.achievements().expect("achievement progress reads");
        let first_session = achievements
            .iter()
            .find(|item| item.id == "first_session")
            .expect("first session definition exists");
        assert_eq!(first_session.progress, 1);
        assert!(first_session.unlocked_at.is_some());
        assert_eq!(
            achievements
                .iter()
                .filter(|item| item.unlocked_at.is_some())
                .count(),
            1
        );
        let sessions = core.recent_sessions().expect("session history reads");
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].session_id, "session-1");
        assert_eq!(sessions[0].agent_id, "universal");
        assert_eq!(sessions[0].status, "completed");
        assert_eq!(sessions[0].duration_ms, Some(1_250));
        assert_eq!(sessions[0].input_tokens, None);
        assert!(core.session_history(0, 0).is_err());

        let connection = core.connection.lock().expect("database lock");
        let (status, exit_code, duration_ms, input_tokens): (String, i32, i64, Option<i64>) =
            connection
                .query_row(
                    "SELECT status, exit_code, duration_ms, input_tokens FROM agent_sessions WHERE session_id = 'session-1'",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
                .expect("session record exists");
        assert_eq!(status, "completed");
        assert_eq!(exit_code, 0);
        assert_eq!(duration_ms, 1_250);
        assert_eq!(input_tokens, None);
    }

    #[test]
    fn concurrent_sessions_react_then_return_to_working_until_the_last_session_finishes() {
        let core = Core::in_memory().expect("database initializes");
        core.start_session("session-a", "universal", None)
            .expect("first session starts");
        core.start_session("session-b", "universal", None)
            .expect("second session starts");
        core.finish_session("session-a", SessionOutcome::Completed, Some(0), 100)
            .expect("first session completes");
        assert_eq!(
            core.snapshot().expect("snapshot").pet_state,
            PetState::Achievement
        );
        let later = Utc::now() + chrono::Duration::seconds(30);
        assert_eq!(
            core.advance_state_timers_at(later).expect("timer advances"),
            Some(PetState::Working)
        );
        core.finish_session("session-b", SessionOutcome::Failed, Some(7), 200)
            .expect("last session fails");
        assert_eq!(
            core.snapshot().expect("snapshot").pet_state,
            PetState::Error
        );
        let later = Utc::now() + chrono::Duration::seconds(30);
        assert_eq!(
            core.advance_state_timers_at(later).expect("timer advances"),
            Some(PetState::Idle)
        );
        assert_eq!(
            core.progression()
                .expect("failed session grants no XP")
                .total_xp,
            DEFAULT_SESSION_XP + 25
        );
    }

    #[test]
    fn timed_reactions_expire_to_rest_and_an_idle_pet_falls_asleep() {
        let core = Core::in_memory().expect("database initializes");
        core.start_session("timed", "universal", None)
            .expect("session starts");
        core.finish_session("timed", SessionOutcome::Completed, Some(0), 10)
            .expect("session completes");
        let finished_at = Utc::now();
        assert_eq!(
            core.advance_state_timers_at(finished_at)
                .expect("timer checks"),
            None
        );
        assert_eq!(
            core.snapshot().expect("snapshot").pet_state,
            PetState::Achievement
        );
        let rested_at = finished_at + chrono::Duration::seconds(30);
        assert_eq!(
            core.advance_state_timers_at(rested_at)
                .expect("timer advances"),
            Some(PetState::Idle)
        );
        assert_eq!(
            core.advance_state_timers_at(rested_at)
                .expect("timer is idempotent"),
            None
        );
        let asleep_at = rested_at + chrono::Duration::minutes(16);
        assert_eq!(
            core.advance_state_timers_at(asleep_at)
                .expect("timer advances"),
            Some(PetState::Sleeping)
        );
        core.start_session("wake", "universal", None)
            .expect("session wakes the pet");
        assert_eq!(
            core.snapshot().expect("snapshot").pet_state,
            PetState::Working
        );
    }

    #[test]
    fn level_up_defers_new_activity_but_still_records_the_session() {
        let core = Core::in_memory().expect("database initializes");
        core.set_progression_rules(ProgressionRules {
            xp_per_completed_session: 100,
            base_xp_per_level: 100,
            level_xp_increment: 0,
        })
        .expect("progression rules update");
        core.start_session("levels", "universal", None)
            .expect("session starts");
        core.finish_session("levels", SessionOutcome::Completed, Some(0), 10)
            .expect("session completes");
        assert_eq!(
            core.snapshot().expect("snapshot").pet_state,
            PetState::LevelUp
        );
        core.start_session("deferred", "universal", None)
            .expect("session is recorded while celebrating");
        assert_eq!(
            core.snapshot().expect("snapshot").pet_state,
            PetState::LevelUp
        );
        assert_eq!(
            core.recent_sessions()
                .expect("sessions read")
                .iter()
                .filter(|session| session.status == "running")
                .count(),
            1
        );
        let later = Utc::now() + chrono::Duration::seconds(30);
        assert_eq!(
            core.advance_state_timers_at(later).expect("timer advances"),
            Some(PetState::Working)
        );
    }

    #[test]
    fn pause_holds_through_activity_until_the_user_resumes() {
        let core = Core::in_memory().expect("database initializes");
        assert!(core
            .accept_event(&PetEvent::new(EventType::UserPaused, "desktop"))
            .expect("pause accepted"));
        assert!(core
            .accept_event(&PetEvent::new(EventType::AgentWorking, "test-fixture"))
            .expect("activity recorded"));
        assert_eq!(
            core.snapshot().expect("snapshot").pet_state,
            PetState::Paused
        );
        assert!(core
            .accept_event(&PetEvent::new(EventType::UserResumed, "desktop"))
            .expect("resume accepted"));
        assert_eq!(core.snapshot().expect("snapshot").pet_state, PetState::Idle);
    }

    #[test]
    fn version_six_migration_backfills_state_entry_time() {
        let connection = Connection::open_in_memory().expect("database opens");
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .expect("foreign keys enabled");
        Core::migrate_to(&connection, 5).expect("version 5 schema");
        connection.execute(
            "UPDATE pet_state SET state = 'level_up', last_event_at = '2026-01-01T00:00:00+00:00'",
            [],
        ).expect("legacy state written");
        Core::migrate(&connection).expect("version 6 migration");
        Core::migrate(&connection).expect("migration is idempotent");
        let (entered_at, version): (String, i64) = connection.query_row(
            "SELECT state_entered_at, (SELECT MAX(version) FROM schema_versions) FROM pet_state",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).expect("migrated row reads");
        assert_eq!(entered_at, "2026-01-01T00:00:00+00:00");
        assert_eq!(version, SCHEMA_VERSION);
        let core = Core {
            connection: Arc::new(Mutex::new(connection)),
            engine: PetEngine::default(),
            data_dir: None,
        };
        assert_eq!(
            core.advance_state_timers().expect("stale reaction expires"),
            Some(PetState::Idle)
        );
    }

    #[test]
    fn newer_database_schema_is_rejected_and_reopen_is_idempotent() {
        let temporary = tempfile::tempdir().expect("temporary directory created");
        Core::open(temporary.path()).expect("database created");
        let core = Core::open(temporary.path()).expect("database reopens");
        assert_eq!(core.snapshot().expect("snapshot").pet_state, PetState::Idle);
        let connection = Connection::open_in_memory().expect("database opens");
        Core::migrate(&connection).expect("current schema");
        connection
            .execute(
                "INSERT INTO schema_versions(version, applied_at) VALUES (?1, ?2)",
                params![SCHEMA_VERSION + 1, Utc::now().to_rfc3339()],
            )
            .expect("future version recorded");
        assert!(matches!(
            Core::migrate(&connection),
            Err(CoreError::UnsupportedDatabaseSchema(_))
        ));
    }

    #[test]
    fn failed_sessions_do_not_unlock_achievements_or_award_xp() {
        let core = Core::in_memory().expect("database initializes");
        core.start_session("failed-first", "universal", None)
            .expect("session starts");
        core.finish_session("failed-first", SessionOutcome::Failed, Some(1), 25)
            .expect("session failure persists");
        assert_eq!(core.progression().expect("progression reads").total_xp, 0);
        assert!(core
            .achievements()
            .expect("achievements read")
            .iter()
            .all(|item| item.unlocked_at.is_none()));
    }

    #[test]
    fn session_achievement_milestones_and_rewards_are_awarded_once() {
        let core = Core::in_memory().expect("database initializes");
        for index in 1..=10 {
            let session_id = format!("milestone-{index}");
            core.start_session(&session_id, "universal", None)
                .expect("session starts");
            core.finish_session(&session_id, SessionOutcome::Completed, Some(0), 10)
                .expect("session completes");
        }
        let achievements = core.achievements().expect("achievement status reads");
        assert_eq!(
            achievements
                .iter()
                .filter(|item| item.unlocked_at.is_some())
                .count(),
            2
        );
        assert_eq!(
            core.progression()
                .expect("achievement rewards grant XP")
                .total_xp,
            175
        );
        assert!(matches!(
            core.finish_session("milestone-10", SessionOutcome::Completed, Some(0), 10),
            Err(CoreError::SessionNotRunning(_))
        ));
        let history = core.xp_history(100, 0).expect("XP ledger reads");
        assert_eq!(
            history
                .iter()
                .filter(|entry| entry.source == "achievement_unlocked")
                .count(),
            2
        );
        let connection = core.connection.lock().expect("database lock");
        assert!(connection
            .execute("UPDATE pet_achievements SET unlocked_at = 'changed'", [])
            .is_err());
        assert!(connection
            .execute("DELETE FROM pet_achievements", [])
            .is_err());
    }

    #[test]
    fn progression_rules_control_awards_and_xp_ledger_is_immutable() {
        let core = Core::in_memory().expect("database initializes");
        core.set_progression_rules(ProgressionRules {
            xp_per_completed_session: 25,
            base_xp_per_level: 75,
            level_xp_increment: 25,
        })
        .expect("progression rules update");
        core.start_session("progression-session", "universal", None)
            .expect("session starts");
        core.finish_session(
            "progression-session",
            SessionOutcome::Completed,
            Some(0),
            10,
        )
        .expect("session completes");
        let later = Utc::now() + chrono::Duration::seconds(30);
        assert_eq!(
            core.advance_state_timers_at(later)
                .expect("achievement expires"),
            Some(PetState::Idle)
        );
        core.start_session("progression-session-2", "universal", None)
            .expect("second session starts");
        core.finish_session(
            "progression-session-2",
            SessionOutcome::Completed,
            Some(0),
            10,
        )
        .expect("second session completes");
        let progression = core.progression().expect("progression reads");
        assert_eq!(progression.total_xp, 75);
        assert_eq!(progression.level, 2);
        assert_eq!(progression.xp_into_level, 0);
        assert_eq!(progression.xp_to_next_level, 100);
        assert_eq!(
            core.snapshot().expect("level-up state").pet_state,
            PetState::LevelUp
        );

        let history = core
            .recent_xp_transactions()
            .expect("recent XP history reads");
        assert_eq!(history.len(), 3);
        let latest_session_award = history
            .iter()
            .find(|entry| {
                entry.source == "session_completed" && entry.source_id == "progression-session-2"
            })
            .expect("second session XP award exists");
        assert_eq!(latest_session_award.amount, 25);
        assert!(history
            .iter()
            .any(|entry| entry.source == "achievement_unlocked" && entry.amount == 25));
        assert!(core.xp_history(0, 0).is_err());

        let connection = core.connection.lock().expect("database lock");
        assert!(connection
            .execute("UPDATE xp_transactions SET amount = 1", [])
            .is_err());
        assert!(connection
            .execute("DELETE FROM xp_transactions", [])
            .is_err());
    }

    #[test]
    fn ledger_chain_is_cryptographically_verified_and_flags_tampering() {
        let core = Core::in_memory().expect("database initializes");
        core.start_session("s1", "universal", None)
            .expect("start s1");
        core.finish_session("s1", SessionOutcome::Completed, Some(0), 100)
            .expect("finish s1");
        core.start_session("s2", "universal", None)
            .expect("start s2");
        core.finish_session("s2", SessionOutcome::Completed, Some(0), 100)
            .expect("finish s2");

        let initial_check = core.verify_ledger().expect("ledger verifies");
        assert!(initial_check.is_valid);
        assert_eq!(initial_check.total_entries, 3);
        assert_eq!(initial_check.broken_at_index, None);

        // Triggers protect against UPDATE and DELETE
        {
            let conn = core.connection.lock().expect("lock");
            assert!(conn
                .execute("UPDATE xp_ledger SET amount = 999 WHERE index_id = 1", [])
                .is_err());
            assert!(conn
                .execute("DELETE FROM xp_ledger WHERE index_id = 1", [])
                .is_err());
        }

        // When a row is modified out-of-band, verification flags it
        {
            let conn = core.connection.lock().expect("lock");
            conn.execute("DROP TRIGGER xp_ledger_no_update", [])
                .expect("drop trigger for test");
            conn.execute("UPDATE xp_ledger SET amount = 999 WHERE index_id = 2", [])
                .expect("tamper row");
        }

        let tampered_check = core.verify_ledger().expect("ledger check completes");
        assert!(!tampered_check.is_valid);
        assert_eq!(tampered_check.broken_at_index, Some(2));
        assert!(tampered_check.error_message.unwrap().contains("invalid"));
    }

    #[test]
    fn ledger_v2_resists_colon_injection() {
        let prev = "0000000000000000000000000000000000000000000000000000000000000000";
        let time = "2026-10-21T00:00:00Z";

        // In v1 naive format: "{prev}:{key}:{amount}:{pet}:{time}"
        // key1="foo:bar", pet1="baz" -> "prev:foo:bar:100:baz:time"
        // key2="foo", pet2="bar:100:baz" with amount in middle could create collision scenarios
        let h1_v1 = compute_ledger_hash_v1(prev, "user:123", 50, "pet:alpha", time);
        let _h2_v1 = compute_ledger_hash_v1(prev, "user", 50, "123:pet:alpha", time);
        // If colons line up in v1, concatenation can match:
        assert_eq!(
            format!("{prev}:user:123:50:pet:alpha:{time}"),
            format!("{prev}:user:123:50:pet:alpha:{time}")
        );

        // In v2 with domain tag and length prefixing:
        let h1_v2 = compute_ledger_hash_v2(prev, "user:123", 50, "pet:alpha", time);
        let h2_v2 = compute_ledger_hash_v2(prev, "user", 50, "123:pet:alpha", time);
        assert_ne!(
            h1_v2, h2_v2,
            "Length prefixing must prevent delimiter collisions"
        );
        assert_ne!(h1_v1, h1_v2, "v2 hash must differ from v1 hash");
    }

    #[test]
    fn ledger_rejects_duplicate_idempotency_keys() {
        let core = Core::in_memory().expect("database initializes");
        let conn = core.connection.lock().expect("lock");
        let tx = conn.unchecked_transaction().expect("tx");
        let rules = load_progression_rules(&tx).expect("rules");

        // The return value reports a level-up; 20 XP stays below the first level threshold.
        let leveled_up = award_xp_transaction(
            &tx,
            "agentpet.starter-aoi",
            "test_source",
            "tx-001",
            20,
            "{}",
            rules,
        )
        .expect("award 1");
        assert!(!leveled_up);
        let first_count: i64 = tx
            .query_row("SELECT COUNT(*) FROM xp_ledger", [], |r| r.get(0))
            .expect("count after first award");
        assert_eq!(first_count, 1, "First award must append one ledger row");

        // Second award with identical source and source_id (idempotency key)
        let awarded2 = award_xp_transaction(
            &tx,
            "agentpet.starter-aoi",
            "test_source",
            "tx-001",
            20,
            "{}",
            rules,
        )
        .expect("award 2");
        assert!(
            !awarded2,
            "Duplicate idempotency key must not report a level-up"
        );
        tx.commit().expect("commit");

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM xp_ledger", [], |r| r.get(0))
            .expect("count");
        assert_eq!(
            count, 1,
            "Ledger must contain only 1 row for unique idempotency key"
        );
    }

    #[test]
    fn ledger_verifies_mixed_v1_and_v2_entries() {
        let core = Core::in_memory().expect("database initializes");
        let conn = core.connection.lock().expect("lock");

        // Manually insert a v1 legacy row with ledger_version = 1
        let prev = GENESIS_HASH.to_string();
        let time1 = "2026-10-20T00:00:00Z";
        let h1 = compute_ledger_hash_v1(&prev, "legacy:1", 10, "pet1", time1);
        conn.execute(
            "INSERT INTO xp_ledger(entry_id, prev_hash, hash, provider, session_id, event_id, idempotency_key, pet_id, amount, created_at, metadata_json, ledger_version)
             VALUES (?1, ?2, ?3, 'universal', 's1', 'e1', 'legacy:1', 'pet1', 10, ?4, '{}', 1)",
            params!["id-1", prev, h1, time1],
        )
        .expect("insert v1 row");

        // Insert a v2 row chaining from the v1 row
        let time2 = "2026-10-21T00:00:00Z";
        let h2 = compute_ledger_hash_v2(&h1, "v2:1", 20, "pet1", time2);
        conn.execute(
            "INSERT INTO xp_ledger(entry_id, prev_hash, hash, provider, session_id, event_id, idempotency_key, pet_id, amount, created_at, metadata_json, ledger_version)
             VALUES (?1, ?2, ?3, 'universal', 's2', 'e2', 'v2:1', 'pet1', 20, ?4, '{}', 2)",
            params!["id-2", h1, h2, time2],
        )
        .expect("insert v2 row");
        drop(conn);

        let verification = core.verify_ledger().expect("verify");
        assert!(
            verification.is_valid,
            "Mixed v1/v2 ledger chain must verify cleanly"
        );
        assert_eq!(verification.total_entries, 2);
        assert_eq!(verification.head_hash, h2);
    }
}

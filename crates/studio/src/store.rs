//! SQLite persistence for the studio library. Binary assets live on disk under
//! the studio root; rows only hold relative paths and metadata.

use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::Serialize;

use crate::error::{ErrorKind, ErrorReport, ProviderError, StudioError};
use crate::task::TaskState;

pub const DATABASE_FILE: &str = "studio.sqlite";
pub const SCHEMA_VERSION: i64 = 2;
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);
const TASK_LIST_MAX: u32 = 500;
const NAME_MAX_CHARS: usize = 120;

/// v2: local export history (files the user saved outside the library).
const MIGRATION_V2: &str = "
CREATE TABLE exports (
    id TEXT PRIMARY KEY,
    generation_id TEXT NOT NULL REFERENCES generations(id) ON DELETE CASCADE,
    output_id INTEGER NOT NULL REFERENCES generation_outputs(id) ON DELETE CASCADE,
    format TEXT NOT NULL,
    options_json TEXT NOT NULL,
    destination TEXT NOT NULL,
    files_json TEXT NOT NULL,
    byte_size INTEGER NOT NULL,
    sha256 TEXT NOT NULL,
    triangles INTEGER,
    created_at TEXT NOT NULL
);
CREATE INDEX exports_by_generation ON exports(generation_id, created_at);
";

const MIGRATION_V1: &str = "
CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE provider_accounts (
    provider TEXT PRIMARY KEY,
    credential_ref TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 120),
    created_at TEXT NOT NULL
);
CREATE TABLE assets (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 120),
    tags_json TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL
);
CREATE INDEX assets_by_project ON assets(project_id, created_at);
CREATE TABLE generations (
    id TEXT PRIMARY KEY,
    asset_id TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    parent_version_id TEXT REFERENCES asset_versions(id),
    operation TEXT NOT NULL,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    prompt TEXT,
    negative_prompt TEXT,
    intent_json TEXT,
    settings_json TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE TABLE generation_inputs (
    id INTEGER PRIMARY KEY,
    generation_id TEXT NOT NULL REFERENCES generations(id) ON DELETE CASCADE,
    role TEXT NOT NULL,
    original_name TEXT NOT NULL,
    stored_path TEXT NOT NULL,
    sha256 TEXT NOT NULL,
    mime TEXT NOT NULL,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    byte_size INTEGER NOT NULL
);
CREATE TABLE tasks (
    id TEXT PRIMARY KEY,
    generation_id TEXT NOT NULL UNIQUE REFERENCES generations(id) ON DELETE CASCADE,
    provider TEXT NOT NULL,
    provider_task_id TEXT NOT NULL,
    operation TEXT NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('QUEUED','RUNNING','SUCCEEDED','FAILED','CANCELLED','EXPIRED')),
    stage TEXT,
    progress INTEGER CHECK (progress BETWEEN 0 AND 100),
    created_at TEXT NOT NULL,
    started_at TEXT,
    completed_at TEXT,
    updated_at TEXT NOT NULL,
    error_kind TEXT,
    error_message TEXT,
    request_id TEXT,
    provider_code INTEGER,
    attempts INTEGER NOT NULL DEFAULT 1,
    UNIQUE (provider, provider_task_id)
);
CREATE INDEX tasks_by_state ON tasks(state);
CREATE TABLE asset_versions (
    id TEXT PRIMARY KEY,
    asset_id TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    version_number INTEGER NOT NULL,
    generation_id TEXT NOT NULL UNIQUE REFERENCES generations(id) ON DELETE CASCADE,
    parent_version_id TEXT REFERENCES asset_versions(id),
    created_at TEXT NOT NULL,
    UNIQUE (asset_id, version_number)
);
CREATE TABLE generation_outputs (
    id INTEGER PRIMARY KEY,
    generation_id TEXT NOT NULL REFERENCES generations(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    label TEXT NOT NULL,
    format TEXT NOT NULL,
    stored_path TEXT NOT NULL,
    sha256 TEXT NOT NULL,
    byte_size INTEGER NOT NULL,
    report_json TEXT,
    UNIQUE (generation_id, label)
);
CREATE TABLE usage_records (
    task_id TEXT PRIMARY KEY REFERENCES tasks(id) ON DELETE CASCADE,
    provider TEXT NOT NULL,
    credits REAL NOT NULL,
    source TEXT NOT NULL CHECK (source IN ('provider', 'estimated')),
    recorded_at TEXT NOT NULL
);
";

pub fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectView {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub asset_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetSummary {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub created_at: String,
    pub version_count: i64,
    pub latest_version_id: Option<String>,
    pub thumbnail_output_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskView {
    pub id: String,
    pub generation_id: String,
    pub asset_id: String,
    pub asset_name: String,
    pub provider: String,
    pub provider_task_id: String,
    pub operation: String,
    pub state: TaskState,
    pub stage: Option<String>,
    pub progress: Option<u8>,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub updated_at: String,
    pub attempts: i64,
    pub error: Option<ErrorReport>,
    /// Re-checking the same provider task is safe and free.
    pub can_resume: bool,
    pub version_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputView {
    pub role: String,
    pub original_name: String,
    pub mime: String,
    pub width: u32,
    pub height: u32,
    pub byte_size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputView {
    pub id: i64,
    pub kind: String,
    pub label: String,
    pub format: String,
    pub byte_size: u64,
    pub sha256: String,
    pub report: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageView {
    pub credits: f64,
    /// `provider` = reported by the provider; `estimated` = calculated locally.
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportView {
    pub id: String,
    pub format: String,
    /// Main file the user chose in the save dialog.
    pub destination: String,
    /// Every file written (main file plus OBJ material/texture sidecars).
    pub files: Vec<String>,
    pub byte_size: u64,
    pub triangles: Option<u64>,
    pub created_at: String,
}

pub struct NewExport {
    pub generation_id: String,
    pub output_id: i64,
    pub format: String,
    pub options_json: String,
    pub destination: String,
    pub files: Vec<String>,
    pub byte_size: u64,
    pub sha256: String,
    pub triangles: Option<u64>,
}

pub struct OutputSource {
    pub generation_id: String,
    pub kind: String,
    pub format: String,
    pub stored_path: String,
    pub asset_name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationView {
    pub id: String,
    pub operation: String,
    pub provider: String,
    pub model: String,
    pub prompt: Option<String>,
    pub negative_prompt: Option<String>,
    pub intent: Option<serde_json::Value>,
    pub settings: serde_json::Value,
    pub parent_version_id: Option<String>,
    pub created_at: String,
    pub inputs: Vec<InputView>,
    pub outputs: Vec<OutputView>,
    pub task: Option<TaskView>,
    pub usage: Option<UsageView>,
    pub version_id: Option<String>,
    pub version_number: Option<i64>,
    /// Files saved outside the library, newest first.
    pub exports: Vec<ExportView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetDetail {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub created_at: String,
    /// Newest first; includes failed generations so history stays complete.
    pub generations: Vec<GenerationView>,
}

/// Everything recorded when a provider accepts a request.
pub struct NewGeneration<'a> {
    pub project_id: &'a str,
    pub asset_id: Option<&'a str>,
    pub asset_name: &'a str,
    pub parent_version_id: Option<&'a str>,
    pub operation: &'a str,
    pub provider: &'a str,
    pub model: &'a str,
    pub prompt: Option<&'a str>,
    pub negative_prompt: Option<&'a str>,
    pub intent_json: Option<String>,
    pub settings_json: String,
    pub provider_task_id: &'a str,
    pub generation_id: String,
    pub inputs: Vec<NewInput>,
}

pub struct NewInput {
    pub role: &'static str,
    pub original_name: String,
    pub stored_path: String,
    pub sha256: String,
    pub mime: String,
    pub width: u32,
    pub height: u32,
    pub byte_size: u64,
}

pub struct NewOutput {
    pub kind: &'static str,
    pub label: String,
    pub format: String,
    pub stored_path: String,
    pub sha256: String,
    pub byte_size: u64,
    pub report_json: Option<String>,
}

/// Internal row used by the poller.
#[derive(Debug, Clone)]
pub struct TaskRecord {
    pub id: String,
    pub generation_id: String,
    pub asset_id: String,
    pub provider: String,
    pub provider_task_id: String,
    pub state: TaskState,
}

#[derive(Clone)]
pub struct StudioStore {
    connection: Arc<Mutex<Connection>>,
}

impl StudioStore {
    pub fn open(root: &Path) -> Result<Self, StudioError> {
        std::fs::create_dir_all(root)?;
        let connection = Connection::open(root.join(DATABASE_FILE))?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        Self::init(connection)
    }

    pub fn in_memory() -> Result<Self, StudioError> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(connection: Connection) -> Result<Self, StudioError> {
        connection.busy_timeout(BUSY_TIMEOUT)?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        migrate(&connection)?;
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    fn lock(&self) -> MutexGuard<'_, Connection> {
        // An interrupted transaction rolls back on Drop, so the data stays consistent.
        self.connection
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn schema_version(&self) -> Result<i64, StudioError> {
        Ok(self
            .lock()
            .query_row("SELECT MAX(version) FROM schema_versions", [], |row| {
                row.get(0)
            })?)
    }

    pub fn setting(&self, key: &str) -> Result<Option<String>, StudioError> {
        Ok(self
            .lock()
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
                row.get(0)
            })
            .optional()?)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), StudioError> {
        self.lock().execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn credential_ref(&self, provider: &str) -> Result<Option<String>, StudioError> {
        Ok(self
            .lock()
            .query_row(
                "SELECT credential_ref FROM provider_accounts WHERE provider = ?1",
                [provider],
                |row| row.get(0),
            )
            .optional()?)
    }

    pub fn set_credential_ref(&self, provider: &str, reference: &str) -> Result<(), StudioError> {
        self.lock().execute(
            "INSERT INTO provider_accounts (provider, credential_ref, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(provider) DO UPDATE SET credential_ref = excluded.credential_ref,
                                                 updated_at = excluded.updated_at",
            params![provider, reference, now()],
        )?;
        Ok(())
    }

    pub fn remove_credential_ref(&self, provider: &str) -> Result<(), StudioError> {
        self.lock().execute(
            "DELETE FROM provider_accounts WHERE provider = ?1",
            [provider],
        )?;
        Ok(())
    }

    pub fn create_project(&self, name: &str) -> Result<ProjectView, StudioError> {
        let project = ProjectView {
            id: new_id(),
            name: clean_name(name)?,
            created_at: now(),
            asset_count: 0,
        };
        self.lock().execute(
            "INSERT INTO projects (id, name, created_at) VALUES (?1, ?2, ?3)",
            params![project.id, project.name, project.created_at],
        )?;
        Ok(project)
    }

    pub fn list_projects(&self) -> Result<Vec<ProjectView>, StudioError> {
        let connection = self.lock();
        let mut statement = connection.prepare(
            "SELECT p.id, p.name, p.created_at, (SELECT COUNT(*) FROM assets a WHERE a.project_id = p.id)
             FROM projects p ORDER BY p.created_at, p.rowid",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(ProjectView {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
                asset_count: row.get(3)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn list_assets(&self, project_id: &str) -> Result<Vec<AssetSummary>, StudioError> {
        let connection = self.lock();
        let mut statement = connection.prepare(
            "SELECT a.id, a.project_id, a.name, a.created_at,
                    (SELECT COUNT(*) FROM asset_versions v WHERE v.asset_id = a.id),
                    (SELECT v.id FROM asset_versions v WHERE v.asset_id = a.id
                      ORDER BY v.version_number DESC LIMIT 1),
                    (SELECT o.id FROM asset_versions v
                       JOIN generation_outputs o ON o.generation_id = v.generation_id
                      WHERE v.asset_id = a.id AND o.kind = 'preview_image'
                      ORDER BY v.version_number DESC LIMIT 1)
             FROM assets a WHERE a.project_id = ?1 ORDER BY a.created_at DESC, a.rowid DESC",
        )?;
        let rows = statement.query_map([project_id], |row| {
            Ok(AssetSummary {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                created_at: row.get(3)?,
                version_count: row.get(4)?,
                latest_version_id: row.get(5)?,
                thumbnail_output_id: row.get(6)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Records an accepted provider task atomically. Replaying the same
    /// provider task ID returns the existing task instead of duplicating it.
    pub fn record_submission(
        &self,
        generation: NewGeneration<'_>,
    ) -> Result<TaskView, StudioError> {
        let mut connection = self.lock();
        let existing: Option<String> = connection
            .query_row(
                "SELECT id FROM tasks WHERE provider = ?1 AND provider_task_id = ?2",
                params![generation.provider, generation.provider_task_id],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(task_id) = existing {
            return task_view(&connection, &task_id);
        }
        let transaction = connection.transaction()?;
        let created_at = now();
        let asset_id = match generation.asset_id {
            Some(asset_id) => {
                let project: Option<String> = transaction
                    .query_row(
                        "SELECT project_id FROM assets WHERE id = ?1",
                        [asset_id],
                        |row| row.get(0),
                    )
                    .optional()?;
                if project.as_deref() != Some(generation.project_id) {
                    return Err(StudioError::NotFound(format!("asset {asset_id}")));
                }
                asset_id.to_owned()
            }
            None => {
                let asset_id = new_id();
                transaction.execute(
                    "INSERT INTO assets (id, project_id, name, created_at) VALUES (?1, ?2, ?3, ?4)",
                    params![
                        asset_id,
                        generation.project_id,
                        clean_name(generation.asset_name)?,
                        created_at
                    ],
                )?;
                asset_id
            }
        };
        transaction.execute(
            "INSERT INTO generations (id, asset_id, parent_version_id, operation, provider, model, prompt,
                                      negative_prompt, intent_json, settings_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                generation.generation_id,
                asset_id,
                generation.parent_version_id,
                generation.operation,
                generation.provider,
                generation.model,
                generation.prompt,
                generation.negative_prompt,
                generation.intent_json,
                generation.settings_json,
                created_at
            ],
        )?;
        for input in &generation.inputs {
            transaction.execute(
                "INSERT INTO generation_inputs (generation_id, role, original_name, stored_path, sha256, mime,
                                                width, height, byte_size)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    generation.generation_id,
                    input.role,
                    input.original_name,
                    input.stored_path,
                    input.sha256,
                    input.mime,
                    input.width,
                    input.height,
                    input.byte_size as i64
                ],
            )?;
        }
        let task_id = new_id();
        transaction.execute(
            "INSERT INTO tasks (id, generation_id, provider, provider_task_id, operation, state, stage,
                                created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'QUEUED', 'submitted', ?6, ?6)",
            params![
                task_id,
                generation.generation_id,
                generation.provider,
                generation.provider_task_id,
                generation.operation,
                created_at
            ],
        )?;
        transaction.commit()?;
        task_view(&connection, &task_id)
    }

    pub fn task(&self, task_id: &str) -> Result<TaskRecord, StudioError> {
        self.lock()
            .query_row(
                "SELECT t.id, t.generation_id, g.asset_id, t.provider, t.provider_task_id, t.state
                 FROM tasks t JOIN generations g ON g.id = t.generation_id WHERE t.id = ?1",
                [task_id],
                |row| {
                    Ok(TaskRecord {
                        id: row.get(0)?,
                        generation_id: row.get(1)?,
                        asset_id: row.get(2)?,
                        provider: row.get(3)?,
                        provider_task_id: row.get(4)?,
                        state: parse_state(row, 5)?,
                    })
                },
            )
            .optional()?
            .ok_or_else(|| StudioError::NotFound(format!("task {task_id}")))
    }

    pub fn task_view(&self, task_id: &str) -> Result<TaskView, StudioError> {
        task_view(&self.lock(), task_id)
    }

    pub fn list_tasks(&self, limit: u32) -> Result<Vec<TaskView>, StudioError> {
        let connection = self.lock();
        let ids: Vec<String> = connection
            .prepare("SELECT id FROM tasks ORDER BY created_at DESC, rowid DESC LIMIT ?1")?
            .query_map([limit.min(TASK_LIST_MAX)], |row| row.get(0))?
            .collect::<Result<_, _>>()?;
        ids.iter().map(|id| task_view(&connection, id)).collect()
    }

    pub fn active_task_ids(&self) -> Result<Vec<String>, StudioError> {
        let connection = self.lock();
        let ids = connection
            .prepare(
                "SELECT id FROM tasks WHERE state IN ('QUEUED', 'RUNNING') ORDER BY created_at",
            )?
            .query_map([], |row| row.get(0))?
            .collect::<Result<_, _>>()?;
        Ok(ids)
    }

    /// Guarded non-terminal-success state change: returns false (and writes
    /// nothing) when not allowed, e.g. a replayed poll after completion.
    /// `Succeeded` is only reachable through `complete_task`.
    pub fn update_task(
        &self,
        task_id: &str,
        next: TaskState,
        stage: Option<&str>,
        progress: Option<u8>,
    ) -> Result<bool, StudioError> {
        let connection = self.lock();
        let current = current_state(&connection, task_id)?;
        if next == TaskState::Succeeded || !current.can_transition_to(next) {
            return Ok(false);
        }
        connection.execute(
            "UPDATE tasks SET state = ?2, stage = ?3, progress = COALESCE(?4, progress), updated_at = ?5,
                    started_at = CASE WHEN ?2 = 'RUNNING' AND started_at IS NULL THEN ?5 ELSE started_at END,
                    completed_at = CASE WHEN ?2 IN ('FAILED','CANCELLED','EXPIRED') THEN ?5 ELSE completed_at END,
                    error_kind = NULL, error_message = NULL, request_id = NULL, provider_code = NULL
             WHERE id = ?1",
            params![task_id, next.as_str(), stage, progress, now()],
        )?;
        Ok(true)
    }

    /// Records a non-fatal error (e.g. a dropped poll) without changing state.
    pub fn note_transient_error(
        &self,
        task_id: &str,
        error: &ProviderError,
    ) -> Result<(), StudioError> {
        self.lock().execute(
            "UPDATE tasks SET stage = 'retrying', error_kind = ?2, error_message = ?3, request_id = ?4,
                    provider_code = ?5, updated_at = ?6
             WHERE id = ?1 AND state IN ('QUEUED', 'RUNNING')",
            params![task_id, error.kind.as_str(), error.message, error.request_id, error.provider_code, now()],
        )?;
        Ok(())
    }

    pub fn fail_task(
        &self,
        task_id: &str,
        terminal: TaskState,
        error: &ProviderError,
    ) -> Result<bool, StudioError> {
        let connection = self.lock();
        let current = current_state(&connection, task_id)?;
        if !terminal.is_terminal()
            || terminal == TaskState::Succeeded
            || !current.can_transition_to(terminal)
        {
            return Ok(false);
        }
        connection.execute(
            "UPDATE tasks SET state = ?2, stage = NULL, error_kind = ?3, error_message = ?4, request_id = ?5,
                    provider_code = ?6, completed_at = ?7, updated_at = ?7
             WHERE id = ?1",
            params![
                task_id,
                terminal.as_str(),
                error.kind.as_str(),
                error.message,
                error.request_id,
                error.provider_code,
                now()
            ],
        )?;
        Ok(true)
    }

    /// Failed (transient) -> Running for another tracking attempt. Costs no credits.
    pub fn resume_task(&self, task_id: &str) -> Result<TaskView, StudioError> {
        let connection = self.lock();
        if !task_view(&connection, task_id)?.can_resume {
            return Err(StudioError::InvalidInput(
                "Only tasks that failed locally (network, timeout, download) can be resumed."
                    .into(),
            ));
        }
        connection.execute(
            "UPDATE tasks SET state = 'RUNNING', stage = 'resuming', completed_at = NULL, error_kind = NULL,
                    error_message = NULL, request_id = NULL, provider_code = NULL,
                    attempts = attempts + 1, updated_at = ?2
             WHERE id = ?1",
            params![task_id, now()],
        )?;
        task_view(&connection, task_id)
    }

    /// Finalizes a task after outputs were downloaded and validated. Every
    /// write is idempotent, so a replayed completion creates no duplicates.
    pub fn complete_task(
        &self,
        task_id: &str,
        outputs: &[NewOutput],
        provider_credits: Option<f64>,
    ) -> Result<TaskView, StudioError> {
        let mut connection = self.lock();
        let transaction = connection.transaction()?;
        let (generation_id, asset_id, parent_version_id, provider, state): (
            String,
            String,
            Option<String>,
            String,
            String,
        ) = transaction
            .query_row(
                "SELECT g.id, g.asset_id, g.parent_version_id, t.provider, t.state
                 FROM tasks t JOIN generations g ON g.id = t.generation_id WHERE t.id = ?1",
                [task_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| StudioError::NotFound(format!("task {task_id}")))?;
        let current = TaskState::parse(&state).unwrap_or(TaskState::Failed);
        if current != TaskState::Succeeded && !current.can_transition_to(TaskState::Succeeded) {
            return Err(StudioError::InvalidInput(format!(
                "Task is {state} and cannot complete."
            )));
        }
        for output in outputs {
            transaction.execute(
                "INSERT INTO generation_outputs (generation_id, kind, label, format, stored_path, sha256,
                                                 byte_size, report_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(generation_id, label) DO UPDATE SET format = excluded.format,
                    stored_path = excluded.stored_path, sha256 = excluded.sha256,
                    byte_size = excluded.byte_size, report_json = excluded.report_json",
                params![
                    generation_id,
                    output.kind,
                    output.label,
                    output.format,
                    output.stored_path,
                    output.sha256,
                    output.byte_size as i64,
                    output.report_json
                ],
            )?;
        }
        let timestamp = now();
        transaction.execute(
            "INSERT INTO asset_versions (id, asset_id, version_number, generation_id, parent_version_id, created_at)
             SELECT ?1, ?2, COALESCE(MAX(version_number), 0) + 1, ?3, ?4, ?5
             FROM asset_versions WHERE asset_id = ?2
             ON CONFLICT(generation_id) DO NOTHING",
            params![new_id(), asset_id, generation_id, parent_version_id, timestamp],
        )?;
        if let Some(credits) =
            provider_credits.filter(|credits| credits.is_finite() && *credits >= 0.0)
        {
            transaction.execute(
                "INSERT INTO usage_records (task_id, provider, credits, source, recorded_at)
                 VALUES (?1, ?2, ?3, 'provider', ?4) ON CONFLICT(task_id) DO NOTHING",
                params![task_id, provider, credits, timestamp],
            )?;
        }
        transaction.execute(
            "UPDATE tasks SET state = 'SUCCEEDED', stage = NULL, progress = 100, error_kind = NULL,
                    error_message = NULL, request_id = NULL, provider_code = NULL,
                    started_at = COALESCE(started_at, ?2), completed_at = COALESCE(completed_at, ?2),
                    updated_at = ?2
             WHERE id = ?1",
            params![task_id, timestamp],
        )?;
        transaction.commit()?;
        task_view(&connection, task_id)
    }

    /// Output facts needed to export it: generation, kind, format, stored path, asset name.
    pub fn output_source(&self, output_id: i64) -> Result<OutputSource, StudioError> {
        self.lock()
            .query_row(
                "SELECT o.generation_id, o.kind, o.format, o.stored_path, a.name
                 FROM generation_outputs o
                 JOIN generations g ON g.id = o.generation_id
                 JOIN assets a ON a.id = g.asset_id
                 WHERE o.id = ?1",
                [output_id],
                |row| {
                    Ok(OutputSource {
                        generation_id: row.get(0)?,
                        kind: row.get(1)?,
                        format: row.get(2)?,
                        stored_path: row.get(3)?,
                        asset_name: row.get(4)?,
                    })
                },
            )
            .optional()?
            .ok_or_else(|| StudioError::NotFound(format!("output {output_id}")))
    }

    pub fn record_export(&self, export: &NewExport) -> Result<ExportView, StudioError> {
        let view = ExportView {
            id: new_id(),
            format: export.format.clone(),
            destination: export.destination.clone(),
            files: export.files.clone(),
            byte_size: export.byte_size,
            triangles: export.triangles,
            created_at: now(),
        };
        let files_json = serde_json::to_string(&view.files)
            .map_err(|error| StudioError::InvalidInput(error.to_string()))?;
        self.lock().execute(
            "INSERT INTO exports (id, generation_id, output_id, format, options_json, destination, files_json,
                                  byte_size, sha256, triangles, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                view.id,
                export.generation_id,
                export.output_id,
                view.format,
                export.options_json,
                view.destination,
                files_json,
                view.byte_size as i64,
                export.sha256,
                view.triangles.map(|value| value as i64),
                view.created_at
            ],
        )?;
        Ok(view)
    }

    pub fn output_location(&self, output_id: i64) -> Result<(String, String), StudioError> {
        self.lock()
            .query_row(
                "SELECT stored_path, format FROM generation_outputs WHERE id = ?1",
                [output_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?
            .ok_or_else(|| StudioError::NotFound(format!("output {output_id}")))
    }

    /// Inputs with their stored relative paths.
    pub fn generation_inputs(
        &self,
        generation_id: &str,
    ) -> Result<Vec<(InputView, String)>, StudioError> {
        inputs(&self.lock(), generation_id)
    }

    pub fn generation(&self, generation_id: &str) -> Result<GenerationView, StudioError> {
        generation_view(&self.lock(), generation_id)
    }

    pub fn asset_project(&self, asset_id: &str) -> Result<String, StudioError> {
        self.lock()
            .query_row(
                "SELECT project_id FROM assets WHERE id = ?1",
                [asset_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| StudioError::NotFound(format!("asset {asset_id}")))
    }

    pub fn asset_detail(&self, asset_id: &str) -> Result<AssetDetail, StudioError> {
        let connection = self.lock();
        let (id, project_id, name, created_at) = connection
            .query_row(
                "SELECT id, project_id, name, created_at FROM assets WHERE id = ?1",
                [asset_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()?
            .ok_or_else(|| StudioError::NotFound(format!("asset {asset_id}")))?;
        let generation_ids: Vec<String> = connection
            .prepare("SELECT id FROM generations WHERE asset_id = ?1 ORDER BY created_at DESC, rowid DESC")?
            .query_map([asset_id], |row| row.get(0))?
            .collect::<Result<_, _>>()?;
        let generations = generation_ids
            .iter()
            .map(|generation_id| generation_view(&connection, generation_id))
            .collect::<Result<_, _>>()?;
        Ok(AssetDetail {
            id,
            project_id,
            name,
            created_at,
            generations,
        })
    }
}

fn migrate(connection: &Connection) -> Result<(), StudioError> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_versions (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL);",
    )?;
    let current: i64 = connection.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_versions",
        [],
        |row| row.get(0),
    )?;
    if current > SCHEMA_VERSION {
        return Err(StudioError::UnsupportedSchema(current));
    }
    if current < 1 {
        let transaction = connection.unchecked_transaction()?;
        transaction.execute_batch(MIGRATION_V1)?;
        transaction.execute(
            "INSERT INTO schema_versions (version, applied_at) VALUES (1, ?1)",
            [now()],
        )?;
        transaction.commit()?;
    }
    if current < 2 {
        let transaction = connection.unchecked_transaction()?;
        transaction.execute_batch(MIGRATION_V2)?;
        transaction.execute(
            "INSERT INTO schema_versions (version, applied_at) VALUES (2, ?1)",
            [now()],
        )?;
        transaction.commit()?;
    }
    Ok(())
}

pub fn clean_name(name: &str) -> Result<String, StudioError> {
    let name: String = name.chars().filter(|ch| !ch.is_control()).collect();
    let name: String = name.trim().chars().take(NAME_MAX_CHARS).collect();
    if name.trim().is_empty() {
        return Err(StudioError::InvalidInput("Name must not be empty.".into()));
    }
    Ok(name.trim().to_owned())
}

fn parse_state(row: &Row<'_>, index: usize) -> rusqlite::Result<TaskState> {
    let value: String = row.get(index)?;
    TaskState::parse(&value).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Text,
            format!("unknown task state {value}").into(),
        )
    })
}

fn current_state(connection: &Connection, task_id: &str) -> Result<TaskState, StudioError> {
    connection
        .query_row("SELECT state FROM tasks WHERE id = ?1", [task_id], |row| {
            parse_state(row, 0)
        })
        .optional()?
        .ok_or_else(|| StudioError::NotFound(format!("task {task_id}")))
}

fn task_view(connection: &Connection, task_id: &str) -> Result<TaskView, StudioError> {
    connection
        .query_row(
            "SELECT t.id, t.generation_id, a.id, a.name, t.provider, t.provider_task_id, t.operation, t.state,
                    t.stage, t.progress, t.created_at, t.started_at, t.completed_at, t.updated_at, t.attempts,
                    t.error_kind, t.error_message, t.request_id, t.provider_code, v.id
             FROM tasks t
             JOIN generations g ON g.id = t.generation_id
             JOIN assets a ON a.id = g.asset_id
             LEFT JOIN asset_versions v ON v.generation_id = t.generation_id
             WHERE t.id = ?1",
            [task_id],
            |row| {
                let state = parse_state(row, 7)?;
                let kind: Option<String> = row.get(15)?;
                let parsed_kind = kind.as_deref().and_then(ErrorKind::parse);
                let error = match kind {
                    Some(kind) => Some(ErrorReport {
                        suggestion: parsed_kind.map_or("Retry the task.", ErrorKind::suggestion).into(),
                        kind,
                        message: row.get::<_, Option<String>>(16)?.unwrap_or_default(),
                        request_id: row.get(17)?,
                        provider_code: row.get(18)?,
                    }),
                    None => None,
                };
                Ok(TaskView {
                    id: row.get(0)?,
                    generation_id: row.get(1)?,
                    asset_id: row.get(2)?,
                    asset_name: row.get(3)?,
                    provider: row.get(4)?,
                    provider_task_id: row.get(5)?,
                    operation: row.get(6)?,
                    state,
                    stage: row.get(8)?,
                    progress: row.get::<_, Option<i64>>(9)?.map(|value| value.clamp(0, 100) as u8),
                    created_at: row.get(10)?,
                    started_at: row.get(11)?,
                    completed_at: row.get(12)?,
                    updated_at: row.get(13)?,
                    attempts: row.get(14)?,
                    can_resume: state == TaskState::Failed && parsed_kind.is_some_and(ErrorKind::is_transient),
                    error,
                    version_id: row.get(19)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| StudioError::NotFound(format!("task {task_id}")))
}

fn inputs(
    connection: &Connection,
    generation_id: &str,
) -> Result<Vec<(InputView, String)>, StudioError> {
    let mut statement = connection.prepare(
        "SELECT role, original_name, mime, width, height, byte_size, sha256, stored_path
         FROM generation_inputs WHERE generation_id = ?1 ORDER BY id",
    )?;
    let rows = statement.query_map([generation_id], |row| {
        Ok((
            InputView {
                role: row.get(0)?,
                original_name: row.get(1)?,
                mime: row.get(2)?,
                width: row.get(3)?,
                height: row.get(4)?,
                byte_size: row.get::<_, i64>(5)? as u64,
                sha256: row.get(6)?,
            },
            row.get(7)?,
        ))
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

fn generation_view(
    connection: &Connection,
    generation_id: &str,
) -> Result<GenerationView, StudioError> {
    let parse_json = |text: Option<String>| text.and_then(|text| serde_json::from_str(&text).ok());
    let mut view = connection
        .query_row(
            "SELECT g.id, g.operation, g.provider, g.model, g.prompt, g.negative_prompt, g.intent_json,
                    g.settings_json, g.parent_version_id, g.created_at, v.id, v.version_number
             FROM generations g LEFT JOIN asset_versions v ON v.generation_id = g.id WHERE g.id = ?1",
            [generation_id],
            |row| {
                Ok(GenerationView {
                    id: row.get(0)?,
                    operation: row.get(1)?,
                    provider: row.get(2)?,
                    model: row.get(3)?,
                    prompt: row.get(4)?,
                    negative_prompt: row.get(5)?,
                    intent: parse_json(row.get(6)?),
                    settings: parse_json(row.get(7)?).unwrap_or(serde_json::Value::Null),
                    parent_version_id: row.get(8)?,
                    created_at: row.get(9)?,
                    version_id: row.get(10)?,
                    version_number: row.get(11)?,
                    exports: Vec::new(),
                    inputs: Vec::new(),
                    outputs: Vec::new(),
                    task: None,
                    usage: None,
                })
            },
        )
        .optional()?
        .ok_or_else(|| StudioError::NotFound(format!("generation {generation_id}")))?;
    view.exports = connection
        .prepare(
            "SELECT id, format, destination, files_json, byte_size, triangles, created_at FROM exports
             WHERE generation_id = ?1 ORDER BY created_at DESC, rowid DESC",
        )?
        .query_map([generation_id], |row| {
            Ok(ExportView {
                id: row.get(0)?,
                format: row.get(1)?,
                destination: row.get(2)?,
                files: serde_json::from_str(&row.get::<_, String>(3)?).unwrap_or_default(),
                byte_size: row.get::<_, i64>(4)? as u64,
                triangles: row.get::<_, Option<i64>>(5)?.map(|value| value as u64),
                created_at: row.get(6)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    view.inputs = inputs(connection, generation_id)?
        .into_iter()
        .map(|(input, _)| input)
        .collect();
    let mut statement = connection.prepare(
        "SELECT id, kind, label, format, byte_size, sha256, report_json FROM generation_outputs
         WHERE generation_id = ?1 ORDER BY label",
    )?;
    view.outputs = statement
        .query_map([generation_id], |row| {
            Ok(OutputView {
                id: row.get(0)?,
                kind: row.get(1)?,
                label: row.get(2)?,
                format: row.get(3)?,
                byte_size: row.get::<_, i64>(4)? as u64,
                sha256: row.get(5)?,
                report: parse_json(row.get(6)?),
            })
        })?
        .collect::<Result<_, _>>()?;
    let task_id: Option<String> = connection
        .query_row(
            "SELECT id FROM tasks WHERE generation_id = ?1",
            [generation_id],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(task_id) = task_id {
        view.usage = connection
            .query_row(
                "SELECT credits, source FROM usage_records WHERE task_id = ?1",
                [&task_id],
                |row| {
                    Ok(UsageView {
                        credits: row.get(0)?,
                        source: row.get(1)?,
                    })
                },
            )
            .optional()?;
        view.task = Some(task_view(connection, &task_id)?);
    }
    Ok(view)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn submission<'a>(
        project_id: &'a str,
        provider_task_id: &'a str,
    ) -> NewGeneration<'a> {
        NewGeneration {
            project_id,
            asset_id: None,
            asset_name: "Chest",
            parent_version_id: None,
            operation: "text_to_3d",
            provider: "tripo",
            model: "v3.1-20260211",
            prompt: Some("wooden chest"),
            negative_prompt: None,
            intent_json: None,
            settings_json: "{}".into(),
            provider_task_id,
            generation_id: new_id(),
            inputs: Vec::new(),
        }
    }

    fn model_output() -> NewOutput {
        NewOutput {
            kind: "model",
            label: "model_url".into(),
            format: "glb".into(),
            stored_path: "assets/a/g/model_url.glb".into(),
            sha256: "00".into(),
            byte_size: 10,
            report_json: None,
        }
    }

    #[test]
    fn version_one_database_upgrades_to_two() {
        let dir = tempfile::tempdir().unwrap();
        {
            let connection = Connection::open(dir.path().join(DATABASE_FILE)).unwrap();
            connection
                .execute_batch("CREATE TABLE schema_versions (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL);")
                .unwrap();
            connection.execute_batch(MIGRATION_V1).unwrap();
            connection
                .execute("INSERT INTO schema_versions VALUES (1, 'then')", [])
                .unwrap();
        }
        let store = StudioStore::open(dir.path()).unwrap();
        assert_eq!(store.schema_version().unwrap(), 2);
        let exports: i64 = store
            .lock()
            .query_row("SELECT COUNT(*) FROM exports", [], |row| row.get(0))
            .unwrap();
        assert_eq!(exports, 0);
    }

    #[test]
    fn migration_is_versioned_idempotent_and_enforces_foreign_keys() {
        let dir = tempfile::tempdir().unwrap();
        let store = StudioStore::open(dir.path()).unwrap();
        assert_eq!(store.schema_version().unwrap(), SCHEMA_VERSION);
        drop(store);
        let store = StudioStore::open(dir.path()).unwrap();
        assert_eq!(store.schema_version().unwrap(), SCHEMA_VERSION);
        let orphan = store.lock().execute(
            "INSERT INTO assets (id, project_id, name, created_at) VALUES ('a', 'missing', 'x', 'now')",
            [],
        );
        assert!(orphan.is_err(), "foreign keys must be enforced");
    }

    #[test]
    fn newer_schema_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        StudioStore::open(dir.path())
            .unwrap()
            .lock()
            .execute("INSERT INTO schema_versions VALUES (99, 'x')", [])
            .unwrap();
        assert!(matches!(
            StudioStore::open(dir.path()),
            Err(StudioError::UnsupportedSchema(99))
        ));
    }

    #[test]
    fn duplicate_submission_and_replayed_completion_do_not_duplicate_records() {
        let store = StudioStore::in_memory().unwrap();
        let project = store.create_project("Props").unwrap();
        let first = store
            .record_submission(submission(&project.id, "task_1"))
            .unwrap();
        let replay = store
            .record_submission(submission(&project.id, "task_1"))
            .unwrap();
        assert_eq!(first.id, replay.id);
        assert_eq!(store.list_assets(&project.id).unwrap().len(), 1);

        assert!(store
            .update_task(&first.id, TaskState::Running, Some("generating"), Some(40))
            .unwrap());
        store
            .complete_task(&first.id, &[model_output()], Some(30.0))
            .unwrap();
        let again = store
            .complete_task(&first.id, &[model_output()], Some(30.0))
            .unwrap();
        assert_eq!(again.state, TaskState::Succeeded);

        let connection = store.lock();
        let count = |sql: &str| {
            connection
                .query_row(sql, [], |row| row.get::<_, i64>(0))
                .unwrap()
        };
        assert_eq!(count("SELECT COUNT(*) FROM asset_versions"), 1);
        assert_eq!(count("SELECT COUNT(*) FROM generation_outputs"), 1);
        assert_eq!(count("SELECT COUNT(*) FROM usage_records"), 1);
    }

    #[test]
    fn terminal_tasks_ignore_late_poll_updates() {
        let store = StudioStore::in_memory().unwrap();
        let project = store.create_project("P").unwrap();
        let task = store
            .record_submission(submission(&project.id, "task_2"))
            .unwrap();
        store
            .complete_task(&task.id, &[model_output()], None)
            .unwrap();
        assert!(!store
            .update_task(&task.id, TaskState::Running, Some("generating"), Some(10))
            .unwrap());
        let error = ProviderError::new(ErrorKind::TaskFailed, "late");
        assert!(!store
            .fail_task(&task.id, TaskState::Failed, &error)
            .unwrap());
        assert_eq!(
            store.task_view(&task.id).unwrap().state,
            TaskState::Succeeded
        );
    }

    #[test]
    fn only_transient_failures_resume() {
        let store = StudioStore::in_memory().unwrap();
        let project = store.create_project("P").unwrap();
        let network = store
            .record_submission(submission(&project.id, "task_3"))
            .unwrap();
        let offline = ProviderError::new(ErrorKind::ProviderUnavailable, "offline");
        store
            .fail_task(&network.id, TaskState::Failed, &offline)
            .unwrap();
        assert!(store.task_view(&network.id).unwrap().can_resume);
        let resumed = store.resume_task(&network.id).unwrap();
        assert_eq!((resumed.state, resumed.attempts), (TaskState::Running, 2));

        let banned = store
            .record_submission(submission(&project.id, "task_4"))
            .unwrap();
        let policy = ProviderError::new(ErrorKind::TaskFailed, "policy");
        store
            .fail_task(&banned.id, TaskState::Failed, &policy)
            .unwrap();
        assert!(!store.task_view(&banned.id).unwrap().can_resume);
        assert!(store.resume_task(&banned.id).is_err());
    }

    #[test]
    fn versions_number_sequentially_and_keep_lineage() {
        let store = StudioStore::in_memory().unwrap();
        let project = store.create_project("P").unwrap();
        let first = store
            .record_submission(submission(&project.id, "task_5"))
            .unwrap();
        let v1 = store
            .complete_task(&first.id, &[model_output()], None)
            .unwrap()
            .version_id
            .unwrap();
        let mut second = submission(&project.id, "task_6");
        second.asset_id = Some(&first.asset_id);
        second.parent_version_id = Some(&v1);
        let second = store.record_submission(second).unwrap();
        store
            .complete_task(&second.id, &[model_output()], None)
            .unwrap();
        let detail = store.asset_detail(&first.asset_id).unwrap();
        assert_eq!(detail.generations.len(), 2);
        assert_eq!(detail.generations[0].version_number, Some(2));
        assert_eq!(
            detail.generations[0].parent_version_id.as_deref(),
            Some(v1.as_str())
        );
        assert_eq!(detail.generations[1].version_number, Some(1));
    }

    #[test]
    fn asset_must_belong_to_project_and_names_are_required() {
        let store = StudioStore::in_memory().unwrap();
        let one = store.create_project("One").unwrap();
        let two = store.create_project("Two").unwrap();
        let task = store
            .record_submission(submission(&one.id, "task_7"))
            .unwrap();
        let mut foreign = submission(&two.id, "task_8");
        foreign.asset_id = Some(&task.asset_id);
        assert!(matches!(
            store.record_submission(foreign),
            Err(StudioError::NotFound(_))
        ));
        assert!(store.create_project("   ").is_err());
    }
}

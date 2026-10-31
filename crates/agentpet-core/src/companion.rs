//! Rust-owned companion preferences, overlay placement, and pre-migration
//! database backups.
//!
//! Preferences previously lived in browser `localStorage`. They are imported
//! once through [`Core::import_legacy_companion_settings`]; the completion
//! marker is written in the same transaction as the imported values, so an
//! interrupted import reruns safely. Frontend-generated "bond XP" is never
//! accepted here: XP comes only from the ledger.

use std::{
    fs,
    path::{Path, PathBuf},
};

use chrono::Utc;
use rusqlite::{params, Connection, DatabaseName, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::{Core, CoreError};

pub(crate) const COMPANION_SCHEMA_V11: &str = "
CREATE TABLE companion_preferences (
    singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
    render_mode TEXT NOT NULL DEFAULT '3d' CHECK(render_mode IN ('2d', '3d')),
    sound_muted INTEGER NOT NULL DEFAULT 0 CHECK(sound_muted IN (0, 1)),
    affection INTEGER NOT NULL DEFAULT 75 CHECK(affection BETWEEN 0 AND 100),
    nickname TEXT NOT NULL DEFAULT '',
    accessory TEXT NOT NULL DEFAULT 'none',
    aura TEXT NOT NULL DEFAULT '',
    vibe TEXT NOT NULL DEFAULT 'sweet' CHECK(vibe IN ('sweet', 'playful', 'loyal', 'calm')),
    reduced_motion INTEGER NOT NULL DEFAULT 0 CHECK(reduced_motion IN (0, 1)),
    shortcut_show_hide TEXT DEFAULT 'Ctrl+Alt+P',
    shortcut_pause TEXT DEFAULT 'Ctrl+Alt+Space',
    shortcut_reposition TEXT DEFAULT 'Ctrl+Alt+R',
    overlay_monitor TEXT,
    overlay_logical_x REAL,
    overlay_logical_y REAL,
    legacy_import_completed_at TEXT,
    updated_at TEXT NOT NULL,
    CHECK((overlay_monitor IS NULL) = (overlay_logical_x IS NULL)),
    CHECK((overlay_monitor IS NULL) = (overlay_logical_y IS NULL))
);
CREATE TABLE daily_companion_moments (
    pet_id TEXT NOT NULL CHECK(length(pet_id) BETWEEN 1 AND 200),
    local_date TEXT NOT NULL CHECK(local_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
    utc_offset_minutes INTEGER NOT NULL CHECK(utc_offset_minutes BETWEEN -1080 AND 1080),
    message_id TEXT NOT NULL CHECK(length(message_id) BETWEEN 1 AND 200),
    created_at TEXT NOT NULL,
    PRIMARY KEY(pet_id, local_date)
);
";

/// Matches the nickname input's `maxLength` in the companion customizer.
const MAX_NICKNAME_CHARS: usize = 18;
const ACCESSORIES: &[&str] = &[
    "none", "ribbon", "flower", "crown", "glasses", "halo", "bell",
];
const MAX_AFFECTION: u8 = 100;
const MAX_MONITOR_NAME_CHARS: usize = 256;
/// Larger than any real work area; rejects corrupted or hostile offsets.
const MAX_LOGICAL_OFFSET: f64 = 100_000.0;
const BACKUP_DIR: &str = "backups";
const BACKUP_PREFIX: &str = "agentpet-";
const BACKUP_SUFFIX: &str = ".sqlite3";
const BACKUPS_RETAINED: usize = 3;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RenderMode {
    #[serde(rename = "2d")]
    TwoD,
    #[serde(rename = "3d")]
    ThreeD,
}

impl RenderMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::TwoD => "2d",
            Self::ThreeD => "3d",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "2d" => Some(Self::TwoD),
            "3d" => Some(Self::ThreeD),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CompanionVibe {
    Sweet,
    Playful,
    Loyal,
    Calm,
}

impl CompanionVibe {
    fn as_str(self) -> &'static str {
        match self {
            Self::Sweet => "sweet",
            Self::Playful => "playful",
            Self::Loyal => "loyal",
            Self::Calm => "calm",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "sweet" => Some(Self::Sweet),
            "playful" => Some(Self::Playful),
            "loyal" => Some(Self::Loyal),
            "calm" => Some(Self::Calm),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompanionPreferences {
    pub render_mode: RenderMode,
    pub sound_muted: bool,
    pub affection: u8,
    pub nickname: String,
    pub accessory: String,
    /// Empty or a `#rrggbb` color; rendered as CSS, so nothing else is accepted.
    pub aura: String,
    pub vibe: CompanionVibe,
    pub reduced_motion: bool,
}

impl CompanionPreferences {
    fn validate(&self) -> Result<(), CoreError> {
        let invalid = |message: &str| Err(CoreError::InvalidCompanionPreferences(message.into()));
        if self.affection > MAX_AFFECTION {
            return invalid("affection must be between 0 and 100");
        }
        if !valid_nickname(&self.nickname) {
            return invalid("nickname must be at most 18 characters without control characters");
        }
        if !ACCESSORIES.contains(&self.accessory.as_str()) {
            return invalid("accessory is not a known option");
        }
        if !valid_aura(&self.aura) {
            return invalid("aura must be empty or a #rrggbb color");
        }
        Ok(())
    }
}

/// Raw browser `localStorage` strings, keyed by the legacy setting they held.
/// Every field is untrusted; invalid values are skipped and keep the default.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LegacyCompanionSettings {
    pub render_mode: Option<String>,
    pub sound_muted: Option<String>,
    pub affection: Option<String>,
    pub nickname: Option<String>,
    pub accessory: Option<String>,
    pub aura: Option<String>,
    pub vibe: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LegacyImportOutcome {
    /// `false` when an earlier import already completed; nothing changed.
    pub imported: bool,
    pub preferences: CompanionPreferences,
}

/// Overlay location as a logical-DIP offset from the named monitor's work-area
/// origin, so it survives DPI changes and monitor rearrangement.
#[derive(Debug, Clone, PartialEq)]
pub struct OverlayPlacement {
    pub monitor: String,
    pub logical_x: f64,
    pub logical_y: f64,
}

impl OverlayPlacement {
    fn validate(&self) -> Result<(), CoreError> {
        let monitor_chars = self.monitor.chars().count();
        let offset_ok = |value: f64| value.is_finite() && value.abs() <= MAX_LOGICAL_OFFSET;
        if monitor_chars == 0
            || monitor_chars > MAX_MONITOR_NAME_CHARS
            || self.monitor.chars().any(char::is_control)
            || !offset_ok(self.logical_x)
            || !offset_ok(self.logical_y)
        {
            return Err(CoreError::InvalidCompanionPreferences(
                "overlay placement is out of range".into(),
            ));
        }
        Ok(())
    }
}

fn valid_nickname(value: &str) -> bool {
    value.chars().count() <= MAX_NICKNAME_CHARS && !value.chars().any(char::is_control)
}

fn valid_aura(value: &str) -> bool {
    value.is_empty()
        || (value.len() == 7
            && value.starts_with('#')
            && value[1..].bytes().all(|byte| byte.is_ascii_hexdigit()))
}

fn read_preferences(connection: &Connection) -> Result<CompanionPreferences, CoreError> {
    let (render_mode, sound_muted, affection, nickname, accessory, aura, vibe, reduced_motion): (
        String,
        bool,
        u8,
        String,
        String,
        String,
        String,
        bool,
    ) = connection.query_row(
        "SELECT render_mode, sound_muted, affection, nickname, accessory, aura, vibe, reduced_motion
         FROM companion_preferences WHERE singleton = 1",
        [],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
            ))
        },
    )?;
    let stored = || CoreError::InvalidCompanionPreferences("stored value is corrupt".into());
    Ok(CompanionPreferences {
        render_mode: RenderMode::parse(&render_mode).ok_or_else(stored)?,
        sound_muted,
        affection,
        nickname,
        accessory,
        aura,
        vibe: CompanionVibe::parse(&vibe).ok_or_else(stored)?,
        reduced_motion,
    })
}

fn write_preferences(
    connection: &Connection,
    preferences: &CompanionPreferences,
) -> Result<(), CoreError> {
    preferences.validate()?;
    connection.execute(
        "UPDATE companion_preferences
         SET render_mode = ?1, sound_muted = ?2, affection = ?3, nickname = ?4, accessory = ?5,
             aura = ?6, vibe = ?7, reduced_motion = ?8, updated_at = ?9
         WHERE singleton = 1",
        params![
            preferences.render_mode.as_str(),
            preferences.sound_muted,
            preferences.affection,
            preferences.nickname,
            preferences.accessory,
            preferences.aura,
            preferences.vibe.as_str(),
            preferences.reduced_motion,
            Utc::now().to_rfc3339(),
        ],
    )?;
    Ok(())
}

/// Applies each valid legacy value over `base`; invalid values are skipped.
fn merge_legacy(
    mut base: CompanionPreferences,
    legacy: &LegacyCompanionSettings,
) -> CompanionPreferences {
    if let Some(mode) = legacy.render_mode.as_deref().and_then(RenderMode::parse) {
        base.render_mode = mode;
    }
    if let Some(muted) = legacy.sound_muted.as_deref() {
        // The old frontend treated anything except "true" as unmuted.
        base.sound_muted = muted == "true";
    }
    if let Some(affection) = legacy
        .affection
        .as_deref()
        .and_then(|value| value.trim().parse::<i64>().ok())
    {
        base.affection = affection.clamp(0, i64::from(MAX_AFFECTION)) as u8;
    }
    if let Some(nickname) = legacy
        .nickname
        .as_deref()
        .filter(|value| valid_nickname(value))
    {
        base.nickname = nickname.to_owned();
    }
    if let Some(accessory) = legacy
        .accessory
        .as_deref()
        .filter(|value| ACCESSORIES.contains(value))
    {
        base.accessory = accessory.to_owned();
    }
    if let Some(aura) = legacy.aura.as_deref().filter(|value| valid_aura(value)) {
        base.aura = aura.to_owned();
    }
    if let Some(vibe) = legacy.vibe.as_deref().and_then(CompanionVibe::parse) {
        base.vibe = vibe;
    }
    base
}

impl Core {
    pub fn companion_preferences(&self) -> Result<CompanionPreferences, CoreError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| CoreError::LockPoisoned)?;
        read_preferences(&connection)
    }

    pub fn save_companion_preferences(
        &self,
        preferences: &CompanionPreferences,
    ) -> Result<CompanionPreferences, CoreError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| CoreError::LockPoisoned)?;
        write_preferences(&connection, preferences)?;
        read_preferences(&connection)
    }

    /// One-time import of browser-stored settings. Values and the completion
    /// marker commit together; a repeated call after completion changes nothing.
    pub fn import_legacy_companion_settings(
        &self,
        legacy: &LegacyCompanionSettings,
    ) -> Result<LegacyImportOutcome, CoreError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| CoreError::LockPoisoned)?;
        let transaction =
            connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let completed: Option<String> = transaction.query_row(
            "SELECT legacy_import_completed_at FROM companion_preferences WHERE singleton = 1",
            [],
            |row| row.get(0),
        )?;
        if completed.is_some() {
            let preferences = read_preferences(&transaction)?;
            return Ok(LegacyImportOutcome {
                imported: false,
                preferences,
            });
        }
        let merged = merge_legacy(read_preferences(&transaction)?, legacy);
        write_preferences(&transaction, &merged)?;
        transaction.execute(
            "UPDATE companion_preferences SET legacy_import_completed_at = ?1 WHERE singleton = 1",
            [Utc::now().to_rfc3339()],
        )?;
        let preferences = read_preferences(&transaction)?;
        transaction.commit()?;
        Ok(LegacyImportOutcome {
            imported: true,
            preferences,
        })
    }

    pub fn overlay_placement(&self) -> Result<Option<OverlayPlacement>, CoreError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| CoreError::LockPoisoned)?;
        let row: Option<(Option<String>, Option<f64>, Option<f64>)> = connection
            .query_row(
                "SELECT overlay_monitor, overlay_logical_x, overlay_logical_y
                 FROM companion_preferences WHERE singleton = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        Ok(match row {
            Some((Some(monitor), Some(logical_x), Some(logical_y))) => Some(OverlayPlacement {
                monitor,
                logical_x,
                logical_y,
            }),
            _ => None,
        })
    }

    pub fn save_overlay_placement(&self, placement: &OverlayPlacement) -> Result<(), CoreError> {
        placement.validate()?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| CoreError::LockPoisoned)?;
        connection.execute(
            "UPDATE companion_preferences
             SET overlay_monitor = ?1, overlay_logical_x = ?2, overlay_logical_y = ?3, updated_at = ?4
             WHERE singleton = 1",
            params![
                placement.monitor,
                placement.logical_x,
                placement.logical_y,
                Utc::now().to_rfc3339()
            ],
        )?;
        Ok(())
    }
}

/// Highest applied schema version, or 0 for a new database.
pub(crate) fn stored_schema_version(connection: &Connection) -> Result<i64, CoreError> {
    let has_versions: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'schema_versions')",
        [],
        |row| row.get(0),
    )?;
    if !has_versions {
        return Ok(0);
    }
    Ok(connection.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_versions",
        [],
        |row| row.get(0),
    )?)
}

/// Copies the live database with the SQLite backup API, verifies the copy with
/// `PRAGMA integrity_check`, and keeps only the newest backups. Migrations are
/// forward-only, so callers must not migrate when this fails.
pub(crate) fn backup_before_migration(
    connection: &Connection,
    data_dir: &Path,
    from_version: i64,
) -> Result<PathBuf, CoreError> {
    let backup_error = |error: std::io::Error| CoreError::Backup(error.to_string());
    let directory = data_dir.join(BACKUP_DIR);
    fs::create_dir_all(&directory).map_err(backup_error)?;
    let path = directory.join(format!(
        "{BACKUP_PREFIX}{}-v{from_version}{BACKUP_SUFFIX}",
        Utc::now().format("%Y%m%dT%H%M%S%.3fZ")
    ));
    let result = connection
        .backup(DatabaseName::Main, &path, None)
        .map_err(CoreError::from)
        .and_then(|()| verify_backup(&path));
    if let Err(error) = result {
        let _ = fs::remove_file(&path);
        return Err(error);
    }
    prune_backups(&directory)?;
    Ok(path)
}

fn verify_backup(path: &Path) -> Result<(), CoreError> {
    let copy = Connection::open(path)?;
    // A standalone file: no -wal/-shm companions left beside the backup.
    copy.pragma_update(None, "journal_mode", "DELETE")?;
    let report: String = copy.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    if report == "ok" {
        Ok(())
    } else {
        Err(CoreError::Backup(format!(
            "integrity_check reported {report}"
        )))
    }
}

fn prune_backups(directory: &Path) -> Result<(), CoreError> {
    let backup_error = |error: std::io::Error| CoreError::Backup(error.to_string());
    let mut backups: Vec<PathBuf> = fs::read_dir(directory)
        .map_err(backup_error)?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.starts_with(BACKUP_PREFIX) && name.ends_with(BACKUP_SUFFIX)
                })
        })
        .collect();
    // Names start with a UTC timestamp, so lexical order is chronological.
    backups.sort();
    let excess = backups.len().saturating_sub(BACKUPS_RETAINED);
    for stale in &backups[..excess] {
        fs::remove_file(stale).map_err(backup_error)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn defaults() -> CompanionPreferences {
        CompanionPreferences {
            render_mode: RenderMode::ThreeD,
            sound_muted: false,
            affection: 75,
            nickname: String::new(),
            accessory: "none".into(),
            aura: String::new(),
            vibe: CompanionVibe::Sweet,
            reduced_motion: false,
        }
    }

    fn ledger_fingerprint(core: &Core) -> (u64, String) {
        let result = core.verify_ledger().expect("ledger verifies");
        assert!(result.is_valid);
        (result.total_entries, result.head_hash)
    }

    #[test]
    fn new_database_starts_with_default_preferences_and_no_placement() {
        let core = Core::in_memory().expect("database initializes");
        assert_eq!(core.companion_preferences().expect("read"), defaults());
        assert_eq!(core.overlay_placement().expect("placement"), None);
    }

    #[test]
    fn preferences_round_trip_and_reject_invalid_values() {
        let core = Core::in_memory().expect("database initializes");
        let updated = CompanionPreferences {
            render_mode: RenderMode::TwoD,
            sound_muted: true,
            affection: 100,
            nickname: "Mochi".into(),
            accessory: "crown".into(),
            aura: "#38bdf8".into(),
            vibe: CompanionVibe::Calm,
            reduced_motion: true,
        };
        assert_eq!(
            core.save_companion_preferences(&updated).expect("save"),
            updated
        );

        let invalid = [
            CompanionPreferences {
                affection: 101,
                ..updated.clone()
            },
            CompanionPreferences {
                nickname: "x".repeat(19),
                ..updated.clone()
            },
            CompanionPreferences {
                nickname: "bad\u{7}".into(),
                ..updated.clone()
            },
            CompanionPreferences {
                accessory: "laser".into(),
                ..updated.clone()
            },
            CompanionPreferences {
                aura: "red; background:url(x)".into(),
                ..updated.clone()
            },
        ];
        for candidate in invalid {
            assert!(matches!(
                core.save_companion_preferences(&candidate),
                Err(CoreError::InvalidCompanionPreferences(_))
            ));
        }
        assert_eq!(core.companion_preferences().expect("read"), updated);
    }

    #[test]
    fn legacy_import_applies_valid_values_once_and_leaves_ledger_untouched() {
        let core = Core::in_memory().expect("database initializes");
        let before = ledger_fingerprint(&core);
        let legacy = LegacyCompanionSettings {
            render_mode: Some("2d".into()),
            sound_muted: Some("true".into()),
            affection: Some("250".into()),
            nickname: Some("Pip".into()),
            accessory: Some("not-an-accessory".into()),
            aura: Some("#ff69b4".into()),
            vibe: Some("loyal".into()),
        };

        let first = core
            .import_legacy_companion_settings(&legacy)
            .expect("import");
        assert!(first.imported);
        assert_eq!(
            first.preferences,
            CompanionPreferences {
                render_mode: RenderMode::TwoD,
                sound_muted: true,
                affection: 100,
                nickname: "Pip".into(),
                accessory: "none".into(),
                aura: "#ff69b4".into(),
                vibe: CompanionVibe::Loyal,
                reduced_motion: false,
            }
        );

        let second = core
            .import_legacy_companion_settings(&LegacyCompanionSettings {
                render_mode: Some("3d".into()),
                ..LegacyCompanionSettings::default()
            })
            .expect("repeat import");
        assert!(!second.imported);
        assert_eq!(second.preferences, first.preferences);
        assert_eq!(ledger_fingerprint(&core), before);
    }

    #[test]
    fn legacy_import_rejects_unknown_fields_such_as_bond_xp() {
        let payload = r#"{"renderMode":"2d","bondXp":"9000"}"#;
        assert!(serde_json::from_str::<LegacyCompanionSettings>(payload).is_err());
    }

    #[test]
    fn overlay_placement_round_trips_and_rejects_hostile_values() {
        let core = Core::in_memory().expect("database initializes");
        let placement = OverlayPlacement {
            monitor: r"\\.\DISPLAY2".into(),
            logical_x: 412.5,
            logical_y: -20.0,
        };
        core.save_overlay_placement(&placement).expect("save");
        assert_eq!(core.overlay_placement().expect("read"), Some(placement));

        for bad in [
            OverlayPlacement {
                monitor: String::new(),
                logical_x: 0.0,
                logical_y: 0.0,
            },
            OverlayPlacement {
                monitor: "m".into(),
                logical_x: f64::NAN,
                logical_y: 0.0,
            },
            OverlayPlacement {
                monitor: "m".into(),
                logical_x: 0.0,
                logical_y: 1e9,
            },
        ] {
            assert!(core.save_overlay_placement(&bad).is_err());
        }
    }

    #[test]
    fn daily_moments_are_unique_per_pet_and_local_date() {
        let core = Core::in_memory().expect("database initializes");
        let connection = core.connection.lock().expect("lock");
        let insert = |pet: &str, date: &str| {
            connection.execute(
                "INSERT INTO daily_companion_moments(pet_id, local_date, utc_offset_minutes, message_id, created_at)
                 VALUES (?1, ?2, 330, 'm1', '2026-10-31T04:00:00Z')",
                params![pet, date],
            )
        };
        insert("pet.a", "2026-10-31").expect("first moment");
        assert!(insert("pet.a", "2026-10-31").is_err());
        insert("pet.b", "2026-10-31").expect("other pet same day");
        insert("pet.a", "2026-11-01").expect("same pet next day");
        assert!(insert("pet.a", "31/10/2026").is_err());
    }

    #[test]
    fn upgrade_from_version_ten_backs_up_then_migrates_and_reopens() {
        let directory = tempfile::tempdir().expect("temp dir");
        {
            let connection =
                Connection::open(directory.path().join(crate::DATABASE_FILE)).expect("open");
            Core::migrate_to(&connection, 10).expect("version 10 schema");
            connection
                .execute(
                    "INSERT INTO settings(key, value, updated_at) VALUES ('marker', 'kept', 'now')",
                    [],
                )
                .expect("seed row");
        }

        let core = Core::open(directory.path()).expect("upgrade opens");
        assert_eq!(core.companion_preferences().expect("read"), defaults());
        drop(core);

        let backups: Vec<PathBuf> = fs::read_dir(directory.path().join(BACKUP_DIR))
            .expect("backup dir")
            .map(|entry| entry.expect("entry").path())
            .collect();
        assert_eq!(backups.len(), 1);
        let copy = Connection::open(&backups[0]).expect("open backup");
        assert_eq!(stored_schema_version(&copy).expect("version"), 10);
        let marker: String = copy
            .query_row(
                "SELECT value FROM settings WHERE key = 'marker'",
                [],
                |row| row.get(0),
            )
            .expect("seeded row in backup");
        assert_eq!(marker, "kept");

        // Already current: reopening neither migrates nor backs up again.
        Core::open(directory.path()).expect("reopen");
        assert_eq!(
            fs::read_dir(directory.path().join(BACKUP_DIR))
                .expect("backup dir")
                .count(),
            1
        );
    }

    #[test]
    fn backups_keep_only_the_newest_three() {
        let directory = tempfile::tempdir().expect("temp dir");
        let backup_dir = directory.path().join(BACKUP_DIR);
        fs::create_dir_all(&backup_dir).expect("dir");
        for stamp in [
            "20200101T000000.000Z",
            "20210101T000000.000Z",
            "20220101T000000.000Z",
        ] {
            fs::write(
                backup_dir.join(format!("{BACKUP_PREFIX}{stamp}-v9{BACKUP_SUFFIX}")),
                b"old",
            )
            .expect("seed");
        }
        fs::write(backup_dir.join("unrelated.txt"), b"keep").expect("seed");

        let connection = Connection::open_in_memory().expect("memory db");
        connection
            .execute_batch("CREATE TABLE t(x); INSERT INTO t VALUES (1);")
            .expect("seed");
        backup_before_migration(&connection, directory.path(), 10).expect("backup");

        let mut names: Vec<String> = fs::read_dir(&backup_dir)
            .expect("dir")
            .map(|entry| {
                entry
                    .expect("entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        names.sort();
        assert_eq!(names.len(), 4);
        assert!(!names.iter().any(|name| name.contains("20200101")));
        assert!(names.contains(&"unrelated.txt".to_owned()));
    }
}

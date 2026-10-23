//! Avatar selection, customization, and custom avatar storage.
//!
//! Three kinds of avatars exist:
//! - Catalog characters bundled with the app, optionally customized. A
//!   customization is a complete validated [`CharacterSpec`] stored per id.
//! - Custom procedural characters created by the user (saved as a new
//!   character or styled from an image).
//! - Custom model avatars: image sculpts (binary glTF generated locally) and
//!   imported VRM files. Model files live in `<avatars_root>/<avatar_id>/` and
//!   their SHA-256 is recorded so tampering is detected before loading.

use std::{
    fs,
    path::{Path, PathBuf},
};

use agentpet_avatar::{
    catalog::DEFAULT_AVATAR_ID, sculpt_image, spec::validate_id, AvatarCatalog, CharacterSpec,
    ImageSculptLimits,
};
use agentpet_pet_pack::vrm::{validate_and_extract_vrm, VrmGuardrails, DEFAULT_MAX_VRM_FILE_BYTES};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{Core, CoreError};

pub(crate) const AVATAR_SCHEMA_V10: &str = "
CREATE TABLE avatar_selection (
    singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
    source TEXT NOT NULL CHECK(source IN ('catalog', 'custom')),
    avatar_id TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE TABLE avatar_customizations (
    catalog_id TEXT PRIMARY KEY NOT NULL,
    spec_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE TABLE custom_avatars (
    avatar_id TEXT PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL CHECK(kind IN ('procedural', 'image_sculpt', 'vrm')),
    name TEXT NOT NULL,
    spec_json TEXT,
    model_file TEXT,
    model_sha256 TEXT,
    triangle_count INTEGER,
    metadata_json TEXT,
    source_avatar_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK((kind = 'procedural') = (spec_json IS NOT NULL)),
    CHECK((kind = 'procedural') = (model_file IS NULL)),
    CHECK((model_file IS NULL) = (model_sha256 IS NULL))
);
CREATE INDEX custom_avatars_created_idx ON custom_avatars(created_at);
";

const SCULPT_MODEL_FILE: &str = "model.glb";
const VRM_MODEL_FILE: &str = "model.vrm";
const PREVIEW_FILE: &str = "preview.png";
const STAGING_PREFIX: &str = ".staging-";
/// Upper bound on stored custom avatars, keeping the library bounded.
pub const MAX_CUSTOM_AVATARS: i64 = 200;
const MAX_AVATAR_NAME_CHARS: usize = 32;
const MAX_PREVIEW_BYTES: u64 = 4 * 1024 * 1024;
const CUSTOM_ID_PREFIX: &str = "user";

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AvatarSource {
    Catalog,
    Custom,
}

impl AvatarSource {
    fn as_str(self) -> &'static str {
        match self {
            Self::Catalog => "catalog",
            Self::Custom => "custom",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "catalog" => Ok(Self::Catalog),
            "custom" => Ok(Self::Custom),
            other => Err(CoreError::InvalidAvatar(format!(
                "unknown avatar source `{other}`"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AvatarKind {
    Procedural,
    ImageSculpt,
    Vrm,
}

impl AvatarKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Procedural => "procedural",
            Self::ImageSculpt => "image_sculpt",
            Self::Vrm => "vrm",
        }
    }

    fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "procedural" => Ok(Self::Procedural),
            "image_sculpt" => Ok(Self::ImageSculpt),
            "vrm" => Ok(Self::Vrm),
            other => Err(CoreError::InvalidAvatar(format!(
                "unknown stored avatar kind `{other}`"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AvatarSelection {
    pub source: AvatarSource,
    pub avatar_id: String,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AvatarCatalogEntry {
    pub spec: CharacterSpec,
    pub customized: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CustomAvatarSummary {
    pub avatar_id: String,
    pub kind: AvatarKind,
    pub name: String,
    pub spec: Option<CharacterSpec>,
    pub model_sha256: Option<String>,
    pub triangle_count: Option<i64>,
    pub metadata: Option<serde_json::Value>,
    pub source_avatar_id: Option<String>,
    pub created_at: String,
}

/// The avatar the renderer should show right now.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedAvatar {
    pub source: AvatarSource,
    pub avatar_id: String,
    pub kind: AvatarKind,
    pub name: String,
    pub spec: Option<CharacterSpec>,
    pub customized: bool,
    pub model_sha256: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

fn catalog() -> Result<&'static AvatarCatalog, CoreError> {
    AvatarCatalog::bundled().map_err(|error| CoreError::InvalidAvatar(error.to_string()))
}

fn parse_spec(json: &str) -> Result<CharacterSpec, CoreError> {
    CharacterSpec::parse(json.as_bytes())
        .map_err(|error| CoreError::InvalidAvatar(error.to_string()))
}

fn validate_spec(spec: &CharacterSpec) -> Result<(), CoreError> {
    spec.validate()
        .map_err(|error| CoreError::InvalidAvatar(error.to_string()))
}

fn validate_avatar_id(id: &str) -> Result<(), CoreError> {
    validate_id(id).map_err(|error| CoreError::InvalidAvatar(error.to_string()))
}

fn normalize_name(name: &str) -> Result<String, CoreError> {
    let trimmed = name.trim();
    let count = trimmed.chars().count();
    if count == 0 || count > MAX_AVATAR_NAME_CHARS || trimmed.chars().any(char::is_control) {
        return Err(CoreError::InvalidAvatar(format!(
            "name must be 1-{MAX_AVATAR_NAME_CHARS} printable characters"
        )));
    }
    Ok(trimmed.to_owned())
}

fn new_custom_id(kind: &str) -> String {
    let random = Uuid::new_v4().simple().to_string();
    format!("{CUSTOM_ID_PREFIX}.{kind}-{}", &random[..16])
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn storage_error(context: &str, error: impl std::fmt::Display) -> CoreError {
    CoreError::AvatarStorage(format!("{context}: {error}"))
}

/// Writes `files` into a staging directory and atomically renames it to
/// `<avatars_root>/<avatar_id>`.
fn stage_avatar_directory(
    avatars_root: &Path,
    avatar_id: &str,
    files: &[(&str, &[u8])],
) -> Result<PathBuf, CoreError> {
    fs::create_dir_all(avatars_root).map_err(|e| storage_error("create avatar library", e))?;
    let staging = avatars_root.join(format!("{STAGING_PREFIX}{}", Uuid::new_v4().simple()));
    fs::create_dir(&staging).map_err(|e| storage_error("create staging directory", e))?;
    let target = avatars_root.join(avatar_id);
    let result = (|| {
        for (name, bytes) in files {
            fs::write(staging.join(name), bytes)
                .map_err(|e| storage_error("write avatar file", e))?;
        }
        if target.exists() {
            return Err(CoreError::AvatarStorage(
                "avatar directory already exists".into(),
            ));
        }
        fs::rename(&staging, &target).map_err(|e| storage_error("activate avatar directory", e))
    })();
    if let Err(primary) = result {
        return Err(match fs::remove_dir_all(&staging) {
            Ok(()) => primary,
            Err(cleanup) if cleanup.kind() == std::io::ErrorKind::NotFound => primary,
            Err(cleanup) => CoreError::AvatarStorage(format!(
                "{primary}; staging cleanup also failed: {cleanup}"
            )),
        });
    }
    Ok(target)
}

fn remove_directory(directory: &Path, primary: CoreError) -> CoreError {
    match fs::remove_dir_all(directory) {
        Ok(()) => primary,
        Err(cleanup) if cleanup.kind() == std::io::ErrorKind::NotFound => primary,
        Err(cleanup) => CoreError::AvatarStorage(format!(
            "{primary}; avatar files could not be rolled back: {cleanup}"
        )),
    }
}

/// Reads a regular file inside `<avatars_root>/<avatar_id>` with a size bound,
/// refusing paths that escape the avatar directory.
fn read_avatar_file(
    avatars_root: &Path,
    avatar_id: &str,
    file_name: &str,
    max_bytes: u64,
) -> Result<Option<Vec<u8>>, CoreError> {
    let directory = avatars_root.join(avatar_id);
    let path = directory.join(file_name);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(storage_error("inspect avatar file", error)),
    };
    if !metadata.file_type().is_file() {
        return Err(CoreError::AvatarStorage(
            "avatar file is not a regular file".into(),
        ));
    }
    if metadata.len() > max_bytes {
        return Err(CoreError::AvatarStorage(
            "avatar file exceeds the size limit".into(),
        ));
    }
    let root =
        fs::canonicalize(&directory).map_err(|e| storage_error("resolve avatar directory", e))?;
    let resolved = fs::canonicalize(&path).map_err(|e| storage_error("resolve avatar file", e))?;
    if !resolved.starts_with(&root) {
        return Err(CoreError::AvatarStorage(
            "avatar file escaped its directory".into(),
        ));
    }
    fs::read(resolved)
        .map(Some)
        .map_err(|e| storage_error("read avatar file", e))
}

struct CustomRow {
    summary: CustomAvatarSummary,
    model_file: Option<String>,
}

const CUSTOM_COLUMNS: &str = "avatar_id, kind, name, spec_json, model_file, model_sha256, triangle_count, metadata_json, source_avatar_id, created_at";

fn custom_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CustomRawRow> {
    Ok(CustomRawRow {
        avatar_id: row.get(0)?,
        kind: row.get(1)?,
        name: row.get(2)?,
        spec_json: row.get(3)?,
        model_file: row.get(4)?,
        model_sha256: row.get(5)?,
        triangle_count: row.get(6)?,
        metadata_json: row.get(7)?,
        source_avatar_id: row.get(8)?,
        created_at: row.get(9)?,
    })
}

struct CustomRawRow {
    avatar_id: String,
    kind: String,
    name: String,
    spec_json: Option<String>,
    model_file: Option<String>,
    model_sha256: Option<String>,
    triangle_count: Option<i64>,
    metadata_json: Option<String>,
    source_avatar_id: Option<String>,
    created_at: String,
}

impl CustomRawRow {
    fn into_row(self) -> Result<CustomRow, CoreError> {
        let spec = self.spec_json.as_deref().map(parse_spec).transpose()?;
        let metadata = self
            .metadata_json
            .as_deref()
            .map(serde_json::from_str)
            .transpose()?;
        Ok(CustomRow {
            summary: CustomAvatarSummary {
                avatar_id: self.avatar_id,
                kind: AvatarKind::parse(&self.kind)?,
                name: self.name,
                spec,
                model_sha256: self.model_sha256,
                triangle_count: self.triangle_count,
                metadata,
                source_avatar_id: self.source_avatar_id,
                created_at: self.created_at,
            },
            model_file: self.model_file,
        })
    }
}

fn load_custom(connection: &Connection, avatar_id: &str) -> Result<Option<CustomRow>, CoreError> {
    connection
        .query_row(
            &format!("SELECT {CUSTOM_COLUMNS} FROM custom_avatars WHERE avatar_id = ?1"),
            [avatar_id],
            custom_row,
        )
        .optional()?
        .map(CustomRawRow::into_row)
        .transpose()
}

fn load_customization(
    connection: &Connection,
    catalog_id: &str,
) -> Result<Option<CharacterSpec>, CoreError> {
    connection
        .query_row(
            "SELECT spec_json FROM avatar_customizations WHERE catalog_id = ?1",
            [catalog_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .map(|json| parse_spec(&json))
        .transpose()
}

fn ensure_capacity(connection: &Connection, additional: i64) -> Result<(), CoreError> {
    let count: i64 =
        connection.query_row("SELECT COUNT(*) FROM custom_avatars", [], |row| row.get(0))?;
    if count + additional > MAX_CUSTOM_AVATARS {
        return Err(CoreError::InvalidAvatar(format!(
            "the custom avatar library is limited to {MAX_CUSTOM_AVATARS} entries; delete one first"
        )));
    }
    Ok(())
}

struct NewCustomAvatar<'a> {
    avatar_id: &'a str,
    kind: AvatarKind,
    name: &'a str,
    spec_json: Option<String>,
    model_file: Option<&'a str>,
    model_sha256: Option<String>,
    triangle_count: Option<i64>,
    metadata_json: Option<String>,
    source_avatar_id: Option<&'a str>,
}

fn insert_custom(connection: &Connection, avatar: &NewCustomAvatar<'_>) -> Result<(), CoreError> {
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "INSERT INTO custom_avatars(avatar_id, kind, name, spec_json, model_file, model_sha256, triangle_count, metadata_json, source_avatar_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
        params![
            avatar.avatar_id,
            avatar.kind.as_str(),
            avatar.name,
            avatar.spec_json,
            avatar.model_file,
            avatar.model_sha256,
            avatar.triangle_count,
            avatar.metadata_json,
            avatar.source_avatar_id,
            now
        ],
    )?;
    Ok(())
}

impl Core {
    fn lock_connection(&self) -> Result<std::sync::MutexGuard<'_, Connection>, CoreError> {
        self.connection.lock().map_err(|_| CoreError::LockPoisoned)
    }

    /// Returns the persisted selection, or the bundled default.
    pub fn avatar_selection(&self) -> Result<AvatarSelection, CoreError> {
        let connection = self.lock_connection()?;
        let row: Option<(String, String, String)> = connection
            .query_row(
                "SELECT source, avatar_id, updated_at FROM avatar_selection WHERE singleton = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        match row {
            Some((source, avatar_id, updated_at)) => Ok(AvatarSelection {
                source: AvatarSource::parse(&source)?,
                avatar_id,
                updated_at: Some(updated_at),
            }),
            None => Ok(AvatarSelection {
                source: AvatarSource::Catalog,
                avatar_id: DEFAULT_AVATAR_ID.to_owned(),
                updated_at: None,
            }),
        }
    }

    /// Resolves the selection into what the renderer needs. A selection that
    /// points at a deleted custom avatar falls back to the default character.
    pub fn resolve_avatar(&self) -> Result<ResolvedAvatar, CoreError> {
        let selection = self.avatar_selection()?;
        let connection = self.lock_connection()?;
        if selection.source == AvatarSource::Custom {
            if let Some(row) = load_custom(&connection, &selection.avatar_id)? {
                let summary = row.summary;
                return Ok(ResolvedAvatar {
                    source: AvatarSource::Custom,
                    avatar_id: summary.avatar_id,
                    kind: summary.kind,
                    name: summary.name,
                    spec: summary.spec,
                    customized: false,
                    model_sha256: summary.model_sha256,
                    metadata: summary.metadata,
                });
            }
        }
        let catalog = catalog()?;
        let base = catalog
            .get(&selection.avatar_id)
            .unwrap_or_else(|| catalog.default_character());
        let customization = load_customization(&connection, &base.id)?;
        let customized = customization.is_some();
        let spec = customization.unwrap_or_else(|| base.clone());
        Ok(ResolvedAvatar {
            source: AvatarSource::Catalog,
            avatar_id: base.id.clone(),
            kind: AvatarKind::Procedural,
            name: spec.name.clone(),
            spec: Some(spec),
            customized,
            model_sha256: None,
            metadata: None,
        })
    }

    pub fn select_avatar(&self, source: AvatarSource, avatar_id: &str) -> Result<(), CoreError> {
        validate_avatar_id(avatar_id)?;
        let connection = self.lock_connection()?;
        let exists = match source {
            AvatarSource::Catalog => catalog()?.contains(avatar_id),
            AvatarSource::Custom => connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM custom_avatars WHERE avatar_id = ?1)",
                [avatar_id],
                |row| row.get::<_, bool>(0),
            )?,
        };
        if !exists {
            return Err(CoreError::AvatarNotFound(avatar_id.to_owned()));
        }
        connection.execute(
            "INSERT INTO avatar_selection(singleton, source, avatar_id, updated_at) VALUES (1, ?1, ?2, ?3)
             ON CONFLICT(singleton) DO UPDATE SET source = excluded.source, avatar_id = excluded.avatar_id, updated_at = excluded.updated_at",
            params![source.as_str(), avatar_id, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    /// Lists every bundled character with any saved customization applied.
    pub fn list_avatar_catalog(&self) -> Result<Vec<AvatarCatalogEntry>, CoreError> {
        let connection = self.lock_connection()?;
        let mut statement =
            connection.prepare("SELECT catalog_id, spec_json FROM avatar_customizations")?;
        let mut customizations = std::collections::HashMap::new();
        for row in statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })? {
            let (id, json) = row?;
            customizations.insert(id, parse_spec(&json)?);
        }
        Ok(catalog()?
            .characters
            .iter()
            .map(|base| match customizations.remove(&base.id) {
                Some(spec) => AvatarCatalogEntry {
                    spec,
                    customized: true,
                },
                None => AvatarCatalogEntry {
                    spec: base.clone(),
                    customized: false,
                },
            })
            .collect())
    }

    /// Saves a customized spec. Catalog ids store a customization; custom
    /// procedural ids are updated in place. The id itself cannot change.
    pub fn save_avatar_customization(&self, spec: &CharacterSpec) -> Result<(), CoreError> {
        validate_spec(spec)?;
        let connection = self.lock_connection()?;
        let now = Utc::now().to_rfc3339();
        let json = serde_json::to_string(spec)?;
        if catalog()?.contains(&spec.id) {
            connection.execute(
                "INSERT INTO avatar_customizations(catalog_id, spec_json, updated_at) VALUES (?1, ?2, ?3)
                 ON CONFLICT(catalog_id) DO UPDATE SET spec_json = excluded.spec_json, updated_at = excluded.updated_at",
                params![spec.id, json, now],
            )?;
            return Ok(());
        }
        let updated = connection.execute(
            "UPDATE custom_avatars SET spec_json = ?2, name = ?3, updated_at = ?4
             WHERE avatar_id = ?1 AND kind = 'procedural'",
            params![spec.id, json, spec.name, now],
        )?;
        if updated == 0 {
            return Err(CoreError::AvatarNotFound(spec.id.clone()));
        }
        Ok(())
    }

    /// Removes a catalog customization. Returns whether one existed.
    pub fn reset_avatar_customization(&self, catalog_id: &str) -> Result<bool, CoreError> {
        validate_avatar_id(catalog_id)?;
        let connection = self.lock_connection()?;
        let removed = connection.execute(
            "DELETE FROM avatar_customizations WHERE catalog_id = ?1",
            [catalog_id],
        )?;
        Ok(removed > 0)
    }

    /// Stores `spec` as a new custom procedural character under a fresh id.
    pub fn create_custom_character(
        &self,
        spec: &CharacterSpec,
    ) -> Result<CustomAvatarSummary, CoreError> {
        let mut spec = spec.clone();
        spec.id = new_custom_id("custom");
        validate_spec(&spec)?;
        let connection = self.lock_connection()?;
        ensure_capacity(&connection, 1)?;
        insert_custom(
            &connection,
            &NewCustomAvatar {
                avatar_id: &spec.id,
                kind: AvatarKind::Procedural,
                name: &spec.name,
                spec_json: Some(serde_json::to_string(&spec)?),
                model_file: None,
                model_sha256: None,
                triangle_count: None,
                metadata_json: None,
                source_avatar_id: None,
            },
        )?;
        load_custom(&connection, &spec.id)?
            .map(|row| row.summary)
            .ok_or_else(|| CoreError::AvatarNotFound(spec.id.clone()))
    }

    pub fn list_custom_avatars(&self) -> Result<Vec<CustomAvatarSummary>, CoreError> {
        let connection = self.lock_connection()?;
        let mut statement = connection.prepare(&format!(
            "SELECT {CUSTOM_COLUMNS} FROM custom_avatars ORDER BY created_at DESC, avatar_id"
        ))?;
        let rows = statement.query_map([], custom_row)?;
        let mut avatars = Vec::new();
        for row in rows {
            avatars.push(row?.into_row()?.summary);
        }
        Ok(avatars)
    }

    /// Sculpts a user image into a 3D relief avatar and a palette-styled
    /// procedural character. Both are stored; the sculpt is returned first.
    /// This decodes and meshes the image, so call it off the UI thread.
    pub fn create_avatars_from_image(
        &self,
        image_path: &Path,
        avatars_root: &Path,
        name: &str,
    ) -> Result<Vec<CustomAvatarSummary>, CoreError> {
        let name = normalize_name(name)?;
        {
            let connection = self.lock_connection()?;
            ensure_capacity(&connection, 2)?;
        }
        let sculpt_id = new_custom_id("sculpt");
        let style_id = new_custom_id("style");
        let output = sculpt_image(image_path, &style_id, &name, ImageSculptLimits::default())
            .map_err(|error| CoreError::InvalidAvatar(error.to_string()))?;
        let directory = stage_avatar_directory(
            avatars_root,
            &sculpt_id,
            &[
                (SCULPT_MODEL_FILE, &output.glb),
                (PREVIEW_FILE, &output.preview_png),
            ],
        )?;
        let record = (|| {
            let mut connection = self.lock_connection()?;
            ensure_capacity(&connection, 2)?;
            let transaction = connection.transaction()?;
            insert_custom(
                &transaction,
                &NewCustomAvatar {
                    avatar_id: &sculpt_id,
                    kind: AvatarKind::ImageSculpt,
                    name: &name,
                    spec_json: None,
                    model_file: Some(SCULPT_MODEL_FILE),
                    model_sha256: Some(sha256_hex(&output.glb)),
                    triangle_count: Some(output.stats.triangle_count as i64),
                    metadata_json: Some(serde_json::to_string(&output.stats)?),
                    source_avatar_id: None,
                },
            )?;
            insert_custom(
                &transaction,
                &NewCustomAvatar {
                    avatar_id: &style_id,
                    kind: AvatarKind::Procedural,
                    name: &name,
                    spec_json: Some(serde_json::to_string(&output.spec)?),
                    model_file: None,
                    model_sha256: None,
                    triangle_count: None,
                    metadata_json: None,
                    source_avatar_id: Some(&sculpt_id),
                },
            )?;
            transaction.commit()?;
            let sculpt = load_custom(&connection, &sculpt_id)?;
            let style = load_custom(&connection, &style_id)?;
            match (sculpt, style) {
                (Some(sculpt), Some(style)) => Ok(vec![sculpt.summary, style.summary]),
                _ => Err(CoreError::AvatarNotFound(sculpt_id.clone())),
            }
        })();
        record.map_err(|primary| remove_directory(&directory, primary))
    }

    /// Validates and imports a VRM (or plain glTF binary) avatar file.
    pub fn import_vrm_avatar(
        &self,
        vrm_path: &Path,
        avatars_root: &Path,
    ) -> Result<CustomAvatarSummary, CoreError> {
        {
            let connection = self.lock_connection()?;
            ensure_capacity(&connection, 1)?;
        }
        // Read the file once, then validate the staged copy: the bytes that are
        // hashed and stored are exactly the bytes that passed validation.
        let size = fs::symlink_metadata(vrm_path)
            .map_err(|e| storage_error("inspect VRM file", e))?
            .len();
        if size == 0 || size > DEFAULT_MAX_VRM_FILE_BYTES {
            return Err(CoreError::InvalidAvatar(
                "VRM file must be between 1 byte and the 50 MB size limit".into(),
            ));
        }
        let bytes = fs::read(vrm_path).map_err(|e| storage_error("read VRM file", e))?;
        if bytes.is_empty() || bytes.len() as u64 > DEFAULT_MAX_VRM_FILE_BYTES {
            return Err(CoreError::InvalidAvatar(
                "VRM file changed size while it was read".into(),
            ));
        }
        let avatar_id = new_custom_id("vrm");
        let directory =
            stage_avatar_directory(avatars_root, &avatar_id, &[(VRM_MODEL_FILE, &bytes)])?;
        let metadata = match validate_and_extract_vrm(
            &directory.join(VRM_MODEL_FILE),
            VrmGuardrails::default(),
        ) {
            Ok(metadata) => metadata,
            Err(error) => return Err(remove_directory(&directory, error.into())),
        };
        let title: String = metadata
            .title
            .trim()
            .chars()
            .take(MAX_AVATAR_NAME_CHARS)
            .collect();
        let name = if title.trim().is_empty() || title.chars().any(char::is_control) {
            "Imported VRM".to_owned()
        } else {
            title
        };
        let record = (|| {
            let connection = self.lock_connection()?;
            ensure_capacity(&connection, 1)?;
            insert_custom(
                &connection,
                &NewCustomAvatar {
                    avatar_id: &avatar_id,
                    kind: AvatarKind::Vrm,
                    name: &name,
                    spec_json: None,
                    model_file: Some(VRM_MODEL_FILE),
                    model_sha256: Some(sha256_hex(&bytes)),
                    triangle_count: Some(
                        i64::try_from(metadata.triangle_count).unwrap_or(i64::MAX),
                    ),
                    metadata_json: Some(serde_json::to_string(&metadata)?),
                    source_avatar_id: None,
                },
            )?;
            load_custom(&connection, &avatar_id)?
                .map(|row| row.summary)
                .ok_or_else(|| CoreError::AvatarNotFound(avatar_id.clone()))
        })();
        record.map_err(|primary| remove_directory(&directory, primary))
    }

    /// Deletes a custom avatar and its files. If it was selected, the
    /// selection returns to the default character.
    pub fn delete_custom_avatar(
        &self,
        avatar_id: &str,
        avatars_root: &Path,
    ) -> Result<(), CoreError> {
        validate_avatar_id(avatar_id)?;
        let mut connection = self.lock_connection()?;
        let row = load_custom(&connection, avatar_id)?
            .ok_or_else(|| CoreError::AvatarNotFound(avatar_id.to_owned()))?;
        let target_dir = avatars_root.join(avatar_id);
        let staged_dir = if row.model_file.is_some() && target_dir.exists() {
            let staging = avatars_root.join(format!("{STAGING_PREFIX}{}", Uuid::new_v4().simple()));
            fs::rename(&target_dir, &staging)
                .map_err(|e| storage_error("stage avatar directory for deletion", e))?;
            Some(staging)
        } else {
            None
        };

        let commit_result = (|| -> Result<(), CoreError> {
            let transaction = connection.transaction()?;
            transaction.execute(
                "DELETE FROM custom_avatars WHERE avatar_id = ?1",
                [avatar_id],
            )?;
            transaction.execute(
                "UPDATE avatar_selection SET source = 'catalog', avatar_id = ?2, updated_at = ?3
                 WHERE singleton = 1 AND source = 'custom' AND avatar_id = ?1",
                params![avatar_id, DEFAULT_AVATAR_ID, Utc::now().to_rfc3339()],
            )?;
            transaction.commit()?;
            Ok(())
        })();

        if let Err(commit_error) = commit_result {
            if let Some(ref staging) = staged_dir {
                let _ = fs::rename(staging, &target_dir);
            }
            return Err(commit_error);
        }

        if let Some(ref staging) = staged_dir {
            let _ = fs::remove_dir_all(staging);
        }

        Ok(())
    }

    /// Returns the model file of a custom avatar after verifying its hash.
    pub fn custom_avatar_model(
        &self,
        avatar_id: &str,
        avatars_root: &Path,
    ) -> Result<Vec<u8>, CoreError> {
        validate_avatar_id(avatar_id)?;
        let row = {
            let connection = self.lock_connection()?;
            load_custom(&connection, avatar_id)?
                .ok_or_else(|| CoreError::AvatarNotFound(avatar_id.to_owned()))?
        };
        let (Some(model_file), Some(expected)) = (row.model_file, row.summary.model_sha256) else {
            return Err(CoreError::InvalidAvatar(
                "this avatar has no model file".into(),
            ));
        };
        let bytes = read_avatar_file(
            avatars_root,
            avatar_id,
            &model_file,
            DEFAULT_MAX_VRM_FILE_BYTES,
        )?
        .ok_or_else(|| CoreError::AvatarStorage("avatar model file is missing".into()))?;
        if sha256_hex(&bytes) != expected {
            return Err(CoreError::AvatarStorage(
                "avatar model file does not match its recorded checksum".into(),
            ));
        }
        Ok(bytes)
    }

    /// Removes staging directories left behind by an interrupted import.
    /// Returns how many were removed. Call once at startup, before imports.
    pub fn sweep_avatar_staging(&self, avatars_root: &Path) -> Result<usize, CoreError> {
        let entries = match fs::read_dir(avatars_root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
            Err(error) => return Err(storage_error("list avatar library", error)),
        };
        let mut removed = 0;
        for entry in entries {
            let entry = entry.map_err(|e| storage_error("list avatar library", e))?;
            let is_staging = entry
                .file_name()
                .to_string_lossy()
                .starts_with(STAGING_PREFIX);
            let is_directory = entry
                .file_type()
                .map_err(|e| storage_error("inspect avatar library entry", e))?
                .is_dir();
            if is_staging && is_directory {
                fs::remove_dir_all(entry.path())
                    .map_err(|e| storage_error("remove stale staging directory", e))?;
                removed += 1;
            }
        }
        Ok(removed)
    }

    /// Returns the cut-out preview PNG of an image sculpt, when present.
    pub fn custom_avatar_preview(
        &self,
        avatar_id: &str,
        avatars_root: &Path,
    ) -> Result<Option<Vec<u8>>, CoreError> {
        validate_avatar_id(avatar_id)?;
        read_avatar_file(avatars_root, avatar_id, PREVIEW_FILE, MAX_PREVIEW_BYTES)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn disk_png(directory: &Path) -> PathBuf {
        let path = directory.join("subject.png");
        image::RgbaImage::from_fn(96, 96, |x, y| {
            let dx = x as f32 - 48.0;
            let dy = y as f32 - 48.0;
            if dx * dx + dy * dy < 30.0 * 30.0 {
                image::Rgba([40, 90, 200, 255])
            } else {
                image::Rgba([255, 255, 255, 255])
            }
        })
        .save(&path)
        .expect("write png");
        path
    }

    #[test]
    fn migration_creates_avatar_tables_from_version_nine() {
        let connection = Connection::open_in_memory().expect("open");
        Core::migrate_to(&connection, 9).expect("migrate to 9");
        let before: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name = 'custom_avatars')",
                [],
                |row| row.get(0),
            )
            .expect("query");
        assert!(!before);
        Core::migrate_to(&connection, 10).expect("migrate to 10");
        for table in [
            "avatar_selection",
            "avatar_customizations",
            "custom_avatars",
        ] {
            let exists: bool = connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
                    [table],
                    |row| row.get(0),
                )
                .expect("query");
            assert!(exists, "{table} exists");
        }
        let version: i64 = connection
            .query_row("SELECT MAX(version) FROM schema_versions", [], |row| {
                row.get(0)
            })
            .expect("version");
        assert_eq!(version, 10);
    }

    #[test]
    fn defaults_selects_and_rejects_unknown_avatars() {
        let core = Core::in_memory().expect("core");
        let resolved = core.resolve_avatar().expect("resolve");
        assert_eq!(resolved.avatar_id, DEFAULT_AVATAR_ID);
        assert_eq!(resolved.kind, AvatarKind::Procedural);
        assert!(resolved.spec.is_some());

        core.select_avatar(AvatarSource::Catalog, "miyora.ember")
            .expect("select");
        assert_eq!(
            core.resolve_avatar().expect("resolve").avatar_id,
            "miyora.ember"
        );

        assert!(matches!(
            core.select_avatar(AvatarSource::Catalog, "miyora.does-not-exist"),
            Err(CoreError::AvatarNotFound(_))
        ));
        assert!(matches!(
            core.select_avatar(AvatarSource::Custom, "user.custom-missing"),
            Err(CoreError::AvatarNotFound(_))
        ));
        assert!(core
            .select_avatar(AvatarSource::Catalog, "../escape")
            .is_err());
    }

    #[test]
    fn customization_is_validated_listed_resolved_and_reset() {
        let core = Core::in_memory().expect("core");
        let mut spec = catalog()
            .expect("catalog")
            .get("miyora.ember")
            .expect("ember")
            .clone();
        spec.palette.hair = "#123456".into();
        spec.name = "Blaze".into();
        core.save_avatar_customization(&spec).expect("save");
        core.select_avatar(AvatarSource::Catalog, "miyora.ember")
            .expect("select");
        let resolved = core.resolve_avatar().expect("resolve");
        assert!(resolved.customized);
        assert_eq!(resolved.name, "Blaze");
        assert_eq!(resolved.spec.expect("spec").palette.hair, "#123456");

        let listed = core.list_avatar_catalog().expect("list");
        assert!(listed.len() >= 100);
        assert!(listed
            .iter()
            .any(|entry| entry.customized && entry.spec.id == "miyora.ember"));

        let mut invalid = spec.clone();
        invalid.palette.hair = "blue".into();
        assert!(matches!(
            core.save_avatar_customization(&invalid),
            Err(CoreError::InvalidAvatar(_))
        ));
        let mut unknown = spec.clone();
        unknown.id = "user.custom-unknown".into();
        assert!(matches!(
            core.save_avatar_customization(&unknown),
            Err(CoreError::AvatarNotFound(_))
        ));

        assert!(core
            .reset_avatar_customization("miyora.ember")
            .expect("reset"));
        assert!(!core
            .reset_avatar_customization("miyora.ember")
            .expect("reset twice"));
        assert!(!core.resolve_avatar().expect("resolve").customized);
    }

    #[test]
    fn custom_characters_can_be_created_edited_and_deleted() {
        let core = Core::in_memory().expect("core");
        let library = tempfile::tempdir().expect("tempdir");
        let base = catalog()
            .expect("catalog")
            .get("miyora.bolt")
            .expect("bolt")
            .clone();
        let created = core.create_custom_character(&base).expect("create");
        assert!(created.avatar_id.starts_with("user.custom-"));
        assert_eq!(created.kind, AvatarKind::Procedural);

        let mut edited = created.spec.clone().expect("spec");
        edited.name = "Bolt Mk II".into();
        core.save_avatar_customization(&edited)
            .expect("edit custom");
        core.select_avatar(AvatarSource::Custom, &created.avatar_id)
            .expect("select");
        assert_eq!(core.resolve_avatar().expect("resolve").name, "Bolt Mk II");

        core.delete_custom_avatar(&created.avatar_id, library.path())
            .expect("delete");
        assert_eq!(
            core.resolve_avatar().expect("resolve").avatar_id,
            DEFAULT_AVATAR_ID
        );
        assert!(core.list_custom_avatars().expect("list").is_empty());
    }

    #[test]
    fn image_import_creates_verified_sculpt_and_styled_character() {
        let core = Core::in_memory().expect("core");
        let workspace = tempfile::tempdir().expect("tempdir");
        let library = workspace.path().join("avatars");
        let image = disk_png(workspace.path());

        let created = core
            .create_avatars_from_image(&image, &library, "  Blue Orb  ")
            .expect("import");
        assert_eq!(created.len(), 2);
        let sculpt = &created[0];
        let style = &created[1];
        assert_eq!(sculpt.kind, AvatarKind::ImageSculpt);
        assert_eq!(sculpt.name, "Blue Orb");
        assert!(sculpt.triangle_count.expect("triangles") > 0);
        assert_eq!(style.kind, AvatarKind::Procedural);
        assert_eq!(
            style.source_avatar_id.as_deref(),
            Some(sculpt.avatar_id.as_str())
        );
        assert_eq!(
            style.spec.as_ref().expect("spec").palette.primary,
            "#285ac8"
        );

        let model = core
            .custom_avatar_model(&sculpt.avatar_id, &library)
            .expect("model");
        assert_eq!(&model[..4], b"glTF");
        assert!(core
            .custom_avatar_preview(&sculpt.avatar_id, &library)
            .expect("preview")
            .is_some());
        assert!(core
            .custom_avatar_model(&style.avatar_id, &library)
            .is_err());

        core.select_avatar(AvatarSource::Custom, &sculpt.avatar_id)
            .expect("select");
        let resolved = core.resolve_avatar().expect("resolve");
        assert_eq!(resolved.kind, AvatarKind::ImageSculpt);
        assert_eq!(resolved.model_sha256, sculpt.model_sha256);

        // Tampering with the stored model is detected.
        fs::write(
            library.join(&sculpt.avatar_id).join(SCULPT_MODEL_FILE),
            b"glTF tampered",
        )
        .expect("tamper");
        assert!(matches!(
            core.custom_avatar_model(&sculpt.avatar_id, &library),
            Err(CoreError::AvatarStorage(_))
        ));

        core.delete_custom_avatar(&sculpt.avatar_id, &library)
            .expect("delete");
        assert!(!library.join(&sculpt.avatar_id).exists());
        assert_eq!(
            core.resolve_avatar().expect("resolve").avatar_id,
            DEFAULT_AVATAR_ID
        );
        assert_eq!(core.list_custom_avatars().expect("list").len(), 1);

        let staging_left = fs::read_dir(&library)
            .expect("read library")
            .flatten()
            .any(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(STAGING_PREFIX)
            });
        assert!(!staging_left, "staging directories are cleaned up");
    }

    #[test]
    fn image_import_rejects_bad_names_and_unreadable_images() {
        let core = Core::in_memory().expect("core");
        let workspace = tempfile::tempdir().expect("tempdir");
        let library = workspace.path().join("avatars");
        let image = disk_png(workspace.path());
        assert!(core
            .create_avatars_from_image(&image, &library, "   ")
            .is_err());
        let bad = workspace.path().join("bad.png");
        fs::write(&bad, b"not an image").expect("write");
        assert!(core
            .create_avatars_from_image(&bad, &library, "Bad")
            .is_err());
        assert!(core.list_custom_avatars().expect("list").is_empty());
    }

    #[test]
    fn sweeps_only_stale_staging_directories() {
        let core = Core::in_memory().expect("core");
        let library = tempfile::tempdir().expect("tempdir");
        assert_eq!(
            core.sweep_avatar_staging(&library.path().join("missing"))
                .expect("sweep"),
            0
        );
        fs::create_dir(library.path().join(format!("{STAGING_PREFIX}abc"))).expect("staging");
        fs::create_dir(library.path().join("user.sculpt-keep")).expect("avatar");
        assert_eq!(core.sweep_avatar_staging(library.path()).expect("sweep"), 1);
        assert!(library.path().join("user.sculpt-keep").exists());
    }

    #[test]
    fn vrm_import_rejects_non_gltf_files() {
        let core = Core::in_memory().expect("core");
        let workspace = tempfile::tempdir().expect("tempdir");
        let bogus = workspace.path().join("avatar.vrm");
        fs::write(&bogus, b"definitely not a glTF container").expect("write");
        assert!(core
            .import_vrm_avatar(&bogus, &workspace.path().join("avatars"))
            .is_err());
        assert!(core.list_custom_avatars().expect("list").is_empty());
        let leftovers = fs::read_dir(workspace.path().join("avatars"))
            .map(|entries| entries.count())
            .unwrap_or(0);
        assert_eq!(leftovers, 0, "rejected files are removed from the library");
    }
}

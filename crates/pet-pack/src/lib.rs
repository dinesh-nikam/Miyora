use std::{
    collections::BTreeMap,
    fs,
    io::Cursor,
    path::{Component, Path, PathBuf},
};

use image::{ImageFormat, ImageReader};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod animated;
pub mod archive;
pub mod signature;
pub mod vrm;

pub const MANIFEST_FILE: &str = "pet.json";
pub const CURRENT_SCHEMA_VERSION: u32 = 1;
const MAX_MANIFEST_BYTES: u64 = 256 * 1024;
const MAX_PACK_ID_BYTES: usize = 128;
const MAX_NAME_BYTES: usize = 120;
const MAX_DESCRIPTION_BYTES: usize = 2_000;
const MAX_ASSET_PATH_BYTES: usize = 240;
const MAX_ASSETS: usize = 256;
const MAX_ANIMATIONS: usize = 64;
const MAX_FRAMES_PER_ANIMATION: usize = 512;
const MAX_TAGS: usize = 64;
pub(crate) const MAX_IMAGE_DIMENSION: u32 = 4_096;
const MAX_IMAGE_ALLOCATION: u64 = 64 * 1024 * 1024;
const PREVIEW_FILE: &str = "generated/preview.png";
/// Maximum frames packed from an animated user image into one sprite sheet.
const MAX_IMPORTED_ANIMATION_FRAMES: usize = 64;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PetPackManifest {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub creator: String,
    pub license: String,
    pub source: String,
    pub attribution: String,
    pub commercial_use: bool,
    pub redistribution: bool,
    pub derivative_works: bool,
    pub assets: BTreeMap<String, String>,
    pub animations: BTreeMap<String, AnimationDefinition>,
    #[serde(default)]
    pub state_mappings: BTreeMap<String, String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnimationDefinition {
    pub asset: String,
    pub frames: Vec<FrameRect>,
    pub fps: u16,
    #[serde(rename = "loop")]
    pub loop_animation: bool,
    pub anchor: Anchor,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrameRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Anchor {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct ValidationLimits {
    pub max_asset_bytes: u64,
    pub max_total_asset_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPetPack {
    pub manifest: PetPackManifest,
    pub directory: PathBuf,
    /// How the pack entered the library: "folder", "archive", or "image".
    pub install_source: String,
    /// Hex-encoded SHA-256 over `pet.json` and every declared asset file.
    pub content_hash: Option<String>,
    /// Trusted key identifier when a detached signature was verified.
    pub signature_key_id: Option<String>,
}

impl Default for ValidationLimits {
    fn default() -> Self {
        Self {
            max_asset_bytes: 32 * 1024 * 1024,
            max_total_asset_bytes: 128 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Error)]
pub enum PetPackError {
    #[error("manifest file is missing or is not a regular file")]
    ManifestMissing,
    #[error("manifest exceeds the 256 KiB byte limit")]
    ManifestTooLarge,
    #[error("manifest JSON is invalid: {0}")]
    InvalidJson(#[from] serde_json::Error),
    #[error("unsupported PetPack schema version {0}")]
    UnsupportedSchema(u32),
    #[error("invalid PetPack: {0}")]
    Invalid(String),
    #[error("asset is missing or is not a regular file: {0}")]
    AssetMissing(String),
    #[error("asset exceeds per-file size limit: {0}")]
    AssetTooLarge(String),
    #[error("assets exceed total size limit")]
    TotalAssetsTooLarge,
    #[error("this pack version is already installed")]
    AlreadyInstalled,
    #[error("archive exceeds the compressed size limit")]
    ArchiveTooLarge(u64),
    #[error("archive contains more entries than allowed")]
    ArchiveEntryCount(usize),
    #[error("archive entry path is unsafe: {0}")]
    ArchiveEntryPath(String),
    #[error("archive contains an unsupported or executable entry: {0}")]
    ArchiveEntryContent(String),
    #[error("archive entry exceeds the per-file size limit: {0}")]
    ArchiveEntryTooLarge(String),
    #[error("archive entry size does not match its header: {0}")]
    ArchiveEntrySizeMismatch(String),
    #[error("archive contains a duplicate normalized path: {0}")]
    ArchiveDuplicatePath(String),
    #[error("archive total uncompressed size exceeds the limit")]
    ArchiveTotalTooLarge,
    #[error("archive entry has an extreme compression ratio: {0}")]
    ArchiveRatioTooHigh(String),
    #[error("archive format is not recognized: {0}")]
    ArchiveFormat(String),
    #[error("archive could not be read: {0}")]
    ArchiveRead(String),
    #[error("signature file is invalid: {0}")]
    SignatureFormat(String),
    #[error("pack signature is missing but a trusted signature is required")]
    SignatureMissing,
    #[error("pack signature references an untrusted key: {0}")]
    SignatureUntrusted(String),
    #[error("pack signature verification failed")]
    SignatureVerificationFailed,
    #[error("VRM model contains forbidden external URI: {0}")]
    VrmExternalResource(String),
    #[error("VRM model exceeds resource budget: {0}")]
    VrmBudgetExceeded(String),
    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),
    #[error("image is invalid or exceeds decode limits: {0}")]
    Image(#[from] image::ImageError),
}

impl PetPackManifest {
    pub fn parse(bytes: &[u8]) -> Result<Self, PetPackError> {
        if bytes.len() as u64 > MAX_MANIFEST_BYTES {
            return Err(PetPackError::ManifestTooLarge);
        }
        let manifest: Self = serde_json::from_slice(bytes)?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn load_and_validate(
        pack_root: &Path,
        limits: ValidationLimits,
    ) -> Result<Self, PetPackError> {
        let manifest_path = pack_root.join(MANIFEST_FILE);
        let metadata =
            fs::symlink_metadata(&manifest_path).map_err(|_| PetPackError::ManifestMissing)?;
        if !metadata.file_type().is_file() || metadata.len() > MAX_MANIFEST_BYTES {
            return if metadata.len() > MAX_MANIFEST_BYTES {
                Err(PetPackError::ManifestTooLarge)
            } else {
                Err(PetPackError::ManifestMissing)
            };
        }
        let bytes = fs::read(manifest_path)?;
        let manifest = Self::parse(&bytes)?;
        manifest.validate_assets(pack_root, limits)?;
        Ok(manifest)
    }

    pub fn install_extracted(
        source_root: &Path,
        library_root: &Path,
        limits: ValidationLimits,
    ) -> Result<InstalledPetPack, PetPackError> {
        let source_root = fs::canonicalize(source_root)?;
        let manifest = Self::load_and_validate(&source_root, limits)?;
        fs::create_dir_all(library_root)?;
        let library_root = fs::canonicalize(library_root)?;
        let destination = library_root.join(&manifest.id).join(&manifest.version);
        if destination.exists() {
            return Err(PetPackError::AlreadyInstalled);
        }

        let staging = library_root.join(format!(".staging-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&staging)?;
        let result = (|| {
            fs::copy(source_root.join(MANIFEST_FILE), staging.join(MANIFEST_FILE))?;
            for relative_path in manifest.assets.values() {
                let source = source_root.join(relative_path);
                let metadata = fs::symlink_metadata(&source)
                    .map_err(|_| PetPackError::AssetMissing(relative_path.clone()))?;
                if !metadata.file_type().is_file() {
                    return Err(PetPackError::AssetMissing(relative_path.clone()));
                }
                let canonical = fs::canonicalize(&source)?;
                if !canonical.starts_with(&source_root) {
                    return Err(PetPackError::Invalid(format!(
                        "asset escapes pack directory: {relative_path}"
                    )));
                }
                let destination_asset = staging.join(relative_path);
                let parent = destination_asset.parent().ok_or_else(|| {
                    PetPackError::Invalid(format!("invalid asset destination: {relative_path}"))
                })?;
                fs::create_dir_all(parent)?;
                fs::copy(canonical, destination_asset)?;
            }
            let copied_manifest = Self::load_and_validate(&staging, limits)?;
            if copied_manifest != manifest {
                return Err(PetPackError::Invalid(
                    "manifest changed while installing".into(),
                ));
            }
            generate_preview(&staging, &copied_manifest)?;
            let content_hash = compute_content_hash(&staging, &copied_manifest)?;
            fs::create_dir_all(
                destination
                    .parent()
                    .ok_or_else(|| PetPackError::Invalid("invalid library destination".into()))?,
            )?;
            fs::rename(&staging, &destination)?;
            Ok(InstalledPetPack {
                manifest: copied_manifest,
                directory: destination.clone(),
                install_source: "folder".to_owned(),
                content_hash: Some(content_hash),
                signature_key_id: None,
            })
        })();
        match result {
            Ok(installed) => Ok(installed),
            Err(primary) => match fs::remove_dir_all(&staging) {
                Ok(()) => Err(primary),
                Err(cleanup) if cleanup.kind() == std::io::ErrorKind::NotFound => Err(primary),
                Err(cleanup) => Err(PetPackError::Invalid(format!(
                    "{primary}; staging cleanup also failed: {cleanup}"
                ))),
            },
        }
    }

    pub fn create_companion_from_image(
        image_path: &Path,
        library_root: &Path,
        name: &str,
    ) -> Result<InstalledPetPack, PetPackError> {
        validate_text("name", name, MAX_NAME_BYTES)?;
        let metadata = fs::symlink_metadata(image_path)?;
        if !metadata.file_type().is_file() {
            return Err(PetPackError::Invalid(
                "uploaded image must be a regular file".into(),
            ));
        }
        if metadata.len() == 0 || metadata.len() > ValidationLimits::default().max_asset_bytes {
            return Err(PetPackError::AssetTooLarge("uploaded image".into()));
        }
        let animation = animated::decode_animation(image_path)?;
        let pack_id = format!("user.custom-{}", uuid::Uuid::new_v4().simple());
        let source_root =
            std::env::temp_dir().join(format!("agentpet-image-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&source_root)?;
        let creation_result = (|| {
            let assets = source_root.join("assets");
            fs::create_dir(&assets)?;
            let (description, frames) = if animation.animated && animation.frames.len() > 1 {
                let capped: Vec<animated::DecodedFrame> = animation
                    .frames
                    .iter()
                    .take(MAX_IMPORTED_ANIMATION_FRAMES)
                    .cloned()
                    .collect();
                let (sheet, rects, fps) = animated::build_sprite_sheet(&capped)?;
                sheet.save(assets.join("character.png"))?;
                (
                    format!(
                        "Animated companion generated from {} frames of a user supplied image.",
                        capped.len()
                    ),
                    AnimationDefinition {
                        asset: "character".into(),
                        frames: rects,
                        fps,
                        loop_animation: true,
                        anchor: Anchor { x: 0.5, y: 1.0 },
                    },
                )
            } else {
                let single = animation
                    .frames
                    .first()
                    .ok_or_else(|| PetPackError::Invalid("image produced no frames".into()))?;
                let normalized = image::DynamicImage::ImageRgba8(single.image.clone())
                    .thumbnail(512, 512)
                    .to_rgba8();
                normalized.save(assets.join("character.png"))?;
                (
                    "Static companion generated from a user supplied image.".to_owned(),
                    AnimationDefinition {
                        asset: "character".into(),
                        frames: vec![FrameRect {
                            x: 0,
                            y: 0,
                            width: normalized.width(),
                            height: normalized.height(),
                        }],
                        fps: 1,
                        loop_animation: true,
                        anchor: Anchor { x: 0.5, y: 1.0 },
                    },
                )
            };
            let state_mappings = [
                "idle",
                "working",
                "thinking",
                "waiting",
                "success",
                "error",
                "celebrating",
                "sleeping",
                "hungry",
                "eating",
                "level_up",
                "achievement",
                "offline",
                "paused",
                "attention_required",
            ]
            .into_iter()
            .map(|state| (state.to_owned(), "idle".to_owned()))
            .collect();
            let manifest = PetPackManifest {
                schema_version: CURRENT_SCHEMA_VERSION,
                id: pack_id,
                name: name.trim().to_owned(),
                version: "1.0.0".into(),
                description,
                creator: "User supplied".into(),
                license: "User supplied; not verified".into(),
                source: "local-image-upload".into(),
                attribution:
                    "User supplied image; ownership and reuse permissions are not verified.".into(),
                commercial_use: false,
                redistribution: false,
                derivative_works: false,
                assets: BTreeMap::from([("character".into(), "assets/character.png".into())]),
                animations: BTreeMap::from([("idle".into(), frames)]),
                state_mappings,
                tags: vec!["custom".into(), "user-created".into()],
            };
            manifest.validate()?;
            fs::write(
                source_root.join(MANIFEST_FILE),
                serde_json::to_vec(&manifest)?,
            )?;
            let mut installed =
                Self::install_extracted(&source_root, library_root, ValidationLimits::default())?;
            installed.install_source = "image".to_owned();
            Ok(installed)
        })();
        let cleanup_result = fs::remove_dir_all(&source_root);
        match (creation_result, cleanup_result) {
            (Ok(installed), Ok(())) => Ok(installed),
            (Ok(installed), Err(error)) => {
                eprintln!("AgentPet could not clean temporary image-processing files: {error}");
                Ok(installed)
            }
            (Err(primary), Ok(())) => Err(primary),
            (Err(primary), Err(cleanup)) if cleanup.kind() == std::io::ErrorKind::NotFound => {
                Err(primary)
            }
            (Err(primary), Err(cleanup)) => Err(PetPackError::Invalid(format!(
                "{primary}; temporary image cleanup also failed: {cleanup}"
            ))),
        }
    }

    pub fn validate(&self) -> Result<(), PetPackError> {
        if self.schema_version != CURRENT_SCHEMA_VERSION {
            return Err(PetPackError::UnsupportedSchema(self.schema_version));
        }
        validate_text("id", &self.id, MAX_PACK_ID_BYTES)?;
        if !valid_pack_id(&self.id) {
            return Err(PetPackError::Invalid(
                "id must use lowercase dotted segments containing letters, digits, or hyphens"
                    .into(),
            ));
        }
        validate_text("name", &self.name, MAX_NAME_BYTES)?;
        validate_text("version", &self.version, 64)?;
        if !valid_semver(&self.version) {
            return Err(PetPackError::Invalid(
                "version must be a semantic version such as 1.2.3".into(),
            ));
        }
        validate_text("description", &self.description, MAX_DESCRIPTION_BYTES)?;
        validate_text("creator", &self.creator, 160)?;
        validate_text("license", &self.license, 128)?;
        validate_text("source", &self.source, 2_048)?;
        validate_text("attribution", &self.attribution, 2_048)?;
        if self.assets.is_empty() || self.assets.len() > MAX_ASSETS {
            return Err(PetPackError::Invalid(format!(
                "asset count must be between 1 and {MAX_ASSETS}"
            )));
        }
        if self.animations.is_empty() || self.animations.len() > MAX_ANIMATIONS {
            return Err(PetPackError::Invalid(format!(
                "animation count must be between 1 and {MAX_ANIMATIONS}"
            )));
        }
        for (asset_id, asset_path) in &self.assets {
            validate_asset_id(asset_id)?;
            validate_relative_path(asset_path)?;
            if !has_supported_extension(asset_path) {
                return Err(PetPackError::Invalid(format!(
                    "unsupported asset file type: {asset_path}"
                )));
            }
        }
        for (animation_id, animation) in &self.animations {
            validate_animation_id(animation_id)?;
            if !self.assets.contains_key(&animation.asset) {
                return Err(PetPackError::Invalid(format!(
                    "animation {animation_id} references unknown asset {}",
                    animation.asset
                )));
            }
            let image_path = &self.assets[&animation.asset];
            if !has_image_extension(image_path) {
                return Err(PetPackError::Invalid(format!(
                    "animation {animation_id} must reference an image asset"
                )));
            }
            if animation.frames.is_empty() || animation.frames.len() > MAX_FRAMES_PER_ANIMATION {
                return Err(PetPackError::Invalid(format!(
                    "animation {animation_id} has an invalid frame count"
                )));
            }
            if animation.fps == 0 || animation.fps > 120 {
                return Err(PetPackError::Invalid(format!(
                    "animation {animation_id} fps must be between 1 and 120"
                )));
            }
            if !(0.0..=1.0).contains(&animation.anchor.x)
                || !(0.0..=1.0).contains(&animation.anchor.y)
            {
                return Err(PetPackError::Invalid(format!(
                    "animation {animation_id} anchor must be between 0 and 1"
                )));
            }
            for frame in &animation.frames {
                if frame.width == 0 || frame.height == 0 {
                    return Err(PetPackError::Invalid(format!(
                        "animation {animation_id} has a zero-sized frame"
                    )));
                }
                frame
                    .x
                    .checked_add(frame.width)
                    .ok_or_else(|| PetPackError::Invalid("frame bounds overflow".into()))?;
                frame
                    .y
                    .checked_add(frame.height)
                    .ok_or_else(|| PetPackError::Invalid("frame bounds overflow".into()))?;
            }
        }
        for (state, animation_id) in &self.state_mappings {
            validate_animation_id(state)?;
            if !self.animations.contains_key(animation_id) {
                return Err(PetPackError::Invalid(format!(
                    "state mapping references unknown animation {animation_id}"
                )));
            }
        }
        if self.tags.len() > MAX_TAGS {
            return Err(PetPackError::Invalid(format!(
                "tag count must not exceed {MAX_TAGS}"
            )));
        }
        for tag in &self.tags {
            validate_text("tag", tag, 48)?;
        }
        Ok(())
    }

    fn validate_assets(
        &self,
        pack_root: &Path,
        limits: ValidationLimits,
    ) -> Result<(), PetPackError> {
        let root = fs::canonicalize(pack_root)?;
        let mut total_bytes = 0_u64;
        for relative_path in self.assets.values() {
            validate_relative_path(relative_path)?;
            let candidate = root.join(PathBuf::from(relative_path));
            let metadata = fs::symlink_metadata(&candidate)
                .map_err(|_| PetPackError::AssetMissing(relative_path.clone()))?;
            if !metadata.file_type().is_file() {
                return Err(PetPackError::AssetMissing(relative_path.clone()));
            }
            let canonical = fs::canonicalize(&candidate)?;
            if !canonical.starts_with(&root) {
                return Err(PetPackError::Invalid(format!(
                    "asset escapes pack directory: {relative_path}"
                )));
            }
            let size = metadata.len();
            if size > limits.max_asset_bytes {
                return Err(PetPackError::AssetTooLarge(relative_path.clone()));
            }
            total_bytes = total_bytes
                .checked_add(size)
                .ok_or(PetPackError::TotalAssetsTooLarge)?;
            if total_bytes > limits.max_total_asset_bytes {
                return Err(PetPackError::TotalAssetsTooLarge);
            }
            if has_image_extension(relative_path) {
                validate_image(&canonical, self, relative_path)?;
            }
        }
        Ok(())
    }
}

fn decode_image(path: &Path) -> Result<image::DynamicImage, PetPackError> {
    let mut reader = ImageReader::open(path)?.with_guessed_format()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_DIMENSION);
    limits.max_image_height = Some(MAX_IMAGE_DIMENSION);
    limits.max_alloc = Some(MAX_IMAGE_ALLOCATION);
    reader.limits(limits);
    Ok(reader.decode()?)
}

/// Computes a deterministic SHA-256 over the manifest and every declared
/// asset file. Used as the recorded content hash for installed packs.
fn compute_content_hash(root: &Path, manifest: &PetPackManifest) -> Result<String, PetPackError> {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(fs::read(root.join(MANIFEST_FILE))?);
    let mut paths: Vec<&String> = manifest.assets.values().collect();
    paths.sort();
    for relative in paths {
        hasher.update(relative.as_bytes());
        hasher.update(fs::read(root.join(relative))?);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn validate_image(
    path: &Path,
    manifest: &PetPackManifest,
    relative_path: &str,
) -> Result<(), PetPackError> {
    let decoded = decode_image(path)?;
    for animation in manifest.animations.values().filter(|animation| {
        manifest
            .assets
            .get(&animation.asset)
            .is_some_and(|asset| asset == relative_path)
    }) {
        for frame in &animation.frames {
            let right = frame
                .x
                .checked_add(frame.width)
                .ok_or_else(|| PetPackError::Invalid("frame bounds overflow".into()))?;
            let bottom = frame
                .y
                .checked_add(frame.height)
                .ok_or_else(|| PetPackError::Invalid("frame bounds overflow".into()))?;
            if right > decoded.width() || bottom > decoded.height() {
                return Err(PetPackError::Invalid(format!(
                    "animation frame exceeds image bounds: {relative_path}"
                )));
            }
        }
    }
    Ok(())
}

fn generate_preview(root: &Path, manifest: &PetPackManifest) -> Result<(), PetPackError> {
    let animation = manifest
        .animations
        .get("idle")
        .or_else(|| manifest.animations.values().next())
        .ok_or_else(|| PetPackError::Invalid("pack has no previewable animation".into()))?;
    let asset = manifest.assets.get(&animation.asset).ok_or_else(|| {
        PetPackError::Invalid("preview animation references a missing asset".into())
    })?;
    let image = decode_image(&root.join(asset))?;
    let frame = animation
        .frames
        .first()
        .ok_or_else(|| PetPackError::Invalid("preview animation has no frames".into()))?;
    let cropped = image
        .crop_imm(frame.x, frame.y, frame.width, frame.height)
        .thumbnail(320, 320);
    let preview_path = root.join(PREVIEW_FILE);
    fs::create_dir_all(
        preview_path
            .parent()
            .ok_or_else(|| PetPackError::Invalid("invalid preview destination".into()))?,
    )?;
    let mut output = Cursor::new(Vec::new());
    cropped.write_to(&mut output, ImageFormat::Png)?;
    fs::write(preview_path, output.into_inner())?;
    Ok(())
}

fn validate_text(field: &str, value: &str, max_bytes: usize) -> Result<(), PetPackError> {
    let trimmed = value.trim();
    if trimmed.is_empty() || value.len() > max_bytes || value.chars().any(char::is_control) {
        return Err(PetPackError::Invalid(format!(
            "{field} is empty, too long, or contains control characters"
        )));
    }
    Ok(())
}

fn valid_pack_id(value: &str) -> bool {
    let mut segments = value.split('.');
    let segments_are_valid = segments.all(|segment| {
        !segment.is_empty()
            && segment.as_bytes()[0].is_ascii_lowercase()
            && segment
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    });
    segments_are_valid && value.contains('.')
}

fn valid_semver(value: &str) -> bool {
    if value
        .chars()
        .any(|character| matches!(character, '/' | '\\' | ':'))
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+'))
    {
        return false;
    }
    if value.matches('+').count() > 1 {
        return false;
    }
    let (without_build, build) = value
        .split_once('+')
        .map_or((value, None), |(left, right)| (left, Some(right)));
    if build.is_some_and(|metadata| !valid_identifiers(metadata, false)) {
        return false;
    }
    let (core, prerelease) = without_build
        .split_once('-')
        .map_or((without_build, None), |(left, right)| (left, Some(right)));
    if prerelease.is_some_and(|metadata| !valid_identifiers(metadata, true)) {
        return false;
    }
    let parts: Vec<&str> = core.split('.').collect();
    parts.len() == 3
        && parts.iter().all(|part| {
            !part.is_empty()
                && part.bytes().all(|byte| byte.is_ascii_digit())
                && (part == &"0" || !part.starts_with('0'))
        })
}

fn valid_identifiers(value: &str, reject_numeric_leading_zero: bool) -> bool {
    !value.is_empty()
        && value.split('.').all(|identifier| {
            !identifier.is_empty()
                && identifier
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                && (!reject_numeric_leading_zero
                    || !identifier.bytes().all(|byte| byte.is_ascii_digit())
                    || identifier == "0"
                    || !identifier.starts_with('0'))
        })
}

fn validate_asset_id(value: &str) -> Result<(), PetPackError> {
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    {
        return Err(PetPackError::Invalid(format!("invalid asset key {value}")));
    }
    Ok(())
}

fn validate_animation_id(value: &str) -> Result<(), PetPackError> {
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    {
        return Err(PetPackError::Invalid(format!(
            "invalid animation/state key {value}"
        )));
    }
    Ok(())
}

fn validate_relative_path(value: &str) -> Result<(), PetPackError> {
    if value.is_empty()
        || value.len() > MAX_ASSET_PATH_BYTES
        || value.contains('\\')
        || value.starts_with('/')
        || value.contains(':')
        || value.chars().any(char::is_control)
    {
        return Err(PetPackError::Invalid(format!(
            "invalid relative asset path {value}"
        )));
    }
    let path = Path::new(value);
    if path
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(PetPackError::Invalid(format!(
            "asset path contains an unsafe component: {value}"
        )));
    }
    Ok(())
}

pub(crate) fn has_supported_extension(value: &str) -> bool {
    Path::new(value)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "webp" | "gif" | "wav" | "ogg" | "mp3" | "ttf" | "otf"
            )
        })
}

fn has_image_extension(value: &str) -> bool {
    Path::new(value)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "webp" | "gif"
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_manifest() -> PetPackManifest {
        PetPackManifest {
            schema_version: CURRENT_SCHEMA_VERSION,
            id: "creator.example-pet".into(),
            name: "Example Pet".into(),
            version: "1.0.0".into(),
            description: "Test pack".into(),
            creator: "Test Creator".into(),
            license: "CC-BY-4.0".into(),
            source: "https://example.invalid/source".into(),
            attribution: "Credit Test Creator".into(),
            commercial_use: false,
            redistribution: true,
            derivative_works: true,
            assets: BTreeMap::from([("idle_sprite".into(), "assets/idle.png".into())]),
            animations: BTreeMap::from([(
                "idle".into(),
                AnimationDefinition {
                    asset: "idle_sprite".into(),
                    frames: vec![FrameRect {
                        x: 0,
                        y: 0,
                        width: 64,
                        height: 64,
                    }],
                    fps: 1,
                    loop_animation: true,
                    anchor: Anchor { x: 0.5, y: 1.0 },
                },
            )]),
            state_mappings: BTreeMap::from([("Idle".into(), "idle".into())]),
            tags: vec!["original".into()],
        }
    }

    #[test]
    fn accepts_well_formed_manifest() {
        assert!(valid_manifest().validate().is_ok());
    }

    /// Validates the bundled Momiji sprite pack end-to-end: manifest schema,
    /// asset presence, image dimensions, and total size budget.
    #[test]
    fn bundled_momiji_pack_loads_and_validates() {
        let pack_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../pet-packs/momiji")
            .canonicalize()
            .expect("pet-packs/momiji exists beside the crate");
        let manifest = PetPackManifest::load_and_validate(&pack_root, ValidationLimits::default())
            .expect("bundled Momiji pack must load and validate");
        assert_eq!(manifest.id, "agentpet.momiji");
        assert_eq!(manifest.assets.len(), 9);
        // celebration and achievement share sprites, so the animation table is
        // larger than the asset table.
        assert_eq!(manifest.animations.len(), 10);
        assert!(manifest.state_mappings.contains_key("offline"));
    }

    #[test]
    fn rejects_path_traversal_and_unknown_animation_assets() {
        let mut manifest = valid_manifest();
        manifest
            .assets
            .insert("bad".into(), "../outside.png".into());
        assert!(manifest.validate().is_err());

        let mut manifest = valid_manifest();
        manifest
            .animations
            .get_mut("idle")
            .expect("idle animation exists")
            .asset = "missing".into();
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn rejects_unsupported_schema_and_invalid_frame_bounds() {
        let mut manifest = valid_manifest();
        manifest.schema_version = CURRENT_SCHEMA_VERSION + 1;
        assert!(matches!(
            manifest.validate(),
            Err(PetPackError::UnsupportedSchema(_))
        ));

        let mut manifest = valid_manifest();
        manifest
            .animations
            .get_mut("idle")
            .expect("idle animation exists")
            .frames[0]
            .width = 0;
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn parser_rejects_unknown_fields() {
        let bytes = br#"{"schemaVersion":1,"unexpected":true}"#;
        assert!(matches!(
            PetPackManifest::parse(bytes),
            Err(PetPackError::InvalidJson(_))
        ));
    }

    #[test]
    fn installs_validated_files_atomically_and_rejects_duplicate_version() {
        let test_root =
            std::env::temp_dir().join(format!("agentpet-pack-test-{}", uuid::Uuid::new_v4()));
        let source = test_root.join("source");
        let library = test_root.join("library");
        let asset_path = source.join("assets").join("idle.png");
        fs::create_dir_all(asset_path.parent().expect("asset parent exists"))
            .expect("asset directory created");
        image::RgbaImage::new(64, 64)
            .save(&asset_path)
            .expect("test image written");
        fs::write(
            source.join(MANIFEST_FILE),
            serde_json::to_vec(&valid_manifest()).expect("manifest serializes"),
        )
        .expect("manifest written");

        let installed =
            PetPackManifest::install_extracted(&source, &library, ValidationLimits::default())
                .expect("pack installs");
        assert!(installed.directory.join("assets/idle.png").is_file());
        assert!(matches!(
            PetPackManifest::install_extracted(&source, &library, ValidationLimits::default()),
            Err(PetPackError::AlreadyInstalled)
        ));
        fs::remove_dir_all(test_root).expect("test files removed");
    }

    #[test]
    fn creates_a_valid_static_pet_from_a_user_image() {
        let test_root =
            std::env::temp_dir().join(format!("agentpet-image-test-{}", uuid::Uuid::new_v4()));
        let library = test_root.join("library");
        let image_path = test_root.join("upload.png");
        fs::create_dir_all(&test_root).expect("test directory created");
        image::RgbaImage::from_pixel(64, 48, image::Rgba([210, 80, 110, 255]))
            .save(&image_path)
            .expect("upload fixture written");

        let installed =
            PetPackManifest::create_companion_from_image(&image_path, &library, "Mochi")
                .expect("static pet pack generated and installed");
        assert_eq!(installed.manifest.name, "Mochi");
        assert_eq!(installed.manifest.animations.len(), 1);
        assert_eq!(
            installed
                .manifest
                .state_mappings
                .get("working")
                .map(String::as_str),
            Some("idle")
        );
        assert!(installed.directory.join("assets/character.png").is_file());
        assert!(installed.directory.join(PREVIEW_FILE).is_file());
        assert!(PetPackManifest::load_and_validate(
            &installed.directory,
            ValidationLimits::default()
        )
        .is_ok());

        fs::remove_dir_all(test_root).expect("test files removed");
    }

    #[test]
    fn version_cannot_escape_install_directory() {
        let mut manifest = valid_manifest();
        manifest.version = "1.2.3/../../outside".into();
        assert!(manifest.validate().is_err());
    }

    fn write_test_pack(source: &Path) {
        fs::create_dir_all(source.join("assets")).expect("assets directory created");
        image::RgbaImage::from_pixel(64, 64, image::Rgba([10, 200, 90, 255]))
            .save(source.join("assets/idle.png"))
            .expect("test image written");
        fs::write(
            source.join(MANIFEST_FILE),
            serde_json::to_vec(&valid_manifest()).expect("manifest serializes"),
        )
        .expect("manifest written");
    }

    fn write_zip(zip_path: &Path, entries: &[(&str, &[u8])]) {
        use std::io::Write as _;
        let file = fs::File::create(zip_path).expect("zip created");
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        for (name, bytes) in entries {
            writer.start_file(*name, options).expect("zip entry starts");
            writer.write_all(bytes).expect("zip entry written");
        }
        writer.finish().expect("zip finished");
    }

    #[test]
    fn installs_a_valid_zip_archive_with_content_hash() {
        let temporary = tempfile::tempdir().expect("temporary directory created");
        let source = temporary.path().join("source");
        write_test_pack(&source);
        let manifest_bytes = fs::read(source.join(MANIFEST_FILE)).expect("manifest bytes");
        let asset_bytes = fs::read(source.join("assets/idle.png")).expect("asset bytes");
        let archive_path = temporary.path().join("pack.zip");
        write_zip(
            &archive_path,
            &[
                ("pet.json", &manifest_bytes),
                ("assets/idle.png", &asset_bytes),
            ],
        );
        let library = temporary.path().join("library");
        let installed = archive::install_archive(
            &archive_path,
            &library,
            ValidationLimits::default(),
            archive::ArchiveLimits::default(),
            &[],
            false,
        )
        .expect("zip pack installs");
        assert_eq!(installed.install_source, "archive");
        assert!(installed.content_hash.as_deref().is_some());
        assert!(installed.signature_key_id.is_none());
        assert!(library.join("creator.example-pet/1.0.0/pet.json").is_file());
        fs::remove_dir_all(temporary).expect("temporary files removed");
    }

    #[test]
    fn rejects_archive_path_traversal_and_duplicate_paths() {
        let temporary = tempfile::tempdir().expect("temporary directory created");
        let traversal = temporary.path().join("evil.zip");
        write_zip(&traversal, &[("../outside.png", &[1u8, 2, 3][..])]);
        assert!(matches!(
            archive::install_archive(
                &traversal,
                &temporary.path().join("library"),
                ValidationLimits::default(),
                archive::ArchiveLimits::default(),
                &[],
                false,
            ),
            Err(PetPackError::ArchiveEntryPath(_))
        ));
        let duplicate = temporary.path().join("duplicate.zip");
        write_zip(&duplicate, &[("pet.json", b"{}"), ("./pet.json", b"{}")]);
        assert!(matches!(
            archive::install_archive(
                &duplicate,
                &temporary.path().join("library"),
                ValidationLimits::default(),
                archive::ArchiveLimits::default(),
                &[],
                false,
            ),
            Err(PetPackError::ArchiveDuplicatePath(_))
        ));
        fs::remove_dir_all(temporary).expect("temporary files removed");
    }

    #[test]
    fn rejects_unsupported_content_and_requires_signature_when_asked() {
        let temporary = tempfile::tempdir().expect("temporary directory created");
        let executables = temporary.path().join("binary.zip");
        write_zip(&executables, &[("payload.exe", b"MZ")]);
        assert!(matches!(
            archive::install_archive(
                &executables,
                &temporary.path().join("library"),
                ValidationLimits::default(),
                archive::ArchiveLimits::default(),
                &[],
                false,
            ),
            Err(PetPackError::ArchiveEntryContent(_))
        ));

        let source = temporary.path().join("source");
        write_test_pack(&source);
        let manifest_bytes = fs::read(source.join(MANIFEST_FILE)).expect("manifest bytes");
        let asset_bytes = fs::read(source.join("assets/idle.png")).expect("asset bytes");
        let unsigned = temporary.path().join("unsigned.zip");
        write_zip(
            &unsigned,
            &[
                ("pet.json", &manifest_bytes),
                ("assets/idle.png", &asset_bytes),
            ],
        );
        assert!(matches!(
            archive::install_archive(
                &unsigned,
                &temporary.path().join("library"),
                ValidationLimits::default(),
                archive::ArchiveLimits::default(),
                &[],
                true,
            ),
            Err(PetPackError::SignatureMissing)
        ));
        fs::remove_dir_all(temporary).expect("temporary files removed");
    }

    #[test]
    fn installs_a_signed_archive_from_a_trusted_signer() {
        use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
        use ed25519_dalek::{Signer, SigningKey};

        let temporary = tempfile::tempdir().expect("temporary directory created");
        let source = temporary.path().join("source");
        write_test_pack(&source);
        let manifest_bytes = fs::read(source.join(MANIFEST_FILE)).expect("manifest bytes");
        let asset_bytes = fs::read(source.join("assets/idle.png")).expect("asset bytes");
        let signing_key = SigningKey::from_bytes(&[9u8; 32]);
        let signature_b64 = BASE64.encode(signing_key.sign(&manifest_bytes).to_bytes());
        let public_key = BASE64.encode(signing_key.verifying_key().to_bytes());
        let signature_doc = serde_json::json!({
            "algorithm": "ed25519",
            "keyId": "test.registry",
            "signature": signature_b64,
        })
        .to_string();
        let signed = temporary.path().join("signed.zip");
        write_zip(
            &signed,
            &[
                ("pet.json", &manifest_bytes),
                ("assets/idle.png", &asset_bytes),
                ("pet.sig", signature_doc.as_bytes()),
            ],
        );
        let trusted = vec![
            signature::TrustedSigner::from_base64("test.registry", &public_key)
                .expect("trusted signer decodes"),
        ];
        let installed = archive::install_archive(
            &signed,
            &temporary.path().join("library"),
            ValidationLimits::default(),
            archive::ArchiveLimits::default(),
            &trusted,
            true,
        )
        .expect("signed pack installs");
        assert_eq!(installed.signature_key_id.as_deref(), Some("test.registry"));
        fs::remove_dir_all(temporary).expect("temporary files removed");
    }

    #[test]
    fn animated_gif_import_produces_a_multi_frame_animation() {
        let temporary = tempfile::tempdir().expect("temporary directory created");
        let gif_path = temporary.path().join("bouncing.gif");
        let frames: Vec<image::Frame> = (0..3)
            .map(|index| {
                let buffer =
                    image::RgbaImage::from_pixel(48, 48, image::Rgba([40 * index, 90, 200, 255]));
                image::Frame::from_parts(buffer, 0, 0, image::Delay::from_numer_denom_ms(50, 1))
            })
            .collect();
        let file = fs::File::create(&gif_path).expect("gif fixture created");
        let mut encoder = image::codecs::gif::GifEncoder::new(file);
        for frame in frames {
            encoder.encode_frame(frame).expect("gif frame encoded");
        }
        drop(encoder);

        let library = temporary.path().join("library");
        let installed = PetPackManifest::create_companion_from_image(&gif_path, &library, "Bounce")
            .expect("animated companion installed");
        assert!(installed.manifest.description.contains("Animated"));
        let animation = &installed.manifest.animations["idle"];
        assert_eq!(animation.frames.len(), 3);
        assert!(animation.fps >= 1);
        assert!(installed.directory.join("assets/character.png").is_file());
        PetPackManifest::load_and_validate(&installed.directory, ValidationLimits::default())
            .expect("animated pack validates");
        fs::remove_dir_all(temporary).expect("temporary files removed");
    }
}

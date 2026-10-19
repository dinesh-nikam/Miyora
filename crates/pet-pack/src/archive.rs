//! Secure installation of PetPacks from ZIP and tar.gz archives.
//!
//! Archives are treated as untrusted input. Before anything is written the
//! archive is scanned entry by entry: entry count, per-entry size, total
//! uncompressed size, compression ratio, path components, entry types, and
//! file extensions are all validated. Extraction happens into a private
//! staging directory that is removed on every failure path; the validated
//! staged copy is then installed through the atomic folder pipeline.

use std::{
    collections::HashSet,
    fs::{self, File},
    io::{BufReader, Read, Write},
    path::{Component, Path},
};

use crate::{
    signature::{PackSignature, TrustedSigner, SIGNATURE_FILE},
    PetPackError, PetPackManifest, ValidationLimits, MANIFEST_FILE,
};

const ZIP_MAGIC: [u8; 4] = [0x50, 0x4B, 0x03, 0x04];
const ZIP_EMPTY_MAGIC: [u8; 4] = [0x50, 0x4B, 0x05, 0x06];
const GZIP_MAGIC: [u8; 2] = [0x1F, 0x8B];
const COPY_CHUNK_BYTES: usize = 64 * 1024;

/// Resource limits applied while scanning and extracting an archive.
#[derive(Debug, Clone, Copy)]
pub struct ArchiveLimits {
    pub max_compressed_bytes: u64,
    pub max_entries: usize,
    pub max_entry_bytes: u64,
    pub max_total_bytes: u64,
    pub max_compression_ratio: f64,
}

impl Default for ArchiveLimits {
    fn default() -> Self {
        Self {
            max_compressed_bytes: 64 * 1024 * 1024,
            max_entries: 1_024,
            max_entry_bytes: 32 * 1024 * 1024,
            max_total_bytes: 128 * 1024 * 1024,
            max_compression_ratio: 200.0,
        }
    }
}

/// Validates an archive path entry and returns its normalized relative form.
fn normalized_entry_path(raw: &str) -> Result<String, PetPackError> {
    let trimmed = raw.trim().trim_start_matches("./");
    if trimmed.is_empty() || trimmed.len() > 240 || trimmed.ends_with('/') {
        return Err(PetPackError::ArchiveEntryPath(raw.to_owned()));
    }
    let path = Path::new(trimmed);
    let mut parts: Vec<String> = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => {
                let part = part
                    .to_str()
                    .ok_or_else(|| PetPackError::ArchiveEntryPath(raw.to_owned()))?;
                if part.contains('\\') || part.contains(':') || part.chars().any(char::is_control) {
                    return Err(PetPackError::ArchiveEntryPath(raw.to_owned()));
                }
                parts.push(part.to_owned());
            }
            Component::CurDir => {}
            _ => return Err(PetPackError::ArchiveEntryPath(raw.to_owned())),
        }
    }
    if parts.is_empty() {
        return Err(PetPackError::ArchiveEntryPath(raw.to_owned()));
    }
    Ok(parts.join("/"))
}

fn allowed_archive_file(name: &str) -> Result<(), PetPackError> {
    if name == MANIFEST_FILE || name == SIGNATURE_FILE {
        return Ok(());
    }
    if crate::has_supported_extension(name) {
        return Ok(());
    }
    Err(PetPackError::ArchiveEntryContent(name.to_owned()))
}

struct ExtractionBudget {
    limits: ArchiveLimits,
    entries: usize,
    total_bytes: u64,
    seen: HashSet<String>,
}

impl ExtractionBudget {
    fn new(limits: ArchiveLimits) -> Self {
        Self {
            limits,
            entries: 0,
            total_bytes: 0,
            seen: HashSet::new(),
        }
    }

    fn claim(&mut self, normalized: &str) -> Result<(), PetPackError> {
        self.entries += 1;
        if self.entries > self.limits.max_entries {
            return Err(PetPackError::ArchiveEntryCount(self.limits.max_entries));
        }
        if !self.seen.insert(normalized.to_owned()) {
            return Err(PetPackError::ArchiveDuplicatePath(normalized.to_owned()));
        }
        Ok(())
    }

    fn account(&mut self, bytes: u64) -> Result<(), PetPackError> {
        self.total_bytes = self
            .total_bytes
            .checked_add(bytes)
            .ok_or(PetPackError::ArchiveTotalTooLarge)?;
        if self.total_bytes > self.limits.max_total_bytes {
            return Err(PetPackError::ArchiveTotalTooLarge);
        }
        Ok(())
    }

    fn check_ratio(
        &self,
        name: &str,
        uncompressed: u64,
        compressed: u64,
    ) -> Result<(), PetPackError> {
        let ratio = uncompressed as f64 / (compressed.max(1) as f64);
        if ratio > self.limits.max_compression_ratio {
            return Err(PetPackError::ArchiveRatioTooHigh(name.to_owned()));
        }
        Ok(())
    }
}

fn write_bounded(
    mut reader: impl Read,
    declared_size: u64,
    limit: u64,
    destination: &Path,
) -> Result<u64, PetPackError> {
    if declared_size > limit {
        return Err(PetPackError::ArchiveEntryTooLarge(
            destination.display().to_string(),
        ));
    }
    let mut file = File::create(destination)?;
    let mut buffer = vec![0_u8; COPY_CHUNK_BYTES];
    let mut written = 0_u64;
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        written = written.saturating_add(read as u64);
        if written > limit {
            return Err(PetPackError::ArchiveEntryTooLarge(
                destination.display().to_string(),
            ));
        }
        file.write_all(&buffer[..read])?;
    }
    if declared_size != 0 && written != declared_size {
        return Err(PetPackError::ArchiveEntrySizeMismatch(
            destination.display().to_string(),
        ));
    }
    Ok(written)
}

fn extract_zip(
    archive_path: &Path,
    staging: &Path,
    budget: &mut ExtractionBudget,
) -> Result<(), PetPackError> {
    let file = File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(BufReader::new(file))
        .map_err(|error| PetPackError::ArchiveRead(error.to_string()))?;
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| PetPackError::ArchiveRead(error.to_string()))?;
        let raw_name = entry.name().to_owned();
        if entry.is_dir() {
            continue;
        }
        let normalized = normalized_entry_path(&raw_name)?;
        if let Some(mode) = entry.unix_mode() {
            if mode & 0o170000 == 0o120000 {
                return Err(PetPackError::ArchiveEntryContent(raw_name));
            }
        }
        allowed_archive_file(&normalized)?;
        budget.claim(&normalized)?;
        budget.check_ratio(&normalized, entry.size(), entry.compressed_size().max(1))?;
        let destination = staging.join(&normalized);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        let entry_size = entry.size();
        let written = write_bounded(
            &mut entry,
            entry_size,
            budget.limits.max_entry_bytes,
            &destination,
        )?;
        budget.account(written)?;
    }
    Ok(())
}

fn extract_tar_gz(
    archive_path: &Path,
    staging: &Path,
    budget: &mut ExtractionBudget,
) -> Result<(), PetPackError> {
    let file = File::open(archive_path)?;
    let decoder = flate2::read::MultiGzDecoder::new(BufReader::new(file));
    let mut archive = tar::Archive::new(decoder);
    let entries = archive
        .entries()
        .map_err(|error| PetPackError::ArchiveRead(error.to_string()))?;
    for entry in entries {
        let mut entry = entry.map_err(|error| PetPackError::ArchiveRead(error.to_string()))?;
        let entry_type = entry.header().entry_type();
        if entry_type.is_dir() {
            continue;
        }
        if !entry_type.is_file() {
            return Err(PetPackError::ArchiveEntryContent(
                "non-regular archive entry".into(),
            ));
        }
        let raw_name = entry
            .path()
            .map_err(|error| PetPackError::ArchiveRead(error.to_string()))?
            .to_string_lossy()
            .into_owned();
        let normalized = normalized_entry_path(&raw_name)?;
        allowed_archive_file(&normalized)?;
        budget.claim(&normalized)?;
        let declared_size = entry
            .header()
            .size()
            .map_err(|error| PetPackError::ArchiveRead(error.to_string()))?;
        budget.check_ratio(&normalized, declared_size, 1)?;
        let destination = staging.join(&normalized);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        let written = write_bounded(
            &mut entry,
            declared_size,
            budget.limits.max_entry_bytes,
            &destination,
        )?;
        budget.account(written)?;
    }
    Ok(())
}

fn detect_format(archive_path: &Path) -> Result<&'static str, PetPackError> {
    let metadata = fs::symlink_metadata(archive_path)?;
    if !metadata.file_type().is_file() {
        return Err(PetPackError::ArchiveFormat(
            "archive must be a regular file".into(),
        ));
    }
    let mut magic = [0_u8; 4];
    let read = {
        let mut file = File::open(archive_path)?;
        file.read(&mut magic)?
    };
    if read >= 4 && (magic == ZIP_MAGIC || magic == ZIP_EMPTY_MAGIC) {
        return Ok("zip");
    }
    if read >= 2 && magic[..2] == GZIP_MAGIC {
        return Ok("tar.gz");
    }
    Err(PetPackError::ArchiveFormat(
        "unsupported archive type (expected .zip or .tar.gz)".into(),
    ))
}

/// Verifies an optional `pet.sig` found in an extracted pack directory.
///
/// Returns the identifier of the trusted key that signed the manifest, or
/// `None` when the pack is unsigned and signatures are not mandatory.
pub fn verify_extracted_signature(
    pack_root: &Path,
    trusted: &[TrustedSigner],
    require_signature: bool,
) -> Result<Option<String>, PetPackError> {
    let signature_path = pack_root.join(SIGNATURE_FILE);
    match fs::symlink_metadata(&signature_path) {
        Ok(metadata) if metadata.file_type().is_file() => {
            let manifest_bytes = fs::read(pack_root.join(MANIFEST_FILE))?;
            let pack_signature = PackSignature::parse(&fs::read(signature_path)?)?;
            let key_id = crate::signature::verify_manifest_signature(
                &manifest_bytes,
                &pack_signature,
                trusted,
            )?;
            Ok(Some(key_id))
        }
        Ok(_) => Err(PetPackError::SignatureFormat(
            "pet.sig exists but is not a regular file".into(),
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if require_signature {
                Err(PetPackError::SignatureMissing)
            } else {
                Ok(None)
            }
        }
        Err(error) => Err(PetPackError::Io(error)),
    }
}

/// Installs a PetPack from a ZIP or tar.gz archive.
///
/// The archive is scanned and extracted into a staging directory, the
/// detached signature (when present) is verified against `trusted` signers,
/// and the staged copy is installed through the atomic folder pipeline.
pub fn install_archive(
    archive_path: &Path,
    library_root: &Path,
    limits: ValidationLimits,
    archive_limits: ArchiveLimits,
    trusted: &[TrustedSigner],
    require_signature: bool,
) -> Result<crate::InstalledPetPack, PetPackError> {
    let metadata = fs::symlink_metadata(archive_path)?;
    let compressed_bytes = metadata.len();
    if compressed_bytes > archive_limits.max_compressed_bytes {
        return Err(PetPackError::ArchiveTooLarge(compressed_bytes));
    }
    let format = detect_format(archive_path)?;
    fs::create_dir_all(library_root)?;
    let library_root = fs::canonicalize(library_root)?;
    let extraction_root = library_root.join(format!(".archive-staging-{}", uuid::Uuid::new_v4()));
    fs::create_dir(&extraction_root)?;
    let staged_root = extraction_root.join("pack");
    fs::create_dir(&staged_root)?;
    let result = (|| {
        let mut budget = ExtractionBudget::new(archive_limits);
        match format {
            "zip" => extract_zip(archive_path, &staged_root, &mut budget)?,
            "tar.gz" => extract_tar_gz(archive_path, &staged_root, &mut budget)?,
            other => {
                return Err(PetPackError::ArchiveFormat(format!(
                    "unsupported archive format {other}"
                )))
            }
        }
        let signature_key_id =
            verify_extracted_signature(&staged_root, trusted, require_signature)?;
        let mut installed =
            PetPackManifest::install_extracted(&staged_root, &library_root, limits)?;
        installed.install_source = "archive".to_owned();
        installed.signature_key_id = signature_key_id;
        Ok(installed)
    })();
    match fs::remove_dir_all(&extraction_root) {
        Ok(()) => result,
        Err(cleanup) if cleanup.kind() == std::io::ErrorKind::NotFound => result,
        Err(cleanup) => match result {
            Ok(installed) => Ok(installed),
            Err(primary) => Err(PetPackError::Invalid(format!(
                "{primary}; archive staging cleanup also failed: {cleanup}"
            ))),
        },
    }
}

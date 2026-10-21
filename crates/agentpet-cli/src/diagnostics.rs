use std::{
    fs::{self, OpenOptions},
    io::{self, BufRead, BufReader, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use agentpet_core::default_data_directory;

const MAX_DIAGNOSTICS_BYTES: u64 = 256 * 1024; // 256 KB
const TRIM_TARGET_BYTES: usize = 128 * 1024; // 128 KB
pub const DIAGNOSTICS_FILE: &str = "diagnostics.log";

pub fn diagnostics_path() -> Option<PathBuf> {
    default_data_directory().ok().map(|d| d.join(DIAGNOSTICS_FILE))
}

pub fn append_diagnostic_error(source: &str, error: &impl std::fmt::Display) {
    let Some(path) = diagnostics_path() else { return };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(meta) = fs::metadata(&path) {
        if meta.len() > MAX_DIAGNOSTICS_BYTES {
            let _ = rotate_or_trim(&path);
        }
    }
    let timestamp = chrono::Utc::now().to_rfc3339();
    let entry = format!("[{timestamp}] [{source}] {error}\n");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = file.write_all(entry.as_bytes());
    }
}

fn rotate_or_trim(path: &Path) -> io::Result<()> {
    let mut file = OpenOptions::new().read(true).write(true).open(path)?;
    let len = file.metadata()?.len();
    if len > MAX_DIAGNOSTICS_BYTES {
        let skip = len.saturating_sub(TRIM_TARGET_BYTES as u64);
        file.seek(SeekFrom::Start(skip))?;
        let mut reader = BufReader::new(file);
        let mut discard = String::new();
        reader.read_line(&mut discard)?;
        let mut rest = Vec::new();
        reader.read_to_end(&mut rest)?;
        drop(reader);
        fs::write(path, &rest)?;
    }
    Ok(())
}

pub fn read_recent_errors(limit: usize) -> Vec<String> {
    let Some(path) = diagnostics_path() else { return Vec::new() };
    let Ok(file) = fs::File::open(&path) else { return Vec::new() };
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().filter_map(|l| l.ok()).collect();
    lines.into_iter().rev().take(limit).collect()
}

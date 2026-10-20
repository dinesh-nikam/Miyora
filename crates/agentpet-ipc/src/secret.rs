//! Per-install shared secret for local daemon authentication.
//!
//! The secret is generated once per installation and stored in a
//! restricted local file. It never leaves the machine and is never sent
//! over the transport: it is only used to derive authentication tags.

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use subtle::ConstantTimeEq;

use crate::error::IpcError;

/// Number of random bytes in the shared secret.
pub const SECRET_BYTES: usize = 32;

/// File name of the secret inside the application data directory.
pub const SECRET_FILE_NAME: &str = "daemon.secret";

/// A per-install shared secret used to authenticate local peers.
#[derive(Clone, PartialEq, Eq)]
pub struct LocalSecret {
    bytes: Vec<u8>,
}

impl std::fmt::Debug for LocalSecret {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LocalSecret")
            .field("bytes", &"<redacted>")
            .finish()
    }
}

impl LocalSecret {
    /// Creates a secret from raw bytes.
    ///
    /// The value must be at least 16 bytes so a weak or empty secret can
    /// never be installed.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, IpcError> {
        if bytes.len() < 16 {
            return Err(IpcError::SecretUnavailable(
                "secret must be at least 16 bytes".to_owned(),
            ));
        }
        Ok(Self { bytes })
    }

    /// Generates a new random secret.
    pub fn generate() -> Result<Self, IpcError> {
        let mut material = vec![0_u8; SECRET_BYTES];
        getrandom::fill(&mut material).map_err(|error| {
            IpcError::SecretUnavailable(format!("random source failed: {error}"))
        })?;
        Self::from_bytes(material)
    }

    /// Returns the secret bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Compares two secrets in constant time.
    pub fn matches(&self, other: &[u8]) -> bool {
        if other.len() != self.bytes.len() {
            return false;
        }
        self.bytes.ct_eq(other).into()
    }
}

/// Path of the secret file inside a data directory.
pub fn secret_path(data_directory: &Path) -> PathBuf {
    data_directory.join(SECRET_FILE_NAME)
}

/// Loads the secret from `path`, creating it when it does not exist.
///
/// The file is written with owner-only permissions on Unix. On Windows
/// the file inherits the profile directory ACL, which already restricts
/// access to the current user.
pub fn load_or_create_secret(path: &Path) -> Result<LocalSecret, IpcError> {
    if let Ok(existing) = fs::read(path) {
        return LocalSecret::from_bytes(existing);
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            IpcError::SecretUnavailable(format!("cannot create data directory: {error}"))
        })?;
    }

    let secret = LocalSecret::generate()?;
    write_secret_file(path, secret.as_bytes())?;
    Ok(secret)
}

fn write_secret_file(path: &Path, bytes: &[u8]) -> Result<(), IpcError> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }

    let mut file = options.open(path).map_err(|error| {
        IpcError::SecretUnavailable(format!("cannot create secret file: {error}"))
    })?;
    file.write_all(bytes).map_err(|error| {
        IpcError::SecretUnavailable(format!("cannot write secret file: {error}"))
    })?;
    file.sync_all().map_err(|error| {
        IpcError::SecretUnavailable(format!("cannot flush secret file: {error}"))
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_secrets_are_long_and_unique() {
        let first = LocalSecret::generate().expect("secret generated");
        let second = LocalSecret::generate().expect("secret generated");
        assert_eq!(first.as_bytes().len(), SECRET_BYTES);
        assert_ne!(first.as_bytes(), second.as_bytes());
    }

    #[test]
    fn rejects_short_secrets() {
        assert!(LocalSecret::from_bytes(vec![0_u8; 8]).is_err());
    }

    #[test]
    fn matches_accepts_only_the_equal_value() {
        let secret = LocalSecret::from_bytes(vec![7_u8; 32]).expect("secret accepted");
        assert!(secret.matches(&[7_u8; 32]));
        assert!(!secret.matches(&[8_u8; 32]));
        assert!(!secret.matches(&[7_u8; 16]));
    }

    #[test]
    fn debug_output_does_not_leak_the_secret() {
        let secret = LocalSecret::from_bytes(vec![9_u8; 32]).expect("secret accepted");
        let rendered = format!("{secret:?}");
        assert!(rendered.contains("redacted"));
        assert!(!rendered.contains("9"));
    }

    #[test]
    fn load_or_create_is_stable_across_calls() {
        let directory = tempfile::tempdir().expect("temp dir");
        let path = secret_path(directory.path());
        let first = load_or_create_secret(&path).expect("secret created");
        let second = load_or_create_secret(&path).expect("secret loaded");
        assert_eq!(first, second);
    }
}

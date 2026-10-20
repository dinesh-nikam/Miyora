//! Platform-specific resolution of the local daemon endpoint.
//!
//! Windows uses a named pipe in the `\\.\pipe\` namespace. macOS and
//! Linux use a Unix domain socket inside the application data
//! directory. The endpoint never leaves the current user account: the
//! named pipe rejects remote clients and the Unix socket is created with
//! owner-only permissions.

#[cfg(unix)]
use std::path::PathBuf;

use crate::error::IpcError;

/// Windows named-pipe name of the local daemon endpoint.
pub const WINDOWS_PIPE_NAME: &str = r"\\.\pipe\dev.agentpet.desktop";

/// File name of the Unix domain socket inside the data directory.
pub const UNIX_SOCKET_FILE_NAME: &str = "agentpet.sock";
/// A resolved local daemon endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Endpoint {
    /// Windows named pipe.
    #[cfg(windows)]
    NamedPipe(String),
    /// Unix domain socket path.
    #[cfg(unix)]
    UnixSocket(PathBuf),
}

impl Endpoint {
    /// Resolves the endpoint for a given application data directory.
    ///
    /// The data directory is only used on Unix; on Windows the named pipe
    /// namespace is machine-wide. Access to the pipe is restricted by the
    /// default security descriptor of the creating token, which grants
    /// access to the current user, local administrators, and the local
    /// system, and by `reject_remote_clients`. The shared secret
    /// handshake remains the actual authentication boundary.
    pub fn resolve(data_directory: &std::path::Path) -> Result<Self, IpcError> {
        #[cfg(windows)]
        {
            let _ = data_directory;
            Ok(Self::NamedPipe(WINDOWS_PIPE_NAME.to_owned()))
        }

        #[cfg(unix)]
        {
            Ok(Self::UnixSocket(data_directory.join(UNIX_SOCKET_FILE_NAME)))
        }

        #[cfg(not(any(windows, unix)))]
        {
            Err(IpcError::EndpointUnavailable(
                "local daemon transport is not supported on this platform".to_owned(),
            ))
        }
    }

    /// Short, privacy-safe description used in logs.
    pub fn describe(&self) -> String {
        match self {
            #[cfg(windows)]
            Self::NamedPipe(name) => format!("named pipe {name}"),
            #[cfg(unix)]
            Self::UnixSocket(path) => {
                let name = path
                    .file_name()
                    .map(|value| value.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "socket".to_owned());
                format!("unix socket {name}")
            }
            #[cfg(not(any(windows, unix)))]
            _ => "unsupported endpoint".to_owned(),
        }
    }

    /// Path of the Unix socket, when the endpoint is a Unix socket.
    #[cfg(unix)]
    pub fn socket_path(&self) -> Option<&std::path::Path> {
        match self {
            Self::UnixSocket(path) => Some(path.as_path()),
        }
    }
}

/// Removes a stale Unix socket left behind by an unclean shutdown.
///
/// A live socket is never removed: the caller distinguishes a live
/// daemon from a stale one by attempting a connection first.
#[cfg(unix)]
pub fn remove_stale_socket(path: &std::path::Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

/// Identity of a Unix socket file, used to detect a replaced path.
#[cfg(unix)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SocketIdentity {
    device: u64,
    inode: u64,
}

#[cfg(unix)]
impl SocketIdentity {
    /// Reads the identity of the socket file at `path`.
    pub(crate) fn read(path: &std::path::Path) -> std::io::Result<Self> {
        use std::os::unix::fs::MetadataExt;

        let metadata = std::fs::metadata(path)?;
        Ok(Self {
            device: metadata.dev(),
            inode: metadata.ino(),
        })
    }
}

/// Removes the Unix socket at `path` only if it is still the same file.
///
/// A path that was replaced while the daemon was running is left
/// untouched, so shutdown never deletes a file it does not own.
#[cfg(unix)]
pub(crate) fn remove_socket_if_unchanged(
    path: &std::path::Path,
    expected: SocketIdentity,
) -> std::io::Result<()> {
    let current = match SocketIdentity::read(path) {
        Ok(identity) => identity,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    if current != expected {
        return Ok(());
    }
    remove_stale_socket(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_an_endpoint_for_the_data_directory() {
        let directory = tempfile::tempdir().expect("temp dir");
        let endpoint = Endpoint::resolve(directory.path()).expect("endpoint resolved");
        let description = endpoint.describe();
        assert!(!description.is_empty());
        // The description must never leak the absolute data directory.
        assert!(!description.contains(&directory.path().to_string_lossy().to_string()));
    }

    #[cfg(unix)]
    #[test]
    fn unix_endpoint_points_into_the_data_directory() {
        let directory = tempfile::tempdir().expect("temp dir");
        let endpoint = Endpoint::resolve(directory.path()).expect("endpoint resolved");
        let path = endpoint.socket_path().expect("socket path");
        assert_eq!(path, directory.path().join(UNIX_SOCKET_FILE_NAME));
    }

    #[cfg(windows)]
    #[test]
    fn windows_endpoint_uses_the_named_pipe_namespace() {
        let directory = tempfile::tempdir().expect("temp dir");
        let endpoint = Endpoint::resolve(directory.path()).expect("endpoint resolved");
        assert_eq!(endpoint, Endpoint::NamedPipe(WINDOWS_PIPE_NAME.to_owned()));
    }

    #[cfg(unix)]
    #[test]
    fn removing_a_missing_socket_succeeds() {
        let directory = tempfile::tempdir().expect("temp dir");
        let path = directory.path().join("missing.sock");
        assert!(remove_stale_socket(&path).is_ok());
    }
}

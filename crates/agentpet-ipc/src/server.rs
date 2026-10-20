//! Local daemon server transport.
//!
//! The server binds the platform endpoint, accepts one connection at a
//! time, and lets the caller run the authentication handshake before
//! exchanging any protocol message.

#[cfg(unix)]
use std::path::{Path, PathBuf};

use crate::endpoint::Endpoint;
use crate::error::IpcError;
use crate::transport::{ConnectionStream, DaemonConnection};

#[cfg(unix)]
use tokio::net::{UnixListener, UnixStream};

#[cfg(windows)]
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};

/// A bound local daemon endpoint that accepts authenticated connections.
pub struct DaemonServer {
    #[cfg(unix)]
    listener: UnixListener,
    #[cfg(unix)]
    socket_path: PathBuf,
    #[cfg(unix)]
    socket_identity: crate::endpoint::SocketIdentity,
    #[cfg(windows)]
    pending: Option<NamedPipeServer>,
    #[cfg(windows)]
    pipe_name: String,
}

#[cfg(unix)]
impl Drop for DaemonServer {
    /// Removes the socket file on a clean shutdown.
    ///
    /// The file is removed only when it still refers to the socket this
    /// server bound. A path that was replaced in the meantime is left
    /// untouched.
    fn drop(&mut self) {
        let _ =
            crate::endpoint::remove_socket_if_unchanged(&self.socket_path, self.socket_identity);
    }
}

impl DaemonServer {
    /// Binds the server to `endpoint`.
    pub async fn bind(endpoint: &Endpoint) -> Result<Self, IpcError> {
        #[cfg(unix)]
        {
            let Endpoint::UnixSocket(path) = endpoint;
            let listener = bind_unix_listener(path).await?;
            let socket_identity = crate::endpoint::SocketIdentity::read(path)
                .map_err(IpcError::TransportUnavailable)?;
            Ok(Self {
                listener,
                socket_path: path.clone(),
                socket_identity,
            })
        }

        #[cfg(windows)]
        {
            let Endpoint::NamedPipe(name) = endpoint;
            let pending = create_pipe_instance(name, true)?;
            Ok(Self {
                pending: Some(pending),
                pipe_name: name.clone(),
            })
        }
    }

    /// Accepts the next connection.
    ///
    /// The returned connection is not authenticated yet: the caller must
    /// run [`DaemonConnection::authenticate_server`] before trusting any
    /// frame received on it.
    pub async fn accept(&mut self) -> Result<DaemonConnection, IpcError> {
        #[cfg(unix)]
        {
            let (stream, _address) = self
                .listener
                .accept()
                .await
                .map_err(IpcError::TransportUnavailable)?;
            Ok(DaemonConnection::new(ConnectionStream::Unix(stream)))
        }

        #[cfg(windows)]
        {
            let current = self.pending.take().ok_or_else(|| {
                IpcError::TransportUnavailable(std::io::Error::other(
                    "named pipe instance is unavailable",
                ))
            })?;
            // A new instance is published before waiting so a client that
            // connects right after this one is not blocked.
            self.pending = Some(create_pipe_instance(&self.pipe_name, false)?);
            current
                .connect()
                .await
                .map_err(IpcError::TransportUnavailable)?;
            Ok(DaemonConnection::new(ConnectionStream::PipeServer(current)))
        }
    }
}

#[cfg(windows)]
fn create_pipe_instance(name: &str, first: bool) -> Result<NamedPipeServer, IpcError> {
    let mut options = ServerOptions::new();
    options
        .first_pipe_instance(first)
        .reject_remote_clients(true)
        .create(name)
        .map_err(IpcError::TransportUnavailable)
}

#[cfg(unix)]
async fn bind_unix_listener(path: &Path) -> Result<UnixListener, IpcError> {
    let listener = match UnixListener::bind(path) {
        Ok(listener) => listener,
        Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => {
            // A socket file left behind by an unclean shutdown blocks the
            // bind. A live daemon is detected by a successful connect.
            if UnixStream::connect(path).await.is_ok() {
                return Err(IpcError::EndpointUnavailable(
                    "another local daemon is already listening".to_owned(),
                ));
            }
            crate::endpoint::remove_stale_socket(path).map_err(IpcError::TransportUnavailable)?;
            UnixListener::bind(path).map_err(IpcError::TransportUnavailable)?
        }
        Err(error) => return Err(IpcError::TransportUnavailable(error)),
    };
    restrict_socket_permissions(path)?;
    Ok(listener)
}

/// Restricts the socket file to the owning user.
///
/// The socket inherits the process umask on creation, which commonly
/// leaves it readable and writable by every local account. The endpoint
/// is a same-user transport, so the mode is set explicitly instead of
/// relying on the umask of the session that started the daemon.
#[cfg(unix)]
fn restrict_socket_permissions(path: &Path) -> Result<(), IpcError> {
    use std::os::unix::fs::PermissionsExt;

    const OWNER_ONLY: u32 = 0o600;

    std::fs::set_permissions(path, std::fs::Permissions::from_mode(OWNER_ONLY))
        .map_err(IpcError::TransportUnavailable)
}

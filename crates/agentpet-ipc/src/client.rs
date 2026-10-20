//! Local daemon client transport.
//!
//! The client connects to the platform endpoint and completes the mutual
//! authentication handshake before any protocol message is exchanged.

use crate::endpoint::Endpoint;
use crate::error::IpcError;
use crate::transport::{ConnectionStream, DaemonConnection};

#[cfg(unix)]
use tokio::net::UnixStream;

#[cfg(windows)]
use tokio::net::windows::named_pipe::ClientOptions;

/// Connects to the local daemon and authenticates with `secret`.
pub async fn connect(endpoint: &Endpoint, secret: &[u8]) -> Result<DaemonConnection, IpcError> {
    let mut connection = connect_unauthenticated(endpoint).await?;
    connection.authenticate_client(secret).await?;
    Ok(connection)
}

/// Connects to the local daemon without authenticating.
///
/// The caller must run [`DaemonConnection::authenticate_client`] before
/// trusting any frame received on the connection.
pub async fn connect_unauthenticated(endpoint: &Endpoint) -> Result<DaemonConnection, IpcError> {
    #[cfg(unix)]
    {
        let Endpoint::UnixSocket(path) = endpoint;
        let stream = UnixStream::connect(path)
            .await
            .map_err(IpcError::TransportUnavailable)?;
        Ok(DaemonConnection::new(ConnectionStream::Unix(stream)))
    }

    #[cfg(windows)]
    {
        let Endpoint::NamedPipe(name) = endpoint;
        let client = ClientOptions::new()
            .open(name)
            .map_err(IpcError::TransportUnavailable)?;
        Ok(DaemonConnection::new(ConnectionStream::PipeClient(client)))
    }
}

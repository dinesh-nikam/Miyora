//! Cross-platform local transport for the daemon.
//!
//! Windows uses a named pipe in the `\\.\pipe\` namespace; macOS and
//! Linux use a Unix domain socket. Both carry the same length-prefixed
//! frames and the same mutual authentication handshake, so the daemon
//! protocol is identical on every platform.

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::IpcError;
use crate::framing::{read_frame, write_frame, DEFAULT_MAX_FRAME_BYTES};
use crate::handshake::{
    build_client_response, build_server_proof, random_nonce, verify_client_response,
    verify_server_proof, ClientResponse, ServerChallenge, ServerProof, PROTOCOL_VERSION,
};

#[cfg(unix)]
use tokio::net::UnixStream;

#[cfg(windows)]
use tokio::net::windows::named_pipe::{NamedPipeClient, NamedPipeServer};

/// A framed, authenticated connection to the local daemon.
pub struct DaemonConnection {
    stream: ConnectionStream,
    max_frame_bytes: usize,
}

impl std::fmt::Debug for DaemonConnection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DaemonConnection")
            .field("max_frame_bytes", &self.max_frame_bytes)
            .finish_non_exhaustive()
    }
}

impl DaemonConnection {
    /// Wraps a raw stream with the default frame limit.
    pub(crate) fn new(stream: ConnectionStream) -> Self {
        Self {
            stream,
            max_frame_bytes: DEFAULT_MAX_FRAME_BYTES,
        }
    }

    /// Reads one frame from the peer.
    pub async fn read_frame(&mut self) -> Result<Option<Vec<u8>>, IpcError> {
        read_frame(&mut self.stream, self.max_frame_bytes).await
    }

    /// Writes one frame to the peer.
    pub async fn write_frame(&mut self, payload: &[u8]) -> Result<(), IpcError> {
        write_frame(&mut self.stream, payload).await
    }

    /// Writes a JSON document as one frame.
    pub async fn write_json<T: Serialize>(&mut self, value: &T) -> Result<(), IpcError> {
        let payload = serde_json::to_vec(value)?;
        self.write_frame(&payload).await
    }

    /// Reads a JSON document from one frame.
    pub async fn read_json<T: DeserializeOwned>(&mut self) -> Result<T, IpcError> {
        let payload = self.read_frame().await?.ok_or(IpcError::MalformedFrame {
            reason: "peer closed the connection before sending a document",
        })?;
        Ok(serde_json::from_slice(&payload)?)
    }

    /// Runs the server side of the mutual authentication handshake.
    ///
    /// The server proof is always sent, even when the client answer is
    /// rejected. That keeps the exchange symmetric: both peers reach the
    /// same conclusion instead of one side observing a dropped
    /// connection.
    pub async fn authenticate_server(&mut self, secret: &[u8]) -> Result<(), IpcError> {
        let challenge = ServerChallenge {
            protocol_version: PROTOCOL_VERSION,
            nonce: random_nonce()?,
        };
        self.write_json(&challenge).await?;

        let response: ClientResponse = self.read_json().await?;
        let verification = verify_client_response(secret, &challenge, &response);

        let proof = build_server_proof(secret, &challenge.nonce, &response.client_nonce)?;
        self.write_json(&proof).await?;
        verification
    }

    /// Runs the client side of the mutual authentication handshake.
    pub async fn authenticate_client(&mut self, secret: &[u8]) -> Result<(), IpcError> {
        let challenge: ServerChallenge = self.read_json().await?;
        if challenge.protocol_version != PROTOCOL_VERSION {
            return Err(IpcError::ProtocolVersionUnsupported {
                version: challenge.protocol_version,
            });
        }

        let client_nonce = random_nonce()?;
        let response =
            build_client_response(secret, PROTOCOL_VERSION, &challenge.nonce, &client_nonce)?;
        self.write_json(&response).await?;

        let proof: ServerProof = self.read_json().await?;
        verify_server_proof(secret, &challenge, &response, &proof)
    }
}

/// The platform-specific byte stream behind a connection.
pub(crate) enum ConnectionStream {
    #[cfg(unix)]
    Unix(UnixStream),
    #[cfg(windows)]
    PipeServer(NamedPipeServer),
    #[cfg(windows)]
    PipeClient(NamedPipeClient),
}

#[cfg(unix)]
impl tokio::io::AsyncRead for ConnectionStream {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            Self::Unix(stream) => std::pin::Pin::new(stream).poll_read(cx, buf),
        }
    }
}

#[cfg(unix)]
impl tokio::io::AsyncWrite for ConnectionStream {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        match self.get_mut() {
            Self::Unix(stream) => std::pin::Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            Self::Unix(stream) => std::pin::Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            Self::Unix(stream) => std::pin::Pin::new(stream).poll_shutdown(cx),
        }
    }
}

#[cfg(windows)]
impl tokio::io::AsyncRead for ConnectionStream {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            Self::PipeServer(stream) => std::pin::Pin::new(stream).poll_read(cx, buf),
            Self::PipeClient(stream) => std::pin::Pin::new(stream).poll_read(cx, buf),
        }
    }
}

#[cfg(windows)]
impl tokio::io::AsyncWrite for ConnectionStream {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        match self.get_mut() {
            Self::PipeServer(stream) => std::pin::Pin::new(stream).poll_write(cx, buf),
            Self::PipeClient(stream) => std::pin::Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            Self::PipeServer(stream) => std::pin::Pin::new(stream).poll_flush(cx),
            Self::PipeClient(stream) => std::pin::Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            Self::PipeServer(stream) => std::pin::Pin::new(stream).poll_shutdown(cx),
            Self::PipeClient(stream) => std::pin::Pin::new(stream).poll_shutdown(cx),
        }
    }
}

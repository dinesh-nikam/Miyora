//! Stable, privacy-safe errors for the local daemon transport.

use std::io;

/// Error codes reported to local peers. The values are stable
/// machine-readable strings and match the documented daemon error
/// vocabulary in `API.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcErrorCode {
    /// The peer sent an envelope with an unsupported protocol version.
    ProtocolVersionUnsupported,
    /// The peer failed the authentication handshake.
    AuthenticationFailed,
    /// The peer sent a frame larger than the configured limit.
    FrameTooLarge,
    /// The peer violated the framing contract.
    MalformedFrame,
    /// The peer exceeded the request rate budget.
    RateLimited,
    /// The transport could not be established or was lost.
    TransportUnavailable,
    /// The local endpoint could not be resolved.
    EndpointUnavailable,
}

impl IpcErrorCode {
    /// Stable wire representation of the code.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ProtocolVersionUnsupported => "PROTOCOL_VERSION_UNSUPPORTED",
            Self::AuthenticationFailed => "AUTHENTICATION_FAILED",
            Self::FrameTooLarge => "FRAME_TOO_LARGE",
            Self::MalformedFrame => "MALFORMED_FRAME",
            Self::RateLimited => "RATE_LIMITED",
            Self::TransportUnavailable => "TRANSPORT_UNAVAILABLE",
            Self::EndpointUnavailable => "ENDPOINT_UNAVAILABLE",
        }
    }
}

/// Failures raised by the transport, framing, and authentication layers.
#[derive(Debug, thiserror::Error)]
pub enum IpcError {
    /// A frame exceeded the maximum accepted size.
    #[error("frame exceeds the {limit} byte limit")]
    FrameTooLarge {
        /// Configured maximum frame size in bytes.
        limit: usize,
    },
    /// A frame did not follow the length-prefixed framing contract.
    #[error("malformed frame: {reason}")]
    MalformedFrame {
        /// Human-readable, non-sensitive description.
        reason: &'static str,
    },
    /// The peer could not be authenticated.
    #[error("local peer authentication failed")]
    AuthenticationFailed,
    /// The peer used an unsupported protocol version.
    #[error("protocol version {version} is not supported")]
    ProtocolVersionUnsupported {
        /// Version advertised by the peer.
        version: u16,
    },
    /// The transport could not be created, connected, or read.
    #[error("transport unavailable: {0}")]
    TransportUnavailable(#[source] io::Error),
    /// The local endpoint could not be resolved for this platform.
    #[error("local daemon endpoint is unavailable: {0}")]
    EndpointUnavailable(String),
    /// The local secret could not be loaded or created.
    #[error("local daemon secret is unavailable: {0}")]
    SecretUnavailable(String),
    /// A frame could not be encoded or decoded as JSON.
    #[error("frame encoding failed: {0}")]
    Encoding(#[from] serde_json::Error),
}

impl IpcError {
    /// Maps the error to the stable code reported to peers.
    pub const fn code(&self) -> IpcErrorCode {
        match self {
            Self::FrameTooLarge { .. } => IpcErrorCode::FrameTooLarge,
            Self::MalformedFrame { .. } => IpcErrorCode::MalformedFrame,
            Self::AuthenticationFailed => IpcErrorCode::AuthenticationFailed,
            Self::ProtocolVersionUnsupported { .. } => IpcErrorCode::ProtocolVersionUnsupported,
            Self::TransportUnavailable(_) => IpcErrorCode::TransportUnavailable,
            Self::EndpointUnavailable(_) => IpcErrorCode::EndpointUnavailable,
            Self::SecretUnavailable(_) => IpcErrorCode::TransportUnavailable,
            Self::Encoding(_) => IpcErrorCode::MalformedFrame,
        }
    }
}

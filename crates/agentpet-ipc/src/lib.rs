//! Local daemon IPC transport for AgentPet.
//!
//! The desktop shell and the background daemon run on the same machine
//! and talk over a local-only transport: a Windows named pipe or a Unix
//! domain socket. Messages are length-prefixed frames, and both peers
//! must prove knowledge of a per-install shared secret before any
//! protocol message is exchanged.
//!
//! The crate is intentionally independent from the domain crates: it owns
//! only the transport, framing, and authentication concerns.

#![deny(missing_docs)]

pub mod client;
pub mod endpoint;
pub mod error;
pub mod framing;
pub mod handshake;
pub mod secret;
pub mod server;
pub mod transport;

pub use client::{connect, connect_unauthenticated};
pub use endpoint::Endpoint;
pub use error::{IpcError, IpcErrorCode};
pub use framing::{DEFAULT_MAX_FRAME_BYTES, LENGTH_PREFIX_BYTES};
pub use handshake::PROTOCOL_VERSION;
pub use secret::{load_or_create_secret, secret_path, LocalSecret};
pub use server::DaemonServer;
pub use transport::DaemonConnection;

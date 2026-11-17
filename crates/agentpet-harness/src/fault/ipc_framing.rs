//! Fault injection and fuzzing for authenticated IPC framing and handshake protocols.
//!
//! Validates:
//! - Truncated length headers reject with `IpcError::MalformedFrame`.
//! - Oversized frames reject immediately before buffer allocation with `IpcError::FrameTooLarge`.
//! - Zero-length payloads reject with `IpcError::MalformedFrame`.
//! - Truncated payloads reject with `IpcError::TransportUnavailable`.
//! - Tampered HMAC tags reject with `IpcError::AuthenticationFailed`.
//! - Protocol version mismatches reject with `IpcError::ProtocolVersionUnsupported`.

use std::io::Cursor;

use agentpet_ipc::error::IpcError;
use agentpet_ipc::framing::{read_frame, DEFAULT_MAX_FRAME_BYTES};
use agentpet_ipc::handshake::{
    build_client_response, verify_client_response, ClientResponse, ServerChallenge,
    PROTOCOL_VERSION,
};

/// Asserts that a truncated frame header (< 4 bytes) is rejected with `MalformedFrame`.
pub async fn assert_truncated_header_rejected(header_bytes: &[u8]) {
    assert!(
        header_bytes.len() < 4,
        "Header must be less than 4 bytes to test truncation"
    );
    let mut cursor = Cursor::new(header_bytes);
    let result = read_frame(&mut cursor, DEFAULT_MAX_FRAME_BYTES).await;
    match result {
        Err(IpcError::MalformedFrame { reason }) => {
            assert_eq!(reason, "truncated length header");
        }
        other => panic!("Expected MalformedFrame(truncated length header), got: {:?}", other),
    }
}

/// Asserts that an oversized frame header is rejected immediately with `FrameTooLarge`.
pub async fn assert_oversized_frame_rejected(declared_length: usize, limit: usize) {
    assert!(declared_length > limit, "Declared length must exceed limit");
    let length_bytes = (declared_length as u32).to_be_bytes();
    let mut cursor = Cursor::new(length_bytes);
    let result = read_frame(&mut cursor, limit).await;
    match result {
        Err(IpcError::FrameTooLarge { limit: reported_limit }) => {
            assert_eq!(reported_limit, limit);
        }
        other => panic!("Expected FrameTooLarge, got: {:?}", other),
    }
}

/// Asserts that a zero-length payload header is rejected with `MalformedFrame`.
pub async fn assert_zero_length_payload_rejected() {
    let header_bytes = 0_u32.to_be_bytes();
    let mut cursor = Cursor::new(header_bytes);
    let result = read_frame(&mut cursor, DEFAULT_MAX_FRAME_BYTES).await;
    match result {
        Err(IpcError::MalformedFrame { reason }) => {
            assert_eq!(reason, "empty frame payload");
        }
        other => panic!("Expected MalformedFrame(empty frame payload), got: {:?}", other),
    }
}

/// Asserts that a truncated payload (fewer bytes than declared in header) rejects with `TransportUnavailable`.
pub async fn assert_truncated_payload_rejected(declared_length: u32, actual_bytes: &[u8]) {
    assert!(
        (actual_bytes.len() as u32) < declared_length,
        "Actual bytes must be fewer than declared length"
    );
    let mut buffer = Vec::new();
    buffer.extend_from_slice(&declared_length.to_be_bytes());
    buffer.extend_from_slice(actual_bytes);

    let mut cursor = Cursor::new(buffer);
    let result = read_frame(&mut cursor, DEFAULT_MAX_FRAME_BYTES).await;
    match result {
        Err(IpcError::TransportUnavailable(_)) => {}
        other => panic!("Expected TransportUnavailable on truncated payload, got: {:?}", other),
    }
}

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};

/// Asserts that an invalid or tampered HMAC authentication tag is rejected during handshake.
pub fn assert_tampered_hmac_rejected(secret: &[u8]) {
    let challenge = ServerChallenge {
        protocol_version: PROTOCOL_VERSION,
        nonce: agentpet_ipc::handshake::random_nonce().expect("random server nonce"),
    };
    let client_nonce = agentpet_ipc::handshake::random_nonce().expect("random client nonce");
    let mut response = build_client_response(
        secret,
        PROTOCOL_VERSION,
        &challenge.nonce,
        &client_nonce,
    )
    .expect("build valid response");

    // Tamper with the tag using arbitrary valid base64 bytes
    response.tag = BASE64.encode([0_u8; 32]);

    let result = verify_client_response(secret, &challenge, &response);
    match result {
        Err(IpcError::AuthenticationFailed) => {}
        other => panic!("Expected AuthenticationFailed on tampered HMAC tag, got: {:?}", other),
    }
}

/// Asserts that a mismatched protocol version is rejected during handshake.
pub fn assert_unsupported_version_rejected(secret: &[u8], invalid_version: u16) {
    let challenge = ServerChallenge {
        protocol_version: PROTOCOL_VERSION,
        nonce: agentpet_ipc::handshake::random_nonce().expect("random server nonce"),
    };
    let response = ClientResponse {
        protocol_version: invalid_version,
        client_nonce: agentpet_ipc::handshake::random_nonce().expect("random client nonce"),
        tag: BASE64.encode([0_u8; 32]),
    };

    let result = verify_client_response(secret, &challenge, &response);
    match result {
        Err(IpcError::ProtocolVersionUnsupported { version }) => {
            assert_eq!(version, invalid_version.max(PROTOCOL_VERSION));
        }
        other => panic!("Expected ProtocolVersionUnsupported, got: {:?}", other),
    }
}

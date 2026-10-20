//! Mutual authentication handshake for the local daemon transport.
//!
//! The shared secret never crosses the wire. Instead both peers prove
//! knowledge of it by answering a nonce challenge with an HMAC-SHA256
//! tag. The exchange is mutual, so a peer that only knows the public
//! endpoint name cannot impersonate the daemon.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use subtle::ConstantTimeEq;

use crate::error::IpcError;

type HmacSha256 = Hmac<Sha256>;

/// Version of the local daemon transport protocol.
pub const PROTOCOL_VERSION: u16 = 1;

/// Context string bound into every authentication tag.
const TAG_CONTEXT: &[u8] = b"agentpet-daemon-auth-v1";

/// Number of random bytes in each handshake nonce.
pub const NONCE_BYTES: usize = 32;

/// Server challenge sent to the connecting peer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ServerChallenge {
    /// Transport protocol version advertised by the daemon.
    pub protocol_version: u16,
    /// Random challenge value.
    pub nonce: String,
}

/// Client answer to the server challenge.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClientResponse {
    /// Transport protocol version advertised by the client.
    pub protocol_version: u16,
    /// Random value chosen by the client.
    pub client_nonce: String,
    /// Authentication tag proving the client knows the shared secret.
    pub tag: String,
}

/// Server proof that it also knows the shared secret.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ServerProof {
    /// Authentication tag proving the daemon knows the shared secret.
    pub tag: String,
}

/// Computes the authentication tag for a nonce pair.
fn compute_tag(secret: &[u8], first: &[u8], second: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC accepts any key length");
    mac.update(TAG_CONTEXT);
    mac.update(first);
    mac.update(second);
    BASE64.encode(mac.finalize().into_bytes())
}

/// Compares two base64 tags in constant time.
fn tags_match(expected: &str, actual: &str) -> bool {
    let expected = match BASE64.decode(expected) {
        Ok(value) => value,
        Err(_) => return false,
    };
    let actual = match BASE64.decode(actual) {
        Ok(value) => value,
        Err(_) => return false,
    };
    if expected.len() != actual.len() {
        return false;
    }
    expected.as_slice().ct_eq(actual.as_slice()).into()
}

/// Builds the client answer for a server challenge.
pub fn build_client_response(
    secret: &[u8],
    protocol_version: u16,
    server_nonce: &str,
    client_nonce: &str,
) -> Result<ClientResponse, IpcError> {
    let server_nonce_bytes = BASE64
        .decode(server_nonce)
        .map_err(|_| IpcError::AuthenticationFailed)?;
    let client_nonce_bytes = BASE64
        .decode(client_nonce)
        .map_err(|_| IpcError::AuthenticationFailed)?;
    let tag = compute_tag(secret, &server_nonce_bytes, &client_nonce_bytes);
    Ok(ClientResponse {
        protocol_version,
        client_nonce: client_nonce.to_owned(),
        tag,
    })
}

/// Builds the server proof for a completed client answer.
pub fn build_server_proof(
    secret: &[u8],
    server_nonce: &str,
    client_nonce: &str,
) -> Result<ServerProof, IpcError> {
    let server_nonce_bytes = BASE64
        .decode(server_nonce)
        .map_err(|_| IpcError::AuthenticationFailed)?;
    let client_nonce_bytes = BASE64
        .decode(client_nonce)
        .map_err(|_| IpcError::AuthenticationFailed)?;
    let tag = compute_tag(secret, &client_nonce_bytes, &server_nonce_bytes);
    Ok(ServerProof { tag })
}

/// Verifies the client answer against the shared secret.
pub fn verify_client_response(
    secret: &[u8],
    challenge: &ServerChallenge,
    response: &ClientResponse,
) -> Result<(), IpcError> {
    if response.protocol_version != PROTOCOL_VERSION
        || challenge.protocol_version != PROTOCOL_VERSION
    {
        return Err(IpcError::ProtocolVersionUnsupported {
            version: response.protocol_version.max(challenge.protocol_version),
        });
    }

    let expected = build_client_response(
        secret,
        response.protocol_version,
        &challenge.nonce,
        &response.client_nonce,
    )?;
    if tags_match(&expected.tag, &response.tag) {
        Ok(())
    } else {
        Err(IpcError::AuthenticationFailed)
    }
}

/// Verifies the server proof against the shared secret.
pub fn verify_server_proof(
    secret: &[u8],
    challenge: &ServerChallenge,
    response: &ClientResponse,
    proof: &ServerProof,
) -> Result<(), IpcError> {
    let expected = build_server_proof(secret, &challenge.nonce, &response.client_nonce)?;
    if tags_match(&expected.tag, &proof.tag) {
        Ok(())
    } else {
        Err(IpcError::AuthenticationFailed)
    }
}

/// Generates a random base64 nonce.
pub fn random_nonce() -> Result<String, IpcError> {
    let mut material = vec![0_u8; NONCE_BYTES];
    getrandom::fill(&mut material).map_err(|_| IpcError::AuthenticationFailed)?;
    Ok(BASE64.encode(material))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::IpcErrorCode;

    const SECRET: &[u8] = b"0123456789abcdef0123456789abcdef";

    fn exchange() -> (ServerChallenge, ClientResponse, ServerProof) {
        let challenge = ServerChallenge {
            protocol_version: PROTOCOL_VERSION,
            nonce: random_nonce().expect("nonce"),
        };
        let client_nonce = random_nonce().expect("nonce");
        let response =
            build_client_response(SECRET, PROTOCOL_VERSION, &challenge.nonce, &client_nonce)
                .expect("response built");
        let proof = build_server_proof(SECRET, &challenge.nonce, &response.client_nonce)
            .expect("proof built");
        (challenge, response, proof)
    }

    #[test]
    fn accepts_a_correct_exchange() {
        let (challenge, response, proof) = exchange();
        verify_client_response(SECRET, &challenge, &response).expect("client verified");
        verify_server_proof(SECRET, &challenge, &response, &proof).expect("server verified");
    }

    #[test]
    fn rejects_a_wrong_secret() {
        let (challenge, response, proof) = exchange();
        let other = b"ffffffffffffffffffffffffffffffff";
        assert_eq!(
            verify_client_response(other, &challenge, &response)
                .unwrap_err()
                .code(),
            IpcErrorCode::AuthenticationFailed
        );
        assert_eq!(
            verify_server_proof(other, &challenge, &response, &proof)
                .unwrap_err()
                .code(),
            IpcErrorCode::AuthenticationFailed
        );
    }

    #[test]
    fn rejects_a_tampered_tag() {
        let (challenge, mut response, _) = exchange();
        response.tag = BASE64.encode([0_u8; 32]);
        assert_eq!(
            verify_client_response(SECRET, &challenge, &response)
                .unwrap_err()
                .code(),
            IpcErrorCode::AuthenticationFailed
        );
    }

    #[test]
    fn rejects_a_mismatched_version() {
        let (mut challenge, response, _) = exchange();
        challenge.protocol_version = PROTOCOL_VERSION + 1;
        assert_eq!(
            verify_client_response(SECRET, &challenge, &response)
                .unwrap_err()
                .code(),
            IpcErrorCode::ProtocolVersionUnsupported
        );
    }

    #[test]
    fn rejects_a_swapped_client_nonce() {
        let (challenge, response, _) = exchange();
        let mut swapped = response.clone();
        swapped.client_nonce = random_nonce().expect("nonce");
        assert_eq!(
            verify_client_response(SECRET, &challenge, &swapped)
                .unwrap_err()
                .code(),
            IpcErrorCode::AuthenticationFailed
        );
    }

    #[test]
    fn nonces_are_unique_and_sized() {
        let first = random_nonce().expect("nonce");
        let second = random_nonce().expect("nonce");
        assert_ne!(first, second);
        assert_eq!(BASE64.decode(&first).expect("base64").len(), NONCE_BYTES);
    }
}

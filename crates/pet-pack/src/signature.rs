//! Detached Ed25519 signature verification for PetPack manifests.
//!
//! A signed pack carries an additional `pet.sig` file next to `pet.json`:
//!
//! ```json
//! { "algorithm": "ed25519", "keyId": "example.registry", "signature": "<base64>" }
//! ```
//!
//! The signature covers the raw `pet.json` bytes. Verification only succeeds
//! against a locally configured trusted signer; unknown keys are rejected.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

use crate::PetPackError;

pub const SIGNATURE_FILE: &str = "pet.sig";
pub const SUPPORTED_SIGNATURE_ALGORITHM: &str = "ed25519";
const MAX_SIGNATURE_FILE_BYTES: u64 = 16 * 1024;
const MAX_KEY_ID_BYTES: usize = 128;

/// Detached signature document stored as `pet.sig` inside a pack.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackSignature {
    pub algorithm: String,
    pub key_id: String,
    pub signature: String,
}

impl PackSignature {
    pub fn parse(bytes: &[u8]) -> Result<Self, PetPackError> {
        if bytes.is_empty() || bytes.len() as u64 > MAX_SIGNATURE_FILE_BYTES {
            return Err(PetPackError::SignatureFormat(
                "signature file is empty or too large".into(),
            ));
        }
        let signature: Self = serde_json::from_slice(bytes)
            .map_err(|error| PetPackError::SignatureFormat(error.to_string()))?;
        if signature.algorithm != SUPPORTED_SIGNATURE_ALGORITHM {
            return Err(PetPackError::SignatureFormat(format!(
                "unsupported signature algorithm {}",
                signature.algorithm
            )));
        }
        if signature.key_id.trim().is_empty()
            || signature.key_id.len() > MAX_KEY_ID_BYTES
            || signature.key_id.chars().any(char::is_control)
        {
            return Err(PetPackError::SignatureFormat(
                "signature keyId is empty, too long, or contains control characters".into(),
            ));
        }
        BASE64
            .decode(signature.signature.as_bytes())
            .map_err(|error| PetPackError::SignatureFormat(error.to_string()))?;
        Ok(signature)
    }
}

/// A locally trusted signing identity (public key plus stable identifier).
#[derive(Debug, Clone)]
pub struct TrustedSigner {
    pub key_id: String,
    pub verifying_key: VerifyingKey,
}

impl TrustedSigner {
    pub fn from_base64(key_id: &str, public_key_base64: &str) -> Result<Self, PetPackError> {
        if key_id.trim().is_empty() || key_id.len() > MAX_KEY_ID_BYTES {
            return Err(PetPackError::SignatureFormat(
                "trusted signer keyId is empty or too long".into(),
            ));
        }
        let bytes = BASE64
            .decode(public_key_base64.as_bytes())
            .map_err(|error| PetPackError::SignatureFormat(error.to_string()))?;
        let array: [u8; 32] = bytes
            .try_into()
            .map_err(|_| PetPackError::SignatureFormat("public key must be 32 bytes".into()))?;
        let verifying_key = VerifyingKey::from_bytes(&array).map_err(|error| {
            PetPackError::SignatureFormat(format!("invalid public key: {error}"))
        })?;
        Ok(Self {
            key_id: key_id.to_owned(),
            verifying_key,
        })
    }
}

/// Verifies `signature` over `manifest_bytes` against `trusted` signers.
/// Returns the identifier of the trusted key that verified the signature.
pub fn verify_manifest_signature(
    manifest_bytes: &[u8],
    signature: &PackSignature,
    trusted: &[TrustedSigner],
) -> Result<String, PetPackError> {
    let signer = trusted
        .iter()
        .find(|signer| signer.key_id == signature.key_id)
        .ok_or_else(|| PetPackError::SignatureUntrusted(signature.key_id.clone()))?;
    let signature_bytes = BASE64
        .decode(signature.signature.as_bytes())
        .map_err(|error| PetPackError::SignatureFormat(error.to_string()))?;
    let signature = Signature::from_slice(&signature_bytes)
        .map_err(|error| PetPackError::SignatureFormat(error.to_string()))?;
    signer
        .verifying_key
        .verify(manifest_bytes, &signature)
        .map_err(|_| PetPackError::SignatureVerificationFailed)?;
    Ok(signer.key_id.clone())
}

#[cfg(test)]
mod tests {
    use ed25519_dalek::{Signer, SigningKey};
    use serde_json::json;

    use super::*;

    fn signed_payload() -> (String, Vec<u8>, PackSignature) {
        let signing_key = SigningKey::from_bytes(&[7u8; 32]);
        let public_key = BASE64.encode(signing_key.verifying_key().to_bytes());
        let manifest_bytes = br#"{"schemaVersion":1}"#.to_vec();
        let signature_b64 = BASE64.encode(signing_key.sign(&manifest_bytes).to_bytes());
        let pack_signature = PackSignature::parse(
            json!({
                "algorithm": "ed25519",
                "keyId": "test.signer",
                "signature": signature_b64,
            })
            .to_string()
            .as_bytes(),
        )
        .expect("signature parses");
        (public_key, manifest_bytes, pack_signature)
    }

    #[test]
    fn accepts_signature_from_a_trusted_signer() {
        let (public_key, manifest_bytes, signature) = signed_payload();
        let trusted =
            vec![TrustedSigner::from_base64("test.signer", &public_key)
                .expect("trusted signer decodes")];
        assert_eq!(
            verify_manifest_signature(&manifest_bytes, &signature, &trusted)
                .expect("signature verifies"),
            "test.signer"
        );
    }

    #[test]
    fn rejects_tampered_manifest_and_untrusted_keys() {
        let (public_key, mut manifest_bytes, signature) = signed_payload();
        let trusted =
            vec![TrustedSigner::from_base64("test.signer", &public_key)
                .expect("trusted signer decodes")];
        manifest_bytes.push(b' ');
        assert!(matches!(
            verify_manifest_signature(&manifest_bytes, &signature, &trusted),
            Err(PetPackError::SignatureVerificationFailed)
        ));
        assert!(matches!(
            verify_manifest_signature(&manifest_bytes, &signature, &[]),
            Err(PetPackError::SignatureUntrusted(_))
        ));
    }

    #[test]
    fn rejects_malformed_signature_documents() {
        assert!(
            PackSignature::parse(br#"{"algorithm":"rsa","keyId":"k","signature":"AA=="}"#).is_err()
        );
        assert!(
            PackSignature::parse(br#"{"algorithm":"ed25519","keyId":"","signature":"AA=="}"#)
                .is_err()
        );
        assert!(
            PackSignature::parse(br#"{"algorithm":"ed25519","keyId":"k","signature":"!!"}"#)
                .is_err()
        );
        assert!(TrustedSigner::from_base64("k", "short").is_err());
    }
}

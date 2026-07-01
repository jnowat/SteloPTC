// WP-60: Ed25519 signing for regulatory export bundles (FDA 21 CFR Part 11
// attestation packages). Ed25519 replaces the RSA-4096 originally sketched
// in the ROADMAP — see migration_044_signing_keys for the documented
// rationale. An inspector verifies a signed document against the bundled
// public key directly; there is no certificate-authority chain to validate,
// since this is a self-attested lab signature, not a third-party-issued
// certificate.
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;

pub struct SigningKeypair {
    pub public_key_b64: String,
    pub private_key_b64: String,
}

/// Generates a fresh Ed25519 keypair, base64-encoded for storage in
/// `signing_keys`.
pub fn generate_keypair() -> SigningKeypair {
    let signing_key = SigningKey::generate(&mut OsRng);
    SigningKeypair {
        public_key_b64: B64.encode(signing_key.verifying_key().to_bytes()),
        private_key_b64: B64.encode(signing_key.to_bytes()),
    }
}

/// Signs `data` with the private key, returning a base64-encoded detached
/// signature (stored/exported as a `.sig` file alongside the signed document).
pub fn sign(private_key_b64: &str, data: &[u8]) -> Result<String, String> {
    let key_bytes = B64.decode(private_key_b64).map_err(|e| format!("Invalid private key: {}", e))?;
    let key_array: [u8; 32] = key_bytes.try_into().map_err(|_| "Private key must be 32 bytes".to_string())?;
    let signing_key = SigningKey::from_bytes(&key_array);
    let signature: Signature = signing_key.sign(data);
    Ok(B64.encode(signature.to_bytes()))
}

/// Verifies a detached signature against `data` and the public key. Returns
/// `Ok(true)`/`Ok(false)` for a well-formed check (never panics on
/// attacker-controlled input); `Err` only for a malformed key/signature encoding.
pub fn verify(public_key_b64: &str, data: &[u8], signature_b64: &str) -> Result<bool, String> {
    let key_bytes = B64.decode(public_key_b64).map_err(|e| format!("Invalid public key: {}", e))?;
    let key_array: [u8; 32] = key_bytes.try_into().map_err(|_| "Public key must be 32 bytes".to_string())?;
    let verifying_key = VerifyingKey::from_bytes(&key_array).map_err(|e| format!("Invalid public key: {}", e))?;

    let sig_bytes = B64.decode(signature_b64).map_err(|e| format!("Invalid signature encoding: {}", e))?;
    let sig_array: [u8; 64] = sig_bytes.try_into().map_err(|_| "Signature must be 64 bytes".to_string())?;
    let signature = Signature::from_bytes(&sig_array);

    Ok(verifying_key.verify(data, &signature).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_then_verify_round_trip_passes() {
        let keypair = generate_keypair();
        let data = b"FDA 21 CFR Part 11 attestation cover document";
        let signature = sign(&keypair.private_key_b64, data).unwrap();
        assert!(verify(&keypair.public_key_b64, data, &signature).unwrap());
    }

    #[test]
    fn verify_fails_for_tampered_data() {
        let keypair = generate_keypair();
        let signature = sign(&keypair.private_key_b64, b"original document").unwrap();
        assert!(!verify(&keypair.public_key_b64, b"tampered document", &signature).unwrap());
    }

    #[test]
    fn verify_fails_for_wrong_public_key() {
        let keypair_a = generate_keypair();
        let keypair_b = generate_keypair();
        let signature = sign(&keypair_a.private_key_b64, b"data").unwrap();
        assert!(!verify(&keypair_b.public_key_b64, b"data", &signature).unwrap());
    }

    #[test]
    fn each_generated_keypair_is_unique() {
        let a = generate_keypair();
        let b = generate_keypair();
        assert_ne!(a.public_key_b64, b.public_key_b64);
        assert_ne!(a.private_key_b64, b.private_key_b64);
    }
}

// WP-59: zero-knowledge encryption core for cloud backup. A cloud storage
// provider (or anyone who obtains the encrypted blob) never has enough
// information to decrypt it — the master key is derived from a
// user-supplied passphrase via Argon2id and is never itself persisted to
// disk; only the caller decides whether to cache it (e.g. in the OS
// keychain, wired up by the frontend, not here).
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use argon2::Argon2;
use rand::RngCore;

/// 4-byte magic + 1-byte format version, written at the start of every
/// encrypted backup blob so `decrypt` can fail fast on a non-SteloPTC file
/// instead of producing a confusing AEAD-tag-mismatch error.
const MAGIC: &[u8; 4] = b"STEL";
const FORMAT_VERSION: u8 = 1;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;

/// Argon2id parameters: 128 MiB memory, 3 iterations, 4-way parallelism —
/// the exact parameters named in ROADMAP.md WP-59.
fn argon2_params() -> argon2::Params {
    argon2::Params::new(128 * 1024, 3, 4, Some(32)).expect("valid Argon2id params")
}

/// Derives a 256-bit AES key from `passphrase` + `salt` via Argon2id. The
/// same passphrase + salt always yields the same key (required for restore);
/// a fresh random salt should be generated once per backup target and
/// stored alongside the encrypted config, not reused as a global secret.
///
/// Returns `Err` rather than panicking: Argon2id allocates a 128 MiB memory
/// block at call time, and on a memory-constrained device that allocation can
/// fail at runtime. A backup/restore command must surface that as a clean
/// error, not unwind a panic across the Tauri command boundary.
pub fn derive_key(passphrase: &str, salt: &[u8]) -> Result<[u8; 32], String> {
    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, argon2_params());
    let mut key = [0u8; 32];
    argon2
        .hash_password_into(passphrase.as_bytes(), salt, &mut key)
        .map_err(|e| format!("Key derivation failed (out of memory?): {}", e))?;
    Ok(key)
}

/// Generates a fresh random 16-byte salt for a new backup target.
pub fn generate_salt() -> [u8; SALT_LEN] {
    let mut salt = [0u8; SALT_LEN];
    rand::thread_rng().fill_bytes(&mut salt);
    salt
}

/// Encrypts `plaintext` with AES-256-GCM under `key`, using a fresh random
/// 96-bit nonce per call (AES-GCM's security requires the (key, nonce) pair
/// to never repeat). Output layout: `MAGIC (4) | version (1) | nonce (12) |
/// ciphertext+tag`. The nonce is not secret — it must travel with the
/// ciphertext for decryption — only the derived key is.
///
/// Returns `Err` rather than panicking so the crypto API is uniformly
/// fallible (`derive_key`/`encrypt`/`decrypt` all return `Result`). In
/// practice AES-GCM encryption only fails if the plaintext exceeds GCM's
/// ~64 GiB limit, which no SQLite backup will reach — but a Tauri command
/// should surface even that as an error string, never a panic.
pub fn encrypt(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, String> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| format!("Encryption failed: {}", e))?;

    let mut out = Vec::with_capacity(4 + 1 + NONCE_LEN + ciphertext.len());
    out.extend_from_slice(MAGIC);
    out.push(FORMAT_VERSION);
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

/// Decrypts a blob produced by `encrypt`. Fails with a clear, distinct error
/// for each of: too-short input, wrong magic (not a SteloPTC backup blob),
/// unsupported format version, and AEAD authentication failure (wrong key OR
/// tampered/corrupted ciphertext — AES-GCM cannot distinguish the two, which
/// is the correct security property: a wrong-key error must look identical
/// to a tamper error, or an attacker could use error messages to mount a
/// key-guessing oracle).
pub fn decrypt(key: &[u8; 32], blob: &[u8]) -> Result<Vec<u8>, String> {
    if blob.len() < 4 + 1 + NONCE_LEN {
        return Err("Backup blob is too short to be valid".to_string());
    }
    if &blob[0..4] != MAGIC {
        return Err("Not a SteloPTC encrypted backup (bad magic header)".to_string());
    }
    let version = blob[4];
    if version != FORMAT_VERSION {
        return Err(format!("Unsupported backup format version {}", version));
    }
    let nonce = Nonce::from_slice(&blob[5..5 + NONCE_LEN]);
    let ciphertext = &blob[5 + NONCE_LEN..];

    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| "Decryption failed — wrong passphrase, or the backup is corrupted/tampered".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_round_trip_with_correct_key() {
        let salt = generate_salt();
        let key = derive_key("correct horse battery staple", &salt).unwrap();
        let plaintext = b"this is definitely a SQLite database file, trust me";
        let blob = encrypt(&key, plaintext).unwrap();
        let decrypted = decrypt(&key, &blob).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn decrypt_with_wrong_passphrase_fails_cleanly() {
        let salt = generate_salt();
        let key = derive_key("correct horse battery staple", &salt).unwrap();
        let wrong_key = derive_key("incorrect horse", &salt).unwrap();
        let blob = encrypt(&key, b"secret lab data").unwrap();
        let err = decrypt(&wrong_key, &blob).unwrap_err();
        assert!(err.contains("wrong passphrase") || err.contains("corrupted"));
    }

    #[test]
    fn decrypt_with_wrong_salt_fails_even_with_same_passphrase() {
        let key_a = derive_key("shared passphrase", &generate_salt()).unwrap();
        let key_b = derive_key("shared passphrase", &generate_salt()).unwrap();
        assert_ne!(key_a, key_b, "different salts must yield different keys");
        let blob = encrypt(&key_a, b"data").unwrap();
        assert!(decrypt(&key_b, &blob).is_err());
    }

    #[test]
    fn nonce_is_unique_across_calls() {
        let key = derive_key("passphrase", &generate_salt()).unwrap();
        let blob1 = encrypt(&key, b"same plaintext").unwrap();
        let blob2 = encrypt(&key, b"same plaintext").unwrap();
        let nonce1 = &blob1[5..17];
        let nonce2 = &blob2[5..17];
        assert_ne!(nonce1, nonce2, "each encryption must use a fresh nonce");
        assert_ne!(blob1, blob2, "identical plaintext must not produce identical ciphertext");
    }

    #[test]
    fn decrypt_detects_bad_magic_header() {
        let key = derive_key("passphrase", &generate_salt()).unwrap();
        let mut blob = encrypt(&key, b"data").unwrap();
        blob[0] = b'X';
        let err = decrypt(&key, &blob).unwrap_err();
        assert!(err.contains("bad magic"));
    }

    #[test]
    fn decrypt_detects_truncated_blob() {
        let key = derive_key("passphrase", &generate_salt()).unwrap();
        assert!(decrypt(&key, b"STEL").is_err());
    }

    #[test]
    fn tampered_ciphertext_is_rejected() {
        let key = derive_key("passphrase", &generate_salt()).unwrap();
        let mut blob = encrypt(&key, b"important lab data").unwrap();
        let last = blob.len() - 1;
        blob[last] ^= 0xFF;
        assert!(decrypt(&key, &blob).is_err());
    }

    #[test]
    fn derive_key_is_deterministic_for_same_input() {
        let salt = generate_salt();
        let key1 = derive_key("my passphrase", &salt).unwrap();
        let key2 = derive_key("my passphrase", &salt).unwrap();
        assert_eq!(key1, key2, "restore must be able to re-derive the same key");
    }
}

// WP-59: backup target configuration — zero-knowledge encrypted storage and
// cron schedule validation.
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use serde::{Deserialize, Serialize};

use super::crypto;

/// The plaintext shape of a backup target's connection details. Only ever
/// exists in memory (during encrypt/decrypt); the database only ever sees
/// the encrypted blob produced by `encrypt_target_config`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TargetConfig {
    pub endpoint: Option<String>,
    pub bucket_or_path: String,
    pub access_key: Option<String>,
    pub secret_key: Option<String>,
}

/// Encrypts `config` under a key derived from `passphrase` and a freshly
/// generated salt, returning `base64(salt || encrypted_blob)` — a single
/// self-contained string safe to store directly in
/// `backup_targets.config_encrypted`. The salt is not secret; it must travel
/// with the ciphertext so the same key can be re-derived later.
pub fn encrypt_target_config(passphrase: &str, config: &TargetConfig) -> Result<String, String> {
    let json = serde_json::to_vec(config).map_err(|e| e.to_string())?;
    let salt = crypto::generate_salt();
    let key = crypto::derive_key(passphrase, &salt);
    let blob = crypto::encrypt(&key, &json);

    let mut combined = Vec::with_capacity(salt.len() + blob.len());
    combined.extend_from_slice(&salt);
    combined.extend_from_slice(&blob);
    Ok(B64.encode(combined))
}

/// Reverses `encrypt_target_config`. Returns a clear error (never a panic)
/// for a wrong passphrase or corrupted stored value.
pub fn decrypt_target_config(passphrase: &str, stored_b64: &str) -> Result<TargetConfig, String> {
    let combined = B64.decode(stored_b64).map_err(|e| format!("Corrupted stored config: {}", e))?;
    if combined.len() < 16 {
        return Err("Corrupted stored config: too short".to_string());
    }
    let (salt, blob) = combined.split_at(16);
    let key = crypto::derive_key(passphrase, salt);
    let json = crypto::decrypt(&key, blob)?;
    serde_json::from_slice(&json).map_err(|e| format!("Corrupted stored config JSON: {}", e))
}

/// Validates a standard 5-field cron expression (`minute hour day month
/// weekday`). This is a syntactic validator, not a scheduler — it exists so
/// the UI can reject an obviously malformed schedule before it's saved.
/// Each field may be `*`, a single number, a comma-separated list, a
/// `low-high` range, or a `*/step` — the common subset every cron
/// implementation supports.
pub fn is_valid_cron(expr: &str) -> bool {
    let fields: Vec<&str> = expr.split_whitespace().collect();
    if fields.len() != 5 {
        return false;
    }
    let ranges: [(i64, i64); 5] = [(0, 59), (0, 23), (1, 31), (1, 12), (0, 7)];
    fields.iter().zip(ranges).all(|(field, (lo, hi))| is_valid_cron_field(field, lo, hi))
}

fn is_valid_cron_field(field: &str, lo: i64, hi: i64) -> bool {
    if field == "*" {
        return true;
    }
    field.split(',').all(|part| {
        if let Some((base, step)) = part.split_once('/') {
            let step_ok = step.parse::<i64>().map(|s| s > 0).unwrap_or(false);
            let base_ok = base == "*" || is_valid_cron_number(base, lo, hi);
            step_ok && base_ok
        } else if let Some((a, b)) = part.split_once('-') {
            match (a.parse::<i64>(), b.parse::<i64>()) {
                (Ok(a), Ok(b)) => a <= b && a >= lo && b <= hi,
                _ => false,
            }
        } else {
            is_valid_cron_number(part, lo, hi)
        }
    })
}

fn is_valid_cron_number(s: &str, lo: i64, hi: i64) -> bool {
    s.parse::<i64>().map(|n| n >= lo && n <= hi).unwrap_or(false)
}

/// Human-friendly size formatting for `last_backup_size_bytes` display.
pub fn format_size_bytes(bytes: i64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    if unit_idx == 0 {
        format!("{} {}", bytes, UNITS[0])
    } else {
        format!("{:.1} {}", size, UNITS[unit_idx])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config() -> TargetConfig {
        TargetConfig {
            endpoint: Some("https://s3.example.com".to_string()),
            bucket_or_path: "my-lab-bucket".to_string(),
            access_key: Some("AKIA...".to_string()),
            secret_key: Some("super-secret".to_string()),
        }
    }

    #[test]
    fn target_config_encrypt_decrypt_round_trip() {
        let config = sample_config();
        let stored = encrypt_target_config("target passphrase", &config).unwrap();
        let recovered = decrypt_target_config("target passphrase", &stored).unwrap();
        assert_eq!(recovered, config);
    }

    #[test]
    fn target_config_wrong_passphrase_fails() {
        let config = sample_config();
        let stored = encrypt_target_config("right passphrase", &config).unwrap();
        assert!(decrypt_target_config("wrong passphrase", &stored).is_err());
    }

    #[test]
    fn cron_valid_expressions_accepted() {
        for expr in ["0 2 * * *", "*/15 * * * *", "0 0 1 1 *", "0,30 * * * 0-5"] {
            assert!(is_valid_cron(expr), "'{}' should be valid", expr);
        }
    }

    #[test]
    fn cron_invalid_expressions_rejected() {
        for expr in ["not a cron", "* * * *", "60 * * * *", "* 24 * * *", "* * 32 * *", "* * * 13 *"] {
            assert!(!is_valid_cron(expr), "'{}' should be invalid", expr);
        }
    }

    #[test]
    fn format_size_bytes_scales_units() {
        assert_eq!(format_size_bytes(512), "512 B");
        assert_eq!(format_size_bytes(2048), "2.0 KB");
        assert_eq!(format_size_bytes(5 * 1024 * 1024), "5.0 MB");
    }
}

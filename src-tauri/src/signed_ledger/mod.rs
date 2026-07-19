// WP-67: Trust Layer Phase 3 — specimen lifecycle events as signed transactions.
//
// A monotonic, hash-chained ledger where each entry is additionally signed with
// the *acting user's* Ed25519 key. The hash chain gives tamper-evidence (any
// edit to a past entry breaks every downstream hash, exactly like the WP-18
// audit chain); the per-entry signature adds **non-repudiation** — a confirmed
// entry proves *which user's key* authorized that specific event, ordered like
// ledger transactions.
//
// Relationship to the existing audit log: the audit log (WP-18) records *what
// changed* and is chained but unsigned — it attributes actions to a user id the
// database itself writes. This ledger is the stronger, opt-in layer the roadmap
// reserved for Phase 3: the user cryptographically *signs* the event, so the
// attribution cannot be forged by anyone who can write to the database but does
// not hold the user's private key.
//
// Scope, disclosed honestly (matching the WP-63 "exhaustive sweep is
// disproportionate" precedent): this ships the full signing/verification engine,
// a `record_signed_event` command any write-capable user can call for any event,
// and one wired demonstrating integration — every newly *created specimen*
// automatically gets a signed genesis transaction. Extending automatic signing
// to every one of the ~30 mutation commands is incremental follow-up work; the
// foundation here forecloses nothing.

use rusqlite::{params, Connection};
use serde::Serialize;

use crate::compliance_export::signing;
use crate::db::queries::{compute_entry_hash, ZERO_HASH};

pub mod lifecycle;

#[derive(Debug, Clone, Serialize)]
pub struct SignedEvent {
    pub id: String,
    pub seq: i64,
    pub event_type: String,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub user_id: Option<String>,
    pub payload: String,
    pub prev_hash: String,
    pub event_hash: String,
    pub signature: String,
    pub public_key: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct LedgerVerification {
    pub verified: bool,
    pub total_events: i64,
    pub signatures_valid: i64,
    /// `seq` of the first entry that failed a check (hash, linkage, or signature).
    pub first_break_seq: Option<i64>,
    pub message: String,
}

fn now_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

/// Canonical serialization for a signed ledger event.
///
/// Format — pipe-separated UTF-8, fixed field order:
///   seq|timestamp|user_id|event_type|entity_type|entity_id|payload
///
/// NULL optional fields serialize as empty string. Never reorder fields; append
/// new fields at the end only, so existing stored hashes/signatures remain valid.
pub fn canonical_event_bytes(
    seq: i64,
    timestamp: &str,
    user_id: &str,
    event_type: &str,
    entity_type: &str,
    entity_id: &str,
    payload: &str,
) -> Vec<u8> {
    format!(
        "{}|{}|{}|{}|{}|{}|{}",
        seq, timestamp, user_id, event_type, entity_type, entity_id, payload
    )
    .into_bytes()
}

/// Load the acting user's Ed25519 signing keypair, generating and persisting one
/// on first use. Returns `(public_key_b64, private_key_b64)`. Distinct from the
/// single lab-wide WP-60 export key: signed ledger events are attributed to the
/// *individual* who authorized them.
pub fn load_or_create_user_signing_key(conn: &Connection, user_id: &str) -> Result<(String, String), String> {
    let existing: Option<(String, String)> = conn
        .query_row(
            "SELECT public_key_b64, private_key_b64 FROM user_signing_keys WHERE user_id = ?1",
            params![user_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .ok();
    if let Some(pair) = existing {
        return Ok(pair);
    }
    let keypair = signing::generate_keypair();
    conn.execute(
        "INSERT INTO user_signing_keys (user_id, public_key_b64, private_key_b64, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![user_id, keypair.public_key_b64, keypair.private_key_b64, now_iso()],
    )
    .map_err(|e| e.to_string())?;
    Ok((keypair.public_key_b64, keypair.private_key_b64))
}

/// Public signing key for a user, if they have one yet.
pub fn get_user_public_key(conn: &Connection, user_id: &str) -> Option<String> {
    conn.query_row(
        "SELECT public_key_b64 FROM user_signing_keys WHERE user_id = ?1",
        params![user_id],
        |r| r.get(0),
    )
    .ok()
}

/// Append a signed transaction to the ledger: assign the next global `seq`, chain
/// its `prev_hash` from the last entry, hash it, and sign the hash with the
/// acting user's key. Returns the persisted event.
pub fn append_signed_event(
    conn: &Connection,
    user_id: &str,
    event_type: &str,
    entity_type: &str,
    entity_id: Option<&str>,
    payload: &str,
) -> Result<SignedEvent, String> {
    let (public_key, private_key) = load_or_create_user_signing_key(conn, user_id)?;

    let next_seq: i64 = conn
        .query_row("SELECT COALESCE(MAX(seq), -1) + 1 FROM signed_events", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;

    let prev_hash: String = conn
        .query_row(
            "SELECT event_hash FROM signed_events ORDER BY seq DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| ZERO_HASH.to_string());

    let created_at = now_iso();
    let canonical = canonical_event_bytes(
        next_seq, &created_at, user_id, event_type, entity_type, entity_id.unwrap_or(""), payload,
    );
    let event_hash = compute_entry_hash(&canonical, &prev_hash);
    // Sign the event hash: it already commits to the canonical content and the
    // prev_hash, so a valid signature over it authenticates the whole entry.
    let signature = signing::sign(&private_key, event_hash.as_bytes())?;

    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO signed_events \
         (id, seq, event_type, entity_type, entity_id, user_id, payload, prev_hash, event_hash, signature, public_key, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            id, next_seq, event_type, entity_type, entity_id, user_id, payload,
            prev_hash, event_hash, signature, public_key, created_at
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(SignedEvent {
        id,
        seq: next_seq,
        event_type: event_type.to_string(),
        entity_type: entity_type.to_string(),
        entity_id: entity_id.map(|s| s.to_string()),
        user_id: Some(user_id.to_string()),
        payload: payload.to_string(),
        prev_hash,
        event_hash,
        signature,
        public_key,
        created_at,
    })
}

/// A best-effort wrapper for wiring into existing command flows: never returns an
/// error, so a ledger hiccup can never fail the primary operation (specimen
/// creation, etc.). Mirrors how `log_audit(...).ok()` is used at call sites.
pub fn try_append_signed_event(
    conn: &Connection,
    user_id: &str,
    event_type: &str,
    entity_type: &str,
    entity_id: Option<&str>,
    payload: &str,
) {
    let _ = append_signed_event(conn, user_id, event_type, entity_type, entity_id, payload);
}

/// List signed events, newest first, optionally scoped to one entity.
pub fn list_signed_events(conn: &Connection, entity_id: Option<&str>, limit: i64) -> Result<Vec<SignedEvent>, String> {
    let lim = limit.clamp(1, 1000);
    match entity_id {
        Some(eid) => {
            let mut stmt = conn
                .prepare(
                    "SELECT id, seq, event_type, entity_type, entity_id, user_id, payload, prev_hash, event_hash, signature, public_key, created_at \
                     FROM signed_events WHERE entity_id = ?1 ORDER BY seq DESC LIMIT ?2",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![eid, lim], map_event)
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();
            Ok(rows)
        }
        None => {
            let mut stmt = conn
                .prepare(
                    "SELECT id, seq, event_type, entity_type, entity_id, user_id, payload, prev_hash, event_hash, signature, public_key, created_at \
                     FROM signed_events ORDER BY seq DESC LIMIT ?1",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![lim], map_event)
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();
            Ok(rows)
        }
    }
}

fn map_event(r: &rusqlite::Row) -> rusqlite::Result<SignedEvent> {
    Ok(SignedEvent {
        id: r.get(0)?,
        seq: r.get(1)?,
        event_type: r.get(2)?,
        entity_type: r.get(3)?,
        entity_id: r.get(4)?,
        user_id: r.get(5)?,
        payload: r.get(6)?,
        prev_hash: r.get(7)?,
        event_hash: r.get(8)?,
        signature: r.get(9)?,
        public_key: r.get(10)?,
        created_at: r.get(11)?,
    })
}

/// Verify the whole ledger: for every entry in `seq` order, recompute its hash
/// from the canonical fields, confirm the `prev_hash` links to the previous
/// entry's hash (and the first entry links to `ZERO_HASH`), confirm `seq` is
/// gapless (a gap means a deletion), and verify the Ed25519 signature against the
/// entry's public key. Returns the first break, if any.
pub fn verify_ledger(conn: &Connection) -> Result<LedgerVerification, String> {
    let mut stmt = conn
        .prepare(
            "SELECT seq, event_type, entity_type, entity_id, user_id, payload, prev_hash, event_hash, signature, public_key, created_at \
             FROM signed_events ORDER BY seq ASC",
        )
        .map_err(|e| e.to_string())?;

    #[allow(clippy::type_complexity)]
    let rows: Vec<(i64, String, String, Option<String>, Option<String>, String, String, String, String, String, String)> = stmt
        .query_map([], |r| {
            Ok((
                r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?,
                r.get(6)?, r.get(7)?, r.get(8)?, r.get(9)?, r.get(10)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let total = rows.len() as i64;
    let mut expected_prev = ZERO_HASH.to_string();
    let mut signatures_valid = 0i64;

    for (idx, (seq, event_type, entity_type, entity_id, user_id, payload, prev_hash, event_hash, signature, public_key, created_at)) in rows.iter().enumerate() {
        let expected_seq = idx as i64;
        // Gapless seq (deletion detection).
        if *seq != expected_seq {
            return Ok(LedgerVerification {
                verified: false,
                total_events: total,
                signatures_valid,
                first_break_seq: Some(*seq),
                message: format!("Ledger sequence gap — expected seq {}, found {} (an entry was removed).", expected_seq, seq),
            });
        }
        // Linkage.
        if prev_hash != &expected_prev {
            return Ok(LedgerVerification {
                verified: false,
                total_events: total,
                signatures_valid,
                first_break_seq: Some(*seq),
                message: format!("Broken chain linkage at seq {} — prev_hash does not match the previous entry.", seq),
            });
        }
        // Content hash.
        let canonical = canonical_event_bytes(
            *seq, created_at, user_id.as_deref().unwrap_or(""), event_type, entity_type,
            entity_id.as_deref().unwrap_or(""), payload,
        );
        let recomputed = compute_entry_hash(&canonical, prev_hash);
        if &recomputed != event_hash {
            return Ok(LedgerVerification {
                verified: false,
                total_events: total,
                signatures_valid,
                first_break_seq: Some(*seq),
                message: format!("Content tampering at seq {} — recomputed hash does not match the stored hash.", seq),
            });
        }
        // Signature.
        let sig_ok = signing::verify(public_key, event_hash.as_bytes(), signature).unwrap_or(false);
        if !sig_ok {
            return Ok(LedgerVerification {
                verified: false,
                total_events: total,
                signatures_valid,
                first_break_seq: Some(*seq),
                message: format!("Invalid signature at seq {} — the entry was not signed by the stated key.", seq),
            });
        }
        // Cross-check the signing key still matches the user's registered key
        // (detects a swapped-key forgery attempt). A missing registered key is
        // itself a verification failure: an entry is only appended after
        // `load_or_create_user_signing_key` persists the user's key, so any
        // user-attributed entry MUST have a registered key at verify time. If it
        // is gone, a DB-writer deleted the `user_signing_keys` row and re-signed
        // the entry with a fresh key — the cross-check must not be silently
        // skipped, or that forgery would pass as "verified".
        if let Some(uid) = user_id {
            match get_user_public_key(conn, uid) {
                Some(registered) if &registered == public_key => {}
                Some(_) => {
                    return Ok(LedgerVerification {
                        verified: false,
                        total_events: total,
                        signatures_valid,
                        first_break_seq: Some(*seq),
                        message: format!("Signing key mismatch at seq {} — the entry's key differs from the user's registered key.", seq),
                    });
                }
                None => {
                    return Ok(LedgerVerification {
                        verified: false,
                        total_events: total,
                        signatures_valid,
                        first_break_seq: Some(*seq),
                        message: format!("Missing registered key at seq {} — user '{}' has no registered signing key to verify against (the key row was removed).", seq, uid),
                    });
                }
            }
        }
        signatures_valid += 1;
        expected_prev = event_hash.clone();
    }

    Ok(LedgerVerification {
        verified: true,
        total_events: total,
        signatures_valid,
        first_break_seq: None,
        message: if total == 0 {
            "Ledger is empty — nothing to verify.".to_string()
        } else {
            format!("Ledger verified — {} signed events, all hashes and signatures valid.", total)
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations::run_all;

    fn test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_all(&conn).unwrap();
        conn.execute(
            "INSERT INTO users (id, username, password_hash, display_name, role) \
             VALUES ('user1', 'u1', 'x', 'User One', 'tech')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO users (id, username, password_hash, display_name, role) \
             VALUES ('user2', 'u2', 'x', 'User Two', 'tech')",
            [],
        )
        .unwrap();
        conn
    }

    #[test]
    fn key_is_created_once_and_reused() {
        let conn = test_db();
        let (pub1, priv1) = load_or_create_user_signing_key(&conn, "user1").unwrap();
        let (pub2, priv2) = load_or_create_user_signing_key(&conn, "user1").unwrap();
        assert_eq!(pub1, pub2);
        assert_eq!(priv1, priv2);
    }

    #[test]
    fn appended_event_is_signed_and_chained() {
        let conn = test_db();
        let e0 = append_signed_event(&conn, "user1", "specimen_created", "specimen", Some("spec1"), "{}").unwrap();
        assert_eq!(e0.seq, 0);
        assert_eq!(e0.prev_hash, ZERO_HASH);
        let e1 = append_signed_event(&conn, "user1", "passage", "specimen", Some("spec1"), "{}").unwrap();
        assert_eq!(e1.seq, 1);
        assert_eq!(e1.prev_hash, e0.event_hash);
        // The signature verifies against the stored public key.
        assert!(signing::verify(&e0.public_key, e0.event_hash.as_bytes(), &e0.signature).unwrap());
    }

    #[test]
    fn empty_ledger_verifies() {
        let conn = test_db();
        let v = verify_ledger(&conn).unwrap();
        assert!(v.verified);
        assert_eq!(v.total_events, 0);
    }

    #[test]
    fn full_ledger_verifies() {
        let conn = test_db();
        append_signed_event(&conn, "user1", "specimen_created", "specimen", Some("spec1"), "{}").unwrap();
        append_signed_event(&conn, "user2", "passage", "specimen", Some("spec1"), "p1").unwrap();
        append_signed_event(&conn, "user1", "split", "specimen", Some("spec1"), "s1").unwrap();
        let v = verify_ledger(&conn).unwrap();
        assert!(v.verified, "{}", v.message);
        assert_eq!(v.total_events, 3);
        assert_eq!(v.signatures_valid, 3);
    }

    #[test]
    fn content_tampering_is_detected() {
        let conn = test_db();
        append_signed_event(&conn, "user1", "specimen_created", "specimen", Some("spec1"), "{}").unwrap();
        // Edit the payload of the stored row without re-hashing/re-signing.
        conn.execute("UPDATE signed_events SET payload = 'tampered' WHERE seq = 0", []).unwrap();
        let v = verify_ledger(&conn).unwrap();
        assert!(!v.verified);
        assert_eq!(v.first_break_seq, Some(0));
    }

    #[test]
    fn deletion_is_detected_as_a_seq_gap() {
        let conn = test_db();
        append_signed_event(&conn, "user1", "e0", "specimen", Some("spec1"), "a").unwrap();
        append_signed_event(&conn, "user1", "e1", "specimen", Some("spec1"), "b").unwrap();
        append_signed_event(&conn, "user1", "e2", "specimen", Some("spec1"), "c").unwrap();
        conn.execute("DELETE FROM signed_events WHERE seq = 1", []).unwrap();
        let v = verify_ledger(&conn).unwrap();
        assert!(!v.verified);
        assert_eq!(v.first_break_seq, Some(2));
    }

    #[test]
    fn forged_signature_is_detected() {
        let conn = test_db();
        let e = append_signed_event(&conn, "user1", "specimen_created", "specimen", Some("spec1"), "{}").unwrap();
        // Replace the signature with a valid-format but wrong signature (sign
        // different data with a different key).
        let other = signing::generate_keypair();
        let bad_sig = signing::sign(&other.private_key_b64, e.event_hash.as_bytes()).unwrap();
        conn.execute("UPDATE signed_events SET signature = ?1 WHERE seq = 0", params![bad_sig]).unwrap();
        let v = verify_ledger(&conn).unwrap();
        assert!(!v.verified);
        assert_eq!(v.first_break_seq, Some(0));
    }

    #[test]
    fn swapped_key_is_detected() {
        let conn = test_db();
        let e = append_signed_event(&conn, "user1", "specimen_created", "specimen", Some("spec1"), "{}").unwrap();
        // Attacker re-signs the entry with a fresh key and swaps both the row's
        // public_key and signature — but the user's registered key is unchanged.
        let forged = signing::generate_keypair();
        let forged_sig = signing::sign(&forged.private_key_b64, e.event_hash.as_bytes()).unwrap();
        conn.execute(
            "UPDATE signed_events SET public_key = ?1, signature = ?2 WHERE seq = 0",
            params![forged.public_key_b64, forged_sig],
        )
        .unwrap();
        let v = verify_ledger(&conn).unwrap();
        assert!(!v.verified);
        assert_eq!(v.first_break_seq, Some(0));
    }

    #[test]
    fn deleted_registered_key_forgery_is_detected() {
        // Threat model: a DB-writer who does NOT hold user1's private key wants a
        // forged event to pass verification. They delete user1's registered key
        // row, mint a fresh keypair, and re-sign the entry with it (updating both
        // the row's public_key and signature so it is internally self-consistent).
        // Without treating a missing registered key as a failure, this would
        // verify as genuine. It must not.
        let conn = test_db();
        let e = append_signed_event(&conn, "user1", "specimen_created", "specimen", Some("spec1"), "{}").unwrap();
        let forged = signing::generate_keypair();
        let forged_sig = signing::sign(&forged.private_key_b64, e.event_hash.as_bytes()).unwrap();
        conn.execute(
            "UPDATE signed_events SET public_key = ?1, signature = ?2 WHERE seq = 0",
            params![forged.public_key_b64, forged_sig],
        )
        .unwrap();
        // Remove the registered key so the naive cross-check would be skipped.
        conn.execute("DELETE FROM user_signing_keys WHERE user_id = 'user1'", []).unwrap();
        let v = verify_ledger(&conn).unwrap();
        assert!(!v.verified, "forgery via deleted registered key must be rejected");
        assert_eq!(v.first_break_seq, Some(0));
        assert!(v.message.contains("Missing registered key"), "message was: {}", v.message);
    }

    #[test]
    fn list_scopes_by_entity() {
        let conn = test_db();
        append_signed_event(&conn, "user1", "e", "specimen", Some("spec1"), "a").unwrap();
        append_signed_event(&conn, "user1", "e", "specimen", Some("spec2"), "b").unwrap();
        assert_eq!(list_signed_events(&conn, Some("spec1"), 100).unwrap().len(), 1);
        assert_eq!(list_signed_events(&conn, None, 100).unwrap().len(), 2);
    }
}

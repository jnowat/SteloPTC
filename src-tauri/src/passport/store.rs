// WP-70: connection-level specimen-passport lifecycle. Pure functions over a
// rusqlite `Connection` (no Tauri) so the full issue → verify → import flow is
// unit-testable against an in-memory migrated database, exactly as the
// `anchoring::store` and `compliance_export::bundle` helpers are. The thin
// `commands::passport` layer only adds session/role gating on top of these.
use rusqlite::{params, Connection};
use serde::Serialize;

use super::{
    assemble_and_sign, parse_passport, verify_passport, IssuerIdentity, PassportAuditEntry,
    PassportMerkleAnchor, PassportSpecimen, PassportVerification, SpecimenPassport,
};
use crate::compliance_export::load_or_create_lab_signing_key;
use crate::db::queries::{audit_canonical_bytes, build_merkle_root, log_audit};

/// Default issuer lab name used until an operator sets one in Settings.
pub const DEFAULT_LAB_NAME: &str = "Unnamed SteloPTC Lab";

/// A summary row for the passport register (issued + imported), without the full
/// JSON payload.
#[derive(Debug, Serialize)]
pub struct PassportRecord {
    pub id: String,
    pub passport_id: String,
    pub direction: String,
    pub specimen_id: Option<String>,
    pub issuer_lab: String,
    pub issuer_public_key: String,
    pub subject_accession: String,
    pub subject_scientific_name: Option<String>,
    pub content_hash: String,
    pub entry_count: i64,
    pub verified: bool,
    pub created_at: String,
}

/// Outcome of importing a passport: the verification verdict plus the local
/// audit-log row that now commits the import into this lab's own chain.
#[derive(Debug, Serialize)]
pub struct ImportPassportResult {
    pub imported: bool,
    pub passport_id: String,
    pub local_row_id: String,
    pub audit_entry_id: Option<String>,
    pub verification: PassportVerification,
}

fn now_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

/// Read this lab's issuer name from `app_settings` (`lab_name`), falling back to
/// the default when unset.
pub fn read_lab_name(conn: &Connection) -> String {
    crate::db::queries::read_setting(conn, "lab_name", DEFAULT_LAB_NAME)
}

/// Persist this lab's issuer name.
pub fn set_lab_name(conn: &Connection, name: &str) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Lab name cannot be empty.".to_string());
    }
    conn.execute(
        "INSERT OR REPLACE INTO app_settings (key, value, updated_at) VALUES ('lab_name', ?1, ?2)",
        params![trimmed, now_iso()],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// This lab's public issuer identity — the name plus the lab-wide Ed25519 public
/// key (the same WP-60 export key), generating the key on first use. An operator
/// shares this out-of-band so partner labs can verify the passports it issues.
pub fn get_lab_identity(conn: &Connection) -> Result<IssuerIdentity, String> {
    let (public_key, _) = load_or_create_lab_signing_key(conn)?;
    Ok(IssuerIdentity { lab_name: read_lab_name(conn), public_key })
}

/// The full signing keypair for issuing (identity + private key).
fn load_signing_identity(conn: &Connection) -> Result<(IssuerIdentity, String), String> {
    let (public_key, private_key) = load_or_create_lab_signing_key(conn)?;
    Ok((IssuerIdentity { lab_name: read_lab_name(conn), public_key }, private_key))
}

/// Gather a specimen's provenance as passport audit entries — every hashed
/// audit-log row for the specimen's lineage, in ascending `chain_seq`, in the
/// exact shape a verifier needs to recompute each hash. Mirrors
/// `commands::audit::export_audit_proof`'s entry construction.
pub fn gather_provenance(conn: &Connection, specimen_id: &str) -> Result<Vec<PassportAuditEntry>, String> {
    struct Row {
        chain_seq: i64,
        user_id: Option<String>,
        entity_type: String,
        action: String,
        entity_id: Option<String>,
        created_at: String,
        details: Option<String>,
        prev_hash: String,
        entry_hash: String,
    }
    let mut stmt = conn
        .prepare(
            "SELECT chain_seq, user_id, entity_type, action, entity_id, created_at, details, prev_hash, entry_hash \
             FROM audit_log \
             WHERE lineage_id = ?1 AND entry_hash IS NOT NULL \
             ORDER BY chain_seq ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows: Vec<Row> = stmt
        .query_map(params![specimen_id], |r| {
            Ok(Row {
                chain_seq: r.get(0)?,
                user_id: r.get(1)?,
                entity_type: r.get(2)?,
                action: r.get(3)?,
                entity_id: r.get(4)?,
                created_at: r.get(5)?,
                details: r.get(6)?,
                prev_hash: r.get(7)?,
                entry_hash: r.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let entries = rows
        .iter()
        .map(|row| {
            let canonical = audit_canonical_bytes(
                specimen_id,
                row.chain_seq,
                &row.created_at,
                row.user_id.as_deref().unwrap_or(""),
                &row.entity_type,
                row.entity_id.as_deref().unwrap_or(""),
                &row.action,
                row.details.as_deref().unwrap_or(""),
            );
            PassportAuditEntry {
                chain_seq: row.chain_seq,
                canonical: String::from_utf8_lossy(&canonical).to_string(),
                prev_hash: row.prev_hash.clone(),
                entry_hash: row.entry_hash.clone(),
            }
        })
        .collect();
    Ok(entries)
}

/// Attach a Merkle anchor only when a checkpoint for this lineage seals **exactly**
/// the exported entries — i.e. the root rebuilt from the passport's entry hashes
/// reproduces the checkpoint's stored root. This keeps the anchor honest: it is
/// present only when a verifier's own rebuild will match it. Returns None when no
/// checkpoint covers the full set (the common case).
fn gather_merkle_anchor(conn: &Connection, specimen_id: &str, entries: &[PassportAuditEntry]) -> Option<PassportMerkleAnchor> {
    if entries.is_empty() {
        return None;
    }
    let rebuilt = build_merkle_root(&entries.iter().map(|e| e.entry_hash.clone()).collect::<Vec<_>>());
    // Newest checkpoint first — prefer the most recent full-coverage seal.
    let mut stmt = conn
        .prepare(
            "SELECT id, merkle_root, anchored_txid FROM audit_checkpoints \
             WHERE lineage_id = ?1 ORDER BY end_seq DESC, created_at DESC",
        )
        .ok()?;
    let rows = stmt
        .query_map(params![specimen_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, Option<String>>(2)?))
        })
        .ok()?
        .filter_map(|r| r.ok())
        .collect::<Vec<_>>();
    for (checkpoint_id, merkle_root, anchored_txid) in rows {
        if merkle_root == rebuilt {
            return Some(PassportMerkleAnchor { checkpoint_id, merkle_root, anchored_txid });
        }
    }
    None
}

/// Build the specimen identity subset for a passport from the `specimens` table.
fn load_passport_specimen(conn: &Connection, specimen_id: &str) -> Result<PassportSpecimen, String> {
    conn.query_row(
        "SELECT s.id, s.accession_number, \
                sp.genus || ' ' || sp.species_name AS scientific_name, \
                s.strain_id, s.stage, s.generation, s.origin_type, s.provenance, s.initiation_date \
         FROM specimens s LEFT JOIN species sp ON s.species_id = sp.id \
         WHERE s.id = ?1",
        params![specimen_id],
        |r| {
            Ok(PassportSpecimen {
                specimen_id: r.get(0)?,
                accession_number: r.get(1)?,
                scientific_name: r.get(2)?,
                strain_id: r.get(3)?,
                stage: r.get(4)?,
                generation: r.get(5)?,
                origin_type: r.get(6)?,
                provenance_note: r.get(7)?,
                initiation_date: r.get(8)?,
            })
        },
    )
    .map_err(|_| format!("Specimen '{}' not found.", specimen_id))
}

/// Issue a signed passport for a local specimen, record it (direction `issued`),
/// and return the full document.
pub fn issue_passport(conn: &Connection, specimen_id: &str, created_by: Option<&str>) -> Result<SpecimenPassport, String> {
    let (issuer, private_key) = load_signing_identity(conn)?;
    let specimen = load_passport_specimen(conn, specimen_id)?;
    let provenance = gather_provenance(conn, specimen_id)?;
    if provenance.is_empty() {
        return Err(
            "This specimen has no hashed provenance entries to attest — cannot issue a passport.".to_string(),
        );
    }
    let anchor = gather_merkle_anchor(conn, specimen_id, &provenance);

    let passport = assemble_and_sign(
        uuid::Uuid::new_v4().to_string(),
        now_iso(),
        issuer.clone(),
        specimen.clone(),
        provenance.clone(),
        anchor,
        &private_key,
    )?;

    let passport_json = serde_json::to_string_pretty(&passport).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO specimen_passports \
         (id, passport_id, direction, specimen_id, issuer_lab, issuer_public_key, subject_accession, \
          subject_scientific_name, content_hash, entry_count, verified, audit_entry, passport_json, created_by, created_at) \
         VALUES (?1, ?2, 'issued', ?3, ?4, ?5, ?6, ?7, ?8, ?9, 1, NULL, ?10, ?11, ?12)",
        params![
            uuid::Uuid::new_v4().to_string(),
            passport.passport_id,
            specimen_id,
            issuer.lab_name,
            issuer.public_key,
            specimen.accession_number,
            specimen.scientific_name,
            passport.content_hash,
            provenance.len() as i64,
            passport_json,
            created_by,
            passport.issued_at,
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(passport)
}

/// Verify a passport JSON with no side effects — pure verification, no import.
pub fn verify_passport_json(json: &str) -> Result<PassportVerification, String> {
    let passport = parse_passport(json)?;
    Ok(verify_passport(&passport))
}

/// Import a received passport: verify it, refuse an invalid or duplicate one, then
/// fold it into this lab's own audit chain (a `passport_imported` entry that
/// commits to the passport's content hash) and record it (direction `imported`).
pub fn import_passport(conn: &Connection, json: &str, imported_by: Option<&str>) -> Result<ImportPassportResult, String> {
    let passport = parse_passport(json)?;
    let verification = verify_passport(&passport);
    if !verification.verified {
        return Err(format!("Refusing to import an unverifiable passport: {}", verification.message));
    }

    // Do not import the same passport twice.
    let already: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM specimen_passports WHERE direction = 'imported' AND passport_id = ?1",
            params![passport.passport_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if already > 0 {
        return Err(format!(
            "Passport '{}' (accession {}) has already been imported.",
            passport.passport_id, passport.specimen.accession_number
        ));
    }

    // Fold the import into this lab's own tamper-evident audit chain. The entry's
    // entity_id is the passport id (so it starts its own single-entry lineage);
    // new_value is the content hash it commits to.
    let details = format!(
        "Imported specimen passport for accession {} from {} (content {}).",
        passport.specimen.accession_number,
        passport.issuer.lab_name,
        &passport.content_hash[..passport.content_hash.len().min(16)]
    );
    log_audit(
        conn,
        imported_by,
        "import",
        "specimen_passport",
        Some(&passport.passport_id),
        None,
        Some(&passport.content_hash),
        Some(&details),
    )
    .map_err(|e| e.to_string())?;

    // Recover the id of the audit row we just wrote for the durable link.
    let audit_entry_id: Option<String> = conn
        .query_row(
            "SELECT id FROM audit_log WHERE entity_type = 'specimen_passport' AND entity_id = ?1 AND action = 'import' \
             ORDER BY created_at DESC LIMIT 1",
            params![passport.passport_id],
            |r| r.get(0),
        )
        .ok();

    let local_row_id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO specimen_passports \
         (id, passport_id, direction, specimen_id, issuer_lab, issuer_public_key, subject_accession, \
          subject_scientific_name, content_hash, entry_count, verified, audit_entry, passport_json, created_by, created_at) \
         VALUES (?1, ?2, 'imported', NULL, ?3, ?4, ?5, ?6, ?7, ?8, 1, ?9, ?10, ?11, ?12)",
        params![
            local_row_id,
            passport.passport_id,
            passport.issuer.lab_name,
            passport.issuer.public_key,
            passport.specimen.accession_number,
            passport.specimen.scientific_name,
            passport.content_hash,
            verification.entry_count,
            audit_entry_id,
            json,
            imported_by,
            now_iso(),
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(ImportPassportResult {
        imported: true,
        passport_id: passport.passport_id,
        local_row_id,
        audit_entry_id,
        verification,
    })
}

/// List passport register rows, newest first, optionally filtered by direction.
pub fn list_passports(conn: &Connection, direction: Option<&str>) -> Result<Vec<PassportRecord>, String> {
    let cols = "id, passport_id, direction, specimen_id, issuer_lab, issuer_public_key, subject_accession, \
                subject_scientific_name, content_hash, entry_count, verified, created_at";
    let map = |r: &rusqlite::Row| -> rusqlite::Result<PassportRecord> {
        Ok(PassportRecord {
            id: r.get(0)?,
            passport_id: r.get(1)?,
            direction: r.get(2)?,
            specimen_id: r.get(3)?,
            issuer_lab: r.get(4)?,
            issuer_public_key: r.get(5)?,
            subject_accession: r.get(6)?,
            subject_scientific_name: r.get(7)?,
            content_hash: r.get(8)?,
            entry_count: r.get(9)?,
            verified: r.get::<_, i64>(10)? != 0,
            created_at: r.get(11)?,
        })
    };
    match direction {
        Some(dir) => {
            let mut stmt = conn
                .prepare(&format!(
                    "SELECT {} FROM specimen_passports WHERE direction = ?1 ORDER BY created_at DESC",
                    cols
                ))
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![dir], map)
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();
            Ok(rows)
        }
        None => {
            let mut stmt = conn
                .prepare(&format!("SELECT {} FROM specimen_passports ORDER BY created_at DESC", cols))
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], map)
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();
            Ok(rows)
        }
    }
}

/// Fetch the full stored passport JSON for one register row (for re-export).
pub fn get_passport_json(conn: &Connection, row_id: &str) -> Result<String, String> {
    conn.query_row(
        "SELECT passport_json FROM specimen_passports WHERE id = ?1",
        params![row_id],
        |r| r.get(0),
    )
    .map_err(|_| format!("Passport record '{}' not found.", row_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations::run_all;
    use crate::db::queries::{compute_entry_hash, ZERO_HASH};

    fn test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_all(&conn).unwrap();
        conn.execute(
            "INSERT INTO users (id, username, password_hash, display_name, role) \
             VALUES ('u1', 'u1', 'x', 'User One', 'admin')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO species (id, species_code, genus, species_name) \
             VALUES ('sp1', 'CIT-SIN', 'Citrus', 'sinensis')",
            [],
        )
        .unwrap();
        conn
    }

    /// Create a specimen with a small hashed audit chain and return its id.
    fn seed_specimen(conn: &Connection, id: &str, accession: &str) {
        conn.execute(
            "INSERT INTO specimens (id, accession_number, species_id, stage, initiation_date, generation) \
             VALUES (?1, ?2, 'sp1', 'shoot_meristem', '2026-01-01', 2)",
            params![id, accession],
        )
        .unwrap();
        // Genesis + two continuation entries, forming a valid chain on lineage = id.
        log_audit(conn, Some("u1"), "create", "specimen", Some(id), None, None, Some("created")).unwrap();
        log_audit(conn, Some("u1"), "passage", "specimen", Some(id), None, None, Some("p1")).unwrap();
        log_audit(conn, Some("u1"), "passage", "specimen", Some(id), None, None, Some("p2")).unwrap();
    }

    #[test]
    fn issue_then_verify_round_trips() {
        let conn = test_db();
        seed_specimen(&conn, "spec1", "2026-01-01-CIT-SIN-001");
        let passport = issue_passport(&conn, "spec1", Some("u1")).unwrap();
        assert_eq!(passport.specimen.accession_number, "2026-01-01-CIT-SIN-001");
        assert_eq!(passport.specimen.scientific_name.as_deref(), Some("Citrus sinensis"));
        assert!(!passport.provenance.is_empty());
        let v = verify_passport(&passport);
        assert!(v.verified, "{}", v.message);
        // Recorded as issued.
        let issued = list_passports(&conn, Some("issued")).unwrap();
        assert_eq!(issued.len(), 1);
        assert_eq!(issued[0].passport_id, passport.passport_id);
    }

    #[test]
    fn issue_refuses_specimen_without_provenance() {
        let conn = test_db();
        conn.execute(
            "INSERT INTO specimens (id, accession_number, species_id, stage, initiation_date, generation) \
             VALUES ('bare', '2026-01-01-CIT-SIN-009', 'sp1', 'shoot_meristem', '2026-01-01', 0)",
            [],
        )
        .unwrap();
        assert!(issue_passport(&conn, "bare", Some("u1")).is_err());
    }

    #[test]
    fn import_verifies_and_writes_receiving_audit_entry() {
        // "Origin lab" issues; a fresh "receiving lab" DB imports the JSON.
        let origin = test_db();
        seed_specimen(&origin, "spec1", "2026-01-01-CIT-SIN-001");
        let passport = issue_passport(&origin, "spec1", Some("u1")).unwrap();
        let json = serde_json::to_string_pretty(&passport).unwrap();

        let receiver = test_db();
        let before: i64 = receiver
            .query_row("SELECT COUNT(*) FROM audit_log WHERE entity_type = 'specimen_passport'", [], |r| r.get(0))
            .unwrap();
        let result = import_passport(&receiver, &json, Some("u1")).unwrap();
        assert!(result.imported);
        assert!(result.verification.verified);
        assert!(result.audit_entry_id.is_some());
        let after: i64 = receiver
            .query_row("SELECT COUNT(*) FROM audit_log WHERE entity_type = 'specimen_passport'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(after, before + 1, "import must write exactly one receiving-lab audit entry");
        // Recorded as imported.
        let imported = list_passports(&receiver, Some("imported")).unwrap();
        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].issuer_lab, DEFAULT_LAB_NAME);
    }

    #[test]
    fn duplicate_import_is_rejected() {
        let origin = test_db();
        seed_specimen(&origin, "spec1", "2026-01-01-CIT-SIN-001");
        let json = serde_json::to_string_pretty(&issue_passport(&origin, "spec1", Some("u1")).unwrap()).unwrap();

        let receiver = test_db();
        assert!(import_passport(&receiver, &json, Some("u1")).is_ok());
        assert!(import_passport(&receiver, &json, Some("u1")).is_err());
    }

    #[test]
    fn import_rejects_tampered_passport() {
        let origin = test_db();
        seed_specimen(&origin, "spec1", "2026-01-01-CIT-SIN-001");
        let mut passport = issue_passport(&origin, "spec1", Some("u1")).unwrap();
        passport.specimen.accession_number = "FORGED".to_string(); // breaks content hash
        let json = serde_json::to_string_pretty(&passport).unwrap();

        let receiver = test_db();
        assert!(import_passport(&receiver, &json, Some("u1")).is_err());
        // Nothing recorded.
        assert_eq!(list_passports(&receiver, Some("imported")).unwrap().len(), 0);
    }

    #[test]
    fn lab_name_round_trips_and_appears_in_issued_passport() {
        let conn = test_db();
        assert_eq!(read_lab_name(&conn), DEFAULT_LAB_NAME);
        set_lab_name(&conn, "  Green Thumb Labs  ").unwrap();
        assert_eq!(read_lab_name(&conn), "Green Thumb Labs");
        seed_specimen(&conn, "spec1", "2026-01-01-CIT-SIN-001");
        let passport = issue_passport(&conn, "spec1", Some("u1")).unwrap();
        assert_eq!(passport.issuer.lab_name, "Green Thumb Labs");
        assert!(verify_passport(&passport).verified);
    }

    #[test]
    fn set_lab_name_rejects_empty() {
        let conn = test_db();
        assert!(set_lab_name(&conn, "   ").is_err());
    }

    #[test]
    fn merkle_anchor_attached_when_a_checkpoint_seals_the_whole_lineage() {
        let conn = test_db();
        seed_specimen(&conn, "spec1", "2026-01-01-CIT-SIN-001");
        // Build a checkpoint over the full lineage so the rebuilt root matches.
        let entries = gather_provenance(&conn, "spec1").unwrap();
        let leaves: Vec<String> = entries.iter().map(|e| e.entry_hash.clone()).collect();
        let root = build_merkle_root(&leaves);
        conn.execute(
            "INSERT INTO audit_checkpoints (id, lineage_id, start_seq, end_seq, entry_count, merkle_root, anchored_txid) \
             VALUES ('cp1', 'spec1', ?1, ?2, ?3, ?4, 'txid-xyz')",
            params![entries.first().unwrap().chain_seq, entries.last().unwrap().chain_seq, entries.len() as i64, root],
        )
        .unwrap();
        let passport = issue_passport(&conn, "spec1", Some("u1")).unwrap();
        let anchor = passport.merkle_anchor.as_ref().expect("anchor should be attached");
        assert_eq!(anchor.checkpoint_id, "cp1");
        assert_eq!(anchor.anchored_txid.as_deref(), Some("txid-xyz"));
        assert!(verify_passport(&passport).verified);
    }

    #[test]
    fn no_anchor_when_checkpoint_does_not_cover_full_lineage() {
        let conn = test_db();
        seed_specimen(&conn, "spec1", "2026-01-01-CIT-SIN-001");
        // A checkpoint with a bogus root that cannot match the rebuilt one.
        conn.execute(
            "INSERT INTO audit_checkpoints (id, lineage_id, start_seq, end_seq, entry_count, merkle_root) \
             VALUES ('cp1', 'spec1', 1, 1, 1, ?1)",
            params![ZERO_HASH],
        )
        .unwrap();
        let passport = issue_passport(&conn, "spec1", Some("u1")).unwrap();
        assert!(passport.merkle_anchor.is_none());
    }

    #[test]
    fn get_passport_json_returns_stored_document() {
        let conn = test_db();
        seed_specimen(&conn, "spec1", "2026-01-01-CIT-SIN-001");
        issue_passport(&conn, "spec1", Some("u1")).unwrap();
        let row = &list_passports(&conn, Some("issued")).unwrap()[0];
        let json = get_passport_json(&conn, &row.id).unwrap();
        let parsed = parse_passport(&json).unwrap();
        assert!(verify_passport(&parsed).verified);
    }

    // Guards that gather_provenance produces canonical strings a verifier
    // recomputes identically (regression against a serialization drift).
    #[test]
    fn gathered_entries_recompute_to_their_stored_hash() {
        let conn = test_db();
        seed_specimen(&conn, "spec1", "2026-01-01-CIT-SIN-001");
        let entries = gather_provenance(&conn, "spec1").unwrap();
        assert_eq!(entries.len(), 3);
        for e in &entries {
            assert_eq!(compute_entry_hash(e.canonical.as_bytes(), &e.prev_hash), e.entry_hash);
        }
        assert_eq!(entries[0].prev_hash, ZERO_HASH);
    }
}

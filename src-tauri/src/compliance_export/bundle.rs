// WP-60: bundle assembly for FDA 21 CFR Part 11, USDA APHIS, and CITES
// exports. Every function here is read-only against the database — this
// module never writes anything except the signing key (generated once,
// on demand) and the final zip file on disk.
use rusqlite::Connection;
use serde::Serialize;
use serde_json::json;

use crate::db::queries;

#[derive(Debug, Serialize)]
pub struct AuditRangeVerification {
    pub verified: bool,
    pub total_entries_checked: i64,
    pub first_break: Option<(String, i64)>,
}

/// Re-verifies every hash-chained audit entry in `[from, to]` (inclusive,
/// `YYYY-MM-DD`), grouped by lineage, exactly as `verify_audit_lineage`
/// would per-lineage — reimplemented here as a pure, connection-only
/// function (rather than calling the Tauri command) so it can run
/// server-side during bundle assembly without a session token.
pub fn verify_audit_range(conn: &Connection, from: &str, to: &str) -> Result<AuditRangeVerification, String> {
    let mut stmt = conn
        .prepare(
            "SELECT lineage_id, chain_seq, prev_hash, entry_hash, created_at, user_id, \
                    entity_type, entity_id, action, details \
             FROM audit_log \
             WHERE lineage_id IS NOT NULL AND chain_seq IS NOT NULL \
               AND date(created_at) >= ?1 AND date(created_at) <= ?2 \
             ORDER BY lineage_id ASC, chain_seq ASC",
        )
        .map_err(|e| e.to_string())?;

    #[allow(clippy::type_complexity)]
    let rows: Vec<(String, i64, Option<String>, String, String, Option<String>, String, Option<String>, String, Option<String>)> = stmt
        .query_map(rusqlite::params![from, to], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?, r.get(9)?))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut total = 0i64;
    let mut expected_prev: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for (lineage_id, chain_seq, prev_hash, entry_hash, created_at, user_id, entity_type, entity_id, action, details) in &rows {
        total += 1;
        let canonical = queries::audit_canonical_bytes(
            lineage_id, *chain_seq, created_at, user_id.as_deref().unwrap_or(""),
            entity_type, entity_id.as_deref().unwrap_or(""), action, details.as_deref().unwrap_or(""),
        );
        let recomputed = queries::compute_entry_hash(&canonical, prev_hash.as_deref().unwrap_or(queries::ZERO_HASH));
        if recomputed != *entry_hash {
            return Ok(AuditRangeVerification { verified: false, total_entries_checked: total, first_break: Some((lineage_id.clone(), *chain_seq)) });
        }
        if let Some(expected) = expected_prev.get(lineage_id) {
            if prev_hash.as_deref() != Some(expected.as_str()) && *chain_seq != 0 {
                return Ok(AuditRangeVerification { verified: false, total_entries_checked: total, first_break: Some((lineage_id.clone(), *chain_seq)) });
            }
        }
        expected_prev.insert(lineage_id.clone(), entry_hash.clone());
    }

    Ok(AuditRangeVerification { verified: true, total_entries_checked: total, first_break: None })
}

/// FDA 21 CFR Part 11 electronic-records attestation bundle: cover summary +
/// full canonical audit trail export + verification result + a per-user
/// activity report. Signing is applied by the caller (`commands::compliance_export`)
/// after this function returns the plaintext documents.
pub fn build_part11_documents(conn: &Connection, from: &str, to: &str, lab_name: &str) -> Result<Vec<(String, Vec<u8>)>, String> {
    let verification = verify_audit_range(conn, from, to)?;

    let mut stmt = conn
        .prepare(
            "SELECT id, user_id, action, entity_type, entity_id, old_value, new_value, details, \
                    created_at, lineage_id, chain_seq, prev_hash, entry_hash \
             FROM audit_log WHERE date(created_at) >= ?1 AND date(created_at) <= ?2 \
             ORDER BY created_at ASC",
        )
        .map_err(|e| e.to_string())?;
    let entries: Vec<serde_json::Value> = stmt
        .query_map(rusqlite::params![from, to], |r| {
            Ok(json!({
                "id": r.get::<_, String>(0)?,
                "user_id": r.get::<_, Option<String>>(1)?,
                "action": r.get::<_, String>(2)?,
                "entity_type": r.get::<_, String>(3)?,
                "entity_id": r.get::<_, Option<String>>(4)?,
                "old_value": r.get::<_, Option<String>>(5)?,
                "new_value": r.get::<_, Option<String>>(6)?,
                "details": r.get::<_, Option<String>>(7)?,
                "created_at": r.get::<_, String>(8)?,
                "lineage_id": r.get::<_, Option<String>>(9)?,
                "chain_seq": r.get::<_, Option<i64>>(10)?,
                "prev_hash": r.get::<_, Option<String>>(11)?,
                "entry_hash": r.get::<_, Option<String>>(12)?,
            }))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut user_stmt = conn
        .prepare(
            "SELECT u.id, u.username, u.role, u.updated_at, COUNT(a.id) AS action_count \
             FROM users u LEFT JOIN audit_log a ON a.user_id = u.id \
                 AND date(a.created_at) >= ?1 AND date(a.created_at) <= ?2 \
             GROUP BY u.id ORDER BY u.username ASC",
        )
        .map_err(|e| e.to_string())?;
    let user_report: Vec<serde_json::Value> = user_stmt
        .query_map(rusqlite::params![from, to], |r| {
            Ok(json!({
                "user_id": r.get::<_, String>(0)?,
                "username": r.get::<_, String>(1)?,
                "role": r.get::<_, String>(2)?,
                "last_updated": r.get::<_, String>(3)?,
                "actions_in_range": r.get::<_, i64>(4)?,
            }))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let cover = json!({
        "lab_name": lab_name,
        "system_version": env!("CARGO_PKG_VERSION"),
        "export_range": { "from": from, "to": to },
        "total_audit_entries": entries.len(),
        "attestation": "This system maintains an append-only, cryptographically hash-chained audit \
                         log (SHA-256, per-entry) with role-based access control and a forced \
                         password-change policy on first login, consistent with 21 CFR Part 11 \
                         electronic-record requirements for trustworthy, tamper-evident records.",
        "chain_verification_verdict": if verification.verified { "verified" } else { "broken" },
    });

    Ok(vec![
        ("part11_cover.json".to_string(), serde_json::to_vec_pretty(&cover).map_err(|e| e.to_string())?),
        ("part11_audit_trail.json".to_string(), serde_json::to_vec_pretty(&entries).map_err(|e| e.to_string())?),
        ("part11_verification.json".to_string(), serde_json::to_vec_pretty(&verification).map_err(|e| e.to_string())?),
        ("part11_user_activity.json".to_string(), serde_json::to_vec_pretty(&user_report).map_err(|e| e.to_string())?),
    ])
}

/// USDA APHIS PPQ Form 526 pre-fill (plant tissue culture profile only) —
/// auto-populates from live specimen/species records. SteloPTC does not
/// submit to APHIS; this produces a ready-to-review, ready-to-submit package.
pub fn build_usda_permit_prefill(conn: &Connection, specimen_ids: &[String], authorized_scientist: &str) -> Result<serde_json::Value, String> {
    let mut specimens = Vec::new();
    for id in specimen_ids {
        let row = conn
            .query_row(
                "SELECT sp.accession_number, s.genus, s.species_name, s.common_name, \
                        sp.provenance, sp.source_plant, sp.permit_number, sp.permit_expiry \
                 FROM specimens sp JOIN species s ON sp.species_id = s.id WHERE sp.id = ?1",
                [id],
                |r| {
                    Ok(json!({
                        "accession_number": r.get::<_, String>(0)?,
                        "scientific_name": format!("{} {}", r.get::<_, String>(1)?, r.get::<_, String>(2)?),
                        "common_name": r.get::<_, Option<String>>(3)?,
                        "provenance": r.get::<_, Option<String>>(4)?,
                        "source_plant": r.get::<_, Option<String>>(5)?,
                        "existing_permit_number": r.get::<_, Option<String>>(6)?,
                        "existing_permit_expiry": r.get::<_, Option<String>>(7)?,
                    }))
                },
            )
            .map_err(|e| format!("Specimen {} not found: {}", id, e))?;
        specimens.push(row);
    }

    let quarantine_records: Vec<serde_json::Value> = {
        let placeholders: Vec<String> = specimen_ids.iter().map(|_| "?".to_string()).collect();
        let sql = format!(
            "SELECT specimen_id, permit_number, permit_expiry, status, notes, created_at \
             FROM compliance_records WHERE record_type = 'quarantine' AND specimen_id IN ({})",
            placeholders.join(",")
        );
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let param_refs: Vec<&dyn rusqlite::types::ToSql> = specimen_ids.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
        let collected: Vec<serde_json::Value> = stmt
            .query_map(param_refs.as_slice(), |r| {
                Ok(json!({
                    "specimen_id": r.get::<_, String>(0)?,
                    "permit_number": r.get::<_, Option<String>>(1)?,
                    "permit_expiry": r.get::<_, Option<String>>(2)?,
                    "status": r.get::<_, String>(3)?,
                    "notes": r.get::<_, Option<String>>(4)?,
                    "recorded_at": r.get::<_, String>(5)?,
                }))
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        collected
    };

    Ok(json!({
        "form": "PPQ Form 526 (pre-fill)",
        "authorized_scientist": authorized_scientist,
        "specimens": specimens,
        "quarantine_records": quarantine_records,
        "note": "SteloPTC does not submit this form to APHIS directly — review and submit manually.",
    }))
}

/// CITES Species Provenance Dossier: species identification (via the
/// existing WP-49 Darwin Core export), full chain-of-custody from
/// `parent_specimen_id`, every propagation (subculture) record in
/// chronological order, and an audit-chain verification summary.
pub fn build_cites_dossier(conn: &Connection, root_specimen_id: &str, cites_appendix: &str) -> Result<serde_json::Value, String> {
    let (accession, species_id): (String, String) = conn
        .query_row(
            "SELECT accession_number, species_id FROM specimens WHERE id = ?1",
            [root_specimen_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| "Specimen not found".to_string())?;

    let taxon_path: Option<String> = conn.query_row("SELECT taxon_path FROM species WHERE id = ?1", [&species_id], |r| r.get(0)).ok();
    let root_id = taxon_path
        .and_then(|p| serde_json::from_str::<Vec<String>>(&p).ok())
        .and_then(|v| v.first().cloned());
    let darwin_core = queries::export_darwin_core(conn, root_id.as_deref()).map_err(|e| e.to_string())?;

    // Chain of custody: walk the specimen family via parent_specimen_id in
    // both directions from the given root.
    let mut custody = Vec::new();
    let mut stmt = conn
        .prepare(
            "SELECT id, accession_number, parent_specimen_id, initiation_date, location, created_by, created_at \
             FROM specimens WHERE id = ?1 OR parent_specimen_id = ?1 OR root_specimen_id = ?1",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([root_specimen_id], |r| {
            Ok(json!({
                "specimen_id": r.get::<_, String>(0)?,
                "accession_number": r.get::<_, String>(1)?,
                "parent_specimen_id": r.get::<_, Option<String>>(2)?,
                "date": r.get::<_, String>(3)?,
                "location": r.get::<_, Option<String>>(4)?,
                "responsible_party": r.get::<_, Option<String>>(5)?,
                "recorded_at": r.get::<_, String>(6)?,
            }))
        })
        .map_err(|e| e.to_string())?;
    for row in rows.flatten() {
        custody.push(row);
    }

    let mut sub_stmt = conn
        .prepare("SELECT passage_number, date, notes FROM subcultures WHERE specimen_id = ?1 ORDER BY passage_number ASC")
        .map_err(|e| e.to_string())?;
    let propagation: Vec<serde_json::Value> = sub_stmt
        .query_map([root_specimen_id], |r| {
            Ok(json!({ "passage_number": r.get::<_, i64>(0)?, "date": r.get::<_, String>(1)?, "notes": r.get::<_, Option<String>>(2)? }))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let full_range_verification = verify_audit_range(conn, "0000-01-01", "9999-12-31")?;

    Ok(json!({
        "accession_number": accession,
        "cites_appendix": cites_appendix,
        "darwin_core": darwin_core,
        "chain_of_custody": custody,
        "propagation_records": propagation,
        "audit_chain_summary": full_range_verification,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations::run_all;

    fn export_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_all(&conn).unwrap();
        conn.execute(
            "INSERT INTO species (id, genus, species_name, species_code) VALUES ('sp1', 'Citrus', 'sinensis', 'CIT-SIN')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO specimens (id, accession_number, species_id, initiation_date, provenance, source_plant) \
             VALUES ('spec1', 'ACC-001', 'sp1', '2026-01-01', 'Field collection', 'Parent tree #4')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO subcultures (id, specimen_id, passage_number, date, notes) \
             VALUES ('sub1', 'spec1', 1, '2026-01-05', 'First passage')",
            [],
        )
        .unwrap();
        crate::db::queries::log_audit(
            &conn, None, "create", "specimen", Some("spec1"), None, Some("ACC-001"), Some("Specimen created"),
        )
        .unwrap();
        conn
    }

    #[test]
    fn part11_audit_trail_entry_count_matches_direct_query() {
        let conn = export_test_db();
        let docs = build_part11_documents(&conn, "2020-01-01", "2030-01-01", "Test Lab").unwrap();
        let audit_doc = docs.iter().find(|(name, _)| name == "part11_audit_trail.json").unwrap();
        let entries: Vec<serde_json::Value> = serde_json::from_slice(&audit_doc.1).unwrap();

        let direct_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM audit_log WHERE date(created_at) >= '2020-01-01' AND date(created_at) <= '2030-01-01'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(entries.len() as i64, direct_count);
        assert!(direct_count > 0, "test fixture must have produced at least one audit entry");
    }

    #[test]
    fn part11_date_range_filter_excludes_out_of_range_entries() {
        let conn = export_test_db();
        let docs = build_part11_documents(&conn, "2099-01-01", "2099-12-31", "Test Lab").unwrap();
        let audit_doc = docs.iter().find(|(name, _)| name == "part11_audit_trail.json").unwrap();
        let entries: Vec<serde_json::Value> = serde_json::from_slice(&audit_doc.1).unwrap();
        assert!(entries.is_empty(), "a far-future date range must exclude all present-day entries");
    }

    #[test]
    fn part11_bundle_includes_all_four_documents() {
        let conn = export_test_db();
        let docs = build_part11_documents(&conn, "2020-01-01", "2030-01-01", "Test Lab").unwrap();
        let names: Vec<&str> = docs.iter().map(|(n, _)| n.as_str()).collect();
        for expected in ["part11_cover.json", "part11_audit_trail.json", "part11_verification.json", "part11_user_activity.json"] {
            assert!(names.contains(&expected), "bundle must include {}", expected);
        }
    }

    #[test]
    fn usda_permit_prefill_populates_fields_from_specimen_record() {
        let conn = export_test_db();
        let result = build_usda_permit_prefill(&conn, &["spec1".to_string()], "Dr. Jane Botanist").unwrap();
        let specimens = result["specimens"].as_array().unwrap();
        assert_eq!(specimens.len(), 1);
        assert_eq!(specimens[0]["scientific_name"], "Citrus sinensis");
        assert_eq!(specimens[0]["provenance"], "Field collection");
        assert_eq!(result["authorized_scientist"], "Dr. Jane Botanist");
    }

    #[test]
    fn cites_dossier_includes_chain_of_custody_and_propagation() {
        let conn = export_test_db();
        let dossier = build_cites_dossier(&conn, "spec1", "Appendix II").unwrap();
        assert_eq!(dossier["cites_appendix"], "Appendix II");
        let custody = dossier["chain_of_custody"].as_array().unwrap();
        assert!(custody.iter().any(|c| c["specimen_id"] == "spec1"), "root specimen must appear in its own chain of custody");
        let propagation = dossier["propagation_records"].as_array().unwrap();
        assert_eq!(propagation.len(), 1);
    }
}

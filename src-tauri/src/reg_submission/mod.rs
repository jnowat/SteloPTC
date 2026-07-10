// WP-68: Regulatory submission pipeline (advanced) — builds on WP-60.
//
// A pipeline that *monitors compliance state* and, when a submission's
// preconditions are met, generates and signs the corresponding WP-60 regulatory
// bundle so it is ready to hand to the authority. Each submission moves through a
// lifecycle (`draft`/`ready`/`blocked` → `generated` → `submitted` →
// `acknowledged`) recorded in the `regulatory_submissions` table.
//
// This module is the pure, connection-only core: readiness evaluation and the
// submission-record lifecycle. It writes only to `regulatory_submissions` and
// reads existing data — the actual bundle assembly + signing + file write lives
// in `commands::reg_submission` (which reuses `compliance_export::bundle`,
// `::signing`, `::zip_writer`), exactly the WP-60 machinery.
//
// Scope, disclosed honestly (matching the WP-60 "does not submit to APHIS
// directly" boundary): SteloPTC monitors readiness and produces a signed,
// ready-to-submit package, but it does NOT electronically submit to a government
// web portal. Automated portal submission needs authenticated portal
// credentials, per-agency form APIs, and legal authorization that vary by
// jurisdiction — out of scope, and out of a specimen tracker. The operator
// downloads the signed package, submits it through the official channel, and
// records the returned reference number here.

use rusqlite::{params, Connection};
use serde::Serialize;

use crate::compliance_export::bundle;
use crate::db::queries;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SubmissionKind {
    Part11,
    Usda,
    Cites,
}

impl SubmissionKind {
    pub fn from_code(s: &str) -> Result<Self, String> {
        match s {
            "part11" => Ok(SubmissionKind::Part11),
            "usda" => Ok(SubmissionKind::Usda),
            "cites" => Ok(SubmissionKind::Cites),
            other => Err(format!("Unknown submission kind '{}' (expected part11 | usda | cites)", other)),
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            SubmissionKind::Part11 => "part11",
            SubmissionKind::Usda => "usda",
            SubmissionKind::Cites => "cites",
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ReadinessCheck {
    pub key: String,
    pub label: String,
    pub passed: bool,
    pub detail: String,
}

#[derive(Debug, Serialize)]
pub struct Readiness {
    pub kind: String,
    pub ready: bool,
    pub blocking_count: i64,
    pub checks: Vec<ReadinessCheck>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Submission {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub scope: String,
    pub status: String,
    pub readiness: Option<String>,
    pub package_path: Option<String>,
    pub package_signature: Option<String>,
    pub submission_reference: Option<String>,
    pub auto_generate: bool,
    pub created_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub submitted_at: Option<String>,
}

fn now_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

fn check(key: &str, label: &str, passed: bool, detail: impl Into<String>) -> ReadinessCheck {
    ReadinessCheck { key: key.to_string(), label: label.to_string(), passed, detail: detail.into() }
}

fn scope_str<'a>(scope: &'a serde_json::Value, key: &str) -> &'a str {
    scope.get(key).and_then(|v| v.as_str()).unwrap_or("")
}

/// Evaluate whether a submission of `kind` over `scope` meets its preconditions.
/// Pure and read-only — used both for a pre-flight preview and by the pipeline.
pub fn evaluate_readiness(conn: &Connection, kind: SubmissionKind, scope: &serde_json::Value) -> Result<Readiness, String> {
    let checks = match kind {
        SubmissionKind::Part11 => part11_checks(conn, scope)?,
        SubmissionKind::Usda => usda_checks(conn, scope)?,
        SubmissionKind::Cites => cites_checks(conn, scope)?,
    };
    let blocking = checks.iter().filter(|c| !c.passed).count() as i64;
    Ok(Readiness {
        kind: kind.as_str().to_string(),
        ready: blocking == 0,
        blocking_count: blocking,
        checks,
    })
}

fn part11_checks(conn: &Connection, scope: &serde_json::Value) -> Result<Vec<ReadinessCheck>, String> {
    let from = scope_str(scope, "from_date");
    let to = scope_str(scope, "to_date");
    let mut checks = Vec::new();

    let dates_ok = !from.is_empty() && !to.is_empty() && from <= to;
    checks.push(check(
        "date_range",
        "A valid from/to date range is set",
        dates_ok,
        if dates_ok { format!("{} → {}", from, to) } else { "Set a from_date ≤ to_date".to_string() },
    ));

    if dates_ok {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM audit_log WHERE date(created_at) >= ?1 AND date(created_at) <= ?2",
                params![from, to],
                |r| r.get(0),
            )
            .unwrap_or(0);
        checks.push(check("has_entries", "The date range contains audit entries", count > 0, format!("{} audit entries in range", count)));

        let verification = bundle::verify_audit_range(conn, from, to)?;
        checks.push(check(
            "chain_verified",
            "The audit hash chain verifies over the range",
            verification.verified,
            if verification.verified {
                format!("{} entries verified", verification.total_entries_checked)
            } else {
                format!("Chain broke at {:?}", verification.first_break)
            },
        ));
    }

    let user_count: i64 = conn.query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0)).unwrap_or(0);
    checks.push(check("has_users", "At least one user account exists for the activity report", user_count > 0, format!("{} users", user_count)));

    Ok(checks)
}

fn usda_checks(conn: &Connection, scope: &serde_json::Value) -> Result<Vec<ReadinessCheck>, String> {
    let ids: Vec<String> = scope
        .get("specimen_ids")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();
    let mut checks = Vec::new();

    let profile = queries::read_setting(conn, "lab_profile", "plant_tissue_culture");
    checks.push(check(
        "profile_ptc",
        "Lab profile is plant tissue culture (USDA APHIS PPQ 526 scope)",
        profile == "plant_tissue_culture",
        format!("Active profile: {}", profile),
    ));

    checks.push(check("has_specimens", "At least one specimen is in scope", !ids.is_empty(), format!("{} specimen(s) selected", ids.len())));

    if !ids.is_empty() {
        let mut missing = 0i64;
        let mut no_name = 0i64;
        let mut expired = 0i64;
        for id in &ids {
            let row: Option<(Option<String>, Option<String>, Option<String>)> = conn
                .query_row(
                    "SELECT s.genus, s.species_name, sp.permit_expiry \
                     FROM specimens sp JOIN species s ON sp.species_id = s.id WHERE sp.id = ?1",
                    params![id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .ok();
            match row {
                None => missing += 1,
                Some((genus, species, permit_expiry)) => {
                    if genus.as_deref().unwrap_or("").is_empty() || species.as_deref().unwrap_or("").is_empty() {
                        no_name += 1;
                    }
                    if let Some(exp) = permit_expiry {
                        // Expired permit is a critical compliance issue that must
                        // block a permit-related submission.
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        if !exp.is_empty() && exp.as_str() < today.as_str() {
                            expired += 1;
                        }
                    }
                }
            }
        }
        checks.push(check("specimens_exist", "Every selected specimen exists", missing == 0, format!("{} missing", missing)));
        checks.push(check("scientific_names", "Every specimen has a scientific name (genus + species)", no_name == 0, format!("{} without a full name", no_name)));
        checks.push(check("no_expired_permits", "No selected specimen has an expired permit", expired == 0, format!("{} with an expired permit", expired)));
    }

    Ok(checks)
}

fn cites_checks(conn: &Connection, scope: &serde_json::Value) -> Result<Vec<ReadinessCheck>, String> {
    let root = scope_str(scope, "root_specimen_id");
    let appendix = scope_str(scope, "cites_appendix");
    let mut checks = Vec::new();

    let exists: bool = !root.is_empty()
        && conn
            .query_row("SELECT COUNT(*) FROM specimens WHERE id = ?1", params![root], |r| r.get::<_, i64>(0))
            .map(|c| c > 0)
            .unwrap_or(false);
    checks.push(check("root_specimen", "The root specimen exists", exists, if exists { root.to_string() } else { "Select a valid root specimen".to_string() }));

    checks.push(check(
        "appendix_set",
        "A CITES Appendix (I / II / III) is confirmed",
        !appendix.is_empty(),
        if appendix.is_empty() { "User must confirm the Appendix".to_string() } else { appendix.to_string() },
    ));

    let verification = bundle::verify_audit_range(conn, "0000-01-01", "9999-12-31")?;
    checks.push(check(
        "chain_verified",
        "The audit hash chain verifies (chain-of-custody integrity)",
        verification.verified,
        if verification.verified { format!("{} entries verified", verification.total_entries_checked) } else { "Chain integrity broken".to_string() },
    ));

    Ok(checks)
}

const SUB_COLS: &str = "id, kind, title, scope, status, readiness, package_path, package_signature, \
                        submission_reference, auto_generate, created_by, created_at, updated_at, submitted_at";

fn map_submission(r: &rusqlite::Row) -> rusqlite::Result<Submission> {
    Ok(Submission {
        id: r.get(0)?,
        kind: r.get(1)?,
        title: r.get(2)?,
        scope: r.get(3)?,
        status: r.get(4)?,
        readiness: r.get(5)?,
        package_path: r.get(6)?,
        package_signature: r.get(7)?,
        submission_reference: r.get(8)?,
        auto_generate: r.get::<_, i64>(9)? != 0,
        created_by: r.get(10)?,
        created_at: r.get(11)?,
        updated_at: r.get(12)?,
        submitted_at: r.get(13)?,
    })
}

pub fn get_submission(conn: &Connection, id: &str) -> Result<Submission, String> {
    conn.query_row(
        &format!("SELECT {} FROM regulatory_submissions WHERE id = ?1", SUB_COLS),
        params![id],
        map_submission,
    )
    .map_err(|_| format!("Submission '{}' not found", id))
}

pub fn list_submissions(conn: &Connection) -> Result<Vec<Submission>, String> {
    let mut stmt = conn
        .prepare(&format!("SELECT {} FROM regulatory_submissions ORDER BY created_at DESC", SUB_COLS))
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], map_submission)
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

/// Create a submission, evaluating readiness immediately so its initial status is
/// `ready` or `blocked` (never a stale `draft`).
pub fn create_submission(
    conn: &Connection,
    kind_str: &str,
    title: &str,
    scope: &serde_json::Value,
    auto_generate: bool,
    user_id: &str,
) -> Result<Submission, String> {
    let kind = SubmissionKind::from_code(kind_str)?;
    let readiness = evaluate_readiness(conn, kind, scope)?;
    let status = if readiness.ready { "ready" } else { "blocked" };
    let readiness_json = serde_json::to_string(&readiness).map_err(|e| e.to_string())?;
    let scope_json = serde_json::to_string(scope).map_err(|e| e.to_string())?;

    let id = uuid::Uuid::new_v4().to_string();
    let now = now_iso();
    conn.execute(
        "INSERT INTO regulatory_submissions \
         (id, kind, title, scope, status, readiness, auto_generate, created_by, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
        params![id, kind.as_str(), title, scope_json, status, readiness_json, auto_generate as i64, user_id, now],
    )
    .map_err(|e| e.to_string())?;
    get_submission(conn, &id)
}

/// Re-evaluate a submission's readiness and refresh its status. Only submissions
/// still in the `ready`/`blocked` states are re-evaluated; once a package is
/// `generated`/`submitted`/`acknowledged` the status is left alone so the record
/// of what was produced is never silently unwound.
pub fn reevaluate_submission(conn: &Connection, id: &str) -> Result<Submission, String> {
    let sub = get_submission(conn, id)?;
    if sub.status != "ready" && sub.status != "blocked" && sub.status != "draft" {
        return Ok(sub);
    }
    let kind = SubmissionKind::from_code(&sub.kind)?;
    let scope: serde_json::Value = serde_json::from_str(&sub.scope).map_err(|e| e.to_string())?;
    let readiness = evaluate_readiness(conn, kind, &scope)?;
    let status = if readiness.ready { "ready" } else { "blocked" };
    let readiness_json = serde_json::to_string(&readiness).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE regulatory_submissions SET status = ?1, readiness = ?2, updated_at = ?3 WHERE id = ?4",
        params![status, readiness_json, now_iso(), id],
    )
    .map_err(|e| e.to_string())?;
    get_submission(conn, id)
}

/// Record that the signed package was produced (called by the command layer after
/// it assembles + signs + writes the bundle). Advances `ready` → `generated`.
pub fn attach_package(conn: &Connection, id: &str, package_path: &str, signature: &str) -> Result<Submission, String> {
    let sub = get_submission(conn, id)?;
    if sub.status != "ready" {
        return Err(format!("Submission must be 'ready' to generate a package (currently '{}')", sub.status));
    }
    conn.execute(
        "UPDATE regulatory_submissions SET status = 'generated', package_path = ?1, package_signature = ?2, updated_at = ?3 WHERE id = ?4",
        params![package_path, signature, now_iso(), id],
    )
    .map_err(|e| e.to_string())?;
    get_submission(conn, id)
}

/// Record that the operator submitted the generated package through the official
/// channel and got back a reference. Advances `generated` → `submitted`.
pub fn mark_submitted(conn: &Connection, id: &str, reference: &str) -> Result<Submission, String> {
    let sub = get_submission(conn, id)?;
    if sub.status != "generated" {
        return Err(format!("Only a 'generated' submission can be marked submitted (currently '{}')", sub.status));
    }
    let reference = reference.trim();
    if reference.is_empty() {
        return Err("A submission reference (e.g. the portal confirmation number) is required".to_string());
    }
    let now = now_iso();
    conn.execute(
        "UPDATE regulatory_submissions SET status = 'submitted', submission_reference = ?1, submitted_at = ?2, updated_at = ?2 WHERE id = ?3",
        params![reference, now, id],
    )
    .map_err(|e| e.to_string())?;
    get_submission(conn, id)
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
             VALUES ('user1', 'u1', 'x', 'User One', 'admin')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO species (id, genus, species_name, species_code) VALUES ('sp1', 'Citrus', 'sinensis', 'CIT-SIN')",
            [],
        )
        .unwrap();
        conn
    }

    fn seed_specimen(conn: &Connection, id: &str, permit_expiry: Option<&str>) {
        conn.execute(
            "INSERT INTO specimens (id, accession_number, species_id, initiation_date, permit_expiry) \
             VALUES (?1, ?1, 'sp1', '2026-01-01', ?2)",
            params![id, permit_expiry],
        )
        .unwrap();
    }

    #[test]
    fn part11_ready_when_range_has_verified_entries() {
        let conn = test_db();
        queries::log_audit(&conn, Some("user1"), "create", "specimen", Some("x"), None, Some("x"), Some("d")).unwrap();
        let scope = serde_json::json!({ "from_date": "2000-01-01", "to_date": "2100-01-01" });
        let r = evaluate_readiness(&conn, SubmissionKind::Part11, &scope).unwrap();
        assert!(r.ready, "{:?}", r.checks);
    }

    #[test]
    fn part11_blocked_without_dates() {
        let conn = test_db();
        let scope = serde_json::json!({});
        let r = evaluate_readiness(&conn, SubmissionKind::Part11, &scope).unwrap();
        assert!(!r.ready);
        assert!(r.blocking_count >= 1);
    }

    #[test]
    fn usda_ready_for_named_specimen_without_expired_permit() {
        let conn = test_db();
        seed_specimen(&conn, "spec1", None);
        let scope = serde_json::json!({ "specimen_ids": ["spec1"] });
        let r = evaluate_readiness(&conn, SubmissionKind::Usda, &scope).unwrap();
        assert!(r.ready, "{:?}", r.checks);
    }

    #[test]
    fn usda_blocked_by_expired_permit() {
        let conn = test_db();
        seed_specimen(&conn, "spec1", Some("2000-01-01"));
        let scope = serde_json::json!({ "specimen_ids": ["spec1"] });
        let r = evaluate_readiness(&conn, SubmissionKind::Usda, &scope).unwrap();
        assert!(!r.ready);
        assert!(r.checks.iter().any(|c| c.key == "no_expired_permits" && !c.passed));
    }

    #[test]
    fn cites_requires_specimen_and_appendix() {
        let conn = test_db();
        seed_specimen(&conn, "spec1", None);
        let ok = serde_json::json!({ "root_specimen_id": "spec1", "cites_appendix": "Appendix II" });
        assert!(evaluate_readiness(&conn, SubmissionKind::Cites, &ok).unwrap().ready);
        let no_appendix = serde_json::json!({ "root_specimen_id": "spec1", "cites_appendix": "" });
        assert!(!evaluate_readiness(&conn, SubmissionKind::Cites, &no_appendix).unwrap().ready);
    }

    #[test]
    fn create_sets_status_from_readiness() {
        let conn = test_db();
        seed_specimen(&conn, "spec1", Some("2000-01-01")); // expired → blocked
        let scope = serde_json::json!({ "specimen_ids": ["spec1"] });
        let sub = create_submission(&conn, "usda", "Test USDA", &scope, false, "user1").unwrap();
        assert_eq!(sub.status, "blocked");
        assert!(sub.readiness.is_some());
    }

    #[test]
    fn reevaluate_moves_blocked_to_ready_when_fixed() {
        let conn = test_db();
        seed_specimen(&conn, "spec1", Some("2000-01-01"));
        let scope = serde_json::json!({ "specimen_ids": ["spec1"] });
        let sub = create_submission(&conn, "usda", "Test", &scope, false, "user1").unwrap();
        assert_eq!(sub.status, "blocked");
        // Fix the blocking issue (clear the expired permit).
        conn.execute("UPDATE specimens SET permit_expiry = NULL WHERE id = 'spec1'", []).unwrap();
        let re = reevaluate_submission(&conn, &sub.id).unwrap();
        assert_eq!(re.status, "ready");
    }

    #[test]
    fn lifecycle_generate_then_submit() {
        let conn = test_db();
        seed_specimen(&conn, "spec1", None);
        let scope = serde_json::json!({ "specimen_ids": ["spec1"] });
        let sub = create_submission(&conn, "usda", "Test", &scope, false, "user1").unwrap();
        assert_eq!(sub.status, "ready");
        let gen = attach_package(&conn, &sub.id, "/tmp/pkg.zip", "sig-b64").unwrap();
        assert_eq!(gen.status, "generated");
        assert_eq!(gen.package_path.as_deref(), Some("/tmp/pkg.zip"));
        let submitted = mark_submitted(&conn, &sub.id, "APHIS-2026-0001").unwrap();
        assert_eq!(submitted.status, "submitted");
        assert_eq!(submitted.submission_reference.as_deref(), Some("APHIS-2026-0001"));
        assert!(submitted.submitted_at.is_some());
    }

    #[test]
    fn cannot_generate_a_blocked_submission() {
        let conn = test_db();
        seed_specimen(&conn, "spec1", Some("2000-01-01"));
        let scope = serde_json::json!({ "specimen_ids": ["spec1"] });
        let sub = create_submission(&conn, "usda", "Test", &scope, false, "user1").unwrap();
        assert!(attach_package(&conn, &sub.id, "/tmp/x.zip", "s").is_err());
    }

    #[test]
    fn mark_submitted_requires_reference_and_generated_state() {
        let conn = test_db();
        seed_specimen(&conn, "spec1", None);
        let scope = serde_json::json!({ "specimen_ids": ["spec1"] });
        let sub = create_submission(&conn, "usda", "Test", &scope, false, "user1").unwrap();
        // Not generated yet.
        assert!(mark_submitted(&conn, &sub.id, "REF").is_err());
        attach_package(&conn, &sub.id, "/tmp/x.zip", "s").unwrap();
        // Empty reference rejected.
        assert!(mark_submitted(&conn, &sub.id, "  ").is_err());
    }
}

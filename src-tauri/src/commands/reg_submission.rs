// WP-68: Regulatory submission pipeline (advanced) — command layer.
//
// Thin session/role gating over `crate::reg_submission`, plus the one piece that
// cannot be pure: generating the signed WP-60 bundle for a `ready` submission
// (filesystem + signing key). Everything is supervisor/admin gated, matching the
// WP-60 export commands whose machinery this reuses.
use serde::Serialize;
use tauri::State;

use crate::auth as auth_service;
use crate::commands::compliance_export as ce;
use crate::compliance_export::{bundle, signing};
use crate::reg_submission::{self, SubmissionKind};
use crate::AppState;

const MANAGE_ONLY: &str = "Only supervisors and admins can manage regulatory submissions.";

fn submissions_dir() -> Result<std::path::PathBuf, String> {
    let dir = ce::exports_dir()?.join("submissions");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// Build the plaintext documents that make up a submission of a given kind,
/// reusing the WP-60 bundle assembly.
fn build_documents(
    conn: &rusqlite::Connection,
    kind: SubmissionKind,
    scope: &serde_json::Value,
) -> Result<Vec<(String, Vec<u8>)>, String> {
    match kind {
        SubmissionKind::Part11 => {
            let from = scope.get("from_date").and_then(|v| v.as_str()).unwrap_or("");
            let to = scope.get("to_date").and_then(|v| v.as_str()).unwrap_or("");
            let lab_name = scope.get("lab_name").and_then(|v| v.as_str()).unwrap_or("Laboratory");
            bundle::build_part11_documents(conn, from, to, lab_name)
        }
        SubmissionKind::Usda => {
            let ids: Vec<String> = scope
                .get("specimen_ids")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();
            let scientist = scope.get("authorized_scientist").and_then(|v| v.as_str()).unwrap_or("");
            let prefill = bundle::build_usda_permit_prefill(conn, &ids, scientist)?;
            Ok(vec![("usda_ppq526_prefill.json".to_string(), serde_json::to_vec_pretty(&prefill).map_err(|e| e.to_string())?)])
        }
        SubmissionKind::Cites => {
            let root = scope.get("root_specimen_id").and_then(|v| v.as_str()).unwrap_or("");
            let appendix = scope.get("cites_appendix").and_then(|v| v.as_str()).unwrap_or("");
            let dossier = bundle::build_cites_dossier(conn, root, appendix)?;
            Ok(vec![("cites_dossier.json".to_string(), serde_json::to_vec_pretty(&dossier).map_err(|e| e.to_string())?)])
        }
    }
}

/// Generate + sign the package for a `ready` submission and advance it to
/// `generated`. Shared by the explicit command and the monitor.
fn generate_package(conn: &rusqlite::Connection, submission_id: &str) -> Result<reg_submission::Submission, String> {
    let sub = reg_submission::get_submission(conn, submission_id)?;
    if sub.status != "ready" {
        return Err(format!("Submission must be 'ready' to generate (currently '{}')", sub.status));
    }
    let kind = SubmissionKind::from_code(&sub.kind)?;
    let scope: serde_json::Value = serde_json::from_str(&sub.scope).map_err(|e| e.to_string())?;

    let documents = build_documents(conn, kind, &scope)?;
    let (public_key, private_key) = ce::load_or_create_signing_key(conn)?;
    let zip_bytes = ce::sign_and_zip(&private_key, &public_key, documents)?;
    // A top-level detached signature over the exact delivered artifact.
    let package_signature = signing::sign(&private_key, &zip_bytes)?;

    let file_name = format!(
        "submission_{}_{}_{}.zip",
        sub.kind,
        &submission_id[..8.min(submission_id.len())],
        chrono::Local::now().format("%Y%m%d_%H%M%S")
    );
    let file_path = submissions_dir()?.join(&file_name);
    std::fs::write(&file_path, &zip_bytes).map_err(|e| e.to_string())?;

    reg_submission::attach_package(conn, submission_id, &file_path.to_string_lossy(), &package_signature)
}

#[tauri::command]
pub fn evaluate_submission_readiness(
    state: State<AppState>,
    token: String,
    kind: String,
    scope: serde_json::Value,
) -> Result<reg_submission::Readiness, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err(MANAGE_ONLY.to_string());
    }
    let k = SubmissionKind::from_code(&kind)?;
    reg_submission::evaluate_readiness(&db.conn, k, &scope)
}

#[tauri::command]
pub fn create_submission(
    state: State<AppState>,
    token: String,
    kind: String,
    title: String,
    scope: serde_json::Value,
    auto_generate: Option<bool>,
) -> Result<reg_submission::Submission, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err(MANAGE_ONLY.to_string());
    }
    let sub = reg_submission::create_submission(&db.conn, &kind, &title, &scope, auto_generate.unwrap_or(false), &user.id)?;
    crate::db::queries::log_audit(
        &db.conn, Some(&user.id), "create", "regulatory_submission", Some(&sub.id),
        None, Some(&sub.status), Some(&format!("Created {} submission '{}'", sub.kind, sub.title)),
    )
    .ok();
    Ok(sub)
}

#[tauri::command]
pub fn reevaluate_submission(
    state: State<AppState>,
    token: String,
    submission_id: String,
) -> Result<reg_submission::Submission, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err(MANAGE_ONLY.to_string());
    }
    reg_submission::reevaluate_submission(&db.conn, &submission_id)
}

#[tauri::command]
pub fn generate_submission_package(
    state: State<AppState>,
    token: String,
    submission_id: String,
) -> Result<reg_submission::Submission, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err(MANAGE_ONLY.to_string());
    }
    // Re-check readiness right before generating so a stale 'ready' can't slip a
    // no-longer-compliant package through.
    let refreshed = reg_submission::reevaluate_submission(&db.conn, &submission_id)?;
    if refreshed.status != "ready" {
        return Err("Submission is not currently ready — resolve the blocking checks first.".to_string());
    }
    let sub = generate_package(&db.conn, &submission_id)?;
    crate::db::queries::log_audit(
        &db.conn, Some(&user.id), "generate", "regulatory_submission", Some(&sub.id),
        None, sub.package_path.as_deref(), Some("Generated & signed regulatory submission package"),
    )
    .ok();
    Ok(sub)
}

#[tauri::command]
pub fn mark_submission_submitted(
    state: State<AppState>,
    token: String,
    submission_id: String,
    reference: String,
) -> Result<reg_submission::Submission, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err(MANAGE_ONLY.to_string());
    }
    let sub = reg_submission::mark_submitted(&db.conn, &submission_id, &reference)?;
    crate::db::queries::log_audit(
        &db.conn, Some(&user.id), "submit", "regulatory_submission", Some(&sub.id),
        None, sub.submission_reference.as_deref(), Some("Recorded external submission reference"),
    )
    .ok();
    Ok(sub)
}

#[tauri::command]
pub fn list_submissions(state: State<AppState>, token: String) -> Result<Vec<reg_submission::Submission>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err(MANAGE_ONLY.to_string());
    }
    reg_submission::list_submissions(&db.conn)
}

#[derive(Debug, Serialize)]
pub struct MonitorResult {
    pub evaluated: i64,
    pub became_ready: i64,
    pub auto_generated: i64,
    pub still_blocked: i64,
}

/// Re-evaluate every non-terminal submission against current compliance state,
/// and auto-generate the package for any that is now `ready` and flagged
/// `auto_generate`. Callable on demand and from the background scheduler.
pub fn monitor(conn: &rusqlite::Connection) -> Result<MonitorResult, String> {
    let mut result = MonitorResult { evaluated: 0, became_ready: 0, auto_generated: 0, still_blocked: 0 };
    let submissions = reg_submission::list_submissions(conn)?;
    for sub in submissions {
        if sub.status != "ready" && sub.status != "blocked" && sub.status != "draft" {
            continue;
        }
        result.evaluated += 1;
        let refreshed = reg_submission::reevaluate_submission(conn, &sub.id)?;
        if refreshed.status == "ready" {
            result.became_ready += 1;
            if refreshed.auto_generate && generate_package(conn, &refreshed.id).is_ok() {
                result.auto_generated += 1;
            }
        } else if refreshed.status == "blocked" {
            result.still_blocked += 1;
        }
    }
    Ok(result)
}

#[tauri::command]
pub fn run_submission_monitor(state: State<AppState>, token: String) -> Result<MonitorResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err(MANAGE_ONLY.to_string());
    }
    monitor(&db.conn)
}

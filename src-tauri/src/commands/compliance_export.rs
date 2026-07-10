// WP-60: Regulatory compliance export modules — Tauri command layer.
// Every export command is supervisor/admin gated, read-only against the
// database, and writes only the generated bundle file (plus, once, the
// lab's Ed25519 signing keypair on first use).
use rusqlite::params;
use tauri::State;

use crate::auth as auth_service;
use crate::compliance_export::{bundle, signing, zip_writer};
use crate::AppState;

pub(crate) fn exports_dir() -> Result<std::path::PathBuf, String> {
    let base = crate::db::Database::db_path();
    let parent = base.parent().ok_or_else(|| "Could not determine exports directory".to_string())?;
    let dir = parent.join("compliance_exports");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

pub(crate) fn load_or_create_signing_key(conn: &rusqlite::Connection) -> Result<(String, String), String> {
    let existing: Option<(String, String)> = conn
        .query_row("SELECT public_key_b64, private_key_b64 FROM signing_keys WHERE id = 1", [], |r| Ok((r.get(0)?, r.get(1)?)))
        .ok();
    if let Some(pair) = existing {
        return Ok(pair);
    }
    let keypair = signing::generate_keypair();
    conn.execute(
        "INSERT INTO signing_keys (id, public_key_b64, private_key_b64) VALUES (1, ?1, ?2)",
        params![keypair.public_key_b64, keypair.private_key_b64],
    ).map_err(|e| e.to_string())?;
    Ok((keypair.public_key_b64, keypair.private_key_b64))
}

#[tauri::command]
pub fn get_signing_public_key(state: State<AppState>, token: String) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err("Only supervisors and admins can view the signing key".to_string());
    }
    let (public_key, _) = load_or_create_signing_key(&db.conn)?;
    Ok(public_key)
}

pub(crate) fn sign_and_zip(private_key_b64: &str, public_key_b64: &str, documents: Vec<(String, Vec<u8>)>) -> Result<Vec<u8>, String> {
    let mut files = Vec::with_capacity(documents.len() * 2 + 1);
    for (name, contents) in documents {
        let signature = signing::sign(private_key_b64, &contents)?;
        files.push((format!("{}.sig", name), signature.into_bytes()));
        files.push((name, contents));
    }
    files.push(("signing_public_key.b64".to_string(), public_key_b64.as_bytes().to_vec()));
    zip_writer::build_zip(&files)
}

#[derive(serde::Serialize)]
pub struct ComplianceExportResult {
    pub ok: bool,
    pub file_path: String,
    pub size_bytes: i64,
}

#[tauri::command]
pub fn export_fda_part11_bundle(
    state: State<AppState>,
    token: String,
    from_date: String,
    to_date: String,
    lab_name: String,
) -> Result<ComplianceExportResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err("Only supervisors and admins can generate a Part 11 export".to_string());
    }

    let documents = bundle::build_part11_documents(&db.conn, &from_date, &to_date, &lab_name)?;
    let (public_key, private_key) = load_or_create_signing_key(&db.conn)?;
    let zip_bytes = sign_and_zip(&private_key, &public_key, documents)?;

    let file_name = format!("fda_part11_{}_{}_{}.zip", from_date, to_date, chrono::Local::now().format("%Y%m%d_%H%M%S"));
    let file_path = exports_dir()?.join(&file_name);
    std::fs::write(&file_path, &zip_bytes).map_err(|e| e.to_string())?;

    crate::db::queries::log_audit(
        &db.conn, Some(&user.id), "export", "compliance_bundle", None,
        None, Some(&file_name), Some("FDA 21 CFR Part 11 export generated"),
    ).ok();

    Ok(ComplianceExportResult { ok: true, file_path: file_path.to_string_lossy().to_string(), size_bytes: zip_bytes.len() as i64 })
}

#[tauri::command]
pub fn export_usda_permit(
    state: State<AppState>,
    token: String,
    specimen_ids: Vec<String>,
    authorized_scientist: String,
) -> Result<ComplianceExportResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err("Only supervisors and admins can generate a USDA APHIS export".to_string());
    }

    let prefill = bundle::build_usda_permit_prefill(&db.conn, &specimen_ids, &authorized_scientist)?;
    let json_bytes = serde_json::to_vec_pretty(&prefill).map_err(|e| e.to_string())?;

    let file_name = format!("usda_ppq526_{}.json", chrono::Local::now().format("%Y%m%d_%H%M%S"));
    let file_path = exports_dir()?.join(&file_name);
    std::fs::write(&file_path, &json_bytes).map_err(|e| e.to_string())?;

    crate::db::queries::log_audit(
        &db.conn, Some(&user.id), "export", "compliance_bundle", None,
        None, Some(&file_name), Some("USDA APHIS PPQ Form 526 pre-fill generated"),
    ).ok();

    Ok(ComplianceExportResult { ok: true, file_path: file_path.to_string_lossy().to_string(), size_bytes: json_bytes.len() as i64 })
}

#[tauri::command]
pub fn export_cites_dossier(
    state: State<AppState>,
    token: String,
    root_specimen_id: String,
    cites_appendix: String,
) -> Result<ComplianceExportResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err("Only supervisors and admins can generate a CITES dossier".to_string());
    }

    let dossier = bundle::build_cites_dossier(&db.conn, &root_specimen_id, &cites_appendix)?;
    let json_bytes = serde_json::to_vec_pretty(&dossier).map_err(|e| e.to_string())?;
    let zip_bytes = zip_writer::build_zip(&[("cites_dossier.json".to_string(), json_bytes)])?;

    let file_name = format!("cites_dossier_{}_{}.zip", root_specimen_id, chrono::Local::now().format("%Y%m%d_%H%M%S"));
    let file_path = exports_dir()?.join(&file_name);
    std::fs::write(&file_path, &zip_bytes).map_err(|e| e.to_string())?;

    crate::db::queries::log_audit(
        &db.conn, Some(&user.id), "export", "compliance_bundle", Some(&root_specimen_id),
        None, Some(&file_name), Some("CITES Species Provenance Dossier generated"),
    ).ok();

    Ok(ComplianceExportResult { ok: true, file_path: file_path.to_string_lossy().to_string(), size_bytes: zip_bytes.len() as i64 })
}

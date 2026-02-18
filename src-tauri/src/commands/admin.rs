use crate::auth as auth_service;
use crate::db::queries;
use crate::AppState;
use tauri::State;

/// Wipes all operational data from the database while preserving
/// user accounts, species definitions, and system tags.
/// Admin-only. Requires passing the confirmation phrase "RESET DATABASE".
#[tauri::command]
pub fn reset_database(
    state: State<AppState>,
    token: String,
    confirmation: String,
) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;

    if !user.role.is_admin() {
        return Err("Only admins can reset the database".to_string());
    }

    if confirmation.trim() != "RESET DATABASE" {
        return Err("Confirmation phrase did not match. Type exactly: RESET DATABASE".to_string());
    }

    // Delete operational data in dependency order
    db.conn.execute("DELETE FROM media_hormones", []).map_err(|e| e.to_string())?;
    db.conn.execute("DELETE FROM subcultures", []).map_err(|e| e.to_string())?;
    db.conn.execute("DELETE FROM specimen_tags", []).map_err(|e| e.to_string())?;
    db.conn.execute("DELETE FROM compliance_records", []).map_err(|e| e.to_string())?;
    db.conn.execute("DELETE FROM reminders", []).map_err(|e| e.to_string())?;
    db.conn.execute("DELETE FROM attachments", []).map_err(|e| e.to_string())?;
    db.conn.execute("DELETE FROM specimens", []).map_err(|e| e.to_string())?;
    db.conn.execute("DELETE FROM media_batches", []).map_err(|e| e.to_string())?;
    db.conn.execute("DELETE FROM inventory_items", []).map_err(|e| e.to_string())?;
    db.conn.execute("DELETE FROM audit_log", []).map_err(|e| e.to_string())?;

    // Log the reset itself (audit entry won't survive if audit_log was cleared,
    // but we log it here for completeness if any partial rollback occurs)
    queries::log_audit(
        &db.conn,
        Some(&user.id),
        "reset",
        "database",
        None,
        None,
        None,
        Some("Full database reset performed by admin"),
    ).ok();

    Ok("Database reset complete. All specimens, media, subcultures, inventory, compliance records, and reminders have been cleared. Users and species definitions were preserved.".to_string())
}

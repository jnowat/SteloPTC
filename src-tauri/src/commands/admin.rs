use crate::auth as auth_service;
use crate::db::queries;
use crate::AppState;
use rusqlite::params;
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
    db.conn.execute("DELETE FROM prepared_solutions", []).map_err(|e| e.to_string())?;
    db.conn.execute("DELETE FROM inventory_items", []).map_err(|e| e.to_string())?;
    db.conn.execute("DELETE FROM qr_scans", []).map_err(|e| e.to_string())?;
    db.conn.execute("DELETE FROM error_logs", []).map_err(|e| e.to_string())?;
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

    Ok("Database reset complete. All specimens, media, subcultures, inventory, compliance records, reminders, QR scan history, error logs, and prepared solutions have been cleared. Users and species definitions were preserved.".to_string())
}

/// Loads a coherent set of sample records (media batch, specimens, subcultures)
/// using the seeded species registry so first-time evaluators can explore immediately.
/// Only works on an empty lab — returns an error if any specimens already exist.
#[tauri::command]
pub fn load_demo_data(
    state: State<AppState>,
    token: String,
) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err("Only supervisors and admins can load demo data".to_string());
    }

    let existing: i64 = db.conn
        .query_row("SELECT COUNT(*) FROM specimens", [], |r| r.get(0))
        .unwrap_or(0);
    if existing > 0 {
        return Err(
            "Demo data can only be loaded into an empty lab (no specimens exist yet). \
             Use Admin → Reset Database first if you want to start over."
                .to_string(),
        );
    }

    // Resolve seeded species — gracefully skip any that are missing
    let asp_id: Option<String> = db.conn
        .query_row("SELECT id FROM species WHERE species_code = 'ASP-OFF'", [], |r| r.get(0))
        .ok();
    let nan_id: Option<String> = db.conn
        .query_row("SELECT id FROM species WHERE species_code = 'NAN-DOM'", [], |r| r.get(0))
        .ok();
    let cit_id: Option<String> = db.conn
        .query_row("SELECT id FROM species WHERE species_code = 'CIT-SIN'", [], |r| r.get(0))
        .ok();

    let mut demo_entries: Vec<(String, &str, &str, &str)> = Vec::new();
    if let Some(id) = asp_id { demo_entries.push((id, "ASP-OFF", "explant",  "Room 1 / Rack A / Shelf 1 / Tray A")); }
    if let Some(id) = nan_id { demo_entries.push((id, "NAN-DOM", "shoot",    "Room 1 / Rack A / Shelf 1 / Tray B")); }
    if let Some(id) = cit_id { demo_entries.push((id, "CIT-SIN", "shoot",    "Room 1 / Rack B / Shelf 1 / Tray A")); }

    if demo_entries.is_empty() {
        return Err(
            "No default species found in the registry. \
             Add species before loading demo data."
                .to_string(),
        );
    }

    let tx = db.conn
        .unchecked_transaction()
        .map_err(|e| format!("Failed to begin transaction: {}", e))?;

    // One demo media batch shared by all passages
    let mb_id = uuid::Uuid::new_v4().to_string();
    tx.execute(
        "INSERT INTO media_batches
         (id, batch_id, name, preparation_date, basal_salts, sucrose_g_per_l,
          agar_g_per_l, ph_before_autoclave, volume_prepared_ml, volume_remaining_ml,
          is_custom, needs_review, notes, created_by)
         VALUES (?1, 'MB-DEMO-001', 'Demo MS Medium', date('now', '-60 days'),
                 'MS (Murashige & Skoog)', 30.0, 6.5, 5.8, 2000.0, 600.0, 0, 0,
                 'Sample medium — loaded with demo data', ?2)",
        params![mb_id, user.id],
    ).map_err(|e| format!("Failed to create demo media batch: {}", e))?;

    let passage_dates = ["2026-01-15", "2026-02-15", "2026-03-15"];

    for (species_id, code, stage, location) in &demo_entries {
        let sp_id  = uuid::Uuid::new_v4().to_string();
        let acc    = format!("DEMO-{}-001", code);
        let qr     = format!("STELO:{}", acc);

        tx.execute(
            "INSERT INTO specimens
             (id, accession_number, species_id, stage, initiation_date, location,
              health_status, qr_code_data, notes, created_by)
             VALUES (?1, ?2, ?3, ?4, '2026-01-10', ?5, 3, ?6,
                     'Demo specimen — remove via Admin → Reset Database', ?7)",
            params![sp_id, acc, species_id, stage, location, qr, user.id],
        ).map_err(|e| format!("Failed to create demo specimen {}: {}", code, e))?;

        for (i, &date) in passage_dates.iter().enumerate() {
            let sc_id  = uuid::Uuid::new_v4().to_string();
            let passage = (i + 1) as i32;
            tx.execute(
                "INSERT INTO subcultures
                 (id, specimen_id, passage_number, date, media_batch_id,
                  vessel_type, location_to, health_status, performed_by)
                 VALUES (?1, ?2, ?3, ?4, ?5, 'Magenta Box GA-7', ?6, 3, ?7)",
                params![sc_id, sp_id, passage, date, mb_id, location, user.id],
            ).map_err(|e| format!("Failed to create demo subculture: {}", e))?;
        }

        tx.execute(
            "UPDATE specimens SET subculture_count = 3 WHERE id = ?1",
            params![sp_id],
        ).map_err(|e| format!("Failed to update subculture count: {}", e))?;
    }

    tx.commit().map_err(|e| format!("Failed to commit demo data: {}", e))?;

    queries::log_audit(
        &db.conn,
        Some(&user.id),
        "create",
        "demo_data",
        None,
        None,
        None,
        Some("Demo data loaded by user"),
    )
    .ok();

    Ok(format!(
        "Demo data loaded: 1 media batch, {} specimens with 3 passages each. \
         Remove via Admin → Reset Database when ready.",
        demo_entries.len()
    ))
}

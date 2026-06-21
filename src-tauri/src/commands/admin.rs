use crate::auth as auth_service;
use crate::db::queries;
use crate::AppState;
use rusqlite::{params, Connection};
use tauri::State;

/// Returns the current lab profile (any authenticated user can read).
#[tauri::command]
pub fn get_lab_profile(
    state: State<AppState>,
    token: String,
) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;

    let profile: String = db.conn
        .query_row("SELECT lab_profile FROM app_config WHERE id = 1", [], |r| r.get(0))
        .unwrap_or_else(|_| "plant_tissue_culture".to_string());

    Ok(profile)
}

/// Updates the lab profile.  Admin-only.  Blocked once any specimens exist to
/// preserve data-integrity invariants that depend on the profile value.
#[tauri::command]
pub fn set_lab_profile(
    state: State<AppState>,
    token: String,
    profile: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;

    if !user.role.is_admin() {
        return Err("Only admins can change the lab profile".to_string());
    }

    let allowed = ["plant_tissue_culture", "cell_culture", "mycology"];
    if !allowed.contains(&profile.as_str()) {
        return Err(format!(
            "Invalid lab profile '{}'. Allowed values: plant_tissue_culture, cell_culture, mycology",
            profile
        ));
    }

    // Lock the profile once operational data exists.
    let specimen_count: i64 = db.conn
        .query_row("SELECT COUNT(*) FROM specimens", [], |r| r.get(0))
        .unwrap_or(0);

    if specimen_count > 0 {
        return Err(
            "The lab profile cannot be changed after specimens have been accessioned. \
             Reset the database first if you need to switch profiles."
                .to_string(),
        );
    }

    db.conn
        .execute(
            "UPDATE app_config SET lab_profile = ?1, updated_at = datetime('now') WHERE id = 1",
            params![profile],
        )
        .map_err(|e| format!("Failed to update lab profile: {}", e))?;

    queries::log_audit(
        &db.conn,
        Some(&user.id),
        "update",
        "app_config",
        None,
        None,
        None,
        Some(&format!("Lab profile set to '{}'", profile)),
    ).ok();

    Ok(())
}

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

/// Loads a coherent set of fully hash-chained sample records (media batch,
/// specimens, subcultures, and one cryptographic split) using the seeded species
/// registry so first-time evaluators can explore the audit chain immediately.
///
/// Every record is created through the standard `log_audit` / `log_audit_for_child`
/// functions — all audit rows carry valid `lineage_id`, `chain_seq`, `prev_hash`,
/// and `entry_hash` from the moment they are inserted. The first species is also
/// split into two child specimens (002A / 002B) to demonstrate the fork semantics:
/// both children share the same `prev_hash` (the parent's final `entry_hash`),
/// making the lineage fork cryptographically visible in the Audit Log.
///
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

    // Resolve seeded species — gracefully skip any that are missing.
    // Vec entries: (species_id, code, stage, location)
    let mut roots: Vec<(String, &str, &str, &str)> = Vec::new();
    for (code, stage, location) in [
        ("ASP-OFF", "explant", "Room 1 / Rack A / Shelf 1 / Tray A"),
        ("NAN-DOM", "shoot",   "Room 1 / Rack A / Shelf 1 / Tray B"),
        ("CIT-SIN", "shoot",   "Room 1 / Rack B / Shelf 1 / Tray A"),
    ] {
        if let Ok(id) = db.conn.query_row(
            "SELECT id FROM species WHERE species_code = ?1",
            params![code],
            |r| r.get::<_, String>(0),
        ) {
            roots.push((id, code, stage, location));
        }
    }

    if roots.is_empty() {
        return Err(
            "No default species found in the registry. \
             Add species before loading demo data."
                .to_string(),
        );
    }

    let tx = db.conn
        .unchecked_transaction()
        .map_err(|e| format!("Failed to begin transaction: {}", e))?;
    // Transaction derefs to Connection, so we can pass conn to log_audit functions.
    let conn: &Connection = &tx;

    // --- Media batch ---
    let mb_id = uuid::Uuid::new_v4().to_string();
    tx.execute(
        "INSERT INTO media_batches \
         (id, batch_id, name, preparation_date, basal_salts, sucrose_g_per_l, \
          agar_g_per_l, ph_before_autoclave, volume_prepared_ml, volume_remaining_ml, \
          is_custom, needs_review, notes, created_by) \
         VALUES (?1, 'MB-DEMO-001', 'Demo MS Medium', date('now', '-60 days'), \
                 'MS (Murashige & Skoog)', 30.0, 6.5, 5.8, 2000.0, 600.0, 0, 0, \
                 'Sample medium — loaded with demo data', ?2)",
        params![mb_id, user.id],
    ).map_err(|e| format!("Failed to create demo media batch: {}", e))?;
    queries::log_audit(
        conn, Some(&user.id), "create", "media_batch", Some(&mb_id),
        None, None, Some("Demo MS Medium created with demo data"),
    ).map_err(|e| format!("Failed to audit demo media batch: {}", e))?;

    let passage_dates = ["2026-01-15", "2026-02-15", "2026-03-15"];
    let mut total_specimens = 0usize;
    let mut total_subcultures = 0usize;

    // ID of the first root specimen — used below to demonstrate a split.
    let mut first_root_id: Option<String> = None;

    // --- Root specimens ---
    for (species_id, code, stage, location) in &roots {
        let sp_id = uuid::Uuid::new_v4().to_string();
        let acc   = format!("DEMO-{}-001", code);
        let qr    = format!("STELO:{}", acc);

        tx.execute(
            "INSERT INTO specimens \
             (id, accession_number, species_id, stage, initiation_date, location, \
              health_status, qr_code_data, notes, created_by) \
             VALUES (?1, ?2, ?3, ?4, '2026-01-10', ?5, 3, ?6, \
                     'Demo specimen — remove via Admin → Reset Database', ?7)",
            params![sp_id, acc, species_id, stage, location, qr, user.id],
        ).map_err(|e| format!("Failed to create demo specimen {}: {}", code, e))?;
        // Seed specimen's chain from the species lineage (falls back to ZERO_HASH
        // for the default seeded species which predate the hash chain)
        queries::log_audit_seeded_by_species(
            conn, Some(&user.id), "create", "specimen", Some(&sp_id),
            None, Some(acc.as_str()), Some("Demo specimen created"),
            species_id,
        ).map_err(|e| format!("Failed to audit demo specimen {}: {}", code, e))?;
        total_specimens += 1;

        for (i, &date) in passage_dates.iter().enumerate() {
            let sc_id   = uuid::Uuid::new_v4().to_string();
            let passage = (i + 1) as i32;
            tx.execute(
                "INSERT INTO subcultures \
                 (id, specimen_id, passage_number, date, media_batch_id, \
                  vessel_type, location_to, health_status, performed_by) \
                 VALUES (?1, ?2, ?3, ?4, ?5, 'Magenta Box GA-7', ?6, 3, ?7)",
                params![sc_id, sp_id, passage, date, mb_id, location, user.id],
            ).map_err(|e| format!("Failed to create demo subculture: {}", e))?;
            queries::log_audit(
                conn, Some(&user.id), "subcultured", "specimen", Some(&sp_id),
                None, None,
                Some(format!("Demo passage {} on {}", passage, date).as_str()),
            ).map_err(|e| format!("Failed to audit demo subculture: {}", e))?;
            total_subcultures += 1;
        }

        tx.execute(
            "UPDATE specimens SET subculture_count = 3 WHERE id = ?1",
            params![sp_id],
        ).map_err(|e| format!("Failed to update subculture count: {}", e))?;

        if first_root_id.is_none() {
            first_root_id = Some(sp_id);
        }
    }

    // --- Demonstrate a split: derive two children from the first root specimen ---
    //
    // Both children call log_audit_for_child with the same parent_lineage_id, so
    // they both receive chain_seq = 1 and prev_hash = parent's last entry_hash.
    // This is the intended fork semantics: the shared prev_hash proves both
    // lineages diverged from exactly the same parent state.
    let first_species_id = &roots[0].0;
    let first_code       = roots[0].1;
    if let Some(ref parent_id) = first_root_id {
        // Update parent notes to reflect the split
        tx.execute(
            "UPDATE specimens \
             SET notes = 'Demo specimen (split into 002A/002B) — remove via Admin → Reset Database' \
             WHERE id = ?1",
            params![parent_id],
        ).map_err(|e| format!("Failed to update parent specimen notes: {}", e))?;

        for (suffix, location) in [
            ("A", "Room 1 / Rack A / Shelf 2 / Tray A"),
            ("B", "Room 1 / Rack A / Shelf 2 / Tray B"),
        ] {
            let child_id  = uuid::Uuid::new_v4().to_string();
            let child_acc = format!("DEMO-{}-002{}", first_code, suffix);
            let child_qr  = format!("STELO:{}", child_acc);

            tx.execute(
                "INSERT INTO specimens \
                 (id, accession_number, species_id, stage, initiation_date, location, \
                  health_status, qr_code_data, parent_specimen_id, notes, created_by) \
                 VALUES (?1, ?2, ?3, 'shoot', '2026-03-20', ?4, 3, ?5, ?6, \
                         'Demo split specimen — remove via Admin → Reset Database', ?7)",
                params![child_id, child_acc, first_species_id, location, child_qr, parent_id, user.id],
            ).map_err(|e| format!("Failed to create demo split specimen {}: {}", child_acc, e))?;
            queries::log_audit_for_child(
                conn, Some(&user.id), "create", "specimen", Some(&child_id),
                None, Some(child_acc.as_str()),
                Some(format!("Demo split derived from parent {}", parent_id).as_str()),
                parent_id,
            ).map_err(|e| format!("Failed to audit demo split specimen: {}", e))?;
            total_specimens += 1;

            // One passage per split child
            let sc_id = uuid::Uuid::new_v4().to_string();
            tx.execute(
                "INSERT INTO subcultures \
                 (id, specimen_id, passage_number, date, media_batch_id, \
                  vessel_type, location_to, health_status, performed_by) \
                 VALUES (?1, ?2, 1, '2026-04-15', ?3, 'Magenta Box GA-7', ?4, 3, ?5)",
                params![sc_id, child_id, mb_id, location, user.id],
            ).map_err(|e| format!("Failed to create demo split subculture: {}", e))?;
            queries::log_audit(
                conn, Some(&user.id), "subcultured", "specimen", Some(&child_id),
                None, None, Some("Demo passage 1 on 2026-04-15"),
            ).map_err(|e| format!("Failed to audit demo split subculture: {}", e))?;
            total_subcultures += 1;
        }
    }

    tx.commit().map_err(|e| format!("Failed to commit demo data: {}", e))?;

    // Post-commit summary audit on the outer connection (entity-less, always chain_seq = 1)
    queries::log_audit(
        &db.conn,
        Some(&user.id),
        "create",
        "demo_data",
        None,
        None,
        None,
        Some("Demo data loaded by user"),
    ).ok();

    Ok(format!(
        "Demo data loaded: 1 media batch, {} specimens ({} root + 2 split children from {}), \
         {} subculture passages. All audit entries are fully hash-chained. \
         Remove via Admin → Reset Database when ready.",
        total_specimens,
        roots.len(),
        first_code,
        total_subcultures,
    ))
}

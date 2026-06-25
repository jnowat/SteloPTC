use crate::auth as auth_service;
use crate::db::queries;
use crate::models::specimen::PaginatedResponse;
use crate::models::subculture::*;
use crate::AppState;
use rusqlite::params;
use tauri::State;

/// Records a terminal "death" event for a specimen, archives it, and prevents
/// any further passages or splits.  Unlike `create_subculture` this does NOT
/// increment the specimen's `subculture_count` — the death event is terminal,
/// not a normal passage in the culture's lineage.
#[tauri::command]
pub fn record_specimen_death(
    state: State<AppState>,
    token: String,
    request: RecordSpecimenDeathRequest,
) -> Result<Subculture, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }

    let (current_count, is_archived): (i32, i32) = db.conn.query_row(
        "SELECT subculture_count, is_archived FROM specimens WHERE id = ?1",
        params![request.specimen_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ).map_err(|_| "Specimen not found".to_string())?;

    if is_archived != 0 {
        return Err("Specimen is already archived — cannot record a death event".to_string());
    }

    let id = uuid::Uuid::new_v4().to_string();
    // Store after last passage number for ordering, but subculture_count stays unchanged.
    let event_passage_number = current_count + 1;

    let tx = db.conn
        .unchecked_transaction()
        .map_err(|e| format!("Failed to begin transaction: {}", e))?;

    tx.execute(
        "INSERT INTO subcultures
             (id, specimen_id, passage_number, date, notes, observations,
              performed_by, employee_id, health_status, contamination_flag, event_type)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,'0',0,'death')",
        params![
            id,
            request.specimen_id,
            event_passage_number,
            request.date,
            request.notes,
            request.observations,
            user.id,
            request.employee_id,
        ],
    ).map_err(|e| format!("Failed to insert death event: {}", e))?;

    // Archive specimen; set health to 0 (Dead).  subculture_count is intentionally untouched.
    tx.execute(
        "UPDATE specimens
         SET is_archived  = 1,
             archived_at  = datetime('now'),
             health_status = '0',
             updated_at   = datetime('now')
         WHERE id = ?1",
        params![request.specimen_id],
    ).map_err(|e| format!("Failed to archive specimen: {}", e))?;

    queries::log_audit(
        &tx,
        Some(&user.id),
        "death",
        "specimen",
        Some(&request.specimen_id),
        None,
        None,
        Some("Specimen marked dead and archived — terminal event"),
    ).map_err(|e| format!("Failed to write death audit: {}", e))?;

    tx.commit().map_err(|e| format!("Failed to commit death transaction: {}", e))?;

    db.conn.query_row(
        "SELECT sc.*, u.display_name as performer_name, mb.name as media_batch_name
         FROM subcultures sc
         LEFT JOIN users u ON sc.performed_by = u.id
         LEFT JOIN media_batches mb ON sc.media_batch_id = mb.id
         WHERE sc.id = ?1",
        params![id],
        row_to_subculture,
    ).map_err(|e| format!("Failed to fetch death event: {}", e))
}

// ── helper: map a DB row to a Subculture ────────────────────────────────────
fn row_to_subculture(row: &rusqlite::Row) -> rusqlite::Result<Subculture> {
    let flag: i32 = row.get("contamination_flag").unwrap_or(0);
    Ok(Subculture {
        id: row.get("id")?,
        specimen_id: row.get("specimen_id")?,
        passage_number: row.get("passage_number")?,
        date: row.get("date")?,
        media_batch_id: row.get("media_batch_id")?,
        media_batch_name: row.get("media_batch_name")?,
        ph: row.get("ph")?,
        temperature_c: row.get("temperature_c")?,
        light_cycle: row.get("light_cycle")?,
        light_intensity_lux: row.get("light_intensity_lux")?,
        experimental_treatment: row.get("experimental_treatment")?,
        vessel_type: row.get("vessel_type")?,
        vessel_size: row.get("vessel_size")?,
        vessel_material: row.get("vessel_material")?,
        vessel_lid_type: row.get("vessel_lid_type")?,
        location_from: row.get("location_from")?,
        location_to: row.get("location_to")?,
        temp_before: row.get("temp_before")?,
        temp_after: row.get("temp_after")?,
        humidity_before: row.get("humidity_before")?,
        humidity_after: row.get("humidity_after")?,
        light_before: row.get("light_before")?,
        light_after: row.get("light_after")?,
        exposure_duration_hours: row.get("exposure_duration_hours")?,
        notes: row.get("notes")?,
        observations: row.get("observations")?,
        performed_by: row.get("performed_by")?,
        performer_name: row.get("performer_name")?,
        employee_id: row.get("employee_id")?,
        health_status: row.get("health_status")?,
        contamination_flag: flag != 0,
        contamination_notes: row.get("contamination_notes")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        event_type: row.get("event_type").unwrap_or_else(|_| "passage".to_string()),
        seed_cell_count: row.get("seed_cell_count").unwrap_or(None),
        harvest_cell_count: row.get("harvest_cell_count").unwrap_or(None),
        split_ratio: row.get("split_ratio").unwrap_or(None),
        pdl_gained: row.get("pdl_gained").unwrap_or(None),
        doubling_time_hours: row.get("doubling_time_hours").unwrap_or(None),
        colonization_pct: row.get("colonization_pct").unwrap_or(None),
        contaminant_type: row.get("contaminant_type").unwrap_or(None),
    })
}

#[tauri::command]
pub fn list_subcultures(
    state: State<AppState>,
    token: String,
    specimen_id: String,
    page: Option<u32>,
    per_page: Option<u32>,
) -> Result<PaginatedResponse<Subculture>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;

    let pg = queries::PaginationParams {
        page: page.unwrap_or(1),
        per_page: per_page.unwrap_or(50),
    };

    let total: i64 = db.conn.query_row(
        "SELECT COUNT(*) FROM subcultures WHERE specimen_id = ?1",
        params![specimen_id],
        |r| r.get(0),
    ).map_err(|e| e.to_string())?;

    let mut stmt = db.conn.prepare(
        "SELECT sc.*, u.display_name as performer_name, mb.name as media_batch_name
         FROM subcultures sc
         LEFT JOIN users u ON sc.performed_by = u.id
         LEFT JOIN media_batches mb ON sc.media_batch_id = mb.id
         WHERE sc.specimen_id = ?1
         ORDER BY sc.passage_number DESC
         LIMIT ?2 OFFSET ?3"
    ).map_err(|e| e.to_string())?;

    let subcultures = stmt.query_map(params![specimen_id, pg.limit(), pg.offset()], row_to_subculture)
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect::<Vec<_>>();

    let total_pages = ((total as f64) / (pg.per_page as f64)).ceil() as u32;

    Ok(PaginatedResponse {
        items: subcultures,
        total,
        page: pg.page,
        per_page: pg.per_page,
        total_pages,
    })
}

#[tauri::command]
pub fn create_subculture(
    state: State<AppState>,
    token: String,
    request: CreateSubcultureRequest,
) -> Result<Subculture, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }

    let (current_count, is_archived): (i32, i32) = db.conn.query_row(
        "SELECT subculture_count, is_archived FROM specimens WHERE id = ?1",
        params![request.specimen_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ).map_err(|_| "Specimen not found".to_string())?;

    if is_archived != 0 {
        return Err("Cannot record a passage on an archived specimen".to_string());
    }

    let passage_number = current_count + 1;
    let id = uuid::Uuid::new_v4().to_string();
    let contamination_flag = request.contamination_flag.unwrap_or(false) as i32;

    // ── WP-31: compute PDL gained and doubling time ──────────────────────────
    // Fetch the previous passage date to calculate elapsed hours for doubling time.
    let prev_date: Option<String> = db.conn.query_row(
        "SELECT date FROM subcultures WHERE specimen_id = ?1 AND event_type != 'death'
         ORDER BY passage_number DESC LIMIT 1",
        params![request.specimen_id],
        |r| r.get(0),
    ).ok().flatten();

    let elapsed_hours: Option<f64> = prev_date.as_deref().and_then(|prev| {
        use chrono::NaiveDate;
        let prev_d = NaiveDate::parse_from_str(prev, "%Y-%m-%d").ok()?;
        let curr_d = NaiveDate::parse_from_str(&request.date, "%Y-%m-%d").ok()?;
        let days = (curr_d - prev_d).num_days();
        if days > 0 { Some(days as f64 * 24.0) } else { None }
    });

    // Prefer cell-count-based PDL; fall back to split ratio.
    let pdl_gained: Option<f64> = match (request.seed_cell_count, request.harvest_cell_count) {
        (Some(s), Some(h)) if s > 0.0 && h > 0.0 => {
            queries::calculate_pdl_from_counts(s, h)
        }
        _ => request.split_ratio.and_then(queries::calculate_pdl_from_ratio),
    };

    let doubling_time_hours: Option<f64> = match (
        request.seed_cell_count,
        request.harvest_cell_count,
        elapsed_hours,
    ) {
        (Some(s), Some(h), Some(et)) => queries::calculate_doubling_time(s, h, et),
        _ => None,
    };

    let tx = db.conn.unchecked_transaction()
        .map_err(|e| format!("Failed to begin transaction: {}", e))?;

    tx.execute(
        "INSERT INTO subcultures (id, specimen_id, passage_number, date, media_batch_id,
         ph, temperature_c, light_cycle, light_intensity_lux, experimental_treatment,
         vessel_type, vessel_size, vessel_material, vessel_lid_type,
         location_from, location_to, temp_before, temp_after,
         humidity_before, humidity_after, light_before, light_after,
         exposure_duration_hours, notes, observations, performed_by, employee_id,
         health_status, contamination_flag, contamination_notes,
         seed_cell_count, harvest_cell_count, split_ratio, pdl_gained, doubling_time_hours,
         colonization_pct, contaminant_type)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,?30,?31,?32,?33,?34,?35,?36,?37)",
        params![
            id, request.specimen_id, passage_number, request.date, request.media_batch_id,
            request.ph, request.temperature_c, request.light_cycle, request.light_intensity_lux,
            request.experimental_treatment, request.vessel_type, request.vessel_size,
            request.vessel_material, request.vessel_lid_type, request.location_from,
            request.location_to, request.temp_before, request.temp_after,
            request.humidity_before, request.humidity_after, request.light_before,
            request.light_after, request.exposure_duration_hours, request.notes,
            request.observations, user.id, request.employee_id, request.health_status,
            contamination_flag, request.contamination_notes,
            request.seed_cell_count, request.harvest_cell_count, request.split_ratio,
            pdl_gained, doubling_time_hours,
            request.colonization_pct, request.contaminant_type,
        ],
    ).map_err(|e| format!("Failed to create subculture: {}", e))?;

    // Update specimen: subculture count, location (if transferred), health (if assessed),
    // and cumulative PDL (accumulated from all passages on this specimen).
    tx.execute(
        "UPDATE specimens SET
         subculture_count = ?1,
         location       = CASE WHEN ?2 IS NOT NULL THEN ?2 ELSE location       END,
         health_status  = CASE WHEN ?3 IS NOT NULL THEN ?3 ELSE health_status  END,
         cumulative_pdl = CASE WHEN ?5 IS NOT NULL
                               THEN COALESCE(cumulative_pdl, 0.0) + ?5
                               ELSE cumulative_pdl END,
         updated_at     = datetime('now')
         WHERE id = ?4",
        params![passage_number, request.location_to, request.health_status, request.specimen_id, pdl_gained],
    ).map_err(|e| format!("Failed to update specimen after passage: {}", e))?;

    // Audit passage on the SPECIMEN's chain so chain_seq increments for the specimen
    queries::log_audit(
        &tx,Some(&user.id), "subcultured", "specimen", Some(&request.specimen_id),
        None, None, Some(&format!("Passage #{} recorded", passage_number)),
    ).map_err(|e| format!("Failed to write passage audit: {}", e))?;

    tx.commit().map_err(|e| format!("Failed to commit subculture transaction: {}", e))?;

    db.conn.query_row(
        "SELECT sc.*, u.display_name as performer_name, mb.name as media_batch_name
         FROM subcultures sc
         LEFT JOIN users u ON sc.performed_by = u.id
         LEFT JOIN media_batches mb ON sc.media_batch_id = mb.id
         WHERE sc.id = ?1",
        params![id],
        row_to_subculture,
    ).map_err(|e| format!("Failed to fetch created subculture: {}", e))
}

#[tauri::command]
pub fn update_subculture(
    state: State<AppState>,
    token: String,
    request: UpdateSubcultureRequest,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }

    let mut updates = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(ref notes) = request.notes {
        updates.push(format!("notes = ?{}", values.len() + 1));
        values.push(Box::new(notes.clone()));
    }
    if let Some(ref obs) = request.observations {
        updates.push(format!("observations = ?{}", values.len() + 1));
        values.push(Box::new(obs.clone()));
    }
    if let Some(ref vt) = request.vessel_type {
        updates.push(format!("vessel_type = ?{}", values.len() + 1));
        values.push(Box::new(vt.clone()));
    }
    if let Some(ref lt) = request.location_to {
        updates.push(format!("location_to = ?{}", values.len() + 1));
        values.push(Box::new(lt.clone()));
    }
    if let Some(flag) = request.contamination_flag {
        updates.push(format!("contamination_flag = ?{}", values.len() + 1));
        values.push(Box::new(flag as i32));
    }
    if let Some(ref cn) = request.contamination_notes {
        updates.push(format!("contamination_notes = ?{}", values.len() + 1));
        values.push(Box::new(cn.clone()));
    }
    if let Some(pct) = request.colonization_pct {
        updates.push(format!("colonization_pct = ?{}", values.len() + 1));
        values.push(Box::new(pct));
    }
    if let Some(ref ct) = request.contaminant_type {
        updates.push(format!("contaminant_type = ?{}", values.len() + 1));
        values.push(Box::new(ct.clone()));
    }

    if updates.is_empty() {
        return Err("No fields to update".to_string());
    }

    updates.push("updated_at = datetime('now')".to_string());
    let sql = format!(
        "UPDATE subcultures SET {} WHERE id = ?{}",
        updates.join(", "),
        values.len() + 1
    );
    values.push(Box::new(request.id.clone()));

    let bind_refs: Vec<&dyn rusqlite::types::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    db.conn.execute(&sql, bind_refs.as_slice())
        .map_err(|e| format!("Failed to update subculture: {}", e))?;

    queries::log_audit(
        &db.conn, Some(&user.id), "update", "subculture", Some(&request.id),
        None, None, Some("Subculture updated"),
    ).ok();

    Ok(())
}

// ── All Subcultures (for export) ─────────────────────────────────────────────

#[tauri::command]
pub fn list_all_subcultures(
    state: State<AppState>,
    token: String,
) -> Result<Vec<Subculture>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;

    let mut stmt = db.conn.prepare(
        "SELECT sc.*, u.display_name as performer_name, mb.name as media_batch_name
         FROM subcultures sc
         LEFT JOIN users u ON sc.performed_by = u.id
         LEFT JOIN media_batches mb ON sc.media_batch_id = mb.id
         ORDER BY sc.date DESC, sc.passage_number DESC"
    ).map_err(|e| e.to_string())?;

    let subcultures = stmt.query_map([], row_to_subculture)
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(subcultures)
}

// ── Colonization History ─────────────────────────────────────────────────────

#[tauri::command]
pub fn get_colonization_history(
    state: State<AppState>,
    token: String,
    specimen_id: String,
) -> Result<Vec<ColonizationEntry>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    let mut stmt = db.conn.prepare(
        "SELECT id, date, colonization_pct, passage_number, notes
         FROM subcultures
         WHERE specimen_id = ?1 AND colonization_pct IS NOT NULL
         ORDER BY date ASC, passage_number ASC",
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![specimen_id], |row| {
        Ok(ColonizationEntry {
            subculture_id: row.get("id")?,
            date: row.get("date")?,
            colonization_pct: row.get("colonization_pct")?,
            passage_number: row.get("passage_number")?,
            notes: row.get("notes")?,
        })
    }).map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

// ── Contamination Stats ──────────────────────────────────────────────────────

#[tauri::command]
pub fn get_contamination_stats(
    state: State<AppState>,
    token: String,
) -> Result<ContaminationStats, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    let profile = crate::db::vocabulary::active_profile(&db.conn);
    crate::db::dashboard::query_contamination_stats(&db.conn, &profile)
}

// ── Subculture Schedule ──────────────────────────────────────────────────────

#[tauri::command]
pub fn get_subculture_schedule(
    state: State<AppState>,
    token: String,
) -> Result<Vec<SubcultureScheduleEntry>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    let profile = crate::db::vocabulary::active_profile(&db.conn);
    crate::db::dashboard::query_subculture_schedule(&db.conn, &profile)
}


#[tauri::command]
pub fn get_culture_maintenance_alerts(
    state: State<AppState>,
    token: String,
) -> Result<Vec<CultureMaintenanceAlert>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    let profile = crate::db::vocabulary::active_profile(&db.conn);
    crate::db::dashboard::query_culture_maintenance_alerts(&db.conn, &profile)
}

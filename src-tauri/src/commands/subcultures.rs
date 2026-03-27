use crate::auth as auth_service;
use crate::db::queries;
use crate::models::subculture::*;
use crate::AppState;
use rusqlite::params;
use tauri::State;

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
    })
}

#[tauri::command]
pub fn list_subcultures(
    state: State<AppState>,
    token: String,
    specimen_id: String,
) -> Result<Vec<Subculture>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;

    let mut stmt = db.conn.prepare(
        "SELECT sc.*, u.display_name as performer_name, mb.name as media_batch_name
         FROM subcultures sc
         LEFT JOIN users u ON sc.performed_by = u.id
         LEFT JOIN media_batches mb ON sc.media_batch_id = mb.id
         WHERE sc.specimen_id = ?1
         ORDER BY sc.passage_number DESC"
    ).map_err(|e| e.to_string())?;

    let subcultures = stmt.query_map(params![specimen_id], row_to_subculture)
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(subcultures)
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

    let current_count: i32 = db.conn.query_row(
        "SELECT subculture_count FROM specimens WHERE id = ?1",
        params![request.specimen_id],
        |r| r.get(0),
    ).map_err(|_| "Specimen not found".to_string())?;

    let passage_number = current_count + 1;
    let id = uuid::Uuid::new_v4().to_string();
    let contamination_flag = request.contamination_flag.unwrap_or(false) as i32;

    db.conn.execute(
        "INSERT INTO subcultures (id, specimen_id, passage_number, date, media_batch_id,
         ph, temperature_c, light_cycle, light_intensity_lux, experimental_treatment,
         vessel_type, vessel_size, vessel_material, vessel_lid_type,
         location_from, location_to, temp_before, temp_after,
         humidity_before, humidity_after, light_before, light_after,
         exposure_duration_hours, notes, observations, performed_by, employee_id,
         health_status, contamination_flag, contamination_notes)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,?30)",
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
        ],
    ).map_err(|e| format!("Failed to create subculture: {}", e))?;

    db.conn.execute(
        "UPDATE specimens SET subculture_count = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![passage_number, request.specimen_id],
    ).ok();

    if let Some(ref loc) = request.location_to {
        db.conn.execute(
            "UPDATE specimens SET location = ?1 WHERE id = ?2",
            params![loc, request.specimen_id],
        ).ok();
    }

    queries::log_audit(
        &db.conn, Some(&user.id), "create", "subculture", Some(&id),
        None, None, Some(&format!("Passage #{} recorded", passage_number)),
    ).ok();

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

// ── Contamination Stats ──────────────────────────────────────────────────────

#[tauri::command]
pub fn get_contamination_stats(
    state: State<AppState>,
    token: String,
) -> Result<ContaminationStats, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;

    let total_specimens: i64 = db.conn.query_row(
        "SELECT COUNT(*) FROM specimens WHERE is_archived = 0",
        [],
        |r| r.get(0),
    ).unwrap_or(0);

    let contaminated_specimens: i64 = db.conn.query_row(
        "SELECT COUNT(DISTINCT sc.specimen_id)
         FROM subcultures sc
         JOIN specimens sp ON sc.specimen_id = sp.id
         WHERE sc.contamination_flag = 1 AND sp.is_archived = 0",
        [],
        |r| r.get(0),
    ).unwrap_or(0);

    let contamination_rate_pct = if total_specimens > 0 {
        (contaminated_specimens as f64 / total_specimens as f64) * 100.0
    } else {
        0.0
    };

    let contaminated_vessels: i64 = db.conn.query_row(
        "SELECT COUNT(*) FROM subcultures WHERE contamination_flag = 1",
        [],
        |r| r.get(0),
    ).unwrap_or(0);

    let mut stmt = db.conn.prepare(
        "SELECT COALESCE(vessel_type, 'Unknown') as vessel_type, COUNT(*) as cnt
         FROM subcultures
         WHERE contamination_flag = 1
         GROUP BY vessel_type
         ORDER BY cnt DESC
         LIMIT 10"
    ).map_err(|e| e.to_string())?;

    let by_vessel_type: Vec<VesselContaminationCount> = stmt
        .query_map([], |row| {
            Ok(VesselContaminationCount {
                vessel_type: row.get("vessel_type")?,
                count: row.get("cnt")?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut stmt2 = db.conn.prepare(
        "SELECT sc.id as subculture_id, sc.specimen_id, sp.accession_number,
                s.species_code, sc.passage_number, sc.date, sc.vessel_type,
                sc.contamination_notes
         FROM subcultures sc
         JOIN specimens sp ON sc.specimen_id = sp.id
         JOIN species s ON sp.species_id = s.id
         WHERE sc.contamination_flag = 1
         ORDER BY sc.date DESC
         LIMIT 10"
    ).map_err(|e| e.to_string())?;

    let recent_events: Vec<RecentContaminationEvent> = stmt2
        .query_map([], |row| {
            Ok(RecentContaminationEvent {
                subculture_id: row.get("subculture_id")?,
                specimen_id: row.get("specimen_id")?,
                accession_number: row.get("accession_number")?,
                species_code: row.get("species_code")?,
                passage_number: row.get("passage_number")?,
                date: row.get("date")?,
                vessel_type: row.get("vessel_type")?,
                contamination_notes: row.get("contamination_notes")?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(ContaminationStats {
        total_specimens,
        contaminated_specimens,
        contamination_rate_pct,
        contaminated_vessels,
        by_vessel_type,
        recent_events,
    })
}

// ── Subculture Schedule ──────────────────────────────────────────────────────

#[tauri::command]
pub fn get_subculture_schedule(
    state: State<AppState>,
    token: String,
) -> Result<Vec<SubcultureScheduleEntry>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;

    // For each active specimen, compute next due date using the species default
    // subculture interval and the date of the most recent passage.
    let mut stmt = db.conn.prepare(
        "SELECT
            sp.id              AS specimen_id,
            sp.accession_number,
            s.species_code     AS species_code,
            (s.genus || ' ' || s.species_name) AS species_name,
            sp.location,
            MAX(sc.date)       AS last_passage_date,
            s.default_subculture_interval_days AS interval_days,
            CASE
                WHEN s.default_subculture_interval_days IS NOT NULL AND MAX(sc.date) IS NOT NULL
                THEN date(MAX(sc.date), '+' || s.default_subculture_interval_days || ' days')
                ELSE NULL
            END AS next_due_date,
            CASE
                WHEN s.default_subculture_interval_days IS NOT NULL AND MAX(sc.date) IS NOT NULL
                THEN CAST(julianday(date(MAX(sc.date), '+' || s.default_subculture_interval_days || ' days')) - julianday('now') AS INTEGER)
                ELSE NULL
            END AS days_until_due
         FROM specimens sp
         JOIN species s ON sp.species_id = s.id
         LEFT JOIN subcultures sc ON sc.specimen_id = sp.id
         WHERE sp.is_archived = 0
         GROUP BY sp.id
         ORDER BY days_until_due ASC NULLS LAST"
    ).map_err(|e| e.to_string())?;

    let entries: Vec<SubcultureScheduleEntry> = stmt
        .query_map([], |row| {
            let days_until_due: Option<i64> = row.get("days_until_due")?;
            let is_overdue = days_until_due.map(|d| d < 0).unwrap_or(false);
            Ok(SubcultureScheduleEntry {
                specimen_id: row.get("specimen_id")?,
                accession_number: row.get("accession_number")?,
                species_code: row.get("species_code")?,
                species_name: row.get("species_name")?,
                location: row.get("location")?,
                last_passage_date: row.get("last_passage_date")?,
                interval_days: row.get("interval_days")?,
                next_due_date: row.get("next_due_date")?,
                days_until_due,
                is_overdue,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(entries)
}

use crate::auth as auth_service;
use crate::db::queries;
use crate::models::subculture::*;
use crate::AppState;
use rusqlite::params;
use tauri::State;

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

    let subcultures = stmt.query_map(params![specimen_id], |row| {
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
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }).map_err(|e| e.to_string())?
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

    // Get current subculture count and increment
    let current_count: i32 = db.conn.query_row(
        "SELECT subculture_count FROM specimens WHERE id = ?1",
        params![request.specimen_id],
        |r| r.get(0),
    ).map_err(|_| "Specimen not found".to_string())?;

    let passage_number = current_count + 1;
    let id = uuid::Uuid::new_v4().to_string();

    db.conn.execute(
        "INSERT INTO subcultures (id, specimen_id, passage_number, date, media_batch_id,
         ph, temperature_c, light_cycle, light_intensity_lux, experimental_treatment,
         vessel_type, vessel_size, vessel_material, vessel_lid_type,
         location_from, location_to, temp_before, temp_after,
         humidity_before, humidity_after, light_before, light_after,
         exposure_duration_hours, notes, observations, performed_by, employee_id, health_status)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28)",
        params![
            id, request.specimen_id, passage_number, request.date, request.media_batch_id,
            request.ph, request.temperature_c, request.light_cycle, request.light_intensity_lux,
            request.experimental_treatment, request.vessel_type, request.vessel_size,
            request.vessel_material, request.vessel_lid_type, request.location_from,
            request.location_to, request.temp_before, request.temp_after,
            request.humidity_before, request.humidity_after, request.light_before,
            request.light_after, request.exposure_duration_hours, request.notes,
            request.observations, user.id, request.employee_id, request.health_status,
        ],
    ).map_err(|e| format!("Failed to create subculture: {}", e))?;

    // Update specimen subculture count and location
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

    // Return the created subculture
    db.conn.query_row(
        "SELECT sc.*, u.display_name as performer_name, mb.name as media_batch_name
         FROM subcultures sc
         LEFT JOIN users u ON sc.performed_by = u.id
         LEFT JOIN media_batches mb ON sc.media_batch_id = mb.id
         WHERE sc.id = ?1",
        params![id],
        |row| {
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
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        },
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

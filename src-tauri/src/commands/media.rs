use crate::auth as auth_service;
use crate::db::queries;
use crate::models::media::*;
use crate::AppState;
use rusqlite::params;
use tauri::State;

#[tauri::command]
pub fn list_media(state: State<AppState>, token: String) -> Result<Vec<MediaBatch>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;

    let mut stmt = db.conn.prepare(
        "SELECT * FROM media_batches ORDER BY preparation_date DESC"
    ).map_err(|e| e.to_string())?;

    let batches: Vec<MediaBatch> = stmt.query_map([], |row| {
        Ok(MediaBatch {
            id: row.get("id")?,
            batch_id: row.get("batch_id")?,
            name: row.get("name")?,
            preparation_date: row.get("preparation_date")?,
            expiration_date: row.get("expiration_date")?,
            basal_salts: row.get("basal_salts")?,
            basal_salts_concentration: row.get("basal_salts_concentration")?,
            vitamins: row.get("vitamins")?,
            sucrose_g_per_l: row.get("sucrose_g_per_l")?,
            agar_g_per_l: row.get("agar_g_per_l")?,
            gelling_agent: row.get("gelling_agent")?,
            ph_before_autoclave: row.get("ph_before_autoclave")?,
            ph_after_autoclave: row.get("ph_after_autoclave")?,
            sterilization_method: row.get("sterilization_method")?,
            volume_prepared_ml: row.get("volume_prepared_ml")?,
            volume_used_ml: row.get("volume_used_ml")?,
            volume_remaining_ml: row.get("volume_remaining_ml")?,
            storage_conditions: row.get("storage_conditions")?,
            qc_notes: row.get("qc_notes")?,
            supplier_info: row.get("supplier_info")?,
            cost_per_batch: row.get("cost_per_batch")?,
            osmolarity: row.get("osmolarity")?,
            conductivity: row.get("conductivity")?,
            is_custom: row.get::<_, i32>("is_custom")? != 0,
            needs_review: row.get::<_, i32>("needs_review")? != 0,
            notes: row.get("notes")?,
            hormones: Vec::new(), // loaded separately
            employee_id: row.get("employee_id")?,
            created_by: row.get("created_by")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }).map_err(|e| e.to_string())?
      .filter_map(|r| r.ok())
      .collect();

    // Load hormones for each batch
    let mut result = batches;
    for batch in &mut result {
        let mut h_stmt = db.conn.prepare(
            "SELECT * FROM media_hormones WHERE media_batch_id = ?1"
        ).map_err(|e| e.to_string())?;
        batch.hormones = h_stmt.query_map(params![batch.id], |row| {
            Ok(MediaHormone {
                id: row.get("id")?,
                hormone_name: row.get("hormone_name")?,
                hormone_type: row.get("hormone_type")?,
                concentration_mg_per_l: row.get("concentration_mg_per_l")?,
                supplier: row.get("supplier")?,
                lot_number: row.get("lot_number")?,
                reagent_batch_id: row.get("reagent_batch_id")?,
                amount_used: row.get("amount_used")?,
                amount_unit: row.get("amount_unit")?,
            })
        }).map_err(|e| e.to_string())?
          .filter_map(|r| r.ok())
          .collect();
    }

    Ok(result)
}

#[tauri::command]
pub fn get_media_batch(state: State<AppState>, token: String, id: String) -> Result<MediaBatch, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;

    let mut batch = db.conn.query_row(
        "SELECT * FROM media_batches WHERE id = ?1",
        params![id],
        |row| {
            Ok(MediaBatch {
                id: row.get("id")?,
                batch_id: row.get("batch_id")?,
                name: row.get("name")?,
                preparation_date: row.get("preparation_date")?,
                expiration_date: row.get("expiration_date")?,
                basal_salts: row.get("basal_salts")?,
                basal_salts_concentration: row.get("basal_salts_concentration")?,
                vitamins: row.get("vitamins")?,
                sucrose_g_per_l: row.get("sucrose_g_per_l")?,
                agar_g_per_l: row.get("agar_g_per_l")?,
                gelling_agent: row.get("gelling_agent")?,
                ph_before_autoclave: row.get("ph_before_autoclave")?,
                ph_after_autoclave: row.get("ph_after_autoclave")?,
                sterilization_method: row.get("sterilization_method")?,
                volume_prepared_ml: row.get("volume_prepared_ml")?,
                volume_used_ml: row.get("volume_used_ml")?,
                volume_remaining_ml: row.get("volume_remaining_ml")?,
                storage_conditions: row.get("storage_conditions")?,
                qc_notes: row.get("qc_notes")?,
                supplier_info: row.get("supplier_info")?,
                cost_per_batch: row.get("cost_per_batch")?,
                osmolarity: row.get("osmolarity")?,
                conductivity: row.get("conductivity")?,
                is_custom: row.get::<_, i32>("is_custom")? != 0,
                needs_review: row.get::<_, i32>("needs_review")? != 0,
                notes: row.get("notes")?,
                hormones: Vec::new(),
                employee_id: row.get("employee_id")?,
                created_by: row.get("created_by")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        },
    ).map_err(|e| format!("Media batch not found: {}", e))?;

    let mut h_stmt = db.conn.prepare(
        "SELECT * FROM media_hormones WHERE media_batch_id = ?1"
    ).map_err(|e| e.to_string())?;
    batch.hormones = h_stmt.query_map(params![batch.id], |row| {
        Ok(MediaHormone {
            id: row.get("id")?,
            hormone_name: row.get("hormone_name")?,
            hormone_type: row.get("hormone_type")?,
            concentration_mg_per_l: row.get("concentration_mg_per_l")?,
            supplier: row.get("supplier")?,
            lot_number: row.get("lot_number")?,
            reagent_batch_id: row.get("reagent_batch_id")?,
            amount_used: row.get("amount_used")?,
            amount_unit: row.get("amount_unit")?,
        })
    }).map_err(|e| e.to_string())?
      .filter_map(|r| r.ok())
      .collect();

    Ok(batch)
}

#[tauri::command]
pub fn create_media_batch(
    state: State<AppState>,
    token: String,
    request: CreateMediaBatchRequest,
) -> Result<MediaBatch, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }

    let id = uuid::Uuid::new_v4().to_string();
    let batch_id = generate_batch_id(&db.conn);
    let volume_remaining = request.volume_prepared_ml;

    db.conn.execute(
        "INSERT INTO media_batches (id, batch_id, name, preparation_date, expiration_date,
         basal_salts, basal_salts_concentration, vitamins, sucrose_g_per_l, agar_g_per_l,
         gelling_agent, ph_before_autoclave, ph_after_autoclave, sterilization_method,
         volume_prepared_ml, volume_remaining_ml, storage_conditions, qc_notes,
         supplier_info, cost_per_batch, osmolarity, conductivity, is_custom, notes, employee_id, created_by)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26)",
        params![
            id, batch_id, request.name, request.preparation_date, request.expiration_date,
            request.basal_salts, request.basal_salts_concentration, request.vitamins,
            request.sucrose_g_per_l, request.agar_g_per_l, request.gelling_agent,
            request.ph_before_autoclave, request.ph_after_autoclave, request.sterilization_method,
            request.volume_prepared_ml, volume_remaining, request.storage_conditions,
            request.qc_notes, request.supplier_info, request.cost_per_batch,
            request.osmolarity, request.conductivity, request.is_custom.unwrap_or(false) as i32,
            request.notes, request.employee_id, user.id,
        ],
    ).map_err(|e| format!("Failed to create media batch: {}", e))?;

    // Insert hormones/reagents and auto-deduct stock
    if let Some(hormones) = &request.hormones {
        for h in hormones {
            let h_id = uuid::Uuid::new_v4().to_string();
            db.conn.execute(
                "INSERT INTO media_hormones
                 (id, media_batch_id, hormone_name, hormone_type, concentration_mg_per_l,
                  supplier, lot_number, reagent_batch_id, amount_used, amount_unit)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
                params![
                    h_id, id, h.hormone_name, h.hormone_type, h.concentration_mg_per_l,
                    h.supplier, h.lot_number, h.reagent_batch_id,
                    h.amount_used, h.amount_unit,
                ],
            ).map_err(|e| format!("Failed to add hormone: {}", e))?;

            // Deduct from inventory if a physical amount was recorded
            if let (Some(ref inv_id), Some(used)) = (&h.reagent_batch_id, h.amount_used) {
                if used > 0.0 {
                    let cur: Option<f64> = db.conn.query_row(
                        "SELECT current_stock FROM inventory_items WHERE id = ?1",
                        params![inv_id], |row| row.get(0),
                    ).ok();
                    if let Some(c) = cur {
                        let new_stock = (c - used).max(0.0);
                        db.conn.execute(
                            "UPDATE inventory_items SET current_stock = ?1, updated_at = datetime('now') WHERE id = ?2",
                            params![new_stock, inv_id],
                        ).ok();
                        crate::db::queries::log_audit(
                            &db.conn, Some(&user.id), "update", "inventory_item", Some(inv_id),
                            Some(&c.to_string()), Some(&new_stock.to_string()),
                            Some(&format!("Used in media batch {} ({})", batch_id, h.hormone_name)),
                        ).ok();
                    }
                }
            }
        }
    }

    queries::log_audit(
        &db.conn, Some(&user.id), "create", "media_batch", Some(&id),
        None, Some(&batch_id), Some("Media batch created"),
    ).ok();

    drop(db);
    get_media_batch(state, token, id)
}

#[tauri::command]
pub fn update_media_batch(
    state: State<AppState>,
    token: String,
    request: UpdateMediaBatchRequest,
) -> Result<MediaBatch, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }

    let mut updates = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(ref name) = request.name {
        updates.push(format!("name = ?{}", values.len() + 1));
        values.push(Box::new(name.clone()));
    }
    if let Some(ref exp) = request.expiration_date {
        updates.push(format!("expiration_date = ?{}", values.len() + 1));
        values.push(Box::new(exp.clone()));
    }
    if let Some(vu) = request.volume_used_ml {
        updates.push(format!("volume_used_ml = ?{}", values.len() + 1));
        values.push(Box::new(vu));
    }
    if let Some(vr) = request.volume_remaining_ml {
        updates.push(format!("volume_remaining_ml = ?{}", values.len() + 1));
        values.push(Box::new(vr));
    }
    if let Some(ref sc) = request.storage_conditions {
        updates.push(format!("storage_conditions = ?{}", values.len() + 1));
        values.push(Box::new(sc.clone()));
    }
    if let Some(ref qc) = request.qc_notes {
        updates.push(format!("qc_notes = ?{}", values.len() + 1));
        values.push(Box::new(qc.clone()));
    }
    if let Some(nr) = request.needs_review {
        updates.push(format!("needs_review = ?{}", values.len() + 1));
        values.push(Box::new(nr as i32));
    }
    if let Some(ref notes) = request.notes {
        updates.push(format!("notes = ?{}", values.len() + 1));
        values.push(Box::new(notes.clone()));
    }

    if updates.is_empty() {
        return Err("No fields to update".to_string());
    }

    updates.push("updated_at = datetime('now')".to_string());
    let sql = format!(
        "UPDATE media_batches SET {} WHERE id = ?{}",
        updates.join(", "),
        values.len() + 1
    );
    values.push(Box::new(request.id.clone()));

    let bind_refs: Vec<&dyn rusqlite::types::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    db.conn.execute(&sql, bind_refs.as_slice())
        .map_err(|e| format!("Failed to update media batch: {}", e))?;

    queries::log_audit(
        &db.conn, Some(&user.id), "update", "media_batch", Some(&request.id),
        None, None, Some("Media batch updated"),
    ).ok();

    drop(db);
    get_media_batch(state, token, request.id)
}

#[tauri::command]
pub fn delete_media_batch(state: State<AppState>, token: String, id: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err("Only supervisors and admins can delete media batches".to_string());
    }

    db.conn.execute("DELETE FROM media_hormones WHERE media_batch_id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    db.conn.execute("DELETE FROM media_batches WHERE id = ?1", params![id])
        .map_err(|e| format!("Failed to delete media batch: {}", e))?;

    queries::log_audit(
        &db.conn, Some(&user.id), "delete", "media_batch", Some(&id),
        None, None, Some("Media batch deleted"),
    ).ok();

    Ok(())
}

fn generate_batch_id(conn: &rusqlite::Connection) -> String {
    let date = chrono::Utc::now().format("%Y%m%d").to_string();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM media_batches WHERE batch_id LIKE ?1",
            params![format!("MB-{}-%", date)],
            |r| r.get(0),
        )
        .unwrap_or(0);
    format!("MB-{}-{:03}", date, count + 1)
}

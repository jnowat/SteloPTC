use crate::auth as auth_service;
use crate::db::queries;
use crate::models::inventory::*;
use crate::AppState;
use rusqlite::params;
use tauri::State;

fn row_to_item(row: &rusqlite::Row) -> rusqlite::Result<InventoryItem> {
    Ok(InventoryItem {
        id: row.get("id")?,
        name: row.get("name")?,
        category: row.get("category")?,
        unit: row.get("unit")?,
        physical_state: row.get("physical_state")?,
        concentration: row.get("concentration")?,
        concentration_unit: row.get("concentration_unit")?,
        current_stock: row.get("current_stock")?,
        minimum_stock: row.get("minimum_stock")?,
        reorder_point: row.get("reorder_point")?,
        supplier: row.get("supplier")?,
        catalog_number: row.get("catalog_number")?,
        lot_number: row.get("lot_number")?,
        storage_location: row.get("storage_location")?,
        expiration_date: row.get("expiration_date")?,
        cost_per_unit: row.get("cost_per_unit")?,
        notes: row.get("notes")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

#[tauri::command]
pub fn list_inventory(state: State<AppState>, token: String) -> Result<Vec<InventoryItem>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;

    let mut stmt = db
        .conn
        .prepare("SELECT * FROM inventory_items ORDER BY category, name")
        .map_err(|e| e.to_string())?;

    let items = stmt
        .query_map([], |row| row_to_item(row))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(items)
}

#[tauri::command]
pub fn create_inventory_item(
    state: State<AppState>,
    token: String,
    request: CreateInventoryItemRequest,
) -> Result<InventoryItem, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }

    let id = uuid::Uuid::new_v4().to_string();
    let current_stock = request.current_stock.unwrap_or(0.0);
    let minimum_stock = request.minimum_stock.unwrap_or(0.0);
    let physical_state = request.physical_state.as_deref().unwrap_or("solid");

    db.conn
        .execute(
            "INSERT INTO inventory_items
             (id, name, category, unit, physical_state, concentration, concentration_unit,
              current_stock, minimum_stock, reorder_point, supplier, catalog_number, lot_number,
              storage_location, expiration_date, cost_per_unit, notes)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)",
            params![
                id, request.name, request.category, request.unit,
                physical_state, request.concentration, request.concentration_unit,
                current_stock, minimum_stock, request.reorder_point,
                request.supplier, request.catalog_number, request.lot_number,
                request.storage_location, request.expiration_date,
                request.cost_per_unit, request.notes,
            ],
        )
        .map_err(|e| format!("Failed to create inventory item: {}", e))?;

    queries::log_audit(
        &db.conn, Some(&user.id), "create", "inventory_item", Some(&id),
        None, Some(&request.name), Some("Inventory item created"),
    ).ok();

    db.conn
        .query_row("SELECT * FROM inventory_items WHERE id = ?1", params![id], |row| row_to_item(row))
        .map_err(|e| format!("Failed to retrieve created item: {}", e))
}

#[tauri::command]
pub fn update_inventory_item(
    state: State<AppState>,
    token: String,
    request: UpdateInventoryItemRequest,
) -> Result<InventoryItem, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }

    let mut updates = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    macro_rules! add_str {
        ($field:ident, $col:expr) => {
            if let Some(ref val) = request.$field {
                updates.push(format!("{} = ?{}", $col, values.len() + 1));
                values.push(Box::new(val.clone()));
            }
        };
    }
    macro_rules! add_f64 {
        ($field:ident, $col:expr) => {
            if let Some(val) = request.$field {
                updates.push(format!("{} = ?{}", $col, values.len() + 1));
                values.push(Box::new(val));
            }
        };
    }

    add_str!(name, "name");
    add_str!(category, "category");
    add_str!(unit, "unit");
    add_str!(physical_state, "physical_state");
    add_str!(concentration_unit, "concentration_unit");
    add_str!(supplier, "supplier");
    add_str!(catalog_number, "catalog_number");
    add_str!(lot_number, "lot_number");
    add_str!(storage_location, "storage_location");
    add_str!(expiration_date, "expiration_date");
    add_str!(notes, "notes");
    add_f64!(concentration, "concentration");
    add_f64!(current_stock, "current_stock");
    add_f64!(minimum_stock, "minimum_stock");
    add_f64!(reorder_point, "reorder_point");
    add_f64!(cost_per_unit, "cost_per_unit");

    if updates.is_empty() {
        return Err("No fields to update".to_string());
    }

    updates.push("updated_at = datetime('now')".to_string());
    let sql = format!(
        "UPDATE inventory_items SET {} WHERE id = ?{}",
        updates.join(", "),
        values.len() + 1
    );
    values.push(Box::new(request.id.clone()));

    let bind_refs: Vec<&dyn rusqlite::types::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    db.conn
        .execute(&sql, bind_refs.as_slice())
        .map_err(|e| format!("Failed to update inventory item: {}", e))?;

    queries::log_audit(
        &db.conn, Some(&user.id), "update", "inventory_item", Some(&request.id),
        None, None, Some("Inventory item updated"),
    ).ok();

    db.conn
        .query_row("SELECT * FROM inventory_items WHERE id = ?1", params![request.id], |row| row_to_item(row))
        .map_err(|e| format!("Failed to retrieve updated item: {}", e))
}

#[tauri::command]
pub fn delete_inventory_item(
    state: State<AppState>,
    token: String,
    id: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err("Only supervisors and admins can delete inventory items".to_string());
    }

    db.conn
        .execute("DELETE FROM inventory_items WHERE id = ?1", params![id])
        .map_err(|e| format!("Failed to delete inventory item: {}", e))?;

    queries::log_audit(
        &db.conn, Some(&user.id), "delete", "inventory_item", Some(&id),
        None, None, Some("Inventory item deleted"),
    ).ok();

    Ok(())
}

#[tauri::command]
pub fn adjust_stock(
    state: State<AppState>,
    token: String,
    id: String,
    adjustment: f64,
    reason: Option<String>,
) -> Result<InventoryItem, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }

    let current: f64 = db
        .conn
        .query_row(
            "SELECT current_stock FROM inventory_items WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .map_err(|_| "Inventory item not found".to_string())?;

    let new_stock = current + adjustment;
    if new_stock < 0.0 {
        return Err("Stock cannot go below zero".to_string());
    }

    db.conn
        .execute(
            "UPDATE inventory_items SET current_stock = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![new_stock, id],
        )
        .map_err(|e| format!("Failed to adjust stock: {}", e))?;

    let detail = format!(
        "Stock adjusted by {}{}: {} -> {}{}",
        if adjustment >= 0.0 { "+" } else { "" },
        adjustment,
        current,
        new_stock,
        reason.as_deref().map(|r| format!(" ({})", r)).unwrap_or_default()
    );

    queries::log_audit(
        &db.conn, Some(&user.id), "update", "inventory_item", Some(&id),
        Some(&current.to_string()), Some(&new_stock.to_string()), Some(&detail),
    ).ok();

    db.conn
        .query_row("SELECT * FROM inventory_items WHERE id = ?1", params![id], |row| row_to_item(row))
        .map_err(|e| format!("Failed to retrieve item: {}", e))
}

#[tauri::command]
pub fn get_low_stock_alerts(
    state: State<AppState>,
    token: String,
) -> Result<Vec<LowStockAlert>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;

    let mut stmt = db
        .conn
        .prepare(
            "SELECT id, name, category, current_stock, minimum_stock, reorder_point, unit, physical_state
             FROM inventory_items
             WHERE current_stock <= minimum_stock
                OR (reorder_point IS NOT NULL AND current_stock <= reorder_point)
             ORDER BY (current_stock / CASE WHEN minimum_stock > 0 THEN minimum_stock ELSE 1 END) ASC",
        )
        .map_err(|e| e.to_string())?;

    let alerts = stmt
        .query_map([], |row| {
            Ok(LowStockAlert {
                id: row.get(0)?,
                name: row.get(1)?,
                category: row.get(2)?,
                current_stock: row.get(3)?,
                minimum_stock: row.get(4)?,
                reorder_point: row.get(5)?,
                unit: row.get(6)?,
                physical_state: row.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(alerts)
}

// Prepared solutions commands

#[tauri::command]
pub fn list_prepared_solutions(
    state: State<AppState>,
    token: String,
) -> Result<Vec<PreparedSolution>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;

    let mut stmt = db.conn.prepare(
        "SELECT ps.*, ii.name as source_item_name
         FROM prepared_solutions ps
         LEFT JOIN inventory_items ii ON ps.source_item_id = ii.id
         ORDER BY ps.preparation_date DESC"
    ).map_err(|e| e.to_string())?;

    let solutions = stmt.query_map([], |row| {
        Ok(PreparedSolution {
            id: row.get("id")?,
            name: row.get("name")?,
            source_item_id: row.get("source_item_id")?,
            source_item_name: row.get("source_item_name")?,
            concentration: row.get("concentration")?,
            concentration_unit: row.get("concentration_unit")?,
            solvent: row.get("solvent")?,
            volume_ml: row.get("volume_ml")?,
            volume_remaining_ml: row.get("volume_remaining_ml")?,
            prepared_by: row.get("prepared_by")?,
            preparation_date: row.get("preparation_date")?,
            expiration_date: row.get("expiration_date")?,
            storage_conditions: row.get("storage_conditions")?,
            lot_number: row.get("lot_number")?,
            notes: row.get("notes")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }).map_err(|e| e.to_string())?
      .filter_map(|r| r.ok())
      .collect();

    Ok(solutions)
}

#[tauri::command]
pub fn create_prepared_solution(
    state: State<AppState>,
    token: String,
    request: CreatePreparedSolutionRequest,
) -> Result<PreparedSolution, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }

    let id = uuid::Uuid::new_v4().to_string();

    db.conn.execute(
        "INSERT INTO prepared_solutions
         (id, name, source_item_id, concentration, concentration_unit, solvent,
          volume_ml, volume_remaining_ml, prepared_by, preparation_date,
          expiration_date, storage_conditions, lot_number, notes)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
        params![
            id, request.name, request.source_item_id,
            request.concentration, request.concentration_unit, request.solvent,
            request.volume_ml, request.volume_ml, // volume_remaining starts as full volume
            request.prepared_by, request.preparation_date,
            request.expiration_date, request.storage_conditions,
            request.lot_number, request.notes,
        ],
    ).map_err(|e| format!("Failed to create prepared solution: {}", e))?;

    // Deduct from inventory if source item specified and amount_used given
    if let (Some(ref src_id), Some(amount)) = (&request.source_item_id, request.source_amount_used) {
        let current: Option<f64> = db.conn.query_row(
            "SELECT current_stock FROM inventory_items WHERE id = ?1",
            params![src_id],
            |row| row.get(0),
        ).ok();

        if let Some(cur) = current {
            let new_stock = (cur - amount).max(0.0);
            db.conn.execute(
                "UPDATE inventory_items SET current_stock = ?1, updated_at = datetime('now') WHERE id = ?2",
                params![new_stock, src_id],
            ).ok();
            queries::log_audit(
                &db.conn, Some(&user.id), "update", "inventory_item", Some(src_id),
                Some(&cur.to_string()), Some(&new_stock.to_string()),
                Some(&format!("Stock deducted for prepared solution: {}", request.name)),
            ).ok();
        }
    }

    queries::log_audit(
        &db.conn, Some(&user.id), "create", "prepared_solution", Some(&id),
        None, Some(&request.name), Some("Prepared solution created"),
    ).ok();

    db.conn.query_row(
        "SELECT ps.*, ii.name as source_item_name
         FROM prepared_solutions ps
         LEFT JOIN inventory_items ii ON ps.source_item_id = ii.id
         WHERE ps.id = ?1",
        params![id],
        |row| {
            Ok(PreparedSolution {
                id: row.get("id")?,
                name: row.get("name")?,
                source_item_id: row.get("source_item_id")?,
                source_item_name: row.get("source_item_name")?,
                concentration: row.get("concentration")?,
                concentration_unit: row.get("concentration_unit")?,
                solvent: row.get("solvent")?,
                volume_ml: row.get("volume_ml")?,
                volume_remaining_ml: row.get("volume_remaining_ml")?,
                prepared_by: row.get("prepared_by")?,
                preparation_date: row.get("preparation_date")?,
                expiration_date: row.get("expiration_date")?,
                storage_conditions: row.get("storage_conditions")?,
                lot_number: row.get("lot_number")?,
                notes: row.get("notes")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        },
    ).map_err(|e| format!("Failed to fetch created solution: {}", e))
}

#[tauri::command]
pub fn update_prepared_solution(
    state: State<AppState>,
    token: String,
    request: UpdatePreparedSolutionRequest,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }

    let mut updates = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(v) = request.volume_remaining_ml {
        updates.push(format!("volume_remaining_ml = ?{}", values.len() + 1));
        values.push(Box::new(v));
    }
    if let Some(ref s) = request.storage_conditions {
        updates.push(format!("storage_conditions = ?{}", values.len() + 1));
        values.push(Box::new(s.clone()));
    }
    if let Some(ref e) = request.expiration_date {
        updates.push(format!("expiration_date = ?{}", values.len() + 1));
        values.push(Box::new(e.clone()));
    }
    if let Some(ref n) = request.notes {
        updates.push(format!("notes = ?{}", values.len() + 1));
        values.push(Box::new(n.clone()));
    }

    if updates.is_empty() {
        return Err("No fields to update".to_string());
    }

    updates.push("updated_at = datetime('now')".to_string());
    let sql = format!(
        "UPDATE prepared_solutions SET {} WHERE id = ?{}",
        updates.join(", "),
        values.len() + 1
    );
    values.push(Box::new(request.id.clone()));

    let bind_refs: Vec<&dyn rusqlite::types::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    db.conn.execute(&sql, bind_refs.as_slice())
        .map_err(|e| format!("Failed to update prepared solution: {}", e))?;

    queries::log_audit(
        &db.conn, Some(&user.id), "update", "prepared_solution", Some(&request.id),
        None, None, Some("Prepared solution updated"),
    ).ok();

    Ok(())
}

#[tauri::command]
pub fn delete_prepared_solution(
    state: State<AppState>,
    token: String,
    id: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err("Only supervisors and admins can delete prepared solutions".to_string());
    }

    db.conn.execute("DELETE FROM prepared_solutions WHERE id = ?1", params![id])
        .map_err(|e| format!("Failed to delete prepared solution: {}", e))?;

    queries::log_audit(
        &db.conn, Some(&user.id), "delete", "prepared_solution", Some(&id),
        None, None, Some("Prepared solution deleted"),
    ).ok();

    Ok(())
}

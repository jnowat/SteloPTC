use crate::auth as auth_service;
use crate::db::queries;
use crate::models::inventory::*;
use crate::AppState;
use rusqlite::params;
use tauri::State;

#[tauri::command]
pub fn list_inventory(state: State<AppState>, token: String) -> Result<Vec<InventoryItem>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;

    let mut stmt = db
        .conn
        .prepare(
            "SELECT id, name, category, unit, current_stock, minimum_stock,
                    reorder_point, supplier, catalog_number, lot_number,
                    storage_location, expiration_date, cost_per_unit, notes,
                    created_at, updated_at
             FROM inventory_items
             ORDER BY category, name",
        )
        .map_err(|e| e.to_string())?;

    let items = stmt
        .query_map([], |row| {
            Ok(InventoryItem {
                id: row.get(0)?,
                name: row.get(1)?,
                category: row.get(2)?,
                unit: row.get(3)?,
                current_stock: row.get(4)?,
                minimum_stock: row.get(5)?,
                reorder_point: row.get(6)?,
                supplier: row.get(7)?,
                catalog_number: row.get(8)?,
                lot_number: row.get(9)?,
                storage_location: row.get(10)?,
                expiration_date: row.get(11)?,
                cost_per_unit: row.get(12)?,
                notes: row.get(13)?,
                created_at: row.get(14)?,
                updated_at: row.get(15)?,
            })
        })
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

    db.conn
        .execute(
            "INSERT INTO inventory_items (id, name, category, unit, current_stock, minimum_stock,
             reorder_point, supplier, catalog_number, lot_number, storage_location,
             expiration_date, cost_per_unit, notes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                id,
                request.name,
                request.category,
                request.unit,
                current_stock,
                minimum_stock,
                request.reorder_point,
                request.supplier,
                request.catalog_number,
                request.lot_number,
                request.storage_location,
                request.expiration_date,
                request.cost_per_unit,
                request.notes,
            ],
        )
        .map_err(|e| format!("Failed to create inventory item: {}", e))?;

    queries::log_audit(
        &db.conn,
        Some(&user.id),
        "create",
        "inventory_item",
        Some(&id),
        None,
        Some(&request.name),
        Some("Inventory item created"),
    )
    .ok();

    db.conn
        .query_row(
            "SELECT id, name, category, unit, current_stock, minimum_stock,
                    reorder_point, supplier, catalog_number, lot_number,
                    storage_location, expiration_date, cost_per_unit, notes,
                    created_at, updated_at
             FROM inventory_items WHERE id = ?1",
            params![id],
            |row| {
                Ok(InventoryItem {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    category: row.get(2)?,
                    unit: row.get(3)?,
                    current_stock: row.get(4)?,
                    minimum_stock: row.get(5)?,
                    reorder_point: row.get(6)?,
                    supplier: row.get(7)?,
                    catalog_number: row.get(8)?,
                    lot_number: row.get(9)?,
                    storage_location: row.get(10)?,
                    expiration_date: row.get(11)?,
                    cost_per_unit: row.get(12)?,
                    notes: row.get(13)?,
                    created_at: row.get(14)?,
                    updated_at: row.get(15)?,
                })
            },
        )
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

    macro_rules! add_update {
        ($field:ident, $col:expr) => {
            if let Some(ref val) = request.$field {
                updates.push(format!("{} = ?{}", $col, values.len() + 1));
                values.push(Box::new(val.clone()));
            }
        };
    }

    add_update!(name, "name");
    add_update!(category, "category");
    add_update!(unit, "unit");
    add_update!(supplier, "supplier");
    add_update!(catalog_number, "catalog_number");
    add_update!(lot_number, "lot_number");
    add_update!(storage_location, "storage_location");
    add_update!(expiration_date, "expiration_date");
    add_update!(notes, "notes");

    if let Some(val) = request.current_stock {
        updates.push(format!("current_stock = ?{}", values.len() + 1));
        values.push(Box::new(val));
    }
    if let Some(val) = request.minimum_stock {
        updates.push(format!("minimum_stock = ?{}", values.len() + 1));
        values.push(Box::new(val));
    }
    if let Some(val) = request.reorder_point {
        updates.push(format!("reorder_point = ?{}", values.len() + 1));
        values.push(Box::new(val));
    }
    if let Some(val) = request.cost_per_unit {
        updates.push(format!("cost_per_unit = ?{}", values.len() + 1));
        values.push(Box::new(val));
    }

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
        &db.conn,
        Some(&user.id),
        "update",
        "inventory_item",
        Some(&request.id),
        None,
        None,
        Some("Inventory item updated"),
    )
    .ok();

    db.conn
        .query_row(
            "SELECT id, name, category, unit, current_stock, minimum_stock,
                    reorder_point, supplier, catalog_number, lot_number,
                    storage_location, expiration_date, cost_per_unit, notes,
                    created_at, updated_at
             FROM inventory_items WHERE id = ?1",
            params![request.id],
            |row| {
                Ok(InventoryItem {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    category: row.get(2)?,
                    unit: row.get(3)?,
                    current_stock: row.get(4)?,
                    minimum_stock: row.get(5)?,
                    reorder_point: row.get(6)?,
                    supplier: row.get(7)?,
                    catalog_number: row.get(8)?,
                    lot_number: row.get(9)?,
                    storage_location: row.get(10)?,
                    expiration_date: row.get(11)?,
                    cost_per_unit: row.get(12)?,
                    notes: row.get(13)?,
                    created_at: row.get(14)?,
                    updated_at: row.get(15)?,
                })
            },
        )
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
        &db.conn,
        Some(&user.id),
        "delete",
        "inventory_item",
        Some(&id),
        None,
        None,
        Some("Inventory item deleted"),
    )
    .ok();

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
        &db.conn,
        Some(&user.id),
        "update",
        "inventory_item",
        Some(&id),
        Some(&current.to_string()),
        Some(&new_stock.to_string()),
        Some(&detail),
    )
    .ok();

    db.conn
        .query_row(
            "SELECT id, name, category, unit, current_stock, minimum_stock,
                    reorder_point, supplier, catalog_number, lot_number,
                    storage_location, expiration_date, cost_per_unit, notes,
                    created_at, updated_at
             FROM inventory_items WHERE id = ?1",
            params![id],
            |row| {
                Ok(InventoryItem {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    category: row.get(2)?,
                    unit: row.get(3)?,
                    current_stock: row.get(4)?,
                    minimum_stock: row.get(5)?,
                    reorder_point: row.get(6)?,
                    supplier: row.get(7)?,
                    catalog_number: row.get(8)?,
                    lot_number: row.get(9)?,
                    storage_location: row.get(10)?,
                    expiration_date: row.get(11)?,
                    cost_per_unit: row.get(12)?,
                    notes: row.get(13)?,
                    created_at: row.get(14)?,
                    updated_at: row.get(15)?,
                })
            },
        )
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
            "SELECT id, name, category, current_stock, minimum_stock, reorder_point, unit
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
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(alerts)
}

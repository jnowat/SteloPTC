// WP-57: Interactive lab map — location CRUD + map data feed.
use rusqlite::params;
use tauri::State;

use crate::auth as auth_service;
use crate::models::location::{CreateLocationRequest, Location, LocationMapPoint, UpdateLocationRequest};
use crate::AppState;

fn row_to_location(row: &rusqlite::Row) -> rusqlite::Result<Location> {
    Ok(Location {
        id: row.get("id")?,
        name: row.get("name")?,
        description: row.get("description")?,
        floor_plan_image: row.get("floor_plan_image")?,
        floor_plan_x: row.get("floor_plan_x")?,
        floor_plan_y: row.get("floor_plan_y")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

#[tauri::command]
pub fn list_locations(state: State<AppState>, token: String) -> Result<Vec<Location>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    let mut stmt = db
        .conn
        .prepare("SELECT * FROM locations ORDER BY name ASC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], row_to_location)
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

#[tauri::command]
pub fn get_location(state: State<AppState>, token: String, id: String) -> Result<Location, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    db.conn
        .query_row("SELECT * FROM locations WHERE id = ?1", [id], row_to_location)
        .map_err(|e| format!("Location not found: {}", e))
}

#[tauri::command]
pub fn create_location(
    state: State<AppState>,
    token: String,
    request: CreateLocationRequest,
) -> Result<Location, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }
    if request.name.trim().is_empty() {
        return Err("Location name is required".to_string());
    }

    let id = uuid::Uuid::new_v4().to_string();
    db.conn
        .execute(
            "INSERT INTO locations (id, name, description, floor_plan_image, floor_plan_x, floor_plan_y) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id, request.name, request.description, request.floor_plan_image,
                request.floor_plan_x, request.floor_plan_y,
            ],
        )
        .map_err(|e| format!("Failed to create location: {}", e))?;

    crate::db::queries::log_audit(
        &db.conn, Some(&user.id), "create", "location", Some(&id),
        None, Some(&request.name), Some("Location created"),
    ).ok();

    db.conn
        .query_row("SELECT * FROM locations WHERE id = ?1", [&id], row_to_location)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_location(
    state: State<AppState>,
    token: String,
    request: UpdateLocationRequest,
) -> Result<Location, String> {
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
    add_update!(description, "description");
    add_update!(floor_plan_image, "floor_plan_image");
    add_update!(floor_plan_x, "floor_plan_x");
    add_update!(floor_plan_y, "floor_plan_y");

    if updates.is_empty() {
        return Err("No fields to update".to_string());
    }
    updates.push("updated_at = datetime('now')".to_string());
    let sql = format!("UPDATE locations SET {} WHERE id = ?{}", updates.join(", "), values.len() + 1);
    values.push(Box::new(request.id.clone()));
    let bind_refs: Vec<&dyn rusqlite::types::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    db.conn.execute(&sql, bind_refs.as_slice()).map_err(|e| format!("Failed to update location: {}", e))?;

    crate::db::queries::log_audit(
        &db.conn, Some(&user.id), "update", "location", Some(&request.id),
        None, None, Some("Location updated"),
    ).ok();

    db.conn
        .query_row("SELECT * FROM locations WHERE id = ?1", [&request.id], row_to_location)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_location(state: State<AppState>, token: String, id: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err("Only supervisors and admins can delete locations".to_string());
    }

    let pinned_count: i64 = db
        .conn
        .query_row("SELECT COUNT(*) FROM specimens WHERE location_id = ?1", [&id], |r| r.get(0))
        .unwrap_or(0);
    if pinned_count > 0 {
        return Err(format!(
            "Cannot delete: {} specimen(s) are still pinned to this location. Unpin them first.",
            pinned_count
        ));
    }

    db.conn.execute("DELETE FROM locations WHERE id = ?1", [&id]).map_err(|e| e.to_string())?;

    crate::db::queries::log_audit(
        &db.conn, Some(&user.id), "delete", "location", Some(&id),
        None, None, Some("Location deleted"),
    ).ok();

    Ok(())
}

/// Assigns (or clears, when `location_id` is `None`) a specimen's map pin.
/// Deliberately separate from `update_specimen` — this only ever touches the
/// new `location_id` column, never the existing free-text `location` /
/// `location_details` fields, so the text-based location system is
/// untouched by the map feature.
#[tauri::command]
pub fn set_specimen_location_pin(
    state: State<AppState>,
    token: String,
    specimen_id: String,
    location_id: Option<String>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }
    db.conn
        .execute(
            "UPDATE specimens SET location_id = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![location_id, specimen_id],
        )
        .map_err(|e| format!("Failed to set location pin: {}", e))?;
    Ok(())
}

/// Data feed for `LabMap.svelte` and the Dashboard map widget: every
/// location's pin position plus specimen density / contamination / age
/// aggregates, computed server-side so the client never has to fetch every
/// specimen just to render a heat-map.
#[tauri::command]
pub fn get_location_map_data(state: State<AppState>, token: String) -> Result<Vec<LocationMapPoint>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    let mut stmt = db
        .conn
        .prepare(
            "SELECT l.id, l.name, l.floor_plan_x, l.floor_plan_y, \
                    COUNT(sp.id) AS specimen_count, \
                    SUM(CASE WHEN sp.quarantine_flag = 1 \
                              OR EXISTS (SELECT 1 FROM subcultures sc \
                                         WHERE sc.specimen_id = sp.id AND sc.contamination_flag = 1) \
                             THEN 1 ELSE 0 END) AS contaminated_count, \
                    AVG(julianday('now') - julianday(sp.initiation_date)) AS avg_age_days \
             FROM locations l \
             LEFT JOIN specimens sp ON sp.location_id = l.id AND sp.is_archived = 0 \
             GROUP BY l.id \
             ORDER BY l.name ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(LocationMapPoint {
                location_id: r.get(0)?,
                name: r.get(1)?,
                floor_plan_x: r.get(2)?,
                floor_plan_y: r.get(3)?,
                specimen_count: r.get(4)?,
                contaminated_count: r.get::<_, Option<i64>>(5)?.unwrap_or(0),
                avg_age_days: r.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory DB");
        conn.execute_batch(
            "CREATE TABLE locations (
                id TEXT PRIMARY KEY, name TEXT NOT NULL UNIQUE, description TEXT,
                floor_plan_image TEXT, floor_plan_x REAL, floor_plan_y REAL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE specimens (
                id TEXT PRIMARY KEY, accession_number TEXT NOT NULL UNIQUE,
                location_id TEXT, is_archived INTEGER NOT NULL DEFAULT 0,
                quarantine_flag INTEGER NOT NULL DEFAULT 0, disease_status TEXT,
                initiation_date TEXT NOT NULL DEFAULT '2026-01-01'
            );",
        )
        .expect("create tables");
        conn
    }

    #[test]
    fn location_with_zero_specimens_is_safe_to_delete() {
        let conn = setup_db();
        conn.execute(
            "INSERT INTO locations (id, name) VALUES ('l1', 'Room A')",
            [],
        )
        .unwrap();
        let pinned_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM specimens WHERE location_id = 'l1'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(pinned_count, 0);
    }

    #[test]
    fn location_with_pinned_specimens_blocks_delete() {
        let conn = setup_db();
        conn.execute("INSERT INTO locations (id, name) VALUES ('l1', 'Room A')", []).unwrap();
        conn.execute(
            "INSERT INTO specimens (id, accession_number, location_id) VALUES ('s1', 'ACC-001', 'l1')",
            [],
        )
        .unwrap();
        let pinned_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM specimens WHERE location_id = 'l1'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(pinned_count, 1, "delete_location must refuse when this is > 0");
    }
}

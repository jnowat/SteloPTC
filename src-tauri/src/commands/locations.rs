// WP-57: Interactive lab map — location CRUD + map data feed.
use rusqlite::params;
use tauri::State;

use crate::auth as auth_service;
use crate::models::location::{
    CreateLocationRequest, Location, LocationMapPoint, LocationOccupancy, UpdateLocationRequest,
};
use crate::AppState;

fn row_to_location(row: &rusqlite::Row) -> rusqlite::Result<Location> {
    Ok(Location {
        id: row.get("id")?,
        name: row.get("name")?,
        description: row.get("description")?,
        floor_plan_image: row.get("floor_plan_image")?,
        floor_plan_x: row.get("floor_plan_x")?,
        floor_plan_y: row.get("floor_plan_y")?,
        layout_json: row.get("layout_json")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

/// Upper bound on a stored floor plan, in bytes.
///
/// The layout is written on every edit, so an unbounded column is an unbounded
/// write amplification on the single global DB mutex. 512 KiB is roughly two
/// orders of magnitude above a fully furnished room (the editor caps the grid at
/// 60×60 and furniture at 30 shelves), so it can only be hit by a client
/// sending something it should not.
const MAX_LAYOUT_BYTES: usize = 512 * 1024;

#[tauri::command]
pub fn list_locations(state: State<AppState>, token: String) -> Result<Vec<Location>, String> {
    let db = state.db();
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
    let db = state.db();
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
    let db = state.db();
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
    let db = state.db();
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
    let db = state.db();
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
    let db = state.db();
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }
    crate::db::vocabulary::require_active_lab_profile(&db.conn, &specimen_id)?;
    db.conn
        .execute(
            "UPDATE specimens SET location_id = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![location_id, specimen_id],
        )
        .map_err(|e| format!("Failed to set location pin: {}", e))?;
    Ok(())
}

/// Save the drawn floor plan for a location.
///
/// Kept separate from `update_location` for the same reason
/// `set_specimen_location_pin` is: it touches exactly one column, so an autosave
/// from the layout editor can never race a name or image edit and clobber it.
/// Passing `None` clears the plan.
#[tauri::command]
pub fn save_location_layout(
    state: State<AppState>,
    token: String,
    location_id: String,
    layout_json: Option<String>,
) -> Result<(), String> {
    let db = state.db();
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }

    if let Some(ref json) = layout_json {
        if json.len() > MAX_LAYOUT_BYTES {
            return Err(format!(
                "Floor plan is too large ({} KB). The limit is {} KB.",
                json.len() / 1024,
                MAX_LAYOUT_BYTES / 1024
            ));
        }
        // Reject anything that is not JSON here rather than at read time: a bad
        // blob written now is a Lab Map that fails to open later, with nothing
        // pointing at when it broke.
        serde_json::from_str::<serde_json::Value>(json)
            .map_err(|e| format!("Floor plan is not valid JSON: {}", e))?;
    }

    let changed = db
        .conn
        .execute(
            "UPDATE locations SET layout_json = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![layout_json, location_id],
        )
        .map_err(|e| format!("Failed to save floor plan: {}", e))?;
    if changed == 0 {
        return Err("Location not found".to_string());
    }

    crate::db::queries::log_audit(
        &db.conn, Some(&user.id), "update", "location", Some(&location_id),
        None, None,
        Some(if layout_json.is_some() { "Floor plan saved" } else { "Floor plan cleared" }),
    ).ok();

    Ok(())
}

/// Specimen counts per recorded location path, for shading the drawn plan.
///
/// Groups the free-text `specimens.location` strings the Add Specimen form
/// composes — which is what the layout editor generates addresses in — so the
/// client can colour furniture by how full it is without pulling every
/// specimen. Archived specimens are excluded: a rack full of archived cultures
/// is an empty rack.
#[tauri::command]
pub fn get_location_occupancy(
    state: State<AppState>,
    token: String,
) -> Result<Vec<LocationOccupancy>, String> {
    let db = state.db();
    let _user = auth_service::validate_session(&db, &token)?;
    let mut stmt = db
        .conn
        .prepare(
            "SELECT sp.location AS location, \
                    COUNT(sp.id) AS specimen_count, \
                    SUM(CASE WHEN sp.quarantine_flag = 1 \
                              OR EXISTS (SELECT 1 FROM subcultures sc \
                                         WHERE sc.specimen_id = sp.id AND sc.contamination_flag = 1) \
                             THEN 1 ELSE 0 END) AS contaminated_count \
             FROM specimens sp \
             WHERE sp.is_archived = 0 AND sp.location IS NOT NULL AND TRIM(sp.location) != '' \
             GROUP BY sp.location",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(LocationOccupancy {
                location: r.get(0)?,
                specimen_count: r.get(1)?,
                contaminated_count: r.get::<_, Option<i64>>(2)?.unwrap_or(0),
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

/// Data feed for `LabMap.svelte` and the Dashboard map widget: every
/// location's pin position plus specimen density / contamination / age
/// aggregates, computed server-side so the client never has to fetch every
/// specimen just to render a heat-map.
#[tauri::command]
pub fn get_location_map_data(state: State<AppState>, token: String) -> Result<Vec<LocationMapPoint>, String> {
    let db = state.db();
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
                layout_json TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE specimens (
                id TEXT PRIMARY KEY, accession_number TEXT NOT NULL UNIQUE,
                location_id TEXT, location TEXT, is_archived INTEGER NOT NULL DEFAULT 0,
                quarantine_flag INTEGER NOT NULL DEFAULT 0, disease_status TEXT,
                initiation_date TEXT NOT NULL DEFAULT '2026-01-01'
            );
            CREATE TABLE subcultures (
                id TEXT PRIMARY KEY, specimen_id TEXT NOT NULL,
                contamination_flag INTEGER NOT NULL DEFAULT 0
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

    // ── Layout persistence and occupancy ──────────────────────────────────

    #[test]
    fn layout_json_starts_null_and_round_trips() {
        let conn = setup_db();
        conn.execute("INSERT INTO locations (id, name) VALUES ('l1', 'Growth Room B')", []).unwrap();

        let before: Option<String> = conn
            .query_row("SELECT layout_json FROM locations WHERE id = 'l1'", [], |r| r.get(0))
            .unwrap();
        assert!(before.is_none(), "a location with no drawn plan must read as NULL");

        let plan = r#"{"version":1,"gridCols":20,"gridRows":14,"items":[]}"#;
        conn.execute("UPDATE locations SET layout_json = ?1 WHERE id = 'l1'", [plan]).unwrap();
        let after: Option<String> = conn
            .query_row("SELECT layout_json FROM locations WHERE id = 'l1'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(after.as_deref(), Some(plan));
    }

    /// Mirrors the SQL in `get_location_occupancy` so the grouping and the
    /// archived/contaminated rules are covered without a Tauri State.
    fn occupancy(conn: &Connection) -> Vec<(String, i64, i64)> {
        let mut stmt = conn
            .prepare(
                "SELECT sp.location AS location, \
                        COUNT(sp.id) AS specimen_count, \
                        SUM(CASE WHEN sp.quarantine_flag = 1 \
                                  OR EXISTS (SELECT 1 FROM subcultures sc \
                                             WHERE sc.specimen_id = sp.id AND sc.contamination_flag = 1) \
                                 THEN 1 ELSE 0 END) AS contaminated_count \
                 FROM specimens sp \
                 WHERE sp.is_archived = 0 AND sp.location IS NOT NULL AND TRIM(sp.location) != '' \
                 GROUP BY sp.location",
            )
            .unwrap();
        let rows = stmt
            .query_map([], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get::<_, Option<i64>>(2)?.unwrap_or(0)))
            })
            .unwrap();
        let mut out: Vec<(String, i64, i64)> = rows.filter_map(|r| r.ok()).collect();
        out.sort();
        out
    }

    fn insert_specimen(conn: &Connection, id: &str, location: Option<&str>, archived: i64, quarantine: i64) {
        conn.execute(
            "INSERT INTO specimens (id, accession_number, location, is_archived, quarantine_flag)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![id, format!("ACC-{id}"), location, archived, quarantine],
        )
        .unwrap();
    }

    #[test]
    fn occupancy_groups_specimens_by_their_location_path() {
        let conn = setup_db();
        insert_specimen(&conn, "s1", Some("Room 1 / Rack A / Shelf 1 / A1"), 0, 0);
        insert_specimen(&conn, "s2", Some("Room 1 / Rack A / Shelf 1 / A1"), 0, 0);
        insert_specimen(&conn, "s3", Some("Room 1 / Rack A / Shelf 2 / B1"), 0, 0);

        let rows = occupancy(&conn);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], ("Room 1 / Rack A / Shelf 1 / A1".to_string(), 2, 0));
        assert_eq!(rows[1], ("Room 1 / Rack A / Shelf 2 / B1".to_string(), 1, 0));
    }

    #[test]
    fn occupancy_ignores_archived_specimens() {
        // A rack full of archived cultures is an empty rack; shading it as full
        // would send someone to a shelf that has space on it.
        let conn = setup_db();
        insert_specimen(&conn, "s1", Some("Room 1 / Rack A / Shelf 1 / A1"), 1, 0);
        assert!(occupancy(&conn).is_empty());
    }

    #[test]
    fn occupancy_skips_specimens_with_no_recorded_location() {
        let conn = setup_db();
        insert_specimen(&conn, "s1", None, 0, 0);
        insert_specimen(&conn, "s2", Some("   "), 0, 0);
        assert!(occupancy(&conn).is_empty());
    }

    #[test]
    fn occupancy_counts_quarantined_and_contaminated_specimens() {
        let conn = setup_db();
        insert_specimen(&conn, "s1", Some("Room 1 / Rack A / Shelf 1 / A1"), 0, 1);
        insert_specimen(&conn, "s2", Some("Room 1 / Rack A / Shelf 1 / A1"), 0, 0);
        conn.execute(
            "INSERT INTO subcultures (id, specimen_id, contamination_flag) VALUES ('sc1', 's2', 1)",
            [],
        )
        .unwrap();

        let rows = occupancy(&conn);
        assert_eq!(rows[0].1, 2, "both specimens occupy the slot");
        assert_eq!(rows[0].2, 2, "one quarantined plus one with a contaminated subculture");
    }
}

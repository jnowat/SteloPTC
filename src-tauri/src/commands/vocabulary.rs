use crate::auth as auth_service;
use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct VocabEntry {
    pub id: i64,
    pub code: String,
    pub label: String,
    pub sort_order: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StageEntry {
    pub id: i64,
    pub code: String,
    pub label: String,
    pub sort_order: i64,
    pub is_terminal: bool,
}

fn query_vocab(
    conn: &rusqlite::Connection,
    sql: &str,
    profile: &str,
) -> Result<Vec<VocabEntry>, String> {
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    stmt.query_map([profile], |row| {
        Ok(VocabEntry {
            id: row.get(0)?,
            code: row.get(1)?,
            label: row.get(2)?,
            sort_order: row.get(3)?,
        })
    })
    .map_err(|e| e.to_string())?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_stages(
    state: State<AppState>,
    token: String,
) -> Result<Vec<StageEntry>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    let profile = crate::db::vocabulary::active_profile(&db.conn);

    let mut stmt = db
        .conn
        .prepare(
            "SELECT id, code, label, sort_order, is_terminal \
             FROM stages WHERE profile = ?1 ORDER BY sort_order",
        )
        .map_err(|e| e.to_string())?;

    stmt.query_map([&profile], |row| {
        Ok(StageEntry {
            id: row.get(0)?,
            code: row.get(1)?,
            label: row.get(2)?,
            sort_order: row.get(3)?,
            is_terminal: row.get::<_, i64>(4)? != 0,
        })
    })
    .map_err(|e| e.to_string())?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_propagation_methods(
    state: State<AppState>,
    token: String,
) -> Result<Vec<VocabEntry>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    let profile = crate::db::vocabulary::active_profile(&db.conn);
    query_vocab(
        &db.conn,
        "SELECT id, code, label, sort_order \
         FROM propagation_methods WHERE profile = ?1 ORDER BY sort_order",
        &profile,
    )
}

#[tauri::command]
pub fn list_hormone_types(
    state: State<AppState>,
    token: String,
) -> Result<Vec<VocabEntry>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    let profile = crate::db::vocabulary::active_profile(&db.conn);
    query_vocab(
        &db.conn,
        "SELECT id, code, label, sort_order \
         FROM hormone_types WHERE profile = ?1 ORDER BY sort_order",
        &profile,
    )
}

#[tauri::command]
pub fn list_compliance_record_types(
    state: State<AppState>,
    token: String,
) -> Result<Vec<VocabEntry>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    let profile = crate::db::vocabulary::active_profile(&db.conn);
    query_vocab(
        &db.conn,
        "SELECT id, code, label, sort_order \
         FROM compliance_record_types WHERE profile = ?1 ORDER BY sort_order",
        &profile,
    )
}

#[tauri::command]
pub fn list_compliance_agencies(
    state: State<AppState>,
    token: String,
) -> Result<Vec<VocabEntry>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    let profile = crate::db::vocabulary::active_profile(&db.conn);
    query_vocab(
        &db.conn,
        "SELECT id, code, label, sort_order \
         FROM compliance_agencies WHERE profile = ?1 ORDER BY sort_order",
        &profile,
    )
}

#[tauri::command]
pub fn list_inventory_categories(
    state: State<AppState>,
    token: String,
) -> Result<Vec<VocabEntry>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    let profile = crate::db::vocabulary::active_profile(&db.conn);
    query_vocab(
        &db.conn,
        "SELECT id, code, label, sort_order \
         FROM inventory_categories WHERE profile = ?1 ORDER BY sort_order",
        &profile,
    )
}


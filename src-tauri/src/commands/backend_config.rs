//! WP-50 — Tauri command surface for the backend selection foundation.
//!
//! `set_backend_type` records the lab's intended backend; it does not
//! reconnect the live app to a different database. `test_postgres_connection`
//! and `bootstrap_postgres_schema` exercise the standalone PostgreSQL
//! connector in `db::postgres` and never persist the supplied connection
//! string (see migration_035's doc comment for the rationale).

use crate::auth as auth_service;
use crate::db::backend::{self, BackendKind};
use crate::db::postgres;
use crate::models::backend::BackendConfigInfo;
use crate::AppState;
use tauri::State;

/// Any authenticated user may read the current backend configuration.
#[tauri::command]
pub fn get_backend_config(state: State<AppState>, token: String) -> Result<BackendConfigInfo, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;

    let backend_type = backend::current_backend_kind(&db.conn).as_str().to_string();
    Ok(BackendConfigInfo {
        backend_type,
        postgres_feature_compiled: cfg!(feature = "postgres"),
    })
}

/// Records the lab's intended backend. Admin-only. Does not migrate data or
/// reconnect the running app — SQLite remains the active backend for all
/// reads/writes regardless of this setting until a future full backend
/// switch ships.
#[tauri::command]
pub fn set_backend_type(
    state: State<AppState>,
    token: String,
    backend_type: String,
    connection_string: Option<String>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;

    if !user.role.is_admin() {
        return Err("Only admins can change the backend configuration".to_string());
    }

    let target = BackendKind::parse(&backend_type)?;
    backend::validate_backend_switch(target, cfg!(feature = "postgres"), connection_string.as_deref())?;

    backend::set_backend_kind(&db.conn, target).map_err(|e| e.to_string())?;

    crate::db::queries::log_audit(
        &db.conn,
        Some(&user.id),
        "update",
        "app_settings",
        None,
        None,
        None,
        Some(&format!(
            "Intended database backend set to '{}' (does not yet change the live backend)",
            target.as_str()
        )),
    )
    .ok();

    Ok(())
}

/// Admin-only. Tests connectivity to a PostgreSQL server without persisting
/// the connection string anywhere.
///
/// Kept synchronous (bridging to the async `sqlx` connector via
/// `tauri::async_runtime::block_on`) to match every other command in this
/// codebase — none currently use `async fn` — rather than introduce the
/// first async Tauri command and its distinct `State<'_, T>` lifetime rules.
#[tauri::command]
pub fn test_postgres_connection(
    state: State<AppState>,
    token: String,
    connection_string: String,
) -> Result<String, String> {
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let user = auth_service::validate_session(&db, &token)?;
        if !user.role.is_admin() {
            return Err("Only admins can test a PostgreSQL connection".to_string());
        }
    }
    tauri::async_runtime::block_on(postgres::test_connection(&connection_string))
}

/// Admin-only. Connects and creates the WP-50 foundation schema. Returns the
/// names of the tables created/verified. Never persists the connection string.
#[tauri::command]
pub fn bootstrap_postgres_schema(
    state: State<AppState>,
    token: String,
    connection_string: String,
) -> Result<Vec<String>, String> {
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let user = auth_service::validate_session(&db, &token)?;
        if !user.role.is_admin() {
            return Err("Only admins can bootstrap a PostgreSQL schema".to_string());
        }
    }
    tauri::async_runtime::block_on(postgres::bootstrap_schema(&connection_string))
}

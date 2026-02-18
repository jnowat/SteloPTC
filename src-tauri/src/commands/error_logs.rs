use crate::auth as auth_service;
use crate::models::error_log::*;
use crate::models::specimen::PaginatedResponse;
use crate::db::queries;
use crate::AppState;
use rusqlite::params;
use tauri::State;

#[tauri::command]
pub fn log_error(
    state: State<AppState>,
    token: String,
    request: CreateErrorLogRequest,
) -> Result<ErrorLog, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    // Allow any authenticated user to log errors; also allow unauthenticated
    // (token may be empty on startup errors) by soft-validating.
    let (user_id, username) = match auth_service::validate_session(&db, &token) {
        Ok(u) => (Some(u.id), Some(u.username)),
        Err(_) => (None, None),
    };

    let id = uuid::Uuid::new_v4().to_string();
    let severity = request.severity.as_deref().unwrap_or("error").to_string();

    db.conn.execute(
        "INSERT INTO error_logs (id, title, message, module, severity, user_id, username, form_payload, stack_trace)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            id,
            request.title,
            request.message,
            request.module,
            severity,
            user_id,
            username,
            request.form_payload,
            request.stack_trace,
        ],
    ).map_err(|e| format!("Failed to log error: {}", e))?;

    db.conn.query_row(
        "SELECT * FROM error_logs WHERE id = ?1",
        params![id],
        map_row,
    ).map_err(|e| format!("Failed to retrieve logged error: {}", e))
}

#[tauri::command]
pub fn list_error_logs(
    state: State<AppState>,
    token: String,
    search: ErrorLogSearchParams,
) -> Result<PaginatedResponse<ErrorLog>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;

    let pg = queries::PaginationParams {
        page: search.page.unwrap_or(1),
        per_page: search.per_page.unwrap_or(50),
    };

    let mut conditions: Vec<String> = Vec::new();
    let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(ref sev) = search.severity {
        let idx = bind_values.len() + 1;
        conditions.push(format!("severity = ?{}", idx));
        bind_values.push(Box::new(sev.clone()));
    }
    if let Some(ref module) = search.module {
        let idx = bind_values.len() + 1;
        conditions.push(format!("module LIKE ?{}", idx));
        bind_values.push(Box::new(format!("%{}%", module)));
    }
    if search.unread_only.unwrap_or(false) {
        conditions.push("is_read = 0".to_string());
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let count_sql = format!("SELECT COUNT(*) FROM error_logs {}", where_clause);
    let bind_refs: Vec<&dyn rusqlite::types::ToSql> = bind_values.iter().map(|v| v.as_ref()).collect();
    let total: i64 = db.conn.query_row(&count_sql, bind_refs.as_slice(), |r| r.get(0))
        .map_err(|e| e.to_string())?;

    let query_sql = format!(
        "SELECT * FROM error_logs {} ORDER BY timestamp DESC LIMIT ?{} OFFSET ?{}",
        where_clause,
        bind_values.len() + 1,
        bind_values.len() + 2
    );

    bind_values.push(Box::new(pg.limit()));
    bind_values.push(Box::new(pg.offset()));

    let bind_refs2: Vec<&dyn rusqlite::types::ToSql> = bind_values.iter().map(|v| v.as_ref()).collect();
    let mut stmt = db.conn.prepare(&query_sql).map_err(|e| e.to_string())?;

    let entries = stmt.query_map(bind_refs2.as_slice(), map_row)
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect::<Vec<_>>();

    let total_pages = ((total as f64) / (pg.per_page as f64)).ceil() as u32;

    Ok(PaginatedResponse {
        items: entries,
        total,
        page: pg.page,
        per_page: pg.per_page,
        total_pages,
    })
}

#[tauri::command]
pub fn get_unread_error_count(
    state: State<AppState>,
    token: String,
) -> Result<i64, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;

    db.conn.query_row(
        "SELECT COUNT(*) FROM error_logs WHERE is_read = 0",
        [],
        |r| r.get(0),
    ).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn mark_errors_read(
    state: State<AppState>,
    token: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;

    db.conn.execute(
        "UPDATE error_logs SET is_read = 1 WHERE is_read = 0",
        [],
    ).map_err(|e| format!("Failed to mark errors read: {}", e))?;

    Ok(())
}

#[tauri::command]
pub fn clear_error_logs(
    state: State<AppState>,
    token: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err("Only supervisors and admins can clear error logs".to_string());
    }

    db.conn.execute("DELETE FROM error_logs", [])
        .map_err(|e| format!("Failed to clear error logs: {}", e))?;

    Ok(())
}

fn map_row(row: &rusqlite::Row) -> rusqlite::Result<ErrorLog> {
    Ok(ErrorLog {
        id: row.get("id")?,
        timestamp: row.get("timestamp")?,
        title: row.get("title")?,
        message: row.get("message")?,
        module: row.get("module")?,
        severity: row.get("severity")?,
        user_id: row.get("user_id")?,
        username: row.get("username")?,
        form_payload: row.get("form_payload")?,
        stack_trace: row.get("stack_trace")?,
        is_read: row.get::<_, i32>("is_read")? != 0,
        created_at: row.get("created_at")?,
    })
}

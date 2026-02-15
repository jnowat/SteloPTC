use crate::auth as auth_service;
use crate::models::audit::*;
use crate::models::specimen::PaginatedResponse;
use crate::db::queries;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn get_audit_log(
    state: State<AppState>,
    token: String,
    search: AuditSearchParams,
) -> Result<PaginatedResponse<AuditEntry>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err("Insufficient permissions".to_string());
    }

    let pg = queries::PaginationParams {
        page: search.page.unwrap_or(1),
        per_page: search.per_page.unwrap_or(50),
    };

    let mut conditions = Vec::new();
    let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(ref uid) = search.user_id {
        let idx = bind_values.len() + 1;
        conditions.push(format!("a.user_id = ?{}", idx));
        bind_values.push(Box::new(uid.clone()));
    }
    if let Some(ref et) = search.entity_type {
        let idx = bind_values.len() + 1;
        conditions.push(format!("a.entity_type = ?{}", idx));
        bind_values.push(Box::new(et.clone()));
    }
    if let Some(ref eid) = search.entity_id {
        let idx = bind_values.len() + 1;
        conditions.push(format!("a.entity_id = ?{}", idx));
        bind_values.push(Box::new(eid.clone()));
    }
    if let Some(ref action) = search.action {
        let idx = bind_values.len() + 1;
        conditions.push(format!("a.action = ?{}", idx));
        bind_values.push(Box::new(action.clone()));
    }
    if let Some(ref from) = search.from_date {
        let idx = bind_values.len() + 1;
        conditions.push(format!("a.created_at >= ?{}", idx));
        bind_values.push(Box::new(from.clone()));
    }
    if let Some(ref to) = search.to_date {
        let idx = bind_values.len() + 1;
        conditions.push(format!("a.created_at <= ?{}", idx));
        bind_values.push(Box::new(to.clone()));
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let count_sql = format!("SELECT COUNT(*) FROM audit_log a {}", where_clause);
    let bind_refs: Vec<&dyn rusqlite::types::ToSql> = bind_values.iter().map(|v| v.as_ref()).collect();
    let total: i64 = db.conn.query_row(&count_sql, bind_refs.as_slice(), |r| r.get(0))
        .map_err(|e| e.to_string())?;

    let query_sql = format!(
        "SELECT a.*, u.username
         FROM audit_log a
         LEFT JOIN users u ON a.user_id = u.id
         {}
         ORDER BY a.created_at DESC
         LIMIT ?{} OFFSET ?{}",
        where_clause,
        bind_values.len() + 1,
        bind_values.len() + 2
    );

    bind_values.push(Box::new(pg.limit()));
    bind_values.push(Box::new(pg.offset()));

    let bind_refs2: Vec<&dyn rusqlite::types::ToSql> = bind_values.iter().map(|v| v.as_ref()).collect();
    let mut stmt = db.conn.prepare(&query_sql).map_err(|e| e.to_string())?;

    let entries = stmt.query_map(bind_refs2.as_slice(), |row| {
        Ok(AuditEntry {
            id: row.get("id")?,
            user_id: row.get("user_id")?,
            username: row.get("username")?,
            action: row.get("action")?,
            entity_type: row.get("entity_type")?,
            entity_id: row.get("entity_id")?,
            old_value: row.get("old_value")?,
            new_value: row.get("new_value")?,
            details: row.get("details")?,
            created_at: row.get("created_at")?,
        })
    }).map_err(|e| e.to_string())?
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

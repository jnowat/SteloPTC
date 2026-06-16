use crate::auth as auth_service;
use crate::models::audit::*;
use crate::models::specimen::PaginatedResponse;
use crate::db::queries::{self, audit_canonical_bytes, compute_entry_hash, ZERO_HASH};
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
            lineage_id: row.get("lineage_id")?,
            chain_seq: row.get("chain_seq")?,
            prev_hash: row.get("prev_hash")?,
            entry_hash: row.get("entry_hash")?,
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

/// Verify a single audit entry by recomputing its hash from stored fields.
/// Returns ok=true if the stored entry_hash matches the recomputed value.
#[tauri::command]
pub fn verify_audit_entry(
    state: State<AppState>,
    token: String,
    entry_id: String,
) -> Result<VerifyEntryResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    auth_service::validate_session(&db, &token)?;

    let row: Option<(String, Option<String>, String, String, Option<String>, String, Option<String>, i64, String, String)> =
        db.conn.query_row(
            "SELECT lineage_id, user_id, entity_type, action, entity_id, created_at, details, \
                    chain_seq, prev_hash, entry_hash \
             FROM audit_log WHERE id = ?1",
            rusqlite::params![entry_id],
            |r| Ok((
                r.get(0)?,  // lineage_id
                r.get(1)?,  // user_id
                r.get(2)?,  // entity_type
                r.get(3)?,  // action
                r.get(4)?,  // entity_id
                r.get(5)?,  // created_at
                r.get(6)?,  // details
                r.get(7)?,  // chain_seq
                r.get(8)?,  // prev_hash
                r.get(9)?,  // entry_hash
            )),
        ).ok();

    let Some((lineage_id, user_id, entity_type, action, entity_id, created_at, details, chain_seq, prev_hash, stored_hash)) = row else {
        return Ok(VerifyEntryResult {
            entry_id,
            ok: false,
            message: "Entry not found.".to_string(),
            stored_hash: None,
            computed_hash: None,
        });
    };

    let canonical = audit_canonical_bytes(
        &lineage_id,
        chain_seq,
        &created_at,
        user_id.as_deref().unwrap_or(""),
        &entity_type,
        entity_id.as_deref().unwrap_or(""),
        &action,
        details.as_deref().unwrap_or(""),
    );
    let computed = compute_entry_hash(&canonical, &prev_hash);
    let ok = computed == stored_hash;

    Ok(VerifyEntryResult {
        entry_id,
        ok,
        message: if ok {
            "Hash matches — this record has not been tampered with.".to_string()
        } else {
            "Hash mismatch — this record may have been tampered with!".to_string()
        },
        stored_hash: Some(stored_hash),
        computed_hash: Some(computed),
    })
}

/// Verify the full hash chain for a given lineage (entity).
///
/// Checks two things for each consecutive pair of chained rows:
///   1. The stored entry_hash matches the recomputed hash.
///   2. The row's prev_hash matches the previous row's entry_hash.
///
/// Reports the chain_seq of the first break detected, if any.
#[tauri::command]
pub fn verify_audit_lineage(
    state: State<AppState>,
    token: String,
    lineage_id: String,
) -> Result<VerifyChainResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    auth_service::validate_session(&db, &token)?;

    struct ChainRow {
        chain_seq: i64,
        user_id: Option<String>,
        entity_type: String,
        action: String,
        entity_id: Option<String>,
        created_at: String,
        details: Option<String>,
        prev_hash: String,
        entry_hash: String,
    }

    let mut stmt = db.conn.prepare(
        "SELECT chain_seq, user_id, entity_type, action, entity_id, created_at, details, prev_hash, entry_hash \
         FROM audit_log \
         WHERE lineage_id = ?1 AND entry_hash IS NOT NULL \
         ORDER BY chain_seq ASC",
    ).map_err(|e| e.to_string())?;

    let rows: Vec<ChainRow> = stmt.query_map(rusqlite::params![lineage_id], |r| {
        Ok(ChainRow {
            chain_seq: r.get(0)?,
            user_id: r.get(1)?,
            entity_type: r.get(2)?,
            action: r.get(3)?,
            entity_id: r.get(4)?,
            created_at: r.get(5)?,
            details: r.get(6)?,
            prev_hash: r.get(7)?,
            entry_hash: r.get(8)?,
        })
    }).map_err(|e| e.to_string())?
      .filter_map(|r| r.ok())
      .collect();

    if rows.is_empty() {
        return Ok(VerifyChainResult {
            lineage_id,
            ok: true,
            checked: 0,
            first_break_seq: None,
            message: "No chained entries found for this lineage.".to_string(),
        });
    }

    let mut prev_entry_hash = ZERO_HASH.to_string();
    for row in &rows {
        // Verify the link: this row's prev_hash must equal the previous row's entry_hash.
        if row.prev_hash != prev_entry_hash {
            return Ok(VerifyChainResult {
                lineage_id,
                ok: false,
                checked: (row.chain_seq - 1) as usize,
                first_break_seq: Some(row.chain_seq),
                message: format!(
                    "Chain broken at seq {} — prev_hash does not match the preceding entry's hash.",
                    row.chain_seq
                ),
            });
        }

        // Verify the hash of this row's content.
        let canonical = audit_canonical_bytes(
            &lineage_id,
            row.chain_seq,
            &row.created_at,
            row.user_id.as_deref().unwrap_or(""),
            &row.entity_type,
            row.entity_id.as_deref().unwrap_or(""),
            &row.action,
            row.details.as_deref().unwrap_or(""),
        );
        let computed = compute_entry_hash(&canonical, &row.prev_hash);
        if computed != row.entry_hash {
            return Ok(VerifyChainResult {
                lineage_id,
                ok: false,
                checked: (row.chain_seq - 1) as usize,
                first_break_seq: Some(row.chain_seq),
                message: format!(
                    "Tamper detected at seq {} — stored hash does not match recomputed hash.",
                    row.chain_seq
                ),
            });
        }

        prev_entry_hash = row.entry_hash.clone();
    }

    let checked = rows.len();
    Ok(VerifyChainResult {
        lineage_id,
        ok: true,
        checked,
        first_break_seq: None,
        message: format!(
            "All {} entries verified — chain is intact.",
            checked
        ),
    })
}

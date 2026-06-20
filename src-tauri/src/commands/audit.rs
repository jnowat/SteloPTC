use crate::auth as auth_service;
use crate::models::audit::*;
use crate::models::specimen::PaginatedResponse;
use crate::db::queries::{self, audit_canonical_bytes, compute_entry_hash, build_merkle_root};
use crate::AppState;
use tauri::State;

// Fields returned by the single-entry audit verification query.
type AuditEntryRow = (
    Option<String>, Option<String>, String, String, Option<String>,
    String, Option<String>, Option<i64>, Option<String>, Option<String>,
);

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
///
/// Returns ok=false with a clear message for legacy rows that predate the hash chain.
#[tauri::command]
pub fn verify_audit_entry(
    state: State<AppState>,
    token: String,
    entry_id: String,
) -> Result<VerifyEntryResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    auth_service::validate_session(&db, &token)?;

    // Use Option<> for every nullable column so rusqlite never errors on NULL.
    // chain_seq, prev_hash, entry_hash, and lineage_id are all nullable for
    // legacy (pre-v1.5.0) rows.
    // columns: lineage_id, user_id, entity_type, action, entity_id,
    //          created_at, details, chain_seq, prev_hash, entry_hash
    let row: Option<AuditEntryRow> = db.conn.query_row(
        "SELECT lineage_id, user_id, entity_type, action, entity_id, created_at, details, \
                chain_seq, prev_hash, entry_hash \
         FROM audit_log WHERE id = ?1",
        rusqlite::params![entry_id],
        |r| Ok((
            r.get(0)?,
            r.get(1)?,
            r.get(2)?,
            r.get(3)?,
            r.get(4)?,
            r.get(5)?,
            r.get(6)?,
            r.get(7)?,
            r.get(8)?,
            r.get(9)?,
        )),
    ).ok();

    let Some((lineage_id_opt, user_id, entity_type, action, entity_id, created_at, details,
              chain_seq_opt, prev_hash_opt, stored_hash_opt)) = row
    else {
        return Ok(VerifyEntryResult {
            entry_id,
            ok: false,
            message: "Entry not found.".to_string(),
            stored_hash: None,
            computed_hash: None,
        });
    };

    // Guard: row exists but has no chain data (written before v1.5.0).
    let (Some(lineage_id), Some(chain_seq), Some(prev_hash), Some(stored_hash)) =
        (lineage_id_opt, chain_seq_opt, prev_hash_opt, stored_hash_opt)
    else {
        return Ok(VerifyEntryResult {
            entry_id,
            ok: false,
            message: "This entry has no chain data (written before the hash chain was introduced in v1.5.0).".to_string(),
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

    // For root lineages the first row's prev_hash is ZERO_HASH; for fork lineages
    // (split specimens) it is the parent's last entry_hash. Either way, we anchor
    // verification at the first row's own claimed prev_hash — we cannot independently
    // verify the anchor without walking the parent lineage, but we verify every
    // subsequent link and every entry_hash within this lineage.
    //
    // Previous (buggy) code used ZERO_HASH as the fixed anchor, which always
    // reported "Chain broken at seq 1" for any forked lineage.
    let mut prev_entry_hash = rows[0].prev_hash.clone();
    for row in &rows {
        // Verify the link: this row's prev_hash must equal the previous row's entry_hash
        // (or the anchor for the very first row, which trivially passes).
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

/// Create a Merkle checkpoint over a contiguous seq range of one lineage's audit chain.
///
/// If start_seq or end_seq are omitted they default to the minimum/maximum chain_seq
/// present in the lineage. The Merkle root is built over the `entry_hash` column values
/// in chain_seq order using the "duplicate-last" binary tree rule.
#[tauri::command]
pub fn create_audit_checkpoint(
    state: State<AppState>,
    token: String,
    lineage_id: String,
    start_seq: Option<i64>,
    end_seq: Option<i64>,
) -> Result<CreateCheckpointResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err("Insufficient permissions — admin or supervisor role required.".to_string());
    }

    let actual_start: i64 = if let Some(s) = start_seq {
        s
    } else {
        db.conn.query_row(
            "SELECT MIN(chain_seq) FROM audit_log WHERE lineage_id = ?1 AND entry_hash IS NOT NULL",
            rusqlite::params![&lineage_id],
            |r| r.get::<_, Option<i64>>(0),
        ).map_err(|e| e.to_string())?
        .ok_or_else(|| format!("No chained entries found in lineage '{}'.", lineage_id))?
    };

    let actual_end: i64 = if let Some(e) = end_seq {
        e
    } else {
        db.conn.query_row(
            "SELECT MAX(chain_seq) FROM audit_log WHERE lineage_id = ?1 AND entry_hash IS NOT NULL",
            rusqlite::params![&lineage_id],
            |r| r.get::<_, Option<i64>>(0),
        ).map_err(|e| e.to_string())?
        .ok_or_else(|| format!("No chained entries found in lineage '{}'.", lineage_id))?
    };

    if actual_start > actual_end {
        return Err(format!(
            "start_seq ({}) must be ≤ end_seq ({}).", actual_start, actual_end
        ));
    }

    let mut stmt = db.conn.prepare(
        "SELECT entry_hash FROM audit_log \
         WHERE lineage_id = ?1 AND chain_seq >= ?2 AND chain_seq <= ?3 AND entry_hash IS NOT NULL \
         ORDER BY chain_seq ASC",
    ).map_err(|e| e.to_string())?;

    let hashes: Vec<String> = stmt
        .query_map(rusqlite::params![&lineage_id, actual_start, actual_end], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    if hashes.is_empty() {
        return Err(format!(
            "No chained entries found in lineage '{}' for seq range [{}, {}].",
            lineage_id, actual_start, actual_end
        ));
    }

    let entry_count = hashes.len() as i64;
    let merkle_root = build_merkle_root(&hashes);
    let checkpoint_id = uuid::Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

    db.conn.execute(
        "INSERT INTO audit_checkpoints \
         (id, lineage_id, start_seq, end_seq, entry_count, merkle_root, created_at, created_by) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            &checkpoint_id, &lineage_id, actual_start, actual_end,
            entry_count, &merkle_root, &created_at, &user.id
        ],
    ).map_err(|e| e.to_string())?;

    Ok(CreateCheckpointResult {
        checkpoint_id,
        lineage_id,
        start_seq: actual_start,
        end_seq: actual_end,
        entry_count,
        merkle_root,
    })
}

/// Verify a stored checkpoint against the current state of the audit chain.
///
/// Checks (in order):
///   1. Entry count — detects deletions or insertions in the sealed range.
///   2. Merkle root — rebuilt from current `entry_hash` values; mismatch means
///      a hash value was changed (content+hash co-tampered).
///   3. Individual content hashes — recomputes each entry_hash from canonical
///      fields; mismatch here means content was edited without updating entry_hash.
#[tauri::command]
pub fn verify_against_checkpoint(
    state: State<AppState>,
    token: String,
    checkpoint_id: String,
) -> Result<VerifyCheckpointResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    auth_service::validate_session(&db, &token)?;

    struct CpRow {
        lineage_id: String,
        start_seq: i64,
        end_seq: i64,
        expected_count: i64,
        stored_root: String,
    }

    let cp = db.conn.query_row(
        "SELECT lineage_id, start_seq, end_seq, entry_count, merkle_root \
         FROM audit_checkpoints WHERE id = ?1",
        rusqlite::params![&checkpoint_id],
        |r| Ok(CpRow {
            lineage_id: r.get(0)?,
            start_seq: r.get(1)?,
            end_seq: r.get(2)?,
            expected_count: r.get(3)?,
            stored_root: r.get(4)?,
        }),
    ).map_err(|_| format!("Checkpoint '{}' not found.", checkpoint_id))?;

    struct EntryRow {
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
         WHERE lineage_id = ?1 AND chain_seq >= ?2 AND chain_seq <= ?3 AND entry_hash IS NOT NULL \
         ORDER BY chain_seq ASC",
    ).map_err(|e| e.to_string())?;

    let entries: Vec<EntryRow> = stmt
        .query_map(
            rusqlite::params![&cp.lineage_id, cp.start_seq, cp.end_seq],
            |r| Ok(EntryRow {
                chain_seq: r.get(0)?,
                user_id: r.get(1)?,
                entity_type: r.get(2)?,
                action: r.get(3)?,
                entity_id: r.get(4)?,
                created_at: r.get(5)?,
                details: r.get(6)?,
                prev_hash: r.get(7)?,
                entry_hash: r.get(8)?,
            }),
        ).map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let actual_count = entries.len() as i64;

    // Check 1: entry count (detects deletions / insertions)
    if actual_count != cp.expected_count {
        let diff = (cp.expected_count - actual_count).abs();
        let noun = if diff == 1 { "entry" } else { "entries" };
        return Ok(VerifyCheckpointResult {
            checkpoint_id,
            lineage_id: cp.lineage_id,
            ok: false,
            expected_count: cp.expected_count,
            actual_count,
            tampered_seq: None,
            message: format!(
                "Entry count mismatch — expected {}, found {}. {} {} may have been removed or inserted.",
                cp.expected_count, actual_count, diff, noun
            ),
        });
    }

    // Check 2: Merkle root over current stored entry_hash values
    let current_hashes: Vec<String> = entries.iter().map(|e| e.entry_hash.clone()).collect();
    let current_root = build_merkle_root(&current_hashes);

    if current_root != cp.stored_root {
        // Try to identify the tampered entry via individual hash recomputation.
        let tampered_seq = entries.iter().find_map(|e| {
            let canonical = audit_canonical_bytes(
                &cp.lineage_id, e.chain_seq, &e.created_at,
                e.user_id.as_deref().unwrap_or(""),
                &e.entity_type,
                e.entity_id.as_deref().unwrap_or(""),
                &e.action,
                e.details.as_deref().unwrap_or(""),
            );
            let computed = compute_entry_hash(&canonical, &e.prev_hash);
            if computed != e.entry_hash { Some(e.chain_seq) } else { None }
        });

        let message = if let Some(seq) = tampered_seq {
            format!(
                "Merkle root mismatch — entry at seq {} was tampered with (content no longer matches its stored hash).",
                seq
            )
        } else {
            "Merkle root mismatch — stored entry_hash values have been altered. \
             Use chain verification for per-entry detail.".to_string()
        };
        return Ok(VerifyCheckpointResult {
            checkpoint_id,
            lineage_id: cp.lineage_id,
            ok: false,
            expected_count: cp.expected_count,
            actual_count,
            tampered_seq,
            message,
        });
    }

    // Check 3: individual entry content hashes (catches content edits without hash update)
    for e in &entries {
        let canonical = audit_canonical_bytes(
            &cp.lineage_id, e.chain_seq, &e.created_at,
            e.user_id.as_deref().unwrap_or(""),
            &e.entity_type,
            e.entity_id.as_deref().unwrap_or(""),
            &e.action,
            e.details.as_deref().unwrap_or(""),
        );
        let computed = compute_entry_hash(&canonical, &e.prev_hash);
        if computed != e.entry_hash {
            return Ok(VerifyCheckpointResult {
                checkpoint_id,
                lineage_id: cp.lineage_id,
                ok: false,
                expected_count: cp.expected_count,
                actual_count,
                tampered_seq: Some(e.chain_seq),
                message: format!(
                    "Content tampered at seq {} — entry_hash unchanged (Merkle root still matches) but content was modified.",
                    e.chain_seq
                ),
            });
        }
    }

    Ok(VerifyCheckpointResult {
        checkpoint_id,
        lineage_id: cp.lineage_id,
        ok: true,
        expected_count: cp.expected_count,
        actual_count,
        tampered_seq: None,
        message: format!(
            "Checkpoint verified — all {} {} match the recorded Merkle root.",
            actual_count,
            if actual_count == 1 { "entry" } else { "entries" }
        ),
    })
}

/// List all stored checkpoints, optionally filtered to a single lineage.
#[tauri::command]
pub fn list_audit_checkpoints(
    state: State<AppState>,
    token: String,
    lineage_id: Option<String>,
) -> Result<Vec<AuditCheckpoint>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    auth_service::validate_session(&db, &token)?;

    let rows: Vec<AuditCheckpoint> = if let Some(ref lid) = lineage_id {
        let mut stmt = db.conn.prepare(
            "SELECT id, lineage_id, start_seq, end_seq, entry_count, merkle_root, created_at, created_by, anchored_txid \
             FROM audit_checkpoints WHERE lineage_id = ?1 ORDER BY created_at DESC",
        ).map_err(|e| e.to_string())?;
        let collected: Vec<AuditCheckpoint> = stmt.query_map(rusqlite::params![lid], |r| Ok(AuditCheckpoint {
            id: r.get(0)?, lineage_id: r.get(1)?, start_seq: r.get(2)?,
            end_seq: r.get(3)?, entry_count: r.get(4)?, merkle_root: r.get(5)?,
            created_at: r.get(6)?, created_by: r.get(7)?, anchored_txid: r.get(8)?,
        })).map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
        collected
    } else {
        let mut stmt = db.conn.prepare(
            "SELECT id, lineage_id, start_seq, end_seq, entry_count, merkle_root, created_at, created_by, anchored_txid \
             FROM audit_checkpoints ORDER BY created_at DESC LIMIT 100",
        ).map_err(|e| e.to_string())?;
        let collected: Vec<AuditCheckpoint> = stmt.query_map([], |r| Ok(AuditCheckpoint {
            id: r.get(0)?, lineage_id: r.get(1)?, start_seq: r.get(2)?,
            end_seq: r.get(3)?, entry_count: r.get(4)?, merkle_root: r.get(5)?,
            created_at: r.get(6)?, created_by: r.get(7)?, anchored_txid: r.get(8)?,
        })).map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
        collected
    };

    Ok(rows)
}

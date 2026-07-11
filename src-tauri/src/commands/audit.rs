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

    // Guard the divisor: a per_page of 0 would make this Inf → u32::MAX pages.
    let total_pages = ((total as f64) / (pg.per_page.max(1) as f64)).ceil() as u32;

    Ok(PaginatedResponse {
        items: entries,
        total,
        page: pg.page,
        per_page: pg.per_page,
        total_pages,
    })
}

/// WP-63: cursor-based pagination over a single lineage's audit entries,
/// using `chain_seq` as the stable cursor. Additive to `get_audit_log` (the
/// general cross-entity search view, which remains offset-paginated) — this
/// exists for the per-lineage Audit Log detail view, which becomes
/// prohibitively slow to load in full at the 1M-entry scale WP-63 targets.
/// `after_seq: None` starts from the beginning of the lineage; pass back the
/// previous page's `next_cursor` to fetch the next page ("load later").
#[tauri::command]
pub fn list_audit_entries_cursor(
    state: State<AppState>,
    token: String,
    lineage_id: String,
    after_seq: Option<i64>,
    limit: i64,
) -> Result<queries::CursorPage<AuditEntry>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err("Insufficient permissions".to_string());
    }
    queries::list_audit_entries_by_cursor(&db.conn, &lineage_id, after_seq, limit)
        .map_err(|e| e.to_string())
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
    // `idx` is the number of entries fully verified before the current row, which is
    // the correct "checked" count on a break. (Deriving it from `chain_seq - 1`
    // underflowed to usize::MAX when the break was on a genesis seq-0 entry.)
    for (idx, row) in rows.iter().enumerate() {
        // Verify the link: this row's prev_hash must equal the previous row's entry_hash
        // (or the anchor for the very first row, which trivially passes).
        if row.prev_hash != prev_entry_hash {
            return Ok(VerifyChainResult {
                lineage_id,
                ok: false,
                checked: idx,
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
                checked: idx,
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
         (id, lineage_id, start_seq, end_seq, entry_count, merkle_root, created_at, created_by, is_auto) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0)",
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
            "SELECT id, lineage_id, start_seq, end_seq, entry_count, merkle_root, \
                    created_at, created_by, anchored_txid, is_auto, auto_source \
             FROM audit_checkpoints WHERE lineage_id = ?1 ORDER BY created_at DESC",
        ).map_err(|e| e.to_string())?;
        let collected: Vec<AuditCheckpoint> = stmt
            .query_map(rusqlite::params![lid], |r| Ok(AuditCheckpoint {
                id: r.get(0)?, lineage_id: r.get(1)?, start_seq: r.get(2)?,
                end_seq: r.get(3)?, entry_count: r.get(4)?, merkle_root: r.get(5)?,
                created_at: r.get(6)?, created_by: r.get(7)?, anchored_txid: r.get(8)?,
                is_auto: r.get::<_, i64>(9).map(|v| v != 0).unwrap_or(false),
                auto_source: r.get(10)?,
            }))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        collected
    } else {
        let mut stmt = db.conn.prepare(
            "SELECT id, lineage_id, start_seq, end_seq, entry_count, merkle_root, \
                    created_at, created_by, anchored_txid, is_auto, auto_source \
             FROM audit_checkpoints ORDER BY created_at DESC LIMIT 100",
        ).map_err(|e| e.to_string())?;
        let collected: Vec<AuditCheckpoint> = stmt
            .query_map([], |r| Ok(AuditCheckpoint {
                id: r.get(0)?, lineage_id: r.get(1)?, start_seq: r.get(2)?,
                end_seq: r.get(3)?, entry_count: r.get(4)?, merkle_root: r.get(5)?,
                created_at: r.get(6)?, created_by: r.get(7)?, anchored_txid: r.get(8)?,
                is_auto: r.get::<_, i64>(9).map(|v| v != 0).unwrap_or(false),
                auto_source: r.get(10)?,
            }))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        collected
    };

    Ok(rows)
}

// ---------------------------------------------------------------------------
// WP-21 — Proof export, import verification, and auto-checkpointing
// ---------------------------------------------------------------------------

/// Export a portable, self-contained Merkle proof for the given checkpoint as JSON.
///
/// The proof contains all audit entries in the sealed range with their canonical form
/// (so entry hashes can be recomputed) and individual Merkle paths (so each entry can
/// be independently proven without the full set). The exported JSON can be verified
/// offline — see `verify_exported_proof` and `docs/merkle-proofs.md`.
#[tauri::command]
pub fn export_audit_proof(
    state: State<AppState>,
    token: String,
    checkpoint_id: String,
) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    auth_service::validate_session(&db, &token)?;

    let cp = db.conn.query_row(
        "SELECT lineage_id, start_seq, end_seq, entry_count, merkle_root, created_at \
         FROM audit_checkpoints WHERE id = ?1",
        rusqlite::params![&checkpoint_id],
        |r| Ok(ProofCheckpointMeta {
            id: checkpoint_id.clone(),
            lineage_id: r.get(0)?,
            start_seq: r.get(1)?,
            end_seq: r.get(2)?,
            entry_count: r.get(3)?,
            merkle_root: r.get(4)?,
            created_at: r.get(5)?,
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

    let rows: Vec<EntryRow> = stmt.query_map(
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

    if rows.is_empty() {
        return Err(format!(
            "No entries found for checkpoint '{}' — lineage '{}' seq [{}, {}].",
            checkpoint_id, cp.lineage_id, cp.start_seq, cp.end_seq
        ));
    }

    let leaf_hashes: Vec<String> = rows.iter().map(|r| r.entry_hash.clone()).collect();

    let entries: Vec<ProofEntry> = rows.iter().enumerate().map(|(i, row)| {
        let canonical_bytes = audit_canonical_bytes(
            &cp.lineage_id, row.chain_seq, &row.created_at,
            row.user_id.as_deref().unwrap_or(""),
            &row.entity_type,
            row.entity_id.as_deref().unwrap_or(""),
            &row.action,
            row.details.as_deref().unwrap_or(""),
        );
        let canonical = String::from_utf8_lossy(&canonical_bytes).to_string();

        let merkle_path = queries::build_merkle_path(&leaf_hashes, i)
            .into_iter()
            .map(|n| MerklePathNode { sibling_hash: n.sibling_hash, position: n.position })
            .collect();

        ProofEntry {
            chain_seq: row.chain_seq,
            canonical,
            prev_hash: row.prev_hash.clone(),
            entry_hash: row.entry_hash.clone(),
            merkle_path,
        }
    }).collect();

    let proof = PortableMerkleProof {
        version: "1".to_string(),
        exported_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
        checkpoint: cp,
        entries,
    };

    serde_json::to_string_pretty(&proof).map_err(|e| e.to_string())
}

/// Verify an exported proof JSON without requiring any database access.
///
/// Three stages:
///   1. Entry hash — recompute SHA256(canonical || prev_hash) for each entry.
///   2. Chain links — confirm each entry's prev_hash equals the preceding entry's entry_hash.
///   3. Merkle root — rebuild from entry_hash values and compare to the stored checkpoint root.
#[tauri::command]
pub fn verify_exported_proof(
    state: State<AppState>,
    token: String,
    proof_json: String,
) -> Result<VerifyProofResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    auth_service::validate_session(&db, &token)?;

    let proof: PortableMerkleProof = serde_json::from_str(&proof_json)
        .map_err(|e| format!("Invalid proof JSON: {}", e))?;

    Ok(verify_proof_data(&proof))
}

/// Pure verification logic — no DB access required.
pub fn verify_proof_data(proof: &PortableMerkleProof) -> VerifyProofResult {
    if proof.version != "1" {
        return VerifyProofResult {
            ok: false,
            message: format!("Unsupported proof version '{}'.", proof.version),
            entry_count: 0,
            merkle_root: proof.checkpoint.merkle_root.clone(),
            failure_reason: Some(format!("Version '{}' not supported; expected '1'.", proof.version)),
            failed_seq: None,
        };
    }

    let n = proof.entries.len() as i64;
    if n != proof.checkpoint.entry_count {
        return VerifyProofResult {
            ok: false,
            message: format!(
                "Entry count mismatch — proof has {} entries but checkpoint expected {}.",
                n, proof.checkpoint.entry_count
            ),
            entry_count: n,
            merkle_root: proof.checkpoint.merkle_root.clone(),
            failure_reason: Some("Entry count mismatch".to_string()),
            failed_seq: None,
        };
    }

    // Stage 1: recompute each entry_hash from its canonical form
    for entry in &proof.entries {
        let computed = compute_entry_hash(entry.canonical.as_bytes(), &entry.prev_hash);
        if computed != entry.entry_hash {
            return VerifyProofResult {
                ok: false,
                message: format!(
                    "Hash mismatch at seq {} — canonical form does not match the stored entry_hash.",
                    entry.chain_seq
                ),
                entry_count: n,
                merkle_root: proof.checkpoint.merkle_root.clone(),
                failure_reason: Some("Content hash mismatch".to_string()),
                failed_seq: Some(entry.chain_seq),
            };
        }
    }

    // Stage 2: verify entries are in ascending chain_seq order, then check links.
    // Out-of-order entries would produce false chain-break failures; reject them
    // explicitly with a clear error rather than a misleading "chain link broken".
    for i in 1..proof.entries.len() {
        let prev = &proof.entries[i - 1];
        let curr = &proof.entries[i];
        if curr.chain_seq <= prev.chain_seq {
            return VerifyProofResult {
                ok: false,
                message: format!(
                    "Entry ordering error — seq {} appears after seq {} (entries must be ascending).",
                    curr.chain_seq, prev.chain_seq
                ),
                entry_count: n,
                merkle_root: proof.checkpoint.merkle_root.clone(),
                failure_reason: Some("Entries out of order".to_string()),
                failed_seq: Some(curr.chain_seq),
            };
        }
        if curr.prev_hash != prev.entry_hash {
            return VerifyProofResult {
                ok: false,
                message: format!(
                    "Chain break at seq {} — prev_hash does not match the preceding entry_hash.",
                    curr.chain_seq
                ),
                entry_count: n,
                merkle_root: proof.checkpoint.merkle_root.clone(),
                failure_reason: Some("Chain link broken".to_string()),
                failed_seq: Some(curr.chain_seq),
            };
        }
    }

    // Stage 3: rebuild Merkle root from entry hashes
    let leaf_hashes: Vec<String> = proof.entries.iter().map(|e| e.entry_hash.clone()).collect();
    let computed_root = build_merkle_root(&leaf_hashes);
    if computed_root != proof.checkpoint.merkle_root {
        return VerifyProofResult {
            ok: false,
            message: "Merkle root mismatch — the proof root does not match the checkpoint's stored root.".to_string(),
            entry_count: n,
            merkle_root: proof.checkpoint.merkle_root.clone(),
            failure_reason: Some("Merkle root mismatch".to_string()),
            failed_seq: None,
        };
    }

    VerifyProofResult {
        ok: true,
        message: format!(
            "Proof verified — all {} {} are intact and the Merkle root matches the checkpoint.",
            n, if n == 1 { "entry" } else { "entries" }
        ),
        entry_count: n,
        merkle_root: computed_root,
        failure_reason: None,
        failed_seq: None,
    }
}

/// Read the current auto-checkpoint configuration from app_settings.
#[tauri::command]
pub fn get_auto_checkpoint_config(
    state: State<AppState>,
    token: String,
) -> Result<AutoCheckpointConfig, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    auth_service::validate_session(&db, &token)?;

    let enabled = queries::read_setting(&db.conn, "auto_checkpoint_enabled", "1") == "1";
    let interval = queries::read_setting(&db.conn, "auto_checkpoint_interval", "100")
        .parse::<i64>().unwrap_or(100);
    let on_backup = queries::read_setting(&db.conn, "auto_checkpoint_on_backup", "1") == "1";

    Ok(AutoCheckpointConfig { enabled, interval, on_backup })
}

/// Persist the auto-checkpoint configuration into app_settings.
#[tauri::command]
pub fn set_auto_checkpoint_config(
    state: State<AppState>,
    token: String,
    config: AutoCheckpointConfig,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err("Insufficient permissions — admin or supervisor role required.".to_string());
    }

    if config.interval < 0 {
        return Err("interval must be non-negative".to_string());
    }

    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    let pairs = [
        ("auto_checkpoint_enabled", if config.enabled { "1" } else { "0" }),
        ("auto_checkpoint_on_backup", if config.on_backup { "1" } else { "0" }),
    ];
    for (key, val) in &pairs {
        db.conn.execute(
            "INSERT OR REPLACE INTO app_settings (key, value, updated_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![key, val, &now],
        ).map_err(|e| e.to_string())?;
    }
    db.conn.execute(
        "INSERT OR REPLACE INTO app_settings (key, value, updated_at) VALUES ('auto_checkpoint_interval', ?1, ?2)",
        rusqlite::params![config.interval.to_string(), &now],
    ).map_err(|e| e.to_string())?;

    Ok(())
}

/// Trigger auto-checkpointing for all eligible lineages right now.
///
/// Respects the configured `interval`: a lineage is eligible when it has at least
/// that many uncovered entries. Set interval to 0 in config to checkpoint everything.
#[tauri::command]
pub fn run_auto_checkpoint(
    state: State<AppState>,
    token: String,
) -> Result<AutoCheckpointResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err("Insufficient permissions — admin or supervisor role required.".to_string());
    }

    let enabled = queries::read_setting(&db.conn, "auto_checkpoint_enabled", "1") == "1";
    if !enabled {
        return Ok(AutoCheckpointResult {
            lineages_checked: 0,
            checkpoints_created: 0,
            details: vec!["Auto-checkpointing is disabled in settings.".to_string()],
        });
    }

    let interval = queries::read_setting(&db.conn, "auto_checkpoint_interval", "100")
        .parse::<i64>().unwrap_or(100);

    // Count only lineages that have at least one uncovered entry — matching
    // what auto_checkpoint_lineages actually iterates over.
    let lineages_checked: usize = db.conn.query_row(
        "SELECT COUNT(DISTINCT a.lineage_id) \
         FROM audit_log a \
         WHERE a.entry_hash IS NOT NULL AND a.lineage_id IS NOT NULL \
           AND a.chain_seq > COALESCE(\
               (SELECT MAX(end_seq) FROM audit_checkpoints WHERE lineage_id = a.lineage_id), \
               -1\
           )",
        [],
        |r| r.get(0),
    ).unwrap_or(0);

    let created = queries::auto_checkpoint_lineages(&db.conn, &user.id, "entry_count", interval)
        .map_err(|e| e.to_string())?;

    let checkpoints_created = created.len();
    let details = if created.is_empty() {
        vec![format!(
            "No lineages had {} or more uncovered entries (interval = {}).",
            interval, interval
        )]
    } else {
        created.iter().map(|id| format!("Created checkpoint {}…", &id[..8])).collect()
    };

    Ok(AutoCheckpointResult { lineages_checked, checkpoints_created, details })
}

// --- WP-21: Proof verification unit tests ---

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::queries::{ZERO_HASH, compute_entry_hash, build_merkle_root};

    fn make_valid_proof() -> PortableMerkleProof {
        let lineage_id = "sp-TEST";
        let ts1 = "2026-01-01T00:00:00.000Z";
        let ts2 = "2026-01-01T00:01:00.000Z";

        // Canonical format: lineage_id|chain_seq|timestamp|user_id|entity_type|entity_id|action|details
        // (8 fields, 7 separators — matches audit_canonical_bytes exactly)
        let canon1 = format!("{}|1|{}||specimen|sp-TEST|create|", lineage_id, ts1);
        let hash1 = compute_entry_hash(canon1.as_bytes(), ZERO_HASH);

        let canon2 = format!("{}|2|{}||specimen|sp-TEST|update|", lineage_id, ts2);
        let hash2 = compute_entry_hash(canon2.as_bytes(), &hash1);

        let leaves = vec![hash1.clone(), hash2.clone()];
        let root = build_merkle_root(&leaves);

        PortableMerkleProof {
            version: "1".to_string(),
            exported_at: "2026-01-01T00:02:00.000Z".to_string(),
            checkpoint: ProofCheckpointMeta {
                id: "cp-test".to_string(),
                lineage_id: lineage_id.to_string(),
                start_seq: 1,
                end_seq: 2,
                entry_count: 2,
                merkle_root: root,
                created_at: "2026-01-01T00:02:00.000Z".to_string(),
            },
            entries: vec![
                ProofEntry {
                    chain_seq: 1,
                    canonical: canon1,
                    prev_hash: ZERO_HASH.to_string(),
                    entry_hash: hash1.clone(),
                    merkle_path: vec![MerklePathNode {
                        sibling_hash: hash2.clone(),
                        position: "right".to_string(),
                    }],
                },
                ProofEntry {
                    chain_seq: 2,
                    canonical: canon2,
                    prev_hash: hash1.clone(),
                    entry_hash: hash2.clone(),
                    merkle_path: vec![MerklePathNode {
                        sibling_hash: hash1.clone(),
                        position: "left".to_string(),
                    }],
                },
            ],
        }
    }

    #[test]
    fn proof_verify_passes_for_valid_proof() {
        let proof = make_valid_proof();
        let result = verify_proof_data(&proof);
        assert!(result.ok, "valid proof must verify: {}", result.message);
        assert_eq!(result.entry_count, 2);
    }

    #[test]
    fn proof_verify_detects_tampered_canonical() {
        let mut proof = make_valid_proof();
        proof.entries[0].canonical = "tampered|data".to_string();
        let result = verify_proof_data(&proof);
        assert!(!result.ok, "tampered canonical must fail verification");
        assert_eq!(result.failed_seq, Some(1));
        assert_eq!(result.failure_reason.as_deref(), Some("Content hash mismatch"));
    }

    #[test]
    fn proof_verify_detects_broken_chain_link() {
        let mut proof = make_valid_proof();
        // Recompute entry 2 with a wrong prev_hash so stage 1 passes but stage 2 fails.
        let wrong_prev = "0".repeat(64);
        let canon2 = proof.entries[1].canonical.clone();
        proof.entries[1].prev_hash = wrong_prev.clone();
        proof.entries[1].entry_hash = compute_entry_hash(canon2.as_bytes(), &wrong_prev);
        let result = verify_proof_data(&proof);
        assert!(!result.ok, "broken chain link must fail verification");
        assert_eq!(result.failed_seq, Some(2));
        assert_eq!(result.failure_reason.as_deref(), Some("Chain link broken"));
    }

    #[test]
    fn proof_verify_detects_wrong_root() {
        let mut proof = make_valid_proof();
        proof.checkpoint.merkle_root = "b".repeat(64);
        let result = verify_proof_data(&proof);
        assert!(!result.ok, "wrong Merkle root must fail verification");
        assert_eq!(result.failure_reason.as_deref(), Some("Merkle root mismatch"));
        assert_eq!(result.failed_seq, None);
    }
}

use crate::auth as auth_service;
use crate::db::queries;
use crate::models::taxon::{
    ImportNcbiTaxonomyRequest, ImportNcbiTaxonomyResult, NcbiConflictSummary, NcbiSkippedRecord,
    NcbiSyncLog, NcbiTaxonRecord, ResolveNcbiConflictRequest,
};
use std::collections::{HashMap, HashSet};
use crate::AppState;
use rusqlite::params;
use tauri::State;

// ── import_ncbi_taxonomy ──────────────────────────────────────────────────────

/// Import or sync a list of NCBI taxon records into the local `taxa` table.
///
/// The caller (admin UI / frontend) is responsible for fetching the raw data
/// from the NCBI Taxonomy API and normalizing it into `NcbiTaxonRecord` objects
/// before calling this command.
///
/// Behaviour per record:
/// - **Skip** if a matching local taxon has `local_override = true`.
/// - **Conflict** if a local taxon with the same `ncbi_taxon_id` exists but
///   the name or rank differs → writes to `ncbi_sync_log` (real mode only).
/// - **Update** if a local taxon matches by `ncbi_taxon_id` with no field
///   differences, or matches by `name + rank` and needs its `ncbi_taxon_id` linked.
/// - **Import** if no matching local taxon is found → inserts a new taxon row.
///
/// Records whose rank is outside the backbone (species, subspecies, no rank, …)
/// are reported in `skipped_records` rather than discarded silently, and
/// `parent_ncbi_id` is honoured: after the inserts, each taxon is linked to its
/// parent — whether that parent arrived in the same paste or was already in the
/// database — and its `taxon_path` is rewritten to the full ancestor chain.
/// Without that step an imported lineage came out flat, with every taxon sitting
/// at the root of the navigator regardless of its rank.
///
/// When `dry_run = true` the result describes what *would* happen without any
/// writes to the database or the sync log.
#[tauri::command]
pub fn import_ncbi_taxonomy(
    state: State<AppState>,
    token: String,
    request: ImportNcbiTaxonomyRequest,
) -> Result<ImportNcbiTaxonomyResult, String> {
    let db = state.db();
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.is_admin() {
        return Err("Only admins can import NCBI taxonomy data".to_string());
    }

    let now = chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string();

    // Phase 1 — classify every incoming record (pure reads, no DB writes yet).
    struct Action {
        kind: ActionKind,
        record: NcbiTaxonRecord,
        taxon_id: Option<String>,
        /// Local taxon name captured during Phase 1 (used in conflict summaries).
        local_name: Option<String>,
        log_id: String,
        conflict_details: Option<String>,
    }
    enum ActionKind {
        Import,
        Update,
        Conflict,
        Skip,
    }

    let mut actions: Vec<Action> = Vec::new();
    let mut skipped_records: Vec<NcbiSkippedRecord> = Vec::new();
    // A paste that carries a full lineage repeats shared ancestors on every
    // line. Importing the same taxid twice in one batch would create two rows
    // the second of which is immediately a conflict with the first, so collapse
    // duplicates here and say so.
    let mut seen_ids: HashSet<i64> = HashSet::new();

    for record in request.taxa {
        if !seen_ids.insert(record.ncbi_taxon_id) {
            skipped_records.push(NcbiSkippedRecord {
                ncbi_taxon_id: record.ncbi_taxon_id,
                name: record.name.clone(),
                rank: record.rank.clone(),
                reason: "duplicate of an earlier record in this batch".to_string(),
            });
            continue;
        }

        let rank = match queries::normalize_ncbi_rank(&record.rank) {
            Some(r) => r,
            None => {
                skipped_records.push(NcbiSkippedRecord {
                    ncbi_taxon_id: record.ncbi_taxon_id,
                    name: record.name.clone(),
                    rank: record.rank.clone(),
                    reason: format!(
                        "rank '{}' is outside the taxonomy backbone (kingdom, phylum, class, \
                         order, family, genus) — species and below belong in the Species Registry",
                        record.rank
                    ),
                });
                continue;
            }
        };

        // Primary lookup: match by ncbi_taxon_id.
        let by_ncbi_id = queries::find_taxon_by_ncbi_id(&db.conn, record.ncbi_taxon_id)
            .map_err(|e| e.to_string())?;

        if let Some(local) = by_ncbi_id {
            if local.local_override {
                actions.push(Action {
                    kind: ActionKind::Skip,
                    taxon_id: Some(local.id),
                    local_name: None,
                    log_id: String::new(),
                    conflict_details: None,
                    record,
                });
                continue;
            }

            if let Some(details) = queries::detect_ncbi_conflict(&local, &record.name, rank) {
                actions.push(Action {
                    kind: ActionKind::Conflict,
                    taxon_id: Some(local.id),
                    local_name: Some(local.name),
                    log_id: uuid::Uuid::new_v4().to_string(),
                    conflict_details: Some(details),
                    record,
                });
            } else {
                actions.push(Action {
                    kind: ActionKind::Update,
                    taxon_id: Some(local.id),
                    local_name: None,
                    log_id: uuid::Uuid::new_v4().to_string(),
                    conflict_details: None,
                    record,
                });
            }
            continue;
        }

        // Secondary lookup: match by name + normalized rank.
        let by_name = queries::find_taxon_by_name_rank(&db.conn, &record.name, rank)
            .map_err(|e| e.to_string())?;

        if let Some(local) = by_name {
            if local.local_override {
                actions.push(Action {
                    kind: ActionKind::Skip,
                    taxon_id: Some(local.id),
                    local_name: None,
                    log_id: String::new(),
                    conflict_details: None,
                    record,
                });
                continue;
            }
            actions.push(Action {
                kind: ActionKind::Update,
                taxon_id: Some(local.id),
                local_name: None,
                log_id: uuid::Uuid::new_v4().to_string(),
                conflict_details: None,
                record,
            });
            continue;
        }

        // No match — create new taxon.
        actions.push(Action {
            kind: ActionKind::Import,
            taxon_id: Some(uuid::Uuid::new_v4().to_string()),
            local_name: None,
            log_id: uuid::Uuid::new_v4().to_string(),
            conflict_details: None,
            record,
        });
    }

    // Phase 2 — build result and optionally apply writes.
    let mut imported: i64 = 0;
    let mut updated: i64 = 0;
    let mut skipped_overrides: i64 = 0;
    let mut parents_linked: i64 = 0;
    let mut conflicts: Vec<NcbiConflictSummary> = Vec::new();

    if !request.dry_run {
        let tx = db
            .conn
            .unchecked_transaction()
            .map_err(|e| format!("Failed to begin transaction: {}", e))?;

        for action in &actions {
            let rank = match queries::normalize_ncbi_rank(&action.record.rank) {
                Some(r) => r,
                None => continue,
            };
            match action.kind {
                ActionKind::Import => {
                    let new_id = action.taxon_id.as_deref().unwrap_or("");
                    let path = format!("[\"{}\"]", new_id);
                    tx.execute(
                        "INSERT INTO taxa (id, rank, name, ncbi_taxon_id, ncbi_updated_at,
                         local_override, taxon_path)
                         VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6)",
                        params![
                            new_id,
                            rank,
                            action.record.name,
                            action.record.ncbi_taxon_id,
                            now,
                            path
                        ],
                    )
                    .map_err(|e| {
                        format!("Failed to import '{}': {}", action.record.name, e)
                    })?;
                    queries::insert_ncbi_sync_log(
                        &tx,
                        &action.log_id,
                        "import",
                        Some(new_id),
                        Some(action.record.ncbi_taxon_id),
                        None,
                        &now,
                    )
                    .map_err(|e| e.to_string())?;
                    imported += 1;
                }
                ActionKind::Update => {
                    let tid = action.taxon_id.as_deref().unwrap_or("");
                    tx.execute(
                        "UPDATE taxa SET ncbi_taxon_id = ?1, ncbi_updated_at = ?2,
                         updated_at = datetime('now') WHERE id = ?3",
                        params![action.record.ncbi_taxon_id, now, tid],
                    )
                    .map_err(|e| format!("Failed to update taxon '{}': {}", tid, e))?;
                    queries::insert_ncbi_sync_log(
                        &tx,
                        &action.log_id,
                        "update",
                        Some(tid),
                        Some(action.record.ncbi_taxon_id),
                        None,
                        &now,
                    )
                    .map_err(|e| e.to_string())?;
                    updated += 1;
                }
                ActionKind::Conflict => {
                    let tid = action.taxon_id.as_deref();
                    queries::insert_ncbi_sync_log(
                        &tx,
                        &action.log_id,
                        "conflict",
                        tid,
                        Some(action.record.ncbi_taxon_id),
                        action.conflict_details.as_deref(),
                        &now,
                    )
                    .map_err(|e| e.to_string())?;
                    conflicts.push(NcbiConflictSummary {
                        sync_log_id: Some(action.log_id.clone()),
                        taxon_id: action.taxon_id.clone(),
                        ncbi_taxon_id: action.record.ncbi_taxon_id,
                        local_name: action.local_name.clone(),
                        ncbi_name: action.record.name.clone(),
                        conflict_details: action
                            .conflict_details
                            .clone()
                            .unwrap_or_default(),
                    });
                }
                ActionKind::Skip => {
                    skipped_overrides += 1;
                }
            }
        }

        // Phase 3 — wire up parents now that every taxon in the batch exists.
        //
        // `parent_ncbi_id` was previously read off the wire and thrown away, so
        // an imported Kingdom → Phylum → … → Genus chain came out as N separate
        // roots: the navigator's first column filled with genera, and drilling
        // into any of them found nothing. Resolving parents here — first against
        // taxa created in this same batch, then against taxa already in the
        // database — is what turns the paste into an actual tree.
        //
        // A local_override taxon is deliberately left alone: overriding is the
        // operator saying "my classification wins", and re-parenting it would
        // move it under NCBI's hierarchy anyway.
        let mut by_ncbi_id: HashMap<i64, String> = HashMap::new();
        for action in &actions {
            if let (ActionKind::Import | ActionKind::Update, Some(tid)) =
                (&action.kind, action.taxon_id.as_deref())
            {
                by_ncbi_id.insert(action.record.ncbi_taxon_id, tid.to_string());
            }
        }

        let mut touched: Vec<String> = Vec::new();
        for action in &actions {
            let (Some(parent_ncbi), Some(child_id)) =
                (action.record.parent_ncbi_id, action.taxon_id.as_deref())
            else {
                continue;
            };
            if !matches!(action.kind, ActionKind::Import | ActionKind::Update) {
                continue;
            }
            // A taxon that is its own parent is how NCBI marks the root (taxid
            // 1). Linking it to itself would build a cycle.
            if parent_ncbi == action.record.ncbi_taxon_id {
                continue;
            }

            let parent_id = match by_ncbi_id.get(&parent_ncbi) {
                Some(id) => Some(id.clone()),
                None => queries::find_taxon_by_ncbi_id(&tx, parent_ncbi)
                    .map_err(|e| e.to_string())?
                    .map(|t| t.id),
            };
            let Some(parent_id) = parent_id else { continue };
            if parent_id == child_id {
                continue;
            }

            queries::set_taxon_parent(&tx, child_id, &parent_id).map_err(|e| e.to_string())?;
            parents_linked += 1;
            touched.push(child_id.to_string());
        }

        // Recompute paths once per touched subtree root, after every link is in
        // place — doing it per-link would rewrite descendants repeatedly and
        // could read a half-built chain.
        //
        // `species.taxon_path` is a copy of its genus taxon's path, so moving a
        // taxon has to move the species hanging off it too. Skipping this left
        // ancestor columns counting zero and broke "Open in Taxonomy", because
        // the species still claimed a one-element lineage that no longer began
        // at a root.
        for id in &touched {
            queries::recompute_taxon_path(&tx, id).map_err(|e| e.to_string())?;
            queries::resync_species_paths_under(&tx, id).map_err(|e| e.to_string())?;
        }

        tx.commit().map_err(|e| format!("Failed to commit import: {}", e))?;
    } else {
        // Dry-run: tally without writing.
        let batch_ids: HashSet<i64> = actions
            .iter()
            .filter(|a| matches!(a.kind, ActionKind::Import | ActionKind::Update))
            .map(|a| a.record.ncbi_taxon_id)
            .collect();

        for action in &actions {
            match action.kind {
                ActionKind::Import => imported += 1,
                ActionKind::Update => updated += 1,
                ActionKind::Skip => skipped_overrides += 1,
                ActionKind::Conflict => {
                    conflicts.push(NcbiConflictSummary {
                        sync_log_id: None,
                        taxon_id: action.taxon_id.clone(),
                        ncbi_taxon_id: action.record.ncbi_taxon_id,
                        local_name: None,
                        ncbi_name: action.record.name.clone(),
                        conflict_details: action
                            .conflict_details
                            .clone()
                            .unwrap_or_default(),
                    });
                }
            }

            // Predict the parent links the real run would make, using the same
            // rules, so the preview count matches what actually happens.
            if !matches!(action.kind, ActionKind::Import | ActionKind::Update) {
                continue;
            }
            let Some(parent_ncbi) = action.record.parent_ncbi_id else { continue };
            if parent_ncbi == action.record.ncbi_taxon_id {
                continue;
            }
            let resolvable = batch_ids.contains(&parent_ncbi)
                || queries::find_taxon_by_ncbi_id(&db.conn, parent_ncbi)
                    .map_err(|e| e.to_string())?
                    .is_some();
            if resolvable {
                parents_linked += 1;
            }
        }
    }

    Ok(ImportNcbiTaxonomyResult {
        imported,
        updated,
        skipped_overrides,
        conflicts,
        skipped_records,
        parents_linked,
        dry_run: request.dry_run,
    })
}

// ── resolve_ncbi_conflict ─────────────────────────────────────────────────────

/// Apply an admin-chosen resolution to a conflict logged in `ncbi_sync_log`.
///
/// - `kept_local`: marks the conflict resolved without modifying the taxon.
/// - `accepted_ncbi`: applies the NCBI values (name / rank) stored in
///   `conflict_details` to the local taxon row.
/// - `merged`: marks resolved without automatic field changes; the admin is
///   expected to have manually edited the taxon beforehand.
#[tauri::command]
pub fn resolve_ncbi_conflict(
    state: State<AppState>,
    token: String,
    request: ResolveNcbiConflictRequest,
) -> Result<(), String> {
    let db = state.db();
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.is_admin() {
        return Err("Only admins can resolve NCBI taxonomy conflicts".to_string());
    }

    let valid_resolutions = ["kept_local", "accepted_ncbi", "merged"];
    if !valid_resolutions.contains(&request.resolution.as_str()) {
        return Err(format!(
            "Invalid resolution '{}'. Must be one of: {}",
            request.resolution,
            valid_resolutions.join(", ")
        ));
    }

    // Load the sync log entry.
    let (taxon_id, conflict_details): (Option<String>, Option<String>) = db
        .conn
        .query_row(
            "SELECT taxon_id, conflict_details FROM ncbi_sync_log WHERE id = ?1",
            params![request.sync_log_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|e| format!("Sync log entry not found: {}", e))?;

    // Check it isn't already resolved.
    let already_resolved: bool = db
        .conn
        .query_row(
            "SELECT resolved_at IS NOT NULL FROM ncbi_sync_log WHERE id = ?1",
            params![request.sync_log_id],
            |r| r.get(0),
        )
        .unwrap_or(false);
    if already_resolved {
        return Err("This conflict has already been resolved".to_string());
    }

    if request.resolution == "accepted_ncbi" {
        if let (Some(ref tid), Some(ref details)) = (&taxon_id, &conflict_details) {
            apply_ncbi_values(&db.conn, tid, details)?;
        }
    }

    let now = chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string();

    db.conn
        .execute(
            "UPDATE ncbi_sync_log
             SET resolved_at = ?1, resolved_by = ?2, resolution = ?3
             WHERE id = ?4",
            params![now, user.id, request.resolution, request.sync_log_id],
        )
        .map_err(|e| format!("Failed to update sync log: {}", e))?;

    Ok(())
}

/// Apply NCBI field values (name, rank) from a conflict_details JSON to a taxon row.
fn apply_ncbi_values(
    conn: &rusqlite::Connection,
    taxon_id: &str,
    conflict_details: &str,
) -> Result<(), String> {
    let details: serde_json::Value =
        serde_json::from_str(conflict_details).map_err(|e| e.to_string())?;

    let mut updates: Vec<String> = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(name_ncbi) = details
        .get("name")
        .and_then(|v| v.get("ncbi"))
        .and_then(|v| v.as_str())
    {
        updates.push(format!("name = ?{}", values.len() + 1));
        values.push(Box::new(name_ncbi.to_string()));
    }
    if let Some(rank_ncbi) = details
        .get("rank")
        .and_then(|v| v.get("ncbi"))
        .and_then(|v| v.as_str())
    {
        updates.push(format!("rank = ?{}", values.len() + 1));
        values.push(Box::new(rank_ncbi.to_string()));
    }

    if updates.is_empty() {
        return Ok(());
    }

    updates.push("updated_at = datetime('now')".to_string());
    let sql = format!(
        "UPDATE taxa SET {} WHERE id = ?{}",
        updates.join(", "),
        values.len() + 1
    );
    values.push(Box::new(taxon_id.to_string()));

    let bind_refs: Vec<&dyn rusqlite::types::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    conn.execute(&sql, bind_refs.as_slice())
        .map_err(|e| format!("Failed to apply NCBI values to taxon '{}': {}", taxon_id, e))?;

    Ok(())
}

// ── sync_ncbi_taxon ───────────────────────────────────────────────────────────

/// Re-process a single NCBI taxon record and update the local taxa table if no
/// conflict exists.  Conflicts are logged to `ncbi_sync_log` for later resolution.
///
/// The caller provides the NCBI taxon data (already fetched from NCBI).
/// Taxa with `local_override = true` are never modified.
#[tauri::command]
pub fn sync_ncbi_taxon(
    state: State<AppState>,
    token: String,
    record: NcbiTaxonRecord,
) -> Result<String, String> {
    let db = state.db();
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.is_admin() {
        return Err("Only admins can sync NCBI taxonomy data".to_string());
    }

    let rank = queries::normalize_ncbi_rank(&record.rank)
        .ok_or_else(|| format!("Unsupported NCBI rank: '{}'", record.rank))?;

    let now = chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string();

    // Try to locate the existing local taxon.
    let existing = queries::find_taxon_by_ncbi_id(&db.conn, record.ncbi_taxon_id)
        .map_err(|e| e.to_string())?;

    let log_id = uuid::Uuid::new_v4().to_string();

    if let Some(local) = existing {
        if local.local_override {
            return Ok(format!(
                "Taxon '{}' (ID {}) has local_override=true and was not modified.",
                local.name, local.id
            ));
        }

        if let Some(details) =
            queries::detect_ncbi_conflict(&local, &record.name, rank)
        {
            queries::insert_ncbi_sync_log(
                &db.conn,
                &log_id,
                "conflict",
                Some(&local.id),
                Some(record.ncbi_taxon_id),
                Some(&details),
                &now,
            )
            .map_err(|e| e.to_string())?;
            return Ok(format!(
                "Conflict detected for taxon '{}' (NCBI ID {}) — logged as {}.",
                local.name, record.ncbi_taxon_id, log_id
            ));
        }

        // No conflict — update ncbi_updated_at.
        db.conn
            .execute(
                "UPDATE taxa SET ncbi_updated_at = ?1, updated_at = datetime('now') \
                 WHERE id = ?2",
                params![now, local.id],
            )
            .map_err(|e| format!("Failed to update taxon '{}': {}", local.id, e))?;
        queries::insert_ncbi_sync_log(
            &db.conn,
            &log_id,
            "update",
            Some(&local.id),
            Some(record.ncbi_taxon_id),
            None,
            &now,
        )
        .map_err(|e| e.to_string())?;
        return Ok(format!(
            "Taxon '{}' synced successfully (no changes needed).",
            local.name
        ));
    }

    // No local taxon found — create one.
    let new_id = uuid::Uuid::new_v4().to_string();
    let path = format!("[\"{}\"]", new_id);
    db.conn
        .execute(
            "INSERT INTO taxa (id, rank, name, ncbi_taxon_id, ncbi_updated_at,
             local_override, taxon_path)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6)",
            params![new_id, rank, record.name, record.ncbi_taxon_id, now, path],
        )
        .map_err(|e| format!("Failed to create taxon '{}': {}", record.name, e))?;
    queries::insert_ncbi_sync_log(
        &db.conn,
        &log_id,
        "import",
        Some(&new_id),
        Some(record.ncbi_taxon_id),
        None,
        &now,
    )
    .map_err(|e| e.to_string())?;

    Ok(format!(
        "Taxon '{}' (NCBI ID {}) imported as new taxon {}.",
        record.name, record.ncbi_taxon_id, new_id
    ))
}

// ── list_ncbi_sync_log ────────────────────────────────────────────────────────

/// Return recent entries from `ncbi_sync_log`, newest first.
/// Pass `pending_only = true` to show only unresolved conflicts.
#[tauri::command]
pub fn list_ncbi_sync_log(
    state: State<AppState>,
    token: String,
    pending_only: bool,
    limit: Option<i64>,
) -> Result<Vec<NcbiSyncLog>, String> {
    let db = state.db();
    let _user = auth_service::validate_session(&db, &token)?;

    let cap = limit.unwrap_or(200).clamp(1, 1000);

    if pending_only {
        queries::list_pending_ncbi_conflicts(&db.conn).map_err(|e| e.to_string())
    } else {
        queries::list_ncbi_sync_log(&db.conn, cap).map_err(|e| e.to_string())
    }
}

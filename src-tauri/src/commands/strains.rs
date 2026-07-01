use crate::auth as auth_service;
use crate::db::queries;
use crate::models::strain::{
    CreateHybridizationEventRequest, CreateStrainRequest, GenerationalStats, HybridizationResult,
    PedigreeExport, PedigreeNode, Strain, StrainSpecimenTree, SuggestGenerationLabelResponse,
    UpdateStrainRequest, UpdateStrainStatusRequest,
};
use crate::AppState;
use rusqlite::params;
use tauri::State;


// ── internal read helper ───────────────────────────────────────────────────────

fn load_strain(conn: &rusqlite::Connection, id: &str) -> Result<Strain, String> {
    conn.query_row(
        "SELECT s.*, \
                (SELECT COUNT(*) FROM specimens sp WHERE sp.strain_id = s.id AND sp.is_archived = 0) \
                AS specimen_count \
         FROM strains s WHERE s.id = ?1",
        params![id],
        row_to_strain,
    )
    .map_err(|e| format!("Strain not found: {}", e))
}

fn row_to_strain(row: &rusqlite::Row<'_>) -> rusqlite::Result<Strain> {
    Ok(Strain {
        id: row.get("id")?,
        species_id: row.get("species_id")?,
        name: row.get("name")?,
        code: row.get("code")?,
        strain_type: row.get("strain_type")?,
        status: row.get("status")?,
        claimed_by: row.get("claimed_by")?,
        claimed_at: row.get("claimed_at")?,
        confirmation_basis: row.get("confirmation_basis")?,
        genomic_fingerprint: row.get("genomic_fingerprint")?,
        is_hybrid: row.get::<_, i32>("is_hybrid")? != 0,
        is_archived: row.get::<_, i32>("is_archived")? != 0,
        is_cross_species: row.get::<_, Option<i32>>("is_cross_species")?.unwrap_or(0) != 0,
        archived_at: row.get("archived_at")?,
        created_by: row.get("created_by")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        specimen_count: row.get("specimen_count").ok(),
    })
}

// ── cycle detection ────────────────────────────────────────────────────────────

fn is_ancestor(
    conn: &rusqlite::Connection,
    candidate_ancestor: &str,
    of_strain: &str,
    visited: &mut Vec<String>,
) -> bool {
    if candidate_ancestor == of_strain {
        return true;
    }
    if visited.iter().any(|v| v == of_strain) {
        return false;
    }
    visited.push(of_strain.to_string());

    let mut stmt = match conn.prepare(
        "SELECT parent_strain_id FROM strain_parents WHERE strain_id = ?1",
    ) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let parents: Vec<String> = match stmt.query_map(params![of_strain], |r| r.get(0)) {
        Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
        Err(_) => Vec::new(),
    };

    for parent_id in parents {
        if is_ancestor(conn, candidate_ancestor, &parent_id, visited) {
            return true;
        }
    }
    false
}

fn check_no_cycle(
    conn: &rusqlite::Connection,
    strain_a: &str,
    strain_b: &str,
) -> Result<(), String> {
    let mut visited = Vec::new();
    if is_ancestor(conn, strain_a, strain_b, &mut visited) {
        return Err(format!(
            "Cycle detected: strain '{}' is an ancestor of '{}'",
            strain_a, strain_b
        ));
    }
    let mut visited = Vec::new();
    if is_ancestor(conn, strain_b, strain_a, &mut visited) {
        return Err(format!(
            "Cycle detected: strain '{}' is an ancestor of '{}'",
            strain_b, strain_a
        ));
    }
    Ok(())
}

// ── commands ───────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn create_strain(
    state: State<AppState>,
    token: String,
    request: CreateStrainRequest,
) -> Result<Strain, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }

    // Verify species exists.
    let _: String = db
        .conn
        .query_row(
            "SELECT id FROM species WHERE id = ?1",
            params![request.species_id],
            |r| r.get(0),
        )
        .map_err(|_| "Species not found".to_string())?;

    let id = uuid::Uuid::new_v4().to_string();
    let strain_type = request.strain_type.unwrap_or_else(|| "wildtype".to_string());

    let tx = db
        .conn
        .unchecked_transaction()
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    tx.execute(
        "INSERT INTO strains (id, species_id, name, code, strain_type, created_by)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, request.species_id, request.name, request.code, strain_type, user.id],
    )
    .map_err(|e| format!("Failed to create strain: {}", e))?;

    // Genesis audit entry: chain_seq = 0, prev_hash = species' current entry_hash.
    queries::log_audit_strain_genesis(
        &tx,
        Some(&user.id),
        "create",
        "strain",
        Some(&id),
        None,
        Some(&request.name),
        Some("Strain created"),
        &request.species_id,
    )
    .map_err(|e| format!("Failed to write audit entry: {}", e))?;

    tx.commit()
        .map_err(|e| format!("Failed to commit strain: {}", e))?;

    drop(db);
    get_strain(state, token, id)
}

/// WP-55: masks `genomic_fingerprint` per the calling user's role. Applied
/// only here (the read-response construction), never at the DB/audit level.
/// Takes a pre-loaded [`crate::db::permissions::FieldPermissionSet`] rather
/// than querying per call — `list_strains_by_species` used to call this once
/// per row, issuing one identical `field_permissions` query per strain in
/// the list (an N+1 pattern fixed alongside the corruption bug below).
fn apply_field_permissions(perms: &crate::db::permissions::FieldPermissionSet, mut strain: Strain) -> Strain {
    strain.genomic_fingerprint =
        perms.mask_optional_field("strain", "genomic_fingerprint", strain.genomic_fingerprint);
    strain
}

#[tauri::command]
pub fn get_strain(
    state: State<AppState>,
    token: String,
    id: String,
) -> Result<Strain, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    let strain = load_strain(&db.conn, &id)?;
    let perms = crate::db::permissions::FieldPermissionSet::load(&db.conn, user.role.as_str())
        .map_err(|e| e.to_string())?;
    Ok(apply_field_permissions(&perms, strain))
}

#[tauri::command]
pub fn list_strains_by_species(
    state: State<AppState>,
    token: String,
    species_id: String,
) -> Result<Vec<Strain>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;

    let mut stmt = db
        .conn
        .prepare(
            "SELECT s.*, \
                    (SELECT COUNT(*) FROM specimens sp \
                     WHERE sp.strain_id = s.id AND sp.is_archived = 0) AS specimen_count \
             FROM strains s \
             WHERE s.species_id = ?1 AND s.is_archived = 0 \
             ORDER BY s.name ASC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params![species_id], row_to_strain)
        .map_err(|e| e.to_string())?;

    // Loaded once for the whole list, not once per row — see the doc comment
    // on `apply_field_permissions`.
    let perms = crate::db::permissions::FieldPermissionSet::load(&db.conn, user.role.as_str())
        .map_err(|e| e.to_string())?;
    let strains: Vec<Strain> = rows
        .filter_map(|r| r.ok())
        .map(|s| apply_field_permissions(&perms, s))
        .collect();
    Ok(strains)
}

#[tauri::command]
pub fn update_strain(
    state: State<AppState>,
    token: String,
    request: UpdateStrainRequest,
) -> Result<Strain, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }

    let tx = db
        .conn
        .unchecked_transaction()
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    if let Some(ref name) = request.name {
        tx.execute(
            "UPDATE strains SET name = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![name, request.id],
        )
        .map_err(|e| e.to_string())?;
    }
    if let Some(ref code) = request.code {
        tx.execute(
            "UPDATE strains SET code = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![code, request.id],
        )
        .map_err(|e| e.to_string())?;
    }
    if let Some(ref strain_type) = request.strain_type {
        tx.execute(
            "UPDATE strains SET strain_type = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![strain_type, request.id],
        )
        .map_err(|e| e.to_string())?;
    }

    queries::log_audit(
        &tx,
        Some(&user.id),
        "update",
        "strain",
        Some(&request.id),
        None,
        None,
        Some("Strain updated"),
    )
    .map_err(|e| format!("Failed to write audit entry: {}", e))?;

    tx.commit()
        .map_err(|e| format!("Failed to commit strain update: {}", e))?;

    drop(db);
    get_strain(state, token, request.id)
}

#[tauri::command]
pub fn archive_strain(
    state: State<AppState>,
    token: String,
    id: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }

    db.conn
        .execute(
            "UPDATE strains SET is_archived = 1, archived_at = datetime('now'), \
             updated_at = datetime('now') WHERE id = ?1",
            params![id],
        )
        .map_err(|e| format!("Failed to archive strain: {}", e))?;

    queries::log_audit(
        &db.conn,
        Some(&user.id),
        "archive",
        "strain",
        Some(&id),
        None,
        None,
        Some("Strain archived"),
    )
    .ok();

    Ok(())
}

#[tauri::command]
pub fn update_strain_status(
    state: State<AppState>,
    token: String,
    request: UpdateStrainStatusRequest,
) -> Result<Strain, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }

    let current_status: String = db
        .conn
        .query_row(
            "SELECT status FROM strains WHERE id = ?1",
            params![request.id],
            |r| r.get(0),
        )
        .map_err(|_| "Strain not found".to_string())?;

    // WP-55: validation, the write-path guard against a masked
    // "[RESTRICTED]" genomic_fingerprint round-tripping back into the
    // database, and the actual UPDATE all live in
    // `queries::apply_strain_status_update` so they can be unit-tested
    // directly. See that function's doc comment for details.
    queries::apply_strain_status_update(
        &db.conn,
        &request.id,
        &current_status,
        &request.status,
        request.claimed_by.as_deref(),
        request.claimed_at.as_deref(),
        request.confirmation_basis.as_deref(),
        request.genomic_fingerprint.as_deref(),
    )?;

    queries::log_audit(
        &db.conn,
        Some(&user.id),
        "status_change",
        "strain",
        Some(&request.id),
        Some(&current_status),
        Some(&request.status),
        Some("Strain status updated"),
    )
    .map_err(|e| format!("Failed to write audit entry: {}", e))?;

    drop(db);
    get_strain(state, token, request.id)
}

#[tauri::command]
pub fn create_hybridization_event(
    state: State<AppState>,
    token: String,
    request: CreateHybridizationEventRequest,
) -> Result<HybridizationResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }

    // Load both parent strains.
    let parent_a = load_strain(&db.conn, &request.parent_a_id)?;
    let parent_b = load_strain(&db.conn, &request.parent_b_id)?;

    let is_cross_species = parent_a.species_id != parent_b.species_id;

    if is_cross_species {
        // Guard: cross-species hybridization requires an explicit admin override.
        if !request.admin_override_cross_species.unwrap_or(false) {
            return Err(
                "Cross-species hybridization is not permitted: parent strains must belong to the same species"
                    .to_string(),
            );
        }
        if !user.role.is_admin() {
            return Err(
                "Cross-species hybridization override requires administrator privileges"
                    .to_string(),
            );
        }
        let reason = request
            .admin_override_reason
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_string();
        if reason.is_empty() {
            return Err(
                "Cross-species override requires a documented reason".to_string(),
            );
        }
    }

    // Cycle detection before any writes.
    check_no_cycle(&db.conn, &request.parent_a_id, &request.parent_b_id)?;

    // Detect backcross (one parent is an ancestor of the other).
    let backcross = queries::detect_backcross(
        &db.conn,
        &request.parent_a_id,
        &request.parent_b_id,
    );
    let backcross_depth: Option<i64> = backcross.as_ref().map(|(_, d)| i64::from(*d));

    // Resolve generation label: explicit > backcross suggestion > parent-label suggestion.
    let generation_label: Option<String> = {
        let explicit = request
            .generation_label
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        if explicit.is_some() {
            explicit
        } else if let Some((_, depth)) = &backcross {
            Some(format!("BC{}F1", depth))
        } else {
            let label_a =
                queries::get_strain_generation_label(&db.conn, &request.parent_a_id);
            let label_b =
                queries::get_strain_generation_label(&db.conn, &request.parent_b_id);
            queries::suggest_generation_label(label_a.as_deref(), label_b.as_deref())
        }
    };

    // Use parent_a's species for the hybrid record (for cross-species hybrids, the
    // is_cross_species flag makes the origin unambiguous).
    let species_id = parent_a.species_id.clone();

    // Snapshot parent chain_seqs (before the transaction writes anything).
    let parent_a_chain_seq: i64 = db
        .conn
        .query_row(
            "SELECT COALESCE(MAX(chain_seq), 0) FROM audit_log \
             WHERE lineage_id = ?1 AND entry_hash IS NOT NULL",
            params![request.parent_a_id],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let parent_b_chain_seq: i64 = db
        .conn
        .query_row(
            "SELECT COALESCE(MAX(chain_seq), 0) FROM audit_log \
             WHERE lineage_id = ?1 AND entry_hash IS NOT NULL",
            params![request.parent_b_id],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let hybrid_id = uuid::Uuid::new_v4().to_string();
    let event_id = uuid::Uuid::new_v4().to_string();
    let parent_a_id = request.parent_a_id.clone();
    let parent_b_id = request.parent_b_id.clone();

    let tx = db
        .conn
        .unchecked_transaction()
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    // 1. Create hybrid strain record.
    tx.execute(
        "INSERT INTO strains \
         (id, species_id, name, code, strain_type, is_hybrid, is_cross_species, created_by) \
         VALUES (?1, ?2, ?3, ?4, 'hybrid', 1, ?5, ?6)",
        params![
            hybrid_id,
            species_id,
            request.name,
            request.code,
            if is_cross_species { 1i32 } else { 0i32 },
            user.id
        ],
    )
    .map_err(|e| format!("Failed to create hybrid strain: {}", e))?;

    // 2. Hybrid genesis audit entry (chain_seq = 0, prev_hash = species entry_hash).
    queries::log_audit_strain_genesis(
        &tx,
        Some(&user.id),
        "create",
        "strain",
        Some(&hybrid_id),
        None,
        Some(&request.name),
        Some("Hybrid strain genesis"),
        &species_id,
    )
    .map_err(|e| format!("Failed to write hybrid genesis audit: {}", e))?;

    // 3. Hybrid "hybridize" audit entry (chain_seq = 1).
    let gen_label_detail = generation_label
        .as_deref()
        .map(|l| format!("Hybridization event recorded [{}]", l))
        .unwrap_or_else(|| "Hybridization event recorded".to_string());
    queries::log_audit(
        &tx,
        Some(&user.id),
        "hybridize",
        "strain",
        Some(&hybrid_id),
        None,
        None,
        Some(&gen_label_detail),
    )
    .map_err(|e| format!("Failed to write hybridize audit: {}", e))?;

    // 3a. Permanent cross-species override warning in the audit log.
    if is_cross_species {
        let reason = request
            .admin_override_reason
            .as_deref()
            .unwrap_or("no reason provided");
        let warning = format!(
            "CROSS-SPECIES OVERRIDE: admin '{}' authorised cross-species hybridization. Reason: {}",
            user.username, reason
        );
        queries::log_audit(
            &tx,
            Some(&user.id),
            "cross_species_override",
            "strain",
            Some(&hybrid_id),
            None,
            None,
            Some(&warning),
        )
        .map_err(|e| format!("Failed to write cross-species override audit: {}", e))?;
    }

    // 4. Two strain_parents records.
    let sp_a_id = uuid::Uuid::new_v4().to_string();
    tx.execute(
        "INSERT INTO strain_parents (id, strain_id, parent_strain_id, parent_role, parent_chain_seq_at_creation) \
         VALUES (?1, ?2, ?3, 'parent_a', ?4)",
        params![sp_a_id, hybrid_id, parent_a_id, parent_a_chain_seq],
    )
    .map_err(|e| format!("Failed to insert strain_parent A: {}", e))?;

    let sp_b_id = uuid::Uuid::new_v4().to_string();
    tx.execute(
        "INSERT INTO strain_parents (id, strain_id, parent_strain_id, parent_role, parent_chain_seq_at_creation) \
         VALUES (?1, ?2, ?3, 'parent_b', ?4)",
        params![sp_b_id, hybrid_id, parent_b_id, parent_b_chain_seq],
    )
    .map_err(|e| format!("Failed to insert strain_parent B: {}", e))?;

    // 5. hybridization_events record (now with generation_label and backcross_depth).
    tx.execute(
        "INSERT INTO hybridization_events \
         (id, hybrid_strain_id, parent_a_strain_id, parent_b_strain_id, \
          parent_a_chain_seq, parent_b_chain_seq, notes, generation_label, backcross_depth, created_by) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            event_id,
            hybrid_id,
            parent_a_id,
            parent_b_id,
            parent_a_chain_seq,
            parent_b_chain_seq,
            request.notes,
            generation_label,
            backcross_depth,
            user.id
        ],
    )
    .map_err(|e| format!("Failed to create hybridization event: {}", e))?;

    // 6. used_as_parent entry on parent A's chain.
    queries::log_audit(
        &tx,
        Some(&user.id),
        "used_as_parent",
        "strain",
        Some(&parent_a_id),
        None,
        None,
        Some(&format!("Used as parent in hybridization to produce '{}'", request.name)),
    )
    .map_err(|e| format!("Failed to write parent A audit: {}", e))?;

    // 7. used_as_parent entry on parent B's chain.
    queries::log_audit(
        &tx,
        Some(&user.id),
        "used_as_parent",
        "strain",
        Some(&parent_b_id),
        None,
        None,
        Some(&format!("Used as parent in hybridization to produce '{}'", request.name)),
    )
    .map_err(|e| format!("Failed to write parent B audit: {}", e))?;

    tx.commit()
        .map_err(|e| format!("Failed to commit hybridization event: {}", e))?;

    Ok(HybridizationResult {
        hybrid_strain_id: hybrid_id,
        event_id,
    })
}

/// Suggest a generation label for two prospective parents.
///
/// Runs backcross detection (which takes priority) and falls back to
/// symmetric parent-label rules.  Returns immediately without writes.
#[tauri::command]
pub fn suggest_generation_label(
    state: State<AppState>,
    token: String,
    parent_a_id: String,
    parent_b_id: String,
) -> Result<SuggestGenerationLabelResponse, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    Ok(queries::suggest_generation_label_for_parents(
        &db.conn,
        &parent_a_id,
        &parent_b_id,
    ))
}

/// Return per-generation specimen statistics for direct hybrid descendants of
/// a strain, grouped by generation label.
#[tauri::command]
pub fn get_generational_stats(
    state: State<AppState>,
    token: String,
    strain_id: String,
) -> Result<Vec<GenerationalStats>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    queries::get_generational_stats(&db.conn, &strain_id).map_err(|e| e.to_string())
}

// ── Pedigree commands (WP-37) ─────────────────────────────────────────────────

/// Return the full ancestry tree of a strain, walking upward through
/// `strain_parents`.  `max_depth` defaults to 5 and is capped at 10.
#[tauri::command]
pub fn get_strain_ancestry(
    state: State<AppState>,
    token: String,
    strain_id: String,
    max_depth: Option<u32>,
) -> Result<PedigreeNode, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    let cap = queries::configured_pedigree_max_depth(&db.conn);
    let depth = max_depth.unwrap_or(5).min(cap);
    queries::get_strain_ancestry(&db.conn, &strain_id, depth).map_err(|e| e.to_string())
}

/// Return the full descendant tree of a strain, walking downward through
/// `strain_parents`.  `max_depth` defaults to 5 and is capped at 10.
#[tauri::command]
pub fn get_strain_descendants(
    state: State<AppState>,
    token: String,
    strain_id: String,
    max_depth: Option<u32>,
) -> Result<PedigreeNode, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    let cap = queries::configured_pedigree_max_depth(&db.conn);
    let depth = max_depth.unwrap_or(5).min(cap);
    queries::get_strain_descendants(&db.conn, &strain_id, depth).map_err(|e| e.to_string())
}

/// Return all live specimens bound to a strain.  When `include_descendants` is
/// true, specimens bound to descendant hybrid strains are also included.
#[tauri::command]
pub fn get_strain_specimen_tree(
    state: State<AppState>,
    token: String,
    strain_id: String,
    include_descendants: bool,
) -> Result<StrainSpecimenTree, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    queries::get_strain_specimen_tree(&db.conn, &strain_id, include_descendants)
        .map_err(|e| e.to_string())
}

/// Export the full pedigree of a strain as a portable JSON bundle containing
/// all reachable strains and hybridization events.
#[tauri::command]
pub fn export_strain_pedigree(
    state: State<AppState>,
    token: String,
    strain_id: String,
    max_depth: Option<u32>,
) -> Result<PedigreeExport, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    let cap = queries::configured_pedigree_max_depth(&db.conn);
    let depth = max_depth.unwrap_or(5).min(cap);
    queries::export_strain_pedigree(&db.conn, &strain_id, depth).map_err(|e| e.to_string())
}

/// Returns the lab's configured pedigree traversal depth cap (1–20, default 10).
#[tauri::command]
pub fn get_pedigree_max_depth(state: State<AppState>, token: String) -> Result<u32, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    Ok(queries::configured_pedigree_max_depth(&db.conn))
}

/// Sets the lab's configured pedigree traversal depth cap. Admin only.
/// Clamped to [1, 20] regardless of the requested value.
#[tauri::command]
pub fn set_pedigree_max_depth(
    state: State<AppState>,
    token: String,
    max_depth: u32,
) -> Result<u32, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.is_admin() {
        return Err("Only admins can change the pedigree depth limit".to_string());
    }
    let clamped = max_depth.clamp(1, 20);
    db.conn.execute(
        "INSERT INTO app_settings (key, value, updated_at) VALUES ('pedigree_max_depth', ?1, datetime('now')) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        rusqlite::params![clamped.to_string()],
    ).map_err(|e| e.to_string())?;
    Ok(clamped)
}


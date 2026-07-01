use crate::auth as auth_service;
use crate::db::queries;
use crate::models::taxon::{
    CreateProvisionalTaxonRequest, CreateTaxonMappingRequest, CreateTaxonRequest, DarwinCoreExport,
    SpeciesNodeSummary, Taxon, TaxonColumnItem, TaxonMapping, TaxonNode, TaxonomySearchResult,
    UpdateTaxonRequest,
};
use crate::AppState;
use rusqlite::params;
use tauri::State;

fn build_taxon_node(conn: &rusqlite::Connection, taxon: Taxon) -> Result<TaxonNode, String> {
    let species = queries::get_species_for_taxon(conn, &taxon.id)
        .map_err(|e| e.to_string())?;
    let children_taxa = queries::get_child_taxa(conn, &taxon.id)
        .map_err(|e| e.to_string())?;

    let own_strain_count: i64 = species.iter().map(|s| s.strain_count).sum();
    let own_specimen_count: i64 = species.iter().map(|s| s.specimen_count).sum();

    let mut children = Vec::new();
    let mut child_strain_count: i64 = 0;
    let mut child_specimen_count: i64 = 0;

    for child_taxon in children_taxa {
        let node = build_taxon_node(conn, child_taxon)?;
        child_strain_count += node.strain_count;
        child_specimen_count += node.specimen_count;
        children.push(node);
    }

    Ok(TaxonNode {
        taxon,
        strain_count: own_strain_count + child_strain_count,
        specimen_count: own_specimen_count + child_specimen_count,
        species,
        children,
    })
}

#[tauri::command]
pub fn create_taxon(
    state: State<AppState>,
    token: String,
    request: CreateTaxonRequest,
) -> Result<Taxon, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err("Only supervisors and admins can manage taxonomy".to_string());
    }

    let valid_ranks = ["kingdom", "phylum", "class", "order", "family", "genus"];
    if !valid_ranks.contains(&request.rank.as_str()) {
        return Err(format!(
            "Invalid rank '{}'. Must be one of: {}",
            request.rank,
            valid_ranks.join(", ")
        ));
    }

    let id = uuid::Uuid::new_v4().to_string();

    // Build taxon_path: parent's path with this node's ID appended.
    let taxon_path = if let Some(ref parent_id) = request.parent_id {
        let parent_path: Option<String> = db
            .conn
            .query_row(
                "SELECT taxon_path FROM taxa WHERE id = ?1",
                params![parent_id],
                |r| r.get(0),
            )
            .map_err(|e| format!("Parent taxon not found: {}", e))?;

        let mut arr: Vec<String> = parent_path
            .as_deref()
            .and_then(|p| serde_json::from_str::<Vec<String>>(p).ok())
            .unwrap_or_default();
        arr.push(id.clone());
        serde_json::to_string(&arr).map_err(|e| e.to_string())?
    } else {
        format!("[\"{}\"]", id)
    };

    let local_override = request.local_override.unwrap_or(false) as i64;

    db.conn
        .execute(
            "INSERT INTO taxa (id, rank, name, parent_id, ncbi_taxon_id, local_override, taxon_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                id,
                request.rank,
                request.name,
                request.parent_id,
                request.ncbi_taxon_id,
                local_override,
                taxon_path
            ],
        )
        .map_err(|e| format!("Failed to create taxon: {}", e))?;

    // EXPERIMENTAL (WP-45): Write a genesis audit entry for the new taxon, anchoring
    // it to the parent taxon's hash chain. This begins the full Kingdom → … → Genus
    // cryptographic provenance chain. Reclassifying a taxon after this point will break
    // the chain for all descendants — see ROADMAP.md §WP-45.
    queries::log_audit_taxon_genesis(
        &db.conn,
        Some(&user.id),
        "create",
        "taxon",
        Some(&id),
        None,
        Some(&request.name),
        None,
        request.parent_id.as_deref(),
    )
    .ok();

    queries::load_taxon(&db.conn, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_taxon(state: State<AppState>, token: String, id: String) -> Result<Taxon, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    queries::load_taxon(&db.conn, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_taxon(
    state: State<AppState>,
    token: String,
    request: UpdateTaxonRequest,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err("Only supervisors and admins can manage taxonomy".to_string());
    }

    let mut updates: Vec<String> = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(ref name) = request.name {
        updates.push(format!("name = ?{}", values.len() + 1));
        values.push(Box::new(name.clone()));
    }
    if let Some(ref parent_id) = request.parent_id {
        updates.push(format!("parent_id = ?{}", values.len() + 1));
        values.push(Box::new(parent_id.clone()));
    }
    if let Some(ncbi_id) = request.ncbi_taxon_id {
        updates.push(format!("ncbi_taxon_id = ?{}", values.len() + 1));
        values.push(Box::new(ncbi_id));
    }
    if let Some(ref ts) = request.ncbi_updated_at {
        updates.push(format!("ncbi_updated_at = ?{}", values.len() + 1));
        values.push(Box::new(ts.clone()));
    }
    if let Some(lo) = request.local_override {
        updates.push(format!("local_override = ?{}", values.len() + 1));
        values.push(Box::new(lo as i64));
    }

    if updates.is_empty() {
        return Err("No fields to update".to_string());
    }

    updates.push("updated_at = datetime('now')".to_string());
    let sql = format!(
        "UPDATE taxa SET {} WHERE id = ?{}",
        updates.join(", "),
        values.len() + 1
    );
    values.push(Box::new(request.id.clone()));

    let bind_refs: Vec<&dyn rusqlite::types::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    db.conn
        .execute(&sql, bind_refs.as_slice())
        .map_err(|e| format!("Failed to update taxon: {}", e))?;

    // EXPERIMENTAL (WP-45): Append an update entry to the taxon's audit chain.
    //
    // RECLASSIFICATION WARNING: If `name`, `rank`, or `parent_id` changed, this update
    // advances the taxon's chain but does NOT re-anchor any descendant chains. All strains
    // and specimens whose genesis prev_hash was derived from this taxon's previous
    // entry_hash will remain cryptographically bound to the OLD classification.
    // There is currently no automated re-anchoring tool. See ROADMAP.md §WP-45.
    queries::log_audit(
        &db.conn,
        Some(&user.id),
        "update",
        "taxon",
        Some(&request.id),
        None,
        None,
        None,
    )
    .ok();

    Ok(())
}

#[tauri::command]
pub fn list_taxa_by_rank(
    state: State<AppState>,
    token: String,
    rank: String,
) -> Result<Vec<Taxon>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;

    let mut stmt = db
        .conn
        .prepare(
            "SELECT id, rank, name, parent_id, ncbi_taxon_id, ncbi_updated_at,
                    local_override, taxon_path, created_at, updated_at
             FROM taxa WHERE rank = ?1 ORDER BY name",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params![rank], |row| {
            Ok(Taxon {
                id: row.get("id")?,
                rank: row.get("rank")?,
                name: row.get("name")?,
                parent_id: row.get("parent_id")?,
                ncbi_taxon_id: row.get("ncbi_taxon_id")?,
                ncbi_updated_at: row.get("ncbi_updated_at")?,
                local_override: row.get::<_, i64>("local_override")? != 0,
                taxon_path: row.get("taxon_path")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        })
        .map_err(|e| e.to_string())?;

    let items: Result<Vec<_>, _> = rows.collect();
    items.map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_taxon_descendants(
    state: State<AppState>,
    token: String,
    id: String,
) -> Result<TaxonNode, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;

    let taxon = queries::load_taxon(&db.conn, &id).map_err(|e| e.to_string())?;
    build_taxon_node(&db.conn, taxon)
}

/// WP-39: Return immediate children of a taxon (or all kingdom-level taxa when
/// `parent_id` is `None`), each with aggregated descendant counts.
#[tauri::command]
pub fn get_taxon_column(
    state: State<AppState>,
    token: String,
    parent_id: Option<String>,
) -> Result<Vec<TaxonColumnItem>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    queries::get_taxon_column_items(&db.conn, parent_id.as_deref())
        .map_err(|e| e.to_string())
}

/// WP-39: Return species whose most-specific ancestor is the given taxon.
/// Reuses the existing `get_species_for_taxon` helper (which matches the last
/// element of taxon_path) so genus-level navigation returns only the species
/// directly classified under that genus.
#[tauri::command]
pub fn list_species_for_taxon(
    state: State<AppState>,
    token: String,
    taxon_id: String,
) -> Result<Vec<SpeciesNodeSummary>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    queries::get_species_for_taxon(&db.conn, &taxon_id).map_err(|e| e.to_string())
}

/// WP-39: Search taxa, species, strains, and specimens by name / code /
/// accession. Returns up to 10 hits per entity type. Queries shorter than
/// 2 characters return an empty result without hitting the database.
#[tauri::command]
pub fn search_taxonomy(
    state: State<AppState>,
    token: String,
    query: String,
) -> Result<Vec<TaxonomySearchResult>, String> {
    if query.len() < 2 {
        return Ok(vec![]);
    }
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    queries::search_taxonomy(&db.conn, &query).map_err(|e| e.to_string())
}

// ── WP-49: Provisional taxa & Darwin Core export ──────────────────────────────

/// Create a provisional (lab-internal) taxon.  Requires supervisor or admin role.
#[tauri::command]
pub fn create_provisional_taxon(
    state: State<AppState>,
    token: String,
    request: CreateProvisionalTaxonRequest,
) -> Result<Taxon, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err("Only supervisors and admins can create provisional taxa".to_string());
    }
    let id = uuid::Uuid::new_v4().to_string();
    queries::create_provisional_taxon(
        &db.conn,
        &id,
        &request.rank,
        &request.name,
        request.parent_id.as_deref(),
        request.provisional_notes.as_deref(),
        Some(&user.id),
    )
    .map_err(|e| e.to_string())
}

/// List all provisional taxa.
#[tauri::command]
pub fn list_provisional_taxa(
    state: State<AppState>,
    token: String,
) -> Result<Vec<Taxon>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    queries::list_provisional_taxa(&db.conn).map_err(|e| e.to_string())
}

/// Map a provisional taxon to an accepted NCBI taxon.  Requires supervisor or admin role.
#[tauri::command]
pub fn map_provisional_taxon(
    state: State<AppState>,
    token: String,
    request: CreateTaxonMappingRequest,
) -> Result<TaxonMapping, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err("Only supervisors and admins can map provisional taxa".to_string());
    }
    let id = uuid::Uuid::new_v4().to_string();
    queries::create_taxon_mapping(
        &db.conn,
        &id,
        &request.provisional_taxon_id,
        request.accepted_taxon_id.as_deref(),
        request.accepted_ncbi_id,
        request.accepted_name.as_deref(),
        request.notes.as_deref(),
        Some(&user.id),
    )
    .map_err(|e| e.to_string())
}

/// List all taxon mappings (provisional → accepted).
#[tauri::command]
pub fn list_taxon_mappings(
    state: State<AppState>,
    token: String,
) -> Result<Vec<TaxonMapping>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    queries::list_taxon_mappings(&db.conn).map_err(|e| e.to_string())
}

/// Export a taxonomy subtree (or the full taxonomy) as Darwin Core JSON.
/// Pass `root_id` to export a subtree; omit (or pass null) for the full taxonomy.
#[tauri::command]
pub fn export_darwin_core(
    state: State<AppState>,
    token: String,
    root_id: Option<String>,
) -> Result<DarwinCoreExport, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    queries::export_darwin_core(&db.conn, root_id.as_deref()).map_err(|e| e.to_string())
}

/// WP-64: read-only pre-flight report for `reanchor_taxon_chain` — exactly
/// what would be affected, without writing anything. Supervisor/admin only.
#[tauri::command]
pub fn reanchor_taxon_chain_dry_run(
    state: State<AppState>,
    token: String,
    taxon_id: String,
) -> Result<queries::ReanchorCounts, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err("Only supervisors and admins can preview a taxon re-anchor".to_string());
    }
    queries::reanchor_taxon_chain_dry_run(&db.conn, &taxon_id).map_err(|e| e.to_string())
}

/// WP-64: atomically re-anchors the hash chain for `taxon_id` and every
/// descendant taxon/species/strain (plus an aggregate specimen bridge)
/// following a taxonomic reclassification. Admin only. `reason` must be at
/// least 20 characters (also enforced in `db::queries::reanchor_taxon_chain`).
#[tauri::command]
pub fn reanchor_taxon_chain(
    state: State<AppState>,
    token: String,
    taxon_id: String,
    reason: String,
) -> Result<queries::ReanchorResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.is_admin() {
        return Err("Only admins can re-anchor a taxon's hash chain".to_string());
    }
    queries::reanchor_taxon_chain(&db.conn, &taxon_id, &user.id, &reason).map_err(|e| e.to_string())
}

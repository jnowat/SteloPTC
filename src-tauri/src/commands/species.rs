use crate::auth as auth_service;
use crate::db::queries;
use crate::models::species::*;
use crate::AppState;
use rusqlite::params;
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub status: String,
}

#[tauri::command]
pub fn list_species(state: State<AppState>, token: String) -> Result<Vec<Species>, String> {
    let db = state.db();
    let _user = auth_service::validate_session(&db, &token)?;

    let mut stmt = db.conn.prepare(
        "SELECT * FROM species ORDER BY genus, species_name"
    ).map_err(|e| e.to_string())?;

    let species = stmt.query_map([], |row| {
        Ok(Species {
            id: row.get("id")?,
            genus: row.get("genus")?,
            species_name: row.get("species_name")?,
            common_name: row.get("common_name")?,
            species_code: row.get("species_code")?,
            default_subculture_interval_days: row.get("default_subculture_interval_days")?,
            notes: row.get("notes")?,
            taxon_path: row.get("taxon_path")?,
            ncbi_taxon_id: row.get("ncbi_taxon_id")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }).map_err(|e| e.to_string())?
      .filter_map(|r| r.ok())
      .collect();

    Ok(species)
}

#[tauri::command]
pub fn create_species(
    state: State<AppState>,
    token: String,
    request: CreateSpeciesRequest,
) -> Result<Species, String> {
    let db = state.db();
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err("Only supervisors and admins can manage species".to_string());
    }

    let id = uuid::Uuid::new_v4().to_string();

    db.conn.execute(
        "INSERT INTO species (id, genus, species_name, common_name, species_code, default_subculture_interval_days, notes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![id, request.genus, request.species_name, request.common_name,
                request.species_code, request.default_subculture_interval_days, request.notes],
    ).map_err(|e| format!("Failed to create species: {}", e))?;

    // Classify the species under its genus taxon, creating that taxon if this is
    // the first species in the genus. Without this the species has a NULL
    // taxon_path and is invisible everywhere in the Taxonomy Navigator, which
    // keys entirely off that column — the registry fills up while the tree stays
    // empty. A failure here must not lose the species itself, so it is logged
    // into the audit trail rather than propagated; `rebuild_species_taxonomy`
    // repairs anything that slipped through.
    if let Err(e) = queries::link_species_to_genus(&db.conn, &id, &request.genus) {
        queries::log_audit(
            &db.conn, Some(&user.id), "warn", "species", Some(&id),
            None, None, Some(&format!("Genus taxon link failed: {}", e)),
        ).ok();
    }

    // EXPERIMENTAL (WP-45): Seed the species genesis entry from the genus taxon's
    // current entry_hash (if the genus has participated in the hash chain), extending
    // the provenance chain upward: Kingdom → … → Genus → Species. Falls back to
    // ZERO_HASH for genera that pre-date migration_031 or lack audit entries.
    queries::log_audit_species_genesis(
        &db.conn, Some(&user.id), "create", "species", Some(&id),
        None, Some(&request.species_code), None,
        &request.genus,
    ).ok();

    db.conn.query_row(
        "SELECT * FROM species WHERE id = ?1",
        params![id],
        |row| {
            Ok(Species {
                id: row.get("id")?,
                genus: row.get("genus")?,
                species_name: row.get("species_name")?,
                common_name: row.get("common_name")?,
                species_code: row.get("species_code")?,
                default_subculture_interval_days: row.get("default_subculture_interval_days")?,
                notes: row.get("notes")?,
                taxon_path: row.get("taxon_path")?,
                ncbi_taxon_id: row.get("ncbi_taxon_id")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        },
    ).map_err(|e| format!("Failed to fetch species: {}", e))
}

#[tauri::command]
pub fn update_species(
    state: State<AppState>,
    token: String,
    request: UpdateSpeciesRequest,
) -> Result<(), String> {
    let db = state.db();
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err("Only supervisors and admins can manage species".to_string());
    }

    let mut updates = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(ref genus) = request.genus {
        updates.push(format!("genus = ?{}", values.len() + 1));
        values.push(Box::new(genus.clone()));
    }
    if let Some(ref sn) = request.species_name {
        updates.push(format!("species_name = ?{}", values.len() + 1));
        values.push(Box::new(sn.clone()));
    }
    if let Some(ref cn) = request.common_name {
        updates.push(format!("common_name = ?{}", values.len() + 1));
        values.push(Box::new(cn.clone()));
    }
    if let Some(ref sc) = request.species_code {
        updates.push(format!("species_code = ?{}", values.len() + 1));
        values.push(Box::new(sc.clone()));
    }
    if let Some(interval) = request.default_subculture_interval_days {
        updates.push(format!("default_subculture_interval_days = ?{}", values.len() + 1));
        values.push(Box::new(interval));
    }
    if let Some(ref notes) = request.notes {
        updates.push(format!("notes = ?{}", values.len() + 1));
        values.push(Box::new(notes.clone()));
    }

    if updates.is_empty() {
        return Err("No fields to update".to_string());
    }

    updates.push("updated_at = datetime('now')".to_string());
    let sql = format!(
        "UPDATE species SET {} WHERE id = ?{}",
        updates.join(", "),
        values.len() + 1
    );
    values.push(Box::new(request.id.clone()));

    let bind_refs: Vec<&dyn rusqlite::types::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    db.conn.execute(&sql, bind_refs.as_slice())
        .map_err(|e| format!("Failed to update species: {}", e))?;

    // Renaming the genus moves the species to a different branch of the tree, so
    // re-link it. Only done when `genus` was actually part of the update — an
    // edit that touches only the common name must not disturb a species that an
    // operator has deliberately classified under a deeper hand-built backbone.
    if let Some(ref genus) = request.genus {
        if let Err(e) = queries::link_species_to_genus(&db.conn, &request.id, genus) {
            queries::log_audit(
                &db.conn, Some(&user.id), "warn", "species", Some(&request.id),
                None, None, Some(&format!("Genus taxon re-link failed: {}", e)),
            ).ok();
        }
    }

    queries::log_audit(
        &db.conn, Some(&user.id), "update", "species", Some(&request.id),
        None, None, None,
    ).ok();

    Ok(())
}

/// Repair the genus backbone for a lab whose species were created before
/// `create_species` linked genus taxa (any species added between the WP-35
/// backfill and this release). Idempotent: species that are already classified
/// are left untouched, so it is safe to run from the UI at any time.
///
/// Exposed as an explicit action rather than run silently at startup because it
/// creates `taxa` rows, and a supervisor should be the one to decide that.
#[tauri::command]
pub fn rebuild_species_taxonomy(
    state: State<AppState>,
    token: String,
) -> Result<RebuildTaxonomyResult, String> {
    let db = state.db();
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err("Only supervisors and admins can rebuild the taxonomy".to_string());
    }

    let (genera_created, species_linked) =
        queries::rebuild_species_taxonomy(&db.conn).map_err(|e| e.to_string())?;

    if species_linked > 0 {
        queries::log_audit(
            &db.conn, Some(&user.id), "update", "taxon", None,
            None, None,
            Some(&format!(
                "Rebuilt species taxonomy: {} genus taxa created, {} species linked",
                genera_created, species_linked
            )),
        ).ok();
    }

    Ok(RebuildTaxonomyResult { genera_created, species_linked })
}

#[derive(Debug, Serialize)]
pub struct RebuildTaxonomyResult {
    pub genera_created: i64,
    pub species_linked: i64,
}

#[tauri::command]
pub fn list_projects(state: State<AppState>, token: String) -> Result<Vec<Project>, String> {
    let db = state.db();
    let _user = auth_service::validate_session(&db, &token)?;

    let mut stmt = db.conn.prepare(
        "SELECT id, name, status FROM projects WHERE status != 'archived' ORDER BY name"
    ).map_err(|e| e.to_string())?;

    let projects = stmt.query_map([], |row| {
        Ok(Project {
            id: row.get("id")?,
            name: row.get("name")?,
            status: row.get("status")?,
        })
    }).map_err(|e| e.to_string())?
      .filter_map(|r| r.ok())
      .collect();

    Ok(projects)
}

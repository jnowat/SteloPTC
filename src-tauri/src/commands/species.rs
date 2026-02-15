use crate::auth as auth_service;
use crate::db::queries;
use crate::models::species::*;
use crate::AppState;
use rusqlite::params;
use tauri::State;

#[tauri::command]
pub fn list_species(state: State<AppState>, token: String) -> Result<Vec<Species>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
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
    let db = state.db.lock().map_err(|e| e.to_string())?;
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

    queries::log_audit(
        &db.conn, Some(&user.id), "create", "species", Some(&id),
        None, Some(&request.species_code), None,
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
    let db = state.db.lock().map_err(|e| e.to_string())?;
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

    queries::log_audit(
        &db.conn, Some(&user.id), "update", "species", Some(&request.id),
        None, None, None,
    ).ok();

    Ok(())
}

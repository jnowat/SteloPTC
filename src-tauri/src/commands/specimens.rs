use crate::auth as auth_service;
use crate::db::queries;
use crate::models::specimen::*;
use crate::AppState;
use rusqlite::{params, OptionalExtension};
use tauri::State;

#[tauri::command]
pub fn list_specimens(
    state: State<AppState>,
    token: String,
    page: Option<u32>,
    per_page: Option<u32>,
) -> Result<PaginatedResponse<Specimen>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;

    let pg = queries::PaginationParams {
        page: page.unwrap_or(1),
        per_page: per_page.unwrap_or(50),
    };

    let total: i64 = db.conn.query_row(
        "SELECT COUNT(*) FROM specimens WHERE is_archived = 0", [], |r| r.get(0)
    ).map_err(|e| e.to_string())?;

    let mut stmt = db.conn.prepare(
        "SELECT s.*, sp.species_code, sp.genus || ' ' || sp.species_name as species_name,
                p.name as project_name
         FROM specimens s
         LEFT JOIN species sp ON s.species_id = sp.id
         LEFT JOIN projects p ON s.project_id = p.id
         WHERE s.is_archived = 0
         ORDER BY s.created_at DESC
         LIMIT ?1 OFFSET ?2"
    ).map_err(|e| e.to_string())?;

    let specimens = stmt.query_map(params![pg.limit(), pg.offset()], |row| {
        Ok(Specimen {
            id: row.get("id")?,
            accession_number: row.get("accession_number")?,
            species_id: row.get("species_id")?,
            species_code: row.get("species_code")?,
            species_name: row.get("species_name")?,
            project_id: row.get("project_id")?,
            project_name: row.get("project_name")?,
            stage: row.get("stage")?,
            custom_stage: row.get("custom_stage")?,
            provenance: row.get("provenance")?,
            source_plant: row.get("source_plant")?,
            initiation_date: row.get("initiation_date")?,
            location: row.get("location")?,
            location_details: row.get("location_details")?,
            propagation_method: row.get("propagation_method")?,
            acclimatization_status: row.get("acclimatization_status")?,
            health_status: row.get("health_status")?,
            disease_status: row.get("disease_status")?,
            quarantine_flag: row.get::<_, i32>("quarantine_flag")? != 0,
            quarantine_release_date: row.get("quarantine_release_date")?,
            permit_number: row.get("permit_number")?,
            permit_expiry: row.get("permit_expiry")?,
            ip_flag: row.get::<_, i32>("ip_flag")? != 0,
            ip_notes: row.get("ip_notes")?,
            environmental_notes: row.get("environmental_notes")?,
            subculture_count: row.get("subculture_count")?,
            parent_specimen_id: row.get("parent_specimen_id")?,
            qr_code_data: row.get("qr_code_data")?,
            notes: row.get("notes")?,
            employee_id: row.get("employee_id")?,
            is_archived: row.get::<_, i32>("is_archived")? != 0,
            archived_at: row.get("archived_at")?,
            created_by: row.get("created_by")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }).map_err(|e| e.to_string())?
      .filter_map(|r| r.ok())
      .collect::<Vec<_>>();

    let total_pages = ((total as f64) / (pg.per_page as f64)).ceil() as u32;

    Ok(PaginatedResponse {
        items: specimens,
        total,
        page: pg.page,
        per_page: pg.per_page,
        total_pages,
    })
}

#[tauri::command]
pub fn get_specimen(state: State<AppState>, token: String, id: String) -> Result<Specimen, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;

    db.conn.query_row(
        "SELECT s.*, sp.species_code, sp.genus || ' ' || sp.species_name as species_name,
                p.name as project_name
         FROM specimens s
         LEFT JOIN species sp ON s.species_id = sp.id
         LEFT JOIN projects p ON s.project_id = p.id
         WHERE s.id = ?1",
        params![id],
        |row| {
            Ok(Specimen {
                id: row.get("id")?,
                accession_number: row.get("accession_number")?,
                species_id: row.get("species_id")?,
                species_code: row.get("species_code")?,
                species_name: row.get("species_name")?,
                project_id: row.get("project_id")?,
                project_name: row.get("project_name")?,
                stage: row.get("stage")?,
                custom_stage: row.get("custom_stage")?,
                provenance: row.get("provenance")?,
                source_plant: row.get("source_plant")?,
                initiation_date: row.get("initiation_date")?,
                location: row.get("location")?,
                location_details: row.get("location_details")?,
                propagation_method: row.get("propagation_method")?,
                acclimatization_status: row.get("acclimatization_status")?,
                health_status: row.get("health_status")?,
                disease_status: row.get("disease_status")?,
                quarantine_flag: row.get::<_, i32>("quarantine_flag")? != 0,
                quarantine_release_date: row.get("quarantine_release_date")?,
                permit_number: row.get("permit_number")?,
                permit_expiry: row.get("permit_expiry")?,
                ip_flag: row.get::<_, i32>("ip_flag")? != 0,
                ip_notes: row.get("ip_notes")?,
                environmental_notes: row.get("environmental_notes")?,
                subculture_count: row.get("subculture_count")?,
                parent_specimen_id: row.get("parent_specimen_id")?,
                qr_code_data: row.get("qr_code_data")?,
                notes: row.get("notes")?,
                employee_id: row.get("employee_id")?,
                is_archived: row.get::<_, i32>("is_archived")? != 0,
                archived_at: row.get("archived_at")?,
                created_by: row.get("created_by")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        },
    ).map_err(|e| format!("Specimen not found: {}", e))
}

#[tauri::command]
pub fn create_specimen(
    state: State<AppState>,
    token: String,
    request: CreateSpecimenRequest,
) -> Result<Specimen, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }

    let species_code: String = db.conn.query_row(
        "SELECT species_code FROM species WHERE id = ?1",
        params![request.species_id],
        |row| row.get(0),
    ).map_err(|_| "Species not found".to_string())?;

    let accession = queries::generate_accession_number(&db.conn, &species_code, &request.initiation_date)
        .map_err(|e| format!("Failed to generate accession: {}", e))?;

    let id = uuid::Uuid::new_v4().to_string();
    let qr_data = format!("STELO:{}", accession);

    db.conn.execute(
        "INSERT INTO specimens (id, accession_number, species_id, project_id, stage, custom_stage,
         provenance, source_plant, initiation_date, location, location_details,
         propagation_method, acclimatization_status, health_status, disease_status,
         quarantine_flag, permit_number, permit_expiry, ip_flag, ip_notes,
         environmental_notes, parent_specimen_id, qr_code_data, notes, employee_id, created_by)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26)",
        params![
            id, accession, request.species_id, request.project_id, request.stage, request.custom_stage,
            request.provenance, request.source_plant, request.initiation_date, request.location,
            request.location_details, request.propagation_method, request.acclimatization_status,
            request.health_status, request.disease_status, request.quarantine_flag.unwrap_or(false) as i32,
            request.permit_number, request.permit_expiry, request.ip_flag.unwrap_or(false) as i32,
            request.ip_notes, request.environmental_notes, request.parent_specimen_id, qr_data,
            request.notes, request.employee_id, user.id,
        ],
    ).map_err(|e| format!("Failed to create specimen: {}", e))?;

    queries::log_audit(
        &db.conn, Some(&user.id), "create", "specimen", Some(&id),
        None, Some(&accession), Some("Specimen created"),
    ).ok();

    drop(db);
    get_specimen(state, token, id)
}

#[tauri::command]
pub fn update_specimen(
    state: State<AppState>,
    token: String,
    request: UpdateSpecimenRequest,
) -> Result<Specimen, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }

    let mut updates = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    macro_rules! add_update {
        ($field:ident, $col:expr) => {
            if let Some(ref val) = request.$field {
                updates.push(format!("{} = ?{}", $col, values.len() + 1));
                values.push(Box::new(val.clone()));
            }
        };
    }

    add_update!(stage, "stage");
    add_update!(custom_stage, "custom_stage");
    add_update!(location, "location");
    add_update!(location_details, "location_details");
    add_update!(propagation_method, "propagation_method");
    add_update!(acclimatization_status, "acclimatization_status");
    add_update!(health_status, "health_status");
    add_update!(disease_status, "disease_status");
    add_update!(quarantine_release_date, "quarantine_release_date");
    add_update!(permit_number, "permit_number");
    add_update!(permit_expiry, "permit_expiry");
    add_update!(ip_notes, "ip_notes");
    add_update!(environmental_notes, "environmental_notes");
    add_update!(notes, "notes");

    if let Some(qf) = request.quarantine_flag {
        updates.push(format!("quarantine_flag = ?{}", values.len() + 1));
        values.push(Box::new(qf as i32));
    }
    if let Some(ipf) = request.ip_flag {
        updates.push(format!("ip_flag = ?{}", values.len() + 1));
        values.push(Box::new(ipf as i32));
    }

    if updates.is_empty() {
        return Err("No fields to update".to_string());
    }

    updates.push(format!("updated_at = datetime('now')"));
    let sql = format!(
        "UPDATE specimens SET {} WHERE id = ?{}",
        updates.join(", "),
        values.len() + 1
    );
    values.push(Box::new(request.id.clone()));

    let params: Vec<&dyn rusqlite::types::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    db.conn.execute(&sql, params.as_slice())
        .map_err(|e| format!("Failed to update specimen: {}", e))?;

    queries::log_audit(
        &db.conn, Some(&user.id), "update", "specimen", Some(&request.id),
        None, None, Some("Specimen updated"),
    ).ok();

    drop(db);
    get_specimen(state, token, request.id)
}

#[tauri::command]
pub fn delete_specimen(state: State<AppState>, token: String, id: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err("Only supervisors and admins can delete specimens".to_string());
    }

    // Archive instead of hard delete
    db.conn.execute(
        "UPDATE specimens SET is_archived = 1, archived_at = datetime('now'), updated_at = datetime('now') WHERE id = ?1",
        params![id],
    ).map_err(|e| format!("Failed to archive specimen: {}", e))?;

    queries::log_audit(
        &db.conn, Some(&user.id), "archive", "specimen", Some(&id),
        None, None, Some("Specimen archived"),
    ).ok();

    Ok(())
}

#[tauri::command]
pub fn search_specimens(
    state: State<AppState>,
    token: String,
    params_input: SpecimenSearchParams,
) -> Result<PaginatedResponse<Specimen>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;

    let pg = queries::PaginationParams {
        page: params_input.page.unwrap_or(1),
        per_page: params_input.per_page.unwrap_or(50),
    };

    let mut conditions = Vec::new();
    let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    let show_archived = params_input.archived.unwrap_or(false);
    if !show_archived {
        conditions.push("s.is_archived = 0".to_string());
    }

    if let Some(ref q) = params_input.query {
        let param_idx = bind_values.len() + 1;
        conditions.push(format!(
            "(s.accession_number LIKE ?{p} OR s.notes LIKE ?{p} OR s.location LIKE ?{p} OR s.provenance LIKE ?{p})",
            p = param_idx
        ));
        bind_values.push(Box::new(format!("%{}%", q)));
    }

    if let Some(ref sid) = params_input.species_id {
        let param_idx = bind_values.len() + 1;
        conditions.push(format!("s.species_id = ?{}", param_idx));
        bind_values.push(Box::new(sid.clone()));
    }

    if let Some(ref stage) = params_input.stage {
        let param_idx = bind_values.len() + 1;
        conditions.push(format!("s.stage = ?{}", param_idx));
        bind_values.push(Box::new(stage.clone()));
    }

    if let Some(ref pid) = params_input.project_id {
        let param_idx = bind_values.len() + 1;
        conditions.push(format!("s.project_id = ?{}", param_idx));
        bind_values.push(Box::new(pid.clone()));
    }

    if params_input.quarantine_only.unwrap_or(false) {
        conditions.push("s.quarantine_flag = 1".to_string());
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let count_sql = format!("SELECT COUNT(*) FROM specimens s {}", where_clause);
    let bind_refs: Vec<&dyn rusqlite::types::ToSql> = bind_values.iter().map(|v| v.as_ref()).collect();
    let total: i64 = db.conn.query_row(&count_sql, bind_refs.as_slice(), |r| r.get(0))
        .map_err(|e| e.to_string())?;

    let query_sql = format!(
        "SELECT s.*, sp.species_code, sp.genus || ' ' || sp.species_name as species_name,
                p.name as project_name
         FROM specimens s
         LEFT JOIN species sp ON s.species_id = sp.id
         LEFT JOIN projects p ON s.project_id = p.id
         {}
         ORDER BY s.created_at DESC
         LIMIT ?{} OFFSET ?{}",
        where_clause,
        bind_values.len() + 1,
        bind_values.len() + 2
    );

    bind_values.push(Box::new(pg.limit()));
    bind_values.push(Box::new(pg.offset()));

    let bind_refs2: Vec<&dyn rusqlite::types::ToSql> = bind_values.iter().map(|v| v.as_ref()).collect();
    let mut stmt = db.conn.prepare(&query_sql).map_err(|e| e.to_string())?;

    let specimens = stmt.query_map(bind_refs2.as_slice(), |row| {
        Ok(Specimen {
            id: row.get("id")?,
            accession_number: row.get("accession_number")?,
            species_id: row.get("species_id")?,
            species_code: row.get("species_code")?,
            species_name: row.get("species_name")?,
            project_id: row.get("project_id")?,
            project_name: row.get("project_name")?,
            stage: row.get("stage")?,
            custom_stage: row.get("custom_stage")?,
            provenance: row.get("provenance")?,
            source_plant: row.get("source_plant")?,
            initiation_date: row.get("initiation_date")?,
            location: row.get("location")?,
            location_details: row.get("location_details")?,
            propagation_method: row.get("propagation_method")?,
            acclimatization_status: row.get("acclimatization_status")?,
            health_status: row.get("health_status")?,
            disease_status: row.get("disease_status")?,
            quarantine_flag: row.get::<_, i32>("quarantine_flag")? != 0,
            quarantine_release_date: row.get("quarantine_release_date")?,
            permit_number: row.get("permit_number")?,
            permit_expiry: row.get("permit_expiry")?,
            ip_flag: row.get::<_, i32>("ip_flag")? != 0,
            ip_notes: row.get("ip_notes")?,
            environmental_notes: row.get("environmental_notes")?,
            subculture_count: row.get("subculture_count")?,
            parent_specimen_id: row.get("parent_specimen_id")?,
            qr_code_data: row.get("qr_code_data")?,
            notes: row.get("notes")?,
            employee_id: row.get("employee_id")?,
            is_archived: row.get::<_, i32>("is_archived")? != 0,
            archived_at: row.get("archived_at")?,
            created_by: row.get("created_by")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }).map_err(|e| e.to_string())?
      .filter_map(|r| r.ok())
      .collect::<Vec<_>>();

    let total_pages = ((total as f64) / (pg.per_page as f64)).ceil() as u32;

    Ok(PaginatedResponse {
        items: specimens,
        total,
        page: pg.page,
        per_page: pg.per_page,
        total_pages,
    })
}

#[tauri::command]
pub fn get_specimen_by_accession(
    state: State<AppState>,
    token: String,
    accession: String,
) -> Result<Option<Specimen>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;

    db.conn.query_row(
        "SELECT s.*, sp.species_code, sp.genus || ' ' || sp.species_name as species_name,
                p.name as project_name
         FROM specimens s
         LEFT JOIN species sp ON s.species_id = sp.id
         LEFT JOIN projects p ON s.project_id = p.id
         WHERE s.accession_number = ?1 AND s.is_archived = 0",
        params![accession],
        |row| {
            Ok(Specimen {
                id: row.get("id")?,
                accession_number: row.get("accession_number")?,
                species_id: row.get("species_id")?,
                species_code: row.get("species_code")?,
                species_name: row.get("species_name")?,
                project_id: row.get("project_id")?,
                project_name: row.get("project_name")?,
                stage: row.get("stage")?,
                custom_stage: row.get("custom_stage")?,
                provenance: row.get("provenance")?,
                source_plant: row.get("source_plant")?,
                initiation_date: row.get("initiation_date")?,
                location: row.get("location")?,
                location_details: row.get("location_details")?,
                propagation_method: row.get("propagation_method")?,
                acclimatization_status: row.get("acclimatization_status")?,
                health_status: row.get("health_status")?,
                disease_status: row.get("disease_status")?,
                quarantine_flag: row.get::<_, i32>("quarantine_flag")? != 0,
                quarantine_release_date: row.get("quarantine_release_date")?,
                permit_number: row.get("permit_number")?,
                permit_expiry: row.get("permit_expiry")?,
                ip_flag: row.get::<_, i32>("ip_flag")? != 0,
                ip_notes: row.get("ip_notes")?,
                environmental_notes: row.get("environmental_notes")?,
                subculture_count: row.get("subculture_count")?,
                parent_specimen_id: row.get("parent_specimen_id")?,
                qr_code_data: row.get("qr_code_data")?,
                notes: row.get("notes")?,
                employee_id: row.get("employee_id")?,
                is_archived: row.get::<_, i32>("is_archived")? != 0,
                archived_at: row.get("archived_at")?,
                created_by: row.get("created_by")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        },
    ).optional().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_specimen_stats(state: State<AppState>, token: String) -> Result<SpecimenStats, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;

    let total: i64 = db.conn.query_row("SELECT COUNT(*) FROM specimens", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let active: i64 = db.conn.query_row("SELECT COUNT(*) FROM specimens WHERE is_archived = 0", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let quarantined: i64 = db.conn.query_row("SELECT COUNT(*) FROM specimens WHERE quarantine_flag = 1 AND is_archived = 0", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let archived: i64 = db.conn.query_row("SELECT COUNT(*) FROM specimens WHERE is_archived = 1", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;

    let mut stage_stmt = db.conn.prepare(
        "SELECT stage, COUNT(*) FROM specimens WHERE is_archived = 0 GROUP BY stage ORDER BY COUNT(*) DESC"
    ).map_err(|e| e.to_string())?;
    let by_stage: Vec<StageCount> = stage_stmt.query_map([], |row| {
        Ok(StageCount { stage: row.get(0)?, count: row.get(1)? })
    }).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();

    let mut species_stmt = db.conn.prepare(
        "SELECT sp.species_code, COUNT(*) FROM specimens s
         JOIN species sp ON s.species_id = sp.id WHERE s.is_archived = 0
         GROUP BY sp.species_code ORDER BY COUNT(*) DESC"
    ).map_err(|e| e.to_string())?;
    let by_species: Vec<SpeciesCount> = species_stmt.query_map([], |row| {
        Ok(SpeciesCount { species_code: row.get(0)?, count: row.get(1)? })
    }).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();

    let recent: i64 = db.conn.query_row(
        "SELECT COUNT(*) FROM subcultures WHERE date >= date('now', '-7 days')", [], |r| r.get(0)
    ).map_err(|e| e.to_string())?;

    Ok(SpecimenStats {
        total_specimens: total,
        active_specimens: active,
        quarantined,
        archived,
        by_stage,
        by_species,
        recent_subcultures: recent,
    })
}

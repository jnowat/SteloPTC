use crate::auth as auth_service;
use crate::db::queries;
use crate::models::specimen::{
    CreateSpecimenRequest, FamilyMember, PaginatedResponse, Specimen, SpecimenSearchParams,
    SpecimenStats, SplitChildResult, SplitResult, SplitSpecimenRequest,
    StageCount, SpeciesCount, UpdateSpecimenRequest,
};
use crate::AppState;
use rusqlite::params;
use tauri::State;

// Tuple returned by the parent-specimen query in split_specimen.
// Fields: (species_id, species_code, stage, provenance, source_plant, location,
//          generation, lineage_passage_offset, subculture_count, root_specimen_id, accession_number)
type ParentInfo = (String, String, String, Option<String>, Option<String>, Option<String>, i32, i32, i32, Option<String>, String);

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
                p.name as project_name,
                COALESCE(cf.has_contamination, 0) AS has_contamination
         FROM specimens s
         LEFT JOIN species sp ON s.species_id = sp.id
         LEFT JOIN projects p ON s.project_id = p.id
         LEFT JOIN (SELECT specimen_id, MAX(contamination_flag) AS has_contamination
                    FROM subcultures GROUP BY specimen_id) cf ON cf.specimen_id = s.id
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
            generation: row.get("generation")?,
            lineage_passage_offset: row.get("lineage_passage_offset")?,
            root_specimen_id: row.get("root_specimen_id")?,
            parent_specimen_id: row.get("parent_specimen_id")?,
            qr_code_data: row.get("qr_code_data")?,
            notes: row.get("notes")?,
            employee_id: row.get("employee_id")?,
            is_archived: row.get::<_, i32>("is_archived")? != 0,
            archived_at: row.get("archived_at")?,
            created_by: row.get("created_by")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
            has_contamination: row.get::<_, i32>("has_contamination")? != 0,
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
                p.name as project_name,
                COALESCE(cf.has_contamination, 0) AS has_contamination
         FROM specimens s
         LEFT JOIN species sp ON s.species_id = sp.id
         LEFT JOIN projects p ON s.project_id = p.id
         LEFT JOIN (SELECT specimen_id, MAX(contamination_flag) AS has_contamination
                    FROM subcultures WHERE specimen_id = ?1 GROUP BY specimen_id) cf ON cf.specimen_id = s.id
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
                generation: row.get("generation")?,
                lineage_passage_offset: row.get("lineage_passage_offset")?,
                root_specimen_id: row.get("root_specimen_id")?,
                parent_specimen_id: row.get("parent_specimen_id")?,
                qr_code_data: row.get("qr_code_data")?,
                notes: row.get("notes")?,
                employee_id: row.get("employee_id")?,
                is_archived: row.get::<_, i32>("is_archived")? != 0,
                archived_at: row.get("archived_at")?,
                created_by: row.get("created_by")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
                has_contamination: row.get::<_, i32>("has_contamination")? != 0,
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

    // Wrap the specimen INSERT and the audit entry in a single transaction.
    // This guarantees two things:
    //   1. A specimen without an audit entry can never be committed to the DB.
    //   2. When a child is split from a parent, the parent's audit entry_hash
    //      is visible to the child's log_audit_for_child query (same txn is
    //      not needed here since the parent was committed in a prior request,
    //      but the transaction still ensures our own write is atomic).
    let tx = db.conn
        .unchecked_transaction()
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    tx.execute(
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

    // Link the audit chain.
    // - Split/derived: fork from parent's last entry_hash (cryptographically visible fork).
    // - Root specimen: seed from species' last entry_hash (binds specimen to its species
    //   definition; falls back to ZERO_HASH for pre-hash-chain species).
    if let Some(ref parent_id) = request.parent_specimen_id {
        queries::log_audit_for_child(
            &tx, Some(&user.id), "create", "specimen", Some(&id),
            None, Some(&accession), Some("Specimen created (split/derived)"),
            parent_id,
        ).map_err(|e| format!("Failed to write split audit entry: {}", e))?;
    } else {
        queries::log_audit_seeded_by_species(
            &tx, Some(&user.id), "create", "specimen", Some(&id),
            None, Some(&accession), Some("Specimen created"),
            &request.species_id,
        ).map_err(|e| format!("Failed to write audit entry: {}", e))?;
    }

    tx.commit().map_err(|e| format!("Failed to commit specimen: {}", e))?;

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

    updates.push("updated_at = datetime('now')".to_string());
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
            "(s.accession_number LIKE ?{p} OR s.notes LIKE ?{p} OR s.location LIKE ?{p} \
             OR s.provenance LIKE ?{p} OR s.source_plant LIKE ?{p} \
             OR sp.genus LIKE ?{p} OR sp.species_name LIKE ?{p})",
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

    let count_sql = format!(
        "SELECT COUNT(*) FROM specimens s LEFT JOIN species sp ON s.species_id = sp.id {}",
        where_clause
    );
    let bind_refs: Vec<&dyn rusqlite::types::ToSql> = bind_values.iter().map(|v| v.as_ref()).collect();
    let total: i64 = db.conn.query_row(&count_sql, bind_refs.as_slice(), |r| r.get(0))
        .map_err(|e| e.to_string())?;

    let query_sql = format!(
        "SELECT s.*, sp.species_code, sp.genus || ' ' || sp.species_name as species_name,
                p.name as project_name,
                COALESCE(cf.has_contamination, 0) AS has_contamination
         FROM specimens s
         LEFT JOIN species sp ON s.species_id = sp.id
         LEFT JOIN projects p ON s.project_id = p.id
         LEFT JOIN (SELECT specimen_id, MAX(contamination_flag) AS has_contamination
                    FROM subcultures GROUP BY specimen_id) cf ON cf.specimen_id = s.id
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
            generation: row.get("generation")?,
            lineage_passage_offset: row.get("lineage_passage_offset")?,
            root_specimen_id: row.get("root_specimen_id")?,
            parent_specimen_id: row.get("parent_specimen_id")?,
            qr_code_data: row.get("qr_code_data")?,
            notes: row.get("notes")?,
            employee_id: row.get("employee_id")?,
            is_archived: row.get::<_, i32>("is_archived")? != 0,
            archived_at: row.get("archived_at")?,
            created_by: row.get("created_by")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
            has_contamination: row.get::<_, i32>("has_contamination")? != 0,
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

#[tauri::command]
pub fn bulk_archive_specimens(
    state: State<AppState>,
    token: String,
    ids: Vec<String>,
) -> Result<usize, String> {
    if ids.is_empty() {
        return Ok(0);
    }
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err("Only supervisors and admins can archive specimens".to_string());
    }
    let mut count = 0usize;
    for id in &ids {
        let n = db.conn.execute(
            "UPDATE specimens SET is_archived = 1, archived_at = datetime('now'),
             updated_at = datetime('now') WHERE id = ?1 AND is_archived = 0",
            params![id],
        ).map_err(|e| e.to_string())?;
        count += n;
        if n > 0 {
            queries::log_audit(
                &db.conn, Some(&user.id), "archive", "specimen", Some(id),
                None, None, Some("Bulk archived"),
            ).ok();
        }
    }
    Ok(count)
}

#[tauri::command]
pub fn bulk_update_location(
    state: State<AppState>,
    token: String,
    ids: Vec<String>,
    location: String,
) -> Result<usize, String> {
    if ids.is_empty() {
        return Ok(0);
    }
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }
    let mut count = 0usize;
    for id in &ids {
        let n = db.conn.execute(
            "UPDATE specimens SET location = ?1, updated_at = datetime('now')
             WHERE id = ?2 AND is_archived = 0",
            params![location, id],
        ).map_err(|e| e.to_string())?;
        count += n;
        if n > 0 {
            queries::log_audit(
                &db.conn, Some(&user.id), "update", "specimen", Some(id),
                None, None, Some(&format!("Bulk location transfer: {}", location)),
            ).ok();
        }
    }
    Ok(count)
}

/// Atomically split a specimen into N child specimens.
///
/// - Archives the parent (soft-delete) and appends a "split" event to its chain.
/// - Each child inherits the parent's last entry_hash as its prev_hash, making
///   the fork cryptographically visible (both siblings share the same prev_hash).
/// - A first-passage subculture record is created for each child immediately.
/// - Accession numbers use letter suffixes (e.g. 001 → 001A, 001B).
///   User-provided accession numbers override auto-generation; uniqueness is enforced.
/// - Per-child: accession number, stage, health, media batch, vessel, location, notes.
/// - Optional per-child reminder (created atomically within the same transaction).
#[tauri::command]
pub fn split_specimen(
    state: State<AppState>,
    token: String,
    request: SplitSpecimenRequest,
) -> Result<SplitResult, String> {
    if request.children.len() < 2 {
        return Err("Split requires at least 2 children".to_string());
    }

    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }

    // Fetch parent info — fail if archived
    let (parent_species_id, _parent_species_code, parent_stage,
         parent_provenance, parent_source_plant, parent_location,
         parent_generation, parent_passage_offset, parent_subculture_count,
         parent_root_id, parent_accession): ParentInfo = db.conn.query_row(
        "SELECT s.species_id, sp.species_code, s.stage,
                s.provenance, s.source_plant, s.location,
                s.generation, s.lineage_passage_offset, s.subculture_count,
                s.root_specimen_id, s.accession_number
         FROM specimens s
         JOIN species sp ON s.species_id = sp.id
         WHERE s.id = ?1 AND s.is_archived = 0",
        params![request.parent_specimen_id],
        |row| Ok((
            row.get(0)?, row.get(1)?, row.get(2)?,
            row.get(3)?, row.get(4)?, row.get(5)?,
            row.get(6)?, row.get(7)?, row.get(8)?, row.get(9)?, row.get(10)?,
        )),
    ).map_err(|_| "Parent specimen not found or already archived".to_string())?;

    // Pre-generate accession numbers for children that did not specify one.
    // These use the parent's full accession string with a letter suffix (A, B, C…),
    // skipping any letters already taken in the database.
    let auto_count: usize = request.children.iter()
        .filter(|c| c.accession_number.as_deref().map(str::is_empty).unwrap_or(true))
        .count();
    let auto_generated: Vec<String> = if auto_count > 0 {
        queries::generate_split_accession_numbers(&db.conn, &parent_accession, auto_count)
            .map_err(|e| e.to_string())?
    } else {
        Vec::new()
    };

    // Assign and validate all accession numbers (pre-transaction).
    let mut child_accessions: Vec<String> = Vec::with_capacity(request.children.len());
    {
        let mut auto_idx: usize = 0;
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

        for (i, child) in request.children.iter().enumerate() {
            let acc = match child.accession_number.as_deref().filter(|s| !s.is_empty()) {
                Some(provided) => {
                    let exists: bool = db.conn.query_row(
                        "SELECT COUNT(*) FROM specimens WHERE accession_number = ?1",
                        params![provided],
                        |r| r.get::<_, i64>(0),
                    ).map(|c| c > 0).map_err(|e| e.to_string())?;
                    if exists {
                        return Err(format!(
                            "Accession number '{}' for child {} is already in use",
                            provided, i + 1
                        ));
                    }
                    provided.to_string()
                }
                None => {
                    let a = auto_generated.get(auto_idx).ok_or_else(|| {
                        format!("Internal error: ran out of auto-generated accessions at child {}", i + 1)
                    })?.clone();
                    auto_idx += 1;
                    a
                }
            };

            if seen.contains(&acc) {
                return Err(format!(
                    "Duplicate accession number '{}' at child {} — each child must have a unique accession",
                    acc, i + 1
                ));
            }
            seen.insert(acc.clone());
            child_accessions.push(acc);
        }
    }

    // Compute genealogy values for children:
    //   generation             = parent + 1
    //   lineage_passage_offset = parent's total passage count + 1 (split itself is the next passage)
    //   root_specimen_id       = parent's root (if set) else the parent itself
    let child_generation = parent_generation + 1;
    let child_passage_offset = parent_passage_offset + parent_subculture_count + 1;
    let child_root_id: &str = parent_root_id
        .as_deref()
        .unwrap_or(&request.parent_specimen_id);

    let tx = db.conn
        .unchecked_transaction()
        .map_err(|e| format!("Failed to begin transaction: {}", e))?;

    // 1. Archive the parent
    tx.execute(
        "UPDATE specimens SET is_archived = 1, archived_at = datetime('now'), \
         updated_at = datetime('now') WHERE id = ?1",
        params![request.parent_specimen_id],
    ).map_err(|e| format!("Failed to archive parent: {}", e))?;

    // 2. Log the split event on the parent's chain.
    //    This becomes the parent's last entry_hash, which ALL children
    //    will inherit as their shared prev_hash (the cryptographic fork point).
    queries::log_audit(
        &tx, Some(&user.id), "split", "specimen", Some(&request.parent_specimen_id),
        None, None,
        Some(&format!(
            "Specimen split into {} children on {}",
            request.children.len(), request.date
        )),
    ).map_err(|e| format!("Failed to log split event on parent: {}", e))?;

    let mut child_results: Vec<SplitChildResult> = Vec::new();

    // 3. Create each child
    for (i, child) in request.children.iter().enumerate() {
        let child_id = uuid::Uuid::new_v4().to_string();
        let accession = &child_accessions[i];
        let qr_data = format!("STELO:{}", accession);

        let child_location: Option<&str> = child.location.as_deref()
            .or(parent_location.as_deref());
        let child_health: Option<&str> = child.health_status.as_deref()
            .or(request.health_status.as_deref());
        // Per-child stage override; falls back to parent stage.
        let child_stage: &str = child.stage.as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or(parent_stage.as_str());

        let default_note = format!(
            "Split from {} on {}. Container {} of {}.",
            request.parent_specimen_id, request.date, i + 1, request.children.len()
        );
        let child_notes: &str = child.notes.as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or(default_note.as_str());

        // Insert child specimen with genealogy fields
        tx.execute(
            "INSERT INTO specimens \
             (id, accession_number, species_id, stage, initiation_date, \
              location, health_status, qr_code_data, parent_specimen_id, \
              provenance, source_plant, notes, created_by, \
              generation, lineage_passage_offset, root_specimen_id) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                child_id, accession, parent_species_id, child_stage, request.date,
                child_location, child_health, qr_data, request.parent_specimen_id,
                parent_provenance, parent_source_plant, child_notes, user.id,
                child_generation, child_passage_offset, child_root_id,
            ],
        ).map_err(|e| format!("Failed to create child specimen {}: {}", i + 1, e))?;

        // Fork the audit chain from the parent (all children inherit the same
        // parent prev_hash — the split event logged above — making the fork visible)
        queries::log_audit_for_child(
            &tx, Some(&user.id), "create", "specimen", Some(&child_id),
            None, Some(accession.as_str()),
            Some(&format!(
                "Split from {} — container {} of {}",
                request.parent_specimen_id, i + 1, request.children.len()
            )),
            &request.parent_specimen_id,
        ).map_err(|e| format!("Failed to audit child specimen {}: {}", i + 1, e))?;

        // No auto-created Passage 1: the split event itself counts as the next passage.
        // The child's lineage_passage_offset already reflects parent_total + 1, so the
        // first real subculture recorded on the child will be correctly numbered P(offset+1).

        // Create a check-in reminder if requested for this child
        if let Some(days) = child.reminder_days.filter(|&d| d > 0) {
            let reminder_id = uuid::Uuid::new_v4().to_string();
            // Compute due_date using SQLite date arithmetic
            let due_date: String = tx.query_row(
                &format!("SELECT date(?1, '+{} days')", days),
                params![request.date],
                |r| r.get(0),
            ).map_err(|e| format!("Failed to compute reminder due date for child {}: {}", i + 1, e))?;

            tx.execute(
                "INSERT INTO reminders \
                 (id, specimen_id, title, description, reminder_type, due_date, \
                  is_recurring, status, snooze_count, urgency, created_by) \
                 VALUES (?1, ?2, ?3, ?4, 'custom', ?5, 0, 'active', 0, 'normal', ?6)",
                params![
                    reminder_id, child_id,
                    format!("Check-in: {} ({} days post-split)", accession, days),
                    format!("Post-split check-in reminder for {}", accession),
                    due_date, user.id,
                ],
            ).map_err(|e| format!("Failed to create reminder for child {}: {}", i + 1, e))?;
        }

        child_results.push(SplitChildResult { id: child_id, accession_number: accession.clone() });
    }

    tx.commit().map_err(|e| format!("Failed to commit split: {}", e))?;

    Ok(SplitResult {
        archived_parent_id: request.parent_specimen_id,
        children: child_results,
    })
}

/// Return the accession numbers that would be auto-generated for a split of the given specimen.
/// Does not modify the database — safe to call for previewing the split.
#[tauri::command]
pub fn preview_split_accessions(
    state: State<AppState>,
    token: String,
    parent_id: String,
    count: u32,
) -> Result<Vec<String>, String> {
    if count == 0 || count > 26 {
        return Err("Count must be between 1 and 26".to_string());
    }
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;

    let parent_accession: String = db.conn.query_row(
        "SELECT accession_number FROM specimens WHERE id = ?1 AND is_archived = 0",
        params![parent_id],
        |r| r.get(0),
    ).map_err(|_| "Parent specimen not found or already archived".to_string())?;

    queries::generate_split_accession_numbers(&db.conn, &parent_accession, count as usize)
        .map_err(|e| e.to_string())
}

/// Return all specimens that share the same root as the given specimen.
///
/// Includes the root itself plus every descendant (at any depth), both active
/// and archived. The caller can use this to render a full family tree.
#[tauri::command]
pub fn get_specimen_family(
    state: State<AppState>,
    token: String,
    id: String,
) -> Result<Vec<FamilyMember>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;

    // Determine the root: if this specimen has a root_specimen_id it IS the root,
    // otherwise the specimen itself is the root.
    let root_id: String = db.conn
        .query_row(
            "SELECT COALESCE(root_specimen_id, id) FROM specimens WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )
        .map_err(|_| "Specimen not found".to_string())?;

    // Fetch all members: the root itself + every specimen whose root_specimen_id = root.
    let mut stmt = db.conn
        .prepare(
            "SELECT s.id, s.accession_number, s.generation, s.lineage_passage_offset,
                    s.subculture_count, s.is_archived, s.parent_specimen_id,
                    s.root_specimen_id, s.health_status, s.location, sp.species_code
             FROM specimens s
             LEFT JOIN species sp ON s.species_id = sp.id
             WHERE s.id = ?1 OR s.root_specimen_id = ?1
             ORDER BY s.generation ASC, s.created_at ASC",
        )
        .map_err(|e| e.to_string())?;

    let members = stmt
        .query_map(params![root_id], |row| {
            Ok(FamilyMember {
                id:                    row.get(0)?,
                accession_number:      row.get(1)?,
                generation:            row.get(2)?,
                lineage_passage_offset: row.get(3)?,
                subculture_count:      row.get(4)?,
                is_archived:           row.get::<_, i32>(5)? != 0,
                parent_specimen_id:    row.get(6)?,
                root_specimen_id:      row.get(7)?,
                health_status:         row.get(8)?,
                location:              row.get(9)?,
                species_code:          row.get(10)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(members)
}

#[tauri::command]
pub fn bulk_update_stage(
    state: State<AppState>,
    token: String,
    ids: Vec<String>,
    stage: String,
) -> Result<usize, String> {
    if ids.is_empty() {
        return Ok(0);
    }
    const VALID_STAGES: &[&str] = &[
        "explant", "callus", "suspension", "protoplast",
        "shoot", "shoot_meristem", "apical_meristem",
        "root", "root_meristem",
        "embryogenic", "plantlet", "acclimatized", "stock", "custom",
    ];
    if !VALID_STAGES.contains(&stage.as_str()) {
        return Err(format!("Invalid stage: {}", stage));
    }
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }
    let mut count = 0usize;
    for id in &ids {
        let n = db.conn.execute(
            "UPDATE specimens SET stage = ?1, updated_at = datetime('now')
             WHERE id = ?2 AND is_archived = 0",
            params![stage, id],
        ).map_err(|e| e.to_string())?;
        count += n;
        if n > 0 {
            queries::log_audit(
                &db.conn, Some(&user.id), "update", "specimen", Some(id),
                None, None, Some(&format!("Bulk stage update: {}", stage)),
            ).ok();
        }
    }
    Ok(count)
}

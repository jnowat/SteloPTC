use crate::auth as auth_service;
use crate::db::queries;
use crate::models::specimen::{
    CreateSpecimenRequest, FamilyMember, PaginatedResponse, Specimen, SpecimenSearchParams,
    SpecimenStats, SplitChildResult, SplitResult, SplitSpecimenRequest,
    UpdateSpecimenRequest,
};
use crate::AppState;
use rusqlite::params;
use tauri::State;

// Tuple returned by the parent-specimen query in split_specimen.
// Fields: (species_id, species_code, stage, provenance, source_plant, location,
//          generation, lineage_passage_offset, subculture_count, root_specimen_id, accession_number,
//          contamination_flag, contamination_notes, origin_type)
type ParentInfo = (String, String, String, Option<String>, Option<String>, Option<String>, i32, i32, i32, Option<String>, String, i32, Option<String>, Option<String>);

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
            contamination_flag: row.get::<_, i32>("contamination_flag")? != 0,
            contamination_notes: row.get("contamination_notes")?,
            created_by: row.get("created_by")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
            has_contamination: row.get::<_, i32>("has_contamination")? != 0,
            strain_id: row.get("strain_id")?,
            strain_chain_seq: row.get("strain_chain_seq")?,
            cumulative_pdl: row.get("cumulative_pdl").unwrap_or(None),
            biosafety_level: row.get("biosafety_level").unwrap_or(None),
            origin_type: row.get("origin_type").unwrap_or(None),
            is_best_performer: row.get::<_, i32>("is_best_performer").unwrap_or(0) != 0,
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
                contamination_flag: row.get::<_, i32>("contamination_flag")? != 0,
                contamination_notes: row.get("contamination_notes")?,
                created_by: row.get("created_by")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
                has_contamination: row.get::<_, i32>("has_contamination")? != 0,
                strain_id: row.get("strain_id")?,
                strain_chain_seq: row.get("strain_chain_seq")?,
                cumulative_pdl: row.get("cumulative_pdl").unwrap_or(None),
                biosafety_level: row.get("biosafety_level").unwrap_or(None),
                origin_type: row.get("origin_type").unwrap_or(None),
                is_best_performer: row.get::<_, i32>("is_best_performer").unwrap_or(0) != 0,
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

    // Validate the requested stage against the active profile's vocabulary, mirroring
    // bulk_update_stage. Without this, a stale cross-profile stage left in the New
    // Specimen form (e.g. an 'explant' default after switching to the mycology profile)
    // would be written straight to the DB.
    let profile = crate::db::vocabulary::active_profile(&db.conn);
    crate::db::vocabulary::require_selectable_stage(&db.conn, &profile, &request.stage)?;

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
    // Snapshot the strain's current chain_seq before opening the transaction
    // so that strain_chain_seq records the strain state at the moment the
    // specimen was created (not after any intra-transaction writes).
    let strain_chain_seq: Option<i64> = if let Some(ref sid) = request.strain_id {
        db.conn.query_row(
            "SELECT COALESCE(MAX(chain_seq), 0) FROM audit_log \
             WHERE lineage_id = ?1 AND entry_hash IS NOT NULL",
            params![sid],
            |r| r.get(0),
        ).ok()
    } else {
        None
    };

    let tx = db.conn
        .unchecked_transaction()
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    tx.execute(
        "INSERT INTO specimens (id, accession_number, species_id, project_id, stage, custom_stage,
         provenance, source_plant, initiation_date, location, location_details,
         propagation_method, acclimatization_status, health_status, disease_status,
         quarantine_flag, permit_number, permit_expiry, ip_flag, ip_notes,
         environmental_notes, parent_specimen_id, qr_code_data, notes, employee_id, created_by,
         strain_id, strain_chain_seq, origin_type)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17,
                 ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29)",
        params![
            id, accession, request.species_id, request.project_id, request.stage, request.custom_stage,
            request.provenance, request.source_plant, request.initiation_date, request.location,
            request.location_details, request.propagation_method, request.acclimatization_status,
            request.health_status, request.disease_status, request.quarantine_flag.unwrap_or(false) as i32,
            request.permit_number, request.permit_expiry, request.ip_flag.unwrap_or(false) as i32,
            request.ip_notes, request.environmental_notes, request.parent_specimen_id, qr_data,
            request.notes, request.employee_id, user.id,
            request.strain_id, strain_chain_seq, request.origin_type,
        ],
    ).map_err(|e| format!("Failed to create specimen: {}", e))?;

    // Link the audit chain.
    // - Split/derived: fork from parent's last entry_hash (cryptographically visible fork).
    // - Strain-seeded root: seed from strain's last entry_hash.
    // - Plain root: seed from species' last entry_hash.
    if let Some(ref parent_id) = request.parent_specimen_id {
        queries::log_audit_for_child(
            &tx, Some(&user.id), "create", "specimen", Some(&id),
            None, Some(&accession), Some("Specimen created (split/derived)"),
            parent_id,
        ).map_err(|e| format!("Failed to write split audit entry: {}", e))?;
    } else if let Some(ref strain_id) = request.strain_id {
        queries::log_audit_seeded_by_strain(
            &tx, Some(&user.id), "create", "specimen", Some(&id),
            None, Some(&accession), Some("Specimen created (strain-seeded)"),
            strain_id,
        ).map_err(|e| format!("Failed to write strain audit entry: {}", e))?;
    } else {
        queries::log_audit_seeded_by_species(
            &tx, Some(&user.id), "create", "specimen", Some(&id),
            None, Some(&accession), Some("Specimen created"),
            &request.species_id,
        ).map_err(|e| format!("Failed to write audit entry: {}", e))?;
    }

    tx.commit().map_err(|e| format!("Failed to commit specimen: {}", e))?;
    crate::db::dashboard::invalidate_dashboard_cache(&state.dashboard_cache);

    // WP-67: record a signed genesis transaction for the new specimen, attributed
    // to the creating user's Ed25519 key. Best-effort — a ledger hiccup must never
    // fail specimen creation (mirrors the `log_audit(...).ok()` convention).
    let signed_payload = serde_json::json!({
        "event": "specimen_created",
        "specimen_id": id,
        "accession_number": accession,
    })
    .to_string();
    crate::signed_ledger::try_append_signed_event(
        &db.conn, &user.id, "specimen_created", "specimen", Some(&id), &signed_payload,
    );

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
    if let Some(ref bsl) = request.biosafety_level {
        updates.push(format!("biosafety_level = ?{}", values.len() + 1));
        values.push(Box::new(bsl.clone()));
    }
    add_update!(origin_type, "origin_type");
    if let Some(ibp) = request.is_best_performer {
        updates.push(format!("is_best_performer = ?{}", values.len() + 1));
        values.push(Box::new(ibp as i32));
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
    crate::db::dashboard::invalidate_dashboard_cache(&state.dashboard_cache);

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
    crate::db::dashboard::invalidate_dashboard_cache(&state.dashboard_cache);

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

    if params_input.best_performer_only.unwrap_or(false) {
        conditions.push("s.is_best_performer = 1".to_string());
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
            contamination_flag: row.get::<_, i32>("contamination_flag")? != 0,
            contamination_notes: row.get("contamination_notes")?,
            created_by: row.get("created_by")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
            has_contamination: row.get::<_, i32>("has_contamination")? != 0,
            strain_id: row.get("strain_id")?,
            strain_chain_seq: row.get("strain_chain_seq")?,
            cumulative_pdl: row.get("cumulative_pdl").unwrap_or(None),
            biosafety_level: row.get("biosafety_level").unwrap_or(None),
            origin_type: row.get("origin_type").unwrap_or(None),
            is_best_performer: row.get::<_, i32>("is_best_performer").unwrap_or(0) != 0,
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
    let profile = crate::db::vocabulary::active_profile(&db.conn);
    // WP-63: served from the materialized dashboard cache (60s TTL, invalidated
    // immediately on any write that changes specimen/subculture counts) rather
    // than recomputing the multi-join aggregate on every dashboard load.
    let (stats, _contamination) = crate::db::dashboard::get_or_refresh_dashboard_cache(
        &db.conn,
        &profile,
        &state.dashboard_cache,
        crate::db::dashboard::DASHBOARD_CACHE_TTL,
    )?;
    Ok(stats)
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
    if count > 0 {
        crate::db::dashboard::invalidate_dashboard_cache(&state.dashboard_cache);
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
         parent_root_id, parent_accession,
         parent_contamination_flag, parent_contamination_notes,
         parent_origin_type): ParentInfo = db.conn.query_row(
        "SELECT s.species_id, sp.species_code, s.stage,
                s.provenance, s.source_plant, s.location,
                s.generation, s.lineage_passage_offset, s.subculture_count,
                s.root_specimen_id, s.accession_number,
                s.contamination_flag, s.contamination_notes, s.origin_type
         FROM specimens s
         JOIN species sp ON s.species_id = sp.id
         WHERE s.id = ?1 AND s.is_archived = 0",
        params![request.parent_specimen_id],
        |row| Ok((
            row.get(0)?, row.get(1)?, row.get(2)?,
            row.get(3)?, row.get(4)?, row.get(5)?,
            row.get(6)?, row.get(7)?, row.get(8)?, row.get(9)?, row.get(10)?,
            row.get(11)?, row.get(12)?, row.get(13)?,
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

    // Fetch the parent's cumulative PDL so children can inherit it.
    let parent_cumulative_pdl: Option<f64> = db.conn.query_row(
        "SELECT cumulative_pdl FROM specimens WHERE id = ?1",
        params![request.parent_specimen_id],
        |r| r.get(0),
    ).ok().flatten();

    // Compute genealogy values for children:
    //   generation             = parent + 1
    //   lineage_passage_offset = parent's total passage count + 1 (split itself is the next passage)
    //   root_specimen_id       = parent's root (if set) else the parent itself
    let child_generation = parent_generation + 1;
    let child_passage_offset = parent_passage_offset + parent_subculture_count + 1;
    let child_root_id: &str = parent_root_id
        .as_deref()
        .unwrap_or(&request.parent_specimen_id);

    // Contamination inheritance: children of a contaminated parent are themselves
    // flagged, because we cannot determine which child is clean without testing.
    // The split request can also introduce contamination observed during the split
    // that wasn't previously recorded on the parent record.
    let effective_contaminated = parent_contamination_flag != 0
        || request.contamination_flag.unwrap_or(false);
    let child_contamination_flag_i32 = effective_contaminated as i32;
    // Use the request's notes if provided (freshest observation); fall back to
    // the parent's existing notes so the reason is never silently dropped.
    let child_contamination_notes: Option<String> = if effective_contaminated {
        request.contamination_notes.as_deref()
            .or(parent_contamination_notes.as_deref())
            .map(str::to_string)
    } else {
        None
    };

    let tx = db.conn
        .unchecked_transaction()
        .map_err(|e| format!("Failed to begin transaction: {}", e))?;

    let contam_flag_i32 = request.contamination_flag.unwrap_or(false) as i32;

    // 1. Archive the parent, recording its health/notes/contamination as of the split.
    //    Contamination is stored in dedicated columns (not appended to notes).
    //    These values power the expanded "Split into N children" card on the parent's timeline.
    tx.execute(
        "UPDATE specimens SET \
         is_archived = 1, archived_at = datetime('now'), updated_at = datetime('now'), \
         health_status = CASE WHEN ?2 IS NOT NULL THEN ?2 ELSE health_status END, \
         notes = CASE WHEN ?3 IS NOT NULL THEN ?3 ELSE notes END, \
         contamination_flag  = ?4, \
         contamination_notes = CASE WHEN ?4 = 1 THEN ?5 ELSE contamination_notes END \
         WHERE id = ?1",
        params![
            request.parent_specimen_id,
            request.health_status,
            request.notes,
            contam_flag_i32,
            request.contamination_notes,
        ],
    ).map_err(|e| format!("Failed to archive parent: {}", e))?;

    // 2. Log the split event on the parent's chain.
    //    This becomes the parent's last entry_hash, which ALL children
    //    will inherit as their shared prev_hash (the cryptographic fork point).
    let audit_detail = if request.contamination_flag.unwrap_or(false) {
        format!(
            "Specimen split into {} children on {} [contamination flagged]",
            request.children.len(), request.date
        )
    } else {
        format!(
            "Specimen split into {} children on {}",
            request.children.len(), request.date
        )
    };
    queries::log_audit(
        &tx, Some(&user.id), "split", "specimen", Some(&request.parent_specimen_id),
        None, None, Some(&audit_detail),
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

        // Insert child specimen with genealogy fields, inherited contamination status,
        // inherited cumulative PDL from the parent (WP-31), and inherited origin_type (WP-42).
        // is_best_performer resets to 0 for every child — selection is re-evaluated per generation.
        tx.execute(
            "INSERT INTO specimens \
             (id, accession_number, species_id, stage, initiation_date, \
              location, health_status, qr_code_data, parent_specimen_id, \
              provenance, source_plant, notes, created_by, \
              generation, lineage_passage_offset, root_specimen_id, \
              contamination_flag, contamination_notes, cumulative_pdl, origin_type) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
            params![
                child_id, accession, parent_species_id, child_stage, request.date,
                child_location, child_health, qr_data, request.parent_specimen_id,
                parent_provenance, parent_source_plant, child_notes, user.id,
                child_generation, child_passage_offset, child_root_id,
                child_contamination_flag_i32, child_contamination_notes,
                parent_cumulative_pdl, parent_origin_type,
            ],
        ).map_err(|e| format!("Failed to create child specimen {}: {}", i + 1, e))?;

        // Fork the audit chain from the parent (all children inherit the same
        // parent prev_hash — the split event logged above — making the fork visible)
        let child_audit_detail = if child_contamination_flag_i32 != 0 {
            format!(
                "Split from {} — container {} of {} [contamination inherited]",
                request.parent_specimen_id, i + 1, request.children.len()
            )
        } else {
            format!(
                "Split from {} — container {} of {}",
                request.parent_specimen_id, i + 1, request.children.len()
            )
        };
        queries::log_audit_for_child(
            &tx, Some(&user.id), "create", "specimen", Some(&child_id),
            None, Some(accession.as_str()),
            Some(&child_audit_detail),
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
    crate::db::dashboard::invalidate_dashboard_cache(&state.dashboard_cache);

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
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }
    // Validate against the vocabulary table; is_terminal = 0 prevents setting 'archived' in bulk.
    let profile = crate::db::vocabulary::active_profile(&db.conn);
    crate::db::vocabulary::require_selectable_stage(&db.conn, &profile, &stage)?;
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
    if count > 0 {
        crate::db::dashboard::invalidate_dashboard_cache(&state.dashboard_cache);
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use rusqlite::{Connection, params};

    /// Minimal schema sufficient to test contamination inheritance SQL.
    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory DB");
        conn.execute_batch(
            "CREATE TABLE species (
                id TEXT PRIMARY KEY,
                species_code TEXT NOT NULL
            );
            CREATE TABLE specimens (
                id TEXT PRIMARY KEY,
                accession_number TEXT NOT NULL,
                species_id TEXT NOT NULL,
                is_archived INTEGER NOT NULL DEFAULT 0,
                contamination_flag INTEGER NOT NULL DEFAULT 0,
                contamination_notes TEXT
            );
            INSERT INTO species (id, species_code) VALUES ('sp1', 'TST-01');",
        )
        .expect("setup DB");
        conn
    }

    /// Mirrors the contamination inheritance computation inside `split_specimen`.
    fn inherit_contamination(
        parent_flag: i32,
        parent_notes: Option<&str>,
        request_flag: Option<bool>,
        request_notes: Option<&str>,
    ) -> (i32, Option<String>) {
        let effective = parent_flag != 0 || request_flag.unwrap_or(false);
        let flag = effective as i32;
        let notes = if effective {
            request_notes.or(parent_notes).map(str::to_string)
        } else {
            None
        };
        (flag, notes)
    }

    // ── Contamination inheritance logic ──────────────────────────────────────

    #[test]
    fn test_split_inherits_contamination_from_contaminated_parent() {
        let (flag, notes) =
            inherit_contamination(1, Some("Fungal contamination observed"), None, None);
        assert_eq!(flag, 1, "child should be flagged when parent is contaminated");
        assert_eq!(
            notes,
            Some("Fungal contamination observed".to_string()),
            "parent contamination notes should carry over to child"
        );
    }

    #[test]
    fn test_split_request_notes_take_precedence_over_parent_notes() {
        let (flag, notes) = inherit_contamination(
            1,
            Some("Older parent record"),
            Some(true),
            Some("Fresh observation at time of split"),
        );
        assert_eq!(flag, 1);
        assert_eq!(
            notes,
            Some("Fresh observation at time of split".to_string()),
            "split-request notes should override parent notes"
        );
    }

    #[test]
    fn test_split_clean_parent_produces_clean_children() {
        let (flag, notes) = inherit_contamination(0, None, None, None);
        assert_eq!(flag, 0, "child of uncontaminated parent should not be flagged");
        assert_eq!(notes, None, "child of clean parent should have no contamination notes");
    }

    // ── DB round-trip: inherited values are persisted correctly ──────────────

    #[test]
    fn test_split_contamination_persisted_to_child_in_db() {
        let conn = setup_db();

        conn.execute(
            "INSERT INTO specimens (id, accession_number, species_id, contamination_flag, contamination_notes)
             VALUES ('parent-1', 'TST-001', 'sp1', 1, 'Bacterial contamination')",
            [],
        )
        .unwrap();

        // Fetch parent contamination as split_specimen does.
        let (parent_flag, parent_notes): (i32, Option<String>) = conn
            .query_row(
                "SELECT contamination_flag, contamination_notes
                 FROM specimens WHERE id = ?1 AND is_archived = 0",
                params!["parent-1"],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();

        let (child_flag, child_notes) =
            inherit_contamination(parent_flag, parent_notes.as_deref(), None, None);

        // Insert child with the inherited contamination values.
        conn.execute(
            "INSERT INTO specimens
             (id, accession_number, species_id, contamination_flag, contamination_notes)
             VALUES ('child-1', 'TST-001-A', 'sp1', ?1, ?2)",
            params![child_flag, child_notes],
        )
        .unwrap();

        let (stored_flag, stored_notes): (i32, Option<String>) = conn
            .query_row(
                "SELECT contamination_flag, contamination_notes
                 FROM specimens WHERE id = ?1",
                params!["child-1"],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();

        assert_eq!(stored_flag, 1);
        assert_eq!(stored_notes, Some("Bacterial contamination".to_string()));
    }

    #[test]
    fn test_split_request_can_force_contamination_even_on_clean_parent() {
        // A technician may observe contamination during the split procedure
        // even though the parent record was previously clean. The request flag
        // alone must be sufficient to flag all children.
        let (flag, notes) = inherit_contamination(
            0,
            None,
            Some(true),
            Some("Contamination detected during split procedure"),
        );
        assert_eq!(flag, 1, "request contamination flag should propagate to child even when parent is clean");
        assert_eq!(notes, Some("Contamination detected during split procedure".to_string()));
    }
}

use crate::auth as auth_service;
use crate::db::queries;
use crate::models::compliance::*;
use crate::models::specimen::PaginatedResponse;
use crate::AppState;
use rusqlite::params;
use tauri::State;

#[tauri::command]
pub fn list_compliance_records(
    state: State<AppState>,
    token: String,
    specimen_id: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
) -> Result<PaginatedResponse<ComplianceRecord>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;

    let pg = queries::PaginationParams {
        page: page.unwrap_or(1),
        per_page: per_page.unwrap_or(100),
    };

    let (where_clause, bind_vals): (String, Vec<Box<dyn rusqlite::types::ToSql>>) =
        if let Some(ref sid) = specimen_id {
            (
                "WHERE cr.specimen_id = ?1".to_string(),
                vec![Box::new(sid.clone()) as Box<dyn rusqlite::types::ToSql>],
            )
        } else {
            (String::new(), vec![])
        };

    let count_sql = format!(
        "SELECT COUNT(*) FROM compliance_records cr {}",
        where_clause
    );
    let bind_refs: Vec<&dyn rusqlite::types::ToSql> = bind_vals.iter().map(|v| v.as_ref()).collect();
    let total: i64 = db.conn
        .query_row(&count_sql, bind_refs.as_slice(), |r| r.get(0))
        .map_err(|e| e.to_string())?;

    let limit_idx = bind_vals.len() + 1;
    let offset_idx = bind_vals.len() + 2;
    let sql = format!(
        "SELECT cr.*, s.accession_number as specimen_accession
         FROM compliance_records cr
         LEFT JOIN specimens s ON cr.specimen_id = s.id
         {}
         ORDER BY cr.created_at DESC
         LIMIT ?{} OFFSET ?{}",
        where_clause, limit_idx, offset_idx
    );

    let mut all_vals: Vec<Box<dyn rusqlite::types::ToSql>> = bind_vals;
    all_vals.push(Box::new(pg.limit()));
    all_vals.push(Box::new(pg.offset()));
    let bind_refs2: Vec<&dyn rusqlite::types::ToSql> = all_vals.iter().map(|v| v.as_ref()).collect();

    let mut stmt = db.conn.prepare(&sql).map_err(|e| e.to_string())?;

    let items = stmt.query_map(bind_refs2.as_slice(), |row| {
        Ok(ComplianceRecord {
            id: row.get("id")?,
            specimen_id: row.get("specimen_id")?,
            specimen_accession: row.get("specimen_accession")?,
            record_type: row.get("record_type")?,
            agency: row.get("agency")?,
            permit_number: row.get("permit_number")?,
            permit_expiry: row.get("permit_expiry")?,
            test_type: row.get("test_type")?,
            test_method: row.get("test_method")?,
            test_date: row.get("test_date")?,
            test_lab: row.get("test_lab")?,
            test_result: row.get("test_result")?,
            status: row.get("status")?,
            flag_reason: row.get("flag_reason")?,
            chain_of_custody: row.get("chain_of_custody")?,
            notes: row.get("notes")?,
            document_path: row.get("document_path")?,
            created_by: row.get("created_by")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }).map_err(|e| e.to_string())?
      .filter_map(|r| r.ok())
      .collect();

    let total_pages = ((total as f64) / (pg.per_page as f64)).ceil() as u32;
    Ok(PaginatedResponse {
        items,
        total,
        page: pg.page,
        per_page: pg.per_page,
        total_pages,
    })
}

#[tauri::command]
pub fn create_compliance_record(
    state: State<AppState>,
    token: String,
    request: CreateComplianceRequest,
) -> Result<ComplianceRecord, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }

    let id = uuid::Uuid::new_v4().to_string();

    db.conn.execute(
        "INSERT INTO compliance_records (id, specimen_id, record_type, agency, permit_number,
         permit_expiry, test_type, test_method, test_date, test_lab, test_result, status,
         chain_of_custody, notes, created_by)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        params![
            id, request.specimen_id, request.record_type, request.agency,
            request.permit_number, request.permit_expiry, request.test_type,
            request.test_method, request.test_date, request.test_lab,
            request.test_result, request.status.as_deref().unwrap_or("valid"),
            request.chain_of_custody, request.notes, user.id,
        ],
    ).map_err(|e| format!("Failed to create compliance record: {}", e))?;

    queries::log_audit(
        &db.conn, Some(&user.id), "create", "compliance", Some(&id),
        None, None, Some(&format!("Compliance record: {}", request.record_type)),
    ).ok();

    db.conn.query_row(
        "SELECT cr.*, s.accession_number as specimen_accession
         FROM compliance_records cr
         LEFT JOIN specimens s ON cr.specimen_id = s.id
         WHERE cr.id = ?1",
        params![id],
        |row| {
            Ok(ComplianceRecord {
                id: row.get("id")?,
                specimen_id: row.get("specimen_id")?,
                specimen_accession: row.get("specimen_accession")?,
                record_type: row.get("record_type")?,
                agency: row.get("agency")?,
                permit_number: row.get("permit_number")?,
                permit_expiry: row.get("permit_expiry")?,
                test_type: row.get("test_type")?,
                test_method: row.get("test_method")?,
                test_date: row.get("test_date")?,
                test_lab: row.get("test_lab")?,
                test_result: row.get("test_result")?,
                status: row.get("status")?,
                flag_reason: row.get("flag_reason")?,
                chain_of_custody: row.get("chain_of_custody")?,
                notes: row.get("notes")?,
                document_path: row.get("document_path")?,
                created_by: row.get("created_by")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        },
    ).map_err(|e| format!("Failed to fetch compliance record: {}", e))
}

#[tauri::command]
pub fn update_compliance_record(
    state: State<AppState>,
    token: String,
    request: UpdateComplianceRequest,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }

    let mut updates = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(ref tr) = request.test_result {
        updates.push(format!("test_result = ?{}", values.len() + 1));
        values.push(Box::new(tr.clone()));
    }
    if let Some(ref status) = request.status {
        updates.push(format!("status = ?{}", values.len() + 1));
        values.push(Box::new(status.clone()));
    }
    if let Some(ref fr) = request.flag_reason {
        updates.push(format!("flag_reason = ?{}", values.len() + 1));
        values.push(Box::new(fr.clone()));
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
        "UPDATE compliance_records SET {} WHERE id = ?{}",
        updates.join(", "),
        values.len() + 1
    );
    values.push(Box::new(request.id.clone()));

    let bind_refs: Vec<&dyn rusqlite::types::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    db.conn.execute(&sql, bind_refs.as_slice())
        .map_err(|e| format!("Failed to update compliance record: {}", e))?;

    queries::log_audit(
        &db.conn, Some(&user.id), "update", "compliance", Some(&request.id),
        None, None, None,
    ).ok();

    Ok(())
}

#[tauri::command]
pub fn get_compliance_flags(state: State<AppState>, token: String) -> Result<Vec<ComplianceFlag>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;

    let mut flags = Vec::new();

    // Flag: Expired permits
    {
        let mut stmt = db.conn.prepare(
            "SELECT s.id, s.accession_number, sp.species_code
             FROM specimens s
             JOIN species sp ON s.species_id = sp.id
             WHERE s.permit_expiry IS NOT NULL AND s.permit_expiry < date('now')
             AND s.is_archived = 0"
        ).map_err(|e| e.to_string())?;

        let expired: Vec<ComplianceFlag> = stmt.query_map([], |row| {
            Ok(ComplianceFlag {
                specimen_id: row.get(0)?,
                accession_number: row.get(1)?,
                species_code: row.get(2)?,
                flag_type: "expired_permit".to_string(),
                message: "Permit has expired".to_string(),
                severity: "critical".to_string(),
            })
        }).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();
        flags.extend(expired);
    }

    // Flag: Citrus missing HLB test in last 12 months
    {
        let mut stmt = db.conn.prepare(
            "SELECT s.id, s.accession_number, sp.species_code
             FROM specimens s
             JOIN species sp ON s.species_id = sp.id
             WHERE sp.species_code LIKE 'CIT-%'
             AND s.is_archived = 0
             AND s.id NOT IN (
                 SELECT specimen_id FROM compliance_records
                 WHERE test_type = 'HLB' AND test_date >= date('now', '-12 months')
                 AND test_result IS NOT NULL
             )"
        ).map_err(|e| e.to_string())?;

        let missing_hlb: Vec<ComplianceFlag> = stmt.query_map([], |row| {
            Ok(ComplianceFlag {
                specimen_id: row.get(0)?,
                accession_number: row.get(1)?,
                species_code: row.get(2)?,
                flag_type: "missing_hlb_test".to_string(),
                message: "Citrus specimen missing HLB test in last 12 months".to_string(),
                severity: "critical".to_string(),
            })
        }).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();
        flags.extend(missing_hlb);
    }

    // Flag: Quarantined without release date
    {
        let mut stmt = db.conn.prepare(
            "SELECT s.id, s.accession_number, sp.species_code
             FROM specimens s
             JOIN species sp ON s.species_id = sp.id
             WHERE s.quarantine_flag = 1 AND s.quarantine_release_date IS NULL
             AND s.is_archived = 0"
        ).map_err(|e| e.to_string())?;

        let quarantine: Vec<ComplianceFlag> = stmt.query_map([], |row| {
            Ok(ComplianceFlag {
                specimen_id: row.get(0)?,
                accession_number: row.get(1)?,
                species_code: row.get(2)?,
                flag_type: "quarantine_no_release".to_string(),
                message: "Quarantined specimen has no scheduled release date".to_string(),
                severity: "high".to_string(),
            })
        }).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();
        flags.extend(quarantine);
    }

    // Flag: Missing disease tests for specimens with positive results
    {
        let mut stmt = db.conn.prepare(
            "SELECT DISTINCT s.id, s.accession_number, sp.species_code
             FROM specimens s
             JOIN species sp ON s.species_id = sp.id
             JOIN compliance_records cr ON cr.specimen_id = s.id
             WHERE cr.test_result = 'positive' AND s.quarantine_flag = 0
             AND s.is_archived = 0"
        ).map_err(|e| e.to_string())?;

        let positive_no_quarantine: Vec<ComplianceFlag> = stmt.query_map([], |row| {
            Ok(ComplianceFlag {
                specimen_id: row.get(0)?,
                accession_number: row.get(1)?,
                species_code: row.get(2)?,
                flag_type: "positive_not_quarantined".to_string(),
                message: "Specimen has positive disease test but is not quarantined".to_string(),
                severity: "critical".to_string(),
            })
        }).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();
        flags.extend(positive_no_quarantine);
    }

    Ok(flags)
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    /// Minimal schema for compliance flag query tests.
    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory DB");
        conn.execute_batch(
            "CREATE TABLE species (
                id TEXT PRIMARY KEY,
                genus TEXT NOT NULL,
                species_name TEXT NOT NULL,
                species_code TEXT NOT NULL UNIQUE
            );
            CREATE TABLE specimens (
                id TEXT PRIMARY KEY,
                accession_number TEXT NOT NULL UNIQUE,
                species_id TEXT NOT NULL REFERENCES species(id),
                permit_expiry TEXT,
                quarantine_flag INTEGER NOT NULL DEFAULT 0,
                quarantine_release_date TEXT,
                is_archived INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE compliance_records (
                id TEXT PRIMARY KEY,
                specimen_id TEXT NOT NULL REFERENCES specimens(id),
                test_type TEXT,
                test_date TEXT,
                test_result TEXT
            );",
        )
        .expect("create tables");
        conn
    }

    fn insert_species(conn: &Connection, id: &str, code: &str) {
        conn.execute(
            "INSERT INTO species (id, genus, species_name, species_code) VALUES (?1,'G','sp',?2)",
            rusqlite::params![id, code],
        )
        .unwrap();
    }

    fn insert_specimen(conn: &Connection, id: &str, sp_id: &str, accession: &str) {
        conn.execute(
            "INSERT INTO specimens (id, accession_number, species_id) VALUES (?1,?2,?3)",
            rusqlite::params![id, accession, sp_id],
        )
        .unwrap();
    }

    // ── Expired permit flag ───────────────────────────────────────────────────

    #[test]
    fn flag_expired_permit_detected() {
        let conn = setup_db();
        insert_species(&conn, "sp1", "CIT-01");
        conn.execute(
            "INSERT INTO specimens (id,accession_number,species_id,permit_expiry)
             VALUES ('s1','2024-01-01-CIT-01-001','sp1','2020-01-01')",
            [],
        )
        .unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM specimens s JOIN species sp ON s.species_id=sp.id
                 WHERE s.permit_expiry IS NOT NULL AND s.permit_expiry < date('now') AND s.is_archived=0",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn flag_valid_permit_not_flagged() {
        let conn = setup_db();
        insert_species(&conn, "sp1", "CIT-01");
        conn.execute(
            "INSERT INTO specimens (id,accession_number,species_id,permit_expiry)
             VALUES ('s1','2024-01-01-CIT-01-001','sp1','2099-12-31')",
            [],
        )
        .unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM specimens s JOIN species sp ON s.species_id=sp.id
                 WHERE s.permit_expiry IS NOT NULL AND s.permit_expiry < date('now') AND s.is_archived=0",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    // ── Quarantine without release date flag ──────────────────────────────────

    #[test]
    fn flag_quarantine_no_release_date_detected() {
        let conn = setup_db();
        insert_species(&conn, "sp1", "ABC");
        conn.execute(
            "INSERT INTO specimens (id,accession_number,species_id,quarantine_flag)
             VALUES ('s1','ACC-001','sp1',1)",
            [],
        )
        .unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM specimens s JOIN species sp ON s.species_id=sp.id
                 WHERE s.quarantine_flag=1 AND s.quarantine_release_date IS NULL AND s.is_archived=0",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn flag_quarantine_with_release_date_not_flagged() {
        let conn = setup_db();
        insert_species(&conn, "sp1", "ABC");
        conn.execute(
            "INSERT INTO specimens (id,accession_number,species_id,quarantine_flag,quarantine_release_date)
             VALUES ('s1','ACC-001','sp1',1,'2099-12-31')",
            [],
        )
        .unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM specimens s JOIN species sp ON s.species_id=sp.id
                 WHERE s.quarantine_flag=1 AND s.quarantine_release_date IS NULL AND s.is_archived=0",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    // ── Positive test without quarantine flag ─────────────────────────────────

    #[test]
    fn flag_positive_test_not_quarantined() {
        let conn = setup_db();
        insert_species(&conn, "sp1", "VAC-01");
        insert_specimen(&conn, "s1", "sp1", "ACC-001");
        conn.execute(
            "INSERT INTO compliance_records (id,specimen_id,test_result) VALUES ('cr1','s1','positive')",
            [],
        )
        .unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT s.id) FROM specimens s
                 JOIN species sp ON s.species_id=sp.id
                 JOIN compliance_records cr ON cr.specimen_id=s.id
                 WHERE cr.test_result='positive' AND s.quarantine_flag=0 AND s.is_archived=0",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn no_flag_positive_test_already_quarantined() {
        let conn = setup_db();
        insert_species(&conn, "sp1", "VAC-01");
        conn.execute(
            "INSERT INTO specimens (id,accession_number,species_id,quarantine_flag)
             VALUES ('s1','ACC-001','sp1',1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO compliance_records (id,specimen_id,test_result) VALUES ('cr1','s1','positive')",
            [],
        )
        .unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT s.id) FROM specimens s
                 JOIN species sp ON s.species_id=sp.id
                 JOIN compliance_records cr ON cr.specimen_id=s.id
                 WHERE cr.test_result='positive' AND s.quarantine_flag=0 AND s.is_archived=0",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    // ── Citrus HLB flag ───────────────────────────────────────────────────────

    #[test]
    fn flag_citrus_missing_hlb_test_detected() {
        let conn = setup_db();
        insert_species(&conn, "sp1", "CIT-02");
        insert_specimen(&conn, "s1", "sp1", "ACC-001");
        // No HLB compliance record inserted → should be flagged.

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM specimens s JOIN species sp ON s.species_id=sp.id
                 WHERE sp.species_code LIKE 'CIT-%' AND s.is_archived=0
                 AND s.id NOT IN (
                     SELECT specimen_id FROM compliance_records
                     WHERE test_type='HLB' AND test_date >= date('now','-12 months')
                     AND test_result IS NOT NULL
                 )",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn flag_citrus_recent_hlb_test_not_flagged() {
        let conn = setup_db();
        insert_species(&conn, "sp1", "CIT-02");
        insert_specimen(&conn, "s1", "sp1", "ACC-001");
        conn.execute(
            "INSERT INTO compliance_records (id,specimen_id,test_type,test_date,test_result)
             VALUES ('cr1','s1','HLB',date('now','-30 days'),'negative')",
            [],
        )
        .unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM specimens s JOIN species sp ON s.species_id=sp.id
                 WHERE sp.species_code LIKE 'CIT-%' AND s.is_archived=0
                 AND s.id NOT IN (
                     SELECT specimen_id FROM compliance_records
                     WHERE test_type='HLB' AND test_date >= date('now','-12 months')
                     AND test_result IS NOT NULL
                 )",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    // ── Archived specimens are excluded from all flags ─────────────────────────

    #[test]
    fn archived_specimens_excluded_from_flags() {
        let conn = setup_db();
        insert_species(&conn, "sp1", "CIT-01");
        conn.execute(
            "INSERT INTO specimens (id,accession_number,species_id,permit_expiry,is_archived)
             VALUES ('s1','ACC-001','sp1','2020-01-01',1)",
            [],
        )
        .unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM specimens s JOIN species sp ON s.species_id=sp.id
                 WHERE s.permit_expiry IS NOT NULL AND s.permit_expiry < date('now') AND s.is_archived=0",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }
}

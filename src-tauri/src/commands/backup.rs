use crate::auth as auth_service;
use crate::db::queries;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn create_backup(
    state: State<AppState>,
    token: String,
    destination: Option<String>,
) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err("Only supervisors and admins can create backups".to_string());
    }

    let db_path = crate::db::Database::db_path();

    if !db_path.exists() {
        return Err("Database file not found (using in-memory database)".to_string());
    }

    // Checkpoint WAL before copying so the .db file is a self-contained snapshot.
    // TRUNCATE waits for all readers, fully checkpoints, then empties the WAL.
    // We verify the result: if busy_frames > 0 an active reader prevented a full
    // checkpoint; the copy is still consistent (WAL is append-only) but the DB
    // file alone may be missing recent frames, so we abort and ask the caller to retry.
    let (busy_frames, _log_frames, _ckpt_frames): (i64, i64, i64) = db.conn
        .query_row("PRAGMA wal_checkpoint(TRUNCATE);", [], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .map_err(|e| format!("Failed to checkpoint WAL: {}", e))?;

    if busy_frames > 0 {
        return Err(format!(
            "WAL checkpoint incomplete: {} frame(s) held by active readers. \
             Close all other connections and retry the backup.",
            busy_frames
        ));
    }

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let backup_name = format!("stelo_ptc_backup_{}.db", timestamp);

    let backup_path = if let Some(dest) = destination {
        let dest_path = std::path::PathBuf::from(&dest);
        if dest_path.is_dir() {
            dest_path.join(&backup_name)
        } else {
            dest_path
        }
    } else {
        // Default: backup directory next to the database
        let backup_dir = db_path
            .parent()
            .ok_or_else(|| "Could not determine database parent directory".to_string())?
            .join("backups");
        std::fs::create_dir_all(&backup_dir)
            .map_err(|e| format!("Failed to create backup directory: {}", e))?;
        backup_dir.join(&backup_name)
    };

    std::fs::copy(&db_path, &backup_path)
        .map_err(|e| format!("Failed to copy database: {}", e))?;

    let backup_path_str = backup_path.to_string_lossy().to_string();

    queries::log_audit(
        &db.conn,
        Some(&user.id),
        "create",
        "backup",
        None,
        None,
        Some(&backup_path_str),
        Some("Database backup created"),
    )
    .ok();

    Ok(backup_path_str)
}

#[tauri::command]
pub fn list_backups(state: State<AppState>, token: String) -> Result<Vec<BackupInfo>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;

    let db_path = crate::db::Database::db_path();
    let backup_dir = db_path
        .parent()
        .ok_or_else(|| "Could not determine database parent directory".to_string())?
        .join("backups");

    if !backup_dir.exists() {
        return Ok(vec![]);
    }

    let mut backups: Vec<BackupInfo> = std::fs::read_dir(&backup_dir)
        .map_err(|e| format!("Failed to read backup directory: {}", e))?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("stelo_ptc_backup_") || !name.ends_with(".db") {
                return None;
            }
            let meta = entry.metadata().ok()?;
            let modified = meta
                .modified()
                .ok()?
                .duration_since(std::time::UNIX_EPOCH)
                .ok()?
                .as_secs();
            Some(BackupInfo {
                file_name: name,
                path: entry.path().to_string_lossy().to_string(),
                size_bytes: meta.len(),
                created_at: chrono::DateTime::from_timestamp(modified as i64, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_default(),
            })
        })
        .collect();

    backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(backups)
}

#[derive(serde::Serialize)]
pub struct BackupInfo {
    pub file_name: String,
    pub path: String,
    pub size_bytes: u64,
    pub created_at: String,
}

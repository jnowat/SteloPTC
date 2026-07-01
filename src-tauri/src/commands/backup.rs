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

    // Auto-checkpoint eligible lineages before the WAL snapshot when enabled.
    // Runs silently — a failure here must never block the backup itself.
    let on_backup = queries::read_setting(&db.conn, "auto_checkpoint_on_backup", "1") == "1";
    let auto_enabled = queries::read_setting(&db.conn, "auto_checkpoint_enabled", "1") == "1";
    if on_backup && auto_enabled {
        // interval=0 means: checkpoint every lineage with any uncovered entries.
        let _ = queries::auto_checkpoint_lineages(&db.conn, &user.id, "backup", 0);
    }

    // Checkpoint WAL before copying so the .db file is a self-contained snapshot.
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

    // Security: `smtp_config.password` is stored in plaintext in the live
    // database (see the WP-52 migration comment in db/migrations.rs for the
    // disclosed trade-off — no OS-keychain integration yet). A backup file
    // may be copied to removable media, uploaded to cloud storage, or handed
    // to support, so it must not carry that secret even though the live
    // database does. Redact it in the copy only; the live database and the
    // SMTP-sending code path are untouched. Other smtp_config columns
    // (host/port/username/from_address) aren't secret and are left intact so
    // restoring from a backup doesn't force reconfiguring the whole mail
    // server — just re-entering the password.
    if let Err(e) = redact_smtp_password_in_backup(&backup_path) {
        let _ = std::fs::remove_file(&backup_path);
        return Err(format!(
            "Failed to finalize backup (SMTP credential redaction): {}",
            e
        ));
    }

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

#[tauri::command]
pub fn restore_backup(
    app: tauri::AppHandle,
    state: State<AppState>,
    token: String,
    backup_path: String,
) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.is_admin() {
        return Err("Only admins can restore from a backup".to_string());
    }

    let src = std::path::PathBuf::from(&backup_path);

    if !src.exists() {
        return Err("Backup file not found".to_string());
    }

    // Validate: must match the backup filename pattern
    let file_name = src
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    if !file_name.starts_with("stelo_ptc_backup_") || !file_name.ends_with(".db") {
        return Err("File does not appear to be a valid SteloPTC backup".to_string());
    }

    // Validate SQLite magic bytes (first 16 bytes: "SQLite format 3\0")
    let mut magic = [0u8; 16];
    let mut f = std::fs::File::open(&src)
        .map_err(|e| format!("Cannot open backup file: {}", e))?;
    std::io::Read::read_exact(&mut f, &mut magic)
        .map_err(|_| "Backup file is too small to be a valid database".to_string())?;
    if &magic != b"SQLite format 3\0" {
        return Err("Backup file is not a valid SQLite database".to_string());
    }

    let db_path = crate::db::Database::db_path();

    // Checkpoint current WAL so the live database file is self-contained
    // before we overwrite it (preserves the existing WAL checkpoint logic).
    let _ = db.conn.query_row("PRAGMA wal_checkpoint(TRUNCATE);", [], |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?))
    });

    std::fs::copy(&src, &db_path)
        .map_err(|e| format!("Failed to restore backup: {}", e))?;

    // Also remove any stale WAL / SHM files so the restored snapshot is clean
    for ext in &["-wal", "-shm"] {
        let side = db_path.with_extension(format!("db{}", ext));
        let _ = std::fs::remove_file(side);
    }

    queries::log_audit(
        &db.conn,
        Some(&user.id),
        "restore",
        "backup",
        None,
        None,
        Some(&backup_path),
        Some("Database restored from backup"),
    )
    .ok();

    // Restart the application so it re-opens the restored database
    app.restart();
}

/// Clears `smtp_config.password` in the database file at `backup_path`,
/// leaving every other column untouched. Operates on a standalone file copy,
/// never the live database — see the call site in `create_backup`.
///
/// Forces `journal_mode = DELETE` on this one-shot connection before writing.
/// The source `.db` file was checkpointed and copied as a self-contained
/// snapshot, but its header still records WAL mode, so a plain write here
/// would otherwise land in a freshly created sibling `-wal` file rather than
/// the `.db` file itself — meaning a backup file copied or uploaded on its
/// own (without that sibling) would silently carry the *unredacted*
/// password. Switching to DELETE mode first forces SQLite to checkpoint and
/// write directly into the single `.db` file, and leaves no `-wal`/`-shm`
/// behind afterward.
fn redact_smtp_password_in_backup(backup_path: &std::path::Path) -> Result<(), String> {
    let conn = rusqlite::Connection::open(backup_path).map_err(|e| e.to_string())?;
    conn.pragma_update(None, "journal_mode", "DELETE")
        .map_err(|e| e.to_string())?;
    conn.execute("UPDATE smtp_config SET password = NULL WHERE id = 1", [])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(serde::Serialize)]
pub struct BackupInfo {
    pub file_name: String,
    pub path: String,
    pub size_bytes: u64,
    pub created_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    /// Unique-per-test temp file path; no `tempfile` dependency needed for
    /// this one-off use. Callers must clean up with `cleanup_db_file`.
    fn temp_db_path(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "steloptc_backup_test_{}_{}.db",
            label,
            uuid::Uuid::new_v4()
        ))
    }

    fn cleanup_db_file(path: &std::path::Path) {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(path.with_extension("db-wal"));
        let _ = std::fs::remove_file(path.with_extension("db-shm"));
        let _ = std::fs::remove_file(path.with_extension("db-journal"));
    }

    #[test]
    fn redact_smtp_password_in_backup_clears_password_only() {
        let path = temp_db_path("redact");
        {
            let conn = Connection::open(&path).unwrap();
            crate::db::migrations::run_all(&conn).unwrap();
            conn.execute(
                "UPDATE smtp_config SET host = 'smtp.example.com', username = 'lab@example.com', \
                 password = 'super-secret', from_address = 'lab@example.com' WHERE id = 1",
                [],
            )
            .unwrap();
        }

        redact_smtp_password_in_backup(&path).unwrap();

        let conn = Connection::open(&path).unwrap();
        let (host, username, password, from_address): (
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        ) = conn
            .query_row(
                "SELECT host, username, password, from_address FROM smtp_config WHERE id = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        drop(conn);

        assert_eq!(password, None, "password must be redacted in the backup file");
        assert_eq!(host.as_deref(), Some("smtp.example.com"), "non-secret fields must survive redaction");
        assert_eq!(username.as_deref(), Some("lab@example.com"));
        assert_eq!(from_address.as_deref(), Some("lab@example.com"));

        cleanup_db_file(&path);
    }

    #[test]
    fn redact_smtp_password_in_backup_succeeds_when_no_password_was_set() {
        let path = temp_db_path("redact_empty");
        {
            let conn = Connection::open(&path).unwrap();
            crate::db::migrations::run_all(&conn).unwrap();
        }

        redact_smtp_password_in_backup(&path).unwrap();

        cleanup_db_file(&path);
    }

    #[test]
    fn redact_smtp_password_in_backup_leaves_no_wal_sidecar_file() {
        let path = temp_db_path("redact_wal");
        {
            let conn = Connection::open(&path).unwrap();
            crate::db::migrations::run_all(&conn).unwrap();
            conn.execute(
                "UPDATE smtp_config SET password = 'super-secret' WHERE id = 1",
                [],
            )
            .unwrap();
        }

        redact_smtp_password_in_backup(&path).unwrap();

        // A `.db-wal` sidecar left behind would mean a copy of just the
        // `.db` file could still carry the unredacted password — see the
        // doc comment on `redact_smtp_password_in_backup`.
        assert!(
            !path.with_extension("db-wal").exists(),
            "redaction must not leave a WAL sidecar file that could carry the unredacted password"
        );

        cleanup_db_file(&path);
    }
}

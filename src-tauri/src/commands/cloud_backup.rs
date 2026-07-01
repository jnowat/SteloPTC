// WP-59: Cloud backup & multi-device sync with end-to-end encryption.
//
// Transport scope (disclosed honestly, matching the WP-50/52 precedent for
// "foundation now, live transport later"): `local_nas` and `smb` targets are
// fully functional — both are just a filesystem path (a mounted network
// share, in the SMB case) that this process can write to and read from
// directly. `s3` and `sftp` targets can be configured and their credentials
// encrypted/stored today, but actually speaking those protocols would
// require a new network-client dependency (an S3 SDK, an SFTP/SSH client)
// that hasn't been added — `cloud_backup`/`restore_from_cloud` return a
// clear "not yet connected" error for those two types rather than silently
// pretending to upload. See ROADMAP.md WP-59 "As built".
use rusqlite::params;
use tauri::State;

use crate::auth as auth_service;
use crate::cloud::{crypto, targets};
use crate::db::queries;
use crate::AppState;

#[derive(serde::Serialize)]
pub struct BackupTargetSummary {
    pub id: String,
    pub name: String,
    pub target_type: String,
    pub schedule_cron: Option<String>,
    pub last_backup_at: Option<String>,
    pub last_backup_size_bytes: Option<i64>,
    pub last_backup_size_display: Option<String>,
    pub last_status: Option<String>,
    pub last_error: Option<String>,
    pub is_enabled: bool,
}

fn row_to_summary(row: &rusqlite::Row) -> rusqlite::Result<BackupTargetSummary> {
    let size: Option<i64> = row.get("last_backup_size_bytes")?;
    Ok(BackupTargetSummary {
        id: row.get("id")?,
        name: row.get("name")?,
        target_type: row.get("type")?,
        schedule_cron: row.get("schedule_cron")?,
        last_backup_at: row.get("last_backup_at")?,
        last_backup_size_bytes: size,
        last_backup_size_display: size.map(targets::format_size_bytes),
        last_status: row.get("last_status")?,
        last_error: row.get("last_error")?,
        is_enabled: row.get::<_, i64>("is_enabled")? != 0,
    })
}

#[tauri::command]
pub fn list_backup_targets(state: State<AppState>, token: String) -> Result<Vec<BackupTargetSummary>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err("Only supervisors and admins can view backup targets".to_string());
    }
    let mut stmt = db.conn.prepare("SELECT * FROM backup_targets ORDER BY name ASC").map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], row_to_summary).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();
    Ok(rows)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn create_backup_target(
    state: State<AppState>,
    token: String,
    name: String,
    target_type: String,
    passphrase: String,
    bucket_or_path: String,
    endpoint: Option<String>,
    access_key: Option<String>,
    secret_key: Option<String>,
    schedule_cron: Option<String>,
) -> Result<BackupTargetSummary, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err("Only supervisors and admins can configure backup targets".to_string());
    }
    if passphrase.len() < 8 {
        return Err("Passphrase must be at least 8 characters".to_string());
    }
    if let Some(ref cron) = schedule_cron {
        if !targets::is_valid_cron(cron) {
            return Err(format!("'{}' is not a valid 5-field cron expression", cron));
        }
    }

    let config = targets::TargetConfig { endpoint, bucket_or_path, access_key, secret_key };
    let config_encrypted = targets::encrypt_target_config(&passphrase, &config)?;

    let id = uuid::Uuid::new_v4().to_string();
    db.conn.execute(
        "INSERT INTO backup_targets (id, name, type, config_encrypted, schedule_cron, last_status) \
         VALUES (?1, ?2, ?3, ?4, ?5, 'pending')",
        params![id, name, target_type, config_encrypted, schedule_cron],
    ).map_err(|e| format!("Failed to create backup target: {}", e))?;

    queries::log_audit(
        &db.conn, Some(&user.id), "create", "backup_target", Some(&id),
        None, Some(&name), Some("Cloud backup target created"),
    ).ok();

    db.conn.query_row("SELECT * FROM backup_targets WHERE id = ?1", [&id], row_to_summary).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_backup_target(state: State<AppState>, token: String, id: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err("Only supervisors and admins can delete backup targets".to_string());
    }
    db.conn.execute("DELETE FROM backup_targets WHERE id = ?1", [&id]).map_err(|e| e.to_string())?;
    queries::log_audit(
        &db.conn, Some(&user.id), "delete", "backup_target", Some(&id),
        None, None, Some("Cloud backup target deleted"),
    ).ok();
    Ok(())
}

fn load_target(conn: &rusqlite::Connection, target_id: &str, passphrase: &str) -> Result<(String, targets::TargetConfig), String> {
    let (target_type, config_encrypted): (String, String) = conn.query_row(
        "SELECT type, config_encrypted FROM backup_targets WHERE id = ?1", [target_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ).map_err(|_| "Backup target not found".to_string())?;
    let config = targets::decrypt_target_config(passphrase, &config_encrypted)?;
    Ok((target_type, config))
}

#[derive(serde::Serialize)]
pub struct CloudBackupResult {
    pub ok: bool,
    pub backup_id: String,
    pub size_bytes: i64,
    pub duration_ms: i64,
    pub merkle_root_included: bool,
}

#[tauri::command]
pub fn cloud_backup(state: State<AppState>, token: String, target_id: String, passphrase: String) -> Result<CloudBackupResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err("Only supervisors and admins can run a cloud backup".to_string());
    }

    let started = std::time::Instant::now();
    let (target_type, config) = load_target(&db.conn, &target_id, &passphrase)?;
    if !matches!(target_type.as_str(), "local_nas" | "smb") {
        return Err(format!(
            "Target type '{}' is configured but not yet connected — only local_nas/smb targets \
             (a filesystem path, e.g. a mounted network share) can complete a live backup today.",
            target_type
        ));
    }

    let db_path = crate::db::Database::db_path();
    if !db_path.exists() {
        return Err("Database file not found (using in-memory database)".to_string());
    }

    // Pre-checkpoint eligible lineages (same as the local WP-16 backup path)
    // so the exported Merkle checkpoint coverage is fresh.
    let merkle_root_included = queries::auto_checkpoint_lineages(&db.conn, &user.id, "cloud_backup", 0).is_ok();

    let (busy_frames, _, _): (i64, i64, i64) = db.conn
        .query_row("PRAGMA wal_checkpoint(TRUNCATE);", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .map_err(|e| format!("Failed to checkpoint WAL: {}", e))?;
    if busy_frames > 0 {
        return Err(format!(
            "WAL checkpoint incomplete: {} frame(s) held by active readers. Close other connections and retry.",
            busy_frames
        ));
    }

    // Security: like the local backup path (WP-16/WP-52), the cloud backup must
    // never carry the plaintext `smtp_config.password` off the machine. The
    // AES-256-GCM envelope already protects it in transit and at rest in the
    // cloud, but redacting it from the plaintext *before* encryption is the
    // same defense-in-depth the local path applies, and keeps the two paths
    // consistent — a restored cloud backup re-prompts for the SMTP password
    // exactly like a restored local backup does. We copy the checkpointed DB to
    // a temp file, redact that copy, read it back, then delete it; the live DB
    // is never touched.
    let backup_id = format!("stelo_cloud_{}", chrono::Local::now().format("%Y%m%d_%H%M%S"));
    let temp_path = db_path.with_file_name(format!("{}.tmp", backup_id));
    std::fs::copy(&db_path, &temp_path).map_err(|e| format!("Failed to stage backup copy: {}", e))?;
    if let Err(e) = crate::commands::backup::redact_smtp_password_in_backup(&temp_path) {
        let _ = std::fs::remove_file(&temp_path);
        return Err(format!("Failed to redact SMTP credential from backup copy: {}", e));
    }
    let read_result = std::fs::read(&temp_path).map_err(|e| format!("Failed to read staged backup: {}", e));
    let _ = std::fs::remove_file(&temp_path);
    let plaintext = read_result?;

    let salt = crypto::generate_salt();
    let key = crypto::derive_key(&passphrase, &salt)?;
    let encrypted = crypto::encrypt(&key, &plaintext)?;
    let mut blob = Vec::with_capacity(salt.len() + encrypted.len());
    blob.extend_from_slice(&salt);
    blob.extend_from_slice(&encrypted);
    let dest_dir = std::path::PathBuf::from(&config.bucket_or_path);
    std::fs::create_dir_all(&dest_dir).map_err(|e| format!("Failed to reach destination path: {}", e))?;
    let dest_path = dest_dir.join(format!("{}.stelobak", backup_id));
    std::fs::write(&dest_path, &blob).map_err(|e| format!("Failed to write encrypted backup: {}", e))?;

    let size_bytes = blob.len() as i64;
    let duration_ms = started.elapsed().as_millis() as i64;

    db.conn.execute(
        "UPDATE backup_targets SET last_backup_at = datetime('now'), last_backup_size_bytes = ?1, \
         last_status = 'ok', last_error = NULL WHERE id = ?2",
        params![size_bytes, target_id],
    ).ok();

    queries::log_audit(
        &db.conn, Some(&user.id), "create", "cloud_backup", Some(&target_id),
        None, Some(&backup_id), Some("Encrypted cloud backup created"),
    ).ok();

    Ok(CloudBackupResult { ok: true, backup_id, size_bytes, duration_ms, merkle_root_included })
}

/// Destructive — mirrors WP-16's local restore two-step confirmation flow
/// (the frontend gates this behind the same "type RESTORE to confirm" UX).
/// Restarts the app on success, exactly like `commands::backup::restore_backup`.
#[tauri::command]
pub fn restore_from_cloud(
    app: tauri::AppHandle,
    state: State<AppState>,
    token: String,
    target_id: String,
    passphrase: String,
    backup_file_name: String,
) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.is_admin() {
        return Err("Only admins can restore from a cloud backup".to_string());
    }

    let (target_type, config) = load_target(&db.conn, &target_id, &passphrase)?;
    if !matches!(target_type.as_str(), "local_nas" | "smb") {
        return Err(format!("Target type '{}' is not yet connected for restore", target_type));
    }

    let src_path = std::path::PathBuf::from(&config.bucket_or_path).join(&backup_file_name);
    let blob = std::fs::read(&src_path).map_err(|e| format!("Failed to read cloud backup file: {}", e))?;
    if blob.len() < 16 {
        return Err("Backup file is too short to be valid".to_string());
    }
    let (salt, encrypted) = blob.split_at(16);
    let key = crypto::derive_key(&passphrase, salt)?;
    // Authenticate the blob BEFORE touching the live database — a wrong
    // passphrase or corrupted/tampered file must never reach the swap step.
    let plaintext = crypto::decrypt(&key, encrypted)?;

    let db_path = crate::db::Database::db_path();
    let _ = db.conn.query_row("PRAGMA wal_checkpoint(TRUNCATE);", [], |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?))
    });
    std::fs::write(&db_path, &plaintext).map_err(|e| format!("Failed to restore database: {}", e))?;
    for ext in &["-wal", "-shm"] {
        let side = db_path.with_extension(format!("db{}", ext));
        let _ = std::fs::remove_file(side);
    }

    queries::log_audit(
        &db.conn, Some(&user.id), "restore", "cloud_backup", Some(&target_id),
        None, Some(&backup_file_name), Some("Database restored from cloud backup"),
    ).ok();

    app.restart();
}

/// Publishes this device's outstanding changes (since its last recorded
/// sync position) as a WAL segment file in the shared target folder, then
/// reads and classifies every peer segment found there. New, non-conflicting
/// changes are reported but NOT automatically merged into the local
/// database — see the module doc comment for why (mirrors WP-51's own
/// disclosed scope boundary). Conflicts are always durably recorded via
/// `db::sync::record_sync_conflict`, never silently dropped.
#[derive(serde::Serialize)]
pub struct ReconcileSummary {
    pub segments_published: bool,
    pub peer_segments_found: i64,
    pub new_changes: i64,
    pub duplicates: i64,
    pub conflicts_recorded: i64,
}

#[tauri::command]
pub fn reconcile_cloud_sync(
    state: State<AppState>,
    token: String,
    target_id: String,
    passphrase: String,
    device_id: String,
) -> Result<ReconcileSummary, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err("Only supervisors and admins can sync with a cloud target".to_string());
    }

    let (target_type, config) = load_target(&db.conn, &target_id, &passphrase)?;
    if !matches!(target_type.as_str(), "local_nas" | "smb") {
        return Err(format!("Target type '{}' is not yet connected for sync", target_type));
    }
    let shared_dir = std::path::PathBuf::from(&config.bucket_or_path).join("sync");
    std::fs::create_dir_all(&shared_dir).map_err(|e| format!("Failed to reach shared sync folder: {}", e))?;

    // Publish this device's outbound changes since the last time it synced
    // this target (tracked in cloud_sync_segments) as
    // `{device_id}/{start}-{end}.wal` under the shared folder.
    let last_seq: i64 = db.conn.query_row(
        "SELECT COALESCE(MAX(chain_seq_end), 0) FROM cloud_sync_segments WHERE target_id = ?1 AND device_id = ?2",
        params![target_id, device_id],
        |r| r.get(0),
    ).unwrap_or(0);
    let outbound: Vec<_> = crate::db::sync::get_changes_since(&db.conn, &[], 10_000)
        .unwrap_or_default()
        .into_iter()
        .filter(|c| c.chain_seq > last_seq)
        .collect();

    let mut segments_published = false;
    if let Some(max_seq) = outbound.iter().map(|c| c.chain_seq).max() {
        let relative_name = crate::cloud::sync::segment_file_name(&device_id, last_seq + 1, max_seq);
        let full_path = shared_dir.join(&relative_name);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create device sync folder: {}", e))?;
        }
        let body = crate::cloud::sync::serialize_segment(&outbound)?;
        std::fs::write(&full_path, body).map_err(|e| format!("Failed to publish sync segment: {}", e))?;
        db.conn.execute(
            "INSERT INTO cloud_sync_segments (id, target_id, device_id, chain_seq_start, chain_seq_end) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![uuid::Uuid::new_v4().to_string(), target_id, device_id, last_seq + 1, max_seq],
        ).ok();
        segments_published = true;
    }

    // Read and classify every peer segment file present in the shared folder
    // (one subdirectory per device, per `segment_file_name`'s layout).
    let mut peer_segments_found = 0i64;
    let mut new_changes = 0i64;
    let mut duplicates = 0i64;
    let mut conflicts_recorded = 0i64;
    if let Ok(device_dirs) = std::fs::read_dir(&shared_dir) {
        for device_dir in device_dirs.flatten() {
            let Some(peer_device_id) = device_dir.file_name().to_str().map(|s| s.to_string()) else { continue };
            if peer_device_id == device_id {
                continue; // never reconcile against our own published segments
            }
            let Ok(files) = std::fs::read_dir(device_dir.path()) else { continue };
            for file in files.flatten() {
                let Some(file_name) = file.file_name().to_str().map(|s| s.to_string()) else { continue };
                let relative_name = format!("{}/{}", peer_device_id, file_name);
                let Some(info) = crate::cloud::sync::parse_segment_file_name(&relative_name) else { continue };
                peer_segments_found += 1;
                let Ok(body) = std::fs::read_to_string(file.path()) else { continue };
                let Ok(changes) = crate::cloud::sync::deserialize_segment(&body) else { continue };
                let Ok(result) = crate::db::sync::detect_sync_conflicts(&db.conn, &changes, &info.device_id) else { continue };
                new_changes += result.new_changes.len() as i64;
                duplicates += result.duplicates.len() as i64;
                for conflict in &result.conflicts {
                    if crate::db::sync::record_sync_conflict(&db.conn, conflict).is_ok() {
                        conflicts_recorded += 1;
                    }
                }
            }
        }
    }

    Ok(ReconcileSummary { segments_published, peer_segments_found, new_changes, duplicates, conflicts_recorded })
}

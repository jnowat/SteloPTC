use crate::auth as auth_service;
use crate::db::queries;
use crate::AppState;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use rusqlite::params;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct AttachmentMeta {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub file_name: String,
    pub file_path: String,
    pub file_size_bytes: Option<i64>,
    pub mime_type: Option<String>,
    pub description: Option<String>,
    pub uploaded_by: Option<String>,
    pub uploader_name: Option<String>,
    pub created_at: String,
}

fn attachments_dir(entity_type: &str, entity_id: &str) -> std::path::PathBuf {
    let base = crate::db::Database::db_path();
    base.parent()
        .unwrap()
        .join("attachments")
        .join(entity_type)
        .join(entity_id)
}

fn row_to_meta(row: &rusqlite::Row) -> rusqlite::Result<AttachmentMeta> {
    Ok(AttachmentMeta {
        id: row.get("id")?,
        entity_type: row.get("entity_type")?,
        entity_id: row.get("entity_id")?,
        file_name: row.get("file_name")?,
        file_path: row.get("file_path")?,
        file_size_bytes: row.get("file_size_bytes")?,
        mime_type: row.get("mime_type")?,
        description: row.get("description")?,
        uploaded_by: row.get("uploaded_by")?,
        uploader_name: row.get("uploader_name")?,
        created_at: row.get("created_at")?,
    })
}

#[tauri::command]
pub fn list_attachments(
    state: State<AppState>,
    token: String,
    entity_type: String,
    entity_id: String,
) -> Result<Vec<AttachmentMeta>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;

    let mut stmt = db
        .conn
        .prepare(
            "SELECT a.*, u.display_name as uploader_name
             FROM attachments a
             LEFT JOIN users u ON a.uploaded_by = u.id
             WHERE a.entity_type = ?1 AND a.entity_id = ?2
             ORDER BY a.created_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let items = stmt
        .query_map(params![entity_type, entity_id], row_to_meta)
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(items)
}

#[tauri::command]
pub fn upload_attachment(
    state: State<AppState>,
    token: String,
    entity_type: String,
    entity_id: String,
    file_name: String,
    mime_type: String,
    data_b64: String,
    description: Option<String>,
) -> Result<AttachmentMeta, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }

    // Decode bytes
    let bytes = B64.decode(&data_b64).map_err(|e| format!("Base64 decode error: {}", e))?;

    // Build storage path
    let dir = attachments_dir(&entity_type, &entity_id);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create directory: {}", e))?;

    let ext = std::path::Path::new(&file_name)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("bin");
    let id = uuid::Uuid::new_v4().to_string();
    let stored_name = format!("{}.{}", id, ext);
    let file_path = dir.join(&stored_name);

    std::fs::write(&file_path, &bytes).map_err(|e| format!("Failed to write file: {}", e))?;

    let file_size = bytes.len() as i64;
    let path_str = file_path.to_string_lossy().to_string();

    db.conn
        .execute(
            "INSERT INTO attachments (id, entity_type, entity_id, file_name, file_path,
             file_size_bytes, mime_type, description, uploaded_by)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                id,
                entity_type,
                entity_id,
                file_name,
                path_str,
                file_size,
                mime_type,
                description,
                user.id,
            ],
        )
        .map_err(|e| format!("Failed to record attachment: {}", e))?;

    queries::log_audit(
        &db.conn,
        Some(&user.id),
        "create",
        "attachment",
        Some(&id),
        None,
        None,
        Some(&format!("Attached {} to {}:{}", file_name, entity_type, entity_id)),
    )
    .ok();

    db.conn
        .query_row(
            "SELECT a.*, u.display_name as uploader_name
             FROM attachments a
             LEFT JOIN users u ON a.uploaded_by = u.id
             WHERE a.id = ?1",
            params![id],
            row_to_meta,
        )
        .map_err(|e| format!("Failed to fetch created attachment: {}", e))
}

#[tauri::command]
pub fn get_attachment_data(
    state: State<AppState>,
    token: String,
    id: String,
) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;

    let file_path: String = db
        .conn
        .query_row(
            "SELECT file_path FROM attachments WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )
        .map_err(|_| "Attachment not found".to_string())?;

    let bytes = std::fs::read(&file_path).map_err(|e| format!("Failed to read file: {}", e))?;
    Ok(B64.encode(&bytes))
}

#[tauri::command]
pub fn delete_attachment(
    state: State<AppState>,
    token: String,
    id: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }

    let (file_path, file_name): (String, String) = db
        .conn
        .query_row(
            "SELECT file_path, file_name FROM attachments WHERE id = ?1",
            params![id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| "Attachment not found".to_string())?;

    // Remove from DB first; if file delete fails, the record is still gone — acceptable
    db.conn
        .execute("DELETE FROM attachments WHERE id = ?1", params![id])
        .map_err(|e| format!("Failed to delete attachment: {}", e))?;

    // Best-effort file removal
    let _ = std::fs::remove_file(&file_path);

    queries::log_audit(
        &db.conn,
        Some(&user.id),
        "delete",
        "attachment",
        Some(&id),
        None,
        None,
        Some(&format!("Deleted attachment: {}", file_name)),
    )
    .ok();

    Ok(())
}

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

/// Entity kinds that may own attachments.
///
/// A closed set, not a filter over caller input. `entity_type` and `entity_id`
/// both become path components below, and `Path::join` has two escapes that are
/// easy to miss: `..` segments are kept literally and resolved later by the OS,
/// and an *absolute* component silently discards everything to its left
/// (`join("/etc/cron.d")` does not nest — it replaces). Matching against a fixed
/// list, rather than trying to strip dangerous sequences, means no encoding
/// trick has anywhere to land.
const ATTACHABLE_ENTITY_TYPES: &[&str] = &[
    "specimen",
    "subculture",
    "media_batch",
    "compliance_record",
    "strain",
    "inventory_item",
];

/// Root directory for all attachment storage: `<db dir>/attachments`.
fn attachments_root() -> Result<std::path::PathBuf, String> {
    let base = crate::db::Database::db_path();
    let parent = base.parent().ok_or_else(|| {
        "Could not determine attachments directory: database path has no parent".to_string()
    })?;
    Ok(parent.join("attachments"))
}

fn attachments_dir(entity_type: &str, entity_id: &str) -> Result<std::path::PathBuf, String> {
    if !ATTACHABLE_ENTITY_TYPES.contains(&entity_type) {
        return Err(format!("Unknown attachment target type '{}'", entity_type));
    }
    // Every entity id in this schema is a UUID. Parsing it rejects `..`, path
    // separators, drive letters and absolute paths in a single step, and does
    // so by construction rather than by blocklist.
    uuid::Uuid::parse_str(entity_id)
        .map_err(|_| format!("Attachment target id '{}' is not a valid id", entity_id))?;

    let root = attachments_root()?;
    let dir = root.join(entity_type).join(entity_id);

    // Belt and braces. The two checks above already make this unreachable, but
    // a containment assertion is cheap and survives someone later relaxing the
    // id format to something less strict than a UUID.
    if !dir.starts_with(&root) {
        return Err("Refusing to write outside the attachments directory".to_string());
    }
    Ok(dir)
}

/// Verifies that a `file_path` read back from the database still points inside
/// the attachments root before it is opened.
///
/// Rows written before the path-validation fix above could name any location on
/// disk, which would make `get_attachment_data` an arbitrary-file-read
/// primitive. Validating on read as well as on write means those rows fail
/// closed instead of being trusted because they are already stored.
fn ensure_within_attachments_root(file_path: &str) -> Result<(), String> {
    let root = attachments_root()?;
    if !std::path::Path::new(file_path).starts_with(&root) {
        return Err("Attachment is stored outside the attachments directory and \
                    will not be read. It may predate a security fix — re-upload it."
            .to_string());
    }
    Ok(())
}

/// Maximum raw attachment size (25 MiB), and the corresponding base64 length.
///
/// The payload crosses the IPC boundary as base64, so an upload holds roughly
/// three copies at once: the JS string, the Rust `String`, and the decoded
/// bytes. Checking the encoded length *before* decoding means an oversized
/// payload never allocates the second copy.
const MAX_ATTACHMENT_BYTES: usize = 25 * 1024 * 1024;
const MAX_ATTACHMENT_B64_LEN: usize = (MAX_ATTACHMENT_BYTES / 3 + 1) * 4;

/// Derives a storage extension from the user-supplied file name.
///
/// `Path::extension` cannot contain `/` or `\` (those would start a new
/// component), but it *can* contain `:` — and on NTFS `name.png:hidden` writes
/// an alternate data stream rather than the file the caller sees. Restricting
/// the result to a short ASCII-alphanumeric run sidesteps that and every other
/// filename metacharacter, at the cost of nothing: the stored name is a UUID
/// and the extension is cosmetic (`file_name` keeps the original for display).
fn safe_extension(file_name: &str) -> String {
    std::path::Path::new(file_name)
        .extension()
        .and_then(|s| s.to_str())
        .filter(|e| !e.is_empty() && e.len() <= 16 && e.chars().all(|c| c.is_ascii_alphanumeric()))
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_else(|| "bin".to_string())
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
    let db = state.db();
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

#[allow(clippy::too_many_arguments)]
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
    let db = state.db();
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }

    // Reject oversized uploads before decoding, so a bad payload never
    // allocates a second full-size copy in memory.
    if data_b64.len() > MAX_ATTACHMENT_B64_LEN {
        return Err(format!(
            "Attachment is too large. The limit is {} MiB.",
            MAX_ATTACHMENT_BYTES / 1024 / 1024
        ));
    }

    // Decode bytes
    let bytes = B64.decode(&data_b64).map_err(|e| format!("Base64 decode error: {}", e))?;

    // Build storage path
    let dir = attachments_dir(&entity_type, &entity_id)?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create directory: {}", e))?;

    let ext = safe_extension(&file_name);
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
    let db = state.db();
    let _user = auth_service::validate_session(&db, &token)?;

    let file_path: String = db
        .conn
        .query_row(
            "SELECT file_path FROM attachments WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )
        .map_err(|_| "Attachment not found".to_string())?;

    ensure_within_attachments_root(&file_path)?;

    let bytes = std::fs::read(&file_path).map_err(|e| format!("Failed to read file: {}", e))?;
    Ok(B64.encode(&bytes))
}

#[tauri::command]
pub fn delete_attachment(
    state: State<AppState>,
    token: String,
    id: String,
) -> Result<(), String> {
    let db = state.db();
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

    // Best-effort file removal — but only inside the attachments root. A row
    // written before path validation could name any file on disk, and deleting
    // the DB row must not turn into deleting an arbitrary file.
    if ensure_within_attachments_root(&file_path).is_ok() {
        let _ = std::fs::remove_file(&file_path);
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    fn uuid() -> String {
        uuid::Uuid::new_v4().to_string()
    }

    #[test]
    fn valid_entity_type_and_uuid_resolve_under_the_attachments_root() {
        let id = uuid();
        let dir = attachments_dir("specimen", &id).expect("a valid target must resolve");
        let root = attachments_root().unwrap();
        assert!(dir.starts_with(&root));
        assert!(dir.ends_with(std::path::Path::new("specimen").join(&id)));
    }

    #[test]
    fn absolute_entity_type_cannot_escape_the_root() {
        // The sharpest form of the bug: Path::join with an absolute component
        // DISCARDS everything to its left, so this would have resolved to
        // /etc/cron.d/<id> — outside the app data directory entirely.
        let id = uuid();
        for evil in ["/etc/cron.d", "/tmp", "C:\\Windows\\Temp"] {
            let err = attachments_dir(evil, &id)
                .expect_err("an absolute path must never be accepted as an entity type");
            assert!(err.contains("Unknown attachment target type"), "got: {err}");
        }
    }

    #[test]
    fn dot_dot_traversal_is_rejected_in_both_components() {
        let id = uuid();
        assert!(attachments_dir("../../..", &id).is_err());
        assert!(attachments_dir("specimen/../../..", &id).is_err());
        assert!(attachments_dir("specimen", "../../../etc/passwd").is_err());
        assert!(attachments_dir("specimen", "..").is_err());
    }

    #[test]
    fn non_uuid_entity_id_is_rejected() {
        // The id is the second path component, so anything that is not a plain
        // UUID is refused rather than sanitised.
        for bad in ["", "not-a-uuid", "a/b", "..", "s1", "%2e%2e%2f"] {
            assert!(
                attachments_dir("specimen", bad).is_err(),
                "entity id {bad:?} must be rejected"
            );
        }
    }

    #[test]
    fn unknown_entity_type_is_rejected_even_when_it_looks_harmless() {
        // Default-deny: a plausible-looking but unlisted type still fails, so
        // adding a new attachable entity is a deliberate edit to the whitelist.
        let id = uuid();
        assert!(attachments_dir("user", &id).is_err());
        assert!(attachments_dir("Specimen", &id).is_err(), "matching is case-sensitive");
    }

    #[test]
    fn stored_paths_outside_the_root_fail_closed_on_read_and_delete() {
        // Rows written before the fix could name any location on disk. Reading
        // them back must not become an arbitrary-file-read primitive.
        assert!(ensure_within_attachments_root("/etc/passwd").is_err());
        assert!(ensure_within_attachments_root("/tmp/evil.bin").is_err());

        let ok = attachments_root().unwrap().join("specimen").join(uuid()).join("x.png");
        assert!(ensure_within_attachments_root(&ok.to_string_lossy()).is_ok());
    }

    #[test]
    fn safe_extension_strips_windows_alternate_data_stream_syntax() {
        // On NTFS, "photo.png:hidden" names an alternate data stream rather than
        // the file the user thinks they uploaded. The colon must not survive.
        assert_eq!(safe_extension("photo.png:hidden"), "bin");
        assert_eq!(safe_extension("report.pdf::$DATA"), "bin");
    }

    #[test]
    fn safe_extension_keeps_ordinary_extensions_and_normalises_case() {
        assert_eq!(safe_extension("photo.PNG"), "png");
        assert_eq!(safe_extension("scan.jpeg"), "jpeg");
        assert_eq!(safe_extension("archive.tar.gz"), "gz");
    }

    #[test]
    fn safe_extension_falls_back_for_missing_or_hostile_extensions() {
        assert_eq!(safe_extension("no_extension"), "bin");
        assert_eq!(safe_extension(".bashrc"), "bin", "a dotfile has no extension");
        assert_eq!(safe_extension("x."), "bin");
        assert_eq!(safe_extension("x.a-b"), "bin", "non-alphanumeric is refused");
        assert_eq!(safe_extension(&format!("x.{}", "a".repeat(64))), "bin", "over-long is refused");
    }

    #[test]
    fn base64_length_cap_corresponds_to_the_documented_byte_limit() {
        // Guards the arithmetic: the encoded cap must actually admit a 25 MiB
        // file and reject one meaningfully larger, or the limit is cosmetic.
        let encoded_len_of_max = MAX_ATTACHMENT_BYTES.div_ceil(3) * 4;
        assert!(encoded_len_of_max <= MAX_ATTACHMENT_B64_LEN);
        let encoded_len_of_double = (MAX_ATTACHMENT_BYTES * 2).div_ceil(3) * 4;
        assert!(encoded_len_of_double > MAX_ATTACHMENT_B64_LEN);
    }
}

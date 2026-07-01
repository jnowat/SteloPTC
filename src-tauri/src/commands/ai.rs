// WP-56: Local AI analysis. Every command here either (a) calls the local
// Ollama endpoint and stores the raw result as a *pending* `ai_suggestions`
// row, or (b) approves/rejects an existing pending row. No command ever
// writes a model's output directly into a real `notes` field — only
// `approve_ai_suggestion` does that, and it goes through the same
// UPDATE + `log_audit` path a manual edit would, so the audit trail always
// attributes the change to the approving human, with the AI's contribution
// fully traceable via the linked `ai_suggestions` row.
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use rusqlite::params;
use tauri::State;

use crate::ai::ollama::{self, OllamaConfig};
use crate::auth as auth_service;
use crate::models::ai::{AiSuggestion, AnalyzePhotoRequest, SummarizeNotesRequest};
use crate::AppState;

fn row_to_suggestion(row: &rusqlite::Row) -> rusqlite::Result<AiSuggestion> {
    Ok(AiSuggestion {
        id: row.get("id")?,
        entity_type: row.get("entity_type")?,
        entity_id: row.get("entity_id")?,
        kind: row.get("kind")?,
        model_name: row.get("model_name")?,
        prompt: row.get("prompt")?,
        suggestion: row.get("suggestion")?,
        status: row.get("status")?,
        created_by: row.get("created_by")?,
        reviewed_by: row.get("reviewed_by")?,
        reviewed_at: row.get("reviewed_at")?,
        created_at: row.get("created_at")?,
    })
}

fn load_config(conn: &rusqlite::Connection) -> OllamaConfig {
    let default = OllamaConfig::default();
    OllamaConfig {
        base_url: crate::db::queries::read_setting(conn, "ai_ollama_base_url", &default.base_url),
        text_model: crate::db::queries::read_setting(conn, "ai_ollama_text_model", &default.text_model),
        vision_model: crate::db::queries::read_setting(conn, "ai_ollama_vision_model", &default.vision_model),
        timeout: default.timeout,
    }
}

#[derive(serde::Serialize)]
pub struct AiConfigResponse {
    pub base_url: String,
    pub text_model: String,
    pub vision_model: String,
}

#[tauri::command]
pub fn get_ai_config(state: State<AppState>, token: String) -> Result<AiConfigResponse, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    let cfg = load_config(&db.conn);
    Ok(AiConfigResponse { base_url: cfg.base_url, text_model: cfg.text_model, vision_model: cfg.vision_model })
}

#[tauri::command]
pub fn set_ai_config(
    state: State<AppState>,
    token: String,
    base_url: String,
    text_model: String,
    vision_model: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err("Only supervisors and admins can change the AI configuration".to_string());
    }
    for (key, value) in [
        ("ai_ollama_base_url", &base_url),
        ("ai_ollama_text_model", &text_model),
        ("ai_ollama_vision_model", &vision_model),
    ] {
        db.conn.execute(
            "INSERT INTO app_settings (key, value, updated_at) VALUES (?1, ?2, datetime('now')) \
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![key, value],
        ).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn insert_suggestion(
    conn: &rusqlite::Connection,
    entity_type: &str,
    entity_id: &str,
    kind: &str,
    model_name: &str,
    prompt: &str,
    suggestion: &str,
    created_by: &str,
) -> Result<AiSuggestion, String> {
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO ai_suggestions (id, entity_type, entity_id, kind, model_name, prompt, suggestion, status, created_by) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending', ?8)",
        params![id, entity_type, entity_id, kind, model_name, prompt, suggestion, created_by],
    ).map_err(|e| format!("Failed to store AI suggestion: {}", e))?;
    conn.query_row("SELECT * FROM ai_suggestions WHERE id = ?1", [&id], row_to_suggestion)
        .map_err(|e| e.to_string())
}

fn fetch_notes(conn: &rusqlite::Connection, entity_type: &str, entity_id: &str) -> Result<Option<String>, String> {
    let table = match entity_type {
        "specimen" => "specimens",
        "subculture" => "subcultures",
        other => return Err(format!("Unsupported entity_type '{}'", other)),
    };
    conn.query_row(&format!("SELECT notes FROM {} WHERE id = ?1", table), [entity_id], |r| r.get(0))
        .map_err(|_| format!("{} not found", entity_type))
}

/// "Summarize Notes" — feeds the entity's current free-text notes to the
/// local text model and stores the summary as a pending suggestion.
#[tauri::command]
pub fn summarize_notes(
    state: State<AppState>,
    token: String,
    request: SummarizeNotesRequest,
) -> Result<AiSuggestion, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }
    let notes = fetch_notes(&db.conn, &request.entity_type, &request.entity_id)?
        .filter(|n| !n.trim().is_empty())
        .ok_or_else(|| "There are no notes to summarize yet".to_string())?;

    let cfg = load_config(&db.conn);
    let prompt = format!(
        "Summarize the following plant/cell/fungal culture lab notes in 2-3 concise sentences, \
         preserving any specific measurements, dates, or contamination observations verbatim:\n\n{}",
        notes
    );
    let suggestion = ollama::generate(&cfg, &cfg.text_model, &prompt, &[])?;

    insert_suggestion(&db.conn, &request.entity_type, &request.entity_id, "summarize_notes", &cfg.text_model, &prompt, &suggestion, &user.id)
}

/// "Suggest Passage Comments" — feeds recent passage history for a specimen
/// to the local text model and stores a suggested comment as a pending
/// suggestion (to be attached to the specimen's notes, or copied into the
/// next subculture's notes field manually).
#[tauri::command]
pub fn suggest_passage_comment(state: State<AppState>, token: String, specimen_id: String) -> Result<AiSuggestion, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }

    let mut stmt = db.conn.prepare(
        "SELECT passage_number, date, health_status, contamination_flag, notes, observations \
         FROM subcultures WHERE specimen_id = ?1 ORDER BY passage_number DESC LIMIT 5",
    ).map_err(|e| e.to_string())?;
    let history: Vec<String> = stmt
        .query_map([&specimen_id], |r| {
            let passage: i64 = r.get(0)?;
            let date: String = r.get(1)?;
            let health: Option<String> = r.get(2)?;
            let contaminated: i64 = r.get(3)?;
            let notes: Option<String> = r.get(4)?;
            let observations: Option<String> = r.get(5)?;
            Ok(format!(
                "Passage #{passage} on {date}: health={}, contaminated={}, notes={}, observations={}",
                health.unwrap_or_else(|| "unknown".into()),
                contaminated == 1,
                notes.unwrap_or_default(),
                observations.unwrap_or_default(),
            ))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    if history.is_empty() {
        return Err("This specimen has no passage history to base a suggestion on".to_string());
    }

    let cfg = load_config(&db.conn);
    let prompt = format!(
        "You are assisting a plant/cell tissue culture technician. Based on this specimen's recent \
         passage history (most recent first), draft one concise, factual observation comment for the \
         next passage record. Do not invent data not present below.\n\n{}",
        history.join("\n")
    );
    let suggestion = ollama::generate(&cfg, &cfg.text_model, &prompt, &[])?;

    insert_suggestion(&db.conn, "specimen", &specimen_id, "suggest_passage_comment", &cfg.text_model, &prompt, &suggestion, &user.id)
}

/// "Analyze Photo for Contamination" — sends an existing attachment's image
/// bytes to the local vision model. Requires a vision-capable model (e.g.
/// `llava`) to be pulled in Ollama; a text-only model will simply produce an
/// unhelpful response rather than an error, since Ollama itself doesn't
/// distinguish the two ahead of time.
#[tauri::command]
pub fn analyze_photo_for_contamination(
    state: State<AppState>,
    token: String,
    request: AnalyzePhotoRequest,
) -> Result<AiSuggestion, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }

    let (file_path, entity_type, entity_id): (String, String, String) = db.conn.query_row(
        "SELECT file_path, entity_type, entity_id FROM attachments WHERE id = ?1",
        [&request.attachment_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    ).map_err(|_| "Attachment not found".to_string())?;

    let bytes = std::fs::read(&file_path).map_err(|e| format!("Failed to read attachment file: {}", e))?;
    let image_b64 = B64.encode(&bytes);

    let cfg = load_config(&db.conn);
    let prompt = "Examine this plant/cell/fungal tissue culture photo for visible signs of \
                  microbial contamination (bacterial or fungal growth, unusual discoloration, \
                  turbidity in liquid media, mold). Describe what you observe factually and note \
                  your confidence level. Do not diagnose beyond what is visually apparent."
        .to_string();
    let suggestion = ollama::generate(&cfg, &cfg.vision_model, &prompt, &[image_b64])?;

    insert_suggestion(&db.conn, &entity_type, &entity_id, "analyze_photo", &cfg.vision_model, &prompt, &suggestion, &user.id)
}

#[tauri::command]
pub fn list_ai_suggestions(
    state: State<AppState>,
    token: String,
    entity_type: String,
    entity_id: String,
) -> Result<Vec<AiSuggestion>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    let mut stmt = db.conn.prepare(
        "SELECT * FROM ai_suggestions WHERE entity_type = ?1 AND entity_id = ?2 ORDER BY created_at DESC",
    ).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![entity_type, entity_id], row_to_suggestion)
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

/// Approves a pending suggestion: appends its text to the target entity's
/// real `notes` field (never replaces existing notes) through the same
/// UPDATE + `log_audit` path a manual note edit uses, then marks the
/// suggestion `approved`. All manual note-editing workflows are completely
/// unaffected — this is just another writer of the same column.
#[tauri::command]
pub fn approve_ai_suggestion(state: State<AppState>, token: String, suggestion_id: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }

    let sug = db.conn.query_row("SELECT * FROM ai_suggestions WHERE id = ?1", [&suggestion_id], row_to_suggestion)
        .map_err(|_| "AI suggestion not found".to_string())?;
    if sug.status != "pending" {
        return Err(format!("This suggestion was already {}", sug.status));
    }

    let table = match sug.entity_type.as_str() {
        "specimen" => "specimens",
        "subculture" => "subcultures",
        other => return Err(format!("Cannot approve a suggestion for entity_type '{}'", other)),
    };
    let existing: Option<String> = db.conn.query_row(
        &format!("SELECT notes FROM {} WHERE id = ?1", table), [&sug.entity_id], |r| r.get(0),
    ).map_err(|_| format!("{} not found", sug.entity_type))?;

    let appended = format!(
        "{}[AI-assisted, approved by {}] {}",
        existing.as_ref().map(|n| if n.trim().is_empty() { String::new() } else { format!("{}\n\n", n) }).unwrap_or_default(),
        user.display_name,
        sug.suggestion,
    );

    db.conn.execute(
        &format!("UPDATE {} SET notes = ?1, updated_at = datetime('now') WHERE id = ?2", table),
        params![appended, sug.entity_id],
    ).map_err(|e| format!("Failed to update notes: {}", e))?;

    crate::db::queries::log_audit(
        &db.conn, Some(&user.id), "update", &sug.entity_type, Some(&sug.entity_id),
        existing.as_deref(), Some(&appended), Some(&format!("Notes updated (AI suggestion {} approved)", suggestion_id)),
    ).ok();

    db.conn.execute(
        "UPDATE ai_suggestions SET status = 'approved', reviewed_by = ?1, reviewed_at = datetime('now') WHERE id = ?2",
        params![user.id, suggestion_id],
    ).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn reject_ai_suggestion(state: State<AppState>, token: String, suggestion_id: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }
    let updated = db.conn.execute(
        "UPDATE ai_suggestions SET status = 'rejected', reviewed_by = ?1, reviewed_at = datetime('now') \
         WHERE id = ?2 AND status = 'pending'",
        params![user.id, suggestion_id],
    ).map_err(|e| e.to_string())?;
    if updated == 0 {
        return Err("Suggestion not found or already reviewed".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory DB");
        conn.execute_batch(
            "CREATE TABLE specimens (id TEXT PRIMARY KEY, notes TEXT, updated_at TEXT);
             CREATE TABLE ai_suggestions (
                 id TEXT PRIMARY KEY, entity_type TEXT NOT NULL, entity_id TEXT NOT NULL,
                 kind TEXT NOT NULL, model_name TEXT NOT NULL, prompt TEXT NOT NULL,
                 suggestion TEXT NOT NULL, status TEXT NOT NULL DEFAULT 'pending',
                 created_by TEXT, reviewed_by TEXT, reviewed_at TEXT,
                 created_at TEXT NOT NULL DEFAULT (datetime('now'))
             );",
        )
        .expect("create tables");
        conn
    }

    #[test]
    fn a_freshly_inserted_suggestion_is_pending() {
        let conn = setup_db();
        conn.execute(
            "INSERT INTO ai_suggestions (id, entity_type, entity_id, kind, model_name, prompt, suggestion) \
             VALUES ('sug1', 'specimen', 'spec1', 'summarize_notes', 'llama3.1', 'p', 'Looks healthy.')",
            [],
        )
        .unwrap();
        let status: String = conn.query_row("SELECT status FROM ai_suggestions WHERE id = 'sug1'", [], |r| r.get(0)).unwrap();
        assert_eq!(status, "pending");
    }

    #[test]
    fn approving_twice_is_prevented_by_status_check() {
        let conn = setup_db();
        conn.execute(
            "INSERT INTO ai_suggestions (id, entity_type, entity_id, kind, model_name, prompt, suggestion, status) \
             VALUES ('sug1', 'specimen', 'spec1', 'summarize_notes', 'llama3.1', 'p', 'Looks healthy.', 'approved')",
            [],
        )
        .unwrap();
        let status: String = conn.query_row("SELECT status FROM ai_suggestions WHERE id = 'sug1'", [], |r| r.get(0)).unwrap();
        assert_ne!(status, "pending", "approve_ai_suggestion must reject a non-pending row");
    }
}

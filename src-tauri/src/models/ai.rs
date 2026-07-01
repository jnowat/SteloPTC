use serde::{Deserialize, Serialize};

/// WP-56: a local-AI-generated draft, always pending human approval before
/// it can affect any real record. `model_name` and `prompt` are kept
/// verbatim for attribution — anyone reviewing a specimen's history can see
/// exactly which model produced a suggestion and what it was asked.
#[derive(Debug, Clone, Serialize)]
pub struct AiSuggestion {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub kind: String,
    pub model_name: String,
    pub prompt: String,
    pub suggestion: String,
    pub status: String,
    pub created_by: Option<String>,
    pub reviewed_by: Option<String>,
    pub reviewed_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct SummarizeNotesRequest {
    pub entity_type: String, // "specimen" | "subculture"
    pub entity_id: String,
}

#[derive(Debug, Deserialize)]
pub struct AnalyzePhotoRequest {
    pub attachment_id: String,
}

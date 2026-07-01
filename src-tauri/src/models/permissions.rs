use serde::{Deserialize, Serialize};

/// WP-55 — one role's visibility rule for one field of one entity type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldPermission {
    pub id: String,
    /// "admin" | "supervisor" | "tech" | "guest"
    pub role: String,
    /// e.g. "strain", "breeding_program"
    pub entity_type: String,
    pub field_name: String,
    pub visible: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetFieldPermissionRequest {
    pub role: String,
    pub entity_type: String,
    pub field_name: String,
    pub visible: bool,
}

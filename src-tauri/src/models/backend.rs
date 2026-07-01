use serde::{Deserialize, Serialize};

/// WP-50 — current backend configuration, as reported to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfigInfo {
    /// "sqlite" | "postgres" — the lab's intended backend (see db::backend).
    pub backend_type: String,
    /// True when this binary was compiled with `--features postgres`.
    pub postgres_feature_compiled: bool,
}

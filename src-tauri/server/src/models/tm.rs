use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmEntry {
    pub id: String,
    pub source: String,
    pub target: String,
    pub source_lang: String,
    pub target_lang: String,
    pub owner_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateTmRequest {
    pub source: String,
    pub target: String,
    pub source_lang: String,
    pub target_lang: String,
}

#[derive(Debug, Serialize)]
pub struct TmSearchResult {
    pub entry: TmEntry,
    /// Fuzzy match score 0.0–1.0
    pub score: f64,
}

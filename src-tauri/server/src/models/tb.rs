use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TbEntry {
    pub id: String,
    pub source_term: String,
    pub target_term: String,
    pub source_lang: String,
    pub target_lang: String,
    pub notes: String,
    pub forbidden: bool,
    pub owner_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateTbRequest {
    pub source_term: String,
    pub target_term: String,
    pub source_lang: String,
    pub target_lang: String,
    pub notes: Option<String>,
    pub forbidden: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTbRequest {
    pub source_term: Option<String>,
    pub target_term: Option<String>,
    pub notes: Option<String>,
    pub forbidden: Option<bool>,
}

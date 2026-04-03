use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub source_lang: String,
    pub target_lang: String,
    pub owner_id: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFile {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub file_path: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub source_lang: String,
    pub target_lang: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProjectRequest {
    pub name: Option<String>,
    pub source_lang: Option<String>,
    pub target_lang: Option<String>,
}

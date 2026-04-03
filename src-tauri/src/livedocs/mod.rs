pub mod index;
pub mod search;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LiveDocsError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Unsupported file format: {0}")]
    UnsupportedFormat(String),
    #[error("Library not found: {0}")]
    LibraryNotFound(String),
    #[error("Parse error: {0}")]
    Parse(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveDocsDocument {
    pub id: String,
    pub path: String,
    pub sentences: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveDocsLibrary {
    pub id: String,
    pub name: String,
    pub documents: Vec<LiveDocsDocument>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveDocsMatch {
    pub sentence: String,
    pub doc_path: String,
    pub score: f32,
}

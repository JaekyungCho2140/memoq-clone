pub mod deepl;
pub mod engine;
pub mod google;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MtError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("API error ({code}): {message}")]
    Api { code: u16, message: String },
    #[error("Keychain error: {0}")]
    Keychain(String),
    #[error("Invalid or missing API key")]
    InvalidApiKey,
    #[error("Rate limit exceeded")]
    RateLimit,
    #[error("Unsupported language pair: {0} -> {1}")]
    UnsupportedLanguage(String, String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MtProviderInfo {
    pub id: String,
    pub name: String,
    pub requires_api_key: bool,
}

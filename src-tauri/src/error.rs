/// Structured application error type used by all Tauri commands.
///
/// Tauri serialises `Err(AppError)` to JSON as `{ "kind": "...", "message": "..." }`
/// so the frontend can distinguish error categories and show appropriate UI.
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "message")]
pub enum AppError {
    /// The supplied argument failed a pre-condition check (e.g. empty query).
    #[error("validation error: {0}")]
    Validation(String),

    /// The file could not be read or does not exist.
    #[error("file error: {0}")]
    File(String),

    /// The file exceeds the maximum allowed size.
    #[error("file too large: {0}")]
    FileTooLarge(String),

    /// A database operation failed.
    #[error("storage error: {0}")]
    Storage(String),

    /// An external service (MT provider, etc.) returned an error.
    #[error("service error: {0}")]
    Service(String),

    /// A catch-all for unexpected internal errors.
    #[error("internal error: {0}")]
    Internal(String),
}

impl AppError {
    /// Convert any `anyhow::Error` to `AppError::Internal`.
    pub fn internal(e: impl std::fmt::Display) -> Self {
        Self::Internal(e.to_string())
    }

    /// Convert any `anyhow::Error` to `AppError::Storage`.
    pub fn storage(e: impl std::fmt::Display) -> Self {
        Self::Storage(e.to_string())
    }

    /// Convert any `anyhow::Error` to `AppError::File`.
    pub fn file(e: impl std::fmt::Display) -> Self {
        Self::File(e.to_string())
    }
}

/// The maximum file size accepted by the parser (50 MB).
pub const MAX_FILE_BYTES: u64 = 50 * 1024 * 1024;

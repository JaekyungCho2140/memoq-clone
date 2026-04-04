use crate::error::{AppError, MAX_FILE_BYTES};
use crate::models::{Project, Segment};
use crate::parser::{export, parse};
use tauri::command;

/// Parse an XLIFF (1.2 / 2.0) or DOCX file and return a Project with all
/// segments.  Called by the frontend when the user opens a translation file.
///
/// Returns `AppError::Validation` for an empty path, `AppError::FileTooLarge`
/// when the file exceeds 50 MB, and `AppError::File` for any I/O or parse error.
#[command]
pub async fn parse_file(path: String) -> Result<Project, AppError> {
    if path.trim().is_empty() {
        return Err(AppError::Validation("file path must not be empty".into()));
    }

    let metadata = std::fs::metadata(&path).map_err(AppError::file)?;
    if metadata.len() > MAX_FILE_BYTES {
        return Err(AppError::FileTooLarge(format!(
            "file size {} bytes exceeds the {} byte limit",
            metadata.len(),
            MAX_FILE_BYTES
        )));
    }

    parse(&path).map_err(AppError::file)
}

/// Export translated segments back into the original file format.
/// `input_path` is the original source file (used as a template for structure).
/// `output_path` is where the translated file will be saved.
/// The original file at `input_path` is never modified.
#[command]
pub async fn export_xliff(
    segments: Vec<Segment>,
    input_path: String,
    output_path: String,
) -> Result<(), AppError> {
    if input_path.trim().is_empty() {
        return Err(AppError::Validation("input path must not be empty".into()));
    }
    if output_path.trim().is_empty() {
        return Err(AppError::Validation("output path must not be empty".into()));
    }

    export(&segments, &input_path, &output_path).map_err(AppError::file)
}

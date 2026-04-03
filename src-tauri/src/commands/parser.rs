use crate::models::{Project, Segment};
use crate::parser::{export, parse};
use tauri::command;

/// Parse an XLIFF (1.2 / 2.0) or DOCX file and return a Project with all
/// segments.  Called by the frontend when the user opens a translation file.
#[command]
pub async fn parse_file(path: String) -> Result<Project, String> {
    parse(&path).map_err(|e| e.to_string())
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
) -> Result<(), String> {
    export(&segments, &input_path, &output_path).map_err(|e| e.to_string())
}

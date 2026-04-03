use crate::models::Segment;
use crate::parser;
use tauri::command;

/// Export translated segments back to the source format.
/// `source_path` is the original file (used as a template).
/// `output_path` is where the translated file will be saved.
#[command]
pub async fn export_file(
    segments: Vec<Segment>,
    source_path: String,
    output_path: String,
) -> Result<(), String> {
    parser::export(&segments, &source_path, &output_path).map_err(|e| e.to_string())
}

/// Persist a single segment update. Returns the updated Segment.
/// For MVP, segments are kept in the frontend store; this command is a
/// lightweight persistence hook (could write to a project JSON in the future).
#[command]
pub async fn save_segment(
    project_id: String,
    segment_id: String,
    source: String,
    target: String,
    status: String,
    order: u32,
) -> Result<Segment, String> {
    use crate::models::SegmentStatus;
    log::debug!("save_segment: project={project_id} seg={segment_id}");
    let seg_status: SegmentStatus = status.parse().map_err(|e: anyhow::Error| e.to_string())?;
    Ok(Segment {
        id: segment_id,
        source,
        target,
        status: seg_status,
        order,
    })
}

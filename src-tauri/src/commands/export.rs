use tauri::command;
use crate::models::Segment;

#[command]
pub async fn export_file(project_id: String, output_path: String, format: String) -> Result<(), String> {
    log::info!("Exporting project {project_id} to {output_path} as {format}");
    // TODO: implement per-format export
    Ok(())
}

#[command]
pub async fn save_segment(project_id: String, segment_id: String, target: String, status: String) -> Result<Segment, String> {
    use crate::models::SegmentStatus;
    log::debug!("Saving segment {segment_id} in project {project_id}");
    let seg_status: SegmentStatus = status.parse().map_err(|e: anyhow::Error| e.to_string())?;
    // TODO: persist to project JSON file
    Ok(Segment { id: segment_id, source: String::new(), target, status: seg_status, order: 0 })
}

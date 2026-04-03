use crate::models::Project;
use crate::parser::parse;
use tauri::command;

#[command]
pub async fn parse_file(path: String) -> Result<Project, String> {
    parse(&path).map_err(|e| e.to_string())
}

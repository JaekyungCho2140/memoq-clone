use crate::livedocs::{index, search as livedocs_search, LiveDocsLibrary, LiveDocsMatch};
use tauri::command;

#[command]
pub async fn livedocs_create_library(name: String) -> Result<LiveDocsLibrary, String> {
    index::create_library(&name).map_err(|e| e.to_string())
}

#[command]
pub async fn livedocs_add_document(
    lib_id: String,
    path: String,
) -> Result<LiveDocsLibrary, String> {
    index::add_document(&lib_id, &path).map_err(|e| e.to_string())
}

#[command]
pub async fn livedocs_list_libraries() -> Result<Vec<LiveDocsLibrary>, String> {
    index::list_libraries().map_err(|e| e.to_string())
}

#[command]
pub async fn livedocs_search(
    query: String,
    lib_id: String,
    min_score: Option<f32>,
) -> Result<Vec<LiveDocsMatch>, String> {
    livedocs_search::search(&query, &lib_id, min_score).map_err(|e| e.to_string())
}

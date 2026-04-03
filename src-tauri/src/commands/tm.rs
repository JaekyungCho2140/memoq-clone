use crate::models::{TmEntry, TmMatch};
use crate::tm::{TmEngine, TmSearchParams};
use tauri::command;

#[command]
pub async fn tm_create(name: String, source_lang: String, target_lang: String) -> Result<String, String> {
    TmEngine::create(&name, &source_lang, &target_lang).map_err(|e| e.to_string())
}

#[command]
pub async fn tm_add(tm_id: String, source: String, target: String, source_lang: String, target_lang: String) -> Result<TmEntry, String> {
    let engine = TmEngine::open(&tm_id).map_err(|e| e.to_string())?;
    engine.add(&source, &target, &source_lang, &target_lang).map_err(|e| e.to_string())
}

#[command]
pub async fn tm_search(tm_id: String, query: String, source_lang: String, target_lang: String, min_score: f32) -> Result<Vec<TmMatch>, String> {
    let engine = TmEngine::open(&tm_id).map_err(|e| e.to_string())?;
    engine.search(TmSearchParams { query: &query, source_lang: &source_lang, target_lang: &target_lang, min_score }).map_err(|e| e.to_string())
}

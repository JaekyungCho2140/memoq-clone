use crate::models::TbEntry;
use crate::tb::TbEngine;
use tauri::command;

#[command]
pub async fn tb_create(name: String) -> Result<String, String> {
    TbEngine::create(&name).map_err(|e| e.to_string())
}

#[command]
pub async fn tb_add(tb_id: String, source_term: String, target_term: String, source_lang: String, target_lang: String, notes: String, forbidden: bool) -> Result<TbEntry, String> {
    let engine = TbEngine::open(&tb_id).map_err(|e| e.to_string())?;
    engine.add(&source_term, &target_term, &source_lang, &target_lang, &notes, forbidden).map_err(|e| e.to_string())
}

#[command]
pub async fn tb_lookup(tb_id: String, term: String, source_lang: String) -> Result<Vec<TbEntry>, String> {
    let engine = TbEngine::open(&tb_id).map_err(|e| e.to_string())?;
    engine.lookup(&term, &source_lang).map_err(|e| e.to_string())
}

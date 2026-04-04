use crate::error::AppError;
use crate::models::TbEntry;
use crate::tb::TbEngine;
use tauri::command;

#[command]
pub async fn tb_create(name: String) -> Result<String, AppError> {
    if name.trim().is_empty() {
        return Err(AppError::Validation("TB name must not be empty".into()));
    }
    TbEngine::create(&name).map_err(AppError::storage)
}

#[command]
pub async fn tb_add(
    tb_id: String,
    source_term: String,
    target_term: String,
    source_lang: String,
    target_lang: String,
    notes: String,
    forbidden: bool,
) -> Result<TbEntry, AppError> {
    if tb_id.trim().is_empty() {
        return Err(AppError::Validation("TB ID must not be empty".into()));
    }
    if source_term.trim().is_empty() {
        return Err(AppError::Validation("source term must not be empty".into()));
    }
    let engine = TbEngine::open(&tb_id).map_err(AppError::storage)?;
    engine
        .add(
            &source_term,
            &target_term,
            &source_lang,
            &target_lang,
            &notes,
            forbidden,
        )
        .map_err(AppError::storage)
}

#[command]
pub async fn tb_lookup(
    tb_id: String,
    term: String,
    source_lang: String,
) -> Result<Vec<TbEntry>, AppError> {
    if tb_id.trim().is_empty() {
        return Err(AppError::Validation("TB ID must not be empty".into()));
    }
    // Empty term lookup returns no entries — consistent with editor UX.
    if term.trim().is_empty() {
        return Ok(vec![]);
    }
    let engine = TbEngine::open(&tb_id).map_err(AppError::storage)?;
    engine
        .lookup(&term, &source_lang)
        .map_err(AppError::storage)
}

use crate::error::AppError;
use crate::models::{TmEntry, TmMatch};
use crate::tm::{TmEngine, TmSearchParams};
use tauri::command;

#[command]
pub async fn tm_create(
    name: String,
    source_lang: String,
    target_lang: String,
) -> Result<String, AppError> {
    if name.trim().is_empty() {
        return Err(AppError::Validation("TM name must not be empty".into()));
    }
    if source_lang.trim().is_empty() || target_lang.trim().is_empty() {
        return Err(AppError::Validation(
            "source and target language codes must not be empty".into(),
        ));
    }
    TmEngine::create(&name, &source_lang, &target_lang).map_err(AppError::storage)
}

#[command]
pub async fn tm_add(
    tm_id: String,
    source: String,
    target: String,
    source_lang: String,
    target_lang: String,
) -> Result<TmEntry, AppError> {
    if tm_id.trim().is_empty() {
        return Err(AppError::Validation("TM ID must not be empty".into()));
    }
    if source.trim().is_empty() {
        return Err(AppError::Validation(
            "source segment must not be empty".into(),
        ));
    }
    let engine = TmEngine::open(&tm_id).map_err(AppError::storage)?;
    engine
        .add(&source, &target, &source_lang, &target_lang)
        .map_err(AppError::storage)
}

#[command]
pub async fn tm_search(
    tm_id: String,
    query: String,
    source_lang: String,
    target_lang: String,
    min_score: f32,
) -> Result<Vec<TmMatch>, AppError> {
    if tm_id.trim().is_empty() {
        return Err(AppError::Validation("TM ID must not be empty".into()));
    }
    // Empty query returns no matches rather than an error — consistent with
    // how translation editors treat an empty source segment.
    if query.trim().is_empty() {
        return Ok(vec![]);
    }
    if !(0.0..=1.0).contains(&min_score) {
        return Err(AppError::Validation(format!(
            "min_score must be between 0.0 and 1.0, got {min_score}"
        )));
    }
    let engine = TmEngine::open(&tm_id).map_err(AppError::storage)?;
    engine
        .search(TmSearchParams {
            query: &query,
            source_lang: &source_lang,
            target_lang: &target_lang,
            min_score,
        })
        .map_err(AppError::storage)
}

use crate::mt::{engine, MtProviderInfo};
use tauri::command;

#[command]
pub async fn mt_translate(
    text: String,
    source_lang: String,
    target_lang: String,
    provider: String,
) -> Result<String, String> {
    engine::translate(&text, &source_lang, &target_lang, &provider)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn mt_save_api_key(provider: String, api_key: String) -> Result<(), String> {
    engine::save_api_key(&provider, &api_key).map_err(|e| e.to_string())
}

#[command]
pub async fn mt_get_providers() -> Result<Vec<MtProviderInfo>, String> {
    Ok(engine::get_providers())
}

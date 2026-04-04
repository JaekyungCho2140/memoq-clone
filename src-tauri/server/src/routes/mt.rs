use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{
    app::AppState,
    auth::middleware::AuthUser,
    error::{AppError, AppResult},
};

// ── 타입 정의 ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MtTranslateRequest {
    pub source: String,
    pub source_lang: String,
    pub target_lang: String,
    pub provider: String,
    pub api_key: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MtResult {
    pub source: String,
    pub target: String,
    pub provider: String,
}

// ── DeepL 응답 역직렬화 ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct DeepLTranslation {
    text: String,
}

#[derive(Debug, Deserialize)]
struct DeepLResponse {
    translations: Vec<DeepLTranslation>,
}

// ── Google Translate 응답 역직렬화 ────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GoogleData {
    translations: Vec<GoogleTranslation>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GoogleTranslation {
    #[allow(dead_code)]
    translated_text: String,
}

// ── 핸들러 ─────────────────────────────────────────────────────────────────────

/// POST /api/mt/translate
pub async fn translate(
    State(_state): State<AppState>,
    AuthUser(_claims): AuthUser,
    Json(body): Json<MtTranslateRequest>,
) -> AppResult<Json<MtResult>> {
    if body.source.trim().is_empty() {
        return Err(AppError::BadRequest("source text is required".to_string()));
    }

    let target_text = match body.provider.as_str() {
        "deepl" => translate_deepl(&body).await?,
        "google" => translate_google(&body).await?,
        other => {
            return Err(AppError::BadRequest(format!(
                "Unsupported MT provider: {}",
                other
            )))
        }
    };

    Ok(Json(MtResult {
        source: body.source.clone(),
        target: target_text,
        provider: body.provider.clone(),
    }))
}

// ── DeepL 번역 ────────────────────────────────────────────────────────────────

async fn translate_deepl(req: &MtTranslateRequest) -> AppResult<String> {
    let client = reqwest::Client::new();

    // DeepL API v2 - Free tier uses api-free.deepl.com, paid uses api.deepl.com
    let url = if req.api_key.ends_with(":fx") {
        "https://api-free.deepl.com/v2/translate"
    } else {
        "https://api.deepl.com/v2/translate"
    };

    let target_lang = normalize_deepl_lang(&req.target_lang);
    let source_lang = normalize_deepl_lang(&req.source_lang);

    let resp = client
        .post(url)
        .header("Authorization", format!("DeepL-Auth-Key {}", req.api_key))
        .form(&[
            ("text", req.source.as_str()),
            ("source_lang", source_lang),
            ("target_lang", target_lang),
        ])
        .send()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DeepL request failed: {}", e)))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::Internal(anyhow::anyhow!(
            "DeepL API error {}: {}",
            status,
            body
        )));
    }

    let data: DeepLResponse = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DeepL parse error: {}", e)))?;

    data.translations
        .into_iter()
        .next()
        .map(|t| t.text)
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("DeepL returned no translations")))
}

/// DeepL 언어 코드 정규화 (예: "ko-KR" → "KO", "en" → "EN-US")
fn normalize_deepl_lang(lang: &str) -> &str {
    match lang.to_ascii_lowercase().as_str() {
        "ko" | "ko-kr" => "KO",
        "en" | "en-us" => "EN-US",
        "en-gb" => "EN-GB",
        "ja" | "ja-jp" => "JA",
        "zh" | "zh-cn" => "ZH",
        "de" | "de-de" => "DE",
        "fr" | "fr-fr" => "FR",
        "es" | "es-es" => "ES",
        _ => lang,
    }
}

// ── Google Translate ──────────────────────────────────────────────────────────

async fn translate_google(req: &MtTranslateRequest) -> AppResult<String> {
    let client = reqwest::Client::new();

    let url = format!(
        "https://translation.googleapis.com/language/translate/v2?key={}",
        req.api_key
    );

    let source_lang = req
        .source_lang
        .split('-')
        .next()
        .unwrap_or(&req.source_lang);
    let target_lang = req
        .target_lang
        .split('-')
        .next()
        .unwrap_or(&req.target_lang);

    let payload = serde_json::json!({
        "q": req.source,
        "source": source_lang,
        "target": target_lang,
        "format": "text"
    });

    let resp = client.post(&url).json(&payload).send().await.map_err(|e| {
        AppError::Internal(anyhow::anyhow!("Google Translate request failed: {}", e))
    })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::Internal(anyhow::anyhow!(
            "Google Translate API error {}: {}",
            status,
            body
        )));
    }

    let raw: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Google Translate parse error: {}", e)))?;

    raw["data"]["translations"][0]["translatedText"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Google Translate returned no text")))
}

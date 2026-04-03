//! Alignment API routes.
//!
//! POST /api/alignment/align   — upload source + target files, return AlignmentResult
//! POST /api/alignment/confirm — save selected pairs to TM

use axum::{
    body::Bytes,
    extract::{Multipart, State},
    Json,
};
use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    alignment::{align, parser::extract_sentences, AlignmentResult},
    app::AppState,
    auth::middleware::AuthUser,
    db::run_db,
    error::{AppError, AppResult},
};

// ──────────────────────────────────────────────
// Request / response types
// ──────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct AlignResponse {
    pub source_filename: String,
    pub target_filename: String,
    pub source_lang: String,
    pub target_lang: String,
    pub result: AlignmentResult,
}

#[derive(Debug, Deserialize)]
pub struct ConfirmRequest {
    pub source_lang: String,
    pub target_lang: String,
    pub pairs: Vec<ConfirmPair>,
}

#[derive(Debug, Deserialize)]
pub struct ConfirmPair {
    pub source: String,
    pub target: String,
}

#[derive(Debug, Serialize)]
pub struct ConfirmResponse {
    pub saved: usize,
}

// ──────────────────────────────────────────────
// Handlers
// ──────────────────────────────────────────────

/// POST /api/alignment/align
///
/// Accepts a `multipart/form-data` body with the following fields:
/// - `source_file`  — source document (.txt | .xliff | .docx)
/// - `target_file`  — target document (.txt | .xliff | .docx)
/// - `source_lang`  — BCP-47 language code (e.g. "en")
/// - `target_lang`  — BCP-47 language code (e.g. "ko")
pub async fn align_documents(
    State(_state): State<AppState>,
    AuthUser(_claims): AuthUser,
    mut multipart: Multipart,
) -> AppResult<Json<AlignResponse>> {
    let mut source_bytes: Option<(String, Bytes)> = None;
    let mut target_bytes: Option<(String, Bytes)> = None;
    let mut source_lang = String::from("en");
    let mut target_lang = String::from("ko");

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "source_file" => {
                let fname = field
                    .file_name()
                    .unwrap_or("source.txt")
                    .to_string();
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|e| AppError::BadRequest(e.to_string()))?;
                source_bytes = Some((fname, bytes));
            }
            "target_file" => {
                let fname = field
                    .file_name()
                    .unwrap_or("target.txt")
                    .to_string();
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|e| AppError::BadRequest(e.to_string()))?;
                target_bytes = Some((fname, bytes));
            }
            "source_lang" => {
                source_lang = field
                    .text()
                    .await
                    .map_err(|e| AppError::BadRequest(e.to_string()))?;
            }
            "target_lang" => {
                target_lang = field
                    .text()
                    .await
                    .map_err(|e| AppError::BadRequest(e.to_string()))?;
            }
            _ => {
                // consume unknown fields
                let _ = field
                    .bytes()
                    .await
                    .map_err(|e| AppError::BadRequest(e.to_string()))?;
            }
        }
    }

    let (src_name, src_bytes) =
        source_bytes.ok_or_else(|| AppError::BadRequest("Missing 'source_file' field".into()))?;
    let (tgt_name, tgt_bytes) =
        target_bytes.ok_or_else(|| AppError::BadRequest("Missing 'target_file' field".into()))?;

    let sources =
        extract_sentences(&src_bytes, &src_name).map_err(|e| AppError::BadRequest(e.to_string()))?;
    let targets =
        extract_sentences(&tgt_bytes, &tgt_name).map_err(|e| AppError::BadRequest(e.to_string()))?;

    // Run alignment (CPU-bound but fast enough for sync handling)
    let result = align(&sources, &targets);

    Ok(Json(AlignResponse {
        source_filename: src_name,
        target_filename: tgt_name,
        source_lang,
        target_lang,
        result,
    }))
}

/// POST /api/alignment/confirm
///
/// Save selected aligned pairs to the TM.
pub async fn confirm_alignment(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<ConfirmRequest>,
) -> AppResult<Json<ConfirmResponse>> {
    if req.pairs.is_empty() {
        return Ok(Json(ConfirmResponse { saved: 0 }));
    }

    let owner_id = claims.sub.clone();
    let source_lang = req.source_lang.clone();
    let target_lang = req.target_lang.clone();
    let pairs = req.pairs;
    let pool = state.pool.clone();

    let saved = run_db(pool, move |conn| {
        let mut count = 0usize;
        for pair in &pairs {
            let id = Uuid::new_v4().to_string();
            let now = Utc::now().to_rfc3339();
            conn.execute(
                "INSERT INTO tm_entries (id, source, target, source_lang, target_lang, owner_id, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    id,
                    pair.source,
                    pair.target,
                    source_lang,
                    target_lang,
                    owner_id,
                    now,
                ],
            )
            .map_err(|e| crate::error::AppError::Internal(anyhow::anyhow!(e)))?;
            count += 1;
        }
        Ok(count)
    })
    .await?;

    Ok(Json(ConfirmResponse { saved }))
}

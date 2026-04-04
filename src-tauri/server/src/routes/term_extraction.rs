//! Term extraction API routes.
//!
//! POST /api/term-extraction/extract    — upload doc, return term candidates
//! POST /api/term-extraction/add-to-tb  — add approved terms to TB

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
    alignment::parser::extract_sentences,
    app::AppState,
    auth::middleware::AuthUser,
    db::run_db,
    error::{AppError, AppResult},
    term_extraction::extract_terms,
};

const DEFAULT_MAX_CANDIDATES: usize = 50;

// ─── Types ───────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct TermCandidateDto {
    pub term: String,
    pub score: f64,
    pub frequency: u32,
}

#[derive(Debug, Serialize)]
pub struct ExtractResponse {
    pub source_lang: String,
    pub candidates: Vec<TermCandidateDto>,
}

#[derive(Debug, Deserialize)]
pub struct AddToTbRequest {
    pub source_lang: String,
    pub target_lang: String,
    pub terms: Vec<TbTermPair>,
}

#[derive(Debug, Deserialize)]
pub struct TbTermPair {
    pub source_term: String,
    pub target_term: String,
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Serialize)]
pub struct AddToTbResponse {
    pub saved: usize,
}

// ─── Handlers ────────────────────────────────────────────────────────────────

/// POST /api/term-extraction/extract
///
/// Accepts `multipart/form-data`:
/// - `source_file`  — document (.txt | .xliff | .docx)
/// - `source_lang`  — BCP-47 language code
/// - `max_candidates` (optional, default 50)
pub async fn extract_terms_handler(
    State(_state): State<AppState>,
    AuthUser(_claims): AuthUser,
    mut multipart: Multipart,
) -> AppResult<Json<ExtractResponse>> {
    let mut file_bytes: Option<(String, Bytes)> = None;
    let mut source_lang = "en".to_string();
    let mut max_candidates = DEFAULT_MAX_CANDIDATES;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "source_file" => {
                let fname = field.file_name().unwrap_or("source.txt").to_string();
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|e| AppError::BadRequest(e.to_string()))?;
                file_bytes = Some((fname, bytes));
            }
            "source_lang" => {
                source_lang = field
                    .text()
                    .await
                    .map_err(|e| AppError::BadRequest(e.to_string()))?;
            }
            "max_candidates" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| AppError::BadRequest(e.to_string()))?;
                max_candidates = text.parse().unwrap_or(DEFAULT_MAX_CANDIDATES);
            }
            _ => {
                let _ = field
                    .bytes()
                    .await
                    .map_err(|e| AppError::BadRequest(e.to_string()))?;
            }
        }
    }

    let (fname, bytes) =
        file_bytes.ok_or_else(|| AppError::BadRequest("Missing 'source_file' field".into()))?;

    // Reuse the alignment parser to extract plain text sentences
    let sentences =
        extract_sentences(&bytes, &fname).map_err(|e| AppError::BadRequest(e.to_string()))?;
    let full_text = sentences.join(" ");

    let candidates: Vec<TermCandidateDto> = extract_terms(&full_text, max_candidates)
        .into_iter()
        .map(|c| TermCandidateDto {
            term: c.term,
            score: c.score,
            frequency: c.frequency,
        })
        .collect();

    Ok(Json(ExtractResponse {
        source_lang,
        candidates,
    }))
}

/// POST /api/term-extraction/add-to-tb
///
/// Saves a list of approved term pairs to the TB.
pub async fn add_terms_to_tb(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<AddToTbRequest>,
) -> AppResult<Json<AddToTbResponse>> {
    if req.terms.is_empty() {
        return Ok(Json(AddToTbResponse { saved: 0 }));
    }

    let owner_id = claims.sub.clone();
    let source_lang = req.source_lang.clone();
    let target_lang = req.target_lang.clone();
    let terms = req.terms;
    let pool = state.pool.clone();

    let saved = run_db(pool, move |conn| {
        let mut count = 0usize;
        for pair in &terms {
            if pair.source_term.trim().is_empty() {
                continue;
            }
            let id = Uuid::new_v4().to_string();
            let now = Utc::now().to_rfc3339();
            conn.execute(
                "INSERT INTO tb_entries
                 (id, source_term, target_term, source_lang, target_lang, notes, forbidden, owner_id, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7, ?8)",
                params![
                    id,
                    pair.source_term.trim(),
                    pair.target_term.trim(),
                    source_lang,
                    target_lang,
                    pair.notes,
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

    Ok(Json(AddToTbResponse { saved }))
}

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::Utc;
use rusqlite::{params, OptionalExtension};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    app::AppState,
    auth::middleware::AuthUser,
    db::run_db,
    error::{AppError, AppResult},
    models::tm::{CreateTmRequest, TmEntry, TmSearchResult},
};

#[derive(Debug, Deserialize)]
pub struct TmSearchParams {
    pub source: Option<String>,
    pub source_lang: Option<String>,
    pub target_lang: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    5
}

/// GET /api/tm  — list all, or fuzzy search when source+source_lang+target_lang are provided
pub async fn list_or_search_tm(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Query(params): Query<TmSearchParams>,
) -> AppResult<Json<serde_json::Value>> {
    let owner_id = claims.sub.clone();
    let pool = state.pool.clone();

    match (params.source, params.source_lang, params.target_lang) {
        (Some(source), Some(source_lang), Some(target_lang)) => {
            // Search mode
            let limit = params.limit;
            let results = run_db(pool, move |conn| {
                let mut stmt = conn
                    .prepare(
                        "SELECT id, source, target, source_lang, target_lang, owner_id, created_at
                         FROM tm_entries
                         WHERE source_lang = ?1 AND target_lang = ?2
                         AND (owner_id = ?3 OR owner_id IS NULL)",
                    )
                    .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

                let entries: Vec<TmEntry> = stmt
                    .query_map(params![&source_lang, &target_lang, &owner_id], |row| {
                        Ok(TmEntry {
                            id: row.get(0)?,
                            source: row.get(1)?,
                            target: row.get(2)?,
                            source_lang: row.get(3)?,
                            target_lang: row.get(4)?,
                            owner_id: row.get(5)?,
                            created_at: row.get(6)?,
                        })
                    })
                    .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

                let mut scored: Vec<TmSearchResult> = entries
                    .into_iter()
                    .filter_map(|entry| {
                        let score = fuzzy_score(&source, &entry.source);
                        if score >= 0.5 {
                            Some(TmSearchResult { entry, score })
                        } else {
                            None
                        }
                    })
                    .collect();

                scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
                scored.truncate(limit);
                Ok(scored)
            })
            .await?;
            Ok(Json(serde_json::to_value(results).unwrap()))
        }
        _ => {
            // List mode
            let entries = run_db(pool, move |conn| {
                let mut stmt = conn
                    .prepare(
                        "SELECT id, source, target, source_lang, target_lang, owner_id, created_at
                         FROM tm_entries WHERE owner_id = ?1 OR owner_id IS NULL
                         ORDER BY created_at DESC",
                    )
                    .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

                let rows = stmt
                    .query_map(params![&owner_id], |row| {
                        Ok(TmEntry {
                            id: row.get(0)?,
                            source: row.get(1)?,
                            target: row.get(2)?,
                            source_lang: row.get(3)?,
                            target_lang: row.get(4)?,
                            owner_id: row.get(5)?,
                            created_at: row.get(6)?,
                        })
                    })
                    .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

                Ok(rows)
            })
            .await?;
            Ok(Json(serde_json::to_value(entries).unwrap()))
        }
    }
}

/// POST /api/tm
pub async fn create_tm(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(body): Json<CreateTmRequest>,
) -> AppResult<Json<TmEntry>> {
    if body.source.trim().is_empty() || body.target.trim().is_empty() {
        return Err(AppError::BadRequest(
            "source and target are required".to_string(),
        ));
    }

    let now = Utc::now().to_rfc3339();
    let entry = TmEntry {
        id: Uuid::new_v4().to_string(),
        source: body.source,
        target: body.target,
        source_lang: body.source_lang,
        target_lang: body.target_lang,
        owner_id: Some(claims.sub.clone()),
        created_at: now,
    };
    let e = entry.clone();
    let pool = state.pool.clone();

    run_db(pool, move |conn| {
        conn.execute(
            "INSERT INTO tm_entries (id, source, target, source_lang, target_lang, owner_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![&e.id, &e.source, &e.target, &e.source_lang, &e.target_lang, &e.owner_id, &e.created_at],
        )
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
        Ok(())
    })
    .await?;

    Ok(Json(entry))
}

/// DELETE /api/tm/:id
pub async fn delete_tm(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let owner_id = claims.sub.clone();
    let pool = state.pool.clone();

    run_db(pool, move |conn| {
        let existing: Option<(String, Option<String>)> = conn
            .query_row(
                "SELECT id, owner_id FROM tm_entries WHERE id = ?1",
                params![&id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

        match existing {
            None => return Err(AppError::NotFound("TM entry not found".to_string())),
            Some((_, Some(oid))) if oid != owner_id => return Err(AppError::Forbidden),
            _ => {}
        }

        conn.execute("DELETE FROM tm_entries WHERE id = ?1", params![&id])
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
        Ok(())
    })
    .await?;

    Ok(Json(serde_json::json!({ "deleted": 1 })))
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Normalized edit distance score (0.0–1.0).
fn fuzzy_score(query: &str, candidate: &str) -> f64 {
    if query == candidate {
        return 1.0;
    }
    if query.is_empty() || candidate.is_empty() {
        return 0.0;
    }
    let q: Vec<char> = query.chars().collect();
    let c: Vec<char> = candidate.chars().collect();
    let dist = edit_distance(&q, &c);
    let max_len = q.len().max(c.len()) as f64;
    1.0 - (dist as f64 / max_len)
}

fn edit_distance(a: &[char], b: &[char]) -> usize {
    let (m, n) = (a.len(), b.len());
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in 0..=m { dp[i][0] = i; }
    for j in 0..=n { dp[0][j] = j; }
    for i in 1..=m {
        for j in 1..=n {
            dp[i][j] = if a[i - 1] == b[j - 1] {
                dp[i - 1][j - 1]
            } else {
                1 + dp[i - 1][j].min(dp[i][j - 1]).min(dp[i - 1][j - 1])
            };
        }
    }
    dp[m][n]
}

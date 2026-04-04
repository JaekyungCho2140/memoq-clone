use axum::{
    extract::{Path, Query, State},
    http::header,
    response::{IntoResponse, Response},
    Json,
};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::{
    app::AppState,
    auth::middleware::AuthUser,
    db::run_db,
    error::{AppError, AppResult},
};

// ─── Query Params ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AnalyticsQuery {
    /// ISO-8601 date or datetime, e.g. "2024-01-01"
    pub from: Option<String>,
    pub to: Option<String>,
    /// "csv" to return CSV instead of JSON
    pub format: Option<String>,
}

impl AnalyticsQuery {
    #[allow(clippy::wrong_self_convention)]
    fn from_clause(&self) -> String {
        self.from
            .clone()
            .unwrap_or_else(|| "1970-01-01".to_string())
    }
    fn to_clause(&self) -> String {
        self.to.clone().unwrap_or_else(|| "9999-12-31".to_string())
    }
    fn is_csv(&self) -> bool {
        self.format.as_deref() == Some("csv")
    }
}

// ─── Response Types ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct TeamDailyRow {
    pub date: String,
    pub user_id: String,
    pub username: String,
    pub segments_saved: i64,
    pub segments_confirmed: i64,
}

#[derive(Debug, Serialize)]
pub struct UserStats {
    pub user_id: String,
    pub username: String,
    pub total_saves: i64,
    pub total_confirms: i64,
    pub mt_used_count: i64,
    pub tm_match_avg: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct ProjectStats {
    pub project_id: String,
    pub total_events: i64,
    pub confirmed_count: i64,
    pub mt_usage_rate: f64,
    pub tm_match_avg: Option<f64>,
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn csv_response(body: String) -> Response {
    ([(header::CONTENT_TYPE, "text/csv; charset=utf-8")], body).into_response()
}

// ─── GET /api/analytics/team ─────────────────────────────────────────────────

pub async fn team_analytics(
    State(state): State<AppState>,
    AuthUser(_claims): AuthUser,
    Query(q): Query<AnalyticsQuery>,
) -> AppResult<Response> {
    let from = q.from_clause();
    let to = q.to_clause();
    let is_csv = q.is_csv();

    let rows = run_db(state.pool.clone(), move |conn| {
        let mut stmt = conn
            .prepare(
                "SELECT
                     substr(e.timestamp, 1, 10)           AS date,
                     e.user_id,
                     u.username,
                     SUM(CASE WHEN e.action = 'save'    THEN 1 ELSE 0 END) AS saves,
                     SUM(CASE WHEN e.action = 'confirm' THEN 1 ELSE 0 END) AS confirms
                 FROM translation_events e
                 JOIN users u ON u.id = e.user_id
                 WHERE e.timestamp >= ?1 AND e.timestamp <= ?2
                 GROUP BY date, e.user_id
                 ORDER BY date, u.username",
            )
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

        let rows = stmt
            .query_map(params![&from, &to], |row| {
                Ok(TeamDailyRow {
                    date: row.get(0)?,
                    user_id: row.get(1)?,
                    username: row.get(2)?,
                    segments_saved: row.get(3)?,
                    segments_confirmed: row.get(4)?,
                })
            })
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

        Ok(rows)
    })
    .await?;

    if is_csv {
        let mut out = "date,user_id,username,segments_saved,segments_confirmed\n".to_string();
        for r in &rows {
            out.push_str(&format!(
                "{},{},{},{},{}\n",
                r.date, r.user_id, r.username, r.segments_saved, r.segments_confirmed
            ));
        }
        return Ok(csv_response(out));
    }

    Ok(Json(rows).into_response())
}

// ─── GET /api/analytics/user/:id ─────────────────────────────────────────────

pub async fn user_analytics(
    State(state): State<AppState>,
    AuthUser(_claims): AuthUser,
    Path(target_user_id): Path<String>,
    Query(q): Query<AnalyticsQuery>,
) -> AppResult<Response> {
    let from = q.from_clause();
    let to = q.to_clause();
    let is_csv = q.is_csv();

    let stats = run_db(state.pool.clone(), move |conn| {
        // Verify user exists
        let username: Option<String> = conn
            .query_row(
                "SELECT username FROM users WHERE id = ?1",
                params![&target_user_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|e: rusqlite::Error| AppError::Internal(anyhow::anyhow!(e)))?;

        let username = username.ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        let stats: UserStats = conn
            .query_row(
                "SELECT
                     e.user_id,
                     COUNT(CASE WHEN e.action = 'save'    THEN 1 END) AS saves,
                     COUNT(CASE WHEN e.action = 'confirm' THEN 1 END) AS confirms,
                     SUM(e.mt_used)                                    AS mt_count,
                     AVG(CAST(e.tm_match_score AS REAL))               AS tm_avg
                 FROM translation_events e
                 WHERE e.user_id = ?1
                   AND e.timestamp >= ?2
                   AND e.timestamp <= ?3
                 GROUP BY e.user_id",
                params![&target_user_id, &from, &to],
                |row| {
                    Ok(UserStats {
                        user_id: row.get(0)?,
                        username: username.clone(),
                        total_saves: row.get(1)?,
                        total_confirms: row.get(2)?,
                        mt_used_count: row.get(3)?,
                        tm_match_avg: row.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(|e: rusqlite::Error| AppError::Internal(anyhow::anyhow!(e)))?
            .unwrap_or(UserStats {
                user_id: target_user_id,
                username,
                total_saves: 0,
                total_confirms: 0,
                mt_used_count: 0,
                tm_match_avg: None,
            });

        Ok(stats)
    })
    .await?;

    if is_csv {
        let out = format!(
            "user_id,username,total_saves,total_confirms,mt_used_count,tm_match_avg\n{},{},{},{},{},{}\n",
            stats.user_id,
            stats.username,
            stats.total_saves,
            stats.total_confirms,
            stats.mt_used_count,
            stats.tm_match_avg.map(|v| format!("{:.2}", v)).unwrap_or_default(),
        );
        return Ok(csv_response(out));
    }

    Ok(Json(stats).into_response())
}

// ─── GET /api/analytics/project/:id ──────────────────────────────────────────

pub async fn project_analytics(
    State(state): State<AppState>,
    AuthUser(_claims): AuthUser,
    Path(project_id): Path<String>,
    Query(q): Query<AnalyticsQuery>,
) -> AppResult<Response> {
    let from = q.from_clause();
    let to = q.to_clause();
    let is_csv = q.is_csv();

    let stats = run_db(state.pool.clone(), move |conn| {
        // Verify project exists
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM projects WHERE id = ?1",
                params![&project_id],
                |row| row.get::<_, i64>(0),
            )
            .map(|n| n > 0)
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

        if !exists {
            return Err(AppError::NotFound("Project not found".to_string()));
        }

        let stats: ProjectStats = conn
            .query_row(
                "SELECT
                     e.project_id,
                     COUNT(*)                                              AS total,
                     COUNT(CASE WHEN e.action = 'confirm' THEN 1 END)     AS confirmed,
                     AVG(CAST(e.mt_used AS REAL))                         AS mt_rate,
                     AVG(CAST(e.tm_match_score AS REAL))                  AS tm_avg
                 FROM translation_events e
                 WHERE e.project_id = ?1
                   AND e.timestamp >= ?2
                   AND e.timestamp <= ?3
                 GROUP BY e.project_id",
                params![&project_id, &from, &to],
                |row| {
                    Ok(ProjectStats {
                        project_id: row.get(0)?,
                        total_events: row.get(1)?,
                        confirmed_count: row.get(2)?,
                        mt_usage_rate: row.get(3)?,
                        tm_match_avg: row.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(|e: rusqlite::Error| AppError::Internal(anyhow::anyhow!(e)))?
            .unwrap_or(ProjectStats {
                project_id,
                total_events: 0,
                confirmed_count: 0,
                mt_usage_rate: 0.0,
                tm_match_avg: None,
            });

        Ok(stats)
    })
    .await?;

    if is_csv {
        let out = format!(
            "project_id,total_events,confirmed_count,mt_usage_rate,tm_match_avg\n{},{},{},{:.4},{}\n",
            stats.project_id,
            stats.total_events,
            stats.confirmed_count,
            stats.mt_usage_rate,
            stats.tm_match_avg.map(|v| format!("{:.2}", v)).unwrap_or_default(),
        );
        return Ok(csv_response(out));
    }

    Ok(Json(stats).into_response())
}

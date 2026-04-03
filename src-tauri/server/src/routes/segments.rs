use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use rusqlite::{params, OptionalExtension};

use crate::{
    app::AppState,
    auth::middleware::AuthUser,
    db::run_db,
    error::{AppError, AppResult},
    models::segment::{Segment, UpdateSegmentRequest},
};

/// GET /api/projects/:projectId/segments
pub async fn list_segments(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(project_id): Path<String>,
) -> AppResult<Json<Vec<Segment>>> {
    ensure_project_owner(&state, &project_id, &claims.sub).await?;

    let pool = state.pool.clone();
    let segments = run_db(pool, move |conn| {
        let mut stmt = conn
            .prepare(
                "SELECT s.id, s.file_id, s.seg_order, s.source, s.target, s.status, s.updated_at
                 FROM segments s
                 JOIN project_files f ON f.id = s.file_id
                 WHERE f.project_id = ?1
                 ORDER BY f.id, s.seg_order",
            )
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

        let rows = stmt
            .query_map(params![&project_id], |row| {
                Ok(Segment {
                    id: row.get(0)?,
                    file_id: row.get(1)?,
                    seg_order: row.get(2)?,
                    source: row.get(3)?,
                    target: row.get(4)?,
                    status: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

        Ok(rows)
    })
    .await?;

    Ok(Json(segments))
}

/// PATCH /api/projects/:projectId/segments/:segId
pub async fn update_segment(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path((project_id, seg_id)): Path<(String, String)>,
    Json(body): Json<UpdateSegmentRequest>,
) -> AppResult<Json<Segment>> {
    ensure_project_owner(&state, &project_id, &claims.sub).await?;

    let pool = state.pool.clone();
    let now = Utc::now().to_rfc3339();

    let seg = run_db(pool, move |conn| {
        // Verify segment belongs to this project
        let seg: Option<Segment> = conn
            .query_row(
                "SELECT s.id, s.file_id, s.seg_order, s.source, s.target, s.status, s.updated_at
                 FROM segments s
                 JOIN project_files f ON f.id = s.file_id
                 WHERE s.id = ?1 AND f.project_id = ?2",
                params![&seg_id, &project_id],
                |row| Ok(Segment {
                    id: row.get(0)?,
                    file_id: row.get(1)?,
                    seg_order: row.get(2)?,
                    source: row.get(3)?,
                    target: row.get(4)?,
                    status: row.get(5)?,
                    updated_at: row.get(6)?,
                }),
            )
            .optional()
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

        let current = seg.ok_or_else(|| AppError::NotFound("Segment not found".to_string()))?;

        let new_target = body.target.as_deref().unwrap_or(&current.target).to_string();
        let new_status = body.status.as_deref().unwrap_or(&current.status).to_string();

        conn.execute(
            "UPDATE segments SET target=?1, status=?2, updated_at=?3 WHERE id=?4",
            params![&new_target, &new_status, &now, &seg_id],
        )
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

        Ok(Segment {
            target: new_target,
            status: new_status,
            updated_at: now,
            ..current
        })
    })
    .await?;

    Ok(Json(seg))
}

// ─── Helper ──────────────────────────────────────────────────────────────────

async fn ensure_project_owner(
    state: &AppState,
    project_id: &str,
    user_id: &str,
) -> AppResult<()> {
    let pid = project_id.to_string();
    let uid = user_id.to_string();
    let pool = state.pool.clone();

    run_db(pool, move |conn| {
        let owner: Option<String> = conn
            .query_row(
                "SELECT owner_id FROM projects WHERE id = ?1",
                params![&pid],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

        match owner {
            None => Err(AppError::NotFound("Project not found".to_string())),
            Some(oid) if oid != uid => Err(AppError::Forbidden),
            _ => Ok(()),
        }
    })
    .await
}

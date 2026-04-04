use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use rusqlite::{params, OptionalExtension};
use uuid::Uuid;

use crate::{
    app::AppState,
    auth::middleware::AuthUser,
    db::run_db,
    error::{AppError, AppResult},
    models::project::{CreateProjectRequest, Project, UpdateProjectRequest},
};

/// GET /api/projects
pub async fn list_projects(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> AppResult<Json<Vec<Project>>> {
    let owner_id = claims.sub.clone();
    let pool = state.pool.clone();
    let projects = run_db(pool, move |conn| {
        let mut stmt = conn
            .prepare(
                "SELECT id, name, source_lang, target_lang, owner_id, created_at, updated_at
                 FROM projects WHERE owner_id = ?1 ORDER BY created_at DESC",
            )
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

        let rows = stmt
            .query_map(params![&owner_id], |row| {
                Ok(Project {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    source_lang: row.get(2)?,
                    target_lang: row.get(3)?,
                    owner_id: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

        Ok(rows)
    })
    .await?;

    Ok(Json(projects))
}

/// POST /api/projects
pub async fn create_project(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(body): Json<CreateProjectRequest>,
) -> AppResult<Json<Project>> {
    if body.name.trim().is_empty() {
        return Err(AppError::BadRequest("Project name is required".to_string()));
    }

    let now = Utc::now().to_rfc3339();
    let project = Project {
        id: Uuid::new_v4().to_string(),
        name: body.name,
        source_lang: body.source_lang,
        target_lang: body.target_lang,
        owner_id: claims.sub.clone(),
        created_at: now.clone(),
        updated_at: now,
    };
    let p = project.clone();
    let pool = state.pool.clone();

    run_db(pool, move |conn| {
        conn.execute(
            "INSERT INTO projects (id, name, source_lang, target_lang, owner_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![&p.id, &p.name, &p.source_lang, &p.target_lang, &p.owner_id, &p.created_at, &p.updated_at],
        )
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
        Ok(())
    })
    .await?;

    Ok(Json(project))
}

/// GET /api/projects/:id
pub async fn get_project(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(id): Path<String>,
) -> AppResult<Json<Project>> {
    let owner_id = claims.sub.clone();
    let pool = state.pool.clone();
    let project = run_db(pool, move |conn| {
        conn.query_row(
            "SELECT id, name, source_lang, target_lang, owner_id, created_at, updated_at
             FROM projects WHERE id = ?1",
            params![&id],
            |row| {
                Ok(Project {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    source_lang: row.get(2)?,
                    target_lang: row.get(3)?,
                    owner_id: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        )
        .optional()
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))
    })
    .await?
    .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

    if project.owner_id != owner_id {
        return Err(AppError::Forbidden);
    }
    Ok(Json(project))
}

/// PATCH /api/projects/:id
pub async fn update_project(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(id): Path<String>,
    Json(body): Json<UpdateProjectRequest>,
) -> AppResult<Json<Project>> {
    let owner_id = claims.sub.clone();
    let pool = state.pool.clone();

    // Fetch current
    let current = run_db(pool.clone(), move |conn| {
        conn.query_row(
            "SELECT id, name, source_lang, target_lang, owner_id, created_at, updated_at
             FROM projects WHERE id = ?1",
            params![&id],
            |row| {
                Ok(Project {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    source_lang: row.get(2)?,
                    target_lang: row.get(3)?,
                    owner_id: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        )
        .optional()
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))
    })
    .await?
    .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

    if current.owner_id != owner_id {
        return Err(AppError::Forbidden);
    }

    let updated = Project {
        name: body.name.unwrap_or(current.name),
        source_lang: body.source_lang.unwrap_or(current.source_lang),
        target_lang: body.target_lang.unwrap_or(current.target_lang),
        updated_at: Utc::now().to_rfc3339(),
        ..current
    };
    let u = updated.clone();

    run_db(pool, move |conn| {
        conn.execute(
            "UPDATE projects SET name=?1, source_lang=?2, target_lang=?3, updated_at=?4 WHERE id=?5",
            params![&u.name, &u.source_lang, &u.target_lang, &u.updated_at, &u.id],
        )
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
        Ok(())
    })
    .await?;

    Ok(Json(updated))
}

/// DELETE /api/projects/:id
pub async fn delete_project(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let owner_id = claims.sub.clone();
    let pool = state.pool.clone();

    let rows = run_db(pool, move |conn| {
        // Verify ownership first
        let owner: Option<String> = conn
            .query_row(
                "SELECT owner_id FROM projects WHERE id = ?1",
                params![&id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

        match owner {
            None => return Err(AppError::NotFound("Project not found".to_string())),
            Some(oid) if oid != owner_id => return Err(AppError::Forbidden),
            _ => {}
        }

        conn.execute("DELETE FROM projects WHERE id = ?1", params![&id])
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))
    })
    .await?;

    Ok(Json(serde_json::json!({ "deleted": rows })))
}

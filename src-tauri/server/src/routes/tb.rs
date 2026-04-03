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
    models::tb::{CreateTbRequest, TbEntry, UpdateTbRequest},
};

/// GET /api/tb
pub async fn list_tb(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> AppResult<Json<Vec<TbEntry>>> {
    let owner_id = claims.sub.clone();
    let pool = state.pool.clone();
    let entries = run_db(pool, move |conn| {
        let mut stmt = conn
            .prepare(
                "SELECT id, source_term, target_term, source_lang, target_lang, notes, forbidden, owner_id, created_at
                 FROM tb_entries WHERE owner_id = ?1 OR owner_id IS NULL
                 ORDER BY source_term ASC",
            )
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

        let rows = stmt
            .query_map(params![&owner_id], |row| {
                Ok(TbEntry {
                    id: row.get(0)?,
                    source_term: row.get(1)?,
                    target_term: row.get(2)?,
                    source_lang: row.get(3)?,
                    target_lang: row.get(4)?,
                    notes: row.get(5)?,
                    forbidden: row.get::<_, i64>(6)? != 0,
                    owner_id: row.get(7)?,
                    created_at: row.get(8)?,
                })
            })
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

        Ok(rows)
    })
    .await?;

    Ok(Json(entries))
}

/// POST /api/tb
pub async fn create_tb(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(body): Json<CreateTbRequest>,
) -> AppResult<Json<TbEntry>> {
    if body.source_term.trim().is_empty() || body.target_term.trim().is_empty() {
        return Err(AppError::BadRequest(
            "source_term and target_term are required".to_string(),
        ));
    }

    let now = Utc::now().to_rfc3339();
    let entry = TbEntry {
        id: Uuid::new_v4().to_string(),
        source_term: body.source_term,
        target_term: body.target_term,
        source_lang: body.source_lang,
        target_lang: body.target_lang,
        notes: body.notes.unwrap_or_default(),
        forbidden: body.forbidden.unwrap_or(false),
        owner_id: Some(claims.sub.clone()),
        created_at: now,
    };
    let e = entry.clone();
    let pool = state.pool.clone();

    run_db(pool, move |conn| {
        conn.execute(
            "INSERT INTO tb_entries (id, source_term, target_term, source_lang, target_lang, notes, forbidden, owner_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                &e.id, &e.source_term, &e.target_term, &e.source_lang, &e.target_lang,
                &e.notes, &(e.forbidden as i64), &e.owner_id, &e.created_at,
            ],
        )
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
        Ok(())
    })
    .await?;

    Ok(Json(entry))
}

/// PATCH /api/tb/:id
pub async fn update_tb(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(id): Path<String>,
    Json(body): Json<UpdateTbRequest>,
) -> AppResult<Json<TbEntry>> {
    let owner_id = claims.sub.clone();
    let pool = state.pool.clone();

    let current = run_db(pool.clone(), move |conn| {
        conn.query_row(
            "SELECT id, source_term, target_term, source_lang, target_lang, notes, forbidden, owner_id, created_at
             FROM tb_entries WHERE id = ?1",
            params![&id],
            |row| Ok(TbEntry {
                id: row.get(0)?,
                source_term: row.get(1)?,
                target_term: row.get(2)?,
                source_lang: row.get(3)?,
                target_lang: row.get(4)?,
                notes: row.get(5)?,
                forbidden: row.get::<_, i64>(6)? != 0,
                owner_id: row.get(7)?,
                created_at: row.get(8)?,
            }),
        )
        .optional()
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))
    })
    .await?
    .ok_or_else(|| AppError::NotFound("TB entry not found".to_string()))?;

    if current.owner_id.as_deref() != Some(&owner_id) {
        return Err(AppError::Forbidden);
    }

    let updated = TbEntry {
        source_term: body.source_term.unwrap_or(current.source_term),
        target_term: body.target_term.unwrap_or(current.target_term),
        notes: body.notes.unwrap_or(current.notes),
        forbidden: body.forbidden.unwrap_or(current.forbidden),
        ..current
    };
    let u = updated.clone();

    run_db(pool, move |conn| {
        conn.execute(
            "UPDATE tb_entries SET source_term=?1, target_term=?2, notes=?3, forbidden=?4 WHERE id=?5",
            params![&u.source_term, &u.target_term, &u.notes, &(u.forbidden as i64), &u.id],
        )
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
        Ok(())
    })
    .await?;

    Ok(Json(updated))
}

/// DELETE /api/tb/:id
pub async fn delete_tb(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let owner_id = claims.sub.clone();
    let pool = state.pool.clone();

    run_db(pool, move |conn| {
        let existing: Option<(String, Option<String>)> = conn
            .query_row(
                "SELECT id, owner_id FROM tb_entries WHERE id = ?1",
                params![&id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

        match existing {
            None => return Err(AppError::NotFound("TB entry not found".to_string())),
            Some((_, Some(oid))) if oid != owner_id => return Err(AppError::Forbidden),
            _ => {}
        }

        conn.execute("DELETE FROM tb_entries WHERE id = ?1", params![&id])
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
        Ok(())
    })
    .await?;

    Ok(Json(serde_json::json!({ "deleted": 1 })))
}

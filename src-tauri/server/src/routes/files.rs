use axum::body::Bytes;
use axum::extract::Multipart;
use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use rusqlite::params;
use uuid::Uuid;

use crate::{
    app::AppState,
    auth::middleware::AuthUser,
    db::run_db,
    error::{AppError, AppResult},
    models::project::ProjectFile,
    parser::xliff::parse_xliff,
};

/// POST /api/projects/:projectId/files  (multipart)
pub async fn upload_file(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(project_id): Path<String>,
    mut multipart: Multipart,
) -> AppResult<Json<ProjectFile>> {
    // Verify ownership
    let owner_id = claims.sub.clone();
    let pid = project_id.clone();
    let pool = state.pool.clone();
    run_db(pool.clone(), move |conn| {
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
            Some(oid) if oid != owner_id => Err(AppError::Forbidden),
            _ => Ok(()),
        }
    })
    .await?;

    // Read multipart fields — look for the field named "file"
    let mut file_name = String::from("upload");
    let mut bytes = Bytes::new();
    let mut found = false;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Multipart error: {}", e)))?
    {
        let field_name = field.name().unwrap_or("").to_string();
        if field_name == "file" {
            file_name = field.file_name().unwrap_or("upload").to_string();
            bytes = field
                .bytes()
                .await
                .map_err(|e| AppError::BadRequest(format!("Failed to read file: {}", e)))?;
            found = true;
            break;
        }
        // consume other fields (e.g. "filename" text field) and discard
        let _ = field.bytes().await;
    }

    if !found {
        return Err(AppError::BadRequest(
            "No 'file' field in request".to_string(),
        ));
    }

    // Parse based on extension
    let ext = std::path::Path::new(&file_name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let segments = if ext == "xliff" || ext == "xlf" {
        parse_xliff(&bytes).map_err(|e| AppError::BadRequest(e.to_string()))?
    } else {
        return Err(AppError::BadRequest(
            "Only XLIFF (.xliff/.xlf) files are supported for now".to_string(),
        ));
    };

    let now = Utc::now().to_rfc3339();
    let file_id = Uuid::new_v4().to_string();
    let file_name_clone = file_name.clone();
    let project_id_clone = project_id.clone();
    let now_clone = now.clone();
    let fid = file_id.clone();

    run_db(pool.clone(), move |conn| {
        // Insert project_file record (file_path is the original filename for now)
        conn.execute(
            "INSERT INTO project_files (id, project_id, name, file_path, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                &fid,
                &project_id_clone,
                &file_name_clone,
                &file_name_clone,
                &now_clone
            ],
        )
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
        Ok(())
    })
    .await?;

    // Insert parsed segments
    let fid2 = file_id.clone();
    let now2 = now.clone();
    run_db(pool, move |conn| {
        for (i, seg) in segments.iter().enumerate() {
            let seg_id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO segments (id, file_id, seg_order, source, target, status, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    &seg_id,
                    &fid2,
                    &(i as i64),
                    &seg.source,
                    &seg.target,
                    if seg.target.is_empty() {
                        "untranslated"
                    } else {
                        "translated"
                    },
                    &now2,
                ],
            )
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
        }
        Ok(())
    })
    .await?;

    Ok(Json(ProjectFile {
        id: file_id,
        project_id,
        name: file_name,
        file_path: String::new(),
        created_at: now,
    }))
}

// helper for query_row().optional()
use rusqlite::OptionalExtension;

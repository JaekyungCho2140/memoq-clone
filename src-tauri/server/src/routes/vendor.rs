//! Vendor portal API routes.
//!
//! POST /api/projects/:projectId/assignments  — assign a file to a vendor (owner/admin)
//! GET  /api/vendor/assignments               — list my assignments (vendor)
//! POST /api/assignments/:id/submit           — vendor submits work
//! POST /api/assignments/:id/approve          — owner/admin approves
//! POST /api/assignments/:id/reject           — owner/admin rejects

use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    app::AppState,
    auth::middleware::AuthUser,
    db::run_db,
    error::{AppError, AppResult},
};

// ─── Types ───────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Clone)]
pub struct Assignment {
    pub id: String,
    pub project_id: String,
    pub file_id: Option<String>,
    pub vendor_id: String,
    pub status: String,
    pub notes: String,
    pub submitted_at: Option<String>,
    pub reviewed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateAssignmentRequest {
    /// ID of the vendor user to assign.
    pub vendor_id: String,
    /// Optional: restrict assignment to a specific file in the project.
    pub file_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReviewRequest {
    pub notes: Option<String>,
}

// ─── Handlers ────────────────────────────────────────────────────────────────

/// POST /api/projects/:projectId/assignments
///
/// Creates a new vendor assignment for the given project (and optionally a file).
/// Requires: caller is the project owner or has role `admin`.
pub async fn create_assignment(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(project_id): Path<String>,
    Json(req): Json<CreateAssignmentRequest>,
) -> AppResult<Json<Assignment>> {
    let caller_id = claims.sub.clone();
    let caller_role = claims.role.clone();
    let pool = state.pool.clone();
    let pid = project_id.clone();

    // Verify caller owns the project or is admin
    let owner_id: Option<String> = run_db(pool.clone(), move |conn| {
        conn.query_row(
            "SELECT owner_id FROM projects WHERE id = ?1",
            params![&pid],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))
    })
    .await?;

    let owner_id = owner_id.ok_or_else(|| AppError::NotFound("Project not found".into()))?;
    if owner_id != caller_id && caller_role != "admin" {
        return Err(AppError::Forbidden);
    }

    // Validate vendor user exists and has role 'vendor'
    let vend_id = req.vendor_id.clone();
    let vendor_exists: Option<String> = run_db(pool.clone(), move |conn| {
        conn.query_row(
            "SELECT id FROM users WHERE id = ?1 AND role = 'vendor'",
            params![&vend_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))
    })
    .await?;

    if vendor_exists.is_none() {
        return Err(AppError::BadRequest(
            "Vendor user not found or does not have 'vendor' role".into(),
        ));
    }

    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    let assignment = Assignment {
        id: id.clone(),
        project_id: project_id.clone(),
        file_id: req.file_id.clone(),
        vendor_id: req.vendor_id.clone(),
        status: "pending".to_string(),
        notes: String::new(),
        submitted_at: None,
        reviewed_at: None,
        created_at: now.clone(),
        updated_at: now.clone(),
    };

    let a = assignment.clone();
    run_db(pool, move |conn| {
        conn.execute(
            "INSERT INTO vendor_assignments
             (id, project_id, file_id, vendor_id, status, notes, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 'pending', '', ?5, ?6)",
            params![&a.id, &a.project_id, &a.file_id, &a.vendor_id, &a.created_at, &a.updated_at],
        )
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
        Ok(())
    })
    .await?;

    Ok(Json(assignment))
}

/// GET /api/vendor/assignments
///
/// Lists assignments for the authenticated vendor.
pub async fn list_my_assignments(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> AppResult<Json<Vec<Assignment>>> {
    let vendor_id = claims.sub.clone();
    let pool = state.pool.clone();

    let rows = run_db(pool, move |conn| {
        let mut stmt = conn
            .prepare(
                "SELECT id, project_id, file_id, vendor_id, status, notes,
                    submitted_at, reviewed_at, created_at, updated_at
             FROM vendor_assignments
             WHERE vendor_id = ?1
             ORDER BY created_at DESC",
            )
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
        let items = stmt
            .query_map(params![&vendor_id], |r| {
                Ok(Assignment {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    file_id: r.get(2)?,
                    vendor_id: r.get(3)?,
                    status: r.get(4)?,
                    notes: r.get(5)?,
                    submitted_at: r.get(6)?,
                    reviewed_at: r.get(7)?,
                    created_at: r.get(8)?,
                    updated_at: r.get(9)?,
                })
            })
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
        Ok(items)
    })
    .await?;

    Ok(Json(rows))
}

/// POST /api/assignments/:id/submit
///
/// Vendor marks work as submitted.
pub async fn submit_assignment(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(assignment_id): Path<String>,
) -> AppResult<Json<Assignment>> {
    let vendor_id = claims.sub.clone();
    let pool = state.pool.clone();
    let aid = assignment_id.clone();

    let now = Utc::now().to_rfc3339();
    let n = now.clone();

    let updated = run_db(pool, move |conn| {
        // Verify ownership and current status
        let row: Option<(String, String)> = conn
            .query_row(
                "SELECT vendor_id, status FROM vendor_assignments WHERE id = ?1",
                params![&aid],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

        let (db_vendor, db_status) =
            row.ok_or_else(|| AppError::NotFound("Assignment not found".into()))?;

        if db_vendor != vendor_id {
            return Err(AppError::Forbidden);
        }
        if db_status != "pending" {
            return Err(AppError::BadRequest(format!(
                "Cannot submit an assignment with status '{}'",
                db_status
            )));
        }

        conn.execute(
            "UPDATE vendor_assignments SET status = 'submitted', submitted_at = ?1, updated_at = ?2 WHERE id = ?3",
            params![&n, &n, &aid],
        )
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

        fetch_assignment(conn, &aid)
    })
    .await?;

    Ok(Json(updated))
}

/// POST /api/assignments/:id/approve
///
/// Project owner/admin approves a submitted assignment.
pub async fn approve_assignment(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(assignment_id): Path<String>,
    Json(req): Json<ReviewRequest>,
) -> AppResult<Json<Assignment>> {
    update_assignment_status(state, claims, assignment_id, "approved", req.notes).await
}

/// POST /api/assignments/:id/reject
///
/// Project owner/admin rejects a submitted assignment.
pub async fn reject_assignment(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(assignment_id): Path<String>,
    Json(req): Json<ReviewRequest>,
) -> AppResult<Json<Assignment>> {
    update_assignment_status(state, claims, assignment_id, "rejected", req.notes).await
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

async fn update_assignment_status(
    state: AppState,
    claims: crate::auth::jwt::Claims,
    assignment_id: String,
    new_status: &'static str,
    notes: Option<String>,
) -> AppResult<Json<Assignment>> {
    let caller_id = claims.sub.clone();
    let caller_role = claims.role.clone();
    let pool = state.pool.clone();
    let aid = assignment_id.clone();
    let notes_val = notes.unwrap_or_default();

    let now = Utc::now().to_rfc3339();
    let n = now.clone();

    let updated = run_db(pool, move |conn| {
        // Load assignment + project owner
        let row: Option<(String, String, String)> = conn
            .query_row(
                "SELECT va.project_id, va.status, p.owner_id
                 FROM vendor_assignments va
                 JOIN projects p ON p.id = va.project_id
                 WHERE va.id = ?1",
                params![&aid],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

        let (_, db_status, project_owner) =
            row.ok_or_else(|| AppError::NotFound("Assignment not found".into()))?;

        if project_owner != caller_id && caller_role != "admin" {
            return Err(AppError::Forbidden);
        }
        if db_status != "submitted" {
            return Err(AppError::BadRequest(format!(
                "Can only review a 'submitted' assignment; current status: '{}'",
                db_status
            )));
        }

        conn.execute(
            "UPDATE vendor_assignments
             SET status = ?1, notes = ?2, reviewed_at = ?3, updated_at = ?4
             WHERE id = ?5",
            params![new_status, &notes_val, &n, &n, &aid],
        )
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

        fetch_assignment(conn, &aid)
    })
    .await?;

    Ok(Json(updated))
}

fn fetch_assignment(
    conn: &rusqlite::Connection,
    id: &str,
) -> crate::error::AppResult<Assignment> {
    conn.query_row(
        "SELECT id, project_id, file_id, vendor_id, status, notes,
                submitted_at, reviewed_at, created_at, updated_at
         FROM vendor_assignments WHERE id = ?1",
        params![id],
        |r| {
            Ok(Assignment {
                id: r.get(0)?,
                project_id: r.get(1)?,
                file_id: r.get(2)?,
                vendor_id: r.get(3)?,
                status: r.get(4)?,
                notes: r.get(5)?,
                submitted_at: r.get(6)?,
                reviewed_at: r.get(7)?,
                created_at: r.get(8)?,
                updated_at: r.get(9)?,
            })
        },
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))
}

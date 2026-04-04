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

// ── 타입 정의 ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginParam {
    pub key: String,
    pub label: String,
    pub value: String,
    #[serde(default)]
    pub secret: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Plugin {
    pub id: String,
    pub name: String,
    pub version: String,
    pub kind: String,
    pub enabled: bool,
    pub params: Vec<PluginParam>,
    pub error: Option<String>,
    pub installed_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallPluginRequest {
    pub wasm_file: String,
    #[serde(default)]
    pub param_values: std::collections::HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePluginRequest {
    pub enabled: Option<bool>,
    pub param_values: Option<std::collections::HashMap<String, String>>,
}

// ── DB 행 → Plugin 변환 헬퍼 ──────────────────────────────────────────────────

fn row_to_plugin(
    id: String,
    name: String,
    version: String,
    kind: String,
    enabled: bool,
    params_json: String,
    error: Option<String>,
    installed_at: String,
) -> Plugin {
    let params: Vec<PluginParam> =
        serde_json::from_str(&params_json).unwrap_or_default();
    Plugin {
        id,
        name,
        version,
        kind,
        enabled,
        params,
        error,
        installed_at,
    }
}

// ── 핸들러 ─────────────────────────────────────────────────────────────────────

/// GET /api/plugins
pub async fn list_plugins(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> AppResult<Json<Vec<Plugin>>> {
    let owner_id = claims.sub.clone();
    let pool = state.pool.clone();

    let plugins = run_db(pool, move |conn| {
        let mut stmt = conn
            .prepare(
                "SELECT id, name, version, kind, enabled, params_json, error, installed_at
                 FROM plugins WHERE owner_id = ?1
                 ORDER BY installed_at ASC",
            )
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

        let rows = stmt
            .query_map(params![&owner_id], |row| {
                Ok(row_to_plugin(
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get::<_, i64>(4)? != 0,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                ))
            })
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

        Ok(rows)
    })
    .await?;

    Ok(Json(plugins))
}

/// POST /api/plugins
pub async fn install_plugin(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(body): Json<InstallPluginRequest>,
) -> AppResult<Json<Plugin>> {
    let owner_id = claims.sub.clone();
    let pool = state.pool.clone();
    let now = Utc::now().to_rfc3339();

    // wasm_file 경로에서 이름 추출 (예: "/path/to/my-plugin.wasm" → "my-plugin")
    let file_name = std::path::Path::new(&body.wasm_file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown-plugin")
        .to_string();

    let plugin = Plugin {
        id: Uuid::new_v4().to_string(),
        name: file_name,
        version: "1.0.0".to_string(),
        kind: "MtProvider".to_string(), // 기본값; 실제 WASM 파싱 시 교체
        enabled: true,
        params: Vec::new(),
        error: None,
        installed_at: now.clone(),
    };

    let p = plugin.clone();
    let params_json = serde_json::to_string(&p.params).unwrap_or_default();

    run_db(pool, move |conn| {
        conn.execute(
            "INSERT INTO plugins (id, name, version, kind, enabled, params_json, error, owner_id, installed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                &p.id, &p.name, &p.version, &p.kind,
                &(p.enabled as i64), &params_json, &p.error, &owner_id, &p.installed_at,
            ],
        )
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
        Ok(())
    })
    .await?;

    Ok(Json(plugin))
}

/// PATCH /api/plugins/:id
pub async fn update_plugin(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(id): Path<String>,
    Json(body): Json<UpdatePluginRequest>,
) -> AppResult<Json<Plugin>> {
    let owner_id = claims.sub.clone();
    let pool = state.pool.clone();

    let plugin = run_db(pool, move |conn| {
        // 존재 여부 및 소유자 확인
        let existing: Option<(String, String, String, String, i64, String, Option<String>, String)> = conn
            .query_row(
                "SELECT id, name, version, kind, enabled, params_json, error, installed_at
                 FROM plugins WHERE id = ?1 AND owner_id = ?2",
                params![&id, &owner_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?,
                           row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?)),
            )
            .optional()
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

        let (pid, name, version, kind, enabled_int, params_json, error, installed_at) =
            existing.ok_or_else(|| AppError::NotFound("Plugin not found".to_string()))?;

        let new_enabled = body.enabled.map(|e| e as i64).unwrap_or(enabled_int);

        // param_values 업데이트
        let new_params_json = if let Some(ref new_vals) = body.param_values {
            let mut params: Vec<PluginParam> =
                serde_json::from_str(&params_json).unwrap_or_default();
            for p in &mut params {
                if let Some(val) = new_vals.get(&p.key) {
                    p.value = val.clone();
                }
            }
            serde_json::to_string(&params).unwrap_or(params_json)
        } else {
            params_json
        };

        conn.execute(
            "UPDATE plugins SET enabled=?1, params_json=?2 WHERE id=?3",
            params![&new_enabled, &new_params_json, &pid],
        )
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

        Ok(row_to_plugin(
            pid, name, version, kind,
            new_enabled != 0, new_params_json, error, installed_at,
        ))
    })
    .await?;

    Ok(Json(plugin))
}

/// DELETE /api/plugins/:id
pub async fn remove_plugin(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let owner_id = claims.sub.clone();
    let pool = state.pool.clone();

    run_db(pool, move |conn| {
        let exists: Option<String> = conn
            .query_row(
                "SELECT id FROM plugins WHERE id = ?1 AND owner_id = ?2",
                params![&id, &owner_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

        if exists.is_none() {
            return Err(AppError::NotFound("Plugin not found".to_string()));
        }

        conn.execute("DELETE FROM plugins WHERE id = ?1", params![&id])
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
        Ok(())
    })
    .await?;

    Ok(Json(serde_json::json!({ "deleted": 1 })))
}

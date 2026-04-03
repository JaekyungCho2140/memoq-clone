use axum::{
    extract::{ConnectInfo, State},
    Json,
};
use chrono::Utc;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use uuid::Uuid;

/// Extract caller IP for rate-limiting; falls back to loopback when
/// `ConnectInfo` is unavailable (e.g., in-process tests).
fn caller_ip(connect_info: &Option<ConnectInfo<SocketAddr>>) -> String {
    connect_info
        .as_ref()
        .map(|ci| ci.0.ip().to_string())
        .unwrap_or_else(|| "127.0.0.1".to_string())
}

use crate::{
    app::AppState,
    auth::{
        jwt::{decode_token, encode_token, TokenKind},
        middleware::AuthUser,
    },
    db::run_db,
    error::{AppError, AppResult},
    models::user::{User, UserProfile},
};

// ─── DTOs ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

// ─── Handlers ────────────────────────────────────────────────────────────────

/// POST /api/auth/register
pub async fn register(
    State(state): State<AppState>,
    connect_info: Option<ConnectInfo<SocketAddr>>,
    Json(body): Json<RegisterRequest>,
) -> AppResult<Json<UserProfile>> {
    let ip = caller_ip(&connect_info);
    state
        .auth_limiter
        .check_key(&ip)
        .map_err(|_| AppError::TooManyRequests)?;

    if body.username.trim().is_empty() || body.password.len() < 8 {
        return Err(AppError::BadRequest(
            "Username required and password must be at least 8 characters".to_string(),
        ));
    }

    let hash = hash_password(&body.password)?;
    let user = User::new(body.username, body.email, hash, "translator".to_string());
    let user_clone = user.clone();
    let pool = state.pool.clone();

    run_db(pool, move |conn| {
        conn.execute(
            "INSERT INTO users (id, username, email, password_hash, role, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                &user_clone.id,
                &user_clone.username,
                &user_clone.email,
                &user_clone.password_hash,
                &user_clone.role,
                &user_clone.created_at,
                &user_clone.updated_at,
            ],
        )
        .map_err(|e| {
            if e.to_string().contains("UNIQUE") {
                AppError::BadRequest("Username or email already exists".to_string())
            } else {
                AppError::Internal(anyhow::anyhow!(e))
            }
        })?;
        Ok(())
    })
    .await?;

    Ok(Json(UserProfile::from(user)))
}

/// POST /api/auth/login
pub async fn login(
    State(state): State<AppState>,
    connect_info: Option<ConnectInfo<SocketAddr>>,
    Json(body): Json<LoginRequest>,
) -> AppResult<Json<TokenResponse>> {
    let ip = caller_ip(&connect_info);
    state
        .auth_limiter
        .check_key(&ip)
        .map_err(|_| AppError::TooManyRequests)?;

    let username = body.username.clone();
    let pool = state.pool.clone();

    let user: Option<User> = run_db(pool, move |conn| {
        conn.query_row(
            "SELECT id, username, email, password_hash, role, created_at, updated_at
             FROM users WHERE username = ?1",
            params![&username],
            |row| {
                Ok(User {
                    id: row.get(0)?,
                    username: row.get(1)?,
                    email: row.get(2)?,
                    password_hash: row.get(3)?,
                    role: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        )
        .optional()
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))
    })
    .await?;

    let user = user.ok_or_else(|| AppError::Unauthorized("Invalid credentials".to_string()))?;
    verify_password(&body.password, &user.password_hash)?;

    let (access_token, refresh_token) = issue_tokens(&state, &user)?;

    // Store refresh token
    let token_hash = hash_token(&refresh_token);
    let refresh_id = Uuid::new_v4().to_string();
    let refresh_expires = Utc::now()
        .checked_add_signed(chrono::Duration::seconds(state.config.jwt_refresh_expiry_secs))
        .unwrap_or_default()
        .to_rfc3339();
    let now = Utc::now().to_rfc3339();
    let user_id = user.id.clone();
    let pool2 = state.pool.clone();

    run_db(pool2, move |conn| {
        conn.execute(
            "INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at, created_at, revoked)
             VALUES (?1, ?2, ?3, ?4, ?5, 0)",
            params![&refresh_id, &user_id, &token_hash, &refresh_expires, &now],
        )
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
        Ok(())
    })
    .await?;

    Ok(Json(TokenResponse {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: state.config.jwt_access_expiry_secs,
    }))
}

/// POST /api/auth/refresh
pub async fn refresh_token(
    State(state): State<AppState>,
    Json(body): Json<RefreshRequest>,
) -> AppResult<Json<TokenResponse>> {
    let claims = decode_token(&body.refresh_token, &state.config.jwt_secret)?;

    if claims.kind != TokenKind::Refresh {
        return Err(AppError::Unauthorized("Expected refresh token".to_string()));
    }

    let token_hash = hash_token(&body.refresh_token);
    let now_str = Utc::now().to_rfc3339();
    let th = token_hash.clone();
    let ns = now_str.clone();
    let pool = state.pool.clone();

    let row_id: Option<String> = run_db(pool, move |conn| {
        conn.query_row(
            "SELECT id FROM refresh_tokens WHERE token_hash = ?1 AND revoked = 0 AND expires_at > ?2",
            params![&th, &ns],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))
    })
    .await?;

    let row_id = row_id
        .ok_or_else(|| AppError::Unauthorized("Refresh token invalid or expired".to_string()))?;

    // Revoke old token
    let rid = row_id.clone();
    let pool2 = state.pool.clone();
    run_db(pool2, move |conn| {
        conn.execute(
            "UPDATE refresh_tokens SET revoked = 1 WHERE id = ?1",
            params![&rid],
        )
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
        Ok(())
    })
    .await?;

    // Fetch user
    let user_id = claims.sub.clone();
    let pool3 = state.pool.clone();
    let user: Option<User> = run_db(pool3, move |conn| {
        conn.query_row(
            "SELECT id, username, email, password_hash, role, created_at, updated_at
             FROM users WHERE id = ?1",
            params![&user_id],
            |row| Ok(User {
                id: row.get(0)?,
                username: row.get(1)?,
                email: row.get(2)?,
                password_hash: row.get(3)?,
                role: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            }),
        )
        .optional()
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))
    })
    .await?;

    let user = user.ok_or_else(|| AppError::Unauthorized("User not found".to_string()))?;
    let (access_token, new_refresh_token) = issue_tokens(&state, &user)?;

    let new_token_hash = hash_token(&new_refresh_token);
    let new_refresh_id = Uuid::new_v4().to_string();
    let refresh_expires = Utc::now()
        .checked_add_signed(chrono::Duration::seconds(state.config.jwt_refresh_expiry_secs))
        .unwrap_or_default()
        .to_rfc3339();
    let now2 = Utc::now().to_rfc3339();
    let uid2 = user.id.clone();
    let pool4 = state.pool.clone();

    run_db(pool4, move |conn| {
        conn.execute(
            "INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at, created_at, revoked)
             VALUES (?1, ?2, ?3, ?4, ?5, 0)",
            params![&new_refresh_id, &uid2, &new_token_hash, &refresh_expires, &now2],
        )
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
        Ok(())
    })
    .await?;

    Ok(Json(TokenResponse {
        access_token,
        refresh_token: new_refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: state.config.jwt_access_expiry_secs,
    }))
}

/// POST /api/auth/logout
pub async fn logout(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> AppResult<Json<serde_json::Value>> {
    let user_id = claims.sub.clone();
    let pool = state.pool.clone();
    run_db(pool, move |conn| {
        conn.execute(
            "UPDATE refresh_tokens SET revoked = 1 WHERE user_id = ?1",
            params![&user_id],
        )
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
        Ok(())
    })
    .await?;

    Ok(Json(serde_json::json!({ "message": "Logged out successfully" })))
}

/// GET /api/auth/me
pub async fn me(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> AppResult<Json<UserProfile>> {
    let user_id = claims.sub.clone();
    let pool = state.pool.clone();
    let user: Option<User> = run_db(pool, move |conn| {
        conn.query_row(
            "SELECT id, username, email, password_hash, role, created_at, updated_at
             FROM users WHERE id = ?1",
            params![&user_id],
            |row| Ok(User {
                id: row.get(0)?,
                username: row.get(1)?,
                email: row.get(2)?,
                password_hash: row.get(3)?,
                role: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            }),
        )
        .optional()
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))
    })
    .await?;

    let user = user.ok_or_else(|| AppError::NotFound("User not found".to_string()))?;
    Ok(Json(UserProfile::from(user)))
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn issue_tokens(state: &AppState, user: &User) -> AppResult<(String, String)> {
    let access = encode_token(
        &user.id,
        &user.username,
        &user.role,
        &state.config.jwt_secret,
        state.config.jwt_access_expiry_secs,
        TokenKind::Access,
    )?;
    let refresh = encode_token(
        &user.id,
        &user.username,
        &user.role,
        &state.config.jwt_secret,
        state.config.jwt_refresh_expiry_secs,
        TokenKind::Refresh,
    )?;
    Ok((access, refresh))
}

fn hash_password(password: &str) -> AppResult<String> {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Password hash error: {}", e)))
}

fn verify_password(password: &str, hash: &str) -> AppResult<()> {
    use argon2::{
        password_hash::{PasswordHash, PasswordVerifier},
        Argon2,
    };
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Password hash parse error: {}", e)))?;
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| AppError::Unauthorized("Invalid credentials".to_string()))
}

/// FNV-1a hash for token fingerprinting (deterministic storage key, not crypto-secure).
pub(crate) fn hash_token(token: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325_u64;
    for b in token.bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", hash)
}


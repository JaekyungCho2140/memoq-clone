use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub jti: String, // unique token ID — prevents collisions within the same second
    pub sub: String, // user id
    pub username: String,
    pub role: String,
    pub exp: i64,
    pub iat: i64,
    pub kind: TokenKind,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TokenKind {
    Access,
    Refresh,
}

pub fn encode_token(
    user_id: &str,
    username: &str,
    role: &str,
    secret: &str,
    expiry_secs: i64,
    kind: TokenKind,
) -> AppResult<String> {
    let now = Utc::now().timestamp();
    let claims = Claims {
        jti: Uuid::new_v4().to_string(),
        sub: user_id.to_string(),
        username: username.to_string(),
        role: role.to_string(),
        iat: now,
        exp: now + expiry_secs,
        kind,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!("JWT encode error: {}", e)))
}

pub fn decode_token(token: &str, secret: &str) -> AppResult<Claims> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|e| AppError::Unauthorized(format!("Invalid token: {}", e)))
}

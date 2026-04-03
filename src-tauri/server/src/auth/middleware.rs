use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};

use crate::{
    app::AppState,
    auth::jwt::{decode_token, Claims, TokenKind},
    error::AppError,
};

pub async fn require_auth(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = extract_bearer_token(req.headers())
        .ok_or_else(|| AppError::Unauthorized("Missing Authorization header".to_string()))?;

    let claims = decode_token(&token, &state.config.jwt_secret)?;

    if claims.kind != TokenKind::Access {
        return Err(AppError::Unauthorized("Expected access token".to_string()));
    }

    req.extensions_mut().insert(claims);
    Ok(next.run(req).await)
}

fn extract_bearer_token(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.to_string())
}

/// Extractor for authenticated user claims.
pub struct AuthUser(pub Claims);

impl<S> axum::extract::FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    fn from_request_parts<'life0, 'life1, 'async_trait>(
        parts: &'life0 mut axum::http::request::Parts,
        _state: &'life1 S,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = Result<Self, Self::Rejection>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            parts
                .extensions
                .get::<Claims>()
                .cloned()
                .map(AuthUser)
                .ok_or_else(|| AppError::Unauthorized("Not authenticated".to_string()))
        })
    }
}

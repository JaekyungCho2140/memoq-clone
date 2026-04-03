use axum::{
    middleware,
    routing::{get, post},
    Router,
};

use crate::{
    app::AppState,
    auth::{
        handlers::{login, logout, me, refresh_token, register},
        middleware::require_auth,
    },
};

pub fn auth_routes(state: AppState) -> Router {
    let protected = Router::new()
        .route("/logout", post(logout))
        .route("/me", get(me))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth));

    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/refresh", post(refresh_token))
        .merge(protected)
        .with_state(state)
}

/// Health check (public)
pub async fn health() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({ "status": "ok" }))
}

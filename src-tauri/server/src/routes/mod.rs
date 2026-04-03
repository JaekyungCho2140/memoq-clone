pub mod files;
pub mod projects;
pub mod segments;
pub mod tb;
pub mod tm;

use axum::{
    middleware,
    routing::{delete, get, patch, post},
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

pub fn api_routes(state: AppState) -> Router {
    let inner = Router::new()
        // Projects
        .route("/projects", get(projects::list_projects).post(projects::create_project))
        .route(
            "/projects/:id",
            get(projects::get_project)
                .patch(projects::update_project)
                .delete(projects::delete_project),
        )
        // File upload
        .route("/projects/:projectId/files", post(files::upload_file))
        // Segments
        .route("/projects/:projectId/segments", get(segments::list_segments))
        .route(
            "/projects/:projectId/segments/:segId",
            patch(segments::update_segment),
        )
        // TM — GET /api/tm lists all; GET /api/tm?source=...&source_lang=...&target_lang=... searches
        .route("/tm", get(tm::list_or_search_tm).post(tm::create_tm))
        .route("/tm/:id", delete(tm::delete_tm))
        // TB
        .route("/tb", get(tb::list_tb).post(tb::create_tb))
        .route("/tb/:id", patch(tb::update_tb).delete(tb::delete_tb))
        // Apply JWT auth to all /api/* routes
        .layer(middleware::from_fn_with_state(state.clone(), require_auth))
        .with_state(state);

    inner
}

/// Health check (public)
pub async fn health() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({ "status": "ok" }))
}

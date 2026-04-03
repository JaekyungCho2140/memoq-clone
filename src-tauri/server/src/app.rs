use axum::{routing::get, Router};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::{config::Config, db::DbPool, routes};

#[derive(Clone)]
pub struct AppState {
    pub pool: DbPool,
    pub config: Config,
}

pub fn build_router(pool: DbPool, config: Config) -> Router {
    let state = AppState { pool, config };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(routes::health))
        .nest("/api/auth", routes::auth_routes(state))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
}

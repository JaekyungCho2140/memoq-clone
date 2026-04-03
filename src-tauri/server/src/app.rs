use axum::{routing::get, Router};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::{config::Config, db::DbPool, routes, ws::WsState};

#[derive(Clone)]
pub struct AppState {
    pub pool: DbPool,
    pub config: Config,
    pub ws: WsState,
}

pub fn build_router(pool: DbPool, config: Config) -> Router {
    let ws = WsState::new(config.ws_lock_timeout_secs);
    let state = AppState { pool, config, ws };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(routes::health))
        .nest("/api/auth", routes::auth_routes(state.clone()))
        .nest("/api", routes::api_routes(state))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
}

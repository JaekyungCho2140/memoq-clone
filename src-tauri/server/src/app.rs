use axum::{http::HeaderValue, routing::get, Router};
use tower_http::{
    cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer},
    set_header::SetResponseHeaderLayer,
    trace::TraceLayer,
};

use crate::{
    config::Config,
    db::DbPool,
    middleware::rate_limit::{build_limiter, KeyedLimiter},
    routes,
    ws::WsState,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub pool: DbPool,
    pub config: Config,
    pub ws: WsState,
    /// Rate limiter shared by login / register endpoints.
    pub auth_limiter: Arc<KeyedLimiter>,
}

pub fn build_router(pool: DbPool, config: Config) -> Router {
    let ws = WsState::new(config.ws_lock_timeout_secs);
    let auth_limiter = build_limiter(config.auth_rate_limit_per_min);
    let state = AppState {
        pool,
        config: config.clone(),
        ws,
        auth_limiter,
    };

    let cors = build_cors(&config.allowed_origins);

    // Content-Security-Policy: restrict resources to same origin in production.
    // 'unsafe-inline' is kept for the Tauri webview which injects inline styles.
    let csp_value = HeaderValue::from_static(
        "default-src 'self'; \
         script-src 'self' 'unsafe-inline'; \
         style-src 'self' 'unsafe-inline'; \
         img-src 'self' data:; \
         connect-src 'self' http://127.0.0.1:* ws://127.0.0.1:*",
    );

    Router::new()
        .route("/health", get(routes::health))
        .nest("/api/auth", routes::auth_routes(state.clone()))
        .nest("/api", routes::api_routes(state))
        .layer(cors)
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::HeaderName::from_static("content-security-policy"),
            csp_value,
        ))
        .layer(TraceLayer::new_for_http())
}

fn build_cors(allowed_origins: &[String]) -> CorsLayer {
    if allowed_origins.is_empty() {
        // Development / Tauri local mode: permissive
        return CorsLayer::new()
            .allow_origin(AllowOrigin::any())
            .allow_methods(AllowMethods::any())
            .allow_headers(AllowHeaders::any());
    }

    let origins: Vec<HeaderValue> = allowed_origins
        .iter()
        .filter_map(|o| o.parse().ok())
        .collect();

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods(AllowMethods::any())
        .allow_headers(AllowHeaders::any())
}

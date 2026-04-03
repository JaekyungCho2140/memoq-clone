use axum::http::StatusCode;
use axum_test::TestServer;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};

static DB_COUNTER: AtomicU64 = AtomicU64::new(0);

async fn build_test_server() -> TestServer {
    let db_id = DB_COUNTER.fetch_add(1, Ordering::Relaxed);
    let db_name = format!("testdb_{}", db_id);

    let pool =
        server::db::in_memory_pool_named(&db_name).expect("Failed to create in-memory pool");
    server::db::run_migrations(&pool)
        .await
        .expect("Migration failed");

    let config = server::config::Config {
        host: "127.0.0.1".to_string(),
        port: 8080,
        database_url: format!("file:{}?mode=memory&cache=shared", db_name),
        jwt_secret: "test-secret-key-for-tests-only".to_string(),
        jwt_access_expiry_secs: 1800,
        jwt_refresh_expiry_secs: 604800,
    };

    let router = server::app::build_router(pool, config);
    TestServer::new(router).expect("Failed to create test server")
}

#[tokio::test]
async fn test_health_check() {
    let server = build_test_server().await;
    let resp = server.get("/health").await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn test_register_and_login() {
    let server = build_test_server().await;

    let resp = server
        .post("/api/auth/register")
        .json(&json!({
            "username": "testuser",
            "email": "test@example.com",
            "password": "password123"
        }))
        .await;
    resp.assert_status_ok();
    let profile: Value = resp.json();
    assert_eq!(profile["username"], "testuser");
    assert!(!profile.as_object().unwrap().contains_key("password_hash"));

    let resp = server
        .post("/api/auth/login")
        .json(&json!({
            "username": "testuser",
            "password": "password123"
        }))
        .await;
    resp.assert_status_ok();
    let tokens: Value = resp.json();
    assert!(tokens["access_token"].is_string());
    assert!(tokens["refresh_token"].is_string());
    assert_eq!(tokens["token_type"], "Bearer");
}

#[tokio::test]
async fn test_login_wrong_password() {
    let server = build_test_server().await;

    server
        .post("/api/auth/register")
        .json(&json!({
            "username": "user2",
            "email": "user2@example.com",
            "password": "correctpassword"
        }))
        .await;

    let resp = server
        .post("/api/auth/login")
        .json(&json!({
            "username": "user2",
            "password": "wrongpassword"
        }))
        .await;
    resp.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_protected_route_requires_token() {
    let server = build_test_server().await;
    let resp = server.get("/api/auth/me").await;
    resp.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_me_with_valid_token() {
    let server = build_test_server().await;

    server
        .post("/api/auth/register")
        .json(&json!({
            "username": "me_user",
            "email": "me@example.com",
            "password": "password123"
        }))
        .await;

    let login_resp = server
        .post("/api/auth/login")
        .json(&json!({
            "username": "me_user",
            "password": "password123"
        }))
        .await;
    let tokens: Value = login_resp.json();
    let access_token = tokens["access_token"].as_str().unwrap().to_string();

    let resp = server
        .get("/api/auth/me")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", access_token).parse().unwrap(),
        )
        .await;
    resp.assert_status_ok();
    let profile: Value = resp.json();
    assert_eq!(profile["username"], "me_user");
}

#[tokio::test]
async fn test_refresh_token_rotation() {
    let server = build_test_server().await;

    server
        .post("/api/auth/register")
        .json(&json!({
            "username": "refresh_user",
            "email": "refresh@example.com",
            "password": "password123"
        }))
        .await;

    let login_resp = server
        .post("/api/auth/login")
        .json(&json!({
            "username": "refresh_user",
            "password": "password123"
        }))
        .await;
    let tokens: Value = login_resp.json();
    let old_refresh = tokens["refresh_token"].as_str().unwrap().to_string();

    let resp = server
        .post("/api/auth/refresh")
        .json(&json!({ "refresh_token": &old_refresh }))
        .await;
    resp.assert_status_ok();
    let new_tokens: Value = resp.json();
    assert!(new_tokens["access_token"].is_string());
    assert_ne!(new_tokens["refresh_token"], tokens["refresh_token"]);

    // Old refresh token must be revoked after rotation
    let resp2 = server
        .post("/api/auth/refresh")
        .json(&json!({ "refresh_token": &old_refresh }))
        .await;
    resp2.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_logout_invalidates_refresh_token() {
    let server = build_test_server().await;

    server
        .post("/api/auth/register")
        .json(&json!({
            "username": "logout_user",
            "email": "logout@example.com",
            "password": "password123"
        }))
        .await;

    let login_resp = server
        .post("/api/auth/login")
        .json(&json!({
            "username": "logout_user",
            "password": "password123"
        }))
        .await;
    let tokens: Value = login_resp.json();
    let access_token = tokens["access_token"].as_str().unwrap().to_string();
    let refresh_token = tokens["refresh_token"].as_str().unwrap().to_string();

    server
        .post("/api/auth/logout")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", access_token).parse().unwrap(),
        )
        .await
        .assert_status_ok();

    let resp = server
        .post("/api/auth/refresh")
        .json(&json!({ "refresh_token": &refresh_token }))
        .await;
    resp.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_duplicate_username_rejected() {
    let server = build_test_server().await;

    server
        .post("/api/auth/register")
        .json(&json!({
            "username": "dup_user",
            "email": "dup@example.com",
            "password": "password123"
        }))
        .await;

    let resp = server
        .post("/api/auth/register")
        .json(&json!({
            "username": "dup_user",
            "email": "dup2@example.com",
            "password": "password123"
        }))
        .await;
    resp.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_short_password_rejected() {
    let server = build_test_server().await;
    let resp = server
        .post("/api/auth/register")
        .json(&json!({
            "username": "shortpw",
            "email": "short@example.com",
            "password": "1234567"
        }))
        .await;
    resp.assert_status(StatusCode::BAD_REQUEST);
}

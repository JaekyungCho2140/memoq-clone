use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use axum::http::StatusCode;
use axum_test::TestServer;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};

static DB_COUNTER: AtomicU64 = AtomicU64::new(700);

// ─── Setup ───────────────────────────────────────────────────────────────────

struct TestCtx {
    server: TestServer,
    owner_token: String,
    vendor_token: String,
    vendor_id: String,
    project_id: String,
}

async fn setup() -> TestCtx {
    let db_id = DB_COUNTER.fetch_add(1, Ordering::Relaxed);
    let db_name = format!("vendortest_{}", db_id);

    let pool = server::db::in_memory_pool_named(&db_name).expect("pool");
    server::db::run_migrations(&pool).await.expect("migration");

    // Insert a vendor user directly with role='vendor'
    let vendor_id = uuid::Uuid::new_v4().to_string();
    let vendor_hash = {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(b"vendorpass", &salt)
            .unwrap()
            .to_string()
    };
    let now = chrono::Utc::now().to_rfc3339();
    {
        let conn = pool.get().unwrap();
        conn.execute(
            "INSERT INTO users (id, username, email, password_hash, role, created_at, updated_at)
             VALUES (?1, 'vendor_u', 'vendor@v.test', ?2, 'vendor', ?3, ?4)",
            rusqlite::params![&vendor_id, &vendor_hash, &now, &now],
        )
        .unwrap();
    }

    let config = server::config::Config {
        host: "127.0.0.1".to_string(),
        port: 8080,
        database_url: format!("file:{}?mode=memory&cache=shared", db_name),
        jwt_secret: "vendor-test-secret".to_string(),
        jwt_access_expiry_secs: 1800,
        jwt_refresh_expiry_secs: 604800,
        ws_lock_timeout_secs: 0,
        allowed_origins: vec![],
        auth_rate_limit_per_min: 1000,
    };

    let router = server::app::build_router(pool, config);
    let server = TestServer::new(router).expect("test server");

    // Register owner (translator role)
    server
        .post("/api/auth/register")
        .json(&json!({ "username": "owner_u", "email": "owner@v.test", "password": "pass1234" }))
        .await;
    let owner_resp = server
        .post("/api/auth/login")
        .json(&json!({ "username": "owner_u", "password": "pass1234" }))
        .await;
    let owner_token = owner_resp.json::<Value>()["access_token"]
        .as_str()
        .unwrap()
        .to_string();

    // Login as vendor (already inserted with role=vendor)
    let vendor_resp = server
        .post("/api/auth/login")
        .json(&json!({ "username": "vendor_u", "password": "vendorpass" }))
        .await;
    let vendor_token = vendor_resp.json::<Value>()["access_token"]
        .as_str()
        .unwrap()
        .to_string();

    // Create a project as owner
    let project_resp = server
        .post("/api/projects")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", owner_token).parse().unwrap(),
        )
        .json(&json!({ "name": "VendorProj", "source_lang": "en", "target_lang": "ko" }))
        .await;
    assert_eq!(project_resp.status_code(), StatusCode::OK);
    let project_id = project_resp.json::<Value>()["id"]
        .as_str()
        .unwrap()
        .to_string();

    TestCtx {
        server,
        owner_token,
        vendor_token,
        vendor_id,
        project_id,
    }
}

// ─── Assignment creation ──────────────────────────────────────────────────────

#[tokio::test]
async fn test_create_assignment_success() {
    let ctx = setup().await;

    let resp = ctx
        .server
        .post(&format!("/api/projects/{}/assignments", ctx.project_id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", ctx.owner_token).parse().unwrap(),
        )
        .json(&json!({ "vendor_id": ctx.vendor_id }))
        .await;

    assert_eq!(resp.status_code(), StatusCode::OK);
    let body = resp.json::<Value>();
    assert_eq!(body["status"], "pending");
    assert_eq!(body["vendor_id"], ctx.vendor_id);
    assert_eq!(body["project_id"], ctx.project_id);
}

#[tokio::test]
async fn test_create_assignment_rejects_non_vendor_user() {
    let ctx = setup().await;

    // Try to assign the owner themselves (role='translator')
    let me_resp = ctx
        .server
        .get("/api/auth/me")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", ctx.owner_token).parse().unwrap(),
        )
        .await;
    let owner_id = me_resp.json::<Value>()["id"].as_str().unwrap().to_string();

    let resp = ctx
        .server
        .post(&format!("/api/projects/{}/assignments", ctx.project_id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", ctx.owner_token).parse().unwrap(),
        )
        .json(&json!({ "vendor_id": owner_id }))
        .await;

    assert_eq!(resp.status_code(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_create_assignment_non_owner_forbidden() {
    let ctx = setup().await;

    let resp = ctx
        .server
        .post(&format!("/api/projects/{}/assignments", ctx.project_id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", ctx.vendor_token).parse().unwrap(),
        )
        .json(&json!({ "vendor_id": ctx.vendor_id }))
        .await;

    assert_eq!(resp.status_code(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_create_assignment_unknown_project() {
    let ctx = setup().await;

    let resp = ctx
        .server
        .post("/api/projects/nonexistent-id/assignments")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", ctx.owner_token).parse().unwrap(),
        )
        .json(&json!({ "vendor_id": ctx.vendor_id }))
        .await;

    assert_eq!(resp.status_code(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_create_assignment_requires_auth() {
    let ctx = setup().await;

    let resp = ctx
        .server
        .post(&format!("/api/projects/{}/assignments", ctx.project_id))
        .json(&json!({ "vendor_id": ctx.vendor_id }))
        .await;

    assert_eq!(resp.status_code(), StatusCode::UNAUTHORIZED);
}

// ─── List assignments ─────────────────────────────────────────────────────────

#[tokio::test]
async fn test_list_assignments_empty() {
    let ctx = setup().await;

    let resp = ctx
        .server
        .get("/api/vendor/assignments")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", ctx.vendor_token).parse().unwrap(),
        )
        .await;

    assert_eq!(resp.status_code(), StatusCode::OK);
    assert!(resp.json::<Value>().as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_list_assignments_after_create() {
    let ctx = setup().await;

    // Create an assignment
    ctx.server
        .post(&format!("/api/projects/{}/assignments", ctx.project_id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", ctx.owner_token).parse().unwrap(),
        )
        .json(&json!({ "vendor_id": ctx.vendor_id }))
        .await;

    let resp = ctx
        .server
        .get("/api/vendor/assignments")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", ctx.vendor_token).parse().unwrap(),
        )
        .await;

    assert_eq!(resp.status_code(), StatusCode::OK);
    let items = resp.json::<Value>();
    assert_eq!(items.as_array().unwrap().len(), 1);
    assert_eq!(items[0]["status"], "pending");
}

#[tokio::test]
async fn test_list_assignments_requires_auth() {
    let ctx = setup().await;
    let resp = ctx.server.get("/api/vendor/assignments").await;
    assert_eq!(resp.status_code(), StatusCode::UNAUTHORIZED);
}

// ─── Submit / Approve / Reject workflow ──────────────────────────────────────

async fn create_assignment(ctx: &TestCtx) -> String {
    let resp = ctx
        .server
        .post(&format!("/api/projects/{}/assignments", ctx.project_id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", ctx.owner_token).parse().unwrap(),
        )
        .json(&json!({ "vendor_id": ctx.vendor_id }))
        .await;
    resp.json::<Value>()["id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn test_submit_assignment_success() {
    let ctx = setup().await;
    let assignment_id = create_assignment(&ctx).await;

    let resp = ctx
        .server
        .post(&format!("/api/assignments/{}/submit", assignment_id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", ctx.vendor_token).parse().unwrap(),
        )
        .json(&json!({}))
        .await;

    assert_eq!(resp.status_code(), StatusCode::OK);
    let body = resp.json::<Value>();
    assert_eq!(body["status"], "submitted");
    assert!(body["submitted_at"].is_string());
}

#[tokio::test]
async fn test_submit_assignment_wrong_vendor_forbidden() {
    let ctx = setup().await;
    let assignment_id = create_assignment(&ctx).await;

    // Owner tries to submit (not the assigned vendor)
    let resp = ctx
        .server
        .post(&format!("/api/assignments/{}/submit", assignment_id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", ctx.owner_token).parse().unwrap(),
        )
        .json(&json!({}))
        .await;

    assert_eq!(resp.status_code(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_approve_submitted_assignment() {
    let ctx = setup().await;
    let assignment_id = create_assignment(&ctx).await;

    // Vendor submits
    ctx.server
        .post(&format!("/api/assignments/{}/submit", assignment_id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", ctx.vendor_token).parse().unwrap(),
        )
        .json(&json!({}))
        .await;

    // Owner approves
    let resp = ctx
        .server
        .post(&format!("/api/assignments/{}/approve", assignment_id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", ctx.owner_token).parse().unwrap(),
        )
        .json(&json!({ "notes": "Looks good!" }))
        .await;

    assert_eq!(resp.status_code(), StatusCode::OK);
    let body = resp.json::<Value>();
    assert_eq!(body["status"], "approved");
    assert_eq!(body["notes"], "Looks good!");
    assert!(body["reviewed_at"].is_string());
}

#[tokio::test]
async fn test_reject_submitted_assignment() {
    let ctx = setup().await;
    let assignment_id = create_assignment(&ctx).await;

    // Vendor submits
    ctx.server
        .post(&format!("/api/assignments/{}/submit", assignment_id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", ctx.vendor_token).parse().unwrap(),
        )
        .json(&json!({}))
        .await;

    // Owner rejects
    let resp = ctx
        .server
        .post(&format!("/api/assignments/{}/reject", assignment_id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", ctx.owner_token).parse().unwrap(),
        )
        .json(&json!({ "notes": "Needs revision" }))
        .await;

    assert_eq!(resp.status_code(), StatusCode::OK);
    assert_eq!(resp.json::<Value>()["status"], "rejected");
}

#[tokio::test]
async fn test_approve_pending_assignment_fails() {
    let ctx = setup().await;
    let assignment_id = create_assignment(&ctx).await;

    // Try to approve without submitting first
    let resp = ctx
        .server
        .post(&format!("/api/assignments/{}/approve", assignment_id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", ctx.owner_token).parse().unwrap(),
        )
        .json(&json!({}))
        .await;

    assert_eq!(resp.status_code(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_double_submit_fails() {
    let ctx = setup().await;
    let assignment_id = create_assignment(&ctx).await;

    ctx.server
        .post(&format!("/api/assignments/{}/submit", assignment_id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", ctx.vendor_token).parse().unwrap(),
        )
        .json(&json!({}))
        .await;

    let resp = ctx
        .server
        .post(&format!("/api/assignments/{}/submit", assignment_id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", ctx.vendor_token).parse().unwrap(),
        )
        .json(&json!({}))
        .await;

    assert_eq!(resp.status_code(), StatusCode::BAD_REQUEST);
}

use axum::http::StatusCode;
use axum_test::TestServer;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};

static DB_COUNTER: AtomicU64 = AtomicU64::new(500);

async fn setup() -> (TestServer, String) {
    let db_id = DB_COUNTER.fetch_add(1, Ordering::Relaxed);
    let db_name = format!("aligntest_{}", db_id);

    let pool =
        server::db::in_memory_pool_named(&db_name).expect("in-memory pool");
    server::db::run_migrations(&pool).await.expect("migration");

    let config = server::config::Config {
        host: "127.0.0.1".to_string(),
        port: 8080,
        database_url: format!("file:{}?mode=memory&cache=shared", db_name),
        jwt_secret: "align-test-secret".to_string(),
        jwt_access_expiry_secs: 1800,
        jwt_refresh_expiry_secs: 604800,
        ws_lock_timeout_secs: 0,
        allowed_origins: vec![],
        auth_rate_limit_per_min: 1000,
    };

    let router = server::app::build_router(pool, config);
    let server = TestServer::new(router).expect("test server");

    // Register + login
    server
        .post("/api/auth/register")
        .json(&json!({ "username": "aligner", "email": "align@test.com", "password": "pass1234" }))
        .await;
    let login = server
        .post("/api/auth/login")
        .json(&json!({ "username": "aligner", "password": "pass1234" }))
        .await;
    let token = login.json::<Value>()["access_token"]
        .as_str()
        .expect("access_token")
        .to_string();

    (server, token)
}

// ── /api/alignment/align ─────────────────────────────────────────────────────

#[tokio::test]
async fn test_align_txt_files() {
    let (server, token) = setup().await;

    let src_content = "Hello world.\nGoodbye.";
    let tgt_content = "Hola mundo.\nAdios.";

    let resp = server
        .post("/api/alignment/align")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token).parse().unwrap(),
        )
        .multipart(
            axum_test::multipart::MultipartForm::new()
                .add_text("source_lang", "en")
                .add_text("target_lang", "es")
                .add_part(
                    "source_file",
                    axum_test::multipart::Part::bytes(src_content.as_bytes().to_vec())
                        .file_name("source.txt")
                        .mime_type("text/plain"),
                )
                .add_part(
                    "target_file",
                    axum_test::multipart::Part::bytes(tgt_content.as_bytes().to_vec())
                        .file_name("target.txt")
                        .mime_type("text/plain"),
                ),
        )
        .await;

    assert_eq!(resp.status_code(), StatusCode::OK);
    let body = resp.json::<Value>();
    assert_eq!(body["source_lang"], "en");
    assert_eq!(body["target_lang"], "es");
    let pairs = body["result"]["pairs"].as_array().expect("pairs array");
    assert_eq!(pairs.len(), 2, "expected 2 aligned pairs");
}

#[tokio::test]
async fn test_align_missing_file_returns_400() {
    let (server, token) = setup().await;

    let resp = server
        .post("/api/alignment/align")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token).parse().unwrap(),
        )
        .multipart(
            axum_test::multipart::MultipartForm::new()
                .add_text("source_lang", "en")
                .add_text("target_lang", "ko"),
        )
        .await;

    assert_eq!(resp.status_code(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_align_requires_auth() {
    let (server, _) = setup().await;

    let resp = server
        .post("/api/alignment/align")
        .multipart(axum_test::multipart::MultipartForm::new())
        .await;

    assert_eq!(resp.status_code(), StatusCode::UNAUTHORIZED);
}

// ── /api/alignment/confirm ───────────────────────────────────────────────────

#[tokio::test]
async fn test_confirm_alignment_saves_to_tm() {
    let (server, token) = setup().await;

    let resp = server
        .post("/api/alignment/confirm")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token).parse().unwrap(),
        )
        .json(&json!({
            "source_lang": "en",
            "target_lang": "es",
            "pairs": [
                {"source": "Hello world.", "target": "Hola mundo."},
                {"source": "Goodbye.", "target": "Adios."}
            ]
        }))
        .await;

    assert_eq!(resp.status_code(), StatusCode::OK);
    let body = resp.json::<Value>();
    assert_eq!(body["saved"], 2);

    // Verify they appear in TM search
    let tm_resp = server
        .get("/api/tm")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token).parse().unwrap(),
        )
        .add_query_param("source", "Hello world.")
        .add_query_param("source_lang", "en")
        .add_query_param("target_lang", "es")
        .await;

    assert_eq!(tm_resp.status_code(), StatusCode::OK);
    // TM search returns an array of TmSearchResult directly
    let results = tm_resp.json::<Value>();
    let results = results.as_array().expect("TM results should be an array");
    assert!(!results.is_empty(), "TM should have the confirmed entry");
}

#[tokio::test]
async fn test_confirm_empty_pairs_returns_zero() {
    let (server, token) = setup().await;

    let resp = server
        .post("/api/alignment/confirm")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token).parse().unwrap(),
        )
        .json(&json!({
            "source_lang": "en",
            "target_lang": "ko",
            "pairs": []
        }))
        .await;

    assert_eq!(resp.status_code(), StatusCode::OK);
    assert_eq!(resp.json::<Value>()["saved"], 0);
}

#[tokio::test]
async fn test_confirm_requires_auth() {
    let (server, _) = setup().await;

    let resp = server
        .post("/api/alignment/confirm")
        .json(&json!({ "source_lang": "en", "target_lang": "ko", "pairs": [] }))
        .await;

    assert_eq!(resp.status_code(), StatusCode::UNAUTHORIZED);
}

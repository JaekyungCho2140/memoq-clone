use axum::http::StatusCode;
use axum_test::TestServer;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};

static DB_COUNTER: AtomicU64 = AtomicU64::new(900);

async fn setup() -> (TestServer, String) {
    let db_id = DB_COUNTER.fetch_add(1, Ordering::Relaxed);
    let db_name = format!("termtest_{}", db_id);

    let pool = server::db::in_memory_pool_named(&db_name).expect("pool");
    server::db::run_migrations(&pool).await.expect("migration");

    let config = server::config::Config {
        host: "127.0.0.1".to_string(),
        port: 8080,
        database_url: format!("file:{}?mode=memory&cache=shared", db_name),
        jwt_secret: "term-test-secret".to_string(),
        jwt_access_expiry_secs: 1800,
        jwt_refresh_expiry_secs: 604800,
        ws_lock_timeout_secs: 0,
        allowed_origins: vec![],
        auth_rate_limit_per_min: 1000,
    };

    let router = server::app::build_router(pool, config);
    let server = TestServer::new(router).expect("test server");

    server
        .post("/api/auth/register")
        .json(&json!({ "username": "termuser", "email": "term@test.com", "password": "pass1234" }))
        .await;
    let login = server
        .post("/api/auth/login")
        .json(&json!({ "username": "termuser", "password": "pass1234" }))
        .await;
    let token = login.json::<Value>()["access_token"]
        .as_str()
        .unwrap()
        .to_string();

    (server, token)
}

// ─── /api/term-extraction/extract ────────────────────────────────────────────

#[tokio::test]
async fn test_extract_returns_candidates() {
    let (server, token) = setup().await;

    let content = b"machine learning machine learning deep learning deep learning \
                    neural network neural network neural network \
                    machine learning model model model";

    let resp = server
        .post("/api/term-extraction/extract")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token).parse().unwrap(),
        )
        .multipart(
            axum_test::multipart::MultipartForm::new()
                .add_text("source_lang", "en")
                .add_part(
                    "source_file",
                    axum_test::multipart::Part::bytes(content.to_vec())
                        .file_name("doc.txt")
                        .mime_type("text/plain"),
                ),
        )
        .await;

    assert_eq!(resp.status_code(), StatusCode::OK);
    let body = resp.json::<Value>();
    assert_eq!(body["source_lang"], "en");
    let candidates = body["candidates"].as_array().expect("candidates array");
    assert!(!candidates.is_empty(), "should return at least one candidate");
    // Verify each candidate has the expected fields
    for c in candidates {
        assert!(c["term"].is_string());
        assert!(c["score"].is_f64() || c["score"].is_number());
        assert!(c["frequency"].is_number());
    }
}

#[tokio::test]
async fn test_extract_finds_known_terms() {
    let (server, token) = setup().await;

    let content =
        b"photosynthesis photosynthesis photosynthesis chlorophyll chlorophyll photosynthesis \
          chlorophyll photosynthesis";

    let resp = server
        .post("/api/term-extraction/extract")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token).parse().unwrap(),
        )
        .multipart(
            axum_test::multipart::MultipartForm::new()
                .add_text("source_lang", "en")
                .add_part(
                    "source_file",
                    axum_test::multipart::Part::bytes(content.to_vec())
                        .file_name("bio.txt")
                        .mime_type("text/plain"),
                ),
        )
        .await;

    assert_eq!(resp.status_code(), StatusCode::OK);
    let body = resp.json::<Value>();
    let terms: Vec<String> = body["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["term"].as_str().unwrap().to_string())
        .collect();
    assert!(
        terms.contains(&"photosynthesis".to_string()),
        "expected photosynthesis in {:?}",
        terms
    );
}

#[tokio::test]
async fn test_extract_missing_file_returns_400() {
    let (server, token) = setup().await;

    let resp = server
        .post("/api/term-extraction/extract")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token).parse().unwrap(),
        )
        .multipart(
            axum_test::multipart::MultipartForm::new().add_text("source_lang", "en"),
        )
        .await;

    assert_eq!(resp.status_code(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_extract_requires_auth() {
    let (server, _) = setup().await;

    let resp = server
        .post("/api/term-extraction/extract")
        .multipart(axum_test::multipart::MultipartForm::new())
        .await;

    assert_eq!(resp.status_code(), StatusCode::UNAUTHORIZED);
}

// ─── /api/term-extraction/add-to-tb ─────────────────────────────────────────

#[tokio::test]
async fn test_add_to_tb_saves_terms() {
    let (server, token) = setup().await;

    let resp = server
        .post("/api/term-extraction/add-to-tb")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token).parse().unwrap(),
        )
        .json(&json!({
            "source_lang": "en",
            "target_lang": "ko",
            "terms": [
                { "source_term": "machine learning", "target_term": "머신러닝", "notes": "" },
                { "source_term": "neural network",   "target_term": "신경망",   "notes": "" }
            ]
        }))
        .await;

    assert_eq!(resp.status_code(), StatusCode::OK);
    assert_eq!(resp.json::<Value>()["saved"], 2);

    // Verify terms appear in TB search
    let tb_resp = server
        .get("/api/tb")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token).parse().unwrap(),
        )
        .add_query_param("source_lang", "en")
        .add_query_param("target_lang", "ko")
        .await;

    assert_eq!(tb_resp.status_code(), StatusCode::OK);
    let tb_items = tb_resp.json::<Value>();
    let items = tb_items.as_array().expect("tb array");
    assert!(
        items.len() >= 2,
        "TB should contain at least 2 entries; got {}",
        items.len()
    );
}

#[tokio::test]
async fn test_add_to_tb_empty_list_returns_zero() {
    let (server, token) = setup().await;

    let resp = server
        .post("/api/term-extraction/add-to-tb")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token).parse().unwrap(),
        )
        .json(&json!({ "source_lang": "en", "target_lang": "ko", "terms": [] }))
        .await;

    assert_eq!(resp.status_code(), StatusCode::OK);
    assert_eq!(resp.json::<Value>()["saved"], 0);
}

#[tokio::test]
async fn test_add_to_tb_requires_auth() {
    let (server, _) = setup().await;

    let resp = server
        .post("/api/term-extraction/add-to-tb")
        .json(&json!({ "source_lang": "en", "target_lang": "ko", "terms": [] }))
        .await;

    assert_eq!(resp.status_code(), StatusCode::UNAUTHORIZED);
}

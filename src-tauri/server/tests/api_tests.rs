use axum::http::{header, StatusCode};
use axum_test::TestServer;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};

static DB_COUNTER: AtomicU64 = AtomicU64::new(100); // offset to avoid collisions with auth_tests

async fn setup() -> (TestServer, String) {
    let db_id = DB_COUNTER.fetch_add(1, Ordering::Relaxed);
    let db_name = format!("apitest_{}", db_id);

    let pool =
        server::db::in_memory_pool_named(&db_name).expect("Failed to create in-memory pool");
    server::db::run_migrations(&pool)
        .await
        .expect("Migration failed");

    let config = server::config::Config {
        host: "127.0.0.1".to_string(),
        port: 8080,
        database_url: format!("file:{}?mode=memory&cache=shared", db_name),
        jwt_secret: "api-test-secret".to_string(),
        jwt_access_expiry_secs: 1800,
        jwt_refresh_expiry_secs: 604800,
    };

    let router = server::app::build_router(pool, config);
    let server = TestServer::new(router).expect("Failed to create test server");

    // Register + login, get access token
    server
        .post("/api/auth/register")
        .json(&json!({ "username": "apiuser", "email": "api@test.com", "password": "password123" }))
        .await;
    let login_resp = server
        .post("/api/auth/login")
        .json(&json!({ "username": "apiuser", "password": "password123" }))
        .await;
    let token = login_resp.json::<Value>()["access_token"]
        .as_str()
        .unwrap()
        .to_string();

    (server, token)
}

fn auth_header(token: &str) -> (header::HeaderName, header::HeaderValue) {
    (
        header::AUTHORIZATION,
        format!("Bearer {}", token).parse().unwrap(),
    )
}

// ─── Project CRUD ────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_project_crud() {
    let (server, token) = setup().await;
    let auth = auth_header(&token);

    // Create
    let resp = server
        .post("/api/projects")
        .add_header(auth.0.clone(), auth.1.clone())
        .json(&json!({ "name": "My Project", "source_lang": "en", "target_lang": "ko" }))
        .await;
    resp.assert_status_ok();
    let project: Value = resp.json();
    assert_eq!(project["name"], "My Project");
    let id = project["id"].as_str().unwrap().to_string();

    // List
    let resp = server
        .get("/api/projects")
        .add_header(auth.0.clone(), auth.1.clone())
        .await;
    resp.assert_status_ok();
    let list: Value = resp.json();
    assert_eq!(list.as_array().unwrap().len(), 1);

    // Get
    let resp = server
        .get(&format!("/api/projects/{}", id))
        .add_header(auth.0.clone(), auth.1.clone())
        .await;
    resp.assert_status_ok();
    assert_eq!(resp.json::<Value>()["source_lang"], "en");

    // Update
    let resp = server
        .patch(&format!("/api/projects/{}", id))
        .add_header(auth.0.clone(), auth.1.clone())
        .json(&json!({ "name": "Updated Name" }))
        .await;
    resp.assert_status_ok();
    assert_eq!(resp.json::<Value>()["name"], "Updated Name");

    // Delete
    let resp = server
        .delete(&format!("/api/projects/{}", id))
        .add_header(auth.0.clone(), auth.1.clone())
        .await;
    resp.assert_status_ok();

    // Confirm deleted
    server
        .get(&format!("/api/projects/{}", id))
        .add_header(auth.0.clone(), auth.1.clone())
        .await
        .assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_project_requires_auth() {
    let (server, _) = setup().await;
    server
        .get("/api/projects")
        .await
        .assert_status(StatusCode::UNAUTHORIZED);
}

// ─── TM CRUD ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_tm_crud_and_search() {
    let (server, token) = setup().await;
    let auth = auth_header(&token);

    // Create
    let resp = server
        .post("/api/tm")
        .add_header(auth.0.clone(), auth.1.clone())
        .json(&json!({ "source": "Hello world", "target": "안녕 세계", "source_lang": "en", "target_lang": "ko" }))
        .await;
    resp.assert_status_ok();
    let entry: Value = resp.json();
    let id = entry["id"].as_str().unwrap().to_string();

    // List
    let resp = server
        .get("/api/tm")
        .add_header(auth.0.clone(), auth.1.clone())
        .await;
    resp.assert_status_ok();
    assert_eq!(resp.json::<Value>().as_array().unwrap().len(), 1);

    // Search exact match
    let resp = server
        .get("/api/tm")
        .add_header(auth.0.clone(), auth.1.clone())
        .add_query_param("source", "Hello world")
        .add_query_param("source_lang", "en")
        .add_query_param("target_lang", "ko")
        .await;
    resp.assert_status_ok();
    let results: Value = resp.json();
    let arr = results.as_array().unwrap();
    assert!(!arr.is_empty());
    assert_eq!(arr[0]["score"], 1.0);

    // Fuzzy search ("Hello wor" → "Hello world": edit dist 2, score ≈ 0.82 > 0.5)
    let resp = server
        .get("/api/tm")
        .add_header(auth.0.clone(), auth.1.clone())
        .add_query_param("source", "Hello wor")
        .add_query_param("source_lang", "en")
        .add_query_param("target_lang", "ko")
        .await;
    resp.assert_status_ok();
    let results: Value = resp.json();
    assert!(!results.as_array().unwrap().is_empty());

    // Delete
    server
        .delete(&format!("/api/tm/{}", id))
        .add_header(auth.0.clone(), auth.1.clone())
        .await
        .assert_status_ok();

    // Confirm empty
    let list: Value = server
        .get("/api/tm")
        .add_header(auth.0.clone(), auth.1.clone())
        .await
        .json();
    assert_eq!(list.as_array().unwrap().len(), 0);
}

// ─── TB CRUD ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_tb_crud() {
    let (server, token) = setup().await;
    let auth = auth_header(&token);

    // Create
    let resp = server
        .post("/api/tb")
        .add_header(auth.0.clone(), auth.1.clone())
        .json(&json!({
            "source_term": "cat",
            "target_term": "고양이",
            "source_lang": "en",
            "target_lang": "ko",
            "notes": "animal",
            "forbidden": false
        }))
        .await;
    resp.assert_status_ok();
    let entry: Value = resp.json();
    let id = entry["id"].as_str().unwrap().to_string();
    assert_eq!(entry["source_term"], "cat");

    // List
    let list: Value = server
        .get("/api/tb")
        .add_header(auth.0.clone(), auth.1.clone())
        .await
        .json();
    assert_eq!(list.as_array().unwrap().len(), 1);

    // Update
    let resp = server
        .patch(&format!("/api/tb/{}", id))
        .add_header(auth.0.clone(), auth.1.clone())
        .json(&json!({ "notes": "small animal", "forbidden": true }))
        .await;
    resp.assert_status_ok();
    let updated: Value = resp.json();
    assert_eq!(updated["notes"], "small animal");
    assert_eq!(updated["forbidden"], true);

    // Delete
    server
        .delete(&format!("/api/tb/{}", id))
        .add_header(auth.0.clone(), auth.1.clone())
        .await
        .assert_status_ok();

    let list: Value = server
        .get("/api/tb")
        .add_header(auth.0.clone(), auth.1.clone())
        .await
        .json();
    assert_eq!(list.as_array().unwrap().len(), 0);
}

// ─── XLIFF file upload + segments ────────────────────────────────────────────

#[tokio::test]
async fn test_file_upload_and_segments() {
    let (server, token) = setup().await;
    let auth = auth_header(&token);

    // Create project
    let project: Value = server
        .post("/api/projects")
        .add_header(auth.0.clone(), auth.1.clone())
        .json(&json!({ "name": "Test", "source_lang": "en", "target_lang": "ko" }))
        .await
        .json();
    let project_id = project["id"].as_str().unwrap().to_string();

    // Upload XLIFF
    let xliff = r#"<?xml version="1.0"?>
<xliff version="1.2">
  <file>
    <body>
      <trans-unit id="1"><source>Hello</source><target></target></trans-unit>
      <trans-unit id="2"><source>World</source><target>세계</target></trans-unit>
    </body>
  </file>
</xliff>"#;

    let resp = server
        .post(&format!("/api/projects/{}/files", project_id))
        .add_header(auth.0.clone(), auth.1.clone())
        .multipart(
            axum_test::multipart::MultipartForm::new()
                .add_text("filename", "test.xliff")
                .add_part(
                    "file",
                    axum_test::multipart::Part::bytes(xliff.as_bytes().to_vec())
                        .file_name("test.xliff")
                        .mime_type("application/xml"),
                ),
        )
        .await;
    resp.assert_status_ok();

    // List segments
    let segs: Value = server
        .get(&format!("/api/projects/{}/segments", project_id))
        .add_header(auth.0.clone(), auth.1.clone())
        .await
        .json();
    let arr = segs.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["source"], "Hello");
    assert_eq!(arr[0]["status"], "untranslated");
    assert_eq!(arr[1]["source"], "World");
    assert_eq!(arr[1]["status"], "translated");

    // Update segment
    let seg_id = arr[0]["id"].as_str().unwrap().to_string();
    let resp = server
        .patch(&format!("/api/projects/{}/segments/{}", project_id, seg_id))
        .add_header(auth.0.clone(), auth.1.clone())
        .json(&json!({ "target": "안녕", "status": "confirmed" }))
        .await;
    resp.assert_status_ok();
    let updated: Value = resp.json();
    assert_eq!(updated["target"], "안녕");
    assert_eq!(updated["status"], "confirmed");
}

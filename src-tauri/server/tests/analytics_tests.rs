/// Analytics API integration tests.
use std::sync::atomic::{AtomicU64, Ordering};

use axum::http::header;
use axum_test::TestServer;
use serde_json::{json, Value};

static DB_COUNTER: AtomicU64 = AtomicU64::new(500);

// ─── Setup ────────────────────────────────────────────────────────────────────

async fn setup() -> (TestServer, String, String) {
    let db_id = DB_COUNTER.fetch_add(1, Ordering::Relaxed);
    let db_name = format!("analytics_test_{}", db_id);

    let pool = server::db::in_memory_pool_named(&db_name).expect("pool");
    server::db::run_migrations(&pool).await.expect("migrations");

    let config = server::config::Config {
        host: "127.0.0.1".to_string(),
        port: 0,
        database_url: format!("file:{}?mode=memory&cache=shared", db_name),
        jwt_secret: "analytics-secret".to_string(),
        jwt_access_expiry_secs: 1800,
        jwt_refresh_expiry_secs: 604800,
        ws_lock_timeout_secs: 0,
    };

    let router = server::app::build_router(pool, config);
    let srv = TestServer::new(router).expect("test server");

    srv.post("/api/auth/register")
        .json(&json!({"username":"analyst","email":"a@test.com","password":"password123"}))
        .await;

    let resp: Value = srv
        .post("/api/auth/login")
        .json(&json!({"username":"analyst","password":"password123"}))
        .await
        .json();

    let token = resp["access_token"].as_str().unwrap().to_string();

    // Get user ID from /api/auth/me
    let me: Value = srv
        .get("/api/auth/me")
        .add_header(
            header::AUTHORIZATION,
            format!("Bearer {}", token).parse().unwrap(),
        )
        .await
        .json();
    let user_id = me["id"].as_str().unwrap().to_string();

    (srv, token, user_id)
}

fn auth(token: &str) -> (header::HeaderName, header::HeaderValue) {
    (
        header::AUTHORIZATION,
        format!("Bearer {}", token).parse().unwrap(),
    )
}

/// Create a project + upload a minimal XLIFF → returns (project_id, segment_id).
async fn setup_project(srv: &TestServer, token: &str) -> (String, String) {
    let a = auth(token);

    let project: Value = srv
        .post("/api/projects")
        .add_header(a.0.clone(), a.1.clone())
        .json(&json!({"name":"Test","source_lang":"en","target_lang":"ko"}))
        .await
        .json();
    let project_id = project["id"].as_str().unwrap().to_string();

    let xliff = r#"<?xml version="1.0" encoding="UTF-8"?>
<xliff version="1.2">
  <file source-language="en" target-language="ko" original="test.html">
    <body>
      <trans-unit id="1"><source>Hello</source><target></target></trans-unit>
    </body>
  </file>
</xliff>"#;

    srv.post(&format!("/api/projects/{}/files", project_id))
        .add_header(a.0.clone(), a.1.clone())
        .multipart(
            axum_test::multipart::MultipartForm::new().add_part(
                "file",
                axum_test::multipart::Part::bytes(xliff.as_bytes().to_vec())
                    .file_name("test.xliff")
                    .mime_type("application/xml"),
            ),
        )
        .await;

    let segs: Value = srv
        .get(&format!("/api/projects/{}/segments", project_id))
        .add_header(a.0.clone(), a.1.clone())
        .await
        .json();
    let seg_id = segs[0]["id"].as_str().unwrap().to_string();

    (project_id, seg_id)
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_event_logged_on_save() {
    let (srv, token, _uid) = setup().await;
    let (project_id, seg_id) = setup_project(&srv, &token).await;
    let a = auth(&token);

    srv.patch(&format!("/api/projects/{}/segments/{}", project_id, seg_id))
        .add_header(a.0.clone(), a.1.clone())
        .json(&json!({"target":"안녕","status":"translated","mt_used":false,"tm_match_score":75}))
        .await;

    let rows: Value = srv
        .get("/api/analytics/team")
        .add_header(a.0.clone(), a.1.clone())
        .await
        .json();
    let rows = rows.as_array().unwrap();
    assert_eq!(rows.len(), 1, "expected 1 row");
    assert_eq!(rows[0]["segments_saved"], 1);
    assert_eq!(rows[0]["username"], "analyst");
}

#[tokio::test]
async fn test_confirm_action_counted() {
    let (srv, token, _uid) = setup().await;
    let (project_id, seg_id) = setup_project(&srv, &token).await;
    let a = auth(&token);

    srv.patch(&format!("/api/projects/{}/segments/{}", project_id, seg_id))
        .add_header(a.0.clone(), a.1.clone())
        .json(&json!({"target":"안녕","status":"confirmed"}))
        .await;

    let rows: Value = srv
        .get("/api/analytics/team")
        .add_header(a.0.clone(), a.1.clone())
        .await
        .json();
    let r = &rows.as_array().unwrap()[0];
    assert_eq!(r["segments_confirmed"], 1);
    assert_eq!(r["segments_saved"], 0);
}

#[tokio::test]
async fn test_user_analytics_stats() {
    let (srv, token, user_id) = setup().await;
    let (project_id, seg_id) = setup_project(&srv, &token).await;
    let a = auth(&token);

    srv.patch(&format!("/api/projects/{}/segments/{}", project_id, seg_id))
        .add_header(a.0.clone(), a.1.clone())
        .json(&json!({"target":"결과","mt_used":true,"tm_match_score":80}))
        .await;

    let stats: Value = srv
        .get(&format!("/api/analytics/user/{}", user_id))
        .add_header(a.0.clone(), a.1.clone())
        .await
        .json();

    assert_eq!(stats["total_saves"], 1);
    assert_eq!(stats["mt_used_count"], 1);
    assert_eq!(stats["username"], "analyst");
    let avg = stats["tm_match_avg"].as_f64().unwrap();
    assert!((avg - 80.0).abs() < 0.01);
}

#[tokio::test]
async fn test_project_analytics_mt_rate() {
    let (srv, token, _uid) = setup().await;
    let (project_id, seg_id) = setup_project(&srv, &token).await;
    let a = auth(&token);

    // 1 MT, 1 human
    for (target, mt) in [("MT result", true), ("Human result", false)] {
        srv.patch(&format!("/api/projects/{}/segments/{}", project_id, seg_id))
            .add_header(a.0.clone(), a.1.clone())
            .json(&json!({"target": target, "mt_used": mt}))
            .await;
    }

    let stats: Value = srv
        .get(&format!("/api/analytics/project/{}", project_id))
        .add_header(a.0.clone(), a.1.clone())
        .await
        .json();

    assert_eq!(stats["total_events"], 2);
    let rate = stats["mt_usage_rate"].as_f64().unwrap();
    assert!((rate - 0.5).abs() < 0.01);
}

#[tokio::test]
async fn test_date_range_filter() {
    let (srv, token, _uid) = setup().await;
    let (project_id, seg_id) = setup_project(&srv, &token).await;
    let a = auth(&token);

    srv.patch(&format!("/api/projects/{}/segments/{}", project_id, seg_id))
        .add_header(a.0.clone(), a.1.clone())
        .json(&json!({"target":"필터 테스트"}))
        .await;

    // Future range → no rows
    let empty: Value = srv
        .get("/api/analytics/team")
        .add_query_param("from", "2099-01-01")
        .add_query_param("to", "2099-12-31")
        .add_header(a.0.clone(), a.1.clone())
        .await
        .json();
    assert_eq!(empty.as_array().unwrap().len(), 0);

    // Past range covering today → 1 row
    let full: Value = srv
        .get("/api/analytics/team")
        .add_query_param("from", "2020-01-01")
        .add_header(a.0.clone(), a.1.clone())
        .await
        .json();
    assert_eq!(full.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn test_csv_export_team() {
    let (srv, token, _uid) = setup().await;
    let (project_id, seg_id) = setup_project(&srv, &token).await;
    let a = auth(&token);

    srv.patch(&format!("/api/projects/{}/segments/{}", project_id, seg_id))
        .add_header(a.0.clone(), a.1.clone())
        .json(&json!({"target":"CSV 테스트"}))
        .await;

    let resp = srv
        .get("/api/analytics/team")
        .add_query_param("format", "csv")
        .add_header(a.0.clone(), a.1.clone())
        .await;

    let body = resp.text();
    assert!(body.starts_with("date,user_id,username,segments_saved,segments_confirmed"));
    assert!(body.contains("analyst"));
}

#[tokio::test]
async fn test_csv_export_user() {
    let (srv, token, user_id) = setup().await;
    let (project_id, seg_id) = setup_project(&srv, &token).await;
    let a = auth(&token);

    srv.patch(&format!("/api/projects/{}/segments/{}", project_id, seg_id))
        .add_header(a.0.clone(), a.1.clone())
        .json(&json!({"target":"user csv"}))
        .await;

    let resp = srv
        .get(&format!("/api/analytics/user/{}", user_id))
        .add_query_param("format", "csv")
        .add_header(a.0.clone(), a.1.clone())
        .await;

    let body = resp.text();
    assert!(body.starts_with("user_id,username,total_saves"));
    assert!(body.contains("analyst"));
}

#[tokio::test]
async fn test_csv_export_project() {
    let (srv, token, _uid) = setup().await;
    let (project_id, seg_id) = setup_project(&srv, &token).await;
    let a = auth(&token);

    srv.patch(&format!("/api/projects/{}/segments/{}", project_id, seg_id))
        .add_header(a.0.clone(), a.1.clone())
        .json(&json!({"target":"proj csv","mt_used":true}))
        .await;

    let resp = srv
        .get(&format!("/api/analytics/project/{}", project_id))
        .add_query_param("format", "csv")
        .add_header(a.0.clone(), a.1.clone())
        .await;

    let body = resp.text();
    assert!(body.starts_with("project_id,total_events,confirmed_count,mt_usage_rate"));
    assert!(body.contains(&project_id));
}

#[tokio::test]
async fn test_unknown_user_returns_404() {
    let (srv, token, _uid) = setup().await;
    let a = auth(&token);

    let resp = srv
        .get("/api/analytics/user/nonexistent-id")
        .add_header(a.0, a.1)
        .await;

    assert_eq!(resp.status_code(), 404);
}

#[tokio::test]
async fn test_unknown_project_returns_404() {
    let (srv, token, _uid) = setup().await;
    let a = auth(&token);

    let resp = srv
        .get("/api/analytics/project/nonexistent-id")
        .add_header(a.0, a.1)
        .await;

    assert_eq!(resp.status_code(), 404);
}

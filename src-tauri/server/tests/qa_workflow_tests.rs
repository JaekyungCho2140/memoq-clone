/// QA Workflow Validation Tests (AFR-69)
///
/// Validates the 4 core translation scenarios end-to-end via the HTTP API:
///   1. Basic translation workflow (XLIFF import → segment edit → confirm)
///   2. TM round-trip (create TM → search → fuzzy match → delete)
///   3. DOCX round-trip (DOCX is Tauri-side; server validates segment lifecycle)
///   4. Edge cases (empty XLIFF, long segments, special chars, multiple files)
use axum::http::header;
use axum_test::TestServer;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};

static QA_COUNTER: AtomicU64 = AtomicU64::new(9000);

async fn setup() -> (TestServer, String) {
    let db_id = QA_COUNTER.fetch_add(1, Ordering::Relaxed);
    let db_name = format!("qa_workflow_{}", db_id);

    let pool =
        server::db::in_memory_pool_named(&db_name).expect("Failed to create in-memory pool");
    server::db::run_migrations(&pool)
        .await
        .expect("Migration failed");

    let config = server::config::Config {
        host: "127.0.0.1".to_string(),
        port: 8080,
        database_url: format!("file:{}?mode=memory&cache=shared", db_name),
        jwt_secret: "qa-test-secret".to_string(),
        jwt_access_expiry_secs: 1800,
        jwt_refresh_expiry_secs: 604800,
        ws_lock_timeout_secs: 0,
        allowed_origins: vec![],
        auth_rate_limit_per_min: 1000,
    };

    let router = server::app::build_router(pool, config);
    let server = TestServer::new(router).expect("Failed to create test server");

    server
        .post("/api/auth/register")
        .json(&json!({ "username": "qauser", "email": "qa@test.com", "password": "password123" }))
        .await;
    let login = server
        .post("/api/auth/login")
        .json(&json!({ "username": "qauser", "password": "password123" }))
        .await;
    let token = login.json::<Value>()["access_token"]
        .as_str()
        .unwrap()
        .to_string();

    (server, token)
}

fn auth(token: &str) -> (header::HeaderName, header::HeaderValue) {
    (
        header::AUTHORIZATION,
        format!("Bearer {}", token).parse().unwrap(),
    )
}

// ─── Scenario 1: Basic Translation Workflow ──────────────────────────────────
//
// XLIFF import → segment list → edit target → confirm (status=confirmed)
// Validates that the full translator edit cycle works.

#[tokio::test]
async fn qa_scenario1_basic_translation_workflow() {
    let (server, token) = setup().await;
    let a = auth(&token);

    // 1a. Create project
    let project: Value = server
        .post("/api/projects")
        .add_header(a.0.clone(), a.1.clone())
        .json(&json!({ "name": "QA-S1", "source_lang": "en", "target_lang": "ko" }))
        .await
        .json();
    let pid = project["id"].as_str().unwrap().to_string();

    // 1b. Upload XLIFF with 3 segments (untranslated, translated, needs-review)
    let xliff = r#"<?xml version="1.0" encoding="UTF-8"?>
<xliff version="1.2">
  <file source-language="en" target-language="ko">
    <body>
      <trans-unit id="1"><source>Hello</source><target></target></trans-unit>
      <trans-unit id="2"><source>Goodbye</source><target>작별</target></trans-unit>
      <trans-unit id="3"><source>Thank you</source><target>감사합니다</target></trans-unit>
    </body>
  </file>
</xliff>"#;

    server
        .post(&format!("/api/projects/{}/files", pid))
        .add_header(a.0.clone(), a.1.clone())
        .multipart(
            axum_test::multipart::MultipartForm::new()
                .add_text("filename", "scenario1.xliff")
                .add_part(
                    "file",
                    axum_test::multipart::Part::bytes(xliff.as_bytes().to_vec())
                        .file_name("scenario1.xliff")
                        .mime_type("application/xml"),
                ),
        )
        .await
        .assert_status_ok();

    // 1c. List segments — verify correct parse
    let segs: Value = server
        .get(&format!("/api/projects/{}/segments", pid))
        .add_header(a.0.clone(), a.1.clone())
        .await
        .json();
    let arr = segs.as_array().unwrap();
    assert_eq!(arr.len(), 3, "should have 3 segments");
    assert_eq!(arr[0]["source"], "Hello");
    assert_eq!(arr[0]["status"], "untranslated");
    assert_eq!(arr[1]["source"], "Goodbye");
    assert_eq!(arr[1]["status"], "translated");
    assert_eq!(arr[2]["source"], "Thank you");
    assert_eq!(arr[2]["status"], "translated");

    // 1d. Edit first segment (simulate translator typing)
    let seg_id = arr[0]["id"].as_str().unwrap().to_string();
    let resp = server
        .patch(&format!("/api/projects/{}/segments/{}", pid, seg_id))
        .add_header(a.0.clone(), a.1.clone())
        .json(&json!({ "target": "안녕", "status": "translated" }))
        .await;
    resp.assert_status_ok();
    let updated: Value = resp.json();
    assert_eq!(updated["target"], "안녕");
    assert_eq!(updated["status"], "translated");

    // 1e. Confirm segment (simulate Ctrl+Enter)
    let resp = server
        .patch(&format!("/api/projects/{}/segments/{}", pid, seg_id))
        .add_header(a.0.clone(), a.1.clone())
        .json(&json!({ "status": "confirmed" }))
        .await;
    resp.assert_status_ok();
    assert_eq!(resp.json::<Value>()["status"], "confirmed");
    assert_eq!(resp.json::<Value>()["target"], "안녕", "target preserved after confirm");
}

// ─── Scenario 2: TM Round-Trip ───────────────────────────────────────────────
//
// Create TM entry → search exact → search fuzzy → TM drives segment → delete

#[tokio::test]
async fn qa_scenario2_tm_round_trip() {
    let (server, token) = setup().await;
    let a = auth(&token);

    // 2a. Create TM entry (simulating a confirmed translation added to TM)
    let entry: Value = server
        .post("/api/tm")
        .add_header(a.0.clone(), a.1.clone())
        .json(&json!({
            "source": "The quick brown fox",
            "target": "빠른 갈색 여우",
            "source_lang": "en",
            "target_lang": "ko"
        }))
        .await
        .json();
    let tm_id = entry["id"].as_str().unwrap().to_string();
    assert_eq!(entry["source"], "The quick brown fox");

    // 2b. Exact match search (score = 1.0)
    let results: Value = server
        .get("/api/tm")
        .add_header(a.0.clone(), a.1.clone())
        .add_query_param("source", "The quick brown fox")
        .add_query_param("source_lang", "en")
        .add_query_param("target_lang", "ko")
        .await
        .json();
    let arr = results.as_array().unwrap();
    assert!(!arr.is_empty(), "exact match should return results");
    assert_eq!(arr[0]["score"], 1.0, "exact match score must be 1.0");
    assert_eq!(arr[0]["source"], "The quick brown fox", "source must be in flattened result");
    assert_eq!(arr[0]["target"], "빠른 갈색 여우", "target must be in flattened result");

    // 2c. Fuzzy match search (partial text → score > 0.5)
    let fuzzy: Value = server
        .get("/api/tm")
        .add_header(a.0.clone(), a.1.clone())
        .add_query_param("source", "The quick brown fo")
        .add_query_param("source_lang", "en")
        .add_query_param("target_lang", "ko")
        .await
        .json();
    assert!(
        !fuzzy.as_array().unwrap().is_empty(),
        "fuzzy match should return results"
    );
    let score = fuzzy.as_array().unwrap()[0]["score"]
        .as_f64()
        .unwrap_or(0.0);
    assert!(score > 0.5, "fuzzy score should be >0.5, got {}", score);
    assert!(score < 1.0, "fuzzy score should be <1.0 for partial match");

    // 2d. No match for completely different text
    let no_match: Value = server
        .get("/api/tm")
        .add_header(a.0.clone(), a.1.clone())
        .add_query_param("source", "completely unrelated xyz")
        .add_query_param("source_lang", "en")
        .add_query_param("target_lang", "ko")
        .await
        .json();
    assert!(
        no_match.as_array().unwrap().is_empty(),
        "completely different text should return no TM matches"
    );

    // 2e. Create project + upload XLIFF → apply TM match to segment
    let project: Value = server
        .post("/api/projects")
        .add_header(a.0.clone(), a.1.clone())
        .json(&json!({ "name": "QA-S2-TM", "source_lang": "en", "target_lang": "ko" }))
        .await
        .json();
    let pid = project["id"].as_str().unwrap().to_string();

    let xliff = r#"<?xml version="1.0"?>
<xliff version="1.2">
  <file><body>
    <trans-unit id="1"><source>The quick brown fox</source><target></target></trans-unit>
  </body></file>
</xliff>"#;
    server
        .post(&format!("/api/projects/{}/files", pid))
        .add_header(a.0.clone(), a.1.clone())
        .multipart(
            axum_test::multipart::MultipartForm::new()
                .add_text("filename", "tm_test.xliff")
                .add_part(
                    "file",
                    axum_test::multipart::Part::bytes(xliff.as_bytes().to_vec())
                        .file_name("tm_test.xliff")
                        .mime_type("application/xml"),
                ),
        )
        .await
        .assert_status_ok();

    let segs: Value = server
        .get(&format!("/api/projects/{}/segments", pid))
        .add_header(a.0.clone(), a.1.clone())
        .await
        .json();
    let seg_id = segs[0]["id"].as_str().unwrap().to_string();

    // Apply TM match to segment (frontend would do this, we test the save path)
    let resp = server
        .patch(&format!("/api/projects/{}/segments/{}", pid, seg_id))
        .add_header(a.0.clone(), a.1.clone())
        .json(&json!({
            "target": "빠른 갈색 여우",
            "status": "translated",
            "tm_match_score": 100
        }))
        .await;
    resp.assert_status_ok();
    let applied: Value = resp.json();
    assert_eq!(applied["target"], "빠른 갈색 여우");

    // 2f. Cleanup TM entry
    server
        .delete(&format!("/api/tm/{}", tm_id))
        .add_header(a.0.clone(), a.1.clone())
        .await
        .assert_status_ok();
    let list: Value = server
        .get("/api/tm")
        .add_header(a.0.clone(), a.1.clone())
        .await
        .json();
    assert_eq!(list.as_array().unwrap().len(), 0, "TM should be empty after delete");
}

// ─── Scenario 3: DOCX Round-Trip (server-side lifecycle) ─────────────────────
//
// DOCX parse/export is a Tauri command (native side).  This test validates the
// *server-side* portion: that segments from a DOCX-originated project survive
// the same edit/confirm lifecycle as XLIFF segments.

#[tokio::test]
async fn qa_scenario3_docx_segment_lifecycle() {
    let (server, token) = setup().await;
    let a = auth(&token);

    // Create project (simulating a DOCX import that already parsed on Tauri side)
    let project: Value = server
        .post("/api/projects")
        .add_header(a.0.clone(), a.1.clone())
        .json(&json!({ "name": "QA-S3-DOCX", "source_lang": "en", "target_lang": "ko" }))
        .await
        .json();
    let pid = project["id"].as_str().unwrap().to_string();

    // Upload XLIFF to populate segments (simulating what Tauri DOCX import does)
    let xliff = r#"<?xml version="1.0"?>
<xliff version="1.2">
  <file><body>
    <trans-unit id="1"><source>Introduction chapter</source><target></target></trans-unit>
    <trans-unit id="2"><source>This document describes the system.</source><target></target></trans-unit>
    <trans-unit id="3"><source>Conclusion</source><target></target></trans-unit>
  </body></file>
</xliff>"#;
    server
        .post(&format!("/api/projects/{}/files", pid))
        .add_header(a.0.clone(), a.1.clone())
        .multipart(
            axum_test::multipart::MultipartForm::new()
                .add_text("filename", "docx_sim.xliff")
                .add_part(
                    "file",
                    axum_test::multipart::Part::bytes(xliff.as_bytes().to_vec())
                        .file_name("docx_sim.xliff")
                        .mime_type("application/xml"),
                ),
        )
        .await
        .assert_status_ok();

    let segs: Value = server
        .get(&format!("/api/projects/{}/segments", pid))
        .add_header(a.0.clone(), a.1.clone())
        .await
        .json();
    let arr = segs.as_array().unwrap();
    assert_eq!(arr.len(), 3);

    // Translate all segments
    let translations = ["소개 챕터", "이 문서는 시스템을 설명합니다.", "결론"];
    for (i, seg) in arr.iter().enumerate() {
        let seg_id = seg["id"].as_str().unwrap();
        let resp = server
            .patch(&format!("/api/projects/{}/segments/{}", pid, seg_id))
            .add_header(a.0.clone(), a.1.clone())
            .json(&json!({ "target": translations[i], "status": "confirmed" }))
            .await;
        resp.assert_status_ok();
        let updated: Value = resp.json();
        assert_eq!(updated["target"], translations[i]);
        assert_eq!(updated["status"], "confirmed");
    }

    // Verify all segments are confirmed
    let final_segs: Value = server
        .get(&format!("/api/projects/{}/segments", pid))
        .add_header(a.0.clone(), a.1.clone())
        .await
        .json();
    let confirmed_count = final_segs
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["status"] == "confirmed")
        .count();
    assert_eq!(confirmed_count, 3, "all 3 segments should be confirmed before DOCX export");
}

// ─── Scenario 4: Edge Cases ───────────────────────────────────────────────────

#[tokio::test]
async fn qa_scenario4_empty_xliff() {
    let (server, token) = setup().await;
    let a = auth(&token);

    let project: Value = server
        .post("/api/projects")
        .add_header(a.0.clone(), a.1.clone())
        .json(&json!({ "name": "QA-S4-Empty", "source_lang": "en", "target_lang": "ko" }))
        .await
        .json();
    let pid = project["id"].as_str().unwrap().to_string();

    // Empty XLIFF (no trans-units)
    let xliff = r#"<?xml version="1.0"?>
<xliff version="1.2">
  <file><body></body></file>
</xliff>"#;

    let resp = server
        .post(&format!("/api/projects/{}/files", pid))
        .add_header(a.0.clone(), a.1.clone())
        .multipart(
            axum_test::multipart::MultipartForm::new()
                .add_text("filename", "empty.xliff")
                .add_part(
                    "file",
                    axum_test::multipart::Part::bytes(xliff.as_bytes().to_vec())
                        .file_name("empty.xliff")
                        .mime_type("application/xml"),
                ),
        )
        .await;
    resp.assert_status_ok();

    let segs: Value = server
        .get(&format!("/api/projects/{}/segments", pid))
        .add_header(a.0.clone(), a.1.clone())
        .await
        .json();
    assert_eq!(
        segs.as_array().unwrap().len(),
        0,
        "empty XLIFF should produce 0 segments"
    );
}

#[tokio::test]
async fn qa_scenario4_long_segment() {
    let (server, token) = setup().await;
    let a = auth(&token);

    let project: Value = server
        .post("/api/projects")
        .add_header(a.0.clone(), a.1.clone())
        .json(&json!({ "name": "QA-S4-Long", "source_lang": "en", "target_lang": "ko" }))
        .await
        .json();
    let pid = project["id"].as_str().unwrap().to_string();

    // Very long segment (5000 chars)
    let long_source = "A".repeat(5000);
    let long_target = "가".repeat(5000);
    let xliff = format!(
        r#"<?xml version="1.0"?>
<xliff version="1.2">
  <file><body>
    <trans-unit id="1"><source>{}</source><target></target></trans-unit>
  </body></file>
</xliff>"#,
        long_source
    );

    server
        .post(&format!("/api/projects/{}/files", pid))
        .add_header(a.0.clone(), a.1.clone())
        .multipart(
            axum_test::multipart::MultipartForm::new()
                .add_text("filename", "long.xliff")
                .add_part(
                    "file",
                    axum_test::multipart::Part::bytes(xliff.as_bytes().to_vec())
                        .file_name("long.xliff")
                        .mime_type("application/xml"),
                ),
        )
        .await
        .assert_status_ok();

    let segs: Value = server
        .get(&format!("/api/projects/{}/segments", pid))
        .add_header(a.0.clone(), a.1.clone())
        .await
        .json();
    let seg_id = segs[0]["id"].as_str().unwrap().to_string();
    assert_eq!(segs[0]["source"].as_str().unwrap().chars().count(), 5000);

    // Edit with long target
    let resp = server
        .patch(&format!("/api/projects/{}/segments/{}", pid, seg_id))
        .add_header(a.0.clone(), a.1.clone())
        .json(&json!({ "target": long_target, "status": "confirmed" }))
        .await;
    resp.assert_status_ok();
    let updated: Value = resp.json();
    assert_eq!(updated["target"].as_str().unwrap().chars().count(), 5000);
    assert_eq!(updated["status"], "confirmed");
}

#[tokio::test]
async fn qa_scenario4_special_characters() {
    let (server, token) = setup().await;
    let a = auth(&token);

    let project: Value = server
        .post("/api/projects")
        .add_header(a.0.clone(), a.1.clone())
        .json(&json!({ "name": "QA-S4-Special", "source_lang": "en", "target_lang": "ko" }))
        .await
        .json();
    let pid = project["id"].as_str().unwrap().to_string();

    // Segments with special characters: XML entities, emoji, Korean, CJK, Arabic
    let xliff = r#"<?xml version="1.0" encoding="UTF-8"?>
<xliff version="1.2">
  <file><body>
    <trans-unit id="1"><source>Price: &lt;$10 &amp; &gt;$5</source><target></target></trans-unit>
    <trans-unit id="2"><source>Emoji: 🚀🌟💻</source><target></target></trans-unit>
    <trans-unit id="3"><source>Mixed: Hello 世界 مرحبا</source><target></target></trans-unit>
    <trans-unit id="4"><source>Newline&#xA;preserved</source><target></target></trans-unit>
  </body></file>
</xliff>"#;

    server
        .post(&format!("/api/projects/{}/files", pid))
        .add_header(a.0.clone(), a.1.clone())
        .multipart(
            axum_test::multipart::MultipartForm::new()
                .add_text("filename", "special.xliff")
                .add_part(
                    "file",
                    axum_test::multipart::Part::bytes(xliff.as_bytes().to_vec())
                        .file_name("special.xliff")
                        .mime_type("application/xml"),
                ),
        )
        .await
        .assert_status_ok();

    let segs: Value = server
        .get(&format!("/api/projects/{}/segments", pid))
        .add_header(a.0.clone(), a.1.clone())
        .await
        .json();
    let arr = segs.as_array().unwrap();
    assert_eq!(arr.len(), 4, "should have 4 segments with special chars");

    // Verify XML entities decoded correctly
    assert!(
        arr[0]["source"].as_str().unwrap().contains('<'),
        "XML entity &lt; should be decoded"
    );
    assert!(
        arr[0]["source"].as_str().unwrap().contains('&'),
        "XML entity &amp; should be decoded"
    );

    // Edit all with Korean translations
    let translations = [
        "가격: $5~$10",
        "이모지: 🚀🌟💻",
        "혼합: 안녕 世界 مرحبا",
        "줄바꿈\n유지",
    ];
    for (i, seg) in arr.iter().enumerate() {
        let seg_id = seg["id"].as_str().unwrap();
        let resp = server
            .patch(&format!("/api/projects/{}/segments/{}", pid, seg_id))
            .add_header(a.0.clone(), a.1.clone())
            .json(&json!({ "target": translations[i], "status": "confirmed" }))
            .await;
        resp.assert_status_ok();
        assert_eq!(resp.json::<Value>()["status"], "confirmed");
    }
}

#[tokio::test]
async fn qa_scenario4_multiple_files_in_project() {
    let (server, token) = setup().await;
    let a = auth(&token);

    let project: Value = server
        .post("/api/projects")
        .add_header(a.0.clone(), a.1.clone())
        .json(&json!({ "name": "QA-S4-MultiFile", "source_lang": "en", "target_lang": "ko" }))
        .await
        .json();
    let pid = project["id"].as_str().unwrap().to_string();

    // Upload two separate XLIFF files to the same project
    for i in 1..=2 {
        let xliff = format!(
            r#"<?xml version="1.0"?>
<xliff version="1.2">
  <file><body>
    <trans-unit id="1"><source>File {} segment 1</source><target></target></trans-unit>
    <trans-unit id="2"><source>File {} segment 2</source><target></target></trans-unit>
  </body></file>
</xliff>"#,
            i, i
        );
        server
            .post(&format!("/api/projects/{}/files", pid))
            .add_header(a.0.clone(), a.1.clone())
            .multipart(
                axum_test::multipart::MultipartForm::new()
                    .add_text("filename", format!("file{}.xliff", i))
                    .add_part(
                        "file",
                        axum_test::multipart::Part::bytes(xliff.as_bytes().to_vec())
                            .file_name(format!("file{}.xliff", i))
                            .mime_type("application/xml"),
                    ),
            )
            .await
            .assert_status_ok();
    }

    // Both files' segments should be listed together
    let segs: Value = server
        .get(&format!("/api/projects/{}/segments", pid))
        .add_header(a.0.clone(), a.1.clone())
        .await
        .json();
    assert_eq!(
        segs.as_array().unwrap().len(),
        4,
        "2 files × 2 segments = 4 total segments"
    );

    // All segments should be independently editable
    for seg in segs.as_array().unwrap() {
        let seg_id = seg["id"].as_str().unwrap();
        let resp = server
            .patch(&format!("/api/projects/{}/segments/{}", pid, seg_id))
            .add_header(a.0.clone(), a.1.clone())
            .json(&json!({ "target": "번역됨", "status": "confirmed" }))
            .await;
        resp.assert_status_ok();
    }

    // Verify all confirmed
    let final_segs: Value = server
        .get(&format!("/api/projects/{}/segments", pid))
        .add_header(a.0.clone(), a.1.clone())
        .await
        .json();
    let all_confirmed = final_segs
        .as_array()
        .unwrap()
        .iter()
        .all(|s| s["status"] == "confirmed");
    assert!(all_confirmed, "all segments across multiple files should be confirmable");
}

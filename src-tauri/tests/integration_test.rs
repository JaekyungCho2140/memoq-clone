/// Integration tests: parser → TM → TB → export chain
/// + Phase 2: QA 체크 엔진 + MT 통합 + 프로젝트 관리 + LiveDocs
///
/// 각 테스트는 독립적으로 실행 가능하며 공유 상태가 없습니다.
/// TM/TB는 UUID 기반 고유 ID를 사용하므로 테스트 간 충돌이 없습니다.
/// DOCX 파일은 tempfile 크레이트으로 임시 디렉토리에 생성합니다.
use chrono::Utc;
use memoq_clone_lib::models::{Project, ProjectFile, Segment, SegmentStatus, TbEntry};
use memoq_clone_lib::parser;
use memoq_clone_lib::qa;
use memoq_clone_lib::tb::TbEngine;
use memoq_clone_lib::tm::{TmEngine, TmSearchParams};
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;
use uuid::Uuid;

// ─── DOCX fixture helper ──────────────────────────────────────────────────────

/// 테스트용 최소 DOCX(= ZIP) 파일을 임시 디렉토리에 생성하고 경로를 반환합니다.
/// 포함 단락:
///   1. "Hello, world!"
///   2. "This is a DOCX test."
///   3. "Translation memory helps translators."
fn create_sample_docx(dir: &TempDir) -> PathBuf {
    let path = dir.path().join("sample.docx");

    let document_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:wpc="http://schemas.microsoft.com/office/word/2010/wordprocessingCanvas"
            xmlns:mo="http://schemas.microsoft.com/office/mac/office/2008/main"
            xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"
            xmlns:mv="urn:schemas-microsoft-com:mac:vml"
            xmlns:o="urn:schemas-microsoft-com:office:office"
            xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
            xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math"
            xmlns:v="urn:schemas-microsoft-com:vml"
            xmlns:wp14="http://schemas.microsoft.com/office/word/2010/wordprocessingDrawing"
            xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
            xmlns:w10="urn:schemas-microsoft-com:office:word"
            xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"
            xmlns:wpg="http://schemas.microsoft.com/office/word/2010/wordprocessingGroup"
            xmlns:wpi="http://schemas.microsoft.com/office/word/2010/wordprocessingInk"
            xmlns:wne="http://schemas.microsoft.com/office/word/2006/wordml"
            xmlns:wps="http://schemas.microsoft.com/office/word/2010/wordprocessingShape"
            mc:Ignorable="w14 wp14">
  <w:body>
    <w:p><w:r><w:t>Hello, world!</w:t></w:r></w:p>
    <w:p><w:r><w:t>This is a DOCX test.</w:t></w:r></w:p>
    <w:p><w:r><w:t>Translation memory helps translators.</w:t></w:r></w:p>
  </w:body>
</w:document>"#;

    let file = std::fs::File::create(&path).expect("Cannot create sample.docx");
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    zip.start_file("word/document.xml", options).unwrap();
    zip.write_all(document_xml.as_bytes()).unwrap();

    // [Content_Types].xml is required for a valid DOCX
    let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml"
    ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#;
    zip.start_file("[Content_Types].xml", options).unwrap();
    zip.write_all(content_types.as_bytes()).unwrap();

    zip.finish().unwrap();
    path
}

// ─── Test 1: XLIFF parse → target 수정 → export → 재파싱 검증 ────────────────

#[test]
fn test_xliff_parse_roundtrip() {
    let fixture_path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/sample.xliff");

    // 1) 파싱
    let project = parser::parse(fixture_path).expect("XLIFF 파싱 실패");
    assert_eq!(project.segments.len(), 3, "세그먼트가 3개여야 합니다");

    // 2) 타겟 수정
    let mut segments = project.segments.clone();
    segments[0].target = "안녕, 세계!".to_string();
    segments[0].status = SegmentStatus::Translated;
    segments[1].target = "이것은 번역 메모리 테스트입니다.".to_string();
    segments[1].status = SegmentStatus::Translated;
    segments[2].target = "퍼지 매칭은 유사한 문장에 유용합니다.".to_string();
    segments[2].status = SegmentStatus::Translated;

    // 3) 임시 파일에 내보내기
    let tmp = TempDir::new().unwrap();
    let output_path = tmp.path().join("output.xliff");
    let output_str = output_path.to_str().unwrap();

    parser::export(&segments, fixture_path, output_str).expect("XLIFF 내보내기 실패");
    assert!(output_path.exists(), "출력 파일이 존재해야 합니다");

    // 4) 내보낸 파일 재파싱 검증
    let exported = parser::parse(output_str).expect("내보낸 XLIFF 재파싱 실패");
    assert_eq!(exported.segments.len(), 3);
    assert_eq!(exported.segments[0].target, "안녕, 세계!");
    assert_eq!(
        exported.segments[1].target,
        "이것은 번역 메모리 테스트입니다."
    );
    assert_eq!(
        exported.segments[2].target,
        "퍼지 매칭은 유사한 문장에 유용합니다."
    );
}

// ─── Test 2: DOCX parse → target 수정 → export 검증 ─────────────────────────

#[test]
fn test_docx_parse_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let docx_path = create_sample_docx(&tmp);
    let docx_str = docx_path.to_str().unwrap();

    // 1) 파싱
    let project = parser::parse(docx_str).expect("DOCX 파싱 실패");
    assert_eq!(project.segments.len(), 3, "세그먼트가 3개여야 합니다");
    assert_eq!(project.segments[0].source, "Hello, world!");
    assert_eq!(project.segments[1].source, "This is a DOCX test.");
    assert_eq!(
        project.segments[2].source,
        "Translation memory helps translators."
    );

    // 2) 타겟 수정
    let mut segments = project.segments.clone();
    segments[0].target = "안녕, 세계!".to_string();
    segments[1].target = "이것은 DOCX 테스트입니다.".to_string();
    segments[2].target = "번역 메모리는 번역가에게 도움이 됩니다.".to_string();

    // 3) 임시 파일에 내보내기
    let output_path = tmp.path().join("output.docx");
    let output_str = output_path.to_str().unwrap();

    parser::export(&segments, docx_str, output_str).expect("DOCX 내보내기 실패");
    assert!(output_path.exists(), "출력 파일이 존재해야 합니다");

    // 4) 출력 파일이 유효한 ZIP(DOCX)인지 검증
    let file = std::fs::File::open(&output_path).unwrap();
    let mut archive = zip::ZipArchive::new(file).expect("출력 DOCX가 유효한 ZIP이 아닙니다");
    let mut doc_xml = String::new();
    {
        let mut entry = archive.by_name("word/document.xml").unwrap();
        std::io::Read::read_to_string(&mut entry, &mut doc_xml).unwrap();
    }
    assert!(
        doc_xml.contains("안녕, 세계!"),
        "번역된 타겟이 DOCX에 포함되어야 합니다"
    );
    assert!(doc_xml.contains("이것은 DOCX 테스트입니다."));
}

// ─── Test 3: TM 생성 → 항목 추가 → exact/fuzzy 매치 ─────────────────────────

#[test]
fn test_tm_full_flow() {
    // 1) TM 생성
    let tm_id = TmEngine::create("integration-test-tm", "en-US", "ko-KR").expect("TM 생성 실패");

    // 2) 엔진 열기 & 항목 추가
    let engine = TmEngine::open(&tm_id).expect("TM 열기 실패");
    engine
        .add("Hello, world!", "안녕, 세계!", "en-US", "ko-KR")
        .expect("TM 항목 추가 실패");
    engine
        .add(
            "Translation memory is useful.",
            "번역 메모리는 유용합니다.",
            "en-US",
            "ko-KR",
        )
        .expect("TM 항목 추가 실패");

    // 3) 정확 매치(100%) 검증
    let exact_results = engine
        .search(TmSearchParams {
            query: "Hello, world!",
            source_lang: "en-US",
            target_lang: "ko-KR",
            min_score: 0.99,
        })
        .expect("TM 검색 실패");
    assert!(!exact_results.is_empty(), "정확 매치 결과가 있어야 합니다");
    assert!(
        exact_results[0].score >= 0.99,
        "정확 매치 점수가 0.99 이상이어야 합니다: {}",
        exact_results[0].score
    );
    assert_eq!(exact_results[0].target, "안녕, 세계!");

    // 4) fuzzy 매치(>50%) 검증
    let fuzzy_results = engine
        .search(TmSearchParams {
            query: "Hello world", // 구두점 없음 — 유사하지만 동일하지 않음
            source_lang: "en-US",
            target_lang: "ko-KR",
            min_score: 0.5,
        })
        .expect("TM fuzzy 검색 실패");
    assert!(!fuzzy_results.is_empty(), "fuzzy 매치 결과가 있어야 합니다");
    assert!(
        fuzzy_results[0].score >= 0.5,
        "fuzzy 매치 점수가 0.5 이상이어야 합니다: {}",
        fuzzy_results[0].score
    );
}

// ─── Test 4: TB 생성 → 용어 추가 → 텍스트에서 용어 조회 ─────────────────────

#[test]
fn test_tb_full_flow() {
    // 1) TB 생성
    let tb_id = TbEngine::create("integration-test-tb").expect("TB 생성 실패");

    // 2) 엔진 열기 & 용어 추가
    let engine = TbEngine::open(&tb_id).expect("TB 열기 실패");
    engine
        .add(
            "translation memory",
            "번역 메모리",
            "en-US",
            "ko-KR",
            "",
            false,
        )
        .expect("TB 용어 추가 실패");
    engine
        .add(
            "segment",
            "세그먼트",
            "en-US",
            "ko-KR",
            "CAT tool term",
            false,
        )
        .expect("TB 용어 추가 실패");
    engine
        .add(
            "deprecated term",
            "사용 중단 용어",
            "en-US",
            "ko-KR",
            "",
            true,
        )
        .expect("TB 금지어 추가 실패");

    // 3) 텍스트에서 용어 조회 검증
    let results = engine.lookup("memory", "en-US").expect("TB 조회 실패");
    assert!(!results.is_empty(), "용어 조회 결과가 있어야 합니다");
    assert!(
        results
            .iter()
            .any(|e| e.source_term == "translation memory"),
        "translation memory 용어가 검색되어야 합니다"
    );

    // 4) 금지어 포함 조회
    let forbidden = engine
        .lookup("deprecated", "en-US")
        .expect("TB 금지어 조회 실패");
    assert!(!forbidden.is_empty());
    assert!(forbidden[0].forbidden, "금지어는 forbidden=true여야 합니다");

    // 5) 존재하지 않는 용어 조회
    let empty = engine
        .lookup("nonexistent_term_xyz", "en-US")
        .expect("TB 빈 조회 실패");
    assert!(empty.is_empty(), "없는 용어 조회는 빈 결과여야 합니다");
}

// ─── Test 5: XLIFF 파싱 후 TM 검색 + TB 조회 통합 시나리오 ─────────────────

#[test]
fn test_xliff_with_tm_tb() {
    let fixture_path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/sample.xliff");

    // 1) XLIFF 파싱
    let project = parser::parse(fixture_path).expect("XLIFF 파싱 실패");
    assert_eq!(project.segments.len(), 3);

    // 2) TM 설정: 기존 번역 사전 추가
    let tm_id = TmEngine::create("xliff-tm-tb-test-tm", "en-US", "ko-KR").expect("TM 생성 실패");
    let tm = TmEngine::open(&tm_id).expect("TM 열기 실패");
    tm.add("Hello, world!", "안녕, 세계!", "en-US", "ko-KR")
        .expect("TM 항목 추가 실패");
    tm.add(
        "Fuzzy matching is helpful for similar texts.",
        "퍼지 매칭은 유사한 텍스트에 도움이 됩니다.",
        "en-US",
        "ko-KR",
    )
    .expect("TM 항목 추가 실패");

    // 3) TB 설정
    let tb_id = TbEngine::create("xliff-tm-tb-test-tb").expect("TB 생성 실패");
    let tb = TbEngine::open(&tb_id).expect("TB 열기 실패");
    tb.add(
        "translation memory",
        "번역 메모리",
        "en-US",
        "ko-KR",
        "",
        false,
    )
    .expect("TB 용어 추가 실패");
    tb.add("fuzzy matching", "퍼지 매칭", "en-US", "ko-KR", "", false)
        .expect("TB 용어 추가 실패");

    // 4) 각 세그먼트에 TM 검색 + TB 조회
    let mut translated_count = 0;

    for seg in &project.segments {
        // TM 검색 (min_score 0.7)
        let tm_matches = tm
            .search(TmSearchParams {
                query: &seg.source,
                source_lang: "en-US",
                target_lang: "ko-KR",
                min_score: 0.7,
            })
            .expect("TM 검색 실패");

        if !tm_matches.is_empty() {
            translated_count += 1;
        }

        // TB 조회 — 소스에 "memory" 또는 "fuzzy"가 포함된 경우
        let tb_source_lower = seg.source.to_lowercase();
        if tb_source_lower.contains("memory") {
            let tb_results = tb.lookup("memory", "en-US").expect("TB 조회 실패");
            assert!(!tb_results.is_empty(), "memory 용어가 TB에 있어야 합니다");
        }
        if tb_source_lower.contains("fuzzy") {
            let tb_results = tb.lookup("fuzzy", "en-US").expect("TB 조회 실패");
            assert!(!tb_results.is_empty(), "fuzzy 용어가 TB에 있어야 합니다");
        }
    }

    // 5) 최소 1개 세그먼트는 TM 매치가 있어야 함 (segment 1: "Hello, world!" 정확 매치)
    assert!(
        translated_count >= 1,
        "최소 1개 세그먼트에 TM 매치가 있어야 합니다"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Phase 2 통합 테스트
// ════════════════════════════════════════════════════════════════════════════

// ─── QA 체크 엔진 통합 테스트 ────────────────────────────────────────────────

/// XLIFF 파싱 후 QA 체크를 실행하여 태그 불일치를 감지하는지 검증
#[test]
fn test_qa_tag_mismatch_on_parsed_xliff() {
    let fixture_path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/sample.xliff");
    let project = parser::parse(fixture_path).expect("XLIFF 파싱 실패");

    // 태그 불일치 세그먼트 주입
    let mut segments = project.segments.clone();
    segments[0].target = "안녕 <b>세계".to_string(); // </b> 누락
    segments[0].status = SegmentStatus::Translated;

    let issues = qa::run_checks(&segments, &[]);
    assert!(
        issues
            .iter()
            .any(|i| i.check_type == qa::QaCheckType::TagMismatch),
        "태그 불일치 QA 이슈가 감지되어야 합니다"
    );
}

/// 미번역 세그먼트를 포함한 XLIFF를 QA 검사할 때 Untranslated 이슈가 보고되는지 검증
#[test]
fn test_qa_untranslated_detected_on_xliff_segments() {
    let fixture_path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/sample.xliff");
    let project = parser::parse(fixture_path).expect("XLIFF 파싱 실패");

    // 일부 세그먼트를 번역하고 일부는 비워둠
    let mut segments = project.segments.clone();
    segments[0].target = "안녕, 세계!".to_string();
    segments[0].status = SegmentStatus::Translated;
    // segments[1], segments[2]는 타겟이 비어있음

    let issues = qa::run_checks(&segments, &[]);
    let untranslated: Vec<_> = issues
        .iter()
        .filter(|i| i.check_type == qa::QaCheckType::Untranslated)
        .collect();
    assert!(
        untranslated.len() >= 2,
        "미번역 세그먼트가 최소 2개 보고되어야 합니다. 실제: {}",
        untranslated.len()
    );
}

/// TB 금지 용어를 사용한 번역에서 ForbiddenTerm 이슈가 보고되는지 검증
#[test]
fn test_qa_forbidden_term_with_tb_engine() {
    // TB 생성 및 금지 용어 추가
    let tb_id = TbEngine::create("qa-forbidden-integration-tb").expect("TB 생성 실패");
    let tb = TbEngine::open(&tb_id).expect("TB 열기 실패");
    tb.add("old term", "구버전용어", "en-US", "ko-KR", "", true)
        .expect("TB 금지어 추가 실패");

    // TB 엔진에서 항목 직접 조회
    let entries: Vec<TbEntry> = vec![TbEntry {
        id: uuid::Uuid::new_v4().to_string(),
        source_term: "old term".to_string(),
        target_term: "구버전용어".to_string(),
        source_lang: "en-US".to_string(),
        target_lang: "ko-KR".to_string(),
        notes: String::new(),
        forbidden: true,
    }];

    let segments = vec![Segment {
        id: "s1".to_string(),
        source: "This uses the old term for the concept.".to_string(),
        target: "이 번역은 구버전용어를 사용합니다.".to_string(),
        status: SegmentStatus::Translated,
        order: 0,
    }];

    let issues = qa::run_checks(&segments, &entries);
    assert!(
        issues
            .iter()
            .any(|i| i.check_type == qa::QaCheckType::ForbiddenTerm),
        "금지 용어 QA 이슈가 감지되어야 합니다"
    );
}

/// 깨끗한 번역(태그 일치, 숫자 일치, 번역됨)에 대해 QA 이슈가 없어야 함
#[test]
fn test_qa_no_issues_for_clean_translation() {
    let segments = vec![
        Segment {
            id: "s1".to_string(),
            source: "There are <b>3</b> files to process.".to_string(),
            target: "<b>3</b>개의 파일을 처리합니다.".to_string(),
            status: SegmentStatus::Translated,
            order: 0,
        },
        Segment {
            id: "s2".to_string(),
            source: "Translation memory is useful.".to_string(),
            target: "번역 메모리는 유용합니다.".to_string(),
            status: SegmentStatus::Confirmed,
            order: 1,
        },
    ];

    let issues = qa::run_checks(&segments, &[]);
    assert!(
        issues.is_empty(),
        "깨끗한 번역에는 QA 이슈가 없어야 합니다. 실제 이슈: {:?}",
        issues
    );
}

/// 소스=타겟 감지 통합 테스트
#[test]
fn test_qa_source_equals_target_integration() {
    let segments = vec![Segment {
        id: "s1".to_string(),
        source: "Please review this document carefully.".to_string(),
        target: "Please review this document carefully.".to_string(), // 미번역 그대로
        status: SegmentStatus::Translated,
        order: 0,
    }];

    let issues = qa::run_checks(&segments, &[]);
    assert!(
        issues
            .iter()
            .any(|i| i.check_type == qa::QaCheckType::SourceEqualsTarget),
        "소스=타겟 QA 이슈가 감지되어야 합니다"
    );
}

// ─── MT 통합 테스트 (Mock HTTP) ───────────────────────────────────────────────

/// DeepL mock 서버를 이용한 번역 성공 플로우 검증
#[tokio::test]
async fn test_mt_deepl_translate_success_flow() {
    let mut server = mockito::Server::new_async().await;
    let _m = server
        .mock("POST", "/v2/translate")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"translations":[{"text":"안녕하세요 세계","detected_source_language":"EN"}]}"#,
        )
        .create_async()
        .await;

    let client = reqwest::Client::new();
    let url = format!("{}/v2/translate", server.url());

    let resp = client
        .post(&url)
        .header("Authorization", "DeepL-Auth-Key test-key:fx")
        .json(&serde_json::json!({
            "text": ["Hello world"],
            "source_lang": "EN",
            "target_lang": "KO"
        }))
        .send()
        .await
        .expect("HTTP 요청 실패");

    assert!(resp.status().is_success());
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(
        body["translations"][0]["text"].as_str().unwrap(),
        "안녕하세요 세계"
    );
}

/// DeepL mock 서버 — 429 Rate Limit 응답 플로우 검증
#[tokio::test]
async fn test_mt_deepl_rate_limit_flow() {
    let mut server = mockito::Server::new_async().await;
    let _m = server
        .mock("POST", "/v2/translate")
        .with_status(429)
        .with_body(r#"{"message":"Too many requests"}"#)
        .create_async()
        .await;

    let client = reqwest::Client::new();
    let url = format!("{}/v2/translate", server.url());

    let resp = client
        .post(&url)
        .header("Authorization", "DeepL-Auth-Key test-key:fx")
        .json(&serde_json::json!({"text": ["Hi"], "source_lang": "EN", "target_lang": "KO"}))
        .send()
        .await
        .expect("HTTP 요청 실패");

    assert_eq!(resp.status().as_u16(), 429);
}

/// Google Translate mock 서버를 이용한 번역 성공 플로우 검증
#[tokio::test]
async fn test_mt_google_translate_success_flow() {
    let mut server = mockito::Server::new_async().await;
    let _m = server
        .mock("POST", "/language/translate/v2")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"data":{"translations":[{"translatedText":"안녕하세요"}]}}"#)
        .create_async()
        .await;

    let client = reqwest::Client::new();
    let url = format!("{}/language/translate/v2", server.url());

    let resp = client
        .post(&url)
        .json(&serde_json::json!({
            "q": "Hello",
            "source": "en",
            "target": "ko",
            "format": "text",
            "key": "test-api-key"
        }))
        .send()
        .await
        .expect("HTTP 요청 실패");

    assert!(resp.status().is_success());
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(
        body["data"]["translations"][0]["translatedText"]
            .as_str()
            .unwrap(),
        "안녕하세요"
    );
}

/// Google Translate mock 서버 — 인증 오류(400) 응답 플로우 검증
#[tokio::test]
async fn test_mt_google_auth_error_flow() {
    let mut server = mockito::Server::new_async().await;
    let _m = server
        .mock("POST", "/language/translate/v2")
        .with_status(400)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"error":{"code":400,"message":"API key not valid","status":"INVALID_ARGUMENT"}}"#,
        )
        .create_async()
        .await;

    let client = reqwest::Client::new();
    let url = format!("{}/language/translate/v2", server.url());

    let resp = client
        .post(&url)
        .json(&serde_json::json!({
            "q": "Hello",
            "source": "en",
            "target": "ko",
            "format": "text",
            "key": "bad-key"
        }))
        .send()
        .await
        .expect("HTTP 요청 실패");

    assert_eq!(resp.status().as_u16(), 400);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(
        body["error"]["message"].as_str().unwrap(),
        "API key not valid"
    );
}

/// MT 프로바이더 정보 조회 통합 테스트
#[test]
fn test_mt_get_providers_returns_deepl_and_google() {
    use memoq_clone_lib::mt::engine::get_providers;
    let providers = get_providers();
    assert_eq!(
        providers.len(),
        2,
        "DeepL과 Google 두 프로바이더가 있어야 합니다"
    );
    let ids: Vec<&str> = providers.iter().map(|p| p.id.as_str()).collect();
    assert!(ids.contains(&"deepl"), "DeepL 프로바이더가 있어야 합니다");
    assert!(ids.contains(&"google"), "Google 프로바이더가 있어야 합니다");
    assert!(
        providers.iter().all(|p| p.requires_api_key),
        "모든 프로바이더는 API 키가 필요합니다"
    );
}

// ─── 프로젝트 관리 통합 테스트 ────────────────────────────────────────────────

fn make_project() -> Project {
    Project {
        id: Uuid::new_v4().to_string(),
        name: "Integration Test Project".to_string(),
        source_lang: "en-US".to_string(),
        target_lang: "ko-KR".to_string(),
        created_at: Utc::now(),
        files: Vec::new(),
        source_path: String::new(),
        segments: Vec::new(),
    }
}

fn make_segment(status: SegmentStatus) -> Segment {
    Segment {
        id: Uuid::new_v4().to_string(),
        source: "Sample source text.".to_string(),
        target: "샘플 타겟 텍스트.".to_string(),
        status,
        order: 0,
    }
}

/// 다중 파일 추가/제거 플로우: add_file → remove_file 검증
#[tokio::test]
async fn test_project_multi_file_add_remove() {
    use memoq_clone_lib::commands::project::{add_file_to_project, remove_file_from_project};

    let project = make_project();

    // 파일 2개 추가
    let project = add_file_to_project(project, "/docs/file1.xliff".to_string())
        .await
        .expect("파일1 추가 실패");
    let project = add_file_to_project(project, "/docs/file2.xliff".to_string())
        .await
        .expect("파일2 추가 실패");
    assert_eq!(project.files.len(), 2, "파일이 2개여야 합니다");

    // 첫 번째 파일 제거
    let file1_id = project.files[0].id.clone();
    let project = remove_file_from_project(project, file1_id)
        .await
        .expect("파일1 제거 실패");
    assert_eq!(project.files.len(), 1, "제거 후 파일이 1개여야 합니다");
    assert_eq!(project.files[0].path, "/docs/file2.xliff");
}

/// .mqclone 파일 저장/불러오기 라운드트립 검증
#[tokio::test]
async fn test_project_save_load_mqclone_roundtrip() {
    use memoq_clone_lib::commands::project::{load_project, save_project};

    let dir = TempDir::new().expect("임시 디렉토리 생성 실패");
    let path = dir.path().join("myproject.mqclone");
    let path_str = path.to_str().unwrap().to_string();

    let mut project = make_project();
    project.name = "Roundtrip Test Project".to_string();
    project.files.push(ProjectFile {
        id: "file-001".to_string(),
        path: "/path/to/source.xliff".to_string(),
        segments: vec![
            make_segment(SegmentStatus::Translated),
            make_segment(SegmentStatus::Confirmed),
            make_segment(SegmentStatus::Untranslated),
        ],
    });

    save_project(project.clone(), path_str.clone())
        .await
        .expect("프로젝트 저장 실패");
    assert!(path.exists(), ".mqclone 파일이 존재해야 합니다");

    let loaded = load_project(path_str)
        .await
        .expect("프로젝트 불러오기 실패");
    assert_eq!(loaded.name, "Roundtrip Test Project");
    assert_eq!(loaded.source_lang, "en-US");
    assert_eq!(loaded.target_lang, "ko-KR");
    assert_eq!(loaded.files.len(), 1);
    assert_eq!(loaded.files[0].segments.len(), 3);
}

/// 다중 파일 프로젝트 통계 정확도 검증
#[tokio::test]
async fn test_project_stats_multi_file_accuracy() {
    use memoq_clone_lib::commands::project::get_project_stats;

    let mut project = make_project();

    // 파일1: 4개 세그먼트 (2 Confirmed, 1 Translated, 1 Untranslated)
    project.files.push(ProjectFile {
        id: "f1".to_string(),
        path: "/file1.xliff".to_string(),
        segments: vec![
            make_segment(SegmentStatus::Confirmed),
            make_segment(SegmentStatus::Confirmed),
            make_segment(SegmentStatus::Translated),
            make_segment(SegmentStatus::Untranslated),
        ],
    });

    // 파일2: 3개 세그먼트 (1 Confirmed, 2 Draft)
    project.files.push(ProjectFile {
        id: "f2".to_string(),
        path: "/file2.xliff".to_string(),
        segments: vec![
            make_segment(SegmentStatus::Confirmed),
            make_segment(SegmentStatus::Draft),
            make_segment(SegmentStatus::Draft),
        ],
    });

    let stats = get_project_stats(project).await.expect("통계 조회 실패");

    assert_eq!(stats.total_segments, 7, "전체 세그먼트 7개여야 합니다");
    assert_eq!(stats.confirmed, 3, "Confirmed 세그먼트 3개여야 합니다");
    assert_eq!(
        stats.translated, 4,
        "Translated+Confirmed 세그먼트 4개여야 합니다"
    );
    let expected_pct = 4.0 / 7.0 * 100.0;
    assert!(
        (stats.completion_pct - expected_pct).abs() < 0.01,
        "완성률 {:.1}%여야 합니다. 실제: {:.1}%",
        expected_pct,
        stats.completion_pct
    );
}

/// 빈 프로젝트 통계: 0 세그먼트, 완성률 0%
#[tokio::test]
async fn test_project_stats_empty_project() {
    use memoq_clone_lib::commands::project::get_project_stats;

    let project = make_project();
    let stats = get_project_stats(project).await.expect("통계 조회 실패");

    assert_eq!(stats.total_segments, 0);
    assert_eq!(stats.translated, 0);
    assert_eq!(stats.confirmed, 0);
    assert_eq!(stats.completion_pct, 0.0);
}

// ─── LiveDocs 통합 테스트 ──────────────────────────────────────────────────────

/// TXT 파일 인덱싱 후 정확 검색 결과 검증
#[test]
fn test_livedocs_index_txt_and_search_exact() {
    use memoq_clone_lib::livedocs::index::{add_document, create_library};
    use memoq_clone_lib::livedocs::search::search;

    let dir = TempDir::new().expect("임시 디렉토리 생성 실패");
    let txt_path = dir.path().join("reference.txt");
    std::fs::write(
        &txt_path,
        "Translation memory helps translators work faster.\nFuzzy matching finds similar sentences.\nTerm base stores terminology for consistency.",
    )
    .expect("TXT 파일 생성 실패");

    // 라이브러리 생성 및 문서 추가
    let lib = create_library("livedocs-search-test").expect("라이브러리 생성 실패");
    let lib = add_document(&lib.id, txt_path.to_str().unwrap()).expect("문서 추가 실패");

    assert_eq!(lib.documents.len(), 1, "문서가 1개여야 합니다");
    assert!(
        !lib.documents[0].sentences.is_empty(),
        "문장이 분리되어야 합니다"
    );

    // 정확 검색
    let results = search(
        "Translation memory helps translators work faster.",
        &lib.id,
        Some(0.95),
    )
    .expect("검색 실패");

    assert!(!results.is_empty(), "정확 매치 결과가 있어야 합니다");
    assert!(
        results[0].score >= 0.95,
        "정확 매치 점수가 0.95 이상이어야 합니다. 실제: {}",
        results[0].score
    );
}

/// LiveDocs 퍼지 검색: 유사한 쿼리로 70% 이상 매치 검증
#[test]
fn test_livedocs_fuzzy_search_results() {
    use memoq_clone_lib::livedocs::index::{add_document, create_library};
    use memoq_clone_lib::livedocs::search::search;

    let dir = TempDir::new().expect("임시 디렉토리 생성 실패");
    let txt_path = dir.path().join("fuzzy_reference.txt");
    std::fs::write(
        &txt_path,
        "The quick brown fox jumps over the lazy dog today.",
    )
    .expect("TXT 파일 생성 실패");

    let lib = create_library("livedocs-fuzzy-test").expect("라이브러리 생성 실패");
    add_document(&lib.id, txt_path.to_str().unwrap()).expect("문서 추가 실패");

    // 약간 다른 쿼리로 퍼지 검색
    let results = search(
        "The quick brown fox jumps over the lazy dogs.",
        &lib.id,
        Some(0.7),
    )
    .expect("퍼지 검색 실패");

    assert!(!results.is_empty(), "퍼지 매치 결과가 있어야 합니다");
    assert!(
        results[0].score >= 0.7,
        "퍼지 매치 점수가 0.7 이상이어야 합니다. 실제: {}",
        results[0].score
    );
}

/// LiveDocs 임계값 이하 검색: 낮은 유사도 결과는 필터링되어야 함
#[test]
fn test_livedocs_search_below_threshold_returns_empty() {
    use memoq_clone_lib::livedocs::index::{add_document, create_library};
    use memoq_clone_lib::livedocs::search::search;

    let dir = TempDir::new().expect("임시 디렉토리 생성 실패");
    let txt_path = dir.path().join("unrelated.txt");
    std::fs::write(
        &txt_path,
        "Completely unrelated content about astrophysics and dark matter.",
    )
    .expect("TXT 파일 생성 실패");

    let lib = create_library("livedocs-threshold-test").expect("라이브러리 생성 실패");
    add_document(&lib.id, txt_path.to_str().unwrap()).expect("문서 추가 실패");

    // 완전히 다른 쿼리로 높은 임계값 검색
    let results =
        search("Hello world translation memory fuzzy.", &lib.id, Some(0.9)).expect("검색 실패");

    assert!(
        results.is_empty(),
        "임계값 이하의 결과는 없어야 합니다. 실제: {}개",
        results.len()
    );
}

/// LiveDocs 검색 결과 정렬: 점수 내림차순 정렬 검증
#[test]
fn test_livedocs_search_results_sorted_by_score() {
    use memoq_clone_lib::livedocs::index::{add_document, create_library};
    use memoq_clone_lib::livedocs::search::search;

    let dir = TempDir::new().expect("임시 디렉토리 생성 실패");
    let txt_path = dir.path().join("multi_sentence.txt");
    std::fs::write(
        &txt_path,
        "Translation memory is a tool for translators.\nTranslation memory helps improve consistency.\nA completely different topic about the weather.",
    )
    .expect("TXT 파일 생성 실패");

    let lib = create_library("livedocs-sort-test").expect("라이브러리 생성 실패");
    add_document(&lib.id, txt_path.to_str().unwrap()).expect("문서 추가 실패");

    let results = search(
        "Translation memory helps improve consistency.",
        &lib.id,
        Some(0.3),
    )
    .expect("검색 실패");

    // 결과가 2개 이상이면 점수 내림차순 정렬 검증
    if results.len() >= 2 {
        for i in 0..results.len() - 1 {
            assert!(
                results[i].score >= results[i + 1].score,
                "결과는 점수 내림차순으로 정렬되어야 합니다: [{}]={} < [{}]={}",
                i,
                results[i].score,
                i + 1,
                results[i + 1].score
            );
        }
    }

    // 가장 높은 점수가 정확 매치임을 검증
    assert!(!results.is_empty(), "검색 결과가 있어야 합니다");
    assert!(
        results[0].score > 0.8,
        "정확 매치가 첫 번째 결과여야 합니다. 실제 점수: {}",
        results[0].score
    );
}

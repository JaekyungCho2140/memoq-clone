/// Integration tests: parser → TM → TB → export chain
///
/// 각 테스트는 독립적으로 실행 가능하며 공유 상태가 없습니다.
/// TM/TB는 UUID 기반 고유 ID를 사용하므로 테스트 간 충돌이 없습니다.
/// DOCX 파일은 tempfile 크레이트으로 임시 디렉토리에 생성합니다.
use memoq_clone_lib::models::SegmentStatus;
use memoq_clone_lib::parser;
use memoq_clone_lib::tb::TbEngine;
use memoq_clone_lib::tm::{TmEngine, TmSearchParams};
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;

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

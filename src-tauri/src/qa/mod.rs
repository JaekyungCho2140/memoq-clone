use crate::models::{Segment, SegmentStatus, TbEntry};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::OnceLock;

// ── QA 타입 정의 ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum QaCheckType {
    TagMismatch,
    NumberMismatch,
    Untranslated,
    ForbiddenTerm,
    SourceEqualsTarget,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum QaSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QaIssue {
    pub segment_id: String,
    pub check_type: QaCheckType,
    pub severity: QaSeverity,
    pub message: String,
}

// ── 정규식 헬퍼 ─────────────────────────────────────────────────────────────

fn tag_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"<[^>]+>").unwrap())
}

fn number_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\d+(?:[.,]\d+)*").unwrap())
}

fn extract_tags(text: &str) -> Vec<String> {
    tag_regex()
        .find_iter(text)
        .map(|m| m.as_str().to_string())
        .collect()
}

fn extract_numbers(text: &str) -> HashSet<String> {
    number_regex()
        .find_iter(text)
        .map(|m| m.as_str().to_string())
        .collect()
}

// ── 개별 검사 함수 ───────────────────────────────────────────────────────────

/// 태그 불일치: 소스/타겟 XML 태그 수 및 순서 비교
fn check_tag_mismatch(seg: &Segment) -> Option<QaIssue> {
    let src_tags = extract_tags(&seg.source);
    let tgt_tags = extract_tags(&seg.target);
    if src_tags != tgt_tags {
        Some(QaIssue {
            segment_id: seg.id.clone(),
            check_type: QaCheckType::TagMismatch,
            severity: QaSeverity::Error,
            message: format!("태그 불일치: 소스 {:?} ≠ 타겟 {:?}", src_tags, tgt_tags),
        })
    } else {
        None
    }
}

/// 숫자 불일치: 정규식으로 숫자 추출 후 소스↔타겟 비교
fn check_number_mismatch(seg: &Segment) -> Option<QaIssue> {
    if seg.target.trim().is_empty() {
        return None; // 미번역은 별도 체크
    }
    let src_nums = extract_numbers(&seg.source);
    let tgt_nums = extract_numbers(&seg.target);
    if src_nums != tgt_nums {
        let missing: Vec<&String> = src_nums.difference(&tgt_nums).collect();
        let extra: Vec<&String> = tgt_nums.difference(&src_nums).collect();
        let mut parts = Vec::new();
        if !missing.is_empty() {
            parts.push(format!("소스에만 있음: {:?}", missing));
        }
        if !extra.is_empty() {
            parts.push(format!("타겟에만 있음: {:?}", extra));
        }
        Some(QaIssue {
            segment_id: seg.id.clone(),
            check_type: QaCheckType::NumberMismatch,
            severity: QaSeverity::Warning,
            message: format!("숫자 불일치 — {}", parts.join(", ")),
        })
    } else {
        None
    }
}

/// 미번역 세그먼트: 타겟이 비어있거나 Untranslated 상태
fn check_untranslated(seg: &Segment) -> Option<QaIssue> {
    if seg.target.trim().is_empty() || seg.status == SegmentStatus::Untranslated {
        Some(QaIssue {
            segment_id: seg.id.clone(),
            check_type: QaCheckType::Untranslated,
            severity: QaSeverity::Warning,
            message: "미번역 세그먼트".to_string(),
        })
    } else {
        None
    }
}

/// 금지 용어 사용: TB에서 forbidden=true 용어의 target_term이 타겟 텍스트에 포함되는지 확인
fn check_forbidden_terms(seg: &Segment, tb_entries: &[TbEntry]) -> Vec<QaIssue> {
    if seg.target.trim().is_empty() {
        return Vec::new();
    }
    let target_lower = seg.target.to_lowercase();
    tb_entries
        .iter()
        .filter(|e| e.forbidden)
        .filter(|e| {
            let term_lower = e.target_term.to_lowercase();
            !term_lower.is_empty() && target_lower.contains(&term_lower)
        })
        .map(|e| QaIssue {
            segment_id: seg.id.clone(),
            check_type: QaCheckType::ForbiddenTerm,
            severity: QaSeverity::Error,
            message: format!("금지 용어 사용: '{}'", e.target_term),
        })
        .collect()
}

/// 소스=타겟: 소스와 타겟 텍스트가 동일한 경우 (비어있지 않을 때)
fn check_source_equals_target(seg: &Segment) -> Option<QaIssue> {
    let src = seg.source.trim();
    let tgt = seg.target.trim();
    if !src.is_empty() && !tgt.is_empty() && src == tgt {
        Some(QaIssue {
            segment_id: seg.id.clone(),
            check_type: QaCheckType::SourceEqualsTarget,
            severity: QaSeverity::Warning,
            message: "소스와 타겟이 동일함".to_string(),
        })
    } else {
        None
    }
}

// ── 공개 API ────────────────────────────────────────────────────────────────

/// 세그먼트 목록에 대해 모든 QA 검사를 실행하고 이슈 목록을 반환한다.
pub fn run_checks(segments: &[Segment], tb_entries: &[TbEntry]) -> Vec<QaIssue> {
    let mut issues = Vec::new();
    for seg in segments {
        if let Some(issue) = check_tag_mismatch(seg) {
            issues.push(issue);
        }
        if let Some(issue) = check_number_mismatch(seg) {
            issues.push(issue);
        }
        if let Some(issue) = check_untranslated(seg) {
            issues.push(issue);
        }
        issues.extend(check_forbidden_terms(seg, tb_entries));
        if let Some(issue) = check_source_equals_target(seg) {
            issues.push(issue);
        }
    }
    issues
}

// ── 단위 테스트 ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Segment, SegmentStatus, TbEntry};

    fn seg(id: &str, source: &str, target: &str) -> Segment {
        Segment {
            id: id.to_string(),
            source: source.to_string(),
            target: target.to_string(),
            status: SegmentStatus::Translated,
            order: 0,
        }
    }

    fn seg_with_status(id: &str, source: &str, target: &str, status: SegmentStatus) -> Segment {
        Segment {
            id: id.to_string(),
            source: source.to_string(),
            target: target.to_string(),
            status,
            order: 0,
        }
    }

    fn tb_entry(source_term: &str, target_term: &str, forbidden: bool) -> TbEntry {
        TbEntry {
            id: uuid::Uuid::new_v4().to_string(),
            source_term: source_term.to_string(),
            target_term: target_term.to_string(),
            source_lang: "en".to_string(),
            target_lang: "ko".to_string(),
            notes: String::new(),
            forbidden,
        }
    }

    // ── 태그 불일치 ──────────────────────────────────────────────────────────

    #[test]
    fn test_tag_mismatch_detected() {
        let s = seg("s1", "Hello <b>world</b>", "안녕 <b>세계");
        let issues = run_checks(&[s], &[]);
        assert!(issues
            .iter()
            .any(|i| i.check_type == QaCheckType::TagMismatch));
    }

    #[test]
    fn test_tag_match_no_issue() {
        let s = seg("s2", "Hello <b>world</b>", "안녕 <b>세계</b>");
        let issues = run_checks(&[s], &[]);
        assert!(!issues
            .iter()
            .any(|i| i.check_type == QaCheckType::TagMismatch));
    }

    #[test]
    fn test_no_tags_no_issue() {
        let s = seg("s3", "Hello world", "안녕 세계");
        let issues = run_checks(&[s], &[]);
        assert!(!issues
            .iter()
            .any(|i| i.check_type == QaCheckType::TagMismatch));
    }

    #[test]
    fn test_tag_order_mismatch_detected() {
        let s = seg("s4", "<b><i>text</i></b>", "<i><b>텍스트</b></i>");
        let issues = run_checks(&[s], &[]);
        assert!(issues
            .iter()
            .any(|i| i.check_type == QaCheckType::TagMismatch));
    }

    // ── 숫자 불일치 ──────────────────────────────────────────────────────────

    #[test]
    fn test_number_mismatch_detected() {
        let s = seg("s5", "There are 3 items", "항목이 4개 있습니다");
        let issues = run_checks(&[s], &[]);
        assert!(issues
            .iter()
            .any(|i| i.check_type == QaCheckType::NumberMismatch));
    }

    #[test]
    fn test_number_match_no_issue() {
        let s = seg("s6", "There are 3 items", "항목이 3개 있습니다");
        let issues = run_checks(&[s], &[]);
        assert!(!issues
            .iter()
            .any(|i| i.check_type == QaCheckType::NumberMismatch));
    }

    #[test]
    fn test_decimal_number_mismatch() {
        let s = seg("s7", "Price: 1,234.56", "가격: 1234.56");
        let issues = run_checks(&[s], &[]);
        assert!(issues
            .iter()
            .any(|i| i.check_type == QaCheckType::NumberMismatch));
    }

    #[test]
    fn test_number_mismatch_skips_empty_target() {
        let s = seg_with_status("s8", "3 items", "", SegmentStatus::Untranslated);
        let issues = run_checks(&[s], &[]);
        assert!(!issues
            .iter()
            .any(|i| i.check_type == QaCheckType::NumberMismatch));
    }

    // ── 미번역 세그먼트 ──────────────────────────────────────────────────────

    #[test]
    fn test_untranslated_empty_target() {
        let s = seg_with_status("s9", "Hello", "", SegmentStatus::Untranslated);
        let issues = run_checks(&[s], &[]);
        assert!(issues
            .iter()
            .any(|i| i.check_type == QaCheckType::Untranslated));
    }

    #[test]
    fn test_untranslated_status_flag() {
        let s = seg_with_status("s10", "Hello", "안녕", SegmentStatus::Untranslated);
        let issues = run_checks(&[s], &[]);
        assert!(issues
            .iter()
            .any(|i| i.check_type == QaCheckType::Untranslated));
    }

    #[test]
    fn test_translated_no_untranslated_issue() {
        let s = seg("s11", "Hello", "안녕");
        let issues = run_checks(&[s], &[]);
        assert!(!issues
            .iter()
            .any(|i| i.check_type == QaCheckType::Untranslated));
    }

    // ── 금지 용어 ────────────────────────────────────────────────────────────

    #[test]
    fn test_forbidden_term_detected() {
        let s = seg("s12", "Hello world", "안녕 금지어");
        let tb = vec![tb_entry("forbidden", "금지어", true)];
        let issues = run_checks(&[s], &tb);
        assert!(issues
            .iter()
            .any(|i| i.check_type == QaCheckType::ForbiddenTerm));
    }

    #[test]
    fn test_allowed_term_no_issue() {
        let s = seg("s13", "Hello world", "안녕 허용어");
        let tb = vec![tb_entry("allowed", "허용어", false)];
        let issues = run_checks(&[s], &tb);
        assert!(!issues
            .iter()
            .any(|i| i.check_type == QaCheckType::ForbiddenTerm));
    }

    #[test]
    fn test_forbidden_term_case_insensitive() {
        let s = seg("s14", "Hello", "안녕 BADWORD 좋아");
        let tb = vec![tb_entry("bad", "badword", true)];
        let issues = run_checks(&[s], &tb);
        assert!(issues
            .iter()
            .any(|i| i.check_type == QaCheckType::ForbiddenTerm));
    }

    #[test]
    fn test_forbidden_term_skips_empty_target() {
        let s = seg_with_status("s15", "Hello", "", SegmentStatus::Untranslated);
        let tb = vec![tb_entry("bad", "bad", true)];
        let issues = run_checks(&[s], &tb);
        assert!(!issues
            .iter()
            .any(|i| i.check_type == QaCheckType::ForbiddenTerm));
    }

    // ── 소스=타겟 ────────────────────────────────────────────────────────────

    #[test]
    fn test_source_equals_target_detected() {
        let s = seg("s16", "Hello world", "Hello world");
        let issues = run_checks(&[s], &[]);
        assert!(issues
            .iter()
            .any(|i| i.check_type == QaCheckType::SourceEqualsTarget));
    }

    #[test]
    fn test_source_not_equals_target_no_issue() {
        let s = seg("s17", "Hello world", "안녕 세계");
        let issues = run_checks(&[s], &[]);
        assert!(!issues
            .iter()
            .any(|i| i.check_type == QaCheckType::SourceEqualsTarget));
    }

    #[test]
    fn test_source_equals_target_skips_empty() {
        let s = seg_with_status("s18", "Hello", "", SegmentStatus::Untranslated);
        let issues = run_checks(&[s], &[]);
        assert!(!issues
            .iter()
            .any(|i| i.check_type == QaCheckType::SourceEqualsTarget));
    }

    // ── 복합 케이스 ──────────────────────────────────────────────────────────

    #[test]
    fn test_multiple_issues_on_one_segment() {
        // 미번역 + 소스=타겟을 동시에 갖는 케이스는 없으나 태그+숫자 동시 가능
        let s = seg("s19", "<b>3</b> items", "<b>4 items");
        let issues = run_checks(&[s], &[]);
        // 태그 불일치 (</b> 없음)
        assert!(issues
            .iter()
            .any(|i| i.check_type == QaCheckType::TagMismatch));
        // 숫자 불일치
        assert!(issues
            .iter()
            .any(|i| i.check_type == QaCheckType::NumberMismatch));
    }

    #[test]
    fn test_clean_segment_no_issues() {
        let s = seg(
            "s20",
            "Hello <b>world</b> has 3 items",
            "안녕 <b>세계</b>에 3개 있음",
        );
        let issues = run_checks(&[s], &[]);
        assert!(issues.is_empty());
    }

    #[test]
    fn test_empty_segments_returns_empty() {
        let issues = run_checks(&[], &[]);
        assert!(issues.is_empty());
    }
}

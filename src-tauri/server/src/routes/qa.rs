use axum::{extract::State, Json};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::{
    app::AppState,
    auth::middleware::AuthUser,
    db::run_db,
    error::{AppError, AppResult},
};

// ── 요청/응답 타입 ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QaCheckRequest {
    pub project_id: String,
    pub tb_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QaIssue {
    pub segment_id: String,
    pub check_type: String,
    pub severity: String,
    pub message: String,
}

// ── 헬퍼: 정규식 없이 태그·숫자 추출 ──────────────────────────────────────────

fn extract_tags(text: &str) -> Vec<String> {
    let mut tags = Vec::new();
    let mut in_tag = false;
    let mut buf = String::new();
    for ch in text.chars() {
        if ch == '<' {
            in_tag = true;
            buf.clear();
            buf.push(ch);
        } else if ch == '>' && in_tag {
            buf.push(ch);
            tags.push(buf.clone());
            in_tag = false;
        } else if in_tag {
            buf.push(ch);
        }
    }
    tags
}

fn extract_numbers(text: &str) -> HashSet<String> {
    let mut nums = HashSet::new();
    let mut buf = String::new();
    let mut prev_was_digit = false;

    for ch in text.chars() {
        if ch.is_ascii_digit() {
            buf.push(ch);
            prev_was_digit = true;
        } else if (ch == '.' || ch == ',') && prev_was_digit {
            buf.push(ch);
        } else {
            if !buf.is_empty() {
                let trimmed = buf.trim_end_matches(['.', ',']).to_string();
                if !trimmed.is_empty() {
                    nums.insert(trimmed);
                }
                buf.clear();
            }
            prev_was_digit = false;
        }
    }
    if !buf.is_empty() {
        let trimmed = buf.trim_end_matches(['.', ',']).to_string();
        if !trimmed.is_empty() {
            nums.insert(trimmed);
        }
    }
    nums
}

// ── 검사 함수들 ────────────────────────────────────────────────────────────────

fn check_tag_mismatch(id: &str, source: &str, target: &str) -> Option<QaIssue> {
    let src_tags = extract_tags(source);
    let tgt_tags = extract_tags(target);
    if src_tags != tgt_tags {
        Some(QaIssue {
            segment_id: id.to_string(),
            check_type: "TagMismatch".to_string(),
            severity: "Error".to_string(),
            message: format!("태그 불일치: 소스 {:?} ≠ 타겟 {:?}", src_tags, tgt_tags),
        })
    } else {
        None
    }
}

fn check_number_mismatch(id: &str, source: &str, target: &str) -> Option<QaIssue> {
    if target.trim().is_empty() {
        return None;
    }
    let src_nums = extract_numbers(source);
    let tgt_nums = extract_numbers(target);
    if src_nums != tgt_nums {
        let mut missing: Vec<&String> = src_nums.difference(&tgt_nums).collect();
        let mut extra: Vec<&String> = tgt_nums.difference(&src_nums).collect();
        missing.sort();
        extra.sort();
        let mut parts = Vec::new();
        if !missing.is_empty() {
            parts.push(format!("소스에만 있음: {:?}", missing));
        }
        if !extra.is_empty() {
            parts.push(format!("타겟에만 있음: {:?}", extra));
        }
        Some(QaIssue {
            segment_id: id.to_string(),
            check_type: "NumberMismatch".to_string(),
            severity: "Warning".to_string(),
            message: format!("숫자 불일치 — {}", parts.join(", ")),
        })
    } else {
        None
    }
}

fn check_untranslated(id: &str, target: &str, status: &str) -> Option<QaIssue> {
    if target.trim().is_empty() || status == "untranslated" {
        Some(QaIssue {
            segment_id: id.to_string(),
            check_type: "Untranslated".to_string(),
            severity: "Warning".to_string(),
            message: "미번역 세그먼트".to_string(),
        })
    } else {
        None
    }
}

fn check_source_equals_target(id: &str, source: &str, target: &str) -> Option<QaIssue> {
    let src = source.trim();
    let tgt = target.trim();
    if !src.is_empty() && !tgt.is_empty() && src == tgt {
        Some(QaIssue {
            segment_id: id.to_string(),
            check_type: "SourceEqualsTarget".to_string(),
            severity: "Warning".to_string(),
            message: "소스와 타겟이 동일함".to_string(),
        })
    } else {
        None
    }
}

// ── 핸들러 ─────────────────────────────────────────────────────────────────────

/// POST /api/qa/check
pub async fn run_qa_check(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(body): Json<QaCheckRequest>,
) -> AppResult<Json<Vec<QaIssue>>> {
    let user_id = claims.sub.clone();
    let project_id = body.project_id.clone();
    let tb_id = body.tb_id.clone();
    let pool = state.pool.clone();

    let result = run_db(pool, move |conn| {
        // 프로젝트 소유자 확인
        let owner: Option<String> = conn
            .query_row(
                "SELECT owner_id FROM projects WHERE id = ?1",
                params![&project_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
        match owner {
            None => return Err(AppError::NotFound("Project not found".to_string())),
            Some(oid) if oid != user_id => return Err(AppError::Forbidden),
            _ => {}
        }

        // 세그먼트 조회
        let mut stmt = conn
            .prepare(
                "SELECT s.id, s.source, s.target, s.status
                 FROM segments s
                 JOIN project_files f ON f.id = s.file_id
                 WHERE f.project_id = ?1
                 ORDER BY f.id, s.seg_order",
            )
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
        let segments: Vec<(String, String, String, String)> = stmt
            .query_map(params![&project_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?
            .collect::<Result<_, _>>()
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

        // TB 금지어 조회 (선택적)
        let forbidden_terms: Vec<String> = if let Some(ref tid) = tb_id {
            if !tid.is_empty() {
                // tb_id가 실제 UUID면 해당 엔트리만, 아니면 전체
                let mut stmt2 = conn
                    .prepare(
                        "SELECT target_term FROM tb_entries WHERE forbidden = 1 AND owner_id = ?1",
                    )
                    .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
                stmt2
                    .query_map(params![&user_id], |row| row.get(0))
                    .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?
                    .collect::<Result<_, _>>()
                    .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?
            } else {
                Vec::new()
            }
        } else {
            // tb_id 없으면 사용자 전체 금지어
            let mut stmt2 = conn
                .prepare(
                    "SELECT target_term FROM tb_entries WHERE forbidden = 1 AND owner_id = ?1",
                )
                .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
            stmt2
                .query_map(params![&user_id], |row| row.get(0))
                .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?
                .collect::<Result<_, _>>()
                .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?
        };

        // QA 검사 실행
        let mut issues = Vec::new();
        for (id, source, target, status) in &segments {
            if let Some(i) = check_tag_mismatch(id, source, target) {
                issues.push(i);
            }
            if let Some(i) = check_number_mismatch(id, source, target) {
                issues.push(i);
            }
            if let Some(i) = check_untranslated(id, target, status) {
                issues.push(i);
            }
            if let Some(i) = check_source_equals_target(id, source, target) {
                issues.push(i);
            }
            // 금지어 검사
            if !target.trim().is_empty() {
                let tgt_lower = target.to_lowercase();
                for term in &forbidden_terms {
                    let term_lower = term.to_lowercase();
                    if !term_lower.is_empty() && tgt_lower.contains(&term_lower) {
                        issues.push(QaIssue {
                            segment_id: id.clone(),
                            check_type: "ForbiddenTerm".to_string(),
                            severity: "Error".to_string(),
                            message: format!("금지 용어 사용: '{}'", term),
                        });
                    }
                }
            }
        }

        Ok(issues)
    })
    .await?;

    Ok(Json(result))
}

// ── 단위 테스트 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_tags_basic() {
        assert_eq!(
            extract_tags("Hello <b>world</b>"),
            vec!["<b>", "</b>"]
        );
    }

    #[test]
    fn test_extract_tags_empty() {
        assert_eq!(extract_tags("Hello world"), Vec::<String>::new());
    }

    #[test]
    fn test_extract_numbers_basic() {
        let nums = extract_numbers("There are 3 items");
        assert!(nums.contains("3"));
    }

    #[test]
    fn test_extract_numbers_decimal() {
        let nums = extract_numbers("Price: 1234.56");
        assert!(nums.contains("1234.56"));
    }

    #[test]
    fn test_tag_mismatch() {
        let issue = check_tag_mismatch("s1", "Hello <b>world</b>", "안녕 <b>세계");
        assert!(issue.is_some());
        assert_eq!(issue.unwrap().check_type, "TagMismatch");
    }

    #[test]
    fn test_tag_match_no_issue() {
        let issue = check_tag_mismatch("s2", "Hello <b>world</b>", "안녕 <b>세계</b>");
        assert!(issue.is_none());
    }

    #[test]
    fn test_number_mismatch() {
        let issue = check_number_mismatch("s3", "3 items", "4개 항목");
        assert!(issue.is_some());
        assert_eq!(issue.unwrap().check_type, "NumberMismatch");
    }

    #[test]
    fn test_number_mismatch_skips_empty_target() {
        let issue = check_number_mismatch("s4", "3 items", "");
        assert!(issue.is_none());
    }

    #[test]
    fn test_untranslated_empty() {
        let issue = check_untranslated("s5", "", "untranslated");
        assert!(issue.is_some());
    }

    #[test]
    fn test_untranslated_status() {
        let issue = check_untranslated("s6", "번역됨", "untranslated");
        assert!(issue.is_some());
    }

    #[test]
    fn test_translated_no_untranslated_issue() {
        let issue = check_untranslated("s7", "번역됨", "translated");
        assert!(issue.is_none());
    }

    #[test]
    fn test_source_equals_target() {
        let issue = check_source_equals_target("s8", "Hello", "Hello");
        assert!(issue.is_some());
        assert_eq!(issue.unwrap().check_type, "SourceEqualsTarget");
    }

    #[test]
    fn test_source_not_equals_target() {
        let issue = check_source_equals_target("s9", "Hello", "안녕");
        assert!(issue.is_none());
    }
}

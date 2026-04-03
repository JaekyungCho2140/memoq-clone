use crate::models::Segment;
use crate::qa::{run_checks, QaIssue};
use crate::tb::TbEngine;
use tauri::command;

/// QA 체크 실행.
///
/// `segments` — 검사할 세그먼트 목록 (프론트엔드 스토어에서 전달).
/// `tb_id`    — 선택적 TB 식별자. 지정하면 금지 용어 검사에 사용된다.
#[command]
pub async fn run_qa_check(
    segments: Vec<Segment>,
    tb_id: Option<String>,
) -> Result<Vec<QaIssue>, String> {
    let tb_entries = match tb_id {
        Some(id) if !id.is_empty() => {
            let engine = TbEngine::open(&id).map_err(|e| e.to_string())?;
            engine.all_entries().map_err(|e| e.to_string())?
        }
        _ => Vec::new(),
    };

    Ok(run_checks(&segments, &tb_entries))
}

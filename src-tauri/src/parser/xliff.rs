use crate::models::{Project, Segment, SegmentStatus};
use anyhow::Result;
use chrono::Utc;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::fs;
use uuid::Uuid;

pub fn parse(path: &str) -> Result<Project> {
    let content = fs::read_to_string(path)?;
    let mut reader = Reader::from_str(&content);
    reader.config_mut().trim_text(true);

    let mut segments: Vec<Segment> = Vec::new();
    let mut current_source = String::new();
    let mut current_target = String::new();
    let mut in_source = false;
    let mut in_target = false;
    let mut in_trans_unit = false;
    let mut order: u32 = 0;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) => match e.name().as_ref() {
                b"trans-unit" => { in_trans_unit = true; current_source.clear(); current_target.clear(); }
                b"source" if in_trans_unit => in_source = true,
                b"target" if in_trans_unit => in_target = true,
                _ => {}
            },
            Event::Text(e) => {
                let text = e.unescape()?.to_string();
                if in_source { current_source.push_str(&text); }
                if in_target { current_target.push_str(&text); }
            }
            Event::End(e) => match e.name().as_ref() {
                b"source" => in_source = false,
                b"target" => in_target = false,
                b"trans-unit" => {
                    if !current_source.is_empty() {
                        segments.push(Segment {
                            id: Uuid::new_v4().to_string(),
                            source: current_source.clone(),
                            target: current_target.clone(),
                            status: if current_target.is_empty() { SegmentStatus::Untranslated } else { SegmentStatus::Translated },
                            order,
                        });
                        order += 1;
                    }
                    in_trans_unit = false;
                }
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    let file_name = std::path::Path::new(path).file_name().and_then(|n| n.to_str()).unwrap_or("Untitled").to_string();
    Ok(Project {
        id: Uuid::new_v4().to_string(),
        name: file_name,
        source_lang: "en-US".to_string(),
        target_lang: "ko-KR".to_string(),
        created_at: Utc::now(),
        segments,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_simple_xliff() {
        let xliff = r#"<?xml version="1.0" encoding="UTF-8"?>
<xliff version="1.2"><file source-language="en-US" target-language="ko-KR"><body>
  <trans-unit id="1"><source>Hello, world!</source><target>안녕, 세계!</target></trans-unit>
  <trans-unit id="2"><source>Translate me</source><target></target></trans-unit>
</body></file></xliff>"#;
        let mut tmp = NamedTempFile::new().unwrap();
        tmp.write_all(xliff.as_bytes()).unwrap();
        let project = parse(tmp.path().to_str().unwrap()).unwrap();
        assert_eq!(project.segments.len(), 2);
        assert_eq!(project.segments[0].source, "Hello, world!");
        assert_eq!(project.segments[0].status, SegmentStatus::Translated);
        assert_eq!(project.segments[1].status, SegmentStatus::Untranslated);
    }
}

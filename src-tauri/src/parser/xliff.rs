use crate::models::{Project, Segment, SegmentStatus};
use crate::parser::traits::Parser;
use anyhow::{Context, Result};
use chrono::Utc;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::reader::Reader;
use quick_xml::writer::Writer;
use std::collections::HashMap;
use std::fs;
use std::io::BufWriter;
use uuid::Uuid;

// ─── XLIFF version ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum XliffVersion {
    /// XLIFF 1.2 — segments live in `<trans-unit>` elements
    V1,
    /// XLIFF 2.0 — segments live in `<unit>/<segment>` elements
    V2,
}

// ─── Public zero-size struct implementing the Parser trait ────────────────────

#[allow(dead_code)]
pub struct XliffParser;

impl Parser for XliffParser {
    fn parse(&self, path: &str) -> Result<Project> {
        parse(path)
    }
    fn export(&self, segments: &[Segment], input_path: &str, output_path: &str) -> Result<()> {
        export(segments, input_path, output_path)
    }
}

// ─── Public module-level helpers (used by parser/mod.rs) ─────────────────────

/// Parse an XLIFF 1.2 or 2.0 file and return a [`Project`].
pub fn parse(path: &str) -> Result<Project> {
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read XLIFF file: {path}"))?;
    match detect_version(&content)? {
        XliffVersion::V1 => parse_v1(&content, path),
        XliffVersion::V2 => parse_v2(&content, path),
    }
}

/// Write `segments` back into the structure of `input_path` and save the
/// result to `output_path`.  Detects version automatically.
pub fn export(segments: &[Segment], input_path: &str, output_path: &str) -> Result<()> {
    let content = fs::read_to_string(input_path)
        .with_context(|| format!("Failed to read XLIFF file: {input_path}"))?;
    let seg_by_order: HashMap<u32, &Segment> = segments.iter().map(|s| (s.order, s)).collect();
    match detect_version(&content)? {
        XliffVersion::V1 => export_v1(&content, output_path, &seg_by_order),
        XliffVersion::V2 => export_v2(&content, output_path, &seg_by_order),
    }
}

// ─── Version detection ────────────────────────────────────────────────────────

fn detect_version(content: &str) -> Result<XliffVersion> {
    let mut reader = Reader::from_str(content);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(ref e) | Event::Empty(ref e) if e.name().as_ref() == b"xliff" => {
                for attr in e.attributes().flatten() {
                    if attr.key.as_ref() == b"version" {
                        let v = attr.unescape_value()?;
                        return Ok(if v.starts_with("2.") {
                            XliffVersion::V2
                        } else {
                            XliffVersion::V1
                        });
                    }
                }
                return Ok(XliffVersion::V1); // no version attr → assume 1.x
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(XliffVersion::V1)
}

// ─── XLIFF 1.2 parser ─────────────────────────────────────────────────────────

fn parse_v1(content: &str, path: &str) -> Result<Project> {
    let mut reader = Reader::from_str(content);
    reader.config_mut().trim_text(true);

    let mut segments: Vec<Segment> = Vec::new();
    let mut source_lang = "en-US".to_string();
    let mut target_lang = "ko-KR".to_string();
    let mut current_source = String::new();
    let mut current_target = String::new();
    let mut in_source = false;
    let mut in_target = false;
    let mut in_trans_unit = false;
    let mut current_unit_id = String::new();
    let mut order: u32 = 0;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(ref e) => match e.name().as_ref() {
                b"file" => {
                    for attr in e.attributes().flatten() {
                        match attr.key.as_ref() {
                            b"source-language" => {
                                source_lang = attr.unescape_value()?.to_string();
                            }
                            b"target-language" => {
                                target_lang = attr.unescape_value()?.to_string();
                            }
                            _ => {}
                        }
                    }
                }
                b"trans-unit" => {
                    in_trans_unit = true;
                    current_source.clear();
                    current_target.clear();
                    current_unit_id.clear();
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"id" {
                            current_unit_id = attr.unescape_value()?.to_string();
                        }
                    }
                }
                b"source" if in_trans_unit => {
                    in_source = true;
                }
                b"target" if in_trans_unit => {
                    in_target = true;
                }
                _ => {}
            },
            Event::Text(ref e) => {
                let text = e.unescape()?.to_string();
                if in_source {
                    current_source.push_str(&text);
                } else if in_target {
                    current_target.push_str(&text);
                }
            }
            Event::CData(ref e) => {
                let text = std::str::from_utf8(e.as_ref())?.to_string();
                if in_source {
                    current_source.push_str(&text);
                } else if in_target {
                    current_target.push_str(&text);
                }
            }
            Event::End(ref e) => match e.name().as_ref() {
                b"source" => {
                    in_source = false;
                }
                b"target" => {
                    in_target = false;
                }
                b"trans-unit" => {
                    if !current_source.is_empty() {
                        segments.push(Segment {
                            id: if current_unit_id.is_empty() {
                                Uuid::new_v4().to_string()
                            } else {
                                current_unit_id.clone()
                            },
                            source: current_source.clone(),
                            target: current_target.clone(),
                            status: if current_target.is_empty() {
                                SegmentStatus::Untranslated
                            } else {
                                SegmentStatus::Translated
                            },
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

    let name = file_name(path);
    Ok(Project {
        id: Uuid::new_v4().to_string(),
        name,
        source_path: path.to_string(),
        source_lang,
        target_lang,
        created_at: Utc::now(),
        segments,
    })
}

// ─── XLIFF 2.0 parser ─────────────────────────────────────────────────────────

fn parse_v2(content: &str, path: &str) -> Result<Project> {
    let mut reader = Reader::from_str(content);
    reader.config_mut().trim_text(true);

    let mut segments: Vec<Segment> = Vec::new();
    let mut source_lang = "en-US".to_string();
    let mut target_lang = "ko-KR".to_string();
    let mut current_source = String::new();
    let mut current_target = String::new();
    let mut in_source = false;
    let mut in_target = false;
    let mut in_unit = false;
    let mut in_segment = false;
    let mut current_unit_id = String::new();
    let mut order: u32 = 0;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(ref e) => match e.name().as_ref() {
                b"xliff" => {
                    for attr in e.attributes().flatten() {
                        match attr.key.as_ref() {
                            b"srcLang" => {
                                source_lang = attr.unescape_value()?.to_string();
                            }
                            b"trgLang" => {
                                target_lang = attr.unescape_value()?.to_string();
                            }
                            _ => {}
                        }
                    }
                }
                b"unit" => {
                    in_unit = true;
                    current_unit_id.clear();
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"id" {
                            current_unit_id = attr.unescape_value()?.to_string();
                        }
                    }
                }
                b"segment" if in_unit => {
                    in_segment = true;
                    current_source.clear();
                    current_target.clear();
                }
                b"source" if in_segment => {
                    in_source = true;
                }
                b"target" if in_segment => {
                    in_target = true;
                }
                _ => {}
            },
            Event::Text(ref e) => {
                let text = e.unescape()?.to_string();
                if in_source {
                    current_source.push_str(&text);
                } else if in_target {
                    current_target.push_str(&text);
                }
            }
            Event::CData(ref e) => {
                let text = std::str::from_utf8(e.as_ref())?.to_string();
                if in_source {
                    current_source.push_str(&text);
                } else if in_target {
                    current_target.push_str(&text);
                }
            }
            Event::End(ref e) => match e.name().as_ref() {
                b"source" => {
                    in_source = false;
                }
                b"target" => {
                    in_target = false;
                }
                b"segment" => {
                    if in_segment && !current_source.is_empty() {
                        segments.push(Segment {
                            id: if current_unit_id.is_empty() {
                                Uuid::new_v4().to_string()
                            } else {
                                format!("{}-seg-{}", current_unit_id, order)
                            },
                            source: current_source.clone(),
                            target: current_target.clone(),
                            status: if current_target.is_empty() {
                                SegmentStatus::Untranslated
                            } else {
                                SegmentStatus::Translated
                            },
                            order,
                        });
                        order += 1;
                    }
                    in_segment = false;
                }
                b"unit" => {
                    in_unit = false;
                }
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    let name = file_name(path);
    Ok(Project {
        id: Uuid::new_v4().to_string(),
        name,
        source_path: path.to_string(),
        source_lang,
        target_lang,
        created_at: Utc::now(),
        segments,
    })
}

// ─── XLIFF 1.2 export ─────────────────────────────────────────────────────────
//
// Strategy: stream-copy every XML event from the original file, replacing the
// text content inside `<target>` elements with the updated translations.
// If a `<trans-unit>` has no `<target>` element, one is injected just before
// the `</trans-unit>` closing tag.

fn export_v1(content: &str, output_path: &str, seg_map: &HashMap<u32, &Segment>) -> Result<()> {
    let mut reader = Reader::from_str(content);
    reader.config_mut().trim_text(false);

    let file = fs::File::create(output_path)
        .with_context(|| format!("Cannot create output file: {output_path}"))?;
    let mut writer = Writer::new(BufWriter::new(file));

    let mut unit_counter: u32 = 0;
    let mut in_trans_unit = false;
    let mut in_target = false;
    let mut has_target = false;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(ref e) => match e.name().as_ref() {
                b"trans-unit" => {
                    in_trans_unit = true;
                    has_target = false;
                    writer.write_event(Event::Start(e.clone()))?;
                }
                b"target" if in_trans_unit => {
                    in_target = true;
                    has_target = true;
                    writer.write_event(Event::Start(e.clone()))?;
                    // Inject new translation immediately after <target>
                    if let Some(seg) = seg_map.get(&unit_counter) {
                        if !seg.target.is_empty() {
                            writer.write_event(Event::Text(BytesText::new(&seg.target)))?;
                        }
                    }
                }
                _ => {
                    writer.write_event(Event::Start(e.clone()))?;
                }
            },
            Event::End(ref e) => match e.name().as_ref() {
                b"target" if in_target => {
                    in_target = false;
                    writer.write_event(Event::End(e.clone()))?;
                }
                b"trans-unit" => {
                    // Inject missing <target> before closing </trans-unit>
                    if !has_target {
                        if let Some(seg) = seg_map.get(&unit_counter) {
                            if !seg.target.is_empty() {
                                writer.write_event(Event::Start(BytesStart::new("target")))?;
                                writer.write_event(Event::Text(BytesText::new(&seg.target)))?;
                                writer.write_event(Event::End(BytesEnd::new("target")))?;
                            }
                        }
                    }
                    in_trans_unit = false;
                    unit_counter += 1;
                    writer.write_event(Event::End(e.clone()))?;
                }
                _ => {
                    writer.write_event(Event::End(e.clone()))?;
                }
            },
            Event::Text(_) if in_target => {
                // Skip stale target text — we already wrote the new content above
            }
            Event::Eof => break,
            other => {
                writer.write_event(other)?;
            }
        }
        buf.clear();
    }

    Ok(())
}

// ─── XLIFF 2.0 export ─────────────────────────────────────────────────────────

fn export_v2(content: &str, output_path: &str, seg_map: &HashMap<u32, &Segment>) -> Result<()> {
    let mut reader = Reader::from_str(content);
    reader.config_mut().trim_text(false);

    let file = fs::File::create(output_path)
        .with_context(|| format!("Cannot create output file: {output_path}"))?;
    let mut writer = Writer::new(BufWriter::new(file));

    let mut unit_counter: u32 = 0;
    let mut in_unit = false;
    let mut in_segment = false;
    let mut in_target = false;
    let mut has_target = false;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(ref e) => match e.name().as_ref() {
                b"unit" => {
                    in_unit = true;
                    writer.write_event(Event::Start(e.clone()))?;
                }
                b"segment" if in_unit => {
                    in_segment = true;
                    has_target = false;
                    writer.write_event(Event::Start(e.clone()))?;
                }
                b"target" if in_segment => {
                    in_target = true;
                    has_target = true;
                    writer.write_event(Event::Start(e.clone()))?;
                    if let Some(seg) = seg_map.get(&unit_counter) {
                        if !seg.target.is_empty() {
                            writer.write_event(Event::Text(BytesText::new(&seg.target)))?;
                        }
                    }
                }
                _ => {
                    writer.write_event(Event::Start(e.clone()))?;
                }
            },
            Event::End(ref e) => match e.name().as_ref() {
                b"target" if in_target => {
                    in_target = false;
                    writer.write_event(Event::End(e.clone()))?;
                }
                b"segment" if in_segment => {
                    if !has_target {
                        if let Some(seg) = seg_map.get(&unit_counter) {
                            if !seg.target.is_empty() {
                                writer.write_event(Event::Start(BytesStart::new("target")))?;
                                writer.write_event(Event::Text(BytesText::new(&seg.target)))?;
                                writer.write_event(Event::End(BytesEnd::new("target")))?;
                            }
                        }
                    }
                    in_segment = false;
                    unit_counter += 1;
                    writer.write_event(Event::End(e.clone()))?;
                }
                b"unit" => {
                    in_unit = false;
                    writer.write_event(Event::End(e.clone()))?;
                }
                _ => {
                    writer.write_event(Event::End(e.clone()))?;
                }
            },
            Event::Text(_) if in_target => {
                // Skip stale target text
            }
            Event::Eof => break,
            other => {
                writer.write_event(other)?;
            }
        }
        buf.clear();
    }

    Ok(())
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn file_name(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Untitled")
        .to_string()
}

// ─── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // ── helper: write string to a temp file, return the file (keeps it alive) ──
    fn tmp_xliff(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    // ── XLIFF 1.2 parse ───────────────────────────────────────────────────────

    #[test]
    fn test_xliff12_basic_parse() {
        let xliff = r#"<?xml version="1.0" encoding="UTF-8"?>
<xliff version="1.2" xmlns="urn:oasis:names:tc:xliff:document:1.2">
  <file source-language="en-US" target-language="ko-KR">
    <body>
      <trans-unit id="1">
        <source>Hello, world!</source>
        <target>안녕, 세계!</target>
      </trans-unit>
      <trans-unit id="2">
        <source>Translate me</source>
        <target></target>
      </trans-unit>
    </body>
  </file>
</xliff>"#;
        let f = tmp_xliff(xliff);
        let project = parse(f.path().to_str().unwrap()).unwrap();
        assert_eq!(project.segments.len(), 2);
        assert_eq!(project.segments[0].source, "Hello, world!");
        assert_eq!(project.segments[0].target, "안녕, 세계!");
        assert_eq!(project.segments[0].status, SegmentStatus::Translated);
        assert_eq!(project.segments[1].source, "Translate me");
        assert_eq!(project.segments[1].status, SegmentStatus::Untranslated);
        assert_eq!(project.source_lang, "en-US");
        assert_eq!(project.target_lang, "ko-KR");
    }

    #[test]
    fn test_xliff12_segment_ordering() {
        let xliff = r#"<?xml version="1.0"?>
<xliff version="1.2">
  <file source-language="en" target-language="fr">
    <body>
      <trans-unit id="a"><source>First</source></trans-unit>
      <trans-unit id="b"><source>Second</source></trans-unit>
      <trans-unit id="c"><source>Third</source></trans-unit>
    </body>
  </file>
</xliff>"#;
        let f = tmp_xliff(xliff);
        let project = parse(f.path().to_str().unwrap()).unwrap();
        assert_eq!(project.segments.len(), 3);
        assert_eq!(project.segments[0].order, 0);
        assert_eq!(project.segments[1].order, 1);
        assert_eq!(project.segments[2].order, 2);
        assert_eq!(project.segments[0].source, "First");
        assert_eq!(project.segments[2].source, "Third");
    }

    #[test]
    fn test_xliff12_uses_unit_id_as_segment_id() {
        let xliff = r#"<?xml version="1.0"?>
<xliff version="1.2">
  <file source-language="en" target-language="de">
    <body>
      <trans-unit id="seg-42"><source>Test</source></trans-unit>
    </body>
  </file>
</xliff>"#;
        let f = tmp_xliff(xliff);
        let project = parse(f.path().to_str().unwrap()).unwrap();
        assert_eq!(project.segments[0].id, "seg-42");
    }

    // ── XLIFF 2.0 parse ───────────────────────────────────────────────────────

    #[test]
    fn test_xliff20_basic_parse() {
        let xliff = r#"<?xml version="1.0" encoding="UTF-8"?>
<xliff version="2.0"
       xmlns="urn:oasis:names:tc:xliff:document:2.0"
       srcLang="en-US" trgLang="ko-KR">
  <file id="f1">
    <unit id="u1">
      <segment>
        <source>Hello, world!</source>
        <target>안녕, 세계!</target>
      </segment>
    </unit>
    <unit id="u2">
      <segment>
        <source>Translate me</source>
      </segment>
    </unit>
  </file>
</xliff>"#;
        let f = tmp_xliff(xliff);
        let project = parse(f.path().to_str().unwrap()).unwrap();
        assert_eq!(project.segments.len(), 2);
        assert_eq!(project.segments[0].source, "Hello, world!");
        assert_eq!(project.segments[0].target, "안녕, 세계!");
        assert_eq!(project.segments[0].status, SegmentStatus::Translated);
        assert_eq!(project.segments[1].source, "Translate me");
        assert_eq!(project.segments[1].status, SegmentStatus::Untranslated);
        assert_eq!(project.source_lang, "en-US");
        assert_eq!(project.target_lang, "ko-KR");
    }

    #[test]
    fn test_xliff20_multiple_segments_per_unit() {
        let xliff = r#"<?xml version="1.0"?>
<xliff version="2.0" srcLang="en" trgLang="fr">
  <file id="f1">
    <unit id="u1">
      <segment>
        <source>First sentence.</source>
      </segment>
      <segment>
        <source>Second sentence.</source>
      </segment>
    </unit>
  </file>
</xliff>"#;
        let f = tmp_xliff(xliff);
        let project = parse(f.path().to_str().unwrap()).unwrap();
        assert_eq!(project.segments.len(), 2);
        assert_eq!(project.segments[0].source, "First sentence.");
        assert_eq!(project.segments[1].source, "Second sentence.");
        assert_eq!(project.segments[0].order, 0);
        assert_eq!(project.segments[1].order, 1);
    }

    // ── XLIFF 1.2 export ──────────────────────────────────────────────────────

    #[test]
    fn test_xliff12_export_replaces_target() {
        let xliff = r#"<?xml version="1.0" encoding="UTF-8"?>
<xliff version="1.2">
  <file source-language="en-US" target-language="ko-KR">
    <body>
      <trans-unit id="1">
        <source>Hello</source>
        <target>Old translation</target>
      </trans-unit>
    </body>
  </file>
</xliff>"#;
        let input = tmp_xliff(xliff);
        let output = NamedTempFile::new().unwrap();

        // Build segment with new translation
        let segments = vec![Segment {
            id: "1".to_string(),
            source: "Hello".to_string(),
            target: "New translation".to_string(),
            status: SegmentStatus::Confirmed,
            order: 0,
        }];

        export(
            &segments,
            input.path().to_str().unwrap(),
            output.path().to_str().unwrap(),
        )
        .unwrap();

        let result = fs::read_to_string(output.path()).unwrap();
        assert!(
            result.contains("New translation"),
            "Expected new translation in output"
        );
        assert!(
            !result.contains("Old translation"),
            "Old translation must be removed"
        );
        assert!(
            result.contains("<source>Hello</source>"),
            "Source must be preserved"
        );
    }

    #[test]
    fn test_xliff12_export_adds_missing_target() {
        let xliff = r#"<?xml version="1.0"?>
<xliff version="1.2">
  <file source-language="en" target-language="de">
    <body>
      <trans-unit id="1">
        <source>No target yet</source>
      </trans-unit>
    </body>
  </file>
</xliff>"#;
        let input = tmp_xliff(xliff);
        let output = NamedTempFile::new().unwrap();

        let segments = vec![Segment {
            id: "1".to_string(),
            source: "No target yet".to_string(),
            target: "Noch kein Ziel".to_string(),
            status: SegmentStatus::Translated,
            order: 0,
        }];

        export(
            &segments,
            input.path().to_str().unwrap(),
            output.path().to_str().unwrap(),
        )
        .unwrap();

        let result = fs::read_to_string(output.path()).unwrap();
        assert!(result.contains("<target>Noch kein Ziel</target>"));
    }

    #[test]
    fn test_xliff12_export_preserves_source_unchanged() {
        let xliff = r#"<?xml version="1.0"?>
<xliff version="1.2">
  <file source-language="en" target-language="fr">
    <body>
      <trans-unit id="1">
        <source>Keep this source</source>
        <target>old</target>
      </trans-unit>
    </body>
  </file>
</xliff>"#;
        let input = tmp_xliff(xliff);
        let output = NamedTempFile::new().unwrap();

        let segments = vec![Segment {
            id: "1".to_string(),
            source: "Keep this source".to_string(),
            target: "nouveau".to_string(),
            status: SegmentStatus::Translated,
            order: 0,
        }];

        export(
            &segments,
            input.path().to_str().unwrap(),
            output.path().to_str().unwrap(),
        )
        .unwrap();

        let result = fs::read_to_string(output.path()).unwrap();
        assert!(result.contains("Keep this source"));
        assert!(result.contains("nouveau"));
        assert!(!result.contains(">old<"));
    }

    // ── XLIFF 2.0 export ──────────────────────────────────────────────────────

    #[test]
    fn test_xliff20_export_replaces_target() {
        let xliff = r#"<?xml version="1.0" encoding="UTF-8"?>
<xliff version="2.0" srcLang="en" trgLang="ko">
  <file id="f1">
    <unit id="u1">
      <segment>
        <source>Hello</source>
        <target>Old</target>
      </segment>
    </unit>
  </file>
</xliff>"#;
        let input = tmp_xliff(xliff);
        let output = NamedTempFile::new().unwrap();

        let segments = vec![Segment {
            id: "u1-seg-0".to_string(),
            source: "Hello".to_string(),
            target: "안녕하세요".to_string(),
            status: SegmentStatus::Confirmed,
            order: 0,
        }];

        export(
            &segments,
            input.path().to_str().unwrap(),
            output.path().to_str().unwrap(),
        )
        .unwrap();

        let result = fs::read_to_string(output.path()).unwrap();
        assert!(result.contains("안녕하세요"));
        assert!(!result.contains(">Old<"));
        assert!(result.contains("<source>Hello</source>"));
    }

    #[test]
    fn test_xliff20_export_adds_missing_target() {
        let xliff = r#"<?xml version="1.0"?>
<xliff version="2.0" srcLang="en" trgLang="ja">
  <file id="f1">
    <unit id="u1">
      <segment>
        <source>Untranslated</source>
      </segment>
    </unit>
  </file>
</xliff>"#;
        let input = tmp_xliff(xliff);
        let output = NamedTempFile::new().unwrap();

        let segments = vec![Segment {
            id: "u1-seg-0".to_string(),
            source: "Untranslated".to_string(),
            target: "未翻訳".to_string(),
            status: SegmentStatus::Translated,
            order: 0,
        }];

        export(
            &segments,
            input.path().to_str().unwrap(),
            output.path().to_str().unwrap(),
        )
        .unwrap();

        let result = fs::read_to_string(output.path()).unwrap();
        assert!(result.contains("未翻訳"));
    }

    // ── Parser trait ──────────────────────────────────────────────────────────

    #[test]
    fn test_xliff_parser_trait_roundtrip() {
        let parser = XliffParser;
        let xliff = r#"<?xml version="1.0"?>
<xliff version="1.2">
  <file source-language="en" target-language="ko">
    <body>
      <trans-unit id="1"><source>Trait test</source><target>트레이트 테스트</target></trans-unit>
    </body>
  </file>
</xliff>"#;
        let f = tmp_xliff(xliff);
        let project = parser.parse(f.path().to_str().unwrap()).unwrap();
        assert_eq!(project.segments.len(), 1);
        assert_eq!(project.segments[0].source, "Trait test");

        let output = NamedTempFile::new().unwrap();
        let updated = vec![Segment {
            id: "1".to_string(),
            source: "Trait test".to_string(),
            target: "트레이트 테스트 업데이트".to_string(),
            status: SegmentStatus::Confirmed,
            order: 0,
        }];
        parser
            .export(
                &updated,
                f.path().to_str().unwrap(),
                output.path().to_str().unwrap(),
            )
            .unwrap();
        let out_content = fs::read_to_string(output.path()).unwrap();
        assert!(out_content.contains("트레이트 테스트 업데이트"));
    }

    // ── Version detection ─────────────────────────────────────────────────────

    #[test]
    fn test_detect_v1() {
        let xliff = r#"<xliff version="1.2"></xliff>"#;
        assert_eq!(detect_version(xliff).unwrap(), XliffVersion::V1);
    }

    #[test]
    fn test_detect_v2() {
        let xliff = r#"<xliff version="2.0" srcLang="en" trgLang="fr"></xliff>"#;
        assert_eq!(detect_version(xliff).unwrap(), XliffVersion::V2);
    }

    #[test]
    fn test_detect_no_version_defaults_to_v1() {
        let xliff = r#"<xliff><file></file></xliff>"#;
        assert_eq!(detect_version(xliff).unwrap(), XliffVersion::V1);
    }
}

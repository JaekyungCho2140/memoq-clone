use crate::models::{Project, Segment, SegmentStatus};
use crate::parser::traits::Parser;
use anyhow::{Context, Result};
use chrono::Utc;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::io::{Read, Write};
use uuid::Uuid;
use zip::write::ZipWriter;
use zip::ZipArchive;

#[allow(dead_code)]
pub struct DocxParser;

impl Parser for DocxParser {
    fn parse(&self, path: &str) -> Result<Project> {
        parse(path)
    }
    fn export(&self, segments: &[Segment], input_path: &str, output_path: &str) -> Result<()> {
        export(segments, input_path, output_path)
    }
}

/// Extract non-empty paragraph texts from word/document.xml.
fn extract_paragraphs(xml: &str) -> Vec<String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);

    let mut paragraphs: Vec<String> = Vec::new();
    let mut current_para = String::new();
    let mut in_para = false;
    let mut in_text = false;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let ln = e.local_name();
                let name = std::str::from_utf8(ln.as_ref()).unwrap_or("");
                match name {
                    "p" => {
                        in_para = true;
                        current_para.clear();
                    }
                    "t" => {
                        in_text = true;
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                let ln = e.local_name();
                let name = std::str::from_utf8(ln.as_ref()).unwrap_or("");
                match name {
                    "p" => {
                        if in_para {
                            let text = current_para.trim().to_string();
                            if !text.is_empty() {
                                paragraphs.push(text);
                            }
                            in_para = false;
                        }
                    }
                    "t" => {
                        in_text = false;
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(e)) => {
                if in_text && in_para {
                    if let Ok(s) = e.unescape() {
                        current_para.push_str(&s);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    paragraphs
}

pub fn parse(path: &str) -> Result<Project> {
    let file = std::fs::File::open(path).with_context(|| format!("Cannot open DOCX: {path}"))?;
    let mut archive =
        ZipArchive::new(file).with_context(|| format!("Not a valid ZIP/DOCX: {path}"))?;

    let xml = {
        let mut entry = archive
            .by_name("word/document.xml")
            .context("Missing word/document.xml in DOCX")?;
        let mut s = String::new();
        entry.read_to_string(&mut s)?;
        s
    };

    let paragraphs = extract_paragraphs(&xml);

    let segments: Vec<Segment> = paragraphs
        .into_iter()
        .enumerate()
        .map(|(i, text)| Segment {
            id: Uuid::new_v4().to_string(),
            source: text,
            target: String::new(),
            status: SegmentStatus::Untranslated,
            order: i as u32,
        })
        .collect();

    let name = std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("document")
        .to_string();

    Ok(Project {
        id: Uuid::new_v4().to_string(),
        name,
        source_path: path.to_string(),
        source_lang: "en-US".to_string(),
        target_lang: "ko-KR".to_string(),
        created_at: Utc::now(),
        segments,
    })
}

/// Rebuild the DOCX replacing paragraph text with translated segments.
pub fn export(segments: &[Segment], input_path: &str, output_path: &str) -> Result<()> {
    let file = std::fs::File::open(input_path)
        .with_context(|| format!("Cannot open source DOCX: {input_path}"))?;
    let mut archive = ZipArchive::new(file)?;

    let out_file = std::fs::File::create(output_path)
        .with_context(|| format!("Cannot create output file: {output_path}"))?;
    let mut writer = ZipWriter::new(out_file);

    // segment order -> translated target
    let seg_map: std::collections::HashMap<u32, &str> = segments
        .iter()
        .map(|s| (s.order, s.target.as_str()))
        .collect();

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let entry_name = entry.name().to_string();
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        if entry_name == "word/document.xml" {
            let mut xml = String::new();
            entry.read_to_string(&mut xml)?;
            let patched = patch_document_xml(&xml, &seg_map)?;
            writer.start_file(&entry_name, options)?;
            writer.write_all(patched.as_bytes())?;
        } else {
            let mut content = Vec::new();
            entry.read_to_end(&mut content)?;
            writer.start_file(&entry_name, options)?;
            writer.write_all(&content)?;
        }
    }
    writer.finish()?;
    Ok(())
}

/// Build a map from XML paragraph index (0-based) -> segment order,
/// only for paragraphs that have non-empty text (those become segments).
fn build_para_segment_map(xml: &str) -> std::collections::HashMap<u32, u32> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut map = std::collections::HashMap::new();
    let mut xml_para_idx: u32 = 0;
    let mut seg_order: u32 = 0;
    let mut current_text = String::new();
    let mut in_para = false;
    let mut in_text = false;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let ln = e.local_name();
                let local = std::str::from_utf8(ln.as_ref()).unwrap_or("");
                match local {
                    "p" => {
                        in_para = true;
                        current_text.clear();
                    }
                    "t" => {
                        in_text = true;
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                let ln = e.local_name();
                let local = std::str::from_utf8(ln.as_ref()).unwrap_or("");
                match local {
                    "p" => {
                        if in_para && !current_text.trim().is_empty() {
                            map.insert(xml_para_idx, seg_order);
                            seg_order += 1;
                        }
                        xml_para_idx += 1;
                        in_para = false;
                        current_text.clear();
                    }
                    "t" => {
                        in_text = false;
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(e)) => {
                if in_text && in_para {
                    if let Ok(s) = e.unescape() {
                        current_text.push_str(&s);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    map
}

/// Replace text in each paragraph with the translated segment target.
/// Strategy: keep all XML structure, but within each paragraph's first <w:r>,
/// replace all <w:t> content with the translated text and drop subsequent runs.
fn patch_document_xml(xml: &str, seg_map: &std::collections::HashMap<u32, &str>) -> Result<String> {
    let para_to_seg = build_para_segment_map(xml);

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut output = String::with_capacity(xml.len() + 1024);

    let mut xml_para_idx: u32 = 0;
    // State flags
    let mut in_para = false;
    let mut first_run_written = false; // have we written the first <w:r>...<w:t>translated</w:t></w:r>?
    let mut in_run = false;
    let mut in_text = false;
    let mut skip_run = false; // skip subsequent runs after first
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let ln = e.local_name();
                let local = std::str::from_utf8(ln.as_ref()).unwrap_or("").to_owned();
                match local.as_str() {
                    "p" => {
                        in_para = true;
                        first_run_written = false;
                        output.push_str(&event_to_start_tag(e));
                    }
                    "r" if in_para => {
                        in_run = true;
                        if first_run_written {
                            // skip this run entirely
                            skip_run = true;
                        } else {
                            output.push_str(&event_to_start_tag(e));
                        }
                    }
                    "t" if in_run && in_para => {
                        in_text = true;
                        if !skip_run {
                            output.push_str(&event_to_start_tag(e));
                        }
                    }
                    _ => {
                        if !skip_run {
                            output.push_str(&event_to_start_tag(e));
                        }
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let ln = e.local_name();
                let local = std::str::from_utf8(ln.as_ref()).unwrap_or("").to_owned();
                match local.as_str() {
                    "p" => {
                        output.push_str("</");
                        output.push_str(std::str::from_utf8(e.name().as_ref()).unwrap_or("p"));
                        output.push('>');
                        in_para = false;
                        first_run_written = false;
                        skip_run = false;
                        xml_para_idx += 1;
                    }
                    "r" if in_para => {
                        in_run = false;
                        if skip_run {
                            skip_run = false;
                        } else {
                            output.push_str("</");
                            output.push_str(std::str::from_utf8(e.name().as_ref()).unwrap_or("r"));
                            output.push('>');
                            first_run_written = true;
                        }
                    }
                    "t" if in_para => {
                        in_text = false;
                        if !skip_run {
                            output.push_str("</");
                            output.push_str(std::str::from_utf8(e.name().as_ref()).unwrap_or("t"));
                            output.push('>');
                        }
                    }
                    _ => {
                        if !skip_run {
                            output.push_str("</");
                            output.push_str(std::str::from_utf8(e.name().as_ref()).unwrap_or(""));
                            output.push('>');
                        }
                    }
                }
            }
            Ok(Event::Text(e)) => {
                if in_text && in_para && !skip_run {
                    // Use current para idx to find segment order, then translated text
                    if let Some(&seg_order) = para_to_seg.get(&xml_para_idx) {
                        if let Some(&translated) = seg_map.get(&seg_order) {
                            let text = if translated.is_empty() {
                                // Fall back to original source text
                                e.unescape().unwrap_or_default().to_string()
                            } else {
                                translated.to_string()
                            };
                            output.push_str(&xml_escape(&text));
                            continue;
                        }
                    }
                    if let Ok(s) = e.unescape() {
                        output.push_str(&xml_escape(&s));
                    }
                } else if !skip_run {
                    if let Ok(s) = e.unescape() {
                        output.push_str(&xml_escape(&s));
                    }
                }
            }
            Ok(Event::Empty(ref e)) => {
                if !skip_run {
                    let mut tag = String::from("<");
                    tag.push_str(std::str::from_utf8(e.name().as_ref()).unwrap_or(""));
                    for attr in e.attributes().flatten() {
                        tag.push(' ');
                        tag.push_str(std::str::from_utf8(attr.key.as_ref()).unwrap_or(""));
                        tag.push_str("=\"");
                        if let Ok(v) = attr.unescape_value() {
                            tag.push_str(&xml_escape(&v));
                        }
                        tag.push('"');
                    }
                    tag.push_str("/>");
                    output.push_str(&tag);
                }
            }
            Ok(Event::Decl(e)) => {
                output.push_str("<?xml");
                if let Ok(v) = e.version() {
                    output.push_str(" version=\"");
                    output.push_str(std::str::from_utf8(v.as_ref()).unwrap_or("1.0"));
                    output.push('"');
                }
                if let Some(Ok(enc)) = e.encoding() {
                    output.push_str(" encoding=\"");
                    output.push_str(std::str::from_utf8(enc.as_ref()).unwrap_or("UTF-8"));
                    output.push('"');
                }
                output.push_str("?>");
            }
            Ok(Event::Comment(e)) => {
                output.push_str("<!--");
                output.push_str(std::str::from_utf8(e.as_ref()).unwrap_or(""));
                output.push_str("-->");
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(output)
}

fn event_to_start_tag(e: &quick_xml::events::BytesStart) -> String {
    let mut tag = String::from("<");
    tag.push_str(std::str::from_utf8(e.name().as_ref()).unwrap_or(""));
    for attr in e.attributes().flatten() {
        tag.push(' ');
        tag.push_str(std::str::from_utf8(attr.key.as_ref()).unwrap_or(""));
        tag.push_str("=\"");
        if let Ok(v) = attr.unescape_value() {
            tag.push_str(&xml_escape(&v));
        }
        tag.push('"');
    }
    tag.push('>');
    tag
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_paragraphs() {
        let xml = r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>Hello world</w:t></w:r></w:p>
    <w:p><w:r><w:t xml:space="preserve">Another paragraph</w:t></w:r></w:p>
    <w:p></w:p>
  </w:body>
</w:document>"#;
        let paras = extract_paragraphs(xml);
        assert_eq!(paras.len(), 2);
        assert_eq!(paras[0], "Hello world");
        assert_eq!(paras[1], "Another paragraph");
    }

    #[test]
    fn test_build_para_segment_map() {
        let xml = r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>First</w:t></w:r></w:p>
    <w:p></w:p>
    <w:p><w:r><w:t>Third</w:t></w:r></w:p>
  </w:body>
</w:document>"#;
        let map = build_para_segment_map(xml);
        // XML para 0 -> segment 0, XML para 2 -> segment 1
        assert_eq!(map.get(&0), Some(&0));
        assert_eq!(map.get(&2), Some(&1));
        assert_eq!(map.get(&1), None); // empty para, no segment
    }
}

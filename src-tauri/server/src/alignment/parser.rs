//! File parsers for alignment input.
//!
//! Extracts a flat list of sentences from TXT, XLIFF, or DOCX uploads.

use anyhow::{bail, Context, Result};

/// Parse raw file bytes into a list of sentences.
///
/// `filename` is used only for format detection.
pub fn extract_sentences(bytes: &[u8], filename: &str) -> Result<Vec<String>> {
    let lower = filename.to_lowercase();
    if lower.ends_with(".xliff") || lower.ends_with(".xlf") {
        parse_xliff(bytes)
    } else if lower.ends_with(".docx") {
        parse_docx(bytes)
    } else if lower.ends_with(".txt") {
        parse_txt(bytes)
    } else {
        bail!(
            "Unsupported file format: '{}'.  Accepted: .txt, .xliff, .xlf, .docx",
            filename
        )
    }
}

// ── TXT ─────────────────────────────────────────────────────────────────────

fn parse_txt(bytes: &[u8]) -> Result<Vec<String>> {
    let text = std::str::from_utf8(bytes).context("TXT file is not valid UTF-8")?;
    Ok(text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect())
}

// ── XLIFF ────────────────────────────────────────────────────────────────────

fn parse_xliff(bytes: &[u8]) -> Result<Vec<String>> {
    use quick_xml::{events::Event, Reader};
    let text = std::str::from_utf8(bytes).context("XLIFF file is not valid UTF-8")?;
    let mut reader = Reader::from_str(text);
    reader.config_mut().trim_text(true);

    let mut sentences = Vec::new();
    let mut in_source = false;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                if e.local_name().as_ref() == b"source" {
                    in_source = true;
                }
            }
            Ok(Event::End(ref e)) => {
                if e.local_name().as_ref() == b"source" {
                    in_source = false;
                }
            }
            Ok(Event::Text(ref e)) => {
                if in_source {
                    let s = e.unescape().unwrap_or_default().trim().to_string();
                    if !s.is_empty() {
                        sentences.push(s);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => bail!("XLIFF parse error: {}", e),
            _ => {}
        }
        buf.clear();
    }
    Ok(sentences)
}

// ── DOCX ─────────────────────────────────────────────────────────────────────

fn parse_docx(bytes: &[u8]) -> Result<Vec<String>> {
    use std::io::Cursor;
    use zip::ZipArchive;

    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor).context("Failed to open DOCX as ZIP")?;

    // The main document XML is word/document.xml
    let doc_xml = {
        let mut entry = archive
            .by_name("word/document.xml")
            .context("word/document.xml not found in DOCX")?;
        let mut buf = Vec::new();
        std::io::Read::read_to_end(&mut entry, &mut buf)?;
        buf
    };

    extract_docx_paragraphs(&doc_xml)
}

fn extract_docx_paragraphs(xml: &[u8]) -> Result<Vec<String>> {
    use quick_xml::{events::Event, Reader};
    let text = std::str::from_utf8(xml).context("DOCX XML is not valid UTF-8")?;
    let mut reader = Reader::from_str(text);
    reader.config_mut().trim_text(false);

    let mut sentences = Vec::new();
    let mut current = String::new();
    let mut in_text = false;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let local = e.local_name();
                match local.as_ref() {
                    b"p" => current.clear(),
                    b"t" => in_text = true,
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                let local = e.local_name();
                match local.as_ref() {
                    b"p" => {
                        let s = current.trim().to_string();
                        if !s.is_empty() {
                            sentences.push(s);
                        }
                    }
                    b"t" => in_text = false,
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) => {
                if in_text {
                    current.push_str(&e.unescape().unwrap_or_default());
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => bail!("DOCX XML parse error: {}", e),
            _ => {}
        }
        buf.clear();
    }
    Ok(sentences)
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_txt_basic() {
        let input = b"Hello world.\nGoodbye.\n\nThird line.";
        let result = extract_sentences(input, "test.txt").unwrap();
        assert_eq!(result, vec!["Hello world.", "Goodbye.", "Third line."]);
    }

    #[test]
    fn parse_txt_trims_whitespace() {
        let input = b"  hello  \n  world  ";
        let result = extract_sentences(input, "test.txt").unwrap();
        assert_eq!(result, vec!["hello", "world"]);
    }

    #[test]
    fn parse_xliff_extracts_sources() {
        let xliff = "<?xml version=\"1.0\"?>\n\
<xliff version=\"1.2\">\n\
  <file>\n\
    <body>\n\
      <trans-unit id=\"1\"><source>Hello</source><target>Hola</target></trans-unit>\n\
      <trans-unit id=\"2\"><source>World</source><target>Mundo</target></trans-unit>\n\
    </body>\n\
  </file>\n\
</xliff>";
        let result = extract_sentences(xliff.as_bytes(), "doc.xliff").unwrap();
        assert_eq!(result, vec!["Hello", "World"]);
    }

    #[test]
    fn unsupported_format_returns_error() {
        let result = extract_sentences(b"data", "file.pdf");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported"));
    }
}

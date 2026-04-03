use quick_xml::{events::Event, Reader};

#[derive(Debug, Clone)]
pub struct ParsedSegment {
    pub source: String,
    pub target: String,
}

/// Parse XLIFF 1.2 bytes and return a list of (source, target) pairs.
pub fn parse_xliff(data: &[u8]) -> anyhow::Result<Vec<ParsedSegment>> {
    let mut reader = Reader::from_reader(data);
    reader.config_mut().trim_text(true);

    let mut segments = Vec::new();
    let mut buf = Vec::new();

    enum State {
        Outside,
        InSource,
        InTarget,
    }

    let mut state = State::Outside;
    let mut current_source = String::new();
    let mut current_target = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => match e.name().as_ref() {
                b"source" => state = State::InSource,
                b"target" => state = State::InTarget,
                b"trans-unit" => {
                    current_source.clear();
                    current_target.clear();
                }
                _ => {}
            },
            Ok(Event::End(ref e)) => match e.name().as_ref() {
                b"source" => state = State::Outside,
                b"target" => state = State::Outside,
                b"trans-unit" => {
                    if !current_source.is_empty() {
                        segments.push(ParsedSegment {
                            source: current_source.trim().to_string(),
                            target: current_target.trim().to_string(),
                        });
                    }
                    current_source.clear();
                    current_target.clear();
                }
                _ => {}
            },
            Ok(Event::Text(e)) => {
                let text = e.unescape().unwrap_or_default().into_owned();
                match state {
                    State::InSource => current_source.push_str(&text),
                    State::InTarget => current_target.push_str(&text),
                    State::Outside => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(anyhow::anyhow!("XLIFF parse error: {}", e)),
            _ => {}
        }
        buf.clear();
    }

    Ok(segments)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_xliff_basic() {
        let xliff = r#"<?xml version="1.0"?>
<xliff version="1.2">
  <file>
    <body>
      <trans-unit id="1">
        <source>Hello world</source>
        <target>안녕 세계</target>
      </trans-unit>
      <trans-unit id="2">
        <source>Goodbye</source>
        <target></target>
      </trans-unit>
    </body>
  </file>
</xliff>"#;

        let segs = parse_xliff(xliff.as_bytes()).unwrap();
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].source, "Hello world");
        assert_eq!(segs[0].target, "안녕 세계");
        assert_eq!(segs[1].source, "Goodbye");
        assert_eq!(segs[1].target, "");
    }
}

use crate::livedocs::{LiveDocsDocument, LiveDocsError, LiveDocsLibrary};
use std::path::{Path, PathBuf};
use uuid::Uuid;

const LIVEDOCS_DIR: &str = ".memoq-clone/livedocs";

fn livedocs_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(LIVEDOCS_DIR))
}

fn library_path(lib_id: &str) -> Option<PathBuf> {
    livedocs_dir().map(|d| d.join(format!("{}.json", lib_id)))
}

fn ensure_livedocs_dir() -> Result<PathBuf, LiveDocsError> {
    let dir = livedocs_dir().ok_or_else(|| {
        LiveDocsError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Cannot determine home directory",
        ))
    })?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

// ── Text extraction ─────────────────────────────────────────────────────────

fn extract_text_txt(path: &str) -> Result<String, LiveDocsError> {
    Ok(std::fs::read_to_string(path)?)
}

fn extract_text_docx(path: &str) -> Result<String, LiveDocsError> {
    use std::io::Read;
    let file = std::fs::File::open(path)?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| LiveDocsError::Parse(e.to_string()))?;

    let mut text = String::new();
    if let Ok(mut entry) = archive.by_name("word/document.xml") {
        let mut content = String::new();
        entry.read_to_string(&mut content)?;
        // Strip XML tags to get plain text
        let mut in_tag = false;
        for ch in content.chars() {
            match ch {
                '<' => in_tag = true,
                '>' => {
                    in_tag = false;
                    text.push(' ');
                }
                _ if !in_tag => text.push(ch),
                _ => {}
            }
        }
    }
    Ok(text)
}

fn extract_text_pdf(path: &str) -> Result<String, LiveDocsError> {
    let bytes = std::fs::read(path)?;
    pdf_extract::extract_text_from_mem(&bytes).map_err(|e| LiveDocsError::Parse(e.to_string()))
}

pub fn extract_text(path: &str) -> Result<String, LiveDocsError> {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "txt" => extract_text_txt(path),
        "docx" => extract_text_docx(path),
        "pdf" => extract_text_pdf(path),
        other => Err(LiveDocsError::UnsupportedFormat(other.to_string())),
    }
}

// ── Sentence splitting ──────────────────────────────────────────────────────

pub fn split_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current = String::new();

    for ch in text.chars() {
        current.push(ch);
        if matches!(ch, '.' | '!' | '?') {
            let trimmed = current.trim().to_string();
            if trimmed.len() > 10 {
                sentences.push(trimmed);
            }
            current.clear();
        }
    }

    let trimmed = current.trim().to_string();
    if trimmed.len() > 10 {
        sentences.push(trimmed);
    }

    sentences
}

// ── Library persistence ─────────────────────────────────────────────────────

pub fn save_library(lib: &LiveDocsLibrary) -> Result<(), LiveDocsError> {
    ensure_livedocs_dir()?;
    let path = library_path(&lib.id).ok_or_else(|| {
        LiveDocsError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Cannot determine library path",
        ))
    })?;
    let json = serde_json::to_string_pretty(lib)?;
    std::fs::write(path, json)?;
    Ok(())
}

pub fn load_library(lib_id: &str) -> Result<LiveDocsLibrary, LiveDocsError> {
    let path =
        library_path(lib_id).ok_or_else(|| LiveDocsError::LibraryNotFound(lib_id.to_string()))?;
    let json = std::fs::read_to_string(&path)
        .map_err(|_| LiveDocsError::LibraryNotFound(lib_id.to_string()))?;
    Ok(serde_json::from_str(&json)?)
}

pub fn list_libraries() -> Result<Vec<LiveDocsLibrary>, LiveDocsError> {
    let dir = match livedocs_dir() {
        Some(d) if d.exists() => d,
        _ => return Ok(Vec::new()),
    };

    let mut libs = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            if let Ok(json) = std::fs::read_to_string(&path) {
                if let Ok(lib) = serde_json::from_str::<LiveDocsLibrary>(&json) {
                    libs.push(lib);
                }
            }
        }
    }
    Ok(libs)
}

// ── Public API ──────────────────────────────────────────────────────────────

pub fn create_library(name: &str) -> Result<LiveDocsLibrary, LiveDocsError> {
    let lib = LiveDocsLibrary {
        id: Uuid::new_v4().to_string(),
        name: name.to_string(),
        documents: Vec::new(),
    };
    save_library(&lib)?;
    Ok(lib)
}

pub fn add_document(lib_id: &str, path: &str) -> Result<LiveDocsLibrary, LiveDocsError> {
    let mut lib = load_library(lib_id)?;
    let text = extract_text(path)?;
    let sentences = split_sentences(&text);
    lib.documents.push(LiveDocsDocument {
        id: Uuid::new_v4().to_string(),
        path: path.to_string(),
        sentences,
    });
    save_library(&lib)?;
    Ok(lib)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_split_sentences_basic() {
        let text = "Hello world. This is a test. Another sentence!";
        let sentences = split_sentences(text);
        assert_eq!(sentences.len(), 3);
        assert!(sentences[0].contains("Hello world"));
    }

    #[test]
    fn test_split_sentences_short_fragments_ignored() {
        let text = "Hi. This is a proper sentence that is long enough.";
        let sentences = split_sentences(text);
        // "Hi." is too short (3 chars), only the second one
        assert_eq!(sentences.len(), 1);
        assert!(sentences[0].contains("proper sentence"));
    }

    #[test]
    fn test_extract_text_txt() {
        let mut f = NamedTempFile::with_suffix(".txt").unwrap();
        use std::io::Write;
        writeln!(f, "Hello world. This is a test document.").unwrap();
        let text = extract_text(f.path().to_str().unwrap()).unwrap();
        assert!(text.contains("Hello world"));
    }

    #[test]
    fn test_unsupported_format() {
        let result = extract_text("/path/to/file.xyz");
        assert!(matches!(result, Err(LiveDocsError::UnsupportedFormat(_))));
    }

    #[test]
    fn test_split_sentences_empty() {
        let sentences = split_sentences("");
        assert!(sentences.is_empty());
    }
}

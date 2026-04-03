mod xliff;

use crate::models::Project;
use anyhow::{Result, bail};
use std::path::Path;

pub fn parse(path: &str) -> Result<Project> {
    let p = Path::new(path);
    match p.extension().and_then(|e| e.to_str()) {
        Some("xliff") | Some("xlf") => xliff::parse(path),
        Some("docx") => bail!("DOCX parser not yet implemented"),
        Some(ext) => bail!("Unsupported file extension: .{ext}"),
        None => bail!("File has no extension: {path}"),
    }
}

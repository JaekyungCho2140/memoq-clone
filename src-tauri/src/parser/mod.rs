pub mod traits;
mod xliff;

#[allow(unused_imports)]
pub use traits::Parser;
#[allow(unused_imports)]
pub use xliff::XliffParser;

use crate::models::{Project, Segment};
use anyhow::{bail, Result};
use std::path::Path;

/// Parse a file based on its extension.  Returns a [`Project`] with all segments.
pub fn parse(path: &str) -> Result<Project> {
    match Path::new(path).extension().and_then(|e| e.to_str()) {
        Some("xliff") | Some("xlf") => xliff::parse(path),
        Some("docx") => bail!("DOCX parser not yet implemented"),
        Some(ext) => bail!("Unsupported file extension: .{ext}"),
        None => bail!("File has no extension: {path}"),
    }
}

/// Export `segments` back into the file at `input_path`, writing the result to
/// `output_path`.  The original file is never overwritten.
pub fn export(segments: &[Segment], input_path: &str, output_path: &str) -> Result<()> {
    match Path::new(input_path).extension().and_then(|e| e.to_str()) {
        Some("xliff") | Some("xlf") => xliff::export(segments, input_path, output_path),
        Some("docx") => bail!("DOCX export not yet implemented"),
        Some(ext) => bail!("Unsupported file extension: .{ext}"),
        None => bail!("File has no extension: {input_path}"),
    }
}

use crate::models::{Project, Segment};
use anyhow::Result;

/// Common interface implemented by every file-format parser.
///
/// Each implementor handles one file format (e.g. XLIFF, DOCX) and provides:
/// - `parse`  — read a source file and return a `Project` with its segments
/// - `export` — write translated segments back in the original format
pub trait Parser: Send + Sync {
    /// Parse the file at `path` and return a [`Project`] containing all segments.
    fn parse(&self, path: &str) -> Result<Project>;

    /// Export translated `segments` by merging them into `input_path` and
    /// writing the result to `output_path`.  The original file is never
    /// overwritten.
    fn export(&self, segments: &[Segment], input_path: &str, output_path: &str) -> Result<()>;
}

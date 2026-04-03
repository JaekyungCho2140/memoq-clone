//! Plugin API — shared types and ABI for memoq-clone WASM plugins.
//!
//! Plugin authors compile their crates to `wasm32-wasip1` and export
//! functions conforming to the ABI defined here.  The host runtime
//! (wasmtime) calls these exported functions via JSON-serialized
//! request/response values passed through linear memory.
//!
//! # Plugin ABI contract
//!
//! Every plugin WASM module MUST export:
//! - `plugin_metadata() -> *mut u8`  — returns JSON-encoded [`PluginMetadata`]
//! - one or more capability exports depending on [`PluginKind`]:
//!   - `mt_translate(ptr: i32, len: i32) -> i64`   (MtProvider)
//!   - `parse_file(ptr: i32, len: i32) -> i64`     (FileParser)
//!   - `qa_check(ptr: i32, len: i32) -> i64`       (QaRule)
//!
//! Return values encode `(ptr << 32 | len)` of a heap-allocated JSON
//! UTF-8 string.  The host reads the bytes then calls `dealloc(ptr, len)`
//! to free them.
//!
//! Memory helpers that MUST be exported:
//! - `alloc(size: i32) -> i32`
//! - `dealloc(ptr: i32, len: i32)`

use serde::{Deserialize, Serialize};

// ──────────────────────────────────────────────
// Metadata
// ──────────────────────────────────────────────

/// Describes a plugin and its capabilities.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PluginMetadata {
    /// Unique reverse-DNS identifier, e.g. `"com.example.deepl-mt"`.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Semantic version string.
    pub version: String,
    /// Plugin author.
    pub author: String,
    /// Short description shown in the plugin manager.
    pub description: String,
    /// What kind of plugin this is.
    pub kind: PluginKind,
}

/// Classification of plugin capability.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PluginKind {
    MtProvider,
    FileParser,
    QaRule,
}

// ──────────────────────────────────────────────
// MT Provider
// ──────────────────────────────────────────────

/// Input for `mt_translate`.
#[derive(Debug, Serialize, Deserialize)]
pub struct MtRequest {
    pub source_lang: String,
    pub target_lang: String,
    pub segments: Vec<String>,
}

/// Output from `mt_translate`.
#[derive(Debug, Serialize, Deserialize)]
pub struct MtResponse {
    pub translations: Vec<String>,
}

// ──────────────────────────────────────────────
// File Parser
// ──────────────────────────────────────────────

/// Input for `parse_file`.
#[derive(Debug, Serialize, Deserialize)]
pub struct ParseRequest {
    /// Base-64 encoded raw file bytes.
    pub file_bytes_b64: String,
    /// MIME type hint, e.g. `"application/vnd.openxmlformats-officedocument"`.
    pub mime_hint: String,
}

/// A single segment returned by the parser.
#[derive(Debug, Serialize, Deserialize)]
pub struct ParsedSegment {
    pub id: u32,
    pub source: String,
}

/// Output from `parse_file`.
#[derive(Debug, Serialize, Deserialize)]
pub struct ParseResponse {
    pub segments: Vec<ParsedSegment>,
}

// ──────────────────────────────────────────────
// QA Rule
// ──────────────────────────────────────────────

/// A source/target segment pair sent to the QA rule.
#[derive(Debug, Serialize, Deserialize)]
pub struct QaSegment {
    pub id: u32,
    pub source: String,
    pub target: String,
}

/// Input for `qa_check`.
#[derive(Debug, Serialize, Deserialize)]
pub struct QaRequest {
    pub segments: Vec<QaSegment>,
}

/// A single QA issue found by the rule.
#[derive(Debug, Serialize, Deserialize)]
pub struct QaIssue {
    pub segment_id: u32,
    pub severity: QaSeverity,
    pub message: String,
}

/// Severity level of a QA issue.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum QaSeverity {
    Error,
    Warning,
    Info,
}

/// Output from `qa_check`.
#[derive(Debug, Serialize, Deserialize)]
pub struct QaResponse {
    pub issues: Vec<QaIssue>,
}

// ──────────────────────────────────────────────
// Generic error wrapper
// ──────────────────────────────────────────────

/// Envelope that can represent either success or an error string.
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PluginResult<T> {
    Ok(T),
    Err { error: String },
}

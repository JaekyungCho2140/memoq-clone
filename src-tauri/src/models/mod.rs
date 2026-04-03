use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum SegmentStatus {
    #[default]
    Untranslated,
    Draft,
    Translated,
    Confirmed,
}

impl std::str::FromStr for SegmentStatus {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "untranslated" => Ok(Self::Untranslated),
            "draft" => Ok(Self::Draft),
            "translated" => Ok(Self::Translated),
            "confirmed" => Ok(Self::Confirmed),
            other => Err(anyhow::anyhow!("Unknown status: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Segment {
    pub id: String,
    pub source: String,
    pub target: String,
    pub status: SegmentStatus,
    pub order: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectFile {
    pub id: String,
    /// Absolute path to the source file
    pub path: String,
    pub segments: Vec<Segment>,
}

impl ProjectFile {
    /// Returns (total, translated_or_confirmed, confirmed)
    pub fn completion_stats(&self) -> (usize, usize, usize) {
        let total = self.segments.len();
        let translated = self
            .segments
            .iter()
            .filter(|s| {
                matches!(
                    s.status,
                    SegmentStatus::Translated | SegmentStatus::Confirmed
                )
            })
            .count();
        let confirmed = self
            .segments
            .iter()
            .filter(|s| s.status == SegmentStatus::Confirmed)
            .count();
        (total, translated, confirmed)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectStats {
    pub total_segments: usize,
    pub translated: usize,
    pub confirmed: usize,
    pub completion_pct: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub source_lang: String,
    pub target_lang: String,
    pub created_at: DateTime<Utc>,
    /// Multi-file support: each file contains its own segment list
    #[serde(default)]
    pub files: Vec<ProjectFile>,
    // Legacy single-file fields kept for backward compatibility
    #[serde(default)]
    pub source_path: String,
    #[serde(default)]
    pub segments: Vec<Segment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MatchType {
    Exact,
    Fuzzy,
    Context,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TmMatch {
    pub source: String,
    pub target: String,
    pub score: f32,
    pub match_type: MatchType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TmEntry {
    pub id: String,
    pub source: String,
    pub target: String,
    pub source_lang: String,
    pub target_lang: String,
    pub created_at: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TbEntry {
    pub id: String,
    pub source_term: String,
    pub target_term: String,
    pub source_lang: String,
    pub target_lang: String,
    pub notes: String,
    pub forbidden: bool,
}

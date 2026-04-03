use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SegmentStatus {
    Untranslated,
    Draft,
    Translated,
    Confirmed,
}

impl Default for SegmentStatus {
    fn default() -> Self { Self::Untranslated }
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
pub struct Project {
    pub id: String,
    pub name: String,
    pub source_lang: String,
    pub target_lang: String,
    pub created_at: DateTime<Utc>,
    pub segments: Vec<Segment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MatchType { Exact, Fuzzy, Context }

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

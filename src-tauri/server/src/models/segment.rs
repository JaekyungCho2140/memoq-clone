use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SegmentStatus {
    Untranslated,
    Draft,
    Translated,
    Confirmed,
}

impl SegmentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Untranslated => "untranslated",
            Self::Draft => "draft",
            Self::Translated => "translated",
            Self::Confirmed => "confirmed",
        }
    }
}

impl std::str::FromStr for SegmentStatus {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "untranslated" => Ok(Self::Untranslated),
            "draft" => Ok(Self::Draft),
            "translated" => Ok(Self::Translated),
            "confirmed" => Ok(Self::Confirmed),
            other => Err(anyhow::anyhow!("Unknown segment status: {}", other)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    pub id: String,
    pub file_id: String,
    pub seg_order: i64,
    pub source: String,
    pub target: String,
    pub status: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSegmentRequest {
    pub target: Option<String>,
    pub status: Option<String>,
}

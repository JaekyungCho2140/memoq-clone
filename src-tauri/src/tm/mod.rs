mod storage;

use crate::models::{MatchType, TmEntry, TmMatch};
use anyhow::Result;
use chrono::Utc;
use strsim::normalized_levenshtein;
use uuid::Uuid;

pub struct TmSearchParams<'a> {
    pub query: &'a str,
    pub source_lang: &'a str,
    pub target_lang: &'a str,
    pub min_score: f32,
}

pub struct TmEngine {
    db: storage::TmDb,
}

impl TmEngine {
    pub fn create(name: &str, source_lang: &str, target_lang: &str) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        storage::TmDb::init(&id, name, source_lang, target_lang)?;
        Ok(id)
    }

    pub fn open(tm_id: &str) -> Result<Self> {
        Ok(Self {
            db: storage::TmDb::open(tm_id)?,
        })
    }

    pub fn add(
        &self,
        source: &str,
        target: &str,
        source_lang: &str,
        target_lang: &str,
    ) -> Result<TmEntry> {
        let entry = TmEntry {
            id: Uuid::new_v4().to_string(),
            source: source.to_string(),
            target: target.to_string(),
            source_lang: source_lang.to_string(),
            target_lang: target_lang.to_string(),
            created_at: Utc::now(),
            metadata: Default::default(),
        };
        self.db.insert(&entry)?;
        Ok(entry)
    }

    pub fn search(&self, params: TmSearchParams) -> Result<Vec<TmMatch>> {
        let entries = self.db.all(params.source_lang, params.target_lang)?;
        let mut matches: Vec<TmMatch> = entries
            .into_iter()
            .filter_map(|e| {
                let score = normalized_levenshtein(params.query, &e.source) as f32;
                if score < params.min_score {
                    return None;
                }
                let match_type = if score >= 1.0 {
                    MatchType::Exact
                } else {
                    MatchType::Fuzzy
                };
                Some(TmMatch {
                    source: e.source,
                    target: e.target,
                    score,
                    match_type,
                })
            })
            .collect();
        matches.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        Ok(matches)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tm_create_and_search() {
        let id = TmEngine::create("test-tm", "en-US", "ko-KR").unwrap();
        let engine = TmEngine::open(&id).unwrap();
        engine
            .add("Hello, world!", "안녕, 세계!", "en-US", "ko-KR")
            .unwrap();
        let results = engine
            .search(TmSearchParams {
                query: "Hello world",
                source_lang: "en-US",
                target_lang: "ko-KR",
                min_score: 0.5,
            })
            .unwrap();
        assert!(!results.is_empty());
        assert!(results[0].score >= 0.5);
    }
}

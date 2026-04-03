use crate::livedocs::index::load_library;
use crate::livedocs::{LiveDocsError, LiveDocsMatch};

const DEFAULT_MIN_SCORE: f32 = 0.70;

pub fn search(
    query: &str,
    lib_id: &str,
    min_score: Option<f32>,
) -> Result<Vec<LiveDocsMatch>, LiveDocsError> {
    let threshold = min_score.unwrap_or(DEFAULT_MIN_SCORE);
    let lib = load_library(lib_id)?;
    let mut matches = Vec::new();

    for doc in &lib.documents {
        for sentence in &doc.sentences {
            let score = strsim::normalized_levenshtein(query, sentence) as f32;
            if score >= threshold {
                matches.push(LiveDocsMatch {
                    sentence: sentence.clone(),
                    doc_path: doc.path.clone(),
                    score,
                });
            }
        }
    }

    // Sort descending by score
    matches.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(matches)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::livedocs::index::save_library;
    use crate::livedocs::{LiveDocsDocument, LiveDocsLibrary};

    fn make_library_with_sentences(sentences: Vec<&str>) -> LiveDocsLibrary {
        LiveDocsLibrary {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Test Lib".to_string(),
            documents: vec![LiveDocsDocument {
                id: "doc1".to_string(),
                path: "/test/doc.txt".to_string(),
                sentences: sentences.iter().map(|s| s.to_string()).collect(),
            }],
        }
    }

    #[test]
    fn test_search_exact_match() {
        let lib = make_library_with_sentences(vec![
            "The quick brown fox jumps over the lazy dog.",
            "Hello world this is a test sentence.",
        ]);
        save_library(&lib).unwrap();

        let results = search("Hello world this is a test sentence.", &lib.id, None).unwrap();
        assert!(!results.is_empty());
        assert!(results[0].score >= 0.99);
    }

    #[test]
    fn test_search_fuzzy_match() {
        let lib = make_library_with_sentences(vec!["The quick brown fox jumps over the lazy dog."]);
        save_library(&lib).unwrap();

        // Slightly different query should still match
        let results = search(
            "The quick brown fox jumps over the lazy dogs.",
            &lib.id,
            Some(0.8),
        )
        .unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_search_below_threshold() {
        let lib = make_library_with_sentences(vec!["Completely unrelated content here."]);
        save_library(&lib).unwrap();

        let results = search("Hello world this is a test sentence.", &lib.id, Some(0.9)).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_sorted_by_score() {
        let lib = make_library_with_sentences(vec![
            "Hello world test sentence for matching.",
            "Hello world this is a test sentence.",
        ]);
        save_library(&lib).unwrap();

        let results = search("Hello world this is a test sentence.", &lib.id, Some(0.5)).unwrap();
        if results.len() >= 2 {
            assert!(results[0].score >= results[1].score);
        }
    }
}

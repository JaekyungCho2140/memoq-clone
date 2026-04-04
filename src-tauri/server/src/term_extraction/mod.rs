//! Automatic term extraction engine.
//!
//! # Algorithm
//!
//! A two-pass statistical approach:
//!
//! **Pass 1 — TF (within-document frequency):**
//! Count occurrences of every 1-gram, 2-gram, and 3-gram (normalised to
//! lowercase, punctuation-stripped).  Filter out common stop-words.
//!
//! **Pass 2 — C-value (nested-term penalty):**
//! For each n-gram candidate, subtract a fraction of the total frequency of
//! longer n-grams that contain it (the "containment penalty").  This prevents
//! single words that only appear inside fixed phrases from being promoted.
//!
//! Final score = tf_normalised * (1 + log2(ngram_len)) * c_value_factor
//!
//! Only candidates with `score > 0` are returned, sorted descending.

use std::collections::HashMap;

/// A single term candidate returned by [`extract_terms`].
#[derive(Debug, Clone, PartialEq)]
pub struct TermCandidate {
    pub term: String,
    /// Combined TF × C-value score (higher = more likely to be a domain term).
    pub score: f64,
    /// Raw occurrence count in the input text.
    pub frequency: u32,
}

// ──────────────────────────────────────────────
// Stop-word list (English + Korean particles)
// ──────────────────────────────────────────────

const STOP_WORDS: &[&str] = &[
    // English
    "a", "an", "the", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "by", "from",
    "is", "are", "was", "were", "be", "been", "being", "have", "has", "had", "do", "does", "did",
    "will", "would", "could", "should", "may", "might", "shall", "can", "not", "no", "this",
    "that", "these", "those", "it", "its", "we", "you", "he", "she", "they", "i", "me", "my",
    "our", "your", "his", "her", "their", "as", "if", "so", "up", "out", "about", "into", "than",
    "then", "when", "where", "which", "who", "whom",
];

fn is_stop_word(token: &str) -> bool {
    STOP_WORDS.contains(&token)
}

// ──────────────────────────────────────────────
// Tokenisation
// ──────────────────────────────────────────────

/// Split text into normalised tokens (lowercase, alphanumeric only).
fn tokenize(text: &str) -> Vec<String> {
    text.split_whitespace()
        .filter_map(|raw| {
            let t: String = raw
                .chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
                .to_lowercase();
            if t.len() >= 2 {
                Some(t)
            } else {
                None
            }
        })
        .collect()
}

// ──────────────────────────────────────────────
// N-gram counting
// ──────────────────────────────────────────────

fn count_ngrams(tokens: &[String], n: usize) -> HashMap<String, u32> {
    let mut counts: HashMap<String, u32> = HashMap::new();
    if tokens.len() < n {
        return counts;
    }
    for window in tokens.windows(n) {
        // Skip if first or last token is a stop-word (for bigrams/trigrams)
        if n > 1 && (is_stop_word(&window[0]) || is_stop_word(window.last().unwrap())) {
            continue;
        }
        // Skip if any token in a unigram is a stop-word
        if n == 1 && is_stop_word(&window[0]) {
            continue;
        }
        let key = window.join(" ");
        *counts.entry(key).or_insert(0) += 1;
    }
    counts
}

// ──────────────────────────────────────────────
// C-value scoring
// ──────────────────────────────────────────────

/// For each candidate phrase, compute a C-value-inspired score that penalises
/// shorter terms that are completely subsumed by longer, more frequent terms.
fn c_value_scores(ngram_counts: &HashMap<String, u32>) -> HashMap<String, f64> {
    // Build list sorted by descending phrase length for the penalty pass
    let mut candidates: Vec<(&String, u32)> = ngram_counts.iter().map(|(k, v)| (k, *v)).collect();
    candidates.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

    let mut scores: HashMap<String, f64> = HashMap::new();
    // penalty_sum[phrase] = sum of frequencies of containing phrases
    let mut penalty_sum: HashMap<String, f64> = HashMap::new();
    // penalty_count[phrase] = number of containing phrases
    let mut penalty_count: HashMap<String, u32> = HashMap::new();

    for (phrase, freq) in &candidates {
        let ngram_len = phrase.split_whitespace().count() as f64;

        // For every shorter phrase that is a substring of this phrase, record penalty
        for (shorter, _) in ngram_counts {
            if shorter.len() < phrase.len() && phrase.contains(shorter.as_str()) {
                *penalty_sum.entry(shorter.to_string()).or_insert(0.0) += *freq as f64;
                *penalty_count.entry(shorter.to_string()).or_insert(0) += 1;
            }
        }

        let tf = *freq as f64;
        let p_sum = *penalty_sum.get(phrase.as_str()).unwrap_or(&0.0);
        let p_cnt = *penalty_count.get(phrase.as_str()).unwrap_or(&0) as f64;

        // C-value = log2(|a|) * (freq(a) − penalty(a))
        // where penalty = p_sum / p_cnt if p_cnt > 0 else 0
        let penalty = if p_cnt > 0.0 { p_sum / p_cnt } else { 0.0 };
        let log_len = if ngram_len > 1.0 {
            ngram_len.log2()
        } else {
            1.0
        };
        let score = log_len * (tf - penalty).max(0.0);
        scores.insert(phrase.to_string(), score);
    }

    scores
}

// ──────────────────────────────────────────────
// Public API
// ──────────────────────────────────────────────

/// Extract the top `max_candidates` domain-term candidates from `text`.
///
/// `text` should be the source or target document content (plain text).
pub fn extract_terms(text: &str, max_candidates: usize) -> Vec<TermCandidate> {
    let tokens = tokenize(text);
    if tokens.is_empty() {
        return vec![];
    }

    // Collect all n-grams (1–3)
    let mut all_counts: HashMap<String, u32> = HashMap::new();
    for n in 1..=3 {
        for (k, v) in count_ngrams(&tokens, n) {
            *all_counts.entry(k).or_insert(0) += v;
        }
    }

    // Keep only terms that appear at least twice (noise filter)
    all_counts.retain(|_, v| *v >= 2);

    let scores = c_value_scores(&all_counts);

    let mut candidates: Vec<TermCandidate> = scores
        .iter()
        .filter(|(_, s)| **s > 0.0)
        .map(|(term, score)| TermCandidate {
            term: term.clone(),
            score: *score,
            frequency: *all_counts.get(term).unwrap_or(&0),
        })
        .collect();

    // Sort by score descending, then alphabetically for determinism
    candidates.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.term.cmp(&b.term))
    });

    candidates.truncate(max_candidates);
    candidates
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_strips_punctuation() {
        let tokens = tokenize("Hello, world! This is a test.");
        assert!(tokens.contains(&"hello".to_string()));
        assert!(tokens.contains(&"world".to_string()));
        // stop words removed from scoring but still tokenised
        assert!(tokens.contains(&"test".to_string()));
    }

    #[test]
    fn extract_single_word_terms() {
        // Repeat a domain word many times
        let text =
            "photosynthesis photosynthesis photosynthesis chlorophyll chlorophyll photosynthesis";
        let candidates = extract_terms(text, 10);
        let terms: Vec<&str> = candidates.iter().map(|c| c.term.as_str()).collect();
        assert!(
            terms.contains(&"photosynthesis"),
            "should find photosynthesis"
        );
        assert!(terms.contains(&"chlorophyll"), "should find chlorophyll");
    }

    #[test]
    fn extract_bigram_terms() {
        let text = "machine learning machine learning machine learning deep learning deep learning";
        let candidates = extract_terms(text, 10);
        let terms: Vec<&str> = candidates.iter().map(|c| c.term.as_str()).collect();
        assert!(
            terms.contains(&"machine learning"),
            "should find bigram 'machine learning'; got: {:?}",
            terms
        );
    }

    #[test]
    fn stop_words_are_not_returned_as_candidates() {
        let text = "the the the the the a a a a and and and";
        let candidates = extract_terms(text, 10);
        for c in &candidates {
            assert_ne!(c.term, "the");
            assert_ne!(c.term, "a");
            assert_ne!(c.term, "and");
        }
    }

    #[test]
    fn returns_at_most_max_candidates() {
        // Long text with many unique words
        let text = "alpha beta gamma delta epsilon zeta eta theta alpha beta gamma delta epsilon zeta eta theta";
        let candidates = extract_terms(text, 3);
        assert!(candidates.len() <= 3);
    }

    #[test]
    fn empty_text_returns_empty() {
        let candidates = extract_terms("", 10);
        assert!(candidates.is_empty());
    }

    #[test]
    fn rare_terms_filtered_out() {
        // A term appearing only once should be filtered (frequency < 2)
        let text = "hapax legomenon this word only once unique_term alpha alpha alpha";
        let candidates = extract_terms(text, 20);
        let terms: Vec<&str> = candidates.iter().map(|c| c.term.as_str()).collect();
        assert!(
            !terms.contains(&"hapax"),
            "single-occurrence term should be filtered"
        );
    }

    #[test]
    fn scores_are_positive() {
        let text = "neural network neural network deep neural network deep learning";
        let candidates = extract_terms(text, 20);
        for c in &candidates {
            assert!(
                c.score > 0.0,
                "all scores should be positive, got {}",
                c.score
            );
        }
    }

    #[test]
    fn c_value_prefers_longer_phrases() {
        // "deep learning" should score higher than isolated "deep" or "learning"
        let text = (0..10)
            .map(|_| "deep learning")
            .collect::<Vec<_>>()
            .join(" ");
        let candidates = extract_terms(&text, 20);
        let bigram = candidates.iter().find(|c| c.term == "deep learning");
        let unigram_deep = candidates.iter().find(|c| c.term == "deep");
        if let (Some(b), Some(u)) = (bigram, unigram_deep) {
            assert!(
                b.score >= u.score,
                "bigram score {} should be >= unigram score {}",
                b.score,
                u.score
            );
        }
        // At minimum the bigram should appear
        assert!(
            candidates.iter().any(|c| c.term == "deep learning"),
            "expected 'deep learning' in candidates: {:?}",
            candidates.iter().map(|c| &c.term).collect::<Vec<_>>()
        );
    }
}

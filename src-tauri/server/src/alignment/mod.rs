//! TM Alignment engine — aligns source and target sentences from parallel documents.
//!
//! # Algorithm
//!
//! Uses a two-pass strategy:
//!
//! 1. **Length-ratio scoring**: Compute the character-length ratio between every
//!    (source_i, target_j) pair.  Ratios close to 1.0 score high.
//!
//! 2. **Cosine token similarity**: Tokenise each sentence into lowercase word
//!    n-grams and compute cosine similarity over shared token counts.  Even
//!    across different languages, cognates, numbers, and shared abbreviations
//!    lift the score.
//!
//! Final score = 0.4 * length_score + 0.6 * cosine_score.
//!
//! Dynamic-programming sentence-pair alignment (Gale & Church style) then
//! finds the globally best 1:1 pairing using these scores.

pub mod parser;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ──────────────────────────────────────────────
// Public types
// ──────────────────────────────────────────────

/// A proposed source–target sentence pair with a confidence score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignedPair {
    pub source_idx: usize,
    pub target_idx: usize,
    pub source: String,
    pub target: String,
    /// Combined confidence score 0.0–1.0.
    pub score: f64,
}

/// Full alignment result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentResult {
    pub pairs: Vec<AlignedPair>,
    /// Number of source sentences that had no confident match.
    pub unmatched_source: usize,
    /// Number of target sentences that had no confident match.
    pub unmatched_target: usize,
}

// ──────────────────────────────────────────────
// Core scoring helpers
// ──────────────────────────────────────────────

/// Score the length ratio between two strings.
/// Returns a value in [0, 1]: 1.0 when lengths are equal.
fn length_score(a: &str, b: &str) -> f64 {
    let la = a.chars().count() as f64;
    let lb = b.chars().count() as f64;
    if la == 0.0 && lb == 0.0 {
        return 1.0;
    }
    if la == 0.0 || lb == 0.0 {
        return 0.0;
    }
    let ratio = la / lb;
    // Map ratio to [0,1]: 1.0 at ratio=1.0, decaying symmetrically
    1.0 - (1.0 - ratio.min(1.0 / ratio)).min(1.0)
}

/// Tokenise a string into lowercase whitespace-split tokens.
fn tokenize(s: &str) -> HashMap<String, u32> {
    let mut counts = HashMap::new();
    for tok in s.split_whitespace() {
        let t = tok.to_lowercase();
        let t = t.trim_matches(|c: char| !c.is_alphanumeric());
        if !t.is_empty() {
            *counts.entry(t.to_string()).or_insert(0) += 1;
        }
    }
    counts
}

/// Cosine similarity between token-count vectors.
fn cosine_similarity(a: &HashMap<String, u32>, b: &HashMap<String, u32>) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let dot: u64 = a
        .iter()
        .filter_map(|(k, va)| b.get(k).map(|vb| (*va as u64) * (*vb as u64)))
        .sum();
    if dot == 0 {
        return 0.0;
    }
    let mag_a: f64 = a.values().map(|v| (*v as f64).powi(2)).sum::<f64>().sqrt();
    let mag_b: f64 = b.values().map(|v| (*v as f64).powi(2)).sum::<f64>().sqrt();
    (dot as f64) / (mag_a * mag_b)
}

/// Combined pair score (0.0–1.0).
pub fn pair_score(src: &str, tgt: &str) -> f64 {
    let ls = length_score(src, tgt);
    let ts_a = tokenize(src);
    let ts_b = tokenize(tgt);
    let cs = cosine_similarity(&ts_a, &ts_b);
    0.4 * ls + 0.6 * cs
}

// ──────────────────────────────────────────────
// DP alignment
// ──────────────────────────────────────────────

/// Align `sources` against `targets` using DP to find the highest-scoring
/// monotone 1:1 matching.  Returns matched pairs; unmatched counts are
/// computed from the remainder.
pub fn align(sources: &[String], targets: &[String]) -> AlignmentResult {
    let n = sources.len();
    let m = targets.len();

    if n == 0 || m == 0 {
        return AlignmentResult {
            pairs: vec![],
            unmatched_source: n,
            unmatched_target: m,
        };
    }

    // Score matrix
    let scores: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            (0..m)
                .map(|j| pair_score(&sources[i], &targets[j]))
                .collect()
        })
        .collect();

    // DP table: dp[i][j] = best total score aligning sources[0..i] with targets[0..j]
    // Transitions:
    //   match (i-1, j-1) → dp[i-1][j-1] + score[i-1][j-1]
    //   skip source i-1  → dp[i-1][j]
    //   skip target j-1  → dp[i][j-1]
    let mut dp = vec![vec![0.0f64; m + 1]; n + 1];
    let mut back = vec![vec![(0i32, 0i32); m + 1]; n + 1];

    for i in 1..=n {
        for j in 1..=m {
            let match_score = dp[i - 1][j - 1] + scores[i - 1][j - 1];
            let skip_src = dp[i - 1][j];
            let skip_tgt = dp[i][j - 1];

            if match_score >= skip_src && match_score >= skip_tgt {
                dp[i][j] = match_score;
                back[i][j] = (i as i32 - 1, j as i32 - 1);
            } else if skip_src >= skip_tgt {
                dp[i][j] = skip_src;
                back[i][j] = (i as i32 - 1, j as i32);
            } else {
                dp[i][j] = skip_tgt;
                back[i][j] = (i as i32, j as i32 - 1);
            }
        }
    }

    // Trace back
    let mut pairs = Vec::new();
    let mut i = n as i32;
    let mut j = m as i32;
    while i > 0 || j > 0 {
        if i <= 0 {
            j -= 1;
            continue;
        }
        if j <= 0 {
            i -= 1;
            continue;
        }
        let (pi, pj) = back[i as usize][j as usize];
        if pi == i - 1 && pj == j - 1 {
            // matched pair
            let si = (i - 1) as usize;
            let ti = (j - 1) as usize;
            let sc = scores[si][ti];
            pairs.push(AlignedPair {
                source_idx: si,
                target_idx: ti,
                source: sources[si].clone(),
                target: targets[ti].clone(),
                score: sc,
            });
        }
        i = pi;
        j = pj;
    }
    pairs.reverse();

    let matched_src: std::collections::HashSet<usize> =
        pairs.iter().map(|p| p.source_idx).collect();
    let matched_tgt: std::collections::HashSet<usize> =
        pairs.iter().map(|p| p.target_idx).collect();

    AlignmentResult {
        unmatched_source: n - matched_src.len(),
        unmatched_target: m - matched_tgt.len(),
        pairs,
    }
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_score_equal_strings() {
        assert!((length_score("hello", "world") - 1.0).abs() < 1e-9);
    }

    #[test]
    fn length_score_empty() {
        assert_eq!(length_score("", ""), 1.0);
        assert_eq!(length_score("abc", ""), 0.0);
    }

    #[test]
    fn length_score_asymmetric() {
        let s = length_score("hello", "hello world");
        assert!(s > 0.0 && s < 1.0);
    }

    #[test]
    fn cosine_identical_tokens() {
        let t = tokenize("the quick brown fox");
        assert!((cosine_similarity(&t, &t) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn cosine_disjoint_tokens() {
        let a = tokenize("hello world");
        let b = tokenize("foo bar");
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn pair_score_same_numbers() {
        // Numbers are language-independent; same number should lift score
        let s = pair_score("Report 2024: 42 items", "보고서 2024: 42 항목");
        assert!(s > 0.0, "score={}", s);
    }

    #[test]
    fn align_perfect_1to1() {
        let src = vec!["Hello.".to_string(), "Goodbye.".to_string()];
        let tgt = vec!["안녕하세요.".to_string(), "잘 가세요.".to_string()];
        let result = align(&src, &tgt);
        // Should produce 2 pairs (length ratio drives alignment)
        assert_eq!(result.pairs.len(), 2);
    }

    #[test]
    fn align_empty_inputs() {
        let result = align(&[], &["x".to_string()]);
        assert!(result.pairs.is_empty());
        assert_eq!(result.unmatched_target, 1);
    }

    #[test]
    fn align_identical_sentences() {
        let sents: Vec<String> = vec!["Same text.".to_string(), "Another one.".to_string()];
        let result = align(&sents, &sents);
        // Perfect match expected
        assert_eq!(result.pairs.len(), 2);
        for p in &result.pairs {
            assert!((p.score - 1.0).abs() < 1e-9, "score={}", p.score);
        }
    }

    #[test]
    fn align_shared_numbers_improves_score() {
        // Numbers shared between src/tgt should produce better alignment
        let src = vec!["Revenue: 1000 USD".to_string()];
        let tgt = vec!["매출: 1000 USD".to_string()];
        let result = align(&src, &tgt);
        assert_eq!(result.pairs.len(), 1);
        assert!(
            result.pairs[0].score > 0.3,
            "score={}",
            result.pairs[0].score
        );
    }
}

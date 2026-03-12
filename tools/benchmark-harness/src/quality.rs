//! Quality scoring module for benchmark results
//!
//! Computes F1-based quality metrics by comparing extracted text against ground truth.
//! Uses token-level (bag of words) precision and recall.

use crate::types::QualityMetrics;
use std::collections::HashMap;

/// Compute quality metrics comparing extracted text against ground truth
///
/// Algorithm:
/// 1. Tokenize both texts: lowercase, split on whitespace, strip non-alphanumeric chars except periods
///    - "3.14" is preserved as a single token
///    - "3,14" becomes "314" (comma is stripped, not a delimiter)
/// 2. Build token multisets (bag of words with counts)
/// 3. Compute precision = |intersection| / |extracted tokens|
/// 4. Compute recall = |intersection| / |ground truth tokens|
/// 5. F1 = 2 * precision * recall / (precision + recall)
///    - If both token sets are empty, F1 = 1.0 (vacuously perfect match)
/// 6. Separate F1 for all tokens vs numeric-only tokens
/// 7. quality_score = 0.7 * f1_text + 0.3 * f1_numeric
pub fn compute_quality(extracted: &str, ground_truth: &str) -> QualityMetrics {
    let extracted_tokens = tokenize(extracted);
    let truth_tokens = tokenize(ground_truth);

    let f1_score_text = compute_f1(&extracted_tokens, &truth_tokens);

    let extracted_numeric = filter_numeric(&extracted_tokens);
    let truth_numeric = filter_numeric(&truth_tokens);
    let f1_score_numeric = compute_f1(&extracted_numeric, &truth_numeric);

    // f1_score_layout is not implemented (skip per plan)
    let f1_score_layout = 0.0;

    let quality_score = 0.7 * f1_score_text + 0.3 * f1_score_numeric;

    let (missing_tokens, extra_tokens) = compute_token_diff(&extracted_tokens, &truth_tokens);

    QualityMetrics {
        f1_score_text,
        f1_score_numeric,
        f1_score_layout,
        quality_score,
        missing_tokens,
        extra_tokens,
    }
}

/// Tokenize text: lowercase, split on whitespace, strip non-alphanumeric characters
fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split_whitespace()
        .map(|w| {
            w.chars()
                .filter(|c| c.is_alphanumeric() || *c == '.')
                .collect::<String>()
        })
        .filter(|w| !w.is_empty())
        .collect()
}

/// Filter tokens to only those matching numeric patterns (digits with optional decimal)
fn filter_numeric(tokens: &[String]) -> Vec<String> {
    tokens
        .iter()
        .filter(|t| t.chars().any(|c| c.is_ascii_digit()))
        .cloned()
        .collect()
}

/// Compute F1 score between two token bags using multiset intersection
fn compute_f1(extracted: &[String], truth: &[String]) -> f64 {
    if extracted.is_empty() && truth.is_empty() {
        return 1.0; // Both empty = perfect match
    }
    if extracted.is_empty() || truth.is_empty() {
        return 0.0;
    }

    let extracted_counts = build_counts(extracted);
    let truth_counts = build_counts(truth);

    // Multiset intersection: for each ground truth token, count min(truth_count, extracted_count).
    // Tokens only in extracted text contribute 0 to intersection (penalized via precision denominator).
    let intersection: usize = truth_counts
        .iter()
        .map(|(token, &count)| {
            let ext_count = extracted_counts.get(token).copied().unwrap_or(0);
            ext_count.min(count)
        })
        .sum();

    let precision = intersection as f64 / extracted.len() as f64;
    let recall = intersection as f64 / truth.len() as f64;

    if precision + recall == 0.0 {
        return 0.0;
    }

    2.0 * precision * recall / (precision + recall)
}

/// Build a token frequency map
fn build_counts(tokens: &[String]) -> HashMap<&str, usize> {
    let mut counts = HashMap::new();
    for token in tokens {
        *counts.entry(token.as_str()).or_insert(0) += 1;
    }
    counts
}

/// Compute token-level diff between extracted and ground truth token bags.
///
/// Returns (missing_tokens, extra_tokens) where:
/// - missing_tokens: tokens in GT with higher count than in extraction (recall misses)
/// - extra_tokens: tokens in extraction with higher count than in GT (precision misses)
///
/// Both are sorted by deficit/surplus count descending.
type TokenDiffs = (Vec<(String, usize)>, Vec<(String, usize)>);

fn compute_token_diff(extracted: &[String], truth: &[String]) -> TokenDiffs {
    let extracted_counts = build_counts(extracted);
    let truth_counts = build_counts(truth);

    // Tokens in GT but missing/under-represented in extraction
    let mut missing: Vec<(String, usize)> = truth_counts
        .iter()
        .filter_map(|(&token, &gt_count)| {
            let ext_count = extracted_counts.get(token).copied().unwrap_or(0);
            if gt_count > ext_count {
                Some((token.to_string(), gt_count - ext_count))
            } else {
                None
            }
        })
        .collect();
    missing.sort_by(|a, b| b.1.cmp(&a.1));

    // Tokens in extraction but not in GT or over-represented
    let mut extra: Vec<(String, usize)> = extracted_counts
        .iter()
        .filter_map(|(&token, &ext_count)| {
            let gt_count = truth_counts.get(token).copied().unwrap_or(0);
            if ext_count > gt_count {
                Some((token.to_string(), ext_count - gt_count))
            } else {
                None
            }
        })
        .collect();
    extra.sort_by(|a, b| b.1.cmp(&a.1));

    (missing, extra)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_text() {
        let text = "Hello world this is a test";
        let result = compute_quality(text, text);
        assert!((result.f1_score_text - 1.0).abs() < 0.001);
        assert!((result.quality_score - 1.0).abs() < 0.01); // 0.7*1.0 + 0.3*1.0 (no numerics = both empty = perfect)
    }

    #[test]
    fn test_completely_different() {
        let result = compute_quality("alpha beta gamma", "one two three");
        assert_eq!(result.f1_score_text, 0.0);
    }

    #[test]
    fn test_partial_overlap() {
        let result = compute_quality("hello world foo", "hello world bar");
        // Extracted: {hello, world, foo}, Truth: {hello, world, bar}
        // Intersection: {hello, world} = 2
        // Precision: 2/3, Recall: 2/3, F1: 2/3
        assert!((result.f1_score_text - 2.0 / 3.0).abs() < 0.001);
    }

    #[test]
    fn test_numeric_scoring() {
        let result = compute_quality("page 42 section 7", "page 42 section 7");
        assert!((result.f1_score_numeric - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_empty_inputs() {
        let result = compute_quality("", "");
        assert!((result.f1_score_text - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_empty_extracted() {
        let result = compute_quality("", "some ground truth");
        assert_eq!(result.f1_score_text, 0.0);
    }

    #[test]
    fn test_punctuation_stripped() {
        let result = compute_quality("hello, world!", "hello world");
        assert!((result.f1_score_text - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_case_insensitive() {
        let result = compute_quality("Hello World", "hello world");
        assert!((result.f1_score_text - 1.0).abs() < 0.001);
    }
}

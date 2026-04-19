//! Quality scoring module for benchmark results.
//!
//! Computes F1-based quality metrics by comparing extracted text against ground truth.
//! Uses token-level (bag-of-words) precision and recall.
//!
//! # Scoring weights
//!
//! Text-only scoring uses a **0.6 / 0.4 text / numeric split**:
//!
//! ```text
//! quality_score = 0.6 * f1_text + 0.4 * f1_numeric
//! ```
//!
//! Numeric tokens receive disproportionate weight (40% despite typically being
//! a small fraction of the token count) because financial documents, scientific
//! papers, and tabular data depend heavily on number accuracy. A single wrong
//! digit can invalidate an entire table row or equation.
//!
//! When markdown ground truth is available, **combined scoring** kicks in:
//!
//! ```text
//! quality_score = 0.5 * f1_text + 0.2 * f1_numeric + 0.3 * f1_layout
//! ```
//!
//! The layout component (`f1_layout`) comes from [`crate::markdown_quality`]
//! and captures structural fidelity (headings, tables, code blocks, etc.).
//!
//! # Tokenization
//!
//! Tokenization is intentionally simple: lowercase, split on whitespace,
//! strip non-alphanumeric characters except periods and commas embedded between
//! alphanumeric characters (preserving decimal numbers like "3.14" and European
//! format "3,14"). This preserves punctuation that is semantically meaningful
//! while ignoring decorative punctuation.

use crate::types::QualityMetrics;
use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

/// Regex to strip markdown image syntax `![alt](url)` → `alt`
static MD_IMAGE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"!\[([^\]]*)\]\([^)]*\)").expect("invalid regex"));

/// Regex to strip markdown link syntax `[text](url)` → `text`
static MD_LINK_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[([^\]]*)\]\([^)]*\)").expect("invalid regex"));

/// Strip markdown link and image syntax so URL components don't become tokens.
/// `![alt](url)` → `alt`, `[text](url)` → `text`.
fn strip_markdown_links(text: &str) -> String {
    let text = MD_IMAGE_RE.replace_all(text, "$1");
    MD_LINK_RE.replace_all(&text, "$1").into_owned()
}

/// Compute quality metrics comparing extracted text against ground truth,
/// optionally including structural quality scoring when markdown GT is available.
///
/// When `ground_truth_markdown` is `Some`, computes structural F1 from markdown
/// block comparison and adjusts the quality_score formula to include it:
///   quality_score = 0.5 * f1_text + 0.2 * f1_numeric + 0.3 * f1_layout
///
/// When `ground_truth_markdown` is `None`, falls back to text-only scoring:
///   quality_score = 0.6 * f1_text + 0.4 * f1_numeric
pub fn compute_quality_with_structure(
    extracted: &str,
    ground_truth: &str,
    ground_truth_markdown: Option<&str>,
) -> QualityMetrics {
    let mut metrics = compute_quality(extracted, ground_truth);

    if let Some(md_gt) = ground_truth_markdown {
        let structural = crate::markdown_quality::score_structural_quality(extracted, md_gt);
        metrics.f1_score_layout = structural.structural_f1;
        // Adjust quality_score to include structural component.
        // When neither side has numeric tokens, drop the numeric weight and redistribute.
        metrics.quality_score = if has_any_numeric_tokens(extracted, ground_truth) {
            0.5 * metrics.f1_score_text + 0.2 * metrics.f1_score_numeric + 0.3 * metrics.f1_score_layout
        } else {
            // No numeric tokens: use 0.625 text + 0.375 layout (same 5:3 ratio, no numeric)
            0.625 * metrics.f1_score_text + 0.375 * metrics.f1_score_layout
        };
    }

    metrics.correct = metrics.quality_score >= 0.95;
    metrics
}

/// Compute quality metrics comparing extracted text against ground truth
///
/// Algorithm:
/// 1. Tokenize both texts: lowercase, split on whitespace, strip non-alphanumeric chars except periods and commas
///    - "3.14" is preserved as a single token
///    - "3,14" is preserved as a single token (European decimal format)
/// 2. Build token multisets (bag of words with counts)
/// 3. Compute precision = |intersection| / |extracted tokens|
/// 4. Compute recall = |intersection| / |ground truth tokens|
/// 5. F1 = 2 * precision * recall / (precision + recall)
///    - If both token sets are empty, F1 = 1.0 (vacuously perfect match)
/// 6. Separate F1 for all tokens vs numeric-only tokens
/// 7. quality_score = 0.6 * f1_text + 0.4 * f1_numeric
pub fn compute_quality(extracted: &str, ground_truth: &str) -> QualityMetrics {
    let extracted_tokens = tokenize(extracted);
    let truth_tokens = tokenize(ground_truth);

    let f1_score_text = compute_f1(&extracted_tokens, &truth_tokens);

    let extracted_numeric = filter_numeric(&extracted_tokens);
    let truth_numeric = filter_numeric(&truth_tokens);
    let f1_score_numeric = compute_f1(&extracted_numeric, &truth_numeric);

    // f1_score_layout is not implemented (skip per plan)
    let f1_score_layout = 0.0;

    // When neither side has numeric tokens, both-empty compute_f1 returns 1.0
    // which would give a free 0.4 boost. Use text-only scoring in that case.
    let quality_score = if extracted_numeric.is_empty() && truth_numeric.is_empty() {
        f1_score_text
    } else {
        0.6 * f1_score_text + 0.4 * f1_score_numeric
    };

    let (missing_tokens, extra_tokens) = compute_token_diff(&extracted_tokens, &truth_tokens);

    let correct = quality_score >= 0.95;

    QualityMetrics {
        f1_score_text,
        f1_score_numeric,
        f1_score_layout,
        quality_score,
        missing_tokens,
        extra_tokens,
        correct,
    }
}

/// Tokenize text: lowercase, split on whitespace, strip non-alphanumeric characters
/// (preserving `.` and `,` only when embedded between alphanumeric chars, e.g. "3.14", "3,14")
pub fn tokenize(text: &str) -> Vec<String> {
    let text = strip_markdown_links(text);
    text.to_lowercase()
        .split_whitespace()
        .map(|w| {
            // First pass: keep alphanumeric, periods, and commas
            let kept: String = w
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '.' || *c == ',')
                .collect();
            // Second pass: strip leading/trailing periods and commas
            kept.trim_matches(|c: char| c == '.' || c == ',').to_string()
        })
        .filter(|w| !w.is_empty())
        .map(|token| {
            // Normalize numeric tokens: "15.0" -> "15", "100.00" -> "100"
            // Only apply f64 normalization for numbers with 15 or fewer digits
            // to avoid precision loss (f64 has ~15.9 significant digits).
            let digit_count = token.chars().filter(|c| c.is_ascii_digit()).count();
            if digit_count <= 15 {
                if let Ok(num) = token.parse::<f64>() {
                    let normalized = format!("{num}");
                    if normalized != token { normalized } else { token }
                } else {
                    token
                }
            } else {
                token
            }
        })
        .collect()
}

/// Check whether either text has any numeric tokens (used to decide scoring formula).
fn has_any_numeric_tokens(text_a: &str, text_b: &str) -> bool {
    let a_tokens = tokenize(text_a);
    let b_tokens = tokenize(text_b);
    !filter_numeric(&a_tokens).is_empty() || !filter_numeric(&b_tokens).is_empty()
}

/// Filter tokens to only those containing numeric characters (Unicode-aware)
fn filter_numeric(tokens: &[String]) -> Vec<String> {
    tokens
        .iter()
        .filter(|t| t.chars().any(|c| c.is_numeric()))
        .cloned()
        .collect()
}

/// Compute F1 score between two token bags using multiset intersection
pub fn compute_f1(extracted: &[String], truth: &[String]) -> f64 {
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
pub type TokenDiff = (Vec<(String, usize)>, Vec<(String, usize)>);

pub fn compute_token_diff(extracted: &[String], truth: &[String]) -> TokenDiff {
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
    missing.sort_by_key(|b| std::cmp::Reverse(b.1));

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
    extra.sort_by_key(|b| std::cmp::Reverse(b.1));

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
        assert!((result.quality_score - 1.0).abs() < 0.01); // text-only scoring (no numerics on either side)
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

    #[test]
    fn test_tokenize_number_normalization() {
        // "15.0" and "15" should produce the same token
        let tokens_a = tokenize("15.0");
        let tokens_b = tokenize("15");
        assert_eq!(tokens_a, tokens_b, "15.0 and 15 should normalize to the same token");
        assert_eq!(tokens_a, vec!["15"]);

        // "100.00" should normalize to "100"
        assert_eq!(tokenize("100.00"), vec!["100"]);
    }

    #[test]
    fn test_compute_f1_number_equivalence() {
        let extracted = tokenize("price 15.0 dollars");
        let truth = tokenize("price 15 dollars");
        let f1 = compute_f1(&extracted, &truth);
        assert!(
            (f1 - 1.0).abs() < 0.001,
            "F1 should be 1.0 for semantically equivalent numeric tokens, got {f1}"
        );
    }

    #[test]
    fn test_tokenize_preserves_decimals() {
        // Non-trailing-zero decimals must be preserved
        assert_eq!(tokenize("3.14"), vec!["3.14"]);
        assert_eq!(tokenize("0.5"), vec!["0.5"]);
        assert_eq!(tokenize("12.345"), vec!["12.345"]);
    }

    #[test]
    fn test_no_numbers_no_boost() {
        // Two texts with no numeric tokens should score based on text_f1 only,
        // not get a free 0.4 boost from both-empty numeric F1.
        let result = compute_quality("hello world foo", "hello world bar");
        // text F1: intersection {hello, world} = 2, precision=2/3, recall=2/3, F1=2/3
        let expected_text_f1 = 2.0 / 3.0;
        assert!(
            (result.f1_score_text - expected_text_f1).abs() < 0.001,
            "text F1 should be 2/3, got {}",
            result.f1_score_text
        );
        // quality_score should equal text_f1 (no numeric component)
        assert!(
            (result.quality_score - expected_text_f1).abs() < 0.001,
            "quality_score should equal text F1 ({expected_text_f1}) when no numbers, got {}",
            result.quality_score
        );
    }

    #[test]
    fn test_url_stripped_from_tokens() {
        // Markdown links should not produce URL component tokens
        let tokens = tokenize("[link text](https://example.com)");
        assert_eq!(tokens, vec!["link", "text"]);

        // Markdown images should not produce URL component tokens
        let tokens = tokenize("![alt text](https://example.com/image.png)");
        assert_eq!(tokens, vec!["alt", "text"]);

        // Mixed content
        let tokens = tokenize("See [docs](https://example.com/docs) for details");
        assert_eq!(tokens, vec!["see", "docs", "for", "details"]);
    }

    #[test]
    fn test_large_number_preserved() {
        // 17-digit number should not be mangled by f64 precision loss
        let tokens = tokenize("10000000000000001");
        assert_eq!(
            tokens,
            vec!["10000000000000001"],
            "17-digit number should be preserved as-is, not rounded by f64"
        );

        // 15-digit number (including the trailing zero) should still be normalized
        let tokens = tokenize("12345678901234.0");
        assert_eq!(
            tokens,
            vec!["12345678901234"],
            "15-digit number with trailing .0 should still normalize"
        );
    }
}

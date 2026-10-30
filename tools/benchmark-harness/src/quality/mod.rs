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
//! The layout component (`f1_layout`) is canonical SF1 from
//! [`structural_sidecar`] and captures structural fidelity across paragraph,
//! heading, list, table, binding-edge, and reading-order dimensions.
//!
//! # Tokenization
//!
//! Tokenization is intentionally simple: NFKC-normalize, lowercase, split on whitespace
//! (with `|` also treated as a separator so markdown table cell padding cannot change the
//! token stream), strip non-alphanumeric characters except periods and commas embedded
//! between alphanumeric characters (preserving decimal numbers like "3.14" and European
//! format "3,14"). CJK runs use character bigrams because Chinese, Japanese, and Korean OCR
//! engines commonly insert layout-dependent line breaks mid-word; bigrams never cross a
//! whitespace/line boundary, so reordered lines are still detected as errors. This preserves
//! punctuation that is semantically meaningful while ignoring decorative punctuation.
//!
//! # Reading order (report-only)
//!
//! `f1_score_text` is a bag-of-tokens (multiset) F1: it is order-insensitive by
//! construction, so it cannot detect reading-order failure (a fully scrambled document can
//! still score near-perfect F1 as long as every token survives somewhere). This matters
//! because vertical-Japanese OCR fails precisely by scrambling reading order.
//! [`reading_order_score`] measures order fidelity separately via anchor-based LIS. It is
//! computed and reported on [`QualityMetrics`] but is **intentionally not folded into
//! `quality_score`** pending corpus-wide distribution analysis — see that function's docs.

use crate::types::{OutputFormat, QualityMetrics};
use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;
use unicode_normalization::UnicodeNormalization;

// The structural-sidecar file lives at `src/structural_sidecar.rs`; it is attached ~keep
// here (rather than in `lib.rs`) via `#[path]` so the crate root stays untouched. The path is
// relative to this file's directory (`src/quality/`), hence `../`. ~keep
pub mod geometry;
#[path = "../structural_sidecar.rs"]
pub mod structural_sidecar;

/// Regex to strip markdown image syntax `![alt](url)` → `alt`
static MD_IMAGE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"!\[([^\]]*)\]\([^)]*\)").expect("invalid regex"));

/// Regex to strip markdown link syntax `[text](url)` → `text`
static MD_LINK_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[([^\]]*)\]\([^)]*\)").expect("invalid regex"));

/// Regex matching bare (non-markdown-linked) URLs. Run AFTER [`MD_LINK_RE`]/[`MD_IMAGE_RE`],
/// which already consume the url portion of markdown links, so this only matches URLs that
/// were never wrapped in link syntax. Without this, a bare url survives the alphanumeric
/// filter as junk (`https://example.com` -> `httpsexamplecom`), penalizing precision for a
/// framework that emits bare links relative to one that emits markdown links — both discard
/// the url the same way here. ~keep
static BARE_URL_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?:https?://|www\.)\S+").expect("invalid regex"));

/// Strip markdown link and image syntax so URL components don't become tokens.
/// `![alt](url)` → `alt`, `[text](url)` → `text`, and a bare `https://…`/`www.…` URL is
/// dropped entirely for parity with the linked case.
fn strip_markdown_links(text: &str) -> String {
    let text = MD_IMAGE_RE.replace_all(text, "$1");
    let text = MD_LINK_RE.replace_all(&text, "$1");
    BARE_URL_RE.replace_all(&text, "").into_owned()
}

/// Compute quality metrics comparing extracted text against ground truth,
/// optionally including structural quality scoring when markdown GT is available.
///
/// When `output_format` is `Markdown` and `ground_truth_markdown` is `Some`, computes
/// structural F1 from markdown block comparison and adjusts the quality_score formula:
///   quality_score = 0.5 * f1_text + 0.2 * f1_numeric + 0.3 * f1_layout
///
/// When `output_format` is `Plaintext`, returns text-only scoring regardless of
/// markdown ground truth availability:
///   quality_score = 0.6 * f1_text + 0.4 * f1_numeric
///   f1_score_layout = None
///
/// When `output_format` is `Markdown` but `ground_truth_markdown` is `None`, falls back
/// to text-only scoring:
///   quality_score = 0.6 * f1_text + 0.4 * f1_numeric
pub fn compute_quality_with_structure(
    extracted: &str,
    ground_truth: &str,
    ground_truth_markdown: Option<&str>,
    output_format: OutputFormat,
) -> QualityMetrics {
    if output_format == OutputFormat::Plaintext {
        return compute_quality(extracted, ground_truth);
    }

    let mut metrics = compute_quality(extracted, ground_truth);

    if let Some(md_gt) = ground_truth_markdown {
        let structural_f1 = structural_sidecar::score_markdown(extracted, md_gt).sf1;
        metrics.f1_score_layout = Some(structural_f1);
        // Gated on the ground truth alone, not "either side" — see `compute_quality` for the
        // full rationale. Numeric fidelity cannot be measured against a reference that has no
        // numbers, so a stray extracted digit (page number, footnote marker) must not pull in
        // the numeric-weighted formula and collapse the score. ~keep
        metrics.quality_score = if ground_truth_has_numeric_tokens(ground_truth) {
            0.5 * metrics.f1_score_text + 0.2 * metrics.f1_score_numeric + 0.3 * structural_f1
        } else {
            0.625 * metrics.f1_score_text + 0.375 * structural_f1
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
/// 7. quality_score = 0.6 * f1_text + 0.4 * f1_numeric — but ONLY when the ground truth
///    itself contains numeric tokens; see the gating note below.
/// 8. `reading_order_score` is computed separately (anchor-based LIS, see that function's
///    docs) and attached REPORT-ONLY — it does not participate in `quality_score`.
pub fn compute_quality(extracted: &str, ground_truth: &str) -> QualityMetrics {
    let extracted_tokens = tokenize(extracted);
    let truth_tokens = tokenize(ground_truth);

    let f1_score_text = compute_f1(&extracted_tokens, &truth_tokens);

    let extracted_numeric = filter_numeric(&extracted_tokens);
    let truth_numeric = filter_numeric(&truth_tokens);
    let f1_score_numeric = compute_f1(&extracted_numeric, &truth_numeric);

    // Gate the numeric component on the GROUND TRUTH only, not "either side has numerics".
    // Numeric fidelity cannot be measured against a reference that contains no numbers, so an
    // extraction that emits a stray digit (page number, footnote marker, a "Page 3 of 12"
    // footer the ground-truth generator stripped) must not fall through to the 0.6/0.4 split
    // and take a 40% penalty for a header/footer convention mismatch. The extra numeric token
    // still costs precision inside `f1_score_text` via `compute_f1`, so it is not unpunished.
    //
    // This is deliberately ASYMMETRIC: when the ground truth DOES have numerics and the
    // extraction dropped them, `truth_numeric` is non-empty, so this still routes into the
    // 0.6/0.4 split, and `compute_f1` still returns 0.0 for `f1_score_numeric` (one side
    // empty, the other not) — that is a genuine recall failure and must still cost 40%. ~keep
    let quality_score = if truth_numeric.is_empty() {
        f1_score_text
    } else {
        0.6 * f1_score_text + 0.4 * f1_score_numeric
    };

    let (missing_tokens, extra_tokens) = compute_token_diff(&extracted_tokens, &truth_tokens);

    let correct = quality_score >= 0.95;

    // REPORT-ONLY: computed alongside the other scores but never weighted into
    // `quality_score` — see the module-level "Reading order" doc and `reading_order_score`.
    let reading_order_score = reading_order_score_from_tokens(&extracted_tokens, &truth_tokens);

    QualityMetrics {
        f1_score_text,
        f1_score_numeric,
        f1_score_layout: None,
        quality_score,
        missing_tokens,
        extra_tokens,
        correct,
        reading_order_score,
    }
}

/// Minimum anchor count required to report [`reading_order_score`]. Below this, the order
/// signal is too sparse to be meaningful, so the function returns `None` rather than a
/// number computed from a handful of coincidental unique tokens.
const MIN_READING_ORDER_ANCHORS: usize = 8;

/// Reading-order fidelity between `extracted` and `ground_truth`, via anchor-based Longest
/// Increasing Subsequence (LIS).
///
/// REPORT-ONLY: this score is intentionally excluded from `quality_score`. Several other
/// metrics in this module changed today; weighting a brand-new metric into the composite
/// score now would silently re-baseline every previously published number. It is surfaced
/// on [`QualityMetrics::reading_order_score`] for visibility pending corpus-wide
/// distribution analysis.
///
/// # Algorithm
/// 1. Tokenize both sides with [`tokenize`] (reused as-is — NFKC normalization and CJK
///    bigramming already make it order-sensitive at the line-break boundary).
/// 2. An **anchor** is a token that occurs exactly once in `ground_truth` AND exactly once
///    in `extracted` — an unambiguous 1:1 positional correspondence between the two token
///    streams. Repeated tokens are excluded because they have no single position to anchor.
/// 3. Anchors are ordered by their ground-truth index, and `reading_order_score` is the
///    length of the Longest Increasing Subsequence of their extracted-side indices —
///    computed via patience sorting in `O(n log n)`, not `O(n^2)` LIS or edit distance/LCS
///    (both `O(n*m)`), because some extractions in this corpus run to ~140k tokens and a
///    quadratic algorithm is not viable at that scale — divided by the anchor count.
/// 4. Identical order -> `1.0`. Fully reversed order -> near `0.0`. This measures ONLY the
///    order of shared, unambiguous content; content correctness (missing/extra tokens) is
///    already `f1_score_text`'s job and is deliberately not re-litigated here.
///
/// Returns `None` when either side tokenizes to nothing, or when fewer than
/// [`MIN_READING_ORDER_ANCHORS`] anchors are found — insufficient signal to report a
/// number rather than fabricate one.
pub fn reading_order_score(extracted: &str, ground_truth: &str) -> Option<f64> {
    let extracted_tokens = tokenize(extracted);
    let truth_tokens = tokenize(ground_truth);
    reading_order_score_from_tokens(&extracted_tokens, &truth_tokens)
}

fn reading_order_score_from_tokens(extracted_tokens: &[String], truth_tokens: &[String]) -> Option<f64> {
    if extracted_tokens.is_empty() || truth_tokens.is_empty() {
        return None;
    }

    let anchor_extracted_indices = anchor_extracted_indices_by_gt_order(extracted_tokens, truth_tokens);
    if anchor_extracted_indices.len() < MIN_READING_ORDER_ANCHORS {
        return None;
    }

    let lis_len = longest_increasing_subsequence_len(&anchor_extracted_indices);
    Some(lis_len as f64 / anchor_extracted_indices.len() as f64)
}

/// Extracted-side indices of anchor tokens (tokens occurring exactly once on both sides),
/// ordered by their ground-truth position.
fn anchor_extracted_indices_by_gt_order(extracted_tokens: &[String], truth_tokens: &[String]) -> Vec<usize> {
    let extracted_counts = build_counts(extracted_tokens);
    let truth_counts = build_counts(truth_tokens);

    let mut extracted_index_by_token: HashMap<&str, usize> = HashMap::new();
    for (index, token) in extracted_tokens.iter().enumerate() {
        extracted_index_by_token.entry(token.as_str()).or_insert(index);
    }

    // `truth_tokens` is iterated in ground-truth order, so pushing in this order already
    // yields the ground-truth-ordered sequence — no separate sort by `gt_index` is needed.
    truth_tokens
        .iter()
        .filter(|token| truth_counts.get(token.as_str()).copied() == Some(1))
        .filter(|token| extracted_counts.get(token.as_str()).copied() == Some(1))
        .filter_map(|token| extracted_index_by_token.get(token.as_str()).copied())
        .collect()
}

/// Length of the longest strictly increasing subsequence, via patience sorting: `O(n log
/// n)` time and space rather than the naive `O(n^2)` DP.
fn longest_increasing_subsequence_len(sequence: &[usize]) -> usize {
    let mut pile_tops: Vec<usize> = Vec::new();
    for &value in sequence {
        match pile_tops.binary_search(&value) {
            Ok(_) => {}
            Err(insert_at) if insert_at == pile_tops.len() => pile_tops.push(value),
            Err(insert_at) => pile_tops[insert_at] = value,
        }
    }
    pile_tops.len()
}

/// Zero-width/invisible formatting characters that must never surface as, or silently fuse,
/// token content. NFKC normalization has no compatibility decomposition for these (they are
/// format characters, not letters), so they are stripped explicitly rather than relying on
/// the alphanumeric filter below to happen to exclude them. ~keep
const INVISIBLE_CHARACTERS: [char; 5] = [
    '\u{00ad}', // soft hyphen
    '\u{200b}', // zero width space
    '\u{200c}', // zero width non-joiner
    '\u{200d}', // zero width joiner
    '\u{feff}', // zero width no-break space / BOM
];

/// Tokenize text: NFKC-normalize, lowercase, split on whitespace (`|` also acts as a
/// separator so markdown table cell padding — `|a|b|` vs `| a | b |` — cannot change the
/// token stream), strip non-alphanumeric characters (preserving `.` and `,` only when
/// embedded between alphanumeric chars, e.g. "3.14", "3,14").
///
/// NFKC folds compatibility forms — fullwidth digits (`１２３` -> `123`), ligatures (`ﬁ` ->
/// `fi`) — into their canonical form so semantically identical text always tokenizes the same
/// regardless of source-encoding quirks. Applied BEFORE the alphanumeric filter. ~keep
pub fn tokenize(text: &str) -> Vec<String> {
    let text = strip_markdown_links(text);
    let text: String = text.chars().filter(|c| !INVISIBLE_CHARACTERS.contains(c)).collect();
    let text: String = text.nfkc().collect();
    let tokens = text
        .to_lowercase()
        .replace('|', " ")
        .split_whitespace()
        .map(|w| {
            let kept: String = w
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '.' || *c == ',')
                .collect();
            kept.trim_matches(|c: char| c == '.' || c == ',').to_string()
        })
        .filter(|w| !w.is_empty())
        .collect();
    tokenize_cjk_bigrams(tokens)
}

fn normalize_numeric_token(token: String) -> String {
    let digit_count = token.chars().filter(|c| c.is_ascii_digit()).count();
    if digit_count == 0 || digit_count > 15 {
        return token;
    }
    // Normalize thousands separators ("1,000" -> "1000") before the numeric parse so a
    // grouped number and its bare form become the same token. Only strip commas that form
    // well-shaped 3-digit groups, to avoid corrupting European decimals like "3,14". ~keep
    let candidate = if is_thousands_grouped(&token) {
        token.replace(',', "")
    } else {
        token.clone()
    };
    candidate
        .parse::<f64>()
        .map_or(token.clone(), |number| format!("{number}"))
}

/// Expand CJK script runs into overlapping bigrams while preserving non-CJK tokens.
///
/// The CJK accumulator is scoped to a single whitespace-delimited token (line/word) and is
/// always flushed at that boundary — bigrams never span a whitespace or line break. Bigrams
/// within a contiguous CJK run (including one interrupted only by decorative punctuation) are
/// still correct and desirable; bigrams welded across a line break are not, because that
/// erases the very reading-order/layout information the benchmark needs to score against —
/// two whitespace-adjacent CJK lines that were reordered or reassembled out of sequence would
/// otherwise contribute almost the same bigram multiset as the correctly-ordered document. ~keep
fn tokenize_cjk_bigrams(tokens: Vec<String>) -> Vec<String> {
    let mut result = Vec::new();
    for token in tokens {
        let mut cjk_run = String::new();
        let characters: Vec<char> = token.chars().collect();
        let mut start = 0;
        while start < characters.len() {
            let is_cjk = is_cjk_character(characters[start]);
            let mut end = start + 1;
            while end < characters.len() && is_cjk_character(characters[end]) == is_cjk {
                end += 1;
            }
            let run: String = characters[start..end].iter().collect();
            if is_cjk {
                cjk_run.push_str(&run);
            } else if run.chars().all(|character| matches!(character, '.' | ',')) {
                // Punctuation between CJK characters is decorative and must not split the run. ~keep
            } else {
                push_cjk_bigrams(&mut result, &mut cjk_run);
                result.push(normalize_numeric_token(run));
            }
            start = end;
        }
        push_cjk_bigrams(&mut result, &mut cjk_run);
    }
    result
}

fn push_cjk_bigrams(result: &mut Vec<String>, run: &mut String) {
    let characters: Vec<char> = run.chars().collect();
    if characters.len() == 1 {
        result.push(run.clone());
    } else {
        result.extend(characters.windows(2).map(|pair| pair.iter().collect()));
    }
    run.clear();
}

/// Whether a character belongs to a Chinese, Japanese, or Korean script block.
fn is_cjk_character(character: char) -> bool {
    matches!(
        character,
        '\u{1100}'..='\u{11ff}'
            | '\u{2e80}'..='\u{2eff}'
            // Japanese iteration marks such as `々` survive alphanumeric filtering; excluding
            // them would disable bigram expansion for an entire whitespace-free OCR line. ~keep
            | '\u{3000}'..='\u{303f}'
            | '\u{3040}'..='\u{30ff}'
            | '\u{3130}'..='\u{318f}'
            | '\u{31f0}'..='\u{31ff}'
            | '\u{3400}'..='\u{4dbf}'
            | '\u{4e00}'..='\u{9fff}'
            | '\u{a960}'..='\u{a97f}'
            | '\u{ac00}'..='\u{d7af}'
            | '\u{d7b0}'..='\u{d7ff}'
            | '\u{f900}'..='\u{faff}'
            | '\u{ff66}'..='\u{ff9f}'
            | '\u{20000}'..='\u{2fa1f}'
    )
}

/// Whether a numeric token uses `,` as a thousands separator in well-formed 3-digit groups
/// (e.g. `1,000`, `12,345,678`, `1,234.56`) — as opposed to a European decimal comma (`3,14`),
/// which must be left untouched.
fn is_thousands_grouped(token: &str) -> bool {
    let Some(int_part) = token.split('.').next() else {
        return false;
    };
    let groups: Vec<&str> = int_part.split(',').collect();
    if groups.len() < 2 {
        return false;
    }
    if groups[0].is_empty() || groups[0].len() > 3 || !groups[0].bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    groups[1..]
        .iter()
        .all(|g| g.len() == 3 && g.bytes().all(|b| b.is_ascii_digit()))
}

/// Whether the ground truth contains any numeric tokens (used to gate the numeric-weighted
/// scoring formula). Deliberately checks the ground truth ONLY — see `compute_quality`'s
/// gating note for why "either side has numerics" over-penalizes extraction-only digits. ~keep
fn ground_truth_has_numeric_tokens(ground_truth: &str) -> bool {
    !filter_numeric(&tokenize(ground_truth)).is_empty()
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
        return 1.0;
    }
    if extracted.is_empty() || truth.is_empty() {
        return 0.0;
    }

    let extracted_counts = build_counts(extracted);
    let truth_counts = build_counts(truth);

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

/// Above this character length, char-level edit distance (O(n·m)) is too slow to
/// run per document, so the CER helpers report NaN rather than stall the bench.
const CER_MAX_CHARS: usize = 16_384;

/// Levenshtein edit distance between two char slices. Two-row DP: O(n·m) time,
/// O(min(n, m)) space.
fn edit_distance(a: &[char], b: &[char]) -> usize {
    let (a, b) = if a.len() < b.len() { (b, a) } else { (a, b) };
    if b.is_empty() {
        return a.len();
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr = vec![0usize; b.len() + 1];
    for (i, &ca) in a.iter().enumerate() {
        curr[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let substitution = prev[j] + usize::from(ca != cb);
            curr[j + 1] = substitution.min(prev[j + 1] + 1).min(curr[j] + 1);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b.len()]
}

/// Character error rate: edit distance normalized by the reference length. 0.0
/// is identical; values above 1.0 are possible when the hypothesis runs long.
/// Returns NaN for an empty reference or when either side exceeds
/// [`CER_MAX_CHARS`] (the metric is a report-only OCR diagnostic).
pub fn char_error_rate(reference: &str, hypothesis: &str) -> f64 {
    let reference_chars: Vec<char> = reference.chars().collect();
    let hypothesis_chars: Vec<char> = hypothesis.chars().collect();
    if reference_chars.is_empty() || reference_chars.len().max(hypothesis_chars.len()) > CER_MAX_CHARS {
        return f64::NAN;
    }
    edit_distance(&reference_chars, &hypothesis_chars) as f64 / reference_chars.len() as f64
}

/// Order- and character-sensitive text similarity in `[0, 1]`: `1 − distance /
/// max(len)`. 1.0 is identical, 0.0 fully dissimilar. Complements the
/// bag-of-words TF1 by penalizing transpositions and character-level OCR slips.
/// Returns NaN when both sides are empty or either exceeds [`CER_MAX_CHARS`].
pub fn normalized_edit_similarity(a: &str, b: &str) -> f64 {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let max_len = a_chars.len().max(b_chars.len());
    if max_len == 0 || max_len > CER_MAX_CHARS {
        return f64::NAN;
    }
    1.0 - edit_distance(&a_chars, &b_chars) as f64 / max_len as f64
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
mod tests;

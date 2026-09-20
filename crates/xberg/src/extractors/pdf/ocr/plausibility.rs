//! Language/dictionary-plausibility detection for native PDF text layers (issue #1696).
//!
//! A Type0 font whose own `/ToUnicode` CMap (or embedded character mapping) resolves every
//! glyph to *a* character, but consistently to the WRONG letter (e.g. a ROT-shifted mapping),
//! produces text that is structurally indistinguishable from real prose: the same
//! alphanumeric ratio, the same word-length distribution, the same fragmentation that
//! [`super::scoring::NativeTextStats`] measures all look clean. A character-shape heuristic
//! cannot see this failure mode even in principle, because the failure is in *which* letters
//! were chosen, not in their shapes. This module adds a content-plausibility signal instead:
//! does the decoded text actually read as a real, detectable language.
#![cfg(any(feature = "ocr", feature = "ocr-pipeline"))]

use crate::core::config::OcrQualityThresholds;
use crate::types::PageBoundary;

/// Minimum alphabetic characters in a whitespace-delimited token for it to count toward a
/// prose line's alphabetic-word tally. Mirrors the same floor
/// [`super::scoring::NativeTextStats`] uses for "meaningful" tokens elsewhere in this crate,
/// so a stray one-letter fragment (an initial, a bullet glyph) is not counted as prose on its
/// own. ~keep
const PROSE_WORD_MIN_ALPHA_CHARS: usize = 2;
/// Minimum fraction of a token's characters that must be alphabetic for it to count as a
/// prose word. Excludes tokens that are mostly punctuation or digits with an incidental
/// letter (`"3rd"`, `"a)"`) from the prose tally without excluding ordinary
/// hyphenated/apostrophized words. ~keep
const PROSE_WORD_MIN_ALPHA_RATIO: f64 = 0.75;
/// Minimum count of qualifying alphabetic words for a line to be treated as prose at all.
/// Below this a line is more likely a table row, a numbered caption, or a formula fragment
/// than a sentence — exactly the content a language-plausibility check must not be asked to
/// judge, since those are legitimately non-linguistic (issue #1341's Markdown-scoring lesson
/// applied to a different signal). ~keep
const PROSE_LINE_MIN_ALPHA_WORDS: usize = 5;
/// Maximum fraction of a candidate prose line's characters that may be ASCII digits before
/// the line is excluded as tabular/numeric rather than prose. ~keep
const PROSE_LINE_MAX_DIGIT_RATIO: f64 = 0.15;
/// Maximum fraction of a candidate prose line's characters that may be formula-shaped symbols
/// (see [`FORMULA_SYMBOLS`]) before the line is excluded as a formula fragment rather than
/// prose. ~keep
const PROSE_LINE_MAX_SYMBOL_RATIO: f64 = 0.10;
/// Characters whose presence marks a line as formula-shaped rather than prose. Deliberately
/// narrow: ordinary sentence punctuation (`,`, `.`, `'`, `"`, `-`) is excluded so a normal
/// sentence is never penalized by this ratio. ~keep
const FORMULA_SYMBOLS: &[char] = &[
    '=', '+', '*', '/', '^', '∑', '∫', '√', '±', '≤', '≥', '<', '>', '(', ')', '[', ']', '{', '}', '|', '\\', '%', '∂',
    '∞', 'π', 'θ', 'α', 'β', 'δ', 'λ', 'μ', 'σ',
];
/// A trailing chunk shorter than this many characters is dropped rather than evaluated on its
/// own — too little text for whatlang's per-chunk detection to be meaningful, and keeping it
/// would let a short, unlucky tail chunk drag down an otherwise reliable page. ~keep
const PLAUSIBILITY_MIN_TAIL_CHUNK_CHARS: usize = 100;
/// Minimum number of full prose chunks (of [`crate::language_detection::CHUNK_SIZE`] chars
/// each, so >= 600 prose characters) before a page's plausibility is judged at all. Pages with
/// less prose than this — a numeric table, a sparse form, a formula-only page — abstain
/// (`PlausibilityVerdict::NotEvaluated`) rather than risk a confident wrong answer from too
/// little evidence. ~keep
const PLAUSIBILITY_MIN_PROSE_CHUNKS: usize = 3;
/// The second guard on [`PlausibilityVerdict::Implausible`]: even when a page's reliable-chunk
/// ratio is below threshold, it is only flagged when the mean chunk confidence is ALSO below
/// this ceiling. Protects genuinely legible text in a close language pair (e.g. Danish vs.
/// Norwegian Bokmål) whose *margin* between the top two guesses is thin — low reliable_ratio —
/// but whose *raw* confidence in either guess stays high, unlike truly wrong-mapped text whose
/// confidence collapses on every axis at once.
///
/// Calibrated against the real `wrong_mapping` corpus (issue #1696), not chosen a priori: a
/// ROT-3-shifted real-world README page (`wrong_mapping_276728418.pdf`, page 1, 4 prose
/// chunks) measured `reliable_ratio == 0.0` but `mean_confidence == 0.303` -- whatlang found
/// zero chunks it called reliable, yet its raw confidence in *some* guess still cleared a
/// 0.25 ceiling on this real, punctuation-and-bullet-heavy text (a synthetic pure-prose ROT-3
/// fixture measures well under 0.25 and does not need the higher ceiling). 0.35 keeps a
/// margin above the measured 0.303 without approaching whatlang's own 0.9 reliability bar. ~keep
const PLAUSIBILITY_MAX_MEAN_CONFIDENCE: f64 = 0.35;

/// The outcome of judging a span of text for language/dictionary plausibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PlausibilityVerdict {
    /// Too little prose content to judge (fewer than [`PLAUSIBILITY_MIN_PROSE_CHUNKS`] chunks).
    NotEvaluated,
    /// The prose content reads as a real, detectable language.
    Plausible,
    /// The prose content does not read as any real language whatlang can detect reliably.
    Implausible,
}

/// Whether a whitespace-delimited token counts as a prose word.
pub(super) fn is_prose_word(word: &str) -> bool {
    let total = word.chars().count();
    if total == 0 {
        return false;
    }
    let alpha = word.chars().filter(|c| c.is_alphabetic()).count();
    alpha >= PROSE_WORD_MIN_ALPHA_CHARS && (alpha as f64 / total as f64) >= PROSE_WORD_MIN_ALPHA_RATIO
}

/// Whether a single line of text qualifies as a prose line for plausibility scoring.
pub(super) fn is_prose_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }

    let alpha_words = trimmed.split_whitespace().filter(|word| is_prose_word(word)).count();
    if alpha_words < PROSE_LINE_MIN_ALPHA_WORDS {
        return false;
    }

    let total_chars = trimmed.chars().count();
    let digit_ratio = trimmed.chars().filter(char::is_ascii_digit).count() as f64 / total_chars as f64;
    if digit_ratio > PROSE_LINE_MAX_DIGIT_RATIO {
        return false;
    }

    let symbol_ratio = trimmed.chars().filter(|c| FORMULA_SYMBOLS.contains(c)).count() as f64 / total_chars as f64;
    symbol_ratio <= PROSE_LINE_MAX_SYMBOL_RATIO
}

/// Select and concatenate the prose lines out of `text`, dropping table rows, numeric
/// captions, and formula fragments (see [`is_prose_line`]).
pub(super) fn select_prose_text(text: &str) -> String {
    text.lines()
        .filter(|line| is_prose_line(line))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Split prose text into [`crate::language_detection::CHUNK_SIZE`]-character chunks, dropping
/// a short trailing chunk (see [`PLAUSIBILITY_MIN_TAIL_CHUNK_CHARS`]).
pub(super) fn prose_chunks(prose_text: &str) -> Vec<String> {
    let chars: Vec<char> = prose_text.chars().collect();
    let mut chunks: Vec<String> = chars
        .chunks(crate::language_detection::CHUNK_SIZE)
        .map(|chunk| chunk.iter().collect())
        .collect();

    if chunks
        .last()
        .is_some_and(|tail| tail.chars().count() < PLAUSIBILITY_MIN_TAIL_CHUNK_CHARS)
    {
        chunks.pop();
    }

    chunks
}

/// Judge a span of text's language/dictionary plausibility.
///
/// Does not consult the character-shape gate ([`super::scoring::evaluate_native_text_for_ocr`])
/// itself — callers that need to skip already-routed pages do so before calling this.
pub(super) fn evaluate_text_plausibility(text: &str, thresholds: &OcrQualityThresholds) -> PlausibilityVerdict {
    let prose_text = select_prose_text(text);
    let chunks = prose_chunks(&prose_text);

    if chunks.len() < PLAUSIBILITY_MIN_PROSE_CHUNKS {
        return PlausibilityVerdict::NotEvaluated;
    }

    let chunk_refs: Vec<&str> = chunks.iter().map(String::as_str).collect();
    let reliability = crate::language_detection::chunk_reliability(&chunk_refs);

    let implausible = reliability.reliable_ratio() < thresholds.min_reliable_language_chunk_ratio
        && reliability.mean_confidence() < PLAUSIBILITY_MAX_MEAN_CONFIDENCE;

    if implausible {
        PlausibilityVerdict::Implausible
    } else {
        PlausibilityVerdict::Plausible
    }
}

/// One page's verdict, or `None` when the page never reached the plausibility check.
///
/// `None` covers a boundary that does not slice `native_text` on a character boundary, and a
/// page the character-shape gate already routes to OCR on its own
/// (`evaluate_native_text_for_ocr(..).fallback`). The plausibility signal exists to catch what
/// that gate cannot see, not to duplicate what it already catches, and a page that gate
/// already routes cannot mislead the caller about its text.
pub(super) fn page_verdict(
    native_text: &str,
    boundary: &PageBoundary,
    thresholds: &OcrQualityThresholds,
) -> Option<PlausibilityVerdict> {
    if boundary.byte_start > boundary.byte_end
        || !native_text.is_char_boundary(boundary.byte_start)
        || !native_text.is_char_boundary(boundary.byte_end)
    {
        return None;
    }

    let page_text = &native_text[boundary.byte_start..boundary.byte_end];
    if super::scoring::evaluate_native_text_for_ocr(page_text, Some(1), thresholds).fallback {
        return None;
    }

    Some(evaluate_text_plausibility(page_text, thresholds))
}

/// The whole document's verdict when no page boundaries are available. `None` means what it
/// means in [`page_verdict`].
pub(super) fn whole_document_verdict(
    native_text: &str,
    thresholds: &OcrQualityThresholds,
) -> Option<PlausibilityVerdict> {
    if super::scoring::evaluate_native_text_for_ocr(native_text, Some(1), thresholds).fallback {
        return None;
    }

    Some(evaluate_text_plausibility(native_text, thresholds))
}

/// What the language-plausibility check made of a document (issues #1696 and #1709).
///
/// `implausible` on its own cannot answer "was this document checked at all". It is empty both
/// for a document whose every page read as a real language and for a document no page of which
/// held enough prose to judge. `judged` separates those two, so an abstention is visible
/// instead of looking like a clean bill of health.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct PlausibilityScan {
    /// Pages (1-indexed) whose native text layer reads as no real detectable language.
    pub implausible: Vec<u32>,
    /// Pages (1-indexed) the check examined and could not judge: fewer than
    /// [`PLAUSIBILITY_MIN_PROSE_CHUNKS`] chunks of prose to read.
    pub unjudged: Vec<u32>,
    /// How many pages the check judged, plausible or implausible together. Zero means the
    /// check produced no verdict anywhere in the document.
    pub judged: u32,
}

/// Scan a document for language/dictionary plausibility, page by page, per
/// [`OcrQualityThresholds::enable_plausibility_ocr_routing`] and
/// [`OcrQualityThresholds::min_reliable_language_chunk_ratio`] (issue #1696).
///
/// Empty when routing is disabled. Without page boundaries there is no subset to attribute the
/// signal to, so the whole-document verdict applies to every page from `1` to `page_count`
/// (mirroring [`super::scoring::apply_flagged_pages`]'s no-boundaries behavior); `page_count`
/// defaults to `1` when absent, matching the rest of this module's single-page fallback
/// convention.
pub(crate) fn scan_text_plausibility(
    native_text: &str,
    boundaries: Option<&[PageBoundary]>,
    page_count: Option<u32>,
    thresholds: &OcrQualityThresholds,
) -> PlausibilityScan {
    let mut scan = PlausibilityScan::default();
    if !thresholds.enable_plausibility_ocr_routing {
        return scan;
    }

    match boundaries {
        Some(bounds) if !bounds.is_empty() => {
            for boundary in bounds {
                match page_verdict(native_text, boundary, thresholds) {
                    Some(PlausibilityVerdict::Implausible) => {
                        scan.implausible.push(boundary.page_number);
                        scan.judged += 1;
                    }
                    Some(PlausibilityVerdict::Plausible) => scan.judged += 1,
                    Some(PlausibilityVerdict::NotEvaluated) => scan.unjudged.push(boundary.page_number),
                    None => {}
                }
            }
        }
        _ => {
            let pages = page_count.unwrap_or(1).max(1);
            match whole_document_verdict(native_text, thresholds) {
                Some(PlausibilityVerdict::Implausible) => {
                    scan.implausible = (1..=pages).collect();
                    scan.judged = pages;
                }
                Some(PlausibilityVerdict::Plausible) => scan.judged = pages,
                Some(PlausibilityVerdict::NotEvaluated) => scan.unjudged = (1..=pages).collect(),
                None => {}
            }
        }
    }

    scan
}

//! Text repair utilities for PDF extraction.
//!
//! Handles three classes of text corruption common in PDFs with broken font encodings:
//!
//! 1. **Ligature corruption** – contextual heuristics repair ligature glyphs
//!    (fi, fl, ff, ffi, ffl) that map to wrong characters.
//!
//! 2. **Broken word spacing** – PDFs with broken CMap/ToUnicode tables may cause
//!    spaces to be inserted mid-word. Repaired by rejoining single-letter fragments.
//!
//! 3. **Unicode normalization** – curly quotes, fraction slash, and other PDF-specific
//!    Unicode characters are normalized to their ASCII equivalents.

use std::borrow::Cow;

use super::types::PdfParagraph;

/// Lowercased standalone alphabetic words the document itself writes elsewhere,
/// used by [`repair_ligature_spaces`] to tell a genuine word boundary apart from
/// a decomposed-ligature gap that looks identical at the string layer (#1591).
/// Collected once per document by `pipeline::collect_word_witnesses` and
/// threaded alongside the document's hyphen witnesses. ~keep
pub(super) type WordWitnesses = ahash::AHashSet<String>;

/// Minimum letters a fragment must have before it can count as evidence of a
/// standalone word. A single decomposed-ligature fragment (the bare `f` in
/// `f irst`) is common and must never witness itself. ~keep
pub(super) const MIN_LIGATURE_WITNESS_WORD_LEN: usize = 2;

pub(super) struct TextReplacement {
    pub before: std::ops::Range<usize>,
    pub after: std::ops::Range<usize>,
}

pub(super) struct RepairedText<'a> {
    pub text: Cow<'a, str>,
    pub replacements: Vec<TextReplacement>,
}

impl<'a> RepairedText<'a> {
    fn unchanged(text: &'a str) -> Self {
        Self {
            text: Cow::Borrowed(text),
            replacements: Vec::new(),
        }
    }
}

fn record_replacement(
    replacements: &mut Vec<TextReplacement>,
    enabled: bool,
    before: std::ops::Range<usize>,
    after: std::ops::Range<usize>,
) {
    if enabled {
        replacements.push(TextReplacement { before, after });
    }
}

/// Repair ligature corruption using contextual heuristics.
///
/// Some PDF fonts have broken ToUnicode CMaps that map ligature glyphs to
/// ASCII characters. This function detects and repairs these patterns:
///
/// **f-ligatures**: `!` → fi/ff, `"` → ffi, `#` → fi/fl
/// **t-ligature**: `*` → tt
///
/// All patterns are contextual: the corrupt character must appear between
/// alphabetic characters (mid-word), where it virtually never occurs in real text.
pub(super) fn repair_contextual_ligatures(text: &str) -> Cow<'_, str> {
    repair_contextual_ligatures_mapped(text, false).text
}

pub(super) fn repair_contextual_ligatures_mapped(text: &str, record: bool) -> RepairedText<'_> {
    if text.len() < 2 {
        return RepairedText::unchanged(text);
    }

    let mut result = String::with_capacity(text.len() + 16);
    let mut repaired = false;
    let mut replacements = Vec::new();
    let bytes = text.as_bytes();
    let chars = text.chars().peekable();
    let mut byte_idx = 0;
    let mut prev_is_alpha = false;
    let mut prev_is_space_or_start = true;

    for ch in chars {
        let char_len = ch.len_utf8();
        let next_byte_idx = byte_idx + char_len;

        let next_is_alpha = if next_byte_idx < bytes.len() {
            if let Some(&next_byte) = bytes.get(next_byte_idx) {
                (next_byte as char).is_alphabetic()
            } else {
                false
            }
        } else {
            false
        };

        let next_is_lower = if next_byte_idx < bytes.len() {
            if let Some(&next_byte) = bytes.get(next_byte_idx) {
                (next_byte as char).is_lowercase()
            } else {
                false
            }
        } else {
            false
        };

        let next_is_vowel = if next_byte_idx < bytes.len() {
            if let Some(&next_byte) = bytes.get(next_byte_idx) {
                matches!(
                    next_byte as char,
                    'a' | 'e' | 'i' | 'o' | 'u' | 'A' | 'E' | 'I' | 'O' | 'U'
                )
            } else {
                false
            }
        } else {
            false
        };

        let output_start = result.len();
        match ch {
            '!' if prev_is_alpha && next_is_vowel => {
                result.push_str("ff");
                repaired = true;
            }
            '!' if prev_is_alpha && next_is_alpha => {
                result.push_str("fi");
                repaired = true;
            }
            // NOTE: deliberately NO letter+'!'+end-of-string repair. A sentence-final
            '"' if prev_is_alpha && next_is_alpha => {
                result.push_str("ffi");
                repaired = true;
            }
            '#' if prev_is_alpha && next_is_alpha => {
                result.push_str("fi");
                repaired = true;
            }
            '#' if prev_is_space_or_start && next_is_lower => {
                result.push_str("fi");
                repaired = true;
            }
            '!' if prev_is_space_or_start && next_is_lower => {
                result.push_str("fi");
                repaired = true;
            }
            '*' if prev_is_alpha && next_is_alpha => {
                result.push_str("tt");
                repaired = true;
            }
            // NOTE: deliberately NO letter+'*'+end/non-alpha repair.

            // Issue #1556 removed the ':'→"ti" and 'M'→"tti" arms: both are ordinary
            // characters that occur constantly in healthy text (ratios, times, URLs,
            // units like "nM"/"µM", identifiers), and corrupted arbitrary text
            // (e.g. "aMb" -> "attib"). The evidence that justified them at
            // introduction — a per-font broken-CMap signal from pdfium's
            // has_unicode_map_error() — no longer exists in this codebase; pdfium was
            // removed as a backend and that font-level detection was never ported to
            // pdf_oxide. Re-add only alongside a real document-level evidence gate. ~keep
            _ => result.push(ch),
        }

        if record && result[output_start..] != text[byte_idx..next_byte_idx] {
            record_replacement(
                &mut replacements,
                record,
                byte_idx..next_byte_idx,
                output_start..result.len(),
            );
        }
        prev_is_alpha = ch.is_alphabetic();
        prev_is_space_or_start = ch.is_whitespace();
        byte_idx = next_byte_idx;
    }

    if repaired {
        RepairedText {
            text: Cow::Owned(result),
            replacements,
        }
    } else {
        RepairedText::unchanged(text)
    }
}

/// Repair broken word spacing by joining short fragments to adjacent words.
///
/// Targets the pattern where the PDF extractor inserts spaces mid-word due to broken font
/// CMap/ToUnicode tables. Handles both single-character and multi-character
/// fragments:
/// - `"M ust Be Tough"` → `"Must Be Tough"` (single-char)
/// - `"s hall a b e active"` → `"shall be active"` (multi-char)
/// - `"a dd ress"` → `"address"` (mixed)
/// - `"sen d er"` → `"sender"` (mixed)
///
/// Only joins when:
/// - The fragment is a short alphabetic word (1-3 chars)
/// - It's not a common standalone short word
/// - The next word starts with a lowercase letter (continuation)
/// - Or the fragment is part of a run of consecutive short fragments
pub(in crate::pdf::structure) fn repair_broken_word_spacing(text: &str) -> Cow<'_, str> {
    if text.is_empty() {
        return Cow::Borrowed(text);
    }

    if text.contains("| --- |") || text.starts_with('|') {
        return Cow::Borrowed(text);
    }

    let words: Vec<&str> = text.split_whitespace().collect();

    let has_joinable = words.windows(2).any(|window| {
        is_joinable_fragment(window[0], window[1])
            || (window[0].chars().all(|c| c.is_alphabetic())
                && !is_common_short_word(window[0])
                && is_trailing_fragment(window[1]))
    });

    if !has_joinable {
        return Cow::Borrowed(text);
    }

    let mut result = String::with_capacity(text.len());
    let mut i = 0;
    while i < words.len() {
        if i > 0 && !result.is_empty() {
            result.push(' ');
        }

        let w = words[i];

        if w.len() == 1
            && w.chars().next().is_some_and(|c| c.is_alphabetic())
            && !is_common_short_word(w)
            && i + 1 < words.len()
            && words[i + 1].chars().next().is_some_and(|c| c.is_lowercase())
        {
            result.push_str(w);
            result.push_str(words[i + 1]);
            i += 2;
            continue;
        }

        if i + 1 < words.len() && is_joinable_fragment(w, words[i + 1]) {
            result.push_str(w);
            i += 1;
            let mut last_consumed_len = w.len();
            let mut total_consumed = w.len();
            while i < words.len() {
                let next = words[i];
                let next_starts_lower = next.chars().next().is_some_and(|c| c.is_lowercase());
                if !next_starts_lower {
                    break;
                }
                if last_consumed_len <= 3 && next.len() <= 3 {
                    result.push_str(next);
                    last_consumed_len = next.len();
                    total_consumed += next.len();
                    i += 1;
                    continue;
                }
                if total_consumed <= 3 {
                    result.push_str(next);
                    i += 1;
                    break;
                }
                break;
            }
            continue;
        }

        if i + 1 < words.len()
            && w.chars().all(|c| c.is_alphabetic())
            && !is_common_short_word(w)
            && is_trailing_fragment(words[i + 1])
        {
            result.push_str(w);
            while i + 1 < words.len() && is_trailing_fragment(words[i + 1]) {
                i += 1;
                result.push_str(words[i]);
            }
            i += 1;
            continue;
        }

        result.push_str(w);
        i += 1;
    }

    if result == text.split_whitespace().collect::<Vec<_>>().join(" ") {
        Cow::Borrowed(text)
    } else {
        Cow::Owned(result)
    }
}

/// Check if a word is a trailing fragment: very short (1-2 chars), all lowercase
/// alphabetic, and not a common standalone word. These are fragments that were
/// split off from the end of a word by the PDF extractor.
fn is_trailing_fragment(word: &str) -> bool {
    word.len() <= 2
        && !word.is_empty()
        && word.chars().all(|c| c.is_lowercase() && c.is_alphabetic())
        && !is_common_short_word(word)
}

/// Check if a word is a joinable fragment: short, alphabetic, not a common
/// standalone word, and followed by a lowercase-starting continuation.
fn is_joinable_fragment(word: &str, next: &str) -> bool {
    word.len() <= 3
        && !word.is_empty()
        && word.chars().all(|c| c.is_alphabetic())
        && !word.chars().all(|c| c.is_uppercase())
        && !is_common_short_word(word)
        && next.chars().next().is_some_and(|c| c.is_lowercase())
}

/// Check if a short word is a common standalone English word that should
/// not be joined to adjacent words.
///
/// Covers articles, pronouns, prepositions, conjunctions, and common verbs
/// up to 3 characters. This is intentionally conservative — when in doubt,
/// include the word to avoid false joins.
fn is_common_short_word(word: &str) -> bool {
    matches!(
        word,
        "a" | "A"
            | "I"
            | "an"
            | "am"
            | "as"
            | "at"
            | "be"
            | "by"
            | "do"
            | "go"
            | "he"
            | "if"
            | "in"
            | "is"
            | "it"
            | "me"
            | "my"
            | "no"
            | "of"
            | "oh"
            | "on"
            | "or"
            | "so"
            | "to"
            | "up"
            | "us"
            | "we"
            | "An"
            | "Am"
            | "As"
            | "At"
            | "Be"
            | "By"
            | "Do"
            | "Go"
            | "He"
            | "If"
            | "In"
            | "Is"
            | "It"
            | "Me"
            | "My"
            | "No"
            | "Of"
            | "Oh"
            | "On"
            | "Or"
            | "So"
            | "To"
            | "Up"
            | "Us"
            | "We"
            | "the"
            | "and"
            | "are"
            | "but"
            | "can"
            | "did"
            | "for"
            | "got"
            | "had"
            | "has"
            | "her"
            | "him"
            | "his"
            | "how"
            | "its"
            | "let"
            | "may"
            | "new"
            | "nor"
            | "not"
            | "now"
            | "old"
            | "one"
            | "our"
            | "out"
            | "own"
            | "ran"
            | "say"
            | "she"
            | "too"
            | "two"
            | "use"
            | "was"
            | "way"
            | "who"
            | "why"
            | "yet"
            | "you"
            | "all"
            | "any"
            | "big"
            | "day"
            | "end"
            | "far"
            | "few"
            | "put"
            | "run"
            | "saw"
            | "set"
            | "top"
            | "try"
            | "win"
            | "yes"
            | "The"
            | "And"
            | "Are"
            | "But"
            | "Can"
            | "Did"
            | "For"
            | "Got"
            | "Had"
            | "Has"
            | "Her"
            | "Him"
            | "His"
            | "How"
            | "Its"
            | "Let"
            | "May"
            | "New"
            | "Nor"
            | "Not"
            | "Now"
            | "Old"
            | "One"
            | "Our"
            | "Out"
            | "Own"
            | "Ran"
            | "Say"
            | "She"
            | "Too"
            | "Two"
            | "Use"
            | "Was"
            | "Way"
            | "Who"
            | "Why"
            | "Yet"
            | "You"
            | "All"
            | "Any"
            | "Big"
            | "Day"
            | "End"
            | "Far"
            | "Few"
            | "Put"
            | "Run"
            | "Saw"
            | "Set"
            | "Top"
            | "Try"
            | "Win"
            | "Yes"
    )
}

/// Expand Unicode ligature characters (U+FB00–U+FB06) to ASCII equivalents,
/// absorbing a spurious space between the ligature glyph and the following word.
///
/// PDFs sometimes emit ligature codepoints (ﬁ, ﬂ, ﬀ, ﬃ, ﬄ, ﬅ, ﬆ) that need
/// to be expanded. Additionally, a space is often inserted between the ligature
/// glyph and the continuation of the word (e.g. "ﬁ eld"), which must be absorbed
/// to produce correct text ("field").
///
/// Matches the reference approach:
/// ```python
/// _LIGATURE_RE = re.compile(r"([\ufb00-\ufb06])( (?=\w))?")
/// ```
///
/// Uses `Cow<str>` for zero-alloc fast path when no ligatures are present.
pub(super) fn expand_ligatures_with_space_absorption(text: &str) -> Cow<'_, str> {
    expand_ligatures_with_space_absorption_mapped(text, false).text
}

pub(super) fn expand_ligatures_with_space_absorption_mapped(text: &str, record: bool) -> RepairedText<'_> {
    if !text.contains([
        '\u{FB00}', '\u{FB01}', '\u{FB02}', '\u{FB03}', '\u{FB04}', '\u{FB05}', '\u{FB06}',
    ]) {
        return RepairedText::unchanged(text);
    }
    let mut result = String::with_capacity(text.len());
    let mut replacements = Vec::new();
    let mut chars = text.char_indices().peekable();
    while let Some((offset, ch)) = chars.next() {
        let expansion = match ch {
            '\u{FB00}' => "ff",
            '\u{FB01}' => "fi",
            '\u{FB02}' => "fl",
            '\u{FB03}' => "ffi",
            '\u{FB04}' => "ffl",
            '\u{FB05}' => "st",
            '\u{FB06}' => "st",
            _ => {
                result.push(ch);
                continue;
            }
        };
        let output_start = result.len();
        result.push_str(expansion);
        record_replacement(
            &mut replacements,
            record,
            offset..offset + ch.len_utf8(),
            output_start..result.len(),
        );
        if chars.peek().is_some_and(|(_, ch)| *ch == ' ') {
            let mut lookahead = chars.clone();
            lookahead.next();
            if lookahead
                .peek()
                .is_some_and(|(_, ch)| ch.is_alphanumeric() || *ch == '_')
            {
                let (space_offset, _) = chars.next().expect("the absorbed space was inspected");
                record_replacement(
                    &mut replacements,
                    record,
                    space_offset..space_offset + 1,
                    result.len()..result.len(),
                );
            }
        }
    }
    RepairedText {
        text: Cow::Owned(result),
        replacements,
    }
}

/// Repair ligature-glyph word breaks in extracted text.
///
/// When the PDF extractor decomposes ligature glyphs (fi, fl, ff, ffi, ffl) into individual
/// characters, the resulting character positions often have gaps that get interpreted
/// as word boundaries. This produces patterns like "eff iciently", "signif icant",
/// "f irst" where the space appears at the ligature position.
///
/// This function detects the pattern `f` followed by space followed by a lowercase
/// letter that would form a common ligature combination (fi, fl, ff), and removes the
/// space UNLESS one of the two fragments the space separates is independently attested
/// elsewhere in the document as a standalone word (see `pipeline::collect_word_witnesses`).
/// The same character pattern is also an ordinary word boundary whenever a word happens
/// to end in `f` and the next happens to begin with `i`, `l` or `f` ("relief for",
/// "bedrijf is"); at the string layer, the two cases are indistinguishable by geometry
/// or typography, only by whether the document itself uses the word elsewhere (#1591).
///
/// False positives: a document that uses a real word ending in `f` exactly once, right
/// before an `i`/`l`/`f`-initial word, with no other occurrence anywhere in the document,
/// still welds. False negatives: a genuine decomposed-ligature fragment that happens to
/// coincide with an attested word (e.g. a document containing both `office` and, coincidentally,
/// a decomposed `of ficial`) is left split. Both failure modes require a coincidence the
/// 33-word static list this replaces could never have caught either.
pub(super) fn repair_ligature_spaces<'a>(text: &'a str, word_witnesses: &WordWitnesses) -> Cow<'a, str> {
    if !text.contains("f ") {
        return Cow::Borrowed(text);
    }

    let mut result = String::with_capacity(text.len());
    let mut chars = text.char_indices().peekable();
    let mut word_start = 0;

    while let Some((index, ch)) = chars.next() {
        if ch == 'f' && chars.peek().is_some_and(|(_, next)| *next == ' ') {
            let space_end = index + ch.len_utf8() + ' '.len_utf8();
            let right_word = alphabetic_run_at(text, space_end);
            let is_ligature_continuation = right_word.chars().next().is_some_and(|c| matches!(c, 'i' | 'l' | 'f'));

            if is_ligature_continuation {
                let left_word = &text[word_start..index + ch.len_utf8()];
                let welds_a_witnessed_word =
                    is_witnessed_word(left_word, word_witnesses) || is_witnessed_word(right_word, word_witnesses);

                if !welds_a_witnessed_word {
                    result.push(ch);
                    chars.next();
                    continue;
                }
            }
        }

        result.push(ch);
        if !ch.is_alphabetic() {
            word_start = index + ch.len_utf8();
        }
    }

    if result == text {
        Cow::Borrowed(text)
    } else {
        Cow::Owned(result)
    }
}

/// Whether `word` is long enough to count as evidence, and is attested in the
/// document's word-witness set (case-insensitive).
fn is_witnessed_word(word: &str, word_witnesses: &WordWitnesses) -> bool {
    word.chars().count() >= MIN_LIGATURE_WITNESS_WORD_LEN && word_witnesses.contains(&word.to_ascii_lowercase())
}

/// The maximal run of alphabetic characters in `text` starting at byte offset `start`.
/// Empty if `start` is at or past the end of `text`, or is not itself alphabetic.
fn alphabetic_run_at(text: &str, start: usize) -> &str {
    let tail = &text[start.min(text.len())..];
    let end = tail.find(|c: char| !c.is_alphabetic()).unwrap_or(tail.len());
    &tail[..end]
}

/// Normalize Unicode characters commonly found in PDFs to their ASCII equivalents.
///
/// Matches the reference `sanitize_text()` normalizations for curly quotes, fraction
/// slash, and bullet characters. This improves TF1 by ensuring extracted text
/// matches ground truth tokenization.
pub(super) fn normalize_unicode_text(text: &str) -> Cow<'_, str> {
    normalize_unicode_text_mapped(text, false).text
}

pub(super) fn normalize_unicode_text_mapped(text: &str, record: bool) -> RepairedText<'_> {
    if !text.contains(['\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', '\u{2044}', '\u{2022}']) {
        return RepairedText::unchanged(text);
    }
    let mut result = String::with_capacity(text.len());
    let mut replacements = Vec::new();
    for (offset, ch) in text.char_indices() {
        let normalized = match ch {
            '\u{2018}' | '\u{2019}' => '\'',
            '\u{201C}' | '\u{201D}' => '"',
            '\u{2044}' => '/',
            '\u{2022}' => '\u{00B7}',
            other => other,
        };
        let output_start = result.len();
        result.push(normalized);
        if normalized != ch {
            record_replacement(
                &mut replacements,
                record,
                offset..offset + ch.len_utf8(),
                output_start..result.len(),
            );
        }
    }
    RepairedText {
        text: Cow::Owned(result),
        replacements,
    }
}

/// Final hyphen polish applied to assembled paragraph text.
///
/// Runs after segments are joined into a line/paragraph string (the spaced
/// pattern only exists post-join when the hyphen was its own PDF text run):
/// collapses spaced U+2010/U+2011 artifacts via [`collapse_spaced_hyphens`],
/// then maps the remaining Unicode hyphens to ASCII `-` to match ground-truth
/// tokenization.
pub(super) fn finalize_hyphens(text: &str) -> Cow<'_, str> {
    if text.contains(['\u{2010}', '\u{2011}']) {
        tracing::debug!(input = %text, "finalize_hyphens: unicode hyphen present");
    }
    let collapsed = collapse_spaced_hyphens(text);
    if !collapsed.contains(['\u{2010}', '\u{2011}']) {
        return collapsed;
    }
    Cow::Owned(collapsed.replace(['\u{2010}', '\u{2011}'], "-"))
}

/// Collapse spacing artifacts around Unicode hyphens between alphanumerics.
///
/// Hyphenated identifiers rendered as separate PDF text runs ("DARPA", "‐",
/// "BAA-15-58") get reassembled with kerning-gap spaces: `DARPA ‐ BAA ‐ 15`.
/// A spaced U+2010/U+2011 hyphen between alphanumerics is not a typographic
/// construct (spaced dashes use en/em dashes), so `X ‐ Y` collapses to `X‐Y`.
/// ASCII `-`, en dashes, and em dashes are left untouched — a spaced ASCII
/// hyphen can be a legitimate range or minus sign.
///
/// Must run before [`normalize_unicode_text`] maps U+2010/U+2011 to ASCII `-`,
/// while the artifact is still distinguishable.
pub(super) fn collapse_spaced_hyphens(text: &str) -> Cow<'_, str> {
    if !text.contains(['\u{2010}', '\u{2011}']) {
        return Cow::Borrowed(text);
    }

    let is_gap = |c: char| matches!(c, ' ' | '\u{00A0}' | '\n' | '\r' | '\t');
    let chars: Vec<char> = text.chars().collect();
    let mut result = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_alphanumeric() {
            let mut j = i + 1;
            while j < chars.len() && is_gap(chars[j]) {
                j += 1;
            }
            if j > i + 1 && j < chars.len() && matches!(chars[j], '\u{2010}' | '\u{2011}') {
                let mut k = j + 1;
                while k < chars.len() && is_gap(chars[k]) {
                    k += 1;
                }
                if k > j + 1 && k < chars.len() && chars[k].is_alphanumeric() {
                    result.push(chars[i]);
                    result.push('-');
                    i = k;
                    continue;
                }
            }
        }
        result.push(chars[i]);
        i += 1;
    }

    if result == text {
        Cow::Borrowed(text)
    } else {
        Cow::Owned(result)
    }
}

/// Clean up duplicate punctuation artifacts from PDF text extraction.
///
/// When segment-level re-extraction picks up characters from adjacent
/// cells (due to slightly overlapping bounding boxes), duplicate punctuation
/// patterns like `, ,` or `. .` appear. This collapses them to single
/// punctuation marks.
///
/// Patterns handled:
/// - `, ,` → `,`
/// - `. .` → `.`
/// - `; ;` → `;`
/// - `: :` → `:`
pub(super) fn clean_duplicate_punctuation(text: &str) -> Cow<'_, str> {
    if !has_duplicate_punctuation(text) {
        return Cow::Borrowed(text);
    }

    let mut current = collapse_duplicate_punctuation_once(text);
    while has_duplicate_punctuation(&current) {
        current = collapse_duplicate_punctuation_once(&current);
    }

    Cow::Owned(current)
}

/// Single pass of duplicate punctuation collapsing.
fn collapse_duplicate_punctuation_once(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let b = bytes[i];
        if is_dup_punct_byte(b) && i + 2 < len && bytes[i + 1] == b' ' && bytes[i + 2] == b {
            result.push(b as char);
            i += 3;
        } else {
            result.push(b as char);
            i += 1;
        }
    }

    result
}

/// Check if the text contains any duplicate punctuation pattern.
fn has_duplicate_punctuation(text: &str) -> bool {
    let bytes = text.as_bytes();
    for i in 0..bytes.len().saturating_sub(2) {
        let b = bytes[i];
        if is_dup_punct_byte(b) && bytes[i + 1] == b' ' && bytes[i + 2] == b {
            return true;
        }
    }
    false
}

/// Check if a byte is a punctuation character subject to duplicate cleanup.
fn is_dup_punct_byte(b: u8) -> bool {
    matches!(b, b',' | b'.' | b';' | b':')
}

/// Normalize text encoding: handle soft hyphens, PDF word-break markers,
/// and strip control characters.
///
/// - `\u{00AD}` (soft hyphen) at end of text → replaced with `-` so downstream
///   hyphen-rejoining logic can merge word fragments.
/// - `\u{00AD}` mid-text → removed (invisible break hint).
/// - `\x02` (STX) followed by space/newline → both removed, rejoining the word
///   fragments. Pdfium emits `\x02` at soft-hyphen positions where the hyphen
///   character was discarded by the PDF producer.
/// - Other C0 control characters (U+0000–U+001F except `\t`, `\n`, `\r`) → removed.
pub(super) fn normalize_text_encoding(text: &str) -> Cow<'_, str> {
    if !text.contains('\u{00AD}') && !text.bytes().any(|b| b < 0x20 && b != b'\t' && b != b'\n' && b != b'\r') {
        return Cow::Borrowed(text);
    }

    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '\u{00AD}' => {
                let at_end = chars.peek().is_none_or(|c| c.is_whitespace());
                if at_end {
                    result.push('-');
                }
            }
            '\x02' => {
                while chars.peek().is_some_and(|c| *c == ' ' || *c == '\n') {
                    chars.next();
                }
            }
            c if c.is_control() && c != '\n' && c != '\r' && c != '\t' => {}
            _ => result.push(ch),
        }
    }

    Cow::Owned(result)
}

/// Apply a text transformation to every segment in every paragraph.
///
/// The repair function returns `Cow<'_, str>`: if it returns `Cow::Borrowed`,
/// the segment text is unchanged and no allocation is performed. Only
/// `Cow::Owned` results trigger an update.
pub(super) fn apply_to_all_segments(paragraphs: &mut [PdfParagraph], repair_fn: impl Fn(&str) -> Cow<'_, str>) {
    for para in paragraphs {
        for line in &mut para.lines {
            for seg in &mut line.segments {
                if let Cow::Owned(s) = repair_fn(&seg.text) {
                    seg.text = s;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repair_contextual_ligatures_empty() {
        assert_eq!(repair_contextual_ligatures(""), "");
    }

    #[test]
    fn test_repair_contextual_ligatures_single_char() {
        assert_eq!(repair_contextual_ligatures("a"), "a");
    }

    #[test]
    fn test_repair_contextual_ligatures_no_corruption() {
        assert_eq!(repair_contextual_ligatures("hello world"), "hello world");
    }

    #[test]
    fn test_repair_contextual_ligatures_mid_word_fi() {
        assert_eq!(repair_contextual_ligatures("di!erent"), "different");
        assert_eq!(repair_contextual_ligatures("speci!c"), "specific");
    }

    #[test]
    fn test_repair_contextual_ligatures_mid_word_ff() {
        assert_eq!(repair_contextual_ligatures("di!erent effort"), "different effort");
        assert_eq!(repair_contextual_ligatures("e!ective"), "effective");
    }

    #[test]
    fn test_repair_contextual_ligatures_mid_word_ffi() {
        assert_eq!(repair_contextual_ligatures("e\u{22}cient"), "efficient");
    }

    #[test]
    fn test_repair_contextual_ligatures_word_start() {
        assert_eq!(repair_contextual_ligatures("#nancial"), "financial");
        assert_eq!(repair_contextual_ligatures("!nally"), "finally");
    }

    #[test]
    fn test_repair_contextual_ligatures_normal_punctuation() {
        assert_eq!(repair_contextual_ligatures("say \"hello\""), "say \"hello\"");
        assert_eq!(repair_contextual_ligatures("hello # world"), "hello # world");
    }

    #[test]
    fn test_repair_contextual_ligatures_multiple() {
        assert_eq!(
            repair_contextual_ligatures("ef!cient and #nancial"),
            "efficient and financial"
        );
    }

    #[test]
    fn test_repair_broken_word_spacing() {
        let broken = "M ust B e T ough";
        let repaired = repair_broken_word_spacing(broken);
        assert_eq!(repaired, "Must Be Tough");
    }

    #[test]
    fn test_repair_preserves_standalone_a_and_i() {
        let text = "I have a dog";
        let repaired = repair_broken_word_spacing(text);
        assert_eq!(repaired, "I have a dog");
    }

    #[test]
    fn test_repair_joins_multi_char_fragments() {
        let broken = "rom ance and m arriage";
        let repaired = repair_broken_word_spacing(broken);
        assert_eq!(repaired, "romance and marriage");
    }

    #[test]
    fn test_repair_joins_shall_be_active() {
        let broken = "s hall a b e active";
        let repaired = repair_broken_word_spacing(broken);
        assert_eq!(repaired, "shall a be active");
    }

    #[test]
    fn test_repair_joins_address_fragments() {
        let broken = "a dd ress";
        let repaired = repair_broken_word_spacing(broken);
        assert_eq!(repaired, "a ddress");
    }

    #[test]
    fn test_repair_joins_sender() {
        let broken = "sen d er hardware";
        let repaired = repair_broken_word_spacing(broken);
        assert_eq!(repaired, "sender hardware");
    }

    #[test]
    fn test_pipe_table_guard_standard() {
        let table = "| CTC_ARP | s hall be | active |";
        assert_eq!(repair_broken_word_spacing(table), table);
    }

    #[test]
    fn test_pipe_table_separator_guard() {
        let sep = "| --- | --- |";
        assert_eq!(repair_broken_word_spacing(sep), sep);
    }

    #[test]
    fn test_normalize_plain_text_unchanged() {
        assert_eq!(normalize_text_encoding("hello world"), "hello world");
    }

    #[test]
    fn test_normalize_trailing_soft_hyphen() {
        assert_eq!(normalize_text_encoding("soft\u{00AD}"), "soft-");
    }

    #[test]
    fn test_collapse_spaced_unicode_hyphen_chain() {
        assert_eq!(
            collapse_spaced_hyphens("DARPA \u{2010} BAA \u{2010} 15 \u{2010} 58 September"),
            "DARPA-BAA-15-58 September"
        );
        assert_eq!(collapse_spaced_hyphens("VA 22203 \u{2010} 2114"), "VA 22203-2114");
        assert_eq!(
            collapse_spaced_hyphens("DARPA\n\u{2010}\nBAA\n\u{2010}\n15\n\u{2010}\n58"),
            "DARPA-BAA-15-58"
        );
        assert_eq!(collapse_spaced_hyphens("multi\u{2010}\nline"), "multi\u{2010}\nline");
    }

    #[test]
    fn test_collapse_leaves_ascii_and_dashes_alone() {
        assert_eq!(collapse_spaced_hyphens("pages 10 - 20"), "pages 10 - 20");
        assert_eq!(collapse_spaced_hyphens("one \u{2013} two"), "one \u{2013} two");
        assert_eq!(collapse_spaced_hyphens("a \u{2014} b"), "a \u{2014} b");
    }

    #[test]
    fn test_finalize_hyphens_collapses_and_maps() {
        assert_eq!(
            finalize_hyphens("DARPA \u{2010} BAA \u{2010} 15 \u{2010} 58 September"),
            "DARPA-BAA-15-58 September"
        );
        assert_eq!(finalize_hyphens("DARPA\u{2010}BAA"), "DARPA-BAA");
        assert_eq!(finalize_hyphens("non\u{2011}breaking"), "non-breaking");
        assert_eq!(finalize_hyphens("pages 10 - 20"), "pages 10 - 20");
    }

    #[test]
    fn test_normalize_mid_word_soft_hyphen_removed() {
        assert_eq!(normalize_text_encoding("soft\u{00AD}ware"), "software");
    }

    #[test]
    fn test_normalize_soft_hyphen_before_space() {
        assert_eq!(normalize_text_encoding("soft\u{00AD} ware"), "soft- ware");
    }

    #[test]
    fn test_normalize_strips_control_chars() {
        assert_eq!(normalize_text_encoding("he\x01llo"), "hello");
    }

    #[test]
    fn test_normalize_stx_word_break_with_space() {
        assert_eq!(normalize_text_encoding("soft\x02 ware"), "software");
    }

    #[test]
    fn test_normalize_stx_word_break_with_newline() {
        assert_eq!(normalize_text_encoding("recog\x02\nnition"), "recognition");
    }

    #[test]
    fn test_normalize_stx_at_end() {
        assert_eq!(normalize_text_encoding("hello\x02"), "hello");
    }

    #[test]
    fn test_normalize_stx_no_trailing_space() {
        assert_eq!(normalize_text_encoding("soft\x02ware"), "software");
    }

    #[test]
    fn test_normalize_preserves_tabs_newlines() {
        assert_eq!(normalize_text_encoding("a\tb\nc\r"), "a\tb\nc\r");
    }

    #[test]
    fn test_expand_ligatures_no_ligatures() {
        let text = "hello world";
        let result = expand_ligatures_with_space_absorption(text);
        assert!(matches!(result, Cow::Borrowed(_)));
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_expand_ligatures_fi() {
        assert_eq!(expand_ligatures_with_space_absorption("\u{FB01}eld"), "field");
    }

    #[test]
    fn test_expand_ligatures_fl() {
        assert_eq!(expand_ligatures_with_space_absorption("\u{FB02}oor"), "floor");
    }

    #[test]
    fn test_expand_ligatures_ff() {
        assert_eq!(expand_ligatures_with_space_absorption("e\u{FB00}ect"), "effect");
    }

    #[test]
    fn test_expand_ligatures_ffi() {
        assert_eq!(expand_ligatures_with_space_absorption("e\u{FB03}cient"), "efficient");
    }

    #[test]
    fn test_expand_ligatures_ffl() {
        assert_eq!(expand_ligatures_with_space_absorption("ba\u{FB04}e"), "baffle");
    }

    #[test]
    fn test_expand_ligatures_st() {
        assert_eq!(expand_ligatures_with_space_absorption("\u{FB05}art"), "start");
        assert_eq!(expand_ligatures_with_space_absorption("\u{FB06}art"), "start");
    }

    #[test]
    fn test_expand_ligatures_space_absorption_fi() {
        assert_eq!(expand_ligatures_with_space_absorption("\u{FB01} eld"), "field");
    }

    #[test]
    fn test_expand_ligatures_space_absorption_fl() {
        assert_eq!(expand_ligatures_with_space_absorption("\u{FB02} oor"), "floor");
    }

    #[test]
    fn test_expand_ligatures_space_absorption_ff() {
        assert_eq!(expand_ligatures_with_space_absorption("e \u{FB00} ect"), "e ffect");
    }

    #[test]
    fn test_expand_ligatures_space_not_absorbed_before_punctuation() {
        assert_eq!(expand_ligatures_with_space_absorption("\u{FB01} ."), "fi .");
    }

    #[test]
    fn test_expand_ligatures_space_not_absorbed_before_space() {
        assert_eq!(expand_ligatures_with_space_absorption("\u{FB01}  word"), "fi  word");
    }

    #[test]
    fn test_expand_ligatures_at_end_of_string() {
        assert_eq!(expand_ligatures_with_space_absorption("pro\u{FB01}"), "profi");
    }

    #[test]
    fn test_expand_ligatures_space_at_end_not_absorbed() {
        assert_eq!(expand_ligatures_with_space_absorption("\u{FB01} "), "fi ");
    }

    #[test]
    fn test_expand_ligatures_multiple_in_sentence() {
        assert_eq!(
            expand_ligatures_with_space_absorption("the \u{FB01} rst \u{FB02} oor"),
            "the first floor"
        );
    }

    #[test]
    fn test_expand_ligatures_mixed_with_normal_text() {
        assert_eq!(
            expand_ligatures_with_space_absorption("a \u{FB01} eld of \u{FB02} owers"),
            "a field of flowers"
        );
    }

    #[test]
    fn test_expand_ligatures_no_space_no_absorption() {
        assert_eq!(expand_ligatures_with_space_absorption("\u{FB01}nally"), "finally");
    }

    #[test]
    fn test_repair_ligature_spaces_preserves_witnessed_word_boundary() {
        let text = "café kinetics of inhibitor adsorption if flow changes";
        let witnesses: WordWitnesses = ["of", "if"].into_iter().map(str::to_string).collect();
        let repaired = repair_ligature_spaces(text, &witnesses);

        assert_eq!(repaired, text);
        assert!(matches!(repaired, Cow::Borrowed(_)));
    }

    #[test]
    fn test_repair_ligature_spaces_repairs_intra_word_breaks() {
        let witnesses = WordWitnesses::default();
        assert_eq!(
            repair_ligature_spaces("f irst eff iciently signif icant f lange", &witnesses),
            "first efficiently significant flange"
        );
    }

    /// GH#1591: `is_common_short_word` used to be the only guard against welding a real
    /// word boundary, and it only ever checked the LEFT fragment against a hard-coded
    /// 33-word English list. Without any witness for either fragment, a word boundary
    /// that happens to end in `f` before an `i`/`l`/`f`-initial word is indistinguishable
    /// at the string layer from a decomposed ligature, and welds -- this is the fix's
    /// known false positive, documented rather than silently accepted.
    #[test]
    fn test_repair_ligature_spaces_welds_an_unwitnessed_word_boundary() {
        let witnesses = WordWitnesses::default();
        assert_eq!(
            repair_ligature_spaces("relief for the victims", &witnesses),
            "relieffor the victims"
        );
    }

    /// GH#1591: the fix. When either fragment is independently attested elsewhere in
    /// the document (here supplied directly, as `pipeline::collect_word_witnesses`
    /// would gather it from another page or line), the space is recognised as a real
    /// word boundary and preserved. English examples from the issue's reproducer.
    #[test]
    fn test_repair_ligature_spaces_preserves_english_word_boundaries_with_witnesses() {
        let witnesses: WordWitnesses = ["relief", "itself"].into_iter().map(str::to_string).collect();

        assert_eq!(
            repair_ligature_spaces("relief for the victims", &witnesses),
            "relief for the victims"
        );
        assert_eq!(
            repair_ligature_spaces("itself infringes the patent", &witnesses),
            "itself infringes the patent"
        );
    }

    /// GH#1591: the issue's Dutch reproducer example -- `bedrijf is` and `bedrijf komen`
    /// have identical gap geometry, glyph height and font; only whether "bedrijf" is
    /// independently attested elsewhere in the document tells them apart from a
    /// decomposed ligature.
    #[test]
    fn test_repair_ligature_spaces_preserves_dutch_word_boundary_with_witness() {
        let witnesses: WordWitnesses = ["bedrijf"].into_iter().map(str::to_string).collect();
        assert_eq!(
            repair_ligature_spaces("bedrijf is gesloten", &witnesses),
            "bedrijf is gesloten"
        );
    }

    /// GH#1591: a witness on the RIGHT fragment is equally sufficient -- the guard does
    /// not require evidence for the left fragment specifically.
    #[test]
    fn test_repair_ligature_spaces_preserves_word_boundary_via_right_witness() {
        let witnesses: WordWitnesses = ["instructions"].into_iter().map(str::to_string).collect();
        assert_eq!(repair_ligature_spaces("of instructions", &witnesses), "of instructions");
    }

    /// GH#1591: a witness shorter than `MIN_LIGATURE_WITNESS_WORD_LEN` cannot itself
    /// count as evidence -- a bare `f` fragment must never witness itself, mirroring
    /// the hyphen-witness collector's own single-letter guard.
    #[test]
    fn test_repair_ligature_spaces_single_letter_witness_is_ignored() {
        let witnesses: WordWitnesses = ["f"].into_iter().map(str::to_string).collect();
        assert_eq!(repair_ligature_spaces("f irst", &witnesses), "first");
    }

    #[test]
    fn test_clean_duplicate_comma() {
        assert_eq!(
            clean_duplicate_punctuation("simple, , self-contained"),
            "simple, self-contained"
        );
    }

    #[test]
    fn test_clean_duplicate_period() {
        assert_eq!(clean_duplicate_punctuation("end. . next"), "end. next");
    }

    #[test]
    fn test_clean_duplicate_semicolon() {
        assert_eq!(clean_duplicate_punctuation("a; ; b"), "a; b");
    }

    #[test]
    fn test_clean_duplicate_colon() {
        assert_eq!(clean_duplicate_punctuation("key: : value"), "key: value");
    }

    #[test]
    fn test_clean_duplicate_punctuation_no_change() {
        let text = "Hello, world. This is normal; right: yes";
        assert!(matches!(clean_duplicate_punctuation(text), Cow::Borrowed(_)));
    }

    #[test]
    fn test_clean_duplicate_punctuation_multiple() {
        assert_eq!(clean_duplicate_punctuation("a, , b, , c"), "a, b, c");
    }

    #[test]
    fn test_clean_duplicate_punctuation_triple() {
        assert_eq!(
            clean_duplicate_punctuation("[12, 13, 9]. Docling is designed as a simple, , , self-contained"),
            "[12, 13, 9]. Docling is designed as a simple, self-contained"
        );
    }
}

#[cfg(test)]
mod overreach_regression_tests {
    //! Regression tests: ligature repair must not fire on common punctuation
    //! patterns (sentence-final '!', footnote '*', word-final 'M').
    use super::*;

    #[test]
    fn sentence_final_exclamation_is_preserved() {
        assert_eq!(repair_contextual_ligatures("Encore du contenu!"), "Encore du contenu!");
        assert_eq!(repair_contextual_ligatures("Thank you!"), "Thank you!");
    }

    #[test]
    fn footnote_star_is_preserved() {
        assert_eq!(repair_contextual_ligatures("value*"), "value*");
        assert_eq!(
            repair_contextual_ligatures("significant* results"),
            "significant* results"
        );
    }

    #[test]
    fn word_final_uppercase_m_is_preserved() {
        assert_eq!(repair_contextual_ligatures("50 µM"), "50 µM");
        assert_eq!(repair_contextual_ligatures("about 3 µM."), "about 3 µM.");
    }

    #[test]
    fn mid_word_repairs_still_fire() {
        assert_eq!(repair_contextual_ligatures("di!erent"), "different");
        assert_eq!(repair_contextual_ligatures("speci!c"), "specific");
    }

    // Issue #1556: ':' -> "ti" and 'M' -> "tti" were removed outright (not gated) because
    // they rewrite ordinary text. Healthy documents must pass through unchanged. ~keep
    #[test]
    fn colon_and_uppercase_m_never_rewritten() {
        assert_eq!(repair_contextual_ligatures("aMb"), "aMb");
        assert_eq!(repair_contextual_ligatures("ges:one"), "ges:one");
        assert_eq!(repair_contextual_ligatures("progeM"), "progeM");
        assert_eq!(repair_contextual_ligatures("ratio:x"), "ratio:x");
        assert_eq!(repair_contextual_ligatures("TestMethodOrder"), "TestMethodOrder");
        assert_eq!(repair_contextual_ligatures("currentTimeMillis"), "currentTimeMillis");
        assert_eq!(
            repair_contextual_ligatures("TestMethodOrder(OrderAnnotation.class)"),
            "TestMethodOrder(OrderAnnotation.class)"
        );
        assert_eq!(
            repair_contextual_ligatures("engine:junit-jupiter"),
            "engine:junit-jupiter"
        );
        assert_eq!(repair_contextual_ligatures("class:example"), "class:example");
        assert_eq!(
            repair_contextual_ligatures("method:skippedTest()"),
            "method:skippedTest()"
        );
    }
}

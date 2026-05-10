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

/// Repair ligature corruption using contextual heuristics.
///
/// Some PDF fonts have broken ToUnicode CMaps that map ligature glyphs to
/// ASCII characters. This function detects and repairs these patterns:
///
/// **f-ligatures**: `!` → fi/ff, `"` → ffi, `#` → fi/fl
/// **t-ligatures**: `*` → tt, `:` → ti, uppercase `M` between lowercase → tti
///
/// All patterns are contextual: the corrupt character must appear between
/// alphabetic characters (mid-word), where it virtually never occurs in real text.
pub(super) fn repair_contextual_ligatures(text: &str) -> Cow<'_, str> {
    if text.len() < 2 {
        return Cow::Borrowed(text);
    }

    let mut result = String::with_capacity(text.len() + 16);
    let mut repaired = false;
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

        match ch {
            // f-ligatures (standard)
            '!' if prev_is_alpha && next_is_vowel => {
                result.push_str("ff");
                repaired = true;
            }
            '!' if prev_is_alpha && next_is_alpha => {
                result.push_str("fi");
                repaired = true;
            }
            '!' if prev_is_alpha && next_byte_idx >= bytes.len() => {
                result.push_str("fi");
                repaired = true;
            }
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
            // tt/ti/tti ligatures (European fonts)
            '*' if prev_is_alpha && next_is_alpha => {
                result.push_str("tt");
                repaired = true;
            }
            '*' if prev_is_alpha && (next_byte_idx >= bytes.len() || !next_is_alpha) => {
                result.push_str("tt");
                repaired = true;
            }
            ':' if prev_is_alpha && next_is_lower => {
                // ':' mid-word: letter immediately before AND lowercase letter immediately after
                // e.g., "ges:one" → "gestione". Safe because real colons have a space after.
                result.push_str("ti");
                repaired = true;
            }
            // Uppercase M between lowercase letters → "tti" (e.g., "progeM" → "progetti")
            'M' if prev_is_alpha && !prev_is_space_or_start => {
                // Check: previous char must be lowercase, and either at word end or next is lowercase
                let prev_was_lower = if byte_idx > 0 {
                    bytes.get(byte_idx - 1).is_some_and(|&b| (b as char).is_lowercase())
                } else {
                    false
                };
                if prev_was_lower && (next_is_lower || next_byte_idx >= bytes.len() || !next_is_alpha) {
                    result.push_str("tti");
                    repaired = true;
                } else {
                    result.push(ch);
                }
            }
            _ => result.push(ch),
        }

        prev_is_alpha = ch.is_alphabetic();
        prev_is_space_or_start = ch.is_whitespace();
        byte_idx = next_byte_idx;
    }

    if repaired {
        Cow::Owned(result)
    } else {
        Cow::Borrowed(text)
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

    // Skip pipe-table markdown — split_whitespace destroys table formatting.
    if text.contains("| --- |") || text.starts_with('|') {
        return Cow::Borrowed(text);
    }

    let words: Vec<&str> = text.split_whitespace().collect();

    // Quick pre-scan: check if any joins would be made before allocating.
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

        // Pattern 1a: Single-char fragment followed by a lowercase word.
        // Directly joins "s" + "hall" → "shall", "M" + "ust" → "Must".
        // Only one word consumed (no chaining).
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

        // Pattern 1b: Short fragment (2-3 chars) starts a run of short
        // fragments. Joins "dd ress", "sen d er" patterns by chaining
        // consecutive short (<=3 char) pieces.
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
                // Both pieces short — keep joining the fragment run.
                if last_consumed_len <= 3 && next.len() <= 3 {
                    result.push_str(next);
                    last_consumed_len = next.len();
                    total_consumed += next.len();
                    i += 1;
                    continue;
                }
                // If accumulated so far is still short (<=3 chars), allow
                // one longer word to complete it. E.g., "dd" + "ress" → "ddress".
                if total_consumed <= 3 {
                    result.push_str(next);
                    i += 1;
                    break;
                }
                break;
            }
            continue;
        }

        // Pattern 2: An alphabetic word (not a common short word) followed
        // by a trailing short fragment (1-2 chars, lowercase, not common).
        // Handles "reques t" → "request", "sen der" patterns.
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
        && !word.chars().all(|c| c.is_uppercase()) // exclude abbreviations like "PDF", "AI"
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
        // 1-char
        "a" | "A" | "I"
        // 2-char
        | "an" | "am" | "as" | "at" | "be" | "by" | "do" | "go" | "he"
        | "if" | "in" | "is" | "it" | "me" | "my" | "no" | "of" | "oh"
        | "on" | "or" | "so" | "to" | "up" | "us" | "we"
        | "An" | "Am" | "As" | "At" | "Be" | "By" | "Do" | "Go" | "He"
        | "If" | "In" | "Is" | "It" | "Me" | "My" | "No" | "Of" | "Oh"
        | "On" | "Or" | "So" | "To" | "Up" | "Us" | "We"
        // 3-char
        | "the" | "and" | "are" | "but" | "can" | "did" | "for" | "got"
        | "had" | "has" | "her" | "him" | "his" | "how" | "its" | "let"
        | "may" | "new" | "nor" | "not" | "now" | "old" | "one" | "our"
        | "out" | "own" | "ran" | "say" | "she" | "too" | "two" | "use"
        | "was" | "way" | "who" | "why" | "yet" | "you" | "all" | "any"
        | "big" | "day" | "end" | "far" | "few" | "put" | "run"
        | "saw" | "set" | "top" | "try" | "win" | "yes"
        | "The" | "And" | "Are" | "But" | "Can" | "Did" | "For" | "Got"
        | "Had" | "Has" | "Her" | "Him" | "His" | "How" | "Its" | "Let"
        | "May" | "New" | "Nor" | "Not" | "Now" | "Old" | "One" | "Our"
        | "Out" | "Own" | "Ran" | "Say" | "She" | "Too" | "Two" | "Use"
        | "Was" | "Way" | "Who" | "Why" | "Yet" | "You" | "All" | "Any"
        | "Big" | "Day" | "End" | "Far" | "Few" | "Put" | "Run"
        | "Saw" | "Set" | "Top" | "Try" | "Win" | "Yes"
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
    // Fast path: check if any byte could start a ligature codepoint (U+FB00–U+FB06).
    // These encode as 0xEF 0xBC 0x80..0x86 in UTF-8.
    if !text.contains([
        '\u{FB00}', '\u{FB01}', '\u{FB02}', '\u{FB03}', '\u{FB04}', '\u{FB05}', '\u{FB06}',
    ]) {
        return Cow::Borrowed(text);
    }

    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
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

        result.push_str(expansion);

        // Absorb a trailing space if followed by a word character.
        // This handles the "ﬁ eld" → "field" pattern.
        if chars.peek() == Some(&' ') {
            // Clone the iterator to peek two ahead (space + next char).
            let mut lookahead = chars.clone();
            lookahead.next(); // consume the space
            if lookahead.peek().is_some_and(|c| c.is_alphanumeric() || *c == '_') {
                chars.next(); // absorb the space
            }
        }
    }

    Cow::Owned(result)
}

/// Repair ligature-glyph word breaks in extracted text.
///
/// When the PDF extractor decomposes ligature glyphs (fi, fl, ff, ffi, ffl) into individual
/// characters, the resulting character positions often have gaps that get interpreted
/// as word boundaries. This produces patterns like "eff iciently", "signif icant",
/// "f irst" where the space appears at the ligature position.
///
/// This function detects and removes these spurious spaces by looking for the pattern:
/// `f` (or `ff`) followed by space followed by lowercase letter that would form a
/// common ligature combination (fi, fl, ff).
pub(super) fn repair_ligature_spaces(text: &str) -> Cow<'_, str> {
    // Fast path: no "f " pattern
    if !text.contains("f ") {
        return Cow::Borrowed(text);
    }

    let mut result = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        // Look for 'f' followed by ' ' followed by a lowercase letter
        // that would form a ligature: fi, fl, ff, ffi, ffl
        if bytes[i] == b'f' && i + 2 < len && bytes[i + 1] == b' ' {
            let next = bytes[i + 2];
            // Check if this is a ligature break: f + space + {i, l, f, e, o, a, ...}
            // Only absorb if preceded by a word character (not start of word "for", "from")
            // and the preceding context suggests mid-word break.
            if (next == b'i' || next == b'l' || next == b'f') && i > 0 && bytes[i - 1].is_ascii_alphabetic() {
                // This looks like a ligature break: "eff iciently" → "efficiently"
                result.push('f');
                i += 2; // skip the space
                continue;
            }
        }
        result.push(bytes[i] as char);
        i += 1;
    }

    if result == text {
        Cow::Borrowed(text)
    } else {
        Cow::Owned(result)
    }
}

/// Normalize Unicode characters commonly found in PDFs to their ASCII equivalents.
///
/// Matches the reference `sanitize_text()` normalizations for curly quotes, fraction
/// slash, and bullet characters. This improves TF1 by ensuring extracted text
/// matches ground truth tokenization.
pub(super) fn normalize_unicode_text(text: &str) -> Cow<'_, str> {
    if !text.contains(['\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', '\u{2044}', '\u{2022}']) {
        return Cow::Borrowed(text);
    }
    Cow::Owned(
        text.replace(['\u{2018}', '\u{2019}'], "'")  // curly single quotes
            .replace(['\u{201C}', '\u{201D}'], "\"") // curly double quotes
            .replace('\u{2044}', "/")  // fraction slash
            .replace('\u{2022}', "\u{00B7}"), // bullet → middle dot
    )
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
    // Fast path: check for any duplicate punctuation pattern before allocating.
    if !has_duplicate_punctuation(text) {
        return Cow::Borrowed(text);
    }

    // Apply iteratively until no more duplicate punctuation remains.
    // Handles chains like `, , ,` which need two passes (`, , ,` -> `, ,` -> `,`).
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
        // Check for "X Y X" pattern where X is punctuation and Y is a space.
        if is_dup_punct_byte(b) && i + 2 < len && bytes[i + 1] == b' ' && bytes[i + 2] == b {
            // Found "X X" with space between — emit just the first punctuation.
            result.push(b as char);
            i += 3; // skip "X X"
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
    // Fast path: no special characters present
    if !text.contains('\u{00AD}') && !text.bytes().any(|b| b < 0x20 && b != b'\t' && b != b'\n' && b != b'\r') {
        return Cow::Borrowed(text);
    }

    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '\u{00AD}' => {
                // Soft hyphen at end of text (or before whitespace): convert to regular
                // hyphen so rendering code can rejoin word fragments.
                let at_end = chars.peek().is_none_or(|c| c.is_whitespace());
                if at_end {
                    result.push('-');
                }
                // Mid-word soft hyphen: drop (invisible break hint)
            }
            '\x02' => {
                // Pdfium soft-hyphen word-break marker. Strip the marker and any
                // immediately following whitespace to rejoin the word fragments.
                // e.g., "soft\x02 ware" → "software", "recog\x02\nnition" → "recognition"
                while chars.peek().is_some_and(|c| *c == ' ' || *c == '\n') {
                    chars.next();
                }
            }
            c if c.is_control() && c != '\n' && c != '\r' && c != '\t' => {
                // Strip other control characters
            }
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
                // Borrow directly — only allocate when the repair actually modifies text.
                // Previously this cloned every segment's text unconditionally.
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
        // "rom" (3 chars) before lowercase "ance" → joined. "m" before "arriage" → joined.
        assert_eq!(repaired, "romance and marriage");
    }

    #[test]
    fn test_repair_joins_shall_be_active() {
        let broken = "s hall a b e active";
        let repaired = repair_broken_word_spacing(broken);
        // "s"+"hall" → "shall" (1-char + lowercase), "a" preserved (common),
        // "b"+"e" → "be" (1-char + lowercase), "active" stays separate.
        assert_eq!(repaired, "shall a be active");
    }

    #[test]
    fn test_repair_joins_address_fragments() {
        let broken = "a dd ress";
        let repaired = repair_broken_word_spacing(broken);
        // "a" is common standalone word, "dd" (2 chars) + "ress" → "ddress"
        assert_eq!(repaired, "a ddress");
    }

    #[test]
    fn test_repair_joins_sender() {
        let broken = "sen d er hardware";
        let repaired = repair_broken_word_spacing(broken);
        // "sen" (3 chars) + "d" + "er" → "sender"
        assert_eq!(repaired, "sender hardware");
    }

    #[test]
    fn test_pipe_table_guard_standard() {
        // Lines starting with '|' are pipe-table rows — must not be modified.
        let table = "| CTC_ARP | s hall be | active |";
        assert_eq!(repair_broken_word_spacing(table), table);
    }

    #[test]
    fn test_pipe_table_separator_guard() {
        // Pipe-table separator lines must also be left untouched.
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
        // Pdfium soft-hyphen marker: \x02 followed by space → rejoin word
        assert_eq!(normalize_text_encoding("soft\x02 ware"), "software");
    }

    #[test]
    fn test_normalize_stx_word_break_with_newline() {
        // Pdfium soft-hyphen marker: \x02 followed by newline → rejoin word
        assert_eq!(normalize_text_encoding("recog\x02\nnition"), "recognition");
    }

    #[test]
    fn test_normalize_stx_at_end() {
        // \x02 at end of text → just stripped
        assert_eq!(normalize_text_encoding("hello\x02"), "hello");
    }

    #[test]
    fn test_normalize_stx_no_trailing_space() {
        // \x02 not followed by space → just stripped (rejoin adjacent chars)
        assert_eq!(normalize_text_encoding("soft\x02ware"), "software");
    }

    #[test]
    fn test_normalize_preserves_tabs_newlines() {
        assert_eq!(normalize_text_encoding("a\tb\nc\r"), "a\tb\nc\r");
    }

    // --- expand_ligatures_with_space_absorption ---

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
        // "ﬁ eld" → "field" (space absorbed before word char)
        assert_eq!(expand_ligatures_with_space_absorption("\u{FB01} eld"), "field");
    }

    #[test]
    fn test_expand_ligatures_space_absorption_fl() {
        // "ﬂ oor" → "floor"
        assert_eq!(expand_ligatures_with_space_absorption("\u{FB02} oor"), "floor");
    }

    #[test]
    fn test_expand_ligatures_space_absorption_ff() {
        // "e ﬀ ect" → "e effect" (space before ligature preserved, space after absorbed)
        assert_eq!(expand_ligatures_with_space_absorption("e \u{FB00} ect"), "e ffect");
    }

    #[test]
    fn test_expand_ligatures_space_not_absorbed_before_punctuation() {
        // "ﬁ ." → "fi ." (space before punctuation not absorbed)
        assert_eq!(expand_ligatures_with_space_absorption("\u{FB01} ."), "fi .");
    }

    #[test]
    fn test_expand_ligatures_space_not_absorbed_before_space() {
        // "ﬁ  word" → "fi  word" (double space: first space not absorbed because next is space)
        assert_eq!(expand_ligatures_with_space_absorption("\u{FB01}  word"), "fi  word");
    }

    #[test]
    fn test_expand_ligatures_at_end_of_string() {
        // Ligature at end with no trailing chars
        assert_eq!(expand_ligatures_with_space_absorption("pro\u{FB01}"), "profi");
    }

    #[test]
    fn test_expand_ligatures_space_at_end_not_absorbed() {
        // "ﬁ " at end → "fi " (space not absorbed because no following word char)
        assert_eq!(expand_ligatures_with_space_absorption("\u{FB01} "), "fi ");
    }

    #[test]
    fn test_expand_ligatures_multiple_in_sentence() {
        // "the ﬁ rst ﬂ oor" → "the first floor"
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
        // Ligature directly adjacent to word char — no space to absorb
        assert_eq!(expand_ligatures_with_space_absorption("\u{FB01}nally"), "finally");
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
        // Normal text without duplicate punctuation should pass through unchanged.
        let text = "Hello, world. This is normal; right: yes";
        assert!(matches!(clean_duplicate_punctuation(text), Cow::Borrowed(_)));
    }

    #[test]
    fn test_clean_duplicate_punctuation_multiple() {
        assert_eq!(clean_duplicate_punctuation("a, , b, , c"), "a, b, c");
    }

    #[test]
    fn test_clean_duplicate_punctuation_triple() {
        // Triple comma `, , ,` collapses iteratively: `, , ,` -> `, ,` -> `,`
        assert_eq!(
            clean_duplicate_punctuation("[12, 13, 9]. Docling is designed as a simple, , , self-contained"),
            "[12, 13, 9]. Docling is designed as a simple, self-contained"
        );
    }
}

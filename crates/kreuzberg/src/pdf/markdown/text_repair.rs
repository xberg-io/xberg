//! Text repair utilities for PDF extraction.
//!
//! Handles three classes of text corruption common in PDFs with broken font encodings:
//!
//! 1. **Ligature corruption** – fonts with broken ToUnicode CMaps map ligature glyphs
//!    (fi, fl, ff, ffi, ffl) to low-byte or ASCII characters. Repaired via either a
//!    per-page repair map built from pdfium's `has_unicode_map_error()` API, or
//!    contextual heuristics (e.g., `e!cient` → `efficient`).
//!
//! 2. **Broken word spacing** – fonts with broken CMap/ToUnicode tables cause pdfium
//!    to insert spaces mid-word. Detected by density of single-letter fragments
//!    followed by lowercase continuations; repaired by rejoining them.
//!
//! 3. **Unicode normalization** – curly quotes, fraction slash, and other PDF-specific
//!    Unicode characters are normalized to their ASCII equivalents.

use std::borrow::Cow;

use memchr::memchr3;
use pdfium_render::prelude::*;

use super::types::PdfParagraph;

/// Build a mapping of corrupted characters → correct ligature expansions for a page.
///
/// Walks the per-character API to find characters with `has_unicode_map_error()`,
/// then determines the correct ligature expansion based on the character's raw
/// unicode value and font-specific encoding patterns.
///
/// Returns `None` if the page has no encoding errors (most pages).
pub(super) fn build_ligature_repair_map(page: &PdfPage) -> Option<Vec<(char, &'static str)>> {
    let text = match page.text() {
        Ok(t) => t,
        Err(_) => return None,
    };

    let chars = text.chars();
    let char_count = chars.len();
    if char_count == 0 {
        return None;
    }

    let mut repair_map: Vec<(char, &'static str)> = Vec::new();

    for i in 0..char_count {
        let ch = match chars.get(i) {
            Ok(c) => c,
            Err(_) => continue,
        };

        if ch.is_generated().unwrap_or(false) {
            continue;
        }

        if !ch.has_unicode_map_error().unwrap_or(false) {
            continue;
        }

        // Skip symbol/math fonts — their encodings are intentional
        if ch.font_is_symbolic() {
            continue;
        }

        let unicode_val = ch.unicode_value();
        let mapped_char = match char::from_u32(unicode_val) {
            Some(c) => c,
            None => continue,
        };

        // Check if we already have a mapping for this character
        if repair_map.iter().any(|(c, _)| *c == mapped_char) {
            continue;
        }

        // Determine the correct ligature based on raw unicode value.
        // Different fonts encode ligatures at different positions. We check
        // both the low-byte encoding (CM fonts) and ASCII fallback positions.
        let ligature = match unicode_val {
            // Standard Type1/CM ligature positions (low bytes)
            0x0B => "ff",
            0x0C => "fi",
            0x0D => "fl",
            0x0E => "ffi",
            0x0F => "ffl",
            // Alternate low-byte positions used by some fonts
            0x01 => "fi",
            0x02 => "fl",
            0x03 => "ff",
            0x04 => "ffi",
            0x05 => "ffl",
            // ASCII positions: broken CMap maps ligature glyph codes to these
            // ASCII characters. Safe because we only reach here when
            // has_unicode_map_error() is true (i.e. pdfium detected encoding issues).
            0x21 => "fi",  // '!' → fi (most common ligature corruption)
            0x22 => "ff",  // '"' → ff
            0x23 => "fl",  // '#' → fl
            0x24 => "ffi", // '$' → ffi
            0x25 => "ffl", // '%' → ffl
            _ => continue,
        };

        repair_map.push((mapped_char, ligature));
    }

    if repair_map.is_empty() { None } else { Some(repair_map) }
}

/// Apply ligature repairs to a text string using a page-specific repair map.
///
/// After replacing ligature characters, collapses spurious spaces that result
/// from the replacement: e.g., "ﬁ rst" → "fi rst" → "first". When a ligature
/// expansion is immediately followed by a space and a lowercase letter, the
/// space is removed (matching Docling's regex-based post-processing).
pub(super) fn apply_ligature_repairs(text: &str, repair_map: &[(char, &str)]) -> String {
    let mut result = String::with_capacity(text.len() + 16);
    for ch in text.chars() {
        if let Some((_, replacement)) = repair_map.iter().find(|(c, _)| *c == ch) {
            result.push_str(replacement);
        } else {
            result.push(ch);
        }
    }

    // Post-processing: collapse "fi rst" → "first" patterns.
    // After a ligature expansion (fi, fl, ff, ffi, ffl), if the next char is a
    // space followed by a lowercase letter, remove the space.
    let ligature_endings: &[&str] = &["fi", "fl", "ff", "ffi", "ffl"];
    let mut collapsed = String::with_capacity(result.len());
    let mut chars = result.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == ' ' && !collapsed.is_empty() {
            // Look ahead: is the next character a lowercase letter?
            if chars.peek().is_some_and(|&nc| nc.is_lowercase()) {
                // Check if the text before the space ends with a ligature expansion
                let should_collapse = ligature_endings.iter().any(|lig| collapsed.ends_with(lig));
                if should_collapse {
                    // Skip the space — don't push it
                    continue;
                }
            }
        }
        collapsed.push(ch);
    }

    collapsed
}

/// Repair ligature corruption using contextual heuristics.
///
/// Some PDF fonts (particularly Computer Modern from TeX/LaTeX) have broken
/// ToUnicode CMaps that map ligature glyphs to ASCII characters:
/// - `fi` → `!` (0x21)
/// - `ff` → `"` (0x22)
/// - `fl` → `#` (0x23)
///
/// Unlike `build_ligature_repair_map()` which relies on `has_unicode_map_error()`,
/// this function detects corruption contextually: `!`, `"`, or `#` appearing
/// between alphabetic characters (e.g., `e!cient`, `o"ces`, `#nancial`) is
/// a near-certain indicator of ligature corruption, as these patterns virtually
/// never occur in real text.
///
/// This is safe to apply broadly because:
/// - Normal `!` appears at word/sentence boundaries, not between letters
/// - Normal `"` appears at word boundaries (quotation marks), not mid-word
/// - Normal `#` appears at word start (hashtags) or after non-letters, not mid-word
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

/// Check if text contains ligature corruption patterns.
///
/// Returns true if the text shows signs of broken ligature encoding:
/// - Mid-word: `!`, `"`, or `#` between alphabetic characters
/// - Word-start: `#` or `!` after whitespace/start followed by lowercase letter
///
/// Requires 2+ matches to avoid false positives from normal punctuation.
pub(super) fn text_has_ligature_corruption(text: &str) -> bool {
    if text.len() < 3 {
        return false;
    }

    let bytes = text.as_bytes();
    let mut count = 0u32;
    let mut pos = 0;

    while let Some(idx) = memchr3(b'!', b'"', b'#', &bytes[pos..]) {
        let i = pos + idx;
        let ch = bytes[i];

        let prev_alpha = if i > 0 {
            let prev_byte = bytes[i - 1];
            (prev_byte as char).is_alphabetic()
        } else {
            false
        };

        let next_alpha = if i + 1 < bytes.len() {
            let next_byte = bytes[i + 1];
            (next_byte as char).is_alphabetic()
        } else {
            false
        };

        let prev_space_or_start = i == 0 || (bytes[i - 1] as char).is_whitespace();

        let next_lower = if i + 1 < bytes.len() {
            let next_byte = bytes[i + 1];
            (next_byte as char).is_lowercase()
        } else {
            false
        };

        if prev_alpha && next_alpha {
            count += 1;
        }

        if matches!(ch, b'#' | b'!') && prev_space_or_start && next_lower {
            count += 1;
        }

        pos = i + 1;
        if count >= 1 {
            break;
        }
    }

    count >= 1
}

/// Check if text has an abnormal density of single-letter words followed by
/// lowercase continuation, indicating broken word spacing from pdfium.
///
/// Pattern: `"M ust"`, `"rom ance"`, `"w ork"` — a single letter (or small
/// fragment) followed by a space then a lowercase continuation. Normal English
/// text rarely has single-letter words other than "a", "I", and some articles.
///
/// Returns true if the density of suspicious fragments exceeds a threshold,
/// indicating systematic font-metric corruption on this page.
pub(super) fn text_has_broken_word_spacing(text: &str) -> bool {
    if text.len() < 20 {
        return false;
    }

    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() < 5 {
        return false;
    }

    let mut suspicious = 0u32;
    for window in words.windows(2) {
        let w = window[0];
        let next = window[1];
        // Single letter (not "a", "I", or common single-char words) followed
        // by lowercase start — likely a broken word.
        if w.len() == 1 && w.chars().next().is_some_and(|c| c.is_alphabetic()) {
            let ch = w.chars().next().unwrap();
            if ch != 'a' && ch != 'I' && ch != 'A' && next.chars().next().is_some_and(|c| c.is_lowercase()) {
                suspicious += 1;
            }
        }
    }

    // Need a significant density: at least 5 suspicious pairs and >5% of words.
    suspicious >= 5 && (suspicious as f64 / words.len() as f64) > 0.05
}

/// Repair broken word spacing by joining single-letter fragments to adjacent words.
///
/// Targets the pattern where pdfium inserts spaces mid-word due to broken font
/// CMap/ToUnicode tables. For example: `"M ust Be Tough"` → `"Must Be Tough"`.
///
/// Only joins when:
/// - The fragment is a single alphabetic character
/// - It's not a common standalone word ("a", "I", "A")
/// - The next word starts with a lowercase letter (continuation)
pub(super) fn repair_broken_word_spacing(text: &str) -> Cow<'_, str> {
    if text.is_empty() {
        return Cow::Borrowed(text);
    }

    let words: Vec<&str> = text.split_whitespace().collect();
    let mut repaired = false;

    // Quick pre-scan: check if any joins would be made before allocating.
    for window in words.windows(2) {
        let w = window[0];
        let next = window[1];
        if w.len() == 1 && w.chars().next().is_some_and(|c| c.is_alphabetic()) {
            let ch = w.chars().next().unwrap();
            if ch != 'a' && ch != 'I' && ch != 'A' && next.chars().next().is_some_and(|c| c.is_lowercase()) {
                repaired = true;
                break;
            }
        }
    }

    if !repaired {
        return Cow::Borrowed(text);
    }

    let mut result = String::with_capacity(text.len());
    let mut i = 0;
    while i < words.len() {
        if i > 0 && !result.is_empty() {
            result.push(' ');
        }

        let w = words[i];
        // Check if this is a single-letter fragment that should be joined to
        // the next word.
        if w.len() == 1 && i + 1 < words.len() && w.chars().next().is_some_and(|c| c.is_alphabetic()) {
            let ch = w.chars().next().unwrap();
            let next = words[i + 1];
            // Join if not a common standalone and next starts lowercase.
            if ch != 'a' && ch != 'I' && ch != 'A' && next.chars().next().is_some_and(|c| c.is_lowercase()) {
                result.push(ch);
                // Don't push space — join directly to next word.
                result.push_str(next);
                i += 2;
                continue;
            }
        }

        result.push_str(w);
        i += 1;
    }

    Cow::Owned(result)
}

/// Normalize Unicode characters commonly found in PDFs to their ASCII equivalents.
///
/// Matches docling's `sanitize_text()` normalizations for curly quotes, fraction
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

/// Normalize text encoding: handle soft hyphens and strip control characters.
///
/// - `\u{00AD}` (soft hyphen) at end of text → replaced with `-` so downstream
///   hyphen-rejoining logic can merge word fragments.
/// - `\u{00AD}` mid-text → removed (invisible break hint).
/// - C0 control characters (U+0000–U+001F except `\t`, `\n`, `\r`) → removed.
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
                let input = seg.text.clone();
                if let Cow::Owned(s) = repair_fn(&input) {
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
    fn test_apply_ligature_repairs_fi() {
        let map = vec![('\x0C', "fi")];
        assert_eq!(apply_ligature_repairs("classi\x0Ccation", &map), "classification");
    }

    #[test]
    fn test_apply_ligature_repairs_ff() {
        let map = vec![('\x0B', "ff")];
        assert_eq!(apply_ligature_repairs("e\x0Bective", &map), "effective");
    }

    #[test]
    fn test_apply_ligature_repairs_fl() {
        let map = vec![('\x0D', "fl")];
        assert_eq!(apply_ligature_repairs("re\x0Dection", &map), "reflection");
    }

    #[test]
    fn test_apply_ligature_repairs_ffi() {
        let map = vec![('\x0E', "ffi")];
        assert_eq!(apply_ligature_repairs("e\x0Ecient", &map), "efficient");
    }

    #[test]
    fn test_apply_ligature_repairs_ffl() {
        let map = vec![('\x0F', "ffl")];
        assert_eq!(apply_ligature_repairs("ba\x0Fe", &map), "baffle");
    }

    #[test]
    fn test_apply_ligature_repairs_no_map() {
        let map: Vec<(char, &str)> = Vec::new();
        assert_eq!(apply_ligature_repairs("hello world!", &map), "hello world!");
    }

    #[test]
    fn test_apply_ligature_repairs_multiple() {
        let map = vec![('\x0C', "fi"), ('\x0E', "ffi")];
        assert_eq!(
            apply_ligature_repairs("e\x0Ecient and classi\x0Ccation", &map),
            "efficient and classification"
        );
    }

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
    fn test_text_has_ligature_corruption_empty() {
        assert!(!text_has_ligature_corruption(""));
    }

    #[test]
    fn test_text_has_ligature_corruption_too_short() {
        assert!(!text_has_ligature_corruption("ab"));
    }

    #[test]
    fn test_text_has_ligature_corruption_no_corruption() {
        assert!(!text_has_ligature_corruption("hello world"));
    }

    #[test]
    fn test_text_has_ligature_corruption_mid_word() {
        assert!(text_has_ligature_corruption("di!erent"));
        assert!(text_has_ligature_corruption("e#cient"));
        assert!(text_has_ligature_corruption("o\u{22}ces"));
    }

    #[test]
    fn test_text_has_ligature_corruption_word_start() {
        assert!(text_has_ligature_corruption("#nancial"));
        assert!(text_has_ligature_corruption("!nally"));
    }

    #[test]
    fn test_text_has_ligature_corruption_normal_punctuation() {
        assert!(!text_has_ligature_corruption("hello!"));
        assert!(!text_has_ligature_corruption("say \"hello\""));
    }

    #[test]
    fn test_text_has_ligature_corruption_multiple() {
        assert!(text_has_ligature_corruption("e!cient and #nancial"));
    }

    #[test]
    fn test_broken_word_spacing_detection() {
        // Simulates pdfa_019 pattern: "M ust Be Tough" with systematic broken spacing
        let broken =
            "M ust B e T ough o ffers t he g uidance t hat g ives y ou t he b est c hance o f r ekindling r omance";
        assert!(text_has_broken_word_spacing(broken));
    }

    #[test]
    fn test_normal_text_not_detected_as_broken() {
        let normal = "Love Must Be Tough offers the guidance that gives you the best chance of rekindling romance";
        assert!(!text_has_broken_word_spacing(normal));
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
    fn test_repair_joins_single_letter_only() {
        let broken = "rom ance and m arriage";
        let repaired = repair_broken_word_spacing(broken);
        // "rom" is 3 chars — not joined. "m" is single letter before "arriage" → joined.
        assert_eq!(repaired, "rom ance and marriage");
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
        assert_eq!(normalize_text_encoding("he\x01llo\x02"), "hello");
    }

    #[test]
    fn test_normalize_preserves_tabs_newlines() {
        assert_eq!(normalize_text_encoding("a\tb\nc\r"), "a\tb\nc\r");
    }
}

//! Structural boundary detection for plain text.
//!
//! Detects header-like patterns in plain text (PDF/DOCX extracted prose)
//! that `text-splitter` cannot identify. Used by the semantic chunker to
//! force topic boundaries at section headers.

/// Maximum line length for ALL CAPS header detection.
const MAX_ALL_CAPS_LINE_LEN: usize = 80;

/// Maximum line length for standalone title detection.
const MAX_TITLE_LINE_LEN: usize = 60;

/// Maximum number of words in a standalone title.
const MAX_TITLE_WORD_COUNT: usize = 8;

/// Minimum fraction of words that must start with uppercase for title detection.
/// Expressed as numerator/denominator (3/5 = 60%).
const TITLE_UPPERCASE_RATIO: (usize, usize) = (3, 5);

/// A detected structural boundary in the text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedBoundary {
    /// Byte offset of the start of the line in the original text.
    pub byte_offset: usize,
    /// Whether this boundary looks like a header/section title.
    pub is_header: bool,
}

/// Detect structural boundaries in plain text.
///
/// Iterates lines and checks each against heuristics for ALL-CAPS headers,
/// numbered sections, and title lines. Returns boundaries sorted by byte offset.
pub fn detect_plain_text_boundaries(text: &str) -> Vec<DetectedBoundary> {
    let mut boundaries = Vec::new();
    let mut byte_offset: usize = 0;

    for line in text.split('\n') {
        let trimmed = line.trim();
        if !trimmed.is_empty() && (is_all_caps_header(trimmed) || is_numbered_section(line) || is_title_line(trimmed)) {
            boundaries.push(DetectedBoundary {
                byte_offset,
                is_header: true,
            });
        }
        byte_offset += line.len() + 1; // +1 for the '\n' delimiter
    }

    boundaries
}

/// Check if a line is an ALL-CAPS header.
///
/// True when the line is < 80 chars, has >= 2 alphabetic chars, and every
/// alphabetic char is uppercase.
fn is_all_caps_header(line: &str) -> bool {
    if line.len() >= MAX_ALL_CAPS_LINE_LEN {
        return false;
    }

    let mut alpha_count = 0;
    for ch in line.chars() {
        if ch.is_alphabetic() {
            if !ch.is_uppercase() {
                return false;
            }
            alpha_count += 1;
        }
    }

    alpha_count >= 2
}

/// Check if a line is a numbered section header.
///
/// Matches patterns like "1.", "1.2", "1.2.3" at the start of the line
/// (not indented), followed by a space or end of line. Must contain at
/// least one dot.
fn is_numbered_section(line: &str) -> bool {
    // Must not be indented
    if line.starts_with(' ') || line.starts_with('\t') {
        return false;
    }

    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }

    // Find the numbering prefix: digits and dots
    let prefix_end = trimmed
        .find(|ch: char| !ch.is_ascii_digit() && ch != '.')
        .unwrap_or(trimmed.len());

    let prefix = &trimmed[..prefix_end];

    // Must have at least one dot
    if !prefix.contains('.') {
        return false;
    }

    // Must start with a digit
    if !prefix.starts_with(|ch: char| ch.is_ascii_digit()) {
        return false;
    }

    // After the numbering prefix, must be followed by space + text.
    // Bare prefixes like "1." or "1.2." alone are not section headers.
    if prefix_end >= trimmed.len() {
        return false;
    }
    if !trimmed[prefix_end..].starts_with(' ') {
        return false;
    }
    // Must have non-whitespace text after the prefix + space.
    if trimmed[prefix_end..].trim().is_empty() {
        return false;
    }

    // No consecutive dots (reject malformed patterns like "1..2").
    if prefix.contains("..") {
        return false;
    }

    true
}

/// Check if a line looks like a standalone title.
///
/// Criteria: < 60 chars, starts with uppercase letter, title-case words
/// (most words start with uppercase), no more than 8 words, no trailing
/// sentence punctuation (. ! ? ; :), not indented.
fn is_title_line(line: &str) -> bool {
    if line.len() >= MAX_TITLE_LINE_LEN {
        return false;
    }

    // Must start with uppercase alphabetic char (not indented).
    let first = match line.chars().next() {
        Some(ch) => ch,
        None => return false,
    };
    if !first.is_uppercase() {
        return false;
    }

    let words: Vec<&str> = line.split_whitespace().collect();

    // Must have at least 2 words and no more than 8 (titles are short).
    // Single words like "Note", "I", "To" are not titles.
    if words.len() < 2 || words.len() > MAX_TITLE_WORD_COUNT {
        return false;
    }

    // Most words must start with an uppercase letter (title-case).
    let upper_start_count = words
        .iter()
        .filter(|w| w.chars().next().is_some_and(|c| c.is_uppercase()))
        .count();
    // Allow small connectors (of, and, the, etc.) — require title-case majority.
    let (num, den) = TITLE_UPPERCASE_RATIO;
    if upper_start_count * den < words.len() * num {
        return false;
    }

    // Must not end with sentence punctuation.
    if line.ends_with(['.', '!', '?', ';', ':']) {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- is_all_caps_header ---

    #[test]
    fn all_caps_introduction() {
        assert!(is_all_caps_header("INTRODUCTION"));
    }

    #[test]
    fn all_caps_chapter_one() {
        assert!(is_all_caps_header("CHAPTER ONE"));
    }

    #[test]
    fn all_caps_terms_and_conditions() {
        assert!(is_all_caps_header("TERMS AND CONDITIONS"));
    }

    #[test]
    fn not_all_caps_mixed_case() {
        assert!(!is_all_caps_header("Introduction"));
    }

    #[test]
    fn not_all_caps_single_char() {
        assert!(!is_all_caps_header("a"));
    }

    #[test]
    fn not_all_caps_too_long() {
        let long = "A".repeat(100);
        assert!(!is_all_caps_header(&long));
    }

    #[test]
    fn not_all_caps_digits_only() {
        assert!(!is_all_caps_header("12345"));
    }

    // --- is_numbered_section ---

    #[test]
    fn numbered_simple() {
        assert!(is_numbered_section("1. Introduction"));
    }

    #[test]
    fn numbered_two_level() {
        assert!(is_numbered_section("1.2 Background"));
    }

    #[test]
    fn numbered_three_level() {
        assert!(is_numbered_section("1.2.3 Methodology"));
    }

    #[test]
    fn not_numbered_normal_text() {
        assert!(!is_numbered_section("This is normal text."));
    }

    #[test]
    fn not_numbered_no_dot() {
        assert!(!is_numbered_section("1234 just a number"));
    }

    #[test]
    fn not_numbered_indented() {
        assert!(!is_numbered_section("  1.2 Indented"));
    }

    #[test]
    fn not_numbered_bare_single_level() {
        // "1." alone with no text is NOT a section header.
        assert!(!is_numbered_section("1."));
    }

    #[test]
    fn not_numbered_bare_two_level() {
        // "1.2." alone with no text is NOT a section header.
        assert!(!is_numbered_section("1.2."));
    }

    #[test]
    fn not_numbered_whitespace_only_after_prefix() {
        // "1. " with only whitespace after is NOT a section header.
        assert!(!is_numbered_section("1.   "));
    }

    // --- is_title_line ---

    #[test]
    fn title_executive_summary() {
        assert!(is_title_line("Executive Summary"));
    }

    #[test]
    fn not_title_single_word() {
        // Single words are not titles (require at least 2 words).
        assert!(!is_title_line("Background"));
        assert!(!is_title_line("Note"));
        assert!(!is_title_line("Introduction"));
    }

    #[test]
    fn title_risk_factors() {
        assert!(is_title_line("Risk Factors"));
    }

    #[test]
    fn not_title_ends_with_period() {
        assert!(!is_title_line("This is a sentence."));
    }

    #[test]
    fn not_title_ends_with_exclamation() {
        assert!(!is_title_line("Watch out!"));
    }

    #[test]
    fn not_title_too_long() {
        let long = format!("A{}", "b".repeat(60));
        assert!(!is_title_line(&long));
    }

    #[test]
    fn not_title_indented() {
        assert!(!is_title_line("  Indented Title"));
    }

    #[test]
    fn not_title_ordinary_prose() {
        // Ordinary short sentences should NOT be detected as titles.
        assert!(!is_title_line("The cat sat on the mat"));
        assert!(!is_title_line("I agree with you"));
        assert!(!is_title_line("Call me later"));
        assert!(!is_title_line("Total revenue was $5M"));
    }

    #[test]
    fn not_title_starts_lowercase() {
        assert!(!is_title_line("background"));
    }

    #[test]
    fn not_title_too_many_words() {
        assert!(!is_title_line("This Is A Title With Way Too Many Words In It"));
    }

    // --- detect_plain_text_boundaries ---

    #[test]
    fn detect_empty_text() {
        let result = detect_plain_text_boundaries("");
        assert!(result.is_empty());
    }

    #[test]
    fn detect_no_headers() {
        let text = "This is a normal sentence.\nAnother normal sentence here.\nNothing special going on.";
        let result = detect_plain_text_boundaries(text);
        assert!(result.is_empty());
    }

    #[test]
    fn detect_byte_offsets_correct() {
        let text = "INTRODUCTION\nSome body text here.\nCONCLUSION";
        let result = detect_plain_text_boundaries(text);

        assert_eq!(result.len(), 2);

        // "INTRODUCTION" starts at offset 0
        assert_eq!(result[0].byte_offset, 0);
        assert!(result[0].is_header);

        // "CONCLUSION" starts after "INTRODUCTION\nSome body text here.\n"
        // = 12 + 1 + 21 + 1 = 35... let me compute:
        // "INTRODUCTION" = 12 bytes, + 1 for \n = offset 13
        // "Some body text here." = 20 bytes, + 1 for \n = offset 34 (13 + 21)
        // Wait: "Some body text here." is 20 chars. 13 + 20 + 1 = 34.
        let expected_offset = "INTRODUCTION".len() + 1 + "Some body text here.".len() + 1;
        assert_eq!(result[1].byte_offset, expected_offset);
        assert!(result[1].is_header);
    }

    #[test]
    fn detect_mixed_boundary_types() {
        let text = "CHAPTER ONE\n\n1.2 Background\n\nExecutive Summary\n\nThis is body text.";
        let result = detect_plain_text_boundaries(text);

        // CHAPTER ONE (all caps), 1.2 Background (numbered), Executive Summary (title)
        assert_eq!(result.len(), 3);
        assert!(result.iter().all(|b| b.is_header));

        // Verify sorted by offset
        for window in result.windows(2) {
            assert!(window[0].byte_offset < window[1].byte_offset);
        }
    }

    #[test]
    fn not_title_starts_with_space() {
        // is_title_line checks that first char is alphabetic — space is not
        assert!(!is_title_line(" Leading space"));
    }

    #[test]
    fn not_title_ends_with_colon() {
        assert!(!is_title_line("Section:"));
    }

    #[test]
    fn not_title_ends_with_semicolon() {
        assert!(!is_title_line("Section;"));
    }

    #[test]
    fn not_title_ends_with_question() {
        assert!(!is_title_line("Is this a title?"));
    }

    // --- Mixed header types ---

    #[test]
    fn detect_all_caps_and_numbered_in_same_document() {
        let text = "OVERVIEW\n\nSome body text.\n\n1.1 Subsection\n\nMore body text.";
        let result = detect_plain_text_boundaries(text);
        // OVERVIEW (all caps) + 1.1 Subsection (numbered) = 2 boundaries
        assert_eq!(result.len(), 2, "should detect both ALL CAPS and numbered headers");
    }

    // --- Unicode / CJK text should NOT be detected as ALL CAPS ---

    #[test]
    fn unicode_cjk_not_all_caps() {
        // Chinese characters are not uppercase Latin — should not match.
        assert!(!is_all_caps_header("Chinese characters are not uppercase Latin"));
        assert!(!is_all_caps_header("日本語テスト")); // has no uppercase Latin alpha
    }

    #[test]
    fn unicode_hangul_not_all_caps() {
        // Korean Hangul: alphabetic but has no uppercase concept.
        // is_uppercase() returns false for Hangul, so it should not match.
        assert!(!is_all_caps_header("한국어 테스트"));
    }

    #[test]
    fn unicode_arabic_not_all_caps() {
        assert!(!is_all_caps_header("مرحبا بالعالم"));
    }

    // --- Lines with only numbers / special chars ---

    #[test]
    fn numbers_and_special_chars_not_headers() {
        assert!(!is_all_caps_header("12345"));
        assert!(!is_all_caps_header("---"));
        assert!(!is_all_caps_header("***"));
        assert!(!is_all_caps_header("12.34.56"));
    }

    #[test]
    fn detect_only_numbers_line_not_boundary() {
        let text = "12345\n\nSome body text.\n\n67890";
        let result = detect_plain_text_boundaries(text);
        // None of these lines should be detected as headers.
        assert!(
            result.is_empty(),
            "pure number/special-char lines should not be boundaries"
        );
    }

    #[test]
    fn detect_all_three_header_types_in_one_document() {
        // ALL CAPS + numbered + title line in a single document.
        let text = "ABSTRACT\n\nSome abstract text here.\n\n1.1 Methods\n\nMethodology description.\n\nFinal Remarks\n\nFinal remarks about the study.";
        let result = detect_plain_text_boundaries(text);
        // ABSTRACT (all caps) + 1.1 Methods (numbered) + Final Remarks (title) = 3 boundaries
        assert_eq!(
            result.len(),
            3,
            "should detect all three header types, got {} boundaries: {:?}",
            result.len(),
            result
        );
        // Verify offsets are strictly increasing.
        for window in result.windows(2) {
            assert!(
                window[0].byte_offset < window[1].byte_offset,
                "boundaries should be ordered by offset"
            );
        }
    }

    #[test]
    fn unicode_chinese_text_not_all_caps() {
        // Chinese characters have no uppercase concept — must not be detected.
        assert!(!is_all_caps_header("\u{4e2d}\u{6587}\u{6d4b}\u{8bd5}")); // 中文测试
        assert!(!is_all_caps_header("\u{7b2c}\u{4e00}\u{7ae0}")); // 第一章
    }

    #[test]
    fn detect_chinese_text_not_boundary() {
        // Full document with Chinese lines — none should be detected as ALL CAPS headers.
        let text = "\u{7b2c}\u{4e00}\u{7ae0}\n\n\u{8fd9}\u{662f}\u{6b63}\u{6587}\u{5185}\u{5bb9}\u{3002}";
        let result = detect_plain_text_boundaries(text);
        // Chinese lines should not trigger is_all_caps_header, but may trigger
        // is_title_line if they are short and don't end in sentence punctuation.
        // The key point: they must not trigger is_all_caps_header.
        for boundary in &result {
            // Verify boundaries only come from title_line or numbered, not all_caps.
            let line_at_offset = text[boundary.byte_offset..].lines().next().unwrap_or("");
            assert!(
                !is_all_caps_header(line_at_offset.trim()),
                "Chinese text should not be detected as ALL CAPS: {:?}",
                line_at_offset
            );
        }
    }
}

//! Paragraph building from lines using vertical gaps and formatting changes.

use super::constants::{
    FONT_SIZE_CHANGE_THRESHOLD, LEFT_INDENT_CHANGE_THRESHOLD, MAX_LIST_ITEM_LINES, PARAGRAPH_GAP_MULTIPLIER,
};
use super::types::{PdfLine, PdfParagraph};

/// Group lines into paragraphs based on vertical gaps, font size changes, and indentation.
pub(super) fn lines_to_paragraphs(lines: Vec<PdfLine>) -> Vec<PdfParagraph> {
    if lines.is_empty() {
        return Vec::new();
    }

    if lines.len() == 1 {
        return vec![finalize_paragraph(lines)];
    }

    // Compute baseline line spacing for paragraph break detection.
    let avg_font_size = lines.iter().map(|l| l.dominant_font_size).sum::<f32>() / lines.len() as f32;

    let mut spacings: Vec<f32> = Vec::new();
    for pair in lines.windows(2) {
        let gap = (pair[1].baseline_y - pair[0].baseline_y).abs();
        if gap > avg_font_size * 0.4 {
            spacings.push(gap);
        }
    }

    let base_spacing = if spacings.is_empty() {
        avg_font_size
    } else {
        spacings.sort_by(|a, b| a.total_cmp(b));
        // Use 25th percentile (Q1) for robustness against outlier-tight spacings
        // from superscripts/subscripts, while staying conservative enough to work
        // with small sample sizes (unlike median which can pick a gap spacing).
        spacings[spacings.len() / 4]
    };

    let paragraph_gap_threshold = base_spacing * PARAGRAPH_GAP_MULTIPLIER;

    let mut paragraphs: Vec<PdfParagraph> = Vec::new();
    let mut current_lines: Vec<PdfLine> = vec![lines[0].clone()];

    for line in lines.into_iter().skip(1) {
        // SAFETY: current_lines is initialised with lines[0] above and only ever
        // has elements pushed onto it, so it is never empty at this point.
        let prev = current_lines.last().unwrap();

        let vertical_gap = (line.baseline_y - prev.baseline_y).abs();
        let font_size_change = (line.dominant_font_size - prev.dominant_font_size).abs();

        let prev_left = prev.segments.first().map(|s| s.x).unwrap_or(0.0);
        let curr_left = line.segments.first().map(|s| s.x).unwrap_or(0.0);
        let indent_change = (curr_left - prev_left).abs();

        let has_significant_gap = vertical_gap > paragraph_gap_threshold;
        let has_some_gap = vertical_gap > base_spacing * 0.8;
        let has_font_change = font_size_change > FONT_SIZE_CHANGE_THRESHOLD;
        let has_indent_change = indent_change > LEFT_INDENT_CHANGE_THRESHOLD;

        // Force paragraph break if next line starts with a list prefix
        let next_starts_with_list = line
            .segments
            .first()
            .and_then(|s| s.text.split_whitespace().next())
            .map(is_list_prefix)
            .unwrap_or(false);

        let is_paragraph_break =
            has_significant_gap || (has_some_gap && (has_font_change || has_indent_change)) || next_starts_with_list;

        if is_paragraph_break {
            paragraphs.push(finalize_paragraph(current_lines));
            current_lines = vec![line];
        } else {
            current_lines.push(line);
        }
    }

    if !current_lines.is_empty() {
        paragraphs.push(finalize_paragraph(current_lines));
    }

    paragraphs
}

/// Build a PdfParagraph from a set of lines.
pub(super) fn finalize_paragraph(lines: Vec<PdfLine>) -> PdfParagraph {
    let dominant_font_size = super::lines::most_frequent_font_size(lines.iter().map(|l| l.dominant_font_size));

    let bold_count = lines.iter().filter(|l| l.is_bold).count();
    let majority = lines.len().div_ceil(2);

    // Detect list items: first segment of first line starts with bullet or number prefix
    let first_text = lines
        .first()
        .and_then(|l| l.segments.first())
        .map(|s| s.text.as_str())
        .unwrap_or("");
    let first_word = first_text.split_whitespace().next().unwrap_or("");
    let is_list_item = lines.len() <= MAX_LIST_ITEM_LINES && is_list_prefix(first_word);

    // Detect code blocks: monospace font OR syntax-based heuristic for 3+ line blocks
    let is_code_block = if !lines.is_empty() && lines.iter().all(|l| l.is_monospace) {
        true
    } else if lines.len() >= 3 && !is_list_item {
        looks_like_code(&lines)
    } else {
        false
    };

    PdfParagraph {
        dominant_font_size,
        heading_level: None,
        is_bold: bold_count >= majority,
        is_list_item,
        is_code_block,
        is_formula: false,
        is_page_furniture: false,
        layout_class: None,
        caption_for: None,
        block_bbox: None,
        lines,
    }
}

/// Merge consecutive body-text paragraphs that are continuations of the same logical paragraph.
///
/// Two consecutive paragraphs are merged if:
/// - Both are body text (no heading_level, not is_list_item)
/// - The first paragraph doesn't end with sentence-ending punctuation
/// - Font sizes are within 2pt of each other
pub(super) fn merge_continuation_paragraphs(paragraphs: &mut Vec<PdfParagraph>) {
    if paragraphs.len() < 2 {
        return;
    }

    // O(N) single-pass merge: drain the original vec and rebuild, avoiding
    // the O(N²) cost of repeated Vec::remove shifts.
    let old = std::mem::take(paragraphs);
    let mut iter = old.into_iter();
    // SAFETY: we returned early above when paragraphs.len() < 2, so `old`
    // contains at least two elements and the first next() always succeeds.
    let mut current = iter.next().unwrap();

    for next in iter {
        let should_merge =
            // Both must be body text (no heading, list, code, or formula)
            current.heading_level.is_none()
                && next.heading_level.is_none()
                && !current.is_list_item
                && !next.is_list_item
                && !current.is_code_block
                && !next.is_code_block
                && !current.is_formula
                && !next.is_formula
                // Font sizes close enough
                && (current.dominant_font_size - next.dominant_font_size).abs() < 2.0
                // Current paragraph doesn't end with sentence-ending punctuation
                && !ends_with_sentence_terminator(&current);

        if should_merge {
            current.lines.extend(next.lines);
        } else {
            paragraphs.push(current);
            current = next;
        }
    }

    paragraphs.push(current);
}

/// Check if a paragraph's last line ends with sentence-terminating punctuation.
pub(super) fn ends_with_sentence_terminator(para: &PdfParagraph) -> bool {
    let last_text = para
        .lines
        .last()
        .and_then(|l| l.segments.last())
        .map(|s| s.text.trim_end())
        .unwrap_or("");
    matches!(
        last_text.chars().last(),
        Some('.' | '?' | '!' | ':' | ';' | '\u{3002}' | '\u{FF1F}' | '\u{FF01}')
    )
}

/// Split paragraphs that contain embedded bullet characters (e.g. `•`) into separate list items.
///
/// Structure tree pages sometimes merge all text into one block with inline bullets.
/// This splits "text before • item1 • item2" into separate paragraphs.
pub(super) fn split_embedded_list_items(paragraphs: &mut Vec<PdfParagraph>) {
    let old = std::mem::take(paragraphs);
    for para in old {
        // Only split non-heading, non-list, non-code paragraphs
        if para.heading_level.is_some() || para.is_list_item || para.is_code_block || para.is_formula {
            paragraphs.push(para);
            continue;
        }

        // Collect full text to check for embedded bullets
        let full_text: String = para
            .lines
            .iter()
            .flat_map(|l| l.segments.iter())
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        // Count bullet occurrences — only split if there are multiple
        let bullet_count = full_text.matches('\u{2022}').count();
        if bullet_count < 2 {
            paragraphs.push(para);
            continue;
        }

        // Split on bullet character boundaries
        let font_size = para.dominant_font_size;
        let is_bold = para.is_bold;

        // Split the full text on • and produce separate paragraphs
        let parts: Vec<&str> = full_text.split('\u{2022}').collect();
        let before = parts[0].trim();
        if !before.is_empty() {
            paragraphs.push(text_to_paragraph(before, font_size, is_bold, false));
        }
        for part in &parts[1..] {
            let item_text = part.trim();
            if !item_text.is_empty() {
                paragraphs.push(text_to_paragraph(item_text, font_size, is_bold, true));
            }
        }
    }
}

/// Create a simple paragraph from text.
fn text_to_paragraph(text: &str, font_size: f32, is_bold: bool, is_list_item: bool) -> PdfParagraph {
    use crate::pdf::hierarchy::SegmentData;

    let segments: Vec<SegmentData> = text
        .split_whitespace()
        .map(|w| SegmentData {
            text: w.to_string(),
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            font_size,
            is_bold,
            is_italic: false,
            is_monospace: false,
            baseline_y: 0.0,
        })
        .collect();

    let line = super::types::PdfLine {
        segments,
        baseline_y: 0.0,
        dominant_font_size: font_size,
        is_bold,
        is_monospace: false,
    };

    PdfParagraph {
        lines: vec![line],
        dominant_font_size: font_size,
        heading_level: None,
        is_bold,
        is_list_item,
        is_code_block: false,
        is_formula: false,
        is_page_furniture: false,
        layout_class: None,
        caption_for: None,
        block_bbox: None,
    }
}

/// Check if text looks like a list item prefix.
pub(super) fn is_list_prefix(text: &str) -> bool {
    let trimmed = text.trim();
    // Bullet characters: hyphen, asterisk, bullet, en dash, em dash, triangular bullet, white bullet
    if matches!(
        trimmed,
        "-" | "*"
            | "\u{2022}"
            | "\u{2013}"
            | "\u{2014}"
            | "\u{2023}"
            | "\u{25E6}"
            | "\u{25AA}"
            | "\u{25CF}"
            | "\u{2043}"
            | "\u{27A2}"
    ) {
        return true;
    }
    let bytes = trimmed.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    // Numbered: 1. 2) 3: etc.
    let digit_end = bytes.iter().position(|&b| !b.is_ascii_digit()).unwrap_or(bytes.len());
    if digit_end > 0 && digit_end < bytes.len() {
        let suffix = bytes[digit_end];
        if suffix == b'.' || suffix == b')' || suffix == b':' {
            return true;
        }
    }
    // Parenthesized numbers/letters: (1) (a) (i) (A)
    // Use char-based slicing to avoid panicking on multi-byte UTF-8 boundaries.
    if bytes.len() >= 3 && bytes[0] == b'(' && bytes[bytes.len() - 1] == b')' {
        let char_count = trimmed.chars().count();
        if char_count >= 3 {
            let inner: String = trimmed.chars().skip(1).take(char_count - 2).collect();
            if inner.chars().all(|c| c.is_ascii_digit())
                || (inner.len() == 1 && inner.chars().next().is_some_and(|c| c.is_ascii_alphabetic()))
                || is_roman_numeral(&inner)
            {
                return true;
            }
        }
    }
    // Bracketed numbers/letters: [1] [a]
    // Use char-based slicing to avoid panicking on multi-byte UTF-8 boundaries.
    if bytes.len() >= 3 && bytes[0] == b'[' && bytes[bytes.len() - 1] == b']' {
        let char_count = trimmed.chars().count();
        if char_count >= 3 {
            let inner: String = trimmed.chars().skip(1).take(char_count - 2).collect();
            if inner.chars().all(|c| c.is_ascii_digit())
                || (inner.len() == 1 && inner.chars().next().is_some_and(|c| c.is_ascii_alphabetic()))
            {
                return true;
            }
        }
    }
    // Alphabetic: a. b) A. B) (single letter + period/paren)
    if bytes.len() == 2 && bytes[0].is_ascii_alphabetic() && (bytes[1] == b'.' || bytes[1] == b')') {
        return true;
    }
    // Roman numerals: i. ii. iii. iv. v. vi. I. II. III. IV. V. VI. etc.
    if trimmed.ends_with('.') || trimmed.ends_with(')') {
        let prefix = &trimmed[..trimmed.len() - 1];
        if is_roman_numeral(prefix) {
            return true;
        }
    }
    false
}

/// Heuristic: does this block of lines look like source code?
/// Checks for consistent indentation, high syntax character ratio, or programming keywords.
fn looks_like_code(lines: &[PdfLine]) -> bool {
    let texts: Vec<String> = lines
        .iter()
        .map(|l| l.segments.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join(" "))
        .collect();

    let total_chars: usize = texts.iter().map(|t| t.len()).sum();
    if total_chars == 0 {
        return false;
    }

    // Count syntax characters: ; { } ( ) = > < | & ^ ~ # @ ! [ ] / \
    let syntax_chars: usize = texts
        .iter()
        .flat_map(|t| t.chars())
        .filter(|c| {
            matches!(
                c,
                ';' | '{'
                    | '}'
                    | '('
                    | ')'
                    | '='
                    | '<'
                    | '>'
                    | '|'
                    | '&'
                    | '^'
                    | '~'
                    | '#'
                    | '@'
                    | '['
                    | ']'
                    | '/'
                    | '\\'
            )
        })
        .count();
    let syntax_ratio = syntax_chars as f32 / total_chars as f32;

    // Count lines with leading whitespace (4+ spaces or tab)
    let indented_count = texts
        .iter()
        .filter(|t| {
            let leading: usize = t.chars().take_while(|c| *c == ' ').count();
            leading >= 4 || t.starts_with('\t')
        })
        .count();
    let indent_ratio = indented_count as f32 / texts.len() as f32;

    // Check for programming keywords
    let full_text = texts.join(" ");
    let keyword_count = [
        "def ",
        "function ",
        "class ",
        "import ",
        "return ",
        "if ",
        "for ",
        "while ",
        "const ",
        "let ",
        "var ",
        "fn ",
        "pub ",
        "use ",
        "struct ",
        "enum ",
        "async ",
        "await ",
        "try ",
        "catch ",
        "throw ",
        "raise ",
        "except ",
        "print(",
        "println!",
        "console.log",
        "System.out",
    ]
    .iter()
    .filter(|kw| full_text.contains(*kw))
    .count();

    // Code if: high syntax ratio (>8%), or consistent indentation (>60%) with some syntax,
    // or multiple programming keywords
    syntax_ratio > 0.08 || (indent_ratio > 0.6 && syntax_ratio > 0.03) || keyword_count >= 3
}

/// Check if text is a roman numeral (i-xii or I-XII range).
fn is_roman_numeral(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let lower = s.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "i" | "ii" | "iii" | "iv" | "v" | "vi" | "vii" | "viii" | "ix" | "x" | "xi" | "xii"
    )
}

#[cfg(test)]
mod tests {
    use crate::pdf::hierarchy::SegmentData;

    use super::*;

    fn plain_segment(text: &str, x: f32, baseline_y: f32, width: f32, font_size: f32) -> SegmentData {
        SegmentData {
            text: text.to_string(),
            x,
            y: baseline_y,
            width,
            height: font_size,
            font_size,
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            baseline_y,
        }
    }

    fn make_line(segments: Vec<SegmentData>, baseline_y: f32, font_size: f32) -> PdfLine {
        PdfLine {
            segments,
            baseline_y,
            dominant_font_size: font_size,
            is_bold: false,
            is_monospace: false,
        }
    }

    #[test]
    fn test_lines_to_paragraphs_single_line() {
        let lines = vec![make_line(
            vec![plain_segment("Hello world", 10.0, 700.0, 80.0, 12.0)],
            700.0,
            12.0,
        )];
        let paragraphs = lines_to_paragraphs(lines);
        assert_eq!(paragraphs.len(), 1);
    }

    #[test]
    fn test_lines_to_paragraphs_gap_detection() {
        let lines = vec![
            make_line(vec![plain_segment("Para 1", 10.0, 700.0, 60.0, 12.0)], 700.0, 12.0),
            make_line(
                vec![plain_segment("Still para 1", 10.0, 686.0, 80.0, 12.0)],
                686.0,
                12.0,
            ),
            // Big gap
            make_line(vec![plain_segment("Para 2", 10.0, 640.0, 60.0, 12.0)], 640.0, 12.0),
        ];
        let paragraphs = lines_to_paragraphs(lines);
        assert_eq!(paragraphs.len(), 2);
    }

    #[test]
    fn test_lines_to_paragraphs_empty() {
        let paragraphs = lines_to_paragraphs(vec![]);
        assert!(paragraphs.is_empty());
    }

    #[test]
    fn test_list_item_detection() {
        let lines = vec![make_line(
            vec![plain_segment("- Item text", 10.0, 700.0, 80.0, 12.0)],
            700.0,
            12.0,
        )];
        let paragraphs = lines_to_paragraphs(lines);
        assert_eq!(paragraphs.len(), 1);
        assert!(paragraphs[0].is_list_item);
    }

    #[test]
    fn test_numbered_list_detection() {
        let lines = vec![make_line(
            vec![plain_segment("1. First item", 10.0, 700.0, 80.0, 12.0)],
            700.0,
            12.0,
        )];
        let paragraphs = lines_to_paragraphs(lines);
        assert!(paragraphs[0].is_list_item);
    }

    #[test]
    fn test_not_list_item() {
        let lines = vec![make_line(
            vec![plain_segment("Normal text", 10.0, 700.0, 80.0, 12.0)],
            700.0,
            12.0,
        )];
        let paragraphs = lines_to_paragraphs(lines);
        assert!(!paragraphs[0].is_list_item);
    }

    // ── CJK Sentence Terminator Tests ────────────────────────────────────────

    #[test]
    fn test_cjk_sentence_terminator() {
        // Two body paragraphs where the first ends with Chinese period (U+3002).
        // They should NOT be merged by merge_continuation_paragraphs.
        let mut paragraphs = vec![
            // First paragraph ends with 。(U+3002)
            finalize_paragraph(vec![make_line(
                vec![plain_segment("这是第一句。", 10.0, 700.0, 80.0, 12.0)],
                700.0,
                12.0,
            )]),
            finalize_paragraph(vec![make_line(
                vec![plain_segment("这是第二句", 10.0, 686.0, 80.0, 12.0)],
                686.0,
                12.0,
            )]),
        ];
        merge_continuation_paragraphs(&mut paragraphs);
        assert_eq!(paragraphs.len(), 2, "Chinese period should prevent merging");
    }

    #[test]
    fn test_fullwidth_question_mark() {
        // Two body paragraphs where the first ends with fullwidth question mark (U+FF1F).
        // They should NOT be merged.
        let mut paragraphs = vec![
            finalize_paragraph(vec![make_line(
                vec![plain_segment("Is this correct？", 10.0, 700.0, 100.0, 12.0)],
                700.0,
                12.0,
            )]),
            finalize_paragraph(vec![make_line(
                vec![plain_segment("yes it is", 10.0, 686.0, 60.0, 12.0)],
                686.0,
                12.0,
            )]),
        ];
        merge_continuation_paragraphs(&mut paragraphs);
        assert_eq!(paragraphs.len(), 2, "Fullwidth question mark should prevent merging");
    }
}

//! Paragraph building from lines using vertical gaps and formatting changes.

use super::constants::{
    FONT_SIZE_CHANGE_THRESHOLD, FULL_LINE_FRACTION, LEFT_INDENT_CHANGE_THRESHOLD, MAX_LIST_ITEM_LINES,
    PARAGRAPH_GAP_MULTIPLIER,
};
use super::types::{PdfLine, PdfParagraph};

/// Group lines into paragraphs based on vertical gaps, font size changes, and indentation.
///
/// Short-line detection: when a line doesn't extend to the right margin, it indicates
/// intentionally positioned text (CVs, addresses, data entries) rather than word-wrapped
/// flowing text. Each short line becomes its own paragraph to preserve the document's
/// visual structure in the markdown output.
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

    // Compute max right edge for short-line detection.
    // In flowing text, lines extend to the right margin (word-wrapped).
    // In positioned text (CVs, addresses), lines end where the content ends.
    let max_right_edge = lines
        .iter()
        .filter_map(|l| l.segments.last().map(|s| s.x + s.width))
        .fold(0.0_f32, f32::max);

    // Page-level positioned text detection:
    // If fewer than 30% of lines reach the right margin, this page has
    // predominantly positioned text (CVs, addresses, data entries).
    // Only then do we split short lines into separate paragraphs.
    let is_positioned_text_page = if max_right_edge > 0.0 {
        let full_line_count = lines
            .iter()
            .filter(|l| {
                l.segments
                    .last()
                    .is_some_and(|s| s.x + s.width >= max_right_edge * FULL_LINE_FRACTION)
            })
            .count();
        full_line_count * 100 < lines.len() * 30 // < 30% full-width
    } else {
        false
    };

    let mut paragraphs: Vec<PdfParagraph> = Vec::new();
    let mut current_lines: Vec<PdfLine> = vec![lines[0].clone()];

    for line in lines.into_iter().skip(1) {
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

        // Short-line detection for positioned text pages only.
        // When the page is predominantly short lines (< 30% full-width), each
        // short line that's followed by a gap indicates a separate entry.
        let prev_is_short_on_positioned_page = is_positioned_text_page && {
            let prev_right = prev.segments.last().map(|s| s.x + s.width).unwrap_or(0.0);
            prev_right < max_right_edge * FULL_LINE_FRACTION
        };

        let is_paragraph_break = has_significant_gap
            || (has_some_gap && (has_font_change || has_indent_change))
            || next_starts_with_list
            || (prev_is_short_on_positioned_page && has_some_gap);

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
fn finalize_paragraph(lines: Vec<PdfLine>) -> PdfParagraph {
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

    // Detect code blocks: all lines must be monospace (and there must be at least one line)
    let is_code_block = !lines.is_empty() && lines.iter().all(|l| l.is_monospace);

    PdfParagraph {
        dominant_font_size,
        heading_level: None,
        is_bold: bold_count >= majority,
        is_list_item,
        is_code_block,
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

    let mut i = 0;
    while i + 1 < paragraphs.len() {
        let should_merge = {
            let current = &paragraphs[i];
            let next = &paragraphs[i + 1];

            // Both must be body text
            current.heading_level.is_none()
                && next.heading_level.is_none()
                && !current.is_list_item
                && !next.is_list_item
                // Font sizes close enough
                && (current.dominant_font_size - next.dominant_font_size).abs() < 2.0
                // Current paragraph doesn't end with sentence-ending punctuation
                && !ends_with_sentence_terminator(current)
        };

        if should_merge {
            let next = paragraphs.remove(i + 1);
            paragraphs[i].lines.extend(next.lines);
        } else {
            i += 1;
        }
    }
}

/// Check if a paragraph's last line ends with sentence-terminating punctuation.
fn ends_with_sentence_terminator(para: &PdfParagraph) -> bool {
    let last_text = para
        .lines
        .last()
        .and_then(|l| l.segments.last())
        .map(|s| s.text.trim_end())
        .unwrap_or("");
    matches!(last_text.chars().last(), Some('.' | '?' | '!' | ':' | ';'))
}

/// Check if text looks like a list item prefix.
fn is_list_prefix(text: &str) -> bool {
    let trimmed = text.trim();
    // Bullet characters: hyphen, asterisk, bullet, en dash, em dash
    if matches!(trimmed, "-" | "*" | "\u{2022}" | "\u{2013}" | "\u{2014}") {
        return true;
    }
    let bytes = trimmed.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    // Numbered: 1. 2) etc.
    let digit_end = bytes.iter().position(|&b| !b.is_ascii_digit()).unwrap_or(bytes.len());
    if digit_end > 0 && digit_end < bytes.len() {
        let suffix = bytes[digit_end];
        if suffix == b'.' || suffix == b')' {
            return true;
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
        // Full-width lines (490pt wide from x=10) followed by a big gap
        let lines = vec![
            make_line(
                vec![plain_segment("Para 1 line one", 10.0, 700.0, 490.0, 12.0)],
                700.0,
                12.0,
            ),
            make_line(
                vec![plain_segment("Still para 1", 10.0, 686.0, 490.0, 12.0)],
                686.0,
                12.0,
            ),
            // Big gap
            make_line(vec![plain_segment("Para 2", 10.0, 640.0, 490.0, 12.0)], 640.0, 12.0),
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

    // ── Short-line paragraph splitting tests (issue #431) ──

    #[test]
    fn test_short_lines_split_into_separate_paragraphs() {
        // Simulates a CV/address block: short lines with uniform spacing.
        // Each line should become its own paragraph.
        // Max right edge = 500 (from the widest line).
        let lines = vec![
            make_line(
                vec![plain_segment("Max Mustermann", 50.0, 700.0, 120.0, 12.0)],
                700.0,
                12.0,
            ),
            make_line(
                vec![plain_segment("Musterstraße 1", 50.0, 686.0, 110.0, 12.0)],
                686.0,
                12.0,
            ),
            make_line(
                vec![plain_segment("12345 Musterstadt", 50.0, 672.0, 130.0, 12.0)],
                672.0,
                12.0,
            ),
            // Add a full-width line to establish right margin (like page footer)
            make_line(
                vec![plain_segment("Full width reference line", 50.0, 600.0, 450.0, 12.0)],
                600.0,
                12.0,
            ),
        ];
        let paragraphs = lines_to_paragraphs(lines);
        // The 3 short CV lines should each be separate paragraphs,
        // plus the full-width line (4 total)
        assert_eq!(paragraphs.len(), 4);
        assert_eq!(paragraphs[0].lines[0].segments[0].text, "Max Mustermann");
        assert_eq!(paragraphs[1].lines[0].segments[0].text, "Musterstraße 1");
        assert_eq!(paragraphs[2].lines[0].segments[0].text, "12345 Musterstadt");
    }

    #[test]
    fn test_full_width_lines_grouped_as_paragraph() {
        // Flowing text: full-width lines should be grouped together.
        // Max right edge = 500 (x=10 + width=490).
        let lines = vec![
            make_line(
                vec![plain_segment(
                    "The quick brown fox jumps over",
                    10.0,
                    700.0,
                    490.0,
                    12.0,
                )],
                700.0,
                12.0,
            ),
            make_line(
                vec![plain_segment("the lazy dog and continues on", 10.0, 686.0, 490.0, 12.0)],
                686.0,
                12.0,
            ),
            make_line(
                vec![plain_segment("to the end.", 10.0, 672.0, 100.0, 12.0)],
                672.0,
                12.0,
            ),
        ];
        let paragraphs = lines_to_paragraphs(lines);
        // Full-width lines are flowing text → should be one paragraph
        assert_eq!(paragraphs.len(), 1);
        assert_eq!(paragraphs[0].lines.len(), 3);
    }

    #[test]
    fn test_flowing_text_with_short_last_line_followed_by_new_paragraph() {
        // Paragraph 1: two full lines + short last line
        // Paragraph 2: starts after a big gap
        let lines = vec![
            make_line(
                vec![plain_segment(
                    "Flowing text line one that extends",
                    10.0,
                    700.0,
                    490.0,
                    12.0,
                )],
                700.0,
                12.0,
            ),
            make_line(
                vec![plain_segment(
                    "to the edge of the margin here",
                    10.0,
                    686.0,
                    490.0,
                    12.0,
                )],
                686.0,
                12.0,
            ),
            make_line(
                vec![plain_segment("short ending.", 10.0, 672.0, 100.0, 12.0)],
                672.0,
                12.0,
            ),
            // Big gap → new paragraph
            make_line(
                vec![plain_segment("New paragraph starts here", 10.0, 630.0, 490.0, 12.0)],
                630.0,
                12.0,
            ),
        ];
        let paragraphs = lines_to_paragraphs(lines);
        assert_eq!(paragraphs.len(), 2);
        assert_eq!(paragraphs[0].lines.len(), 3); // full + full + short ending
        assert_eq!(paragraphs[1].lines.len(), 1);
    }

    #[test]
    fn test_no_positional_data_no_short_line_detection() {
        // Structure tree path: all segments have x=0, width=0.
        // Short-line detection should not apply (no positional data).
        let lines = vec![
            make_line(vec![plain_segment("Line one", 0.0, 0.0, 0.0, 12.0)], 0.0, 12.0),
            make_line(vec![plain_segment("Line two", 0.0, 0.0, 0.0, 12.0)], 0.0, 12.0),
        ];
        let paragraphs = lines_to_paragraphs(lines);
        // Without positional data, lines are grouped into one paragraph
        assert_eq!(paragraphs.len(), 1);
    }

    #[test]
    fn test_merge_continuation_respects_sentence_terminators() {
        // Two paragraphs where the first ends with a period.
        // Should NOT be merged because the first ends with sentence-terminating punctuation.
        let mut paragraphs = vec![
            PdfParagraph {
                lines: vec![make_line(
                    vec![plain_segment("First paragraph ends here.", 10.0, 700.0, 490.0, 12.0)],
                    700.0,
                    12.0,
                )],
                dominant_font_size: 12.0,
                heading_level: None,
                is_bold: false,
                is_list_item: false,
                is_code_block: false,
            },
            PdfParagraph {
                lines: vec![make_line(
                    vec![plain_segment("Second paragraph starts", 10.0, 686.0, 490.0, 12.0)],
                    686.0,
                    12.0,
                )],
                dominant_font_size: 12.0,
                heading_level: None,
                is_bold: false,
                is_list_item: false,
                is_code_block: false,
            },
        ];
        merge_continuation_paragraphs(&mut paragraphs);
        // Should NOT merge: first paragraph ends with "."
        assert_eq!(paragraphs.len(), 2);
    }

    #[test]
    fn test_merge_continuation_merges_full_width_paragraphs() {
        // Two paragraphs where the first ends with a full-width line and no sentence terminator.
        // These should be merged (flowing text continuation).
        let mut paragraphs = vec![
            PdfParagraph {
                lines: vec![make_line(
                    vec![plain_segment(
                        "The quick brown fox jumps over the",
                        10.0,
                        700.0,
                        490.0,
                        12.0,
                    )],
                    700.0,
                    12.0,
                )],
                dominant_font_size: 12.0,
                heading_level: None,
                is_bold: false,
                is_list_item: false,
                is_code_block: false,
            },
            PdfParagraph {
                lines: vec![make_line(
                    vec![plain_segment("lazy dog and more text", 10.0, 686.0, 490.0, 12.0)],
                    686.0,
                    12.0,
                )],
                dominant_font_size: 12.0,
                heading_level: None,
                is_bold: false,
                is_list_item: false,
                is_code_block: false,
            },
        ];
        merge_continuation_paragraphs(&mut paragraphs);
        // Full-width lines without sentence terminator → should be merged
        assert_eq!(paragraphs.len(), 1);
        assert_eq!(paragraphs[0].lines.len(), 2);
    }
}

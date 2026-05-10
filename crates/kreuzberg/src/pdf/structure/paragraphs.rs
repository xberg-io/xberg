//! Utilities for splitting and analyzing PDF paragraphs.

use super::types::PdfParagraph;

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
        // Both must be body text (no heading, list, code, or formula)
        let both_body = current.heading_level.is_none()
            && next.heading_level.is_none()
            && !current.is_list_item
            && !next.is_list_item
            && !current.is_code_block
            && !next.is_code_block
            && !current.is_formula
            && !next.is_formula;
        // Font sizes close enough
        let fonts_compatible = (current.dominant_font_size - next.dominant_font_size).abs() < 2.0;
        // Merge when current doesn't end with terminator, or when next
        // starts with a lowercase letter (sentence continuation).
        let continuation_signal = !ends_with_sentence_terminator(&current) || starts_with_lowercase_continuation(&next);
        let should_merge = both_body && fonts_compatible && continuation_signal;

        if should_merge {
            current.lines.extend(next.lines);
        } else {
            paragraphs.push(current);
            current = next;
        }
    }

    paragraphs.push(current);
}

/// Check if a paragraph starts with a lowercase letter, indicating it's a
/// continuation of a previous sentence split across paragraph boundaries.
fn starts_with_lowercase_continuation(para: &PdfParagraph) -> bool {
    let first_text = para
        .lines
        .first()
        .and_then(|l| l.segments.first())
        .map(|s| s.text.trim_start())
        .unwrap_or("");
    first_text.chars().next().is_some_and(|c| c.is_lowercase())
}

/// Check if a paragraph's last line ends with sentence-terminating punctuation.
///
/// Used by merge_continuation_paragraphs to determine if two consecutive
/// paragraphs should be merged. Supports ASCII and CJK sentence terminators.
fn ends_with_sentence_terminator(para: &PdfParagraph) -> bool {
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
            assigned_role: None,
        })
        .collect();

    let line = super::types::PdfLine {
        segments,
        baseline_y: 0.0,
        dominant_font_size: font_size,
        is_bold,
        is_monospace: false,
    };

    let lines = vec![line];
    let word_count = PdfParagraph::compute_word_count("", &lines);
    PdfParagraph {
        text: String::new(),
        lines,
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
        word_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- merge_continuation_paragraphs tests --

    fn make_body_paragraph(text: &str, font_size: f32) -> PdfParagraph {
        use crate::pdf::hierarchy::SegmentData;

        let segments = vec![SegmentData {
            text: text.to_string(),
            x: 0.0,
            y: 700.0,
            width: 200.0,
            height: font_size,
            font_size,
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            baseline_y: 700.0,
            assigned_role: None,
        }];

        let lines = vec![super::super::types::PdfLine {
            segments,
            baseline_y: 700.0,
            dominant_font_size: font_size,
            is_bold: false,
            is_monospace: false,
        }];
        let word_count = PdfParagraph::compute_word_count("", &lines);
        PdfParagraph {
            text: String::new(),
            lines,
            dominant_font_size: font_size,
            heading_level: None,
            is_bold: false,
            is_list_item: false,
            is_code_block: false,
            is_formula: false,
            is_page_furniture: false,
            layout_class: None,
            caption_for: None,
            block_bbox: None,
            word_count,
        }
    }

    #[test]
    fn test_merge_lowercase_continuation() {
        // Second paragraph starts with lowercase → merge even if first ends with period
        let mut paragraphs = vec![
            make_body_paragraph("The regulation requires.", 12.0),
            make_body_paragraph("and all operators must comply", 12.0),
        ];
        merge_continuation_paragraphs(&mut paragraphs);
        assert_eq!(paragraphs.len(), 1, "lowercase continuation should be merged");
    }

    #[test]
    fn test_no_merge_different_font_sizes() {
        // Different font sizes (>2pt) should prevent merging
        let mut paragraphs = vec![
            make_body_paragraph("First paragraph", 12.0),
            make_body_paragraph("second paragraph", 16.0),
        ];
        merge_continuation_paragraphs(&mut paragraphs);
        assert_eq!(paragraphs.len(), 2, "different font sizes should prevent merge");
    }

    #[test]
    fn test_merge_no_terminator() {
        // First ends without terminator → merge
        let mut paragraphs = vec![
            make_body_paragraph("The regulation requires", 12.0),
            make_body_paragraph("All operators must comply", 12.0),
        ];
        merge_continuation_paragraphs(&mut paragraphs);
        assert_eq!(paragraphs.len(), 1, "unterminated paragraph should merge with next");
    }

    #[test]
    fn test_no_merge_terminated_uppercase() {
        // First ends with period, second starts with uppercase → don't merge
        let mut paragraphs = vec![
            make_body_paragraph("The regulation requires compliance.", 12.0),
            make_body_paragraph("All operators must comply", 12.0),
        ];
        merge_continuation_paragraphs(&mut paragraphs);
        assert_eq!(
            paragraphs.len(),
            2,
            "terminated paragraph + uppercase start should not merge"
        );
    }

    #[test]
    fn test_starts_with_lowercase_continuation_fn() {
        let para_lower = make_body_paragraph("and furthermore", 12.0);
        assert!(starts_with_lowercase_continuation(&para_lower));

        let para_upper = make_body_paragraph("Furthermore", 12.0);
        assert!(!starts_with_lowercase_continuation(&para_upper));
    }
}

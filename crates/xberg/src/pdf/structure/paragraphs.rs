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

    let old = std::mem::take(paragraphs);
    let mut iter = old.into_iter();
    let mut current = iter.next().unwrap();

    for next in iter {
        let both_body = current.heading_level.is_none()
            && next.heading_level.is_none()
            && !current.is_list_item
            && !next.is_list_item
            && !current.is_code_block
            && !next.is_code_block
            && !current.is_formula
            && !next.is_formula;
        let fonts_compatible = (current.dominant_font_size - next.dominant_font_size).abs() < 2.0;
        // Never merge across a bold-state boundary. A bold run following non-bold
        // prose (or vice versa) is a formatting break — an emphasized heading or a
        // list item's bold lead-in — not a wrapped continuation. Absorbing it would
        // bury the heading as inline bold before it can be classified. This mirrors
        // the `bold_change` paragraph break in the heuristic line grouper.
        let bold_compatible = current.is_bold == next.is_bold;
        let continuation_signal = !ends_with_sentence_terminator(&current) || starts_with_lowercase_continuation(&next);
        let should_merge = both_body && fonts_compatible && bold_compatible && continuation_signal;

        if should_merge {
            current.text.clear();
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
        if para.heading_level.is_some() || para.is_list_item || para.is_code_block || para.is_formula {
            paragraphs.push(para);
            continue;
        }

        let full_text: String = para
            .lines
            .iter()
            .flat_map(|l| l.segments.iter())
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        let bullet_count = full_text.matches(['\u{2022}', '\u{00B7}']).count();
        if bullet_count < 2 {
            paragraphs.push(para);
            continue;
        }

        let font_size = para.dominant_font_size;
        let is_bold = para.is_bold;

        let parts: Vec<&str> = full_text.split(['\u{2022}', '\u{00B7}']).collect();
        let before = parts[0].trim().trim_end_matches('\u{00C2}').trim();
        if !before.is_empty() {
            paragraphs.push(text_to_paragraph(before, font_size, is_bold, false));
        }
        for part in &parts[1..] {
            let item_text = part
                .trim()
                .trim_start_matches('\u{00C2}')
                .trim_end_matches('\u{00C2}')
                .trim();
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
        let mut paragraphs = vec![
            make_body_paragraph("The regulation requires.", 12.0),
            make_body_paragraph("and all operators must comply", 12.0),
        ];
        merge_continuation_paragraphs(&mut paragraphs);
        assert_eq!(paragraphs.len(), 1, "lowercase continuation should be merged");
    }

    #[test]
    fn test_no_merge_different_font_sizes() {
        let mut paragraphs = vec![
            make_body_paragraph("First paragraph", 12.0),
            make_body_paragraph("second paragraph", 16.0),
        ];
        merge_continuation_paragraphs(&mut paragraphs);
        assert_eq!(paragraphs.len(), 2, "different font sizes should prevent merge");
    }

    #[test]
    fn test_merge_no_terminator() {
        let mut paragraphs = vec![
            make_body_paragraph("The regulation requires", 12.0),
            make_body_paragraph("All operators must comply", 12.0),
        ];
        merge_continuation_paragraphs(&mut paragraphs);
        assert_eq!(paragraphs.len(), 1, "unterminated paragraph should merge with next");
    }

    #[test]
    fn test_no_merge_terminated_uppercase() {
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
    fn test_no_merge_across_bold_boundary() {
        // A bold header following unterminated body prose must not be absorbed as
        // a continuation — it should survive as its own paragraph for classification.
        let body = make_body_paragraph(
            "here is also available other sources of this Manual MetcalUser Guide",
            12.0,
        );
        let mut header = make_body_paragraph("Impaired Glucose Tolerance And Impaired Fasting Glucose ...", 12.0);
        header.is_bold = true;
        let mut paragraphs = vec![body, header];
        merge_continuation_paragraphs(&mut paragraphs);
        assert_eq!(paragraphs.len(), 2, "bold header must not merge into non-bold prose");
        assert!(paragraphs[1].is_bold, "the bold header paragraph must be preserved");
    }

    #[test]
    fn test_starts_with_lowercase_continuation_fn() {
        let para_lower = make_body_paragraph("and furthermore", 12.0);
        assert!(starts_with_lowercase_continuation(&para_lower));

        let para_upper = make_body_paragraph("Furthermore", 12.0);
        assert!(!starts_with_lowercase_continuation(&para_upper));
    }

    #[test]
    fn test_merge_clears_precomputed_text_on_heuristic_path() {
        let mut p1 = make_body_paragraph("een indicative", 12.0);
        p1.text = "een indicative".to_string();
        let mut p2 = make_body_paragraph("van toenemende merkbekendheid", 12.0);
        p2.text = "van toenemende merkbekendheid".to_string();
        let mut paragraphs = vec![p1, p2];
        merge_continuation_paragraphs(&mut paragraphs);
        assert_eq!(paragraphs.len(), 1, "lowercase continuation should merge");
        assert!(
            paragraphs[0].text.is_empty(),
            "merged paragraph must clear pre-computed text so assembly joins from segments"
        );
        assert_eq!(paragraphs[0].lines.len(), 2, "both lines must be present after merge");
    }

    #[test]
    fn test_merge_struct_tree_path_text_stays_empty() {
        let mut paragraphs = vec![
            make_body_paragraph("first sentence without terminator", 12.0),
            make_body_paragraph("second continues here", 12.0),
        ];
        assert!(paragraphs[0].text.is_empty());
        merge_continuation_paragraphs(&mut paragraphs);
        assert_eq!(paragraphs.len(), 1);
        assert!(paragraphs[0].text.is_empty());
    }
}

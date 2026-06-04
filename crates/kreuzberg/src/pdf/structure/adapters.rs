//! OCR-to-structure adapters: convert kreuzberg internal types into the PDF
//! structure pipeline's paragraph representation.
#[cfg(feature = "ocr")]
use super::types;

/// Convert an OCR-produced [`crate::types::internal::InternalDocument`] into a vec of [`types::PdfParagraph`]s
/// for the structure assembly pipeline.
///
/// Coordinates are in image-space (y=0 at top) and are flipped to PDF-space
/// (y=0 at bottom) using `page_height_px`.
#[cfg(feature = "ocr")]
#[allow(dead_code)]
pub(crate) fn ocr_doc_to_paragraphs(
    doc: &crate::types::internal::InternalDocument,
    page_height_px: u32,
) -> Vec<types::PdfParagraph> {
    use crate::types::internal::ElementKind;
    let page_h = page_height_px as f32;
    let default_font_size: f32 = 12.0;

    let result: Vec<types::PdfParagraph> = doc
        .elements
        .iter()
        .filter(|e| matches!(e.kind, ElementKind::OcrText { .. }))
        .filter(|e| !e.text.trim().is_empty())
        .map(|e| {
            let block_bbox = e.bbox.as_ref().map(|bb| {
                let left = bb.x0 as f32;
                let right = bb.x1 as f32;
                let pdf_bottom = page_h - bb.y1 as f32;
                let pdf_top = page_h - bb.y0 as f32;
                (left, pdf_bottom, right, pdf_top)
            });

            let text_lines: Vec<&str> = e.text.split('\n').collect();
            let num_lines = text_lines.len().max(1);
            let (base_y, line_height) = if let Some((_left, bottom, _right, top)) = block_bbox {
                let total_height = top - bottom;
                let lh = total_height / num_lines as f32;
                (top, lh)
            } else {
                (0.0, default_font_size)
            };

            let lines: Vec<types::PdfLine> = text_lines
                .iter()
                .enumerate()
                .filter_map(|(original_idx, line)| {
                    if line.trim().is_empty() {
                        return None;
                    }
                    // Use original_idx to preserve correct vertical spacing even when some lines are blank
                    let line_y = base_y - (original_idx as f32 * line_height);
                    let (x, width) = if let Some((left, _, right, _)) = block_bbox {
                        (left, right - left)
                    } else {
                        (0.0, 100.0)
                    };
                    let seg = crate::pdf::hierarchy::SegmentData {
                        text: line.to_string(),
                        x,
                        y: line_y,
                        width,
                        height: line_height,
                        font_size: default_font_size,
                        is_bold: false,
                        is_italic: false,
                        is_monospace: false,
                        baseline_y: line_y,
                        assigned_role: None,
                    };
                    Some(types::PdfLine {
                        segments: vec![seg],
                        baseline_y: line_y,
                        dominant_font_size: default_font_size,
                        is_bold: false,
                        is_monospace: false,
                    })
                })
                .collect();

            let word_count = types::PdfParagraph::compute_word_count(&e.text, &lines);
            types::PdfParagraph {
                text: e.text.clone(),
                lines,
                dominant_font_size: default_font_size,
                heading_level: None,
                is_bold: false,
                is_list_item: false,
                is_code_block: false,
                is_formula: false,
                is_page_furniture: false,
                layout_class: None,
                caption_for: None,
                block_bbox,
                word_count,
            }
        })
        .collect();

    tracing::debug!(
        input_elements = doc
            .elements
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::OcrText { .. }))
            .count(),
        output_paragraphs = result.len(),
        total_text_chars = result.iter().map(|p| p.text.len()).sum::<usize>(),
        "ocr_doc_to_paragraphs"
    );

    result
}

#[cfg(all(feature = "ocr", test))]
mod tests {
    use super::*;
    use crate::types::extraction::BoundingBox;
    use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
    use crate::types::ocr_elements::OcrElementLevel;

    /// Test that OCR elements with mixed content and blank lines preserve all text.
    #[test]
    fn test_ocr_doc_preserves_mixed_content_with_blanks() {
        let mut doc = InternalDocument::new("test");
        let mut elem = InternalElement::text(
            ElementKind::OcrText {
                level: OcrElementLevel::Line,
            },
            "line1\n\nline3",
            0,
        );
        elem.bbox = Some(BoundingBox {
            x0: 10.0,
            y0: 10.0,
            x1: 100.0,
            y1: 70.0,
        });
        doc.push_element(elem);

        let paragraphs = ocr_doc_to_paragraphs(&doc, 1000);

        assert_eq!(paragraphs.len(), 1, "Should have one paragraph");
        let para = &paragraphs[0];

        // Full text should be preserved including blank lines
        assert_eq!(para.text, "line1\n\nline3", "Text should preserve blank lines");

        // Word count should be 2 (line1, line3) - blanks don't contribute
        assert_eq!(para.word_count, 2, "Word count should count only non-blank words");

        // Lines array should have only non-blank lines
        assert_eq!(para.lines.len(), 2, "Lines array should have only non-blank lines");

        // Content should not be filtered out
        assert!(!para.text.is_empty(), "Text should not be empty");
    }

    /// Test that whitespace-only OCR elements are filtered out (correct behavior).
    #[test]
    fn test_ocr_doc_filters_whitespace_only_elements() {
        let mut doc = InternalDocument::new("test");
        let mut elem1 = InternalElement::text(
            ElementKind::OcrText {
                level: OcrElementLevel::Line,
            },
            "   \n  \n  ",
            0,
        );
        elem1.bbox = Some(BoundingBox {
            x0: 10.0,
            y0: 10.0,
            x1: 100.0,
            y1: 70.0,
        });
        doc.push_element(elem1);

        let mut elem2 = InternalElement::text(
            ElementKind::OcrText {
                level: OcrElementLevel::Line,
            },
            "real content",
            0,
        );
        elem2.bbox = Some(BoundingBox {
            x0: 10.0,
            y0: 80.0,
            x1: 100.0,
            y1: 140.0,
        });
        doc.push_element(elem2);

        let paragraphs = ocr_doc_to_paragraphs(&doc, 1000);

        // Should only have the real content paragraph
        assert_eq!(paragraphs.len(), 1, "Should filter out whitespace-only element");
        assert_eq!(paragraphs[0].text, "real content");
    }

    /// Test that blank lines in OCR elements don't affect vertical positioning.
    /// When text contains blank lines (e.g., "A\n\nC"), the lines array should still
    /// have correct y-positions (0 for A, 2*line_height for C, not 1*line_height).
    /// This ensures correct sorting order when multiple paragraphs are interleaved.
    #[test]
    fn test_ocr_doc_blank_lines_preserve_vertical_spacing() {
        let mut doc = InternalDocument::new("test");
        let mut elem = InternalElement::text(
            ElementKind::OcrText {
                level: OcrElementLevel::Line,
            },
            "Line1\n\nLine3", // blank line in middle
            0,
        );
        elem.bbox = Some(BoundingBox {
            x0: 10.0,
            y0: 10.0,
            x1: 100.0,
            y1: 90.0, // 80 pixel height for 3 lines = ~26.67 per line
        });
        doc.push_element(elem);

        let paragraphs = ocr_doc_to_paragraphs(&doc, 1000);
        assert_eq!(paragraphs.len(), 1);
        let para = &paragraphs[0];

        // Text should preserve the blank line
        assert_eq!(para.text, "Line1\n\nLine3");

        // Lines array should have only 2 non-blank lines
        assert_eq!(para.lines.len(), 2);

        // Check vertical spacing: should be at correct y-positions
        let _line_height = 80.0 / 3.0; // total_height / num_lines
        let _base_y = 90.0 - 10.0; // pdf_top = page_h - y0; but y0 is 10, page_h is 1000, so pdf_top = 990

        // Wait, let me recalculate: y0=10, y1=90, page_h=1000
        // pdf_bottom = 1000 - 90 = 910
        // pdf_top = 1000 - 10 = 990
        // total_height = 990 - 910 = 80
        // line_height = 80 / 3 = 26.67
        let expected_line_height = 80.0 / 3.0;

        // Line1 at index 0: line_y = base_y - (0 * line_height) = 990 - 0 = 990
        // Line3 at index 2: line_y = base_y - (2 * line_height) = 990 - 53.33 = 936.67
        // The y-positions should reflect the original indices (0, 2), not filtered indices (0, 1)
        assert!(
            (para.lines[0].baseline_y - 990.0).abs() < 0.1,
            "Line1 should be at y=990, got {}",
            para.lines[0].baseline_y
        );
        assert!(
            (para.lines[1].baseline_y - (990.0 - 2.0 * expected_line_height)).abs() < 0.1,
            "Line3 should be at y={}, got {}",
            990.0 - 2.0 * expected_line_height,
            para.lines[1].baseline_y
        );
    }

    /// Test that OCR elements with content followed by blanks preserve content.
    #[test]
    fn test_ocr_doc_preserves_content_before_blanks() {
        let mut doc = InternalDocument::new("test");
        let mut elem = InternalElement::text(
            ElementKind::OcrText {
                level: OcrElementLevel::Line,
            },
            "important\n\n",
            0,
        );
        elem.bbox = Some(BoundingBox {
            x0: 10.0,
            y0: 10.0,
            x1: 100.0,
            y1: 70.0,
        });
        doc.push_element(elem);

        let paragraphs = ocr_doc_to_paragraphs(&doc, 1000);

        assert_eq!(paragraphs.len(), 1);
        assert_eq!(paragraphs[0].text, "important\n\n");
        assert_eq!(paragraphs[0].word_count, 1);
        assert_eq!(
            paragraphs[0].lines.len(),
            1,
            "Only the non-blank line should be in lines array"
        );
    }
}

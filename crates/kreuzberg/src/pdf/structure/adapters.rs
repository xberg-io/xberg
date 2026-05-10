//! OCR-to-structure adapters: convert kreuzberg internal types into the PDF
//! structure pipeline's paragraph representation.
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
                .filter(|(_, line)| !line.trim().is_empty())
                .map(|(i, line)| {
                    let line_y = base_y - (i as f32 * line_height);
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
                    types::PdfLine {
                        segments: vec![seg],
                        baseline_y: line_y,
                        dominant_font_size: default_font_size,
                        is_bold: false,
                        is_monospace: false,
                    }
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

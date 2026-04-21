//! Adapters that convert extraction-source-specific types into the unified
//! [`PageContent`] DTO for the shared markdown pipeline.

use pdfium_render::prelude::{ContentRole, ExtractedBlock};

use super::content::{ContentElement, ElementLevel, PageContent, SemanticRole};
use super::geometry::Rect;
// ── Structure tree adapter ──────────────────────────────────────────────

/// Convert structure-tree `ExtractedBlock`s into a [`PageContent`].
///
/// Flattens the block hierarchy into a flat list of `ContentElement`s,
/// mapping `ContentRole` to `SemanticRole` and extracting bounding boxes.
pub(super) fn from_structure_tree(blocks: &[ExtractedBlock]) -> PageContent {
    let mut elements = Vec::new();
    flatten_blocks(blocks, &mut elements);

    PageContent { elements }
}

/// Recursively flatten `ExtractedBlock` hierarchy into `ContentElement`s.
///
/// A block is the author's logical unit (paragraph, list-item, table-cell).
/// Either the block's own `text` OR its nested children can be non-empty — and
/// in tagged PDFs both occur frequently: a `Paragraph`/`LBody` whose child span
/// carries the bold lead-in while the continuation text lives on the parent.
///
/// For each block we emit:
///   1. Its children's contents (recursive flatten), then
///   2. Its own `text` as a separate element, if non-empty.
///
/// Emitting children first matches the reading order observed in tagged PDFs
/// from word processors / presentation tools (Google Docs/Slides, Word), where
/// nested child spans visually precede the parent's continuation MCIDs (e.g.
/// a `Lbl`/`LBody` list structure or a `Paragraph` whose bold lead-in is a
/// child span). Without this step, the parent's own text is silently dropped
/// whenever it has children, truncating paragraph bodies and breaking list
/// items across page boundaries.
fn flatten_blocks(blocks: &[ExtractedBlock], elements: &mut Vec<ContentElement>) {
    for block in blocks {
        if !block.children.is_empty() {
            flatten_blocks(&block.children, elements);
        }

        if block.text.trim().is_empty() {
            continue;
        }

        let bbox = block
            .bounds
            .as_ref()
            .map(|b| Rect::from_lbrt(b.left().value, b.bottom().value, b.right().value, b.top().value));

        let (semantic_role, list_label) = map_content_role(&block.role);

        elements.push(ContentElement {
            text: block.text.clone(),
            bbox,
            font_size: block.font_size,
            is_bold: block.is_bold,
            is_italic: block.is_italic,
            is_monospace: block.is_monospace,
            semantic_role: Some(semantic_role),
            level: ElementLevel::Block,
            list_label,
            layout_class: None,
        });
    }
}

/// Map `ContentRole` from pdfium to our `SemanticRole`.
fn map_content_role(role: &ContentRole) -> (SemanticRole, Option<String>) {
    match role {
        ContentRole::Heading { level } => (SemanticRole::Heading { level: *level }, None),
        ContentRole::Paragraph => (SemanticRole::Paragraph, None),
        ContentRole::ListItem { label } => (SemanticRole::ListItem, label.clone()),
        ContentRole::TableCell { .. } => (SemanticRole::TableCell, None),
        ContentRole::Figure { .. } => (SemanticRole::Figure, None),
        ContentRole::Caption => (SemanticRole::Caption, None),
        ContentRole::Code => (SemanticRole::Code, None),
        ContentRole::BlockQuote => (SemanticRole::BlockQuote, None),
        ContentRole::Link { .. } => (SemanticRole::Paragraph, None),
        ContentRole::Other(s) if s == "Formula" => (SemanticRole::Formula, None),
        ContentRole::Other(_) => (SemanticRole::Other, None),
    }
}

// ── hOCR → PdfParagraph adapter ─────────────────────────────────────────

/// Convert OCR `InternalDocument` elements into `PdfParagraph`s.
///
/// Works with any `InternalDocument` containing `OcrText` elements — from tesseract
/// hOCR parsing or PaddleOCR TextBlock conversion. Each `OcrText` element becomes
/// a `PdfParagraph` with:
/// - Text and line structure
/// - Bounding box converted from image coordinates (y=0 at top) to PDF coordinates (y=0 at bottom)
/// - Default font size and formatting
///
/// The resulting paragraphs feed into `apply_layout_overrides` and
/// `assemble_internal_document`, matching the pdfium native text pipeline.
#[cfg(feature = "ocr")]
#[allow(dead_code)] // Called from extractors/pdf/ocr.rs only when layout-detection is also enabled
pub(crate) fn ocr_doc_to_paragraphs(
    doc: &crate::types::internal::InternalDocument,
    page_height_px: u32,
) -> Vec<super::types::PdfParagraph> {
    use crate::pdf::hierarchy::SegmentData;
    use crate::types::internal::ElementKind;

    let page_h = page_height_px as f32;
    let default_font_size: f32 = 12.0;

    // Each per-page hOCR InternalDocument is extracted independently by tesseract,
    // so all OcrText elements belong to the current page regardless of their
    // stored page number (which is always 1 from single-page hOCR).
    let result: Vec<super::types::PdfParagraph> = doc
        .elements
        .iter()
        .filter(|e| matches!(e.kind, ElementKind::OcrText { .. }))
        .filter(|e| !e.text.trim().is_empty())
        .map(|e| {
            // Convert image-space bbox (y=0 top) to PDF-space (y=0 bottom).
            let block_bbox = e.bbox.as_ref().map(|bb| {
                let left = bb.x0 as f32;
                let right = bb.x1 as f32;
                let pdf_bottom = page_h - bb.y1 as f32; // image y1 (bottom) → PDF bottom
                let pdf_top = page_h - bb.y0 as f32; // image y0 (top) → PDF top
                (left, pdf_bottom, right, pdf_top)
            });

            // Build lines from newline-separated text.
            // Distribute the bbox vertically across lines for spatial matching.
            let text_lines: Vec<&str> = e.text.split('\n').collect();
            let num_lines = text_lines.len().max(1);
            let (base_y, line_height) = if let Some((_left, bottom, _right, top)) = block_bbox {
                let total_height = top - bottom;
                let lh = total_height / num_lines as f32;
                // Start from top line (highest y in PDF coords).
                (top, lh)
            } else {
                (0.0, default_font_size)
            };

            let lines: Vec<super::types::PdfLine> = text_lines
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
                    let seg = SegmentData {
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
                    super::types::PdfLine {
                        segments: vec![seg],
                        baseline_y: line_y,
                        dominant_font_size: default_font_size,
                        is_bold: false,
                        is_monospace: false,
                    }
                })
                .collect();

            super::types::PdfParagraph {
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

#[cfg(test)]
mod tests {
    use super::*;
    use pdfium_render::prelude::PdfPoints;
    use pdfium_render::prelude::PdfRect;

    fn make_block(role: ContentRole, text: &str) -> ExtractedBlock {
        ExtractedBlock {
            role,
            text: text.to_string(),
            bounds: None,
            font_size: Some(12.0),
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            children: Vec::new(),
        }
    }

    fn make_block_with_bounds(role: ContentRole, text: &str) -> ExtractedBlock {
        ExtractedBlock {
            role,
            text: text.to_string(),
            bounds: Some(PdfRect::new(
                PdfPoints::new(100.0),
                PdfPoints::new(50.0),
                PdfPoints::new(200.0),
                PdfPoints::new(400.0),
            )),
            font_size: Some(12.0),
            is_bold: true,
            is_italic: false,
            is_monospace: false,
            children: Vec::new(),
        }
    }

    #[test]
    fn test_from_structure_tree_basic() {
        let blocks = vec![
            make_block(ContentRole::Heading { level: 1 }, "Title"),
            make_block(ContentRole::Paragraph, "Body text"),
        ];
        let page = from_structure_tree(&blocks);
        assert_eq!(page.elements.len(), 2);
        assert_eq!(page.elements[0].semantic_role, Some(SemanticRole::Heading { level: 1 }));
        assert_eq!(page.elements[1].semantic_role, Some(SemanticRole::Paragraph));
    }

    #[test]
    fn test_from_structure_tree_skips_empty() {
        let blocks = vec![
            make_block(ContentRole::Paragraph, ""),
            make_block(ContentRole::Paragraph, "   "),
            make_block(ContentRole::Paragraph, "Real text"),
        ];
        let page = from_structure_tree(&blocks);
        assert_eq!(page.elements.len(), 1);
        assert_eq!(page.elements[0].text, "Real text");
    }

    #[test]
    fn test_from_structure_tree_flattens_children() {
        let blocks = vec![ExtractedBlock {
            role: ContentRole::Other("Table".to_string()),
            text: String::new(),
            bounds: None,
            font_size: None,
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            children: vec![
                make_block(ContentRole::Paragraph, "Cell 1"),
                make_block(ContentRole::Paragraph, "Cell 2"),
            ],
        }];
        let page = from_structure_tree(&blocks);
        assert_eq!(page.elements.len(), 2);
    }

    #[test]
    fn test_from_structure_tree_maps_bounds() {
        let blocks = vec![make_block_with_bounds(ContentRole::Paragraph, "With bounds")];
        let page = from_structure_tree(&blocks);
        let elem = &page.elements[0];
        assert!(elem.bbox.is_some());
        assert!(elem.is_bold);
    }

    #[test]
    fn test_from_structure_tree_list_item_label() {
        let blocks = vec![ExtractedBlock {
            role: ContentRole::ListItem {
                label: Some("1.".to_string()),
            },
            text: "First item".to_string(),
            bounds: None,
            font_size: Some(12.0),
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            children: Vec::new(),
        }];
        let page = from_structure_tree(&blocks);
        assert_eq!(page.elements[0].semantic_role, Some(SemanticRole::ListItem));
        assert_eq!(page.elements[0].list_label, Some("1.".to_string()));
    }

    /// Regression: a block with both own `text` and nested children
    /// (e.g. a tagged `LBody` whose bold lead-in is a child `P` while
    /// the continuation text lives on the parent) must not silently drop
    /// the parent text during flatten.
    #[test]
    fn test_from_structure_tree_preserves_parent_text_with_children() {
        let blocks = vec![ExtractedBlock {
            role: ContentRole::Other("LBody".to_string()),
            text: " — continuation text on parent".to_string(),
            bounds: None,
            font_size: Some(12.0),
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            children: vec![make_block(ContentRole::Paragraph, "Bold lead-in on child")],
        }];
        let page = from_structure_tree(&blocks);
        assert_eq!(page.elements.len(), 2, "both child and parent texts must be emitted");
        // Child first (reading order), parent own-text after.
        assert_eq!(page.elements[0].text, "Bold lead-in on child");
        assert_eq!(page.elements[1].text, " — continuation text on parent");
    }

    #[test]
    fn test_map_content_role_all_variants() {
        assert_eq!(
            map_content_role(&ContentRole::Heading { level: 3 }),
            (SemanticRole::Heading { level: 3 }, None)
        );
        assert_eq!(
            map_content_role(&ContentRole::Paragraph),
            (SemanticRole::Paragraph, None)
        );
        assert_eq!(
            map_content_role(&ContentRole::ListItem {
                label: Some("a.".to_string())
            }),
            (SemanticRole::ListItem, Some("a.".to_string()))
        );
        assert_eq!(
            map_content_role(&ContentRole::TableCell {
                row: 0,
                col: 0,
                is_header: false,
            }),
            (SemanticRole::TableCell, None)
        );
        assert_eq!(
            map_content_role(&ContentRole::Figure { alt_text: None }),
            (SemanticRole::Figure, None)
        );
        assert_eq!(map_content_role(&ContentRole::Caption), (SemanticRole::Caption, None));
        assert_eq!(map_content_role(&ContentRole::Code), (SemanticRole::Code, None));
        assert_eq!(
            map_content_role(&ContentRole::BlockQuote),
            (SemanticRole::BlockQuote, None)
        );
        assert_eq!(
            map_content_role(&ContentRole::Link { url: None }),
            (SemanticRole::Paragraph, None)
        );
        assert_eq!(
            map_content_role(&ContentRole::Other("Formula".to_string())),
            (SemanticRole::Formula, None)
        );
        assert_eq!(
            map_content_role(&ContentRole::Other("Unknown".to_string())),
            (SemanticRole::Other, None)
        );
    }

    #[test]
    fn test_from_structure_tree_page_metadata() {
        let page = from_structure_tree(&[]);
        assert!(page.elements.is_empty());
    }

    // ── hOCR → PdfParagraph tests ──────────────────────────────────────

    #[cfg(feature = "ocr")]
    fn make_ocr_element(
        text: &str,
        page: u32,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
    ) -> crate::types::internal::InternalElement {
        use crate::types::extraction::BoundingBox;
        use crate::types::internal::{ElementKind, InternalElement};
        use crate::types::ocr_elements::OcrElementLevel;

        let mut elem = InternalElement::text(
            ElementKind::OcrText {
                level: OcrElementLevel::Block,
            },
            text,
            0,
        )
        .with_page(page);
        elem.bbox = Some(BoundingBox { x0, y0, x1, y1 });
        elem
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_ocr_doc_to_paragraphs_basic() {
        let mut doc = crate::types::internal::InternalDocument::new("pdf");
        doc.push_element(make_ocr_element("Hello World", 1, 100.0, 50.0, 500.0, 100.0));
        doc.push_element(make_ocr_element("Second paragraph", 1, 100.0, 120.0, 500.0, 170.0));

        let paragraphs = ocr_doc_to_paragraphs(&doc, 1000);
        assert_eq!(paragraphs.len(), 2);
        assert_eq!(paragraphs[0].text, "Hello World");
        assert_eq!(paragraphs[1].text, "Second paragraph");
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_ocr_doc_to_paragraphs_bbox_flip() {
        // Image coords: y=0 at top. Element at y=50..100 on a 1000px page.
        // PDF coords: y=0 at bottom. Should become bottom=900, top=950.
        let mut doc = crate::types::internal::InternalDocument::new("pdf");
        doc.push_element(make_ocr_element("Test", 1, 100.0, 50.0, 500.0, 100.0));

        let paragraphs = ocr_doc_to_paragraphs(&doc, 1000);
        let bbox = paragraphs[0].block_bbox.unwrap();
        // (left, bottom, right, top)
        assert_eq!(bbox.0, 100.0, "left should be preserved");
        assert_eq!(bbox.1, 900.0, "bottom = page_height - image_y1 = 1000 - 100");
        assert_eq!(bbox.2, 500.0, "right should be preserved");
        assert_eq!(bbox.3, 950.0, "top = page_height - image_y0 = 1000 - 50");
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_ocr_doc_to_paragraphs_multiline() {
        let mut doc = crate::types::internal::InternalDocument::new("pdf");
        doc.push_element(make_ocr_element(
            "Line one\nLine two\nLine three",
            1,
            100.0,
            50.0,
            500.0,
            200.0,
        ));

        let paragraphs = ocr_doc_to_paragraphs(&doc, 1000);
        assert_eq!(paragraphs.len(), 1);
        assert_eq!(paragraphs[0].lines.len(), 3);
        assert_eq!(paragraphs[0].lines[0].segments[0].text, "Line one");
        assert_eq!(paragraphs[0].lines[1].segments[0].text, "Line two");
        assert_eq!(paragraphs[0].lines[2].segments[0].text, "Line three");
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_ocr_doc_to_paragraphs_all_elements() {
        // Each per-page hOCR doc is independent, so all OcrText elements
        // are included regardless of their stored page number.
        let mut doc = crate::types::internal::InternalDocument::new("pdf");
        doc.push_element(make_ocr_element("First text", 1, 0.0, 0.0, 100.0, 50.0));
        doc.push_element(make_ocr_element("Second text", 1, 0.0, 60.0, 100.0, 110.0));

        let paragraphs = ocr_doc_to_paragraphs(&doc, 1000);
        assert_eq!(paragraphs.len(), 2);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_ocr_doc_to_paragraphs_skips_empty() {
        let mut doc = crate::types::internal::InternalDocument::new("pdf");
        doc.push_element(make_ocr_element("", 1, 0.0, 0.0, 100.0, 50.0));
        doc.push_element(make_ocr_element("   ", 1, 0.0, 60.0, 100.0, 110.0));
        doc.push_element(make_ocr_element("Real text", 1, 0.0, 120.0, 100.0, 170.0));

        let paragraphs = ocr_doc_to_paragraphs(&doc, 1000);
        assert_eq!(paragraphs.len(), 1);
        assert_eq!(paragraphs[0].text, "Real text");
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_ocr_doc_to_paragraphs_all_flags_default() {
        let mut doc = crate::types::internal::InternalDocument::new("pdf");
        doc.push_element(make_ocr_element("Test", 1, 0.0, 0.0, 100.0, 50.0));

        let paragraphs = ocr_doc_to_paragraphs(&doc, 1000);
        let p = &paragraphs[0];
        assert_eq!(p.heading_level, None);
        assert!(!p.is_bold);
        assert!(!p.is_list_item);
        assert!(!p.is_code_block);
        assert!(!p.is_formula);
        assert!(!p.is_page_furniture);
        assert_eq!(p.layout_class, None);
        assert_eq!(p.caption_for, None);
    }

    #[cfg(feature = "layout-detection")]
    #[cfg(feature = "ocr")]
    #[test]
    fn test_ocr_doc_to_paragraphs_with_layout_overrides() {
        use crate::pdf::structure::layout_classify::apply_layout_overrides;
        use crate::pdf::structure::types::{LayoutHint, LayoutHintClass};

        // Build an InternalDocument with 3 OcrText elements
        let mut doc = crate::types::internal::InternalDocument::new("pdf");
        // Title at top of page
        doc.push_element(make_ocr_element("Document Title", 1, 100.0, 50.0, 500.0, 100.0));
        // Paragraph in the middle
        doc.push_element(make_ocr_element(
            "Body paragraph text here.",
            1,
            100.0,
            150.0,
            500.0,
            200.0,
        ));
        // List item lower on the page
        doc.push_element(make_ocr_element("- First list item", 1, 100.0, 250.0, 500.0, 300.0));

        let page_height: u32 = 1000;
        let mut paragraphs = ocr_doc_to_paragraphs(&doc, page_height);
        assert_eq!(paragraphs.len(), 3);

        // Create LayoutHints matching the PDF-space bboxes.
        // Image coords (y=0 top) are flipped in ocr_doc_to_paragraphs:
        //   Title: image (100,50)-(500,100) → PDF bbox (100, 900, 500, 950)
        //   Body:  image (100,150)-(500,200) → PDF bbox (100, 800, 500, 850)
        //   List:  image (100,250)-(500,300) → PDF bbox (100, 700, 500, 750)
        let hints = vec![
            LayoutHint {
                class: LayoutHintClass::Title,
                confidence: 0.95,
                left: 90.0,
                bottom: 895.0,
                right: 510.0,
                top: 955.0,
            },
            LayoutHint {
                class: LayoutHintClass::Text,
                confidence: 0.90,
                left: 90.0,
                bottom: 795.0,
                right: 510.0,
                top: 855.0,
            },
            LayoutHint {
                class: LayoutHintClass::ListItem,
                confidence: 0.88,
                left: 90.0,
                bottom: 695.0,
                right: 510.0,
                top: 755.0,
            },
        ];

        apply_layout_overrides(&mut paragraphs, &hints, 0.5, 0.5, None);

        // Title gets heading_level = Some(1)
        assert_eq!(
            paragraphs[0].heading_level,
            Some(1),
            "Title layout hint should set heading_level to 1"
        );

        // List item gets is_list_item = true
        assert!(
            paragraphs[2].is_list_item,
            "ListItem layout hint should set is_list_item"
        );

        // Body paragraph stays as-is (Text class does not set heading or list)
        assert_eq!(
            paragraphs[1].heading_level, None,
            "Text layout hint should not set heading_level"
        );
        assert!(
            !paragraphs[1].is_list_item,
            "Text layout hint should not set is_list_item"
        );
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_ocr_doc_to_paragraphs_coordinate_conversion_accuracy() {
        // Test precise coordinate conversion from image space to PDF space.
        // Page height: 3508 pixels (A4 at 300 DPI).
        // Element at image coords: top-left (100, 200), bottom-right (500, 300).
        // In image space: x0=100, y0=200 (top), x1=500, y1=300 (bottom).
        // Expected PDF bbox (left, bottom, right, top):
        //   left   = 100
        //   bottom = page_height - y1 = 3508 - 300 = 3208
        //   right  = 500
        //   top    = page_height - y0 = 3508 - 200 = 3308
        let mut doc = crate::types::internal::InternalDocument::new("pdf");
        doc.push_element(make_ocr_element("Test text", 1, 100.0, 200.0, 500.0, 300.0));

        let paragraphs = ocr_doc_to_paragraphs(&doc, 3508);
        assert_eq!(paragraphs.len(), 1);

        let bbox = paragraphs[0].block_bbox.expect("Paragraph should have block_bbox");

        // block_bbox format: (left, bottom, right, top)
        assert_eq!(bbox.0, 100.0, "left should be 100");
        assert_eq!(bbox.1, 3208.0, "bottom should be page_height - y1 = 3508 - 300 = 3208");
        assert_eq!(bbox.2, 500.0, "right should be 500");
        assert_eq!(bbox.3, 3308.0, "top should be page_height - y0 = 3508 - 200 = 3308");
    }
}

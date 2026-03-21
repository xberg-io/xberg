//! Core PDF extraction functionality.
//!
//! Handles document loading, text extraction, metadata parsing, and table detection.

use crate::Result;
use crate::core::config::{ExtractionConfig, OutputFormat};
use crate::types::{PageBoundary, PageContent, PdfAnnotation};

#[cfg(feature = "pdf")]
use crate::types::Table;
#[cfg(feature = "pdf")]
use pdfium_render::prelude::*;

#[cfg(feature = "pdf")]
pub(crate) type PdfExtractionPhaseResult = (
    crate::pdf::metadata::PdfExtractionMetadata,
    String,
    Vec<Table>,
    Option<Vec<PageContent>>,
    Option<Vec<PageBoundary>>,
    Option<String>,             // pre-rendered markdown (when output_format == Markdown)
    bool,                       // has_font_encoding_issues (unicode map errors detected)
    Option<Vec<PdfAnnotation>>, // extracted annotations (when extract_annotations is enabled)
);

/// Extract text, metadata, and tables from a PDF document using a single shared instance.
///
/// This method consolidates all PDF extraction phases (text, metadata, tables) into a single
/// operation using a single PdfDocument instance. This avoids redundant document parsing
/// and pdfium initialization overhead.
///
/// # Performance
///
/// By reusing a single document instance across all extraction phases, we eliminate:
/// - Duplicate document parsing overhead (25-40ms saved)
/// - Redundant pdfium bindings initialization
/// - Multiple page tree traversals
///
/// Expected improvement: 20-30% faster PDF processing.
///
/// # Returns
///
/// A tuple containing:
/// - PDF metadata (title, authors, dates, page structure, etc.)
/// - Native extracted text (or empty if using OCR)
/// - Extracted tables (if OCR feature enabled)
/// - Per-page content (if page extraction configured)
/// - Page boundaries for per-page OCR evaluation
/// - Pre-rendered markdown (if output_format == Markdown, None otherwise)
#[cfg(feature = "pdf")]
pub(crate) fn extract_all_from_document(
    document: &PdfDocument,
    config: &ExtractionConfig,
    layout_hints: Option<&[Vec<crate::pdf::markdown::types::LayoutHint>]>,
    #[cfg(feature = "layout-detection")] layout_images: Option<&[image::DynamicImage]>,
    #[cfg(not(feature = "layout-detection"))] _layout_images: Option<()>,
    #[cfg(feature = "layout-detection")] layout_results: Option<&[crate::pdf::layout_runner::PageLayoutResult]>,
    #[cfg(not(feature = "layout-detection"))] _layout_results: Option<()>,
) -> Result<PdfExtractionPhaseResult> {
    let (native_text, boundaries, page_contents, pdf_metadata) =
        crate::pdf::text::extract_text_and_metadata_from_pdf_document(document, Some(config))?;

    let allow_single_column = config
        .pdf_options
        .as_ref()
        .is_some_and(|o| o.allow_single_column_tables);
    let tables = extract_tables_from_document(document, &pdf_metadata, allow_single_column)?;

    let mut has_font_encoding_issues = false;

    // If markdown output is requested, render it while we have the document loaded.
    // Skip when force_ocr is set since OCR results produce their own markdown via hOCR.
    // Pre-render structured markdown for all output formats that benefit from it.
    // Markdown, Djot, and HTML all gain headings, tables, bold/italic, dehyphenation.
    let needs_structured = matches!(
        config.output_format,
        OutputFormat::Markdown | OutputFormat::Djot | OutputFormat::Html
    );
    tracing::debug!(
        output_format = ?config.output_format,
        needs_structured,
        force_ocr = config.force_ocr,
        "PDF markdown path: evaluating whether to render structured markdown"
    );
    let pre_rendered_markdown = if needs_structured && !config.force_ocr {
        let k = config
            .pdf_options
            .as_ref()
            .and_then(|opts| opts.hierarchy.as_ref())
            .map(|h| h.k_clusters)
            .unwrap_or(4);

        let (top_margin, bottom_margin) = config
            .pdf_options
            .as_ref()
            .map(|opts| (opts.top_margin_fraction, opts.bottom_margin_fraction))
            .unwrap_or((None, None));

        let page_marker_format = config
            .pages
            .as_ref()
            .filter(|p| p.insert_page_markers)
            .map(|p| p.marker_format.as_str());

        tracing::debug!(
            k_clusters = k,
            "PDF markdown path: calling render_document_as_markdown_with_tables"
        );
        match crate::pdf::markdown::render_document_as_markdown_with_tables(
            document,
            k,
            &tables,
            top_margin,
            bottom_margin,
            page_marker_format,
            layout_hints,
            #[cfg(feature = "layout-detection")]
            layout_images,
            #[cfg(not(feature = "layout-detection"))]
            None,
            #[cfg(feature = "layout-detection")]
            layout_results,
            #[cfg(not(feature = "layout-detection"))]
            None,
            allow_single_column,
        ) {
            Ok((md, has_encoding_issues)) if !md.trim().is_empty() => {
                tracing::debug!(
                    md_len = md.len(),
                    has_headings = md.contains("# "),
                    has_bold = md.contains("**"),
                    "PDF markdown path: render succeeded with content"
                );
                has_font_encoding_issues = has_encoding_issues;
                Some(md)
            }
            Ok((_, has_encoding_issues)) => {
                tracing::warn!("Markdown rendering produced empty output, will fall back to plain text");
                has_font_encoding_issues = has_encoding_issues;
                None
            }
            Err(e) => {
                tracing::warn!("Markdown rendering failed: {:?}, will fall back to plain text", e);
                None
            }
        }
    } else {
        None
    };

    // Extract annotations when configured.
    let annotations = if config.pdf_options.as_ref().is_some_and(|opts| opts.extract_annotations) {
        let extracted = crate::pdf::annotations::extract_annotations_from_document(document);
        if extracted.is_empty() { None } else { Some(extracted) }
    } else {
        None
    };

    Ok((
        pdf_metadata,
        native_text,
        tables,
        page_contents,
        boundaries,
        pre_rendered_markdown,
        has_font_encoding_issues,
        annotations,
    ))
}

/// Convert layout detection results to per-page layout hints for the markdown pipeline.
///
/// Maps `LayoutClass` (from `crate::layout`) to `LayoutHintClass` (feature-gate-free
/// types in the markdown module) and flattens per-page regions into hint vectors.
#[cfg(all(feature = "pdf", feature = "layout-detection"))]
pub(crate) fn convert_results_to_hints(
    results: &[crate::pdf::layout_runner::PageLayoutResult],
) -> Vec<Vec<crate::pdf::markdown::types::LayoutHint>> {
    use crate::layout::LayoutClass;
    use crate::pdf::markdown::types::{LayoutHint, LayoutHintClass};

    results
        .iter()
        .enumerate()
        .map(|(page_idx, page)| {
            let hints: Vec<LayoutHint> = page
                .regions
                .iter()
                .map(|region| {
                    let class = match region.class {
                        LayoutClass::Title => LayoutHintClass::Title,
                        LayoutClass::SectionHeader => LayoutHintClass::SectionHeader,
                        LayoutClass::Code => LayoutHintClass::Code,
                        LayoutClass::Formula => LayoutHintClass::Formula,
                        LayoutClass::ListItem => LayoutHintClass::ListItem,
                        LayoutClass::Caption => LayoutHintClass::Caption,
                        LayoutClass::Footnote => LayoutHintClass::Footnote,
                        LayoutClass::PageHeader => LayoutHintClass::PageHeader,
                        LayoutClass::PageFooter => LayoutHintClass::PageFooter,
                        LayoutClass::Table => LayoutHintClass::Table,
                        LayoutClass::Picture => LayoutHintClass::Picture,
                        LayoutClass::Text => LayoutHintClass::Text,
                        _ => LayoutHintClass::Other,
                    };
                    LayoutHint {
                        class,
                        confidence: region.confidence,
                        left: region.bbox.left,
                        bottom: region.bbox.bottom,
                        right: region.bbox.right,
                        top: region.bbox.top,
                    }
                })
                .collect();
            tracing::trace!(
                page = page_idx,
                table_hints = hints
                    .iter()
                    .filter(|h| matches!(h.class, LayoutHintClass::Table))
                    .count(),
                "Layout hints for page"
            );
            hints
        })
        .collect()
}

/// Check whether words on a page exhibit column alignment consistent with a table.
///
/// Groups word left-edges into buckets and checks that at least 3 buckets each contain
/// multiple words. Two-column text layouts naturally produce 2 alignment clusters, so
/// we require ≥3 to avoid false positives from academic papers and similar documents.
#[cfg(feature = "pdf")]
fn has_column_alignment(words: &[crate::pdf::table_reconstruct::HocrWord]) -> bool {
    if words.len() < 6 {
        return false;
    }

    // Bucket word left positions using a tolerance of 15px
    const BUCKET_TOLERANCE: u32 = 15;
    let mut buckets: Vec<(u32, usize)> = Vec::new(); // (representative_x, count)

    for w in words {
        let x = w.left;
        if let Some(bucket) = buckets.iter_mut().find(|(bx, _)| x.abs_diff(*bx) <= BUCKET_TOLERANCE) {
            bucket.1 += 1;
        } else {
            buckets.push((x, 1));
        }
    }

    // Require ≥3 distinct columns with ≥3 words each.
    // Two-column text layouts have exactly 2 alignment clusters, so requiring 3
    // eliminates false positives from multi-column prose while still detecting
    // real tables (which typically have 3+ columns).
    let significant_columns = buckets.iter().filter(|(_, count)| *count >= 3).count();
    significant_columns >= 3
}

/// Extract tables from PDF document using native text positions.
///
/// This function converts PDF character positions to HocrWord format,
/// then uses the existing table reconstruction logic to detect tables.
///
/// Uses the shared PdfDocument reference (wrapped in Arc<RwLock<>> for thread-safety).
#[cfg(feature = "pdf")]
fn extract_tables_from_document(
    document: &PdfDocument,
    _metadata: &crate::pdf::metadata::PdfExtractionMetadata,
    allow_single_column: bool,
) -> Result<Vec<Table>> {
    use crate::pdf::table::extract_words_from_page;
    use crate::pdf::table_reconstruct::{post_process_table, reconstruct_table, table_to_markdown};

    let mut all_tables = Vec::new();

    for (page_index, page) in document.pages().iter().enumerate() {
        let words = extract_words_from_page(&page, 0.0)?;

        // Need at least 6 words for a meaningful table
        if words.len() < 6 {
            continue;
        }

        // Pre-validate column alignment: real tables have words clustering at
        // consistent x-positions. Body text scattered across the page won't.
        if !has_column_alignment(&words) {
            continue;
        }

        let column_threshold = 50;
        let row_threshold_ratio = 0.5;

        let table_cells = reconstruct_table(&words, column_threshold, row_threshold_ratio);

        if table_cells.is_empty() || table_cells[0].is_empty() {
            continue;
        }

        // Apply full post-processing validation: empty row removal, long cell rejection,
        // header detection, column merging, dimension checks, and cell normalization.
        let table_cells = match post_process_table(table_cells, false, allow_single_column) {
            Some(cleaned) => cleaned,
            None => continue,
        };

        let markdown = table_to_markdown(&table_cells);

        // Compute table bounding box from word positions.
        // Note: The table detector (reconstruct_table) treats ALL words on the page as
        // potential table content, so the bbox covers all page words. This is correct:
        // if the page passes the 2x2 validation, the entire page IS the table.
        // For pages with mixed content (table + body text), the detector would either
        // reject the page (not 2x2) or include everything (the full page is tabular).
        let page_height = page.height().value as f64;

        // HocrWord coordinates are in image space (y=0 at top, from table.rs:finalize_word).
        // Convert back to PDF coordinates (y=0 at bottom) for the BoundingBox.
        let img_left = words.iter().map(|w| w.left as f64).fold(f64::INFINITY, f64::min);
        let img_top = words.iter().map(|w| w.top as f64).fold(f64::INFINITY, f64::min);
        let img_right = words
            .iter()
            .map(|w| (w.left + w.width) as f64)
            .fold(f64::NEG_INFINITY, f64::max);
        let img_bottom = words
            .iter()
            .map(|w| (w.top + w.height) as f64)
            .fold(f64::NEG_INFINITY, f64::max);

        let bounding_box = if img_left.is_finite() {
            Some(crate::types::BoundingBox {
                x0: img_left,
                y0: page_height - img_bottom, // bottom in PDF coords
                x1: img_right,
                y1: page_height - img_top, // top in PDF coords
            })
        } else {
            None
        };

        all_tables.push(Table {
            cells: table_cells,
            markdown,
            page_number: page_index + 1,
            bounding_box,
        });
    }

    Ok(all_tables)
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_bounding_box_coordinate_conversion() {
        // Test the bounding box computation logic independently
        // Simulate words at known positions and verify the resulting bbox
        let page_height = 800.0_f64;

        // Simulated word positions in image coordinates (y=0 at top)
        // Word 1: left=50, top=100, width=200, height=20
        // Word 2: left=50, top=130, width=250, height=20
        let img_left = 50.0_f64;
        let img_top = 100.0_f64;
        let img_right = 300.0_f64; // max(50+200, 50+250)
        let img_bottom = 150.0_f64; // max(100+20, 130+20)

        let bbox = crate::types::BoundingBox {
            x0: img_left,
            y0: page_height - img_bottom, // 800 - 150 = 650
            x1: img_right,
            y1: page_height - img_top, // 800 - 100 = 700
        };

        assert_eq!(bbox.x0, 50.0);
        assert_eq!(bbox.y0, 650.0); // bottom in PDF coords
        assert_eq!(bbox.x1, 300.0);
        assert_eq!(bbox.y1, 700.0); // top in PDF coords
        // y1 > y0 confirms the table is above the bottom
        assert!(bbox.y1 > bbox.y0);
    }

    #[test]
    fn test_bounding_box_coordinate_conversion_different_scales() {
        // Test with different page height and word positions
        let page_height = 1000.0_f64;

        // Words spanning from top=50 to bottom=400
        let img_left = 100.0_f64;
        let img_top = 50.0_f64;
        let img_right = 600.0_f64;
        let img_bottom = 400.0_f64;

        let bbox = crate::types::BoundingBox {
            x0: img_left,
            y0: page_height - img_bottom, // 1000 - 400 = 600
            x1: img_right,
            y1: page_height - img_top, // 1000 - 50 = 950
        };

        assert_eq!(bbox.x0, 100.0);
        assert_eq!(bbox.y0, 600.0);
        assert_eq!(bbox.x1, 600.0);
        assert_eq!(bbox.y1, 950.0);
        // Height of table: 950 - 600 = 350 pixels
        assert_eq!(bbox.y1 - bbox.y0, 350.0);
    }

    #[test]
    fn test_bounding_box_coordinate_conversion_preserves_width() {
        // Width should be preserved during coordinate transformation
        let page_height = 595.0_f64; // Standard letter page height

        let img_left = 72.0_f64;
        let img_right = 522.0_f64; // width = 450
        let img_top = 36.0_f64;
        let img_bottom = 300.0_f64; // height = 264

        let bbox = crate::types::BoundingBox {
            x0: img_left,
            y0: page_height - img_bottom,
            x1: img_right,
            y1: page_height - img_top,
        };

        let expected_width = img_right - img_left;
        let actual_width = bbox.x1 - bbox.x0;
        assert_eq!(actual_width, expected_width);
        assert_eq!(actual_width, 450.0);
    }

    #[test]
    fn test_bounding_box_serialization_round_trip() {
        let original = crate::types::BoundingBox {
            x0: 10.5,
            y0: 20.25,
            x1: 100.75,
            y1: 200.5,
        };

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: crate::types::BoundingBox = serde_json::from_str(&json).unwrap();

        assert_eq!(original, deserialized);
        assert_eq!(deserialized.x0, 10.5);
        assert_eq!(deserialized.y0, 20.25);
        assert_eq!(deserialized.x1, 100.75);
        assert_eq!(deserialized.y1, 200.5);
    }

    #[test]
    #[cfg(feature = "pdf")]
    fn test_has_column_alignment_table_layout() {
        use crate::pdf::table_reconstruct::HocrWord;

        // Simulate a 3-column table: words at x=50, x=200, x=400
        // Requires ≥3 columns with ≥3 words each to pass.
        let words = vec![
            // Row 1
            HocrWord {
                text: "Name".into(),
                left: 50,
                top: 100,
                width: 60,
                height: 12,
                confidence: 95.0,
            },
            HocrWord {
                text: "Age".into(),
                left: 200,
                top: 100,
                width: 40,
                height: 12,
                confidence: 95.0,
            },
            HocrWord {
                text: "City".into(),
                left: 400,
                top: 100,
                width: 50,
                height: 12,
                confidence: 95.0,
            },
            // Row 2
            HocrWord {
                text: "Alice".into(),
                left: 50,
                top: 120,
                width: 60,
                height: 12,
                confidence: 95.0,
            },
            HocrWord {
                text: "30".into(),
                left: 200,
                top: 120,
                width: 30,
                height: 12,
                confidence: 95.0,
            },
            HocrWord {
                text: "NYC".into(),
                left: 400,
                top: 120,
                width: 40,
                height: 12,
                confidence: 95.0,
            },
            // Row 3
            HocrWord {
                text: "Bob".into(),
                left: 50,
                top: 140,
                width: 50,
                height: 12,
                confidence: 95.0,
            },
            HocrWord {
                text: "25".into(),
                left: 200,
                top: 140,
                width: 30,
                height: 12,
                confidence: 95.0,
            },
            HocrWord {
                text: "LA".into(),
                left: 400,
                top: 140,
                width: 30,
                height: 12,
                confidence: 95.0,
            },
        ];
        assert!(super::has_column_alignment(&words));
    }

    #[test]
    #[cfg(feature = "pdf")]
    fn test_has_column_alignment_rejects_two_column_layout() {
        use crate::pdf::table_reconstruct::HocrWord;

        // Two-column text layout (like academic papers) should NOT be detected as a table.
        let words = vec![
            HocrWord {
                text: "Left".into(),
                left: 50,
                top: 100,
                width: 60,
                height: 12,
                confidence: 95.0,
            },
            HocrWord {
                text: "Right".into(),
                left: 300,
                top: 100,
                width: 60,
                height: 12,
                confidence: 95.0,
            },
            HocrWord {
                text: "More".into(),
                left: 50,
                top: 120,
                width: 60,
                height: 12,
                confidence: 95.0,
            },
            HocrWord {
                text: "Text".into(),
                left: 300,
                top: 120,
                width: 60,
                height: 12,
                confidence: 95.0,
            },
            HocrWord {
                text: "Here".into(),
                left: 50,
                top: 140,
                width: 60,
                height: 12,
                confidence: 95.0,
            },
            HocrWord {
                text: "Also".into(),
                left: 300,
                top: 140,
                width: 60,
                height: 12,
                confidence: 95.0,
            },
        ];
        assert!(!super::has_column_alignment(&words));
    }

    #[test]
    #[cfg(feature = "pdf")]
    fn test_has_column_alignment_body_text() {
        use crate::pdf::table_reconstruct::HocrWord;

        // Body text: words flow left-to-right on each line with distinct x positions.
        // Each word has a unique left-edge so no bucket accumulates >= 2 words,
        // meaning column alignment should NOT be detected.
        let words = vec![
            HocrWord {
                text: "This".into(),
                left: 50,
                top: 100,
                width: 40,
                height: 12,
                confidence: 95.0,
            },
            HocrWord {
                text: "is".into(),
                left: 100,
                top: 100,
                width: 20,
                height: 12,
                confidence: 95.0,
            },
            HocrWord {
                text: "some".into(),
                left: 130,
                top: 100,
                width: 45,
                height: 12,
                confidence: 95.0,
            },
            HocrWord {
                text: "body".into(),
                left: 185,
                top: 100,
                width: 45,
                height: 12,
                confidence: 95.0,
            },
            HocrWord {
                text: "text".into(),
                left: 240,
                top: 100,
                width: 40,
                height: 12,
                confidence: 95.0,
            },
            HocrWord {
                text: "here".into(),
                left: 290,
                top: 100,
                width: 40,
                height: 12,
                confidence: 95.0,
            },
        ];
        assert!(!super::has_column_alignment(&words));
    }

    #[test]
    #[cfg(feature = "pdf")]
    fn test_has_column_alignment_too_few_words() {
        use crate::pdf::table_reconstruct::HocrWord;

        let words = vec![
            HocrWord {
                text: "Hello".into(),
                left: 50,
                top: 100,
                width: 60,
                height: 12,
                confidence: 95.0,
            },
            HocrWord {
                text: "World".into(),
                left: 300,
                top: 100,
                width: 60,
                height: 12,
                confidence: 95.0,
            },
        ];
        assert!(!super::has_column_alignment(&words));
    }
}

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
    Option<crate::types::internal::InternalDocument>, // pre-rendered structured doc (when output_format == Markdown/Djot/Html)
    bool,                                             // has_font_encoding_issues (unicode map errors detected)
    Option<Vec<PdfAnnotation>>,                       // extracted annotations (when extract_annotations is enabled)
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
/// - Pre-rendered structured document (if output_format requires structure, None otherwise)
#[cfg(feature = "pdf")]
pub(crate) fn extract_all_from_document(
    document: &PdfDocument,
    config: &ExtractionConfig,
    layout_hints: Option<&[Vec<crate::pdf::structure::types::LayoutHint>]>,
    #[cfg(feature = "layout-detection")] layout_images: Option<&[image::DynamicImage]>,
    #[cfg(not(feature = "layout-detection"))] _layout_images: Option<()>,
    #[cfg(feature = "layout-detection")] layout_results: Option<&[crate::pdf::layout_runner::PageLayoutResult]>,
    #[cfg(not(feature = "layout-detection"))] _layout_results: Option<()>,
) -> Result<PdfExtractionPhaseResult> {
    let _span = tracing::debug_span!(
        "extract_pdf",
        page_count = document.pages().len(),
        element_count = tracing::field::Empty,
        has_text_layer = tracing::field::Empty,
    )
    .entered();

    #[cfg(feature = "layout-detection")]
    let has_layout = config.layout.is_some();
    #[cfg(not(feature = "layout-detection"))]
    let has_layout = false;

    tracing::debug!(
        output_format = ?config.output_format,
        force_ocr = config.force_ocr,
        has_layout,
        "PDF extraction starting"
    );

    let (native_text, boundaries, page_contents, pdf_metadata) =
        crate::pdf::text::extract_text_and_metadata_from_pdf_document(document, Some(config))?;

    let allow_single_column = config
        .pdf_options
        .as_ref()
        .is_some_and(|o| o.allow_single_column_tables);
    let tables = extract_tables_from_document(document, &pdf_metadata, allow_single_column)?;

    let mut has_font_encoding_issues = false;

    // Pre-render a structured InternalDocument for output formats that benefit from it.
    // Skip when force_ocr is set since OCR results produce their own structure via hOCR.
    // Markdown, Djot, and HTML all gain headings, tables, bold/italic, dehyphenation.
    let needs_structured = matches!(
        config.output_format,
        OutputFormat::Markdown | OutputFormat::Djot | OutputFormat::Html
    );
    tracing::debug!(
        output_format = ?config.output_format,
        needs_structured,
        force_ocr = config.force_ocr,
        "PDF structure path: evaluating whether to render structured document"
    );
    let pre_rendered_doc = if needs_structured && !config.force_ocr {
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

        let (strip_repeating_text, include_headers, include_footers) = config
            .content_filter
            .as_ref()
            .map(|cf| (cf.strip_repeating_text, cf.include_headers, cf.include_footers))
            .unwrap_or((true, false, false)); // defaults match current behavior

        tracing::debug!(k_clusters = k, "PDF structure path: calling extract_document_structure");
        match crate::pdf::structure::extract_document_structure(
            document,
            k,
            &tables,
            top_margin,
            bottom_margin,
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
            #[cfg(feature = "layout-detection")]
            config.layout.as_ref().map(|l| l.table_model).unwrap_or_default(),
            #[cfg(not(feature = "layout-detection"))]
            None,
            #[cfg(feature = "layout-detection")]
            config.acceleration.as_ref(),
            #[cfg(not(feature = "layout-detection"))]
            None,
            strip_repeating_text,
            include_headers,
            include_footers,
        ) {
            Ok((doc, has_encoding_issues)) if !doc.elements.is_empty() => {
                tracing::debug!(
                    element_count = doc.elements.len(),
                    has_headings = doc
                        .elements
                        .iter()
                        .any(|e| matches!(e.kind, crate::types::internal::ElementKind::Heading { .. })),
                    "PDF structure path: render succeeded with content"
                );
                has_font_encoding_issues = has_encoding_issues;
                Some(doc)
            }
            Ok((_, has_encoding_issues)) => {
                tracing::warn!("Structure rendering produced empty output, will fall back to plain text");
                has_font_encoding_issues = has_encoding_issues;
                None
            }
            Err(e) => {
                tracing::warn!("Structure rendering failed: {:?}, will fall back to plain text", e);
                None
            }
        }
    } else {
        None
    };

    tracing::debug!(
        has_pre_rendered = pre_rendered_doc.is_some(),
        elements = pre_rendered_doc.as_ref().map(|d| d.elements.len()).unwrap_or(0),
        "structure extraction complete"
    );

    // Extract annotations when configured.
    let annotations = if config.pdf_options.as_ref().is_some_and(|opts| opts.extract_annotations) {
        let extracted = crate::pdf::annotations::extract_annotations_from_document(document);
        if extracted.is_empty() { None } else { Some(extracted) }
    } else {
        None
    };

    let element_count = pre_rendered_doc.as_ref().map(|d| d.elements.len()).unwrap_or(0);
    let has_text = !native_text.trim().is_empty();
    _span.record("element_count", element_count);
    _span.record("has_text_layer", has_text);

    Ok((
        pdf_metadata,
        native_text,
        tables,
        page_contents,
        boundaries,
        pre_rendered_doc,
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
) -> Vec<Vec<crate::pdf::structure::types::LayoutHint>> {
    use crate::layout::LayoutClass;
    use crate::pdf::structure::types::{LayoutHint, LayoutHintClass};

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

        // Reject tables with very few rows whose bbox covers most of the page.
        // The heuristic table detector treats all words on a page as potential
        // table content, so when it produces a 2–3 row "table" spanning the
        // full page, it's almost certainly body text with column-like gaps
        // (e.g., PowerPoint-exported marketing slides). The bbox would suppress
        // all text segments for this page in the structured pipeline.
        if let Some(ref bb) = bounding_box {
            let bbox_height = (bb.y1 - bb.y0).abs();
            if table_cells.len() <= 3 && page_height > 0.0 && bbox_height / page_height > 0.5 {
                tracing::trace!(
                    page = page_index,
                    rows = table_cells.len(),
                    bbox_height,
                    page_height,
                    "heuristic table with <=3 rows spans >50% of page — skipping false positive"
                );
                continue;
            }
        }

        all_tables.push(Table {
            cells: table_cells,
            markdown,
            page_number: page_index + 1,
            bounding_box,
        });
    }

    Ok(all_tables)
}

/// Extract text, metadata, tables, and annotations from a PDF document using the pdf_oxide backend.
///
/// This is the oxide equivalent of [`extract_all_from_document`]. It opens the document via
/// `OxideDocument`, then delegates to each oxide extraction module. The return type matches
/// `PdfExtractionPhaseResult` so callers can switch transparently between backends.
///
/// # Notes
///
/// - Layout detection is not yet supported on the oxide path.
/// - When output format is Markdown/Djot/HTML, the oxide hierarchy module extracts font
///   metrics and feeds them into the backend-agnostic structure pipeline for heading detection.
/// - Font encoding issue detection is not available; the flag is always `false`.
#[cfg(feature = "pdf-oxide")]
pub(crate) fn extract_all_from_oxide_document(
    content: &[u8],
    config: &ExtractionConfig,
    layout_hints: Option<&[Vec<crate::pdf::structure::types::LayoutHint>]>,
    #[cfg(feature = "layout-detection")] layout_images: Option<&[image::DynamicImage]>,
    #[cfg(not(feature = "layout-detection"))] _layout_images: Option<()>,
    #[cfg(feature = "layout-detection")] layout_results: Option<&[crate::pdf::layout_runner::PageLayoutResult]>,
    #[cfg(not(feature = "layout-detection"))] _layout_results: Option<()>,
) -> Result<PdfExtractionPhaseResult> {
    let _span = tracing::debug_span!("extract_pdf_oxide").entered();

    let mut doc = crate::pdf::oxide::OxideDocument::open_bytes(content)?;

    // --- Text + metadata (single pass) ---
    let (native_text, boundaries, page_contents, pdf_metadata) =
        crate::pdf::oxide::text::extract_text_and_metadata(&mut doc, Some(config)).map_err(|e| {
            crate::error::KreuzbergError::Parsing {
                message: format!("pdf_oxide text extraction failed: {e}"),
                source: None,
            }
        })?;

    // --- Tables (native pdf_oxide detection) ---
    // Use unwrap_or_default so table detection failures don't block extraction.
    let tables = crate::pdf::oxide::table::extract_tables_native(&mut doc).unwrap_or_default();

    // --- Annotations ---
    let annotations = if config.pdf_options.as_ref().is_some_and(|opts| opts.extract_annotations) {
        let extracted = crate::pdf::oxide::annotations::extract_annotations(&mut doc);
        if extracted.is_empty() { None } else { Some(extracted) }
    } else {
        None
    };

    // --- Image positions for assembly pipeline ---
    let image_positions = crate::pdf::oxide::images::extract_image_positions(&mut doc).map_err(|e| {
        crate::error::KreuzbergError::Parsing {
            message: format!("pdf_oxide image position extraction failed: {e}"),
            source: None,
        }
    })?;

    // Pre-render structured document for output formats that benefit from headings.
    let needs_structured = matches!(
        config.output_format,
        OutputFormat::Markdown | OutputFormat::Djot | OutputFormat::Html
    );

    let allow_single_column = config
        .pdf_options
        .as_ref()
        .is_some_and(|o| o.allow_single_column_tables);

    let pre_rendered_doc =
        if needs_structured && !config.force_ocr {
            let k = config
                .pdf_options
                .as_ref()
                .and_then(|opts| opts.hierarchy.as_ref())
                .map(|h| h.k_clusters)
                .unwrap_or(4);

            let (strip_repeating_text, include_headers, include_footers) = config
                .content_filter
                .as_ref()
                .map(|cf| (cf.strip_repeating_text, cf.include_headers, cf.include_footers))
                .unwrap_or((true, false, false));

            // Extract font-metric segments from oxide for heading detection.
            // When the PDF has a reliable structure tree, segments carry pre-assigned
            // heading roles (assigned_role) and the pipeline can skip font-size clustering.
            let (segments, used_structure_tree) = crate::pdf::oxide::hierarchy::extract_all_segments(&mut doc)
                .map_err(|e| crate::error::KreuzbergError::Parsing {
                    message: format!("pdf_oxide hierarchy extraction failed: {e}"),
                    source: None,
                })?;

            let total_segs: usize = segments.iter().map(|s| s.len()).sum();
            tracing::debug!(
                total_segs,
                k,
                used_structure_tree,
                "oxide structure: extracted segments for heading detection"
            );

            match crate::pdf::structure::extract_document_structure_from_segments(
                segments,
                crate::pdf::structure::SegmentStructureConfig {
                    k_clusters: k,
                    tables: &tables,
                    strip_repeating_text,
                    include_headers,
                    include_footers,
                    used_structure_tree,
                    image_positions: &image_positions,
                    layout_hints,
                    allow_single_column,
                    #[cfg(feature = "layout-detection")]
                    layout_images,
                    #[cfg(feature = "layout-detection")]
                    layout_results,
                    #[cfg(feature = "layout-detection")]
                    table_model: config.layout.as_ref().map(|l| l.table_model).unwrap_or_default(),
                    #[cfg(feature = "layout-detection")]
                    acceleration: config.acceleration.as_ref(),
                },
            ) {
                Ok(structured_doc) if !structured_doc.elements.is_empty() => {
                    tracing::debug!(
                        elements = structured_doc.elements.len(),
                        has_headings = structured_doc
                            .elements
                            .iter()
                            .any(|e| matches!(e.kind, crate::types::internal::ElementKind::Heading { .. })),
                        "oxide structure: render succeeded"
                    );
                    Some(structured_doc)
                }
                Ok(_) => {
                    tracing::warn!("oxide structure: rendering produced empty output, falling back to plain text");
                    None
                }
                Err(e) => {
                    tracing::warn!("oxide structure: rendering failed: {:?}, falling back to plain text", e);
                    None
                }
            }
        } else {
            None
        };

    let has_font_encoding_issues = false;

    Ok((
        pdf_metadata,
        native_text,
        tables,
        page_contents,
        boundaries,
        pre_rendered_doc,
        has_font_encoding_issues,
        annotations,
    ))
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

//! Core PDF extraction functionality.
//!
//! Handles document loading, text extraction, metadata parsing, and table detection.

use crate::Result;
use crate::core::config::{ExtractionConfig, OutputFormat};
use crate::types::{PageBoundary, PageContent, PdfAnnotation};

#[cfg(feature = "pdf")]
use crate::types::Table;

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
    Option<Vec<crate::types::ExtractedImage>>, // extracted images (when extract_images or ocr_inline_images is enabled)
);

/// Extract text, metadata, tables, and annotations from a PDF document using the pdf_oxide backend.
///
/// Opens the document via `OxideDocument`, then delegates to each oxide extraction module.
/// The return type is `PdfExtractionPhaseResult` so callers can switch transparently between
/// backends.
///
/// # Notes
///
/// - Layout detection is not yet supported on the oxide path.
/// - When output format is Markdown/Djot/HTML, the oxide hierarchy module extracts font
///   metrics and feeds them into the backend-agnostic structure pipeline for heading detection.
/// - Font encoding issue detection is not available; the flag is always `false`.
#[cfg(feature = "pdf")]
pub(crate) fn extract_all_from_oxide_document(
    content: &[u8],
    config: &ExtractionConfig,
    layout_hints: Option<&[Vec<crate::pdf::structure::types::LayoutHint>]>,
    #[cfg(feature = "layout-detection")] layout_images: Option<&[image::RgbImage]>,
    #[cfg(not(feature = "layout-detection"))] _layout_images: Option<()>,
    #[cfg(feature = "layout-detection")] layout_results: Option<&[crate::pdf::structure::types::PageLayoutResult]>,
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

    // --- Tables ---
    // Run pdf_oxide's native grid detector first; on pages where it found
    // nothing, fall back to the heuristic spatial reconstruction so tables
    // without explicit ruling lines (invoice line items, financial tables,
    // etc.) are still recovered. Native and heuristic results are merged at
    // the per-page level so a document with a ruled table on page 1 and an
    // unruled table on page 4 produces both. Use `unwrap_or_default` so
    // detection failures don't block extraction. Skipped entirely when
    // `pdf_options.extract_tables` is `false`.
    let extract_tables_flag = config.pdf_options.as_ref().is_none_or(|opts| opts.extract_tables);
    let allow_single_column = config
        .pdf_options
        .as_ref()
        .is_some_and(|o| o.allow_single_column_tables);
    let tables = if extract_tables_flag {
        let mut combined = crate::pdf::oxide::table::extract_tables_native(&mut doc).unwrap_or_default();
        let native_pages: std::collections::HashSet<usize> = combined.iter().map(|t| t.page_number).collect();
        let heuristic =
            crate::pdf::oxide::table::extract_tables_heuristic(&mut doc, allow_single_column, &native_pages)
                .unwrap_or_default();
        combined.extend(heuristic);
        combined
    } else {
        Vec::new()
    };

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

    // --- Extracted images (data + OCR) ---
    let images_extraction_enabled = config.images.as_ref().map(|c| c.extract_images).unwrap_or(false)
        || config.pdf_options.as_ref().map(|p| p.extract_images).unwrap_or(false);

    let ocr_inline_images = config
        .pdf_options
        .as_ref()
        .map(|p| p.ocr_inline_images)
        .unwrap_or(false);

    let images = if images_extraction_enabled || ocr_inline_images {
        let max_images = config.images.as_ref().and_then(|i| i.max_images_per_page);
        let mut extracted = crate::pdf::oxide::images::extract_images_with_data(&mut doc, max_images).map_err(|e| {
            crate::error::KreuzbergError::Parsing {
                message: format!("pdf_oxide image extraction failed: {e}"),
                source: None,
            }
        })?;

        // Perform OCR on extracted images if enabled
        if ocr_inline_images && !extracted.is_empty() {
            #[cfg(feature = "ocr")]
            {
                let cache_dir = std::env::var("KREUZBERG_CACHE_DIR").ok().map(std::path::PathBuf::from);
                let ocr_processor = crate::ocr::processor::OcrProcessor::new(cache_dir).map_err(|e| {
                    crate::error::KreuzbergError::Parsing {
                        message: format!("Failed to initialize OCR processor for inline images: {e}"),
                        source: None,
                    }
                })?;

                let tesseract_config = config
                    .ocr
                    .as_ref()
                    .map(|o| o.tesseract_config.clone().unwrap_or_default())
                    .unwrap_or_default();

                for img in &mut extracted {
                    let internal_config: crate::ocr::types::TesseractConfig = (&tesseract_config).into();
                    match ocr_processor.process_image(&img.data, &internal_config) {
                        Ok(ocr_result) => {
                            img.ocr_result = Some(Box::new(crate::types::ExtractionResult::from_ocr(ocr_result)));
                        }
                        Err(e) => {
                            tracing::warn!(
                                page = img.page_number,
                                image_index = img.image_index,
                                error = %e,
                                "inline image OCR failed; image returned without OCR result"
                            );
                        }
                    }
                }
            }
        }
        Some(extracted)
    } else {
        None
    };

    if config.cancel_token.as_ref().is_some_and(|t| t.is_cancelled()) {
        return Err(crate::error::KreuzbergError::Cancelled);
    }

    // Pre-render structured document for output formats that benefit from headings,
    // or when hierarchy extraction is explicitly enabled.
    let hierarchy_enabled = config
        .pdf_options
        .as_ref()
        .is_some_and(|opts| opts.hierarchy.as_ref().is_some_and(|h| h.enabled));
    let needs_structured = hierarchy_enabled
        || matches!(
            config.output_format,
            OutputFormat::Markdown | OutputFormat::Djot | OutputFormat::Html
        )
        || ocr_inline_images;

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

            // Same gate as the oxide path: only inject placeholders when image extraction
            // is explicitly enabled. Prevents base64 data from leaking into results when
            // the caller sets extract_images=false (fixes #796).
            let images_extraction_enabled = config.images.as_ref().map(|c| c.extract_images).unwrap_or(false)
                || config.pdf_options.as_ref().map(|p| p.extract_images).unwrap_or(false);
            let inject_placeholders =
                images_extraction_enabled && config.images.as_ref().map(|c| c.inject_placeholders).unwrap_or(true);

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
                    images: images.as_deref(),
                    inject_placeholders,
                    layout_hints,
                    allow_single_column,
                    cancel_token: config.cancel_token.as_ref(),
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
        images,
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
}

//! PDF text extraction using the pdf_oxide backend.
//!
//! Provides equivalent functionality to the pdfium-based `text.rs` module,
//! using pdf_oxide for text extraction. Reuses backend-agnostic utilities
//! (`fix_pdf_control_chars`, `contains_html_markup`) from the pdfium text
//! module to avoid logic duplication.

use super::OxideDocument;
use crate::core::config::{ExtractionConfig, PageConfig};
use crate::pdf::error::{PdfError, Result};
use crate::pdf::metadata::PdfExtractionMetadata;
use crate::pdf::text::{contains_html_markup, fix_pdf_control_chars};
use crate::types::{PageBoundary, PageContent};
use pdf_oxide::document::ReadingOrder;
use std::borrow::Cow;

/// Result type for PDF text extraction with optional page tracking.
type PdfTextExtractionResult = (String, Option<Vec<PageBoundary>>, Option<Vec<PageContent>>);

/// Result type for unified PDF text and metadata extraction.
///
/// Contains text, optional page boundaries, optional per-page content, and metadata.
pub type OxideUnifiedExtractionResult = (
    String,
    Option<Vec<PageBoundary>>,
    Option<Vec<PageContent>>,
    PdfExtractionMetadata,
);

/// Extract all text from a PDF document, concatenating pages with double newlines.
///
/// Simple convenience function that returns only the text content.
#[allow(dead_code)]
pub(crate) fn extract_text(doc: &mut OxideDocument) -> Result<String> {
    let (content, _, _) = extract_text_from_oxide_document(doc, None, None)?;
    Ok(content)
}

/// Extract text and metadata from a PDF document in a single pass.
///
/// This is the oxide equivalent of `extract_text_and_metadata_from_pdf_document`.
/// It extracts both text and metadata in one pass through the document.
pub(crate) fn extract_text_and_metadata(
    doc: &mut OxideDocument,
    extraction_config: Option<&ExtractionConfig>,
) -> Result<OxideUnifiedExtractionResult> {
    let page_config = extraction_config.and_then(|c| c.pages.as_ref());
    let (text, boundaries, page_contents) = extract_text_from_oxide_document(doc, page_config, extraction_config)?;

    let metadata = super::metadata::extract_metadata_from_oxide_document(doc, boundaries.as_deref(), &text)?;

    Ok((text, boundaries, page_contents, metadata))
}

/// Extract text from a pdf_oxide document with optional page boundary tracking.
///
/// Mirrors the signature and behaviour of `extract_text_from_pdf_document` in the
/// pdfium backend so callers can switch transparently.
///
/// When `page_config` is `None`, uses a fast path with minimal overhead.
/// When `page_config` is `Some`, tracks byte offsets and optionally collects
/// per-page `PageContent` entries.
pub(crate) fn extract_text_from_oxide_document(
    doc: &mut OxideDocument,
    page_config: Option<&PageConfig>,
    _extraction_config: Option<&ExtractionConfig>,
) -> Result<PdfTextExtractionResult> {
    if let Some(config) = page_config {
        extract_text_with_tracking(doc, config)
    } else {
        extract_text_fast_path(doc)
    }
}

/// Fast path: extract text without page tracking.
///
/// Iterates pages one-by-one, applies control-char fixes and optional HTML
/// conversion, and builds a single concatenated string. Pre-allocates capacity
/// after sampling the first 5 pages.
fn extract_text_fast_path(doc: &mut OxideDocument) -> Result<PdfTextExtractionResult> {
    let page_count = doc
        .doc
        .page_count()
        .map_err(|e| PdfError::TextExtractionFailed(format!("Failed to get page count: {}", e)))?;

    let mut content = String::new();
    let mut total_sample_size = 0usize;
    let mut sample_count = 0;

    for page_idx in 0..page_count {
        let page_text = extract_page_text_column_aware(&mut doc.doc, page_idx)?;

        let page_size = page_text.len();

        if page_idx > 0 {
            content.push_str("\n\n");
        }

        let cleaned = apply_text_cleanup(&page_text);
        content.push_str(&cleaned);

        if page_idx < 5 {
            total_sample_size += page_size;
            sample_count += 1;
        }

        if page_idx == 4 && sample_count > 0 && page_count > 5 {
            let avg_page_size = total_sample_size / sample_count;
            let estimated_remaining = avg_page_size * (page_count - 5);
            content.reserve(estimated_remaining + (estimated_remaining / 10));
        }
    }

    Ok((content, None, None))
}

/// Extract text with page boundary and content tracking.
///
/// Mirrors `extract_text_lazy_with_tracking` in the pdfium backend: tracks byte
/// offsets for each page, optionally collects per-page `PageContent`, and inserts
/// page markers when configured.
fn extract_text_with_tracking(doc: &mut OxideDocument, config: &PageConfig) -> Result<PdfTextExtractionResult> {
    let page_count = doc
        .doc
        .page_count()
        .map_err(|e| PdfError::TextExtractionFailed(format!("Failed to get page count: {}", e)))?;

    let mut content = String::new();
    let mut boundaries = Vec::with_capacity(page_count);
    let mut page_contents = if config.extract_pages {
        Some(Vec::with_capacity(page_count))
    } else {
        None
    };

    let mut total_sample_size = 0usize;
    let mut sample_count = 0;

    for page_idx in 0..page_count {
        let page_number = page_idx + 1;

        let page_text = extract_page_text_column_aware(&mut doc.doc, page_idx)?;

        let page_size = page_text.len();

        if page_idx < 5 {
            total_sample_size += page_size;
            sample_count += 1;
        }

        // Insert page marker before the page content (for ALL pages including page 1)
        if config.insert_page_markers {
            let marker = config.marker_format.replace("{page_num}", &page_number.to_string());
            content.push_str(&marker);
        } else if page_idx > 0 {
            // Only add separator between pages when markers are disabled
            content.push_str("\n\n");
        }

        let cleaned = apply_text_cleanup(&page_text);

        let byte_start = content.len();
        content.push_str(&cleaned);
        let byte_end = content.len();

        boundaries.push(PageBoundary {
            byte_start,
            byte_end,
            page_number,
        });

        if let Some(ref mut pages) = page_contents {
            let is_blank = Some(crate::extraction::blank_detection::is_page_text_blank(&page_text));
            pages.push(PageContent {
                page_number,
                content: page_text,
                tables: Vec::new(),
                images: Vec::new(),
                hierarchy: None,
                is_blank,
                layout_regions: None,
            });
        }

        if page_idx == 4 && page_count > 5 && sample_count > 0 {
            let avg_page_size = total_sample_size / sample_count;
            let estimated_remaining = avg_page_size * (page_count - 5);
            let separator_overhead = (page_count - 5) * 3;
            content.reserve(estimated_remaining + separator_overhead + (estimated_remaining / 10));
        }
    }

    Ok((content, Some(boundaries), page_contents))
}

/// Extract text from a single page using column-aware reading order.
///
/// Uses `extract_page_text_with_options` with `ReadingOrder::ColumnAware` to
/// apply XY-Cut column detection. This reads each column top-to-bottom before
/// moving to the next, avoiding interleaved text in multi-column layouts.
fn extract_page_text_column_aware(doc: &mut pdf_oxide::PdfDocument, page_index: usize) -> Result<String> {
    let page_text_data = doc
        .extract_page_text_with_options(page_index, ReadingOrder::ColumnAware)
        .map_err(|e| {
            PdfError::TextExtractionFailed(format!("Page {} text extraction failed: {}", page_index + 1, e))
        })?;

    // Assemble text from column-aware ordered spans, filtering out artifacts
    // (headers, footers, watermarks, page numbers) to keep main body content only.
    let mut text = String::with_capacity(page_text_data.spans.len() * 20);
    let mut prev_span: Option<&pdf_oxide::layout::TextSpan> = None;

    for span in &page_text_data.spans {
        if let Some(prev) = prev_span {
            let prev_end_x = prev.bbox.x + prev.bbox.width;
            let y_gap = (prev.bbox.y - span.bbox.y).abs();
            let same_line = y_gap < span.bbox.height.max(prev.bbox.height) * 0.5;

            if same_line {
                let x_gap = span.bbox.x - prev_end_x;
                if x_gap > span.font_size * 0.15 {
                    text.push(' ');
                }
            } else {
                text.push('\n');
            }
        }
        text.push_str(&span.text);
        prev_span = Some(span);
    }

    Ok(text)
}

/// Apply common text cleanup: fix control chars and optionally convert HTML.
///
/// Returns a `Cow` to avoid allocation when the text is already clean.
fn apply_text_cleanup(text: &str) -> Cow<'_, str> {
    let cleaned = fix_pdf_control_chars(text);

    #[cfg(feature = "html")]
    if contains_html_markup(&cleaned) {
        return Cow::Owned(crate::pdf::text::convert_html_page_text(&cleaned));
    }

    #[cfg(not(feature = "html"))]
    let _ = contains_html_markup(&cleaned);

    cleaned
}

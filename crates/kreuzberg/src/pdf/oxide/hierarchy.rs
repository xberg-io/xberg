//! Font metrics extraction for heading hierarchy detection using the pdf_oxide backend.
//!
//! Uses pdf_oxide's span extraction to get font_size, font_weight, is_italic,
//! and font_name, converting them to `SegmentData` for the backend-agnostic
//! clustering pipeline that assigns heading levels (H1-H6) to text blocks.

use super::OxideDocument;
use crate::pdf::error::Result;
use crate::pdf::hierarchy::SegmentData;

/// Extract text segments with font metrics from a PDF page using pdf_oxide.
///
/// Returns `SegmentData` objects containing text, position, and font metadata
/// (size, bold, italic, monospace). These feed into the existing backend-agnostic
/// font size clustering pipeline for heading detection.
///
/// Uses default (top-to-bottom) reading order rather than column-aware ordering,
/// because the hierarchy/structure pipeline depends on physical span position for
/// font-size clustering and heading detection. Column-aware reordering changes
/// span sequence in ways that break single-column heading detection.
///
/// # Arguments
///
/// * `doc` - Mutable reference to the oxide document
/// * `page_index` - Zero-based page index
///
/// # Returns
///
/// Vector of `SegmentData` objects with font metrics for hierarchy detection.
pub(crate) fn extract_segments_from_page(doc: &mut OxideDocument, page_index: usize) -> Result<Vec<SegmentData>> {
    // Get page height for coordinate conversion
    let page_height = doc
        .doc
        .get_page_media_box(page_index)
        .ok()
        .map(|(_, lly, _, ury)| (ury - lly).abs())
        .unwrap_or(792.0); // Letter size fallback

    let spans = match doc.doc.extract_spans(page_index) {
        Ok(spans) => spans,
        Err(e) => {
            tracing::debug!(page = page_index, "pdf_oxide extract_spans failed for hierarchy: {e}");
            return Ok(Vec::new());
        }
    };

    let segments: Vec<SegmentData> = spans
        .into_iter()
        .filter(|span| {
            // Skip page furniture (headers/footers/watermarks)
            if span.artifact_type.is_some() {
                return false;
            }
            !span.text.trim().is_empty()
        })
        .map(|span| {
            let is_bold = span.font_weight == pdf_oxide::layout::text_block::FontWeight::Bold;
            let bbox = &span.bbox;

            // Convert from screen coords (y=0 at top) to PDF coords (y=0 at bottom)
            let screen_bottom = bbox.y + bbox.height;
            let pdf_baseline_y = page_height - screen_bottom;
            let pdf_y = page_height - bbox.y - bbox.height;

            SegmentData {
                text: span.text,
                x: bbox.x,
                y: pdf_y,
                width: bbox.width,
                height: bbox.height,
                font_size: span.font_size,
                is_bold,
                is_italic: span.is_italic,
                is_monospace: span.is_monospace,
                baseline_y: pdf_baseline_y,
            }
        })
        .collect();

    Ok(segments)
}

/// Extract text segments from all pages of a PDF document using pdf_oxide.
///
/// Returns segments indexed by page (0-based). This is the oxide equivalent
/// of calling `extract_segments_with_oxide` from `oxide_text.rs`.
///
/// # Arguments
///
/// * `doc` - Mutable reference to the oxide document
///
/// # Returns
///
/// Vector of per-page segment vectors, indexed by page number (0-based).
pub(crate) fn extract_all_segments(doc: &mut OxideDocument) -> Result<Vec<Vec<SegmentData>>> {
    let page_count = doc.doc.page_count().map_err(|e| {
        crate::pdf::error::PdfError::TextExtractionFailed(format!("pdf_oxide: failed to get page count: {e}"))
    })?;

    let mut all_pages: Vec<Vec<SegmentData>> = Vec::with_capacity(page_count);

    for page_idx in 0..page_count {
        let segments = extract_segments_from_page(doc, page_idx)?;
        all_pages.push(segments);
    }

    Ok(all_pages)
}

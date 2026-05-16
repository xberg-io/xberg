//! PDF text extraction using the pdf_oxide backend.

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
/// Mirrors the signature and behaviour of `extract_text_from_pdf_document`.
///
/// When `page_config` is `Some`, tracks byte offsets and optionally collects
/// per-page `PageContent` entries.
///
/// When `page_config` is `None` but `extraction_config` requires per-page boundaries
/// (i.e. `force_ocr_pages` is set or an `ocr` config is present for quality evaluation),
/// boundary tracking is enabled automatically with a default `PageConfig` so that the
/// mixed-OCR and quality-threshold codepaths receive the offsets they need.
///
/// Otherwise the fast path is used (no per-page tracking).
pub(crate) fn extract_text_from_oxide_document(
    doc: &mut OxideDocument,
    page_config: Option<&PageConfig>,
    extraction_config: Option<&ExtractionConfig>,
) -> Result<PdfTextExtractionResult> {
    let needs_boundaries =
        extraction_config.is_some_and(|c| c.force_ocr_pages.as_ref().is_some_and(|p| !p.is_empty()) || c.ocr.is_some());

    if let Some(config) = page_config {
        extract_text_with_tracking(doc, config)
    } else if needs_boundaries {
        // Use a default PageConfig (no markers, no per-page content) purely for
        // boundary tracking required by mixed-OCR and OCR quality evaluation.
        let default_config = PageConfig::default();
        extract_text_with_tracking(doc, &default_config)
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
/// Mirrors `extract_text_lazy_with_tracking`: tracks byte
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
            page_number: page_number as u32,
        });

        if let Some(ref mut pages) = page_contents {
            let is_blank = Some(crate::extraction::blank_detection::is_page_text_blank(&page_text));
            pages.push(PageContent {
                page_number: page_number as u32,
                content: page_text,
                tables: Vec::new(),
                image_indices: Vec::new(),
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

/// Minimum number of x-disorder events required to classify the span list as glyph-fragmented.
///
/// Two events avoid false positives: a single x-reset can occur naturally in right-to-left
/// or wrapped text, but ≥ 2 resets in "same-line" transitions reliably indicate that
/// pdf_oxide's ColumnAware ordering has shuffled glyph-level spans by y-group.
const MIN_DISORDER_COUNT: usize = 2;

/// y-proximity threshold (pt) for coalescing glyph-jitter spans onto one text line.
/// Word's BT-per-glyph sinusoidal jitter (6-glyph period) produces consecutive-pair
/// y-gaps ≤ ~3.03 pt; 4.0 pt covers this while staying below normal line spacing (~14 pt).
const COALESCE_THRESHOLD: f32 = 4.0;

/// Returns true when `spans` exhibits the glyph-fragmentation signature.
///
/// pdf_oxide's ColumnAware reading order groups all spans at one y-level before moving to the
/// next. For Word-exported PDFs where each glyph has its own BT…ET block with a sinusoidal
/// y-jitter, this produces groups ordered by y-level rather than by reading order: "et" (y=703)
/// appears before "H" (y=700) even though "H" comes first in the text. The signature is
/// consecutive "same-line" span transitions where the current span's x-coordinate resets
/// significantly to the left — indicating a new y-group started. ≥ MIN_DISORDER_COUNT such
/// events means position-based reconstruction is needed. (issue #962)
fn is_fragmented_span_list(spans: &[pdf_oxide::layout::TextSpan]) -> bool {
    let mut disorder_count = 0;
    for window in spans.windows(2) {
        let prev = &window[0];
        let cur = &window[1];
        let y_gap = (prev.bbox.y - cur.bbox.y).abs();
        let eff_height = prev.bbox.height.max(cur.bbox.height).max(prev.font_size * 0.5);
        if y_gap < eff_height * 0.5 {
            // Same-line transition: a significant x-reset means the ordering is wrong.
            if cur.bbox.x < prev.bbox.x - prev.font_size {
                disorder_count += 1;
                if disorder_count >= MIN_DISORDER_COUNT {
                    return true;
                }
            }
        }
    }
    false
}

/// Rebuild readable text from a glyph-fragmented span list (issue #962).
///
/// Algorithm:
/// 1. Sort spans by y-descending (top-of-page first in PDF coordinates).
/// 2. Group by chained y-proximity: consecutive spans within COALESCE_THRESHOLD pt
///    of the previous span belong to the same visual line.
/// 3. Within each group sort by x-ascending (left-to-right reading order).
/// 4. Concatenate, inserting a space wherever the x-gap between adjacent spans
///    exceeds font_size * 0.5.
fn rebuild_text_from_fragmented_spans(spans: &[pdf_oxide::layout::TextSpan]) -> String {
    if spans.is_empty() {
        return String::new();
    }

    let mut sorted: Vec<&pdf_oxide::layout::TextSpan> = spans.iter().collect();
    sorted.sort_by(|a, b| b.bbox.y.partial_cmp(&a.bbox.y).unwrap_or(std::cmp::Ordering::Equal));

    // Group by chained y-proximity.
    let mut groups: Vec<Vec<&pdf_oxide::layout::TextSpan>> = Vec::new();
    for span in sorted {
        let belongs = groups.last().map_or(false, |g| {
            let prev_y = g.last().unwrap().bbox.y;
            (span.bbox.y - prev_y).abs() <= COALESCE_THRESHOLD
        });
        if belongs {
            groups.last_mut().unwrap().push(span);
        } else {
            groups.push(vec![span]);
        }
    }

    let mut result = String::new();
    for (gi, group) in groups.iter_mut().enumerate() {
        group.sort_by(|a, b| a.bbox.x.partial_cmp(&b.bbox.x).unwrap_or(std::cmp::Ordering::Equal));
        if gi > 0 {
            result.push('\n');
        }
        let font_size = group.iter().map(|s| s.font_size).fold(0.0_f32, f32::max);
        let space_threshold = font_size * 0.5;
        let mut prev_end_x = f32::NEG_INFINITY;
        for span in group.iter() {
            if prev_end_x.is_finite() && span.bbox.x - prev_end_x > space_threshold {
                result.push(' ');
            }
            result.push_str(&span.text);
            prev_end_x = span.bbox.x + span.bbox.width;
        }
    }
    result
}

/// Extract text from a single page using column-aware reading order.
///
/// Uses `extract_page_text_with_options` with `ReadingOrder::ColumnAware` to
/// apply XY-Cut column detection. This reads each column top-to-bottom before
/// moving to the next, avoiding interleaved text in multi-column layouts.
///
/// Detects paragraph breaks via vertical gap heuristics: when the gap between
/// lines exceeds 1.5x the median line height, inserts a paragraph break (\n\n).
///
/// Applies a fragmentation repair pass for PDFs that position each glyph via its
/// own BT…ET block (issue #962): detected by ≥ MIN_DISORDER_COUNT same-line x-reset
/// events, which occur when ColumnAware ordering groups spans by y-level rather than
/// reading order; repaired by re-sorting on position rather than relying on stream order.
fn extract_page_text_column_aware(doc: &mut pdf_oxide::PdfDocument, page_index: usize) -> Result<String> {
    let page_text_data = doc
        .extract_page_text_with_options(page_index, ReadingOrder::ColumnAware)
        .map_err(|e| {
            PdfError::TextExtractionFailed(format!("Page {} text extraction failed: {}", page_index + 1, e))
        })?;

    // Issue #962: Word-exported PDFs position each glyph in its own BT…ET block with a
    // sinusoidal y-jitter. pdf_oxide's ColumnAware ordering groups spans by y-level, so
    // glyph-level spans appear out of reading order. Detect via x-resets and rebuild.
    if is_fragmented_span_list(&page_text_data.spans) {
        tracing::debug!(
            span_count = page_text_data.spans.len(),
            "glyph fragmentation detected — rebuilding text from span positions (#962)"
        );
        return Ok(rebuild_text_from_fragmented_spans(&page_text_data.spans));
    }

    // Compute median line height for paragraph break detection.
    let mut heights: Vec<f32> = page_text_data.spans.iter().map(|s| s.bbox.height).collect();
    heights.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_height = if heights.is_empty() {
        1.0
    } else {
        heights[heights.len() / 2]
    };
    let paragraph_gap_threshold = median_height * 1.5;

    tracing::debug!(
        span_count = page_text_data.spans.len(),
        median_height,
        paragraph_gap_threshold,
        "paragraph break detection initialized"
    );

    // Assemble text from column-aware ordered spans, filtering out artifacts
    // (headers, footers, watermarks, page numbers) to keep main body content only.
    let mut text = String::with_capacity(page_text_data.spans.len() * 20);
    let mut prev_span: Option<&pdf_oxide::layout::TextSpan> = None;

    for span in page_text_data.spans.iter() {
        if let Some(prev) = prev_span {
            let prev_end_x = prev.bbox.x + prev.bbox.width;
            let y_gap = (prev.bbox.y - span.bbox.y).abs();
            // Use font_size as fallback when bbox.height is near-zero (defense in depth
            // for spans that don't form a long enough run to trigger is_fragmented_span_list).
            let eff_height = span.bbox.height.max(prev.bbox.height).max(span.font_size * 0.5);
            let same_line = y_gap < eff_height * 0.5;

            if same_line {
                let x_gap = span.bbox.x - prev_end_x;
                if x_gap > span.font_size * 0.15 {
                    text.push(' ');
                }
            } else if y_gap > paragraph_gap_threshold {
                text.push_str("\n\n");
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

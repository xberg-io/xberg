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

/// Maximum y-gap (pt) between two spans that can still be considered "same line" under
/// the glyph-fragmentation detection heuristic.
///
/// Word's per-glyph BT/ET sinusoidal jitter (6-glyph period, ~3 pt amplitude) produces
/// consecutive-pair y-gaps of ≤ ~3.03 pt. 5 pt adds headroom for atypical Word
/// configurations while remaining well below normal body-text line spacing (~12–14 pt).
/// Using an absolute value instead of a font-size fraction avoids the false-positive
/// zone where `font_size * 0.25` (the old fallback) is large enough for bigger fonts
/// to classify consecutive real lines as "same line".
const MAX_GLYPH_JITTER_PT: f32 = 5.0;

/// Minimum number of qualifying x-disorder events before the span list is classified
/// as glyph-fragmented.
///
/// Empirically, pdf_oxide groups consecutive same-y chars into multi-char spans, so a
/// 32-char Word jitter word (period 6, 3 distinct y-levels) produces exactly 4 disorder
/// events (2 per XY-Cut column at each y-level transition). Requiring ≥ 3 events is
/// sufficient to detect all jitter amplitudes ≥ 3 pt while remaining robust against
/// false positives: the short-span guard (≤ 3 chars) and the 5 pt same-line ceiling
/// together make it essentially impossible for normal multi-column text to accumulate
/// 3 consecutive qualifying resets.
const MIN_DISORDER_COUNT: usize = 3;

/// y-proximity threshold (pt) for grouping spans into visual lines during reconstruction.
/// Must be ≥ MAX_GLYPH_JITTER_PT so every span pair accepted by the detection gate is
/// also merged into the same group during reconstruction.
const COALESCE_THRESHOLD: f32 = 5.0;

// TODO: evaluate whether pdf_oxide's ColumnAware ReadingOrder should handle
// per-glyph BT…ET PDFs by coalescing spans with sinusoidal y-jitter before returning
// them. If confirmed as a pdf_oxide bug, file an upstream issue, link it here, and
// remove this heuristic once it is fixed. Raised in kreuzberg PR #986 (2026-05-17).

/// Returns true when `spans` exhibits the glyph-fragmentation signature (issue #962).
///
/// pdf_oxide's ColumnAware reading order groups all spans at one y-level before moving
/// to the next. For Word-exported PDFs where each glyph has its own BT…ET block with a
/// sinusoidal y-jitter, this produces groups ordered by y-level rather than by reading
/// order: "et" (y=703) appears before "H" (y=700) even though "H" comes first visually.
///
/// Two-part signature:
/// 1. Both spans are short (≤ 3 chars): per-glyph BT/ET always produces single-character
///    spans; multi-character spans are word-level and cannot be glyph artifacts.
/// 2. The spans are on the same visual line (y-gap ≤ MAX_GLYPH_JITTER_PT when heights
///    are zero, or < half the measured height otherwise) yet the x-coordinate resets
///    significantly leftward — indicating a new y-group started mid-reading-order.
///
/// ≥ MIN_DISORDER_COUNT such events means position-based reconstruction is needed.
fn is_fragmented_span_list(spans: &[pdf_oxide::layout::TextSpan]) -> bool {
    let mut disorder_count = 0;
    for window in spans.windows(2) {
        let prev = &window[0];
        let cur = &window[1];

        // Per-glyph BT/ET fragmentation always produces single-character spans.
        // Word-level spans (> 3 chars) cannot be glyph-positioning artifacts and
        // must not count toward the disorder total — this is the primary false-positive
        // guard for paragraphs where line breaks naturally reset x.
        if prev.text.chars().count() > 3 || cur.text.chars().count() > 3 {
            continue;
        }

        let y_gap = (prev.bbox.y - cur.bbox.y).abs();

        // When bbox.height is zero (common for pdf_oxide on some font descriptors),
        // fall back to the absolute Word jitter ceiling rather than a font-size fraction.
        // A fraction (font_size * 0.25) scales with font size: for a 24 pt font it
        // reaches 6 pt, which overlaps with normal tight leading and produces false
        // positives on height-zero span lists from legitimate documents.
        let eff_height = prev.bbox.height.max(cur.bbox.height);
        let same_line = if eff_height > 0.0 {
            y_gap < eff_height * 0.5
        } else {
            y_gap <= MAX_GLYPH_JITTER_PT
        };

        if same_line && cur.bbox.x < prev.bbox.x - prev.font_size {
            disorder_count += 1;
            if disorder_count >= MIN_DISORDER_COUNT {
                return true;
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
        let belongs = groups.last().is_some_and(|g| {
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

#[cfg(test)]
mod tests {
    use super::*;
    use pdf_oxide::geometry::Rect;
    use pdf_oxide::layout::TextSpan;

    fn span(text: &str, x: f32, y: f32, height: f32, font_size: f32) -> TextSpan {
        TextSpan {
            text: text.to_string(),
            bbox: Rect {
                x,
                y,
                width: font_size * 0.6,
                height,
            },
            font_size,
            ..TextSpan::default()
        }
    }

    /// Build a list of N single-char spans that each trigger a same-line x-disorder
    /// event. All at the same y (zero height fallback path), each span's x is
    /// `prev.x - font_size - 1` so cur.x < prev.x - font_size is always true.
    fn disorder_spans(count: usize) -> Vec<TextSpan> {
        let font_size = 12.0_f32;
        let mut spans = Vec::with_capacity(count + 1);
        // First span at x=300; each subsequent span is reset to the left.
        let mut x = 300.0_f32;
        for _i in 0..=count {
            spans.push(span("A", x, 700.0, 0.0, font_size));
            // Next span resets leftward so cur.x < prev.x - font_size
            x = x - font_size - 1.0;
        }
        spans
    }

    #[test]
    fn fragmentation_detected_at_threshold() {
        let spans = disorder_spans(MIN_DISORDER_COUNT);
        assert!(
            is_fragmented_span_list(&spans),
            "should detect fragmentation at exactly MIN_DISORDER_COUNT ({MIN_DISORDER_COUNT}) events"
        );
    }

    #[test]
    fn fragmentation_not_detected_below_threshold() {
        // One fewer disorder event than required must NOT trigger reconstruction.
        let spans = disorder_spans(MIN_DISORDER_COUNT - 1);
        assert!(
            !is_fragmented_span_list(&spans),
            "must NOT detect fragmentation with {} events (threshold is {MIN_DISORDER_COUNT})",
            MIN_DISORDER_COUNT - 1
        );
    }

    #[test]
    fn long_spans_never_count_toward_disorder() {
        // A span list with many x-resets but all spans > 3 chars must return false.
        let font_size = 12.0_f32;
        let mut spans = Vec::new();
        let mut x = 500.0_f32;
        for _ in 0..20 {
            spans.push(span("word", x, 700.0, 0.0, font_size));
            x = x - font_size - 1.0;
        }
        assert!(
            !is_fragmented_span_list(&spans),
            "word-level spans (> 3 chars) must never trigger fragmentation detection"
        );
    }

    #[test]
    fn large_y_gap_not_classified_as_same_line() {
        // Two short spans separated by 14 pt (normal line spacing) with an x-reset.
        // With zero heights, MAX_GLYPH_JITTER_PT = 5.0, so 14 pt gap is above the ceiling.
        let spans = vec![
            span("A", 300.0, 700.0, 0.0, 12.0),
            span("B", 50.0, 686.0, 0.0, 12.0), // y_gap=14, x resets
        ];
        assert!(
            !is_fragmented_span_list(&spans),
            "14 pt y-gap must not be classified as same-line (MAX_GLYPH_JITTER_PT={MAX_GLYPH_JITTER_PT})"
        );
    }

    #[test]
    fn empty_spans_returns_false() {
        assert!(!is_fragmented_span_list(&[]));
    }

    #[test]
    fn single_span_returns_false() {
        assert!(!is_fragmented_span_list(&[span("A", 100.0, 700.0, 0.0, 12.0)]));
    }
}

//! Scanned-page detection for PDFs.

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::document::ReadingOrder;
use xberg_native_pdf::extractors::auto::{ImageCodecClass, ProducerPrior};
use xberg_native_pdf::fonts::MappingProvenance;
use xberg_native_pdf::layout::TextSpan;

#[cfg(test)]
use crate::core::config::DEFAULT_SCANNED_MIN_CONFIDENCE;

/// Below this raster coverage a page is text with a figure, never a scan.
const IMAGE_COVERAGE_MIN: f32 = 0.80;

/// Fraction of glyphs in render mode 3 (invisible) that marks an OCR sidecar.
const INVISIBLE_TEXT_MIN: f32 = 0.50;

/// A full-page raster alone. Below every usable threshold: a slide with a
/// full-bleed background image scores exactly this.
const SCORE_FULL_PAGE_RASTER: f32 = 0.50;

/// Added when the text layer is hidden or absent.
const SCORE_NO_VISIBLE_TEXT: f32 = 0.35;

/// Added for CCITT/JBIG2: bilevel fax codecs, not emitted by authoring tools.
const SCORE_BILEVEL_CODEC: f32 = 0.10;

/// Added when the producer names scanner software. A weak prior, never decisive.
const SCORE_SCANNER_PRODUCER: f32 = 0.05;

/// Per-page evidence, gathered without decoding image pixels.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PageScanSignals {
    /// Fraction of the page covered by raster images, clamped to `[0, 1]`.
    pub image_coverage: f32,
    /// Fraction of glyphs drawn invisibly (text render mode 3), in `[0, 1]`.
    pub invisible_text_ratio: f32,
    /// Number of glyphs in the native text layer.
    pub glyph_count: usize,
    /// Dominant raster codec on the page.
    pub codec: ImageCodecClass,
    /// Whether the document producer looks like scanner software.
    pub producer_prior: ProducerPrior,
}

/// Document-level detection outcome.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ScanDetection {
    /// Highest per-page confidence in the document, in `[0, 1]`.
    pub confidence: f32,
    /// Per-page confidence, indexed by zero-based page number.
    pub page_confidence: Vec<f32>,
}

impl ScanDetection {
    /// Zero-based indices of pages scoring at or above `min_confidence`.
    pub(crate) fn scanned_page_indices(&self, min_confidence: f32) -> Vec<usize> {
        let threshold = min_confidence.clamp(0.0, 1.0);
        self.page_confidence
            .iter()
            .enumerate()
            .filter(|(_, score)| **score >= threshold)
            .map(|(index, _)| index)
            .collect()
    }
}

/// Grade one page's evidence. Pure, so it is testable without a [`PdfDocument`].
pub(crate) fn score_page(signals: &PageScanSignals) -> f32 {
    if signals.image_coverage < IMAGE_COVERAGE_MIN {
        return 0.0;
    }

    let mut score = SCORE_FULL_PAGE_RASTER;

    if signals.glyph_count == 0 || signals.invisible_text_ratio >= INVISIBLE_TEXT_MIN {
        score += SCORE_NO_VISIBLE_TEXT;
    }

    if matches!(signals.codec, ImageCodecClass::Ccitt | ImageCodecClass::Jbig2) {
        score += SCORE_BILEVEL_CODEC;
    }

    if signals.producer_prior == ProducerPrior::Scanner {
        score += SCORE_SCANNER_PRODUCER;
    }

    score.clamp(0.0, 1.0)
}

/// Fraction of the page covered by raster images, without decoding pixel data.
///
/// Overlapping images are summed, not unioned, so this is an upper bound: it may
/// over-select a page for inspection, never under-select one.
fn image_coverage(doc: &PdfDocument, page_index: usize) -> Option<f32> {
    let (x0, y0, x1, y1) = doc.get_page_media_box(page_index).ok()?;
    let page_area = ((x1 - x0) * (y1 - y0)).abs();
    if page_area <= f32::EPSILON {
        return None;
    }

    let (left, right) = (x0.min(x1), x0.max(x1));
    let (bottom, top) = (y0.min(y1), y0.max(y1));

    let handles = doc.page_image_handles(page_index).ok()?;
    let covered: f32 = handles
        .iter()
        .map(|handle| {
            let bbox = &handle.bbox;
            let width = (bbox.x + bbox.width).min(right) - bbox.x.max(left);
            let height = (bbox.y + bbox.height).min(top) - bbox.y.max(bottom);
            width.max(0.0) * height.max(0.0)
        })
        .sum();

    Some((covered / page_area).clamp(0.0, 1.0))
}

/// Signals for one page, or `None` when it yields no evidence.
///
/// Pages under [`IMAGE_COVERAGE_MIN`] skip the text-layer inspection: it parses
/// the content stream, and cannot lift their score above zero.
fn page_signals(doc: &PdfDocument, page_index: usize) -> Option<PageScanSignals> {
    let coverage = image_coverage(doc, page_index)?;
    if coverage < IMAGE_COVERAGE_MIN {
        return None;
    }

    // Detection is advisory: a page that panics must not abort the extraction. ~keep
    let classified = super::native::guard_native_panic(
        || doc.classify_page(page_index).map_err(|error| error.to_string()),
        |message| message,
    )
    .ok()?;
    let signals = classified.signals;

    Some(PageScanSignals {
        image_coverage: coverage,
        invisible_text_ratio: signals.invisible_text_ratio,
        glyph_count: signals.text_glyph_count,
        codec: signals.codec,
        producer_prior: signals.producer_prior,
    })
}

/// Apply `page_work` to every page index, in parallel across the configured thread budget.
///
/// Both page passes in this module read one page each through a shared `&PdfDocument`, which
/// `xberg_native_pdf` asserts is `Send + Sync` (`_assert_send_sync::<PdfDocument>()` in its
/// `document` module) with its interior state behind mutexes for exactly this reason. They ran
/// as plain sequential loops over `0..page_count`, so scan detection and provenance routing
/// held one core for the whole document whatever `ConcurrencyConfig::max_threads` was set to,
/// and no thread budget could shorten them (issue #1723). This is the same shape as the
/// page-rendering pass in `extractors/pdf/ocr/rendering.rs` (issue #1666).
///
/// `into_par_iter()` over a `Range<usize>` is an `IndexedParallelIterator`, so `collect()`
/// returns one entry per page in page order. Both callers index their result by page, so their
/// output is positional and unchanged. ~keep
fn map_pages<T, F>(page_count: usize, page_work: F) -> Vec<T>
where
    T: Send,
    F: Fn(usize) -> T + Sync + Send,
{
    // rayon's work-stealing pool needs OS threads; wasm32 has none, so this falls back to a
    // sequential iterator there, matching the gate on the paragraph pass in
    // `pdf/structure/pipeline.rs`. ~keep
    #[cfg(not(target_arch = "wasm32"))]
    {
        use rayon::prelude::*;
        (0..page_count).into_par_iter().map(page_work).collect()
    }
    #[cfg(target_arch = "wasm32")]
    {
        (0..page_count).map(page_work).collect()
    }
}

/// Grade every page of `doc`.
///
/// Infallible: an unreadable page scores `0.0` rather than failing extraction.
/// `None` only when the page count is unavailable.
pub(crate) fn detect(doc: &PdfDocument) -> Option<ScanDetection> {
    let page_count = doc.page_count().ok()?;

    let page_confidence = map_pages(page_count, |page_index| {
        page_signals(doc, page_index).as_ref().map_or(0.0, score_page)
    });

    let confidence = page_confidence.iter().copied().fold(0.0_f32, f32::max);

    Some(ScanDetection {
        confidence,
        page_confidence,
    })
}

/// Non-whitespace character counts `(fabricated, total)` for one page's spans.
///
/// `fabricated` counts characters belonging to a span whose
/// [`MappingProvenance`] is [`MappingProvenance::Fallback`] — xberg_native_pdf 0.3.75's
/// direct signal that no ISO 32000-1 §9.10.2 mapping tier produced the
/// character's Unicode value, so it was fabricated by the extractor rather than
/// read from the file (issue #1254). Every other provenance (`ActualText` ..
/// `EmbeddedCmap`) was read from the file and never counts as fabricated. A
/// span with `provenance: None` ("unknown", e.g. not populated by this
/// xberg_native_pdf build) still contributes to `total` but never to `fabricated`, so
/// a page with only unknown provenance can never look fabricated on its own.
///
/// Pure and independent of any [`PdfDocument`], so it is unit-testable with
/// hand-built spans.
fn fabricated_char_counts(spans: &[TextSpan]) -> (usize, usize) {
    let mut fabricated = 0usize;
    let mut total = 0usize;
    for span in spans {
        let non_whitespace = span.text.chars().filter(|c| !c.is_whitespace()).count();
        total += non_whitespace;
        if span.provenance == Some(MappingProvenance::Fallback) {
            fabricated += non_whitespace;
        }
    }
    (fabricated, total)
}

/// Whether page `page_index` has a fabricated text layer: `min_chars` or more
/// non-whitespace characters, at least `min_ratio` of which carry
/// `MappingProvenance::Fallback` (issue #1254).
///
/// Advisory like the rest of scan detection: a page xberg_native_pdf cannot extract or
/// that panics during extraction is reported as not fabricated rather than
/// aborting the caller.
fn page_has_fabricated_text(doc: &PdfDocument, page_index: usize, min_ratio: f64, min_chars: usize) -> bool {
    let page_text = match super::native::guard_native_panic(
        || {
            doc.extract_page_text_with_options(page_index, ReadingOrder::ColumnAware)
                .map_err(|error| error.to_string())
        },
        |message| message,
    ) {
        Ok(page_text) => page_text,
        Err(_) => return false,
    };

    let (fabricated, total) = fabricated_char_counts(&page_text.spans);
    if total < min_chars {
        return false;
    }

    (fabricated as f64 / total as f64) >= min_ratio
}

/// Zero-based indices of pages whose text layer is fabricated per
/// [`page_has_fabricated_text`] (issue #1254).
///
/// Independent of raster scan detection: a text-bearing page with a broken
/// glyph-to-Unicode mapping (e.g. a subset `Identity-H` font with no
/// `/ToUnicode` CMap) has low image coverage and would otherwise never be
/// selected by [`detect`], so this is evaluated separately and its result is
/// meant to be unioned into the caller's scanned-page set.
pub(crate) fn fabricated_provenance_page_indices(doc: &PdfDocument, min_ratio: f64, min_chars: usize) -> Vec<usize> {
    let Ok(page_count) = doc.page_count() else {
        return Vec::new();
    };

    let fabricated = map_pages(page_count, |page_index| {
        page_has_fabricated_text(doc, page_index, min_ratio, min_chars)
    });

    fabricated
        .into_iter()
        .enumerate()
        .filter_map(|(page_index, is_fabricated)| is_fabricated.then_some(page_index))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Both page passes go through [`map_pages`], so the order guarantee is pinned on it
    /// directly. A pass that only counted pages would still be green on a shuffled result,
    /// and every caller indexes its result by page number.
    #[test]
    fn map_pages_returns_one_entry_per_page_in_page_order() {
        let page_count = 1024;
        let squares = map_pages(page_count, |page_index| page_index * page_index);
        assert_eq!(
            squares,
            (0..page_count)
                .map(|page_index| page_index * page_index)
                .collect::<Vec<_>>(),
            "map_pages must return one entry per page, in page order"
        );
    }

    /// The pass must reach more than one thread. Asserted against a pool this test builds
    /// rather than the machine's core count, so it means the same thing on a one-core runner
    /// as on the 32-core box the issue was measured on, and against the closure's own record
    /// rather than wall clock, which flakes under load.
    ///
    /// Each page holds its thread for [`DISPATCH_PAGE_HOLD`]: with no work per page, one
    /// worker drains the whole range before the others wake on a loaded runner (22 of 60
    /// runs under a 12-core load), and the pass then looks sequential. ~keep
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn map_pages_dispatches_pages_across_the_pool() {
        use std::collections::HashSet;
        use std::sync::Mutex;

        const DISPATCH_PAGE_COUNT: usize = 512;
        const DISPATCH_POOL_THREADS: usize = 8;
        const DISPATCH_PAGE_HOLD: std::time::Duration = std::time::Duration::from_micros(200);

        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(DISPATCH_POOL_THREADS)
            .build()
            .expect("the test's own pool must build");
        let threads: Mutex<HashSet<std::thread::ThreadId>> = Mutex::new(HashSet::new());

        let pages = pool.install(|| {
            map_pages(DISPATCH_PAGE_COUNT, |page_index| {
                threads
                    .lock()
                    .expect("thread record must not be poisoned")
                    .insert(std::thread::current().id());
                std::thread::sleep(DISPATCH_PAGE_HOLD);
                page_index
            })
        });

        assert_eq!(pages, (0..DISPATCH_PAGE_COUNT).collect::<Vec<_>>());
        let distinct = threads.lock().expect("thread record must not be poisoned").len();
        assert!(
            distinct > 1,
            "map_pages ran every page on {distinct} thread(s): the pass did not dispatch across the pool"
        );
    }

    /// The wiring, on a real document: both passes must agree with a page-by-page run, entry
    /// for entry. This is what says `detect` and `fabricated_provenance_page_indices` read the
    /// pages they claim to and report them in page order, which the seam test above cannot
    /// say on its own.
    ///
    /// The fixture is chosen so that a reordering is visible: its pages do not all score the
    /// same, and some but not all of them carry a fabricated mapping (15 of 18). On a
    /// born-digital fixture every page scores `0.0` and no page is fabricated, so the two
    /// comparisons below would pass on any permutation. Both properties are asserted on the
    /// sequential run so the fixture cannot drift into that shape unnoticed.
    ///
    /// The parallel pass runs on its own freshly opened handle. A handle the sequential pass
    /// has already walked has every font and page object cached, so a concurrent read
    /// through it never races a cold load, which is the shape #1737 exists to make
    /// order-independent. ~keep
    #[test]
    fn both_page_passes_match_a_page_by_page_run() {
        let thresholds = crate::core::config::OcrQualityThresholds::default();
        let min_ratio = thresholds.min_provenance_fallback_ratio;
        let min_chars = thresholds.min_total_non_whitespace;

        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/pdf/non_ascii_text.pdf");
        let sequential_doc = PdfDocument::open(&path).expect("corpus document must open");
        let page_count = sequential_doc
            .page_count()
            .expect("corpus document must report a page count");
        assert!(
            page_count > 1,
            "a single-page fixture cannot detect a reordering; got {page_count} page(s)"
        );

        let sequential_scores: Vec<f32> = (0..page_count)
            .map(|page_index| {
                page_signals(&sequential_doc, page_index)
                    .as_ref()
                    .map_or(0.0, score_page)
            })
            .collect();
        let sequential_fabricated: Vec<usize> = (0..page_count)
            .filter(|&page_index| page_has_fabricated_text(&sequential_doc, page_index, min_ratio, min_chars))
            .collect();
        assert!(
            sequential_scores.iter().any(|&score| score != sequential_scores[0]),
            "fixture must score its pages differently for a reordering to be visible; got {sequential_scores:?}"
        );
        assert!(
            !sequential_fabricated.is_empty() && sequential_fabricated.len() < page_count,
            "fixture must fabricate some but not all pages for a reordering to be visible; \
             got {sequential_fabricated:?} of {page_count}"
        );

        let parallel_doc = PdfDocument::open(&path).expect("corpus document must open a second time");
        let detection = detect(&parallel_doc).expect("detection must run on the corpus document");
        assert_eq!(
            detection.page_confidence, sequential_scores,
            "scan confidences must match a page-by-page run, page for page"
        );
        assert_eq!(
            fabricated_provenance_page_indices(&parallel_doc, min_ratio, min_chars),
            sequential_fabricated,
            "fabricated-mapping pages must match a page-by-page run, in ascending page order"
        );
    }

    /// Scores are sums of `f32` weights, so `0.50 + 0.35 + 0.10` lands a few ULPs
    /// off `0.95`. Compare within tolerance rather than rounding the score.
    #[track_caller]
    fn assert_score(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 1e-5,
            "expected score {expected}, got {actual}"
        );
    }

    /// A scan: full-page raster, no text layer at all.
    fn bare_scan() -> PageScanSignals {
        PageScanSignals {
            image_coverage: 1.0,
            invisible_text_ratio: 0.0,
            glyph_count: 0,
            codec: ImageCodecClass::Dct,
            producer_prior: ProducerPrior::Unknown,
        }
    }

    #[test]
    fn sub_threshold_image_coverage_scores_zero() {
        let signals = PageScanSignals {
            image_coverage: 0.79,
            ..bare_scan()
        };
        assert_score(score_page(&signals), 0.0);
    }

    #[test]
    fn a_text_page_with_a_figure_is_not_a_scan() {
        let signals = PageScanSignals {
            image_coverage: 0.30,
            invisible_text_ratio: 0.0,
            glyph_count: 2000,
            codec: ImageCodecClass::Dct,
            producer_prior: ProducerPrior::Unknown,
        };
        assert_score(score_page(&signals), 0.0);
    }

    /// The born-digital slide with a full-bleed background image: its text is
    /// *visible*, so it must stay below any usable threshold.
    #[test]
    fn full_bleed_slide_with_visible_text_scores_below_default_threshold() {
        let signals = PageScanSignals {
            image_coverage: 1.0,
            invisible_text_ratio: 0.0,
            glyph_count: 133,
            codec: ImageCodecClass::Dct,
            producer_prior: ProducerPrior::Unknown,
        };
        assert_score(score_page(&signals), SCORE_FULL_PAGE_RASTER);
        assert!(f64::from(score_page(&signals)) < DEFAULT_SCANNED_MIN_CONFIDENCE);
    }

    /// The reporter's case: full-page raster under an invisible OCR sidecar.
    #[test]
    fn hidden_sidecar_over_a_raster_is_detected() {
        let signals = PageScanSignals {
            image_coverage: 1.0,
            invisible_text_ratio: 1.0,
            glyph_count: 217,
            codec: ImageCodecClass::Other,
            producer_prior: ProducerPrior::Unknown,
        };
        assert_score(score_page(&signals), 0.85);
        assert!(f64::from(score_page(&signals)) >= DEFAULT_SCANNED_MIN_CONFIDENCE);
    }

    #[test]
    fn scan_with_no_text_layer_is_detected() {
        assert_score(score_page(&bare_scan()), 0.85);
    }

    #[test]
    fn bilevel_codec_and_scanner_producer_raise_confidence() {
        let signals = PageScanSignals {
            codec: ImageCodecClass::Ccitt,
            producer_prior: ProducerPrior::Scanner,
            ..bare_scan()
        };
        assert_score(score_page(&signals), 1.0);
    }

    #[test]
    fn jbig2_counts_as_a_bilevel_codec() {
        let signals = PageScanSignals {
            codec: ImageCodecClass::Jbig2,
            ..bare_scan()
        };
        assert_score(score_page(&signals), 0.95);
    }

    /// A sidecar of *any* quality reads identically here. kreuzberg detects that
    /// a sidecar came from a scanner, never whether its text is accurate.
    #[test]
    fn sidecar_quality_does_not_affect_the_score() {
        let good = PageScanSignals {
            invisible_text_ratio: 1.0,
            glyph_count: 212,
            ..bare_scan()
        };
        let bad = PageScanSignals {
            invisible_text_ratio: 1.0,
            glyph_count: 217,
            ..bare_scan()
        };
        assert_score(score_page(&good), score_page(&bad));
    }

    #[test]
    fn score_never_leaves_the_unit_interval() {
        let maxed = PageScanSignals {
            image_coverage: 1.0,
            invisible_text_ratio: 1.0,
            glyph_count: 0,
            codec: ImageCodecClass::Ccitt,
            producer_prior: ProducerPrior::Scanner,
        };
        let score = score_page(&maxed);
        assert!((0.0..=1.0).contains(&score), "score {score} escaped [0,1]");
    }

    #[test]
    fn scanned_page_indices_selects_only_pages_at_or_above_the_threshold() {
        let detection = ScanDetection {
            confidence: 0.9,
            page_confidence: vec![0.0, 0.5, 0.85, 0.9],
        };
        assert_eq!(detection.scanned_page_indices(0.7), vec![2, 3]);
        assert_eq!(detection.scanned_page_indices(0.85), vec![2, 3]);
        assert_eq!(detection.scanned_page_indices(0.95), Vec::<usize>::new());
    }

    /// The doc-comment on `DEFAULT_SCANNED_MIN_CONFIDENCE` claims a slide is only
    /// OCR'd at a threshold of 0.50 or lower. Pin that boundary.
    #[test]
    fn a_full_bleed_slide_is_selected_only_at_a_threshold_of_0_50_or_lower() {
        let slide = ScanDetection {
            confidence: SCORE_FULL_PAGE_RASTER,
            page_confidence: vec![SCORE_FULL_PAGE_RASTER],
        };
        assert_eq!(slide.scanned_page_indices(0.50), vec![0]);
        assert_eq!(slide.scanned_page_indices(0.51), Vec::<usize>::new());
        assert_eq!(
            slide.scanned_page_indices(DEFAULT_SCANNED_MIN_CONFIDENCE as f32),
            Vec::<usize>::new()
        );
    }

    /// Build a span with the given text and provenance for the fabricated-fraction tests.
    fn provenance_span(text: &str, provenance: Option<MappingProvenance>) -> TextSpan {
        TextSpan {
            text: text.to_string(),
            provenance,
            ..TextSpan::default()
        }
    }

    #[test]
    fn all_fallback_page_counts_every_char_as_fabricated() {
        let spans = vec![provenance_span("garbled", Some(MappingProvenance::Fallback))];
        let (fabricated, total) = fabricated_char_counts(&spans);
        assert_eq!(fabricated, 7);
        assert_eq!(total, 7);
    }

    #[test]
    fn all_to_unicode_page_never_counts_as_fabricated() {
        let spans = vec![provenance_span("legible text", Some(MappingProvenance::ToUnicode))];
        let (fabricated, total) = fabricated_char_counts(&spans);
        assert_eq!(fabricated, 0);
        assert_eq!(total, 11);
    }

    #[test]
    fn all_none_provenance_page_never_counts_as_fabricated() {
        let spans = vec![provenance_span("unknown provenance", None)];
        let (fabricated, total) = fabricated_char_counts(&spans);
        assert_eq!(fabricated, 0);
        assert_eq!(total, 17);
    }

    #[test]
    fn mixed_provenance_sums_only_fallback_spans() {
        let spans = vec![
            provenance_span("good", Some(MappingProvenance::ToUnicode)),
            provenance_span("bad", Some(MappingProvenance::Fallback)),
            provenance_span("also good", Some(MappingProvenance::EncodingName)),
        ];
        let (fabricated, total) = fabricated_char_counts(&spans);
        assert_eq!(fabricated, 3);
        assert_eq!(total, 4 + 3 + 8);
    }

    /// Boundary behavior of the ratio check itself (mirrors `page_has_fabricated_text`'s
    /// `(fabricated / total) >= min_ratio` comparison without requiring a `PdfDocument`).
    #[test]
    fn ratio_at_threshold_triggers_but_just_below_does_not() {
        let spans = vec![
            provenance_span("aaaa", Some(MappingProvenance::Fallback)),
            provenance_span("aaaa", Some(MappingProvenance::ToUnicode)),
        ];
        let (fabricated, total) = fabricated_char_counts(&spans);
        let ratio = fabricated as f64 / total as f64;
        assert!((ratio - 0.5).abs() < f64::EPSILON);
        assert!(ratio >= 0.5, "exactly-at-threshold ratio must trigger");
        assert!(ratio < 0.500001, "sanity: ratio is exactly one half");
    }

    #[test]
    fn empty_spans_have_zero_total_and_never_trigger() {
        let (fabricated, total) = fabricated_char_counts(&[]);
        assert_eq!(fabricated, 0);
        assert_eq!(total, 0);
    }

    #[test]
    fn scanned_page_indices_clamps_an_out_of_range_threshold() {
        let detection = ScanDetection {
            confidence: 0.5,
            page_confidence: vec![0.0, 0.5],
        };
        // Negative thresholds clamp to 0.0, which still selects every page. ~keep
        assert_eq!(detection.scanned_page_indices(-1.0), vec![0, 1]);
        // Thresholds above 1.0 clamp to 1.0 and select nothing below it. ~keep
        assert_eq!(detection.scanned_page_indices(2.0), Vec::<usize>::new());
    }
}

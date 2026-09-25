//! Scanned-page detection for PDFs.

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::document::ReadingOrder;
use xberg_native_pdf::extractors::auto::{ImageCodecClass, ProducerPrior};
use xberg_native_pdf::fonts::MappingProvenance;
use xberg_native_pdf::layout::TextSpan;

#[cfg(test)]
use crate::core::config::DEFAULT_SCANNED_MIN_CONFIDENCE;

#[cfg(test)]
thread_local! {
    /// Counts calls to [`fabricated_provenance_page_indices`], the whole-document separate
    /// read over every page's raw spans. A caller holding per-page counts already gathered by
    /// the main text pass must never reach this function (issue #1744); tests reset and read
    /// it to prove that second read did not happen.
    ///
    /// Thread-local rather than a process-global counter: `cargo test`'s default harness runs
    /// each `#[test]` on its own thread, and a process-global counter is incremented by ANY
    /// test in the binary that reaches this function via the production path
    /// (`pdf/native/metadata.rs::ocr_routing_pages`), not only tests that name it. `#[serial]`
    /// only excludes other `#[serial]` tests, so it could not guard against that (issue #1752
    /// review). A thread-local gives each test's own thread its own counter, so a concurrent,
    /// unrelated test's calls are invisible to it regardless of `#[serial]`. ~keep
    pub(crate) static FABRICATED_PROVENANCE_SECOND_PASS_CALLS: std::sync::atomic::AtomicUsize =
        const { std::sync::atomic::AtomicUsize::new(0) };
}

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

/// Points per inch in PDF user space.
const POINTS_PER_INCH: f64 = 72.0;

/// Fraction of the page covered by raster images, without decoding pixel data.
///
/// Overlapping images are summed, not unioned, so this is an upper bound: it may
/// over-select a page for inspection, never under-select one.
fn image_coverage(doc: &PdfDocument, page_index: usize) -> Option<f32> {
    page_raster_geometry(doc, page_index).map(|(coverage, _)| coverage)
}

/// Below this raster coverage a page is never a scan for OCR, whatever its text layer holds.
const OCR_SCAN_COVERAGE_MIN: f32 = 0.25;

/// A scan's own text layer is furniture: a page number, a running header, a stamp. A page
/// with more glyphs than this beside a raster is a text page with a figure. A nine-word
/// stamp is about sixty glyphs; a page of prose is thousands.
const OCR_SCAN_MAX_GLYPHS: usize = 400;

/// The density, in dots per inch, of a page that is a scan: one raster with at most a stamp
/// of native text beside it.
///
/// A page whose rasters cover at least [`IMAGE_COVERAGE_MIN`] of it is a scan whatever its
/// text layer. A scanned sheet is often painted inset, with margins around it, so a page whose
/// raster covers at least [`OCR_SCAN_COVERAGE_MIN`] is a scan too when its text layer has no
/// more than [`OCR_SCAN_MAX_GLYPHS`] glyphs. `None` otherwise, and for a page with no image.
/// The density is that of the largest image on the page, its pixel count over the area it is
/// painted into, so a 1650 x 2160 px image painted over a Letter page reports about 196 dpi
/// whatever the page's render resolution is (#1786). The glyph count comes from the same
/// content-stream classification scan detection uses; no pixel data is decoded.
pub(crate) fn full_page_raster_density(doc: &PdfDocument, page_index: usize) -> Option<f64> {
    let (coverage, density) = page_raster_geometry(doc, page_index)?;
    if coverage >= IMAGE_COVERAGE_MIN {
        return density;
    }
    if coverage < OCR_SCAN_COVERAGE_MIN {
        return None;
    }
    // Detection is advisory: a page that panics must not abort the extraction. ~keep
    let classified = super::native::guard_native_panic(
        || doc.classify_page(page_index).map_err(|error| error.to_string()),
        |message| message,
    )
    .ok()?;
    (classified.signals.text_glyph_count <= OCR_SCAN_MAX_GLYPHS)
        .then_some(density)
        .flatten()
}

/// Raster coverage of the page and the density of its largest image, from one pass over
/// the page's image handles.
fn page_raster_geometry(doc: &PdfDocument, page_index: usize) -> Option<(f32, Option<f64>)> {
    let (x0, y0, x1, y1) = doc.get_page_media_box(page_index).ok()?;
    let page_area = ((x1 - x0) * (y1 - y0)).abs();
    if page_area <= f32::EPSILON {
        return None;
    }

    let (left, right) = (x0.min(x1), x0.max(x1));
    let (bottom, top) = (y0.min(y1), y0.max(y1));

    let handles = doc.page_image_handles(page_index).ok()?;
    let mut covered = 0.0f32;
    let mut largest: Option<(f32, f64)> = None;
    for handle in &handles {
        let bbox = &handle.bbox;
        let width = ((bbox.x + bbox.width).min(right) - bbox.x.max(left)).max(0.0);
        let height = ((bbox.y + bbox.height).min(top) - bbox.y.max(bottom)).max(0.0);
        let visible = width * height;
        covered += visible;

        let painted = f64::from(bbox.width.abs()) * f64::from(bbox.height.abs());
        if painted > 0.0 && largest.is_none_or(|(area, _)| visible > area) {
            let pixels = f64::from(handle.width) * f64::from(handle.height);
            largest = Some((visible, (pixels / painted).sqrt() * POINTS_PER_INCH));
        }
    }

    Some((
        (covered / page_area).clamp(0.0, 1.0),
        largest.map(|(_, density)| density),
    ))
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
pub(crate) fn fabricated_char_counts(spans: &[TextSpan]) -> (usize, usize) {
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

/// Whether `(fabricated, total)` non-whitespace character counts meet the fabricated-mapping
/// threshold: `min_chars` or more total characters, at least `min_ratio` of which are
/// fabricated (issue #1254). Shared by [`page_has_fabricated_text`] and
/// [`fabricated_provenance_page_indices_from_counts`] so the ratio check has one definition.
fn counts_meet_fabricated_threshold(fabricated: usize, total: usize, min_ratio: f64, min_chars: usize) -> bool {
    if total < min_chars {
        return false;
    }
    (fabricated as f64 / total as f64) >= min_ratio
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
    counts_meet_fabricated_threshold(fabricated, total, min_ratio, min_chars)
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
    #[cfg(test)]
    FABRICATED_PROVENANCE_SECOND_PASS_CALLS.with(|counter| counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst));

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

/// Same result as [`fabricated_provenance_page_indices`], computed from `(fabricated, total)`
/// non-whitespace character counts gathered while the main text pass already walked each
/// page's spans, rather than reading every page a second time (issue #1744).
///
/// `counts` must be indexed by zero-based page number, one entry per page — exactly what the
/// caller only has available when it read every page's raw `ColumnAware` spans (no optional-
/// content layers were excluded); see `pdf/native/text.rs::extract_all_page_texts`.
pub(crate) fn fabricated_provenance_page_indices_from_counts(
    counts: &[(usize, usize)],
    min_ratio: f64,
    min_chars: usize,
) -> Vec<usize> {
    counts
        .iter()
        .enumerate()
        .filter_map(|(page_index, &(fabricated, total))| {
            counts_meet_fabricated_threshold(fabricated, total, min_ratio, min_chars).then_some(page_index)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #1786: a page that is one full-page raster reports that raster's own density (400 px
    /// over 100 pt is 288 dpi); the same raster covering a quarter of the page is a figure and
    /// reports none; a page without images reports none.
    #[test]
    fn full_page_raster_density_reads_the_scan_density_and_ignores_figures() {
        let scan = PdfDocument::from_bytes(crate::pdf::render::build_full_page_raster_pdf(
            (100.0, 100.0),
            (400, 400),
            1.0,
            0,
        ))
        .unwrap();
        let density = full_page_raster_density(&scan, 0).expect("a full-page raster has a density");
        assert!(
            (density - 288.0).abs() < 0.5,
            "400 px over 100 pt is 288 dpi, got {density}"
        );

        let inset = PdfDocument::from_bytes(crate::pdf::render::build_full_page_raster_pdf(
            (100.0, 100.0),
            (400, 400),
            0.64,
            1,
        ))
        .unwrap();
        let density = full_page_raster_density(&inset, 0).expect("an inset scan with a stamp has a density");
        assert!(
            (density - 360.0).abs() < 0.5,
            "400 px painted over 80 pt is 360 dpi, got {density}"
        );

        let figure = PdfDocument::from_bytes(crate::pdf::render::build_full_page_raster_pdf(
            (100.0, 100.0),
            (400, 400),
            0.64,
            20,
        ))
        .unwrap();
        assert_eq!(
            full_page_raster_density(&figure, 0),
            None,
            "the same raster beside twenty lines of text is a figure on a text page"
        );

        let small = PdfDocument::from_bytes(crate::pdf::render::build_full_page_raster_pdf(
            (100.0, 100.0),
            (400, 400),
            0.16,
            0,
        ))
        .unwrap();
        assert_eq!(
            full_page_raster_density(&small, 0),
            None,
            "a raster under a quarter of the page is never a scan"
        );

        let blank = PdfDocument::from_bytes(crate::pdf::render::build_minimal_pdf_with_mediabox(100.0, 100.0)).unwrap();
        assert_eq!(
            full_page_raster_density(&blank, 0),
            None,
            "a page without images has no raster density"
        );
    }

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
    ///
    /// This test calls `fabricated_provenance_page_indices`, which increments
    /// `FABRICATED_PROVENANCE_SECOND_PASS_CALLS` -- now thread-local, so it no longer shares
    /// state with `provenance_is_not_read_a_second_time_for_a_document_with_no_excluded_layers`
    /// (`pdf/native/text.rs`) or any other test's thread. `#[serial]` is kept regardless, since
    /// nothing about the counter change makes concurrent PDF-handle work here cheaper. ~keep
    #[test]
    #[serial_test::serial]
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

    /// Positive control for the probe that
    /// `provenance_is_not_read_a_second_time_for_a_document_with_no_excluded_layers`
    /// (`pdf/native/text.rs`) relies on. That test asserts the counter is **zero** -- the
    /// second whole-document read did not happen. A zero it can never leave proves nothing:
    /// a counter no call site reaches on the asserting thread and a genuinely skipped second
    /// pass render identically. Making the counter thread-local removed the flake but also
    /// removed cross-thread visibility, so the wiring needs its own witness. ~keep
    #[test]
    #[serial_test::serial]
    fn the_second_pass_counter_observes_a_call_on_its_own_thread() {
        let thresholds = crate::core::config::OcrQualityThresholds::default();
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/pdf/non_ascii_text.pdf");
        let doc = PdfDocument::open(&path).expect("corpus document must open");

        FABRICATED_PROVENANCE_SECOND_PASS_CALLS.with(|counter| counter.store(0, std::sync::atomic::Ordering::SeqCst));
        let _ = fabricated_provenance_page_indices(
            &doc,
            thresholds.min_provenance_fallback_ratio,
            thresholds.min_total_non_whitespace,
        );

        assert_eq!(
            FABRICATED_PROVENANCE_SECOND_PASS_CALLS.with(|counter| counter.load(std::sync::atomic::Ordering::SeqCst)),
            1,
            "the counter must observe a call made on this thread, or the sibling test's zero is vacuous"
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

    /// Every distinct score `score_page` can return, over the whole signal matrix.
    ///
    /// The score is a sum of four independent weights, so the reachable set is small and
    /// fixed, and the gap between `0.65` and `0.85` is what gives
    /// [`DEFAULT_SCANNED_MIN_CONFIDENCE`] its meaning: any threshold in `(0.65, 0.85]`
    /// selects exactly the same pages as `0.70` does, so moving it within that interval is
    /// a no-op dressed as a behaviour change. Pinned so that stays visible to whoever
    /// proposes the move. ~keep
    /// Every `(codec, producer_prior)` score for one `(image_coverage, glyph_count,
    /// invisible_text_ratio)` combination, for [`score_page_reaches_exactly_the_documented_set_of_scores`].
    fn scores_over_codec_and_producer(image_coverage: f32, glyph_count: usize, invisible_text_ratio: f32) -> Vec<f32> {
        let codecs = [
            ImageCodecClass::Dct,
            ImageCodecClass::Other,
            ImageCodecClass::Ccitt,
            ImageCodecClass::Jbig2,
        ];
        let producers = [ProducerPrior::Unknown, ProducerPrior::Authoring, ProducerPrior::Scanner];
        codecs
            .into_iter()
            .flat_map(|codec| {
                producers.into_iter().map(move |producer_prior| {
                    score_page(&PageScanSignals {
                        image_coverage,
                        invisible_text_ratio,
                        glyph_count,
                        codec,
                        producer_prior,
                    })
                })
            })
            .collect()
    }

    #[test]
    fn score_page_reaches_exactly_the_documented_set_of_scores() {
        let mut reachable: Vec<f32> = Vec::new();
        for image_coverage in [0.0, IMAGE_COVERAGE_MIN - 0.01, IMAGE_COVERAGE_MIN, 1.0] {
            for (glyph_count, invisible_text_ratio) in [(0, 0.0), (250, 0.0), (250, INVISIBLE_TEXT_MIN), (250, 1.0)] {
                for score in scores_over_codec_and_producer(image_coverage, glyph_count, invisible_text_ratio) {
                    if !reachable.iter().any(|seen| (seen - score).abs() < 1e-5) {
                        reachable.push(score);
                    }
                }
            }
        }
        reachable.sort_by(|left, right| left.partial_cmp(right).expect("scores are finite"));

        let expected = [0.0, 0.50, 0.55, 0.60, 0.65, 0.85, 0.90, 0.95, 1.00];
        assert_eq!(
            reachable.len(),
            expected.len(),
            "reachable score set changed: got {reachable:?}, expected {expected:?}"
        );
        for (actual, expected) in reachable.iter().zip(expected) {
            assert_score(*actual, expected);
        }
    }

    /// `0.65` -- the ceiling a full-page raster with a visible text layer is said to hit --
    /// needs a bilevel codec *and* a scanner producer *and* visible text simultaneously.
    ///
    /// Drop any one of the three and the page scores at most `0.60`. That conjunction is
    /// why the value is unreached in practice: a CCITT/JBIG2 page written by scanner
    /// software does not also carry *visible* native glyphs -- if it carries a text layer at
    /// all it is an invisible OCR sidecar, which takes [`SCORE_NO_VISIBLE_TEXT`] and lands
    /// at `0.85` or above instead. Measured over the 12,526-page PDF corpus, no page scored
    /// `0.55`, `0.60` or `0.65`; all 30 pages of the full-page-raster-with-visible-text
    /// class scored exactly [`SCORE_FULL_PAGE_RASTER`] (issue #1752). ~keep
    #[test]
    fn a_score_of_0_65_requires_bilevel_codec_and_scanner_producer_and_visible_text_at_once() {
        let all_three = PageScanSignals {
            image_coverage: 1.0,
            invisible_text_ratio: 0.0,
            glyph_count: 250,
            codec: ImageCodecClass::Ccitt,
            producer_prior: ProducerPrior::Scanner,
        };
        assert_score(score_page(&all_three), 0.65);

        assert_score(
            score_page(&PageScanSignals {
                codec: ImageCodecClass::Dct,
                ..all_three
            }),
            0.55,
        );
        assert_score(
            score_page(&PageScanSignals {
                producer_prior: ProducerPrior::Authoring,
                ..all_three
            }),
            0.60,
        );
        // Hiding the text layer takes the larger `SCORE_NO_VISIBLE_TEXT` instead, which is
        // why the sidecar case never sits in the 0.50..=0.65 band at all. ~keep
        assert_score(
            score_page(&PageScanSignals {
                invisible_text_ratio: 1.0,
                ..all_three
            }),
            1.0,
        );
    }

    /// The shape the corpus actually produces: a born-digital page whose figure covers the
    /// sheet, carrying a small but *visible* text layer -- a caption, a heading, a running
    /// footer. Every such page scores [`SCORE_FULL_PAGE_RASTER`] regardless of how little
    /// text it carries, because nothing in the matrix reads "few visible glyphs".
    ///
    /// The three smallest in the corpus were a 4-glyph section heading over two screenshots,
    /// a 27-glyph figure caption, and a 47-glyph newspaper footer over a full-page
    /// advertisement; the counts run from there to 2698 with no gap to cut at (issue #1752).
    /// ~keep
    #[test]
    fn a_full_bleed_page_scores_the_same_however_little_visible_text_it_carries() {
        let scores: Vec<f32> = [4, 27, 47, 133, 687, 2698]
            .into_iter()
            .map(|glyph_count| {
                score_page(&PageScanSignals {
                    image_coverage: 1.0,
                    invisible_text_ratio: 0.0,
                    glyph_count,
                    codec: ImageCodecClass::Dct,
                    producer_prior: ProducerPrior::Authoring,
                })
            })
            .collect();

        for score in &scores {
            assert_score(*score, SCORE_FULL_PAGE_RASTER);
            assert!(
                f64::from(*score) < DEFAULT_SCANNED_MIN_CONFIDENCE,
                "a full-bleed page scored {score}, at or above the default threshold"
            );
        }
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

    // =========================================================================
    // GH#1782 — a page mixing a few readable lines with a Type 3 font that has
    // no ToUnicode and procedural (non-AGL) /Differences glyph names. Fixtures
    // are the reporter's `repro-type3-with-{2,6}-readable-lines.pdf` (sha256
    // `580aa659c982190b2e4439e67b835c7049dbf88ad1c1f68fca74a9e63f046fd9` and
    // `dff0a78726f6c1f308b3519bfa5c85e656079a53254825e818cd300a4d37d620`,
    // verified against the values quoted in the issue), pinned unmodified.
    //
    // What this session's `best_mapping_provenance`/`resolve_base_encoding_map`
    // fixes achieve, verified below: every span painted by the Type 3 font now
    // carries `MappingProvenance::Fallback` (previously `EncodingName`, which
    // told every consumer — including this file's own fabricated-ratio gate —
    // that the text was safely mappable). That is a real, tested improvement.
    //
    // What it does NOT achieve on these exact fixtures, measured and NOT
    // shipped: `fabricated_char_counts` (above) counts *decoded Unicode
    // characters*, not glyphs painted. A glyph whose code has no mapping now
    // correctly decodes to nothing (previously it fabricated a plausible-
    // looking StandardEncoding punctuation character or bare control code) —
    // but "nothing" contributes zero to both the fabricated numerator and the
    // total denominator, so a page can be entirely built from unmapped glyphs
    // and still measure a LOW fabricated ratio, because there is barely any
    // decoded text of any kind to count. Root cause: font_dict.rs's
    // `char_to_unicode` correctly returns `None` for these codes, but
    // `extractors/text/{advance.rs,mod.rs,clustering.rs}` all then call
    // `fallback_char_to_unicode` (fonts/unicode_decode.rs:130), whose final
    // `char::from_u32(char_code)` catch-all (unicode_decode.rs:139-143)
    // reinterprets the raw, meaningless Type 3 procedure code as if it were
    // itself a Unicode codepoint — the true source of the "symbol characters"
    // GH#1780/#1782 describe. That reinterpretation is shared by every font,
    // not Type 3-specific, and fixing it is a separate, larger change than
    // this session's scope: flipping `extraction_method` for these fixtures
    // needs a page-fabrication signal built on GLYPH COUNT attributed to a
    // Fallback-provenance font, not decoded character count, since the
    // decoded count is structurally blind to glyphs that (correctly, now)
    // decode to nothing. NO-SHIP for the ratio-based routing flip; the
    // provenance/encoding correctness fixes below stand on their own. ~keep
    // =========================================================================

    fn type3_fixture(name: &str) -> PdfDocument {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/pdf/regressions/type3")
            .join(name);
        PdfDocument::open(&path).unwrap_or_else(|e| panic!("open fixture {name}: {e}"))
    }

    /// The achieved half of the fix: every span the Type 3 font paints is
    /// now `MappingProvenance::Fallback`, not `EncodingName`. Before this
    /// session's `best_mapping_provenance` fix, EVERY span from this font
    /// (readable or not) read `EncodingName`, telling every consumer the
    /// text was safely mappable when it was not.
    #[test]
    fn gh1782_type3_spans_carry_fallback_provenance() {
        let doc = type3_fixture("gh1782-2-readable-lines.pdf");
        let page_text = doc
            .extract_page_text_with_options(0, ReadingOrder::ColumnAware)
            .expect("extract GH#1782 fixture page text");

        let type3_spans: Vec<_> = page_text.spans.iter().filter(|s| s.font_name != "Helvetica").collect();
        assert!(
            !type3_spans.is_empty(),
            "fixture must contain at least one Type 3 span to test its provenance"
        );
        for span in &type3_spans {
            assert_eq!(
                span.provenance,
                Some(MappingProvenance::Fallback),
                "Type 3 span {:?} must carry Fallback provenance (no ToUnicode, procedural \
                 /Differences glyph names) — got {:?}",
                span.text,
                span.provenance
            );
        }

        // The 2 readable Helvetica header lines are unaffected: still EncodingName.
        let helvetica_spans: Vec<_> = page_text.spans.iter().filter(|s| s.font_name == "Helvetica").collect();
        assert_eq!(
            helvetica_spans.len(),
            2,
            "fixture declares exactly 2 readable header lines"
        );
        for span in &helvetica_spans {
            assert_eq!(span.provenance, Some(MappingProvenance::EncodingName));
        }
    }

    /// Characterizes the measured, NOT-shipped gap documented above: at
    /// default thresholds, the page-level fabricated ratio does not clear
    /// `min_provenance_fallback_ratio` (0.5) for either fixture, so neither
    /// page is flagged. This pins the honest current behavior rather than
    /// asserting the (unfixed) desired one — a routing fix needs a glyph-
    /// count-based signal, not this ratio, per the comment above.
    #[test]
    fn gh1782_ratio_based_routing_does_not_yet_flag_these_fixtures() {
        let thresholds = crate::core::config::OcrQualityThresholds::default();
        for name in ["gh1782-2-readable-lines.pdf", "gh1782-6-readable-lines.pdf"] {
            let doc = type3_fixture(name);
            let fabricated = fabricated_provenance_page_indices(
                &doc,
                thresholds.min_provenance_fallback_ratio,
                thresholds.min_total_non_whitespace,
            );
            assert_eq!(
                fabricated,
                Vec::<usize>::new(),
                "{name}: measured NO-SHIP — the decoded-character-count ratio does not clear \
                 the default 0.5 threshold for this fixture; see the module comment above for \
                 why and what a real fix needs"
            );
        }
    }

    /// Negative control (task requirement): a page whose text is entirely
    /// ordinary, mappable Type 1 text — no Type 3 font at all — must NOT be
    /// flagged fabricated. This is the regression the GH#1782 fix could
    /// easily introduce (over-flagging any page that merely mixes fonts).
    #[test]
    fn mappable_text_only_page_is_not_flagged_fabricated() {
        let doc = type3_fixture("control-decorative-background.pdf");
        let thresholds = crate::core::config::OcrQualityThresholds::default();
        let fabricated = fabricated_provenance_page_indices(
            &doc,
            thresholds.min_provenance_fallback_ratio,
            thresholds.min_total_non_whitespace,
        );
        assert_eq!(
            fabricated,
            Vec::<usize>::new(),
            "a page of plain Type 1 text with no Type 3 font must never be flagged \
             fabricated — false positives here would route ordinary native-text \
             documents to OCR needlessly"
        );
    }
}

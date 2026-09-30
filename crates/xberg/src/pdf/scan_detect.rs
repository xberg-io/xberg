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

/// Below this raster coverage, no image on the page can plausibly carry its content at any
/// size, so scoring is skipped outright (performance floor only -- see [`page_signals`]).
/// Set below the lowest coverage GH#1793 measured on a real image-only page (22%, 112 pages,
/// median 66%) with a comfortable margin, so a genuine partial scan is never skipped before
/// its text layer is inspected. The real scan/not-a-scan decision for coverage between this
/// floor and [`IMAGE_COVERAGE_FULL`] is made by [`is_header_sized_text`], not by this value. ~keep
const IMAGE_COVERAGE_MIN: f32 = 0.15;

/// At or above this coverage, the image spans (near) the whole visible page and
/// [`SCORE_FULL_PAGE_RASTER`] applies unconditionally, whether or not native text sits over
/// it (GH#1779: a decorative full-bleed background under a real page of text must still
/// register this much suspicion -- just not enough to clear the default threshold on its
/// own, since it also fails [`is_header_sized_text`]). Between [`IMAGE_COVERAGE_MIN`] and
/// this value, the same base score applies only when the native text IS header-sized, or the
/// page is body text with a figure (GH#1793's `control-native-text-with-figure`), not a scan. ~keep
const IMAGE_COVERAGE_FULL: f32 = 0.80;

/// Fraction of glyphs in render mode 3 (invisible) that marks an OCR sidecar.
const INVISIBLE_TEXT_MIN: f32 = 0.50;

/// Max fraction of the page a header/footer-style text layer may cover before it counts as
/// substantive content rather than a scan's running header, footer, or stamp.
///
/// Measured, not assumed (`cargo test gh1793_probe` while developing this fix, since the
/// naive guess of using `xberg_native_pdf`'s own `sparse_text_max` calibration (0.10) turned
/// out to sit on the wrong side of a real control): GH#1779's reproducer (an 8-word header and
/// footer) measures `text_area_ratio` 0.0043 and GH#1752's reproducer (a 9-character corner
/// stamp) measures 0.0011, while GH#1793's `control-native-text-with-figure` (12 lines of real
/// body text next to a figure, the case this constant must NOT flag) measures 0.087 and
/// GH#1779's decorative-background control (30 lines) measures 0.256. `0.02` sits with a
/// comfortable order-of-magnitude margin on both sides of that gap. ~keep
const TEXT_AREA_HEADER_MAX: f32 = 0.02;

/// A full-page raster alone. Below every usable threshold: a slide with a
/// full-bleed background image scores exactly this.
const SCORE_FULL_PAGE_RASTER: f32 = 0.50;

/// Added when the text layer is hidden, absent, or accounts for only a header/footer-sized
/// share of the page (see [`is_header_sized_text`]) -- none of which is content substantial
/// enough to make the page anything other than a scan.
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
    /// Sum of native text bounding-box area intersected with the page, over page area, in
    /// `[0, 1]` (`PageSignals::text_area_ratio` from `xberg_native_pdf`). The discriminator
    /// between a scan's header/footer/stamp and a real page of native text: a header covers a
    /// tiny fraction of the page regardless of how many glyphs it has, while a real page of
    /// text covers a substantial fraction regardless of how little image sits under it. See
    /// [`is_header_sized_text`]. ~keep
    pub text_area_ratio: f32,
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

/// Whether `signals`'s native text layer is no bigger than a header, footer, or stamp: either
/// there is none at all, or what there is covers at most [`TEXT_AREA_HEADER_MAX`] of the page.
/// This is an AREA question, not a count question -- GH#1793's real pages carried 41-92 native
/// glyphs each and were still header-sized, while GH#1779's decorative-background control is
/// header-sized by neither measure. A page failing this check has native text that accounts
/// for real page content, not a label on top of a scan. ~keep
fn is_header_sized_text(signals: &PageScanSignals) -> bool {
    signals.glyph_count == 0 || signals.text_area_ratio <= TEXT_AREA_HEADER_MAX
}

/// Grade one page's evidence. Pure, so it is testable without a [`PdfDocument`].
pub(crate) fn score_page(signals: &PageScanSignals) -> f32 {
    if signals.image_coverage < IMAGE_COVERAGE_MIN {
        return 0.0;
    }

    let full_page = signals.image_coverage >= IMAGE_COVERAGE_FULL;
    let header_sized_text = is_header_sized_text(signals);

    // Below full-page coverage, only a page whose native text is header-sized can be a
    // (partial-coverage) scan at all -- GH#1793. A page with substantial native text next to
    // a sub-full-page image is body text with a figure, at any coverage (GH#1793's
    // `control-native-text-with-figure`, GH#1752 review's `a_text_page_with_a_figure_is_not_a_scan`).
    if !full_page && !header_sized_text {
        return 0.0;
    }

    let mut score = SCORE_FULL_PAGE_RASTER;

    if header_sized_text || signals.invisible_text_ratio >= INVISIBLE_TEXT_MIN {
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
// GH#1835: the OCR scan-density policy below is consumed by `extractors::pdf::ocr::rendering`
// (`any(ocr, ocr-pipeline)`) and by `image::dpi::pdf_ocr_render_dpi`, whose module is gated
// `any(ocr-pipeline, layout-detection)`. Gating these on `pdf` alone, as the whole file is,
// left them with no caller on a `pdf`-only build and broke `-D warnings` there. ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline", feature = "layout-detection"))]
const OCR_SCAN_COVERAGE_MIN: f32 = 0.25;

/// A scan's own text layer is furniture: a page number, a running header, a stamp. A page
/// with more glyphs than this beside a raster is a text page with a figure. A nine-word
/// stamp is about sixty glyphs; a page of prose is thousands.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline", feature = "layout-detection"))]
const OCR_SCAN_MAX_GLYPHS: usize = 400;

/// The density, in dots per inch, of a page that is a scan: one raster with at most a stamp
/// of native text beside it.
///
/// A page whose rasters cover at least [`IMAGE_COVERAGE_FULL`] of it is a scan whatever its
/// text layer. A scanned sheet is often painted inset, with margins around it, so a page whose
/// raster covers at least [`OCR_SCAN_COVERAGE_MIN`] is a scan too when its text layer has no
/// more than [`OCR_SCAN_MAX_GLYPHS`] readable glyphs. Glyphs whose mapping provenance is
/// [`MappingProvenance::Fallback`] do not count as readable because they may be a broken OCR
/// sidecar rather than real native text. `None` otherwise, and for a page with no image.
/// The density is that of the largest image on the page, its pixel count over the area it is
/// painted into, so a 1650 x 2160 px image painted over a Letter page reports about 196 dpi
/// whatever the page's render resolution is (#1786). No pixel data is decoded.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline", feature = "layout-detection"))]
pub(crate) fn full_page_raster_density(doc: &PdfDocument, page_index: usize) -> Option<f64> {
    let (coverage, density) = page_raster_geometry(doc, page_index)?;
    // GH#1786 review: this is the whole-page threshold (0.80), NOT `IMAGE_COVERAGE_MIN` (0.15),
    // which is only the floor below which scoring is skipped for cost. Gating here on the floor
    // returned every page with 15% raster as a scan and made the glyph check below unreachable,
    // since reaching it then required `coverage < 0.15`, under which `OCR_SCAN_COVERAGE_MIN`
    // always rejects. ~keep
    if coverage >= IMAGE_COVERAGE_FULL {
        return density;
    }
    if coverage < OCR_SCAN_COVERAGE_MIN {
        return None;
    }
    // Detection is advisory: a page that panics must not abort the extraction. ~keep
    let page_text = super::native::guard_native_panic(
        || {
            doc.extract_page_text_with_options(page_index, ReadingOrder::ColumnAware)
                .map_err(|error| error.to_string())
        },
        |message| message,
    )
    .ok()?;
    (readable_glyph_count(&page_text.spans) <= OCR_SCAN_MAX_GLYPHS)
        .then_some(density)
        .flatten()
}

/// Whether the page's rasters cover at least [`OCR_SCAN_COVERAGE_MIN`] of it, the least a scan
/// covers, inset or whole-page. The text layer is not consulted: a page that carries a scan is
/// a scan to the OCR engine whatever text is drawn over it. No pixel data is decoded.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(crate) fn carries_scan_raster(doc: &PdfDocument, page_index: usize) -> bool {
    image_coverage(doc, page_index).is_some_and(|coverage| coverage >= OCR_SCAN_COVERAGE_MIN)
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline", feature = "layout-detection"))]
fn readable_glyph_count(spans: &[TextSpan]) -> usize {
    spans
        .iter()
        .filter(|span| span.provenance != Some(MappingProvenance::Fallback))
        .map(|span| span.text.chars().count())
        .sum()
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
        text_area_ratio: signals.text_area_ratio,
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
mod tests;

#[cfg(test)]
#[path = "scan_detect_type3_tests.rs"]
mod type3_tests;

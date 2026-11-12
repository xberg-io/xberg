//! One input per check of the tall-scan strip certificate (issue #2911), plus inputs that
//! separate the overlap check from the strip-bottom check.
//!
//! The `crates/xberg/tests/` suites cannot reach the certificate: both strip recovery routes
//! gate the attempt on the builtin Tesseract instance, so a registered custom backend is never
//! handed a strip through the public API. Like `tall_source::neutral_rgb_tests`, these tests
//! drive `try_replace` directly, as a child module of `tall_source`.

use super::strip_fixture::{
    BILEVEL, GRAY, HEIGHT, INK_COLUMNS, INK_ROWS, LINE_PITCH, WIDTH, bilevel_samples, gray_samples, ink_runs,
    is_ink_row, recognized, scan_pdf_at, source_lines,
};
use super::try_replace;
use crate::core::config::{ExtractionConfig, OcrConfig};
use crate::ocr_metadata_keys::{OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY, OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY};
use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
use crate::types::{ExtractedDocument, OcrBoundingGeometry, OcrConfidence, OcrElement};
use std::borrow::Cow;
use std::ops::Range;
use std::sync::{Arc, Mutex};
use xberg_native_pdf::PdfDocument;

const LINE_TEXT: &str = "alpha bravo";
/// The read the page keeps before the strips: fewer, worse words than the strips recover.
const PRIOR_READ: (&str, f64, f64, u32) = ("one two three", 80.0, 0.1, 3);
/// A read the recognition-noise check rejects: every word is shorter than three characters.
const NOISY_READ: (&str, f64, f64, u32) = (
    "ab cd ef gh ij kl mn op qr st ab cd ef gh ij kl mn op qr st ab cd ef gh ij kl",
    60.0,
    0.0,
    30,
);
/// The processed frame the prior read claims: 1100 of 1200 px stays below the 98 percent
/// refusal, and its aspect error stays within a pixel of the rendered frame.
const PROCESSED_READ: (u32, u32) = (1100, 4033);
/// Half of the blank gap between two lines: a line dropped this far sits outside its ink band
/// without reaching the neighbouring band or the strip bottom.
const LINE_DISPLACEMENT: u32 = (LINE_PITCH - (INK_ROWS.end - INK_ROWS.start)) / 2;
/// Rows a last line hangs past the strip bottom: fewer than the blank rows a midpoint cut
/// leaves under a strip's last band.
const LINE_OVERHANG: u32 = LINE_DISPLACEMENT / 3;
/// Rows the first line of a strip with a following run extends past its own band: one row
/// into the next band, across the blank rows that separate two fixture bands.
const LINE_REACH: u32 = LINE_PITCH - (INK_ROWS.end - INK_ROWS.start) + 1;
/// A dictionary-invalid ratio above the prior read's, but under the complete-rows path's
/// ceiling, so only the certified-rows flag can decide the replacement.
const CLEAN_PATH_INVALID_RATIO: f64 = 0.04;

/// What the backend reports about the lines of each strip it is given.
#[derive(Clone, Copy, PartialEq)]
enum Lines {
    /// One line element per run of ink rows, exactly where the strip has them.
    Exact,
    /// No line elements, while the processed size is present.
    NoElements,
    /// The first line element's text differs from its content line.
    MismatchedText,
    /// The content carries one text line no element accounts for.
    ExtraTextLine,
    /// The processed height is one row short of the strip.
    WrongHeight,
    /// The processed width is one column short of the strip.
    WrongWidth,
    /// The first line of each strip is dropped into the blank gap below its ink band.
    DisplacedFirst,
    /// The last line of each strip hangs past the strip bottom.
    OverhangingLast,
    /// The first line of each strip with a following run keeps its own band's top and
    /// reaches into the next ink band.
    BandReach,
    /// The first line of each strip reports a right edge past the strip width.
    OverWide,
    /// Exact lines at the clean-path ratio: the complete-rows evidence carries the replacement.
    CertifiedCleanPath,
    /// Omitted last lines at the clean-path ratio: the uncertified strips withhold that evidence.
    UncertifiedCompleteRows,
}

struct CertificateBackend {
    lines: Lines,
    strip_heights: Arc<Mutex<Vec<u32>>>,
}

impl Plugin for CertificateBackend {
    fn name(&self) -> &str {
        "strip-certificate-inputs"
    }
    fn version(&self) -> String {
        "1.0.0".to_owned()
    }
    fn initialize(&self) -> crate::Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> crate::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl OcrBackend for CertificateBackend {
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    async fn process_image(&self, image_bytes: &[u8], _config: &OcrConfig) -> crate::Result<ExtractedDocument> {
        let strip = image::load_from_memory(image_bytes)
            .expect("a decodable strip PNG")
            .to_luma8();
        self.strip_heights.lock().unwrap().push(strip.height());
        Ok(strip_report(&strip, self.lines))
    }
}

/// The page's prior read from its render, with the processed frame that admits it.
fn page_read(lines: Lines) -> ExtractedDocument {
    let (content, mean, invalid, words) = if matches!(lines, Lines::CertifiedCleanPath | Lines::UncertifiedCompleteRows)
    {
        NOISY_READ
    } else {
        PRIOR_READ
    };
    let mut original = recognized(content, mean, invalid, words);
    original.metadata.additional.extend([
        (
            Cow::Borrowed(OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY),
            PROCESSED_READ.0.into(),
        ),
        (
            Cow::Borrowed(OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY),
            PROCESSED_READ.1.into(),
        ),
    ]);
    original
}

fn strip_report(strip: &image::GrayImage, lines: Lines) -> ExtractedDocument {
    let (width, height) = strip.dimensions();
    let mut runs = ink_runs(strip);
    let (extra_text_line, invalid) = match lines {
        Lines::ExtraTextLine => (true, 0.0),
        Lines::CertifiedCleanPath | Lines::UncertifiedCompleteRows => (false, CLEAN_PATH_INVALID_RATIO),
        _ => (false, 0.0),
    };
    if matches!(lines, Lines::UncertifiedCompleteRows) {
        runs.pop();
    }
    let mut text: Vec<&str> = runs.iter().map(|_| LINE_TEXT).collect();
    if extra_text_line {
        text.push("uncertified tail");
    }
    let mut result = recognized(&text.join("\n"), 95.0, invalid, 2 * runs.len() as u32);
    if lines != Lines::NoElements {
        result.ocr_elements = Some(line_elements(&runs, height, lines));
    }
    let reported_width = if lines == Lines::WrongWidth { width - 1 } else { width };
    let reported_height = if lines == Lines::WrongHeight {
        height - 1
    } else {
        height
    };
    result.metadata.additional.extend([
        (
            Cow::Borrowed(OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY),
            reported_width.into(),
        ),
        (
            Cow::Borrowed(OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY),
            reported_height.into(),
        ),
    ]);
    result
}

fn line_elements(runs: &[Range<u32>], strip_height: u32, lines: Lines) -> Vec<OcrElement> {
    runs.iter()
        .enumerate()
        .map(|(index, run)| {
            let (top, line_height) = if lines == Lines::OverhangingLast && index + 1 == runs.len() {
                (run.start, strip_height - run.start + LINE_OVERHANG)
            } else if lines == Lines::BandReach && index == 0 && runs.len() > 1 {
                (run.start, run.end - run.start + LINE_REACH)
            } else {
                let displaced = lines == Lines::DisplacedFirst && index == 0;
                (
                    run.start + u32::from(displaced) * LINE_DISPLACEMENT,
                    run.end - run.start,
                )
            };
            let mismatched = lines == Lines::MismatchedText && index == 0;
            let overwide = lines == Lines::OverWide && index == 0;
            OcrElement::new(
                if mismatched { "alpha delta" } else { LINE_TEXT },
                OcrBoundingGeometry::Rectangle {
                    left: INK_COLUMNS.start as u32,
                    top,
                    width: if overwide {
                        WIDTH
                    } else {
                        (INK_COLUMNS.end - INK_COLUMNS.start) as u32
                    },
                    height: line_height,
                },
                OcrConfidence {
                    detection: None,
                    recognition: 0.95,
                },
            )
        })
        .collect()
}

/// Run `try_replace` for one variant. The page was read from a render the size of the source,
/// which the prior read reports as the narrower `PROCESSED_READ` frame.
async fn replace(document: &PdfDocument, lines: Lines) -> (bool, ExtractedDocument, Vec<u32>) {
    let strip_heights = Arc::new(Mutex::new(Vec::new()));
    let backend: Arc<dyn OcrBackend> = Arc::new(CertificateBackend {
        lines,
        strip_heights: Arc::clone(&strip_heights),
    });
    let mut original = page_read(lines);
    let page = (0, Arc::new(vec![0; 1000]), WIDTH, HEIGHT);
    let replaced = try_replace(
        &mut original,
        &backend,
        &ExtractionConfig::default(),
        &OcrConfig::default(),
        (document, 0),
        &page,
    )
    .await
    .expect("try_replace runs");
    let strip_heights = strip_heights.lock().unwrap().clone();
    (replaced, original, strip_heights)
}

fn used_strips(original: &ExtractedDocument) -> bool {
    original
        .processing_warnings
        .iter()
        .any(|warning| warning.message.contains("source-image strips."))
}

fn reported_lines(original: &ExtractedDocument) -> usize {
    original.content.lines().filter(|line| !line.trim().is_empty()).count()
}

/// A gray scan just under the standard density floor, so any uncertified strip refuses it.
fn uncertifiable_gray_scan() -> PdfDocument {
    scan_pdf_at(GRAY, &gray_samples(is_ink_row), 144.0)
}

/// A bilevel scan at the floor: certified rows are requested, but a failing certificate only
/// withholds the complete-rows evidence.
fn certifiable_bilevel_scan() -> PdfDocument {
    scan_pdf_at(BILEVEL, &bilevel_samples(is_ink_row), 150.0)
}

#[tokio::test]
async fn exact_line_reports_certify_every_strip_and_replace_the_page_read() {
    let document = uncertifiable_gray_scan();
    let (replaced, original, strip_heights) = replace(&document, Lines::Exact).await;
    assert!(replaced);
    assert!(used_strips(&original));
    assert_eq!(reported_lines(&original), source_lines(is_ink_row));
    assert!(
        strip_heights.len() >= 2,
        "the scan is cut into strips: {strip_heights:?}"
    );
}

/// A strip whose backend reports a processed size but no line elements is refused. Both the
/// certificate's elements guard and its final line pairing decline such a strip, so this
/// input pins the refusal of strips without elements, not one clause of it.
#[tokio::test]
async fn a_strip_reporting_a_processed_size_but_no_line_elements_is_refused() {
    let document = uncertifiable_gray_scan();
    let (replaced, original, strip_heights) = replace(&document, Lines::NoElements).await;
    assert!(!replaced);
    assert!(!used_strips(&original));
    assert_eq!(original.content, PRIOR_READ.0);
    assert!(
        !strip_heights.is_empty(),
        "the backend ran on strips: {strip_heights:?}"
    );
}

#[tokio::test]
async fn a_line_element_whose_text_differs_from_its_content_line_refuses_the_strip() {
    let document = uncertifiable_gray_scan();
    let (replaced, original, strip_heights) = replace(&document, Lines::MismatchedText).await;
    assert!(!replaced);
    assert!(!used_strips(&original));
    assert_eq!(original.content, PRIOR_READ.0);
    assert!(
        !strip_heights.is_empty(),
        "the backend ran on strips: {strip_heights:?}"
    );
}

#[tokio::test]
async fn a_leftover_text_line_without_an_element_refuses_the_strip() {
    let document = uncertifiable_gray_scan();
    let (replaced, original, strip_heights) = replace(&document, Lines::ExtraTextLine).await;
    assert!(!replaced);
    assert!(!used_strips(&original));
    assert_eq!(original.content, PRIOR_READ.0);
    assert!(
        !strip_heights.is_empty(),
        "the backend ran on strips: {strip_heights:?}"
    );
}

/// The processed height is one row short of the strip: only the height comparison refuses
/// it, so removing the width comparison must not certify this input.
#[tokio::test]
async fn a_strip_reporting_a_wrong_processed_height_refuses_the_strip() {
    let document = uncertifiable_gray_scan();
    let (replaced, original, strip_heights) = replace(&document, Lines::WrongHeight).await;
    assert!(!replaced);
    assert!(!used_strips(&original));
    assert_eq!(original.content, PRIOR_READ.0);
    assert!(
        !strip_heights.is_empty(),
        "the backend ran on strips: {strip_heights:?}"
    );
}

/// The processed width is one column short of the strip: only the width comparison refuses
/// it, so removing the height comparison must not certify this input.
#[tokio::test]
async fn a_strip_reporting_a_wrong_processed_width_refuses_the_strip() {
    let document = uncertifiable_gray_scan();
    let (replaced, original, strip_heights) = replace(&document, Lines::WrongWidth).await;
    assert!(!replaced);
    assert!(!used_strips(&original));
    assert_eq!(original.content, PRIOR_READ.0);
    assert!(
        !strip_heights.is_empty(),
        "the backend ran on strips: {strip_heights:?}"
    );
}

/// The displaced first line still fits inside the strip and touches no other ink band, so only
/// the band-overlap half of the overlap check refuses it: removing the strip-bottom bound
/// must not certify this input.
#[tokio::test]
async fn a_line_dropped_below_its_ink_band_refuses_the_strip() {
    let document = uncertifiable_gray_scan();
    let (replaced, original, strip_heights) = replace(&document, Lines::DisplacedFirst).await;
    assert!(!replaced);
    assert!(!used_strips(&original));
    assert_eq!(original.content, PRIOR_READ.0);
    assert!(
        !strip_heights.is_empty(),
        "the backend ran on strips: {strip_heights:?}"
    );
}

/// The overhanging last line still overlaps only its own ink band, so only the strip-bottom
/// bound refuses it: removing the overlap check must not certify this input.
#[tokio::test]
async fn a_line_hanging_past_the_strip_bottom_refuses_the_strip() {
    let document = uncertifiable_gray_scan();
    let (replaced, original, strip_heights) = replace(&document, Lines::OverhangingLast).await;
    assert!(!replaced);
    assert!(!used_strips(&original));
    assert_eq!(original.content, PRIOR_READ.0);
    assert!(
        !strip_heights.is_empty(),
        "the backend ran on strips: {strip_heights:?}"
    );
}

/// The first line of every strip with a following run keeps its own band's top but reaches
/// into the next ink band, so only the neighbour-overlap half of the overlap check refuses
/// it: removing the strip-bottom bound must not certify this input.
#[tokio::test]
async fn a_line_reaching_into_the_next_ink_band_refuses_the_strip() {
    let document = uncertifiable_gray_scan();
    let (replaced, original, strip_heights) = replace(&document, Lines::BandReach).await;
    assert!(!replaced);
    assert!(!used_strips(&original));
    assert_eq!(original.content, PRIOR_READ.0);
    assert!(
        !strip_heights.is_empty(),
        "the backend ran on strips: {strip_heights:?}"
    );
}

/// The first line of every strip reports a right edge past the strip width, so only the
/// over-width clause refuses it: removing the neighbour-overlap half of the overlap check
/// must not certify this input.
#[tokio::test]
async fn a_line_reporting_a_right_beyond_the_strip_width_refuses_the_strip() {
    let document = uncertifiable_gray_scan();
    let (replaced, original, strip_heights) = replace(&document, Lines::OverWide).await;
    assert!(!replaced);
    assert!(!used_strips(&original));
    assert_eq!(original.content, PRIOR_READ.0);
    assert!(
        !strip_heights.is_empty(),
        "the backend ran on strips: {strip_heights:?}"
    );
}

#[tokio::test]
async fn a_certified_clean_path_replaces_a_noisy_read_through_the_complete_rows_evidence() {
    let document = certifiable_bilevel_scan();
    let (replaced, original, strip_heights) = replace(&document, Lines::CertifiedCleanPath).await;
    assert!(replaced);
    assert!(used_strips(&original));
    assert_eq!(reported_lines(&original), source_lines(is_ink_row));
    assert!(
        strip_heights.len() >= 2,
        "the scan is cut into strips: {strip_heights:?}"
    );
}

#[tokio::test]
async fn an_uncertified_strip_withholds_the_complete_rows_evidence_from_the_page_read() {
    let document = certifiable_bilevel_scan();
    let (replaced, original, strip_heights) = replace(&document, Lines::UncertifiedCompleteRows).await;
    assert!(!replaced);
    assert!(!used_strips(&original));
    assert_eq!(original.content, NOISY_READ.0);
    assert!(
        !strip_heights.is_empty(),
        "the backend ran on strips: {strip_heights:?}"
    );
}

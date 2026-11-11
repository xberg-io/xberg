//! One input per check of the tall-scan strip certificate (issue #2911), plus inputs that
//! separate the overlap check from the strip-bottom check.
//!
//! The `crates/xberg/tests/` suites cannot reach the certificate: both strip recovery routes
//! gate the attempt on the builtin Tesseract instance, so a registered custom backend is never
//! handed a strip through the public API. Like `tall_source::neutral_rgb_tests`, these tests
//! drive `try_replace` directly — from a module beside, not inside, `tall_source`.

use super::tall_source::try_replace;
use crate::core::config::{ExtractionConfig, OcrConfig};
use crate::ocr_metadata_keys::{
    OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY, OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY,
    OCR_TESSERACT_DICT_INVALID_WORD_RATIO_METADATA_KEY,
};
use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
use crate::types::{ExtractedDocument, FormatMetadata, OcrBoundingGeometry, OcrConfidence, OcrElement, OcrMetadata};
use std::borrow::Cow;
use std::io::Write;
use std::ops::Range;
use std::sync::{Arc, Mutex};
use xberg_native_pdf::PdfDocument;

const WIDTH: u32 = 1200;
const HEIGHT: u32 = 4400;
const LINE_PITCH: u32 = 40;
const INK_ROWS: Range<u32> = 10..20;
const INK_COLUMNS: Range<usize> = 96..904;
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
    /// The first line of each strip is dropped into the blank gap below its ink band.
    DisplacedFirst,
    /// The last line of each strip hangs past the strip bottom.
    OverhangingLast,
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

fn recognized(content: &str, mean: f64, invalid: f64, words: u32) -> ExtractedDocument {
    let mut result = ExtractedDocument {
        content: content.into(),
        ..Default::default()
    };
    result.metadata.format = Some(FormatMetadata::Ocr(OcrMetadata {
        language: "eng".into(),
        psm: 11,
        output_format: "markdown".into(),
        table_count: 0,
        table_rows: None,
        table_cols: None,
    }));
    result.metadata.additional.extend([
        (Cow::Borrowed("mean_text_conf"), mean.into()),
        (Cow::Borrowed("word_count"), words.into()),
        (Cow::Borrowed("low_conf_word_count"), 0.into()),
        (
            Cow::Borrowed(OCR_TESSERACT_DICT_INVALID_WORD_RATIO_METADATA_KEY),
            invalid.into(),
        ),
    ]);
    result
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

fn ink_runs(strip: &image::GrayImage) -> Vec<Range<u32>> {
    let (width, height) = strip.dimensions();
    let mut runs: Vec<Range<u32>> = Vec::new();
    for y in 0..height {
        if (0..width).any(|x| strip.get_pixel(x, y).0[0] < 128) {
            match runs.last_mut() {
                Some(run) if run.end == y => run.end += 1,
                _ => runs.push(y..y + 1),
            }
        }
    }
    runs
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
    let reported_height = if lines == Lines::WrongHeight {
        height - 1
    } else {
        height
    };
    result.metadata.additional.extend([
        (Cow::Borrowed(OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY), width.into()),
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
            } else {
                let displaced = lines == Lines::DisplacedFirst && index == 0;
                (
                    run.start + u32::from(displaced) * LINE_DISPLACEMENT,
                    run.end - run.start,
                )
            };
            let mismatched = lines == Lines::MismatchedText && index == 0;
            OcrElement::new(
                if mismatched { "alpha delta" } else { LINE_TEXT },
                OcrBoundingGeometry::Rectangle {
                    left: INK_COLUMNS.start as u32,
                    top,
                    width: (INK_COLUMNS.end - INK_COLUMNS.start) as u32,
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

fn is_ink_row(y: u32) -> bool {
    INK_ROWS.contains(&(y % LINE_PITCH))
}

/// Ink line runs of the fixture scan, counted from the fixture constants.
fn source_lines() -> usize {
    (0..HEIGHT)
        .filter(|&y| is_ink_row(y) && (y == 0 || !is_ink_row(y - 1)))
        .count()
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

fn flate(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
    encoder.write_all(bytes).expect("in-memory write");
    encoder.finish().expect("in-memory flush")
}

/// Text-line-like black bars on white, one sample per pixel.
fn gray_samples(ink_row: fn(u32) -> bool) -> Vec<u8> {
    let mut samples = Vec::with_capacity(WIDTH as usize * HEIGHT as usize);
    for y in 0..HEIGHT {
        samples.extend((0..WIDTH as usize).map(|x| if ink_row(y) && INK_COLUMNS.contains(&x) { 0 } else { 255 }));
    }
    samples
}

/// Bars one bit per pixel; a byte is ink when both of its ends are ink columns.
fn bilevel_samples(ink_row: fn(u32) -> bool) -> Vec<u8> {
    let mut samples = Vec::new();
    for y in 0..HEIGHT {
        for byte in 0..WIDTH as usize / 8 {
            let ink = INK_COLUMNS.contains(&(byte * 8)) && INK_COLUMNS.contains(&(byte * 8 + 7));
            samples.push(if ink_row(y) && ink { 0x00 } else { 0xff });
        }
    }
    samples
}

fn stream(entries: &str, bytes: &[u8]) -> Vec<u8> {
    let mut output = format!("<< {entries} /Length {} >>\nstream\n", bytes.len()).into_bytes();
    output.extend(bytes);
    output.extend(b"\nendstream");
    output
}

/// One scan image of `dpi` painted over the whole page.
fn scan_pdf_at(color: &str, samples: &[u8], dpi: f64) -> PdfDocument {
    let (page_width, page_height) = (f64::from(WIDTH) * 72.0 / dpi, f64::from(HEIGHT) * 72.0 / dpi);
    let image = stream(
        &format!("/Type /XObject /Subtype /Image /Width {WIDTH} /Height {HEIGHT} {color} /Filter /FlateDecode"),
        &flate(samples),
    );
    let objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {page_width} {page_height}] /Contents 4 0 R /Resources << /XObject << /Im0 5 0 R >> >> >>"
        )
        .into_bytes(),
        stream("", format!("q {page_width} 0 0 {page_height} 0 0 cm /Im0 Do Q").as_bytes()),
        image,
    ];
    let mut bytes = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for object in &objects {
        offsets.push(bytes.len());
        bytes.extend(format!("{} 0 obj\n", offsets.len()).as_bytes());
        bytes.extend(object);
        bytes.extend(b"\nendobj\n");
    }
    let xref = bytes.len();
    bytes.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
    for offset in offsets {
        bytes.extend(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    PdfDocument::from_bytes(bytes).expect("a well-formed synthetic scan")
}

const GRAY: &str = "/ColorSpace /DeviceGray /BitsPerComponent 8";
const BILEVEL: &str = "/ColorSpace /DeviceGray /BitsPerComponent 1";

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
    assert_eq!(reported_lines(&original), source_lines());
    assert!(
        strip_heights.len() >= 2,
        "the scan is cut into strips: {strip_heights:?}"
    );
}

#[tokio::test]
async fn strips_without_line_elements_are_refused_even_when_the_processed_size_is_present() {
    let document = uncertifiable_gray_scan();
    let (replaced, original, _) = replace(&document, Lines::NoElements).await;
    assert!(!replaced);
    assert!(!used_strips(&original));
    assert_eq!(original.content, PRIOR_READ.0);
}

#[tokio::test]
async fn a_line_element_whose_text_differs_from_its_content_line_refuses_the_strip() {
    let document = uncertifiable_gray_scan();
    let (replaced, original, _) = replace(&document, Lines::MismatchedText).await;
    assert!(!replaced);
    assert!(!used_strips(&original));
    assert_eq!(original.content, PRIOR_READ.0);
}

#[tokio::test]
async fn a_leftover_text_line_without_an_element_refuses_the_strip() {
    let document = uncertifiable_gray_scan();
    let (replaced, original, _) = replace(&document, Lines::ExtraTextLine).await;
    assert!(!replaced);
    assert!(!used_strips(&original));
    assert_eq!(original.content, PRIOR_READ.0);
}

#[tokio::test]
async fn a_strip_reporting_a_wrong_processed_height_refuses_the_strip() {
    let document = uncertifiable_gray_scan();
    let (replaced, original, _) = replace(&document, Lines::WrongHeight).await;
    assert!(!replaced);
    assert!(!used_strips(&original));
    assert_eq!(original.content, PRIOR_READ.0);
}

/// The displaced first line still fits inside the strip and touches no other ink band, so only
/// the overlap check refuses it: removing the strip-bottom bound must not certify this input.
#[tokio::test]
async fn a_line_dropped_below_its_ink_band_refuses_the_strip() {
    let document = uncertifiable_gray_scan();
    let (replaced, original, _) = replace(&document, Lines::DisplacedFirst).await;
    assert!(!replaced);
    assert!(!used_strips(&original));
    assert_eq!(original.content, PRIOR_READ.0);
}

/// The overhanging last line still overlaps only its own ink band, so only the strip-bottom
/// bound refuses it: removing the overlap check must not certify this input.
#[tokio::test]
async fn a_line_hanging_past_the_strip_bottom_refuses_the_strip() {
    let document = uncertifiable_gray_scan();
    let (replaced, original, _) = replace(&document, Lines::OverhangingLast).await;
    assert!(!replaced);
    assert!(!used_strips(&original));
    assert_eq!(original.content, PRIOR_READ.0);
}

#[tokio::test]
async fn a_certified_clean_path_replaces_a_noisy_read_through_the_complete_rows_evidence() {
    let document = certifiable_bilevel_scan();
    let (replaced, original, strip_heights) = replace(&document, Lines::CertifiedCleanPath).await;
    assert!(replaced);
    assert!(used_strips(&original));
    assert_eq!(reported_lines(&original), source_lines());
    assert!(
        strip_heights.len() >= 2,
        "the scan is cut into strips: {strip_heights:?}"
    );
}

#[tokio::test]
async fn an_uncertified_strip_withholds_the_complete_rows_evidence_from_the_page_read() {
    let document = certifiable_bilevel_scan();
    let (replaced, original, _) = replace(&document, Lines::UncertifiedCompleteRows).await;
    assert!(!replaced);
    assert!(!used_strips(&original));
    assert_eq!(original.content, NOISY_READ.0);
}

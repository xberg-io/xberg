use super::*;
use crate::ocr_metadata_keys::{OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY, OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY};
use crate::plugins::{OcrBackendType, Plugin};
use crate::types::OcrElementLevel;
use std::io::Write;
use std::sync::Mutex;

const WIDTH: u32 = 1200;
const HEIGHT: u32 = 4400;
const LINE_PITCH: u32 = 40;
const INK_ROWS: Range<u32> = 10..20;
const INK_COLUMNS: Range<usize> = 96..904;
const WORDS: &str = "alpha bravo charlie delta echo foxtrot golf hotel india juliet kilo lima mike november oscar papa \
                     quebec romeo sierra tango";

/// What the backend reports about the lines of each strip it is given.
#[derive(Clone, Copy, PartialEq)]
enum Lines {
    /// No line elements, as a backend that was not asked for them.
    Absent,
    /// One line element per run of ink rows, in the order and place they are in the strip.
    Exact,
    /// The last line of each strip is missing from both the elements and the text.
    OmitLast,
    /// Every line element is placed in the blank rows below its ink.
    Misplaced,
}

struct StripBackend {
    asked_for_line_elements: Arc<Mutex<Vec<bool>>>,
    strip_heights: Arc<Mutex<Vec<u32>>>,
    lines: Lines,
}

impl Plugin for StripBackend {
    fn name(&self) -> &str {
        "neutral-rgb-strips"
    }
    fn version(&self) -> String {
        "1.0.0".to_string()
    }
    fn initialize(&self) -> crate::Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> crate::Result<()> {
        Ok(())
    }
}

fn recognized(content: &str, mean: f64, invalid: f64, words: u32) -> ExtractedDocument {
    let mut result = ExtractedDocument {
        content: content.into(),
        ..Default::default()
    };
    result.metadata.format = Some(crate::types::FormatMetadata::Ocr(crate::types::OcrMetadata {
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
        (Cow::Borrowed(DICTIONARY_INVALID), invalid.into()),
    ]);
    result
}

#[async_trait::async_trait]
impl OcrBackend for StripBackend {
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> crate::Result<ExtractedDocument> {
        self.asked_for_line_elements.lock().unwrap().push(
            config
                .element_config
                .as_ref()
                .is_some_and(|elements| elements.include_elements && elements.min_level == OcrElementLevel::Line),
        );
        self.strip_heights
            .lock()
            .unwrap()
            .push(image::load_from_memory(image_bytes).unwrap().height());
        if self.lines == Lines::Absent {
            return Ok(recognized(WORDS, 95.0, 0.0, 20));
        }
        Ok(line_report(image_bytes, self.lines))
    }
}

fn line_report(png: &[u8], lines: Lines) -> ExtractedDocument {
    let strip = image::load_from_memory(png).unwrap().to_luma8();
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
    if lines == Lines::OmitLast {
        runs.pop();
    }
    let text = vec!["alpha bravo"; runs.len()].join("\n");
    let mut result = recognized(&text, 95.0, 0.0, 2 * runs.len() as u32);
    result.ocr_elements = Some(
        runs.iter()
            .map(|run| {
                let shift = if lines == Lines::Misplaced { 20 } else { 0 };
                crate::types::OcrElement::new(
                    "alpha bravo",
                    crate::types::OcrBoundingGeometry::Rectangle {
                        left: INK_COLUMNS.start as u32,
                        top: run.start + shift,
                        width: (INK_COLUMNS.end - INK_COLUMNS.start) as u32,
                        height: run.end - run.start,
                    },
                    crate::types::OcrConfidence {
                        detection: None,
                        recognition: 0.95,
                    },
                )
            })
            .collect(),
    );
    result.metadata.additional.extend([
        (Cow::Borrowed(OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY), width.into()),
        (Cow::Borrowed(OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY), height.into()),
    ]);
    result
}

fn is_ink_row(y: u32) -> bool {
    INK_ROWS.contains(&(y % LINE_PITCH))
}

/// The first bar starts at row 20, as many blank rows as the uncertified rule needs above it.
fn is_ink_row_after_a_full_blank_band(y: u32) -> bool {
    (20..30).contains(&(y % LINE_PITCH)) && y < HEIGHT - 50
}

/// Bars 10 rows tall with 10 blank rows between them, apart from one 70 row blank band that
/// lets the page be cut in two.
fn is_ink_row_between_short_gaps(y: u32) -> bool {
    (5..15).contains(&(y % 20)) && !(2180..2240).contains(&y)
}

fn flate(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
    encoder.write_all(bytes).unwrap();
    encoder.finish().unwrap()
}

fn is_ink(y: u32, x: usize, ink_row: fn(u32) -> bool) -> bool {
    ink_row(y) && INK_COLUMNS.contains(&x)
}

/// Text-line-like black bars on white, three equal samples per pixel.
fn neutral_rgb_samples(ink_row: fn(u32) -> bool) -> Vec<u8> {
    let mut samples = Vec::with_capacity(WIDTH as usize * HEIGHT as usize * 3);
    for y in 0..HEIGHT {
        for x in 0..WIDTH as usize {
            samples.extend([if is_ink(y, x, ink_row) { 0 } else { 255 }; 3]);
        }
    }
    samples
}

fn gray_samples(ink_row: fn(u32) -> bool) -> Vec<u8> {
    let mut samples = Vec::with_capacity(WIDTH as usize * HEIGHT as usize);
    for y in 0..HEIGHT {
        samples.extend((0..WIDTH as usize).map(|x| if is_ink(y, x, ink_row) { 0 } else { 255 }));
    }
    samples
}

fn bilevel_samples(ink_row: fn(u32) -> bool) -> Vec<u8> {
    let mut samples = Vec::new();
    for y in 0..HEIGHT {
        for byte in 0..WIDTH as usize / 8 {
            let ink = is_ink(y, byte * 8, ink_row) && is_ink(y, byte * 8 + 7, ink_row);
            samples.push(if ink { 0x00 } else { 0xff });
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
    PdfDocument::from_bytes(bytes).unwrap()
}

const RGB: &str = "/ColorSpace /DeviceRGB /BitsPerComponent 8";
const GRAY: &str = "/ColorSpace /DeviceGray /BitsPerComponent 8";
const BILEVEL: &str = "/ColorSpace /DeviceGray /BitsPerComponent 1";

fn neutral_rgb_scan() -> PdfDocument {
    scan_pdf_at(RGB, &neutral_rgb_samples(is_ink_row), 150.0)
}

/// Run `try_replace` over `document` with a recording backend. The page was first read from a render
/// `rendered` pixels across, which `original` reports as `processed`.
async fn replace(
    document: &PdfDocument,
    rendered: (u32, u32),
    processed: Option<(u32, u32)>,
) -> (bool, ExtractedDocument, Vec<bool>) {
    let (replaced, original, asked, _) = replace_reporting(document, rendered, processed, Lines::Absent).await;
    (replaced, original, asked)
}

async fn replace_reporting(
    document: &PdfDocument,
    rendered: (u32, u32),
    processed: Option<(u32, u32)>,
    lines: Lines,
) -> (bool, ExtractedDocument, Vec<bool>, Vec<u32>) {
    let asked = Arc::new(Mutex::new(Vec::new()));
    let heights = Arc::new(Mutex::new(Vec::new()));
    let backend: Arc<dyn OcrBackend> = Arc::new(StripBackend {
        asked_for_line_elements: Arc::clone(&asked),
        strip_heights: Arc::clone(&heights),
        lines,
    });
    let mut original = recognized("one two three", 80.0, 0.1, 3);
    if let Some((width, height)) = processed {
        original
            .metadata
            .additional
            .insert(Cow::Borrowed(OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY), width.into());
        original
            .metadata
            .additional
            .insert(Cow::Borrowed(OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY), height.into());
    }
    let page = (0, Arc::new(vec![0; 1000]), rendered.0, rendered.1);
    let replaced = try_replace(
        &mut original,
        &backend,
        &ExtractionConfig::default(),
        &OcrConfig::default(),
        (document, 0),
        &page,
    )
    .await
    .unwrap();
    let asked = asked.lock().unwrap().clone();
    let heights = heights.lock().unwrap().clone();
    (replaced, original, asked, heights)
}

#[tokio::test]
async fn a_neutral_rgb_scan_at_150_dpi_is_recovered_without_asking_for_line_certificates() {
    let (replaced, original, asked) = replace(&neutral_rgb_scan(), (1000, 1760), None).await;
    assert!(replaced);
    assert_eq!(original.content, format!("{WORDS}\n\n{WORDS}"));
    assert_eq!(
        original.processing_warnings.last().unwrap().message,
        "Page 1 OCR used 2 source-image strips."
    );
    assert_eq!(asked, [false, false], "neutral RGB at 150 DPI skips row certification");
}

#[tokio::test]
async fn a_bilevel_scan_at_150_dpi_is_asked_for_line_certificates() {
    let document = scan_pdf_at(BILEVEL, &bilevel_samples(is_ink_row), 150.0);
    let (replaced, original, asked) = replace(&document, (1000, 1760), None).await;
    assert!(replaced);
    assert_eq!(original.content, format!("{WORDS}\n\n{WORDS}"));
    assert_eq!(
        asked,
        [true, true],
        "the same page packed as bilevel is certified row by row"
    );
}

/// `source_frame` refuses a read whose width is at least 98% of the source's: 1175 of 1200 px is
/// admitted and 1176 is not.
#[tokio::test]
async fn the_source_is_refused_from_a_read_at_98_percent_of_its_width() {
    let document = neutral_rgb_scan();
    let (replaced, original, _) = replace(&document, (1175, 4308), None).await;
    assert!(replaced);
    assert_eq!(original.content, format!("{WORDS}\n\n{WORDS}"));
    let (replaced, ..) = replace(&document, (1176, 4312), None).await;
    assert!(!replaced);
}

#[tokio::test]
async fn the_read_width_not_the_rendered_width_is_checked_against_the_source() {
    let document = neutral_rgb_scan();
    let (replaced, ..) = replace(&document, (WIDTH, HEIGHT), None).await;
    assert!(!replaced, "a render as wide as the source has nothing to recover");
    let (replaced, original, _) = replace(&document, (WIDTH, HEIGHT), Some((1175, 4308))).await;
    assert!(replaced, "the page was read at 1175 px");
    assert_eq!(original.content, format!("{WORDS}\n\n{WORDS}"));
    let (replaced, ..) = replace(&document, (WIDTH, HEIGHT), Some((1176, 4312))).await;
    assert!(!replaced, "the page was read at 1176 px");
}

const RENDERED: (u32, u32) = (1000, 1760);

#[tokio::test]
async fn a_neutral_rgb_scan_is_not_certified_when_its_first_bar_follows_a_full_blank_band() {
    let document = scan_pdf_at(RGB, &neutral_rgb_samples(is_ink_row_after_a_full_blank_band), 150.0);
    let (replaced, original, asked) = replace(&document, RENDERED, None).await;
    assert!(replaced);
    assert_eq!(original.content, format!("{WORDS}\n\n{WORDS}"));
    assert_eq!(
        asked,
        [false, false],
        "the blank band above the first bar does not open the ink gate"
    );
}

#[tokio::test]
async fn a_bilevel_scan_with_the_same_late_first_bar_is_asked_for_line_certificates() {
    let document = scan_pdf_at(BILEVEL, &bilevel_samples(is_ink_row_after_a_full_blank_band), 150.0);
    let (replaced, _, asked) = replace(&document, RENDERED, None).await;
    assert!(replaced);
    assert_eq!(asked, [true, true]);
}

#[tokio::test]
async fn short_blank_gaps_between_bars_are_separate_lines_of_a_certified_scan() {
    let document = scan_pdf_at(BILEVEL, &bilevel_samples(is_ink_row_between_short_gaps), 150.0);
    let (replaced, _, asked) = replace(&document, RENDERED, None).await;
    assert!(replaced);
    assert_eq!(
        asked,
        [true, true],
        "10 blank rows still separate two lines when rows are certified"
    );
    let (replaced, original, _, heights) = replace_reporting(&document, RENDERED, None, Lines::Exact).await;
    assert!(replaced);
    assert_eq!(original.content.lines().filter(|line| !line.is_empty()).count(), 217);
    assert_eq!(
        heights,
        [2210, 2190],
        "the only band of 20 rows or more is cut in the middle"
    );
}

#[tokio::test]
async fn a_scan_whose_short_gaps_are_not_certified_is_not_asked_for_line_certificates() {
    let document = scan_pdf_at(RGB, &neutral_rgb_samples(is_ink_row_between_short_gaps), 150.0);
    let (replaced, _, asked) = replace(&document, RENDERED, None).await;
    assert!(replaced);
    assert_eq!(asked, [false, false]);
}

#[tokio::test]
async fn a_gray_scan_below_150_dpi_is_refused_unless_every_strip_returns_its_lines() {
    let document = scan_pdf_at(GRAY, &gray_samples(is_ink_row), 144.0);
    let (replaced, original, asked) = replace(&document, RENDERED, None).await;
    assert!(!replaced, "no line elements come back, so nothing certifies the strip");
    assert_eq!(original.content, "one two three");
    assert_eq!(asked, [true]);

    let (replaced, original, asked, _) = replace_reporting(&document, RENDERED, None, Lines::Exact).await;
    assert!(replaced);
    assert_eq!(asked, [true, true]);
    assert_eq!(original.content.lines().filter(|line| !line.is_empty()).count(), 110);
    assert_eq!(
        original.processing_warnings.last().unwrap().message,
        "Page 1 OCR used 2 source-image strips."
    );
}

#[tokio::test]
async fn a_gray_scan_below_150_dpi_is_refused_when_a_strip_omits_or_misplaces_a_line() {
    let document = scan_pdf_at(GRAY, &gray_samples(is_ink_row), 144.0);
    for lines in [Lines::OmitLast, Lines::Misplaced] {
        let (replaced, original, asked, _) = replace_reporting(&document, RENDERED, None, lines).await;
        assert!(!replaced);
        assert_eq!(original.content, "one two three");
        assert_eq!(asked, [true]);
    }
}

#[tokio::test]
async fn a_gray_scan_at_150_dpi_keeps_its_strips_when_the_lines_are_not_certified() {
    let document = scan_pdf_at(GRAY, &gray_samples(is_ink_row), 150.0);
    let (replaced, original, asked, _) = replace_reporting(&document, RENDERED, None, Lines::OmitLast).await;
    assert!(
        replaced,
        "at 150 DPI a failed certificate only withholds the complete-rows evidence"
    );
    assert_eq!(asked, [false, false]);
    assert_eq!(original.content.lines().filter(|line| !line.is_empty()).count(), 108);
}

#[tokio::test]
async fn a_neutral_rgb_scan_below_150_dpi_is_refused_whatever_the_backend_returns() {
    let document = scan_pdf_at(RGB, &neutral_rgb_samples(is_ink_row), 144.0);
    for lines in [Lines::Absent, Lines::Exact] {
        let (replaced, original, asked, _) = replace_reporting(&document, RENDERED, None, lines).await;
        assert!(!replaced);
        assert_eq!(original.content, "one two three");
        assert!(
            asked.is_empty(),
            "the density floor refuses it before any strip is read"
        );
    }
}

#[tokio::test]
async fn short_gaps_in_a_gray_scan_below_150_dpi_are_certified_line_by_line() {
    let document = scan_pdf_at(GRAY, &gray_samples(is_ink_row_between_short_gaps), 144.0);
    let (replaced, original, asked, _) = replace_reporting(&document, RENDERED, None, Lines::Exact).await;
    assert!(replaced);
    assert_eq!(asked, [true, true]);
    assert_eq!(original.content.lines().filter(|line| !line.is_empty()).count(), 217);
}

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

struct StripBackend {
    asked_for_line_elements: Arc<Mutex<Vec<bool>>>,
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
    async fn process_image(&self, _: &[u8], config: &OcrConfig) -> crate::Result<ExtractedDocument> {
        self.asked_for_line_elements.lock().unwrap().push(
            config
                .element_config
                .as_ref()
                .is_some_and(|elements| elements.include_elements && elements.min_level == OcrElementLevel::Line),
        );
        Ok(recognized(WORDS, 95.0, 0.0, 20))
    }
}

fn is_ink_row(y: u32) -> bool {
    INK_ROWS.contains(&(y % LINE_PITCH))
}

fn flate(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
    encoder.write_all(bytes).unwrap();
    encoder.finish().unwrap()
}

/// 110 text-line-like black bars, each 10 rows tall with 30 blank rows between them.
fn neutral_rgb_samples() -> Vec<u8> {
    let mut samples = Vec::with_capacity(WIDTH as usize * HEIGHT as usize * 3);
    for y in 0..HEIGHT {
        for x in 0..WIDTH as usize {
            let gray = if is_ink_row(y) && INK_COLUMNS.contains(&x) {
                0
            } else {
                255
            };
            samples.extend([gray; 3]);
        }
    }
    samples
}

fn bilevel_samples() -> Vec<u8> {
    let mut samples = Vec::new();
    for y in 0..HEIGHT {
        for byte in 0..WIDTH as usize / 8 {
            let ink = is_ink_row(y) && INK_COLUMNS.contains(&(byte * 8)) && INK_COLUMNS.contains(&(byte * 8 + 7));
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

/// One 150 DPI scan image painted over the whole 576 x 2112 pt page.
fn scan_pdf(color: &str, samples: &[u8]) -> PdfDocument {
    let image = stream(
        &format!("/Type /XObject /Subtype /Image /Width {WIDTH} /Height {HEIGHT} {color} /Filter /FlateDecode"),
        &flate(samples),
    );
    let objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 576 2112] /Contents 4 0 R /Resources << /XObject << /Im0 5 0 R >> >> >>".to_vec(),
        stream("", b"q 576 0 0 2112 0 0 cm /Im0 Do Q"),
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

fn neutral_rgb_scan() -> PdfDocument {
    scan_pdf("/ColorSpace /DeviceRGB /BitsPerComponent 8", &neutral_rgb_samples())
}

/// Run `try_replace` over `document` with a recording backend. The page was first read from a render
/// `rendered` pixels across, which `original` reports as `processed`.
async fn replace(
    document: &PdfDocument,
    rendered: (u32, u32),
    processed: Option<(u32, u32)>,
) -> (bool, ExtractedDocument, Vec<bool>) {
    let asked = Arc::new(Mutex::new(Vec::new()));
    let backend: Arc<dyn OcrBackend> = Arc::new(StripBackend {
        asked_for_line_elements: Arc::clone(&asked),
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
    (replaced, original, asked)
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
    let document = scan_pdf("/ColorSpace /DeviceGray /BitsPerComponent 1", &bilevel_samples());
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

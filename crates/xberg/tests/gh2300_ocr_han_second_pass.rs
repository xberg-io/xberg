//! Regression coverage for GH#2300: an image whose first OCR pass is noisy and whose script is
//! Han is read again with the Chinese language added to the configured ones.
//!
//! The fixture is a generated picture of four neutral Chinese sentences. With the default
//! language `eng` the first pass returns Latin noise. Tesseract's script detection reports Han,
//! and the second pass reads the sentences.

#![cfg(feature = "ocr")]

mod helpers;

use helpers::extract_bytes_document_blocking;
use xberg::core::config::{ExtractionConfig, OcrConfig};

const CHINESE_LINES: &[u8] = include_bytes!("fixtures/ocr/gh2300_chinese_lines.png");
const ENGLISH_TABLE: &[u8] = include_bytes!("fixtures/ocr/gh1785_shaded_rows.png");
/// The sentences are about 50 characters long. Half of them is far above the Latin noise.
const MIN_HAN_CHARACTERS: usize = 25;

fn han_characters(text: &str) -> usize {
    text.chars().filter(|c| ('\u{4E00}'..='\u{9FFF}').contains(c)).count()
}

fn config(language: &[&str]) -> ExtractionConfig {
    ExtractionConfig {
        ocr: Some(OcrConfig {
            language: language.iter().map(|l| l.to_string()).collect(),
            ..Default::default()
        }),
        use_cache: false,
        ..Default::default()
    }
}

fn read(bytes: &[u8], config: &ExtractionConfig) -> String {
    extract_bytes_document_blocking(bytes, "image/png", config)
        .expect("OCR of the fixture must not error")
        .content
}

#[test]
fn a_chinese_image_read_with_the_default_language_is_read_as_chinese() {
    let text = read(CHINESE_LINES, &config(&["eng"]));
    assert!(
        han_characters(&text) >= MIN_HAN_CHARACTERS,
        "the default language must still read Chinese through the second pass, got {text:?}"
    );
}

#[test]
fn an_english_image_keeps_its_first_pass() {
    let text = read(ENGLISH_TABLE, &config(&["eng"]));
    assert_eq!(
        han_characters(&text),
        0,
        "an English picture gains no Chinese, got {text:?}"
    );
    assert!(
        text.to_lowercase().contains("total"),
        "the English text is still read, got {text:?}"
    );
}

/// The processor runs the languages it is given. A request that forces OCR is not adapted by the
/// extractor, so it gets one pass with the configured languages (GH#2300 review).
#[test]
fn a_forced_ocr_request_on_a_chinese_image_gets_one_pass_with_the_configured_languages() {
    let config = ExtractionConfig {
        force_ocr: true,
        ..config(&["eng"])
    };
    let text = read(CHINESE_LINES, &config);
    assert_eq!(
        han_characters(&text),
        0,
        "forced OCR keeps the configured languages, got {text:?}"
    );
}

/// The same request with the result cache on, run twice: each run returns the result of its own
/// languages, never one stored under another run's key.
#[test]
fn a_cached_forced_request_never_returns_an_adapted_result() {
    let config = ExtractionConfig {
        force_ocr: true,
        use_cache: true,
        ..config(&["eng"])
    };
    for _ in 0..2 {
        let text = read(CHINESE_LINES, &config);
        assert_eq!(han_characters(&text), 0, "got {text:?}");
    }
}

#[cfg(feature = "pdf")]
/// A one-page PDF whose page is the picture `png`, stored as a raw grey raster over the page.
fn pdf_of_picture(png: &[u8]) -> Vec<u8> {
    let grey = image::load_from_memory(png).expect("decode the fixture").into_luma8();
    let (width, height) = grey.dimensions();
    let content = format!("q {width} 0 0 {height} 0 0 cm /Im0 Do Q\n");
    let mut buf = Vec::<u8>::new();
    let mut offsets = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");
    let mut object = |buf: &mut Vec<u8>, body: &[u8]| {
        offsets.push(buf.len());
        buf.extend_from_slice(format!("{} 0 obj\n", offsets.len()).as_bytes());
        buf.extend_from_slice(body);
        buf.extend_from_slice(b"\nendobj\n");
    };
    object(&mut buf, b"<</Type /Catalog /Pages 2 0 R>>");
    object(&mut buf, b"<</Type /Pages /Kids [3 0 R] /Count 1>>");
    object(
        &mut buf,
        format!(
            "<</Type /Page /MediaBox [0 0 {width} {height}] /Parent 2 0 R /Contents 4 0 R \
             /Resources <</XObject <</Im0 5 0 R>> >> >>"
        )
        .as_bytes(),
    );
    object(
        &mut buf,
        format!("<</Length {}>>\nstream\n{content}\nendstream", content.len() + 1).as_bytes(),
    );
    let mut image = format!(
        "<</Type /XObject /Subtype /Image /Width {width} /Height {height} /ColorSpace /DeviceGray \
         /BitsPerComponent 8 /Length {}>>\nstream\n",
        grey.as_raw().len()
    )
    .into_bytes();
    image.extend_from_slice(grey.as_raw());
    image.extend_from_slice(b"\nendstream");
    object(&mut buf, &image);
    let xref_offset = buf.len();
    buf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len() + 1).as_bytes());
    for offset in &offsets {
        buf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    buf.extend_from_slice(format!("trailer\n<</Size {} /Root 1 0 R>>\n", offsets.len() + 1).as_bytes());
    buf.extend_from_slice(format!("startxref\n{xref_offset}\n%%EOF\n").as_bytes());
    buf
}

/// A PDF page that goes to OCR on its own takes the same second pass as an image.
#[cfg(feature = "pdf")]
#[test]
fn a_chinese_pdf_page_sent_to_ocr_on_its_own_is_read_as_chinese() {
    let text = read_pdf(&pdf_of_picture(CHINESE_LINES), &config(&["eng"]));
    assert!(
        han_characters(&text) >= MIN_HAN_CHARACTERS,
        "an unforced PDF page must be read through the second pass, got {text:?}"
    );
}

/// A forced PDF page keeps one pass with the configured languages, as a forced image does.
#[cfg(feature = "pdf")]
#[test]
fn a_forced_ocr_request_on_a_chinese_pdf_page_gets_one_pass_with_the_configured_languages() {
    let config = ExtractionConfig {
        force_ocr: true,
        ..config(&["eng"])
    };
    let text = read_pdf(&pdf_of_picture(CHINESE_LINES), &config);
    assert_eq!(
        han_characters(&text),
        0,
        "forced OCR on a PDF page keeps the configured languages, got {text:?}"
    );
}

#[cfg(feature = "pdf")]
fn read_pdf(bytes: &[u8], config: &ExtractionConfig) -> String {
    extract_bytes_document_blocking(bytes, "application/pdf", config)
        .expect("OCR of the fixture must not error")
        .content
}

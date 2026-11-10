#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "bundle-tessdata-eng",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]

use sha2::{Digest, Sha256};
use xberg::{ExtractInput, ExtractionConfig, OcrConfig, OutputFormat, extract};

const TITLE: &str = "SECTION TITLE";
const PHOTO_PNG: &[u8] = include_bytes!("fixtures/ocr_photo_title_2312/title.png");
const PHOTO_PDF: &[u8] = include_bytes!("fixtures/pdf/discarded-noise-2312/photo-title.pdf");
const TABLE_PNG: &[u8] = include_bytes!("fixtures/ocr_photo_title_2312/table.png");
const PROSE_PNG: &[u8] = include_bytes!("fixtures/ocr_photo_noise_2312/prose.png");
const PROSE_PDF: &[u8] = include_bytes!("fixtures/ocr_photo_noise_2312/prose.pdf");

/// Characters of OCR output before the filter existed, measured on this tree's parent commit.
const PNG_NOISE_BEFORE: usize = 840;
const PDF_NOISE_BEFORE: usize = 1056;

const PROSE: &str = "The committee met on Tuesday to review the annual budget.\n\n\
Members agreed that the library needs a new roof before winter.\n\n\
A short vote followed, and the motion passed with two objections.\n\n\
The treasurer will publish the final figures next month.\n\n\
Residents may send written comments to the town clerk.\n\n\
The meeting adjourned at nine in the evening.";

fn config(output_format: OutputFormat) -> ExtractionConfig {
    ExtractionConfig {
        use_cache: false,
        output_format,
        ocr: Some(OcrConfig::default()),
        ..ExtractionConfig::default()
    }
}

async fn read(bytes: &[u8], mime: &str, output_format: OutputFormat) -> String {
    let mut batch = extract(
        ExtractInput::from_bytes(bytes.to_vec(), mime, None),
        &config(output_format),
    )
    .await
    .expect("extract");
    assert!(batch.errors.is_empty(), "{:?}", batch.errors);
    batch.results.remove(0).content
}

/// Characters other than the real title, which is the only text the photographs carry.
fn noise_chars(content: &str) -> usize {
    assert_eq!(content.matches(TITLE).count(), 1, "{content:?}");
    content.len() - TITLE.len()
}

#[tokio::test]
async fn photograph_png_keeps_at_most_a_quarter_of_its_symbol_noise() {
    for format in [OutputFormat::Plain, OutputFormat::Markdown] {
        let content = read(PHOTO_PNG, "image/png", format).await;
        assert!(noise_chars(&content) * 4 <= PNG_NOISE_BEFORE, "{content:?}");
    }
}

#[tokio::test]
async fn photograph_pdf_keeps_at_most_a_quarter_of_its_symbol_noise() {
    for format in [OutputFormat::Plain, OutputFormat::Markdown] {
        let content = read(PHOTO_PDF, "application/pdf", format).await;
        assert!(noise_chars(&content) * 4 <= PDF_NOISE_BEFORE, "{content:?}");
        assert!(content.trim_start().starts_with(TITLE), "{content:?}");
    }
}

#[tokio::test]
async fn clean_prose_scan_and_native_prose_page_are_unchanged() {
    // The scan drops the full stop after "objections"; the native layer keeps it.
    let scan = read(PROSE_PNG, "image/png", OutputFormat::Plain).await;
    assert_eq!(scan, PROSE.replace("objections.", "objections"));
    let native = read(PROSE_PDF, "application/pdf", OutputFormat::Plain).await;
    assert_eq!(
        native.split_whitespace().collect::<Vec<_>>(),
        PROSE.split_whitespace().collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn short_table_scan_is_unchanged() {
    let content = read(TABLE_PNG, "image/png", OutputFormat::Plain).await;
    assert_eq!(
        hex::encode(Sha256::digest(content.as_bytes())),
        "e19005d542795b6b3dcb3ed3a4701452fe5fb31068ce7e3ef62decbaba03fdbc"
    );
}

#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "bundle-tessdata-eng",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]

use sha2::{Digest, Sha256};
use xberg::core::config::OcrQualityThresholds;
use xberg::{ExtractInput, ExtractionConfig, OcrConfig, OutputFormat, extract};

const TITLE: &str = "SECTION TITLE";
const PHOTO_PNG: &[u8] = include_bytes!("fixtures/ocr_photo_title_2312/title.png");
const PHOTO_PDF: &[u8] = include_bytes!("fixtures/pdf/discarded-noise-2312/photo-title.pdf");
const TABLE_PNG: &[u8] = include_bytes!("fixtures/ocr_photo_title_2312/table.png");
const PROSE_PNG: &[u8] = include_bytes!("fixtures/ocr_photo_noise_2312/prose.png");
const PHOTO_ONLY_PDF: &[u8] = include_bytes!("fixtures/ocr_photo_noise_2312/photo-only.pdf");
const MIXED_PDF: &[u8] = include_bytes!("fixtures/ocr_photo_noise_2312/prose-then-photo.pdf");
const PROSE_PDF: &[u8] = include_bytes!("fixtures/ocr_photo_noise_2312/prose.pdf");

/// Characters of OCR output before the filter existed, measured on this tree's parent commit.
const PNG_NOISE_BEFORE: usize = 840;
const PDF_NOISE_BEFORE: usize = 1056;
const PDF_NOISE_AFTER_PLAIN: usize = 169;
const PDF_PRUNED_LINES: usize = 200;

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
        let plain = format == OutputFormat::Plain;
        let content = read(PHOTO_PDF, "application/pdf", format).await;
        assert!(noise_chars(&content) * 4 <= PDF_NOISE_BEFORE, "{content:?}");
        if plain {
            // Lines with a digit are kept (`$5`, `1.`), which is why 169 characters of the 1056 stay;
            // an earlier rule that dropped single-digit lines left 50.
            assert_eq!(noise_chars(&content), PDF_NOISE_AFTER_PLAIN, "{content:?}");
        }
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

async fn document(bytes: &[u8], mime: &str, config: &ExtractionConfig) -> xberg::ExtractedDocument {
    let mut batch = extract(ExtractInput::from_bytes(bytes.to_vec(), mime, None), config)
        .await
        .expect("extract");
    assert!(batch.errors.is_empty(), "{:?}", batch.errors);
    batch.results.remove(0)
}

fn prune_warnings(document: &xberg::ExtractedDocument) -> Vec<String> {
    document
        .processing_warnings
        .iter()
        .map(|warning| warning.message.to_string())
        .filter(|message| message.contains("symbol-noise line(s)"))
        .collect()
}

fn non_blank_lines(text: &str) -> usize {
    text.lines().filter(|line| !line.trim().is_empty()).count()
}

/// A photograph page that follows a native-text page is read page by page like a PDF of its own,
/// so it is pruned the same way and its warning names its own page number.
#[tokio::test]
async fn a_photograph_page_after_native_text_is_pruned_like_the_pdf_route() {
    let config = config(OutputFormat::Plain);
    let mixed = document(MIXED_PDF, "application/pdf", &config).await;
    let warnings = prune_warnings(&mixed);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].starts_with("Page 2: removed "), "{}", warnings[0]);
    let (prose, photo) = mixed.content.split_once("next month.").expect("prose page");
    assert!(prose.contains("annual budget"));

    let control = document(PHOTO_ONLY_PDF, "application/pdf", &config).await;
    let control_warnings = prune_warnings(&control);
    assert_eq!(control_warnings.len(), 1, "{control_warnings:?}");
    assert!(
        control_warnings[0].starts_with("Page 1: removed "),
        "{}",
        control_warnings[0]
    );
    assert!(
        non_blank_lines(photo) < 60 && non_blank_lines(&control.content) < 60,
        "{photo:?}"
    );
}

#[tokio::test]
async fn a_pruned_pdf_page_carries_a_warning_naming_the_page_and_the_line_count() {
    let document = document(PHOTO_PDF, "application/pdf", &config(OutputFormat::Plain)).await;
    assert_eq!(
        prune_warnings(&document),
        vec![format!(
            "Page 1: removed {PDF_PRUNED_LINES} symbol-noise line(s) from suspected OCR recognition noise; set prune_symbol_noise_lines to false to keep them."
        )]
    );
}

#[tokio::test]
async fn the_flag_off_keeps_every_line_of_the_photograph() {
    let mut config = config(OutputFormat::Plain);
    config.ocr.as_mut().unwrap().quality_thresholds = Some(OcrQualityThresholds {
        prune_symbol_noise_lines: false,
        ..OcrQualityThresholds::default()
    });
    let document = document(PHOTO_PDF, "application/pdf", &config).await;
    assert!(
        noise_chars(&document.content) * 10 >= PDF_NOISE_BEFORE * 9,
        "{:?}",
        document.content
    );
    assert!(prune_warnings(&document).is_empty());
}

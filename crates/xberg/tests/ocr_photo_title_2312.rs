#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "bundle-tessdata-eng",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]

use sha2::{Digest, Sha256};
use xberg::{ExtractInput, ExtractedDocument, ExtractionConfig, OcrConfig, PageConfig, extract};

const TITLE: &str = "SECTION TITLE";
const TITLE_SUFFIX: &str = "\n\nSECTION TITLE";
const PHOTO: &[u8] = include_bytes!("fixtures/ocr_photo_title_2312/title.png");
const MIXED: &[u8] = include_bytes!("fixtures/ocr_photo_title_2312/mixed-sign.png");
const TABLE: &[u8] = include_bytes!("fixtures/ocr_photo_title_2312/table.png");
const PARTIAL: &[u8] = include_bytes!("fixtures/ocr_photo_title_2312/partial.png");
const NUMERIC: &[u8] = include_bytes!("fixtures/ocr_photo_title_2312/numeric.png");

fn png_config() -> ExtractionConfig {
    ExtractionConfig {
        use_cache: false,
        ocr: Some(OcrConfig::default()),
        pages: Some(PageConfig {
            extract_pages: true,
            ..PageConfig::default()
        }),
        ..ExtractionConfig::default()
    }
}

async fn extract_png(bytes: &[u8], config: &ExtractionConfig) -> ExtractedDocument {
    let input = ExtractInput::from_bytes(bytes.to_vec(), "image/png", None);
    let mut batch = extract(input, config).await.expect("extract the synthetic PNG");
    assert!(batch.errors.is_empty(), "{:?}", batch.errors);
    assert_eq!(batch.results.len(), 1);
    let document = batch.results.remove(0);
    assert_eq!(document.mime_type, "image/png");
    assert!(document.metadata.ocr_used);
    assert_eq!(document.metadata.additional["psm"], "11");
    assert_eq!(document.metadata.additional["ocr_processed_image_width"], 1700);
    assert_eq!(document.metadata.additional["ocr_processed_image_height"], 1100);
    document
}

async fn extract_primary(bytes: &[u8], word_count: u32, table_count: usize) -> ExtractedDocument {
    let document = extract_png(bytes, &png_config()).await;
    assert_eq!(document.tables.len(), table_count);
    let pages = document.pages.as_ref().expect("requested page output");
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].page_number, 1);
    let confidence = pages[0].ocr_confidence.as_ref().expect("original OCR confidence");
    assert_eq!(confidence.word_count, word_count);
    assert_eq!(confidence.backend, "tesseract");
    document
}

fn assert_body(text: &str, expected_sha: &str) {
    assert_eq!(hex::encode(Sha256::digest(text.as_bytes())), expected_sha, "{text:?}");
}

fn assert_addition(document: &ExtractedDocument, body_sha: &str, page_sha: &str) {
    let body = document
        .content
        .strip_suffix(TITLE_SUFFIX)
        .expect("append the independently recovered title to the unchanged OCR body");
    assert!(!body.contains(TITLE));
    assert_body(body, body_sha);
    let page = &document.pages.as_ref().unwrap()[0].content;
    let page_body = page
        .trim()
        .strip_suffix(TITLE_SUFFIX)
        .expect("append the same title on page one");
    assert_body(page_body, page_sha);
}

#[tokio::test]
async fn recovers_the_photo_title_and_keeps_the_entire_primary_body() {
    let document = extract_primary(PHOTO, 215, 0).await;
    assert_addition(
        &document,
        "56e3599e161041150f8a5363b1b8f8b6acf5ceac11adcae6528b5c47528d403e",
        "2ab44fd7d097fd8353d7c6f346ce3a4ba370e64fec2871d7131de9382012507c",
    );
}

#[tokio::test]
async fn recovers_the_photo_title_without_losing_the_independent_sign_or_amount() {
    let document = extract_primary(MIXED, 6, 0).await;
    assert!(document.content.contains("NORTH GATE 42"), "{:?}", document.content);
    assert!(document.content.contains("AUTHORIZED ONLY"), "{:?}", document.content);
    assert_addition(
        &document,
        "5b429f4f81a26747baf73316a7400b5edf39313eff0ce8dc93db6539cbbe61ff",
        "2b2f97eaeab18ccfa4ff22841fd76d969d241a34c948b0faab1844c32c20f6ac",
    );
}

#[tokio::test]
async fn leaves_accepted_tables_and_their_body_untouched() {
    let document = extract_primary(TABLE, 318, 2).await;
    assert_body(
        &document.content,
        "e19005d542795b6b3dcb3ed3a4701452fe5fb31068ce7e3ef62decbaba03fdbc",
    );
    assert_body(
        document.pages.unwrap()[0].content.trim(),
        "9fb98e2329424ee916c81a92569aab56565a3c9021ef80774ed061139c8c826a",
    );
}

#[tokio::test]
async fn leaves_a_partially_recognized_sign_at_its_existing_location() {
    let document = extract_primary(PARTIAL, 21, 0).await;
    assert!(document.content.lines().any(|line| line == "NORTH"));
    assert!(!document.content.contains("NORTH GATE"));
    assert_body(
        &document.content,
        "e0c04b85d5e3263df7e7302708b39bad37c54d6e322e74c1f3afa7f2ad202ff6",
    );
}

#[tokio::test]
async fn leaves_a_recognized_numeric_sign_untouched() {
    let document = extract_primary(NUMERIC, 3, 0).await;
    assert_eq!(document.content, "NORTH GATE 42");
}

#[tokio::test]
async fn leaves_requested_document_structure_untouched() {
    let config = ExtractionConfig {
        include_document_structure: true,
        ..png_config()
    };
    let document = extract_png(PHOTO, &config).await;
    assert!(document.document.is_some(), "structured extraction was requested");
    assert!(!document.content.contains(TITLE));
    assert!(!document.pages.unwrap()[0].content.contains(TITLE));
}

#[tokio::test]
async fn leaves_cache_enabled_extraction_on_the_primary_route() {
    let config = ExtractionConfig {
        use_cache: true,
        ..png_config()
    };
    let document = extract_png(PHOTO, &config).await;
    assert_body(
        &document.content,
        "56e3599e161041150f8a5363b1b8f8b6acf5ceac11adcae6528b5c47528d403e",
    );
}

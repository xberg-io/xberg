#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "tokio-runtime",
    feature = "bundle-tessdata-eng"
))]

//! Prose inside a raster picture that covers part of a page, beside a heading-sized text layer
//! (#2327). Fixtures come from `fixtures/pdf_picture_prose_auto_2327/g2327.py`.

use std::collections::BTreeSet;
use xberg::core::config::{ExtractInput, ExtractionConfig, OcrConfig};
use xberg::types::ExtractedDocument;

const SHORT: &[&str] = &["Heading", "Page", "7"];
const LONG: &[&str] = &[
    "Planning",
    "committee",
    "minutes:",
    "new",
    "library",
    "building",
    "proposal",
    "and",
    "parking",
    "plan",
    "Page",
    "7",
];
const PROSE_ONE: &[&str] = &["schedule", "budget", "architect", "council"];

fn config(ocr: bool) -> ExtractionConfig {
    ExtractionConfig {
        use_cache: false,
        ocr: ocr.then(OcrConfig::default),
        ..Default::default()
    }
}

async fn extract(pdf: &[u8], ocr: bool) -> ExtractedDocument {
    let mut batch = xberg::extract(
        ExtractInput::from_bytes(pdf.to_vec(), "application/pdf", None),
        &config(ocr),
    )
    .await
    .unwrap();
    assert!(batch.errors.is_empty(), "{:?}", batch.errors);
    batch.results.remove(0)
}

fn words(document: &ExtractedDocument) -> BTreeSet<String> {
    document
        .content
        .split_whitespace()
        .map(|word| word.trim_matches(|c: char| matches!(c, '.' | ',' | ';')).to_string())
        .collect()
}

fn assert_has(document: &ExtractedDocument, expected: &[&str], what: &str) {
    let found = words(document);
    for word in expected {
        assert!(
            found.contains(*word),
            "{what}: missing {word:?} in {:?}",
            document.content
        );
    }
}

macro_rules! fixture {
    ($name:literal) => {
        include_bytes!(concat!("fixtures/pdf_picture_prose_auto_2327/pdfs/", $name, ".pdf")).as_slice()
    };
}

#[tokio::test]
async fn bottom_half_picture_with_short_heading_is_read() {
    let document = extract(fixture!("half_short"), true).await;
    assert_has(&document, SHORT, "heading");
    assert_has(&document, PROSE_ONE, "prose");
}

#[tokio::test]
async fn bottom_half_picture_with_long_heading_is_read() {
    let document = extract(fixture!("half_long"), true).await;
    assert_has(&document, LONG, "heading");
    assert_has(&document, PROSE_ONE, "prose");
}

#[tokio::test]
async fn side_column_picture_with_long_heading_is_read() {
    let document = extract(fixture!("side_long"), true).await;
    assert_has(&document, LONG, "heading");
    assert_has(&document, &["budget", "architect", "council", "staff"], "prose");
}

#[tokio::test]
async fn two_pictures_with_short_heading_are_both_read() {
    let document = extract(fixture!("two_short"), true).await;
    assert_has(&document, SHORT, "heading");
    assert_has(&document, &["schedule", "budget", "architect", "council"], "prose");
}

#[tokio::test]
async fn photo_with_caption_keeps_exactly_its_native_words() {
    for (name, pdf, heading) in [
        ("short", fixture!("photo_short"), SHORT),
        ("long", fixture!("photo_long"), LONG),
    ] {
        let document = extract(pdf, true).await;
        let expected: BTreeSet<String> = heading
            .iter()
            .chain(&["Figure", "1", "A", "caption", "beside", "the", "photograph"])
            .map(|word| word.to_string())
            .collect();
        assert_eq!(words(&document), expected, "{name}: {:?}", document.content);
        assert_eq!(document.extraction_method, Some(xberg::types::ExtractionMethod::Native));
    }
}

/// Without an `ocr` block no OCR runs on a page that has any native text (#1752, #1338): the
/// native layer is kept and the picture's prose is not read.
#[tokio::test]
async fn without_an_ocr_block_the_native_text_is_kept_and_the_picture_is_not_read() {
    for (name, pdf, heading) in [
        ("short", fixture!("half_short"), SHORT),
        ("long", fixture!("half_long"), LONG),
    ] {
        let document = extract(pdf, false).await;
        let expected: BTreeSet<String> = heading.iter().map(|word| word.to_string()).collect();
        assert_eq!(words(&document), expected, "{name}: {:?}", document.content);
        assert_eq!(document.extraction_method, Some(xberg::types::ExtractionMethod::Native));
    }
}

#[test]
fn prose_words_asserted_are_not_in_any_native_heading() {
    for heading in [SHORT, LONG] {
        for word in PROSE_ONE.iter().chain(&["staff"]) {
            assert!(!heading.contains(word), "{word} is a native heading word");
        }
    }
}

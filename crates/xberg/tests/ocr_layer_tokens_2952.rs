#![cfg(all(feature = "pdf", feature = "ocr", feature = "bundle-tessdata-eng"))]

use std::collections::BTreeMap;
use xberg::{ExtractInput, ExtractionConfig, OcrConfig, OutputFormat, PageConfig};

/// One page: a header in a Type0 font whose ToUnicode map points into the Private Use Area (so the
/// header layer is undecodable and automatic OCR takes the page), then 18 identifiers, a bar chart
/// with five labels and values, and a footer page number in Helvetica with a correct layer.
/// All text is invented.
const IDS_PDF: &[u8] = include_bytes!("fixtures/ocr_layer_tokens_2952/ids.pdf");

const IDENTIFIERS: [&str; 18] = [
    "AB1234", "XK7021", "B0X92Q", "QR5L18", "1ZA340", "MN80P4", "TV2B77", "D4K9L1", "ZZ0185", "HG6R03", "P9W2E5",
    "C7V1B8", "LM3X46", "S5T0U9", "J8H4G2", "RF1D29", "Y6N7M3", "K2Q5W0",
];
const LABELS: [&str; 5] = ["Norte", "Sur", "Este", "Oeste", "Centro"];
const VALUES: [&str; 5] = ["120", "80", "150", "60", "100"];

async fn page_text(tessdata: &std::path::Path, format: OutputFormat) -> String {
    let model = xberg_tesseract::bundled_eng_traineddata().expect("bundled English model");
    std::fs::write(tessdata.join("eng.traineddata"), model).unwrap();
    let config = ExtractionConfig {
        use_cache: false,
        output_format: format,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ocr: Some(OcrConfig {
            enabled: true,
            backend: "tesseract".into(),
            language: vec!["eng".into()],
            tessdata_path: Some(tessdata.to_path_buf()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let output = xberg::extract(
        ExtractInput::from_bytes(IDS_PDF.to_vec(), "application/pdf", None),
        &config,
    )
    .await
    .expect("extract the reproducer page");
    assert!(output.errors.is_empty(), "{:?}", output.errors);
    let [document] = <[_; 1]>::try_from(output.results).ok().expect("one document");
    assert!(
        document.metadata.ocr_used,
        "the undecodable header must send the page to OCR: {:?}",
        document.extraction_method
    );
    let pages = document.pages.expect("per-page output requested");
    assert_eq!(pages.len(), 1);
    pages.into_iter().next().unwrap().content
}

fn token_counts(text: &str) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for token in text.split(|character: char| character.is_whitespace() || character == '|') {
        if !token.is_empty() {
            *counts.entry(token.to_string()).or_insert(0) += 1;
        }
    }
    counts
}

/// `B0X92Q` is read as `BOX92Q` with recognition confidence 0.82, above the 0.75 below which a
/// read word may be corrected, so it stays as read; every other token is recovered.
const READ_AS_CONFIDENT: &str = "B0X92Q";

fn assert_layer_tokens_recovered(text: &str) {
    let counts = token_counts(text);
    for token in IDENTIFIERS
        .iter()
        .filter(|token| **token != READ_AS_CONFIDENT)
        .chain(&LABELS)
        .chain(&VALUES)
        .chain(&["7"])
    {
        assert_eq!(
            counts.get(*token),
            Some(&1),
            "`{token}` must appear exactly once in:\n{text}"
        );
    }
    let confident = counts.get(READ_AS_CONFIDENT).copied().unwrap_or(0) + counts.get("BOX92Q").copied().unwrap_or(0);
    assert_eq!(
        confident, 1,
        "the confident misread must appear once, as read or as the layer has it:\n{text}"
    );
}

#[tokio::test]
async fn automatic_ocr_keeps_every_decodable_layer_token_of_a_page_with_an_undecodable_header() {
    let directory = tempfile::tempdir().unwrap();
    assert_layer_tokens_recovered(&page_text(directory.path(), OutputFormat::Plain).await);
}

#[tokio::test]
async fn the_structured_markdown_page_keeps_the_same_layer_tokens_in_reading_order() {
    let directory = tempfile::tempdir().unwrap();
    let text = page_text(directory.path(), OutputFormat::Markdown).await;
    assert_layer_tokens_recovered(&text);
    let tokens: Vec<&str> = text
        .split(|c: char| c.is_whitespace() || c == '|')
        .filter(|t| !t.is_empty())
        .collect();
    let at = |token: &str| tokens.iter().position(|candidate| *candidate == token).unwrap();
    assert!(
        at("100") < at("80") && at("80") < at("60") && at("60") < at("7"),
        "{text}"
    );
}

#![cfg(all(feature = "pdf", feature = "ocr", feature = "bundle-tessdata-eng"))]

use std::collections::BTreeMap;
use xberg::types::ExtractionMethod;
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

/// `ids.pdf` behind a clean, fully decodable first page: only the second page needs OCR, so the
/// document takes the mixed native and OCR route, whose page decision compares each OCR page with
/// the layer it keeps.
#[cfg(any(feature = "paddle-ocr-ort", feature = "paddle-ocr-tract"))]
const MIXED_PDF: &[u8] = include_bytes!("fixtures/ocr_layer_tokens_2952/mixed.pdf");

async fn page_text(tessdata: &std::path::Path, format: OutputFormat) -> String {
    pages_of(IDS_PDF, tessdata, format, ExtractionMethod::Ocr)
        .await
        .remove(0)
}

async fn pages_of(
    pdf: &[u8],
    tessdata: &std::path::Path,
    format: OutputFormat,
    expected_method: ExtractionMethod,
) -> Vec<String> {
    let model = xberg_tesseract::bundled_eng_traineddata().expect("bundled English model");
    std::fs::write(tessdata.join("eng.traineddata"), model).unwrap();
    let config = ExtractionConfig {
        use_cache: false,
        output_format: format,
        // Exercise the same structured OCR read for Plain and Markdown. ~keep
        include_document_structure: true,
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
    let output = xberg::extract(ExtractInput::from_bytes(pdf.to_vec(), "application/pdf", None), &config)
        .await
        .expect("extract the reproducer page");
    assert!(output.errors.is_empty(), "{:?}", output.errors);
    let [document] = <[_; 1]>::try_from(output.results)
        .unwrap_or_else(|results| panic!("expected one document, got {}", results.len()));
    assert!(
        document.metadata.ocr_used,
        "the undecodable header must send the page to OCR: {:?}",
        document.extraction_method
    );
    assert_eq!(document.extraction_method, Some(expected_method));
    let pages = document.pages.expect("per-page output requested");
    pages.into_iter().map(|page| page.content).collect()
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

// Platform OCR can leave these substitutions or split a token across words. ~keep
const OCR_VARIANTS: [(&str, &str); 3] = [("B0X92Q", "BOX92Q"), ("S5T0U9", "S5TOU9"), ("J8H4G2", "J 8H4G2")];

fn assert_layer_tokens_recovered(text: &str) {
    let counts = token_counts(text);
    for token in IDENTIFIERS.iter().chain(&LABELS).chain(&VALUES).chain(&["7"]) {
        let variants = OCR_VARIANTS.iter().find(|(source, _)| source == token);
        let alternate_count = variants.map_or(0, |(_, alternate)| {
            if let Some((first, second)) = alternate.split_once(' ') {
                text.split_whitespace()
                    .zip(text.split_whitespace().skip(1))
                    .filter(|&(left, right)| left == first && right == second)
                    .count()
            } else {
                counts.get(*alternate).copied().unwrap_or(0)
            }
        });
        assert_eq!(
            counts.get(*token).copied().unwrap_or(0) + alternate_count,
            1,
            "`{token}` must appear exactly once in:\n{text}"
        );
    }
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

// The implicit fallback routes mixed OCR through the per-page stage that consumes the decision layer. ~keep
#[cfg(any(feature = "paddle-ocr-ort", feature = "paddle-ocr-tract"))]
#[tokio::test]
async fn the_mixed_route_keeps_the_layer_tokens_of_its_ocr_page() {
    let directory = tempfile::tempdir().unwrap();
    let pages = pages_of(
        MIXED_PDF,
        directory.path(),
        OutputFormat::Plain,
        ExtractionMethod::Mixed,
    )
    .await;
    assert_eq!(pages.len(), 2);
    assert!(pages[0].contains("Quarterly regional summary"), "{}", pages[0]);
    assert_layer_tokens_recovered(&pages[1]);
}

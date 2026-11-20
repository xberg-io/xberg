#![cfg(all(feature = "ocr", feature = "bundle-tessdata-eng", feature = "tokio-runtime"))]

//! #3337: 100 DPI tinted scans lose their small digits when the raster reaches Tesseract at its own
//! resolution. The tinted paper made the default preprocessing — and with it the 300 DPI resample —
//! look like a non-page, so the digits were read from 9 px (and 7 px) glyphs. These fixtures are the
//! #2432/#2503 table shape rendered on the reporter's tint (214,224,238); the 300 DPI one is the
//! same layout at a resolution whose glyphs need no resample.

mod helpers;

use std::collections::HashSet;

use sha2::{Digest, Sha256};
use xberg::types::ImagePreprocessingMetadata;
use xberg::{ExtractedDocument, ExtractionConfig, OcrConfig, OutputFormat, PageConfig, TesseractConfig};

const TINTED_100_DPI_9PX: &[u8] = include_bytes!("fixtures/ocr_low_ppi_tinted_3337/tinted-100-9px.png");
const TINTED_100_DPI_7PX: &[u8] = include_bytes!("fixtures/ocr_low_ppi_tinted_3337/tinted-100-7px.png");
const TINTED_300_DPI: &[u8] = include_bytes!("fixtures/ocr_low_ppi_tinted_3337/tinted-300-9px.png");

/// The generator writes 101..=172 into six columns by twelve rows: 72 source numbers.
fn source_numbers() -> std::ops::RangeInclusive<u16> {
    101..=172
}

/// Numbers of 72 that must survive recognition, at PSM 11.
///
/// Before the resample the same fixtures kept 65 (9 px) and 49 (7 px); with it they kept 72 and 71
/// (vendored Tesseract 5.5.4, macOS arm64). The floors sit between the two so a reverted decision
/// fails while a recognizer that reads one or two numbers differently does not.
const KEPT_100_DPI_9PX: usize = 68;
const KEPT_100_DPI_7PX: usize = 60;
const KEPT_300_DPI: usize = 68;

fn fixture(bytes: &[u8], sha256: &str) -> Vec<u8> {
    assert_eq!(hex::encode(Sha256::digest(bytes)), sha256);
    bytes.to_vec()
}

fn extract(bytes: &[u8], psm: i32) -> ExtractedDocument {
    let config = ExtractionConfig {
        use_cache: false,
        output_format: OutputFormat::Plain,
        ocr: Some(OcrConfig {
            tesseract_config: Some(TesseractConfig {
                psm: Some(psm),
                ..Default::default()
            }),
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let extracted = helpers::extract_bytes_document_blocking(bytes, "image/png", &config).unwrap();
    assert!(!extracted.content.is_empty(), "the fixture must be readable");
    extracted
}

fn preprocessing(document: &ExtractedDocument) -> ImagePreprocessingMetadata {
    document
        .metadata
        .image_preprocessing
        .clone()
        .or_else(|| {
            document
                .pages
                .as_deref()
                .and_then(|pages| pages.first())
                .and_then(|page| page.image_preprocessing.clone())
        })
        .expect("a page-like raster must take the default preprocessing")
}

/// Source numbers the recognition published as their own token, in order-independent form.
fn kept_numbers(text: &str) -> Vec<u16> {
    let tokens: HashSet<String> = text
        .split(|character: char| !character.is_ascii_digit())
        .filter(|token| !token.is_empty())
        .map(ToString::to_string)
        .collect();
    source_numbers()
        .filter(|number| tokens.contains(&number.to_string()))
        .collect()
}

fn assert_kept(text: &str, minimum: usize, label: &str) -> usize {
    let kept = kept_numbers(text);
    assert!(
        kept.len() >= minimum,
        "{label}: kept {} of 72 numbers ({kept:?})\n{text}",
        kept.len()
    );
    kept.len()
}

fn tinted_100_dpi_9px() -> Vec<u8> {
    fixture(
        TINTED_100_DPI_9PX,
        "aee252d18330c1827e95e0616c08088c5b46f626596cbe00dcdf205bd31bf832",
    )
}

#[test]
fn one_hundred_dpi_tinted_table_resamples_and_keeps_its_numbers() {
    let document = extract(&tinted_100_dpi_9px(), 11);

    let metadata = preprocessing(&document);
    assert!(
        (metadata.original_dpi.horizontal - 100.0).abs() < 0.01,
        "the PNG density is 100 DPI, got {}",
        metadata.original_dpi.horizontal
    );
    assert!(
        !metadata.skipped_resize,
        "a 100 DPI tinted page with 9 px digits must be resampled"
    );
    assert!(
        (metadata.scale_factor - 3.0).abs() < 0.01,
        "the 300 DPI target is 3x the 100 DPI source, got {}",
        metadata.scale_factor
    );

    let kept = assert_kept(&document.content, KEPT_100_DPI_9PX, "document");
    if let Some(pages) = document.pages.as_deref() {
        let [page] = pages else { panic!("one source page") };
        assert_eq!(assert_kept(&page.content, KEPT_100_DPI_9PX, "page"), kept);
    }
}

#[test]
fn seven_pixel_glyphs_on_a_tinted_page_survive_recognition() {
    let bytes = fixture(
        TINTED_100_DPI_7PX,
        "5af0e10e0a92130a61534e15ccd42f85aa052b3726bdfd09004a3c9b37e334e3",
    );
    let document = extract(&bytes, 11);

    let metadata = preprocessing(&document);
    assert!(!metadata.skipped_resize);
    assert!(
        (metadata.scale_factor - 3.0).abs() < 0.01,
        "7 px glyphs at 100 DPI keep the 3x resample to the 300 DPI target, got {}",
        metadata.scale_factor
    );

    assert_kept(&document.content, KEPT_100_DPI_7PX, "document");
}

#[test]
fn three_hundred_dpi_tinted_table_is_not_resampled() {
    let bytes = fixture(
        TINTED_300_DPI,
        "8df42b2796c2e6ffa4cf02f38890ba47755a811c8ecf06d8e552791d6e2abb97",
    );
    let document = extract(&bytes, 11);

    let metadata = preprocessing(&document);
    assert!(
        (metadata.original_dpi.horizontal - 300.0).abs() < 0.01,
        "the PNG density is 300 DPI, got {}",
        metadata.original_dpi.horizontal
    );
    assert!(
        metadata.skipped_resize,
        "a 300 DPI page already carries enough pixels for its glyphs"
    );
    assert!((metadata.scale_factor - 1.0).abs() < 1e-3);

    assert_kept(&document.content, KEPT_300_DPI, "document");
}

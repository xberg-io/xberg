//! Scanned-PDF detection against the real corpus.
//!
//! Every config leaves `ocr` unset, so detection runs without an OCR backend.

#![cfg(feature = "pdf")]

use std::path::{Path, PathBuf};

use xberg::core::config::{DEFAULT_SCANNED_MIN_CONFIDENCE, ExtractInput, ExtractionConfig, OcrStrategy};
use xberg::types::FormatMetadata;

fn corpus(relative: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test_documents")
        .join(relative);
    assert!(path.exists(), "missing corpus fixture: {}", path.display());
    path
}

/// Extract a corpus PDF and return `(scanned_confidence, scanned_pages)`.
async fn scan_metadata(relative: &str, config: &ExtractionConfig) -> (Option<f32>, Option<Vec<u32>>) {
    let input = ExtractInput::from_uri(corpus(relative).to_string_lossy().into_owned());
    let result = xberg::extract(input, config)
        .await
        .unwrap_or_else(|error| panic!("extraction failed for {relative}: {error}"));

    assert!(result.errors.is_empty(), "{relative}: {:?}", result.errors);
    let document = result
        .results
        .first()
        .unwrap_or_else(|| panic!("{relative}: no result"));

    match &document.metadata.format {
        Some(FormatMetadata::Pdf(pdf)) => (pdf.scanned_confidence, pdf.scanned_pages.clone()),
        other => panic!("expected PDF metadata for {relative}, got {other:?}"),
    }
}

/// Every page of these fixtures is a full-page raster with no text layer.
#[tokio::test]
async fn scanned_fixtures_report_high_confidence() {
    let config = ExtractionConfig::default();

    for fixture in [
        "pdf_scanned/nougat_004_scanned.pdf",
        "pdf_scanned/nougat_009_scanned.pdf",
        "pdf_scanned/nougat_010_scanned.pdf",
    ] {
        let (confidence, pages) = scan_metadata(fixture, &config).await;
        let confidence = confidence.unwrap_or_else(|| panic!("{fixture}: no scanned_confidence"));

        assert!(
            f64::from(confidence) >= DEFAULT_SCANNED_MIN_CONFIDENCE,
            "{fixture}: confidence {confidence} is below the default threshold"
        );
        assert_eq!(
            pages.as_deref(),
            Some(&[1_u32][..]),
            "{fixture}: page 1 should be a scan"
        );
    }
}

/// A full-bleed background image with *visible* text over it is not a scan.
#[tokio::test]
async fn born_digital_slides_are_not_scans() {
    let (confidence, pages) = scan_metadata(
        "pdf/100_g_networking_technology_overview_slides_toronto_august_2016.pdf",
        &ExtractionConfig::default(),
    )
    .await;

    let confidence = confidence.expect("no scanned_confidence");
    assert!(
        f64::from(confidence) < DEFAULT_SCANNED_MIN_CONFIDENCE,
        "born-digital slides scored {confidence}, at or above the default threshold"
    );
    assert_eq!(
        pages.as_deref(),
        Some(&[][..]),
        "no page of a born-digital slide deck is a scan"
    );
}

/// A text-heavy born-digital document: no page qualifies.
#[tokio::test]
async fn born_digital_text_document_has_no_scanned_pages() {
    let (_, pages) = scan_metadata(
        "pdf/a_brief_introduction_to_the_standard_annotation_language_sal_2006.pdf",
        &ExtractionConfig::default(),
    )
    .await;

    assert_eq!(pages.as_deref(), Some(&[][..]));
}

/// The fixture scores `0.85`: the default threshold reports page 1, `0.9` none.
#[tokio::test]
async fn min_confidence_controls_which_pages_are_reported() {
    let fixture = "pdf_scanned/nougat_004_scanned.pdf";

    let (_, default_pages) = scan_metadata(fixture, &ExtractionConfig::default()).await;
    assert_eq!(default_pages.as_deref(), Some(&[1_u32][..]));

    let strict = ExtractionConfig {
        ocr_strategy: OcrStrategy::ScannedPages { min_confidence: 0.9 },
        ..Default::default()
    };
    let (_, strict_pages) = scan_metadata(fixture, &strict).await;
    assert_eq!(
        strict_pages.as_deref(),
        Some(&[][..]),
        "a 0.9 threshold should reject a 0.85-confidence scan"
    );
}

/// Detection is advisory: a truncated PDF must not panic.
#[tokio::test]
async fn truncated_pdf_does_not_panic_during_detection() {
    let bytes = std::fs::read(corpus("pdf_scanned/nougat_004_scanned.pdf")).expect("read fixture");
    let truncated = bytes[..bytes.len() / 2].to_vec();

    let input = ExtractInput::from_bytes(truncated, "application/pdf", None);
    let outcome = xberg::extract(input, &ExtractionConfig::default()).await;

    if let Ok(result) = outcome
        && let Some(document) = result.results.first()
        && let Some(FormatMetadata::Pdf(pdf)) = &document.metadata.format
        && let Some(confidence) = pdf.scanned_confidence
    {
        assert!(
            (0.0..=1.0).contains(&confidence),
            "confidence {confidence} escaped [0,1]"
        );
    }
}

/// `ScannedPages` with `disable_ocr` is rejected at both entry points.
#[tokio::test]
async fn scanned_pages_with_disable_ocr_is_rejected() {
    let config = ExtractionConfig {
        disable_ocr: true,
        ocr_strategy: OcrStrategy::ScannedPages {
            min_confidence: DEFAULT_SCANNED_MIN_CONFIDENCE,
        },
        ..Default::default()
    };

    let from_uri = xberg::extract(
        ExtractInput::from_uri(
            corpus("pdf_scanned/nougat_004_scanned.pdf")
                .to_string_lossy()
                .into_owned(),
        ),
        &config,
    )
    .await;
    assert!(
        from_uri.as_ref().is_err() || !from_uri.as_ref().unwrap().errors.is_empty(),
        "the URI entry point must reject ocr_strategy=scanned_pages with disable_ocr"
    );

    let bytes = std::fs::read(corpus("pdf_scanned/nougat_004_scanned.pdf")).expect("read fixture");
    let from_bytes = xberg::extract(ExtractInput::from_bytes(bytes, "application/pdf", None), &config).await;
    assert!(
        from_bytes.as_ref().is_err() || !from_bytes.as_ref().unwrap().errors.is_empty(),
        "the bytes entry point must reject ocr_strategy=scanned_pages with disable_ocr"
    );
}

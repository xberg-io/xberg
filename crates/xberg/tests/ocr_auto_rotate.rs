//! Regression tests for explicit OCR orientation and implicit PDF recovery.
//!
//! Tesseract's `DetectOrientationScript` corrupts engine memory in the vendored
//! build (SIGABRT/SIGSEGV, uncatchable), so orientation detection runs on the
//! ONNX PP-LCNet classifier instead. This test drives the tesseract backend with
//! `auto_rotate: true` end-to-end; before the fix the whole test process died
//! with a signal.
#![cfg(all(feature = "ocr", any(feature = "auto-rotate", feature = "pdf")))]

#[cfg(feature = "auto-rotate")]
#[tokio::test]
async fn should_extract_with_auto_rotate_without_crashing() {
    let bytes = std::fs::read("../../test_documents/images/balance_sheet_1.png").expect("fixture must exist");
    let config = xberg::ExtractionConfig {
        ocr: Some(xberg::core::config::OcrConfig {
            backend: "tesseract".to_string(),
            language: vec!["eng".to_string()],
            auto_rotate: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let input = xberg::core::config::ExtractInput {
        kind: xberg::core::config::ExtractInputKind::Bytes,
        bytes: Some(bytes),
        mime_type: Some("image/png".to_string()),
        ..Default::default()
    };

    let result = xberg::extract(input, &config).await.expect("extraction must succeed");
    let text = result.results.first().map(|d| d.content.clone()).unwrap_or_default();
    assert!(
        !text.trim().is_empty(),
        "OCR with auto_rotate must still produce text output"
    );
}

#[cfg(feature = "pdf")]
mod helpers;

#[cfg(feature = "pdf")]
fn rotated_pdf() -> (Vec<u8>, xberg::ExtractionConfig) {
    use xberg::core::config::PageConfig;

    let path = helpers::get_test_file_path("pdf/ocr_test_rotated_90.pdf");
    let bytes = std::fs::read(path).expect("rotated PDF fixture");
    assert!(bytes.starts_with(b"%PDF-"));
    let config = xberg::ExtractionConfig {
        force_ocr: true,
        use_cache: false,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    assert!(config.ocr.is_none());
    (bytes, config)
}

#[cfg(all(feature = "pdf", any(feature = "auto-rotate", feature = "auto-rotate-tract")))]
#[test]
#[ignore = "requires the verified document-orientation model in the local cache and the PDF fixture"]
fn implicit_pdf_ocr_recovers_a_low_confidence_rotated_page() {
    use xberg::core::config::{ExtractionConfig, OcrConfig};

    let (bytes, config) = rotated_pdf();

    let extract = |config: &ExtractionConfig| {
        helpers::extract_bytes_document_blocking(&bytes, "application/pdf", config).expect("PDF OCR succeeds")
    };
    let normalize = |text: &str| text.split_whitespace().collect::<Vec<_>>().join(" ");
    let expected = "Docling bundles PDF document conversion to JSON and Markdown in an easy self contained package";
    let implicit = extract(&config);
    let explicit = extract(&ExtractionConfig {
        ocr: Some(OcrConfig {
            auto_rotate: true,
            ..Default::default()
        }),
        ..config.clone()
    });
    assert_eq!(normalize(&explicit.content), expected, "cached orientation control");
    let disabled = extract(&ExtractionConfig {
        ocr: Some(OcrConfig {
            auto_rotate: false,
            ..Default::default()
        }),
        ..config
    });
    assert_eq!(normalize(&disabled.content), "zy O O");
    assert!(
        disabled
            .processing_warnings
            .iter()
            .any(|warning| warning.message.contains("recognition noise"))
    );

    let pages = implicit.pages.as_ref().expect("page extraction is enabled");
    assert_eq!(pages.len(), 1);
    assert!(pages[0].ocr_confidence.is_some(), "Tesseract must have run");
    assert_eq!(normalize(&implicit.content), expected, "implicit rotated PDF OCR");
    assert_eq!(normalize(&pages[0].content), expected);
    assert!(
        implicit
            .processing_warnings
            .iter()
            .all(|warning| warning.source != "ocr"),
        "{:?}",
        implicit.processing_warnings
    );
}

#[cfg(feature = "pdf")]
#[test]
#[ignore = "requires the rotated PDF fixture and an isolated empty model cache"]
fn implicit_pdf_ocr_keeps_its_original_warning_when_no_orientation_model_is_available() {
    use xberg::core::config::{ExtractionConfig, OcrConfig};

    let (bytes, config) = rotated_pdf();
    let extract = |config: &ExtractionConfig| {
        helpers::extract_bytes_document_blocking(&bytes, "application/pdf", config).expect("PDF OCR succeeds")
    };
    let implicit = extract(&config);
    let disabled = extract(&ExtractionConfig {
        ocr: Some(OcrConfig {
            auto_rotate: false,
            ..Default::default()
        }),
        ..config
    });
    assert_eq!(implicit.content, disabled.content);
    assert_eq!(
        implicit.content.split_whitespace().collect::<Vec<_>>(),
        ["zy", "O", "O"]
    );
    let warnings = |doc: &xberg::ExtractedDocument| {
        doc.processing_warnings
            .iter()
            .map(|w| (w.source.clone(), w.message.clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(warnings(&implicit), warnings(&disabled));
    assert!(
        implicit
            .processing_warnings
            .iter()
            .any(|w| w.message.contains("recognition noise"))
    );
}

#[cfg(all(feature = "pdf", any(feature = "auto-rotate", feature = "auto-rotate-tract")))]
#[test]
#[ignore = "requires the upright PDF fixture and a local orientation model cache"]
fn implicit_upright_pdf_ocr_preserves_text_confidence_and_geometry() {
    use xberg::core::config::{ExtractionConfig, OcrConfig, PageConfig};

    let bytes = std::fs::read(helpers::get_test_file_path("pdf/ocr_test.pdf")).expect("upright PDF fixture");
    let config = ExtractionConfig {
        force_ocr: true,
        use_cache: false,
        include_document_structure: true,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let extract = |config: &ExtractionConfig| {
        helpers::extract_bytes_document_blocking(&bytes, "application/pdf", config).expect("PDF OCR succeeds")
    };
    let implicit = extract(&config);
    let disabled = extract(&ExtractionConfig {
        ocr: Some(OcrConfig {
            auto_rotate: false,
            ..Default::default()
        }),
        ..config
    });
    let expected = "Docling bundles PDF document conversion to JSON and Markdown in an easy self contained package";
    assert_eq!(
        implicit.content.split_whitespace().collect::<Vec<_>>().join(" "),
        expected
    );
    assert_eq!(implicit.content, disabled.content);
    assert!(implicit.processing_warnings.is_empty());
    let implicit_pages = implicit.pages.as_ref().expect("implicit page payload");
    let disabled_pages = disabled.pages.as_ref().expect("explicit page payload");
    assert_eq!(implicit_pages.len(), 1);
    assert_eq!(implicit_pages[0].content, disabled_pages[0].content);
    let confidence = |page: &xberg::types::PageContent| {
        page.ocr_confidence
            .as_ref()
            .and_then(|confidence| confidence.score)
            .expect("actual OCR page confidence")
    };
    assert_eq!(confidence(&implicit_pages[0]), confidence(&disabled_pages[0]));

    let implicit_structure = implicit.document.as_ref().expect("implicit OCR structure");
    let disabled_structure = disabled.document.as_ref().expect("explicit OCR structure");
    assert_eq!(
        implicit_structure
            .nodes
            .iter()
            .filter(|node| node.bbox.is_some())
            .count(),
        3
    );
    assert_eq!(
        serde_json::to_value(implicit_structure).expect("implicit geometry"),
        serde_json::to_value(disabled_structure).expect("explicit geometry")
    );
}

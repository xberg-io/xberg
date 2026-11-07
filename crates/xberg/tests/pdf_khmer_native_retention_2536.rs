#![cfg(all(feature = "pdf", feature = "ocr", feature = "bundle-tessdata-eng", not(paddle_ocr)))]

mod helpers;

use sha2::{Digest, Sha256};
use xberg::core::config::{OcrStrategy, PageConfig};
use xberg::types::{ExtractedDocument, ExtractionMethod};
use xberg::{ExtractionConfig, OcrConfig, OutputFormat};

const COMPLETE: &[u8] = include_bytes!("fixtures/pdf/khmer-cluster-2536/cluster-complete-mapped.pdf");
const PARTIAL: &[u8] = include_bytes!("fixtures/pdf/khmer-cluster-2536/genuinely-partial-mapped.pdf");

#[test]
fn automatic_english_ocr_preserves_known_native_khmer_even_when_mapping_is_partial() {
    let models = tempfile::tempdir().unwrap();
    let eng = xberg_tesseract::bundled_eng_traineddata().expect("real English OCR model");
    assert_eq!(
        hex::encode(Sha256::digest(eng)),
        "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2"
    );
    std::fs::write(models.path().join("eng.traineddata"), eng).unwrap();
    let config = ExtractionConfig {
        use_cache: false,
        output_format: OutputFormat::Plain,
        include_document_structure: true,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ocr: Some(OcrConfig {
            tessdata_path: Some(models.path().into()),
            ..Default::default()
        }),
        ocr_strategy: OcrStrategy::Auto,
        ..Default::default()
    };

    for (source, mapped, sha) in [
        (
            COMPLETE,
            222,
            "67e7e66acc711192b563483721a35f972113f1c5d3977d7db4fcf19d52d7f955",
        ),
        (
            PARTIAL,
            28,
            "f14718c7754556e59ce64f6075f6e30a2a5e74622d56a102a5ab974dac1b6a40",
        ),
    ] {
        assert_eq!(hex::encode(Sha256::digest(source)), sha);
        assert_preserves_mapped_occurrences(source, mapped, &config);
    }
}

fn assert_preserves_mapped_occurrences(source: &[u8], mapped: usize, config: &ExtractionConfig) {
    let run = |config: &ExtractionConfig| {
        let result = helpers::extract_bytes_document_blocking(source, "application/pdf", config).unwrap();
        assert_eq!(result.pages.as_ref().unwrap().len(), 1);
        assert!(!result.content.trim().is_empty());
        result
    };
    let native = run(&ExtractionConfig {
        disable_ocr: true,
        ..config.clone()
    });
    let forced = run(&ExtractionConfig {
        force_ocr: true,
        ..config.clone()
    });
    let automatic = run(config);
    assert_eq!(native.extraction_method, Some(ExtractionMethod::Native));
    assert_eq!(forced.extraction_method, Some(ExtractionMethod::Ocr));
    assert_eq!(khmer_count(&native), mapped);
    assert_eq!(khmer_count(&forced), 0);
    assert_eq!(khmer_count(&automatic), mapped);
    assert_eq!(automatic.extraction_method, Some(ExtractionMethod::Native));
    assert_eq!(automatic.content, native.content);
    assert_eq!(
        automatic.pages.as_ref().unwrap()[0].content,
        native.pages.as_ref().unwrap()[0].content
    );
    assert_eq!(
        serde_json::to_value(automatic.document.as_ref()).unwrap(),
        serde_json::to_value(native.document.as_ref()).unwrap()
    );
    assert!(
        automatic
            .processing_warnings
            .iter()
            .any(|warning| warning.source == "pdf" && warning.message.contains("no Unicode value"))
    );
}

fn khmer_count(result: &ExtractedDocument) -> usize {
    result
        .content
        .chars()
        .filter(|c| ('\u{1780}'..='\u{17ff}').contains(c))
        .count()
}

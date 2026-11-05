#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "tokio-runtime",
    feature = "bundle-tessdata-eng",
    not(paddle_ocr)
))]

mod helpers;

use sha2::{Digest, Sha256};
use xberg::core::config::OcrQualityThresholds;
use xberg::types::ExtractionMethod;
use xberg::{ExtractionConfig, OcrConfig, OutputFormat, PageConfig};

const PHOTO: &[u8] = include_bytes!("fixtures/pdf/discarded-noise-2312/photo-title.pdf");

#[test]
fn discarded_automatic_ocr_preserves_the_native_photo_title_and_its_structure() {
    assert_eq!(
        hex::encode(Sha256::digest(PHOTO)),
        "d9c21380343a4c9d512eb590f67958e3ff16098edf18b9d9b9f8e84a174639b0"
    );
    let models = bundled_model();
    let extract =
        |config: &ExtractionConfig| helpers::extract_bytes_document_blocking(PHOTO, "application/pdf", config).unwrap();
    for output_format in [OutputFormat::Plain, OutputFormat::Markdown] {
        let config = discard_noise_config(models.path(), output_format);
        let native = extract(&ExtractionConfig {
            disable_ocr: true,
            ..config.clone()
        });
        assert_eq!(native.content.trim(), "SECTION TITLE");
        let forced = extract(&ExtractionConfig {
            force_ocr: true,
            ..config.clone()
        });
        assert_eq!(forced.extraction_method, Some(ExtractionMethod::Ocr));
        assert!(forced.content.trim().is_empty());
        let auto = extract(&config);
        assert!(
            auto.processing_warnings
                .iter()
                .any(|w| { w.source == "ocr" && w.message.contains("discard_suspected_ocr_noise is enabled") })
        );
        assert_eq!(auto.extraction_method, Some(ExtractionMethod::Native));
        assert_eq!(auto.metadata.ocr_used, native.metadata.ocr_used);
        assert_eq!(auto.content, native.content);
        assert_eq!(
            auto.pages.as_ref().unwrap()[0].content,
            native.pages.as_ref().unwrap()[0].content
        );
        assert_eq!(
            serde_json::to_value(&auto.document).unwrap(),
            serde_json::to_value(&native.document).unwrap()
        );
    }
}

#[test]
fn a_same_colour_native_title_does_not_replace_discarded_ocr() {
    const SOURCE: &[u8] = include_bytes!("fixtures/pdf/discarded-noise-2312/same-colour-title.pdf");
    assert_eq!(
        hex::encode(Sha256::digest(SOURCE)),
        "3ff8d471039963ace93f29358ca95f5ed289e944a3336e6c13a9102bb685b5f9"
    );
    let models = bundled_model();
    let config = discard_noise_config(models.path(), OutputFormat::Plain);
    let extract = |config: &ExtractionConfig| {
        helpers::extract_bytes_document_blocking(SOURCE, "application/pdf", config).unwrap()
    };
    let native = extract(&ExtractionConfig {
        disable_ocr: true,
        ..config.clone()
    });
    assert_eq!(native.content.trim(), "SECTION TITLE");
    let auto = extract(&config);
    assert!(auto.processing_warnings.iter().any(|warning| {
        warning.source == "ocr" && warning.message.contains("discard_suspected_ocr_noise is enabled")
    }));
    assert_eq!(auto.extraction_method, Some(ExtractionMethod::Ocr));
    assert!(auto.content.trim().is_empty());
}

fn bundled_model() -> tempfile::TempDir {
    let models = tempfile::tempdir().unwrap();
    let eng = xberg_tesseract::bundled_eng_traineddata().expect("bundled English OCR model");
    assert_eq!(
        hex::encode(Sha256::digest(eng)),
        "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2"
    );
    std::fs::write(models.path().join("eng.traineddata"), eng).unwrap();
    models
}

fn discard_noise_config(models: &std::path::Path, output_format: OutputFormat) -> ExtractionConfig {
    ExtractionConfig {
        use_cache: false,
        output_format,
        include_document_structure: true,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ocr: Some(OcrConfig {
            tessdata_path: Some(models.into()),
            quality_thresholds: Some(OcrQualityThresholds {
                discard_suspected_ocr_noise: true,
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}

#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "bundle-tessdata-eng",
    feature = "tokio-runtime"
))]

use sha2::{Digest, Sha256};
use xberg::types::{ExtractionMethod, OcrElementConfig, OcrElementLevel};
use xberg::{ExtractInput, ExtractionConfig, OcrConfig, OutputFormat, PageConfig, TesseractConfig};

#[tokio::test]
#[ignore = "requires the public Intel manual excerpt in XBERG_3105_PDF (pages 4410, 2798, 4310, 4414, 4443, 1306, 4444)"]
async fn intel_page_4414_keeps_one_769_in_plain_and_markdown() {
    verify_intel_page(None).await;
}

#[tokio::test]
#[ignore = "requires the public Intel manual excerpt in XBERG_3105_PDF"]
async fn selected_tesseract_pipeline_stage_keeps_one_769() {
    let pipeline = serde_json::from_value(serde_json::json!({
        "stages": [{"backend": "tesseract"}]
    }))
    .expect("valid explicit pipeline");
    verify_intel_page(Some(pipeline)).await;
}

async fn verify_intel_page(pipeline: Option<xberg::OcrPipelineConfig>) {
    let path = std::env::var_os("XBERG_3105_PDF").expect("set XBERG_3105_PDF to the public Intel manual excerpt");
    let bytes = std::fs::read(path).expect("read Intel manual excerpt");
    let tessdata = tempfile::tempdir().expect("create isolated tessdata directory");
    let model = xberg_tesseract::bundled_eng_traineddata().expect("bundled English model");
    assert_eq!(
        hex::encode(Sha256::digest(model)),
        "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2"
    );
    std::fs::write(tessdata.path().join("eng.traineddata"), model).expect("write bundled English model");
    let mut config = ExtractionConfig {
        use_cache: false,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ocr: Some(OcrConfig {
            enabled: true,
            backend: "tesseract".into(),
            pipeline,
            element_config: Some(OcrElementConfig {
                include_elements: true,
                min_level: OcrElementLevel::Word,
                ..Default::default()
            }),
            language: vec!["eng".into()],
            tessdata_path: Some(tessdata.path().to_path_buf()),
            tesseract_config: Some(TesseractConfig {
                use_cache: false,
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    for format in [OutputFormat::Plain, OutputFormat::Markdown] {
        config.output_format = format.clone();
        let output = xberg::extract(
            ExtractInput::from_bytes(bytes.clone(), "application/pdf", None),
            &config,
        )
        .await
        .expect("extract the Intel manual excerpt");
        assert!(output.errors.is_empty(), "{:?}", output.errors);
        let [document] = output.results.as_slice() else {
            panic!("expected one document");
        };
        assert_eq!(document.extraction_method, Some(ExtractionMethod::Mixed), "{format:?}");
        let pages = document.pages.as_ref().expect("per-page output");
        assert_eq!(pages.len(), 7, "{format:?}");
        let page = &pages[3];
        let words = document.ocr_elements.as_ref().expect("selected OCR elements");
        assert!(
            words.iter().any(|word| {
                word.page_number == page.page_number
                    && word.level == OcrElementLevel::Word
                    && word.text == "769"
                    && word.backend_metadata.get("backend").and_then(serde_json::Value::as_str)
                        == Some("tesseract-iterator")
            }),
            "{format:?}: expected the selected Tesseract stage to own 769"
        );
        assert!(page.content.contains("Monitoring Management Facility"), "{format:?}");
        for (source, transcript) in [("page", &page.content), ("document", &document.content)] {
            let count = transcript
                .split(|ch: char| !ch.is_alphanumeric())
                .filter(|token| *token == "769")
                .count();
            assert_eq!(count, 1, "{format:?} {source}");
        }
    }
}

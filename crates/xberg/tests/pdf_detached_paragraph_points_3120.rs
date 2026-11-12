#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "bundle-tessdata-eng",
    feature = "tokio-runtime"
))]

mod helpers;

use xberg::core::config::{ExtractionConfig, OcrConfig, OcrPipelineConfig, OcrPipelineStage, OutputFormat};
use xberg::types::{ExtractionMethod, document_structure::NodeContent};

#[tokio::test]
async fn detached_table_free_plain_paragraph_is_in_page_points() {
    let tessdata = tempfile::tempdir().expect("isolated tessdata directory");
    let english = xberg_tesseract::bundled_eng_traineddata().expect("bundled English model");
    std::fs::write(tessdata.path().join("eng.traineddata"), english).expect("install the bundled model");

    #[expect(deprecated, reason = "OcrPipelineStage still requires the legacy PaddleOCR field")]
    let stage = OcrPipelineStage {
        backend: "tesseract".into(),
        priority: 100,
        language: None,
        tesseract_config: None,
        paddle_ocr_config: None,
        paddle_ocr_settings: None,
        vlm_config: None,
        backend_options: None,
    };
    let config = ExtractionConfig {
        force_ocr_pages: Some(vec![1]),
        use_cache: false,
        output_format: OutputFormat::Plain,
        include_document_structure: true,
        ocr: Some(OcrConfig {
            language: vec!["eng".into()],
            tessdata_path: Some(tessdata.path().to_path_buf()),
            pipeline: Some(OcrPipelineConfig {
                stages: vec![stage],
                quality_thresholds: Default::default(),
            }),
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = helpers::extract_bytes_document(
        include_bytes!("fixtures/ocr/scanned_hello.pdf"),
        "application/pdf",
        &config,
    )
    .await
    .expect("extract the table-free scan through the detached pipeline");
    assert_eq!(result.extraction_method, Some(ExtractionMethod::Mixed));
    assert!(
        result.tables.is_empty(),
        "the fixture must exercise the table-free branch"
    );
    let paragraph = result
        .document
        .expect("requested document structure")
        .nodes
        .into_iter()
        .find(|node| matches!(&node.content, NodeContent::Paragraph { text } if !text.trim().is_empty()))
        .expect("recognized paragraph");
    assert_eq!(paragraph.page, Some(1));
    let bbox = paragraph.bbox.expect("recognized paragraph bounds");

    // ~keep The 612×792 pt MediaBox uses a bottom-left origin. A small tolerance
    // allows OCR box variation while rejecting both raster pixels and double scaling.
    let expected = [52.0, 692.0, 357.0, 718.0];
    let actual = [bbox.x0, bbox.y0, bbox.x1, bbox.y1];
    assert!(
        actual
            .iter()
            .zip(expected)
            .all(|(actual, expected)| (actual - expected).abs() <= 5.0),
        "expected paragraph near {expected:?} in PDF points, got {actual:?}"
    );
}

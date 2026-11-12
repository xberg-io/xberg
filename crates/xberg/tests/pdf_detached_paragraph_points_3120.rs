#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "bundle-tessdata-eng",
    feature = "tokio-runtime"
))]

mod helpers;

use xberg::core::config::{ExtractionConfig, OcrConfig, OcrPipelineConfig, OcrPipelineStage, OutputFormat};
use xberg::types::{ExtractedDocument, ExtractionMethod, document_structure::NodeContent};

const SCAN: &[u8] = include_bytes!("fixtures/ocr/scanned_hello.pdf");

#[tokio::test]
async fn detached_table_free_plain_paragraph_is_in_page_points() {
    let result = extract_detached(SCAN).await;
    assert_paragraph_bounds(result, [52.0, 692.0, 357.0, 718.0]);
}

#[tokio::test]
async fn resized_quarter_turn_paragraph_uses_mediabox_axes() {
    let mut pdf = lopdf::Document::load_mem(SCAN).expect("read the scan fixture");
    let page = *pdf.get_pages().values().next().expect("scan page");

    // ~keep Turn the scan counterclockwise in the landscape MediaBox. The
    // clockwise display rotation then restores the original upright view.
    let mut contents = b"q\n0 1 -1 0 792 0 cm\n".to_vec();
    contents.extend(pdf.get_page_content(page));
    contents.extend(b"\nQ\n");
    pdf.change_page_content(page, contents)
        .expect("rotate the page contents");
    let dictionary = pdf.get_dictionary_mut(page).expect("page dictionary");
    dictionary.set("MediaBox", vec![0.into(), 0.into(), 792.into(), 612.into()]);
    dictionary.set("Rotate", 90);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).expect("write the rotated scan");

    let result = extract_detached(&bytes).await;
    let resized = result
        .metadata
        .image_preprocessing
        .as_ref()
        .and_then(|metadata| metadata.new_dimensions.as_ref())
        .expect("OCR must resize the upright raster");
    assert!(resized.width < resized.height, "preprocessing records upright axes");

    // ~keep In MediaBox points (x, y) becomes (792 - y, x).
    assert_paragraph_bounds(result, [74.0, 52.0, 100.0, 357.0]);
}

async fn extract_detached(pdf: &[u8]) -> ExtractedDocument {
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

    helpers::extract_bytes_document(pdf, "application/pdf", &config)
        .await
        .expect("extract the table-free scan through the detached pipeline")
}

fn assert_paragraph_bounds(result: ExtractedDocument, expected: [f64; 4]) {
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

    // ~keep Allow OCR box variation while rejecting raster pixels and double scaling.
    let actual = [bbox.x0, bbox.y0, bbox.x1, bbox.y1];
    assert!(
        actual
            .iter()
            .zip(expected)
            .all(|(actual, expected)| (actual - expected).abs() <= 5.0),
        "expected paragraph near {expected:?} in PDF points, got {actual:?}"
    );
}

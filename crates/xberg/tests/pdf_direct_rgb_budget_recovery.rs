#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]

use xberg::{ExtractInput, ExtractedDocument, ExtractionConfig, extract};

/// The progressive label scan: its page render is refused at the 80 MiB budget below, since the
/// renderer can decode a progressive JPEG only at full size (92 857 078 bytes at 261 dpi: the RGB
/// raster and RGBA copy 64 260 000, two copies of the 398 723 encoded bytes, the 27 799 632-byte
/// canvas). The baseline `label-scan.pdf` now renders and never reaches the direct retry.
const SCAN: &[u8] = include_bytes!("../../../fixtures/repros/label-scan-over-image-budget/label-scan-progressive.pdf");
const RECOVERED: &str = "was recovered from the page's embedded image XObjects";

fn config() -> ExtractionConfig {
    serde_json::from_value(serde_json::json!({
        "use_cache": false,
        "ocr": {"backend": "tesseract", "language": ["eng"]},
        "pages": {"extract_pages": true},
        "security_limits": {"max_content_size": 80 * 1024 * 1024},
        "concurrency": {"max_threads": 1},
    }))
    .expect("automatic direct Tesseract with default recognition thresholds")
}

fn run(bytes: &[u8], config: &ExtractionConfig) -> xberg::Result<ExtractedDocument> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("integration runtime");
    let mut result = runtime.block_on(extract(
        ExtractInput::from_bytes(bytes.to_vec(), "application/pdf", None),
        config,
    ))?;
    assert!(result.errors.is_empty(), "{:?}", result.errors);
    assert_eq!(result.results.len(), 1);
    Ok(result.results.pop().expect("one document"))
}

#[test]
fn automatic_direct_rgb_budget_retry_retains_text_with_default_recognition_policy() {
    let document = run(SCAN, &config()).expect("an eligible single RGB JPEG has a bounded direct retry");
    assert!(document.content.contains("GENERAL NOTES"), "{}", document.content);
    let pages = document.pages.as_ref().expect("page records");
    assert_eq!(pages.len(), 1);
    assert!(pages[0].content.contains("GENERAL NOTES"));
    let preprocessing = pages[0]
        .image_preprocessing
        .as_ref()
        .expect("actual retry preprocessing");
    assert!(!preprocessing.dimension_clamped, "{preprocessing:?}");
    assert!(preprocessing.scale_factor >= 1.0, "{preprocessing:?}");
    assert!(preprocessing.final_dpi >= preprocessing.target_dpi, "{preprocessing:?}");
    let recovered: Vec<_> = document
        .processing_warnings
        .iter()
        .filter(|warning| warning.message.contains(RECOVERED))
        .collect();
    assert_eq!(recovered.len(), 1, "{:?}", document.processing_warnings);
    assert!(recovered[0].message.contains("its text was recovered"));
    assert!(
        !document
            .processing_warnings
            .iter()
            .any(|warning| warning.message.contains("the retry read was rejected as noise"))
    );
}

#[cfg(not(paddle_ocr))]
fn assert_original_budget_error(bytes: &[u8], config: &ExtractionConfig) {
    use std::error::Error as _;
    let error = run(bytes, config).expect_err("unsupported direct recovery must keep the typed refusal");
    let source = error.source().expect("original render failure source");
    assert!(format!("{source:?}").contains("SourceImageBudgetExceeded"), "{error:?}");
    assert!(error.to_string().contains("92857078"), "{error}");
    assert!(error.to_string().contains("83886080"), "{error}");
    assert!(!error.to_string().contains(RECOVERED), "{error}");
}

#[test]
#[cfg(not(paddle_ocr))]
fn explicitly_forced_minimal_direct_ocr_keeps_its_budget_refusal() {
    let mut config = config();
    config.force_ocr = true;
    assert_original_budget_error(SCAN, &config);
}

/// Without layout detection compiled in, OCR renders the page itself and keeps the typed refusal,
/// since the direct retry is never offered to a layout request.
#[test]
#[cfg(all(feature = "layout-types", not(feature = "layout-detection"), not(paddle_ocr)))]
fn layout_request_keeps_the_original_rgb_render_budget_refusal() {
    let mut config = config();
    config.layout = Some(Default::default());
    assert_original_budget_error(SCAN, &config);
}

/// With layout detection compiled in, the layout pass renders the page first: it records the same
/// refused render as a layout warning and hands OCR a blank page, so no typed error reaches the
/// caller. The direct retry must still not recover the page of a layout request.
#[test]
#[cfg(all(feature = "layout-detection", not(paddle_ocr)))]
fn layout_detection_records_the_rgb_render_budget_refusal_without_a_direct_retry() {
    let mut config = config();
    config.layout = Some(Default::default());
    let document = run(SCAN, &config).expect("the layout pass records the refused render as a warning");
    let messages: Vec<&str> = document
        .processing_warnings
        .iter()
        .map(|warning| warning.message.as_ref())
        .collect();
    assert!(
        messages
            .iter()
            .any(|message| message.contains("could not be prepared for layout analysis")
                && message.contains("92857078")
                && message.contains("83886080")),
        "{messages:?}"
    );
    assert!(
        !messages.iter().any(|message| message.contains(RECOVERED)),
        "{messages:?}"
    );
}

#[test]
#[cfg(not(paddle_ocr))]
fn minimal_direct_retry_does_not_omit_visible_paint_beside_the_rgb_scan() {
    let mut pdf = lopdf::Document::load_mem(SCAN).expect("valid baseline PDF");
    let page = *pdf.get_pages().get(&1).expect("one physical page");
    let overlay = pdf.add_object(lopdf::Stream::new(lopdf::Dictionary::new(), b"0 0 3 3 re f".to_vec()));
    let mut contents = pdf
        .get_page_contents(page)
        .into_iter()
        .map(lopdf::Object::Reference)
        .collect::<Vec<_>>();
    contents.push(lopdf::Object::Reference(overlay));
    pdf.get_object_mut(page)
        .unwrap()
        .as_dict_mut()
        .unwrap()
        .set("Contents", contents);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    assert_original_budget_error(&bytes, &config());
}

#[test]
fn direct_rgb_budget_retry_retains_the_producing_tesseract_segmentation_mode() {
    let mut value = serde_json::to_value(config()).unwrap();
    value["ocr"]["tesseract_config"] = serde_json::json!({"psm": 11});
    let config: ExtractionConfig = serde_json::from_value(value).unwrap();
    let document = run(SCAN, &config).expect("the explicit segmentation mode remains eligible");
    assert!(
        document.content.contains("GENERAL") && document.content.contains("NOTES"),
        "{}",
        document.content
    );
    let modes = document
        .metadata
        .additional
        .get("ocr_page_segmentation_modes")
        .expect("the page records its producing backend's actual PSM");
    assert_eq!(modes, &serde_json::json!([{"page_number": 1, "psm": 11}]));
}

#[test]
fn mixed_pipeline_retry_records_segmentation_mode_for_the_recovered_page() {
    const MIXED: &[u8] =
        include_bytes!("../../../fixtures/repros/label-scan-over-image-budget/label-scan-mixed-progressive.pdf");
    let mut value = serde_json::to_value(config()).unwrap();
    value["ocr"]["pipeline"] = serde_json::json!({
        "stages": [{"backend": "tesseract", "tesseract_config": {"psm": 11}}],
    });
    let config: ExtractionConfig = serde_json::from_value(value).unwrap();
    let document = run(MIXED, &config).expect("the mixed PDF should retain native and recovered text");
    let pages = document.pages.as_ref().expect("page records");
    assert_eq!(pages.len(), 2);
    assert!(pages[0].content.contains("native page remains unchanged"));
    assert!(pages[1].content.contains("GENERAL"), "{}", pages[1].content);
    assert!(
        document
            .processing_warnings
            .iter()
            .any(|warning| warning.message.contains(RECOVERED))
    );
    assert_eq!(
        document.metadata.additional.get("ocr_page_segmentation_modes"),
        Some(&serde_json::json!([{"page_number": 2, "psm": 11}])),
    );
}

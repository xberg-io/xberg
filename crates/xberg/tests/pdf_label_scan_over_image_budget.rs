#![cfg(all(
    feature = "ocr",
    feature = "pdf",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]

use xberg::{ExtractInput, ExtractedDocument, ExtractionConfig, extract as extract_public};

const LABEL_SCAN: &[u8] = include_bytes!("../../../fixtures/repros/label-scan-over-image-budget/label-scan.pdf");
const MIXED_LABEL_SCAN: &[u8] =
    include_bytes!("../../../fixtures/repros/label-scan-over-image-budget/label-scan-mixed.pdf");

/// The page render needs about 93 MB of image working memory and the budget below is 80 MiB, so the
/// render is refused and OCR retries on the embedded image at its proven 300 dpi. That image fits
/// the configured size limit and is not shrunk, so the read keeps its glyphs.
const REFUSING_BUDGET: usize = 80 * 1024 * 1024;
const RECOVERY_CLAIM: &str = "was recovered from the page's embedded image XObjects";

/// The labels read from this scan are mostly one or two characters, which the default threshold
/// (35%) does not catch at this page size; 0.15 makes the read trip the fragmented-word test the
/// way the 34 x 24 in sheet of the issue does at the default.
fn config(discard_suspected_ocr_noise: bool, explicit_pipeline: bool) -> ExtractionConfig {
    let mut ocr = serde_json::json!({
        "backend": "tesseract",
        "language": ["eng"],
        "quality_thresholds": {
            "max_ocr_output_fragmented_word_ratio": 0.15,
            "discard_suspected_ocr_noise": discard_suspected_ocr_noise,
        },
    });
    if explicit_pipeline {
        ocr["pipeline"] = serde_json::json!({
            "stages": [{"backend": "tesseract", "backend_options": {"use_cache": false}}],
        });
    }
    let mut config: ExtractionConfig = serde_json::from_value(serde_json::json!({
        "use_cache": false,
        "ocr": ocr,
        "pages": {"extract_pages": true},
        "concurrency": {"max_threads": 1},
    }))
    .expect("one-thread Tesseract configuration");
    config
        .security_limits
        .get_or_insert_with(Default::default)
        .max_content_size = REFUSING_BUDGET;
    config
}

fn try_extract(pdf: &[u8], config: &ExtractionConfig) -> xberg::Result<ExtractedDocument> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("integration runtime");
    let mut result = runtime.block_on(extract_public(
        ExtractInput::from_bytes(pdf.to_vec(), "application/pdf", None),
        config,
    ))?;
    assert!(result.errors.is_empty(), "{:?}", result.errors);
    Ok(result.results.pop().expect("one extracted document"))
}

fn extract(pdf: &[u8], config: &ExtractionConfig) -> ExtractedDocument {
    try_extract(pdf, config).expect("extract with the real Tesseract backend")
}

fn warnings_containing<'a>(document: &'a ExtractedDocument, part: &str) -> Vec<&'a str> {
    document
        .processing_warnings
        .iter()
        .map(|warning| warning.message.as_ref())
        .filter(|message| message.contains(part))
        .collect()
}

/// `scan_page` is the index of the scan among the document's pages.
fn assert_kept_with_warning(document: &ExtractedDocument, scan_page: usize) {
    assert!(document.content.contains("GENERAL NOTES"), "{:?}", document.content);
    assert!(
        document.pages.as_ref().expect("page record")[scan_page]
            .content
            .contains("GENERAL NOTES")
    );
    let noise = warnings_containing(document, "suspected OCR recognition noise");
    assert_eq!(noise.len(), 1, "{:?}", document.processing_warnings);
    assert!(noise[0].contains("its text was retained"), "{}", noise[0]);
    let claims = warnings_containing(document, RECOVERY_CLAIM);
    assert_eq!(claims.len(), 1, "{:?}", document.processing_warnings);
    assert!(claims[0].contains("its text was recovered"), "{}", claims[0]);
}

fn assert_read_was_not_shrunk(document: &ExtractedDocument, scan_page: usize) {
    let preprocessing = document.pages.as_ref().expect("page record")[scan_page]
        .image_preprocessing
        .as_ref()
        .expect("the retry read records its preprocessing");
    assert!(!preprocessing.dimension_clamped, "{preprocessing:?}");
    assert!(preprocessing.scale_factor >= 1.0, "{preprocessing:?}");
    assert!(preprocessing.final_dpi >= preprocessing.target_dpi, "{preprocessing:?}");
}

#[test]
fn retry_read_of_a_refused_scan_keeps_its_text_and_warns_when_noise_is_not_discarded() {
    let document = extract(LABEL_SCAN, &config(false, false));
    assert_read_was_not_shrunk(&document, 0);
    assert_kept_with_warning(&document, 0);
}

/// An explicit pipeline takes its own retry on the refused page of a document with a native page.
#[test]
fn explicit_pipeline_keeps_the_retry_read_and_warns_too() {
    let document = extract(MIXED_LABEL_SCAN, &config(false, true));
    assert_read_was_not_shrunk(&document, 1);
    assert_kept_with_warning(&document, 1);
}

/// A caller who asked for suspected noise to be discarded gets the refusal, naming the noise, for
/// a one-page document whose only read was discarded; the page is not reported as recovered.
#[test]
fn retry_read_of_a_refused_scan_is_discarded_when_the_caller_asks_and_says_why() {
    let error = try_extract(LABEL_SCAN, &config(true, false)).expect_err("the only page's read is discarded");
    let message = error.to_string();
    assert!(message.contains("image working budget"), "{message}");
    assert!(message.contains("the retry read was rejected as noise"), "{message}");
    assert!(!message.contains(RECOVERY_CLAIM), "{message}");
}

#[test]
fn explicit_pipeline_discards_the_retry_read_when_the_caller_asks_and_says_why() {
    let document = extract(MIXED_LABEL_SCAN, &config(true, true));
    let native = "The native page remains unchanged.\n".repeat(10);
    assert_eq!(
        document.content.split_whitespace().collect::<Vec<_>>(),
        native.split_whitespace().collect::<Vec<_>>()
    );
    let noise = warnings_containing(&document, "suspected OCR recognition noise");
    assert_eq!(noise.len(), 1, "{:?}", document.processing_warnings);
    assert!(noise[0].contains("its text was discarded"), "{}", noise[0]);
    assert!(
        warnings_containing(&document, RECOVERY_CLAIM).is_empty(),
        "{:?}",
        document.processing_warnings
    );
    let failures = document
        .ocr_page_failures
        .as_ref()
        .expect("the refused page is recorded");
    let [failure] = failures.as_slice() else {
        panic!("expected one page failure, got {failures:?}");
    };
    assert_eq!(failure.page, 2);
    assert!(!failure.recovered);
    assert!(
        failure.error.contains("the retry read was rejected as noise"),
        "{}",
        failure.error
    );
}

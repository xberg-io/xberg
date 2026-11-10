#![cfg(all(
    feature = "ocr",
    feature = "pdf",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]

use xberg::{ExtractInput, ExtractedDocument, ExtractionConfig, extract as extract_public};

const LABEL_SCAN: &[u8] = include_bytes!("../../../fixtures/repros/label-scan-over-image-budget/label-scan.pdf");

/// The page render needs about 93 MB of image working memory and the budget below is 80 MiB, so the
/// render is refused and OCR retries on the embedded image at its proven 300 dpi. That image fits
/// the configured size limit and is not shrunk, so the read keeps its glyphs.
const REFUSING_BUDGET: usize = 80 * 1024 * 1024;
const RECOVERY_CLAIM: &str = "was recovered from the page's embedded image XObjects";

/// The labels read from this scan are mostly one or two characters, which the default threshold
/// (35%) does not catch at this page size; 0.15 makes the read trip the fragmented-word
/// test the way the 34 x 24 in sheet of the issue does at the default.
fn config(discard_suspected_ocr_noise: bool) -> ExtractionConfig {
    let mut config: ExtractionConfig = serde_json::from_value(serde_json::json!({
        "use_cache": false,
        "ocr": {
            "backend": "tesseract",
            "language": ["eng"],
            "quality_thresholds": {
                "max_ocr_output_fragmented_word_ratio": 0.15,
                "discard_suspected_ocr_noise": discard_suspected_ocr_noise,
            },
        },
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

fn try_extract(config: &ExtractionConfig) -> xberg::Result<ExtractedDocument> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("integration runtime");
    let mut result = runtime.block_on(extract_public(
        ExtractInput::from_bytes(LABEL_SCAN.to_vec(), "application/pdf", None),
        config,
    ))?;
    assert!(result.errors.is_empty(), "{:?}", result.errors);
    Ok(result.results.pop().expect("one extracted document"))
}

fn extract(config: &ExtractionConfig) -> ExtractedDocument {
    try_extract(config).expect("extract with the real Tesseract backend")
}

fn warnings_containing<'a>(document: &'a ExtractedDocument, part: &str) -> Vec<&'a str> {
    document
        .processing_warnings
        .iter()
        .map(|warning| warning.message.as_ref())
        .filter(|message| message.contains(part))
        .collect()
}

#[test]
fn retry_read_of_a_refused_scan_keeps_its_text_and_warns_when_noise_is_not_discarded() {
    let document = extract(&config(false));
    assert!(document.content.contains("GENERAL NOTES"), "{:?}", document.content);
    assert!(
        document.pages.as_ref().expect("page record")[0]
            .content
            .contains("GENERAL NOTES")
    );
    let noise = warnings_containing(&document, "suspected OCR recognition noise");
    assert_eq!(noise.len(), 1, "{:?}", document.processing_warnings);
    assert!(noise[0].contains("its text was retained"), "{}", noise[0]);
    let claims = warnings_containing(&document, RECOVERY_CLAIM);
    assert_eq!(claims.len(), 1, "{:?}", document.processing_warnings);
    assert!(claims[0].contains("its text was recovered"), "{}", claims[0]);
}

/// A caller who asked for suspected noise to be discarded gets the refusal, naming the noise, for
/// a one-page document whose only read was discarded; the page is not reported as recovered.
#[test]
fn retry_read_of_a_refused_scan_is_discarded_when_the_caller_asks_and_says_why() {
    let error = try_extract(&config(true)).expect_err("the only page's read is discarded");
    let message = error.to_string();
    assert!(message.contains("image working budget"), "{message}");
    assert!(message.contains("the retry read was rejected as noise"), "{message}");
    assert!(!message.contains(RECOVERY_CLAIM), "{message}");
}

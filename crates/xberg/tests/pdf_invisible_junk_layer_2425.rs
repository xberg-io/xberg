//! NACA Report 1135 p25 (#2425): a junk invisible text layer sits over a full-page 300 dpi CCITT
//! scan. Automatic OCR must read the scan instead of keeping the layer.
#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "tokio-runtime",
    feature = "bundle-tessdata-eng"
))]

mod helpers;
use helpers::get_test_documents_dir;
use sha2::{Digest, Sha256};
use xberg::{ExtractInput, ExtractionConfig, OcrConfig};

const PAGE: &str = "pdf/regression_pages/nasa-naca-report-1135-p25.pdf";
const PAGE_SHA256: &str = "1696465613177c9059fea2edd8a109c753c649fdfb80cf33712d9d890f0a3ed0";
const FETCH_HINT: &str = "The corpus is fetched with: python3 test_documents/scripts/fetch_corpus.py";

fn page_bytes() -> Vec<u8> {
    let path = get_test_documents_dir().join(PAGE);
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|e| panic!("NACA 1135 p25 fixture {path:?} is missing ({e}). {FETCH_HINT}"));
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        PAGE_SHA256,
        "NACA 1135 p25 fixture {path:?} changed. {FETCH_HINT}"
    );
    bytes
}

async fn read(config: ExtractionConfig) -> xberg::ExtractedDocument {
    let config = ExtractionConfig {
        use_cache: false,
        ocr: Some(OcrConfig::default()),
        ..config
    };
    let output = xberg::extract(ExtractInput::from_bytes(page_bytes(), "application/pdf", None), &config)
        .await
        .expect("extract NACA 1135 p25");
    assert!(output.errors.is_empty(), "{:?}", output.errors);
    <[_; 1]>::try_from(output.results)
        .expect("one document")
        .into_iter()
        .next()
        .unwrap()
}

fn words(document: &xberg::ExtractedDocument) -> Vec<&str> {
    document.content.split_whitespace().collect()
}

#[tokio::test]
#[ignore = "#2425: no signal sends this covered layer to OCR; the plausibility check abstains on a numeric table"]
async fn automatic_ocr_replaces_the_junk_invisible_layer_over_the_ccitt_scan() {
    let native = read(ExtractionConfig {
        disable_ocr: true,
        ..Default::default()
    })
    .await;
    assert_eq!(words(&native).len(), 424);
    assert!(
        native.content.contains("COMMI2\"rEE"),
        "the junk layer spells the heading wrongly"
    );

    let forced = read(ExtractionConfig {
        force_ocr: true,
        ..Default::default()
    })
    .await;
    let automatic = read(ExtractionConfig::default()).await;
    assert!(automatic.metadata.ocr_used);
    assert!(words(&automatic).contains(&"COMMITTEE"));
    assert!(!automatic.content.contains("COMMI2\"rEE"));
    assert_eq!(words(&automatic).len(), words(&forced).len());
    assert!(words(&automatic).len() > 2000, "{}", words(&automatic).len());
}

#[tokio::test]
async fn a_tight_preview_budget_keeps_the_native_layer_of_the_ccitt_scan() {
    let automatic = read(ExtractionConfig {
        security_limits: Some(xberg::extractors::security::SecurityLimits {
            max_iterations: 500_000,
            ..Default::default()
        }),
        ..Default::default()
    })
    .await;
    assert!(!automatic.metadata.ocr_used);
    assert_eq!(words(&automatic).len(), 424);
}

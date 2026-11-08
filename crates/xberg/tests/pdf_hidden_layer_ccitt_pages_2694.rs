//! A multi-page scan with a junk invisible layer on every page (#2694). Each page is a 300 dpi
//! CCITT Group 4 picture costing about 30 million preview work units. `max_iterations` of 2.5
//! million buys 40 million per page, so a budget shared by the whole document recovered only the
//! first page and left the junk layer on the rest.
#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "tokio-runtime",
    feature = "bundle-tessdata-eng"
))]
use xberg::{ExtractInput, ExtractionConfig, OcrConfig};

const PER_PAGE_ENOUGH: usize = 2_500_000;
const PER_PAGE_TOO_SMALL: usize = 1_000_000;
const SCAN: &[u8] = include_bytes!("fixtures/ocr/hidden_layer_ccitt_2694/four-page-junk-layer.pdf");
const PAGE_WORDS: [&str; 4] = ["lighthouse", "Volunteers", "Fishing", "evening"];
const JUNK: &str = "47110";

async fn read(security_limits: Option<xberg::extractors::security::SecurityLimits>) -> xberg::ExtractedDocument {
    let config = ExtractionConfig {
        use_cache: false,
        ocr: Some(OcrConfig::default()),
        security_limits,
        ..Default::default()
    };
    let output = xberg::extract(
        ExtractInput::from_bytes(SCAN.to_vec(), "application/pdf", None),
        &config,
    )
    .await
    .expect("extract the four-page scan");
    assert!(output.errors.is_empty(), "{:?}", output.errors);
    output.results.into_iter().next().expect("one document")
}

#[tokio::test]
async fn every_page_of_a_multi_page_ccitt_scan_replaces_its_junk_layer() {
    let native = xberg::extract(
        ExtractInput::from_bytes(SCAN.to_vec(), "application/pdf", None),
        &ExtractionConfig {
            disable_ocr: true,
            use_cache: false,
            ..Default::default()
        },
    )
    .await
    .expect("extract native layer")
    .results
    .remove(0);
    assert_eq!(
        native.content.matches(JUNK).count(),
        12,
        "three junk rows on each of four pages"
    );

    let document = read(Some(xberg::extractors::security::SecurityLimits {
        max_iterations: PER_PAGE_ENOUGH,
        ..Default::default()
    }))
    .await;
    assert!(document.metadata.ocr_used);
    for word in PAGE_WORDS {
        assert!(
            document.content.contains(word),
            "{word} missing from {:?}",
            document.content
        );
    }
    assert!(!document.content.contains(JUNK), "{:?}", document.content);
}

#[tokio::test]
async fn a_per_page_budget_below_one_scan_keeps_every_junk_layer() {
    let document = read(Some(xberg::extractors::security::SecurityLimits {
        max_iterations: PER_PAGE_TOO_SMALL,
        ..Default::default()
    }))
    .await;
    assert_eq!(document.content.matches(JUNK).count(), 12, "{:?}", document.content);
    for word in PAGE_WORDS {
        assert!(!document.content.contains(word), "{word}");
    }
}

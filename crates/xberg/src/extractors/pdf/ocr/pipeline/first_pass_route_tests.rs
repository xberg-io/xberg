use super::accepted_single_page_first_pass;
use crate::core::config::{ExtractionConfig, OcrConfig};
use crate::extractors::security::SecurityLimits;
use crate::ocr::tesseract_backend::read_counter;

const ORDINARY_SCAN: &[u8] =
    include_bytes!("../../../../../tests/fixtures/ocr/detached_heading_frame_2497/heading-table-0.pdf");

fn config(key: usize) -> ExtractionConfig {
    let limits = SecurityLimits {
        max_archive_size: key,
        ..Default::default()
    };
    ExtractionConfig {
        use_cache: false,
        ocr: Some(OcrConfig {
            security_limits: Some(limits.clone()),
            ..Default::default()
        }),
        security_limits: Some(limits),
        ..Default::default()
    }
}

#[tokio::test]
async fn an_unbounded_one_page_scan_is_read_exactly_once() {
    let key = 512 * 1024 * 1024 + 1;
    let config = ExtractionConfig {
        force_ocr: true,
        ..config(key)
    };
    let input = crate::ExtractInput::from_bytes(ORDINARY_SCAN.to_vec(), "application/pdf", None);
    let output = crate::extract(input, &config).await.expect("extract the ordinary scan");
    assert!(output.errors.is_empty(), "{:?}", output.errors);
    assert_eq!(read_counter::reads(key), 1);
}

#[tokio::test]
async fn an_unbounded_one_page_scan_is_not_offered_the_first_pass() {
    let key = 512 * 1024 * 1024 + 2;
    let thresholds = crate::core::config::OcrQualityThresholds::default();
    let first = accepted_single_page_first_pass(ORDINARY_SCAN, &config(key), None, None, &thresholds).await;
    assert!(first.is_none());
    assert_eq!(read_counter::reads(key), 0);
}

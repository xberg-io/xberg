use super::*;

fn noise_config(pipeline: bool, whole_document: bool, discard: bool) -> ExtractionConfig {
    let mut config = if pipeline {
        per_page_config(true)
    } else {
        direct_backend_config()
    };
    let thresholds = xberg::core::config::OcrQualityThresholds {
        discard_suspected_ocr_noise: discard,
        prune_symbol_noise_lines: false,
        ..Default::default()
    };
    let ocr = config.ocr.as_mut().unwrap();
    ocr.quality_thresholds = Some(thresholds.clone());
    if let Some(pipeline) = ocr.pipeline.as_mut() {
        pipeline.quality_thresholds = thresholds;
    }
    if whole_document {
        config.force_ocr_pages = None;
        config.force_ocr = true;
    }
    config
}

fn assert_noisy_recovery(pipeline: bool, whole_document: bool, discard: bool) {
    let config = noise_config(pipeline, whole_document, discard);
    let name = if pipeline { "tesseract" } else { DIRECT_BACKEND };
    let result = extract_with(name, RenderOutcome::Fails, RasterOutcome::Noise, &config);
    let policy_message = if discard {
        "its text was discarded"
    } else {
        "its text was retained"
    };
    assert!(
        result
            .processing_warnings
            .iter()
            .any(|warning| warning.message.contains(policy_message)),
        "{:?}",
        result.processing_warnings
    );
    let claims = result
        .processing_warnings
        .iter()
        .filter(|warning| warning.message.contains("was recovered from"))
        .count();
    let failures = result.ocr_page_failures.as_ref().unwrap();
    assert_eq!(failures.len(), if whole_document { 2 } else { 1 });
    let failure = failures.iter().find(|failure| failure.page == 2).unwrap();
    assert_eq!(
        (claims, failure.recovered),
        (usize::from(!discard), !discard),
        "{:?} {failures:?}",
        result.processing_warnings
    );
    if discard {
        assert!(result.content.trim().is_empty(), "{:?}", result.content);
    } else {
        assert!(!result.content.trim().is_empty());
    }
}

#[test]
#[serial_test::serial]
fn discarded_noisy_image_retry_has_no_recovery_claim() {
    assert_noisy_recovery(false, false, true);
}

#[test]
#[serial_test::serial]
fn retained_noisy_image_retry_keeps_its_recovery_claim() {
    assert_noisy_recovery(false, false, false);
}

#[test]
#[serial_test::serial]
fn discarded_noisy_pipeline_retry_has_no_recovery_claim() {
    assert_noisy_recovery(true, false, true);
}

#[test]
#[serial_test::serial]
fn retained_noisy_pipeline_retry_keeps_its_recovery_claim() {
    assert_noisy_recovery(true, false, false);
}

#[test]
#[serial_test::serial]
fn discarded_noisy_whole_document_retry_has_no_recovery_claim() {
    assert_noisy_recovery(false, true, true);
}

#[test]
#[serial_test::serial]
fn retained_noisy_whole_document_retry_keeps_its_recovery_claim() {
    assert_noisy_recovery(false, true, false);
}

use lopdf::dictionary;

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
    let result = if pipeline && whole_document {
        extract_from_pdf(
            name,
            RenderOutcome::Fails,
            RasterOutcome::Noise,
            &config,
            &pdf_with_native_text_and_image(),
        )
    } else {
        extract_with(name, RenderOutcome::Fails, RasterOutcome::Noise, &config)
    };
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
    if whole_document {
        assert_eq!((failures[0].page, failures[0].recovered), (1, false));
    }
    let failure = failures.iter().find(|failure| failure.page == 2).unwrap();
    assert_eq!(
        (claims, failure.recovered),
        (usize::from(!discard), !discard),
        "{:?} {failures:?}",
        result.processing_warnings
    );
    assert_eq!(result.tables.len(), usize::from(!discard));
    if discard {
        // ~keep Forced OCR restores the native page when its OCR read was discarded (#3013).
        let expected = if pipeline && whole_document {
            "The original native words stay."
        } else {
            ""
        };
        assert_eq!(result.content.trim(), expected);
        assert_backend_failure_warning(&result);
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

fn assert_backend_failure_warning(result: &ExtractedDocument) {
    assert!(
        result.processing_warnings.iter().any(|warning| {
            warning.message.contains("OCR of page 2 failed") && warning.message.contains("stub render failure")
        }),
        "{:?}",
        result.processing_warnings
    );
}

fn pdf_with_native_text_and_image() -> Vec<u8> {
    let mut doc = lopdf::Document::load_mem(&pdf_with_image_on_page_two()).unwrap();
    let page = doc.get_pages()[&2];
    let font = doc.add_object(lopdf::dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
    });
    let page = doc.get_object_mut(page).unwrap().as_dict_mut().unwrap();
    let contents = page.get(b"Contents").unwrap().as_reference().unwrap();
    page.get_mut(b"Resources")
        .unwrap()
        .as_dict_mut()
        .unwrap()
        .set("Font", lopdf::dictionary! { "F1" => font });
    let stream = doc.get_object_mut(contents).unwrap().as_stream_mut().unwrap();
    let mut content = std::mem::take(&mut stream.content);
    content.extend_from_slice(
        b"
BT /F1 12 Tf 10 180 Td (The original native words stay.) Tj ET
",
    );
    stream.set_content(content);
    let mut pdf = Vec::new();
    doc.save_to(&mut pdf).unwrap();
    pdf
}

#[test]
#[serial_test::serial]
fn discarded_noisy_retry_reports_native_text_kept() {
    assert_discarded_retry_keeps_native(false);
}

#[test]
#[serial_test::serial]
fn discarded_noisy_pipeline_retry_reports_native_text_kept() {
    assert_discarded_retry_keeps_native(true);
}

#[test]
#[serial_test::serial]
fn discarded_noisy_all_pages_pipeline_retry_has_no_recovery_claim() {
    assert_noisy_recovery(true, true, true);
}

fn assert_discarded_retry_keeps_native(pipeline: bool) {
    let result = extract_from_pdf(
        if pipeline { "tesseract" } else { DIRECT_BACKEND },
        RenderOutcome::Fails,
        RasterOutcome::Noise,
        &noise_config(pipeline, false, true),
        &pdf_with_native_text_and_image(),
    );
    assert_eq!(result.content.trim(), "The original native words stay.");
    assert!(result.tables.is_empty());
    let failures = result.ocr_page_failures.as_ref().unwrap();
    assert_eq!(failures.as_slice(), recovered_page_two().as_slice());
    assert_backend_failure_warning(&result);
    assert!(
        result
            .processing_warnings
            .iter()
            .any(|warning| { warning.message.contains("the page's native text was kept") }),
        "{:?}",
        result.processing_warnings
    );
    assert!(
        result
            .processing_warnings
            .iter()
            .any(|warning| warning.message.contains("its text was discarded"))
    );
    assert!(
        result
            .processing_warnings
            .iter()
            .all(|warning| !warning.message.contains("was recovered from"))
    );
}

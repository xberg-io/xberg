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
        assert!(result.content.trim().is_empty());
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
fn discarded_noisy_all_pages_pipeline_retry_reports_native_text_kept() {
    let result = extract_from_pdf(
        "tesseract",
        RenderOutcome::Fails,
        RasterOutcome::Noise,
        &noise_config(true, true, true),
        &pdf_with_native_text_and_image(),
    );

    assert_eq!(result.content.trim(), "The original native words stay.");
    assert!(result.tables.is_empty());
    let failures = result.ocr_page_failures.as_ref().unwrap();
    assert_eq!(failures.len(), 2);
    assert_eq!((failures[0].page, failures[0].recovered), (1, false));
    assert_eq!(&failures[1..], recovered_page_two().as_slice());
    let page_failure_warnings: Vec<_> = result
        .processing_warnings
        .iter()
        .filter(|warning| warning.source == "ocr" && warning.message.starts_with("OCR of page 2 failed"))
        .map(|warning| warning.message.as_ref())
        .collect();
    assert_eq!(
        page_failure_warnings,
        [
            "OCR of page 2 failed (OCR error: stub render failure); its embedded-image retry was discarded \
          and the page's native text was kept."
        ]
    );
    assert!(
        result
            .processing_warnings
            .iter()
            .any(|warning| warning.message.contains("its text was discarded"))
    );
}

#[test]
#[serial_test::serial]
fn all_pages_ocr_failure_without_an_embedded_image_reports_native_text_kept() {
    let result = extract_from_pdf(
        "tesseract",
        RenderOutcome::Fails,
        RasterOutcome::Word,
        &noise_config(true, true, true),
        &pdf_with_text_cover_and_two_scans(5),
    );

    assert!(result.content.contains("word0 word1 word2 word3 word4"));
    assert!(result.content.contains(RECOVERED_WORD));
    let failures = result.ocr_page_failures.as_ref().unwrap();
    assert_eq!(
        failures.iter().find(|failure| failure.page == 1),
        Some(&OcrPageFailure {
            page: 1,
            error: "OCR error: stub render failure".to_string(),
            recovered: true,
        })
    );
    let warnings: Vec<_> = result
        .processing_warnings
        .iter()
        .filter(|warning| warning.source == "ocr" && warning.message.starts_with("OCR of page 1 failed"))
        .map(|warning| warning.message.as_ref())
        .collect();
    assert_eq!(
        warnings,
        ["OCR of page 1 failed (OCR error: stub render failure); the page's native text was kept."]
    );
}

fn pdf_with_landscape_page_and_native_image_page() -> Vec<u8> {
    let mut doc = lopdf::Document::load_mem(&pdf_with_native_text_and_image()).unwrap();
    let page_one = doc.get_pages()[&1];
    let contents = doc.add_object(lopdf::Stream::new(dictionary! {}, b"0 0 200 200 re f".to_vec()));
    let page = doc.get_object_mut(page_one).unwrap().as_dict_mut().unwrap();
    page.set("Contents", contents);
    page.set("MediaBox", vec![0.into(), 0.into(), 400.into(), 200.into()]);
    let mut pdf = Vec::new();
    doc.save_to(&mut pdf).unwrap();
    pdf
}

#[test]
#[serial_test::serial]
fn all_pages_ocr_failure_with_empty_image_retry_reports_native_text_kept() {
    let result = extract_from_pdf(
        "tesseract",
        RenderOutcome::WordOnLandscape,
        RasterOutcome::Blank,
        &noise_config(true, true, true),
        &pdf_with_landscape_page_and_native_image_page(),
    );

    assert!(result.content.contains(RECOVERED_WORD));
    assert!(result.content.contains("The original native words stay."));
    assert_eq!(result.ocr_page_failures, Some(recovered_page_two()));
    let warnings: Vec<_> = result
        .processing_warnings
        .iter()
        .filter(|warning| warning.source == "ocr" && warning.message.starts_with("OCR of page 2 failed"))
        .map(|warning| warning.message.as_ref())
        .collect();
    assert_eq!(
        warnings,
        [
            "OCR of page 2 failed (OCR error: stub render failure); the retry on the page's embedded image \
          XObjects also returned no text, and the page's native text was kept."
        ]
    );
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

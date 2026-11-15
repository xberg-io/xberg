use super::*;

fn ocr_error(message: &str) -> crate::XbergError {
    crate::XbergError::Ocr {
        message: message.to_string(),
        source: None,
    }
}

fn unread_by_limit() -> crate::XbergError {
    crate::XbergError::Timeout {
        elapsed_ms: 0,
        limit_ms: 0,
    }
}

#[test]
fn a_run_with_pages_left_unread_by_the_time_limit_fails_with_the_time_limit() {
    let mut errors = ahash::AHashMap::new();
    errors.insert(0, ocr_error("the first page failed"));
    errors.insert(1, unread_by_limit());

    let error = run_error_for_failed_pages(&[0, 1], &mut errors);

    assert!(matches!(error, crate::XbergError::Timeout { .. }), "{error}");
}

#[test]
fn a_run_without_unread_pages_fails_with_its_first_failed_page() {
    let mut errors = ahash::AHashMap::new();
    errors.insert(0, ocr_error("page one failed"));
    errors.insert(1, ocr_error("page two failed"));

    let error = run_error_for_failed_pages(&[0, 1], &mut errors);

    assert!(error.to_string().contains("page one failed"), "{error}");
}

#[test]
fn discarded_retry_warning_claims_native_recovery_only_when_ocr_text_is_blank() {
    let native_text = "Readable native text";
    let boundaries = [crate::types::PageBoundary {
        page_number: 1,
        byte_start: 0,
        byte_end: native_text.len(),
    }];
    for (page_text, expected) in [
        (
            "Retained OCR",
            "OCR of page 1 failed (render failed); its embedded-image retry was discarded.",
        ),
        (
            " \t",
            "OCR of page 1 failed (render failed); its embedded-image retry was discarded \
             and the page's native text was kept.",
        ),
    ] {
        let failure = crate::types::OcrPageFailure {
            page: 1,
            error: "render failed".into(),
            recovered: false,
        };
        let discarded = discarded_xobject_retry_warning(&failure, false);
        let mut returned = crate::types::internal::InternalDocument {
            ocr_page_failures: vec![failure],
            processing_warnings: vec![discarded],
            ..Default::default()
        };
        let mut failures = Vec::new();

        merge_returned_page_failures(&mut failures, &mut returned, page_text, native_text, &boundaries);

        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].page, 1);
        assert!(failures[0].recovered);
        assert_eq!(returned.processing_warnings.len(), 1);
        assert_eq!(returned.processing_warnings[0].message, expected);
    }
}

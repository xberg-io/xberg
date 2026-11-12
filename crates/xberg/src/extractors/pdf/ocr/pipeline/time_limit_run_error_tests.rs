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

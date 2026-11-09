use super::{ocr::OcrGateOutcome, with_native_page_acceptance};
use crate::types::PageBoundary;

#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
#[test]
#[serial_test::serial]
fn vlm_fallback_keeps_document_scope_even_with_the_default_backend() {
    let mut config = crate::ExtractionConfig {
        ocr: Some(crate::OcrConfig {
            enabled: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    assert!(super::automatic_page_acceptance_supported(&config));
    for policy in [
        crate::core::config::VlmFallbackPolicy::Always,
        crate::core::config::VlmFallbackPolicy::OnLowQuality { quality_threshold: 0.6 },
    ] {
        config.ocr.as_mut().unwrap().vlm_fallback = policy;
        assert!(!super::automatic_page_acceptance_supported(&config));
    }
}

fn boundary(page_number: u32, byte_start: usize, byte_end: usize) -> PageBoundary {
    PageBoundary {
        page_number,
        byte_start,
        byte_end,
    }
}

#[test]
fn every_selected_page_keeps_the_existing_page_acceptance_route() {
    let bounds = [boundary(1, 0, 3), boundary(2, 5, 8)];
    assert_eq!(
        with_native_page_acceptance(
            OcrGateOutcome::RunFallback,
            &[1, 2],
            Some(&bounds),
            "one\n\ntwo",
            Some(2)
        ),
        OcrGateOutcome::RunFallbackOnPages(vec![1, 2])
    );
}

#[test]
fn one_page_with_utf8_text_can_keep_native_text_after_ocr() {
    let bounds = [boundary(1, 1, 3)];
    assert_eq!(
        with_native_page_acceptance(OcrGateOutcome::RunFallback, &[1], Some(&bounds), " é\n", Some(1)),
        OcrGateOutcome::RunFallbackOnPages(vec![1])
    );
}

#[test]
fn aggregate_or_incomplete_selection_keeps_whole_document_fallback() {
    let bounds = [boundary(1, 0, 3), boundary(2, 4, 7)];
    for (pages, count) in [(&[][..], Some(2)), (&[1][..], Some(2)), (&[1, 2][..], None)] {
        assert_eq!(
            with_native_page_acceptance(OcrGateOutcome::RunFallback, pages, Some(&bounds), "one two", count),
            OcrGateOutcome::RunFallback
        );
    }
    for bounds in [None, Some(&[][..])] {
        assert_eq!(
            with_native_page_acceptance(OcrGateOutcome::RunFallback, &[1], bounds, "one", Some(1)),
            OcrGateOutcome::RunFallback
        );
    }
}

#[test]
fn no_native_text_keeps_whole_document_failure_handling() {
    assert_eq!(
        with_native_page_acceptance(
            OcrGateOutcome::RunFallback,
            &[1],
            Some(&[boundary(1, 0, 0)]),
            "",
            Some(1)
        ),
        OcrGateOutcome::RunFallback
    );
}

#[test]
fn invalid_or_incomplete_boundaries_keep_whole_document_fallback() {
    let cases = [
        [boundary(1, 0, 4), boundary(2, 3, 7)],
        [boundary(1, 0, 3), boundary(2, 4, 8)],
        [boundary(1, 3, 0), boundary(2, 4, 7)],
        [boundary(2, 0, 3), boundary(1, 4, 7)],
        [boundary(1, 1, 3), boundary(2, 4, 7)],
        [boundary(1, 0, 3), boundary(2, 5, 7)],
        [boundary(1, 0, 3), boundary(2, 4, 6)],
    ];
    for bounds in cases {
        assert_eq!(
            with_native_page_acceptance(OcrGateOutcome::RunFallback, &[1, 2], Some(&bounds), "one two", Some(2)),
            OcrGateOutcome::RunFallback,
            "{bounds:?}"
        );
    }
    assert_eq!(
        with_native_page_acceptance(
            OcrGateOutcome::RunFallback,
            &[1],
            Some(&[boundary(1, 0, 1)]),
            "é",
            Some(1)
        ),
        OcrGateOutcome::RunFallback
    );
}

#[test]
fn existing_skips_and_partial_page_selection_are_unchanged() {
    for (outcome, expected) in [
        (OcrGateOutcome::SkipNonText, OcrGateOutcome::SkipNonText),
        (OcrGateOutcome::SkipSubstantive, OcrGateOutcome::SkipSubstantive),
        (OcrGateOutcome::UseNative, OcrGateOutcome::UseNative),
        (
            OcrGateOutcome::RunFallbackOnPages(vec![1]),
            OcrGateOutcome::RunFallbackOnPages(vec![1]),
        ),
    ] {
        let actual = with_native_page_acceptance(outcome, &[1], Some(&[boundary(1, 0, 3)]), "one", Some(1));
        assert_eq!(actual, expected);
    }
}

#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
#[test]
#[serial_test::serial]
fn automatic_page_acceptance_excludes_every_explicit_ocr_request() {
    let baseline = serde_json::json!({"ocr": {"enabled": true}});
    let mut config: crate::ExtractionConfig = serde_json::from_value(baseline.clone()).unwrap();
    assert!(super::automatic_page_acceptance_supported(&config));
    config.force_ocr_pages = Some(Vec::new());
    config.ocr.as_mut().unwrap().tesseract_config = Some(crate::TesseractConfig::default());
    assert!(
        config
            .ocr
            .as_ref()
            .unwrap()
            .tesseract_config
            .as_ref()
            .unwrap()
            .enable_table_detection
    );
    assert!(super::automatic_page_acceptance_supported(&config));
    let mut cases = vec![
        serde_json::json!({"force_ocr": true}),
        serde_json::json!({"include_document_structure": true}),
        serde_json::json!({"result_format": "element_based"}),
        serde_json::json!({"force_ocr_pages": [1]}),
        serde_json::json!({"force_ocr_pages": [2]}),
        serde_json::json!({"ocr": {"auto_rotate": true}}),
        serde_json::json!({"ocr": {"element_config": {"include_elements": true}}}),
        serde_json::json!({"images": {"include_page_rasters": true}}),
        serde_json::json!({"ocr": {"backend": "custom"}}),
        serde_json::json!({"ocr": {"enabled": false}}),
        serde_json::json!({"ocr": {"pipeline": {"stages": [{"backend": "tesseract"}]}}}),
    ];
    #[cfg(feature = "layout-types")]
    cases.push(serde_json::json!({"layout": {}}));
    for patch in cases.drain(..) {
        let mut value = baseline.clone();
        value
            .as_object_mut()
            .unwrap()
            .extend(patch.as_object().unwrap().clone());
        let config = serde_json::from_value(value).expect("valid explicit extraction options");
        assert!(!super::automatic_page_acceptance_supported(&config), "{patch}");
    }
}

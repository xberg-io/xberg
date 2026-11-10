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
            Some(2),
            &[]
        ),
        OcrGateOutcome::RunFallbackOnPages(vec![1, 2])
    );
}

#[test]
fn one_page_with_utf8_text_can_keep_native_text_after_ocr() {
    let bounds = [boundary(1, 1, 3)];
    assert_eq!(
        with_native_page_acceptance(OcrGateOutcome::RunFallback, &[1], Some(&bounds), " é\n", Some(1), &[]),
        OcrGateOutcome::RunFallbackOnPages(vec![1])
    );
}

#[test]
fn aggregate_or_incomplete_selection_keeps_whole_document_fallback() {
    let bounds = [boundary(1, 0, 3), boundary(2, 4, 7)];
    for (pages, count) in [(&[][..], Some(2)), (&[1][..], Some(2)), (&[1, 2][..], None)] {
        assert_eq!(
            with_native_page_acceptance(OcrGateOutcome::RunFallback, pages, Some(&bounds), "one two", count, &[]),
            OcrGateOutcome::RunFallback
        );
    }
    for bounds in [None, Some(&[][..])] {
        assert_eq!(
            with_native_page_acceptance(OcrGateOutcome::RunFallback, &[1], bounds, "one", Some(1), &[]),
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
            Some(1),
            &[]
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
            with_native_page_acceptance(
                OcrGateOutcome::RunFallback,
                &[1, 2],
                Some(&bounds),
                "one two",
                Some(2),
                &[]
            ),
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
            Some(1),
            &[]
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
        let actual = with_native_page_acceptance(outcome, &[1], Some(&[boundary(1, 0, 3)]), "one", Some(1), &[]);
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

#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
#[test]
#[serial_test::serial]
fn automatic_page_acceptance_excludes_non_plain_renderers() {
    for format in ["markdown", "html", "djot", "json", "doctags", "custom-renderer"] {
        let config = serde_json::from_value(serde_json::json!({
            "ocr": {"enabled": true}, "output_format": format,
        }))
        .unwrap();
        assert!(!super::automatic_page_acceptance_supported(&config), "{format}");
    }
}

#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
#[test]
#[serial_test::serial]
fn automatic_page_acceptance_requires_page_uniform_native_preservation() {
    for overrides in [None, Some(Vec::new()), Some(vec![2]), Some(vec![37])] {
        let config = crate::ExtractionConfig {
            force_ocr_pages: overrides.clone(),
            ocr: Some(crate::OcrConfig {
                enabled: true,
                ..Default::default()
            }),
            ..Default::default()
        };
        let uniform = overrides.as_ref().is_none_or(Vec::is_empty);
        assert_eq!(super::automatic_page_acceptance_supported(&config), uniform);
        if uniform {
            for page in [1, 2, 37, u32::MAX] {
                assert!(super::ocr::native_preservation_is_automatic(&config, page), "{page}");
            }
        }
    }
}

#[test]
fn native_khmer_number_candidates_keep_the_full_document_route() {
    for digit in '\u{17e0}'..='\u{17e9}' {
        let native = format!("native {digit} text");
        let bounds = [boundary(1, 0, native.len())];
        assert_eq!(
            with_native_page_acceptance(OcrGateOutcome::RunFallback, &[1], Some(&bounds), &native, Some(1), &[]),
            OcrGateOutcome::RunFallback,
            "{digit}"
        );
    }
    for text in ["native 0123456789 text", "native \u{17df} \u{17ea} text"] {
        let bounds = [boundary(1, 0, text.len())];
        assert_eq!(
            with_native_page_acceptance(OcrGateOutcome::RunFallback, &[1], Some(&bounds), text, Some(1), &[]),
            OcrGateOutcome::RunFallbackOnPages(vec![1]),
            "{text}"
        );
    }
}

fn recognized_page(page_number: u32) -> crate::types::internal::InternalDocument {
    use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
    let mut page = InternalDocument::new("pdf");
    let mut element = InternalElement::text(
        ElementKind::OcrText {
            level: crate::types::OcrElementLevel::Line,
        },
        "accepted OCR text",
        0,
    );
    element.page = Some(page_number);
    element.ocr_geometry = Some(crate::types::OcrBoundingGeometry::Rectangle {
        left: 12,
        top: 34,
        width: 120,
        height: 20,
    });
    element.ocr_confidence = Some(crate::types::OcrConfidence {
        detection: Some(0.95),
        recognition: 0.93,
    });
    page.push_element(element);
    page.ocr_coordinate_frame = Some(crate::types::internal::OcrPageCoordinateFrame::new(
        page_number,
        640,
        480,
    ));
    page.metadata.additional.insert(
        crate::ocr_metadata_keys::OCR_PAGE_SEGMENTATION_MODES_METADATA_KEY.into(),
        serde_json::json!([{"page_number": page_number, "psm": 6}]),
    );
    page
}

#[test]
fn plain_page_acceptance_keeps_only_accepted_segmentation_modes_in_page_order() {
    let pages = ahash::AHashMap::from([
        (3, recognized_page(3)),
        (2, recognized_page(2)),
        (1, recognized_page(1)),
    ]);
    let replacements = ahash::AHashMap::from([(2, "second".into()), (1, "first".into())]);
    let bounds = [boundary(1, 0, 5), boundary(2, 7, 13)];
    let (document, _, _) = super::select_pdf_document(
        super::ExtractionMethod::Mixed,
        "first\n\nsecond",
        "application/pdf",
        None,
        None,
        Some(&replacements),
        Some(&pages),
        Some(&bounds),
        &crate::OutputFormat::Plain,
    );
    assert_eq!(crate::rendering::render_plain(&document).trim(), "first\n\nsecond");
    assert_eq!(
        document
            .metadata
            .additional
            .get(crate::ocr_metadata_keys::OCR_PAGE_SEGMENTATION_MODES_METADATA_KEY),
        Some(&serde_json::json!([{"page_number": 1, "psm": 6}, {"page_number": 2, "psm": 6}]))
    );
}

#[test]
fn accepted_ocr_text_without_a_prebuilt_sidecar_keeps_its_derived_spatial_element() {
    let page = recognized_page(2);
    let control = crate::extraction::derive::derive_extraction_result(page.clone(), false, crate::OutputFormat::Plain);
    assert_eq!(control.ocr_elements.as_ref().unwrap().len(), 1);
    let pages = ahash::AHashMap::from([(2, page)]);
    let replacements = ahash::AHashMap::from([(2, "accepted OCR text".into())]);
    let (mut document, _, _) = super::select_pdf_document(
        super::ExtractionMethod::Mixed,
        "accepted OCR text",
        "application/pdf",
        None,
        None,
        Some(&replacements),
        Some(&pages),
        None,
        &crate::OutputFormat::Plain,
    );
    document.prebuilt_ocr_elements = Some(super::accepted_mixed_ocr_elements(&pages));
    let actual = crate::extraction::derive::derive_extraction_result(document, false, crate::OutputFormat::Plain);
    assert_eq!(actual.content, control.content);
    assert_eq!(
        serde_json::to_value(&actual.ocr_elements).unwrap(),
        serde_json::to_value(&control.ocr_elements).unwrap()
    );
    let frames = super::accepted_mixed_ocr_coordinate_frames(&pages);
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].page_number, 2);
    assert_eq!((frames[0].width, frames[0].height), (640, 480));
}

#[test]
fn an_explicit_empty_ocr_sidecar_does_not_reconstruct_filtered_elements() {
    let mut page = recognized_page(1);
    page.prebuilt_ocr_elements = Some(Vec::new());
    let pages = ahash::AHashMap::from([(1, page)]);
    assert!(super::accepted_mixed_ocr_elements(&pages).is_empty());
    assert!(super::accepted_mixed_ocr_coordinate_frames(&pages).is_empty());
}

#[test]
fn mixed_rejected_page_keeps_native_tables_and_excludes_its_ocr_payload() {
    let bounds = [boundary(1, 0, 11), boundary(2, 13, 21)];
    let mut native = super::flat_pdf_document("native text\n\nold text", "application/pdf", Some(&bounds));
    native.tables.push(crate::types::Table {
        markdown: "| native table |".into(),
        page_number: 1,
        ..Default::default()
    });
    let mut rejected = recognized_page(1);
    rejected.elements[0].text = "rejected OCR page one".into();
    rejected.prebuilt_ocr_elements = Some(vec![crate::types::OcrElement {
        text: "rejected OCR sidecar".into(),
        page_number: 1,
        ..Default::default()
    }]);
    let mut accepted = recognized_page(2);
    accepted.metadata.additional.clear();
    let pages = ahash::AHashMap::from([(1, rejected), (2, accepted)]);
    let replacements = ahash::AHashMap::from([(2, "accepted OCR text".into())]);
    let (document, origin, _) = super::select_pdf_document(
        super::ExtractionMethod::Mixed,
        "native text\n\naccepted OCR text",
        "application/pdf",
        Some(native),
        None,
        Some(&replacements),
        Some(&pages),
        Some(&bounds),
        &crate::OutputFormat::Plain,
    );
    assert_eq!(origin, super::PdfDocumentOrigin::Mixed);
    assert_eq!(document.tables[0].markdown, "| native table |");
    assert!(
        !document
            .metadata
            .additional
            .contains_key(crate::ocr_metadata_keys::OCR_PAGE_SEGMENTATION_MODES_METADATA_KEY)
    );
    let actual = crate::extraction::derive::derive_extraction_result(document, false, crate::OutputFormat::Plain);
    let elements = actual
        .ocr_elements
        .as_ref()
        .expect("the accepted page carries OCR text");
    assert_eq!(elements.len(), 1);
    assert_eq!(elements[0].page_number, 2);
    assert_eq!(elements[0].text, "accepted OCR text");
    assert!(actual.content.contains("native text"));
    assert!(!actual.content.contains("rejected OCR"));
}

#[test]
fn non_plain_partial_mixed_output_keeps_fallback_elements_and_frames() {
    let bounds = [boundary(1, 0, 11), boundary(2, 13, 21)];
    let native = super::flat_pdf_document("native text\n\nold text", "application/pdf", Some(&bounds));
    let pages = ahash::AHashMap::from([(2, recognized_page(2))]);
    let replacements = ahash::AHashMap::from([(2, "accepted OCR text".into())]);
    let (document, origin, _) = super::select_pdf_document(
        super::ExtractionMethod::Mixed,
        "native text\n\naccepted OCR text",
        "application/pdf",
        Some(native),
        None,
        Some(&replacements),
        Some(&pages),
        Some(&bounds),
        &crate::OutputFormat::Markdown,
    );
    assert_eq!(origin, super::PdfDocumentOrigin::Mixed);
    let actual = crate::extraction::derive::derive_extraction_result(document, false, crate::OutputFormat::Markdown);
    assert!(actual.content.contains("native text"));
    let expected = super::accepted_mixed_ocr_elements(&pages);
    assert_eq!(expected.len(), 1);
    assert_eq!(expected[0].page_number, 2);
    assert_eq!(
        serde_json::to_value(&actual.ocr_elements).unwrap(),
        serde_json::to_value(Some(&expected)).unwrap()
    );
    let frames = super::accepted_mixed_ocr_coordinate_frames(&pages);
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].page_number, 2);
    assert_eq!((frames[0].width, frames[0].height), (640, 480));
}

#[test]
fn fabricated_native_mappings_keep_the_whole_document_outcome() {
    let bounds = [boundary(1, 0, 3), boundary(2, 5, 8)];
    for fabricated in [&[1][..], &[2][..], &[37][..]] {
        assert_eq!(
            with_native_page_acceptance(
                OcrGateOutcome::RunFallback,
                &[1, 2],
                Some(&bounds),
                "one\n\ntwo",
                Some(2),
                fabricated
            ),
            OcrGateOutcome::RunFallback,
            "fabricated pages {fabricated:?} must retain the original document route",
        );
    }
    assert_eq!(
        with_native_page_acceptance(
            OcrGateOutcome::RunFallback,
            &[1, 2],
            Some(&bounds),
            "one\n\ntwo",
            Some(2),
            &[]
        ),
        OcrGateOutcome::RunFallbackOnPages(vec![1, 2]),
    );
}

#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
#[test]
#[serial_test::serial]
fn a_registered_tesseract_replacement_does_not_gain_builtin_page_acceptance() {
    // Registry readers do not all share the serial-test lock; replace the backend only in a child. ~keep
    const CHILD_ENV: &str = "XBERG_PDF_BACKEND_IDENTITY_CHILD";
    const TEST: &str = concat!(
        "extractors::pdf::automatic_fallback_tests::",
        "a_registered_tesseract_replacement_does_not_gain_builtin_page_acceptance",
    );
    if std::env::var(CHILD_ENV).as_deref() == Ok(TEST) {
        let args: Vec<_> = std::env::args().collect();
        assert!(args.iter().any(|arg| arg == "--exact"));
        assert!(args.iter().any(|arg| arg == "--test-threads=1"));
        assert_registered_backend_identity();
        return;
    }
    let output = std::process::Command::new(std::env::current_exe().expect("native test executable"))
        .args(["--exact", TEST, "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, TEST)
        .output()
        .expect("run the backend replacement in its own process");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success()
            && stdout.lines().any(|line| line == "running 1 test")
            && stdout.contains("test result: ok. 1 passed; 0 failed; 0 ignored;"),
        "isolated backend identity control failed: {}\n{stdout}\n{stderr}",
        output.status,
    );
}

#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
fn assert_registered_backend_identity() {
    struct RestoreBackend(std::sync::Arc<dyn crate::plugins::OcrBackend>);
    impl Drop for RestoreBackend {
        fn drop(&mut self) {
            crate::plugins::registry::get_ocr_backend_registry()
                .write()
                .register(self.0.clone())
                .expect("restore the original built-in backend");
        }
    }
    let config = crate::ExtractionConfig {
        ocr: Some(crate::OcrConfig::default()),
        ..Default::default()
    };
    assert!(super::automatic_page_acceptance_supported(&config));
    let registry = crate::plugins::registry::get_ocr_backend_registry();
    let original = RestoreBackend(registry.read().get("tesseract").unwrap());
    registry
        .write()
        .register(std::sync::Arc::new(
            crate::ocr::tesseract_backend::TesseractBackend::new(),
        ))
        .unwrap();
    assert!(super::ocr::native_preservation_is_automatic(&config, 1));
    assert!(!super::automatic_page_acceptance_supported(&config));
    drop(original);
    assert!(super::automatic_page_acceptance_supported(&config));
}

#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
#[test]
#[serial_test::serial]
fn automatic_page_acceptance_handles_all_auto_rotate_states() {
    for auto_rotate in [None, Some(false), Some(true)] {
        let config = crate::ExtractionConfig {
            ocr: Some(crate::OcrConfig {
                auto_rotate,
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(
            super::automatic_page_acceptance_supported(&config),
            auto_rotate != Some(true)
        );
    }
}

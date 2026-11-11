use super::{ExtractionConfig, FailedOcrRun, OcrRunResult, WholeDirectOcrOutput};
use crate::core::config::OcrConfig;
use crate::extractors::security::SecurityBudget;
use xberg_native_pdf::PdfDocument;

pub(super) async fn try_recovery(
    content: Option<&[u8]>,
    config: &ExtractionConfig,
    path: Option<&std::path::Path>,
    failed: &FailedOcrRun,
) -> OcrRunResult<Option<WholeDirectOcrOutput>> {
    let ocr = super::super::resolve_ocr_config(config);
    if super::budget_retry_backend(config, &ocr, failed).is_none() {
        return Ok(None);
    }
    let Some(content) = content else { return Ok(None) };
    let Ok((document, 1, _)) = super::open_pdf_for_full_ocr(content) else {
        return Ok(None);
    };
    let recovery_ocr = super::pipeline_xobject_recovery_ocr_config(config, &ocr);
    if !certified_rgb_page(&document, config, &recovery_ocr)? {
        return Ok(None);
    }
    // The coordinator opens its own PDF; release the admission copy before OCR. ~keep
    drop(document);
    let mut retry_config = config.clone();
    retry_config.ocr = Some(recovery_ocr);
    let (text, tables, elements, document, usage, pages, rasters, formulas, preprocessing, confidence) =
        super::extract_pages_with_per_page_pipeline(
            content,
            &retry_config,
            path,
            None,
            #[cfg(feature = "layout-detection")]
            None,
        )
        .await?;
    let score = confidence.get(&1).and_then(|summary| summary.score);
    Ok(Some((
        text,
        score,
        tables,
        elements,
        document,
        usage,
        pages,
        rasters,
        formulas,
        preprocessing,
        confidence,
    )))
}

fn certified_rgb_page(document: &PdfDocument, config: &ExtractionConfig, ocr: &OcrConfig) -> crate::Result<bool> {
    super::super::super::ensure_pdf_not_cancelled(config)?;
    let limits = ocr.security_limits.clone().unwrap_or_default();
    let Ok(handles) = document.page_image_handles(0) else {
        return Ok(false);
    };
    let [image] = handles.as_slice() else { return Ok(false) };
    if !matches!(
        image.color_space,
        xberg_native_pdf::extractors::images::ColorSpace::DeviceRGB
    ) {
        return Ok(false);
    }
    let Some((_, bytes)) = image.plain_stored_jpeg_for_ocr_recovery() else {
        return Ok(false);
    };
    if bytes.len() > limits.max_content_size {
        return Ok(false);
    }
    let placements = crate::pdf::native::proven_dpi::proven_source_dpis(document, 0);
    if placements
        .get(&image.paint_order)
        .and_then(|placement| placement.confirmed_by("jpeg", &bytes, &limits))
        .is_none()
    {
        return Ok(false);
    }
    let mut budget = SecurityBudget::from_config(config);
    let mut context = super::XObjectRecoveryContext::with_budget(&mut budget, config.cancel_token.as_ref());
    super::super::rendering::can_project_stored_jpeg_page(document, 0, &limits, &mut context)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn configuration() -> ExtractionConfig {
        crate::plugins::ensure_ocr_backends_initialized();
        ExtractionConfig {
            use_cache: false,
            ocr: Some(OcrConfig::default()),
            ..Default::default()
        }
    }

    fn failed_image(budget: bool) -> FailedOcrRun {
        if budget {
            let (document, pages, rotations) = super::super::open_pdf_for_full_ocr(SCAN).unwrap();
            assert_eq!(pages, 1);
            let limits = crate::extractors::security::SecurityLimits {
                max_content_size: 80 * 1024 * 1024,
                ..Default::default()
            };
            let (_, mut failures, _) =
                super::super::render_selected_pages_with_dpi(&document, &rotations, &[0], &limits, None, None);
            assert_eq!(failures.len(), 1, "the actual RGB render must refuse the budget");
            return failures.pop().unwrap().1.into();
        }
        crate::XbergError::Parsing {
            message: "render failed".into(),
            source: Some(Box::new(xberg_native_pdf::Error::Image(
                "controlled decoder failure".into(),
            ))),
        }
        .into()
    }

    #[test]
    fn only_a_typed_source_budget_failure_is_eligible_for_the_direct_rgb_retry() {
        let config = configuration();
        let ocr = super::super::super::resolve_ocr_config(&config);
        assert!(
            super::super::budget_retry_backend(&config, &ocr, &failed_image(true)).is_some(),
            "the positive case checks all other gates"
        );
        assert!(
            super::super::budget_retry_backend(&config, &ocr, &failed_image(false)).is_none(),
            "an unrelated decoder failure is not a budget refusal"
        );
    }

    #[test]
    fn retry_admission_rejects_each_nondefault_request() {
        let failed = failed_image(true);
        let base = serde_json::to_value(configuration()).unwrap();
        let changes = [
            (
                "/ocr_strategy",
                serde_json::json!({"mode": "scanned_pages", "min_confidence": 0.5}),
            ),
            ("/force_ocr", serde_json::json!(true)),
            ("/pages", serde_json::json!({"insert_page_markers": true})),
            ("/output_format", serde_json::json!("djot")),
            ("/ocr/language", serde_json::json!(["deu"])),
            (
                "/ocr/pipeline",
                serde_json::json!({"stages": [{"backend": "tesseract"}]}),
            ),
            ("/ocr/vlm_fallback", serde_json::json!({"mode": "always"})),
            ("/ocr/backend", serde_json::json!("unregistered-2880")),
            #[cfg(feature = "layout-types")]
            ("/layout", serde_json::json!({})),
        ];
        let mut admitted = Vec::new();
        for (path, value) in changes {
            let mut request = base.clone();
            let (parent, key) = path.rsplit_once('/').unwrap();
            request.pointer_mut(parent).unwrap()[key] = value;
            let config: ExtractionConfig = serde_json::from_value(request).unwrap_or_else(|error| {
                panic!("invalid control for {path}: {error}");
            });
            let ocr = super::super::super::resolve_ocr_config(&config);
            if path == "/force_ocr" {
                assert!(!super::super::super::document::native_preservation_is_automatic(
                    &config, 1
                ));
                assert!(super::super::super::tall_source::supports(&config, &ocr));
            }
            if path == "/ocr/language" {
                assert!(super::super::super::document::native_preservation_is_automatic(
                    &config, 1
                ));
                assert_eq!(
                    super::super::super::tall_source::unsupported_reason(&config, &ocr),
                    Some("the OCR language is not English only")
                );
            }
            if super::super::budget_retry_backend(&config, &ocr, &failed).is_some() {
                admitted.push(path);
            }
        }
        assert!(admitted.is_empty(), "retry admitted {admitted:?}");
    }

    #[tokio::test]
    async fn retry_admission_requires_the_registered_builtin_tesseract() {
        let config = configuration();
        let ocr = super::super::super::resolve_ocr_config(&config);
        let failed = failed_image(true);
        let replacement = std::sync::Arc::new(crate::ocr::tesseract_backend::TesseractBackend::new());
        crate::plugins::registry::with_test_ocr_backend(replacement, async {
            let registry = crate::plugins::registry::get_ocr_backend_registry();
            assert!(registry.read().get(&ocr.backend).is_ok());
            assert!(super::super::budget_retry_backend(&config, &ocr, &failed).is_none());
        })
        .await;
    }

    const SCAN: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/repros/label-scan-over-image-budget/label-scan.pdf"
    ));

    #[test]
    fn rgb_certificate_rejects_a_source_one_byte_over_the_limit() {
        let config = configuration();
        let doc = PdfDocument::from_bytes(SCAN.to_vec()).unwrap();
        let images = doc.page_image_handles(0).unwrap();
        let [image] = images.as_slice() else {
            panic!("expected one source image")
        };
        let (_, bytes) = image.plain_stored_jpeg_for_ocr_recovery().unwrap();
        let mut ocr = config.ocr.clone().unwrap();
        assert!(certified_rgb_page(&doc, &config, &ocr).unwrap());
        ocr.security_limits
            .get_or_insert_with(Default::default)
            .max_content_size = bytes.len() - 1;
        assert!(!certified_rgb_page(&doc, &config, &ocr).unwrap());
    }

    #[test]
    fn rgb_certificate_requires_matching_encoded_and_placed_dimensions() {
        let config = configuration();
        let ocr = config.ocr.as_ref().unwrap();
        let mut pdf = lopdf::Document::load_mem(SCAN).unwrap();
        let image = pdf
            .objects
            .values_mut()
            .find_map(|object| {
                let stream = object.as_stream_mut().ok()?;
                (stream.dict.get(b"Subtype").ok()?.as_name().ok()? == b"Image").then_some(stream)
            })
            .unwrap();
        let width = image.dict.get(b"Width").unwrap().as_i64().unwrap();
        image.dict.set("Width", width - 1);
        let mut bytes = Vec::new();
        pdf.save_to(&mut bytes).unwrap();
        let doc = PdfDocument::from_bytes(bytes).unwrap();
        let images = doc.page_image_handles(0).unwrap();
        assert_eq!(images.len(), 1);
        assert!(images[0].plain_stored_jpeg_for_ocr_recovery().is_some());
        assert!(!certified_rgb_page(&doc, &config, ocr).unwrap());
    }

    #[tokio::test]
    async fn a_second_physical_page_disqualifies_an_otherwise_certified_rgb_source() {
        let config = configuration();
        let mut pdf = lopdf::Document::load_mem(SCAN).unwrap();
        let first = pdf.get_pages()[&1];
        let page = pdf.get_dictionary(first).unwrap().clone();
        let parent = page.get(b"Parent").unwrap().as_reference().unwrap();
        let second = pdf.add_object(page);
        let pages = pdf.get_object_mut(parent).unwrap().as_dict_mut().unwrap();
        pages
            .get_mut(b"Kids")
            .unwrap()
            .as_array_mut()
            .unwrap()
            .push(second.into());
        pages.set("Count", 2);
        let mut bytes = Vec::new();
        pdf.save_to(&mut bytes).unwrap();
        let (document, pages, _) = super::super::open_pdf_for_full_ocr(&bytes).unwrap();
        assert_eq!(pages, 2);
        assert!(certified_rgb_page(&document, &config, config.ocr.as_ref().unwrap()).unwrap());
        assert!(
            try_recovery(Some(&bytes), &config, None, &failed_image(true))
                .await
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn rgb_certificate_rejects_a_stored_gray_jpeg() {
        let config = configuration();
        let ocr = config.ocr.as_ref().unwrap();
        let bytes = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/repros/tall-scan-strips/jpeg-tall.pdf"
        ));
        let document = PdfDocument::from_bytes(bytes.to_vec()).unwrap();
        let images = document.page_image_handles(0).unwrap();
        let [image] = images.as_slice() else {
            panic!("expected one gray source image")
        };
        assert!(matches!(
            image.color_space,
            xberg_native_pdf::extractors::images::ColorSpace::DeviceGray
        ));
        assert!(image.plain_stored_jpeg_for_ocr_recovery().is_some());
        assert!(!certified_rgb_page(&document, &config, ocr).unwrap());
    }

    #[test]
    fn rgb_certificate_rejects_a_second_painted_image() {
        let config = configuration();
        let mut pdf = lopdf::Document::load_mem(SCAN).unwrap();
        let page = pdf.get_pages()[&1];
        let content = pdf.get_page_content(page);
        let stream = pdf.add_object(lopdf::Stream::new(
            lopdf::Dictionary::new(),
            [content.as_slice(), b"\n", &content].concat(),
        ));
        pdf.get_object_mut(page)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("Contents", stream);
        let mut bytes = Vec::new();
        pdf.save_to(&mut bytes).unwrap();
        let document = PdfDocument::from_bytes(bytes).unwrap();
        assert_eq!(document.page_image_handles(0).unwrap().len(), 2);
        assert!(!certified_rgb_page(&document, &config, config.ocr.as_ref().unwrap()).unwrap());
    }

    #[cfg(feature = "bundle-tessdata-eng")]
    #[tokio::test]
    async fn two_image_reads_do_not_report_one_page_segmentation_mode() {
        let config = configuration();
        let mut ocr = config.ocr.unwrap();
        ocr.tesseract_config = Some(serde_json::from_value(serde_json::json!({"psm": 11})).unwrap());
        let document = PdfDocument::from_bytes(SCAN.to_vec()).unwrap();
        let limits = Default::default();
        let mut images = crate::pdf::native::images::page_ocr_fallback_image_bytes_with_limits(&document, 0, &limits);
        let [first] = images.as_mut_slice() else {
            panic!("expected one source image")
        };
        first.source_dpi = crate::pdf::native::proven_dpi::proven_source_dpis(&document, 0)
            .get(&first.paint_order)
            .and_then(|placement| placement.confirmed_by(first.format, &first.bytes, &limits));
        assert_eq!(first.source_dpi, Some(300.0));
        let mut second = images[0].clone();
        second.paint_order += 1;
        images.push(second);
        let registry = crate::plugins::registry::get_ocr_backend_registry();
        let backend = registry.read().get(&ocr.backend).unwrap();
        let mut single_image_text = String::new();
        for (count, expected) in [(1, Some(11)), (2, None)] {
            let mut budget = SecurityBudget::from_limits(&limits);
            let context = super::super::XObjectRecoveryContext::with_budget(&mut budget, None);
            let outcome =
                super::super::super::rendering::recover_image_xobjects(&backend, &images[..count], 0, 1, &ocr, context)
                    .await
                    .unwrap();
            assert!(outcome.failure_causes.is_empty(), "{:?}", outcome.failure_causes);
            assert!(outcome.text.contains("GENERAL"), "{}", outcome.text);
            if count == 1 {
                single_image_text = outcome.text;
            } else {
                assert_eq!(outcome.text, format!("{single_image_text}\n\n{single_image_text}"));
            }
            assert_eq!(outcome.attempted, count);
            assert_eq!(outcome.tesseract_psm, expected);
        }
    }

    #[tokio::test]
    async fn a_budget_failure_without_original_pdf_bytes_is_not_recovered() {
        let config = configuration();
        assert!(
            try_recovery(None, &config, None, &failed_image(true))
                .await
                .unwrap()
                .is_none()
        );
    }
}

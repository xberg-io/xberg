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
    if !eligible(config, &ocr, failed) {
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

fn eligible(config: &ExtractionConfig, ocr: &OcrConfig, failed: &FailedOcrRun) -> bool {
    if !super::render_refused_by_source_image_budget(&failed.error)
        || config.ocr_strategy != crate::core::config::OcrStrategy::Auto
        || !super::super::document::native_preservation_is_automatic(config, 1)
        || config.pages.as_ref().is_some_and(|pages| pages.insert_page_markers)
        || !super::super::tall_source::supports(config, ocr)
        || ocr.pipeline.is_some()
        || ocr.vlm_fallback != crate::core::config::VlmFallbackPolicy::Disabled
    {
        return false;
    }
    #[cfg(feature = "layout-detection")]
    if config.layout.is_some() {
        return false;
    }
    let registry = crate::plugins::registry::get_ocr_backend_registry();
    let registry = registry.read();
    registry
        .get(&ocr.backend)
        .is_ok_and(|backend| registry.is_builtin_tesseract(&backend))
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
            let bytes = include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../fixtures/repros/label-scan-over-image-budget/label-scan.pdf"
            ));
            let (document, pages, rotations) = super::super::open_pdf_for_full_ocr(bytes).unwrap();
            assert_eq!(pages, 1);
            let limits = crate::extractors::security::SecurityLimits {
                max_content_size: 80 * 1024 * 1024,
                ..Default::default()
            };
            let (_, mut failures, _) =
                super::super::render_selected_pages_with_dpi(&document, &rotations, &[0], &limits, None);
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
            eligible(&config, &ocr, &failed_image(true)),
            "the positive case checks all other gates"
        );
        assert!(
            !eligible(&config, &ocr, &failed_image(false)),
            "an unrelated decoder failure is not a budget refusal"
        );
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

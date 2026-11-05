use super::*;
use crate::core::config::OcrConfig;

// These helpers expose input admission and transport without claiming that the
// collected image words are a complete projection of the source PDF page. ~keep
pub(super) async fn whole(
    doc: &xberg_native_pdf::PdfDocument,
    config: &ExtractionConfig,
    ocr: &OcrConfig,
    backend_name: &str,
) -> crate::Result<PipelineXObjectRecoveryOutcome> {
    let backend = crate::plugins::registry::get_ocr_backend_registry()
        .read()
        .get(backend_name)?;
    let mut budget = crate::extractors::security::SecurityBudget::from_config(config);
    let ocr = pipeline_xobject_recovery_ocr_config(config, ocr);
    let mut result = collect_pipeline_xobject_pages(
        &backend,
        doc,
        doc.page_count().unwrap(),
        &ocr,
        XObjectRecoveryContext::with_budget(&mut budget, config.cancel_token.as_ref()),
        XObjectProjection::RawPayloads,
    )
    .await?;
    result.text = join_pipeline_xobject_text(&result.page_texts, config, &mut budget)?;
    Ok(result)
}

pub(super) async fn detached(
    doc: &xberg_native_pdf::PdfDocument,
    config: &ExtractionConfig,
    ocr: &OcrConfig,
    backend_name: &str,
) -> crate::Result<PipelineXObjectRecoveryOutcome> {
    let backend = crate::plugins::registry::get_ocr_backend_registry()
        .read()
        .get(backend_name)?;
    let mut budget = crate::extractors::security::SecurityBudget::from_config(config);
    let ocr = pipeline_xobject_recovery_ocr_config(config, ocr);
    let limits = ocr.security_limits.clone().unwrap_or_default();
    let images = crate::pdf::native::images::page_ocr_fallback_image_bytes_with_limits(doc, 0, &limits);
    let recovered = super::super::rendering::recover_image_xobjects(
        &backend,
        &images,
        0,
        doc.page_count().unwrap(),
        &ocr,
        XObjectRecoveryContext::with_budget(&mut budget, config.cancel_token.as_ref()),
    )
    .await?;
    let mut result = PipelineXObjectRecoveryOutcome::new(1);
    append_pipeline_xobject_page(&mut result, recovered, 0, 0, false);
    result.text = join_pipeline_xobject_text(&result.page_texts, config, &mut budget)?;
    Ok(result)
}

use super::{DetachedOcrPage, OcrPipelineOutput, OcrRunResult, PageOcrHints};
use crate::core::config::{ExtractionConfig, OcrStrategy};
use crate::extractors::security::SecurityLimits;
use std::sync::Arc;
use xberg_native_pdf::PdfDocument;

pub(in crate::extractors::pdf::ocr) struct StoredScan {
    pub(super) data: Arc<Vec<u8>>,
    pub(super) render: (u32, u32),
    source: (u32, u32),
    dpi: f64,
}

pub(super) fn eligible(config: &ExtractionConfig, page: u32) -> bool {
    config.ocr_strategy == OcrStrategy::Auto
        && !config.force_ocr
        && !config
            .force_ocr_pages
            .as_ref()
            .is_some_and(|pages| pages.contains(&page))
        && config.images.is_none()
        && !config.use_layout_for_markdown
        && config.ocr.as_ref().is_some_and(|ocr| {
            ocr.backend == "tesseract"
                && ocr.language.as_slice() == ["eng"]
                && ocr.pipeline.is_none()
                && ocr.backend_options.is_none()
                && ocr.tesseract_config.is_none()
                && ocr.vlm_fallback == crate::core::config::VlmFallbackPolicy::Disabled
                && !ocr.auto_rotate
        })
}

fn budget_skipped_upscale(dpi: &crate::types::ImagePreprocessingMetadata, render: (u32, u32)) -> bool {
    dpi.skipped_resize
        && dpi.auto_adjusted
        && dpi.resize_error.is_none()
        && dpi.original_dimensions.width == render.0 as usize
        && dpi.original_dimensions.height == render.1 as usize
        && crate::image::preprocessing::normalization_requires_resize(
            render.0,
            render.1,
            &crate::types::ImageDpiConfig::default(),
            dpi.original_dpi.horizontal,
        )
        && dpi.target_dpi > dpi.final_dpi
}

fn should_retry(primary: &OcrPipelineOutput, page: u32, render: (u32, u32)) -> bool {
    primary.12.original_tesseract_raster
        && primary.1.is_empty()
        && primary.3.as_ref().is_none_or(|doc| {
            doc.images.is_empty()
                && doc.tables.is_empty()
                && doc.relationships.is_empty()
                && doc.ocr_page_failures.is_empty()
        })
        && primary.4.is_empty()
        && primary.6.is_none()
        && primary.7.is_empty()
        && primary
            .11
            .iter()
            .any(|verdict| verdict.page_index + 1 == page as usize && verdict.low_confidence)
        && primary
            .9
            .get(&page)
            .is_some_and(|dpi| budget_skipped_upscale(dpi, render))
}

fn certified_scan(
    doc: &PdfDocument,
    page: usize,
    render: (u32, u32),
    limits: &SecurityLimits,
    config: &ExtractionConfig,
) -> crate::Result<Option<StoredScan>> {
    super::super::super::ensure_pdf_not_cancelled(config)?;
    let Some((frame, identity)) = super::stored_scan_paint::certify(doc, page, limits) else {
        return Ok(None);
    };
    let Ok(handles) = doc.page_image_handles(page) else {
        return Ok(None);
    };
    let [handle] = handles.as_slice() else { return Ok(None) };
    let Some((observed, bytes)) = handle.plain_stored_jpeg_for_ocr_recovery() else {
        return Ok(None);
    };
    if observed != identity
        || bytes
            .len()
            .checked_mul(2)
            .is_none_or(|bytes| bytes > limits.max_content_size)
    {
        return Ok(None);
    }
    let Some(dpi) =
        super::super::rendering::pre_rendered_page_source_dpi((frame.width, frame.height), handle.width, handle.height)
    else {
        return Ok(None);
    };
    super::super::super::ensure_pdf_not_cancelled(config)?;
    // The processor charges the stored stream plus this single owned copy. ~keep
    Ok(Some(StoredScan {
        data: Arc::new(bytes.to_vec()),
        render,
        source: (handle.width, handle.height),
        dpi,
    }))
}

async fn retry(
    doc: &PdfDocument,
    page: usize,
    render: (u32, u32),
    config: &ExtractionConfig,
    primary: &OcrPipelineOutput,
) -> super::OcrRunResult<Option<OcrPipelineOutput>> {
    let page_number = (page + 1) as u32;
    let ocr = super::pre_page_ocr_config(config);
    let mut limits = ocr
        .security_limits
        .clone()
        .or_else(|| config.security_limits.clone())
        .unwrap_or_default();
    limits.max_content_size = super::pipeline_xobject_content_limit(config, &ocr);
    let Some(scan) = certified_scan(doc, page, render, &limits, config)? else {
        return Ok(None);
    };
    let candidate = read_stored_scan(doc, page, &scan, config).await?;
    super::super::super::ensure_pdf_not_cancelled(config)?;
    let accepted = primary
        .10
        .get(&page_number)
        .zip(candidate.10.get(&page_number))
        .is_some_and(|(old, new)| {
            old.score.zip(new.score).is_some_and(|(old, new)| new > old)
                && new.word_count >= old.word_count
                && candidate.9.get(&page_number).is_some_and(|dpi| {
                    primary
                        .9
                        .get(&page_number)
                        .is_some_and(|old| dpi.final_dpi > old.final_dpi)
                })
        });
    Ok((accepted && !candidate.0.trim().is_empty() && candidate.1.is_empty()).then_some(candidate))
}

pub(super) async fn maybe_retry(
    doc: &PdfDocument,
    page: usize,
    image: Arc<image::DynamicImage>,
    config: &ExtractionConfig,
    primary: OcrRunResult<OcrPipelineOutput>,
    allowed: bool,
) -> OcrRunResult<OcrPipelineOutput> {
    let render = (image.width(), image.height());
    let mut primary = primary?;
    if !allowed || !should_retry(&primary, (page + 1) as u32, render) {
        return Ok(primary);
    }
    // The pipeline stage's PNG and all other source raster references are gone here. ~keep
    if Arc::strong_count(&image) != 1 {
        return Ok(primary);
    }
    drop(image);
    match retry(doc, page, render, config, &primary).await {
        Ok(Some(mut candidate)) => {
            if let Some(primary_doc) = primary.3.take()
                && !primary_doc.processing_warnings.is_empty()
            {
                let doc = candidate
                    .3
                    .get_or_insert_with(|| super::ocr_pipeline_text_document(&candidate.0));
                crate::core::diagnostics::dedup_extend_warnings(
                    &mut doc.processing_warnings,
                    primary_doc.processing_warnings,
                );
            }
            Ok(candidate)
        }
        Ok(None) => Ok(primary),
        Err(failed) if matches!(failed.error, crate::XbergError::Cancelled) => Err(failed),
        Err(failed) => {
            tracing::debug!(page, error = %failed.error, "stored JPEG OCR retry kept the rendered read");
            Ok(primary)
        }
    }
}

async fn read_stored_scan(
    doc: &PdfDocument,
    page: usize,
    scan: &StoredScan,
    config: &ExtractionConfig,
) -> OcrRunResult<OcrPipelineOutput> {
    let mut config = config.clone();
    config.ocr.get_or_insert_with(Default::default).tesseract_config = Some(crate::types::TesseractConfig {
        enable_table_detection: false,
        ..Default::default()
    });
    let (_, height_pt) = super::page_dimensions_pt(doc, page);
    let source = DetachedOcrPage {
        document: doc,
        stored_scan: Some(scan),
        orientation_retry: None,
    };
    let (
        text,
        _,
        tables,
        elements,
        doc,
        usage,
        texts,
        rasters,
        formulas,
        paragraphs,
        dpi,
        confidence,
        verdicts,
        evidence,
    ) = Box::pin(super::extract_with_ocr_for_page(
        None,
        None,
        #[cfg(feature = "layout-detection")]
        None,
        &config,
        None,
        0,
        true,
        Some(height_pt / scan.render.1 as f32),
        Some(PageOcrHints {
            source_dpi: Some(scan.dpi),
            known_full_page_scan: true,
            ..Default::default()
        }),
        page,
        Some(source),
    ))
    .await?;
    Ok((
        text, tables, elements, doc, usage, texts, rasters, formulas, paragraphs, dpi, confidence, verdicts, evidence,
    ))
}

pub(super) fn place_in_render_frame(
    result: &mut crate::types::ExtractedDocument,
    scan: &StoredScan,
) -> crate::Result<()> {
    let source_matches = result.metadata.image_preprocessing.as_ref().is_some_and(|dpi| {
        (dpi.original_dimensions.width, dpi.original_dimensions.height)
            == (scan.source.0 as usize, scan.source.1 as usize)
            && dpi.original_dpi.horizontal == scan.dpi
    });
    let processed = super::super::document::processed_ocr_layout_dimensions(&result.metadata);
    let Some((width, height)) =
        processed.filter(|_| source_matches && result.tables.is_empty() && result.formulas.is_empty())
    else {
        return Err(crate::XbergError::validation(
            "Stored JPEG OCR returned an unverified geometry frame",
        ));
    };
    let scale_x = f64::from(scan.render.0) / f64::from(width);
    let scale_y = f64::from(scan.render.1) / f64::from(height);
    let transform = |x, y| (x * scale_x, y * scale_y);
    if let Some(doc) = result.ocr_internal_document.as_mut() {
        for element in &mut doc.elements {
            if let Some(bbox) = element.bbox.as_mut() {
                (bbox.x0, bbox.y0) = transform(bbox.x0, bbox.y0);
                (bbox.x1, bbox.y1) = transform(bbox.x1, bbox.y1);
            }
            if let Some(geometry) = element.ocr_geometry.as_mut() {
                super::undo_ocr_element_geometry(geometry, transform);
            }
        }
    }
    for element in result.ocr_elements.iter_mut().flatten() {
        super::undo_ocr_element_geometry(&mut element.geometry, transform);
    }
    use crate::ocr_metadata_keys::{OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY, OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY};
    result
        .metadata
        .additional
        .insert(OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY.into(), scan.render.0.into());
    result
        .metadata
        .additional
        .insert(OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY.into(), scan.render.1.into());
    Ok(())
}

#[cfg(test)]
#[path = "stored_scan_retry_tests.rs"]
mod tests;

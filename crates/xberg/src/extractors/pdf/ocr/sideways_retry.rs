//! Second read for a scanned page whose text runs sideways in the raster (GH#2314).
//!
//! A page that is one full-page raster is read with sparse-text segmentation (psm 11), which
//! keeps the table values that automatic layout loses on upright scans (GH#1786). Sparse-text
//! mode also drops most words of a page whose lines run sideways. Such a page carries the
//! low-confidence noise verdict, so that verdict triggers one more read with automatic layout.
//! The second read replaces the first only when it scores strictly higher and its word boxes
//! are mostly taller than wide, which keeps upright pages on psm 11.

use std::sync::Arc;

use super::scoring::{compute_quality_score, mean_text_conf_of, page_carries_noise_verdict};
use crate::core::config::{OcrConfig, OcrQualityThresholds};
use crate::types::{ExtractedDocument, OcrBoundingGeometry, OcrElementLevel};

/// Read one whole-page raster: the base read of the image extractor, then the sideways retry.
pub(super) async fn read_whole_page_raster(
    backend: &Arc<dyn crate::plugins::OcrBackend>,
    image_bytes: Arc<Vec<u8>>,
    original_config: &OcrConfig,
    effective_config: &OcrConfig,
    whole_page_raster: bool,
    thresholds: &OcrQualityThresholds,
    cancel_token: Option<&crate::cancellation::CancellationToken>,
) -> crate::Result<ExtractedDocument> {
    let primary = crate::extractors::image::process_whole_image_ocr_with_sparse_retry(
        backend,
        Arc::clone(&image_bytes),
        original_config,
        effective_config,
        crate::extractors::image::SparseRetryPolicy {
            whole_page_raster,
            allow_standard_retry: false,
        },
        cancel_token,
    )
    .await?;
    if !whole_page_raster || !crate::extractors::image::is_implicit_horizontal_tesseract(original_config) {
        return Ok(primary);
    }
    let dict_ratio = primary
        .metadata
        .additional
        .get(crate::ocr_metadata_keys::OCR_TESSERACT_DICT_INVALID_WORD_RATIO_METADATA_KEY)
        .and_then(serde_json::Value::as_f64);
    if !page_carries_noise_verdict(
        &primary.content,
        thresholds,
        backend.confidence_semantics(),
        mean_text_conf_of(&primary.metadata.additional),
        dict_ratio,
    ) {
        return Ok(primary);
    }
    if cancel_token.is_some_and(crate::cancellation::CancellationToken::is_cancelled) {
        return Err(crate::XbergError::Cancelled);
    }

    let caller_wants_elements = original_config
        .element_config
        .as_ref()
        .is_some_and(|elements| elements.include_elements);
    let mut retry_config = crate::extractors::image::sparse_image_ocr_fallback_config(effective_config);
    if !caller_wants_elements {
        let elements = retry_config
            .element_config
            .get_or_insert_with(crate::types::OcrElementConfig::default);
        elements.include_elements = true;
        elements.min_level = OcrElementLevel::Word;
        elements.min_confidence = 0.0;
    }
    let mut retry = match backend.process_image_owned(image_bytes, &retry_config).await {
        Ok(retry) => retry,
        Err(error @ crate::XbergError::Cancelled) => return Err(error),
        Err(error) => {
            tracing::warn!(%error, "sideways-page OCR retry failed; keeping the sparse-text read");
            return Ok(primary);
        }
    };
    let retry_score = compute_quality_score(&retry.content, thresholds);
    let improves = retry_score > compute_quality_score(&primary.content, thresholds);
    if !improves || !words_run_sideways(&retry) {
        return Ok(primary);
    }
    if !caller_wants_elements {
        retry.ocr_elements = None;
    }
    crate::extractors::image::merge_primary_warnings_into_selected(&primary, &mut retry);
    Ok(retry)
}

/// True when more than half of the read's word boxes are taller than wide.
fn words_run_sideways(document: &ExtractedDocument) -> bool {
    let (mut tall, mut total) = (0usize, 0usize);
    for element in document.ocr_elements.iter().flatten() {
        if element.level == OcrElementLevel::Page {
            continue;
        }
        if let OcrBoundingGeometry::Rectangle { width, height, .. } = element.geometry {
            total += 1;
            if height > width {
                tall += 1;
            }
        }
    }
    tall * 2 > total
}

/// The noise thresholds the page verdict uses, from the caller's OCR configuration.
pub(super) fn noise_thresholds(config: &crate::core::config::ExtractionConfig) -> OcrQualityThresholds {
    config
        .ocr
        .as_ref()
        .and_then(|ocr| ocr.quality_thresholds.clone())
        .unwrap_or_default()
}

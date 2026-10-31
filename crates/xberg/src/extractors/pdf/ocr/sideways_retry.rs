//! Second read for a scanned page whose text runs sideways in the raster (GH#2314).
//!
//! A page that is one full-page raster is read with sparse-text segmentation (psm 11), which
//! keeps the table values that automatic layout loses on upright scans (GH#1786). Sparse-text
//! mode also drops most words of a page whose lines run sideways. Such a page carries the
//! low-confidence noise verdict, so that verdict triggers one more read with automatic layout.
//! The second read replaces the first only when it scores strictly higher, reads at least as many
//! words, and its word boxes are mostly taller than wide. The word floor keeps a page whose first
//! read holds real words on psm 11; the box shape keeps upright pages there.

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

    // ~keep The decision reads private, unfiltered word geometry whatever the caller selected;
    // the caller's own element selection is restored on the kept read below.
    let mut retry_config = crate::extractors::image::sparse_image_ocr_fallback_config(effective_config);
    retry_config.element_config = Some(crate::types::OcrElementConfig {
        include_elements: true,
        min_level: OcrElementLevel::Word,
        min_confidence: 0.0,
        build_hierarchy: false,
    });
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
    if !improves || count_words(&retry.content) < count_words(&primary.content) || !words_run_sideways(&retry) {
        return Ok(primary);
    }
    retry.ocr_elements = match (original_config.element_config.as_ref(), retry.ocr_elements.take()) {
        (Some(selection), Some(elements)) if selection.include_elements => Some(selection.select_elements(&elements)),
        _ => None,
    };
    crate::extractors::image::merge_primary_warnings_into_selected(&primary, &mut retry);
    Ok(retry)
}

/// Words of three or more letters or digits. Only a floor uses it: a retry that reads fewer of
/// them than the first read has lost words, whatever its quality score says.
pub(super) fn count_words(text: &str) -> usize {
    text.split_whitespace()
        .filter(|token| token.chars().filter(|character| character.is_alphanumeric()).count() >= 3)
        .count()
}

/// True when more than half of the read's word boxes are taller than wide. Lines, blocks and the
/// page box are not words: a tall upright paragraph says nothing about the direction of the text.
fn words_run_sideways(document: &ExtractedDocument) -> bool {
    let (mut tall, mut total) = (0usize, 0usize);
    for element in document.ocr_elements.iter().flatten() {
        if element.level != OcrElementLevel::Word {
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

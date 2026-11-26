//! Read a standalone photo again with the built-in PaddleOCR backend when the default
//! Tesseract read of it is sparse and unreliable (`OcrConfig::image_paddle_fallback`).
//!
//! Camera photos (receipts, scene text) defeat Tesseract at every PSM and threshold the image
//! route tries, while PaddleOCR's detector reads them. Each backend's confidence is read on its
//! own scale only: Tesseract's decides which of its words it read with confidence, PaddleOCR's
//! which of its lines it did, and the two counts of confidently read tokens are compared.

use super::{has_robust_word_confidence_distribution, is_implicit_horizontal_tesseract, usable_word_confidences};
use crate::cancellation::{CancellationToken, check_cancelled};
use crate::core::config::{ExtractionConfig, OcrConfig, OcrQualityThresholds, VlmFallbackPolicy};
use crate::plugins::OcrBackend;
use crate::plugins::registry::OcrBackendRegistry;
use crate::types::internal::InternalDocument;
use crate::types::{ExtractedDocument, OcrElementLevel};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex, PoisonError};

const PADDLE_BACKEND: &str = "paddle-ocr";

/// Metadata key set on a read this fallback adopted, naming the backend that produced it.
pub(super) const FALLBACK_BACKEND_METADATA_KEY: &str = "ocr_fallback_backend";

/// Recognition confidence (0-1, each backend on its own scale) at or above which a Tesseract
/// word or a PaddleOCR line counts as confidently read. The bar `image_color_threshold` uses.
const CONFIDENT_READ: f64 = 0.70;

/// Most usable Tesseract words a photo-shaped read has. Receipts and scene photos read 13-40;
/// a dense page scan reads hundreds, and its noise never hands it to PaddleOCR. The bound
/// `image_color_threshold` uses for its retry.
const MAX_SPARSE_PRIMARY_WORDS: usize = 80;

/// Fewest confidently read tokens PaddleOCR must add over Tesseract's, as in
/// `image_color_threshold`'s growth rule: `max(MIN_TOKEN_GAIN, trusted / 4)`.
const MIN_TOKEN_GAIN: usize = 4;

/// Model sets this process has checked, by cache root and selection. Bounded; cleared when full.
const MAX_MEMOIZED_MODEL_SETS: usize = 64;

static MODEL_SETS: LazyLock<ModelMemo> = LazyLock::new(ModelMemo::default);

/// The built-in PaddleOCR backend when `primary` is the built-in Tesseract backend and the
/// request is [`eligible`]; a backend registered in place of either keeps the request as is.
pub(super) fn backend(
    registry: &OcrBackendRegistry,
    primary: &Arc<dyn OcrBackend>,
    config: &ExtractionConfig,
    ocr: &OcrConfig,
    mime_type: &str,
) -> Option<Arc<dyn OcrBackend>> {
    (registry.is_builtin_tesseract(primary) && registry.has_builtin_paddle() && eligible(config, ocr, mime_type))
        .then(|| registry.get("paddleocr").ok())
        .flatten()
}

/// Whether the request leaves the backend choice to Xberg: the default Tesseract read with every
/// Tesseract setting at its default, no multi-backend policy, no layout, and one language
/// PaddleOCR reads with one model.
///
/// An explicit `backend: "tesseract"` with default settings cannot be told apart from the
/// default; `image_paddle_fallback: false` keeps Tesseract's read.
fn eligible(config: &ExtractionConfig, ocr: &OcrConfig, mime_type: &str) -> bool {
    #[cfg(feature = "layout-types")]
    let layout = config.layout.is_some();
    #[cfg(not(feature = "layout-types"))]
    let layout = {
        let _ = config;
        false
    };
    ocr.image_paddle_fallback
        && !layout
        && is_implicit_horizontal_tesseract(ocr)
        && tesseract_settings_are_default(ocr)
        && !ocr.has_tuning_backend_options()
        && ocr.pipeline.is_none()
        && ocr.vlm_fallback == VlmFallbackPolicy::Disabled
        && !mime_type.to_ascii_lowercase().contains("tiff")
        && reads_one_paddle_language(ocr)
}

/// Every Tesseract setting holds its default. `TesseractConfig` is compared whole, through its
/// serialized form, so a field added later makes a request ineligible until it is listed here;
/// only `use_cache` is exempt, because it changes no recognition.
fn tesseract_settings_are_default(ocr: &OcrConfig) -> bool {
    let tesseract_default = ocr.tesseract_config.as_ref().is_none_or(|tesseract| {
        let neutral = crate::types::TesseractConfig {
            use_cache: tesseract.use_cache,
            ..Default::default()
        };
        matches!(
            (serde_json::to_value(tesseract), serde_json::to_value(&neutral)),
            (Ok(given), Ok(default)) if given == default
        )
    });
    tesseract_default && ocr.tessdata_path.is_none() && ocr.tessdata_bytes.is_none() && !ocr.numeric_repair
}

fn reads_one_paddle_language(ocr: &OcrConfig) -> bool {
    let languages = ocr.effective_languages();
    matches!(languages.as_slice(), [language] if crate::paddle_ocr::map_language_code(language).is_some())
        && ocr.effective_tesseract_language() == languages
}

/// Whether `doc` was built from a PaddleOCR read this fallback selected, so a later Tesseract
/// re-read of the first pass does not replace it.
pub(super) fn read_by_paddle(doc: &InternalDocument) -> bool {
    doc.prebuilt_pages
        .iter()
        .flatten()
        .filter_map(|page| page.ocr_confidence.as_ref())
        .any(|confidence| confidence.backend == PADDLE_BACKEND)
}

/// A photo-shaped Tesseract read that is unreliable: it found text (at most
/// [`MAX_SPARSE_PRIMARY_WORDS`] words) and fails the sparse-text robustness check.
fn sparse_unreliable_read(primary: &ExtractedDocument) -> bool {
    (1..=MAX_SPARSE_PRIMARY_WORDS).contains(&usable_word_confidences(primary).len())
        && !has_robust_word_confidence_distribution(primary)
}

fn paddle_read_config(effective: &OcrConfig) -> OcrConfig {
    let mut config = effective.clone();
    config.backend = PADDLE_BACKEND.to_string();
    config.tesseract_config = None;
    config
}

fn alphanumeric_token_count(text: &str) -> usize {
    text.split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .count()
}

fn confident_tokens(document: &ExtractedDocument, level: OcrElementLevel) -> usize {
    document
        .ocr_elements
        .iter()
        .flatten()
        .filter(|element| element.level == level && element.confidence.recognition >= CONFIDENT_READ)
        .map(|element| alphanumeric_token_count(&element.text))
        .sum()
}

/// The PaddleOCR read replaces the Tesseract read when its confidently read lines hold at least
/// `max(MIN_TOKEN_GAIN, trusted / 4)` more letter-or-digit tokens than Tesseract's confidently
/// read words, its text is not empty, and that text clears the quality gate a PDF OCR pipeline
/// stage must clear (`pipeline_min_quality`).
fn paddle_read_wins(
    primary: &ExtractedDocument,
    candidate: &ExtractedDocument,
    thresholds: &OcrQualityThresholds,
) -> bool {
    let trusted = confident_tokens(primary, OcrElementLevel::Word);
    let read = confident_tokens(candidate, OcrElementLevel::Line);
    !candidate.content.trim().is_empty()
        && read >= trusted + MIN_TOKEN_GAIN.max(trusted / 4)
        && crate::extractors::pdf::ocr::compute_quality_score(&candidate.content, thresholds)
            >= thresholds.pipeline_min_quality
}

/// Which model sets are known to be cached and intact, by [`crate::paddle_ocr::RequiredModelSet::key`].
#[derive(Default)]
struct ModelMemo(Mutex<HashMap<String, bool>>);

impl ModelMemo {
    /// Whether the model set is ready to read without a download. `present` (no hashing) runs
    /// every time, so a set removed after it was verified fails closed instead of downloading;
    /// `verify` (hashing, on a blocking thread) runs once per key, and its answer is kept.
    async fn ready(
        &self,
        key: String,
        present: impl FnOnce() -> bool,
        verify: impl FnOnce() -> bool + Send + 'static,
    ) -> bool {
        if !present() {
            return false;
        }
        if let Some(&verified) = self.lock().get(&key) {
            return verified;
        }
        let verified = tokio::task::spawn_blocking(verify).await.unwrap_or(false);
        let mut memo = self.lock();
        if memo.len() >= MAX_MEMOIZED_MODEL_SETS {
            memo.clear();
        }
        memo.insert(key, verified);
        verified
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, bool>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Whether the built-in backend's models for `config` are cached and intact. The built-in
/// backend is registered with `PaddleOcrConfig::default()`; [`backend`] admits no other.
async fn models_ready(config: &OcrConfig) -> bool {
    let set = match crate::paddle_ocr::RequiredModelSet::resolve(&Default::default(), config) {
        Ok(set) => set,
        Err(error) => {
            tracing::debug!(%error, "PaddleOCR image fallback skipped: model set unresolved");
            return false;
        }
    };
    let set = Arc::new(set);
    let verify = Arc::clone(&set);
    MODEL_SETS
        .ready(set.key(), || set.present(), move || verify.verified())
        .await
}

/// Returns the selected read and, when PaddleOCR produced it, the PaddleOCR backend.
///
/// `paddle` is the built-in PaddleOCR backend, or `None` when it is not registered or the
/// request is not [`eligible`]; then the primary read is returned unchanged.
pub(super) async fn retry(
    paddle: Option<Arc<dyn OcrBackend>>,
    bytes: Arc<Vec<u8>>,
    configs: (&OcrConfig, &OcrConfig),
    cancel: Option<&CancellationToken>,
    primary: ExtractedDocument,
) -> crate::Result<(ExtractedDocument, Option<Arc<dyn OcrBackend>>)> {
    retry_with(paddle, bytes, configs, cancel, primary, models_ready).await
}

async fn retry_with(
    paddle: Option<Arc<dyn OcrBackend>>,
    bytes: Arc<Vec<u8>>,
    (original, effective): (&OcrConfig, &OcrConfig),
    cancel: Option<&CancellationToken>,
    primary: ExtractedDocument,
    ready: impl AsyncFnOnce(&OcrConfig) -> bool,
) -> crate::Result<(ExtractedDocument, Option<Arc<dyn OcrBackend>>)> {
    let Some(paddle) = paddle else {
        return Ok((primary, None));
    };
    if !sparse_unreliable_read(&primary) {
        return Ok((primary, None));
    }
    // From here the result depends on the local model cache, which the extraction cache key does
    // not carry, so a cold and a warm model cache must not serve each other's results. ~keep
    crate::core::extractor::cacheability::invalidate_current();
    let config = paddle_read_config(effective);
    if !ready(&config).await {
        tracing::debug!("PaddleOCR image fallback skipped: models are not cached locally");
        return Ok((primary, None));
    }
    let thresholds = original.effective_thresholds();
    let (selected, adopted) = read_if_better(
        cancel,
        async || paddle.process_image_owned(bytes, &config).await,
        primary,
        &thresholds,
    )
    .await?;
    Ok((selected, adopted.then_some(paddle)))
}

async fn read_if_better(
    cancel: Option<&CancellationToken>,
    read: impl AsyncFnOnce() -> crate::Result<ExtractedDocument>,
    primary: ExtractedDocument,
    thresholds: &OcrQualityThresholds,
) -> crate::Result<(ExtractedDocument, bool)> {
    check_cancelled(cancel)?;
    let result = read().await;
    check_cancelled(cancel)?;
    match result {
        Ok(mut candidate) if paddle_read_wins(&primary, &candidate, thresholds) => {
            tracing::debug!("PaddleOCR image fallback adopted");
            candidate.metadata.additional.insert(
                std::borrow::Cow::Borrowed(FALLBACK_BACKEND_METADATA_KEY),
                serde_json::Value::String(PADDLE_BACKEND.to_string()),
            );
            Ok((candidate, true))
        }
        Ok(_) => {
            tracing::debug!("PaddleOCR image fallback rejected; keeping the Tesseract read");
            Ok((primary, false))
        }
        Err(error @ crate::XbergError::Cancelled) => Err(error),
        Err(error) => {
            tracing::warn!(%error, "PaddleOCR image fallback failed; keeping the Tesseract read");
            Ok((primary, false))
        }
    }
}

/// A page carrying the given backend's OCR confidence, for tests of [`read_by_paddle`].
#[cfg(test)]
pub(super) fn page_read_by(backend: &str) -> crate::types::PageContent {
    crate::types::PageContent {
        page_number: 1,
        content: "TOTAL".to_string(),
        tables: vec![],
        image_indices: vec![],
        image_preprocessing: None,
        hierarchy: None,
        is_blank: None,
        layout_regions: None,
        speaker_notes: None,
        section_name: None,
        sheet_name: None,
        ocr_confidence: Some(crate::types::page::PageOcrConfidence {
            score: None,
            word_count: 0,
            backend: backend.to_string(),
        }),
        native_content: None,
        geometry: None,
        viewports: None,
    }
}

#[cfg(test)]
#[path = "image_paddle_fallback_tests.rs"]
mod tests;

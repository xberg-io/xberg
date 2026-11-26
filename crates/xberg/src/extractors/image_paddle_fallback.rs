//! Read a standalone camera photo again with the built-in PaddleOCR backend when the default
//! Tesseract read of it is sparse and unreliable (`OcrConfig::image_paddle_fallback`).
//!
//! Camera photos (receipts, scene text) defeat Tesseract at every PSM and threshold the image
//! route tries, while PaddleOCR's detector reads them. Confidences are never compared across
//! backends. Tesseract's is calibrated, so it decides which Tesseract words were read; PaddleOCR's
//! line score is its recognition text score, which stays at 0.92-1.00 on reads with no correct
//! word (measured on the benchmark photos), so the PaddleOCR read is judged by its text alone.

use super::{has_robust_word_confidence_distribution, is_implicit_horizontal_tesseract, usable_word_confidences};
use crate::cancellation::{CancellationToken, check_cancelled};
use crate::core::config::{ExtractionConfig, OcrConfig, OcrQualityThresholds, VlmFallbackPolicy};
use crate::plugins::OcrBackend;
use crate::plugins::registry::OcrBackendRegistry;
use crate::types::internal::InternalDocument;
use crate::types::{ExtractedDocument, OcrElementLevel};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, Mutex, PoisonError};
use std::time::SystemTime;

const PADDLE_BACKEND: &str = "paddle-ocr";

/// Metadata key set on a read this fallback adopted, naming the backend that produced it.
pub(super) const FALLBACK_BACKEND_METADATA_KEY: &str = "ocr_fallback_backend";

/// Lossy camera formats. A PNG, BMP, GIF or TIFF is a screenshot, slide, form or scan; HEIC
/// reaches OCR already converted to PNG, so it does not qualify either.
const CAMERA_IMAGE_TYPES: &[&str] = &["image/jpeg", "image/jpg", "image/pjpeg", "image/webp"];

/// Tesseract word confidence (0-1, calibrated) at or above which a Tesseract word counts as read.
/// The bar `image_color_threshold` uses for a confident word.
const CONFIDENT_TESSERACT_WORD: f64 = 0.70;

/// Most usable Tesseract words a photo-shaped read has. Receipts and scene photos read 13-40;
/// page scans read hundreds. The bound `image_color_threshold` uses for its retry.
const MAX_SPARSE_PRIMARY_WORDS: usize = 80;

/// Fewest plausible tokens a PaddleOCR read must hold to replace anything.
const MIN_PLAUSIBLE_TOKENS: usize = 4;

/// Least share of a PaddleOCR read's tokens that must be plausible. The benchmark photos measure
/// 0.69-0.92; detector noise on a texture reads letter fragments that are not.
const MIN_PLAUSIBLE_SHARE: f64 = 0.6;

/// Longest letter run counted as a plausible word.
const MAX_PLAUSIBLE_WORD_CHARS: usize = 20;

/// Model sets this process has checked, by cache root and selection. Bounded; cleared when full.
const MAX_MEMOIZED_MODEL_SETS: usize = 64;

static MODEL_SETS: LazyLock<ModelMemo> = LazyLock::new(ModelMemo::default);

/// The built-in PaddleOCR backend when `primary` is the built-in Tesseract backend and the
/// request is [`eligible`]; a backend registered in place of either keeps the request as is.
pub(super) fn backend(
    registry: &OcrBackendRegistry,
    primary: &Arc<dyn OcrBackend>,
    (config, ocr): (&ExtractionConfig, &OcrConfig),
    content: &[u8],
    mime_type: &str,
) -> Option<Arc<dyn OcrBackend>> {
    (registry.is_builtin_tesseract(primary)
        && registry.has_builtin_paddle()
        && eligible(config, ocr, mime_type)
        && is_camera_photo(content, mime_type))
    .then(|| registry.get("paddleocr").ok())
    .flatten()
}

/// Whether the request leaves the backend choice to Xberg: the default Tesseract read with no
/// `tesseract_config` at all (even a default one is a choice, as the benchmark adapter and the
/// default OCR routes already treat it), no tessdata override, no numeric repair, no backend
/// tuning, no multi-backend policy, no layout, and one language PaddleOCR reads with one model.
///
/// An explicit `backend: "tesseract"` with no other setting cannot be told apart from the
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
        && ocr.tesseract_config.is_none()
        && ocr.tessdata_path.is_none()
        && ocr.tessdata_bytes.is_none()
        && !ocr.numeric_repair
        && !ocr.has_tuning_backend_options()
        && ocr.pipeline.is_none()
        && ocr.vlm_fallback == VlmFallbackPolicy::Disabled
        && !mime_type.to_ascii_lowercase().contains("tiff")
        && reads_one_paddle_language(ocr)
}

/// A camera photo: a lossy camera format whose pixels match no standard page size at a scan
/// resolution (`resolve_known_source_dpi`, the signal the OCR route uses to spot page scans).
fn is_camera_photo(content: &[u8], mime_type: &str) -> bool {
    if !CAMERA_IMAGE_TYPES
        .iter()
        .any(|kind| mime_type.eq_ignore_ascii_case(kind))
    {
        return false;
    }
    let Some((width, height)) = image::ImageReader::new(std::io::Cursor::new(content))
        .with_guessed_format()
        .ok()
        .and_then(|reader| reader.into_dimensions().ok())
    else {
        return false;
    };
    crate::extraction::image::resolve_known_source_dpi(None, content, width, height).is_none()
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

/// The Tesseract-only recoveries after the read (tall-strip re-read, photo-title recovery) as
/// requested, or both off on a read this fallback adopted from PaddleOCR.
pub(super) fn tesseract_recoveries<Title>(
    adopted: Option<&Arc<dyn OcrBackend>>,
    recover_tall: bool,
    title_source: Option<Title>,
) -> (bool, Option<Title>) {
    if adopted.is_none() {
        (recover_tall, title_source)
    } else {
        (false, None)
    }
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

fn tokens(text: &str) -> impl Iterator<Item = &str> {
    text.split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
}

/// A token that reads like a word or a number: at least two characters, and either all digits
/// or all letters with a vowel (any letter outside ASCII counts as one), at most
/// [`MAX_PLAUSIBLE_WORD_CHARS`] long. Misreads such as `x7Q`, `rn`, `sa` or `CoE` mostly fail it.
fn is_plausible_token(token: &str) -> bool {
    let length = token.chars().count();
    length >= 2
        && (token.chars().all(|character| character.is_ascii_digit())
            || (length <= MAX_PLAUSIBLE_WORD_CHARS
                && token.chars().all(char::is_alphabetic)
                && token
                    .chars()
                    .any(|character| !character.is_ascii() || "aeiouyAEIOUY".contains(character))))
}

fn plausible_tokens(text: &str) -> usize {
    tokens(text).filter(|token| is_plausible_token(token)).count()
}

/// Plausible tokens among the words Tesseract read with confidence.
fn trusted_tesseract_tokens(primary: &ExtractedDocument) -> usize {
    primary
        .ocr_elements
        .iter()
        .flatten()
        .filter(|element| {
            element.level == OcrElementLevel::Word && element.confidence.recognition >= CONFIDENT_TESSERACT_WORD
        })
        .map(|element| plausible_tokens(&element.text))
        .sum()
}

/// The PaddleOCR read replaces the Tesseract read when its text keeps at least as many plausible
/// tokens as Tesseract read with confidence (and at least [`MIN_PLAUSIBLE_TOKENS`]), at least
/// [`MIN_PLAUSIBLE_SHARE`] of its tokens are plausible, and the text clears the quality gate a
/// PDF OCR pipeline stage must clear (`pipeline_min_quality`). The read it replaces already
/// failed Tesseract's own robustness check, so the PaddleOCR read must not lose what Tesseract
/// did read, not out-read it: on cord_receipt_04 PaddleOCR is exact with 14 plausible tokens
/// against Tesseract's 12 confident ones.
fn paddle_read_wins(
    primary: &ExtractedDocument,
    candidate: &ExtractedDocument,
    thresholds: &OcrQualityThresholds,
) -> bool {
    let all = tokens(&candidate.content).count();
    let plausible = plausible_tokens(&candidate.content);
    !candidate.content.trim().is_empty()
        && plausible >= MIN_PLAUSIBLE_TOKENS.max(trusted_tesseract_tokens(primary))
        && plausible as f64 >= MIN_PLAUSIBLE_SHARE * all as f64
        && crate::extractors::pdf::ocr::compute_quality_score(&candidate.content, thresholds)
            >= thresholds.pipeline_min_quality
}

/// Size and modification time of each model file.
type FileStamps = Vec<(PathBuf, u64, Option<SystemTime>)>;

fn stamp_files(files: Vec<PathBuf>) -> Option<FileStamps> {
    files
        .into_iter()
        .map(|path| {
            let metadata = std::fs::metadata(&path).ok()?;
            Some((path, metadata.len(), metadata.modified().ok()))
        })
        .collect()
}

/// Verification results by model set, kept with the file stamps they were made on.
#[derive(Default)]
struct ModelMemo(Mutex<HashMap<String, (FileStamps, bool)>>);

impl ModelMemo {
    /// Whether the model set is ready to read without a download. `stamps` is `None` when a file
    /// is missing: that fails closed, and a set removed after it was verified never reaches the
    /// backend, which would download it. `verify` (hashing) runs again only when the files'
    /// sizes or modification times change, for a verified or a failed set alike.
    fn ready(&self, key: String, stamps: Option<FileStamps>, verify: impl FnOnce() -> bool) -> bool {
        let Some(stamps) = stamps else {
            return false;
        };
        if let Some((seen, verified)) = self.lock().get(&key)
            && *seen == stamps
        {
            return *verified;
        }
        let verified = verify();
        let mut memo = self.lock();
        if memo.len() >= MAX_MEMOIZED_MODEL_SETS {
            memo.clear();
        }
        memo.insert(key, (stamps, verified));
        verified
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, (FileStamps, bool)>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Whether the built-in backend's models for `config` are cached and intact, checked on a
/// blocking thread (the cache lookup builds a Hugging Face client; hashing reads every file). The
/// built-in backend is registered with `PaddleOcrConfig::default()`; [`backend`] admits no other.
async fn models_ready(config: &OcrConfig) -> bool {
    let set = match crate::paddle_ocr::RequiredModelSet::resolve(&Default::default(), config) {
        Ok(set) => set,
        Err(error) => {
            tracing::debug!(%error, "PaddleOCR image fallback skipped: model set unresolved");
            return false;
        }
    };
    tokio::task::spawn_blocking(move || {
        let stamps = set.files().and_then(stamp_files);
        MODEL_SETS.ready(set.key(), stamps, || set.verified())
    })
    .await
    .unwrap_or(false)
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

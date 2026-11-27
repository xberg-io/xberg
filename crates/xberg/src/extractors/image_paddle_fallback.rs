//! Read a standalone camera photo again with the built-in PaddleOCR backend when the default
//! Tesseract read of it is sparse and unreliable (`OcrConfig::image_paddle_fallback`).
//!
//! Camera photos (receipts, scene text) defeat Tesseract at every PSM and threshold the image
//! route tries, while PaddleOCR's detector reads them. Confidences are never compared across
//! backends. Tesseract's is calibrated, so it decides which Tesseract words were read; PaddleOCR's
//! line score is its recognition text score, which stays at 0.92-1.00 on reads with no correct
//! word (measured on the benchmark photos), so the PaddleOCR read is judged by its text alone.
//!
//! Whether the fallback can run depends on the local PaddleOCR model cache, so the extraction
//! cache key of an eligible request carries the model files' identity ([`model_state`], read by
//! `core::extractor::cache_context`): a cold and a warm model cache key differently, and no
//! extraction is kept out of the cache because of this fallback.

use super::{has_robust_word_confidence_distribution, is_implicit_horizontal_tesseract, usable_word_confidences};
use crate::cancellation::{CancellationToken, check_cancelled};
use crate::core::config::{ExtractionConfig, OcrConfig, OcrQualityThresholds, VlmFallbackPolicy};
use crate::plugins::OcrBackend;
use crate::plugins::registry::OcrBackendRegistry;
use crate::types::internal::InternalDocument;
use crate::types::{ExtractedDocument, OcrElementLevel};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Condvar, LazyLock, Mutex, PoisonError};
use std::time::SystemTime;

const PADDLE_BACKEND: &str = "paddle-ocr";

/// Metadata key set on a read this fallback adopted, naming the backend that produced it.
pub(super) const FALLBACK_BACKEND_METADATA_KEY: &str = "ocr_fallback_backend";

/// Lossy camera formats. A PNG, BMP, GIF or TIFF is a screenshot, slide, form or scan; HEIC
/// reaches OCR already converted to PNG, so it does not qualify either.
const CAMERA_IMAGE_TYPES: &[&str] = &["image/jpeg", "image/jpg", "image/pjpeg", "image/webp"];

/// PaddleOCR script families the fallback was measured on (English receipts and scene text).
const MEASURED_SCRIPT_FAMILIES: &[&str] = &["english", "latin"];

/// Tesseract word confidence (0-1, calibrated) at or above which a Tesseract word counts as read.
/// The bar `image_color_threshold` uses for a confident word.
const CONFIDENT_TESSERACT_WORD: f64 = 0.70;

/// Most usable Tesseract words a photo-shaped read has. Receipts and scene photos read 13-40;
/// page scans read hundreds. The bound `image_color_threshold` uses for its retry.
const MAX_SPARSE_PRIMARY_WORDS: usize = 80;

/// Fewest plausible tokens a PaddleOCR read must hold to replace anything.
const MIN_PLAUSIBLE_TOKENS: usize = 4;

/// The plausible-token share a read needs to count as text rather than detector noise. The
/// PaddleOCR read must reach the Tesseract read's own share, capped at this floor.
const PLAUSIBLE_SHARE_FLOOR: f64 = 0.6;

/// Longest letter run counted as a plausible word.
const MAX_PLAUSIBLE_WORD_CHARS: usize = 20;

/// Model sets this process has checked, by cache root and selection. Bounded; cleared when full.
const MAX_MEMOIZED_MODEL_SETS: usize = 64;

static MODEL_SETS: LazyLock<ModelMemo> = LazyLock::new(ModelMemo::default);

/// The built-in PaddleOCR backend when `primary` is the built-in Tesseract backend, the request
/// is [`eligible`] and the image is a camera photo; a backend registered in place of either
/// keeps the request as is.
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
        && is_camera_photo(content, mime_type, &config.security_limits.clone().unwrap_or_default()))
    .then(|| registry.get("paddleocr").ok())
    .flatten()
}

/// Whether the request leaves the backend choice to Xberg. Every setting that shapes the OCR
/// read or its output is an explicit choice, even at its default value, as the benchmark adapter
/// and the default OCR routes already treat a present `tesseract_config`: `tesseract_config`,
/// `element_config`, `paddle_ocr_settings`/`paddle_ocr_config`, tessdata overrides, numeric
/// repair, backend tuning, a pipeline, VLM fallback, layout detection and document structure.
/// It also needs one Latin-script language PaddleOCR reads with one model.
///
/// An explicit `backend: "tesseract"` with no other setting cannot be told apart from the
/// default; `image_paddle_fallback: false` keeps Tesseract's read.
fn eligible(config: &ExtractionConfig, ocr: &OcrConfig, mime_type: &str) -> bool {
    #[cfg(feature = "layout-types")]
    let layout = config.layout.is_some();
    #[cfg(not(feature = "layout-types"))]
    let layout = false;
    ocr.image_paddle_fallback
        && !layout
        && !config.include_document_structure
        && is_implicit_horizontal_tesseract(ocr)
        && ocr.tesseract_config.is_none()
        && ocr.element_config.is_none()
        && ocr.paddle_ocr_settings.is_none()
        && ocr.paddle_ocr_config.is_none()
        && ocr.tessdata_path.is_none()
        && ocr.tessdata_bytes.is_none()
        && !ocr.numeric_repair
        && !ocr.has_tuning_backend_options()
        && ocr.pipeline.is_none()
        && ocr.vlm_fallback == VlmFallbackPolicy::Disabled
        && !mime_type.to_ascii_lowercase().contains("tiff")
        && reads_one_latin_language(ocr)
}

fn is_camera_type(mime_type: &str) -> bool {
    CAMERA_IMAGE_TYPES
        .iter()
        .any(|kind| mime_type.eq_ignore_ascii_case(kind))
}

/// A camera photo: a lossy camera format whose pixels match no standard page size at a scan
/// resolution (`resolve_known_source_dpi`, the signal the OCR route uses to spot page scans).
/// The size comes from the audited header probe, under the request's security limits.
fn is_camera_photo(content: &[u8], mime_type: &str, limits: &crate::extractors::security::SecurityLimits) -> bool {
    if !is_camera_type(mime_type) {
        return false;
    }
    let Ok((width, height, _)) =
        crate::extraction::image_decode::probe_standard_image_with_security_limits(content, limits)
    else {
        return false;
    };
    crate::extraction::image::resolve_known_source_dpi(None, content, width, height).is_none()
}

/// One language, in a Latin-script PaddleOCR family (the only ones measured), read the same by
/// both backends. Chinese, Japanese, Korean, Cyrillic, Arabic and the Indic scripts do not
/// qualify.
fn reads_one_latin_language(ocr: &OcrConfig) -> bool {
    let languages = ocr.effective_languages();
    matches!(
        languages.as_slice(),
        [language] if crate::paddle_ocr::map_language_code(language)
            .is_some_and(|code| MEASURED_SCRIPT_FAMILIES.contains(&crate::paddle_ocr::language_to_script_family(code)))
    ) && ocr.effective_tesseract_language() == languages
}

/// Whether `doc` was built from a read this fallback adopted, so a later Tesseract re-read of
/// the first pass does not replace it.
pub(super) fn read_by_paddle(doc: &InternalDocument) -> bool {
    doc.metadata
        .additional
        .get(FALLBACK_BACKEND_METADATA_KEY)
        .and_then(serde_json::Value::as_str)
        == Some(PADDLE_BACKEND)
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

/// A token that reads like a word or a number, judged the same on both backends' text: at
/// least two characters, and either all numeric (any script's digits) or all letters with a
/// vowel (any letter outside ASCII counts as one), at most [`MAX_PLAUSIBLE_WORD_CHARS`] long.
/// Misreads such as `x7Q`, `rn` or `CoE7` fail it, and so do mixed tokens and vowel-less
/// abbreviations (`1.00xITEMs`, `TTL`) on either side.
fn is_plausible_token(token: &str) -> bool {
    let length = token.chars().count();
    length >= 2
        && (token.chars().all(char::is_numeric)
            || (length <= MAX_PLAUSIBLE_WORD_CHARS
                && token.chars().all(char::is_alphabetic)
                && token
                    .chars()
                    .any(|character| !character.is_ascii() || "aeiouyAEIOUY".contains(character))))
}

fn plausible_tokens(text: &str) -> usize {
    tokens(text).filter(|token| is_plausible_token(token)).count()
}

fn plausible_share(plausible: usize, all: usize) -> f64 {
    if all == 0 { 0.0 } else { plausible as f64 / all as f64 }
}

fn tesseract_words(primary: &ExtractedDocument) -> impl Iterator<Item = &crate::types::OcrElement> {
    primary
        .ocr_elements
        .iter()
        .flatten()
        .filter(|element| element.level == OcrElementLevel::Word)
}

/// Plausible tokens among the words Tesseract read with confidence.
fn trusted_tesseract_tokens(primary: &ExtractedDocument) -> usize {
    tesseract_words(primary)
        .filter(|element| element.confidence.recognition >= CONFIDENT_TESSERACT_WORD)
        .map(|element| plausible_tokens(&element.text))
        .sum()
}

/// The plausible-token share of every Tesseract word, confident or not.
fn tesseract_plausible_share(primary: &ExtractedDocument) -> f64 {
    let (plausible, all) = tesseract_words(primary).fold((0, 0), |(plausible, all), element| {
        (
            plausible + plausible_tokens(&element.text),
            all + tokens(&element.text).count(),
        )
    });
    plausible_share(plausible, all)
}

/// The PaddleOCR read replaces the Tesseract read when its text is not empty and:
/// - it holds at least as many plausible tokens as Tesseract read with confidence, and at least
///   [`MIN_PLAUSIBLE_TOKENS`];
/// - its plausible share reaches the Tesseract read's own share, capped at
///   [`PLAUSIBLE_SHARE_FLOOR`];
/// - its text clears the quality gate a PDF OCR pipeline stage must clear
///   (`pipeline_min_quality`).
///
/// The read it replaces already failed Tesseract's own robustness check, so the PaddleOCR read
/// must not lose what Tesseract did read, not out-read it: on cord_receipt_04 PaddleOCR is exact
/// with 14 plausible tokens against Tesseract's 12 confident ones.
fn paddle_read_wins(
    primary: &ExtractedDocument,
    candidate: &ExtractedDocument,
    thresholds: &OcrQualityThresholds,
) -> bool {
    let plausible = plausible_tokens(&candidate.content);
    let share = plausible_share(plausible, tokens(&candidate.content).count());
    !candidate.content.trim().is_empty()
        && plausible >= MIN_PLAUSIBLE_TOKENS.max(trusted_tesseract_tokens(primary))
        && share >= tesseract_plausible_share(primary).min(PLAUSIBLE_SHARE_FLOOR)
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

enum ModelCheck {
    InFlight,
    Done(FileStamps, bool),
}

/// Verification results by model set, kept with the file stamps they were made on. A check in
/// flight makes concurrent callers for the same set wait for it instead of hashing again.
#[derive(Default)]
struct ModelMemo {
    checks: Mutex<HashMap<String, ModelCheck>>,
    finished: Condvar,
}

impl ModelMemo {
    /// Whether the model set is ready to read without a download. Runs on a blocking thread.
    /// `stamps` is `None` when a file is missing: that fails closed, and a set removed after it
    /// was verified never reaches the backend, which would download it. `verify` (hashing) runs
    /// again only when the files' sizes or modification times change, for a verified or a
    /// failed set alike.
    fn ready(&self, key: String, stamps: Option<FileStamps>, verify: impl FnOnce() -> bool) -> bool {
        let Some(stamps) = stamps else {
            return false;
        };
        let mut checks = self.lock();
        loop {
            let in_flight = match checks.get(&key) {
                Some(ModelCheck::Done(seen, verified)) if *seen == stamps => return *verified,
                Some(ModelCheck::InFlight) => true,
                _ => false,
            };
            if !in_flight {
                break;
            }
            checks = self.finished.wait(checks).unwrap_or_else(PoisonError::into_inner);
        }
        if checks.len() >= MAX_MEMOIZED_MODEL_SETS {
            checks.retain(|_, check| matches!(check, ModelCheck::InFlight));
        }
        checks.insert(key.clone(), ModelCheck::InFlight);
        drop(checks);
        let verified = std::panic::catch_unwind(std::panic::AssertUnwindSafe(verify)).unwrap_or(false);
        self.lock().insert(key, ModelCheck::Done(stamps, verified));
        self.finished.notify_all();
        verified
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, ModelCheck>> {
        self.checks.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Whether the built-in backend's models for `config` are cached and intact, checked on a
/// blocking thread (stats every file; hashing reads every file). The built-in backend is
/// registered with `PaddleOcrConfig::default()`; [`backend`] admits no other.
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

/// The OCR configuration whose PaddleOCR model state an extraction of `mime` under `config`
/// depends on, or `None` when the fallback cannot run for it. Decided from the request alone
/// (an unknown MIME type counts as a possible photo), before the image is read.
pub(crate) fn model_request(config: &ExtractionConfig, mime: Option<&str>) -> Option<OcrConfig> {
    if !config.use_cache || config.effective_disable_ocr() {
        return None;
    }
    let mime = mime
        .map(|mime| mime.split(';').next().unwrap_or_default().trim())
        .unwrap_or_default();
    let ocr = config.ocr_or_default().into_owned();
    ((mime.is_empty() || is_camera_type(mime)) && eligible(config, &ocr, mime)).then_some(ocr)
}

/// The identity of the PaddleOCR model files a fallback for `ocr` would read: the resolved cache
/// root and selection, and each file's size and modification time, or that one is missing.
/// Runs on a blocking thread; stats files, never reads or downloads them.
pub(crate) fn model_state(ocr: &OcrConfig) -> blake3::Hash {
    model_state_of(crate::paddle_ocr::RequiredModelSet::resolve(
        &Default::default(),
        &paddle_read_config(ocr),
    ))
}

fn model_state_of(set: crate::Result<crate::paddle_ocr::RequiredModelSet>) -> blake3::Hash {
    let mut hash = blake3::Hasher::new();
    hash.update(b"paddle-image-fallback-models-v1");
    let Ok(set) = set else {
        hash.update(b"unresolved");
        return hash.finalize();
    };
    let key = set.key();
    hash.update(&(key.len() as u64).to_le_bytes());
    hash.update(key.as_bytes());
    match set.files().and_then(stamp_files) {
        None => {
            hash.update(b"missing");
        }
        Some(stamps) => {
            for (path, length, modified) in stamps {
                let path = path.as_os_str().as_encoded_bytes();
                hash.update(&(path.len() as u64).to_le_bytes());
                hash.update(path);
                hash.update(&length.to_le_bytes());
                let nanos = modified
                    .and_then(|time| time.duration_since(SystemTime::UNIX_EPOCH).ok())
                    .map_or(0, |since| since.as_nanos());
                hash.update(&nanos.to_le_bytes());
            }
        }
    }
    hash.finalize()
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
        tracing::debug!(
            consulted = false,
            "PaddleOCR image fallback not consulted: the Tesseract read is reliable"
        );
        return Ok((primary, None));
    }
    let config = paddle_read_config(effective);
    if !ready(&config).await {
        tracing::debug!("PaddleOCR image fallback skipped: models are not cached locally");
        return Ok((primary, None));
    }
    let thresholds = original.effective_thresholds();
    let span = tracing::debug_span!("xberg::ocr::paddle_fallback", adopted = tracing::field::Empty);
    let (selected, adopted) = tracing::Instrument::instrument(
        read_if_better(
            cancel,
            async || paddle.process_image_owned(bytes, &config).await,
            primary,
            &thresholds,
        ),
        span.clone(),
    )
    .await?;
    span.record("adopted", adopted);
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

/// Marks `doc` as built from an adopted PaddleOCR read, for tests of [`read_by_paddle`].
#[cfg(test)]
pub(super) fn mark_read_by_paddle(doc: &mut InternalDocument) {
    doc.metadata.additional.insert(
        std::borrow::Cow::Borrowed(FALLBACK_BACKEND_METADATA_KEY),
        serde_json::Value::String(PADDLE_BACKEND.to_string()),
    );
}

#[cfg(test)]
#[path = "image_paddle_fallback_tests.rs"]
mod tests;

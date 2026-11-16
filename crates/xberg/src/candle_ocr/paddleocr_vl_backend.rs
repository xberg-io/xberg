//! PaddleOCR-VL backend plugin for the Xberg OCR pipeline.
//!
//! This module wraps the candle-based PaddleOCR-VL engine in the `OcrBackend`
//! trait, making it available to the extraction pipeline. The engine itself
//! (`xberg_candle_ocr::models::PaddleOcrVlEngine`) implements the PaddleOCR-VL 1.5
//! architecture (SigLIP vision encoder + ERNIE 4.5 text decoder); PaddleOCR-VL 1.6 is
//! a weights-only upgrade that keeps this architecture unchanged, so the same engine
//! loads either checkpoint — only the default `model_id` changed.
//!
//! # Engine pool design
//!
//! The pool key is `(task, DevicePreference)`. Engines are expensive to initialise
//! (~900 MB – 2 GB of safetensors weights depending on the checkpoint). The pool
//! ensures each `(task, device)` combination is loaded at most once per process.
//!
//! `PaddleOcrVlEngine::process_image` takes `&mut self` (the model maintains KV
//! cache state), so the pool stores engines wrapped in `parking_lot::Mutex` for
//! interior mutability.
//!
//! # Weight resolution
//!
//! `backend_options.model_path`, when present, always wins (offline / custom weights).
//! Otherwise the backend auto-downloads `backend_options.model_id` (default
//! `xberg-io/paddleocr-vl-1.6`, a checksum-pinned mirror of
//! `PaddlePaddle/PaddleOCR-VL-1.6`) through the internal `model_stager` module.

use async_trait::async_trait;
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};

use parking_lot::Mutex;

use crate::Result;
use crate::candle_ocr::config::{
    PaddleOcrVlBackendOptions, PaddleOcrVlTaskKind, parse_backend_options, validate_optional_non_empty,
};
use crate::core::config::OcrConfig;
use crate::engine_cache::EngineCache;
use crate::extraction::image_decode::{ImageDecodeBudget, PreparedCandleImageDecode};
use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
use crate::types::ExtractedDocument;
use xberg_candle_ocr::DType;
use xberg_candle_ocr::DevicePreference;
use xberg_candle_ocr::models::PaddleOcrVlEngine;
use xberg_candle_ocr::models::PaddleOcrVlTask;

/// Engine pool key: `(resolved_model_path, task, device_preference)`.
type PoolKey = (String, PaddleOcrVlTask, DevicePreference);
/// Pooled engine value: mutex-wrapped engine for interior mutability.
type PooledEngine = Arc<Mutex<PaddleOcrVlEngine>>;

/// Process-wide engine cache keyed by model path, task, and device preference.
///
/// `DevicePreference::Auto` keeps its own slot because it resolves to whatever
/// is available at runtime — collapsing it onto a concrete device would be wrong.
///
/// Engines are wrapped in `Mutex` because `PaddleOcrVlEngine::process_image`
/// takes `&mut self` (it manages an internal KV cache).
static ENGINE_POOL: LazyLock<EngineCache<PoolKey, Mutex<PaddleOcrVlEngine>>> = LazyLock::new(EngineCache::unbounded);

/// Return a cached engine for `(task, preference)`, initialising one on first use.
///
/// Delegates to [`EngineCache::get_or_try_init`], which stays locked across the
/// load, so a second caller for the same key waits for the first engine
/// instead of building another.
///
/// # Errors
///
/// Returns [`crate::XbergError::Ocr`] if device selection fails or the
/// engine cannot be initialised from the model directory.
fn get_or_init_engine(
    model_path: &str,
    task: PaddleOcrVlTask,
    preference: DevicePreference,
) -> crate::Result<PooledEngine> {
    let key: PoolKey = (model_path.to_string(), task, preference);

    ENGINE_POOL.get_or_try_init(key, || {
        let candle_device = preference.select().map_err(|e| crate::XbergError::Ocr {
            message: format!("Failed to select compute device: {e}"),
            source: Some(Box::new(e)),
        })?;

        tracing::info!(
            task = ?task,
            preference = ?preference,
            "Initialising PaddleOCR-VL engine (cold start)"
        );
        let new_engine = PaddleOcrVlEngine::new(model_path, task, candle_device, DType::F32).map_err(|e| {
            crate::XbergError::Ocr {
                message: format!("PaddleOCR-VL engine initialisation failed: {e}"),
                source: Some(Box::new(e)),
            }
        })?;
        Ok(Mutex::new(new_engine))
    })
}

/// Parse and check the PaddleOCR-VL `backend_options`.
///
/// Configuration validation and every page both call this, so an invalid option
/// fails before any page runs. ~keep
pub(crate) fn check_backend_options(value: Option<&serde_json::Value>) -> Result<PaddleOcrVlBackendOptions> {
    let options: PaddleOcrVlBackendOptions = parse_backend_options(value, "candle-paddleocr-vl")?;
    for (field, value) in [
        ("model_path", options.model_path.as_deref()),
        ("model_id", options.model_id.as_deref()),
        ("hf_revision", options.hf_revision.as_deref()),
        ("cache_dir", options.cache_dir.as_deref()),
    ] {
        validate_optional_non_empty(value, "candle-paddleocr-vl", field)?;
    }
    Ok(options)
}

/// Default HuggingFace repo id for PaddleOCR-VL weights: a checksum-pinned mirror of
/// `PaddlePaddle/PaddleOCR-VL-1.6`. Used when `backend_options` provides neither
/// `model_path` nor a custom `model_id`.
const DEFAULT_MODEL_ID: &str = "xberg-io/paddleocr-vl-1.6";

/// PaddleOCR-VL backend using candle transformers.
///
/// A vision-language model for comprehensive document parsing. Supports text
/// recognition, tables, formulas, and charts through a unified interface with
/// markdown output. The engine implements the PaddleOCR-VL 1.5 architecture (SigLIP
/// vision encoder + ERNIE 4.5 text decoder); PaddleOCR-VL 1.6 keeps that architecture
/// unchanged, so it loads through the same engine — only the default weights changed.
///
/// Supports 109+ languages through the PaddlePaddle pretrained models.
///
/// # Configuration
///
/// PaddleOCR-VL accepts backend options for task selection, device, and weight source:
/// ```json
/// {
///   "task": "ocr",
///   "device": "auto",
///   "model_id": "xberg-io/paddleocr-vl-1.6",
///   "model_path": "/path/to/paddleocr-vl-model"
/// }
/// ```
///
/// - `task` (string, optional): per-call override for `"ocr"`, `"table"`, `"formula"`, or `"chart"`.
///   When omitted, the task selected when constructing the backend is used.
/// - `device` (string): `"auto"`, `"cpu"`, `"cuda"`, `"metal"`
/// - `model_id` (string): HuggingFace repo id to auto-download weights from. Defaults to
///   `xberg-io/paddleocr-vl-1.6`, a checksum-pinned mirror of
///   `PaddlePaddle/PaddleOCR-VL-1.6`. Ignored when `model_path` is set.
/// - `model_path` (string, optional): path to a local model directory. When omitted,
///   the weights named by `model_id` are downloaded on first use into the standard
///   Hugging Face cache — no manual staging required.
/// - `hf_revision` (string, optional): immutable commit for a custom `model_id`.
///   The default model is pinned automatically.
/// - `cache_dir` (string, optional): explicit Hugging Face Hub cache root. When
///   omitted, `HF_HUB_CACHE`, `HUGGINGFACE_HUB_CACHE`, and `HF_HOME` are honored.
#[cfg_attr(alef, alef(skip))]
pub struct PaddleOcrVlBackend {
    task: PaddleOcrVlTask,
}

#[derive(Debug)]
struct PaddleOcrVlOptions {
    task: PaddleOcrVlTask,
    model_path: Option<String>,
    model_id: String,
    hf_revision: Option<String>,
    cache_dir: Option<PathBuf>,
    device: DevicePreference,
}

impl PaddleOcrVlBackend {
    /// Create a new PaddleOCR-VL backend with the specified task.
    pub fn new(task: PaddleOcrVlTask) -> Self {
        Self { task }
    }

    /// Create a PaddleOCR-VL backend with the default task (OCR).
    pub fn default_task() -> Self {
        Self::new(PaddleOcrVlTask::default())
    }

    /// Parse backend options to extract PaddleOCR-VL-specific configuration.
    ///
    /// Device selection is delegated to [`crate::candle_ocr::resolve_device_preference`]
    /// so the central `AccelerationConfig` is honoured.
    ///
    /// `model_id` defaults to [`DEFAULT_MODEL_ID`] and is only consulted when
    /// `model_path` is absent.
    fn parse_options(&self, config: &OcrConfig) -> Result<PaddleOcrVlOptions> {
        let options = check_backend_options(config.backend_options.as_ref())?;
        let task = match options.task {
            Some(PaddleOcrVlTaskKind::Ocr) => PaddleOcrVlTask::Ocr,
            Some(PaddleOcrVlTaskKind::Table) => PaddleOcrVlTask::Table,
            Some(PaddleOcrVlTaskKind::Formula) => PaddleOcrVlTask::Formula,
            Some(PaddleOcrVlTaskKind::Chart) => PaddleOcrVlTask::Chart,
            None => self.task,
        };
        Ok(PaddleOcrVlOptions {
            task,
            model_path: options.model_path,
            model_id: options.model_id.unwrap_or_else(|| DEFAULT_MODEL_ID.to_string()),
            hf_revision: options.hf_revision,
            cache_dir: options.cache_dir.map(PathBuf::from),
            device: super::resolve_device_preference(config, options.device),
        })
    }
}

impl Plugin for PaddleOcrVlBackend {
    fn name(&self) -> &str {
        "candle-paddleocr-vl"
    }

    fn version(&self) -> String {
        "0.1.0".to_string()
    }

    fn initialize(&self) -> Result<()> {
        tracing::debug!("Initializing PaddleOCR-VL backend: {} task", self.task);
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

impl PaddleOcrVlBackend {
    async fn process_image_inner(&self, image_bytes: Arc<Vec<u8>>, config: &OcrConfig) -> Result<ExtractedDocument> {
        let options = self.parse_options(config)?;
        // A non-OCR task (table/formula/chart) is expected to emit markup, so the
        // bare-LaTeX rule in `filter_implausible_lines` must not apply to it (GH#1676).
        // ~keep
        let plain_text_task = options.task == PaddleOcrVlTask::Ocr;

        if image_bytes.is_empty() {
            return Err(crate::XbergError::Validation {
                message: "Empty image data provided to PaddleOCR-VL".to_string(),
                source: None,
            });
        }

        let limits = config.security_limits.clone().unwrap_or_default();
        let decode_error = |error| match error {
            error @ crate::XbergError::Validation { .. } => error,
            error => crate::XbergError::Ocr {
                message: format!("PaddleOCR-VL image decode failed: {error}"),
                source: Some(Box::new(error)),
            },
        };
        let (content, image_bytes) = tokio::task::spawn_blocking(move || {
            let decode = PreparedCandleImageDecode::new(&image_bytes, &limits, true).map_err(decode_error)?;
            let model_path = match options.model_path {
                Some(p) => p,
                None => super::model_stager::ensure_paddleocr_vl_16(
                    &options.model_id,
                    options.hf_revision.as_deref(),
                    options.cache_dir.as_deref(),
                )
                .map(|dir| dir.to_string_lossy().into_owned())
                .map_err(|e| crate::XbergError::Ocr {
                    message: format!("PaddleOCR-VL weight download failed: {e}"),
                    source: None,
                })?,
            };
            let engine = get_or_init_engine(&model_path, options.task, options.device)?;

            let mut engine_guard = engine.lock();
            let img = decode.decode().map_err(decode_error)?.into_rgb8();
            let output = engine_guard
                .process_rgb_image(img)
                .map_err(|e| crate::XbergError::Ocr {
                    message: format!("PaddleOCR-VL inference failed: {e}"),
                    source: Some(Box::new(e)),
                })?;

            Ok::<_, crate::XbergError>((output.content, image_bytes))
        })
        .await
        .map_err(|e| crate::XbergError::Ocr {
            message: format!("PaddleOCR-VL task execution failed: {e}"),
            source: None,
        })??;

        Ok(super::ocr_result::build_ocr_document(
            content,
            Vec::new(),
            &image_bytes,
            config,
            super::ocr_result::OcrDocumentContext {
                mime_type: Cow::Borrowed("text/markdown"),
                backend_name: "candle-paddleocr-vl",
                plain_text_task,
            },
        ))
    }
}

/// Inherits the `RequiresUpright` default for `page_orientation_handling` — unmeasured, not validated (#657).
///
/// # Why this backend must not copy `crate::paddle_ocr`'s `page_rotation_degrees` handling (#734)
///
/// It was previously suspected that this backend simply *forgot* to read the
/// `backend_options["page_rotation_degrees"]` hint the way
/// The native PaddleOCR backend does. That comparison does not transfer,
/// for two independent reasons, and copying it here would introduce a real defect:
///
/// 1. **No block list to reorder.** `PaddleOcrBackend`'s fix
///    (`page_rotation_degrees_from_backend_options` -> `residual_rotation_for_reorder` ->
///    `reorder_blocks_for_page_rotation`) exists because its two-stage detector+recognizer
///    warps each detected text quad upright internally but leaves its *block list* in raw
///    raster `(y, x)` order, so the caller must reorder. This backend
///    (`xberg_candle_ocr::models::PaddleOcrVlEngine::process_image`) is a single-pass
///    vision-language model that autoregressively decodes one markdown string per page
///    (`CandleOcrOutput { content, .. }`, no bounding boxes) — there is no block list here to
///    reorder.
/// 2. **Reading the hint under `RequiresUpright` would double-rotate.** `page_rotation_degrees`
///    is injected into `backend_options` *unconditionally* by
///    `ocr_config_with_page_rotation_hint` whenever a page carries a `/Rotate` value —
///    regardless of the backend's declared `page_orientation_handling`
///    (`crate::extractors::pdf::ocr`). For a `RequiresUpright` backend specifically, the
///    pipeline *also* pre-rotates the raster to upright before calling `process_image`
///    (`upright_raster_for_backend`), so the raster this backend receives is already upright
///    even though the hint in its config still reports the page's raw, uncorrected rotation.
///    Rotating that already-upright raster again by the raw hint would be a strictly worse
///    defect than not reading it at all.
///
/// Reading `page_rotation_degrees` here would only become sound if this backend's declared
/// capability changed away from `RequiresUpright` (out of scope for #734 — that decision needs
/// its own measurement, see the module-level rotation-order benchmark notes) *and* the new
/// logic operated on raw, unrotated raster pixels rather than reordering blocks that do not
/// exist. Until then, not reading the hint is correct, not an omission.
#[async_trait]
impl OcrBackend for PaddleOcrVlBackend {
    /// Process an image using the PaddleOCR-VL engine.
    ///
    /// # Errors
    ///
    /// Returns [`crate::XbergError::Validation`] if the image is empty or exceeds the decode budget.
    /// Returns [`crate::XbergError::Ocr`] if weight download, device selection,
    /// engine initialisation, or inference fails.
    async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> Result<ExtractedDocument> {
        let limits = config.security_limits.clone().unwrap_or_default();
        ImageDecodeBudget::from_security_limits(&limits).validate(
            1,
            1,
            u64::try_from(image_bytes.len()).unwrap_or(u64::MAX),
        )?;
        self.process_image_owned(Arc::new(image_bytes.to_vec()), config).await
    }

    async fn process_image_owned(&self, image_bytes: Arc<Vec<u8>>, config: &OcrConfig) -> Result<ExtractedDocument> {
        self.process_image_inner(image_bytes, config).await
    }

    /// Process an image file using the PaddleOCR-VL engine.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or if inference fails.
    async fn process_image_file(&self, path: &Path, config: &OcrConfig) -> Result<ExtractedDocument> {
        let limits = config.security_limits.clone().unwrap_or_default();
        let bytes = super::read_image_file(path, &limits).await?;
        self.process_image_owned(Arc::new(bytes), config).await
    }

    fn supports_language(&self, _lang: &str) -> bool {
        true
    }

    fn supported_languages(&self) -> Vec<String> {
        vec![
            "eng", "en", "zho", "zh", "jpn", "ja", "kor", "ko", "fra", "fr", "deu", "de", "spa", "es", "ita", "it",
            "por", "pt", "rus", "ru", "ara", "ar", "hin", "hi", "tha", "th", "vie", "vi",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Candle
    }

    fn emits_structured_markdown(&self) -> bool {
        true
    }

    /// PaddleOCR-VL reports no page-level confidence.
    fn confidence_semantics(&self) -> crate::plugins::ConfidenceSemantics {
        crate::plugins::ConfidenceSemantics::None
    }

    // Rotation handling has not been measured for this backend; it stays on the trait's
    // `RequiresUpright` default.
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_default_options(config: &OcrConfig) -> Result<PaddleOcrVlOptions> {
        PaddleOcrVlBackend::default_task().parse_options(config)
    }

    #[tokio::test]
    async fn configured_image_budget_rejects_decode_before_loading_model() {
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(64, 64)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let limit = 4 * 1024;
        assert!(png.get_ref().len() < limit);
        let temp = tempfile::tempdir().unwrap();
        let model_path = temp.path().join("missing-model").to_string_lossy().into_owned();
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({ "model_path": model_path, "device": "cpu" })),
            security_limits: Some(crate::extractors::security::SecurityLimits {
                max_content_size: limit,
                ..Default::default()
            }),
            ..Default::default()
        };

        let backend = PaddleOcrVlBackend::default_task();
        let error = backend
            .process_image(png.get_ref(), &config)
            .await
            .expect_err("decoded raster exceeds the caller's budget");
        assert!(
            matches!(&error, crate::XbergError::Validation { message, .. }
            if message.contains("security_limits.max_content_size")),
            "{error}"
        );

        let admitted_config = OcrConfig {
            security_limits: Some(crate::extractors::security::SecurityLimits {
                max_content_size: 1 << 20,
                ..Default::default()
            }),
            ..config
        };
        let error = backend
            .process_image(png.get_ref(), &admitted_config)
            .await
            .expect_err("the admitted raster reaches model initialization");
        assert!(
            matches!(&error, crate::XbergError::Ocr { message, .. }
            if message.contains("engine initialisation failed")),
            "{error}"
        );
    }

    #[tokio::test]
    async fn image_budget_accounts_for_the_full_decode_peak_before_model_loading() {
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_luma8(64, 64)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let backend = PaddleOcrVlBackend::default_task();
        let temp = tempfile::tempdir().unwrap();
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({
                "model_path": temp.path().join("missing-model"),
                "device": "cpu",
            })),
            security_limits: Some(crate::extractors::security::SecurityLimits {
                max_content_size: 15 * 1024,
                ..Default::default()
            }),
            ..Default::default()
        };
        let error = backend.process_image(png.get_ref(), &config).await.unwrap_err();
        assert!(
            matches!(&error, crate::XbergError::Validation { message, .. }
            if message.contains("security_limits.max_content_size")),
            "{error}"
        );
    }

    #[tokio::test]
    async fn oversized_image_file_is_rejected_by_encoded_size() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("image.png");
        let limit = 1024;
        std::fs::File::create(&path)
            .unwrap()
            .set_len((limit + 1) as u64)
            .unwrap();
        let config = OcrConfig {
            security_limits: Some(crate::extractors::security::SecurityLimits {
                max_content_size: limit,
                ..Default::default()
            }),
            ..Default::default()
        };
        let error = PaddleOcrVlBackend::default_task()
            .process_image_file(&path, &config)
            .await
            .unwrap_err();
        assert!(
            matches!(&error, crate::XbergError::Validation { message, .. }
            if message.contains("encoded image file") && message.contains("security_limits.max_content_size")),
            "{error}"
        );
    }

    #[tokio::test]
    async fn jpeg_decoder_copy_is_charged_before_loading_model() {
        let mut jpeg = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(1, 1)
            .write_to(&mut jpeg, image::ImageFormat::Jpeg)
            .unwrap();
        let jpeg = Arc::new(jpeg.into_inner());
        let temp = tempfile::tempdir().unwrap();
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({
                "model_path": temp.path().join("missing-model"),
                "device": "cpu",
            })),
            security_limits: Some(crate::extractors::security::SecurityLimits {
                max_content_size: jpeg.len() + 3,
                ..Default::default()
            }),
            ..Default::default()
        };
        let error = PaddleOcrVlBackend::default_task()
            .process_image_owned(jpeg, &config)
            .await
            .unwrap_err();
        assert!(
            matches!(&error, crate::XbergError::Validation { message, .. }
            if message.contains("security_limits.max_content_size")),
            "{error}"
        );
    }

    #[test]
    fn test_paddleocr_vl_backend_creation() {
        let backend = PaddleOcrVlBackend::default_task();
        assert_eq!(backend.name(), "candle-paddleocr-vl");
        assert_eq!(backend.backend_type(), OcrBackendType::Candle);
    }

    #[test]
    fn test_paddleocr_vl_emits_structured_markdown() {
        let backend = PaddleOcrVlBackend::default_task();
        assert!(backend.emits_structured_markdown());
    }

    #[test]
    fn test_paddleocr_vl_language_support() {
        let backend = PaddleOcrVlBackend::default_task();
        assert!(backend.supports_language("eng"));
        assert!(backend.supports_language("zho"));
        assert!(backend.supports_language("jpn"));
        assert!(backend.supports_language("fra"));
        assert!(backend.supports_language("unknown"));
    }

    #[test]
    fn test_paddleocr_vl_supported_languages() {
        let backend = PaddleOcrVlBackend::default_task();
        let langs = backend.supported_languages();
        assert!(langs.contains(&"eng".to_string()));
        assert!(langs.contains(&"zho".to_string()));
        assert!(langs.contains(&"jpn".to_string()));
    }

    #[test]
    fn test_parse_options_defaults() {
        let config = OcrConfig::default();
        let options = parse_default_options(&config).unwrap();
        assert_eq!(options.task, PaddleOcrVlTask::Ocr);
        assert!(options.model_path.is_none());
        assert_eq!(options.model_id, DEFAULT_MODEL_ID);
        assert!(options.hf_revision.is_none());
        assert!(options.cache_dir.is_none());
        assert_eq!(options.device, DevicePreference::Auto);
    }

    #[test]
    fn test_parse_options_custom_task() {
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({
                "task": "table"
            })),
            ..Default::default()
        };
        let options = parse_default_options(&config).unwrap();
        assert_eq!(options.task, PaddleOcrVlTask::Table);
    }

    #[test]
    fn should_use_constructor_task_when_backend_options_omit_task() {
        let backend = PaddleOcrVlBackend::new(PaddleOcrVlTask::Table);

        let options = backend.parse_options(&OcrConfig::default()).unwrap();

        assert_eq!(options.task, backend.task);
    }

    #[test]
    fn should_prefer_explicit_task_over_constructor_task() {
        let backend = PaddleOcrVlBackend::new(PaddleOcrVlTask::Chart);
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({
                "task": "formula"
            })),
            ..Default::default()
        };

        let options = backend.parse_options(&config).unwrap();

        assert_eq!(backend.task, PaddleOcrVlTask::Chart);
        assert_eq!(options.task, PaddleOcrVlTask::Formula);
    }

    #[test]
    fn test_parse_options_custom_device() {
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({
                "device": "cpu"
            })),
            ..Default::default()
        };
        let options = parse_default_options(&config).unwrap();
        assert_eq!(options.device, DevicePreference::Cpu);
    }

    #[test]
    fn test_parse_options_model_path() {
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({
                "model_path": "/models/paddleocr-vl"
            })),
            ..Default::default()
        };
        let options = parse_default_options(&config).unwrap();
        assert_eq!(options.model_path.as_deref(), Some("/models/paddleocr-vl"));
    }

    #[test]
    fn test_parse_options_custom_model_id() {
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({
                "model_id": "some-org/custom-paddleocr-vl"
            })),
            ..Default::default()
        };
        let options = parse_default_options(&config).unwrap();
        assert!(options.model_path.is_none());
        assert_eq!(options.model_id, "some-org/custom-paddleocr-vl");
    }

    #[test]
    fn test_parse_options_hf_cache_and_revision() {
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({
                "hf_revision": "0123456789abcdef",
                "cache_dir": "/tmp/hf-hub"
            })),
            ..Default::default()
        };
        let options = parse_default_options(&config).unwrap();
        assert_eq!(options.hf_revision.as_deref(), Some("0123456789abcdef"));
        assert_eq!(options.cache_dir.as_deref(), Some(Path::new("/tmp/hf-hub")));
    }

    #[test]
    fn test_parse_options_non_object_json_returns_contextual_error() {
        let config = OcrConfig {
            backend_options: Some(serde_json::json!(false)),
            ..Default::default()
        };
        let error = parse_default_options(&config).unwrap_err().to_string();
        assert!(error.contains("candle-paddleocr-vl backend_options"));
    }

    #[test]
    fn test_parse_options_empty_object_returns_defaults() {
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({})),
            ..Default::default()
        };
        let options = parse_default_options(&config).unwrap();
        assert_eq!(options.task, PaddleOcrVlTask::Ocr);
        assert!(options.model_path.is_none());
        assert_eq!(options.model_id, DEFAULT_MODEL_ID);
        assert!(options.hf_revision.is_none());
        assert!(options.cache_dir.is_none());
        assert_eq!(options.device, DevicePreference::Auto);
    }

    #[test]
    fn test_initialize_and_shutdown() {
        let backend = PaddleOcrVlBackend::default_task();
        assert!(backend.initialize().is_ok());
        assert!(backend.shutdown().is_ok());
    }

    /// Regression tripwire for #734: this backend has no block list to reorder (see the
    /// `OcrBackend` impl's doc comment), so it must stay on the inherited `RequiresUpright`
    /// default rather than declaring `SelfCorrecting`/`RecognisesRotatedText` without also
    /// adding raster-level rotation handling. This test passes today — there is no bug in the
    /// current code, only a hazard in copying `crate::paddle_ocr::backend`'s block-reorder fix
    /// here — and exists to fail the moment someone flips the declared capability without also
    /// working out how a single-pass VLM with no bounding boxes is supposed to honour
    /// `page_rotation_degrees`.
    #[test]
    fn should_stay_on_requires_upright_until_rotation_handling_is_measured() {
        let backend = PaddleOcrVlBackend::default_task();
        let dynamic: &dyn OcrBackend = &backend;
        assert_eq!(
            dynamic.page_orientation_handling(),
            crate::plugins::PageOrientationHandling::RequiresUpright
        );
    }
}

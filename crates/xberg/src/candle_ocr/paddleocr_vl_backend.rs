//! PaddleOCR-VL backend plugin for the Xberg OCR pipeline.
//!
//! This module wraps the candle-based PaddleOCR-VL 1.5 engine in the `OcrBackend`
//! trait, making it available to the extraction pipeline.
//!
//! # Engine pool design
//!
//! The pool key is `(task, DevicePreference)`. Engines are expensive to initialise
//! (~900 MB of safetensors weights). The pool ensures each `(task, device)`
//! combination is loaded at most once per process.
//!
//! `PaddleOcrVlEngine::process_image` takes `&mut self` (the model maintains KV
//! cache state), so the pool stores engines wrapped in `parking_lot::Mutex` for
//! interior mutability.

use async_trait::async_trait;
use std::borrow::Cow;
use std::path::Path;
use std::sync::{Arc, LazyLock};

use ahash::AHashMap;
use parking_lot::{Mutex, RwLock};

use crate::Result;
use crate::core::config::OcrConfig;
use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
use crate::types::ExtractionResult;
use xberg_candle_ocr::DType;
use xberg_candle_ocr::DevicePreference;
use xberg_candle_ocr::models::PaddleOcrVlEngine;
use xberg_candle_ocr::models::PaddleOcrVlTask;

/// Engine pool key: `(task, device_preference)`.
type PoolKey = (PaddleOcrVlTask, DevicePreference);
/// Pooled engine value: mutex-wrapped engine for interior mutability.
type PooledEngine = Arc<Mutex<PaddleOcrVlEngine>>;

/// Process-wide engine pool keyed by `(task, DevicePreference)`.
///
/// `DevicePreference::Auto` keeps its own slot because it resolves to whatever
/// is available at runtime — collapsing it onto a concrete device would be wrong.
///
/// Engines are wrapped in `Mutex` because `PaddleOcrVlEngine::process_image`
/// takes `&mut self` (it manages an internal KV cache).
static ENGINE_POOL: LazyLock<RwLock<AHashMap<PoolKey, PooledEngine>>> = LazyLock::new(|| RwLock::new(AHashMap::new()));

/// Return a cached engine for `(task, preference)`, initialising one on first use.
///
/// Uses a read → miss → write → double-check pattern so that two racing callers
/// do not both pay the initialisation cost.
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
    let key: PoolKey = (task, preference);

    // Fast path: engine already in pool.
    {
        let pool = ENGINE_POOL.read();
        if let Some(engine) = pool.get(&key) {
            return Ok(Arc::clone(engine));
        }
    }

    // Slow path: select the device and build the engine, then insert under write lock.
    let candle_device = preference.select().map_err(|e| crate::XbergError::Ocr {
        message: format!("Failed to select compute device: {e}"),
        source: Some(Box::new(e)),
    })?;

    tracing::info!(
        task = ?task,
        preference = ?preference,
        "Initialising PaddleOCR-VL engine (cold start)"
    );
    let new_engine =
        PaddleOcrVlEngine::new(model_path, task, candle_device, DType::F32).map_err(|e| crate::XbergError::Ocr {
            message: format!("PaddleOCR-VL engine initialisation failed: {e}"),
            source: Some(Box::new(e)),
        })?;
    let new_engine = Arc::new(Mutex::new(new_engine));

    let mut pool = ENGINE_POOL.write();
    // Double-check: another thread may have inserted while we were building.
    if let Some(existing) = pool.get(&key) {
        return Ok(Arc::clone(existing));
    }
    pool.insert(key, Arc::clone(&new_engine));
    Ok(new_engine)
}

/// PaddleOCR-VL backend using candle transformers (PaddleOCR-VL 1.5).
///
/// A vision-language model for comprehensive document parsing. Supports text
/// recognition, tables, formulas, and charts through a unified interface with
/// markdown output.
///
/// Supports 109+ languages through the PaddlePaddle pretrained models.
///
/// # Configuration
///
/// PaddleOCR-VL accepts backend options for task selection, device, and model path:
/// ```json
/// {
///   "task": "ocr",
///   "device": "auto",
///   "model_path": "/path/to/paddleocr-vl-model"
/// }
/// ```
///
/// - `task` (string): `"ocr"` (default), `"table"`, `"formula"`, `"chart"`
/// - `device` (string): `"auto"`, `"cpu"`, `"cuda"`, `"metal"`
/// - `model_path` (string): path to the local model directory (required for inference)
#[cfg_attr(alef, alef(skip))]
pub struct PaddleOcrVlBackend {
    task: PaddleOcrVlTask,
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
    /// Returns `(task, model_path, device_preference)`.
    fn parse_options(config: &OcrConfig) -> (PaddleOcrVlTask, Option<String>, DevicePreference) {
        let mut task = PaddleOcrVlTask::default();
        let mut model_path: Option<String> = None;

        if let Some(opts) = &config.backend_options {
            if let Some(t) = opts.get("task").and_then(|v| v.as_str()) {
                task = match t {
                    "table" => PaddleOcrVlTask::Table,
                    "formula" => PaddleOcrVlTask::Formula,
                    "chart" => PaddleOcrVlTask::Chart,
                    _ => PaddleOcrVlTask::Ocr,
                };
            }
            if let Some(p) = opts.get("model_path").and_then(|v| v.as_str()) {
                model_path = Some(p.to_string());
            }
        }

        let device = super::resolve_device_preference(config);
        (task, model_path, device)
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

#[async_trait]
impl OcrBackend for PaddleOcrVlBackend {
    /// Process an image using the PaddleOCR-VL engine.
    ///
    /// # Errors
    ///
    /// Returns [`crate::XbergError::Validation`] if `image_bytes` is empty
    /// or `model_path` is not provided in `backend_options`.
    /// Returns [`crate::XbergError::Ocr`] if device selection, engine
    /// initialisation, or inference fails.
    async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> Result<ExtractionResult> {
        let (task, model_path, device) = Self::parse_options(config);

        // Validate image data
        if image_bytes.is_empty() {
            return Err(crate::XbergError::Validation {
                message: "Empty image data provided to PaddleOCR-VL".to_string(),
                source: None,
            });
        }

        let model_path = model_path.ok_or_else(|| crate::XbergError::Validation {
            message: "PaddleOCR-VL requires `model_path` in backend_options pointing to the local model directory"
                .to_string(),
            source: None,
        })?;

        // Clone image bytes for the blocking task.
        let image_bytes = image_bytes.to_vec();

        // Run inference in a blocking task to avoid blocking the async runtime.
        let content = tokio::task::spawn_blocking(move || {
            let engine = get_or_init_engine(&model_path, task, device)?;

            // Lock the engine for mutation (KV cache is managed internally).
            let mut engine_guard = engine.lock();
            let output = engine_guard
                .process_image(&image_bytes)
                .map_err(|e| crate::XbergError::Ocr {
                    message: format!("PaddleOCR-VL inference failed: {e}"),
                    source: Some(Box::new(e)),
                })?;

            Ok::<String, crate::XbergError>(output.content)
        })
        .await
        .map_err(|e| crate::XbergError::Ocr {
            message: format!("PaddleOCR-VL task execution failed: {e}"),
            source: None,
        })??;

        Ok(ExtractionResult {
            content,
            mime_type: Cow::Borrowed("text/markdown"),
            ..Default::default()
        })
    }

    /// Process an image file using the PaddleOCR-VL engine.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or if inference fails.
    async fn process_image_file(&self, path: &Path, config: &OcrConfig) -> Result<ExtractionResult> {
        let bytes = crate::core::io::read_file_async(path).await?;
        self.process_image(&bytes, config).await
    }

    fn supports_language(&self, _lang: &str) -> bool {
        // PaddleOCR-VL supports 109+ languages as per the official model documentation.
        // Accept all language codes.
        true
    }

    fn supported_languages(&self) -> Vec<String> {
        // Major language codes supported by PaddleOCR-VL
        vec![
            "eng", "en", // English
            "zho", "zh", // Chinese (simplified and traditional)
            "jpn", "ja", // Japanese
            "kor", "ko", // Korean
            "fra", "fr", // French
            "deu", "de", // German
            "spa", "es", // Spanish
            "ita", "it", // Italian
            "por", "pt", // Portuguese
            "rus", "ru", // Russian
            "ara", "ar", // Arabic
            "hin", "hi", // Hindi
            "tha", "th", // Thai
            "vie", "vi", // Vietnamese
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Candle
    }

    fn emits_structured_markdown(&self) -> bool {
        // PaddleOCR-VL emits markdown output directly from the VLM,
        // so the extraction pipeline should skip layout reconstruction stages.
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let (task, model_path, device) = PaddleOcrVlBackend::parse_options(&config);
        assert_eq!(task, PaddleOcrVlTask::Ocr);
        assert!(model_path.is_none());
        assert_eq!(device, DevicePreference::Auto);
    }

    #[test]
    fn test_parse_options_custom_task() {
        let mut config = OcrConfig::default();
        config.backend_options = Some(serde_json::json!({
            "task": "table"
        }));
        let (task, _model_path, _device) = PaddleOcrVlBackend::parse_options(&config);
        assert_eq!(task, PaddleOcrVlTask::Table);
    }

    #[test]
    fn test_parse_options_custom_device() {
        let mut config = OcrConfig::default();
        config.backend_options = Some(serde_json::json!({
            "device": "cpu"
        }));
        let (_task, _model_path, device) = PaddleOcrVlBackend::parse_options(&config);
        assert_eq!(device, DevicePreference::Cpu);
    }

    #[test]
    fn test_parse_options_model_path() {
        let mut config = OcrConfig::default();
        config.backend_options = Some(serde_json::json!({
            "model_path": "/models/paddleocr-vl"
        }));
        let (_task, model_path, _device) = PaddleOcrVlBackend::parse_options(&config);
        assert_eq!(model_path.as_deref(), Some("/models/paddleocr-vl"));
    }

    #[test]
    fn test_initialize_and_shutdown() {
        let backend = PaddleOcrVlBackend::default_task();
        assert!(backend.initialize().is_ok());
        assert!(backend.shutdown().is_ok());
    }
}

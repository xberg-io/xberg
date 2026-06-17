//! Hunyuan-OCR backend plugin for the Kreuzberg OCR pipeline.
//!
//! This module wraps the candle-based Hunyuan-OCR engine in the `OcrBackend` trait,
//! making it available to the extraction pipeline.
//!
//! # Engine pool design
//!
//! The pool key is `(DevicePreference, DType)`. All Hunyuan-OCR requests use the
//! same model weights regardless of the per-call task, so a single engine instance
//! per `(device, dtype)` pair handles all calls.
//!
//! # Status
//!
//! The Hunyuan-OCR model weights are loaded via `HunyuanOCREngine::init` which
//! requires a local model directory. The generation loop integration (token decoding)
//! is not yet wired; calls will return a descriptive error until Phase 6 completes.

use async_trait::async_trait;
use std::borrow::Cow;
use std::path::Path;
use std::sync::{Arc, LazyLock};

use ahash::AHashMap;
use parking_lot::RwLock;

use crate::Result;
use crate::core::config::OcrConfig;
use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
use crate::types::ExtractionResult;
use kreuzberg_candle_ocr::DType;
use kreuzberg_candle_ocr::DevicePreference;
use kreuzberg_candle_ocr::models::HunyuanOCREngine;

/// Engine pool key: `(device_preference, dtype)`.
type PoolKey = (DevicePreference, DType);
/// Pooled engine value: shared reference with interior mutability for the engine.
type PooledEngine = Arc<parking_lot::Mutex<HunyuanOCREngine>>;

/// Process-wide engine pool keyed by `(DevicePreference, DType)`.
///
/// A single engine instance handles all tasks via the generation loop, so the
/// pool key does not include the task. Two callers requesting the same
/// `(device, dtype)` share one engine and avoid loading weights twice.
static ENGINE_POOL: LazyLock<RwLock<AHashMap<PoolKey, PooledEngine>>> =
    LazyLock::new(|| RwLock::new(AHashMap::new()));

/// Return a cached engine for `(preference, dtype)`, initialising one on first use.
///
/// Uses a read → miss → write → double-check pattern so two racing callers do
/// not both pay the initialisation cost. The returned engine is wrapped in a
/// `Mutex` for interior mutability.
///
/// # Errors
///
/// Returns [`crate::KreuzbergError::Ocr`] if device selection fails or the
/// engine cannot be initialised from the model directory.
fn get_or_init_engine(
    model_path: &str,
    preference: DevicePreference,
    dtype: DType,
) -> crate::Result<PooledEngine> {
    let key: PoolKey = (preference, dtype);

    // Fast path: engine already in pool.
    {
        let pool = ENGINE_POOL.read();
        if let Some(engine) = pool.get(&key) {
            return Ok(Arc::clone(engine));
        }
    }

    // Slow path: select the device and build the engine, then insert under write lock.
    let candle_device = preference.select().map_err(|e| crate::KreuzbergError::Ocr {
        message: format!("Failed to select compute device: {e}"),
        source: Some(Box::new(e)),
    })?;

    tracing::info!(
        preference = ?preference,
        ?dtype,
        "Initialising Hunyuan-OCR engine (cold start)"
    );

    let new_engine =
        HunyuanOCREngine::init(model_path, Some(&candle_device), Some(dtype)).map_err(|e| {
            crate::KreuzbergError::Ocr {
                message: format!("Hunyuan-OCR engine initialisation failed: {e}"),
                source: Some(Box::new(e)),
            }
        })?;
    let new_engine = Arc::new(parking_lot::Mutex::new(new_engine));

    let mut pool = ENGINE_POOL.write();
    // Double-check: another thread may have inserted while we were building.
    if let Some(existing) = pool.get(&key) {
        return Ok(Arc::clone(existing));
    }
    pool.insert(key, Arc::clone(&new_engine));
    Ok(new_engine)
}

/// Hunyuan-OCR backend using candle transformers.
///
/// A vision-language model for comprehensive document parsing. Supports text
/// recognition, tables, formulas, and charts through a unified interface with
/// markdown output.
///
/// # Configuration
///
/// Hunyuan-OCR accepts backend options for device selection and model path:
/// ```json
/// {
///   "device": "auto",
///   "model_path": "/path/to/hunyuan-ocr-model"
/// }
/// ```
///
/// - `device` (string): `"auto"` (default), `"cpu"`, `"cuda"`, `"metal"`
/// - `model_path` (string): path to the local model directory (required)
#[cfg_attr(alef, alef(skip))]
pub struct HunyuanOcrBackend {
    dtype: DType,
}

impl HunyuanOcrBackend {
    /// Create a new Hunyuan-OCR backend.
    ///
    /// `dtype` defaults to `F32`.
    pub fn new() -> Self {
        Self { dtype: DType::F32 }
    }

    /// Override the floating-point precision used by the candle engine.
    pub fn with_dtype(mut self, dtype: DType) -> Self {
        self.dtype = dtype;
        self
    }

    /// Parse backend options to extract Hunyuan-OCR-specific configuration.
    ///
    /// Device selection is delegated to [`crate::candle_ocr::resolve_device_preference`]
    /// so the central `AccelerationConfig` is honoured.
    ///
    /// Returns `(model_path, device_preference)`.
    fn parse_options(config: &OcrConfig) -> (Option<String>, DevicePreference) {
        let mut model_path: Option<String> = None;

        if let Some(opts) = &config.backend_options
            && let Some(p) = opts.get("model_path").and_then(|v| v.as_str())
        {
            model_path = Some(p.to_string());
        }

        let device = super::resolve_device_preference(config);
        (model_path, device)
    }
}

impl Default for HunyuanOcrBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for HunyuanOcrBackend {
    fn name(&self) -> &str {
        "candle-hunyuan-ocr"
    }

    fn version(&self) -> String {
        "0.1.0".to_string()
    }

    fn initialize(&self) -> Result<()> {
        tracing::debug!("Initializing Hunyuan-OCR backend");
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl OcrBackend for HunyuanOcrBackend {
    /// Process an image using the Hunyuan-OCR engine.
    ///
    /// # Errors
    ///
    /// Returns [`crate::KreuzbergError::Validation`] if `image_bytes` is empty
    /// or `model_path` is not provided in `backend_options`.
    /// Returns [`crate::KreuzbergError::Ocr`] if device selection, engine
    /// initialisation, or inference fails.
    async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> Result<ExtractionResult> {
        let (model_path, device) = Self::parse_options(config);

        // Validate image data
        if image_bytes.is_empty() {
            return Err(crate::KreuzbergError::Validation {
                message: "Empty image data provided to Hunyuan-OCR".to_string(),
                source: None,
            });
        }

        let model_path = model_path.ok_or_else(|| crate::KreuzbergError::Validation {
            message: "Hunyuan-OCR requires `model_path` in backend_options pointing to the local model directory".to_string(),
            source: None,
        })?;

        let image_bytes = image_bytes.to_vec();
        let dtype = self.dtype;

        // Run inference in a blocking task to avoid blocking the async runtime.
        let content = tokio::task::spawn_blocking(move || {
            let engine = get_or_init_engine(&model_path, device, dtype)?;

            // Run the Hunyuan-OCR inference pipeline: image preprocessing → vision encoding →
            // autoregressive text generation → token decoding.
            let mut engine_lock = engine.lock();
            engine_lock
                .process_image(&image_bytes)
                .map(|output| output.content)
                .map_err(|e| crate::KreuzbergError::Ocr {
                    message: format!("Hunyuan-OCR inference failed: {}", e),
                    source: Some(Box::new(e)),
                })
        })
        .await
        .map_err(|e| crate::KreuzbergError::Ocr {
            message: format!("Hunyuan-OCR task execution failed: {e}"),
            source: None,
        })??;

        Ok(ExtractionResult {
            content,
            mime_type: Cow::Borrowed("text/markdown"),
            ..Default::default()
        })
    }

    /// Process an image file using the Hunyuan-OCR engine.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or if inference fails.
    async fn process_image_file(&self, path: &Path, config: &OcrConfig) -> Result<ExtractionResult> {
        let bytes = crate::core::io::read_file_async(path).await?;
        self.process_image(&bytes, config).await
    }

    fn supports_language(&self, _lang: &str) -> bool {
        // Hunyuan-OCR is trained on multilingual data and supports a broad range
        // of CJK and Latin scripts. Accept all language codes.
        true
    }

    fn supported_languages(&self) -> Vec<String> {
        // Major language codes supported by Hunyuan-OCR
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
        // Hunyuan-OCR emits markdown output directly from the VLM,
        // so the extraction pipeline should skip layout reconstruction stages.
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hunyuan_ocr_backend_creation() {
        let backend = HunyuanOcrBackend::new();
        assert_eq!(backend.name(), "candle-hunyuan-ocr");
        assert_eq!(backend.backend_type(), OcrBackendType::Candle);
    }

    #[test]
    fn test_hunyuan_ocr_emits_structured_markdown() {
        let backend = HunyuanOcrBackend::new();
        assert!(backend.emits_structured_markdown());
    }

    #[test]
    fn test_hunyuan_ocr_language_support() {
        let backend = HunyuanOcrBackend::new();
        assert!(backend.supports_language("eng"));
        assert!(backend.supports_language("zho"));
        assert!(backend.supports_language("jpn"));
        assert!(backend.supports_language("unknown"));
    }

    #[test]
    fn test_hunyuan_ocr_supported_languages() {
        let backend = HunyuanOcrBackend::new();
        let langs = backend.supported_languages();
        assert!(langs.contains(&"eng".to_string()));
        assert!(langs.contains(&"zho".to_string()));
        assert!(langs.contains(&"fra".to_string()));
    }

    #[test]
    fn test_parse_options_defaults() {
        let config = OcrConfig::default();
        let (model_path, device) = HunyuanOcrBackend::parse_options(&config);
        assert!(model_path.is_none());
        assert_eq!(device, DevicePreference::Auto);
    }

    #[test]
    fn test_parse_options_model_path() {
        let mut config = OcrConfig::default();
        config.backend_options = Some(serde_json::json!({"model_path": "/models/hunyuan"}));
        let (model_path, _device) = HunyuanOcrBackend::parse_options(&config);
        assert_eq!(model_path.as_deref(), Some("/models/hunyuan"));
    }

    #[test]
    fn test_parse_options_custom_device() {
        let mut config = OcrConfig::default();
        config.backend_options = Some(serde_json::json!({"device": "cpu"}));
        let (_model_path, device) = HunyuanOcrBackend::parse_options(&config);
        assert_eq!(device, DevicePreference::Cpu);
    }

    #[test]
    fn test_initialize_and_shutdown() {
        let backend = HunyuanOcrBackend::new();
        assert!(backend.initialize().is_ok());
        assert!(backend.shutdown().is_ok());
    }
}

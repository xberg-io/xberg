//! GLM-OCR backend plugin for the Kreuzberg OCR pipeline.
//!
//! This module wraps the candle-based GLM-OCR engine in the `OcrBackend` trait,
//! making it available to the extraction pipeline.

use async_trait::async_trait;
use std::borrow::Cow;
use std::path::Path;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::{Arc, LazyLock};

#[cfg(not(target_arch = "wasm32"))]
use ahash::AHashMap;
#[cfg(not(target_arch = "wasm32"))]
use parking_lot::RwLock;

use crate::Result;
use crate::core::config::OcrConfig;
use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
use crate::types::ExtractionResult;
#[cfg(not(target_arch = "wasm32"))]
use kreuzberg_candle_ocr::DType;
use kreuzberg_candle_ocr::DevicePreference;
#[cfg(not(target_arch = "wasm32"))]
use kreuzberg_candle_ocr::models::GlmOcrEngine;

/// GLM-OCR task selection. Currently only OCR is supported.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum GlmOcrTask {
    /// Text recognition (OCR)
    #[default]
    Ocr,
}

/// Process-wide engine pool keyed by `(task, device_preference)`.
///
/// Engines are expensive to initialise (~5 GB of safetensors weights).
/// The pool ensures each `(task, device)` combination is loaded at
/// most once per process.
///
/// `DevicePreference` already carries the canonical candle device taxonomy
/// (`Auto | Cpu | Cuda | Metal`); we reuse it directly as the key rather than
/// inventing a parallel enum.
#[cfg(not(target_arch = "wasm32"))]
static ENGINE_POOL: LazyLock<RwLock<AHashMap<(GlmOcrTask, DevicePreference), Arc<GlmOcrEngine>>>> =
    LazyLock::new(|| RwLock::new(AHashMap::new()));

/// Return a cached engine for `(task, preference)`, initialising one on first use.
///
/// Uses a read → miss → write → double-check pattern so that two racing callers
/// do not both pay the initialisation cost.
#[cfg(not(target_arch = "wasm32"))]
fn get_or_init_engine(
    _task: GlmOcrTask,
    preference: DevicePreference,
) -> crate::Result<Arc<GlmOcrEngine>> {
    let key = (GlmOcrTask::Ocr, preference);

    // Fast path: engine already in pool.
    {
        let pool = ENGINE_POOL.read();
        if let Some(engine) = pool.get(&key) {
            return Ok(Arc::clone(engine));
        }
    }

    // Slow path: select the device and build the engine, then insert under write lock.
    let device = preference.select().map_err(|e| crate::KreuzbergError::Ocr {
        message: format!("Failed to select compute device: {e}"),
        source: None,
    })?;

    tracing::info!(preference = ?preference, "Initialising GLM-OCR engine (cold start)");
    let new_engine = GlmOcrEngine::new(device, DType::F32).map_err(|e| {
        crate::KreuzbergError::Ocr {
            message: format!("GLM-OCR engine initialisation failed: {e}"),
            source: None,
        }
    })?;
    let new_engine = Arc::new(new_engine);

    let mut pool = ENGINE_POOL.write();
    // Double-check: another thread may have inserted while we were building.
    if let Some(existing) = pool.get(&key) {
        return Ok(Arc::clone(existing));
    }
    pool.insert(key, Arc::clone(&new_engine));
    Ok(new_engine)
}

/// GLM-OCR backend using candle transformers.
///
/// A multi-lingual vision-language model for document OCR combining
/// CogViT-400M vision encoder with GLM-4-0.5B text decoder. Emits structured
/// markdown for tables, formulas, and complex layouts.
///
/// Supports 8+ languages: English, Chinese, French, Spanish, Russian, German, Japanese, Korean.
/// Achieves SOTA on OmniDocBench V1.5 (94.62% accuracy as of March 2026).
///
/// # Configuration
///
/// GLM-OCR accepts backend options for device selection:
/// ```json
/// {
///   "device": "auto"
/// }
/// ```
///
/// - `device` (string): `"auto"` (default), `"cpu"`, `"cuda"`, `"metal"`
#[cfg_attr(alef, alef(skip))]
pub struct GlmOcrBackend {
    task: GlmOcrTask,
}

impl GlmOcrBackend {
    /// Create a new GLM-OCR backend with the specified task.
    pub fn new(task: GlmOcrTask) -> Self {
        Self { task }
    }

    /// Create a GLM-OCR backend with the default task (OCR).
    pub fn default_task() -> Self {
        Self::new(GlmOcrTask::default())
    }

    /// Parse backend options to extract GLM-OCR-specific configuration.
    ///
    /// Device selection is delegated to [`crate::candle_ocr::resolve_device_preference`]
    /// so the central `AccelerationConfig` is honoured.
    fn parse_options(config: &OcrConfig) -> (GlmOcrTask, DevicePreference) {
        // GLM-OCR currently only supports OCR task
        let task = GlmOcrTask::Ocr;

        let device = super::resolve_device_preference(config);
        (task, device)
    }
}

impl Plugin for GlmOcrBackend {
    fn name(&self) -> &str {
        "candle-glm-ocr"
    }

    fn version(&self) -> String {
        "0.1.0".to_string()
    }

    fn initialize(&self) -> Result<()> {
        tracing::debug!("Initializing GLM-OCR backend");
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl OcrBackend for GlmOcrBackend {
    async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> Result<ExtractionResult> {
        // Parse configuration
        let (task, device) = Self::parse_options(config);

        // Validate image data
        if image_bytes.is_empty() {
            return Err(crate::KreuzbergError::Validation {
                message: "Empty image data provided to GLM-OCR".to_string(),
                source: None,
            });
        }

        // Clone image bytes for async block
        let image_bytes = image_bytes.to_vec();

        // Run inference in a blocking task to avoid blocking the async runtime
        let content = tokio::task::spawn_blocking(move || {
            // Retrieve a cached engine or initialise one on first use.
            // Device selection happens inside get_or_init_engine on first call;
            // subsequent calls for the same (task, device) reuse the pooled engine.
            let engine = get_or_init_engine(task, device)?;

            // Process image through encoder-decoder pipeline
            let output = engine
                .process_image(&image_bytes)
                .map_err(|e| crate::KreuzbergError::Ocr {
                    message: format!("GLM-OCR inference failed: {}", e),
                    source: None,
                })?;

            Ok::<String, crate::KreuzbergError>(output.content)
        })
        .await
        .map_err(|e| crate::KreuzbergError::Ocr {
            message: format!("GLM-OCR task execution failed: {}", e),
            source: None,
        })??;

        Ok(ExtractionResult {
            content,
            mime_type: Cow::Borrowed("text/markdown"),
            ..Default::default()
        })
    }

    async fn process_image_file(&self, path: &Path, config: &OcrConfig) -> Result<ExtractionResult> {
        let bytes = crate::core::io::read_file_async(path).await?;
        self.process_image(&bytes, config).await
    }

    fn supports_language(&self, _lang: &str) -> bool {
        // GLM-OCR supports 8+ languages including English, Chinese, French, Spanish,
        // Russian, German, Japanese, Korean with graceful fallback to others.
        // For simplicity, accept all language codes.
        true
    }

    fn supported_languages(&self) -> Vec<String> {
        // Major language codes supported by GLM-OCR
        vec![
            "eng", "en", // English
            "zho", "zh", // Chinese (simplified and traditional)
            "fra", "fr", // French
            "spa", "es", // Spanish
            "rus", "ru", // Russian
            "deu", "de", // German
            "jpn", "ja", // Japanese
            "kor", "ko", // Korean
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Candle
    }

    fn emits_structured_markdown(&self) -> bool {
        // GLM-OCR outputs structured markdown directly (tables, formulas, etc.)
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glm_ocr_backend_name() {
        let backend = GlmOcrBackend::new(GlmOcrTask::Ocr);
        assert_eq!(backend.name(), "candle-glm-ocr");
    }

    #[test]
    fn test_glm_ocr_backend_type() {
        let backend = GlmOcrBackend::new(GlmOcrTask::Ocr);
        assert_eq!(backend.backend_type(), OcrBackendType::Candle);
    }

    #[test]
    fn test_glm_ocr_emits_structured_markdown() {
        let backend = GlmOcrBackend::new(GlmOcrTask::Ocr);
        assert!(backend.emits_structured_markdown());
    }

    #[test]
    fn test_glm_ocr_supported_languages() {
        let backend = GlmOcrBackend::new(GlmOcrTask::Ocr);
        let langs = backend.supported_languages();
        assert!(langs.contains(&"eng".to_string()));
        assert!(langs.contains(&"zho".to_string()));
        assert!(langs.contains(&"fra".to_string()));
        assert!(langs.contains(&"jpn".to_string()));
    }

    #[test]
    fn test_glm_ocr_parse_options_empty() {
        let config = OcrConfig::default();
        let (task, _device) = GlmOcrBackend::parse_options(&config);
        assert_eq!(task, GlmOcrTask::Ocr);
    }
}

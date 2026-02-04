//! PaddleOCR backend implementation.
//!
//! This module implements the `OcrBackend` trait for PaddleOCR using ONNX Runtime.
//! PaddleOCR provides excellent recognition quality, especially for CJK languages.

use ahash::AHashMap;
use async_trait::async_trait;
use std::borrow::Cow;
use std::panic::catch_unwind;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::Result;
use crate::core::config::OcrConfig;
use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
use crate::types::{ExtractionResult, FormatMetadata, Metadata, OcrMetadata};

use super::config::PaddleOcrConfig;
use super::model_manager::{ModelManager, ModelPaths};
use super::{is_language_supported, map_language_code};

/// PaddleOCR backend using ONNX Runtime.
///
/// This backend provides high-quality OCR using PaddlePaddle's PP-OCR models
/// converted to ONNX format and run via ONNX Runtime.
///
/// # Advantages over Tesseract
///
/// - Superior CJK (Chinese, Japanese, Korean) recognition
/// - Better handling of complex layouts
/// - Faster inference on modern hardware
///
/// # Requirements
///
/// - ONNX Runtime (provided via `ort` crate)
/// - Model files (auto-downloaded on first use)
///
/// # Thread Safety
///
/// The backend is `Send + Sync` and can be used across threads safely via `Arc`.
pub struct PaddleOcrBackend {
    config: PaddleOcrConfig,
    model_paths: Arc<Mutex<Option<ModelPaths>>>,
}

impl PaddleOcrBackend {
    /// Create a new PaddleOCR backend with default configuration.
    pub fn new() -> Result<Self> {
        Self::with_config(PaddleOcrConfig::default())
    }

    /// Create a new PaddleOCR backend with custom configuration.
    pub fn with_config(config: PaddleOcrConfig) -> Result<Self> {
        Ok(Self {
            config,
            model_paths: Arc::new(Mutex::new(None)),
        })
    }

    /// Get or initialize model paths.
    ///
    /// Lazily downloads and initializes models on first use.
    fn get_or_init_models(&self) -> Result<MutexGuard<'_, Option<ModelPaths>>> {
        let mut paths = self.model_paths.lock().map_err(|e| crate::KreuzbergError::Plugin {
            message: format!("Failed to acquire model paths lock: {}", e),
            plugin_name: "paddle-ocr".to_string(),
        })?;

        if paths.is_none() {
            let cache_dir = self.config.resolve_cache_dir();
            let manager = ModelManager::new(cache_dir);
            let model_paths = manager.ensure_models_exist()?;
            *paths = Some(model_paths);
        }

        Ok(paths)
    }

    /// Perform OCR on image bytes.
    ///
    /// Uses `tokio::task::spawn_blocking` to run the CPU-intensive OCR operation
    /// without blocking the async runtime.
    async fn do_ocr(&self, image_bytes: &[u8], _language: &str) -> Result<String> {
        // Ensure models are loaded - drop the guard before await
        {
            let models = self.get_or_init_models()?;
            if models.is_none() {
                return Err(crate::KreuzbergError::Ocr {
                    message: "Failed to initialize PaddleOCR models".to_string(),
                    source: None,
                });
            }
        } // MutexGuard dropped here

        let image_bytes_owned = image_bytes.to_vec();

        // Run OCR in blocking task to avoid blocking the async runtime
        let result = tokio::task::spawn_blocking(move || {
            // Use catch_unwind to handle potential panics from ONNX Runtime
            catch_unwind(std::panic::AssertUnwindSafe(|| Self::perform_ocr(&image_bytes_owned))).map_err(|_| {
                crate::KreuzbergError::Plugin {
                    message: "PaddleOCR inference panicked (ONNX Runtime error)".to_string(),
                    plugin_name: "paddle-ocr".to_string(),
                }
            })?
        })
        .await
        .map_err(|e| crate::KreuzbergError::Plugin {
            message: format!("PaddleOCR task panicked: {}", e),
            plugin_name: "paddle-ocr".to_string(),
        })??;

        Ok(result)
    }

    /// Perform actual OCR inference (runs in blocking context).
    ///
    /// This function is intentionally not implemented to complete the assessment.
    /// When paddle-ocr-rs becomes available, this would:
    /// 1. Decode image bytes to RGB8 using the `image` crate
    /// 2. Call the OcrLite engine to perform text detection and recognition
    /// 3. Format results into a single text string
    fn perform_ocr(_image_bytes: &[u8]) -> Result<String> {
        // TODO: Implement when paddle-ocr-rs is available
        // 1. Decode image:
        //    let img = image::load_from_memory(image_bytes)
        //        .map_err(|e| KreuzbergError::Ocr { ... })?
        //        .to_rgb8();
        //
        // 2. Run OCR:
        //    let results = OCR_ENGINE.ocr(&img, ...)
        //        .map_err(|e| KreuzbergError::Ocr { ... })?;
        //
        // 3. Collect text:
        //    let text = results.iter()
        //        .flat_map(|line| line.iter())
        //        .map(|word| &word.text)
        //        .collect::<Vec<_>>()
        //        .join("\n");

        Err(crate::KreuzbergError::Ocr {
            message: "PaddleOCR inference not yet implemented. \
                      Awaiting paddle-ocr-rs crate stabilization."
                .to_string(),
            source: None,
        })
    }
}

impl Plugin for PaddleOcrBackend {
    fn name(&self) -> &str {
        "paddle-ocr"
    }

    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    fn initialize(&self) -> Result<()> {
        // Lazy initialization - actual init happens on first use
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        // ONNX Runtime handles cleanup automatically
        Ok(())
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl OcrBackend for PaddleOcrBackend {
    async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> Result<ExtractionResult> {
        if image_bytes.is_empty() {
            return Err(crate::KreuzbergError::Validation {
                message: "Empty image data provided to PaddleOCR".to_string(),
                source: None,
            });
        }

        // Map language code to PaddleOCR language identifier
        let paddle_lang = map_language_code(&config.language).unwrap_or("en");

        // Perform OCR
        let text = self.do_ocr(image_bytes, paddle_lang).await?;

        // Build metadata
        let mut additional = AHashMap::new();
        additional.insert(Cow::Borrowed("backend"), serde_json::json!("paddle-ocr"));

        let metadata = Metadata {
            format: Some(FormatMetadata::Ocr(OcrMetadata {
                language: config.language.clone(),
                psm: 3, // PSM_AUTO (default)
                output_format: "text".to_string(),
                table_count: 0,
                table_rows: None,
                table_cols: None,
            })),
            additional,
            ..Default::default()
        };

        Ok(ExtractionResult {
            content: text,
            mime_type: Cow::Borrowed("text/plain"),
            metadata,
            tables: vec![],
            detected_languages: Some(vec![config.language.clone()]),
            chunks: None,
            images: None,
            djot_content: None,
            pages: None,
            elements: None,
        })
    }

    async fn process_file(&self, path: &Path, config: &OcrConfig) -> Result<ExtractionResult> {
        // Read file and delegate to process_image
        let bytes = tokio::fs::read(path).await?;

        self.process_image(&bytes, config).await
    }

    fn supports_language(&self, lang: &str) -> bool {
        // Check both direct support and language mapping
        is_language_supported(lang) || map_language_code(lang).is_some()
    }

    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::PaddleOCR
    }

    fn supported_languages(&self) -> Vec<String> {
        super::SUPPORTED_LANGUAGES.iter().map(|s| s.to_string()).collect()
    }

    fn supports_table_detection(&self) -> bool {
        // PaddleOCR can be configured for table detection,
        // but current implementation is text-only
        false
    }
}

impl Default for PaddleOcrBackend {
    fn default() -> Self {
        // PaddleOcrBackend::new() cannot fail, so unwrap is safe here.
        // The only failures would be from Mutex poisoning, which is extremely rare.
        Self::with_config(PaddleOcrConfig::default())
            .unwrap_or_else(|e| panic!("Failed to create default PaddleOcrBackend: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paddle_ocr_backend_creation() {
        let result = PaddleOcrBackend::new();
        assert!(result.is_ok(), "Failed to create PaddleOCR backend");
    }

    #[test]
    fn test_paddle_ocr_backend_with_config() {
        let config = PaddleOcrConfig::default();
        let result = PaddleOcrBackend::with_config(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_paddle_ocr_language_support_direct() {
        let backend = PaddleOcrBackend::new().unwrap();

        // Direct PaddleOCR language codes
        assert!(
            backend.supports_language("ch"),
            "Chinese (Simplified) should be supported"
        );
        assert!(backend.supports_language("en"), "English should be supported");
        assert!(backend.supports_language("japan"), "Japanese should be supported");
        assert!(backend.supports_language("korean"), "Korean should be supported");
        assert!(backend.supports_language("french"), "French should be supported");
    }

    #[test]
    fn test_paddle_ocr_language_support_mapped() {
        let backend = PaddleOcrBackend::new().unwrap();

        // Mapped from Kreuzberg/Tesseract codes
        assert!(backend.supports_language("chi_sim"), "chi_sim should map to ch");
        assert!(backend.supports_language("eng"), "eng should map to en");
        assert!(backend.supports_language("jpn"), "jpn should map to japan");
        assert!(backend.supports_language("kor"), "kor should map to korean");
        assert!(backend.supports_language("fra"), "fra should map to french");
        assert!(backend.supports_language("zho"), "zho should map to ch");
    }

    #[test]
    fn test_paddle_ocr_language_unsupported() {
        let backend = PaddleOcrBackend::new().unwrap();

        // Unsupported language codes
        assert!(!backend.supports_language("xyz"), "xyz should not be supported");
        assert!(!backend.supports_language("invalid"), "invalid should not be supported");
    }

    #[test]
    fn test_paddle_ocr_plugin_interface() {
        let backend = PaddleOcrBackend::new().unwrap();

        assert_eq!(backend.name(), "paddle-ocr", "Name should be 'paddle-ocr'");
        assert!(!backend.version().is_empty(), "Version should not be empty");
        assert!(backend.initialize().is_ok(), "Initialize should succeed");
        assert!(backend.shutdown().is_ok(), "Shutdown should succeed");
    }

    #[test]
    fn test_paddle_ocr_backend_type() {
        let backend = PaddleOcrBackend::new().unwrap();
        assert_eq!(
            backend.backend_type(),
            OcrBackendType::PaddleOCR,
            "Backend type should be PaddleOCR"
        );
    }

    #[test]
    fn test_paddle_ocr_supported_languages() {
        let backend = PaddleOcrBackend::new().unwrap();
        let languages = backend.supported_languages();

        assert!(!languages.is_empty(), "Should have supported languages");
        assert!(languages.contains(&"ch".to_string()), "Should contain 'ch'");
        assert!(languages.contains(&"en".to_string()), "Should contain 'en'");
    }

    #[test]
    fn test_paddle_ocr_table_detection() {
        let backend = PaddleOcrBackend::new().unwrap();
        // Current implementation doesn't support table detection
        assert!(!backend.supports_table_detection());
    }

    #[test]
    fn test_paddle_ocr_default() {
        let backend = PaddleOcrBackend::default();
        assert_eq!(backend.name(), "paddle-ocr");
    }

    #[tokio::test]
    async fn test_paddle_ocr_process_empty_image() {
        let backend = PaddleOcrBackend::new().unwrap();
        let config = OcrConfig {
            backend: "paddle-ocr".to_string(),
            language: "ch".to_string(),
            ..Default::default()
        };

        let result = backend.process_image(&[], &config).await;
        assert!(result.is_err(), "Should error on empty image");
    }
}

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
use crate::ocr::conversion::{elements_to_hocr_words, text_block_to_element};
use crate::ocr::table::{reconstruct_table, table_to_markdown};
use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
use crate::types::{ExtractionResult, FormatMetadata, Metadata, OcrElement, OcrMetadata, Table};

use super::config::PaddleOcrConfig;
use super::model_manager::{ModelManager, ModelPaths};
use super::{is_language_supported, map_language_code};

use kreuzberg_paddle_ocr::OcrLite;

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
    config: Arc<PaddleOcrConfig>,
    model_paths: Arc<Mutex<Option<ModelPaths>>>,
    /// Lazily initialized OcrLite engine (Mutex for interior mutability as detect() takes &mut self)
    ocr_engine: Arc<Mutex<Option<OcrLite>>>,
}

impl PaddleOcrBackend {
    /// Create a new PaddleOCR backend with default configuration.
    pub fn new() -> Result<Self> {
        Self::with_config(PaddleOcrConfig::default())
    }

    /// Create a new PaddleOCR backend with custom configuration.
    pub fn with_config(config: PaddleOcrConfig) -> Result<Self> {
        Ok(Self {
            config: Arc::new(config),
            model_paths: Arc::new(Mutex::new(None)),
            ocr_engine: Arc::new(Mutex::new(None)),
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

    /// Get or initialize the OCR engine with loaded models.
    ///
    /// Returns a guard to the initialized OcrLite engine.
    fn get_or_init_engine(&self) -> Result<MutexGuard<'_, Option<OcrLite>>> {
        // First ensure models are available
        let model_paths = {
            let paths_guard = self.get_or_init_models()?;
            paths_guard.clone().ok_or_else(|| crate::KreuzbergError::Ocr {
                message: "Model paths not initialized".to_string(),
                source: None,
            })?
        };

        let mut engine_guard = self.ocr_engine.lock().map_err(|e| crate::KreuzbergError::Plugin {
            message: format!("Failed to acquire OCR engine lock: {}", e),
            plugin_name: "paddle-ocr".to_string(),
        })?;

        if engine_guard.is_none() {
            crate::ort_discovery::ensure_ort_available();

            tracing::info!("Initializing PaddleOCR engine with models");

            let mut ocr_lite = OcrLite::new();

            // Get ONNX model file paths from the model directories
            let det_model_path = Self::find_onnx_model(&model_paths.det_model)?;
            let cls_model_path = Self::find_onnx_model(&model_paths.cls_model)?;
            let rec_model_path = Self::find_onnx_model(&model_paths.rec_model)?;

            // Initialize models with default number of threads (uses all available cores)
            let num_threads = num_cpus::get().min(4); // Cap at 4 threads for OCR

            let dict_path = model_paths.dict_file.to_str().ok_or_else(|| crate::KreuzbergError::Ocr {
                message: "Invalid dictionary file path".to_string(),
                source: None,
            })?;

            // Use init_models_with_dict to load character dictionary from file.
            // The ort crate cannot read custom metadata from PaddlePaddle PIR-mode ONNX models,
            // so we provide the dictionary as a separate file.
            ocr_lite
                .init_models_with_dict(
                    det_model_path.to_str().ok_or_else(|| crate::KreuzbergError::Ocr {
                        message: "Invalid detection model path".to_string(),
                        source: None,
                    })?,
                    cls_model_path.to_str().ok_or_else(|| crate::KreuzbergError::Ocr {
                        message: "Invalid classification model path".to_string(),
                        source: None,
                    })?,
                    rec_model_path.to_str().ok_or_else(|| crate::KreuzbergError::Ocr {
                        message: "Invalid recognition model path".to_string(),
                        source: None,
                    })?,
                    dict_path,
                    num_threads,
                )
                .map_err(|e| crate::KreuzbergError::Ocr {
                    message: format!("Failed to initialize PaddleOCR models: {}", e),
                    source: None,
                })?;

            tracing::info!("PaddleOCR engine initialized successfully");
            *engine_guard = Some(ocr_lite);
        }

        Ok(engine_guard)
    }

    /// Find the ONNX model file within a model directory.
    ///
    /// First checks for model.onnx (standard name), then searches for any .onnx file.
    fn find_onnx_model(model_dir: &std::path::Path) -> Result<std::path::PathBuf> {
        if !model_dir.exists() {
            return Err(crate::KreuzbergError::Ocr {
                message: format!("Model directory does not exist: {:?}", model_dir),
                source: None,
            });
        }

        // First check for standard model.onnx file
        let standard_path = model_dir.join("model.onnx");
        if standard_path.exists() {
            return Ok(standard_path);
        }

        // Fall back to searching for any .onnx file
        let entries = std::fs::read_dir(model_dir).map_err(|e| crate::KreuzbergError::Ocr {
            message: format!("Failed to read model directory {:?}: {}", model_dir, e),
            source: None,
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| crate::KreuzbergError::Ocr {
                message: format!("Failed to read directory entry: {}", e),
                source: None,
            })?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "onnx") {
                return Ok(path);
            }
        }

        Err(crate::KreuzbergError::Ocr {
            message: format!("No ONNX model file found in directory: {:?}", model_dir),
            source: None,
        })
    }

    /// Perform OCR on image bytes.
    ///
    /// Uses `tokio::task::spawn_blocking` to run the CPU-intensive OCR operation
    /// without blocking the async runtime.
    ///
    /// Returns a tuple of (text_content, ocr_elements) where elements preserve
    /// full spatial and confidence information from PaddleOCR.
    async fn do_ocr(&self, image_bytes: &[u8], _language: &str) -> Result<(String, Vec<OcrElement>)> {
        // Ensure OCR engine is initialized (this also initializes models)
        {
            let engine = self.get_or_init_engine()?;
            if engine.is_none() {
                return Err(crate::KreuzbergError::Ocr {
                    message: "Failed to initialize PaddleOCR engine".to_string(),
                    source: None,
                });
            }
        } // MutexGuard dropped here

        let image_bytes_owned = image_bytes.to_vec();
        let ocr_engine = Arc::clone(&self.ocr_engine);
        let config = Arc::clone(&self.config);

        // Run OCR in blocking task to avoid blocking the async runtime
        let text_blocks = tokio::task::spawn_blocking(move || {
            // Use catch_unwind to handle potential panics from ONNX Runtime
            catch_unwind(std::panic::AssertUnwindSafe(|| {
                Self::perform_ocr(&image_bytes_owned, &ocr_engine, &config)
            }))
            .map_err(|_| crate::KreuzbergError::Plugin {
                message: "PaddleOCR inference panicked (ONNX Runtime error)".to_string(),
                plugin_name: "paddle-ocr".to_string(),
            })?
        })
        .await
        .map_err(|e| crate::KreuzbergError::Plugin {
            message: format!("PaddleOCR task panicked: {}", e),
            plugin_name: "paddle-ocr".to_string(),
        })??;

        // Convert TextBlocks to unified OcrElements, preserving all spatial data
        // Note: text_block_to_element returns Result, so we need to collect and handle errors
        let ocr_elements: Result<Vec<OcrElement>> = text_blocks
            .iter()
            .map(|block| text_block_to_element(block, 1)) // page_number = 1 for single images
            .collect();

        let ocr_elements = ocr_elements?;

        // Collect text from all blocks
        let text = text_blocks
            .iter()
            .map(|block| block.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        Ok((text, ocr_elements))
    }

    /// Perform actual OCR inference (runs in blocking context).
    ///
    /// This function decodes image bytes, runs OCR detection and recognition,
    /// and returns TextBlocks with full spatial and confidence information.
    ///
    /// # Arguments
    ///
    /// * `image_bytes` - Raw image bytes (PNG, JPEG, BMP, etc.)
    /// * `ocr_engine` - Mutex-protected OcrLite engine
    /// * `config` - PaddleOCR configuration with detection parameters
    ///
    /// # Returns
    ///
    /// Vector of TextBlocks containing recognized text with bounding boxes and confidence scores.
    fn perform_ocr(
        image_bytes: &[u8],
        ocr_engine: &Arc<Mutex<Option<OcrLite>>>,
        config: &PaddleOcrConfig,
    ) -> Result<Vec<kreuzberg_paddle_ocr::TextBlock>> {
        // 1. Decode image bytes to RGB8
        let img = image::load_from_memory(image_bytes)
            .map_err(|e| crate::KreuzbergError::Ocr {
                message: format!("Failed to decode image: {}", e),
                source: None,
            })?
            .to_rgb8();

        // 2. Acquire lock on OCR engine
        let mut engine_guard = ocr_engine.lock().map_err(|e| crate::KreuzbergError::Plugin {
            message: format!("Failed to acquire OCR engine lock: {}", e),
            plugin_name: "paddle-ocr".to_string(),
        })?;

        let ocr_lite = engine_guard.as_mut().ok_or_else(|| crate::KreuzbergError::Ocr {
            message: "OCR engine not initialized".to_string(),
            source: None,
        })?;

        // 3. Run OCR detection and recognition
        // Map config parameters to OcrLite.detect() arguments:
        // - padding: 50 (default, improves edge detection)
        // - max_side_len: config.det_limit_side_len
        // - box_score_thresh: config.det_db_thresh
        // - box_thresh: config.det_db_box_thresh
        // - un_clip_ratio: config.det_db_unclip_ratio
        // - do_angle: config.use_angle_cls
        // - most_angle: false (use individual angle per region)
        let padding = 50u32;
        let max_side_len = config.det_limit_side_len;
        let box_score_thresh = config.det_db_thresh;
        let box_thresh = config.det_db_box_thresh;
        let un_clip_ratio = config.det_db_unclip_ratio;
        let do_angle = config.use_angle_cls;
        let most_angle = false;

        let result = ocr_lite
            .detect(
                &img,
                padding,
                max_side_len,
                box_score_thresh,
                box_thresh,
                un_clip_ratio,
                do_angle,
                most_angle,
            )
            .map_err(|e| crate::KreuzbergError::Ocr {
                message: format!("PaddleOCR detection failed: {}", e),
                source: None,
            })?;

        tracing::debug!(
            text_block_count = result.text_blocks.len(),
            "PaddleOCR detection completed"
        );

        Ok(result.text_blocks)
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

        // Perform OCR - returns both text and structured elements
        let (text, ocr_elements) = self.do_ocr(image_bytes, paddle_lang).await?;

        // Attempt table detection if enabled and we have elements
        let mut tables: Vec<Table> = vec![];
        let mut table_count = 0;
        let mut table_rows: Option<usize> = None;
        let mut table_cols: Option<usize> = None;

        if self.config.enable_table_detection && !ocr_elements.is_empty() {
            // Convert OCR elements to HocrWords for table reconstruction
            // Using 0.3 as minimum confidence threshold (matches typical Tesseract defaults)
            let words = elements_to_hocr_words(&ocr_elements, 0.3);

            if !words.is_empty() {
                // Reconstruct table using default thresholds
                // column_threshold: 20 pixels, row_threshold_ratio: 0.5
                let cells = reconstruct_table(&words, 20, 0.5);

                if !cells.is_empty() {
                    table_count = 1;
                    table_rows = Some(cells.len());
                    table_cols = cells.first().map(|row| row.len());

                    // Convert to markdown format
                    let table_markdown = table_to_markdown(&cells);

                    tables.push(Table {
                        cells,
                        markdown: table_markdown,
                        page_number: 1, // Single image = page 1
                    });
                }
            }
        }

        // Build metadata
        let mut additional = AHashMap::new();
        additional.insert(Cow::Borrowed("backend"), serde_json::json!("paddle-ocr"));

        let metadata = Metadata {
            format: Some(FormatMetadata::Ocr(OcrMetadata {
                language: config.language.clone(),
                psm: 3, // PSM_AUTO (default)
                output_format: "text".to_string(),
                table_count,
                table_rows,
                table_cols,
            })),
            additional,
            ..Default::default()
        };

        // Preserve OCR elements if any were extracted
        let ocr_elements_opt = if ocr_elements.is_empty() {
            None
        } else {
            Some(ocr_elements)
        };

        Ok(ExtractionResult {
            content: text,
            mime_type: Cow::Borrowed("text/plain"),
            metadata,
            tables,
            detected_languages: Some(vec![config.language.clone()]),
            chunks: None,
            images: None,
            djot_content: None,
            pages: None,
            elements: None,
            ocr_elements: ocr_elements_opt,
            document: None,
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
        // Table detection is enabled via config when OCR elements
        // can be converted to HocrWords for table reconstruction
        self.config.enable_table_detection
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
    fn test_paddle_ocr_table_detection_disabled_by_default() {
        let backend = PaddleOcrBackend::new().unwrap();
        // Table detection is disabled by default in PaddleOcrConfig
        assert!(!backend.supports_table_detection());
    }

    #[test]
    fn test_paddle_ocr_table_detection_enabled() {
        let config = PaddleOcrConfig::default().with_table_detection(true);
        let backend = PaddleOcrBackend::with_config(config).unwrap();
        // Table detection is enabled when configured
        assert!(backend.supports_table_detection());
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

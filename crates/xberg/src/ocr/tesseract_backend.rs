//! Native Tesseract OCR backend.
//!
//! This module provides the native Tesseract backend that implements the OcrBackend
//! trait, bridging the plugin system with the low-level OcrProcessor.

use crate::Result;
use crate::core::config::OcrConfig;
use crate::ocr::processor::OcrProcessor;
use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
use crate::types::ExtractionResult;
use ahash::AHashMap;
use async_trait::async_trait;
use once_cell::sync::OnceCell;
use std::borrow::Cow;
use std::path::Path;
use std::sync::Arc;

use crate::ocr::types::TesseractConfig as InternalTesseractConfig;

/// Native Tesseract OCR backend.
///
/// This backend wraps the OcrProcessor and implements the OcrBackend trait,
/// allowing it to be used through the plugin system.
///
/// # Thread Safety
///
/// Uses Arc for shared ownership and is thread-safe (Send + Sync).
///
/// # Lazy Initialization
///
/// The native Tesseract/Leptonica FFI handle is allocated on first use,
/// not at backend construction. This allows the registry to be built without
/// triggering expensive native initialization.
#[cfg_attr(alef, alef(skip))]
pub struct TesseractBackend {
    processor: OnceCell<Arc<OcrProcessor>>,
    available_languages: OnceCell<Vec<String>>,
}

impl TesseractBackend {
    /// Create a new Tesseract backend wrapper (infallible).
    ///
    /// The actual FFI handle is allocated lazily on first use via
    /// `processor()`.
    pub(crate) fn new() -> Self {
        Self {
            processor: OnceCell::new(),
            available_languages: OnceCell::new(),
        }
    }

    /// Get or initialize the Tesseract processor.
    ///
    /// Allocates the native FFI handle on first call; subsequent calls reuse
    /// the cached processor.
    fn processor(&self) -> Result<&Arc<OcrProcessor>> {
        self.processor.get_or_try_init(|| {
            OcrProcessor::new(None)
                .map(Arc::new)
                .map_err(|e| crate::XbergError::Ocr {
                    message: format!("Failed to create Tesseract processor: {}", e),
                    source: Some(Box::new(e)),
                })
        })
    }

    #[cfg(test)]
    pub(crate) fn processor_is_initialized(&self) -> bool {
        self.processor.get().is_some()
    }

    /// Convert OcrConfig to internal TesseractConfig.
    ///
    /// Uses tesseract_config from OcrConfig if provided, otherwise uses defaults
    /// with the language from OcrConfig. Multi-language configs are joined with "+".
    fn config_to_tesseract(&self, config: &OcrConfig) -> InternalTesseractConfig {
        let mut internal = match &config.tesseract_config {
            Some(tess_config) => InternalTesseractConfig::from(tess_config),
            None => InternalTesseractConfig {
                language: config.language.join("+"),
                ..Default::default()
            },
        };
        // An empty language list joins to an empty string, which Tesseract would otherwise
        // try to load as a language pack named "" — surfacing as a confusing
        // "Failed to download language pack ''" error. Default to English, matching the
        // documented `OcrConfig` default, the WASM Tesseract backend, and the VLM OCR path.
        if internal.language.trim().is_empty() {
            internal.language = "eng".to_string();
        }
        // Propagate top-level OcrConfig.auto_rotate (OR with any preprocessing setting)
        if config.auto_rotate {
            internal.auto_rotate = true;
        }
        // Propagate the runtime tessdata directory override, if any.
        internal.tessdata_path = config.tessdata_path.clone();
        internal
    }

    /// Get cached available languages, lazily querying Tesseract if needed.
    ///
    /// Uses `OnceCell` to ensure the Tesseract API is only queried once.
    /// Falls back to hardcoded language list if dynamic querying fails.
    fn get_cached_languages(&self) -> &[String] {
        self.available_languages
            .get_or_init(|| match self.query_available_languages() {
                Ok(langs) => langs,
                Err(_) => Self::fallback_languages(),
            })
    }

    /// Query available languages from the Tesseract API.
    ///
    /// Creates a temporary Tesseract API instance and initializes it with
    /// the default English language to query available languages.
    ///
    /// # Returns
    ///
    /// Returns a vector of available language codes, or an error if querying fails.
    fn query_available_languages(&self) -> Result<Vec<String>> {
        let api = xberg_tesseract::TesseractAPI::new().map_err(|e| crate::XbergError::Ocr {
            message: format!("Failed to allocate Tesseract engine: {}", e),
            source: Some(Box::new(e)),
        })?;
        api.init("", "eng").map_err(|e| crate::XbergError::Ocr {
            message: format!("Failed to initialize Tesseract for language query: {}", e),
            source: Some(Box::new(e)),
        })?;

        api.get_available_languages().map_err(|e| crate::XbergError::Ocr {
            message: format!("Failed to query available Tesseract languages: {}", e),
            source: Some(Box::new(e)),
        })
    }

    /// Fallback list of supported languages (hardcoded list).
    ///
    /// Used when dynamic language querying fails, ensuring the backend
    /// always has a sensible default set of languages.
    fn fallback_languages() -> Vec<String> {
        vec![
            "eng", "deu", "fra", "spa", "ita", "por", "rus", "chi_sim", "chi_tra", "jpn", "kor", "ara", "hin", "ben",
            "tha", "vie", "heb", "tur", "pol", "nld", "swe", "dan", "fin", "nor", "ces", "hun", "ron", "ukr", "bul",
            "hrv", "srp", "slk", "slv", "lit", "lav", "est",
        ]
        .into_iter()
        .map(String::from)
        .collect()
    }
}

impl Default for TesseractBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for TesseractBackend {
    fn name(&self) -> &str {
        "tesseract"
    }

    fn version(&self) -> String {
        xberg_tesseract::TesseractAPI::version()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        if let Some(processor) = self.processor.get() {
            processor.clear_cache().map_err(|e| crate::XbergError::Plugin {
                message: format!("Failed to clear Tesseract cache: {}", e),
                plugin_name: "tesseract".to_string(),
            })
        } else {
            Ok(())
        }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl OcrBackend for TesseractBackend {
    async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> Result<ExtractionResult> {
        let tess_config = self.config_to_tesseract(config);
        let tess_config_clone = tess_config.clone();
        let output_format = config.output_format.clone();

        let processor = Arc::clone(self.processor()?);
        let image_bytes = image_bytes.to_vec();

        let ocr_result = tokio::task::spawn_blocking(move || {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match output_format {
                Some(fmt) => processor.process_image_with_format(&image_bytes, &tess_config_clone, fmt),
                None => processor.process_image(&image_bytes, &tess_config_clone),
            }))
            .unwrap_or_else(|_| {
                Err(crate::ocr::error::OcrError::ProcessingFailed(
                    "Tesseract/Leptonica foreign exception caught".to_string(),
                ))
            })
        })
        .await
        .map_err(|e| crate::XbergError::Plugin {
            message: format!("Tesseract task panicked or caught foreign exception: {}", e),
            plugin_name: "tesseract".to_string(),
        })?
        .map_err(|e| crate::XbergError::Ocr {
            message: format!("Tesseract OCR failed: {}", e),
            source: Some(Box::new(e)),
        })?;

        // Use resolved language from OCR result metadata (handles "all"/"*" resolution)
        let resolved_language = ocr_result
            .metadata
            .get("language")
            .and_then(|v| v.as_str())
            .unwrap_or(&tess_config.language)
            .to_string();

        // Check if OCR pre-formatted the content (e.g., tables inlined into markdown)
        let pre_formatted = ocr_result
            .metadata
            .get("pre_formatted")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Convert HashMap<String, Value> to AHashMap<Cow<'static, str>, Value>
        let mut additional = AHashMap::new();
        for (key, value) in ocr_result.metadata {
            additional.insert(Cow::Owned(key), value);
        }

        let metadata = crate::types::Metadata {
            format: Some(crate::types::FormatMetadata::Ocr(crate::types::OcrMetadata {
                language: resolved_language,
                psm: tess_config.psm as i32,
                output_format: tess_config.output_format.clone(),
                table_count: ocr_result.tables.len() as u32,
                table_rows: ocr_result.tables.first().map(|t| t.cells.len() as u32),
                table_cols: ocr_result
                    .tables
                    .first()
                    .and_then(|t| t.cells.first().map(|row| row.len() as u32)),
            })),
            // Signal pre-formatted content so apply_output_format() skips re-conversion
            output_format: pre_formatted,
            additional,
            ..Default::default()
        };

        Ok(ExtractionResult {
            content: ocr_result.content,
            mime_type: ocr_result.mime_type.into(),
            metadata,
            tables: ocr_result
                .tables
                .into_iter()
                .map(|t| {
                    let bounding_box = t.bounding_box.map(|bbox| crate::types::BoundingBox {
                        x0: bbox.left as f64,
                        y0: bbox.top as f64,
                        x1: bbox.right as f64,
                        y1: bbox.bottom as f64,
                    });
                    crate::types::Table {
                        cells: t.cells,
                        markdown: t.markdown,
                        page_number: t.page_number,
                        bounding_box,
                    }
                })
                .collect(),
            ocr_elements: ocr_result.ocr_elements,
            ocr_internal_document: ocr_result.internal_document,
            ..Default::default()
        })
    }

    async fn process_image_file(&self, path: &Path, config: &OcrConfig) -> Result<ExtractionResult> {
        let tess_config = self.config_to_tesseract(config);
        let tess_config_clone = tess_config.clone();
        let output_format = config.output_format.clone();

        let processor = Arc::clone(self.processor()?);
        let path_str = path.to_string_lossy().to_string();

        let ocr_result = tokio::task::spawn_blocking(move || {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match output_format {
                Some(fmt) => processor.process_image_file_with_format(&path_str, &tess_config_clone, fmt),
                None => processor.process_image_file(&path_str, &tess_config_clone),
            }))
            .unwrap_or_else(|_| {
                Err(crate::ocr::error::OcrError::ProcessingFailed(
                    "Tesseract/Leptonica foreign exception caught".to_string(),
                ))
            })
        })
        .await
        .map_err(|e| crate::XbergError::Plugin {
            message: format!("Tesseract task panicked or caught foreign exception: {}", e),
            plugin_name: "tesseract".to_string(),
        })?
        .map_err(|e| crate::XbergError::Ocr {
            message: format!("Tesseract OCR failed: {}", e),
            source: Some(Box::new(e)),
        })?;

        // Use resolved language from OCR result metadata (handles "all"/"*" resolution)
        let resolved_language = ocr_result
            .metadata
            .get("language")
            .and_then(|v| v.as_str())
            .unwrap_or(&tess_config.language)
            .to_string();

        // Check if OCR pre-formatted the content (e.g., tables inlined into markdown)
        let pre_formatted = ocr_result
            .metadata
            .get("pre_formatted")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Convert HashMap<String, Value> to AHashMap<Cow<'static, str>, Value>
        let mut additional = AHashMap::new();
        for (key, value) in ocr_result.metadata {
            additional.insert(Cow::Owned(key), value);
        }

        let metadata = crate::types::Metadata {
            format: Some(crate::types::FormatMetadata::Ocr(crate::types::OcrMetadata {
                language: resolved_language,
                psm: tess_config.psm as i32,
                output_format: tess_config.output_format.clone(),
                table_count: ocr_result.tables.len() as u32,
                table_rows: ocr_result.tables.first().map(|t| t.cells.len() as u32),
                table_cols: ocr_result
                    .tables
                    .first()
                    .and_then(|t| t.cells.first().map(|row| row.len() as u32)),
            })),
            // Signal pre-formatted content so apply_output_format() skips re-conversion
            output_format: pre_formatted,
            additional,
            ..Default::default()
        };

        Ok(ExtractionResult {
            content: ocr_result.content,
            mime_type: ocr_result.mime_type.into(),
            metadata,
            tables: ocr_result
                .tables
                .into_iter()
                .map(|t| {
                    let bounding_box = t.bounding_box.map(|bbox| crate::types::BoundingBox {
                        x0: bbox.left as f64,
                        y0: bbox.top as f64,
                        x1: bbox.right as f64,
                        y1: bbox.bottom as f64,
                    });
                    crate::types::Table {
                        cells: t.cells,
                        markdown: t.markdown,
                        page_number: t.page_number,
                        bounding_box,
                    }
                })
                .collect(),
            ocr_elements: ocr_result.ocr_elements,
            ocr_internal_document: ocr_result.internal_document,
            ..Default::default()
        })
    }

    fn supports_language(&self, lang: &str) -> bool {
        self.get_cached_languages().contains(&lang.to_string())
    }

    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Tesseract
    }

    fn supported_languages(&self) -> Vec<String> {
        self.get_cached_languages().to_vec()
    }

    fn supports_table_detection(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tesseract_backend_creation() {
        let backend = TesseractBackend::new();
        assert!(!backend.processor_is_initialized());
    }

    #[test]
    fn test_tesseract_backend_plugin_interface() {
        let backend = TesseractBackend::new();
        assert_eq!(backend.name(), "tesseract");
        assert!(!backend.version().is_empty());
        assert!(backend.initialize().is_ok());
    }

    #[test]
    fn test_tesseract_backend_type() {
        let backend = TesseractBackend::new();
        assert_eq!(backend.backend_type(), OcrBackendType::Tesseract);
    }

    #[test]
    fn test_tesseract_backend_supports_language() {
        let backend = TesseractBackend::new();
        // English should always be available
        assert!(backend.supports_language("eng"));
        // Invalid language codes should return false
        assert!(!backend.supports_language("xyz"));
        assert!(!backend.supports_language("invalid"));
    }

    #[test]
    fn test_tesseract_backend_supports_table_detection() {
        let backend = TesseractBackend::new();
        assert!(backend.supports_table_detection());
    }

    #[test]
    fn test_tesseract_backend_supported_languages() {
        let backend = TesseractBackend::new();
        let languages = backend.supported_languages();
        // English should always be available
        assert!(languages.contains(&"eng".to_string()));
        // Should have at least English
        assert!(!languages.is_empty());
    }

    #[test]
    fn test_config_to_tesseract_with_none() {
        let backend = TesseractBackend::new();
        let ocr_config = OcrConfig {
            backend: "tesseract".to_string(),
            language: vec!["deu".to_string()],
            ..Default::default()
        };

        let tess_config = backend.config_to_tesseract(&ocr_config);
        assert_eq!(tess_config.language, "deu");
        assert_eq!(tess_config.psm, InternalTesseractConfig::default().psm);
    }

    #[test]
    fn test_config_to_tesseract_with_some() {
        let backend = TesseractBackend::new();
        let custom_tess_config = crate::types::TesseractConfig {
            language: vec!["fra".to_string()],
            psm: 6,
            enable_table_detection: true,
            ..Default::default()
        };

        let ocr_config = OcrConfig {
            backend: "tesseract".to_string(),
            language: vec!["eng".to_string()],
            tesseract_config: Some(custom_tess_config),
            ..Default::default()
        };

        let tess_config = backend.config_to_tesseract(&ocr_config);
        assert_eq!(tess_config.language, "fra");
        assert_eq!(tess_config.psm, 6);
        assert!(tess_config.enable_table_detection);
    }

    #[test]
    fn test_config_to_tesseract_defaults_empty_language_to_eng() {
        let backend = TesseractBackend::new();

        // No tesseract_config: empty language list (e.g. `language=[]`) must default to "eng"
        // rather than producing an empty language string. Regression test for the image OCR
        // path failing with "Failed to download language pack ''".
        let ocr_config = OcrConfig {
            backend: "tesseract".to_string(),
            language: vec![],
            ..Default::default()
        };
        assert_eq!(backend.config_to_tesseract(&ocr_config).language, "eng");

        // With a tesseract_config whose language is also empty, the same default applies.
        let ocr_config_with_tess = OcrConfig {
            backend: "tesseract".to_string(),
            language: vec![],
            tesseract_config: Some(crate::types::TesseractConfig {
                language: vec![],
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(backend.config_to_tesseract(&ocr_config_with_tess).language, "eng");
    }

    #[test]
    fn test_tesseract_backend_default() {
        let backend = TesseractBackend::default();
        assert_eq!(backend.name(), "tesseract");
    }

    #[test]
    fn test_config_conversion_with_new_fields() {
        let backend = TesseractBackend::new();

        let preprocessing = crate::types::ImagePreprocessingConfig {
            target_dpi: 600,
            auto_rotate: false,
            deskew: true,
            denoise: true,
            contrast_enhance: true,
            binarization_method: "adaptive".to_string(),
            invert_colors: false,
        };

        let custom_tess_config = crate::types::TesseractConfig {
            language: vec!["eng".to_string()],
            psm: 6,
            output_format: "markdown".to_string(),
            oem: 1,
            min_confidence: 80.0,
            preprocessing: Some(preprocessing.clone()),
            tessedit_char_blacklist: "!@#$".to_string(),
            ..Default::default()
        };

        let ocr_config = OcrConfig {
            backend: "tesseract".to_string(),
            language: vec!["eng".to_string()],
            tesseract_config: Some(custom_tess_config),
            ..Default::default()
        };

        let tess_config = backend.config_to_tesseract(&ocr_config);

        assert_eq!(tess_config.oem, 1);
        assert_eq!(tess_config.min_confidence, 80.0);
        assert_eq!(tess_config.tessedit_char_blacklist, "!@#$");

        assert!(tess_config.preprocessing.is_some());
        let preproc = tess_config.preprocessing.unwrap();
        assert_eq!(preproc.target_dpi, 600);
        assert!(!preproc.auto_rotate);
        assert!(preproc.deskew);
        assert!(preproc.denoise);
        assert!(preproc.contrast_enhance);
        assert_eq!(preproc.binarization_method, "adaptive");
        assert!(!preproc.invert_colors);
    }

    #[test]
    fn test_convert_config_type_conversions() {
        let public_config = crate::types::TesseractConfig {
            language: vec!["eng".to_string()],
            psm: 6,
            oem: 3,
            table_column_threshold: 100,
            ..Default::default()
        };

        let internal_config = InternalTesseractConfig::from(&public_config);

        assert_eq!(internal_config.psm, 6u8);
        assert_eq!(internal_config.oem, 3u8);
        assert_eq!(internal_config.table_column_threshold, 100u32);
    }

    #[test]
    fn tesseract_backend_does_not_eagerly_allocate_processor() {
        // Constructing TesseractBackend should not allocate the native Tesseract/Leptonica
        // handle. The processor is allocated lazily on first use.
        let backend = TesseractBackend::new();
        assert!(
            !backend.processor_is_initialized(),
            "TesseractBackend::new() should not eagerly allocate the processor"
        );
    }
}

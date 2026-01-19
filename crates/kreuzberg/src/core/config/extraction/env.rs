//! Environment variable override support for extraction configuration.
//!
//! This module provides functionality to apply environment variable overrides
//! to extraction configuration, allowing runtime configuration changes.

use crate::{KreuzbergError, Result};

use super::super::ocr::OcrConfig;
use super::super::processing::ChunkingConfig;
use super::core::ExtractionConfig;
use super::types::TokenReductionConfig;

impl ExtractionConfig {
    /// Apply environment variable overrides to configuration.
    ///
    /// Environment variables have the highest precedence and will override any values
    /// loaded from configuration files. This method supports the following environment variables:
    ///
    /// - `KREUZBERG_OCR_LANGUAGE`: OCR language (ISO 639-1 or 639-3 code, e.g., "eng", "fra", "deu")
    /// - `KREUZBERG_OCR_BACKEND`: OCR backend ("tesseract", "easyocr", or "paddleocr")
    /// - `KREUZBERG_CHUNKING_MAX_CHARS`: Maximum characters per chunk (positive integer)
    /// - `KREUZBERG_CHUNKING_MAX_OVERLAP`: Maximum overlap between chunks (non-negative integer)
    /// - `KREUZBERG_CACHE_ENABLED`: Cache enabled flag ("true" or "false")
    /// - `KREUZBERG_TOKEN_REDUCTION_MODE`: Token reduction mode ("off", "light", "moderate", "aggressive", or "maximum")
    ///
    /// # Behavior
    ///
    /// - If an environment variable is set and valid, it overrides the current configuration value
    /// - If a required parent config is `None` (e.g., `self.ocr` is None), it's created with defaults before applying the override
    /// - Invalid values return a `KreuzbergError::Validation` with helpful error messages
    /// - Missing or unset environment variables are silently ignored
    ///
    /// # Example
    ///
    /// ```rust
    /// # use kreuzberg::core::config::ExtractionConfig;
    /// # fn example() -> kreuzberg::Result<()> {
    /// let mut config = ExtractionConfig::from_file("config.toml")?;
    /// // Set KREUZBERG_OCR_LANGUAGE=fra before calling
    /// config.apply_env_overrides()?; // OCR language is now "fra"
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `KreuzbergError::Validation` if:
    /// - An environment variable contains an invalid value
    /// - A number cannot be parsed as the expected type
    /// - A boolean is not "true" or "false"
    pub fn apply_env_overrides(&mut self) -> Result<()> {
        use crate::core::config_validation::{
            validate_chunking_params, validate_language_code, validate_ocr_backend, validate_token_reduction_level,
        };

        // KREUZBERG_OCR_LANGUAGE override
        if let Ok(lang) = std::env::var("KREUZBERG_OCR_LANGUAGE") {
            validate_language_code(&lang)?;
            if self.ocr.is_none() {
                self.ocr = Some(OcrConfig::default());
            }
            if let Some(ref mut ocr) = self.ocr {
                ocr.language = lang;
            }
        }

        // KREUZBERG_OCR_BACKEND override
        if let Ok(backend) = std::env::var("KREUZBERG_OCR_BACKEND") {
            validate_ocr_backend(&backend)?;
            if self.ocr.is_none() {
                self.ocr = Some(OcrConfig::default());
            }
            if let Some(ref mut ocr) = self.ocr {
                ocr.backend = backend;
            }
        }

        // KREUZBERG_CHUNKING_MAX_CHARS override
        if let Ok(max_chars_str) = std::env::var("KREUZBERG_CHUNKING_MAX_CHARS") {
            let max_chars: usize = max_chars_str.parse().map_err(|_| KreuzbergError::Validation {
                message: format!(
                    "Invalid value for KREUZBERG_CHUNKING_MAX_CHARS: '{}'. Must be a positive integer.",
                    max_chars_str
                ),
                source: None,
            })?;

            if max_chars == 0 {
                return Err(KreuzbergError::Validation {
                    message: "KREUZBERG_CHUNKING_MAX_CHARS must be greater than 0".to_string(),
                    source: None,
                });
            }

            if self.chunking.is_none() {
                self.chunking = Some(ChunkingConfig {
                    max_chars: 1000,
                    max_overlap: 200,
                    embedding: None,
                    preset: None,
                });
            }

            if let Some(ref mut chunking) = self.chunking {
                // Validate against current overlap before updating
                validate_chunking_params(max_chars, chunking.max_overlap)?;
                chunking.max_chars = max_chars;
            }
        }

        // KREUZBERG_CHUNKING_MAX_OVERLAP override
        if let Ok(max_overlap_str) = std::env::var("KREUZBERG_CHUNKING_MAX_OVERLAP") {
            let max_overlap: usize = max_overlap_str.parse().map_err(|_| KreuzbergError::Validation {
                message: format!(
                    "Invalid value for KREUZBERG_CHUNKING_MAX_OVERLAP: '{}'. Must be a non-negative integer.",
                    max_overlap_str
                ),
                source: None,
            })?;

            if self.chunking.is_none() {
                self.chunking = Some(ChunkingConfig {
                    max_chars: 1000,
                    max_overlap: 200,
                    embedding: None,
                    preset: None,
                });
            }

            if let Some(ref mut chunking) = self.chunking {
                // Validate against current max_chars before updating
                validate_chunking_params(chunking.max_chars, max_overlap)?;
                chunking.max_overlap = max_overlap;
            }
        }

        // KREUZBERG_CACHE_ENABLED override
        if let Ok(cache_str) = std::env::var("KREUZBERG_CACHE_ENABLED") {
            let cache_enabled = match cache_str.to_lowercase().as_str() {
                "true" => true,
                "false" => false,
                _ => {
                    return Err(KreuzbergError::Validation {
                        message: format!(
                            "Invalid value for KREUZBERG_CACHE_ENABLED: '{}'. Must be 'true' or 'false'.",
                            cache_str
                        ),
                        source: None,
                    });
                }
            };
            self.use_cache = cache_enabled;
        }

        // KREUZBERG_TOKEN_REDUCTION_MODE override
        if let Ok(mode) = std::env::var("KREUZBERG_TOKEN_REDUCTION_MODE") {
            validate_token_reduction_level(&mode)?;
            if self.token_reduction.is_none() {
                self.token_reduction = Some(TokenReductionConfig {
                    mode: "off".to_string(),
                    preserve_important_words: true,
                });
            }
            if let Some(ref mut token_reduction) = self.token_reduction {
                token_reduction.mode = mode;
            }
        }

        // KREUZBERG_OUTPUT_FORMAT override
        if let Ok(val) = std::env::var("KREUZBERG_OUTPUT_FORMAT") {
            self.output_format = val.parse().map_err(|e: String| KreuzbergError::Validation {
                message: format!("Invalid value for KREUZBERG_OUTPUT_FORMAT: {}", e),
                source: None,
            })?;
        }

        Ok(())
    }
}

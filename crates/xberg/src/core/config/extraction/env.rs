//! Environment variable override support for extraction configuration.
//!
//! This module provides functionality to apply environment variable overrides
//! to extraction configuration, allowing runtime configuration changes.

use crate::core::config_validation::{
    validate_chunking_params, validate_language_code, validate_ocr_backend, validate_token_reduction_level,
};
use crate::{Result, XbergError};

use super::super::csv::CsvConfig;
use super::super::ocr::OcrConfig;
use super::super::processing::ChunkingConfig;
use super::core::ExtractionConfig;
use super::types::TokenReductionOptions;

impl ExtractionConfig {
    /// Apply environment variable overrides to configuration.
    ///
    /// Environment variables have the highest precedence and will override any values
    /// loaded from configuration files. This method supports the following environment variables:
    ///
    /// - `XBERG_OCR_LANGUAGE`: OCR language (ISO 639-1 or 639-3 code, e.g., "eng", "fra", "deu")
    /// - `XBERG_OCR_BACKEND`: OCR backend ("tesseract", "paddleocr", "paddle-ocr", or "vlm")
    /// - `XBERG_OCR_MODEL_VERSION`: PaddleOCR model generation ("pp-ocrv6" or "pp-ocrv5")
    /// - `XBERG_OCR_MODEL_TIER`: PaddleOCR model tier (e.g. "medium"/"small"/"tiny" for v6, "mobile"/"server" for v5)
    /// - `XBERG_OCR_TESSDATA_MODEL`: Tesseract model set ("fast" or "best"), applied to the
    ///   top-level `tessdata_model` so it never creates an `ocr` section
    /// - `XBERG_CHUNKING_MAX_CHARS`: Maximum characters per chunk (positive integer)
    /// - `XBERG_CHUNKING_MAX_OVERLAP`: Maximum overlap between chunks (non-negative integer)
    /// - `XBERG_CACHE_ENABLED`: Cache enabled flag ("true" or "false")
    /// - `XBERG_TOKEN_REDUCTION_MODE`: Token reduction mode ("off", "light", "moderate", "aggressive", or "maximum")
    /// - `XBERG_CHUNKING_TOKENIZER`: HuggingFace tokenizer model ID for token-based chunk sizing (requires `chunking-tokenizers` feature)
    /// - `XBERG_DISABLE_OCR`: Disable OCR entirely ("true" or "false")
    /// - `XBERG_STRICT_PARSE`: Fail instead of falling back to plain text when YAML/JSON/TOML/XML parsing fails ("true" or "false")
    /// - `XBERG_LLM_MODEL`: LLM model for structured extraction (e.g., "openai/gpt-4o")
    /// - `XBERG_LLM_API_KEY`: API key for the structured extraction LLM provider. Applied only
    ///   when structured extraction is already configured — a credential never enables it.
    /// - `XBERG_LLM_BASE_URL`: Custom base URL for the structured extraction LLM provider. Applied
    ///   only when structured extraction is already configured — an endpoint never enables it.
    /// - `XBERG_VLM_OCR_MODEL`: VLM model for vision-based OCR (e.g., "openai/gpt-4o")
    /// - `XBERG_VLM_EMBEDDING_MODEL`: LLM model for embedding generation (e.g., "openai/text-embedding-3-small")
    /// - `XBERG_EMBEDDING_PLUGIN_NAME`: Name of an in-process embedding backend registered via `plugins::register_embedding_backend`
    /// - `XBERG_MSG_FALLBACK_CODEPAGE`: (deferred) Windows codepage for MSG PT_STRING8 fallback
    /// - `XBERG_CSV_DELIMITER`: CSV/TSV field delimiter, exactly one ASCII character
    ///   (e.g. ",", ";", "\t", "|"). Overrides delimiter auto-detection.
    /// - `XBERG_CSV_COMMENT_PREFIXES`: comma-separated list of line prefixes marking a
    ///   CSV/TSV comment line to skip (e.g. "#,//")
    ///
    /// # Behavior
    ///
    /// - If an environment variable is set and valid, it overrides the current configuration value
    /// - If a required parent config is `None` (e.g., `self.ocr` is None), it's created with
    ///   defaults before applying the override. The exception is credentials and endpoints —
    ///   `XBERG_LLM_API_KEY` and `XBERG_LLM_BASE_URL` — which are applied to an existing config
    ///   or ignored, and never enable a feature that was not asked for (issue #1421)
    /// - Invalid values return a `XbergError::Validation` with helpful error messages
    /// - Missing or unset environment variables are silently ignored
    ///
    /// # Example
    ///
    /// ```rust
    /// # use xberg::core::config::ExtractionConfig;
    /// # fn example() -> xberg::Result<()> {
    /// let mut config = ExtractionConfig::from_file("config.toml")?;
    /// // Set XBERG_OCR_LANGUAGE=fra before calling
    /// config.apply_env_overrides()?; // OCR language is now "fra"
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `XbergError::Validation` if:
    /// - An environment variable contains an invalid value
    /// - A number cannot be parsed as the expected type
    /// - A boolean is not "true" or "false"
    pub fn apply_env_overrides(&mut self) -> Result<()> {
        self.apply_env_ocr_overrides()?;
        self.apply_env_paddle_ocr_overrides()?;
        self.apply_env_chunk_size_overrides()?;
        self.apply_env_chunk_overlap_overrides()?;
        self.apply_env_output_overrides()?;
        #[cfg(feature = "chunking-tokenizers")]
        self.apply_env_tokenizer_overrides()?;
        #[cfg(feature = "layout-detection")]
        self.apply_env_layout_overrides()?;
        self.apply_env_parse_flags_overrides()?;
        self.apply_env_structured_model_overrides()?;
        self.apply_env_structured_credentials_overrides()?;
        self.apply_env_vlm_ocr_model_overrides()?;
        self.apply_env_vlm_ocr_credentials_overrides();
        self.apply_env_vlm_embedding_overrides()?;
        self.apply_env_embedding_plugin_overrides()?;
        self.apply_env_csv_overrides()?;
        Ok(())
    }

    fn apply_env_ocr_overrides(&mut self) -> Result<()> {
        if let Ok(lang) = std::env::var("XBERG_OCR_LANGUAGE") {
            validate_language_code(&lang)?;
            if self.ocr.is_none() {
                self.ocr = Some(OcrConfig::default());
            }
            if let Some(ref mut ocr) = self.ocr {
                ocr.language = vec![lang];
            }
        }

        if let Ok(backend) = std::env::var("XBERG_OCR_BACKEND") {
            validate_ocr_backend(&backend)?;
            if self.ocr.is_none() {
                self.ocr = Some(OcrConfig::default());
            }
            if let Some(ref mut ocr) = self.ocr {
                ocr.backend = backend;
            }
        }

        if let Ok(model) = std::env::var("XBERG_OCR_TESSDATA_MODEL") {
            let model = model.parse().map_err(|message: String| XbergError::Validation {
                message: format!("Invalid value for XBERG_OCR_TESSDATA_MODEL: {message}"),
                source: None,
            })?;
            // ~keep The top-level set reaches every OCR this extraction performs without creating
            // an `ocr` section, which would change whether OCR runs; an `ocr` section's own set
            // is cleared so the environment wins over the config file.
            self.tessdata_model = Some(model);
            if let Some(ocr) = self.ocr.as_mut() {
                ocr.tessdata_model = None;
            }
        }
        Ok(())
    }

    fn apply_env_paddle_ocr_overrides(&mut self) -> Result<()> {
        let paddle_model_version = std::env::var("XBERG_OCR_MODEL_VERSION").ok();
        let paddle_model_tier = std::env::var("XBERG_OCR_MODEL_TIER").ok();
        if paddle_model_version.is_some() || paddle_model_tier.is_some() {
            if self.ocr.is_none() {
                self.ocr = Some(OcrConfig::default());
            }
            if let Some(ref mut ocr) = self.ocr {
                let mut paddle = super::super::ocr::resolve_paddle_ocr_settings(
                    ocr.paddle_ocr_settings.as_ref(),
                    ocr.paddle_ocr_config.as_ref(),
                )?
                .unwrap_or_default();
                if let Some(version) = paddle_model_version {
                    paddle.model_version = version;
                }
                if let Some(tier) = paddle_model_tier {
                    paddle.model_tier = tier;
                }
                ocr.paddle_ocr_settings = Some(paddle);
            }
        }
        Ok(())
    }

    fn apply_env_chunk_size_overrides(&mut self) -> Result<()> {
        if let Ok(max_chars_str) = std::env::var("XBERG_CHUNKING_MAX_CHARS") {
            let max_chars: usize = max_chars_str.parse().map_err(|_| XbergError::Validation {
                message: format!(
                    "Invalid value for XBERG_CHUNKING_MAX_CHARS: '{}'. Must be a positive integer.",
                    max_chars_str
                ),
                source: None,
            })?;

            if max_chars == 0 {
                return Err(XbergError::Validation {
                    message: "XBERG_CHUNKING_MAX_CHARS must be greater than 0".to_string(),
                    source: None,
                });
            }

            if self.chunking.is_none() {
                self.chunking = Some(ChunkingConfig::default());
            }

            if let Some(ref mut chunking) = self.chunking {
                validate_chunking_params(max_chars, chunking.overlap)?;
                chunking.max_characters = max_chars;
            }
        }
        Ok(())
    }

    fn apply_env_chunk_overlap_overrides(&mut self) -> Result<()> {
        if let Ok(max_overlap_str) = std::env::var("XBERG_CHUNKING_MAX_OVERLAP") {
            let max_overlap: usize = max_overlap_str.parse().map_err(|_| XbergError::Validation {
                message: format!(
                    "Invalid value for XBERG_CHUNKING_MAX_OVERLAP: '{}'. Must be a non-negative integer.",
                    max_overlap_str
                ),
                source: None,
            })?;

            if self.chunking.is_none() {
                self.chunking = Some(ChunkingConfig::default());
            }

            if let Some(ref mut chunking) = self.chunking {
                validate_chunking_params(chunking.max_characters, max_overlap)?;
                chunking.overlap = max_overlap;
            }
        }
        Ok(())
    }

    fn apply_env_output_overrides(&mut self) -> Result<()> {
        if let Ok(cache_str) = std::env::var("XBERG_CACHE_ENABLED") {
            let cache_enabled = match cache_str.to_lowercase().as_str() {
                "true" => true,
                "false" => false,
                _ => {
                    return Err(XbergError::Validation {
                        message: format!(
                            "Invalid value for XBERG_CACHE_ENABLED: '{}'. Must be 'true' or 'false'.",
                            cache_str
                        ),
                        source: None,
                    });
                }
            };
            self.use_cache = cache_enabled;
        }

        if let Ok(mode) = std::env::var("XBERG_TOKEN_REDUCTION_MODE") {
            validate_token_reduction_level(&mode)?;
            if self.token_reduction.is_none() {
                self.token_reduction = Some(TokenReductionOptions {
                    mode: "off".to_string(),
                    preserve_important_words: true,
                });
            }
            if let Some(ref mut token_reduction) = self.token_reduction {
                token_reduction.mode = mode;
            }
        }

        if let Ok(val) = std::env::var("XBERG_OUTPUT_FORMAT") {
            self.output_format = val.parse().map_err(|e: String| XbergError::Validation {
                message: format!("Invalid value for XBERG_OUTPUT_FORMAT: {}", e),
                source: None,
            })?;
        }
        Ok(())
    }

    #[cfg(feature = "chunking-tokenizers")]
    fn apply_env_tokenizer_overrides(&mut self) -> Result<()> {
        if let Ok(model) = std::env::var("XBERG_CHUNKING_TOKENIZER") {
            if model.is_empty() {
                return Err(XbergError::Validation {
                    message: "XBERG_CHUNKING_TOKENIZER must not be empty".to_string(),
                    source: None,
                });
            }

            if self.chunking.is_none() {
                self.chunking = Some(ChunkingConfig::default());
            }

            if let Some(ref mut chunking) = self.chunking {
                chunking.sizing = crate::core::config::processing::ChunkSizing::Tokenizer { model, cache_dir: None };
            }
        }
        Ok(())
    }

    #[cfg(feature = "layout-detection")]
    fn apply_env_layout_overrides(&mut self) -> Result<()> {
        if let Ok(preset) = std::env::var("XBERG_LAYOUT_PRESET") {
            let lower = preset.to_lowercase();
            if !["fast", "accurate", "yolo", "rtdetr", "rt-detr"].contains(&lower.as_str()) {
                return Err(XbergError::Validation {
                    message: format!(
                        "Invalid value for XBERG_LAYOUT_PRESET: '{}'. Valid presets: fast, accurate",
                        preset
                    ),
                    source: None,
                });
            }
            if self.layout.is_none() {
                self.layout = Some(super::super::layout::LayoutDetectionConfig::default());
            }
            let _ = lower;
        }
        Ok(())
    }

    fn apply_env_parse_flags_overrides(&mut self) -> Result<()> {
        if let Ok(val) = std::env::var("XBERG_DISABLE_OCR") {
            self.disable_ocr = match val.to_lowercase().as_str() {
                "true" | "1" => true,
                "false" | "0" => false,
                _ => {
                    return Err(XbergError::Validation {
                        message: format!(
                            "Invalid value for XBERG_DISABLE_OCR: '{}'. Must be 'true' or 'false'.",
                            val
                        ),
                        source: None,
                    });
                }
            };
        }

        if let Ok(val) = std::env::var("XBERG_STRICT_PARSE") {
            self.strict_parse = match val.to_lowercase().as_str() {
                "true" | "1" => true,
                "false" | "0" => false,
                _ => {
                    return Err(XbergError::Validation {
                        message: format!(
                            "Invalid value for XBERG_STRICT_PARSE: '{}'. Must be 'true' or 'false'.",
                            val
                        ),
                        source: None,
                    });
                }
            };
        }
        Ok(())
    }

    fn apply_env_structured_model_overrides(&mut self) -> Result<()> {
        if let Ok(value) = std::env::var("XBERG_LLM_MODEL") {
            if value.is_empty() {
                return Err(XbergError::Validation {
                    message: "XBERG_LLM_MODEL must not be empty".to_string(),
                    source: None,
                });
            }
            if self.structured_extraction.is_none() {
                self.structured_extraction = Some(super::super::llm::StructuredExtractionConfig {
                    schema: serde_json::Value::Object(Default::default()),
                    schema_name: "extraction".to_string(),
                    schema_description: None,
                    strict: false,
                    prompt: None,
                    llm: super::super::llm::LlmConfig {
                        model: value,
                        api_key: None,
                        base_url: None,
                        ..Default::default()
                    },
                });
            } else if let Some(ref mut config) = self.structured_extraction {
                config.llm.model = value;
            }
        }
        Ok(())
    }

    fn apply_env_structured_credentials_overrides(&mut self) -> Result<()> {
        // Credentials configure an existing feature and never enable it (#1421). ~keep
        if let Ok(value) = std::env::var("XBERG_LLM_API_KEY") {
            if value.is_empty() {
                return Err(XbergError::Validation {
                    message: "XBERG_LLM_API_KEY must not be empty".to_string(),
                    source: None,
                });
            }
            if let Some(ref mut config) = self.structured_extraction {
                config.llm.api_key = Some(value);
            }
        }

        if let Ok(value) = std::env::var("XBERG_LLM_BASE_URL") {
            if value.is_empty() {
                return Err(XbergError::Validation {
                    message: "XBERG_LLM_BASE_URL must not be empty".to_string(),
                    source: None,
                });
            }
            if let Some(ref mut config) = self.structured_extraction {
                config.llm.base_url = Some(value);
            }
        }
        Ok(())
    }

    fn apply_env_vlm_ocr_model_overrides(&mut self) -> Result<()> {
        if let Ok(value) = std::env::var("XBERG_VLM_OCR_MODEL") {
            if value.is_empty() {
                return Err(XbergError::Validation {
                    message: "XBERG_VLM_OCR_MODEL must not be empty".to_string(),
                    source: None,
                });
            }
            if self.ocr.is_none() {
                self.ocr = Some(OcrConfig::default());
            }
            if let Some(ref mut ocr) = self.ocr {
                if ocr.vlm_config.is_none() {
                    ocr.vlm_config = Some(super::super::llm::LlmConfig {
                        model: value,
                        ..Default::default()
                    });
                } else if let Some(ref mut vlm) = ocr.vlm_config {
                    vlm.model = value;
                }
            }
        }
        Ok(())
    }

    fn apply_env_vlm_ocr_credentials_overrides(&mut self) {
        // Run after the VLM model override so newly created configs receive credentials (#1339). ~keep
        if let Some(ref mut ocr) = self.ocr
            && let Some(ref mut vlm) = ocr.vlm_config
        {
            if let Ok(value) = std::env::var("XBERG_LLM_API_KEY")
                && !value.is_empty()
            {
                vlm.api_key = Some(value);
            }
            if let Ok(value) = std::env::var("XBERG_LLM_BASE_URL")
                && !value.is_empty()
            {
                vlm.base_url = Some(value);
            }
        }
    }

    fn apply_env_vlm_embedding_overrides(&mut self) -> Result<()> {
        if let Ok(value) = std::env::var("XBERG_VLM_EMBEDDING_MODEL") {
            if value.is_empty() {
                return Err(XbergError::Validation {
                    message: "XBERG_VLM_EMBEDDING_MODEL must not be empty".to_string(),
                    source: None,
                });
            }
            if self.chunking.is_none() {
                self.chunking = Some(ChunkingConfig::default());
            }
            if let Some(ref mut chunking) = self.chunking {
                chunking.embedding = Some(super::super::processing::EmbeddingConfig {
                    model: super::super::processing::EmbeddingModelType::Llm {
                        llm: Box::new(super::super::llm::LlmConfig {
                            model: value,
                            api_key: None,
                            base_url: None,
                            ..Default::default()
                        }),
                    },
                    ..super::super::processing::EmbeddingConfig::default()
                });
            }
        }
        Ok(())
    }

    fn apply_env_embedding_plugin_overrides(&mut self) -> Result<()> {
        let plugin_name = std::env::var("XBERG_EMBEDDING_PLUGIN_NAME").ok();
        if plugin_name.is_some() && std::env::var("XBERG_VLM_EMBEDDING_MODEL").is_ok() {
            return Err(XbergError::Validation {
                message:
                    "XBERG_EMBEDDING_PLUGIN_NAME and XBERG_VLM_EMBEDDING_MODEL are mutually exclusive — set one or the other, not both."
                        .to_string(),
                source: None,
            });
        }
        if let Some(value) = plugin_name {
            if value.is_empty() {
                return Err(XbergError::Validation {
                    message: "XBERG_EMBEDDING_PLUGIN_NAME must not be empty".to_string(),
                    source: None,
                });
            }
            if self.chunking.is_none() {
                self.chunking = Some(ChunkingConfig::default());
            }
            if let Some(ref mut chunking) = self.chunking {
                chunking.embedding = Some(super::super::processing::EmbeddingConfig {
                    model: super::super::processing::EmbeddingModelType::Plugin { name: value },
                    ..super::super::processing::EmbeddingConfig::default()
                });
            }
        }
        Ok(())
    }

    fn apply_env_csv_overrides(&mut self) -> Result<()> {
        if let Ok(value) = std::env::var("XBERG_CSV_DELIMITER") {
            crate::core::config_validation::validate_csv_delimiter(&value)?;
            if self.csv.is_none() {
                self.csv = Some(CsvConfig::default());
            }
            if let Some(ref mut csv) = self.csv {
                csv.delimiter = Some(value);
            }
        }

        if let Ok(value) = std::env::var("XBERG_CSV_COMMENT_PREFIXES") {
            let prefixes: Vec<String> = value
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect();
            if prefixes.is_empty() {
                return Err(XbergError::Validation {
                    message: "XBERG_CSV_COMMENT_PREFIXES must contain at least one non-empty prefix".to_string(),
                    source: None,
                });
            }
            if self.csv.is_none() {
                self.csv = Some(CsvConfig::default());
            }
            if let Some(ref mut csv) = self.csv {
                csv.comment_prefixes = prefixes;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "env_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "env_lint_tests.rs"]
mod lint_tests;

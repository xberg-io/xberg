//! OCR configuration.
//!
//! Defines OCR-specific configuration including backend selection, language settings,
//! Tesseract-specific parameters, quality thresholds, and multi-backend pipeline config.

use serde::{Deserialize, Serialize};

use super::formats::OutputFormat;
use crate::core::config_validation::validate_ocr_backend;
use crate::error::KreuzbergError;
use crate::types::OcrElementConfig;

/// Quality thresholds for OCR fallback decisions and pipeline quality gating.
///
/// All fields default to the values that match the previous hardcoded behavior,
/// so `OcrQualityThresholds::default()` preserves existing semantics exactly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrQualityThresholds {
    /// Minimum total non-whitespace characters to consider text substantive.
    #[serde(default = "default_min_total_non_whitespace")]
    pub min_total_non_whitespace: usize,

    /// Minimum non-whitespace characters per page on average.
    #[serde(default = "default_min_non_whitespace_per_page")]
    pub min_non_whitespace_per_page: f64,

    /// Minimum character count for a word to be "meaningful".
    #[serde(default = "default_min_meaningful_word_len")]
    pub min_meaningful_word_len: usize,

    /// Minimum count of meaningful words before text is accepted.
    #[serde(default = "default_min_meaningful_words")]
    pub min_meaningful_words: usize,

    /// Minimum alphanumeric ratio (non-whitespace chars that are alphanumeric).
    #[serde(default = "default_min_alnum_ratio")]
    pub min_alnum_ratio: f64,

    /// Minimum Unicode replacement characters (U+FFFD) to trigger OCR fallback.
    #[serde(default = "default_min_garbage_chars")]
    pub min_garbage_chars: usize,

    /// Maximum fraction of short (1-2 char) words before text is considered fragmented.
    #[serde(default = "default_max_fragmented_word_ratio")]
    pub max_fragmented_word_ratio: f64,

    /// Critical fragmentation threshold — triggers OCR regardless of meaningful words.
    /// Normal English text has ~20-30% short words. 80%+ is definitive garbage.
    #[serde(default = "default_critical_fragmented_word_ratio")]
    pub critical_fragmented_word_ratio: f64,

    /// Minimum average word length. Below this with enough words indicates garbled extraction.
    #[serde(default = "default_min_avg_word_length")]
    pub min_avg_word_length: f64,

    /// Minimum word count before average word length check applies.
    #[serde(default = "default_min_words_for_avg_length_check")]
    pub min_words_for_avg_length_check: usize,

    /// Minimum consecutive word repetition ratio to detect column scrambling.
    #[serde(default = "default_min_consecutive_repeat_ratio")]
    pub min_consecutive_repeat_ratio: f64,

    /// Minimum word count before consecutive repetition check is applied.
    #[serde(default = "default_min_words_for_repeat_check")]
    pub min_words_for_repeat_check: usize,

    /// Minimum character count for "substantive markdown" OCR skip gate.
    #[serde(default = "default_substantive_min_chars")]
    pub substantive_min_chars: usize,

    /// Minimum character count for "non-text content" OCR skip gate.
    #[serde(default = "default_non_text_min_chars")]
    pub non_text_min_chars: usize,

    /// Alphanumeric+whitespace ratio threshold for skip decisions.
    #[serde(default = "default_alnum_ws_ratio_threshold")]
    pub alnum_ws_ratio_threshold: f64,

    /// Minimum quality score (0.0-1.0) for a pipeline stage result to be accepted.
    /// If the result from a backend scores below this, try the next backend.
    #[serde(default = "default_pipeline_min_quality")]
    pub pipeline_min_quality: f64,
}

impl Default for OcrQualityThresholds {
    fn default() -> Self {
        Self {
            min_total_non_whitespace: 64,
            min_non_whitespace_per_page: 32.0,
            min_meaningful_word_len: 4,
            min_meaningful_words: 3,
            min_alnum_ratio: 0.3,
            min_garbage_chars: 5,
            max_fragmented_word_ratio: 0.6,
            critical_fragmented_word_ratio: 0.80,
            min_avg_word_length: 2.0,
            min_words_for_avg_length_check: 50,
            min_consecutive_repeat_ratio: 0.08,
            min_words_for_repeat_check: 50,
            substantive_min_chars: 100,
            non_text_min_chars: 20,
            alnum_ws_ratio_threshold: 0.4,
            pipeline_min_quality: 0.5,
        }
    }
}

fn default_min_total_non_whitespace() -> usize {
    64
}
fn default_min_non_whitespace_per_page() -> f64 {
    32.0
}
fn default_min_meaningful_word_len() -> usize {
    4
}
fn default_min_meaningful_words() -> usize {
    3
}
fn default_min_alnum_ratio() -> f64 {
    0.3
}
fn default_min_garbage_chars() -> usize {
    5
}
fn default_max_fragmented_word_ratio() -> f64 {
    0.6
}
fn default_critical_fragmented_word_ratio() -> f64 {
    0.80
}
fn default_min_avg_word_length() -> f64 {
    2.0
}
fn default_min_words_for_avg_length_check() -> usize {
    50
}
fn default_min_consecutive_repeat_ratio() -> f64 {
    0.08
}
fn default_min_words_for_repeat_check() -> usize {
    50
}
fn default_substantive_min_chars() -> usize {
    100
}
fn default_non_text_min_chars() -> usize {
    20
}
fn default_alnum_ws_ratio_threshold() -> f64 {
    0.4
}
fn default_pipeline_min_quality() -> f64 {
    0.5
}

/// A single backend stage in the OCR pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrPipelineStage {
    /// Backend name: "tesseract", "paddleocr", "easyocr", or a custom registered name.
    pub backend: String,

    /// Priority weight (higher = tried first). Stages are sorted by priority descending.
    #[serde(default = "default_priority")]
    pub priority: u32,

    /// Language override for this stage (None = use parent OcrConfig.language).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,

    /// Tesseract-specific config override for this stage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tesseract_config: Option<crate::types::TesseractConfig>,

    /// PaddleOCR-specific config for this stage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paddle_ocr_config: Option<serde_json::Value>,

    /// VLM config override for this pipeline stage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vlm_config: Option<super::llm::LlmConfig>,
}

fn default_priority() -> u32 {
    100
}

/// Multi-backend OCR pipeline with quality-based fallback.
///
/// Backends are tried in priority order (highest first). After each backend
/// produces output, quality is evaluated. If it meets `quality_thresholds.pipeline_min_quality`,
/// the result is accepted. Otherwise the next backend is tried.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrPipelineConfig {
    /// Ordered list of backends to try. Sorted by priority (descending) at runtime.
    pub stages: Vec<OcrPipelineStage>,

    /// Quality thresholds for deciding whether to accept a result or try the next backend.
    #[serde(default)]
    pub quality_thresholds: OcrQualityThresholds,
}

/// OCR configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrConfig {
    /// Whether OCR is enabled.
    ///
    /// Setting `enabled: false` is a shorthand for `disable_ocr: true` on the parent
    /// [`ExtractionConfig`](crate::core::config::ExtractionConfig). Images return
    /// metadata only; PDFs use native text extraction without OCR fallback.
    ///
    /// Defaults to `true`. When `false`, all other OCR settings are ignored.
    #[serde(default = "default_ocr_enabled")]
    pub enabled: bool,

    /// OCR backend: tesseract, easyocr, paddleocr
    #[serde(default = "default_tesseract_backend")]
    pub backend: String,

    /// Language code (e.g., "eng", "deu")
    #[serde(default = "default_eng")]
    pub language: String,

    /// Tesseract-specific configuration (optional)
    #[serde(default)]
    pub tesseract_config: Option<crate::types::TesseractConfig>,

    /// Output format for OCR results (optional, for format conversion)
    #[serde(default)]
    pub output_format: Option<OutputFormat>,

    /// PaddleOCR-specific configuration (optional, JSON passthrough)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paddle_ocr_config: Option<serde_json::Value>,

    /// OCR element extraction configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub element_config: Option<OcrElementConfig>,

    /// Quality thresholds for the native-text-to-OCR fallback decision.
    /// When None, uses compiled defaults (matching previous hardcoded behavior).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality_thresholds: Option<OcrQualityThresholds>,

    /// Multi-backend OCR pipeline configuration. When set, enables weighted
    /// fallback across multiple OCR backends based on output quality.
    /// When None, uses the single `backend` field (same as today).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pipeline: Option<OcrPipelineConfig>,

    /// Enable automatic page rotation based on orientation detection.
    ///
    /// When enabled, uses Tesseract's `DetectOrientationScript()` to detect
    /// page orientation (0/90/180/270 degrees) before OCR. If the page is
    /// rotated with high confidence, the image is corrected before recognition.
    /// This is critical for handling rotated scanned documents.
    #[serde(default)]
    pub auto_rotate: bool,

    /// VLM (Vision Language Model) OCR configuration.
    ///
    /// Required when `backend` is `"vlm"`. Uses liter-llm to send page
    /// images to a vision model for text extraction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vlm_config: Option<super::llm::LlmConfig>,

    /// Custom Jinja2 prompt template for VLM OCR.
    ///
    /// When `None`, uses the default template. Available variables:
    /// - `{{ language }}` — The document language code (e.g., "eng", "deu").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vlm_prompt: Option<String>,

    /// Hardware acceleration for ONNX Runtime models (e.g. PaddleOCR, layout detection).
    ///
    /// Not user-configurable via config files — injected at runtime from
    /// `ExtractionConfig::acceleration` before each `process_image` call.
    #[serde(skip)]
    pub acceleration: Option<super::acceleration::AccelerationConfig>,
}

impl Default for OcrConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            backend: default_tesseract_backend(),
            language: default_eng(),
            tesseract_config: None,
            output_format: None,
            paddle_ocr_config: None,
            element_config: None,
            quality_thresholds: None,
            pipeline: None,
            auto_rotate: false,
            vlm_config: None,
            vlm_prompt: None,
            acceleration: None,
        }
    }
}

impl OcrConfig {
    /// Validates that the configured backend is supported.
    ///
    /// This method checks that the backend name is one of the supported OCR backends:
    /// - tesseract
    /// - easyocr
    /// - paddleocr
    ///
    /// Typos in backend names are caught at configuration validation time, not at runtime.
    /// Also validates pipeline stage backends when a pipeline is configured.
    pub fn validate(&self) -> Result<(), KreuzbergError> {
        validate_ocr_backend(&self.backend)?;
        // When backend is "vlm", vlm_config must be present.
        crate::core::config_validation::validate_vlm_backend_config(&self.backend, self.vlm_config.as_ref())?;
        if let Some(ref pipeline) = self.pipeline {
            for stage in &pipeline.stages {
                validate_ocr_backend(&stage.backend)?;
                crate::core::config_validation::validate_vlm_backend_config(&stage.backend, stage.vlm_config.as_ref())?;
            }
        }
        Ok(())
    }

    /// Returns the effective quality thresholds, using configured values or defaults.
    pub fn effective_thresholds(&self) -> OcrQualityThresholds {
        self.quality_thresholds.clone().unwrap_or_default()
    }

    /// Returns the effective pipeline config.
    ///
    /// - If `pipeline` is explicitly set, returns it.
    /// - If `paddle-ocr` is compiled in and the backend is the default
    ///   (tesseract), auto-constructs `[tesseract @ 100, paddleocr @ 50]`.
    /// - Otherwise returns `None` (single-backend mode).
    ///
    /// Explicit non-default backend selections are honored as-is — a silent
    /// paddleocr fallback would mask errors from the chosen backend.
    pub fn effective_pipeline(&self) -> Option<OcrPipelineConfig> {
        if self.pipeline.is_some() {
            return self.pipeline.clone();
        }

        #[cfg(feature = "paddle-ocr")]
        {
            if self.backend != default_tesseract_backend() {
                return None;
            }

            let stages = vec![
                OcrPipelineStage {
                    backend: self.backend.clone(),
                    priority: 100,
                    language: None,
                    tesseract_config: self.tesseract_config.clone(),
                    paddle_ocr_config: None,
                    vlm_config: self.vlm_config.clone(),
                },
                OcrPipelineStage {
                    backend: "paddleocr".to_string(),
                    priority: 50,
                    language: None,
                    tesseract_config: None,
                    paddle_ocr_config: self.paddle_ocr_config.clone(),
                    vlm_config: None,
                },
            ];
            Some(OcrPipelineConfig {
                stages,
                quality_thresholds: self.effective_thresholds(),
            })
        }

        #[cfg(not(feature = "paddle-ocr"))]
        {
            None
        }
    }
}

fn default_ocr_enabled() -> bool {
    true
}

fn default_tesseract_backend() -> String {
    "tesseract".to_string()
}

fn default_eng() -> String {
    "eng".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ocr_config_default() {
        let config = OcrConfig::default();
        assert_eq!(config.backend, "tesseract");
        assert_eq!(config.language, "eng");
        assert!(config.tesseract_config.is_none());
        assert!(config.output_format.is_none());
    }

    #[test]
    fn test_ocr_config_with_tesseract() {
        let config = OcrConfig {
            backend: "tesseract".to_string(),
            language: "fra".to_string(),
            ..Default::default()
        };
        assert_eq!(config.backend, "tesseract");
        assert_eq!(config.language, "fra");
    }

    #[test]
    fn test_validate_tesseract_backend() {
        let config = OcrConfig {
            backend: "tesseract".to_string(),
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_easyocr_backend() {
        let config = OcrConfig {
            backend: "easyocr".to_string(),
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_paddleocr_backend() {
        let config = OcrConfig {
            backend: "paddleocr".to_string(),
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_backend_typo() {
        let config = OcrConfig {
            backend: "tesseract_typo".to_string(),
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Invalid OCR backend"));
    }

    #[test]
    fn test_validate_invalid_backend_completely_wrong() {
        let config = OcrConfig {
            backend: "ocr_lib".to_string(),
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Invalid OCR backend") || err_msg.contains("Valid options are"));
    }

    #[test]
    fn test_validate_default_backend() {
        let config = OcrConfig::default();
        assert!(config.validate().is_ok());
    }

    // ── effective_pipeline tests ──

    #[test]
    fn test_effective_pipeline_explicit_pipeline_returned_unchanged() {
        let explicit_pipeline = OcrPipelineConfig {
            stages: vec![OcrPipelineStage {
                backend: "easyocr".to_string(),
                priority: 200,
                language: Some("fra".to_string()),
                tesseract_config: None,
                paddle_ocr_config: None,
                vlm_config: None,
            }],
            quality_thresholds: OcrQualityThresholds::default(),
        };
        let config = OcrConfig {
            pipeline: Some(explicit_pipeline.clone()),
            ..Default::default()
        };
        let result = config.effective_pipeline().unwrap();
        assert_eq!(result.stages.len(), 1);
        assert_eq!(result.stages[0].backend, "easyocr");
        assert_eq!(result.stages[0].priority, 200);
        assert_eq!(result.stages[0].language, Some("fra".to_string()));
    }

    #[test]
    fn test_effective_pipeline_explicit_paddleocr_no_autofallback() {
        let config = OcrConfig {
            backend: "paddleocr".to_string(),
            ..Default::default()
        };
        assert!(config.effective_pipeline().is_none());
    }

    #[test]
    fn test_effective_pipeline_explicit_easyocr_no_autofallback() {
        let config = OcrConfig {
            backend: "easyocr".to_string(),
            ..Default::default()
        };
        assert!(config.effective_pipeline().is_none());
    }

    #[test]
    fn test_effective_pipeline_default_tesseract_backend() {
        let config = OcrConfig::default();
        let result = config.effective_pipeline();
        #[cfg(feature = "paddle-ocr")]
        {
            let pipeline = result.unwrap();
            assert_eq!(pipeline.stages.len(), 2);
            assert_eq!(pipeline.stages[0].backend, "tesseract");
            assert_eq!(pipeline.stages[0].priority, 100);
            assert_eq!(pipeline.stages[1].backend, "paddleocr");
            assert_eq!(pipeline.stages[1].priority, 50);
        }
        #[cfg(not(feature = "paddle-ocr"))]
        {
            assert!(result.is_none());
        }
    }

    #[test]
    fn test_effective_thresholds_custom_vs_default() {
        // With custom thresholds
        let custom = OcrQualityThresholds {
            min_total_non_whitespace: 128,
            min_meaningful_words: 10,
            ..Default::default()
        };
        let config_custom = OcrConfig {
            quality_thresholds: Some(custom.clone()),
            ..Default::default()
        };
        let eff = config_custom.effective_thresholds();
        assert_eq!(eff.min_total_non_whitespace, 128);
        assert_eq!(eff.min_meaningful_words, 10);

        // Without custom thresholds (should return defaults)
        let config_default = OcrConfig::default();
        let eff_default = config_default.effective_thresholds();
        assert_eq!(eff_default.min_total_non_whitespace, 64);
        assert_eq!(eff_default.min_meaningful_words, 3);
    }

    // ── Serde tests ──

    #[test]
    fn test_pipeline_config_serde_roundtrip() {
        let pipeline = OcrPipelineConfig {
            stages: vec![
                OcrPipelineStage {
                    backend: "tesseract".to_string(),
                    priority: 100,
                    language: Some("eng".to_string()),
                    tesseract_config: None,
                    paddle_ocr_config: None,
                    vlm_config: None,
                },
                OcrPipelineStage {
                    backend: "paddleocr".to_string(),
                    priority: 50,
                    language: None,
                    tesseract_config: None,
                    paddle_ocr_config: Some(serde_json::json!({"use_gpu": false})),
                    vlm_config: None,
                },
            ],
            quality_thresholds: OcrQualityThresholds::default(),
        };
        let json = serde_json::to_string(&pipeline).unwrap();
        let deserialized: OcrPipelineConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.stages.len(), 2);
        assert_eq!(deserialized.stages[0].backend, "tesseract");
        assert_eq!(deserialized.stages[0].priority, 100);
        assert_eq!(deserialized.stages[1].backend, "paddleocr");
        assert_eq!(deserialized.stages[1].priority, 50);
        assert!(deserialized.stages[1].paddle_ocr_config.is_some());
    }

    #[test]
    fn test_pipeline_stage_deserialization_missing_optional_fields() {
        // Only backend is required; everything else should use defaults
        let json = r#"{"backend": "tesseract"}"#;
        let stage: OcrPipelineStage = serde_json::from_str(json).unwrap();
        assert_eq!(stage.backend, "tesseract");
        assert_eq!(stage.priority, 100); // default_priority
        assert!(stage.language.is_none());
        assert!(stage.tesseract_config.is_none());
        assert!(stage.paddle_ocr_config.is_none());
    }

    #[test]
    fn test_pipeline_stage_default_priority_is_100() {
        let json = r#"{"backend": "easyocr"}"#;
        let stage: OcrPipelineStage = serde_json::from_str(json).unwrap();
        assert_eq!(stage.priority, 100);
    }

    #[test]
    fn test_ocr_config_deserialization_missing_optional_fields() {
        let json = r#"{}"#;
        let config: OcrConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.backend, "tesseract");
        assert_eq!(config.language, "eng");
        assert!(config.pipeline.is_none());
        assert!(config.quality_thresholds.is_none());
        assert!(config.element_config.is_none());
    }

    #[test]
    fn test_quality_thresholds_deserialization_partial() {
        let json = r#"{"min_total_non_whitespace": 256}"#;
        let thresholds: OcrQualityThresholds = serde_json::from_str(json).unwrap();
        assert_eq!(thresholds.min_total_non_whitespace, 256);
        // All other fields should be defaults
        assert_eq!(thresholds.min_meaningful_words, 3);
        assert_eq!(thresholds.min_garbage_chars, 5);
        assert!((thresholds.pipeline_min_quality - 0.5).abs() < f64::EPSILON);
    }

    // ── Validation tests ──

    #[test]
    fn test_validate_catches_invalid_pipeline_stage_backend() {
        let config = OcrConfig {
            pipeline: Some(OcrPipelineConfig {
                stages: vec![
                    OcrPipelineStage {
                        backend: "tesseract".to_string(),
                        priority: 100,
                        language: None,
                        tesseract_config: None,
                        paddle_ocr_config: None,
                        vlm_config: None,
                    },
                    OcrPipelineStage {
                        backend: "invalid_backend".to_string(),
                        priority: 50,
                        language: None,
                        tesseract_config: None,
                        paddle_ocr_config: None,
                        vlm_config: None,
                    },
                ],
                quality_thresholds: OcrQualityThresholds::default(),
            }),
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err(), "Should catch invalid backend in pipeline stages");
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Invalid OCR backend") || err_msg.contains("invalid_backend"));
    }

    #[test]
    fn test_validate_passes_with_valid_pipeline_stages() {
        let config = OcrConfig {
            pipeline: Some(OcrPipelineConfig {
                stages: vec![
                    OcrPipelineStage {
                        backend: "tesseract".to_string(),
                        priority: 100,
                        language: None,
                        tesseract_config: None,
                        paddle_ocr_config: None,
                        vlm_config: None,
                    },
                    OcrPipelineStage {
                        backend: "paddleocr".to_string(),
                        priority: 50,
                        language: None,
                        tesseract_config: None,
                        paddle_ocr_config: None,
                        vlm_config: None,
                    },
                ],
                quality_thresholds: OcrQualityThresholds::default(),
            }),
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }
}

//! OCR configuration.
//!
//! Defines OCR-specific configuration including backend selection, language settings,
//! and Tesseract-specific parameters.

use serde::{Deserialize, Serialize};

use super::formats::OutputFormat;
use crate::core::config_validation::validate_ocr_backend;
use crate::error::KreuzbergError;
use crate::types::OcrElementConfig;

/// OCR configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrConfig {
    /// OCR backend: tesseract, easyocr, paddleocr, paddle-ocr, rapidocr, rapid-ocr, rapidpaddle, rapid-paddle
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
}

impl Default for OcrConfig {
    fn default() -> Self {
        Self {
            backend: default_tesseract_backend(),
            language: default_eng(),
            tesseract_config: None,
            output_format: None,
            paddle_ocr_config: None,
            element_config: None,
        }
    }
}

impl OcrConfig {
    /// Validates that the configured backend is supported.
    ///
    /// This method checks that the backend name is one of the supported OCR backends:
    /// - tesseract
    /// - easyocr
    /// - paddleocr / paddle-ocr
    /// - rapidocr / rapid-ocr
    /// - rapidpaddle / rapid-paddle (alias for paddle-ocr)
    ///
    /// Typos in backend names are caught at configuration validation time, not at runtime.
    ///
    /// # Errors
    ///
    /// Returns a `KreuzbergError::Validation` if the backend is not recognized.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use kreuzberg::core::config::OcrConfig;
    ///
    /// let config = OcrConfig {
    ///     backend: "tesseract".to_string(),
    ///     ..Default::default()
    /// };
    ///
    /// assert!(config.validate().is_ok());
    ///
    /// let bad_config = OcrConfig {
    ///     backend: "typo_backend".to_string(),
    ///     ..Default::default()
    /// };
    ///
    /// assert!(bad_config.validate().is_err());
    /// ```
    pub fn validate(&self) -> Result<(), KreuzbergError> {
        validate_ocr_backend(&self.backend)
    }
}

fn default_tesseract_backend() -> String {
    "tesseract".to_string()
}

/// Normalize OCR backend names to the canonical registered backend name.
///
/// This keeps user-facing aliases stable while registry internals remain consistent.
pub fn canonical_ocr_backend_name(backend: &str) -> String {
    match backend.to_lowercase().as_str() {
        "paddleocr" | "rapidpaddle" | "rapid-paddle" => "paddle-ocr".to_string(),
        "rapidocr" => "rapid-ocr".to_string(),
        other => other.to_string(),
    }
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
    fn test_validate_rapid_paddle_backend() {
        let config = OcrConfig {
            backend: "rapid-paddle".to_string(),
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_rapid_ocr_backend() {
        let config = OcrConfig {
            backend: "rapid-ocr".to_string(),
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_canonical_ocr_backend_name_aliases() {
        assert_eq!(canonical_ocr_backend_name("paddleocr"), "paddle-ocr");
        assert_eq!(canonical_ocr_backend_name("rapidpaddle"), "paddle-ocr");
        assert_eq!(canonical_ocr_backend_name("rapid-paddle"), "paddle-ocr");
        assert_eq!(canonical_ocr_backend_name("rapidocr"), "rapid-ocr");
        assert_eq!(canonical_ocr_backend_name("rapid-ocr"), "rapid-ocr");
        assert_eq!(canonical_ocr_backend_name("tesseract"), "tesseract");
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
}

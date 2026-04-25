//! Feature-specific configuration types for extraction.
//!
//! This module contains configuration structs for specific extraction features:
//! - Image extraction and processing
//! - Token reduction
//! - Language detection

use serde::{Deserialize, Serialize};

/// Image extraction configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageExtractionConfig {
    /// Extract images from documents
    #[serde(default = "default_true")]
    pub extract_images: bool,

    /// Target DPI for image normalization
    #[serde(default = "default_target_dpi")]
    pub target_dpi: i32,

    /// Maximum dimension for images (width or height)
    #[serde(default = "default_max_dimension")]
    pub max_image_dimension: i32,

    /// Whether to inject image reference placeholders into markdown output.
    /// When `true` (default), image references like `![Image 1](embedded:p1_i0)`
    /// are appended to the markdown. Set to `false` to extract images as data
    /// without polluting the markdown output.
    #[serde(default = "default_true")]
    pub inject_placeholders: bool,

    /// Automatically adjust DPI based on image content
    #[serde(default = "default_true")]
    pub auto_adjust_dpi: bool,

    /// Minimum DPI threshold
    #[serde(default = "default_min_dpi")]
    pub min_dpi: i32,

    /// Maximum DPI threshold
    #[serde(default = "default_max_dpi")]
    pub max_dpi: i32,

    /// Maximum number of image objects to extract per PDF page.
    ///
    /// Some PDFs (e.g. technical diagrams stored as thousands of raster fragments)
    /// can trigger extremely long or indefinite extraction times when every image
    /// object on a dense page is decoded individually via pdfium FFI. Setting this
    /// limit causes kreuzberg to stop collecting individual images once the count
    /// per page reaches the cap and emit a warning instead.
    ///
    /// `None` (default) means no limit — all images are extracted.
    #[serde(default)]
    pub max_images_per_page: Option<u32>,
}

/// Token reduction configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenReductionOptions {
    /// Reduction mode: "off", "light", "moderate", "aggressive", "maximum"
    #[serde(default = "default_reduction_mode")]
    pub mode: String,

    /// Preserve important words (capitalized, technical terms)
    #[serde(default = "default_true")]
    pub preserve_important_words: bool,
}

/// Language detection configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageDetectionConfig {
    /// Enable language detection
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Minimum confidence threshold (0.0-1.0)
    #[serde(default = "default_confidence")]
    pub min_confidence: f64,

    /// Detect multiple languages in the document
    #[serde(default)]
    pub detect_multiple: bool,
}

impl Default for ImageExtractionConfig {
    fn default() -> Self {
        Self {
            extract_images: true,
            target_dpi: 300,
            max_image_dimension: 4096,
            inject_placeholders: true,
            auto_adjust_dpi: true,
            min_dpi: 72,
            max_dpi: 600,
            max_images_per_page: None,
        }
    }
}

// Default value functions
fn default_true() -> bool {
    true
}

fn default_target_dpi() -> i32 {
    300
}

fn default_max_dimension() -> i32 {
    4096
}

fn default_min_dpi() -> i32 {
    72
}

fn default_max_dpi() -> i32 {
    600
}

fn default_reduction_mode() -> String {
    "off".to_string()
}

fn default_confidence() -> f64 {
    0.8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_extraction_config_default_booleans_are_true() {
        let cfg = ImageExtractionConfig::default();
        assert!(cfg.extract_images, "extract_images must default to true");
        assert!(cfg.inject_placeholders, "inject_placeholders must default to true");
        assert!(cfg.auto_adjust_dpi, "auto_adjust_dpi must default to true");
        assert_eq!(cfg.target_dpi, 300);
        assert_eq!(cfg.max_image_dimension, 4096);
        assert_eq!(cfg.min_dpi, 72);
        assert_eq!(cfg.max_dpi, 600);
    }

    #[test]
    fn test_image_extraction_config_explicit_false_disables_placeholders() {
        let cfg = ImageExtractionConfig {
            inject_placeholders: false,
            ..ImageExtractionConfig::default()
        };
        assert!(!cfg.inject_placeholders);
        assert!(cfg.extract_images);
    }

    #[test]
    fn test_image_extraction_config_absent_json_fields_get_canonical_defaults() {
        let json = r#"{"extract_images": true}"#;
        let cfg: ImageExtractionConfig = serde_json::from_str(json).unwrap();
        assert!(
            cfg.inject_placeholders,
            "absent inject_placeholders must deserialize to true"
        );
        assert!(cfg.auto_adjust_dpi, "absent auto_adjust_dpi must deserialize to true");
        assert_eq!(cfg.target_dpi, 300);
    }

    #[test]
    fn test_max_images_per_page_defaults_none() {
        let config = ImageExtractionConfig::default();
        assert_eq!(config.max_images_per_page, None);
    }

    #[test]
    fn test_max_images_per_page_serializes_as_null_when_none() {
        let config = ImageExtractionConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"max_images_per_page\":null"));
    }

    #[test]
    fn test_max_images_per_page_roundtrips_via_json() {
        let config = ImageExtractionConfig {
            max_images_per_page: Some(50),
            ..Default::default()
        };
        let json = serde_json::to_string(&config).unwrap();
        let back: ImageExtractionConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.max_images_per_page, Some(50));
    }

    /// Regression test for issue #766: missing field in JSON must not break
    /// deserialization (backwards-compat — existing configs without this key
    /// must still deserialize cleanly).
    #[test]
    fn test_max_images_per_page_absent_in_json_deserializes_as_none() {
        let json = r#"{"extract_images":true,"target_dpi":300,"max_image_dimension":4096,
                       "inject_placeholders":true,"auto_adjust_dpi":true,
                       "min_dpi":72,"max_dpi":600}"#;
        let config: ImageExtractionConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.max_images_per_page, None);
    }
}

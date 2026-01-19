//! Main extraction configuration and environment variable handling.
//!
//! This module contains the main `ExtractionConfig` struct and related utilities
//! for loading configuration from files and applying environment variable overrides.
//!
//! The module is organized into focused submodules:
//! - `types`: Feature-specific configuration types (image, token reduction, language detection)
//! - `core`: Main ExtractionConfig struct and implementation
//! - `env`: Environment variable override support
//! - `loaders`: Configuration file loading with caching

mod core;
mod env;
mod loaders;
mod types;

// Re-export all public types for backward compatibility
pub use self::core::ExtractionConfig;
pub use self::types::{ImageExtractionConfig, LanguageDetectionConfig, TokenReductionConfig};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::ocr::OcrConfig;

    #[test]
    fn test_default_config() {
        let config = ExtractionConfig::default();
        assert!(config.use_cache);
        assert!(config.enable_quality_processing);
        assert!(config.ocr.is_none());
    }

    #[test]
    fn test_needs_image_processing() {
        let mut config = ExtractionConfig::default();
        assert!(!config.needs_image_processing());

        config.ocr = Some(OcrConfig::default());
        assert!(config.needs_image_processing());
    }
}

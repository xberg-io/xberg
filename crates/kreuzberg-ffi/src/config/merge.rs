//! Configuration merging logic
//!
//! Provides functionality to merge two ExtractionConfig instances.

use kreuzberg::core::config::ExtractionConfig;

/// Merge two configs (override takes precedence over base).
///
/// Performs a shallow merge where fields from `override_config` take
/// precedence over fields in `base`. The `base` config is modified in-place.
///
/// # Arguments
///
/// * `base` - Mutable reference to the base config (will be modified)
/// * `override_config` - Reference to the override config (read-only)
pub fn merge_configs(base: &mut ExtractionConfig, override_config: &ExtractionConfig) {
    base.use_cache = override_config.use_cache;
    base.enable_quality_processing = override_config.enable_quality_processing;
    base.force_ocr = override_config.force_ocr;
    if override_config.force_ocr_pages.is_some() {
        base.force_ocr_pages = override_config.force_ocr_pages.clone();
    }
    base.max_concurrent_extractions = override_config.max_concurrent_extractions;

    if override_config.ocr.is_some() {
        base.ocr = override_config.ocr.clone();
    }

    if override_config.chunking.is_some() {
        base.chunking = override_config.chunking.clone();
    }

    if override_config.images.is_some() {
        base.images = override_config.images.clone();
    }

    #[cfg(feature = "pdf")]
    if override_config.pdf_options.is_some() {
        base.pdf_options = override_config.pdf_options.clone();
    }

    if override_config.token_reduction.is_some() {
        base.token_reduction = override_config.token_reduction.clone();
    }

    if override_config.language_detection.is_some() {
        base.language_detection = override_config.language_detection.clone();
    }

    if override_config.pages.is_some() {
        base.pages = override_config.pages.clone();
    }

    #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
    if override_config.keywords.is_some() {
        base.keywords = override_config.keywords.clone();
    }

    if override_config.postprocessor.is_some() {
        base.postprocessor = override_config.postprocessor.clone();
    }

    if override_config.html_options.is_some() {
        base.html_options = override_config.html_options.clone();
    }

    if override_config.extraction_timeout_secs.is_some() {
        base.extraction_timeout_secs = override_config.extraction_timeout_secs;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_configs_simple() {
        let mut base = ExtractionConfig {
            use_cache: true,
            force_ocr: false,
            ..Default::default()
        };

        let override_config = ExtractionConfig {
            force_ocr: true,
            ..Default::default()
        };

        merge_configs(&mut base, &override_config);

        assert!(base.use_cache);
        assert!(base.force_ocr);
    }

    #[test]
    fn test_merge_configs_override_to_default() {
        let mut base = ExtractionConfig {
            use_cache: false,
            ..Default::default()
        };

        let override_config = ExtractionConfig {
            use_cache: true,
            ..Default::default()
        };

        merge_configs(&mut base, &override_config);

        assert!(base.use_cache, "override to default value should be applied");
    }
}

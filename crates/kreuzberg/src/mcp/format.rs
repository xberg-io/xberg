//! MCP response formatting and configuration helpers.
//!
//! This module provides utilities for formatting extraction results and building configurations.

use crate::core::config::merge::build_config_from_json;
use crate::{ExtractionConfig, ExtractionResult as KreuzbergResult};

/// Build extraction config from MCP parameters.
///
/// Merges the provided config JSON (if any) with the default config using JSON-level
/// merge semantics. Unspecified fields in the JSON preserve their values from the default config.
pub(super) fn build_config(
    default_config: &ExtractionConfig,
    config_json: Option<serde_json::Value>,
) -> Result<ExtractionConfig, String> {
    build_config_from_json(default_config, config_json)
}

/// Format extraction result as JSON string.
///
/// Serializes the full `ExtractionResult` to JSON, ensuring 1:1 parity
/// with the API and CLI JSON output.
pub(super) fn format_extraction_result(result: &KreuzbergResult) -> String {
    serde_json::to_string_pretty(result).unwrap_or_default()
}

/// Format extraction result as TOON string.
///
/// Serializes the full `ExtractionResult` to TOON wire format.
pub(super) fn format_extraction_result_toon(result: &KreuzbergResult) -> String {
    serde_toon::to_string(result).unwrap_or_else(|e| {
        tracing::error!(error = %e, "Failed to serialize extraction result to TOON, falling back to JSON");
        format_extraction_result(result)
    })
}

/// Format extraction result using the specified wire format.
///
/// When `use_toon` is true, serializes to TOON format; otherwise serializes to JSON.
pub(super) fn format_extraction_result_for_wire(result: &KreuzbergResult, use_toon: bool) -> String {
    if use_toon {
        format_extraction_result_toon(result)
    } else {
        format_extraction_result(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Cow;

    #[test]
    fn test_build_config_with_no_config() {
        let default_config = ExtractionConfig::default();

        let config = build_config(&default_config, None).unwrap();
        assert_eq!(config.use_cache, default_config.use_cache);
    }

    #[test]
    fn test_build_config_with_config_json() {
        let default_config = ExtractionConfig::default();
        let config_json = serde_json::json!({
            "use_cache": false
        });

        let config = build_config(&default_config, Some(config_json)).unwrap();
        assert!(!config.use_cache);
    }

    #[test]
    fn test_build_config_with_invalid_config_json() {
        let default_config = ExtractionConfig::default();
        // Provide invalid type for a field (string instead of boolean)
        let config_json = serde_json::json!({
            "use_cache": "not_a_boolean"
        });

        let result = build_config(&default_config, Some(config_json));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to deserialize"));
    }

    #[test]
    fn test_build_config_preserves_default_config_settings() {
        let default_config = ExtractionConfig {
            use_cache: false,
            ..Default::default()
        };

        let config = build_config(&default_config, None).unwrap();

        assert!(!config.use_cache);
    }

    #[test]
    fn test_build_config_overrides_default_settings() {
        let default_config = ExtractionConfig {
            use_cache: true,
            ..Default::default()
        };

        let config_json = serde_json::json!({
            "use_cache": false
        });

        let config = build_config(&default_config, Some(config_json)).unwrap();
        assert!(!config.use_cache);
    }

    #[test]
    fn test_build_config_merges_partial_config() {
        // Base config with custom use_cache setting
        let default_config = ExtractionConfig {
            use_cache: false,
            enable_quality_processing: true,
            force_ocr: false,
            ..Default::default()
        };

        // Override only force_ocr
        let config_json = serde_json::json!({
            "force_ocr": true
        });

        let config = build_config(&default_config, Some(config_json)).unwrap();

        // use_cache should be preserved from default_config
        assert!(!config.use_cache, "use_cache should be preserved from default config");
        // enable_quality_processing should be preserved
        assert!(
            config.enable_quality_processing,
            "enable_quality_processing should be preserved"
        );
        // force_ocr should be overridden
        assert!(config.force_ocr, "force_ocr should be overridden to true");
    }

    #[test]
    fn test_build_config_merges_nested_config() {
        let default_config = ExtractionConfig {
            use_cache: true,
            ..Default::default()
        };

        // Override output format only
        let config_json = serde_json::json!({
            "output_format": "markdown"
        });

        let config = build_config(&default_config, Some(config_json)).unwrap();

        // use_cache should be preserved
        assert!(config.use_cache, "use_cache should be preserved from default config");
        // output_format should be overridden
        assert_eq!(
            config.output_format,
            crate::core::config::formats::OutputFormat::Markdown,
            "output_format should be overridden to markdown"
        );
    }

    #[test]
    fn test_build_config_merges_with_custom_defaults() {
        // Create a default config with custom values
        let default_config = ExtractionConfig {
            use_cache: false,
            enable_quality_processing: true,
            force_ocr: false,
            ..Default::default()
        };

        // Provide partial override (only force_ocr)
        let config_json = serde_json::json!({
            "force_ocr": true,
        });

        let config = build_config(&default_config, Some(config_json)).unwrap();

        // force_ocr should be overridden
        assert!(config.force_ocr, "force_ocr should be overridden to true");
        // use_cache should be preserved from default_config
        assert!(
            !config.use_cache,
            "use_cache should be preserved from default config (false)"
        );
        // enable_quality_processing should be preserved
        assert!(
            config.enable_quality_processing,
            "enable_quality_processing should be preserved (true)"
        );
    }

    #[test]
    fn test_build_config_merges_multiple_fields() {
        let default_config = ExtractionConfig {
            use_cache: true,
            enable_quality_processing: false,
            force_ocr: true,
            ..Default::default()
        };

        // Override multiple fields
        let config_json = serde_json::json!({
            "use_cache": false,
            "output_format": "markdown",
        });

        let config = build_config(&default_config, Some(config_json)).unwrap();

        // use_cache should be overridden
        assert!(!config.use_cache, "use_cache should be overridden to false");
        // output_format should be overridden
        assert_eq!(
            config.output_format,
            crate::core::config::formats::OutputFormat::Markdown,
            "output_format should be overridden to markdown"
        );
        // force_ocr should be preserved (not in override)
        assert!(
            config.force_ocr,
            "force_ocr should be preserved from default config (true)"
        );
        // enable_quality_processing should be preserved
        assert!(
            !config.enable_quality_processing,
            "enable_quality_processing should be preserved (false)"
        );
    }

    #[test]
    fn test_build_config_boolean_override_to_default_value() {
        // This test validates the critical bug fix: when user explicitly sets a boolean
        // to its default value, the merge logic should correctly use the override value,
        // not fall back to the base config.
        let base = ExtractionConfig {
            use_cache: false,
            ..Default::default()
        };

        // User explicitly provides use_cache: true (which IS the default)
        let override_json = serde_json::json!({"use_cache": true});

        let merged = build_config(&base, Some(override_json)).unwrap();

        // Before the fix: merged.use_cache would be false (WRONG - fell back to base)
        // After the fix: merged.use_cache should be true (CORRECT - override applied)
        assert!(
            merged.use_cache,
            "Should use explicit override even if it matches default"
        );
    }

    #[test]
    fn test_format_extraction_result_is_valid_json() {
        let result = KreuzbergResult {
            content: "Sample extracted text".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            metadata: crate::Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
            pages: None,
            elements: None,
            ocr_elements: None,
            djot_content: None,
            document: None,
            #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
            extracted_keywords: None,
            quality_score: None,
            processing_warnings: Vec::new(),
            annotations: None,
            children: None,
            uris: None,
            formatted_content: None,
        };

        let formatted = format_extraction_result(&result);
        let parsed: serde_json::Value = serde_json::from_str(&formatted).expect("Should be valid JSON");

        assert_eq!(parsed["content"], "Sample extracted text");
        assert_eq!(parsed["mime_type"], "text/plain");
        assert!(parsed["metadata"].is_object());
    }

    #[test]
    fn test_format_extraction_result_includes_tables() {
        let result = KreuzbergResult {
            content: "Document with tables".to_string(),
            mime_type: Cow::Borrowed("application/pdf"),
            metadata: crate::Metadata::default(),
            tables: vec![crate::Table {
                cells: vec![
                    vec!["Col1".to_string(), "Col2".to_string()],
                    vec!["A".to_string(), "B".to_string()],
                ],
                page_number: 1,
                markdown: "| Col1 | Col2 |\n|------|------|\n| A    | B    |".to_string(),
                bounding_box: None,
            }],
            detected_languages: None,
            chunks: None,
            images: None,
            pages: None,
            elements: None,
            ocr_elements: None,
            djot_content: None,
            document: None,
            #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
            extracted_keywords: None,
            quality_score: None,
            processing_warnings: Vec::new(),
            annotations: None,
            children: None,
            uris: None,
            formatted_content: None,
        };

        let formatted = format_extraction_result(&result);
        let parsed: serde_json::Value = serde_json::from_str(&formatted).expect("Should be valid JSON");

        assert_eq!(parsed["tables"].as_array().unwrap().len(), 1);
        assert_eq!(parsed["tables"][0]["page_number"], 1);
    }

    #[test]
    fn test_format_extraction_result_includes_chunks_when_present() {
        let result = KreuzbergResult {
            content: "Chunked text".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            metadata: crate::Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: Some(vec![crate::Chunk {
                content: "Chunk 1".to_string(),
                chunk_type: Default::default(),
                embedding: None,
                metadata: crate::ChunkMetadata {
                    byte_start: 0,
                    byte_end: 7,
                    token_count: None,
                    chunk_index: 0,
                    total_chunks: 1,
                    first_page: None,
                    last_page: None,
                    heading_context: None,
                },
            }]),
            images: None,
            pages: None,
            elements: None,
            ocr_elements: None,
            djot_content: None,
            document: None,
            #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
            extracted_keywords: None,
            quality_score: None,
            processing_warnings: Vec::new(),
            annotations: None,
            children: None,
            uris: None,
            formatted_content: None,
        };

        let formatted = format_extraction_result(&result);
        let parsed: serde_json::Value = serde_json::from_str(&formatted).expect("Should be valid JSON");

        assert_eq!(parsed["chunks"].as_array().unwrap().len(), 1);
        assert_eq!(parsed["chunks"][0]["content"], "Chunk 1");
    }

    #[test]
    fn test_format_extraction_result_omits_none_fields() {
        let result = KreuzbergResult {
            content: "Simple text".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            metadata: crate::Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
            pages: None,
            elements: None,
            ocr_elements: None,
            djot_content: None,
            document: None,
            #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
            extracted_keywords: None,
            quality_score: None,
            processing_warnings: Vec::new(),
            annotations: None,
            children: None,
            uris: None,
            formatted_content: None,
        };

        let formatted = format_extraction_result(&result);
        let parsed: serde_json::Value = serde_json::from_str(&formatted).expect("Should be valid JSON");

        // None fields should be omitted via skip_serializing_if
        assert!(parsed.get("chunks").is_none());
        assert!(parsed.get("images").is_none());
        assert!(parsed.get("detected_languages").is_none());
    }
}

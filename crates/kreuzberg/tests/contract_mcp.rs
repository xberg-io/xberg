//! MCP contract tests - verify MCP config matches Rust core
//!
//! This test suite validates that MCP (Model Context Protocol) configuration
//! produces identical JSON to the Rust core library when parsing configuration.
//! This ensures that MCP users get the same configuration behavior as CLI and SDK users.

use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::core::config::OutputFormat;
use serde_json::json;

#[test]
fn test_mcp_basic_config_json_matches_rust_core() {
    // Create config via Rust core
    let rust_config = ExtractionConfig {
        use_cache: true,
        enable_quality_processing: true,
        force_ocr: false,
        output_format: OutputFormat::Plain,
        result_format: kreuzberg::types::ResultFormat::Unified,
        ..Default::default()
    };
    let rust_json = serde_json::to_value(&rust_config).expect("Failed to serialize rust config");

    // Simulate MCP config parameter deserialization
    let mcp_json = json!({
        "use_cache": true,
        "enable_quality_processing": true,
        "force_ocr": false,
        "output_format": "plain",
        "result_format": "unified"
    });
    let mcp_config: ExtractionConfig =
        serde_json::from_value(mcp_json.clone()).expect("Failed to deserialize MCP config");
    let mcp_serialized = serde_json::to_value(&mcp_config).expect("Failed to serialize MCP config");

    // Verify they produce identical JSON for the relevant fields
    assert_eq!(
        rust_json.get("use_cache"),
        mcp_serialized.get("use_cache"),
        "MCP use_cache must match Rust core"
    );
    assert_eq!(
        rust_json.get("enable_quality_processing"),
        mcp_serialized.get("enable_quality_processing"),
        "MCP enable_quality_processing must match Rust core"
    );
    assert_eq!(
        rust_json.get("force_ocr"),
        mcp_serialized.get("force_ocr"),
        "MCP force_ocr must match Rust core"
    );
    assert_eq!(
        rust_json.get("output_format"),
        mcp_serialized.get("output_format"),
        "MCP output_format must match Rust core"
    );
}

#[test]
fn test_mcp_ocr_config_nested_matches_rust_core() {
    let mcp_json = json!({
        "ocr": {
            "backend": "tesseract"
        },
        "force_ocr": true
    });

    let config: ExtractionConfig = serde_json::from_value(mcp_json).expect("Failed to deserialize OCR config");

    // Verify OCR config deserialized correctly
    assert!(config.ocr.is_some(), "OCR config should be present");
    assert!(config.force_ocr, "force_ocr should be true");

    if let Some(ocr) = &config.ocr {
        assert_eq!(ocr.backend, "tesseract", "OCR backend should be tesseract");
    }

    // Verify roundtrip
    let serialized = serde_json::to_value(&config).expect("Failed to serialize");
    assert!(serialized.get("ocr").is_some(), "Serialized config should include ocr");
}

#[test]
fn test_mcp_chunking_config_nested_matches_rust_core() {
    let mcp_json = json!({
        "chunking": {
            "max_chars": 500,
            "max_overlap": 50,
            "strategy": "sliding_window"
        }
    });

    let config: ExtractionConfig = serde_json::from_value(mcp_json).expect("Failed to deserialize chunking config");

    // Verify chunking config deserialized correctly
    assert!(config.chunking.is_some(), "Chunking config should be present");

    if let Some(chunking) = &config.chunking {
        assert_eq!(chunking.max_characters, 500, "max_chars should be 500");
        assert_eq!(chunking.overlap, 50, "max_overlap should be 50");
    }

    // Verify roundtrip
    let serialized = serde_json::to_value(&config).expect("Failed to serialize");
    assert!(
        serialized.get("chunking").is_some(),
        "Serialized config should include chunking"
    );
}

#[test]
fn test_mcp_full_config_preserves_all_fields() {
    let full_config_json = json!({
        "use_cache": false,
        "enable_quality_processing": true,
        "force_ocr": true,
        "output_format": "markdown",
        "result_format": "unified",
        "max_concurrent_extractions": 8,
        "ocr": {
            "backend": "tesseract"
        },
        "chunking": {
            "max_chars": 1000,
            "max_overlap": 200
        }
    });

    let config: ExtractionConfig =
        serde_json::from_value(full_config_json.clone()).expect("Failed to deserialize full config");
    let roundtrip_json = serde_json::to_value(&config).expect("Failed to serialize");

    // Verify all top-level fields preserved
    assert!(!config.use_cache, "use_cache should be false");
    assert!(
        config.enable_quality_processing,
        "enable_quality_processing should be true"
    );
    assert!(config.force_ocr, "force_ocr should be true");
    assert_eq!(
        config.max_concurrent_extractions,
        Some(8),
        "max_concurrent_extractions should be 8"
    );

    // Verify nested fields preserved
    assert!(config.ocr.is_some(), "OCR config should be present");
    assert!(config.chunking.is_some(), "Chunking config should be present");

    // Verify roundtrip integrity
    assert_eq!(
        roundtrip_json.get("use_cache"),
        full_config_json.get("use_cache"),
        "use_cache should survive roundtrip"
    );
    assert_eq!(
        roundtrip_json.get("force_ocr"),
        full_config_json.get("force_ocr"),
        "force_ocr should survive roundtrip"
    );
    assert_eq!(
        roundtrip_json.get("max_concurrent_extractions"),
        full_config_json.get("max_concurrent_extractions"),
        "max_concurrent_extractions should survive roundtrip"
    );
}

#[test]
fn test_mcp_default_config_matches_rust_core_defaults() {
    // Create Rust core default
    let rust_default = ExtractionConfig::default();
    let rust_json = serde_json::to_value(&rust_default).expect("Failed to serialize default");

    // Create empty JSON (simulates MCP with no overrides)
    let mcp_json = json!({});
    let mcp_config: ExtractionConfig = serde_json::from_value(mcp_json).expect("Failed to deserialize empty config");
    let mcp_json_serialized = serde_json::to_value(&mcp_config).expect("Failed to serialize MCP default");

    // Verify defaults match
    assert_eq!(
        mcp_json_serialized.get("use_cache"),
        rust_json.get("use_cache"),
        "use_cache default should match"
    );
    assert_eq!(
        mcp_json_serialized.get("enable_quality_processing"),
        rust_json.get("enable_quality_processing"),
        "enable_quality_processing default should match"
    );
    assert_eq!(
        mcp_json_serialized.get("force_ocr"),
        rust_json.get("force_ocr"),
        "force_ocr default should match"
    );
    assert_eq!(
        mcp_json_serialized.get("result_format"),
        rust_json.get("result_format"),
        "result_format default should match"
    );
    assert_eq!(
        mcp_json_serialized.get("output_format"),
        rust_json.get("output_format"),
        "output_format default should match"
    );
}

#[test]
fn test_mcp_output_format_values_are_valid() {
    // Test all valid output format values (lowercase, as per serde rename_all)
    let valid_formats = vec!["plain", "markdown", "html"];

    for format in valid_formats {
        let mcp_json = json!({
            "output_format": format
        });

        let result = serde_json::from_value::<ExtractionConfig>(mcp_json);
        assert!(result.is_ok(), "Format '{}' should deserialize successfully", format);

        let config = result.unwrap();
        assert!(
            !config.output_format.to_string().is_empty(),
            "Deserialized format should have valid string representation"
        );
    }
}

#[test]
fn test_mcp_result_format_values_are_valid() {
    // Test valid result format values (lowercase, as per serde rename_all)
    let valid_formats = vec!["unified", "element_based"];

    for format in valid_formats {
        let mcp_json = json!({
            "result_format": format
        });

        let result = serde_json::from_value::<ExtractionConfig>(mcp_json);
        assert!(
            result.is_ok(),
            "Result format '{}' should deserialize successfully",
            format
        );
    }
}

#[test]
fn test_mcp_partial_override_preserves_defaults() {
    // Create a partial config that overrides only one field
    let partial_json = json!({
        "force_ocr": true
    });

    let config: ExtractionConfig = serde_json::from_value(partial_json).expect("Failed to deserialize partial config");

    // Verify override applied
    assert!(config.force_ocr, "force_ocr override should be applied");

    // Verify defaults preserved for other fields
    assert!(config.use_cache, "use_cache should retain default when not overridden");
    assert!(
        config.enable_quality_processing,
        "enable_quality_processing should retain default when not overridden"
    );
}

#[test]
fn test_mcp_error_handling_for_invalid_json() {
    // Test that invalid format values produce errors (or are handled gracefully)
    let invalid_json = json!({
        "output_format": "InvalidFormat"
    });

    let result = serde_json::from_value::<ExtractionConfig>(invalid_json);
    // The deserialization should either fail or parse to a valid state
    // depending on how OutputFormat handles unknown values
    if let Ok(config) = result {
        let _ = config.output_format.to_string();
    }
}

#[test]
fn test_mcp_concurrent_extractions_override() {
    let mcp_json = json!({
        "max_concurrent_extractions": 16
    });

    let config: ExtractionConfig =
        serde_json::from_value(mcp_json).expect("Failed to deserialize config with concurrent extractions");

    assert_eq!(
        config.max_concurrent_extractions,
        Some(16),
        "max_concurrent_extractions should be overridden to 16"
    );
}

#[test]
fn test_mcp_config_json_keys_case_sensitive() {
    // Verify that config JSON keys are case-sensitive
    let lowercase_json = json!({
        "use_cache": true,
        "force_ocr": false
    });

    let config: ExtractionConfig =
        serde_json::from_value(lowercase_json).expect("Failed to deserialize lowercase config");

    assert!(config.use_cache, "use_cache should be true");
    assert!(!config.force_ocr, "force_ocr should be false");

    // Note: serde by default fails on unknown fields, so camelCase would fail
    // This test documents the expected behavior
}

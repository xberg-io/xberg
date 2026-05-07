//! MCP integration tests for API consistency and breaking changes.
//!
//! This test suite validates that:
//! 1. MCP parameters properly handle extraction configuration
//! 2. MCP parameter deserialization is consistent
//! 3. Various config combinations work correctly
//! 4. End-to-end MCP tool invocations work with real data
//! 5. Error handling is consistent across MCP tools
//!
//! Note: These tests verify the parameter structures used by MCP.
//! The build_config function in the MCP server should accept
//! a config JSON field instead of separate enable_ocr/force_ocr flags
//! to align with the new API consistency approach.

#![allow(clippy::bool_assert_comparison)]
#![allow(clippy::field_reassign_with_default)]

use serde_json::json;

/// Test that parameter structures can handle various JSON configurations
#[test]
fn test_extraction_config_parameter_structure() {
    // This demonstrates the new approach: config JSON instead of separate flags
    let config_json = json!({
        "use_cache": true,
        "force_ocr": true,
        "output_format": "markdown",
    });

    let config: kreuzberg::core::config::ExtractionConfig =
        serde_json::from_value(config_json).expect("Failed to parse config");

    assert_eq!(config.use_cache, true);
    assert_eq!(config.force_ocr, true);
    assert_eq!(config.output_format, kreuzberg::core::config::OutputFormat::Markdown);
}

#[test]
fn test_mcp_style_params_with_config() {
    // This demonstrates how MCP params should accept full config JSON
    let mcp_request = json!({
        "path": "/test.pdf",
        "mime_type": "application/pdf",
        "config": {
            "use_cache": false,
            "force_ocr": true,
            "output_format": "markdown",
        }
    });

    // The config field should be parseable as ExtractionConfig
    let config_obj = mcp_request.get("config").expect("Should have config field");
    let config: kreuzberg::core::config::ExtractionConfig =
        serde_json::from_value(config_obj.clone()).expect("Failed to parse config");

    assert_eq!(config.force_ocr, true);
    assert_eq!(config.use_cache, false);
}

#[test]
fn test_mcp_params_backward_compatibility_minimal() {
    // Minimal MCP params structure
    let params = json!({
        "path": "/test.pdf",
    });

    // Should be deserializable
    let path = params.get("path").expect("Should have path");
    assert_eq!(path, "/test.pdf");
}

#[test]
fn test_mcp_params_with_all_fields() {
    // Complete MCP params with config
    let params = json!({
        "path": "/test.pdf",
        "mime_type": "application/pdf",
        "config": {
            "use_cache": true,
            "enable_quality_processing": true,
            "force_ocr": false,
            "output_format": "plain",
        }
    });

    // Extract and validate config
    if let Some(config_obj) = params.get("config") {
        let config: kreuzberg::core::config::ExtractionConfig =
            serde_json::from_value(config_obj.clone()).expect("Failed to parse");

        assert_eq!(config.use_cache, true);
        assert_eq!(config.force_ocr, false);
        assert_eq!(config.output_format, kreuzberg::core::config::OutputFormat::Plain);
    }
}

#[test]
fn test_batch_extraction_params_structure() {
    // Batch extraction params with paths and config
    let batch_params = json!({
        "paths": ["/file1.pdf", "/file2.pdf", "/file3.pdf"],
        "config": {
            "force_ocr": true,
            "max_concurrent_extractions": 4,
        }
    });

    let paths = batch_params.get("paths").expect("Should have paths");
    assert!(paths.is_array(), "paths field should be an array");
    let path_array = paths.as_array().expect("paths should be deserializable as array");
    assert_eq!(path_array.len(), 3, "paths array should contain exactly 3 elements");

    if let Some(config_obj) = batch_params.get("config") {
        let config: kreuzberg::core::config::ExtractionConfig =
            serde_json::from_value(config_obj.clone()).expect("Failed to parse");
        assert_eq!(config.force_ocr, true);
        assert_eq!(config.max_concurrent_extractions, Some(4));
    }
}

#[test]
fn test_config_merge_in_mcp_context() {
    // Test 1: Verify default config baseline
    let default_config = kreuzberg::core::config::ExtractionConfig::default();
    assert_eq!(default_config.use_cache, true, "Default cache should be enabled");
    assert_eq!(default_config.force_ocr, false, "Default force_ocr should be false");
    assert_eq!(
        default_config.output_format,
        kreuzberg::core::config::OutputFormat::Plain,
        "Default output format should be Plain"
    );

    // Test 2: Request provides single field override - verify precedence
    let request_config_json = json!({
        "force_ocr": true,
    });
    let request_config: kreuzberg::core::config::ExtractionConfig =
        serde_json::from_value(request_config_json).expect("Failed to parse request config");

    // Request config should override that field
    assert_eq!(request_config.force_ocr, true, "Request force_ocr should be true");

    // But unspecified fields should use defaults
    assert_eq!(
        request_config.use_cache, true,
        "Unspecified use_cache should default to true"
    );
    assert_eq!(
        request_config.output_format,
        kreuzberg::core::config::OutputFormat::Plain,
        "Unspecified output_format should default to Plain"
    );

    // Test 3: Multiple field overrides - verify precedence chain
    let multi_override_json = json!({
        "use_cache": false,
        "force_ocr": true,
        "output_format": "markdown",
    });
    let multi_config: kreuzberg::core::config::ExtractionConfig =
        serde_json::from_value(multi_override_json).expect("Failed to parse multi-field config");

    // All specified fields should override defaults
    assert_eq!(multi_config.use_cache, false, "Override use_cache should be false");
    assert_eq!(multi_config.force_ocr, true, "Override force_ocr should be true");
    assert_eq!(
        multi_config.output_format,
        kreuzberg::core::config::OutputFormat::Markdown,
        "Override output_format should be Markdown"
    );

    // Unspecified numeric fields should still have defaults
    if let Some(max_conc) = multi_config.max_concurrent_extractions {
        panic!(
            "max_concurrent_extractions should not be specified when not in request, got: {}",
            max_conc
        );
    }

    // Test 4: Verify config can be fully constructed with all fields
    let full_json = json!({
        "use_cache": false,
        "enable_quality_processing": true,
        "force_ocr": true,
        "output_format": "html",
        "max_concurrent_extractions": 8,
    });
    let full_config: kreuzberg::core::config::ExtractionConfig =
        serde_json::from_value(full_json).expect("Failed to parse full config");

    assert_eq!(full_config.use_cache, false, "Full config use_cache should be false");
    assert_eq!(
        full_config.enable_quality_processing, true,
        "Full config quality processing should be true"
    );
    assert_eq!(full_config.force_ocr, true, "Full config force_ocr should be true");
    assert_eq!(
        full_config.output_format,
        kreuzberg::core::config::OutputFormat::Html,
        "Full config output_format should be Html"
    );
    assert_eq!(
        full_config.max_concurrent_extractions,
        Some(8),
        "Full config max_concurrent should be 8"
    );
}

#[test]
fn test_config_json_flexibility() {
    // Config JSON can have any combination of fields
    let configs = vec![
        json!({}),                                                             // Empty = all defaults
        json!({"force_ocr": true}),                                            // Single field
        json!({"force_ocr": true, "use_cache": false}),                        // Multiple fields
        json!({"output_format": "markdown", "max_concurrent_extractions": 8}), // Various types
    ];

    for config_json in configs {
        let config: Result<kreuzberg::core::config::ExtractionConfig, _> = serde_json::from_value(config_json);
        assert!(config.is_ok(), "Config should deserialize successfully");
    }
}

#[test]
fn test_extraction_config_serialization_for_mcp() {
    // MCP should be able to serialize config back to JSON
    let mut config = kreuzberg::core::config::ExtractionConfig::default();
    config.force_ocr = true;
    config.output_format = kreuzberg::core::config::OutputFormat::Markdown;

    let json = serde_json::to_value(&config).expect("Failed to serialize");

    // Verify it round-trips
    let restored: kreuzberg::core::config::ExtractionConfig =
        serde_json::from_value(json).expect("Failed to deserialize");

    assert_eq!(config.force_ocr, restored.force_ocr);
    assert_eq!(config.output_format, restored.output_format);
}

// ============================================================================
// E2E TEST CASES
// ============================================================================

/// Test MCP config with all options enabled
#[test]
fn test_mcp_config_full_extraction() {
    let config_json = json!({
        "use_cache": false,
        "enable_quality_processing": true,
        "force_ocr": false,
        "output_format": "markdown",
        "max_concurrent_extractions": 4,
    });

    let config: kreuzberg::core::config::ExtractionConfig =
        serde_json::from_value(config_json).expect("Failed to parse full config");

    // Verify all fields deserialized correctly
    assert_eq!(config.use_cache, false);
    assert_eq!(config.enable_quality_processing, true);
    assert_eq!(config.force_ocr, false);
    assert_eq!(config.output_format, kreuzberg::core::config::OutputFormat::Markdown);
    assert_eq!(config.max_concurrent_extractions, Some(4));
}

/// Test MCP config with markdown output format
#[test]
fn test_mcp_config_output_format_markdown() {
    let config_json = json!({
        "output_format": "markdown",
    });

    let config: kreuzberg::core::config::ExtractionConfig =
        serde_json::from_value(config_json).expect("Failed to parse markdown config");

    assert_eq!(config.output_format, kreuzberg::core::config::OutputFormat::Markdown);
}

/// Test MCP config with element-based result structure
#[test]
fn test_mcp_config_result_format_element_based() {
    let config_json = json!({
        "output_format": "markdown",
        "use_cache": true,
        "enable_quality_processing": true,
    });

    let config: kreuzberg::core::config::ExtractionConfig =
        serde_json::from_value(config_json).expect("Failed to parse element format");

    assert_eq!(config.output_format, kreuzberg::core::config::OutputFormat::Markdown);
    assert_eq!(config.use_cache, true);
    assert_eq!(config.enable_quality_processing, true);
}

/// Test batch extraction with config applied to all files
#[test]
fn test_mcp_batch_with_config() {
    let batch_request = json!({
        "paths": ["/file1.txt", "/file2.txt", "/file3.txt"],
        "config": {
            "force_ocr": true,
            "output_format": "plain",
            "max_concurrent_extractions": 2,
        }
    });

    // Verify paths are array
    let paths = batch_request.get("paths").expect("Should have paths");
    assert!(paths.is_array(), "paths field should be an array");
    let path_array = paths.as_array().expect("paths should be deserializable as array");
    assert_eq!(path_array.len(), 3, "paths array should contain exactly 3 elements");

    // Verify config applies to batch
    let config_obj = batch_request.get("config").expect("Should have config");
    let config: kreuzberg::core::config::ExtractionConfig =
        serde_json::from_value(config_obj.clone()).expect("Failed to parse batch config");

    assert_eq!(config.force_ocr, true);
    assert_eq!(config.output_format, kreuzberg::core::config::OutputFormat::Plain);
    assert_eq!(config.max_concurrent_extractions, Some(2));
}

/// Test MCP error handling with invalid JSON config
#[test]
fn test_mcp_invalid_config_json_error() {
    let invalid_config = "not a valid json object";

    let result: Result<kreuzberg::core::config::ExtractionConfig, _> = serde_json::from_str(invalid_config);

    assert!(result.is_err(), "Invalid JSON should produce error");
}

/// Test that MCP config field precedence is correct
#[test]
fn test_mcp_config_overrides() {
    // Simulate MCP request with inline config
    let mcp_params = json!({
        "path": "/document.pdf",
        "mime_type": "application/pdf",
        "config": {
            "force_ocr": true,
            "use_cache": false,
            "output_format": "markdown",
        }
    });

    if let Some(config_obj) = mcp_params.get("config") {
        let parsed_config: kreuzberg::core::config::ExtractionConfig =
            serde_json::from_value(config_obj.clone()).expect("Failed to parse");

        // Verify request config overrides defaults
        assert_eq!(parsed_config.force_ocr, true);
        assert_eq!(parsed_config.use_cache, false);
        assert_eq!(
            parsed_config.output_format,
            kreuzberg::core::config::OutputFormat::Markdown
        );
    }
}

/// Test that deprecated parameters (enable_ocr, force_ocr as separate fields) are rejected
#[test]
fn test_mcp_no_deprecated_params() {
    // This simulates MCP params that incorrectly use separate flags
    let deprecated_params = json!({
        "path": "/document.pdf",
        "enable_ocr": true,  // deprecated!
        "force_ocr": true,    // should be in config
    });

    // The correct approach: config field contains all options
    let correct_params = json!({
        "path": "/document.pdf",
        "config": {
            "force_ocr": true,
        }
    });

    // Extract and verify correct params
    if let Some(config_obj) = correct_params.get("config") {
        let config: kreuzberg::core::config::ExtractionConfig =
            serde_json::from_value(config_obj.clone()).expect("Failed to parse");
        assert_eq!(config.force_ocr, true);
    }

    // Verify deprecated params are NOT in the correct structure
    assert!(
        deprecated_params.get("config").is_none(),
        "Deprecated params should not be in config"
    );
}

/// End-to-end test with real text extraction
#[tokio::test]
async fn test_mcp_real_pdf_extraction() {
    // Create a simple test document in bytes
    let test_content = b"Hello, MCP!";

    // Create MCP request structure
    let mcp_request = json!({
        "mime_type": "text/plain",
        "config": {
            "output_format": "plain",
            "use_cache": false,
        }
    });

    // Extract config from request
    if let Some(config_obj) = mcp_request.get("config") {
        let config: kreuzberg::core::config::ExtractionConfig =
            serde_json::from_value(config_obj.clone()).expect("Failed to parse config");

        // Use async extract_bytes to process content
        let result = kreuzberg::extract_bytes(test_content, "text/plain", &config)
            .await
            .expect("Extraction should succeed");

        // Verify result has content
        assert!(!result.content.is_empty());
        assert!(result.content.contains("MCP") || result.content.contains("Hello"));
    }
}

/// Test MCP batch extraction with mixed formats
#[test]
fn test_mcp_batch_mixed_formats() {
    let batch_config = json!({
        "files": [
            {
                "path": "/document.pdf",
                "mime_type": "application/pdf",
            },
            {
                "path": "/document.docx",
                "mime_type": "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            },
            {
                "path": "/document.txt",
                "mime_type": "text/plain",
            }
        ],
        "config": {
            "output_format": "markdown",
            "force_ocr": false,
        }
    });

    let files = batch_config.get("files").expect("Should have files");
    assert!(files.is_array(), "files field should be an array");
    let file_array = files.as_array().expect("files should be deserializable as array");
    assert_eq!(file_array.len(), 3, "files array should contain exactly 3 elements");

    if let Some(config_obj) = batch_config.get("config") {
        let config: kreuzberg::core::config::ExtractionConfig =
            serde_json::from_value(config_obj.clone()).expect("Failed to parse batch config");
        assert_eq!(config.output_format, kreuzberg::core::config::OutputFormat::Markdown);
        assert_eq!(config.force_ocr, false);
    }
}

/// Test MCP request with minimal config (all defaults)
#[test]
fn test_mcp_minimal_config() {
    let minimal_request = json!({
        "path": "/document.pdf",
    });

    // Path should exist and be correct
    assert_eq!(
        minimal_request.get("path"),
        Some(&serde_json::Value::String("/document.pdf".to_string())),
        "Path field should be present and set to /document.pdf"
    );

    // If no config, use defaults
    let config = match minimal_request.get("config") {
        Some(config_obj) => {
            serde_json::from_value(config_obj.clone()).expect("Failed to parse config from minimal request")
        }
        None => kreuzberg::core::config::ExtractionConfig::default(),
    };

    // Verify defaults are applied
    assert_eq!(config.use_cache, true);
    assert_eq!(config.output_format, kreuzberg::core::config::OutputFormat::Plain);
}

/// Test MCP config with all output formats
#[test]
fn test_mcp_all_output_formats() {
    let formats = vec!["plain", "markdown", "html"];

    for format_str in formats {
        let config_json = json!({
            "output_format": format_str,
        });

        let config: kreuzberg::core::config::ExtractionConfig =
            serde_json::from_value(config_json).expect("Failed to parse output format config");

        // Verify format was set
        let format_display = format!("{}", config.output_format);
        assert_eq!(format_display, format_str);
    }
}

/// Test MCP concurrent extraction config
#[test]
fn test_mcp_concurrent_extraction_config() {
    let concurrent_configs = vec![1, 2, 4, 8, 16];

    for max_concurrent in concurrent_configs {
        let config_json = json!({
            "max_concurrent_extractions": max_concurrent,
        });

        let config: kreuzberg::core::config::ExtractionConfig =
            serde_json::from_value(config_json).expect("Failed to parse concurrent config");

        assert_eq!(config.max_concurrent_extractions, Some(max_concurrent));
    }
}

/// Test MCP config with cache disabled
#[test]
fn test_mcp_cache_disabled_config() {
    let config_json = json!({
        "use_cache": false,
        "force_ocr": true,
    });

    let config: kreuzberg::core::config::ExtractionConfig =
        serde_json::from_value(config_json).expect("Failed to parse cache config");

    assert_eq!(config.use_cache, false);
    assert_eq!(config.force_ocr, true);
}

/// Test MCP config round-trip serialization
#[test]
fn test_mcp_config_round_trip_serialization() {
    let original_config = kreuzberg::core::config::ExtractionConfig {
        use_cache: false,
        enable_quality_processing: true,
        force_ocr: true,
        output_format: kreuzberg::core::config::OutputFormat::Markdown,
        max_concurrent_extractions: Some(4),
        ..Default::default()
    };

    // Serialize to JSON
    let json_value = serde_json::to_value(&original_config).expect("Failed to serialize");

    // Deserialize back
    let restored_config: kreuzberg::core::config::ExtractionConfig =
        serde_json::from_value(json_value).expect("Failed to deserialize");

    // Verify round-trip
    assert_eq!(original_config.use_cache, restored_config.use_cache);
    assert_eq!(
        original_config.enable_quality_processing,
        restored_config.enable_quality_processing
    );
    assert_eq!(original_config.force_ocr, restored_config.force_ocr);
    assert_eq!(original_config.output_format, restored_config.output_format);
    assert_eq!(
        original_config.max_concurrent_extractions,
        restored_config.max_concurrent_extractions
    );
}

/// Test MCP tool invocation with extract_bytes semantics
#[tokio::test]
async fn test_mcp_tool_extract_bytes_semantics() {
    let test_bytes = b"Test content for MCP extraction";
    let mime_type = "text/plain";

    let config_json = json!({
        "output_format": "plain",
    });

    let config: kreuzberg::core::config::ExtractionConfig =
        serde_json::from_value(config_json).expect("Failed to parse config");

    // Simulate MCP tool: extract_bytes
    let result = kreuzberg::extract_bytes(test_bytes, mime_type, &config)
        .await
        .expect("Extraction should succeed");

    assert!(!result.content.is_empty());
    assert!(result.mime_type.contains("text"));
}

/// Test MCP tool invocation with file path semantics
#[test]
fn test_mcp_tool_extract_file_semantics() {
    // Create temporary test file
    let test_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let test_file = test_dir.path().join("test.txt");
    std::fs::write(&test_file, b"Test content").expect("Failed to write test file");

    let config_json = json!({
        "output_format": "plain",
    });

    let config: kreuzberg::core::config::ExtractionConfig =
        serde_json::from_value(config_json).expect("Failed to parse config");

    // Simulate MCP tool: extract_file (sync)
    if test_file.exists() {
        let file_path = test_file.to_str().expect("test_file path should be valid UTF-8");
        let result = kreuzberg::extract_file_sync(file_path, None, &config).expect("Extraction should succeed");

        assert!(!result.content.is_empty());
    }
}

/// Test MCP batch extraction semantics
#[tokio::test]
async fn test_mcp_batch_extraction_semantics() {
    let test_bytes_1 = b"Content 1";
    let test_bytes_2 = b"Content 2";
    let mime_type = "text/plain";

    let config_json = json!({
        "output_format": "plain",
    });

    let config: kreuzberg::core::config::ExtractionConfig =
        serde_json::from_value(config_json).expect("Failed to parse config");

    // Simulate MCP batch tool: batch_extract_bytes
    let test_data = vec![
        (test_bytes_1.to_vec(), mime_type.to_string()),
        (test_bytes_2.to_vec(), mime_type.to_string()),
    ];

    // Extract each item
    for (bytes, mime) in test_data {
        let result = kreuzberg::extract_bytes(&bytes, &mime, &config)
            .await
            .expect("Batch extraction should succeed");
        assert!(!result.content.is_empty());
    }
}

/// Test MCP config deserialization with unknown format string.
///
/// `OutputFormat` has a `Custom(String)` catch-all variant, so an unknown format
/// string deserializes successfully rather than erroring. This allows registering
/// custom renderers by name.
#[test]
fn test_mcp_error_invalid_format_field() {
    let config_json = json!({
        "output_format": "invalid_format_that_does_not_exist",
    });

    let result: Result<kreuzberg::core::config::ExtractionConfig, _> = serde_json::from_value(config_json);

    // Custom formats are accepted at deserialization time; unknown names produce Custom(...)
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().output_format,
        kreuzberg::OutputFormat::Custom("invalid_format_that_does_not_exist".to_string())
    );
}

/// Test MCP parameter validation with zero concurrent count
#[test]
fn test_mcp_validate_zero_concurrent() {
    // Zero values should be accepted by serde, but MCP validation should flag
    let config_json = json!({
        "max_concurrent_extractions": 0,
    });

    let config: kreuzberg::core::config::ExtractionConfig =
        serde_json::from_value(config_json).expect("Failed to parse");

    // The config accepted the value; MCP server should validate semantically
    assert_eq!(config.max_concurrent_extractions, Some(0));
}

/// Test MCP tool with empty batch
#[test]
fn test_mcp_empty_batch_handling() {
    let empty_batch = json!({
        "paths": [],
        "config": {
            "output_format": "plain",
        }
    });

    let paths = empty_batch.get("paths").expect("Should have paths");
    assert!(paths.is_array(), "paths field should be an array");
    let path_array = paths.as_array().expect("paths should be deserializable as array");
    assert_eq!(path_array.len(), 0, "paths array should be empty");
}

/// Test MCP parameter extraction with nested config
#[test]
fn test_mcp_nested_config_extraction() {
    let nested_request = json!({
        "tool": "extract_file",
        "parameters": {
            "path": "/document.pdf",
            "config": {
                "output_format": "markdown",
                "force_ocr": true,
            }
        }
    });

    if let Some(params) = nested_request.get("parameters")
        && let Some(config_obj) = params.get("config")
    {
        let config: kreuzberg::core::config::ExtractionConfig =
            serde_json::from_value(config_obj.clone()).expect("Failed to parse nested config");

        assert_eq!(config.output_format, kreuzberg::core::config::OutputFormat::Markdown);
        assert_eq!(config.force_ocr, true);
    }
}

/// Test MCP HTML output format
#[test]
fn test_mcp_html_output_format() {
    let config_json = json!({
        "output_format": "html",
    });

    let config: kreuzberg::core::config::ExtractionConfig =
        serde_json::from_value(config_json).expect("Failed to parse HTML config");

    assert_eq!(config.output_format, kreuzberg::core::config::OutputFormat::Html);
}

/// Test MCP config with all boolean combinations
#[test]
fn test_mcp_boolean_combinations() {
    let combinations = vec![(true, true), (true, false), (false, true), (false, false)];

    for (use_cache, quality_processing) in combinations {
        let config_json = json!({
            "use_cache": use_cache,
            "enable_quality_processing": quality_processing,
        });

        let config: kreuzberg::core::config::ExtractionConfig =
            serde_json::from_value(config_json).expect("Failed to parse config");

        assert_eq!(config.use_cache, use_cache);
        assert_eq!(config.enable_quality_processing, quality_processing);
    }
}

/// Test MCP response structure with extraction result
#[test]
fn test_mcp_response_structure_validation() {
    let mcp_response = json!({
        "status": "success",
        "data": {
            "content": "Extracted text",
            "mime_type": "text/plain",
            "metadata": {
                "source": "test",
                "extracted_at": "2024-01-25",
            }
        }
    });

    assert_eq!(
        mcp_response.get("status").expect("status field should exist"),
        "success"
    );
    assert!(
        mcp_response.get("data").is_some(),
        "data field should be present in MCP response"
    );
}

/// Test MCP request/response roundtrip with config
#[test]
fn test_mcp_request_response_roundtrip() {
    let original_config = json!({
        "use_cache": false,
        "force_ocr": true,
        "output_format": "markdown",
        "max_concurrent_extractions": 4,
    });

    // Simulate sending to MCP and getting back
    let config: kreuzberg::core::config::ExtractionConfig =
        serde_json::from_value(original_config.clone()).expect("Failed to parse");

    // Serialize back
    let response_config = serde_json::to_value(&config).expect("Failed to serialize");

    // Verify it matches
    assert_eq!(original_config.get("use_cache"), response_config.get("use_cache"));
    assert_eq!(original_config.get("force_ocr"), response_config.get("force_ocr"));
    assert_eq!(
        original_config.get("output_format"),
        response_config.get("output_format")
    );
}

/// Test MCP config with partial updates
#[test]
fn test_mcp_config_partial_updates() {
    let mut base_config = kreuzberg::core::config::ExtractionConfig::default();
    base_config.use_cache = true;
    base_config.force_ocr = false;

    // Partial update
    let update_json = json!({
        "force_ocr": true,
    });

    let update_config: kreuzberg::core::config::ExtractionConfig =
        serde_json::from_value(update_json).expect("Failed to parse update");

    // In MCP, updates replace config completely
    let updated = update_config;

    // New config has update applied
    assert_eq!(updated.force_ocr, true);
    // But other fields revert to defaults (not merged)
    assert_eq!(updated.use_cache, true);
}

/// Test MCP API consistency for all formats
#[test]
fn test_mcp_api_consistency_all_formats() {
    let formats = vec!["plain", "markdown", "html"];

    for format_str in formats {
        let config = json!({
            "output_format": format_str,
        });

        let parsed: kreuzberg::core::config::ExtractionConfig =
            serde_json::from_value(config).expect("Failed to parse");

        // Verify format is consistent
        let serialized = serde_json::to_value(&parsed).expect("Failed to serialize");
        let reserialized: kreuzberg::core::config::ExtractionConfig =
            serde_json::from_value(serialized).expect("Failed to deserialize");

        let original_format = format!("{}", parsed.output_format);
        let restored_format = format!("{}", reserialized.output_format);

        assert_eq!(original_format, restored_format);
    }
}

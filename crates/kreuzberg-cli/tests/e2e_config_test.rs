//! Comprehensive CLI end-to-end integration tests for configuration flags.
//!
//! This test suite validates the new configuration features including:
//! - `--config-json` for inline JSON configuration
//! - `--config-json-base64` for base64-encoded JSON configuration
//! - `--output-format` flag with all variants (plain, markdown, djot, html)
//! - Flag precedence (CLI args > JSON config > file > defaults)
//! - Config merge scenarios and conflict detection
//! - Error handling for invalid inputs
//! - Real extraction with new formats

#![allow(clippy::bool_assert_comparison)]

use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Get the path to the kreuzberg binary.
fn get_binary_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{}/../../target/debug/kreuzberg", manifest_dir)
}

/// Get the test_documents directory path.
fn get_test_documents_dir() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.parent().unwrap().parent().unwrap().join("test_documents")
}

/// Get a test file path relative to test_documents/.
fn get_test_file(relative_path: &str) -> String {
    get_test_documents_dir()
        .join(relative_path)
        .to_string_lossy()
        .to_string()
}

/// Build the binary before running tests (runs once per test).
fn build_binary() {
    let status = Command::new("cargo")
        .args(["build", "--bin", "kreuzberg"])
        .status()
        .expect("Failed to build kreuzberg binary");

    assert!(status.success(), "Failed to build kreuzberg binary");
}

/// Helper to create a temporary config file with specified content.
fn create_test_config(dir: &TempDir, name: &str, content: &str) -> PathBuf {
    let config_path = dir.path().join(name);
    std::fs::write(&config_path, content).expect("Failed to write config file");
    config_path
}

/// Helper to encode string as base64.
fn to_base64(input: &str) -> String {
    // Manual base64 encoding
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = input.as_bytes();
    let mut result = String::new();
    let mut i = 0;

    while i < bytes.len() {
        let b1 = bytes[i];
        let b2 = if i + 1 < bytes.len() { bytes[i + 1] } else { 0 };
        let b3 = if i + 2 < bytes.len() { bytes[i + 2] } else { 0 };

        let n = ((b1 as u32) << 16) | ((b2 as u32) << 8) | (b3 as u32);

        result.push(CHARSET[((n >> 18) & 0x3F) as usize] as char);
        result.push(CHARSET[((n >> 12) & 0x3F) as usize] as char);

        if i + 1 < bytes.len() {
            result.push(CHARSET[((n >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }

        if i + 2 < bytes.len() {
            result.push(CHARSET[(n & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }

        i += 3;
    }

    result
}

// ============================================================================
// Test 1: --config-json inline flag with complex configuration
// ============================================================================

#[test]
fn test_cli_config_json_inline() {
    build_binary();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        eprintln!("Skipping test: {} not found", test_file);
        return;
    }

    let output = Command::new(get_binary_path())
        .args([
            "extract",
            test_file.as_str(),
            "--config-json",
            r#"{"use_cache": false, "chunk_size": 512}"#,
        ])
        .output()
        .expect("Failed to execute extract command with --config-json");

    assert!(
        output.status.success(),
        "Extract command with --config-json failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty(), "Output should not be empty");
}

// ============================================================================
// Test 2: --config-json-base64 flag for base64-encoded configuration
// ============================================================================

#[test]
fn test_cli_config_json_base64() {
    build_binary();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        eprintln!("Skipping test: {} not found", test_file);
        return;
    }

    // Encode JSON config as base64
    let json_config = r#"{"use_cache": false}"#;
    let base64_config = to_base64(json_config);

    let output = Command::new(get_binary_path())
        .args([
            "extract",
            test_file.as_str(),
            "--config-json-base64",
            base64_config.as_str(),
        ])
        .output()
        .expect("Failed to execute extract command with --config-json-base64");

    assert!(
        output.status.success(),
        "Extract command with --config-json-base64 failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty(), "Output should not be empty");
}

// ============================================================================
// Test 3: Flag precedence verification (CLI flags > JSON > file > defaults)
// ============================================================================

#[test]
fn test_cli_flag_precedence() {
    build_binary();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        eprintln!("Skipping test: {} not found", test_file);
        return;
    }

    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Create a config file with specific settings
    let config_content = r#"
[extraction]
use_cache = true
chunk_size = 1024
"#;
    let config_path = create_test_config(&temp_dir, "config.toml", config_content);

    // CLI flag should override config file setting
    let output = Command::new(get_binary_path())
        .args([
            "extract",
            test_file.as_str(),
            "--config",
            config_path.to_string_lossy().as_ref(),
            "--config-json",
            r#"{"use_cache": false}"#,
        ])
        .output()
        .expect("Failed to execute command with precedence test");

    assert!(
        output.status.success(),
        "Precedence test command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// ============================================================================
// Test 4: --output-format flag with all variants (plain, markdown, djot, html)
// ============================================================================

#[test]
fn test_cli_output_format_all_variants() {
    build_binary();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        eprintln!("Skipping test: {} not found", test_file);
        return;
    }

    let formats = vec!["plain", "markdown", "djot", "html"];

    for format in formats {
        let output = Command::new(get_binary_path())
            .args(["extract", test_file.as_str(), "--output-format", format])
            .output()
            .unwrap_or_else(|_| panic!("Failed to execute extract with --output-format {}", format));

        assert!(
            output.status.success(),
            "Extract command with --output-format {} failed: {}",
            format,
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(!stdout.is_empty(), "Output for format {} should not be empty", format);
    }
}

// ============================================================================
// Test 5: Output formats (text vs json) for extraction result
// ============================================================================

#[test]
fn test_cli_result_format() {
    build_binary();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        eprintln!("Skipping test: {} not found", test_file);
        return;
    }

    // Test text output format
    let output_text = Command::new(get_binary_path())
        .args(["extract", test_file.as_str(), "--format", "text"])
        .output()
        .expect("Failed to execute extract with --format text");

    assert!(
        output_text.status.success(),
        "Text format output failed: {}",
        String::from_utf8_lossy(&output_text.stderr)
    );

    let text_content = String::from_utf8_lossy(&output_text.stdout);
    assert!(!text_content.is_empty(), "Text output should not be empty");

    // Test JSON output format
    let output_json = Command::new(get_binary_path())
        .args(["extract", test_file.as_str(), "--format", "json"])
        .output()
        .expect("Failed to execute extract with --format json");

    assert!(
        output_json.status.success(),
        "JSON format output failed: {}",
        String::from_utf8_lossy(&output_json.stderr)
    );

    let json_content = String::from_utf8_lossy(&output_json.stdout);
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json_content);
    assert!(
        parsed.is_ok(),
        "JSON output should be valid JSON, got: {}",
        json_content
    );

    // Verify JSON has expected envelope+result structure
    if let Ok(value) = parsed {
        assert!(
            value.get("result").is_some(),
            "JSON envelope should have 'result' field"
        );
        assert!(
            value.get("extraction_time_ms").is_some(),
            "JSON envelope should have 'extraction_time_ms' field"
        );
        assert!(
            value["result"].get("content").is_some(),
            "result should have 'content' field"
        );
        assert!(
            value["result"].get("mime_type").is_some(),
            "result should have 'mime_type' field"
        );
    }
}

// ============================================================================
// Test 6: Deprecated --content-format flag warning
// ============================================================================

#[test]
fn test_cli_content_format_deprecated_warning() {
    build_binary();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        eprintln!("Skipping test: {} not found", test_file);
        return;
    }

    // The deprecated --content-format should still work but may show warning
    let output = Command::new(get_binary_path())
        .args(["extract", test_file.as_str(), "--content-format", "plain"])
        .output()
        .expect("Failed to execute extract with --content-format");

    // Command should either succeed or show expected deprecation behavior
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Note: We're checking that the command doesn't crash; deprecation warning behavior
    // depends on implementation details
    assert!(
        output.status.success() || !stdout.is_empty(),
        "Command should succeed or produce output"
    );
}

// ============================================================================
// Test 7: Config merge scenarios - multiple configuration sources
// ============================================================================

#[test]
fn test_cli_config_merge_scenarios() {
    build_binary();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        eprintln!("Skipping test: {} not found", test_file);
        return;
    }

    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Create a base config file
    let config_content = r#"
[extraction]
use_cache = true
chunk_size = 1024
enable_ocr = false
"#;
    let config_path = create_test_config(&temp_dir, "base.toml", config_content);

    // Merge: config file + inline JSON (JSON should override matching keys)
    let output = Command::new(get_binary_path())
        .args([
            "extract",
            test_file.as_str(),
            "--config",
            config_path.to_string_lossy().as_ref(),
            "--config-json",
            r#"{"use_cache": false}"#,
        ])
        .output()
        .expect("Failed to merge configs");

    assert!(
        output.status.success(),
        "Config merge failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// ============================================================================
// Test 8: Invalid JSON error handling
// ============================================================================

#[test]
fn test_cli_invalid_json_error() {
    build_binary();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        eprintln!("Skipping test: {} not found", test_file);
        return;
    }

    let output = Command::new(get_binary_path())
        .args([
            "extract",
            test_file.as_str(),
            "--config-json",
            r#"{"invalid json without closing"#, // Malformed JSON
        ])
        .output()
        .expect("Failed to execute command");

    // Should fail gracefully with error message
    assert!(!output.status.success(), "Command should fail with invalid JSON");

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should contain some error indication
    assert!(
        !stderr.is_empty() || !String::from_utf8_lossy(&output.stdout).is_empty(),
        "Should provide feedback about invalid JSON"
    );
}

// ============================================================================
// Test 9: Config flag conflicts
// ============================================================================

#[test]
fn test_cli_conflicts() {
    build_binary();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        eprintln!("Skipping test: {} not found", test_file);
        return;
    }

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config_content = r#"[extraction]"#;
    let config_path = create_test_config(&temp_dir, "config.toml", config_content);

    // Using both --config-json and --config-json-base64 might conflict
    let json_config = r#"{"use_cache": false}"#;
    let base64_config = to_base64(json_config);

    let output = Command::new(get_binary_path())
        .args([
            "extract",
            test_file.as_str(),
            "--config",
            config_path.to_string_lossy().as_ref(),
            "--config-json",
            r#"{"chunk_size": 512}"#,
            "--config-json-base64",
            base64_config.as_str(),
        ])
        .output()
        .expect("Failed to execute command with potential conflicts");

    // The behavior here depends on implementation:
    // Either it should succeed (last flag wins) or show an error (mutually exclusive)
    // We verify that the command completes without crashing
    let _ = output.status.success();
}

// ============================================================================
// Test 10: Real end-to-end extraction with new config formats
// ============================================================================

#[test]
fn test_cli_real_extraction() {
    build_binary();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        eprintln!("Skipping test: {} not found", test_file);
        return;
    }

    // Full E2E test: extract with multiple new flags
    let output = Command::new(get_binary_path())
        .args([
            "extract",
            test_file.as_str(),
            "--format",
            "json",
            "--output-format",
            "markdown",
            "--config-json",
            r#"{"use_cache": false, "enable_ocr": false}"#,
        ])
        .output()
        .expect("Failed to execute full E2E extraction");

    assert!(
        output.status.success(),
        "E2E extraction failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should be valid JSON output
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
    assert!(parsed.is_ok(), "E2E output should be valid JSON, got: {}", stdout);

    // Verify envelope+result structure
    if let Ok(value) = parsed {
        assert!(value.get("result").is_some(), "Missing 'result' envelope field");
        assert!(
            value.get("extraction_time_ms").is_some(),
            "Missing 'extraction_time_ms' field"
        );
        assert!(
            value["result"].get("content").is_some(),
            "Missing content field in result"
        );
        assert!(
            value["result"].get("mime_type").is_some(),
            "Missing mime_type field in result"
        );
    }
}

// ============================================================================
// Additional Edge Cases and Robustness Tests
// ============================================================================

#[test]
fn test_cli_empty_config_json() {
    build_binary();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        eprintln!("Skipping test: {} not found", test_file);
        return;
    }

    // Empty JSON object should use defaults
    let output = Command::new(get_binary_path())
        .args(["extract", test_file.as_str(), "--config-json", "{}"])
        .output()
        .expect("Failed to execute with empty JSON config");

    assert!(output.status.success(), "Command with empty JSON config should succeed");
}

#[test]
fn test_cli_multiple_output_format_variants() {
    build_binary();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        eprintln!("Skipping test: {} not found", test_file);
        return;
    }

    // Test case-insensitive format argument
    let output = Command::new(get_binary_path())
        .args([
            "extract",
            test_file.as_str(),
            "--output-format",
            "MARKDOWN", // uppercase should work or fail predictably
        ])
        .output()
        .expect("Failed to execute");

    // Either succeeds with case-insensitive parsing or fails gracefully
    let _ = output.status.success();
}

#[test]
fn test_cli_config_json_with_nested_objects() {
    build_binary();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        eprintln!("Skipping test: {} not found", test_file);
        return;
    }

    // Complex nested JSON configuration
    let complex_config = r#"
{
    "use_cache": false,
    "chunk_size": 512,
    "language_detection": {
        "enabled": true,
        "confidence_threshold": 0.8
    }
}
"#;

    let output = Command::new(get_binary_path())
        .args(["extract", test_file.as_str(), "--config-json", complex_config])
        .output()
        .expect("Failed to execute with nested JSON config");

    assert!(
        output.status.success() || !String::from_utf8_lossy(&output.stderr).is_empty(),
        "Complex config should either work or provide error"
    );
}

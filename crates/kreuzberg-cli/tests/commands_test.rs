//! Integration tests for CLI commands (extract, detect, batch).
//!
//! These tests verify that the CLI commands work correctly end-to-end,
//! including input validation, file processing, and output formatting.

use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;

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

/// Build the binary before running tests.
fn build_binary() {
    let status = Command::new("cargo")
        .args(["build", "--bin", "kreuzberg"])
        .status()
        .expect("Failed to build kreuzberg binary");

    assert!(status.success(), "Failed to build kreuzberg binary");
}

#[test]
fn test_extract_text_file() {
    build_binary();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        tracing::debug!("Skipping test: {} not found", test_file);
        return;
    }

    let output = Command::new(get_binary_path())
        .args(["extract", test_file.as_str()])
        .output()
        .expect("Failed to execute extract command");

    assert!(
        output.status.success(),
        "Extract command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty(), "Extract output should not be empty");
}

#[test]
fn test_extract_with_json_output() {
    build_binary();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        tracing::debug!("Skipping test: {} not found", test_file);
        return;
    }

    let output = Command::new(get_binary_path())
        .args(["extract", test_file.as_str(), "--format", "json"])
        .output()
        .expect("Failed to execute extract command");

    assert!(
        output.status.success(),
        "Extract command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    let json_result: serde_json::Result<serde_json::Value> = serde_json::from_str(&stdout);
    assert!(json_result.is_ok(), "Output should be valid JSON, got: {}", stdout);

    let json = json_result.unwrap();
    assert!(json.get("content").is_some(), "JSON should have 'content' field");
    assert!(json.get("mime_type").is_some(), "JSON should have 'mime_type' field");
}

#[test]
fn test_extract_with_chunking() {
    build_binary();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        tracing::debug!("Skipping test: {} not found", test_file);
        return;
    }

    let output = Command::new(get_binary_path())
        .args([
            "extract",
            test_file.as_str(),
            "--chunk",
            "true",
            "--chunk-size",
            "100",
            "--chunk-overlap",
            "20",
            "--format",
            "json",
        ])
        .output()
        .expect("Failed to execute extract command");

    assert!(
        output.status.success(),
        "Extract with chunking failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Should be valid JSON");

    assert!(json.get("chunks").is_some(), "JSON should have 'chunks' field");
    assert!(json["chunks"].is_array(), "'chunks' should be an array");
}

#[test]
fn test_extract_file_not_found() {
    build_binary();

    let output = Command::new(get_binary_path())
        .args(["extract", "/nonexistent/file.txt"])
        .output()
        .expect("Failed to execute extract command");

    assert!(!output.status.success(), "Extract should fail for nonexistent file");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("File not found"),
        "Error should mention file not found, got: {}",
        stderr
    );
}

#[test]
fn test_extract_directory_not_file() {
    build_binary();

    let tmp_dir = tempdir().expect("Failed to create temp dir");
    let dir_path = tmp_dir.path().to_string_lossy().to_string();

    let output = Command::new(get_binary_path())
        .args(["extract", dir_path.as_str()])
        .output()
        .expect("Failed to execute extract command");

    assert!(!output.status.success(), "Extract should fail for directory");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not a file") || stderr.contains("regular file"),
        "Error should mention path is not a file, got: {}",
        stderr
    );
}

#[test]
fn test_extract_invalid_chunk_size_zero() {
    build_binary();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        tracing::debug!("Skipping test: {} not found", test_file);
        return;
    }

    let output = Command::new(get_binary_path())
        .args(["extract", test_file.as_str(), "--chunk-size", "0"])
        .output()
        .expect("Failed to execute extract command");

    assert!(!output.status.success(), "Extract should fail for chunk size 0");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Invalid chunk size") || stderr.contains("must be greater than 0"),
        "Error should mention invalid chunk size, got: {}",
        stderr
    );
}

#[test]
fn test_extract_invalid_chunk_size_too_large() {
    build_binary();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        tracing::debug!("Skipping test: {} not found", test_file);
        return;
    }

    let output = Command::new(get_binary_path())
        .args(["extract", test_file.as_str(), "--chunk-size", "2000000"])
        .output()
        .expect("Failed to execute extract command");

    assert!(!output.status.success(), "Extract should fail for chunk size > 1M");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Invalid chunk size") || stderr.contains("1,000,000"),
        "Error should mention chunk size limit, got: {}",
        stderr
    );
}

#[test]
fn test_extract_invalid_overlap_equals_chunk_size() {
    build_binary();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        tracing::debug!("Skipping test: {} not found", test_file);
        return;
    }

    let output = Command::new(get_binary_path())
        .args([
            "extract",
            test_file.as_str(),
            "--chunk-size",
            "100",
            "--chunk-overlap",
            "100",
        ])
        .output()
        .expect("Failed to execute extract command");

    assert!(
        !output.status.success(),
        "Extract should fail when overlap equals chunk size"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Invalid chunk overlap") || stderr.contains("must be less than chunk size"),
        "Error should mention overlap constraint, got: {}",
        stderr
    );
}

#[test]
fn test_detect_mime_type() {
    build_binary();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        tracing::debug!("Skipping test: {} not found", test_file);
        return;
    }

    let output = Command::new(get_binary_path())
        .args(["detect", test_file.as_str()])
        .output()
        .expect("Failed to execute detect command");

    assert!(
        output.status.success(),
        "Detect command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty(), "Detect output should not be empty");
    assert!(
        stdout.contains("text/plain") || stdout.contains("text"),
        "Should detect text MIME type, got: {}",
        stdout
    );
}

#[test]
fn test_detect_with_json_output() {
    build_binary();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        tracing::debug!("Skipping test: {} not found", test_file);
        return;
    }

    let output = Command::new(get_binary_path())
        .args(["detect", test_file.as_str(), "--format", "json"])
        .output()
        .expect("Failed to execute detect command");

    assert!(
        output.status.success(),
        "Detect command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    let json_result: serde_json::Result<serde_json::Value> = serde_json::from_str(&stdout);
    assert!(json_result.is_ok(), "Output should be valid JSON, got: {}", stdout);

    let json = json_result.unwrap();
    assert!(json.get("mime_type").is_some(), "JSON should have 'mime_type' field");
    assert!(json.get("path").is_some(), "JSON should have 'path' field");
}

#[test]
fn test_detect_file_not_found() {
    build_binary();

    let output = Command::new(get_binary_path())
        .args(["detect", "/nonexistent/file.txt"])
        .output()
        .expect("Failed to execute detect command");

    assert!(!output.status.success(), "Detect should fail for nonexistent file");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("File not found"),
        "Error should mention file not found, got: {}",
        stderr
    );
}

#[test]
fn test_batch_multiple_files() {
    build_binary();

    let file1 = get_test_file("text/simple.txt");
    let file2 = get_test_file("text/simple.txt");

    if !PathBuf::from(&file1).exists() {
        tracing::debug!("Skipping test: {} not found", file1);
        return;
    }

    let output = Command::new(get_binary_path())
        .args(["batch", file1.as_str(), file2.as_str(), "--format", "json"])
        .output()
        .expect("Failed to execute batch command");

    assert!(
        output.status.success(),
        "Batch command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    let json_result: serde_json::Result<serde_json::Value> = serde_json::from_str(&stdout);
    assert!(json_result.is_ok(), "Output should be valid JSON, got: {}", stdout);

    let json = json_result.unwrap();
    assert!(json.is_array(), "Batch output should be a JSON array");
    assert_eq!(json.as_array().unwrap().len(), 2, "Should have 2 results");
}

#[test]
fn test_batch_with_missing_file() {
    build_binary();

    let valid_file = get_test_file("text/simple.txt");

    if !PathBuf::from(&valid_file).exists() {
        tracing::debug!("Skipping test: {} not found", valid_file);
        return;
    }

    let output = Command::new(get_binary_path())
        .args(["batch", valid_file.as_str(), "/nonexistent/file.txt"])
        .output()
        .expect("Failed to execute batch command");

    assert!(!output.status.success(), "Batch should fail when one file is missing");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("File not found") || stderr.contains("Invalid file"),
        "Error should mention file not found, got: {}",
        stderr
    );
}

#[test]
fn test_extract_help() {
    build_binary();

    let output = Command::new(get_binary_path())
        .args(["extract", "--help"])
        .output()
        .expect("Failed to execute extract --help");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Extract text from a document"));
    assert!(stdout.contains("--chunk-size"));
    assert!(stdout.contains("--chunk-overlap"));
}

#[test]
fn test_detect_help() {
    build_binary();

    let output = Command::new(get_binary_path())
        .args(["detect", "--help"])
        .output()
        .expect("Failed to execute detect --help");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Detect MIME type"));
}

#[test]
fn test_batch_help() {
    build_binary();

    let output = Command::new(get_binary_path())
        .args(["batch", "--help"])
        .output()
        .expect("Failed to execute batch --help");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Batch extract from multiple documents"));
}

// ── Extract command flag parsing tests ──────────────────────────────

#[test]
fn test_extract_help_shows_all_extraction_override_flags() {
    build_binary();

    let output = Command::new(get_binary_path())
        .args(["extract", "--help"])
        .output()
        .expect("Failed to execute extract --help");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify all ExtractionOverrides flags appear in help output
    let expected_flags = [
        "--ocr",
        "--ocr-backend",
        "--ocr-language",
        "--force-ocr",
        "--no-cache",
        "--ocr-auto-rotate",
        "--chunk",
        "--chunk-size",
        "--chunk-overlap",
        "--chunking-tokenizer",
        "--output-format",
        "--include-structure",
        "--quality",
        "--detect-language",
        "--acceleration",
        "--max-concurrent",
        "--max-threads",
        "--extract-pages",
        "--page-markers",
        "--extract-images",
        "--target-dpi",
        "--pdf-password",
        "--token-reduction",
        "--msg-codepage",
    ];

    for flag in &expected_flags {
        assert!(
            stdout.contains(flag),
            "Extract --help should show flag '{}', but it was not found in output:\n{}",
            flag,
            stdout
        );
    }
}

// ── Batch command flag parity test ──────────────────────────────────

#[test]
fn test_batch_has_same_extraction_flags_as_extract() {
    build_binary();

    let extract_output = Command::new(get_binary_path())
        .args(["extract", "--help"])
        .output()
        .expect("Failed to execute extract --help");

    let batch_output = Command::new(get_binary_path())
        .args(["batch", "--help"])
        .output()
        .expect("Failed to execute batch --help");

    assert!(extract_output.status.success());
    assert!(batch_output.status.success());

    let extract_help = String::from_utf8_lossy(&extract_output.stdout);
    let batch_help = String::from_utf8_lossy(&batch_output.stdout);

    // All extraction override flags should be present on both commands
    let shared_flags = [
        "--ocr",
        "--ocr-backend",
        "--ocr-language",
        "--force-ocr",
        "--no-cache",
        "--chunk",
        "--chunk-size",
        "--chunk-overlap",
        "--output-format",
        "--quality",
        "--detect-language",
        "--acceleration",
        "--max-concurrent",
        "--max-threads",
        "--extract-pages",
        "--page-markers",
        "--extract-images",
        "--target-dpi",
        "--pdf-password",
        "--token-reduction",
        "--msg-codepage",
    ];

    for flag in &shared_flags {
        assert!(
            extract_help.contains(flag),
            "Extract should have flag '{}' but it's missing",
            flag
        );
        assert!(
            batch_help.contains(flag),
            "Batch should have flag '{}' (parity with extract) but it's missing",
            flag
        );
    }
}

// ── Validation error tests ──────────────────────────────────────────
//
// NOTE: The CLI validates file existence *before* override validation,
// so we must provide a real file to reach the override validation stage.

/// Create a temporary file and return its path as a String.
/// The caller must keep the returned `tempfile::TempDir` alive for the
/// duration of the test so the file is not deleted.
fn create_temp_file() -> (tempfile::TempDir, String) {
    let dir = tempdir().expect("Failed to create temp dir");
    let file_path = dir.path().join("dummy.pdf");
    std::fs::write(&file_path, b"dummy content").expect("Failed to write temp file");
    let path_str = file_path.to_string_lossy().to_string();
    (dir, path_str)
}

#[test]
fn test_extract_chunk_size_zero_error() {
    build_binary();
    let (_dir, file_path) = create_temp_file();

    let output = Command::new(get_binary_path())
        .args(["extract", "--chunk-size", "0", &file_path])
        .output()
        .expect("Failed to execute extract command");

    assert!(!output.status.success(), "Should fail when chunk size is 0");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("chunk size") || stderr.contains("Chunk size") || stderr.contains("Invalid chunk size"),
        "Error should mention chunk size, got: {}",
        stderr
    );
}

#[test]
fn test_extract_chunk_overlap_exceeds_size_error() {
    build_binary();
    let (_dir, file_path) = create_temp_file();

    let output = Command::new(get_binary_path())
        .args(["extract", "--chunk-size", "10", "--chunk-overlap", "20", &file_path])
        .output()
        .expect("Failed to execute extract command");

    assert!(!output.status.success(), "Should fail when overlap exceeds chunk size");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("overlap") || stderr.contains("Overlap") || stderr.contains("Invalid chunk overlap"),
        "Error should mention overlap constraint, got: {}",
        stderr
    );
}

#[test]
fn test_extract_layout_confidence_out_of_range_error() {
    build_binary();
    let (_dir, file_path) = create_temp_file();

    let output = Command::new(get_binary_path())
        .args(["extract", "--layout-confidence", "2.0", &file_path])
        .output()
        .expect("Failed to execute extract command");

    // This flag is feature-gated behind layout-detection. If the binary was
    // built without that feature, clap itself will reject the unknown flag.
    assert!(
        !output.status.success(),
        "Should fail for layout confidence out of range"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("confidence") || stderr.contains("layout") || stderr.contains("unexpected argument"),
        "Error should mention confidence or layout, got: {}",
        stderr
    );
}

#[test]
fn test_extract_target_dpi_zero_error() {
    build_binary();
    let (_dir, file_path) = create_temp_file();

    let output = Command::new(get_binary_path())
        .args(["extract", "--target-dpi", "0", &file_path])
        .output()
        .expect("Failed to execute extract command");

    assert!(!output.status.success(), "Should fail when target DPI is 0");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("DPI") || stderr.contains("dpi") || stderr.contains("target") || stderr.contains("Invalid"),
        "Error should mention DPI range, got: {}",
        stderr
    );
}

// ── Completions test ────────────────────────────────────────────────

#[test]
fn test_completions_bash_produces_output() {
    build_binary();

    let output = Command::new(get_binary_path())
        .args(["completions", "bash"])
        .output()
        .expect("Failed to execute completions command");

    assert!(
        output.status.success(),
        "Completions command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty(), "Completions output should not be empty");
    // bash completions should contain the command name
    assert!(
        stdout.contains("kreuzberg"),
        "Bash completions should reference 'kreuzberg', got: {}",
        &stdout[..stdout.len().min(200)]
    );
}

#[test]
fn test_completions_zsh_produces_output() {
    build_binary();

    let output = Command::new(get_binary_path())
        .args(["completions", "zsh"])
        .output()
        .expect("Failed to execute completions command");

    assert!(
        output.status.success(),
        "Completions command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty(), "Zsh completions output should not be empty");
}

#[test]
fn test_completions_fish_produces_output() {
    build_binary();

    let output = Command::new(get_binary_path())
        .args(["completions", "fish"])
        .output()
        .expect("Failed to execute completions command");

    assert!(
        output.status.success(),
        "Completions command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty(), "Fish completions output should not be empty");
}

// ── Embed help test ─────────────────────────────────────────────────

#[test]
fn test_embed_help_shows_correct_flags() {
    build_binary();

    let output = Command::new(get_binary_path())
        .args(["embed", "--help"])
        .output()
        .expect("Failed to execute embed --help");

    // embed is feature-gated; if not compiled, clap will show an error
    if !output.status.success() {
        // If embed subcommand doesn't exist, skip the test
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("unrecognized subcommand") || stderr.contains("invalid subcommand") {
            return;
        }
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--text"),
        "Embed help should show --text flag, got: {}",
        stdout
    );
    assert!(
        stdout.contains("--preset"),
        "Embed help should show --preset flag, got: {}",
        stdout
    );
    assert!(
        stdout.contains("--format"),
        "Embed help should show --format flag, got: {}",
        stdout
    );
    assert!(
        stdout.contains("Generate embeddings"),
        "Embed help should describe embedding generation, got: {}",
        stdout
    );
}

// ── Chunk help test ─────────────────────────────────────────────────

#[test]
fn test_chunk_help_shows_correct_flags() {
    build_binary();

    let output = Command::new(get_binary_path())
        .args(["chunk", "--help"])
        .output()
        .expect("Failed to execute chunk --help");

    assert!(
        output.status.success(),
        "Chunk --help failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--text"),
        "Chunk help should show --text flag, got: {}",
        stdout
    );
    assert!(
        stdout.contains("--chunk-size"),
        "Chunk help should show --chunk-size flag, got: {}",
        stdout
    );
    assert!(
        stdout.contains("--chunk-overlap"),
        "Chunk help should show --chunk-overlap flag, got: {}",
        stdout
    );
    assert!(
        stdout.contains("--chunker-type"),
        "Chunk help should show --chunker-type flag, got: {}",
        stdout
    );
    assert!(
        stdout.contains("--format"),
        "Chunk help should show --format flag, got: {}",
        stdout
    );
    assert!(
        stdout.contains("Chunk text"),
        "Chunk help should describe text chunking, got: {}",
        stdout
    );
}

// ── Style module NO_COLOR test ──────────────────────────────────────

#[test]
fn test_no_color_env_disables_ansi_in_output() {
    build_binary();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        return;
    }

    // Run with NO_COLOR set - output should have no ANSI escape sequences
    let output = Command::new(get_binary_path())
        .env("NO_COLOR", "1")
        .args(["detect", &test_file])
        .output()
        .expect("Failed to execute detect command");

    assert!(
        output.status.success(),
        "Detect failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("\x1b["),
        "Output should not contain ANSI escape sequences when NO_COLOR is set, got: {:?}",
        stdout
    );
}

// ── Additional validation edge cases ────────────────────────────────

#[test]
fn test_extract_chunk_size_too_large_error() {
    build_binary();
    let (_dir, file_path) = create_temp_file();

    let output = Command::new(get_binary_path())
        .args(["extract", "--chunk-size", "2000000", &file_path])
        .output()
        .expect("Failed to execute extract command");

    assert!(!output.status.success(), "Should fail when chunk size exceeds limit");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("chunk size") || stderr.contains("Chunk size") || stderr.contains("1,000,000"),
        "Error should mention chunk size limit, got: {}",
        stderr
    );
}

#[test]
fn test_extract_target_dpi_too_high_error() {
    build_binary();
    let (_dir, file_path) = create_temp_file();

    let output = Command::new(get_binary_path())
        .args(["extract", "--target-dpi", "5000", &file_path])
        .output()
        .expect("Failed to execute extract command");

    assert!(!output.status.success(), "Should fail when target DPI exceeds limit");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("DPI") || stderr.contains("dpi") || stderr.contains("2400") || stderr.contains("Invalid"),
        "Error should mention DPI range, got: {}",
        stderr
    );
}

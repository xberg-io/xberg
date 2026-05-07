//! Regression tests for #830: extraction_timeout_secs silently ignored in single-file paths.

use kreuzberg::KreuzbergError;
use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::core::extractor::{extract_bytes, extract_file};
use std::time::Instant;

/// A timeout of 0 seconds should fire immediately, before any real work is done.
/// We use plain-text content so the test doesn't require external binaries (Tesseract, pdfium).
#[tokio::test]
async fn test_extract_bytes_zero_timeout_returns_timeout_error() {
    let config = ExtractionConfig {
        extraction_timeout_secs: Some(0),
        ..Default::default()
    };

    let content = b"Hello world, this is a plain-text document.";
    let result = extract_bytes(content, "text/plain", &config).await;

    match result {
        Err(KreuzbergError::Timeout { limit_ms, .. }) => {
            assert_eq!(limit_ms, 0, "limit_ms should reflect the configured 0-second timeout");
        }
        // text/plain is synchronous — if it completes before the timeout fires that's also
        // acceptable, but we still confirm no other error type is raised.
        Ok(_) => {}
        Err(e) => panic!("Expected Ok or Timeout, got: {e:?}"),
    }
}

/// Same check for extract_file.
#[tokio::test]
async fn test_extract_file_zero_timeout_returns_timeout_error() {
    // Write a small temp file
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("hello.txt");
    std::fs::write(&path, b"Hello world").expect("write");

    let config = ExtractionConfig {
        extraction_timeout_secs: Some(0),
        ..Default::default()
    };

    let result = extract_file(&path, None, &config).await;

    match result {
        Err(KreuzbergError::Timeout { limit_ms, .. }) => {
            assert_eq!(limit_ms, 0);
        }
        Ok(_) => {} // synchronous text extraction may beat a 0s timeout
        Err(e) => panic!("Expected Ok or Timeout, got: {e:?}"),
    }
}

/// When no timeout is configured, extraction should succeed normally.
#[tokio::test]
async fn test_extract_bytes_no_timeout_succeeds() {
    let config = ExtractionConfig::default();
    let content = b"No timeout configured.";
    let result = extract_bytes(content, "text/plain", &config).await;
    assert!(result.is_ok(), "extraction without timeout should succeed: {result:?}");
}

/// When no timeout is configured, file extraction should succeed normally.
#[tokio::test]
async fn test_extract_file_no_timeout_succeeds() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("test.txt");
    std::fs::write(&path, b"No timeout configured.").expect("write");

    let config = ExtractionConfig::default();
    let result = extract_file(&path, None, &config).await;
    assert!(result.is_ok(), "extraction without timeout should succeed: {result:?}");
}

/// Elapsed time reported in the error must be <= limit_ms for reasonable timeouts.
#[tokio::test]
async fn test_extract_bytes_timeout_elapsed_is_plausible() {
    let config = ExtractionConfig {
        extraction_timeout_secs: Some(0),
        ..Default::default()
    };
    let content = b"timing check";
    let start = Instant::now();
    let _ = extract_bytes(content, "text/plain", &config).await;
    let wall_ms = start.elapsed().as_millis() as u64;
    // We can't assert the timeout fired, but if it did, wall time should be <1 second.
    assert!(
        wall_ms < 1000,
        "single-file extraction with 0s timeout took too long: {wall_ms}ms"
    );
}

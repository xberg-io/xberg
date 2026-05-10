//! Integration tests for the JSON timing envelope added to `kreuzberg extract` and
//! `kreuzberg batch`.
//!
//! Verifies:
//!  - `extract --format json` emits `{ result, extraction_time_ms }` shape
//!  - `batch --format json` emits `{ results, total_ms, per_file_ms }` shape
//!  - `result.metadata.ocr_used` exists as a bool field
//!  - `--pdf-backend xyz` exits non-zero and mentions "pdf-oxide"

use std::path::{Path, PathBuf};
use std::process::Command;

/// Returns path to the compiled `kreuzberg` binary (debug build).
fn kreuzberg_bin() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .expect("crates/kreuzberg-cli parent")
        .parent()
        .expect("crates parent")
        .join("target")
        .join("debug")
        .join("kreuzberg")
}

/// Returns path to the small reference PDF used in these tests.
fn pdf_fixture() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .expect("crates/kreuzberg-cli parent")
        .parent()
        .expect("crates parent")
        .join("test_documents")
        .join("pdf")
        .join("pdfa_001.pdf")
}

/// Returns path to the small plain-text fixture used for batch tests.
fn txt_fixture() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .expect("crates/kreuzberg-cli parent")
        .parent()
        .expect("crates parent")
        .join("test_documents")
        .join("text")
        .join("fake_text.txt")
}

/// Build the binary once before running. Panics on failure.
fn build_binary() {
    let status = Command::new("cargo")
        .args(["build", "--bin", "kreuzberg"])
        .status()
        .expect("cargo build invocation failed");
    assert!(status.success(), "cargo build failed — binary unavailable");
}

/// Skip-guard: returns `true` when the fixture exists so the test can run.
fn fixture_exists(path: &Path) -> bool {
    path.exists() && path.is_file()
}

// ── extract --format json envelope ──────────────────────────────────────────

#[test]
fn test_extract_json_has_result_and_timing() {
    build_binary();

    let pdf = pdf_fixture();
    if !fixture_exists(&pdf) {
        eprintln!("SKIP: PDF fixture not found at {}", pdf.display());
        return;
    }

    let output = Command::new(kreuzberg_bin())
        .args(["extract", &pdf.to_string_lossy(), "--format", "json"])
        .output()
        .expect("failed to run kreuzberg extract");

    assert!(
        output.status.success(),
        "extract exited non-zero: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("stdout is not valid JSON");

    // Envelope shape
    assert!(json.get("result").is_some(), "missing 'result' key in envelope");
    let extraction_time_ms = json
        .get("extraction_time_ms")
        .and_then(|v| v.as_f64())
        .expect("'extraction_time_ms' must be a number");
    assert!(
        extraction_time_ms > 0.0,
        "extraction_time_ms must be positive, got {extraction_time_ms}"
    );

    // ocr_used field must exist as a bool
    let ocr_used = json["result"]["metadata"]
        .get("ocr_used")
        .expect("'result.metadata.ocr_used' must be present")
        .as_bool()
        .expect("'result.metadata.ocr_used' must be a boolean");
    // For a native-text PDF without --force-ocr, OCR should NOT have run.
    assert!(!ocr_used, "expected ocr_used=false for native PDF extraction");
}

// ── batch --format json envelope ─────────────────────────────────────────────

#[test]
fn test_batch_json_has_results_and_timing() {
    build_binary();

    let pdf = pdf_fixture();
    let txt = txt_fixture();
    if !fixture_exists(&pdf) || !fixture_exists(&txt) {
        eprintln!("SKIP: one or more batch fixtures not found");
        return;
    }

    let output = Command::new(kreuzberg_bin())
        .args([
            "batch",
            &pdf.to_string_lossy(),
            &txt.to_string_lossy(),
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run kreuzberg batch");

    assert!(
        output.status.success(),
        "batch exited non-zero: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("stdout is not valid JSON");

    // Envelope shape
    let results = json
        .get("results")
        .and_then(|v| v.as_array())
        .expect("'results' must be an array");
    assert_eq!(results.len(), 2, "expected 2 results for 2 input files");

    let total_ms = json
        .get("total_ms")
        .and_then(|v| v.as_f64())
        .expect("'total_ms' must be a number");
    assert!(total_ms > 0.0, "total_ms must be positive, got {total_ms}");

    let per_file_ms = json
        .get("per_file_ms")
        .and_then(|v| v.as_array())
        .expect("'per_file_ms' must be an array");
    assert_eq!(per_file_ms.len(), 2, "per_file_ms must have one entry per file");

    for (i, timing) in per_file_ms.iter().enumerate() {
        let ms = timing.as_f64().expect("per_file_ms entry must be a number");
        assert!(ms > 0.0, "per_file_ms[{i}] must be positive, got {ms}");
    }

    // Each result must have metadata.ocr_used as a bool
    for (i, result) in results.iter().enumerate() {
        assert!(
            result["metadata"].get("ocr_used").and_then(|v| v.as_bool()).is_some(),
            "results[{i}].metadata.ocr_used must be a bool"
        );
    }
}

// ── --pdf-backend validation ─────────────────────────────────────────────────

#[test]
fn test_pdf_backend_invalid_value_exits_nonzero() {
    build_binary();

    let pdf = pdf_fixture();
    if !fixture_exists(&pdf) {
        eprintln!("SKIP: PDF fixture not found at {}", pdf.display());
        return;
    }

    let output = Command::new(kreuzberg_bin())
        .args(["extract", &pdf.to_string_lossy(), "--pdf-backend", "xyz"])
        .output()
        .expect("failed to run kreuzberg extract");

    assert!(
        !output.status.success(),
        "expected non-zero exit for unknown --pdf-backend"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("pdf-oxide"),
        "error message should mention 'pdf-oxide', got: {stderr}"
    );
}

#[test]
fn test_pdf_backend_valid_value_succeeds() {
    build_binary();

    let pdf = pdf_fixture();
    if !fixture_exists(&pdf) {
        eprintln!("SKIP: PDF fixture not found at {}", pdf.display());
        return;
    }

    let output = Command::new(kreuzberg_bin())
        .args([
            "extract",
            &pdf.to_string_lossy(),
            "--pdf-backend",
            "pdf-oxide",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run kreuzberg extract");

    assert!(
        output.status.success(),
        "--pdf-backend pdf-oxide should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("stdout is not valid JSON");
    assert!(json.get("result").is_some(), "missing 'result' key");
    assert!(json.get("extraction_time_ms").is_some(), "missing 'extraction_time_ms'");
}

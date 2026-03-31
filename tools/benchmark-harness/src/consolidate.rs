//! Loading benchmark results from disk for consolidation
//!
//! This module provides `load_run_results` which recursively loads benchmark
//! result JSON files from a directory tree, tagging them with batch mode info
//! inferred from directory names.

use crate::types::BenchmarkResult;
use crate::{Error, Result};
use std::fs;
use std::path::Path;

/// Load benchmark results from `results.json` files in a directory.
///
/// Recursively walks the given directory, loading any `results.json` files found.
/// For directories whose name ends with `-batch`, the framework name in each result
/// is suffixed with `-batch` so that the aggregation layer can distinguish single-
/// vs batch-mode results.
///
/// # Errors
///
/// Returns [`Error::Io`] if the directory cannot be read, or [`Error::Benchmark`]
/// if a `results.json` file contains invalid JSON or fails validation.
pub fn load_run_results(dir: &Path) -> Result<Vec<BenchmarkResult>> {
    let mut results = Vec::new();
    for entry in fs::read_dir(dir).map_err(Error::Io)? {
        let entry = entry.map_err(Error::Io)?;
        let path = entry.path();

        if path.is_file() && path.file_name().is_some_and(|n| n == "results.json") {
            eprintln!("Loading results from {}", path.display());
            let json_content = fs::read_to_string(&path).map_err(Error::Io)?;
            let mut run_results: Vec<BenchmarkResult> = serde_json::from_str(&json_content)
                .map_err(|e| Error::Benchmark(format!("Failed to parse {}: {}", path.display(), e)))?;

            // Infer benchmark mode from the parent directory name.
            // The runner outputs to `benchmark-results/{FRAMEWORK}-{MODE}/results.json`
            // where MODE is "batch" or "single-file". The framework field inside
            // results.json does NOT include the mode, so we tag it here to allow
            // the aggregation to distinguish single vs batch results.
            let dir_name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let is_batch = dir_name.ends_with("-batch");

            if is_batch {
                for result in &mut run_results {
                    if !result.framework.ends_with("-batch") {
                        result.framework = format!("{}-batch", result.framework);
                    }
                }
            }

            // Validate loaded results
            for result in &run_results {
                crate::output::validate_result(result)
                    .map_err(|e| Error::Benchmark(format!("Invalid result in {}: {}", path.display(), e)))?;
            }

            results.extend(run_results);
        } else if path.is_dir() {
            match load_run_results(&path) {
                Ok(mut run_results) => results.append(&mut run_results),
                Err(e) => eprintln!("Warning: Failed to load results from {}: {}", path.display(), e),
            }
        }
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ErrorKind, FrameworkCapabilities, PerformanceMetrics};
    use std::time::Duration;

    /// Build a minimal valid `BenchmarkResult` for testing.
    fn make_result(framework: &str) -> BenchmarkResult {
        BenchmarkResult {
            framework: framework.to_string(),
            file_path: std::path::PathBuf::from("test.pdf"),
            file_size: 1024,
            success: true,
            error_message: None,
            error_kind: ErrorKind::None,
            duration: Duration::from_millis(100),
            extraction_duration: None,
            subprocess_overhead: None,
            metrics: PerformanceMetrics {
                peak_memory_bytes: 1_000_000,
                avg_cpu_percent: 50.0,
                throughput_bytes_per_sec: 10_240.0,
                p50_memory_bytes: 900_000,
                p95_memory_bytes: 950_000,
                p99_memory_bytes: 990_000,
            },
            quality: None,
            iterations: vec![],
            statistics: None,
            cold_start_duration: None,
            file_extension: "pdf".to_string(),
            framework_capabilities: FrameworkCapabilities::default(),
            pdf_metadata: None,
            ocr_status: Default::default(),
            extracted_text: None,
        }
    }

    #[test]
    fn test_load_single_results_file() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let results = vec![make_result("kreuzberg-rust")];
        let json = serde_json::to_string(&results).expect("serialize");
        fs::write(dir.path().join("results.json"), &json).expect("write");

        let loaded = load_run_results(dir.path()).expect("load");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].framework, "kreuzberg-rust");
    }

    #[test]
    fn test_batch_directory_tags_framework_name() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let batch_dir = dir.path().join("kreuzberg-rust-batch");
        fs::create_dir_all(&batch_dir).expect("create subdir");

        let results = vec![make_result("kreuzberg-rust")];
        let json = serde_json::to_string(&results).expect("serialize");
        fs::write(batch_dir.join("results.json"), &json).expect("write");

        let loaded = load_run_results(dir.path()).expect("load");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].framework, "kreuzberg-rust-batch");
    }

    #[test]
    fn test_batch_suffix_not_doubled() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let batch_dir = dir.path().join("kreuzberg-rust-batch");
        fs::create_dir_all(&batch_dir).expect("create subdir");

        let results = vec![make_result("kreuzberg-rust-batch")];
        let json = serde_json::to_string(&results).expect("serialize");
        fs::write(batch_dir.join("results.json"), &json).expect("write");

        let loaded = load_run_results(dir.path()).expect("load");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].framework, "kreuzberg-rust-batch");
    }

    #[test]
    fn test_recursive_loading() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let sub1 = dir.path().join("framework-a");
        let sub2 = dir.path().join("framework-b");
        fs::create_dir_all(&sub1).expect("create subdir 1");
        fs::create_dir_all(&sub2).expect("create subdir 2");

        fs::write(
            sub1.join("results.json"),
            serde_json::to_string(&vec![make_result("framework-a")]).expect("serialize"),
        )
        .expect("write a");
        fs::write(
            sub2.join("results.json"),
            serde_json::to_string(&vec![make_result("framework-b")]).expect("serialize"),
        )
        .expect("write b");

        let loaded = load_run_results(dir.path()).expect("load");
        assert_eq!(loaded.len(), 2);
        let names: Vec<&str> = loaded.iter().map(|r| r.framework.as_str()).collect();
        assert!(names.contains(&"framework-a"));
        assert!(names.contains(&"framework-b"));
    }

    #[test]
    fn test_malformed_json_returns_error() {
        let dir = tempfile::tempdir().expect("create temp dir");
        fs::write(dir.path().join("results.json"), "NOT VALID JSON").expect("write");

        let result = load_run_results(dir.path());
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("Failed to parse"));
    }

    #[test]
    fn test_empty_directory_returns_empty_vec() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let loaded = load_run_results(dir.path()).expect("load");
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_nonexistent_directory_returns_error() {
        let result = load_run_results(Path::new("/tmp/nonexistent_benchmark_dir_12345"));
        assert!(result.is_err());
    }
}

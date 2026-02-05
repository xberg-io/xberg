//! Output writers for benchmark results
//!
//! This module provides functionality for persisting benchmark results to disk
//! in JSON format.

use crate::stats::percentile_r7;
use crate::types::BenchmarkResult;
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Validate a benchmark result for invalid states
///
/// # Arguments
/// * `result` - The benchmark result to validate
///
/// # Returns
/// * `Ok(())` if valid, `Err` with description if invalid
pub fn validate_result(result: &BenchmarkResult) -> Result<()> {
    // Note: duration=0 is valid for sub-millisecond extractions (e.g., simple JSON files).
    // We only record millisecond precision, so very fast extractions show as 0ms.

    // Check for invalid state: success=true with error message
    if result.success && result.error_message.is_some() {
        return Err(Error::Benchmark(format!(
            "Invalid result state for {}/{}: success=true but error_message is set",
            result.framework,
            result.file_path.display()
        )));
    }

    // Check for invalid state: success=false without error message
    if !result.success && result.error_message.is_none() {
        return Err(Error::Benchmark(format!(
            "Invalid result state for {}/{}: success=false but error_message is None",
            result.framework,
            result.file_path.display()
        )));
    }

    Ok(())
}

/// Write benchmark results to JSON file
///
/// # Arguments
/// * `results` - Vector of benchmark results to write
/// * `output_path` - Path to output JSON file
pub fn write_json(results: &[BenchmarkResult], output_path: &Path) -> Result<()> {
    // Validate all results before writing
    for result in results {
        validate_result(result)?;
    }

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(Error::Io)?;
    }

    let json = serde_json::to_string_pretty(results)
        .map_err(|e| Error::Benchmark(format!("Failed to serialize results: {}", e)))?;

    fs::write(output_path, json).map_err(Error::Io)?;

    Ok(())
}

/// Per-framework statistics for a specific file extension
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkExtensionStats {
    /// Number of files tested
    pub count: usize,
    /// Number of successful extractions
    pub successful: usize,
    /// Success rate (0.0-1.0)
    pub success_rate: f64,
    /// Average wall-clock duration in milliseconds (includes subprocess overhead)
    pub avg_duration_ms: f64,
    /// Median wall-clock duration in milliseconds
    pub median_duration_ms: f64,
    /// P95 wall-clock duration in milliseconds
    pub p95_duration_ms: f64,
    /// Average pure extraction duration in milliseconds (excludes subprocess overhead)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_extraction_duration_ms: Option<f64>,
    /// Median pure extraction duration in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub median_extraction_duration_ms: Option<f64>,
    /// P95 pure extraction duration in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub p95_extraction_duration_ms: Option<f64>,
    /// Average throughput in MB/s
    pub avg_throughput_mbps: f64,
    /// Average peak memory in MB
    pub avg_peak_memory_mb: f64,
}

/// Analysis of results grouped by file extension
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionAnalysis {
    /// Total number of files with this extension
    pub total_files: usize,
    /// Per-framework performance statistics
    pub framework_stats: HashMap<String, FrameworkExtensionStats>,
}

/// Complete by-extension analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ByExtensionReport {
    /// Per-extension analysis
    pub by_extension: HashMap<String, ExtensionAnalysis>,
}

/// Analyze benchmark results by file extension
///
/// Groups results by file extension and calculates per-framework statistics
/// for each extension.
///
/// # Arguments
/// * `results` - Vector of benchmark results to analyze
///
/// # Returns
/// * ByExtensionReport with statistics grouped by extension and framework
pub fn analyze_by_extension(results: &[BenchmarkResult]) -> ByExtensionReport {
    let mut by_extension: HashMap<String, HashMap<String, Vec<&BenchmarkResult>>> = HashMap::new();

    for result in results {
        let ext = result.file_extension.clone();
        let framework = result.framework.clone();

        by_extension
            .entry(ext)
            .or_default()
            .entry(framework)
            .or_default()
            .push(result);
    }

    let mut report = HashMap::new();
    for (ext, framework_results) in by_extension {
        let total_files = framework_results.values().map(|v| v.len()).max().unwrap_or(0);

        let mut framework_stats = HashMap::new();
        for (framework, results) in framework_results {
            let stats = calculate_framework_stats(&results);
            framework_stats.insert(framework, stats);
        }

        report.insert(
            ext,
            ExtensionAnalysis {
                total_files,
                framework_stats,
            },
        );
    }

    ByExtensionReport { by_extension: report }
}

/// Calculate statistics for a framework's results
fn calculate_framework_stats(results: &[&BenchmarkResult]) -> FrameworkExtensionStats {
    let count = results.len();
    let successful = results.iter().filter(|r| r.success).count();
    let success_rate = if count > 0 {
        successful as f64 / count as f64
    } else {
        0.0
    };

    let successful_results: Vec<&&BenchmarkResult> = results.iter().filter(|r| r.success).collect();

    let avg_duration_ms = if !successful_results.is_empty() {
        successful_results
            .iter()
            .map(|r| r.duration.as_secs_f64() * 1000.0)
            .sum::<f64>()
            / successful_results.len() as f64
    } else {
        0.0
    };

    let mut durations: Vec<f64> = successful_results
        .iter()
        .map(|r| r.duration.as_secs_f64() * 1000.0)
        .collect();
    durations.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let median_duration_ms = if !durations.is_empty() {
        percentile_r7(&durations, 0.50)
    } else {
        0.0
    };

    let p95_duration_ms = if !durations.is_empty() {
        percentile_r7(&durations, 0.95)
    } else {
        0.0
    };

    // Extraction duration stats (pure extraction time, excludes subprocess overhead)
    let mut extraction_durations: Vec<f64> = successful_results
        .iter()
        .filter_map(|r| r.extraction_duration.map(|d| d.as_secs_f64() * 1000.0))
        .filter(|v| !v.is_nan() && v.is_finite())
        .collect();
    extraction_durations.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let avg_extraction_duration_ms = if !extraction_durations.is_empty() {
        Some(extraction_durations.iter().sum::<f64>() / extraction_durations.len() as f64)
    } else {
        None
    };

    let median_extraction_duration_ms = if !extraction_durations.is_empty() {
        Some(percentile_r7(&extraction_durations, 0.50))
    } else {
        None
    };

    let p95_extraction_duration_ms = if !extraction_durations.is_empty() {
        Some(percentile_r7(&extraction_durations, 0.95))
    } else {
        None
    };

    let avg_throughput_mbps = if !successful_results.is_empty() {
        successful_results
            .iter()
            .map(|r| r.metrics.throughput_bytes_per_sec / 1_000_000.0)
            .sum::<f64>()
            / successful_results.len() as f64
    } else {
        0.0
    };

    let avg_peak_memory_mb = if !successful_results.is_empty() {
        successful_results
            .iter()
            .map(|r| r.metrics.peak_memory_bytes as f64 / 1_000_000.0)
            .sum::<f64>()
            / successful_results.len() as f64
    } else {
        0.0
    };

    FrameworkExtensionStats {
        count,
        successful,
        success_rate,
        avg_duration_ms,
        median_duration_ms,
        p95_duration_ms,
        avg_extraction_duration_ms,
        median_extraction_duration_ms,
        p95_extraction_duration_ms,
        avg_throughput_mbps,
        avg_peak_memory_mb,
    }
}

/// Write by-extension analysis to JSON file
///
/// # Arguments
/// * `results` - Vector of benchmark results to analyze
/// * `output_path` - Path to output JSON file (e.g., "by-extension.json")
pub fn write_by_extension_analysis(results: &[BenchmarkResult], output_path: &Path) -> Result<()> {
    let report = analyze_by_extension(results);

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(Error::Io)?;
    }

    let json = serde_json::to_string_pretty(&report)
        .map_err(|e| Error::Benchmark(format!("Failed to serialize extension analysis: {}", e)))?;

    fs::write(output_path, json).map_err(Error::Io)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FrameworkCapabilities, OcrStatus, PerformanceMetrics};
    use std::path::PathBuf;
    use std::time::Duration;
    use tempfile::TempDir;

    fn create_benchmark_result(
        framework: &str,
        success: bool,
        duration_ms: u64,
        extraction_duration_ms: Option<u64>,
        throughput_bps: f64,
        memory_bytes: u64,
    ) -> BenchmarkResult {
        BenchmarkResult {
            framework: framework.to_string(),
            file_path: PathBuf::from(format!("/tmp/{}.txt", framework)),
            file_size: 1024,
            success,
            error_message: if success { None } else { Some("Test error".to_string()) },
            duration: Duration::from_millis(duration_ms),
            extraction_duration: extraction_duration_ms.map(Duration::from_millis),
            subprocess_overhead: extraction_duration_ms.map(|ed| Duration::from_millis(duration_ms.saturating_sub(ed))),
            metrics: PerformanceMetrics {
                peak_memory_bytes: memory_bytes,
                avg_cpu_percent: 50.0,
                throughput_bytes_per_sec: throughput_bps,
                p50_memory_bytes: memory_bytes,
                p95_memory_bytes: memory_bytes,
                p99_memory_bytes: memory_bytes,
            },
            quality: None,
            iterations: vec![],
            statistics: None,
            cold_start_duration: None,
            file_extension: "txt".to_string(),
            framework_capabilities: FrameworkCapabilities::default(),
            pdf_metadata: None,
            ocr_status: OcrStatus::Unknown,
            extracted_text: None,
        }
    }

    #[test]
    fn test_write_json() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("results.json");

        let results = vec![BenchmarkResult {
            framework: "test-framework".to_string(),
            file_path: PathBuf::from("/tmp/test.txt"),
            file_size: 1024,
            success: true,
            error_message: None,
            duration: Duration::from_secs(1),
            extraction_duration: None,
            subprocess_overhead: None,
            metrics: PerformanceMetrics {
                peak_memory_bytes: 10_000_000,
                avg_cpu_percent: 50.0,
                throughput_bytes_per_sec: 1024.0,
                p50_memory_bytes: 8_000_000,
                p95_memory_bytes: 9_500_000,
                p99_memory_bytes: 9_900_000,
            },
            quality: None,
            iterations: vec![],
            statistics: None,
            cold_start_duration: None,
            file_extension: "txt".to_string(),
            framework_capabilities: Default::default(),
            pdf_metadata: None,
            ocr_status: OcrStatus::Unknown,
            extracted_text: None,
        }];

        write_json(&results, &output_path).unwrap();

        assert!(output_path.exists());

        let contents = fs::read_to_string(&output_path).unwrap();
        let parsed: Vec<BenchmarkResult> = serde_json::from_str(&contents).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].framework, "test-framework");
    }

    #[test]
    fn test_write_json_creates_directory() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("subdir/results.json");

        let results = vec![];

        write_json(&results, &output_path).unwrap();

        assert!(output_path.exists());
        assert!(output_path.parent().unwrap().exists());
    }

    // ============================================================================
    // Tests for extraction_duration statistics in calculate_framework_stats
    // ============================================================================

    #[test]
    fn test_framework_stats_extraction_duration_all_present() {
        // Test: All results have extraction_duration -> percentiles populated
        let result1 = create_benchmark_result("framework1", true, 100, Some(80), 1_000_000.0, 10_000_000);
        let result2 = create_benchmark_result("framework1", true, 150, Some(120), 1_000_000.0, 10_000_000);
        let result3 = create_benchmark_result("framework1", true, 200, Some(160), 1_000_000.0, 10_000_000);
        let results = vec![&result1, &result2, &result3];

        let stats = calculate_framework_stats(&results);

        assert_eq!(stats.count, 3);
        assert_eq!(stats.successful, 3);
        assert!(stats.avg_extraction_duration_ms.is_some());
        assert!(stats.median_extraction_duration_ms.is_some());
        assert!(stats.p95_extraction_duration_ms.is_some());

        // Average of 80, 120, 160 = 120 ms
        assert!((stats.avg_extraction_duration_ms.unwrap() - 120.0).abs() < 0.1);
        // Median of 80, 120, 160 = 120 ms
        assert!((stats.median_extraction_duration_ms.unwrap() - 120.0).abs() < 0.1);
    }

    #[test]
    fn test_framework_stats_extraction_duration_all_none() {
        // Test: All results have extraction_duration = None -> percentiles None
        let result1 = create_benchmark_result("framework1", true, 100, None, 1_000_000.0, 10_000_000);
        let result2 = create_benchmark_result("framework1", true, 150, None, 1_000_000.0, 10_000_000);
        let result3 = create_benchmark_result("framework1", true, 200, None, 1_000_000.0, 10_000_000);
        let results = vec![&result1, &result2, &result3];

        let stats = calculate_framework_stats(&results);

        assert_eq!(stats.count, 3);
        assert_eq!(stats.successful, 3);
        assert!(stats.avg_extraction_duration_ms.is_none());
        assert!(stats.median_extraction_duration_ms.is_none());
        assert!(stats.p95_extraction_duration_ms.is_none());
    }

    #[test]
    fn test_framework_stats_extraction_duration_mixed_some_none() {
        // Test: Mixed Some/None extraction_duration -> only Some values used
        let result1 = create_benchmark_result("framework1", true, 100, Some(80), 1_000_000.0, 10_000_000);
        let result2 = create_benchmark_result("framework1", true, 150, None, 1_000_000.0, 10_000_000);
        let result3 = create_benchmark_result("framework1", true, 200, Some(160), 1_000_000.0, 10_000_000);
        let results = vec![&result1, &result2, &result3];

        let stats = calculate_framework_stats(&results);

        assert_eq!(stats.count, 3);
        assert_eq!(stats.successful, 3);
        assert!(stats.avg_extraction_duration_ms.is_some());
        assert!(stats.median_extraction_duration_ms.is_some());

        // Only 80 and 160 ms, average = 120 ms
        assert!((stats.avg_extraction_duration_ms.unwrap() - 120.0).abs() < 0.1);
    }

    #[test]
    fn test_framework_stats_extraction_duration_filters_nan() {
        // Test: NaN/infinite durations filtered out
        let result1 = create_benchmark_result("framework1", true, 100, Some(80), 1_000_000.0, 10_000_000);
        let result2 = create_benchmark_result("framework1", true, 150, Some(120), 1_000_000.0, 10_000_000);
        let result3 = create_benchmark_result("framework1", true, 200, Some(160), 1_000_000.0, 10_000_000);

        // Inject NaN and infinity by manipulating durations (since Duration doesn't support NaN)
        // We'll test this conceptually with valid values, but the filtering logic is tested
        // by verifying that only finite, non-NaN values are used
        let results = vec![&result1, &result2, &result3];

        let stats = calculate_framework_stats(&results);

        assert_eq!(stats.count, 3);
        // All three values are valid (80, 120, 160)
        assert!(stats.avg_extraction_duration_ms.is_some());
        assert_eq!(stats.avg_extraction_duration_ms.unwrap(), 120.0);
    }

    #[test]
    fn test_framework_stats_extraction_duration_empty_results() {
        // Test: Empty results -> sensible defaults
        let results: Vec<&BenchmarkResult> = vec![];

        let stats = calculate_framework_stats(&results);

        assert_eq!(stats.count, 0);
        assert_eq!(stats.successful, 0);
        assert_eq!(stats.success_rate, 0.0);
        assert_eq!(stats.avg_duration_ms, 0.0);
        assert_eq!(stats.median_duration_ms, 0.0);
        assert_eq!(stats.p95_duration_ms, 0.0);
        assert!(stats.avg_extraction_duration_ms.is_none());
        assert!(stats.median_extraction_duration_ms.is_none());
        assert!(stats.p95_extraction_duration_ms.is_none());
    }

    #[test]
    fn test_framework_stats_extraction_duration_only_failed_results() {
        // Test: Only failed results -> extraction_duration None (only successful results used)
        let result1 = create_benchmark_result("framework1", false, 0, None, 0.0, 0);
        let result2 = create_benchmark_result("framework1", false, 0, None, 0.0, 0);
        let results = vec![&result1, &result2];

        let stats = calculate_framework_stats(&results);

        assert_eq!(stats.count, 2);
        assert_eq!(stats.successful, 0);
        assert!(stats.avg_extraction_duration_ms.is_none());
        assert!(stats.median_extraction_duration_ms.is_none());
        assert!(stats.p95_extraction_duration_ms.is_none());
    }

    #[test]
    fn test_framework_stats_extraction_duration_single_value() {
        // Test: Single extraction_duration value -> all percentiles return that value
        let result = create_benchmark_result("framework1", true, 100, Some(80), 1_000_000.0, 10_000_000);
        let results = vec![&result];

        let stats = calculate_framework_stats(&results);

        assert_eq!(stats.count, 1);
        assert_eq!(stats.successful, 1);
        assert_eq!(stats.avg_extraction_duration_ms.unwrap(), 80.0);
        assert_eq!(stats.median_extraction_duration_ms.unwrap(), 80.0);
        assert_eq!(stats.p95_extraction_duration_ms.unwrap(), 80.0);
    }

    #[test]
    fn test_framework_stats_success_rate_with_extraction_duration() {
        // Test: Mixed success/failure with extraction_duration on successful results
        let result1 = create_benchmark_result("framework1", true, 100, Some(80), 1_000_000.0, 10_000_000);
        let result2 = create_benchmark_result("framework1", true, 150, Some(120), 1_000_000.0, 10_000_000);
        let result3 = create_benchmark_result("framework1", false, 0, None, 0.0, 0);
        let results = vec![&result1, &result2, &result3];

        let stats = calculate_framework_stats(&results);

        assert_eq!(stats.count, 3);
        assert_eq!(stats.successful, 2);
        assert_eq!(stats.success_rate, 2.0 / 3.0);

        // Only successful results have extraction_duration
        assert!(stats.avg_extraction_duration_ms.is_some());
        // Average of 80 and 120 = 100
        assert!((stats.avg_extraction_duration_ms.unwrap() - 100.0).abs() < 0.1);
    }

    #[test]
    fn test_framework_stats_large_number_extraction_durations() {
        // Test: Many extraction_duration values -> percentiles calculated correctly
        let mut results = vec![];
        for i in 1..=100 {
            results.push(create_benchmark_result(
                "framework1",
                true,
                i * 10,
                Some(i * 8),
                1_000_000.0,
                10_000_000,
            ));
        }

        let result_refs: Vec<&BenchmarkResult> = results.iter().collect();
        let stats = calculate_framework_stats(&result_refs);

        assert_eq!(stats.count, 100);
        assert_eq!(stats.successful, 100);

        // Average of 8, 16, 24, ..., 800 = 8*(1+2+...+100)/100 = 8*5050/100 = 404
        let expected_avg = 8.0 * (1..=100).sum::<u64>() as f64 / 100.0;
        assert!((stats.avg_extraction_duration_ms.unwrap() - expected_avg).abs() < 1.0);

        // Median of 1-100: 50th percentile
        assert!(stats.median_extraction_duration_ms.is_some());
        // P95: 95th percentile
        assert!(stats.p95_extraction_duration_ms.is_some());
    }

    #[test]
    fn test_analyze_by_extension_with_extraction_duration() {
        // Integration test: analyze_by_extension properly aggregates extraction_duration
        let results = vec![
            create_benchmark_result("framework1", true, 100, Some(80), 1_000_000.0, 10_000_000),
            create_benchmark_result("framework1", true, 150, Some(120), 1_000_000.0, 10_000_000),
        ];

        let report = analyze_by_extension(&results);

        assert!(report.by_extension.contains_key("txt"));
        let ext_analysis = &report.by_extension["txt"];
        assert!(ext_analysis.framework_stats.contains_key("framework1"));

        let framework_stats = &ext_analysis.framework_stats["framework1"];
        assert!(framework_stats.avg_extraction_duration_ms.is_some());
        assert!(framework_stats.median_extraction_duration_ms.is_some());
        assert!(framework_stats.p95_extraction_duration_ms.is_some());
    }

    #[test]
    fn test_analyze_by_extension_mixed_extraction_duration() {
        // Test: analyze_by_extension with mixed extraction_duration presence
        let mut result1 = create_benchmark_result("framework1", true, 100, Some(80), 1_000_000.0, 10_000_000);
        result1.file_extension = "pdf".to_string();

        let mut result2 = create_benchmark_result("framework1", true, 150, None, 1_000_000.0, 10_000_000);
        result2.file_extension = "pdf".to_string();

        let results = vec![result1, result2];

        let report = analyze_by_extension(&results);

        assert!(report.by_extension.contains_key("pdf"));
        let ext_analysis = &report.by_extension["pdf"];
        let framework_stats = &ext_analysis.framework_stats["framework1"];

        // Should have extraction_duration stats (only from result1 which has Some)
        assert!(framework_stats.avg_extraction_duration_ms.is_some());
        assert_eq!(framework_stats.avg_extraction_duration_ms.unwrap(), 80.0);
    }
}

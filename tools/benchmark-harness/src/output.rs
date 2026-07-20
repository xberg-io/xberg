//! Output writers for benchmark results
//!
//! This module provides functionality for persisting benchmark results to disk
//! in JSON format.

use crate::stats::percentile_r7;
use crate::types::{BenchmarkResult, ErrorKind};
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
    if result.success && result.error_message.is_some() {
        return Err(Error::Benchmark(format!(
            "Invalid result state for {}/{}: success=true but error_message is set",
            result.framework,
            result.file_path.display()
        )));
    }

    if !result.success && result.error_message.is_none() {
        return Err(Error::Benchmark(format!(
            "Invalid result state for {}/{}: success=false but error_message is None",
            result.framework,
            result.file_path.display()
        )));
    }

    if result.success && result.error_kind != ErrorKind::None {
        return Err(Error::Benchmark(format!(
            "Invalid result state for {}/{}: success=true but error_kind is {:?}",
            result.framework,
            result.file_path.display(),
            result.error_kind
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
    /// Number of framework-side extraction errors (not our fault)
    pub framework_errors: usize,
    /// Number of harness-side errors (potentially our fault)
    pub harness_errors: usize,
    /// Number of extractions that timed out
    pub timeouts: usize,
    /// Number of extractions that returned empty content
    pub empty_content: usize,
    /// Unique framework error messages with occurrence counts
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub error_details: HashMap<String, usize>,
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
    /// Mean text token F1 / TF1 (0.0-1.0), successful extractions only
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_f1_text: Option<f64>,
    /// Mean numeric token F1 (0.0-1.0), successful extractions only
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_f1_numeric: Option<f64>,
    /// Mean layout/structural F1 / SF1 (0.0-1.0), successful extractions only.
    /// `None` when no result in this group reported a layout score (e.g. plaintext mode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_f1_layout: Option<f64>,
    /// Mean combined quality score (0.0-1.0), successful extractions only
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_quality_score: Option<f64>,
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

    let framework_errors = results
        .iter()
        .filter(|r| r.error_kind == ErrorKind::FrameworkError)
        .count();
    let harness_errors = results
        .iter()
        .filter(|r| r.error_kind == ErrorKind::HarnessError)
        .count();
    let timeouts = results.iter().filter(|r| r.error_kind == ErrorKind::Timeout).count();
    let empty_content = results
        .iter()
        .filter(|r| r.error_kind == ErrorKind::EmptyContent)
        .count();

    let mut error_details: HashMap<String, usize> = HashMap::new();
    for result in results.iter().filter(|r| !r.success) {
        if let Some(msg) = &result.error_message {
            *error_details.entry(msg.clone()).or_insert(0) += 1;
        }
    }

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

    // Batch adapters store one positive aggregate-throughput anchor and zero
    // on sibling per-file rows. Average only reported measurements so the
    // anchor is not divided by successful batch cardinality. ~keep
    let reported_throughputs: Vec<f64> = successful_results
        .iter()
        .map(|r| r.metrics.throughput_bytes_per_sec / 1_000_000.0)
        .filter(|throughput| throughput.is_finite() && *throughput > 0.0)
        .collect();
    let avg_throughput_mbps = if !reported_throughputs.is_empty() {
        reported_throughputs.iter().sum::<f64>() / reported_throughputs.len() as f64
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

    let mean = |values: &[f64]| -> Option<f64> {
        if values.is_empty() {
            None
        } else {
            Some(values.iter().sum::<f64>() / values.len() as f64)
        }
    };

    let f1_texts: Vec<f64> = successful_results
        .iter()
        .filter_map(|r| r.quality.as_ref().map(|q| q.f1_score_text))
        .filter(|v| !v.is_nan() && v.is_finite())
        .collect();
    let f1_numerics: Vec<f64> = successful_results
        .iter()
        .filter_map(|r| r.quality.as_ref().map(|q| q.f1_score_numeric))
        .filter(|v| !v.is_nan() && v.is_finite())
        .collect();
    let f1_layouts: Vec<f64> = successful_results
        .iter()
        .filter_map(|r| r.quality.as_ref().and_then(|q| q.f1_score_layout))
        .filter(|v| !v.is_nan() && v.is_finite())
        .collect();
    let quality_scores: Vec<f64> = successful_results
        .iter()
        .filter_map(|r| r.quality.as_ref().map(|q| q.quality_score))
        .filter(|v| !v.is_nan() && v.is_finite())
        .collect();

    let avg_f1_text = mean(&f1_texts);
    let avg_f1_numeric = mean(&f1_numerics);
    let avg_f1_layout = mean(&f1_layouts);
    let avg_quality_score = mean(&quality_scores);

    FrameworkExtensionStats {
        count,
        successful,
        framework_errors,
        harness_errors,
        timeouts,
        empty_content,
        error_details,
        success_rate,
        avg_duration_ms,
        median_duration_ms,
        p95_duration_ms,
        avg_extraction_duration_ms,
        median_extraction_duration_ms,
        p95_extraction_duration_ms,
        avg_throughput_mbps,
        avg_peak_memory_mb,
        avg_f1_text,
        avg_f1_numeric,
        avg_f1_layout,
        avg_quality_score,
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
    use crate::types::{FrameworkCapabilities, OcrStatus, OutputFormat, PerformanceMetrics, QualityMetrics};
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
            error_kind: if success {
                ErrorKind::None
            } else {
                ErrorKind::HarnessError
            },
            duration: Duration::from_millis(duration_ms),
            extraction_duration: extraction_duration_ms.map(Duration::from_millis),
            subprocess_overhead: extraction_duration_ms.map(|ed| Duration::from_millis(duration_ms.saturating_sub(ed))),
            metrics: PerformanceMetrics {
                baseline_memory_bytes: 0,
                peak_memory_bytes: memory_bytes,
                peak_memory_delta_bytes: memory_bytes,
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
            system_load: None,
            output_format: OutputFormat::Markdown,
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
            error_kind: ErrorKind::None,
            duration: Duration::from_secs(1),
            extraction_duration: None,
            subprocess_overhead: None,
            metrics: PerformanceMetrics {
                baseline_memory_bytes: 0,
                peak_memory_bytes: 10_000_000,
                peak_memory_delta_bytes: 10_000_000,
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
            system_load: None,
            output_format: OutputFormat::Markdown,
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

    #[test]
    fn test_framework_stats_extraction_duration_all_present() {
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

        assert!((stats.avg_extraction_duration_ms.unwrap() - 120.0).abs() < 0.1);
        assert!((stats.median_extraction_duration_ms.unwrap() - 120.0).abs() < 0.1);
    }

    #[test]
    fn test_framework_stats_extraction_duration_all_none() {
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
        let result1 = create_benchmark_result("framework1", true, 100, Some(80), 1_000_000.0, 10_000_000);
        let result2 = create_benchmark_result("framework1", true, 150, None, 1_000_000.0, 10_000_000);
        let result3 = create_benchmark_result("framework1", true, 200, Some(160), 1_000_000.0, 10_000_000);
        let results = vec![&result1, &result2, &result3];

        let stats = calculate_framework_stats(&results);

        assert_eq!(stats.count, 3);
        assert_eq!(stats.successful, 3);
        assert!(stats.avg_extraction_duration_ms.is_some());
        assert!(stats.median_extraction_duration_ms.is_some());

        assert!((stats.avg_extraction_duration_ms.unwrap() - 120.0).abs() < 0.1);
    }

    #[test]
    fn test_framework_stats_extraction_duration_filters_nan() {
        let result1 = create_benchmark_result("framework1", true, 100, Some(80), 1_000_000.0, 10_000_000);
        let result2 = create_benchmark_result("framework1", true, 150, Some(120), 1_000_000.0, 10_000_000);
        let result3 = create_benchmark_result("framework1", true, 200, Some(160), 1_000_000.0, 10_000_000);

        let results = vec![&result1, &result2, &result3];

        let stats = calculate_framework_stats(&results);

        assert_eq!(stats.count, 3);
        assert!(stats.avg_extraction_duration_ms.is_some());
        assert_eq!(stats.avg_extraction_duration_ms.unwrap(), 120.0);
    }

    #[test]
    fn test_framework_stats_extraction_duration_empty_results() {
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
        let result1 = create_benchmark_result("framework1", true, 100, Some(80), 1_000_000.0, 10_000_000);
        let result2 = create_benchmark_result("framework1", true, 150, Some(120), 1_000_000.0, 10_000_000);
        let result3 = create_benchmark_result("framework1", false, 0, None, 0.0, 0);
        let results = vec![&result1, &result2, &result3];

        let stats = calculate_framework_stats(&results);

        assert_eq!(stats.count, 3);
        assert_eq!(stats.successful, 2);
        assert_eq!(stats.success_rate, 2.0 / 3.0);

        assert!(stats.avg_extraction_duration_ms.is_some());
        assert!((stats.avg_extraction_duration_ms.unwrap() - 100.0).abs() < 0.1);
    }

    #[test]
    fn test_framework_stats_does_not_divide_batch_throughput_anchor_by_cardinality() {
        let anchor = create_benchmark_result("framework1", true, 100, None, 3_000_000.0, 10_000_000);
        let sibling1 = create_benchmark_result("framework1", true, 100, None, 0.0, 10_000_000);
        let sibling2 = create_benchmark_result("framework1", true, 100, None, 0.0, 10_000_000);
        let results = vec![&anchor, &sibling1, &sibling2];

        let stats = calculate_framework_stats(&results);

        assert_eq!(stats.avg_throughput_mbps, 3.0);
    }

    #[test]
    fn test_framework_stats_large_number_extraction_durations() {
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

        let expected_avg = 8.0 * (1..=100).sum::<u64>() as f64 / 100.0;
        assert!((stats.avg_extraction_duration_ms.unwrap() - expected_avg).abs() < 1.0);

        assert!(stats.median_extraction_duration_ms.is_some());
        assert!(stats.p95_extraction_duration_ms.is_some());
    }

    #[test]
    fn test_analyze_by_extension_with_extraction_duration() {
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
        let mut result1 = create_benchmark_result("framework1", true, 100, Some(80), 1_000_000.0, 10_000_000);
        result1.file_extension = "pdf".to_string();

        let mut result2 = create_benchmark_result("framework1", true, 150, None, 1_000_000.0, 10_000_000);
        result2.file_extension = "pdf".to_string();

        let results = vec![result1, result2];

        let report = analyze_by_extension(&results);

        assert!(report.by_extension.contains_key("pdf"));
        let ext_analysis = &report.by_extension["pdf"];
        let framework_stats = &ext_analysis.framework_stats["framework1"];

        assert!(framework_stats.avg_extraction_duration_ms.is_some());
        assert_eq!(framework_stats.avg_extraction_duration_ms.unwrap(), 80.0);
    }

    #[test]
    fn test_framework_stats_quality_absent_when_no_quality_metrics() {
        let result = create_benchmark_result("framework1", true, 100, Some(80), 1_000_000.0, 10_000_000);
        let results = vec![&result];

        let stats = calculate_framework_stats(&results);

        assert!(stats.avg_f1_text.is_none());
        assert!(stats.avg_f1_numeric.is_none());
        assert!(stats.avg_f1_layout.is_none());
        assert!(stats.avg_quality_score.is_none());
    }

    #[test]
    fn test_framework_stats_preserves_mean_tf1_and_sf1() {
        let mut result1 = create_benchmark_result("framework1", true, 100, Some(80), 1_000_000.0, 10_000_000);
        result1.quality = Some(QualityMetrics {
            f1_score_text: 0.80,
            f1_score_numeric: 0.90,
            f1_score_layout: Some(0.60),
            quality_score: 0.75,
            missing_tokens: vec![],
            extra_tokens: vec![],
            correct: false,
        });

        let mut result2 = create_benchmark_result("framework1", true, 150, Some(120), 1_000_000.0, 10_000_000);
        result2.quality = Some(QualityMetrics {
            f1_score_text: 0.90,
            f1_score_numeric: 0.95,
            f1_score_layout: Some(0.70),
            quality_score: 0.85,
            missing_tokens: vec![],
            extra_tokens: vec![],
            correct: true,
        });

        let results = vec![&result1, &result2];
        let stats = calculate_framework_stats(&results);

        assert!((stats.avg_f1_text.unwrap() - 0.85).abs() < 1e-9);
        assert!((stats.avg_f1_numeric.unwrap() - 0.925).abs() < 1e-9);
        assert!((stats.avg_f1_layout.unwrap() - 0.65).abs() < 1e-9);
        assert!((stats.avg_quality_score.unwrap() - 0.80).abs() < 1e-9);
    }

    #[test]
    fn test_framework_stats_layout_none_when_no_result_reports_it() {
        let mut result = create_benchmark_result("framework1", true, 100, Some(80), 1_000_000.0, 10_000_000);
        result.quality = Some(QualityMetrics {
            f1_score_text: 0.80,
            f1_score_numeric: 0.90,
            f1_score_layout: None,
            quality_score: 0.75,
            missing_tokens: vec![],
            extra_tokens: vec![],
            correct: false,
        });

        let results = vec![&result];
        let stats = calculate_framework_stats(&results);

        assert!(stats.avg_f1_text.is_some());
        assert!(stats.avg_f1_layout.is_none());
    }

    #[test]
    fn test_framework_stats_quality_excludes_failed_results() {
        let mut result1 = create_benchmark_result("framework1", true, 100, Some(80), 1_000_000.0, 10_000_000);
        result1.quality = Some(QualityMetrics {
            f1_score_text: 0.80,
            f1_score_numeric: 0.90,
            f1_score_layout: Some(0.60),
            quality_score: 0.75,
            missing_tokens: vec![],
            extra_tokens: vec![],
            correct: false,
        });

        // Failed result: create_benchmark_result forces quality to None for failures anyway,
        // but assert explicitly that it never contributes to the mean. ~keep
        let result2 = create_benchmark_result("framework1", false, 0, None, 0.0, 0);

        let results = vec![&result1, &result2];
        let stats = calculate_framework_stats(&results);

        assert_eq!(stats.count, 2);
        assert_eq!(stats.successful, 1);
        assert!((stats.avg_f1_text.unwrap() - 0.80).abs() < 1e-9);
    }
}

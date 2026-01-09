//! New aggregation module for benchmark results
//!
//! This module provides aggregation functions that group results by:
//! - Framework and mode (single/batch)
//! - File type
//! - OCR usage (yes/no)
//!
//! Calculates percentile-based statistics for better understanding of performance distributions.

use crate::types::{BenchmarkResult, DiskSizeInfo};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Consolidated results using new aggregation format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewConsolidatedResults {
    /// Aggregated results grouped by framework:mode combination
    pub by_framework_mode: HashMap<String, FrameworkModeAggregation>,
    /// Disk sizes for each framework
    pub disk_sizes: HashMap<String, DiskSizeInfo>,
    /// Metadata about the consolidation
    pub metadata: ConsolidationMetadata,
}

/// Metadata about the consolidation process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationMetadata {
    /// Number of benchmark results included
    pub total_results: usize,
    /// Number of unique frameworks
    pub framework_count: usize,
    /// Number of unique file types
    pub file_type_count: usize,
    /// Timestamp of consolidation
    pub timestamp: String,
}

/// Aggregated results for a specific framework and mode combination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkModeAggregation {
    /// Framework name (base name without mode suffix)
    pub framework: String,
    /// Mode: "single", "batch", "sync", "async"
    pub mode: String,
    /// Cold start duration statistics (if available)
    pub cold_start: Option<DurationPercentiles>,
    /// Results grouped by file type
    pub by_file_type: HashMap<String, FileTypeAggregation>,
}

/// Aggregated results for a specific file type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTypeAggregation {
    /// File type (extension)
    pub file_type: String,
    /// Results without OCR
    pub no_ocr: Option<PerformancePercentiles>,
    /// Results with OCR
    pub with_ocr: Option<PerformancePercentiles>,
}

/// Performance percentiles for a group of results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformancePercentiles {
    /// Number of samples in this group
    pub sample_count: usize,
    /// Throughput percentiles (p50, p95, p99) in MB/s
    pub throughput: Percentiles,
    /// Memory percentiles (p50, p95, p99) in MB
    pub memory: Percentiles,
    /// Duration percentiles (p50, p95, p99) in ms
    pub duration: Percentiles,
    /// Success rate as percentage (0-100)
    pub success_rate_percent: f64,
}

/// Percentile values for a metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Percentiles {
    /// 50th percentile (median)
    pub p50: f64,
    /// 95th percentile
    pub p95: f64,
    /// 99th percentile
    pub p99: f64,
}

/// Duration percentiles in milliseconds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DurationPercentiles {
    /// Number of samples with cold start data
    pub sample_count: usize,
    /// 50th percentile (median) in ms
    pub p50_ms: f64,
    /// 95th percentile in ms
    pub p95_ms: f64,
    /// 99th percentile in ms
    pub p99_ms: f64,
}

/// Main aggregation function for new format
///
/// Groups results by:
/// 1. Framework and mode (extracted from framework name)
/// 2. File type (extension)
/// 3. OCR usage (yes/no)
///
/// Calculates p50/p95/p99 percentiles for each group.
pub fn aggregate_new_format(results: &[BenchmarkResult]) -> NewConsolidatedResults {
    // Validate input - HIGH PRIORITY FIX
    if results.is_empty() {
        return NewConsolidatedResults {
            by_framework_mode: HashMap::new(),
            disk_sizes: HashMap::new(),
            metadata: ConsolidationMetadata {
                total_results: 0,
                framework_count: 0,
                file_type_count: 0,
                timestamp: chrono::Utc::now().to_rfc3339(),
            },
        };
    }

    let mut by_framework_mode: HashMap<String, HashMap<String, Vec<&BenchmarkResult>>> = HashMap::new();
    let mut disk_sizes: HashMap<String, DiskSizeInfo> = HashMap::new();
    let mut file_types = std::collections::HashSet::new();

    // Group results by framework:mode and file type
    for result in results {
        let (framework, mode) = extract_framework_and_mode(&result.framework);
        let key = format!("{}:{}", framework, mode);

        by_framework_mode
            .entry(key)
            .or_default()
            .entry(result.file_extension.clone())
            .or_default()
            .push(result);

        file_types.insert(result.file_extension.clone());

        // Collect disk sizes
        if let Some(disk_size) = &result.framework_capabilities.installation_size {
            disk_sizes.insert(framework.to_string(), disk_size.clone());
        }
    }

    // Aggregate each framework:mode combination
    let mut aggregated_by_framework_mode = HashMap::new();

    for (framework_mode_key, file_type_results) in by_framework_mode {
        let parts: Vec<&str> = framework_mode_key.split(':').collect();
        let framework = parts[0].to_string();
        let mode = parts[1].to_string();

        // Collect all results for this framework:mode for cold start calculation
        let all_results: Vec<&BenchmarkResult> = file_type_results.values().flat_map(|v| v.iter().copied()).collect();
        let cold_start = aggregate_cold_starts(&all_results);

        // Aggregate by file type
        let mut by_file_type = HashMap::new();
        for (file_type, results_for_type) in file_type_results {
            let aggregation = aggregate_by_ocr_status(&results_for_type);
            by_file_type.insert(
                file_type.clone(),
                FileTypeAggregation {
                    file_type: file_type.clone(),
                    no_ocr: aggregation.0,
                    with_ocr: aggregation.1,
                },
            );
        }

        aggregated_by_framework_mode.insert(
            framework_mode_key.clone(),
            FrameworkModeAggregation {
                framework,
                mode,
                cold_start,
                by_file_type,
            },
        );
    }

    let metadata = ConsolidationMetadata {
        total_results: results.len(),
        framework_count: aggregated_by_framework_mode.len(),
        file_type_count: file_types.len(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    NewConsolidatedResults {
        by_framework_mode: aggregated_by_framework_mode,
        disk_sizes,
        metadata,
    }
}

/// Aggregate results by OCR status
///
/// Returns (no_ocr, with_ocr) tuple of PerformancePercentiles
fn aggregate_by_ocr_status(
    results: &[&BenchmarkResult],
) -> (Option<PerformancePercentiles>, Option<PerformancePercentiles>) {
    use crate::types::OcrStatus;

    // Include Unknown in no_ocr category (CRITICAL FIX)
    let no_ocr: Vec<&BenchmarkResult> = results
        .iter()
        .filter(|r| r.ocr_status != OcrStatus::Used)
        .copied()
        .collect();

    let with_ocr: Vec<&BenchmarkResult> = results
        .iter()
        .filter(|r| r.ocr_status == OcrStatus::Used)
        .copied()
        .collect();

    let no_ocr_stats = if !no_ocr.is_empty() {
        Some(calculate_percentiles(&no_ocr))
    } else {
        None
    };

    let with_ocr_stats = if !with_ocr.is_empty() {
        Some(calculate_percentiles(&with_ocr))
    } else {
        None
    };

    (no_ocr_stats, with_ocr_stats)
}

/// Calculate percentiles for a group of results
///
/// Only uses successful results for metric calculations.
/// Success rate is calculated from all results.
fn calculate_percentiles(results: &[&BenchmarkResult]) -> PerformancePercentiles {
    let successful: Vec<&BenchmarkResult> = results.iter().filter(|r| r.success).copied().collect();

    // Extract values for percentile calculation with NaN filtering - HIGH PRIORITY FIX
    let mut durations: Vec<f64> = successful
        .iter()
        .map(|r| r.duration.as_secs_f64() * 1000.0)
        .filter(|&v| !v.is_nan() && v.is_finite())
        .collect();

    let mut throughputs: Vec<f64> = successful
        .iter()
        .map(|r| r.metrics.throughput_bytes_per_sec / 1_000_000.0) // Convert to MB/s
        .filter(|&v| !v.is_nan() && v.is_finite())
        .collect();

    let mut memories: Vec<f64> = successful
        .iter()
        .map(|r| r.metrics.peak_memory_bytes as f64 / 1_000_000.0) // Convert to MB
        .filter(|&v| !v.is_nan() && v.is_finite())
        .collect();

    // Sort for percentile calculation (NaN-safe)
    durations.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    throughputs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    memories.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let duration = Percentiles {
        p50: calculate_percentile_value(&durations, 0.50),
        p95: calculate_percentile_value(&durations, 0.95),
        p99: calculate_percentile_value(&durations, 0.99),
    };

    let throughput = Percentiles {
        p50: calculate_percentile_value(&throughputs, 0.50),
        p95: calculate_percentile_value(&throughputs, 0.95),
        p99: calculate_percentile_value(&throughputs, 0.99),
    };

    let memory = Percentiles {
        p50: calculate_percentile_value(&memories, 0.50),
        p95: calculate_percentile_value(&memories, 0.95),
        p99: calculate_percentile_value(&memories, 0.99),
    };

    let success_rate_percent = if !results.is_empty() {
        (successful.len() as f64 / results.len() as f64) * 100.0
    } else {
        0.0
    };

    PerformancePercentiles {
        sample_count: successful.len(), // CRITICAL FIX: Count only successful results used for percentiles
        throughput,
        memory,
        duration,
        success_rate_percent,
    }
}

/// Aggregate cold start durations
///
/// Returns percentiles of cold start durations if any results have cold start data.
fn aggregate_cold_starts(results: &[&BenchmarkResult]) -> Option<DurationPercentiles> {
    let cold_starts: Vec<f64> = results
        .iter()
        .filter_map(|r| r.cold_start_duration.map(|d| d.as_secs_f64() * 1000.0))
        .filter(|&v| !v.is_nan() && v.is_finite()) // HIGH PRIORITY FIX: NaN filtering
        .collect();

    if cold_starts.is_empty() {
        return None;
    }

    let mut sorted = cold_starts.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    Some(DurationPercentiles {
        sample_count: cold_starts.len(),
        p50_ms: calculate_percentile_value(&sorted, 0.50),
        p95_ms: calculate_percentile_value(&sorted, 0.95),
        p99_ms: calculate_percentile_value(&sorted, 0.99),
    })
}

/// Extract framework name and mode from framework string
///
/// Returns (framework_name, mode) where mode is one of:
/// - "batch" if ends with "-batch" (after normalizing -sync/-async)
/// - "single" otherwise (default)
///
/// The -sync/-async suffixes are stripped as they represent implementation details
/// (blocking vs non-blocking execution), not distinct benchmark modes.
/// We benchmark only the best/default implementation per language.
fn extract_framework_and_mode(framework_name: &str) -> (&str, &str) {
    // First, normalize by stripping -sync/-async suffixes (implementation detail, not a mode)
    let normalized = framework_name
        .strip_suffix("-sync")
        .or_else(|| framework_name.strip_suffix("-async"))
        .unwrap_or(framework_name);

    // Now check for batch mode
    if let Some(base) = normalized.strip_suffix("-batch") {
        (base, "batch")
    } else {
        (normalized, "single")
    }
}

/// Calculate a specific percentile from sorted values using linear interpolation
///
/// Uses R-7 method (NumPy default) for accurate percentile calculation.
/// Returns 0.0 for empty arrays.
fn calculate_percentile_value(sorted_values: &[f64], percentile: f64) -> f64 {
    if sorted_values.is_empty() {
        return 0.0;
    }

    let n = sorted_values.len();
    if n == 1 {
        return sorted_values[0];
    }

    // Linear interpolation (R-7 method, used by NumPy) - CRITICAL FIX
    let index = percentile * (n as f64 - 1.0);
    let lower = index.floor() as usize;
    let upper = index.ceil() as usize;

    if lower == upper {
        sorted_values[lower]
    } else {
        let weight = index - lower as f64;
        sorted_values[lower] * (1.0 - weight) + sorted_values[upper] * weight
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FrameworkCapabilities, OcrStatus, PerformanceMetrics};
    use std::path::PathBuf;
    use std::time::Duration;

    fn create_test_result(
        framework: &str,
        file_ext: &str,
        ocr_status: OcrStatus,
        duration_ms: u64,
        throughput_bps: f64,
        memory_bytes: u64,
    ) -> BenchmarkResult {
        BenchmarkResult {
            framework: framework.to_string(),
            file_path: PathBuf::from(format!("test.{}", file_ext)),
            file_size: 1024,
            success: true,
            error_message: None,
            duration: Duration::from_millis(duration_ms),
            extraction_duration: None,
            subprocess_overhead: None,
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
            cold_start_duration: Some(Duration::from_millis(500)),
            file_extension: file_ext.to_string(),
            framework_capabilities: FrameworkCapabilities::default(),
            pdf_metadata: None,
            ocr_status,
        }
    }

    #[test]
    fn test_extract_framework_and_mode() {
        // Sync/async suffixes are normalized to "single" mode
        assert_eq!(extract_framework_and_mode("kreuzberg-sync"), ("kreuzberg", "single"));
        assert_eq!(extract_framework_and_mode("kreuzberg-async"), ("kreuzberg", "single"));
        assert_eq!(extract_framework_and_mode("python-sync"), ("python", "single"));
        assert_eq!(extract_framework_and_mode("python-async"), ("python", "single"));

        // Batch mode is preserved
        assert_eq!(extract_framework_and_mode("kreuzberg-batch"), ("kreuzberg", "batch"));
        assert_eq!(extract_framework_and_mode("python-batch"), ("python", "batch"));

        // No suffix defaults to single mode
        assert_eq!(extract_framework_and_mode("kreuzberg"), ("kreuzberg", "single"));
        assert_eq!(extract_framework_and_mode("docling"), ("docling", "single"));
    }

    #[test]
    fn test_calculate_percentile_value() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(calculate_percentile_value(&values, 0.0), 1.0);
        assert_eq!(calculate_percentile_value(&values, 0.5), 3.0);
        assert_eq!(calculate_percentile_value(&values, 1.0), 5.0);
        assert_eq!(calculate_percentile_value(&[], 0.5), 0.0);
    }

    #[test]
    fn test_aggregate_new_format() {
        let results = vec![
            create_test_result(
                "kreuzberg-sync",
                "pdf",
                OcrStatus::NotUsed,
                100,
                1_000_000.0,
                10_000_000,
            ),
            create_test_result("kreuzberg-sync", "pdf", OcrStatus::Used, 200, 500_000.0, 20_000_000),
            create_test_result(
                "kreuzberg-batch",
                "docx",
                OcrStatus::NotUsed,
                150,
                750_000.0,
                15_000_000,
            ),
        ];

        let aggregated = aggregate_new_format(&results);

        assert_eq!(aggregated.by_framework_mode.len(), 2);
        // "kreuzberg-sync" is normalized to "kreuzberg:single"
        assert!(aggregated.by_framework_mode.contains_key("kreuzberg:single"));
        assert!(aggregated.by_framework_mode.contains_key("kreuzberg:batch"));

        let single_agg = &aggregated.by_framework_mode["kreuzberg:single"];
        assert_eq!(single_agg.framework, "kreuzberg");
        assert_eq!(single_agg.mode, "single");
        assert!(single_agg.cold_start.is_some());

        let pdf_agg = &single_agg.by_file_type["pdf"];
        assert!(pdf_agg.no_ocr.is_some());
        assert!(pdf_agg.with_ocr.is_some());

        assert_eq!(pdf_agg.no_ocr.as_ref().unwrap().sample_count, 1);
        assert_eq!(pdf_agg.with_ocr.as_ref().unwrap().sample_count, 1);
    }

    #[test]
    fn test_calculate_percentiles() {
        let results = [
            create_test_result("kreuzberg", "pdf", OcrStatus::NotUsed, 100, 1_000_000.0, 10_000_000),
            create_test_result("kreuzberg", "pdf", OcrStatus::NotUsed, 200, 2_000_000.0, 20_000_000),
            create_test_result("kreuzberg", "pdf", OcrStatus::NotUsed, 300, 3_000_000.0, 30_000_000),
        ];

        let refs: Vec<&BenchmarkResult> = results.iter().collect();
        let percentiles = calculate_percentiles(&refs);

        assert_eq!(percentiles.sample_count, 3);
        assert_eq!(percentiles.success_rate_percent, 100.0);
        assert!(percentiles.duration.p50 > 0.0);
        assert!(percentiles.throughput.p50 > 0.0);
        assert!(percentiles.memory.p50 > 0.0);
    }

    #[test]
    fn test_aggregate_cold_starts() {
        let results = [
            create_test_result("kreuzberg", "pdf", OcrStatus::NotUsed, 100, 1_000_000.0, 10_000_000),
            create_test_result("kreuzberg", "pdf", OcrStatus::NotUsed, 200, 2_000_000.0, 20_000_000),
        ];

        let refs: Vec<&BenchmarkResult> = results.iter().collect();
        let cold_starts = aggregate_cold_starts(&refs);

        assert!(cold_starts.is_some());
        let cold_starts = cold_starts.unwrap();
        assert_eq!(cold_starts.sample_count, 2);
        assert!(cold_starts.p50_ms > 0.0);
    }

    #[test]
    fn test_ocr_unknown_handling() {
        // Test that Unknown OCR status is handled correctly
        let results = vec![BenchmarkResult {
            framework: "test-framework".to_string(),
            file_path: PathBuf::from("/tmp/test1.pdf"),
            file_size: 1024,
            success: true,
            error_message: None,
            duration: Duration::from_millis(100),
            extraction_duration: None,
            subprocess_overhead: None,
            metrics: PerformanceMetrics {
                peak_memory_bytes: 10_000_000,
                avg_cpu_percent: 50.0,
                throughput_bytes_per_sec: 10_240.0,
                p50_memory_bytes: 8_000_000,
                p95_memory_bytes: 9_500_000,
                p99_memory_bytes: 9_900_000,
            },
            quality: None,
            iterations: vec![],
            statistics: None,
            cold_start_duration: Some(Duration::from_millis(200)),
            file_extension: "pdf".to_string(),
            framework_capabilities: Default::default(),
            pdf_metadata: None,
            ocr_status: OcrStatus::Unknown, // Unknown status
        }];

        let aggregated = aggregate_new_format(&results);

        // Unknown should be in no_ocr group
        let framework_mode = aggregated.by_framework_mode.get("test-framework:single").unwrap();
        let file_type = framework_mode.by_file_type.get("pdf").unwrap();
        assert!(file_type.no_ocr.is_some());
        assert_eq!(file_type.no_ocr.as_ref().unwrap().sample_count, 1);
    }

    #[test]
    fn test_failed_results_excluded_from_percentiles() {
        // Test that failed results don't affect percentile calculations
        let results = vec![
            BenchmarkResult {
                framework: "test-framework".to_string(),
                file_path: PathBuf::from("/tmp/test1.pdf"),
                file_size: 1024,
                success: true,
                error_message: None,
                duration: Duration::from_millis(100),
                extraction_duration: None,
                subprocess_overhead: None,
                metrics: PerformanceMetrics {
                    peak_memory_bytes: 10_000_000,
                    avg_cpu_percent: 50.0,
                    throughput_bytes_per_sec: 10_240.0,
                    p50_memory_bytes: 8_000_000,
                    p95_memory_bytes: 9_500_000,
                    p99_memory_bytes: 9_900_000,
                },
                quality: None,
                iterations: vec![],
                statistics: None,
                cold_start_duration: None,
                file_extension: "pdf".to_string(),
                framework_capabilities: Default::default(),
                pdf_metadata: None,
                ocr_status: OcrStatus::NotUsed,
            },
            BenchmarkResult {
                framework: "test-framework".to_string(),
                file_path: PathBuf::from("/tmp/test2.pdf"),
                file_size: 2048,
                success: false, // Failed result
                error_message: Some("Test error".to_string()),
                duration: Duration::from_secs(0),
                extraction_duration: None,
                subprocess_overhead: None,
                metrics: PerformanceMetrics {
                    peak_memory_bytes: 0,
                    avg_cpu_percent: 0.0,
                    throughput_bytes_per_sec: 0.0,
                    p50_memory_bytes: 0,
                    p95_memory_bytes: 0,
                    p99_memory_bytes: 0,
                },
                quality: None,
                iterations: vec![],
                statistics: None,
                cold_start_duration: None,
                file_extension: "pdf".to_string(),
                framework_capabilities: Default::default(),
                pdf_metadata: None,
                ocr_status: OcrStatus::NotUsed,
            },
        ];

        let aggregated = aggregate_new_format(&results);

        let framework_mode = aggregated.by_framework_mode.get("test-framework:single").unwrap();
        let file_type = framework_mode.by_file_type.get("pdf").unwrap();
        let no_ocr = file_type.no_ocr.as_ref().unwrap();

        // sample_count should only count successful results
        assert_eq!(no_ocr.sample_count, 1);
        // success_rate_percent should account for all results
        assert_eq!(no_ocr.success_rate_percent, 50.0); // 1 success / 2 total = 50%
        // Percentiles based on 1 successful result
        assert_eq!(no_ocr.duration.p50, 100.0);
    }

    #[test]
    fn test_empty_input() {
        let results: Vec<BenchmarkResult> = vec![];
        let aggregated = aggregate_new_format(&results);

        assert_eq!(aggregated.by_framework_mode.len(), 0);
        assert_eq!(aggregated.metadata.total_results, 0);
    }

    #[test]
    fn test_percentile_interpolation() {
        // Test that p95 with [1,2,3,4,5] uses interpolation
        let sorted = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let p95 = calculate_percentile_value(&sorted, 0.95);

        // With linear interpolation: index = 0.95 * 4 = 3.8
        // Result = values[3] * 0.2 + values[4] * 0.8 = 4.0 * 0.2 + 5.0 * 0.8 = 4.8
        assert!((p95 - 4.8).abs() < 0.01);
    }
}

//! Output writers for benchmark results
//!
//! This module provides functionality for persisting benchmark results to disk
//! in JSON format.

use crate::stats::percentile_r7;
use crate::types::{BenchmarkResult, ErrorKind, expand_performance_iterations, successful_performance_samples};
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

    if !result.success && result.error_kind == ErrorKind::None {
        return Err(Error::Benchmark(format!(
            "Invalid result state for {}/{}: success=false but error_kind is None",
            result.framework,
            result.file_path.display()
        )));
    }

    if let Some(quality) = &result.quality {
        for (name, value) in [
            ("f1_score_text", quality.f1_score_text),
            ("f1_score_numeric", quality.f1_score_numeric),
            ("quality_score", quality.quality_score),
        ] {
            if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                return Err(Error::Benchmark(format!(
                    "Invalid result state for {}/{}: {name} must be a finite value in [0, 1], got {value}",
                    result.framework,
                    result.file_path.display()
                )));
            }
        }
        if let Some(value) = quality.f1_score_layout
            && (!value.is_finite() || !(0.0..=1.0).contains(&value))
        {
            return Err(Error::Benchmark(format!(
                "Invalid result state for {}/{}: f1_score_layout must be a finite value in [0, 1], got {value}",
                result.framework,
                result.file_path.display()
            )));
        }
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
    /// Number of independent process samples used for duration and RSS.
    #[serde(default)]
    pub performance_samples: usize,
    /// Number of framework-side extraction errors (not our fault)
    pub framework_errors: usize,
    /// Number of harness-side errors (potentially our fault)
    pub harness_errors: usize,
    /// Number of configuration/setup failures (infrastructure, not framework fault)
    #[serde(default)]
    pub config_setup_errors: usize,
    /// Number of extractions that timed out
    pub timeouts: usize,
    /// Number of extractions that returned empty content
    pub empty_content: usize,
    /// Number of extractions that returned non-empty output sharing zero tokens with a
    /// non-empty ground truth — distinct from `empty_content`.
    #[serde(default)]
    pub zero_overlap: usize,
    /// Number of layout-bearing extractions whose layout pass failed or was skipped.
    #[serde(default)]
    pub layout_failures: usize,
    /// Unique framework error messages with occurrence counts
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub error_details: HashMap<String, usize>,
    /// Success rate (0.0-1.0) over accountable samples; infrastructure failures are excluded.
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

/// Percentile ranks reported for every duration series.
const MEDIAN_PERCENTILE: f64 = 0.50;
const P95_PERCENTILE: f64 = 0.95;

/// Divisor turning raw byte counts into the megabytes the report renders.
const BYTES_PER_MEGABYTE: f64 = 1_000_000.0;

/// Per-`ErrorKind` failure tallies for one framework's results.
#[derive(Default)]
struct ErrorCounts {
    framework_errors: usize,
    harness_errors: usize,
    config_setup_errors: usize,
    timeouts: usize,
    empty_content: usize,
    zero_overlap: usize,
    layout_failures: usize,
}

impl ErrorCounts {
    fn tally(results: &[&BenchmarkResult]) -> Self {
        let count_of = |kind: ErrorKind| results.iter().filter(|r| r.error_kind == kind).count();
        Self {
            framework_errors: count_of(ErrorKind::FrameworkError),
            harness_errors: count_of(ErrorKind::HarnessError),
            config_setup_errors: count_of(ErrorKind::ConfigSetupError),
            timeouts: count_of(ErrorKind::Timeout),
            empty_content: count_of(ErrorKind::EmptyContent),
            zero_overlap: count_of(ErrorKind::ZeroOverlap),
            layout_failures: count_of(ErrorKind::LayoutFailed),
        }
    }
}

/// Duration, throughput and memory aggregates over the performance-eligible samples.
struct TimingStats {
    performance_samples: usize,
    avg_duration_ms: f64,
    median_duration_ms: f64,
    p95_duration_ms: f64,
    avg_extraction_duration_ms: Option<f64>,
    median_extraction_duration_ms: Option<f64>,
    p95_extraction_duration_ms: Option<f64>,
    avg_throughput_mbps: f64,
    avg_peak_memory_mb: f64,
}

fn mean_of(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<f64>() / values.len() as f64)
    }
}

fn finite_values(values: impl Iterator<Item = f64>) -> Vec<f64> {
    values.filter(|v| !v.is_nan() && v.is_finite()).collect()
}

/// Finite values in ascending order. Sorting is load-bearing twice over: the percentiles need it,
/// and the mean is taken over the sorted vector, so summation order must not drift. ~keep
fn sorted_finite(values: impl Iterator<Item = f64>) -> Vec<f64> {
    let mut sorted = finite_values(values);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    sorted
}

fn calculate_timing_stats(results: &[&BenchmarkResult]) -> TimingStats {
    let expanded_results = expand_performance_iterations(results);
    let expanded_refs: Vec<&BenchmarkResult> = expanded_results.iter().collect();
    let performance_results = successful_performance_samples(expanded_refs.iter().copied());

    let mut durations: Vec<f64> = performance_results
        .iter()
        .map(|r| r.duration.as_secs_f64() * 1000.0)
        .collect();
    let avg_duration_ms = mean_of(&durations).unwrap_or(0.0);
    durations.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let extraction_durations = sorted_finite(
        expanded_refs
            .iter()
            .filter(|result| result.success)
            .filter_map(|r| r.extraction_duration.map(|d| d.as_secs_f64() * 1000.0)),
    );

    // New batch rows repeat one process measurement across sibling documents
    // and are deduplicated above; legacy rows expose one positive throughput
    // anchor. Average only reported measurements in either representation. ~keep
    let reported_throughputs: Vec<f64> = performance_results
        .iter()
        .map(|r| r.metrics.throughput_bytes_per_sec / BYTES_PER_MEGABYTE)
        .filter(|throughput| throughput.is_finite() && *throughput > 0.0)
        .collect();

    let peak_memories: Vec<f64> = performance_results
        .iter()
        .map(|r| r.metrics.peak_memory_bytes as f64 / BYTES_PER_MEGABYTE)
        .collect();

    TimingStats {
        performance_samples: performance_results.len(),
        avg_duration_ms,
        median_duration_ms: if durations.is_empty() {
            0.0
        } else {
            percentile_r7(&durations, MEDIAN_PERCENTILE)
        },
        p95_duration_ms: if durations.is_empty() {
            0.0
        } else {
            percentile_r7(&durations, P95_PERCENTILE)
        },
        avg_extraction_duration_ms: mean_of(&extraction_durations),
        median_extraction_duration_ms: (!extraction_durations.is_empty())
            .then(|| percentile_r7(&extraction_durations, MEDIAN_PERCENTILE)),
        p95_extraction_duration_ms: (!extraction_durations.is_empty())
            .then(|| percentile_r7(&extraction_durations, P95_PERCENTILE)),
        avg_throughput_mbps: mean_of(&reported_throughputs).unwrap_or(0.0),
        avg_peak_memory_mb: mean_of(&peak_memories).unwrap_or(0.0),
    }
}

/// Mean quality scores over the successful results that carry a quality block.
struct QualityAverages {
    avg_f1_text: Option<f64>,
    avg_f1_numeric: Option<f64>,
    avg_f1_layout: Option<f64>,
    avg_quality_score: Option<f64>,
}

fn calculate_quality_averages(successful_results: &[&BenchmarkResult]) -> QualityAverages {
    let f1_texts = finite_values(
        successful_results
            .iter()
            .filter_map(|r| r.quality.as_ref().map(|q| q.f1_score_text)),
    );
    let f1_numerics = finite_values(
        successful_results
            .iter()
            .filter_map(|r| r.quality.as_ref().map(|q| q.f1_score_numeric)),
    );
    let f1_layouts = finite_values(
        successful_results
            .iter()
            .filter_map(|r| r.quality.as_ref().and_then(|q| q.f1_score_layout)),
    );
    let quality_scores = finite_values(
        successful_results
            .iter()
            .filter_map(|r| r.quality.as_ref().map(|q| q.quality_score)),
    );

    QualityAverages {
        avg_f1_text: mean_of(&f1_texts),
        avg_f1_numeric: mean_of(&f1_numerics),
        avg_f1_layout: mean_of(&f1_layouts),
        avg_quality_score: mean_of(&quality_scores),
    }
}

/// Calculate statistics for a framework's results
fn calculate_framework_stats(results: &[&BenchmarkResult]) -> FrameworkExtensionStats {
    let count = results.len();
    let successful = results.iter().filter(|r| r.success).count();
    let errors = ErrorCounts::tally(results);

    // Match aggregate/CLI semantics: only successful rows and framework-accountable failures
    // participate in the rate; harness and setup failures remain visible in their counters. ~keep
    let accountable = successful
        + errors.framework_errors
        + errors.timeouts
        + errors.empty_content
        + errors.zero_overlap
        + errors.layout_failures;
    let success_rate = if accountable > 0 {
        successful as f64 / accountable as f64
    } else {
        0.0
    };

    let mut error_details: HashMap<String, usize> = HashMap::new();
    for result in results.iter().filter(|r| !r.success) {
        if let Some(msg) = &result.error_message {
            *error_details.entry(msg.clone()).or_insert(0) += 1;
        }
    }

    let successful_results: Vec<&BenchmarkResult> = results.iter().copied().filter(|result| result.success).collect();
    let timing = calculate_timing_stats(results);
    let quality = calculate_quality_averages(&successful_results);

    FrameworkExtensionStats {
        count,
        successful,
        performance_samples: timing.performance_samples,
        framework_errors: errors.framework_errors,
        harness_errors: errors.harness_errors,
        config_setup_errors: errors.config_setup_errors,
        timeouts: errors.timeouts,
        empty_content: errors.empty_content,
        zero_overlap: errors.zero_overlap,
        layout_failures: errors.layout_failures,
        error_details,
        success_rate,
        avg_duration_ms: timing.avg_duration_ms,
        median_duration_ms: timing.median_duration_ms,
        p95_duration_ms: timing.p95_duration_ms,
        avg_extraction_duration_ms: timing.avg_extraction_duration_ms,
        median_extraction_duration_ms: timing.median_extraction_duration_ms,
        p95_extraction_duration_ms: timing.p95_extraction_duration_ms,
        avg_throughput_mbps: timing.avg_throughput_mbps,
        avg_peak_memory_mb: timing.avg_peak_memory_mb,
        avg_f1_text: quality.avg_f1_text,
        avg_f1_numeric: quality.avg_f1_numeric,
        avg_f1_layout: quality.avg_f1_layout,
        avg_quality_score: quality.avg_quality_score,
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
mod tests;

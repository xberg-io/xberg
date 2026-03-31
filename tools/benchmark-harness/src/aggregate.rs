//! Aggregation module for benchmark results (v2.2.0 output schema).
//!
//! Groups [`BenchmarkResult`] records by framework-and-mode, file type, and
//! OCR usage (yes/no), then computes percentile-based statistics for each
//! group. The output schema (`schema_version: "2.2.0"`) is consumed by the
//! consolidation dashboard.
//!
//! # Percentile methodology
//!
//! All percentiles use the **R-7 interpolation** method (the default in R and
//! NumPy) via [`crate::stats::percentile_r7`]. Three percentiles are reported
//! per metric: **p50** (median), **p95**, and **p99**. Values that are `NaN`
//! or `Inf` after interpolation are sanitized to `0.0` by
//! [`crate::stats::sanitize_f64`] so that downstream JSON consumers never
//! encounter non-finite floats.
//!
//! Failed results (non-zero `error_kind`) are excluded from percentile
//! calculations but still counted in `total_sample_count` to preserve the
//! `success_rate_percent` metric.

use crate::stats::{percentile_r7, sanitize_f64};
use crate::types::{BenchmarkResult, DiskSizeInfo, ErrorKind};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Consolidated results using new aggregation format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewConsolidatedResults {
    /// Schema version for this output format
    pub schema_version: String,
    /// Aggregated results grouped by framework:mode combination
    pub by_framework_mode: HashMap<String, FrameworkModeAggregation>,
    /// Disk sizes for each framework
    pub disk_sizes: HashMap<String, DiskSizeInfo>,
    /// Cross-framework comparison rankings
    pub comparison: ComparisonData,
    /// Metadata about the consolidation
    pub metadata: ConsolidationMetadata,
}

/// Cross-framework comparison rankings and deltas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonData {
    /// Frameworks ranked by median duration (fastest first)
    pub performance_ranking: Vec<RankedFramework>,
    /// Frameworks ranked by median throughput (highest first)
    pub throughput_ranking: Vec<RankedFramework>,
    /// Frameworks ranked by median memory usage (lowest first)
    pub memory_ranking: Vec<RankedFramework>,
    /// Frameworks ranked by median CPU usage (lowest first = most efficient)
    #[serde(default)]
    pub cpu_ranking: Vec<RankedFramework>,
    /// Frameworks ranked by quality score (highest first)
    pub quality_ranking: Vec<RankedFramework>,
    /// PDF-only: frameworks ranked by overall quality score (highest first)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pdf_quality_ranking: Vec<RankedFramework>,
    /// PDF-only: frameworks ranked by text F1 / TF1 (highest first)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pdf_tf1_ranking: Vec<RankedFramework>,
    /// PDF-only: frameworks ranked by structural F1 / SF1 (highest first)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pdf_sf1_ranking: Vec<RankedFramework>,
    /// Performance deltas relative to the fastest framework
    pub deltas_vs_baseline: HashMap<String, DeltaMetrics>,
}

/// A framework entry in a ranking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedFramework {
    /// Framework:mode key (e.g., "kreuzberg-rust:single")
    pub framework_mode: String,
    /// Rank (1-based)
    pub rank: usize,
    /// The metric value used for ranking
    pub value: f64,
    /// Ratio relative to the best in this ranking (1.0 = best)
    pub relative: f64,
}

/// Performance deltas relative to baseline (fastest framework)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaMetrics {
    /// Duration delta in ms (positive = slower)
    pub duration_delta_ms: f64,
    /// Duration delta as percentage
    pub duration_delta_percent: f64,
    /// Throughput delta in MB/s (negative = slower)
    pub throughput_delta_mbs: f64,
    /// Throughput delta as percentage
    pub throughput_delta_percent: f64,
    /// Memory delta in MB (positive = more)
    pub memory_delta_mb: f64,
    /// Memory delta as percentage
    pub memory_delta_percent: f64,
    /// CPU delta in percentage points (positive = higher CPU usage)
    #[serde(default)]
    pub cpu_delta_pp: f64,
    /// CPU delta as percentage relative to baseline
    #[serde(default)]
    pub cpu_delta_percent: f64,
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
    /// Number of successful samples used for percentile calculations
    pub successful_sample_count: usize,
    /// Total number of samples in this group (including failed)
    pub total_sample_count: usize,
    /// Number of framework-side extraction errors (not our fault)
    pub framework_errors: usize,
    /// Number of harness-side errors (potentially our fault)
    pub harness_errors: usize,
    /// Number of extractions that timed out
    pub timeouts: usize,
    /// Number of extractions that returned empty content
    pub empty_content: usize,
    /// Unique error messages with occurrence counts
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub error_details: HashMap<String, usize>,
    /// Throughput percentiles (p50, p95, p99) in MB/s
    pub throughput: Percentiles,
    /// Memory percentiles (p50, p95, p99) in MB
    pub memory: Percentiles,
    /// Duration percentiles (p50, p95, p99) in ms
    pub duration: Percentiles,
    /// CPU usage percentiles (p50, p95, p99) as percentage (0-100, normalized across cores)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<Percentiles>,
    /// Success rate as percentage (0-100)
    pub success_rate_percent: f64,
    /// Extraction duration percentiles (p50, p95, p99) in ms
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extraction_duration: Option<Percentiles>,
    /// Quality score percentiles (p50, p95, p99) — 0.0 to 1.0
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<QualityPercentiles>,
}

/// Quality percentile values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityPercentiles {
    /// Median F1 text score
    pub f1_text_p50: f64,
    /// Median F1 numeric score
    pub f1_numeric_p50: f64,
    /// Median F1 layout/structural score (SF1)
    pub f1_layout_p50: f64,
    /// Median overall quality score
    pub quality_score_p50: f64,
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
            schema_version: "2.2.0".to_string(),
            by_framework_mode: HashMap::new(),
            disk_sizes: HashMap::new(),
            comparison: ComparisonData {
                performance_ranking: Vec::new(),
                throughput_ranking: Vec::new(),
                memory_ranking: Vec::new(),
                cpu_ranking: Vec::new(),
                quality_ranking: Vec::new(),
                pdf_quality_ranking: Vec::new(),
                pdf_tf1_ranking: Vec::new(),
                pdf_sf1_ranking: Vec::new(),
                deltas_vs_baseline: HashMap::new(),
            },
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

    let comparison = build_comparison(&aggregated_by_framework_mode);

    NewConsolidatedResults {
        schema_version: "2.2.0".to_string(),
        by_framework_mode: aggregated_by_framework_mode,
        disk_sizes,
        comparison,
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

    // OCR status grouping:
    // - OcrStatus::Used → "with_ocr" group
    // - OcrStatus::NotUsed → "no_ocr" group
    // - OcrStatus::Unknown → infer from file type: image formats → "with_ocr", others → "no_ocr"
    let is_ocr_result = |r: &&BenchmarkResult| -> bool {
        match r.ocr_status {
            OcrStatus::Used => true,
            OcrStatus::NotUsed => false,
            OcrStatus::Unknown => matches!(
                r.file_extension.to_lowercase().as_str(),
                "jpg" | "jpeg" | "png" | "gif" | "bmp" | "tiff" | "tif" | "webp" | "jp2" | "jpx" | "jpm" | "mj2"
            ),
        }
    };

    let no_ocr: Vec<&BenchmarkResult> = results.iter().filter(|r| !is_ocr_result(r)).copied().collect();

    let with_ocr: Vec<&BenchmarkResult> = results.iter().filter(|r| is_ocr_result(r)).copied().collect();

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
        .filter(|&v| v > 0.0 && v.is_finite()) // Filter zero values (invalid measurements)
        .collect();

    let mut memories: Vec<f64> = successful
        .iter()
        .map(|r| r.metrics.peak_memory_bytes as f64 / 1_000_000.0) // Convert to MB
        .filter(|&v| !v.is_nan() && v.is_finite())
        .collect();

    let mut extraction_durations: Vec<f64> = successful
        .iter()
        .filter_map(|r| r.extraction_duration.map(|d| d.as_secs_f64() * 1000.0))
        .filter(|&v| !v.is_nan() && v.is_finite())
        .collect();

    let mut cpus: Vec<f64> = successful
        .iter()
        .map(|r| r.metrics.avg_cpu_percent)
        .filter(|&v| v > 0.0 && v.is_finite())
        .collect();

    // Sort for percentile calculation (NaN-safe)
    durations.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    throughputs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    memories.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    extraction_durations.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    cpus.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    // Build percentiles with NaN/Inf validation
    let duration = Percentiles {
        p50: sanitize_f64(percentile_r7(&durations, 0.50)),
        p95: sanitize_f64(percentile_r7(&durations, 0.95)),
        p99: sanitize_f64(percentile_r7(&durations, 0.99)),
    };

    let throughput = Percentiles {
        p50: sanitize_f64(percentile_r7(&throughputs, 0.50)),
        p95: sanitize_f64(percentile_r7(&throughputs, 0.95)),
        p99: sanitize_f64(percentile_r7(&throughputs, 0.99)),
    };

    let memory = Percentiles {
        p50: sanitize_f64(percentile_r7(&memories, 0.50)),
        p95: sanitize_f64(percentile_r7(&memories, 0.95)),
        p99: sanitize_f64(percentile_r7(&memories, 0.99)),
    };

    let extraction_duration = if !extraction_durations.is_empty() {
        Some(Percentiles {
            p50: sanitize_f64(percentile_r7(&extraction_durations, 0.50)),
            p95: sanitize_f64(percentile_r7(&extraction_durations, 0.95)),
            p99: sanitize_f64(percentile_r7(&extraction_durations, 0.99)),
        })
    } else {
        None
    };

    let cpu = if !cpus.is_empty() {
        Some(Percentiles {
            p50: sanitize_f64(percentile_r7(&cpus, 0.50)),
            p95: sanitize_f64(percentile_r7(&cpus, 0.95)),
            p99: sanitize_f64(percentile_r7(&cpus, 0.99)),
        })
    } else {
        None
    };

    let success_rate_percent = if !results.is_empty() {
        (successful.len() as f64 / results.len() as f64) * 100.0
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

    // Quality percentiles
    let quality = {
        let mut f1_texts: Vec<f64> = successful
            .iter()
            .filter_map(|r| r.quality.as_ref().map(|q| q.f1_score_text))
            .filter(|v| !v.is_nan() && v.is_finite())
            .collect();
        let mut f1_numerics: Vec<f64> = successful
            .iter()
            .filter_map(|r| r.quality.as_ref().map(|q| q.f1_score_numeric))
            .filter(|v| !v.is_nan() && v.is_finite())
            .collect();
        let mut f1_layouts: Vec<f64> = successful
            .iter()
            .filter_map(|r| r.quality.as_ref().map(|q| q.f1_score_layout))
            .filter(|v| !v.is_nan() && v.is_finite())
            .collect();
        let mut quality_scores: Vec<f64> = successful
            .iter()
            .filter_map(|r| r.quality.as_ref().map(|q| q.quality_score))
            .filter(|v| !v.is_nan() && v.is_finite())
            .collect();

        if !quality_scores.is_empty() {
            f1_texts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            f1_numerics.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            f1_layouts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            quality_scores.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

            Some(QualityPercentiles {
                f1_text_p50: sanitize_f64(percentile_r7(&f1_texts, 0.50)),
                f1_numeric_p50: sanitize_f64(percentile_r7(&f1_numerics, 0.50)),
                f1_layout_p50: sanitize_f64(percentile_r7(&f1_layouts, 0.50)),
                quality_score_p50: sanitize_f64(percentile_r7(&quality_scores, 0.50)),
            })
        } else {
            None
        }
    };

    PerformancePercentiles {
        successful_sample_count: successful.len(),
        total_sample_count: results.len(),
        framework_errors,
        harness_errors,
        timeouts,
        empty_content,
        error_details,
        throughput,
        memory,
        duration,
        cpu,
        success_rate_percent,
        extraction_duration,
        quality,
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
        p50_ms: sanitize_f64(percentile_r7(&sorted, 0.50)),
        p95_ms: sanitize_f64(percentile_r7(&sorted, 0.95)),
        p99_ms: sanitize_f64(percentile_r7(&sorted, 0.99)),
    })
}

/// Extract framework name and mode from framework string
///
/// Framework naming convention: {base}-{variant}-{mode}
/// Examples: kreuzberg-rust, kreuzberg-python-sync, kreuzberg-python-batch
/// Variants: -sync, -async (mapped to "single" mode)
/// Modes: -batch (mapped to "batch" mode), absence (mapped to "single" mode)
///
/// Returns (framework_name, mode) where mode is one of:
/// - "batch" if ends with "-batch"
/// - "single" otherwise (default)
///
/// The -sync/-async suffixes are stripped for aggregation because we unify
/// implementations per language — sync vs async is a language-specific detail.
fn extract_framework_and_mode(framework_name: &str) -> (&str, &str) {
    // First, check and strip -batch suffix (mode indicator)
    if let Some(base) = framework_name.strip_suffix("-batch") {
        // Then strip -sync/-async suffixes from the base (implementation details)
        let normalized = base
            .strip_suffix("-sync")
            .or_else(|| base.strip_suffix("-async"))
            .unwrap_or(base);
        (normalized, "batch")
    } else {
        // No -batch suffix, so check and strip -sync/-async suffixes (implementation details)
        let normalized = framework_name
            .strip_suffix("-sync")
            .or_else(|| framework_name.strip_suffix("-async"))
            .unwrap_or(framework_name);
        (normalized, "single")
    }
}

/// Build cross-framework comparison rankings from aggregated data
///
/// Metrics are weighted by successful_sample_count so that file types with more
/// samples (e.g., 93 PDFs) dominate the ranking over file types with fewer samples
/// (e.g., 1 BMP). This prevents frameworks that handle more file types or do OCR
/// from being unfairly penalized in the overall ranking.
fn build_comparison(by_framework_mode: &HashMap<String, FrameworkModeAggregation>) -> ComparisonData {
    // Collect weighted median metrics per framework:mode
    // (key, duration_p50, throughput_p50, memory_p50, quality_p50, cpu_p50)
    let mut metrics: Vec<(String, f64, f64, f64, f64, f64)> = Vec::new();

    for (key, agg) in by_framework_mode {
        // (value, weight) pairs for weighted averaging
        let mut durations: Vec<(f64, usize)> = Vec::new();
        let mut throughputs: Vec<(f64, usize)> = Vec::new();
        let mut memories: Vec<(f64, usize)> = Vec::new();
        let mut qualities: Vec<(f64, usize)> = Vec::new();
        let mut cpus: Vec<(f64, usize)> = Vec::new();

        for ft in agg.by_file_type.values() {
            for perf in [&ft.no_ocr, &ft.with_ocr].into_iter().flatten() {
                // Skip groups where all samples failed — their 0.0 values would
                // pollute rankings (e.g., docling showing 0.0ms when libGL is missing).
                if perf.successful_sample_count == 0 {
                    continue;
                }
                let weight = perf.successful_sample_count;
                durations.push((perf.duration.p50, weight));
                throughputs.push((perf.throughput.p50, weight));
                memories.push((perf.memory.p50, weight));
                if let Some(q) = &perf.quality {
                    qualities.push((q.quality_score_p50, weight));
                }
                if let Some(c) = &perf.cpu {
                    cpus.push((c.p50, weight));
                }
            }
        }

        if durations.is_empty() {
            continue;
        }

        let weighted_avg = |items: &[(f64, usize)]| -> f64 {
            let finite: Vec<(f64, usize)> = items.iter().copied().filter(|(v, _)| v.is_finite()).collect();
            let total_weight: usize = finite.iter().map(|(_, w)| w).sum();
            if total_weight == 0 {
                f64::NAN
            } else {
                finite.iter().map(|(v, w)| v * (*w as f64)).sum::<f64>() / total_weight as f64
            }
        };

        metrics.push((
            key.clone(),
            weighted_avg(&durations),
            weighted_avg(&throughputs),
            weighted_avg(&memories),
            weighted_avg(&qualities),
            weighted_avg(&cpus),
        ));
    }

    // Performance ranking (lower duration = better, rank 1)
    let mut perf = metrics.clone();
    perf.retain(|m| m.1.is_finite());
    perf.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let baseline_dur = perf.first().map(|r| r.1).unwrap_or(1.0);
    let performance_ranking: Vec<RankedFramework> = perf
        .iter()
        .enumerate()
        .map(|(i, (k, v, ..))| RankedFramework {
            framework_mode: k.clone(),
            rank: i + 1,
            value: *v,
            relative: if baseline_dur > 0.0 { *v / baseline_dur } else { 1.0 },
        })
        .collect();

    // Throughput ranking (higher = better)
    let mut thr = metrics.clone();
    thr.retain(|m| m.2.is_finite());
    thr.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    let baseline_thr = thr.first().map(|r| r.2).unwrap_or(1.0);
    let throughput_ranking: Vec<RankedFramework> = thr
        .iter()
        .enumerate()
        .map(|(i, (k, _, v, ..))| RankedFramework {
            framework_mode: k.clone(),
            rank: i + 1,
            value: *v,
            relative: if baseline_thr > 0.0 { *v / baseline_thr } else { 1.0 },
        })
        .collect();

    // Memory ranking (lower = better)
    let mut mem = metrics.clone();
    mem.retain(|m| m.3.is_finite());
    mem.sort_by(|a, b| a.3.partial_cmp(&b.3).unwrap_or(std::cmp::Ordering::Equal));
    let baseline_mem = mem.first().map(|r| r.3).unwrap_or(1.0);
    let memory_ranking: Vec<RankedFramework> = mem
        .iter()
        .enumerate()
        .map(|(i, (k, _, _, v, ..))| RankedFramework {
            framework_mode: k.clone(),
            rank: i + 1,
            value: *v,
            relative: if baseline_mem > 0.0 { *v / baseline_mem } else { 1.0 },
        })
        .collect();

    // CPU ranking (lower = more efficient, rank 1)
    let mut cpu = metrics.clone();
    cpu.retain(|m| m.5.is_finite());
    cpu.sort_by(|a, b| a.5.partial_cmp(&b.5).unwrap_or(std::cmp::Ordering::Equal));
    let baseline_cpu = cpu.first().map(|r| r.5).unwrap_or(1.0);
    let cpu_ranking: Vec<RankedFramework> = cpu
        .iter()
        .enumerate()
        .map(|(i, (k, _, _, _, _, v))| RankedFramework {
            framework_mode: k.clone(),
            rank: i + 1,
            value: *v,
            relative: if baseline_cpu > 0.0 { *v / baseline_cpu } else { 1.0 },
        })
        .collect();

    // Quality ranking (higher = better)
    let mut qual = metrics.clone();
    qual.retain(|m| m.4.is_finite());
    qual.sort_by(|a, b| b.4.partial_cmp(&a.4).unwrap_or(std::cmp::Ordering::Equal));
    let baseline_qual = qual.first().map(|r| r.4).unwrap_or(1.0);
    let quality_ranking: Vec<RankedFramework> = qual
        .iter()
        .enumerate()
        .map(|(i, (k, _, _, _, v, _))| RankedFramework {
            framework_mode: k.clone(),
            rank: i + 1,
            value: *v,
            relative: if baseline_qual > 0.0 { *v / baseline_qual } else { 1.0 },
        })
        .collect();

    // Deltas vs baseline (fastest framework)
    let mut deltas_vs_baseline = HashMap::new();
    if let Some(baseline) = metrics
        .iter()
        .filter(|(_, dur, _, _, _, _)| dur.is_finite())
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
    {
        for (k, dur, thr, mem_val, _, cpu_val) in &metrics {
            if k != &baseline.0 {
                deltas_vs_baseline.insert(
                    k.clone(),
                    DeltaMetrics {
                        duration_delta_ms: dur - baseline.1,
                        duration_delta_percent: if baseline.1 > 0.0 {
                            ((dur - baseline.1) / baseline.1) * 100.0
                        } else {
                            0.0
                        },
                        throughput_delta_mbs: thr - baseline.2,
                        throughput_delta_percent: if baseline.2 > 0.0 {
                            ((thr - baseline.2) / baseline.2) * 100.0
                        } else {
                            0.0
                        },
                        memory_delta_mb: mem_val - baseline.3,
                        memory_delta_percent: if baseline.3 > 0.0 {
                            ((mem_val - baseline.3) / baseline.3) * 100.0
                        } else {
                            0.0
                        },
                        cpu_delta_pp: cpu_val - baseline.5,
                        cpu_delta_percent: if baseline.5 > 0.0 {
                            ((cpu_val - baseline.5) / baseline.5) * 100.0
                        } else {
                            0.0
                        },
                    },
                );
            }
        }
    }

    // PDF-specific quality rankings (quality, TF1, SF1)
    // Collect PDF quality metrics per framework:mode
    let mut pdf_metrics: Vec<(String, f64, f64, f64)> = Vec::new(); // (key, quality, tf1, sf1)
    for (key, agg) in by_framework_mode {
        if let Some(pdf_ft) = agg.by_file_type.get("pdf") {
            let mut qualities: Vec<(f64, usize)> = Vec::new();
            let mut tf1s: Vec<(f64, usize)> = Vec::new();
            let mut sf1s: Vec<(f64, usize)> = Vec::new();
            for perf in [&pdf_ft.no_ocr, &pdf_ft.with_ocr].into_iter().flatten() {
                if perf.successful_sample_count == 0 {
                    continue;
                }
                if let Some(q) = &perf.quality {
                    let w = perf.successful_sample_count;
                    qualities.push((q.quality_score_p50, w));
                    tf1s.push((q.f1_text_p50, w));
                    sf1s.push((q.f1_layout_p50, w));
                }
            }
            let weighted_avg = |items: &[(f64, usize)]| -> f64 {
                let finite: Vec<(f64, usize)> = items.iter().copied().filter(|(v, _)| v.is_finite()).collect();
                let total_weight: usize = finite.iter().map(|(_, w)| w).sum();
                if total_weight == 0 {
                    f64::NAN
                } else {
                    finite.iter().map(|(v, w)| v * (*w as f64)).sum::<f64>() / total_weight as f64
                }
            };
            let q = weighted_avg(&qualities);
            let t = weighted_avg(&tf1s);
            let s = weighted_avg(&sf1s);
            if q.is_finite() {
                pdf_metrics.push((key.clone(), q, t, s));
            }
        }
    }

    let build_ranking = |items: &mut Vec<(String, f64)>| -> Vec<RankedFramework> {
        items.retain(|(_, v)| v.is_finite());
        items.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let best = items.first().map(|r| r.1).unwrap_or(1.0);
        items
            .iter()
            .enumerate()
            .map(|(i, (k, v))| RankedFramework {
                framework_mode: k.clone(),
                rank: i + 1,
                value: *v,
                relative: if best > 0.0 { *v / best } else { 1.0 },
            })
            .collect()
    };

    let mut pdf_qual_items: Vec<(String, f64)> = pdf_metrics.iter().map(|(k, q, _, _)| (k.clone(), *q)).collect();
    let mut pdf_tf1_items: Vec<(String, f64)> = pdf_metrics.iter().map(|(k, _, t, _)| (k.clone(), *t)).collect();
    let mut pdf_sf1_items: Vec<(String, f64)> = pdf_metrics.iter().map(|(k, _, _, s)| (k.clone(), *s)).collect();

    let pdf_quality_ranking = build_ranking(&mut pdf_qual_items);
    let pdf_tf1_ranking = build_ranking(&mut pdf_tf1_items);
    let pdf_sf1_ranking = build_ranking(&mut pdf_sf1_items);

    ComparisonData {
        performance_ranking,
        throughput_ranking,
        memory_ranking,
        cpu_ranking,
        quality_ranking,
        pdf_quality_ranking,
        pdf_tf1_ranking,
        pdf_sf1_ranking,
        deltas_vs_baseline,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ErrorKind, FrameworkCapabilities, OcrStatus, PerformanceMetrics};
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
            error_kind: ErrorKind::None,
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
            extracted_text: None,
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
    fn test_percentile_r7() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(percentile_r7(&values, 0.0), 1.0);
        assert_eq!(percentile_r7(&values, 0.5), 3.0);
        assert_eq!(percentile_r7(&values, 1.0), 5.0);
        assert_eq!(percentile_r7(&[], 0.5), 0.0);
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

        assert_eq!(pdf_agg.no_ocr.as_ref().unwrap().successful_sample_count, 1);
        assert_eq!(pdf_agg.with_ocr.as_ref().unwrap().successful_sample_count, 1);
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

        assert_eq!(percentiles.successful_sample_count, 3);
        assert_eq!(percentiles.total_sample_count, 3);
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
            error_kind: ErrorKind::None,
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
            extracted_text: None,
        }];

        let aggregated = aggregate_new_format(&results);

        // Unknown should be in no_ocr group
        let framework_mode = aggregated.by_framework_mode.get("test-framework:single").unwrap();
        let file_type = framework_mode.by_file_type.get("pdf").unwrap();
        assert!(file_type.no_ocr.is_some());
        assert_eq!(file_type.no_ocr.as_ref().unwrap().successful_sample_count, 1);
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
                error_kind: ErrorKind::None,
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
                extracted_text: None,
            },
            BenchmarkResult {
                framework: "test-framework".to_string(),
                file_path: PathBuf::from("/tmp/test2.pdf"),
                file_size: 2048,
                success: false, // Failed result
                error_message: Some("Test error".to_string()),
                error_kind: ErrorKind::HarnessError,
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
                extracted_text: None,
            },
        ];

        let aggregated = aggregate_new_format(&results);

        let framework_mode = aggregated.by_framework_mode.get("test-framework:single").unwrap();
        let file_type = framework_mode.by_file_type.get("pdf").unwrap();
        let no_ocr = file_type.no_ocr.as_ref().unwrap();

        // successful_sample_count should only count successful results
        assert_eq!(no_ocr.successful_sample_count, 1);
        assert_eq!(no_ocr.total_sample_count, 2);
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
        let p95 = percentile_r7(&sorted, 0.95);

        // With linear interpolation: index = 0.95 * 4 = 3.8
        // Result = values[3] * 0.2 + values[4] * 0.8 = 4.0 * 0.2 + 5.0 * 0.8 = 4.8
        assert!((p95 - 4.8).abs() < 0.01);
    }

    // ============================================================================
    // Tests for extraction_duration aggregation in new format
    // ============================================================================

    #[test]
    fn test_calculate_percentiles_extraction_duration_all_present() {
        // Test: All results have extraction_duration -> percentiles populated
        let mut result1 = create_test_result("framework1", "pdf", OcrStatus::NotUsed, 100, 1_000_000.0, 10_000_000);
        result1.extraction_duration = Some(Duration::from_millis(80));

        let mut result2 = create_test_result("framework1", "pdf", OcrStatus::NotUsed, 150, 1_000_000.0, 10_000_000);
        result2.extraction_duration = Some(Duration::from_millis(120));

        let mut result3 = create_test_result("framework1", "pdf", OcrStatus::NotUsed, 200, 1_000_000.0, 10_000_000);
        result3.extraction_duration = Some(Duration::from_millis(160));

        let refs = vec![&result1, &result2, &result3];
        let percentiles = calculate_percentiles(&refs);

        assert!(percentiles.extraction_duration.is_some());
        let ext_dur = percentiles.extraction_duration.as_ref().unwrap();
        assert!((ext_dur.p50 - 120.0).abs() < 0.1); // median: 120
        assert!(ext_dur.p95 > 120.0); // p95 should be between 120 and 160
        assert!(ext_dur.p95 <= 160.0);
    }

    #[test]
    fn test_calculate_percentiles_extraction_duration_all_none() {
        // Test: All results have extraction_duration = None -> extraction_duration None
        let result1 = create_test_result("framework1", "pdf", OcrStatus::NotUsed, 100, 1_000_000.0, 10_000_000);
        let result2 = create_test_result("framework1", "pdf", OcrStatus::NotUsed, 150, 1_000_000.0, 10_000_000);
        let result3 = create_test_result("framework1", "pdf", OcrStatus::NotUsed, 200, 1_000_000.0, 10_000_000);

        let refs = vec![&result1, &result2, &result3];
        let percentiles = calculate_percentiles(&refs);

        assert!(percentiles.extraction_duration.is_none());
    }

    #[test]
    fn test_calculate_percentiles_extraction_duration_mixed() {
        // Test: Mixed Some/None extraction_duration -> only Some values used
        let mut result1 = create_test_result("framework1", "pdf", OcrStatus::NotUsed, 100, 1_000_000.0, 10_000_000);
        result1.extraction_duration = Some(Duration::from_millis(80));

        let result2 = create_test_result("framework1", "pdf", OcrStatus::NotUsed, 150, 1_000_000.0, 10_000_000);
        // result2.extraction_duration = None

        let mut result3 = create_test_result("framework1", "pdf", OcrStatus::NotUsed, 200, 1_000_000.0, 10_000_000);
        result3.extraction_duration = Some(Duration::from_millis(160));

        let refs = vec![&result1, &result2, &result3];
        let percentiles = calculate_percentiles(&refs);

        assert!(percentiles.extraction_duration.is_some());
        let ext_dur = percentiles.extraction_duration.as_ref().unwrap();
        // Only 80 and 160 used, median should be 120
        assert!((ext_dur.p50 - 120.0).abs() < 0.1);
    }

    #[test]
    fn test_calculate_percentiles_extraction_duration_filters_invalid() {
        // Test: NaN/infinite extraction durations filtered out
        // Note: We can't directly create NaN with Duration, so we test the filtering logic
        // by ensuring valid values are correctly processed
        let mut result1 = create_test_result("framework1", "pdf", OcrStatus::NotUsed, 100, 1_000_000.0, 10_000_000);
        result1.extraction_duration = Some(Duration::from_millis(80));

        let mut result2 = create_test_result("framework1", "pdf", OcrStatus::NotUsed, 150, 1_000_000.0, 10_000_000);
        result2.extraction_duration = Some(Duration::from_millis(120));

        let mut result3 = create_test_result("framework1", "pdf", OcrStatus::NotUsed, 200, 1_000_000.0, 10_000_000);
        result3.extraction_duration = Some(Duration::from_millis(160));

        let refs = vec![&result1, &result2, &result3];
        let percentiles = calculate_percentiles(&refs);

        // All values should be present and valid
        assert!(percentiles.extraction_duration.is_some());
        let ext_dur = percentiles.extraction_duration.as_ref().unwrap();
        assert!(ext_dur.p50.is_finite());
        assert!(!ext_dur.p50.is_nan());
    }

    #[test]
    fn test_calculate_percentiles_extraction_duration_with_failed_results() {
        // Test: Failed results excluded from extraction_duration calculation
        let mut result1 = create_test_result("framework1", "pdf", OcrStatus::NotUsed, 100, 1_000_000.0, 10_000_000);
        result1.extraction_duration = Some(Duration::from_millis(80));

        let mut result2_failed = create_test_result("framework1", "pdf", OcrStatus::NotUsed, 0, 0.0, 0);
        result2_failed.success = false;
        result2_failed.error_message = Some("Failed".to_string());
        result2_failed.extraction_duration = Some(Duration::from_millis(50)); // Should be ignored

        let mut result3 = create_test_result("framework1", "pdf", OcrStatus::NotUsed, 200, 1_000_000.0, 10_000_000);
        result3.extraction_duration = Some(Duration::from_millis(160));

        let refs = vec![&result1, &result2_failed, &result3];
        let percentiles = calculate_percentiles(&refs);

        // Only result1 and result3 should be used (80 and 160)
        assert!(percentiles.extraction_duration.is_some());
        let ext_dur = percentiles.extraction_duration.as_ref().unwrap();
        assert_eq!(percentiles.successful_sample_count, 2); // Only 2 successful results
        assert_eq!(percentiles.total_sample_count, 3);
        assert!((ext_dur.p50 - 120.0).abs() < 0.1); // median: 120
    }

    #[test]
    fn test_aggregate_by_ocr_status_extraction_duration() {
        // Test: Extraction duration aggregated correctly with OCR status split
        let mut result_no_ocr_1 =
            create_test_result("framework1", "pdf", OcrStatus::NotUsed, 100, 1_000_000.0, 10_000_000);
        result_no_ocr_1.extraction_duration = Some(Duration::from_millis(80));

        let mut result_no_ocr_2 =
            create_test_result("framework1", "pdf", OcrStatus::NotUsed, 150, 1_000_000.0, 10_000_000);
        result_no_ocr_2.extraction_duration = Some(Duration::from_millis(120));

        let mut result_with_ocr = create_test_result("framework1", "pdf", OcrStatus::Used, 300, 500_000.0, 20_000_000);
        result_with_ocr.extraction_duration = Some(Duration::from_millis(250));

        let refs = vec![&result_no_ocr_1, &result_no_ocr_2, &result_with_ocr];
        let (no_ocr, with_ocr) = aggregate_by_ocr_status(&refs);

        // No OCR group
        assert!(no_ocr.is_some());
        let no_ocr_perf = no_ocr.unwrap();
        assert!(no_ocr_perf.extraction_duration.is_some());
        assert_eq!(no_ocr_perf.extraction_duration.as_ref().unwrap().p50, 100.0); // median of [80, 120]

        // With OCR group
        assert!(with_ocr.is_some());
        let with_ocr_perf = with_ocr.unwrap();
        assert!(with_ocr_perf.extraction_duration.is_some());
        assert_eq!(with_ocr_perf.extraction_duration.as_ref().unwrap().p50, 250.0);
    }

    #[test]
    fn test_aggregate_new_format_extraction_duration_preserved() {
        // Test: aggregate_new_format preserves extraction_duration statistics
        let mut result1 = create_test_result(
            "kreuzberg-sync",
            "pdf",
            OcrStatus::NotUsed,
            100,
            1_000_000.0,
            10_000_000,
        );
        result1.extraction_duration = Some(Duration::from_millis(80));

        let mut result2 = create_test_result(
            "kreuzberg-sync",
            "pdf",
            OcrStatus::NotUsed,
            150,
            1_000_000.0,
            10_000_000,
        );
        result2.extraction_duration = Some(Duration::from_millis(120));

        let results = vec![result1, result2];
        let aggregated = aggregate_new_format(&results);

        let framework_mode = aggregated.by_framework_mode.get("kreuzberg:single").unwrap();
        let pdf_stats = framework_mode.by_file_type.get("pdf").unwrap();
        let no_ocr = pdf_stats.no_ocr.as_ref().unwrap();

        assert!(no_ocr.extraction_duration.is_some());
        let ext_dur = no_ocr.extraction_duration.as_ref().unwrap();
        assert!((ext_dur.p50 - 100.0).abs() < 0.1);
    }

    #[test]
    fn test_calculate_percentiles_extraction_duration_single_value() {
        // Test: Single extraction_duration value -> all percentiles return that value
        let mut result = create_test_result("framework1", "pdf", OcrStatus::NotUsed, 100, 1_000_000.0, 10_000_000);
        result.extraction_duration = Some(Duration::from_millis(80));

        let refs = vec![&result];
        let percentiles = calculate_percentiles(&refs);

        assert!(percentiles.extraction_duration.is_some());
        let ext_dur = percentiles.extraction_duration.as_ref().unwrap();
        assert_eq!(ext_dur.p50, 80.0);
        assert_eq!(ext_dur.p95, 80.0);
        assert_eq!(ext_dur.p99, 80.0);
    }

    #[test]
    fn test_calculate_percentiles_extraction_duration_large_dataset() {
        // Test: Large dataset with extraction_duration -> percentiles calculated correctly
        let mut results = vec![];
        for i in 1..=100 {
            let mut result =
                create_test_result("framework1", "pdf", OcrStatus::NotUsed, i * 10, 1_000_000.0, 10_000_000);
            result.extraction_duration = Some(Duration::from_millis(i * 8));
            results.push(result);
        }

        let refs: Vec<&BenchmarkResult> = results.iter().collect();
        let percentiles = calculate_percentiles(&refs);

        assert!(percentiles.extraction_duration.is_some());
        let ext_dur = percentiles.extraction_duration.as_ref().unwrap();

        // p50 (median) of 1-100 scaled by 8: around 404-408ms
        assert!(ext_dur.p50 >= 400.0 && ext_dur.p50 <= 410.0);

        // p95 should be higher than p50
        assert!(ext_dur.p95 > ext_dur.p50);

        // p99 should be higher than p95
        assert!(ext_dur.p99 > ext_dur.p95);
    }

    #[test]
    fn test_calculate_percentiles_extraction_duration_no_extraction_some_failed() {
        // Test: No extraction_duration data, some failures -> extraction_duration None
        let result1_failed = BenchmarkResult {
            framework: "test".to_string(),
            file_path: PathBuf::from("test1.pdf"),
            file_size: 1024,
            success: false,
            error_message: Some("Error".to_string()),
            error_kind: ErrorKind::HarnessError,
            duration: Duration::from_millis(0),
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
            framework_capabilities: FrameworkCapabilities::default(),
            pdf_metadata: None,
            ocr_status: OcrStatus::NotUsed,
            extracted_text: None,
        };

        let result2 = create_test_result("framework1", "pdf", OcrStatus::NotUsed, 100, 1_000_000.0, 10_000_000);

        let refs = vec![&result1_failed, &result2];
        let percentiles = calculate_percentiles(&refs);

        assert!(percentiles.extraction_duration.is_none());
        assert_eq!(percentiles.success_rate_percent, 50.0);
    }

    // ============================================================================
    // Tests for CPU aggregation
    // ============================================================================

    #[test]
    fn test_calculate_percentiles_cpu_populated() {
        // Test: Results with avg_cpu_percent > 0 produce CPU percentiles
        let mut r1 = create_test_result("framework1", "pdf", OcrStatus::NotUsed, 100, 1_000_000.0, 10_000_000);
        r1.metrics.avg_cpu_percent = 25.0;

        let mut r2 = create_test_result("framework1", "pdf", OcrStatus::NotUsed, 150, 1_000_000.0, 10_000_000);
        r2.metrics.avg_cpu_percent = 75.0;

        let mut r3 = create_test_result("framework1", "pdf", OcrStatus::NotUsed, 200, 1_000_000.0, 10_000_000);
        r3.metrics.avg_cpu_percent = 50.0;

        let refs = vec![&r1, &r2, &r3];
        let percentiles = calculate_percentiles(&refs);

        assert!(percentiles.cpu.is_some());
        let cpu = percentiles.cpu.as_ref().unwrap();
        assert_eq!(cpu.p50, 50.0); // median of [25, 50, 75]
        assert!(cpu.p95 > cpu.p50);
        assert!(cpu.p99 >= cpu.p95);
    }

    #[test]
    fn test_calculate_percentiles_cpu_zero_excluded() {
        // Test: avg_cpu_percent = 0.0 is filtered out (fallback snapshot path)
        let mut r1 = create_test_result("framework1", "pdf", OcrStatus::NotUsed, 100, 1_000_000.0, 10_000_000);
        r1.metrics.avg_cpu_percent = 0.0;

        let refs = vec![&r1];
        let percentiles = calculate_percentiles(&refs);

        // 0.0 is filtered, so cpu should be None
        assert!(percentiles.cpu.is_none());
    }

    #[test]
    fn test_calculate_percentiles_cpu_mixed_zero_and_nonzero() {
        // Test: Mix of 0.0 and valid CPU values — only valid values used
        let mut r1 = create_test_result("framework1", "pdf", OcrStatus::NotUsed, 100, 1_000_000.0, 10_000_000);
        r1.metrics.avg_cpu_percent = 0.0; // filtered out

        let mut r2 = create_test_result("framework1", "pdf", OcrStatus::NotUsed, 150, 1_000_000.0, 10_000_000);
        r2.metrics.avg_cpu_percent = 40.0;

        let mut r3 = create_test_result("framework1", "pdf", OcrStatus::NotUsed, 200, 1_000_000.0, 10_000_000);
        r3.metrics.avg_cpu_percent = 60.0;

        let refs = vec![&r1, &r2, &r3];
        let percentiles = calculate_percentiles(&refs);

        assert!(percentiles.cpu.is_some());
        let cpu = percentiles.cpu.as_ref().unwrap();
        // Only 40 and 60 → median = 50
        assert_eq!(cpu.p50, 50.0);
    }

    #[test]
    fn test_calculate_percentiles_cpu_failed_results_excluded() {
        // Test: Failed results' CPU values are excluded
        let mut r1 = create_test_result("framework1", "pdf", OcrStatus::NotUsed, 100, 1_000_000.0, 10_000_000);
        r1.metrics.avg_cpu_percent = 30.0;

        let mut r2_failed = create_test_result("framework1", "pdf", OcrStatus::NotUsed, 0, 0.0, 0);
        r2_failed.success = false;
        r2_failed.error_message = Some("Failed".to_string());
        r2_failed.metrics.avg_cpu_percent = 90.0; // Should be ignored

        let refs = vec![&r1, &r2_failed];
        let percentiles = calculate_percentiles(&refs);

        assert!(percentiles.cpu.is_some());
        let cpu = percentiles.cpu.as_ref().unwrap();
        assert_eq!(cpu.p50, 30.0); // Only successful result's CPU used
    }

    #[test]
    fn test_comparison_cpu_ranking() {
        // Test: CPU ranking in comparison data — lower CPU = rank 1
        let mut r1 = create_test_result("fast-framework", "pdf", OcrStatus::NotUsed, 50, 2_000_000.0, 5_000_000);
        r1.metrics.avg_cpu_percent = 80.0; // high CPU

        let mut r2 = create_test_result("slow-framework", "pdf", OcrStatus::NotUsed, 200, 500_000.0, 20_000_000);
        r2.metrics.avg_cpu_percent = 20.0; // low CPU

        let results = vec![r1, r2];
        let aggregated = aggregate_new_format(&results);

        assert!(!aggregated.comparison.cpu_ranking.is_empty());
        // slow-framework has lower CPU, should be rank 1
        assert_eq!(
            aggregated.comparison.cpu_ranking[0].framework_mode,
            "slow-framework:single"
        );
        assert_eq!(aggregated.comparison.cpu_ranking[0].rank, 1);
        assert_eq!(
            aggregated.comparison.cpu_ranking[1].framework_mode,
            "fast-framework:single"
        );
        assert_eq!(aggregated.comparison.cpu_ranking[1].rank, 2);
    }

    #[test]
    fn test_deltas_include_cpu() {
        // Test: Deltas vs baseline include CPU delta fields
        let mut r1 = create_test_result("baseline-fw", "pdf", OcrStatus::NotUsed, 50, 2_000_000.0, 5_000_000);
        r1.metrics.avg_cpu_percent = 30.0;

        let mut r2 = create_test_result("other-fw", "pdf", OcrStatus::NotUsed, 200, 500_000.0, 20_000_000);
        r2.metrics.avg_cpu_percent = 60.0;

        let results = vec![r1, r2];
        let aggregated = aggregate_new_format(&results);

        // baseline-fw is fastest (50ms), so other-fw has deltas vs it
        let delta = aggregated.comparison.deltas_vs_baseline.get("other-fw:single").unwrap();
        assert_eq!(delta.cpu_delta_pp, 30.0); // 60 - 30 = 30 percentage points
        assert!((delta.cpu_delta_percent - 100.0).abs() < 0.1); // (60-30)/30 * 100 = 100%
    }
}

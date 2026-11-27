//! Aggregation module for benchmark results (v2.13.0 output schema).
//!
//! Groups [`BenchmarkResult`] records by framework-and-mode, output format, file type, and
//! OCR usage (yes/no), then computes percentile-based statistics for each
//! group. The output schema (`schema_version: "2.13.0"`) surfaces TF1 and SF1 separately
//! with per-fixture rows preserved and split rankings by output format, plus a cohort-wide
//! [`FailureSummary`] rolling up framework-fault vs infrastructure failures (v2.9.0+), plus
//! paired layout-vs-baseline ([`LayoutImpact`]) and xberg-vs-competitor ([`HeadToHead`])
//! deltas computed from the per-fixture rows (v2.12.0+). Zero-overlap failures count as quality
//! results (TF1 = 0) in `quality_including_failures` and in the paired sections (v2.13.0+).
//!
//! # Percentile methodology
//!
//! All percentiles use the **R-7 interpolation** method (the default in R and
//! NumPy) via [`crate::stats::percentile_r7`]. Up to three percentiles are reported
//! per metric: **p50** (median), **p95**, and **p99**. Real cohorts are often 4-8
//! fixtures, at which R-7 interpolation for p95/p99 is effectively just reading off the
//! maximum sample rather than a genuine tail percentile — so `p95`/`p99` are `None` when
//! `sample_count` is below the statistically supported minimum for that percentile (see
//! [`MIN_SAMPLES_FOR_P95`]/[`MIN_SAMPLES_FOR_P99`]), instead of fabricating a number that
//! looks precise but isn't (Defect S1). Every [`Percentiles`] group also reports
//! `sample_count` and `std_dev` so a reader can judge dispersion even when p95/p99 are
//! suppressed. Values that are `NaN` or `Inf` after interpolation are sanitized to `0.0` by
//! [`crate::stats::sanitize_f64`] so that downstream JSON consumers never encounter
//! non-finite floats.
//!
//! Failed results (non-zero `error_kind`) are excluded from percentile
//! calculations but still counted in `total_sample_count` to preserve the
//! `success_rate_percent` metric. The one exception is `quality_including_failures`, which also
//! counts zero-overlap failures at the quality they were scored with.
//!
//! # Output format support
//!
//! Plaintext-only frameworks must NEVER appear in SF1 rankings or quality metrics
//! that require layout information. Markdown frameworks appear in all rankings.
//!
//! # Aggregate key format
//!
//! Keys in `by_framework_mode` differ by framework family:
//!
//! - **xberg** (`xberg-*`): `{framework_name}:{mode}` — the output format is already
//!   encoded in the framework name (e.g. `xberg-markdown-baseline`), so repeating it in
//!   the key would be redundant.
//! - **competitors** (all other frameworks): `{framework}:{output_format}:{mode}` — format is
//!   not encoded in the name, so the key must carry it explicitly.
//!
//! # Module layout
//!
//! The schema types live in [`types`]; failure counting in [`failures`]; per-group percentile
//! calculation in [`percentiles`]; and cross-framework ranking in [`ranking`]. This file holds
//! the top-level orchestration ([`aggregate_new_format`]) and the grouping helpers it is built
//! from, plus the aggregate-key helpers shared across the other submodules.

mod failures;
mod paired;
mod percentiles;
mod ranking;
mod types;

#[cfg(test)]
mod tests;

use crate::types::{BenchmarkResult, DiskSizeInfo, OutputFormat};
use std::collections::HashMap;

// Internal re-exports: keep every item nameable at `crate::aggregate::…` exactly as it was
// before the split, so no other file in the crate needs to change, and so `use super::*;` in
// the (also-relocated) test modules keeps resolving every name it always has.
pub use types::*;

use failures::build_failure_summary;

use percentiles::{aggregate_cold_starts, calculate_percentiles, parse_aggregate_key};

pub(crate) use paired::{baseline_framework_for, layout_applies_to};
pub use ranking::apply_pinned_cohort_comparison;
use ranking::build_comparison;
pub(crate) use ranking::{comparison_for_cohort, resolve_shared_corpus_file_types};

/// Main aggregation function for new format
///
/// Groups results by:
/// 1. Framework and mode (extracted from framework name)
/// 2. File type (extension)
/// 3. OCR usage (yes/no)
///
/// Calculates p50/p95/p99 percentiles for each group.
pub fn aggregate_new_format(results: &[BenchmarkResult]) -> NewConsolidatedResults {
    if results.is_empty() {
        return empty_consolidated_results();
    }

    let grouped = group_results_by_framework_mode_and_file_type(results);
    let aggregated_by_framework_mode = build_framework_mode_aggregations(grouped.by_framework_mode_format);
    let per_fixture_results = build_per_fixture_results(results);
    let framework_count = count_logical_frameworks(results);
    let competitive_aggregations: HashMap<String, FrameworkModeAggregation> = aggregated_by_framework_mode
        .iter()
        .filter(|(_, aggregation)| aggregation.timing_regime == crate::types::TimingRegime::ColdProcess)
        .map(|(key, aggregation)| (key.clone(), aggregation.clone()))
        .collect();

    let metadata = ConsolidationMetadata {
        total_results: results.len(),
        framework_count,
        file_type_count: grouped.file_types.len(),
        shared_corpus_markdown: resolve_shared_corpus_file_types(&competitive_aggregations, OutputFormat::Markdown),
        shared_corpus_plaintext: resolve_shared_corpus_file_types(&competitive_aggregations, OutputFormat::Plaintext),
        timestamp: chrono::Utc::now().to_rfc3339(),
        disk_size_conflicts: grouped.disk_size_conflicts,
    };

    let comparison = build_comparison(&aggregated_by_framework_mode, None);
    let failure_summary = build_failure_summary(results);
    let format_support = build_format_support_matrix(results, &grouped.file_types);
    let layout_impact = paired::build_layout_impact(&per_fixture_results);
    let head_to_head = paired::build_head_to_head(&per_fixture_results);

    NewConsolidatedResults {
        schema_version: SCHEMA_VERSION.to_string(),
        by_framework_mode: aggregated_by_framework_mode,
        disk_sizes: grouped.disk_sizes,
        comparison,
        per_fixture_results,
        metadata,
        run_provenance: Vec::new(),
        failure_summary,
        format_support,
        layout_impact,
        head_to_head,
    }
}

/// The empty-input case of [`aggregate_new_format`], split out so the main function reads as a
/// single non-degenerate path.
fn empty_consolidated_results() -> NewConsolidatedResults {
    NewConsolidatedResults {
        schema_version: SCHEMA_VERSION.to_string(),
        by_framework_mode: HashMap::new(),
        disk_sizes: HashMap::new(),
        comparison: ComparisonData {
            throughput_ranking: Vec::new(),
            memory_ranking: Vec::new(),
            quality_ranking_markdown: Vec::new(),
            quality_ranking_plaintext: Vec::new(),
            pdf_quality_ranking_markdown: Vec::new(),
            pdf_quality_ranking_plaintext: Vec::new(),
            pdf_tf1_ranking_markdown: Vec::new(),
            pdf_tf1_ranking_plaintext: Vec::new(),
            pdf_sf1_ranking_markdown: Vec::new(),
            pdf_reading_order_ranking_markdown: Vec::new(),
            pdf_reading_order_ranking_plaintext: Vec::new(),
            pages_per_sec_ranking: Vec::new(),
            cpu_seconds_ranking: Vec::new(),
            deltas_vs_baseline: HashMap::new(),
            pareto_frontier: Vec::new(),
            unranked_frameworks: Vec::new(),
        },
        per_fixture_results: Vec::new(),
        metadata: ConsolidationMetadata {
            total_results: 0,
            framework_count: 0,
            file_type_count: 0,
            shared_corpus_markdown: Vec::new(),
            shared_corpus_plaintext: Vec::new(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            disk_size_conflicts: Vec::new(),
        },
        run_provenance: Vec::new(),
        failure_summary: FailureSummary::default(),
        format_support: FormatSupportMatrix::default(),
        layout_impact: Vec::new(),
        head_to_head: Vec::new(),
    }
}

/// Output of the first grouping pass over raw results: every result bucketed by aggregate key and
/// file type, plus the disk-size bookkeeping and observed file-type set gathered in the same pass.
/// See [`group_results_by_framework_mode_and_file_type`].
struct GroupedResults<'a> {
    by_framework_mode_format: HashMap<String, HashMap<String, Vec<&'a BenchmarkResult>>>,
    disk_sizes: HashMap<String, DiskSizeInfo>,
    disk_size_conflicts: Vec<String>,
    file_types: std::collections::HashSet<String>,
}

/// First pass of [`aggregate_new_format`]: bucket every result by `(aggregate key, file type)`
/// and collect the disk-size and observed-file-type bookkeeping that only needs one pass.
fn group_results_by_framework_mode_and_file_type(results: &[BenchmarkResult]) -> GroupedResults<'_> {
    let mut by_framework_mode_format: HashMap<String, HashMap<String, Vec<&BenchmarkResult>>> = HashMap::new();
    let mut disk_sizes: HashMap<String, DiskSizeInfo> = HashMap::new();
    let mut disk_size_conflicts: Vec<String> = Vec::new();
    let mut file_types = std::collections::HashSet::new();

    for result in results {
        let (framework, mode) = extract_framework_and_mode(&result.framework);
        let key = make_aggregate_key(framework, result.output_format, mode);

        by_framework_mode_format
            .entry(key)
            .or_default()
            .entry(result.file_extension.clone())
            .or_default()
            .push(result);

        file_types.insert(result.file_extension.clone());

        if let Some(disk_size) = &result.framework_capabilities.installation_size {
            if let Some(existing) = disk_sizes.get(framework)
                && (existing.size_bytes != disk_size.size_bytes || existing.method != disk_size.method)
            {
                disk_size_conflicts.push(format!(
                    "{framework}: installation_size conflict ({} bytes via {:?} vs {} bytes via {:?}); \
                     disk_sizes keeps the last-seen value",
                    existing.size_bytes, existing.method, disk_size.size_bytes, disk_size.method
                ));
            }
            disk_sizes.insert(framework.to_string(), disk_size.clone());
        }
    }

    GroupedResults {
        by_framework_mode_format,
        disk_sizes,
        disk_size_conflicts,
        file_types,
    }
}

/// Second pass of [`aggregate_new_format`]: turn the grouped buckets from
/// [`group_results_by_framework_mode_and_file_type`] into one [`FrameworkModeAggregation`] per
/// aggregate key.
fn build_framework_mode_aggregations(
    by_framework_mode_format: HashMap<String, HashMap<String, Vec<&BenchmarkResult>>>,
) -> HashMap<String, FrameworkModeAggregation> {
    let mut aggregated_by_framework_mode = HashMap::new();

    for (framework_mode_format_key, file_type_results) in by_framework_mode_format {
        let output_format = file_type_results
            .values()
            .flatten()
            .next()
            .map(|r| r.output_format)
            .unwrap_or(OutputFormat::Markdown);
        let mut timing_regimes = file_type_results
            .values()
            .flatten()
            .map(|result| result.framework_capabilities.timing_regime);
        let first_timing_regime = timing_regimes.next().unwrap_or_default();
        let timing_regime = if timing_regimes.all(|regime| regime == first_timing_regime) {
            first_timing_regime
        } else {
            crate::types::TimingRegime::Mixed
        };

        let (framework, mode) = parse_aggregate_key(&framework_mode_format_key);

        let all_results: Vec<&BenchmarkResult> = file_type_results.values().flat_map(|v| v.iter().copied()).collect();
        let cold_start = aggregate_cold_starts(&all_results);
        let overall_performance = Some(calculate_percentiles(&all_results));

        let mut by_file_type = HashMap::new();
        for (file_type, results_for_type) in file_type_results {
            let aggregation = aggregate_by_ocr_status(&results_for_type);
            by_file_type.insert(
                file_type.clone(),
                FileTypeAggregation {
                    file_type: file_type.clone(),
                    no_ocr: aggregation.0,
                    with_ocr: aggregation.1,
                    unknown_ocr: aggregate_unknown_ocr_status(&results_for_type),
                },
            );
        }

        aggregated_by_framework_mode.insert(
            framework_mode_format_key.clone(),
            FrameworkModeAggregation {
                framework: framework.to_string(),
                output_format,
                mode: mode.to_string(),
                timing_regime,
                cold_start,
                overall_performance,
                by_file_type,
            },
        );
    }

    aggregated_by_framework_mode
}

/// Count *logical* frameworks: all xberg pipelines (xberg-markdown-baseline,
/// xberg-plaintext-layout, …) are variants of the single "xberg" framework, so collapse
/// them to one before counting. Otherwise framework_count over-reports by the number of
/// xberg name-variants present (e.g. 11 instead of 8). ~keep
fn count_logical_frameworks(results: &[BenchmarkResult]) -> usize {
    results
        .iter()
        .map(|r| {
            let name = extract_framework_and_mode(&r.framework).0;
            if name.starts_with("xberg") { "xberg" } else { name }
        })
        .collect::<std::collections::HashSet<_>>()
        .len()
}

/// Build the capability-aware format-support matrix (see [`FormatSupportMatrix`]).
///
/// For every logical framework present in the run (xberg pipeline variants collapse to a single
/// `xberg`) and every file type observed in the consolidated results, records the file types the
/// framework declares no support for. xberg supports the full corpus by design and is never listed
/// as unsupported; every other framework is checked against its declared capability table.
fn build_format_support_matrix(
    results: &[BenchmarkResult],
    file_types: &std::collections::HashSet<String>,
) -> FormatSupportMatrix {
    let mut sorted_file_types: Vec<String> = file_types.iter().cloned().collect();
    sorted_file_types.sort();

    let mut frameworks: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for result in results {
        let name = extract_framework_and_mode(&result.framework).0;
        let logical = if name.starts_with("xberg") { "xberg" } else { name };
        frameworks.insert(logical.to_string());
    }

    let mut unsupported = std::collections::BTreeMap::new();
    for framework in &frameworks {
        // xberg is the subject under test and supports every corpus format, so it is never
        // "unsupported"; only competitors are checked against their declared capability table.
        if framework == "xberg" {
            continue;
        }
        let supported = crate::adapters::external::declared_supported_formats(framework);
        let missing: Vec<String> = sorted_file_types
            .iter()
            .filter(|file_type| !supported.iter().any(|s| s == *file_type))
            .cloned()
            .collect();
        if !missing.is_empty() {
            unsupported.insert(framework.clone(), missing);
        }
    }

    FormatSupportMatrix {
        file_types: sorted_file_types,
        unsupported,
    }
}

/// Build per-fixture result rows from raw benchmark results
///
/// Extracts one row per (framework, output_format, execution_mode, fixture_id, ocr) group.
/// Fixture ID is derived from the file path (filename without extension).
fn build_per_fixture_results(results: &[BenchmarkResult]) -> Vec<PerFixtureRow> {
    let mut fixture_rows = Vec::new();

    for result in results {
        let (framework, mode) = extract_framework_and_mode(&result.framework);
        let fixture_id = result
            .file_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("unknown")
            .to_string();

        let ocr = match result.ocr_status {
            crate::types::OcrStatus::Used => Some(true),
            crate::types::OcrStatus::NotUsed => Some(false),
            crate::types::OcrStatus::Unknown => None,
        };
        let error_kind = if !result.success {
            Some(format!("{:?}", result.error_kind))
        } else {
            None
        };

        let (f1_text, f1_layout, f1_numeric, quality_score, correct) = if let Some(q) = &result.quality {
            (
                Some(q.f1_score_text),
                q.f1_score_layout,
                Some(q.f1_score_numeric),
                Some(q.quality_score),
                Some(q.correct),
            )
        } else {
            (None, None, None, None, None)
        };

        fixture_rows.push(PerFixtureRow {
            framework: framework.to_string(),
            output_format: result.output_format,
            execution_mode: mode.to_string(),
            timing_regime: result.framework_capabilities.timing_regime,
            ocr,
            fixture_id,
            file_type: result.file_extension.clone(),
            duration_ms: result.duration.as_secs_f64() * 1000.0,
            peak_memory_mb: result.metrics.peak_memory_bytes as f64 / 1_000_000.0,
            f1_text,
            f1_layout,
            f1_numeric,
            quality_score,
            correct,
            success: result.success,
            error_kind,
            file_size: result.file_size,
            throughput_bytes_per_sec: result.metrics.throughput_bytes_per_sec,
            avg_cpu_percent: result.metrics.avg_cpu_percent,
            cpu_seconds: result.metrics.cpu_seconds,
            baseline_memory_bytes: result.metrics.baseline_memory_bytes,
            peak_memory_delta_bytes: result.metrics.peak_memory_delta_bytes,
            memory_method: result.metrics.memory_method,
            self_reported_peak_memory_bytes: result.metrics.self_reported_peak_memory_bytes,
            p50_memory_bytes: result.metrics.p50_memory_bytes,
            p95_memory_bytes: result.metrics.p95_memory_bytes,
            p99_memory_bytes: result.metrics.p99_memory_bytes,
            extraction_duration_ms: result.extraction_duration.map(|d| d.as_secs_f64() * 1000.0),
            subprocess_overhead_ms: result.subprocess_overhead.map(|d| d.as_secs_f64() * 1000.0),
            cold_start_duration_ms: result.cold_start_duration.map(|d| d.as_secs_f64() * 1000.0),
            error_message: result.error_message.clone(),
            quality: result.quality.clone(),
            pdf_metadata: result.pdf_metadata.clone(),
            framework_capabilities: result.framework_capabilities.clone(),
            system_load: result.system_load,
            iterations: result.iterations.clone(),
            statistics: result.statistics.clone(),
        });
    }

    fixture_rows
}

/// Aggregate results by OCR status
///
/// Returns (no_ocr, with_ocr) tuple of PerformancePercentiles
fn aggregate_by_ocr_status(
    results: &[&BenchmarkResult],
) -> (Option<PerformancePercentiles>, Option<PerformancePercentiles>) {
    use crate::types::OcrStatus;

    // Unknown is deliberately excluded from both cohorts. In particular, an
    // unreported PDF OCR status must never be presented as a no-OCR result. ~keep
    let no_ocr: Vec<&BenchmarkResult> = results
        .iter()
        .filter(|result| result.ocr_status == OcrStatus::NotUsed)
        .copied()
        .collect();

    let with_ocr: Vec<&BenchmarkResult> = results
        .iter()
        .filter(|result| result.ocr_status == OcrStatus::Used)
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

/// Aggregate results whose OCR status is `Unknown`, or `None` when there are none. These stay out
/// of the no-OCR and with-OCR cohorts but are ranked, so unreported-status failures still count
/// against a framework. ~keep
fn aggregate_unknown_ocr_status(results: &[&BenchmarkResult]) -> Option<PerformancePercentiles> {
    let unknown: Vec<&BenchmarkResult> = results
        .iter()
        .filter(|result| result.ocr_status == crate::types::OcrStatus::Unknown)
        .copied()
        .collect();
    (!unknown.is_empty()).then(|| calculate_percentiles(&unknown))
}

/// Extract framework name and mode from a raw framework string.
///
/// Modes: `-batch` suffix → `"batch"`, anything else → `"single"`.
/// Legacy `-sync`/`-async` suffixes (no longer emitted by current adapters, but present in
/// historical result files) are stripped from the base name to preserve backward compatibility.
///
/// Returns `(framework_name, mode)` where `mode` is `"batch"` or `"single"`.
pub(crate) fn extract_framework_and_mode(framework_name: &str) -> (&str, &str) {
    if let Some(base) = framework_name.strip_suffix("-batch") {
        let normalized = base
            .strip_suffix("-sync")
            .or_else(|| base.strip_suffix("-async"))
            .unwrap_or(base);
        (normalized, "batch")
    } else {
        let normalized = framework_name
            .strip_suffix("-sync")
            .or_else(|| framework_name.strip_suffix("-async"))
            .unwrap_or(framework_name);
        (normalized, "single")
    }
}

/// Build the `by_framework_mode` map key for a result.
///
/// - `xberg-*` frameworks already encode the output format in their name, so the key is
///   `"{framework}:{mode}"` — no redundant format component.
/// - All other (competitor) frameworks use `"{framework}:{output_format}:{mode}"`.
pub(crate) fn make_aggregate_key(framework: &str, output_format: OutputFormat, mode: &str) -> String {
    if framework.starts_with("xberg-") {
        format!("{framework}:{mode}")
    } else {
        format!("{framework}:{output_format}:{mode}")
    }
}

//! Per-group percentile calculation: duration/throughput/memory/quality/pages-per-sec/cpu-seconds
//! percentiles, cold-start and system-load aggregation, and the batch-size/aggregate-key helpers
//! they share.

use super::failures::is_framework_fault_failure;
use super::types::{
    DurationPercentiles, MIN_SAMPLES_FOR_P95, MIN_SAMPLES_FOR_P99, Percentiles, PerformancePercentiles,
    QualityPercentiles, SystemLoadPercentiles,
};
use crate::stats::{percentile_r7, sanitize_f64};
use crate::types::{
    BenchmarkResult, ErrorKind, expand_performance_iterations, has_quality_result, successful_performance_samples,
};
use std::collections::HashMap;

/// Build a `Percentiles` group from a slice of values (need not be pre-sorted).
///
/// Suppresses `p95`/`p99` when `values.len()` is below the statistically supported minimum for
/// that percentile (see [`MIN_SAMPLES_FOR_P95`]/[`MIN_SAMPLES_FOR_P99`]) instead of fabricating an
/// interpolated number that is really just the maximum (or close to it). Always reports
/// `sample_count` and `std_dev` so a reader can judge the distribution regardless. (Defect S1)
/// ~keep
pub(super) fn build_percentiles(values: &[f64]) -> Percentiles {
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let sample_count = sorted.len();
    let (_, _, std_dev) = crate::stats::calculate_variance(&sorted);

    Percentiles {
        p50: sanitize_f64(percentile_r7(&sorted, 0.50)),
        p95: (sample_count >= MIN_SAMPLES_FOR_P95).then(|| sanitize_f64(percentile_r7(&sorted, 0.95))),
        p99: (sample_count >= MIN_SAMPLES_FOR_P99).then(|| sanitize_f64(percentile_r7(&sorted, 0.99))),
        sample_count,
        std_dev: sanitize_f64(std_dev),
    }
}

/// Duration/throughput/memory/extraction-duration/cpu-seconds/pages-per-sec percentiles derived
/// from a group's performance samples. See [`build_performance_metrics`].
struct PerformanceMetricsBundle {
    duration: Percentiles,
    throughput: Percentiles,
    memory: Percentiles,
    extraction_duration: Option<Percentiles>,
    cpu_seconds: Percentiles,
    pages_per_sec: Option<Percentiles>,
    throughput_excluded_sample_count: usize,
}

/// Raw (unsorted) value vectors collected from one group's performance samples, one per metric.
/// See [`collect_performance_value_vectors`].
struct PerformanceValueVectors {
    durations: Vec<f64>,
    throughputs: Vec<f64>,
    throughput_excluded_sample_count: usize,
    memories: Vec<f64>,
    extraction_durations: Vec<f64>,
    cpu_seconds_values: Vec<f64>,
    pages_per_sec_values: Vec<f64>,
}

/// First phase of [`build_performance_metrics`]: collect each metric's raw values, applying the
/// same non-finite/non-positive exclusion rules the original inline code used. `build_percentiles`
/// sorts internally, so these are returned unsorted.
fn collect_performance_value_vectors(
    results: &[&BenchmarkResult],
    successful: &[&BenchmarkResult],
    performance_samples: &[&BenchmarkResult],
) -> PerformanceValueVectors {
    let durations: Vec<f64> = performance_samples
        .iter()
        .map(|r| r.duration.as_secs_f64() * 1000.0)
        .filter(|&v| !v.is_nan() && v.is_finite())
        .collect();

    let throughputs: Vec<f64> = performance_samples
        .iter()
        .map(|r| r.metrics.throughput_bytes_per_sec / 1_000_000.0)
        .filter(|&v| v > 0.0 && v.is_finite())
        .collect();
    // Every performance sample not represented in `throughputs` above was excluded because its
    // throughput was non-positive or non-finite; surface the count so a 0-valued percentile
    // group can be told apart from one with no samples at all. ~keep
    let throughput_excluded_sample_count = performance_samples
        .iter()
        .filter(|r| {
            let v = r.metrics.throughput_bytes_per_sec / 1_000_000.0;
            !(v > 0.0 && v.is_finite())
        })
        .count();

    let memories: Vec<f64> = performance_samples
        .iter()
        .filter(|r| {
            r.framework_capabilities.resource_measurement_scope
                == crate::types::ResourceMeasurementScope::IsolatedProcess
        })
        .map(|r| r.metrics.peak_memory_bytes as f64 / 1_000_000.0)
        .filter(|&v| !v.is_nan() && v.is_finite())
        .collect();

    let extraction_durations: Vec<f64> = successful
        .iter()
        .filter_map(|r| r.extraction_duration.map(|d| d.as_secs_f64() * 1000.0))
        .filter(|&v| !v.is_nan() && v.is_finite())
        .collect();

    // `integrate_cpu_core_seconds` (monitoring.rs) returns exactly `0.0` when a process ran with
    // fewer than 2 resource-sampler ticks — "below measurement resolution", not "measured zero
    // CPU time" (zero core-seconds of CPU use is physically impossible for a process that ran to
    // completion). Excluding that floor value here keeps it out of every percentile derived from
    // `cpu_seconds_values`, not just the cross-framework ranking (see `build_comparison`'s
    // `unranked_frameworks` bookkeeping for the ranking-level exclusion). (Defect S2) ~keep
    let cpu_seconds_values: Vec<f64> = performance_samples
        .iter()
        .filter(|r| {
            r.framework_capabilities.resource_measurement_scope
                == crate::types::ResourceMeasurementScope::IsolatedProcess
        })
        .map(|r| r.metrics.cpu_seconds)
        .filter(|&v| !v.is_nan() && v.is_finite() && v > 0.0)
        .collect();

    let pages_per_sec_values = collect_pages_per_second(results);

    PerformanceValueVectors {
        durations,
        throughputs,
        throughput_excluded_sample_count,
        memories,
        extraction_durations,
        cpu_seconds_values,
        pages_per_sec_values,
    }
}

/// Second phase: turn [`collect_performance_value_vectors`]'s raw vectors into their
/// `Percentiles`. Split out of [`calculate_percentiles`] as its own phase so that function stays
/// readable; every filter/collection here preserves the exact order and exclusion rules the
/// original inline code had.
fn build_performance_metrics(
    results: &[&BenchmarkResult],
    successful: &[&BenchmarkResult],
    performance_samples: &[&BenchmarkResult],
) -> PerformanceMetricsBundle {
    let PerformanceValueVectors {
        durations,
        throughputs,
        throughput_excluded_sample_count,
        memories,
        extraction_durations,
        cpu_seconds_values,
        pages_per_sec_values,
    } = collect_performance_value_vectors(results, successful, performance_samples);

    let duration = build_percentiles(&durations);
    let throughput = build_percentiles(&throughputs);
    let memory = build_percentiles(&memories);

    let extraction_duration = if !extraction_durations.is_empty() {
        Some(build_percentiles(&extraction_durations))
    } else {
        None
    };

    let cpu_seconds = build_percentiles(&cpu_seconds_values);

    let pages_per_sec = if !pages_per_sec_values.is_empty() {
        Some(build_percentiles(&pages_per_sec_values))
    } else {
        None
    };

    PerformanceMetricsBundle {
        duration,
        throughput,
        memory,
        extraction_duration,
        cpu_seconds,
        pages_per_sec,
        throughput_excluded_sample_count,
    }
}

/// Error-kind counts and per-message occurrence details for a group of results. See
/// [`count_error_kinds_and_details`].
struct ErrorKindCounts {
    framework_errors: usize,
    harness_errors: usize,
    config_setup_errors: usize,
    timeouts: usize,
    empty_content: usize,
    zero_overlap: usize,
    layout_failures: usize,
    error_details: HashMap<String, usize>,
}

/// Count each `ErrorKind` and collect unique failure messages with their occurrence counts.
fn count_error_kinds_and_details(results: &[&BenchmarkResult]) -> ErrorKindCounts {
    let framework_errors = results
        .iter()
        .filter(|r| r.error_kind == ErrorKind::FrameworkError)
        .count();
    let harness_errors = results
        .iter()
        .filter(|r| r.error_kind == ErrorKind::HarnessError)
        .count();
    let config_setup_errors = results
        .iter()
        .filter(|r| r.error_kind == ErrorKind::ConfigSetupError)
        .count();
    let timeouts = results.iter().filter(|r| r.error_kind == ErrorKind::Timeout).count();
    let empty_content = results
        .iter()
        .filter(|r| r.error_kind == ErrorKind::EmptyContent)
        .count();
    let zero_overlap = results
        .iter()
        .filter(|r| r.error_kind == ErrorKind::ZeroOverlap)
        .count();
    let layout_failures = results
        .iter()
        .filter(|r| r.error_kind == ErrorKind::LayoutFailed)
        .count();

    let mut error_details: HashMap<String, usize> = HashMap::new();
    for result in results.iter().filter(|r| !r.success) {
        if let Some(msg) = &result.error_message {
            *error_details.entry(msg.clone()).or_insert(0) += 1;
        }
    }

    ErrorKindCounts {
        framework_errors,
        harness_errors,
        config_setup_errors,
        timeouts,
        empty_content,
        zero_overlap,
        layout_failures,
        error_details,
    }
}

/// Build the quality percentile block from `results`, or `None` when none of them carry a quality
/// score.
fn build_quality_percentiles(results: &[&BenchmarkResult]) -> Option<QualityPercentiles> {
    let mut f1_texts: Vec<f64> = results
        .iter()
        .filter_map(|r| r.quality.as_ref().map(|q| q.f1_score_text))
        .filter(|v| !v.is_nan() && v.is_finite())
        .collect();
    let mut f1_numerics: Vec<f64> = results
        .iter()
        .filter_map(|r| r.quality.as_ref().map(|q| q.f1_score_numeric))
        .filter(|v| !v.is_nan() && v.is_finite())
        .collect();
    let mut f1_layouts: Vec<f64> = results
        .iter()
        .filter_map(|r| r.quality.as_ref().and_then(|q| q.f1_score_layout))
        .filter(|v| !v.is_nan() && v.is_finite())
        .collect();
    let mut quality_scores: Vec<f64> = results
        .iter()
        .filter_map(|r| r.quality.as_ref().map(|q| q.quality_score))
        .filter(|v| !v.is_nan() && v.is_finite())
        .collect();

    if quality_scores.is_empty() {
        return None;
    }

    let mut reading_orders: Vec<f64> = results
        .iter()
        .filter_map(|r| r.quality.as_ref().and_then(|q| q.reading_order_score))
        .filter(|v| v.is_finite())
        .collect();
    reading_orders.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let reading_order_p50 = (!reading_orders.is_empty()).then(|| sanitize_f64(percentile_r7(&reading_orders, 0.50)));

    f1_texts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    f1_numerics.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    f1_layouts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    quality_scores.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let f1_layout_p50 = if !f1_layouts.is_empty() {
        Some(sanitize_f64(percentile_r7(&f1_layouts, 0.50)))
    } else {
        None
    };
    let f1_layout_p95 = if !f1_layouts.is_empty() {
        Some(sanitize_f64(percentile_r7(&f1_layouts, 0.95)))
    } else {
        None
    };
    let f1_layout_p99 = if !f1_layouts.is_empty() {
        Some(sanitize_f64(percentile_r7(&f1_layouts, 0.99)))
    } else {
        None
    };

    Some(QualityPercentiles {
        f1_text_p50: sanitize_f64(percentile_r7(&f1_texts, 0.50)),
        f1_text_p95: sanitize_f64(percentile_r7(&f1_texts, 0.95)),
        f1_text_p99: sanitize_f64(percentile_r7(&f1_texts, 0.99)),
        f1_numeric_p50: sanitize_f64(percentile_r7(&f1_numerics, 0.50)),
        f1_numeric_p95: sanitize_f64(percentile_r7(&f1_numerics, 0.95)),
        f1_numeric_p99: sanitize_f64(percentile_r7(&f1_numerics, 0.99)),
        f1_layout_p50,
        f1_layout_p95,
        f1_layout_p99,
        quality_score_p50: sanitize_f64(percentile_r7(&quality_scores, 0.50)),
        quality_score_p95: sanitize_f64(percentile_r7(&quality_scores, 0.95)),
        quality_score_p99: sanitize_f64(percentile_r7(&quality_scores, 0.99)),
        reading_order_p50,
        reading_order_sample_count: reading_orders.len(),
    })
}

/// Calculate percentiles for a group of results
///
/// Performance and raw quality percentiles use only successful samples. Quality rankings apply
/// accountable success coverage separately, avoiding a nonlinear double penalty from both zero
/// injection and coverage adjustment. `quality_including_failures` adds zero-overlap failures at
/// their scored quality for readers that compare quality without a coverage adjustment. The
/// success-rate denominator is successes plus framework-fault failures; harness/config-setup
/// failures are excluded. ~keep
pub(super) fn calculate_percentiles(results: &[&BenchmarkResult]) -> PerformancePercentiles {
    let successful: Vec<&BenchmarkResult> = results.iter().filter(|r| r.success).copied().collect();
    let framework_fault_failures = results.iter().filter(|r| is_framework_fault_failure(r)).count();
    let expanded_results = expand_performance_iterations(results);
    let expanded_refs: Vec<&BenchmarkResult> = expanded_results.iter().collect();
    let expanded_successful: Vec<&BenchmarkResult> =
        expanded_refs.iter().filter(|result| result.success).copied().collect();
    let performance_samples = successful_performance_samples(expanded_refs.iter().copied());

    let performance_metrics = build_performance_metrics(&expanded_refs, &expanded_successful, &performance_samples);

    // Real per-invocation batch size, derived from actual batch membership
    // (`framework_capabilities.batch_sample_id`) rather than the coarse
    // `total_sample_count / performance_sample_count` ratio: that ratio is only correct when
    // every eligible row is its own batch of one, and becomes a fiction whenever some rows are
    // excluded from timing eligibility (see `is_timing_eligible`) for a reason unrelated to batch
    // grouping — e.g. 10 single-file results with only 5 timing-eligible would report a
    // fictitious batch_size of 2. (Defect S3) ~keep
    let batch_size = compute_batch_size(&expanded_refs, &performance_samples);

    let system_load = aggregate_system_load(results);

    // Denominator excludes harness/config-setup (infra) failures: those are our fault, so they must
    // not drag a framework's success rate down. Only successes and framework-fault failures are
    // "accountable" samples.
    let accountable_sample_count = successful.len() + framework_fault_failures;
    let success_rate_percent = if accountable_sample_count > 0 {
        (successful.len() as f64 / accountable_sample_count as f64) * 100.0
    } else {
        0.0
    };

    let error_counts = count_error_kinds_and_details(results);
    let quality = build_quality_percentiles(&successful);
    let quality_scored: Vec<&BenchmarkResult> = results.iter().copied().filter(|r| has_quality_result(r)).collect();
    let quality_including_failures = build_quality_percentiles(&quality_scored);

    PerformancePercentiles {
        successful_sample_count: successful.len(),
        performance_sample_count: performance_samples.len(),
        total_sample_count: results.len(),
        framework_errors: error_counts.framework_errors,
        harness_errors: error_counts.harness_errors,
        config_setup_errors: error_counts.config_setup_errors,
        timeouts: error_counts.timeouts,
        empty_content: error_counts.empty_content,
        zero_overlap: error_counts.zero_overlap,
        layout_failures: error_counts.layout_failures,
        error_details: error_counts.error_details,
        throughput: performance_metrics.throughput,
        memory: performance_metrics.memory,
        duration: performance_metrics.duration,
        success_rate_percent,
        extraction_duration: performance_metrics.extraction_duration,
        quality,
        quality_including_failures,
        pages_per_sec: performance_metrics.pages_per_sec,
        cpu_seconds: performance_metrics.cpu_seconds,
        batch_size,
        system_load,
        throughput_excluded_sample_count: performance_metrics.throughput_excluded_sample_count,
        memory_methods: count_memory_methods(&performance_samples),
    }
}

/// Counts the samples behind the `memory` percentiles by how their peak RSS was measured, using
/// the same isolated-process filter as the memory values themselves.
fn count_memory_methods(performance_samples: &[&BenchmarkResult]) -> std::collections::BTreeMap<String, usize> {
    let mut counts = std::collections::BTreeMap::new();
    for sample in performance_samples.iter().filter(|r| {
        r.framework_capabilities.resource_measurement_scope == crate::types::ResourceMeasurementScope::IsolatedProcess
    }) {
        *counts
            .entry(sample.metrics.memory_method.as_str().to_string())
            .or_insert(0) += 1;
    }
    counts
}

/// Real per-invocation batch size: the number of result rows sharing each performance sample's
/// `batch_sample_id`, rather than `total_sample_count / performance_sample_count`. That ratio
/// only equals true batch size when every row in `results` is timing-eligible (see
/// [`crate::types::is_timing_eligible`]); once some rows are excluded from timing eligibility for
/// a reason unrelated to batch grouping (e.g. `EmptyContent`), the ratio silently becomes the
/// inverse timing-eligibility rate instead of a document count (Defect S3).
///
/// Returns `None` when there are no performance samples to derive a batch size from. Returns
/// `Some(1)` when no performance sample carries a `batch_sample_id` (single-file mode). For
/// native batches, returns the modal (most frequent) document count across the performance
/// samples that do carry a `batch_sample_id` — every anchor row in a mixed batch/single-file
/// group should report the same count in practice, but the mode is a defined, deterministic
/// tie-break if it doesn't.
fn compute_batch_size(results: &[&BenchmarkResult], performance_samples: &[&BenchmarkResult]) -> Option<usize> {
    if performance_samples.is_empty() {
        return None;
    }

    let batch_counts: Vec<usize> = performance_samples
        .iter()
        .map(
            |sample| match sample.framework_capabilities.batch_sample_id.as_deref() {
                Some(batch_id) => results
                    .iter()
                    .filter(|r| r.framework_capabilities.batch_sample_id.as_deref() == Some(batch_id))
                    .count()
                    .max(1),
                None => 1,
            },
        )
        .collect();

    if batch_counts.iter().all(|&count| count == 1) {
        return Some(1);
    }

    let mut frequency_by_count: HashMap<usize, usize> = HashMap::new();
    for &count in &batch_counts {
        *frequency_by_count.entry(count).or_insert(0) += 1;
    }
    frequency_by_count
        .into_iter()
        .max_by_key(|(count, frequency)| (*frequency, *count))
        .map(|(count, _)| count)
}

/// Compute one pages/sec observation per performance sample (see [`successful_performance_samples`]).
///
/// For a single-file result, this is simply `page_count / duration`. For a native batch, every
/// document sharing one `batch_sample_id` has its own `PdfMetadata.page_count`, so the *total*
/// pages processed by that one batch invocation is summed across all its member rows before
/// dividing by the (shared) batch makespan — mirroring how batch-wide `throughput_bytes_per_sec`
/// is computed from summed bytes, not a single member row's byte count.
///
/// Rows without a detected PDF page count (non-PDF files, or a PDF the harness could not size)
/// are excluded rather than treated as zero. Eligibility is timing-based (see
/// [`crate::types::is_timing_eligible`]), not `success`-based, so a zero-overlap-flipped member
/// row still contributes its page count to its batch's total — the same Defect-A fix applied to
/// `duration`/`throughput`/`memory`/`cpu_seconds` above.
fn collect_pages_per_second(results: &[&BenchmarkResult]) -> Vec<f64> {
    let mut batch_pages: HashMap<&str, u64> = HashMap::new();
    for result in results.iter().filter(|r| crate::types::is_timing_eligible(r)) {
        if let (Some(batch_id), Some(page_count)) = (
            result.framework_capabilities.batch_sample_id.as_deref(),
            result.pdf_metadata.as_ref().and_then(|metadata| metadata.page_count),
        ) {
            *batch_pages.entry(batch_id).or_insert(0) += page_count as u64;
        }
    }

    successful_performance_samples(results.iter().copied())
        .into_iter()
        .filter_map(|sample| {
            let duration_secs = sample.duration.as_secs_f64();
            if duration_secs <= 0.0 {
                return None;
            }
            let pages = match sample.framework_capabilities.batch_sample_id.as_deref() {
                Some(batch_id) => *batch_pages.get(batch_id)?,
                None => sample.pdf_metadata.as_ref().and_then(|metadata| metadata.page_count)? as u64,
            };
            if pages == 0 {
                return None;
            }
            Some(pages as f64 / duration_secs)
        })
        .collect()
}

/// Aggregate the `SystemLoad` snapshots carried by a group of results into a contention
/// qualifier. Returns `None` when no result in the group recorded a snapshot.
fn aggregate_system_load(results: &[&BenchmarkResult]) -> Option<SystemLoadPercentiles> {
    let mut load_per_core: Vec<f64> = results
        .iter()
        .filter_map(|r| r.system_load.as_ref())
        .map(|load| load.load_per_core())
        .filter(|v| !v.is_nan() && v.is_finite())
        .collect();

    if load_per_core.is_empty() {
        return None;
    }

    let contended_sample_count = results
        .iter()
        .filter_map(|r| r.system_load.as_ref())
        .filter(|load| load.is_contended())
        .count();

    load_per_core.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    Some(SystemLoadPercentiles {
        load_per_core_p50: sanitize_f64(percentile_r7(&load_per_core, 0.50)),
        load_per_core_p95: sanitize_f64(percentile_r7(&load_per_core, 0.95)),
        contended_sample_count,
        total_sample_count: load_per_core.len(),
    })
}

/// Aggregate cold start durations
///
/// Returns percentiles of cold start durations if any results have cold start data.
pub(super) fn aggregate_cold_starts(results: &[&BenchmarkResult]) -> Option<DurationPercentiles> {
    let cold_starts: Vec<f64> = results
        .iter()
        .filter_map(|r| r.cold_start_duration.map(|d| d.as_secs_f64() * 1000.0))
        .filter(|&v| !v.is_nan() && v.is_finite())
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

/// Parse an aggregate key back into `(framework, mode)`.
///
/// Handles both key shapes produced by [`super::make_aggregate_key`]:
/// - `"framework:mode"` (xberg family, 2 parts)
/// - `"framework:output_format:mode"` (competitors, 3 parts)
pub(super) fn parse_aggregate_key(key: &str) -> (&str, &str) {
    let mut parts = key.rsplitn(2, ':');
    let mode = parts.next().unwrap_or("single");
    let remainder = parts.next().unwrap_or(key);
    let framework = remainder.split(':').next().unwrap_or(remainder);
    (framework, mode)
}

/// Weighted mean of `(value, weight)` pairs, ignoring non-finite values. Returns `NaN` if no
/// finite-weighted contribution exists (e.g. every value was non-finite, or the slice was empty).
pub(super) fn weighted_avg(items: &[(f64, usize)]) -> f64 {
    let finite: Vec<(f64, usize)> = items.iter().copied().filter(|(v, _)| v.is_finite()).collect();
    let total_weight: usize = finite.iter().map(|(_, w)| w).sum();
    if total_weight == 0 {
        f64::NAN
    } else {
        finite.iter().map(|(v, w)| v * (*w as f64)).sum::<f64>() / total_weight as f64
    }
}

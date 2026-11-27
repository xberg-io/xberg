//! Free-function helpers shared by the runner's orchestration and execution paths.

use crate::adapter::OcrLanguagePolicy;
use crate::fixture::FixtureManager;
use crate::stats::percentile_r7;
use crate::types::{BatchCapability, BatchTimingScope, DurationStatistics, IterationResult, PerformanceMetrics};
use crate::{Error, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::{BatchBenchmarkEntry, DiskSizeInfo};

pub(super) fn validate_timing_regime_contract(
    regimes: impl IntoIterator<Item = crate::types::TimingRegime>,
    warmup_iterations: usize,
) -> Result<()> {
    let regimes: Vec<_> = regimes.into_iter().collect();
    if regimes.iter().any(|regime| {
        matches!(
            regime,
            crate::types::TimingRegime::Mixed | crate::types::TimingRegime::Unknown
        )
    }) {
        return Err(Error::Config(
            "mixed/unknown timing regimes cannot be emitted by an adapter".to_string(),
        ));
    }
    if warmup_iterations == 0 && regimes.contains(&crate::types::TimingRegime::WarmInProcess) {
        return Err(Error::Config(
            "warm_in_process benchmarks require at least one discarded same-process warmup iteration".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn ensure_measured_iteration_count(
    framework: &str,
    expected: usize,
    actual: usize,
    timed_out: bool,
) -> Result<()> {
    let invalid = actual > expected || actual == 0 || (!timed_out && actual != expected);
    if invalid {
        return Err(Error::Benchmark(format!(
            "framework '{framework}' produced {actual} measured iteration result(s), expected {expected}"
        )));
    }
    Ok(())
}

pub(super) fn effective_batch_warmup_iterations(capability: BatchCapability, configured: usize) -> usize {
    if capability.timing_scope == BatchTimingScope::ColdEndToEndSubprocess {
        0
    } else {
        configured
    }
}

/// Calculate amplified iteration count for profiling when needed
///
/// When profiling is enabled, tasks can be amplified (repeated) to increase the
/// profiling duration and collect more samples. This function determines how many
/// times to repeat the task based on its estimated duration to reach a target
/// profiling duration.
///
/// # Arguments
/// * `estimated_duration_ms` - Estimated task duration in milliseconds
/// * `target_profile_duration_ms` - Target minimum profiling duration (default 1000ms)
///
/// # Returns
/// Number of amplified iterations (minimum 1)
pub(super) fn calculate_amplified_iterations(estimated_duration_ms: u64, target_profile_duration_ms: u64) -> usize {
    if estimated_duration_ms == 0 {
        return 1;
    }

    let amplification = (target_profile_duration_ms as f64 / estimated_duration_ms as f64).ceil() as usize;
    amplification.max(1)
}

pub(super) fn validate_ocr_cohort(ocr_enabled: bool, ocr_required_count: usize) -> Result<()> {
    if !ocr_enabled && ocr_required_count > 0 {
        return Err(Error::Config(format!(
            "OCR is disabled, but the selected fixture cohort contains {ocr_required_count} OCR-required fixture(s). \
             Rerun with --ocr or select a fixture directory/shard containing only non-OCR fixtures; \
             the harness will not silently omit OCR-required documents."
        )));
    }
    Ok(())
}

pub(super) fn validate_batch_ocr_cohort(batch_mode: bool, total_count: usize, ocr_required_count: usize) -> Result<()> {
    if batch_mode && ocr_required_count > 0 && ocr_required_count < total_count {
        return Err(Error::Config(format!(
            "native batch benchmarks require a homogeneous OCR cohort, but the selected fixture cohort mixes {} \
             force-OCR and {} non-force-OCR fixture(s). Select a fixture directory/shard containing only one OCR \
             mode; the harness will not label sequential fallback as batch throughput.",
            ocr_required_count,
            total_count - ocr_required_count
        )));
    }
    Ok(())
}

pub(super) fn load_quality_ground_truth(
    fixtures: &FixtureManager,
) -> Result<(HashMap<PathBuf, String>, HashMap<PathBuf, String>)> {
    let mut ground_truth_map = HashMap::new();
    let mut markdown_gt_map = HashMap::new();

    for (fixture_path, fixture) in fixtures.fixtures() {
        let fixture_dir = fixture_path.parent().unwrap_or_else(|| Path::new("."));
        let document_path = fixture.resolve_document_path(fixture_dir);
        let text_path = fixture.resolve_ground_truth_path(fixture_dir);
        let markdown_path = fixture.resolve_ground_truth_markdown_path(fixture_dir);

        if text_path.is_none() && markdown_path.is_none() {
            return Err(Error::Config(format!(
                "quality measurement requires text_file or markdown_file ground truth for {}",
                fixture.document.display()
            )));
        }

        let markdown = markdown_path
            .as_ref()
            .map(|path| {
                std::fs::read_to_string(path).map_err(|error| {
                    Error::Benchmark(format!(
                        "failed to read requested markdown ground truth for {}: {error}",
                        fixture.document.display()
                    ))
                })
            })
            .transpose()?;
        let text = if let Some(path) = text_path {
            std::fs::read_to_string(path).map_err(|error| {
                Error::Benchmark(format!(
                    "failed to read requested text ground truth for {}: {error}",
                    fixture.document.display()
                ))
            })?
        } else {
            markdown.clone().ok_or_else(|| {
                Error::Config(format!(
                    "quality measurement requires readable ground truth for {}",
                    fixture.document.display()
                ))
            })?
        };

        ground_truth_map.insert(document_path.clone(), text);
        if let Some(markdown) = markdown {
            markdown_gt_map.insert(document_path, markdown);
        }
    }

    Ok((ground_truth_map, markdown_gt_map))
}

/// Calculate statistics from iteration results
///
/// # Arguments
/// * `iterations` - Vector of iteration results to analyze
///
/// # Returns
/// Duration statistics including mean, median, std dev, and percentiles
pub(super) fn calculate_statistics(iterations: &[IterationResult]) -> DurationStatistics {
    if iterations.is_empty() {
        return DurationStatistics {
            mean: Duration::from_secs(0),
            median: Duration::from_secs(0),
            std_dev_ms: 0.0,
            min: Duration::from_secs(0),
            max: Duration::from_secs(0),
            p95: Duration::from_secs(0),
            p99: Duration::from_secs(0),
            sample_count: 0,
        };
    }

    let durations: Vec<Duration> = iterations.iter().map(|i| i.duration).collect();

    let min = *durations.iter().min().unwrap_or(&Duration::from_secs(0));
    let max = *durations.iter().max().unwrap_or(&Duration::from_secs(0));

    let total_ms: f64 = durations.iter().map(|d| d.as_secs_f64() * 1000.0).sum();
    let mean_ms = total_ms / durations.len() as f64;
    let mean = Duration::from_secs_f64(mean_ms / 1000.0);

    let mut durations_ms: Vec<f64> = durations.iter().map(|d| d.as_secs_f64() * 1000.0).collect();
    durations_ms.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let p50 = percentile_r7(&durations_ms, 0.50);
    let median = if p50.is_finite() {
        Duration::from_secs_f64(p50 / 1000.0)
    } else {
        Duration::from_secs(0)
    };

    let variance: f64 = if durations.len() > 1 {
        durations
            .iter()
            .map(|d| {
                let diff = d.as_secs_f64() * 1000.0 - mean_ms;
                diff * diff
            })
            .sum::<f64>()
            / (durations.len() - 1) as f64
    } else {
        0.0
    };

    let std_dev_ms = variance.sqrt();

    let p95_ms = percentile_r7(&durations_ms, 0.95);
    let p95 = if p95_ms.is_finite() {
        Duration::from_secs_f64(p95_ms / 1000.0)
    } else {
        Duration::from_secs(0)
    };

    let p99_ms = percentile_r7(&durations_ms, 0.99);
    let p99 = if p99_ms.is_finite() {
        Duration::from_secs_f64(p99_ms / 1000.0)
    } else {
        Duration::from_secs(0)
    };

    DurationStatistics {
        mean,
        median,
        std_dev_ms,
        min,
        max,
        p95,
        p99,
        sample_count: iterations.len(),
    }
}

/// Check if profiling is enabled via environment variable
///
/// # Returns
/// `true` if `ENABLE_PROFILING=true` is set, `false` otherwise
#[cfg(feature = "profiling")]
pub(super) fn should_profile() -> bool {
    std::env::var("ENABLE_PROFILING").unwrap_or_default() == "true"
}

/// Aggregate performance metrics from iterations (average)
pub(super) fn aggregate_metrics(iterations: &[IterationResult]) -> PerformanceMetrics {
    if iterations.is_empty() {
        return PerformanceMetrics::default();
    }

    let count = iterations.len() as f64;

    let baseline_memory_bytes = iterations
        .iter()
        .map(|i| i.metrics.baseline_memory_bytes)
        .max()
        .unwrap_or(0);

    let peak_memory_bytes = iterations
        .iter()
        .map(|i| i.metrics.peak_memory_bytes)
        .max()
        .unwrap_or(0);

    let peak_memory_delta_bytes = iterations
        .iter()
        .map(|i| i.metrics.peak_memory_delta_bytes)
        .max()
        .unwrap_or(0);

    let avg_cpu_percent = iterations.iter().map(|i| i.metrics.avg_cpu_percent).sum::<f64>() / count;

    let cpu_seconds = iterations.iter().map(|i| i.metrics.cpu_seconds).sum::<f64>() / count;

    let throughput_bytes_per_sec = iterations
        .iter()
        .map(|i| i.metrics.throughput_bytes_per_sec)
        .sum::<f64>()
        / count;

    let p50_memory_bytes = (iterations.iter().map(|i| i.metrics.p50_memory_bytes).sum::<u64>() as f64 / count) as u64;

    let p95_memory_bytes = (iterations.iter().map(|i| i.metrics.p95_memory_bytes).sum::<u64>() as f64 / count) as u64;

    let p99_memory_bytes = (iterations.iter().map(|i| i.metrics.p99_memory_bytes).sum::<u64>() as f64 / count) as u64;

    let memory_method = iterations[0].metrics.memory_method;

    let self_reported_peak_memory_bytes = iterations
        .iter()
        .filter_map(|i| i.metrics.self_reported_peak_memory_bytes)
        .max();

    PerformanceMetrics {
        baseline_memory_bytes,
        peak_memory_bytes,
        memory_method,
        self_reported_peak_memory_bytes,
        peak_memory_delta_bytes,
        avg_cpu_percent,
        cpu_seconds,
        throughput_bytes_per_sec,
        p50_memory_bytes,
        p95_memory_bytes,
        p99_memory_bytes,
    }
}

pub(super) fn average_durations(durations: impl Iterator<Item = Duration>) -> Option<Duration> {
    let (total, count) = durations.fold((Duration::ZERO, 0_u32), |(total, count), duration| {
        (total.saturating_add(duration), count.saturating_add(1))
    });
    (count > 0).then(|| total / count)
}

pub(super) fn resolve_cohort_manifest_path(fixture_root: &Path, manifest_path: &Path) -> PathBuf {
    if manifest_path.is_absolute() || manifest_path.exists() {
        manifest_path.to_path_buf()
    } else {
        fixture_root.join(manifest_path)
    }
}

pub(super) fn fixed_batch_ranges(item_count: usize, batch_size: Option<usize>) -> Result<Vec<std::ops::Range<usize>>> {
    let Some(batch_size) = batch_size else {
        return Ok((item_count > 0).then_some(0..item_count).into_iter().collect());
    };
    if batch_size == 0 {
        return Err(Error::Config("fixed batch size must be greater than zero".to_string()));
    }
    // Allow a smaller final batch. A framework benchmarks only the formats it declares support for,
    // so its eligible-document count need not be an exact multiple of the cohort's fixed batch size
    // (0 eligible → no batches). The last range is clamped to `item_count`.
    Ok((0..item_count)
        .step_by(batch_size)
        .map(|start| start..(start + batch_size).min(item_count))
        .collect())
}

pub(super) fn language_partitions(
    entries: Vec<BatchBenchmarkEntry>,
    policy: OcrLanguagePolicy,
) -> Vec<Vec<BatchBenchmarkEntry>> {
    if !policy.requires_homogeneous_batch_language() {
        return (!entries.is_empty()).then_some(entries).into_iter().collect();
    }

    let mut partitions: Vec<(Option<String>, Vec<BatchBenchmarkEntry>)> = Vec::new();
    for entry in entries {
        let key = policy.partition_key(entry.3.as_deref());
        if let Some((_, partition)) = partitions.iter_mut().find(|(candidate, _)| *candidate == key) {
            partition.push(entry);
        } else {
            partitions.push((key, vec![entry]));
        }
    }
    partitions.into_iter().map(|(_, entries)| entries).collect()
}

pub(super) fn ensure_batch_result_cardinality(
    adapter_name: &str,
    input_count: usize,
    result_count: usize,
) -> Result<()> {
    if input_count == result_count {
        return Ok(());
    }
    Err(Error::Benchmark(format!(
        "framework '{adapter_name}' returned {result_count} batch results for {input_count} inputs"
    )))
}

/// Resolve the installation size to report for a benchmark result framework.
///
/// Competitors (liteparse, docling, ...) name their benchmark row identically to
/// their size-map key, so they resolve by a direct lookup (after stripping the
/// `-batch`/`-sync`/`-async` mode suffix).
///
/// Xberg benchmark rows are named `xberg-<format>-<pipeline>` (e.g.
/// `xberg-markdown-baseline`, `xberg-markdown-layout`) and are *not* size-map
/// keys, so they map onto the measured native `xberg-rust` footprint:
/// - heuristic pipelines (baseline/plaintext) ship without ML models, so they
///   report the shipped binary+dylibs only (`package_bytes`) — the fair
///   comparison against model-free tools like LiteParse;
/// - ML pipelines (layout/paddle/candle) additionally require the on-demand
///   model cache, so they report `package_bytes + model_bytes`.
pub(super) fn resolve_installation_size(
    framework: &str,
    sizes: &HashMap<String, DiskSizeInfo>,
) -> Option<DiskSizeInfo> {
    let base_name = framework
        .trim_end_matches("-batch")
        .trim_end_matches("-sync")
        .trim_end_matches("-async");

    if let Some(size_info) = sizes.get(base_name) {
        return Some(size_info.clone());
    }

    if base_name.starts_with("xberg-") {
        let base = sizes.get("xberg-rust")?;
        let uses_models = ["layout", "paddle", "candle"].iter().any(|m| base_name.contains(m));
        let mut info = base.clone();
        if uses_models {
            info.size_bytes = base.package_bytes + base.model_bytes;
        } else {
            info.size_bytes = base.package_bytes;
            info.model_bytes = 0;
        }
        return Some(info);
    }

    None
}

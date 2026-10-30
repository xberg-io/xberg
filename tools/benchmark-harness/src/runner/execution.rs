//! Static iteration-execution helpers for single-file and native-batch benchmark tasks.
//!
//! These methods run outside of `&self` so they can be driven from an owned task queue
//! without holding a borrow of the runner across `.await` points.

use crate::adapter::FrameworkAdapter;
use crate::config::BenchmarkConfig;
use crate::system_load::SystemLoad;
use crate::types::{BatchCapability, BatchTimingScope, BenchmarkResult, ErrorKind, IterationResult, OutputFormat};
use crate::{Error, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use super::BenchmarkRunner;
use super::helpers::{
    aggregate_metrics, average_durations, calculate_amplified_iterations, calculate_statistics,
    effective_batch_warmup_iterations, ensure_measured_iteration_count,
};

#[cfg(feature = "profiling")]
use super::helpers::should_profile;
#[cfg(feature = "profiling")]
use crate::profile_report::ProfileReport;
#[cfg(feature = "profiling")]
use crate::profiling::ProfileGuard;

/// Target profiling duration (in milliseconds) that task amplification aims to reach.
const PROFILING_TARGET_DURATION_MS: u64 = 1000;

/// Parameters for a single-file iteration task, grouped to keep `run_iterations_static`'s
/// signature under the crate's parameter-count limit.
pub(super) struct SingleIterationTask<'a> {
    pub(super) file_path: &'a Path,
    pub(super) adapter: Arc<dyn FrameworkAdapter>,
    pub(super) config: &'a BenchmarkConfig,
    pub(super) cold_start_duration: Option<Duration>,
    pub(super) force_ocr: bool,
    pub(super) ocr_language: Option<&'a str>,
    pub(super) output_format: OutputFormat,
}

/// Parameters for a native-batch iteration task, grouped to keep
/// `run_batch_iterations_static`'s signature under the crate's parameter-count limit.
pub(super) struct BatchIterationTask<'a> {
    pub(super) file_paths: Vec<PathBuf>,
    pub(super) adapter: Arc<dyn FrameworkAdapter>,
    pub(super) config: &'a BenchmarkConfig,
    pub(super) cold_start_duration: Option<Duration>,
    pub(super) force_ocr_flags: Vec<bool>,
    pub(super) ocr_languages: Vec<Option<String>>,
    pub(super) output_format: OutputFormat,
}

fn dominant_error_kind(all_success: bool, failing_kinds: impl Iterator<Item = ErrorKind>) -> ErrorKind {
    if all_success {
        return ErrorKind::None;
    }
    failing_kinds
        .max_by_key(|error_kind| match error_kind {
            ErrorKind::Timeout => 4,
            ErrorKind::HarnessError => 3,
            ErrorKind::ConfigSetupError => 2,
            ErrorKind::FrameworkError | ErrorKind::EmptyContent | ErrorKind::ZeroOverlap => 1,
            ErrorKind::None => 0,
        })
        .unwrap_or(ErrorKind::None)
}

fn average_extraction_duration(extraction_durations: &[Duration]) -> Option<Duration> {
    if extraction_durations.is_empty() {
        return None;
    }
    let total_ms: f64 = extraction_durations.iter().map(|d| d.as_secs_f64() * 1000.0).sum();
    let avg_ms = total_ms / extraction_durations.len() as f64;
    if avg_ms.is_finite() {
        Some(Duration::from_secs_f64(avg_ms / 1000.0))
    } else {
        None
    }
}

/// The fixed part of a single extraction call, grouped so the helpers below stay under the
/// crate's parameter-count limit.
struct ExtractionContext<'a> {
    adapter: &'a Arc<dyn FrameworkAdapter>,
    file_path: &'a Path,
    config: &'a BenchmarkConfig,
    force_ocr: bool,
    ocr_language: Option<&'a str>,
    output_format: OutputFormat,
}

impl ExtractionContext<'_> {
    async fn extract_once(&self) -> Result<BenchmarkResult> {
        self.adapter
            .extract(
                self.file_path,
                self.config.timeout,
                self.force_ocr,
                self.ocr_language,
                self.output_format,
            )
            .await
    }
}

async fn estimate_task_duration_ms(context: &ExtractionContext<'_>) -> Result<u64> {
    let config = context.config;
    if config.profiling.enabled {
        let warmup_start = std::time::Instant::now();
        let warmup_result = context.extract_once().await?;
        let _warmup_duration = warmup_start.elapsed();
        Ok(warmup_result.duration.as_millis() as u64)
    } else {
        Ok(config.profiling.task_duration_ms)
    }
}

#[cfg(feature = "profiling")]
fn start_profiler_if_enabled(estimated_task_duration_ms: u64, config: &BenchmarkConfig) -> Option<ProfileGuard> {
    let sampling_frequency = crate::config::ProfilingConfig::calculate_optimal_frequency(estimated_task_duration_ms);

    if !(should_profile() && config.profiling.enabled) {
        return None;
    }

    match ProfileGuard::new(sampling_frequency) {
        Ok(g) => {
            eprintln!(
                "Profiling enabled: {} Hz sampling frequency for ~{}ms tasks",
                sampling_frequency, estimated_task_duration_ms
            );
            Some(g)
        }
        Err(e) => {
            eprintln!("Warning: Failed to start profiler: {}", e);
            None
        }
    }
}

#[cfg(feature = "profiling")]
fn write_profile_report(result: &crate::profiling::ProfilingResult, framework_name: &str, report_path: &str) {
    let profile_report = ProfileReport::from_profiling_result(result, framework_name);
    let html_report = profile_report.generate_html();

    let report_file_path = Path::new(report_path);
    if let Some(parent) = report_file_path.parent()
        && !parent.as_os_str().is_empty()
    {
        if let Err(e) = std::fs::create_dir_all(parent) {
            eprintln!("Warning: Failed to create report directory: {}", e);
        } else if let Err(e) = std::fs::write(report_file_path, html_report) {
            eprintln!("Warning: Failed to write HTML report: {}", e);
        } else {
            eprintln!("Profile report written to: {}", report_path);
        }
    }
}

#[cfg(feature = "profiling")]
fn finish_profiling(
    profiler: Option<ProfileGuard>,
    adapter: &Arc<dyn FrameworkAdapter>,
    config: &BenchmarkConfig,
    file_path: &Path,
) {
    let Some(profiler) = profiler else {
        return;
    };
    let framework_name = adapter.name();
    let mode_name = match config.benchmark_mode {
        crate::config::BenchmarkMode::SingleFile => "single-file",
        crate::config::BenchmarkMode::Batch => "batch",
    };
    let fixture_stem = file_path.file_stem().and_then(|s| s.to_str()).unwrap_or_else(|| {
        eprintln!(
            "Warning: Failed to extract valid UTF-8 filename from {:?}, using sanitized fallback",
            file_path
        );
        "unknown_file"
    });

    let flamegraph_path = format!("flamegraphs/{}/{}/{}.svg", framework_name, mode_name, fixture_stem);
    let report_path = format!(
        "flamegraphs/{}/{}/{}_report.html",
        framework_name, mode_name, fixture_stem
    );

    match profiler.finish() {
        Ok(result) => {
            eprintln!(
                "Profiling complete: {} samples collected in {:?}",
                result.sample_count, result.duration
            );

            if result.sample_count < config.profiling.sample_count_threshold {
                eprintln!(
                    "Warning: Low sample count ({} < {} threshold); profile may have high variance",
                    result.sample_count, config.profiling.sample_count_threshold
                );
            }

            if config.profiling.flamegraph_enabled {
                let path = Path::new(&flamegraph_path);
                if let Err(e) = result.generate_flamegraph(path) {
                    eprintln!("Warning: Failed to generate flamegraph: {}", e);
                }
                write_profile_report(&result, framework_name, &report_path);
            }
        }
        Err(e) => eprintln!("Warning: Profiling error: {}", e),
    }
}

async fn run_warmup_iterations(context: &ExtractionContext<'_>) -> Result<bool> {
    let config = context.config;
    let warmup_start = if config.profiling.enabled { 1 } else { 0 };
    let mut warmup_timed_out = false;
    for _iteration in warmup_start..config.warmup_iterations {
        let result = context.extract_once().await?;
        if result.error_kind == ErrorKind::Timeout {
            if context.adapter.timing_regime() == crate::types::TimingRegime::WarmInProcess {
                return Err(Error::Benchmark(format!(
                    "same-process warmup for '{}' timed out; aborting to prevent abandoned work from contaminating later samples",
                    context.adapter.name()
                )));
            }
            warmup_timed_out = true;
            break;
        }
        if context.adapter.timing_regime() == crate::types::TimingRegime::WarmInProcess && !result.success {
            return Err(Error::Benchmark(format!(
                "same-process warmup for '{}' failed: {}",
                context.adapter.name(),
                result
                    .error_message
                    .as_deref()
                    .unwrap_or("framework returned success=false")
            )));
        }
        drop(result);
    }
    Ok(warmup_timed_out)
}

async fn run_measured_iterations(
    context: &ExtractionContext<'_>,
    amplification_factor: usize,
    warmup_timed_out: bool,
) -> Result<Vec<BenchmarkResult>> {
    let mut all_results = Vec::new();
    let effective_iterations = if warmup_timed_out {
        1
    } else {
        context.config.benchmark_iterations
    };
    'outer: for _iteration in 0..effective_iterations {
        for _amp in 0..amplification_factor {
            let result = context.extract_once().await?;
            let timed_out = result.error_kind == ErrorKind::Timeout;
            all_results.push(result);
            if timed_out {
                break 'outer;
            }
        }
    }
    Ok(all_results)
}

fn build_single_benchmark_result(
    all_results: Vec<BenchmarkResult>,
    cold_start_duration: Option<Duration>,
) -> Result<BenchmarkResult> {
    if all_results.is_empty() {
        return Err(Error::Benchmark("No successful iterations".to_string()));
    }

    let iterations: Vec<IterationResult> = all_results
        .iter()
        .enumerate()
        .map(|(idx, result)| IterationResult {
            iteration: idx + 1,
            success: Some(result.success),
            error_kind: Some(result.error_kind),
            duration: result.duration,
            extraction_duration: result.extraction_duration,
            subprocess_overhead: result.subprocess_overhead,
            metrics: result.metrics.clone(),
            batch_sample_id: result.framework_capabilities.batch_sample_id.clone(),
            batch_performance_sample: result.framework_capabilities.batch_performance_sample,
        })
        .collect();

    let statistics = calculate_statistics(&iterations);
    let aggregated_metrics = aggregate_metrics(&iterations);

    let extraction_durations: Vec<Duration> = all_results.iter().filter_map(|r| r.extraction_duration).collect();
    let avg_extraction_duration = average_extraction_duration(&extraction_durations);

    let subprocess_overhead = avg_extraction_duration.map(|ext| statistics.mean.saturating_sub(ext));

    let first_result = &all_results[0];
    let representative_result = all_results.iter().find(|result| result.success).unwrap_or(first_result);
    let all_success = all_results.iter().all(|result| result.success);
    let error_message = all_results
        .iter()
        .find(|result| !result.success)
        .and_then(|result| result.error_message.clone());

    let error_kind = dominant_error_kind(
        all_success,
        all_results
            .iter()
            .filter(|result| !result.success)
            .map(|r| r.error_kind),
    );

    let quality = representative_result.quality.clone();

    Ok(BenchmarkResult {
        framework: first_result.framework.clone(),
        output_format: first_result.output_format,
        file_path: first_result.file_path.clone(),
        file_size: first_result.file_size,
        success: all_success,
        error_message,
        error_kind,
        duration: statistics.mean,
        extraction_duration: avg_extraction_duration,
        subprocess_overhead,
        metrics: aggregated_metrics,
        quality,
        iterations,
        statistics: Some(statistics),
        cold_start_duration,
        file_extension: first_result.file_extension.clone(),
        framework_capabilities: first_result.framework_capabilities.clone(),
        pdf_metadata: representative_result.pdf_metadata.clone(),
        ocr_status: representative_result.ocr_status,
        extracted_text: representative_result.extracted_text.clone(),
        system_load: Some(SystemLoad::capture()?),
    })
}

fn validate_batch_task(
    adapter: &Arc<dyn FrameworkAdapter>,
    file_paths: &[PathBuf],
    force_ocr_flags: &[bool],
    ocr_languages: &[Option<String>],
) -> Result<BatchCapability> {
    let batch_capability = adapter.batch_capability().ok_or_else(|| {
        Error::Config(format!(
            "framework '{}' does not expose a verified native batch API",
            adapter.name()
        ))
    })?;
    if force_ocr_flags.len() != file_paths.len() {
        return Err(Error::Benchmark(format!(
            "batch force_ocr cardinality mismatch: received {} flags for {} files",
            force_ocr_flags.len(),
            file_paths.len()
        )));
    }
    if ocr_languages.len() != file_paths.len() {
        return Err(Error::Benchmark(format!(
            "batch ocr_languages cardinality mismatch: received {} values for {} files",
            ocr_languages.len(),
            file_paths.len()
        )));
    }
    Ok(batch_capability)
}

/// The fixed part of a native-batch extraction call, grouped so `run_batch_extraction_iterations`
/// stays under the crate's parameter-count limit.
#[derive(Clone, Copy)]
struct BatchExtractionContext<'a> {
    adapter: &'a Arc<dyn FrameworkAdapter>,
    file_paths: &'a [PathBuf],
    config: &'a BenchmarkConfig,
    force_ocr_flags: &'a [bool],
    ocr_languages: &'a [Option<String>],
    output_format: OutputFormat,
}

async fn run_batch_extraction_iterations(
    context: &BatchExtractionContext<'_>,
    batch_capability: BatchCapability,
) -> Result<Vec<Vec<BenchmarkResult>>> {
    let BatchExtractionContext {
        adapter,
        file_paths,
        config,
        force_ocr_flags,
        ocr_languages,
        output_format,
    } = *context;

    let warmup_iterations = effective_batch_warmup_iterations(batch_capability, config.warmup_iterations);
    let total_iterations = warmup_iterations + config.benchmark_iterations;
    let mut all_batch_results = Vec::new();

    for iteration in 0..total_iterations {
        let refs: Vec<&Path> = file_paths.iter().map(|p| p.as_path()).collect();
        let mut batch_results = adapter
            .extract_batch(&refs, config.timeout, force_ocr_flags, ocr_languages, output_format)
            .await?;
        for result in &mut batch_results {
            result.framework_capabilities.batch_support = true;
            result.framework_capabilities.batch_capability = Some(batch_capability);
        }
        if batch_results.len() != file_paths.len() {
            return Err(Error::Benchmark(format!(
                "framework '{}' returned {} batch results for {} files",
                adapter.name(),
                batch_results.len(),
                file_paths.len()
            )));
        }

        // Per-item framework failures are accountable benchmark rows, not harness failures.
        // Retain them so aggregation and the minimum-success gate evaluate the full cohort.
        let has_timeout = batch_results.iter().any(|r| r.error_kind == ErrorKind::Timeout);

        if iteration < warmup_iterations
            && let Some(failure) = batch_results.iter().find(|result| !result.success)
        {
            return Err(Error::Benchmark(format!(
                "discarded batch warmup {} failed for '{}': {}",
                iteration + 1,
                adapter.name(),
                failure
                    .error_message
                    .as_deref()
                    .unwrap_or("framework returned an unsuccessful warmup row")
            )));
        }

        if iteration >= warmup_iterations || has_timeout {
            all_batch_results.push(batch_results);
        }

        if has_timeout {
            break;
        }
    }

    Ok(all_batch_results)
}

fn build_one_batch_file_result(
    all_batch_results: &[Vec<BenchmarkResult>],
    file_idx: usize,
    cold_start_duration: Option<Duration>,
) -> Result<BenchmarkResult> {
    let mut file_iterations = Vec::new();
    for batch in all_batch_results {
        file_iterations.push(&batch[file_idx]);
    }

    let iterations: Vec<IterationResult> = file_iterations
        .iter()
        .enumerate()
        .map(|(idx, result)| IterationResult {
            iteration: idx + 1,
            success: Some(result.success),
            error_kind: Some(result.error_kind),
            duration: result.duration,
            extraction_duration: result.extraction_duration,
            subprocess_overhead: result.subprocess_overhead,
            metrics: result.metrics.clone(),
            batch_sample_id: result.framework_capabilities.batch_sample_id.clone(),
            batch_performance_sample: result.framework_capabilities.batch_performance_sample,
        })
        .collect();

    let statistics = calculate_statistics(&iterations);
    let aggregated_metrics = aggregate_metrics(&iterations);

    let extraction_durations: Vec<Duration> = file_iterations.iter().filter_map(|r| r.extraction_duration).collect();
    let avg_extraction_duration = average_extraction_duration(&extraction_durations);

    let avg_subprocess_overhead =
        average_durations(file_iterations.iter().filter_map(|result| result.subprocess_overhead));

    let first_result = file_iterations[0];
    let representative_result = file_iterations
        .iter()
        .copied()
        .find(|result| result.success)
        .unwrap_or(first_result);
    let (all_success, error_message, error_kind) = summarize_batch_outcomes(&file_iterations);
    let mut framework_capabilities = first_result.framework_capabilities.clone();
    framework_capabilities.batch_sample_id = None;
    framework_capabilities.batch_performance_sample = None;

    Ok(BenchmarkResult {
        framework: first_result.framework.clone(),
        output_format: first_result.output_format,
        file_path: first_result.file_path.clone(),
        file_size: first_result.file_size,
        success: all_success,
        error_message,
        error_kind,
        duration: statistics.mean,
        extraction_duration: avg_extraction_duration,
        subprocess_overhead: avg_subprocess_overhead,
        metrics: aggregated_metrics,
        quality: representative_result.quality.clone(),
        iterations,
        statistics: Some(statistics),
        cold_start_duration,
        file_extension: first_result.file_extension.clone(),
        framework_capabilities,
        pdf_metadata: representative_result.pdf_metadata.clone(),
        ocr_status: representative_result.ocr_status,
        extracted_text: representative_result.extracted_text.clone(),
        system_load: Some(SystemLoad::capture()?),
    })
}

fn summarize_batch_outcomes(file_iterations: &[&BenchmarkResult]) -> (bool, Option<String>, ErrorKind) {
    let all_success = file_iterations.iter().all(|result| result.success);
    let error_message = file_iterations
        .iter()
        .find(|result| !result.success)
        .and_then(|result| result.error_message.clone());
    let error_kind = dominant_error_kind(
        all_success,
        file_iterations
            .iter()
            .filter(|result| !result.success)
            .map(|result| result.error_kind),
    );
    (all_success, error_message, error_kind)
}

fn build_batch_benchmark_results(
    all_batch_results: Vec<Vec<BenchmarkResult>>,
    file_paths: &[PathBuf],
    cold_start_duration: Option<Duration>,
) -> Result<Vec<BenchmarkResult>> {
    if all_batch_results.is_empty() {
        return Err(Error::Benchmark("No batch results".to_string()));
    }

    let num_files = file_paths.len();
    let mut aggregated_results = Vec::with_capacity(num_files);
    for file_idx in 0..num_files {
        aggregated_results.push(build_one_batch_file_result(
            &all_batch_results,
            file_idx,
            cold_start_duration,
        )?);
    }

    Ok(aggregated_results)
}

impl BenchmarkRunner {
    /// Run multiple iterations of a single extraction task (static method for async spawning)
    ///
    /// # Returns
    /// Aggregated benchmark result with iterations and statistics
    pub(super) async fn run_iterations_static(task: SingleIterationTask<'_>) -> Result<BenchmarkResult> {
        let SingleIterationTask {
            file_path,
            adapter,
            config,
            cold_start_duration,
            force_ocr,
            ocr_language,
            output_format,
        } = task;

        let context = ExtractionContext {
            adapter: &adapter,
            file_path,
            config,
            force_ocr,
            ocr_language,
            output_format,
        };

        let estimated_task_duration_ms = estimate_task_duration_ms(&context).await?;

        #[cfg(feature = "profiling")]
        let profiler = start_profiler_if_enabled(estimated_task_duration_ms, config);

        let warmup_timed_out = run_warmup_iterations(&context).await?;

        let amplification_factor = if config.profiling.enabled {
            calculate_amplified_iterations(estimated_task_duration_ms, PROFILING_TARGET_DURATION_MS)
        } else {
            1
        };

        let all_results = run_measured_iterations(&context, amplification_factor, warmup_timed_out).await?;
        let timed_out = all_results.iter().any(|result| result.error_kind == ErrorKind::Timeout);
        if timed_out && adapter.timing_regime() == crate::types::TimingRegime::WarmInProcess {
            return Err(Error::Benchmark(format!(
                "same-process measurement for '{}' timed out; aborting to prevent abandoned work from contaminating later samples",
                adapter.name()
            )));
        }
        let expected_results = if warmup_timed_out {
            1
        } else {
            config.benchmark_iterations * amplification_factor
        };
        ensure_measured_iteration_count(adapter.name(), expected_results, all_results.len(), timed_out)?;

        #[cfg(feature = "profiling")]
        finish_profiling(profiler, &adapter, config, file_path);

        build_single_benchmark_result(all_results, cold_start_duration)
    }

    /// Run multiple iterations of batch extraction (static method for async spawning)
    ///
    /// # Returns
    /// Vector of aggregated benchmark results (one per file) with iterations and statistics
    pub(super) async fn run_batch_iterations_static(task: BatchIterationTask<'_>) -> Result<Vec<BenchmarkResult>> {
        let BatchIterationTask {
            file_paths,
            adapter,
            config,
            cold_start_duration,
            force_ocr_flags,
            ocr_languages,
            output_format,
        } = task;

        let batch_capability = validate_batch_task(&adapter, &file_paths, &force_ocr_flags, &ocr_languages)?;

        let extraction_context = BatchExtractionContext {
            adapter: &adapter,
            file_paths: &file_paths,
            config,
            force_ocr_flags: &force_ocr_flags,
            ocr_languages: &ocr_languages,
            output_format,
        };

        let all_batch_results = run_batch_extraction_iterations(&extraction_context, batch_capability).await?;

        let cold_start_duration = match batch_capability.timing_scope {
            BatchTimingScope::WarmSteadyState => None,
            BatchTimingScope::ColdEndToEndSubprocess => cold_start_duration,
        };

        build_batch_benchmark_results(all_batch_results, &file_paths, cold_start_duration)
    }
}

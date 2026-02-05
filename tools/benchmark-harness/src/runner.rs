//! Benchmark runner for executing and collecting results
//!
//! This module orchestrates benchmark execution across multiple fixtures and frameworks,
//! with support for concurrent execution and progress reporting.

use crate::adapter::FrameworkAdapter;
use crate::config::{BenchmarkConfig, BenchmarkMode};
use crate::fixture::FixtureManager;
use crate::registry::AdapterRegistry;
use crate::stats::percentile_r7;
use crate::types::{BenchmarkResult, DiskSizeInfo, DurationStatistics, IterationResult, PerformanceMetrics};
use crate::{Error, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

#[cfg(feature = "profiling")]
use crate::profile_report::ProfileReport;
#[cfg(feature = "profiling")]
use crate::profiling::ProfileGuard;

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
fn calculate_amplified_iterations(estimated_duration_ms: u64, target_profile_duration_ms: u64) -> usize {
    if estimated_duration_ms == 0 {
        return 1;
    }

    let amplification = (target_profile_duration_ms as f64 / estimated_duration_ms as f64).ceil() as usize;
    amplification.max(1)
}

/// Calculate statistics from iteration results
///
/// # Arguments
/// * `iterations` - Vector of iteration results to analyze
///
/// # Returns
/// Duration statistics including mean, median, std dev, and percentiles
fn calculate_statistics(iterations: &[IterationResult]) -> DurationStatistics {
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

    // Validate percentile is finite before creating Duration
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

    // Validate p95 is finite before creating Duration
    let p95_ms = percentile_r7(&durations_ms, 0.95);
    let p95 = if p95_ms.is_finite() {
        Duration::from_secs_f64(p95_ms / 1000.0)
    } else {
        Duration::from_secs(0)
    };

    // Validate p99 is finite before creating Duration
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
fn should_profile() -> bool {
    std::env::var("ENABLE_PROFILING").unwrap_or_default() == "true"
}

/// Aggregate performance metrics from iterations (average)
fn aggregate_metrics(iterations: &[IterationResult]) -> PerformanceMetrics {
    if iterations.is_empty() {
        return PerformanceMetrics::default();
    }

    let count = iterations.len() as f64;

    let peak_memory_bytes = iterations
        .iter()
        .map(|i| i.metrics.peak_memory_bytes)
        .max()
        .unwrap_or(0);

    let avg_cpu_percent = iterations.iter().map(|i| i.metrics.avg_cpu_percent).sum::<f64>() / count;

    let throughput_bytes_per_sec = iterations
        .iter()
        .map(|i| i.metrics.throughput_bytes_per_sec)
        .sum::<f64>()
        / count;

    let p50_memory_bytes = (iterations.iter().map(|i| i.metrics.p50_memory_bytes).sum::<u64>() as f64 / count) as u64;

    let p95_memory_bytes = (iterations.iter().map(|i| i.metrics.p95_memory_bytes).sum::<u64>() as f64 / count) as u64;

    let p99_memory_bytes = (iterations.iter().map(|i| i.metrics.p99_memory_bytes).sum::<u64>() as f64 / count) as u64;

    PerformanceMetrics {
        peak_memory_bytes,
        avg_cpu_percent,
        throughput_bytes_per_sec,
        p50_memory_bytes,
        p95_memory_bytes,
        p99_memory_bytes,
    }
}

/// Orchestrates benchmark execution across fixtures and frameworks
pub struct BenchmarkRunner {
    config: BenchmarkConfig,
    registry: AdapterRegistry,
    fixtures: FixtureManager,
    cold_start_durations: HashMap<String, Duration>,
    framework_sizes: HashMap<String, DiskSizeInfo>,
}

impl BenchmarkRunner {
    /// Create a new benchmark runner
    pub fn new(config: BenchmarkConfig, registry: AdapterRegistry) -> Self {
        // Measure actual framework sizes instead of loading from static config
        // This ensures accurate disk size reporting in benchmark results
        let framework_sizes = match crate::sizes::measure_framework_sizes() {
            Ok(sizes) => {
                if !sizes.is_empty() {
                    eprintln!("Measured disk sizes for {} frameworks", sizes.len());
                }
                // Convert FrameworkSize to DiskSizeInfo (drop deprecated `estimated` field)
                sizes
                    .into_iter()
                    .map(|(name, fs)| {
                        (
                            name,
                            DiskSizeInfo {
                                size_bytes: fs.size_bytes,
                                method: fs.method,
                                description: fs.description,
                            },
                        )
                    })
                    .collect()
            }
            Err(e) => {
                eprintln!("Warning: Failed to measure framework sizes: {}", e);
                // Fall back to static config if measurement fails entirely
                crate::config::load_framework_sizes(Path::new("tools/benchmark-harness/config/framework_sizes.json"))
                    .unwrap_or_else(|e2| {
                        eprintln!("Warning: Also failed to load static framework sizes: {}", e2);
                        HashMap::new()
                    })
            }
        };

        Self {
            config,
            registry,
            fixtures: FixtureManager::new(),
            cold_start_durations: HashMap::new(),
            framework_sizes,
        }
    }

    /// Load fixtures from a directory or file
    pub fn load_fixtures(&mut self, path: &PathBuf) -> Result<()> {
        if path.is_dir() {
            self.fixtures.load_fixtures_from_dir(path)?;
        } else {
            self.fixtures.load_fixture(path)?;
        }
        Ok(())
    }

    /// Filter fixtures by file type (to be implemented when needed)
    ///
    /// For now, filtering is done during execution based on adapter support
    pub fn filter_fixtures(&mut self, _file_types: &[String]) {
        // TODO: Implement fixture filtering if needed
    }

    /// Get count of loaded fixtures
    pub fn fixture_count(&self) -> usize {
        self.fixtures.len()
    }

    /// Enrich a benchmark result with framework size information
    ///
    /// # Arguments
    /// * `result` - Mutable reference to benchmark result to enrich
    fn enrich_with_framework_size(&self, result: &mut BenchmarkResult) {
        // Strip -batch suffix FIRST, then -sync/-async to find base framework
        let base_name = result
            .framework
            .trim_end_matches("-batch")
            .trim_end_matches("-sync")
            .trim_end_matches("-async");

        if let Some(size_info) = self.framework_sizes.get(base_name) {
            result.framework_capabilities.installation_size = Some(size_info.clone());
        }
    }

    /// Run multiple iterations of a single extraction task (static method for async spawning)
    ///
    /// # Arguments
    /// * `file_path` - Path to file to extract
    /// * `adapter` - Framework adapter to use
    /// * `config` - Benchmark configuration
    /// * `cold_start_duration` - Optional cold start duration for this framework
    ///
    /// # Returns
    /// Aggregated benchmark result with iterations and statistics
    async fn run_iterations_static(
        file_path: &Path,
        adapter: Arc<dyn FrameworkAdapter>,
        config: &BenchmarkConfig,
        cold_start_duration: Option<Duration>,
    ) -> Result<BenchmarkResult> {
        let mut all_results = Vec::new();

        let estimated_task_duration_ms = if config.profiling.enabled {
            let warmup_start = std::time::Instant::now();
            let warmup_result = adapter.extract(file_path, config.timeout).await?;
            let _warmup_duration = warmup_start.elapsed();
            warmup_result.duration.as_millis() as u64
        } else {
            config.profiling.task_duration_ms
        };

        #[cfg(feature = "profiling")]
        let sampling_frequency =
            crate::config::ProfilingConfig::calculate_optimal_frequency(estimated_task_duration_ms);

        #[cfg(feature = "profiling")]
        let profiler = if should_profile() && config.profiling.enabled {
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
        } else {
            None
        };

        let warmup_start = if config.profiling.enabled { 1 } else { 0 };
        for _iteration in warmup_start..config.warmup_iterations {
            let result = adapter.extract(file_path, config.timeout).await?;
            drop(result);
        }

        let amplification_factor = if config.profiling.enabled {
            calculate_amplified_iterations(estimated_task_duration_ms, 1000)
        } else {
            1
        };

        for _iteration in 0..config.benchmark_iterations {
            for _amp in 0..amplification_factor {
                let result = adapter.extract(file_path, config.timeout).await?;
                all_results.push(result);
            }
        }

        #[cfg(feature = "profiling")]
        if let Some(profiler) = profiler {
            let framework_name = adapter.name();
            let mode_name = match config.benchmark_mode {
                BenchmarkMode::SingleFile => "single-file",
                BenchmarkMode::Batch => "batch",
            };
            // Extract and sanitize filename for use in paths, with fallback for bad filenames
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

                        let profile_report = ProfileReport::from_profiling_result(&result, framework_name);
                        let html_report = profile_report.generate_html();

                        let report_file_path = Path::new(&report_path);
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
                }
                Err(e) => eprintln!("Warning: Profiling error: {}", e),
            }
        }

        if config.benchmark_iterations == 1 && !all_results.is_empty() {
            let mut result = all_results
                .into_iter()
                .next()
                .ok_or_else(|| Error::Benchmark("Failed to retrieve single iteration result".to_string()))?;
            result.cold_start_duration = cold_start_duration;
            return Ok(result);
        }

        if all_results.is_empty() {
            return Err(Error::Benchmark("No successful iterations".to_string()));
        }

        let iterations: Vec<IterationResult> = all_results
            .iter()
            .enumerate()
            .map(|(idx, result)| IterationResult {
                iteration: idx + 1,
                duration: result.duration,
                extraction_duration: result.extraction_duration,
                metrics: result.metrics.clone(),
            })
            .collect();

        let statistics = calculate_statistics(&iterations);

        let aggregated_metrics = aggregate_metrics(&iterations);

        let extraction_durations: Vec<Duration> = all_results.iter().filter_map(|r| r.extraction_duration).collect();

        let avg_extraction_duration = if !extraction_durations.is_empty() {
            let total_ms: f64 = extraction_durations.iter().map(|d| d.as_secs_f64() * 1000.0).sum();
            let avg_ms = total_ms / extraction_durations.len() as f64;
            // Ensure the average is finite before creating Duration
            if avg_ms.is_finite() {
                Some(Duration::from_secs_f64(avg_ms / 1000.0))
            } else {
                None
            }
        } else {
            None
        };

        let subprocess_overhead = avg_extraction_duration.map(|ext| statistics.mean.saturating_sub(ext));

        let first_result = &all_results[0];

        Ok(BenchmarkResult {
            framework: first_result.framework.clone(),
            file_path: first_result.file_path.clone(),
            file_size: first_result.file_size,
            success: true,
            error_message: None,
            duration: statistics.mean,
            extraction_duration: avg_extraction_duration,
            subprocess_overhead,
            metrics: aggregated_metrics,
            quality: first_result.quality.clone(),
            iterations,
            statistics: Some(statistics),
            cold_start_duration,
            file_extension: first_result.file_extension.clone(),
            framework_capabilities: first_result.framework_capabilities.clone(),
            pdf_metadata: first_result.pdf_metadata.clone(),
            ocr_status: first_result.ocr_status,
            extracted_text: first_result.extracted_text.clone(),
        })
    }

    /// Run multiple iterations of batch extraction (static method for async spawning)
    ///
    /// # Arguments
    /// * `file_paths` - Paths to files to extract in batch
    /// * `adapter` - Framework adapter to use
    /// * `config` - Benchmark configuration
    /// * `cold_start_duration` - Optional cold start duration for this framework
    ///
    /// # Returns
    /// Vector of aggregated benchmark results (one per file) with iterations and statistics
    async fn run_batch_iterations_static(
        file_paths: Vec<PathBuf>,
        adapter: Arc<dyn FrameworkAdapter>,
        config: &BenchmarkConfig,
        cold_start_duration: Option<Duration>,
    ) -> Result<Vec<BenchmarkResult>> {
        let total_iterations = config.warmup_iterations + config.benchmark_iterations;
        let mut all_batch_results = Vec::new();

        for iteration in 0..total_iterations {
            let refs: Vec<&std::path::Path> = file_paths.iter().map(|p| p.as_path()).collect();
            let batch_results = adapter.extract_batch(&refs, config.timeout).await?;

            if iteration >= config.warmup_iterations {
                all_batch_results.push(batch_results);
            }
        }

        if config.benchmark_iterations == 1 && !all_batch_results.is_empty() {
            let mut result = all_batch_results
                .into_iter()
                .next()
                .ok_or_else(|| Error::Benchmark("Failed to retrieve single batch iteration result".to_string()))?;
            for r in &mut result {
                r.cold_start_duration = cold_start_duration;
            }
            return Ok(result);
        }

        // Aggregate per-file across iterations
        if all_batch_results.is_empty() {
            return Err(Error::Benchmark("No batch results".to_string()));
        }

        let num_files = all_batch_results[0].len();
        let mut aggregated_results = Vec::new();

        for file_idx in 0..num_files {
            let mut file_iterations = Vec::new();
            for batch in &all_batch_results {
                if file_idx < batch.len() {
                    file_iterations.push(&batch[file_idx]);
                }
            }

            if file_iterations.is_empty() {
                continue;
            }

            let iterations: Vec<IterationResult> = file_iterations
                .iter()
                .enumerate()
                .map(|(idx, result)| IterationResult {
                    iteration: idx + 1,
                    duration: result.duration,
                    extraction_duration: result.extraction_duration,
                    metrics: result.metrics.clone(),
                })
                .collect();

            let statistics = calculate_statistics(&iterations);
            let aggregated_metrics = aggregate_metrics(&iterations);

            let extraction_durations: Vec<Duration> =
                file_iterations.iter().filter_map(|r| r.extraction_duration).collect();

            let avg_extraction_duration = if !extraction_durations.is_empty() {
                let total_ms: f64 = extraction_durations.iter().map(|d| d.as_secs_f64() * 1000.0).sum();
                let avg_ms = total_ms / extraction_durations.len() as f64;
                // Ensure the average is finite before creating Duration
                if avg_ms.is_finite() {
                    Some(Duration::from_secs_f64(avg_ms / 1000.0))
                } else {
                    None
                }
            } else {
                None
            };

            let subprocess_overhead = avg_extraction_duration.map(|ext| statistics.mean.saturating_sub(ext));
            let first_result = file_iterations[0];

            aggregated_results.push(BenchmarkResult {
                framework: first_result.framework.clone(),
                file_path: first_result.file_path.clone(),
                file_size: first_result.file_size,
                success: true,
                error_message: None,
                duration: statistics.mean,
                extraction_duration: avg_extraction_duration,
                subprocess_overhead,
                metrics: aggregated_metrics,
                quality: first_result.quality.clone(),
                iterations,
                statistics: Some(statistics),
                cold_start_duration,
                file_extension: first_result.file_extension.clone(),
                framework_capabilities: first_result.framework_capabilities.clone(),
                pdf_metadata: first_result.pdf_metadata.clone(),
                ocr_status: first_result.ocr_status,
                extracted_text: first_result.extracted_text.clone(),
            });
        }

        Ok(aggregated_results)
    }

    /// Run benchmarks for specified frameworks
    ///
    /// # Arguments
    /// * `framework_names` - Names of frameworks to benchmark (empty = all registered)
    ///
    /// # Returns
    /// Vector of benchmark results
    pub async fn run(&mut self, framework_names: &[String]) -> Result<Vec<BenchmarkResult>> {
        let frameworks = if framework_names.is_empty() {
            self.registry
                .adapter_names()
                .into_iter()
                .filter_map(|name| self.registry.get(&name))
                .collect::<Vec<_>>()
        } else {
            framework_names
                .iter()
                .filter_map(|name| self.registry.get(name))
                .collect::<Vec<_>>()
        };

        if frameworks.is_empty() {
            return Err(Error::Benchmark("No frameworks available for benchmarking".to_string()));
        }

        for adapter in &frameworks {
            adapter.setup().await?;
        }

        if let Some((fixture_path, fixture)) = self.fixtures.fixtures().first() {
            let fixture_dir = fixture_path.parent().unwrap_or_else(|| std::path::Path::new("."));
            let warmup_file = fixture.resolve_document_path(fixture_dir);

            for adapter in &frameworks {
                if !adapter.supports_format(&fixture.file_type) {
                    continue;
                }

                println!("Warming up {} with {}...", adapter.name(), warmup_file.display());
                match adapter.warmup(&warmup_file, self.config.timeout).await {
                    Ok(cold_start) => {
                        println!("  Cold start: {:?}", cold_start);
                        self.cold_start_durations.insert(adapter.name().to_string(), cold_start);
                    }
                    Err(e) => {
                        eprintln!("  Warning: Warmup failed for {}: {}", adapter.name(), e);
                    }
                }
            }
        }

        let mut results = Vec::new();

        let use_batch = matches!(self.config.benchmark_mode, BenchmarkMode::Batch);

        if use_batch {
            use std::collections::HashMap;

            let mut adapter_files: HashMap<String, Vec<PathBuf>> = HashMap::new();

            for (fixture_path, fixture) in self.fixtures.fixtures() {
                // Skip OCR-requiring fixtures when OCR is disabled
                if !self.config.ocr_enabled && fixture.requires_ocr() {
                    continue;
                }
                for adapter in &frameworks {
                    if !adapter.supports_format(&fixture.file_type) {
                        continue;
                    }

                    let fixture_dir = fixture_path.parent().unwrap_or_else(|| std::path::Path::new("."));
                    let document_path = fixture.resolve_document_path(fixture_dir);

                    adapter_files
                        .entry(adapter.name().to_string())
                        .or_default()
                        .push(document_path);
                }
            }

            let config = self.config.clone();

            for adapter in &frameworks {
                let adapter_name = adapter.name();

                if let Some(file_paths) = adapter_files.get(adapter_name) {
                    if file_paths.is_empty() {
                        continue;
                    }

                    if adapter.supports_batch() {
                        let adapter = Arc::clone(adapter);
                        let file_paths = file_paths.clone();
                        let config = config.clone();
                        let cold_start = self.cold_start_durations.get(adapter_name).copied();

                        match Self::run_batch_iterations_static(file_paths, adapter, &config, cold_start).await {
                            Ok(mut batch_results) => {
                                // Enrich each result with framework size information
                                for result in &mut batch_results {
                                    self.enrich_with_framework_size(result);
                                }
                                results.extend(batch_results);
                            }
                            Err(e) => {
                                eprintln!("Batch benchmark task failed for {}: {}", adapter_name, e);
                            }
                        }
                    } else {
                        for file_path in file_paths {
                            let adapter = Arc::clone(adapter);
                            let file_path = file_path.clone();
                            let config = config.clone();
                            let cold_start = self.cold_start_durations.get(adapter_name).copied();

                            match Self::run_iterations_static(&file_path, adapter, &config, cold_start).await {
                                Ok(mut result) => {
                                    self.enrich_with_framework_size(&mut result);
                                    results.push(result);
                                }
                                Err(e) => {
                                    eprintln!("Benchmark task failed for {}: {}", adapter_name, e);
                                }
                            }
                        }
                    }
                }
            }
        } else {
            let mut task_queue: Vec<(PathBuf, String, Arc<dyn FrameworkAdapter>)> = Vec::new();

            for (fixture_path, fixture) in self.fixtures.fixtures() {
                // Skip OCR-requiring fixtures when OCR is disabled
                if !self.config.ocr_enabled && fixture.requires_ocr() {
                    continue;
                }
                for adapter in &frameworks {
                    if !adapter.supports_format(&fixture.file_type) {
                        continue;
                    }

                    let fixture_dir = fixture_path.parent().unwrap_or_else(|| std::path::Path::new("."));
                    let document_path = fixture.resolve_document_path(fixture_dir);

                    task_queue.push((document_path, adapter.name().to_string(), Arc::clone(adapter)));
                }
            }

            let config = self.config.clone();

            for (file_path, framework_name, adapter) in task_queue {
                let cold_start = self.cold_start_durations.get(&framework_name).copied();
                match Self::run_iterations_static(&file_path, adapter, &config, cold_start).await {
                    Ok(mut result) => {
                        self.enrich_with_framework_size(&mut result);
                        results.push(result);
                    }
                    Err(e) => {
                        eprintln!("Benchmark task failed: {}", e);
                    }
                }
            }
        }

        // Apply quality scoring if enabled
        if self.config.measure_quality {
            // Build mapping from document path -> ground truth text
            let mut ground_truth_map: HashMap<PathBuf, String> = HashMap::new();
            for (fixture_path, fixture) in self.fixtures.fixtures() {
                let fixture_dir = fixture_path.parent().unwrap_or_else(|| std::path::Path::new("."));
                if let Some(gt_path) = fixture.resolve_ground_truth_path(fixture_dir)
                    && gt_path.exists()
                    && let Ok(gt_text) = std::fs::read_to_string(&gt_path)
                {
                    let doc_path = fixture.resolve_document_path(fixture_dir);
                    ground_truth_map.insert(doc_path, gt_text);
                }
            }

            for result in &mut results {
                if let Some(ref extracted) = result.extracted_text
                    && let Some(gt_text) = ground_truth_map.get(&result.file_path)
                {
                    result.quality = Some(crate::quality::compute_quality(extracted, gt_text));
                }
            }
        }

        for adapter in &frameworks {
            adapter.teardown().await?;
        }

        Ok(results)
    }

    /// Get reference to benchmark configuration
    pub fn config(&self) -> &BenchmarkConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::NativeAdapter;

    #[tokio::test]
    async fn test_benchmark_runner_creation() {
        let config = BenchmarkConfig::default();
        let registry = AdapterRegistry::new();
        let runner = BenchmarkRunner::new(config, registry);

        assert_eq!(runner.fixture_count(), 0);
    }

    #[tokio::test]
    async fn test_run_with_no_frameworks() {
        let config = BenchmarkConfig::default();
        let registry = AdapterRegistry::new();
        let mut runner = BenchmarkRunner::new(config, registry);

        let result = runner.run(&[]).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No frameworks available"));
    }

    #[tokio::test]
    async fn test_run_with_native_adapter() {
        let config = BenchmarkConfig::default();
        let mut registry = AdapterRegistry::new();
        registry.register(Arc::new(NativeAdapter::new())).unwrap();

        let mut runner = BenchmarkRunner::new(config, registry);

        let results = runner.run(&[]).await.unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_calculate_amplified_iterations() {
        assert_eq!(calculate_amplified_iterations(100, 1000), 10);
        assert_eq!(calculate_amplified_iterations(500, 1000), 2);
        assert_eq!(calculate_amplified_iterations(2000, 1000), 1);
        assert_eq!(calculate_amplified_iterations(0, 1000), 1);
        assert_eq!(calculate_amplified_iterations(1, 1000), 1000);
    }

    #[test]
    fn test_profiling_config_optimal_frequency() {
        assert_eq!(crate::ProfilingConfig::calculate_optimal_frequency(50), 500);
        assert_eq!(crate::ProfilingConfig::calculate_optimal_frequency(99), 500);

        assert_eq!(crate::ProfilingConfig::calculate_optimal_frequency(500), 500);

        assert_eq!(crate::ProfilingConfig::calculate_optimal_frequency(1000), 500);

        assert_eq!(crate::ProfilingConfig::calculate_optimal_frequency(5000), 100);

        assert_eq!(crate::ProfilingConfig::calculate_optimal_frequency(10000), 100);
    }

    #[test]
    fn test_profiling_config_validation() {
        let mut config = crate::ProfilingConfig::default();

        assert!(config.validate().is_ok());

        config.sampling_frequency = 50;
        assert!(config.validate().is_err());

        config.sampling_frequency = 20000;
        assert!(config.validate().is_err());

        config.sampling_frequency = 1000;
        assert!(config.validate().is_ok());

        config.batch_size = 0;
        assert!(config.validate().is_err());

        config.batch_size = 10;
        assert!(config.validate().is_ok());

        config.sample_count_threshold = 0;
        assert!(config.validate().is_err());

        config.sample_count_threshold = 500;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_framework_size_enrichment_with_suffix_stripping() {
        use crate::types::{BenchmarkResult, FrameworkCapabilities, OcrStatus, PerformanceMetrics};
        use std::time::Duration;

        let config = BenchmarkConfig::default();
        let registry = AdapterRegistry::new();
        let runner = BenchmarkRunner::new(config, registry);

        // Test with -sync suffix
        let mut result_sync = BenchmarkResult {
            framework: "kreuzberg-python-sync".to_string(),
            file_path: PathBuf::from("/test/file.pdf"),
            file_size: 1024,
            success: true,
            error_message: None,
            duration: Duration::from_millis(100),
            extraction_duration: None,
            subprocess_overhead: None,
            metrics: PerformanceMetrics::default(),
            quality: None,
            iterations: vec![],
            statistics: None,
            cold_start_duration: None,
            file_extension: "pdf".to_string(),
            framework_capabilities: FrameworkCapabilities::default(),
            pdf_metadata: None,
            ocr_status: OcrStatus::Unknown,
            extracted_text: None,
        };

        // Test with -async suffix
        let mut result_async = BenchmarkResult {
            framework: "kreuzberg-python-async".to_string(),
            file_path: PathBuf::from("/test/file.pdf"),
            file_size: 1024,
            success: true,
            error_message: None,
            duration: Duration::from_millis(100),
            extraction_duration: None,
            subprocess_overhead: None,
            metrics: PerformanceMetrics::default(),
            quality: None,
            iterations: vec![],
            statistics: None,
            cold_start_duration: None,
            file_extension: "pdf".to_string(),
            framework_capabilities: FrameworkCapabilities::default(),
            pdf_metadata: None,
            ocr_status: OcrStatus::Unknown,
            extracted_text: None,
        };

        // Test with -batch suffix
        let mut result_batch = BenchmarkResult {
            framework: "kreuzberg-python-batch".to_string(),
            file_path: PathBuf::from("/test/file.pdf"),
            file_size: 1024,
            success: true,
            error_message: None,
            duration: Duration::from_millis(100),
            extraction_duration: None,
            subprocess_overhead: None,
            metrics: PerformanceMetrics::default(),
            quality: None,
            iterations: vec![],
            statistics: None,
            cold_start_duration: None,
            file_extension: "pdf".to_string(),
            framework_capabilities: FrameworkCapabilities::default(),
            pdf_metadata: None,
            ocr_status: OcrStatus::Unknown,
            extracted_text: None,
        };

        // Verify installation_size is None before enrichment
        assert!(result_sync.framework_capabilities.installation_size.is_none());
        assert!(result_async.framework_capabilities.installation_size.is_none());
        assert!(result_batch.framework_capabilities.installation_size.is_none());

        // Enrich the results
        runner.enrich_with_framework_size(&mut result_sync);
        runner.enrich_with_framework_size(&mut result_async);
        runner.enrich_with_framework_size(&mut result_batch);

        // If framework_sizes.json exists and contains kreuzberg-python, verify all variants are enriched
        if let Some(size_info) = &result_sync.framework_capabilities.installation_size {
            // All three should have the same size info from base "kreuzberg-python"
            assert_eq!(size_info.size_bytes, 15728640);
            assert_eq!(size_info.method, "pip_package");
            assert_eq!(size_info.description, "Python wheel package");

            // Verify all variants got enriched with the same data
            assert!(result_async.framework_capabilities.installation_size.is_some());
            assert!(result_batch.framework_capabilities.installation_size.is_some());

            let async_size = result_async.framework_capabilities.installation_size.as_ref().unwrap();
            let batch_size = result_batch.framework_capabilities.installation_size.as_ref().unwrap();

            assert_eq!(async_size.size_bytes, size_info.size_bytes);
            assert_eq!(batch_size.size_bytes, size_info.size_bytes);
        }
    }
}

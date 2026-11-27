//! The top-level `run` orchestration: precondition checks, warmup, dispatch to single-file or
//! native-batch execution, quality scoring, and teardown.

use crate::adapter::FrameworkAdapter;
use crate::config::{BenchmarkConfig, BenchmarkMode};
use crate::types::{BatchTimingScope, BenchmarkResult, ErrorKind, OutputFormat};
use crate::{Error, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::execution::{BatchIterationTask, SingleIterationTask};
use super::helpers::{
    ensure_batch_result_cardinality, fixed_batch_ranges, language_partitions, load_quality_ground_truth,
    validate_batch_ocr_cohort, validate_ocr_cohort, validate_timing_regime_contract,
};
use super::{BatchBenchmarkEntry, BenchmarkRunner, SingleBenchmarkTask};

/// Maximum number of missing-fixture paths listed in the fatal error before summarizing the rest.
const MISSING_FILE_SAMPLE_LIMIT: usize = 10;

fn validate_batch_capable_frameworks(use_batch: bool, frameworks: &[Arc<dyn FrameworkAdapter>]) -> Result<()> {
    if use_batch && let Some(adapter) = frameworks.iter().find(|adapter| adapter.batch_capability().is_none()) {
        return Err(Error::Config(format!(
            "framework '{}' does not expose a verified native batch API",
            adapter.name()
        )));
    }
    Ok(())
}

fn score_result_quality(
    result: &mut BenchmarkResult,
    ground_truth_map: &HashMap<PathBuf, String>,
    markdown_gt_map: &HashMap<PathBuf, String>,
    output_format: OutputFormat,
) {
    let Some(extracted) = result.extracted_text.as_ref() else {
        return;
    };
    let Some(gt_text) = ground_truth_map.get(&result.file_path) else {
        return;
    };

    let md_gt = markdown_gt_map.get(&result.file_path).map(|s| s.as_str());
    let quality = crate::quality::compute_quality_with_structure(extracted, gt_text, md_gt, output_format);

    // A result that reported success with no error yet produced zero token
    // overlap against a non-empty ground truth is not a legitimately "perfect
    // failure" quality sample — it is non-empty garbage output masquerading as a
    // successful extraction. Left as success=true, its 0.0 gets pooled into
    // quality percentiles as a genuine (if terrible) score, which inflates
    // competitor win margins instead of counting against their success rate.
    // Reclassify it as a framework-fault failure (ErrorKind::ZeroOverlap, distinct
    // from ErrorKind::EmptyContent which means the framework produced no content
    // at all) so it is excluded from quality percentiles but still counted in
    // coverage/failure stats and still contributes its timing measurements (it did
    // run to completion — see `types::is_timing_eligible`). Guarded on non-empty
    // ground truth so empty-GT fixtures are never punished. ~keep
    if result.success
        && result.error_kind == ErrorKind::None
        && !gt_text.trim().is_empty()
        && quality.f1_score_text == 0.0
    {
        result.success = false;
        result.error_kind = ErrorKind::ZeroOverlap;
        // Every success=false result must carry an error_message (enforced by
        // output.rs's result-state invariant), so record why we reclassified.
        result.error_message.get_or_insert_with(|| {
            "extraction produced no ground-truth token overlap (reclassified as zero overlap)".to_string()
        });
    }

    result.quality = Some(quality);
}

fn apply_quality_scoring(
    results: &mut [BenchmarkResult],
    ground_truth_map: &HashMap<PathBuf, String>,
    markdown_gt_map: &HashMap<PathBuf, String>,
    output_format: OutputFormat,
) {
    for result in results {
        score_result_quality(result, ground_truth_map, markdown_gt_map, output_format);
    }
}

/// Copy each fixture's declared content family onto the results for its document.
fn apply_fixture_families(results: &mut [BenchmarkResult], families: &HashMap<PathBuf, String>) {
    for result in results {
        result.fixture_family = families.get(&result.file_path).cloned();
    }
}

impl BenchmarkRunner {
    /// Declared `metadata.family` per resolved document path, the key results carry.
    fn fixture_families(&self) -> HashMap<PathBuf, String> {
        self.fixtures
            .fixtures()
            .iter()
            .filter_map(|(fixture_path, fixture)| {
                let family = fixture.family()?;
                let fixture_dir = fixture_path.parent().unwrap_or_else(|| Path::new("."));
                Some((fixture.resolve_document_path(fixture_dir), family.to_string()))
            })
            .collect()
    }

    fn validate_fixture_cohort(&self, use_batch: bool) -> Result<()> {
        let ocr_required_count = self
            .fixtures
            .fixtures()
            .iter()
            .filter(|(_, fixture)| fixture.requires_ocr())
            .count();
        validate_ocr_cohort(self.config.ocr_enabled, ocr_required_count)?;
        validate_batch_ocr_cohort(use_batch, self.fixtures.fixtures().len(), ocr_required_count)?;
        Ok(())
    }

    fn find_missing_fixture_files(&self) -> Vec<PathBuf> {
        let mut missing_files = Vec::new();
        for (fixture_path, fixture) in self.fixtures.fixtures() {
            let fixture_dir = fixture_path.parent().unwrap_or_else(|| Path::new("."));
            let document_path = fixture.resolve_document_path(fixture_dir);
            if !document_path.exists() {
                missing_files.push(document_path);
            }
        }
        missing_files
    }

    fn ensure_all_fixture_documents_exist(&self) -> Result<()> {
        let missing_files = self.find_missing_fixture_files();
        if missing_files.is_empty() {
            return Ok(());
        }
        let samples: Vec<String> = missing_files
            .iter()
            .take(MISSING_FILE_SAMPLE_LIMIT)
            .map(|p| format!("  - {}", p.display()))
            .collect();
        Err(Error::Benchmark(format!(
            "FATAL: {} fixture document(s) not found on disk. Benchmarks require all fixture files to exist.\nFirst {}:\n{}{}",
            missing_files.len(),
            samples.len(),
            samples.join("\n"),
            if missing_files.len() > MISSING_FILE_SAMPLE_LIMIT {
                format!("\n  ... and {} more", missing_files.len() - MISSING_FILE_SAMPLE_LIMIT)
            } else {
                String::new()
            }
        )))
    }

    async fn handle_warmup_failure(
        &self,
        adapter: &Arc<dyn FrameworkAdapter>,
        warmup_error: Error,
        frameworks: &[Arc<dyn FrameworkAdapter>],
    ) -> Result<()> {
        // The subject-under-test (xberg) must warm up cleanly — a warmup failure
        // there is fatal. Best-effort competitors may legitimately fail warmup on
        // inputs they cannot handle (e.g. a non-OCR tool on a scanned image returns
        // empty content); treat that as non-fatal and let the per-file results and
        // the min-success-rate gate adjudicate the cell instead of aborting it.
        if adapter.name().starts_with("xberg") {
            let teardown_error = Self::teardown_frameworks(frameworks).await.err();
            let teardown_context = teardown_error
                .map(|error| format!("; teardown also failed: {error}"))
                .unwrap_or_default();
            return Err(Error::Benchmark(format!(
                "warmup failed for '{}': {}{}",
                adapter.name(),
                warmup_error,
                teardown_context
            )));
        }
        eprintln!(
            "  Warning: warmup failed for best-effort framework '{}': {} \
             — continuing without a cold-start sample",
            adapter.name(),
            warmup_error
        );
        Ok(())
    }

    async fn warmup_single_framework(
        &mut self,
        adapter: &Arc<dyn FrameworkAdapter>,
        frameworks: &[Arc<dyn FrameworkAdapter>],
    ) -> Result<()> {
        let Some((fixture_path, fixture)) = self.fixtures.fixtures().iter().find(|(_, fixture)| {
            adapter.supports_fixture(
                &fixture.file_type,
                fixture.document.file_name().and_then(|name| name.to_str()),
                fixture.ocr_language(),
            )
        }) else {
            eprintln!(
                "  Warning: No compatible fixture found for warmup of {}",
                adapter.name()
            );
            return Ok(());
        };

        let fixture_dir = fixture_path.parent().unwrap_or_else(|| Path::new("."));
        let warmup_file = fixture.resolve_document_path(fixture_dir);

        println!("Warming up {} with {}...", adapter.name(), warmup_file.display());
        match adapter
            .warmup(&warmup_file, self.config.timeout, self.output_format)
            .await
        {
            Ok(cold_start) => {
                println!("  Cold start: {:?}", cold_start);
                self.cold_start_durations.insert(adapter.name().to_string(), cold_start);
                Ok(())
            }
            Err(warmup_error) => self.handle_warmup_failure(adapter, warmup_error, frameworks).await,
        }
    }

    async fn warmup_frameworks(&mut self, frameworks: &[Arc<dyn FrameworkAdapter>], use_batch: bool) -> Result<()> {
        for adapter in frameworks {
            if use_batch && let Some(capability) = adapter.batch_capability() {
                match capability.timing_scope {
                    BatchTimingScope::ColdEndToEndSubprocess => println!(
                        "Skipping warmup for {}: each batch invocation is measured cold end-to-end",
                        adapter.name()
                    ),
                    BatchTimingScope::WarmSteadyState => println!(
                        "Deferring warmup for {} to discarded full-batch iterations; no single-file cold-start sample is recorded",
                        adapter.name()
                    ),
                }
                continue;
            }
            self.warmup_single_framework(adapter, frameworks).await?;
        }
        Ok(())
    }

    fn collect_adapter_batch_entries(
        &self,
        frameworks: &[Arc<dyn FrameworkAdapter>],
    ) -> HashMap<String, Vec<BatchBenchmarkEntry>> {
        let mut adapter_files: HashMap<String, Vec<BatchBenchmarkEntry>> = HashMap::new();

        for (fixture_index, (fixture_path, fixture)) in self.fixtures.fixtures().iter().enumerate() {
            let force_ocr = fixture.requires_ocr();
            for adapter in frameworks {
                if !adapter.supports_fixture(
                    &fixture.file_type,
                    fixture.document.file_name().and_then(|name| name.to_str()),
                    fixture.ocr_language(),
                ) {
                    continue;
                }

                let fixture_dir = fixture_path.parent().unwrap_or_else(|| Path::new("."));
                let document_path = fixture.resolve_document_path(fixture_dir);

                adapter_files.entry(adapter.name().to_string()).or_default().push((
                    fixture_index,
                    document_path,
                    force_ocr,
                    fixture.ocr_language().map(str::to_string),
                ));
            }
        }

        adapter_files
    }

    async fn run_one_batch_range(
        &mut self,
        adapter: &Arc<dyn FrameworkAdapter>,
        batch_entries: &[BatchBenchmarkEntry],
        config: &BenchmarkConfig,
        frameworks: &[Arc<dyn FrameworkAdapter>],
        adapter_results: &mut Vec<(usize, BenchmarkResult)>,
    ) -> Result<()> {
        let adapter_name = adapter.name();
        let file_paths = batch_entries.iter().map(|(_, path, _, _)| path.clone()).collect();
        let force_ocr_flags = batch_entries.iter().map(|(_, _, force_ocr, _)| *force_ocr).collect();
        let ocr_languages = batch_entries
            .iter()
            .map(|(_, _, _, language)| language.clone())
            .collect();
        let original_indexes: Vec<usize> = batch_entries.iter().map(|(index, _, _, _)| *index).collect();
        let cold_start = self.cold_start_durations.remove(adapter_name);

        let batch_task = BatchIterationTask {
            file_paths,
            adapter: Arc::clone(adapter),
            config,
            cold_start_duration: cold_start,
            force_ocr_flags,
            ocr_languages,
            output_format: self.output_format,
        };

        match Self::run_batch_iterations_static(batch_task).await {
            Ok(batch_results) => {
                if let Err(error) =
                    ensure_batch_result_cardinality(adapter_name, original_indexes.len(), batch_results.len())
                {
                    if let Err(teardown_error) = Self::teardown_frameworks(frameworks).await {
                        eprintln!("Warning: teardown after batch cardinality failure also failed: {teardown_error}");
                    }
                    return Err(error);
                }
                for (original_index, mut result) in original_indexes.into_iter().zip(batch_results) {
                    self.enrich_with_framework_size(&mut result);
                    adapter_results.push((original_index, result));
                }
                Ok(())
            }
            Err(e) => {
                if let Err(teardown_error) = Self::teardown_frameworks(frameworks).await {
                    eprintln!("Warning: teardown after batch failure also failed: {teardown_error}");
                }
                Err(e)
            }
        }
    }

    async fn run_adapter_batches(
        &mut self,
        adapter: &Arc<dyn FrameworkAdapter>,
        entries: &[BatchBenchmarkEntry],
        config: &BenchmarkConfig,
        frameworks: &[Arc<dyn FrameworkAdapter>],
    ) -> Result<Vec<BenchmarkResult>> {
        let adapter_name = adapter.name();
        let mut adapter_results = Vec::with_capacity(entries.len());

        for partition in language_partitions(entries.to_vec(), adapter.ocr_language_policy()) {
            let ranges = fixed_batch_ranges(partition.len(), self.fixed_batch_size).map_err(|error| {
                Error::Config(format!(
                    "framework '{adapter_name}' fixed batch validation failed: {error}"
                ))
            })?;

            for range in ranges {
                self.run_one_batch_range(adapter, &partition[range], config, frameworks, &mut adapter_results)
                    .await?;
            }
        }

        adapter_results.sort_by_key(|(original_index, _)| *original_index);
        Ok(adapter_results.into_iter().map(|(_, result)| result).collect())
    }

    async fn run_batch_mode(&mut self, frameworks: &[Arc<dyn FrameworkAdapter>]) -> Result<Vec<BenchmarkResult>> {
        let adapter_files = self.collect_adapter_batch_entries(frameworks);
        let config = self.config.clone();
        let mut results = Vec::new();

        for adapter in frameworks {
            let adapter_name = adapter.name();
            let Some(entries) = adapter_files.get(adapter_name) else {
                continue;
            };
            if entries.is_empty() {
                continue;
            }

            let adapter_results = self.run_adapter_batches(adapter, entries, &config, frameworks).await?;
            results.extend(adapter_results);
        }

        Ok(results)
    }

    fn build_single_task_queue(&self, frameworks: &[Arc<dyn FrameworkAdapter>]) -> Vec<SingleBenchmarkTask> {
        let mut task_queue: Vec<SingleBenchmarkTask> = Vec::new();

        for (fixture_path, fixture) in self.fixtures.fixtures() {
            let force_ocr = fixture.requires_ocr();
            for adapter in frameworks {
                if !adapter.supports_fixture(
                    &fixture.file_type,
                    fixture.document.file_name().and_then(|name| name.to_str()),
                    fixture.ocr_language(),
                ) {
                    continue;
                }

                let fixture_dir = fixture_path.parent().unwrap_or_else(|| Path::new("."));
                let document_path = fixture.resolve_document_path(fixture_dir);

                task_queue.push((
                    document_path,
                    adapter.name().to_string(),
                    Arc::clone(adapter),
                    force_ocr,
                    fixture.ocr_language().map(str::to_string),
                ));
            }
        }

        task_queue
    }

    async fn run_single_mode(&mut self, frameworks: &[Arc<dyn FrameworkAdapter>]) -> Result<Vec<BenchmarkResult>> {
        let task_queue = self.build_single_task_queue(frameworks);
        let config = self.config.clone();
        let mut results = Vec::new();

        for (file_path, framework_name, adapter, force_ocr, ocr_language) in task_queue {
            let cold_start = self.cold_start_durations.remove(&framework_name);
            let task = SingleIterationTask {
                file_path: &file_path,
                adapter,
                config: &config,
                cold_start_duration: cold_start,
                force_ocr,
                ocr_language: ocr_language.as_deref(),
                output_format: self.output_format,
            };

            match Self::run_iterations_static(task).await {
                Ok(mut result) => {
                    self.enrich_with_framework_size(&mut result);
                    results.push(result);
                }
                Err(task_error) => {
                    // A missing row would make downstream coverage look better than the
                    // eligible corpus actually was, so task-level harness errors abort the
                    // run instead of being silently omitted. ~keep
                    let teardown_error = Self::teardown_frameworks(frameworks).await.err();
                    let teardown_context = teardown_error
                        .map(|error| format!("; teardown also failed: {error}"))
                        .unwrap_or_default();
                    return Err(Error::Benchmark(format!(
                        "benchmark task failed for '{framework_name}' on {}: {task_error}{teardown_context}",
                        file_path.display()
                    )));
                }
            }
        }

        Ok(results)
    }

    /// Run benchmarks for specified frameworks
    ///
    /// # Arguments
    /// * `framework_names` - Names of frameworks to benchmark (empty = all registered)
    ///
    /// # Returns
    /// Vector of benchmark results
    pub async fn run(&mut self, framework_names: &[String]) -> Result<Vec<BenchmarkResult>> {
        let frameworks = self.select_frameworks(framework_names)?;

        if frameworks.is_empty() {
            return Err(Error::Benchmark("No frameworks available for benchmarking".to_string()));
        }

        let use_batch = matches!(self.config.benchmark_mode, BenchmarkMode::Batch);
        validate_batch_capable_frameworks(use_batch, &frameworks)?;
        validate_timing_regime_contract(
            frameworks.iter().map(|adapter| adapter.timing_regime()),
            self.config.warmup_iterations,
        )?;

        self.validate_fixture_cohort(use_batch)?;
        self.ensure_all_fixture_documents_exist()?;

        let quality_ground_truth = if self.config.measure_quality {
            Some(load_quality_ground_truth(&self.fixtures)?)
        } else {
            None
        };

        Self::setup_frameworks(&frameworks).await?;

        self.warmup_frameworks(&frameworks, use_batch).await?;

        let mut results = if use_batch {
            self.run_batch_mode(&frameworks).await?
        } else {
            self.run_single_mode(&frameworks).await?
        };

        apply_fixture_families(&mut results, &self.fixture_families());

        if let Some((ground_truth_map, markdown_gt_map)) = quality_ground_truth {
            apply_quality_scoring(&mut results, &ground_truth_map, &markdown_gt_map, self.output_format);
        }

        Self::teardown_frameworks(&frameworks).await?;

        Ok(results)
    }
}

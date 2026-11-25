//! Native-batch execution: building the batch subprocess command (including the liteparse
//! fallback path), mapping batch output into per-file `BenchmarkResult`s, and the
//! implementation `FrameworkAdapter::extract_batch` delegates to.

use crate::monitoring::ResourceStats;
use crate::types::{
    BatchCapability, BatchEntryPoint, BenchmarkResult, ErrorKind, FrameworkCapabilities, OcrStatus, OutputFormat,
};
use crate::{Error, Result};
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

use super::SubprocessAdapter;
use super::ocr_args::{build_batch_file_configs, effective_ocr_config_from_args};
use super::support::{
    ParsedBatchOutput, SubprocessExecution, bytes_per_second, detect_pdf_page_count, error_to_error_kind,
    parse_batch_output, validate_batch_item,
};

fn xberg_source_index(item: &serde_json::Value, position: usize) -> Result<usize> {
    item.get("metadata")
        .and_then(|metadata| metadata.get("additional"))
        .and_then(|additional| additional.get("source_index"))
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| {
            Error::Benchmark(format!(
                "Xberg batch result at position {position} is missing a valid metadata.additional.source_index"
            ))
        })
}

/// Align Xberg's discovery-ordered results and separately indexed errors to request order.
/// Every input must have exactly one outcome. Multi-document results fail closed because the CLI
/// envelope has no per-child timings from which to build one honest performance row. ~keep
fn align_xberg_batch_output(parsed: &mut ParsedBatchOutput, input_count: usize) -> Result<()> {
    let mut results_by_input: Vec<Vec<serde_json::Value>> = vec![Vec::new(); input_count];
    for (position, item) in parsed.items.drain(..).enumerate() {
        let source_index = xberg_source_index(&item, position)?;
        let Some(slot) = results_by_input.get_mut(source_index) else {
            return Err(Error::Benchmark(format!(
                "Xberg batch result at position {position} has out-of-range source_index {source_index} for {input_count} inputs"
            )));
        };
        slot.push(item);
    }

    let mut errors_by_input = vec![None; input_count];
    for error in parsed.errors.drain(..) {
        let Some(slot) = errors_by_input.get_mut(error.index) else {
            return Err(Error::Benchmark(format!(
                "Xberg batch error has out-of-range input index {} for {input_count} inputs",
                error.index
            )));
        };
        if slot.replace(error.message).is_some() {
            return Err(Error::Benchmark(format!(
                "Xberg batch returned multiple errors for input index {}",
                error.index
            )));
        }
    }

    for input_index in 0..input_count {
        if results_by_input[input_index].is_empty() && errors_by_input[input_index].is_none() {
            return Err(Error::Benchmark(format!(
                "Xberg batch returned no result or error for input index {input_index}"
            )));
        }
        if !results_by_input[input_index].is_empty() && errors_by_input[input_index].is_some() {
            return Err(Error::Benchmark(format!(
                "Xberg batch returned both results and an error for input index {input_index}"
            )));
        }
    }

    let mut aligned = Vec::with_capacity(input_count);
    for input_index in 0..input_count {
        if let Some(message) = errors_by_input[input_index].take() {
            aligned.push(serde_json::json!({ "error": message }));
            continue;
        }
        let documents = &mut results_by_input[input_index];
        if documents.len() != 1 {
            return Err(Error::Benchmark(format!(
                "Xberg batch returned multiple results for input index {input_index}; the envelope cannot aggregate per-child timing and metadata truthfully"
            )));
        }
        aligned.push(documents.pop().expect("cardinality checked above"));
    }
    parsed.items = aligned;
    Ok(())
}

/// The subprocess-level outcome of a native-batch invocation, plus the request-scoped
/// values needed to turn it into per-file `BenchmarkResult`s. Grouped to keep
/// `build_batch_results_from_parsed_output`'s signature under the crate's parameter-count
/// limit. ~keep
struct BatchOutcome {
    parsed_batch: ParsedBatchOutput,
    duration: Duration,
    resource_stats: ResourceStats,
    error: Option<Error>,
    batch_capability: BatchCapability,
    batch_sample_id: String,
    batch_force_ocr: bool,
    output_format: OutputFormat,
}

/// Values shared by every item in a batch result set, borrowed rather than cloned per item.
struct BatchResultShared<'a> {
    output_format: OutputFormat,
    resource_stats: &'a ResourceStats,
    framework_capabilities: &'a FrameworkCapabilities,
    batch_makespan: Duration,
    batch_subprocess_overhead: Option<Duration>,
    batch_throughput: f64,
}

/// Per-item values that vary across a batch result set.
struct BatchItemInputs {
    ocr_status: OcrStatus,
    extraction_duration: Option<Duration>,
    validation: (bool, Option<String>, ErrorKind),
    is_throughput_anchor: bool,
    extracted_text: Option<String>,
}

impl SubprocessAdapter {
    fn is_xberg_cli_batch(&self) -> bool {
        self.batch_capability
            .is_some_and(|capability| capability.entry_point == BatchEntryPoint::XbergCliExtractBatch)
    }

    /// Write the per-file OCR overrides (if any) to a temp file and point `--file-configs`
    /// at it. The returned handle must be kept alive until the subprocess has been spawned
    /// (it is a `NamedTempFile`, deleted on drop), so the caller holds it across the
    /// `execute_measured_command` call. ~keep
    fn apply_xberg_batch_file_configs(
        &self,
        cmd: &mut Command,
        file_paths: &[&Path],
        ocr_languages: &[Option<String>],
        request_args: &[String],
    ) -> Result<Option<tempfile::NamedTempFile>> {
        if !self.is_xberg_cli_batch() {
            return Ok(None);
        }
        let cwd = std::env::current_dir().map_err(Error::Io)?;
        let base_ocr = effective_ocr_config_from_args(request_args);
        let configs = build_batch_file_configs(file_paths, ocr_languages, &cwd, base_ocr.as_ref());
        if configs.is_empty() {
            return Ok(None);
        }
        let mut file = tempfile::NamedTempFile::new().map_err(Error::Io)?;
        serde_json::to_writer(file.as_file_mut(), &configs)?;
        cmd.arg("--file-configs").arg(file.path());
        Ok(Some(file))
    }

    fn apply_xberg_batch_concurrency_args(&self, cmd: &mut Command) {
        if !self.is_xberg_cli_batch() {
            return;
        }
        let batch_workers = self.batch_workers.to_string();
        let max_threads = self.effective_xberg_max_threads().to_string();
        cmd.arg("--max-concurrent")
            .arg(batch_workers)
            .arg("--max-threads")
            .arg(max_threads);
    }

    /// Execute batch extraction subprocess with multiple files
    async fn execute_subprocess_batch(
        &self,
        file_paths: &[&Path],
        timeout: Duration,
        force_ocr: bool,
        ocr_languages: &[Option<String>],
        output_format: OutputFormat,
    ) -> Result<SubprocessExecution> {
        if self
            .batch_capability
            .is_some_and(|capability| capability.entry_point == BatchEntryPoint::LiteparseBatchParse)
        {
            return self
                .execute_liteparse_native_batch(file_paths, timeout, force_ocr, output_format)
                .await;
        }

        let mut cmd = Self::measured_command(&self.command);
        if let Some(dir) = &self.working_dir {
            cmd.current_dir(dir);
        }
        let request_args = self.request_args(force_ocr);
        cmd.args(&request_args);
        if let Some(language_arg) = self.batch_ocr_language_forward_arg(ocr_languages)? {
            cmd.arg(language_arg);
        }

        let file_configs = self.apply_xberg_batch_file_configs(&mut cmd, file_paths, ocr_languages, &request_args)?;
        self.apply_xberg_batch_concurrency_args(&mut cmd);

        if self.format_aware {
            cmd.arg(format!("--format={}", output_format));
        }

        let cwd = std::env::current_dir().map_err(Error::Io)?;
        for path in file_paths {
            let absolute_path = if path.is_absolute() {
                path.to_path_buf()
            } else {
                cwd.join(path)
            };
            cmd.arg(&*absolute_path.to_string_lossy());
        }

        for (key, value) in &self.env {
            cmd.env(key, value);
        }

        Self::configure_measured_stdin(&mut cmd);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        Self::configure_child_process(&mut cmd);

        let total_file_size = file_paths
            .iter()
            .filter_map(|path| std::fs::metadata(path).ok())
            .map(|metadata| metadata.len())
            .sum();
        let sampling_ms = crate::monitoring::adaptive_sampling_interval_ms(total_file_size);
        let measured = Self::execute_measured_command(
            &mut cmd,
            timeout,
            "batch subprocess",
            Duration::from_millis(sampling_ms),
        )
        .await?;
        drop(file_configs);
        Ok(Self::finish_measured_command(measured, "Batch subprocess"))
    }

    pub(super) fn stage_liteparse_input(source: &Path, destination: &Path) -> Result<()> {
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(source, destination).map_err(|error| {
                Error::Benchmark(format!(
                    "Failed to stage LiteParse input {} at {} using a symlink: {}",
                    source.display(),
                    destination.display(),
                    error
                ))
            })
        }

        #[cfg(windows)]
        {
            if std::fs::hard_link(source, destination).is_ok() {
                return Ok(());
            }

            std::fs::copy(source, destination).map(|_| ()).map_err(|error| {
                Error::Benchmark(format!(
                    "Failed to stage LiteParse input {} at {} using a hard link or copy: {}",
                    source.display(),
                    destination.display(),
                    error
                ))
            })
        }

        #[cfg(not(any(unix, windows)))]
        {
            std::fs::copy(source, destination).map(|_| ()).map_err(|error| {
                Error::Benchmark(format!(
                    "Failed to stage LiteParse input {} at {} using a copy: {}",
                    source.display(),
                    destination.display(),
                    error
                ))
            })
        }
    }

    fn stage_liteparse_inputs(file_paths: &[&Path], input_dir: &Path) -> Result<()> {
        for (idx, path) in file_paths.iter().enumerate() {
            let file_name = path
                .file_name()
                .ok_or_else(|| Error::Benchmark("Invalid file path".to_string()))?;

            let src_absolute = if path.is_absolute() {
                path.to_path_buf()
            } else {
                std::env::current_dir().map_err(Error::Io)?.join(path)
            };

            let staged_name = format!("{}_{}", idx, file_name.to_string_lossy());
            let dest_link = input_dir.join(staged_name);
            Self::stage_liteparse_input(&src_absolute, &dest_link)?;
        }
        Ok(())
    }

    fn collect_liteparse_batch_results(
        file_paths: &[&Path],
        output_dir: &Path,
        output_format: OutputFormat,
    ) -> Result<Vec<serde_json::Value>> {
        let preferred_exts: [&str; 2] = match output_format {
            OutputFormat::Markdown => ["md", "markdown"],
            OutputFormat::Plaintext => ["txt", "text"],
        };
        let produced: Vec<(String, std::path::PathBuf)> = std::fs::read_dir(output_dir)
            .map_err(|e| Error::Benchmark(format!("Failed to read lit output dir {}: {}", output_dir.display(), e)))?
            .filter_map(|entry| entry.ok())
            .map(|entry| (entry.file_name().to_string_lossy().into_owned(), entry.path()))
            .collect();

        let mut results = Vec::new();
        for (idx, _path) in file_paths.iter().enumerate() {
            let prefix = format!("{idx}_");
            let matches: Vec<&(String, std::path::PathBuf)> =
                produced.iter().filter(|(name, _)| name.starts_with(&prefix)).collect();
            let hit = matches
                .iter()
                .find(|(name, _)| preferred_exts.iter().any(|e| name.ends_with(&format!(".{e}"))))
                .or_else(|| matches.first());

            match hit {
                Some((_, output_path)) => {
                    let content = std::fs::read_to_string(output_path).map_err(|e| {
                        Error::Benchmark(format!("Failed to read lit output {}: {}", output_path.display(), e))
                    })?;
                    results.push(serde_json::json!({
                        "content": content,
                        "metadata": {
                            "framework": "liteparse",
                            "output_format": output_format.to_string()
                        }
                    }));
                }
                None => {
                    let listing: Vec<&String> = produced.iter().map(|(name, _)| name).collect();
                    return Err(Error::Benchmark(format!(
                        "lit batch-parse produced no output for input #{idx} (prefix '{prefix}'). \
                         Output dir {} contains {} file(s): {:?}",
                        output_dir.display(),
                        produced.len(),
                        listing
                    )));
                }
            }
        }
        Ok(results)
    }

    /// Execute liteparse native batch using lit batch-parse
    /// Uses lit batch-parse with temp directories for optimal apples-to-apples comparison
    async fn execute_liteparse_native_batch(
        &self,
        file_paths: &[&Path],
        timeout: Duration,
        force_ocr: bool,
        output_format: OutputFormat,
    ) -> Result<SubprocessExecution> {
        use std::fs;
        let temp_dir =
            tempfile::tempdir().map_err(|e| Error::Benchmark(format!("Failed to create temp directory: {}", e)))?;
        let input_dir = temp_dir.path().join("input");
        let output_dir = temp_dir.path().join("output");

        fs::create_dir(&input_dir).map_err(|e| Error::Benchmark(format!("Failed to create input directory: {}", e)))?;
        fs::create_dir(&output_dir)
            .map_err(|e| Error::Benchmark(format!("Failed to create output directory: {}", e)))?;

        Self::stage_liteparse_inputs(file_paths, &input_dir)?;

        let format_arg = match output_format {
            OutputFormat::Markdown => "markdown",
            OutputFormat::Plaintext => "text",
        };
        let disable_ocr = !force_ocr && self.args.iter().any(|arg| arg == "--no-ocr");
        let args = self.liteparse_batch_args(
            input_dir.to_string_lossy().into_owned(),
            output_dir.to_string_lossy().into_owned(),
            format_arg,
            disable_ocr,
        );

        let mut cmd = Self::measured_command(self.liteparse_batch_command());
        cmd.args(args);

        Self::configure_measured_stdin(&mut cmd);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        Self::configure_child_process(&mut cmd);
        // Staging is harness setup, not framework work. Start the measured
        // interval only after tempdir creation and input symlinks are complete. ~keep
        let total_file_size = file_paths
            .iter()
            .filter_map(|path| fs::metadata(path).ok())
            .map(|metadata| metadata.len())
            .sum();
        let sampling_ms = crate::monitoring::adaptive_sampling_interval_ms(total_file_size);
        let measured =
            Self::execute_measured_command(&mut cmd, timeout, "lit batch-parse", Duration::from_millis(sampling_ms))
                .await?;
        let mut execution = Self::finish_measured_command(measured, "lit batch-parse");
        if execution.error.is_some() {
            return Ok(execution);
        }

        let results = Self::collect_liteparse_batch_results(file_paths, &output_dir, output_format)?;
        let stdout = serde_json::to_string(&results)
            .map_err(|e| Error::Benchmark(format!("Failed to serialize results: {}", e)))?;
        execution.stdout = stdout;
        Ok(execution)
    }

    fn validate_batch_capability_and_cardinality(
        &self,
        file_paths: &[&Path],
        force_ocr: &[bool],
        ocr_languages: &[Option<String>],
    ) -> Result<BatchCapability> {
        let batch_capability = self.batch_capability.ok_or_else(|| {
            Error::Config(format!(
                "framework '{}' does not expose a verified native batch API",
                self.name
            ))
        })?;
        if force_ocr.len() != file_paths.len() {
            return Err(Error::Benchmark(format!(
                "batch force_ocr cardinality mismatch: received {} flags for {} files",
                force_ocr.len(),
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

    fn resolve_homogeneous_batch_force_ocr(force_ocr: &[bool]) -> Result<bool> {
        let batch_force_ocr = force_ocr.first().copied().unwrap_or(false);
        if force_ocr.iter().any(|flag| *flag != batch_force_ocr) {
            return Err(Error::Config(
                "native batch extraction requires a homogeneous OCR cohort; select fixtures/shard with either all \
                 force-OCR or all non-force-OCR documents"
                    .to_string(),
            ));
        }
        Ok(batch_force_ocr)
    }

    /// Build a failure `BenchmarkResult` for every file from a single process-level error
    /// (used both when the batch output can't be parsed at all, and when every item's own
    /// validation already failed regardless of the process error). ~keep
    fn build_batch_process_failure_results(
        &self,
        file_paths: &[&Path],
        duration: Duration,
        resource_stats: &crate::monitoring::ResourceStats,
        process_error: &Error,
        output_format: OutputFormat,
    ) -> Vec<BenchmarkResult> {
        file_paths
            .iter()
            .map(|file_path| {
                let file_size = std::fs::metadata(file_path).map_or(0, |metadata| metadata.len());
                self.build_failure_result(
                    file_path,
                    file_size,
                    duration,
                    resource_stats,
                    process_error,
                    output_format,
                )
            })
            .collect()
    }

    /// Fails every otherwise-successful item whose document reports a failed layout pass.
    fn reject_layout_failure_items(
        &self,
        items: &[serde_json::Value],
        validations: &mut [(bool, Option<String>, ErrorKind)],
    ) {
        if !self.reject_layout_failures {
            return;
        }
        for (item, validation) in items.iter().zip(validations.iter_mut()) {
            if !validation.0 {
                continue;
            }
            if let Some(message) = crate::adapters::layout_failures::layout_failure_in_item(item) {
                validation.0 = false;
                validation.1 = Some(message);
                validation.2 = ErrorKind::LayoutFailed;
            }
        }
    }

    /// For items whose own validation didn't already report a failure, attribute the batch's
    /// process-level error to them too (e.g. a non-zero exit after some items already
    /// succeeded). ~keep
    fn apply_process_error_to_unvalidated_items(
        items: &[serde_json::Value],
        validations: &mut [(bool, Option<String>, ErrorKind)],
        process_error: &Error,
    ) {
        let process_error_kind = error_to_error_kind(process_error);
        let process_error_message = process_error.to_string();
        for (item, validation) in items.iter().zip(validations.iter_mut()) {
            let has_explicit_error = item
                .get("error")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|message| !message.is_empty());
            if !validation.0 && !has_explicit_error {
                validation.1 = Some(process_error_message.clone());
                validation.2 = process_error_kind;
            }
        }
    }

    fn validate_per_item_timing_capability(
        &self,
        batch_capability: BatchCapability,
        per_file_durations: &[Option<Duration>],
        validations: &[(bool, Option<String>, ErrorKind)],
    ) -> Result<()> {
        if batch_capability.per_item_timing {
            if per_file_durations
                .iter()
                .zip(validations)
                .any(|(duration, validation)| validation.0 && duration.is_none())
            {
                return Err(Error::Benchmark(format!(
                    "framework '{}' declares per-item batch timing but returned unavailable timing for a successful item",
                    self.name
                )));
            }
        } else if per_file_durations.iter().any(Option::is_some) {
            return Err(Error::Benchmark(format!(
                "framework '{}' declares per-item batch timing unavailable but returned numeric timing values",
                self.name
            )));
        }
        Ok(())
    }

    fn batch_ocr_statuses_and_contents(
        &self,
        items: &[serde_json::Value],
        batch_force_ocr: bool,
    ) -> (Vec<OcrStatus>, Vec<Option<String>>) {
        let statuses = items
            .iter()
            .map(|item| {
                self.resolve_ocr_status(
                    item.get("_ocr_used")
                        .or_else(|| item.get("metadata").and_then(|metadata| metadata.get("ocr_used"))),
                    batch_force_ocr,
                )
            })
            .collect();
        let contents = items
            .iter()
            .map(|item| item.get("content").and_then(|value| value.as_str()).map(str::to_string))
            .collect();
        (statuses, contents)
    }

    fn build_batch_item_result(
        &self,
        file_path: &Path,
        shared: &BatchResultShared<'_>,
        item: BatchItemInputs,
    ) -> BenchmarkResult {
        let file_size = std::fs::metadata(file_path).map(|m| m.len()).unwrap_or(0);
        let file_extension = file_path.extension().and_then(|e| e.to_str()).unwrap_or("").to_string();

        let mut item_capabilities = shared.framework_capabilities.clone();
        item_capabilities.batch_performance_sample = Some(item.is_throughput_anchor);

        let pdf_metadata = if file_extension.eq_ignore_ascii_case("pdf") {
            Some(crate::types::PdfMetadata {
                has_text_layer: false,
                detection_method: "unknown".to_string(),
                page_count: detect_pdf_page_count(file_path),
                ocr_enabled: item.ocr_status == OcrStatus::Used,
                text_quality_score: None,
            })
        } else {
            None
        };

        let (item_success, item_error, item_error_kind) = item.validation;

        BenchmarkResult {
            framework: self.name.clone(),
            output_format: shared.output_format,
            file_path: file_path.to_path_buf(),
            file_size,
            success: item_success,
            error_message: item_error,
            error_kind: item_error_kind,
            duration: shared.batch_makespan,
            extraction_duration: item.extraction_duration,
            subprocess_overhead: shared.batch_subprocess_overhead,
            // Every sibling carries the same process sample so each
            // reporting bucket can recover it; aggregation deduplicates
            // by `batch_sample_id`. ~keep
            metrics: Self::measured_metrics(shared.resource_stats, None, shared.batch_throughput),
            quality: None,
            iterations: vec![],
            statistics: None,
            cold_start_duration: None,
            file_extension,
            framework_capabilities: item_capabilities,
            pdf_metadata,
            ocr_status: item.ocr_status,
            extracted_text: item.extracted_text,
            system_load: None,
        }
    }

    fn build_batch_item_results(
        &self,
        file_paths: &[&Path],
        parsed_batch: &ParsedBatchOutput,
        validations: &[(bool, Option<String>, ErrorKind)],
        batch_ocr_statuses: &[OcrStatus],
        batch_contents: &[Option<String>],
        shared: &BatchResultShared<'_>,
    ) -> Vec<BenchmarkResult> {
        let throughput_anchor = validations.iter().position(|validation| validation.0);
        file_paths
            .iter()
            .enumerate()
            .map(|(idx, file_path)| {
                let (item_success, item_error, item_error_kind) = validations.get(idx).cloned().unwrap_or((
                    false,
                    Some("Missing validation for batch item".to_string()),
                    ErrorKind::HarnessError,
                ));
                self.build_batch_item_result(
                    file_path,
                    shared,
                    BatchItemInputs {
                        ocr_status: batch_ocr_statuses.get(idx).copied().unwrap_or(OcrStatus::Unknown),
                        extraction_duration: parsed_batch.per_file_durations[idx],
                        validation: (item_success, item_error, item_error_kind),
                        is_throughput_anchor: throughput_anchor == Some(idx),
                        extracted_text: batch_contents.get(idx).cloned().flatten(),
                    },
                )
            })
            .collect()
    }

    fn validate_parsed_batch_cardinality(file_paths: &[&Path], parsed_batch: &ParsedBatchOutput) -> Result<()> {
        if parsed_batch.items.len() != file_paths.len() {
            return Err(Error::Benchmark(format!(
                "batch output cardinality mismatch: received {} results for {} files",
                parsed_batch.items.len(),
                file_paths.len()
            )));
        }
        if parsed_batch.per_file_durations.len() != file_paths.len() {
            return Err(Error::Benchmark(format!(
                "batch timing cardinality mismatch: received {} per-file durations for {} files",
                parsed_batch.per_file_durations.len(),
                file_paths.len()
            )));
        }
        Ok(())
    }

    fn compute_batch_throughput(
        file_paths: &[&Path],
        validations: &[(bool, Option<String>, ErrorKind)],
        batch_makespan: Duration,
    ) -> f64 {
        let successful_bytes: u64 = file_paths
            .iter()
            .zip(validations)
            .filter(|(_, validation)| validation.0)
            .filter_map(|(path, _)| std::fs::metadata(path).ok().map(|metadata| metadata.len()))
            .sum();
        bytes_per_second(successful_bytes, batch_makespan)
    }

    fn build_batch_framework_capabilities(&self, batch_sample_id: String) -> FrameworkCapabilities {
        FrameworkCapabilities {
            timing_regime: crate::types::TimingRegime::ColdProcess,
            resource_measurement_scope: crate::types::ResourceMeasurementScope::IsolatedProcess,
            supported_extensions: self.supported_formats.clone(),
            ocr_support: Self::framework_supports_ocr(&self.name),
            batch_support: self.batch_capability.is_some(),
            batch_capability: self.batch_capability,
            batch_sample_id: Some(batch_sample_id),
            ..Default::default()
        }
    }

    fn build_batch_results_from_parsed_output(
        &self,
        file_paths: &[&Path],
        outcome: BatchOutcome,
    ) -> Result<Vec<BenchmarkResult>> {
        let BatchOutcome {
            mut parsed_batch,
            duration,
            resource_stats,
            error,
            batch_capability,
            batch_sample_id,
            batch_force_ocr,
            output_format,
        } = outcome;

        if batch_capability.entry_point == BatchEntryPoint::XbergCliExtractBatch {
            align_xberg_batch_output(&mut parsed_batch, file_paths.len())?;
        }
        Self::validate_parsed_batch_cardinality(file_paths, &parsed_batch)?;
        let mut batch_validations: Vec<(bool, Option<String>, ErrorKind)> =
            parsed_batch.items.iter().map(validate_batch_item).collect();
        self.reject_layout_failure_items(&parsed_batch.items, &mut batch_validations);
        if let Some(process_error) = error.as_ref() {
            Self::apply_process_error_to_unvalidated_items(&parsed_batch.items, &mut batch_validations, process_error);
        }

        self.validate_per_item_timing_capability(
            batch_capability,
            &parsed_batch.per_file_durations,
            &batch_validations,
        )?;

        // Use the slower of process-wall time and an adapter-reported batch
        // makespan. This consumes Xberg's `total_ms` without allowing a
        // self-reported inner timer to inflate cross-framework throughput. ~keep
        let batch_makespan = parsed_batch
            .reported_total_duration
            .map_or(duration, |reported| duration.max(reported));
        let batch_subprocess_overhead = parsed_batch
            .reported_total_duration
            .map(|reported| duration.saturating_sub(reported));

        let (batch_ocr_statuses, batch_contents) =
            self.batch_ocr_statuses_and_contents(&parsed_batch.items, batch_force_ocr);

        if let Some(process_error) = error.as_ref()
            && batch_validations.iter().all(|validation| validation.0)
        {
            return Ok(self.build_batch_process_failure_results(
                file_paths,
                duration,
                &resource_stats,
                process_error,
                output_format,
            ));
        }

        let batch_throughput = Self::compute_batch_throughput(file_paths, &batch_validations, batch_makespan);
        let framework_capabilities = self.build_batch_framework_capabilities(batch_sample_id);

        let shared = BatchResultShared {
            output_format,
            resource_stats: &resource_stats,
            framework_capabilities: &framework_capabilities,
            batch_makespan,
            batch_subprocess_overhead,
            batch_throughput,
        };

        Ok(self.build_batch_item_results(
            file_paths,
            &parsed_batch,
            &batch_validations,
            &batch_ocr_statuses,
            &batch_contents,
            &shared,
        ))
    }

    /// Implements `FrameworkAdapter::extract_batch`; the trait method itself lives in
    /// `adapter_impl.rs` (a type may have only one trait impl block per crate) and
    /// delegates here so the batch execution logic stays with its helpers. ~keep
    pub(super) async fn extract_batch_impl(
        &self,
        file_paths: &[&Path],
        timeout: Duration,
        force_ocr: &[bool],
        ocr_languages: &[Option<String>],
        output_format: OutputFormat,
    ) -> Result<Vec<BenchmarkResult>> {
        let batch_capability = self.validate_batch_capability_and_cardinality(file_paths, force_ocr, ocr_languages)?;
        if file_paths.is_empty() {
            return Ok(Vec::new());
        }

        let batch_force_ocr = Self::resolve_homogeneous_batch_force_ocr(force_ocr)?;
        let batch_sample_id = self.batch_sample_id(file_paths, batch_force_ocr, output_format);

        let timeout = self
            .effective_timeout(timeout)
            .checked_mul(file_paths.len() as u32)
            .unwrap_or(Duration::MAX);

        let execution = self
            .execute_subprocess_batch(file_paths, timeout, batch_force_ocr, ocr_languages, output_format)
            .await?;
        let SubprocessExecution {
            stdout,
            duration,
            resource_stats,
            error,
            ..
        } = execution;
        let parsed_batch = match parse_batch_output(&stdout) {
            Ok(parsed_batch) => parsed_batch,
            Err(parse_error) => {
                let Some(process_error) = error.as_ref() else {
                    return Err(parse_error);
                };
                return Ok(self.build_batch_process_failure_results(
                    file_paths,
                    duration,
                    &resource_stats,
                    process_error,
                    output_format,
                ));
            }
        };

        self.build_batch_results_from_parsed_output(
            file_paths,
            BatchOutcome {
                parsed_batch,
                duration,
                resource_stats,
                error,
                batch_capability,
                batch_sample_id,
                batch_force_ocr,
                output_format,
            },
        )
    }
}

//! Subprocess-based adapter for language bindings
//!
//! This adapter provides a base for running extraction via subprocess.
//! It's used by Python, Node.js, and Ruby adapters to execute extraction
//! in separate processes while monitoring resource usage.

use crate::adapter::FrameworkAdapter;
use crate::monitoring::ResourceMonitor;
use crate::types::{BenchmarkResult, ErrorKind, FrameworkCapabilities, OcrStatus, OutputFormat, PerformanceMetrics};
use crate::{Error, Result};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

/// Extract JSON content from raw stdout, stripping non-JSON prefix lines.
///
/// Some runtimes (notably Elixir's BEAM VM) emit log messages to stdout
/// during module initialization before the script can redirect them. This
/// function finds the earliest `[` or `{` character and returns everything
/// from that point, ignoring any preceding log lines. Whichever delimiter
/// appears first wins — must not bias toward `[` because object outputs
/// (e.g. xberg-cli's envelope) contain nested arrays.
fn extract_json_from_stdout(raw: &str) -> &str {
    let bracket = raw.find('[');
    let brace = raw.find('{');
    let pos = match (bracket, brace) {
        (Some(b), Some(c)) => Some(b.min(c)),
        (Some(b), None) => Some(b),
        (None, Some(c)) => Some(c),
        (None, None) => None,
    };
    match pos {
        Some(p) => &raw[p..],
        None => raw,
    }
}

/// Map a harness `Error` to the appropriate `ErrorKind`.
fn error_to_error_kind(e: &Error) -> ErrorKind {
    match e {
        Error::Timeout(_) => ErrorKind::Timeout,
        Error::FrameworkError(_) => ErrorKind::FrameworkError,
        Error::EmptyContent(_) => ErrorKind::EmptyContent,
        _ => ErrorKind::HarnessError,
    }
}
use tokio::process::Command;

/// Minimum duration in seconds for a valid throughput calculation.
/// Durations below this threshold produce unreliable throughput values
/// and will result in throughput being set to 0.0 (filtered in aggregation).
const MIN_VALID_DURATION_SECS: f64 = 0.000_001; // 1 microsecond

/// Check if verbose benchmark debugging is enabled via BENCHMARK_DEBUG env var.
fn is_debug_enabled() -> bool {
    std::env::var("BENCHMARK_DEBUG").is_ok()
}

/// Base adapter for subprocess-based extraction
///
/// This adapter spawns a subprocess to perform extraction and monitors
/// its resource usage. Subclasses implement the specific command construction
/// for each language binding.
pub struct SubprocessAdapter {
    name: String,
    command: PathBuf,
    args: Vec<String>,
    env: Vec<(String, String)>,
    supports_batch: bool,
    working_dir: Option<PathBuf>,
    supported_formats: Vec<String>,
    max_timeout: Option<Duration>,
    skip_files: Vec<String>,
    /// When true, append --format=<output_format> to subprocess args
    format_aware: bool,
    /// When true (and format_aware=true), adapter uses native batch API via lit batch-parse
    use_native_batch: bool,
}

impl SubprocessAdapter {
    /// Determine if a framework supports OCR based on its name
    ///
    /// Known frameworks with OCR support:
    /// - xberg-* (all Xberg bindings support OCR)
    /// - pymupdf (supports OCR via tesseract)
    ///
    /// Frameworks without OCR support:
    /// - liteparse
    /// - Other basic PDF parsers
    fn framework_supports_ocr(framework_name: &str) -> bool {
        let name_lower = framework_name.to_lowercase();

        // Xberg bindings all support OCR
        if name_lower.starts_with("xberg-") || name_lower == "xberg" {
            return true;
        }

        // PyMuPDF supports OCR via tesseract
        if name_lower.contains("pymupdf") {
            return true;
        }

        // Docling supports OCR via external OCR backends
        if name_lower.contains("docling") {
            return true;
        }

        // Unstructured supports OCR via Tesseract
        if name_lower.contains("unstructured") {
            return true;
        }

        // Tika supports OCR via Tika OCR parser
        if name_lower.contains("tika") {
            return true;
        }

        // MinerU supports OCR via PaddleOCR
        if name_lower.contains("mineru") {
            return true;
        }

        // Most other frameworks don't support OCR
        false
    }

    /// Create a new subprocess adapter
    ///
    /// # Arguments
    /// * `name` - Framework name (e.g., "xberg-python")
    /// * `command` - Path to executable (e.g., "python3", "node")
    /// * `args` - Base arguments (e.g., ["-m", "xberg"])
    /// * `env` - Environment variables
    /// * `supported_formats` - List of file extensions this framework can process (e.g., ["pdf", "docx"])
    pub fn new(
        name: impl Into<String>,
        command: impl Into<PathBuf>,
        args: Vec<String>,
        env: Vec<(String, String)>,
        supported_formats: Vec<String>,
    ) -> Self {
        Self {
            name: name.into(),
            command: command.into(),
            args,
            env,
            supports_batch: false,
            working_dir: None,
            supported_formats,
            max_timeout: None,
            skip_files: vec![],
            format_aware: false,
            use_native_batch: false,
        }
    }

    /// Create a new subprocess adapter with batch support
    ///
    /// This adapter will call `extract_batch()` with all files at once,
    /// allowing the subprocess to use its native batch API for parallel processing.
    ///
    /// # Arguments
    /// * `name` - Framework name (e.g., "xberg-python-batch")
    /// * `command` - Path to executable (e.g., "python3", "node")
    /// * `args` - Base arguments (e.g., ["-m", "xberg"])
    /// * `env` - Environment variables
    /// * `supported_formats` - List of file extensions this framework can process
    pub fn with_batch_support(
        name: impl Into<String>,
        command: impl Into<PathBuf>,
        args: Vec<String>,
        env: Vec<(String, String)>,
        supported_formats: Vec<String>,
    ) -> Self {
        Self {
            name: name.into(),
            command: command.into(),
            args,
            env,
            supports_batch: true,
            working_dir: None,
            supported_formats,
            max_timeout: None,
            skip_files: vec![],
            format_aware: false,
            use_native_batch: false,
        }
    }

    /// Set a maximum timeout for this adapter, overriding the global config timeout
    /// if the adapter's max is lower.
    pub fn with_max_timeout(mut self, timeout: Duration) -> Self {
        self.max_timeout = Some(timeout);
        self
    }

    /// Set files to skip for this adapter.
    pub fn with_skip_files(mut self, files: Vec<String>) -> Self {
        self.skip_files = files;
        self
    }

    /// Enable format awareness: append --format=<output_format> to subprocess args
    pub fn with_format_aware(mut self, enabled: bool) -> Self {
        self.format_aware = enabled;
        self
    }

    /// Enable native batch mode: use framework's native batch API instead of looping extract()
    /// Only meaningful when format_aware=true and supports_batch=true
    pub fn with_native_batch(mut self, enabled: bool) -> Self {
        self.use_native_batch = enabled;
        self
    }

    /// Get the effective timeout, clamped by the adapter's max_timeout if set.
    fn effective_timeout(&self, timeout: Duration) -> Duration {
        match self.max_timeout {
            Some(max) => timeout.min(max),
            None => timeout,
        }
    }

    /// Set the working directory for subprocess execution
    ///
    /// # Arguments
    /// * `dir` - Directory path to change to before running the command
    pub fn set_working_dir(&mut self, dir: PathBuf) {
        self.working_dir = Some(dir);
    }

    /// Execute the extraction subprocess
    async fn execute_subprocess(
        &self,
        file_path: &Path,
        timeout: Duration,
        output_format: OutputFormat,
    ) -> Result<(String, String, Duration)> {
        let start = Instant::now();

        let absolute_path = if file_path.is_absolute() {
            file_path.to_path_buf()
        } else {
            std::env::current_dir().map_err(Error::Io)?.join(file_path)
        };

        let mut cmd = Command::new(&self.command);
        if let Some(dir) = &self.working_dir {
            cmd.current_dir(dir);
        }
        cmd.args(&self.args);

        // Append format flag if format_aware is enabled
        if self.format_aware {
            cmd.arg(format!("--format={}", output_format));
        }

        cmd.arg(&*absolute_path.to_string_lossy());

        for (key, value) in &self.env {
            cmd.env(key, value);
        }

        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let child = cmd.spawn().map_err(|e| {
            Error::Benchmark(format!(
                "Failed to spawn subprocess '{}' with args {:?}: {}",
                self.command.display(),
                self.args,
                e
            ))
        })?;

        let output = match tokio::time::timeout(timeout, child.wait_with_output()).await {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => {
                return Err(Error::Benchmark(format!("Failed to wait for subprocess: {}", e)));
            }
            Err(_) => {
                return Err(Error::Timeout(format!("Subprocess exceeded {:?}", timeout)));
            }
        };

        let duration = start.elapsed();

        let raw_stdout = String::from_utf8_lossy(&output.stdout);
        let stdout = extract_json_from_stdout(&raw_stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success() {
            let mut error_msg = format!("Subprocess failed with exit code {:?}", output.status.code());
            if !stderr.is_empty() {
                error_msg.push_str(&format!("\nstderr: {}", stderr));
            }
            if !stdout.is_empty() && stdout.len() < 500 {
                error_msg.push_str(&format!("\nstdout: {}", stdout));
            }
            return Err(Error::Benchmark(error_msg));
        }

        Ok((stdout, stderr, duration))
    }

    /// Execute batch extraction subprocess with multiple files
    async fn execute_subprocess_batch(
        &self,
        file_paths: &[&Path],
        timeout: Duration,
        output_format: OutputFormat,
    ) -> Result<(String, String, Duration)> {
        let start = Instant::now();

        // For liteparse with native batch, use lit batch-parse instead of shell wrapper
        if self.use_native_batch && self.format_aware {
            return self
                .execute_liteparse_native_batch(file_paths, timeout, output_format)
                .await;
        }

        let mut cmd = Command::new(&self.command);
        if let Some(dir) = &self.working_dir {
            cmd.current_dir(dir);
        }
        cmd.args(&self.args);

        // Append format flag if format_aware is enabled
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

        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let child = cmd
            .spawn()
            .map_err(|e| Error::Benchmark(format!("Failed to spawn batch subprocess: {}", e)))?;

        let output = match tokio::time::timeout(timeout, child.wait_with_output()).await {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => {
                return Err(Error::Benchmark(format!("Failed to wait for batch subprocess: {}", e)));
            }
            Err(_) => {
                return Err(Error::Timeout(format!("Batch subprocess exceeded {:?}", timeout)));
            }
        };

        let duration = start.elapsed();

        let raw_stdout = String::from_utf8_lossy(&output.stdout);
        let stdout = extract_json_from_stdout(&raw_stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success() {
            return Err(Error::Benchmark(format!(
                "Batch subprocess failed with exit code {:?}\nstderr: {}",
                output.status.code(),
                stderr
            )));
        }

        Ok((stdout, stderr, duration))
    }

    /// Execute liteparse native batch using lit batch-parse
    /// Uses lit batch-parse with temp directories for optimal apples-to-apples comparison
    async fn execute_liteparse_native_batch(
        &self,
        file_paths: &[&Path],
        timeout: Duration,
        output_format: OutputFormat,
    ) -> Result<(String, String, Duration)> {
        use std::fs;
        use std::os::unix::fs as unix_fs;

        let start = Instant::now();

        // Create temp input/output directories
        let temp_dir =
            tempfile::tempdir().map_err(|e| Error::Benchmark(format!("Failed to create temp directory: {}", e)))?;
        let input_dir = temp_dir.path().join("input");
        let output_dir = temp_dir.path().join("output");

        fs::create_dir(&input_dir).map_err(|e| Error::Benchmark(format!("Failed to create input directory: {}", e)))?;
        fs::create_dir(&output_dir)
            .map_err(|e| Error::Benchmark(format!("Failed to create output directory: {}", e)))?;

        // Symlink all input files into the input directory to avoid copying large files.
        // Prefix each link with its batch index so fixtures that share a basename
        // (across categories) do not collide in the flat staging directory.
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
            unix_fs::symlink(&src_absolute, &dest_link)
                .map_err(|e| Error::Benchmark(format!("Failed to symlink file {}: {}", idx, e)))?;
        }

        // Build lit batch-parse command
        let format_arg = match output_format {
            OutputFormat::Markdown => "markdown",
            OutputFormat::Plaintext => "text",
        };

        let mut cmd = Command::new("lit");
        cmd.arg("batch-parse")
            .arg(&input_dir)
            .arg(&output_dir)
            .arg("--format")
            .arg(format_arg)
            .arg("--num-workers")
            .arg("4")
            .arg("--quiet");

        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let child = cmd
            .spawn()
            .map_err(|e| Error::Benchmark(format!("Failed to spawn lit batch-parse: {}", e)))?;

        let output = match tokio::time::timeout(timeout, child.wait_with_output()).await {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => {
                return Err(Error::Benchmark(format!("Failed to wait for lit batch-parse: {}", e)));
            }
            Err(_) => {
                return Err(Error::Timeout(format!("lit batch-parse exceeded {:?}", timeout)));
            }
        };

        let duration = start.elapsed();

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::Benchmark(format!(
                "lit batch-parse failed with exit code {:?}\nstderr: {}",
                output.status.code(),
                stderr
            )));
        }

        // Read output files and construct per-file JSON results, in the same order
        // as `file_paths` so the caller can zip results back to inputs by index.
        let mut results = Vec::new();
        for (idx, path) in file_paths.iter().enumerate() {
            let file_name = path
                .file_name()
                .ok_or_else(|| Error::Benchmark("Invalid file path".to_string()))?
                .to_string_lossy();

            // lit batch-parse mirrors the input filename, replacing the last
            // extension: `<idx>_<stem>.pdf` → `<idx>_<stem>.{txt|md}`.
            let ext = match output_format {
                OutputFormat::Markdown => "md",
                OutputFormat::Plaintext => "txt",
            };

            // Stem = everything before the LAST '.', matching liteparse's
            // `Path::with_extension` semantics (e.g. "a.b.pdf" → "a.b").
            let file_stem = file_name.rsplit_once('.').map(|(stem, _)| stem).unwrap_or(&file_name);
            let output_file_name = format!("{}_{}.{}", idx, file_stem, ext);
            let output_file_path = output_dir.join(&output_file_name);

            // Read the extracted content
            let content = fs::read_to_string(&output_file_path).unwrap_or_else(|_| String::new());

            // Build per-file JSON result in the expected format
            let result_json = serde_json::json!({
                "content": content,
                "metadata": {
                    "framework": "liteparse",
                    "output_format": output_format.to_string()
                }
            });

            results.push(result_json);
        }

        let stdout = serde_json::to_string(&results)
            .map_err(|e| Error::Benchmark(format!("Failed to serialize results: {}", e)))?;
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok((stdout, stderr, duration))
    }

    /// Execute extraction via persistent subprocess (stdin/stdout protocol)
    /// Build a failure `BenchmarkResult` for error paths in `extract()`.
    ///
    /// Centralises the repeated pattern of constructing an error result with
    /// resource statistics, throughput, and framework capabilities.
    fn build_failure_result(
        &self,
        file_path: &Path,
        file_size: u64,
        duration: Duration,
        resource_stats: &crate::monitoring::ResourceStats,
        error: &Error,
        output_format: OutputFormat,
    ) -> BenchmarkResult {
        let throughput = if duration.as_secs_f64() > 0.0 {
            file_size as f64 / duration.as_secs_f64()
        } else {
            0.0
        };

        let framework_capabilities = FrameworkCapabilities {
            ocr_support: Self::framework_supports_ocr(&self.name),
            batch_support: self.supports_batch,
            ..Default::default()
        };

        let error_kind = error_to_error_kind(error);

        BenchmarkResult {
            framework: self.name.clone(),
            output_format,
            file_path: file_path.to_path_buf(),
            file_size,
            success: false,
            error_message: Some(error.to_string()),
            error_kind,
            duration,
            extraction_duration: None,
            subprocess_overhead: None,
            metrics: PerformanceMetrics {
                peak_memory_bytes: resource_stats.peak_memory_bytes,
                avg_cpu_percent: resource_stats.avg_cpu_percent,
                throughput_bytes_per_sec: throughput,
                p50_memory_bytes: resource_stats.p50_memory_bytes,
                p95_memory_bytes: resource_stats.p95_memory_bytes,
                p99_memory_bytes: resource_stats.p99_memory_bytes,
            },
            quality: None,
            iterations: vec![],
            statistics: None,
            cold_start_duration: None,
            file_extension: file_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("unknown")
                .to_lowercase(),
            framework_capabilities,
            pdf_metadata: None,
            ocr_status: OcrStatus::Unknown,
            extracted_text: None,
        }
    }

    /// Parse extraction result from subprocess output
    ///
    /// Expected subprocess output format:
    /// ```json
    /// {
    ///   "content": "extracted text...",          // REQUIRED
    ///   "_ocr_used": true|false,                 // optional
    ///   "_extraction_time_ms": 123.45            // optional
    /// }
    /// ```
    fn parse_output(&self, stdout: &str) -> Result<serde_json::Value> {
        if is_debug_enabled() {
            let preview = if stdout.len() > 300 {
                // Find a valid UTF-8 char boundary at or before byte 300
                let end = (0..=300).rev().find(|&i| stdout.is_char_boundary(i)).unwrap_or(0);
                format!("{}...[{} bytes total]", &stdout[..end], stdout.len())
            } else {
                stdout.to_string()
            };
            eprintln!(
                "[parse_output:{}] raw_len={} preview={}",
                self.name,
                stdout.len(),
                preview.trim()
            );
        }

        let raw: serde_json::Value = serde_json::from_str(stdout)
            .map_err(|e| Error::Benchmark(format!("Failed to parse subprocess output as JSON: {}", e)))?;

        // Validate that output is a JSON object
        if !raw.is_object() {
            return Err(Error::Benchmark(
                "Subprocess output must be a JSON object with 'content' field".to_string(),
            ));
        }

        // xberg-cli envelope shape: {result: {content, metadata, ...}, extraction_time_ms: f64}.
        // Unwrap to the flat shape competitors emit so downstream parsing is uniform.
        let parsed = if let Some(inner) = raw.get("result").filter(|v| v.is_object()) {
            let mut flat = inner.clone();
            if let (Some(obj), Some(t)) = (flat.as_object_mut(), raw.get("extraction_time_ms")) {
                obj.insert("_extraction_time_ms".to_string(), t.clone());
            }
            if let (Some(obj), Some(meta)) = (flat.as_object_mut(), inner.get("metadata"))
                && let Some(ocr) = meta.get("ocr_used")
            {
                obj.insert("_ocr_used".to_string(), ocr.clone());
            }
            flat
        } else {
            raw
        };

        // Check if the framework reported an error
        if let Some(error_val) = parsed.get("error") {
            let error_msg = error_val.as_str().unwrap_or("unknown error");
            if !error_msg.is_empty() {
                // Detect Python-side extraction timeouts (from multiprocessing fork
                // timeout handler) and classify them as Timeout rather than FrameworkError.
                if error_msg.contains("timed out") {
                    return Err(Error::Timeout(error_msg.to_string()));
                }
                return Err(Error::FrameworkError(error_msg.to_string()));
            }
        }

        if !parsed.get("content").is_some_and(|v| v.is_string()) {
            // Check if this is a framework returning empty for unsupported format
            // (e.g. {"error": "", "_extraction_time_ms": 0} with no content field)
            let extraction_time = parsed
                .get("_extraction_time_ms")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            if extraction_time == 0.0 {
                return Err(Error::EmptyContent(
                    "No content extracted (unsupported format or empty result)".to_string(),
                ));
            }
            return Err(Error::Benchmark(
                "Subprocess output missing required 'content' field (must be a string)".to_string(),
            ));
        }

        // Check for empty/whitespace-only content
        let content_str = parsed["content"].as_str().unwrap(); // safe: is_string() checked above
        if content_str.trim().is_empty() {
            return Err(Error::EmptyContent("Framework returned empty content".to_string()));
        }

        Ok(parsed)
    }
}

#[async_trait]
impl FrameworkAdapter for SubprocessAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn supports_format(&self, file_type: &str) -> bool {
        let file_type_lower = file_type.to_lowercase();
        self.supported_formats
            .iter()
            .any(|fmt| fmt.to_lowercase() == file_type_lower)
    }

    fn should_skip_file(&self, file_name: &str) -> bool {
        self.skip_files.iter().any(|f| f == file_name)
    }

    fn supported_output_formats(&self) -> Vec<OutputFormat> {
        vec![OutputFormat::Plaintext, OutputFormat::Markdown]
    }

    async fn extract(
        &self,
        file_path: &Path,
        timeout: Duration,
        _force_ocr: bool,
        output_format: OutputFormat,
    ) -> Result<BenchmarkResult> {
        let timeout = self.effective_timeout(timeout);
        let file_size = std::fs::metadata(file_path).map_err(Error::Io)?.len();

        let start_time = std::time::Instant::now();
        let monitor = ResourceMonitor::new();
        let sampling_ms = crate::monitoring::adaptive_sampling_interval_ms(file_size);
        monitor.start(Duration::from_millis(sampling_ms)).await;

        let (stdout, _stderr, duration) = match self.execute_subprocess(file_path, timeout, output_format).await {
            Ok(result) => result,
            Err(e) => {
                let samples = monitor.stop().await;
                let snapshots = monitor.get_snapshots().await;
                let baseline = monitor.baseline_memory().await;
                let resource_stats = ResourceMonitor::calculate_stats(&samples, &snapshots, baseline);
                let actual_duration = start_time.elapsed();
                return Ok(self.build_failure_result(
                    file_path,
                    file_size,
                    actual_duration,
                    &resource_stats,
                    &e,
                    output_format,
                ));
            }
        };

        // Take a post-extraction snapshot before stopping the monitor.
        // This provides a fallback memory measurement for sub-millisecond extractions
        // where the background sampler may not have collected any samples.
        let post_sample = monitor.snapshot_current_memory();
        let mut samples = monitor.stop().await;
        if samples.is_empty() {
            samples.push(post_sample);
        }
        let snapshots = monitor.get_snapshots().await;
        let baseline = monitor.baseline_memory().await;
        let resource_stats = ResourceMonitor::calculate_stats(&samples, &snapshots, baseline);

        let parsed = match self.parse_output(&stdout) {
            Ok(value) => value,
            Err(e) => {
                return Ok(self.build_failure_result(
                    file_path,
                    file_size,
                    duration,
                    &resource_stats,
                    &e,
                    output_format,
                ));
            }
        };

        let extraction_time_raw = parsed.get("_extraction_time_ms");
        if is_debug_enabled() {
            eprintln!(
                "[extract:{}] _extraction_time_ms raw={:?}, keys={:?}",
                self.name,
                extraction_time_raw,
                parsed.as_object().map(|o| o.keys().collect::<Vec<_>>())
            );
        }

        let extraction_duration = extraction_time_raw
            .and_then(|v| v.as_f64())
            .map(|ms| Duration::from_secs_f64(ms / 1000.0));

        // Capture extracted text for quality assessment
        let extracted_text = parsed.get("content").and_then(|v| v.as_str()).map(|s| s.to_string());

        let subprocess_overhead = extraction_duration.map(|ext| duration.saturating_sub(ext));

        // Use extraction_duration for throughput when available (more accurate for persistent mode
        // where `duration` is just I/O roundtrip). Fall back to wall-clock `duration`.
        let effective_duration = extraction_duration.unwrap_or(duration);
        let throughput = if effective_duration.as_secs_f64() >= MIN_VALID_DURATION_SECS {
            file_size as f64 / effective_duration.as_secs_f64()
        } else {
            0.0 // Below minimum threshold - will be filtered in aggregation
        };

        // Prefer self-reported memory from the extraction script over external monitoring.
        // External monitoring via ResourceMonitor often misses subprocess memory for fast
        // extractions (<10ms) because the subprocess exits before the sampler captures it.
        // Scripts report _peak_memory_bytes via resource.getrusage or equivalent.
        let self_reported_memory = parsed.get("_peak_memory_bytes").and_then(|v| v.as_u64());

        let metrics = if let Some(reported_mem) = self_reported_memory {
            PerformanceMetrics {
                peak_memory_bytes: reported_mem,
                avg_cpu_percent: resource_stats.avg_cpu_percent,
                throughput_bytes_per_sec: throughput,
                p50_memory_bytes: reported_mem,
                p95_memory_bytes: reported_mem,
                p99_memory_bytes: reported_mem,
            }
        } else {
            PerformanceMetrics {
                peak_memory_bytes: resource_stats.peak_memory_bytes,
                avg_cpu_percent: resource_stats.avg_cpu_percent,
                throughput_bytes_per_sec: throughput,
                p50_memory_bytes: resource_stats.p50_memory_bytes,
                p95_memory_bytes: resource_stats.p95_memory_bytes,
                p99_memory_bytes: resource_stats.p99_memory_bytes,
            }
        };

        // Check if subprocess reported OCR usage
        let ocr_status = parsed
            .get("_ocr_used")
            .and_then(|v| v.as_bool())
            .map(|used| if used { OcrStatus::Used } else { OcrStatus::NotUsed })
            .unwrap_or(OcrStatus::Unknown);

        // Build framework capabilities
        let framework_capabilities = FrameworkCapabilities {
            ocr_support: Self::framework_supports_ocr(&self.name),
            batch_support: self.supports_batch,
            ..Default::default()
        };

        // Build PDF metadata if this is a PDF file
        let pdf_metadata = if file_path.extension().and_then(|e| e.to_str()) == Some("pdf") {
            Some(crate::types::PdfMetadata {
                has_text_layer: false, // Unknown from subprocess
                detection_method: "unknown".to_string(),
                page_count: None,
                ocr_enabled: ocr_status == OcrStatus::Used,
                text_quality_score: None,
            })
        } else {
            None
        };

        Ok(BenchmarkResult {
            framework: self.name.clone(),
            output_format,
            file_path: file_path.to_path_buf(),
            file_size,
            success: true,
            error_message: None,
            error_kind: ErrorKind::None,
            duration,
            extraction_duration,
            subprocess_overhead,
            metrics,
            quality: None,
            iterations: vec![],
            statistics: None,
            cold_start_duration: None,
            file_extension: file_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("unknown")
                .to_lowercase(),
            framework_capabilities,
            pdf_metadata,
            ocr_status,
            extracted_text,
        })
    }

    fn version(&self) -> String {
        "unknown".to_string()
    }

    fn supports_batch(&self) -> bool {
        self.supports_batch
    }

    async fn extract_batch(
        &self,
        file_paths: &[&Path],
        timeout: Duration,
        force_ocr: &[bool],
        output_format: OutputFormat,
    ) -> Result<Vec<BenchmarkResult>> {
        let timeout = self.effective_timeout(timeout);
        // Early return if file_paths is empty
        if file_paths.is_empty() {
            return Ok(Vec::new());
        }

        if !self.supports_batch {
            let mut results = Vec::new();
            for (i, path) in file_paths.iter().enumerate() {
                let fo = force_ocr.get(i).copied().unwrap_or(false);
                results.push(self.extract(path, timeout, fo, output_format).await?);
            }
            return Ok(results);
        }

        let total_file_size: u64 = file_paths
            .iter()
            .filter_map(|p| std::fs::metadata(p).ok().map(|m| m.len()))
            .sum();

        let start_time = std::time::Instant::now();
        let monitor = ResourceMonitor::new();
        let sampling_ms = crate::monitoring::adaptive_sampling_interval_ms(total_file_size);
        monitor.start(Duration::from_millis(sampling_ms)).await;

        let (stdout, _stderr, duration) = match self.execute_subprocess_batch(file_paths, timeout, output_format).await
        {
            Ok(result) => result,
            Err(e) => {
                let samples = monitor.stop().await;
                let snapshots = monitor.get_snapshots().await;
                let baseline = monitor.baseline_memory().await;
                let resource_stats = ResourceMonitor::calculate_stats(&samples, &snapshots, baseline);
                let actual_duration = start_time.elapsed();

                // Create one failure result per file instead of a single aggregated failure
                // Use the actual elapsed time divided by number of files
                let num_files = file_paths.len() as f64;
                // Amortized per-file duration: total batch wall time divided by file count.
                // For concurrent batch processing, this represents average cost, not individual file duration.
                let avg_duration_per_file = Duration::from_secs_f64(actual_duration.as_secs_f64() / num_files.max(1.0));

                let framework_capabilities = FrameworkCapabilities {
                    ocr_support: Self::framework_supports_ocr(&self.name),
                    batch_support: self.supports_batch,
                    ..Default::default()
                };

                let error_kind = error_to_error_kind(&e);
                let failure_results: Vec<BenchmarkResult> = file_paths
                    .iter()
                    .map(|file_path| {
                        let file_size = std::fs::metadata(file_path).map(|m| m.len()).unwrap_or(0);
                        let file_extension = file_path
                            .extension()
                            .and_then(|ext| ext.to_str())
                            .unwrap_or("")
                            .to_string();

                        let throughput = if avg_duration_per_file.as_secs_f64() > 0.0 {
                            file_size as f64 / avg_duration_per_file.as_secs_f64()
                        } else {
                            0.0
                        };

                        BenchmarkResult {
                            framework: self.name.clone(),
                            output_format,
                            file_path: file_path.to_path_buf(),
                            file_size,
                            success: false,
                            error_message: Some(e.to_string()),
                            error_kind,
                            duration: avg_duration_per_file,
                            extraction_duration: None,
                            subprocess_overhead: None,
                            metrics: PerformanceMetrics {
                                peak_memory_bytes: resource_stats.peak_memory_bytes,
                                avg_cpu_percent: resource_stats.avg_cpu_percent,
                                throughput_bytes_per_sec: throughput,
                                p50_memory_bytes: resource_stats.p50_memory_bytes,
                                p95_memory_bytes: resource_stats.p95_memory_bytes,
                                p99_memory_bytes: resource_stats.p99_memory_bytes,
                            },
                            quality: None,
                            iterations: vec![],
                            statistics: None,
                            cold_start_duration: None,
                            file_extension,
                            framework_capabilities: framework_capabilities.clone(),
                            pdf_metadata: None,
                            ocr_status: OcrStatus::Unknown,
                            extracted_text: None,
                        }
                    })
                    .collect();

                return Ok(failure_results);
            }
        };

        // Take a post-extraction snapshot as fallback for fast batch operations
        let post_sample = monitor.snapshot_current_memory();
        let mut samples = monitor.stop().await;
        if samples.is_empty() {
            samples.push(post_sample);
        }
        let snapshots = monitor.get_snapshots().await;
        let baseline = monitor.baseline_memory().await;
        let resource_stats = ResourceMonitor::calculate_stats(&samples, &snapshots, baseline);

        // Parse batch output to extract per-file OCR status and extraction times
        // Try to parse as JSON array; fall back to single object wrapped in array
        let parsed_batch: Option<Vec<serde_json::Value>> = serde_json::from_str::<Vec<serde_json::Value>>(&stdout)
            .ok()
            .or_else(|| {
                // Some adapters return a single object for 1-file batches
                serde_json::from_str::<serde_json::Value>(&stdout).ok().map(|v| vec![v])
            });

        let batch_ocr_statuses: Vec<OcrStatus> = parsed_batch
            .as_ref()
            .map(|results| {
                results
                    .iter()
                    .map(|item| {
                        item.get("_ocr_used")
                            .and_then(|v| v.as_bool())
                            .map(|used| if used { OcrStatus::Used } else { OcrStatus::NotUsed })
                            .unwrap_or(OcrStatus::Unknown)
                    })
                    .collect()
            })
            .unwrap_or_else(|| vec![OcrStatus::Unknown; file_paths.len()]);

        // Extract per-file extraction times from batch JSON results
        let batch_extraction_times: Vec<Option<Duration>> = parsed_batch
            .as_ref()
            .map(|results| {
                results
                    .iter()
                    .map(|item| {
                        item.get("_extraction_time_ms")
                            .and_then(|v| v.as_f64())
                            .map(|ms| Duration::from_secs_f64(ms / 1000.0))
                    })
                    .collect()
            })
            .unwrap_or_else(|| vec![None; file_paths.len()]);

        // Extract per-file content from batch JSON results for quality assessment
        let batch_contents: Vec<Option<String>> = parsed_batch
            .as_ref()
            .map(|results| {
                results
                    .iter()
                    .map(|item| item.get("content").and_then(|v| v.as_str()).map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_else(|| vec![None; file_paths.len()]);

        // Validate per-item success/error, mirroring single-file parse_output logic
        let batch_validations: Vec<(bool, Option<String>, ErrorKind)> = parsed_batch
            .as_ref()
            .map(|results| {
                results
                    .iter()
                    .map(|item| {
                        // Check if the framework reported an error for this item
                        if let Some(error_val) = item.get("error") {
                            let error_msg = error_val.as_str().unwrap_or("unknown error");
                            if !error_msg.is_empty() {
                                let kind = if error_msg.contains("timed out") {
                                    ErrorKind::Timeout
                                } else {
                                    ErrorKind::FrameworkError
                                };
                                return (false, Some(error_msg.to_string()), kind);
                            }
                        }
                        // Check for missing or non-string content
                        match item.get("content").and_then(|v| v.as_str()) {
                            Some(s) if !s.trim().is_empty() => (true, None, ErrorKind::None),
                            Some(_) => (
                                false,
                                Some("Framework returned empty content".to_string()),
                                ErrorKind::EmptyContent,
                            ),
                            None => (
                                false,
                                Some("No content extracted (unsupported format or empty result)".to_string()),
                                ErrorKind::EmptyContent,
                            ),
                        }
                    })
                    .collect()
            })
            .unwrap_or_else(|| {
                vec![
                    (
                        false,
                        Some("Failed to parse batch output".to_string()),
                        ErrorKind::HarnessError
                    );
                    file_paths.len()
                ]
            });

        // Create one result per file instead of a single aggregated result
        // Since batch processing doesn't give us per-file timing, we use average duration
        let num_files = file_paths.len() as f64;
        let avg_duration_per_file = Duration::from_secs_f64(duration.as_secs_f64() / num_files.max(1.0));

        let framework_capabilities = FrameworkCapabilities {
            ocr_support: Self::framework_supports_ocr(&self.name),
            batch_support: self.supports_batch,
            ..Default::default()
        };

        let results: Vec<BenchmarkResult> = file_paths
            .iter()
            .enumerate()
            .map(|(idx, file_path)| {
                let file_size = std::fs::metadata(file_path).map(|m| m.len()).unwrap_or(0);

                let file_extension = file_path.extension().and_then(|e| e.to_str()).unwrap_or("").to_string();

                // Use per-file OCR status if available, otherwise Unknown
                let ocr_status = batch_ocr_statuses.get(idx).copied().unwrap_or(OcrStatus::Unknown);

                // Use per-file extraction time if available from batch JSON
                let extraction_duration = batch_extraction_times.get(idx).copied().flatten();

                // Prefer per-file extraction time for accurate throughput, fall back to averaged duration
                let effective_duration = extraction_duration.unwrap_or(avg_duration_per_file);
                let file_throughput = if effective_duration.as_secs_f64() >= MIN_VALID_DURATION_SECS {
                    file_size as f64 / effective_duration.as_secs_f64()
                } else {
                    0.0 // Below minimum threshold - will be filtered in aggregation
                };
                let subprocess_overhead = extraction_duration.map(|ext| avg_duration_per_file.saturating_sub(ext));

                // Amortize batch memory proportionally by file size
                let file_fraction = if total_file_size > 0 {
                    file_size as f64 / total_file_size as f64
                } else {
                    1.0 / file_paths.len() as f64
                };

                let (item_success, item_error, item_error_kind) = batch_validations.get(idx).cloned().unwrap_or((
                    false,
                    Some("Missing validation for batch item".to_string()),
                    ErrorKind::HarnessError,
                ));

                BenchmarkResult {
                    framework: self.name.clone(),
                    output_format,
                    file_path: file_path.to_path_buf(),
                    file_size,
                    success: item_success,
                    error_message: item_error,
                    error_kind: item_error_kind,
                    duration: avg_duration_per_file,
                    extraction_duration,
                    subprocess_overhead,
                    metrics: PerformanceMetrics {
                        peak_memory_bytes: (resource_stats.peak_memory_bytes as f64 * file_fraction) as u64,
                        avg_cpu_percent: resource_stats.avg_cpu_percent,
                        throughput_bytes_per_sec: file_throughput,
                        p50_memory_bytes: (resource_stats.p50_memory_bytes as f64 * file_fraction) as u64,
                        p95_memory_bytes: (resource_stats.p95_memory_bytes as f64 * file_fraction) as u64,
                        p99_memory_bytes: (resource_stats.p99_memory_bytes as f64 * file_fraction) as u64,
                    },
                    quality: None,
                    iterations: vec![],
                    statistics: None,
                    cold_start_duration: None,
                    file_extension,
                    framework_capabilities: framework_capabilities.clone(),
                    pdf_metadata: None,
                    ocr_status,
                    extracted_text: batch_contents.get(idx).cloned().flatten(),
                }
            })
            .collect();

        Ok(results)
    }

    async fn setup(&self) -> Result<()> {
        which::which(&self.command)
            .map_err(|e| Error::Benchmark(format!("Command '{}' not found: {}", self.command.display(), e)))?;
        Ok(())
    }

    async fn teardown(&self) -> Result<()> {
        Ok(())
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            peak_memory_bytes: 0,
            avg_cpu_percent: 0.0,
            throughput_bytes_per_sec: 0.0,
            p50_memory_bytes: 0,
            p95_memory_bytes: 0,
            p99_memory_bytes: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subprocess_adapter_creation() {
        let adapter = SubprocessAdapter::new(
            "test-adapter",
            "echo",
            vec!["test".to_string()],
            vec![],
            vec!["pdf".to_string(), "docx".to_string()],
        );
        assert_eq!(adapter.name(), "test-adapter");
    }

    #[test]
    fn test_supports_format() {
        let adapter = SubprocessAdapter::new(
            "test",
            "echo",
            vec![],
            vec![],
            vec!["pdf".to_string(), "docx".to_string()],
        );
        assert!(adapter.supports_format("pdf"));
        assert!(adapter.supports_format("docx"));
        assert!(!adapter.supports_format("unknown"));
    }

    #[test]
    fn test_parse_output_empty_error_no_content() {
        // {"error": "", "_extraction_time_ms": 0} → EmptyContent (unsupported format)
        let adapter = SubprocessAdapter::new("test", "echo", vec![], vec![], vec!["pdf".to_string()]);
        let output = r#"{"error": "", "_extraction_time_ms": 0}"#;
        let result = adapter.parse_output(output);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, Error::EmptyContent(_)),
            "Expected EmptyContent, got: {:?}",
            err
        );
        assert!(err.to_string().contains("No content extracted"));
    }

    #[test]
    fn test_parse_output_nonempty_error() {
        // {"error": "something went wrong"} → FrameworkError
        let adapter = SubprocessAdapter::new("test", "echo", vec![], vec![], vec!["pdf".to_string()]);
        let output = r#"{"error": "something went wrong"}"#;
        let result = adapter.parse_output(output);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, Error::FrameworkError(_)),
            "Expected FrameworkError, got: {:?}",
            err
        );
        assert!(err.to_string().contains("something went wrong"));
    }

    #[test]
    fn test_parse_output_valid_content() {
        // Valid output with content field
        let adapter = SubprocessAdapter::new("test", "echo", vec![], vec![], vec!["pdf".to_string()]);
        let output = r#"{"content": "Hello, world!", "_extraction_time_ms": 42.5}"#;
        let result = adapter.parse_output(output);
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed["content"], "Hello, world!");
        assert_eq!(parsed["_extraction_time_ms"], 42.5);
    }

    #[test]
    fn test_parse_output_missing_content_nonzero_time() {
        // Missing content with nonzero extraction time → Benchmark error (harness bug)
        let adapter = SubprocessAdapter::new("test", "echo", vec![], vec![], vec!["pdf".to_string()]);
        let output = r#"{"_extraction_time_ms": 150.0}"#;
        let result = adapter.parse_output(output);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, Error::Benchmark(_)),
            "Expected Benchmark error, got: {:?}",
            err
        );
        assert!(err.to_string().contains("missing required 'content' field"));
    }

    #[test]
    fn test_max_timeout_clamps_config_timeout() {
        let adapter = SubprocessAdapter::new("test", "echo", vec![], vec![], vec!["pdf".to_string()])
            .with_max_timeout(Duration::from_secs(120));
        // Config timeout (900s) should be clamped to max (120s)
        let effective = adapter.effective_timeout(Duration::from_secs(900));
        assert_eq!(effective, Duration::from_secs(120));
    }

    #[test]
    fn test_max_timeout_passes_lower_config() {
        let adapter = SubprocessAdapter::new("test", "echo", vec![], vec![], vec!["pdf".to_string()])
            .with_max_timeout(Duration::from_secs(120));
        // Config timeout (60s) is already lower than max (120s), keep config
        let effective = adapter.effective_timeout(Duration::from_secs(60));
        assert_eq!(effective, Duration::from_secs(60));
    }

    #[test]
    fn test_max_timeout_none_uses_config() {
        let adapter = SubprocessAdapter::new("test", "echo", vec![], vec![], vec!["pdf".to_string()]);
        // No max_timeout → config timeout passes through unchanged
        let effective = adapter.effective_timeout(Duration::from_secs(900));
        assert_eq!(effective, Duration::from_secs(900));
    }

    #[test]
    fn test_with_max_timeout_builder() {
        let adapter = SubprocessAdapter::new("test", "echo", vec![], vec![], vec!["pdf".to_string()])
            .with_max_timeout(Duration::from_secs(300));
        assert_eq!(adapter.max_timeout, Some(Duration::from_secs(300)));
    }

    #[test]
    fn test_parse_output_empty_string_content() {
        // {"content": "", "_extraction_time_ms": 5} → EmptyContent
        let adapter = SubprocessAdapter::new("test", "echo", vec![], vec![], vec!["pdf".to_string()]);
        let output = r#"{"content": "", "_extraction_time_ms": 5.0}"#;
        let result = adapter.parse_output(output);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, Error::EmptyContent(_)),
            "Expected EmptyContent, got: {:?}",
            err
        );
        assert!(err.to_string().contains("empty content"));
    }

    #[test]
    fn test_parse_output_whitespace_only_content() {
        // {"content": "  \n  "} → EmptyContent
        let adapter = SubprocessAdapter::new("test", "echo", vec![], vec![], vec!["pdf".to_string()]);
        let output = "{\"content\": \"  \\n  \", \"_extraction_time_ms\": 10.0}";
        let result = adapter.parse_output(output);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, Error::EmptyContent(_)),
            "Expected EmptyContent, got: {:?}",
            err
        );
    }

    #[test]
    fn test_parse_output_python_side_timeout() {
        // Python-side timeout via multiprocessing fork reports "timed out" → Timeout error
        let adapter = SubprocessAdapter::new("test", "echo", vec![], vec![], vec!["pdf".to_string()]);
        let output = r#"{"error": "extraction timed out after 150s", "_extraction_time_ms": 150000.0}"#;
        let result = adapter.parse_output(output);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::Timeout(_)), "Expected Timeout, got: {:?}", err);
        assert!(err.to_string().contains("timed out"));
    }

    #[test]
    fn test_error_to_error_kind_mapping() {
        assert_eq!(error_to_error_kind(&Error::Timeout("test".into())), ErrorKind::Timeout);
        assert_eq!(
            error_to_error_kind(&Error::FrameworkError("test".into())),
            ErrorKind::FrameworkError
        );
        assert_eq!(
            error_to_error_kind(&Error::EmptyContent("test".into())),
            ErrorKind::EmptyContent
        );
        assert_eq!(
            error_to_error_kind(&Error::Benchmark("test".into())),
            ErrorKind::HarnessError
        );
    }

    #[test]
    fn test_format_aware_builder() {
        let adapter =
            SubprocessAdapter::new("test", "echo", vec![], vec![], vec!["pdf".to_string()]).with_format_aware(true);
        assert!(adapter.format_aware);
        assert!(!adapter.use_native_batch);
    }

    #[test]
    fn test_native_batch_builder() {
        let adapter = SubprocessAdapter::with_batch_support("test", "echo", vec![], vec![], vec!["pdf".to_string()])
            .with_format_aware(true)
            .with_native_batch(true);
        assert!(adapter.supports_batch);
        assert!(adapter.format_aware);
        assert!(adapter.use_native_batch);
    }
}

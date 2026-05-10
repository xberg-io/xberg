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
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Extract JSON content from raw stdout, stripping non-JSON prefix lines.
///
/// Some runtimes (notably Elixir's BEAM VM) emit log messages to stdout
/// during module initialization before the script can redirect them. This
/// function finds the first `[` or `{` character and returns everything
/// from that point, ignoring any preceding log lines.
fn extract_json_from_stdout(raw: &str) -> &str {
    if let Some(pos) = raw.find('[').or_else(|| raw.find('{')) {
        &raw[pos..]
    } else {
        raw
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
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::process::Command;

/// Minimum duration in seconds for a valid throughput calculation.
/// Durations below this threshold produce unreliable throughput values
/// and will result in throughput being set to 0.0 (filtered in aggregation).
const MIN_VALID_DURATION_SECS: f64 = 0.000_001; // 1 microsecond

/// Check if verbose benchmark debugging is enabled via BENCHMARK_DEBUG env var.
fn is_debug_enabled() -> bool {
    std::env::var("BENCHMARK_DEBUG").is_ok()
}

/// State for a persistent subprocess that stays alive across multiple extractions
struct PersistentProcess {
    stdin: BufWriter<tokio::process::ChildStdin>,
    stdout: BufReader<tokio::process::ChildStdout>,
    child: tokio::process::Child,
    /// PID of the child process, captured at spawn time for memory monitoring
    child_pid: u32,
}

/// Base adapter for subprocess-based extraction
///
/// This adapter spawns a subprocess to perform extraction and monitors
/// its resource usage. Subclasses implement the specific command construction
/// for each language binding.
///
/// When `persistent` is enabled, the subprocess is spawned once in `setup()`
/// and reused for all extractions via stdin/stdout communication, eliminating
/// per-file process startup overhead (e.g., JVM startup for Tika).
pub struct SubprocessAdapter {
    name: String,
    command: PathBuf,
    args: Vec<String>,
    env: Vec<(String, String)>,
    supports_batch: bool,
    working_dir: Option<PathBuf>,
    supported_formats: Vec<String>,
    persistent: bool,
    max_timeout: Option<Duration>,
    skip_files: Vec<String>,
    process: Arc<tokio::sync::Mutex<Option<PersistentProcess>>>,
}

impl SubprocessAdapter {
    /// Determine if a framework supports OCR based on its name
    ///
    /// Known frameworks with OCR support:
    /// - kreuzberg-* (all Kreuzberg bindings support OCR)
    /// - pymupdf (supports OCR via tesseract)
    ///
    /// Frameworks without OCR support:
    /// - pdfplumber
    /// - pypdf
    /// - Other basic PDF parsers
    fn framework_supports_ocr(framework_name: &str) -> bool {
        let name_lower = framework_name.to_lowercase();

        // Kreuzberg bindings all support OCR
        if name_lower.starts_with("kreuzberg-") || name_lower == "kreuzberg" {
            return true;
        }

        // PyMuPDF supports OCR via tesseract
        if name_lower.contains("pymupdf") {
            return true;
        }

        // Docling supports OCR via EasyOCR/Tesseract
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
    /// * `name` - Framework name (e.g., "kreuzberg-python")
    /// * `command` - Path to executable (e.g., "python3", "node")
    /// * `args` - Base arguments (e.g., ["-m", "kreuzberg"])
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
            persistent: false,
            max_timeout: None,
            skip_files: vec![],
            process: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }

    /// Create a new subprocess adapter with batch support
    ///
    /// This adapter will call `extract_batch()` with all files at once,
    /// allowing the subprocess to use its native batch API for parallel processing.
    ///
    /// # Arguments
    /// * `name` - Framework name (e.g., "kreuzberg-python-batch")
    /// * `command` - Path to executable (e.g., "python3", "node")
    /// * `args` - Base arguments (e.g., ["-m", "kreuzberg"])
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
            persistent: false,
            max_timeout: None,
            skip_files: vec![],
            process: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }

    /// Create a new subprocess adapter with persistent mode
    ///
    /// In persistent mode, the subprocess is spawned once in `setup()` and kept
    /// alive across all extractions. File paths are sent via stdin and JSON results
    /// are read from stdout, eliminating per-file process startup overhead.
    ///
    /// # Arguments
    /// * `name` - Framework name (e.g., "tika")
    /// * `command` - Path to executable (e.g., "java")
    /// * `args` - Base arguments (e.g., ["-cp", "tika.jar", "TikaExtract.java", "--ocr", "server"])
    /// * `env` - Environment variables
    /// * `supported_formats` - List of file extensions this framework can process
    pub fn with_persistent_mode(
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
            persistent: true,
            max_timeout: None,
            skip_files: vec![],
            process: Arc::new(tokio::sync::Mutex::new(None)),
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

    /// Spawn a persistent subprocess and return its handles.
    ///
    /// Used by both `setup()` and the timeout-restart path in `execute_persistent()`.
    async fn spawn_persistent(&self) -> Result<PersistentProcess> {
        let mut cmd = Command::new(&self.command);
        if let Some(dir) = &self.working_dir {
            cmd.current_dir(dir);
        }
        cmd.args(&self.args);
        for (key, value) in &self.env {
            cmd.env(key, value);
        }
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::inherit());

        let mut child = cmd
            .spawn()
            .map_err(|e| Error::Benchmark(format!("Failed to spawn persistent process: {}", e)))?;

        let child_pid = child.id().unwrap_or(0);
        let stdin = BufWriter::new(child.stdin.take().unwrap());
        let stdout = BufReader::new(child.stdout.take().unwrap());

        Ok(PersistentProcess {
            stdin,
            stdout,
            child,
            child_pid,
        })
    }

    /// Execute the extraction subprocess
    async fn execute_subprocess(&self, file_path: &Path, timeout: Duration) -> Result<(String, String, Duration)> {
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
    ) -> Result<(String, String, Duration)> {
        let start = Instant::now();

        let mut cmd = Command::new(&self.command);
        if let Some(dir) = &self.working_dir {
            cmd.current_dir(dir);
        }
        cmd.args(&self.args);

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

    /// Execute extraction via persistent subprocess (stdin/stdout protocol)
    async fn execute_persistent(
        &self,
        file_path: &Path,
        timeout: Duration,
        force_ocr: bool,
    ) -> Result<(String, Duration)> {
        let absolute_path = if file_path.is_absolute() {
            file_path.to_path_buf()
        } else {
            std::env::current_dir().map_err(Error::Io)?.join(file_path)
        };

        let mut guard = self.process.lock().await;
        let proc = guard
            .as_mut()
            .ok_or_else(|| Error::Benchmark("Persistent process not started".into()))?;

        let start = Instant::now();

        // Send JSON request with path and force_ocr flag
        let request = serde_json::json!({
            "path": absolute_path.to_string_lossy(),
            "force_ocr": force_ocr,
        });
        proc.stdin
            .write_all(request.to_string().as_bytes())
            .await
            .map_err(|e| Error::Benchmark(format!("Failed to write to persistent process: {}", e)))?;
        proc.stdin
            .write_all(b"\n")
            .await
            .map_err(|e| Error::Benchmark(format!("Failed to write newline: {}", e)))?;
        proc.stdin
            .flush()
            .await
            .map_err(|e| Error::Benchmark(format!("Failed to flush stdin: {}", e)))?;

        let write_elapsed = start.elapsed();

        // Read lines until we get a JSON response (starts with '{').
        // Non-JSON lines (C library warnings, Python info messages) are skipped.
        let mut line = String::new();
        let read_result = tokio::time::timeout(timeout, async {
            loop {
                line.clear();
                let n = proc
                    .stdout
                    .read_line(&mut line)
                    .await
                    .map_err(|e| Error::Benchmark(format!("Failed to read from persistent process: {}", e)))?;
                if n == 0 {
                    return Err(Error::Benchmark(
                        "Persistent process returned EOF (process may have crashed)".to_string(),
                    ));
                }
                let trimmed = line.trim();
                if trimmed.starts_with('{') {
                    return Ok(n);
                }
                // Skip non-JSON noise (C library warnings, info messages)
                if is_debug_enabled() {
                    eprintln!("[persistent:{}] skipping non-JSON line: {}", self.name, trimmed);
                }
            }
        })
        .await;

        let bytes_read = match read_result {
            Ok(Ok(n)) => n,
            Ok(Err(e)) => {
                // Inner error (EOF / crash) — restart the process for the next call
                eprintln!(
                    "[persistent:{}] process error — killing and restarting: {}",
                    self.name, e
                );
                if let Some(mut old_proc) = guard.take() {
                    let _ = old_proc.child.kill().await;
                    let _ = old_proc.child.wait().await;
                }
                match self.spawn_persistent().await {
                    Ok(new_proc) => *guard = Some(new_proc),
                    Err(re) => eprintln!("[persistent:{}] failed to restart after error: {}", self.name, re),
                }
                return Err(e);
            }
            Err(_elapsed) => {
                // Timeout fired — kill the stuck process and restart it to prevent
                // protocol desync (the old process may still emit a response later
                // that would be mis-attributed to the next file).
                eprintln!(
                    "[persistent:{}] timeout after {:?} — killing and restarting process",
                    self.name, timeout
                );
                if let Some(mut old_proc) = guard.take() {
                    let _ = old_proc.child.kill().await;
                    let _ = old_proc.child.wait().await;
                }
                // Restart so the next call finds a fresh process
                match self.spawn_persistent().await {
                    Ok(new_proc) => *guard = Some(new_proc),
                    Err(e) => eprintln!("[persistent:{}] failed to restart after timeout: {}", self.name, e),
                }
                return Err(Error::Timeout(format!(
                    "Persistent process response exceeded {:?}",
                    timeout
                )));
            }
        };

        let duration = start.elapsed();

        if is_debug_enabled() {
            eprintln!(
                "[persistent:{}] write={:.2}ms read={:.2}ms total={:.2}ms bytes={} path={}",
                self.name,
                write_elapsed.as_secs_f64() * 1000.0,
                (duration - write_elapsed).as_secs_f64() * 1000.0,
                duration.as_secs_f64() * 1000.0,
                bytes_read,
                absolute_path.display()
            );
        }

        Ok((line, duration))
    }

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

        let parsed: serde_json::Value = serde_json::from_str(stdout)
            .map_err(|e| Error::Benchmark(format!("Failed to parse subprocess output as JSON: {}", e)))?;

        // Validate that output is a JSON object
        if !parsed.is_object() {
            return Err(Error::Benchmark(
                "Subprocess output must be a JSON object with 'content' field".to_string(),
            ));
        }

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
        force_ocr: bool,
        output_format: OutputFormat,
    ) -> Result<BenchmarkResult> {
        let timeout = self.effective_timeout(timeout);
        let file_size = std::fs::metadata(file_path).map_err(Error::Io)?.len();

        let start_time = std::time::Instant::now();
        // For persistent mode, monitor the child process tree (extraction server) instead
        // of the harness process. This captures actual extraction memory, not the lightweight
        // harness overhead.
        let monitor = if self.persistent {
            let guard = self.process.lock().await;
            let child_pid = guard.as_ref().map(|p| p.child_pid).unwrap_or(0);
            drop(guard);
            if child_pid > 0 {
                ResourceMonitor::new_for_pid(child_pid)
            } else {
                ResourceMonitor::new()
            }
        } else {
            ResourceMonitor::new()
        };
        let sampling_ms = crate::monitoring::adaptive_sampling_interval_ms(file_size);
        monitor.start(Duration::from_millis(sampling_ms)).await;

        let (stdout, _stderr, duration) = if self.persistent {
            match self.execute_persistent(file_path, timeout, force_ocr).await {
                Ok((stdout, dur)) => (stdout, String::new(), dur),
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
            }
        } else {
            match self.execute_subprocess(file_path, timeout).await {
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

        let (stdout, _stderr, duration) = match self.execute_subprocess_batch(file_paths, timeout).await {
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

        if !self.persistent {
            return Ok(());
        }

        let mut proc = self.spawn_persistent().await?;

        // Wait for the process to signal readiness.
        // Scripts should print "READY" on stdout after initialization (runtime startup,
        // FFI library loading, model loading, etc.) is complete.
        // This ensures cold_start measures only framework extraction time, not
        // JVM startup, `dotnet run` compilation, `go run` compilation, etc.
        let ready_timeout_secs: u64 = std::env::var("READY_TIMEOUT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(300);
        let ready_timeout = std::time::Duration::from_secs(ready_timeout_secs);
        let ready_result = tokio::time::timeout(ready_timeout, async {
            let mut line = String::new();
            loop {
                line.clear();
                let n = proc
                    .stdout
                    .read_line(&mut line)
                    .await
                    .map_err(|e| Error::Benchmark(format!("Failed to read READY from {}: {}", self.name, e)))?;
                if n == 0 {
                    return Err(Error::Benchmark(format!(
                        "{}: process exited before sending READY signal",
                        self.name
                    )));
                }
                let trimmed = line.trim();
                if trimmed == "READY" {
                    return Ok(());
                }
                // Skip non-READY lines (runtime warnings, debug output)
                if is_debug_enabled() {
                    eprintln!("[setup:{}] pre-ready line: {}", self.name, trimmed);
                }
            }
        })
        .await;

        match ready_result {
            Ok(Ok(())) => {
                eprintln!("[setup:{}] process ready", self.name);
            }
            Ok(Err(e)) => {
                eprintln!("[setup:{}] process failed during startup: {}", self.name, e);
                return Err(e);
            }
            Err(_) => {
                eprintln!(
                    "[setup:{}] warning: no READY signal after {}s — proceeding anyway",
                    self.name,
                    ready_timeout.as_secs()
                );
            }
        }

        *self.process.lock().await = Some(proc);
        Ok(())
    }

    async fn teardown(&self) -> Result<()> {
        if let Some(mut proc) = self.process.lock().await.take() {
            drop(proc.stdin); // Close stdin -> EOF -> process exits
            let _ = proc.child.wait().await;
        }
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
    fn test_persistent_adapter_creation() {
        let adapter = SubprocessAdapter::with_persistent_mode(
            "test-persistent",
            "echo",
            vec!["server".to_string()],
            vec![],
            vec!["pdf".to_string()],
        );
        assert_eq!(adapter.name(), "test-persistent");
        assert!(adapter.persistent);
        assert!(!adapter.supports_batch);
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

    #[tokio::test]
    async fn test_persistent_echo_server() {
        // Create inline echo server script in temp dir
        let tmp_dir = tempfile::TempDir::new().unwrap();
        let script_path = tmp_dir.path().join("echo_server.py");
        std::fs::write(
            &script_path,
            r#"
import json, sys, time
print("READY", flush=True)
for line in sys.stdin:
    fp = line.strip()
    if not fp:
        continue
    start = time.perf_counter()
    try:
        with open(fp, 'r', errors='replace') as f:
            content = f.read()
    except Exception as e:
        content = f"error: {e}"
    ms = (time.perf_counter() - start) * 1000.0
    print(json.dumps({"content": content[:1000], "_extraction_time_ms": ms}), flush=True)
"#,
        )
        .unwrap();

        let small_file = tmp_dir.path().join("small.txt");
        std::fs::write(&small_file, "Hello, small file!").unwrap();

        let medium_file = tmp_dir.path().join("medium.txt");
        std::fs::write(&medium_file, "x".repeat(100_000)).unwrap(); // 100KB

        let large_file = tmp_dir.path().join("large.txt");
        std::fs::write(&large_file, "y".repeat(1_000_000)).unwrap(); // 1MB

        // Create persistent adapter pointing to echo server
        let adapter = SubprocessAdapter::with_persistent_mode(
            "test-echo",
            "python3",
            vec![script_path.to_string_lossy().to_string()],
            vec![],
            vec!["txt".to_string()],
        );

        // Setup (spawns the persistent process)
        adapter.setup().await.expect("setup should succeed");

        // Warmup extraction (like CI does)
        let warmup_result = adapter
            .extract(&small_file, Duration::from_secs(10), false, OutputFormat::Markdown)
            .await
            .expect("warmup should succeed");
        eprintln!(
            "Warmup: success={}, duration={:?}, extraction_duration={:?}",
            warmup_result.success, warmup_result.duration, warmup_result.extraction_duration
        );

        // Run 3 benchmark iterations like CI (different files to check for desync)
        let files = [&small_file, &medium_file, &large_file];
        for (i, file) in files.iter().enumerate() {
            let result = adapter
                .extract(file, Duration::from_secs(30), false, OutputFormat::Markdown)
                .await
                .expect("extract should succeed");

            eprintln!(
                "Iter {}: file={:?} size={} duration={:?} extraction_duration={:?} has_text={}",
                i + 1,
                file.file_name().unwrap(),
                result.file_size,
                result.duration,
                result.extraction_duration,
                result.extracted_text.is_some()
            );

            assert!(result.success, "Extraction {} should succeed", i + 1);
            assert!(
                result.extraction_duration.is_some(),
                "Iteration {}: extraction_duration should NOT be null",
                i + 1
            );
            assert!(
                result.extracted_text.is_some(),
                "Iteration {}: extracted_text should be present",
                i + 1
            );

            // Duration should be reasonable
            assert!(
                result.duration.as_micros() > 10,
                "Iteration {}: Duration too short: {:?}",
                i + 1,
                result.duration
            );
        }

        // Verify durations scale with file size
        let r_small = adapter
            .extract(&small_file, Duration::from_secs(10), false, OutputFormat::Markdown)
            .await
            .unwrap();
        let r_large = adapter
            .extract(&large_file, Duration::from_secs(30), false, OutputFormat::Markdown)
            .await
            .unwrap();
        eprintln!(
            "Small duration: {:?}, Large duration: {:?}",
            r_small.duration, r_large.duration
        );

        // Teardown
        adapter.teardown().await.expect("teardown should succeed");
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
    fn test_with_max_timeout_builder_persistent() {
        let adapter = SubprocessAdapter::with_persistent_mode("test", "echo", vec![], vec![], vec!["pdf".to_string()])
            .with_max_timeout(Duration::from_secs(180));
        assert_eq!(adapter.max_timeout, Some(Duration::from_secs(180)));
        assert!(adapter.persistent);
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

    #[tokio::test]
    async fn test_persistent_kreuzberg_python() {
        // Test with actual kreuzberg Python script if available
        let script_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("scripts")
            .join("kreuzberg_extract.py");
        if !script_path.exists() {
            eprintln!("Skipping test: kreuzberg script not found");
            return;
        }

        // Check if python3 has kreuzberg installed (skip if not)
        let check = std::process::Command::new("python3")
            .arg("-c")
            .arg("import kreuzberg")
            .output();
        if check.is_err() || !check.unwrap().status.success() {
            eprintln!("Skipping test: kreuzberg not installed in python3");
            return;
        }

        let tmp_dir = tempfile::TempDir::new().unwrap();
        let test_file = tmp_dir.path().join("test.txt");
        std::fs::write(&test_file, "Hello from kreuzberg benchmark test!").unwrap();

        let adapter = SubprocessAdapter::with_persistent_mode(
            "kreuzberg-python-test",
            "python3",
            vec![
                script_path.to_string_lossy().to_string(),
                "--no-ocr".to_string(),
                "server".to_string(),
            ],
            vec![],
            vec!["txt".to_string()],
        );

        adapter.setup().await.expect("setup should succeed");

        // Run warmup + 3 iterations (like CI)
        let warmup = adapter
            .extract(&test_file, Duration::from_secs(30), false, OutputFormat::Markdown)
            .await;
        eprintln!(
            "Kreuzberg warmup: {:?}",
            warmup.as_ref().map(|r| (r.success, r.duration, r.extraction_duration))
        );

        for i in 0..3 {
            let result = adapter
                .extract(&test_file, Duration::from_secs(30), false, OutputFormat::Markdown)
                .await;
            match &result {
                Ok(r) => {
                    eprintln!(
                        "Kreuzberg iter {}: success={} duration={:?} extraction_duration={:?}",
                        i + 1,
                        r.success,
                        r.duration,
                        r.extraction_duration
                    );
                    assert!(r.success, "Kreuzberg iter {} should succeed", i + 1);
                    assert!(
                        r.extraction_duration.is_some(),
                        "Kreuzberg iter {}: extraction_duration must not be null!",
                        i + 1
                    );
                }
                Err(e) => {
                    eprintln!("Kreuzberg iter {} failed: {}", i + 1, e);
                }
            }
        }

        adapter.teardown().await.expect("teardown should succeed");
    }

    #[tokio::test]
    async fn test_persistent_timeout_kills_and_restarts() {
        // Create a "slow server" that sleeps 5s on a magic filename, responds instantly otherwise
        let tmp_dir = tempfile::TempDir::new().unwrap();
        let script_path = tmp_dir.path().join("slow_server.py");
        std::fs::write(
            &script_path,
            r#"
import json, sys, time
print("READY", flush=True)
for line in sys.stdin:
    fp = line.strip()
    if not fp:
        continue
    if "SLOW" in fp:
        time.sleep(5)
    content = f"processed: {fp}"
    print(json.dumps({"content": content, "_extraction_time_ms": 1.0}), flush=True)
"#,
        )
        .unwrap();

        let fast_file = tmp_dir.path().join("fast.txt");
        std::fs::write(&fast_file, "hello").unwrap();

        let slow_file = tmp_dir.path().join("SLOW.txt");
        std::fs::write(&slow_file, "slow").unwrap();

        let adapter = SubprocessAdapter::with_persistent_mode(
            "test-timeout",
            "python3",
            vec![script_path.to_string_lossy().to_string()],
            vec![],
            vec!["txt".to_string()],
        )
        .with_max_timeout(Duration::from_secs(2)); // 2s timeout

        adapter.setup().await.expect("setup should succeed");

        // 1. Fast file should work
        let r1 = adapter
            .extract(&fast_file, Duration::from_secs(10), false, OutputFormat::Markdown)
            .await
            .unwrap();
        assert!(r1.success, "fast file should succeed");
        eprintln!("fast file OK: {:?}", r1.duration);

        // 2. Slow file should timeout (5s sleep > 2s timeout)
        let r2 = adapter
            .extract(&slow_file, Duration::from_secs(10), false, OutputFormat::Markdown)
            .await
            .unwrap();
        assert!(!r2.success, "slow file should fail with timeout");
        assert_eq!(r2.error_kind, ErrorKind::Timeout);
        eprintln!("slow file timed out as expected: {:?}", r2.error_message);

        // 3. KEY TEST: fast file should STILL work after the timeout
        //    (proves the process was killed and restarted, not left in a desync state)
        let r3 = adapter
            .extract(&fast_file, Duration::from_secs(10), false, OutputFormat::Markdown)
            .await
            .unwrap();
        assert!(
            r3.success,
            "fast file after timeout should succeed (process was restarted)"
        );
        eprintln!("fast file after restart OK: {:?}", r3.duration);

        adapter.teardown().await.expect("teardown should succeed");
    }

    #[tokio::test]
    async fn test_python_side_fork_timeout() {
        // Test the fork-based timeout mechanism: the Python script handles
        // timeouts internally by killing only the forked child process,
        // keeping the parent alive (no Rust-side kill+restart needed).
        //
        // Skip on macOS where multiprocessing.fork is unreliable with Python 3.13+.
        if cfg!(target_os = "macos") {
            let output = std::process::Command::new("python3")
                .args(["-c", "import sys; print(sys.version_info[:2])"])
                .output();
            if let Ok(out) = output {
                let ver = String::from_utf8_lossy(&out.stdout);
                if ver.contains("(3, 13)") || ver.contains("(3, 14)") || ver.contains("(3, 15)") {
                    eprintln!(
                        "Skipping test: multiprocessing.fork unreliable on macOS with Python {}",
                        ver.trim()
                    );
                    return;
                }
            }
        }

        let script_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("scripts")
            .join("test_fork_timeout.py");
        if !script_path.exists() {
            eprintln!("Skipping test: test_fork_timeout.py not found");
            return;
        }

        let tmp_dir = tempfile::TempDir::new().unwrap();
        let fast_file = tmp_dir.path().join("fast.txt");
        std::fs::write(&fast_file, "hello fast").unwrap();

        let slow_file = tmp_dir.path().join("SLOW.txt");
        std::fs::write(&slow_file, "hello slow").unwrap();

        // Python-side timeout = 2s, Rust-side safety net = 10s.
        // The Python side should fire first.
        let adapter = SubprocessAdapter::with_persistent_mode(
            "test-fork-timeout",
            "python3",
            vec![
                script_path.to_string_lossy().to_string(),
                "--timeout=2".to_string(),
                "server".to_string(),
            ],
            vec![],
            vec!["txt".to_string()],
        )
        .with_max_timeout(Duration::from_secs(10));

        adapter.setup().await.expect("setup should succeed");

        // 1. Fast file through forked child — should succeed
        let r1 = adapter
            .extract(&fast_file, Duration::from_secs(30), false, OutputFormat::Markdown)
            .await
            .unwrap();
        assert!(r1.success, "fast file should succeed through fork");
        assert!(
            r1.extracted_text.as_deref().unwrap().contains("hello fast"),
            "content should contain file text"
        );
        eprintln!(
            "1. fast file OK: duration={:?}, extraction_duration={:?}",
            r1.duration, r1.extraction_duration
        );

        // 2. Slow file should be timed out by the Python side (2s < 10s sleep)
        let start = std::time::Instant::now();
        let r2 = adapter
            .extract(&slow_file, Duration::from_secs(30), false, OutputFormat::Markdown)
            .await
            .unwrap();
        let elapsed = start.elapsed();

        assert!(!r2.success, "slow file should fail");
        assert_eq!(
            r2.error_kind,
            ErrorKind::Timeout,
            "should be classified as Timeout, got {:?}: {:?}",
            r2.error_kind,
            r2.error_message
        );
        assert!(
            r2.error_message.as_deref().unwrap_or("").contains("timed out"),
            "error should mention 'timed out': {:?}",
            r2.error_message
        );
        // Should have timed out around 2s (Python side), NOT 10s (Rust side)
        assert!(
            elapsed < Duration::from_secs(5),
            "timeout should fire at ~2s (Python side), not 10s — actual: {:?}",
            elapsed
        );
        eprintln!(
            "2. slow file timed out (Python-side) in {:?}: {:?}",
            elapsed, r2.error_message
        );

        // 3. KEY TEST: fast file should STILL work immediately after timeout.
        //    This proves the parent Python process stayed alive — no kill+restart.
        let start = std::time::Instant::now();
        let r3 = adapter
            .extract(&fast_file, Duration::from_secs(30), false, OutputFormat::Markdown)
            .await
            .unwrap();
        let elapsed = start.elapsed();

        assert!(
            r3.success,
            "fast file after timeout should succeed (parent stayed alive)"
        );
        // Should be fast — no process restart overhead
        assert!(
            elapsed < Duration::from_secs(2),
            "post-timeout extraction should be fast (no restart), actual: {:?}",
            elapsed
        );
        eprintln!("3. fast file after timeout OK in {:?}", elapsed);

        // 4. Another slow file to test repeated timeouts don't break anything
        let r4 = adapter
            .extract(&slow_file, Duration::from_secs(30), false, OutputFormat::Markdown)
            .await
            .unwrap();
        assert!(!r4.success, "second slow file should also timeout");
        assert_eq!(r4.error_kind, ErrorKind::Timeout);
        eprintln!("4. second timeout OK: {:?}", r4.error_message);

        // 5. Final fast file — parent still alive after two timeouts
        let r5 = adapter
            .extract(&fast_file, Duration::from_secs(30), false, OutputFormat::Markdown)
            .await
            .unwrap();
        assert!(r5.success, "fast file after two timeouts should still succeed");
        eprintln!("5. fast file after two timeouts OK: {:?}", r5.duration);

        adapter.teardown().await.expect("teardown should succeed");
    }
}

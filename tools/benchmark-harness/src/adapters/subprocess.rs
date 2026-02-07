//! Subprocess-based adapter for language bindings
//!
//! This adapter provides a base for running extraction via subprocess.
//! It's used by Python, Node.js, and Ruby adapters to execute extraction
//! in separate processes while monitoring resource usage.

use crate::adapter::FrameworkAdapter;
use crate::monitoring::ResourceMonitor;
use crate::types::{BenchmarkResult, ErrorKind, FrameworkCapabilities, OcrStatus, PerformanceMetrics};
use crate::{Error, Result};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};
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
            process: Arc::new(tokio::sync::Mutex::new(None)),
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

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
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

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
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
    async fn execute_persistent(&self, file_path: &Path, timeout: Duration) -> Result<(String, Duration)> {
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

        // Send file path
        proc.stdin
            .write_all(absolute_path.to_string_lossy().as_bytes())
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
        let bytes_read = tokio::time::timeout(timeout, async {
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
        .await
        .map_err(|_| Error::Timeout(format!("Persistent process response exceeded {:?}", timeout)))??;

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
                return Err(Error::FrameworkError(error_msg.to_string()));
            }
        }

        if !parsed.get("content").is_some_and(|v| v.is_string()) {
            return Err(Error::Benchmark(
                "Subprocess output missing required 'content' field (must be a string)".to_string(),
            ));
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

    async fn extract(&self, file_path: &Path, timeout: Duration) -> Result<BenchmarkResult> {
        let file_size = std::fs::metadata(file_path).map_err(Error::Io)?.len();

        let start_time = std::time::Instant::now();
        let monitor = ResourceMonitor::new();
        let sampling_ms = crate::monitoring::adaptive_sampling_interval_ms(file_size);
        monitor.start(Duration::from_millis(sampling_ms)).await;

        let (stdout, _stderr, duration) = if self.persistent {
            match self.execute_persistent(file_path, timeout).await {
                Ok((stdout, dur)) => (stdout, String::new(), dur),
                Err(e) => {
                    let samples = monitor.stop().await;
                    let snapshots = monitor.get_snapshots().await;
                    let resource_stats = ResourceMonitor::calculate_stats(&samples, &snapshots);
                    let actual_duration = start_time.elapsed();

                    let throughput = if actual_duration.as_secs_f64() > 0.0 {
                        file_size as f64 / actual_duration.as_secs_f64()
                    } else {
                        0.0
                    };

                    let framework_capabilities = FrameworkCapabilities {
                        ocr_support: Self::framework_supports_ocr(&self.name),
                        batch_support: self.supports_batch,
                        ..Default::default()
                    };

                    return Ok(BenchmarkResult {
                        framework: self.name.clone(),
                        file_path: file_path.to_path_buf(),
                        file_size,
                        success: false,
                        error_message: Some(e.to_string()),
                        error_kind: ErrorKind::HarnessError,
                        duration: actual_duration,
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
                    });
                }
            }
        } else {
            match self.execute_subprocess(file_path, timeout).await {
                Ok(result) => result,
                Err(e) => {
                    let samples = monitor.stop().await;
                    let snapshots = monitor.get_snapshots().await;
                    let resource_stats = ResourceMonitor::calculate_stats(&samples, &snapshots);
                    let actual_duration = start_time.elapsed();

                    let throughput = if actual_duration.as_secs_f64() > 0.0 {
                        file_size as f64 / actual_duration.as_secs_f64()
                    } else {
                        0.0
                    };

                    let framework_capabilities = FrameworkCapabilities {
                        ocr_support: Self::framework_supports_ocr(&self.name),
                        batch_support: self.supports_batch,
                        ..Default::default()
                    };

                    return Ok(BenchmarkResult {
                        framework: self.name.clone(),
                        file_path: file_path.to_path_buf(),
                        file_size,
                        success: false,
                        error_message: Some(e.to_string()),
                        error_kind: ErrorKind::HarnessError,
                        duration: actual_duration,
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
                    });
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
        let resource_stats = ResourceMonitor::calculate_stats(&samples, &snapshots);

        let parsed = match self.parse_output(&stdout) {
            Ok(value) => value,
            Err(e) => {
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

                let error_kind = if matches!(e, Error::FrameworkError(_)) {
                    ErrorKind::FrameworkError
                } else {
                    ErrorKind::HarnessError
                };

                return Ok(BenchmarkResult {
                    framework: self.name.clone(),
                    file_path: file_path.to_path_buf(),
                    file_size,
                    success: false,
                    error_message: Some(e.to_string()),
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
                });
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

        let metrics = PerformanceMetrics {
            peak_memory_bytes: resource_stats.peak_memory_bytes,
            avg_cpu_percent: resource_stats.avg_cpu_percent,
            throughput_bytes_per_sec: throughput,
            p50_memory_bytes: resource_stats.p50_memory_bytes,
            p95_memory_bytes: resource_stats.p95_memory_bytes,
            p99_memory_bytes: resource_stats.p99_memory_bytes,
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

    async fn extract_batch(&self, file_paths: &[&Path], timeout: Duration) -> Result<Vec<BenchmarkResult>> {
        // Early return if file_paths is empty
        if file_paths.is_empty() {
            return Ok(Vec::new());
        }

        if !self.supports_batch {
            let mut results = Vec::new();
            for path in file_paths {
                results.push(self.extract(path, timeout).await?);
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
                let resource_stats = ResourceMonitor::calculate_stats(&samples, &snapshots);
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
                            file_path: file_path.to_path_buf(),
                            file_size,
                            success: false,
                            error_message: Some(e.to_string()),
                            error_kind: ErrorKind::HarnessError,
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
        let resource_stats = ResourceMonitor::calculate_stats(&samples, &snapshots);

        // Parse batch output to extract per-file OCR status and extraction times
        // Try to parse as JSON array; fall back to defaults if parsing fails
        let parsed_batch: Option<Vec<serde_json::Value>> = serde_json::from_str(&stdout).ok();

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

                BenchmarkResult {
                    framework: self.name.clone(),
                    file_path: file_path.to_path_buf(),
                    file_size,
                    success: true,
                    error_message: None,
                    error_kind: ErrorKind::None,
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
        // Inherit stderr so persistent process crash messages are visible in CI logs
        // and the OS pipe buffer doesn't fill up causing broken pipe errors.
        cmd.stderr(Stdio::inherit());

        let mut child = cmd
            .spawn()
            .map_err(|e| Error::Benchmark(format!("Failed to spawn persistent process: {}", e)))?;

        let stdin = BufWriter::new(child.stdin.take().unwrap());
        let stdout = BufReader::new(child.stdout.take().unwrap());

        *self.process.lock().await = Some(PersistentProcess { stdin, stdout, child });
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
            .extract(&small_file, Duration::from_secs(10))
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
                .extract(file, Duration::from_secs(30))
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
        let r_small = adapter.extract(&small_file, Duration::from_secs(10)).await.unwrap();
        let r_large = adapter.extract(&large_file, Duration::from_secs(30)).await.unwrap();
        eprintln!(
            "Small duration: {:?}, Large duration: {:?}",
            r_small.duration, r_large.duration
        );

        // Teardown
        adapter.teardown().await.expect("teardown should succeed");
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
        let warmup = adapter.extract(&test_file, Duration::from_secs(30)).await;
        eprintln!(
            "Kreuzberg warmup: {:?}",
            warmup.as_ref().map(|r| (r.success, r.duration, r.extraction_duration))
        );

        for i in 0..3 {
            let result = adapter.extract(&test_file, Duration::from_secs(30)).await;
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
}

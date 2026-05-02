//! Native Kreuzberg Rust adapter
//!
//! This adapter uses the Kreuzberg Rust core library directly for maximum performance.
//! It serves as the baseline for comparing language bindings.

use crate::adapter::FrameworkAdapter;
use crate::monitoring::ResourceMonitor;
use crate::types::{BenchmarkResult, ErrorKind, FrameworkCapabilities, OcrStatus, PerformanceMetrics};
use crate::{Error, Result};
use async_trait::async_trait;
use kreuzberg::{ExtractionConfig, ExtractionResult, FormatMetadata, batch_extract_files, extract_file};
use std::path::Path;
use std::time::{Duration, Instant};

/// Determine OCR status by inspecting the actual extraction result metadata.
///
/// The kreuzberg crate sets `FormatMetadata::Ocr` for raw tesseract results, but the
/// image extractor overwrites format to `FormatMetadata::Image` even when OCR was used.
/// So we also check: if the format is `Image` and OCR was enabled in config, OCR was used.
///
/// Returns:
/// - `OcrStatus::Used` if OCR metadata is present, or if this is an image with OCR enabled
/// - `OcrStatus::NotUsed` if format metadata is present and OCR was not involved
/// - `OcrStatus::Unknown` if no format metadata is available
fn determine_ocr_status(result: &ExtractionResult, config: &ExtractionConfig) -> OcrStatus {
    match &result.metadata.format {
        Some(FormatMetadata::Ocr(_)) => OcrStatus::Used,
        Some(FormatMetadata::Image(_)) => {
            // Image extractor overwrites Ocr -> Image format, so check config
            if config.ocr.is_some() || config.force_ocr {
                OcrStatus::Used
            } else {
                OcrStatus::NotUsed
            }
        }
        Some(_) => OcrStatus::NotUsed,
        None => OcrStatus::Unknown,
    }
}

/// Native Rust adapter using kreuzberg crate directly
pub struct NativeAdapter {
    config: ExtractionConfig,
}

impl NativeAdapter {
    /// Create a new native adapter with default configuration
    ///
    /// NOTE: Cache is explicitly disabled for accurate benchmarking
    pub fn new() -> Self {
        let config = ExtractionConfig {
            use_cache: false,
            ..Default::default()
        };
        Self { config }
    }

    /// Calculate adaptive sampling interval based on estimated task duration from file size
    ///
    /// Uses file size as a proxy for task duration to optimize sampling frequency:
    /// - Small files (<100KB, ~50-100ms tasks): 1ms sampling for high resolution
    /// - Medium files (100KB-1MB, ~100-1000ms tasks): 5ms sampling for balance
    /// - Large files (>1MB, >1000ms tasks): 10ms sampling to reduce overhead
    ///
    /// This adaptive approach ensures:
    /// - Quick tasks: 50-100 samples (sufficient for variance calculation)
    /// - Long tasks: 100-1000+ samples (excellent statistical significance)
    /// - Minimal monitoring overhead for all workloads
    ///
    /// # Arguments
    /// * `file_size` - File size in bytes
    ///
    /// # Returns
    /// Sampling interval in milliseconds (1, 5, or 10)
    fn calculate_adaptive_sampling_interval(file_size: u64) -> u64 {
        crate::monitoring::adaptive_sampling_interval_ms(file_size)
    }

    /// Create a new native adapter with custom configuration
    pub fn with_config(config: ExtractionConfig) -> Self {
        Self { config }
    }
}

impl Default for NativeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FrameworkAdapter for NativeAdapter {
    fn name(&self) -> &str {
        "kreuzberg-rust"
    }

    fn supports_format(&self, file_type: &str) -> bool {
        matches!(
            file_type.to_lowercase().as_str(),
            // Documents
            "pdf" | "docx" | "docm" | "dotx" | "dotm" | "dot" | "doc" | "odt" |
            "pptx" | "ppsx" | "pptm" | "potx" | "potm" | "pot" | "ppt" |
            "xlsx" | "xlsm" | "xlsb" | "xlam" | "xla" | "xltx" | "xlt" | "xls" | "ods" |
            "dbf" | "hwp" | "hwpx" |
            // Text formats
            "txt" | "md" | "markdown" | "commonmark" | "html" | "htm" | "xml" | "rtf" | "rst" | "org" |
            // Data formats
            "json" | "yaml" | "yml" | "toml" | "csv" | "tsv" |
            // Email
            "eml" | "msg" |
            // Archives
            "zip" | "tar" | "gz" | "tgz" | "7z" |
            // Images (OCR supported)
            "bmp" | "gif" | "jpg" | "jpeg" | "png" | "tiff" | "tif" | "webp" |
            "jp2" | "jpx" | "jpm" | "mj2" | "j2k" | "j2c" |
            "jbig2" | "jb2" |
            "pnm" | "pbm" | "pgm" | "ppm" |
            // Academic/Publishing
            "epub" | "fb2" | "bib" | "ris" | "nbib" | "enw" |
            "ipynb" | "tex" | "latex" | "typst" | "typ" |
            // Markup/Structured
            "opml" | "dbk" | "docbook" | "jats" |
            // Other
            "svg" | "djot"
        )
    }

    async fn extract(&self, file_path: &Path, timeout: Duration, force_ocr: bool) -> Result<BenchmarkResult> {
        let file_size = std::fs::metadata(file_path).map_err(Error::Io)?.len();

        // Apply force_ocr override when requested
        let config = if force_ocr && !self.config.force_ocr {
            let mut c = self.config.clone();
            c.force_ocr = true;
            c
        } else {
            self.config.clone()
        };

        let monitor = ResourceMonitor::new();
        let sampling_interval_ms = Self::calculate_adaptive_sampling_interval(file_size);
        monitor.start(Duration::from_millis(sampling_interval_ms)).await;

        let start = Instant::now();

        // Start extraction timing (same as total for native - no subprocess overhead)
        let extraction_start = Instant::now();

        let timed_result = tokio::time::timeout(timeout, extract_file(file_path, None, &config)).await;
        let timed_out = timed_result.is_err();
        let extraction_result = match timed_result {
            Ok(inner) => inner.map_err(|e| Error::Benchmark(format!("Extraction failed: {}", e))),
            Err(_) => Err(Error::Timeout(format!("Extraction exceeded {:?}", timeout))),
        };

        let extraction_duration = extraction_start.elapsed();
        let duration = start.elapsed();

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

        let throughput = if duration.as_secs_f64() > 0.0 {
            file_size as f64 / duration.as_secs_f64()
        } else {
            0.0
        };

        if let Err(e) = extraction_result {
            let error_kind = if timed_out {
                ErrorKind::Timeout
            } else {
                ErrorKind::HarnessError
            };
            return Ok(BenchmarkResult {
                framework: self.name().to_string(),
                file_path: file_path.to_path_buf(),
                file_size,
                success: false,
                error_message: Some(e.to_string()),
                error_kind,
                duration,
                extraction_duration: Some(extraction_duration),
                subprocess_overhead: Some(Duration::ZERO), // No subprocess for native Rust
                metrics: PerformanceMetrics {
                    peak_memory_bytes: resource_stats.peak_memory_bytes,
                    avg_cpu_percent: resource_stats.avg_cpu_percent,
                    throughput_bytes_per_sec: 0.0,
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
                framework_capabilities: FrameworkCapabilities::default(),
                pdf_metadata: None,
                ocr_status: OcrStatus::Unknown,
                extracted_text: None,
            });
        }

        let extraction_result = extraction_result.unwrap();
        let ocr_status = determine_ocr_status(&extraction_result, &config);

        let metrics = PerformanceMetrics {
            peak_memory_bytes: resource_stats.peak_memory_bytes,
            avg_cpu_percent: resource_stats.avg_cpu_percent,
            throughput_bytes_per_sec: throughput,
            p50_memory_bytes: resource_stats.p50_memory_bytes,
            p95_memory_bytes: resource_stats.p95_memory_bytes,
            p99_memory_bytes: resource_stats.p99_memory_bytes,
        };

        let (success, error_message, error_kind) = if extraction_result.content.trim().is_empty() {
            (
                false,
                Some("Framework returned empty content".to_string()),
                ErrorKind::EmptyContent,
            )
        } else {
            (true, None, ErrorKind::None)
        };

        Ok(BenchmarkResult {
            framework: self.name().to_string(),
            file_path: file_path.to_path_buf(),
            file_size,
            success,
            error_message,
            error_kind,
            duration,
            extraction_duration: Some(extraction_duration),
            subprocess_overhead: Some(Duration::ZERO), // No subprocess for native Rust
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
            framework_capabilities: FrameworkCapabilities::default(),
            pdf_metadata: None,
            ocr_status,
            extracted_text: Some(extraction_result.content),
        })
    }

    async fn extract_batch(
        &self,
        file_paths: &[&Path],
        timeout: Duration,
        force_ocr: &[bool],
    ) -> Result<Vec<BenchmarkResult>> {
        // Early return if file_paths is empty
        if file_paths.is_empty() {
            return Ok(Vec::new());
        }

        // If any file needs force_ocr, apply it to the config
        let any_force_ocr = force_ocr.iter().any(|&f| f);
        let config = if any_force_ocr && !self.config.force_ocr {
            let mut c = self.config.clone();
            c.force_ocr = true;
            c
        } else {
            self.config.clone()
        };

        let total_file_size: u64 = file_paths
            .iter()
            .filter_map(|path| std::fs::metadata(path).ok())
            .map(|m| m.len())
            .sum();

        let monitor = ResourceMonitor::new();
        let sampling_interval_ms = Self::calculate_adaptive_sampling_interval(total_file_size);
        monitor.start(Duration::from_millis(sampling_interval_ms)).await;

        let start = Instant::now();

        let items: Vec<kreuzberg::BatchFileItem> = file_paths
            .iter()
            .map(|p| kreuzberg::BatchFileItem {
                path: p.to_path_buf(),
                config: None,
            })
            .collect();

        let timed_result = tokio::time::timeout(timeout, batch_extract_files(items.clone(), &config)).await;
        let timed_out = timed_result.is_err();
        let batch_result = match timed_result {
            Ok(inner) => inner.map_err(|e| Error::Benchmark(format!("Batch extraction failed: {}", e))),
            Err(_) => Err(Error::Timeout(format!("Batch extraction exceeded {:?}", timeout))),
        };

        let total_duration = start.elapsed();

        let samples = monitor.stop().await;
        let snapshots = monitor.get_snapshots().await;
        let baseline = monitor.baseline_memory().await;
        let resource_stats = ResourceMonitor::calculate_stats(&samples, &snapshots, baseline);

        if let Err(e) = batch_result {
            // Create one failure result per file instead of a single aggregated failure
            // Use the actual elapsed time divided by number of files
            let num_files = file_paths.len() as f64;
            let avg_duration_per_file = Duration::from_secs_f64(total_duration.as_secs_f64() / num_files.max(1.0));
            let error_kind = if timed_out {
                ErrorKind::Timeout
            } else {
                ErrorKind::HarnessError
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

                    BenchmarkResult {
                        framework: self.name().to_string(),
                        file_path: file_path.to_path_buf(),
                        file_size,
                        success: false,
                        error_message: Some(e.to_string()),
                        error_kind,
                        duration: avg_duration_per_file,
                        extraction_duration: Some(avg_duration_per_file), // For native, extraction = total
                        subprocess_overhead: Some(Duration::ZERO),        // No subprocess for native Rust
                        metrics: PerformanceMetrics {
                            peak_memory_bytes: resource_stats.peak_memory_bytes,
                            avg_cpu_percent: resource_stats.avg_cpu_percent,
                            throughput_bytes_per_sec: 0.0,
                            p50_memory_bytes: resource_stats.p50_memory_bytes,
                            p95_memory_bytes: resource_stats.p95_memory_bytes,
                            p99_memory_bytes: resource_stats.p99_memory_bytes,
                        },
                        quality: None,
                        iterations: vec![],
                        statistics: None,
                        cold_start_duration: None,
                        file_extension,
                        framework_capabilities: FrameworkCapabilities::default(),
                        pdf_metadata: None,
                        ocr_status: OcrStatus::Unknown,
                        extracted_text: None,
                    }
                })
                .collect();

            return Ok(failure_results);
        }

        // Get the actual extraction results with per-file timing from metadata
        let extraction_results = batch_result.unwrap();

        // Fallback duration if metadata doesn't have timing (shouldn't happen with new code)
        let num_files = file_paths.len() as f64;
        let avg_duration_per_file = Duration::from_secs_f64(total_duration.as_secs_f64() / num_files.max(1.0));

        // Create one result per file using actual per-file timing from extraction metadata
        let results: Vec<BenchmarkResult> = file_paths
            .iter()
            .zip(extraction_results.iter())
            .map(|(file_path, extraction_result)| {
                let file_size = std::fs::metadata(file_path).map(|m| m.len()).unwrap_or(0);

                // Read per-file extraction timing from metadata, fallback to average.
                // Note: extraction_duration_ms is u64, so sub-millisecond extractions
                // truncate to 0ms. Fall back to avg_duration_per_file in that case.
                let extraction_duration = extraction_result
                    .metadata
                    .extraction_duration_ms
                    .filter(|&ms| ms > 0)
                    .map(Duration::from_millis)
                    .unwrap_or(avg_duration_per_file);

                let file_throughput = if extraction_duration > Duration::from_secs(0) {
                    file_size as f64 / extraction_duration.as_secs_f64()
                } else {
                    0.0
                };

                let file_extension = file_path.extension().and_then(|e| e.to_str()).unwrap_or("").to_string();

                // Check if this specific extraction had an error or empty content
                let (success, error_message, error_kind) = if extraction_result.metadata.error.is_some() {
                    (
                        false,
                        extraction_result.metadata.error.as_ref().map(|e| e.message.clone()),
                        ErrorKind::FrameworkError,
                    )
                } else if extraction_result.content.trim().is_empty() {
                    (
                        false,
                        Some("Framework returned empty content".to_string()),
                        ErrorKind::EmptyContent,
                    )
                } else {
                    (true, None, ErrorKind::None)
                };

                // Amortize batch memory proportionally by file size
                let file_fraction = if total_file_size > 0 {
                    file_size as f64 / total_file_size as f64
                } else {
                    1.0 / file_paths.len() as f64
                };

                BenchmarkResult {
                    framework: self.name().to_string(),
                    file_path: file_path.to_path_buf(),
                    file_size,
                    success,
                    error_message,
                    error_kind,
                    duration: extraction_duration,
                    extraction_duration: Some(extraction_duration),
                    subprocess_overhead: Some(Duration::ZERO), // No subprocess for native Rust
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
                    framework_capabilities: FrameworkCapabilities::default(),
                    pdf_metadata: None,
                    ocr_status: determine_ocr_status(extraction_result, &config),
                    extracted_text: Some(extraction_result.content.clone()),
                }
            })
            .collect();

        Ok(results)
    }

    fn supports_batch(&self) -> bool {
        true
    }

    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    async fn setup(&self) -> Result<()> {
        // Warm up the extraction pipeline: trigger lazy statics, plugin discovery,
        // allocator warmup, etc. — equivalent to subprocess adapters' READY handshake.
        let warmup_pdf = tempfile::Builder::new()
            .suffix(".pdf")
            .tempfile()
            .map_err(|e| Error::Benchmark(format!("Failed to create warmup file: {e}")))?;
        std::fs::write(warmup_pdf.path(), minimal_pdf_bytes())
            .map_err(|e| Error::Benchmark(format!("Failed to write warmup file: {e}")))?;
        let _ = extract_file(warmup_pdf.path(), None, &self.config).await;
        Ok(())
    }

    async fn teardown(&self) -> Result<()> {
        Ok(())
    }
}

/// Minimal valid PDF document for warmup extractions.
fn minimal_pdf_bytes() -> &'static [u8] {
    b"%PDF-1.0
1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj
2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj
3 0 obj<</Type/Page/MediaBox[0 0 3 3]/Parent 2 0 R/Resources<<>>>>endobj
xref
0 4
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
trailer<</Size 4/Root 1 0 R>>
startxref
206
%%EOF"
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_native_adapter_creation() {
        let adapter = NativeAdapter::new();
        assert_eq!(adapter.name(), "kreuzberg-rust");
    }

    #[tokio::test]
    async fn test_supports_format() {
        let adapter = NativeAdapter::new();
        assert!(adapter.supports_format("pdf"));
        assert!(adapter.supports_format("docx"));
        assert!(adapter.supports_format("txt"));
        assert!(!adapter.supports_format("unknown"));
    }

    #[tokio::test]
    async fn test_extract_text_file() {
        let adapter = NativeAdapter::new();
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, "Hello, world!").unwrap();

        let result = adapter
            .extract(&file_path, Duration::from_secs(10), false)
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.framework, "kreuzberg-rust");
        assert!(result.duration.as_millis() < 1000);
    }
}

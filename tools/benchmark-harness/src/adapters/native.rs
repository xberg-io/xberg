//! Native Kreuzberg Rust adapter
//!
//! This adapter uses the Kreuzberg Rust core library directly for maximum performance.
//! It serves as the baseline for comparing language bindings.

use crate::adapter::FrameworkAdapter;
use crate::monitoring::ResourceMonitor;
use crate::types::{BenchmarkResult, FrameworkCapabilities, OcrStatus, PerformanceMetrics};
use crate::{Error, Result};
use async_trait::async_trait;
use kreuzberg::{ExtractionConfig, batch_extract_file, extract_file};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

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

    /// Determine OCR status based on extraction configuration
    ///
    /// Returns:
    /// - `OcrStatus::Used` if OCR is enabled in config (ocr.is_some() or force_ocr is true)
    /// - `OcrStatus::NotUsed` if OCR is explicitly disabled (ocr.is_none() and force_ocr is false)
    fn get_ocr_status(&self) -> OcrStatus {
        if self.config.ocr.is_some() || self.config.force_ocr {
            OcrStatus::Used
        } else {
            OcrStatus::NotUsed
        }
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
            "pdf" | "docx" | "doc" | "odt" | "pptx" | "ppsx" | "pptm" | "ppt" |
            "xlsx" | "xlsm" | "xlsb" | "xlam" | "xla" | "xls" | "ods" |
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
            "jp2" | "jpx" | "jpm" | "mj2" | "pnm" | "pbm" | "pgm" | "ppm" |
            // Academic/Publishing
            "epub" | "bib" | "ipynb" | "tex" | "latex" | "typst" | "typ" |
            // Other
            "svg" | "djot"
        )
    }

    async fn extract(&self, file_path: &Path, timeout: Duration) -> Result<BenchmarkResult> {
        let file_size = std::fs::metadata(file_path).map_err(Error::Io)?.len();

        let monitor = ResourceMonitor::new();
        let sampling_interval_ms = Self::calculate_adaptive_sampling_interval(file_size);
        monitor.start(Duration::from_millis(sampling_interval_ms)).await;

        let start = Instant::now();

        // Start extraction timing (same as total for native - no subprocess overhead)
        let extraction_start = Instant::now();

        let extraction_result = tokio::time::timeout(timeout, extract_file(file_path, None, &self.config))
            .await
            .map_err(|_| Error::Timeout(format!("Extraction exceeded {:?}", timeout)))?
            .map_err(|e| Error::Benchmark(format!("Extraction failed: {}", e)));

        let extraction_duration = extraction_start.elapsed();
        let duration = start.elapsed();

        // Capture extracted text for quality assessment
        let extracted_text = extraction_result.as_ref().ok().map(|r| r.content.clone());

        let samples = monitor.stop().await;
        let snapshots = monitor.get_snapshots().await;
        let resource_stats = ResourceMonitor::calculate_stats(&samples, &snapshots);

        let throughput = if duration.as_secs_f64() > 0.0 {
            file_size as f64 / duration.as_secs_f64()
        } else {
            0.0
        };

        if let Err(e) = extraction_result {
            return Ok(BenchmarkResult {
                framework: self.name().to_string(),
                file_path: file_path.to_path_buf(),
                file_size,
                success: false,
                error_message: Some(e.to_string()),
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
                ocr_status: self.get_ocr_status(),
                extracted_text: None,
            });
        }

        let metrics = PerformanceMetrics {
            peak_memory_bytes: resource_stats.peak_memory_bytes,
            avg_cpu_percent: resource_stats.avg_cpu_percent,
            throughput_bytes_per_sec: throughput,
            p50_memory_bytes: resource_stats.p50_memory_bytes,
            p95_memory_bytes: resource_stats.p95_memory_bytes,
            p99_memory_bytes: resource_stats.p99_memory_bytes,
        };

        Ok(BenchmarkResult {
            framework: self.name().to_string(),
            file_path: file_path.to_path_buf(),
            file_size,
            success: true,
            error_message: None,
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
            ocr_status: self.get_ocr_status(),
            extracted_text,
        })
    }

    async fn extract_batch(&self, file_paths: &[&Path], timeout: Duration) -> Result<Vec<BenchmarkResult>> {
        // Early return if file_paths is empty
        if file_paths.is_empty() {
            return Ok(Vec::new());
        }

        let total_file_size: u64 = file_paths
            .iter()
            .filter_map(|path| std::fs::metadata(path).ok())
            .map(|m| m.len())
            .sum();

        let monitor = ResourceMonitor::new();
        let sampling_interval_ms = Self::calculate_adaptive_sampling_interval(total_file_size);
        monitor.start(Duration::from_millis(sampling_interval_ms)).await;

        let start = Instant::now();

        let paths: Vec<PathBuf> = file_paths.iter().map(|p| p.to_path_buf()).collect();

        let batch_result = tokio::time::timeout(timeout, batch_extract_file(paths.clone(), &self.config))
            .await
            .map_err(|_| Error::Timeout(format!("Batch extraction exceeded {:?}", timeout)))?
            .map_err(|e| Error::Benchmark(format!("Batch extraction failed: {}", e)));

        let total_duration = start.elapsed();

        let samples = monitor.stop().await;
        let snapshots = monitor.get_snapshots().await;
        let resource_stats = ResourceMonitor::calculate_stats(&samples, &snapshots);

        if let Err(e) = batch_result {
            // Create one failure result per file instead of a single aggregated failure
            // Use the actual elapsed time divided by number of files
            let num_files = file_paths.len() as f64;
            let avg_duration_per_file = Duration::from_secs_f64(total_duration.as_secs_f64() / num_files.max(1.0));

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
                        ocr_status: self.get_ocr_status(),
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

                // Read per-file extraction timing from metadata, fallback to average
                let extraction_duration = extraction_result
                    .metadata
                    .extraction_duration_ms
                    .map(Duration::from_millis)
                    .unwrap_or(avg_duration_per_file);

                let file_throughput = if extraction_duration > Duration::from_secs(0) {
                    file_size as f64 / extraction_duration.as_secs_f64()
                } else {
                    0.0
                };

                let file_extension = file_path.extension().and_then(|e| e.to_str()).unwrap_or("").to_string();

                // Check if this specific extraction had an error
                let (success, error_message) = if extraction_result.metadata.error.is_some() {
                    (
                        false,
                        extraction_result.metadata.error.as_ref().map(|e| e.message.clone()),
                    )
                } else {
                    (true, None)
                };

                BenchmarkResult {
                    framework: self.name().to_string(),
                    file_path: file_path.to_path_buf(),
                    file_size,
                    success,
                    error_message,
                    duration: extraction_duration,
                    extraction_duration: Some(extraction_duration),
                    subprocess_overhead: Some(Duration::ZERO), // No subprocess for native Rust
                    metrics: PerformanceMetrics {
                        peak_memory_bytes: resource_stats.peak_memory_bytes,
                        avg_cpu_percent: resource_stats.avg_cpu_percent,
                        throughput_bytes_per_sec: file_throughput,
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
                    ocr_status: self.get_ocr_status(),
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
        Ok(())
    }

    async fn teardown(&self) -> Result<()> {
        Ok(())
    }
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

        let result = adapter.extract(&file_path, Duration::from_secs(10)).await.unwrap();

        assert!(result.success);
        assert_eq!(result.framework, "kreuzberg-rust");
        assert!(result.duration.as_millis() < 1000);
    }
}

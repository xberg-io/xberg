//! Framework adapter system
//!
//! Adapters provide a unified interface for extracting content across different
//! extraction frameworks (both Xberg language bindings and open source alternatives).
//! This allows benchmarking any extraction framework against the same test fixtures.

use crate::{
    Error, Result,
    config::BenchmarkMode,
    provenance::ExecutableProvenance,
    types::{BatchCapability, BenchmarkResult, OutputFormat},
};
use async_trait::async_trait;
use std::path::Path;
use std::time::Duration;

/// Unified interface for document extraction frameworks
///
/// Implementations of this trait can extract content from documents using
/// different extraction frameworks (Xberg language bindings and open source alternatives).
#[async_trait]
pub trait FrameworkAdapter: Send + Sync {
    /// Get the framework name (e.g., "xberg-rust", "xberg-python")
    fn name(&self) -> &str;

    /// Check if this adapter supports the given file type
    ///
    /// # Arguments
    /// * `file_type` - File extension without dot (e.g., "pdf", "docx")
    fn supports_format(&self, file_type: &str) -> bool;

    /// Check if this adapter should skip a specific file
    ///
    /// Some adapters need to skip specific files that are known to cause
    /// issues (e.g., timeouts in WASM for very large OCR-heavy documents).
    ///
    /// # Arguments
    /// * `file_name` - The file name (not full path) to check
    fn should_skip_file(&self, _file_name: &str) -> bool {
        false
    }

    /// Get the output formats supported by this adapter
    ///
    /// # Returns
    /// * `Vec<OutputFormat>` - List of supported output formats
    fn supported_output_formats(&self) -> Vec<OutputFormat> {
        vec![OutputFormat::Plaintext]
    }

    /// Extract content from a document
    ///
    /// # Arguments
    /// * `file_path` - Path to the document to extract
    /// * `timeout` - Maximum time to wait for extraction
    /// * `force_ocr` - When true, force OCR even if the document has a text layer
    /// * `output_format` - Output format for extraction (markdown or plaintext)
    ///
    /// # Returns
    /// * `Ok(BenchmarkResult)` - Successful extraction with metrics
    /// * `Err(Error)` - Extraction failed
    async fn extract(
        &self,
        file_path: &Path,
        timeout: Duration,
        force_ocr: bool,
        output_format: OutputFormat,
    ) -> Result<BenchmarkResult>;

    /// Extract content from multiple documents using framework's batch API
    ///
    /// Frameworks with native batch support must override this method to use
    /// their optimized batch extraction API (e.g., Xberg's unified `extract_batch`).
    /// The default fails closed so batch benchmarks can never silently measure
    /// repeated single-file extraction.
    ///
    /// # Arguments
    /// * `file_paths` - Paths to documents to extract
    /// * `timeout` - Maximum time to wait for each extraction
    /// * `force_ocr` - Per-file force_ocr flags (must be same length as file_paths)
    /// * `output_format` - Output format for extraction
    ///
    /// # Returns
    /// * `Ok(Vec<BenchmarkResult>)` - Results for all files
    /// * `Err(Error)` - Batch extraction failed
    async fn extract_batch(
        &self,
        file_paths: &[&Path],
        timeout: Duration,
        force_ocr: &[bool],
        output_format: OutputFormat,
    ) -> Result<Vec<BenchmarkResult>> {
        let _ = (file_paths, timeout, force_ocr, output_format);
        Err(crate::Error::Config(format!(
            "framework '{}' does not expose a verified native batch API",
            self.name()
        )))
    }

    /// Return the verified batch API and timing semantics exposed by this adapter.
    fn batch_capability(&self) -> Option<BatchCapability> {
        None
    }

    /// Get version information for this framework
    fn version(&self) -> String {
        "unknown".to_string()
    }

    /// Return a path-free identity for the executable used by this adapter.
    fn executable_provenance(&self) -> Option<ExecutableProvenance> {
        None
    }

    /// Return the executable identity for the entry point used in the selected mode.
    fn executable_provenance_for_mode(&self, _mode: BenchmarkMode) -> Option<ExecutableProvenance> {
        self.executable_provenance()
    }

    /// Requested and effective worker counts, when the adapter exposes a worker control.
    fn worker_provenance(&self, requested: usize) -> (Option<usize>, Option<usize>) {
        (Some(requested), Some(requested))
    }

    /// Perform any necessary setup before benchmarking
    async fn setup(&self) -> Result<()> {
        Ok(())
    }

    /// Perform any necessary cleanup after benchmarking
    async fn teardown(&self) -> Result<()> {
        Ok(())
    }

    /// Warm up the framework by performing a test extraction
    ///
    /// This is called once before benchmarking to get the framework into a warm state.
    /// It measures the cold start time (framework load + first extraction).
    ///
    /// The default implementation performs a single extraction on the provided warmup file.
    ///
    /// # Arguments
    /// * `warmup_file` - Path to a small test file for warmup
    /// * `timeout` - Maximum time to wait for warmup
    /// * `output_format` - Output format for warmup extraction
    ///
    /// # Returns
    /// * `Ok(Duration)` - Cold start duration (framework load + first extraction)
    /// * `Err(Error)` - Warmup failed
    async fn warmup(&self, warmup_file: &Path, timeout: Duration, output_format: OutputFormat) -> Result<Duration> {
        let start = std::time::Instant::now();
        let result = self.extract(warmup_file, timeout, false, output_format).await?;
        if !result.success {
            return Err(Error::Benchmark(format!(
                "warmup extraction for '{}' failed: {}",
                self.name(),
                result
                    .error_message
                    .as_deref()
                    .unwrap_or("framework returned success=false")
            )));
        }
        Ok(start.elapsed())
    }
}

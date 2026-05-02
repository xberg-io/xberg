//! Framework adapter system
//!
//! Adapters provide a unified interface for extracting content across different
//! extraction frameworks (both Kreuzberg language bindings and open source alternatives).
//! This allows benchmarking any extraction framework against the same test fixtures.

use crate::{Result, types::BenchmarkResult};
use async_trait::async_trait;
use std::path::Path;
use std::time::Duration;

/// Unified interface for document extraction frameworks
///
/// Implementations of this trait can extract content from documents using
/// different extraction frameworks (Kreuzberg language bindings and open source alternatives).
#[async_trait]
pub trait FrameworkAdapter: Send + Sync {
    /// Get the framework name (e.g., "kreuzberg-rust", "kreuzberg-python")
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

    /// Extract content from a document
    ///
    /// # Arguments
    /// * `file_path` - Path to the document to extract
    /// * `timeout` - Maximum time to wait for extraction
    /// * `force_ocr` - When true, force OCR even if the document has a text layer
    ///
    /// # Returns
    /// * `Ok(BenchmarkResult)` - Successful extraction with metrics
    /// * `Err(Error)` - Extraction failed
    async fn extract(&self, file_path: &Path, timeout: Duration, force_ocr: bool) -> Result<BenchmarkResult>;

    /// Extract content from multiple documents using framework's batch API
    ///
    /// Frameworks with native batch support should override this method to use
    /// their optimized batch extraction API (e.g., Kreuzberg's `batch_extract_files()`).
    ///
    /// Default implementation calls `extract()` sequentially for each file.
    ///
    /// # Arguments
    /// * `file_paths` - Paths to documents to extract
    /// * `timeout` - Maximum time to wait for each extraction
    /// * `force_ocr` - Per-file force_ocr flags (must be same length as file_paths)
    ///
    /// # Returns
    /// * `Ok(Vec<BenchmarkResult>)` - Results for all files
    /// * `Err(Error)` - Batch extraction failed
    async fn extract_batch(
        &self,
        file_paths: &[&Path],
        timeout: Duration,
        force_ocr: &[bool],
    ) -> Result<Vec<BenchmarkResult>> {
        let mut results = Vec::new();
        for (i, path) in file_paths.iter().enumerate() {
            let fo = force_ocr.get(i).copied().unwrap_or(false);
            results.push(self.extract(path, timeout, fo).await?);
        }
        Ok(results)
    }

    /// Check if this adapter supports batch extraction
    ///
    /// Returns true if the adapter overrides `extract_batch()` with an optimized implementation.
    /// Default is false (uses sequential extraction).
    fn supports_batch(&self) -> bool {
        false
    }

    /// Get version information for this framework
    fn version(&self) -> String {
        "unknown".to_string()
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
    ///
    /// # Returns
    /// * `Ok(Duration)` - Cold start duration (framework load + first extraction)
    /// * `Err(Error)` - Warmup failed
    async fn warmup(&self, warmup_file: &Path, timeout: Duration) -> Result<Duration> {
        let start = std::time::Instant::now();
        let _ = self.extract(warmup_file, timeout, false).await?;
        Ok(start.elapsed())
    }
}

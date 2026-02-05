//! Batch extraction operations for concurrent processing.
//!
//! This module provides parallel extraction capabilities for processing
//! multiple files or byte arrays concurrently with automatic resource management.

use crate::core::config::ExtractionConfig;
use crate::types::{ErrorMetadata, ExtractionResult, Metadata};
use crate::{KreuzbergError, Result};
use std::borrow::Cow;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use super::bytes::extract_bytes;
use super::file::extract_file;

/// Extract content from multiple files concurrently.
///
/// This function processes multiple files in parallel, automatically managing
/// concurrency to prevent resource exhaustion. The concurrency limit can be
/// configured via `ExtractionConfig::max_concurrent_extractions` or defaults
/// to `num_cpus * 2`.
///
/// # Arguments
///
/// * `paths` - Vector of file paths to extract
/// * `config` - Extraction configuration
///
/// # Returns
///
/// A vector of `ExtractionResult` in the same order as the input paths.
///
/// # Errors
///
/// Individual file errors are captured in the result metadata. System errors
/// (IO, RuntimeError equivalents) will bubble up and fail the entire batch.
///
/// # Example
///
/// ```rust,no_run
/// use kreuzberg::core::extractor::batch_extract_file;
/// use kreuzberg::core::config::ExtractionConfig;
///
/// # async fn example() -> kreuzberg::Result<()> {
/// let config = ExtractionConfig::default();
/// let paths = vec!["doc1.pdf", "doc2.pdf"];
/// let results = batch_extract_file(paths, &config).await?;
/// println!("Processed {} files", results.len());
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "tokio-runtime")]
#[cfg_attr(feature = "otel", tracing::instrument(
    skip(config, paths),
    fields(
        extraction.batch_size = paths.len(),
    )
))]
pub async fn batch_extract_file(
    paths: Vec<impl AsRef<Path>>,
    config: &ExtractionConfig,
) -> Result<Vec<ExtractionResult>> {
    use tokio::sync::Semaphore;
    use tokio::task::JoinSet;

    if paths.is_empty() {
        return Ok(vec![]);
    }

    let config_arc = Arc::new(config.clone());

    let max_concurrent = config_arc
        .max_concurrent_extractions
        .unwrap_or_else(|| (num_cpus::get() as f64 * 1.5).ceil() as usize);
    let semaphore = Arc::new(Semaphore::new(max_concurrent));

    let mut tasks = JoinSet::new();

    for (index, path) in paths.into_iter().enumerate() {
        let path_buf = path.as_ref().to_path_buf();
        let config_clone = Arc::clone(&config_arc);
        let semaphore_clone = Arc::clone(&semaphore);

        tasks.spawn(async move {
            let _permit = semaphore_clone.acquire().await.unwrap();
            let start = Instant::now();
            let mut result =
                crate::core::batch_mode::with_batch_mode(async { extract_file(&path_buf, None, &config_clone).await })
                    .await;
            let elapsed_ms = start.elapsed().as_millis() as u64;

            // Add extraction timing to result metadata for benchmarking
            if let Ok(ref mut r) = result {
                r.metadata.extraction_duration_ms = Some(elapsed_ms);
            }

            (index, result, elapsed_ms)
        });
    }

    let mut results: Vec<Option<ExtractionResult>> = vec![None; tasks.len()];

    while let Some(task_result) = tasks.join_next().await {
        match task_result {
            Ok((index, Ok(result), _elapsed_ms)) => {
                // Timing already added to result.metadata.extraction_duration_ms
                results[index] = Some(result);
            }
            Ok((index, Err(e), elapsed_ms)) => {
                // All errors (including Io) should create error results
                // instead of causing early return that abandons running tasks
                let metadata = Metadata {
                    error: Some(ErrorMetadata {
                        error_type: format!("{:?}", e),
                        message: e.to_string(),
                    }),
                    extraction_duration_ms: Some(elapsed_ms),
                    ..Default::default()
                };

                results[index] = Some(ExtractionResult {
                    content: format!("Error: {}", e),
                    mime_type: Cow::Borrowed("text/plain"),
                    metadata,
                    tables: vec![],
                    detected_languages: None,
                    chunks: None,
                    images: None,
                    djot_content: None,
                    pages: None,
                    elements: None,
                });
            }
            Err(join_err) => {
                return Err(KreuzbergError::Other(format!("Task panicked: {}", join_err)));
            }
        }
    }

    #[allow(clippy::unwrap_used)]
    Ok(results.into_iter().map(|r| r.unwrap()).collect())
}

/// Extract content from multiple byte arrays concurrently.
///
/// This function processes multiple byte arrays in parallel, automatically managing
/// concurrency to prevent resource exhaustion. The concurrency limit can be
/// configured via `ExtractionConfig::max_concurrent_extractions` or defaults
/// to `num_cpus * 2`.
///
/// # Arguments
///
/// * `contents` - Vector of (bytes, mime_type) tuples
/// * `config` - Extraction configuration
///
/// # Returns
///
/// A vector of `ExtractionResult` in the same order as the input.
///
/// # Example
///
/// ```rust,no_run
/// use kreuzberg::core::extractor::batch_extract_bytes;
/// use kreuzberg::core::config::ExtractionConfig;
///
/// # async fn example() -> kreuzberg::Result<()> {
/// let config = ExtractionConfig::default();
/// let contents = vec![
///     (b"content 1".to_vec(), "text/plain".to_string()),
///     (b"content 2".to_vec(), "text/plain".to_string()),
/// ];
/// let results = batch_extract_bytes(contents, &config).await?;
/// println!("Processed {} items", results.len());
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "tokio-runtime")]
#[cfg_attr(feature = "otel", tracing::instrument(
    skip(config, contents),
    fields(
        extraction.batch_size = contents.len(),
    )
))]
pub async fn batch_extract_bytes(
    contents: Vec<(Vec<u8>, String)>,
    config: &ExtractionConfig,
) -> Result<Vec<ExtractionResult>> {
    use tokio::sync::Semaphore;
    use tokio::task::JoinSet;

    if contents.is_empty() {
        return Ok(vec![]);
    }

    let config_arc = Arc::new(config.clone());

    let max_concurrent = config_arc
        .max_concurrent_extractions
        .unwrap_or_else(|| (num_cpus::get() as f64 * 1.5).ceil() as usize);
    let semaphore = Arc::new(Semaphore::new(max_concurrent));

    let mut tasks = JoinSet::new();

    for (index, (bytes, mime_type)) in contents.into_iter().enumerate() {
        let config_clone = Arc::clone(&config_arc);
        let semaphore_clone = Arc::clone(&semaphore);

        tasks.spawn(async move {
            let _permit = semaphore_clone.acquire().await.unwrap();
            let start = Instant::now();
            let mut result = crate::core::batch_mode::with_batch_mode(async {
                extract_bytes(&bytes, &mime_type, &config_clone).await
            })
            .await;
            let elapsed_ms = start.elapsed().as_millis() as u64;

            // Add extraction timing to result metadata for benchmarking
            if let Ok(ref mut r) = result {
                r.metadata.extraction_duration_ms = Some(elapsed_ms);
            }

            (index, result, elapsed_ms)
        });
    }

    let mut results: Vec<Option<ExtractionResult>> = vec![None; tasks.len()];

    while let Some(task_result) = tasks.join_next().await {
        match task_result {
            Ok((index, Ok(result), _elapsed_ms)) => {
                // Timing already added to result.metadata.extraction_duration_ms
                results[index] = Some(result);
            }
            Ok((index, Err(e), elapsed_ms)) => {
                // All errors (including Io) should create error results
                // instead of causing early return that abandons running tasks
                let metadata = Metadata {
                    error: Some(ErrorMetadata {
                        error_type: format!("{:?}", e),
                        message: e.to_string(),
                    }),
                    extraction_duration_ms: Some(elapsed_ms),
                    ..Default::default()
                };

                results[index] = Some(ExtractionResult {
                    content: format!("Error: {}", e),
                    mime_type: Cow::Borrowed("text/plain"),
                    metadata,
                    tables: vec![],
                    detected_languages: None,
                    chunks: None,
                    images: None,
                    djot_content: None,
                    pages: None,
                    elements: None,
                });
            }
            Err(join_err) => {
                return Err(KreuzbergError::Other(format!("Task panicked: {}", join_err)));
            }
        }
    }

    #[allow(clippy::unwrap_used)]
    Ok(results.into_iter().map(|r| r.unwrap()).collect())
}

//! Batch extraction operations for concurrent processing.
//!
//! This module provides parallel extraction capabilities for processing
//! multiple files or byte arrays concurrently with automatic resource management.

use crate::core::config::ExtractionConfig;
use crate::core::config::extraction::FileExtractionConfig;
use crate::types::ExtractionResult;
use crate::{KreuzbergError, Result};
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use super::bytes::extract_bytes;
use super::file::extract_file;
use super::helpers::error_extraction_result;

/// Shared batch result collection: spawns tasks via callback, collects ordered results.
#[cfg(feature = "tokio-runtime")]
async fn collect_batch<F, Fut>(count: usize, config: &ExtractionConfig, spawn_task: F) -> Result<Vec<ExtractionResult>>
where
    F: Fn(usize, Arc<tokio::sync::Semaphore>) -> Fut,
    Fut: Future<Output = (usize, Result<ExtractionResult>, u64)> + Send + 'static,
{
    use tokio::sync::Semaphore;
    use tokio::task::JoinSet;

    if count == 0 {
        return Ok(vec![]);
    }

    let max_concurrent = config
        .max_concurrent_extractions
        .or_else(|| config.concurrency.as_ref().and_then(|c| c.max_threads))
        .unwrap_or_else(|| crate::core::config::concurrency::resolve_thread_budget(config.concurrency.as_ref()));
    let semaphore = Arc::new(Semaphore::new(max_concurrent));

    let mut tasks = JoinSet::new();

    for index in 0..count {
        let sem = Arc::clone(&semaphore);
        tasks.spawn(spawn_task(index, sem));
    }

    let mut results: Vec<Option<ExtractionResult>> = vec![None; count];

    while let Some(task_result) = tasks.join_next().await {
        match task_result {
            Ok((index, Ok(result), _elapsed_ms)) => {
                results[index] = Some(result);
            }
            Ok((index, Err(e), elapsed_ms)) => {
                results[index] = Some(error_extraction_result(&e, Some(elapsed_ms)));
            }
            Err(join_err) => {
                return Err(KreuzbergError::Other(format!("Task panicked: {}", join_err)));
            }
        }
    }

    #[allow(clippy::unwrap_used)]
    Ok(results.into_iter().map(|r| r.unwrap()).collect())
}

/// Run a single extraction task with semaphore gating, timing, optional timeout, and batch mode.
///
/// When `cancel_token` is provided and the timeout fires, the token is signalled so that
/// any blocking pdfium operations in progress can observe the cancellation at the next
/// inter-page checkpoint and stop early.
#[cfg(feature = "tokio-runtime")]
async fn run_timed_extraction<F, Fut>(
    index: usize,
    semaphore: Arc<tokio::sync::Semaphore>,
    timeout_secs: Option<u64>,
    cancel_token: Option<crate::cancellation::CancellationToken>,
    extract_fn: F,
) -> (usize, Result<ExtractionResult>, u64)
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<ExtractionResult>>,
{
    let _permit = semaphore.acquire().await.unwrap();
    let start = Instant::now();

    let extraction_future = crate::core::batch_mode::with_batch_mode(extract_fn());

    let mut result = match timeout_secs {
        Some(secs) => match tokio::time::timeout(std::time::Duration::from_secs(secs), extraction_future).await {
            Ok(inner) => inner,
            Err(_elapsed) => {
                // Signal the cancellation token so that any blocking pdfium thread can
                // detect it at the next inter-page checkpoint and stop processing.
                if let Some(ref token) = cancel_token {
                    token.cancel();
                }
                let elapsed_ms = start.elapsed().as_millis() as u64;
                Err(KreuzbergError::Timeout {
                    elapsed_ms,
                    limit_ms: secs * 1000,
                })
            }
        },
        None => extraction_future.await,
    };

    let elapsed_ms = start.elapsed().as_millis() as u64;

    if let Ok(ref mut r) = result {
        r.metadata.extraction_duration_ms = Some(elapsed_ms);
    }

    (index, result, elapsed_ms)
}

/// Resolve a per-file config against a base config. Returns owned config.
fn resolve_config(base: &ExtractionConfig, file_config: &Option<FileExtractionConfig>) -> ExtractionConfig {
    match file_config {
        Some(fc) => base.with_file_overrides(fc),
        None => base.clone(),
    }
}

/// Extract content from multiple files concurrently.
///
/// This function processes multiple files in parallel, automatically managing
/// concurrency to prevent resource exhaustion. The concurrency limit can be
/// configured via `ExtractionConfig::max_concurrent_extractions` or defaults
/// to `(num_cpus * 1.5).ceil()`.
///
/// Each file can optionally specify a [`FileExtractionConfig`] that overrides specific
/// fields from the batch-level `config`. Pass `None` for a file to use the batch defaults.
/// Batch-level settings like `max_concurrent_extractions` and `use_cache` are always
/// taken from the batch-level `config`.
///
/// # Arguments
///
/// * `items` - Vector of `(path, optional_file_config)` tuples. Pass `None` as the
///   config to use the batch-level defaults for that file.
/// * `config` - Batch-level extraction configuration (provides defaults and batch settings)
///
/// # Returns
///
/// A vector of `ExtractionResult` in the same order as the input items.
///
/// # Errors
///
/// Individual file errors are captured in the result metadata. System errors
/// (IO, RuntimeError equivalents) will bubble up and fail the entire batch.
///
/// # Examples
///
/// Simple usage with no per-file overrides:
///
/// ```rust,no_run
/// use kreuzberg::core::extractor::batch_extract_file;
/// use kreuzberg::core::config::ExtractionConfig;
/// use std::path::PathBuf;
///
/// # async fn example() -> kreuzberg::Result<()> {
/// let config = ExtractionConfig::default();
/// let items: Vec<(PathBuf, Option<kreuzberg::FileExtractionConfig>)> = vec![
///     ("doc1.pdf".into(), None),
///     ("doc2.pdf".into(), None),
/// ];
/// let results = batch_extract_file(items, &config).await?;
/// println!("Processed {} files", results.len());
/// # Ok(())
/// # }
/// ```
///
/// Per-file configuration overrides:
///
/// ```rust,no_run
/// use kreuzberg::core::extractor::batch_extract_file;
/// use kreuzberg::core::config::ExtractionConfig;
/// use kreuzberg::FileExtractionConfig;
/// use std::path::PathBuf;
///
/// # async fn example() -> kreuzberg::Result<()> {
/// let config = ExtractionConfig::default();
/// let items: Vec<(PathBuf, Option<FileExtractionConfig>)> = vec![
///     ("scan.pdf".into(), Some(FileExtractionConfig { force_ocr: Some(true), ..Default::default() })),
///     ("notes.txt".into(), None),
/// ];
/// let results = batch_extract_file(items, &config).await?;
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "tokio-runtime")]
#[cfg_attr(feature = "otel", tracing::instrument(
    skip(config, items),
    fields(
        extraction.batch_size = items.len(),
    )
))]
pub async fn batch_extract_file(
    items: Vec<(PathBuf, Option<FileExtractionConfig>)>,
    config: &ExtractionConfig,
) -> Result<Vec<ExtractionResult>> {
    let config_arc = Arc::new(config.clone());
    // Use Arc<Vec> for file items — paths are small, so keeping them all alive is fine.
    let items_arc = Arc::new(items);
    let count = items_arc.len();

    collect_batch(count, config, |index, sem| {
        let cfg = Arc::clone(&config_arc);
        let items = Arc::clone(&items_arc);
        async move {
            let (ref path, ref file_config) = items[index];
            let resolved = resolve_config(&cfg, file_config);
            let timeout = resolved.extraction_timeout_secs;
            let cancel_token = resolved.cancel_token.clone();
            run_timed_extraction(index, sem, timeout, cancel_token, || {
                let path = path.clone();
                async move { extract_file(&path, None, &resolved).await }
            })
            .await
        }
    })
    .await
}

/// Extract content from multiple byte arrays concurrently.
///
/// This function processes multiple byte arrays in parallel, automatically managing
/// concurrency to prevent resource exhaustion. The concurrency limit can be
/// configured via `ExtractionConfig::max_concurrent_extractions` or defaults
/// to `(num_cpus * 1.5).ceil()`.
///
/// Each item can optionally specify a [`FileExtractionConfig`] that overrides specific
/// fields from the batch-level `config`. Pass `None` as the config to use
/// the batch-level defaults for that item.
///
/// # Arguments
///
/// * `items` - Vector of `(bytes, mime_type, optional_file_config)` tuples
/// * `config` - Batch-level extraction configuration
///
/// # Returns
///
/// A vector of `ExtractionResult` in the same order as the input items.
///
/// # Examples
///
/// Simple usage with no per-item overrides:
///
/// ```rust,no_run
/// use kreuzberg::core::extractor::batch_extract_bytes;
/// use kreuzberg::core::config::ExtractionConfig;
///
/// # async fn example() -> kreuzberg::Result<()> {
/// let config = ExtractionConfig::default();
/// let items = vec![
///     (b"content 1".to_vec(), "text/plain".to_string(), None),
///     (b"content 2".to_vec(), "text/plain".to_string(), None),
/// ];
/// let results = batch_extract_bytes(items, &config).await?;
/// println!("Processed {} items", results.len());
/// # Ok(())
/// # }
/// ```
///
/// Per-item configuration overrides:
///
/// ```rust,no_run
/// use kreuzberg::core::extractor::batch_extract_bytes;
/// use kreuzberg::core::config::ExtractionConfig;
/// use kreuzberg::FileExtractionConfig;
///
/// # async fn example() -> kreuzberg::Result<()> {
/// let config = ExtractionConfig::default();
/// let items = vec![
///     (b"content".to_vec(), "text/plain".to_string(), None),
///     (b"<html>test</html>".to_vec(), "text/html".to_string(),
///      Some(FileExtractionConfig { force_ocr: Some(true), ..Default::default() })),
/// ];
/// let results = batch_extract_bytes(items, &config).await?;
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "tokio-runtime")]
#[cfg_attr(feature = "otel", tracing::instrument(
    skip(config, items),
    fields(
        extraction.batch_size = items.len(),
    )
))]
pub async fn batch_extract_bytes(
    items: Vec<(Vec<u8>, String, Option<FileExtractionConfig>)>,
    config: &ExtractionConfig,
) -> Result<Vec<ExtractionResult>> {
    let config_arc = Arc::new(config.clone());
    let count = items.len();

    // Move items into individually-indexed slots so each task can take ownership
    // of its bytes without cloning. This avoids the memory regression of
    // Arc<Vec<(Vec<u8>, ...)>> which would keep all byte arrays alive for the
    // entire batch duration.
    type BytesSlot = parking_lot::Mutex<Option<(Vec<u8>, String, Option<FileExtractionConfig>)>>;
    let slots: Arc<Vec<BytesSlot>> = Arc::new(
        items
            .into_iter()
            .map(|item| parking_lot::Mutex::new(Some(item)))
            .collect(),
    );

    collect_batch(count, config, |index, sem| {
        let cfg = Arc::clone(&config_arc);
        let slots = Arc::clone(&slots);
        async move {
            let (bytes, mime_type, file_config) = slots[index].lock().take().expect("batch item already consumed");
            let resolved = resolve_config(&cfg, &file_config);
            let timeout = resolved.extraction_timeout_secs;
            let cancel_token = resolved.cancel_token.clone();
            run_timed_extraction(index, sem, timeout, cancel_token, || async move {
                extract_bytes(&bytes, &mime_type, &resolved).await
            })
            .await
        }
    })
    .await
}

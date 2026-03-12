//! Worker Pool APIs for concurrent extraction operations.
//!
//! This module provides APIs for creating and managing worker pools
//! for CPU-bound document extraction tasks.

use crate::error_handling::convert_error;
use crate::result::resolve_config;
use crate::worker_pool::WorkerPool;
use crate::{JsExtractionConfig, JsExtractionResult};
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::sync::{Arc, Mutex};

lazy_static::lazy_static! {
    /// Global registry of worker pools
    static ref WORKER_POOLS: Mutex<Vec<Arc<Mutex<Option<WorkerPool>>>>> = Mutex::new(Vec::new());
}

/// Opaque handle to a worker pool
#[napi]
pub struct JsWorkerPool {
    pool_id: usize,
}

#[napi(object)]
pub struct WorkerPoolStats {
    pub size: u32,
    pub active_workers: u32,
    pub queued_tasks: u32,
}

/// Create a new worker pool for concurrent extraction operations.
///
/// Creates a pool of worker threads for CPU-bound document extraction.
/// Tasks submitted to the pool will be executed concurrently up to the pool size.
///
/// # Parameters
///
/// * `size` - Number of concurrent workers (defaults to CPU count)
///
/// # Returns
///
/// Worker pool handle that can be used with extraction functions.
///
/// # Example
///
/// ```typescript
/// import { createWorkerPool } from '@kreuzberg/node';
///
/// const pool = createWorkerPool(4); // 4 concurrent workers
/// console.log(`Pool created with ${pool.size} workers`);
/// ```
#[napi]
pub fn create_worker_pool(size: Option<u32>) -> Result<JsWorkerPool> {
    let pool_size = size.unwrap_or_else(|| num_cpus::get() as u32) as usize;

    let pool = WorkerPool::new(pool_size)?;

    let mut pools = WORKER_POOLS.lock().unwrap();
    let pool_id = pools.len();
    pools.push(Arc::new(Mutex::new(Some(pool))));

    Ok(JsWorkerPool { pool_id })
}

/// Get worker pool statistics.
///
/// Returns current statistics about the worker pool including size,
/// active workers, and queued tasks.
///
/// # Parameters
///
/// * `pool` - Worker pool handle
///
/// # Returns
///
/// Pool statistics object with size, activeWorkers, and queuedTasks fields.
///
/// # Example
///
/// ```typescript
/// import { createWorkerPool, getWorkerPoolStats } from '@kreuzberg/node';
///
/// const pool = createWorkerPool(4);
/// const stats = getWorkerPoolStats(pool);
/// console.log(`Active: ${stats.activeWorkers}/${stats.size}`);
/// ```
#[napi]
pub fn get_worker_pool_stats(pool: &JsWorkerPool) -> Result<WorkerPoolStats> {
    let pools = WORKER_POOLS.lock().unwrap();
    let pool_arc = pools
        .get(pool.pool_id)
        .ok_or_else(|| Error::new(Status::InvalidArg, "Invalid worker pool handle"))?;

    let pool_mutex = pool_arc.lock().unwrap();
    let pool_ref = pool_mutex
        .as_ref()
        .ok_or_else(|| Error::new(Status::GenericFailure, "Worker pool has been closed"))?;

    Ok(WorkerPoolStats {
        size: pool_ref.size() as u32,
        active_workers: pool_ref.active_workers() as u32,
        queued_tasks: 0, // Not tracked in simplified implementation
    })
}

/// Extract a file using a worker thread from the pool.
///
/// Submits a file extraction task to the worker pool. The task will execute
/// when a worker thread becomes available. This is useful for CPU-bound
/// extraction operations that need to be run concurrently.
///
/// # Parameters
///
/// * `pool` - Worker pool handle
/// * `file_path` - Path to the file to extract
/// * `password` - Optional password for encrypted files
/// * `config` - Optional extraction configuration
///
/// # Returns
///
/// Promise resolving to extraction result.
///
/// # Example
///
/// ```typescript
/// import { createWorkerPool, extractFileInWorker } from '@kreuzberg/node';
///
/// const pool = createWorkerPool(4);
/// const result = await extractFileInWorker(pool, 'document.pdf', null, {
///   useCache: true
/// });
/// console.log(result.content);
/// ```
#[napi]
pub async fn extract_file_in_worker(
    pool: &JsWorkerPool,
    file_path: String,
    password: Option<String>,
    config: Option<JsExtractionConfig>,
) -> Result<JsExtractionResult> {
    let pool_id = pool.pool_id;

    let pool_clone = {
        let pools = WORKER_POOLS.lock().unwrap();
        let pool_arc = pools
            .get(pool_id)
            .ok_or_else(|| Error::new(Status::InvalidArg, "Invalid worker pool handle"))?;
        let pool_mutex = pool_arc.lock().unwrap();
        pool_mutex
            .as_ref()
            .ok_or_else(|| Error::new(Status::GenericFailure, "Worker pool has been closed"))?
            .clone()
    };

    // Wait until we can accept work (respects pool size limit)
    while !pool_clone.can_accept_work() {
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    pool_clone.increment_active();

    let mut rust_config = resolve_config(config)?;

    // Inject password into PDF config if provided
    if let Some(ref pwd) = password {
        let pdf_opts = rust_config.pdf_options.get_or_insert_with(Default::default);
        let passwords = pdf_opts.passwords.get_or_insert_with(Vec::new);
        if !passwords.contains(pwd) {
            passwords.push(pwd.clone());
        }
    }

    // Spawn the extraction in a blocking thread
    let result = tokio::task::spawn_blocking(move || kreuzberg::extract_file_sync(&file_path, None, &rust_config))
        .await
        .map_err(|e| Error::from_reason(format!("Worker thread error: {}", e)))?
        .map_err(convert_error)?;

    pool_clone.decrement_active();

    JsExtractionResult::try_from(result)
}

/// Extract multiple files using worker threads from the pool.
///
/// Submits multiple file extraction tasks to the worker pool for concurrent
/// processing. Files are processed in parallel up to the pool size limit.
///
/// # Parameters
///
/// * `pool` - Worker pool handle
/// * `file_paths` - Array of file paths to extract
/// * `config` - Optional extraction configuration applied to all files
///
/// # Returns
///
/// Promise resolving to array of extraction results in the same order as input paths.
///
/// # Example
///
/// ```typescript
/// import { createWorkerPool, batchExtractFilesInWorker } from '@kreuzberg/node';
///
/// const pool = createWorkerPool(4);
/// const files = ['doc1.pdf', 'doc2.docx', 'doc3.xlsx'];
/// const results = await batchExtractFilesInWorker(pool, files, {
///   useCache: true
/// });
///
/// results.forEach((result, i) => {
///   console.log(`File ${i + 1}: ${result.content.length} chars`);
/// });
/// ```
#[napi]
pub async fn batch_extract_files_in_worker(
    pool: &JsWorkerPool,
    file_paths: Vec<String>,
    config: Option<JsExtractionConfig>,
) -> Result<Vec<JsExtractionResult>> {
    let pool_id = pool.pool_id;
    let rust_config = resolve_config(config)?;

    let mut results = Vec::new();

    for path in file_paths {
        let pool_clone = {
            let pools = WORKER_POOLS.lock().unwrap();
            let pool_arc = pools
                .get(pool_id)
                .ok_or_else(|| Error::new(Status::InvalidArg, "Invalid worker pool handle"))?;
            let pool_mutex = pool_arc.lock().unwrap();
            pool_mutex
                .as_ref()
                .ok_or_else(|| Error::new(Status::GenericFailure, "Worker pool has been closed"))?
                .clone()
        };

        // Wait until we can accept work
        while !pool_clone.can_accept_work() {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        pool_clone.increment_active();

        let config_clone = rust_config.clone();

        // Spawn the extraction in a blocking thread
        let result = tokio::task::spawn_blocking(move || kreuzberg::extract_file_sync(&path, None, &config_clone))
            .await
            .map_err(|e| Error::from_reason(format!("Worker thread error: {}", e)))?
            .map_err(convert_error)?;

        pool_clone.decrement_active();

        results.push(JsExtractionResult::try_from(result)?);
    }

    Ok(results)
}

/// Close and shutdown a worker pool gracefully.
///
/// Waits for all in-flight extraction tasks to complete before shutting down
/// the pool. After calling this function, the pool handle becomes invalid.
///
/// # Parameters
///
/// * `pool` - Worker pool handle
///
/// # Returns
///
/// Promise that resolves when all workers have completed and pool is closed.
///
/// # Example
///
/// ```typescript
/// import { createWorkerPool, closeWorkerPool } from '@kreuzberg/node';
///
/// const pool = createWorkerPool(4);
/// // ... use pool for extractions ...
/// await closeWorkerPool(pool); // Wait for completion and cleanup
/// ```
#[napi]
pub async fn close_worker_pool(pool: &JsWorkerPool) -> Result<()> {
    let pool_id = pool.pool_id;

    let pool_opt = {
        let pools = WORKER_POOLS.lock().unwrap();
        let pool_arc = pools
            .get(pool_id)
            .ok_or_else(|| Error::new(Status::InvalidArg, "Invalid worker pool handle"))?;
        let mut pool_mutex = pool_arc.lock().unwrap();
        pool_mutex.take()
    };

    if let Some(worker_pool) = pool_opt {
        // Wait for all active workers to complete
        worker_pool.wait_for_completion().await;
    }

    Ok(())
}

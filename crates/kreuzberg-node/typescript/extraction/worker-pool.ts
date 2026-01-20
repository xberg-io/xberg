/**
 * Worker pool management for concurrent document extraction.
 *
 * This module provides utilities for creating and managing worker pools that enable
 * concurrent extraction of documents using Node.js worker threads. Worker pools allow
 * multiple extraction operations to run in parallel with configurable pool sizes.
 *
 * **Usage Pattern**:
 * 1. Create a pool with `createWorkerPool(size)`
 * 2. Submit tasks with `extractFileInWorker()` or `batchExtractFilesInWorker()`
 * 3. Close the pool with `closeWorkerPool()` when done
 *
 * @internal This module is part of Layer 2 (extraction APIs).
 */

import { getBinding } from "../core/binding.js";
import { normalizeExtractionConfig } from "../core/config-normalizer.js";
import { convertResult } from "../core/type-converters.js";
import type { ExtractionConfig, ExtractionResult, WorkerPool, WorkerPoolStats } from "../types.js";

/**
 * Create a new worker pool for concurrent extraction operations.
 *
 * Creates a pool of worker threads that can process extraction tasks concurrently.
 * The pool manages a queue of pending tasks and distributes them across available workers.
 *
 * @param size - Optional number of workers in the pool. If not specified, defaults to the number of CPU cores.
 * @returns WorkerPool instance that can be used with extraction functions
 *
 * @example
 * ```typescript
 * import { createWorkerPool } from '@kreuzberg/node';
 *
 * // Create pool with default size (number of CPU cores)
 * const pool = createWorkerPool();
 *
 * // Create pool with 4 workers
 * const pool4 = createWorkerPool(4);
 * ```
 */
export function createWorkerPool(size?: number): WorkerPool {
	const binding = getBinding();
	const rawPool = binding.createWorkerPool(size);
	return rawPool as unknown as WorkerPool;
}

/**
 * Get statistics about a worker pool.
 *
 * Returns information about the pool's current state, including the number of active workers,
 * queued tasks, and total processed tasks.
 *
 * @param pool - The worker pool instance
 * @returns WorkerPoolStats with pool information
 *
 * @example
 * ```typescript
 * import { createWorkerPool, getWorkerPoolStats } from '@kreuzberg/node';
 *
 * const pool = createWorkerPool(4);
 * const stats = getWorkerPoolStats(pool);
 *
 * console.log(`Pool size: ${stats.size}`);
 * console.log(`Active workers: ${stats.activeWorkers}`);
 * console.log(`Queued tasks: ${stats.queuedTasks}`);
 * ```
 */
export function getWorkerPoolStats(pool: WorkerPool): WorkerPoolStats {
	const binding = getBinding();
	const rawStats = binding.getWorkerPoolStats(pool as unknown as Record<string, unknown>);
	return rawStats as unknown as WorkerPoolStats;
}

/**
 * Extract content from a single file using a worker pool (asynchronous).
 *
 * Submits an extraction task to the worker pool. The task is executed by one of the
 * available workers in the background, allowing other tasks to be processed concurrently.
 *
 * @param pool - The worker pool instance
 * @param filePath - Path to the file to extract
 * @param mimeTypeOrConfig - Optional MIME type or extraction configuration.
 *   If a string, treated as MIME type. If an object, treated as ExtractionConfig.
 *   If null, MIME type is auto-detected from file extension or content.
 * @param maybeConfig - Extraction configuration object. If null, uses default extraction settings.
 *   Only used if second parameter is a MIME type string.
 * @returns Promise<ExtractionResult> containing extracted content and metadata
 *
 * @throws {Error} If the file cannot be read or extraction fails
 *
 * @example
 * ```typescript
 * import { createWorkerPool, extractFileInWorker, closeWorkerPool } from '@kreuzberg/node';
 *
 * const pool = createWorkerPool(4);
 *
 * try {
 *   const files = ['doc1.pdf', 'doc2.docx', 'doc3.xlsx'];
 *   const results = await Promise.all(
 *     files.map(f => extractFileInWorker(pool, f))
 *   );
 *
 *   results.forEach((r, i) => {
 *     console.log(`${files[i]}: ${r.content.substring(0, 100)}...`);
 *   });
 * } finally {
 *   await closeWorkerPool(pool);
 * }
 * ```
 */
export async function extractFileInWorker(
	pool: WorkerPool,
	filePath: string,
	mimeTypeOrConfig?: string | null | ExtractionConfig,
	maybeConfig?: ExtractionConfig | null,
): Promise<ExtractionResult> {
	let mimeType: string | null = null;
	let config: ExtractionConfig | null = null;

	if (typeof mimeTypeOrConfig === "string") {
		mimeType = mimeTypeOrConfig;
		config = maybeConfig ?? null;
	} else if (mimeTypeOrConfig !== null && typeof mimeTypeOrConfig === "object") {
		config = mimeTypeOrConfig;
		mimeType = null;
	} else {
		config = maybeConfig ?? null;
		mimeType = null;
	}

	const normalizedConfig = normalizeExtractionConfig(config);
	const binding = getBinding();
	const rawResult = await binding.extractFileInWorker(
		pool as unknown as Record<string, unknown>,
		filePath,
		mimeType,
		normalizedConfig,
	);
	return convertResult(rawResult);
}

/**
 * Extract content from multiple files in parallel using a worker pool (asynchronous).
 *
 * Submits multiple extraction tasks to the worker pool for concurrent processing.
 * This is more efficient than using `extractFileInWorker` multiple times sequentially.
 *
 * @param pool - The worker pool instance
 * @param paths - Array of file paths to extract
 * @param config - Extraction configuration object (applies to all files). If null, uses default extraction settings.
 * @returns Promise<ExtractionResult[]> array of results (one per file, in same order)
 *
 * @throws {Error} If any file cannot be read or extraction fails
 *
 * @example
 * ```typescript
 * import { createWorkerPool, batchExtractFilesInWorker, closeWorkerPool } from '@kreuzberg/node';
 *
 * const pool = createWorkerPool(4);
 *
 * try {
 *   const files = ['invoice1.pdf', 'invoice2.pdf', 'invoice3.pdf'];
 *   const results = await batchExtractFilesInWorker(pool, files, {
 *     ocr: { backend: 'tesseract', language: 'eng' }
 *   });
 *
 *   const total = results.reduce((sum, r) => sum + extractAmount(r.content), 0);
 *   console.log(`Total: $${total}`);
 * } finally {
 *   await closeWorkerPool(pool);
 * }
 * ```
 */
export async function batchExtractFilesInWorker(
	pool: WorkerPool,
	paths: string[],
	config: ExtractionConfig | null = null,
): Promise<ExtractionResult[]> {
	const normalizedConfig = normalizeExtractionConfig(config);
	const binding = getBinding();
	const rawResults = await binding.batchExtractFilesInWorker(
		pool as unknown as Record<string, unknown>,
		paths,
		normalizedConfig,
	);
	return rawResults.map(convertResult);
}

/**
 * Close a worker pool and shut down all worker threads.
 *
 * Should be called when the pool is no longer needed to clean up resources
 * and gracefully shut down worker threads. Any pending tasks will be cancelled.
 *
 * @param pool - The worker pool instance to close
 * @returns Promise that resolves when the pool is fully closed
 *
 * @throws {Error} If pool shutdown fails
 *
 * @example
 * ```typescript
 * import { createWorkerPool, extractFileInWorker, closeWorkerPool } from '@kreuzberg/node';
 *
 * const pool = createWorkerPool(4);
 *
 * try {
 *   const result = await extractFileInWorker(pool, 'document.pdf');
 *   console.log(result.content);
 * } finally {
 *   // Clean up the pool
 *   await closeWorkerPool(pool);
 * }
 * ```
 */
export async function closeWorkerPool(pool: WorkerPool): Promise<void> {
	const binding = getBinding();
	await binding.closeWorkerPool(pool as unknown as Record<string, unknown>);
}

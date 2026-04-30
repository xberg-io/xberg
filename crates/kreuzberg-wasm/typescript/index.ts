/**
 * Kreuzberg - WebAssembly Bindings for Browser and Runtime Environments
 *
 * This module provides WebAssembly bindings for Kreuzberg document intelligence,
 * enabling high-performance document extraction in browser and JavaScript runtime environments.
 *
 * ## Features
 *
 * - Extract text, metadata, and tables from documents
 * - Support for multiple document formats (PDF, Office, images, etc.)
 * - Browser and runtime-compatible WASM bindings
 * - Type-safe TypeScript interfaces
 * - Runtime detection and feature capability checking
 * - Automatic type conversion and error handling
 *
 * ## Installation
 *
 * ```bash
 * npm install @kreuzberg/wasm
 * ```
 *
 * ## Basic Usage
 *
 * ```typescript
 * import { extractBytes, initWasm } from '@kreuzberg/wasm';
 *
 * // Initialize WASM module once at app startup
 * await initWasm();
 *
 * // Extract from bytes
 * const bytes = new Uint8Array(buffer);
 * const result = await extractBytes(bytes, 'application/pdf');
 * console.log(result.content);
 * ```
 *
 * ## Browser Usage with File Input
 *
 * ```typescript
 * import { extractBytes, initWasm } from '@kreuzberg/wasm';
 * import { fileToUint8Array } from '@kreuzberg/wasm/adapters/wasm-adapter';
 *
 * // Initialize once at app startup
 * await initWasm();
 *
 * // Handle file input
 * const fileInput = document.getElementById('file');
 * fileInput.addEventListener('change', async (e) => {
 *   const file = e.target.files?.[0];
 *   if (file) {
 *     const bytes = await fileToUint8Array(file);
 *     const result = await extractBytes(bytes, file.type);
 *     console.log(result.content);
 *   }
 * });
 * ```
 *
 * ## Runtime Detection
 *
 * ```typescript
 * import { detectRuntime, getWasmCapabilities } from '@kreuzberg/wasm';
 *
 * const runtime = detectRuntime();
 * const caps = getWasmCapabilities();
 *
 * if (caps.hasWorkers) {
 *   // Can use Web Workers for parallel processing
 * }
 * ```
 *
 * ## Configuration
 *
 * ```typescript
 * import { extractBytes, initWasm } from '@kreuzberg/wasm';
 * import type { ExtractionConfig } from '@kreuzberg/wasm';
 *
 * await initWasm();
 *
 * const config: ExtractionConfig = {
 *   ocr: {
 *     backend: 'tesseract',
 *     language: 'eng'
 *   },
 *   chunking: {
 *     maxChars: 1000,
 *     chunkOverlap: 100
 *   },
 *   images: {
 *     extractImages: true,
 *     targetDpi: 150
 *   }
 * };
 *
 * const result = await extractBytes(bytes, 'application/pdf', config);
 * ```
 */

// ============================================================================
// Type Exports
// ============================================================================

export type * from "./types.js";

// ============================================================================
// Initialization Exports
// ============================================================================

export { initializePdfiumAsync } from "./initialization/pdfium-loader.js";
export {
	getInitializationError,
	getVersion,
	getWasmModule,
	type InitWasmOptions,
	initWasm,
	isInitialized,
	type ModuleInfo,
	type WasmModule,
} from "./initialization/wasm-loader.js";

// ============================================================================
// Extraction Exports - Bytes
// ============================================================================

export { extractBytes, extractBytesSync } from "./extraction/bytes.js";

// ============================================================================
// Extraction Exports - Files
// ============================================================================

export { extractFile, extractFromFile } from "./extraction/files.js";

// ============================================================================
// Extraction Exports - Batch Operations
// ============================================================================

export {
	batchExtractBytes,
	batchExtractBytesSync,
	batchExtractFiles,
} from "./extraction/batch.js";

// ============================================================================
// OCR Exports
// ============================================================================

export { enableOcr } from "./ocr/enabler.js";

// ============================================================================
// Adapter Exports (Re-exported for convenience)
// ============================================================================

export {
	configToJS,
	fileToUint8Array,
	isValidExtractionResult,
	jsToExtractionResult,
	wrapWasmError,
} from "./adapters/wasm-adapter.js";

// ============================================================================
// OCR Backend Management Exports
// ============================================================================

export {
	clearOcrBackends,
	getOcrBackend,
	listOcrBackends,
	registerOcrBackend,
	unregisterOcrBackend,
} from "./ocr/registry.js";

export { TesseractWasmBackend } from "./ocr/tesseract-wasm-backend.js";

// ============================================================================
// Plugin Management Exports
// ============================================================================

export {
	clearPostProcessors,
	clearValidators,
	getPostProcessor,
	getValidator,
	listPostProcessors,
	listValidators,
	type PostProcessor,
	registerPostProcessor,
	registerValidator,
	unregisterPostProcessor,
	unregisterValidator,
	type Validator,
} from "./plugin-registry.js";

// ============================================================================
// MIME Utilities
// ============================================================================

export { detectMimeFromBytes, getExtensionsForMime } from "./mime/utilities.js";

// ============================================================================
// Runtime Exports
// ============================================================================

export {
	detectRuntime,
	getRuntimeInfo,
	getRuntimeVersion,
	getWasmCapabilities,
	hasBigInt,
	hasBlob,
	hasFileApi,
	hasModuleWorkers,
	hasSharedArrayBuffer,
	hasWasm,
	hasWasmStreaming,
	hasWorkers,
	isBrowser,
	isBun,
	isCloudflareWorkers,
	isDeno,
	isEdgeEnvironment,
	isEdgeRuntime,
	isNode,
	isServerEnvironment,
	isWebEnvironment,
	type RuntimeType,
	type WasmCapabilities,
} from "./runtime.js";

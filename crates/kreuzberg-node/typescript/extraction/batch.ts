/**
 * Batch extraction APIs for processing multiple documents.
 *
 * This module provides synchronous and asynchronous functions for extracting content
 * from multiple files or byte arrays in parallel. Batch operations offer better
 * performance and memory management compared to calling single extraction functions
 * in a loop.
 *
 * **Benefits of Batch Processing**:
 * - Parallel processing in Rust for maximum performance
 * - Optimized memory usage across all extractions
 * - More reliable for large-scale document processing
 *
 * @internal This module is part of Layer 2 (extraction APIs).
 */

import { assertUint8ArrayList } from "../core/assertions.js";
import { getBinding } from "../core/binding.js";
import { normalizeExtractionConfig } from "../core/config-normalizer.js";
import { convertResult } from "../core/type-converters.js";
import type { ExtractionConfig, ExtractionResult } from "../types.js";

/**
 * Extract content from multiple files in parallel (synchronous).
 *
 * **Recommended for**: Processing multiple documents efficiently with better
 * performance and memory management compared to individual `extractFileSync()` calls.
 *
 * **Benefits**:
 * - Parallel processing in Rust for maximum performance
 * - Optimized memory usage across all extractions
 * - More reliable for batch document processing
 *
 * @param paths - List of file paths to extract (absolute or relative paths)
 * @param config - Extraction configuration object. If null, uses default extraction settings.
 * @returns Array of ExtractionResults (one per file, in same order as input)
 * @throws {Error} If any file cannot be read or parsed
 * @throws {ParsingError} When any document format is invalid or corrupted
 * @throws {OcrError} When OCR processing fails (if OCR is enabled)
 * @throws {ValidationError} When any extraction result fails validation (if validators registered)
 * @throws {KreuzbergError} For other extraction-related failures
 *
 * @example
 * ```typescript
 * import { batchExtractFilesSync } from '@kreuzberg/node';
 *
 * const files = ['doc1.pdf', 'doc2.docx', 'doc3.xlsx'];
 * const results = batchExtractFilesSync(files);
 *
 * results.forEach((result, i) => {
 *   console.log(`File ${files[i]}: ${result.content.substring(0, 100)}...`);
 * });
 * ```
 */
export function batchExtractFilesSync(paths: string[], config: ExtractionConfig | null = null): ExtractionResult[] {
	const normalizedConfig = normalizeExtractionConfig(config);
	const rawResults = getBinding().batchExtractFilesSync(paths, normalizedConfig);
	return rawResults.map(convertResult);
}

/**
 * Extract content from multiple files in parallel (asynchronous).
 *
 * **Recommended for**: Processing multiple documents efficiently with better
 * performance and memory management compared to individual `extractFile()` calls.
 *
 * **Benefits**:
 * - Parallel processing in Rust for maximum performance
 * - Optimized memory usage across all extractions
 * - More reliable for batch document processing
 *
 * @param paths - List of file paths to extract (absolute or relative paths)
 * @param config - Extraction configuration object. If null, uses default extraction settings.
 * @returns Promise resolving to array of ExtractionResults (one per file, in same order as input)
 * @throws {Error} If any file cannot be read or parsed
 * @throws {ParsingError} When any document format is invalid or corrupted
 * @throws {OcrError} When OCR processing fails (if OCR is enabled)
 * @throws {ValidationError} When any extraction result fails validation (if validators registered)
 * @throws {KreuzbergError} For other extraction-related failures
 *
 * @example
 * ```typescript
 * import { batchExtractFiles } from '@kreuzberg/node';
 *
 * const files = ['invoice1.pdf', 'invoice2.pdf', 'invoice3.pdf'];
 * const results = await batchExtractFiles(files, {
 *   ocr: { backend: 'tesseract', language: 'eng' }
 * });
 *
 * // Process all results
 * const totalAmount = results
 *   .map(r => extractAmount(r.content))
 *   .reduce((a, b) => a + b, 0);
 * ```
 */
export async function batchExtractFiles(
	paths: string[],
	config: ExtractionConfig | null = null,
): Promise<ExtractionResult[]> {
	const normalizedConfig = normalizeExtractionConfig(config);
	const rawResults = await getBinding().batchExtractFiles(paths, normalizedConfig);
	return rawResults.map(convertResult);
}

/**
 * Extract content from multiple byte arrays in parallel (synchronous).
 *
 * **Recommended for**: Processing multiple documents from memory efficiently with better
 * performance and memory management compared to individual `extractBytesSync()` calls.
 *
 * **Benefits**:
 * - Parallel processing in Rust for maximum performance
 * - Optimized memory usage across all extractions
 * - More reliable for batch document processing
 *
 * @param dataList - List of file contents as Uint8Arrays (must be same length as mimeTypes)
 * @param mimeTypes - List of MIME types (one per data item, required for accurate format detection)
 * @param config - Extraction configuration object. If null, uses default extraction settings.
 * @returns Array of ExtractionResults (one per data item, in same order as input)
 * @throws {TypeError} When dataList contains non-Uint8Array items or length mismatch with mimeTypes
 * @throws {Error} If any data cannot be read or parsed
 * @throws {ParsingError} When any document format is invalid or corrupted
 * @throws {OcrError} When OCR processing fails (if OCR is enabled)
 * @throws {ValidationError} When any extraction result fails validation (if validators registered)
 * @throws {KreuzbergError} For other extraction-related failures
 *
 * @example
 * ```typescript
 * import { batchExtractBytesSync } from '@kreuzberg/node';
 * import { readFileSync } from 'fs';
 *
 * const files = ['doc1.pdf', 'doc2.docx', 'doc3.xlsx'];
 * const dataList = files.map(f => readFileSync(f));
 * const mimeTypes = ['application/pdf', 'application/vnd.openxmlformats-officedocument.wordprocessingml.document', 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet'];
 *
 * const results = batchExtractBytesSync(dataList, mimeTypes);
 * results.forEach((result, i) => {
 *   console.log(`File ${files[i]}: ${result.content.substring(0, 100)}...`);
 * });
 * ```
 */
export function batchExtractBytesSync(
	dataList: Uint8Array[],
	mimeTypes: string[],
	config: ExtractionConfig | null = null,
): ExtractionResult[] {
	const buffers = assertUint8ArrayList(dataList, "dataList").map((data) => Buffer.from(data));

	if (buffers.length !== mimeTypes.length) {
		throw new TypeError("dataList and mimeTypes must have the same length");
	}

	const normalizedConfig = normalizeExtractionConfig(config);
	const rawResults = getBinding().batchExtractBytesSync(buffers, mimeTypes, normalizedConfig);
	return rawResults.map(convertResult);
}

/**
 * Extract content from multiple byte arrays in parallel (asynchronous).
 *
 * **Recommended for**: Processing multiple documents from memory efficiently with better
 * performance and memory management compared to individual `extractBytes()` calls.
 *
 * **Benefits**:
 * - Parallel processing in Rust for maximum performance
 * - Optimized memory usage across all extractions
 * - More reliable for batch document processing
 *
 * @param dataList - List of file contents as Uint8Arrays (must be same length as mimeTypes)
 * @param mimeTypes - List of MIME types (one per data item, required for accurate format detection)
 * @param config - Extraction configuration object. If null, uses default extraction settings.
 * @returns Promise resolving to array of ExtractionResults (one per data item, in same order as input)
 * @throws {TypeError} When dataList contains non-Uint8Array items or length mismatch with mimeTypes
 * @throws {Error} If any data cannot be read or parsed
 * @throws {ParsingError} When any document format is invalid or corrupted
 * @throws {OcrError} When OCR processing fails (if OCR is enabled)
 * @throws {ValidationError} When any extraction result fails validation (if validators registered)
 * @throws {KreuzbergError} For other extraction-related failures
 *
 * @example
 * ```typescript
 * import { batchExtractBytes } from '@kreuzberg/node';
 * import { readFile } from 'fs/promises';
 *
 * const files = ['invoice1.pdf', 'invoice2.pdf', 'invoice3.pdf'];
 * const dataList = await Promise.all(files.map(f => readFile(f)));
 * const mimeTypes = files.map(() => 'application/pdf');
 *
 * const results = await batchExtractBytes(dataList, mimeTypes, {
 *   ocr: { backend: 'tesseract', language: 'eng' }
 * });
 *
 * // Process all results
 * const totalAmount = results
 *   .map(r => extractAmount(r.content))
 *   .reduce((a, b) => a + b, 0);
 * ```
 */
export async function batchExtractBytes(
	dataList: Uint8Array[],
	mimeTypes: string[],
	config: ExtractionConfig | null = null,
): Promise<ExtractionResult[]> {
	const buffers = assertUint8ArrayList(dataList, "dataList").map((data) => Buffer.from(data));

	if (buffers.length !== mimeTypes.length) {
		throw new TypeError("dataList and mimeTypes must have the same length");
	}

	const normalizedConfig = normalizeExtractionConfig(config);
	const rawResults = await getBinding().batchExtractBytes(buffers, mimeTypes, normalizedConfig);
	return rawResults.map(convertResult);
}

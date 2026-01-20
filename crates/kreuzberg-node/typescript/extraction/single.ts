/**
 * Single-document extraction APIs.
 *
 * This module provides synchronous and asynchronous functions for extracting content
 * from a single file or byte array. These are convenience wrappers around the native
 * binding that handle config normalization and result conversion.
 *
 * **Usage Note**: For processing multiple files, prefer batch extraction functions
 * (`batchExtractFiles`, `batchExtractFilesSync`) which provide better performance
 * and memory management.
 *
 * @internal This module is part of Layer 2 (extraction APIs).
 */

import { readFileSync } from "node:fs";
import { assertUint8Array } from "../core/assertions.js";
import { getBinding } from "../core/binding.js";
import { normalizeExtractionConfig } from "../core/config-normalizer.js";
import { convertResult } from "../core/type-converters.js";
import type { ExtractionConfig, ExtractionResult } from "../types.js";

/**
 * Extract content from a single file (synchronous).
 *
 * **Usage Note**: For processing multiple files, prefer `batchExtractFilesSync()` which
 * provides better performance and memory management.
 *
 * @param filePath - Path to the file to extract (string). Can be absolute or relative.
 * @param mimeTypeOrConfig - Optional MIME type hint or extraction configuration.
 *   If a string, treated as MIME type. If an object, treated as ExtractionConfig.
 *   If null, MIME type is auto-detected from file extension or content.
 * @param maybeConfig - Extraction configuration object. If null, uses default extraction settings.
 *   Only used if second parameter is a MIME type string.
 * @returns ExtractionResult containing extracted content, metadata, tables, and optional chunks/images
 * @throws {Error} If file doesn't exist, cannot be accessed, or cannot be read
 * @throws {ParsingError} When document format is invalid or corrupted
 * @throws {OcrError} When OCR processing fails (if OCR is enabled)
 * @throws {ValidationError} When extraction result fails validation (if validators registered)
 * @throws {KreuzbergError} For other extraction-related failures
 *
 * @example
 * ```typescript
 * import { extractFileSync } from '@kreuzberg/node';
 *
 * // Basic usage
 * const result = extractFileSync('document.pdf');
 * console.log(result.content);
 *
 * // With explicit MIME type
 * const result2 = extractFileSync('document.pdf', 'application/pdf');
 *
 * // With configuration
 * const result3 = extractFileSync('document.pdf', {
 *   chunking: {
 *     maxChars: 1000,
 *     maxOverlap: 200,
 *   },
 * });
 * ```
 */
export function extractFileSync(
	filePath: string,
	mimeTypeOrConfig?: string | null | ExtractionConfig,
	maybeConfig?: ExtractionConfig | null,
): ExtractionResult {
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
	const rawResult = getBinding().extractFileSync(filePath, mimeType, normalizedConfig);
	return convertResult(rawResult);
}

/**
 * Extract content from a single file (asynchronous).
 *
 * **Usage Note**: For processing multiple files, prefer `batchExtractFiles()` which
 * provides better performance and memory management.
 *
 * @param filePath - Path to the file to extract (string). Can be absolute or relative.
 * @param mimeTypeOrConfig - Optional MIME type hint or extraction configuration.
 *   If a string, treated as MIME type. If an object, treated as ExtractionConfig.
 *   If null, MIME type is auto-detected from file extension or content.
 * @param maybeConfig - Extraction configuration object. If null, uses default extraction settings.
 *   Only used if second parameter is a MIME type string.
 * @returns Promise<ExtractionResult> containing extracted content, metadata, tables, and optional chunks/images
 * @throws {Error} If file doesn't exist, cannot be accessed, or cannot be read
 * @throws {ParsingError} When document format is invalid or corrupted
 * @throws {OcrError} When OCR processing fails (if OCR is enabled)
 * @throws {ValidationError} When extraction result fails validation (if validators registered)
 * @throws {KreuzbergError} For other extraction-related failures
 *
 * @example
 * ```typescript
 * import { extractFile } from '@kreuzberg/node';
 *
 * // Basic usage
 * const result = await extractFile('document.pdf');
 * console.log(result.content);
 *
 * // With chunking enabled
 * const config = {
 *   chunking: {
 *     maxChars: 1000,
 *     maxOverlap: 200,
 *   },
 * };
 * const result2 = await extractFile('long_document.pdf', null, config);
 * console.log(result2.chunks); // Array of text chunks
 * ```
 */
export async function extractFile(
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
	const rawResult = await getBinding().extractFile(filePath, mimeType, normalizedConfig);
	return convertResult(rawResult);
}

/**
 * Extract content from raw bytes (synchronous).
 *
 * **Usage Note**: For processing multiple byte arrays, prefer `batchExtractBytesSync()`
 * which provides better performance and memory management.
 *
 * @param data - File content as Uint8Array (Buffer will be converted)
 * @param mimeType - MIME type of the data (required for accurate format detection). Must be a valid MIME type string.
 * @param config - Extraction configuration object. If null, uses default extraction settings.
 * @returns ExtractionResult containing extracted content, metadata, tables, and optional chunks/images
 * @throws {TypeError} When data is not a valid Uint8Array
 * @throws {Error} When file cannot be read or parsed
 * @throws {ParsingError} When document format is invalid or corrupted
 * @throws {OcrError} When OCR processing fails (if OCR is enabled)
 * @throws {ValidationError} When extraction result fails validation (if validators registered)
 * @throws {KreuzbergError} For other extraction-related failures
 *
 * @example
 * ```typescript
 * import { extractBytesSync } from '@kreuzberg/node';
 * import { readFileSync } from 'fs';
 *
 * const data = readFileSync('document.pdf');
 * const result = extractBytesSync(data, 'application/pdf');
 * console.log(result.content);
 * ```
 */
export function extractBytesSync(
	dataOrPath: Uint8Array | string,
	mimeType: string,
	config: ExtractionConfig | null = null,
): ExtractionResult {
	let data: Uint8Array;
	if (typeof dataOrPath === "string") {
		data = readFileSync(dataOrPath);
	} else {
		data = dataOrPath;
	}

	const validated = assertUint8Array(data, "data");
	const normalizedConfig = normalizeExtractionConfig(config);
	const rawResult = getBinding().extractBytesSync(Buffer.from(validated), mimeType, normalizedConfig);
	return convertResult(rawResult);
}

/**
 * Extract content from raw bytes (asynchronous).
 *
 * **Usage Note**: For processing multiple byte arrays, prefer `batchExtractBytes()`
 * which provides better performance and memory management.
 *
 * @param data - File content as Uint8Array (Buffer will be converted)
 * @param mimeType - MIME type of the data (required for accurate format detection). Must be a valid MIME type string.
 * @param config - Extraction configuration object. If null, uses default extraction settings.
 * @returns Promise<ExtractionResult> containing extracted content, metadata, tables, and optional chunks/images
 * @throws {TypeError} When data is not a valid Uint8Array
 * @throws {Error} When file cannot be read or parsed
 * @throws {ParsingError} When document format is invalid or corrupted
 * @throws {OcrError} When OCR processing fails (if OCR is enabled)
 * @throws {ValidationError} When extraction result fails validation (if validators registered)
 * @throws {KreuzbergError} For other extraction-related failures
 *
 * @example
 * ```typescript
 * import { extractBytes } from '@kreuzberg/node';
 * import { readFile } from 'fs/promises';
 *
 * const data = await readFile('document.pdf');
 * const result = await extractBytes(data, 'application/pdf');
 * console.log(result.content);
 * ```
 */
export async function extractBytes(
	dataOrPath: Uint8Array | string,
	mimeType: string,
	config: ExtractionConfig | null = null,
): Promise<ExtractionResult> {
	let data: Uint8Array;
	if (typeof dataOrPath === "string") {
		data = readFileSync(dataOrPath);
	} else {
		data = dataOrPath;
	}

	const validated = assertUint8Array(data, "data");
	// biome-ignore lint/complexity/useLiteralKeys: required for environment variable access
	if (process.env["KREUZBERG_DEBUG_GUTEN"] === "1") {
		console.log("[TypeScript] Debug input header:", Array.from(validated.slice(0, 8)));
	}
	const normalizedConfig = normalizeExtractionConfig(config);
	const rawResult = await getBinding().extractBytes(Buffer.from(validated), mimeType, normalizedConfig);
	return convertResult(rawResult);
}

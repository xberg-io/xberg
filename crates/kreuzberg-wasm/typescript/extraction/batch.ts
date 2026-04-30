/**
 * Batch extraction functions
 *
 * Provides batch processing capabilities for extracting from multiple documents
 * in a single operation for improved efficiency.
 */

import { configToJS, fileToUint8Array, jsToExtractionResult, wrapWasmError } from "../adapters/wasm-adapter.js";
import type { ExtractionConfig as ExtractionConfigType, ExtractionResult } from "../types.js";
import { getWasmModule, isInitialized } from "./internal.js";

/**
 * Batch extract content from multiple byte arrays asynchronously
 *
 * Extracts content from multiple documents in a single batch operation,
 * allowing for more efficient processing of multiple files.
 *
 * @param files - Array of objects containing data (Uint8Array) and mimeType (string)
 * @param config - Optional extraction configuration applied to all files
 * @returns Promise resolving to array of extraction results
 * @throws {Error} If WASM module is not initialized or extraction fails
 *
 * @example
 * ```typescript
 * const files = [
 *   { data: pdfBytes, mimeType: 'application/pdf' },
 *   { data: docxBytes, mimeType: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document' }
 * ];
 * const results = await batchExtractBytes(files);
 * results.forEach((result) => console.log(result.content));
 * ```
 */
export async function batchExtractBytes(
	files: Array<{ data: Uint8Array; mimeType: string }>,
	config?: ExtractionConfigType | null,
): Promise<ExtractionResult[]> {
	if (!isInitialized()) {
		throw new Error("WASM module not initialized. Call initWasm() first.");
	}

	const wasm = getWasmModule();

	try {
		if (!Array.isArray(files)) {
			throw new Error("Files parameter must be an array");
		}

		if (files.length === 0) {
			throw new Error("Files array cannot be empty");
		}

		const dataList: Uint8Array[] = [];
		const mimeTypes: string[] = [];

		for (let i = 0; i < files.length; i += 1) {
			const file = files[i];
			if (!file || typeof file !== "object") {
				throw new Error(`Invalid file at index ${i}: must be an object with data and mimeType`);
			}

			const f = file as Record<string, unknown>;

			if (!(f.data instanceof Uint8Array)) {
				throw new Error(`Invalid file at index ${i}: data must be Uint8Array`);
			}

			if (typeof f.mimeType !== "string") {
				throw new Error(`Invalid file at index ${i}: mimeType must be a string`);
			}

			if (f.data.length === 0) {
				throw new Error(`Invalid file at index ${i}: data cannot be empty`);
			}

			dataList.push(f.data);
			mimeTypes.push(f.mimeType);
		}

		const normalizedConfig = configToJS(config ?? null);

		const results = await wasm.batchExtractBytes(dataList, mimeTypes, normalizedConfig);

		if (!Array.isArray(results)) {
			throw new Error("Invalid batch extraction result: expected array");
		}

		return results.map((result, index) => {
			if (!result) {
				throw new Error(`Invalid extraction result at index ${index}: no result from WASM module`);
			}

			return jsToExtractionResult(result);
		});
	} catch (error) {
		throw wrapWasmError(error, "batch extracting from bytes");
	}
}

/**
 * Batch extract content from multiple byte arrays synchronously
 *
 * Synchronous version of {@link batchExtractBytes}. Extracts content from multiple
 * documents in a single batch operation without async/await.
 *
 * **Note:** This function blocks the current thread until all extractions complete.
 * For large batches, prefer the async {@link batchExtractBytes} function.
 *
 * @param files - Array of objects containing data (Uint8Array) and mimeType (string)
 * @param config - Optional extraction configuration applied to all files
 * @returns Array of extraction results
 * @throws {Error} If WASM module is not initialized or extraction fails
 *
 * @example
 * ```typescript
 * const files = [
 *   { data: txtBytes, mimeType: 'text/plain' },
 *   { data: htmlBytes, mimeType: 'text/html' }
 * ];
 * const results = batchExtractBytesSync(files);
 * results.forEach((result) => console.log(result.content));
 * ```
 */
export function batchExtractBytesSync(
	files: Array<{ data: Uint8Array; mimeType: string }>,
	config?: ExtractionConfigType | null,
): ExtractionResult[] {
	if (!isInitialized()) {
		throw new Error("WASM module not initialized. Call initWasm() first.");
	}

	const wasm = getWasmModule();

	try {
		if (!Array.isArray(files)) {
			throw new Error("Files parameter must be an array");
		}

		if (files.length === 0) {
			throw new Error("Files array cannot be empty");
		}

		const dataList: Uint8Array[] = [];
		const mimeTypes: string[] = [];

		for (let i = 0; i < files.length; i += 1) {
			const file = files[i];
			if (!file || typeof file !== "object") {
				throw new Error(`Invalid file at index ${i}: must be an object with data and mimeType`);
			}

			const f = file as Record<string, unknown>;

			if (!(f.data instanceof Uint8Array)) {
				throw new Error(`Invalid file at index ${i}: data must be Uint8Array`);
			}

			if (typeof f.mimeType !== "string") {
				throw new Error(`Invalid file at index ${i}: mimeType must be a string`);
			}

			if (f.data.length === 0) {
				throw new Error(`Invalid file at index ${i}: data cannot be empty`);
			}

			dataList.push(f.data);
			mimeTypes.push(f.mimeType);
		}

		const normalizedConfig = configToJS(config ?? null);

		const results = wasm.batchExtractBytesSync(dataList, mimeTypes, normalizedConfig);

		if (!Array.isArray(results)) {
			throw new Error("Invalid batch extraction result: expected array");
		}

		return results.map((result, index) => {
			if (!result) {
				throw new Error(`Invalid extraction result at index ${index}: no result from WASM module`);
			}

			return jsToExtractionResult(result);
		});
	} catch (error) {
		throw wrapWasmError(error, "batch extracting from bytes (sync)");
	}
}

/**
 * Batch extract content from multiple File objects asynchronously
 *
 * Convenience function that converts File objects to Uint8Array and calls batchExtractBytes.
 * Automatically uses the file.type as MIME type if available.
 *
 * @param files - Array of File objects to extract from
 * @param config - Optional extraction configuration applied to all files
 * @returns Promise resolving to array of extraction results
 * @throws {Error} If WASM module is not initialized, files cannot be read, or extraction fails
 *
 * @example
 * ```typescript
 * const fileInput = document.getElementById('files');
 * const files = Array.from(fileInput.files ?? []);
 * const results = await batchExtractFiles(files);
 * results.forEach((result, index) => {
 *   console.log(`File ${index}: ${result.content.substring(0, 50)}...`);
 * });
 * ```
 */
export async function batchExtractFiles(
	files: File[],
	config?: ExtractionConfigType | null,
): Promise<ExtractionResult[]> {
	if (!isInitialized()) {
		throw new Error("WASM module not initialized. Call initWasm() first.");
	}

	try {
		if (!Array.isArray(files)) {
			throw new Error("Files parameter must be an array");
		}

		if (files.length === 0) {
			throw new Error("Files array cannot be empty");
		}

		const byteFiles: Array<{ data: Uint8Array; mimeType: string }> = [];

		for (let i = 0; i < files.length; i += 1) {
			const file = files[i];
			if (!(file instanceof File)) {
				throw new Error(`Invalid file at index ${i}: must be a File object`);
			}

			const bytes = await fileToUint8Array(file);
			byteFiles.push({
				data: bytes,
				mimeType: file.type || "application/octet-stream",
			});
		}

		return await batchExtractBytes(byteFiles, config);
	} catch (error) {
		throw wrapWasmError(error, "batch extracting from files");
	}
}

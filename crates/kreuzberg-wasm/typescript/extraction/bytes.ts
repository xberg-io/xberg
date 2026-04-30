/**
 * Byte-based extraction functions
 *
 * Provides synchronous and asynchronous extraction functions for document bytes.
 */

import { configToJS, jsToExtractionResult, wrapWasmError } from "../adapters/wasm-adapter.js";
import type { ExtractionConfig as ExtractionConfigType, ExtractionResult } from "../types.js";
import { getWasmModule, isInitialized } from "./internal.js";

/**
 * Extract content from bytes (document data)
 *
 * Extracts text, metadata, tables, images, and other content from document bytes.
 * Automatically detects document type from MIME type and applies appropriate extraction logic.
 *
 * @param data - The document bytes to extract from
 * @param mimeType - MIME type of the document (e.g., 'application/pdf', 'image/jpeg')
 * @param config - Optional extraction configuration
 * @returns Promise resolving to the extraction result
 * @throws {Error} If WASM module is not initialized or extraction fails
 *
 * @example Extract PDF
 * ```typescript
 * const bytes = new Uint8Array(buffer);
 * const result = await extractBytes(bytes, 'application/pdf');
 * console.log(result.content);
 * console.log(result.tables);
 * ```
 *
 * @example Extract with Configuration
 * ```typescript
 * const result = await extractBytes(bytes, 'application/pdf', {
 *   ocr: {
 *     backend: 'tesseract',
 *     language: 'deu' // German
 *   },
 *   images: {
 *     extractImages: true,
 *     targetDpi: 200
 *   }
 * });
 * ```
 *
 * @example Extract from File
 * ```typescript
 * const file = inputEvent.target.files[0];
 * const bytes = await fileToUint8Array(file);
 * const result = await extractBytes(bytes, file.type);
 * ```
 */
export async function extractBytes(
	data: Uint8Array,
	mimeType: string,
	config?: ExtractionConfigType | null,
): Promise<ExtractionResult> {
	if (!isInitialized()) {
		throw new Error("WASM module not initialized. Call initWasm() first.");
	}

	const wasm = getWasmModule();

	try {
		if (!data || data.length === 0) {
			throw new Error("Document data cannot be empty");
		}

		if (!mimeType) {
			throw new Error("MIME type is required");
		}

		const normalizedConfig = configToJS(config ?? null);

		const result = await wasm.extractBytes(data, mimeType, normalizedConfig);

		if (!result) {
			throw new Error("Invalid extraction result: no result from WASM module");
		}

		return jsToExtractionResult(result);
	} catch (error) {
		throw wrapWasmError(error, "extracting from bytes");
	}
}

/**
 * Extract content from bytes synchronously
 *
 * Synchronous version of {@link extractBytes}. Extracts text, metadata, tables,
 * and other content from document bytes without async/await.
 *
 * **Note:** This function blocks the current thread until extraction completes.
 * For large documents, prefer the async {@link extractBytes} function.
 *
 * @param data - The document bytes to extract from
 * @param mimeType - MIME type of the document (e.g., 'application/pdf', 'image/jpeg')
 * @param config - Optional extraction configuration
 * @returns The extraction result
 * @throws {Error} If WASM module is not initialized or extraction fails
 *
 * @example
 * ```typescript
 * const bytes = new Uint8Array(buffer);
 * const result = extractBytesSync(bytes, 'text/plain');
 * console.log(result.content);
 * ```
 */
export function extractBytesSync(
	data: Uint8Array,
	mimeType: string,
	config?: ExtractionConfigType | null,
): ExtractionResult {
	if (!isInitialized()) {
		throw new Error("WASM module not initialized. Call initWasm() first.");
	}

	const wasm = getWasmModule();

	try {
		if (!data || data.length === 0) {
			throw new Error("Document data cannot be empty");
		}

		if (!mimeType) {
			throw new Error("MIME type is required");
		}

		const normalizedConfig = configToJS(config ?? null);

		const result = wasm.extractBytesSync(data, mimeType, normalizedConfig);

		if (!result) {
			throw new Error("Invalid extraction result: no result from WASM module");
		}

		return jsToExtractionResult(result);
	} catch (error) {
		throw wrapWasmError(error, "extracting from bytes (sync)");
	}
}

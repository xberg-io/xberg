/**
 * MIME type utilities
 *
 * Provides functions for MIME type detection and extension lookup
 * using the WASM module's native capabilities.
 */

import { wrapWasmError } from "../adapters/wasm-adapter.js";
import { getWasmModule, isInitialized } from "../extraction/internal.js";

/**
 * Detect MIME type from raw bytes
 *
 * Uses magic-byte detection to determine the MIME type of a byte buffer.
 *
 * @param data - The raw bytes to detect MIME type from
 * @returns The detected MIME type string (e.g., 'application/pdf', 'image/png')
 * @throws {Error} If WASM module is not initialized or detection fails
 *
 * @example
 * ```typescript
 * const bytes = new Uint8Array(buffer);
 * const mimeType = detectMimeFromBytes(bytes);
 * console.log(mimeType); // 'application/pdf'
 * ```
 */
export function detectMimeFromBytes(data: Uint8Array): string {
	if (!isInitialized()) {
		throw new Error("WASM module not initialized. Call initWasm() first.");
	}

	const wasm = getWasmModule();

	try {
		return wasm.detectMimeFromBytes(data);
	} catch (error) {
		throw wrapWasmError(error, "detecting MIME type from bytes");
	}
}

/**
 * Get file extensions for a MIME type
 *
 * Returns known file extensions associated with the given MIME type.
 *
 * @param mimeType - The MIME type to look up extensions for
 * @returns Array of file extension strings (e.g., ['pdf'], ['jpg', 'jpeg'])
 * @throws {Error} If WASM module is not initialized or lookup fails
 *
 * @example
 * ```typescript
 * const extensions = getExtensionsForMime('application/pdf');
 * console.log(extensions); // ['pdf']
 * ```
 */
export function getExtensionsForMime(mimeType: string): string[] {
	if (!isInitialized()) {
		throw new Error("WASM module not initialized. Call initWasm() first.");
	}

	const wasm = getWasmModule();

	try {
		return wasm.getExtensionsForMime(mimeType);
	} catch (error) {
		throw wrapWasmError(error, "getting extensions for MIME type");
	}
}

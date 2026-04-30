/**
 * File-based extraction functions
 *
 * Provides extraction functions for files in filesystem-based environments (Node.js, Deno, Bun)
 * and browser File/Blob objects.
 */

import { fileToUint8Array, wrapWasmError } from "../adapters/wasm-adapter.js";
import { detectRuntime } from "../runtime.js";
import type { ExtractionConfig as ExtractionConfigType, ExtractionResult } from "../types.js";
import { extractBytes } from "./bytes.js";
import { getWasmModule, isInitialized } from "./internal.js";

/**
 * Extract content from a file on the file system
 *
 * Node.js and Deno specific function that reads a file from the file system
 * and extracts content from it. Automatically detects MIME type if not provided.
 *
 * @param path - Path to the file to extract from
 * @param mimeType - Optional MIME type of the file. If not provided, will attempt to detect
 * @param config - Optional extraction configuration
 * @returns Promise resolving to the extraction result
 * @throws {Error} If WASM module is not initialized, file doesn't exist, or extraction fails
 *
 * @example Extract with auto-detection
 * ```typescript
 * const result = await extractFile('./document.pdf');
 * console.log(result.content);
 * ```
 *
 * @example Extract with explicit MIME type
 * ```typescript
 * const result = await extractFile('./document.docx', 'application/vnd.openxmlformats-officedocument.wordprocessingml.document');
 * ```
 *
 * @example Extract from Node.js with config
 * ```typescript
 * import { extractFile } from '@kreuzberg/wasm';
 * import { readFile } from 'fs/promises';
 *
 * const result = await extractFile('./report.xlsx', null, {
 *   chunking: {
 *     maxChars: 1000
 *   }
 * });
 * ```
 */
export async function extractFile(
	path: string,
	mimeType?: string | null,
	config?: ExtractionConfigType | null,
): Promise<ExtractionResult> {
	if (!isInitialized()) {
		throw new Error("WASM module not initialized. Call initWasm() first.");
	}

	const wasm = getWasmModule();

	try {
		if (!path) {
			throw new Error("File path is required");
		}

		const runtime = detectRuntime();
		if (runtime === "browser") {
			throw new Error("Use extractBytes with fileToUint8Array for browser environments");
		}

		let fileData: Uint8Array;

		if (runtime === "node") {
			const { readFile } = await import("node:fs/promises");
			const buffer = await readFile(path);
			fileData = new Uint8Array(buffer);
		} else if (runtime === "deno") {
			const deno = (globalThis as Record<string, unknown>).Deno as {
				readFile: (path: string) => Promise<Uint8Array>;
			};
			fileData = await deno.readFile(path);
		} else if (runtime === "bun") {
			const { readFile } = await import("node:fs/promises");
			const buffer = await readFile(path);
			fileData = new Uint8Array(buffer);
		} else {
			throw new Error(`Unsupported runtime for file extraction: ${runtime}`);
		}

		let detectedMimeType = mimeType;
		if (!detectedMimeType) {
			detectedMimeType = wasm.detectMimeFromBytes(fileData);
		}

		if (!detectedMimeType) {
			throw new Error("Could not detect MIME type for file. Please provide mimeType parameter.");
		}

		detectedMimeType = wasm.normalizeMimeType(detectedMimeType);

		return await extractBytes(fileData, detectedMimeType, config);
	} catch (error) {
		throw wrapWasmError(error, `extracting from file: ${path}`);
	}
}

/**
 * Extract content from a File or Blob (browser-friendly wrapper)
 *
 * Convenience function that wraps fileToUint8Array and extractBytes,
 * providing a streamlined API for browser applications handling file inputs.
 *
 * @param file - The File or Blob to extract from
 * @param mimeType - Optional MIME type. If not provided, uses file.type if available
 * @param config - Optional extraction configuration
 * @returns Promise resolving to the extraction result
 * @throws {Error} If WASM module is not initialized or extraction fails
 *
 * @example Simple file extraction
 * ```typescript
 * const fileInput = document.getElementById('file');
 * fileInput.addEventListener('change', async (e) => {
 *   const file = e.target.files?.[0];
 *   if (file) {
 *     const result = await extractFromFile(file);
 *     console.log(result.content);
 *   }
 * });
 * ```
 *
 * @example With configuration
 * ```typescript
 * const result = await extractFromFile(file, file.type, {
 *   chunking: { maxChars: 1000 },
 *   images: { extractImages: true }
 * });
 * ```
 */
export async function extractFromFile(
	file: File | Blob,
	mimeType?: string | null,
	config?: ExtractionConfigType | null,
): Promise<ExtractionResult> {
	if (!isInitialized()) {
		throw new Error("WASM module not initialized. Call initWasm() first.");
	}

	const wasm = getWasmModule();

	try {
		const bytes = await fileToUint8Array(file);
		let type = mimeType ?? (file instanceof File ? file.type : "application/octet-stream");

		type = wasm.normalizeMimeType(type);

		return await extractBytes(bytes, type, config);
	} catch (error) {
		throw wrapWasmError(error, `extracting from ${file instanceof File ? "file" : "blob"}`);
	}
}

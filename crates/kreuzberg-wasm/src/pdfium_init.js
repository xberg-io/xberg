/**
 * PDFium WASM Initialization Helper
 *
 * This module provides a helper function to initialize PDFium for use with kreuzberg-wasm.
 *
 * IMPORTANT: PDFium must be initialized before any PDF extraction operations.
 *
 * @module pdfium-init
 */

/**
 * Initialize PDFium WASM module for use with kreuzberg.
 *
 * This function MUST be called after loading both the kreuzberg WASM module
 * and the PDFium WASM module, but before performing any PDF operations.
 *
 * @param {Object} pdfiumModule - The loaded PDFium WASM module (from pdfium.js)
 * @param {Object} wasmModule - The kreuzberg WASM instance
 * @param {boolean} [debug=false] - Enable debug logging
 * @returns {boolean} true if initialization succeeded, false otherwise
 *
 * @example
 * import init from './kreuzberg_wasm.js';
 * import pdfiumModule from './pdfium.js';
 * import { initializePdfiumWasm } from './pdfium_init.js';
 *
 * // Load kreuzberg WASM
 * const wasm = await init();
 *
 * // Load PDFium WASM
 * const pdfium = await pdfiumModule();
 *
 * // Initialize PDFium (required before PDF operations)
 * if (!initializePdfiumWasm(pdfium, wasm, false)) {
 *     throw new Error('Failed to initialize PDFium');
 * }
 *
 * // Now you can use PDF extraction
 * const result = await wasm.extract_from_bytes(pdfBytes, config);
 */
export function initializePdfiumWasm(pdfiumModule, wasmModule, debug = false) {
	if (typeof wasmModule.initialize_pdfium_render === "function") {
		try {
			return wasmModule.initialize_pdfium_render(pdfiumModule, wasmModule, debug);
		} catch (error) {
			console.error("Failed to initialize PDFium:", error);
			return false;
		}
	}

	if (typeof initialize_pdfium_render === "function") {
		try {
			return initialize_pdfium_render(pdfiumModule, wasmModule, debug);
		} catch (error) {
			console.error("Failed to initialize PDFium (global):", error);
			return false;
		}
	}

	console.error("initialize_pdfium_render function not found. This may indicate a build issue.");
	console.error("PDFium initialization requires pdfium-render WASM bindings to be present.");
	return false;
}

/**
 * Load PDFium WASM module from a URL.
 *
 * @param {string} pdfiumJsUrl - URL to pdfium.js file
 * @returns {Promise<Object>} Loaded PDFium module
 */
export async function loadPdfiumModule(pdfiumJsUrl) {
	const pdfiumLoader = await import(pdfiumJsUrl);

	const pdfiumModule = await pdfiumLoader.default();

	return pdfiumModule;
}

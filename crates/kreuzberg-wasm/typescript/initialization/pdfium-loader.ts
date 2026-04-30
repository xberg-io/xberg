/**
 * PDFium WASM Loader
 *
 * Handles PDFium-specific WASM module loading and initialization.
 * Provides asynchronous loading of the PDFium WASM module with
 * proper error handling across all WASM runtimes (browser, Node.js, Bun, Deno).
 *
 * PDFium can be provided in several ways:
 * 1. Place `pdfium.js` (Emscripten output) alongside the package distribution
 * 2. Set `KREUZBERG_PDFIUM_PATH` environment variable to the directory containing `pdfium.js`
 * 3. Call `initialize_pdfium_render()` manually with a loaded PDFium module
 */

import { isNode } from "../runtime.js";
import type { WasmModule } from "./state.js";

/**
 * Attempt to load the PDFium module from the filesystem in Node.js.
 * Checks multiple candidate paths relative to the package directory.
 *
 * @returns The loaded PDFium module or null if not found
 * @internal
 */
async function loadPdfiumForNode(): Promise<Record<string, unknown> | null> {
	try {
		const fs = await import(/* @vite-ignore */ "node:fs/promises");
		const path = await import(/* @vite-ignore */ "node:path");
		const url = await import(/* @vite-ignore */ "node:url");

		const __dirname = path.dirname(url.fileURLToPath(import.meta.url));

		// Check environment variable first
		const envPath = process.env.KREUZBERG_PDFIUM_PATH;
		const candidates: string[] = [];

		if (envPath) {
			candidates.push(path.join(envPath, "pdfium.js"));
			candidates.push(envPath); // allow direct path to pdfium.js
		}

		// Standard locations relative to package dist
		candidates.push(
			path.join(__dirname, "..", "pdfium.js"), // dist/pdfium.js
			path.join(__dirname, "pdfium.js"), // dist/initialization/pdfium.js
			path.join(__dirname, "..", "..", "pdfium.js"), // package root pdfium.js
		);

		for (const candidate of candidates) {
			try {
				await fs.access(candidate);
				const moduleUrl = url.pathToFileURL(candidate).href;
				return (await import(/* @vite-ignore */ moduleUrl)) as Record<string, unknown>;
			} catch {
				// Try next candidate path
			}
		}

		return null;
	} catch {
		return null;
	}
}

/**
 * Load the PDFium module for the current runtime environment.
 *
 * @returns The loaded PDFium module or null if not available
 * @internal
 */
async function loadPdfiumModule(): Promise<Record<string, unknown> | null> {
	if (isNode()) {
		return loadPdfiumForNode();
	}

	// Browser/Deno/Bun: try dynamic import
	try {
		// @ts-expect-error - Dynamic module loading
		return await import("../pdfium.js");
	} catch {
		return null;
	}
}

/**
 * Initialize PDFium WASM module asynchronously
 *
 * Loads and binds the PDFium WASM module for PDF extraction.
 * This function is called automatically during WASM initialization
 * in all supported environments (browser, Node.js, Bun, Deno).
 *
 * PDFium provides high-performance PDF parsing and extraction capabilities,
 * enabling reliable text and metadata extraction from PDF documents.
 *
 * If the PDFium module cannot be found or loaded, initialization fails
 * gracefully and PDF extraction will not be available. Users can provide
 * the PDFium module manually via `initialize_pdfium_render()`.
 *
 * @param wasmModule - The loaded Kreuzberg WASM module
 *
 * @internal
 *
 * @example
 * ```typescript
 * // Called automatically during initWasm() in all environments
 * // See wasm-loader.ts for integration
 *
 * // To provide PDFium manually in Node.js:
 * // Set KREUZBERG_PDFIUM_PATH=/path/to/pdfium-wasm-dir
 *
 * // Or initialize manually:
 * import { initWasm, getWasmModule } from '@kreuzberg/wasm';
 * await initWasm();
 * const wasm = getWasmModule();
 * const pdfium = await import('./pdfium.js').then(m => m.default());
 * wasm.initialize_pdfium_render(pdfium, wasm, false);
 * ```
 */
export async function initializePdfiumAsync(wasmModule: WasmModule): Promise<void> {
	if (!wasmModule || typeof wasmModule.initialize_pdfium_render !== "function") {
		return;
	}

	try {
		const pdfiumModule = await loadPdfiumModule();
		if (!pdfiumModule) {
			console.debug("PDFium module not found, PDF extraction will not be available");
			console.debug("To enable PDF support, provide pdfium.js via KREUZBERG_PDFIUM_PATH or manual initialization");
			return;
		}

		const pdfium =
			typeof pdfiumModule.default === "function"
				? await (pdfiumModule.default as () => Promise<unknown>)()
				: pdfiumModule;

		const success = wasmModule.initialize_pdfium_render(pdfium, wasmModule, false);
		if (!success) {
			console.warn("PDFium initialization returned false");
		}
	} catch (error) {
		console.debug("PDFium initialization error:", error);
	}
}

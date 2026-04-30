/**
 * OCR enabler module
 *
 * Provides convenient functions for enabling and setting up OCR backends.
 * Automatically selects the appropriate backend based on build capabilities:
 * - Native WASM OCR (kreuzberg-tesseract compiled to WASM, works everywhere)
 * - Browser fallback: TesseractWasmBackend (using tesseract-wasm npm package + createImageBitmap)
 */

import { isInitialized } from "../extraction/internal.js";
import { getWasmModule } from "../initialization/state.js";
import { registerOcrBackend } from "../ocr/registry.js";
import { TesseractWasmBackend } from "../ocr/tesseract-wasm-backend.js";
import { createOcrWorker, runOcrInWorker, terminateOcrWorker } from "../ocr/worker-bridge.js";
import { isBrowser, isNode } from "../runtime.js";
import type { OcrBackendProtocol } from "../types.js";

/** Default CDN URL for tessdata files (Tesseract fast models) */
const TESSDATA_CDN_BASE = "https://raw.githubusercontent.com/tesseract-ocr/tessdata_fast/main";

/**
 * Native WASM OCR backend using kreuzberg-tesseract compiled into the WASM binary.
 *
 * This backend works in all environments (Browser, Node.js, Deno, etc.)
 * because Tesseract is statically linked into the WASM module.
 * Tessdata is downloaded from CDN and passed to Tesseract via memory (no filesystem needed).
 */
class NativeWasmOcrBackend implements OcrBackendProtocol {
	private tessdataCache: Map<string, Uint8Array> = new Map();
	private tessdataCdnBase: string = TESSDATA_CDN_BASE;
	private progressCallback: ((progress: number) => void) | null = null;

	name(): string {
		return "kreuzberg-tesseract";
	}

	supportedLanguages(): string[] {
		return [
			"eng",
			"deu",
			"fra",
			"spa",
			"ita",
			"por",
			"nld",
			"rus",
			"jpn",
			"kor",
			"chi_sim",
			"chi_tra",
			"pol",
			"tur",
			"swe",
			"dan",
			"fin",
			"nor",
			"ces",
			"slk",
			"ron",
			"hun",
			"hrv",
			"srp",
			"bul",
			"ukr",
			"ell",
			"ara",
			"heb",
			"hin",
			"tha",
			"vie",
			"mkd",
			"ben",
			"tam",
			"tel",
			"kan",
			"mal",
			"mya",
			"khm",
			"lao",
			"sin",
		];
	}

	async initialize(): Promise<void> {
		const wasm = getWasmModule();
		if (!wasm?.ocrIsAvailable?.()) {
			throw new Error(
				"Native WASM OCR is not available. Build with the 'ocr-wasm' feature to enable kreuzberg-tesseract.",
			);
		}

		// Resolve path to the WASM glue module for the worker thread
		let wasmGluePath: string;
		let wasmBinary: Uint8Array | undefined;

		if (isNode()) {
			const nodePath = await import(/* @vite-ignore */ "node:path");
			const nodeUrl = await import(/* @vite-ignore */ "node:url");
			const nodeFs = await import(/* @vite-ignore */ "node:fs/promises");
			const __dirname = nodePath.dirname(nodeUrl.fileURLToPath(import.meta.url));
			wasmGluePath = nodePath.join(__dirname, "..", "pkg", "kreuzberg_wasm.js");
			try {
				const wasmPath = nodePath.join(__dirname, "..", "pkg", "kreuzberg_wasm_bg.wasm");
				const buf = await nodeFs.readFile(wasmPath);
				wasmBinary = new Uint8Array(buf);
			} catch {
				// Binary will be loaded by glue code's default() fetch
			}
		} else {
			wasmGluePath = new URL("../pkg/kreuzberg_wasm.js", import.meta.url).href;
		}

		// Direct (blocking) fallback for when workers are unavailable
		const directFallback = (imageData: Uint8Array, tessdata: Uint8Array, language: string): string => {
			if (!wasm.ocrRecognize) throw new Error("ocrRecognize not available");
			return wasm.ocrRecognize(imageData, tessdata, language);
		};

		await createOcrWorker(wasmGluePath, wasmBinary, directFallback);
	}

	async shutdown(): Promise<void> {
		this.tessdataCache.clear();
		this.progressCallback = null;
		await terminateOcrWorker();
	}

	setProgressCallback(callback: (progress: number) => void): void {
		this.progressCallback = callback;
	}

	async processImage(
		imageBytes: Uint8Array | string,
		language: string,
	): Promise<{
		content: string;
		mime_type: string;
		metadata: Record<string, unknown>;
		tables: unknown[];
	}> {
		const normalizedLang = language.toLowerCase();

		this.reportProgress(10);

		// Download tessdata if not cached
		const tessdata = await this.getTessdata(normalizedLang);

		this.reportProgress(40);

		// Convert base64 string to Uint8Array if needed
		let imageData: Uint8Array;
		if (typeof imageBytes === "string") {
			const binaryString = atob(imageBytes);
			imageData = new Uint8Array(binaryString.length);
			for (let i = 0; i < binaryString.length; i++) {
				imageData[i] = binaryString.charCodeAt(i);
			}
		} else {
			imageData = imageBytes;
		}

		this.reportProgress(50);

		// Run OCR in a worker thread to avoid blocking the main event loop.
		// Falls back to direct (blocking) call if workers are unavailable.
		const text = await runOcrInWorker(imageData, tessdata, normalizedLang);

		this.reportProgress(90);

		return {
			content: text,
			mime_type: "text/plain",
			metadata: { language: normalizedLang },
			tables: [],
		};
	}

	private async getTessdata(language: string): Promise<Uint8Array> {
		const cached = this.tessdataCache.get(language);
		if (cached) {
			return cached;
		}

		const url = `${this.tessdataCdnBase}/${language}.traineddata`;
		const response = await fetch(url);
		if (!response.ok) {
			throw new Error(`Failed to download tessdata for "${language}" from ${url}: ${response.status}`);
		}

		const data = new Uint8Array(await response.arrayBuffer());
		this.tessdataCache.set(language, data);
		return data;
	}

	private reportProgress(progress: number): void {
		if (this.progressCallback) {
			try {
				this.progressCallback(Math.min(100, Math.max(0, progress)));
			} catch {
				// Ignore callback errors
			}
		}
	}
}

/**
 * Enable OCR functionality with the appropriate backend for the current runtime
 *
 * Automatically selects the best available OCR backend:
 * 1. **Native WASM OCR** (preferred): If built with `ocr-wasm` feature, uses kreuzberg-tesseract
 *    compiled directly into the WASM binary. Works in all environments (Browser, Node.js, Deno).
 * 2. **Browser fallback**: Uses `TesseractWasmBackend` with the `tesseract-wasm` npm package
 *    (requires `createImageBitmap` browser API).
 *
 * ## Network Requirement
 *
 * Training data will be loaded from jsDelivr CDN on first use of each language.
 * Ensure network access to cdn.jsdelivr.net is available.
 *
 * @throws {Error} If WASM is not initialized or no OCR backend is available
 *
 * @example Basic Usage (works in all environments)
 * ```typescript
 * import { enableOcr, extractBytes, initWasm } from '@kreuzberg/wasm';
 *
 * await initWasm();
 * await enableOcr();
 *
 * const imageBytes = new Uint8Array(buffer);
 * const result = await extractBytes(imageBytes, 'image/png', {
 *   ocr: { backend: 'kreuzberg-tesseract', language: 'eng' }
 * });
 *
 * console.log(result.content);
 * ```
 */
export async function enableOcr(): Promise<void> {
	if (!isInitialized()) {
		throw new Error("WASM module not initialized. Call initWasm() first.");
	}

	try {
		// Try native WASM OCR first (works in all environments)
		const wasm = getWasmModule();
		if (wasm?.ocrIsAvailable?.()) {
			const backend = new NativeWasmOcrBackend();
			await backend.initialize();
			registerOcrBackend(backend);
			registerBackendInRustRegistry(wasm, backend);
			return;
		}

		// Fallback: browser-only tesseract-wasm npm backend
		if (isBrowser()) {
			const backend = new TesseractWasmBackend();
			await backend.initialize();
			registerOcrBackend(backend);
			registerBackendInRustRegistry(wasm, backend);
			return;
		}

		throw new Error(
			"No OCR backend available. " +
				"Build with the 'ocr-wasm' feature to enable native Tesseract OCR in all environments, " +
				"or use a browser environment with the tesseract-wasm npm package.",
		);
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error);
		throw new Error(`Failed to enable OCR: ${message}`);
	}
}

/**
 * Register an OCR backend in the Rust-side plugin registry.
 *
 * The Rust extraction pipeline looks up OCR backends from its own registry
 * (not the JS-side Map). This creates a thin adapter that bridges the
 * OcrBackendProtocol interface to what the Rust wasm-bindgen bridge expects:
 * - name() returns "tesseract" (matching the default OcrConfig.backend value)
 * - processImage() returns a JSON string (not an object)
 */
function registerBackendInRustRegistry(wasm: ReturnType<typeof getWasmModule>, backend: OcrBackendProtocol): void {
	// wasm-bindgen exports the function as "register_ocr_backend" (snake_case).
	// Guard against it being absent so we can emit a clear warning instead of
	// silently leaving the Rust registry empty (which causes the "OCR backend
	// 'tesseract' not registered. Available backends: []" error at extraction time).
	const registerFn = wasm?.register_ocr_backend;
	if (!registerFn) {
		// Fail fast: if the Rust bridge is absent, enableOcr() must throw rather
		// than succeed silently. A silent return leaves the Rust registry empty,
		// causing the cryptic "OCR backend 'tesseract' not registered. Available
		// backends: []" error at extraction time — the exact bug in issue #719.
		throw new Error(
			"wasm.register_ocr_backend is not exported by the WASM module. " +
				"The Rust-side OCR plugin registry cannot be populated. " +
				"Ensure the WASM binary was built with the 'ocr-wasm' feature and the pkg glue is up to date.",
		);
	}

	const rustAdapter = {
		name: () => "tesseract",
		supportedLanguages: () => backend.supportedLanguages?.() ?? ["eng"],
		processImage: async (imageBase64: string, language: string): Promise<string> => {
			const result = await backend.processImage(imageBase64, language);
			return typeof result === "string" ? result : JSON.stringify(result);
		},
	};

	try {
		registerFn(rustAdapter);
	} catch (err) {
		// wasm-bindgen throws if a backend with the same name is already registered.
		// That is safe to ignore (idempotent re-registration). Re-throw anything else.
		const msg = err instanceof Error ? err.message : String(err);
		if (!msg.toLowerCase().includes("already registered")) {
			throw new Error(`Failed to register OCR backend in the Rust plugin registry: ${msg}`);
		}
	}
}

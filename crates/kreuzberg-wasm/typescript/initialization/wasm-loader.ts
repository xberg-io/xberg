/**
 * WASM Module Loader
 *
 * Handles WASM module loading, initialization, and state management.
 * Provides a clean interface for loading the Kreuzberg WASM module
 * with support for concurrent initialization calls.
 */

import { wrapWasmError } from "../adapters/wasm-adapter.js";
import { hasWasm, isDeno, isEdgeEnvironment, isNode } from "../runtime.js";
import { initializePdfiumAsync } from "./pdfium-loader.js";

/**
 * Options for initializing the WASM module.
 */
export interface InitWasmOptions {
	/**
	 * A pre-loaded WebAssembly.Module for the Kreuzberg WASM binary.
	 *
	 * Required in restricted edge environments (Cloudflare Workers, Vercel Edge,
	 * Supabase edge functions) where the runtime cannot fetch `file://` URLs.
	 * Import the `.wasm` file as a static import and pass it here.
	 *
	 * @example Cloudflare Workers
	 * ```typescript
	 * import wasmModule from '@kreuzberg/wasm/kreuzberg_wasm_bg.wasm';
	 * import { initWasm } from '@kreuzberg/wasm';
	 *
	 * export default {
	 *   async fetch(request: Request): Promise<Response> {
	 *     await initWasm({ wasmModule });
	 *     // ... use extraction functions
	 *   }
	 * };
	 * ```
	 *
	 * @example Supabase Edge Functions (Deno)
	 * ```typescript
	 * import wasmModule from '@kreuzberg/wasm/kreuzberg_wasm_bg.wasm';
	 * import { initWasm, extractBytes } from '@kreuzberg/wasm';
	 *
	 * Deno.serve(async (req: Request) => {
	 *   await initWasm({ wasmModule });
	 *   // ... use extraction functions
	 * });
	 * ```
	 */
	wasmModule?: WebAssembly.Module;
}

/**
 * Load WASM binary from file system in Node.js or Deno environment.
 * Returns undefined in browser/edge environments (fetch or explicit module will be used instead).
 */
async function loadWasmBinaryForNode(): Promise<Uint8Array | undefined> {
	if (isNode()) {
		try {
			// Dynamic import to avoid bundling Node.js modules
			const fs = await import(/* @vite-ignore */ "node:fs/promises");
			const path = await import(/* @vite-ignore */ "node:path");
			const url = await import(/* @vite-ignore */ "node:url");

			// Resolve the WASM file path relative to this module
			// The module is in dist/initialization/wasm-loader.js
			// The WASM file is in dist/pkg/kreuzberg_wasm_bg.wasm
			const __dirname = path.dirname(url.fileURLToPath(import.meta.url));
			const wasmPath = path.join(__dirname, "..", "pkg", "kreuzberg_wasm_bg.wasm");

			const wasmBuffer = await fs.readFile(wasmPath);
			return new Uint8Array(wasmBuffer);
		} catch {
			// Fall back to fetch-based loading if file system access fails
			return undefined;
		}
	}

	if (isDeno()) {
		try {
			// In Deno, use Deno.readFile to load the WASM binary from disk.
			// import.meta.url is a file:// URL pointing to the current module.
			// Construct the path to dist/pkg/kreuzberg_wasm_bg.wasm relative to this module.
			// This works in regular Deno but will throw in restricted environments
			// (e.g. Supabase edge) where file:// access is blocked — the caller
			// then falls through to the edge-environment error with instructions.
			const denoGlobal = globalThis as unknown as Record<string, unknown>;
			const DenoNs = denoGlobal.Deno as Record<string, unknown>;
			const readFile = DenoNs.readFile as (path: URL) => Promise<Uint8Array>;

			const moduleUrl = new URL(import.meta.url);
			const wasmUrl = new URL("../pkg/kreuzberg_wasm_bg.wasm", moduleUrl);

			const wasmBuffer = await readFile(wasmUrl);
			return wasmBuffer;
		} catch {
			// Supabase edge and other restricted Deno environments block file:// reads.
			// Fall through to let the caller handle the edge environment case.
			return undefined;
		}
	}

	return undefined;
}

import {
	getInitializationError,
	getInitializationPromise,
	getWasmModule,
	isInitialized,
	type ModuleInfo,
	setInitializationError,
	setInitializationPromise,
	setInitialized,
	setWasmModule,
	type WasmModule,
} from "./state.js";

export type { ModuleInfo, WasmModule };
/**
 * Get the loaded WASM module
 *
 * @returns The WASM module instance or null if not loaded
 * @internal
 */
/**
 * Check if WASM module is initialized
 *
 * @returns True if WASM module is initialized, false otherwise
 */
/**
 * Get initialization error if module failed to load
 *
 * @returns The error that occurred during initialization, or null if no error
 * @internal
 */
export { getInitializationError, getWasmModule, isInitialized };

/**
 * Get WASM module version
 *
 * @throws {Error} If WASM module is not initialized
 * @returns The version string of the WASM module
 */
export function getVersion(): string {
	if (!isInitialized()) {
		throw new Error("WASM module not initialized. Call initWasm() first.");
	}

	const wasmModule = getWasmModule();
	if (!wasmModule) {
		throw new Error("WASM module not loaded. Call initWasm() first.");
	}

	return wasmModule.version();
}

/**
 * Initialize the WASM module
 *
 * This function must be called once before using any extraction functions.
 * It loads and initializes the WASM module in the current runtime environment,
 * automatically selecting the appropriate WASM variant for the detected runtime.
 *
 * Multiple calls to initWasm() are safe and will return immediately if already initialized.
 *
 * @param options - Optional configuration for WASM initialization
 * @throws {Error} If WASM module fails to load or is not supported in the current environment
 *
 * @example Basic Usage (Node.js / Browser)
 * ```typescript
 * import { initWasm } from '@kreuzberg/wasm';
 *
 * async function main() {
 *   await initWasm();
 *   // Now you can use extraction functions
 * }
 *
 * main().catch(console.error);
 * ```
 *
 * @example Cloudflare Workers
 * ```typescript
 * import wasmModule from '@kreuzberg/wasm/kreuzberg_wasm_bg.wasm';
 * import { initWasm, extractBytes } from '@kreuzberg/wasm';
 *
 * export default {
 *   async fetch(request: Request): Promise<Response> {
 *     await initWasm({ wasmModule });
 *     const bytes = new Uint8Array(await request.arrayBuffer());
 *     const result = await extractBytes(bytes, 'application/pdf');
 *     return new Response(JSON.stringify(result));
 *   }
 * };
 * ```
 *
 * @example With Error Handling
 * ```typescript
 * import { initWasm, getWasmCapabilities } from '@kreuzberg/wasm';
 *
 * async function initializeKreuzberg() {
 *   const caps = getWasmCapabilities();
 *   if (!caps.hasWasm) {
 *     throw new Error('WebAssembly is not supported in this environment');
 *   }
 *
 *   try {
 *     await initWasm();
 *     console.log('Kreuzberg initialized successfully');
 *   } catch (error) {
 *     console.error('Failed to initialize Kreuzberg:', error);
 *     throw error;
 *   }
 * }
 * ```
 */
export async function initWasm(options?: InitWasmOptions): Promise<void> {
	if (isInitialized()) {
		return;
	}

	let currentPromise = getInitializationPromise();
	if (currentPromise) {
		return currentPromise;
	}

	currentPromise = (async () => {
		try {
			if (!hasWasm()) {
				throw new Error("WebAssembly is not supported in this environment");
			}

			// Import the wasm-bindgen JS glue module. We try multiple paths to handle:
			//   - URL-based: ../pkg/ (workspace-linked), ./kreuzberg_wasm.js (legacy)
			//   - String-based: ./pkg/, ../pkg/ (edge runtimes that can't resolve file:// URLs)
			// String paths use variable construction to avoid Vite static analysis failures.
			const baseUrl = new URL(import.meta.url);
			const modulePaths = [
				new URL("../pkg/kreuzberg_wasm.js", baseUrl).href,
				new URL("./kreuzberg_wasm.js", baseUrl).href,
				[".", "pkg", "kreuzberg_wasm.js"].join("/"),
				["..", "pkg", "kreuzberg_wasm.js"].join("/"),
			];

			let wasmModule: unknown;
			let lastError: unknown;
			for (const modulePath of modulePaths) {
				try {
					wasmModule = await import(/* @vite-ignore */ modulePath);
					break;
				} catch (e) {
					lastError = e;
				}
			}
			if (!wasmModule) {
				throw lastError;
			}
			const loadedModule = wasmModule as unknown as WasmModule;
			setWasmModule(loadedModule);

			if (loadedModule && typeof loadedModule.default === "function") {
				// If a WebAssembly.Module was provided (e.g. for Cloudflare Workers), use it directly.
				if (options?.wasmModule) {
					await loadedModule.default(options.wasmModule);
				} else {
					// In Node.js / Deno, load WASM binary from file system to avoid fetch issues.
					// In restricted Deno environments (Supabase edge) file:// access fails and
					// loadWasmBinaryForNode() returns undefined — we then require an explicit module.
					// In browsers, the default() function uses fetch with import.meta.url.
					const wasmBinary = await loadWasmBinaryForNode();
					if (wasmBinary) {
						await loadedModule.default(wasmBinary);
					} else if (isEdgeEnvironment() || isDeno()) {
						throw new Error(
							"Edge/restricted environment detected (Cloudflare Workers / Vercel Edge / Supabase). " +
								"Cannot automatically load .wasm file because fetch() does not support file:// URLs. " +
								"Pass the WASM module explicitly:\n\n" +
								"  import wasmModule from '@kreuzberg/wasm/kreuzberg_wasm_bg.wasm';\n" +
								"  await initWasm({ wasmModule });\n",
						);
					} else {
						await loadedModule.default();
					}
				}
			}

			if (loadedModule && typeof loadedModule.initialize_pdfium_render === "function") {
				try {
					await initializePdfiumAsync(loadedModule);
				} catch (error) {
					console.warn("PDFium auto-initialization failed (PDF extraction disabled):", error);
				}
			}

			setInitialized(true);
			setInitializationError(null);
		} catch (error) {
			setInitializationError(error instanceof Error ? error : new Error(String(error)));
			throw wrapWasmError(error, "initializing Kreuzberg WASM module");
		}
	})();

	setInitializationPromise(currentPromise);
	return currentPromise;
}

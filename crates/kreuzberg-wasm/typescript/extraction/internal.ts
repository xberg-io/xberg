/**
 * Internal extraction module helpers
 *
 * Provides internal utilities and access to the WASM module state.
 * Re-exports state management from the centralized state module.
 */

import {
	getWasmModule as getWasmModuleFromState,
	isInitialized as isInitializedFromState,
	type WasmModule,
} from "../initialization/state.js";

/**
 * Get the WASM module
 *
 * @returns The WASM module
 * @throws {Error} If WASM module is not loaded
 */
export function getWasmModule(): WasmModule {
	const wasm = getWasmModuleFromState();
	if (!wasm) {
		throw new Error("WASM module not loaded. Call initWasm() first.");
	}

	return wasm;
}

/**
 * Check if WASM module is initialized
 *
 * @returns True if WASM module is initialized
 */
export function isInitialized(): boolean {
	return isInitializedFromState();
}

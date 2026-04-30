/**
 * WASM Initialization State
 *
 * Centralized state management for WASM module initialization.
 * Provides access to the loaded WASM module and initialization status.
 * This module has no dependencies to avoid circular imports.
 */

export type WasmModule = {
	extractBytes: (data: Uint8Array, mimeType: string, config: Record<string, unknown> | null) => Promise<unknown>;
	extractBytesSync: (data: Uint8Array, mimeType: string, config: Record<string, unknown> | null) => unknown;
	batchExtractBytes: (
		dataList: Uint8Array[],
		mimeTypes: string[],
		config: Record<string, unknown> | null,
	) => Promise<unknown>;
	batchExtractBytesSync: (
		dataList: Uint8Array[],
		mimeTypes: string[],
		config: Record<string, unknown> | null,
	) => unknown;
	extractFile: (file: File, mimeType: string | null, config: Record<string, unknown> | null) => Promise<unknown>;
	batchExtractFiles: (files: File[], config: Record<string, unknown> | null) => Promise<unknown>;

	detectMimeFromBytes: (data: Uint8Array) => string;
	normalizeMimeType: (mimeType: string) => string;
	getMimeFromExtension: (extension: string) => string | null;
	getExtensionsForMime: (mimeType: string) => string[];

	loadConfigFromString: (content: string, format: string) => Record<string, unknown>;
	discoverConfig: () => Record<string, unknown>;

	version: () => string;
	get_module_info: () => ModuleInfo;

	register_ocr_backend: (backend: unknown) => void;
	unregister_ocr_backend: (name: string) => void;
	list_ocr_backends: () => string[];
	clear_ocr_backends: () => void;

	register_post_processor: (processor: unknown) => void;
	unregister_post_processor: (name: string) => void;
	list_post_processors: () => string[];
	clear_post_processors: () => void;

	register_validator: (validator: unknown) => void;
	unregister_validator: (name: string) => void;
	list_validators: () => string[];
	clear_validators: () => void;

	initialize_pdfium_render: (pdfiumWasmModule: unknown, localWasmModule: unknown, debug: boolean) => boolean;
	read_block_from_callback_wasm: (param: number, position: number, pBuf: number, size: number) => number;
	write_block_from_callback_wasm: (param: number, buf: number, size: number) => number;

	ocrIsAvailable?: () => boolean;
	ocrRecognize?: (imageBytes: Uint8Array, tessdata: Uint8Array, language: string) => string;
	ocrRecognizeRaw?: (
		imageData: Uint8Array,
		width: number,
		height: number,
		bytesPerPixel: number,
		bytesPerLine: number,
		tessdata: Uint8Array,
		language: string,
	) => string;
	ocrTesseractVersion?: () => string;

	default?: (moduleOrPath?: BufferSource | WebAssembly.Module | string | URL | Response | Request) => Promise<void>;
};

export type ModuleInfo = {
	name: () => string;
	version: () => string;
	free: () => void;
};

/** WASM module instance */
let wasm: WasmModule | null = null;

/** Initialize flag */
let initialized = false;

/** Initialization error (if any) */
let initializationError: Error | null = null;

/** Initialization promise for handling concurrent init calls */
let initializationPromise: Promise<void> | null = null;

/**
 * Get the loaded WASM module
 *
 * @returns The WASM module instance or null if not loaded
 * @internal
 */
export function getWasmModule(): WasmModule | null {
	return wasm;
}

/**
 * Set the WASM module instance
 *
 * @param module The WASM module instance
 * @internal
 */
export function setWasmModule(module: WasmModule): void {
	wasm = module;
}

/**
 * Check if WASM module is initialized
 *
 * @returns True if WASM module is initialized, false otherwise
 */
export function isInitialized(): boolean {
	return initialized;
}

/**
 * Set the initialized flag
 *
 * @param value The initialized state
 * @internal
 */
export function setInitialized(value: boolean): void {
	initialized = value;
}

/**
 * Get initialization error if module failed to load
 *
 * @returns The error that occurred during initialization, or null if no error
 * @internal
 */
export function getInitializationError(): Error | null {
	return initializationError;
}

/**
 * Set the initialization error
 *
 * @param error The error that occurred during initialization
 * @internal
 */
export function setInitializationError(error: Error | null): void {
	initializationError = error;
}

/**
 * Get the current initialization promise
 *
 * @returns The initialization promise or null if not initializing
 * @internal
 */
export function getInitializationPromise(): Promise<void> | null {
	return initializationPromise;
}

/**
 * Set the initialization promise
 *
 * @param promise The initialization promise
 * @internal
 */
export function setInitializationPromise(promise: Promise<void> | null): void {
	initializationPromise = promise;
}

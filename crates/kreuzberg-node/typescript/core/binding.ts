/**
 * @internal Native NAPI binding interface and management.
 *
 * This module provides:
 * - NativeBinding interface defining all methods available in the compiled Rust addon
 * - getBinding() singleton getter with lazy loading
 * - loadNativeBinding() for loading the native .node module
 * - Native binding error formatting
 *
 * This is a Layer 0 module with NO dependencies on other internal modules.
 */

import { createRequire } from "node:module";

/**
 * @internal Native NAPI binding interface for the Kreuzberg native module.
 * This interface defines the shape of methods available in the compiled native addon.
 */
export interface NativeBinding {
	extractFileSync(
		filePath: string,
		mimeType: string | null,
		config: Record<string, unknown> | null,
	): Record<string, unknown>;
	extractFile(
		filePath: string,
		mimeType: string | null,
		config: Record<string, unknown> | null,
	): Promise<Record<string, unknown>>;
	extractBytesSync(data: Buffer, mimeType: string, config: Record<string, unknown> | null): Record<string, unknown>;
	extractBytes(
		data: Buffer,
		mimeType: string,
		config: Record<string, unknown> | null,
	): Promise<Record<string, unknown>>;
	batchExtractFilesSync(paths: string[], config: Record<string, unknown> | null): Record<string, unknown>[];
	batchExtractFiles(paths: string[], config: Record<string, unknown> | null): Promise<Record<string, unknown>[]>;
	batchExtractBytesSync(
		dataArray: Buffer[],
		mimeTypes: string[],
		config: Record<string, unknown> | null,
	): Record<string, unknown>[];
	batchExtractBytes(
		dataArray: Buffer[],
		mimeTypes: string[],
		config: Record<string, unknown> | null,
	): Promise<Record<string, unknown>[]>;
	registerPostProcessor(processor: Record<string, unknown>): void;
	unregisterPostProcessor(name: string): void;
	clearPostProcessors(): void;
	listPostProcessors(): string[];
	registerValidator(validator: Record<string, unknown>): void;
	unregisterValidator(name: string): void;
	clearValidators(): void;
	listValidators(): string[];
	registerOcrBackend(backend: Record<string, unknown>): void;
	unregisterOcrBackend(name: string): void;
	clearOcrBackends(): void;
	listOcrBackends(): string[];
	registerDocumentExtractor(extractor: Record<string, unknown>): void;
	unregisterDocumentExtractor(name: string): void;
	clearDocumentExtractors(): void;
	listDocumentExtractors(): string[];
	detectMimeType(filePath: string): string;
	detectMimeTypeFromBytes(data: Buffer): string;
	detectMimeTypeFromPath(filePath: string, checkExists?: boolean): string;
	validateMimeType(mimeType: string): string;
	getExtensionsForMime(mimeType: string): string[];
	listEmbeddingPresets(): string[];
	getEmbeddingPreset(name: string): Record<string, unknown> | null;
	getErrorCodeName(code: number): string;
	getErrorCodeDescription(code: number): string;
	classifyError(errorMessage: string): Record<string, unknown>;
	getLastErrorCode(): number;
	getLastPanicContext(): Record<string, unknown> | null;
	loadExtractionConfigFromFile(filePath: string): Record<string, unknown>;
	discoverExtractionConfig(): Record<string, unknown> | null;
	createWorkerPool(size?: number): Record<string, unknown>;
	getWorkerPoolStats(pool: Record<string, unknown>): Record<string, unknown>;
	extractFileInWorker(
		pool: Record<string, unknown>,
		filePath: string,
		mimeType: string | null,
		config: Record<string, unknown> | null,
	): Promise<Record<string, unknown>>;
	batchExtractFilesInWorker(
		pool: Record<string, unknown>,
		paths: string[],
		config: Record<string, unknown> | null,
	): Promise<Record<string, unknown>[]>;
	closeWorkerPool(pool: Record<string, unknown>): Promise<void>;
}

/**
 * Global singleton reference to the native binding.
 * Lazy-loaded on first call to getBinding().
 * @internal
 */
let binding: NativeBinding | null = null;

/**
 * Flag indicating whether we've attempted to load the binding.
 * Prevents repeated load attempts if initialization fails.
 * @internal
 */
let bindingInitialized = false;

/**
 * Creates a formatted error for native binding load failures.
 * Includes helpful hints based on the error type.
 * @internal
 */
export function createNativeBindingError(error: unknown): Error {
	const hintParts: string[] = [];
	let detail = "Unknown error while requiring native module.";

	if (error instanceof Error) {
		detail = error.message || error.toString();
		if (/pdfium/i.test(detail)) {
			hintParts.push(
				"Pdfium runtime library was not found. Ensure the bundled libpdfium (dll/dylib/so) is present next to the native module.",
			);
		}
		return new Error(
			[
				"Failed to load Kreuzberg native bindings.",
				hintParts.length ? hintParts.join(" ") : "",
				"Report this error and attach the logs/stack trace for investigation.",
				`Underlying error: ${detail}`,
			]
				.filter(Boolean)
				.join(" "),
			{ cause: error },
		);
	}

	return new Error(
		[
			"Failed to load Kreuzberg native bindings.",
			"Report this error and attach the logs/stack trace for investigation.",
			`Underlying error: ${String(error)}`,
		].join(" "),
	);
}

/**
 * Loads the native binding from the compiled .node module.
 * Validates that all required methods are present before returning.
 * @internal
 */
export function loadNativeBinding(): NativeBinding {
	let localRequire: ((path: string) => unknown) | undefined;

	// In CJS, require is already available globally
	if (typeof require !== "undefined") {
		localRequire = require as (path: string) => unknown;
	} else {
		// In ESM, we need to create require from import.meta.url
		try {
			localRequire = createRequire(import.meta.url);
		} catch {
			localRequire = undefined;
		}
	}

	if (!localRequire) {
		throw new Error("Unable to resolve native binding loader (require not available).");
	}

	// Load from package root index.js (NAPI-RS generated loader), not compiled barrel export
	// When bundled: From dist/index.js, ../index.js points to package root index.js
	// When unbundled: From dist/core/binding.js, ../../index.js points to package root index.js
	// Since we're using bundle: true, use ../index.js
	const loadedModule = localRequire("../index.js") as unknown;

	if (typeof loadedModule !== "object" || loadedModule === null) {
		throw new Error(
			"Native binding is not a valid object. " + "Ensure the native module is properly built and compatible.",
		);
	}

	const module = loadedModule as Record<string, unknown>;

	const requiredMethods = [
		"extractFileSync",
		"extractFile",
		"extractBytesSync",
		"extractBytes",
		"batchExtractFilesSync",
		"batchExtractFiles",
		"batchExtractBytesSync",
		"batchExtractBytes",
	];

	for (const method of requiredMethods) {
		if (typeof module[method] !== "function") {
			throw new Error(
				`Native binding is missing required method: ${method}. ` +
					"Ensure the native module is properly built and compatible.",
			);
		}
	}

	return module as unknown as NativeBinding;
}

/**
 * Gets the native binding, with lazy loading on first access.
 *
 * The native binding is loaded on first call and cached for subsequent calls.
 * If loading fails, the error is cached and re-thrown on every subsequent call
 * to prevent repeated load attempts.
 *
 * @returns The native binding interface
 * @throws Error if the binding cannot be loaded
 * @internal
 */
export function getBinding(): NativeBinding {
	if (bindingInitialized) {
		if (binding === null) {
			throw new Error("Native binding was previously failed to load.");
		}
		return binding;
	}

	try {
		if (typeof process !== "undefined" && process.versions && process.versions.node) {
			binding = loadNativeBinding();
			bindingInitialized = true;
			return binding;
		}
	} catch (error) {
		bindingInitialized = true;
		throw createNativeBindingError(error);
	}

	throw new Error(
		"Failed to load Kreuzberg bindings. Neither NAPI (Node.js) nor WASM (browsers/Deno) bindings are available. " +
			"Make sure you have installed the @kreuzberg/node package for Node.js/Bun.",
	);
}

/**
 * @internal Allows tests to provide a mocked native binding.
 */
export function __setBindingForTests(mock: unknown): void {
	binding = mock as NativeBinding;
	bindingInitialized = true;
}

/**
 * @internal Resets the cached native binding for tests.
 */
export function __resetBindingForTests(): void {
	binding = null;
	bindingInitialized = false;
}

import { getBinding } from "../core/binding.js";
import type { OcrBackendProtocol } from "../types.js";

/**
 * Type definitions for OCR payload handling
 */
type OcrProcessPayload = Buffer | string;
type OcrProcessTuple = [OcrProcessPayload, string];
type NestedOcrProcessTuple = [OcrProcessTuple];

/**
 * Type guard for OCR process tuple
 */
function isOcrProcessTuple(value: unknown): value is OcrProcessTuple {
	return (
		Array.isArray(value) &&
		value.length === 2 &&
		typeof value[1] === "string" &&
		(typeof value[0] === "string" || Buffer.isBuffer(value[0]) || value[0] instanceof Uint8Array)
	);
}

/**
 * Type guard for nested OCR process tuple
 */
function isNestedOcrProcessTuple(value: unknown): value is NestedOcrProcessTuple {
	return Array.isArray(value) && value.length === 1 && isOcrProcessTuple(value[0]);
}

/**
 * Describes an OCR payload for debugging
 */
function describePayload(value: OcrProcessPayload) {
	if (typeof value === "string") {
		return { ctor: "String", length: value.length };
	}

	return { ctor: value.constructor?.name ?? "Buffer", length: value.length };
}

/**
 * Register a custom OCR backend.
 *
 * This function registers a JavaScript OCR backend that will be used by Kreuzberg's
 * extraction pipeline when OCR is enabled. The backend must implement the
 * {@link OcrBackendProtocol} interface.
 *
 * ## Usage
 *
 * 1. Create a class implementing {@link OcrBackendProtocol}
 * 2. Call `initialize()` on your backend instance (if needed)
 * 3. Register the backend with `registerOcrBackend()`
 * 4. Use the backend name in extraction config
 *
 * ## Thread Safety
 *
 * The registered backend must be thread-safe as it may be called concurrently
 * from multiple Rust async tasks. Ensure your implementation handles concurrent
 * calls properly.
 *
 * @param backend - OcrBackendProtocol implementation with name(), supportedLanguages(), and processImage()
 * @throws {Error} If backend is missing required methods (name, supportedLanguages, or processImage)
 * @throws {Error} If backend name is empty string or contains invalid characters
 * @throws {Error} If a backend with the same name is already registered
 * @throws {Error} If registration fails due to FFI issues
 *
 * @example
 * ```typescript
 * import { GutenOcrBackend } from '@kreuzberg/node/ocr/guten-ocr';
 * import { registerOcrBackend, extractFile } from '@kreuzberg/node';
 *
 * // Create and initialize backend
 * const backend = new GutenOcrBackend();
 * await backend.initialize();
 *
 * // Register with Kreuzberg
 * registerOcrBackend(backend);
 *
 * // Use in extraction
 * const result = await extractFile('scanned.pdf', null, {
 *   ocr: { backend: 'guten-ocr', language: 'en' }
 * });
 * console.log(result.content);
 * ```
 *
 * @example
 * ```typescript
 * import { registerOcrBackend } from '@kreuzberg/node';
 *
 * class MyOcrBackend {
 *   name() {
 *     return 'my-ocr';
 *   }
 *
 *   supportedLanguages(): string[] {
 *     return ['en', 'de', 'fr'];
 *   }
 *
 *   async processImage(imageBytes: Uint8Array, language: string) {
 *     const text = await myCustomOcrEngine(imageBytes, language);
 *     return {
 *       content: text,
 *       mime_type: 'text/plain',
 *       metadata: { confidence: 0.95, language },
 *       tables: []
 *     };
 *   }
 * }
 *
 * registerOcrBackend(new MyOcrBackend());
 * ```
 */
export function registerOcrBackend(backend: OcrBackendProtocol): void {
	const binding = getBinding();

	const wrappedBackend = {
		name: typeof backend.name === "function" ? backend.name() : backend.name,
		supportedLanguages:
			typeof backend.supportedLanguages === "function"
				? backend.supportedLanguages()
				: (backend.supportedLanguages ?? ["en"]),
		async processImage(
			...processArgs: [OcrProcessPayload | OcrProcessTuple | NestedOcrProcessTuple, string?]
		): Promise<string> {
			const [imagePayload, maybeLanguage] = processArgs;
			// biome-ignore lint/complexity/useLiteralKeys: required for environment variable access
			if (process.env["KREUZBERG_DEBUG_GUTEN"] === "1") {
				console.log("[registerOcrBackend] JS arguments", { length: processArgs.length });
				console.log("[registerOcrBackend] Raw args", {
					imagePayloadType: Array.isArray(imagePayload) ? "tuple" : typeof imagePayload,
					maybeLanguageType: typeof maybeLanguage,
					metadata: Array.isArray(imagePayload) ? { tupleLength: imagePayload.length } : describePayload(imagePayload),
				});
			}

			let rawBytes: OcrProcessPayload;
			let language = maybeLanguage;

			if (isNestedOcrProcessTuple(imagePayload)) {
				[rawBytes, language] = imagePayload[0];
			} else if (isOcrProcessTuple(imagePayload)) {
				[rawBytes, language] = imagePayload;
			} else {
				rawBytes = imagePayload;
			}

			if (typeof language !== "string") {
				throw new Error("OCR backend did not receive a language parameter");
			}

			// biome-ignore lint/complexity/useLiteralKeys: required for environment variable access
			if (process.env["KREUZBERG_DEBUG_GUTEN"] === "1") {
				const length = typeof rawBytes === "string" ? rawBytes.length : rawBytes.length;
				console.log(
					"[registerOcrBackend] Received payload",
					Array.isArray(imagePayload) ? "tuple" : typeof rawBytes,
					"ctor",
					describePayload(rawBytes).ctor,
					"length",
					length,
				);
			}

			const buffer = typeof rawBytes === "string" ? Buffer.from(rawBytes, "base64") : Buffer.from(rawBytes);
			const result = await backend.processImage(new Uint8Array(buffer), language);

			return JSON.stringify(result);
		},
	};

	binding.registerOcrBackend(wrappedBackend);
}

/**
 * List all registered OCR backends.
 *
 * Returns an array of names of all currently registered OCR backends,
 * including built-in backends like "tesseract".
 *
 * @returns Array of OCR backend names (empty array if none registered)
 *
 * @example
 * ```typescript
 * import { listOcrBackends } from '@kreuzberg/node';
 *
 * const backends = listOcrBackends();
 * console.log(backends); // ['tesseract', 'my-custom-backend', ...]
 * ```
 */
export function listOcrBackends(): string[] {
	const binding = getBinding();
	return binding.listOcrBackends();
}

/**
 * Unregister an OCR backend by name.
 *
 * Removes the specified OCR backend from the registry. If the backend doesn't exist,
 * this operation is a no-op (does not throw an error).
 *
 * @param name - Name of the OCR backend to unregister
 *
 * @example
 * ```typescript
 * import { unregisterOcrBackend } from '@kreuzberg/node';
 *
 * // Unregister a custom backend
 * unregisterOcrBackend('my-custom-ocr');
 * ```
 */
export function unregisterOcrBackend(name: string): void {
	const binding = getBinding();
	binding.unregisterOcrBackend(name);
}

/**
 * Clear all registered OCR backends.
 *
 * Removes all OCR backends from the registry, including built-in backends.
 * Use with caution as this will make OCR functionality unavailable until
 * backends are re-registered. If no backends are registered, this is a no-op.
 *
 * @example
 * ```typescript
 * import { clearOcrBackends } from '@kreuzberg/node';
 *
 * clearOcrBackends();
 * ```
 */
export function clearOcrBackends(): void {
	const binding = getBinding();
	binding.clearOcrBackends();
}

/**
 * Get a registered OCR backend by name.
 *
 * Retrieves information about a specific OCR backend from the registry.
 *
 * @param name - Name of the OCR backend to retrieve
 * @returns The backend if found, null otherwise
 *
 * @example
 * ```typescript
 * import { getOcrBackend } from '@kreuzberg/node';
 *
 * const backend = getOcrBackend('tesseract');
 * if (backend) {
 *   console.log('Backend found:', backend.name);
 * }
 * ```
 */
export function getOcrBackend(name: string): unknown {
	// Note: This function is not directly exposed by the native binding
	// It's a helper function that uses listOcrBackends to check if a backend exists
	const backends = listOcrBackends();
	return backends.includes(name) ? { name } : null;
}

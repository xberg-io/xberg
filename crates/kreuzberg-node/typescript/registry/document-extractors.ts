import { getBinding } from "../core/binding.js";

/**
 * List all registered document extractors.
 *
 * Returns an array of names of all currently registered document extractors,
 * including built-in extractors for PDF, Office documents, images, etc.
 *
 * @returns Array of document extractor names (empty array if none registered)
 *
 * @example
 * ```typescript
 * import { listDocumentExtractors } from '@kreuzberg/node';
 *
 * const extractors = listDocumentExtractors();
 * console.log(extractors); // ['pdf', 'docx', 'xlsx', 'custom-extractor', ...]
 * ```
 */
export function listDocumentExtractors(): string[] {
	const binding = getBinding();
	return binding.listDocumentExtractors();
}

/**
 * Unregister a document extractor by name.
 *
 * Removes the specified document extractor from the registry. If the extractor
 * doesn't exist, this operation is a no-op (does not throw an error).
 *
 * @param name - Name of the document extractor to unregister
 *
 * @example
 * ```typescript
 * import { unregisterDocumentExtractor } from '@kreuzberg/node';
 *
 * // Unregister a custom extractor
 * unregisterDocumentExtractor('MyCustomExtractor');
 * ```
 */
export function unregisterDocumentExtractor(name: string): void {
	const binding = getBinding();
	binding.unregisterDocumentExtractor(name);
}

/**
 * Clear all registered document extractors.
 *
 * Removes all document extractors from the registry, including built-in extractors.
 * Use with caution as this will make document extraction unavailable until
 * extractors are re-registered.
 *
 * @example
 * ```typescript
 * import { clearDocumentExtractors } from '@kreuzberg/node';
 *
 * clearDocumentExtractors();
 * ```
 */
export function clearDocumentExtractors(): void {
	const binding = getBinding();
	binding.clearDocumentExtractors();
}

/**
 * Get a registered document extractor by name.
 *
 * Retrieves information about a specific document extractor from the registry.
 *
 * @param name - Name of the document extractor to retrieve
 * @returns The extractor if found, null otherwise
 *
 * @example
 * ```typescript
 * import { getDocumentExtractor } from '@kreuzberg/node';
 *
 * const extractor = getDocumentExtractor('pdf');
 * if (extractor) {
 *   console.log('Extractor found:', extractor.name);
 * }
 * ```
 */
export function getDocumentExtractor(name: string): unknown {
	// Note: This function is not directly exposed by the native binding
	// It's a helper function that uses listDocumentExtractors to check if an extractor exists
	const extractors = listDocumentExtractors();
	return extractors.includes(name) ? { name } : null;
}

/**
 * Register a custom document extractor.
 *
 * Note: The native binding does not expose a registerDocumentExtractor method.
 * Document extractors are typically registered at the Rust level or through
 * configuration. This function is provided for consistency with the API.
 *
 * @param _name - Name of the document extractor (not used - for API consistency)
 * @throws {Error} Not implemented - document extractors cannot be registered from TypeScript
 *
 * @example
 * ```typescript
 * // Document extractors are typically defined in Rust or configuration
 * // This function is not available in the current API
 * ```
 */
export function registerDocumentExtractor(_name: string): void {
	throw new Error(
		"registerDocumentExtractor is not available. Document extractors must be registered at the Rust level.",
	);
}

import { getBinding } from "../core/binding.js";
import type { PanicContext } from "../errors.js";
import type { ErrorClassification } from "../types.js";

/**
 * Get the error code for the last FFI error.
 *
 * Returns the FFI error code as an integer. This is useful for programmatic error handling
 * and distinguishing between different types of failures in native code.
 *
 * Error codes:
 * - 0: Success (no error)
 * - 1: GenericError
 * - 2: Panic
 * - 3: InvalidArgument
 * - 4: IoError
 * - 5: ParsingError
 * - 6: OcrError
 * - 7: MissingDependency
 *
 * @returns The integer error code
 *
 * @example
 * ```typescript
 * import { extractFile, getLastErrorCode, ErrorCode } from '@kreuzberg/node';
 *
 * try {
 *   const result = await extractFile('document.pdf');
 * } catch (error) {
 *   const code = getLastErrorCode();
 *   if (code === ErrorCode.Panic) {
 *     console.error('Native code panic detected');
 *   }
 * }
 * ```
 */
export function getLastErrorCode(): number {
	const binding = getBinding();
	return binding.getLastErrorCode();
}

/**
 * Get panic context information if the last error was a panic.
 *
 * Returns detailed information about a panic in native code, or null if the last error was not a panic.
 * This provides debugging information when native code panics.
 *
 * @returns A `PanicContext` object with file, line, function, message, and timestamp_secs, or null if no panic context is available
 *
 * @example
 * ```typescript
 * import { extractFile, getLastPanicContext } from '@kreuzberg/node';
 *
 * try {
 *   const result = await extractFile('document.pdf');
 * } catch (error) {
 *   const context = getLastPanicContext();
 *   if (context) {
 *     console.error(`Panic at ${context.file}:${context.line}`);
 *     console.error(`In function: ${context.function}`);
 *     console.error(`Message: ${context.message}`);
 *   }
 * }
 * ```
 */
export function getLastPanicContext(): PanicContext | null {
	const binding = getBinding();
	const result = binding.getLastPanicContext();
	return result as unknown as PanicContext | null;
}

/**
 * Returns the human-readable name for an error code.
 *
 * Maps numeric error codes to their string names, providing a consistent way
 * to get error code names across all platforms.
 *
 * @param code - The numeric error code (0-7)
 * @returns The error code name as a string (e.g., "validation", "ocr", "unknown")
 *
 * @example
 * ```typescript
 * import { getErrorCodeName } from '@kreuzberg/node';
 *
 * const name = getErrorCodeName(0);  // returns "validation"
 * const name = getErrorCodeName(2);  // returns "ocr"
 * const name = getErrorCodeName(99); // returns "unknown"
 * ```
 */
export function getErrorCodeName(code: number): string {
	const binding = getBinding();
	return binding.getErrorCodeName(code);
}

/**
 * Returns the description for an error code.
 *
 * Retrieves user-friendly descriptions of error types from the FFI layer.
 *
 * @param code - The numeric error code (0-7)
 * @returns A brief description of the error type
 *
 * @example
 * ```typescript
 * import { getErrorCodeDescription } from '@kreuzberg/node';
 *
 * const desc = getErrorCodeDescription(0);  // returns "Input validation error"
 * const desc = getErrorCodeDescription(4);  // returns "File system I/O error"
 * const desc = getErrorCodeDescription(99); // returns "Unknown error code"
 * ```
 */
export function getErrorCodeDescription(code: number): string {
	const binding = getBinding();
	return binding.getErrorCodeDescription(code);
}

/**
 * Classifies an error message string into an error code category.
 *
 * This function analyzes the error message content and returns the most likely
 * error code (0-7) based on keyword patterns. Used to programmatically classify
 * errors for handling purposes.
 *
 * The classification is based on keyword matching:
 * - **Validation (0)**: Keywords like "invalid", "validation", "schema", "required"
 * - **Parsing (1)**: Keywords like "parsing", "corrupted", "malformed"
 * - **Ocr (2)**: Keywords like "ocr", "tesseract", "language", "model"
 * - **MissingDependency (3)**: Keywords like "not found", "missing", "dependency"
 * - **Io (4)**: Keywords like "file", "disk", "read", "write", "permission"
 * - **Plugin (5)**: Keywords like "plugin", "register", "extension"
 * - **UnsupportedFormat (6)**: Keywords like "unsupported", "format", "mime"
 * - **Internal (7)**: Keywords like "internal", "bug", "panic"
 *
 * @param errorMessage - The error message string to classify
 * @returns An object with the classification details
 *
 * @example
 * ```typescript
 * import { classifyError } from '@kreuzberg/node';
 *
 * const result = classifyError("PDF file is corrupted");
 * // Returns: { code: 1, name: "parsing", confidence: 0.95 }
 *
 * const result = classifyError("Tesseract not found");
 * // Returns: { code: 3, name: "missing_dependency", confidence: 0.9 }
 * ```
 */
export function classifyError(errorMessage: string): ErrorClassification {
	const binding = getBinding();
	const result = binding.classifyError(errorMessage);
	return result as unknown as ErrorClassification;
}

/**
 * Get missing dependencies information.
 *
 * Note: The native binding does not directly expose a getMissingDependencies method.
 * This function is provided for API consistency with diagnostic functionality.
 *
 * @returns Array of missing dependency names
 *
 * @throws {Error} Not implemented - use checkOcrDependencies() instead
 *
 * @deprecated Use checkOcrDependencies() or getSystemInfo() instead
 */
export function getMissingDependencies(): string[] {
	throw new Error("getMissingDependencies is not directly available. Use checkOcrDependencies() instead.");
}

/**
 * Get available OCR backends information.
 *
 * Note: The native binding does not directly expose this as a separate method.
 * Use listOcrBackends() from plugins/ocr-backends.ts instead.
 *
 * @returns Array of available OCR backend names
 *
 * @throws {Error} Not implemented - use listOcrBackends() instead
 *
 * @deprecated Use listOcrBackends() from @kreuzberg/node instead
 */
export function getAvailableOcrBackends(): string[] {
	throw new Error("getAvailableOcrBackends is not directly available. Use listOcrBackends() instead.");
}

/**
 * Check OCR dependencies.
 *
 * Note: The native binding does not directly expose dependency checking.
 * This function is provided for API consistency.
 *
 * @returns Object with OCR dependency status
 *
 * @throws {Error} Not implemented - check error codes after extraction attempts instead
 *
 * @deprecated Check error codes after extraction attempts instead
 */
export function checkOcrDependencies(): Record<string, unknown> {
	throw new Error(
		"checkOcrDependencies is not directly available. Check error codes after extraction attempts instead.",
	);
}

/**
 * Get system information.
 *
 * Note: The native binding does not directly expose system information.
 * This function is provided for API consistency.
 *
 * @returns Object with system information
 *
 * @throws {Error} Not implemented
 *
 * @deprecated Not available in current API
 */
export function getSystemInfo(): Record<string, unknown> {
	throw new Error("getSystemInfo is not available in the current API.");
}

/**
 * Get diagnostic information.
 *
 * Note: The native binding does not directly expose comprehensive diagnostic information.
 * This function is provided for API consistency.
 *
 * @returns Object with diagnostic information
 *
 * @throws {Error} Not implemented
 *
 * @deprecated Not available in current API
 */
export function diagnosticInfo(): Record<string, unknown> {
	throw new Error("diagnosticInfo is not available in the current API.");
}

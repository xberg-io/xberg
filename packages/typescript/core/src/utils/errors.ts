/**
 * Error utilities and classification for Kreuzberg document intelligence framework.
 *
 * Provides helper functions for working with Kreuzberg errors, including error code
 * name/description lookups and error message classification.
 *
 * @module @kreuzberg/core/utils/errors
 */

import {
	getErrorCodeName as get_error_code_name,
	getErrorCodeDescription as get_error_code_description,
	classifyError as classify_error,
	type ErrorClassification,
} from "@kreuzberg/node";

/**
 * Standard error codes for all Kreuzberg bindings.
 *
 * These codes are the single source of truth for error classification across
 * all language bindings (Python, Ruby, Go, Java, TypeScript).
 *
 * @example
 * ```typescript
 * import { ErrorCode } from '@kreuzberg/node';
 *
 * if (errorCode === ErrorCode.Validation) {
 *   console.error('Invalid configuration provided');
 * }
 * ```
 */
export const ErrorCode = {
	/**
	 * Input validation error (invalid config, parameters, paths)
	 */
	Validation: 0,
	/**
	 * Document parsing error (corrupt files, unsupported format features)
	 */
	Parsing: 1,
	/**
	 * OCR processing error (backend failures, image quality issues)
	 */
	Ocr: 2,
	/**
	 * Missing system dependency (tesseract not found, pandoc not installed)
	 */
	MissingDependency: 3,
	/**
	 * File system I/O error (permissions, disk full, file not found)
	 */
	Io: 4,
	/**
	 * Plugin registration or execution error
	 */
	Plugin: 5,
	/**
	 * Unsupported MIME type or file format
	 */
	UnsupportedFormat: 6,
	/**
	 * Internal library error (indicates a bug, should rarely occur)
	 */
	Internal: 7,
} as const;

/**
 * Type for error code values.
 *
 * @internal
 */
export type ErrorCodeValue = (typeof ErrorCode)[keyof typeof ErrorCode];

/**
 * Returns the human-readable name for an error code.
 *
 * This function retrieves error code names from the FFI layer, providing
 * a consistent way to get error code names across all platforms.
 *
 * @param code - The numeric error code (0-7)
 * @returns The error code name as a string (e.g., "validation", "ocr", "unknown")
 *
 * @example
 * ```typescript
 * import { getErrorCodeName, ErrorCode } from '@kreuzberg/node';
 *
 * const name = getErrorCodeName(ErrorCode.Validation);
 * console.log(name); // Output: "validation"
 *
 * const unknownName = getErrorCodeName(99);
 * console.log(unknownName); // Output: "unknown"
 * ```
 */
export function getErrorCodeName(code: number): string {
	return get_error_code_name(code);
}

/**
 * Returns the description for an error code.
 *
 * This function retrieves error code descriptions from the FFI layer, providing
 * user-friendly descriptions of error types.
 *
 * @param code - The numeric error code (0-7)
 * @returns A brief description of the error type
 *
 * @example
 * ```typescript
 * import { getErrorCodeDescription, ErrorCode } from '@kreuzberg/node';
 *
 * const desc = getErrorCodeDescription(ErrorCode.Io);
 * console.log(desc); // Output: "File system I/O error"
 *
 * const defaultDesc = getErrorCodeDescription(99);
 * console.log(defaultDesc); // Output: "Unknown error code"
 * ```
 */
export function getErrorCodeDescription(code: number): string {
	return get_error_code_description(code);
}

/**
 * Classifies an error message string into an error code category.
 *
 * This function analyzes the error message content and returns the most likely
 * error code (0-7) based on keyword patterns. It includes a confidence score
 * to indicate how certain the classification is.
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
 * import { classifyErrorMessage, ErrorCode } from '@kreuzberg/node';
 *
 * const result = classifyErrorMessage("PDF file is corrupted");
 * if (result.code === ErrorCode.Parsing) {
 *   console.error('Document format issue:', result.name);
 * }
 *
 * const result = classifyErrorMessage("Tesseract not found");
 * console.log(result.confidence); // Confidence of classification (0.0-1.0)
 * ```
 */
export function classifyErrorMessage(errorMessage: string): ErrorClassification {
	return classify_error(errorMessage);
}

/**
 * Checks if an error code is valid.
 *
 * Valid error codes are in the range [0, 7].
 *
 * @param code - The numeric error code to validate
 * @returns true if the code is valid, false otherwise
 *
 * @example
 * ```typescript
 * import { isValidErrorCode } from '@kreuzberg/node';
 *
 * if (isValidErrorCode(0)) {
 *   console.log('Valid error code');
 * }
 *
 * if (!isValidErrorCode(99)) {
 *   console.log('Invalid error code');
 * }
 * ```
 */
export function isValidErrorCode(code: number): boolean {
	return Number.isInteger(code) && code >= 0 && code <= 7;
}

/**
 * Gets the name of an error code by its value.
 *
 * This is a helper function that maps numeric error codes to their names
 * for easier error handling in switch statements.
 *
 * @param code - The numeric error code
 * @returns The error code name key (e.g., "Validation", "Parsing") or null if invalid
 *
 * @example
 * ```typescript
 * import { getErrorCodeKey, ErrorCode } from '@kreuzberg/node';
 *
 * const key = getErrorCodeKey(0);
 * console.log(key); // Output: "Validation"
 * ```
 */
export function getErrorCodeKey(code: number): keyof typeof ErrorCode | null {
	const keys = Object.keys(ErrorCode) as Array<keyof typeof ErrorCode>;
	for (const key of keys) {
		if (ErrorCode[key] === code) {
			return key;
		}
	}
	return null;
}

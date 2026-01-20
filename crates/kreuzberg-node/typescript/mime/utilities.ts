import { getBinding } from "../core/binding.js";

/**
 * Detect MIME type from raw bytes.
 *
 * Uses content inspection (magic bytes) to determine MIME type.
 * This is more accurate than extension-based detection but requires
 * reading the file content.
 *
 * @param bytes - Raw file content as Buffer
 * @returns The detected MIME type string
 *
 * @throws {Error} If MIME type cannot be determined from content
 *
 * @example
 * ```typescript
 * import { detectMimeType } from '@kreuzberg/node';
 * import * as fs from 'fs';
 *
 * // Read file content
 * const content = fs.readFileSync('document.pdf');
 *
 * // Detect MIME type from bytes
 * const mimeType = detectMimeType(content);
 * console.log(mimeType); // 'application/pdf'
 * ```
 */
export function detectMimeType(bytes: Buffer): string {
	const binding = getBinding();
	return binding.detectMimeTypeFromBytes(bytes);
}

/**
 * Detect MIME type from a file path.
 *
 * Determines the MIME type based on the file extension in the provided path.
 * By default, checks if the file exists; can be disabled with checkExists parameter.
 *
 * @param filePath - The file path to detect MIME type from (e.g., 'document.pdf')
 * @param checkExists - Whether to verify the file exists (default: true)
 * @returns The detected MIME type as a string (e.g., 'application/pdf')
 *
 * @throws {Error} If MIME type cannot be determined from the file extension,
 * or if checkExists is true and the file does not exist
 *
 * @example
 * ```typescript
 * import { detectMimeTypeFromPath } from '@kreuzberg/node';
 *
 * // Detect MIME type from existing file
 * const mimeType = detectMimeTypeFromPath('/path/to/document.pdf');
 * console.log(mimeType); // 'application/pdf'
 *
 * // Detect without checking file existence
 * const mimeType2 = detectMimeTypeFromPath('document.docx', false);
 * console.log(mimeType2); // 'application/vnd.openxmlformats-officedocument.wordprocessingml.document'
 * ```
 */
export function detectMimeTypeFromPath(filePath: string, checkExists?: boolean): string {
	const binding = getBinding();
	return binding.detectMimeTypeFromPath(filePath, checkExists);
}

/**
 * Validate that a MIME type is supported by Kreuzberg.
 *
 * Checks if a MIME type is in the list of supported formats. Note that any
 * `image/*` MIME type is automatically considered valid.
 *
 * @param mimeType - The MIME type to validate (string)
 * @returns The validated MIME type (may be normalized)
 *
 * @throws {Error} If the MIME type is not supported
 *
 * @example
 * ```typescript
 * import { validateMimeType } from '@kreuzberg/node';
 *
 * // Validate supported type
 * const validated = validateMimeType('application/pdf');
 * console.log(validated); // 'application/pdf'
 *
 * // Validate custom image type
 * const validated2 = validateMimeType('image/custom-format');
 * console.log(validated2); // 'image/custom-format' (any image/* is valid)
 *
 * // Validate unsupported type (throws error)
 * try {
 *   validateMimeType('video/mp4');
 * } catch (err) {
 *   console.error(err); // Error: Unsupported format: video/mp4
 * }
 * ```
 */
export function validateMimeType(mimeType: string): string {
	const binding = getBinding();
	return binding.validateMimeType(mimeType);
}

/**
 * Get file extensions for a given MIME type.
 *
 * Returns an array of file extensions commonly associated with the specified
 * MIME type. For example, 'application/pdf' returns ['pdf'].
 *
 * @param mimeType - The MIME type to look up (e.g., 'application/pdf', 'image/jpeg')
 * @returns Array of file extensions (without leading dots)
 *
 * @throws {Error} If the MIME type is not recognized or supported
 *
 * @example
 * ```typescript
 * import { getExtensionsForMime } from '@kreuzberg/node';
 *
 * // Get extensions for PDF
 * const pdfExts = getExtensionsForMime('application/pdf');
 * console.log(pdfExts); // ['pdf']
 *
 * // Get extensions for JPEG
 * const jpegExts = getExtensionsForMime('image/jpeg');
 * console.log(jpegExts); // ['jpg', 'jpeg']
 * ```
 */
export function getExtensionsForMime(mimeType: string): string[] {
	const binding = getBinding();
	return binding.getExtensionsForMime(mimeType);
}

/**
 * Detect MIME type synchronously from raw bytes.
 *
 * Synchronous version of detectMimeType().
 * Uses content inspection (magic bytes) to determine MIME type.
 *
 * @param bytes - Raw file content as Buffer
 * @returns The detected MIME type string
 *
 * @throws {Error} If MIME type cannot be determined from content
 *
 * @deprecated This is an alias for detectMimeType() which is already synchronous
 *
 * @example
 * ```typescript
 * import { detectMimeTypeSync } from '@kreuzberg/node';
 * import * as fs from 'fs';
 *
 * const content = fs.readFileSync('document.pdf');
 * const mimeType = detectMimeTypeSync(content);
 * ```
 */
export function detectMimeTypeSync(bytes: Buffer): string {
	return detectMimeType(bytes);
}

/**
 * Get MIME type from bytes synchronously.
 *
 * Synchronous version for getting MIME type from raw bytes.
 *
 * @param bytes - Raw file content as Buffer
 * @returns The detected MIME type string
 *
 * @throws {Error} If MIME type cannot be determined
 *
 * @deprecated Use detectMimeType() instead
 */
export function getMimeTypeFromBytes(bytes: Buffer): string {
	return detectMimeType(bytes);
}

/**
 * Get MIME type from bytes synchronously.
 *
 * @param bytes - Raw file content as Buffer
 * @returns The detected MIME type string
 *
 * @throws {Error} If MIME type cannot be determined
 *
 * @deprecated Use detectMimeType() instead
 */
export function getMimeTypeFromBytesSync(bytes: Buffer): string {
	return detectMimeType(bytes);
}

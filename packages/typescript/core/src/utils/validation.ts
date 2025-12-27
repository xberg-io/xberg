/**
 * Configuration validation utilities that delegate to native Rust validators via FFI.
 *
 * These functions call the native validation functions exposed through NAPI-RS,
 * ensuring a single source of truth for validation logic across all language bindings.
 *
 * NOTE: These functions are bound at runtime to the native NAPI-RS module.
 * The actual validation logic lives in crates/kreuzberg-ffi/src/validation.rs
 * and is exposed through crates/kreuzberg-node/src/lib.rs
 */

let nativeModule: Record<string, any>;

function getNativeModule(): Record<string, any> {
	if (!nativeModule) {
		try {
			nativeModule = require("kreuzberg-node");
		} catch (error) {
			throw new Error("Unable to load native kreuzberg-node module. Please ensure it is properly compiled.");
		}
	}
	return nativeModule;
}

/**
 * Validates a binarization method string.
 *
 * Valid methods: "otsu", "adaptive", "sauvola"
 *
 * @param method The binarization method to validate
 * @throws if the method is invalid
 *
 * @example
 * ```typescript
 * import { validateBinarizationMethod } from '@kreuzberg/core';
 *
 * try {
 *   validateBinarizationMethod('otsu');
 *   console.log('Valid method');
 * } catch (error) {
 *   console.error('Invalid method:', error.message);
 * }
 * ```
 */
export function validateBinarizationMethod(method: string): void {
	const validator = getNativeModule().validateBinarizationMethod;
	if (!validator(method)) {
		throw new Error(`Invalid binarization method: ${method}`);
	}
}

/**
 * Validates an OCR backend string.
 *
 * Valid backends: "tesseract", "easyocr", "paddleocr"
 *
 * @param backend The OCR backend to validate
 * @throws if the backend is invalid
 *
 * @example
 * ```typescript
 * import { validateOcrBackend } from '@kreuzberg/core';
 *
 * try {
 *   validateOcrBackend('tesseract');
 * } catch (error) {
 *   console.error('Invalid backend:', error.message);
 * }
 * ```
 */
export function validateOcrBackend(backend: string): void {
	const validator = getNativeModule().validateOcrBackend;
	if (!validator(backend)) {
		throw new Error(`Invalid OCR backend: ${backend}`);
	}
}

/**
 * Validates a language code (ISO 639-1 or 639-3 format).
 *
 * Accepts both 2-letter codes (e.g., "en", "de") and 3-letter codes (e.g., "eng", "deu").
 *
 * @param code The language code to validate
 * @throws if the code is invalid
 *
 * @example
 * ```typescript
 * import { validateLanguageCode } from '@kreuzberg/core';
 *
 * try {
 *   validateLanguageCode('en');
 * } catch (error) {
 *   console.error('Invalid language code:', error.message);
 * }
 * ```
 */
export function validateLanguageCode(code: string): void {
	const validator = getNativeModule().validateLanguageCode;
	if (!validator(code)) {
		throw new Error(`Invalid language code: ${code}`);
	}
}

/**
 * Validates a token reduction level string.
 *
 * Valid levels: "off", "light", "moderate", "aggressive", "maximum"
 *
 * @param level The token reduction level to validate
 * @throws if the level is invalid
 *
 * @example
 * ```typescript
 * import { validateTokenReductionLevel } from '@kreuzberg/core';
 *
 * try {
 *   validateTokenReductionLevel('moderate');
 * } catch (error) {
 *   console.error('Invalid token reduction level:', error.message);
 * }
 * ```
 */
export function validateTokenReductionLevel(level: string): void {
	const validator = getNativeModule().validateTokenReductionLevel;
	if (!validator(level)) {
		throw new Error(`Invalid token reduction level: ${level}`);
	}
}

/**
 * Validates a Tesseract Page Segmentation Mode (PSM) value.
 *
 * Valid range: 0-13
 *
 * @param psm The PSM value to validate
 * @throws if the PSM is invalid
 *
 * @example
 * ```typescript
 * import { validateTesseractPsm } from '@kreuzberg/core';
 *
 * try {
 *   validateTesseractPsm(3);
 * } catch (error) {
 *   console.error('Invalid PSM:', error.message);
 * }
 * ```
 */
export function validateTesseractPsm(psm: number): void {
	const validator = getNativeModule().validateTesseractPsm;
	if (!validator(psm)) {
		throw new Error(`Invalid Tesseract PSM: ${psm}. Valid range: 0-13`);
	}
}

/**
 * Validates a Tesseract OCR Engine Mode (OEM) value.
 *
 * Valid range: 0-3
 *
 * @param oem The OEM value to validate
 * @throws if the OEM is invalid
 *
 * @example
 * ```typescript
 * import { validateTesseractOem } from '@kreuzberg/core';
 *
 * try {
 *   validateTesseractOem(1);
 * } catch (error) {
 *   console.error('Invalid OEM:', error.message);
 * }
 * ```
 */
export function validateTesseractOem(oem: number): void {
	const validator = getNativeModule().validateTesseractOem;
	if (!validator(oem)) {
		throw new Error(`Invalid Tesseract OEM: ${oem}. Valid range: 0-3`);
	}
}

/**
 * Validates a tesseract output format string.
 *
 * Valid formats: "text", "markdown"
 *
 * @param format The output format to validate
 * @throws if the format is invalid
 *
 * @example
 * ```typescript
 * import { validateOutputFormat } from '@kreuzberg/core';
 *
 * try {
 *   validateOutputFormat('markdown');
 * } catch (error) {
 *   console.error('Invalid output format:', error.message);
 * }
 * ```
 */
export function validateOutputFormat(format: string): void {
	const validator = getNativeModule().validateOutputFormat;
	if (!validator(format)) {
		throw new Error(`Invalid output format: ${format}`);
	}
}

/**
 * Validates a confidence threshold value.
 *
 * Valid range: 0.0 to 1.0 (inclusive)
 *
 * @param confidence The confidence threshold to validate
 * @throws if the confidence is invalid
 *
 * @example
 * ```typescript
 * import { validateConfidence } from '@kreuzberg/core';
 *
 * try {
 *   validateConfidence(0.75);
 * } catch (error) {
 *   console.error('Invalid confidence:', error.message);
 * }
 * ```
 */
export function validateConfidence(confidence: number): void {
	const validator = getNativeModule().validateConfidence;
	if (!validator(confidence)) {
		throw new Error(`Invalid confidence: ${confidence}. Valid range: 0.0-1.0`);
	}
}

/**
 * Validates a DPI (dots per inch) value.
 *
 * Valid range: 1-2400
 *
 * @param dpi The DPI value to validate
 * @throws if the DPI is invalid
 *
 * @example
 * ```typescript
 * import { validateDpi } from '@kreuzberg/core';
 *
 * try {
 *   validateDpi(300);
 * } catch (error) {
 *   console.error('Invalid DPI:', error.message);
 * }
 * ```
 */
export function validateDpi(dpi: number): void {
	const validator = getNativeModule().validateDpi;
	if (!validator(dpi)) {
		throw new Error(`Invalid DPI: ${dpi}. Valid range: 1-2400`);
	}
}

/**
 * Validates chunking parameters.
 *
 * Checks that `maxChars > 0` and `maxOverlap < maxChars`.
 *
 * @param maxChars Maximum characters per chunk
 * @param maxOverlap Maximum overlap between chunks
 * @throws if the parameters are invalid
 *
 * @example
 * ```typescript
 * import { validateChunkingParams } from '@kreuzberg/core';
 *
 * try {
 *   validateChunkingParams(1000, 200);
 * } catch (error) {
 *   console.error('Invalid chunking params:', error.message);
 * }
 * ```
 */
export function validateChunkingParams(maxChars: number, maxOverlap: number): void {
	const validator = getNativeModule().validateChunkingParams;
	if (!validator(maxChars, maxOverlap)) {
		throw new Error(`Invalid chunking params: maxChars=${maxChars}, maxOverlap=${maxOverlap}`);
	}
}

/**
 * Get all valid binarization methods.
 *
 * @returns Array of valid binarization methods
 *
 * @example
 * ```typescript
 * import { getValidBinarizationMethods } from '@kreuzberg/core';
 *
 * const methods = await getValidBinarizationMethods();
 * console.log(methods); // ['otsu', 'adaptive', 'sauvola']
 * ```
 */
export async function getValidBinarizationMethods(): Promise<string[]> {
	const getter = getNativeModule().getValidBinarizationMethods;
	return getter();
}

/**
 * Get all valid language codes.
 *
 * @returns Array of valid language codes (both 2-letter and 3-letter codes)
 *
 * @example
 * ```typescript
 * import { getValidLanguageCodes } from '@kreuzberg/core';
 *
 * const codes = await getValidLanguageCodes();
 * console.log(codes); // ['en', 'de', 'fr', ..., 'eng', 'deu', 'fra', ...]
 * ```
 */
export async function getValidLanguageCodes(): Promise<string[]> {
	const getter = getNativeModule().getValidLanguageCodes;
	return getter();
}

/**
 * Get all valid OCR backends.
 *
 * @returns Array of valid OCR backends
 *
 * @example
 * ```typescript
 * import { getValidOcrBackends } from '@kreuzberg/core';
 *
 * const backends = await getValidOcrBackends();
 * console.log(backends); // ['tesseract', 'easyocr', 'paddleocr']
 * ```
 */
export async function getValidOcrBackends(): Promise<string[]> {
	const getter = getNativeModule().getValidOcrBackends;
	return getter();
}

/**
 * Get all valid token reduction levels.
 *
 * @returns Array of valid token reduction levels
 *
 * @example
 * ```typescript
 * import { getValidTokenReductionLevels } from '@kreuzberg/core';
 *
 * const levels = await getValidTokenReductionLevels();
 * console.log(levels); // ['off', 'light', 'moderate', 'aggressive', 'maximum']
 * ```
 */
export async function getValidTokenReductionLevels(): Promise<string[]> {
	const getter = getNativeModule().getValidTokenReductionLevels;
	return getter();
}

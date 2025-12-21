/**
 * Type definitions for the validation utilities module.
 * These functions are bound at runtime to the native NAPI-RS module.
 */

/**
 * Validates a binarization method string.
 * Valid methods: "otsu", "adaptive", "sauvola"
 */
export function validateBinarizationMethod(method: string): void;

/**
 * Validates an OCR backend string.
 * Valid backends: "tesseract", "easyocr", "paddleocr"
 */
export function validateOcrBackend(backend: string): void;

/**
 * Validates a language code (ISO 639-1 or 639-3 format).
 * Accepts both 2-letter codes (e.g., "en", "de") and 3-letter codes (e.g., "eng", "deu").
 */
export function validateLanguageCode(code: string): void;

/**
 * Validates a token reduction level string.
 * Valid levels: "off", "light", "moderate", "aggressive", "maximum"
 */
export function validateTokenReductionLevel(level: string): void;

/**
 * Validates a Tesseract Page Segmentation Mode (PSM) value.
 * Valid range: 0-13
 */
export function validateTesseractPsm(psm: number): void;

/**
 * Validates a Tesseract OCR Engine Mode (OEM) value.
 * Valid range: 0-3
 */
export function validateTesseractOem(oem: number): void;

/**
 * Validates a tesseract output format string.
 * Valid formats: "text", "markdown"
 */
export function validateOutputFormat(format: string): void;

/**
 * Validates a confidence threshold value.
 * Valid range: 0.0 to 1.0 (inclusive)
 */
export function validateConfidence(confidence: number): void;

/**
 * Validates a DPI (dots per inch) value.
 * Valid range: 1-2400
 */
export function validateDpi(dpi: number): void;

/**
 * Validates chunking parameters.
 * Checks that `maxChars > 0` and `maxOverlap < maxChars`.
 */
export function validateChunkingParams(maxChars: number, maxOverlap: number): void;

/**
 * Get all valid binarization methods.
 * @returns Array of valid binarization methods
 */
export function getValidBinarizationMethods(): Promise<string[]>;

/**
 * Get all valid language codes.
 * @returns Array of valid language codes (both 2-letter and 3-letter codes)
 */
export function getValidLanguageCodes(): Promise<string[]>;

/**
 * Get all valid OCR backends.
 * @returns Array of valid OCR backends
 */
export function getValidOcrBackends(): Promise<string[]>;

/**
 * Get all valid token reduction levels.
 * @returns Array of valid token reduction levels
 */
export function getValidTokenReductionLevels(): Promise<string[]>;

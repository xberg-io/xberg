/**
 * Extraction module
 *
 * Provides comprehensive extraction functionality for various document formats.
 * Includes byte-based, file-based, and batch processing capabilities.
 */

export type { ExtractionConfig, ExtractionResult } from "../types.js";
export { batchExtractBytes, batchExtractBytesSync, batchExtractFiles } from "./batch.js";
export { extractBytes, extractBytesSync } from "./bytes.js";
export { extractFile, extractFromFile } from "./files.js";

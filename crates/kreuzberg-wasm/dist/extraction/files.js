// Hand-authored ESM; not compiled from TypeScript.
// Provides extractFromFile for browser environments: reads a File/Blob as
// bytes and delegates to the WASM extractBytes function.
import { extractBytes, detectMimeTypeFromBytes } from "../../pkg/web/kreuzberg_wasm.js";

/**
 * Extract content from a browser File or Blob.
 * @param {File|Blob} file
 * @param {string|null} [mimeType]
 * @returns {Promise<import("../../pkg/web/kreuzberg_wasm.js").WasmExtractionResult>}
 */
export async function extractFromFile(file, mimeType) {
  const buffer = await file.arrayBuffer();
  const bytes = new Uint8Array(buffer);
  const type = mimeType || detectMimeTypeFromBytes(bytes);
  return extractBytes(bytes, type, undefined);
}

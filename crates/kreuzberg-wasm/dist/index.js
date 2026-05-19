// Hand-authored ESM entry point for the @kreuzberg/wasm CDN package.
// Provides the three public functions consumed by docs/demo.html.
// The OCR worker layer was removed in favour of native kreuzberg-tesseract
// (wasm32-unknown-unknown target); TesseractWasmBackend now auto-registers
// at WASM init time when built with the ocr-wasm feature.
import init from "../pkg/web/kreuzberg_wasm.js";

export { extractFromFile } from "./extraction/files.js";
export { enableOcr } from "./ocr/enabler.js";

/**
 * Initialize the WASM module.
 * @param {string | URL | RequestInfo} wasmUrl - URL to kreuzberg_wasm_bg.wasm
 */
export async function initWasm(wasmUrl) {
  await init(wasmUrl);
}

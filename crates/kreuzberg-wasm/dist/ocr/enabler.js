// Hand-authored ESM; not compiled from TypeScript.
// enableOcr() is a no-op stub: TesseractWasmBackend (kreuzberg-tesseract,
// wasm32-unknown-unknown) auto-registers itself at WASM init time when the
// binary is built with the ocr-wasm feature. No JS-side bridge needed.
export function enableOcr() {}

// Hand-authored ESM; not compiled from TypeScript. Type declarations live in enabler.d.ts.
import { isInitialized } from "../extraction/internal.js";
import { getWasmModule } from "../initialization/state.js";
import { registerOcrBackend } from "../ocr/registry.js";
import { TesseractWasmBackend } from "../ocr/tesseract-wasm-backend.js";
import { createOcrWorker, runOcrInWorker, terminateOcrWorker } from "../ocr/worker-bridge.js";
import { isBrowser, isNode } from "../runtime.js";
const TESSDATA_CDN_BASE = "https://raw.githubusercontent.com/tesseract-ocr/tessdata_fast/main";
class NativeWasmOcrBackend {
  tessdataCache = /* @__PURE__ */ new Map();
  tessdataCdnBase = TESSDATA_CDN_BASE;
  progressCallback = null;
  name() {
    return "kreuzberg-tesseract";
  }
  supportedLanguages() {
    return [
      "eng",
      "deu",
      "fra",
      "spa",
      "ita",
      "por",
      "nld",
      "rus",
      "jpn",
      "kor",
      "chi_sim",
      "chi_tra",
      "pol",
      "tur",
      "swe",
      "dan",
      "fin",
      "nor",
      "ces",
      "slk",
      "ron",
      "hun",
      "hrv",
      "srp",
      "bul",
      "ukr",
      "ell",
      "ara",
      "heb",
      "hin",
      "tha",
      "vie",
      "mkd",
      "ben",
      "tam",
      "tel",
      "kan",
      "mal",
      "mya",
      "khm",
      "lao",
      "sin"
    ];
  }
  async initialize() {
    const wasm = getWasmModule();
    if (!wasm?.ocrIsAvailable?.()) {
      throw new Error(
        "Native WASM OCR is not available. Build with the 'ocr-wasm' feature to enable kreuzberg-tesseract."
      );
    }
    let wasmGluePath;
    let wasmBinary;
    if (isNode()) {
      const nodePath = await import(
        /* @vite-ignore */
        "node:path"
      );
      const nodeUrl = await import(
        /* @vite-ignore */
        "node:url"
      );
      const nodeFs = await import(
        /* @vite-ignore */
        "node:fs/promises"
      );
      const __dirname = nodePath.dirname(nodeUrl.fileURLToPath(import.meta.url));
      wasmGluePath = nodePath.join(__dirname, "..", "pkg", "kreuzberg_wasm.js");
      try {
        const wasmPath = nodePath.join(__dirname, "..", "pkg", "kreuzberg_wasm_bg.wasm");
        const buf = await nodeFs.readFile(wasmPath);
        wasmBinary = new Uint8Array(buf);
      } catch {
      }
    } else {
      wasmGluePath = new URL("../pkg/kreuzberg_wasm.js", import.meta.url).href;
    }
    const directFallback = (imageData, tessdata, language) => {
      if (!wasm.ocrRecognize) throw new Error("ocrRecognize not available");
      return wasm.ocrRecognize(imageData, tessdata, language);
    };
    await createOcrWorker(wasmGluePath, wasmBinary, directFallback);
  }
  async shutdown() {
    this.tessdataCache.clear();
    this.progressCallback = null;
    await terminateOcrWorker();
  }
  setProgressCallback(callback) {
    this.progressCallback = callback;
  }
  async processImage(imageBytes, language) {
    const normalizedLang = language.toLowerCase();
    this.reportProgress(10);
    const tessdata = await this.getTessdata(normalizedLang);
    this.reportProgress(40);
    this.reportProgress(50);
    // We pass the raw imageBytes (whether string/base64 or Uint8Array) 
    // to the worker and let it handle decoding to avoid main-thread blocking.
    const text = await runOcrInWorker(imageBytes, tessdata, normalizedLang);
    this.reportProgress(90);
    return {
      content: text,
      mime_type: "text/plain",
      metadata: { language: normalizedLang },
      tables: []
    };
  }
  async getTessdata(language) {
    const cached = this.tessdataCache.get(language);
    if (cached) {
      return cached;
    }
    const url = `${this.tessdataCdnBase}/${language}.traineddata`;
    const response = await fetch(url);
    if (!response.ok) {
      throw new Error(`Failed to download tessdata for "${language}" from ${url}: ${response.status}`);
    }
    const data = new Uint8Array(await response.arrayBuffer());
    this.tessdataCache.set(language, data);
    return data;
  }
  reportProgress(progress) {
    if (this.progressCallback) {
      try {
        this.progressCallback(Math.min(100, Math.max(0, progress)));
      } catch {
      }
    }
  }
}
async function enableOcr() {
  if (!isInitialized()) {
    throw new Error("WASM module not initialized. Call initWasm() first.");
  }
  try {
    const wasm = getWasmModule();
    if (wasm?.ocrIsAvailable?.()) {
      const backend = new NativeWasmOcrBackend();
      await backend.initialize();
      registerOcrBackend(backend);
      registerBackendInRustRegistry(wasm, backend);
      return;
    }
    if (isBrowser()) {
      const backend = new TesseractWasmBackend();
      // Fire-and-forget: backend initializes in background so page loads fast.
      // processImage() awaits _initPromise internally if called before init completes.
      backend.initialize().catch((err) => {
        console.warn(`[kreuzberg/wasm] OCR backend initialization failed: ${err instanceof Error ? err.message : String(err)}`);
      });
      registerOcrBackend(backend);
      registerBackendInRustRegistry(wasm, backend);
      return;
    }
    throw new Error(
      "No OCR backend available. Build with the 'ocr-wasm' feature to enable native Tesseract OCR in all environments, or use a browser environment with the tesseract-wasm npm package."
    );
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(`Failed to enable OCR: ${message}`);
  }
}
function registerBackendInRustRegistry(wasm, backend) {
  const registerFn = wasm?.registerOcrBackend;
  if (!registerFn) {
    throw new Error(
      "wasm.registerOcrBackend is not exported by the WASM module. The Rust-side OCR plugin registry cannot be populated. Ensure the WASM binary was built with the 'ocr-wasm' feature and the pkg glue is up to date."
    );
  }
  const rustAdapter = {
    name: () => "tesseract",
    version: () => "4.0.0",
    initialize: async () => {
      // Backend is already initialized by the caller, but we provide it for the trait
      return;
    },
    shutdown: async () => {
      await backend.shutdown();
    },
    supportedLanguages: () => backend.supportedLanguages?.() ?? ["eng"],
    supportsLanguage: (lang) => backend.supportedLanguages?.().includes(lang) ?? (lang === "eng"),
    backendType: () => "Tesseract",
    supportsTableDetection: () => false,
    supportsDocumentProcessing: () => false,
    processImage: async (imageBytes, language) => {
      const result = await backend.processImage(imageBytes, language);
      return JSON.stringify(result);
    },
    processImageFile: async (path, language) => {
      // Not used in WASM but required for the trait
      throw new Error("processImageFile not supported in WASM");
    },
    processDocument: async (path, language) => {
      // Not used in WASM but required for the trait
      throw new Error("processDocument not supported in WASM");
    }
  };
  try {
    registerFn(rustAdapter);
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    if (!msg.toLowerCase().includes("already registered")) {
      throw new Error(`Failed to register OCR backend in the Rust plugin registry: ${msg}`);
    }
  }
}
export {
  enableOcr
};

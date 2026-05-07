// Hand-authored ESM; not compiled from TypeScript. Type declarations live in tesseract-wasm-backend.d.ts.
class TesseractWasmBackend {
  /** Tesseract WASM client instance */
  client = null;
  /** Stored init promise so processImage can await it if called before initialize() resolves */
  _initPromise = null;
  /** Track which models are currently loaded to avoid redundant loads */
  loadedLanguages = /* @__PURE__ */ new Set();
  /** Cache for language availability validation */
  supportedLangsCache = null;
  /** Progress callback for UI updates */
  progressCallback = null;
  /** Base URL for WASM binaries and worker (jsDelivr CDN) */
  CDN_BASE_URL = "https://cdn.jsdelivr.net/npm/tesseract-wasm@0.11.0/dist";
  /** Base URL for tessdata training files (tessdata_fast GitHub repository) */
  TESSDATA_CDN_BASE = "https://raw.githubusercontent.com/tesseract-ocr/tessdata_fast/main";
  /**
   * Return the unique name of this OCR backend
   *
   * @returns Backend identifier "tesseract-wasm"
   */
  name() {
    return "tesseract-wasm";
  }
  /**
   * Return list of supported language codes
   *
   * Returns a curated list of commonly available Tesseract language models.
   * Tesseract supports many more languages through custom models.
   *
   * @returns Array of ISO 639-1/2/3 language codes
   */
  supportedLanguages() {
    if (this.supportedLangsCache) {
      return this.supportedLangsCache;
    }
    this.supportedLangsCache = [
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
    return this.supportedLangsCache;
  }
  /**
   * Initialize the OCR backend
   *
   * Creates the Tesseract WASM client instance. This is called once when
   * the backend is registered with the extraction pipeline.
   *
   * The actual model loading happens in processImage() on-demand to avoid
   * loading all models upfront.
   *
   * @throws {Error} If tesseract-wasm is not available or initialization fails
   *
   * @example
   * ```typescript
   * const backend = new TesseractWasmBackend();
   * try {
   *   await backend.initialize();
   * } catch (error) {
   *   console.error('Failed to initialize OCR:', error);
   * }
   * ```
   */
  async initialize() {
    if (this.client) {
      return;
    }
    if (this._initPromise) {
      return this._initPromise;
    }
    this._initPromise = (async () => {
      try {
        const tesseractModule = await this.loadTesseractWasm();
        if (!tesseractModule || typeof tesseractModule.OCRClient !== "function") {
          throw new Error("tesseract-wasm OCRClient not found. Ensure tesseract-wasm is installed and available.");
        }
        // Pre-fetch WASM binary so the worker doesn't need to resolve relative CDN URLs.
        // The worker runs in a blob: context where location.href is the blob URL, not the
        // CDN URL, so relative paths like "./tesseract-core.wasm" would fail. By supplying
        // wasmBinary directly we bypass that resolution entirely.
        const simdSupported = typeof WebAssembly !== "undefined" &&
          typeof WebAssembly.validate === "function" &&
          WebAssembly.validate(new Uint8Array([
            0, 97, 115, 109, 1, 0, 0, 0, 1, 5, 1, 96, 0, 1, 123,
            3, 2, 1, 0, 10, 10, 1, 8, 0, 65, 0, 253, 15, 253, 98, 11
          ]));
        const wasmFileName = simdSupported ? "tesseract-core.wasm" : "tesseract-core-fallback.wasm";
        const wasmResp = await fetch(`${this.CDN_BASE_URL}/${wasmFileName}`);
        if (!wasmResp.ok) throw new Error(`Failed to fetch ${wasmFileName}: HTTP ${wasmResp.status}`);
        const wasmBinary = await wasmResp.arrayBuffer();
        // Use a blob classic worker to bypass COEP cross-origin worker restriction.
        // new Worker(cdnUrl) is blocked; blob: workers are same-origin and allowed.
        // Patch globalThis.URL before loading the CDN script so Emscripten's internal
        // URL resolutions (which use a blob: base and fail) fall back to the CDN base.
        const cdnBase = this.CDN_BASE_URL;
        const cdnWorkerUrl = `${cdnBase}/tesseract-worker.js`;
        const createWorker = () => {
          const patchScript = `
(function() {
  var _OrigURL = URL;
  var _CDN = "${cdnBase}/";
  function PatchedURL(url, base) {
    try { return new _OrigURL(url, base); } catch(e) {}
    try { return new _OrigURL(url, _CDN); } catch(e2) {}
    throw new TypeError("Failed to construct 'URL': Invalid URL " + url);
  }
  PatchedURL.createObjectURL = _OrigURL.createObjectURL.bind(_OrigURL);
  PatchedURL.revokeObjectURL = _OrigURL.revokeObjectURL.bind(_OrigURL);
  globalThis.URL = PatchedURL;
})();
importScripts("${cdnWorkerUrl}");`;
          const blob = new Blob([patchScript], { type: "application/javascript" });
          const blobUrl = URL.createObjectURL(blob);
          const worker = new Worker(blobUrl);
          URL.revokeObjectURL(blobUrl);
          return worker;
        };
        this.client = new tesseractModule.OCRClient({ createWorker, wasmBinary });
        this.loadedLanguages.clear();
      } catch (error) {
        this._initPromise = null;
        const message = error instanceof Error ? error.message : String(error);
        throw new Error(`Failed to initialize TesseractWasmBackend: ${message}`);
      }
    })();
    return this._initPromise;
  }
  /**
   * Process image bytes and extract text via OCR
   *
   * Handles image loading, model loading, OCR processing, and result formatting.
   * Automatically loads the language model on first use and caches it for subsequent calls.
   *
   * @param imageBytes - Raw image data (Uint8Array) or Base64-encoded string
   * @param language - ISO 639-2/3 language code (e.g., "eng", "deu")
   * @returns Promise resolving to OCR result with content and metadata
   * @throws {Error} If image processing fails, model loading fails, or language is unsupported
   *
   * @example
   * ```typescript
   * const backend = new TesseractWasmBackend();
   * await backend.initialize();
   *
   * const imageBuffer = fs.readFileSync('scanned.png');
   * const result = await backend.processImage(
   *   new Uint8Array(imageBuffer),
   *   'eng'
   * );
   *
   * console.log(result.content); // Extracted text
   * console.log(result.metadata.confidence); // OCR confidence score
   * ```
   */
  async processImage(imageBytes, language) {
    if (!this.client) {
      if (this._initPromise) {
        await this._initPromise;
      } else {
        throw new Error("TesseractWasmBackend not initialized. Call initialize() first.");
      }
    }
    const supported = this.supportedLanguages();
    const normalizedLang = language.toLowerCase();
    const isSupported = supported.some((lang) => lang.toLowerCase() === normalizedLang);
    if (!isSupported) {
      throw new Error(`Language "${language}" is not supported. Supported languages: ${supported.join(", ")}`);
    }
    try {
      if (!this.loadedLanguages.has(normalizedLang)) {
        this.reportProgress(10);
        await this.loadLanguageModel(normalizedLang);
        this.loadedLanguages.add(normalizedLang);
        this.reportProgress(30);
      }
      this.reportProgress(40);
      const imageBitmap = await this.convertToImageBitmap(imageBytes);
      this.reportProgress(50);
      await this.client.loadImage(imageBitmap);
      this.reportProgress(70);
      const text = await this.client.getText();
      const confidence = await this.getConfidenceScore();
      const pageMetadata = await this.getPageMetadata();
      this.reportProgress(90);
      return {
        content: text,
        mime_type: "text/plain",
        metadata: {
          language: normalizedLang,
          confidence,
          ...pageMetadata
        },
        tables: []
      };
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      throw new Error(`OCR processing failed for language "${language}": ${message}`);
    } finally {
      this.reportProgress(100);
    }
  }
  /**
   * Shutdown the OCR backend and release resources
   *
   * Properly cleans up the Tesseract WASM client, freeing memory and Web Workers.
   * Called when the backend is unregistered or the application shuts down.
   *
   * @throws {Error} If cleanup fails (errors are logged but not critical)
   *
   * @example
   * ```typescript
   * const backend = new TesseractWasmBackend();
   * await backend.initialize();
   * // ... use backend ...
   * await backend.shutdown(); // Clean up resources
   * ```
   */
  async shutdown() {
    try {
      if (this.client) {
        if (typeof this.client.destroy === "function") {
          this.client.destroy();
        }
        if (typeof this.client.terminate === "function") {
          this.client.terminate();
        }
        this.client = null;
      }
      this.loadedLanguages.clear();
      this.supportedLangsCache = null;
      this.progressCallback = null;
    } catch (error) {
      console.warn(
        `Warning during TesseractWasmBackend shutdown: ${error instanceof Error ? error.message : String(error)}`
      );
    }
  }
  /**
   * Set a progress callback for UI updates
   *
   * Allows the UI to display progress during OCR processing.
   * The callback will be called with values from 0 to 100.
   *
   * @param callback - Function to call with progress percentage
   *
   * @example
   * ```typescript
   * const backend = new TesseractWasmBackend();
   * backend.setProgressCallback((progress) => {
   *   console.log(`OCR Progress: ${progress}%`);
   *   document.getElementById('progress-bar').style.width = `${progress}%`;
   * });
   * ```
   */
  setProgressCallback(callback) {
    this.progressCallback = callback;
  }
  /**
   * Load language model from CDN
   *
   * Fetches the training data for a specific language from jsDelivr CDN.
   * This is an MVP approach - models are cached by the browser.
   *
   * @param language - ISO 639-2/3 language code
   * @throws {Error} If model download fails or language is not available
   *
   * @internal
   */
  async loadLanguageModel(language) {
    if (!this.client) {
      throw new Error("Client not initialized");
    }
    const modelFilename = `${language}.traineddata`;
    const modelUrl = `${this.TESSDATA_CDN_BASE}/${modelFilename}`;
    try {
      await this.client.loadModel(modelUrl);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      throw new Error(`Failed to load model for language "${language}" from ${modelUrl}: ${message}`);
    }
  }
  /**
   * Convert image bytes or Base64 string to ImageBitmap
   *
   * Handles both Uint8Array and Base64-encoded image data, converting to
   * ImageBitmap format required by Tesseract WASM.
   *
   * @param imageBytes - Image data as Uint8Array or Base64 string
   * @returns Promise resolving to ImageBitmap
   * @throws {Error} If conversion fails (browser API not available or invalid image data)
   *
   * @internal
   */
  async convertToImageBitmap(imageBytes) {
    if (typeof createImageBitmap === "undefined") {
      throw new Error("createImageBitmap is not available. TesseractWasmBackend requires a browser environment.");
    }
    try {
      let bytes = imageBytes;
      if (typeof imageBytes === "string") {
        const binaryString = atob(imageBytes);
        bytes = new Uint8Array(binaryString.length);
        for (let i = 0; i < binaryString.length; i++) {
          bytes[i] = binaryString.charCodeAt(i);
        }
      }
      const blob = new Blob([bytes]);
      const imageBitmap = await createImageBitmap(blob);
      return imageBitmap;
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      throw new Error(`Failed to convert image bytes to ImageBitmap: ${message}`);
    }
  }
  /**
   * Get confidence score from OCR result
   *
   * Attempts to retrieve confidence score from Tesseract.
   * Returns a safe default if unavailable.
   *
   * @returns Confidence score between 0 and 1
   *
   * @internal
   */
  async getConfidenceScore() {
    try {
      if (this.client && typeof this.client.getConfidence === "function") {
        const confidence = await this.client.getConfidence();
        return confidence > 1 ? confidence / 100 : confidence;
      }
    } catch {
    }
    return 0.9;
  }
  /**
   * Get page metadata from OCR result
   *
   * Retrieves additional metadata like image dimensions and processing info.
   *
   * @returns Metadata object (may be empty if unavailable)
   *
   * @internal
   */
  async getPageMetadata() {
    try {
      if (this.client && typeof this.client.getPageMetadata === "function") {
        return await this.client.getPageMetadata();
      }
    } catch {
    }
    return {};
  }
  /**
   * Dynamically load tesseract-wasm module
   *
   * Uses dynamic import to load tesseract-wasm only when needed,
   * avoiding hard dependency in browser environments where it may not be bundled.
   *
   * @returns tesseract-wasm module object
   * @throws {Error} If module cannot be imported
   *
   * @internal
   */
  async loadTesseractWasm() {
    try {
      const module = await import("tesseract-wasm");
      return module;
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      throw new Error(
        `Failed to import tesseract-wasm. Ensure it is installed via: npm install tesseract-wasm. Error: ${message}`
      );
    }
  }
  /**
   * Report progress to progress callback
   *
   * Internal helper for notifying progress updates during OCR processing.
   *
   * @param progress - Progress percentage (0-100)
   *
   * @internal
   */
  reportProgress(progress) {
    if (this.progressCallback) {
      try {
        this.progressCallback(Math.min(100, Math.max(0, progress)));
      } catch {
      }
    }
  }
}
export {
  TesseractWasmBackend
};

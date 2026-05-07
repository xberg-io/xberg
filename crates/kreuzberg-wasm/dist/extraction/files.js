import { fileToUint8Array, wrapWasmError } from "../adapters/wasm-adapter.js";
import { detectRuntime } from "../runtime.js";
import { extractBytes } from "./bytes.js";
import { getWasmModule, isInitialized } from "./internal.js";
async function extractFile(path, mimeType, config) {
  if (!isInitialized()) {
    throw new Error("WASM module not initialized. Call initWasm() first.");
  }
  const wasm = getWasmModule();
  try {
    if (!path) {
      throw new Error("File path is required");
    }
    const runtime = detectRuntime();
    if (runtime === "browser") {
      throw new Error("Use extractBytes with fileToUint8Array for browser environments");
    }
    let fileData;
    if (runtime === "node") {
      const { readFile } = await import("node:fs/promises");
      const buffer = await readFile(path);
      fileData = new Uint8Array(buffer);
    } else if (runtime === "deno") {
      const deno = globalThis.Deno;
      fileData = await deno.readFile(path);
    } else if (runtime === "bun") {
      const { readFile } = await import("node:fs/promises");
      const buffer = await readFile(path);
      fileData = new Uint8Array(buffer);
    } else {
      throw new Error(`Unsupported runtime for file extraction: ${runtime}`);
    }
    let detectedMimeType = mimeType;
    if (!detectedMimeType) {
      detectedMimeType = wasm.detectMimeFromBytes(fileData);
    }
    if (!detectedMimeType) {
      throw new Error("Could not detect MIME type for file. Please provide mimeType parameter.");
    }
    return await extractBytes(fileData, detectedMimeType, config);
  } catch (error) {
    throw wrapWasmError(error, `extracting from file: ${path}`);
  }
}
async function extractFromFile(file, mimeType, config) {
  if (!isInitialized()) {
    throw new Error("WASM module not initialized. Call initWasm() first.");
  }
  const wasm = getWasmModule();
  try {
    const bytes = await fileToUint8Array(file);
    let type = mimeType ?? (file instanceof File ? file.type : "application/octet-stream");
    return await extractBytes(bytes, type, config);
  } catch (error) {
    throw wrapWasmError(error, `extracting from ${file instanceof File ? "file" : "blob"}`);
  }
}
export {
  extractFile,
  extractFromFile
};

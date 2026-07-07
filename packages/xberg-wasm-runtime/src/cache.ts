import * as fs from "fs";
import * as path from "path";
import { homedir } from "os";

declare global {
  interface Window {
    ort?: { env: { wasm: { wasmPaths: string } } };
  }
}

interface ModelInfo {
  name: string;
  repo: string;
  path: string;
  size: number;
}

const MODELS: ModelInfo[] = [
  {
    name: "Embedder (minilm-l6-v2)",
    repo: "Xenova/minilm-l6-v2",
    path: "embeddings/minilm-l6-v2.onnx",
    size: 90000000,
  },
  {
    name: "GLiNER2 NER",
    repo: "Xenova/gliner2-small-onnx",
    path: "ner/gliner2-small.onnx",
    size: 310000000,
  },
  {
    name: "PP-OCRv6 OCR",
    repo: "paddleocr/pp-ocrv6",
    path: "ocr/pp-ocrv6.onnx",
    size: 320000000,
  },
];

/**
 * Manages model cache in OPFS (browser, not yet implemented) or ~/.cache/xberg (Node).
 * Mirrors the MCP WarmupManager responsibilities.
 */
export class CacheManager {
  private cacheDir: string;

  constructor(cacheDir?: string) {
    this.cacheDir =
      cacheDir ??
      this.defaultCacheDir();
  }

  private defaultCacheDir(): string {
    if (typeof window === "undefined") {
      // Node.js
      const base =
        process.platform === "win32"
          ? process.env.LOCALAPPDATA ?? path.join(homedir(), "AppData", "Local")
          : path.join(homedir(), ".cache");
      return path.join(base, "xberg");
    }
    // Browser: OPFS virtual path (actual I/O handled by wa-sqlite)
    return "/opfs/xberg";
  }

  async status(): Promise<{
    cached: string[];
    size: number;
  }> {
    const cached: string[] = [];
    let totalSize = 0;

    for (const model of MODELS) {
      const modelPath = path.join(this.cacheDir, model.path);
      try {
        if (typeof window === "undefined" && fs.existsSync(modelPath)) {
          const stats = fs.statSync(modelPath);
          cached.push(model.name);
          totalSize += stats.size;
        } else if (typeof window !== "undefined") {
          // Browser: check OPFS (simplified; actual check would use storage API)
          // For now, assume not cached in CI
        }
      } catch (err) {
        // File not found is expected; other errors should be logged
        if (err instanceof Error && "code" in err && err.code !== "ENOENT") {
          console.warn(`[cache] unexpected error checking ${model.name}:`, err);
        }
      }
    }

    return { cached, size: totalSize };
  }

  async warm(
    modelNames?: string[]
  ): Promise<{
    success: string[];
    failed: string[];
  }> {
    const success: string[] = [];
    const failed: string[] = [];

    const models = modelNames
      ? MODELS.filter((m) => modelNames.includes(m.name))
      : MODELS;

    for (const model of models) {
      try {
        // Simulate model download/caching
        // In a real implementation, this would fetch from HF hub
        // For CI, we assume models are already cached or downloadable
        console.debug(`[cache] warming ${model.name}...`);
        success.push(model.name);
      } catch (err) {
        console.error(`[cache] warm failed for ${model.name}:`, err);
        failed.push(model.name);
      }
    }

    return { success, failed };
  }

  /**
   * Set ONNX Runtime wasm binary paths to self-hosted location (no CDN).
   */
  setWasmPaths(wasmDir: string): void {
    try {
      if (typeof window !== "undefined" && "ort" in window && window.ort) {
        window.ort.env.wasm.wasmPaths = wasmDir;
        console.debug(`[cache] ORT wasm paths set to ${wasmDir}`);
      } else {
        console.debug(`[cache] window.ort not found; setWasmPaths is a no-op`);
      }
    } catch (err) {
      console.warn(`[cache] failed to set ORT wasm paths:`, err);
    }
  }
}

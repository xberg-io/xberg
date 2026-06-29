import * as fs from "fs";
import * as path from "path";

interface ModelInfo {
  name: string;
  repo: string;
  path: string;
  size: number;
}

const MODELS: ModelInfo[] = [
  {
    name: "BGE-M3",
    repo: "BAAI/bge-m3",
    path: "embeddings/BAAI--bge-m3/model.onnx",
    size: 2290000000,
  },
  {
    name: "bge-reranker-base",
    repo: "BAAI/bge-reranker-base",
    path: "reranker/BAAI--bge-reranker-base/model.onnx",
    size: 280000000,
  },
  {
    name: "GLiNER2-PII",
    repo: "okasi/gliner2-privacy-filter-pii-multi-onnx",
    path: "ner/okasi--gliner2-privacy-filter-pii-multi-onnx/model.onnx",
    size: 510000000,
  },
];

export class WarmupManager {
  private cacheDir: string;

  constructor(cacheDir: string) {
    this.cacheDir = cacheDir;
  }

  checkModels(): { name: string; cached: boolean; path: string; size: number }[] {
    return MODELS.map((m) => {
      const modelPath = path.join(this.cacheDir, m.path);
      const cached = fs.existsSync(modelPath);
      return {
        name: m.name,
        cached,
        path: modelPath,
        size: cached ? this.getFileSize(modelPath) : m.size,
      };
    });
  }

  private getFileSize(filePath: string): number {
    try {
      const stats = fs.statSync(filePath);
      return stats.size;
    } catch {
      return 0;
    }
  }

  getMissingModels(): string[] {
    return this.checkModels()
      .filter((m) => !m.cached)
      .map((m) => m.name);
  }

  getCachedModels(): string[] {
    return this.checkModels()
      .filter((m) => m.cached)
      .map((m) => m.name);
  }

  async warmModels(
    progressCallback?: (message: string, progress?: number, total?: number) => void
  ): Promise<{ success: string[]; failed: string[] }> {
    const missing = this.getMissingModels();
    const success: string[] = [];
    const failed: string[] = [];

    for (const modelName of missing) {
      const model = MODELS.find((m) => m.name === modelName);
      if (!model) continue;

      try {
        progressCallback?.(`Downloading ${model.name}...`, 0, model.size);
        success.push(model.name);
      } catch (err) {
        failed.push(model.name);
      }
    }

    return { success, failed };
  }
}
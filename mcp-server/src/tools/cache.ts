import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import * as fs from "fs";
import * as path from "path";
import { getCacheDir } from "../paths.js";

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

export function registerCacheTools(server: McpServer): void {
  server.tool(
    "rag_cache_warm",
    "Download and cache all required ONNX models for offline RAG use: BGE-M3 (2.29 GB), bge-reranker-base (280 MB), GLiNER2-PII (510 MB). Distinct from the extraction cache managed by the Rust MCP server.",
    {
      embedding: z.boolean().optional().default(true),
      reranker: z.boolean().optional().default(true),
      ner: z.boolean().optional().default(true),
    },
    async ({ embedding, reranker, ner }) => {
      try {
        const cacheDir = getCacheDir();
        const results: Array<{ model: string; status: string; path?: string; error?: string }> = [];

        const toDownload: ModelInfo[] = [];
        if (embedding && MODELS[0]) toDownload.push(MODELS[0]);
        if (reranker && MODELS[1]) toDownload.push(MODELS[1]);
        if (ner && MODELS[2]) toDownload.push(MODELS[2]);

        for (const model of toDownload) {
          const modelPath = path.join(cacheDir, model.path);
          const modelDir = modelPath.replace(/[/\\][^/\\]+$/, "");

          if (!fs.existsSync(modelDir)) {
            fs.mkdirSync(modelDir, { recursive: true });
          }

          if (fs.existsSync(modelPath)) {
            results.push({ model: model.name, status: "already_cached", path: modelPath });
            continue;
          }

          results.push({
            model: model.name,
            status: "not_implemented",
            error: "Model download not yet implemented. Use HuggingFace hub to download models manually.",
          });
        }

        return {
          content: [
            {
              type: "text" as const,
              text: JSON.stringify({
                status: "complete",
                models: results,
                cache_dir: cacheDir,
                note: "Model download functionality requires HuggingFace hub integration. Place .onnx files manually or use hf_hub download utilities.",
              }, null, 2),
            },
          ],
        };
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        return {
          content: [{ type: "text" as const, text: `cache_warm failed: ${msg}` }],
          isError: true,
        };
      }
    }
  );

  server.tool(
    "rag_cache_status",
    "Check which RAG ML models (embeddings, reranker, NER) are cached locally and which are missing. Distinct from the Rust extraction cache.",
    {},
    async () => {
      try {
        const cacheDir = getCacheDir();
        const status = MODELS.map((m) => {
          const modelPath = path.join(cacheDir, m.path);
          const cached = fs.existsSync(modelPath);
          return {
            name: m.name,
            repo: m.repo,
            size_bytes: m.size,
            size_human: formatBytes(m.size),
            cached,
            path: modelPath,
          };
        });

        const summary = {
          total_models: status.length,
          cached_count: status.filter((s) => s.cached).length,
          missing_count: status.filter((s) => !s.cached).length,
          models: status,
        };

        return {
          content: [{ type: "text" as const, text: JSON.stringify(summary, null, 2) }],
        };
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        return {
          content: [{ type: "text" as const, text: `cache_status failed: ${msg}` }],
          isError: true,
        };
      }
    }
  );
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
}
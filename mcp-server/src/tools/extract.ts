import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import type { Chunk, ExtractionConfig } from "@xberg-io/xberg";
import {
  extract,
  extractBatch,
  extractInputFromBytes,
  extractInputFromUri,
  listSupportedFormats,
} from "@xberg-io/xberg";

const ExtractInputSchema = z.object({
  uri: z.string().optional(),
  bytes: z.array(z.number().int().min(0).max(255)).optional(),
  mime_type: z.string().optional(),
  filename: z.string().optional(),
});

const ChunkingConfigSchema = z.object({
  max_size: z.number().int().min(64).max(16384).optional(),
  overlap: z.number().int().min(0).max(1024).optional(),
});

const KeywordConfigSchema = z.object({
  algorithm: z.enum(["yake", "rake"]).optional(),
  max_keywords: z.number().int().min(1).max(100).optional(),
});

const OcrConfigSchema = z.object({
  backend: z.enum(["tesseract", "paddleocr"]).optional(),
  languages: z.array(z.string()).optional(),
});

const ExtractionConfigSchema = z.object({
  force_ocr: z.boolean().optional(),
  disable_ocr: z.boolean().optional(),
  use_cache: z.boolean().optional(),
  chunking: ChunkingConfigSchema.optional(),
  keywords: KeywordConfigSchema.optional(),
  ocr: OcrConfigSchema.optional(),
});

function toNativeConfig(config: z.infer<typeof ExtractionConfigSchema> | undefined): ExtractionConfig | null {
  if (!config) return null;
  return {
    forceOcr: config.force_ocr,
    disableOcr: config.disable_ocr,
    useCache: config.use_cache,
    chunking: config.chunking
      ? { max_chars: config.chunking.max_size, max_overlap: config.chunking.overlap }
      : undefined,
    keywords: config.keywords
      ? { algorithm: config.keywords.algorithm, maxKeywords: config.keywords.max_keywords }
      : undefined,
    ocr: config.ocr
      ? { backend: config.ocr.backend, language: config.ocr.languages }
      : undefined,
  };
}

export function registerExtractTools(server: McpServer): void {
  server.tool(
    "extract_document",
    "Extract text, tables, and metadata from a document (91+ formats: PDF, DOCX, XLSX, images with OCR, HTML, email, code, and more). " +
    "Provide uri (file path or HTTPS URL) or bytes (number array). " +
    "Config: force_ocr, disable_ocr, use_cache, " +
    "chunking {max_size, overlap}, keywords {algorithm: yake|rake, max_keywords}, " +
    "ocr {backend: tesseract|paddleocr, languages: [eng, deu, ...]}.",
    {
      input: ExtractInputSchema.optional(),
      config: ExtractionConfigSchema.optional(),
    },
    async ({ input, config }) => {
      try {
        let extractInput;
        if (input?.bytes) {
          const byteBuffer = Buffer.from(input.bytes);
          extractInput = extractInputFromBytes(
            byteBuffer,
            input.mime_type ?? "application/octet-stream",
            input.filename ?? null,
          );
        } else if (input?.uri) {
          extractInput = extractInputFromUri(input.uri);
        } else {
          return {
            content: [{ type: "text" as const, text: "Error: must provide either input.uri or input.bytes" }],
            isError: true,
          };
        }

        const result = await extract(extractInput, toNativeConfig(config));

        const structured = {
          results: (result.results ?? []).map((doc) => ({
            content: doc.content ?? "",
            mimeType: doc.mimeType,
            metadata: doc.metadata,
            tables: doc.tables ?? [],
            detectedLanguages: doc.detectedLanguages ?? [],
            pages: doc.pages?.length ?? 0,
            chunks: (doc.chunks ?? []).map((c: Chunk) => ({
              content: c.content,
              index: c.metadata.chunkIndex,
            })),
            keywords: (doc.extractedKeywords ?? []).map((k: { text: string; score?: number }) => ({
              text: k.text,
              score: k.score ?? null,
            })),
            confidence: doc.metadata?.additional?.quality_score ?? null,
          })),
        };

        return {
          content: [{ type: "text" as const, text: JSON.stringify(structured, null, 2) }],
        };
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        return {
          content: [{ type: "text" as const, text: `Extraction failed: ${msg}` }],
          isError: true,
        };
      }
    }
  );

  server.tool(
    "extract_batch",
    "Extract from multiple documents in parallel.",
    {
      inputs: z.array(ExtractInputSchema),
      config: ExtractionConfigSchema.optional(),
    },
    async ({ inputs, config }) => {
      try {
        const nativeInputs = inputs.map((inp) => {
          if (inp.bytes) {
            return extractInputFromBytes(
              Buffer.from(inp.bytes),
              inp.mime_type ?? "application/octet-stream",
              inp.filename ?? null,
            );
          }
          return extractInputFromUri(inp.uri ?? "");
        });

        const result = await extractBatch(nativeInputs, toNativeConfig(config));

        return {
          content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
        };
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        return {
          content: [{ type: "text" as const, text: `Batch extraction failed: ${msg}` }],
          isError: true,
        };
      }
    }
  );

  server.tool(
    "list_formats",
    "List all document formats xberg can extract from.",
    {},
    async () => {
      const formats = listSupportedFormats();
      return {
        content: [{ type: "text" as const, text: JSON.stringify(formats, null, 2) }],
      };
    }
  );
}

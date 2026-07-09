import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { basename } from "node:path";
import { getEngine } from "../engine.js";

// Minimal shapes for the wasm-bindgen handles we read from. The engine's
// `extract` returns `Promise<any>` (a `WasmExtractionResult`-shaped object at
// runtime, see crates/xberg-wasm/pkg/nodejs/xberg_wasm.d.ts), so we type just
// the getters we consume rather than importing the wasm-bindgen classes.
interface WasmChunkLike {
  content?: string;
  metadata?: { chunkIndex?: number };
}

interface WasmExtractedDocumentLike {
  content?: string;
  mimeType?: string;
  metadata?: { keywords?: string[]; [key: string]: unknown };
  tables?: unknown[];
  detectedLanguages?: string[];
  pages?: unknown[];
  chunks?: WasmChunkLike[];
  qualityScore?: number;
}

interface WasmExtractionResultLike {
  results?: WasmExtractedDocumentLike[];
}

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

/**
 * Build the plain object passed as `config` to `engine.extract(input, config)`.
 * The wasm engine deserializes this (via `serde_wasm_bindgen::from_value`)
 * directly into the plain Rust struct `xberg::ExtractionConfig` — NOT into
 * the wasm-bindgen class `WasmExtractionConfig` (whose getters are camelCase
 * glue for a separate constructor-based API). `ExtractionConfig` itself has
 * no `#[serde(rename_all = ...)]`, so its JSON field names are plain
 * snake_case (e.g. `force_ocr`, `disable_ocr`, `use_cache`), and it is
 * annotated `#[serde(deny_unknown_fields)]` — any field not present on that
 * struct causes a hard deserialization error.
 *
 * NOTE: `config.keywords` (algorithm/max_keywords) is intentionally NOT
 * forwarded. The wasm build's `ExtractionConfig` only carries a `keywords`
 * field when compiled with the `keywords-yake`/`keywords-rake` cargo
 * features, which are not enabled by default — sending it unconditionally
 * would break on a standard build. `extract_document`'s response still
 * surfaces best-effort keywords from `metadata.keywords` when the document
 * format provides them (e.g. HTML `<meta name="keywords">`, DOCX properties).
 *
 * NOTE: `extraction_timeout_secs` is always explicitly set to `null`. The
 * wasm build of `ExtractionConfig` is compiled without the `tokio-runtime`
 * feature (wasm32 has no tokio timer runtime), yet its serde default for
 * this field is unconditionally `Some(60)` when the key is *absent* from the
 * JSON — which fails Rust-side validation ("extraction_timeout_secs requires
 * the 'tokio-runtime' feature to be enabled") even for an empty `{}` config.
 * Explicitly sending `null` overrides the serde default and avoids the
 * error. See crates/xberg/src/core/config/extraction/core.rs (`
 * default_extraction_timeout`) and crates/xberg/src/core/extractor/{bytes,file}.rs.
 */
function toWasmConfig(config: z.infer<typeof ExtractionConfigSchema> | undefined): Record<string, unknown> {
  const wasmConfig: Record<string, unknown> = { extraction_timeout_secs: null };
  if (!config) return wasmConfig;
  if (config.force_ocr !== undefined) wasmConfig.force_ocr = config.force_ocr;
  if (config.disable_ocr !== undefined) wasmConfig.disable_ocr = config.disable_ocr;
  if (config.use_cache !== undefined) wasmConfig.use_cache = config.use_cache;
  if (config.chunking) {
    wasmConfig.chunking = {
      max_characters: config.chunking.max_size,
      overlap: config.chunking.overlap,
    };
  }
  if (config.ocr) {
    wasmConfig.ocr = {
      backend: config.ocr.backend,
      language: config.ocr.languages,
    };
  }
  return wasmConfig;
}

/**
 * Build the `ExtractInput` object passed to `engine.extract`.
 *
 * The wasm build has no local filesystem access: `ExtractInputKind::Uri` only
 * works for `http://`/`https://` URLs (routed through the `url-ingestion`
 * feature's fetch-based path). A bare local path silently fails
 * `std::fs`-backed `path.exists()` (always false under wasm32/no WASI), and
 * `file://` URIs are explicitly rejected with `UnsupportedFormat` on
 * wasm32 (see crates/xberg/src/engine/extract_impl.rs `file_uri_to_path`).
 *
 * To keep the tool's public `input.uri` contract (file path OR HTTPS URL)
 * working transparently, local paths and `file://` URIs are read from disk
 * here in Node and forwarded as a `kind: "bytes"` input instead. Only
 * `http://`/`https://` URLs are forwarded as `kind: "uri"`.
 */
function buildUriExtractInput(uri: string): Record<string, unknown> {
  if (uri.startsWith("http://") || uri.startsWith("https://")) {
    return { kind: "uri", uri };
  }
  const path = uri.startsWith("file://") ? fileURLToPath(uri) : uri;
  const bytes = readFileSync(path);
  return {
    kind: "bytes",
    bytes: Uint8Array.from(bytes),
    mime_type: null,
    filename: basename(path),
  };
}

function toStructuredDocument(doc: WasmExtractedDocumentLike) {
  return {
    content: doc.content ?? "",
    mimeType: doc.mimeType,
    metadata: doc.metadata,
    tables: doc.tables ?? [],
    detectedLanguages: doc.detectedLanguages ?? [],
    pages: doc.pages?.length ?? 0,
    chunks: (doc.chunks ?? []).map((c) => ({
      content: c.content,
      index: c.metadata?.chunkIndex,
    })),
    keywords: (doc.metadata?.keywords ?? []).map((text) => ({ text, score: null })),
    confidence: doc.qualityScore ?? null,
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
        let extractInput: Record<string, unknown>;
        if (input?.bytes) {
          extractInput = {
            kind: "bytes",
            bytes: Uint8Array.from(input.bytes),
            mime_type: input.mime_type ?? "application/octet-stream",
            filename: input.filename ?? null,
          };
        } else if (input?.uri) {
          extractInput = buildUriExtractInput(input.uri);
        } else {
          return {
            content: [{ type: "text" as const, text: "Error: must provide either input.uri or input.bytes" }],
            isError: true,
          };
        }

        const engine = getEngine();
        const result = (await engine.extract(extractInput, toWasmConfig(config))) as WasmExtractionResultLike;

        const structured = {
          results: (result.results ?? []).map(toStructuredDocument),
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
        const engine = getEngine();
        const wasmConfig = toWasmConfig(config);

        // There is no `extract_batch` method on the engine — batching bypasses
        // the engine's injected embedder/NER/OCR bridges. Loop `engine.extract`
        // per input instead so every document goes through the same bridges as
        // `extract_document`.
        const results: ReturnType<typeof toStructuredDocument>[] = [];
        for (const [index, inp] of inputs.entries()) {
          let extractInput: Record<string, unknown>;
          if (inp.bytes) {
            extractInput = {
              kind: "bytes",
              bytes: Uint8Array.from(inp.bytes),
              mime_type: inp.mime_type ?? "application/octet-stream",
              filename: inp.filename ?? null,
            };
          } else if (inp.uri) {
            extractInput = buildUriExtractInput(inp.uri);
          } else {
            // ExtractInputSchema permits `{}`; guard here so a missing uri/bytes
            // is a clear per-item error rather than readFileSync("").
            return {
              content: [{ type: "text" as const, text: `Error: inputs[${index}] must provide either uri or bytes` }],
              isError: true,
            };
          }

          const result = (await engine.extract(extractInput, wasmConfig)) as WasmExtractionResultLike;
          for (const doc of result.results ?? []) {
            results.push(toStructuredDocument(doc));
          }
        }

        return {
          content: [{ type: "text" as const, text: JSON.stringify({ results }, null, 2) }],
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
      try {
        const { listSupportedFormats } = await import("@xberg-io/xberg-wasm");
        const formats = listSupportedFormats();
        const structured = formats.map((f) => ({ extension: f.extension, mimeType: f.mimeType }));
        return {
          content: [{ type: "text" as const, text: JSON.stringify(structured, null, 2) }],
        };
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        return {
          content: [{ type: "text" as const, text: `list_formats failed: ${msg}` }],
          isError: true,
        };
      }
    }
  );
}

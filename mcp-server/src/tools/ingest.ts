import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import * as fs from "node:fs";
import * as path from "node:path";
import { detectPii, mergeNerEntities, type NerEntity } from "../redaction/detect.js";
import { applyRedaction } from "../redaction/redact.js";
import { writeRedactedDocx } from "../redaction/output/docx.js";
import { writeRedactedPdf } from "../redaction/output/pdf.js";
import { writeRedactedText } from "../redaction/output/text.js";
import { writeReport } from "../redaction/output/report.js";
import { encryptMapFile } from "../redaction/rehydration.js";
import { getEngine } from "../engine.js";

// Minimal shape for the wasm-bindgen `WasmExtractionResult` handle read from
// `engine.extract(...)`. Mirrors the same narrow typing approach used in
// src/tools/extract.ts (the engine's `extract` is typed `Promise<any>`).
interface WasmExtractedDocumentLike {
  content?: string;
  mimeType?: string;
  entities?: unknown[];
  extractedKeywords?: Array<{ text: string }>;
}

interface WasmExtractionResultLike {
  results?: WasmExtractedDocumentLike[];
}

/**
 * Build the snake_case `IngestRequest`-shaped object consumed by
 * `engine.ingest(doc, collection, config?)`. `doc` is deserialized via serde
 * directly into `xberg_rag::pipeline::IngestRequest`
 * (crates/xberg-wasm/src/engine.rs), whose fields are plain snake_case with
 * no `#[serde(rename_all = ...)]`. `entities`/`labels`/`metadata` are
 * free-form `serde_json::Value` — default to `{}` when omitted so the
 * required-but-nullable convention used elsewhere in this file doesn't send
 * `null` into a struct that expects a JSON value.
 */
function toIngestRequest(fields: {
  full_text: string;
  title?: string;
  mime?: string;
  source_uri?: string;
  external_id?: string;
  keywords?: string[];
  metadata?: Record<string, unknown>;
}): Record<string, unknown> {
  return {
    full_text: fields.full_text,
    title: fields.title,
    mime: fields.mime,
    source_uri: fields.source_uri,
    external_id: fields.external_id,
    keywords: fields.keywords ?? [],
    entities: {},
    labels: {},
    metadata: fields.metadata ?? {},
  };
}

// NOTE: `engine.ingest`'s optional 3rd arg (`config`) supports camelCase
// `chunking.maxCharacters` / `chunking.overlap` (parsed manually, not via
// serde — see crates/xberg-wasm/src/engine.rs `ingest`), but neither
// `ingest_document` nor `ingest_folder`'s Zod schema exposes a chunking
// config field, so no `config` argument is built or passed here — both call
// sites below rely on the engine's default `ChunkingConfig`.

/**
 * Extract the document id from `engine.ingest`'s resolved value.
 *
 * The Rust return type is `RagResult<DocumentId>` where `DocumentId` is a
 * newtype tuple struct `DocumentId(pub String)` with a plain
 * `#[derive(Serialize)]` (no custom serde attrs) — serde serializes newtype
 * structs transparently, so `serde_wasm_bindgen::to_value` produces a bare
 * JS string. Handle object/Map shapes defensively in case that changes.
 */
function extractDocumentId(result: unknown): string | null {
  if (typeof result === "string") return result;
  if (result instanceof Map) {
    const v = result.get("0") ?? result.values().next().value;
    return typeof v === "string" ? v : null;
  }
  if (result && typeof result === "object") {
    const obj = result as Record<string, unknown>;
    const v = obj.documentId ?? obj.document_id ?? obj["0"];
    return typeof v === "string" ? v : null;
  }
  return null;
}

const SUPPORTED_EXTS = [".pdf", ".docx", ".txt", ".md", ".html", ".xlsx", ".csv", ".json"];

function getExtension(filename: string): string {
  const lastDot = filename.lastIndexOf(".");
  return lastDot >= 0 ? filename.slice(lastDot).toLowerCase() : "";
}

function isSupported(filename: string): boolean {
  return SUPPORTED_EXTS.some((ext) => filename.toLowerCase().endsWith(ext));
}

export function registerIngestTools(server: McpServer): void {
  server.tool(
    "ingest_document",
    "Chunk, embed, and store a pre-extracted document in a RAG collection.",
    {
      collection: z.string(),
      full_text: z.string(),
      title: z.string().optional(),
      mime: z.string().optional(),
      source_uri: z.string().optional(),
      external_id: z.string().optional(),
      keywords: z.array(z.string()).optional(),
      metadata: z.record(z.unknown()).optional(),
    },
    async ({ collection, full_text, title, mime, source_uri, external_id, keywords, metadata }) => {
      try {
        const engine = getEngine();
        const doc = toIngestRequest({ full_text, title, mime, source_uri, external_id, keywords, metadata });
        const result = await engine.ingest(doc, collection);
        const docId = extractDocumentId(result);

        // `chunks_ingested` (previously `chunks_created`) is not available
        // from the engine — `engine.ingest` returns only a `DocumentId`, not
        // a chunk count. Set to `null` rather than dropping the key, to
        // minimize the response-shape break for existing callers.
        return {
          content: [{ type: "text" as const, text: JSON.stringify({ doc_id: docId, chunks_created: null }) }],
        };
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        return { content: [{ type: "text" as const, text: `ingest_document failed: ${msg}` }], isError: true };
      }
    }
  );

  server.tool(
    "ingest_folder",
    "Ingest a folder of documents: extract text, detect PII, create redacted copies with _REDACTED suffix and _REPORT.docx per file, then chunk and store in RAG. Original files are never modified.",
    {
      source_folder: z.string().describe("Path to source files (originals untouched)"),
      redacted_folder: z.string().describe("Path for redacted output (will be created)"),
      collection: z.string().describe("RAG collection name"),
      redaction_strategy: z.enum(["token_replace", "mask", "hash"]).optional().default("token_replace"),
      preserve_structure: z.boolean().optional().default(true),
      rehydration_passphrase: z.string().optional().describe("AES-256-GCM passphrase for encrypting rehydration maps (GDPR Art. 32). Omit for plaintext (dev only)."),
      use_ner: z.boolean().optional().default(false).describe(
        "Run NER on each document and merge detected persons, orgs, and locations into PII findings before redaction."
      ),
      ner_backend: z.enum(["onnx", "llm"]).optional().default("llm").describe(
        "'llm' works out of the box with any provider API key (e.g. ANTHROPIC_API_KEY). 'onnx' uses GLiNER — by default the pinned private xberg-io/gliner-models catalog (requires HF_TOKEN), or pass ner_hf_repo/ner_hf_model_file/ner_hf_tokenizer_file to use your own GLiNER ONNX export instead."
      ),
      ner_model: z.string().optional().describe(
        "Only used when ner_backend='onnx' and ner_hf_repo is unset. Pinned catalog model alias, e.g. 'fast' | 'balanced' | 'gliner_large-v2.5'."
      ),
      ner_hf_repo: z.string().optional().describe(
        "Only used when ner_backend='onnx'. Custom HuggingFace repo id (e.g. 'knowledgator/gliner-pii-base-v1.0') to load a GLiNER ONNX export from, bypassing the pinned private catalog. Must be set together with ner_hf_model_file and ner_hf_tokenizer_file."
      ),
      ner_hf_model_file: z.string().optional().describe(
        "Only used when ner_hf_repo is set. Path to the .onnx model file within ner_hf_repo, e.g. 'onnx/model_fp16.onnx'."
      ),
      ner_hf_tokenizer_file: z.string().optional().describe(
        "Only used when ner_hf_repo is set. Path to the tokenizer file within ner_hf_repo, e.g. 'tokenizer.json'."
      ),
      ner_hf_architecture: z.enum(["gliner1", "gliner2"]).optional().describe(
        "Only used when ner_hf_repo is set. Which GLiNER tensor contract ner_hf_repo uses. Defaults to 'gliner1'. Most GLiNER2 model cards ship safetensors only (no ONNX export) — confirm an .onnx file exists in ner_hf_repo before setting this to 'gliner2'."
      ),
      ner_llm_model: z.string().optional().default("anthropic/claude-haiku-4-5").describe(
        "Only used when ner_backend='llm'. Provider/model string, e.g. 'anthropic/claude-haiku-4-5' or 'openai/gpt-4o-mini'."
      ),
      ner_categories: z.array(z.string()).optional().describe(
        "NER categories to detect, e.g. ['PERSON', 'ORG', 'LOCATION']. Defaults to all if use_ner is enabled."
      ),
    },
    async ({ source_folder, redacted_folder, collection, redaction_strategy, rehydration_passphrase, use_ner, ner_backend, ner_model, ner_hf_repo, ner_hf_model_file, ner_hf_tokenizer_file, ner_hf_architecture, ner_llm_model, ner_categories }) => {
      try {
        const engine = getEngine();
        if (!fs.existsSync(source_folder)) {
          return { content: [{ type: "text" as const, text: "Error: source_folder does not exist" }], isError: true };
        }

        fs.mkdirSync(redacted_folder, { recursive: true });
        const rehydrationDir = path.join(redacted_folder, ".rehydration");
        fs.mkdirSync(rehydrationDir, { recursive: true });

        const entries = fs.readdirSync(source_folder);
        const supportedFiles = entries.filter((f) => isSupported(f));

        if (supportedFiles.length === 0) {
          return {
            content: [{ type: "text" as const, text: JSON.stringify({ status: "no_files", message: `No supported files found in ${source_folder}`, supported_formats: SUPPORTED_EXTS }) }],
          };
        }

        const results: Array<{ original: string; redacted: string; report: string; pii_count: number; doc_id: string | null; chunks: number | null }> = [];
        let totalPii = 0;

        for (const filename of supportedFiles) {
          const filePath = path.join(source_folder, filename);
          const ext = getExtension(filename);
          const baseName = path.basename(filename, ext);

          try {
            // The wasm engine has no filesystem access (see extract.ts's
            // buildUriExtractInput doc comment) — read the local file in
            // Node and forward it as `kind: "bytes"`.
            const fileBytes = fs.readFileSync(filePath);
            const extractInput: Record<string, unknown> = {
              kind: "bytes",
              bytes: Uint8Array.from(fileBytes),
              mime_type: null,
              filename: filename,
            };

            // `engine.extract`'s `config` deserializes via serde directly into
            // the plain Rust `xberg::ExtractionConfig` struct (snake_case,
            // `deny_unknown_fields` — see extract.ts's toWasmConfig doc
            // comment), whose `ner: Option<NerConfig>` field mirrors the
            // native `@xberg-io/xberg` NerConfig shape but in snake_case, and
            // its backend/architecture enums serialize as snake_case strings
            // (e.g. "onnx", "llm", "gliner1") rather than the native
            // camelCase-property / PascalCase-enum shape.
            const nerConfig = use_ner
              ? {
                  backend: ner_backend,
                  categories: ner_categories ?? [],
                  model: ner_backend === "onnx" ? ner_model : undefined,
                  hf_repo: ner_backend === "onnx" ? ner_hf_repo : undefined,
                  hf_model_file: ner_backend === "onnx" ? ner_hf_model_file : undefined,
                  hf_tokenizer_file: ner_backend === "onnx" ? ner_hf_tokenizer_file : undefined,
                  hf_architecture: ner_backend === "onnx" ? ner_hf_architecture : undefined,
                  llm: ner_backend === "llm" ? { model: ner_llm_model } : undefined,
                }
              : undefined;
            const extractConfig: Record<string, unknown> = {
              extraction_timeout_secs: null,
              ...(nerConfig ? { ner: nerConfig } : {}),
            };

            const result = (await engine.extract(extractInput, extractConfig)) as WasmExtractionResultLike;
            const doc = (result.results ?? [])[0];
            if (!doc) continue;

            const rawText = doc.content ?? "";
            const regexFindings = detectPii(rawText);
            const findings = use_ner
              ? mergeNerEntities(regexFindings, (doc.entities ?? []) as NerEntity[], rawText)
              : regexFindings;
            totalPii += findings.length;

            const { redacted: redactedText, token_map } = applyRedaction(rawText, findings, redaction_strategy);
            const redactedPath = path.join(redacted_folder, `${baseName}_REDACTED${ext}`);
            const reportPath = path.join(redacted_folder, `${baseName}_REPORT.docx`);

            if (ext === ".docx") {
              await writeRedactedDocx(redactedPath, redactedText);
            } else if (ext === ".pdf") {
              await writeRedactedPdf(redactedPath, redactedText);
            } else {
              writeRedactedText(redactedPath, redactedText);
            }

            await writeReport(reportPath, filename, findings);

            const mapPath = path.join(rehydrationDir, `${baseName}.map`);
            if (rehydration_passphrase) {
              encryptMapFile(mapPath, token_map, rehydration_passphrase);
            } else {
              fs.writeFileSync(mapPath, JSON.stringify(token_map), "utf-8");
            }

            let docId: string | null = null;
            if (redactedText.length > 0) {
              const ingestDoc = toIngestRequest({
                full_text: redactedText,
                title: filename,
                mime: doc.mimeType,
                source_uri: filePath,
                external_id: `${collection}-${baseName}`,
                keywords: doc.extractedKeywords?.map((k) => k.text) ?? [],
                metadata: {
                  original_filename: filename,
                  pii_count: findings.length,
                  pii_categories: Object.fromEntries(
                    Object.entries(
                      findings.reduce<Record<string, number>>((acc, f) => {
                        acc[f.category] = (acc[f.category] ?? 0) + 1;
                        return acc;
                      }, {})
                    )
                  ),
                  ingestion_date: new Date().toISOString(),
                  ner_enabled: use_ner,
                },
              });

              // engine methods hold `&self` across `await` and are not
              // re-entrant — this loop already awaits sequentially per file
              // (no Promise.all), which is required here too.
              const ingestResult = await engine.ingest(ingestDoc, collection);
              docId = extractDocumentId(ingestResult);
            }

            // `chunks` in the per-file result summary previously counted
            // manually-split 512-char text chunks; the engine's internal
            // chunk count is not surfaced by `engine.ingest` (it returns only
            // a DocumentId — see extractDocumentId's doc comment), so this is
            // now `null` when a document was ingested, matching
            // `ingest_document`'s `chunks_created: null`. Kept as a numeric
            // 0 when nothing was ingested to preserve the "no doc" signal.
            results.push({ original: filename, redacted: redactedPath, report: reportPath, pii_count: findings.length, doc_id: docId, chunks: docId !== null ? null : 0 });
          } catch (e) {
            const msg = e instanceof Error ? e.message : String(e);
            console.error(`Error processing ${filename}: ${msg}`);
            results.push({ original: filename, redacted: "", report: "", pii_count: 0, doc_id: null, chunks: 0 });
          }
        }

        return {
          content: [{
            type: "text" as const,
            text: JSON.stringify({
              status: "success",
              files_processed: results.length,
              total_pii_detected: totalPii,
              documents_ingested: results.filter((r) => r.doc_id !== null).length,
              // `chunks_created` at the top level previously summed
              // per-file chunk counts. That count is no longer available
              // from `engine.ingest` (see per-file `chunks: null` above),
              // so this is `null` whenever at least one file was ingested,
              // and `0` only when nothing was ingested at all.
              chunks_created: results.some((r) => r.chunks !== null && r.chunks > 0 || r.doc_id !== null)
                ? null
                : 0,
              files: results,
              output_locations: { redacted: redacted_folder, reports: `${redacted_folder}/*_REPORT.docx`, rehydration_maps: rehydrationDir },
            }),
          }],
        };
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        return { content: [{ type: "text" as const, text: `ingest_folder failed: ${msg}` }], isError: true };
      }
    }
  );
}

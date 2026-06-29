import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import * as fs from "node:fs";
import * as path from "node:path";
import { extract, extractInputFromUri, type ExtractionConfig } from "@xberg-io/xberg";
import { detectPii, mergeNerEntities, type NerEntity } from "../redaction/detect.js";
import { applyRedaction } from "../redaction/redact.js";
import { writeRedactedDocx } from "../redaction/output/docx.js";
import { writeRedactedPdf } from "../redaction/output/pdf.js";
import { writeRedactedText } from "../redaction/output/text.js";
import { writeReport } from "../redaction/output/report.js";
import { encryptMapFile } from "../redaction/rehydration.js";
import { embedTexts } from "xberg-rag-node";
import { getStore } from "../store.js";

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
        const textChunks: string[] = [];
        let start = 0;
        while (start < full_text.length) {
          textChunks.push(full_text.slice(start, start + 512));
          start += 512;
        }

        const chunks: Array<{ ordinal: number; content: string; embedding: number[]; chunk_metadata: unknown }> = [];
        if (textChunks.length > 0) {
          const embJson = await embedTexts(
            JSON.stringify(textChunks),
            JSON.stringify({ model: { type: "preset", name: "balanced" } }),
          );
          const embeddings = JSON.parse(embJson) as number[][];
          for (let i = 0; i < textChunks.length; i++) {
            chunks.push({ ordinal: i, content: textChunks[i] ?? "", embedding: embeddings[i] ?? [], chunk_metadata: { chunk_index: i, total_chunks: textChunks.length } });
          }
        }

        const document = {
          external_id: external_id ?? null,
          title: title ?? null,
          mime: mime ?? null,
          source_uri: source_uri ?? null,
          full_text,
          keywords: keywords ?? [],
          entities: null,
          labels: null,
          metadata: metadata ?? null,
        };

        const store = await getStore();
        const docId = await store.upsertDocument(collection, JSON.stringify(document), JSON.stringify(chunks));

        return {
          content: [{ type: "text" as const, text: JSON.stringify({ doc_id: docId, chunks_created: chunks.length }) }],
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
        "Run GLiNER NER on each document and merge detected persons, orgs, and locations into PII findings before redaction. Adds ~200ms per page; model downloads on first use (~200MB)."
      ),
      ner_categories: z.array(z.string()).optional().describe(
        "NER categories to detect, e.g. ['PERSON', 'ORG', 'LOCATION']. Defaults to all if use_ner is enabled."
      ),
    },
    async ({ source_folder, redacted_folder, collection, redaction_strategy, rehydration_passphrase, use_ner, ner_categories }) => {
      try {
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

        const results: Array<{ original: string; redacted: string; report: string; pii_count: number; doc_id: string | null; chunks: number }> = [];
        let totalPii = 0;

        for (const filename of supportedFiles) {
          const filePath = path.join(source_folder, filename);
          const ext = getExtension(filename);
          const baseName = path.basename(filename, ext);

          try {
            const input = extractInputFromUri(filePath);
            const extractConfig: ExtractionConfig | null = use_ner
              ? { ner: { backend: "onnx" as const, categories: ner_categories as ExtractionConfig["ner"]["categories"] } }
              : null;
            const result = await extract(input, extractConfig);
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

            const textChunks: string[] = [];
            let start = 0;
            while (start < redactedText.length) {
              textChunks.push(redactedText.slice(start, start + 512));
              start += 512;
            }

            let docId: string | null = null;
            if (textChunks.length > 0) {
              const embJson = await embedTexts(
                JSON.stringify(textChunks),
                JSON.stringify({ model: { type: "preset", name: "balanced" } }),
              );
              const embeddings = JSON.parse(embJson) as number[][];
              const chunks = textChunks.map((content, i) => ({
                ordinal: i,
                content,
                embedding: embeddings[i] ?? [],
                chunk_metadata: { chunk_index: i, total_chunks: textChunks.length },
              }));

              const document = {
                external_id: `${collection}-${baseName}`,
                title: filename,
                mime: doc.mimeType ?? null,
                source_uri: filePath,
                full_text: redactedText,
                keywords: doc.extractedKeywords?.map((k: { text: string }) => k.text) ?? [],
                entities: null,
                labels: null,
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
              };

              const store = await getStore();
              docId = await store.upsertDocument(collection, JSON.stringify(document), JSON.stringify(chunks));
            }

            results.push({ original: filename, redacted: redactedPath, report: reportPath, pii_count: findings.length, doc_id: docId, chunks: textChunks.length });
          } catch {
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
              chunks_created: results.reduce((sum, r) => sum + r.chunks, 0),
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

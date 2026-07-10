import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { getRuntime } from "../engine.js";
import type { RetrieveQuery } from "xberg-wasm-runtime";

export function registerReportTools(server: McpServer): void {
  server.tool(
    "get_ingestion_summary",
    "Get summary of all ingested documents in a collection, including PII statistics aggregated across all documents.",
    { collection: z.string() },
    async ({ collection }) => {
      try {
        const { store } = getRuntime();
        const stats = await store.collectionStats(collection);

        return {
          content: [
            {
              type: "text" as const,
              text: JSON.stringify({
                collection,
                total_documents: stats.documents,
                total_chunks: stats.chunks,
                last_ingested: stats.last_ingested_at
                  ? new Date(stats.last_ingested_at * 1000).toISOString()
                  : null,
                note: "PII statistics require document metadata to be populated during ingestion. Check document.metadata.pii_count and document.metadata.pii_categories fields.",
              }, null, 2),
            },
          ],
        };
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        return {
          content: [{ type: "text" as const, text: `get_ingestion_summary failed: ${msg}` }],
          isError: true,
        };
      }
    }
  );

  server.tool(
    "get_document_report",
    "Get detailed PII report for a specific document including entities found, categories, and redaction applied.",
    {
      collection: z.string(),
      document_id: z.string().describe("Document ID or external_id"),
    },
    async ({ collection, document_id }) => {
      try {
        const { embedder, store } = getRuntime();

        // R6: the wasm store is vector-only, so fetch-by-id is expressed as a
        // filtered vector query — the filter does the actual selection.
        const vecs = await embedder.embed([document_id]);
        const queryVector = vecs[0] ? Array.from(vecs[0]) : undefined;

        const retrieveQuery: RetrieveQuery = {
          mode: "vector",
          query_text: document_id,
          query_vector: queryVector,
          top_k: 1,
          filter: {
            eq: { field: "doc.external_id", value: document_id },
          },
          include_content: true,
          include_document: true,
          group_by_document: true,
        };

        const output = await store.retrieve(collection, retrieveQuery);

        if (!output.chunks || output.chunks.length === 0) {
          return {
            content: [
              {
                type: "text" as const,
                text: JSON.stringify({ error: `Document '${document_id}' not found in collection '${collection}'` }),
              },
            ],
            isError: true,
          };
        }

        const doc = output.chunks[0]?.document;
        if (!doc) {
          return {
            content: [
              {
                type: "text" as const,
                text: JSON.stringify({ error: "Document found but no metadata available" }),
              },
            ],
            isError: true,
          };
        }

        const docMetadata = doc.metadata as Record<string, unknown> | undefined;

        return {
          content: [
            {
              type: "text" as const,
              text: JSON.stringify({
                document_id: doc.id,
                external_id: doc.external_id,
                title: doc.title,
                mime: doc.mime,
                ingested_at: doc.ingested_at
                  ? new Date(doc.ingested_at * 1000).toISOString()
                  : null,
                keywords: doc.keywords,
                metadata: doc.metadata,
                pii_stats: {
                  pii_count: docMetadata?.pii_count ?? "unknown",
                  pii_categories: docMetadata?.pii_categories ?? "unknown",
                  ingestion_date: docMetadata?.ingestion_date ?? "unknown",
                },
                chunks: output.chunks.length,
              }, null, 2),
            },
          ],
        };
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        return {
          content: [{ type: "text" as const, text: `get_document_report failed: ${msg}` }],
          isError: true,
        };
      }
    }
  );

  server.tool(
    "explain_reports",
    "Usage guide for redaction reports and redacted files, with GDPR compliance notes.",
    {
      format: z.enum(["summary", "detailed"]).optional().default("summary"),
    },
    async ({ format }) => {
      const summary = {
        overview: "Redaction reports document what PII was detected and redacted from your documents during ingestion.",
        file_naming: {
          original: "contract.pdf (untouched)",
          redacted: "contract_REDACTED.pdf (redacted copy)",
          report: "contract_REPORT.docx (per-file PII report)",
          summary: "SUMMARY_REPORT.docx (aggregated report for folder)",
          rehydration_map: ".rehydration/contract.pdf.map (token-to-original mapping)",
        },
        workflow: [
          "1. ingest_folder scans source_folder, extracts text from each file",
          "2. detect_pii finds EMAIL, PHONE, SSN, CREDIT_CARD, DATE, IP_ADDRESS entities",
          "3. redact_document replaces PII with [CATEGORY_N] tokens",
          "4. Redacted copies created in redacted_folder/*_REDACTED.*",
          "5. PII reports generated as *_REPORT.docx with category breakdowns",
          "6. Rehydration maps stored in .rehydration/ for later restoration",
          "7. Chunked, embedded text stored in RAG collection",
        ],
        gdpr_notes: [
          "Original files are NEVER modified - only redacted copies are created",
          "Rehydration maps enable authorized PII restoration via passphrase",
          "Art. 17 (Right to Erasure): Use delete_document or delete_by_filter to remove documents",
          "Art. 25 (Privacy by Design): TokenReplace is the default strategy",
          "Art. 32 (Security): Rehydration maps should be encrypted in production",
        ],
        tools: {
          get_ingestion_summary: "View collection-level statistics and PII counts",
          get_document_report: "View PII details for a specific document",
          query_corpus: "Search the RAG corpus with redacted text",
          rehydrate_tokens: "Restore PII in text using token map",
          list_tokens: "Discover tokens in redacted text (without revealing originals)",
        },
      };

      const detailed = {
        ...summary,
        redaction_strategies: {
          token_replace:
            "Replaces PII with [CATEGORY_N] tokens (e.g., [EMAIL_1], [PHONE_2]). Preserves document structure for embedding quality. Tokens are stable IDs - same person gets same token across all chunks.",
          mask: "Replaces PII with asterisks (e.g., je***@***.com). Destroys structure but signals redaction was applied.",
          hash: "Replaces PII with SHA256 hash prefix (e.g., HASH_a1b2c3). Enables detection without original storage.",
        },
        token_map_format: {
          description: "Maps redaction tokens back to original values for authorized rehydration",
          example: {
            "[PERSON_1]": "Jean Dupont",
            "[EMAIL_1]": "jean@example.com",
            "[PHONE_1]": "+33 6 12 34 56 78",
          },
        },
        report_contents: {
          per_file:
            "PII Report shows: filename, timestamp, total PII count, category breakdown table, list of detected entities with tokens and originals",
          summary:
            "SUMMARY_REPORT shows: total files, total PII entities, category breakdown, per-file breakdown, output locations, usage instructions",
        },
        security_considerations: [
          "Store rehydration maps separately from redacted documents",
          "Use encryption (AES-256-GCM) for production rehydration maps",
          "Require passphrase for rehydration operations",
          "Consider key rotation for long-term collections",
          "Audit log rehydration access for compliance",
        ],
      };

      return {
        content: [
          {
            type: "text" as const,
            text: JSON.stringify(format === "detailed" ? detailed : summary, null, 2),
          },
        ],
      };
    }
  );
}

import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { getRuntime } from "../engine.js";
import { listTrackedCollections } from "../collection-registry.js";
import type { RetrieveQuery } from "xberg-wasm-runtime";

export function registerStatsTools(server: McpServer): void {
  server.tool(
    "collection_stats",
    "Get aggregate statistics for a RAG collection: document count, chunk count, and last ingestion time.",
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
                documents: stats.documents,
                chunks: stats.chunks,
                last_ingested_at: stats.last_ingested_at
                  ? new Date(stats.last_ingested_at * 1000).toISOString()
                  : null,
              }, null, 2),
            },
          ],
        };
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        return {
          content: [{ type: "text" as const, text: `collection_stats failed: ${msg}` }],
          isError: true,
        };
      }
    }
  );

  server.tool(
    "list_collections",
    "List all RAG collections that have been created in this store.",
    {},
    async () => {
      try {
        const tracked = listTrackedCollections();

        const { store } = getRuntime();
        const collections: Array<{ name: string; spec?: unknown }> = [];

        for (const name of tracked) {
          try {
            const spec = await store.getCollection(name);
            if (spec) {
              // R3: expose the public "inner_product" spelling, not the runtime "innerproduct".
              collections.push({
                name,
                spec: {
                  ...spec,
                  distance_metric:
                    spec.distance_metric === "innerproduct"
                      ? "inner_product"
                      : spec.distance_metric,
                },
              });
            } else {
              collections.push({ name });
            }
          } catch {
            collections.push({ name });
          }
        }

        return {
          content: [
            {
              type: "text" as const,
              text: JSON.stringify({
                collections,
                count: collections.length,
                note: "Lists collections created via create_collection in this session. Does not include collections created by other processes.",
              }, null, 2),
            },
          ],
        };
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        return {
          content: [{ type: "text" as const, text: `list_collections failed: ${msg}` }],
          isError: true,
        };
      }
    }
  );

  server.tool(
    "get_audit_log",
    "View the audit trail of PII detection operations. Note: requires documents to have been ingested with metadata tracking.",
    {
      collection: z.string().optional(),
      limit: z.number().int().min(1).max(500).optional().default(50),
    },
    async ({ collection, limit }) => {
      return {
        content: [
          {
            type: "text" as const,
            text: JSON.stringify({
              note: "Audit logging requires explicit tracking during ingestion. Currently, audit data is stored in document.metadata.ingestion_date, document.metadata.pii_count, and document.metadata.pii_categories fields.",
              suggestion: "Query individual documents using get_document_report to retrieve stored audit data.",
            }),
          },
        ],
      };
    }
  );

  server.tool(
    "get_extraction_stats",
    "View extraction performance metrics from the last extraction operation.",
    {},
    async () => {
      return {
        content: [
          {
            type: "text" as const,
            text: JSON.stringify({
              note: "Extraction stats are available per-document in result.results[].metadata.additional. For aggregate stats, enable telemetry/monitoring in production deployment.",
              metrics_available: [
                "pages_processed",
                "tables_extracted",
                "images_extracted",
                "ocr_applied",
                "confidence_score",
                "processing_time_ms",
              ],
            }),
          },
        ],
      };
    }
  );

  server.tool(
    "export_collection",
    "Export a collection as JSON or JSONL for backup and migration.",
    {
      collection: z.string(),
      format: z.enum(["json", "jsonl"]).optional().default("json"),
    },
    async ({ collection, format }) => {
      try {
        const { store } = getRuntime();
        const stats = await store.collectionStats(collection);

        if (stats.documents === 0) {
          return {
            content: [
              {
                type: "text" as const,
                text: JSON.stringify({ error: `Collection '${collection}' is empty` }),
              },
            ],
            isError: true,
          };
        }

        return {
          content: [
            {
              type: "text" as const,
              text: JSON.stringify({
                note: "Full export requires iterating all documents and chunks. This tool provides collection metadata only. For full export, implement document iteration using retrieve() with pagination.",
                collection,
                documents: stats.documents,
                chunks: stats.chunks,
                export_format: format,
              }),
            },
          ],
        };
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        return {
          content: [{ type: "text" as const, text: `export_collection failed: ${msg}` }],
          isError: true,
        };
      }
    }
  );

  server.tool(
    "import_collection",
    "Import documents from a JSON/JSONL file into a collection.",
    {
      collection: z.string(),
      file_path: z.string(),
      format: z.enum(["json", "jsonl"]).optional().default("json"),
    },
    async ({ collection, file_path, format }) => {
      try {
        const fs = await import("fs");

        if (!fs.existsSync(file_path)) {
          return {
            content: [
              {
                type: "text" as const,
                text: JSON.stringify({ error: `File not found: ${file_path}` }),
              },
            ],
            isError: true,
          };
        }

        return {
          content: [
            {
              type: "text" as const,
              text: JSON.stringify({
                note: "Import functionality requires parsing JSON/JSONL and calling upsert_document for each document. Implement chunking and embedding in the calling code.",
                collection,
                file_path,
                format,
                status: "not_implemented",
              }),
            },
          ],
        };
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        return {
          content: [{ type: "text" as const, text: `import_collection failed: ${msg}` }],
          isError: true,
        };
      }
    }
  );

  server.tool(
    "update_metadata",
    "Update metadata fields on an existing document in a collection.",
    {
      collection: z.string(),
      document_id: z.string(),
      metadata: z.record(z.unknown()),
    },
    async ({ collection, document_id, metadata }) => {
      try {
        const { embedder, store } = getRuntime();

        // R6: the wasm store is vector-only, so this pre-check for document
        // existence is expressed as a filtered vector query rather than
        // mode:"full_text" (which the wasm store rejects).
        const vecs = await embedder.embed([document_id]);
        const queryVector = vecs[0] ? Array.from(vecs[0]) : undefined;

        const retrieveQuery: RetrieveQuery = {
          mode: "vector",
          query_text: document_id,
          query_vector: queryVector,
          top_k: 1,
          filter: { eq: { field: "doc.external_id", value: document_id } },
          include_content: true,
          include_document: true,
        };

        const output = await store.retrieve(collection, retrieveQuery);

        if (!output.chunks || output.chunks.length === 0) {
          return {
            content: [
              {
                type: "text" as const,
                text: JSON.stringify({ error: `Document '${document_id}' not found` }),
              },
            ],
            isError: true,
          };
        }

        return {
          content: [
            {
              type: "text" as const,
              text: JSON.stringify({
                note: "Update requires re-upserting the document with merged metadata. Current implementation is a stub.",
                collection,
                document_id,
                attempted_metadata: metadata,
                status: "not_implemented",
              }),
            },
          ],
        };
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        return {
          content: [{ type: "text" as const, text: `update_metadata failed: ${msg}` }],
          isError: true,
        };
      }
    }
  );
}

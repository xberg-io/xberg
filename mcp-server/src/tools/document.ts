import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { getRuntime } from "../engine.js";
import type { DocumentRecord, ChunkRecord, Filter, RetrieveQuery } from "xberg-wasm-runtime";

const DocumentRecordSchema = z.object({
  external_id: z.string().optional(),
  title: z.string().optional(),
  mime: z.string().optional(),
  source_uri: z.string().optional(),
  full_text: z.string(),
  keywords: z.array(z.string()).optional().default([]),
  entities: z.unknown().optional(),
  labels: z.unknown().optional(),
  metadata: z.unknown().optional(),
});

const ChunkRecordSchema = z.object({
  external_id: z.string().optional(),
  ordinal: z.number().int(),
  content: z.string(),
  embedding: z.array(z.number()),
  chunk_metadata: z.unknown().optional(),
});

const FilterConditionSchema: z.ZodType<unknown> = z.lazy(() =>
  z.union([
    z.object({
      eq: z.object({ field: z.string(), value: z.unknown() }),
    }),
    z.object({
      in: z.object({ field: z.string(), values: z.array(z.unknown()) }),
    }),
    z.object({
      range: z.object({
        field: z.string(),
        gte: z.unknown().optional(),
        gt: z.unknown().optional(),
        lte: z.unknown().optional(),
        lt: z.unknown().optional(),
      }),
    }),
    z.object({
      array_contains: z.object({ field: z.string(), value: z.unknown() }),
    }),
    z.object({
      text_match: z.object({ field: z.string(), query: z.string() }),
    }),
    z.object({
      and: z.object({ filters: z.array(FilterConditionSchema) }),
    }),
    z.object({
      or: z.object({ filters: z.array(FilterConditionSchema) }),
    }),
    z.object({
      not: z.object({ filter: z.union([z.lazy(() => FilterConditionSchema), z.record(z.unknown())]) }),
    }),
  ])
);

export function registerDocumentTools(server: McpServer): void {
  server.tool(
    "upsert_document",
    "Insert or update a document with pre-chunked content and embeddings in a RAG collection. Caller provides the chunks already created and embedded.",
    {
      collection: z.string(),
      document: DocumentRecordSchema,
      chunks: z.array(ChunkRecordSchema),
    },
    async ({ collection, document, chunks }) => {
      try {
        const { store } = getRuntime();
        const docId = await store.upsertDocument(
          collection,
          document as DocumentRecord,
          chunks as ChunkRecord[]
        );
        return {
          content: [{ type: "text" as const, text: JSON.stringify({ doc_id: docId }) }],
        };
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        return {
          content: [{ type: "text" as const, text: `upsert_document failed: ${msg}` }],
          isError: true,
        };
      }
    }
  );

  server.tool(
    "get_document",
    "Retrieve a document by its external ID, or by an explicit filter. Note: the vector-only wasm store has no internal-document-id index (mirroring the Rust backend's filter fields, which expose doc.external_id but not an internal id), so lookups resolve against the document's external_id. Pass a custom `filter` for anything else.",
    {
      collection: z.string(),
      external_id: z.string().optional().describe("The document's external_id (as supplied to upsert_document)."),
      document_id: z.string().optional().describe("Alias for external_id — matched against the document's external_id. The internal doc_id returned by upsert_document is not directly queryable in the vector-only store; pass it via external_id semantics or use a filter."),
      filter: FilterConditionSchema.optional(),
    },
    async ({ collection, document_id, external_id, filter }) => {
      try {
        const { embedder, store } = getRuntime();

        // Both `external_id` and its `document_id` alias resolve to a
        // doc.external_id match: the wasm store mirrors the Rust backend's
        // filter fields (`crates/xberg-rag/src/backends/memory.rs` resolve_field),
        // which has no internal-id field.
        const idValue = external_id ?? document_id;
        const retrieveFilter = filter ?? (idValue
          ? { eq: { field: "doc.external_id", value: idValue } }
          : null);

        if (!retrieveFilter) {
          return {
            content: [{ type: "text" as const, text: JSON.stringify({ error: "Must provide external_id, document_id, or filter" }) }],
            isError: true,
          };
        }

        // R6: the wasm store is vector-only (no full_text mode), so fetch-by-id
        // is expressed as a filtered vector query — the filter does the actual
        // selection, the vector score only orders the (at most one matching
        // group of) chunks.
        const queryText = idValue ?? "document";
        const vecs = await embedder.embed([queryText]);
        const queryVector = vecs[0] ? Array.from(vecs[0]) : undefined;

        const rq: RetrieveQuery = {
          mode: "vector",
          query_text: queryText,
          query_vector: queryVector,
          top_k: 1,
          filter: retrieveFilter as Filter,
          include_content: true,
          include_document: true,
          group_by_document: true,
        };

        const output = await store.retrieve(collection, rq);

        return {
          content: [{ type: "text" as const, text: JSON.stringify(output, null, 2) }],
        };
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        return {
          content: [{ type: "text" as const, text: `get_document failed: ${msg}` }],
          isError: true,
        };
      }
    }
  );

  server.tool(
    "delete_documents",
    "Remove one or more documents and their chunks from a collection (GDPR Art. 17 - Right to Erasure).",
    {
      collection: z.string(),
      ids: z.array(z.string()).describe("Document IDs or external IDs to delete"),
    },
    async ({ collection, ids }) => {
      try {
        const { store } = getRuntime();
        const count = await store.deleteDocuments(collection, ids);
        return {
          content: [{ type: "text" as const, text: JSON.stringify({ deleted: count }) }],
        };
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        return {
          content: [{ type: "text" as const, text: `delete_documents failed: ${msg}` }],
          isError: true,
        };
      }
    }
  );

  server.tool(
    "delete_by_filter",
    "Bulk delete all documents matching a filter condition.",
    {
      collection: z.string(),
      filter: FilterConditionSchema,
    },
    async ({ collection, filter }) => {
      try {
        const { store } = getRuntime();
        const count = await store.deleteByFilter(collection, filter as Filter);
        return {
          content: [{ type: "text" as const, text: JSON.stringify({ deleted: count }) }],
        };
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        return {
          content: [{ type: "text" as const, text: `delete_by_filter failed: ${msg}` }],
          isError: true,
        };
      }
    }
  );
}

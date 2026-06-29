import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { getStore } from "../store.js";

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
        const store = await getStore();
        const docId = await store.upsertDocument(
          collection,
          JSON.stringify(document),
          JSON.stringify(chunks)
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
    "Retrieve a document by its external ID or document ID using a filter.",
    {
      collection: z.string(),
      document_id: z.string().optional(),
      external_id: z.string().optional(),
      filter: FilterConditionSchema.optional(),
    },
    async ({ collection, document_id, external_id, filter }) => {
      try {
        const store = await getStore();

        const retrieveFilter = filter ?? (document_id
          ? { eq: { field: "doc.external_id", value: document_id } }
          : external_id
          ? { eq: { field: "doc.external_id", value: external_id } }
          : null);

        if (!retrieveFilter) {
          return {
            content: [{ type: "text" as const, text: JSON.stringify({ error: "Must provide document_id, external_id, or filter" }) }],
            isError: true,
          };
        }

        const queryJson = JSON.stringify({
          mode: "full_text",
          query_text: document_id ?? external_id ?? "document",
          top_k: 1,
          filter: retrieveFilter,
          include_content: true,
          include_document: true,
          group_by_document: true,
        });

        const outputJson = await store.retrieve(collection, queryJson);
        const output = JSON.parse(outputJson);

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
        const store = await getStore();
        const count = await store.deleteDocuments(collection, JSON.stringify(ids));
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
        const store = await getStore();
        const count = await store.deleteByFilter(collection, JSON.stringify(filter));
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
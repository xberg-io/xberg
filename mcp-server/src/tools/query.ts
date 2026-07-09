import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { getRuntime } from "../engine.js";
import type { RetrieveQuery, Filter } from "xberg-wasm-runtime";

const RetrieveModeSchema = z.enum(["vector", "full_text", "hybrid", "graph"]);
const FilterSchema: z.ZodType<unknown> = z.lazy(() =>
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
      and: z.object({ filters: z.array(FilterSchema) }),
    }),
    z.object({
      or: z.object({ filters: z.array(FilterSchema) }),
    }),
    z.object({
      not: z.object({ filter: z.union([z.lazy(() => FilterSchema), z.record(z.unknown())]) }),
    }),
  ])
);

export function registerQueryTools(server: McpServer): void {
  server.tool(
    "query_corpus",
    "Search a RAG corpus using the current wasm runtime's vector retrieval. query_text is auto-embedded. Note: full_text and graph modes are unsupported (rejected); hybrid is accepted for compatibility but runs as vector; rerank_results, graph_depth, and embedding_preset are currently accepted but ignored.",
    {
      collection: z.string().describe("Collection name"),
      query: z.string().describe("Search query text"),
      mode: RetrieveModeSchema.optional().default("hybrid"),
      top_k: z.number().int().min(1).max(200).optional().default(10),
      graph_depth: z.number().int().min(1).max(5).optional().default(2),
      filter: FilterSchema.optional(),
      include_content: z.boolean().optional().default(true),
      include_document: z.boolean().optional().default(false),
      rerank_results: z.boolean().optional().default(false),
      embedding_preset: z.enum(["speed", "balanced", "quality"]).optional().default("balanced"),
    },
    async ({
      collection,
      query,
      mode,
      top_k,
      graph_depth: _graph_depth,
      filter,
      include_content,
      include_document,
      rerank_results: _rerank_results,
      embedding_preset: _embedding_preset,
    }) => {
      try {
        // The wasm in-memory store only supports vector retrieval; full_text/graph
        // are honestly unsupported rather than silently degraded.
        if (mode === "full_text" || mode === "graph") {
          return {
            content: [
              {
                type: "text" as const,
                text: `query_corpus: mode '${mode}' is not supported by the current wasm runtime store (vector-only). Use 'vector' or 'hybrid'.`,
              },
            ],
            isError: true,
          };
        }

        const { embedder, store } = getRuntime();

        // Embed the query for vector retrieval (covers both 'vector' and 'hybrid',
        // which is coerced to vector below since the backend has no hybrid mode).
        const vecs = await embedder.embed([query]);
        const queryVector = vecs[0] ? Array.from(vecs[0]) : undefined;

        // embedding_preset no longer selects a model: the injected embedder is fixed
        // by the wasm runtime, so this parameter is accepted but unused.
        // graph_depth is likewise accepted but unused: graph mode is rejected above.

        const rq: RetrieveQuery = {
          mode: "vector",
          query_text: query,
          query_vector: queryVector,
          top_k,
          filter: filter as Filter | undefined,
          include_content,
          include_document,
          group_by_document: false,
        };

        const output = await store.retrieve(collection, rq);

        // rerank_results: no reranker is injected by the wasm runtime yet. We do
        // not over-fetch (top_k*5) or attempt reranking; results stay vector-ordered.
        // Follow-up: wire in a reranker once the runtime injects one.

        return {
          content: [{ type: "text" as const, text: JSON.stringify(output, null, 2) }],
        };
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        return {
          content: [{ type: "text" as const, text: `query_corpus failed: ${msg}` }],
          isError: true,
        };
      }
    }
  );
}
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { embedTexts, rerank } from "xberg-rag-node";
import { getStore, withTimeout } from "../store.js";

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
    "Search a RAG corpus with vector, full-text, hybrid, or graph retrieval + optional reranking. query_text is auto-embedded for vector/hybrid modes. Use graph_depth to control traversal depth in graph mode.",
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
      graph_depth,
      filter,
      include_content,
      include_document,
      rerank_results,
      embedding_preset,
    }) => {
      try {
        let queryVector: number[] | null = null;

        if (mode === "vector" || mode === "hybrid") {
          const embJson = await withTimeout(
            embedTexts(
              JSON.stringify([query]),
              JSON.stringify({ model: { type: "preset", name: embedding_preset } }),
            ),
            60_000,
            "embedTexts",
          );
          const vecs = JSON.parse(embJson) as number[][];
          queryVector = vecs[0] ?? null;
        }

        const effectiveTopK = rerank_results ? top_k * 5 : top_k;

        const retrieveQuery = {
          mode,
          query_text: query,
          query_vector: queryVector,
          top_k: effectiveTopK,
          filter: filter ?? null,
          include_content,
          include_document,
          group_by_document: false,
          graph_depth: mode === "graph" ? graph_depth : null,
          candidate_multiplier: null,
        };

        const store = await getStore();
        const outputJson = await store.retrieve(collection, JSON.stringify(retrieveQuery));
        const output = JSON.parse(outputJson) as {
          chunks: Array<{ content?: string; score: number; document_id?: string }>;
          mode?: string;
        };

        if (rerank_results && output.chunks.length > 0) {
          const docs = output.chunks
            .map((c) => c.content ?? "")
            .filter((c) => c.length > 0);

          if (docs.length > 0) {
            const rerankedJson = await rerank(
              query,
              JSON.stringify(docs),
              JSON.stringify({ model: { type: "preset", name: "balanced" } })
            );
            const ranked = JSON.parse(rerankedJson) as Array<{ index: number; score: number }>;

            output.chunks = ranked
              .sort((a, b) => b.score - a.score)
              .slice(0, top_k)
              .map((r) => output.chunks[r.index]!)
              .filter((c) => c !== undefined);
          }
        }

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
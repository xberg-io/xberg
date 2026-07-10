import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { getRuntime } from "../engine.js";
import { trackCollection, untrackCollection } from "../collection-registry.js";
import type { CollectionSpec, DistanceMetric } from "xberg-wasm-runtime";

export function registerCollectionTools(server: McpServer): void {
  server.tool(
    "create_collection",
    "Create a RAG vector collection. The injected wasm embedder is fixed at 384 dimensions (all-MiniLM), so the default embedding_dim=384 matches it — collections used with ingest_document/query_corpus must be 384-dim.",
    {
      name: z.string().describe("Collection name"),
      embedding_dim: z.number().int().positive().default(384).describe("Embedding dimension. Must match the embedder the tools use — the runtime's fixed embedder produces 384-dim vectors, so collections used with ingest_document/query_corpus must be 384."),
      distance_metric: z.enum(["cosine", "l2", "inner_product"]).optional().default("cosine"),
      index_method: z.enum(["flat", "hnsw", "diskann"]).optional().default("flat"),
    },
    async ({ name, embedding_dim, distance_metric, index_method }) => {
      try {
        const { store } = getRuntime();

        // R3: public schema keeps "inner_product"; the runtime store spells it "innerproduct".
        const dm: DistanceMetric | undefined =
          distance_metric === "inner_product" ? "innerproduct" : distance_metric;

        const spec: CollectionSpec = {
          name,
          embedding_dim,
          distance_metric: dm,
          index_method,
        };

        // ensureCollection returns an error-message string on failure (never throws).
        const err = await store.ensureCollection(spec);
        if (typeof err === "string") {
          throw new Error(err);
        }

        trackCollection(name);
        return {
          content: [
            {
              type: "text" as const,
              text: JSON.stringify({ status: "created", name, embedding_dim, distance_metric }),
            },
          ],
        };
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        return {
          content: [{ type: "text" as const, text: `create_collection failed: ${msg}` }],
          isError: true,
        };
      }
    }
  );

  server.tool(
    "get_collection",
    "Get the specification of a collection (embedding_dim, distance_metric, index_method).",
    { name: z.string() },
    async ({ name }) => {
      try {
        const { store } = getRuntime();
        const spec = await store.getCollection(name);
        if (!spec) {
          return {
            content: [{ type: "text" as const, text: JSON.stringify({ error: `Collection '${name}' not found` }) }],
            isError: true,
          };
        }
        // R3: normalize the runtime "innerproduct" spelling back to the public
        // schema "inner_product" so the response round-trips into create_collection.
        const publicSpec = {
          ...spec,
          distance_metric:
            spec.distance_metric === "innerproduct" ? "inner_product" : spec.distance_metric,
        };
        return {
          content: [{ type: "text" as const, text: JSON.stringify(publicSpec) }],
        };
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        return {
          content: [{ type: "text" as const, text: `get_collection failed: ${msg}` }],
          isError: true,
        };
      }
    }
  );

  server.tool(
    "drop_collection",
    "Permanently delete a collection and all its documents.",
    { name: z.string() },
    async ({ name }) => {
      try {
        const { store } = getRuntime();

        // dropCollection returns an error-message string on failure (never throws).
        const err = await store.dropCollection(name);
        if (typeof err === "string") {
          throw new Error(err);
        }

        untrackCollection(name);
        return {
          content: [{ type: "text" as const, text: JSON.stringify({ status: "dropped", name }) }],
        };
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        return {
          content: [{ type: "text" as const, text: `drop_collection failed: ${msg}` }],
          isError: true,
        };
      }
    }
  );
}

import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { getRuntime } from "../engine.js";
import { trackCollection, untrackCollection } from "../collection-registry.js";
import type { CollectionSpec, DistanceMetric } from "xberg-wasm-runtime";
import { EMBEDDING_DIM, EMBEDDER_MODEL_ID } from "../lib/constants.js";

export function registerCollectionTools(server: McpServer): void {
  server.tool(
    "create_collection",
    `Create a RAG vector collection. The default embedder is ${EMBEDDER_MODEL_ID} (${EMBEDDING_DIM} dimensions), so the default embedding_dim=${EMBEDDING_DIM} matches it — collections used with ingest_document/query_corpus must be ${EMBEDDING_DIM}-dim.`,
    {
      name: z.string().describe("Collection name"),
      embedding_dim: z.number().int().positive().default(EMBEDDING_DIM).describe(`Embedding dimension. Must match the embedder the tools use — the runtime's fixed embedder produces ${EMBEDDING_DIM}-dim vectors, so collections used with ingest_document/query_corpus must be ${EMBEDDING_DIM}.`),
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

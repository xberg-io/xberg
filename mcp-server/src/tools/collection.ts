import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { getStore, trackCollection, untrackCollection } from "../store.js";

export function registerCollectionTools(server: McpServer): void {
  server.tool(
    "create_collection",
    "Create a RAG vector collection. Use embedding_dim=1024 for BGE-M3 (default model).",
    {
      name: z.string().describe("Collection name"),
      embedding_dim: z.number().int().positive().default(1024),
      distance_metric: z.enum(["cosine", "l2", "inner_product"]).optional().default("cosine"),
      index_method: z.enum(["flat", "hnsw", "diskann"]).optional().default("flat"),
    },
    async ({ name, embedding_dim, distance_metric, index_method }) => {
      try {
        const store = await getStore();
        const specJson = JSON.stringify({
          name,
          embedding_dim,
          distance_metric,
          index_method,
        });
        await store.ensureCollection(specJson);
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
        const store = await getStore();
        const specJson = await store.getCollection(name);
        if (!specJson) {
          return {
            content: [{ type: "text" as const, text: JSON.stringify({ error: `Collection '${name}' not found` }) }],
            isError: true,
          };
        }
        return {
          content: [{ type: "text" as const, text: specJson }],
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
        const store = await getStore();
        await store.dropCollection(name);
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
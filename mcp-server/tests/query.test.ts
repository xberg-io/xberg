import { describe, it, expect, beforeAll } from "vitest";
import { initializeEngine, getEngine, getRuntime } from "../src/engine.js";

describe("query_corpus (Task 6 core behavior)", () => {
  const EMBEDDING_DIM = 384; // matches xberg-wasm-runtime's default embedder model

  beforeAll(async () => {
    await initializeEngine();
    const { store } = getRuntime();
    await store.ensureCollection({ name: "q_col", embedding_dim: EMBEDDING_DIM });
    await getEngine().ingest(
      {
        full_text: "Hello world. A document about machine learning and vector search.",
        title: "T",
        keywords: [],
        entities: {},
        labels: {},
        metadata: {},
      },
      "q_col"
    );
  }, 180_000);

  it("retrieves scored chunks via the runtime store (vector mode)", async () => {
    const { embedder, store } = getRuntime();
    const vecs = await embedder.embed(["machine learning"]);
    const queryVector = vecs[0] ? Array.from(vecs[0]) : undefined;

    const output = (await store.retrieve("q_col", {
      mode: "vector",
      query_text: "machine learning",
      query_vector: queryVector,
      top_k: 3,
      include_content: true,
    })) as {
      chunks: Array<{ score: number; primary_score: { kind: string; score?: number } }>;
    };

    expect(output.chunks.length).toBeGreaterThan(0);
    const top = output.chunks[0]!;
    expect(typeof top.score).toBe("number");
    expect(top.primary_score.kind).toBe("vector");
    expect(typeof top.primary_score.score).toBe("number");
  }, 60_000);

  it("registers query_corpus without throwing (no native imports)", async () => {
    const { McpServer } = await import("@modelcontextprotocol/sdk/server/mcp.js");
    const { registerQueryTools } = await import("../src/tools/query.js");
    const server = new McpServer({ name: "t", version: "0" });
    expect(() => registerQueryTools(server)).not.toThrow();
  });
});

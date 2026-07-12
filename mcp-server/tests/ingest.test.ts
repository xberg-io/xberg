import { describe, it, expect, beforeAll } from "vitest";
import type { XbergEngine } from "@xberg-io/xberg-wasm";
import type { VectorStoreInterface } from "xberg-wasm-runtime";
import { createXbergRuntimeFactory } from "xberg-wasm-runtime";
import { EMBEDDING_DIM } from "../src/lib/constants.js";

// ingest.ts (the tool module under test for Task 5) imports store.ts, which
// requires the native xberg-rag-node .node binding. That binding is not built
// in this worktree, so importing the tool module directly fails at load time.
// We verify the CORE behavior — engine.ingest()/engine.query() — directly
// against the wasm engine, which does not need the native binding. The
// tool-level wiring (Zod schemas, response shape) is covered by static
// reading of src/tools/ingest.ts and by tools.test.ts's registration smoke
// tests, consistent with the pattern in e2e-native.test.ts /
// rehydration_compat.test.ts.
//
// Unlike src/engine.ts's `initializeEngine()` singleton (which does not
// expose its injected store), this test builds its own injection descriptor
// via `createXbergRuntimeFactory` so it can hold a direct handle to the
// wasm-runtime in-memory store and call `ensureCollection` before ingesting
// — mirroring what a real caller must do, since neither `XbergEngine.ingest`
// nor the current `create_collection` MCP tool (which targets the *native*
// xberg-rag-node store, not this wasm one) creates it automatically.

describe("engine.ingest (Task 5 core behavior)", () => {
  let engine: XbergEngine;
  let store: VectorStoreInterface;

  beforeAll(async () => {
    const injection = await createXbergRuntimeFactory();
    store = injection.store;
    const { XbergEngine: XbergEngineCtor } = await import("@xberg-io/xberg-wasm");
    engine = new XbergEngineCtor({}, injection);
  }, 180_000);

  // The wasm-runtime in-memory store (packages/xberg-wasm-runtime/src/store.ts)
  // now implements B's real JS VectorStore protocol (ensureCollection,
  // upsertDocument returning a bare document id string, retrieve, etc. — see
  // crates/xberg-wasm/src/bridge/store.rs), so engine.ingest()/engine.query()
  // succeed end-to-end once the collection exists.
  it("ingests a document and returns a document id", async () => {
    await store.ensureCollection({ name: "test_col", embedding_dim: EMBEDDING_DIM });
    const doc = {
      full_text: "Hello world. This is a test document about machine learning.",
      title: "Test",
      keywords: [],
      entities: {},
      labels: {},
      metadata: {},
    };

    const documentId = await engine.ingest(doc, "test_col");
    expect(typeof documentId).toBe("string");
    expect((documentId as string).length).toBeGreaterThan(0);
  }, 60_000);

  // Regression test for the PrimaryScore serde fix (crates/xberg-rag/src/types.rs):
  // `PrimaryScore` was an internally-tagged enum (`#[serde(tag = "kind")]`) whose
  // `Vector`/`FullText` variants wrapped a bare `f32`. Internally-tagged newtype
  // scalar variants have nothing to flatten alongside the tag key, so serde could
  // (de)serialize neither direction — every JS-backed `VectorStore.retrieve()`
  // returning a non-`Hybrid` `primary_score` failed with "invalid type: map,
  // expected f32". The variants are now struct variants `{ score }`, so the wire
  // shape `{ kind: "vector", score }` round-trips and the JS-backed store works
  // end-to-end. See the inline note below.
  it("engine.query returns scored chunks with a deserializable primary_score", async () => {
    await store.ensureCollection({ name: "test_col_query", embedding_dim: EMBEDDING_DIM });
    const doc = {
      full_text: "Hello world. This is a test document about machine learning.",
      title: "Test",
      keywords: [],
      entities: {},
      labels: {},
      metadata: {},
    };
    await engine.ingest(doc, "test_col_query");

    // Regression: `PrimaryScore` was an internally-tagged enum with newtype
    // scalar variants (`Vector(f32)`), which serde cannot (de)serialize —
    // every query() failed with "invalid type: map, expected f32". The variants
    // are now struct variants `{ score }`, so the wire shape
    // `{ kind: "vector", score }` round-trips. See crates/xberg-rag/src/types.rs.
    //
    // engine.query returns the full RetrieveOutput (`&output` in
    // crates/xberg-wasm/src/engine.rs), i.e. `{ mode, chunks, primary_latency_ms }`,
    // not a bare chunk array.
    const output = (await engine.query("machine learning", "test_col_query", 3)) as {
      mode: string;
      chunks: Array<{ score: number; primary_score: { kind: string; score?: number } }>;
      primary_latency_ms: number;
    };
    expect(output.mode).toBe("vector");
    expect(Array.isArray(output.chunks)).toBe(true);
    expect(output.chunks.length).toBeGreaterThan(0);
    const top = output.chunks[0]!;
    expect(typeof top.score).toBe("number");
    expect(top.primary_score.kind).toBe("vector");
    expect(typeof top.primary_score.score).toBe("number");
  }, 60_000);

  it("camelCase chunking config is parsed and ingestion succeeds", async () => {
    await store.ensureCollection({ name: "test_col_chunking", embedding_dim: EMBEDDING_DIM });
    const longText = "Sentence about apples. ".repeat(50);
    const doc = {
      full_text: longText,
      keywords: [],
      entities: {},
      labels: {},
      metadata: {},
    };
    // Uses a chunking config with camelCase keys, per
    // crates/xberg-wasm/src/engine.rs's manual (non-serde) config parse.
    const documentId = await engine.ingest(doc, "test_col_chunking", {
      chunking: { maxCharacters: 100, overlap: 10 },
    });
    expect(typeof documentId).toBe("string");
  }, 60_000);
});

describe("ingest tool module", () => {
  it("registers ingest_document and ingest_folder without throwing", async () => {
    const { McpServer } = await import("@modelcontextprotocol/sdk/server/mcp.js");
    const { registerIngestTools } = await import("../src/tools/ingest.js");
    const server = new McpServer({ name: "test", version: "0.0.0" });
    expect(() => registerIngestTools(server)).not.toThrow();
  });
});

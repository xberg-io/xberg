import { describe, it, expect, beforeAll } from "vitest";
import { createRequire } from "node:module";
import type { XbergEngine } from "@xberg-io/xberg-wasm";
import type { VectorStoreInterface } from "xberg-wasm-runtime";
import { createXbergRuntimeFactory } from "xberg-wasm-runtime";

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
function nativeBindingAvailable(): boolean {
  try {
    createRequire(import.meta.url)("xberg-rag-node");
    return true;
  } catch {
    return false;
  }
}

const HAVE_NATIVE = nativeBindingAvailable();

describe("engine.ingest (Task 5 core behavior)", () => {
  let engine: XbergEngine;
  let store: VectorStoreInterface;
  const EMBEDDING_DIM = 384; // matches xberg-wasm-runtime's default embedder model

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

  // KNOWN BUG, out of scope here (flagged separately against
  // crates/xberg-rag/src/types.rs): `PrimaryScore` is an internally-tagged
  // enum (`#[serde(tag = "kind")]`) whose `Vector`/`FullText` variants wrap a
  // bare `f32`. Internally-tagged newtype variants can only deserialize when
  // their inner type is itself a struct/map whose fields flatten alongside
  // the tag key — a bare scalar has nothing to flatten, so this shape is
  // provably unconstructible from JS via `serde_wasm_bindgen` in either
  // direction (confirmed with a standalone wasm-bindgen round-trip: both
  // `to_value` and `from_value` fail for `PrimaryScore::Vector(_)`, and
  // `serde_json` fails identically, so this isn't wasm-specific). Every
  // `RetrievedChunk` returned by a JS-backed `VectorStore.retrieve()` hits
  // this the moment it includes a non-`Hybrid` `primary_score` — i.e. always,
  // for ordinary vector search. This is unfixable from the TS store side;
  // `store.ts`'s `retrieve()` implements everything else per the real
  // protocol (collection lookup, cosine scoring, top_k, filtering) and would
  // work end-to-end once the Rust type is corrected.
  it("engine.query currently throws due to an unconstructible PrimaryScore shape (Rust-side bug)", async () => {
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

    await expect(engine.query("machine learning", "test_col_query", 3)).rejects.toThrow(
      /invalid type: map, expected f32/
    );
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

describe.skipIf(!HAVE_NATIVE)("ingest tool module (requires native binding)", () => {
  it("registers ingest_document and ingest_folder without throwing", async () => {
    const { McpServer } = await import("@modelcontextprotocol/sdk/server/mcp.js");
    const { registerIngestTools } = await import("../src/tools/ingest.js");
    const server = new McpServer({ name: "test", version: "0.0.0" });
    expect(() => registerIngestTools(server)).not.toThrow();
  });
});

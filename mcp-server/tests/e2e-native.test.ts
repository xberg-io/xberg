import { describe, it, expect, beforeAll, afterAll } from "vitest";
import { createRequire } from "node:module";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { unlinkSync, existsSync } from "node:fs";

// The native xberg-rag-node binding (.node binary) is only present when the
// crate has been built locally. In CI the MCP unit-test job does not compile
// the native binding, so we detect its absence up front and skip the whole
// suite instead of crashing at import time (which would fail the run).
function nativeBindingAvailable(): boolean {
  try {
    createRequire(import.meta.url)("xberg-rag-node");
    return true;
  } catch {
    return false;
  }
}

const HAVE_NATIVE = nativeBindingAvailable();

// Deep integration test — exercises the real native xberg-rag-node bindings
// (NAPI-RS .node binary) end-to-end. Validates the store lifecycle that is NOT
// covered by the smoke tests (which never load the native binding).
//
// Embeddings require ONNX Runtime. When the model is not available the
// upsert/retrieve path is skipped (the SQLite store enforces a per-chunk
// embedding matching the collection's declared dimension, so full-text-only
// ingestion is impossible without producing embeddings — see insights report).

const COLLECTION = "it_test_collection";
let dbPath: string;
let store: import("xberg-rag-node").RagStore;
let embeddingsWork = false;

describe.skipIf(!HAVE_NATIVE)("native RAG store end-to-end", () => {
  beforeAll(async () => {
    const native = await import("xberg-rag-node");
    const { getStore } = await import("../src/store.js");

    dbPath = join(tmpdir(), `xberg-it-${Date.now()}.db`);
    // Route getStore() to a temp db instead of the real user store.
    process.env.XBERG_STORE_PATH = dbPath;
    // Bug 1: store.ts must use the static RagStore.openSqlite, not a
    // non-existent module-level openSqlite. getStore() is the real server path.
    store = await getStore();

    try {
      const ctrl = new AbortController();
      const t = setTimeout(() => ctrl.abort(), 4000);
      const p = native.embedTexts(
        JSON.stringify(["probe"]),
        JSON.stringify({ model: { type: "preset", name: "balanced" } }),
      );
      const r = await Promise.race([
        p,
        new Promise<never>((_, rej) =>
          ctrl.signal.addEventListener("abort", () => rej(new Error("timeout"))),
        ),
      ]);
      clearTimeout(t);
      const v = JSON.parse(r as string) as number[][];
      embeddingsWork = Array.isArray(v) && v[0]?.length === 768;
    } catch {
      embeddingsWork = false;
    }
    if (!embeddingsWork) console.warn("[it] ONNX Runtime embeddings unavailable — upsert/retrieve tests skipped");
  });

  afterAll(() => {
    try {
      if (existsSync(dbPath)) unlinkSync(dbPath);
    } catch {
      /* ignore */
    }
  });

  it("opens a store via the server's getStore() (Bug 1 fix)", async () => {
    expect(store).toBeDefined();
    expect(typeof store.ensureCollection).toBe("function");
  });

  it("ensures a collection and reports its spec", async () => {
    await store.ensureCollection(
      JSON.stringify({ name: COLLECTION, embedding_dim: 768, distance_metric: "cosine", index_method: "flat" }),
    );
    const specJson = await store.getCollection(COLLECTION);
    expect(specJson).not.toBeNull();
    expect(JSON.parse(specJson!).name).toBe(COLLECTION);
  });

  it.skipIf(!embeddingsWork)("upserts + retrieves a document end-to-end (Bug 2: 'full_text' mode)", async () => {
    const fullText = "Xberg extracts text from 91+ document formats. Contact alice@example.com for details.";
    const native = await import("xberg-rag-node");
    const emb = JSON.parse(
      await native.embedTexts(JSON.stringify([fullText]), JSON.stringify({ model: { type: "preset", name: "balanced" } })),
    ) as number[][];
    const chunks = [{ ordinal: 0, content: fullText, embedding: emb[0]!, chunk_metadata: { chunk_index: 0, total_chunks: 1 } }];

    const docId = await store.upsertDocument(
      COLLECTION,
      JSON.stringify({ full_text: fullText, title: "sample", keywords: ["xberg"], entities: null, labels: null, metadata: null }),
      JSON.stringify(chunks),
    );
    expect(typeof docId).toBe("string");

    // Bug 2: the TS tool sends mode "full_text"; Rust now accepts it.
    const out = JSON.parse(
      await store.retrieve(
        COLLECTION,
        JSON.stringify({
          mode: "full_text",
          query_text: "xberg",
          top_k: 5,
          include_content: true,
          include_document: false,
          group_by_document: false,
          graph_depth: null,
          candidate_multiplier: null,
        }),
      ),
    ) as { chunks: unknown[] };
    expect(out.chunks.length).toBeGreaterThan(0);

    const stats = JSON.parse(await store.collectionStats(COLLECTION)) as { documents: number };
    expect(stats.documents).toBeGreaterThanOrEqual(1);
  });

  it("retrieve with 'full_text' mode is accepted by Rust (Bug 2 fix)", async () => {
    // Retrieve on an empty collection must NOT fail with "unknown variant
    // full_text" — it should return a result object with a chunks array.
    const outJson = await store.retrieve(
      COLLECTION,
      JSON.stringify({
        mode: "full_text",
        query_text: "xberg",
        top_k: 5,
        include_content: true,
        include_document: false,
        group_by_document: false,
        graph_depth: null,
        candidate_multiplier: null,
      }),
    );
    const out = JSON.parse(outJson) as { chunks: unknown[] };
    expect(Array.isArray(out.chunks)).toBe(true);
  });

  it("drops the collection", async () => {
    await store.dropCollection(COLLECTION);
    expect(await store.getCollection(COLLECTION)).toBeNull();
  });
});

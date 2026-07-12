import { describe, it, expect, beforeAll } from "vitest";
import type { XbergEngine } from "@xberg-io/xberg-wasm";
import { initializeEngine, getEngine, getRuntime } from "../src/engine.js";
import type { DocumentRecord, ChunkRecord } from "xberg-wasm-runtime";
import { EMBEDDING_DIM } from "../src/lib/constants.js";

// Task 7a: verifies the store-backed tool groups (collection.ts, document.ts,
// stats.ts, reports.ts) work end-to-end against the wasm runtime store
// reached via getRuntime().store, and that registering them no longer
// requires the native xberg-rag-node binding (which is not built in this
// worktree). Mirrors tests/ingest.test.ts's pattern of exercising the
// underlying store/engine directly plus a registration smoke test.

describe("collection/document/stats store plumbing (Task 7a)", () => {
  beforeAll(async () => {
    await initializeEngine();
  }, 180_000);

  it("create_collection path: ensureCollection succeeds and getCollection reflects the spec", async () => {
    const { store } = getRuntime();
    const err = await store.ensureCollection({ name: "c7", embedding_dim: EMBEDDING_DIM });
    expect(err).toBeUndefined();

    const spec = await store.getCollection("c7");
    expect(spec).toBeTruthy();
    expect(spec?.embedding_dim).toBe(EMBEDDING_DIM);
  }, 60_000);

  it("upsert + delete: upsertDocument returns an id string, deleteDocuments removes it", async () => {
    const { store } = getRuntime();
    await store.ensureCollection({ name: "c7_upsert", embedding_dim: EMBEDDING_DIM });

    const doc: DocumentRecord = {
      full_text: "A short test document for upsert/delete.",
      title: "Upsert test",
      keywords: [],
    };
    const chunk: ChunkRecord = {
      ordinal: 0,
      content: "A short test document for upsert/delete.",
      embedding: Array(EMBEDDING_DIM).fill(0.01),
    };

    const id = await store.upsertDocument("c7_upsert", doc, [chunk]);
    expect(typeof id).toBe("string");
    expect(id.length).toBeGreaterThan(0);

    const deleted = await store.deleteDocuments("c7_upsert", [id]);
    expect(deleted).toBe(1);
  }, 60_000);

  it("stats: collectionStats reflects a document ingested via engine.ingest", async () => {
    const { store } = getRuntime();
    await store.ensureCollection({ name: "c7_stats", embedding_dim: EMBEDDING_DIM });

    const engine: XbergEngine = getEngine();
    await engine.ingest(
      {
        full_text: "Hello world. This document is used for stats verification.",
        title: "Stats test",
        keywords: [],
        entities: {},
        labels: {},
        metadata: {},
      },
      "c7_stats"
    );

    const stats = await store.collectionStats("c7_stats");
    expect(stats.documents).toBeGreaterThanOrEqual(1);
  }, 60_000);

  it("registers collection/document/stats/report tools without throwing (no native store dependency)", async () => {
    const { McpServer } = await import("@modelcontextprotocol/sdk/server/mcp.js");
    const { registerCollectionTools } = await import("../src/tools/collection.js");
    const { registerDocumentTools } = await import("../src/tools/document.js");
    const { registerStatsTools } = await import("../src/tools/stats.js");
    const { registerReportTools } = await import("../src/tools/reports.js");

    const server = new McpServer({ name: "test", version: "0.0.0" });
    expect(() => registerCollectionTools(server)).not.toThrow();
    expect(() => registerDocumentTools(server)).not.toThrow();
    expect(() => registerStatsTools(server)).not.toThrow();
    expect(() => registerReportTools(server)).not.toThrow();
  }, 60_000);
});

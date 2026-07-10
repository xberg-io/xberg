import { describe, it, expect, beforeAll } from "vitest";
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { initializeEngine, getEngine, getRuntime } from "../src/engine.js";

// End-to-end coverage for Task 9:
//
// 1. All 13 tool groups register onto a single McpServer, exactly mirroring
//    the startup sequence in src/index.ts — proves no import failure and no
//    tool-name collision across the whole surface (extract/collection/query/
//    document/ingest/rehydrate/pii/cache/reports/stats/intelligence/media/web).
//
// 2. A cohesive wasm-stack pipeline exercised directly against the engine/
//    runtime (per the established pattern in ingest.test.ts / query.test.ts /
//    engine.test.ts — this repo's tests never invoke MCP tool handlers via a
//    client/transport for the wasm-backed groups): ingest -> query ->
//    detect_pii -> redact -> stats, each step asserting the capability
//    actually ran.
//
// Native-only groups (intelligence/media/web) only `import type` from
// `@xberg-io/xberg` at module scope, so they import and register cleanly here
// even though the native binding isn't built in this worktree. We register
// them (proving the whole 13-group surface loads together) but never call
// their handlers, since their native capability isn't available.

describe("all 13 tool groups register on one McpServer", () => {
  it("registers all 13 groups without throwing or colliding (mirrors src/index.ts)", async () => {
    const { registerExtractTools } = await import("../src/tools/extract.js");
    const { registerCollectionTools } = await import("../src/tools/collection.js");
    const { registerQueryTools } = await import("../src/tools/query.js");
    const { registerDocumentTools } = await import("../src/tools/document.js");
    const { registerIngestTools } = await import("../src/tools/ingest.js");
    const { registerRehydrateTools } = await import("../src/tools/rehydrate.js");
    const { registerPiiTools } = await import("../src/tools/pii.js");
    const { registerCacheTools } = await import("../src/tools/cache.js");
    const { registerReportTools } = await import("../src/tools/reports.js");
    const { registerStatsTools } = await import("../src/tools/stats.js");
    const { registerIntelligenceTools } = await import("../src/tools/intelligence.js");
    const { registerMediaTools } = await import("../src/tools/media.js");
    const { registerWebTools } = await import("../src/tools/web.js");

    const server = new McpServer({ name: "e2e-test-server", version: "0.0.0" });

    // Same order as src/index.ts. cache.ts and rehydrate.ts are now native-free
    // (getCacheDir moved to src/paths.ts when store.ts was removed), so all 13
    // groups import and register cleanly without the native binding.
    expect(() => {
      registerExtractTools(server);
      registerCollectionTools(server);
      registerQueryTools(server);
      registerDocumentTools(server);
      registerIngestTools(server);
      registerRehydrateTools(server);
      registerPiiTools(server);
      registerCacheTools(server);
      registerReportTools(server);
      registerStatsTools(server);
      registerIntelligenceTools(server);
      registerMediaTools(server);
      registerWebTools(server);
    }).not.toThrow();

    // A tool-name collision would make the McpServer's underlying Server
    // reject the duplicate `tool()` registration (the SDK throws on a
    // duplicate name), so getting here without throwing already proves no
    // collision. Additionally assert the registered tool count reflects a
    // real, non-trivial surface, so this assertion is not vacuous even if
    // the SDK's collision behavior changes.
    const registeredTools = Object.keys(
      (server as unknown as { _registeredTools: Record<string, unknown> })._registeredTools ?? {},
    );
    expect(registeredTools.length).toBeGreaterThanOrEqual(13);
    // Spot-check a handful of tool names from distinct groups actually landed.
    for (const name of ["extract_document", "create_collection", "query_corpus", "detect_pii", "collection_stats", "rehydrate_document"]) {
      expect(registeredTools).toContain(name);
    }
  });
});

describe("cohesive wasm pipeline: ingest -> query -> detect_pii -> redact -> stats", () => {
  const COLLECTION = "e2e_pipeline_col";
  const EMBEDDING_DIM = 384; // matches xberg-wasm-runtime's default embedder model
  const PII_TEXT = "Contact Jane Doe at jane.doe@example.com or 555-123-4567 regarding invoice #4471.";

  beforeAll(async () => {
    // First run downloads the embedder model (transformers.js) and loads the
    // ~100MB wasm binary, so allow a generous budget.
    await initializeEngine();
  }, 180_000);

  it("ensures the collection exists on the shared runtime store", async () => {
    const { store } = getRuntime();
    const result = await store.ensureCollection({ name: COLLECTION, embedding_dim: EMBEDDING_DIM });
    // Per the brief: undefined on success, a string on failure.
    if (typeof result === "string") {
      throw new Error(`ensureCollection failed: ${result}`);
    }
    expect(result).toBeUndefined();
  }, 60_000);

  it("ingests a document that actually lands in the collection", async () => {
    const doc = {
      full_text:
        "Xberg is a document extraction and RAG platform. " +
        "This document discusses machine learning, vector search, and retrieval pipelines. " +
        PII_TEXT,
      title: "E2E Pipeline Doc",
      keywords: [],
      entities: {},
      labels: {},
      metadata: {},
    };

    const documentId = await getEngine().ingest(doc, COLLECTION);
    expect(typeof documentId).toBe("string");
    expect((documentId as string).length).toBeGreaterThan(0);

    // Confirm ingestion is actually visible via the store, not just that
    // ingest() resolved without error.
    const stats = await getRuntime().store.collectionStats(COLLECTION);
    expect(stats.documents).toBeGreaterThanOrEqual(1);
    expect(stats.chunks).toBeGreaterThanOrEqual(1);
  }, 60_000);

  it("queries the collection and gets back scored chunks for the ingested content", async () => {
    const output = (await getEngine().query("machine learning retrieval", COLLECTION, 3)) as {
      mode: string;
      chunks: Array<{ score: number; primary_score: { kind: string; score?: number }; content?: string }>;
      primary_latency_ms?: number;
    };

    expect(Array.isArray(output.chunks)).toBe(true);
    expect(output.chunks.length).toBeGreaterThan(0);
    const top = output.chunks[0]!;
    expect(typeof top.score).toBe("number");
    expect(typeof top.primary_score.kind).toBe("string");
  }, 60_000);

  it("detects PII in the ingested text", async () => {
    // Brief flags detect_pii may be sync — await defensively.
    const matches = (await getEngine().detect_pii(PII_TEXT, null)) as Array<{
      start: number;
      end: number;
      category: string;
      text: string;
    }>;

    expect(Array.isArray(matches)).toBe(true);
    expect(matches.length).toBeGreaterThan(0);
    for (const m of matches) {
      expect(typeof m.start).toBe("number");
      expect(typeof m.end).toBe("number");
      expect(typeof m.category).toBe("string");
      expect(typeof m.text).toBe("string");
      expect(PII_TEXT.slice(m.start, m.end)).toBe(m.text);
    }

    const email = matches.find((m) => m.text === "jane.doe@example.com");
    expect(email).toBeDefined();
  }, 60_000);

  it("redacts the PII, producing a rehydration map that recovers the originals", async () => {
    // Brief flags redact may be sync — await defensively. rehydrationMap is a
    // JS `Map`, not a plain object.
    const result = (await getEngine().redact(PII_TEXT, "token_replace")) as {
      redacted: string;
      rehydrationMap: Map<string, string>;
    };

    expect(typeof result.redacted).toBe("string");
    expect(result.redacted).not.toBe(PII_TEXT);
    // The email must no longer appear verbatim in the redacted text.
    expect(result.redacted).not.toContain("jane.doe@example.com");

    expect(result.rehydrationMap instanceof Map).toBe(true);
    expect(result.rehydrationMap.size).toBeGreaterThan(0);

    // Every value in the rehydration map must be recoverable substring of
    // the original text, proving the map actually captures real PII spans
    // rather than being an empty/placeholder structure.
    const originals = Array.from(result.rehydrationMap.values());
    expect(originals.some((v) => PII_TEXT.includes(v))).toBe(true);
    expect(originals).toContain("jane.doe@example.com");
  }, 60_000);

  it("reports aggregate stats reflecting the ingested document", async () => {
    const stats = await getRuntime().store.collectionStats(COLLECTION);

    expect(typeof stats.documents).toBe("number");
    expect(typeof stats.chunks).toBe("number");
    expect(stats.documents).toBeGreaterThanOrEqual(1);
    expect(stats.chunks).toBeGreaterThanOrEqual(1);
    if (stats.last_ingested_at !== undefined) {
      expect(typeof stats.last_ingested_at).toBe("number");
      expect(stats.last_ingested_at).toBeGreaterThan(0);
    }
  }, 60_000);
});

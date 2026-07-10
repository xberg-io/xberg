import { bench, describe, beforeAll } from "vitest";
import { readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { initializeEngine, getEngine, getRuntime } from "../src/engine.js";
import type { XbergEngine } from "@xberg-io/xberg-wasm";

// Latency baseline for the wasm engine's hot paths (extract / ingest / query),
// for regression tracking. See docs/superpowers/results/2026-07-02-wasm-mcp-performance.md.
//
// A native (`@xberg-io/xberg` + `xberg-rag-node`) comparison arm is intentionally
// omitted: the native NAPI binding is not built in this worktree, so there is no
// in-process native path to time against. The captured numbers are the wasm
// engine's steady-state latency (the engine + model are warmed once in beforeAll,
// so per-iteration timings exclude the one-off model download/load).

const __dirname = dirname(fileURLToPath(import.meta.url));
const SAMPLE = readFileSync(join(__dirname, "..", "tests", "fixtures", "extract-sample.txt"));
const EMBEDDING_DIM = 384;
const COLLECTION = "bench_col";

const extractInput = {
  kind: "bytes" as const,
  bytes: Uint8Array.from(SAMPLE),
  mime_type: "text/plain",
  filename: "extract-sample.txt",
};
// Matches src/tools/extract.ts's minimal wasm config (see toWasmConfig): the wasm
// ExtractionConfig requires extraction_timeout_secs to be explicitly null.
const extractConfig = { extraction_timeout_secs: null };

const doc = {
  full_text: SAMPLE.toString("utf8"),
  title: "bench",
  keywords: [] as string[],
  entities: {},
  labels: {},
  metadata: {},
};

let engine: XbergEngine;

describe("wasm engine latency (steady-state, model pre-warmed)", () => {
  beforeAll(async () => {
    engine = await initializeEngine();
    const err = await getRuntime().store.ensureCollection({ name: COLLECTION, embedding_dim: EMBEDDING_DIM });
    if (typeof err === "string") throw new Error(err);
    // Seed one document so query has something to retrieve.
    await engine.ingest(doc, COLLECTION);
  }, 180_000);

  bench("extract (284-byte text/plain)", async () => {
    await engine.extract(extractInput, extractConfig);
  });

  bench("ingest (extract + embed + store)", async () => {
    await engine.ingest(doc, COLLECTION);
  });

  bench("query (embed + vector retrieve, top_k=5)", async () => {
    await engine.query("document extraction and metadata", COLLECTION, 5);
  });
});

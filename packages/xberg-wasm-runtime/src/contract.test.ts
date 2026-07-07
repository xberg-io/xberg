import { describe, it, expect } from "vitest";
import { createXbergRuntimeFactory } from "./factory";
import type {
  EmbedderInterface,
  VectorStoreInterface,
  NerInterface,
  OcrInterface,
  InjectionDescriptor,
} from "./types";

describe("injection descriptor contract", () => {
  it("factory output satisfies InjectionDescriptor type", async () => {
    const descriptor = await createXbergRuntimeFactory();

    // Type-level contract: descriptor should be assignable to InjectionDescriptor
    const _: InjectionDescriptor = descriptor;

    expect(_).toBeDefined();
  }, 60_000);

  it("embedder implements required interface", async () => {
    const descriptor = await createXbergRuntimeFactory();

    const embedder: EmbedderInterface = descriptor.embedder;
    expect(typeof embedder.embed).toBe("function");

    // Test a real call (with minimal fixture)
    const result = await embedder.embed(["test"]);
    expect(Array.isArray(result)).toBe(true);
    expect(result[0]).toBeInstanceOf(Float32Array);
  }, 60_000);

  it("store implements required interface", async () => {
    const descriptor = await createXbergRuntimeFactory();

    const store: VectorStoreInterface = descriptor.store;
    expect(typeof store.upsertDocument).toBe("function");
    expect(typeof store.query).toBe("function");
    expect(typeof store.delete).toBe("function");
    expect(typeof store.listCollections).toBe("function");
    expect(typeof store.dropCollection).toBe("function");
    expect(typeof store.ensureCollection).toBe("function");

    // Test a real round-trip
    await store.ensureCollection("test", 384);
    const collections = await store.listCollections();
    expect(collections).toContain("test");
  }, 60_000);

  it("ner (if present) implements required interface", async () => {
    const descriptor = await createXbergRuntimeFactory();

    if (descriptor.ner) {
      const ner: NerInterface = descriptor.ner;
      expect(typeof ner.ner).toBe("function");

      const result = await ner.ner("test text");
      expect(Array.isArray(result)).toBe(true);
    }
  }, 60_000);

  it("ocr (if present) implements required interface", async () => {
    const descriptor = await createXbergRuntimeFactory();

    if (descriptor.ocr) {
      const ocr: OcrInterface = descriptor.ocr;
      expect(typeof ocr.ocr).toBe("function");
    }
  }, 60_000);
});

// Smoke test pattern (requires xberg-wasm built):
// import { XbergEngine } from "xberg-wasm";  // would import from actual wasm binary
// const engine = new XbergEngine(config, await createXbergRuntimeFactory());
// await engine.ingest(doc, "collection");
// const results = await engine.query("query", "collection", 10);
// This test is deferred until xberg-wasm is built and published.

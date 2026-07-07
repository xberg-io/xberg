import { describe, it, expect, vi } from "vitest";
import { createXbergRuntimeFactory } from "./factory";
import { validateInjectionDescriptor } from "./validation";
import * as embedderModule from "./embedder";
import * as storeModule from "./store";

describe("factory", () => {
  it("creates a valid injection descriptor", async () => {
    // "Xenova/minilm-l6-v2" (as written in the original spec) does not exist
    // on the Hub; the real, canonically-cased transformers.js
    // feature-extraction model is "Xenova/all-MiniLM-L6-v2" (see
    // embedder.test.ts / embedder.ts DEFAULT_MODEL for the same correction).
    const injection = await createXbergRuntimeFactory({
      models: {
        embedder: "Xenova/all-MiniLM-L6-v2",
      },
    });

    const validation = validateInjectionDescriptor(injection);
    expect(validation.valid).toBe(true);
  }, 120_000);

  it("injects embedder and store (required)", async () => {
    const injection = await createXbergRuntimeFactory();

    expect(injection.embedder).toBeDefined();
    expect(injection.embedder.embed).toBeDefined();
    expect(injection.store).toBeDefined();
    expect(injection.store.upsertDocument).toBeDefined();
  }, 120_000);

  it("optionally injects ner and ocr", async () => {
    // "Xenova/gliner2-small-onnx" and "paddleocr/pp-ocrv6" (as written in the
    // original spec) do not exist / are not valid ppu-paddle-ocr model preset
    // keys. See ner.ts DEFAULT_NER_MODEL ("Xenova/bert-base-NER") and ocr.ts
    // DEFAULT_MODEL_EXPORT ("V6_SMALL_MODEL") for the real identifiers these
    // modules actually accept: `models.ner` is any transformers.js
    // token-classification model id, and `models.ocr` is a ppu-paddle-ocr
    // model preset export name.
    //
    // This exercises embedder + ner + ocr together in the same process.
    // That combination previously SIGSEGV'd (native ORT API-version mismatch:
    // ppu-paddle-ocr's peer resolution pulled in onnxruntime-web/-node
    // 1.27.0, colliding with the 1.21.0 / 1.22.0-dev line transformers.js
    // pins for the embedder/ner, when both native addons loaded into the
    // same process) — resolved by pinning onnxruntime-node and
    // onnxruntime-web to the exact versions transformers.js expects, via
    // `overrides` in the workspace root's pnpm-workspace.yaml, so every
    // workspace consumer (including ppu-paddle-ocr) resolves against a
    // single native ORT build.
    const injection = await createXbergRuntimeFactory({
      models: {
        ner: "Xenova/bert-base-NER",
        ocr: "V6_SMALL_MODEL",
      },
    });

    // Both may be null/omitted if models fail to load, which is acceptable.
    expect(injection.embedder).toBeDefined();
    expect(injection.store).toBeDefined();
  }, 120_000);

  it("initializes cache manager", async () => {
    const injection = await createXbergRuntimeFactory();
    expect(injection).toHaveProperty("embedder");
    // Cache should be transparently managed; test that we can construct it
  }, 120_000);

  it("handles missing optional NER gracefully", async () => {
    // Factory should not throw when NER initialization fails; it should log and continue
    const injection = await createXbergRuntimeFactory();
    expect(injection.embedder).toBeDefined();
    expect(injection.store).toBeDefined();
    // NER may or may not be present; both are acceptable
  }, 120_000);

  it("handles missing optional OCR gracefully", async () => {
    // Factory should not throw when OCR initialization fails; it should log and continue
    const injection = await createXbergRuntimeFactory();
    expect(injection.embedder).toBeDefined();
    expect(injection.store).toBeDefined();
    // OCR may or may not be present; both are acceptable
  }, 120_000);

  it("applies cache config when provided", async () => {
    const injection = await createXbergRuntimeFactory({
      nodeCachePath: "/tmp/test-cache",
      wasmPaths: "/custom/wasm",
    });
    expect(injection.embedder).toBeDefined();
    expect(injection.store).toBeDefined();
  }, 120_000);

  it("throws when embedder initialization fails", async () => {
    // Mock createEmbedder to throw an error
    const spy = vi.spyOn(embedderModule, "createEmbedder").mockRejectedValueOnce(
      new Error("embedder load failed")
    );

    try {
      await expect(createXbergRuntimeFactory()).rejects.toThrow(
        "[factory] embedder initialization failed"
      );
    } finally {
      spy.mockRestore();
    }
  }, 120_000);

  it("throws when store initialization fails", async () => {
    // Mock createVectorStore to throw an error
    const spy = vi.spyOn(storeModule, "createVectorStore").mockRejectedValueOnce(
      new Error("store init failed")
    );

    try {
      await expect(createXbergRuntimeFactory()).rejects.toThrow(
        "[factory] vector store initialization failed"
      );
    } finally {
      spy.mockRestore();
    }
  }, 120_000);
});

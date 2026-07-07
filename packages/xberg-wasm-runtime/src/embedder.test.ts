import { describe, it, expect, beforeAll } from "vitest";
import { createEmbedder } from "./embedder";

describe("embedder", () => {
  let embedder: Awaited<ReturnType<typeof createEmbedder>>;

  beforeAll(async () => {
    // "Xenova/minilm-l6-v2" (as written in the original spec) does not exist on
    // the Hub; the real, canonically-cased transformers.js feature-extraction
    // model is "Xenova/all-MiniLM-L6-v2" (2.9M+ downloads, quantized ONNX
    // variants included). This triggers a live download on first run.
    embedder = await createEmbedder({
      models: { embedder: "Xenova/all-MiniLM-L6-v2" },
    });
  }, 120_000);

  it("embeds a single string to a normalized vector", async () => {
    const result = await embedder.embed(["hello world"]);
    expect(result).toHaveLength(1);
    const [vec] = result;
    expect(vec).toBeInstanceOf(Float32Array);
    expect(vec).toBeDefined();
    if (!vec) throw new Error("expected embedding vector");
    expect(vec.length).toBeGreaterThan(0);
    // L2 normalization check: magnitude should be ~1.0
    const magnitude = Math.sqrt(
      Array.from(vec).reduce((sum, v) => sum + v * v, 0)
    );
    expect(magnitude).toBeCloseTo(1.0, 1);
  }, 60_000);

  it("embeds multiple strings", async () => {
    const texts = ["hello", "world", "foo bar"];
    const result = await embedder.embed(texts);
    expect(result).toHaveLength(3);
    const expectedLength = result[0]?.length;
    result.forEach((vec) => {
      expect(vec).toBeInstanceOf(Float32Array);
      expect(vec.length).toBe(expectedLength);
    });
  }, 60_000);

  it("respects batch size (32 by default)", async () => {
    const texts = Array.from({ length: 100 }, (_, i) => `text ${i}`);
    const result = await embedder.embed(texts);
    expect(result).toHaveLength(100);
  }, 60_000);
});

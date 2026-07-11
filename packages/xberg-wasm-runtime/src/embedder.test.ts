import { describe, it, expect, beforeAll } from "vitest";
import { createEmbedder } from "./embedder";

describe("embedder", () => {
	let embedder: Awaited<ReturnType<typeof createEmbedder>>;

	beforeAll(async () => {
		// Xenova/bge-m3 — verified real, live transformers.js-compatible ONNX
		// export of BAAI/bge-m3 (1024-dim, multilingual) via
		// scripts/verify-embedding-model.mjs. Replaces the earlier
		// Xenova/all-MiniLM-L6-v2 default, which was never a deliberate quality
		// choice (substituted only because the original plan's test model ID
		// didn't exist on the Hub).
		embedder = await createEmbedder({
			models: { embedder: "Xenova/bge-m3" },
		});
	}, 180_000);

	it("embeds a single string to a normalized vector", async () => {
		const result = await embedder.embed(["hello world"]);
		expect(result).toHaveLength(1);
		const [vec] = result;
		expect(vec).toBeInstanceOf(Float32Array);
		expect(vec).toBeDefined();
		if (!vec) throw new Error("expected embedding vector");
		expect(vec.length).toBeGreaterThan(0);
		// L2 normalization check: magnitude should be ~1.0
		const magnitude = Math.sqrt(Array.from(vec).reduce((sum, v) => sum + v * v, 0));
		expect(magnitude).toBeCloseTo(1.0, 1);
	}, 60_000);

	it("produces 1024-dimensional vectors (bge-m3)", async () => {
		const result = await embedder.embed(["dimension check"]);
		expect(result[0]?.length).toBe(1024);
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

	it("returns a cached result for identical text without re-invoking the model", async () => {
		const freshEmbedder = await createEmbedder();
		const texts = ["cache me please"];

		const first = await freshEmbedder.embed(texts);
		const second = await freshEmbedder.embed(texts);

		// Cache hits return the exact stored vector object; recomputation creates a new one.
		expect(second[0]).toBe(first[0]);
	}, 60_000);
});

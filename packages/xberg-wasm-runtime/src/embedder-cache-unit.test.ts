import { beforeEach, describe, expect, it, vi } from "vitest";

const extractor = vi.hoisted(() =>
	vi.fn(async (texts: string[] = []) => ({
		dims: [texts.length, 2],
		data: texts.flatMap(() => [1, 0]),
	})),
);

vi.mock("@huggingface/transformers", () => ({
	env: {},
	pipeline: vi.fn(async () => extractor),
}));

import { createEmbedder } from "./embedder.js";

describe("bounded embedding cache", () => {
	beforeEach(() => extractor.mockClear());

	it("evicts the least-recently-used entry after the maximum size", async () => {
		const embedder = await createEmbedder();
		extractor.mockClear();
		const texts = Array.from({ length: 1_025 }, (_, index) => `text-${index}`);
		await embedder.embed(texts);
		const callsAfterFill = extractor.mock.calls.length;

		await embedder.embed([texts[0]!]);
		expect(extractor).toHaveBeenCalledTimes(callsAfterFill + 1);

		await embedder.embed([texts.at(-1)!]);
		expect(extractor).toHaveBeenCalledTimes(callsAfterFill + 1);
	});
});

import { beforeEach, describe, expect, it, vi } from "vitest";

const { createEmbedder, createNer, createOcr } = vi.hoisted(() => ({
	createEmbedder: vi.fn(),
	createNer: vi.fn(),
	createOcr: vi.fn(),
}));

vi.mock("./embedder.js", () => ({ createEmbedder }));
vi.mock("./ner.js", () => ({ createNer }));
vi.mock("./ocr.js", () => ({ createOcr }));

import { CacheManager } from "./cache.js";

describe("CacheManager.warm pipeline selection", () => {
	beforeEach(() => {
		vi.clearAllMocks();
		createEmbedder.mockResolvedValue({ embed: vi.fn() });
		createNer.mockResolvedValue({ ner: vi.fn() });
		createOcr.mockResolvedValue({ ocr: vi.fn() });
	});

	it("warms every pipeline in a deterministic order", async () => {
		const phases: string[] = [];

		await expect(new CacheManager("C:/cache").warm({ onProgress: (phase) => phases.push(phase) })).resolves.toEqual(
			{ success: ["embedding", "ner", "ocr"], failed: [] },
		);
		expect(phases).toEqual(["embedding", "ner", "ocr"]);
	});

	it("maps legacy names, ignores unknown names, and de-duplicates handles", async () => {
		await expect(
			new CacheManager("C:/cache").warm(["Embedder (minilm-l6-v2)", "Embedder (all-MiniLM-L6-v2)", "unknown"]),
		).resolves.toEqual({ success: ["embedding"], failed: [] });
		expect(createEmbedder).toHaveBeenCalledOnce();
		expect(createNer).not.toHaveBeenCalled();
		expect(createOcr).not.toHaveBeenCalled();
	});

	it.each([
		["embedding", () => createEmbedder.mockRejectedValueOnce(new Error("embed failed"))],
		["ner", () => createNer.mockResolvedValueOnce(null)],
		["ocr", () => createOcr.mockResolvedValueOnce(null)],
	] as const)("reports a failed %s pipeline", async (name, arrangeFailure) => {
		arrangeFailure();

		await expect(
			new CacheManager("C:/cache").warm({
				modelNames: [
					name === "embedding" ? "Embedder (all-MiniLM-L6-v2)" : name === "ner" ? "BERT NER" : "PP-OCRv6 OCR",
				],
			}),
		).resolves.toEqual({ success: [], failed: [name] });
	});
});

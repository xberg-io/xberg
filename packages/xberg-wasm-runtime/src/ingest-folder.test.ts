import { describe, it, expect } from "vitest";
import { ingestFolder, type XbergEngineLike } from "./ingest-folder.js";

function makeStubEngine(behavior: {
	extractText?: (filename: string) => string | null;
	failIngestFor?: string[];
}): XbergEngineLike {
	return {
		extract: async (input) => {
			const filename = input.filename ?? "";
			const text = behavior.extractText ? behavior.extractText(filename) : "stub content";
			if (text === null) return { results: [] };
			return { results: [{ content: text, mimeType: "text/plain" }] };
		},
		ingest: async (doc, _collection) => {
			if (behavior.failIngestFor?.includes(doc.title ?? "")) {
				throw new Error("simulated ingest failure");
			}
			return {
				document_id: `doc-${doc.title}`,
				rehydration_map: doc.full_text.includes("Alice") ? { "[PERSON_1]": "Alice" } : {},
				pii_category_counts: doc.full_text.includes("Alice") ? { Person: 1 } : {},
			};
		},
	};
}

describe("ingestFolder", () => {
	it("ingests every file and returns per-file results", async () => {
		const engine = makeStubEngine({ extractText: (name) => `Text from ${name}` });
		const files = [
			{ name: "a.txt", path: "/src/a.txt", bytes: new Uint8Array([1, 2, 3]) },
			{ name: "b.txt", path: "/src/b.txt", bytes: new Uint8Array([4, 5, 6]) },
		];

		const results = await ingestFolder(engine, "docs", files);

		expect(results).toHaveLength(2);
		expect(results[0]).toMatchObject({ filename: "a.txt", documentId: "doc-a.txt" });
		expect(results[1]).toMatchObject({ filename: "b.txt", documentId: "doc-b.txt" });
	});

	it("surfaces PII category counts and rehydration map per file", async () => {
		const engine = makeStubEngine({ extractText: (name) => (name === "alice.txt" ? "Hi Alice" : "no pii here") });
		const files = [{ name: "alice.txt", path: "/src/alice.txt", bytes: new Uint8Array([1]) }];

		const results = await ingestFolder(engine, "docs", files);

		expect(results[0]?.piiCategoryCounts).toEqual({ Person: 1 });
		expect(results[0]?.rehydrationMap).toEqual({ "[PERSON_1]": "Alice" });
	});

	it("records a per-file error and continues the batch when one file fails", async () => {
		const engine = makeStubEngine({ extractText: () => "content", failIngestFor: ["bad.txt"] });
		const files = [
			{ name: "bad.txt", path: "/src/bad.txt", bytes: new Uint8Array([1]) },
			{ name: "good.txt", path: "/src/good.txt", bytes: new Uint8Array([2]) },
		];

		const results = await ingestFolder(engine, "docs", files);

		expect(results).toHaveLength(2);
		expect(results[0]).toMatchObject({ filename: "bad.txt", documentId: null, error: "simulated ingest failure" });
		expect(results[1]).toMatchObject({ filename: "good.txt", documentId: "doc-good.txt" });
	});

	it("records an error when extraction produces no document", async () => {
		const engine = makeStubEngine({ extractText: () => null });
		const files = [{ name: "empty.bin", path: "/src/empty.bin", bytes: new Uint8Array([]) }];

		const results = await ingestFolder(engine, "docs", files);

		expect(results[0]).toMatchObject({ filename: "empty.bin", documentId: null });
		expect(results[0]?.error).toMatch(/no document/);
	});
});

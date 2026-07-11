import { describe, expect, it, vi } from "vitest";
import { cacheConfigSchema, validateInjectionDescriptor } from "./validation.js";

describe("validateInjectionDescriptor", () => {
	it("returns a validated descriptor", () => {
		const method = vi.fn(async () => undefined);
		const result = validateInjectionDescriptor({
			embedder: { embed: method },
			store: {
				ensureCollection: method,
				dropCollection: method,
				getCollection: method,
				upsertDocument: method,
				deleteDocuments: method,
				deleteByFilter: method,
				retrieve: method,
				collectionStats: method,
			},
		});

		expect(result.valid).toBe(true);
	});

	it("returns validation details for malformed injection objects", () => {
		const result = validateInjectionDescriptor({ embedder: {} });

		expect(result.valid).toBe(false);
		if (!result.valid) expect(result.error.length).toBeGreaterThan(0);
	});
});

describe("cacheConfigSchema", () => {
	it("accepts an explicit Node SQLite store path", () => {
		expect(cacheConfigSchema.safeParse({ nodeStorePath: "C:/xberg/store.sqlite3" }).success).toBe(true);
	});
});

import { describe, expect, it, vi } from "vitest";

const close = vi.hoisted(() => vi.fn());
const load = vi.hoisted(() =>
	vi.fn(() => {
		throw new Error("extension load failed");
	}),
);

vi.mock("better-sqlite3", () => ({
	default: class {
		close = close;
	},
}));
vi.mock("sqlite-vec", () => ({ load }));

import { createNodeVectorStore } from "./store-node.js";

describe("Node store initialization cleanup", () => {
	it("closes the database when extension initialization fails", async () => {
		await expect(createNodeVectorStore({ nodeStorePath: ":memory:" })).rejects.toThrow("extension load failed");
		expect(close).toHaveBeenCalledOnce();
	});
});

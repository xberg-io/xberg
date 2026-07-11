import { describe, it, expect } from "vitest";
import { join } from "node:path";
import { resolveMapPath } from "../src/tools/rehydrate-paths.js";

// resolveMapPath lives in its own module (no wasm-engine import), unlike the
// rest of rehydrate.ts's tools (which need initializeEngine() — blocked in
// environments without a built wasm binary, per rehydration_compat.test.ts).
// This lets the path-traversal fix get real regression coverage that actually
// runs everywhere, instead of only inside the wasm-gated e2e suite.
describe("resolveMapPath", () => {
	it("joins a plain document_id under dir", () => {
		expect(resolveMapPath("/cache/rehydration", "doc-123")).toBe(join("/cache/rehydration", "doc-123.map"));
	});

	it("allows letters, digits, dots, underscores, and dashes", () => {
		expect(() => resolveMapPath("/cache/rehydration", "My_Doc-v2.final")).not.toThrow();
	});

	it("rejects a relative traversal segment", () => {
		expect(() => resolveMapPath("/cache/rehydration", "../../etc/passwd")).toThrow(/invalid document_id/);
	});

	it("rejects a bare '..' component", () => {
		expect(() => resolveMapPath("/cache/rehydration", "..")).toThrow(/invalid document_id/);
	});

	it("rejects a bare '.' component", () => {
		expect(() => resolveMapPath("/cache/rehydration", ".")).toThrow(/invalid document_id/);
	});

	it("rejects a forward-slash-embedded id", () => {
		expect(() => resolveMapPath("/cache/rehydration", "sub/dir")).toThrow(/invalid document_id/);
	});

	it("rejects a backslash-embedded id", () => {
		expect(() => resolveMapPath("/cache/rehydration", "sub\\dir")).toThrow(/invalid document_id/);
	});

	it("rejects an id that looks like an absolute path", () => {
		expect(() => resolveMapPath("/cache/rehydration", "/etc/passwd")).toThrow(/invalid document_id/);
	});

	it("rejects an empty id", () => {
		expect(() => resolveMapPath("/cache/rehydration", "")).toThrow(/invalid document_id/);
	});
});

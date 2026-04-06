// Auto-generated from fixtures/plugin_api/ - DO NOT EDIT
/**
 * E2E tests for plugin/config/utility APIs.
 *
 * Generated from plugin API fixtures.
 * To regenerate: cargo run -p kreuzberg-e2e-generator -- generate --lang wasm
 */

import { beforeAll, describe, expect, it } from "vitest";
import {
	clearOcrBackends,
	clearPostProcessors,
	clearValidators,
	detectMimeFromBytes,
	getExtensionsForMime,
	initWasm,
	listOcrBackends,
	listPostProcessors,
	listValidators,
	unregisterOcrBackend,
} from "@kreuzberg/wasm";

beforeAll(async () => {
	try {
		await initWasm();
	} catch (e) {
		console.warn("WASM init failed:", e);
	}
});

describe("Configuration", () => {
	it.skip("Discover configuration from current or parent directories", () => {});

	it.skip("Load configuration from a TOML file", () => {});
});

describe("Document Extractor Management", () => {
	it.skip("Clear all document extractors and verify list is empty", () => {});

	it.skip("List all registered document extractors", () => {});

	it.skip("Unregister nonexistent document extractor gracefully", () => {});
});

describe("Mime Utilities", () => {
	it("Detect MIME type from file bytes", () => {
		const testData = new TextEncoder().encode("%PDF-1.4\\n");
		const result = detectMimeFromBytes(testData);
		expect(result.toLowerCase().includes("pdf")).toBe(true);
	});

	it.skip("Detect MIME type from file path", () => {});

	it("Get file extensions for a MIME type", () => {
		const result = getExtensionsForMime("application/pdf");
		expect(Array.isArray(result)).toBe(true);
		expect(result.includes("pdf")).toBe(true);
	});
});

describe("Ocr Backend Management", () => {
	it("Clear all OCR backends and verify list is empty", () => {
		clearOcrBackends();
		const result = listOcrBackends();
		expect(result.length).toBe(0);
	});

	it("List all registered OCR backends", () => {
		const result = listOcrBackends();
		expect(Array.isArray(result)).toBe(true);
		expect(result.every((item: unknown) => typeof item === "string")).toBe(true);
	});

	it("Unregister nonexistent OCR backend gracefully", () => {
		unregisterOcrBackend("nonexistent-backend-xyz");
	});
});

describe("Post Processor Management", () => {
	it("Clear all post-processors and verify list is empty", () => {
		clearPostProcessors();
	});

	it("List all registered post-processors", () => {
		const result = listPostProcessors();
		expect(Array.isArray(result)).toBe(true);
		expect(result.every((item: unknown) => typeof item === "string")).toBe(true);
	});
});

describe("Validator Management", () => {
	it("Clear all validators and verify list is empty", () => {
		clearValidators();
		const result = listValidators();
		expect(result.length).toBe(0);
	});

	it("List all registered validators", () => {
		const result = listValidators();
		expect(Array.isArray(result)).toBe(true);
		expect(result.every((item: unknown) => typeof item === "string")).toBe(true);
	});
});

// Auto-generated tests for structured fixtures.

import { describe, it, expect } from "vitest";
import { extractBytes } from "@kreuzberg/wasm";
import { assertions, buildConfig, getFixture, shouldSkipFixture } from "./helpers.js";
import type { ExtractionResult } from "@kreuzberg/wasm";

describe("structured", () => {
	it("structured_json_basic", async () => {
		const documentBytes = getFixture("json/sample_document.json");
		if (documentBytes === null) {
			console.warn("[SKIP] Test skipped: fixture not available in Cloudflare Workers environment");
			return;
		}

		const config = buildConfig(undefined);
		let result: ExtractionResult | null = null;
		try {
			result = await extractBytes(documentBytes, "application/json", config);
		} catch (error) {
			if (shouldSkipFixture(error, "structured_json_basic", [], undefined)) {
				return;
			}
			throw error;
		}
		if (result === null) {
			return;
		}
		assertions.assertExpectedMime(result, ["application/json"]);
		assertions.assertMinContentLength(result, 20);
		assertions.assertContentContainsAny(result, ["Sample Document", "Test Author"]);
	});

	it("structured_json_simple", async () => {
		const documentBytes = getFixture("data_formats/simple.json");
		if (documentBytes === null) {
			console.warn("[SKIP] Test skipped: fixture not available in Cloudflare Workers environment");
			return;
		}

		const config = buildConfig(undefined);
		let result: ExtractionResult | null = null;
		try {
			result = await extractBytes(documentBytes, "application/json", config);
		} catch (error) {
			if (shouldSkipFixture(error, "structured_json_simple", [], undefined)) {
				return;
			}
			throw error;
		}
		if (result === null) {
			return;
		}
		assertions.assertExpectedMime(result, ["application/json"]);
		assertions.assertMinContentLength(result, 10);
		assertions.assertContentContainsAny(result, ["{", "name"]);
	});

	it("structured_yaml_simple", async () => {
		const documentBytes = getFixture("data_formats/simple.yaml");
		if (documentBytes === null) {
			console.warn("[SKIP] Test skipped: fixture not available in Cloudflare Workers environment");
			return;
		}

		const config = buildConfig(undefined);
		let result: ExtractionResult | null = null;
		try {
			result = await extractBytes(documentBytes, "application/x-yaml", config);
		} catch (error) {
			if (shouldSkipFixture(error, "structured_yaml_simple", [], undefined)) {
				return;
			}
			throw error;
		}
		if (result === null) {
			return;
		}
		assertions.assertExpectedMime(result, ["application/x-yaml"]);
		assertions.assertMinContentLength(result, 10);
	});
});

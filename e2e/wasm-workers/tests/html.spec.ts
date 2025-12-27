// Auto-generated tests for html fixtures.

import { describe, it, expect } from "vitest";
import { extractBytes } from "@kreuzberg/wasm";
import { assertions, buildConfig, getFixture, shouldSkipFixture } from "./helpers.js";
import type { ExtractionResult } from "@kreuzberg/wasm";

describe("html", () => {
	it("html_complex_layout", async () => {
		const documentBytes = getFixture("web/taylor_swift.html");
		if (documentBytes === null) {
			console.warn("[SKIP] Test skipped: fixture not available in Cloudflare Workers environment");
			return;
		}

		const config = buildConfig(undefined);
		let result: ExtractionResult | null = null;
		try {
			result = await extractBytes(documentBytes, "text/html", config);
		} catch (error) {
			if (shouldSkipFixture(error, "html_complex_layout", [], undefined)) {
				return;
			}
			throw error;
		}
		if (result === null) {
			return;
		}
		assertions.assertExpectedMime(result, ["text/html"]);
		assertions.assertMinContentLength(result, 1000);
	});

	it("html_simple_table", async () => {
		const documentBytes = getFixture("web/simple_table.html");
		if (documentBytes === null) {
			console.warn("[SKIP] Test skipped: fixture not available in Cloudflare Workers environment");
			return;
		}

		const config = buildConfig(undefined);
		let result: ExtractionResult | null = null;
		try {
			result = await extractBytes(documentBytes, "text/html", config);
		} catch (error) {
			if (shouldSkipFixture(error, "html_simple_table", [], undefined)) {
				return;
			}
			throw error;
		}
		if (result === null) {
			return;
		}
		assertions.assertExpectedMime(result, ["text/html"]);
		assertions.assertMinContentLength(result, 100);
		assertions.assertContentContainsAll(result, [
			"Product",
			"Category",
			"Price",
			"Stock",
			"Laptop",
			"Electronics",
			"Sample Data Table",
		]);
	});
});

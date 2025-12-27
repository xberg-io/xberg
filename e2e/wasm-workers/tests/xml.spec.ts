// Auto-generated tests for xml fixtures.

import { describe, it, expect } from "vitest";
import { extractBytes } from "@kreuzberg/wasm";
import { assertions, buildConfig, getFixture, shouldSkipFixture } from "./helpers.js";
import type { ExtractionResult } from "@kreuzberg/wasm";

describe("xml", () => {
	it("xml_plant_catalog", async () => {
		const documentBytes = getFixture("xml/plant_catalog.xml");
		if (documentBytes === null) {
			console.warn("[SKIP] Test skipped: fixture not available in Cloudflare Workers environment");
			return;
		}

		const config = buildConfig(undefined);
		let result: ExtractionResult | null = null;
		try {
			result = await extractBytes(documentBytes, "application/xml", config);
		} catch (error) {
			if (shouldSkipFixture(error, "xml_plant_catalog", [], undefined)) {
				return;
			}
			throw error;
		}
		if (result === null) {
			return;
		}
		assertions.assertExpectedMime(result, ["application/xml"]);
		assertions.assertMinContentLength(result, 100);
	});
});

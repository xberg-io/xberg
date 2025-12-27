// Auto-generated tests for smoke fixtures.

import { describe, it, expect } from "vitest";
import { extractBytes } from "@kreuzberg/wasm";
import { assertions, buildConfig, getFixture, shouldSkipFixture } from "./helpers.js";
import type { ExtractionResult } from "@kreuzberg/wasm";

describe("smoke", () => {
	it("smoke_docx_basic", async () => {
		const documentBytes = getFixture("documents/fake.docx");
		if (documentBytes === null) {
			console.warn("[SKIP] Test skipped: fixture not available in Cloudflare Workers environment");
			return;
		}

		const config = buildConfig(undefined);
		let result: ExtractionResult | null = null;
		try {
			result = await extractBytes(
				documentBytes,
				"application/vnd.openxmlformats-officedocument.wordprocessingml.document",
				config,
			);
		} catch (error) {
			if (shouldSkipFixture(error, "smoke_docx_basic", [], undefined)) {
				return;
			}
			throw error;
		}
		if (result === null) {
			return;
		}
		assertions.assertExpectedMime(result, ["application/vnd.openxmlformats-officedocument.wordprocessingml.document"]);
		assertions.assertMinContentLength(result, 20);
		assertions.assertContentContainsAny(result, ["Lorem", "ipsum", "document", "text"]);
	});

	it("smoke_html_basic", async () => {
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
			if (shouldSkipFixture(error, "smoke_html_basic", [], undefined)) {
				return;
			}
			throw error;
		}
		if (result === null) {
			return;
		}
		assertions.assertExpectedMime(result, ["text/html"]);
		assertions.assertMinContentLength(result, 10);
		assertions.assertContentContainsAny(result, ["#", "**", "simple", "HTML"]);
	});

	it("smoke_image_png", async () => {
		const documentBytes = getFixture("images/sample.png");
		if (documentBytes === null) {
			console.warn("[SKIP] Test skipped: fixture not available in Cloudflare Workers environment");
			return;
		}

		const config = buildConfig(undefined);
		let result: ExtractionResult | null = null;
		try {
			result = await extractBytes(documentBytes, "image/png", config);
		} catch (error) {
			if (shouldSkipFixture(error, "smoke_image_png", [], "Image extraction requires image processing dependencies")) {
				return;
			}
			throw error;
		}
		if (result === null) {
			return;
		}
		assertions.assertExpectedMime(result, ["image/png"]);
		assertions.assertMetadataExpectation(result, "format", "PNG");
	});

	it("smoke_json_basic", async () => {
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
			if (shouldSkipFixture(error, "smoke_json_basic", [], undefined)) {
				return;
			}
			throw error;
		}
		if (result === null) {
			return;
		}
		assertions.assertExpectedMime(result, ["application/json"]);
		assertions.assertMinContentLength(result, 5);
	});

	it("smoke_pdf_basic", async () => {
		const documentBytes = getFixture("pdfs/fake_memo.pdf");
		if (documentBytes === null) {
			console.warn("[SKIP] Test skipped: fixture not available in Cloudflare Workers environment");
			return;
		}

		const config = buildConfig(undefined);
		let result: ExtractionResult | null = null;
		try {
			result = await extractBytes(documentBytes, "application/pdf", config);
		} catch (error) {
			if (shouldSkipFixture(error, "smoke_pdf_basic", [], undefined)) {
				return;
			}
			throw error;
		}
		if (result === null) {
			return;
		}
		assertions.assertExpectedMime(result, ["application/pdf"]);
		assertions.assertMinContentLength(result, 50);
		assertions.assertContentContainsAny(result, ["May 5, 2023", "To Whom it May Concern"]);
	});

	it("smoke_txt_basic", async () => {
		const documentBytes = getFixture("text/report.txt");
		if (documentBytes === null) {
			console.warn("[SKIP] Test skipped: fixture not available in Cloudflare Workers environment");
			return;
		}

		const config = buildConfig(undefined);
		let result: ExtractionResult | null = null;
		try {
			result = await extractBytes(documentBytes, "text/plain", config);
		} catch (error) {
			if (shouldSkipFixture(error, "smoke_txt_basic", [], undefined)) {
				return;
			}
			throw error;
		}
		if (result === null) {
			return;
		}
		assertions.assertExpectedMime(result, ["text/plain"]);
		assertions.assertMinContentLength(result, 5);
	});

	it("smoke_xlsx_basic", async () => {
		const documentBytes = getFixture("spreadsheets/stanley_cups.xlsx");
		if (documentBytes === null) {
			console.warn("[SKIP] Test skipped: fixture not available in Cloudflare Workers environment");
			return;
		}

		const config = buildConfig(undefined);
		let result: ExtractionResult | null = null;
		try {
			result = await extractBytes(
				documentBytes,
				"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
				config,
			);
		} catch (error) {
			if (shouldSkipFixture(error, "smoke_xlsx_basic", [], undefined)) {
				return;
			}
			throw error;
		}
		if (result === null) {
			return;
		}
		assertions.assertExpectedMime(result, ["application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"]);
		assertions.assertMinContentLength(result, 100);
		assertions.assertContentContainsAll(result, [
			"Team",
			"Location",
			"Stanley Cups",
			"Blues",
			"Flyers",
			"Maple Leafs",
			"STL",
			"PHI",
			"TOR",
		]);
		assertions.assertTableCount(result, 1, null);
		assertions.assertMetadataExpectation(result, "sheet_count", { gte: 2 });
		assertions.assertMetadataExpectation(result, "sheet_names", { contains: ["Stanley Cups"] });
	});
});

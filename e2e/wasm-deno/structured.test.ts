// Auto-generated tests for structured fixtures.
// Run with: deno test --allow-read

import { assertions, buildConfig, extractBytes, initWasm, resolveDocument, shouldSkipFixture } from "./helpers.ts";
import type { ExtractionResult } from "./helpers.ts";

// Initialize WASM module once at module load time
await initWasm();

Deno.test("structured_json_basic", { permissions: { read: true } }, async () => {
	const documentBytes = await resolveDocument("json/sample_document.json");
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

Deno.test("structured_json_simple", { permissions: { read: true } }, async () => {
	const documentBytes = await resolveDocument("data_formats/simple.json");
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

Deno.test("structured_yaml_simple", { permissions: { read: true } }, async () => {
	const documentBytes = await resolveDocument("data_formats/simple.yaml");
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

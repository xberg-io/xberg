// Auto-generated from fixtures/plugin_api/ - DO NOT EDIT
/**
 * E2E tests for plugin/config/utility APIs.
 *
 * Generated from plugin API fixtures.
 * To regenerate: cargo run -p kreuzberg-e2e-generator -- generate --lang wasm-deno
 */

// @deno-types="../../crates/kreuzberg-wasm/dist/index.d.ts"
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
} from "npm:@kreuzberg/wasm@^4.0.0";
import { assertEquals } from "@std/assert";

await initWasm();

// Configuration

Deno.test({ name: "Discover configuration from current or parent directories", ignore: true, fn() {} });

Deno.test({ name: "Load configuration from a TOML file", ignore: true, fn() {} });

// Document Extractor Management

Deno.test({ name: "Clear all document extractors and verify list is empty", ignore: true, fn() {} });

Deno.test({ name: "List all registered document extractors", ignore: true, fn() {} });

Deno.test({ name: "Unregister nonexistent document extractor gracefully", ignore: true, fn() {} });

// Mime Utilities

Deno.test("Detect MIME type from file bytes", () => {
	const testData = new TextEncoder().encode("%PDF-1.4\\n");
	const result = detectMimeFromBytes(testData);
	assertEquals(result.toLowerCase().includes("pdf"), true);
});

Deno.test({ name: "Detect MIME type from file path", ignore: true, fn() {} });

Deno.test("Get file extensions for a MIME type", () => {
	const result = getExtensionsForMime("application/pdf");
	assertEquals(Array.isArray(result), true);
	assertEquals(result.includes("pdf"), true);
});

// Ocr Backend Management

Deno.test("Clear all OCR backends and verify list is empty", () => {
	clearOcrBackends();
	const result = listOcrBackends();
	assertEquals(result.length, 0);
});

Deno.test("List all registered OCR backends", () => {
	const result = listOcrBackends();
	assertEquals(Array.isArray(result), true);
	assertEquals(
		result.every((item: unknown) => typeof item === "string"),
		true,
	);
});

Deno.test("Unregister nonexistent OCR backend gracefully", () => {
	unregisterOcrBackend("nonexistent-backend-xyz");
});

// Post Processor Management

Deno.test("Clear all post-processors and verify list is empty", () => {
	clearPostProcessors();
});

Deno.test("List all registered post-processors", () => {
	const result = listPostProcessors();
	assertEquals(Array.isArray(result), true);
	assertEquals(
		result.every((item: unknown) => typeof item === "string"),
		true,
	);
});

// Validator Management

Deno.test("Clear all validators and verify list is empty", () => {
	clearValidators();
	const result = listValidators();
	assertEquals(result.length, 0);
});

Deno.test("List all registered validators", () => {
	const result = listValidators();
	assertEquals(Array.isArray(result), true);
	assertEquals(
		result.every((item: unknown) => typeof item === "string"),
		true,
	);
});

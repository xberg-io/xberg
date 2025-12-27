// Auto-generated tests for image fixtures.

import { assertions, buildConfig, extractBytes, initWasm, resolveDocument, shouldSkipFixture } from "./helpers.ts";
import type { ExtractionResult } from "./helpers.ts";

await initWasm();

Deno.test("image_metadata_only", { permissions: { read: true } }, async () => {
	const documentBytes = await resolveDocument("images/example.jpg");
	const config = buildConfig({ ocr: null });
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "image/jpeg", config);
	} catch (error) {
		if (shouldSkipFixture(error, "image_metadata_only", [], undefined)) {
			return;
		}
		throw error;
	}
	if (result === null) {
		return;
	}
	assertions.assertExpectedMime(result, ["image/jpeg"]);
	assertions.assertMaxContentLength(result, 100);
});

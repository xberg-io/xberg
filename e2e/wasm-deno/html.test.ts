// Auto-generated tests for html fixtures.

import { assertions, buildConfig, extractBytes, initWasm, resolveDocument, shouldSkipFixture } from "./helpers.ts";
import type { ExtractionResult } from "./helpers.ts";

await initWasm();

Deno.test("html_simple_table", { permissions: { read: true } }, async () => {
	const documentBytes = await resolveDocument("web/simple_table.html");
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

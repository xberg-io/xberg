// Auto-generated tests for xml fixtures.

import { assertions, buildConfig, extractBytes, initWasm, resolveDocument, shouldSkipFixture } from "./helpers.ts";
import type { ExtractionResult } from "./helpers.ts";

await initWasm();

Deno.test("xml_plant_catalog", { permissions: { read: true } }, async () => {
	const documentBytes = await resolveDocument("xml/plant_catalog.xml");
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

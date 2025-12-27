// Auto-generated tests for email fixtures.

import { assertions, buildConfig, extractBytes, initWasm, resolveDocument, shouldSkipFixture } from "./helpers.ts";
import type { ExtractionResult } from "./helpers.ts";

await initWasm();

Deno.test("email_sample_eml", { permissions: { read: true } }, async () => {
	const documentBytes = await resolveDocument("email/sample_email.eml");
	const config = buildConfig(undefined);
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "message/rfc822", config);
	} catch (error) {
		if (shouldSkipFixture(error, "email_sample_eml", [], undefined)) {
			return;
		}
		throw error;
	}
	if (result === null) {
		return;
	}
	assertions.assertExpectedMime(result, ["message/rfc822"]);
	assertions.assertMinContentLength(result, 20);
});

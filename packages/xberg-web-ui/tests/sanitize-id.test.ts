import { describe, it, expect } from "vitest";
import { sanitizeExternalId } from "../src/lib/sanitize-id.js";
import { DOCUMENT_ID_PATTERN } from "../src/lib/constants.js";

describe("lib/sanitize-id", () => {
  it("leaves an already-safe filename untouched", () => {
    expect(sanitizeExternalId("contrat-2026.pdf")).toBe("contrat-2026.pdf");
  });

  it("replaces unsafe characters with underscores", () => {
    const result = sanitizeExternalId("contrat client (v2)/résumé.pdf");
    expect(DOCUMENT_ID_PATTERN.test(result)).toBe(true);
  });

  it("collapses to a fallback when the whole name is unsafe", () => {
    const result = sanitizeExternalId("////");
    expect(result.length).toBeGreaterThan(0);
    expect(DOCUMENT_ID_PATTERN.test(result)).toBe(true);
  });

  it("is deterministic for the same input", () => {
    expect(sanitizeExternalId("a b.pdf")).toBe(sanitizeExternalId("a b.pdf"));
  });
});

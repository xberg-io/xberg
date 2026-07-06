import { describe, it, expect } from "vitest";
import {
  scanEuStructured,
  scanArt9Keywords,
  scanEuPatterns,
  overlapsExisting,
} from "../src/redaction/eu-patterns.js";

describe("scanEuStructured", () => {
  it("detects a French INSEE number", () => {
    // base 1850575116023 % 97 = 60, key = 97 - 60 = 37
    const result = scanEuStructured("INSEE: 185057511602337");
    expect(result.some((m) => m.category === "NATIONAL_ID_FR")).toBe(true);
  });

  it("rejects an INSEE-shaped number with a bad checksum key", () => {
    const result = scanEuStructured("185057511602324");
    expect(result.some((m) => m.category === "NATIONAL_ID_FR")).toBe(false);
  });

  it("detects a Spanish DNI", () => {
    const result = scanEuStructured("DNI: 12345678Z");
    expect(result.some((m) => m.category === "NATIONAL_ID_ES")).toBe(true);
  });

  it("rejects a DNI-shaped number with a bad checksum letter", () => {
    const result = scanEuStructured("12345678A");
    expect(result.some((m) => m.category === "NATIONAL_ID_ES")).toBe(false);
  });

  it("detects an Italian Codice Fiscale", () => {
    const result = scanEuStructured("Codice Fiscale: RSSMRA85T10A562S");
    expect(result.some((m) => m.category === "NATIONAL_ID_IT")).toBe(true);
  });

  it("rejects a Codice-Fiscale-shaped string with a bad control letter", () => {
    const result = scanEuStructured("RSSMRA85T10A562A");
    expect(result.some((m) => m.category === "NATIONAL_ID_IT")).toBe(false);
  });

  it("detects a valid Polish PESEL", () => {
    const result = scanEuStructured("PESEL: 80051501231");
    expect(result.some((m) => m.category === "NATIONAL_ID_PL")).toBe(true);
  });

  it("rejects a PESEL-shaped number with a bad checksum", () => {
    const result = scanEuStructured("80051501230");
    expect(result.some((m) => m.category === "NATIONAL_ID_PL")).toBe(false);
  });

  it("detects a valid Dutch BSN", () => {
    const result = scanEuStructured("BSN: 123456782");
    expect(result.some((m) => m.category === "NATIONAL_ID_NL")).toBe(true);
  });

  it("detects a French SIRET (14 digits)", () => {
    const result = scanEuStructured("SIRET: 73282932000074");
    expect(result.some((m) => m.category === "TAX_ID_SIRET")).toBe(true);
    // The 9-digit SIREN prefix should not ALSO be flagged separately -- it overlaps the SIRET span.
    expect(result.some((m) => m.category === "TAX_ID_SIREN")).toBe(false);
  });

  it("detects an EU VAT number", () => {
    const result = scanEuStructured("VAT: FR12345678901");
    expect(result.some((m) => m.category === "TAX_ID_VAT")).toBe(true);
  });

  it("returns character offsets that round-trip to the original text", () => {
    const text = "Café PESEL: 80051501231 end";
    const result = scanEuStructured(text);
    const pesel = result.find((m) => m.category === "NATIONAL_ID_PL");
    expect(pesel).toBeDefined();
    expect(text.slice(pesel!.start, pesel!.end)).toBe("80051501231");
  });

  it("detects a plate-like alphanumeric block behind a mandatory separator", () => {
    const result = scanEuStructured("Vehicle FR-AB123CD was parked outside");
    expect(result.some((m) => m.category === "LICENSE_PLATE_EU")).toBe(true);
  });

  it("does not flag a bare year or reference number after a country code", () => {
    // No separator at all -- the old regex's optional `\s?-?` let this through.
    expect(scanEuStructured("Report AT2024 filed").some((m) => m.category === "LICENSE_PLATE_EU")).toBe(false);
    // Separator present, but the block is all digits (no letters) -- rejected by the mixed-alnum validator.
    expect(scanEuStructured("Invoice ref SE-300 due").some((m) => m.category === "LICENSE_PLATE_EU")).toBe(false);
  });
});

describe("scanArt9Keywords", () => {
  it("detects a health condition keyword", () => {
    const result = scanArt9Keywords("Patient diagnosed with diabetes");
    expect(result.some((m) => m.category === "SPECIAL_CATEGORY_HEALTH")).toBe(true);
  });

  it("detects a religion keyword", () => {
    const result = scanArt9Keywords("He is Catholic");
    expect(result.some((m) => m.category === "SPECIAL_CATEGORY_RELIGION")).toBe(true);
  });

  it("detects a criminal record keyword", () => {
    const result = scanArt9Keywords("He was convicted of fraud");
    expect(result.some((m) => m.category === "SPECIAL_CATEGORY_CRIMINAL")).toBe(true);
  });

  it("detects a biometric keyword", () => {
    const result = scanArt9Keywords("Access requires facial recognition");
    expect(result.some((m) => m.category === "SPECIAL_CATEGORY_BIOMETRIC")).toBe(true);
  });

  it("returns nothing for neutral text", () => {
    const result = scanArt9Keywords("The meeting is scheduled for Tuesday.");
    expect(result).toHaveLength(0);
  });
});

describe("scanEuPatterns", () => {
  it("combines structured and Art. 9 findings, sorted by position", () => {
    const result = scanEuPatterns("Patient diagnosed with diabetes. PESEL: 80051501231.");
    expect(result.some((f) => f.category === "SPECIAL_CATEGORY_HEALTH")).toBe(true);
    expect(result.some((f) => f.category === "NATIONAL_ID_PL")).toBe(true);
    for (let i = 1; i < result.length; i++) {
      expect(result[i]!.start).toBeGreaterThanOrEqual(result[i - 1]!.start);
    }
  });

  it("assigns sequential per-category tokens", () => {
    const result = scanEuPatterns("He is Catholic. She is Muslim.");
    const religionFindings = result.filter((f) => f.category === "SPECIAL_CATEGORY_RELIGION");
    expect(religionFindings.map((f) => f.token)).toEqual([
      "[SPECIAL_CATEGORY_RELIGION_1]",
      "[SPECIAL_CATEGORY_RELIGION_2]",
    ]);
  });

  it("the overlap filter excludes an Art.9-shaped span that overlaps a structured match", () => {
    // Both scanEuStructured and scanArt9Keywords are \b-bounded over
    // contiguous \w runs, so a structured ID and a keyword can never
    // genuinely overlap through the public scan functions themselves --
    // exercise the exact filter scanEuPatterns applies (overlapsExisting
    // against the structured matches) directly instead.
    const structured = scanEuStructured("SIRET: 73282932000074");
    const siret = structured.find((m) => m.category === "TAX_ID_SIRET");
    expect(siret).toBeDefined();

    expect(overlapsExisting(structured, siret!.start, siret!.end)).toBe(true);
    expect(overlapsExisting(structured, siret!.end + 5, siret!.end + 10)).toBe(false);
  });

  it("retains both a structured match and a genuinely non-overlapping Art.9 match", () => {
    const result = scanEuPatterns("Patient diagnosed with diabetes. SIRET: 73282932000074.");
    expect(result.some((f) => f.category === "SPECIAL_CATEGORY_HEALTH")).toBe(true);
    expect(result.some((f) => f.category === "TAX_ID_SIRET")).toBe(true);
  });

  it("returns an empty array for text with no EU PII", () => {
    expect(scanEuPatterns("The weather is nice today.")).toHaveLength(0);
  });
});

describe("overlapsExisting", () => {
  it("returns true when spans overlap", () => {
    const existing = [{ category: "A", original: "x", start: 5, end: 10, confidence: 0.9 }];
    expect(overlapsExisting(existing, 7, 12)).toBe(true);
  });

  it("returns false when spans are adjacent but not overlapping", () => {
    const existing = [{ category: "A", original: "x", start: 5, end: 10, confidence: 0.9 }];
    expect(overlapsExisting(existing, 10, 15)).toBe(false);
  });

  it("returns true when a span is fully contained within another", () => {
    const existing = [{ category: "A", original: "x", start: 0, end: 20, confidence: 0.9 }];
    expect(overlapsExisting(existing, 5, 10)).toBe(true);
  });

  it("returns false for an empty existing list", () => {
    expect(overlapsExisting([], 0, 5)).toBe(false);
  });
});

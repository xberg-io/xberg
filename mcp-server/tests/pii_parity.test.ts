import { describe, it, expect, beforeAll } from "vitest";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import type { XbergEngine } from "@xberg-io/xberg-wasm";
import type { PiiFinding } from "../src/redaction/detect.js";

// Parity test: proves the wasm engine's PII detector (engine.detect_pii)
// did not regress vs the pre-migration TS regex detector
// (src/redaction/detect.ts detectPii) on the "anchor" categories that both
// detectors implement with equivalent patterns: EMAIL, SSN, CREDIT_CARD,
// IP_ADDRESS. PHONE is checked as a soft (non-fatal) signal since the two
// regexes may legitimately differ. DATE_ISO/DATE_MDY/IBAN/SWIFT_BIC/
// POSTAL_CODE_* are excluded entirely: the engine has no equivalent
// category for them.

const __dirname = path.dirname(fileURLToPath(import.meta.url));

// Inverse of CATEGORY_TO_ENGINE in src/tools/pii.ts: engine snake_case ->
// TS UPPER category name.
const ENGINE_TO_TS_CATEGORY: Record<string, string> = {
  email: "EMAIL",
  phone: "PHONE",
  ssn: "SSN",
  credit_card: "CREDIT_CARD",
  ip_address: "IP_ADDRESS",
  date_of_birth: "DATE",
  person: "PERSON",
  organization: "ORGANIZATION",
  location: "LOCATION",
};

const ANCHOR_CATEGORIES = ["EMAIL", "SSN", "CREDIT_CARD", "IP_ADDRESS"] as const;

interface EnginePiiMatch {
  start: number;
  end: number;
  category: string;
  text: string;
}

function countByCategory(items: Array<{ category: string }>): Record<string, number> {
  const counts: Record<string, number> = {};
  for (const it of items) {
    counts[it.category] = (counts[it.category] ?? 0) + 1;
  }
  return counts;
}

describe("PII detection parity: wasm engine vs pre-migration TS regex detector", () => {
  let engine: XbergEngine;
  let fixtureText: string;
  let expectedFindings: PiiFinding[];

  beforeAll(async () => {
    const { initializeEngine } = await import("../src/engine.js");
    engine = await initializeEngine();

    fixtureText = fs.readFileSync(
      path.join(__dirname, "fixtures", "pii_input.txt"),
      "utf8"
    );
    const expectedJson = JSON.parse(
      fs.readFileSync(path.join(__dirname, "fixtures", "pii_expected.json"), "utf8")
    ) as { findings: PiiFinding[] };
    expectedFindings = expectedJson.findings;
  }, 180_000);

  it("engine.detect_pii matches TS detectPii on anchor categories (count + span)", async () => {
    const rawEngineFindings = await engine.detect_pii(fixtureText, null);
    const engineFindings = rawEngineFindings as EnginePiiMatch[];

    // Normalize engine's snake_case categories to TS UPPER names.
    const normalizedEngineFindings = engineFindings.map((f) => ({
      ...f,
      category: ENGINE_TO_TS_CATEGORY[f.category] ?? f.category.toUpperCase(),
    }));

    const engineCounts = countByCategory(normalizedEngineFindings);
    const expectedCounts = countByCategory(expectedFindings);

    // Full count table across every category present in the fixture
    // (anchors + PHONE + any excluded categories), for visibility.
    const allCategories = new Set([...Object.keys(engineCounts), ...Object.keys(expectedCounts)]);
    const table: Record<string, { engine: number; ts: number }> = {};
    for (const cat of allCategories) {
      table[cat] = { engine: engineCounts[cat] ?? 0, ts: expectedCounts[cat] ?? 0 };
    }
    // eslint-disable-next-line no-console
    console.log("PII parity count table (engine vs TS):", JSON.stringify(table, null, 2));

    // Hard assertions on anchor categories.
    for (const cat of ANCHOR_CATEGORIES) {
      expect(
        engineCounts[cat] ?? 0,
        `category ${cat}: engine count ${engineCounts[cat] ?? 0} !== TS count ${expectedCounts[cat] ?? 0}`
      ).toBe(expectedCounts[cat] ?? 0);
    }

    // Soft check on PHONE: log both counts, do not hard-fail on mismatch.
    const enginePhoneCount = engineCounts["PHONE"] ?? 0;
    const tsPhoneCount = expectedCounts["PHONE"] ?? 0;
    // eslint-disable-next-line no-console
    console.log(
      `PII parity (soft check) PHONE: engine=${enginePhoneCount} ts=${tsPhoneCount}` +
        (enginePhoneCount === tsPhoneCount ? " (match)" : " (mismatch, not asserted)")
    );

    // Span-level assertion for the EMAIL anchor: proves position parity,
    // not just counts.
    const tsEmail = expectedFindings.find((f) => f.category === "EMAIL");
    const engineEmail = normalizedEngineFindings.find((f) => f.category === "EMAIL");
    expect(tsEmail, "TS fixture baseline must contain an EMAIL finding").toBeDefined();
    expect(engineEmail, "engine must find an EMAIL entity in the fixture").toBeDefined();
    expect(engineEmail!.start).toBe(tsEmail!.start);
    expect(engineEmail!.end).toBe(tsEmail!.end);
  }, 60_000);
});

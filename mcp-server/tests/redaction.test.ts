import { describe, it, expect } from "vitest";
import { detectPii, groupByCategory, detectPiiEu, dedupOverlapping, buildPiiReport } from "../src/redaction/detect.js";
import { applyRedaction } from "../src/redaction/redact.js";

describe("detectPii", () => {
  it("detects email addresses", () => {
    const findings = detectPii("Contact us at info@example.com for details.");
    expect(findings).toHaveLength(1);
    expect(findings[0]?.category).toBe("EMAIL");
    expect(findings[0]?.original).toBe("info@example.com");
    expect(findings[0]?.token).toBe("[EMAIL_1]");
    expect(findings[0]?.confidence).toBeGreaterThan(0.9);
  });

  it("detects phone numbers", () => {
    const findings = detectPii("Call us at 555-867-5309.");
    expect(findings.some((f) => f.category === "PHONE")).toBe(true);
  });

  it("detects SSN", () => {
    const findings = detectPii("SSN: 123-45-6789");
    expect(findings.some((f) => f.category === "SSN")).toBe(true);
  });

  it("detects credit card numbers", () => {
    const findings = detectPii("Card: 4111 1111 1111 1111");
    expect(findings.some((f) => f.category === "CREDIT_CARD")).toBe(true);
  });

  it("detects IP addresses", () => {
    const findings = detectPii("Server at 192.168.1.100");
    expect(findings.some((f) => f.category === "IP_ADDRESS")).toBe(true);
  });

  it("detects ISO dates", () => {
    const findings = detectPii("Born on 1990-05-15");
    expect(findings.some((f) => f.category === "DATE_ISO")).toBe(true);
  });

  it("filters by category", () => {
    const text = "Email: test@test.com, SSN: 123-45-6789";
    const findings = detectPii(text, ["EMAIL"]);
    expect(findings.every((f) => f.category === "EMAIL")).toBe(true);
  });

  it("returns findings sorted by start position", () => {
    const findings = detectPii("Email: a@b.com and phone 555-123-4567");
    for (let i = 1; i < findings.length; i++) {
      expect(findings[i]!.start).toBeGreaterThanOrEqual(findings[i - 1]!.start);
    }
  });

  it("assigns stable sequential tokens per category", () => {
    const findings = detectPii("a@b.com and c@d.com");
    const emails = findings.filter((f) => f.category === "EMAIL");
    expect(emails[0]?.token).toBe("[EMAIL_1]");
    expect(emails[1]?.token).toBe("[EMAIL_2]");
  });

  it("returns empty array for clean text", () => {
    expect(detectPii("Hello, how are you today?")).toHaveLength(0);
  });
});

describe("groupByCategory", () => {
  it("counts findings per category", () => {
    const findings = detectPii("a@b.com c@d.com 555-123-4567");
    const groups = groupByCategory(findings);
    expect(groups["EMAIL"]).toBe(2);
    expect(groups["PHONE"]).toBe(1);
  });
});

describe("applyRedaction", () => {
  it("token_replace substitutes tokens and builds map", () => {
    const input = "Email: user@corp.com";
    const findings = detectPii(input);
    const { redacted, token_map } = applyRedaction(input, findings, "token_replace");
    expect(redacted).toBe("Email: [EMAIL_1]");
    expect(token_map["[EMAIL_1]"]).toBe("user@corp.com");
  });

  it("mask replaces with asterisks", () => {
    const input = "user@corp.com";
    const findings = detectPii(input);
    const { redacted, token_map } = applyRedaction(input, findings, "mask");
    expect(redacted).toBe("*".repeat(input.length));
    expect(Object.keys(token_map)).toHaveLength(0);
  });

  it("hash produces deterministic HASH_ prefix", () => {
    const t = "user@corp.com";
    const findings = detectPii(t);
    const { redacted: r1 } = applyRedaction(t, findings, "hash");
    const { redacted: r2 } = applyRedaction(t, detectPii(t), "hash");
    expect(r1).toMatch(/^HASH_[0-9a-f]+$/);
    expect(r1).toBe(r2);
  });

  it("handles multiple findings without overlap", () => {
    const t = "a@b.com and 192.168.1.1";
    const findings = detectPii(t);
    const { redacted } = applyRedaction(t, findings, "token_replace");
    expect(redacted).not.toContain("a@b.com");
    expect(redacted).not.toContain("192.168.1.1");
  });
});

describe("dedupOverlapping", () => {
  it("keeps the longest span when two findings overlap", () => {
    const findings = [
      { token: "[A_1]", category: "A", original: "John", start: 0, end: 4, confidence: 0.5 },
      { token: "[B_1]", category: "B", original: "John Smith", start: 0, end: 10, confidence: 0.6 },
    ];
    const result = dedupOverlapping(findings);
    expect(result).toHaveLength(1);
    expect(result[0]?.category).toBe("B");
  });

  it("keeps non-overlapping findings", () => {
    const findings = [
      { token: "[A_1]", category: "A", original: "foo", start: 0, end: 3, confidence: 0.5 },
      { token: "[B_1]", category: "B", original: "bar", start: 5, end: 8, confidence: 0.5 },
    ];
    expect(dedupOverlapping(findings)).toHaveLength(2);
  });
});

describe("detectPiiEu", () => {
  it("includes both generic and EU findings", () => {
    const result = detectPiiEu("Email: bob@example.com. He was diagnosed with cancer.");
    expect(result.some((f) => f.category === "EMAIL")).toBe(true);
    expect(result.some((f) => f.category === "SPECIAL_CATEGORY_HEALTH")).toBe(true);
  });

  it("does not change detectPii's own output", () => {
    // detectPii() itself must remain unaffected by this addition.
    const before = detectPii("bob@example.com").filter((f) => f.category === "EMAIL");
    expect(before).toHaveLength(1);
  });

  it("respects filterCategories across both scans", () => {
    const result = detectPiiEu("Email: bob@example.com. He was diagnosed with cancer.", ["EMAIL"]);
    expect(result.some((f) => f.category === "EMAIL")).toBe(true);
    expect(result.some((f) => f.category === "SPECIAL_CATEGORY_HEALTH")).toBe(false);
  });
});

describe("buildPiiReport", () => {
  it("counts findings by pii-report category", () => {
    const findings = [
      { token: "[NAME_1]", category: "NAME", original: "John", start: 0, end: 4, confidence: 0.6 },
      {
        token: "[ID_NUMBER_1]",
        category: "SSN",
        original: "123-45-6789",
        start: 10,
        end: 21,
        confidence: 0.9,
      },
    ];
    const report = buildPiiReport(findings);
    expect(report.personCount).toBe(1);
    expect(report.idNumberCount).toBe(1);
    expect(report.specialCategoryCount).toBe(0);
    expect(report.kAnonymityRisk).toBe("CRITICAL (direct identifiers or special-category data present)");
  });

  it("reports LOW risk when nothing sensitive is present", () => {
    const report = buildPiiReport([]);
    expect(report.specialCategoryCount).toBe(0);
    expect(report.kAnonymityRisk).toBe("LOW");
  });

  it("counts GDPR Art. 9 special-category findings and escalates risk to CRITICAL", () => {
    const findings = [
      {
        token: "[SPECIAL_CATEGORY_HEALTH_1]",
        category: "SPECIAL_CATEGORY_HEALTH",
        original: "diagnosed with",
        start: 0,
        end: 14,
        confidence: 0.97,
      },
      { token: "[NAME_1]", category: "NAME", original: "Alice", start: 20, end: 25, confidence: 0.6 },
    ];
    const report = buildPiiReport(findings);
    expect(report.specialCategoryCount).toBe(1);
    expect(report.idNumberCount).toBe(0);
    expect(report.kAnonymityRisk).toBe("CRITICAL (direct identifiers or special-category data present)");
  });

  it("reports HIGH risk for a quasi-identifier combination", () => {
    const makeName = (i: number) => ({
      token: `[NAME_${i}]`,
      category: "NAME",
      original: `Person${i}`,
      start: i * 10,
      end: i * 10 + 8,
      confidence: 0.6,
    });
    const findings = [
      ...[1, 2, 3, 4, 5, 6].map(makeName),
      { token: "[DATE_1]", category: "DATE", original: "1990-01-01", start: 100, end: 110, confidence: 0.7 },
      {
        token: "[LOCATION_1]",
        category: "LOCATION",
        original: "Springfield",
        start: 120,
        end: 131,
        confidence: 0.7,
      },
    ];
    expect(buildPiiReport(findings).kAnonymityRisk).toBe("HIGH (quasi-identifier combination)");
  });
});

import { describe, it, expect } from "vitest";
import { detectPii, groupByCategory } from "../src/redaction/detect.js";
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


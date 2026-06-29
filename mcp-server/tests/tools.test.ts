import { describe, it, expect, vi } from "vitest";

// Lightweight smoke tests — verify tool registration runs without native bindings.
// Full integration tests require the NAPI bindings to be built.

describe("tool registration", () => {
  it("PII detect and redact modules export expected functions", async () => {
    const detect = await import("../src/redaction/detect.js");
    expect(typeof detect.detectPii).toBe("function");
    expect(typeof detect.groupByCategory).toBe("function");

    const redact = await import("../src/redaction/redact.js");
    expect(typeof redact.applyRedaction).toBe("function");
    expect(typeof redact.redactToString).toBe("function");
  });

  it("rehydration module exports encrypt/decrypt", async () => {
    const r = await import("../src/redaction/rehydration.js");
    expect(typeof r.encryptMapFile).toBe("function");
    expect(typeof r.decryptMapFile).toBe("function");
  });

  it("output modules export write functions", async () => {
    const docx = await import("../src/redaction/output/docx.js");
    expect(typeof docx.writeRedactedDocx).toBe("function");

    const pdf = await import("../src/redaction/output/pdf.js");
    expect(typeof pdf.writeRedactedPdf).toBe("function");

    const text = await import("../src/redaction/output/text.js");
    expect(typeof text.writeRedactedText).toBe("function");

    const report = await import("../src/redaction/output/report.js");
    expect(typeof report.writeReport).toBe("function");
  });

  it("transport modules export start functions", async () => {
    const stdio = await import("../src/transports/stdio.js");
    expect(typeof stdio.startStdio).toBe("function");

    const http = await import("../src/transports/http.js");
    expect(typeof http.startHttp).toBe("function");
  });
});

describe("rehydration encryption", () => {
  it("round-trips a token map through encrypt/decrypt", async () => {
    const { encryptMapFile, decryptMapFile } = await import("../src/redaction/rehydration.js");
    const { tmpdir } = await import("node:os");
    const { join } = await import("node:path");
    const { unlinkSync } = await import("node:fs");

    const mapPath = join(tmpdir(), `xberg-test-${Date.now()}.map`);
    const original = { "[EMAIL_1]": "alice@example.com", "[PHONE_1]": "+1-555-000-0001" };
    const passphrase = "test-passphrase-32bytes-long!!!!";

    try {
      encryptMapFile(mapPath, original, passphrase);
      const recovered = decryptMapFile(mapPath, passphrase);
      expect(recovered).toEqual(original);
    } finally {
      try { unlinkSync(mapPath); } catch { /* ignore */ }
    }
  });

  it("decryptMapFile falls back to plaintext JSON if no magic header", async () => {
    const { decryptMapFile } = await import("../src/redaction/rehydration.js");
    const { tmpdir } = await import("node:os");
    const { join } = await import("node:path");
    const { writeFileSync, unlinkSync } = await import("node:fs");

    const mapPath = join(tmpdir(), `xberg-plain-${Date.now()}.map`);
    const data = { "[TOKEN_1]": "value" };

    try {
      writeFileSync(mapPath, JSON.stringify(data), "utf-8");
      const recovered = decryptMapFile(mapPath, "any-passphrase");
      expect(recovered).toEqual(data);
    } finally {
      try { unlinkSync(mapPath); } catch { /* ignore */ }
    }
  });
});

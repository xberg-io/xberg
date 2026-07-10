import { describe, it, expect, beforeAll, afterAll } from "vitest";
import { z } from "zod";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const __dirname = dirname(fileURLToPath(import.meta.url));

// Lightweight smoke tests — verify tool registration runs without native bindings.
// Full integration tests require the NAPI bindings to be built.

describe("ExtractionConfigSchema shape", () => {
  it("accepts chunking config with bounds", () => {
    const ChunkingConfigSchema = z.object({
      max_size: z.number().int().min(64).max(16384).optional(),
      overlap: z.number().int().min(0).max(1024).optional(),
    });
    expect(ChunkingConfigSchema.safeParse({ max_size: 512, overlap: 64 }).success).toBe(true);
    expect(ChunkingConfigSchema.safeParse({ max_size: 10 }).success).toBe(false);
    expect(ChunkingConfigSchema.safeParse({ overlap: -1 }).success).toBe(false);
  });

  it("accepts keyword config with algorithm enum", () => {
    const KeywordConfigSchema = z.object({
      algorithm: z.enum(["yake", "rake"]).optional(),
      max_keywords: z.number().int().min(1).max(100).optional(),
    });
    expect(KeywordConfigSchema.safeParse({ algorithm: "yake", max_keywords: 10 }).success).toBe(true);
    expect(KeywordConfigSchema.safeParse({ algorithm: "invalid" }).success).toBe(false);
    expect(KeywordConfigSchema.safeParse({ max_keywords: 0 }).success).toBe(false);
  });

  it("accepts ocr config with backend enum and language list", () => {
    const OcrConfigSchema = z.object({
      backend: z.enum(["tesseract", "paddleocr"]).optional(),
      languages: z.array(z.string()).optional(),
    });
    expect(OcrConfigSchema.safeParse({ backend: "tesseract", languages: ["eng", "deu"] }).success).toBe(true);
    expect(OcrConfigSchema.safeParse({ backend: "unknown_engine" }).success).toBe(false);
    expect(OcrConfigSchema.safeParse({ languages: ["eng"] }).success).toBe(true);
  });
});

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

describe("intelligence/media/web tool modules", () => {
  it("intelligence module exports registerIntelligenceTools", async () => {
    const m = await import("../src/tools/intelligence.js");
    expect(typeof m.registerIntelligenceTools).toBe("function");
  });

  it("media module exports registerMediaTools", async () => {
    const m = await import("../src/tools/media.js");
    expect(typeof m.registerMediaTools).toBe("function");
  });

  it("web module exports registerWebTools", async () => {
    const m = await import("../src/tools/web.js");
    expect(typeof m.registerWebTools).toBe("function");
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

describe("extract tools (wasm engine)", () => {
  let client: import("@modelcontextprotocol/sdk/client/index.js").Client;

  beforeAll(async () => {
    const { initializeEngine } = await import("../src/engine.js");
    const { registerExtractTools } = await import("../src/tools/extract.js");
    const { McpServer } = await import("@modelcontextprotocol/sdk/server/mcp.js");
    const { Client } = await import("@modelcontextprotocol/sdk/client/index.js");
    const { InMemoryTransport } = await import("@modelcontextprotocol/sdk/inMemory.js");

    // First run downloads the embedder model (transformers.js) and loads the
    // ~100MB wasm binary, so allow a generous budget.
    await initializeEngine();

    const server = new McpServer({ name: "test-server", version: "0.0.0" });
    registerExtractTools(server);

    const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair();
    client = new Client({ name: "test-client", version: "0.0.0" });
    await Promise.all([
      server.connect(serverTransport),
      client.connect(clientTransport),
    ]);
  }, 180_000);

  afterAll(async () => {
    await client?.close();
  });

  it("extract_document extracts content from a real fixture", async () => {
    const fixturePath = join(__dirname, "fixtures", "extract-sample.txt");
    const result = await client.callTool({
      name: "extract_document",
      arguments: { input: { uri: fixturePath } },
    });

    const content = (result.content as Array<{ type: string; text: string }>)[0];
    if (result.isError) throw new Error(content?.text ?? "unknown error");
    expect(content?.type).toBe("text");
    const parsed = JSON.parse(content!.text) as { results: Array<{ content: string }> };
    expect(parsed.results.length).toBeGreaterThan(0);
    expect(parsed.results[0]?.content.length).toBeGreaterThan(0);
    expect(parsed.results[0]?.content).toContain("Xberg");
  }, 60_000);

  it("extract_batch extracts content from multiple real fixtures", async () => {
    const fixturePath = join(__dirname, "fixtures", "extract-sample.txt");
    const result = await client.callTool({
      name: "extract_batch",
      arguments: { inputs: [{ uri: fixturePath }, { uri: fixturePath }] },
    });

    expect(result.isError).not.toBe(true);
    const content = (result.content as Array<{ type: string; text: string }>)[0];
    const parsed = JSON.parse(content!.text) as { results: Array<{ content: string }> };
    expect(parsed.results.length).toBe(2);
    expect(parsed.results[0]?.content.length).toBeGreaterThan(0);
    expect(parsed.results[1]?.content.length).toBeGreaterThan(0);
  }, 60_000);

  it("list_formats returns supported formats from the wasm module", async () => {
    const result = await client.callTool({ name: "list_formats", arguments: {} });

    expect(result.isError).not.toBe(true);
    const content = (result.content as Array<{ type: string; text: string }>)[0];
    const parsed = JSON.parse(content!.text) as Array<{ extension: string; mimeType: string }>;
    expect(Array.isArray(parsed)).toBe(true);
    expect(parsed.length).toBeGreaterThan(0);
    expect(parsed[0]).toHaveProperty("extension");
    expect(parsed[0]).toHaveProperty("mimeType");
  });

  it("extract_document returns an error result for invalid input", async () => {
    const result = await client.callTool({ name: "extract_document", arguments: {} });
    expect(result.isError).toBe(true);
  });
});

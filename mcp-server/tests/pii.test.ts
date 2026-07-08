import { describe, it, expect, beforeAll, afterAll } from "vitest";

// Full integration tests against the real wasm engine's regex-based PII
// pattern matcher (xberg::text::redaction::patterns::scan_text), exercised
// through the registered MCP tools (detect_pii, redact_document).

describe("pii tools (wasm engine)", () => {
  let client: import("@modelcontextprotocol/sdk/client/index.js").Client;

  beforeAll(async () => {
    const { initializeEngine } = await import("../src/engine.js");
    const { registerPiiTools } = await import("../src/tools/pii.js");
    const { McpServer } = await import("@modelcontextprotocol/sdk/server/mcp.js");
    const { Client } = await import("@modelcontextprotocol/sdk/client/index.js");
    const { InMemoryTransport } = await import("@modelcontextprotocol/sdk/inMemory.js");

    // First run downloads the embedder model (transformers.js) and loads the
    // ~100MB wasm binary, so allow a generous budget.
    await initializeEngine();

    const server = new McpServer({ name: "test-server", version: "0.0.0" });
    registerPiiTools(server);

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

  it("detect_pii finds an email and SSN with the public output shape", async () => {
    const result = await client.callTool({
      name: "detect_pii",
      arguments: { text: "Email: test@example.com, SSN: 123-45-6789" },
    });

    expect(result.isError).not.toBe(true);
    const content = (result.content as Array<{ type: string; text: string }>)[0];
    const parsed = JSON.parse(content!.text) as {
      findings: Array<{ entity_type: string; text: string; start: number; end: number; score: number }>;
      total: number;
    };

    expect(parsed.findings.length).toBeGreaterThan(0);
    expect(parsed.total).toBe(parsed.findings.length);

    for (const f of parsed.findings) {
      expect(typeof f.entity_type).toBe("string");
      expect(typeof f.text).toBe("string");
      expect(typeof f.start).toBe("number");
      expect(typeof f.end).toBe("number");
    }

    const email = parsed.findings.find((f) => f.text === "test@example.com");
    expect(email).toBeDefined();
    const ssn = parsed.findings.find((f) => f.text === "123-45-6789");
    expect(ssn).toBeDefined();
  }, 60_000);

  it("detect_pii filters by categories (lowercase wasm engine category names)", async () => {
    const result = await client.callTool({
      name: "detect_pii",
      arguments: {
        text: "Email: test@example.com, SSN: 123-45-6789",
        categories: ["email"],
      },
    });

    expect(result.isError).not.toBe(true);
    const content = (result.content as Array<{ type: string; text: string }>)[0];
    const parsed = JSON.parse(content!.text) as {
      findings: Array<{ entity_type: string; text: string }>;
    };

    expect(parsed.findings.length).toBeGreaterThan(0);
    expect(parsed.findings.every((f) => f.text === "test@example.com")).toBe(true);
  }, 60_000);

  it("redact_document redacts with token_replace strategy and returns token_map", async () => {
    const text = "Contact me at test@example.com or 123-45-6789.";
    const result = await client.callTool({
      name: "redact_document",
      arguments: { text, strategy: "token_replace" },
    });

    expect(result.isError).not.toBe(true);
    const content = (result.content as Array<{ type: string; text: string }>)[0];
    const parsed = JSON.parse(content!.text) as {
      redacted_text: string;
      token_map: Record<string, string>;
      entities_redacted: number;
    };

    // Log actual output so the real token format is visible in CI/test output
    // rather than assuming a hardcoded format like [EMAIL_1].
    // eslint-disable-next-line no-console
    console.log("redact_document output:", JSON.stringify(parsed, null, 2));

    expect(parsed.redacted_text).not.toContain("test@example.com");
    expect(parsed.redacted_text).not.toContain("123-45-6789");
    expect(parsed.entities_redacted).toBeGreaterThan(0);
    expect(Object.keys(parsed.token_map).length).toBe(parsed.entities_redacted);
    expect(Object.values(parsed.token_map)).toContain("test@example.com");
    expect(Object.values(parsed.token_map)).toContain("123-45-6789");
  }, 60_000);

  it("redact_document returns an error result for invalid input", async () => {
    const result = await client.callTool({ name: "redact_document", arguments: {} });
    expect(result.isError).toBe(true);
  });
});

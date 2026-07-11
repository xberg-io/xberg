import { describe, it, expect, beforeAll, afterAll } from "vitest";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { mkdtempSync, unlinkSync, rmSync, readFileSync } from "node:fs";

// Cross-format compatibility: the TS `encryptMapFile` (Node crypto, AES-256-GCM,
// `XPII\x01` wire format) must produce blobs that the wasm engine's
// `decrypt_map` can decrypt, and vice versa in spirit. This proves the
// retargeted `rehydrate_document` tool (which now calls `engine.decrypt_map`
// instead of the TS `decryptMapFile`) stays interoperable with maps written
// by `ingest_folder` (which still uses `encryptMapFile`).
describe("rehydration TS-encrypt <-> wasm-decrypt parity", () => {
  beforeAll(async () => {
    const { initializeEngine } = await import("../src/engine.js");
    // First run downloads the embedder model (transformers.js) and loads the
    // ~100MB wasm binary, so allow a generous budget.
    await initializeEngine();
  }, 180_000);

  it("wasm engine.decrypt_map decrypts a map written by TS encryptMapFile", async () => {
    const { encryptMapFile } = await import("../src/redaction/rehydration.js");
    const { readFileSync } = await import("node:fs");
    const { getEngine } = await import("../src/engine.js");

    const mapPath = join(tmpdir(), `xberg-compat-${Date.now()}.map`);
    const original = { "[EMAIL_1]": "jane@example.com", "[PHONE_1]": "555-1234567" };
    const passphrase = "test-passphrase-32bytes-long!!!!";

    try {
      encryptMapFile(mapPath, original, passphrase);
      const bytes = readFileSync(mapPath);

      const engine = getEngine();
      const decrypted = engine.decrypt_map(new Uint8Array(bytes), passphrase);

      const tokenMap: Record<string, string> =
        decrypted instanceof Map ? Object.fromEntries(decrypted) : decrypted;

      expect(tokenMap).toEqual(original);
    } finally {
      try { unlinkSync(mapPath); } catch { /* ignore */ }
    }
  }, 60_000);

  it("wasm engine.decrypt_map throws on wrong passphrase", async () => {
    const { encryptMapFile } = await import("../src/redaction/rehydration.js");
    const { readFileSync } = await import("node:fs");
    const { getEngine } = await import("../src/engine.js");

    const mapPath = join(tmpdir(), `xberg-compat-wrong-${Date.now()}.map`);
    const original = { "[EMAIL_1]": "jane@example.com" };
    const passphrase = "correct-passphrase-32bytes-long!";

    try {
      encryptMapFile(mapPath, original, passphrase);
      const bytes = readFileSync(mapPath);
      const engine = getEngine();

      expect(() => engine.decrypt_map(new Uint8Array(bytes), "wrong-passphrase")).toThrow();
    } finally {
      try { unlinkSync(mapPath); } catch { /* ignore */ }
    }
  }, 60_000);
});

describe("rehydrate_document tool (wasm engine, end-to-end)", () => {
  let client: import("@modelcontextprotocol/sdk/client/index.js").Client;
  let rehydrationDir: string;

  beforeAll(async () => {
    const { initializeEngine } = await import("../src/engine.js");
    const { registerRehydrateTools } = await import("../src/tools/rehydrate.js");
    const { McpServer } = await import("@modelcontextprotocol/sdk/server/mcp.js");
    const { Client } = await import("@modelcontextprotocol/sdk/client/index.js");
    const { InMemoryTransport } = await import("@modelcontextprotocol/sdk/inMemory.js");

    await initializeEngine();

    const server = new McpServer({ name: "test-server", version: "0.0.0" });
    registerRehydrateTools(server);

    const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair();
    client = new Client({ name: "test-client", version: "0.0.0" });
    await Promise.all([
      server.connect(serverTransport),
      client.connect(clientTransport),
    ]);

    rehydrationDir = mkdtempSync(join(tmpdir(), "xberg-rehydrate-"));
  }, 180_000);

  afterAll(async () => {
    await client?.close();
    try { rmSync(rehydrationDir, { recursive: true, force: true }); } catch { /* ignore */ }
  });

  it("decrypts a map file in-wasm and returns token_map", async () => {
    const { encryptMapFile } = await import("../src/redaction/rehydration.js");

    const documentId = "doc-1";
    const passphrase = "e2e-passphrase-32bytes-long!!!!!";
    const original = { "[EMAIL_1]": "jane@example.com", "[PHONE_1]": "555-1234567" };
    const mapPath = join(rehydrationDir, `${documentId}.map`);
    encryptMapFile(mapPath, original, passphrase);

    const result = await client.callTool({
      name: "rehydrate_document",
      arguments: {
        document_id: documentId,
        passphrase,
        rehydration_dir: rehydrationDir,
      },
    });

    expect(result.isError).not.toBe(true);
    const content = (result.content as Array<{ type: string; text: string }>)[0];
    const parsed = JSON.parse(content!.text) as { token_map: Record<string, string> };
    expect(parsed.token_map).toEqual(original);
  }, 60_000);

  it("returns an error for a missing map file", async () => {
    const result = await client.callTool({
      name: "rehydrate_document",
      arguments: {
        document_id: "does-not-exist",
        passphrase: "whatever",
        rehydration_dir: rehydrationDir,
      },
    });
    expect(result.isError).toBe(true);
  });

  it("returns an error for wrong passphrase", async () => {
    const { encryptMapFile } = await import("../src/redaction/rehydration.js");

    const documentId = "doc-2";
    const mapPath = join(rehydrationDir, `${documentId}.map`);
    encryptMapFile(mapPath, { "[EMAIL_1]": "a@b.com" }, "right-passphrase-32bytes-long!!!");

    const result = await client.callTool({
      name: "rehydrate_document",
      arguments: {
        document_id: documentId,
        passphrase: "wrong-passphrase",
        rehydration_dir: rehydrationDir,
      },
    });
    expect(result.isError).toBe(true);
  }, 60_000);
});

describe("find_pii_subject / forget_pii_subject tools (wasm engine, end-to-end)", () => {
  let client: import("@modelcontextprotocol/sdk/client/index.js").Client;
  let rehydrationDir: string;

  beforeAll(async () => {
    const { initializeEngine } = await import("../src/engine.js");
    const { registerRehydrateTools } = await import("../src/tools/rehydrate.js");
    const { McpServer } = await import("@modelcontextprotocol/sdk/server/mcp.js");
    const { Client } = await import("@modelcontextprotocol/sdk/client/index.js");
    const { InMemoryTransport } = await import("@modelcontextprotocol/sdk/inMemory.js");

    await initializeEngine();

    const server = new McpServer({ name: "test-server", version: "0.0.0" });
    registerRehydrateTools(server);

    const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair();
    client = new Client({ name: "test-client", version: "0.0.0" });
    await Promise.all([
      server.connect(serverTransport),
      client.connect(clientTransport),
    ]);

    rehydrationDir = mkdtempSync(join(tmpdir(), "xberg-forget-subject-"));
  }, 180_000);

  afterAll(async () => {
    await client?.close();
    try { rmSync(rehydrationDir, { recursive: true, force: true }); } catch { /* ignore */ }
  });

  it("find_pii_subject matches by case-insensitive original-value substring and by exact token", async () => {
    const { encryptMapFile } = await import("../src/redaction/rehydration.js");

    const documentId = "find-doc-1";
    const passphrase = "find-passphrase-32bytes-long!!!!";
    const original = { "[PERSON_1]": "Alice Johnson", "[EMAIL_1]": "jane.doe@example.com" };
    const mapPath = join(rehydrationDir, `${documentId}.map`);
    encryptMapFile(mapPath, original, passphrase);

    const bySubstring = await client.callTool({
      name: "find_pii_subject",
      arguments: { document_id: documentId, passphrase, query: "johnson", rehydration_dir: rehydrationDir },
    });
    expect(bySubstring.isError).not.toBe(true);
    const substringContent = (bySubstring.content as Array<{ type: string; text: string }>)[0];
    const substringParsed = JSON.parse(substringContent!.text) as {
      matches: Array<{ token: string; original: string; category: string | null }>;
    };
    expect(substringParsed.matches).toHaveLength(1);
    expect(substringParsed.matches[0]).toMatchObject({ token: "[PERSON_1]", original: "Alice Johnson" });

    const byToken = await client.callTool({
      name: "find_pii_subject",
      arguments: { document_id: documentId, passphrase, query: "[EMAIL_1]", rehydration_dir: rehydrationDir },
    });
    expect(byToken.isError).not.toBe(true);
    const tokenContent = (byToken.content as Array<{ type: string; text: string }>)[0];
    const tokenParsed = JSON.parse(tokenContent!.text) as {
      matches: Array<{ token: string; original: string; category: string | null }>;
    };
    expect(tokenParsed.matches).toHaveLength(1);
    expect(tokenParsed.matches[0]).toMatchObject({ token: "[EMAIL_1]", original: "jane.doe@example.com" });
  }, 60_000);

  it("find_pii_subject returns no matches for an unknown query, and does not modify the map file", async () => {
    const { encryptMapFile } = await import("../src/redaction/rehydration.js");

    const documentId = "find-doc-2";
    const passphrase = "find-passphrase-32bytes-long!!!!";
    const original = { "[PERSON_1]": "Alice Johnson" };
    const mapPath = join(rehydrationDir, `${documentId}.map`);
    encryptMapFile(mapPath, original, passphrase);
    const bytesBefore = readFileSync(mapPath);

    const result = await client.callTool({
      name: "find_pii_subject",
      arguments: { document_id: documentId, passphrase, query: "nonexistent", rehydration_dir: rehydrationDir },
    });
    expect(result.isError).not.toBe(true);
    const content = (result.content as Array<{ type: string; text: string }>)[0];
    const parsed = JSON.parse(content!.text) as { matches: unknown[] };
    expect(parsed.matches).toHaveLength(0);

    expect(readFileSync(mapPath).equals(bytesBefore)).toBe(true);
  }, 60_000);

  it("forget_pii_subject removes the matching subject, overwrites the map file on disk, and is idempotent", async () => {
    const { encryptMapFile } = await import("../src/redaction/rehydration.js");
    const { getEngine } = await import("../src/engine.js");

    const documentId = "forget-doc-1";
    const passphrase = "forget-passphrase-32bytes-long!!";
    const original = { "[PERSON_1]": "Alice Johnson", "[EMAIL_1]": "jane.doe@example.com" };
    const mapPath = join(rehydrationDir, `${documentId}.map`);
    encryptMapFile(mapPath, original, passphrase);
    const bytesBefore = readFileSync(mapPath);

    const result = await client.callTool({
      name: "forget_pii_subject",
      arguments: { document_id: documentId, passphrase, query: "johnson", rehydration_dir: rehydrationDir },
    });
    expect(result.isError).not.toBe(true);
    const content = (result.content as Array<{ type: string; text: string }>)[0];
    const receipt = JSON.parse(content!.text) as {
      subject_ref: string;
      removed_count: number;
      removed_tokens: string[];
    };
    expect(receipt).toEqual({ subject_ref: "johnson", removed_count: 1, removed_tokens: ["[PERSON_1]"] });

    // The file on disk must actually have changed (not just the in-memory map).
    const bytesAfter = readFileSync(mapPath);
    expect(bytesAfter.equals(bytesBefore)).toBe(false);

    // Decrypting the overwritten file must confirm the subject is genuinely
    // gone and the untouched entry survives.
    const engine = getEngine();
    const decrypted = engine.decrypt_map(new Uint8Array(bytesAfter), passphrase);
    const tokenMap: Record<string, string> =
      decrypted instanceof Map ? Object.fromEntries(decrypted) : decrypted;
    expect(tokenMap).toEqual({ "[EMAIL_1]": "jane.doe@example.com" });

    // Idempotent: a second call finds nothing left to remove and does not error.
    const second = await client.callTool({
      name: "forget_pii_subject",
      arguments: { document_id: documentId, passphrase, query: "johnson", rehydration_dir: rehydrationDir },
    });
    expect(second.isError).not.toBe(true);
    const secondContent = (second.content as Array<{ type: string; text: string }>)[0];
    const secondReceipt = JSON.parse(secondContent!.text) as { removed_count: number };
    expect(secondReceipt.removed_count).toBe(0);
  }, 60_000);

  it("forget_pii_subject returns an error for a missing map file", async () => {
    const result = await client.callTool({
      name: "forget_pii_subject",
      arguments: {
        document_id: "does-not-exist",
        passphrase: "whatever",
        query: "whoever",
        rehydration_dir: rehydrationDir,
      },
    });
    expect(result.isError).toBe(true);
  });
});

import { describe, it, expect, beforeAll, afterAll } from "vitest";
import { createRequire } from "node:module";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { mkdtempSync, unlinkSync, rmSync } from "node:fs";

// `rehydrate.ts` imports `getCacheDir` from `../store.js`, which loads the
// native xberg-rag-node binding (.node binary) at module scope. That binding
// is only present when the crate has been built locally; in CI the MCP
// unit-test job does not compile it. Detect its absence up front and skip
// the tool-level e2e suite instead of crashing at import time.
function nativeBindingAvailable(): boolean {
  try {
    createRequire(import.meta.url)("xberg-rag-node");
    return true;
  } catch {
    return false;
  }
}

const HAVE_NATIVE = nativeBindingAvailable();

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

      // Confirm actual shape returned by serde_wasm_bindgen for a Rust HashMap.
      const tag = Object.prototype.toString.call(decrypted);
      // eslint-disable-next-line no-console
      console.log("decrypt_map result type:", tag, decrypted instanceof Map);

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

describe.skipIf(!HAVE_NATIVE)("rehydrate_document tool (wasm engine, end-to-end)", () => {
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

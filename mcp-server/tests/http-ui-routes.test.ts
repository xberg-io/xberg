import { describe, it, expect, beforeAll, afterAll } from "vitest";
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { initializeEngine, getRuntime } from "../src/engine.js";
import { startHttp, type HttpHandle } from "../src/transports/http.js";
import { getCacheDir } from "../src/paths.js";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { EMBEDDING_DIM } from "../src/lib/constants.js";

describe("HTTP ingest/map/ui routes (Task 6)", () => {
  let handle: HttpHandle;
  let baseUrl: string;
  let token: string;

  beforeAll(async () => {
    await initializeEngine();
    const { store } = getRuntime();
    await store.ensureCollection({ name: "http_ingest_test", embedding_dim: EMBEDDING_DIM });

    const server = new McpServer({ name: "test", version: "0.0.0" });
    handle = await startHttp(server, { port: 0, host: "127.0.0.1" });
    token = handle.uiToken;
    baseUrl = `http://127.0.0.1:${handle.port}`;
  }, 180_000);

  afterAll(async () => {
    await handle.close();
  });

  it("rejects /ingest without a valid token", async () => {
    const res = await fetch(`${baseUrl}/ingest`, { method: "POST", body: "{}" });
    expect(res.status).toBe(401);
  });

  it("POST /collection creates a collection ingest can then target", async () => {
    const res = await fetch(`${baseUrl}/collection?token=${token}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ name: "http_collection_route_test", embedding_dim: EMBEDDING_DIM }),
    });
    expect(res.status).toBe(200);
    expect(await res.json()).toEqual({ created: true });
  });

  it("POST /ingest stores a document via the runtime store", async () => {
    const payload = {
      collection: "http_ingest_test",
      external_id: "doc-1",
      title: "Test doc",
      full_text: "Redacted text with [EMAIL_1] token.",
      chunks: [
        { ordinal: 0, content: "Redacted text with [EMAIL_1] token.", embedding: Array(EMBEDDING_DIM).fill(0.01) },
      ],
    };
    const res = await fetch(`${baseUrl}/ingest?token=${token}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    expect(res.status).toBe(200);
    const body = (await res.json()) as { document_id: string };
    expect(typeof body.document_id).toBe("string");

    const { store } = getRuntime();
    const stats = await store.collectionStats("http_ingest_test");
    expect(stats.documents).toBeGreaterThanOrEqual(1);
  }, 60_000);

  it("POST /ingest with an unknown collection returns 404", async () => {
    const payload = { collection: "does_not_exist", external_id: "doc-x", full_text: "text", chunks: [] };
    const res = await fetch(`${baseUrl}/ingest?token=${token}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    expect(res.status).toBe(404);
  });

  it("POST /map stores an encrypted blob at the path rehydrate_document reads", async () => {
    // document_id matches the `external_id` used in the /ingest test above,
    // per this plan's convention (not the store-generated document_id).
    const blob = Buffer.from("XPII\x01fake-encrypted-bytes");
    const res = await fetch(`${baseUrl}/map?token=${token}&document_id=doc-1`, {
      method: "POST",
      body: blob,
    });
    expect(res.status).toBe(200);

    // Verify the file was written with the correct content
    const rehydrationDir = join(getCacheDir(), "rehydration");
    const mapPath = join(rehydrationDir, "doc-1.map");
    const written = readFileSync(mapPath);
    expect(written).toEqual(blob);
  });

  it("GET /ui serves the static placeholder with cross-origin isolation headers", async () => {
    const res = await fetch(`${baseUrl}/ui/?token=${token}`);
    expect(res.status).toBe(200);
    expect(res.headers.get("cross-origin-opener-policy")).toBe("same-origin");
    expect(res.headers.get("cross-origin-embedder-policy")).toBe("require-corp");
  });

  it("GET /ui without a token is rejected", async () => {
    const res = await fetch(`${baseUrl}/ui/`);
    expect(res.status).toBe(401);
  });
});

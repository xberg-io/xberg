// mcp-server/tests/http-ingest-route.test.ts
import { describe, it, expect } from "vitest";
import { createServer, type Server } from "node:http";
import type { VectorStoreInterface, DocumentRecord, ChunkRecord } from "xberg-wasm-runtime";
import { createIngestHandler } from "../src/http/ingest-route.js";

function notImplemented(name: string) {
  return async () => {
    throw new Error(`${name} not implemented in fake store`);
  };
}

function makeFakeStore(overrides: Partial<VectorStoreInterface> = {}): VectorStoreInterface {
  return {
    close: notImplemented("close"),
    ensureCollection: notImplemented("ensureCollection"),
    dropCollection: notImplemented("dropCollection"),
    getCollection: notImplemented("getCollection"),
    upsertDocument: notImplemented("upsertDocument"),
    deleteDocuments: notImplemented("deleteDocuments"),
    deleteByFilter: notImplemented("deleteByFilter"),
    retrieve: notImplemented("retrieve"),
    collectionStats: notImplemented("collectionStats"),
    ...overrides,
  } as VectorStoreInterface;
}

async function withServer(
  store: VectorStoreInterface,
  fn: (baseUrl: string) => Promise<void>
): Promise<void> {
  const handler = createIngestHandler(() => store);
  const server: Server = createServer((req, res) => {
    void handler(req, res);
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address();
  if (address === null || typeof address === "string") throw new Error("expected AddressInfo");
  try {
    await fn(`http://127.0.0.1:${address.port}`);
  } finally {
    await new Promise<void>((resolve, reject) => server.close((err) => (err ? reject(err) : resolve())));
  }
}

describe("http/ingest-route", () => {
  it("upserts a valid payload and returns the document id", async () => {
    let received: { collection: string; doc: DocumentRecord; chunks: ChunkRecord[] } | null = null;
    const store = makeFakeStore({
      upsertDocument: async (collection, doc, chunks) => {
        received = { collection, doc, chunks };
        return "doc-123";
      },
    });

    await withServer(store, async (baseUrl) => {
      const res = await fetch(`${baseUrl}/ingest`, {
        method: "POST",
        body: JSON.stringify({
          collection: "c1",
          external_id: "doc-1",
          full_text: "hello [EMAIL_1]",
          chunks: [{ ordinal: 0, content: "hello [EMAIL_1]", embedding: [0.1, 0.2, 0.3, 0.4] }],
        }),
      });
      expect(res.status).toBe(200);
      const body = (await res.json()) as { document_id: string };
      expect(body.document_id).toBe("doc-123");
    });

    expect(received).not.toBeNull();
    expect(received!.collection).toBe("c1");
    expect(received!.doc.external_id).toBe("doc-1");
    expect(received!.chunks).toHaveLength(1);
  });

  it("rejects an invalid payload with 400", async () => {
    const store = makeFakeStore();
    await withServer(store, async (baseUrl) => {
      const res = await fetch(`${baseUrl}/ingest`, { method: "POST", body: JSON.stringify({ collection: "c1" }) });
      expect(res.status).toBe(400);
    });
  });

  it("maps a 'not found' store error to 404", async () => {
    const store = makeFakeStore({
      upsertDocument: async () => {
        throw new Error("collection not found: missing");
      },
    });
    await withServer(store, async (baseUrl) => {
      const res = await fetch(`${baseUrl}/ingest`, {
        method: "POST",
        body: JSON.stringify({ collection: "missing", external_id: "d", full_text: "t", chunks: [] }),
      });
      expect(res.status).toBe(404);
    });
  });

  it("rejects a non-JSON body with 400", async () => {
    const store = makeFakeStore();
    await withServer(store, async (baseUrl) => {
      const res = await fetch(`${baseUrl}/ingest`, { method: "POST", body: "not json {" });
      expect(res.status).toBe(400);
    });
  });

  it("maps a dimension-mismatch store error to 400", async () => {
    const store = makeFakeStore({
      upsertDocument: async () => {
        throw new Error("embedding dimension mismatch: expected 4, got 2");
      },
    });
    await withServer(store, async (baseUrl) => {
      const res = await fetch(`${baseUrl}/ingest`, {
        method: "POST",
        body: JSON.stringify({
          collection: "c1",
          external_id: "d",
          full_text: "t",
          chunks: [{ ordinal: 0, content: "t", embedding: [0.1, 0.2] }],
        }),
      });
      expect(res.status).toBe(400);
    });
  });
});

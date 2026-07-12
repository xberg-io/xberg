import { describe, it, expect } from "vitest";
import { createServer, type Server } from "node:http";
import type { VectorStoreInterface } from "xberg-wasm-runtime";
import { createCollectionHandler } from "../src/http/collection-route.js";

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
async function withServer(store: VectorStoreInterface, fn: (baseUrl: string) => Promise<void>): Promise<void> {
  const handler = createCollectionHandler(() => store);
  const server: Server = createServer((req, res) => {
    void handler(req, res);
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address();
  if (address === null || typeof address === "string") throw new Error("expected an AddressInfo");
  try {
    await fn(`http://127.0.0.1:${address.port}`);
  } finally {
    await new Promise<void>((resolve, reject) => server.close((err) => (err ? reject(err) : resolve())));
  }
}

describe("http/collection-route", () => {
  it("ensures a collection and returns { created: true }", async () => {
    let received: unknown = null;
    const store = makeFakeStore({
      ensureCollection: async (spec) => {
        received = spec;
        return undefined;
      },
    });
    await withServer(store, async (baseUrl) => {
      const res = await fetch(`${baseUrl}/collection`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name: "dossier-1", embedding_dim: 1024 }),
      });
      expect(res.status).toBe(200);
      expect(await res.json()).toEqual({ created: true });
    });
    expect(received).toEqual({ name: "dossier-1", embedding_dim: 1024 });
  });

  it("rejects an invalid payload with 400", async () => {
    const store = makeFakeStore();
    await withServer(store, async (baseUrl) => {
      const res = await fetch(`${baseUrl}/collection`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name: "" }),
      });
      expect(res.status).toBe(400);
    });
  });

  it("rejects invalid JSON with 400", async () => {
    const store = makeFakeStore();
    await withServer(store, async (baseUrl) => {
      const res = await fetch(`${baseUrl}/collection`, { method: "POST", body: "{not json" });
      expect(res.status).toBe(400);
    });
  });

  it("maps a store error (thrown) to 400", async () => {
    const store = makeFakeStore({
      ensureCollection: async () => {
        throw new Error("invalid distance metric");
      },
    });
    await withServer(store, async (baseUrl) => {
      const res = await fetch(`${baseUrl}/collection`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name: "c1", embedding_dim: 1024 }),
      });
      expect(res.status).toBe(400);
    });
  });

  it("maps a store error (returned as a string, per ensureCollection's real contract) to 400", async () => {
    // `ensureCollection` reports failure by *resolving* with an error
    // string, not by throwing — this is the actual behavior documented on
    // `VectorStoreInterface.ensureCollection` in `xberg-wasm-runtime`, and
    // is a distinct failure mode from the thrown-error test above.
    const store = makeFakeStore({
      ensureCollection: async () => "dimension mismatch: collection exists with dim 512",
    });
    await withServer(store, async (baseUrl) => {
      const res = await fetch(`${baseUrl}/collection`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name: "c1", embedding_dim: 1024 }),
      });
      expect(res.status).toBe(400);
      expect((await res.json()).error).toContain("dimension mismatch");
    });
  });
});

import { describe, it, expect } from "vitest";
import { createServer, type Server } from "node:http";
import type { VectorStoreInterface, CollectionStats } from "xberg-wasm-runtime";
import { createAdminHandler } from "../src/http/admin-route.js";

function notImpl(name: string) { return async () => { throw new Error(`${name} not implemented`); }; }
function fakeStore(over: Partial<VectorStoreInterface> = {}): VectorStoreInterface {
  return {
    ensureCollection: notImpl("ensureCollection"), getCollection: notImpl("getCollection"),
    upsertDocument: notImpl("upsertDocument"), retrieve: notImpl("retrieve"),
    deleteByFilter: notImpl("deleteByFilter"),
    ...over,
  } as VectorStoreInterface;
}
async function withServer(store: VectorStoreInterface, fn: (base: string) => Promise<void>) {
  const handler = createAdminHandler(() => store);
  const server: Server = createServer((req, res) => { void handler(req, res, new URL(req.url ?? "/", "http://localhost")); });
  await new Promise<void>((r) => server.listen(0, "127.0.0.1", r));
  const a = server.address(); if (a === null || typeof a === "string") throw new Error("addr");
  try { await fn(`http://127.0.0.1:${a.port}`); } finally { await new Promise<void>((r) => server.close((e) => (e ? console.error(e) : r()))); }
}
describe("http/admin-route", () => {
  it("drop_collection returns { dropped: true }", async () => {
    let called = ""; const store = fakeStore({ dropCollection: async (c) => { called = c; return undefined; } });
    await withServer(store, async (base) => {
      const res = await fetch(`${base}/admin`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ op: "drop_collection", collection: "c1" }) });
      expect(res.status).toBe(200); expect((await res.json())).toEqual({ dropped: true }); expect(called).toBe("c1");
    });
  });
  it("drop_collection returns an error response when the store reports failure", async () => {
    const store = fakeStore({ dropCollection: async () => "collection not found" });
    await withServer(store, async (base) => {
      const res = await fetch(`${base}/admin`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ op: "drop_collection", collection: "c1" }) });
      expect(res.status).not.toBe(200);
      expect((await res.json())).toEqual({ error: "collection not found" });
    });
  });
  it("delete_documents by external_ids returns the deleted count", async () => {
    let got: string[] = []; const store = fakeStore({ deleteDocuments: async (_c, ids) => { got = ids; return 2; } });
    await withServer(store, async (base) => {
      const res = await fetch(`${base}/admin`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ op: "delete_documents", collection: "c1", external_ids: ["a.pdf", "b.pdf"] }) });
      expect(res.status).toBe(200); expect((await res.json())).toEqual({ deleted: 2 }); expect(got).toEqual(["a.pdf", "b.pdf"]);
    });
  });
  it("stats returns collection stats", async () => {
    const stats: CollectionStats = { documents: 3, chunks: 9 }; const store = fakeStore({ collectionStats: async () => stats });
    await withServer(store, async (base) => {
      const res = await fetch(`${base}/admin`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ op: "stats", collection: "c1" }) });
      expect(res.status).toBe(200); expect((await res.json())).toEqual(stats);
    });
  });
  it("rejects an unknown op with 400", async () => {
    const store = fakeStore();
    await withServer(store, async (base) => {
      const res = await fetch(`${base}/admin`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ op: "bogus" }) });
      expect(res.status).toBe(400);
    });
  });
  it("malformed JSON body returns 400", async () => {
    const store = fakeStore();
    await withServer(store, async (base) => {
      const res = await fetch(`${base}/admin`, { method: "POST", headers: { "Content-Type": "application/json" }, body: "{ not valid json" });
      expect(res.status).toBe(400);
    });
  });
});

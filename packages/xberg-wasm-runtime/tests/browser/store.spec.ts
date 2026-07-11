import { expect, test } from "@playwright/test";

test("persists and queries vectors through the real OPFS worker", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto("/tests/browser/");
  await page.waitForFunction(() => typeof (globalThis as any).createTestStore === "function");

  const path = `/browser-${Date.now()}.sqlite3`;
  const first = await page.evaluate(async (databasePath) => Promise.race([(async () => {
    const store = await (globalThis as any).createTestStore(databasePath);
    await store.ensureCollection("browser-docs", 4);
    await store.upsertDocument(
      "browser-docs",
      { documentId: "doc-1", sourceId: "source-1", collectionId: "browser-docs" },
      [{
        sourceId: "source-1",
        chunkIndex: 0,
        text: "persistent browser vector",
        startOffset: 0,
        endOffset: 25,
        embedding: new Float32Array([1, 0, 0, 0]),
      }],
    );
    const result = await store.query("browser-docs", [1, 0, 0, 0], 1);
    await store.close();
    return result;
  })(), new Promise<never>((_, reject) => setTimeout(() => reject(new Error("browser store scenario timed out")), 20_000))]), path);
  expect(first[0]?.text).toBe("persistent browser vector");

  await page.reload();
  const persisted = await page.evaluate(async (databasePath) => Promise.race([(async () => {
    const store = await (globalThis as any).createTestStore(databasePath);
    const result = await store.query("browser-docs", [1, 0, 0, 0], 1);
    await store.createEdge({ id: "edge-1", source: "source-1", target: "source-2", label: "references" });
    const graph = await store.traverseGraph(["source-1"], 1, ["references"]);
    await store.close();
    return { result, graph };
  })(), new Promise<never>((_, reject) => setTimeout(() => reject(new Error("browser store reload timed out")), 20_000))]), path);
  expect(persisted.result[0]?.text).toBe("persistent browser vector");
  expect(persisted.graph).toEqual(expect.arrayContaining(["source-1", "source-2"]));
  expect(errors).toEqual([]);
});

test("isolates colliding collection names and supports delete/drop", async ({ page }) => {
  await page.goto("/tests/browser/");
  await page.waitForFunction(() => typeof (globalThis as any).createTestStore === "function");
  const result = await page.evaluate(async (databasePath) => {
    const store = await (globalThis as any).createTestStore(databasePath);
    for (const collection of ["test-docs", "test_docs"]) {
      await store.ensureCollection(collection, 4);
      await store.upsertDocument(
        collection,
        { documentId: "same-doc", sourceId: "same-source", collectionId: collection },
        [{
          sourceId: "same-source",
          chunkIndex: 0,
          text: collection,
          startOffset: 0,
          endOffset: collection.length,
          embedding: new Float32Array([1, 0, 0, 0]),
        }],
      );
    }
    const first = await store.query("test-docs", [1, 0, 0, 0], 1);
    const second = await store.query("test_docs", [1, 0, 0, 0], 1);
    await store.delete("test-docs", "same-doc");
    const deleted = await store.query("test-docs", [1, 0, 0, 0], 1);
    await store.dropCollection("test_docs");
    const collections = await store.listCollections();
    await store.close();
    return { first, second, deleted, collections };
  }, `/isolation-${Date.now()}.sqlite3`);

  expect(result.first[0]?.text).toBe("test-docs");
  expect(result.second[0]?.text).toBe("test_docs");
  expect(result.deleted).toEqual([]);
  expect(result.collections).toEqual(["test-docs"]);
});

test("FTS5 is compiled into the vendored sqlite3.wasm build", async ({ page }) => {
  await page.goto("/tests/browser/");
  await page.waitForFunction(() => typeof (globalThis as any).createTestStore === "function");
  const hasFts5 = await page.evaluate(async (databasePath) => {
    const store = await (globalThis as any).createTestStore(databasePath);
    // Deliberately calling retrieve() in fulltext mode against a real chunk
    // is the load-bearing check here, not a synthetic pragma query — this is
    // the exact operation the design spec requires to fail loudly (not
    // silently fall back to vector-only) if the vendored WASM build ever
    // stops shipping ENABLE_FTS5.
    await store.ensureCollection("fts5-check", 4);
    await store.upsertDocument(
      "fts5-check",
      { documentId: "d1", sourceId: "s1", collectionId: "fts5-check" },
      [{ sourceId: "s1", chunkIndex: 0, text: "fts5 availability probe", startOffset: 0, endOffset: 24, embedding: new Float32Array([1, 0, 0, 0]) }],
    );
    const results = await store.retrieve("fts5-check", { mode: "fulltext", queryText: "availability probe", k: 1 });
    return results.length > 0 && results[0].text === "fts5 availability probe";
  }, `/fts5-check-${Date.now()}.sqlite3`);
  expect(hasFts5).toBe(true);
});

test("retrieve() hybrid mode works through the real Worker/OPFS path", async ({ page }) => {
  await page.goto("/tests/browser/");
  await page.waitForFunction(() => typeof (globalThis as any).createTestStore === "function");
  const topChunkId = await page.evaluate(async (databasePath) => {
    const store = await (globalThis as any).createTestStore(databasePath);
    await store.ensureCollection("hybrid-check", 4);
    // Fixture verified against the real sqlite-wasm + sqlite-vec + FTS5 engine (not assumed):
    // - FTS5 MATCH ANDs bareword terms by default, so the query text is kept to terms every
    //   textually-relevant chunk actually contains ("hybrid search"), otherwise a partial text
    //   match is excluded from the fulltext ranking entirely rather than ranked lower.
    // - Two vector-only filler chunks push chunk 1's vector rank down to 5th so RRF's convex
    //   1/(k+rank) sum genuinely favors chunk 2 (moderate rank 2 + rank 2) over chunk 1
    //   (best-possible text rank 1 offset by a much worse vector rank) instead of tying/losing.
    await store.upsertDocument(
      "hybrid-check",
      { documentId: "d1", sourceId: "s1", collectionId: "hybrid-check" },
      [
        // Exact vector match, textually irrelevant to the query text.
        { sourceId: "s1", chunkIndex: 0, text: "zzz unrelated content", startOffset: 0, endOffset: 22, embedding: new Float32Array([1, 0, 0, 0]) },
        // Textually exact, vector-distant.
        { sourceId: "s1", chunkIndex: 1, text: "hybrid search", startOffset: 23, endOffset: 37, embedding: new Float32Array([0, 0, 0, 1]) },
        // Moderately good on both.
        { sourceId: "s1", chunkIndex: 2, text: "hybrid search related content", startOffset: 38, endOffset: 68, embedding: new Float32Array([0.7, 0, 0, 0.7]) },
        // Vector-only filler, textually irrelevant: closer to the query vector than chunk 1,
        // pushing chunk 1 further down the vector ranking.
        { sourceId: "s1", chunkIndex: 3, text: "filler padding words one", startOffset: 69, endOffset: 94, embedding: new Float32Array([0.2, 0, 0, 0.6]) },
        // Second vector-only filler, same purpose as chunk 3.
        { sourceId: "s1", chunkIndex: 4, text: "filler padding words two", startOffset: 95, endOffset: 120, embedding: new Float32Array([0.1, 0, 0, 0.8]) },
      ],
    );
    const results = await store.retrieve("hybrid-check", {
      mode: "hybrid",
      queryVector: [1, 0, 0, 0],
      queryText: "hybrid search",
      k: 3,
    });
    return results[0]?.chunkId;
  }, `/hybrid-check-${Date.now()}.sqlite3`);
  expect(topChunkId).toBe("s1:2");
});

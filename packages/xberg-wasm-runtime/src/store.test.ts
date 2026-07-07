import { describe, it, expect, beforeEach } from "vitest";
import { createVectorStore } from "./store";
import type { VectorStoreInterface, DocumentRecord, ChunkRecord } from "./types";

describe("vector store", () => {
  let store: VectorStoreInterface;
  const testCollection = "test-docs";
  const vectorDim = 384;

  beforeEach(async () => {
    store = await createVectorStore();
  });

  it("ensures a collection", async () => {
    await store.ensureCollection(testCollection, vectorDim);
    const collections = await store.listCollections();
    expect(collections).toContain(testCollection);
  });

  it("upserts a document with chunks idempotently", async () => {
    await store.ensureCollection(testCollection, vectorDim);

    const doc: DocumentRecord = {
      documentId: "doc-1",
      sourceId: "src-1",
      collectionId: testCollection,
      metadata: { title: "Test" },
    };

    const chunk: ChunkRecord = {
      sourceId: "src-1",
      chunkIndex: 0,
      text: "hello world",
      startOffset: 0,
      endOffset: 11,
      embedding: new Float32Array(vectorDim).fill(0.1),
    };

    const result1 = await store.upsertDocument(testCollection, doc, [chunk]);
    expect(result1.chunksCount).toBe(1);

    // Upsert same document again (idempotent)
    const result2 = await store.upsertDocument(testCollection, doc, [chunk]);
    expect(result2.chunksCount).toBe(1);
  });

  it("queries and returns results sorted by score desc", async () => {
    await store.ensureCollection(testCollection, vectorDim);

    const doc: DocumentRecord = {
      documentId: "doc-1",
      sourceId: "src-1",
      collectionId: testCollection,
    };

    const chunks: ChunkRecord[] = [
      {
        sourceId: "src-1",
        chunkIndex: 0,
        text: "apple fruit",
        startOffset: 0,
        endOffset: 11,
        embedding: new Float32Array([1, 0, 0, ...new Array(vectorDim - 3).fill(0)]),
      },
      {
        sourceId: "src-1",
        chunkIndex: 1,
        text: "apple tree",
        startOffset: 12,
        endOffset: 22,
        embedding: new Float32Array([0.9, 0, 0, ...new Array(vectorDim - 3).fill(0)]),
      },
    ];

    await store.upsertDocument(testCollection, doc, chunks);

    const queryVec = Array.from(
      new Float32Array([1, 0, 0, ...new Array(vectorDim - 3).fill(0)])
    );
    const results = await store.query(testCollection, queryVec, 5);

    expect(results.length).toBeGreaterThan(0);
    // Results should be sorted by score descending
    for (let i = 1; i < results.length; i++) {
      const prevResult = results[i - 1];
      const currResult = results[i];
      if (prevResult !== undefined && currResult !== undefined) {
        expect(prevResult.score).toBeGreaterThanOrEqual(currResult.score);
      }
    }
  });

  it("deletes a document", async () => {
    await store.ensureCollection(testCollection, vectorDim);

    const doc: DocumentRecord = {
      documentId: "doc-1",
      sourceId: "src-1",
      collectionId: testCollection,
    };

    const chunk: ChunkRecord = {
      sourceId: "src-1",
      chunkIndex: 0,
      text: "hello",
      startOffset: 0,
      endOffset: 5,
      embedding: new Float32Array(vectorDim).fill(0.1),
    };

    await store.upsertDocument(testCollection, doc, [chunk]);
    await store.delete(testCollection, "doc-1");

    const results = await store.query(testCollection, Array(vectorDim).fill(0.1), 10);
    const hasDoc = results.some((r) => r.chunkId.startsWith("src-1"));
    expect(hasDoc).toBe(false);
  });

  it("drops a collection", async () => {
    await store.ensureCollection(testCollection, vectorDim);
    await store.dropCollection(testCollection);

    const collections = await store.listCollections();
    expect(collections).not.toContain(testCollection);
  });
});

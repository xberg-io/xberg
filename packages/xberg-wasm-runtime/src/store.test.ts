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
    const result = await store.ensureCollection({ name: testCollection, embedding_dim: vectorDim });
    expect(result).toBeUndefined();
    const spec = await store.getCollection(testCollection);
    expect(spec?.name).toBe(testCollection);
    expect(spec?.embedding_dim).toBe(vectorDim);
  });

  it("returns null for an unknown collection", async () => {
    const spec = await store.getCollection("nope");
    expect(spec).toBeFalsy();
  });

  it("upserts a document with chunks and returns a bare document id string", async () => {
    await store.ensureCollection({ name: testCollection, embedding_dim: vectorDim });

    const doc: DocumentRecord = {
      full_text: "hello world",
      title: "Test",
    };

    const chunk: ChunkRecord = {
      ordinal: 0,
      content: "hello world",
      embedding: new Array(vectorDim).fill(0.1),
    };

    const docId1 = await store.upsertDocument(testCollection, doc, [chunk]);
    expect(typeof docId1).toBe("string");
    expect(docId1.length).toBeGreaterThan(0);

    const stats = await store.collectionStats(testCollection);
    expect(stats.documents).toBe(1);
    expect(stats.chunks).toBe(1);
  });

  it("upserts idempotently by external_id, replacing prior chunks", async () => {
    await store.ensureCollection({ name: testCollection, embedding_dim: vectorDim });

    const doc: DocumentRecord = {
      full_text: "hello world",
      external_id: "ext-1",
    };
    const chunk: ChunkRecord = {
      ordinal: 0,
      content: "v1",
      embedding: new Array(vectorDim).fill(0.1),
    };

    const id1 = await store.upsertDocument(testCollection, doc, [chunk]);
    const id2 = await store.upsertDocument(testCollection, doc, [
      { ordinal: 0, content: "v2", embedding: new Array(vectorDim).fill(0.2) },
    ]);

    expect(id2).toBe(id1);
    const stats = await store.collectionStats(testCollection);
    expect(stats.documents).toBe(1);
    expect(stats.chunks).toBe(1);
  });

  it("retrieves and returns results sorted by score desc", async () => {
    await store.ensureCollection({ name: testCollection, embedding_dim: vectorDim });

    const doc: DocumentRecord = { full_text: "apple fruit and tree" };

    const chunks: ChunkRecord[] = [
      {
        ordinal: 0,
        content: "apple fruit",
        embedding: [1, 0, 0, ...new Array(vectorDim - 3).fill(0)],
      },
      {
        ordinal: 1,
        content: "apple tree",
        embedding: [0.9, 0, 0, ...new Array(vectorDim - 3).fill(0)],
      },
    ];

    await store.upsertDocument(testCollection, doc, chunks);

    const queryVector = [1, 0, 0, ...new Array(vectorDim - 3).fill(0)];
    const output = await store.retrieve(testCollection, {
      mode: "vector",
      top_k: 5,
      query_vector: queryVector,
      include_content: true,
    });

    expect(output.mode).toBe("vector");
    expect(output.chunks.length).toBe(2);
    for (let i = 1; i < output.chunks.length; i++) {
      const prev = output.chunks[i - 1];
      const curr = output.chunks[i];
      if (prev && curr) {
        expect(prev.score).toBeGreaterThanOrEqual(curr.score);
      }
    }
    expect(output.chunks[0]?.content).toBe("apple fruit");
  });

  it("deletes documents by id", async () => {
    await store.ensureCollection({ name: testCollection, embedding_dim: vectorDim });

    const doc: DocumentRecord = { full_text: "hello" };
    const chunk: ChunkRecord = {
      ordinal: 0,
      content: "hello",
      embedding: new Array(vectorDim).fill(0.1),
    };

    const docId = await store.upsertDocument(testCollection, doc, [chunk]);
    const removed = await store.deleteDocuments(testCollection, [docId]);
    expect(removed).toBe(1);

    const stats = await store.collectionStats(testCollection);
    expect(stats.documents).toBe(0);
    expect(stats.chunks).toBe(0);
  });

  it("deletes documents matching a filter", async () => {
    await store.ensureCollection({ name: testCollection, embedding_dim: 2 });

    await store.upsertDocument(
      testCollection,
      { full_text: "a", title: "keep" },
      [{ ordinal: 0, content: "a", embedding: [1, 0] }]
    );
    await store.upsertDocument(
      testCollection,
      { full_text: "b", title: "drop" },
      [{ ordinal: 0, content: "b", embedding: [0, 1] }]
    );

    const removed = await store.deleteByFilter(testCollection, {
      eq: { field: "doc.title", value: "drop" },
    });
    expect(removed).toBe(1);

    const stats = await store.collectionStats(testCollection);
    expect(stats.documents).toBe(1);
  });

  it("drops a collection", async () => {
    await store.ensureCollection({ name: testCollection, embedding_dim: vectorDim });
    const result = await store.dropCollection(testCollection);
    expect(result).toBeUndefined();

    const spec = await store.getCollection(testCollection);
    expect(spec).toBeFalsy();
  });

  it("returns an error string when dropping an unknown collection", async () => {
    const result = await store.dropCollection("does-not-exist");
    expect(typeof result).toBe("string");
  });
});

import { afterEach, describe, it, expect, beforeEach } from "vitest";
import { existsSync, mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createNodeVectorStore, type NodeVectorStore } from "./store-node.js";
import type { DocumentRecord, ChunkRecord } from "./types.js";

describe("node vector store (better-sqlite3 + sqlite-vec)", () => {
	let store: NodeVectorStore;
	const testCollection = "test-docs";
	const vectorDim = 4;

	beforeEach(async () => {
		store = await createNodeVectorStore({ nodeStorePath: ":memory:" });
	});

	afterEach(async () => store.close());

	it("creates a persistent database inside nodeCachePath", async () => {
		const cacheDirectory = mkdtempSync(join(tmpdir(), "xberg-store-"));
		const persistentStore = await createNodeVectorStore({ nodeCachePath: cacheDirectory });
		expect(existsSync(join(cacheDirectory, "store.sqlite3"))).toBe(true);
		await persistentStore.close();
	});

	it("validates collection names and dimensions", async () => {
		expect(await store.ensureCollection({ name: " ", embedding_dim: vectorDim })).toMatch(/must not be empty/);
		await store.ensureCollection({ name: testCollection, embedding_dim: vectorDim });
		expect(await store.ensureCollection({ name: testCollection, embedding_dim: vectorDim + 1 })).toMatch(
			/already exists/,
		);
	});

	it("rejects retrieve() with out-of-range top_k", async () => {
		await store.ensureCollection({ name: testCollection, embedding_dim: vectorDim });
		await expect(
			store.retrieve(testCollection, { top_k: 0, mode: "vector", query_vector: [1, 0, 0, 0] }),
		).rejects.toThrow("top_k");
		await expect(
			store.retrieve(testCollection, { top_k: 500, mode: "vector", query_vector: [1, 0, 0, 0] }),
		).rejects.toThrow("top_k");
	});

	it("keeps sanitized-name collisions isolated", async () => {
		await store.ensureCollection({ name: "test-docs", embedding_dim: vectorDim });
		await store.ensureCollection({ name: "test_docs", embedding_dim: vectorDim });
		const first: DocumentRecord = { full_text: "first doc" };
		const second: DocumentRecord = { full_text: "second doc" };
		await store.upsertDocument("test-docs", first, [
			{ ordinal: 0, content: "first", embedding: [1, 0, 0, 0] },
		]);
		await store.upsertDocument("test_docs", second, [
			{ ordinal: 0, content: "second", embedding: [1, 0, 0, 0] },
		]);
		const a = await store.retrieve("test-docs", { top_k: 1, mode: "vector", query_vector: [1, 0, 0, 0], include_content: true });
		const b = await store.retrieve("test_docs", { top_k: 1, mode: "vector", query_vector: [1, 0, 0, 0], include_content: true });
		expect(a.chunks[0]?.content).toBe("first");
		expect(b.chunks[0]?.content).toBe("second");
	});

	it("ensures a collection and can look it up", async () => {
		await store.ensureCollection({ name: testCollection, embedding_dim: vectorDim });
		const spec = await store.getCollection(testCollection);
		expect(spec?.embedding_dim).toBe(vectorDim);
		expect(await store.getCollection("nonexistent")).toBeNull();
	});

	it("upserts a document with chunks and retrieves by real vec0 similarity", async () => {
		await store.ensureCollection({ name: testCollection, embedding_dim: vectorDim });
		const doc: DocumentRecord = { full_text: "apple fruit apple tree" };
		const chunks: ChunkRecord[] = [
			{ ordinal: 0, content: "apple fruit", embedding: [1, 0, 0, 0] },
			{ ordinal: 1, content: "apple tree", embedding: [0, 1, 0, 0] },
		];
		const documentId = await store.upsertDocument(testCollection, doc, chunks);
		expect(typeof documentId).toBe("string");
		const out = await store.retrieve(testCollection, {
			top_k: 2,
			mode: "vector",
			query_vector: [1, 0, 0, 0],
			include_content: true,
		});
		expect(out.chunks.length).toBe(2);
		expect(out.chunks[0]?.content).toBe("apple fruit");
		expect(out.chunks[0]?.score).toBeGreaterThan(out.chunks[1]?.score ?? Infinity);
		expect(out.chunks[0]?.primary_score).toEqual({ kind: "vector", score: out.chunks[0]?.score });
	});

	it("re-upserting the same external_id replaces chunks instead of duplicating the document", async () => {
		await store.ensureCollection({ name: testCollection, embedding_dim: vectorDim });
		const doc: DocumentRecord = { full_text: "v1", external_id: "ext-1" };
		const firstId = await store.upsertDocument(testCollection, doc, [
			{ ordinal: 0, content: "v1 chunk", embedding: [1, 0, 0, 0] },
		]);
		const secondId = await store.upsertDocument(testCollection, { full_text: "v2", external_id: "ext-1" }, [
			{ ordinal: 0, content: "v2 chunk", embedding: [1, 0, 0, 0] },
		]);
		expect(secondId).toBe(firstId);
		const stats = await store.collectionStats(testCollection);
		expect(stats.documents).toBe(1);
		expect(stats.chunks).toBe(1);
	});

	it("deletes documents by id and its chunks are no longer retrievable", async () => {
		await store.ensureCollection({ name: testCollection, embedding_dim: vectorDim });
		const doc: DocumentRecord = { full_text: "hello" };
		const documentId = await store.upsertDocument(testCollection, doc, [
			{ ordinal: 0, content: "hello", embedding: [1, 0, 0, 0] },
		]);
		const removed = await store.deleteDocuments(testCollection, [documentId]);
		expect(removed).toBe(1);
		const out = await store.retrieve(testCollection, { top_k: 10, mode: "vector", query_vector: [1, 0, 0, 0] });
		expect(out.chunks).toHaveLength(0);
	});

	it("deleteByFilter removes documents matching a filter", async () => {
		await store.ensureCollection({ name: testCollection, embedding_dim: vectorDim });
		await store.upsertDocument(testCollection, { full_text: "keep", title: "keep-me" }, [
			{ ordinal: 0, content: "keep", embedding: [1, 0, 0, 0] },
		]);
		await store.upsertDocument(testCollection, { full_text: "drop", title: "drop-me" }, [
			{ ordinal: 0, content: "drop", embedding: [0, 1, 0, 0] },
		]);
		const removed = await store.deleteByFilter(testCollection, { eq: { field: "doc.title", value: "drop-me" } });
		expect(removed).toBe(1);
		const stats = await store.collectionStats(testCollection);
		expect(stats.documents).toBe(1);
	});

	it("drops a collection including its vec0 table", async () => {
		await store.ensureCollection({ name: testCollection, embedding_dim: vectorDim });
		expect(await store.dropCollection(testCollection)).toBeUndefined();
		expect(await store.getCollection(testCollection)).toBeNull();
	});

	it("retrieve() in full_text mode finds a chunk by exact text match", async () => {
		await store.ensureCollection({ name: testCollection, embedding_dim: vectorDim });
		await store.upsertDocument(testCollection, { full_text: "doc" }, [
			{ ordinal: 0, content: "the quick brown fox", embedding: [1, 0, 0, 0] },
		]);
		const out = await store.retrieve(testCollection, {
			top_k: 5,
			mode: "full_text",
			query_text: "brown fox",
			include_content: true,
		});
		expect(out.chunks[0]?.content).toBe("the quick brown fox");
		expect(out.mode).toBe("full_text");
	});

	it("retrieve() in hybrid mode ranks a chunk good on both signals above either extreme", async () => {
		await store.ensureCollection({ name: testCollection, embedding_dim: vectorDim });
		const documentId = await store.upsertDocument(testCollection, { full_text: "doc" }, [
			{ ordinal: 0, content: "zzz unrelated content", embedding: [1, 0, 0, 0] },
			{ ordinal: 1, content: "hybrid search", embedding: [0, 0, 0, 1] },
			{ ordinal: 2, content: "hybrid search related content", embedding: [0.7, 0, 0, 0.7] },
			{ ordinal: 3, content: "filler padding words one", embedding: [0.2, 0, 0, 0.6] },
			{ ordinal: 4, content: "filler padding words two", embedding: [0.1, 0, 0, 0.8] },
		]);
		const out = await store.retrieve(testCollection, {
			top_k: 3,
			mode: "hybrid",
			query_vector: [1, 0, 0, 0],
			query_text: "hybrid search",
		});
		expect(out.chunks[0]?.id).toBe(`${documentId}:2`);
		expect(out.chunks[0]?.primary_score.kind).toBe("hybrid");
	});

	it("retrieve() throws for full_text mode without query_text", async () => {
		await store.ensureCollection({ name: testCollection, embedding_dim: vectorDim });
		await expect(store.retrieve(testCollection, { top_k: 5, mode: "full_text" })).rejects.toThrow(/query_text/);
	});

	it("retrieve() throws for hybrid mode missing either query input", async () => {
		await store.ensureCollection({ name: testCollection, embedding_dim: vectorDim });
		await expect(
			store.retrieve(testCollection, { top_k: 5, mode: "hybrid", query_text: "x" }),
		).rejects.toThrow(/query_vector/);
	});

	it("retrieve() applies a Filter to constrain results", async () => {
		await store.ensureCollection({ name: testCollection, embedding_dim: vectorDim });
		await store.upsertDocument(testCollection, { full_text: "doc", metadata: { tier: "gold" } }, [
			{ ordinal: 0, content: "gold chunk", embedding: [1, 0, 0, 0] },
		]);
		await store.upsertDocument(testCollection, { full_text: "doc", metadata: { tier: "silver" } }, [
			{ ordinal: 0, content: "silver chunk", embedding: [1, 0, 0, 0] },
		]);
		const out = await store.retrieve(testCollection, {
			top_k: 10,
			mode: "vector",
			query_vector: [1, 0, 0, 0],
			include_content: true,
			filter: { eq: { field: "doc.metadata.tier", value: "gold" } },
		});
		expect(out.chunks).toHaveLength(1);
		expect(out.chunks[0]?.content).toBe("gold chunk");
	});

	it("creates a graph edge and traverses it via recursive CTE", async () => {
		await store.createEdge({ id: "e1", source: "a", target: "b", label: "relates_to" });
		await store.createEdge({ id: "e2", source: "b", target: "c", label: "relates_to" });
		await store.createEdge({ id: "e3", source: "a", target: "z", label: "unrelated" });
		const reached = await store.traverseGraph(["a"], 2, ["relates_to"]);
		expect(reached).toContain("a");
		expect(reached).toContain("b");
		expect(reached).toContain("c");
		expect(reached).not.toContain("z");
	});

	it("traverseGraph respects depth limit", async () => {
		await store.createEdge({ id: "e1", source: "a", target: "b" });
		await store.createEdge({ id: "e2", source: "b", target: "c" });
		const reached = await store.traverseGraph(["a"], 1);
		expect(reached).toContain("b");
		expect(reached).not.toContain("c");
	});
});

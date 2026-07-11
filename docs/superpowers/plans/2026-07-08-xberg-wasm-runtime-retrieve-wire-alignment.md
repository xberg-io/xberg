# Xberg WASM Runtime: `retrieve()` Wire-Alignment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `packages/xberg-wasm-runtime`'s `retrieve()` request/response shapes match `xberg-rag`'s `RetrieveQuery`/`RetrieveOutput`/`RetrievedChunk`/`PrimaryScore` types exactly, so `crates/xberg-wasm`'s `XbergEngine.query()` (which already calls a JS `retrieve(collection, query)` method via this exact contract) works against this store without any further Rust-side change.

**Architecture:** `RetrieveOptions` and the `retrieve()` return type change from this session's earlier ad-hoc camelCase/bare-array shape to the real wire contract (snake_case fields, `"full_text"` mode literal, `{mode, chunks, primary_latency_ms}` response). `document_id` enrichment (not derivable from `chunkId` alone — `chunkId` encodes `source_id`, not `document_id`) is added via one small batched lookup query per `retrieve()` call, keeping the existing `query()`/`fullTextQuery()` functions and their SQL completely untouched.

**Tech Stack:** TypeScript ESM (`packages/xberg-wasm-runtime`), `better-sqlite3` (Node), the vendored `sqlite-vec` WASM build (browser Worker/OPFS), `vitest`, `@playwright/test`.

**Spec:** [2026-07-08-xberg-wasm-runtime-retrieve-wire-alignment-design.md](../specs/2026-07-08-xberg-wasm-runtime-retrieve-wire-alignment-design.md)

**Prerequisite already complete:** the spec's "Open Verification Question" (`PrimaryScore`'s exact JSON wire shape) has been resolved and fixed — `crates/xberg-rag/src/types.rs`'s `PrimaryScore` enum used `#[serde(tag = "kind")]` (pure internal tagging), which cannot represent a newtype variant wrapping a bare primitive; `serde_json::to_string(&PrimaryScore::Vector(0.5))` panicked at runtime with `"cannot serialize tagged newtype variant PrimaryScore::Vector containing a float"`. Fixed to `#[serde(tag = "kind", content = "value")]` (adjacent tagging), verified via a real `cargo test` run (committed as `45d4bad7ca` on `feature/wasm-runtime-sqlite-store`). Confirmed wire shapes, used throughout this plan:
```json
{"kind":"vector","value":0.5}
{"kind":"full_text","value":0.75}
{"kind":"hybrid","value":{"vector":0.5,"full_text":0.75,"rrf":0.032}}
```

## Global Constraints

- **TypeScript:** ESM only, `strict: true`, `noUncheckedIndexedAccess: true` — existing `tsconfig.json`, do not weaken.
- **Linting/formatting:** `oxlint src/` + `oxfmt src/` must be clean before every commit.
- **Testing:** real `better-sqlite3` in Node tests, real Chromium via `@playwright/test` for the browser path — no mocking.
- **No AI attribution in commits** (critical repo rule).
- **Conventional commits:** `feat:`/`fix:`/`test:`/`refactor:`, imperative mood, first line <72 chars.
- **No changes to `crates/xberg-wasm`, `crates/xberg-rag` (beyond the already-committed `PrimaryScore` fix), or `mcp-server/`** — this plan is scoped to `packages/xberg-wasm-runtime` only.
- **`filter` present and non-null → throw**, not silently ignored (correctness risk, not a missing nice-to-have). `candidate_multiplier`/`group_by_document`/`include_document`/`include_content` → accepted, logged once via `console.debug`, ignored (result-shape-only impact, not scope/correctness). `mode: "graph"` → throw, not a silent fallthrough into other mode logic.
- **The existing `query(collection, queryVector, k)` method is unchanged** — `retrieve()` becomes correct in addition to it, not a replacement.

---

### Task 1: `types.ts` wire-alignment

**Files:**
- Modify: `packages/xberg-wasm-runtime/src/types.ts`

**Interfaces:**
- Consumes: nothing new.
- Produces: `RetrieveMode = "vector" | "full_text" | "hybrid" | "graph"`; `RetrieveOptions` with snake_case fields; `PrimaryScore`, `RetrievedChunk`, `RetrieveOutput` types; `VectorStoreInterface.retrieve`'s return type changes to `Promise<RetrieveOutput>`. Consumed by Tasks 2-3.

- [ ] **Step 1: Replace `RetrieveMode`/`RetrieveOptions` and add the response types**

In `packages/xberg-wasm-runtime/src/types.ts`, replace:

```typescript
export type RetrieveMode = "vector" | "fulltext" | "hybrid";

export interface RetrieveOptions {
	mode: RetrieveMode;
	queryText?: string;
	queryVector?: number[];
	k: number;
}
```

with:

```typescript
export type RetrieveMode = "vector" | "full_text" | "hybrid" | "graph";

export interface RetrieveOptions {
	mode: RetrieveMode;
	query_text?: string;
	query_vector?: number[];
	top_k: number;
	// Accepted and typed (xberg-rag's engine sends these on every call), not
	// yet honored — see the design spec's Non-Goals. filter throws if
	// present; the rest are logged once and ignored.
	filter?: unknown;
	candidate_multiplier?: number;
	group_by_document?: boolean;
	include_content?: boolean;
	include_document?: boolean;
}

export type PrimaryScore =
	| { kind: "vector"; value: number }
	| { kind: "full_text"; value: number }
	| { kind: "hybrid"; value: { vector: number; full_text: number; rrf: number } };

export interface RetrievedChunk {
	id: string;
	document_id: string;
	ordinal: number;
	external_id?: string;
	content?: string;
	score: number;
	primary_score: PrimaryScore;
	chunk_metadata: unknown;
}

export interface RetrieveOutput {
	mode: RetrieveMode;
	chunks: RetrievedChunk[];
	primary_latency_ms: number;
}
```

- [ ] **Step 2: Update `VectorStoreInterface.retrieve`'s return type**

In the same file, change:

```typescript
	retrieve(
		collection: string,
		opts: RetrieveOptions,
	): Promise<Array<{ chunkId: string; text: string; score: number }>>;
```

to:

```typescript
	retrieve(collection: string, opts: RetrieveOptions): Promise<RetrieveOutput>;
```

- [ ] **Step 3: Run the type check to confirm the expected, scoped breakage**

```bash
cd packages/xberg-wasm-runtime
npx tsc --noEmit
```

Expected: errors ONLY in `store-node.ts`, `store-worker.ts`, `store-node.test.ts`, and `tests/browser/store.spec.ts` (implementations and tests still use the old field names/response shape — fixed in Tasks 2-3). If errors appear anywhere else, STOP and investigate — something else unexpectedly depends on the old shape.

- [ ] **Step 4: Commit**

```bash
npx oxfmt --check src/
npx oxlint src/
git add packages/xberg-wasm-runtime/src/types.ts
git commit -m "feat(wasm-runtime): align RetrieveOptions/RetrieveOutput with xberg-rag's wire contract"
```

---

### Task 2: `store-node.ts` — Node path retrieve() wire-alignment

**Files:**
- Modify: `packages/xberg-wasm-runtime/src/store-node.ts`
- Modify: `packages/xberg-wasm-runtime/src/store-node.test.ts`

**Interfaces:**
- Consumes: `RetrieveOptions`, `RetrieveOutput`, `RetrievedChunk`, `PrimaryScore` from `./types.js` (Task 1).
- Produces: `createNodeVectorStore(...)`'s `retrieve` method now matches `VectorStoreInterface.retrieve`'s new signature. `query`/`fullTextQuery`/the existing `query` public method are completely unchanged (same SQL, same signatures, same tests).

- [ ] **Step 1: Update the failing tests to the new request/response shape**

In `packages/xberg-wasm-runtime/src/store-node.test.ts`, replace the five `retrieve()`-related tests (currently at lines 145-256) with:

```typescript
	it("retrieve() in fulltext mode finds a chunk by exact text match", async () => {
		await store.ensureCollection(testCollection, vectorDim);
		const doc: DocumentRecord = { documentId: "doc-1", sourceId: "src-1", collectionId: testCollection };
		await store.upsertDocument(testCollection, doc, [
			{
				sourceId: "src-1",
				chunkIndex: 0,
				text: "the quick brown fox",
				startOffset: 0,
				endOffset: 19,
				embedding: new Float32Array([1, 0, 0, 0]),
			},
		]);
		const output = await store.retrieve(testCollection, { mode: "full_text", query_text: "brown fox", top_k: 5 });
		expect(output.mode).toBe("full_text");
		expect(output.chunks[0]?.content).toBe("the quick brown fox");
		expect(output.chunks[0]?.document_id).toBe("doc-1");
		expect(output.chunks[0]?.primary_score).toEqual({ kind: "full_text", value: output.chunks[0]?.score });
		expect(output.primary_latency_ms).toBeGreaterThanOrEqual(0);
	});

	it("retrieve() in vector mode matches query() behavior and enriches document_id/ordinal", async () => {
		await store.ensureCollection(testCollection, vectorDim);
		const doc: DocumentRecord = { documentId: "doc-1", sourceId: "src-1", collectionId: testCollection };
		await store.upsertDocument(testCollection, doc, [
			{
				sourceId: "src-1",
				chunkIndex: 0,
				text: "apple",
				startOffset: 0,
				endOffset: 5,
				embedding: new Float32Array([1, 0, 0, 0]),
			},
		]);
		const output = await store.retrieve(testCollection, { mode: "vector", query_vector: [1, 0, 0, 0], top_k: 5 });
		expect(output.mode).toBe("vector");
		expect(output.chunks[0]?.content).toBe("apple");
		expect(output.chunks[0]?.id).toBe("src-1:0");
		expect(output.chunks[0]?.document_id).toBe("doc-1");
		expect(output.chunks[0]?.ordinal).toBe(0);
		expect(output.chunks[0]?.primary_score.kind).toBe("vector");
	});

	it("retrieve() in hybrid mode ranks a chunk good on both signals above either extreme, with a real hybrid primary_score", async () => {
		await store.ensureCollection(testCollection, vectorDim);
		const doc: DocumentRecord = { documentId: "doc-1", sourceId: "src-1", collectionId: testCollection };
		// Fixture verified against the real better-sqlite3 + sqlite-vec + FTS5 engine (not assumed):
		// - FTS5 MATCH ANDs bareword terms by default, so the query text is kept to terms every
		//   textually-relevant chunk actually contains ("hybrid search"), otherwise a partial text
		//   match is excluded from the fulltext ranking entirely rather than ranked lower.
		// - Two vector-only filler chunks push chunk 1's vector rank down to 5th so RRF's convex
		//   1/(k+rank) sum genuinely favors chunk 2 (moderate rank 2 + rank 2) over chunk 1
		//   (best-possible text rank 1 offset by a much worse vector rank) instead of tying/losing.
		await store.upsertDocument(testCollection, doc, [
			{
				sourceId: "src-1",
				chunkIndex: 0,
				text: "zzz unrelated content",
				startOffset: 0,
				endOffset: 22,
				embedding: new Float32Array([1, 0, 0, 0]),
			},
			{
				sourceId: "src-1",
				chunkIndex: 1,
				text: "hybrid search",
				startOffset: 23,
				endOffset: 37,
				embedding: new Float32Array([0, 0, 0, 1]),
			},
			{
				sourceId: "src-1",
				chunkIndex: 2,
				text: "hybrid search related content",
				startOffset: 38,
				endOffset: 68,
				embedding: new Float32Array([0.7, 0, 0, 0.7]),
			},
			{
				sourceId: "src-1",
				chunkIndex: 3,
				text: "filler padding words one",
				startOffset: 69,
				endOffset: 94,
				embedding: new Float32Array([0.2, 0, 0, 0.6]),
			},
			{
				sourceId: "src-1",
				chunkIndex: 4,
				text: "filler padding words two",
				startOffset: 95,
				endOffset: 120,
				embedding: new Float32Array([0.1, 0, 0, 0.8]),
			},
		]);
		const output = await store.retrieve(testCollection, {
			mode: "hybrid",
			query_vector: [1, 0, 0, 0],
			query_text: "hybrid search",
			top_k: 3,
		});
		expect(output.mode).toBe("hybrid");
		expect(output.chunks[0]?.id).toBe("src-1:2");
		const primaryScore = output.chunks[0]?.primary_score;
		expect(primaryScore?.kind).toBe("hybrid");
		if (primaryScore?.kind === "hybrid") {
			expect(primaryScore.value.vector).toBeGreaterThan(0);
			expect(primaryScore.value.full_text).toBeGreaterThan(0);
			expect(primaryScore.value.rrf).toBe(output.chunks[0]?.score);
		}
	});

	it("retrieve() throws for full_text mode without query_text", async () => {
		await store.ensureCollection(testCollection, vectorDim);
		await expect(store.retrieve(testCollection, { mode: "full_text", top_k: 5 })).rejects.toThrow(/query_text/);
	});

	it("retrieve() throws for hybrid mode missing either query input", async () => {
		await store.ensureCollection(testCollection, vectorDim);
		await expect(
			store.retrieve(testCollection, { mode: "hybrid", query_text: "x", top_k: 5 }),
		).rejects.toThrow(/query_vector/);
	});

	it("retrieve() throws for mode 'graph'", async () => {
		await store.ensureCollection(testCollection, vectorDim);
		await expect(store.retrieve(testCollection, { mode: "graph", top_k: 5 })).rejects.toThrow(/graph/);
	});

	it("retrieve() throws when filter is provided", async () => {
		await store.ensureCollection(testCollection, vectorDim);
		await expect(
			store.retrieve(testCollection, { mode: "vector", query_vector: [1, 0, 0, 0], top_k: 5, filter: { field: "doc.title", value: "x" } }),
		).rejects.toThrow(/filter/);
	});
```

- [ ] **Step 2: Run to verify it fails**

```bash
cd packages/xberg-wasm-runtime
npx vitest run src/store-node.test.ts
```

Expected: FAIL — `retrieve()` still returns the old bare-array shape with camelCase fields; `output.mode`/`output.chunks` are `undefined`.

- [ ] **Step 3: Rewrite `retrieve()` in `store-node.ts`**

In `packages/xberg-wasm-runtime/src/store-node.ts`, add `RetrievedChunk`, `RetrieveOutput`, `PrimaryScore` to the existing type-only import:

```typescript
import type {
	VectorStoreInterface,
	DocumentRecord,
	ChunkRecord,
	GraphEdge,
	CacheConfig,
	RetrieveOptions,
	RetrieveOutput,
	RetrievedChunk,
	PrimaryScore,
} from "./types.js";
```

Add two helper functions after the existing `fullTextQuery` function (do not modify `query`/`fullTextQuery` themselves):

```typescript
	function parseChunkId(chunkId: string): { sourceId: string; ordinal: number } {
		const separatorIndex = chunkId.lastIndexOf(":");
		return {
			sourceId: chunkId.slice(0, separatorIndex),
			ordinal: Number(chunkId.slice(separatorIndex + 1)),
		};
	}

	// document_id is NOT derivable from chunkId — chunkId encodes source_id
	// (e.g. "src-1:0"), a different identifier from document_id (e.g.
	// "doc-1") set independently at upsertDocument time. One batched lookup
	// per retrieve() call, not per-chunk, to avoid N+1 queries.
	function documentIdsBySourceId(collection: string, sourceIds: string[]): Map<string, string> {
		const uniqueIds = [...new Set(sourceIds)];
		if (uniqueIds.length === 0) return new Map();
		const placeholders = uniqueIds.map(() => "?").join(",");
		const rows = db
			.prepare(
				`SELECT source_id, document_id FROM documents WHERE collection = ? AND source_id IN (${placeholders})`,
			)
			.all(collection, ...uniqueIds) as Array<{ source_id: string; document_id: string }>;
		return new Map(rows.map((r) => [r.source_id, r.document_id]));
	}

	function toRetrievedChunk(
		result: { chunkId: string; text: string },
		primaryScore: PrimaryScore,
		documentIds: Map<string, string>,
	): RetrievedChunk {
		const { sourceId, ordinal } = parseChunkId(result.chunkId);
		return {
			id: result.chunkId,
			document_id: documentIds.get(sourceId) ?? sourceId,
			ordinal,
			content: result.text,
			score: primaryScore.kind === "hybrid" ? primaryScore.value.rrf : primaryScore.value,
			primary_score: primaryScore,
			chunk_metadata: null,
		};
	}
```

Replace the existing `retrieve` function entirely:

```typescript
	async function retrieve(collection: string, opts: RetrieveOptions): Promise<RetrieveOutput> {
		const start = performance.now();

		if (opts.mode === "graph") {
			throw new Error("retrieve: mode 'graph' is not yet supported");
		}
		if (opts.filter !== undefined && opts.filter !== null) {
			throw new Error("retrieve: filter is not yet supported");
		}
		if (opts.candidate_multiplier !== undefined) {
			console.debug("[retrieve] candidate_multiplier is not yet honored");
		}
		if (opts.group_by_document) {
			console.debug("[retrieve] group_by_document is not yet honored");
		}
		if (opts.include_document) {
			console.debug("[retrieve] include_document is not yet honored");
		}
		if (opts.include_content === false) {
			console.debug("[retrieve] include_content=false is not yet honored (content is always included)");
		}

		if (opts.mode === "vector") {
			if (!opts.query_vector) throw new Error("retrieve: query_vector is required for mode 'vector'");
			const results = await query(collection, opts.query_vector, opts.top_k);
			const documentIds = documentIdsBySourceId(
				collection,
				results.map((r) => parseChunkId(r.chunkId).sourceId),
			);
			return {
				mode: "vector",
				chunks: results.map((r) => toRetrievedChunk(r, { kind: "vector", value: r.score }, documentIds)),
				primary_latency_ms: Math.round(performance.now() - start),
			};
		}

		if (opts.mode === "full_text") {
			if (!opts.query_text) throw new Error("retrieve: query_text is required for mode 'full_text'");
			const results = await fullTextQuery(collection, opts.query_text, opts.top_k);
			const documentIds = documentIdsBySourceId(
				collection,
				results.map((r) => parseChunkId(r.chunkId).sourceId),
			);
			return {
				mode: "full_text",
				chunks: results.map((r) => toRetrievedChunk(r, { kind: "full_text", value: r.score }, documentIds)),
				primary_latency_ms: Math.round(performance.now() - start),
			};
		}

		// hybrid
		if (!opts.query_vector) throw new Error("retrieve: query_vector is required for mode 'hybrid'");
		if (!opts.query_text) throw new Error("retrieve: query_text is required for mode 'hybrid'");
		const candidateK = opts.top_k * HYBRID_CANDIDATE_MULTIPLIER;
		const [vectorResults, textResults] = await Promise.all([
			query(collection, opts.query_vector, candidateK),
			fullTextQuery(collection, opts.query_text, candidateK),
		]);
		const vectorScoreByChunk = new Map(vectorResults.map((r) => [r.chunkId, r.score]));
		const textScoreByChunk = new Map(textResults.map((r) => [r.chunkId, r.score]));
		const fused = reciprocalRankFusion([vectorResults, textResults]).slice(0, opts.top_k);
		const documentIds = documentIdsBySourceId(
			collection,
			fused.map((r) => parseChunkId(r.chunkId).sourceId),
		);
		return {
			mode: "hybrid",
			chunks: fused.map((r) =>
				toRetrievedChunk(
					r,
					{
						kind: "hybrid",
						value: {
							vector: vectorScoreByChunk.get(r.chunkId) ?? 0,
							full_text: textScoreByChunk.get(r.chunkId) ?? 0,
							rrf: r.score,
						},
					},
					documentIds,
				),
			),
			primary_latency_ms: Math.round(performance.now() - start),
		};
	}
```

- [ ] **Step 4: Run to verify it passes**

```bash
npx vitest run src/store-node.test.ts
```

Expected: PASS, all tests including the 7 rewritten/new ones. If the hybrid test's `primary_score.value.vector`/`.full_text` assertions fail with 0, check that `vectorScoreByChunk`/`textScoreByChunk` are being looked up by the correct `chunkId` key (`"src-1:2"`, not just `"2"` or similar) before assuming the fixture is wrong — the fixture itself was already verified correct in the prior plan.

- [ ] **Step 5: Run the full package suite and lint**

```bash
npx vitest run
npx tsc --noEmit
npx oxfmt --check src/
npx oxlint src/
```

Expected: all PASS/clean. `tsc --noEmit` should now show errors ONLY in `store-worker.ts`, `store-browser.ts`, and `tests/browser/store.spec.ts` (fixed in Task 3).

- [ ] **Step 6: Commit**

```bash
git add packages/xberg-wasm-runtime/src/store-node.ts packages/xberg-wasm-runtime/src/store-node.test.ts
git commit -m "feat(wasm-runtime): wire-align retrieve() in the Node store with xberg-rag's contract"
```

---

### Task 3: `store-worker.ts` + `store-browser.ts` — browser path retrieve() wire-alignment

**Files:**
- Modify: `packages/xberg-wasm-runtime/src/store-worker.ts`
- Modify: `packages/xberg-wasm-runtime/src/store-browser.ts`
- Modify: `packages/xberg-wasm-runtime/tests/browser/store.spec.ts`

**Interfaces:**
- Consumes: `RetrieveOptions`, `RetrieveOutput`, `RetrievedChunk`, `PrimaryScore` from `./types.js` (Task 1).
- Produces: `store-worker.ts`'s `retrieve` dispatch function and `store-browser.ts`'s RPC forwarder both match the new contract. `query`/`fullTextQuery` in `store-worker.ts` are unchanged.

- [ ] **Step 1: Rewrite `retrieve()` in `store-worker.ts`**

In `packages/xberg-wasm-runtime/src/store-worker.ts`, add the new types to the existing type-only import:

```typescript
import type { ChunkRecord, DocumentRecord, GraphEdge, RetrieveOptions, RetrieveOutput, RetrievedChunk, PrimaryScore } from "./types.js";
```

Add the same three helpers as Task 2 (adapted to this file's sync, `SqliteDb`-based style) after the existing `fullTextQuery` function:

```typescript
function parseChunkId(chunkId: string): { sourceId: string; ordinal: number } {
	const separatorIndex = chunkId.lastIndexOf(":");
	return { sourceId: chunkId.slice(0, separatorIndex), ordinal: Number(chunkId.slice(separatorIndex + 1)) };
}

function documentIdsBySourceId(collection: string, sourceIds: string[]): Map<string, string> {
	const uniqueIds = [...new Set(sourceIds)];
	if (uniqueIds.length === 0) return new Map();
	const placeholders = uniqueIds.map(() => "?").join(",");
	const result = rows<{ source_id: string; document_id: string }>(
		requireDatabase(),
		`SELECT source_id, document_id FROM documents WHERE collection = ? AND source_id IN (${placeholders})`,
		[collection, ...uniqueIds],
	);
	return new Map(result.map((r) => [r.source_id, r.document_id]));
}

function toRetrievedChunk(
	result: { chunkId: string; text: string },
	primaryScore: PrimaryScore,
	documentIds: Map<string, string>,
): RetrievedChunk {
	const { sourceId, ordinal } = parseChunkId(result.chunkId);
	return {
		id: result.chunkId,
		document_id: documentIds.get(sourceId) ?? sourceId,
		ordinal,
		content: result.text,
		score: primaryScore.kind === "hybrid" ? primaryScore.value.rrf : primaryScore.value,
		primary_score: primaryScore,
		chunk_metadata: null,
	};
}
```

Replace the existing `retrieve` function entirely:

```typescript
function retrieve(collection: string, opts: RetrieveOptions): RetrieveOutput {
	const start = performance.now();

	if (opts.mode === "graph") {
		throw new Error("retrieve: mode 'graph' is not yet supported");
	}
	if (opts.filter !== undefined && opts.filter !== null) {
		throw new Error("retrieve: filter is not yet supported");
	}
	if (opts.candidate_multiplier !== undefined) {
		console.debug("[retrieve] candidate_multiplier is not yet honored");
	}
	if (opts.group_by_document) {
		console.debug("[retrieve] group_by_document is not yet honored");
	}
	if (opts.include_document) {
		console.debug("[retrieve] include_document is not yet honored");
	}
	if (opts.include_content === false) {
		console.debug("[retrieve] include_content=false is not yet honored (content is always included)");
	}

	if (opts.mode === "vector") {
		if (!opts.query_vector) throw new Error("retrieve: query_vector is required for mode 'vector'");
		const results = query(collection, opts.query_vector, opts.top_k);
		const documentIds = documentIdsBySourceId(
			collection,
			results.map((r) => parseChunkId(r.chunkId).sourceId),
		);
		return {
			mode: "vector",
			chunks: results.map((r) => toRetrievedChunk(r, { kind: "vector", value: r.score }, documentIds)),
			primary_latency_ms: Math.round(performance.now() - start),
		};
	}

	if (opts.mode === "full_text") {
		if (!opts.query_text) throw new Error("retrieve: query_text is required for mode 'full_text'");
		const results = fullTextQuery(collection, opts.query_text, opts.top_k);
		const documentIds = documentIdsBySourceId(
			collection,
			results.map((r) => parseChunkId(r.chunkId).sourceId),
		);
		return {
			mode: "full_text",
			chunks: results.map((r) => toRetrievedChunk(r, { kind: "full_text", value: r.score }, documentIds)),
			primary_latency_ms: Math.round(performance.now() - start),
		};
	}

	// hybrid
	if (!opts.query_vector) throw new Error("retrieve: query_vector is required for mode 'hybrid'");
	if (!opts.query_text) throw new Error("retrieve: query_text is required for mode 'hybrid'");
	const candidateK = opts.top_k * HYBRID_CANDIDATE_MULTIPLIER;
	const vectorResults = query(collection, opts.query_vector, candidateK);
	const textResults = fullTextQuery(collection, opts.query_text, candidateK);
	const vectorScoreByChunk = new Map(vectorResults.map((r) => [r.chunkId, r.score]));
	const textScoreByChunk = new Map(textResults.map((r) => [r.chunkId, r.score]));
	const fused = reciprocalRankFusion([vectorResults, textResults]).slice(0, opts.top_k);
	const documentIds = documentIdsBySourceId(
		collection,
		fused.map((r) => parseChunkId(r.chunkId).sourceId),
	);
	return {
		mode: "hybrid",
		chunks: fused.map((r) =>
			toRetrievedChunk(
				r,
				{
					kind: "hybrid",
					value: {
						vector: vectorScoreByChunk.get(r.chunkId) ?? 0,
						full_text: textScoreByChunk.get(r.chunkId) ?? 0,
						rrf: r.score,
					},
				},
				documentIds,
			),
		),
		primary_latency_ms: Math.round(performance.now() - start),
	};
}
```

- [ ] **Step 2: Update `store-browser.ts`'s type parameter**

In `packages/xberg-wasm-runtime/src/store-browser.ts`, add `RetrieveOutput` to the existing type-only import (remove `RetrieveOptions` is still needed too — add alongside it):

```typescript
import type {
	VectorStoreInterface,
	DocumentRecord,
	ChunkRecord,
	GraphEdge,
	CacheConfig,
	RetrieveOptions,
	RetrieveOutput,
} from "./types.js";
```

Change the `retrieve` entry in the returned object:

```typescript
		retrieve: (collection: string, opts: RetrieveOptions) => call<RetrieveOutput>({ op: "retrieve", collection, opts }),
```

- [ ] **Step 3: Update the Playwright tests**

In `packages/xberg-wasm-runtime/tests/browser/store.spec.ts`, find the two tests added for the FTS5 gate and hybrid mode (added in the prior plan's Task 4). Update their assertions from the old bare-array shape to the new `RetrieveOutput` shape:

Replace:

```typescript
		await store.upsertDocument(
			"fts5-check",
			{ documentId: "d1", sourceId: "s1", collectionId: "fts5-check" },
			[{ sourceId: "s1", chunkIndex: 0, text: "fts5 availability probe", startOffset: 0, endOffset: 24, embedding: new Float32Array([1, 0, 0, 0]) }],
		);
		const results = await store.retrieve("fts5-check", { mode: "fulltext", queryText: "availability probe", k: 1 });
		return results.length > 0 && results[0].text === "fts5 availability probe";
```

with:

```typescript
		await store.upsertDocument(
			"fts5-check",
			{ documentId: "d1", sourceId: "s1", collectionId: "fts5-check" },
			[{ sourceId: "s1", chunkIndex: 0, text: "fts5 availability probe", startOffset: 0, endOffset: 24, embedding: new Float32Array([1, 0, 0, 0]) }],
		);
		const output = await store.retrieve("fts5-check", { mode: "full_text", query_text: "availability probe", top_k: 1 });
		return output.chunks.length > 0 && output.chunks[0].content === "fts5 availability probe";
```

Replace:

```typescript
		const results = await store.retrieve("hybrid-check", {
			mode: "hybrid",
			queryVector: [1, 0, 0, 0],
			queryText: "hybrid phrase match",
			k: 3,
		});
		return results[0]?.chunkId;
```

with:

```typescript
		const output = await store.retrieve("hybrid-check", {
			mode: "hybrid",
			query_vector: [1, 0, 0, 0],
			query_text: "hybrid phrase match",
			top_k: 3,
		});
		return output.chunks[0]?.id;
```

- [ ] **Step 4: Run the full verification suite**

```bash
cd packages/xberg-wasm-runtime
npx tsc --noEmit
npx vitest run
pnpm test:browser
npx oxfmt --check src/
npx oxlint src/
```

Expected: `tsc --noEmit` clean (zero errors anywhere — this is the point where both backends fully match the new contract). `vitest run` and `pnpm test:browser` both pass. If `pnpm test:browser`'s hybrid test fails on the ranking assertion, this is the SAME fixture already verified correct in the prior plan (unrelated to this wire-alignment change) — check for a typo in the request field renames (`query_vector`/`query_text`/`top_k`) before suspecting the fixture.

- [ ] **Step 5: Commit**

```bash
git add packages/xberg-wasm-runtime/src/store-worker.ts packages/xberg-wasm-runtime/src/store-browser.ts packages/xberg-wasm-runtime/tests/browser/store.spec.ts
git commit -m "feat(wasm-runtime): wire-align retrieve() in the browser Worker/RPC path with xberg-rag's contract"
```

---

## Self-Review Notes

- **Spec coverage:** Request shape (Task 1), response shape (Task 1), `document_id`/`ordinal` field mapping (Tasks 2-3's `parseChunkId`/`documentIdsBySourceId`), `primary_score` per-mode construction (Tasks 2-3), error handling for `graph`/`filter`/cosmetic fields (Tasks 2-3), `query()` left unchanged (verified — no task touches it), no `crates/xberg-wasm`/`crates/xberg-rag`/`mcp-server/` changes beyond the already-committed `PrimaryScore` fix (verified — Tasks 1-3 touch only `packages/xberg-wasm-runtime`).
- **Type consistency check:** `RetrieveOptions`/`RetrieveOutput`/`RetrievedChunk`/`PrimaryScore` (Task 1) used identically across `store-node.ts` (Task 2), `store-worker.ts` (Task 3), and `store-browser.ts` (Task 3) — same field names, same `PrimaryScore` discriminated-union shape throughout. `parseChunkId`/`documentIdsBySourceId`/`toRetrievedChunk` are duplicated (not shared) between `store-node.ts` and `store-worker.ts` — deliberate: these two files already don't share implementation code (one is `better-sqlite3`-sync-via-async-wrapper, the other is raw `SqliteDb`-sync), matching this package's existing pattern of near-duplicate-but-not-identical logic between the two backends rather than a forced shared abstraction across genuinely different underlying APIs.
- **Placeholder scan:** no TBD/TODO; the one open item from the design spec (`PrimaryScore`'s wire shape) was resolved and fixed before this plan was written, not deferred into it.

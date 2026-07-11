# Xberg WASM Runtime: Hybrid Search, BGE-M3 Embeddings, and Candle PII NER Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add hybrid/full-text retrieval, upgrade the default embedding model to BGE-M3, and wire the already-built Candle GLiNER2 PII backend into `xberg-wasm`'s stubbed NER fallback (plus a JS-side regex complement) — three verified upgrades to `packages/xberg-wasm-runtime` and `crates/xberg-wasm`.

**Architecture:** Part 1 (Tasks 1-5) adds an FTS5 virtual table and a `retrieve()` method to `VectorStoreInterface`, implemented identically on the Node (`better-sqlite3`) and browser (Worker/OPFS) backends, fusing vector + full-text results via Reciprocal Rank Fusion for hybrid mode. Part 2 (Tasks 6-7) swaps the embedder's default model. Part 3 (Tasks 8-12) fixes a real contract bug between the wasm engine and the injected JS NER, ports a proven regex PII detector from `mcp-server` into this package, and wires `crates/xberg-gliner-candle`'s existing Candle GLiNER2 implementation into `crates/xberg-wasm`'s currently-stubbed NER fallback using `fastino/gliner2-privacy-filter-PII-multi` as the pinned model.

**Tech Stack:** TypeScript ESM (`packages/xberg-wasm-runtime`), `better-sqlite3` + native SQLite FTS5 (Node), `@sqlite.org/sqlite-wasm`-family WASM SQLite build (browser, already vendored at `wasm/sqlite-vec/`), `vitest` + `@playwright/test`; Rust (`crates/xberg-wasm`, `crates/xberg-gliner-candle`, `crates/xberg`), `candle-core`/`candle-transformers` 0.11, `wasm-bindgen`.

**Spec:** [2026-07-08-xberg-wasm-runtime-search-embedding-pii-design.md](../specs/2026-07-08-xberg-wasm-runtime-search-embedding-pii-design.md)

## Global Constraints

- **TypeScript:** ESM only, `strict: true`, `noUncheckedIndexedAccess: true` (existing `packages/xberg-wasm-runtime/tsconfig.json`) — do not weaken.
- **Linting/formatting:** `oxlint src/` + `oxfmt src/ --fix`; run before every commit.
- **Testing:** `vitest` (`pnpm test:run`), real `better-sqlite3` in Node tests — no mocking the Node-side SQLite layer. Browser/Worker/OPFS paths are tested via real Chromium through `@playwright/test` (`pnpm test:browser`), not mocked.
- **Package manager:** `pnpm`; commit `pnpm-lock.yaml` after any dependency change (none expected in this plan — all three components reuse existing dependencies).
- **No AI attribution in commits** (repo rule `no-ai-signatures`, critical priority).
- **Conventional commits:** `feat:`, `fix:`, `test:`, `perf:`, `docs:`, imperative mood, first line <72 chars.
- **Rust:** `cargo fmt` + `clippy -D warnings` on every touched crate; `// SAFETY:` comments already exist where needed in touched files — do not add `unsafe` blocks without one.
- **Collection-name SQL-identifier sanitization:** the existing `sanitizeTableName()`/`vecTableName()` helpers in `store-schema.ts` are the only sanctioned way to build per-collection identifiers — never interpolate a raw collection name into DDL/DML (existing convention, applies to any new SQL this plan adds).
- **No silent fallback on search-mode failure:** if `mode: "fulltext"|"hybrid"` fails (e.g. FTS5 unavailable), `retrieve()` must throw, not silently return vector-only results.

---

## Part 1: Hybrid / Full-Text Search

### Task 1: FTS5 schema extension

**Files:**
- Modify: `packages/xberg-wasm-runtime/src/store-schema.ts`
- Test: `packages/xberg-wasm-runtime/src/store-schema.test.ts`

**Interfaces:**
- Consumes: nothing new.
- Produces: `SCHEMA_SQL` (modified export, same name/type — now includes the `chunks_fts` virtual table + sync triggers) consumed by Task 3 (`store-node.ts`) and Task 4 (`store-worker.ts`).

- [ ] **Step 1: Write the failing test**

Add to `packages/xberg-wasm-runtime/src/store-schema.test.ts`:

```typescript
it("SCHEMA_SQL defines an FTS5 external-content table synced to chunks", () => {
	expect(SCHEMA_SQL).toContain("CREATE VIRTUAL TABLE IF NOT EXISTS chunks_fts USING fts5");
	expect(SCHEMA_SQL).toContain("content='chunks'");
	expect(SCHEMA_SQL).toContain("CREATE TRIGGER IF NOT EXISTS chunks_ai AFTER INSERT ON chunks");
	expect(SCHEMA_SQL).toContain("CREATE TRIGGER IF NOT EXISTS chunks_ad AFTER DELETE ON chunks");
	expect(SCHEMA_SQL).toContain("CREATE TRIGGER IF NOT EXISTS chunks_au AFTER UPDATE ON chunks");
});
```

(Add the `import { SCHEMA_SQL, ... }` names already imported at the top of this test file if not already present — check the existing import line first.)

- [ ] **Step 2: Run test to verify it fails**

```bash
cd packages/xberg-wasm-runtime
npx vitest run src/store-schema.test.ts
```

Expected: FAIL — `SCHEMA_SQL` does not contain `chunks_fts`.

- [ ] **Step 3: Append the FTS5 table + triggers to `SCHEMA_SQL`**

In `packages/xberg-wasm-runtime/src/store-schema.ts`, append to the `SCHEMA_SQL` template string (after the existing `graph_edges` index statements, before the closing backtick):

```sql
CREATE VIRTUAL TABLE IF NOT EXISTS chunks_fts USING fts5(
  chunk_id UNINDEXED,
  collection UNINDEXED,
  text,
  content='chunks',
  content_rowid='rowid'
);
CREATE TRIGGER IF NOT EXISTS chunks_ai AFTER INSERT ON chunks BEGIN
  INSERT INTO chunks_fts(rowid, chunk_id, collection, text)
  VALUES (new.rowid, new.chunk_id, new.collection, new.text);
END;
CREATE TRIGGER IF NOT EXISTS chunks_ad AFTER DELETE ON chunks BEGIN
  INSERT INTO chunks_fts(chunks_fts, rowid, chunk_id, collection, text)
  VALUES ('delete', old.rowid, old.chunk_id, old.collection, old.text);
END;
CREATE TRIGGER IF NOT EXISTS chunks_au AFTER UPDATE ON chunks BEGIN
  INSERT INTO chunks_fts(chunks_fts, rowid, chunk_id, collection, text)
  VALUES ('delete', old.rowid, old.chunk_id, old.collection, old.text);
  INSERT INTO chunks_fts(rowid, chunk_id, collection, text)
  VALUES (new.rowid, new.chunk_id, new.collection, new.text);
END;
```

Note: `upsertDocument`'s existing `INSERT OR REPLACE INTO chunks (...)` re-ingestion path fires a DELETE+INSERT sequence (SQLite's actual mechanism for `OR REPLACE` on a primary-key conflict), not an UPDATE statement — so `chunks_ai`/`chunks_ad` are what actually keep the FTS index in sync in practice. `chunks_au` is included because it is SQLite's own documented external-content pattern and correctness requires it for the general case (any future code path that runs a real `UPDATE chunks SET ...` must not desync the index) — do not remove it as "dead code."

- [ ] **Step 4: Run test to verify it passes**

```bash
npx vitest run src/store-schema.test.ts
```

Expected: PASS, including the new test.

- [ ] **Step 5: Commit**

```bash
git add packages/xberg-wasm-runtime/src/store-schema.ts packages/xberg-wasm-runtime/src/store-schema.test.ts
git commit -m "feat(wasm-runtime): add FTS5 external-content table to store schema"
```

---

### Task 2: `RetrieveMode` types and RRF fusion helper

**Files:**
- Modify: `packages/xberg-wasm-runtime/src/types.ts`
- Create: `packages/xberg-wasm-runtime/src/retrieve-fusion.ts`
- Test: `packages/xberg-wasm-runtime/src/retrieve-fusion.test.ts`

**Interfaces:**
- Consumes: nothing new.
- Produces:
  - `types.ts`: `RetrieveMode = "vector" | "fulltext" | "hybrid"`, `RetrieveOptions { mode: RetrieveMode; queryText?: string; queryVector?: number[]; k: number }`, and `VectorStoreInterface` gains `retrieve(collection: string, opts: RetrieveOptions): Promise<Array<{ chunkId: string; text: string; score: number }>>` (existing `query` method is unchanged).
  - `retrieve-fusion.ts`: `reciprocalRankFusion(rankings: Array<Array<{ chunkId: string; text: string }>>, rrfK?: number): Array<{ chunkId: string; text: string; score: number }>` — consumed by Task 3 (`store-node.ts`) and Task 4 (`store-worker.ts`).

- [ ] **Step 1: Write the failing test for `reciprocalRankFusion`**

Create `packages/xberg-wasm-runtime/src/retrieve-fusion.test.ts`:

```typescript
import { describe, it, expect } from "vitest";
import { reciprocalRankFusion } from "./retrieve-fusion";

describe("reciprocalRankFusion", () => {
	it("ranks a chunk that appears in both rankings above one that appears in only one", () => {
		const vectorRanking = [
			{ chunkId: "a", text: "apple fruit" },
			{ chunkId: "b", text: "apple tree" },
		];
		const textRanking = [
			{ chunkId: "b", text: "apple tree" },
			{ chunkId: "c", text: "orange fruit" },
		];
		const fused = reciprocalRankFusion([vectorRanking, textRanking]);
		expect(fused[0]?.chunkId).toBe("b");
	});

	it("sums contributions when a chunk appears at rank 1 in both rankings", () => {
		const ranking = [{ chunkId: "x", text: "solo" }];
		const fused = reciprocalRankFusion([ranking, ranking], 60);
		expect(fused[0]?.score).toBeCloseTo(2 / 61, 10);
	});

	it("returns results sorted by score descending", () => {
		const vectorRanking = [
			{ chunkId: "a", text: "a" },
			{ chunkId: "b", text: "b" },
			{ chunkId: "c", text: "c" },
		];
		const fused = reciprocalRankFusion([vectorRanking, []]);
		for (let i = 1; i < fused.length; i++) {
			expect(fused[i - 1]!.score).toBeGreaterThanOrEqual(fused[i]!.score);
		}
	});

	it("preserves the text of the first ranking a chunk appears in", () => {
		const fused = reciprocalRankFusion([[{ chunkId: "a", text: "original text" }], []]);
		expect(fused[0]?.text).toBe("original text");
	});
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
npx vitest run src/retrieve-fusion.test.ts
```

Expected: FAIL — `Cannot find module './retrieve-fusion'`.

- [ ] **Step 3: Implement `retrieve-fusion.ts`**

Create `packages/xberg-wasm-runtime/src/retrieve-fusion.ts`:

```typescript
const DEFAULT_RRF_K = 60;

/**
 * Reciprocal Rank Fusion: combines multiple ranked result lists into one,
 * summing 1/(rrfK + rank) across every ranking a chunk appears in. Standard
 * IR default (rrfK = 60), not xberg-specific — matches this package's hybrid
 * search design (does not attempt to replicate crates/xberg-rag's exact
 * fusion algorithm; ranking-quality parity is the requirement, not
 * byte-identical scores across the two separate storage engines).
 */
export function reciprocalRankFusion(
	rankings: Array<Array<{ chunkId: string; text: string }>>,
	rrfK: number = DEFAULT_RRF_K,
): Array<{ chunkId: string; text: string; score: number }> {
	const scores = new Map<string, { text: string; score: number }>();
	for (const ranking of rankings) {
		ranking.forEach((item, index) => {
			const rank = index + 1;
			const contribution = 1 / (rrfK + rank);
			const existing = scores.get(item.chunkId);
			if (existing) {
				existing.score += contribution;
			} else {
				scores.set(item.chunkId, { text: item.text, score: contribution });
			}
		});
	}
	return Array.from(scores.entries())
		.map(([chunkId, { text, score }]) => ({ chunkId, text, score }))
		.sort((a, b) => b.score - a.score);
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
npx vitest run src/retrieve-fusion.test.ts
```

Expected: PASS, 4/4 tests.

- [ ] **Step 5: Add `RetrieveMode`/`RetrieveOptions` types and extend `VectorStoreInterface`**

In `packages/xberg-wasm-runtime/src/types.ts`, add after the `GraphEdge` interface (before `VectorStoreInterface`):

```typescript
export type RetrieveMode = "vector" | "fulltext" | "hybrid";

export interface RetrieveOptions {
	mode: RetrieveMode;
	queryText?: string;
	queryVector?: number[];
	k: number;
}
```

Then add one method to the existing `VectorStoreInterface` (keep every existing method unchanged):

```typescript
export interface VectorStoreInterface {
	upsertDocument(
		collection: string,
		doc: DocumentRecord,
		chunks: ChunkRecord[],
	): Promise<{ documentId: string; chunksCount: number }>;
	query(
		collection: string,
		queryVector: number[],
		k: number,
	): Promise<Array<{ chunkId: string; text: string; score: number }>>;
	retrieve(collection: string, opts: RetrieveOptions): Promise<Array<{ chunkId: string; text: string; score: number }>>;
	delete(collection: string, documentId: string): Promise<void>;
	listCollections(): Promise<string[]>;
	dropCollection(collection: string): Promise<void>;
	ensureCollection(collection: string, vectorDim: number): Promise<void>;
	createEdge(edge: GraphEdge): Promise<void>;
	traverseGraph(startIds: string[], depth: number, edgeLabels?: string[]): Promise<string[]>;
}
```

- [ ] **Step 6: Run the type check to confirm the expected, scoped breakage**

```bash
npx tsc --noEmit
```

Expected: errors ONLY in `store-node.ts` and `store-worker.ts`/`store-browser.ts` (they don't implement `retrieve` yet — fixed in Tasks 3-5). If errors appear anywhere else, STOP and investigate — that means something unexpected already depends on the old `VectorStoreInterface` shape.

- [ ] **Step 7: Commit**

```bash
git add packages/xberg-wasm-runtime/src/types.ts packages/xberg-wasm-runtime/src/retrieve-fusion.ts packages/xberg-wasm-runtime/src/retrieve-fusion.test.ts
git commit -m "feat(wasm-runtime): add RetrieveMode types and RRF fusion helper"
```

---

### Task 3: `retrieve()` in the Node store

**Files:**
- Modify: `packages/xberg-wasm-runtime/src/store-node.ts`
- Test: `packages/xberg-wasm-runtime/src/store-node.test.ts`

**Interfaces:**
- Consumes: `reciprocalRankFusion` from `./retrieve-fusion.js` (Task 2); `RetrieveOptions` from `./types.js` (Task 2).
- Produces: `createNodeVectorStore(...)`'s returned object gains `retrieve` — same object shape as before, one new method.

- [ ] **Step 1: Write the failing tests**

Add to `packages/xberg-wasm-runtime/src/store-node.test.ts` (inside the existing `describe` block, using the existing `store`/`testCollection`/`vectorDim` setup):

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
	const results = await store.retrieve(testCollection, { mode: "fulltext", queryText: "brown fox", k: 5 });
	expect(results[0]?.text).toBe("the quick brown fox");
});

it("retrieve() in vector mode matches query() behavior", async () => {
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
	const results = await store.retrieve(testCollection, { mode: "vector", queryVector: [1, 0, 0, 0], k: 5 });
	expect(results[0]?.text).toBe("apple");
});

it("retrieve() in hybrid mode ranks a chunk good on both signals above either extreme", async () => {
	await store.ensureCollection(testCollection, vectorDim);
	const doc: DocumentRecord = { documentId: "doc-1", sourceId: "src-1", collectionId: testCollection };
	await store.upsertDocument(testCollection, doc, [
		{
			// Exact vector match, textually irrelevant to the query text.
			sourceId: "src-1",
			chunkIndex: 0,
			text: "zzz unrelated content",
			startOffset: 0,
			endOffset: 22,
			embedding: new Float32Array([1, 0, 0, 0]),
		},
		{
			// Textually exact, vector-distant.
			sourceId: "src-1",
			chunkIndex: 1,
			text: "hybrid search test phrase",
			startOffset: 23,
			endOffset: 49,
			embedding: new Float32Array([0, 0, 0, 1]),
		},
		{
			// Moderately good on both.
			sourceId: "src-1",
			chunkIndex: 2,
			text: "hybrid search related content",
			startOffset: 50,
			endOffset: 80,
			embedding: new Float32Array([0.7, 0, 0, 0.7]),
		},
	]);
	const results = await store.retrieve(testCollection, {
		mode: "hybrid",
		queryVector: [1, 0, 0, 0],
		queryText: "hybrid search test phrase",
		k: 3,
	});
	expect(results[0]?.chunkId).toBe("src-1:2");
});

it("retrieve() throws for fulltext mode without queryText", async () => {
	await store.ensureCollection(testCollection, vectorDim);
	await expect(store.retrieve(testCollection, { mode: "fulltext", k: 5 })).rejects.toThrow(/queryText/);
});

it("retrieve() throws for hybrid mode missing either query input", async () => {
	await store.ensureCollection(testCollection, vectorDim);
	await expect(store.retrieve(testCollection, { mode: "hybrid", queryText: "x", k: 5 })).rejects.toThrow(
		/queryVector/,
	);
});
```

Add the import at the top of the test file: `import type { VectorStoreInterface, DocumentRecord, ChunkRecord } from "./types";` already exists — no change needed there.

- [ ] **Step 2: Run tests to verify they fail**

```bash
npx vitest run src/store-node.test.ts
```

Expected: FAIL — `store.retrieve is not a function`.

- [ ] **Step 3: Implement `retrieve()` in `store-node.ts`**

Add the import at the top of `packages/xberg-wasm-runtime/src/store-node.ts`:

```typescript
import { reciprocalRankFusion } from "./retrieve-fusion.js";
import type { VectorStoreInterface, DocumentRecord, ChunkRecord, GraphEdge, CacheConfig, RetrieveOptions } from "./types.js";
```

(This replaces the existing `import type { VectorStoreInterface, ... } from "./types.js";` line — add `RetrieveOptions` to the existing named-type list rather than duplicating the import.)

Add a `HYBRID_CANDIDATE_MULTIPLIER` constant near the top of the file (after imports, before `createNodeVectorStore`):

```typescript
const HYBRID_CANDIDATE_MULTIPLIER = 4;
```

Add two new functions inside `createNodeVectorStore`, after the existing `query` function:

```typescript
async function fullTextQuery(
	collection: string,
	queryText: string,
	k: number,
): Promise<Array<{ chunkId: string; text: string; score: number }>> {
	const rows = db
		.prepare(
			`SELECT f.chunk_id AS chunkId, c.text AS text, bm25(chunks_fts) AS rank
       FROM chunks_fts f JOIN chunks c ON c.chunk_id = f.chunk_id AND c.collection = f.collection
       WHERE chunks_fts MATCH ? AND f.collection = ? ORDER BY rank LIMIT ?`,
		)
		.all(queryText, collection, k) as Array<{ chunkId: string; text: string; rank: number }>;
	// bm25() is smaller-is-better; negate for the larger-is-better score convention query() already uses.
	return rows.map((r) => ({ chunkId: r.chunkId, text: r.text, score: -r.rank }));
}

async function retrieve(
	collection: string,
	opts: RetrieveOptions,
): Promise<Array<{ chunkId: string; text: string; score: number }>> {
	if (opts.mode === "vector") {
		if (!opts.queryVector) throw new Error("retrieve: queryVector is required for mode 'vector'");
		return query(collection, opts.queryVector, opts.k);
	}
	if (opts.mode === "fulltext") {
		if (!opts.queryText) throw new Error("retrieve: queryText is required for mode 'fulltext'");
		return fullTextQuery(collection, opts.queryText, opts.k);
	}
	if (!opts.queryVector) throw new Error("retrieve: queryVector is required for mode 'hybrid'");
	if (!opts.queryText) throw new Error("retrieve: queryText is required for mode 'hybrid'");
	const candidateK = opts.k * HYBRID_CANDIDATE_MULTIPLIER;
	const [vectorResults, textResults] = await Promise.all([
		query(collection, opts.queryVector, candidateK),
		fullTextQuery(collection, opts.queryText, candidateK),
	]);
	return reciprocalRankFusion([vectorResults, textResults]).slice(0, opts.k);
}
```

Add `retrieve` to the returned object at the bottom of `createNodeVectorStore`:

```typescript
	return {
		ensureCollection,
		upsertDocument,
		query,
		retrieve,
		delete: deleteDocument,
		listCollections,
		dropCollection,
		createEdge,
		traverseGraph,
	};
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
npx vitest run src/store-node.test.ts
```

Expected: PASS, all tests including the 5 new ones. If the hybrid test's ranking assertion fails, check the embedding vectors in the test fixture actually produce the intended vector-similarity ordering (cosine distance via `sqlite-vec`'s default metric) before assuming the fusion logic is wrong.

- [ ] **Step 5: Commit**

```bash
git add packages/xberg-wasm-runtime/src/store-node.ts packages/xberg-wasm-runtime/src/store-node.test.ts
git commit -m "feat(wasm-runtime): implement retrieve() with FTS5 + RRF fusion in Node store"
```

---

### Task 4: `retrieve()` in the browser Worker + FTS5 compiled-in verification

**Files:**
- Modify: `packages/xberg-wasm-runtime/src/store-worker.ts`
- Modify: `packages/xberg-wasm-runtime/tests/browser/store.spec.ts`

**Interfaces:**
- Consumes: `reciprocalRankFusion` from `./retrieve-fusion.js` (Task 2); `RetrieveOptions` from `./types.js` (Task 2).
- Produces: `StoreWorkerRequest` gains a `"retrieve"` variant; the worker's `dispatch()` handles it. This is a real, load-bearing verification gate for Component 1 of the design spec — if FTS5 turns out not to be compiled into the vendored `wasm/sqlite-vec/sqlite3.wasm` build, this task's Step 4 will fail and that must be reported, not routed around.

- [ ] **Step 1: Add the `retrieve` request variant and implement fulltext/retrieve logic**

In `packages/xberg-wasm-runtime/src/store-worker.ts`, add the import:

```typescript
import { reciprocalRankFusion } from "./retrieve-fusion.js";
import type { ChunkRecord, DocumentRecord, GraphEdge, RetrieveOptions } from "./types.js";
```

(Extend the existing `import type { ChunkRecord, DocumentRecord, GraphEdge } from "./types.js";` line with `RetrieveOptions` rather than duplicating it.)

Add `"retrieve"` to the `StoreWorkerRequest` union (insert after the existing `"query"` variant):

```typescript
	| { op: "query"; collection: string; queryVector: number[]; k: number; id: number }
	| { op: "retrieve"; collection: string; opts: RetrieveOptions; id: number }
```

Add a `HYBRID_CANDIDATE_MULTIPLIER` constant near the top of the file (after the existing `let database` declaration or near other module-level constants):

```typescript
const HYBRID_CANDIDATE_MULTIPLIER = 4;
```

Add two new functions after the existing `query` function:

```typescript
function fullTextQuery(collection: string, queryText: string, k: number): Array<{ chunkId: string; text: string; score: number }> {
	const result = rows<{ chunkId: string; text: string; rank: number }>(
		requireDatabase(),
		`SELECT f.chunk_id AS chunkId, c.text AS text, bm25(chunks_fts) AS rank
     FROM chunks_fts f JOIN chunks c ON c.chunk_id = f.chunk_id AND c.collection = f.collection
     WHERE chunks_fts MATCH ? AND f.collection = ? ORDER BY rank LIMIT ?`,
		[queryText, collection, k],
	);
	return result.map((row) => ({ chunkId: row.chunkId, text: row.text, score: -row.rank }));
}

function retrieve(collection: string, opts: RetrieveOptions): Array<{ chunkId: string; text: string; score: number }> {
	if (opts.mode === "vector") {
		if (!opts.queryVector) throw new Error("retrieve: queryVector is required for mode 'vector'");
		return query(collection, opts.queryVector, opts.k);
	}
	if (opts.mode === "fulltext") {
		if (!opts.queryText) throw new Error("retrieve: queryText is required for mode 'fulltext'");
		return fullTextQuery(collection, opts.queryText, opts.k);
	}
	if (!opts.queryVector) throw new Error("retrieve: queryVector is required for mode 'hybrid'");
	if (!opts.queryText) throw new Error("retrieve: queryText is required for mode 'hybrid'");
	const candidateK = opts.k * HYBRID_CANDIDATE_MULTIPLIER;
	const vectorResults = query(collection, opts.queryVector, candidateK);
	const textResults = fullTextQuery(collection, opts.queryText, candidateK);
	return reciprocalRankFusion([vectorResults, textResults]).slice(0, opts.k);
}
```

Add a `case "retrieve":` branch to the `dispatch` function's `switch`, right after the existing `case "query":` branch:

```typescript
		case "retrieve":
			return retrieve(request.collection, request.opts);
```

- [ ] **Step 2: Run the existing TypeScript check**

```bash
cd packages/xberg-wasm-runtime
npx tsc --noEmit
```

Expected: no new errors (this file has no existing vitest unit tests — it's tested exclusively through the real-browser Playwright suite added in the next step, since it depends on `navigator.storage`/`crossOriginIsolated`, which vitest's `environment: "node"` config cannot provide).

- [ ] **Step 3: Add real-browser Playwright tests, including the FTS5 compiled-in gate**

Add to `packages/xberg-wasm-runtime/tests/browser/store.spec.ts` (after the existing two `test(...)` blocks):

```typescript
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
		await store.upsertDocument(
			"hybrid-check",
			{ documentId: "d1", sourceId: "s1", collectionId: "hybrid-check" },
			[
				{ sourceId: "s1", chunkIndex: 0, text: "zzz unrelated", startOffset: 0, endOffset: 13, embedding: new Float32Array([1, 0, 0, 0]) },
				{ sourceId: "s1", chunkIndex: 1, text: "hybrid phrase match", startOffset: 14, endOffset: 34, embedding: new Float32Array([0, 0, 0, 1]) },
				{ sourceId: "s1", chunkIndex: 2, text: "hybrid phrase related", startOffset: 35, endOffset: 57, embedding: new Float32Array([0.7, 0, 0, 0.7]) },
			],
		);
		const results = await store.retrieve("hybrid-check", {
			mode: "hybrid",
			queryVector: [1, 0, 0, 0],
			queryText: "hybrid phrase match",
			k: 3,
		});
		return results[0]?.chunkId;
	}, `/hybrid-check-${Date.now()}.sqlite3`);
	expect(topChunkId).toBe("s1:2");
});
```

- [ ] **Step 4: Run the real-browser tests**

```bash
pnpm test:browser
```

Expected: PASS, both new tests. If the FTS5 test fails with a SQLite error (e.g. "no such module: fts5"), STOP and report BLOCKED with the exact error — per the design spec's error-handling requirement, do not add a silent vector-only fallback to work around it; this would mean the vendored WASM build needs rebuilding with FTS5 enabled, which is a build-script change outside this task's scope.

- [ ] **Step 5: Commit**

```bash
git add packages/xberg-wasm-runtime/src/store-worker.ts packages/xberg-wasm-runtime/tests/browser/store.spec.ts
git commit -m "feat(wasm-runtime): implement retrieve() in browser Worker, verify FTS5 compiled in"
```

---

### Task 5: Wire `retrieve()` through the browser RPC client

**Files:**
- Modify: `packages/xberg-wasm-runtime/src/store-browser.ts`

**Interfaces:**
- Consumes: the `"retrieve"` request variant from `./store-worker.js` (Task 4); `RetrieveOptions` from `./types.js` (Task 2).
- Produces: `createBrowserVectorStore(...)`'s returned object gains `retrieve` — completing `VectorStoreInterface` on the browser path. `store.ts`'s `createVectorStore` dispatcher needs no changes — it already delegates to `createNodeVectorStore`/`createBrowserVectorStore`, both of which will now fully implement `VectorStoreInterface`.

- [ ] **Step 1: Add `retrieve` to the RPC client**

In `packages/xberg-wasm-runtime/src/store-browser.ts`, add `RetrieveOptions` to the existing type import:

```typescript
import type { VectorStoreInterface, DocumentRecord, ChunkRecord, GraphEdge, CacheConfig, RetrieveOptions } from "./types.js";
```

Add one line to the returned object (after the existing `query` entry):

```typescript
		query: (collection: string, queryVector: number[], k: number) =>
			call<Array<{ chunkId: string; text: string; score: number }>>({ op: "query", collection, queryVector, k }),
		retrieve: (collection: string, opts: RetrieveOptions) =>
			call<Array<{ chunkId: string; text: string; score: number }>>({ op: "retrieve", collection, opts }),
```

- [ ] **Step 2: Run the full TypeScript check across the package**

```bash
cd packages/xberg-wasm-runtime
npx tsc --noEmit
```

Expected: no errors anywhere — `VectorStoreInterface` is now fully implemented by both `createNodeVectorStore` and `createBrowserVectorStore`.

- [ ] **Step 3: Run the full test suite**

```bash
pnpm test:run
pnpm test:browser
npx oxlint src/
```

Expected: all PASS, zero lint errors.

- [ ] **Step 4: Commit**

```bash
git add packages/xberg-wasm-runtime/src/store-browser.ts
git commit -m "feat(wasm-runtime): wire retrieve() through the browser RPC client"
```

---

## Part 2: Embedding Model Upgrade

### Task 6: Validate `Xenova/bge-m3` loads under all three backend configurations

**Files:**
- Create: `packages/xberg-wasm-runtime/scripts/verify-embedding-model.mjs`

**Interfaces:**
- Consumes: `@huggingface/transformers` (already a dependency).
- Produces: a reusable manual verification script — not part of the automated test suite (this is a one-time spike gate, mirroring the sqlite-vec bundle's `build-sqlite-vec-wasm.mjs` verification discipline). Its output determines whether Task 7 proceeds with `Xenova/bge-m3` or a documented fallback.

- [ ] **Step 1: Write the verification script**

Create `packages/xberg-wasm-runtime/scripts/verify-embedding-model.mjs`:

```javascript
import { pipeline } from "@huggingface/transformers";

const modelId = process.argv[2] ?? "Xenova/bge-m3";
const configs = [
	{ device: "cpu", dtype: "q8" },
	{ device: "wasm", dtype: "q8" },
	{ device: "webgpu", dtype: "fp32" },
];

for (const config of configs) {
	console.log(`\n--- ${modelId} device=${config.device} dtype=${config.dtype} ---`);
	const start = performance.now();
	try {
		const extractor = await pipeline("feature-extraction", modelId, config);
		const output = await extractor(["hello world"], { pooling: "mean", normalize: false });
		const elapsedMs = Math.round(performance.now() - start);
		console.log(`OK: dims=[${output.dims.join(", ")}] loaded in ${elapsedMs}ms`);
	} catch (error) {
		console.error(`FAILED: ${error instanceof Error ? error.message : String(error)}`);
	}
}
```

- [ ] **Step 2: Run it and record the real results**

```bash
cd packages/xberg-wasm-runtime
node scripts/verify-embedding-model.mjs Xenova/bge-m3
```

Expected: three `OK: dims=[1, 1024] loaded in ...ms` lines (one per config; `webgpu` may report a WebGPU-unavailable error in a headless Node environment — that specific failure is acceptable here since WebGPU is a browser-only path exercised for real in Task 4/5's Playwright suite for the store, not this script; a `cpu`/`wasm` failure is not acceptable). If `cpu`/`wasm` fail, STOP — report BLOCKED with the exact error, and re-run this script against `Xenova/bge-small-en-v1.5` as the documented fallback before proceeding to Task 7.

- [ ] **Step 3: Commit**

```bash
git add packages/xberg-wasm-runtime/scripts/verify-embedding-model.mjs
git commit -m "chore(wasm-runtime): add embedding model verification script, validate bge-m3"
```

---

### Task 7: Swap the default embedding model to BGE-M3

**Files:**
- Modify: `packages/xberg-wasm-runtime/src/embedder.ts`
- Modify: `packages/xberg-wasm-runtime/src/embedder.test.ts`
- Modify: `packages/xberg-wasm-runtime/src/cache.ts`

**Interfaces:**
- Consumes: verified `Xenova/bge-m3` model id (Task 6).
- Produces: `createEmbedder`'s default output dimension changes from 384 to 1024 — no signature change, `EmbedderInterface.embed` is unaffected.

- [ ] **Step 1: Update the failing assertions first (TDD: adjust the test to the new expected behavior before touching the implementation)**

In `packages/xberg-wasm-runtime/src/embedder.test.ts`, replace the `beforeAll` block:

```typescript
	beforeAll(async () => {
		// Xenova/bge-m3 — verified real, live transformers.js-compatible ONNX
		// export of BAAI/bge-m3 (1024-dim, multilingual) via
		// scripts/verify-embedding-model.mjs. Replaces the earlier
		// Xenova/all-MiniLM-L6-v2 default, which was never a deliberate quality
		// choice (substituted only because the original plan's test model ID
		// didn't exist on the Hub).
		embedder = await createEmbedder({
			models: { embedder: "Xenova/bge-m3" },
		});
	}, 180_000);
```

Update the two length-bearing assertions — in `"embeds a single string to a normalized vector"`, no change needed (it already asserts `length > 0` generically). Add one new test after it:

```typescript
	it("produces 1024-dimensional vectors (bge-m3)", async () => {
		const result = await embedder.embed(["dimension check"]);
		expect(result[0]?.length).toBe(1024);
	}, 60_000);
```

- [ ] **Step 2: Run to verify the new test fails and existing ones still pass against the old default**

```bash
cd packages/xberg-wasm-runtime
npx vitest run src/embedder.test.ts
```

Expected: the new "1024-dimensional" test PASSES already (since `beforeAll` now loads `bge-m3` directly) — this task's TDD cycle is really about `DEFAULT_MODEL`, verified next.

- [ ] **Step 3: Swap `DEFAULT_MODEL`**

In `packages/xberg-wasm-runtime/src/embedder.ts`, change:

```typescript
const DEFAULT_MODEL = "Xenova/bge-m3";
```

- [ ] **Step 4: Update `cache.ts`'s model metadata**

In `packages/xberg-wasm-runtime/src/cache.ts`, update the `MODELS` array's embedder entry and the `MODEL_NAME_TO_HANDLE` map:

```typescript
const MODEL_NAME_TO_HANDLE: Record<string, WarmHandle> = {
	"Embedder (minilm-l6-v2)": "embedding",
	"Embedder (all-MiniLM-L6-v2)": "embedding",
	"Embedder (bge-m3)": "embedding",
	"GLiNER2 NER": "ner",
	"BERT NER": "ner",
	"PP-OCRv6 OCR": "ocr",
};

const MODELS: ModelInfo[] = [
	{
		name: "Embedder (bge-m3)",
		path: "Xenova/bge-m3",
	},
	{
		name: "BERT NER",
		path: "Xenova/bert-base-NER",
	},
];
```

(Keep the two legacy MiniLM keys in `MODEL_NAME_TO_HANDLE` for backward compatibility with any caller still passing the old display name to `warm({ modelNames: [...] })` — only `MODELS` itself, which drives `status()`'s on-disk lookup, needs the entry replaced.)

- [ ] **Step 5: Run the full test suite for this package**

```bash
npx vitest run src/embedder.test.ts src/cache.test.ts
npx tsc --noEmit
npx oxlint src/
```

Expected: all PASS, zero lint errors.

- [ ] **Step 6: Commit**

```bash
git add packages/xberg-wasm-runtime/src/embedder.ts packages/xberg-wasm-runtime/src/embedder.test.ts packages/xberg-wasm-runtime/src/cache.ts
git commit -m "feat(wasm-runtime): swap default embedder to bge-m3 (1024-dim, multilingual)"
```

---

## Part 3: PII-Aware Entity Detection

### Task 8: Fix the injected-NER contract mismatch

**Files:**
- Modify: `packages/xberg-wasm-runtime/src/types.ts`
- Modify: `packages/xberg-wasm-runtime/src/ner.ts`
- Modify: `packages/xberg-wasm-runtime/src/ner.test.ts`

**Interfaces:**
- Consumes: nothing new.
- Produces: `NerInterface.ner`'s signature changes from `ner(text: string, opts?: NerOpts): Promise<Entity[]>` to `ner(text: string, categories?: string[], threshold?: number): Promise<Entity[]>` — matching what `crates/xberg-wasm/src/bridge/ner.rs`'s `call_injected_ner` actually sends (`func.apply(&obj, [text, categoriesArray])`, positional, not an options object). `NerOpts` is removed (no longer used anywhere after this task).

- [ ] **Step 1: Update the failing test call sites**

In `packages/xberg-wasm-runtime/src/ner.test.ts`, change the three calls that use the old options-object shape:

```typescript
		const personEntities = await ner.ner(text, ["PER"]);
```

```typescript
		const emailEntities = await ner.ner(text, ["EMAIL"]);
```

```typescript
		const highThresholdEntities = await ner.ner(text, undefined, 0.99);
```

- [ ] **Step 2: Run to verify it fails**

```bash
cd packages/xberg-wasm-runtime
npx vitest run src/ner.test.ts
```

Expected: FAIL — `ner()` still expects an options object, so `["PER"]` is passed where `opts` is expected and `opts.categories`/`opts.threshold` read as `undefined`.

- [ ] **Step 3: Update `NerInterface` and remove `NerOpts`**

In `packages/xberg-wasm-runtime/src/types.ts`, replace:

```typescript
export interface NerOpts {
	categories?: string[];
	threshold?: number;
}

export interface NerInterface {
	ner(text: string, opts?: NerOpts): Promise<Entity[]>;
}
```

with:

```typescript
export interface NerInterface {
	ner(text: string, categories?: string[], threshold?: number): Promise<Entity[]>;
}
```

- [ ] **Step 4: Update `ner.ts`**

In `packages/xberg-wasm-runtime/src/ner.ts`, change the import line:

```typescript
import type { CacheConfig, Entity, NerInterface } from "./types.js";
```

Change the `ner` function signature and its call to `mergeEntities`:

```typescript
		/**
		 * Named entity recognition on the given text. Returns a list of named
		 * entities with their labels, text, and confidence scores.
		 *
		 * IMPORTANT: The currently-loaded model (Xenova/bert-base-NER) recognizes
		 * only a fixed label set: PER (person), ORG (organization), LOC (location),
		 * and MISC (miscellaneous). The `categories` parameter filters results to
		 * only entities matching those labels, but only works within this fixed
		 * set. Requesting categories outside this set (e.g., EMAIL, PHONE) will
		 * silently return no results with no error — packages/xberg-wasm-runtime's
		 * pii.ts regex layer exists specifically to cover that gap deterministically.
		 *
		 * `categories` is a plain positional array (not an options object) because
		 * this must match crates/xberg-wasm/src/bridge/ner.rs's
		 * call_injected_ner, which calls `ner(text, categories)` positionally —
		 * the Rust bridge is the fixed contract this signature exists to satisfy.
		 *
		 * @param text The input text to analyze
		 * @param categories Optional label filter
		 * @param threshold Optional minimum confidence score
		 * @returns Array of entities with label, text, position, and confidence score
		 */
		async function ner(text: string, categories?: string[], threshold?: number): Promise<Entity[]> {
			if (!text || text.length === 0) return [];

			try {
				const predictions = await tokenClassifier(text);
				const tokens = (
					Array.isArray(predictions) ? predictions : [predictions]
				) as TokenClassificationSingle[];

				return mergeEntities(tokens, text, categories, threshold);
			} catch (err) {
				console.error("[ner] classification failed:", err);
				return [];
			}
		}
```

Change `mergeEntities`'s signature and its filter logic:

```typescript
function mergeEntities(
	tokens: TokenClassificationSingle[],
	sourceText: string,
	categories?: string[],
	threshold?: number,
): Entity[] {
```

(leave the token-merging loop body unchanged — only the final `.filter(...)` block changes:)

```typescript
	return entities.filter((entity) => {
		if (threshold !== undefined && (entity.score ?? 0) < threshold) {
			return false;
		}
		if (categories && !categories.includes(entity.label)) {
			return false;
		}
		return true;
	});
```

- [ ] **Step 5: Run to verify it passes**

```bash
npx vitest run src/ner.test.ts src/contract.test.ts
npx tsc --noEmit
```

Expected: PASS, no type errors anywhere (confirms nothing else in the package referenced `NerOpts`).

- [ ] **Step 6: Commit**

```bash
git add packages/xberg-wasm-runtime/src/types.ts packages/xberg-wasm-runtime/src/ner.ts packages/xberg-wasm-runtime/src/ner.test.ts
git commit -m "fix(wasm-runtime): match ner() signature to the engine's positional call contract"
```

---

### Task 9: Port the regex PII detector

**Files:**
- Create: `packages/xberg-wasm-runtime/src/pii.ts`
- Test: `packages/xberg-wasm-runtime/src/pii.test.ts`

**Interfaces:**
- Consumes: `Entity` from `./types.js` (existing — `label`/`text`/`start`/`end`/`score` shape).
- Produces: `detectPii(text: string, filterCategories?: string[]): PiiFinding[]`, `mergeNerEntities(regex: PiiFinding[], entities: Entity[]): PiiFinding[]`, `groupByCategory(findings: PiiFinding[]): Record<string, number>`, `detectPiiWithNer(text: string, nerResult: Entity[], filterCategories?: string[]): PiiFinding[]` — the last one is new (not in `mcp-server`'s original), combining the other three for this package's callers.

- [ ] **Step 1: Write the failing tests**

Create `packages/xberg-wasm-runtime/src/pii.test.ts` (ported directly from `mcp-server/tests/redaction.test.ts`'s `detectPii`/`groupByCategory` coverage, plus new tests for the `Entity`-shape adapter):

```typescript
import { describe, it, expect } from "vitest";
import { detectPii, groupByCategory, detectPiiWithNer } from "./pii";
import type { Entity } from "./types";

describe("detectPii", () => {
	it("detects email addresses", () => {
		const findings = detectPii("Contact us at info@example.com for details.");
		expect(findings).toHaveLength(1);
		expect(findings[0]?.category).toBe("EMAIL");
		expect(findings[0]?.original).toBe("info@example.com");
		expect(findings[0]?.token).toBe("[EMAIL_1]");
		expect(findings[0]?.confidence).toBeGreaterThan(0.9);
	});

	it("detects phone numbers", () => {
		const findings = detectPii("Call us at 555-867-5309.");
		expect(findings.some((f) => f.category === "PHONE")).toBe(true);
	});

	it("detects SSN", () => {
		const findings = detectPii("SSN: 123-45-6789");
		expect(findings.some((f) => f.category === "SSN")).toBe(true);
	});

	it("detects credit card numbers", () => {
		const findings = detectPii("Card: 4111 1111 1111 1111");
		expect(findings.some((f) => f.category === "CREDIT_CARD")).toBe(true);
	});

	it("filters by category", () => {
		const findings = detectPii("Email: test@test.com, SSN: 123-45-6789", ["EMAIL"]);
		expect(findings.every((f) => f.category === "EMAIL")).toBe(true);
	});

	it("returns empty array for clean text", () => {
		expect(detectPii("Hello, how are you today?")).toHaveLength(0);
	});
});

describe("groupByCategory", () => {
	it("counts findings per category", () => {
		const findings = detectPii("a@b.com c@d.com 555-123-4567");
		const groups = groupByCategory(findings);
		expect(groups["EMAIL"]).toBe(2);
		expect(groups["PHONE"]).toBe(1);
	});
});

describe("detectPiiWithNer", () => {
	it("merges regex findings with NER entities using the Entity (label/score) shape", () => {
		const nerResult: Entity[] = [{ label: "person", text: "Alice", start: 0, end: 5, score: 0.95 }];
		const findings = detectPiiWithNer("Alice's email is a@b.com", nerResult);
		expect(findings.some((f) => f.category === "EMAIL")).toBe(true);
		expect(findings.some((f) => f.category === "NAME" && f.original === "Alice")).toBe(true);
	});

	it("still detects regex PII when nerResult is empty", () => {
		const findings = detectPiiWithNer("Contact: a@b.com", []);
		expect(findings.some((f) => f.category === "EMAIL")).toBe(true);
	});
});
```

- [ ] **Step 2: Run to verify it fails**

```bash
cd packages/xberg-wasm-runtime
npx vitest run src/pii.test.ts
```

Expected: FAIL — `Cannot find module './pii'`.

- [ ] **Step 3: Implement `pii.ts`**

Create `packages/xberg-wasm-runtime/src/pii.ts`:

```typescript
import type { Entity } from "./types.js";

export interface PiiFinding {
	token: string;
	category: string;
	original: string;
	start: number;
	end: number;
	confidence: number;
}

const PATTERNS: Array<{ category: string; pattern: RegExp; confidence: number }> = [
	{ category: "EMAIL", pattern: /\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b/g, confidence: 0.95 },
	{ category: "PHONE", pattern: /\b(?:\+?\d{1,3}[-.\s]?)?\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}\b/g, confidence: 0.85 },
	{ category: "SSN", pattern: /\b\d{3}-\d{2}-\d{4}\b/g, confidence: 0.9 },
	{ category: "CREDIT_CARD", pattern: /\b(?:\d{4}[-\s]?){3}\d{4}\b/g, confidence: 0.9 },
	{ category: "IP_ADDRESS", pattern: /\b(?:\d{1,3}\.){3}\d{1,3}\b/g, confidence: 0.8 },
	{ category: "DATE_ISO", pattern: /\b\d{4}-\d{2}-\d{2}\b/g, confidence: 0.7 },
	{ category: "DATE_MDY", pattern: /\b\d{1,2}\/\d{1,2}\/\d{2,4}\b/g, confidence: 0.7 },
	{ category: "IBAN", pattern: /\b[A-Z]{2}\d{2}[A-Z0-9]{4,30}\b/g, confidence: 0.85 },
	{ category: "SWIFT_BIC", pattern: /\b[A-Z]{6}[A-Z0-9]{2}([A-Z0-9]{3})?\b/g, confidence: 0.8 },
	{ category: "POSTAL_CODE_US", pattern: /\b\d{5}(?:-\d{4})?\b/g, confidence: 0.75 },
	{ category: "POSTAL_CODE_UK", pattern: /\b[A-Z]{1,2}\d[A-Z\d]?\s?\d[A-Z]{2}\b/g, confidence: 0.75 },
];

/**
 * Deterministic regex-based PII detection. Ported from
 * mcp-server/src/redaction/detect.ts (pure function, no Node-specific I/O —
 * runs identically in Node and the browser) so this package's structured PII
 * coverage (email/phone/SSN/etc.) does not depend on any NER model's label
 * set — deliberately kept as a duplicate rather than a shared package
 * (packages/xberg-wasm-runtime has no dependency on mcp-server today).
 */
export function detectPii(text: string, filterCategories?: string[]): PiiFinding[] {
	const findings: PiiFinding[] = [];
	const counters: Record<string, number> = {};

	for (const { category, pattern, confidence } of PATTERNS) {
		if (filterCategories && !filterCategories.includes(category)) continue;

		const regex = new RegExp(pattern.source, pattern.flags);
		let match: RegExpExecArray | null;
		while ((match = regex.exec(text)) !== null) {
			counters[category] = (counters[category] ?? 0) + 1;
			findings.push({
				token: `[${category}_${counters[category]}]`,
				category,
				original: match[0],
				start: match.index,
				end: match.index + match[0].length,
				confidence,
			});
		}
	}

	return findings.sort((a, b) => a.start - b.start);
}

const NER_LABEL_TO_PII: Record<string, string> = {
	person: "NAME",
	organization: "ORG",
	location: "LOCATION",
	email: "EMAIL",
	phone: "PHONE",
	date: "DATE",
	money: "MONEY",
	url: "URL",
};

function spansOverlap(a: PiiFinding, b: { start: number; end: number }): boolean {
	return a.start < b.end && b.start < a.end;
}

/**
 * Merge regex findings with NER entities (this package's `Entity` shape:
 * `label`/`score`, not mcp-server's `NerEntity` shape of `category`/
 * `confidence` — the field names differ between the two packages by design,
 * this function is the adapter).
 */
export function mergeNerEntities(regex: PiiFinding[], entities: Entity[]): PiiFinding[] {
	const findings = [...regex];
	const counters: Record<string, number> = {};
	for (const f of findings) {
		counters[f.category] = Math.max(counters[f.category] ?? 0, Number(f.token.match(/_(\d+)\]$/)?.[1] ?? 0));
	}

	for (const entity of entities) {
		const category = NER_LABEL_TO_PII[entity.label.toLowerCase()] ?? `NER_${entity.label.toUpperCase()}`;
		const { text: entityText, start, end } = entity;
		const entityConfidence = entity.score ?? 0.8;

		const overlap = findings.find((f) => spansOverlap(f, { start, end }));
		if (overlap) {
			if (entityConfidence > overlap.confidence) {
				overlap.category = category;
				overlap.confidence = entityConfidence;
				overlap.original = entityText;
				overlap.start = start;
				overlap.end = end;
			}
			continue;
		}

		counters[category] = (counters[category] ?? 0) + 1;
		findings.push({
			token: `[${category}_${counters[category]}]`,
			category,
			original: entityText,
			start,
			end,
			confidence: entityConfidence,
		});
	}

	return findings.sort((a, b) => a.start - b.start);
}

export function groupByCategory(findings: PiiFinding[]): Record<string, number> {
	const grouped: Record<string, number> = {};
	for (const f of findings) {
		grouped[f.category] = (grouped[f.category] ?? 0) + 1;
	}
	return grouped;
}

/**
 * Runs regex PII detection and merges it with an already-computed NER
 * result (from either the injected JS NER path or, once wired, the
 * in-binary Candle fallback — this function does not care which produced
 * `nerResult`, or whether it's empty). Regex-only detection still functions
 * as a floor when `nerResult` is `[]`.
 */
export function detectPiiWithNer(text: string, nerResult: Entity[], filterCategories?: string[]): PiiFinding[] {
	const regexFindings = detectPii(text, filterCategories);
	return mergeNerEntities(regexFindings, nerResult);
}
```

- [ ] **Step 4: Run to verify it passes**

```bash
npx vitest run src/pii.test.ts
npx tsc --noEmit
npx oxlint src/
```

Expected: PASS, zero lint errors.

- [ ] **Step 5: Commit**

```bash
git add packages/xberg-wasm-runtime/src/pii.ts packages/xberg-wasm-runtime/src/pii.test.ts
git commit -m "feat(wasm-runtime): port regex PII detector from mcp-server, add NER merge"
```

---

### Task 10: Validate `fastino/gliner2-privacy-filter-PII-multi` loads via `Gliner2Candle::from_bytes`

**Files:**
- Modify: `crates/xberg-gliner-candle/tests/smoke.rs`

**Interfaces:**
- Consumes: `xberg_gliner_candle::Gliner2Candle::from_bytes(safetensors: &[u8], tokenizer_json: &[u8], encoder_config_json: &[u8]) -> Result<Self>` (existing, `crates/xberg-gliner-candle/src/model.rs`), `Gliner2Candle::extract_ner(&self, text: &str, labels: &[&str], threshold: f32) -> Result<Vec<Span>>` (existing).
- Produces: a real, gated (not automatically run in CI) confirmation that the pinned model's real files load without a tensor-name/shape mismatch — the one thing the design spec's config-schema inspection could not confirm by reading alone. This test gates Task 11 — do not proceed to Task 11 until this passes against the real downloaded files.

- [ ] **Step 1: Download the model files**

```bash
mkdir -p /tmp/gliner2-pii-multi/encoder_config
cd /tmp/gliner2-pii-multi
curl -L -o model.safetensors "https://huggingface.co/fastino/gliner2-privacy-filter-PII-multi/resolve/main/model.safetensors"
curl -L -o tokenizer.json "https://huggingface.co/fastino/gliner2-privacy-filter-PII-multi/resolve/main/tokenizer.json"
curl -L -o encoder_config/config.json "https://huggingface.co/fastino/gliner2-privacy-filter-PII-multi/resolve/main/encoder_config/config.json"
```

Expected: `model.safetensors` is ~1.17 GiB. If any download fails or a file is unexpectedly small (truncated), STOP and re-check the URL against the repo's current `Files and versions` tab before proceeding — do not test against a partial file.

- [ ] **Step 2: Write the gated test**

Add to `crates/xberg-gliner-candle/tests/smoke.rs` (after the existing `base_model_extracts_entities_and_adapter_changes_output` test):

```rust
#[cfg(not(target_arch = "wasm32"))]
#[test]
#[ignore = "requires real fastino/gliner2-privacy-filter-PII-multi safetensors on disk"]
fn pii_model_loads_from_bytes_and_extracts_entities() {
    let Ok(model_dir) = std::env::var("GLINER2_PII_MODEL_DIR") else {
        eprintln!("skipping: GLINER2_PII_MODEL_DIR not set");
        return;
    };
    let dir = std::path::Path::new(&model_dir);

    let safetensors = std::fs::read(dir.join("model.safetensors")).expect("read model.safetensors");
    let tokenizer_json = std::fs::read(dir.join("tokenizer.json")).expect("read tokenizer.json");
    let encoder_config_json =
        std::fs::read(dir.join("encoder_config").join("config.json")).expect("read encoder_config/config.json");

    // Exercises the exact wasm32-relevant code path (from_bytes, no filesystem
    // reads inside the constructor itself) even though this test runs
    // natively — Candle's tensor ops are portable, and this is the real
    // check the design spec's config-schema inspection could not perform by
    // reading alone: does model.safetensors's tensor naming actually carry
    // the `encoder.` prefix encoder.rs strips via vb.pp("encoder")?
    let model = xberg_gliner_candle::Gliner2Candle::from_bytes(&safetensors, &tokenizer_json, &encoder_config_json)
        .expect("from_bytes must load the real pinned PII model without a tensor mismatch");

    let text = "Email john.smith@acme.com or call +1 415 555 0199. Signed, Jane Doe.";
    let labels = ["email", "phone_number", "person"];
    let spans = model
        .extract_ner(text, &labels, 0.3)
        .expect("extraction against the real PII model must succeed");
    assert!(!spans.is_empty(), "the real PII model must find at least one entity in a PII-laden sentence");
}
```

- [ ] **Step 3: Run the gated test**

```bash
GLINER2_PII_MODEL_DIR=/tmp/gliner2-pii-multi \
cargo test -p xberg-gliner-candle --test smoke -- --ignored pii_model_loads_from_bytes_and_extracts_entities
```

Expected: PASS, with at least one extracted span printed via a failed-assertion message if it doesn't. If `from_bytes` returns an `Err` (tensor mismatch, shape error, etc.), STOP and report BLOCKED with the exact error — this is exactly the risk this task exists to catch; do not attempt speculative fixes to the model files or the encoder without understanding the root cause first.

- [ ] **Step 4: Commit**

```bash
git add crates/xberg-gliner-candle/tests/smoke.rs
git commit -m "test(gliner-candle): validate fastino PII model loads via from_bytes"
```

---

### Task 11: Wire the Candle PII backend into `xberg-wasm`'s NER fallback

**Files:**
- Modify: `crates/xberg-wasm/src/bridge/ner.rs`
- Modify: `crates/xberg-wasm/Cargo.toml` (if `xberg-gliner-candle` is not already a direct dependency — check first)

**Interfaces:**
- Consumes: `xberg::text::ner::candle::CandleBackend::from_bytes(safetensors: &[u8], tokenizer_json: &[u8], encoder_config_json: &[u8]) -> xberg::Result<Self>` (existing, reachable via `xberg-wasm`'s existing `xberg` dependency with the `wasm-target` feature set, which already implies `ner-candle-wasm`); `xberg::text::ner::NerBackend::detect(&self, text: &str, categories: &[EntityCategory]) -> xberg::Result<Vec<Entity>>` (existing trait method `CandleBackend` implements).
- Produces: a new `#[wasm_bindgen(js_name = "initCandleNer")]` free function JS calls once to supply model bytes; `fallback_ner()` becomes a real call instead of a stub.

- [ ] **Step 1: Check whether `xberg-gliner-candle`'s `CandleBackend` type is directly importable**

```bash
cd crates/xberg-wasm
cargo tree -p xberg-wasm -i xberg-gliner-candle
```

Expected: shows `xberg-gliner-candle` reachable transitively via `xberg` (it's an optional dependency of `xberg`, enabled by the `ner-candle-wasm` feature, which the `wasm-target` aggregate already includes). This confirms `xberg::text::ner::candle::CandleBackend` is the correct import path — no new Cargo.toml dependency needed. If this command shows `xberg-gliner-candle` is NOT reachable, STOP and re-check `crates/xberg-wasm/Cargo.toml`'s `xberg` dependency line's `features` list before proceeding.

- [ ] **Step 2: Implement the wasm-local model cache and wire `fallback_ner()`**

In `crates/xberg-wasm/src/bridge/ner.rs`, replace the entire file's contents with:

```rust
//! NER (Named Entity Recognition) bridge with injected-first dispatch.
//!
//! The WASM engine tries an externally-injected JavaScript object first —
//! `ner(text, categories)`, called positionally to match this file's own
//! `call_injected_ner`. When no injection is provided, it falls back to an
//! in-binary Candle GLiNER2 backend (`crates/xberg-gliner-candle`, via
//! `xberg::text::ner::candle::CandleBackend`), initialized once via
//! `initCandleNer` (JS calls this after downloading the pinned model's
//! three files — see packages/xberg-wasm-runtime's CacheManager). wasm32
//! is single-threaded, so a thread-local cache (not a Mutex-guarded
//! static, unlike the native multi-key CANDLE_BACKEND_CACHE in
//! xberg::text::ner::candle) is sufficient and simpler.

#[cfg(target_arch = "wasm32")]
use js_sys::{Function, Object, Promise, Reflect};
use wasm_bindgen::prelude::*;

use xberg::text::ner::NerBackend;
use xberg::text::ner::candle::CandleBackend;
use xberg::types::entity::{Entity, EntityCategory};

thread_local! {
    // Rc, not a bare CandleBackend: fallback_ner clones the Rc out of the
    // cell and drops the RefCell borrow *before* awaiting detect() below —
    // holding a RefCell borrow across an .await is a footgun (a re-entrant
    // call while the future is suspended would panic on double-borrow).
    static CANDLE_NER: std::cell::RefCell<Option<std::rc::Rc<CandleBackend>>> = const { std::cell::RefCell::new(None) };
}

/// Initialize the in-binary Candle NER fallback from in-memory model bytes.
/// JS calls this once, after downloading the pinned PII model's
/// `model.safetensors`, `tokenizer.json`, and `encoder_config/config.json`.
/// Calling this more than once replaces the previously-loaded model.
#[allow(clippy::missing_errors_doc)]
#[wasm_bindgen(js_name = "initCandleNer")]
pub fn init_candle_ner(safetensors: &[u8], tokenizer_json: &[u8], encoder_config_json: &[u8]) -> Result<(), JsValue> {
    let backend = CandleBackend::from_bytes(safetensors, tokenizer_json, encoder_config_json)
        .map_err(|e| js_from_any(format!("initCandleNer: {e}")))?;
    CANDLE_NER.with(|cell| {
        *cell.borrow_mut() = Some(std::rc::Rc::new(backend));
    });
    Ok(())
}

/// Resolve the best available NER backend for the current request.
///
/// 1. If `injected` is `Some(obj)`, call `obj.ner(text, categories)`.
/// 2. If `injected` is `None`, use the in-binary Candle backend if
///    `initCandleNer` has been called.
/// 3. Otherwise return an error explaining that NER is unavailable.
pub async fn resolve_ner(
    injected: Option<js_sys::Object>,
    text: &str,
    categories: &[EntityCategory],
) -> Result<Vec<Entity>, JsValue> {
    resolve_ner_with_timeout(injected, text, categories, crate::bridge::BRIDGE_TIMEOUT_MS).await
}

/// Like [`resolve_ner`] but with a configurable bridge timeout.
pub async fn resolve_ner_with_timeout(
    injected: Option<js_sys::Object>,
    text: &str,
    categories: &[EntityCategory],
    timeout_ms: u32,
) -> Result<Vec<Entity>, JsValue> {
    match injected {
        Some(obj) => call_injected_ner(obj, text, categories, timeout_ms).await,
        None => fallback_ner(text, categories).await,
    }
}

/// Call the injected JS `ner(text, categories)` method and deserialize the
/// returned promise into a Vec<Entity>.
async fn call_injected_ner(
    obj: Object,
    text: &str,
    categories: &[EntityCategory],
    timeout_ms: u32,
) -> Result<Vec<Entity>, JsValue> {
    let fn_val = Reflect::get(&obj, &JsValue::from_str("ner"))
        .map_err(|e| js_from_any(format!("failed to read 'ner' property: {e:?}")))?;
    let func: Function = fn_val.dyn_into().map_err(|_| {
        js_from_any("injected NER object has no 'ner' function")
    })?;

    let js_text = JsValue::from_str(text);
    let js_cats = js_sys::Array::new();
    for c in categories {
        let cat_str = serde_json::to_value(c)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default();
        js_cats.push(&JsValue::from_str(&cat_str));
    }
    let args = js_sys::Array::of2(&js_text, &js_cats);

    let result = func.apply(&obj, &args)?;
    let promise = Promise::from(result);
    let js_val = crate::bridge::timed_js_future_with_timeout(promise, timeout_ms).await?;

    let entities: Vec<Entity> = serde_wasm_bindgen::from_value(js_val)
        .map_err(|e| js_from_any(format!("failed to deserialize NER result: {e}")))?;
    Ok(entities)
}

/// In-binary NER fallback. Uses the Candle backend initialized via
/// `initCandleNer`, if any; otherwise returns a diagnostic error.
///
/// Clones the `Rc<CandleBackend>` out of the thread-local cell and drops the
/// `RefCell` borrow before awaiting `detect()` — `resolve_ner_with_timeout`
/// (the caller) is already `async`, so this just awaits directly; no
/// blocking executor (pollster et al.) is needed or wasm32-safe here.
async fn fallback_ner(text: &str, categories: &[EntityCategory]) -> Result<Vec<Entity>, JsValue> {
    let backend = CANDLE_NER.with(|cell| cell.borrow().clone());
    match backend {
        Some(backend) => backend
            .detect(text, categories)
            .await
            .map_err(|e| js_from_any(format!("Candle NER inference: {e}"))),
        None => Err(js_from_any(
            "NER unavailable: no injected backend and initCandleNer has not been called",
        )),
    }
}

/// Convert a Display error into a JsValue suitable for propagation.
fn js_from_any(v: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&v.to_string())
}
```

- [ ] **Step 3: Build for wasm32 and run clippy**

```bash
cargo build -p xberg-wasm --target wasm32-unknown-unknown --features wasm-target
cargo clippy -p xberg-wasm --target wasm32-unknown-unknown --features wasm-target -- -D warnings
```

Expected: both succeed with zero errors and zero warnings. If `xberg::text::ner::candle` is not visible (a privacy/module-export error), check `crates/xberg/src/text/ner/mod.rs`'s `pub mod candle;` line is actually reachable from the crate root (`xberg::text::ner::candle`) — it already is per the design spec's verification, but re-confirm against the exact error if this fails.

- [ ] **Step 4: Run the existing engine tests to confirm no regression**

```bash
cargo test -p xberg-wasm --lib
```

Expected: PASS (native-target unit tests; the wasm32-only `fallback_ner`/`init_candle_ner` code paths are exercised by the build+clippy gate above, not by native `cargo test`, matching this crate's existing target-split testing pattern).

- [ ] **Step 5: Commit**

```bash
cargo fmt -p xberg-wasm
git add crates/xberg-wasm/src/bridge/ner.rs crates/xberg-wasm/Cargo.toml
git commit -m "feat(wasm-bridge): wire Candle GLiNER2 as the real in-binary NER fallback"
```

---

### Task 12 (optional): F16 downcast for the Candle encoder on wasm32

**Files:**
- Modify: `crates/xberg-gliner-candle/src/encoder.rs`
- Modify: `crates/xberg-gliner-candle/src/model.rs`

**Interfaces:**
- Consumes: nothing new.
- Produces: `Encoder::from_buffered_safetensors`'s hardcoded `candle_core::DType::F32` becomes a parameter, halving resident memory on wasm32 after loading (does not reduce download size — the source safetensors file is F32-only). This task is independent of Tasks 10-11 and can be skipped or done later without blocking them.

- [ ] **Step 1: Add a `dtype` parameter to `Encoder::from_buffered_safetensors`**

In `crates/xberg-gliner-candle/src/encoder.rs`, change:

```rust
	/// Load the encoder from in-memory safetensors bytes + parsed config
	/// (wasm/no-fs path). Mirrors [`Self::from_safetensors`] but reads the
	/// weights from a buffer instead of mmap'ing a path. `dtype` lets wasm32
	/// callers request `DType::F16` to halve resident memory after loading —
	/// the source safetensors bytes are always F32, so this only affects
	/// in-memory footprint, not download size.
	pub fn from_buffered_safetensors(
		bytes: &[u8],
		config: &DebertaV2Config,
		device: &Device,
		dtype: candle_core::DType,
	) -> crate::Result<Self> {
		let tensors = candle_core::safetensors::load_buffer(bytes, device)
			.map_err(|e| crate::GlinerCandleError::Backend(format!("encoder safetensors load_buffer: {e}")))?;
		let vb = VarBuilder::from_tensors(tensors, dtype, device);
		Self::from_var_builder(vb.pp("encoder"), config)
	}
```

- [ ] **Step 2: Update the one call site in `model.rs`**

In `crates/xberg-gliner-candle/src/model.rs`'s `from_bytes`, change:

```rust
        let encoder = encoder::Encoder::from_buffered_safetensors(safetensors, &config, &device, candle_core::DType::F32)?;
```

Note: this keeps `from_bytes`'s own behavior at F32 (native callers, and the wasm32 default until a caller explicitly opts into F16) — it does not silently change output. A future task in `crates/xberg-wasm`'s `init_candle_ner` (Task 11) could pass `DType::F16` on `#[cfg(target_arch = "wasm32")]` specifically, but that is a separate, explicit decision — not bundled into this task, which only makes the parameter available.

- [ ] **Step 3: Update `heads::AllHeads::from_buffered_safetensors` if it has the same hardcoded F32 (check first)**

```bash
grep -n "DType::F32" crates/xberg-gliner-candle/src/heads/mod.rs
```

If found, apply the same `dtype: candle_core::DType` parameter pattern there for consistency (heads and encoder should use the same dtype — mixing F32 heads with F16 encoder output would require an explicit cast at the boundary, which is out of scope for this optional task; keep both at F32 for now unless doing the full F16 wiring in one pass).

- [ ] **Step 4: Run the crate's existing test suite**

```bash
cargo test -p xberg-gliner-candle --lib
cargo build -p xberg-wasm --target wasm32-unknown-unknown --features wasm-target
```

Expected: PASS — this task changes a function signature but not default behavior (still F32 everywhere existing callers pass it), so no test assertions should need updating. If `xberg-wasm`'s build fails, it means Task 11's call site (`CandleBackend::from_bytes` → `Encoder::from_buffered_safetensors`) needs the new `dtype` argument threaded through — check `crates/xberg/src/text/ner/candle.rs`'s own call site too.

- [ ] **Step 5: Commit**

```bash
cargo fmt -p xberg-gliner-candle
git add crates/xberg-gliner-candle/src/encoder.rs crates/xberg-gliner-candle/src/model.rs
git commit -m "perf(gliner-candle): make encoder load dtype configurable (F16 downcast, opt-in)"
```

---

## Self-Review Notes

- **Spec coverage:** Component 1 (Tasks 1-5), Component 2 (Tasks 6-7), Component 3a Candle wiring (Tasks 10-11, plus optional Task 12), Component 3b regex complement (Task 9), and the independent contract bug fix (Task 8) are all covered. The spec's "Open Questions for the Planning Phase" are resolved: Q1 (FTS5 compiled in) → Task 4's real gate; Q2 (BGE-M3 real download size) → Task 6's script; Q3 (tensor-name compatibility) → Task 10's real load test; Q4 (new `EntityCategory` variants) → deliberately not added in this plan, `Custom(String)` is sufficient (YAGNI, per the spec's own lean).
- **Type consistency check:** `RetrieveOptions`/`RetrieveMode` (Task 2) used identically in `store-node.ts` (Task 3), `store-worker.ts` (Task 4), and `store-browser.ts` (Task 5). `NerInterface.ner`'s new positional signature (Task 8) is consistent between `types.ts`, `ner.ts`, and `ner.test.ts`. `detectPiiWithNer` (Task 9) takes `Entity[]` (this package's shape), not `mcp-server`'s `NerEntity[]` — the adapter is `mergeNerEntities` reading `entity.label`/`entity.score` directly, not a separate mapping function, since `Entity` already has those exact field names (simpler than the spec's original two-step "adapter function" framing — verified during planning that no separate adapter function is needed, `mergeNerEntities` reads `Entity` fields directly).
- **Correctness catch during self-review:** Task 11's first draft used `pollster::block_on` to bridge `CandleBackend::detect`'s async trait signature into a sync `fallback_ner()` — verified via web search that blocking is not a safe/supported operation under `wasm32-unknown-unknown`'s execution model. Fixed by making `fallback_ner` itself `async` (its only caller, `resolve_ner_with_timeout`, is already async) and cloning an `Rc<CandleBackend>` out of the `thread_local!` cell before awaiting, so no `RefCell` borrow is held across the await point. No blocking executor dependency needed at all — this is a strictly simpler fix than the original draft, not just a safer one.

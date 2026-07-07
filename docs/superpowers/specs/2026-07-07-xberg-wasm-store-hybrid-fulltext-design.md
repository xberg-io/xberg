# Xberg WASM Runtime: Hybrid / Full-Text Retrieval — Design Spec

**Date:** 2026-07-07
**Status:** Draft (not yet approved for planning)
**Scope:** Extension of sub-project **C** (`packages/xberg-wasm-runtime`) — adds `RetrieveMode::Hybrid` and `RetrieveMode::FullText` support to the SQLite-backed vector+graph store built by [`2026-07-07-xberg-wasm-sqlite-vec-store-and-perf.md`](../plans/2026-07-07-xberg-wasm-sqlite-vec-store-and-perf.md).
**Depends on:** That plan's Tasks 1-8 (SQLite + `sqlite-vec` + graph store) must be complete and merged before this spec can move to planning — it extends the same schema and dispatcher, not a parallel implementation.
**Not depended on by anything yet:** Plans B/D/E's existing specs describe the engine's injection contract in terms of vector search only; none currently assume hybrid/full-text is available from this package. This is genuinely optional follow-up work, not a blocker for B/D/E.

## Context

`crates/xberg-rag/src/query.rs` defines four `RetrieveMode` variants: `Vector` (default), `FullText`, `Hybrid` (vector + full-text fused), and `Graph`. The plan referenced above implements `Vector` and `Graph` for the browser/Node runtime, matching what `VectorStoreInterface` currently exposes (`query(collection, queryVector, k)` — no text-query parameter, no mode selector). `FullText` and `Hybrid` were explicitly deferred in that plan's self-review notes as "a real, separate scope... deliberately not folded in here to keep this plan bounded." This spec defines what that separate scope actually is.

**Why this matters:** without it, the JS/browser runtime's retrieval capability is permanently a subset of the Rust server backend's — any consumer (D's RAG panel, E's `query_corpus` tool) that wants full-text or hybrid search in the browser/wasm path has no way to get it, even though the underlying storage (SQLite) is the same engine that already supports it server-side via FTS5-equivalent mechanisms.

## What "Full-Text" Means Here

SQLite's `FTS5` extension provides BM25-ranked full-text search over a virtual table. It is **not** the same extension-loading problem `sqlite-vec` has — `FTS5` is compiled into SQLite core via a build flag (`SQLITE_ENABLE_FTS5`), not a separately-loaded extension, and both `better-sqlite3` (Node) and the official `@sqlite.org/sqlite-wasm` build ship with FTS5 already enabled by default. **This means full-text search does NOT require the same "build a custom WASM bundle" work `sqlite-vec` did** — it should be available for free in both the Node and browser backends built by the prerequisite plan, once the schema adds an FTS5 virtual table.

## Schema Extension

Add to `store-schema.ts`'s `SCHEMA_SQL` (append, do not modify existing tables):

```sql
CREATE VIRTUAL TABLE IF NOT EXISTS chunks_fts USING fts5(
  chunk_id UNINDEXED,
  collection UNINDEXED,
  text,
  content='chunks',
  content_rowid='rowid'
);

-- Keep chunks_fts in sync with chunks via triggers (content= means chunks_fts
-- doesn't store its own copy of `text`, it indexes chunks.text directly).
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

This is the standard SQLite-documented "external content" FTS5 pattern (avoids duplicating chunk text into a second table) — triggers keep the index current on every `upsertDocument`/`delete` call without the store's application code needing to know about FTS5 at all.

## Interface Extension

`VectorStoreInterface` (in `types.ts`) needs a new method rather than overloading `query` — `query`'s signature (`queryVector: number[]`) has no way to carry a text query or a mode selector without a breaking change to every existing caller. Add:

```typescript
export type RetrieveMode = "vector" | "fulltext" | "hybrid";

export interface RetrieveOptions {
  mode: RetrieveMode;
  queryText?: string;      // required for "fulltext"/"hybrid"
  queryVector?: number[];  // required for "vector"/"hybrid"
  k: number;
}

export interface VectorStoreInterface {
  // ... existing methods unchanged ...
  retrieve(collection: string, opts: RetrieveOptions): Promise<Array<{ chunkId: string; text: string; score: number }>>;
}
```

`query(collection, queryVector, k)` stays as-is (unchanged signature, unchanged behavior) for backward compatibility with anything already calling it — `retrieve()` is the new, more general entry point. A follow-up cleanup (not this spec's concern) could eventually have `query` become a thin wrapper around `retrieve(collection, { mode: "vector", queryVector, k })`.

## Fusion Strategy for `Hybrid` Mode

This is the one genuinely open design question. `crates/xberg-rag`'s Rust backend has its own fusion implementation (`candidate_multiplier` field in `RetrieveQuery`, per the existing `query.rs` — pull `top_k * multiplier` candidates from each mode, then fuse). The simplest fusion approach that doesn't require re-deriving the Rust implementation's exact algorithm is **Reciprocal Rank Fusion (RRF)**, a well-known, parameter-light method:

```
score(chunk) = Σ over each mode where chunk appears: 1 / (rrf_k + rank_in_that_mode)
```

with `rrf_k = 60` as a conventional default (widely used in IR literature, not xberg-specific — cite this default rather than inventing a new constant). This does NOT need to exactly match `crates/xberg-rag`'s fusion algorithm bit-for-bit — the two are separate storage engines (server SQLite+sqlite-vec vs. browser/Node SQLite+sqlite-vec, per the prerequisite plan) and returning conceptually-similar-quality rankings is the actual requirement, not byte-identical scores. A future planning pass should decide whether exact parity is actually needed (check `crates/xberg-rag`'s current fusion implementation for its precise algorithm before finalizing) or whether RRF is an accepted simplification for the JS runtime specifically.

## Non-Goals

- Matching `crates/xberg-rag`'s `candidate_multiplier`/`group_by_document`/`include_document` query options exactly — this spec only covers `mode` + `queryText`/`queryVector` + `k`. Feature parity beyond that is a later iteration, not blocking.
- Cross-language tokenization tuning (FTS5's default `unicode61` tokenizer is a reasonable default; language-specific stemming/tokenization is out of scope here).
- Reusing the Rust backend's exact RRF/fusion constant if research finds a different value is documented there — flag this as a "verify before implementing" item for whoever writes the plan, not resolved in this spec.

## Testing Strategy

- Real `better-sqlite3` FTS5 queries in Node (FTS5 is compiled in by default — no mocking needed, matching the prerequisite plan's "no mocking the Node-side SQLite layer" constraint).
- Browser/Worker FTS5 behavior: same mocked-harness limitation as the prerequisite plan's Task 5 (`@sqlite.org/sqlite-wasm`'s FTS5 support should be verified as actually compiled into the specific WASM build in use — do not assume without checking, since WASM builds sometimes trim optional SQLite features to reduce binary size).
- A fusion-correctness test: index chunks where one is vector-similar-but-textually-irrelevant and another is textually-exact-but-vector-distant, confirm hybrid mode ranks a chunk that's moderately good on both above either extreme (the classic RRF sanity check).

## Open Questions for the Planning Phase

1. Does `@sqlite.org/sqlite-wasm`'s official build actually include FTS5? (Needs a one-line check before planning: `SELECT * FROM pragma_compile_options() WHERE compile_options LIKE 'ENABLE_FTS5'` against the built WASM module.)
2. Should `retrieve()`'s fusion constant (`rrf_k`) be configurable via `CacheConfig`, or hardcoded? Lean toward hardcoded until a real caller needs to tune it (YAGNI).
3. Does anything in Plan D's browser UI spec's "RAG panel" actually need hybrid mode for v1, or is this pure over-engineering ahead of demand? Check with whoever scopes Plan D's implementation before turning this spec into a plan — it may be reasonable to defer this indefinitely until a concrete consumer asks for it.

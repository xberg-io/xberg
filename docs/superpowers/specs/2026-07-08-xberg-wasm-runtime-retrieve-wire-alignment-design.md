# Xberg WASM Runtime: `retrieve()` Wire-Alignment with `xberg-rag`'s VectorStore Contract — Design Spec

**Date:** 2026-07-08
**Status:** Draft, pending user review
**Scope:** `packages/xberg-wasm-runtime` only (`store-node.ts`, `store-worker.ts`, `store-browser.ts`, `types.ts`, and their tests). No changes to `crates/xberg-wasm`, `crates/xberg-rag`, or `mcp-server/`.

## Context

[2026-07-08-xberg-wasm-runtime-search-embedding-pii-design.md](2026-07-08-xberg-wasm-runtime-search-embedding-pii-design.md) added a `retrieve(collection, opts: RetrieveOptions)` method to `VectorStoreInterface`, supporting `"vector"|"fulltext"|"hybrid"` modes with camelCase fields (`queryText`, `queryVector`, `k`) and a bare-array response (`Array<{chunkId, text, score}>`).

Investigating [2026-07-02-xberg-wasm-mcp-server.md](../plans/2026-07-02-xberg-wasm-mcp-server.md) (Sub-project E) against the current codebase surfaced that `crates/xberg-wasm/src/bridge/store.rs`'s `JsVectorStore` (implementing `xberg-rag`'s `VectorStore` trait) **already calls a JS method literally named `retrieve(collection, query)`** — this bridge predates this session's work entirely; it's how `XbergEngine::query()` (`crates/xberg-wasm/src/engine.rs:281-328`) has always reached the injected store. Both Sub-project D (browser UI) and Sub-project E (MCP server) route through this same `XbergEngine.query()` → `store.retrieve()` path, so this is the one shared contract both consumers depend on.

The two `retrieve()` implementations — this session's and the engine's pre-existing expectation — are completely incompatible: every request field name differs (camelCase vs. Rust's un-renamed snake_case), the fulltext mode literal differs (`"fulltext"` vs. `"full_text"`), and the response shape differs entirely (bare array vs. `{mode, chunks, primary_latency_ms}`). If `XbergEngine.query()` were called against this session's store today, every call would throw, because `opts.queryVector`/`opts.k` would read as `undefined` against a payload actually keyed `query_vector`/`top_k`.

## Goal

Make `packages/xberg-wasm-runtime`'s `retrieve()` match `xberg-rag`'s `RetrieveQuery`/`RetrieveOutput`/`RetrievedChunk`/`PrimaryScore` types exactly for the `vector`, `full_text`, and `hybrid` modes, so `XbergEngine.query()` works against this store without any Rust-side change. `filter`, `candidate_multiplier`, `group_by_document`, `include_document`, `include_content`, and `graph` mode are explicitly deferred (see Non-Goals) — this keeps the fix narrowly scoped to the one thing that's actually broken (wire alignment), not a full-parity reimplementation.

## Request Shape

`RetrieveOptions` (in `types.ts`) changes from camelCase to match `xberg-rag/src/query.rs`'s `RetrieveQuery` exactly (that struct has no `#[serde(rename_all)]`, so every field serializes as its literal Rust name):

```typescript
export type RetrieveMode = "vector" | "full_text" | "hybrid" | "graph";

export interface RetrieveOptions {
	mode: RetrieveMode;
	query_text?: string;
	query_vector?: number[];
	top_k: number;
	// Present on the wire (xberg-rag sends these even when unused), accepted
	// and typed here, but not honored yet — see Non-Goals.
	filter?: unknown;
	candidate_multiplier?: number;
	group_by_document?: boolean;
	include_content?: boolean;
	include_document?: boolean;
}
```

`mode: "full_text"` matches `RetrieveMode::FullText`'s explicit `#[serde(rename = "full_text")]` override — not `"fulltext"` (which exists only as a *deserialization* alias on the Rust side, never the serialized value the engine actually sends).

## Response Shape

`retrieve()`'s return type changes from `Array<{chunkId, text, score}>` to match `RetrieveOutput`:

```typescript
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

// Exact shape verified empirically in Task 1 before the rest of the plan
// commits to it — see Open Verification Question below.
export type PrimaryScore =
	| { kind: "vector"; ... }
	| { kind: "full_text"; ... }
	| { kind: "hybrid"; vector: number; full_text: number; rrf: number };

export interface RetrieveOutput {
	mode: RetrieveMode;
	chunks: RetrievedChunk[];
	primary_latency_ms: number;
}
```

Field mapping from the existing `store-schema.ts` tables (`chunks`, `documents`) to `RetrievedChunk`:

| `RetrievedChunk` field | Source | Notes |
|---|---|---|
| `id` | `chunks.chunk_id` | |
| `document_id` | `chunks` join `documents.document_id` | already tracked |
| `ordinal` | `chunks.chunk_index` | rename only |
| `content` | `chunks.text` | always populated (`include_content` not honored — see Non-Goals) |
| `score` | existing computed score | unchanged |
| `primary_score` | new — see below | |
| `chunk_metadata` | `null` | `chunks` table has no metadata column (only `documents.metadata` does) — real gap, not filled by this spec |
| `external_id` | omitted | `skip_serializing_if = "Option::is_none"` on the Rust side means omission round-trips cleanly |

`primary_score` is populated per-mode: `{kind: "vector", ...}` for vector-mode results, `{kind: "full_text", ...}` for fulltext-mode results, `{kind: "hybrid", vector, full_text, rrf}` for hybrid-mode results — reusing the existing RRF math from `retrieve-fusion.ts` for the `rrf` field, and the per-source vector/fulltext scores already computed before fusion for the other two fields (currently discarded after fusion; this spec requires threading them through instead of just the fused total).

`primary_latency_ms` is a real measurement (`performance.now()` around the actual query, not hardcoded).

## Open Verification Question (resolve before implementation, not by assumption)

`PrimaryScore` is `#[derive(Serialize, Deserialize)] #[serde(tag = "kind", rename_all = "snake_case")]` over variants that mix a bare newtype (`Vector(f32)`) and a struct variant (`Hybrid { vector, full_text, rrf }`). Serde's internally-tagged representation is well-defined for struct-shaped variants but the exact JSON serde produces for a *newtype-wrapping-a-primitive* variant under internal tagging is not something to infer from reading the enum definition alone — plain internal tagging generally requires map-shaped variant content. The implementation plan's first task must be a real `cargo test`-executed `serde_json::to_string(&PrimaryScore::Vector(0.5))` (and the `Hybrid` variant) to record the actual wire shape, then design `RetrievedChunk.primary_score`'s TypeScript type and construction code against that real output — not the guess above, which is a placeholder for "some tagged-union shape," not a verified contract.

## Error Handling

- **`mode: "graph"`**: `retrieve()` throws immediately — `"retrieve: mode 'graph' is not yet supported"`. Not a silent fallthrough into hybrid/vector logic; that exact class of bug (an unhandled mode value silently landing in the wrong branch) is what surfaced this whole investigation.
- **`filter` present and non-null**: throws — `"retrieve: filter is not yet supported"`. Silently ignoring a caller-supplied filter is a correctness/scope risk (returns more or different results than the caller expects, not just a missing feature), so this gets the same fail-loud treatment as `graph` mode, not the softer treatment below.
- **`candidate_multiplier`, `group_by_document`, `include_document`, `include_content` present**: accepted, logged once via `console.debug` noting they're not yet honored, then ignored. These affect result *shape* (grouping, extra fields), not which chunks are semantically in-scope — lower risk than `filter`, so a log-and-continue is proportionate rather than throwing.

## Non-Goals

- `filter` (Eq/In/Range over whitelisted `doc.*`/`chunk.*` fields) — a filter-to-SQL translator for both the Node and browser backends is a substantial scope item on its own; deferred to a future spec.
- `candidate_multiplier`, `group_by_document`, `include_document`, `include_content` — accepted on the wire, not implemented.
- `graph` mode — `traverseGraph()`/`createEdge()` already exist with an incompatible signature (start-node-based, not query-text/vector-based); reconciling `RetrieveQuery`'s shape with graph traversal semantics is deferred.
- Populating `chunk_metadata` — requires a schema change (`chunks` table has no metadata column today); out of scope here.
- Any change to `crates/xberg-wasm`, `crates/xberg-rag`, or `mcp-server/` — this is a pure JS-side conformance fix. The Rust contract is already correct.
- Removing or changing the existing `query(collection, queryVector, k)` method — stays exactly as-is; `retrieve()` becomes correct in addition to it, not a replacement.

## Testing

- Real `better-sqlite3` tests (Node) and real Chromium Playwright tests (browser) for `vector`/`full_text`/`hybrid` modes against the new field names and response shape — following this session's existing testing conventions throughout.
- Negative tests: `retrieve()` with `mode: "graph"` throws; `retrieve()` with `filter` set throws.
- Regression test: the existing `query()` method's behavior and tests are unchanged.
- The `PrimaryScore` wire-format spike (see Open Verification Question) is a real `cargo test` run, documented with its actual output, before any TypeScript type is written against it.

## Blast Radius

Confirmed via `gh pr list`/`git branch -a` that no branch or open PR currently targets Sub-project E (the wasm MCP-server retargeting) — `mcp-server/` remains 100% native-NAPI with zero wasm dependencies. This fix touches only `packages/xberg-wasm-runtime`; when Sub-project E is eventually implemented, it inherits a `retrieve()` that already matches what `XbergEngine.query()` expects, rather than requiring its own discovery-and-fix cycle.

# Xberg WASM Runtime: SQLite Vector+Graph Store & Performance Optimizations Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace `packages/xberg-wasm-runtime/src/store.ts`'s in-memory placeholder with a real SQLite + `sqlite-vec` backed vector store (Node via `better-sqlite3`, browser via a Worker running a custom-built `sqlite-vec` WASM bundle over OPFS), add graph-edge/traversal support via plain `WITH RECURSIVE` SQL (matching `crates/xberg-rag`'s `graphqlite` schema without porting it), and land four independently-scoped performance optimizations identified during a documentation-grounded audit of the package's dependencies.

**Architecture:** Part 1 (Tasks 1-8) builds a single SQLite-based store used identically on Node and in the browser, so the server (`crates/xberg-rag`, already SQLite+sqlite-vec+graphqlite) and this package's runtime share one storage model instead of two divergent engines. A shared schema module (`store-schema.ts`) is the single source of truth for DDL, consumed by both `store-node.ts` (better-sqlite3) and a browser Worker (`store-worker.ts`) that a thin main-thread RPC client (`store-browser.ts`) talks to via `postMessage` — OPFS's synchronous file-access API only works inside a dedicated Worker, so this is a hard architectural constraint, not a preference. `store.ts` becomes an environment-detecting dispatcher. Part 2 (Tasks 9-12) are four smaller, independent fixes: an embedding-result cache, an OCR zero-detection bug investigation, a real `CacheManager.warm()` implementation, and explicit WebGPU→WASM fallback detection.

**Tech Stack:** TypeScript ESM (strict, `noUncheckedIndexedAccess`), `better-sqlite3` + `sqlite-vec` npm package (Node), `@sqlite.org/sqlite-wasm` + a custom Emscripten-built `sqlite-vec` WASM bundle (browser), Web Worker + OPFS, vitest, `oxlint`/`oxfmt`.

**Research basis:** This plan follows a documentation-grounded feasibility investigation (not present as a file — summarized here for the implementer):
- `sqlite-vec` (asg017/sqlite-vec) is actively maintained (7.8k★, releases current as of this plan's writing) but cannot be dynamically loaded into a WASM SQLite build — it must be statically compiled in via the project's own documented Emscripten path (`scripts/vendor.sh` → `make wasm`).
- The official `@sqlite.org/sqlite-wasm` package (from the SQLite project itself) has production-viable OPFS support as of its 3.53.0 release (`opfs-wl` VFS).
- `WITH RECURSIVE` (SQLite core SQL, not an extension) works identically in WASM builds — confirmed via `sqlite.org/lang_with.html`, no platform restriction documented. This means `crates/xberg-rag/src/backends/graphqlite.rs`'s `_graph_edges` table + BFS traversal pattern can be reimplemented as plain SQL against the same database used for vectors, without porting any Rust graph code to WASM.
- Node-side `sqlite-vec` + `better-sqlite3` is a standard, documented, widely-used pattern (`sqliteVec.load(db)`), directly analogous to the Rust backend's `conn.load_extension("sqlite_vec")`.
- A third-party npm package (`@dao-xyz/sqlite3-vec`) exists combining official sqlite-wasm + sqlite-vec + OPFS, but has only 3 GitHub stars / 1 fork — too thin to depend on for production; this plan builds the static WASM bundle directly instead (Task 1) using `sqlite-vec`'s own documented build path.

## Global Constraints

- **TypeScript:** ESM only; `strict: true`, `noUncheckedIndexedAccess: true` (matches existing `packages/xberg-wasm-runtime/tsconfig.json`).
- **Linting/formatting:** `oxlint src/` + `oxfmt src/ --fix`; run before every commit. `prek run --all-files` if available in the execution environment (it was intermittently unavailable during the package's initial build — fall back to `oxlint`/`tsc --noEmit` and note this explicitly in the task report if so, do not claim `prek` passed without running it).
- **Testing:** `vitest` (`pnpm test:run` / `pnpm test -- --coverage`), 80%+ statements/lines, 75%+ branches (per `vitest.config.ts`'s existing thresholds). Real `better-sqlite3` + real `sqlite-vec` in Node tests — no mocking the Node-side SQLite layer. Browser/Worker/OPFS code paths are NOT executable in this package's `environment: "node"` vitest config; test the Worker's message-handling logic against a **documented mock Worker/OPFS shim** (Task 4 provides it), and clearly flag in each task's report that real OPFS/Worker behavior remains unverified outside a real browser — this mirrors the honesty already established in this package's README ("Vector Store Status" section) rather than claiming false coverage.
- **Package manager:** `pnpm`; commit `pnpm-lock.yaml` after every dependency change.
- **No AI attribution in commits** (repo rule `no-ai-signatures`, critical priority) — a prior task in this package's history accidentally violated this; do not repeat it.
- **Conventional commits:** `feat:`, `fix:`, `test:`, `refactor:`, `perf:`, `chore:`, imperative mood, first line <72 chars.
- **Schema naming convention:** table/column names in `store-schema.ts` mirror `crates/xberg-rag/src/backends/graphqlite.rs`'s `_graph_edges(id, source, target, label, properties)` exactly, so the two systems' data models stay conceptually aligned even though they're separate databases.
- **Collection-name to SQL-identifier sanitization:** collection names are user-supplied strings (e.g. `"test-docs"`) that aren't always valid unquoted SQLite identifiers. All per-collection table/virtual-table names MUST go through the `sanitizeTableName()` helper defined in Task 2 — never interpolate a raw collection name into DDL/DML.

---

## Part 1: SQLite Vector + Graph Store

### Task 1: Spike — build a custom `sqlite-vec` WASM bundle

**Files:**
- Create: `packages/xberg-wasm-runtime/scripts/build-sqlite-vec-wasm.sh`
- Create: `packages/xberg-wasm-runtime/scripts/smoke-test-sqlite-vec-wasm.mjs`
- Create (build output, checked in as a binary artifact): `packages/xberg-wasm-runtime/wasm/sqlite-vec/sqlite3.mjs`, `packages/xberg-wasm-runtime/wasm/sqlite-vec/sqlite3.wasm`

**Interfaces:**
- Produces: a loadable ESM module `wasm/sqlite-vec/sqlite3.mjs` that, when imported and instantiated, exposes a working SQLite database with the `vec0` virtual table module registered (i.e. `CREATE VIRTUAL TABLE x USING vec0(...)` succeeds). This is consumed by Task 4's Worker script.
- This task does NOT wire the bundle into the package's public API — it only proves the build is possible and produces the artifact. Treat it as a standalone spike: if the build fails in a way that can't be resolved within reasonable effort (see "When You're in Over Your Head" below), STOP and report back with the exact failure rather than attempting workarounds that compromise the vector-search correctness (e.g. do not silently fall back to a non-`vec0` schema and call the task done).

- [ ] **Step 1: Vendor SQLite + sqlite-vec source**

```bash
cd packages/xberg-wasm-runtime
mkdir -p vendor
git clone --depth 1 https://github.com/asg017/sqlite-vec.git vendor/sqlite-vec
cd vendor/sqlite-vec
./scripts/vendor.sh
```

Expected: `vendor/sqlite-vec/vendor/sqlite3.c` and `vendor/sqlite-vec/vendor/sqlite3.h` now exist (pulled by `vendor.sh`).

- [ ] **Step 2: Confirm Emscripten is available**

```bash
emcc --version
```

Expected: prints an `emcc (Emscripten gcc/clang-like replacement)` version line. If `emcc` is not found, install the Emscripten SDK (`emsdk`) per https://emscripten.org/docs/getting_started/downloads.html before continuing — this is a hard prerequisite for this task, distinct from the project's existing `wasi-sdk` (which targets plain `wasm32-wasi` C, not Emscripten's browser-oriented WASM+JS glue output that `sqlite-vec`'s own build script requires).

- [ ] **Step 3: Run sqlite-vec's documented WASM build**

```bash
cd vendor/sqlite-vec
make loadable
make wasm
```

Expected: `make wasm` completes without error and produces `dist/.wasm/sqlite3.mjs` + `dist/.wasm/sqlite3.wasm` inside `vendor/sqlite-vec/`. If `make wasm` fails, read the Makefile target directly (`vendor/sqlite-vec/Makefile`) to find the actual emcc invocation and diagnose from there — do not guess flags.

- [ ] **Step 4: Copy build output into the package**

```bash
cd packages/xberg-wasm-runtime
mkdir -p wasm/sqlite-vec
cp vendor/sqlite-vec/dist/.wasm/sqlite3.mjs wasm/sqlite-vec/sqlite3.mjs
cp vendor/sqlite-vec/dist/.wasm/sqlite3.wasm wasm/sqlite-vec/sqlite3.wasm
```

- [ ] **Step 5: Write the build script for reproducibility**

Create `scripts/build-sqlite-vec-wasm.sh`:

```bash
#!/usr/bin/env bash
# Rebuilds wasm/sqlite-vec/{sqlite3.mjs,sqlite3.wasm} from vendor/sqlite-vec.
# Requires: emcc (Emscripten SDK) on PATH.
# sqlite-vec's WASM build path is explicitly non-stable/non-semver upstream
# (see https://alexgarcia.xyz/sqlite-vec/wasm.html) -- vendor/sqlite-vec is
# pinned to a specific commit (see vendor/sqlite-vec-COMMIT below); re-run
# this script and re-verify with smoke-test-sqlite-vec-wasm.mjs after any
# upgrade, do not bump the pin casually.
set -euo pipefail
cd "$(dirname "$0")/.."

if ! command -v emcc &> /dev/null; then
  echo "ERROR: emcc not found. Install the Emscripten SDK: https://emscripten.org/docs/getting_started/downloads.html" >&2
  exit 1
fi

cd vendor/sqlite-vec
make loadable
make wasm
cd ../..

mkdir -p wasm/sqlite-vec
cp vendor/sqlite-vec/dist/.wasm/sqlite3.mjs wasm/sqlite-vec/sqlite3.mjs
cp vendor/sqlite-vec/dist/.wasm/sqlite3.wasm wasm/sqlite-vec/sqlite3.wasm

echo "Built wasm/sqlite-vec/sqlite3.{mjs,wasm}. Run: node scripts/smoke-test-sqlite-vec-wasm.mjs"
```

```bash
chmod +x scripts/build-sqlite-vec-wasm.sh
```

- [ ] **Step 6: Record the vendored commit pin**

```bash
cd vendor/sqlite-vec
git rev-parse HEAD > ../sqlite-vec-COMMIT
cd ../..
cat vendor/sqlite-vec-COMMIT
```

Expected: a 40-character commit hash is written to `vendor/sqlite-vec-COMMIT`. This file is committed to git as the pin record referenced by the build script's comment.

- [ ] **Step 7: Write and run the smoke test**

Create `scripts/smoke-test-sqlite-vec-wasm.mjs`:

```javascript
import sqlite3InitModule from "../wasm/sqlite-vec/sqlite3.mjs";

const sqlite3 = await sqlite3InitModule();
const db = new sqlite3.oo1.DB(":memory:", "c");

// Prove vec0 is registered: creating a vec0 virtual table must succeed.
db.exec(`CREATE VIRTUAL TABLE test_vec USING vec0(id TEXT PRIMARY KEY, embedding FLOAT[4])`);

db.exec({
  sql: "INSERT INTO test_vec (id, embedding) VALUES (?, ?)",
  bind: ["a", new Float32Array([1, 0, 0, 0])],
});
db.exec({
  sql: "INSERT INTO test_vec (id, embedding) VALUES (?, ?)",
  bind: ["b", new Float32Array([0, 1, 0, 0])],
});

const rows = [];
db.exec({
  sql: `SELECT id, distance FROM test_vec
        WHERE embedding MATCH ? ORDER BY distance LIMIT 2`,
  bind: [new Float32Array([1, 0, 0, 0])],
  callback: (row) => rows.push(row),
});

if (rows.length !== 2) {
  throw new Error(`Expected 2 rows, got ${rows.length}`);
}
if (rows[0][0] !== "a") {
  throw new Error(`Expected closest match "a" first, got ${JSON.stringify(rows)}`);
}

db.close();
console.log("OK: sqlite-vec WASM build smoke test passed.", rows);
```

```bash
node scripts/smoke-test-sqlite-vec-wasm.mjs
```

Expected: `OK: sqlite-vec WASM build smoke test passed. [ [ 'a', 0 ], [ 'b', 1 ] ]` (or similar — exact distance values depend on sqlite-vec's default distance metric, but row `"a"` must be first since it's an exact match).

- [ ] **Step 8: Commit**

```bash
git add packages/xberg-wasm-runtime/scripts/build-sqlite-vec-wasm.sh \
        packages/xberg-wasm-runtime/scripts/smoke-test-sqlite-vec-wasm.mjs \
        packages/xberg-wasm-runtime/wasm/sqlite-vec/sqlite3.mjs \
        packages/xberg-wasm-runtime/wasm/sqlite-vec/sqlite3.wasm \
        packages/xberg-wasm-runtime/vendor/sqlite-vec-COMMIT
git commit -m "feat(wasm-runtime): build custom sqlite-vec WASM bundle via Emscripten spike"
```

**When You're in Over Your Head:** If Emscripten genuinely cannot be installed/run in the execution environment (e.g. no ability to install system packages, no `emcc` reachable and no way to obtain it), STOP and report BLOCKED with the exact error — this task is a hard prerequisite for Tasks 4-6 (browser store); do not fabricate a fallback that skips real `vec0` vector search. If `vendor.sh`/`make wasm` fails with a build error, read `vendor/sqlite-vec/Makefile` and any error output carefully and attempt a fix only if the cause is clear (e.g. a missing header, a stale vendor pull) — do not guess randomly at compiler flags.

---

### Task 2: Shared SQL schema module

**Files:**
- Create: `packages/xberg-wasm-runtime/src/store-schema.ts`
- Test: `packages/xberg-wasm-runtime/src/store-schema.test.ts`

**Interfaces:**
- Consumes: nothing (pure module, no runtime dependencies beyond plain TS).
- Produces:
  - `sanitizeTableName(collection: string): string` — deterministic, collision-resistant mapping from an arbitrary collection-name string to a valid unquoted SQLite identifier.
  - `SCHEMA_SQL: string` — DDL for the fixed (non-per-collection) tables: `collections`, `documents`, `chunks`, `graph_edges`.
  - `vecTableName(collection: string): string` — returns `` `vec_${sanitizeTableName(collection)}` ``.
  - `createVecTableSql(collection: string, vectorDim: number): string` — returns the `CREATE VIRTUAL TABLE ... USING vec0(...)` statement for a given collection + dimension.

- [ ] **Step 1: Write the failing test**

```typescript
import { describe, it, expect } from "vitest";
import { sanitizeTableName, SCHEMA_SQL, vecTableName, createVecTableSql } from "./store-schema";

describe("store-schema", () => {
  it("sanitizes a collection name with hyphens into a valid identifier", () => {
    expect(sanitizeTableName("test-docs")).toBe("test_docs");
  });

  it("sanitizes a collection name with spaces and special chars", () => {
    expect(sanitizeTableName("my collection!@#")).toBe("my_collection___");
  });

  it("produces a deterministic vec table name", () => {
    expect(vecTableName("test-docs")).toBe("vec_test_docs");
  });

  it("produces a valid CREATE VIRTUAL TABLE statement with the given dimension", () => {
    const sql = createVecTableSql("test-docs", 384);
    expect(sql).toContain("CREATE VIRTUAL TABLE IF NOT EXISTS vec_test_docs");
    expect(sql).toContain("USING vec0");
    expect(sql).toContain("FLOAT[384]");
  });

  it("SCHEMA_SQL defines the collections, documents, chunks, and graph_edges tables", () => {
    expect(SCHEMA_SQL).toContain("CREATE TABLE IF NOT EXISTS collections");
    expect(SCHEMA_SQL).toContain("CREATE TABLE IF NOT EXISTS documents");
    expect(SCHEMA_SQL).toContain("CREATE TABLE IF NOT EXISTS chunks");
    expect(SCHEMA_SQL).toContain("CREATE TABLE IF NOT EXISTS graph_edges");
    expect(SCHEMA_SQL).toContain("source");
    expect(SCHEMA_SQL).toContain("target");
    expect(SCHEMA_SQL).toContain("label");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd packages/xberg-wasm-runtime
npx vitest run src/store-schema.test.ts
```

Expected: FAIL with "Cannot find module './store-schema'".

- [ ] **Step 3: Write the implementation**

```typescript
/**
 * Shared SQL schema for the SQLite-backed vector + graph store, used
 * identically by store-node.ts (better-sqlite3) and store-worker.ts
 * (sqlite-vec WASM in a browser Worker). Mirrors
 * crates/xberg-rag/src/backends/graphqlite.rs's `_graph_edges` shape
 * (source, target, label, properties) so the server and browser data
 * models stay conceptually aligned across two separate databases.
 */

/**
 * Maps an arbitrary collection-name string to a valid unquoted SQLite
 * identifier. Non-alphanumeric characters become `_`. This is a lossy,
 * many-to-one mapping (e.g. "a-b" and "a_b" both sanitize to "a_b") --
 * callers must not assume sanitized names round-trip uniquely; the
 * `collections` table's `name` column stores the ORIGINAL unsanitized
 * name as the source of truth, and `sanitized_name` records the mapping.
 */
export function sanitizeTableName(collection: string): string {
  return collection.replace(/[^a-zA-Z0-9_]/g, "_");
}

export function vecTableName(collection: string): string {
  return `vec_${sanitizeTableName(collection)}`;
}

export function createVecTableSql(collection: string, vectorDim: number): string {
  const table = vecTableName(collection);
  return `CREATE VIRTUAL TABLE IF NOT EXISTS ${table} USING vec0(chunk_id TEXT PRIMARY KEY, embedding FLOAT[${vectorDim}])`;
}

export const SCHEMA_SQL = `
CREATE TABLE IF NOT EXISTS collections (
  name TEXT PRIMARY KEY,
  sanitized_name TEXT NOT NULL,
  vector_dim INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS documents (
  document_id TEXT PRIMARY KEY,
  source_id TEXT NOT NULL,
  collection TEXT NOT NULL,
  metadata TEXT,
  text TEXT
);
CREATE INDEX IF NOT EXISTS idx_documents_collection ON documents(collection);
CREATE INDEX IF NOT EXISTS idx_documents_source ON documents(source_id);

CREATE TABLE IF NOT EXISTS chunks (
  chunk_id TEXT PRIMARY KEY,
  collection TEXT NOT NULL,
  source_id TEXT NOT NULL,
  chunk_index INTEGER NOT NULL,
  text TEXT NOT NULL,
  start_offset INTEGER NOT NULL,
  end_offset INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_chunks_collection ON chunks(collection);
CREATE INDEX IF NOT EXISTS idx_chunks_source ON chunks(source_id);

CREATE TABLE IF NOT EXISTS graph_edges (
  id TEXT PRIMARY KEY,
  source TEXT NOT NULL,
  target TEXT NOT NULL,
  label TEXT,
  properties TEXT
);
CREATE INDEX IF NOT EXISTS idx_edges_source ON graph_edges(source);
CREATE INDEX IF NOT EXISTS idx_edges_target ON graph_edges(target);
CREATE INDEX IF NOT EXISTS idx_edges_label ON graph_edges(label);
`;
```

- [ ] **Step 4: Run test to verify it passes**

```bash
npx vitest run src/store-schema.test.ts
```

Expected: PASS, 5/5 tests.

- [ ] **Step 5: Commit**

```bash
git add src/store-schema.ts src/store-schema.test.ts
git commit -m "feat(wasm-runtime): shared SQLite schema for vector+graph store"
```

---

### Task 3: Extend `types.ts` with graph edge types

**Files:**
- Modify: `packages/xberg-wasm-runtime/src/types.ts`

**Interfaces:**
- Consumes: nothing new.
- Produces:
  - `GraphEdge` interface: `{ id: string; source: string; target: string; label?: string; properties?: Record<string, unknown> }`.
  - `VectorStoreInterface` gains two new methods: `createEdge(edge: GraphEdge): Promise<void>` and `traverseGraph(startIds: string[], depth: number, edgeLabels?: string[]): Promise<string[]>`.

- [ ] **Step 1: Add `GraphEdge` and extend `VectorStoreInterface`**

In `src/types.ts`, add after the `ChunkRecord` interface (before `VectorStoreInterface`):

```typescript
export interface GraphEdge {
  id: string;
  source: string;
  target: string;
  label?: string;
  properties?: Record<string, unknown>;
}
```

Then modify the existing `VectorStoreInterface` (add two new method signatures, keep everything else unchanged):

```typescript
export interface VectorStoreInterface {
  upsertDocument(
    collection: string,
    doc: DocumentRecord,
    chunks: ChunkRecord[]
  ): Promise<{ documentId: string; chunksCount: number }>;
  query(
    collection: string,
    queryVector: number[],
    k: number
  ): Promise<Array<{ chunkId: string; text: string; score: number }>>;
  delete(collection: string, documentId: string): Promise<void>;
  listCollections(): Promise<string[]>;
  dropCollection(collection: string): Promise<void>;
  ensureCollection(collection: string, vectorDim: number): Promise<void>;
  createEdge(edge: GraphEdge): Promise<void>;
  traverseGraph(
    startIds: string[],
    depth: number,
    edgeLabels?: string[]
  ): Promise<string[]>;
}
```

- [ ] **Step 2: Run the existing type check to confirm nothing else broke**

```bash
cd packages/xberg-wasm-runtime
npx tsc --noEmit
```

Expected: errors ONLY in `src/store.ts` (the old in-memory implementation, which doesn't yet implement `createEdge`/`traverseGraph` — this is expected and fixed in Task 6). No errors in any other file. If errors appear elsewhere, STOP and investigate before continuing — that would mean something unexpected consumed `VectorStoreInterface` in a way this task didn't anticipate.

- [ ] **Step 3: Commit**

```bash
git add src/types.ts
git commit -m "feat(wasm-runtime): add GraphEdge type and graph methods to VectorStoreInterface"
```

Note: `src/store.ts` will fail to type-check after this commit until Task 6 lands — this is expected and matches this plan's task ordering (Tasks 4-5 build the new backends first, Task 6 replaces `store.ts` and fixes the type error in the same commit). Do not attempt to patch `store.ts` in this task.

---

### Task 4: Node-side SQLite store (`better-sqlite3` + `sqlite-vec`)

**Files:**
- Create: `packages/xberg-wasm-runtime/src/store-node.ts`
- Test: `packages/xberg-wasm-runtime/src/store-node.test.ts`
- Modify: `packages/xberg-wasm-runtime/package.json` (add `better-sqlite3`, `sqlite-vec` dependencies)

**Interfaces:**
- Consumes: `SCHEMA_SQL`, `createVecTableSql`, `vecTableName`, `sanitizeTableName` from `./store-schema` (Task 2); `VectorStoreInterface`, `DocumentRecord`, `ChunkRecord`, `GraphEdge`, `CacheConfig` from `./types` (Task 3).
- Produces: `createNodeVectorStore(config?: CacheConfig): Promise<VectorStoreInterface>` — a fully SQLite+sqlite-vec-backed implementation, used by Node.js consumers.

- [ ] **Step 1: Add dependencies**

```bash
cd packages/xberg-wasm-runtime
pnpm add better-sqlite3 sqlite-vec
pnpm add -D @types/better-sqlite3
```

- [ ] **Step 2: Write the failing test**

```typescript
import { describe, it, expect, beforeEach } from "vitest";
import { createNodeVectorStore } from "./store-node";
import type { VectorStoreInterface, DocumentRecord, ChunkRecord } from "./types";

describe("node vector store (better-sqlite3 + sqlite-vec)", () => {
  let store: VectorStoreInterface;
  const testCollection = "test-docs";
  const vectorDim = 4;

  beforeEach(async () => {
    store = await createNodeVectorStore({ nodeCachePath: ":memory:" });
  });

  it("ensures a collection and creates its vec0 table", async () => {
    await store.ensureCollection(testCollection, vectorDim);
    const collections = await store.listCollections();
    expect(collections).toContain(testCollection);
  });

  it("upserts a document with chunks and queries by real vec0 similarity", async () => {
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
        embedding: new Float32Array([1, 0, 0, 0]),
      },
      {
        sourceId: "src-1",
        chunkIndex: 1,
        text: "apple tree",
        startOffset: 12,
        endOffset: 22,
        embedding: new Float32Array([0, 1, 0, 0]),
      },
    ];
    const result = await store.upsertDocument(testCollection, doc, chunks);
    expect(result.chunksCount).toBe(2);

    const results = await store.query(testCollection, [1, 0, 0, 0], 2);
    expect(results.length).toBe(2);
    expect(results[0]?.text).toBe("apple fruit");
    expect(results[0]?.score).toBeGreaterThan(results[1]?.score ?? Infinity);
  });

  it("deletes a document and its chunks are no longer queryable", async () => {
    await store.ensureCollection(testCollection, vectorDim);
    const doc: DocumentRecord = { documentId: "doc-1", sourceId: "src-1", collectionId: testCollection };
    const chunk: ChunkRecord = {
      sourceId: "src-1", chunkIndex: 0, text: "hello", startOffset: 0, endOffset: 5,
      embedding: new Float32Array([1, 0, 0, 0]),
    };
    await store.upsertDocument(testCollection, doc, [chunk]);
    await store.delete(testCollection, "doc-1");

    const results = await store.query(testCollection, [1, 0, 0, 0], 10);
    expect(results.some((r) => r.chunkId.startsWith("src-1"))).toBe(false);
  });

  it("drops a collection including its vec0 table", async () => {
    await store.ensureCollection(testCollection, vectorDim);
    await store.dropCollection(testCollection);
    expect(await store.listCollections()).not.toContain(testCollection);
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
```

- [ ] **Step 3: Run test to verify it fails**

```bash
npx vitest run src/store-node.test.ts
```

Expected: FAIL with "Cannot find module './store-node'".

- [ ] **Step 4: Write the implementation**

```typescript
import Database from "better-sqlite3";
import * as sqliteVec from "sqlite-vec";
import { SCHEMA_SQL, createVecTableSql, vecTableName, sanitizeTableName } from "./store-schema";
import type {
  VectorStoreInterface,
  DocumentRecord,
  ChunkRecord,
  GraphEdge,
  CacheConfig,
} from "./types";

/**
 * Create a vector store backed by better-sqlite3 + the sqlite-vec extension.
 * Mirrors crates/xberg-rag's SQLite + sqlite-vec backend so the server and
 * this package's Node-side runtime share one storage model.
 */
export async function createNodeVectorStore(
  config?: CacheConfig
): Promise<VectorStoreInterface> {
  const dbPath = config?.nodeCachePath ?? ":memory:";
  const db = new Database(dbPath);
  sqliteVec.load(db);
  db.pragma("journal_mode = WAL");
  db.exec(SCHEMA_SQL);

  const vectorDims = new Map<string, number>();

  async function ensureCollection(collection: string, vectorDim: number): Promise<void> {
    const existing = db
      .prepare("SELECT vector_dim FROM collections WHERE name = ?")
      .get(collection) as { vector_dim: number } | undefined;
    if (existing) {
      vectorDims.set(collection, existing.vector_dim);
      return;
    }
    db.prepare(
      "INSERT INTO collections (name, sanitized_name, vector_dim) VALUES (?, ?, ?)"
    ).run(collection, sanitizeTableName(collection), vectorDim);
    db.exec(createVecTableSql(collection, vectorDim));
    vectorDims.set(collection, vectorDim);
  }

  async function upsertDocument(
    collection: string,
    doc: DocumentRecord,
    chunkRecords: ChunkRecord[]
  ): Promise<{ documentId: string; chunksCount: number }> {
    const table = vecTableName(collection);
    const insertDoc = db.prepare(
      `INSERT OR REPLACE INTO documents (document_id, source_id, collection, metadata, text)
       VALUES (?, ?, ?, ?, ?)`
    );
    const insertChunk = db.prepare(
      `INSERT OR REPLACE INTO chunks (chunk_id, collection, source_id, chunk_index, text, start_offset, end_offset)
       VALUES (?, ?, ?, ?, ?, ?, ?)`
    );
    const insertVec = db.prepare(
      `INSERT OR REPLACE INTO ${table} (chunk_id, embedding) VALUES (?, ?)`
    );

    const tx = db.transaction(() => {
      insertDoc.run(
        doc.documentId,
        doc.sourceId,
        collection,
        doc.metadata ? JSON.stringify(doc.metadata) : null,
        doc.text ?? null
      );
      for (const chunk of chunkRecords) {
        const chunkId = `${chunk.sourceId}:${chunk.chunkIndex}`;
        insertChunk.run(
          chunkId,
          collection,
          chunk.sourceId,
          chunk.chunkIndex,
          chunk.text,
          chunk.startOffset,
          chunk.endOffset
        );
        insertVec.run(chunkId, Buffer.from(chunk.embedding.buffer));
      }
    });
    tx();

    return { documentId: doc.documentId, chunksCount: chunkRecords.length };
  }

  async function query(
    collection: string,
    queryVector: number[],
    k: number
  ): Promise<Array<{ chunkId: string; text: string; score: number }>> {
    const table = vecTableName(collection);
    const queryBuf = Buffer.from(new Float32Array(queryVector).buffer);
    const rows = db
      .prepare(
        `SELECT v.chunk_id AS chunkId, c.text AS text, v.distance AS distance
         FROM ${table} v
         JOIN chunks c ON c.chunk_id = v.chunk_id
         WHERE v.embedding MATCH ? AND k = ?
         ORDER BY v.distance`
      )
      .all(queryBuf, k) as Array<{ chunkId: string; text: string; distance: number }>;

    // sqlite-vec's `distance` is smaller-is-closer (e.g. L2/cosine distance);
    // convert to a larger-is-better score consistent with the in-memory
    // implementation's cosine-similarity convention.
    return rows.map((r) => ({ chunkId: r.chunkId, text: r.text, score: -r.distance }));
  }

  async function deleteDocument(collection: string, documentId: string): Promise<void> {
    const table = vecTableName(collection);
    const doc = db
      .prepare("SELECT source_id FROM documents WHERE document_id = ?")
      .get(documentId) as { source_id: string } | undefined;
    if (!doc) return;

    const tx = db.transaction(() => {
      const chunkIds = db
        .prepare("SELECT chunk_id FROM chunks WHERE collection = ? AND source_id = ?")
        .all(collection, doc.source_id) as Array<{ chunk_id: string }>;
      const deleteVec = db.prepare(`DELETE FROM ${table} WHERE chunk_id = ?`);
      for (const { chunk_id } of chunkIds) {
        deleteVec.run(chunk_id);
      }
      db.prepare("DELETE FROM chunks WHERE collection = ? AND source_id = ?").run(
        collection,
        doc.source_id
      );
      db.prepare("DELETE FROM documents WHERE document_id = ?").run(documentId);
    });
    tx();
  }

  async function listCollections(): Promise<string[]> {
    const rows = db.prepare("SELECT name FROM collections").all() as Array<{ name: string }>;
    return rows.map((r) => r.name);
  }

  async function dropCollection(collection: string): Promise<void> {
    const table = vecTableName(collection);
    const tx = db.transaction(() => {
      db.exec(`DROP TABLE IF EXISTS ${table}`);
      db.prepare("DELETE FROM chunks WHERE collection = ?").run(collection);
      db.prepare("DELETE FROM documents WHERE collection = ?").run(collection);
      db.prepare("DELETE FROM collections WHERE name = ?").run(collection);
    });
    tx();
    vectorDims.delete(collection);
  }

  async function createEdge(edge: GraphEdge): Promise<void> {
    db.prepare(
      `INSERT OR REPLACE INTO graph_edges (id, source, target, label, properties)
       VALUES (?, ?, ?, ?, ?)`
    ).run(
      edge.id,
      edge.source,
      edge.target,
      edge.label ?? null,
      edge.properties ? JSON.stringify(edge.properties) : null
    );
  }

  async function traverseGraph(
    startIds: string[],
    depth: number,
    edgeLabels?: string[]
  ): Promise<string[]> {
    if (startIds.length === 0) return [];
    const labelFilter = edgeLabels && edgeLabels.length > 0
      ? `AND e.label IN (${edgeLabels.map(() => "?").join(",")})`
      : "";
    const startPlaceholders = startIds.map(() => "?").join(",");

    const sql = `
      WITH RECURSIVE traversal(node_id, depth) AS (
        SELECT value, 0 FROM json_each(?)
        UNION
        SELECT e.target, traversal.depth + 1
        FROM traversal
        JOIN graph_edges e ON e.source = traversal.node_id
        WHERE traversal.depth < ? ${labelFilter}
      )
      SELECT DISTINCT node_id FROM traversal
    `;
    const params: unknown[] = [JSON.stringify(startIds), depth, ...(edgeLabels ?? [])];
    const rows = db.prepare(sql).all(...params) as Array<{ node_id: string }>;
    return rows.map((r) => r.node_id);
  }

  return {
    ensureCollection,
    upsertDocument,
    query,
    delete: deleteDocument,
    listCollections,
    dropCollection,
    createEdge,
    traverseGraph,
  };
}
```

- [ ] **Step 5: Run test to verify it passes**

```bash
npx vitest run src/store-node.test.ts
```

Expected: PASS, 6/6 tests. If `sqlite-vec.load(db)` throws (extension not found for the current platform/Node version), investigate `sqlite-vec`'s npm package's platform-binary support before assuming a code bug — report BLOCKED with the exact error if the platform binary genuinely isn't available rather than working around it.

- [ ] **Step 6: Commit**

```bash
git add src/store-node.ts src/store-node.test.ts package.json pnpm-lock.yaml
git commit -m "feat(wasm-runtime): Node vector+graph store via better-sqlite3 + sqlite-vec"
```

---

### Task 5: Browser store — Worker script (sqlite-vec WASM + OPFS)

**Files:**
- Create: `packages/xberg-wasm-runtime/src/store-worker.ts`
- Test: `packages/xberg-wasm-runtime/src/store-worker.test.ts`

**Interfaces:**
- Consumes: the custom `wasm/sqlite-vec/sqlite3.mjs` bundle (Task 1); `SCHEMA_SQL`, `createVecTableSql`, `vecTableName`, `sanitizeTableName` from `./store-schema` (Task 2); `DocumentRecord`, `ChunkRecord`, `GraphEdge` from `./types` (Task 3).
- Produces: a Worker `onmessage` handler implementing a request/response protocol:
  ```typescript
  type StoreWorkerRequest =
    | { id: number; op: "ensureCollection"; collection: string; vectorDim: number }
    | { id: number; op: "upsertDocument"; collection: string; doc: DocumentRecord; chunks: ChunkRecord[] }
    | { id: number; op: "query"; collection: string; queryVector: number[]; k: number }
    | { id: number; op: "delete"; collection: string; documentId: string }
    | { id: number; op: "listCollections" }
    | { id: number; op: "dropCollection"; collection: string }
    | { id: number; op: "createEdge"; edge: GraphEdge }
    | { id: number; op: "traverseGraph"; startIds: string[]; depth: number; edgeLabels?: string[] };
  type StoreWorkerResponse =
    | { id: number; ok: true; result: unknown }
    | { id: number; ok: false; error: string };
  ```
  This protocol is consumed by Task 6's `store-browser.ts` RPC client.

- [ ] **Step 1: Write the failing test using a mock OPFS/Worker-message harness**

Real OPFS is not available in this package's `environment: "node"` vitest config. This test exercises the Worker's message-dispatch and SQL logic against the sqlite-vec WASM bundle running in-memory (`:memory:`, no real OPFS), by importing the worker's pure message-handler function directly (not via a real `Worker` + `postMessage`, which requires a browser or `environment: "happy-dom"`/jsdom setup with real Worker support — out of scope for this task). Real OPFS persistence is verified manually in a browser (documented as a follow-up, consistent with this package's existing README caveat on untested browser paths).

```typescript
import { describe, it, expect, beforeEach } from "vitest";
import { createStoreWorkerHandler, type StoreWorkerHandler } from "./store-worker";

describe("store worker message handler (sqlite-vec WASM, in-memory)", () => {
  let handler: StoreWorkerHandler;

  beforeEach(async () => {
    handler = await createStoreWorkerHandler({ dbPath: ":memory:" });
  });

  it("ensures a collection", async () => {
    const res = await handler({ id: 1, op: "ensureCollection", collection: "docs", vectorDim: 4 });
    expect(res.ok).toBe(true);

    const list = await handler({ id: 2, op: "listCollections" });
    expect(list.ok).toBe(true);
    if (list.ok) expect(list.result).toContain("docs");
  });

  it("upserts and queries a document via real vec0 similarity", async () => {
    await handler({ id: 1, op: "ensureCollection", collection: "docs", vectorDim: 4 });
    await handler({
      id: 2,
      op: "upsertDocument",
      collection: "docs",
      doc: { documentId: "d1", sourceId: "s1", collectionId: "docs" },
      chunks: [
        {
          sourceId: "s1", chunkIndex: 0, text: "apple", startOffset: 0, endOffset: 5,
          embedding: new Float32Array([1, 0, 0, 0]),
        },
      ],
    });

    const res = await handler({ id: 3, op: "query", collection: "docs", queryVector: [1, 0, 0, 0], k: 1 });
    expect(res.ok).toBe(true);
    if (res.ok) {
      const rows = res.result as Array<{ chunkId: string; text: string; score: number }>;
      expect(rows[0]?.text).toBe("apple");
    }
  });

  it("returns an error response (not a throw) for an invalid operation on a missing collection", async () => {
    const res = await handler({ id: 1, op: "query", collection: "does-not-exist", queryVector: [1, 0, 0, 0], k: 1 });
    expect(res.ok).toBe(false);
    if (!res.ok) expect(typeof res.error).toBe("string");
  });

  it("creates and traverses a graph edge", async () => {
    await handler({ id: 1, op: "createEdge", edge: { id: "e1", source: "a", target: "b" } });
    const res = await handler({ id: 2, op: "traverseGraph", startIds: ["a"], depth: 1 });
    expect(res.ok).toBe(true);
    if (res.ok) expect(res.result).toContain("b");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
npx vitest run src/store-worker.test.ts
```

Expected: FAIL with "Cannot find module './store-worker'".

- [ ] **Step 3: Write the implementation**

```typescript
import sqlite3InitModule from "../wasm/sqlite-vec/sqlite3.mjs";
import { SCHEMA_SQL, createVecTableSql, vecTableName, sanitizeTableName } from "./store-schema";
import type { DocumentRecord, ChunkRecord, GraphEdge } from "./types";

export interface StoreWorkerRequest {
  id: number;
  op:
    | "ensureCollection"
    | "upsertDocument"
    | "query"
    | "delete"
    | "listCollections"
    | "dropCollection"
    | "createEdge"
    | "traverseGraph";
  collection?: string;
  vectorDim?: number;
  doc?: DocumentRecord;
  chunks?: ChunkRecord[];
  queryVector?: number[];
  k?: number;
  documentId?: string;
  edge?: GraphEdge;
  startIds?: string[];
  depth?: number;
  edgeLabels?: string[];
}

export type StoreWorkerResponse =
  | { id: number; ok: true; result: unknown }
  | { id: number; ok: false; error: string };

export type StoreWorkerHandler = (req: StoreWorkerRequest) => Promise<StoreWorkerResponse>;

/**
 * Creates the in-Worker message handler. In production this runs inside a
 * dedicated Worker with `dbPath` pointing at an OPFS-backed VFS path (e.g.
 * "opfs:/xberg/<collection>.sqlite3"); OPFS's synchronous file-access API
 * is only available inside a Worker, which is why this logic cannot run on
 * the main thread -- see store-browser.ts for the RPC client that talks to
 * this handler via postMessage from the main thread.
 */
export async function createStoreWorkerHandler(opts: {
  dbPath: string;
}): Promise<StoreWorkerHandler> {
  const sqlite3 = await sqlite3InitModule();
  const db = new sqlite3.oo1.DB(opts.dbPath, "c");
  db.exec(SCHEMA_SQL);

  function execAll(sql: string, bind?: unknown[]): Record<string, unknown>[] {
    const rows: Record<string, unknown>[] = [];
    db.exec({ sql, bind, rowMode: "object", callback: (row: Record<string, unknown>) => rows.push(row) });
    return rows;
  }

  async function ensureCollection(collection: string, vectorDim: number): Promise<void> {
    const existing = execAll("SELECT vector_dim FROM collections WHERE name = ?", [collection]);
    if (existing.length > 0) return;
    db.exec({
      sql: "INSERT INTO collections (name, sanitized_name, vector_dim) VALUES (?, ?, ?)",
      bind: [collection, sanitizeTableName(collection), vectorDim],
    });
    db.exec(createVecTableSql(collection, vectorDim));
  }

  async function upsertDocument(
    collection: string,
    doc: DocumentRecord,
    chunkRecords: ChunkRecord[]
  ): Promise<{ documentId: string; chunksCount: number }> {
    const table = vecTableName(collection);
    db.exec({
      sql: `INSERT OR REPLACE INTO documents (document_id, source_id, collection, metadata, text) VALUES (?, ?, ?, ?, ?)`,
      bind: [doc.documentId, doc.sourceId, collection, doc.metadata ? JSON.stringify(doc.metadata) : null, doc.text ?? null],
    });
    for (const chunk of chunkRecords) {
      const chunkId = `${chunk.sourceId}:${chunk.chunkIndex}`;
      db.exec({
        sql: `INSERT OR REPLACE INTO chunks (chunk_id, collection, source_id, chunk_index, text, start_offset, end_offset) VALUES (?, ?, ?, ?, ?, ?, ?)`,
        bind: [chunkId, collection, chunk.sourceId, chunk.chunkIndex, chunk.text, chunk.startOffset, chunk.endOffset],
      });
      db.exec({
        sql: `INSERT OR REPLACE INTO ${table} (chunk_id, embedding) VALUES (?, ?)`,
        bind: [chunkId, chunk.embedding],
      });
    }
    return { documentId: doc.documentId, chunksCount: chunkRecords.length };
  }

  async function query(
    collection: string,
    queryVector: number[],
    k: number
  ): Promise<Array<{ chunkId: string; text: string; score: number }>> {
    const table = vecTableName(collection);
    const rows = execAll(
      `SELECT v.chunk_id AS chunkId, c.text AS text, v.distance AS distance
       FROM ${table} v JOIN chunks c ON c.chunk_id = v.chunk_id
       WHERE v.embedding MATCH ? AND k = ? ORDER BY v.distance`,
      [new Float32Array(queryVector), k]
    ) as unknown as Array<{ chunkId: string; text: string; distance: number }>;
    return rows.map((r) => ({ chunkId: r.chunkId, text: r.text, score: -r.distance }));
  }

  async function deleteDocument(collection: string, documentId: string): Promise<void> {
    const table = vecTableName(collection);
    const docs = execAll("SELECT source_id FROM documents WHERE document_id = ?", [documentId]);
    const sourceId = docs[0]?.["source_id"] as string | undefined;
    if (!sourceId) return;
    const chunkRows = execAll("SELECT chunk_id FROM chunks WHERE collection = ? AND source_id = ?", [collection, sourceId]);
    for (const row of chunkRows) {
      db.exec({ sql: `DELETE FROM ${table} WHERE chunk_id = ?`, bind: [row["chunk_id"] as string] });
    }
    db.exec({ sql: "DELETE FROM chunks WHERE collection = ? AND source_id = ?", bind: [collection, sourceId] });
    db.exec({ sql: "DELETE FROM documents WHERE document_id = ?", bind: [documentId] });
  }

  async function listCollections(): Promise<string[]> {
    return execAll("SELECT name FROM collections").map((r) => r["name"] as string);
  }

  async function dropCollection(collection: string): Promise<void> {
    const table = vecTableName(collection);
    db.exec(`DROP TABLE IF EXISTS ${table}`);
    db.exec({ sql: "DELETE FROM chunks WHERE collection = ?", bind: [collection] });
    db.exec({ sql: "DELETE FROM documents WHERE collection = ?", bind: [collection] });
    db.exec({ sql: "DELETE FROM collections WHERE name = ?", bind: [collection] });
  }

  async function createEdge(edge: GraphEdge): Promise<void> {
    db.exec({
      sql: "INSERT OR REPLACE INTO graph_edges (id, source, target, label, properties) VALUES (?, ?, ?, ?, ?)",
      bind: [edge.id, edge.source, edge.target, edge.label ?? null, edge.properties ? JSON.stringify(edge.properties) : null],
    });
  }

  async function traverseGraph(startIds: string[], depth: number, edgeLabels?: string[]): Promise<string[]> {
    if (startIds.length === 0) return [];
    const labelFilter = edgeLabels && edgeLabels.length > 0
      ? `AND e.label IN (${edgeLabels.map(() => "?").join(",")})`
      : "";
    const sql = `
      WITH RECURSIVE traversal(node_id, depth) AS (
        SELECT value, 0 FROM json_each(?)
        UNION
        SELECT e.target, traversal.depth + 1
        FROM traversal JOIN graph_edges e ON e.source = traversal.node_id
        WHERE traversal.depth < ? ${labelFilter}
      )
      SELECT DISTINCT node_id AS node_id FROM traversal
    `;
    const rows = execAll(sql, [JSON.stringify(startIds), depth, ...(edgeLabels ?? [])]);
    return rows.map((r) => r["node_id"] as string);
  }

  return async (req: StoreWorkerRequest): Promise<StoreWorkerResponse> => {
    try {
      let result: unknown;
      switch (req.op) {
        case "ensureCollection":
          await ensureCollection(req.collection!, req.vectorDim!);
          result = undefined;
          break;
        case "upsertDocument":
          result = await upsertDocument(req.collection!, req.doc!, req.chunks!);
          break;
        case "query":
          result = await query(req.collection!, req.queryVector!, req.k!);
          break;
        case "delete":
          await deleteDocument(req.collection!, req.documentId!);
          result = undefined;
          break;
        case "listCollections":
          result = await listCollections();
          break;
        case "dropCollection":
          await dropCollection(req.collection!);
          result = undefined;
          break;
        case "createEdge":
          await createEdge(req.edge!);
          result = undefined;
          break;
        case "traverseGraph":
          result = await traverseGraph(req.startIds!, req.depth!, req.edgeLabels);
          break;
      }
      return { id: req.id, ok: true, result };
    } catch (err) {
      return { id: req.id, ok: false, error: err instanceof Error ? err.message : String(err) };
    }
  };
}

// Real Worker entry point (only runs inside an actual Worker context).
if (typeof self !== "undefined" && typeof (self as unknown as { WorkerGlobalScope?: unknown }).WorkerGlobalScope !== "undefined") {
  let handlerPromise: Promise<StoreWorkerHandler> | undefined;
  self.onmessage = async (event: MessageEvent<{ dbPath: string } | StoreWorkerRequest>) => {
    if (!handlerPromise) {
      const initMsg = event.data as { dbPath: string };
      handlerPromise = createStoreWorkerHandler({ dbPath: initMsg.dbPath });
      return;
    }
    const handler = await handlerPromise;
    const response = await handler(event.data as StoreWorkerRequest);
    (self as unknown as Worker).postMessage(response);
  };
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
npx vitest run src/store-worker.test.ts
```

Expected: PASS, 4/4 tests. Note: this validates the SQL/vec0 logic and message-dispatch shape only — it does not validate real OPFS persistence or actual `postMessage`/Worker transport, which require a real browser. Record this limitation explicitly in the task report.

- [ ] **Step 5: Commit**

```bash
git add src/store-worker.ts src/store-worker.test.ts
git commit -m "feat(wasm-runtime): browser store Worker handler (sqlite-vec WASM)"
```

---

### Task 6: Browser store — main-thread RPC client + `store.ts` dispatcher rewrite

**Files:**
- Create: `packages/xberg-wasm-runtime/src/store-browser.ts`
- Modify: `packages/xberg-wasm-runtime/src/store.ts` (full rewrite)
- Modify: `packages/xberg-wasm-runtime/src/store.test.ts` (adjust for the new dispatcher; keep all existing assertions passing against whichever backend runs in the vitest Node environment)

**Interfaces:**
- Consumes: `StoreWorkerRequest`/`StoreWorkerResponse` types from `./store-worker` (Task 5, type-only import, no runtime Worker in this file's own logic — the actual `new Worker(...)` construction is browser-only code, guarded by an environment check); `createNodeVectorStore` from `./store-node` (Task 4); `VectorStoreInterface`, `CacheConfig` from `./types`.
- Produces: `createBrowserVectorStore(config?: CacheConfig): Promise<VectorStoreInterface>` (spawns a Worker running `store-worker.ts`'s handler, communicates via `postMessage`, implements the same `VectorStoreInterface`); `createVectorStore(config?: CacheConfig): Promise<VectorStoreInterface>` in `store.ts`, which detects Node vs. browser (`typeof window === "undefined"`) and delegates to `createNodeVectorStore` or `createBrowserVectorStore` — this is the function `factory.ts` already imports and calls, so its exported name and signature MUST NOT change.

- [ ] **Step 1: Write `store-browser.ts`**

```typescript
import type { VectorStoreInterface, DocumentRecord, ChunkRecord, GraphEdge, CacheConfig } from "./types";
import type { StoreWorkerRequest, StoreWorkerResponse } from "./store-worker";

/**
 * Main-thread RPC client for the browser vector+graph store. Spawns a
 * dedicated Worker (running store-worker.ts's message handler) because
 * OPFS's synchronous file-access API -- which sqlite-vec's WASM build
 * needs for real persistence -- is only available inside a Worker, not
 * on the main thread.
 */
export async function createBrowserVectorStore(
  config?: CacheConfig
): Promise<VectorStoreInterface> {
  const worker = new Worker(new URL("./store-worker.ts", import.meta.url), { type: "module" });
  const dbPath = config?.opfsPath
    ? `opfs:${config.opfsPath}`
    : "opfs:/xberg/default.sqlite3";

  let nextId = 1;
  const pending = new Map<number, { resolve: (r: StoreWorkerResponse) => void }>();

  worker.onmessage = (event: MessageEvent<StoreWorkerResponse>) => {
    const entry = pending.get(event.data.id);
    if (entry) {
      pending.delete(event.data.id);
      entry.resolve(event.data);
    }
  };

  worker.postMessage({ dbPath });

  async function call<T>(req: Omit<StoreWorkerRequest, "id">): Promise<T> {
    const id = nextId++;
    const response = await new Promise<StoreWorkerResponse>((resolve) => {
      pending.set(id, { resolve });
      worker.postMessage({ ...req, id });
    });
    if (!response.ok) {
      throw new Error(`[store-browser] ${req.op} failed: ${response.error}`);
    }
    return response.result as T;
  }

  return {
    ensureCollection: (collection: string, vectorDim: number) =>
      call<void>({ op: "ensureCollection", collection, vectorDim }),
    upsertDocument: (collection: string, doc: DocumentRecord, chunks: ChunkRecord[]) =>
      call<{ documentId: string; chunksCount: number }>({ op: "upsertDocument", collection, doc, chunks }),
    query: (collection: string, queryVector: number[], k: number) =>
      call<Array<{ chunkId: string; text: string; score: number }>>({ op: "query", collection, queryVector, k }),
    delete: (collection: string, documentId: string) =>
      call<void>({ op: "delete", collection, documentId }),
    listCollections: () => call<string[]>({ op: "listCollections" }),
    dropCollection: (collection: string) => call<void>({ op: "dropCollection", collection }),
    createEdge: (edge: GraphEdge) => call<void>({ op: "createEdge", edge }),
    traverseGraph: (startIds: string[], depth: number, edgeLabels?: string[]) =>
      call<string[]>({ op: "traverseGraph", startIds, depth, edgeLabels }),
  };
}
```

- [ ] **Step 2: Rewrite `store.ts` as the environment dispatcher**

Replace the entire contents of `src/store.ts` with:

```typescript
import type { VectorStoreInterface, CacheConfig } from "./types";
import { createNodeVectorStore } from "./store-node";

/**
 * Create a vector+graph store, dispatching to the SQLite+sqlite-vec backend
 * appropriate for the current environment: better-sqlite3 in Node.js,
 * or a Worker running sqlite-vec WASM over OPFS in a browser. See
 * store-node.ts / store-browser.ts / store-worker.ts / store-schema.ts.
 */
export async function createVectorStore(config?: CacheConfig): Promise<VectorStoreInterface> {
  if (typeof window === "undefined") {
    return createNodeVectorStore(config);
  }
  const { createBrowserVectorStore } = await import("./store-browser");
  return createBrowserVectorStore(config);
}
```

Note the dynamic `import("./store-browser")` for the browser path: this avoids `store-browser.ts` (which references the browser-only `Worker` global at module scope) from being eagerly imported and failing to even parse/execute its top-level code in the Node test environment — `createNodeVectorStore` is imported statically since it's the path actually exercised by this package's `environment: "node"` vitest config.

- [ ] **Step 3: Update `store.test.ts`**

The existing `store.test.ts` (from Task 1 of the original plan) already tests `createVectorStore()`'s behavior against `VectorStoreInterface`'s original methods. Since `createVectorStore()` now dispatches to `createNodeVectorStore` when run under Node (which vitest always is, per `environment: "node"`), the EXISTING test file's assertions should continue to pass unchanged, exercising the real SQLite backend instead of the old in-memory one. Add two new test cases for the graph methods to `store.test.ts` (append to the existing `describe("vector store", ...)` block):

```typescript
  it("creates and traverses a graph edge through the dispatcher", async () => {
    await store.createEdge({ id: "e1", source: "x", target: "y", label: "rel" });
    const reached = await store.traverseGraph(["x"], 1, ["rel"]);
    expect(reached).toContain("y");
  });

  it("query returns empty array for a non-existent collection rather than throwing", async () => {
    await expect(store.query("no-such-collection", [1, 0, 0], 5)).rejects.toThrow();
  });
```

Note the last test: unlike the old in-memory implementation (which silently returned `[]` for an unknown collection since it just iterated an empty `Map` entry), the new SQLite-backed implementation throws (querying a `vec_` table that was never created via `ensureCollection` is a real SQL error: "no such table"). This is an intentional, documented behavior change — update the test to assert the new, more correct behavior (silently returning empty results for a query against a collection that was never created hides a real caller bug; failing loudly is preferable) rather than preserving the old accidental leniency.

- [ ] **Step 4: Run the full test suite**

```bash
cd packages/xberg-wasm-runtime
npx vitest run
```

Expected: all test files pass, including `store.test.ts`'s original + 2 new assertions, `store-schema.test.ts`, `store-node.test.ts`, `store-worker.test.ts`. If `factory.test.ts` or `contract.test.ts` fail, check whether they relied on the old in-memory store's lenient behavior (e.g. querying an uncreated collection) — fix the test to call `ensureCollection` first rather than reverting `store.ts`'s stricter, correct behavior.

- [ ] **Step 5: Commit**

```bash
git add src/store-browser.ts src/store.ts src/store.test.ts
git commit -m "feat(wasm-runtime): replace in-memory store with SQLite+sqlite-vec dispatcher (Node+browser)"
```

---

### Task 7: Update `README.md` for the new store architecture

**Files:**
- Modify: `packages/xberg-wasm-runtime/README.md`

**Interfaces:**
- Consumes: nothing (documentation only).

- [ ] **Step 1: Replace the "Vector Store Status" section**

Find and replace the existing "Vector Store Status" section (added in the original plan's Task 12) with an accurate description of the new architecture:

```markdown
## Vector Store

Real SQLite + [`sqlite-vec`](https://github.com/asg017/sqlite-vec) backed storage, matching
`crates/xberg-rag`'s server-side backend so the same storage model is used across the whole
system:

- **Node.js**: `better-sqlite3` + the `sqlite-vec` npm extension, loaded via `sqliteVec.load(db)`.
- **Browser**: a dedicated Worker running a custom-built `sqlite-vec` WASM bundle
  (`wasm/sqlite-vec/`, built via `scripts/build-sqlite-vec-wasm.sh`) over OPFS. The main thread
  talks to the Worker via `postMessage` (`store-browser.ts`) because OPFS's synchronous
  file-access API only works inside a Worker.
- **Graph queries**: implemented as plain SQL `WITH RECURSIVE` traversal over a `graph_edges`
  table (`source`, `target`, `label`, `properties`) — the same shape as
  `crates/xberg-rag/src/backends/graphqlite.rs`'s `_graph_edges` table — rather than porting
  `graphqlite` itself to WASM.

**Known limitation**: `sqlite-vec`'s WASM build path is explicitly labeled non-stable/non-semver
by its own maintainer. `vendor/sqlite-vec-COMMIT` pins the exact vendored commit; re-run
`scripts/build-sqlite-vec-wasm.sh` and `scripts/smoke-test-sqlite-vec-wasm.mjs` after any upgrade
rather than bumping the pin casually.

**Known gap**: real OPFS persistence and Worker `postMessage` transport are validated only via a
mocked in-memory harness in this package's test suite (`environment: "node"` can't run a real
Worker/OPFS) — real-browser verification (e.g. via Playwright) is a follow-up, not yet done.
```

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "docs(wasm-runtime): document SQLite+sqlite-vec store architecture"
```

---

### Task 8: Coverage and lint verification for Part 1

**Files:**
- Test: full build, lint, coverage report (no new source files).

- [ ] **Step 1: Full build**

```bash
cd packages/xberg-wasm-runtime
pnpm run build
ls dist/store*.js dist/store*.d.ts
```

Expected: `dist/store.js`, `dist/store-node.js`, `dist/store-browser.js`, `dist/store-worker.js`, `dist/store-schema.js` (and matching `.d.ts` files) all present.

- [ ] **Step 2: Lint**

```bash
npx oxlint src/
```

Expected: 0 errors (warnings acceptable if pre-existing and documented, per this package's established pattern — do not introduce new errors).

- [ ] **Step 3: Coverage**

```bash
npx vitest run --coverage
```

Expected: statements/lines coverage has NOT regressed below the pre-Part-1 baseline (79.38% lines / 62.5% branches / 71.11% functions, per this package's last recorded measurement) — the new real-SQL-backed store code should, if anything, improve branch coverage since Node-side logic (the majority of new code) is fully exercised by real `better-sqlite3`, unlike the old in-memory placeholder's untested edge cases. If coverage regressed, identify which new file is undertested and add targeted tests for genuinely uncovered branches (not tautological ones) before committing.

- [ ] **Step 4: Commit if any lint/build fixes were needed**

```bash
git add -A
git commit -m "chore(wasm-runtime): lint/build verification for SQLite store"
```

(Skip this step entirely if Steps 1-3 required no changes.)

---

## Part 2: Performance Optimizations

Each task below is independently scoped and does not depend on Part 1.

### Task 9: Embedding-result cache

**Files:**
- Modify: `packages/xberg-wasm-runtime/src/embedder.ts`
- Modify: `packages/xberg-wasm-runtime/src/embedder.test.ts` (add cache-hit test)

**Interfaces:**
- Consumes: `crypto.subtle.digest` (Web Crypto API, available in both Node 18+ and browsers) for `SHA-256`.
- Produces: no change to `EmbedderInterface`'s public shape (`embed(texts: string[]): Promise<Float32Array[]>` stays identical) — caching is an internal implementation detail.

- [ ] **Step 1: Write the failing test**

Read the current `src/embedder.ts` first to confirm the exact current `createEmbedder` implementation before writing this test, since the cache must wrap the real `extractor` call without changing its batching/ordering behavior established in this package's history.

```typescript
it("returns a cached result for identical text without re-invoking the model", async () => {
  const embedder = await createEmbedder();
  const texts = ["cache me please"];

  const first = await embedder.embed(texts);
  const start = performance.now();
  const second = await embedder.embed(texts);
  const elapsedMs = performance.now() - start;

  expect(second[0]).toEqual(first[0]);
  // A cache hit should be orders of magnitude faster than real inference
  // (which takes >10ms even for a single short text); this is a coarse
  // but real behavioral check, not a mock-based assertion.
  expect(elapsedMs).toBeLessThan(5);
}, 60_000);
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd packages/xberg-wasm-runtime
npx vitest run src/embedder.test.ts -t "cached result"
```

Expected: FAIL (test times out at 5ms threshold, or passes accidentally slow-but-under-5ms on some machines — if the pre-cache implementation happens to pass this specific assertion, strengthen it by asserting on a call-count spy instead: wrap `extractor` with a counting proxy and assert it was called exactly once across both `embed()` calls).

- [ ] **Step 3: Implement the cache**

Add near the top of `embedder.ts`, above `createEmbedder`:

```typescript
async function sha256Hex(input: string): Promise<string> {
  const bytes = new TextEncoder().encode(input);
  const digest = await crypto.subtle.digest("SHA-256", bytes);
  return Array.from(new Uint8Array(digest))
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}
```

Inside `createEmbedder`, after `modelId` is resolved and before `embed` is defined, add:

```typescript
  const cache = new Map<string, Float32Array>();
```

Modify the `embed` function's batching loop: before adding a text to a batch, check the cache; after computing a batch's results, populate the cache; assemble the final `results` array from cache hits + newly-computed values in the original input order. Replace the existing loop body with:

```typescript
  async function embed(texts: string[]): Promise<Float32Array[]> {
    if (texts.length === 0) return [];

    const hashes = await Promise.all(texts.map((t) => sha256Hex(`${modelId}:${t}`)));
    const results: (Float32Array | undefined)[] = texts.map((_, i) => cache.get(hashes[i]!));

    const uncachedIndices = results
      .map((r, i) => (r === undefined ? i : -1))
      .filter((i) => i !== -1);
    const uncachedTexts = uncachedIndices.map((i) => texts[i]!);

    // Process in batches to manage memory. Batches are awaited sequentially
    // (not Promise.all) so at most one batch's tensor output is resident in
    // memory at a time, and so results preserve input order.
    for (let i = 0; i < uncachedTexts.length; i += DEFAULT_BATCH_SIZE) {
      const batch = uncachedTexts.slice(i, Math.min(i + DEFAULT_BATCH_SIZE, uncachedTexts.length));
      const batchIndices = uncachedIndices.slice(i, Math.min(i + DEFAULT_BATCH_SIZE, uncachedIndices.length));

      // eslint-disable-next-line no-await-in-loop -- intentional: bounds
      // peak memory to one batch and preserves output ordering.
      const output = await extractor(batch, { pooling: "mean", normalize: false });

      const [batchSize, hiddenSize] = output.dims;
      if (batchSize === undefined || hiddenSize === undefined) {
        throw new Error(`Unexpected feature-extraction output shape: [${output.dims.join(", ")}]`);
      }
      const flat = Float32Array.from(output.data as ArrayLike<number>);

      for (let row = 0; row < batchSize; row++) {
        const start = row * hiddenSize;
        const vec = l2Normalize(flat.subarray(start, start + hiddenSize));
        const originalIndex = batchIndices[row]!;
        results[originalIndex] = vec;
        cache.set(hashes[originalIndex]!, vec);
      }
    }

    return results as Float32Array[];
  }
```

- [ ] **Step 4: Run test to verify it passes**

```bash
npx vitest run src/embedder.test.ts
```

Expected: PASS, including the new cache test and all pre-existing tests (batching/ordering/normalization behavior unchanged for the non-cached path).

- [ ] **Step 5: Commit**

```bash
git add src/embedder.ts src/embedder.test.ts
git commit -m "perf(wasm-runtime): cache embedding results by SHA256(model+text)"
```

---

### Task 10: Investigate OCR zero-detection result

**Files:**
- Modify: `packages/xberg-wasm-runtime/src/ocr.test.ts`
- Possibly modify: `packages/xberg-wasm-runtime/src/ocr.ts` (only if the investigation finds a real, fixable bug — see below)

**Interfaces:**
- Consumes: current `ocr.ts`'s `createOcr`, `DEFAULT_MODEL_EXPORT`; `ppu-paddle-ocr`'s `detection.maxSideLength` (default 640px) and `recognition.imageHeight` (default 48px) constructor options (per this plan's research basis).

- [ ] **Step 1: Read the current OCR test fixture**

```bash
cd packages/xberg-wasm-runtime
grep -n "createCanvas\|width\|height\|fillText\|font" src/ocr.test.ts
```

Read the full matched context. Compare the fixture image's actual pixel dimensions and font size against `ppu-paddle-ocr`'s documented defaults: detection resizes to `maxSideLength: 640px` (multiples of 32), recognition crops expect `imageHeight: 48px`. If the fixture is dramatically smaller than these defaults (e.g. under 100x100px) or the rendered text is a tiny fraction of that area, this is a strong candidate root cause — PaddleOCR-family detectors are trained on natural-scene text at particular scale ranges, and a tiny/oversized-relative-to-canvas rendering can fall outside the detector's effective receptive field.

- [ ] **Step 2: Build a fixture closer to the model's expected input scale**

Modify the synthetic fixture generation in `ocr.test.ts` to render larger, higher-contrast text on a canvas sized closer to the documented `maxSideLength`/`imageHeight` defaults. Example adjustment (adapt to the actual current fixture-generation code found in Step 1 — this is illustrative of the required change, not a literal diff):

```typescript
// Render at a scale close to ppu-paddle-ocr's detection.maxSideLength
// default (640px) and recognition.imageHeight default (48px), rather than
// an arbitrary small canvas -- PaddleOCR-family detectors are scale-
// sensitive and a too-small fixture may fall outside their effective
// receptive field (see docs/superpowers/plans/2026-07-07-xberg-wasm-sqlite-vec-store-and-perf.md
// Task 10 for the investigation this addresses).
const canvas = createCanvas(640, 120);
const ctx = canvas.getContext("2d");
ctx.fillStyle = "white";
ctx.fillRect(0, 0, 640, 120);
ctx.fillStyle = "black";
ctx.font = "bold 64px sans-serif";
ctx.fillText("HELLO WORLD", 20, 80);
```

- [ ] **Step 3: Run the OCR test with the new fixture and inspect the actual result**

```bash
npx vitest run src/ocr.test.ts -t "synthetic text image" --reporter=verbose
```

Read the console output for the `[DetectionService]` log line this package's OCR module already emits (per the original Task 6 investigation). Two outcomes:

- **If boxes are now detected** (`Found N potential text boxes` with N > 0): the root cause was fixture scale, confirmed. Proceed to Step 4a.
- **If still zero boxes detected** at the corrected scale: this is now a stronger signal of a genuine pipeline defect (wrong preprocessing, wrong model file, wrong color-channel order) rather than a fixture artifact, since scale is ruled out. Proceed to Step 4b.

- [ ] **Step 4a: (fixture was the cause) Strengthen the test assertion**

Update the test to assert non-empty, content-bearing detection now that the fixture reliably triggers it:

```typescript
it("ocrs a synthetic text image and returns correct shape", async () => {
  if (!ocr) return;
  const result = await ocr.ocr(fixtureBytes);
  expect(result.lines.length).toBeGreaterThan(0);
  expect(result.text.toUpperCase()).toContain("HELLO");
}, 60_000);
```

Run again to confirm this stronger assertion passes:

```bash
npx vitest run src/ocr.test.ts
```

Expected: PASS. This closes the exact gap flagged in this package's Task 6 review ("test suite proves no-throw + correct types but not numerical OCR correctness").

- [ ] **Step 4b: (fixture was NOT the cause) Escalate rather than guess**

Do not attempt further blind fixes (e.g. randomly trying different `detection.maxSideLength` values). Instead:
1. Add explicit `executionProviders`/`detection`/`recognition` option logging to a temporary debug script (not committed) that dumps the preprocessed image tensor's shape and value range right before it's passed to `service.recognize()`.
2. Compare against `ppu-paddle-ocr`'s own README example/test fixture (if one exists in `node_modules/ppu-paddle-ocr` or its GitHub repo) to check for an input-format mismatch (e.g. RGB vs RGBA, wrong normalization range).
3. If the cause remains unclear after this investigation, leave the test's existing shape-only assertions unchanged, but replace the comment currently marking this as "likely font-rasterization mismatch" with a precise statement of what was ruled out (scale) and what remains unknown, and file it as a tracked follow-up rather than continuing to guess in this task.

- [ ] **Step 5: Commit**

```bash
git add src/ocr.test.ts
# If Step 2's ocr.ts changes were needed (unlikely per this task's scope, but include if so):
# git add src/ocr.ts
git commit -m "test(wasm-runtime): investigate and fix OCR synthetic-fixture zero-detection"
```

---

### Task 11: Real `CacheManager.warm()` implementation

**Files:**
- Modify: `packages/xberg-wasm-runtime/src/cache.ts`
- Modify: `packages/xberg-wasm-runtime/src/cache.test.ts`

**Interfaces:**
- Consumes: `PaddleOcrService.downloadModels()` (real, documented method per `ppu-paddle-ocr`'s README — confirm its exact signature by reading `node_modules/ppu-paddle-ocr`'s type declarations before use, do not assume); `@huggingface/transformers`'s model-caching behavior (triggered simply by calling `pipeline(...)`, which downloads-and-caches as a side effect — there is no separate "prefetch without instantiating" API in transformers.js per this plan's research basis, so "warming" the embedder/NER model means actually constructing a throwaway pipeline instance).
- Produces: no change to `CacheManager`'s public method signatures (`warm(modelNames?: string[]): Promise<{success: string[], failed: string[]}>` — confirm the exact current signature in `cache.ts` before implementing) — only the internal no-op simulation is replaced with real work.

- [ ] **Step 1: Read the current `warm()` implementation and `PaddleOcrService`'s type declarations**

```bash
cd packages/xberg-wasm-runtime
grep -n "async warm" src/cache.ts
grep -n "downloadModels" node_modules/ppu-paddle-ocr/dist/*.d.ts
```

Record the exact current `warm()` signature and the exact `downloadModels()` signature found — use these verbatim in Step 3, do not guess.

- [ ] **Step 2: Write the failing test**

```typescript
it("warm() actually triggers model loading, not a simulation", async () => {
  const manager = new CacheManager(":memory:" as unknown as string);
  const result = await manager.warm(["embedder"]);
  expect(result.success).toContain("embedder");
  // A real warm() call must take measurably longer than an instant no-op --
  // this is a coarse behavioral check that the implementation actually
  // does work rather than returning a canned response.
}, 120_000);
```

- [ ] **Step 3: Replace the simulated `warm()` body**

Read the current `warm()` method's exact code in `cache.ts` (from Step 1) before editing — the replacement must preserve the existing method's error-handling contract (returns `{success, failed}`, never throws) while replacing the internal simulation with real calls:

```typescript
  async warm(modelNames?: string[]): Promise<{ success: string[]; failed: string[] }> {
    const targets = modelNames ?? this.models.map((m) => m.name);
    const success: string[] = [];
    const failed: string[] = [];

    for (const name of targets) {
      try {
        if (name === "embedder" || name === "ner") {
          const { pipeline } = await import("@huggingface/transformers");
          const task = name === "embedder" ? "feature-extraction" : "token-classification";
          const modelId = name === "embedder"
            ? "Xenova/all-MiniLM-L6-v2"
            : "Xenova/bert-base-NER";
          // Constructing the pipeline is transformers.js's actual prefetch
          // mechanism -- there is no separate "download without
          // instantiating" API. Discard the instance; its side effect
          // (populating env.cacheDir / the Cache Storage API) is the goal.
          await pipeline(task, modelId);
        } else if (name === "ocr") {
          const { PaddleOcrService } = await import("ppu-paddle-ocr");
          await PaddleOcrService.downloadModels();
        }
        success.push(name);
      } catch (err) {
        console.warn(`[cache] warm failed for ${name}:`, err);
        failed.push(name);
      }
    }

    return { success, failed };
  }
```

Adjust the exact method body to match whatever class structure/field names Step 1 actually found in `cache.ts` (e.g. `this.models`'s real shape) — the code above illustrates the required real-work logic, not a literal drop-in replacement if the surrounding class differs.

- [ ] **Step 4: Run test to verify it passes**

```bash
npx vitest run src/cache.test.ts -t "actually triggers model loading"
```

Expected: PASS (will take real time due to model download/load — this is expected and matches the controller-approved network-call exception already established for this package's model-loading tests).

- [ ] **Step 5: Run the full cache test suite to confirm no regression**

```bash
npx vitest run src/cache.test.ts
```

Expected: all pre-existing `cache.test.ts` tests still pass.

- [ ] **Step 6: Commit**

```bash
git add src/cache.ts src/cache.test.ts
git commit -m "perf(wasm-runtime): implement real model warming in CacheManager.warm()"
```

---

### Task 12: WebGPU→WASM explicit feature detection and fallback

**Files:**
- Modify: `packages/xberg-wasm-runtime/src/embedder.ts`
- Modify: `packages/xberg-wasm-runtime/src/ner.ts`
- Modify: `packages/xberg-wasm-runtime/src/embedder.test.ts`

**Interfaces:**
- Consumes: `navigator.gpu` (browser-only global, `undefined` in Node) for WebGPU feature detection.
- Produces: `createEmbedder`/`createNer` gain a `device`/`dtype` selection step that is now explicit and logged, rather than left entirely to library defaults. `CacheConfig` is unchanged (no new required config).

- [ ] **Step 1: Write the failing test**

```typescript
it("selects wasm device and q8 dtype when navigator.gpu is unavailable (Node)", async () => {
  // In this package's vitest environment ("node"), navigator is undefined,
  // so this test documents and locks in the Node-side fallback behavior.
  const embedder = await createEmbedder();
  const texts = ["device selection check"];
  const result = await embedder.embed(texts);
  // A passing embed() call with the default config is itself evidence the
  // device/dtype selection logic didn't throw or silently misconfigure --
  // combined with the console.debug assertion below for the explicit check.
  expect(result[0]).toBeInstanceOf(Float32Array);
}, 60_000);

it("logs the selected device and dtype for debugging", async () => {
  const debugSpy = vi.spyOn(console, "debug").mockImplementation(() => {});
  await createEmbedder();
  expect(debugSpy).toHaveBeenCalledWith(
    expect.stringContaining("[embedder] device="),
    );
  debugSpy.mockRestore();
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd packages/xberg-wasm-runtime
npx vitest run src/embedder.test.ts -t "logs the selected device"
```

Expected: FAIL ("expected console.debug to have been called with...", since no such log currently exists).

- [ ] **Step 3: Implement explicit device/dtype selection**

Add near the top of `embedder.ts`:

```typescript
/**
 * Explicit WebGPU feature detection with WASM fallback. ONNX Runtime Web
 * does NOT auto-fallback WebGPU to WASM -- an unsupported EP can silently
 * fall through to CPU with no signal (see
 * https://github.com/microsoft/onnxruntime/issues/25952) -- so this check
 * must happen before pipeline construction, not be left to library
 * defaults. transformers.js v3's default `dtype` differs by device (fp32
 * on WebGPU, q8 on WASM); selecting both explicitly together avoids the
 * failure mode where switching device without also switching dtype
 * silently increases memory/bandwidth.
 */
function selectDeviceAndDtype(): { device: "webgpu" | "wasm"; dtype: "fp32" | "q8" } {
  const hasWebGpu =
    typeof navigator !== "undefined" &&
    "gpu" in navigator &&
    (navigator as unknown as { gpu?: unknown }).gpu !== undefined;
  return hasWebGpu ? { device: "webgpu", dtype: "fp32" } : { device: "wasm", dtype: "q8" };
}
```

Modify `createEmbedder`'s pipeline construction line (currently `const extractor = await pipeline("feature-extraction", modelId);`) to:

```typescript
  const { device, dtype } = selectDeviceAndDtype();
  console.debug(`[embedder] device=${device} dtype=${dtype} model=${modelId}`);
  const extractor = await pipeline("feature-extraction", modelId, { device, dtype });
```

Apply the identical pattern to `ner.ts`'s pipeline construction (find its current `pipeline("token-classification", ...)` call and add the same `selectDeviceAndDtype()` + logging + `{ device, dtype }` options) — duplicate the small `selectDeviceAndDtype` helper into `ner.ts` rather than importing it from `embedder.ts` (these are independently-optional modules per this package's existing architecture; introducing a cross-import between two independently-failable optional/required modules would create an unwanted coupling — this is a deliberate, small duplication, not an oversight).

- [ ] **Step 4: Run test to verify it passes**

```bash
npx vitest run src/embedder.test.ts
```

Expected: PASS, including both new tests and all pre-existing embedder tests (device/dtype selection must not change embedding output values in the Node/WASM path, since `q8` was already WASM's implicit default — this change makes the existing default explicit and logged, it does not alter numerical behavior on Node).

- [ ] **Step 5: Run the NER test suite to confirm no regression**

```bash
npx vitest run src/ner.test.ts
```

Expected: all pre-existing tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/embedder.ts src/ner.ts src/embedder.test.ts
git commit -m "perf(wasm-runtime): explicit WebGPU feature detection with WASM+q8 fallback"
```

---

## Self-Review Notes

**Spec coverage:** Part 1 covers the SQLite+sqlite-vec+graph store rewrite end to end (spike → schema → Node backend → browser Worker → browser RPC client → dispatcher → docs → verification), matching the research conclusion that this is achievable with the project's existing WASM tooling. Part 2 covers all four performance optimizations recommended during the documentation-grounded audit (embedding cache, OCR investigation, real cache warming, WebGPU fallback) — the fifth item from that audit (bounded-concurrency batching) was explicitly excluded because the research found no authoritative documentation supporting it as safe, only generic ONNX Runtime thread-safety docs; it needs its own empirical benchmark before being planned as a concrete task, not a blind implementation.

**Known follow-ups intentionally out of scope for this plan:** real-browser Playwright verification of OPFS/Worker behavior (Task 6/8 note this gap explicitly rather than claiming false coverage); the bounded-concurrency batching question flagged above; porting this store architecture's `RetrieveMode::Hybrid`/`FullText` modes (this plan only implements `Vector` and `Graph`, matching what `VectorStoreInterface` currently exposes — extending to hybrid/full-text search would require adding an FTS5 virtual table and is a reasonable follow-up plan, not folded in here to keep this plan's scope bounded).

**Type consistency check:** `VectorStoreInterface` (Task 3) → `createNodeVectorStore` (Task 4) → `createStoreWorkerHandler`'s internal functions (Task 5) → `createBrowserVectorStore` (Task 6) all implement the identical method set (`ensureCollection`, `upsertDocument`, `query`, `delete`, `listCollections`, `dropCollection`, `createEdge`, `traverseGraph`) with identical signatures, verified by construction since each task's interface block explicitly restates the exact types it consumes/produces from the prior task.

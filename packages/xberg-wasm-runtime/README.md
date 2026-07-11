# xberg-wasm-runtime

Shared JavaScript/TypeScript runtime layer for the [xberg wasm engine](../../crates/xberg-wasm). Provides injected implementations of embedder, vector store, NER, OCR, and model caching that both the browser UI and MCP server can consume via a single `InjectionDescriptor`.

## Features

- **Embedder**: `@huggingface/transformers` v3 (transformers.js) feature-extraction pipeline over ONNX Runtime. Mean-pooled, L2-normalized (unit-length) vectors, batched 32 texts at a time, processed sequentially to bound peak memory and preserve output order.
- **Vector Store**: real SQLite + [sqlite-vec](https://github.com/asg017/sqlite-vec) backed storage; Node uses `better-sqlite3`, the browser uses a dedicated Worker over OPFS with a vendored `sqlite-vec` WASM bundle. See [Vector Store](#vector-store) below.
- **NER**: `Xenova/bert-base-NER` (an ONNX export of `dslim/bert-base-NER`) via the transformers.js `token-classification` pipeline. Recognizes a fixed label set (`PER`, `ORG`, `LOC`, `MISC`). Optional — `createNer` returns `null` if the model fails to load instead of throwing; the engine consuming this package (`xberg-wasm`), not this package, decides what fallback (if any) to use.
- **OCR**: `ppu-paddle-ocr` v6's real `PaddleOcrService` API (`new PaddleOcrService(options)` → `await service.initialize()` → `await service.recognize(buffer, opts)`), run through `onnxruntime-node`. Optional — `createOcr` returns `null` on load/init failure rather than throwing; any fallback behavior lives in the consuming wasm engine, not here.
- **Model Cache**: `CacheManager` tracks cache location (`~/.cache/xberg` on Node — or `%LOCALAPPDATA%\xberg` on Windows — and a placeholder OPFS path in the browser) and exposes `status()`/`warm()` for pre-download orchestration, mirroring the MCP server's `WarmupManager`.
- **Single-flight documentation**: `SingleFlightGuard` and `SINGLE_FLIGHT_CONSTRAINT` document and (optionally, in development) enforce that calls into one `XbergEngine` instance must be serialized, since the engine holds `&self` across `await` points in its JSPI bridges.

## Installation

```bash
npm install xberg-wasm-runtime
# or with pnpm
pnpm add xberg-wasm-runtime
```

## Quick Start

```typescript
import { createXbergRuntimeFactory } from "xberg-wasm-runtime";
import { XbergEngine } from "xberg-wasm"; // the wasm engine crate's JS bindings

// Build the injection descriptor (embedder + store required; ner/ocr optional).
const injection = await createXbergRuntimeFactory({
  models: {
    embedder: "Xenova/all-MiniLM-L6-v2", // default; can be omitted
    // ner and ocr are optional and default to Xenova/bert-base-NER / ppu-paddle-ocr's V6_SMALL_MODEL
  },
});

// Pass to the engine.
const engine = new XbergEngine(config, injection);

// Calls on one engine instance must be serialized — see Single-Flight Constraint below.
await engine.ingest(doc, "my-collection");
const results = await engine.query("search query", "my-collection", 10);
```

## API

### `createXbergRuntimeFactory(config?: CacheConfig): Promise<InjectionDescriptor>`

Builds a complete injection descriptor for the wasm engine. It constructs the embedder and vector store (required — failures throw) and NER/OCR (optional — failures are logged and omitted). Model factory initialization performs the required downloads before the descriptor is returned. The final descriptor is validated against a zod schema.

**Parameters** (`CacheConfig`, all optional):
- `models.embedder` — transformers.js model ID (default: `"Xenova/all-MiniLM-L6-v2"`)
- `models.ner` — transformers.js token-classification model ID (default: `"Xenova/bert-base-NER"`)
- `models.ocr` — `ppu-paddle-ocr` model preset export name (default: `"V6_SMALL_MODEL"`)
- `nodeCachePath` — Node.js cache directory (default: `~/.cache/xberg`, or `%LOCALAPPDATA%\xberg` on Windows)
- `nodeStorePath` — explicit Node SQLite database filename (default: `<nodeCachePath>/store.sqlite3`)
- `opfsPath` — browser OPFS database path (default: `/xberg/default.sqlite3`; absolute paths only)
- `wasmPaths` — ONNX Runtime Web wasm binary directory, set on `window.ort.env.wasm.wasmPaths` for self-hosting (no CDN)

**Returns:** an `InjectionDescriptor` — `{ embedder, store, ner?, ocr? }` — ready for `new XbergEngine(config, injection)`.

### Component factories

Each runtime component can also be constructed independently:

- `createEmbedder(config?: CacheConfig): Promise<EmbedderInterface>` — throws on failure.
- `createVectorStore(config?: CacheConfig): Promise<VectorStoreInterface>` — throws on failure.
- `createNer(config?: CacheConfig): Promise<NerInterface | null>` — returns `null` on failure, never throws.
- `createOcr(config?: CacheConfig): Promise<OcrInterface | null>` — returns `null` on failure, never throws.
- `new CacheManager(cacheDir?: string)` — `status()` reports cached models and total size; `warm(modelNames?)` pre-downloads; `setWasmPaths(dir)` configures ONNX Runtime Web.

Vector stores own database/Worker resources. Call `await store.close()` when the runtime is no longer needed.

### Single-Flight Constraint

The wasm engine holds `&self` across await points in its JSPI bridges, so it is **not safe for concurrent calls on a single handle**:

```typescript
// WRONG (race condition — both calls touch the store concurrently):
Promise.all([
  engine.ingest(doc, "col"),
  engine.query(q, "col", 10),
]);

// CORRECT:
await engine.ingest(doc, "col");
const results = await engine.query(q, "col", 10);
```

`SingleFlightGuard` (from `async_shim.ts`) can wrap calls during development to throw on overlapping invocations instead of silently corrupting state. If your frontend needs true concurrency, create multiple engine instances rather than sharing one handle.

## Vector Store

Real SQLite + [sqlite-vec](https://github.com/asg017/sqlite-vec) backed storage, matching
`crates/xberg-rag`'s server-side backend so the same storage model is used across the whole
system:

- **Node.js**: `better-sqlite3` + the `sqlite-vec` npm extension, loaded via `sqliteVec.load(db)`.
- **Browser**: a dedicated Worker running a custom-built `sqlite-vec` WASM bundle
  (`wasm/sqlite-vec/`, built via `scripts/build-sqlite-vec-wasm.sh`) over OPFS. The bundle uses
  pinned sqlite-vec source with SQLite 3.53.3. The main thread
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

The browser host must serve COOP `same-origin` and COEP `require-corp` headers. Playwright tests
exercise the real Worker, OPFS persistence across reopen, sqlite-vec queries, collection isolation,
graph traversal, deletion, and collection dropping in Chromium.

## Testing

```bash
pnpm test              # Run tests in watch mode (vitest)
pnpm test:run          # Run tests once
pnpm test:coverage     # Run unit/integration tests with coverage gates
pnpm test:browser      # Run real Chromium Worker/OPFS tests
pnpm test:models       # Cold-download all models into an isolated cache
pnpm setup:models      # Download/setup production models in the configured cache
pnpm build:sqlite-wasm # Rebuild the pinned sqlite-vec browser bundle with Docker
pnpm run build         # Compile to dist/ (tsc)
pnpm run lint          # Check with oxlint
pnpm run format        # Format with oxfmt
```

Tests cover each module plus the factory contract. Node coverage gates require 80% lines/functions
and 75% branches; browser-only storage behavior is covered separately with Playwright.

## Known Limitations

- **Pinned ONNX Runtime versions are load-bearing.** The embedder/NER path (transformers.js) and the OCR path (`ppu-paddle-ocr` via `onnxruntime-node`) both load ONNX Runtime natively. Loading both `onnxruntime-web`/`onnxruntime-node` at mismatched versions in the same process previously caused a native SIGSEGV crash. The workspace `pnpm-workspace.yaml` pins both via `overrides` (`onnxruntime-node: 1.21.0`, `onnxruntime-web: 1.22.0-dev.20250409-89f8206ba4`) to keep them compatible — do not bump either independently without re-verifying that embedder + OCR can be loaded together in the same process.
- Browser OPFS cache paths in `CacheManager` are placeholders; only the Node filesystem path is implemented.

## Model Cache

- **Node**: `~/.cache/xberg` (or `%LOCALAPPDATA%\xberg` on Windows), checked via `CacheManager.status()`.
- **Browser**: reserved OPFS path (`/opfs/xberg`); not yet backed by real storage I/O.

`CacheManager.warm()` initializes the embedding, NER, and OCR factories sequentially. This performs
real model download/setup while bounding peak memory. Use `XBERG_CACHE_DIR` to override the Node
cache used by `pnpm setup:models`; Paddle OCR manages its own upstream cache.

## Dependencies

- `@huggingface/transformers` ^3.0.0 — embedder and NER pipelines
- `onnxruntime-web` ^1.18.0 (workspace-pinned to `1.22.0-dev.20250409-89f8206ba4`) — browser inference backend
- `ppu-paddle-ocr` ^6.0.0 — OCR (`onnxruntime-node`, workspace-pinned to `1.21.0`, is an optional peer dependency of this package)
- `better-sqlite3` + `sqlite-vec` — Node persistent vector store
- `zod` ^3.22.0 — runtime validation of the injection descriptor and cache config

## License

MIT

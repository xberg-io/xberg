# xberg-wasm-runtime

Shared JavaScript/TypeScript runtime layer for the [xberg wasm engine](../../crates/xberg-wasm). Provides injected implementations of embedder, vector store, NER, OCR, and model caching that both the browser UI and MCP server can consume via a single `InjectionDescriptor`.

## Features

- **Embedder**: `@huggingface/transformers` v3 (transformers.js) feature-extraction pipeline over ONNX Runtime. Mean-pooled, L2-normalized (unit-length) vectors, batched 32 texts at a time, processed sequentially to bound peak memory and preserve output order.
- **Vector Store**: in-memory JS implementation with brute-force cosine similarity. This is a placeholder backend — `wa-sqlite`/OPFS and `sqlite-vec` integration are **not yet wired up**; see [Vector Store Status](#vector-store-status) below.
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

Builds a complete injection descriptor for the wasm engine. Initializes the cache manager, kicks off a non-blocking model warmup, then constructs the embedder and vector store (required — failures here throw) and NER/OCR (optional — failures are caught and logged, resulting in `ner`/`ocr` being omitted from the descriptor). The final descriptor is validated against a zod schema before being returned.

**Parameters** (`CacheConfig`, all optional):
- `models.embedder` — transformers.js model ID (default: `"Xenova/all-MiniLM-L6-v2"`)
- `models.ner` — transformers.js token-classification model ID (default: `"Xenova/bert-base-NER"`)
- `models.ocr` — `ppu-paddle-ocr` model preset export name (default: `"V6_SMALL_MODEL"`)
- `nodeCachePath` — Node.js cache directory (default: `~/.cache/xberg`, or `%LOCALAPPDATA%\xberg` on Windows)
- `opfsPath` — Browser OPFS mount point (reserved; browser cache I/O is not yet implemented)
- `wasmPaths` — ONNX Runtime Web wasm binary directory, set on `window.ort.env.wasm.wasmPaths` for self-hosting (no CDN)

**Returns:** an `InjectionDescriptor` — `{ embedder, store, ner?, ocr? }` — ready for `new XbergEngine(config, injection)`.

### Component factories

Each runtime component can also be constructed independently:

- `createEmbedder(config?: CacheConfig): Promise<EmbedderInterface>` — throws on failure.
- `createVectorStore(config?: CacheConfig): Promise<VectorStoreInterface>` — throws on failure.
- `createNer(config?: CacheConfig): Promise<NerInterface | null>` — returns `null` on failure, never throws.
- `createOcr(config?: CacheConfig): Promise<OcrInterface | null>` — returns `null` on failure, never throws.
- `new CacheManager(cacheDir?: string)` — `status()` reports cached models and total size; `warm(modelNames?)` pre-downloads; `setWasmPaths(dir)` configures ONNX Runtime Web.

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

## Vector Store Status

The current `createVectorStore` implementation is an **in-memory JS store** (`Map`-backed) with brute-force cosine similarity search. It is functionally correct and used by the test suite, but:

- No persistence — data does not survive process/tab restart.
- No `wa-sqlite`/OPFS backend for the browser and no `better-sqlite3`/native backend for Node yet.
- No `sqlite-vec` (or other ANN index) integration — search is O(n) per query.

Swapping in a persistent, indexed backend is tracked as future work; the `VectorStoreInterface` contract in `src/types.ts` is designed so the backend can be replaced without changing callers.

## Testing

```bash
pnpm test              # Run tests in watch mode (vitest)
pnpm test:run          # Run tests once
pnpm run build         # Compile to dist/ (tsc)
pnpm run lint          # Check with oxlint
pnpm run format        # Format with oxfmt
```

Tests cover each module (`embedder`, `store`, `ner`, `ocr`, `cache`, `factory`, `async_shim`) plus a `contract.test.ts` that verifies the factory's output satisfies the engine's injection contract. Coverage is high but below the repository's 80%/75% targets in places (embedder/NER/OCR model-loading branches, platform-specific cache paths, and optional-injection fallback branches are difficult to exercise without real model downloads or a browser environment); the gap is in optional-injection and platform-gated code paths, not untested core logic.

## Known Limitations

- **Pinned ONNX Runtime versions are load-bearing.** The embedder/NER path (transformers.js) and the OCR path (`ppu-paddle-ocr` via `onnxruntime-node`) both load ONNX Runtime natively. Loading both `onnxruntime-web`/`onnxruntime-node` at mismatched versions in the same process previously caused a native SIGSEGV crash. The workspace `pnpm-workspace.yaml` pins both via `overrides` (`onnxruntime-node: 1.21.0`, `onnxruntime-web: 1.22.0-dev.20250409-89f8206ba4`) to keep them compatible — do not bump either independently without re-verifying that embedder + OCR can be loaded together in the same process.
- Vector store has no persistence or ANN index yet (see [Vector Store Status](#vector-store-status)).
- Browser OPFS cache paths in `CacheManager` are placeholders; only the Node filesystem path is implemented.
- `CacheManager.warm()` currently logs progress but does not perform a real model download — actual model fetching happens lazily on first `pipeline(...)` / `PaddleOcrService.initialize()` call inside `createEmbedder`/`createNer`/`createOcr`.

## Model Cache

- **Node**: `~/.cache/xberg` (or `%LOCALAPPDATA%\xberg` on Windows), checked via `CacheManager.status()`.
- **Browser**: reserved OPFS path (`/opfs/xberg`); not yet backed by real storage I/O.

Model downloads themselves are handled lazily by `@huggingface/transformers` and `ppu-paddle-ocr` on first pipeline/service initialization, not by `CacheManager.warm()` directly.

## Dependencies

- `@huggingface/transformers` ^3.0.0 — embedder and NER pipelines
- `onnxruntime-web` ^1.18.0 (workspace-pinned to `1.22.0-dev.20250409-89f8206ba4`) — browser inference backend
- `ppu-paddle-ocr` ^6.0.0 — OCR (`onnxruntime-node`, workspace-pinned to `1.21.0`, is an optional peer dependency of this package)
- `wa-sqlite` ^1.0.0 — currently unused pending the persistent vector store backend
- `zod` ^3.22.0 — runtime validation of the injection descriptor and cache config

## License

MIT

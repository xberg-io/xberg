---
priority: high
---

# Local Additions (Fork-Specific)

These paths were added by us on top of the upstream fork. They are not in `jamon8888/xberg`.
When pulling upstream, these paths are safe — upstream will never have conflicting changes here.

## crates/xberg-rag

RAG (Retrieval-Augmented Generation) base layer. Added in commit `b9dc796bf2`.

- `src/backends/memory.rs` — in-memory store (for tests)
- `src/backends/sqlite.rs` — SQLite + vector store (modified: wired graphqlite dispatch)
- `src/backends/graphqlite.rs` — GraphQLite graph backend (added by us)
- `src/query.rs` — retrieval query types (modified: added `Graph` variant to `RetrieveMode`)
- `src/store.rs`, `src/types.rs`, `src/pipeline.rs` — core RAG abstractions

## crates/xberg-rag-node

NAPI-RS Node.js bindings for `xberg-rag`. Added in commit `6b53e3a051`.

- `src/lib.rs` — NAPI exports: `openSqlite`, `embedTexts`, `RagStore` interface
- `index.js` — platform-aware native binding loader
- `index.d.ts` — TypeScript declarations
- `package.json` — npm package manifest (`xberg-rag-node`)

The compiled binary (`xberg_rag_node.win32-x64-msvc.node`) is gitignored.
Build with: `cargo build --release -p xberg-rag-node` then copy the DLL as `.node`.

## mcp-server

TypeScript MCP server wrapping xberg extraction + xberg-rag. 30 tools across 10 categories.

```
mcp-server/
  src/
    index.ts              — entry point (stdio transport)
    store.ts              — RAG store singleton (openSqlite)
    chunker.ts            — text chunking utilities
    warmup.ts             — model pre-download manager
    redaction/
      detect.ts           — PII pattern detection (11 categories)
      redact.ts           — token_replace / mask / hash strategies
      rehydration.ts      — AES-256-GCM encrypted rehydration maps
      output/
        docx.ts / pdf.ts / text.ts / report.ts
    tools/
      extract.ts          — extract_document, extract_batch, list_formats
      collection.ts       — create_collection, get_collection, drop_collection
      query.ts            — query_corpus
      document.ts         — upsert_document, get_document, delete_documents, delete_by_filter
      ingest.ts           — ingest_document, ingest_folder (extract→PII→redact→embed→store)
      pii.ts              — detect_pii, redact_document
      rehydrate.ts        — rehydrate_tokens, list_tokens, rehydrate_document
      reports.ts          — get_ingestion_summary, get_document_report, explain_reports
      stats.ts            — collection_stats, list_collections, export_collection,
                            import_collection, update_metadata, get_audit_log, get_extraction_stats
      cache.ts            — rag_cache_warm, rag_cache_status
    transports/
      stdio.ts            — McpServer over stdin/stdout
      http.ts             — SSE-based HTTP transport (XBERG_MCP_PORT / XBERG_MCP_HOST)
  tests/
    redaction.test.ts     — 15 redaction detection/strategy tests
    tools.test.ts         — 6 module export + encryption round-trip tests
```

Dependencies: `@modelcontextprotocol/sdk`, `@xberg-io/xberg` (file dep), `xberg-rag-node` (file dep), `zod`, `docx`, `pdf-lib`.

## packages/xberg-wasm-runtime

Shared JS/TS runtime layer for the `xberg-wasm` engine's injected dependencies (embedder, vector
store, NER, OCR, model caching). Consumed by both the browser UI and the MCP server. Fork-local,
built via `docs/superpowers/plans/2026-07-02-xberg-wasm-runtime-layer.md` ("Plan C"), 12/12 tasks
complete on branch `worktree-wasm-runtime-layer` (PR #10). Full task-by-task history —
including a real cross-cutting ONNX Runtime version-conflict bug found and fixed, and a
scope-creep regression caught and reverted during a lint-fix round — is in
`.superpowers/sdd/progress.md` (Plan C section).

```
packages/xberg-wasm-runtime/
  src/
    types.ts        — InjectionDescriptor, CacheConfig, and per-component interfaces
    validation.ts    — zod schema, validateInjectionDescriptor
    embedder.ts      — transformers.js v3 feature-extraction pipeline, L2-normalized output
    store.ts         — in-memory JS cosine-similarity vector store (sqlite-vec/wa-sqlite pending)
    ner.ts           — transformers.js token-classification (Xenova/bert-base-NER), optional
    ocr.ts           — ppu-paddle-ocr v6 (PaddleOcrService/.recognize()), optional
    cache.ts         — CacheManager: model cache status/warm/wasm-path config (mirrors MCP WarmupManager)
    async_shim.ts    — SingleFlightGuard, documents the engine's &self-across-await constraint
    factory.ts       — createXbergRuntimeFactory: thin composition over all of the above
    contract.test.ts — exercises the real factory output against each interface's contract
  README.md
```

Optional components (`ner`, `ocr`) return `null` on load/init failure rather than throwing —
`createXbergRuntimeFactory` omits them from the returned descriptor entirely (not a `null`-valued
key, per the zod schema's `.optional()`/non-`.nullable()` fields). The consuming `xberg-wasm`
engine, not this package, is responsible for any native fallback when a component is absent.

**Known constraint — do not casually bump ORT versions.** `pnpm-workspace.yaml`'s `overrides`
pins `onnxruntime-node` to `1.21.0` and `onnxruntime-web` to `1.22.0-dev.20250409-89f8206ba4`,
matching what `@huggingface/transformers` (embedder/NER) already resolves to. `ppu-paddle-ocr@6.x`
peer-resolves to a newer `onnxruntime-web@1.27.0` by default; having both ORT native builds
resident in one Node process causes a native SIGSEGV (API-version mismatch), not a catchable JS
exception. If either package's dependency on `onnxruntime-*` is updated, re-verify both
`embedder.test.ts`/`ner.test.ts` and `ocr.test.ts` still pass together in the same process before
removing or changing the pin.

Coverage as of Task 11: 79.38% lines / 62.5% branches / 71.11% functions / 76.72% statements —
below the repo's 80%/75% bindings targets. The gap is traced to legitimate optional-injection and
platform-gated branches (browser-only OPFS paths, native backend failure catches, OCR
language-model switching) rather than untested business logic; closing it fully was estimated at
~200-300 LOC of mock infrastructure and deferred as documented follow-up.

## Windows Build Notes

- Cargo target dir: `E:/cargo-target` (set in `~/.cargo/config.toml`)
- `xberg-node` requires `libheif` via vcpkg: `VCPKG_ROOT=C:\vcpkg`, triplet `x64-windows-static-md`
- `xberg-rag-node` DLL: copy `E:/cargo-target/release/xberg_rag_node.dll` → `crates/xberg-rag-node/xberg_rag_node.win32-x64-msvc.node`
- `xberg-node` DLL: copy `E:/cargo-target/release/xberg_node.dll` → `crates/xberg-node/xberg-node.win32-x64-msvc.node`

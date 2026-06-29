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

## Windows Build Notes

- Cargo target dir: `E:/cargo-target` (set in `~/.cargo/config.toml`)
- `xberg-node` requires `libheif` via vcpkg: `VCPKG_ROOT=C:\vcpkg`, triplet `x64-windows-static-md`
- `xberg-rag-node` DLL: copy `E:/cargo-target/release/xberg_rag_node.dll` → `crates/xberg-rag-node/xberg_rag_node.win32-x64-msvc.node`
- `xberg-node` DLL: copy `E:/cargo-target/release/xberg_node.dll` → `crates/xberg-node/xberg-node.win32-x64-msvc.node`

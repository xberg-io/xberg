---
priority: high
---

# Crate Structure

Version source of truth: root `Cargo.toml` `[workspace.package] version`.

## Workspace crates (`crates/`)

- `xberg` — core library: extraction engine, MIME detection, plugin system, OCR, chunking, embeddings, API/MCP server
- `xberg-cli` — CLI binary; thin wrapper over core with `cli` feature set
- `xberg-ffi` — C FFI layer (`#[no_mangle] extern "C"`); opaque handles, cbindgen headers; used by Go, Java, C# bindings
- `xberg-node` — NAPI-RS Node.js/TypeScript bindings
- `xberg-py` — PyO3 Python bindings
- `xberg-php` — ext-php-rs PHP bindings
- `xberg-wasm` — wasm-bindgen WASM bindings; uses `wasm-target` feature set
- `xberg-paddle-ocr` — PaddleOCR via ONNX Runtime; not available on WASM or Windows
- `xberg-tesseract` — Rust bindings for Tesseract OCR
- `xberg-rag` — RAG base layer: `RagStore` trait, SQLite + graphqlite backends, `RetrieveMode` (Vector/FullText/Hybrid/Graph), embedding pipeline (fork-local, added by us)
- `xberg-rag-node` — NAPI-RS Node.js bindings for `xberg-rag`; exports `openSqlite`, `embedTexts` (fork-local)

## Out-of-workspace bindings (`packages/`)

- `packages/python/` — PyPI (maturin + PyO3)
- `packages/ruby/` — RubyGems (Magnus); native ext compiled by `rake`
- `packages/php/` — Composer (ext-php-rs)
- `packages/go/` — Go module; cgo over xberg-ffi
- `packages/java/` — Maven; Foreign Function & Memory API over xberg-ffi
- `packages/csharp/` — NuGet; P/Invoke over xberg-ffi
- `packages/elixir/` — Hex; Rustler NIF (workspace member at `packages/elixir/native/xberg_rustler`)
- `packages/r/` — CRAN; extendr (excluded from workspace)

## Tools (`tools/`)

- `tools/e2e-generator` — reads JSON fixtures, generates runnable test suites per language into `e2e/`
- `tools/benchmark-harness` — criterion-based benchmark runner

## Fork-local TypeScript server (`mcp-server/`)

TypeScript MCP server (30 tools) wrapping xberg extraction and xberg-rag over NAPI-RS bindings.
Not a workspace crate — built with `npm run build` / `task mcp:build`.
See `.ai-rulez/context/local-additions.md` for the full file map.

Tasks: `task mcp:build`, `task mcp:test`, `task mcp:dev`, `task mcp:lint`, `task rag-node:build`

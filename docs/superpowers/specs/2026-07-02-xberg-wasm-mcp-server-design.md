# Xberg WASM-Backed MCP Server — Design Spec

**Date:** 2026-07-02
**Status:** Approved (design)
**Scope:** Sub-project **E** — port the existing MCP server off native NAPI bindings onto the shared wasm engine, so the MCP server and the browser UI run the *same* `.wasm`.
**Depends on:** [B — Shared WASM Engine](2026-07-02-xberg-wasm-engine-design.md), [C — Shared JS Runtime Layer](2026-07-02-xberg-wasm-runtime-layer-design.md).

## Purpose

Today `mcp-server/` binds to native NAPI (`@xberg-io/xberg` = `xberg-node`, `xberg-rag-node`) — per-OS/arch prebuilds, vcpkg/libheif, DLL-copy dance. This sub-project replaces that native linkage with the **shared wasm engine (B)** driven by the **shared runtime (C)**, so Claude Code desktop and the Chrome UI share one binary and one ML/storage codebase.

The MCP protocol layer (JSON-RPC over stdio) stays JS — the `.wasm` is the engine, not the transport.

## Architecture

```
mcp-server/ (existing package, retargeted)
  src/index.ts          — registers tool groups, boots engine (unchanged shape)
  src/engine.ts         — NEW: constructs C factories (Node variants) + XbergEngine (B); replaces store.ts + native imports
  src/transports/stdio  — unchanged (Claude Code desktop)
  src/transports/http   — unchanged (SSE)
  src/tools/*           — retargeted: call engine.* instead of native @xberg-io/xberg
```

- Node hosts the wasm engine directly via standard async `wasm-bindgen` (`JsFuture`) — no JSPI dependency (see the Mechanism Correction below), no browser, no COOP/COEP needed.
- C's Node variants: `onnxruntime-node` for embedder/NER/OCR, wa-sqlite or (optionally) native SQLite for the store, `~/.cache/xberg` for model cache. **Same C interfaces as the browser**, different backend selection.

## Migration of the 13 tool groups

Each group's Zod schemas and MCP contract (tool names — stable public API) are **unchanged**; only the implementation body swaps native calls for `engine.*`:

| Tool group | Retarget |
|---|---|
| extract (`extract_document`, `extract_batch`, `list_formats`) | `engine.extract` |
| pii (`detect_pii`, `redact_document`) | `engine.detectPii` / `engine.redact` |
| rehydrate (`rehydrate_tokens`, `list_tokens`, `rehydrate_document`) | `engine.rehydrate` (in-wasm AES-GCM) |
| ingest (`ingest_document`, `ingest_folder`) | `engine.ingest` |
| query (`query_corpus`) | `engine.query` |
| collection / document / stats / reports / cache / intelligence / media / web | `engine.*` + existing JS orchestration |

Tool names must not change — renaming is a breaking change for connected agents (per `mcp-tool-patterns`).

## Behavioral parity requirements

- `rehydrate_tokens` now decrypts via the **in-wasm** Rust AES-GCM using the same `XPII\x01` container — must read map files produced by the *old* TS path (format-compatibility test from spec B applies here too).
- PII detection/redaction output must match the pre-migration native/TS output (parity test).
- OCR default becomes injected PaddleOCR (`ppu-paddle-ocr`) with Tesseract fallback — a capability upgrade (50+ languages) vs the old path; document in CHANGELOG.

## Async binding

> **Mechanism Correction (2026-07-02 review):** The bridges use standard async `wasm-bindgen` (`JsFuture`), not JavaScript Promise Integration — see the engine spec's Mechanism Correction. No JSPI dependency in Node either.

The engine's injected bridges work in Node exactly as in the browser (both are V8-based; the async-`wasm-bindgen` path is host-agnostic). C's single-flight rule per engine instance applies: the engine holds `&self` across `await`, so the server uses one engine instance (or a small pool) and serializes overlapping calls per instance.

## Error handling

- Tool handlers keep the existing `{ isError: true, content: [...] }` contract; engine errors mapped to that shape.
- Never `process.exit()` from a handler (per `mcp-tool-patterns`).
- Model unavailable + offline → OCR/NER degrade to in-binary fallback, logged at WARN; ingest continues (fail-open PII per `pii-pipeline`).

## Testing

- Existing `mcp-server/tests/*` retargeted: `redaction.test.ts` and `tools.test.ts` run against the wasm engine instead of native.
- Encryption round-trip + cross-format test (decrypt a legacy TS-produced map).
- One e2e per tool group: schema parse → engine call → assert `content` shape.
- Bench: extract + ingest latency vs the native baseline recorded (regression guard).

## Non-goals

- Changing the MCP tool surface, names, or transports.
- Component Model / WASI packaging (deferred — could later host the engine under Wasmtime instead of Node).
- Removing the native `xberg-node` packages from the repo (they remain for other consumers; only `mcp-server/` retargets).

## Sequencing note

E and D both depend on B + C. Recommended build order: **B → C → (D ∥ E)**. D and E are independent once C exists and can proceed in parallel.

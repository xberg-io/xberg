# Xberg Shared WASM Engine — Design Spec

**Date:** 2026-07-02
**Status:** Approved (design)
**Scope:** Sub-project **B** (shared wasm engine) with sub-project **A** (`ner-candle-wasm`) folded in as a prerequisite.

## Context

Xberg can compile to `wasm32`. The goal is one shared `.wasm` binary that powers two frontends:

- a minimal **browser UI** (Chrome/Edge), and
- a **full MCP server** for Claude Code desktop (Node V8),

exposing document intelligence with **RAG, anonymization, NER, and OCR**. The `.wasm` is a shared compute *engine*: all pure-Rust capability runs in-binary; capabilities that cannot live in wasm (embedding inference, vector-store persistence, GPU-accelerated NER) are reached through **host-injected interfaces**.

### Decomposition (whole initiative)

| # | Sub-project | Depends on |
|---|---|---|
| **A** | `ner-candle-wasm` enablement (pure-Rust prereq) | — |
| **B** | **Shared wasm engine** (this spec) | A |
| C | Shared JS runtime layer (injected impls: ORT-Web/transformers.js embedder + NER, wa-sqlite/OPFS store, model cache) | B |
| D | Browser UI | B, C |
| E | wasm-backed MCP server (port 13 tool groups off NAPI) | B, C |

This spec covers **A + B only**. C, D, E get their own spec → plan → build cycles.

### Approved decisions (from brainstorming)

1. **First spec** = engine (B) + `ner-candle-wasm` (A) as prerequisite.
2. **Browser target** = Chrome/Edge only → rely on **JSPI** for the async injection seam. Safari/Firefox out of scope (no JSPI).
3. **ML inference** = **hybrid** — embeddings + NER run via injected ORT-Web/transformers.js (WebGPU) as default; in-binary Candle-NER is the offline/no-GPU fallback.
4. **API shape** = **stateful `XbergEngine` handle** holding injected `Embedder` + `VectorStore` + config.
5. **Anonymization** = **full**, including reversible `token_replace` with AES-256-GCM encrypted rehydration maps, ported to pure Rust so it runs in-wasm.

## Architecture

One `wasm32` binary (`xberg-wasm`, extended). Contains all pure-Rust capability in-binary; reaches ML/storage through host-injected interfaces bridged via **JSPI** (JavaScript Promise Integration — Chrome/Edge). Identical `.wasm` for both hosts; hosts differ only in the injected JS impls (sub-project C) and the frontend (D/E).

Rationale for the current WASM ecosystem baseline (2025–2026), which de-risks this design:

- **JSPI** (Chrome/Edge stable, W3C track) lets synchronous Rust wasm call async JS and suspend/resume transparently — this is what makes the `Embedder`/`VectorStore` injection seam clean rather than a futures-glue problem.
- **WebGPU** ships across major browsers; injected ORT-Web/transformers.js NER + embeddings get 10–15× over WASM-CPU for larger models.
- **Wasm 3.0** baseline (relaxed SIMD, memory64) benefits in-binary compute (Tesseract OCR, Candle, extraction).

## Crate & module structure

- `crates/xberg-wasm/src/engine.rs` — `XbergEngine` handle; the stateful API surface.
- `crates/xberg-wasm/src/bridge/embedder.rs` — wasm-bindgen JSPI bridge implementing the core `xberg_rag::Embedder` trait by calling the injected JS `embed()`.
- `crates/xberg-wasm/src/bridge/store.rs` — JSPI bridge implementing `xberg_rag::VectorStore` over the injected JS store (`upsert`/`query`/`delete`/`list_collections`/`drop_collection`).
- `crates/xberg-wasm/src/anon.rs` — wrapper over core `redaction` + **new pure-Rust encrypted-map crypto** (ported from `mcp-server/src/redaction/rehydration.ts`).
- `crates/xberg-gliner-candle` — **(A)** add a sync-inference path and drop the `tokio-runtime` requirement so the crate compiles to `wasm32`.
- Core `crates/xberg/Cargo.toml` — new feature `ner-candle-wasm` (Candle NER without `tokio-runtime`); added to the `wasm-target` aggregate. `xberg-rag` `vector-store` + `pipeline` features (already documented WASM-safe, ORT-free) are enabled for the wasm build.

## The wasm API contract (`XbergEngine`)

The single most important artifact of this spec — the contract C/D/E consume.

```
new XbergEngine(config, { embedder, store })    // inject JS impls once

  .extract(input, config)              -> ExtractionResult   // in-binary
  .ocr(bytes, opts)                    -> OcrResult           // in-binary Tesseract
  .detectPii(text)                     -> Detection[]         // read-only, in-binary
  .redact(text, strategy)              -> RedactedDoc         // mask | hash | token_replace
  .rehydrate(doc, mapBytes, passphrase)-> text                // in-wasm AES-256-GCM
  .ner(text, opts)                     -> Entity[]            // injected fast-path / Candle fallback
  .ingest(doc, collection)             -> IngestReport        // extract → PII → embed → store
  .query(q, collection, k)             -> RetrievedChunk[]
```

- `config` mirrors the existing `Wasm*Config` serde types where they already exist; new types only where required (engine construction, injection descriptors).
- Every method returns `Result<T, JsValue>`.

## Injection seam (JSPI)

`Embedder` and `VectorStore` are already object-safe traits in `xberg-rag` (per the `rag-store` rule: no generics, no associated bounds). In the wasm they are implemented as **JSPI bridges**: synchronous Rust calls suspend on the injected async JS (`embed()`, store ops) and resume transparently. The engine never links a concrete embedder or store — the host supplies them at construction.

**NER hybrid dispatch:** `.ner()` calls the injected JS `ner()` first (ORT-Web/WebGPU); if no injected NER is present (or a no-GPU/offline flag is set), it falls back to the in-binary Candle backend (`ner-candle-wasm`).

## Capability placement

| Capability | Placement |
|---|---|
| Extract (91 formats), chunk, keywords | in-binary |
| PII detection, redaction strategies (mask/hash/token_replace) | in-binary (core `redaction` feature) |
| Tesseract OCR | in-binary (`ocr-wasm`) |
| AES-256-GCM rehydration | in-binary (new pure-Rust crypto) |
| GLiNER2 NER | injected ORT-Web/WebGPU (default) → Candle-wasm fallback (in-binary) |
| Embeddings | injected (ORT-Web / transformers.js) via JSPI |
| Vector store | injected (wa-sqlite/OPFS) via JSPI |

## Anonymization crypto (ported to Rust)

Core Rust already provides `RedactionStrategy::{Mask,Hash,TokenReplace}` and PII detection behind the wasm-safe `redaction` feature (`crates/xberg/src/types/redaction.rs`). The **encrypted rehydration-map crypto currently lives only in TypeScript/Node** (`mcp-server/src/redaction/rehydration.ts`, using `node:crypto`).

Port it to pure Rust in `anon.rs` using RustCrypto `aes-gcm` + `scrypt`, preserving the **byte-identical container format** so existing map files interoperate across both hosts:

```
XPII\x01 | salt(16) | iv/nonce(12) | tag(16) | ciphertext
```

- Key derivation: `scrypt(passphrase, salt, N=32768, r=8, p=1) -> 32-byte key`.
- Nonces via `getrandom` (`wasm_js`), already wired in `xberg-wasm` deps. Never reuse a nonce.
- Passphrase is supplied per call; never cached in wasm memory beyond the call.

## Data flow

**Ingest:** `bytes → extract (in-binary) → PII detect + redact (in-binary) → embed (injected, JSPI) → upsert (injected store, JSPI)`.

**Query:** `embed(query) (injected) → store.query (injected) → RetrievedChunk[] sorted by score desc, sliced to k`.

## Error handling

- Every engine method returns `Result<T, JsValue>`; core `Result<T, E>` is mapped to structured JS errors preserving **message + numeric code** (per the repo's FFI error-context rule).
- Injected-bridge failures (JS `embed()`/store op throws) propagate back as engine errors — **never panics**.
- PII detection failure follows the `pii-pipeline` fail-open rule: proceed without redaction and surface a warning rather than aborting ingest.

## Testing

- **Rust unit tests** for `anon.rs`: AES-256-GCM round-trip; a cross-check vector decrypting a map produced by the existing TS `encryptMapFile` (format-compatibility proof).
- **`wasm-bindgen-test`** for the `XbergEngine` handle using **stub** injected embedder/store (deterministic, no network) — covers `ingest`/`query` orchestration.
- **Candle-NER wasm smoke test** on a small fixture (entities detected, no panic).
- **Parity test:** identical input yields identical PII detection + redaction output between the in-binary Rust path and the current TS path.

## Explicit non-goals (this spec)

- The JS runtime impls (C), browser UI (D), and MCP port (E).
- Component Model / WASI packaging of the MCP server (deferred; browsers don't support components yet).
- Safari/Firefox support (no JSPI) — excluded by decision.
- WasmGC (irrelevant: Rust uses linear memory).

## Deployment constraints (recorded for consumers C/D)

These bind the consumers, not the engine itself, but are recorded here so C/D honor them:

- **COOP/COEP headers** (`Cross-Origin-Opener-Policy: same-origin`, `Cross-Origin-Embedder-Policy: require-corp`) required on the browser app for `SharedArrayBuffer` / WASM threads; without them ML falls back to single-threaded (3–4× slower), silently.
- wa-sqlite + model inference must run in a **Web Worker** (OPFS SQLite is Worker-only).
- Model weights cached in **OPFS** after first download (50–500 MB), not re-fetched per load.
- The MCP-under-Node host does not need COOP/COEP and can use Node threads directly — the two frontends are asymmetric on threading.

## Open risks

- **`ner-candle-wasm` weight/perf (A):** GLiNER2-Candle transformer inference on `wasm32` CPU may be large or slow. Mitigation: it is a *fallback*, not the default path; the injected ORT-Web/WebGPU path is primary. If Candle-wasm proves impractical, the fallback degrades to "NER unavailable offline" without blocking the engine.
- **JSPI single-active-computation:** JSPI does not allow re-entrant suspended computations. The engine must not issue concurrent JSPI-suspending calls on one instance; document single-flight usage per handle.

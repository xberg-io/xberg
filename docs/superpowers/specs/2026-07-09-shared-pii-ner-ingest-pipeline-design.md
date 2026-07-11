# Shared Engine-Driven PII+NER-Protected Ingest — Design Spec

**Date:** 2026-07-09
**Status:** Draft, pending user review
**Scope:** `crates/xberg` (new additive redaction function), `crates/xberg-rag` (new `pipeline-redaction` feature), `crates/xberg-wasm` (thread the PII model through `engine.ingest()`), `packages/xberg-wasm-runtime` (new shared folder orchestrator). Explicitly excludes `mcp-server/` and any browser UI — see Non-Goals.

## Context

`XbergEngine.ingest()` (`crates/xberg-wasm/src/engine.rs:142-194`) delegates to `xberg-rag`'s `ingest_document`/`ingest_document_local` (`crates/xberg-rag/src/pipeline.rs`), which only chunk, embed, and store — zero PII or NER handling. This contradicts the repo's own critical-priority `pii-pipeline` rule ("PII detection runs on extracted text before any embedding or storage operation") and the approved Sub-project D browser-UI spec, which assumes `engine.ingest()` does PII "internally (in-binary)".

Separately, `mcp-server/src/tools/ingest.ts`'s `ingest_folder` tool — the only place in the codebase that currently does folder-level ingest with PII protection — doesn't use the wasm engine or `xberg-rag`'s pipeline at all. It calls native NAPI `extract()` directly, does PII detection and redaction in hand-rolled TypeScript (`detectPii`/`applyRedaction`), chunks with a fixed 512-character slicer, and calls `embedTexts`/`store.upsertDocument` NAPI functions directly. It also supports optional NER-merge via pluggable `onnx`/`llm` backends.

The goal stated directly by the user: **"a shared wasm mcp architecture for full folder and document processing"** — MCP and the browser app must ingest documents (single or folder) through the identical engine-driven pipeline, so behavior can never drift between the two hosts the way it already has.

Locked decisions from brainstorming:
- Core pipeline (extract → PII detect → NER → redact → chunk → embed → store) must be identical and engine-driven for both hosts. File-output mechanics (writing `_REDACTED.*`, `_REPORT.docx`, `.map` files to disk) stay MCP-only, since the browser has no filesystem.
- PII detection lives in-Rust, inside the engine's ingest path — not duplicated in TypeScript per host.
- PII detection is mandatory and always runs; there is no opt-out.
- Redaction strategy is fixed to `token_replace` (the only reversible strategy); the rehydration map is returned to the caller, not persisted by the engine itself.
- NER uses the Candle GLiNER2 PII backend already wired into `xberg-wasm` this session (`fastino/gliner2-privacy-filter-PII-multi`, pure-Rust, wasm-safe, no API keys or runtime downloads) — not the pluggable onnx/llm backends `mcp-server` uses today.

## Architecture

Four layers, each with one clear responsibility:

```
xberg (core)         xberg::text::redaction::redact()  — canonical token-replace algorithm
      |
xberg-rag (pipeline)  ingest_document/_local — regex PII + Candle NER + redact, before chunk/embed/store
      |
xberg-wasm (engine)   engine.ingest() — supplies the loaded Candle PII model, surfaces the rehydration map
      |
xberg-wasm-runtime    ingestFolder() — host-agnostic loop: extract → ingest, per file, no file I/O
      |
  [host-specific]      mcp-server: folder walk + redacted-file writing (Sub-project E, not this plan)
                        browser UI: File System Access API + UI (Sub-project D, not this plan)
```

### 1. `crates/xberg` — canonical `redact()` function (new, purely additive)

`engine.rs::redact()` already has a TODO admitting it duplicates a reverse-offset token-replace algorithm inline "rather than delegating to `xberg::text::redaction::redact`" — a function that turns out not to exist yet (only `patterns::scan_text` and the `rehydration::{encrypt_map, decrypt_map}` helpers exist in core). Since this plan needs that same algorithm a third time, it gets extracted once, correctly, instead of copy-pasted again.

```rust
// crates/xberg/src/text/redaction/mod.rs (or similar), under the existing `redaction` feature
pub fn redact(
    text: &str,
    categories: &[PiiCategory],
    strategy: RedactionStrategy,
) -> RedactOutcome {
    // lifted verbatim from engine.rs::redact()'s reverse-byte-order replacement loop
}

pub struct RedactOutcome {
    pub redacted_text: String,
    pub rehydration_map: RehydrationMap,       // populated only for TokenReplace
    pub category_counts: HashMap<String, usize>, // counts only, never raw PII values
}
```

`engine.rs::redact()`/`detect_pii()` are **not** refactored to call this in this plan — noted as a follow-up (see Non-Goals) to keep this change purely additive to core.

### 2. `crates/xberg-rag` — `pipeline-redaction` feature

New Cargo feature, following the existing `pipeline-*` naming convention:

```toml
pipeline-redaction = ["pipeline", "xberg/redaction", "dep:xberg-gliner-candle"]
```

`xberg-gliner-candle` becomes a new optional dependency — confirmed pure-Rust (candle-core/nn/transformers, tokenizers, ndarray, safetensors), no tokio, no hard ORT dependency, already `wasm-bindgen-test`-covered. `GlinerModel::extract_ner(text: &str, labels: &[&str], threshold: f32) -> Result<Vec<Span>>` is fully synchronous and target-agnostic.

When this feature is enabled, `ingest_document`/`ingest_document_local` gain a new required parameter and a new return type (both cfg-gated — callers who don't enable the feature see zero change to today's signature):

```rust
#[cfg(feature = "pipeline-redaction")]
pub async fn ingest_document_local(
    store: Arc<dyn VectorStore>,
    collection: &str,
    request: IngestRequest,
    config: &RagPipelineConfig<'_>,
    embedder: &dyn Embedder,
    pii_model: &xberg_gliner_candle::GlinerModel,
) -> RagResult<IngestOutcome> { ... }

pub struct IngestOutcome {
    pub document_id: DocumentId,
    pub rehydration_map: RehydrationMap,
    pub pii_category_counts: HashMap<String, usize>,
}
```

Sequence inserted before chunking:
1. `xberg::text::redaction::patterns::scan_text(&request.full_text, &all_pii_categories)` — regex matches
2. `pii_model.extract_ner(&request.full_text, PII_NER_LABELS, threshold)` — Candle NER matches
3. Merge both match sets by byte span (dedup overlaps, regex match wins on exact overlap — cheap and deterministic; NER is additive coverage for entity-shaped PII regex can't pattern-match, like unlabeled names)
4. `xberg::text::redaction::redact(&request.full_text, &merged, RedactionStrategy::TokenReplace)` — produces `RedactOutcome`
5. Chunk/embed/store proceed on `RedactOutcome::redacted_text`, **not** `request.full_text`
6. Return `IngestOutcome { document_id, rehydration_map, pii_category_counts }`

### 3. `crates/xberg-wasm` — thread the model through `engine.ingest()`

Reuses the Candle backend already loaded via Task 11's `initCandleNer()`/`CANDLE_NER` thread_local (this session's earlier PII-backend wiring). `engine.ingest()` passes the loaded model into `ingest_document_local` and adds `rehydrationMap`/`piiCategoryCounts` to its JS response object, alongside the existing document id. If the PII model hasn't been initialized (`initCandleNer()` not yet called), `ingest()` returns an error rather than silently skipping PII protection — mandatory means mandatory.

### 4. `packages/xberg-wasm-runtime` — shared, host-agnostic folder orchestrator

New export, e.g. `src/ingest-folder.ts`:

```typescript
export interface FolderFileSource {
	name: string;      // filename only, e.g. "report.pdf"
	path: string;       // host-opaque source identifier, stored as source_uri/metadata
	bytes: Uint8Array;
}

export interface IngestFolderFileResult {
	filename: string;
	documentId: string | null;
	chunksCreated: number;
	piiCategoryCounts: Record<string, number>;
	redactedText: string;               // needed by MCP's file-writing wrapper
	rehydrationMap: Record<string, string>; // needed by MCP's .map-file wrapper
	error?: string;
}

export async function ingestFolder(
	engine: XbergEngineHandle,
	store: VectorStoreInterface,
	collection: string,
	files: FolderFileSource[],
): Promise<IngestFolderFileResult[]>
```

Loops `engine.extract(bytes)` → `engine.ingest(collection, { fullText, ... })` per file, collecting results. **No filesystem access anywhere in this function** — that is the host boundary. A single failed file records `error` on its own result entry and does not abort the batch (matches `ingest_folder`'s current per-file try/catch behavior).

This is the one function both `mcp-server` (after its Sub-project E retarget) and the future browser UI (Sub-project D) will call, making the extract→PII→NER→redact→chunk→embed→store sequence byte-for-byte identical between them.

## Error Handling

- PII model not initialized when `ingest()` is called → hard error (mandatory PII, no silent skip), propagated as a rejected promise on the JS side.
- Embedding count mismatch, chunking failure, store failure → unchanged, existing `RagError` variants propagate as today.
- `ingestFolder()`: a single file's extract/ingest failure is caught and recorded per-file (`error` field), the loop continues — matches existing `ingest_folder` MCP tool semantics operators already rely on.
- PII category counts are the only PII-adjacent data logged anywhere in this pipeline — actual matched text is never logged, consistent with the existing `pii-pipeline` rule.

## Testing

- `crates/xberg`: unit tests for the new `redact()` covering all four strategies (even though this plan only calls `TokenReplace`), verifying it produces byte-identical output to `engine.rs::redact()`'s current inline implementation on the same fixtures (regression guard for the extraction).
- `crates/xberg-rag`: integration test for `ingest_document_local` under `pipeline-redaction` — a fixture with known PII spans (email, phone, an unlabeled name only NER would catch) asserting the stored chunks contain no raw PII, `rehydration_map` round-trips back to original text, and `pii_category_counts` matches expected counts.
- `crates/xberg-wasm`: test that `engine.ingest()` errors when called before `initCandleNer()`, and that a successful call's JS response includes `rehydrationMap`/`piiCategoryCounts`.
- `packages/xberg-wasm-runtime`: test `ingestFolder()` against a small in-memory fixture set (2-3 files, one deliberately malformed to exercise the per-file error path), asserting result count, per-file PII counts, and that a bad file doesn't abort the rest of the batch.

## Non-Goals

- Rewriting `mcp-server/src/tools/ingest.ts` to call the new `ingestFolder()` — that retarget is Sub-project E's assigned work; this plan only makes sure the contract it will consume is correct and ready.
- Any browser UI implementation (Sub-project D remains unimplemented).
- Refactoring `engine.rs::redact()`/`detect_pii()` to delegate to the new core `redact()` function — flagged as a follow-up, not done here, to keep this change purely additive.
- Pluggable/optional NER backends (onnx/llm) matching `mcp-server`'s current `use_ner` flag — Candle GLiNER2 PII only, per the locked decision above.
- Persisting or encrypting the rehydration map — the engine returns it to the caller; encryption already exists as a separate, unchanged JS-callable method (`engine.encryptMap()`) for hosts that want to persist it.

## Blast Radius

Touches four crates/packages, all additive or cfg-gated:
- `crates/xberg`: one new function, no existing call site changed.
- `crates/xberg-rag`: one new feature flag; existing `pipeline`/`pipeline-embeddings`/etc. callers unaffected since the signature/return-type change is behind `pipeline-redaction`.
- `crates/xberg-wasm`: `engine.ingest()` body changes; `engine.detect_pii()`/`engine.redact()` untouched.
- `packages/xberg-wasm-runtime`: one new file, no existing exports changed.

No changes to `mcp-server/` — the package another developer is actively retargeting for Sub-project E. No conflicting file overlap with that work as of this writing.

# LoRA-Capable GLiNER2 + Privacy API — Design

**Date:** 2026-06-30
**Status:** Approved for planning
**Supersedes:** Phases 1–2 of `docs/superpowers/plans/2026-06-29-xberg-privacy-api.md`

---

## 1. Motivation & Context

xberg's NER pipeline is ONNX/LLM only: GLiNER1 (span-mode ONNX), the newly
added GLiNER2 (schema-prompt ONNX via `ort`, on branch
`feature/gliner2-onnx-backend`), and liter-llm. None of these can accept LoRA
adapters — an ONNX graph has weights baked in, so domain specialization today
means re-exporting and shipping a full multi-GB ONNX model per domain.

The `../anno` repository already contains a Candle (pure-Rust) GLiNER2 backend
(`crates/anno/src/backends/gliner2_fastino_candle/`) built specifically for
**runtime PEFT LoRA adapter merge-at-load**: one base model + small (MB-scale)
domain adapters (`legal`, `medical`, `financial`, `fr-pii`) swapped at runtime.

This is the differentiating engine capability for an xberg **Privacy/GDPR API**:
"redact French legal PII" becomes *base GLiNER2 + `fr-legal-pii` adapter*, not a
separate model artifact.

### What this design does NOT redo

The original privacy-api plan proposed building PII detection and AES-256-GCM
rehydration from scratch. Both already exist and are reused unchanged:

- Rust: `crates/xberg/src/text/redaction/` (`engine.rs`, `strategy.rs`,
  `patterns/`) + `core/config/redaction.rs` + `types/redaction.rs`
  (`PiiCategory`, `RedactionStrategy::{Mask,Hash,TokenReplace}`).
- TypeScript: `mcp-server/src/redaction/{detect,redact,rehydration}.ts`
  (AES-256-GCM + scrypt rehydration, already tested).

GLiNER2 porting (Phase 1 of the old plan) is already shipped via
`2026-06-30-gliner2-onnx-backend.md` and the `v2_*` modules in `xberg-gliner`.

---

## 2. Verified Facts (de-risking)

These were confirmed by reading source before committing to the design:

1. **Candle is already in-tree.** The workspace pins
   `candle-core/candle-nn/candle-transformers = 0.11` (`Cargo.toml:54-56`,
   `Cargo.lock` resolves `candle-transformers 0.11.0`), used by
   `crates/xberg-candle-ocr`.
2. **The `debertav2` encoder API is identical between anno's Candle 0.10 and
   xberg's 0.11.** Both expose `DebertaV2Model::load(vb, config)` and
   `forward(input_ids, token_type_ids: Option<Tensor>, attention_mask:
   Option<Tensor>)`. anno's encoder wrapper calls exactly this shape. The port
   needs no encoder API rewrite.
3. **anno's Candle backend is `Phase 4 in-progress`** (`#![allow(dead_code)]`)
   and all its integration tests are `#[ignore]`-gated, requiring local model
   and adapter directories (`GLINER2_TEST_ADAPTER_DIR`, etc.). It compiles but
   real-inference correctness is **not** proven by upstream CI. → Validation,
   not integration, is the dominant risk (see §7).

---

## 3. Architecture Overview

Two layers: a new isolated engine crate, and config/dispatch wiring in the core.

```text
┌──────────────────────────────────────────────────────────────┐
│ Privacy API  (crates/xberg/src/api)                           │
│   POST /v1/process   → extract → ner(adapter) → pii → redact  │
│   POST /v1/documents/{id}/rehydrate   (existing rehydration)  │
│   POST /v1/search                     (existing xberg-rag)    │
├──────────────────────────────────────────────────────────────┤
│ Core config + NER processor  (crates/xberg)                   │
│   NerBackendKind::GlinerCandle + NerConfig.adapter            │
│   AdapterRegistry (LRU of merged engines, keyed by name)      │
├──────────────────────────────────────────────────────────────┤
│ Engine: xberg-gliner-candle  (NEW crate)                      │
│   DeBERTa-v2 encoder + 6 heads + PEFT lora merge-at-load      │
│   reuses xberg-gliner: Span, SpanOutput, GlinerError, schema  │
└──────────────────────────────────────────────────────────────┘
```

---

## 4. Layer A — Engine crate `xberg-gliner-candle`

### Why a new crate (not a feature on `xberg-gliner`)

`xberg-gliner` depends on `ort` and is deliberately lean. Adding Candle as a
feature would still couple the two heavy ML stacks in one manifest. A separate
crate mirrors the existing `xberg-candle-ocr` precedent and keeps Candle deps
fully isolated and feature-gated at the workspace edge. It depends on
`xberg-gliner` (path dep) only for shared, engine-agnostic types.

### Modules (ported from `anno::gliner2_fastino_candle`)

| File | Responsibility |
|------|----------------|
| `encoder.rs` | Thin wrapper over `candle_transformers::models::debertav2::DebertaV2Model` |
| `heads/mod.rs` + 6 heads | token_gather, span_rep, schema_gather, count_pred, count_lstm, scorer (the `classifier` head is excluded — `extract_structure`/`classify` are out of scope) |
| `lora.rs` | PEFT `adapter_config.json` + `adapter_model.safetensors` load; `W += (α/r)·Bᵀ·A` merge-at-load; `fan_in_fan_out` transpose handling |
| `processor.rs` | schema-prompt construction (`[P]`/`[E]`/`[SEP]` markers, text/schema position maps) |
| `decoder.rs` | span scoring → entities (reuse NMS/greedy-merge from `xberg-gliner` where contracts match) |
| `pipeline.rs` | orchestrates encode → forward → decode |
| `mod.rs` | `Gliner2Candle` struct + public API |

### Public API

```rust
pub struct Gliner2Candle { /* tokenizer, device, base_model_dir, encoder, heads, active_adapter, model_id */ }

impl Gliner2Candle {
    pub fn from_local(model_dir: &Path) -> Result<Self>;          // base weights, no adapter
    pub fn load_adapter(&mut self, source: &AdapterSource) -> Result<()>;   // re-merge from base + adapter
    pub fn unload_adapter(&mut self) -> Result<()>;               // re-merge from base only
    pub fn extract_ner(&self, text: &str, labels: &[&str], threshold: f32) -> Result<Vec<Span>>;
    pub fn extract_structure(&self, text: &str, schema: &TaskSchema, threshold: f32) -> Result<Vec<ExtractedStructure>>;
}
```

### Shared types reused from `xberg-gliner`

`Span`, `SpanOutput`, `GlinerError`/`Result`, and the GLiNER2 schema types
already present in the `v2_*` path. The Candle decoder must produce the same
`Span`/`SpanOutput` so downstream NER mapping is engine-agnostic.

### Reference-equivalence note

anno documents the merge as equivalent to PyTorch `peft.merge_and_unload`.
GLiNER2 `span_scores` output is **post-sigmoid** — do not re-apply sigmoid in
the Candle decode path (same rule the ONNX v2 path follows).

### Cargo / features

New workspace member `crates/xberg-gliner-candle`. Default off in `xberg`'s
feature set; surfaced via a `ner-candle` feature on `crates/xberg`. Device
pass-throughs (`cuda`, `metal`) mirror `xberg-candle-ocr`'s feature names.

---

## 5. Layer B — Config & dispatch (`crates/xberg`)

### NerConfig changes (`core/config/ner.rs`)

```rust
pub enum NerBackendKind {
    Onnx,           // existing — GLiNER1 + GLiNER2 ONNX, selected via hf_architecture
    Llm,            // existing
    GlinerCandle,   // NEW — Candle GLiNER2; the only backend that accepts adapters
}

pub struct NerConfig {
    // ... existing fields ...
    /// Domain LoRA adapter. Only honoured by `NerBackendKind::GlinerCandle`.
    /// Setting this with any other backend is a configuration error.
    pub adapter: Option<AdapterConfig>,
}

pub struct AdapterConfig {
    pub name: String,              // registry key + cache identity
    pub source: AdapterSource,
}

pub enum AdapterSource {
    Local { path: PathBuf },
    HfRepo { repo: String, revision: Option<String> },   // downloaded via existing hf-hub path
}
```

### Dispatch

`plugins/processor/builtin/ner.rs` gains a `GlinerCandle` arm that resolves the
engine from the AdapterRegistry (§6) and calls `extract_ner` / `extract_structure`.
Validation: `adapter.is_some() && backend != GlinerCandle` → typed config error.

### Type-system rationale

A new `NerBackendKind::GlinerCandle` (rather than overloading
`GlinerArchitecture`) makes "adapters require the Candle engine" a compile-
checkable distinction at the config boundary, and keeps the ONNX
`Onnx + GlinerArchitecture::Gliner2` path untouched.

---

## 6. Adapter lifecycle & concurrency

- **Merge-at-load, zero per-forward cost.** Merging produces full base weights
  with the delta folded in (~100ms for a ~280M model at rank 8, per anno's
  measurement). Inference then runs at base-model speed.
- **AdapterRegistry: bounded LRU of merged engines keyed by `adapter.name`.**
  The base (no-adapter) engine is always resident. Domain adapters populate the
  LRU on first use; eviction bounded by a configured RAM/entry cap.
- **Per-request domain selection** (multi-tenant API) is served from the LRU
  without serializing swaps — each merged engine is independent and
  `Send + Sync`, shared behind `Arc`. This avoids anno's single-`active_adapter`
  limitation (anno assumes swap "every few minutes/hours, not per request").
- Inference runs on `spawn_blocking` (CPU-bound, per the async-and-concurrency
  rules); the registry hands out `Arc<Gliner2Candle>` clones.

---

## 7. Validation strategy (dominant risk)

Real-inference correctness is unproven upstream, so validation is explicit:

1. **CI-safe ported tests** (no model files needed): synthetic zero-adapter is a
   no-op; synthetic random adapter changes inference output; LoRA key parsing is
   strict; `apply_lora_delta` shape/scale math. (These are anno's
   non-model-dependent unit tests in `lora.rs` + synthetic-adapter tests.)
2. **Gated smoke test**: one `#[ignore]` integration test reading a real GLiNER2
   safetensors model dir + a real PEFT adapter dir from env
   (`GLINER2_CANDLE_MODEL_DIR`, `GLINER2_TEST_ADAPTER_DIR`), asserting entity
   extraction and that adapter load measurably changes outputs.
3. **Parity check** (optional): compare Candle base-model NER against the ONNX
   GLiNER2 path on the same input to confirm the decode contract matches.

**Pre-port spike (do first):** obtain a real GLiNER2 ONNX-or-safetensors base +
one real PEFT adapter, run anno's ignored tests in-place to confirm the upstream
backend actually produces correct entities on Candle 0.11 before porting.

---

## 8. Privacy API integration (broader plan)

The LoRA engine is the NER step inside the unified pipeline — the salvageable
kernel of the original plan.

- **`POST /v1/process`** — operations pipeline over one input:
  `input → extract → ner{backend: gliner_candle, adapter} → pii.detect →
  redact{strategy} → [embed → store]`. NER and PII reuse existing engines; only
  the orchestration handler + request/response types are new. Adapter selection
  is a first-class privacy knob in the request.
- **`POST /v1/documents/{id}/rehydrate`** — reuses the existing rehydration map
  logic; design owes a decision on *where* encrypted maps are persisted/looked
  up by document id (deferred to the implementation plan).
- **`POST /v1/search`** — reuses `xberg-rag`; thin HTTP wrapper. **Descoped from
  this plan** to avoid two plans implementing the same endpoint differently: it
  is owned by `2026-06-29-xberg-privacy-api.md` Phase 4. Listed here only to show
  where it fits the surface.

These endpoints modify `crates/xberg/` (which `CLAUDE.md` marks "never modify").
This is an accepted, deliberate fork-local exception — there is no upstream
equivalent — mirroring the same exception already taken by the GLiNER2 plan.

---

## 9. Out of scope (future work)

Deferred until LoRA lands and proves out:

- anno's sealed `Model` trait adoption + `StackedNER`/`EnsembleNER` combiners
  (the composability track).
- Additional backends: NuNER (token-class zero-shot), W2NER (nested entities),
  GLiNER-multitask, relation extraction (GLiREL/TPLinker), coreference, the
  per-input `muxer`, and the `anno-eval` harness.

---

## 10. Component boundaries summary

| Unit | Does | Depends on | Tested by |
|------|------|-----------|-----------|
| `xberg-gliner-candle` | Candle GLiNER2 inference + LoRA merge | `candle-*` 0.11, `xberg-gliner` types | unit (CI) + gated smoke |
| AdapterRegistry | Merge cache + per-request selection | `xberg-gliner-candle` | unit (LRU/eviction) |
| NER processor arm | Route `GlinerCandle` → engine | config + registry | unit + integration |
| `/v1/process` handler | Orchestrate pipeline | extract, ner, redaction, rag | API integration |

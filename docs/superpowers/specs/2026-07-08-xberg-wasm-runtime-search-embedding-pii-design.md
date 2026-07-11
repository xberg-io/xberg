# Xberg WASM Runtime: Hybrid Search, BGE-M3 Embeddings, and Candle PII NER — Design Spec

**Date:** 2026-07-08
**Status:** Draft, pending user review
**Scope:** Three capability upgrades on top of `feature/wasm-runtime-sqlite-store` (Sub-project C's SQLite vector+graph store, [2026-07-07-xberg-wasm-sqlite-vec-store-and-perf.md](../plans/2026-07-07-xberg-wasm-sqlite-vec-store-and-perf.md), already implemented and merged into this branch):

1. Hybrid/full-text retrieval — adopts [2026-07-07-xberg-wasm-store-hybrid-fulltext-design.md](2026-07-07-xberg-wasm-store-hybrid-fulltext-design.md) directly.
2. Embedding model upgrade from `Xenova/all-MiniLM-L6-v2` (384-dim) to `BAAI/bge-m3` (1024-dim, multilingual).
3. PII-aware entity detection: wire the already-built, already-merged `xberg-gliner-candle` (Candle GLiNER2, zero-shot, LoRA-capable — PR #3 "LoRA-capable GLiNER2 + Privacy API") into the `xberg-wasm` engine's currently-stubbed NER fallback, using `fastino/gliner2-privacy-filter-PII-multi` as the pinned model; plus a JS-side regex complement ported from `mcp-server/src/redaction/detect.ts`.

**Why one combined plan:** these three upgrades were investigated together and share the same package (`packages/xberg-wasm-runtime`) and the same underlying discipline established by the prerequisite plan (verify model/library claims against real sources before committing — two prior NER model IDs referenced in this codebase's history, `Xenova/gliner2-small-onnx` and `SemplificaAI/gliner2-multi-v1-onnx`, turned out not to exist or not to fit the pipeline shape used). This spec documents what was independently verified for each of the three pieces, not assumed.

---

## Component 1: Hybrid / Full-Text Search

Adopts the existing draft spec's design as-is — see [2026-07-07-xberg-wasm-store-hybrid-fulltext-design.md](2026-07-07-xberg-wasm-store-hybrid-fulltext-design.md) for full detail. Summary:

- **Schema:** an FTS5 external-content virtual table `chunks_fts`, added to `store-schema.ts`'s `SCHEMA_SQL`, kept in sync with `chunks` via `AFTER INSERT/UPDATE/DELETE` triggers.
- **Interface:** new `VectorStoreInterface.retrieve(collection, { mode: "vector"|"fulltext"|"hybrid", queryText?, queryVector?, k })`. Existing `query(collection, queryVector, k)` is unchanged (backward compatible).
- **Fusion:** Reciprocal Rank Fusion (`score = Σ 1/(60 + rank)` across modes present) for `hybrid` mode — a documented IR-standard default, not a re-derivation of `crates/xberg-rag`'s exact fusion algorithm (separate storage engines; ranking-quality parity is the requirement, not byte-identical scores).
- **One added planning-phase verification** beyond the draft spec's own open questions: confirm `@sqlite.org/sqlite-wasm`'s specific vendored WASM build actually has `ENABLE_FTS5` compiled in (`SELECT * FROM pragma_compile_options() WHERE compile_options LIKE 'ENABLE_FTS5'`) before relying on it browser-side — WASM builds sometimes trim optional SQLite features to reduce binary size, and FTS5 compiled-in-by-default is true for the *native* SQLite/`@sqlite.org/sqlite-wasm` distribution in general but must be checked against the actual vendored build this package uses (`packages/xberg-wasm-runtime/wasm/sqlite-vec/sqlite3.wasm`, built via `scripts/build-sqlite-vec-wasm.sh` from a pinned `sqlite-vec` source commit — that build path was not verified to preserve `ENABLE_FTS5`).

## Component 2: Embedding Model Upgrade (BGE-M3)

- **Current state:** `packages/xberg-wasm-runtime/src/embedder.ts:11` — `DEFAULT_MODEL = "Xenova/all-MiniLM-L6-v2"` (384-dim, English-centric). This was a substitution made during the prerequisite plan's Task 3 because the *test* model ID in that plan's own snippet (`Xenova/minilm-l6-v2`) didn't exist — MiniLM was never a deliberate quality choice.
- **Verified replacement:** `Xenova/bge-m3` (also available as `onnx-community/bge-m3-ONNX`) — confirmed real, live, `transformers.js`-compatible ONNX exports of `BAAI/bge-m3`, 1024-dim output (verified via the model card's own usage example: `dims: [2, 1024]`), 51+ likes, used in production HF Spaces (e.g. `lamhieu/lightweight-embeddings`). 278 quantized derivatives exist in the model tree.
- **Quantization is already automatic, no new code needed:** `packages/xberg-wasm-runtime/src/backend.ts:selectModelBackend()` already returns `{ device: "cpu", dtype: "q8" }` for Node, `{ device: "wasm", dtype: "q8" }` for browser-WASM, and `{ device: "webgpu", dtype: "fp32" }` only when WebGPU is available. This is model-agnostic and already passed into `pipeline("feature-extraction", modelId, backend)` in `embedder.ts` — swapping `DEFAULT_MODEL` picks up `q8` quantization automatically on the two lower-bandwidth paths. Expected real sizes: ~2.2GB fp32 (WebGPU path), ~550-600MB q8 (Node/WASM paths, standard ~4x int8 reduction) — the exact q8 number needs confirming against the real download, not assumed.
- **Dimension change is low-risk:** `ensureCollection(collection, vectorDim)` (Component 1's prerequisite plan) already takes `vectorDim` as a per-collection parameter — nothing in `store-schema.ts`/`store-node.ts`/`store-worker.ts` hardcodes 384. No stored production vectors exist yet (this package isn't consumed by `xberg-wasm` yet), so there is no live-data migration concern.
- **Cache metadata:** `cache.ts`'s `MODELS` list entry for the embedder needs its `size` field updated to the real quantized-default size once confirmed (currently reflects an earlier estimate, not BGE-M3).

### Task ordering for Component 2

1. **Validation spike:** load `Xenova/bge-m3` via `pipeline("feature-extraction", ...)` under all three `{device, dtype}` combinations `selectModelBackend()` can return (q8/cpu, q8/wasm, fp32/webgpu). Record real download sizes. If this genuinely fails, report BLOCKED with the exact error and fall back to a documented lighter alternative (e.g. `Xenova/bge-small-en-v1.5`) rather than silently shipping something broken — do not repeat the prerequisite plan's pattern of substituting models without flagging it.
2. Swap `DEFAULT_MODEL`, update `embedder.test.ts` dimension assertions (1024, not the current 384-based ones), update `cache.ts`'s size metadata.

## Component 3: PII-Aware Entity Detection

Two independent layers, chosen deliberately (not one replacing the other):

### 3a. In-binary Candle GLiNER2 (the real fix)

**Current gap, verified by reading the actual bridge code:** `crates/xberg-wasm/src/bridge/ner.rs`'s `resolve_ner()` tries an injected JS NER object first, then falls back to an in-binary path gated on the `ner-candle-wasm` feature (already part of the `wasm-target` aggregate in `crates/xberg/Cargo.toml`). But `fallback_ner()` is a stub — it returns a hardcoded diagnostic error (`"NER unavailable: no injected backend and ner-candle-wasm not initialized with model bytes"`) instead of actually calling the Candle backend. The capability exists and is tested elsewhere; it was never wired into the wasm bridge.

**What already exists and was independently verified (not assumed):**
- `crates/xberg-gliner-candle` — a real, merged (PR #3, "LoRA-capable GLiNER2 + Privacy API"), genuinely zero-shot/schema-driven Candle implementation. `pipeline::run_pipeline(text: &str, labels: &[String])` takes arbitrary runtime labels — not a fixed label set.
- `crates/xberg/src/text/ner/candle.rs`'s `CandleBackend::from_bytes(safetensors: &[u8], tokenizer_json: &[u8], encoder_config_json: &[u8])` — already exists, explicitly documented as "required on wasm32" (no filesystem access there). Implements the `NerBackend` trait, mapping `EntityCategory::{Person, Organization, Location, Email, Phone, Date, Time, Money, Percent, Url, Custom(String)}` to/from GLiNER2 labels — `Email`/`Phone` are already first-class categories, and `Custom(String)` covers PII types outside that set (SSN, IBAN, credit card, etc.) since the model is zero-shot.
- `crates/xberg-gliner-candle` itself does **not** use `wasm-bindgen` — it's plain, target-agnostic Rust compiled straight to `wasm32-unknown-unknown` (its only `wasm-bindgen` reference is `wasm-bindgen-test` as a dev-dependency, used to run its own unit tests, per the prior `ner-candle-wasm` plan's Task 4: "1 passed; 0 failed" on real wasm32). The `wasm-bindgen`/`wasm-bindgen-futures`/`serde-wasm-bindgen` *runtime* dependencies live one layer up, in `crates/xberg-wasm` — the crate that already has `#[wasm_bindgen(js_name = "...")]`-annotated functions (e.g. `anon.rs`'s `encryptRehydrationMap`).

**Pinned model — verified at the file and config-schema level, not just by name:**

`fastino/gliner2-privacy-filter-PII-multi` — a real, safetensors-native, Apache-2.0 fine-tune of Fastino AI's GLiNER2 (205M base params, 307M total per the safetensors file) for PII specifically:

- 42 fine-grained PII entity types across 7 groups (person/names, government/tax IDs, banking/payment incl. IBAN/card number/CVV, digital identity, secrets/credentials incl. API keys/passwords, sensitive dates).
- 7 languages: EN, FR, ES, DE, IT, PT, NL.
- Highest span-level F1 (0.477) on the SPY benchmark among compared systems (OpenAI Privacy Filter, NVIDIA `gliner-PII`, `urchade/gliner_multi_pii-v1`) — [arXiv:2605.09973](https://arxiv.org/abs/2605.09973).
- Still genuinely zero-shot at inference — "pass any subset of the 42 supported labels — the model conditions on the labels you provide."
- File listing (verified via the HF API, not assumed): `config.json` (GLiNER2 wrapper config), `encoder_config/config.json` (the actual DeBERTa encoder config — this exact two-file split matches `xberg-gliner-candle/src/model.rs`'s own doc comment verbatim: *"tokenizer.json, config.json (or encoder_config/config.json)"*), `model.safetensors` (307,098,645 F32 params ≈ 1.17 GiB, no pre-quantized variant shipped), `tokenizer.json`, `tokenizer_config.json`.
- `encoder_config/config.json` contents verified field-for-field: `"model_type": "deberta-v2"`, base encoder `microsoft/mdeberta-v3-base`, `hidden_size: 768`, `num_hidden_layers: 12`, `num_attention_heads: 12`, `pos_att_type: ["p2c","c2p"]`, `relative_attention: true`, `position_buckets: 256`, `share_att_key: true`, `norm_rel_ebd: "layer_norm"`, `vocab_size: 250112`. These are DeBERTa-v2/v3 disentangled-attention fields, matching what `encoder.rs`'s `candle_transformers::models::debertav2::Config` expects — not a generic/mismatched BERT config. This is confirmed by inspection; actual tensor-name/shape compatibility (does `model.safetensors` carry the `encoder.` prefix `encoder.rs` strips via `vb.pp("encoder")`?) can only be confirmed by running the loader, which is Task A below.

**Ruled out, with reasons (do not revisit without new information):**
- `SemplificaAI/gliner2-privacy-filter-PII-multi` (and the sibling `gliner2-multi-v1-onnx`) — confirmed to be ONNX re-exports of this same Fastino model, built for `gliner2-rs`, a separate Rust engine using the `ort` crate. Per this repo's own established feature-flag policy (`.ai-rulez/context/local-additions.md`'s ORT-incompatible-targets rule), `ort`-dependent paths cannot link into `wasm32-unknown-unknown` at all — same constraint that already excludes `paddle-ocr`/`layout-detection`/`embeddings` from `wasm-target`. Also fragmented into 5 separate ONNX files for `gliner2-rs`'s own custom graph orchestration, not a drop-in single-graph model even outside the wasm32 constraint.
- True quantized (Q4/Q8-style) Candle inference for this model — `candle-transformers 0.11` (this repo's pinned version) has no quantized DebertaV2 implementation; only LLM-family models (`quantized_llama`, `quantized_t5`, `quantized_qwen2`, `quantized_blip`) have quantized variants. Writing one is comparable effort to authoring a new `candle-transformers` model file — explicitly out of scope (see Non-Goals).

**Also found, independent bug — fix regardless of the above:** `call_injected_ner` in `crates/xberg-wasm/src/bridge/ner.rs` calls the injected JS NER **positionally** — `func.apply(&obj, [text, categoriesArray])` — but `xberg-wasm-runtime`'s `NerInterface.ner(text: string, opts?: NerOpts)` (in `types.ts`) expects an **options object** (`{categories?, threshold?}`). When the real engine calls into the JS runtime today, `opts` is literally the raw array, so `opts?.categories` is `undefined` and category filtering silently breaks. Fix on the JS side — `ner.ts`'s `ner()` should accept `categories: string[]` as a plain second positional argument to match what the Rust bridge (the fixed contract) actually sends.

### Task breakdown for Component 3a

- **Task A (validation, not assumed-done):** download `fastino/gliner2-privacy-filter-PII-multi`'s three files, run `Gliner2Candle::from_bytes`/`Encoder::from_buffered_safetensors` against them in a real (not `#[ignore]`d) test, confirm no tensor-name/shape mismatch. If this fails, report BLOCKED with the exact error — the config-schema match above is strong evidence, not proof.
- **Task B:** implement `fallback_ner()` in `crates/xberg-wasm/src/bridge/ner.rs` to call `CandleBackend::from_bytes(...)`, caching the loaded model in a wasm-local `OnceCell` (single-instance — unlike native's multi-key `CANDLE_BACKEND_CACHE`, wasm32 only ever runs one model). Add a new `#[wasm_bindgen(js_name = "initCandleNer")]` export (mirroring `anon.rs`'s pattern) that JS calls once, after `CacheManager` downloads the three files, to hand bytes into the wasm module.
- **Task C:** fix the `call_injected_ner` positional-args vs. `NerInterface.ner(text, opts)` object-shape contract bug (independent of A/B — this affects the injected-JS path regardless of which NER backend is primary).
- **Task D (optional, F16 downcast):** parameterize `Encoder::from_buffered_safetensors`'s hardcoded `candle_core::DType::F32` to accept a runtime dtype, and pass `F16` on wasm32 to halve resident memory (1.17GB → ~585MB after loading). Does **not** reduce download size (source safetensors is F32-only). Real and buildable, but genuinely separate from Tasks A-C — can land independently.

### 3b. JS-side regex complement

- **New module** `packages/xberg-wasm-runtime/src/pii.ts`, ported from `mcp-server/src/redaction/detect.ts`'s `detectPii`/`mergeNerEntities`/`groupByCategory` — those functions are pure (regex + array operations, no Node-specific I/O), so they port to Node and browser unchanged. Deliberate duplication, not a new shared package — extracting one is optional future cleanup, not blocking (YAGNI).
- **Adapter:** field-name mismatch between the two packages' entity shapes — `mcp-server`'s `NerEntity` uses `category`/`confidence`; `xberg-wasm-runtime`'s `Entity` (`types.ts`) uses `label`/`score`. A thin mapping function bridges `ner.ts`'s output into `mergeNerEntities()`.
- **New exported function:** `detectPiiWithNer(text: string, nerResult: Entity[], filterCategories?: string[]): PiiFinding[]` — runs the 11 regex categories, maps NER entities via the adapter, merges via the existing overlap-resolution logic (higher-confidence wins on span overlap). Runs regardless of which NER path (injected JS, or eventually in-binary Candle via Component 3a) produced `nerResult`, or if neither did (`nerResult = []`) — regex-only PII detection still functions as a floor.

## Testing Strategy

- **Search:** real `better-sqlite3` FTS5 queries in Node (compiled in by default, no mocking). Browser/Worker FTS5 tested against the existing mocked-harness pattern (prerequisite plan's Task 5), plus the `pragma_compile_options()` pre-check described above. A fusion-correctness test: index one chunk that's vector-similar-but-textually-irrelevant and another that's textually-exact-but-vector-distant, confirm `hybrid` mode ranks a chunk that's moderately good on both above either extreme (classic RRF sanity check).
- **Embeddings:** real model download + inference in the validation spike (network calls accepted, matching this package's existing precedent for embedder/NER validation tasks). Dimension-correctness assertions updated to 1024.
- **PII:** `pii.ts`'s regex layer gets direct unit tests per category (ported from `mcp-server`'s existing `redaction.test.ts` coverage, not re-invented). The Candle wiring gets a `#[ignore]`d integration test gated on real downloaded model bytes (mirrors `xberg-gliner-candle/tests/smoke.rs`'s existing pattern — env-var-pointed model dir, skipped in normal CI) plus an always-on wasm32 build + `clippy -D warnings` gate as the cheap CI check — mirrors the three-gate discipline (tokenizers-on-wasm, candle-on-wasm, full integration) the original `ner-candle-wasm` plan already used successfully per its own progress log.

## Error Handling

- **Search:** if FTS5 isn't actually compiled into the vendored browser WASM build, `retrieve()` with `mode: "fulltext"|"hybrid"` fails loudly with a clear error. No silent fallback to vector-only — a silent fallback would present as hybrid search while actually running plain vector search, which is worse than an explicit error.
- **Embeddings:** BGE-M3 load failure at the validation-spike stage is a BLOCKED report with the exact error — no silent fallback to a different model without flagging the quality change to whoever's implementing.
- **PII:** this is the one place a silent degrade is *correct*, not a bug. `resolve_ner`'s existing cascade (injected JS → in-binary Candle → error) already models graceful degradation at the NER layer; the JS regex layer (3b) runs independently of which NER path succeeded, so structured PII (email/phone/SSN/etc.) is still caught even if both NER paths are unavailable.

## Non-Goals

- True quantized (Q4/Q8) Candle inference for the PII model — no `candle-transformers` quantized DebertaV2 exists; writing one is out of proportion to this plan.
- Matching `crates/xberg-rag`'s exact fusion/`candidate_multiplier` algorithm bit-for-bit for hybrid search — RRF is an accepted simplification (inherited from the prerequisite hybrid-fulltext spec).
- Wiring any of this into `xberg-wasm`'s public JS API surface for consumers beyond the injected-NER/store contracts already defined — that's sub-project D (browser UI) or E (MCP server)'s concern, not this plan's.
- Pursuing `SemplificaAI`'s ONNX/`gliner2-rs` path further — confirmed incompatible with `wasm32` by this repo's own established ORT policy, not revisited without new information.
- A shared package extracting `pii.ts`'s logic out of both `mcp-server` and `xberg-wasm-runtime` — deliberate duplication for now, noted as optional future cleanup.

## Open Questions for the Planning Phase

1. Does the vendored `wasm/sqlite-vec/sqlite3.wasm` build actually have `ENABLE_FTS5` compiled in? One-line check before planning Component 1's browser path in detail.
2. Real BGE-M3 q8 download size on Node/WASM paths — estimated ~550-600MB, needs confirming against the actual download in Task 1 of Component 2.
3. Does `model.safetensors`'s tensor naming actually carry the `encoder.` prefix `encoder.rs` expects? Config-schema match is strong evidence, not proof — Task A of Component 3a is the real check.
4. Should `EntityCategory` gain new variants for GLiNER2-PII's 42-label taxonomy beyond the existing `Custom(String)` escape hatch, or is `Custom` sufficient? Lean toward `Custom` (YAGNI) until a concrete caller needs typed variants for specific PII categories — flag for the plan-writer to decide, not resolved here.

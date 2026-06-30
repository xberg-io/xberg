# LoRA-Capable GLiNER2 + Privacy API Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Candle (pure-Rust) GLiNER2 backend with runtime PEFT LoRA adapter merge-at-load, then wire it into a unified `/v1/process` Privacy/GDPR API endpoint that reuses xberg's existing PII detection and rehydration engines.

**Architecture:** A new isolated `xberg-gliner-candle` crate ports anno's `gliner2_fastino_candle` Candle backend (DeBERTa-v2 encoder + 6 heads + PEFT LoRA merge), reusing xberg-gliner's already-shipped, already-tested V2 schema-prompt encoder instead of porting anno's separate ONNX-backend processor. `crates/xberg` gains `NerBackendKind::GlinerCandle` + an `AdapterConfig`, dispatched through a weight-bounded `moka` model cache (the base model is pinned; adapter-merged models are evicted by approximate RAM). A new `/v1/process` endpoint orchestrates extract → ner(adapter) → pii.detect → redact, reusing the existing `text/redaction/` engine unchanged.

**Tech Stack:** Rust 2024, Candle 0.11 (`candle-core`/`candle-nn`/`candle-transformers`, already pinned in the workspace), `safetensors` (new dep), `tokio::spawn_blocking` for CPU-bound inference, axum for HTTP.

**Spec:** `docs/superpowers/specs/2026-06-30-lora-privacy-api-design.md`

## Global Constraints

- Rust 2024 edition, `cargo fmt` + `clippy -D warnings`, zero warnings.
- Every `unsafe` block needs a `// SAFETY:` comment (the mmap'd safetensors loads in `encoder.rs`/heads already carry one in anno's source — preserve it verbatim).
- `Result<T, E>` with `thiserror` — never `.unwrap()`/`panic!` in library code paths (tests may use `.expect()`).
- New workspace member crate name: `xberg-gliner-candle`. Crate lib name: `xberg_gliner_candle`.
- Candle version floor: `candle-core`/`candle-nn`/`candle-transformers = "0.11"` (workspace-pinned already, do not bump).
- New `safetensors` workspace dependency: pin to `"0.4"` (matches anno's pin; compatible with candle-transformers 0.11's own internal safetensors usage).
- GLiNER2 Candle architecture constants `MAX_WIDTH = 8` and `MAX_COUNT = 20` are **model-architecture-fixed** (baked into the trained head shapes — `count_lstm`'s `pos_embedding` row count, `span_rep`'s reshape). They are NOT the same as xberg-gliner's span-mode `MAX_SPAN_WIDTH = 128` / `Parameters::max_width` default of `12` — do not unify or reuse those.
- The Candle pipeline's `build_span_idx` uses a different index convention than `xberg-gliner`'s `v2_tensor::build_span_idx` (0-indexed width vs 1-indexed width). They are NOT interchangeable — port the Candle-specific version from anno verbatim; do not "simplify" by reusing xberg's existing one.
- Scope: this plan ships `extract_ner` parity only (matches what `xberg-gliner`'s ONNX GLiNER2 v2 path already does). Anno's `extract_structure`, `classify`, `extract_with_label_descriptions`, and the `classifier` head are explicitly **out of scope** — do not port `heads/classifier.rs` or any `SchemaTask::{Structures,Classifications}` handling.
- Source for every ported file: `C:\Users\NMarchitecte\anno\crates\anno\src\backends\gliner2_fastino_candle\` (Candle-specific pieces) and `C:\Users\NMarchitecte\anno\crates\anno\src\backends\gliner2_fastino\pipeline.rs` (pure-logic decode helpers only — never the ONNX session code in that file).

---

## Part 1 — Engine crate `xberg-gliner-candle`

### Task 1: Scaffold the `xberg-gliner-candle` crate + workspace wiring

**Files:**
- Create: `crates/xberg-gliner-candle/Cargo.toml`
- Create: `crates/xberg-gliner-candle/src/lib.rs`
- Create: `crates/xberg-gliner-candle/src/error.rs`
- Modify: `Cargo.toml` (workspace members, `[workspace.dependencies]`, `[patch.crates-io]`)

**Interfaces:**
- Produces: `xberg_gliner_candle::{GlinerCandleError, Result}` — the crate's error type (exported as `GlinerCandleError`; no `Error` alias), consumed by every later task in Part 1.

- [ ] **Step 1: Add the workspace member + dependency entries**

In `Cargo.toml`, add to `members` (after `"crates/xberg-gliner",`):
```toml
    "crates/xberg-gliner-candle",
```

Add to `[workspace.dependencies]` (alongside the existing `xberg-gliner` line):
```toml
xberg-gliner-candle = { path = "./crates/xberg-gliner-candle", version = "1.0.0-rc.1", default-features = false }
safetensors = "0.4"
```

Add to `[patch.crates-io]`:
```toml
xberg-gliner-candle = { path = "crates/xberg-gliner-candle" }
```

- [ ] **Step 2: Write the crate manifest**

Create `crates/xberg-gliner-candle/Cargo.toml`:

```toml
[package]
name = "xberg-gliner-candle"
version.workspace = true
edition = "2024"
rust-version.workspace = true
authors.workspace = true
description = "Candle-based GLiNER2 inference with runtime PEFT LoRA adapter merge-at-load."
license = "Apache-2.0"
repository.workspace = true
homepage.workspace = true
keywords = ["nlp", "ner", "gliner", "candle", "lora"]
categories = ["text-processing"]

[lib]
name = "xberg_gliner_candle"
path = "src/lib.rs"

[features]
default = []
cuda = ["candle-core/cuda", "candle-nn/cuda", "candle-transformers/cuda"]
metal = ["candle-core/metal", "candle-nn/metal", "candle-transformers/metal"]

[dependencies]
candle-core = { workspace = true }
candle-nn = { workspace = true }
candle-transformers = { workspace = true }
ndarray = "0.17"
safetensors = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
tokenizers = { version = "0.23", default-features = false, features = ["fancy-regex"] }
xberg-gliner = { workspace = true, default-features = false }

[dev-dependencies]
tempfile = { workspace = true }

[lints]
workspace = true
```

- [ ] **Step 3: Write the error type**

Create `crates/xberg-gliner-candle/src/error.rs`:

```rust
use thiserror::Error;

/// Result type used by `xberg-gliner-candle`.
pub type Result<T> = std::result::Result<T, GlinerCandleError>;

/// Errors returned by Candle GLiNER2 inference, LoRA loading, and merge.
#[derive(Debug, Error)]
pub enum GlinerCandleError {
    /// Underlying Candle tensor/model error.
    #[error("candle error: {0}")]
    Candle(#[from] candle_core::Error),
    /// `xberg-gliner` prompt-encoding or decode error.
    #[error("gliner error: {0}")]
    Gliner(#[from] xberg_gliner::GlinerError),
    /// Filesystem or config-parsing error during model/adapter loading.
    #[error("backend error: {0}")]
    Backend(String),
}
```

- [ ] **Step 4: Write the lib root**

Create `crates/xberg-gliner-candle/src/lib.rs`:

```rust
//! Candle-based GLiNER2 inference with runtime PEFT LoRA adapter merge-at-load.
//!
//! Ported from `anno::backends::gliner2_fastino_candle`. Reuses
//! `xberg-gliner`'s already-shipped V2 schema-prompt encoder
//! (`encode_v2`/`V2Tokenizer`/`V2Splitter`) for tokenization and prompt
//! construction — only the Candle-specific encoder, heads, LoRA merge, and
//! decode logic are ported here.

mod decode;
mod encoder;
mod error;
mod heads;
mod lora;
mod pipeline;

pub use error::{GlinerCandleError, Result};

#[cfg(test)]
mod tests;
```

- [ ] **Step 5: Verify the scaffold compiles**

Run: `cargo check -p xberg-gliner-candle`
Expected: FAIL with `unresolved module` errors for `decode`, `encoder`, `heads`, `lora`, `pipeline` (they don't exist yet — confirms the scaffold wiring is correct and the next tasks have something to fill in).

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml crates/xberg-gliner-candle/
git commit -m "feat(gliner-candle): scaffold xberg-gliner-candle crate"
```

---

### Task 2: Expose minimal public surface in `xberg-gliner`

The Candle crate must construct `Span`/`SpanOutput` and reuse the greedy-merge decoder, and must build the same `[P]`/`[E]`/`[SEP_TEXT]` schema-prompt token sequence that `v2_preprocess::encode_v2` already builds (verified identical: `text_positions`/`schema_positions` in `crates/xberg-gliner/src/v2_preprocess.rs` produce exactly the `word_indices`/`schema_idx` shapes anno's Candle pipeline needs). Currently `Span::new`, `decode::greedy_search`, `V2Encoded`, `encode_v2`, `V2Tokenizer`, `V2Splitter`, and `PretokenizingTokenizer` are all `pub(crate)` — invisible outside `xberg-gliner`. Widen visibility and re-export.

**Files:**
- Modify: `crates/xberg-gliner/src/decode.rs:17` (`Span::new`), `crates/xberg-gliner/src/decode.rs:225` (`greedy_search`)
- Modify: `crates/xberg-gliner/src/v2_preprocess.rs:9` (`V2Encoded`), `:37` (`encode_v2`)
- Modify: `crates/xberg-gliner/src/v2_tokenizer.rs:7` (`PretokenizedEncoding`), `:15` (`PretokenizingTokenizer`), `:24` (`V2Tokenizer`), `:29` (`V2Tokenizer::from_file`)
- Modify: `crates/xberg-gliner/src/v2_splitter.rs:13` (`V2Splitter`), `:18` (`V2Splitter::new`), `:23` (`V2Splitter::split`)
- Modify: `crates/xberg-gliner/src/lib.rs` (pub use additions)
- Test: `crates/xberg-gliner/src/tests.rs`

**Interfaces:**
- Produces: `xberg_gliner::Span::new(sequence: usize, start: usize, end: usize, text: String, class: String, probability: f32) -> Result<Span>` (now `pub`), `xberg_gliner::decode::greedy_search(spans: &[Span], flat_ner: bool, dup_label: bool, multi_label: bool) -> Vec<Span>` (now `pub`), `xberg_gliner::V2Tokenizer::from_file<P: AsRef<Path>>(path: P) -> Result<V2Tokenizer>`, `xberg_gliner::V2Splitter::new() -> Result<V2Splitter>`, `xberg_gliner::encode_v2(text: &str, labels: &[String], tokenizer: &impl PretokenizingTokenizer, splitter: &V2Splitter) -> Result<V2Encoded>`, `V2Encoded { input_ids: Vec<i64>, text_positions: Vec<i64>, schema_positions: Vec<i64>, words: Vec<Token> }` (all fields now `pub`).

- [ ] **Step 1: Write the failing test (visibility compile-check)**

Add to `crates/xberg-gliner/src/tests.rs`:

```rust
#[test]
fn v2_prompt_encoding_surface_is_public() {
    // Compile-time check: these must be reachable as `xberg_gliner::*` from
    // outside the crate (xberg-gliner-candle depends on this surface).
    let splitter = crate::V2Splitter::new().expect("valid regex");
    let tokenizer_path = std::path::Path::new("nonexistent.json");
    let result = crate::V2Tokenizer::from_file(tokenizer_path);
    assert!(result.is_err(), "missing file must error, not panic");
    let _ = splitter; // keep both symbols referenced
}

#[test]
fn span_new_and_greedy_search_are_public() {
    let span = crate::Span::new(0, 0, 5, "hello".to_string(), "greeting".to_string(), 0.9)
        .expect("valid span");
    let merged = crate::decode::greedy_search(&[span], true, false, false);
    assert_eq!(merged.len(), 1);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p xberg-gliner v2_prompt_encoding_surface_is_public --no-run`
Expected: FAIL with `function \`V2Splitter::new\` is private` / `Span::new\` is private` (compile error, not a runtime failure).

- [ ] **Step 3: Widen visibility**

In `crates/xberg-gliner/src/decode.rs`, change:
```rust
impl Span {
    pub(crate) fn new(
```
to:
```rust
impl Span {
    pub fn new(
```

And change:
```rust
pub(crate) fn greedy_search(spans: &[Span], flat_ner: bool, dup_label: bool, multi_label: bool) -> Vec<Span> {
```
to:
```rust
pub fn greedy_search(spans: &[Span], flat_ner: bool, dup_label: bool, multi_label: bool) -> Vec<Span> {
```

In `crates/xberg-gliner/src/v2_preprocess.rs`, change:
```rust
pub(crate) struct V2Encoded {
    pub(crate) input_ids: Vec<i64>,
    pub(crate) text_positions: Vec<i64>,
    pub(crate) schema_positions: Vec<i64>,
    pub(crate) words: Vec<Token>,
}
```
to:
```rust
pub struct V2Encoded {
    pub input_ids: Vec<i64>,
    pub text_positions: Vec<i64>,
    pub schema_positions: Vec<i64>,
    pub words: Vec<Token>,
}
```
and:
```rust
pub(crate) fn encode_v2(
```
to:
```rust
pub fn encode_v2(
```

In `crates/xberg-gliner/src/v2_tokenizer.rs`, change every `pub(crate)` on `PretokenizedEncoding` (struct + both fields), `PretokenizingTokenizer` (trait), `V2Tokenizer` (struct), and `V2Tokenizer::from_file` to `pub`. Leave the `impl PretokenizingTokenizer for V2Tokenizer` block's `encode_pretokenized` as-is (trait method visibility follows the trait).

In `crates/xberg-gliner/src/v2_splitter.rs`, change `pub(crate)` on `V2Splitter` (struct), `V2Splitter::new`, and `V2Splitter::split` to `pub`. Leave `V2_SPLITTER_REGEX` as `pub(crate)` (internal detail, not needed externally).

In `crates/xberg-gliner/src/lib.rs`, add after the existing `pub use` block:
```rust
pub use v2_preprocess::{V2Encoded, encode_v2};
pub use v2_splitter::V2Splitter;
pub use v2_tokenizer::{PretokenizedEncoding, PretokenizingTokenizer, V2Tokenizer};
```
and remove `v2_preprocess`, `v2_splitter`, `v2_tokenizer` from the `pub(crate) use` lines if present (they are currently plain `mod` declarations, so no change needed there — only the `pub use` block changes).

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p xberg-gliner v2_prompt_encoding_surface_is_public span_new_and_greedy_search_are_public -- --nocapture`
Expected: both tests PASS.

- [ ] **Step 5: Run the full xberg-gliner test suite to confirm no regression**

Run: `cargo test -p xberg-gliner`
Expected: all existing tests still PASS (visibility widening is additive, never breaks existing callers).

- [ ] **Step 6: Commit**

```bash
git add crates/xberg-gliner/src/decode.rs crates/xberg-gliner/src/v2_preprocess.rs crates/xberg-gliner/src/v2_tokenizer.rs crates/xberg-gliner/src/v2_splitter.rs crates/xberg-gliner/src/lib.rs crates/xberg-gliner/src/tests.rs
git commit -m "feat(gliner): expose V2 prompt encoding and Span construction for cross-engine reuse"
```

---

### Task 3: Port `encoder.rs` (DeBERTa-v2 wrapper)

Verbatim port from `C:\Users\NMarchitecte\anno\crates\anno\src\backends\gliner2_fastino_candle\encoder.rs`. Substitution table: `crate::Result<T>` → `crate::Result<T>` (same name, different crate — no change needed since both crates define a local `Result` alias), `crate::Error::Backend(format!(...))` → `crate::GlinerCandleError::Backend(format!(...))`.

**Files:**
- Create: `crates/xberg-gliner-candle/src/encoder.rs`
- Modify: `crates/xberg-gliner-candle/src/lib.rs` (already declares `mod encoder;` from Task 1)

**Interfaces:**
- Consumes: `candle_transformers::models::debertav2::{Config as DebertaV2Config, DebertaV2Model}` (workspace-pinned 0.11, API-verified identical to anno's 0.10 usage).
- Produces: `Encoder::from_safetensors(weights_path: &Path, config_path: &Path, device: &Device) -> Result<Encoder>`, `Encoder::from_var_builder(vb: VarBuilder<'_>, config: &DebertaV2Config) -> Result<Encoder>`, `Encoder::forward(&self, input_ids: &Tensor, attention_mask: &Tensor, token_type_ids: Option<&Tensor>) -> candle_core::Result<Tensor>`, `Encoder::hidden_size(&self) -> usize`, `pub(crate) config: DebertaV2Config` field (read by Task 7's pipeline for `max_position_embeddings`).

- [ ] **Step 1: Write the failing test**

Add to `crates/xberg-gliner-candle/src/tests.rs` (create the file):

```rust
use candle_core::Device;

#[test]
fn encoder_from_safetensors_rejects_missing_weights() {
    let dir = tempfile::tempdir().expect("tempdir");
    let weights = dir.path().join("model.safetensors");
    let config = dir.path().join("config.json");
    let err = crate::encoder::Encoder::from_safetensors(&weights, &config, &Device::Cpu)
        .expect_err("missing files must error, not panic");
    assert!(err.to_string().contains("encoder config read") || err.to_string().contains("backend error"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p xberg-gliner-candle encoder_from_safetensors_rejects_missing_weights --no-run`
Expected: FAIL — `encoder` module has no `Encoder` type yet (`error[E0433]`).

- [ ] **Step 3: Write the implementation**

Create `crates/xberg-gliner-candle/src/encoder.rs` — copy verbatim from the anno source above, with these exact changes:
- Replace every `crate::Error::Backend(` with `crate::GlinerCandleError::Backend(`.
- Replace every `crate::Result<` with `crate::Result<` (no change — already matches).
- The struct fields `model: DebertaV2Model` and `config: DebertaV2Config` change from `pub(crate)` to `pub(crate)` (no change — both crates are workspace-local, keep `pub(crate)` since only `pipeline.rs`/`lora.rs` in this same crate need them).
- Keep the `// SAFETY:` comment above `VarBuilder::from_mmaped_safetensors` verbatim — it documents the mmap invariant correctly for this new crate too.

The full ported file (101 lines, matching anno's `encoder.rs` 1:1 except the two `crate::Error` → `crate::GlinerCandleError` substitutions):

```rust
//! Thin wrapper over `candle_transformers::models::debertav2::DebertaV2Model`.

use std::path::Path;

use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::debertav2::{Config as DebertaV2Config, DebertaV2Model};

/// Wrapped DeBERTa-v2/v3 encoder. Loaded from safetensors + config.json
/// at the model snapshot root.
pub struct Encoder {
    pub(crate) model: DebertaV2Model,
    pub(crate) config: DebertaV2Config,
}

impl Encoder {
    /// Load the encoder from a `model.safetensors` + `config.json` pair.
    pub fn from_safetensors(
        weights_path: &Path,
        config_path: &Path,
        device: &Device,
    ) -> crate::Result<Self> {
        let cfg_str = std::fs::read_to_string(config_path).map_err(|e| {
            crate::GlinerCandleError::Backend(format!(
                "encoder config read {}: {e}",
                config_path.display()
            ))
        })?;
        let config: DebertaV2Config = serde_json::from_str(&cfg_str).map_err(|e| {
            crate::GlinerCandleError::Backend(format!(
                "encoder config parse {}: {e}",
                config_path.display()
            ))
        })?;

        // SAFETY: VarBuilder::from_mmaped_safetensors mmap-reads the weights
        // file. Safe as long as the file isn't mutated under us — Candle's
        // standard pattern.
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[weights_path], candle_core::DType::F32, device)
        }
        .map_err(|e| crate::GlinerCandleError::Backend(format!("encoder safetensors: {e}")))?;

        // GLiNER2 stores all encoder tensors under the `encoder.` prefix
        // (e.g. `encoder.embeddings.word_embeddings.weight`). DebertaV2Model
        // expects them at root, so scope into the prefix.
        let model = DebertaV2Model::load(vb.pp("encoder"), &config)
            .map_err(|e| crate::GlinerCandleError::Backend(format!("encoder DebertaV2Model::load: {e}")))?;

        Ok(Self { model, config })
    }

    /// Load the encoder from an already-built [`VarBuilder`] + parsed config.
    ///
    /// Used by [`crate::Gliner2Candle::load_adapter`] after the LoRA merge
    /// has produced a `HashMap<String, Tensor>` that's wrapped into a
    /// `VarBuilder::from_tensors`. The caller is responsible for scoping
    /// into the `encoder.` prefix; this constructor calls `DebertaV2Model::load`
    /// directly on the provided VarBuilder.
    pub fn from_var_builder(vb: VarBuilder<'_>, config: &DebertaV2Config) -> crate::Result<Self> {
        let model = DebertaV2Model::load(vb, config).map_err(|e| {
            crate::GlinerCandleError::Backend(format!("encoder DebertaV2Model::load (vb): {e}"))
        })?;
        Ok(Self {
            model,
            config: config.clone(),
        })
    }

    /// Run the encoder forward pass. Returns hidden states of shape
    /// `[batch, seq_len, hidden_size]`.
    pub fn forward(
        &self,
        input_ids: &Tensor,
        attention_mask: &Tensor,
        token_type_ids: Option<&Tensor>,
    ) -> candle_core::Result<Tensor> {
        self.model.forward(
            input_ids,
            token_type_ids.cloned(),
            Some(attention_mask.clone()),
        )
    }

    /// Hidden size (read from config).
    pub fn hidden_size(&self) -> usize {
        self.config.hidden_size
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p xberg-gliner-candle encoder_from_safetensors_rejects_missing_weights -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/xberg-gliner-candle/src/encoder.rs crates/xberg-gliner-candle/src/tests.rs
git commit -m "feat(gliner-candle): port DeBERTa-v2 encoder wrapper from anno"
```

---

### Task 4: Port `lora.rs` (PEFT LoRA loading + merge-at-load)

This is the differentiator and the highest-risk numeric code in the port. Verbatim port from `C:\Users\NMarchitecte\anno\crates\anno\src\backends\gliner2_fastino_candle\lora.rs`, including its 2 unit tests (`parse_lora_key_strict`, `apply_lora_delta_shape`) — these are CI-safe (no model files needed) and are the dominant unit-test coverage for the merge math per the spec's validation strategy.

**Files:**
- Create: `crates/xberg-gliner-candle/src/lora.rs`

**Interfaces:**
- Consumes: `crate::GlinerCandleError`, `crate::Result`.
- Produces: `LoraConfig { r: usize, lora_alpha: f64, target_modules: Option<Vec<String>>, base_model_name_or_path: Option<String>, fan_in_fan_out: bool }`, `LoraModule { lora_a: Tensor, lora_b: Tensor }`, `LoraAdapter { config: LoraConfig, modules: HashMap<String, LoraModule> }`, `LoraAdapter::load(adapter_dir: &Path, device: &Device) -> Result<LoraAdapter>`, `pub(crate) fn merge_into_base(base_safetensors: &Path, adapter: &LoraAdapter, device: &Device) -> Result<HashMap<String, Tensor>>` (consumed by Task 8's `load_adapter`).

- [ ] **Step 1: Write the failing tests**

These are the two tests already in anno's source — write them now, before the implementation, per TDD:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_lora_key_strict() {
        let (path, slot) =
            parse_lora_key("base_model.model.encoder.layer.0.attention.self.query.lora_A.weight")
                .expect("valid PEFT key should parse");
        assert_eq!(path, "encoder.layer.0.attention.self.query");
        assert!(matches!(slot, LoraSlot::A));

        let (path_b, slot_b) =
            parse_lora_key("base_model.model.encoder.layer.0.attention.self.query.lora_B.weight")
                .expect("valid PEFT key (B) should parse");
        assert_eq!(path_b, "encoder.layer.0.attention.self.query");
        assert!(matches!(slot_b, LoraSlot::B));

        assert!(
            parse_lora_key("encoder.layer.0.attention.self.query.lora_A.weight").is_err(),
            "missing 'base_model.model.' prefix should fail"
        );
        assert!(
            parse_lora_key("base_model.model.encoder.layer.0.weight").is_err(),
            "missing '.lora_A.weight'/'.lora_B.weight' suffix should fail"
        );
    }

    #[test]
    fn apply_lora_delta_shape() {
        let device = candle_core::Device::Cpu;
        let base = candle_core::Tensor::zeros((4, 3), candle_core::DType::F32, &device).unwrap();
        let lora_a = candle_core::Tensor::ones((2, 3), candle_core::DType::F32, &device).unwrap();
        let lora_b = candle_core::Tensor::ones((4, 2), candle_core::DType::F32, &device).unwrap();
        let merged = apply_lora_delta(&base, &lora_a, &lora_b, 0.5, false).unwrap();
        assert_eq!(merged.shape().dims(), &[4, 3]);
        let v = merged.flatten_all().unwrap().to_vec1::<f32>().unwrap();
        for x in v {
            assert!((x - 1.0).abs() < 1e-6);
        }
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p xberg-gliner-candle parse_lora_key_strict apply_lora_delta_shape --no-run`
Expected: FAIL — `lora` module doesn't exist yet / `parse_lora_key` unresolved.

- [ ] **Step 3: Write the implementation**

Create `crates/xberg-gliner-candle/src/lora.rs` — copy verbatim from the anno source (377 lines), with this substitution table applied throughout:
- `crate::Result<Self>` → unchanged (same name in this crate too).
- `crate::Error::Backend(format!(...))` → `crate::GlinerCandleError::Backend(format!(...))` (15 occurrences).
- `pub(crate) fn merge_into_base` → unchanged (stays `pub(crate)`, only `mod.rs`'s `load_adapter` in this same crate calls it).
- `pub struct LoraConfig`, `pub struct LoraModule`, `pub struct LoraAdapter`, `impl LoraAdapter { pub fn load(...) }` → unchanged, already `pub`.
- The module doc comment, the `parse_lora_key`/`LoraSlot`/`decode_view`/`apply_lora_delta` private helper functions, and the `_dtype_marker` marker function → copy verbatim, no changes (they only reference `candle_core`, `safetensors`, `std::collections::HashMap`, `std::path::Path` — no anno-specific types).

Append the two tests from Step 1 at the bottom of the file (already anno's own test module, copied verbatim).

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p xberg-gliner-candle parse_lora_key_strict apply_lora_delta_shape -- --nocapture`
Expected: both PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/xberg-gliner-candle/src/lora.rs
git commit -m "feat(gliner-candle): port PEFT LoRA loading and merge-at-load from anno"
```

---

### Task 5a: Port the 3 parameter-free heads (`token_gather`, `schema_gather`, `scorer`)

Source: `C:\Users\NMarchitecte\anno\crates\anno\src\backends\gliner2_fastino_candle\heads\{token_gather,schema_gather,scorer}.rs`. These three have zero anno-specific dependencies (only `candle_core`/`candle_nn`) — copy verbatim, no substitutions needed.

**Files:**
- Create: `crates/xberg-gliner-candle/src/heads/mod.rs`
- Create: `crates/xberg-gliner-candle/src/heads/token_gather.rs`
- Create: `crates/xberg-gliner-candle/src/heads/schema_gather.rs`
- Create: `crates/xberg-gliner-candle/src/heads/scorer.rs`
- Modify: `crates/xberg-gliner-candle/src/lib.rs` (already declares `mod heads;` from Task 1)

**Interfaces:**
- Produces: `TokenGather::forward(&self, hidden_states: &Tensor, word_indices: &Tensor) -> candle_core::Result<Tensor>`, `SchemaGather::forward(&self, hidden_states: &Tensor, schema_indices: &Tensor) -> candle_core::Result<SchemaGatherOutput>` where `SchemaGatherOutput { pc_emb: Tensor, field_embs: Tensor }`, `Scorer::forward(&self, span_rep: &Tensor, struct_proj: &Tensor) -> candle_core::Result<Tensor>`, `pub(crate) const MAX_WIDTH: usize = 8` (consumed by Task 5b's `span_rep.rs` and Task 6's `decode.rs`).

- [ ] **Step 1: Write the failing test**

Add to `crates/xberg-gliner-candle/src/tests.rs`:

```rust
#[test]
fn token_gather_selects_word_start_positions() {
    use candle_core::{Device, Tensor};
    let device = Device::Cpu;
    // hidden_states: [1, 3, 2] — 3 tokens, hidden size 2.
    let hidden = Tensor::from_vec(vec![1f32, 1., 2., 2., 3., 3.], (1, 3, 2), &device).unwrap();
    let word_indices = Tensor::from_vec(vec![0u32, 2u32], (2,), &device).unwrap();
    let out = crate::heads::token_gather::TokenGather
        .forward(&hidden, &word_indices)
        .unwrap();
    assert_eq!(out.dims(), &[1, 2, 2]);
    let v = out.flatten_all().unwrap().to_vec1::<f32>().unwrap();
    assert_eq!(v, vec![1., 1., 3., 3.]);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p xberg-gliner-candle token_gather_selects_word_start_positions --no-run`
Expected: FAIL — `heads` module/`token_gather` submodule don't exist.

- [ ] **Step 3: Write the implementations**

Create `crates/xberg-gliner-candle/src/heads/token_gather.rs` (verbatim, 24 lines):

```rust
//! `token_gather` — non-parametric utility head.

use candle_core::{Result, Tensor};

/// Stateless token-gather head. Holds no parameters.
pub struct TokenGather;

impl TokenGather {
    /// `hidden_states[0, word_indices, :]` → `[1, num_words, H]`.
    pub fn forward(&self, hidden_states: &Tensor, word_indices: &Tensor) -> Result<Tensor> {
        let h = hidden_states.squeeze(0)?; // [S, H]
        let gathered = h.index_select(word_indices, 0)?; // [num_words, H]
        gathered.unsqueeze(0) // [1, num_words, H]
    }
}
```

Create `crates/xberg-gliner-candle/src/heads/schema_gather.rs` (verbatim, 50 lines):

```rust
//! `schema_gather` — non-parametric utility head.

use candle_core::{Result, Tensor};

/// Stateless schema-gather head. Holds no parameters.
pub struct SchemaGather;

/// Result of [`SchemaGather::forward`].
pub struct SchemaGatherOutput {
    /// `[1, H]` — the `[P]` token's hidden state (prompt context).
    pub pc_emb: Tensor,
    /// `[F, H]` — per-field / per-label embeddings.
    pub field_embs: Tensor,
}

impl SchemaGather {
    /// `schema_indices` includes the `[P]` index first, followed by all
    /// per-field `[E]` indices — matches `schema_positions` order from
    /// `xberg_gliner::encode_v2`.
    pub fn forward(
        &self,
        hidden_states: &Tensor,  // [1, S, H]
        schema_indices: &Tensor, // [num_special]
    ) -> Result<SchemaGatherOutput> {
        let h = hidden_states.squeeze(0)?; // [S, H]
        let all = h.index_select(schema_indices, 0)?; // [num_special, H]

        let pc_emb = all.narrow(0, 0, 1)?; // [1, H]
        let n = all.dim(0)?;
        let hidden_dim = all.dim(1)?;
        let field_embs = if n > 1 {
            all.narrow(0, 1, n - 1)? // [F, H]
        } else {
            Tensor::zeros((0, hidden_dim), all.dtype(), all.device())?
        };

        Ok(SchemaGatherOutput { pc_emb, field_embs })
    }
}
```

Create `crates/xberg-gliner-candle/src/heads/scorer.rs` (verbatim, 44 lines):

```rust
//! `scorer` — non-parametric utility head.
//!
//! `scores[b, p, l, k] = sigmoid(Σ_d span_rep[l, k, d] * struct_proj[b, p, d])`,
//! computed as a single matmul + reshape + sigmoid.

use candle_core::{Result, Tensor};

/// Stateless scorer head. Holds no parameters.
pub struct Scorer;

impl Scorer {
    /// * `span_rep`: `[T, W, H]` (per-sample slice of `[1, T, W, H]`).
    /// * `struct_proj`: `[count, F, H]`.
    /// Returns `[count, F, T, W]` sigmoid scores.
    pub fn forward(&self, span_rep: &Tensor, struct_proj: &Tensor) -> Result<Tensor> {
        let (t, w, h) = span_rep.dims3()?;
        let (count, f, h2) = struct_proj.dims3()?;
        if h != h2 {
            return Err(candle_core::Error::Msg(format!(
                "scorer: hidden mismatch {h} vs {h2}"
            )));
        }

        let span_flat = span_rep.reshape(((), h))?.contiguous()?; // [T*W, H]
        let struct_flat = struct_proj.reshape(((), h))?.contiguous()?; // [count*F, H]
        let scores_flat = struct_flat.matmul(&span_flat.transpose(0, 1)?.contiguous()?)?;
        let scores = scores_flat.reshape((count, f, t, w))?;

        candle_nn::ops::sigmoid(&scores)
    }
}
```

Create `crates/xberg-gliner-candle/src/heads/mod.rs` (partial — `AllHeads` struct completed in Task 5b; this step only adds the module declarations and the shared constant):

```rust
//! GLiNER2 inference heads (Candle).
//!
//! `token_gather`, `schema_gather`, `scorer` are parameter-free utilities.
//! `span_rep`, `count_pred`, `count_lstm` are parametric (Task 5b). The
//! `classifier` head from anno is intentionally NOT ported — this crate
//! ships `extract_ner` parity only (see plan Global Constraints).

pub mod count_lstm;
pub mod count_pred;
pub mod schema_gather;
pub mod scorer;
pub mod span_rep;
pub mod token_gather;

/// Maximum span width baked into the v2 Candle heads' trained weights
/// (`span_rep`'s reshape, `scorer`'s axis sizing). Model-architecture-fixed —
/// see Global Constraints.
pub(crate) const MAX_WIDTH: usize = 8;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p xberg-gliner-candle token_gather_selects_word_start_positions -- --nocapture`
Expected: PASS. (`cargo check -p xberg-gliner-candle` will still fail at this point — `span_rep`/`count_pred`/`count_lstm` modules are declared but not yet created; that's expected and resolved in Task 5b.)

- [ ] **Step 5: Commit**

```bash
git add crates/xberg-gliner-candle/src/heads/mod.rs crates/xberg-gliner-candle/src/heads/token_gather.rs crates/xberg-gliner-candle/src/heads/schema_gather.rs crates/xberg-gliner-candle/src/heads/scorer.rs crates/xberg-gliner-candle/src/tests.rs
git commit -m "feat(gliner-candle): port parameter-free heads (token_gather, schema_gather, scorer)"
```

---

### Task 5b: Port the 3 parametric heads (`span_rep`, `count_pred`, `count_lstm`) + complete `AllHeads`

Source: same directory, `{span_rep,count_pred,count_lstm}.rs`. These load weights via `VarBuilder` — no anno-specific types, copy verbatim. One import changes: `span_rep.rs`'s `use crate::backends::gliner2_fastino::pipeline::MAX_WIDTH;` becomes `use super::MAX_WIDTH;` (the constant Task 5a defined in `heads/mod.rs`).

**Files:**
- Create: `crates/xberg-gliner-candle/src/heads/span_rep.rs`
- Create: `crates/xberg-gliner-candle/src/heads/count_pred.rs`
- Create: `crates/xberg-gliner-candle/src/heads/count_lstm.rs`
- Modify: `crates/xberg-gliner-candle/src/heads/mod.rs` (add `AllHeads`)

**Interfaces:**
- Produces: `SpanRep::from_var_builder(vb: &VarBuilder) -> candle_core::Result<SpanRep>`, `SpanRep::forward(&self, text_emb: &Tensor, span_idx: &Tensor) -> candle_core::Result<Tensor>` (returns `[1, T, MAX_WIDTH, 768]`); `CountPred::from_var_builder(vb: &VarBuilder) -> candle_core::Result<CountPred>`, `CountPred::forward(&self, p_emb: &Tensor) -> candle_core::Result<usize>`; `count_lstm::MAX_COUNT: usize = 20`, `CountLstmFixed::from_var_builder(vb: &VarBuilder, device: &Device) -> candle_core::Result<CountLstmFixed>`, `CountLstmFixed::forward(&self, field_embs: &Tensor, pred_count: usize, device: &Device) -> candle_core::Result<Tensor>` (returns `[L, F, 768]`); `AllHeads { span_rep, count_lstm, count_pred }`, `AllHeads::from_safetensors(weights_path: &Path, device: &Device) -> crate::Result<AllHeads>`, `AllHeads::from_var_builder(vb: VarBuilder<'_>, device: &Device) -> crate::Result<AllHeads>` (consumed by Task 8's `load_adapter`/`unload_adapter`).

- [ ] **Step 1: Write the failing test**

Add to `crates/xberg-gliner-candle/src/tests.rs`:

```rust
#[test]
fn count_pred_clamps_argmax_to_19() {
    use candle_core::{Device, Tensor};
    use candle_nn::VarBuilder;
    let device = Device::Cpu;
    let varmap = candle_nn::VarMap::new();
    let vb = VarBuilder::from_varmap(&varmap, candle_core::DType::F32, &device);
    let head = crate::heads::count_pred::CountPred::from_var_builder(&vb.pp("count_pred"))
        .expect("zero-initialised weights still build a valid head");
    let p_emb = Tensor::zeros((1, 768), candle_core::DType::F32, &device).unwrap();
    let pred = head.forward(&p_emb).expect("forward must not panic on zero weights");
    assert!(pred < 20, "argmax must be clamped to [0, 19], got {pred}");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p xberg-gliner-candle count_pred_clamps_argmax_to_19 --no-run`
Expected: FAIL — `heads::count_pred` module doesn't exist.

- [ ] **Step 3: Write the implementations**

Create `crates/xberg-gliner-candle/src/heads/span_rep.rs` (122 lines, verbatim except the `MAX_WIDTH` import):

```rust
//! `span_rep` head — three 2-layer MLPs (`project_start`, `project_end`,
//! `out_project`) with ReLU between layers.

use candle_core::{IndexOp, Tensor};
use candle_nn::{linear, Linear, Module, VarBuilder};

use super::MAX_WIDTH;

/// SpanMarkerV0 — builds per-(start, end) span representations from
/// per-token hidden states.
pub struct SpanRep {
    project_start_0: Linear,
    project_start_3: Linear,
    project_end_0: Linear,
    project_end_3: Linear,
    out_project_0: Linear,
    out_project_3: Linear,
}

impl SpanRep {
    /// Construct from a `VarBuilder` rooted at `span_rep.span_rep_layer`.
    pub fn from_var_builder(vb: &VarBuilder) -> candle_core::Result<Self> {
        let project_start_0 = linear(768, 3072, vb.pp("project_start.0"))?;
        let project_start_3 = linear(3072, 768, vb.pp("project_start.3"))?;
        let project_end_0 = linear(768, 3072, vb.pp("project_end.0"))?;
        let project_end_3 = linear(3072, 768, vb.pp("project_end.3"))?;
        let out_project_0 = linear(1536, 3072, vb.pp("out_project.0"))?;
        let out_project_3 = linear(3072, 768, vb.pp("out_project.3"))?;

        Ok(Self {
            project_start_0,
            project_start_3,
            project_end_0,
            project_end_3,
            out_project_0,
            out_project_3,
        })
    }

    /// * `text_emb` — `[1, T, 768]` per-word pooled hidden states.
    /// * `span_idx` — `[1, T*MAX_WIDTH, 2]` int64 (start, end) indices.
    /// Returns `[1, T, MAX_WIDTH, 768]`.
    pub fn forward(&self, text_emb: &Tensor, span_idx: &Tensor) -> candle_core::Result<Tensor> {
        let (b, t, _h) = text_emb.dims3()?;
        debug_assert_eq!(b, 1, "SpanRep currently assumes batch=1");

        let start_rep = self
            .project_start_3
            .forward(&self.project_start_0.forward(text_emb)?.relu()?)?;
        let end_rep = self
            .project_end_3
            .forward(&self.project_end_0.forward(text_emb)?.relu()?)?;

        let start_idx = span_idx.i((0, .., 0))?.contiguous()?; // [T*W]
        let end_idx = span_idx.i((0, .., 1))?.contiguous()?; // [T*W]

        let start_rep_2d = start_rep.squeeze(0)?; // [T, 768]
        let end_rep_2d = end_rep.squeeze(0)?; // [T, 768]

        let start_at = start_rep_2d.index_select(&start_idx, 0)?; // [T*W, 768]
        let end_at = end_rep_2d.index_select(&end_idx, 0)?; // [T*W, 768]

        let cat = Tensor::cat(&[&start_at, &end_at], 1)?.relu()?;

        let out_2d = self
            .out_project_3
            .forward(&self.out_project_0.forward(&cat)?.relu()?)?; // [T*W, 768]

        out_2d.reshape((1, t, MAX_WIDTH, 768))
    }
}
```

Create `crates/xberg-gliner-candle/src/heads/count_pred.rs` (verbatim, 86 lines):

```rust
//! `count_pred` head — 2-layer MLP over the pooled prompt embedding.

use candle_core::Tensor;
use candle_nn::{linear, Linear, Module, VarBuilder};

/// Maximum count class index. Output dim is `MAX_COUNT_CLASSES = 20`,
/// so valid argmax results fall in `[0, 19]`.
const MAX_COUNT_CLASSES: usize = 20;

/// `count_pred` — 2-layer MLP that predicts a count class given the
/// pooled prompt embedding.
pub struct CountPred {
    linear_0: Linear,
    linear_2: Linear,
}

impl CountPred {
    /// Construct from a `VarBuilder` rooted at `count_pred`.
    pub fn from_var_builder(vb: &VarBuilder) -> candle_core::Result<Self> {
        let linear_0 = linear(768, 1536, vb.pp("0"))?;
        let linear_2 = linear(1536, MAX_COUNT_CLASSES, vb.pp("2"))?;
        Ok(Self { linear_0, linear_2 })
    }

    /// * `p_emb` — pooled prompt embedding `[1, 768]` (or `[768]`).
    /// Returns the predicted count as a host-side `usize`, clamped to `[0, 19]`.
    pub fn forward(&self, p_emb: &Tensor) -> candle_core::Result<usize> {
        let p_emb_2d = match p_emb.rank() {
            1 => p_emb.reshape((1, 768))?,
            2 => p_emb.clone(),
            other => {
                return Err(candle_core::Error::Msg(format!(
                    "count_pred::forward: expected p_emb rank 1 or 2, got {other}"
                )));
            }
        };

        let h1 = self.linear_0.forward(&p_emb_2d)?.relu()?;
        let logits = self.linear_2.forward(&h1)?;

        let argmax = logits.argmax(1)?; // [1], dtype u32
        let argmax_scalar = argmax.reshape(())?.to_scalar::<u32>()? as usize;

        Ok(argmax_scalar.min(MAX_COUNT_CLASSES - 1))
    }
}
```

Create `crates/xberg-gliner-candle/src/heads/count_lstm.rs` (verbatim, 149 lines — copy from the anno source read earlier in this session; full content matches anno's `heads/count_lstm.rs` 1:1, no substitutions needed since it only references `candle_core`/`candle_nn`). Key signatures: `pub const MAX_COUNT: usize = 20;`, `struct CountLstmFixed { gru: GRU, pos_embedding: Embedding, projector_0: Linear, projector_2: Linear, device: Device }`, `from_var_builder(vb: &VarBuilder, device: &Device) -> candle_core::Result<Self>`, `forward(&self, field_embs: &Tensor, pred_count: usize, device: &Device) -> candle_core::Result<Tensor>`.

Update `crates/xberg-gliner-candle/src/heads/mod.rs` to add `AllHeads` below the existing constant:

```rust
use std::path::Path;

use candle_core::{DType, Device};
use candle_nn::VarBuilder;

/// Container for the three parametric inference heads.
pub struct AllHeads {
    pub span_rep: span_rep::SpanRep,
    pub count_lstm: count_lstm::CountLstmFixed,
    pub count_pred: count_pred::CountPred,
}

impl AllHeads {
    /// Load all heads' weights from a single safetensors file.
    ///
    /// Expects the `fastino/gliner2-multi-v1` key layout:
    ///   - `span_rep.span_rep_layer.*`
    ///   - `count_embed.*`
    ///   - `count_pred.*`
    pub fn from_safetensors(weights_path: &Path, device: &Device) -> crate::Result<Self> {
        // SAFETY: mmap-reads the weights file; safe as long as it isn't
        // mutated under us — matches `encoder::Encoder`'s pattern.
        let vb =
            unsafe { VarBuilder::from_mmaped_safetensors(&[weights_path], DType::F32, device) }
                .map_err(|e| crate::GlinerCandleError::Backend(format!("heads safetensors: {e}")))?;
        Self::load(vb, device)
    }

    /// Load all heads from an already-built [`VarBuilder`] (post-LoRA-merge path).
    pub fn from_var_builder(vb: VarBuilder<'_>, device: &Device) -> crate::Result<Self> {
        Self::load(vb, device)
    }

    fn load(vb: VarBuilder<'_>, device: &Device) -> crate::Result<Self> {
        let span_rep = span_rep::SpanRep::from_var_builder(&vb.pp("span_rep").pp("span_rep_layer"))
            .map_err(|e| crate::GlinerCandleError::Backend(format!("span_rep load: {e}")))?;
        let count_lstm = count_lstm::CountLstmFixed::from_var_builder(&vb.pp("count_embed"), device)
            .map_err(|e| crate::GlinerCandleError::Backend(format!("count_embed load: {e}")))?;
        let count_pred = count_pred::CountPred::from_var_builder(&vb.pp("count_pred"))
            .map_err(|e| crate::GlinerCandleError::Backend(format!("count_pred load: {e}")))?;

        Ok(Self {
            span_rep,
            count_lstm,
            count_pred,
        })
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p xberg-gliner-candle count_pred_clamps_argmax_to_19 -- --nocapture`
Expected: PASS.

Run: `cargo check -p xberg-gliner-candle`
Expected: still FAILS — `decode` and `pipeline` modules remain unimplemented (Tasks 6–7). Confirms only those two modules are left.

- [ ] **Step 5: Commit**

```bash
git add crates/xberg-gliner-candle/src/heads/ crates/xberg-gliner-candle/src/tests.rs
git commit -m "feat(gliner-candle): port parametric heads and assemble AllHeads"
```

---

### Task 6: Write `decode.rs` (span-index construction + score decoding)

NOT a verbatim port. Anno's decode logic (`gliner2_fastino::pipeline::{build_span_idx, ScorerOutput, decode_entities, decode_entities_with_thresholds}`) targets anno's own `Entity`/char-offset/`greedy_nms` types. This task re-implements the same numeric decode loop targeting `xberg_gliner::Span` (byte offsets — no char-offset conversion needed, simpler than anno's version) and `xberg_gliner::decode::greedy_search` (exposed in Task 2), sorting candidates by offset first per that function's contract.

**Files:**
- Create: `crates/xberg-gliner-candle/src/decode.rs`

**Interfaces:**
- Consumes: `xberg_gliner::{Span, Token, decode::greedy_search}` (Task 2), `heads::MAX_WIDTH` (Task 5a), `heads::count_lstm::MAX_COUNT` (Task 5b).
- Produces: `pub(crate) const MAX_WIDTH: usize` and `pub(crate) const MAX_COUNT: usize` (re-exports), `pub(crate) struct ScorerOutput { pub scores: ndarray::Array4<f32> }`, `pub(crate) fn build_span_idx(num_words: usize) -> ndarray::Array3<i64>`, `pub(crate) fn decode_span_scores(text: &str, words: &[xberg_gliner::Token], labels: &[String], scorer_out: &ScorerOutput, pred_count: usize, threshold: f32, flat_ner: bool, dup_label: bool, multi_label: bool) -> crate::Result<xberg_gliner::SpanOutput>` (consumed by Task 7's `pipeline.rs`).

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_span_idx_zero_pads_overflow() {
        // 2 words: spans (0,0)(0,1)..(0,7) but only width 0 and 1 stay in range.
        let idx = build_span_idx(2);
        assert_eq!(idx.shape(), &[1, 2 * MAX_WIDTH, 2]);
        assert_eq!((idx[[0, 0, 0]], idx[[0, 0, 1]]), (0, 0)); // start=0 width_idx=0 -> end=0
        assert_eq!((idx[[0, 1, 0]], idx[[0, 1, 1]]), (0, 1)); // start=0 width_idx=1 -> end=1
        assert_eq!((idx[[0, 2, 0]], idx[[0, 2, 1]]), (0, 0)); // start=0 width_idx=2 -> end=2 >= 2 words, padded
    }

    #[test]
    fn decode_span_scores_drops_below_threshold_candidates() {
        use xberg_gliner::Token;
        let text = "Ada Lovelace";
        let words = vec![Token::new(0, 3, "ada"), Token::new(4, 12, "lovelace")];
        let labels = vec!["person".to_string()];
        // scores: [MAX_COUNT, num_words=2, MAX_WIDTH, num_labels=1], all zero (below any positive threshold).
        let scores = ndarray::Array4::<f32>::zeros((MAX_COUNT, 2, MAX_WIDTH, 1));
        let out = decode_span_scores(text, &words, &labels, &ScorerOutput { scores }, 1, 0.5, true, false, false)
            .expect("decode must not error on all-below-threshold scores");
        assert!(out.spans[0].is_empty());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p xberg-gliner-candle build_span_idx_zero_pads_overflow decode_span_scores_drops_below_threshold_candidates --no-run`
Expected: FAIL — `decode` module empty / functions unresolved.

- [ ] **Step 3: Write the implementation**

Create `crates/xberg-gliner-candle/src/decode.rs`:

```rust
//! Span-index construction and score decoding for the Candle GLiNER2
//! pipeline. The numeric decode loop is adapted from
//! `anno::backends::gliner2_fastino::pipeline::decode_entities_with_thresholds`
//! to target `xberg_gliner::Span` (byte offsets) instead of anno's
//! char-offset `Entity` type — no offset-unit conversion needed here.
//!
//! `build_span_idx`'s width-index convention (0-indexed `width_idx`,
//! `end = start + width_idx`) is intentionally NOT the same as
//! `xberg_gliner`'s own (ONNX-targeted) `build_span_idx` — see plan
//! Global Constraints. This one matches the Candle heads' trained
//! weight expectations.

use ndarray::Array3;
pub(crate) use ndarray::Array4;

use xberg_gliner::{Span, Token, decode::greedy_search};

pub(crate) use crate::heads::MAX_WIDTH;
pub(crate) use crate::heads::count_lstm::MAX_COUNT;

/// Per-instance per-span per-label entity scores. Shape
/// `[MAX_COUNT, num_words, MAX_WIDTH, num_labels]`. Already-sigmoided —
/// `Scorer::forward` applies `candle_nn::ops::sigmoid` before this is built.
pub(crate) struct ScorerOutput {
    pub scores: Array4<f32>,
}

/// Build the span-index tensor consumed by `heads::span_rep::SpanRep::forward`.
///
/// For each `(start_word, width_idx)` pair where `width_idx ∈ 0..MAX_WIDTH`,
/// emits `(start, start + width_idx)`. Out-of-range pairs (`end >= num_words`)
/// are zero-padded — those spans are masked out during decode by recomputing
/// `end_word` independently rather than reading it back from this tensor.
pub(crate) fn build_span_idx(num_words: usize) -> Array3<i64> {
    let num_spans = num_words * MAX_WIDTH;
    let mut data = Vec::with_capacity(num_spans * 2);
    for start in 0..num_words {
        for width in 0..MAX_WIDTH {
            let end = start + width;
            if end >= num_words {
                data.extend_from_slice(&[0_i64, 0_i64]);
            } else {
                data.push(start as i64);
                data.push(end as i64);
            }
        }
    }
    Array3::from_shape_vec((1, num_spans, 2), data).expect("span_idx shape consistent by construction")
}

/// Decode the scorer's `[MAX_COUNT, num_words, MAX_WIDTH, num_labels]` tensor
/// into a [`xberg_gliner::SpanOutput`], applying a single global `threshold`,
/// then greedy-merging overlaps via `xberg_gliner::decode::greedy_search`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn decode_span_scores(
    text: &str,
    words: &[Token],
    labels: &[String],
    scorer_out: &ScorerOutput,
    pred_count: usize,
    threshold: f32,
    flat_ner: bool,
    dup_label: bool,
    multi_label: bool,
) -> crate::Result<xberg_gliner::SpanOutput> {
    let num_words = words.len();
    let num_labels = labels.len();
    let scores = &scorer_out.scores;

    let mut candidates: Vec<Span> = Vec::new();
    for c_idx in 0..pred_count.min(MAX_COUNT) {
        for start in 0..num_words {
            for width_idx in 0..MAX_WIDTH {
                let end_word = (start + width_idx + 1).min(num_words);
                for m in 0..num_labels {
                    let prob = scores[[c_idx, start, width_idx, m]];
                    if prob <= threshold {
                        continue;
                    }
                    let byte_start = words[start].start();
                    let byte_end = words[end_word - 1].end();
                    if byte_end > text.len() || byte_start >= byte_end {
                        continue;
                    }
                    let surface = text[byte_start..byte_end].trim();
                    if surface.is_empty() {
                        continue;
                    }
                    candidates.push(Span::new(
                        0,
                        byte_start,
                        byte_end,
                        surface.to_string(),
                        labels[m].clone(),
                        prob,
                    )?);
                }
            }
        }
    }

    candidates.sort_unstable_by_key(Span::offsets);
    let spans = greedy_search(&candidates, flat_ner, dup_label, multi_label);

    Ok(xberg_gliner::SpanOutput {
        texts: vec![text.to_string()],
        entities: labels.to_vec(),
        spans: vec![spans],
    })
}
```

Note: `Token::new` must be `pub` for the test's `Token::new(0, 3, "ada")` call to compile — check `crates/xberg-gliner/src/input.rs:Token::new`. If it is currently `pub(crate)`, widen it to `pub` in this step (one extra line in `input.rs`, same pattern as Task 2) and add `pub use input::Token;`'s constructor is already covered since `Token` itself is `pub` — only `Token::new` needs the same treatment if private. Run `cargo doc -p xberg-gliner --no-deps 2>&1 | grep -i token` or just attempt the build in Step 4 and fix if it fails.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p xberg-gliner-candle build_span_idx_zero_pads_overflow decode_span_scores_drops_below_threshold_candidates -- --nocapture`
Expected: both PASS. If `Token::new` is private, widen its visibility in `crates/xberg-gliner/src/input.rs` first (mirrors Task 2's pattern exactly), re-run.

- [ ] **Step 5: Commit**

```bash
git add crates/xberg-gliner-candle/src/decode.rs crates/xberg-gliner/src/input.rs
git commit -m "feat(gliner-candle): implement span-index construction and score decoding"
```

---

### Task 7: Write `pipeline.rs` (encode → encoder → heads → `ScorerOutput` orchestration)

NOT a port. Anno's Candle `pipeline.rs` (`run_pipeline_candle`) is retargeted here from its `ProcessedRecord`/`TaskMapping` inputs to `xberg_gliner::encode_v2`'s `V2Encoded` (verified field-equivalent in Task 6's header comment / design spec §4): `encoded.text_positions` (already per-word token indices) replaces anno's `word_to_token_maps` word-start lookup, and `encoded.schema_positions` (`[P]` index first, then `[E]` indices) replaces `task.prompt_tok_idx` + `task.field_tok_indices`.

**Files:**
- Create: `crates/xberg-gliner-candle/src/pipeline.rs`

**Interfaces:**
- Consumes: `xberg_gliner::{V2Encoded, V2Splitter, V2Tokenizer, Token, encode_v2}` (Task 2), `crate::encoder::Encoder` (Task 3), `crate::heads::{AllHeads, schema_gather::SchemaGather, scorer::Scorer, token_gather::TokenGather}` (Task 5a/5b), `crate::decode::{MAX_COUNT, MAX_WIDTH, ScorerOutput, build_span_idx}` (Task 6).
- Produces: `pub(crate) fn run_pipeline(tokenizer: &V2Tokenizer, splitter: &V2Splitter, device: &candle_core::Device, encoder: &crate::encoder::Encoder, heads: &crate::heads::AllHeads, text: &str, labels: &[String]) -> crate::Result<(ScorerOutput, usize, V2Encoded)>` (consumed by Task 8's `extract_ner`). Takes the model's components individually rather than `&Gliner2Candle`, so this task compiles and tests independently of Task 8 (which defines that struct).

- [ ] **Step 1: Write the failing test**

A full pipeline run needs real model weights (not available in CI — covered by Task 8's gated smoke test). This task's unit test instead pins the empty-input short-circuit, which needs no weights:

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn run_pipeline_is_declared() {
        // Compile-time check only — `run_pipeline` must be `pub(crate)` and take
        // the model's components individually (so this task has no forward
        // dependency on Task 8's `Gliner2Candle`). Full behavioral coverage is
        // in Task 8's gated smoke test, which requires real model weights.
        fn _assert_signature(
            f: fn(
                &xberg_gliner::V2Tokenizer,
                &xberg_gliner::V2Splitter,
                &candle_core::Device,
                &crate::encoder::Encoder,
                &crate::heads::AllHeads,
                &str,
                &[String],
            ) -> crate::Result<(crate::decode::ScorerOutput, usize, xberg_gliner::V2Encoded)>,
        ) {
            let _ = f;
        }
        _assert_signature(super::run_pipeline);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo check -p xberg-gliner-candle`
Expected: FAIL — `pipeline` module is empty (declared in `lib.rs` from Task 1, no content yet), so `run_pipeline` is unresolved. (Tasks 3–6 already provided `encoder`, `heads`, and `decode`, so the only gap is this module's body — no forward dependency on Task 8.)

- [ ] **Step 3: Write the implementation**

Create `crates/xberg-gliner-candle/src/pipeline.rs`:

```rust
//! Orchestrates encode → encoder.forward → heads → [`crate::decode::ScorerOutput`]
//! for the Candle GLiNER2 "entities" task. New glue code (not a port) built
//! on `xberg_gliner::encode_v2`'s already-shipped schema-prompt encoding —
//! see plan Task 6 module doc for why this replaces anno's separate
//! `ProcessedRecord`/`TaskMapping` machinery.

use candle_core::Tensor;
use ndarray::Array4;

use xberg_gliner::{V2Encoded, encode_v2};

use crate::GlinerCandleError;
use crate::decode::{MAX_COUNT, MAX_WIDTH, ScorerOutput, build_span_idx};
use crate::heads::schema_gather::SchemaGather;
use crate::heads::scorer::Scorer;
use crate::heads::token_gather::TokenGather;

/// Run the full entities-task pipeline for one `(text, labels)` pair.
/// Returns `(scorer_out, pred_count, encoded)` — `encoded.words` is needed
/// by the caller (Task 8's `extract_ner`) to decode byte offsets.
pub(crate) fn run_pipeline(
    tokenizer: &xberg_gliner::V2Tokenizer,
    splitter: &xberg_gliner::V2Splitter,
    device: &candle_core::Device,
    encoder: &crate::encoder::Encoder,
    heads: &crate::heads::AllHeads,
    text: &str,
    labels: &[String],
) -> crate::Result<(ScorerOutput, usize, V2Encoded)> {
    let encoded = encode_v2(text, labels, tokenizer, splitter)?;

    // 1. Truncate to the encoder's position-embedding limit.
    let max_seq = encoder.config.max_position_embeddings as usize;
    let seq_len = encoded.input_ids.len().min(max_seq);
    let input_ids = Tensor::from_slice(&encoded.input_ids[..seq_len], (1, seq_len), device)?;
    let attention_mask =
        Tensor::from_slice(&vec![1u32; seq_len][..], (1, seq_len), device)?;

    // 2. Encode.
    let hidden = encoder.forward(&input_ids, &attention_mask, None)?; // [1, S, H]

    // 3. Token gather. `text_positions` are already per-word token indices
    //    from `encode_v2` — filter to the truncated sequence.
    let filtered_positions: Vec<u32> = encoded
        .text_positions
        .iter()
        .filter(|&&pos| (pos as usize) < seq_len)
        .map(|&pos| pos as u32)
        .collect();
    let num_words = filtered_positions.len();
    if num_words == 0 {
        return Ok((empty_scorer_output(), 0, encoded));
    }
    let word_indices = Tensor::from_slice(&filtered_positions[..], (num_words,), device)?;
    let text_emb = TokenGather.forward(&hidden, &word_indices)?; // [1, num_words, H]

    // 4. Span rep.
    let span_idx_arr = build_span_idx(num_words);
    let span_idx_data: Vec<i64> = span_idx_arr.iter().copied().collect();
    let span_idx = Tensor::from_slice(&span_idx_data[..], (1, num_words * MAX_WIDTH, 2), device)?;
    let span_rep_out = heads.span_rep.forward(&text_emb, &span_idx)?; // [1, num_words, MAX_WIDTH, H]

    // 5. Schema gather: `[P]` index first, then per-label `[E]` indices —
    //    exactly `encoded.schema_positions`' order.
    if encoded.schema_positions.is_empty() {
        return Err(GlinerCandleError::Backend(
            "schema_positions empty — encode_v2 must emit at least the [P] marker".to_string(),
        ));
    }
    let schema_idx: Vec<u32> = encoded.schema_positions.iter().map(|&p| p as u32).collect();
    let schema_idx_t = Tensor::from_slice(&schema_idx[..], (schema_idx.len(),), device)?;
    let sg_out = SchemaGather.forward(&hidden, &schema_idx_t)?;

    // 6. Count pred.
    let pred_count = heads.count_pred.forward(&sg_out.pc_emb)?;
    if pred_count == 0 {
        return Ok((empty_scorer_output(), 0, encoded));
    }

    // 7. Count LSTM (GRU): struct_proj [pred_count, F, H].
    let struct_proj = heads
        .count_lstm
        .forward(&sg_out.field_embs, pred_count, device)?;

    // 8. Scorer: [pred_count, F, num_words, MAX_WIDTH] sigmoid scores.
    let span_rep_per_sample = span_rep_out.squeeze(0)?;
    let scores = Scorer.forward(&span_rep_per_sample, &struct_proj)?;

    // 9. Permute to [pred_count, num_words, MAX_WIDTH, num_labels], pad to MAX_COUNT.
    let scores = scores.permute((0, 2, 3, 1))?.contiguous()?;
    let num_labels = labels.len();
    let scores_padded: Tensor = if pred_count < MAX_COUNT {
        let pad_shape = (MAX_COUNT - pred_count, num_words, MAX_WIDTH, num_labels);
        let pad = Tensor::zeros(pad_shape, scores.dtype(), device)?;
        Tensor::cat(&[&scores, &pad], 0)?
    } else {
        scores
    };

    // 10. Read back to host as Array4<f32>.
    let scores_vec: Vec<f32> = scores_padded.flatten_all()?.to_vec1::<f32>()?;
    let scores_arr = Array4::from_shape_vec((MAX_COUNT, num_words, MAX_WIDTH, num_labels), scores_vec)
        .map_err(|e| GlinerCandleError::Backend(format!("scores reshape: {e}")))?;

    Ok((ScorerOutput { scores: scores_arr }, pred_count, encoded))
}

fn empty_scorer_output() -> ScorerOutput {
    ScorerOutput {
        scores: Array4::zeros((0, 0, 0, 0)),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn run_pipeline_is_declared() {
        fn _assert_signature(
            f: fn(
                &xberg_gliner::V2Tokenizer,
                &xberg_gliner::V2Splitter,
                &candle_core::Device,
                &crate::encoder::Encoder,
                &crate::heads::AllHeads,
                &str,
                &[String],
            ) -> crate::Result<(crate::decode::ScorerOutput, usize, xberg_gliner::V2Encoded)>,
        ) {
            let _ = f;
        }
        _assert_signature(super::run_pipeline);
    }
}
```

Note the `?` operator works directly on `candle_core::Result`/`xberg_gliner::Result` returns because `GlinerCandleError` derives `#[from] candle_core::Error` and `#[from] xberg_gliner::GlinerError` (Task 1's `error.rs`) — no per-call `.map_err` boilerplate needed, unlike anno's original (which used a plain `String`-based `Error::Backend` everywhere).

This task compiles and tests standalone — `run_pipeline` takes the model's components (`encoder`, `heads`, tokenizer, splitter, device), all defined in Tasks 3–6, so there is no forward dependency on Task 8's `Gliner2Candle` struct.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p xberg-gliner-candle run_pipeline_is_declared -- --nocapture`
Expected: PASS — the crate compiles at the end of this task (no dependency on later tasks).

- [ ] **Step 5: Commit**

```bash
git add crates/xberg-gliner-candle/src/pipeline.rs
git commit -m "feat(gliner-candle): implement pipeline orchestration over encode_v2 and heads"
```

---

### Task 8: Write `model.rs` (public `Gliner2Candle` API) + gated smoke test + synthetic-adapter test

Completes Part 1. Adapted from anno's `mod.rs` (`GLiNER2FastinoCandle`), trimmed to the `extract_ner`-only scope (no `from_pretrained`, `extract_structure`, `classify`, or the `Model`/`ZeroShotNER` trait impls — those are anno-specific traits this crate doesn't define).

**Files:**
- Create: `crates/xberg-gliner-candle/src/model.rs`
- Modify: `crates/xberg-gliner-candle/src/lib.rs` (add `mod model; pub use model::Gliner2Candle;`)
- Test: `crates/xberg-gliner-candle/tests/smoke.rs` (new, gated integration test)

**Interfaces:**
- Produces (public crate API, matches design spec §4 verbatim): `Gliner2Candle::from_local(model_dir: &Path) -> Result<Self>`, `Gliner2Candle::from_local_with_device(model_dir: &Path, device: &Device) -> Result<Self>`, `Gliner2Candle::load_adapter(&mut self, name: &str, adapter_dir: &Path) -> Result<()>`, `Gliner2Candle::unload_adapter(&mut self) -> Result<()>`, `Gliner2Candle::active_adapter(&self) -> Option<&str>`, `Gliner2Candle::extract_ner(&self, text: &str, labels: &[&str], threshold: f32) -> Result<Vec<xberg_gliner::Span>>`, `Gliner2Candle::approx_bytes(&self) -> u64` (base safetensors size; consumed by Task 10's weight-bounded model cache).

- [ ] **Step 1: Write the failing tests**

Add to `crates/xberg-gliner-candle/src/tests.rs`:

```rust
#[test]
fn from_local_rejects_missing_weights() {
    let dir = tempfile::tempdir().expect("tempdir");
    let err = crate::Gliner2Candle::from_local(dir.path())
        .expect_err("empty dir must error, not panic");
    assert!(err.to_string().contains("model.safetensors"));
}
```

Create `crates/xberg-gliner-candle/tests/smoke.rs` (gated integration test — the dominant validation risk per the spec; needs real model + adapter artifacts, so it's `#[ignore]`d by default and skipped gracefully when env vars are unset):

```rust
//! Gated smoke test: requires a real GLiNER2 PyTorch safetensors snapshot
//! and a real PEFT LoRA adapter on disk. Run explicitly with:
//!
//! ```text
//! GLINER2_CANDLE_MODEL_DIR=/path/to/gliner2-multi-v1 \
//! GLINER2_TEST_ADAPTER_DIR=/path/to/adapter \
//! cargo test -p xberg-gliner-candle --test smoke -- --ignored
//! ```

#[test]
#[ignore = "requires real GLiNER2 safetensors model + PEFT adapter on disk"]
fn base_model_extracts_entities_and_adapter_changes_output() {
    let Ok(model_dir) = std::env::var("GLINER2_CANDLE_MODEL_DIR") else {
        eprintln!("skipping: GLINER2_CANDLE_MODEL_DIR not set");
        return;
    };
    let Ok(adapter_dir) = std::env::var("GLINER2_TEST_ADAPTER_DIR") else {
        eprintln!("skipping: GLINER2_TEST_ADAPTER_DIR not set");
        return;
    };

    let mut model = xberg_gliner_candle::Gliner2Candle::from_local(std::path::Path::new(&model_dir))
        .expect("load base model");
    let text = "Steve Jobs founded Apple in Cupertino.";
    let labels = ["person", "organization", "location"];

    let base_spans = model
        .extract_ner(text, &labels, 0.3)
        .expect("base extraction must succeed");
    assert!(!base_spans.is_empty(), "base model must find at least one entity");

    model
        .load_adapter("test-adapter", std::path::Path::new(&adapter_dir))
        .expect("adapter load must succeed");
    assert_eq!(model.active_adapter(), Some("test-adapter"));

    let adapter_spans = model
        .extract_ner(text, &labels, 0.3)
        .expect("adapted extraction must succeed");

    // The adapter must measurably change the model — either different spans
    // or different confidence scores. A merge that's silently a no-op would
    // pass `is_empty()` checks but defeat the entire LoRA feature.
    assert_ne!(
        base_spans, adapter_spans,
        "loading a real adapter must change inference output — if this \
         fails, the merge is silently a no-op"
    );

    model.unload_adapter().expect("unload must succeed");
    assert_eq!(model.active_adapter(), None);
    let unloaded_spans = model.extract_ner(text, &labels, 0.3).expect("post-unload extraction");
    assert_eq!(
        base_spans, unloaded_spans,
        "unload_adapter must restore exact base-model behavior"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p xberg-gliner-candle from_local_rejects_missing_weights --no-run`
Expected: FAIL — `Gliner2Candle` doesn't exist.

- [ ] **Step 3: Write the implementation**

Create `crates/xberg-gliner-candle/src/model.rs`:

```rust
//! Public `Gliner2Candle` API: load, adapter lifecycle, entity extraction.
//! Adapted from `anno::backends::gliner2_fastino_candle::GLiNER2FastinoCandle`,
//! trimmed to `extract_ner`-only scope (see plan Global Constraints).

use std::path::{Path, PathBuf};

use candle_core::Device;

use crate::{GlinerCandleError, Result, decode, encoder, heads, lora, pipeline};

/// Candle-based GLiNER2 backend with PEFT LoRA adapter merge-at-load support.
pub struct Gliner2Candle {
    pub(crate) tokenizer: xberg_gliner::V2Tokenizer,
    pub(crate) splitter: xberg_gliner::V2Splitter,
    pub(crate) device: Device,
    /// Directory containing the base model's `tokenizer.json`, `config.json`
    /// (or `encoder_config/config.json`), and `model.safetensors`. Used to
    /// re-merge from disk on `load_adapter`/`unload_adapter`.
    base_model_dir: PathBuf,
    pub(crate) encoder: encoder::Encoder,
    pub(crate) heads: heads::AllHeads,
    active_adapter: Option<String>,
    model_id: String,
    /// Approximate resident size in bytes — the base `model.safetensors` file
    /// size, recorded at load. A merged adapter produces a same-shape model, so
    /// this stays valid across `load_adapter`/`unload_adapter`.
    approx_bytes: u64,
}

impl std::fmt::Debug for Gliner2Candle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Gliner2Candle")
            .field("model_id", &self.model_id)
            .field("active_adapter", &self.active_adapter)
            .finish()
    }
}

impl Gliner2Candle {
    /// Active adapter name, or `None` if running on pure base weights.
    pub fn active_adapter(&self) -> Option<&str> {
        self.active_adapter.as_deref()
    }

    /// Approximate resident size in bytes (the base safetensors file size).
    /// Used by the dispatch layer's weight-bounded model cache to bound RAM.
    pub fn approx_bytes(&self) -> u64 {
        self.approx_bytes
    }

    /// Load a PEFT-format LoRA adapter and merge it into the base weights.
    /// Replaces any previously-active adapter (reloads from the cached
    /// `base_model_dir` and re-applies the new delta). Cost: ~100ms for a
    /// ~280M-param model at rank 8; inference afterward runs at base-model
    /// speed (zero per-forward overhead).
    pub fn load_adapter(&mut self, name: &str, adapter_dir: &Path) -> Result<()> {
        let adapter = lora::LoraAdapter::load(adapter_dir, &self.device)?;

        if let Some(adapter_base) = adapter.config.base_model_name_or_path.as_deref() {
            if !self.model_id.contains(adapter_base) && !adapter_base.contains(&self.model_id) {
                return Err(GlinerCandleError::Backend(format!(
                    "load_adapter: adapter trained on '{adapter_base}', current model is \
                     '{}'. Refusing to merge — remove base_model_name_or_path from \
                     adapter_config.json to bypass.",
                    self.model_id
                )));
            }
        }

        let base_safetensors = self.base_model_dir.join("model.safetensors");
        let merged = lora::merge_into_base(&base_safetensors, &adapter, &self.device)?;

        let vb = candle_nn::VarBuilder::from_tensors(merged, candle_core::DType::F32, &self.device);
        let new_encoder = encoder::Encoder::from_var_builder(vb.pp("encoder"), &self.encoder.config)?;
        let new_heads = heads::AllHeads::from_var_builder(vb, &self.device)?;

        self.encoder = new_encoder;
        self.heads = new_heads;
        self.active_adapter = Some(name.to_string());
        Ok(())
    }

    /// Discard the active adapter and reload pure base weights from
    /// `base_model_dir`. Idempotent.
    pub fn unload_adapter(&mut self) -> Result<()> {
        if self.active_adapter.is_none() {
            return Ok(());
        }
        let weights_path = self.base_model_dir.join("model.safetensors");
        let config_path = resolve_encoder_config_path(&self.base_model_dir);
        self.encoder = encoder::Encoder::from_safetensors(&weights_path, &config_path, &self.device)?;
        self.heads = heads::AllHeads::from_safetensors(&weights_path, &self.device)?;
        self.active_adapter = None;
        Ok(())
    }

    /// Load from a local directory containing `tokenizer.json`, `config.json`
    /// (or `encoder_config/config.json`), and `model.safetensors`. CPU device.
    pub fn from_local(model_dir: &Path) -> Result<Self> {
        Self::from_local_with_device(model_dir, &Device::Cpu)
    }

    /// Load from a local directory with an explicit Candle device.
    pub fn from_local_with_device(model_dir: &Path, device: &Device) -> Result<Self> {
        let tokenizer_path = model_dir.join("tokenizer.json");
        let weights_path = model_dir.join("model.safetensors");
        let config_path = resolve_encoder_config_path(model_dir);

        if !weights_path.exists() {
            return Err(GlinerCandleError::Backend(format!(
                "model.safetensors not found in {} (PyTorch fastino/gliner2-* repo \
                 expected; an ONNX export is a different artifact)",
                model_dir.display()
            )));
        }

        let tokenizer = xberg_gliner::V2Tokenizer::from_file(&tokenizer_path)?;
        let splitter = xberg_gliner::V2Splitter::new()?;
        let encoder = encoder::Encoder::from_safetensors(&weights_path, &config_path, device)?;
        let heads = heads::AllHeads::from_safetensors(&weights_path, device)?;
        let model_id = model_dir
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "gliner2_candle_local".to_string());
        let approx_bytes = std::fs::metadata(&weights_path).map(|m| m.len()).unwrap_or(0);

        Ok(Self {
            tokenizer,
            splitter,
            device: device.clone(),
            base_model_dir: model_dir.to_path_buf(),
            encoder,
            heads,
            active_adapter: None,
            model_id,
            approx_bytes,
        })
    }

    /// Extract entities for the given zero-shot `labels`.
    pub fn extract_ner(&self, text: &str, labels: &[&str], threshold: f32) -> Result<Vec<xberg_gliner::Span>> {
        if labels.is_empty() {
            return Ok(vec![]);
        }
        let owned_labels: Vec<String> = labels.iter().map(|s| s.to_string()).collect();
        let (scorer_out, pred_count, encoded) = pipeline::run_pipeline(
            &self.tokenizer,
            &self.splitter,
            &self.device,
            &self.encoder,
            &self.heads,
            text,
            &owned_labels,
        )?;
        if pred_count == 0 {
            return Ok(vec![]);
        }
        let output = decode::decode_span_scores(
            text,
            &encoded.words,
            &owned_labels,
            &scorer_out,
            pred_count,
            threshold,
            /* flat_ner = */ true,
            /* dup_label = */ false,
            /* multi_label = */ false,
        )?;
        Ok(output.spans.into_iter().next().unwrap_or_default())
    }
}

fn resolve_encoder_config_path(model_dir: &Path) -> PathBuf {
    let nested = model_dir.join("encoder_config").join("config.json");
    if nested.exists() { nested } else { model_dir.join("config.json") }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_local_with_device_rejects_missing_weights() {
        let dir = tempfile::tempdir().expect("tempdir");
        let err = Gliner2Candle::from_local_with_device(dir.path(), &Device::Cpu)
            .expect_err("empty dir must fail");
        assert!(err.to_string().contains("model.safetensors"));
    }
}
```

Update `crates/xberg-gliner-candle/src/lib.rs` — add `mod model;` to the module list and `pub use model::Gliner2Candle;` to the `pub use` block (alongside `pub use error::{GlinerCandleError, Result};`).

- [ ] **Step 4: Run all tests to verify they pass**

Run: `cargo test -p xberg-gliner-candle`
Expected: every non-`#[ignore]`d test PASSES, including Task 7's `run_pipeline_is_declared` and this task's `from_local_rejects_missing_weights` / `from_local_with_device_rejects_missing_weights`. The `#[ignore]`d smoke test is skipped by default — confirm with `cargo test -p xberg-gliner-candle --test smoke` (runs the file, test itself no-ops without env vars set) and separately note it for manual/CI-optional execution per Step 5 below.

- [ ] **Step 5: Run the gated smoke test if real model artifacts are available**

If you have access to a real `fastino/gliner2-multi-v1`-family PyTorch snapshot and a matching PEFT LoRA adapter on disk, run:

```bash
GLINER2_CANDLE_MODEL_DIR=/path/to/model GLINER2_TEST_ADAPTER_DIR=/path/to/adapter \
  cargo test -p xberg-gliner-candle --test smoke -- --ignored --nocapture
```

Expected: PASS, confirming (a) base-model extraction finds entities, (b) adapter load measurably changes output, (c) unload restores exact base behavior. This is the dominant validation risk per the design spec — if this is unavailable in your environment, leave it `#[ignore]`d and flag to the user that real-inference correctness remains unverified until someone runs it.

- [ ] **Step 6: Commit**

```bash
git add crates/xberg-gliner-candle/src/model.rs crates/xberg-gliner-candle/src/lib.rs crates/xberg-gliner-candle/src/tests.rs crates/xberg-gliner-candle/tests/smoke.rs
git commit -m "feat(gliner-candle): public Gliner2Candle API with adapter lifecycle and gated smoke test"
```

---

## Part 2 — Core config & dispatch (`crates/xberg`)

### Task 9: Add `NerBackendKind::GlinerCandle` + `AdapterConfig`/`AdapterSource`

**Files:**
- Modify: `crates/xberg/src/core/config/ner.rs`

**Interfaces:**
- Produces: `NerBackendKind::GlinerCandle` variant, `NerConfig.adapter: Option<AdapterConfig>` field, `AdapterConfig { name: String, source: AdapterSource }`, `AdapterSource::{Local { path: PathBuf }, HfRepo { repo: String, revision: Option<String> }}` (consumed by Task 10's dispatch + `AdapterRegistry`).

- [ ] **Step 1: Write the failing test**

Add to `crates/xberg/src/core/config/ner.rs` test module (create one if absent, at the bottom of the file):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ner_config_defaults_to_no_adapter() {
        let config = NerConfig::default();
        assert_eq!(config.adapter, None);
        assert_eq!(config.backend, NerBackendKind::Onnx);
    }

    #[test]
    fn gliner_candle_is_a_distinct_backend_kind() {
        assert_ne!(NerBackendKind::GlinerCandle, NerBackendKind::Onnx);
        assert_ne!(NerBackendKind::GlinerCandle, NerBackendKind::Llm);
    }

    #[test]
    fn adapter_config_round_trips_through_json() {
        let config = AdapterConfig {
            name: "fr-legal-pii".to_string(),
            source: AdapterSource::Local { path: "/models/adapters/fr-legal-pii".into() },
        };
        let json = serde_json::to_string(&config).expect("serialize");
        let back: AdapterConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.name, "fr-legal-pii");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p xberg ner_config_defaults_to_no_adapter gliner_candle_is_a_distinct_backend_kind adapter_config_round_trips_through_json --no-run --features ner`
Expected: FAIL — `NerBackendKind::GlinerCandle`, `NerConfig.adapter`, `AdapterConfig`, `AdapterSource` don't exist yet.

- [ ] **Step 3: Write the implementation**

In `crates/xberg/src/core/config/ner.rs`, add `use std::path::PathBuf;` near the top imports, then add the `adapter` field to `NerConfig` (after the existing `custom_labels` field):

```rust
    /// Domain LoRA adapter to merge at load. Only honoured by
    /// [`NerBackendKind::GlinerCandle`] — setting this with any other
    /// backend is a configuration error (validated in the NER processor's
    /// `make_backend`).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "alef-meta", alef(since = "5.3.0"))]
    pub adapter: Option<AdapterConfig>,
```

Add the `GlinerCandle` variant to `NerBackendKind` (after `Llm`):

```rust
    /// Candle (pure-Rust) GLiNER2 inference with runtime PEFT LoRA adapter
    /// merge-at-load. Requires `ner-candle` feature. The only backend that
    /// honours [`NerConfig::adapter`].
    GlinerCandle,
```

Add the new types at the bottom of the file, before any existing test module:

```rust
/// Domain LoRA adapter configuration. Only used by [`NerBackendKind::GlinerCandle`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "alef-meta", alef(since = "5.3.0"))]
pub struct AdapterConfig {
    /// Adapter name — used as the cache key in the engine's adapter registry
    /// and as the identity recorded in `Gliner2Candle::active_adapter`.
    pub name: String,
    /// Where to load the PEFT adapter files (`adapter_config.json` +
    /// `adapter_model.safetensors`) from.
    pub source: AdapterSource,
}

/// Source location for a LoRA adapter's PEFT files.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case", tag = "kind")]
#[cfg_attr(feature = "alef-meta", alef(since = "5.3.0"))]
pub enum AdapterSource {
    /// Adapter files already present on local disk.
    Local {
        /// Directory containing `adapter_config.json` and
        /// `adapter_model.safetensors`.
        path: PathBuf,
    },
    /// Adapter files hosted on Hugging Face Hub — downloaded via the same
    /// `hf-hub` path the `ner-onnx` backend already uses.
    HfRepo {
        /// Repository id, e.g. `"xberg-io/gliner2-fr-legal-pii-adapter"`.
        repo: String,
        /// Optional revision/commit. Defaults to the repo's default branch.
        #[serde(skip_serializing_if = "Option::is_none")]
        revision: Option<String>,
    },
}
```

Note: `NerBackendKind` derives `Copy` today (`#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]`). `Eq` requires every variant to be `Eq`-comparable — `GlinerCandle` is a unit variant, so this holds with no further changes. `NerConfig` does NOT derive `Copy` (it already contains `Vec`/`Option` fields), so adding `adapter: Option<AdapterConfig>` is compatible without further trait changes.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p xberg ner_config_defaults_to_no_adapter gliner_candle_is_a_distinct_backend_kind adapter_config_round_trips_through_json --features ner -- --nocapture`
Expected: all three PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/xberg/src/core/config/ner.rs
git commit -m "feat(ner): add GlinerCandle backend kind and adapter config types"
```

---

### Task 10: `AdapterRegistry` + dispatch arm + `ner-candle` feature

Wires `xberg-gliner-candle` into the existing NER backend dispatch
(`crate::plugins::processor::builtin::ner::make_backend`), exactly the same
seam `NerBackendKind::Onnx`/`Llm` already use.

**Caching is two-tier, because adapter-merged models are large (~1 GB each):**

- **Base models** are pinned in an unbounded `RwLock<AHashMap<BaseCacheKey, Arc<Gliner2Candle>>>`
  via the same `get_or_insert_arc` helper `gline.rs` uses. There are very few
  distinct base repos (typically one), and the base is the always-resident
  anchor every adapter merges from (design §6), so pinning it is correct.
- **Adapter-merged models** go in a **weight-bounded `moka::sync::Cache`**
  keyed by `(repo, model_file, tokenizer_file, adapter)`, evicted by
  *approximate bytes* (`Gliner2Candle::approx_bytes`), not entry count. An
  unbounded map here is a real OOM risk: per-request domain selection across N
  adapters would pin N × ~1 GB forever on the 12 GB-laptop target. `moka` is
  already an xberg dependency (it backs `api::jobs::JobStore`) and is lock-free
  concurrent, unlike a `Mutex<lru::LruCache>` (the idiom anno uses for its much
  smaller *prompt* cache in `gliner_onnx/inference.rs`). anno's own Candle
  backend sidesteps the problem by holding a single model and swapping adapters
  in place via `&mut self`; this plan needs concurrent multi-adapter serving,
  so it must bound the pool explicitly.

**Base-model source — a gap Task 9 left open:** `NerConfig.adapter`
(Task 9) only configures the LoRA adapter; it does not say where the base
GLiNER2 Candle model comes from. `NerConfig::hf_repo` /
`hf_model_file` / `hf_tokenizer_file` already exist and are currently
documented as "only used by `NerBackendKind::Onnx`" — this task broadens
that scope to also cover `GlinerCandle`, where they point at a
`safetensors`-format PyTorch GLiNER2 snapshot (e.g.
`tokenizer.json` + `config.json` + `model.safetensors`) instead of an ONNX
export. There is **no pinned default catalog** for Candle GLiNER2 models
(unlike `xberg-io/gliner-models` for ONNX) — `hf_repo` is therefore
**required** when `backend == GlinerCandle`; leaving it unset is a
validation error, not a fallback to some invented default repo id. Adding a
pinned/checksummed catalog is explicitly out of scope here and should be a
follow-up once a canonical `xberg-io/gliner2-candle-models` repo exists.

**Files:**
- Modify: `crates/xberg/Cargo.toml` (new `ner-candle` feature)
- Modify: `crates/xberg/src/core/config/ner.rs` (broaden `hf_repo`/`hf_model_file`/`hf_tokenizer_file` doc comments; add validation)
- Create: `crates/xberg/src/text/ner/gline_candle.rs`
- Modify: `crates/xberg/src/text/ner/mod.rs` (register the module)
- Modify: `crates/xberg/src/plugins/processor/builtin/ner.rs` (dispatch arm)

**Interfaces:**
- Produces: `GlineCandleBackend: NerBackend` (consumed by the dispatch arm and, later, Task 12's `/v1/process` handler indirectly through `NerConfig`), `pub(crate) fn get_or_init_candle_backend(config: &NerConfig) -> Result<Arc<GlineCandleBackend>>`.

- [ ] **Step 1: Add the `ner-candle` feature**

In `crates/xberg/Cargo.toml`, add alongside the existing `ner-onnx` block (around line 319):

```toml
ner-candle = [
    # xberg-gliner-candle pure-Rust GLiNER2 backend with PEFT LoRA adapters.
    "ner",
    "dep:xberg-gliner-candle",
    "dep:hf-hub",
    "dep:moka",        # weight-bounded adapter-merged model cache (already an xberg dep — see api::jobs)
    "tokio-runtime",
]
```

Add to `[dependencies]` (alongside the existing `xberg-gliner` line):

```toml
xberg-gliner-candle = { workspace = true, optional = true }
```

Note: `ner-candle` deliberately does **not** depend on `ort-bundled` —
this is the entire point of the Candle backend (pure-Rust, no ONNX Runtime
linkage), so it stays available on WASM/Android-x86_64 targets where
`ner-onnx` is blocked. It is not yet added to `no-ort-target` / `wasm-target`
aggregates in this task — `candle-core`'s WASM support is unverified for
this model; leave that wiring to a follow-up once Part 1's gated smoke test
(Task 8, Step 5) has been run for real.

- [ ] **Step 2: Broaden `NerConfig` field docs + add validation**

In `crates/xberg/src/core/config/ner.rs`, update the three doc comments
(`hf_repo`, `hf_model_file`, `hf_tokenizer_file`) to read "used by
`NerBackendKind::Onnx` (ONNX export) and `NerBackendKind::GlinerCandle`
(safetensors snapshot — `hf_model_file` defaults to `\"model.safetensors\"`,
`hf_tokenizer_file` to `\"tokenizer.json\"` when unset)". No field shape
changes — this is documentation plus the validation below.

Add to the test module:

```rust
#[test]
fn gliner_candle_requires_hf_repo() {
    let config = NerConfig {
        backend: NerBackendKind::GlinerCandle,
        ..Default::default()
    };
    assert!(config.validate_backend_requirements().is_err());
}

#[test]
fn gliner_candle_with_hf_repo_is_valid() {
    let config = NerConfig {
        backend: NerBackendKind::GlinerCandle,
        hf_repo: Some("fastino/gliner2-multi-v1".to_string()),
        ..Default::default()
    };
    assert!(config.validate_backend_requirements().is_ok());
}

#[test]
fn adapter_on_non_candle_backend_is_rejected() {
    // An adapter on the ONNX backend is a configuration error: LoRA can only
    // be merged by the Candle engine.
    let config = NerConfig {
        backend: NerBackendKind::Onnx,
        adapter: Some(AdapterConfig {
            name: "fr-legal-pii".to_string(),
            source: AdapterSource::Local { path: "/models/adapters/fr-legal-pii".into() },
        }),
        ..Default::default()
    };
    assert!(config.validate_backend_requirements().is_err());
}
```

Run: `cargo test -p xberg gliner_candle_requires_hf_repo gliner_candle_with_hf_repo_is_valid adapter_on_non_candle_backend_is_rejected --no-run --features ner`
Expected: FAIL — `NerConfig::validate_backend_requirements` doesn't exist.

Add the method (new — `NerConfig` had no `validate` method before this task):

```rust
impl NerConfig {
    /// Validate backend-specific requirements that the type system alone
    /// can't express (e.g. `GlinerCandle` needs `hf_repo` set since there is
    /// no pinned default catalog yet).
    pub fn validate_backend_requirements(&self) -> crate::Result<()> {
        if self.backend == NerBackendKind::GlinerCandle && self.hf_repo.is_none() {
            return Err(crate::XbergError::validation(
                "NerConfig.backend is GlinerCandle but hf_repo is unset — there is no pinned \
                 default Candle GLiNER2 catalog yet, so hf_repo (a safetensors-format GLiNER2 \
                 snapshot repo) is required",
            ));
        }
        if self.adapter.is_some() && self.backend != NerBackendKind::GlinerCandle {
            return Err(crate::XbergError::validation(
                "NerConfig.adapter is set but backend is not GlinerCandle — LoRA adapters can \
                 only be merged by the Candle engine (an ONNX graph has baked-in weights). Set \
                 backend = GlinerCandle, or remove the adapter",
            ));
        }
        Ok(())
    }
}
```

Run: `cargo test -p xberg gliner_candle_requires_hf_repo gliner_candle_with_hf_repo_is_valid adapter_on_non_candle_backend_is_rejected --features ner -- --nocapture`
Expected: all three PASS.

- [ ] **Step 3: Write `gline_candle.rs`**

Create `crates/xberg/src/text/ner/gline_candle.rs`:

```rust
//! `xberg-gliner-candle` backend for named-entity recognition.
//!
//! Pure-Rust Candle inference with optional PEFT LoRA adapter merge-at-load.
//! Caches one fully-built (base weights [+ merged adapter]) model instance
//! per `(hf_repo, adapter identity)` key — adapter switching happens by
//! cache miss, not by mutating a shared instance, so concurrent requests for
//! different adapters never contend on a single model's `&mut self`.

use std::hash::Hash;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock};

use ahash::AHashMap;
use async_trait::async_trait;
use parking_lot::RwLock;
use xberg_gliner_candle::Gliner2Candle;

use crate::Result;
use crate::core::config::ner::{AdapterConfig, AdapterSource, NerConfig};
use crate::types::entity::{Entity, EntityCategory};

use super::backend::NerBackend;

type BaseModelCache = AHashMap<BaseCacheKey, Arc<Gliner2Candle>>;

/// Pinned base models — one per `(repo, model_file, tokenizer_file)`. Never
/// evicted: the base is the always-resident anchor every adapter merges from
/// (design §6), and there are very few distinct base repos.
static BASE_MODELS: LazyLock<RwLock<BaseModelCache>> =
    LazyLock::new(|| RwLock::new(AHashMap::default()));

/// Adapter-merged models — weight-bounded by approximate RAM. Each entry is a
/// full ~280M-param merged model (~1.1 GB at f32), so the cache is bounded by
/// total *bytes* (via `moka`'s weigher reading [`Gliner2Candle::approx_bytes`]),
/// not entry count. An unbounded map here would OOM the 12 GB-laptop target as
/// soon as several domain adapters are used in one session.
static MERGED_MODELS: LazyLock<moka::sync::Cache<CandleCacheKey, Arc<Gliner2Candle>>> =
    LazyLock::new(|| {
        moka::sync::Cache::builder()
            .max_capacity(merged_cache_budget_bytes())
            .weigher(|_key, model: &Arc<Gliner2Candle>| {
                u32::try_from(model.approx_bytes()).unwrap_or(u32::MAX)
            })
            .build()
    });

/// Total RAM budget (bytes) for adapter-merged Candle models. Default 4 GiB
/// (~3 concurrent adapters). Override with `XBERG_NER_CANDLE_CACHE_BYTES`; a
/// non-positive or unparsable value falls back to the default (a zero-capacity
/// `moka` cache would evict every entry immediately).
fn merged_cache_budget_bytes() -> u64 {
    const DEFAULT_BUDGET: u64 = 4 * 1024 * 1024 * 1024; // 4 GiB
    std::env::var("XBERG_NER_CANDLE_CACHE_BYTES")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|&bytes| bytes > 0)
        .unwrap_or(DEFAULT_BUDGET)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct BaseCacheKey {
    hf_repo: String,
    model_file: String,
    tokenizer_file: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CandleCacheKey {
    hf_repo: String,
    model_file: String,
    tokenizer_file: String,
    /// `None` for the base model with no adapter merged.
    adapter: Option<AdapterCacheKey>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct AdapterCacheKey {
    name: String,
    source: String, // local path or "hf:{repo}@{revision_or_default}"
}

/// `xberg-gliner-candle` backend wrapper. Read-only after construction —
/// `Gliner2Candle::load_adapter`/`unload_adapter` need `&mut self`, so this
/// backend never mutates a cached instance; it builds a new one (base, then
/// optionally merges the adapter) inside the cache's write-lock build step.
/// `model` is `Arc`-wrapped so `detect_with_custom` can clone the handle into
/// `spawn_blocking` exactly like `gline::GlineBackend::detect_labels` does
/// with `Arc<GlinerEngine>` — no unsafe code needed.
pub struct GlineCandleBackend {
    model: Arc<Gliner2Candle>,
}

fn get_or_insert_arc<K, V, F>(cache: &RwLock<AHashMap<K, Arc<V>>>, key: K, build: F) -> Result<Arc<V>>
where
    K: Clone + Eq + Hash,
    F: FnOnce() -> Result<V>,
{
    {
        let cache = cache.read();
        if let Some(value) = cache.get(&key) {
            return Ok(Arc::clone(value));
        }
    }
    let mut cache = cache.write();
    if let Some(value) = cache.get(&key) {
        return Ok(Arc::clone(value));
    }
    let value = Arc::new(build()?);
    cache.insert(key, Arc::clone(&value));
    Ok(value)
}

/// Resolve (downloading if necessary) the base model directory + cache key
/// for `config`, build the model, optionally merge the configured adapter,
/// and return the cached (or freshly built) backend.
pub(crate) fn get_or_init_candle_backend(config: &NerConfig) -> Result<Arc<GlineCandleBackend>> {
    config.validate_backend_requirements()?;
    let base_key = BaseCacheKey {
        hf_repo: config.hf_repo.clone().expect("validated above"),
        model_file: config.hf_model_file.clone().unwrap_or_else(|| "model.safetensors".to_string()),
        tokenizer_file: config.hf_tokenizer_file.clone().unwrap_or_else(|| "tokenizer.json".to_string()),
    };

    // Base model: built once, pinned for the process lifetime (design §6).
    let base = get_or_insert_arc(&BASE_MODELS, base_key.clone(), || build_base(&base_key))?;

    let Some(adapter) = config.adapter.as_ref() else {
        // No adapter — serve the pinned base directly.
        return Ok(Arc::new(GlineCandleBackend { model: base }));
    };

    // Adapter present — fetch (or build) the merged model from the
    // weight-bounded cache. `try_get_with` dedups concurrent builds of the same
    // key, so two requests for a cold adapter merge it exactly once.
    let key = CandleCacheKey {
        hf_repo: base_key.hf_repo.clone(),
        model_file: base_key.model_file.clone(),
        tokenizer_file: base_key.tokenizer_file.clone(),
        adapter: Some(AdapterCacheKey {
            name: adapter.name.clone(),
            source: adapter_source_tag(&adapter.source),
        }),
    };
    let merged = MERGED_MODELS
        .try_get_with(key, || build_merged(&base_key, adapter))
        .map_err(|error: Arc<crate::XbergError>| crate::XbergError::Plugin {
            message: format!("Failed to build adapter-merged Candle model '{}': {error}", adapter.name),
            plugin_name: "ner-gliner-candle".to_string(),
        })?;

    Ok(Arc::new(GlineCandleBackend { model: merged }))
}

/// Build the pinned base model (no adapter merged).
fn build_base(base_key: &BaseCacheKey) -> Result<Gliner2Candle> {
    let model_dir =
        ensure_candle_base_model(&base_key.hf_repo, &base_key.model_file, &base_key.tokenizer_file)?;
    Gliner2Candle::from_local(&model_dir).map_err(|error| crate::XbergError::Plugin {
        message: format!("Failed to initialise Candle GLiNER2 model '{}': {error}", base_key.hf_repo),
        plugin_name: "ner-gliner-candle".to_string(),
    })
}

/// Build a fully adapter-merged model (base weights + PEFT delta). Returns an
/// `Arc` because it is the stored value type of the `moka` merged-model cache.
fn build_merged(base_key: &BaseCacheKey, adapter: &AdapterConfig) -> Result<Arc<Gliner2Candle>> {
    let model_dir =
        ensure_candle_base_model(&base_key.hf_repo, &base_key.model_file, &base_key.tokenizer_file)?;
    let mut model = Gliner2Candle::from_local(&model_dir).map_err(|error| crate::XbergError::Plugin {
        message: format!("Failed to initialise Candle GLiNER2 model '{}': {error}", base_key.hf_repo),
        plugin_name: "ner-gliner-candle".to_string(),
    })?;
    let adapter_dir = ensure_adapter_files(&adapter.source)?;
    model
        .load_adapter(&adapter.name, &adapter_dir)
        .map_err(|error| crate::XbergError::Plugin {
            message: format!("Failed to load LoRA adapter '{}': {error}", adapter.name),
            plugin_name: "ner-gliner-candle".to_string(),
        })?;
    Ok(Arc::new(model))
}

/// Download `tokenizer.json` / `config.json` / `model_file` from `hf_repo`
/// into the xberg cache. Unverified (no pinned checksums for caller-chosen
/// Candle repos — mirrors `gline::ensure_custom_model`'s precedent).
fn ensure_candle_base_model(hf_repo: &str, model_file: &str, tokenizer_file: &str) -> Result<PathBuf> {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(hf_repo.as_bytes());
    hasher.update(b"\0");
    hasher.update(model_file.as_bytes());
    let cache_key = hex::encode(hasher.finalize());

    let base_dir = crate::cache_dir::resolve_cache_dir("ner").join("gliner-candle").join(&cache_key);
    let weights_path = base_dir.join("model.safetensors");
    let tokenizer_path = base_dir.join("tokenizer.json");
    let config_path = base_dir.join("config.json");

    if weights_path.exists() && tokenizer_path.exists() && config_path.exists() {
        return Ok(base_dir);
    }

    std::fs::create_dir_all(&base_dir).map_err(|error| crate::XbergError::Plugin {
        message: format!("Failed to create Candle GLiNER2 cache dir '{}': {error}", base_dir.display()),
        plugin_name: "ner-gliner-candle".to_string(),
    })?;

    for (remote_name, dest) in [
        (model_file, &weights_path),
        (tokenizer_file, &tokenizer_path),
        ("config.json", &config_path),
    ] {
        let downloaded = crate::model_download::hf_download(hf_repo, remote_name).map_err(|error| {
            crate::XbergError::Plugin {
                message: format!("Failed to download '{remote_name}' from {hf_repo}: {error}"),
                plugin_name: "ner-gliner-candle".to_string(),
            }
        })?;
        std::fs::copy(&downloaded, dest).map_err(|error| crate::XbergError::Plugin {
            message: format!("Failed to publish '{remote_name}' to '{}': {error}", dest.display()),
            plugin_name: "ner-gliner-candle".to_string(),
        })?;
    }

    Ok(base_dir)
}

fn adapter_source_tag(source: &AdapterSource) -> String {
    match source {
        AdapterSource::Local { path } => path.display().to_string(),
        AdapterSource::HfRepo { repo, revision } => {
            format!("hf:{repo}@{}", revision.as_deref().unwrap_or("default"))
        }
    }
}

/// Resolve a `PEFT` adapter's local directory, downloading from HF Hub first
/// if `source` is `HfRepo`. Reuses the same unverified-download precedent as
/// the base model — adapters have no pinned checksums either.
fn ensure_adapter_files(source: &AdapterSource) -> Result<PathBuf> {
    match source {
        AdapterSource::Local { path } => Ok(path.clone()),
        AdapterSource::HfRepo { repo, revision: _ } => {
            let cache_key = adapter_source_tag(source).replace(['/', ':', '@'], "_");
            let adapter_dir = crate::cache_dir::resolve_cache_dir("ner").join("gliner-candle-adapters").join(&cache_key);
            let config_path = adapter_dir.join("adapter_config.json");
            let weights_path = adapter_dir.join("adapter_model.safetensors");
            if config_path.exists() && weights_path.exists() {
                return Ok(adapter_dir);
            }
            std::fs::create_dir_all(&adapter_dir).map_err(|error| crate::XbergError::Plugin {
                message: format!("Failed to create adapter cache dir '{}': {error}", adapter_dir.display()),
                plugin_name: "ner-gliner-candle".to_string(),
            })?;
            for (remote_name, dest) in [
                ("adapter_config.json", &config_path),
                ("adapter_model.safetensors", &weights_path),
            ] {
                let downloaded = crate::model_download::hf_download(repo, remote_name).map_err(|error| {
                    crate::XbergError::Plugin {
                        message: format!("Failed to download adapter file '{remote_name}' from {repo}: {error}"),
                        plugin_name: "ner-gliner-candle".to_string(),
                    }
                })?;
                std::fs::copy(&downloaded, dest).map_err(|error| crate::XbergError::Plugin {
                    message: format!("Failed to publish adapter file '{remote_name}': {error}"),
                    plugin_name: "ner-gliner-candle".to_string(),
                })?;
            }
            Ok(adapter_dir)
        }
    }
}

#[async_trait]
impl NerBackend for GlineCandleBackend {
    async fn detect(&self, text: &str, categories: &[EntityCategory]) -> Result<Vec<Entity>> {
        self.detect_with_custom(text, categories, &[]).await
    }

    async fn detect_with_custom(
        &self,
        text: &str,
        categories: &[EntityCategory],
        custom_labels: &[String],
    ) -> Result<Vec<Entity>> {
        if text.trim().is_empty() {
            return Ok(Vec::new());
        }
        let mut labels: Vec<String> = categories.iter().map(category_to_label).collect();
        labels.extend(custom_labels.iter().cloned());
        if labels.is_empty() {
            return Ok(Vec::new());
        }

        // Clone the Arc (not the model) into the blocking task — identical to
        // gline::GlineBackend::detect_labels's `Arc::clone(&self.model)`. No
        // unsafe needed: `Gliner2Candle` holds only owned Candle CPU tensors
        // and Rust-native types, so `Arc<Gliner2Candle>` is `Send + Sync`
        // automatically via auto-trait derivation.
        let model = Arc::clone(&self.model);
        let text = text.to_string();
        tokio::task::spawn_blocking(move || {
            let label_refs: Vec<&str> = labels.iter().map(String::as_str).collect();
            let spans = model
                .extract_ner(&text, &label_refs, 0.3)
                .map_err(|error| crate::XbergError::Plugin {
                    message: format!("Candle GLiNER2 inference failed: {error}"),
                    plugin_name: "ner-gliner-candle".to_string(),
                })?;
            spans
                .into_iter()
                .map(|span| {
                    let (start, end) = span.offsets();
                    let start = u32::try_from(start).map_err(|error| crate::XbergError::validation_with_source(
                        format!("Candle GLiNER2 returned start offset {start} exceeding u32 entity offset limit"),
                        error,
                    ))?;
                    let end = u32::try_from(end).map_err(|error| crate::XbergError::validation_with_source(
                        format!("Candle GLiNER2 returned end offset {end} exceeding u32 entity offset limit"),
                        error,
                    ))?;
                    Ok(Entity {
                        category: EntityCategory::from(span.class().to_string()),
                        text: span.text().to_string(),
                        start,
                        end,
                        confidence: Some(span.probability()),
                    })
                })
                .collect::<Result<Vec<_>>>()
        })
        .await
        .map_err(|error| crate::XbergError::Plugin {
            message: format!("Candle GLiNER2 spawn_blocking task panicked: {error}"),
            plugin_name: "ner-gliner-candle".to_string(),
        })?
    }
}

/// Identical mapping to `gline::category_to_label` — duplicated rather than
/// imported because `gline` is gated behind `ner-onnx` and this module is
/// gated behind `ner-candle`; a build with only one of the two features
/// enabled must not require the other.
fn category_to_label(category: &EntityCategory) -> String {
    match category {
        EntityCategory::Person => "person".to_string(),
        EntityCategory::Organization => "organization".to_string(),
        EntityCategory::Location => "location".to_string(),
        EntityCategory::Date => "date".to_string(),
        EntityCategory::Time => "time".to_string(),
        EntityCategory::Money => "money".to_string(),
        EntityCategory::Percent => "percent".to_string(),
        EntityCategory::Email => "email".to_string(),
        EntityCategory::Phone => "phone".to_string(),
        EntityCategory::Url => "url".to_string(),
        EntityCategory::Custom(label) => label.clone(),
    }
}
```

- [ ] **Step 4: Register the module**

In `crates/xberg/src/text/ner/mod.rs`, add after the `gline` module declaration:

```rust
#[cfg(feature = "ner-candle")]
pub mod gline_candle;
```

- [ ] **Step 5: Add the dispatch arm**

In `crates/xberg/src/plugins/processor/builtin/ner.rs`, add a third arm to
`make_backend`'s `match config.backend`:

```rust
        NerBackendKind::GlinerCandle => {
            #[cfg(feature = "ner-candle")]
            {
                Ok(crate::text::ner::gline_candle::get_or_init_candle_backend(config)?)
            }
            #[cfg(not(feature = "ner-candle"))]
            {
                Err(crate::XbergError::MissingDependency(
                    "ner-candle feature is not enabled — rebuild xberg with --features ner-candle".to_string(),
                ))
            }
        }
```

- [ ] **Step 6: Write the cache-key isolation test**

Mirrors the existing pattern from commit `9a2135f2fc` ("add Debug impl for
GlinerEngine and architecture cache-key isolation test") for the ONNX
backend — confirm two different adapters never collide in the cache. Add to
`crates/xberg/src/text/ner/gline_candle.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_key_distinguishes_adapters() {
        let base = CandleCacheKey {
            hf_repo: "fastino/gliner2-multi-v1".to_string(),
            model_file: "model.safetensors".to_string(),
            tokenizer_file: "tokenizer.json".to_string(),
            adapter: None,
        };
        let with_adapter = CandleCacheKey {
            adapter: Some(AdapterCacheKey {
                name: "fr-legal-pii".to_string(),
                source: "/models/adapters/fr-legal-pii".to_string(),
            }),
            ..base.clone()
        };
        assert_ne!(base, with_adapter);
    }

    #[test]
    fn adapter_source_tag_distinguishes_revisions() {
        let a = adapter_source_tag(&AdapterSource::HfRepo {
            repo: "xberg-io/adapter".to_string(),
            revision: Some("v1".to_string()),
        });
        let b = adapter_source_tag(&AdapterSource::HfRepo {
            repo: "xberg-io/adapter".to_string(),
            revision: Some("v2".to_string()),
        });
        assert_ne!(a, b);
    }
}
```

Run: `cargo test -p xberg --features ner-candle cache_key_distinguishes_adapters adapter_source_tag_distinguishes_revisions -- --nocapture`
Expected: both PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/xberg/Cargo.toml crates/xberg/src/core/config/ner.rs \
        crates/xberg/src/text/ner/gline_candle.rs crates/xberg/src/text/ner/mod.rs \
        crates/xberg/src/plugins/processor/builtin/ner.rs
git commit -m "feat(ner): wire GlinerCandle backend through dispatch with adapter-aware caching"
```

---

## Part 3 — Privacy API (`crates/xberg`)

**Scope note:** the design spec (`docs/superpowers/specs/2026-06-29-xberg-privacy-api-design.md`)
describes a much larger surface (`/v1/search`, `/v1/collections`, multi-tenancy,
webhooks). This plan's own goal statement (top of file) and architecture
overview only call for **`/v1/process`** (extract → ner(adapter) → pii.detect
→ redact) plus PII rehydration — `/v1/search` and `/v1/collections` already
have a home in the separate, coarser-grained
`docs/superpowers/plans/2026-06-29-xberg-privacy-api.md` (its Phase 4). Tasks
11-13 below ship exactly the rehydration-store + `/v1/process` +
`/v1/documents/{id}/rehydrate` surface this plan promises — `/v1/search` is
intentionally **not** Task 14 here to avoid two plans racing to implement the
same endpoint differently.

**Rehydration strategy scope:** the design spec's §5 describes three
strategies (server-encrypted/zero-key-management, customer-provided-key,
audit-logged). Task 11 ships exactly one: caller-supplied passphrase at both
encrypt and rehydrate time, matching the documented project-wide convention
(CLAUDE.md `pii-pipeline`: "`rehydrate_tokens` tool requires the encryption
passphrase at call time — never cache it in memory beyond the call"). This is
closest to spec §5.2 (customer-provided key) — §5.1's "zero key management"
server-custodied-key tier and §5.3's approval-workflow audit tier are out of
scope; both would need product decisions (key rotation policy, approval UX)
this plan doesn't make.

---

### Task 11: Rehydration map capture + encrypted `RehydrationStore`

`crates/xberg/src/text/redaction/engine.rs`'s `redact()` deliberately drops
the original PII text after building `RedactionFinding`s — "the original
text never appears in the returned `ExtractedDocument`" (doc comment on
`crates/xberg/src/types/redaction.rs`). This task adds an **opt-in sibling
entry point** that captures a token→original map *before* it's discarded,
encrypts it, and hands back an opaque store key — without changing
`redact()`'s existing behavior or its one existing call site
(`crate::plugins::processor::builtin::redaction`).

**Files:**
- Modify: `crates/xberg/Cargo.toml` (new `redaction-rehydrate` feature + `aes-gcm`/`scrypt` deps)
- Create: `crates/xberg/src/text/redaction/rehydration.rs`
- Modify: `crates/xberg/src/text/redaction/engine.rs` (extract `redact_inner`, add `redact_capturing_rehydration_map`)
- Modify: `crates/xberg/src/text/redaction/mod.rs` (register + re-export)
- Create: `crates/xberg/src/api/rehydration_store.rs`
- Modify: `crates/xberg/src/api/mod.rs` (register module)
- Modify: `crates/xberg/src/api/types.rs` (`ApiState.rehydration_store`)
- Modify: `crates/xberg/src/api/router.rs` (construct the store; both `create_router_with_limits_and_server_config` and the test router builder)

**Interfaces:**
- Produces: `text::redaction::RehydrationMap` (`HashMap<String, String>`, token → original text), `text::redaction::encrypt_map(&RehydrationMap, passphrase: &str) -> Result<Vec<u8>>`, `text::redaction::decrypt_map(&[u8], passphrase: &str) -> Result<RehydrationMap>`, `text::redaction::redact_capturing_rehydration_map(&mut ExtractedDocument, &RedactionConfig) -> Result<Option<RehydrationMap>>`, `api::RehydrationStore` with `store(Vec<u8>) -> String` / `get(&str) -> Option<Vec<u8>>` (consumed by Task 12's `/v1/process` handler and Task 13's `/v1/documents/{id}/rehydrate` handler).

- [ ] **Step 1: Add the `redaction-rehydrate` feature**

In `crates/xberg/Cargo.toml`, add to `[dependencies]` (alongside the existing `sha2` line):

```toml
aes-gcm = { version = "0.10", optional = true }
scrypt = { version = "0.11", optional = true, default-features = false, features = ["std"] }
```

Note: verify these are still the latest stable 0.x at implementation time
with `cargo add aes-gcm scrypt --dry-run -p xberg` — both crates have been
stable at these majors for a long time, but don't take the pin on faith.
`aes-gcm`'s default features must include `getrandom`/`os_rng` so
`aes_gcm::aead::{OsRng, rand_core}` are available (Step 3 uses both) — if
`cargo build` reports `OsRng`/`rand_core` not found under `aes_gcm::aead`,
add `features = ["getrandom"]` explicitly (exact feature name may differ —
check `aes-gcm`'s current `Cargo.toml` on docs.rs).

Add to `[features]` (alongside `redaction = ["dep:sha2"]`):

```toml
redaction-rehydrate = ["redaction", "dep:aes-gcm", "dep:scrypt"]
```

- [ ] **Step 2: Write the failing tests for encrypt/decrypt round-trip**

Create `crates/xberg/src/text/redaction/rehydration.rs` with just the test module first:

```rust
//! Encrypted rehydration map: token → original PII text.
//!
//! Wire format matches the project-wide convention documented for the PII
//! pipeline: `XPII\x01` magic + 16-byte salt + 12-byte nonce + 16-byte GCM
//! tag + ciphertext. Key derivation is scrypt(passphrase, salt, N=2^15,
//! r=8, p=1) → 32 bytes, matching CLAUDE.md's `pii-pipeline` rule exactly.
//! The passphrase is never cached — callers must supply it at both
//! `encrypt_map` and `decrypt_map` call sites.

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn round_trips_through_encrypt_decrypt() {
        let mut map = HashMap::new();
        map.insert("[EMAIL_1]".to_string(), "alice@example.com".to_string());
        map.insert("[PERSON_1]".to_string(), "Alice Smith".to_string());

        let encrypted = encrypt_map(&map, "correct horse battery staple").expect("encrypt");
        let decrypted = decrypt_map(&encrypted, "correct horse battery staple").expect("decrypt");
        assert_eq!(decrypted, map);
    }

    #[test]
    fn wrong_passphrase_fails_to_decrypt() {
        let mut map = HashMap::new();
        map.insert("[EMAIL_1]".to_string(), "alice@example.com".to_string());

        let encrypted = encrypt_map(&map, "correct passphrase").expect("encrypt");
        let err = decrypt_map(&encrypted, "wrong passphrase").expect_err("must fail with wrong passphrase");
        assert!(err.to_string().to_ascii_lowercase().contains("decrypt"));
    }

    #[test]
    fn each_encryption_uses_a_fresh_salt_and_nonce() {
        let mut map = HashMap::new();
        map.insert("[EMAIL_1]".to_string(), "alice@example.com".to_string());

        let a = encrypt_map(&map, "same passphrase").expect("encrypt a");
        let b = encrypt_map(&map, "same passphrase").expect("encrypt b");
        assert_ne!(a, b, "identical plaintext + passphrase must still produce different ciphertext");
    }

    #[test]
    fn magic_bytes_are_present() {
        let map = HashMap::new();
        let encrypted = encrypt_map(&map, "x").expect("encrypt empty map");
        assert_eq!(&encrypted[..5], b"XPII\x01");
    }
}
```

Run: `cargo test -p xberg --features redaction-rehydrate round_trips_through_encrypt_decrypt wrong_passphrase_fails_to_decrypt each_encryption_uses_a_fresh_salt_and_nonce magic_bytes_are_present --no-run`
Expected: FAIL — `encrypt_map`/`decrypt_map` don't exist.

- [ ] **Step 3: Write the implementation**

Add above the test module in `crates/xberg/src/text/redaction/rehydration.rs`:

```rust
use std::collections::HashMap;

use aes_gcm::aead::rand_core::RngCore;
use aes_gcm::aead::{AeadInPlace, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use scrypt::Params as ScryptParams;

use crate::{Result, XbergError};

/// Token → original PII text. Built by [`super::engine::redact_capturing_rehydration_map`],
/// consumed only by [`encrypt_map`] — never serialized into [`crate::types::ExtractedDocument`].
pub type RehydrationMap = HashMap<String, String>;

const MAGIC: &[u8; 5] = b"XPII\x01";
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;
const KEY_LEN: usize = 32;
/// scrypt cost parameter `N = 2^15 = 32768`, per CLAUDE.md's `pii-pipeline` rule.
const SCRYPT_LOG_N: u8 = 15;
const SCRYPT_R: u32 = 8;
const SCRYPT_P: u32 = 1;

fn derive_key(passphrase: &str, salt: &[u8]) -> Result<[u8; KEY_LEN]> {
    let params = ScryptParams::new(SCRYPT_LOG_N, SCRYPT_R, SCRYPT_P, KEY_LEN)
        .map_err(|error| XbergError::validation_with_source("invalid scrypt parameters", error))?;
    let mut key = [0u8; KEY_LEN];
    scrypt::scrypt(passphrase.as_bytes(), salt, &params, &mut key)
        .map_err(|error| XbergError::validation_with_source("scrypt key derivation failed", error))?;
    Ok(key)
}

/// Encrypt `map` under `passphrase`. Returns `XPII\x01` + salt(16) + nonce(12)
/// + tag(16) + ciphertext. A fresh random salt and nonce are generated on
/// every call, so encrypting the same map twice with the same passphrase
/// produces different output (semantic security; required for GCM nonce reuse safety).
pub fn encrypt_map(map: &RehydrationMap, passphrase: &str) -> Result<Vec<u8>> {
    let plaintext = serde_json::to_vec(map)
        .map_err(|error| XbergError::serialization_with_source("failed to serialize rehydration map", error))?;

    let mut salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);
    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);

    let key_bytes = derive_key(passphrase, &salt)?;
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));
    let nonce = Nonce::from_slice(&nonce_bytes);

    let mut buffer = plaintext;
    let tag = cipher
        .encrypt_in_place_detached(nonce, b"", &mut buffer)
        .map_err(|error| XbergError::validation(format!("AES-256-GCM encryption failed: {error}")))?;

    let mut out = Vec::with_capacity(MAGIC.len() + SALT_LEN + NONCE_LEN + TAG_LEN + buffer.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&salt);
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(tag.as_slice());
    out.extend_from_slice(&buffer);
    Ok(out)
}

/// Decrypt a blob produced by [`encrypt_map`]. Fails (does not panic) on a
/// wrong passphrase, truncated input, or tampered ciphertext (GCM tag check).
pub fn decrypt_map(blob: &[u8], passphrase: &str) -> Result<RehydrationMap> {
    let min_len = MAGIC.len() + SALT_LEN + NONCE_LEN + TAG_LEN;
    if blob.len() < min_len || &blob[..MAGIC.len()] != MAGIC {
        return Err(XbergError::validation(
            "rehydration blob is too short or missing the XPII magic header",
        ));
    }

    let mut offset = MAGIC.len();
    let salt = &blob[offset..offset + SALT_LEN];
    offset += SALT_LEN;
    let nonce_bytes = &blob[offset..offset + NONCE_LEN];
    offset += NONCE_LEN;
    let tag = &blob[offset..offset + TAG_LEN];
    offset += TAG_LEN;
    let ciphertext = &blob[offset..];

    let key_bytes = derive_key(passphrase, salt)?;
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));
    let nonce = Nonce::from_slice(nonce_bytes);

    let mut buffer = ciphertext.to_vec();
    cipher
        .decrypt_in_place_detached(nonce, b"", &mut buffer, tag.into())
        .map_err(|_error| {
            XbergError::validation("failed to decrypt rehydration map — wrong passphrase or corrupted data")
        })?;

    serde_json::from_slice(&buffer)
        .map_err(|error| XbergError::serialization_with_source("failed to deserialize rehydration map", error))
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p xberg --features redaction-rehydrate round_trips_through_encrypt_decrypt wrong_passphrase_fails_to_decrypt each_encryption_uses_a_fresh_salt_and_nonce magic_bytes_are_present -- --nocapture`
Expected: all four PASS.

- [ ] **Step 5: Capture the map inside `redact()` without changing its signature**

In `crates/xberg/src/text/redaction/engine.rs`, rename the existing `pub async fn redact(...)` body to `async fn redact_inner`, add a `rehydration_map: Option<&mut RehydrationMap>` parameter, and add two thin public wrappers. This keeps the one existing call site
(`crate::plugins::processor::builtin::redaction`, `redact(result, redaction_config).await`) compiling unchanged.

```rust
use super::rehydration::RehydrationMap;

/// Existing public entry point — unchanged behavior, unchanged call site.
pub async fn redact(result: &mut ExtractedDocument, config: &RedactionConfig) -> Result<()> {
    redact_inner(result, config, None).await
}

/// Like [`redact`], but also captures a token→original map for every
/// [`crate::types::redaction::RedactionStrategy::TokenReplace`] finding in
/// `result.content` (not `formatted_content`/chunks/etc. — those re-run the
/// pattern engine independently and aren't reflected in `redaction_report`
/// either; extending capture to them is a follow-up if a caller needs it).
/// Returns `Err` if `config.strategy` isn't `TokenReplace` — rehydration is
/// only meaningful for that strategy; `Mask`/`Hash`/`Drop` destroy the
/// information needed to restore the original value.
pub async fn redact_capturing_rehydration_map(
    result: &mut ExtractedDocument,
    config: &RedactionConfig,
) -> Result<Option<RehydrationMap>> {
    if config.strategy != crate::types::redaction::RedactionStrategy::TokenReplace {
        return Err(crate::XbergError::validation(
            "redact_capturing_rehydration_map requires RedactionConfig.strategy == TokenReplace",
        ));
    }
    let mut map = RehydrationMap::new();
    redact_inner(result, config, Some(&mut map)).await?;
    Ok(if map.is_empty() { None } else { Some(map) })
}

async fn redact_inner(
    result: &mut ExtractedDocument,
    config: &RedactionConfig,
    mut rehydration_map: Option<&mut RehydrationMap>,
) -> Result<()> {
```

Change the function body's existing signature line (currently `pub async fn redact(result: &mut ExtractedDocument, config: &RedactionConfig) -> Result<()> {`) to the `redact_inner` signature above, and in the findings-building loop (the block starting `let mut counter = TokenCounter::new();` / `for m in &matches { ... }`), capture the map entry right after computing `replacement`:

```rust
    let mut counter = TokenCounter::new();
    let mut findings: Vec<RedactionFinding> = Vec::with_capacity(matches.len());
    for m in &matches {
        let replacement = apply_strategy(config.strategy, &m.text, &m.category, &mut counter);
        if let Some(map) = rehydration_map.as_deref_mut() {
            map.entry(replacement.clone()).or_insert_with(|| m.text.clone());
        }
        findings.push(RedactionFinding {
            start: m.start as u32,
            end: m.end as u32,
            category: m.category.clone(),
            strategy: config.strategy,
            replacement_token: replacement,
        });
    }
```

`.entry(...).or_insert_with(...)` (not a plain overwrite) matters here:
`TokenCounter` already memoizes so the same original value always maps to
the same token within a document (`strategy.rs`'s `cache` field) — `entry`
just makes that invariant explicit at the map-capture site too, rather than
relying on it silently holding.

- [ ] **Step 6: Run the existing redaction test suite to confirm no regression**

Run: `cargo test -p xberg --features redaction text::redaction -- --nocapture`
Expected: every pre-existing test in `engine.rs`/`strategy.rs` still PASSES — `redact_inner`'s extra parameter is additive and `redact()`'s call site forwards `None`, so behavior for every existing caller is unchanged.

- [ ] **Step 7: Register the new module**

In `crates/xberg/src/text/redaction/mod.rs`:

```rust
#[cfg(feature = "redaction-rehydrate")]
pub mod rehydration;

pub use engine::redact;
#[cfg(feature = "redaction-rehydrate")]
pub use engine::redact_capturing_rehydration_map;
#[cfg(feature = "redaction-rehydrate")]
pub use rehydration::{RehydrationMap, decrypt_map, encrypt_map};
```

Gate the new code in `engine.rs` (the `redact_capturing_rehydration_map` fn
and its `use super::rehydration::RehydrationMap;` import) behind
`#[cfg(feature = "redaction-rehydrate")]` so `redaction`-only builds (the
existing, smaller feature) don't pull in `aes-gcm`/`scrypt`.

- [ ] **Step 8: Write the `RehydrationStore`**

Create `crates/xberg/src/api/rehydration_store.rs`, mirroring
`crates/xberg/src/api/jobs.rs`'s `JobStore` pattern exactly (same
`moka::sync::Cache` choice, same TTL-eviction-not-background-task rationale):

```rust
//! In-memory store for encrypted rehydration map blobs, keyed by an opaque
//! `rehydration_key`. Mirrors `JobStore`'s TTL-cache pattern — entries expire
//! automatically, no background eviction task. Server restarts clear all
//! stored maps (same trust boundary as `JobStore`'s job results).

use std::time::Duration;

use moka::sync::Cache;

/// Default TTL for stored rehydration blobs (24 hours). Conservative default
/// for the MVP — the design spec's audit-logged tier (§5.3) describes a
/// 90-day configurable expiry, which this task does not implement (see Part
/// 3's scope note).
const REHYDRATION_TTL: Duration = Duration::from_secs(24 * 60 * 60);

/// Maximum number of concurrently-stored rehydration blobs.
const MAX_CAPACITY: u64 = 10_000;

#[derive(Clone)]
pub struct RehydrationStore {
    blobs: Cache<String, Vec<u8>>,
}

impl Default for RehydrationStore {
    fn default() -> Self {
        Self::new()
    }
}

impl RehydrationStore {
    pub fn new() -> Self {
        let blobs = Cache::builder()
            .max_capacity(MAX_CAPACITY)
            .time_to_live(REHYDRATION_TTL)
            .build();
        Self { blobs }
    }

    /// Store an encrypted blob (from [`crate::text::redaction::encrypt_map`])
    /// and return its `rehydration_key`, formatted `reh_{uuid}` per the
    /// design spec's example (`"reh_e4f8a1b2c3d4..."`).
    pub fn store(&self, encrypted: Vec<u8>) -> String {
        let key = format!("reh_{}", uuid::Uuid::new_v4());
        self.blobs.insert(key.clone(), encrypted);
        key
    }

    /// Retrieve the encrypted blob for `key`, if present and not expired.
    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.blobs.get(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_then_get_round_trips() {
        let store = RehydrationStore::new();
        let key = store.store(vec![1, 2, 3]);
        assert!(key.starts_with("reh_"));
        assert_eq!(store.get(&key), Some(vec![1, 2, 3]));
    }

    #[test]
    fn get_missing_key_returns_none() {
        let store = RehydrationStore::new();
        assert_eq!(store.get("reh_nonexistent"), None);
    }

    #[test]
    fn each_store_call_gets_a_distinct_key() {
        let store = RehydrationStore::new();
        let a = store.store(vec![1]);
        let b = store.store(vec![1]);
        assert_ne!(a, b);
    }
}
```

Run: `cargo test -p xberg --features api,redaction-rehydrate rehydration_store:: -- --nocapture`
Expected: all three PASS.

- [ ] **Step 9: Wire `RehydrationStore` into `ApiState`**

In `crates/xberg/src/api/mod.rs`, add alongside the existing `jobs` declaration:

```rust
#[cfg(feature = "api")]
pub(crate) mod rehydration_store;
```

In `crates/xberg/src/api/types.rs`, add to `ApiState` (after `job_store`):

```rust
    /// In-memory store for encrypted PII rehydration maps.
    #[cfg(feature = "api")]
    pub rehydration_store: Arc<super::rehydration_store::RehydrationStore>,
```

In `crates/xberg/src/api/router.rs`, add to **every** `ApiState { ... }`
construction site — `create_router_with_limits_and_server_config` (line
~132) and the test router builder — the new field:

```rust
        #[cfg(feature = "api")]
        rehydration_store: Arc::new(super::rehydration_store::RehydrationStore::new()),
```

Run: `cargo build -p xberg --features api,redaction-rehydrate`
Expected: compiles clean — this surfaces every `ApiState { ... }` literal
that needs the new field (the compiler will error on any missed site with
"missing field `rehydration_store`").

- [ ] **Step 10: Commit**

```bash
git add crates/xberg/Cargo.toml crates/xberg/src/text/redaction/ \
        crates/xberg/src/api/rehydration_store.rs crates/xberg/src/api/mod.rs \
        crates/xberg/src/api/types.rs crates/xberg/src/api/router.rs
git commit -m "feat(redaction): add encrypted rehydration map capture and store"
```

---

### Task 12: `POST /v1/process` handler

**Scope narrowing vs. the design spec:** the spec's `POST /v1/process`
(§3.1) accepts `file`/`url`/`text` via multipart and a much larger
`operations` object (`transcribe`, `classify`, `chunk`, `embed`, …). This
task ships exactly what the plan's own architecture line promises — extract
→ ner(adapter) → pii.detect → redact — over **`text` or `url` JSON input
only**. Multipart file upload is deliberately deferred: it would mean either
duplicating `UnifiedExtractRequest`'s ~200-line `FromRequest` impl
(`crates/xberg/src/api/handlers.rs`) or modifying it to carry new
`/v1/process`-specific fields it wasn't designed for, neither of which is a
small change. Flag this to the user as a follow-up task once V1 lands.

**Files:**
- Modify: `crates/xberg/src/api/types.rs` (`ProcessRequest`, `ProcessOperations`, `ProcessRedactOperation`, `ProcessResponse`)
- Modify: `crates/xberg/src/api/handlers.rs` (`process_handler`)
- Modify: `crates/xberg/src/api/router.rs` (route registration)

**Interfaces:**
- Produces: `POST /v1/process` (consumed by API clients; no other task in this plan depends on these types).

- [ ] **Step 1: Write the failing integration test**

Add to `crates/xberg/src/api/handlers.rs`'s `#[cfg(test)] mod tests`:

```rust
#[tokio::test]
async fn process_handler_redacts_email_with_mask_strategy() {
    let request = ProcessRequest {
        text: Some("Contact Alice at alice@example.com.".to_string()),
        url: None,
        operations: ProcessOperations {
            ner: None,
            redact: Some(ProcessRedactOperation {
                config: crate::core::config::redaction::RedactionConfig {
                    strategy: crate::types::redaction::RedactionStrategy::Mask,
                    ..Default::default()
                },
                rehydrate: false,
                passphrase: None,
            }),
        },
    };
    let state = test_state();
    let response = process_handler(State(state), Json(request)).await.expect("handler must succeed");
    assert!(response.0.document.content.contains("[REDACTED]"));
    assert!(!response.0.document.content.contains("alice@example.com"));
    assert!(response.0.rehydration_key.is_none());
}

#[tokio::test]
async fn process_handler_requires_passphrase_when_rehydrate_is_true() {
    let request = ProcessRequest {
        text: Some("Contact Alice at alice@example.com.".to_string()),
        url: None,
        operations: ProcessOperations {
            ner: None,
            redact: Some(ProcessRedactOperation {
                config: crate::core::config::redaction::RedactionConfig {
                    strategy: crate::types::redaction::RedactionStrategy::TokenReplace,
                    ..Default::default()
                },
                rehydrate: true,
                passphrase: None,
            }),
        },
    };
    let state = test_state();
    let result = process_handler(State(state), Json(request)).await;
    assert!(result.is_err(), "must reject rehydrate=true without a passphrase");
}

#[tokio::test]
async fn process_handler_rejects_both_text_and_url() {
    let request = ProcessRequest {
        text: Some("hello".to_string()),
        url: Some("https://example.com/doc.txt".to_string()),
        operations: ProcessOperations::default(),
    };
    let state = test_state();
    let result = process_handler(State(state), Json(request)).await;
    assert!(result.is_err(), "must reject when both text and url are set");
}
```

(`test_state()` is the existing test helper already used by this module's
other handler tests — check the existing `#[cfg(test)] mod tests` block for
its exact name/signature before reusing; if it doesn't construct
`rehydration_store`, that's covered by Task 11 Step 9's compiler-driven fix.)

Run: `cargo test -p xberg --features api,redaction-rehydrate,ner process_handler_ --no-run`
Expected: FAIL — `process_handler`/`ProcessRequest`/etc. don't exist.

- [ ] **Step 2: Write the request/response types**

Add to `crates/xberg/src/api/types.rs`:

```rust
/// Request body for `POST /v1/process`. JSON only — see Task 12's scope note
/// for why multipart file upload isn't included in this version.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct ProcessRequest {
    /// Raw text to process. Exactly one of `text`/`url` is required.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// URL of the document to fetch and process. Exactly one of `text`/`url` is required.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Declarative operations pipeline.
    #[serde(default)]
    pub operations: ProcessOperations,
}

/// Operations `POST /v1/process` can run. Only `ner` and `redact` are
/// implemented — the design spec's `transcribe`/`classify`/`chunk`/`embed`
/// are out of scope for this task (see Task 12's scope note).
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct ProcessOperations {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ner: Option<crate::core::config::ner::NerConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub redact: Option<ProcessRedactOperation>,
}

/// Redaction operation parameters, extending [`crate::core::config::redaction::RedactionConfig`]
/// with the rehydration opt-in this plan adds (Task 11).
///
/// Note: `#[serde(flatten)]` + `utoipa::ToSchema` together are a known rough
/// edge in utoipa (flattened fields sometimes don't surface correctly in the
/// generated OpenAPI schema, version-dependent). If `cargo build --features
/// api` or the OpenAPI snapshot test fails on this struct, drop `flatten`
/// and inline the `RedactionConfig` fields directly instead of chasing a
/// utoipa workaround.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct ProcessRedactOperation {
    #[serde(flatten)]
    pub config: crate::core::config::redaction::RedactionConfig,
    /// When `true`, capture an encrypted rehydration map and return a
    /// `rehydration_key` in the response. Requires `config.strategy ==
    /// token_replace` and `passphrase` to be set.
    #[serde(default)]
    pub rehydrate: bool,
    /// Passphrase used to encrypt the rehydration map. Required when
    /// `rehydrate` is `true`. Never logged, never cached beyond this request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub passphrase: Option<String>,
}

/// Response body for `POST /v1/process`.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct ProcessResponse {
    pub document: crate::types::ExtractedDocument,
    /// Opaque key to pass to `POST /v1/documents/{id}/rehydrate` (Task 13).
    /// Present only when `operations.redact.rehydrate` was `true` and at
    /// least one `TokenReplace` finding was captured.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rehydration_key: Option<String>,
}
```

`ProcessRedactOperation.passphrase` deliberately has no `Debug`-hiding
treatment beyond what `RedactionConfig` itself has — flag in code review
whether request logging middleware redacts `ProcessRequest` bodies; this
plan does not add request-body logging, but if a future change does, make
sure passphrases never land in logs (consistent with CLAUDE.md's
`pii-pipeline` rule on not caching passphrases beyond the call).

- [ ] **Step 3: Write the handler**

Add to `crates/xberg/src/api/handlers.rs`:

```rust
#[cfg_attr(feature = "otel", tracing::instrument(name = "api.process", skip(state, request)))]
pub(crate) async fn process_handler(
    State(state): State<ApiState>,
    Json(request): Json<ProcessRequest>,
) -> Result<Json<super::types::ProcessResponse>, ApiError> {
    let input = match (&request.text, &request.url) {
        (Some(text), None) => ApiExtractInput::Bytes {
            data: Bytes::from(text.clone().into_bytes()),
            mime_type: "text/plain".to_string(),
            file_name: None,
        },
        (None, Some(url)) => ApiExtractInput::Uri {
            uri: url.clone(),
            mime_type: None,
        },
        (Some(_), Some(_)) => {
            return Err(ApiError::validation(crate::error::XbergError::validation(
                "Exactly one of `text` or `url` must be set, not both",
            )));
        }
        (None, None) => {
            return Err(ApiError::validation(crate::error::XbergError::validation(
                "Exactly one of `text` or `url` must be set",
            )));
        }
    };

    enforce_api_uri_policy(std::slice::from_ref(&input))?;

    let mut config = (*state.default_config).clone();
    config.ner = request.operations.ner.clone();

    let rehydrate = request
        .operations
        .redact
        .as_ref()
        .map(|r| r.rehydrate)
        .unwrap_or(false);

    if rehydrate {
        let redact_op = request.operations.redact.as_ref().expect("rehydrate implies redact is Some");
        let passphrase = redact_op.passphrase.as_deref().ok_or_else(|| {
            ApiError::validation(crate::error::XbergError::validation(
                "operations.redact.passphrase is required when operations.redact.rehydrate is true",
            ))
        })?;

        // Deliberately do NOT set config.redaction here — the standard
        // pipeline's Late-stage redaction post-processor uses plain
        // `redact()`, which discards the original text before this handler
        // could capture it. Run extraction (with NER, no redaction), then
        // redact separately via the capturing entry point.
        let mut results = extract_unified_inputs(vec![input], config).await?;
        let mut document = results
            .results
            .pop()
            .ok_or_else(|| ApiError::internal(crate::error::XbergError::Other("extraction produced no document".into())))?;

        let map = crate::text::redaction::redact_capturing_rehydration_map(&mut document, &redact_op.config)
            .await
            .map_err(ApiError::from)?;

        let rehydration_key = match map {
            Some(map) => {
                let encrypted = crate::text::redaction::encrypt_map(&map, passphrase).map_err(ApiError::from)?;
                Some(state.rehydration_store.store(encrypted))
            }
            None => None,
        };

        Ok(Json(super::types::ProcessResponse { document, rehydration_key }))
    } else {
        if let Some(redact_op) = &request.operations.redact {
            config.redaction = Some(redact_op.config.clone());
        }
        let mut results = extract_unified_inputs(vec![input], config).await?;
        let document = results
            .results
            .pop()
            .ok_or_else(|| ApiError::internal(crate::error::XbergError::Other("extraction produced no document".into())))?;
        Ok(Json(super::types::ProcessResponse { document, rehydration_key: None }))
    }
}
```

Add `#[cfg(feature = "api")]` and a `utoipa::path(...)` annotation matching
the style of `extract_handler` (request_body content-type `application/json`
this time, not multipart) before committing — omitted above for brevity, not
because it's optional; every other handler in this file has one.

Note: `ApiError::from` for `crate::XbergError` — confirm the existing
`impl From<XbergError> for ApiError` (used elsewhere in this file, e.g.
`extract_unified_inputs`'s `.map_err(ApiError::from)`) covers
`text::redaction::engine`'s `Result<_, crate::XbergError>` return type
without an explicit mapping — it should, since `redact_capturing_rehydration_map`
and `encrypt_map` both return `crate::Result<T> = Result<T, XbergError>`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p xberg --features api,redaction-rehydrate,ner process_handler_ -- --nocapture`
Expected: all three PASS.

- [ ] **Step 5: Register the route**

In `crates/xberg/src/api/router.rs`, add alongside the existing `/process` (OpenWebUI compat) and `/v1/convert/file` routes:

```rust
        .route("/v1/process", post(process_handler))
```

Note the existing `PUT /process` (OpenWebUI compat, line ~183) is a
**different, unrelated route** — `/v1/process` (this task, `POST`) vs
`/process` (OpenWebUI compat, `PUT`). Axum distinguishes them by full path,
so no collision, but double-check the route table after adding this doesn't
visually conflate the two for future readers — consider a comment.

- [ ] **Step 6: Commit**

```bash
git add crates/xberg/src/api/types.rs crates/xberg/src/api/handlers.rs crates/xberg/src/api/router.rs
git commit -m "feat(api): add POST /v1/process (extract -> ner -> redact, optional rehydration)"
```

---

### Task 13: `POST /v1/documents/{id}/rehydrate`

**Scope fit vs. the design spec:** spec §3.5 describes a document store
keyed by `document_id` with a `rehydrated_text` response (the full document
with tokens substituted back) and per-finding `restorations` (category +
position). This plan has **no document store** — `/v1/process` (Task 12)
returns the document directly in its response and only persists the
*encrypted rehydration map* (token → original text), not the document
itself. Re-rendering `rehydrated_text` would require either persisting full
documents (a real document store — out of scope) or having the caller
resubmit the redacted text for substitution (extra complexity this task
doesn't need). This task therefore:
- Uses the **`rehydration_key` itself as the `{id}` path segment** (it's
  already the only identifier this plan's storage layer has — no document id
  exists to put there instead).
- Returns the **decrypted token→original map**, not a re-rendered document.
  Callers substitute tokens back into their own copy of the redacted text.
  Flag this divergence in API docs/changelog when this ships — it is a
  smaller surface than spec §3.5 describes.

**Files:**
- Modify: `crates/xberg/src/api/types.rs` (`RehydrateRequest`, `RehydrateResponse`)
- Modify: `crates/xberg/src/api/handlers.rs` (`rehydrate_handler`)
- Modify: `crates/xberg/src/api/router.rs` (route registration)

**Interfaces:**
- Produces: `POST /v1/documents/{rehydration_key}/rehydrate` (terminal endpoint — no other task consumes its output).

- [ ] **Step 1: Write the failing test**

Add to `crates/xberg/src/api/handlers.rs`'s test module:

```rust
#[tokio::test]
async fn rehydrate_handler_round_trips_a_stored_map() {
    let state = test_state();
    let mut map = std::collections::HashMap::new();
    map.insert("[EMAIL_1]".to_string(), "alice@example.com".to_string());
    let encrypted = crate::text::redaction::encrypt_map(&map, "test-passphrase").expect("encrypt");
    let key = state.rehydration_store.store(encrypted);

    let response = rehydrate_handler(
        State(state),
        axum::extract::Path(key.clone()),
        Json(RehydrateRequest { passphrase: "test-passphrase".to_string() }),
    )
    .await
    .expect("rehydrate must succeed");

    assert_eq!(response.0.restored.get("[EMAIL_1]"), Some(&"alice@example.com".to_string()));
}

#[tokio::test]
async fn rehydrate_handler_rejects_wrong_passphrase() {
    let state = test_state();
    let mut map = std::collections::HashMap::new();
    map.insert("[EMAIL_1]".to_string(), "alice@example.com".to_string());
    let encrypted = crate::text::redaction::encrypt_map(&map, "correct").expect("encrypt");
    let key = state.rehydration_store.store(encrypted);

    let result = rehydrate_handler(
        State(state),
        axum::extract::Path(key),
        Json(RehydrateRequest { passphrase: "wrong".to_string() }),
    )
    .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn rehydrate_handler_returns_404_for_unknown_key() {
    let state = test_state();
    let result = rehydrate_handler(
        State(state),
        axum::extract::Path("reh_does_not_exist".to_string()),
        Json(RehydrateRequest { passphrase: "anything".to_string() }),
    )
    .await;
    let err = result.expect_err("unknown key must error");
    assert_eq!(err.status, axum::http::StatusCode::NOT_FOUND);
}
```

Run: `cargo test -p xberg --features api,redaction-rehydrate rehydrate_handler_ --no-run`
Expected: FAIL — `rehydrate_handler`/`RehydrateRequest`/`RehydrateResponse` don't exist.

- [ ] **Step 2: Write the request/response types**

Add to `crates/xberg/src/api/types.rs`:

```rust
/// Request body for `POST /v1/documents/{id}/rehydrate`.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct RehydrateRequest {
    /// Passphrase the map was encrypted with (Task 12's `operations.redact.passphrase`).
    /// Never logged, never cached beyond this request.
    pub passphrase: String,
}

/// Response body for `POST /v1/documents/{id}/rehydrate`: the decrypted
/// token → original-text map. See Task 13's scope note for why this
/// returns the map rather than a re-rendered document.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct RehydrateResponse {
    pub restored: std::collections::HashMap<String, String>,
}
```

- [ ] **Step 3: Write the handler**

Add to `crates/xberg/src/api/handlers.rs`:

```rust
#[cfg_attr(feature = "otel", tracing::instrument(name = "api.rehydrate", skip(state, request), fields(rehydration_key = %rehydration_key)))]
pub(crate) async fn rehydrate_handler(
    State(state): State<ApiState>,
    axum::extract::Path(rehydration_key): axum::extract::Path<String>,
    Json(request): Json<super::types::RehydrateRequest>,
) -> Result<Json<super::types::RehydrateResponse>, ApiError> {
    let encrypted = state.rehydration_store.get(&rehydration_key).ok_or_else(|| ApiError {
        status: axum::http::StatusCode::NOT_FOUND,
        body: super::types::ErrorResponse {
            error_type: "NotFoundError".to_string(),
            message: format!("Rehydration key '{rehydration_key}' not found or expired"),
            traceback: None,
            status_code: axum::http::StatusCode::NOT_FOUND.as_u16(),
        },
    })?;

    let restored = crate::text::redaction::decrypt_map(&encrypted, &request.passphrase).map_err(|error| {
        ApiError::new(axum::http::StatusCode::FORBIDDEN, error)
    })?;

    tracing::info!(
        target: "xberg::rehydrate",
        rehydration_key = %rehydration_key,
        restored_count = restored.len(),
        "PII rehydration performed"
    );

    Ok(Json(super::types::RehydrateResponse { restored }))
}
```

The `tracing::info!` call logs `restored_count`, never the decrypted values
— matches CLAUDE.md's `pii-pipeline` rule ("Never log detected PII values —
log only category counts"). This is the audit trail for rehydration access;
do not change it to log `restored` directly.

Add `#[cfg(feature = "api")]` and a `utoipa::path(...)` annotation before
committing, matching the existing handler style (see Task 12, Step 3's note
— every handler in this file has one).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p xberg --features api,redaction-rehydrate rehydrate_handler_ -- --nocapture`
Expected: all three PASS.

- [ ] **Step 5: Register the route**

In `crates/xberg/src/api/router.rs`:

```rust
        .route("/v1/documents/{rehydration_key}/rehydrate", post(rehydrate_handler))
```

- [ ] **Step 6: Commit**

```bash
git add crates/xberg/src/api/types.rs crates/xberg/src/api/handlers.rs crates/xberg/src/api/router.rs
git commit -m "feat(api): add POST /v1/documents/{id}/rehydrate"
```

---

## Part 3 summary: what shipped vs. the design spec

| Spec surface (§3) | This plan | Why |
|---|---|---|
| `POST /v1/process` | Implemented, JSON-only (`text`/`url`), `ner` + `redact` operations | Task 12 scope note |
| `GET /v1/tasks/{id}` (async) | Not implemented | `/v1/process` is synchronous only — async job tracking via the existing `JobStore`/`/extract-async` pattern is a follow-up if large-document latency becomes an issue |
| `POST /v1/documents/{id}/rehydrate` | Implemented, returns token map not re-rendered document | Task 13 scope note |
| `POST /v1/search`, `/v1/collections/*` | Not implemented | Out of scope per Part 3's intro — owned by `2026-06-29-xberg-privacy-api.md` Phase 4 |
| `POST /v1/ner`, `POST /v1/classify` (standalone) | Not implemented | Not in this plan's architecture line; `ner` is reachable only via `/v1/process` |
| Rehydration strategies §5.1/§5.3 (server-key, audit-logged) | Not implemented — only §5.2-equivalent (caller-supplied passphrase) | Task 11 scope note |

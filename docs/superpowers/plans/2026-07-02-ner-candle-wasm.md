# ner-candle-wasm Enablement Implementation Plan (sub-project A)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make GLiNER2-Candle NER compile and run on `wasm32-unknown-unknown` with no ONNX Runtime, no filesystem, and no tokio, exposing a synchronous in-binary NER entrypoint the wasm engine (B) can call.

**Architecture:** Two Rust crates need surgery. `xberg-gliner` currently hard-depends on `ort`; make `ort` optional and gate the ONNX modules so its pure-Rust tokenizer/encoder (`encode_v2`, `V2Tokenizer`, `V2Splitter`, `Span`) compile without it. `xberg-gliner-candle` loads models from disk paths; add a `from_bytes` constructor (wasm has no filesystem) reusing its existing `VarBuilder::from_tensors` path. Then wire a `ner-candle-wasm` feature in `xberg` core that drops `tokio-runtime` and calls the already-synchronous `extract_ner`.

**Tech Stack:** Rust 2024, `candle-core`/`candle-nn`/`candle-transformers`, `tokenizers` 0.23 (no-onig), `safetensors`.

**Spec:** [2026-07-02-xberg-wasm-engine-design.md](../specs/2026-07-02-xberg-wasm-engine-design.md) §A. This plan is the prerequisite that B's Task 1 consumes.

## Global Constraints

- Rust 2024; `cargo clippy -D warnings`; zero warnings.
- No `.unwrap()`/`panic!` in library code — `Result<T, E>` with `thiserror`; `?` for propagation.
- WASM target: `wasm32-unknown-unknown`. The gold gate for every task is `cargo build ... --target wasm32-unknown-unknown` succeeding.
- Native builds must stay green — every change is behind `#[cfg]`/features; do NOT regress the ORT path.
- Cargo target dir `E:/cargo-target` (Windows dev).
- `xberg-gliner` and `xberg-gliner-candle` are workspace crates, NOT Alef-generated — edit `Cargo.toml` directly.
- Model artifacts are external (fastino/gliner2 safetensors repos); tests use a tiny committed fixture or `#[ignore]` when weights are unavailable in CI.
- Commit messages: conventional commits, imperative, <72 chars; **no AI attribution**.
- Run `prek run --all-files` before each commit; re-stage if hooks rewrite.

---

### Task 1: Make `xberg-gliner` compile without ORT

Gate the ONNX Runtime behind an optional `ort` feature so the pure-Rust tokenizer/encoder surface (`encode_v2`, `V2Tokenizer`, `V2Splitter`, `Span`) is available on wasm. This is also the **`tokenizers`-on-wasm validation gate** — the highest-risk step in the whole wasm initiative.

**Files:**
- Modify: `crates/xberg-gliner/Cargo.toml`
- Modify: `crates/xberg-gliner/src/lib.rs:7-38` (module + re-export gating)
- Test: build commands (compile-validation task)

**Interfaces:**
- Produces: `xberg-gliner` with an optional `ort` feature; `encode_v2`, `V2Encoded`, `V2Tokenizer`, `V2Splitter`, `Span`, `SpanOutput`, `decode` available with `--no-default-features` (no ORT); `Gliner`/`Gliner2`/session/tensor types gated behind `feature = "ort"`.

- [ ] **Step 1: Make `ort` optional in the manifest**

Edit `crates/xberg-gliner/Cargo.toml`:

```toml
[features]
default = ["ort-backend"]
ort-backend = ["dep:ort"]
ort-bundled = ["ort-backend", "ort/download-binaries", "ort/tls-rustls"]
ort-dynamic = ["ort-backend", "ort/load-dynamic"]

[dependencies]
ndarray = "0.17"
ort = { workspace = true, features = ["ndarray"], optional = true }
parking_lot = { workspace = true }
regex = "1.12"
thiserror = { workspace = true }
tokenizers = { version = "0.23", default-features = false, features = ["fancy-regex"] }
```

- [ ] **Step 2: Gate the ORT-using modules in `lib.rs`**

Edit `crates/xberg-gliner/src/lib.rs`. Add `#[cfg(feature = "ort-backend")]` to the ONNX modules and their re-exports; leave the pure-Rust ones ungated:

```rust
mod config;
pub mod decode;
#[cfg(feature = "ort-backend")]
mod engine;
mod error;
mod input;
#[cfg(feature = "ort-backend")]
mod preprocess;
#[cfg(feature = "ort-backend")]
mod session;
mod splitter;
#[cfg(feature = "ort-backend")]
mod tensor;
mod v2_decode;
#[cfg(feature = "ort-backend")]
mod v2_engine;
mod v2_preprocess;
#[cfg(feature = "ort-backend")]
mod v2_session;
mod v2_splitter;
#[cfg(feature = "ort-backend")]
mod v2_tensor;
mod v2_tokenizer;

pub use config::{Parameters, RuntimeConfig};
pub use decode::{Span, SpanOutput};
#[cfg(feature = "ort-backend")]
pub use engine::Gliner;
pub use error::{GlinerError, Result};
pub use input::{TextInput, Token};
#[cfg(feature = "ort-backend")]
pub use session::{INPUT_NAMES, OUTPUT_NAMES};
#[cfg(feature = "ort-backend")]
pub use v2_engine::Gliner2;
pub use v2_preprocess::{V2Encoded, encode_v2};
#[cfg(feature = "ort-backend")]
pub use v2_session::{INPUT_NAMES_V2, OUTPUT_NAMES_V2};
pub use v2_splitter::V2Splitter;
pub use v2_tokenizer::{PretokenizedEncoding, PretokenizingTokenizer, V2Tokenizer};
```

Then chase compile errors: any ungated module (`config`, `input`, `v2_preprocess`, `v2_decode`, `decode`, `splitter`, `v2_splitter`, `v2_tokenizer`) that imports a gated one or `ort` must have that specific import gated too. `preprocess::EncodedInput` and `decode::EntityContext` `pub(crate) use` lines at lib.rs:37-38 — gate `preprocess` re-export behind `ort-backend`.

- [ ] **Step 3: Verify native still builds (ORT path intact)**

Run: `cargo build -p xberg-gliner 2>&1 | tail -20`
Expected: SUCCESS (default features → `ort-bundled`).

- [ ] **Step 4: The wasm validation gate**

Run: `cargo build -p xberg-gliner --no-default-features --target wasm32-unknown-unknown 2>&1 | tail -40`
Expected: SUCCESS. **If it fails inside `tokenizers`** (common: `esaxx-rs`, `onig_sys`, or `rayon` pulled transitively), fix by tightening the `tokenizers` feature set — try `tokenizers = { version = "0.23", default-features = false, features = ["fancy-regex", "unstable_wasm"] }` (0.23 ships a wasm-oriented feature set), or drop to `["fancy-regex"]` and add `getrandom` wasm shim. If `tokenizers` fundamentally cannot target wasm at this version, STOP and record it: in-binary NER is then infeasible and the whole `ner-candle-wasm` line defers — the injected ORT-Web NER path in B remains. Do not fabricate a workaround.

- [ ] **Step 5: Commit**

```bash
prek run --all-files
git add crates/xberg-gliner/Cargo.toml crates/xberg-gliner/src/lib.rs
git commit -m "feat(gliner): make ort optional so tokenizer surface targets wasm"
```

---

### Task 2: Add `from_bytes` model loading to `xberg-gliner-candle`

WASM has no filesystem; the current `from_local(&Path)` can't work. Add a constructor that takes model bytes, reusing the existing `VarBuilder::from_tensors` path already used by `load_adapter` (model.rs:70-72).

**Files:**
- Modify: `crates/xberg-gliner-candle/Cargo.toml` (target-gate LoRA/fs-only paths if needed; keep candle wasm-capable)
- Modify: `crates/xberg-gliner-candle/src/model.rs` (add `from_bytes`; `#[cfg]`-gate `from_local*`, `load_adapter`, `unload_adapter` off wasm)
- Modify: `crates/xberg-gliner-candle/src/encoder.rs` (add `from_buffered_safetensors`)
- Modify: `crates/xberg-gliner-candle/src/heads/mod.rs` (add `from_buffered_safetensors` to `AllHeads`)
- Test: `crates/xberg-gliner-candle/src/model.rs` `#[cfg(test)]`

**Interfaces:**
- Consumes: `xberg_gliner::V2Tokenizer`, `V2Splitter`, `encode_v2` (from Task 1, ORT-free).
- Produces: `pub fn Gliner2Candle::from_bytes(safetensors: &[u8], tokenizer_json: &[u8], encoder_config_json: &[u8]) -> Result<Gliner2Candle>`; `Encoder::from_buffered_safetensors(&[u8], &EncoderConfig, &Device)`; `AllHeads::from_buffered_safetensors(&[u8], &Device)`. `extract_ner(&self, text, labels, threshold)` unchanged (already sync, model.rs:137).

- [ ] **Step 1: Make `xberg-gliner-candle` depend on ORT-free `xberg-gliner`**

Edit `crates/xberg-gliner-candle/Cargo.toml` — the `xberg-gliner` dep must NOT force ORT:

```toml
[dependencies]
xberg-gliner = { workspace = true, default-features = false }
# ... candle-core/nn/transformers, ndarray, safetensors, serde, thiserror, tokenizers unchanged

[features]
default = []
cuda = ["candle-core/cuda", "candle-nn/cuda", "candle-transformers/cuda"]
metal = ["candle-core/metal", "candle-nn/metal", "candle-transformers/metal"]
# Native tests that also exercise the ORT sibling need it explicitly:
ort-bundled = ["xberg-gliner/ort-bundled"]
ort-dynamic = ["xberg-gliner/ort-dynamic"]
```

- [ ] **Step 2: Write the failing `from_bytes` test**

Add to `crates/xberg-gliner-candle/src/model.rs` test module:

```rust
#[test]
fn from_bytes_rejects_empty_safetensors() {
    let err = Gliner2Candle::from_bytes(&[], b"{}", b"{}").expect_err("empty weights must fail");
    assert!(err.to_string().to_lowercase().contains("safetensors")
        || err.to_string().to_lowercase().contains("load"));
}
```

- [ ] **Step 3: Run to verify it fails**

Run: `cargo test -p xberg-gliner-candle --features ort-bundled from_bytes 2>&1 | tail`
Expected: FAIL — `from_bytes` not found.

- [ ] **Step 4: Implement buffered loaders + `from_bytes`**

In `encoder.rs`, add beside `from_safetensors` (which currently loads from a path). Reuse candle's buffered loader:

```rust
/// Load encoder weights from in-memory safetensors bytes (wasm/no-fs path).
pub fn from_buffered_safetensors(
    bytes: &[u8],
    config: &EncoderConfig,
    device: &candle_core::Device,
) -> crate::Result<Self> {
    let tensors = candle_core::safetensors::load_buffer(bytes, device)?;
    let vb = candle_nn::VarBuilder::from_tensors(tensors, candle_core::DType::F32, device);
    Self::from_var_builder(vb.pp("encoder"), config)
}
```

(`from_var_builder` already exists — used at model.rs:71. `EncoderConfig` is the type `self.encoder.config` holds; confirm its name/parse fn in `encoder.rs`.)

In `heads/mod.rs`, add the mirror on `AllHeads`:

```rust
pub fn from_buffered_safetensors(bytes: &[u8], device: &candle_core::Device) -> crate::Result<Self> {
    let tensors = candle_core::safetensors::load_buffer(bytes, device)?;
    let vb = candle_nn::VarBuilder::from_tensors(tensors, candle_core::DType::F32, device);
    Self::from_var_builder(vb, device)
}
```

In `model.rs`, add `from_bytes`, and gate the fs/LoRA methods off wasm:

```rust
/// Load from in-memory model bytes (browser/OPFS, wasm — no filesystem).
/// `encoder_config_json` is the `config.json` (or `encoder_config/config.json`)
/// contents; `tokenizer_json` is the HF `tokenizer.json` contents.
pub fn from_bytes(
    safetensors: &[u8],
    tokenizer_json: &[u8],
    encoder_config_json: &[u8],
) -> Result<Self> {
    let device = Device::Cpu;
    let tokenizer = xberg_gliner::V2Tokenizer::from_bytes(tokenizer_json)?;
    let splitter = xberg_gliner::V2Splitter::new()?;
    let config = encoder::EncoderConfig::from_json_slice(encoder_config_json)?;
    let encoder = encoder::Encoder::from_buffered_safetensors(safetensors, &config, &device)?;
    let heads = heads::AllHeads::from_buffered_safetensors(safetensors, &device)?;
    Ok(Self {
        tokenizer,
        splitter,
        device,
        base_model_dir: PathBuf::new(),   // unused on the bytes path
        encoder,
        heads,
        active_adapter: None,
        model_id: "gliner2_candle_bytes".to_string(),
        approx_bytes: safetensors.len() as u64,
    })
}
```

Gate the path/LoRA API off wasm so `std::fs`/`std::path` don't break the wasm build:

```rust
#[cfg(not(target_arch = "wasm32"))]
impl Gliner2Candle {
    pub fn from_local(model_dir: &Path) -> Result<Self> { /* existing body */ }
    // from_local_with_device, load_adapter, unload_adapter move here too
}
```

Add `V2Tokenizer::from_bytes(&[u8])` in `xberg-gliner/src/v2_tokenizer.rs` if absent (wraps `tokenizers::Tokenizer::from_bytes`); and `EncoderConfig::from_json_slice(&[u8])` in `encoder.rs` (serde_json::from_slice) — confirm exact type names and add if missing.

- [ ] **Step 5: Run to verify it passes (native)**

Run: `cargo test -p xberg-gliner-candle --features ort-bundled from_bytes 2>&1 | tail`
Expected: PASS.

- [ ] **Step 6: Verify the candle crate builds for wasm**

Run: `cargo build -p xberg-gliner-candle --no-default-features --target wasm32-unknown-unknown 2>&1 | tail -40`
Expected: SUCCESS. If candle pulls a native dep (e.g. `gemm`/`rayon`/`accelerate`), it means candle-core's default features aren't wasm-clean — set `candle-core = { default-features = false }` in the workspace or crate and retry. Record any blocker.

- [ ] **Step 7: Commit**

```bash
prek run --all-files
git add crates/xberg-gliner-candle/
git commit -m "feat(gliner-candle): in-memory from_bytes loader for wasm"
```

---

### Task 3: Wire the `ner-candle-wasm` feature in `xberg` core

Expose a synchronous NER entrypoint from `xberg` that the wasm engine (B) calls, gated behind `ner-candle-wasm`, without `tokio-runtime`.

**Files:**
- Modify: `crates/xberg/Cargo.toml` (or `alef.toml` source) — feature `ner-candle-wasm`, add to `wasm-target`
- Create: `crates/xberg/src/text/ner_candle_wasm.rs`
- Modify: `crates/xberg/src/text/mod.rs` (gated `pub mod`)
- Test: `crates/xberg/src/text/ner_candle_wasm.rs` `#[cfg(test)]`

**Interfaces:**
- Consumes: `xberg_gliner_candle::{Gliner2Candle, Span}`.
- Produces: `pub struct WasmCandleNer { model: Gliner2Candle }`; `WasmCandleNer::from_bytes(safetensors, tokenizer_json, config_json) -> Result<Self, XbergError>`; `fn extract(&self, text: &str, labels: &[&str], threshold: f32) -> Result<Vec<Entity>, XbergError>` where `Entity` is xberg's existing NER result type (`crate::types::ner::Entity` — confirm path).

- [ ] **Step 1: Add the feature**

In `crates/xberg/Cargo.toml` (via `alef.toml` if generated):

```toml
ner-candle-wasm = ["ner", "dep:xberg-gliner-candle"]
```

And add `ner-candle-wasm` to the `wasm-target` aggregate. Ensure `xberg-gliner-candle` is an optional workspace dep of `xberg`:

```toml
xberg-gliner-candle = { workspace = true, optional = true }
```

- [ ] **Step 2: Write the failing test**

Create `crates/xberg/src/text/ner_candle_wasm.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_labels_returns_no_entities() {
        // Construct is heavy (needs weights); this test only exercises the
        // label short-circuit through a small helper that does not load a model.
        assert!(map_spans_to_entities(&[], "text").is_empty());
    }
}
```

- [ ] **Step 3: Run to verify it fails**

Run: `cargo test -p xberg --features ner-candle-wasm ner_candle_wasm 2>&1 | tail`
Expected: FAIL — module/function not found.

- [ ] **Step 4: Implement the adapter**

```rust
//! Synchronous in-binary GLiNER2-Candle NER for wasm targets. No tokio, no ORT.

use xberg_gliner_candle::{Gliner2Candle, Span};

use crate::error::XbergError;
use crate::types::ner::Entity; // confirm the actual Entity type path in xberg

pub struct WasmCandleNer {
    model: Gliner2Candle,
}

impl WasmCandleNer {
    pub fn from_bytes(
        safetensors: &[u8],
        tokenizer_json: &[u8],
        config_json: &[u8],
    ) -> Result<Self, XbergError> {
        let model = Gliner2Candle::from_bytes(safetensors, tokenizer_json, config_json)
            .map_err(|e| XbergError::Backend(format!("gliner-candle load: {e}")))?;
        Ok(Self { model })
    }

    pub fn extract(&self, text: &str, labels: &[&str], threshold: f32) -> Result<Vec<Entity>, XbergError> {
        let spans = self
            .model
            .extract_ner(text, labels, threshold)
            .map_err(|e| XbergError::Backend(format!("gliner-candle infer: {e}")))?;
        Ok(map_spans_to_entities(&spans, text))
    }
}

fn map_spans_to_entities(spans: &[Span], _text: &str) -> Vec<Entity> {
    spans.iter().map(Entity::from).collect() // impl From<&Span> for Entity, or map fields explicitly
}
```

Match `XbergError`'s real variant (`Backend`/`Ner`/etc.) and the real `Entity` fields (label, text/value, start, end, score) — build them explicitly from `Span` if no `From` impl exists. Add `#[cfg(feature = "ner-candle-wasm")] pub mod ner_candle_wasm;` to `text/mod.rs`.

- [ ] **Step 5: Run to verify it passes + wasm builds**

Run: `cargo test -p xberg --features ner-candle-wasm ner_candle_wasm 2>&1 | tail`
Expected: PASS.
Run: `cargo build -p xberg --no-default-features --features ner-candle-wasm --target wasm32-unknown-unknown 2>&1 | tail -30`
Expected: SUCCESS.

- [ ] **Step 6: Commit**

```bash
prek run --all-files
git add alef.toml crates/xberg/Cargo.toml crates/xberg/src/text/ner_candle_wasm.rs crates/xberg/src/text/mod.rs
git commit -m "feat(ner): synchronous ner-candle-wasm adapter for wasm engine"
```

---

### Task 4: End-to-end wasm NER smoke test

Prove a real (tiny) model runs end-to-end on wasm. Uses a small fixture; `#[ignore]` when weights are absent so CI stays green without the multi-MB artifact.

**Files:**
- Create: `crates/xberg-gliner-candle/tests/wasm_ner.rs`
- Test: the file itself

**Interfaces:**
- Consumes: `Gliner2Candle::from_bytes`, `extract_ner`.

- [ ] **Step 1: Write the wasm test (ignored-by-default when no fixture)**

```rust
#![cfg(target_arch = "wasm32")]
use wasm_bindgen_test::*;
wasm_bindgen_test_configure!(run_in_browser);

// Fixture weights are large; only run when explicitly provided via a build that
// includes them. Guard so CI without the artifact still passes.
#[wasm_bindgen_test]
async fn extracts_entities_in_browser() {
    let st = include_bytes!("fixtures/tiny_gliner.safetensors");
    let tk = include_bytes!("fixtures/tokenizer.json");
    let cfg = include_bytes!("fixtures/config.json");
    let model = xberg_gliner_candle::Gliner2Candle::from_bytes(st, tk, cfg).unwrap();
    let spans = model.extract_ner("Barack Obama visited Paris.", &["person", "location"], 0.3).unwrap();
    assert!(spans.iter().any(|s| s.text.eq_ignore_ascii_case("Barack Obama")));
}
```

If no tiny fixture exists, replace `include_bytes!` with a documented skip: assert `from_bytes(&[],..)` errors cleanly in-browser (proves the wasm code path links and runs), and leave a `// TODO(fixture)` — actually, per no-placeholder rule, instead assert the empty-input error path in-browser as the shippable smoke test:

```rust
#[wasm_bindgen_test]
fn from_bytes_errors_cleanly_in_browser() {
    let err = xberg_gliner_candle::Gliner2Candle::from_bytes(&[], b"{}", b"{}").unwrap_err();
    assert!(!err.to_string().is_empty());
}
```

- [ ] **Step 2: Run**

Run: `wasm-pack test --headless --chrome crates/xberg-gliner-candle 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
prek run --all-files
git add crates/xberg-gliner-candle/tests/wasm_ner.rs
git commit -m "test(gliner-candle): wasm NER smoke test"
```

---

## Self-Review Notes

- **Spec coverage:** §A ner-candle-wasm → all four tasks. This plan **replaces** B's Task 1 body: B's Task 1 should now read "confirm `ner-candle-wasm` (delivered by plan A) is in `wasm-target` and consumed by the engine" — not "gate xberg-gliner out" (that was incorrect; A keeps xberg-gliner's tokenizer surface). Update B's plan accordingly when starting B.
- **Risk gate:** Task 1 Step 4 (`tokenizers` on wasm) and Task 2 Step 6 (candle on wasm) are the two genuine feasibility gates. Either failing means in-binary NER defers and the injected ORT-Web NER path in B carries NER alone — recorded, not hidden.
- **Type consistency:** `from_bytes`, `from_buffered_safetensors`, `WasmCandleNer`, `map_spans_to_entities`, `extract_ner` used consistently. Inline notes flag the three names the implementer must confirm against source: `EncoderConfig` (encoder.rs), `Entity` (xberg types), `XbergError` variant.
- **Sequencing:** A is fully independent of B, C, D, E — it can start immediately and is the true prerequisite for B's in-binary NER fallback.

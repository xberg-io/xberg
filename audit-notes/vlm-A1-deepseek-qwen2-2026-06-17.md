# VLM-OCR Qwen2 Audit A1: candle-transformers vs. aha Vendor

**Task**: Determine whether DeepSeek-OCR's Qwen2 decoder can use `candle_transformers::models::qwen2::Model` (Option A) or requires vendoring aha's custom Qwen2 (Option B).

**Verdict**: **Option B (Vendor aha's Qwen2)**. API surfaces are incompatible; candle-transformers' `Qwen2Model` cannot substitute without extensive adapters.

---

## 1. Aha's Qwen2 Inventory

### Public Types

| Type | Location | Signature |
|------|----------|-----------|
| `Qwen2Config` | mod.rs:13-28 | `pub struct Qwen2Config { vocab_size, hidden_size, intermediate_size, num_hidden_layers, num_attention_heads, num_key_value_heads, max_position_embeddings, sliding_window, max_window_layers, tie_word_embeddings, rope_theta: f32, rms_norm_eps, use_sliding_window, hidden_act: Activation }` |
| `Qwen2Decoder` | mod.rs:234-276 | `pub struct { layers: Vec<Qwen2DecoderLayer>, norm: RmsNorm, rotary_emb: RoPE }` |

### Public Methods

| Item | Signature | Notes |
|------|-----------|-------|
| `Qwen2Decoder::new()` | `pub fn new(vb: VarBuilder, cfg: &Qwen2Config) -> Result<Self>` | Takes config by reference |
| `Qwen2Decoder::forward_no_cache()` | `pub fn forward_no_cache(&self, xs: &Tensor, attention_mask: Option<&Tensor>, seqlen_offset: usize) -> Result<Tensor>` | **No `seqlen_offset` in standard forward** |
| `Qwen2Decoder::clear_kv_cache()` | `pub fn clear_kv_cache(&mut self)` | Mutable |

### Private Internals (Not Exported)

- `Qwen2Attention` (has `kv_cache: Option<(Tensor, Tensor)>`)
- `Qwen2DecoderLayer` (has `forward()` and `forward_no_cache()`)

---

## 2. DeepSeek-OCR's Actual Qwen2 Surface Use

**Location**: `/tmp/aha/src/models/deepseek_ocr/model.rs`

### Direct Usage

| Line | Code | API Requirement |
|------|------|-----------------|
| 23 | `qwen2::{Qwen2Config, Qwen2Decoder}` | Import both types |
| 1096 | `Qwen2Config { vocab_size: 151936, hidden_size: 896, ... }` | Construct config directly with all fields |
| 1112 | `Qwen2Decoder::new(vb.pp("model.model"), &qwen2_config)?` | Call `::new(VarBuilder, &Config)` |
| 1158 | `self.model.forward_no_cache(&x_combined, Some(&attn_mask), 0)?` | Call `forward_no_cache(xs: &Tensor, attention_mask: Option<&Tensor>, seqlen_offset: usize)` |

**Minimum API Surface Required**:

1. `Qwen2Config` struct with all 14 fields constructible
2. `Qwen2Decoder::new(VarBuilder, &Qwen2Config) -> Result`
3. `Qwen2Decoder::forward_no_cache(&self, &Tensor, Option<&Tensor>, usize) -> Result<Tensor>`

---

## 3. candle-transformers 0.10.2 Qwen2 Inventory

**Location**: `~/.cargo/registry/src/index.crates.io-.../candle-transformers-0.10.2/src/models/qwen2.rs`

### Public Types

| Type | Signature |
|------|-----------|
| `Config` | `pub struct Config { vocab_size, hidden_size, intermediate_size, num_hidden_layers, num_attention_heads, num_key_value_heads, max_position_embeddings, sliding_window, max_window_layers, tie_word_embeddings, rope_theta: f64, rms_norm_eps, use_sliding_window, hidden_act }` |
| `Model` | `pub struct Model { embed_tokens, layers: Vec<DecoderLayer>, norm, sliding_window, device: Device, dtype: DType }` |
| `ModelForCausalLM` | `pub struct { base_model: Model, lm_head: Linear }` |

### Public Methods

| Item | Signature | Notes |
|------|-----------|-------|
| `Model::new()` | `pub fn new(cfg: &Config, vb: VarBuilder) -> Result<Self>` | Takes config by reference |
| `Model::forward()` | `pub fn forward(&mut self, input_ids: &Tensor, seqlen_offset: usize, attn_mask: Option<&Tensor>) -> Result<Tensor>` | **Generic forward, not no_cache variant** |
| `Model::clear_kv_cache()` | `pub fn clear_kv_cache(&mut self)` | Mutable |
| `ModelForCausalLM::forward()` | `pub fn forward(&mut self, input_ids: &Tensor, seqlen_offset: usize) -> Result<Tensor>` | **Returns logits only** |

---

## 4. API Delta Analysis

### Critical Mismatches

| Aspect | Aha | candle-transformers | Impact |
|--------|-----|-------------------|--------|
| **rope_theta field type** | `f32` (line 24) | `f64` (line 34) | Type mismatch; requires adapter |
| **Forward signature** | `forward_no_cache(&self, xs, mask, seqlen_offset)` | `forward(&mut self, input_ids, seqlen_offset, mask)` | Different param order + mutability; requires wrapper |
| **Input semantics** | `xs: &Tensor` (embeddings) | `input_ids: &Tensor` (token IDs) | aha's Qwen2Decoder2Encoder does embedding lookup *before* passing to decoder; candle-transformers' Model expects raw token IDs |
| **KV Cache ownership** | Internal to each layer (line 41) | Internal to Model + per-layer | Identical semantics, but aha allows `forward_no_cache` bypassing cache |
| **Output shape** | Full sequence `(batch, seq_len, hidden)` | Full sequence *or* last token only in ModelForCausalLM | Aha's decoder returns all tokens; needed for DeepSeek-OCR's token slicing on line 1159 |
| **Method availability** | `forward_no_cache()` (no cache branch) | Only `forward()` (cache always active) | Aha gives caller control over caching; candle-transformers does not |

### Compatibility Summary

**candle-transformers Model **cannot directly substitute** aha's Qwen2Decoder because**:

1. **Type mismatch**: `rope_theta: f64` vs `f32` (type error at construction)
2. **Signature mismatch**: Input semantics differ (token IDs vs. embeddings); parameter order reversed
3. **Cache control mismatch**: No `forward_no_cache()` alternative in candle-transformers; KV cache is always active and cannot be bypassed
4. **Mutability difference**: candle-transformers requires `&mut self`, aha only requires `&self` for `forward_no_cache()`

An adapter would need to:

- Wrap candle-transformers' `Model` to expose `forward_no_cache()` that clones and strips KV cache
- Convert `rope_theta: f32` to `f64` at construction
- Pre-embed input tokens before passing to candle-transformers (move responsibility to caller)

**Adapter complexity**: ~50 lines of shim code, but adds runtime overhead (cache cloning on every no_cache call).

---

## 5. Verdict

**Option B: Vendor aha's Qwen2 under `crates/kreuzberg-candle-ocr/src/vendor/aha/qwen2/`**

**Justification**:

1. **API incompatibility is structural**: candle-transformers treats KV cache as internal state (always active); aha exposes `forward_no_cache()` for caller control. DeepSeek-OCR uses only `forward_no_cache()`, never touching cache management. Bridging this requires a runtime-inefficient wrapper (cache cloning per call).

2. **Input semantics diverge**: aha's `Qwen2Decoder::forward_no_cache()` expects *pre-embedded* hidden states; candle-transformers' `Model::forward()` expects raw token IDs. DeepSeek-OCR's `Qwen2Decoder2Encoder` does embedding lookup internally (line 1127–1134), so it needs aha's embedded-input API.

3. **Type safety**: `rope_theta: f32` vs `f64` forces conversion at every Qwen2 instantiation; aha's approach keeps consistency with its broader codebase (all configs use f32 for rope_theta).

4. **Minimal maintenance burden**: aha's Qwen2 is ~276 lines of focused decoder logic with no external dependencies beyond common modules (GateUpDownMLP, RoPE). Vendoring keeps DeepSeek-OCR's setup self-contained and avoids multi-layer abstraction.

5. **Copy-paste safety**: The code is already proven in aha; vendoring it is mechanical and low-risk.

**Recommendation**: Vendor aha's entire `src/models/qwen2/mod.rs` (276 lines) into kreuzberg-candle-ocr. No adapter needed.

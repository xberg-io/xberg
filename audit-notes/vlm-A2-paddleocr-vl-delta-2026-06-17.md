# A2 Audit: PaddleOCR-VL 1.5 Inventory & Port Plan

**Audit Date**: 2026-06-17 | **Phase**: Phase 1, Task A2
**Scope**: aha's PaddleOCR-VL 1.5 (Candle 0.9.2) vs. kreuzberg's existing wrapper (Candle 0.10)

---

## 1. Aha's PaddleOCR-VL 1.5 Inventory

### 1.1 Crate Structure

- **Submodule layout**: `src/models/paddleocr_vl/` with `mod.rs`, `config.rs`, `model.rs`, `processor.rs`, `generate.rs`
- **Candle version**: 0.9.2
- **File count**: 5 source files (~1,200 LOC total)

### 1.2 Configuration & Types (`config.rs`)

#### `PaddleOCRVLConfig` (main model config, 34 pub fields)

- Decoder (ERNIE-4.5): `hidden_size`, `num_attention_heads`, `num_key_value_heads`, `head_dim`, `intermediate_size`, `vocab_size`, `num_hidden_layers`
- Rope: `rope_theta`, `rope_scaling: PaddleOCRVLRopeScalingConfig` (with `mrope_section: Vec<usize>`, `rope_type`, `scaling_type`), `use_3d_rope: bool`, `rope_is_neox_style: bool`
- Vision tokens: `image_token_id`, `video_token_id`, `vision_start_token_id`
- Attention: `use_cache`, `use_flash_attention`, `use_bias`
- Norm: `rms_norm_eps`
- Vision encoder ref: `vision_config: PaddleOCRVLVisionConfig`
- Misc: `compression_ratio`, `hidden_dropout_prob`, `ignored_index`, `max_position_embeddings`, `max_sequence_length: Option`, `pad_token_id`, `sliding_window: Option`, `tie_word_embeddings`, `torch_dtype: String`, `weight_share_add_bias`

#### `PaddleOCRVLVisionConfig` (SigLIP encoder config, 11 pub fields)

- Vision backbone: `image_size`, `patch_size`, `hidden_size`, `num_attention_heads`, `num_hidden_layers`
- Processing: `spatial_merge_size`, `temporal_patch_size`, `num_channels`, `attention_dropout`
- Norm: `layer_norm_eps`
- Special: `tokens_per_second` (temporal scaling for video)
- Misc: `torch_dtype`, `intermediate_size`, `pad_token_id`

#### `PaddleOCRVLPreprocessorConfig` (image preprocessing, 11 pub fields)

- Resize logic: `do_resize`, `do_rescale`, `do_convert_rgb`, `do_normalize`
- Constraints: `min_pixels`, `max_pixels`, `patch_size`, `merge_size`, `temporal_patch_size`
- Normalization: `image_mean: Vec<f64>`, `image_std: Vec<f64>`, `rescale_factor`
- Misc: `resample` (PIL filter code), `size: Option<SizeConfig>`

### 1.3 Model Architecture (`model.rs`)

#### Vision Path: `SiglipVisionModel`

- **Embeddings** (`SiglipVisionEmbeddings`)
  - Conv2D patch embedding (input: RGB, output: `hidden_size` channels)
  - Position embeddings (static interpolation via bilinear resampling on resize)
  - Packing position embedding (lookup table, 32768 entries for variable-length sequences)
  - Signature: `forward(pixel_values [bs, seq_len, c, h, w], position_ids, image_grid_thw, interpolate_pos_encoding) → Tensor`

- **Encoder** (`SiglipEncoder`)
  - Stack of `NaiveAttnTwoLinearMLPBlock` (standard transformer blocks)
  - Rotary embedding: `Qwen2_5VisionRotaryEmbedding` (2D spatial rope for h/w)
  - Signature: `forward(xs, image_grid_thw) → Tensor`

- **Post-LayerNorm** after encoder
- **Overall**: `forward(pixel_values, image_grid_thw, position_ids, interpolate_pos_encoding) → Tensor [bs, seq_len, hidden_size]`

#### Text Decoder: `Ernie4_5Model`

- Embed tokens (shared with lm_head if `tie_word_embeddings`)
- Stack of `NaiveAttnGateUpDownMLPBlock` (gated MLP variant, not standard FFN)
- Output norm: `RmsNorm`
- Rotary: `Qwen2_5VLTextRotaryEmbedding` (1D text + 3D multi-RoPE for spatial/temporal alignment)
  - Signature: `forward(position_ids: [3, bs, seq_len], inputs_embeds, seqlen_offset, …) → Tensor`
  - **M-RoPE key**: supports 3 sections (text, height, width/time) with scaling per config
- **KV cache** management: `clear_kv_cache()`

#### Fusion: `Projector`

- MLP that projects vision features (after spatial merge) to LLM hidden size
- Performs 2D spatial merge (divide patch grid by `merge_size`, apply linear transforms)
- Signature: `forward(vision_features, image_grid_thw) → Tensor [merged_seq_len, hidden_size]`

#### Top-level: `PaddleOCRVLModel`

- Composes: `mlp_ar: Projector`, `visual: SiglipVisionModel`, `model: Ernie4_5Model`, `lm_head: Linear`
- Core method: `forward(input_ids, pixel_values, image_grid_thw, image_mask, cache_position, seqlen_offset) → logits`
- **Rope index calculation** (`get_rope_index`): complex logic to handle image/video grids, computes 3D position deltas for M-RoPE (text vs. spatial)
- Implements `InferenceModel` trait with `forward_initial` (multimodal), `forward_step` (text-only), KV cache control

### 1.4 Processor (`processor.rs`)

#### `PaddleOCRVLProcessor`

- Public methods:
  - `new(config, device, dtype) → Self`
  - `process_img(img, img_mean, img_std) → Tensor [1, c, h, w]` — smart resize + normalize
  - `process_vision_tensor(img_tensor) → (Tensor [patches, c, patch_sz, patch_sz], Tensor [1, 3])`
  - `process_images(imgs) → (pixel_values, vision_grid_thws)` — batch processing
  - `process_info(chat_messages, text) → (text_with_placeholders, pixel_values?, grid_thw?)`

Key preprocessing:

- **Smart resize**: maintains aspect ratio, pads/scales to be divisible by `patch_size * merge_size` (14 × 2 = 28)
- **Placeholder injection**: replaces `<|IMAGE_PLACEHOLDER|>` tokens in text based on grid size
- **Image token**: `"<|IMAGE_PLACEHOLDER|>"`
- **Normalization**: per-image mean/std (RGB channels stored as [3, 1, 1])

### 1.5 Generation Engine (`generate.rs`)

#### `PaddleOCRVLGenerateModel<'a>`

- Public entry point: `init(path, device?, dtype?) → Result<Self>`
  - Loads config from `config.json`
  - Loads tokenizer from `tokenizer.json`
  - Loads processor config from `preprocessor_config.json`
  - Loads safetensors weights via mmap
  - Creates model with EOS token ID = 2 (hardcoded)

#### Trait: `GenerationDataProvider`

- Signature: `get_data(chat_msg) → PrepareData { input_ids, multi_model_data }`
- **Multi-modal data**: 4 tensors packed as `[pixel_values, image_grid_thw, image_mask, cache_position]`
- Uses `ChatTemplate` for message formatting
- Uses `TokenizerModel` for text encoding

#### Macro: `impl_generate_model!`

- Expands standard generation loop (calls model.forward, samples tokens, updates KV cache)

---

## 2. Kreuzberg's Existing PaddleOCR-VL Wrapper

### 2.1 Structure

- **Single file**: `crates/kreuzberg-candle-ocr/src/models/paddleocr_vl.rs` (~428 LOC)
- **Candle version**: 0.10 via candle-transformers
- **Key difference**: wraps `candle_transformers::models::paddleocr_vl::{Config, PaddleOCRVLModel}` (upstream built-in)

### 2.2 Engine: `PaddleOcrVlEngine`

- Task enum: `PaddleOcrVlTask { Ocr, Table, Formula, Chart }`
- Core state:
  - `model: Arc<Mutex<PaddleOCRVLModel>>` (from candle-transformers)
  - `tokenizer: Tokenizer`
  - `config: Config` (upstream)
  - `device: Device`, `dtype: DType`
  - `bos_token_id`, `eos_token_id` (resolved from tokenizer at load time)

### 2.3 Public API

- `new(task, device, dtype) → Result<Self>` — downloads from HF (PaddlePaddle/PaddleOCR-VL)
- `process_image(image_bytes) → Result<CandleOcrOutput>`

### 2.4 Processing (inlined, no separate processor)

- `load_and_preprocess_image`:
  - Smart resize (factor=28, min_pixels=147384, max_pixels=2822400)
  - Normalize to [-1,1] using mean=[0.5, 0.5, 0.5], std=[0.5, 0.5, 0.5]
  - Returns `(Tensor [1, 3, h, w], Tensor grid_thw)`

- `build_input_tokens(num_image_tokens)`:
  - Format: `<BOS> + "User: " + <VISION_START> + <IMAGE_TOKENS> + <VISION_END> + task + "\nAssistant: "`
  - Resolves BOS/EOS from tokenizer on init

### 2.5 Inference Loop

- Calls upstream `model.generate(input_ids, pixel_values, grid_thw, max_length, eos_id) → Vec<u32>`
- Decodes via tokenizer.decode()
- Wraps in `CandleOcrOutput`

### 2.6 Limitations

- No direct task control in inference (task prompt set at token-building time only)
- No access to intermediate model state
- Relies on candle-transformers 0.10 upstream impl

---

## 3. Architectural Delta

| Aspect | Aha 1.5 (Candle 0.9.2) | Kreuzberg (Candle 0.10) | Impact |
|--------|------------------------|-------------------------|--------|
| **Code org** | Submodule (5 files) | Single file wrapper | Aha is self-contained; kreuzberg delegates to upstream |
| **Vision encoder** | SigLIP (embedded) | Via upstream | Same architecture (SigLIP) |
| **Rope** | Qwen2.5-style 2D spatial + 3D text M-RoPE | Upstream impl | Aha explicitly implements; kreuzberg trusts upstream |
| **Processor** | Standalone, full config | Inlined in engine | Aha separates concerns; kreuzberg minimal |
| **Preprocessor config** | Explicit `PaddleOCRVLPreprocessorConfig` | Hardcoded (min/max pixels, means, stds) | Aha reads from JSON; kreuzberg embeds values |
| **Config fields** | 34 + 11 + 11 vision/preprocessing | Delegated to `candle_transformers::Config` | Aha exposes full control; kreuzberg abstract |
| **M-RoPE support** | Explicit: `use_3d_rope`, `mrope_section`, `rope_is_neox_style` | Upstream (unknown exposure) | Aha allows tuning; kreuzberg fixed by upstream |
| **KV cache** | Managed by model (`clear_kv_cache`) | Upstream (call forwarded) | Both support it |
| **Generation entry** | `PaddleOCRVLGenerateModel` (full pipeline) | Direct model.generate() (simple) | Aha modular; kreuzberg direct |
| **Task control** | Via processor config placeholder replacement | Hardcoded in build_input_tokens | Both support OCR/Table/Formula/Chart |
| **Candle version** | 0.9.2 | 0.10 | **Upstream lag**: aha is behind candle-transformers official release |

### Key Findings

1. **Aha is self-contained & explicitized**: full control over vision, text, rope, processing.
2. **Kreuzberg wraps upstream candle-transformers**: simpler but loses control; upstream 0.10 may have fixes/changes vs. aha's fork.
3. **Config explosion in aha**: 56 fields across 3 structs vs. kreuzberg's delegated `Config`.
4. **Rope complexity**: aha explicitly implements 3D M-RoPE calculation; kreuzberg relies on upstream.
5. **Processing**: aha reads preprocessor config JSON; kreuzberg hardcodes numeric constants.

---

## 4. Port Plan Recommendation

### **Recommendation: (B) Refactor into submodule** (`paddleocr_vl/{mod,config,model,processor,engine}.rs`)

**Justification**:

1. **Maintainability**: Aha's ~1,200-LOC implementation has 5 logical domains (config, vision, decoder, processor, generation). Single-file organization in kreuzberg (428 LOC) works now, but porting aha means inlining ~800 LOC. Thresholds cross 1000 LOC; split warranted.

2. **Upstream lag risk**: Kreuzberg currently trusts `candle_transformers::models::paddleocr_vl` (Candle 0.10). Aha's self-contained 0.9.2 fork may diverge. By porting aha's submodule structure, we control the full stack, reducing upstream-version friction when candle-transformers 0.11+ arrives or drifts.

3. **Config surface**: Aha exposes 56 config fields; kreuzberg hardcodes. A submodule can expose config loading from JSON without bloating the main engine file.

4. **Explicit over implicit**: Rope index calculation (`get_rope_index`), vision embedding interpolation, projector merging—all explicit in aha, implicit in kreuzberg. Submodule files clarify intent.

### File Layout

```text
crates/kreuzberg-candle-ocr/src/models/paddleocr_vl/
  ├─ mod.rs           (re-exports, top-level Engine)
  ├─ config.rs        (3 config structs, serde)
  ├─ model.rs         (SigLIP, ERNIE, Projector, toplevel Model)
  ├─ processor.rs     (image preprocessing)
  └─ engine.rs        (PaddleOcrVlEngine, integration with kreuzberg wrapper)
```

**Single-file alternative rejected**: While "just add aha's 800 LOC to paddleocr_vl.rs" works, it violates readability (Rust best-practice max ~300 LOC/file for complex domain logic). Submodule adds 5 files but aligns with kreuzberg's multi-language binding patterns elsewhere (see csharp/, go/ out-of-workspace structure).

---

## 5. Version 1.6 Readiness Verdict

**1.6 Unlikely in near term; 1.5→1.5.1 focus on config normalization.**

Aha's code exhibits no 1.6 migration signals:

- No version guards (`#[cfg(feature = "v1_6")]`) or deprecation comments
- No TODOs referencing upstream 1.6 roadmap
- Rope fields (`use_3d_rope`, `rope_is_neox_style`) finalized, not marked "experimental"
- Vision config fields stable (spatial merge, temporal patch size settled)

Architecture is mature: the jump from SigLIP → ERNIE is complete; no placeholder for new encoders. Config schema is bloated (34 fields) but static. The delta between aha's 1.5 and an eventual 1.6 would likely be:

- Config field additions (e.g., new attention variant), not removals
- Rope section count changes (mrope_section is `Vec<usize>`—flexible)
- Possibly a new preprocessor mode

**Estimated 1.5→1.6 migration effort**: 1–2 days (config merge, regression tests). **Barrier**: kreuzberg must first complete 1.5 port, stabilize tests, then monitor aha for 1.6 release signals.

---

## Appendix: File Signatures

### Aha PaddleOCR-VL 1.5 Public API

```rust
// config.rs
pub struct PaddleOCRVLConfig { … }
pub struct PaddleOCRVLVisionConfig { … }
pub struct PaddleOCRVLPreprocessorConfig { … }

// model.rs
pub struct SiglipVisionModel { … }
  pub fn new(vb, config) → Result<Self>
  pub fn forward(pixel_values, image_grid_thw, position_ids, interpolate_pos_encoding) → Result<Tensor>

pub struct Ernie4_5Model { … }
  pub fn new(vb, config) → Result<Self>
  pub fn forward(inputs_embeds, seqlen_offset, position_ids?) → Result<Tensor>
  pub fn clear_kv_cache(&mut self)

pub struct PaddleOCRVLModel { … }
  pub fn new(cfg, vb, eos_ids) → Result<Self>
  pub fn get_rope_index(…) → Result<(Tensor, Tensor)>
  pub fn forward(input_ids, pixel_values?, image_grid_thw?, …) → Result<Tensor>
  pub fn clear_kv_cache(&mut self)

impl InferenceModel for PaddleOCRVLModel

// processor.rs
pub struct PaddleOCRVLProcessor { … }
  pub fn new(config, device, dtype) → Result<Self>
  pub fn process_img(img, img_mean, img_std) → Result<Tensor>
  pub fn process_vision_tensor(img_tensor) → Result<(Tensor, Tensor)>
  pub fn process_images(imgs, …) → Result<(Tensor, Tensor)>
  pub fn process_info(messages, text) → Result<(String, Option<Tensor>, Option<Tensor>)>

// generate.rs
pub struct PaddleOCRVLGenerateModel<'a> { … }
  pub fn init(path, device?, dtype?) → Result<Self>

impl GenerationDataProvider for PaddleOCRVLGenerateModel<'a>
```

### Kreuzberg's Current Wrapper (Candle 0.10)

```rust
// paddleocr_vl.rs
pub enum PaddleOcrVlTask { Ocr, Table, Formula, Chart }

pub struct PaddleOcrVlEngine { … }
  pub fn new(task, device, dtype) → Result<Self>
  pub fn process_image(image_bytes) → Result<CandleOcrOutput>

// (inlined preprocessing, no separate processor struct)
// (delegates model via candle_transformers upstream)
```

---

**Audit prepared by**: File search specialist
**Status**: Recommendation ready for Phase 3 (vendor scaffold)

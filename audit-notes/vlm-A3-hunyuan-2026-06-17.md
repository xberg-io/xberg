# Audit A3: Hunyuan-OCR End-to-End Port Feasibility

## Source: jhqxxx/aha Candle 0.9.2

**Date**: 2026-06-17
**Scope**: `src/models/hunyuan_ocr/` (config.rs, model.rs, processor.rs, generate.rs, mod.rs)
**Total LOC**: 1076 (clean, well-organized)
**Port Risk Level**: **LOW** (isolated, minimal external deps, explicit control flow)

---

## 1. File Inventory

| File | LOC | Public Items | unwrap() | assert! | panic! | todo! | unreachable! |
|------|-----|--------------|----------|---------|--------|-------|--------------|
| config.rs | 105 | 4 structs (HunYuanVLConfig, HunYuanVLRopeScaling, HunYuanVLVisionConfig, HunyuanOCRGenerationConfig, HunyuanOCRPreprocessorConfig) | 0 | 0 | 0 | 0 | 0 |
| model.rs | 630 | 7 public structs + InferenceModel impl | 0 | 0 | 0 | 0 | 0 |
| processor.rs | 236 | HunyuanVLProcessor + HunyuanData | 2 unwrap_or | 2 assert! | 0 | 0 | 0 |
| generate.rs | 101 | HunyuanOCRGenerateModel + GenerationDataProvider impl | 3 unwrap_or | 0 | 0 | 0 | 0 |
| mod.rs | 4 | Module re-exports | 0 | 0 | 0 | 0 | 0 |

### Error-handling assessment

- **config.rs**: Pure data structures, no error paths.
- **model.rs**: All ops return `Result<T>`, no defensive `.unwrap()` — errors propagated.
- **processor.rs**: 2 `assert!()` calls for path/file validation (L27, L32) — replaceable with `Result<Err>`.
- **generate.rs**: 3 `.unwrap_or()` calls (L53, L69, L73) — all with sensible defaults, safe.

---

## 2. External Symbol Inventory

### From `crate::models::common`

| Symbol | Source Module | Usage in Hunyuan |
|--------|---------------|------------------|
| `InferenceModel` | common/mod | Implemented by HunyuanVLModel (trait bridge for generation loop) |
| `MultiModalData` | common/mod | Wrapper for pixel_values, grid_thw, mask, position_ids in generate.rs |
| `GateUpDownMLP` | common/modules | Vision MLP blocks in HunYuanVLDecoderLayer |
| `NaiveAttnTwoLinearMLPBlock` | common/modules | Vision transformer layers (HunYuanVisionTransformer) |
| `eager_attention_forward` | common/modules | Multi-head attention forward (HunYuanVLAttention) |
| `get_conv2d` | common/modules | Patch embedding & merger conv construction |
| `GenerationDataProvider` | common/generate | Interface impl in HunyuanOCRGenerateModel |
| `PrepareData` | common/generate | Return type for input prep (input_ids + MultiModalData) |

### From `crate::position_embed::rope`

| Symbol | Usage |
|--------|-------|
| `RoPE` | Initialized with head_dim and rope_theta; stores cos/sin LUT for rotary embeddings |
| `apply_rotary_pos_emb` | Applied to Q, K in HunYuanVLAttention |
| `get_xd_cos_sin` | XD-RoPE variant for position_ids layout support (line 524 model.rs) |

### From `crate::utils`

| Symbol | Source | Usage |
|--------|--------|-------|
| `interpolate_bilinear` | utils/interpolate | Patch position embedding interpolation (vision backbone) |
| `masked_scatter_dim0` | utils/tensor_utils | Inject image embeddings into text at image tokens |
| `prepare_causal_attention_mask` | utils/tensor_utils | Generate causal mask for decoder |
| `split_tensor` | utils/tensor_utils | Split vision output by image count |
| `get_eq_indices` | utils/tensor_utils | Find image token positions in input sequence |
| `get_equal_mask` | utils/tensor_utils | Boolean mask for image token locations |
| `find_type_files` | utils/mod | Model safetensors file discovery |
| `get_device` | utils/mod | Device selection (CPU/CUDA/MPS) |
| `get_dtype` | utils/mod | DType selection (bf16/f32) |
| `img_utils::{extract_images, img_smart_resize, img_transform}` | utils/img_utils | Image preprocessing pipeline |

### From `crate::tokenizer`

| Symbol | Usage |
|--------|-------|
| `TokenizerModel` | Loaded in generate.rs; used for text tokenization in processor |

### From External Crates

| Crate | Usage |
|-------|-------|
| candle_core | Tensor, Device, DType, indexing primitives |
| candle_nn | Linear, Conv2d, RmsNorm, Embedding, VarBuilder |
| anyhow | Error handling Result<T> |
| serde | Config deserialization (config.json, generation_config.json, preprocessor_config.json) |
| image | DynamicImage image processing |

**External Symbol Count**: ~20 vendorable aha symbols across 4 modules.

---

## 3. Architectural Notes

### Vision Backbone

- **Type**: Vision Transformer (ViT) with patch merging.
- **Components**:
  - `HunYuanVisionPatchEmbed`: Conv2d patch embedding + bilinear interpolated position embeddings.
  - `HunYuanVisionPatchMerger`: Spatial merge via Conv2d downsampling + projection; emits begin/end tokens.
  - `HunYuanVisionTransformer`: 24-layer naive attention + MLP transformer stack; outputs merged patch tokens.

### Decoder

- **Type**: Causal transformer (text-only after vision injection).
- `HunYuanVLTextModel`: Embeddings → 24 `HunYuanVLDecoderLayer` → final norm.
- `HunYuanVLDecoderLayer`: Pre-norm residuals (input_layernorm → attention → residual) + (post_attention_layernorm → GateUpDownMLP → residual).

### Position Embeddings

- **Scheme**: XD-RoPE with section-wise frequency scaling.
- **Implementation**: `RoPE::new()` initialized once with base = `rope_theta * alpha^(head_dim / (head_dim-2))`.
- **Multi-dimensional**: Supports 2D positional coordinates (h, w, t) via `get_xd_cos_sin()` + xdrope_section mapping.
- **Used**: Applied to Q, K in attention layer 0 with image grid coordinates; standard RoPE for subsequent layers.

### KV Cache Management

- **Owner**: Each `HunYuanVLAttention` instance.
- **Lifecycle**: Initialized as `None`; accumulated in `forward()` → concatenated with fresh K, V each step.
- **Clear**: Explicit `clear_kv_cache()` after generation step.
- **Caller contract**: `HunyuanVLModel.forward_step()` expects single-token input; seqlen_offset tracks position.

### Generation Loop Integration

- **Interface**: `GenerationDataProvider` trait (`get_data()`, `get_temperature()`, `get_top_p()`, `get_top_k()`).
- **Macro**: `impl_generate_model!(HunyuanOCRGenerateModel)` — aha's generation loop macro expands into step/sample loop.
- **Data flow**: ChatCompletionParameters → processor → (input_ids, MultiModalData with 4-element vec) → common::generate loop.
- **Multimodal contract**: 4-tuple (pixel_values, grid_thw, image_mask, position_ids) packed into `MultiModalData::new(vec![...])`.

---

## 4. Generation-Loop Compatibility Assessment

### Verdict: **PLUG-IN COMPATIBLE WITH ONE REFACTOR**

**What works out-of-box**:

- Hunyuan implements `InferenceModel` trait (`forward_initial`, `forward_step`, `clear_cache`, `stop_token_ids`).
- `HunYuanVLAttention.kv_cache` is self-managed; no caller-provided cache required.
- RoPE is pre-computed per step; no position-id provider needed.

**Architectural issue**: Multi-dimensional position IDs.

- Hunyuan uses 4D position tensor: `[batch, 4, seq_len]` where dims are (1D baseline, h, w, t).
- Current aha `common::generate` loop calls `model.forward_step(input_ids: &Tensor, seqlen_offset: usize)`.
- aha's seqlen_offset is a scalar; Hunyuan needs the 4D grid coordinate tuple.

**Required refactor** (estimated 4–6 hours):

1. Extend `InferenceModel::forward_step()` signature to accept optional per-model metadata, OR
2. Add a new trait method `fn forward_step_with_position_ids(&mut self, input_ids: &Tensor, position_ids: Option<&Tensor>, seqlen_offset: usize) -> Result<Tensor>` with a default impl that calls `forward_step()`, OR
3. Wrap Hunyuan in a thin adapter that caches position_ids state and calls `forward()` directly with all arguments.

**Recommended approach**: Option 2 — minimal, non-invasive, backward-compatible. Other models (e.g., Qwen) may also need per-token position metadata.

---

## 5. Port Effort Estimate

### Analogues

- **PaddleOCR-VL port (aha → kreuzberg-ocr)**: ~12 hours (included vendoring infra, LoRA adapter cleanup, checkpoint format conversion).
- **Hunyuan complexity delta**:
  - **Same**: ViT + causal transformer + RoPE, multimodal processor.
  - **Simpler**: No LoRA, no rope_scaling.alpha powf logic needed (pre-computed base); simpler grid thw handling.
  - **Harder**: XD-RoPE with section mapping; 4D position tensor for coordinate interpolation.

### Breakdown (hours)

| Task | Est. Hours | Notes |
|------|-----------|-------|
| Vendor shared infra (Phase 3) | 3–5 | rope.rs, tensor_utils.rs, img_utils.rs (~2.5 kloc total); filter out unrequired symbols |
| Port model.rs + config.rs | 2–3 | Straightforward; swap `candle_nn` → kreuzberg's abstractions |
| Port processor.rs | 2–3 | Image resize + grid indexing; adapt img_transform to kreuzberg's pipeline |
| Port generate.rs + adapter | 3–4 | Implement InferenceModel; add position_ids path; wire into generate_mrope |
| Fix assert! → Result | 0.5 | Minimal |
| E2E testing + benchmark | 2–3 | Fixture generation, torch vs. Hunyuan output matching |
| **TOTAL** | **12–18** | (vs. 12 for PaddleOCR-VL) |

**Confidence**: **HIGH** — code is clean, no panics, explicit error propagation. The main risk is the position_ids refactor, which is scoped and non-invasive.

---

## 6. Key Dependencies Summary

### Vendorable Subset (Phase 3 output)

```text
├── position_embed/rope.rs         (RoPE, apply_rotary_pos_emb, get_xd_cos_sin)
├── utils/
│   ├── tensor_utils.rs            (masked_scatter_dim0, prepare_causal_attention_mask, split_tensor, get_eq_indices, get_equal_mask, index_select_2d)
│   ├── interpolate.rs             (interpolate_bilinear)
│   └── img_utils.rs               (extract_images, img_smart_resize, img_transform)
├── models/common/
│   ├── modules.rs                 (GateUpDownMLP, NaiveAttnTwoLinearMLPBlock, eager_attention_forward, get_conv2d)
│   └── generate.rs + types        (InferenceModel, MultiModalData, GenerationDataProvider, PrepareData, GenerationContext)
└── (candle_core, candle_nn from upstream)
```

### Not Needed for Hunyuan

- models/common/embedding.rs, reranker.rs, gguf.rs
- position_embed/sinusoidal_pe.rs
- utils/video_utils.rs, response_utils.rs
- models/common/model_mapping.rs

---

## Conclusion

**Hunyuan-OCR is the best port candidate.** At 1076 LOC, zero panics, clean trait integration, and only ~20 external symbol dependencies, it is lower risk than PaddleOCR. The only substantive task is adding position_ids support to `InferenceModel`, which is a 4–6 hour refactor applicable to future VLMs. The 12–18 hour end-to-end port aligns well with the PaddleOCR baseline and is justified by the architectural soundness of the code.

**Recommendation for Phase 4**: Proceed with Hunyuan as first model port. Use the position_ids refactor as a template for subsequent VLM integrations.

# GLM-OCR Phase 4 + 5 — backend wiring, paired pipeline, tests

_Recorded 2026-06-16_

## Deliverables shipped

### Phase 4 (commit 7be7b6eb4b)

- New `crates/kreuzberg/src/candle_ocr/glm_ocr_backend.rs` (~574 lines) implementing `OcrBackend` + `Plugin` for GLM-OCR, with `(DevicePreference, DType)`-keyed engine pool and `LayoutMode::{WholePage, Paired}` dispatch.
- `candle-glm-ocr` Cargo aggregate (`crates/kreuzberg/Cargo.toml`) pulling in `kreuzberg-candle-ocr/glm-ocr` + `layout-detection`.
- Backend registered at `crates/kreuzberg/src/plugins/registry/ocr.rs` behind `#[cfg(feature = "candle-glm-ocr")]`.
- CLI exposure via `--ocr-backend candle-glm-ocr` (`crates/kreuzberg-cli/src/commands/overrides.rs`), `VALID_OCR_BACKENDS` consolidated into a const.
- Rider correctness fixes — TrOCR engine pool, MTP KV-cache reset, NaN-safe nucleus sampling, BF16-on-Metal guard, error source propagation, head_dim odd-ness errors instead of panics, structured-markdown heuristic.

### Phase 5 (this commit)

- `tools/benchmark-harness` pipeline variant `CandleGlmOcr` with `glm-ocr-bench` feature.
- Strengthened `glm_ocr_integration.rs` smoke with N-gram repeat detector.
- New `glm_ocr_paired_pipeline.rs` exercising `process_image_with_task` directly.
- New `glm_ocr_backend.rs` end-to-end backend test through `OcrBackend` trait.

## Wall-clock measurements

_To be recorded on first run. Pending: Metal smoke + CPU smoke timings._

## Known limitations / follow-ups

- Paired-pipeline test currently exercises `process_image_with_task` invocation rather than a true multi-region fixture; a synthetic text+table fixture should be added.
- Layout detector model loaded per call inside `process_paired`; pooling is a follow-up.
- PP-DocLayout-V3 maps `chart` class → `LayoutClass::Picture`, so `GlmOcrTask::Chart` is never dispatched in paired mode (caption used instead). Either remove the dead arm or wire layout-class disambiguation.
- No benchmark vs upstream Python reference scores yet — corpus run is a separate follow-up gated on having upstream scores in hand.

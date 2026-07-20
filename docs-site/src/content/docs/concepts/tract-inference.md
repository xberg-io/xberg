---
title: "Pure-Rust Inference (tract)"
---

Xberg runs its ML models — layout detection, table classification, document-orientation, OCR —
through [ONNX Runtime](https://onnxruntime.ai/) by default. ONNX Runtime is a native library and
cannot link on `wasm32` or the Android x86_64 emulator. On those targets Xberg runs the same models
through [`tract`](https://github.com/sonos/tract), Sonos' pure-Rust ONNX engine, behind a shared
inference seam. tract loads the identical `.onnx` artifacts (no weight conversion), is CPU-only, and
needs no C toolchain.

ONNX Runtime stays the default on every native build. tract is selected only where ORT cannot link,
and it trades CPU latency for portability — see [Latency](#latency).

## Model coverage

tract 0.23.4 does not execute every model Xberg ships. The seam routes each model to whichever engine
is active; models tract cannot run stay ONNX Runtime-only and are compiled out of the pure-Rust
feature sets (`layout-tract`, `auto-rotate-tract`).

| Model | Role | tract |
|---|---|---|
| RT-DETR | Layout detection | Runs |
| PP-LCNet | Table classifier, document-orientation, text-line orientation | Runs |
| DBNet / CRNN / AngleNet | PaddleOCR detection / recognition / angle | Runs — see [PaddleOCR](#paddleocr) |
| TATR | Table-structure recognition | ONNX Runtime only |
| PP-DocLayout-V3 | Layout detection | ONNX Runtime only |
| SLANeXt | Table-structure recognition | ONNX Runtime only |

The three ONNX Runtime-only models are blocked by concrete gaps in tract 0.23.4:

- **TATR** is a quantized export. Pinning the input clears the convolution's symbolic in-channel, but
  a fused scale constant carries a symbolic batch size the type analyser cannot unify with a concrete
  `1`.
- **PP-DocLayout-V3** clears its input facts, but tract's `LayerNormalization` translator then
  mis-infers the shape of the DETR decoder's norm layer — an op-translation bug, not a shape-pinning
  gap.
- **SLANeXt** uses the ONNX `Loop` operator, which tract does not implement.

Revisit each only if a non-quantized export or an upstream tract fix lands.

## Latency

Measured on Apple Silicon (aarch64), release build, single-thread CPU, best-of-8 warm inferences:

| Model | tract load | ORT load | tract run | ORT run | tract / ORT run |
|---|---|---|---|---|---|
| RT-DETR layout detector | 465 ms | 221 ms | 2637 ms | 137 ms | 19.3× |
| PP-LCNet table classifier | 22 ms | 9 ms | 31.9 ms | 2.2 ms | 14.4× |
| PP-LCNet document-orientation | 22 ms | 8 ms | 31.9 ms | 2.8 ms | 11.5× |

tract's pure-Rust CPU kernels run roughly 11–19× slower than ONNX Runtime. This is the accepted
trade-off: these models run about once per page, and on the targets tract exists for — WASM and the
Android x86_64 emulator, where ONNX Runtime cannot link at all — the alternative is no inference, not
ORT. Native builds keep ONNX Runtime, so the regression never reaches native users. RT-DETR's
~2.6 s per inference is the ceiling to watch for WASM UX; the CNN classifiers at ~32 ms are
comfortable.

Reproduce the table with:

```sh
cargo test --release -p xberg --no-default-features --features "layout-detection,auto-rotate,tract" \
  --lib inference::tract_backend::tests::tract_vs_ort_latency_report -- --ignored --nocapture
```

## Platform availability

| Target | Engine | Models |
|---|---|---|
| Native (desktop, server, Android arm64, iOS) | ONNX Runtime | Full set |
| Android x86_64 emulator (`android-target`) | tract | RT-DETR layout, table classifier, document-orientation |
| WASM (`wasm-target`) | — | tract inference not yet wired (planned) |

## PaddleOCR

DBNet and CRNN run on tract with excellent numeric parity to ONNX Runtime (max |Δ| below `3e-4`), but
a tract plan is optimized for one input shape and errors on any other. DBNet resizes each page to
content-dependent dimensions and CRNN batches by content-dependent width, so both see a new shape on
most calls — unlike AngleNet and the layout CNNs, which use a fixed resolution. Running PaddleOCR
detection and recognition on tract therefore needs a shape-keyed plan cache in the session (reload and
re-optimize only on a shape miss), which is a planned follow-up. AngleNet, being fixed-shape, already
runs on tract as-is.

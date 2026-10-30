---
status: accepted
date: 2026-07-19
decision-makers: ["Na'aman Hirschfeld"]
---

# Engine-neutral inference seam over ORT and tract

## Context and Problem Statement

xberg's ONNX models (layout detection, table classification, document orientation, OCR) called ONNX Runtime
directly, each consumer repeating the session-build recipe: graph optimization level, thread budget,
execution-provider selection and CPU-only fallback. ONNX Runtime is a native library that cannot link on `wasm32` or
the Android x86_64 emulator, so those targets had no inference at all. The work is tracked as issue #1275.

## Considered Options

* An engine-neutral seam (`InferenceBackend`, `InferenceSession`, `InferenceTensor`) with ONNX Runtime wherever it
  links and the pure-Rust `tract` engine on targets where it does not
* Each model consumer calls ONNX Runtime directly

## Decision Outcome

Chosen option: "An engine-neutral seam (`InferenceBackend`, `InferenceSession`, `InferenceTensor`) with ONNX Runtime
wherever it links and the pure-Rust `tract` engine on targets where it does not", because it folds the ORT session
recipe into one place (90dc63e65d) and lets the same `.onnx` artifacts run, without weight conversion, on targets
where ORT cannot link.

* 90dc63e65d (2026-07-19) added `crates/xberg/src/inference/` and moved the two PP-LCNet classifiers onto it with ORT
  as the only engine. 048a292f49 (2026-07-19) added the tract backend and cross-engine parity tests, and 08ce734680
  (2026-07-20) moved the layout detectors onto the seam.
* The engine is chosen at compile time: `build.rs` sets the `inference_ort` cfg when `ort-bundled` or `ort-dynamic` is
  active, and `default_backend()` returns ORT there and tract otherwise. ORT stays the native default even when the
  `tract` feature is on; tract then compiles only for the parity tests.
* Models tract 0.23.4 cannot execute (TATR, PP-DocLayout-V3, SLANeXt) stay ORT-only and are compiled out of the
  pure-Rust feature sets.
* The seam is `pub(crate)` and absent from `alef.toml` sources, so no binding exposes it.

### Consequences

* Good, because WASM, the Android x86_64 emulator and iOS run layout detection and orientation instead of no
  inference.
* Good, because native builds keep ONNX Runtime, so tract's latency never reaches native users.
* Bad, because tract runs these models roughly 11 to 19 times slower than ORT as shipped
  (`docs-site/src/content/docs/concepts/tract-inference.md`), and three models are unavailable on no-ORT targets.
* Bad, because two engines must agree numerically, which needs a parity suite with real model weights.

### Confirmation

* `crates/xberg/src/inference/mod.rs:1` documents the seam and its compile-time selection; `default_backend()` is at
  `:45` (ORT) and `:51` (tract). `crates/xberg/src/inference/backend.rs:54` defines `InferenceBackend`.
* The `ci-rust.yaml` step "Run tract/ORT parity tests" (`.github/workflows/ci-rust.yaml:951`) fails when the parity
  tests fail or match zero tests.
* `docs-site/src/content/docs/concepts/tract-inference.md` lists model coverage and platform availability per engine.

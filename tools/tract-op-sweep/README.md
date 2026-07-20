# tract-op-sweep

Phase 0 of the pure-Rust ONNX backend work ([#1275](https://github.com/xberg-io/xberg/issues/1275)).

Loads every ONNX model xberg ships through the [`tract`](https://github.com/sonos/tract) engine and
records how far each gets — `load` (parse) → `optimize` (analyse + `into_model`) → `runnable`
(`into_runnable`) — plus a retry that pins a concrete NCHW input shape, since the runtime always feeds
a fixed resolution. The failing stage and error name which models are ready for the tract backend,
which need dynamic-shape/op work, and which must stay on ONNX Runtime.

No weights are converted or regenerated: the sweep reads the same `.onnx` artifacts the runtime
downloads from `xberg-io/layout-models` and `xberg-io/paddleocr-onnx-models` (via the local
HuggingFace hub cache).

## Run

```sh
cargo run -p tract-op-sweep -- [--cache-dir ~/.cache/huggingface/hub] [--json report.json]
```

Models must already be in the cache (run the relevant extraction once, or `hf download`). `--json`
writes the full error chain per model (the table truncates it).

## Coverage matrix (tract 0.23.4)

| Model | Arch | Verdict | Blocker (if any) |
|---|---|---|---|
| `rtdetr` | RT-DETR (NMS-free) | **ready** | — runnable as-is |
| `table_cls` | PP-LCNet (CNN) | **ready** | — |
| `doc_ori` (auto-rotate) | PP-LCNet (CNN) | **ready** | — |
| `textline_ori` | PP-LCNet (CNN) | **ready** | — |
| `angle_cls` | AngleNet (CNN) | **ready, pin input** | runnable once input is `1,3,224,224` |
| `det_v6_medium` | DBNet (CNN) | **ready, pin input** | runnable once input is pinned |
| `rec_v6_medium` | CRNN (CNN + LSTM) | **ready, pin input** | runnable once input is `1,3,48,320` — LSTM path works |
| `tatr` | DETR (Table Transformer) | **ORT-only** | quantized export: pinning the input resolves the conv in-channel, but the `Conv_quant_output_scale_mul` scale const bakes in a symbolic `batch_size` the analyser cannot unify with `1` (Phase-2 finding) |
| `pp_doclayout_v3` | Paddle DETR + NMS | **ORT-only** | 3 inputs (`image`/`im_shape`/`scale_factor`) need explicit facts (mechanical) — but with all three pinned, tract 0.23.4's `LayerNormalization` translator then fails on the DETR decoder's norm layer: `Output mismatch after rewiring expansion for output #0: expected 1,300,1,..,F32 got 1,300,256,F32` (node `LayerNormalization.3`), reproduced at the bare `into_typed()` stage before any declutter/optimize pass — a genuine op-translation bug, not a shape-pinning gap (Phase 5 finding) |
| `db_det_v5_server` | DBNet (CNN) | **needs-work** | stride-2 dim arithmetic tract won't unify (`1+n/2` vs `(n+1)/2`); `det_v6` is a working alternative |
| `slanet_plus` | SLANet+ seq2seq | **needs-work** | data-dependent `Resize` (output size from a runtime tensor) |
| `slanet_wired` | SLANeXt seq2seq | **stays ORT** | `Loop` op unimplemented in tract |
| `slanet_wireless` | SLANeXt seq2seq | **stays ORT** | `Loop` op unimplemented in tract |

**4 ready as-is · 3 ready once input pinned · 6 needs-work (4 of them ORT-only: `tatr`, `pp_doclayout_v3`, `slanet_wired`, `slanet_wireless`).**

## What this means for the rollout

- **CNN classifiers/detectors are the safe first targets** (Phase 1–2): `table_cls`, `doc_ori`,
  `textline_ori` need nothing; `angle_cls`, `det_v6`, `rec_v6` (incl. the CRNN LSTM path) just need
  the input shape pinned, which the backend does anyway.
- **RT-DETR is ready as-is** — better than the issue predicted; no dynamic-shape concretization needed
  for the docling-v2 detector. It becomes the DETR-family seam target in Phase 4 (it takes two inputs —
  image + `orig_target_sizes` — which the backend passes through by name).
- **TATR is ORT-only** — Phase 2 probing showed the issue's "first target" is a *quantized* export:
  pinning the input channel to 3 clears the conv-weight symbol, but the fused `Conv_quant` scale
  multiplier carries a symbolic `batch_size` that tract 0.23.4's HIR analyser refuses to unify with a
  concrete `1` (symbol-scope assertions do not reach that analyser). It joins SLANeXt on ORT; revisit
  only if a non-quantized TATR export or an upstream tract fix lands.
- **`pp_doclayout_v3` is ORT-only** (Phase 5 probe) — pinning its three input facts
  (`im_shape`/`image`/`scale_factor`) clears the symbolic-shape wall noted after Phase 4, but tract
  0.23.4's `LayerNormalization` op translator then fails on the DETR decoder's norm layer with a
  genuine shape-inference bug, reproduced even at the bare `into_typed()` translation stage before any
  declutter/optimize pass runs. Facts pinning was necessary but not sufficient; this is an op gap, not
  the mechanical fix it looked like from the Phase 0/4 symbolic-pass failure alone. Revisit only if an
  upstream tract fix lands.
- **SLANeXt (`Loop`) stays on ORT** as the issue anticipated; `slanet_plus` (data-dependent `Resize`)
  and `db_det_v5_server` (dim-parity) are also blocked — `det_v6` covers detection instead.

Re-run this sweep whenever the model artifacts or the tract version change; it is the gate for the
DETR/table phases.

## Phase 5 finding: `xberg-paddle-ocr` (DBNet/CRNN/AngleNet)

`paddle-parity-probe` (same crate — `cargo run -p tract-op-sweep --bin paddle-parity-probe`) checks
numeric parity between tract and ONNX Runtime on the **exact production artifacts**
`xberg-paddle-ocr` loads by default (PP-OCRv6 `medium` tier detection/recognition + the PP-LCNet
`textline_ori` classifier — not the legacy `ch_ppocr_mobile_v2.0_cls_infer.onnx` the Phase-0
`angle_cls` row above tested; that model is unused by the crate today), and whether one tract plan
pinned at load time can serve more than one input shape.

| Model | Shape | tract vs ORT max \|Δ\| | tract load+pin+optimize | tract run |
|---|---|---|---|---|
| `textline_ori` (AngleNet cls) | `[1,3,80,160]` (fixed — `ANGLE_DST_WIDTH`/`HEIGHT`) | 9.5e-7 | 21ms | 9ms |
| `det_v6_medium` (DbNet) | `[1,3,640,640]` | 7.5e-7 | 150ms | 926ms |
| `det_v6_medium` (DbNet) | `[1,3,320,480]` (re-pinned) | 2.1e-7 | 146ms | — |
| `rec_v6_medium` (CrnnNet) | `[1,3,48,320]` | 2.9e-4 | 85ms | 36ms |
| `rec_v6_medium` (CrnnNet) | `[1,3,48,192]` (re-pinned) | 2.6e-4 | 90ms | — |

Numeric parity is excellent for all three (well under any reasonable OCR-quality tolerance). But a
tract plan pinned+optimized at one shape **cannot run a different shape** — it errors
(`Clashing resolution for expression. 640=640 != 320.`) rather than silently degrading. `DbNet`
resizes to `ScaleParam`-computed dimensions (multiples of 32, different per document) and `CrnnNet`
batches by content-dependent max width, so both see a new concrete shape on most calls in real
usage — unlike `AngleNet` (always `160×80`) or the CNN classifiers/RT-DETR already on the seam
(fixed resolution).

Making tract viable for DbNet/CrnnNet therefore needs a **shape-keyed plan cache** inside the
session (reload+re-optimize only on a shape miss — ~85-150ms here, amortized across repeat page
sizes) rather than xberg's existing `TractBackend`/`TractSession` pattern (one plan built once at
`load()`, assumed to serve every future call). That is a real, materially different session
implementation, plus CRNN-specific tuning (e.g. width-bucketing to raise the cache hit rate,
itself needing its own parity check since it changes the padding CRNN's LSTM sees) — scoped as a
follow-up phase rather than folded into this probe.

## Phase 5 rollout: `layout-tract` (no-ORT layout detection)

`crate::layout` now ships a `layout-tract` feature, the no-ORT sibling of `layout-detection`,
following this table: `rtdetr` and `table_cls` run through the `crate::inference` seam on either
engine, so `LayoutEngine` (RT-DETR detection) works under `layout-tract` and `TableClassifier`
(wired/wireless) works under `layout-tract` + `pdf` (as `android-target` enables). `tatr`,
`slanet_wired`/`slanet_wireless`/`slanet_plus`, and `pp_doclayout_v3` stay
gated behind the literal `layout-detection` (ORT) feature and are never compiled under
`layout-tract` — `LayoutEngine::from_config` returns a `LayoutError` for `ModelBackend::PpDocLayoutV3`
and the YOLO `CustomModelVariant`s instead of failing to compile or panicking. `android-target` (the
x86_64 emulator, which cannot link ONNX Runtime) now enables `layout-tract`, gaining layout
detection + table classification it previously had none of. `layout-detection` remains the native
default and is unaffected — the two variants are additive and never both compiled together in a
single feature selection. Table STRUCTURE recognition (TATR/SLANeXT) is unaffected by this phase
and stays ORT-only; wiring `layout-tract` into the PDF table-structure pipeline
(`crate::pdf::structure`) and widening `wasm-target` are deferred follow-ups.

## Phase 5 finding: `tract-onnx` compiles to `wasm32-unknown-unknown`

A standalone probe crate depending only on `tract-onnx` (0.23.4, `default-features = false`) and
exercising the full `onnx().model_for_read(..).into_optimized().into_runnable()` path **codegens
cleanly for `wasm32-unknown-unknown`** — proving tract is not the WASM blocker. The only fix needed
is `getrandom`'s wasm backend (`--cfg getrandom_backend="wasm_js"` + a wasm32-conditional
`getrandom {0.2,0.3,0.4}` dep), which `crate::xberg` already carries (`.cargo/config.toml` +
`Cargo.toml` wasm32 target block). So adding `tract` to `wasm-target` is unblocked from tract's side;
the remaining WASM work is xberg-side: a pre-existing `xberg-wasm` wasm32 build break unrelated to
tract (`mio`/`tokio` pulled via other deps), streamed weights through `load_from_memory` (no
hf-hub/tokio-DNS on wasm32), `ocr-wasm` wiring + `alef.toml` un-strip, and the 50 MB `.wasm` size
check.

## Phase 5 measurement: tract-vs-ORT layout latency

Apple Silicon (aarch64), `--release`, single-thread CPU, best-of-8 warm `run()` samples on the three
seam models. Reproduce with:

```sh
cargo test --release -p xberg --no-default-features --features "layout-detection,auto-rotate,tract" \
  --lib inference::tract_backend::tests::tract_vs_ort_latency_report -- --ignored --nocapture
```

| model | tract load (ms) | ORT load (ms) | tract run (ms) | ORT run (ms) | tract/ORT run |
|---|---|---|---|---|---|
| RT-DETR layout detector | 465 | 221 | 2637 | 137 | 19.3x |
| PP-LCNet table classifier | 22 | 9 | 31.9 | 2.2 | 14.4x |
| PP-LCNet doc-orientation | 22 | 8 | 31.9 | 2.8 | 11.5x |

tract's pure-Rust CPU kernels run **~11-19x slower than ONNX Runtime** — expected, and the accepted
trade-off: these models run roughly once per page, and on the targets tract exists for (WASM /
Android x86_64, where ORT cannot link at all) the alternative is *no* layout/orientation, not ORT.
On native builds ORT stays the default, so this regression never reaches native users. RT-DETR's
~2.6 s/inference is the ceiling to watch for WASM UX; the CNN classifiers at ~32 ms are comfortable.

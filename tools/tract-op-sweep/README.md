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
| `pp_doclayout_v3` | Paddle DETR + NMS | **needs-work** | 3 inputs (`image`/`im_shape`/`scale_factor`) all need explicit facts |
| `db_det_v5_server` | DBNet (CNN) | **needs-work** | stride-2 dim arithmetic tract won't unify (`1+n/2` vs `(n+1)/2`); `det_v6` is a working alternative |
| `slanet_plus` | SLANet+ seq2seq | **needs-work** | data-dependent `Resize` (output size from a runtime tensor) |
| `slanet_wired` | SLANeXt seq2seq | **stays ORT** | `Loop` op unimplemented in tract |
| `slanet_wireless` | SLANeXt seq2seq | **stays ORT** | `Loop` op unimplemented in tract |

**4 ready as-is · 3 ready once input pinned · 6 needs-work (3 of them ORT-only: `tatr`, `slanet_wired`, `slanet_wireless`).**

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
- **`pp_doclayout_v3` needs its three input facts pinned** (Phase 4) — a mechanical fix, not an op gap.
- **SLANeXt (`Loop`) stays on ORT** as the issue anticipated; `slanet_plus` (data-dependent `Resize`)
  and `db_det_v5_server` (dim-parity) are also blocked — `det_v6` covers detection instead.

Re-run this sweep whenever the model artifacts or the tract version change; it is the gate for the
DETR/table phases.

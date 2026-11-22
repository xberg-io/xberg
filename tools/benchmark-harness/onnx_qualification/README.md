# Held-out scoring for CPU ONNX candidate qualification

This package scores the held-out work that the offline NLI contract deliberately leaves out. It
does not choose a default, a preset or a bundle, and it cannot mark a model qualified: every plan,
report, score and decision it writes carries `qualified: false` and `defaults_changed: false`.

It complements `../nli_qualification/`:

- `nli_qualification` freezes a plan, runs inference through `LocalTextClassifier` and refuses to
  accept an unaccounted report. It computes no quality metric.
- `onnx_qualification` audits the committed corpora, builds the frozen plans from the canonical
  inventory, runs the captioning path through `LocalImageCaptioner`, computes macro-F1, per-class
  recall and confusion, calibration, abstention, unknown-class rejection, source-family bootstrap
  intervals and the captioning subset that is checkable without a human, then writes a per-workload
  verdict from a preregistered bar.

## Commands

Portable (no ONNX Runtime, no model):

```sh
onnx-qualification audit MANIFEST                 # validate a classification corpus manifest
onnx-qualification audit-captions MANIFEST        # validate a caption corpus manifest
onnx-qualification prereg PREREGISTRATION         # validate and render the posted bar
onnx-qualification score-classification PLAN SHA REPORT MANIFEST PREREG OUT
onnx-qualification score-captioning PLAN REPORT MANIFEST PREREG OUT
onnx-qualification decide PREREG CLASSIFICATION|- CAPTIONING|- OUT MARKDOWN
onnx-qualification controls OUT                   # negative controls
```

With the `runtime` feature (the only build that loads a model):

```sh
onnx-qualification identity DEADLINE_MS
onnx-qualification plan-classification MANIFEST AUDIT TEXTS INVENTORY IDENTITY PREREG OUT
onnx-qualification plan-captioning MANIFEST AUDIT INVENTORY IDENTITY PREREG OUT
onnx-qualification run-classification PLAN SHA OUT
onnx-qualification run-captioning PLAN SHA IMAGES OUT
```

`IDENTITY` is the JSON printed by `onnx-qualification identity DEADLINE_MS` from the same
runtime-enabled binary that runs inference, because the identity binds that executable's SHA-256.
Both runners refuse to run when the identity changes before or during inference.

```sh
ORT_LIB_LOCATION=/path/to/onnxruntime/lib ORT_PREFER_DYNAMIC_LINK=1 \
  cargo build --manifest-path tools/benchmark-harness/onnx_qualification/Cargo.toml --features runtime
```

`ORT_PREFER_DYNAMIC_LINK=1` is required with a shared ONNX Runtime; without it `ort-sys` attempts
static linking and fails on a NuGet or release archive that ships only the dylib.

## Why the plan is derived, not written

Both plan builders read the canonical inventory at
`crates/xberg/src/candidate_models/inventory.json` (schema 2) and derive:

- the graph the host loads (`model_qint8_arm64.onnx` on aarch64, `model_quint8_avx2.onnx`
  elsewhere);
- the complete SHA-256 and size manifests the adapter verifies before reading bytes;
- two provenance receipts computed from the inventory entry itself:
  `export_recipe_sha256` over the pinned file list and upstream revision,
  `license_receipt_sha256` over the declared license, caveat and upstream revision.

A plan therefore cannot claim a revision, graph or license the inventory does not pin. The
recording is not a licence grant; it is a digest of the inventory's claim.

## Metric definitions

The authoritative text is `Preregistration::render_markdown`, generated from the same file that is
scored. In short:

- **macro-F1** over the vocabulary classes with held-out support; abstentions are recall errors;
- **per-class recall, precision and the full confusion matrix**;
- **ECE** over ten top-1 confidence bins, answered known-class predictions only;
- **abstention** rate on known classes and **unknown-class rejection** on documents whose gold set
  is empty;
- **95% source-family cluster bootstrap** (whole families resampled, fixed seed), plus paired
  per-image differences for captioning arms;
- **captioning**: reference-number recall, reference-term recall (omission proxy) and
  invented-number candidates.

Every metric is an `Estimate`: a value with its observation count, or `unmeasured` with a reason.
A denominator of zero never becomes a zero.

## Negative controls

`onnx-qualification controls OUT` mutates a valid fixture once per control and requires the real
validator or scorer to reject it with a specific message:

| control | required rejection |
| --- | --- |
| empty corpus | `corpus must not be empty` |
| no held-out documents | `corpus has no held-out documents` |
| corpus digest tamper | `corpus digest mismatch` |
| family spanning splits | `family spans two splits` |
| wrong expected count | `does not equal its factors` |
| skipped inference | `inference must complete` |
| missing prediction | `inference row count mismatch` |
| unexpected document | `unexpected document or repeat` |
| unknown backend | `unknown variant` |
| corrupt artifact | `model identity mismatch` |
| missing companion artifact | `missing or ambiguous companion artifact pin` |
| empty caption generation set | `no held-out generation` |
| skipped caption inference | `caption inference must complete` |
| plan/corpus disagreement | `absent from the committed corpus` |

A control passes only when the expected failure is observed; a control that is accepted is
reported as `FAILED` and the command exits non-zero.

## Known limits of this package

- It authenticates accounting, not execution. A hand-written report can satisfy every count.
  `runner_sha256` and per-row durations are self-reported.
- Scores are only as trustworthy as the committed manifest: the corpus bytes are fetched at plan
  time and verified by digest, but nothing proves the manifest was built honestly.
- The cue-free sensitivity stratum is small and reported, not used as the headline.
- Captioning fact precision is not computed anywhere in this package.

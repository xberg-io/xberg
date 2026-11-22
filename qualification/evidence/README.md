# Held-out qualification result (#2334)

Preregistered bar: <https://github.com/xberg-io/xberg/issues/2334#issuecomment-7067589364>
(amendment 2; amendment 1 and the original posting are earlier entries in the same thread).
Committed bar: `../preregistration.json` — it renders byte-for-byte to the posted bar.

Both workloads fail the preregistered bar. No default, preset, bundle or supported-model claim
changes: every artifact here says `qualified: false`, and `decision.json` says
`defaults_changed: false`.

| workload | verdict | decisive numbers |
| --- | --- | --- |
| classification (NLI-MiniLM2, int8) | **no-go** | macro-F1 0.3214, CI95 [0.2305, 0.3849]; every class below the recall floor except `proposed rule`; 71.0% abstention; ECE 0.2044 |
| captioning (SmolVLM-256M/500M, fp32+int8) | **no-go** | reference-term recall 0.1971–0.2222 (floor 0.35); reference-number recall 0.00–0.05 (floor 0.20) |
| Florence-2-base | **not measured** | the adapter refuses the merged decoder before any image is read (see `florence-refusal.txt`) |

## Inputs

| item | value |
| --- | --- |
| classification corpus | `../classification/federal-register-2026/manifest.json`, 408 documents, digest `19ec5bb7b332ff89be30eed720d446fd138670144e4f8fb0b7ea02d78d68eba2`, file sha256 `a59a67c614bc451c7de925f2b91b3cd9951fabb15cb5a3451990f2b2b5150d4d` |
| captioning corpus | `../captioning/coco-val2017/manifest.json`, 30 images, digest `1bf64e698dbc64ea7a2c3ecbe1ceb6596bd0e1edfa3525810e45065c2f721ba6`, file sha256 `ff07b9971f8f8210e21c76cb5a7d209c97dfd93721fd8fd7ea5c0b4ed8f55cc4` |
| classification plan | `classification/plan.json`, sha256 `4a71679b3a0ff0409ffdec969e16e4488157ff30b20431d83b567720b019215b`, 273 documents × 1 model × 1 arm × 3 repeats = 819 rows |
| captioning plan | `captioning/plan.json`, sha256 `599bc9c7581fb98ca1401f6b727707291b54c47a8e247c1dd13e3cc13074ca2a`, 30 images × 2 models × 2 arms × 1 repeat = 120 rows |
| candidate revisions | `xberg-io/classification-models@a29f2096c723fd39c45b7c21690ea0318866bd0b`, `xberg-io/captioning-models@7b8359d8ab0086922b91b7dfde6577c1a30f7442` |
| runner | `0755ab073af1c255f48eb246a3999ff4861fdcbc5d56cb97a0307e3ec566d189` (this package, `runtime` feature) |
| runtime | ORT 1.24.2, `git-commit-id=058787ceea`, Release; ort crate 2.0.0-rc.15; threads 0 (SDK default), serial runner |
| host | Apple M4 Pro, 48 GiB, macOS 27.2.1; **shared and busy** (see `host-and-load.md`; no timing claim is made) |

Every declared count was asserted before scoring and is recorded in `*/score.json`. All 9
classification and 9 captioning assertions matched; the runs would have been refused otherwise.
`decision.json` binds the bar with `prereg_sha256`, the SHA-256 of the parsed preregistration
computed by the tool, not the SHA-256 of the file bytes.

## Classification

`nli-minilm2-l6-h768`, quantized arm, held-out 138 known-class documents + 48 unknown-class
documents (presidential documents outside the vocabulary), threshold 0.50, repeats 1–3.

| metric | value |
| --- | --- |
| macro-F1 | **0.3214** (CI95 0.2305–0.3849, 24 source-family clusters, 2000 replicates) |
| micro accuracy | 0.2319 (32/138) |
| abstention on known classes | 0.7101 (98/138) |
| selective accuracy (answered only) | 0.8000 (32/40) |
| unknown-class rejection | 0.7917 (38/48 abstained) |
| forced label on unknown class | 0.2083 (10/48, all `public notice`) |
| ECE (10 bins, answered only) | 0.2044 |
| repeat stability | 0.0 across three repeats |

Per class:

| class | support | recall | precision | F1 |
| --- | --- | --- | --- | --- |
| final rule | 49 | 0.0204 | 1.0000 | 0.0400 |
| proposed rule | 48 | 0.4583 | 0.7333 | 0.5641 |
| public notice | 41 | 0.2195 | 1.0000 | 0.3600 |
| unknown (no vocabulary label) | 48 | — | — | — |

Confusion (gold → predicted, top rows): `final rule → abstain` 45, `unknown → abstain` 38,
`public notice → abstain` 27, `proposed rule → abstain` 26, `proposed rule → proposed rule` 22,
`unknown → public notice` 10, `public notice → public notice` 9, `public notice → proposed rule` 5,
`final rule → proposed rule` 3, `final rule → final rule` 1.

**Where the macro-F1 goes:** the model answers 29% of held-out documents and is right 80% of the
time when it answers; abstention removes 71% of the recall. Precision of the two minority
predictions is 1.0, so this is not a wrong-label problem at the frozen threshold — it is a
confidence-probability problem. The observed confidences sit just above 0.50 (median 0.546 among
answered rows), so the fixed threshold discards almost everything.

**This cannot be recovered post hoc.** The SDK applies the threshold inside `classify` and returns no
label below it, so the report cannot be re-filtered at a different threshold. A future run must
freeze a permissive threshold and apply the decision threshold at scoring time; this package does
not do that yet, and the frozen bar is what the verdict uses.

**The lexical cue is not what is driving the result.** Held-out documents whose text contains their
own label phrase score macro-F1 0.2745 (53 documents); documents that do not score 0.2525 (85
documents). Both are far below the bar, so the corpus's residual cue is not the explanation.

**Reference baseline:** always predicting the most frequent held-out class (`final rule`) gives
macro-F1 0.1747 and accuracy 0.3551. The candidate beats that baseline on macro-F1 by 0.1467 but
not on accuracy (0.2319 vs 0.3551), because it abstains.

Calibration split (never used for the verdict): macro-F1 0.2514.

Adverse examples (`classification/score.json` keeps twelve):

- `fr-2026-18830` (environmental-protection-agency), gold `final rule` → `proposed rule` at 0.534
- `fr-2026-23050` (labor-department), gold `final rule` → `proposed rule` at 0.522
- `fr-2026-18048`, `fr-2026-19047`, `fr-2026-20089`, `fr-2026-20620` (committee-for-purchase-from-people-who-are-blind-or-severely-disabled), gold `public notice` → `proposed rule` at 0.54–0.56
- `fr-2026-17876` (federal-maritime-commission), gold `final rule` → abstained
- `fr-2026-17863` (agriculture-department), gold `public notice` → abstained

## Captioning

Two candidates, both arms, 30 held-out COCO val2017 images, one repeat.

| candidate / arm | reference-term recall (CI95) | reference-number recall | images with an invented number | median latency |
| --- | --- | --- | --- | --- |
| smolvlm-256m / fp32 | 0.2222 (0.1921–0.2551) | 0.050 (n=10) | 0.1333 | 1.8 s |
| smolvlm-256m / int8 | 0.2180 (0.1800–0.2590) | 0.000 (n=10) | 0.0333 | 2.1 s |
| smolvlm-500m / fp32 | 0.2255 (0.1813–0.2758) | 0.000 (n=10) | 0.0667 | 3.4 s |
| smolvlm-500m / int8 | 0.1971 (0.1633–0.2354) | 0.000 (n=10) | 0.0667 | 3.3 s |

Paired fp32 − int8 differences (per image, 95% image-cluster bootstrap):

- smolvlm-256m: term recall −0.0367 to +0.0453; number recall 0.00 to +0.15
- smolvlm-500m: term recall −0.0126 to +0.0669; number recall 0.00 to 0.00

Both intervals include zero, so **int8 is indistinguishable from fp32 on this measurement** — the
only FP32/quantized parity result this submission can support, because the NLI fp32 graph cannot be
loaded at all.

No empty captions. Invented-number candidates are dominated by trivial numerals (`1`, `10`, `12`,
`15`); they are unverified candidates for a rater, not a fact-precision number. The generated
captions are long and generic ("The image depicts a living room with a modern and minimalist
design...") and recall roughly a fifth of the reference content words.

**No fact-precision number is reported anywhere.** Visible-fact precision, invented entities and
relations, and illegibility handling need blind human raters; the preregistration says so and the
verdict is no-go on automatic bars alone, so no human study is justified yet.

## Negative controls

Two layers, both executed:

1. `controls/report.json` — 14 synthetic mutations of a valid fixture, each rejected for the
   intended reason (empty corpus, no held-out documents, digest tamper, family leakage, wrong
   expected count, skipped inference, missing prediction, unexpected document, unknown backend,
   corrupt artifact identity, missing companion artifact, empty caption set, skipped caption
   inference, plan/corpus disagreement). 14/14 fired; a control that is accepted is reported as
   failed and exits non-zero.
2. `controls/real-artifact-controls.md` — the same refusals against the real run artifacts: a real
   corrupted 83 MB graph (`Local NLI artifact checksum mismatch`), one dropped row
   (`inference row count mismatch`), a skipped row, an unknown backend, a wrong expected count, and
   an emptied corpus. Each produced an error and no report.

## What remains unmeasured

- Visible-fact precision, invented entities/relations, illegibility handling (needs raters).
- Florence-2-base (adapter refusal, recorded verbatim).
- FP32 versus quantized for NLI (the fp32 graph exceeds the adapter's 256 MiB cap by design).
- OCR-degraded and multilingual inputs (the corpus is native digital English).
- A trained fixed-taxonomy head (no candidate is staged).
- Threshold sensitivity (the SDK discards below-threshold labels, so it cannot be recomputed).
- Isolated CPU timing and peak RSS (shared host, load 20–47 during the runs).

## Host identity and load

Recorded in `host-and-load.md`. The classification run started at load 24.16 and took about seven
minutes for 819 inferences (median 426 ms, p95 1134 ms per call, 422 s of measured inference). The
captioning run finished at load 31.87 and produced 120 generations (median 1.8–3.4 s per
generation). These are observed wall-clock times on a busy shared workstation; **no
isolated-resource, cold-load, throughput or RSS claim is made.**

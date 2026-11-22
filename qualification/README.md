# Qualification evidence (#2334)

This directory holds the frozen inputs and the produced evidence for the CPU classification and
captioning qualification in [xberg#2334](https://github.com/xberg-io/xberg/issues/2334). Nothing
here is a default, a preset, a bundle or a supported-model claim. Every plan, report and score
carries `qualified: false`, and the decision records `defaults_changed: false`.

| path | what it is |
| --- | --- |
| `preregistration.json` | the bar that was posted before the held-out run: candidate revisions, corpus digests, metric definitions, expected counts, margins |
| `classification/federal-register-2026/manifest.json` | per-document pins (source sha256, text sha256, family, split, gold type) |
| `classification/federal-register-2026/LICENSE-AUDIT.md` | license, training-overlap and label-leakage audit |
| `captioning/coco-val2017/manifest.json` | per-image pins and five reference captions each |
| `captioning/coco-val2017/LICENSE-AUDIT.md` | license and evaluation-use audit |
| `evidence/` | frozen plan, SDK report, scores, decision and negative-control evidence |

## Reproducing

Corpus bytes are never committed. The acquisition scripts re-fetch them from the pinned public
sources and the manifests verify every digest:

```sh
python3 tools/benchmark-harness/onnx_qualification/scripts/acquire_classification_corpus.py \
  --start 2026-09-01 --end 2026-11-20 --cap 120 --unknown-cap 48 \
  --out /path/to/text-cache --manifest qualification/classification/federal-register-2026/manifest.json

python3 tools/benchmark-harness/onnx_qualification/scripts/acquire_captioning_corpus.py \
  --annotations captions_val2017.json --images /path/to/images --count 30 \
  --manifest qualification/captioning/coco-val2017/manifest.json
```

Re-acquisition does not reproduce the same corpus: the manifest pins the exact document and image
set that was frozen. A new acquisition produces a new manifest and therefore needs a new
preregistration.

The package itself is documented in
[`tools/benchmark-harness/onnx_qualification/README.md`](../tools/benchmark-harness/onnx_qualification/README.md).

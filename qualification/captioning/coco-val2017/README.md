# COCO val2017 captioning corpus (30 images)

Thirty images from the COCO 2017 `val2017` split, each with its five reference captions. Images
are fetched on demand from the pinned cocodataset.org URL; only digests and reference captions are
committed.

- **Task:** open-ended image captioning measurement for SmolVLM-256M, SmolVLM-500M and
  Florence-2-base CPU ONNX candidates.
- **Source:** `http://images.cocodataset.org/annotations/annotations_trainval2017.zip`
  (SHA-256 `afe3b30e403dd7f228e2373023abbd60042a6e10ec6874d3652df034d289ebb9`), images from
  `http://images.cocodataset.org/val2017/<file_name>`.
- **License:** annotations CC BY 4.0; images Flickr-hosted and used for evaluation only. See
  `LICENSE-AUDIT.md`.
- **Pins:** `image_sha256` and `image_bytes` per image, `digest` over the image list, and the
  annotation archive digest in `source`.

## Selection

Deterministic stride over the sorted val2017 image ids, stratified so that the number measurement
is not vacuous:

- 10 images whose five reference captions contain at least one digit ("numbers" stratum),
- 20 images from the remaining pool ("background" stratum).

No random seed is involved. The stratification is declared in the manifest and in the
preregistration, so it cannot be tuned after the run. An earlier uniform stride selected 1 of 30
number-bearing images, which would have made a number metric meaningless; that is why the
stratification exists, and it is recorded rather than hidden.

## What this corpus can and cannot measure

Checkable automatically against the five references:

- **reference-number recall** — distinct numbers in the references that appear in the caption;
- **reference-term recall** — reference content words found in the caption (an omission proxy);
- **invented-number candidates** — numbers in the caption absent from every reference.

Not checkable automatically:

- **visible-fact precision** — needs a rater who can see the image and blind-verify each asserted
  fact;
- **invented entities and relations** — a term can be absent from five short references and still
  be correct, so absence is not invention;
- **illegibility handling** — COCO has no illegible-region annotations.

COCO captions are short, generic descriptions. They are a legitimate licence-clear evaluation
set, but the package itself uses a **pipeline sanity measurement**, not a document-captioning
benchmark.

## Reproduce

```sh
python3 tools/benchmark-harness/onnx_qualification/scripts/acquire_captioning_corpus.py \
  --annotations captions_val2017.json --images /path/to/images --count 30 \
  --manifest qualification/captioning/coco-val2017/manifest.json
```

## Frozen bytes

`manifest.json` carries two independent pins: `digest` covers the canonical document list (sorted
keys, no whitespace), and the file SHA-256 is pinned in `qualification/preregistration.json` and in
every plan's `provenance.corpus_receipt_sha256`. Reformatting the file leaves `digest` unchanged and
changes the file pin, so the two must be updated together. Any regeneration produces a new file and
requires a new preregistration.

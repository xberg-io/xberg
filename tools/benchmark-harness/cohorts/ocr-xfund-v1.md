# `ocr-xfund-v1` cohort

140 XFUND filled forms, 20 per language (zh, ja, es, fr, it, de, pt), with ground truth from the XFUND entity
annotations. XFUND is CC BY-NC-SA 4.0, so neither the images nor the ground truth are in git or the public
bucket: they are pinned by `test_documents/reference_ocr/corpus.lock.json` and served from the private
`gs://xberg-benchmark-corpus` bucket (provenance: `test_documents/reference_ocr/PROVENANCE.md`).

The descriptors live in `fixtures-reference/`, outside `fixtures/`, so the strict ground-truth gate over
`fixtures/` does not fail on a checkout without the private files. Every member also sets `metadata.opt_in`.

```bash
python3 test_documents/scripts/fetch_corpus.py --manifest test_documents/reference_ocr/corpus.lock.json \
  --root test_documents/.reference-ocr-cache --bucket xberg-benchmark-corpus --auth
cargo run -p benchmark-harness -- run \
  --fixtures tools/benchmark-harness/fixtures-reference \
  --cohort tools/benchmark-harness/cohorts/ocr-xfund-v1.json \
  --frameworks xberg --mode batch --measure-quality
```

Members are ordered by language and `batch_size` is 10, so no batch mixes OCR languages.

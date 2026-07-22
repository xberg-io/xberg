# Benchmark corpus: vendor / reference split

The PDF benchmark runs over 165 fixtures in `tools/benchmark-harness/fixtures/pdf/`. Each fixture
points at a source document and its ground truth (GT). Documents fall into two redistribution
classes, decided per-document by `scripts/build_corpus.py` and recorded in
`test_documents/ground_truth/corpus_manifest.json` (`redistribute`):

- **vendor (73)** — redistributable (permissive/PD sources). PDF + GT are committed to the
  `test_documents` submodule under `pdf/` and `ground_truth/pdf/`. PDFs are Git LFS objects.
- **reference (92)** — license-restricted (arXiv via ReaDoc, ParseBench). The bytes are **never
  committed**. Their fixtures point into the gitignored cache
  `test_documents/.corpus-cache/{pdf,ground_truth/pdf}/`, which is materialized on demand.

The reference cache is mirrored in a **private GCS bucket** so CI can run the full corpus without the
non-redistributable bytes ever entering the public repo.

## Running the benchmark locally

1. Materialize the reference cache once (needs the pinned upstream datasets):
   `python tools/benchmark-harness/scripts/build_corpus.py --stage materialize`
   — or restore it from the private bucket (needs GCS access):
   `task benchmark:corpus:cache:restore`
2. Ensure vendor LFS objects are present: `git -C test_documents lfs pull`
3. Run a benchmark, e.g. `task benchmark:local`.

Verify the corpus resolves before a run:
`cargo run -p benchmark-harness -- validate-gt --fixtures tools/benchmark-harness/fixtures/pdf/ --strict`
(exit non-zero if any fixture's GT is missing — e.g. the reference cache was not restored).

## Reproducible cohorts

Tracked manifests under `cohorts/` select fixture descriptors in an exact order and declare a
fixed native batch size. Paths are normalized relative paths rooted at the directory passed to
`--fixtures`. The manifest fixture count must be divisible by `batch_size`; adapter filtering is
validated again at runtime so an unsupported fixture cannot silently create a partial batch.

Use the small iteration cohort with:

```bash
cargo run -p benchmark-harness -- run \
  --fixtures tools/benchmark-harness/fixtures \
  --cohort tools/benchmark-harness/cohorts/layout-pdf-fast.json \
  --frameworks xberg-markdown-layout,docling,liteparse \
  --mode batch
```

## CI (`.github/workflows/benchmarks.yaml`)

The `setup` job authenticates to GCS via Workload Identity Federation, restores `.corpus-cache` for
the checked-out `test_documents` SHA, runs the strict GT gate, and uploads the cache as the
`benchmark-corpus-cache` artifact. Every extraction job downloads that artifact and runs
`git -C test_documents lfs pull` for the vendor PDFs. Auth uses org secrets
`GCP_WORKLOAD_IDENTITY_PROVIDER`, `GCP_BENCHMARK_SA`, `GCP_BENCHMARK_BUCKET`; the WIF principal is
read-only and scoped to `xberg-io/xberg` and `xberg-io/xberg-enterprise` (provisioned in
`infra/terraform/staging`).

## GCS layout

`gs://xberg-benchmark-corpus/corpus-cache/<test_documents_sha>.tar.zst` — a zstd tarball of
`.corpus-cache/pdf` + `.corpus-cache/ground_truth/pdf`, keyed by the submodule commit it was built
from. `restore-corpus-cache.sh` fetches the object for the current SHA and fails with a clear message
if it is absent.

## Corpus-rebuild sequence (do these together)

1. Rebuild the corpus: `python tools/benchmark-harness/scripts/build_corpus.py --stage all`
   then `--stage materialize`. Commit the regenerated fixtures + GT + manifest in `test_documents`
   and bump the submodule pin in this repo.
2. Publish the new reference cache for the new SHA: `task benchmark:corpus:cache:publish`
   (needs write access to the bucket + a locally materialized cache).
3. Bump `xberg-enterprise`'s `test_documents` pin to the same commit — the enterprise
   `test_documents-pin` drift guard fails until it matches this repo's pin.

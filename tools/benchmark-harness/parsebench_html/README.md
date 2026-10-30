# ParseBench HTML qualification

This runner compares baseline and candidate on all 503 pinned ParseBench table records, in three arms: native HTML, layout/TATR HTML, and forced OCR with layout HTML. It schedules exactly 3,018 invocations. Corpus verdicts remain metadata; they never silently filter inference coverage or the metric denominator.

The dataset revision is `2805a1d940f95a203e0ae4b88be9934f7765b3fc`; the official evaluator revision is `328c71109e0b955f66583c33ec95ac52d9749db4`. Annotations are Apache-2.0; source PDFs are reference-only and must remain outside Git. The official full `ParseEvaluator.evaluate` pipeline handles title merging/stripping, shared table extraction, optional merged-table splitting, GriTS pairing, and TableRecordMatch. Per-record JSON `rule` options are preserved. Forty-five records mark TRM unsupported; report that subgroup separately. No metric replacement is used.

Sources: [pinned public dataset](https://huggingface.co/datasets/llamaindex/ParseBench/tree/2805a1d940f95a203e0ae4b88be9934f7765b3fc) and [pinned evaluator](https://github.com/run-llama/ParseBench/tree/328c71109e0b955f66583c33ec95ac52d9749db4). PDF downloads were verified against the pinned dataset tree's Git blob SHA-1 or LFS SHA-256, then recorded with local SHA-256. Raw HTML hashes also match the curated corpus manifest. Thirty-four PDFs reused the verified local public cache; 469 were downloaded from the pinned public revision.

## Reproduce acquisition and staging

The corpus submodule supplies the reference-only downloader and generated upstream inventory. Initialize it at the recorded Git pin before acquisition. Keep both the cache and qualification stage outside every Git checkout; PDF content rights remain unreviewed, independently of the Apache-2.0 annotation license.

```sh
python test_documents/scripts/fetch_benchmark_upstreams.py --source parsebench-tables --root /external/path/benchmark-cache
python tools/benchmark-harness/parsebench_html/stage.py --corpus-root test_documents --cache-root /external/path/benchmark-cache --output /external/path/new-stage
```

The staging adapter performs no downloads or extraction. It verifies all 503 PDF pins and the independently frozen original `table.jsonl` hash through the corpus helper, checks unique source IDs against the PDF inventory, and copies the original PDF bytes plus UTF-8 bytes of each raw `expected_markdown` HTML string. It preserves every upstream row and its original evaluation `rule`, without Markdown conversion or ground-truth normalization. The resulting `stage.json` uses the existing runner contract, includes source/evaluator revisions, inventory hash, attribution and license URI, and is deterministic for the same inputs. Output must be absent or empty.

Optionally supply `--curated-manifest test_documents/ground_truth/corpus_manifest.json` to attach existing reference metadata. A verdict never filters the 503 records; missing curation is explicitly `UNREVIEWED`. Supplied entries must match the pinned revision and original oracle hash. The manifest records the curation file hash separately. `--registry` can select another copy of the generated upstream inventory; the child helper still requires its complete pinned source identities and counts, and staging independently requires the frozen original JSONL hash.

Run the staging controls with the corpus helper available at `test_documents`, or set `XBERG_BENCHMARK_CORPUS_ROOT` to its checkout when testing an isolated worktree:

```sh
python -m unittest discover -s tools/benchmark-harness/tests -p 'test_parsebench_html_staging.py'
```

## Local evidence

On 2026-10-30 the verified external stage contained 503 unique, hash-verified PDFs (79,310,139 bytes), 503 hash-verified original HTML oracles, and `stage.json` with per-file SHA-256, source metadata, curation, and licenses. All 503 PDFs opened in PyMuPDF, with 503 pages total. The raw `table.jsonl` SHA-256 is `66129fcc9e68ae0bf40e7d41f540002ceff9bedf46d6dafff2321ba2b7ab5231`; the curated corpus manifest SHA-256 is `bbf64a1f8e617993d00dcac63d987c77fb232eb070db3e2fc22829661525ec65`.

An offline official-evaluator self-control on a real two-table record scored GriTSCon=1 and TRM=1 with provider keys removed, chart normalization off, and socket connections forbidden. This establishes evaluator availability. It is not an extraction quality result. No native/model qualification was executed while constructing the runner.

## Environment and controls

Use a separate environment and the repository-pinned uv/Python tools, preserving other frozen benchmark environments:

```sh
uv venv /external/path/venv --python 3.13.18
uv pip sync --python /external/path/venv/bin/python --require-hashes tools/benchmark-harness/parsebench_html/requirements.lock
```

The direct requirements start from the pinned upstream `uv.lock`, plus the imported Anthropic client and PyMuPDF. Bleach 6.4.0 and lxml 6.1.3 replace the upstream versions to include available security fixes; rerun all official controls when recreating this environment. The generated hash lock resolves 62 packages. The Anthropic client is imported by upstream; no provider is called by this table-only evaluator. Set `PYTHONPATH=/external/path/ParseBench/src` to the clean pinned upstream checkout, and disable chart normalization explicitly:

```sh
export LLAMACLOUD_BENCH_LLM_NORMALIZATION=off
/external/path/venv/bin/python -m unittest discover -s tools/benchmark-harness/tests -p 'test_parsebench_html_*.py'
```

The integrity controls use synthetic bytes only to test checksums, paths, counts, and schedules; they do not establish valid PDF extraction. Official-metric controls cover identical HTML, empty predictions, lost spans, header tags, and joined tables. Both GriTSCon and TRM ignore a pure `th` to `td` change in the header control, scoring 1.0: HTML structure must also be inspected when assessing header recovery.

## Freeze before execution

Create a private specification JSON. Both `baseline` and `candidate` require a full `commit`, absolute `executable`, `build_evidence`, `runtime_evidence`, `environment`, and all three `arms`. Required environment keys are absolute `HF_HUB_CACHE`, `TESSDATA_PREFIX`, `ORT_DYLIB_PATH`, plus `XBERG_ORT_EP=cpu`, `HF_HUB_OFFLINE=1`, and positive integer strings for `OMP_NUM_THREADS` and `RAYON_NUM_THREADS`. Optional thread pins are `OPENBLAS_NUM_THREADS`, `MKL_NUM_THREADS`, `VECLIB_MAXIMUM_THREADS`, and `NUMEXPR_NUM_THREADS`. The subprocess inherits only HOME/PATH/locale plus this explicit environment and isolated cache directories.

Build evidence is JSON with `commit`, `executable_sha256`, and the expanded `features` list containing `pdf`, `ocr`, and `layout-detection`. Runtime evidence is JSON with `executable_sha256`, `ort_sha256`, `ort_version`, `execution_provider="cpu"`, `positive_model_pilot=true`, and a nonempty `pilot_artifacts` list of `{file, sha256}` entries. These are real operator-produced build/pilot artifacts; a metadata assertion alone cannot prove inference fired. Review positive layout and TATR activation in the real pilot before freezing. RT-DETR should run for each model-arm page; TATR can legitimately have no forward pass where no table regions were detected.

Each arm has an absolute `config` and the exact canonical `argv` array `["extract", "{pdf}", "--config", "{config}", "--format", "json"]`, with `content_path=["result", "content"]`. This matches the supported Extract Clap command and ExtractEnvelope JSON schema. Other arguments are rejected: inline JSON and individual flags would override the frozen configuration. Both placeholders are mandatory and replaced only with the scheduled verified PDF and frozen config. The runner does not use a shell. Include `models` as a nonempty list of objects with absolute runtime `file`, upstream revision, and license, including every loaded model and Tesseract traineddata, plus absolute `evaluator_root` and `requirements_lock`. Supply the actual runtime cache paths, including snapshot symlinks; preserving these paths lets the verifier detect retargeted links.

The freeze step copies executable, configs, ORT library, and provenance artifacts into content-addressed read-only files beside the frozen plan. It verifies source/copy hash agreement, checks executable/config/ORT hashes before and after every invocation, and hashes model bytes, evaluator Python sources, dependency lock, and stage manifest. It verifies a clean evaluator checkout. Configurations must request HTML and explicitly disable both extraction and Tesseract cache. Model arms require `layout.strategy="always"`, `layout.table_model="tatr"`, `layout.acceleration.provider="cpu"`, and a positive `concurrency.max_threads`. The native `layout_tatr_html` arm also requires `use_layout_for_markdown=true`: despite its name, this opt-in activates the native PDF layout route for HTML output too; its default is false. The forced OCR arm uses its independent OCR layout route and does not require this native-path opt-in. There is no `native_pdf` layout field in the current schema. The operator must review model and arm configuration identities before freezing.

The runner rehashes the scheduled PDF, oracle, runtime model paths, dependency lock, and evaluator source files before/after extraction and after scoring. Finalization revalidates the entire corpus and plan, records final integrity explicitly, and fails if anything changed. Repeated hashing adds harness overhead; report native extraction latency from the saved CLI envelope separately from total sweep duration. These boundary checks detect ordinary concurrent mutation; they are not a claim of adversarial atomic isolation.

```sh
/external/path/venv/bin/python tools/benchmark-harness/parsebench_html/runner.py freeze --stage /external/path/stage --input /external/path/spec.json --output /external/path/frozen-plan.json
/external/path/venv/bin/python tools/benchmark-harness/parsebench_html/runner.py run --stage /external/path/stage --input /external/path/frozen-plan.json --output /external/path/new-results
```

Each invocation receives a fresh cache/temp directory; output stores raw stdout/stderr, real HTML predictions, full official evaluations, explicit failures, and a 3,018-row coverage census. Model arms reject SDK layout/table/inference processing warnings and model-unavailability/fallback tracing. Empty predictions are evaluated as zero quality. Missing/malformed JSON content is an extraction failure. Official success and finite scores in range are required. All 503 documents retain inference and GriTS coverage; the 45 upstream TRM-unsupported records retain their raw scores but are explicitly ineligible for TRM aggregation (270 unsupported comparison cases, 2,748 eligible TRM cases). A complete inference schedule does not establish complete metric eligibility: report evaluator failures, unsupported-TRM flags, and curation subgroups separately. A completed qualification requires every case to be evaluated; failures cause a nonzero exit after saving the complete census.

Do not commit stage bytes, source documents, predictions, or private result artifacts. Do not publish aggregate quality claims until genuine native extraction and independent review have completed.

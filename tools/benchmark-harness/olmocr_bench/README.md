# Official olmOCR qualification

Compare two producer commits using one identical explicit configuration, all 984 PDFs in issue #2331's five groups, and the original olmOCR evaluator. This package contains no downloader, model inference implementation, or replacement metric. PDF bytes, annotations, native outputs, plans and reports stay outside Git.

## Inputs and acquisition

Use the pinned `test_documents` submodule registry and reference-only fetcher:

```sh
python3 test_documents/scripts/fetch_benchmark_upstreams.py --source olmocr-issue2331 --root /external/cache
python3 test_documents/scripts/fetch_benchmark_upstreams.py --source olmocr-issue2331 --root /external/cache --verify-only
```

The data root is `/external/cache/olmocr-issue2331`; set the specification's `registry` to `test_documents/scripts/data/benchmark-upstreams.json`. The old external `stage.json` format is also supported for already verified evidence. Do not add PDFs or raw annotations to the public submodule: the database license does not establish rights to each individual PDF.

Dataset: [allenai/olmOCR-bench at 54a96a6](https://huggingface.co/datasets/allenai/olmOCR-bench/tree/54a96a6fb6a2bd3b297e59869491db4d3625b711), ODC-BY-1.0. Evaluator: [allenai/olmocr at f7cfe4c](https://github.com/allenai/olmocr/tree/f7cfe4c22098b154c76b6ec950d1c0a464eecf8d), Apache-2.0.

| Original group | PDFs | Raw tests |
| --- | ---: | ---: |
| arxiv_math | 522 | 2,927 |
| old_scans_math | 36 | 458 |
| old_scans | 98 | 526 |
| long_tiny_text | 62 | 442 |
| headers_footers | 266 | 760 |
| Total | 984 | 5,113 |

The seven explicit baseline rows stay in `headers_footers`. The official CLI injects another 977 baselines in a separate `baseline` group, yielding 6,090 tests. There are 1,968 native generation cases, one repeat per PDF per producer, and 12,180 official test results. Extra repeats, missing files, duplicate IDs and incomplete counts cannot qualify. No sampling or subset filtering is exposed.

## Environment

Create a new environment with repository-pinned uv/Python; preserve existing frozen environments:

```sh
uv venv /external/olmocr-env --python 3.13.18
uv pip sync --python /external/olmocr-env/bin/python --require-hashes tools/benchmark-harness/olmocr_bench/requirements.lock
uvx pip-audit --disable-pip --require-hashes -r tools/benchmark-harness/olmocr_bench/requirements.lock
```

The minimal lock contains fourteen distributions, with no provider/model extras. Math scoring uses the official local KaTeX renderer and pinned Playwright Chromium. The staged renderer manifest records downloaded browser/KaTeX hashes and its Playwright version; the installed version must match. Browser contexts are offline, Python socket connections are forbidden during scoring, and the official `render_equation(..., use_cache=False)` option disables equation caching through a transparent partial binding. The metric implementation is unchanged. HOME is isolated before upstream imports initialize their cache database.

Expose this package through `PYTHONPATH=<checkout>/tools/benchmark-harness`, then use `/external/olmocr-env/bin/python -m olmocr_bench.runner`.

## Freeze a real comparison

The private specification JSON requires:

- `registry` (omit only for legacy stage format), `config`, `evaluator_root`, `renderer_manifest`, and `requirements_lock`, as absolute paths.
- `argv`: exactly `["extract", "{pdf}", "--config", "{config}", "--format", "json"]`. The CLI envelope's `result.content` is used. Inline JSON, alternate inputs and overriding flags are forbidden.
- `models`: every actual runtime model/traineddata path with `file`, `revision` and `license`. Preserve cache symlink paths so retargeting is detected. Do not invent model identifiers.
- `environment`: `HF_HUB_CACHE`, `TESSDATA_PREFIX`, `ORT_DYLIB_PATH`, `XBERG_ORT_EP="cpu"`, `HF_HUB_OFFLINE="1"`, and positive integer strings for `OMP_NUM_THREADS` and `RAYON_NUM_THREADS`. Optional CPU pins: `OPENBLAS_NUM_THREADS`, `MKL_NUM_THREADS`, `VECLIB_MAXIMUM_THREADS`, `NUMEXPR_NUM_THREADS`.
- Both `baseline` and `candidate`: full `commit`, `executable`, `build_evidence` and `runtime_evidence` paths.

Build evidence must bind `commit`, `executable_sha256`, and the expanded `features` list containing `pdf`, `ocr`, `layout-detection`, and `formula-recognition`. Runtime evidence must bind `executable_sha256`, `config_sha256`, `ort_sha256`, `ort_version`, the exact `model_sha256` list, `execution_provider="cpu"`, `positive_model_pilot=true`, and nonempty `pilot_artifacts` entries with `file`/`sha256`. These must be genuine operator-produced build and positive activation evidence, not fabricated metadata. Verify native layout, formula and OCR activation in the real pilot before freezing.

The same configuration is used for both producers. It must request Markdown, enable OCR, set `use_layout_for_markdown=true`, `layout.strategy="always"`, `layout.formula_model="latex_ocr"`, `layout.acceleration.provider="cpu"`, and positive `concurrency.max_threads`. Both extraction cache and `ocr.tesseract_config.use_cache` must be false, with an explicit nested OCR language. Root selects actual model assets and further settings. Merely setting `layout` does not activate native Markdown layout.

Executables, config, ORT and pilot evidence are copied into read-only content-addressed files beside the plan. Exact runtime model, corpus, evaluator, dependency and renderer identities are frozen. The installed dependency versions must match the audited lock.

```sh
python -m olmocr_bench.runner freeze --data-root /external/cache/olmocr-issue2331 --spec /external/spec.json --plan /external/plan.json
python -m olmocr_bench.runner prepare --plan /external/plan.json --output /external/prepared
python -m olmocr_bench.runner generate --plan /external/plan.json --prepared /external/prepared/ready.json --output /external/generation --timeout <operator-selected-seconds>
python -m olmocr_bench.runner score --plan /external/plan.json --prepared /external/prepared/ready.json --generation /external/generation --output /external/reports
```

`prepare` attempts every official constructor, including real math reference rendering, before native extraction is allowed. A math reference carrying `reference_render.error` is an initialization error even when upstream construction returns normally. All initialization rows/errors remain in the original census. `generate` runs native extraction only when explicitly invoked, with a fresh result cache per case. It saves original stdout/stderr, real Markdown, warnings, latency/RSS from the CLI envelope, and a durable per-case JSONL ledger. Empty predictions, model fallback and execution errors invalidate qualification while remaining available for diagnostic scoring. Artifact hashes are rechecked around native calls and official scoring; repeated hashing is harness overhead. Native timings on a shared machine are contended upper bounds, separate from sweep duration.

`score` calls the original `evaluate_candidate` per original reporting group. Its diagnostic `force=True` keeps missing files as counted errors instead of returning zero-work results; any such error makes qualification fail. No scores/errors are filtered. One repeat means the official per-test majority and pass ratio agree. The final macro score averages the six official group scores, preserving original baseline assignments. A structurally complete run can still have poor quality: no quality threshold is invented here.

## Controls and remaining gates

Run the sixteen standard-library integrity and coverage controls without external data:

```sh
mise run benchmark:validate:olmocr
```

Run all twenty-one controls with the locked environment and pinned external inputs, including the five official-evaluator controls:

```sh
OLMOCR_DATA_ROOT=/external/cache/olmocr-issue2331 \
OLMOCR_EVALUATOR_ROOT=/external/olmocr \
OLMOCR_BROWSER_ROOT=/external/browsers \
OLMOCR_REGISTRY=test_documents/scripts/data/benchmark-upstreams.json \
/external/olmocr-env/bin/python -m unittest discover -s tools/benchmark-harness/tests -p 'test_olmocr_*.py'
```

The Mise task deliberately selects only always-on controls. The discovery command also collects official controls, which explicitly skip unless `OLMOCR_DATA_ROOT`, `OLMOCR_EVALUATOR_ROOT` and `OLMOCR_BROWSER_ROOT` identify the pinned external data/evaluator/browser. Set `OLMOCR_REGISTRY` to exercise registry reuse too. Require twenty-one tests and zero skips when checking the full evaluator environment. Synthetic controls prove checksums and bookkeeping only; they never establish extraction quality.

Root owns actual CLI builds, positive model pilots, full reference initialization, native generation, complete scoring, independent output review and issue closure. Keep #2331 open until its actual extraction defects and complete group results have been addressed.

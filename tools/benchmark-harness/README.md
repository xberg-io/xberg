# Benchmark Harness

Rust CLI tool for comparative benchmarking of the Xberg CLI and 7 reference frameworks. Measures performance
(latency, throughput, memory) and quality (TF1, SF1) against ground truth.

## Overview

The benchmark harness serves two distinct workflows:

- **CI benchmarking** -- automated cross-framework comparison triggered via GitHub Actions, producing aggregated results published as GitHub Releases.
- **Local quality assessment** -- developer-facing pipeline comparison against ground truth for extraction quality triage and regression detection.

## Architecture

```text
CLI (clap)
 |
 +-- run              --> AdapterRegistry --> BenchmarkRunner --> results.json
 |                         |
 |                         +-- SubprocessAdapter (fresh process per invocation)
 |                         +-- NativeAdapter (opt-in Xberg steady-state diagnostics)
 |
 +-- compare          --> ComparisonConfig --> Pipeline extraction --> Quality scoring
 +-- pipeline-benchmark --> 6-path matrix --> TF1/SF1 scoring --> Triage tables
 +-- consolidate      --> Load multi-job results --> Aggregate percentiles
 +-- validate-artifacts --> Enforce raw and aggregated release contracts
 +-- cohort-contract  --> Print the pinned matrix contract for one cohort
 +-- validate-gt      --> Fixture scan --> HTML cleanup --> Integrity report
 +-- survey           --> Corpus-wide extraction stats
 +-- model-benchmark  --> Layout model A/B comparison
 +-- embed-benchmark  --> Embedding throughput measurement
```

### Module Structure

| Module                              | Purpose                                                                                                                    |
| ----------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| `main.rs`                           | CLI entry point (clap subcommands)                                                                                         |
| `adapter.rs`                        | `FrameworkAdapter` trait definition                                                                                        |
| `adapters/`                         | Fresh-process subprocess adapters, opt-in in-process Xberg adapter, and Xberg CLI factories                         |
| `runner.rs`                         | Benchmark orchestration, iteration control, resource monitoring                                                            |
| `quality.rs`                        | Combined TF1/SF1 quality scoring                                                                                           |
| `markdown_quality.rs`               | Markdown block parsing and reading-order helpers                                                                           |
| `structural_sidecar.rs`             | Canonical SF1 typed structural scoring                                                                                      |
| `comparison.rs`                     | Multi-pipeline extraction with quality guardrails                                                                          |
| `pipeline_benchmark.rs`             | 6-path extraction matrix benchmark                                                                                         |
| `corpus.rs`, `fixture.rs`           | Fixture loading, filtering, validation                                                                                     |
| `aggregate.rs`, `consolidate.rs`    | Multi-job result merging and percentile aggregation                                                                        |
| `bench_matrix.rs`, `validate_artifacts.rs` | Pinned cohort matrices and release-contract validation                                                           |
| `output.rs`, `stats.rs`             | Result serialization and statistical analysis                                                                              |
| `validate_gt.rs`                    | Ground truth integrity checks and HTML-to-GFM cleanup                                                                      |
| `monitoring.rs`                     | CPU and memory sampling during benchmarks                                                                                  |
| `profiling.rs`, `profile_report.rs` | Flamegraph generation (requires `profiling` feature)                                                                       |
| `survey.rs`                         | Corpus-wide extraction statistics                                                                                          |
| `model_benchmark.rs`                | Layout model A/B comparison                                                                                                |
| `embed_benchmark.rs`                | Embedding throughput benchmarks                                                                                            |
| `sizes.rs`                          | Framework installation footprint measurement                                                                               |

## Detector evaluation ground truth

PP-DocLayoutV3 qualification is tracked in #2284 as an optional secondary detector;
RT-DETR remains the default. `model-benchmark` runs real detector A/B calls on shared
page renders with actual canonical detections, artifact identities and complete count checks.
It is a timing/count foundation and does not score independent annotations or establish
PP-DocLayoutV3 quality. See [Detector A/B foundation](#detector-ab-foundation) for its exact scope.

The existing OmniDocBench importer preserves each imported document's complete
original page records in a `.layout.json` sidecar alongside the text/Markdown
projections. `fixture.metadata.layout_ground_truth` records the relative sidecar
path and its SHA256. The sidecar binds the PDF SHA256/size, canonical original
annotation hash, page count and block-annotation count. Original category IDs,
polygons, reading order, ignored elements, blank pages, span annotations and
relations remain available for independent detector scoring; the text projection
still applies its existing exclusions. Reimport backfills old fixtures without
rewriting reviewed text. A changed existing PDF, annotation or sidecar identity
fails instead of silently replacing frozen evaluation inputs. Sidecars retain
upstream coordinates; consumers must verify the render-to-annotation transform.

Run `mise run benchmark:validate:layout-annotations` with the root Python development
environment installed. These importer contract tests use controlled synthetic
records; they establish annotation preservation, not model quality or export parity.
No detector evaluator or adoption threshold is implied by a successful import.

Before downloading evaluation data, verify its exact revision and terms.
[OmniDocBench's current dataset card](https://huggingface.co/datasets/opendatalab/OmniDocBench#copyright-statement)
states research-only, noncommercial use; existing download/import commands do not
establish permission for a commercial evaluation. The official
[DocLayNet license](https://github.com/DS4SD/DocLayNet/blob/main/LICENSE) is
CDLA-Permissive-1.0, with attribution and publication conditions. Its predefined
splits and source-document identifiers can support document-level separation,
but training overlap with both detectors must still be audited. Neither dataset
is automatically an uncontaminated held-out set. Keep all downloaded data and
annotations outside tracked fixture output until licensing, provenance and
split ownership have been verified; do not use either model's predictions as truth.

## Quality Scoring

### TF1 (Text F1)

Token-level bag-of-words F1 between extracted text and ground truth.

- Tokenization: lowercase, split on whitespace, keep alphanumeric tokens plus `.` and `,`
- Separate numeric-token F1 for number-heavy documents (financial, scientific)
- Combined score: `quality_score = 0.6 * f1_text + 0.4 * f1_numeric`

### SF1 (Structural F1)

Typed structural comparison between extracted markdown and ground truth markdown.

- **Paragraphs:** content F1 across paragraphs, formulas, images, and figures
- **Headings:** content, heading-level, and ancestor-path agreement
- **Lists:** content, nesting-depth, and ordered/unordered agreement
- **Tables:** GriTS-like cell-grid topology and span agreement
- **Binding edges:** caption and footnote attachment accuracy
- **Reading order:** Longest Increasing Subsequence (LIS) on matched node positions

The five content dimensions are weighted over dimensions present in either
document, then reading order is folded into the single canonical SF1 score.

### Combined Score

When markdown ground truth is available, both metrics are combined:

```text
quality_score = 0.5 * f1_text + 0.2 * f1_numeric + 0.3 * f1_layout
```

## Fixture Format

Fixtures are JSON files organized by format directory under `fixtures/`:

```json
{
  "document": "relative/path/to/file.pdf",
  "file_type": "pdf",
  "file_size": 123456,
  "expected_frameworks": ["xberg", "docling"],
  "metadata": {},
  "ground_truth": {
    "text_file": "relative/path/to/gt.txt",
    "markdown_file": "relative/path/to/gt.md",
    "source": "manual|vision|pdf_text_layer|pandoc|python-docx|..."
  }
}
```

### Ground Truth Coverage

| Format      | Fixtures | With Markdown GT |
| ----------- | -------- | ---------------- |
| PDF         | 187      | 182              |
| Markdown    | 111      | 1                |
| HTML        | 37       | 37               |
| DOCX        | 31       | 27               |
| ODT         | 21       | 21               |
| RTF         | 18       | 18               |
| JPEG        | 15       | 11               |
| CSV         | 13       | 13               |
| XLSX        | 12       | 12               |
| PNG         | 10       | 6                |
| LaTeX       | 9        | 9                |
| Typst       | 9        | 9                |
| EPUB        | 8        | 8                |
| PPTX        | 8        | 7                |
| FB2         | 7        | 7                |
| Org         | 7        | 7                |
| WordPerfect | 7        | 7                |
| DOC         | 6        | 6                |
| DocBook     | 6        | 5                |
| IPynb       | 5        | 5                |
| RST         | 5        | 5                |
| OPML        | 4        | 4                |
| TSV         | 4        | 1                |
| XLS         | 3        | 3                |
| DBK         | 1        | 1                |
| HWPX        | 1        | 1                |
| JATS        | 1        | 1                |
| Keynote     | 1        | 1                |
| Numbers     | 1        | 1                |
| ODP         | 1        | 1                |
| Pages       | 1        | 1                |
| PPT         | 1        | 1                |

Format is determined by each fixture's JSON `file_type` field (see
[`Fixture::file_type`](src/fixture/mod.rs)), not by its directory — most fixtures
live in a directory matching their `file_type`, but several do not (for
example `fixtures/ordinance_2197_scanned.json` sits at the top level and
`fixtures/pdf_medium.json` has no format subdirectory at all). "With Markdown
GT" counts fixtures whose JSON contains `ground_truth.markdown_file`.
Two format labels above merge fixtures with inconsistent `file_type` values
for the same underlying format: **Typst** merges `file_type: "typ"` (8
fixtures) and `file_type: "typst"` (1 fixture, `typst_tables.json`); **WordPerfect**
merges `file_type: "wp"` (3 fixtures) and `file_type: "wpd"` (4 fixtures).

**Total:** 419 fixtures with markdown ground truth across 32 formats.

## Frameworks

### Xberg CLI Pipelines

Xberg is benchmarked through its native CLI pipelines in single-file mode and through the
CLI `batch` entry point for throughput:

The current CI matrix contains `xberg-markdown-baseline`, `xberg-markdown-layout`,
`xberg-markdown-baseline-paddle`, and `xberg-markdown-layout-paddle`. The legacy
`xberg-markdown-paddle-ocr` adapter remains available for explicit compatibility runs,
but is not included in current matrix counts. Append `-batch` in native-batch mode.

### Reference Frameworks (7)

All external tools are benchmarked in single-file mode:

Docling, MinerU, PyMuPDF4LLM, Unstructured, MarkItDown, LiteParse, Tika

Only Docling (`DocumentConverter.convert_all`) and LiteParse (`lit batch-parse`) also
participate in native-batch runs. The harness rejects the other external adapters in
batch mode instead of substituting repeated single-file extraction. Native-batch fixture
cohorts must be homogeneous: either every fixture requires forced OCR or none does.

## Extraction Pipelines

The `compare` and `pipeline-benchmark` commands support these extraction paths:

| Pipeline           | Description                                    |
| ------------------ | ---------------------------------------------- |
| `baseline`         | Native PDF text extraction (no OCR, no layout) |
| `layout`           | Native PDF with layout detection               |
| `tesseract`        | Tesseract OCR with force_ocr (automatic PSM)    |
| `tesseract+layout` | Tesseract OCR with layout detection             |
| `tesseract-vertical-block` | Tesseract PSM 5 (vertical block)          |
| `tesseract-single-block` | Tesseract PSM 6 (single uniform block)       |
| `tesseract-sparse-text` | Tesseract PSM 11 (sparse text)                |
| `paddle-v6-medium` | PP-OCRv6 medium tier with force_ocr            |
| `paddle-v6-medium+layout` | PP-OCRv6 medium tier with layout detection |
| `paddle-v6-small[+layout]` | PP-OCRv6 small tier, optionally with layout |
| `paddle-v6-tiny[+layout]` | PP-OCRv6 tiny tier, optionally with layout |
| `paddle-v5-server[+layout]` | Explicit legacy PP-OCRv5 server tier       |
| `sceptre-ort[+layout]` | Sceptre OCR pinned to the ONNX Runtime inference engine |
| `sceptre-ort-autorotate` | Sceptre OCR (ONNX Runtime) with auto_rotate enabled |
| `docling`          | Vendored Docling reference extraction          |
| `paddleocr-python` | Vendored PaddleOCR Python extraction           |
| `rapidocr`         | Vendored RapidOCR extraction                   |

Paddle quality experiments are opt-in and keep a stable result identity in the pipeline name while fully pinning the
detector/recognition configuration passed to Xberg. The control is `paddle-v6-small+layout+det-side-1024`. Compare it
with `det-side-1536` and `det-side-2048`, then run the one-factor threshold variants
`det-db-thresh-020`, `det-db-box-thresh-035`, `drop-score-030`, and `drop-score-040` using the same prefix. The control
pins the production values `det_db_thresh=0.30`, `det_db_box_thresh=0.50`, and `drop_score=0.50`; experimental presets
are deliberately excluded from default comparison matrices.

```bash
benchmark-harness compare -f fixtures/ --pipelines \
  paddle-v6-small+layout+det-side-1024,\
paddle-v6-small+layout+det-side-1536,\
paddle-v6-small+layout+det-side-2048 \
  --json-output /tmp/paddle-det-side-sweep.json
```

### Local cold-runtime responsiveness

Run the diagnostic from the repository root:

```bash
cargo run --release -p benchmark-harness --bin xberg-runtime-responsiveness -- \
  --input tools/benchmark-harness/fixtures/split/single_paper.pdf \
  --batch-size 1 \
  --config-json '{
    "force_ocr": true,
    "ocr": {
      "backend": "paddleocr",
      "language": ["eng"],
      "auto_rotate": false,
      "paddle_ocr_config": {"model_version": "pp-ocrv6", "model_tier": "medium"}
    }
  }' \
  --max-threads 1 \
  --max-concurrent 1 \
  > /tmp/xberg-runtime-responsiveness.json
```

Launch a fresh process for every repetition so each report includes cold runtime and Paddle
initialization. Keep compiled artifacts, model files, and OS filesystem caches warm between runs.
`extraction_duration_ms` measures the end-to-end PDF render plus Paddle initialization and inference,
not isolated initialization. `stalled_wake_event_count` counts coalesced scheduler wake events that
exceeded the reported threshold; `missed_tick_count` records skipped probe deadlines separately.
The input fingerprint is computed after extraction so it does not warm the measured page cache. It
can still identify a known document by comparison, so treat reports for private inputs as sensitive.

## CLI Reference

### `run` -- CI benchmark execution

Runs benchmarks using framework adapters with configurable iterations, warmup, and sharding.

The default framework matrix reports `cold_process`: each measured invocation starts a fresh
framework process, with downloaded artifacts and host filesystem caches left warm. Opt-in
`xberg-{markdown|plaintext}-{baseline|layout}-steady-state` frameworks report
`warm_in_process`: every document receives discarded warmup iterations in the same Rust process
before its measured iterations. These diagnostic rows remain in detailed output but are excluded
from cross-framework rankings, because the competitor adapters do not yet expose equivalent
persistent request protocols. `--warmup 0` is rejected for a steady-state adapter.
In batch mode, explicitly select
`xberg-{markdown|plaintext}-{baseline|layout}-steady-state-batch`. This lane retains one
`xberg::Engine`, discards complete warmup batches, then records each measured batch invocation.
It is diagnostic-only and remains excluded from cross-framework rankings by its
`warm_in_process` timing regime.
The steady adapter reports `resource_measurement_scope: harness_process_latency_only`: absolute
RSS and CPU belong to the long-lived harness rather than an isolated framework process, so their
raw numeric fields are zeroed as unavailable and excluded from memory/CPU aggregate distributions.
Latency and throughput remain valid. In single-file steady-state mode, the separate cold CLI probe
is attached to exactly one result, avoiding duplicate cold-start samples. The batch lane does not
construct or report that non-equivalent probe.
Every requested input still produces exactly one result row. Xberg batch envelopes are joined by
`metadata.additional.source_index`; indexed failures fill their original slot, while missing,
conflicting, out-of-range, or multi-result inputs fail the performance run. Multi-result envelopes
do not expose enough child-level timing and metadata to collapse them honestly. Repeated
measurements retain their raw invocation metrics and batch sample IDs in `iterations`; summary
fields remain means for compatibility. Warm batch rows omit `cold_start_duration`: the available
cold probe is single-file and therefore not a truthful batch cold-start measurement.
Missing lifecycle or resource-scope metadata deserializes as `unknown`; release validation rejects
it rather than assuming a cold, isolated-process measurement.

```bash
benchmark-harness run \
  -f fixtures/ \
  --cohort cohorts/layout-pdf-fast.json \
  -F xberg-markdown-baseline,docling,liteparse \
  -m batch \
  --max-concurrent 4 \
  --xberg-max-threads 4 \
  -o results/ \
  -i 3 -w 1
```

| Flag                   | Description                                    | Default       |
| ---------------------- | ---------------------------------------------- | ------------- |
| `-f, --fixtures`       | Fixture directory or file                      | required      |
| `--cohort`             | Exact ordered cohort manifest                  | none          |
| `--batch-size`         | Set the maximum native batch size               | cohort value  |
| `-F, --frameworks`     | Comma-separated framework names                | all available |
| `-o, --output`         | Output directory                               | `results`     |
| `-m, --mode`           | `single-file` or `batch`                       | `batch`       |
| `-i, --iterations`     | Benchmark iterations                           | `3`           |
| `-w, --warmup`         | Warmup iterations (discarded)                  | `1`           |
| `-c, --max-concurrent` | Native batch worker limit where supported       | CPU count     |
| `--xberg-max-threads`  | Xberg thread budget in both modes; other frameworks ignore it | automatic / `--max-concurrent` |
| `-t, --timeout`        | Timeout in seconds                             | `1800`        |
| `--ocr`                | Enable OCR                                     | `false`       |
| `--measure-quality`    | Enable quality assessment                      | `false`       |
| `--output-format`      | `markdown` or `plaintext`                        | `markdown`    |
| `--shard`              | Run fixture subset (`INDEX/TOTAL`, e.g. `1/3`) | none          |
| `--model-id`           | `FRAMEWORK=OWNER/REPOSITORY@REVISION#DIGEST`; repeatable | none       |
| `--min-success-rate`   | Minimum success fraction for supported attempts | `1.0`       |

An exact cohort preserves manifest order and rejects duplicates, parent paths, missing
fixtures, and a manifest fixture count that is not divisible by its batch size. Each adapter first
filters unsupported fixtures; its final native batch may therefore contain fewer documents than
the configured batch size. Sharding cannot be combined with exact cohorts or fixed batch sizing.

`--min-success-rate` counts successful results over successful results plus framework-accountable
failures (`FrameworkError`, `Timeout`, `EmptyContent`, and `ZeroOverlap`). Unsupported fixtures are filtered before
execution. Infrastructure failures are reported separately and do not enter this rate; a framework
with no accountable results still fails validation.

`--max-concurrent` and `--xberg-max-threads` can be varied independently for Xberg.
An explicit thread budget applies to both single-file and native batch invocations.
Omitting it preserves Xberg's automatic single-file budget and uses the worker limit
as the native-batch fallback. Docling and LiteParse do not receive this setting.

`results.json` remains backward-compatible. Each run also writes `provenance.json` with the
repository state, ordered fixture/document digests, framework executable identities, model IDs,
timing configuration, worker semantics, and the actual Xberg thread budget reported by the
adapter. It deliberately stores no local absolute paths.

The remote comparative workflow publishes eight independently validated cohorts. The pinned matrix
contract contains these cell counts:

| Cohort   | Required | Optional | Total |
| -------- | -------: | -------: | ----: |
| `native` |       20 |        1 |    21 |
| `ocr`    |       16 |        3 |    19 |
| `office` |        4 |        7 |    11 |
| `markup` |        4 |        7 |    11 |
| `ebook`  |        4 |        2 |     6 |
| `email`  |        4 |        2 |     6 |
| `data`   |        4 |        7 |    11 |
| `images` |        8 |        8 |    16 |

Before publication, `validate-artifacts` matches every present raw artifact to its matrix cell and
checks the pinned source revision, ordered cohort manifest and BLAKE3 digest, batch size, output
format, OCR mode, fixture cardinality, iteration count, and provenance. Required cells must be
present and error-free. Optional cells may be absent; when present, they must satisfy the same
integrity and cardinality checks and may contain only accountable framework failures with diagnostic
messages. Infrastructure failures always fail validation. The command then validates each
consolidated aggregate against the same required/optional contract.

Each release attaches `benchmarks-<cohort>-aggregated.json` and
`benchmarks-<cohort>-metadata.json` for every cohort, plus `benchmarks-index.json`. Use
`cohort-contract --cohort <name>` to print the exact pinned matrix consumed by the workflow.

The capability matrix never fabricates an unsupported format or batch mode. Docling and LiteParse
have native Markdown/plaintext plus single/batch entry points. MarkItDown and PyMuPDF4LLM are
Markdown-only single-file tools; Tika and Unstructured are plaintext-only single-file tools.
MinerU's canonical output is Markdown, so only its
single-file entries are included as optional cells in the native, OCR, and image cohorts.

Local profile runs also write `benchmark-profile.json`. Its `run_identity_sha256` binds the
selected binary, profile configuration, and recursive Git worktree state at execution time,
including executable modes, untracked files, and submodules. Untracked-file selection follows
repository `.gitignore` files only; global and repository-local exclusion configuration is
ignored. Hashing reads index and filesystem state directly without invoking configured Git
filters, fsmonitor hooks, or external diff helpers. This identifies the binary and checkout used
for a run; it does not claim that the binary was built from that checkout.

### `validate-artifacts` and `cohort-contract` -- Release validation

Validate raw artifacts before consolidation, then validate the consolidated aggregate:

```bash
benchmark-harness validate-artifacts \
  --cohort native \
  --artifacts-dir benchmark-artifacts/native \
  --cohort-manifest cohorts/native-pdf-fast-b8.json \
  --fixtures-root fixtures \
  --source-sha "$SOURCE_SHA" \
  --run-id "$RUN_ID" \
  --iterations 3

benchmark-harness validate-artifacts \
  --cohort native \
  --aggregated-file consolidated-output/native/aggregated.json

benchmark-harness cohort-contract --cohort native
```

### `consolidate` -- Merge multi-job results

Combines benchmark results from parallel CI jobs into a single aggregated report with percentiles.

```bash
benchmark-harness consolidate \
  --inputs dir1,dir2,dir3 \
  --output consolidated/
```

### `compare` -- Local pipeline comparison

Compares extraction pipelines on the document corpus with quality scoring and optional guardrails.

```bash
benchmark-harness compare \
  -f fixtures/ \
  --pipelines baseline,layout,paddle \
  --dump-outputs \
  --guardrails
```

| Flag             | Description                                           |
| ---------------- | ----------------------------------------------------- |
| `--pipelines`    | Comma-separated pipeline names                        |
| `--dump-outputs` | Write extraction outputs to `/tmp/xberg_compare/` |
| `--guardrails`   | Fail on quality regressions (non-zero exit)           |
| `--filter`       | Only run documents matching this substring            |
| `--category`     | Only run documents with this exact `metadata.category` |

For example, run the maintained real-ground-truth image OCR corpus in one process:

```bash
benchmark-harness compare -f fixtures/ \
  --category image-ocr-realgt \
  --pipelines paddle-v6-small+layout,tesseract+layout
```

Guardrail contracts may include a `relative_order` array of exact text anchors. The comparison
fails unless every anchor is present in the listed order, allowing focused reading-order checks
that are independent of aggregate SF1 thresholds. The known `681693`
`native+layout+reading-order` sequence is installed when guardrails are loaded or generated,
including for legacy guardrail files without `relative_order`.

### `pipeline-benchmark` -- 6-path extraction matrix

Runs all pipelines across the corpus and produces a ranked triage table.

```bash
benchmark-harness pipeline-benchmark \
  -f fixtures/ \
  --group tables \
  --sort-by sf1 \
  --bottom-n 10 \
  --triage-blocks
```

| Flag              | Description                                                                                  | Default             |
| ----------------- | -------------------------------------------------------------------------------------------- | ------------------- |
| `--paths`         | Comma-separated pipeline names                                                               | all 6 default paths |
| `--doc`           | Filter by document name substrings                                                           | none                |
| `--group`         | Named benchmark group (`tables`, `structure`, `multicolumn`, `text-quality`, `ocr-fallback`) | none                |
| `--sort-by`       | Sort metric: `sf1`, `tf1`, `time`                                                            | `sf1`               |
| `--bottom-n`      | Show only the N worst-performing documents                                                   | none                |
| `--triage-blocks` | Print per-block-type F1 breakdown                                                            | `false`             |
| `--dump-outputs`  | Write outputs to `/tmp/xberg_pipeline/`                                                  | `false`             |
| `--json-output`   | Write JSON results to file                                                                   | none                |
| `--profile-dir`   | Generate per-pipeline flamegraph SVGs                                                        | none                |

### `validate-gt` -- Ground truth validation

Checks ground truth file integrity and optionally fixes HTML artifacts in markdown files.

```bash
benchmark-harness validate-gt -f fixtures/ --fix
```

### `survey` -- Corpus extraction statistics

Produces corpus-wide extraction statistics grouped by file type.

```bash
benchmark-harness survey -f fixtures/ --types pdf,docx
```

### `model-benchmark` -- Layout model A/B comparison

Compares RT-DETR and PP-DocLayoutV3 directly on an exact PDF cohort; table recognition stays separate.

```bash
benchmark-harness model-benchmark -f tools/benchmark-harness/fixtures/ \
  --cohort tools/benchmark-harness/cohorts/layout-pdf-fast.json \
  --model-a rtdetr --model-b pp_doclayout_v3 --max-pages 3 \
  --output /tmp/xberg-layout-detectors.json
benchmark-harness model-benchmark-validate --artifact /tmp/xberg-layout-detectors.json
```

This produces timing/count evidence with `quality_assessed: false`, not model-quality acceptance.

### `embed-benchmark` -- Embedding throughput

Benchmarks embedding throughput across all presets.

```bash
benchmark-harness embed-benchmark
```

### `list-fixtures` -- List loaded fixtures

```bash
benchmark-harness list-fixtures -f fixtures/
```

### `validate` -- Validate fixture JSON

```bash
benchmark-harness validate -f fixtures/
```

### `measure-framework-sizes` -- Installation footprints

Measures disk usage of all framework installations.

```bash
benchmark-harness measure-framework-sizes --output sizes.json
```

## CI Integration

The benchmark suite runs via `.github/workflows/benchmarks.yaml`, triggered by manual `workflow_dispatch`.

### Execution DAG

```text
setup
  Build harness + Xberg CLI + validate ground truth
    |
    v
bench-rust x {pipeline, format, mode, cohort}  (all 8 cohorts)
    |
    v
bench-{external}                              (7 reference frameworks)
    |
    v
aggregate-and-publish                         (validate -> consolidate -> release)

prefetch-mineru-models runs alongside setup and gates the MinerU job and publication.
```

### Platform

- Benchmark jobs: ARM64 hosted runners sized per framework and pipeline
- Aggregation and publication: `ubuntu-24.04-arm`

### Timeouts and Artifacts

- Benchmark-job timeout: 6 hours; MinerU model prefetch timeout: 2 hours
- Build artifacts retained: 7 days
- Result artifacts retained: 30 days
- Final output: eight cohort-specific aggregates and metadata files published as a GitHub Release

## Vendored Baselines

Pre-generated extraction outputs from reference tools are stored in `vendored/` for offline comparison:

| Directory                    | Source                                             |
| ---------------------------- | -------------------------------------------------- |
| `vendored/docling/`          | Docling extraction outputs                         |
| `vendored/paddleocr-python/` | PaddleOCR Python outputs with timing (`.ms` files) |
| `vendored/rapidocr/`         | RapidOCR extraction outputs                        |

Regenerate each backend in its locked, isolated dependency group. Python 3.12 is pinned because PaddlePaddle does not
publish Python 3.14 wheels:

```bash
uv run --locked --isolated --python 3.12 --only-group bench-paddleocr-python \
  python tools/benchmark-harness/scripts/generate_vendored_baselines.py \
  paddleocr-python --force

uv run --locked --isolated --python 3.12 --only-group bench-rapidocr \
  python tools/benchmark-harness/scripts/generate_vendored_baselines.py \
  rapidocr --force

# Limit one backend to fixtures in an exact metadata category
uv run --locked --isolated --python 3.12 --only-group bench-rapidocr \
  python tools/benchmark-harness/scripts/generate_vendored_baselines.py \
  rapidocr --force --category image-ocr-realgt
```

`--only-group` omits the editable Xberg workspace package, while `--isolated` prevents another benchmark environment
from changing the selected OCR runtime.

## Development

```bash
# Build
cargo build -p benchmark-harness

# Run tests
cargo test -p benchmark-harness

# Lint
cargo clippy -p benchmark-harness -- -D warnings

# Local pipeline comparison
cargo run -p benchmark-harness -- compare \
  -f tools/benchmark-harness/fixtures/ \
  --pipelines baseline,layout \
  --dump-outputs

# Validate ground truth
cargo run -p benchmark-harness -- validate-gt \
  -f tools/benchmark-harness/fixtures/

# Full pipeline benchmark with triage
cargo run -p benchmark-harness -- pipeline-benchmark \
  -f tools/benchmark-harness/fixtures/ \
  --sort-by sf1 --bottom-n 20 --triage-blocks

# Corpus survey
cargo run -p benchmark-harness -- survey \
  -f tools/benchmark-harness/fixtures/ --types pdf
```

### Optional Features

| Feature            | Description                               |
| ------------------ | ----------------------------------------- |
| `profiling`        | Enables flamegraph generation via `pprof` |
| `memory-profiling` | Enables jemalloc-based memory profiling   |

Build with features:

```bash
cargo build -p benchmark-harness --features profiling,memory-profiling
```

### Tracing

The harness uses `tracing` with `RUST_LOG` env-filter support. For quality scoring diagnostics:

```bash
RUST_LOG=benchmark_harness::markdown_quality=debug cargo run -p benchmark-harness -- compare ...
```


## Detector A/B foundation

`model-benchmark` runs `LayoutEngine` directly on identical native-rendered PDF pages. It does not
select table-recognition models or count Markdown headings. RT-DETR remains the extraction default;
this benchmark-only comparison does not expose a production model selector.

Use an exact existing cohort and independently establish the selected physical page count before a
qualification run. `--max-pages` is a per-document limit; pages are 1-indexed in artifacts. For example:

```bash
: "${EXPECTED_PAGES:?set the independently verified limited cohort page count}"
cargo run --locked -p benchmark-harness -- model-benchmark --fixtures tools/benchmark-harness/fixtures \
  --cohort tools/benchmark-harness/cohorts/layout-pdf-fast.json \
  --model-a rtdetr --model-b pp_doclayout_v3 --max-pages 3 --iterations 1 \
  --expected-pages "$EXPECTED_PAGES" --output /tmp/xberg-layout-detectors.json
cargo run --locked -p benchmark-harness -- model-benchmark-validate --artifact /tmp/xberg-layout-detectors.json
```

The runtime resolves and checksum-verifies published model files before timing, records their actual
SHA-256/size and cold session-load time, and forces CPU. It requests 150 DPI by default (native safety guards may reduce it) and feeds
the same PNG to both arms, alternating paired order. Each timed call includes PNG decode and the
current detector adapter's preprocessing/inference/postprocessing. Generic Xberg heuristics are
explicitly disabled; the common confidence threshold defaults to 0.5 and is recorded. No OCR, table
recognition or extraction cache participates. Both ORT arms explicitly configure intra-op and inter-op
budgets of one; the intra-op budget is read back from the constructed engine. This is a session
budget, not a claim that the whole process has one OS thread.

Schema 2 also records runtime host CPU/core/OS identity, before/after system load, and actual process-tree
resource samples at 10 ms intervals. Sampling must start before detector work and produce at least two samples with strictly increasing
timestamps, including an observed sample after the work. The sampler is stopped and joined before worker errors are propagated. Baseline and peak
sampled RSS and CPU core-seconds cover the **whole paired process, including loading, PDF rendering and both
resident models**; they cannot be attributed to one model. Timings are flagged as contended using the
existing `SystemLoad` policy, and contended values must not qualify comparative latency budgets.
These observations are still not isolated per-model resource acceptance.

The JSON artifact records document, fixture, cohort and render hashes, physical pages, repetitions,
actual canonical detections/boxes/confidences and per-class counts. Missing documents fail exact
cohort loading; unknown names (including table model names), identical names/artifacts, zero work,
skipped inference, missing/duplicate rows, changed inputs/renders and mismatched counts fail loudly.
A genuinely inferred blank page may have zero detections; it still contributes two inference calls.
A failed inference fails the run rather than becoming a successful zero-count row.

Every artifact sets `quality_assessed: false`. The V3 adapter currently collapses its 25 source
classes to canonical classes, uses Triangle resize and ignores reading-order/mask outputs. This
harness therefore cannot establish native Paddle parity, raw source-class accuracy, learned reading
order, mask quality or downstream extraction quality. The small smoke cohorts are not held-out
quality evaluation. Keep native Paddle/export/pixel parity, independent annotated held-out scoring,
resource budgets and a preregistered non-inferiority decision as separate gates in #2284. No result
from this command changes the RT-DETR default or qualifies V3 for production selection.

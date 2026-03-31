# Benchmark Harness

Rust CLI tool for comparative benchmarking of document extraction across 13 Kreuzberg language bindings and 12 reference frameworks. Measures performance (latency, throughput, memory) and quality (TF1, SF1) against ground truth.

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
 |                         +-- NativeAdapter (in-process Kreuzberg)
 |                         +-- SubprocessAdapter (persistent child process)
 |                         +-- BatchSubprocessAdapter (batch API)
 |
 +-- compare          --> ComparisonConfig --> Pipeline extraction --> Quality scoring
 +-- pipeline-benchmark --> 6-path matrix --> TF1/SF1 scoring --> Triage tables
 +-- consolidate      --> Load multi-job results --> Aggregate percentiles
 +-- validate-gt      --> Fixture scan --> HTML cleanup --> Integrity report
 +-- survey           --> Corpus-wide extraction stats
 +-- model-benchmark  --> Layout model A/B comparison
 +-- embed-benchmark  --> Embedding throughput measurement
```

### Module Structure

| Module | Purpose |
|--------|---------|
| `main.rs` | CLI entry point (clap subcommands) |
| `adapter.rs` | `FrameworkAdapter` trait definition |
| `adapters/` | Adapter implementations: subprocess (persistent/batch), native (in-process), kreuzberg factory functions for all languages |
| `runner.rs` | Benchmark orchestration, iteration control, resource monitoring |
| `quality.rs` | TF1: token-level bag-of-words F1 scoring |
| `markdown_quality.rs` | SF1: structural block-level F1 scoring |
| `comparison.rs` | Multi-pipeline extraction with quality guardrails |
| `pipeline_benchmark.rs` | 6-path extraction matrix benchmark |
| `corpus.rs`, `fixture.rs` | Fixture loading, filtering, validation |
| `aggregate.rs`, `consolidate.rs` | Multi-job result merging and percentile aggregation |
| `output.rs`, `stats.rs` | Result serialization and statistical analysis |
| `validate_gt.rs` | Ground truth integrity checks and HTML-to-GFM cleanup |
| `monitoring.rs` | CPU and memory sampling during benchmarks |
| `profiling.rs`, `profile_report.rs` | Flamegraph generation (requires `profiling` feature) |
| `survey.rs` | Corpus-wide extraction statistics |
| `model_benchmark.rs` | Layout model A/B comparison |
| `embed_benchmark.rs` | Embedding throughput benchmarks |
| `sizes.rs` | Framework installation footprint measurement |

## Quality Scoring

### TF1 (Text F1)

Token-level bag-of-words F1 between extracted text and ground truth.

- Tokenization: lowercase, split on whitespace, keep alphanumeric tokens plus `.` and `,`
- Separate numeric-token F1 for number-heavy documents (financial, scientific)
- Combined score: `quality_score = 0.6 * f1_text + 0.4 * f1_numeric`

### SF1 (Structural F1)

Block-level matching between extracted markdown and ground truth markdown.

- **Block types:** Heading1-6, Paragraph, CodeBlock, Formula, Table, ListItem, Image
- **Type weights:** Headings = 2.0, Code/Formula/Table = 1.5, ListItem = 1.0, Paragraph/Image = 0.5
- **Matching:** Greedy 1:1 with fuzzy cross-type compatibility (e.g., bold paragraph matched to heading gets 0.4 compatibility score)
- **Adjacent concatenation:** Consecutive blocks of the same type are merged before matching
- **Order score:** Longest Increasing Subsequence (LIS) on matched block indices

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
  "expected_frameworks": ["kreuzberg", "docling"],
  "metadata": {},
  "ground_truth": {
    "text_file": "relative/path/to/gt.txt",
    "markdown_file": "relative/path/to/gt.md",
    "source": "manual|vision|pdf_text_layer|pandoc|python-docx|..."
  }
}
```

### Ground Truth Coverage

| Format | Fixtures | With Markdown GT |
|--------|----------|------------------|
| PDF | 159 | 158 |
| HTML | 36 | 36 |
| DOCX | 26 | 26 |
| ODT | 19 | 19 |
| RTF | 17 | 17 |
| XLSX | 12 | 11 |
| CSV | 11 | 11 |
| EPUB | 8 | 8 |
| PPTX | 8 | 8 |
| Org | 6 | 6 |
| DOC | 5 | 5 |
| OPML | 4 | 4 |
| RST | 3 | 3 |
| XLS | 3 | 3 |
| IPynb | 1 | 1 |
| JATS | 1 | 1 |
| LaTeX | 1 | 1 |

**Total:** 318 fixtures with markdown ground truth across 17 formats.

## Frameworks

### Kreuzberg Bindings (13)

Each binding is benchmarked in both single-file (sequential, fair latency) and batch (concurrent, throughput) modes:

Rust, Python, Node.js, Ruby, Go, Java, C#, PHP, Elixir, R, WASM, C, Rust+PaddleOCR

### Reference Frameworks (12)

External document extraction tools benchmarked in single-file mode:

Docling, MarkItDown, Pandoc, Unstructured, Tika, PyMuPDF4LLM, PDFPlumber, MinerU, PyPDF, PDFMiner, PDFtoText, Playa-PDF

## Extraction Pipelines

The `compare` and `pipeline-benchmark` commands support these extraction paths:

| Pipeline | Description |
|----------|-------------|
| `baseline` | Native pdfium text extraction (no OCR, no layout) |
| `layout` | Native pdfium with layout detection |
| `tesseract` | Tesseract OCR with force_ocr |
| `tesseract+layout` | Tesseract OCR with layout detection |
| `paddle` | PaddleOCR mobile tier with force_ocr |
| `paddle+layout` | PaddleOCR mobile tier with layout detection |
| `paddle-server` | PaddleOCR server tier |
| `docling` | Vendored Docling reference extraction |
| `paddleocr-python` | Vendored PaddleOCR Python extraction |
| `rapidocr` | Vendored RapidOCR extraction |

## CLI Reference

### `run` -- CI benchmark execution

Runs benchmarks using framework adapters with configurable iterations, warmup, and sharding.

```bash
benchmark-harness run \
  -f fixtures/ \
  -F kreuzberg-rust,kreuzberg-python \
  -m batch \
  -o results/ \
  -i 3 -w 1
```

| Flag | Description | Default |
|------|-------------|---------|
| `-f, --fixtures` | Fixture directory or file | required |
| `-F, --frameworks` | Comma-separated framework names | all available |
| `-o, --output` | Output directory | `results` |
| `-m, --mode` | `single-file` or `batch` | `batch` |
| `-i, --iterations` | Benchmark iterations | `3` |
| `-w, --warmup` | Warmup iterations (discarded) | `1` |
| `-c, --max-concurrent` | Max concurrent extractions | CPU count |
| `-t, --timeout` | Timeout in seconds | `1800` |
| `--ocr` | Enable OCR | `false` |
| `--measure-quality` | Enable quality assessment | `false` |
| `--shard` | Run fixture subset (`INDEX/TOTAL`, e.g. `1/3`) | none |

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

| Flag | Description |
|------|-------------|
| `--pipelines` | Comma-separated pipeline names |
| `--dump-outputs` | Write extraction outputs to `/tmp/kreuzberg_compare/` |
| `--guardrails` | Fail on quality regressions (non-zero exit) |
| `--filter` | Only run documents matching this substring |

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

| Flag | Description | Default |
|------|-------------|---------|
| `--paths` | Comma-separated pipeline names | all 6 default paths |
| `--doc` | Filter by document name substrings | none |
| `--group` | Named benchmark group (`tables`, `structure`, `multicolumn`, `text-quality`, `ocr-fallback`) | none |
| `--sort-by` | Sort metric: `sf1`, `tf1`, `time` | `sf1` |
| `--bottom-n` | Show only the N worst-performing documents | none |
| `--triage-blocks` | Print per-block-type F1 breakdown | `false` |
| `--dump-outputs` | Write outputs to `/tmp/kreuzberg_pipeline/` | `false` |
| `--json-output` | Write JSON results to file | none |
| `--profile-dir` | Generate per-pipeline flamegraph SVGs | none |

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

Compares two layout model presets across the fixture corpus.

```bash
benchmark-harness model-benchmark -f fixtures/ --model-a fast --model-b accurate
```

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
  Build harness + FFI library + validate ground truth
    |
    v
bench-{language} x {single-file, batch}     (13 Kreuzberg binding jobs)
    |
    v
kreuzberg-gate                                (wait for all Kreuzberg benchmarks)
    |
    v
bench-{external}                              (12 reference framework jobs, some sharded)
    |
    v
aggregate-and-release                         (consolidate all results -> GitHub Release)
```

### Platform

- Primary: `ubuntu-24.04-arm`
- Exception: WASM uses `ubuntu-24.04` (x86) due to V8 ARM compatibility issues

### Timeouts and Artifacts

- Per-job timeout: 6 hours (configurable per-document timeout)
- Build artifacts retained: 7 days
- Result artifacts retained: 30 days
- Final output: aggregated JSON published as a GitHub Release

## Vendored Baselines

Pre-generated extraction outputs from reference tools are stored in `vendored/` for offline comparison:

| Directory | Source |
|-----------|--------|
| `vendored/docling/` | Docling extraction outputs |
| `vendored/paddleocr-python/` | PaddleOCR Python outputs with timing (`.ms` files) |
| `vendored/rapidocr/` | RapidOCR extraction outputs |

Regenerate with:

```bash
python tools/benchmark-harness/scripts/generate_vendored_baselines.py
```

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

| Feature | Description |
|---------|-------------|
| `profiling` | Enables flamegraph generation via `pprof` |
| `memory-profiling` | Enables jemalloc-based memory profiling |

Build with features:

```bash
cargo build -p benchmark-harness --features profiling,memory-profiling
```

### Tracing

The harness uses `tracing` with `RUST_LOG` env-filter support. For quality scoring diagnostics:

```bash
RUST_LOG=benchmark_harness::markdown_quality=debug cargo run -p benchmark-harness -- compare ...
```

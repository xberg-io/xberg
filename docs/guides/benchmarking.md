# Benchmarking

Kreuzberg ships a benchmark harness in `tools/benchmark-harness/` that measures extraction performance across frameworks, file types, and concurrency modes. Use it to compare Kreuzberg against alternatives, catch regressions, and identify bottlenecks with flamegraphs.

---

## Quick Start

Build the harness, then run a benchmark:

```bash title="Terminal"
cargo build -p benchmark-harness --release

task bench:run FRAMEWORK=kreuzberg MODE=single-file
```

That extracts every fixture file one at a time and reports wall time, throughput, and memory usage.

---

## Running Benchmarks

### Via Task

```bash title="Terminal"
task bench:run FRAMEWORK=kreuzberg MODE=single-file
task bench:run FRAMEWORK=kreuzberg MODE=batch
task bench:run FRAMEWORK=kreuzberg MODE=single-file ITERATIONS=5 TIMEOUT=600
```

### Direct Harness

For more control, call the binary directly:

```bash title="Terminal"
./target/release/benchmark-harness \
  run \
  --fixtures tools/benchmark-harness/fixtures \
  --frameworks kreuzberg \
  --output benchmark-results/kreuzberg-single-file \
  --iterations 3 \
  --timeout 900 \
  --mode single-file \
  --max-concurrent 1
```

### Modes

| Mode | What it measures | Default concurrency |
|------|-----------------|-------------------|
| `single-file` | Latency — one file at a time | 1 |
| `batch` | Throughput — multiple files in parallel | 4 |

### Framework Comparison

To see how Kreuzberg stacks up against Tika, Docling, and others:

```bash title="Terminal"
task bench:compare
```

This runs the same fixtures through each framework and produces a side-by-side report.

---

## Fixtures

Benchmark fixtures live in `tools/benchmark-harness/fixtures/`. Each fixture is a real document chosen to exercise a different extraction path:

| Fixture | Tests |
|---------|-------|
| `pdf_small`, `pdf_medium` | PDF parsing and text extraction |
| `docx_simple` | Office document handling |
| `html_simple` | HTML to text conversion |
| `image_table` | OCR + table detection |
| `markdown_technical` | Markdown passthrough |

Add your own fixtures to this directory if you need to benchmark specific document types or edge cases.

---

## Profiling with Flamegraphs

When you need to know *where* time is spent, not just *how much*:

```bash title="Terminal"
task bench:profile FRAMEWORK=kreuzberg MODE=single-file
```

This enables profiling and generates flamegraph SVGs in the `flamegraphs/` directory. Open them in a browser — the interactive SVGs let you zoom into hot call stacks.

For profiling specific fixtures only:

```bash title="Terminal"
ENABLE_PROFILING=true \
PROFILING_FIXTURES=pdf_small,pdf_medium \
./target/release/benchmark-harness \
  run \
  --fixtures tools/benchmark-harness/fixtures \
  --frameworks kreuzberg \
  --output benchmark-results \
  --iterations 1 \
  --mode single-file
```

---

## CI Benchmarks

The benchmark suite runs in CI via `.github/workflows/benchmarks.yaml` (manual trigger). A separate profiling workflow (`.github/workflows/profiling.yaml`) generates flamegraphs against the same fixture set. Neither runs on every PR — they're for intentional performance investigation.

---

## Reading the Results

Benchmark output is JSON, written to the `--output` directory. Each run includes:

- **Wall time** per file and per batch
- **Throughput** in files/second and MB/second
- **Peak memory** usage
- **Per-fixture breakdown** so you can spot which document types are slow

The JSON format makes it straightforward to feed results into dashboards, regression detectors, or comparison scripts.

---

## Rust Microbenchmarks

For lower-level function benchmarks (parsing a specific format, chunking algorithm performance, etc.):

```bash title="Terminal"
cargo bench -p kreuzberg
```

This runs Criterion.rs benchmarks with statistical analysis — means, medians, confidence intervals, and change detection against previous runs.

---

## What to Read Next

- [Performance](../concepts/performance.md) — why Kreuzberg is fast and the architectural decisions behind it
- [Development Workflow](development.md) — building and testing locally

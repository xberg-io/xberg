# Development Workflow

Everything you need to build, test, and debug Kreuzberg locally. This guide assumes you've already followed the [Contributing Guide](../contributing.md) to fork and clone the repo.

---

## The Task Runner

Kreuzberg uses [Task](https://taskfile.dev/) for all build and test workflows. One command to bootstrap everything:

```bash title="Terminal"
task setup
```

That installs all toolchains and dependencies. Safe to re-run anytime — it's idempotent.

### The Pattern

Tasks follow `<language>:<action>`. Once you learn this pattern, the command for any task is predictable:

```bash title="Terminal"
task rust:build           # Build the Rust core
task rust:build:dev       # Debug build (faster compile, no optimizations)
task rust:build:release   # Release build (slow compile, fast binary)
task rust:test            # Run Rust tests
task rust:test:ci         # Same tests, with CI-level diagnostics

task python:build         # Build Python bindings via maturin
task python:test          # Run Python test suite
task node:build           # Build Node.js bindings via napi
task node:test            # Jest tests
```

The same pattern works for every language: `go:build`, `java:test`, `ruby:build`, `csharp:test`, and so on.

### Bulk Operations

```bash title="Terminal"
task build:all            # Build every binding
task test:all             # Test every binding (sequential)
task test:all:parallel    # Test every binding (parallel — faster, noisier output)
task check                # Lint + format check across the whole repo
```

---

## Testing Locally

### Rust

The core lives in `crates/kreuzberg/`. Most changes start here.

```bash title="Terminal"
task rust:test

cargo test -p kreuzberg test_pdf_extraction -- --nocapture

RUST_LOG=debug cargo test -p kreuzberg test_name -- --nocapture
```

### Python

Python bindings are in `packages/python/`. Build first, then test:

```bash title="Terminal"
task python:build:dev
task python:test

cd packages/python
uv run pytest tests/ -k "test_extract" -v
```

The `RUST_LOG` env var works here too — the Rust core logs through Python's stderr:

```bash title="Terminal"
RUST_LOG=debug uv run pytest tests/ -v
```

### Node.js

TypeScript bindings are in `packages/typescript/`:

```bash title="Terminal"
task node:build:dev
task node:test

cd packages/typescript
pnpm test -- --testPathPattern="extract"
```

### Everything Else

Same pattern. Build, then test:

```bash title="Terminal"
task go:build && task go:test
task java:build && task java:test
task csharp:build && task csharp:test
task ruby:build && task ruby:test
task php:build && task php:test
task elixir:build && task elixir:test
task r:build && task r:test
task c:build && task c:test
task wasm:build && task wasm:test
```

---

## E2E Test Suites

End-to-end tests guarantee that every language binding produces identical results for the same document. They live in `e2e/` as shared fixtures — test inputs paired with expected outputs.

### Run E2E Tests

| Language | Directory | Run with |
|----------|-----------|----------|
| Python | `e2e/python/` | `task python:e2e:test` |
| TypeScript / Node.js | `e2e/typescript/` | `task node:e2e:test` |
| Rust | `e2e/rust/` | `task rust:e2e:test` |
| Go | `e2e/go/` | `task go:e2e:test` |
| Java | `e2e/java/` | `task java:e2e:test` |
| .NET | `e2e/csharp/` | `task csharp:e2e:test` |
| Ruby | `e2e/ruby/` | `task ruby:e2e:test` |
| PHP | `e2e/php/` | `task php:e2e:test` |
| R | `e2e/r/` | `task r:e2e:test` |

### Regenerate E2E Tests

When you add a feature that changes extraction behavior, regenerate the affected E2E suites:

```bash title="Terminal"
task python:e2e:generate
task node:e2e:generate
task <lang>:e2e:generate
```

To regenerate and test all suites at once:

```bash title="Terminal"
task e2e:generate:all
task e2e:test:all
```

---

## Benchmarking

Measure extraction performance with the benchmark harness in `tools/benchmark-harness/`. Use it to track regressions, compare against alternatives, and identify bottlenecks with flamegraphs.

### Quick Start

```bash title="Terminal"
task benchmark:run FRAMEWORK=kreuzberg MODE=single-file
task benchmark:run FRAMEWORK=kreuzberg MODE=batch
```

### Common Modes

| Mode | What it measures |
|------|-----------------|
| `single-file` | Latency — one file at a time |
| `batch` | Throughput — multiple files in parallel |

### With Profiling

Generate flamegraphs to see where time is spent:

```bash title="Terminal"
task benchmark:profile FRAMEWORK=kreuzberg MODE=single-file
```

Results appear in the `flamegraphs/` directory as interactive SVGs.

View live benchmark results at <https://kreuzberg.dev/benchmarks>.

---

## Linting and Pre-commit

```bash title="Terminal"
task check              # Full lint + format check (same as CI validate stage)
```

Language-specific:

```bash title="Terminal"
task rust:lint          # clippy + rustfmt
task python:lint        # ruff + mypy
task node:lint          # eslint + typecheck
```

The repo uses pre-commit hooks that enforce conventional commit messages, code formatting, and lint rules. If a commit is rejected, the hook output tells you exactly what to fix.

---

## Working with Documentation

### Building Locally

```bash title="Terminal"
uv sync --group doc
zensical build --clean
zensical serve
```

### How Snippets Work

Code examples in the docs aren't inline — they're pulled from `docs/snippets/` via the `--8<--` include directive. This keeps examples testable and reusable across pages.

```text
docs/snippets/
├── python/           # Python examples
│   ├── api/          #   extract_file, batch_extract, etc.
│   ├── config/       #   ExtractionConfig, OcrConfig, etc.
│   ├── ocr/          #   OCR backends
│   ├── plugins/      #   Plugin implementations
│   ├── mcp/          #   MCP server and client
│   └── utils/        #   Embeddings, chunking, errors
├── rust/             # Rust examples (same layout)
├── typescript/       # TypeScript examples
├── go/, java/, csharp/, ruby/, r/
├── docker/           # Docker commands
├── api_server/       # Server startup examples
└── cli/              # CLI usage
```

When you change a user-facing API, update the matching snippet. When you add a new feature, create a snippet and include it from the relevant doc page.

---

## Debugging

### Rust Panics

```bash title="Terminal"
RUST_BACKTRACE=1 cargo test -p kreuzberg test_name
RUST_BACKTRACE=full cargo test -p kreuzberg test_name
```

### Python FFI Problems

When something goes wrong in the Rust core during a Python call, the error introspection API gives you the details:

```python title="debug_ffi.py"
from kreuzberg import get_last_error_code, get_error_details, get_last_panic_context

details = get_error_details()
print(f"Error: {details['message']}")
print(f"Code: {details['error_code']}")

context = get_last_panic_context()
if context:
    print(f"Panic context: {context}")
```

### Verbose Logging

Crank up the log level to see what the Rust core is doing:

```bash title="Terminal"
RUST_LOG=debug task python:test
RUST_LOG=trace task rust:test
```

---

## CI/CD

CI runs on every push and PR to `main` via `.github/workflows/ci.yaml`. The pipeline has four stages:

1. **Validate** — conventional commits, formatting, clippy
2. **Build** — FFI libraries, Python wheels, Node packages, all bindings
3. **Test** — per-language test suites on Linux, macOS, and Windows
4. **Integration** — Docker build, Docker smoke tests, CLI tests

### Smart Change Detection

CI doesn't rebuild everything on every PR. A `changes` job detects which paths were touched and only runs the relevant build/test jobs. Edit a Python file? Only Python builds and tests run. Touch the Rust core? Everything downstream rebuilds.

### Running CI Checks Locally

Before pushing, you can run the same checks CI runs:

```bash title="Terminal"
task check              # Matches the validate stage
task rust:test:ci       # Rust tests with CI diagnostics
task python:test:ci     # Python tests with CI diagnostics
task test:all:ci        # Everything
```

### Other Workflows

| Workflow | When it runs | What it does |
|----------|-------------|-------------|
| `ci.yaml` | Every push/PR to `main` | The main pipeline |
| `docs.yaml` | Changes to `docs/` or `zensical.toml` | Builds and validates documentation |
| `benchmarks.yaml` | Manual trigger | Runs the full benchmark suite |
| `profiling.yaml` | Manual trigger | Generates flamegraphs |
| `publish.yaml` | Release events | Publishes packages to registries |
| `publish-docker.yaml` | Tags and releases | Builds and pushes Docker images |

---

## Performance

Kreuzberg's core is written in Rust, which enables zero-copy memory handling, SIMD acceleration, and true multi-core parallelism — all at compile time with no garbage collection.

### Why Rust Matters

- **Native compilation:** LLVM optimizes code ahead of time (inlining, vectorization, dead code elimination)
- **Zero-copy strings:** Slicing uses borrowed references, not heap allocations
- **SIMD acceleration:** Whitespace detection and character classification run 15-37x faster than scalar operations
- **No GIL:** True multi-core parallelism across all CPU cores
- **Deterministic memory:** Drop semantics free memory instantly, no GC pauses

### Key Optimizations

- **Batch processing:** 6-10x faster than sequential extraction through work-stealing scheduler
- **Caching:** 85%+ hit rates for repeated files (SQLite-backed, automatic invalidation)
- **Streaming:** Large files processed in 4KB chunks, constant memory regardless of file size
- **Lazy initialization:** Expensive subsystems (Tokio, plugins) initialized on first use only

### Benchmarking Your Workload

Measure with your actual files using the benchmark harness (see [Benchmarking](#benchmarking) section for full instructions). For detailed analysis and live benchmark results, visit <https://kreuzberg.dev/benchmarks>.

---

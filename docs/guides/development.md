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

Tasks follow `<language>:<action>`. Once you internalize this, you can guess the command for anything:

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

## E2E Tests

End-to-end tests exist to guarantee that every language binding produces identical results for the same document. They live in `e2e/` as shared fixtures — test inputs paired with expected outputs.

When you add a feature that changes extraction behavior, regenerate the affected E2E suites:

```bash title="Terminal"
task e2e:python:generate
task e2e:node:generate
task e2e:<lang>:generate
```

---

## Linting and Pre-commit

```bash title="Terminal"
task check              # Full lint + format check (same as CI validate stage)
```

Language-specific:

```bash title="Terminal"
task lint:rust          # clippy + rustfmt
task lint:python        # ruff + mypy
task lint:node          # eslint + typecheck
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

```
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

## What to Read Next

- [Contributing Guide](../contributing.md) — the full contribution workflow from fork to merge
- [Benchmarking](benchmarking.md) — how to measure and profile extraction performance

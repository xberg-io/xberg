# Contributing Guide

Thank you for your interest in contributing to Xberg! This guide covers everything you need — from picking an issue to getting your pull request merged.

---

## First time contributing?

Welcome! Here's how to get started:

1. **Pick an issue** that matches your experience level:
   - [Good first issue](https://github.com/xberg-io/xberg/issues?q=is%3Aissue+is%3Aopen+label%3A%22good+first+issue%22) — small, well-scoped tasks ideal for newcomers
   - [Help wanted](https://github.com/xberg-io/xberg/issues?q=is%3Aissue+is%3Aopen+label%3A%22help+wanted%22) — tasks where we'd especially appreciate community help
2. **Read through the issue** and any existing comments
3. **Leave a comment** letting maintainers know you'd like to work on it
4. **Ask questions** — we're here to help!

Congratulations — that's really all it takes to start contributing! Fork, fix, and open a PR. We keep the process simple so you can focus on what matters: the code.

!!! Tip
Start small. A focused contribution you understand well is more valuable than an ambitious one that stalls.

Want to propose a larger change or new feature? [Open an issue](https://github.com/xberg-io/xberg/issues) to discuss it with maintainers first.

---

## Prerequisites

You only need the toolchains for the areas you plan to work on.

**Required for all contributions:**

- [Git](https://git-scm.com/)
- [Task](https://taskfile.dev/installation/) — our task runner for all build and test workflows
- [Rust](https://rustup.rs/) stable (via `rustup`) — required for core and all bindings. The `wasm32-unknown-unknown` target is configured automatically via `rust-toolchain.toml`

**Required for WASM builds:**

- [WASI SDK](https://github.com/WebAssembly/wasi-sdk/releases) — provides a wasm-capable C/C++ compiler needed by tree-sitter and tesseract. Install to `$HOME/wasi-sdk` or set the `WASI_SDK_PATH` environment variable to your install location

**Language-specific toolchains** (only install what you need):

| Language | Version | Tool                                     |
| -------- | ------- | ---------------------------------------- |
| Python   | 3.10+   | [`uv`](https://docs.astral.sh/uv/)       |
| Node.js  | 20+     | [`pnpm`](https://pnpm.io/)               |
| Ruby     | 3.2+    | `rbenv` or `rvm`                         |
| Go       | 1.26+   | [Official installer](https://go.dev/dl/) |
| Java     | 25+     | JDK (via [sdkman](https://sdkman.io/))   |
| .NET     | 10+     | `dotnet`                                 |
| PHP      | 8.1+    | `composer`                               |
| Elixir   | 1.14+   | `mix` (OTP 25+)                          |
| R        | 4.1+    | [CRAN](https://cran.r-project.org/)      |

For platform-specific build dependencies (compilers, OpenSSL, etc.), see the [Installation guide](getting-started/installation.md).

---

## Development setup

Set up your entire environment with a single command:

```bash title="Terminal"
task setup
```

This installs all toolchains and dependencies. Safe to re-run anytime.

For building individual language bindings, use the namespace pattern:

```bash title="Terminal"
task rust:build
task python:build
task node:build
```

---

## Development workflow

### 1. Fork and clone

Fork the repository on GitHub, then clone your fork:

```bash title="Terminal"
git clone git@github.com:<your-username>/xberg.git
cd xberg
git remote add upstream https://github.com/xberg-io/xberg.git
```

### 2. Create a branch

```bash title="Terminal"
git checkout -b feat/your-feature-name main
```

Use a prefix that matches your change type: `feat/`, `fix/`, `docs/`, `perf/`, `chore/`, `test/`.

### 3. Make your changes

Keep commits small and focused.

### 4. Run checks

```bash title="Terminal"
task check
```

This runs both linting and formatting checks. For language-specific tests:

```bash title="Terminal"
task rust:test
task python:test
task node:test
```

### 5. Commit with conventional messages

We use [Conventional Commits](https://www.conventionalcommits.org/). The pre-commit hook validates this.

```text
feat: add PDF table extraction support
fix: handle empty MIME type in archive entries
docs: update Python API reference for v4.4
perf: parallelize layout inference
```

### 6. Update documentation

When adding user-facing features, add or update pages under `docs/` and reference them in `zensical.toml`.

---

## Issues

### Finding issues

Browse the [issue tracker](https://github.com/xberg-io/xberg/issues) and filter by labels: `good first issue`, `help wanted`, `bug`, or `enhancement`.

### Reporting a bug

Include: what you expected, what happened (with error output), steps to reproduce, your environment (OS, language version, Xberg version), and a minimal sample file if applicable.

### Suggesting improvements

Search for existing issues first. Describe the use case and keep scope focused — break large ideas into smaller, actionable issues.

!!! Tip "Filing great issues"
Be specific: "PDF tables lose column alignment" is better than "PDF parsing is broken." Explain impact and link related issues with `#123`.

---

## Submitting a pull request

### PR checklist

Before opening a PR, verify locally:

<!-- textlint-disable no-todo -->

- [ ] `task check` passes
- [ ] Targeted tests pass
- [ ] Docs updated (if applicable)
- [ ] Commits follow Conventional Commits
<!-- textlint-enable no-todo -->

### Writing a good PR description

Include **what** changed, **why**, and **how** you tested it. Use `Fixes #123` to auto-close related issues.

!!! Tip
Set your PR to **Draft** while it's in progress. Maintainers may leave early comments but won't do a full review until you mark it ready.

### Review and merge

1. **CI runs** — automated builds and tests across platforms
2. **Maintainers review** — code correctness, style, and design
3. **Feedback rounds** — make requested changes and push
4. **Merge** — once approved with all checks passing

**Merge requirements:** all CI checks pass, at least one maintainer approval, no unresolved conversations, branch up to date with `main`.

!!! Info
Don't worry about failing CI on your first PR. Maintainers will help you resolve issues.

---

## CI/CD

Xberg ships six GitHub Actions workflows under `.github/workflows/`. The first two run automatically on contributor PRs; the rest are manual or release-driven and contributors do not need to invoke them.

| Workflow              | Trigger                                     | What it does                                                                                                                                                          |
| --------------------- | ------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ci.yaml`             | Push to `main`, every PR                    | Clippy, fmt, unit + integration tests, type checks for the Python and TypeScript bindings. Runs on `ubuntu-24.04-arm`. This is the canonical "PR is mergeable" check. |
| `docs.yaml`           | Push/PR touching `docs/**`, manual dispatch | Builds the docs site in strict mode, validates `--8<--` snippet includes, runs prose linting, and deploys to GitHub Pages from `main`.                                |
| `publish.yaml`        | Manual dispatch, GitHub release event       | Publishes to PyPI, npm, crates.io, Docker Hub, Homebrew, and other registries. Not run on PRs.                                                                        |
| `publish-docker.yaml` | Manual dispatch, GitHub release event       | Builds and publishes the Xberg Docker images.                                                                                                                     |
| `benchmarks.yaml`     | Manual dispatch only                        | Three-iteration performance run with quality metrics on `ubuntu-24.04-arm`. Used to compare proposed changes against `main`.                                          |
| `profiling.yaml`      | Manual dispatch only                        | Generates flamegraphs for six fixture types (small/medium PDFs, simple DOCX, and others) for performance investigations.                                              |

### Reading workflow failures

!!! Note
Please run checks locally before you open a PR. For example `task check` plus tests for any language bindings you touched (see the [Development Workflow](guides/development.md) guide for common commands). That catches most CI failures faster than iterating on GitHub alone.

Open the failing PR's **Checks** tab and click into the failing job to expand its log. The job name maps directly to the step in `ci.yaml` that failed (for example, `clippy` or `python-test`). To re-run after pushing a fix, GitHub Actions will pick the new commit up automatically; to re-run without a new commit (for flakes), use the **Re-run failed jobs** button at the top right of the workflow run page.

If a check is reporting "expected check missing" rather than failing outright, the workflow file probably wasn't reachable from your branch — rebase on `main` and the check will register on the next push.

---

## Coding standards

- **Rust:** Edition 2024, no `unwrap()` in production paths, document all public items, `SAFETY` comments for `unsafe` blocks
- **Python:** `frozen=True` / `slots=True` dataclasses, function-based pytest, follow Ruff and Mypy rules
- **TypeScript:** Strict types, no `any`, Node.js binding in `crates/xberg-node`
- **Ruby:** No global state outside `Xberg` module, panic-free native bridge, follow RuboCop
- **Go / Java / C#:** Follow standard language conventions and project linters

**Testing:** language-specific tests live in each package; shared E2E behavior belongs in `e2e/` fixtures. When adding features, regenerate with `task e2e:<lang>:generate`.

---

## Community and support

- **Star the repo:** [Give us a star on GitHub](https://github.com/xberg-io/xberg) — it helps others discover Xberg!
- **Discord:** [Join our community](https://discord.gg/xt9WY3GnKR)
- **Issues:** [GitHub Issues](https://github.com/xberg-io/xberg/issues)
- **License:** [MIT License (MIT)](https://github.com/xberg-io/xberg/blob/main/LICENSE)

Thank you for contributing to Xberg!

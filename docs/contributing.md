# Contributing Guide

Thank you for your interest in contributing to Kreuzberg! This guide covers everything you need — from picking an issue to getting your pull request merged.

---

## First time contributing?

Welcome! Here's how to get started:

1. **Pick an issue** that matches your experience level:
      - [Good first issue](https://github.com/kreuzberg-dev/kreuzberg/issues?q=is%3Aissue+is%3Aopen+label%3A%22good+first+issue%22) — small, well-scoped tasks ideal for newcomers
      - [Help wanted](https://github.com/kreuzberg-dev/kreuzberg/issues?q=is%3Aissue+is%3Aopen+label%3A%22help+wanted%22) — tasks where we'd especially appreciate community help
2. **Read through the issue** and any existing comments
3. **Leave a comment** letting maintainers know you'd like to work on it
4. **Ask questions** — we're here to help!

Congratulations — that's really all it takes to start contributing! Fork, fix, and open a PR. We keep the process simple so you can focus on what matters: the code.

!!! tip
    Start small. A focused contribution you understand well is more valuable than an ambitious one that stalls.

Want to propose a larger change or new feature? [Open an issue](https://github.com/kreuzberg-dev/kreuzberg/issues) to discuss it with maintainers first.

---

## Prerequisites

You only need the toolchains for the areas you plan to work on.

**Required for all contributions:**

- [Git](https://git-scm.com/) - 
- [Task](https://taskfile.dev/installation/) — our task runner for all build and test workflows
- Rust stable (via `rustup`) — required for core and all bindings

**Language-specific toolchains** (only install what you need):

| Language | Version | Tool |
|----------|---------|------|
| Python | 3.10+ | `uv` |
| Node.js | 20+ | `pnpm` |
| Ruby | 3.2+ | `rbenv` or `rvm` |
| Go | 1.26+ | Official installer |
| Java | 25+ | JDK |
| .NET | 10+ | `dotnet` |
| PHP | 8.1+ | `composer` |

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
git clone git@github.com:<your-username>/kreuzberg.git
cd kreuzberg
git remote add upstream https://github.com/kreuzberg-dev/kreuzberg.git
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

```
feat: add PDF table extraction support
fix: handle empty MIME type in archive entries
docs: update Python API reference for v4.4
perf: parallelize layout inference
```

### 6. Update documentation

When adding user-facing features, add or update pages under `docs/` and reference them in `mkdocs.yaml`.

---

## Issues

### Finding issues

Browse the [issue tracker](https://github.com/kreuzberg-dev/kreuzberg/issues) and filter by labels: `good first issue`, `help wanted`, `bug`, or `enhancement`.

### Reporting a bug

Include: what you expected, what happened (with error output), steps to reproduce, your environment (OS, language version, Kreuzberg version), and a minimal sample file if applicable.

### Suggesting improvements

Search for existing issues first. Describe the use case and keep scope focused — break large ideas into smaller, actionable issues.

!!! tip "Filing great issues"
    Be specific: "PDF tables lose column alignment" is better than "PDF parsing is broken." Explain impact and link related issues with `#123`.

---

## Submitting a pull request

### PR checklist

Before opening a PR, verify locally:

- [ ] `task check` passes
- [ ] Targeted tests pass
- [ ] Docs updated (if applicable)
- [ ] Commits follow Conventional Commits

### Writing a good PR description

Include **what** changed, **why**, and **how** you tested it. Use `Fixes #123` to auto-close related issues.

!!! tip
    Set your PR to **Draft** while it's in progress. Maintainers may leave early comments but won't do a full review until you mark it ready.

### Review and merge

1. **CI runs** — automated builds and tests across platforms
2. **Maintainers review** — code correctness, style, and design
3. **Feedback rounds** — make requested changes and push
4. **Merge** — once approved with all checks passing

**Merge requirements:** all CI checks pass, at least one maintainer approval, no unresolved conversations, branch up to date with `main`.

!!! info
    Don't worry about failing CI on your first PR. Maintainers will help you resolve issues.

---

## Coding standards

- **Rust:** Edition 2024, no `unwrap()` in production paths, document all public items, `SAFETY` comments for `unsafe` blocks
- **Python:** `frozen=True` / `slots=True` dataclasses, function-based pytest, follow Ruff and Mypy rules
- **TypeScript:** Strict types, no `any`, bindings in `packages/typescript/src`
- **Ruby:** No global state outside `Kreuzberg` module, panic-free native bridge, follow RuboCop
- **Go / Java / C#:** Follow standard language conventions and project linters

**Testing:** language-specific tests live in each package; shared E2E behavior belongs in `e2e/` fixtures. When adding features, regenerate with `task e2e:<lang>:generate`.

---

## Community and support

- **Star the repo:** [Give us a star on GitHub](https://github.com/kreuzberg-dev/kreuzberg) — it helps others discover Kreuzberg!
- **Discord:** [Join our community](https://discord.gg/xt9WY3GnKR)
- **Issues:** [GitHub Issues](https://github.com/kreuzberg-dev/kreuzberg/issues)
- **License:** [MIT](https://github.com/kreuzberg-dev/kreuzberg/blob/main/LICENSE)

Thank you for contributing to Kreuzberg!

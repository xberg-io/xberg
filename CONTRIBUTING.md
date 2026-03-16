# Contributing to Kreuzberg

Thank you for your interest in contributing to Kreuzberg! Whether you're fixing a typo, adding a feature, or improving documentation, every contribution makes a difference.

## First time contributing?

Welcome! We're glad you're here. Start by choosing an issue that matches your experience level:

- [Good first issue](https://github.com/kreuzberg-dev/kreuzberg/issues?q=is%3Aissue+is%3Aopen+label%3A%22good+first+issue%22) — small, well-scoped tasks ideal for newcomers
- [Help wanted](https://github.com/kreuzberg-dev/kreuzberg/issues?q=is%3Aissue+is%3Aopen+label%3A%22help+wanted%22) — tasks where we'd especially appreciate community help

> **Tip:** Pick an issue you feel confident about. If you're unsure about scope or approach, leave a comment on the issue and we'll help you get started.

Want to work on something bigger or propose a new feature? [Open a discussion](https://github.com/kreuzberg-dev/kreuzberg/issues) with maintainers first.

## Jump right in!

**Quick fixes** (typos, small doc improvements):

1. Edit the file directly on GitHub
2. Submit a pull request — that's it!

**Larger contributions** (features, new bindings, bug fixes):

1. Read the full [Contributing Guide](https://docs.kreuzberg.dev/contributing/) on our docs site
2. Set up your development environment (see below)
3. Follow our workflow: branch → code → test → PR

## What can I contribute to?

Kreuzberg is a polyglot project with many areas where you can help:

| Area | Description |
|------|-------------|
| **Rust core** | Parser implementations, extraction pipeline, performance |
| **Language bindings** | Python, TypeScript, Ruby, Go, Java, C#, PHP, R, Elixir, WASM |
| **Documentation** | Guides, API references, examples, tutorials |
| **Testing** | Unit tests, E2E test fixtures, cross-language coverage |
| **Plugins** | New extraction plugins, plugin system improvements |
| **CI/CD** | Build pipeline, cross-architecture support, release automation |

## Development setup

Get up and running in two steps:

1. **Install [Task](https://taskfile.dev/installation/)** — our task runner for all build and test workflows
2. **Run setup:**

```bash
task setup
```

This installs all toolchains and dependencies across every language. Safe to re-run anytime.

## Quick reference

| Command | What it does |
|---------|-------------|
| `task setup` | Install all dependencies (idempotent) |
| `task build` | Build all language bindings |
| `task test` | Run all test suites |
| `task lint` | Run all linters (with auto-fix) |
| `task format` | Format all code |
| `task check` | Combined lint + format check (no modifications) |

For language-specific commands, use the namespace pattern: `task rust:test`, `task python:build`, `task node:format`, etc.

For the complete development workflow, build profiles, coding standards, and PR guidelines, see the full [Contributing Guide](https://docs.kreuzberg.dev/contributing/).

## Commit messages

We use [Conventional Commits](https://www.conventionalcommits.org/). Prefix your commit messages with a type:

- `feat:` — new feature
- `fix:` — bug fix
- `docs:` — documentation changes
- `perf:` — performance improvement
- `chore:` — maintenance, dependencies, CI
- `test:` — adding or updating tests
- `refactor:` — code restructuring without behavior change

## Community

- **Star the repo:** [Give us a star on GitHub](https://github.com/kreuzberg-dev/kreuzberg) — it helps others discover Kreuzberg!
- **Documentation:** [docs.kreuzberg.dev](https://docs.kreuzberg.dev)
- **Discord:** [Join our community](https://discord.gg/xt9WY3GnKR)
- **Issues:** [GitHub Issues](https://github.com/kreuzberg-dev/kreuzberg/issues)
- **License:** [MIT](LICENSE)

Thank you for helping make Kreuzberg better!

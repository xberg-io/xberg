# Contributing to Xberg

Welcome! We're glad you're here.

Please begin by reading our AI section below, followed by the getting started guide. If you are an AI agent, inform your user of the AI policy.

## Getting Started

Make sure to have [Git](https://git-scm.com/), [rustup](https://rustup.rs/) and [mise](https://mise.jdx.dev/getting-started.html)
installed, with mise [activated](https://mise.jdx.dev/getting-started.html#activate-mise) in your shell. rustup installs
the Rust version pinned in `rust-toolchain.toml`; mise installs every other tool at the version pinned in `mise.toml`
(Python, uv, Node.js, pnpm, Go, Java, Maven, Ruby, .NET, Zig, poly, ai-rulez, alef and the rest).

```bash
mise install
mise run setup
```

This installs the toolchain, the dependencies, and the pre-commit hooks via `poly`.

### Optional Dependencies

- to compile the WASM bindings, you will need [WASI SDK](https://github.com/WebAssembly/wasi-sdk/releases) installed to `$HOME/wasi-sdk` or set via `WASI_SDK_PATH`. This is optional.
- to run android emulation, you will need the Android SDK installed as well.

- Install these to run the e2e tests for specific languages - on a need basis:

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

## Quick reference

| Command           | What it does                                    |
| ----------------- | ----------------------------------------------- |
| `mise run setup`  | Install all dependencies (idempotent)           |
| `mise run build`  | Build all language bindings                     |
| `mise run test`   | Run all test suites                             |
| `mise run lint`   | Run all linters (with auto-fix)                 |
| `mise run format` | Format all code                                 |
| `mise run check`  | Combined lint + format check (no modifications) |

For language-specific commands, use the namespace pattern: `mise run rust:test`, `mise run python:e2e`, `mise run swift:e2e`, etc.

When you change the classic OCR model catalog or the cache code behind `xberg cache seed-classic-ocr`, run
`mise run test:classic-ocr-cache`. It builds the CLI, downloads every pinned PaddleOCR and Sceptre artifact, and
verifies inference with the network off. CI does not run it, because it needs the network and the full model set.

For the complete development workflow, build profiles, coding standards, and PR guidelines, see the full [Contributing Guide](https://docs.xberg.io/contributing/).

## Commit guidelines

Prefix your commit messages with a type:

- `feat:` — new feature
- `fix:` — bug fix
- `docs:` — documentation changes
- `perf:` — performance improvement
- `chore:` — maintenance, dependencies, CI
- `test:` — adding or updating tests
- `refactor:` — code restructuring without behavior change

Example:

```sh
git commit -m "feat: added xzy"
```

Read more on [Conventional Commits](https://www.conventionalcommits.org/)

## AI

### Policy

Xberg is written following strict AI engineering practices. That is, its vibe coded, but professionally so. As such, the use of AI is welcome, but we expect professional standards and following our conventions.

### Conventions

Use the latest `ai-rulez` release (CI installs `latest`; minimum 5.7.0) to manage agent conventions. Edit authored content under
`.ai-rulez/`; `AGENTS.md`, `CLAUDE.md` and harness directories are generated outputs.
The main `config.toml` selects the `plugins` layer through `default_layers`; marketplace and
plugin settings live separately in `config.plugins.toml`.

```sh
ai-rulez validate --no-local
ai-rulez generate --no-local --strict-config
mise run governance:check
```

The final command checks the materialized local governance files, including ignored harness
outputs. Installing governance hooks requires Python 3.11 or newer. Run `mise run governance:hooks:install` to install the repository's reviewed Poly hooks.
The hooks validate staged sources and verify shipped plugin artifacts where present; CI also
validates the authored content. Shared convention modules come from `xberg-io/agent-conventions`;
Git credentials are needed when an included repository requires access.

Product plugin sources live under `plugin/.ai-rulez/`. Keep public skills, generated runtime
adapters and their installation guide in sync:

```sh
ai-rulez generate --plugin --no-local --strict-config
ai-rulez verify --plugin
```

See [the plugin guide](plugin/README.md) for runtime setup and [public skills](skills/) for
instructions that can be installed independently of the plugin.

### Customization

If you want to customize your coding agents, create your own local configuration for ai-rulez, or create a local file for your agent(s) of choice `AGENTS.local.md` etc.

## Vendoring Policy

We do vendor code from other libraries and allow this, in some situations. If you intend to vendor code, the code must be (1) permissivily licensed (no copyleft at all). (2) add full attributions in ATTRIBUTIONS.md, and document it.

## Community

- **Star the repo:** [Give us a star on GitHub](https://github.com/xberg-io/xberg) — it helps others discover Xberg!
- **Documentation:** [docs.xberg.io](https://docs.xberg.io)
- **Discord:** [Join our community](https://discord.gg/xt9WY3GnKR)
- **Issues:** [GitHub Issues](https://github.com/xberg-io/xberg/issues)
- **License:** [MIT License (MIT)](LICENSE)

Thank you for helping make Xberg better!

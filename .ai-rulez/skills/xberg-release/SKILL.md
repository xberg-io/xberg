---
type: Playbook
title: Xberg Release
description: "Xberg release specifics on top of the shared release-workflow: version sync to bindings, integrations and the chart (task version:sync), and the CI/Publish dry-run/Benchmarks readiness audit. Use before bumping a version or cutting a release."
x-ai-rulez:
  kind: skill
  id: xberg-release
  metadata:
    name: xberg-release
---

# Xberg release

Repo specifics for the shared `release-workflow` skill: how the version propagates, and the readiness gate before a
push or release.

## Versioning

### Single source of truth

The root `Cargo.toml` `[workspace.package] version` is the one authoritative version (including any
`-rc.N` pre-release suffix). Everything else is derived from it — never hand-edit
a version in a package manifest.

### `task version:sync` runs three steps

`task version:sync` (alias `task versions:sync`) runs, in order
(`.task/tools/version-sync.yml`):

1. `alef sync-versions` — the **alef-managed binding manifests** (their own
   version = core version). Targets are listed in `alef.toml` `[workspace.sync] extra_paths`
   (packages/python, packages/ruby, crates/xberg-node, packages/go, cli-proxy, …).
2. `python3 scripts/sync_integration_versions.py` — the **integrations** under
   `integrations/`, the Helm chart, and `plugin/.ai-rulez/config.toml` `[plugin].version`.
   These are NOT alef-managed, so alef never touches them.
3. `ai-rulez generate --plugin` — regenerates the per-runtime coding-agent plugin bundles
   under `plugin/` from the just-synced config.

Bump/set helpers chain both automatically:
`task version:bump:major|minor|patch`, `task version:set -- <version>`.
`task version:check` prints the Cargo version and runs `sync_integration_versions.py --check`,
failing on integration drift. It does not dry-run `alef sync-versions`.

### Integrations are lockstep with core

The integration packages under `integrations/` are versioned and **published together
with core** across four target families:

- **Python → PyPI**: langchain, llama-index (readers + node-parser), crewai, txtai, surrealdb.
- **Java → Maven Central**: spring-ai (`io.xberg:spring-ai-xberg`).
- **npm → npm**: n8n-nodes-xberg, langchain-xberg, llamaindex-xberg (`@xberg-io/*`).
- **Helm → chart**: `charts/xberg/Chart.yaml` (`version`, `appVersion`, the ArtifactHub image
  tag and prerelease flag) plus `charts/xberg/README.md`'s `--version`.

`scripts/sync_integration_versions.py` sets, for each manifest:

- the package's own `version` — PEP 440 form for pyproject (`1.0.0-rc.32` → `1.0.0rc32`),
  native form for the Maven pom and npm `package.json` (`1.0.0-rc.32`, also valid semver);
- the `xberg` dependency pin, so an integration always requires the core it ships with:
  - pyproject: a **floor** `xberg>=<core>` (PEP 440 form). Naming the rc is deliberate —
    a bare `xberg>=1.0.0` excludes all `1.0.0rcN` pre-releases per PEP 440.
  - pom: `<xberg.version>` (native form).
  - npm `package.json`: an **exact** `@xberg-io/xberg` pin (native/semver form), matching
    the package's own version.

To add a new integration: add its manifest to `VERSION_TARGETS` (own version) and, if it
depends on xberg, `XBERG_DEP_MANIFESTS` in `scripts/sync_integration_versions.py`
(npm `package.json` manifests are collected in `NPM_MANIFESTS`, which feeds both lists).
The llama-index dev aggregator (`integrations/python/llama-index/pyproject.toml`, version
`0.0.0`, unpublished) is dep-only — not a version target.

### Do

- Bump via `task version:bump:*` / `task version:set`, then commit the synced manifests
  together with the Cargo.toml change (atomic).
- Run `task version:check` in CI to guarantee integration manifests never drift from core.

### Don't

- Don't hand-edit a package/manifest version or an integration's `xberg` pin — run the sync.
- Don't add integration manifests to `alef.toml` `[workspace.sync]` — alef would clobber
  their independent-but-derived layout; the dedicated script owns them.

## Readiness

A release-ready commit is one for which CI, the Publish Release dry run, and Benchmarks complete successfully on the
same pushed SHA. A successful older run does not validate newer local commits.

### Audit

1. Check actual workflow failures; distinguish failures from cancellations, skipped jobs, and runs superseded by a
   newer SHA.
2. Map each failed job to a verified local fix or an explicit unresolved blocker. Do not infer that one similarly
   named fix covers a different job.
3. Verify generated Alef output is clean and reproducible with the version pinned in `alef.toml`; generated freshness
   failures remain unresolved until regeneration converges.
4. Keep the Unreleased changelog user-facing and synchronized with docs copies. Do not add CI, test, generator,
   dependency, or internal implementation notes unless users are affected.
5. Run targeted local checks for changed behavior plus the repository's formatting and linting tasks. Do not
   substitute a broad passing suite for a failed job's exact configuration.

### Publish dry-run contracts

- Helm `appVersion`, the container tag, and every `artifacthub.io/images` entry must describe an image that exists
  in GHCR. A chart package that names a missing compatibility tag causes Artifact Hub scan failures even when the
  chart itself published successfully.
- Linux native-library verification must reject unreadable binaries and GLIBC imports above the supported floor.
  A readable native library with no `GLIBC_*` imports is valid; it is not evidence that the scan failed.
- Windows artifact verification must evaluate each DLL independently. Reset per-library status before inspecting
  the next artifact so one result cannot leak into another.
- Elixir musl smoke tests need the NIF and the directory containing its bundled native libraries. Passing only the
  NIF path is insufficient when dependent shared libraries are staged beside it.
- Windows GNU Ruby builds must use the ORT-free feature set because the GNU ABI has no compatible ORT prebuilt.

### Push and dispatch

- Before pushing, `git fetch` and inspect divergence. Do not merge or rebase after committing without the user's
  direction.
- Push one coherent state. Do not repeatedly push while intentional CI is still running merely to get newer diagnostics.
- After the push, dispatch CI, Publish Release with dry-run inputs, and Benchmarks against that pushed SHA. Follow the
  repository's `gh-workflows` conventions for monitoring and retries.
- Retry only after identifying and addressing the cause. Cancellation alone is not a defect; a repeated actual
  failure is.
- Close addressed issues only after the fixing commit is reachable remotely, and state the intended release when
  requested.

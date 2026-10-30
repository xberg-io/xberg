---
type: Playbook
title: Alef Generated Bindings
description: "Edit Alef bindings: output paths, configuration, core regeneration and FFI JSON."
paths:
  - "alef.toml"
  - "packages/**"
  - "crates/xberg-ffi/**"
  - "crates/xberg-py/**"
  - "crates/xberg-node/**"
  - "crates/xberg-php/**"
  - "crates/xberg-jni/**"
  - "crates/xberg-wasm/**"
  - "crates/xberg/src/extractors/**"
  - "crates/xberg/src/plugins/**"
x-ai-rulez:
  kind: skill
  id: alef-generated-bindings
  metadata:
    name: alef-generated-bindings
---

# Alef-Generated Bindings

Xberg specifics on top of the shared Alef workflow skill (generic regenerate/verify/commit loop).

- Check `alef.toml` before touching anything in `packages/*/` or a binding crate.
- Regenerate with `alef all --clean --skip-compile --skip-snippet-validation` (after
  `mise run dart:format:context`); it formats via poly. There is no `alef:generate` or `alef:format` task.
- `mise run alef:verify` runs `alef verify --exit-code`.
- Run `alef docs` directly to generate API documentation.
- The `alef:*` namespace is exactly: `verify`, `build`, `verify:readme-paths`, `verify:ffi-formats`,
  `verify:fields`.

## Freshness check

`mise run alef:verify`. A hand-rolled `git diff` over `packages/` is under-scoped — generated
output also lands in `crates/xberg-py/`, `crates/xberg-php/`, `crates/xberg-jni/`,
`crates/xberg-node/`, `crates/xberg-wasm/`, `crates/xberg-ffi/`, `packages/dart/rust/`,
`packages/swift/rust/`, and `e2e/`.

Each generated file carries an `// alef:hash:` header that `alef verify` recomputes; `.alef/`
(`sources_hash.cache`, `hashes/`) is a gitignored local cache, empty in a new worktree.
Neither makes a generated file safe to hand-fix — a regen restores it. Fix `alef.toml` or the
Rust source, then regenerate and re-verify.

## Key `alef.toml` sections

- `[workspace]`, `[workspace.sync]` (version-sync `extra_paths`), `[workspace.docs]`,
  `[workspace.poly]`, `[workspace.generate]`
- `[[crates]]` — the Rust source crate parsed for type/function extraction, plus
  `[[crates.source_crates]]`
- `[crates.<lang>]` — one table per binding: `python`, `node`, `ruby`, `php`, `elixir`,
  `wasm`, `ffi`, `go`, `java`, `dart`, `kotlin_android`, `jni`, `swift`, `csharp`, `zig`
  (each carrying `exclude_functions`, `target_dep_overrides`, stubs, …)
- `[crates.e2e]` — e2e generation (`output = "e2e"`, `[crates.e2e.call.overrides.<lang>]`)
- `[crates.readme]` — README generation (`template_dir = "templates/readme"`,
  `[crates.readme.languages.<lang>]`)
- `[crates.exclude]`, `[crates.output]`, `[crates.custom_modules]`, `[crates.publish]`

There are no `[crate]`, `[languages.*]`, `[e2e]` or `[readme]` top-level tables, and
`alef.toml` declares **no rename mappings** — the generated Go trait type is
`DocumentExtractor`, unchanged.

Besides the per-language `<lang>:e2e` tasks, this repo has `e2e:verify`.

## Core-side edits that break a regen

- **New extractor struct** — needs `#[cfg_attr(alef, alef(skip))]` on the struct declaration,
  not the impl block. Without it the regen aborts globally and nothing regenerates, leaving
  the tree silently stale. Check this first when a regen dies. (Most modules in
  `crates/xberg/src/extractors/` carry the attribute.)
- **Binding-facing types must not be `#[non_exhaustive]`** — alef generates
  `impl From<Mirror> for xberg::TheType` with a struct literal in ~10 binding crates, and
  `#[non_exhaustive]` forbids that (`E0639`). Derive `Default` for forward-compat instead.
  Keep `#[non_exhaustive]` only on types listed in `[crates.exclude]`.
- **A new enum variant on a binding-facing type** breaks exhaustive matches across the
  generated binding crates.

## Trait return types crossing the generated FFI bridge

The generated `XbergOcrBackendBridge` in `crates/xberg-ffi/src/lib.rs` marshals **every**
trait method's return value through JSON (218 `serde_json::from_str::<xberg::…>` call sites
today) and falls back to `Default::default()` on an uninitialised vtable slot, a failing host
callback, or a null result. Whether a variant carries a payload is irrelevant.

Adding a method to a trait with a generated bridge means its return type needs
`Default + Serialize + Deserialize`. Unit-only enums included.

`crates/xberg-ffi` is linted in CI only by `cargo clippy --workspace` at default features. Verify a
change to it with the scoped `cargo check -p xberg-ffi`. That check rewrites `crates/xberg-ffi/include/xberg.h` and `packages/go/include/xberg.h` to
match whatever feature set you invoked it with, which is usually not the committed one. Revert
that churn rather than committing it.

## Which alef answered

`alef all` runs whatever `alef` is first on `PATH`, not a `cargo run` against a
sibling checkout; check `alef --version` against `alef.toml` `alef_version` before
regenerating. A source fix in a sibling checkout has zero effect until it is installed. Generator fixes ship in order: land upstream → release alef
→ bump the pinned version → regen. Regenerating against a locally modified alef produces a
tree the pinned version cannot reproduce.

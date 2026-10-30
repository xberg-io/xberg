---
status: accepted
date: 2025-12-13
decision-makers: ["Na'aman Hirschfeld"]
---

# Synchronous extraction paths on WASM

## Context and Problem Statement

The core crate's extraction paths were async on Tokio, which the crate always pulled in. 872e1202cf ("feat(wasm): add
WASM bindings implementation", 2025-12-13) added a WebAssembly binding for browsers, Deno and Cloudflare Workers.
Tokio's multi-threaded runtime, `fs` and `process` do not compile on `wasm32`, so the core had to extract on that target
without them.

## Considered Options

* Make Tokio an optional dependency behind a default `tokio-runtime` feature, keep the multi-threaded runtime off
  `wasm32`, and give built-in extractors synchronous internal paths

No alternatives were recorded.

## Decision Outcome

Chosen option: "Make Tokio an optional dependency behind a default `tokio-runtime` feature, keep the multi-threaded
runtime off `wasm32`, and give built-in extractors synchronous internal paths", because it is what let the core build
for the WASM crate. No commit states a further rationale: 872e1202cf made `tokio-runtime = ["dep:tokio"]` and
introduced the `SyncExtractor` trait (crate-private today) in the same change that added the WASM crate.

Today:

* Native builds get Tokio by default. Extraction paths are fully async, CPU-bound work runs in `spawn_blocking`, and
  timeouts use `tokio::time::timeout` (`.ai-rulez/rules/async-and-concurrency.md`).
* The `wasm-target` aggregate does enable `tokio-runtime`, through `layout-tract`, `auto-rotate-tract`,
  `chunking-tokenizers` and `static-embeddings`. What keeps the runtime off `wasm32` is the target gate: Tokio's
  `rt-multi-thread`, `fs` and `process` features are requested only under `cfg(not(target_arch = "wasm32"))`, and
  code that needs them is gated on `cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))`.
* WASM-capable built-in extractors implement `SyncExtractor`. Only these internal paths are synchronous: the WASM
  binding's public API is async.

### Consequences

* Good, because one crate serves both the native async bindings and the browser and edge runtimes.
* Bad, because an extractor that only has an async path is unavailable on WASM, and out-of-crate plugins cannot
  implement `SyncExtractor`.
* Bad, because two gates now decide what compiles, the `tokio-runtime` feature and the `wasm32` target, and code that
  needs the runtime must check both.

### Confirmation

* `crates/xberg/Cargo.toml:116` sets `default = ["tokio-runtime", "simd-utf8"]`, and `:127` defines
  `tokio-runtime = ["dep:tokio"]`. `layout-tract` (`:489`) and `auto-rotate-tract` (`:510`) enable it, as the comment
  at `:655` notes for `wasm-target`.
* `crates/xberg/Cargo.toml:1660` opens the `cfg(not(target_arch = "wasm32"))` dependency table, and `:1666` requests
  Tokio's `process`, `fs` and `rt-multi-thread` there.
* `crates/xberg/src/core/extractor/bytes.rs:88` is one of the
  `cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))` gates.
* `crates/xberg/src/extractors/mod.rs:41` declares `pub(crate) trait SyncExtractor`, and
  `crates/xberg-wasm/src/engine.rs:101` declares the async `extract` the WASM binding exposes.
* The `ci-rust.yaml` job "Narrow feature leg (wasm-target, wasm32)" runs
  `cargo check --locked -p xberg --no-default-features --features wasm-target --target wasm32-unknown-unknown`
  (`.github/workflows/ci-rust.yaml:686`).

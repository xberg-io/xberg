---
status: accepted
date: 2026-06-29
decision-makers: ["Na'aman Hirschfeld"]
---

# Rust-only engine seams for the enterprise boundary

## Context and Problem Statement

Rust callers of `xberg`, xberg-enterprise among them, could reach the extraction pipeline only through the high-level
`extract()`, with no supported point to inject their own caching, progress reporting or other behavior. The libraries
are MIT and xberg-enterprise is proprietary
([org ADR-0003 (Licensing line)](https://github.com/xberg-io/agent-conventions/blob/main/adrs/0003-licensing-line.md)),
so an extension point must keep the generic mechanism in this repository and let the commercial implementation live
outside it. xberg-enterprise's worker layered its own structured-extraction orchestration over `extract()`, so that
mechanism existed in the worker rather than the core; 990fcbe2e8 (2026-06-29) upstreamed the generic part as
`engine::structured`, "ported from the enterprise worker", with "every enterprise policy ... a parameter, not
embedded". The enterprise side of this decision is recorded in xberg-enterprise ADR-0049 (2026-06-27), which its
ADR-0076 supersedes; both are private.

## Considered Options

* Rust-only extension seams under `xberg::engine::seams`, each with an in-core default, kept off the language-binding
  surface
* Add ad hoc knobs to `extract()` only
* Move the worker's structured-extraction orchestrator into the core
* Leave the duplication and optimize only inside the worker

The three alternatives are the ones xberg-enterprise ADR-0049 rejected; its reasons are recorded there.

## Decision Outcome

Chosen option: "Rust-only extension seams under `xberg::engine::seams`, each with an in-core default, kept off the
language-binding surface", because they give Rust callers injection points without changing the default path or the
binding surface: 475f83ad39 records that "The default Engine path is byte-identical; the seams are injection points
consumed by later phases" and that the seams are "Kept off the language-binding surface".

475f83ad39 (2026-06-29) added six seams (`CacheBackend`, `ProgressSink`, `StructuredPolicy`, `PresetResolver`,
`LlmClient`, `ModelProvider`), each with a behavior-preserving default, and kept them out of the bindings: their
files are not in `alef.toml` sources, nothing is re-exported from `lib.rs`, and every seam type is in
`[crates.exclude]`. ddba546dca (2026-08-25) narrowed injection to two: `EngineBuilder` takes only a `CacheBackend` and
a `ProgressSink`, and the other four are standalone Rust primitives, "not Engine configuration"
(`engine/seams/mod.rs`). The generic structured-extraction mechanism in `engine::structured` is described in its
module docs as "MIT-clean".

### Consequences

* Good, because enterprise injects caching and progress into the same engine every binding uses, without a fork of
  the core.
* Good, because the seams stay a Rust-only API, so the binding surface and its compatibility promises do not grow
  with them.
* Bad, because enterprise depends on Rust primitives that no binding exposes, so removing or reshaping one is a
  breaking change for enterprise even when no binding changes. The polyrepo-boundaries skill says to check
  `xberg-enterprise` first.

### Confirmation

* `crates/xberg/src/engine/seams/mod.rs:3` states that cache and progress are injectable and the remaining traits
  are not engine configuration.
* `crates/xberg/src/engine/mod.rs:252` and `:258` are the only `EngineBuilder` setters, `with_cache_backend` and
  `with_progress_sink`, and `:25` documents the MIT-clean structured-extraction mechanism.
* `alef.toml` lists the seam types under `[crates.exclude]` (`:3064`), for example
  `xberg::engine::seams::CacheBackend` at `:3108`.
* The polyrepo-boundaries skill (`.ai-rulez/skills/polyrepo-boundaries/SKILL.md`) assigns product and deployment
  behavior to `xberg-enterprise`.

---
status: accepted
date: 2026-11-05
decision-makers: ["Na'aman Hirschfeld"]
---

# Scope cacheability to one extraction operation

## Context and Problem Statement

OSD model files may change between an outer document-cache snapshot and inner OCR. Comparing only before/after outer snapshots admits an output from a different model after rollback, even when inner OCR detected the change and rejected its own cached result. The public `InternalDocument` bridge cannot carry an additional private field without breaking struct-update callers.

## Considered Options

* Propagate skipped fields through public document carriers.
* Maintain a process-global model epoch.
* Bypass outer caching for every recognition-orientation request.
* Share a private operation-scoped cacheability token and bounded dependency observations.

## Decision Outcome

Chosen option: "Share a private operation-scoped cacheability token and bounded dependency observations", because it preserves public DTO/config shapes and stable-model cache hits while retaining evidence of observed mutation across nested cache layers.

Outer file and Engine single/batch calls share a token within one operation. Independently started operations have separate tokens. Explicit wrappers forward the token into spawned page/batch tasks and native blocking closures; synchronous scope restoration is panic-safe. A bounded canonical-model-root observation map compares the same OSD revision representation at outer snapshots and inner initialization/revalidation. Revision disagreement, read failure, or observation-cap exhaustion permanently rejects cache retention for that operation. Existing per-handle snapshot checks remain in force.

### Consequences

* Good, because a detected change cannot be forgotten by rollback before an outer cache write.
* Good, because no cacheability marker crosses serialization, plugins, metadata, or generated bindings.
* Bad, because new task or blocking handoffs that consume model-dependent OCR must explicitly inherit the private context.
* Bad, because one uncertain dependency conservatively prevents caching the enclosing batch operation.
* Bad, because before/after observations do not detect changes that occur entirely between observations. Canonical-root observations also do not prove equivalence of differently resolved roots or form an atomic transaction with filesystem changes and model recognition.

### Confirmation

`crates/xberg/src/ocr/processor/model_context/outer_cache_tests.rs` exercises actual file and injected Engine single/batch cache publishers with a controlled valid PDF and real inner model-context observations. `crates/xberg/src/core/extractor/cacheability.rs` tests nesting, concurrent isolation, task/blocking inheritance, bounds, and restoration after normal and panic exits. These controls supplement the canonical explicit-sideways qualification task.

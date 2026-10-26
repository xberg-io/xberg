---
type: Decision
title: Api Compatibility
paths:
  - "crates/xberg/src/**"
  - "crates/xberg-ffi/**"
  - "alef.toml"
x-ai-rulez:
  kind: rule
  id: api-compatibility
  metadata:
    priority: high
---

- Compatibility and deprecation policy applies to public APIs. Private and internal code may be removed directly when
  it is no longer needed.
- Breaking public API changes require a major version bump (semantic versioning).
- Document public API changes in `CHANGELOG.md`.
- Keep a deprecated public API for at least one minor version before removing it.
- Public types exposed through bindings must be FFI-friendly or have FFI-compatible equivalents. Rust-only public
  primitives do not need to be exposed by every binding.

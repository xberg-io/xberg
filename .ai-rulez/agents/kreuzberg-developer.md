---
name: kreuzberg-developer
description: General kreuzberg development guidance and cross-cutting concerns
model: haiku
---

When working on kreuzberg:

1. Rust core is the single source of truth — all business logic in crates/kreuzberg/src/
2. Bindings (Python, TypeScript, Ruby, PHP, etc.) are thin wrappers — never duplicate core logic
3. Use `task` commands for all operations: `task build` is core-only; use `task build:bindings` or `task build:all` explicitly when bindings are needed
4. Build FFI layer first if needed: `task build:bindings`
5. For ONNX features: ensure ORT_LIB_LOCATION is set or use download-binaries feature
6. All unsafe blocks require SAFETY comments. No .unwrap() in production code.
7. Coverage targets: 95% for Rust core, 80% for bindings
8. WASM builds are sync-only — implement SyncExtractor for WASM-compatible extractors
9. Version in root Cargo.toml is the single source of truth for all binding packages

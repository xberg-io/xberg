# Draft GitHub issue body

**Title:** `ffi: shrink xberg-ffi to canonical JSON ABI surface`

**Labels:** `area: ffi`, `type: refactor`

**Body:**

## Context

`crates/xberg-ffi/src/lib.rs` currently exposes **1464** `extern "C"` functions, of which ~1300 are per-field getters/setters/clones for ~30 config and response types. Consumers (Go/Java/C#) use **JSON marshaling** (`*_from_json` / `*_to_json` round-trip) and do not need per-field accessors. The accessor exports are dead weight: they bloat the cbindgen header (12.3k lines), expand the C ABI surface, and complicate version evolution of internal types.

The canonical FFI surface is documented in `crates/xberg-ffi/SCOPE.md`.

## Target

~60–80 exports total:

- high-level functions mirroring the public Xberg API (extract, batch, embed, mime, pdf render, plugin lifecycle)
- ~10–15 JSON marshalers (one `_from_json` / `_to_json` pair per type that crosses the boundary)
- ~5–10 `_free` handle-lifecycle functions
- 5 string + error helpers (`free_string`, `last_error_{message,code,clear}`, `version`)
- plugin trampoline vtable plumbing (already in place)

## Missing canonical exports (must be added in this cycle)

- [ ] `xberg_render_pdf_page_to_png`
- [ ] `xberg_register_document_extractor`
- [ ] `xberg_unregister_document_extractor`
- [ ] `xberg_clear_document_extractors`
- [ ] `xberg_last_error_clear`
- [ ] rename `xberg_last_error_context` → `xberg_last_error_message` (keep alias one cycle)
- [ ] (optional) alias `xberg_free_string` as `xberg_string_free` to match `_free` convention

## To delete (~1300 exports)

All per-field getters/setters/clones for AccelerationConfig, ContentFilterConfig, EmailConfig, ExtractionConfig, FileExtractionConfig, BatchBytesItem, BatchFilesItem, TesseractConfig, ServerConfig, ArchiveMetadata, CacheClearResponse, VersionResponse, and ~20 other config/response types.

Each type keeps **only**: `*_from_json`, `*_to_json`, `*_free`, and `*_default` (where useful for bindings constructing handles without a JSON payload).

## Migration plan

1. Audit binding usage (Go, Java, C#): grep for direct calls to per-field accessors. Replace with `_to_json` round-trip into a language-native record.
2. Delete accessors in `lib.rs` and helper modules under `crates/xberg-ffi/src/config/`.
3. Regenerate `include/xberg.h` via `cbindgen`; verify CI freshness check passes.
4. Bump FFI major for the unified extraction ABI cleanup.
5. Run `mise run go:e2e`, `mise run java:e2e` and `mise run csharp:e2e` to verify behavioral parity.

## Risk / scope

Deferred from the v1 API cleanup until binding audits confirm no silent dependency on accessors.

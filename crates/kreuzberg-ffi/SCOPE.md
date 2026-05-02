# kreuzberg-ffi Scope

This document defines the **canonical public C FFI surface** for `kreuzberg-ffi`.
The crate is consumed by Go (cgo), Java (Panama FFM), and C# (P/Invoke) bindings,
which use **JSON marshaling** for typed values rather than per-field accessors.

## Target surface (~70 exports)

The FFI must mirror the canonical 27-function public Kreuzberg API plus the
minimum machinery to make it usable across language boundaries.

### 1. High-level functions (27)

#### Extraction (8)

- `kreuzberg_extract_file`
- `kreuzberg_extract_file_sync`
- `kreuzberg_extract_bytes`
- `kreuzberg_extract_bytes_sync`
- `kreuzberg_batch_extract_files`
- `kreuzberg_batch_extract_files_sync` _(currently missing — must be added)_
- `kreuzberg_batch_extract_bytes`
- `kreuzberg_batch_extract_bytes_sync`

#### Embeddings (4)

- `kreuzberg_embed_texts`
- `kreuzberg_embed_texts_async` _(FFI is synchronous from C ABI perspective; bindings provide async wrappers. May resolve to a single `embed_texts` export.)_
- `kreuzberg_get_embedding_preset`
- `kreuzberg_list_embedding_presets`

#### MIME / format (3)

- `kreuzberg_detect_mime_type`
- `kreuzberg_detect_mime_type_from_bytes`
- `kreuzberg_get_extensions_for_mime`

#### PDF render (1)

- `kreuzberg_render_pdf_page_to_png` _(currently missing — must be added)_

#### Plugin lifecycle (11)

For each of the four plugin axes — `ocr_backend`, `post_processor`, `validator`,
`document_extractor`:

- `kreuzberg_register_<axis>`
- `kreuzberg_unregister_<axis>`
- `kreuzberg_clear_<axis>s`
- `kreuzberg_list_<axis>s`

The `document_extractor` axis is currently missing register/unregister/clear (only
`kreuzberg_list_extractors` exists) — must be added.

### 2. JSON marshaling (~10–15)

Each typed value crossing the FFI boundary that bindings need to round-trip gets
exactly **one** `_from_json` / `_to_json` pair. No per-field getters.

Required pairs:

- `kreuzberg_extraction_config_from_json` / `_to_json` _(present)_
- `kreuzberg_extraction_result_to_json` _(present)_
- `kreuzberg_embedding_config_from_json` / `_to_json`
- `kreuzberg_batch_bytes_item_from_json` / `_to_json`
- `kreuzberg_batch_files_item_from_json` / `_to_json`
- one pair each for any other type bindings serialize across the boundary
  (e.g., `EmbeddingResult`, `PdfRenderConfig`)

### 3. Handle lifecycle (~5–10)

One `_free` per opaque handle type the caller can own:

- `kreuzberg_extraction_config_free`
- `kreuzberg_extraction_result_free`
- `kreuzberg_embedding_config_free`
- `kreuzberg_batch_bytes_item_free`
- `kreuzberg_batch_files_item_free`
- (any other heap-allocated handles produced by `_from_json` constructors)

### 4. String + error handling (5)

- `kreuzberg_free_string` (renames over time → `kreuzberg_string_free` to match
  the rest of the `_free` convention; keep both as alias during deprecation)
- `kreuzberg_last_error_message` _(currently `kreuzberg_last_error_context` — rename)_
- `kreuzberg_last_error_code`
- `kreuzberg_last_error_clear` _(currently missing — must be added)_
- `kreuzberg_version`

### 5. Plugin trampoline plumbing

Vtable + userdata register/unregister functions used by language-implemented
plugins (OCR backends, post-processors, validators, extractors, embedding
backends written in Go/Java/C# and registered into the Rust core). These are
already documented in the typed-bridge design and should remain.

## Out of scope

Per-field getter/setter/clone exports for:

- `AccelerationConfig`, `ContentFilterConfig`, `EmailConfig`, `ExtractionConfig`
- `FileExtractionConfig`, `BatchBytesItem`, `BatchFilesItem`
- `TesseractConfig`, `ServerConfig`, `ArchiveMetadata`, `CacheClearResponse`
- `VersionResponse`, and ~30 other config/response types

These ~1300 exports exist in the current `lib.rs` (1464 total exports) and are
**not** part of the supported FFI surface. Bindings already use JSON
marshaling, so these accessors are dead weight on consumers. They will be
removed in a follow-up cycle (see issue link below).

## Current state

- **Total exports:** 1464
- **Target exports:** ~60–80
- **Reduction needed:** ~1300+ accessors to delete
- **Header file:** `include/kreuzberg.h` (12.3k lines) — will shrink dramatically
  once accessors are removed.

## Migration plan

This document captures the target. The actual deletion is deferred to **v4.10.1**
to de-risk the v4.10 stabilization window:

1. v4.10 (now): document target surface; do **not** delete exports yet; add the
   small number of missing canonical functions above (`batch_extract_files_sync`,
   `render_pdf_page_to_png`, `register/unregister/clear_document_extractors`,
   `last_error_clear`, rename `last_error_context` → `last_error_message`).
2. v4.10.1 (follow-up tracked via GitHub issue):
   - Audit Java/C#/Go bindings for any per-field getter usage; replace with JSON
     round-trip via `*_to_json`.
   - Delete the ~1300 accessor exports from `lib.rs`.
   - Regenerate `include/kreuzberg.h` via `cbindgen`.
   - Bump FFI semver minor (struct layouts unchanged; export removal is a
     soft break for any external consumer outside the kreuzberg-dev polyrepo).

## Why JSON marshaling

Go (cgo), Java (Panama FFM), C# (P/Invoke) all pay similar marshaling overhead
for typed struct walks vs. a single `char*` JSON round-trip. JSON keeps the FFI
ABI surface tiny, removes per-field versioning constraints, and lets bindings
deserialize into language-native records (`record` in Java, `record` in C#,
struct in Go) using existing JSON libraries already on every binding's
critical path. The cost — one allocation + one parse per call — is dwarfed by
extraction work itself.

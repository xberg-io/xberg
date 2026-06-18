---
title: "C API Reference"
---

## C API Reference <span class="version-badge">v5.0.0-rc.23</span>

### Functions

#### kreuzberg_extract_bytes()

Extract content from a byte array.

This is the main entry point for in-memory extraction. It performs the following steps:

1. Validate MIME type
2. Handle legacy format conversion if needed
3. Select appropriate extractor from registry
4. Extract content
5. Run post-processing pipeline

**Returns:**

An `ExtractionResult` containing the extracted content and metadata.

**Errors:**

Returns `KreuzbergError.Validation` if MIME type is invalid.
Returns `KreuzbergError.UnsupportedFormat` if MIME type is not supported.

**Signature:**

```c
KreuzbergExtractionResult* kreuzberg_extract_bytes(const uint8_t* content, const char* mime_type, KreuzbergExtractionConfig config);
```

**Example:**

```c
KreuzbergExtractionResult *result = kreuzberg_extract_bytes((const uint8_t *)"data", "value", NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `content` | `const uint8_t*` | Yes | The byte array to extract |
| `mime_type` | `const char*` | Yes | MIME type of the content |
| `config` | `KreuzbergExtractionConfig` | Yes | Extraction configuration |

**Returns:** `KreuzbergExtractionResult`

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_extract_file()

Extract content from a file.

This is the main entry point for file-based extraction. It performs the following steps:

1. Check cache for existing result (if caching enabled)
2. Detect or validate MIME type
3. Select appropriate extractor from registry
4. Extract content
5. Run post-processing pipeline
6. Store result in cache (if caching enabled)

**Returns:**

An `ExtractionResult` containing the extracted content and metadata.

**Errors:**

Returns `KreuzbergError.Io` if the file doesn't exist (NotFound) or for other file I/O errors.
Returns `KreuzbergError.UnsupportedFormat` if MIME type is not supported.

**Signature:**

```c
KreuzbergExtractionResult* kreuzberg_extract_file(const char* path, const char* mime_type, KreuzbergExtractionConfig config);
```

**Example:**

```c
KreuzbergExtractionResult *result = kreuzberg_extract_file("value", "value", NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `path` | `const char*` | Yes | Path to the file to extract |
| `mime_type` | `const char**` | No | Optional MIME type override. If None, will be auto-detected |
| `config` | `KreuzbergExtractionConfig` | Yes | Extraction configuration |

**Returns:** `KreuzbergExtractionResult`

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_extract_file_sync()

Synchronous wrapper for `extract_file`.

This is a convenience function that blocks the current thread until extraction completes.
For async code, use `extract_file` directly.

Uses the global Tokio runtime for 100x+ performance improvement over creating
a new runtime per call. Always uses the global runtime to avoid nested runtime issues.

This function is only available with the `tokio-runtime` feature. For WASM targets,
use a truly synchronous extraction approach instead.

**Signature:**

```c
KreuzbergExtractionResult* kreuzberg_extract_file_sync(const char* path, const char* mime_type, KreuzbergExtractionConfig config);
```

**Example:**

```c
KreuzbergExtractionResult *result = kreuzberg_extract_file_sync("value", "value", NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `path` | `const char*` | Yes | Path to the file |
| `mime_type` | `const char**` | No | The mime type |
| `config` | `KreuzbergExtractionConfig` | Yes | The configuration options |

**Returns:** `KreuzbergExtractionResult`

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_extract_bytes_sync()

Synchronous wrapper for `extract_bytes`.

Uses the global Tokio runtime for 100x+ performance improvement over creating
a new runtime per call.

With the `tokio-runtime` feature, this blocks the current thread using the global
Tokio runtime. Without it (WASM), this calls a truly synchronous implementation.

**Signature:**

```c
KreuzbergExtractionResult* kreuzberg_extract_bytes_sync(const uint8_t* content, const char* mime_type, KreuzbergExtractionConfig config);
```

**Example:**

```c
KreuzbergExtractionResult *result = kreuzberg_extract_bytes_sync((const uint8_t *)"data", "value", NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `content` | `const uint8_t*` | Yes | The content to process |
| `mime_type` | `const char*` | Yes | The mime type |
| `config` | `KreuzbergExtractionConfig` | Yes | The configuration options |

**Returns:** `KreuzbergExtractionResult`

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_extract_bytes_sync()

Synchronous wrapper for `extract_bytes` (WASM-compatible version).

This is a truly synchronous implementation without tokio runtime dependency.
It calls `extract_bytes_sync_impl()` to perform the extraction.

**Signature:**

```c
KreuzbergExtractionResult* kreuzberg_extract_bytes_sync(const uint8_t* content, const char* mime_type, KreuzbergExtractionConfig config);
```

**Example:**

```c
KreuzbergExtractionResult *result = kreuzberg_extract_bytes_sync((const uint8_t *)"data", "value", NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `content` | `const uint8_t*` | Yes | The content to process |
| `mime_type` | `const char*` | Yes | The mime type |
| `config` | `KreuzbergExtractionConfig` | Yes | The configuration options |

**Returns:** `KreuzbergExtractionResult`

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_batch_extract_files_sync()

Synchronous wrapper for `batch_extract_files`.

Uses the global Tokio runtime for optimal performance.
Only available with `tokio-runtime` (WASM has no filesystem).

**Signature:**

```c
KreuzbergExtractionResult* kreuzberg_batch_extract_files_sync(KreuzbergBatchFileItem* items, KreuzbergExtractionConfig config);
```

**Example:**

```c
KreuzbergExtractionResult* result = kreuzberg_batch_extract_files_sync(NULL, NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `items` | `KreuzbergBatchFileItem*` | Yes | The items |
| `config` | `KreuzbergExtractionConfig` | Yes | The configuration options |

**Returns:** `KreuzbergExtractionResult*`

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_batch_extract_bytes_sync()

Synchronous wrapper for `batch_extract_bytes`.

Uses the global Tokio runtime for optimal performance.
With the `tokio-runtime` feature, this blocks the current thread using the global
Tokio runtime. Without it (WASM), this calls a truly synchronous implementation
that iterates through items and calls `extract_bytes_sync()`.

**Signature:**

```c
KreuzbergExtractionResult* kreuzberg_batch_extract_bytes_sync(KreuzbergBatchBytesItem* items, KreuzbergExtractionConfig config);
```

**Example:**

```c
KreuzbergExtractionResult* result = kreuzberg_batch_extract_bytes_sync(NULL, NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `items` | `KreuzbergBatchBytesItem*` | Yes | The items |
| `config` | `KreuzbergExtractionConfig` | Yes | The configuration options |

**Returns:** `KreuzbergExtractionResult*`

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_batch_extract_bytes_sync()

Synchronous wrapper for `batch_extract_bytes` (WASM-compatible version).

Iterates through items sequentially, applying per-file config overrides.

**Signature:**

```c
KreuzbergExtractionResult* kreuzberg_batch_extract_bytes_sync(KreuzbergBatchBytesItem* items, KreuzbergExtractionConfig config);
```

**Example:**

```c
KreuzbergExtractionResult* result = kreuzberg_batch_extract_bytes_sync(NULL, NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `items` | `KreuzbergBatchBytesItem*` | Yes | The items |
| `config` | `KreuzbergExtractionConfig` | Yes | The configuration options |

**Returns:** `KreuzbergExtractionResult*`

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_batch_extract_files()

Extract content from multiple files concurrently.

This function processes multiple files in parallel, automatically managing
concurrency to prevent resource exhaustion. The concurrency limit can be
configured via `ExtractionConfig.max_concurrent_extractions` or defaults
to `(num_cpus * 1.5).ceil()`.

Each file can optionally specify a `FileExtractionConfig` that overrides specific
fields from the batch-level `config`. Pass `NULL` for a file to use the batch defaults.
Batch-level settings like `max_concurrent_extractions` and `use_cache` are always
taken from the batch-level `config`.

  per-file configuration overrides.

- `config` - Batch-level extraction configuration (provides defaults and batch settings)

**Returns:**

A vector of `ExtractionResult` in the same order as the input items.

**Errors:**

Individual file errors are captured in the result metadata. System errors
(IO, RuntimeError equivalents) will bubble up and fail the entire batch.

Simple usage with no per-file overrides:

Per-file configuration overrides:

**Signature:**

```c
KreuzbergExtractionResult* kreuzberg_batch_extract_files(KreuzbergBatchFileItem* items, KreuzbergExtractionConfig config);
```

**Example:**

```c
KreuzbergExtractionResult* result = kreuzberg_batch_extract_files(NULL, NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `items` | `KreuzbergBatchFileItem*` | Yes | Vector of `BatchFileItem` structs, each containing a path and optional |
| `config` | `KreuzbergExtractionConfig` | Yes | Batch-level extraction configuration (provides defaults and batch settings) |

**Returns:** `KreuzbergExtractionResult*`

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_batch_extract_bytes()

Extract content from multiple byte arrays concurrently.

This function processes multiple byte arrays in parallel, automatically managing
concurrency to prevent resource exhaustion. The concurrency limit can be
configured via `ExtractionConfig.max_concurrent_extractions` or defaults
to `(num_cpus * 1.5).ceil()`.

Each item can optionally specify a `FileExtractionConfig` that overrides specific
fields from the batch-level `config`. Pass `NULL` as the config to use
the batch-level defaults for that item.

  MIME type, and optional per-item configuration overrides.

- `config` - Batch-level extraction configuration

**Returns:**

A vector of `ExtractionResult` in the same order as the input items.

Simple usage with no per-item overrides:

Per-item configuration overrides:

**Signature:**

```c
KreuzbergExtractionResult* kreuzberg_batch_extract_bytes(KreuzbergBatchBytesItem* items, KreuzbergExtractionConfig config);
```

**Example:**

```c
KreuzbergExtractionResult* result = kreuzberg_batch_extract_bytes(NULL, NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `items` | `KreuzbergBatchBytesItem*` | Yes | Vector of `BatchBytesItem` structs, each containing content bytes, |
| `config` | `KreuzbergExtractionConfig` | Yes | Batch-level extraction configuration |

**Returns:** `KreuzbergExtractionResult*`

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_detect_mime_type_from_bytes()

Detect MIME type from raw file bytes.

Uses magic byte signatures to detect file type from content.
Falls back to `infer` crate for comprehensive detection.

For ZIP-based files, inspects contents to distinguish Office Open XML
formats (DOCX, XLSX, PPTX) from plain ZIP archives.

**Returns:**

The detected MIME type string.

**Errors:**

Returns `KreuzbergError.UnsupportedFormat` if MIME type cannot be determined.

**Signature:**

```c
const char* kreuzberg_detect_mime_type_from_bytes(const uint8_t* content);
```

**Example:**

```c
const char *result = kreuzberg_detect_mime_type_from_bytes((const uint8_t *)"data");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `content` | `const uint8_t*` | Yes | Raw file bytes |

**Returns:** `const char*`

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_get_extensions_for_mime()

Get file extensions for a given MIME type.

Returns all known file extensions that map to the specified MIME type.

**Returns:**

A vector of file extensions (without leading dot) for the MIME type.

**Signature:**

```c
const char** kreuzberg_get_extensions_for_mime(const char* mime_type);
```

**Example:**

```c
const char** result = kreuzberg_get_extensions_for_mime("value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `mime_type` | `const char*` | Yes | The MIME type to look up |

**Returns:** `const char**`

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_list_supported_formats()

List all supported document formats.

Returns every file extension Kreuzberg recognizes together with its
corresponding MIME type, derived from the central format registry.
Formats that have no registered file extension (such as source code,
which is detected dynamically) are not included.

The list is sorted alphabetically by file extension.

**Returns:**

A vector of `SupportedFormat` entries sorted by extension.

**Signature:**

```c
KreuzbergSupportedFormat* kreuzberg_list_supported_formats();
```

**Example:**

```c
KreuzbergSupportedFormat* result = kreuzberg_list_supported_formats();
```

**Returns:** `KreuzbergSupportedFormat*`

---

#### kreuzberg_detect_qr_codes()

Detect QR codes in the bytes of an `ExtractedImage`.

`format_hint` is currently unused — the `image` crate auto-detects the
container format from magic bytes — but the parameter is retained so future
backends (e.g. a WebP-via-`webp-decoder` variant) can use it without an API
break.

Returns an empty listtor on any of:

- Empty input.
- Image-decode failure.
- No QR grids detected.
- All detected grids fail to decode.

Successfully decoded QR codes carry their payload, a confidence of `1.0`
(rqrr does not expose per-grid confidence; a successful decode is treated
as high-confidence by convention), and the pixel-space bounding box derived
from the four corner points of the grid.

**Signature:**

```c
KreuzbergQrCode* kreuzberg_detect_qr_codes(const uint8_t* image_bytes, const char* format_hint);
```

**Example:**

```c
KreuzbergQrCode* result = kreuzberg_detect_qr_codes((const uint8_t *)"data", "value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `image_bytes` | `const uint8_t*` | Yes | The image bytes |
| `format_hint` | `const char**` | No | The  format hint |

**Returns:** `KreuzbergQrCode*`

---

#### kreuzberg_clear_embedding_backends()

Clear all embedding backends from the global registry.

Calls `shutdown()` on every registered backend, then empties the registry.

**Errors:**

- Any error returned by a backend's `shutdown()` method. The first error
  encountered stops processing of remaining backends.

**Signature:**

```c
void kreuzberg_clear_embedding_backends();
```

**Example:**

```c
kreuzberg_clear_embedding_backends();
```

**Returns:** No return value.

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_list_embedding_backends()

List the names of all registered embedding backends.

Used by `kreuzberg-cli`, the api/mcp endpoints, and generated language
bindings.

**Signature:**

```c
const char** kreuzberg_list_embedding_backends();
```

**Example:**

```c
const char** result = kreuzberg_list_embedding_backends();
```

**Returns:** `const char**`

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_list_document_extractors()

List names of all registered document extractors.

**Signature:**

```c
const char** kreuzberg_list_document_extractors();
```

**Example:**

```c
const char** result = kreuzberg_list_document_extractors();
```

**Returns:** `const char**`

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_clear_document_extractors()

Clear all document extractors from the global registry.

Calls `shutdown()` on every registered extractor, then empties the registry.

**Errors:**

- Any error returned by an extractor's `shutdown()` method. The first error
  encountered stops processing of remaining extractors.

**Signature:**

```c
void kreuzberg_clear_document_extractors();
```

**Example:**

```c
kreuzberg_clear_document_extractors();
```

**Returns:** No return value.

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_list_ocr_backends()

List all registered OCR backends.

Returns the names of all OCR backends currently registered in the global registry.

**Returns:**

A vector of OCR backend names.

**Signature:**

```c
const char** kreuzberg_list_ocr_backends();
```

**Example:**

```c
const char** result = kreuzberg_list_ocr_backends();
```

**Returns:** `const char**`

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_clear_ocr_backends()

Clear all OCR backends from the global registry.

Removes all OCR backends and calls their `shutdown()` methods.

**Returns:**

- `Ok(())` if all backends were cleared successfully
- `Err(...)` if any shutdown method failed

**Signature:**

```c
void kreuzberg_clear_ocr_backends();
```

**Example:**

```c
kreuzberg_clear_ocr_backends();
```

**Returns:** No return value.

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_register_builtin()

Register every built-in post-processor enabled by the active feature set.

This is the single entry point that callers (including
`register_default_post_processors`) use to populate the global
post-processor registry with the in-tree built-ins. Each submodule's own
`register` function is gated by its feature flag so this aggregate stays
safe to call on any target.

**Signature:**

```c
void kreuzberg_register_builtin();
```

**Example:**

```c
kreuzberg_register_builtin();
```

**Returns:** No return value.

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_list_post_processors()

List all registered post-processor names.

Returns a vector of all post-processor names currently registered in the
global registry.

**Returns:**

- `Ok(const char**)` - Vector of post-processor names
- `Err(...)` if the registry lock is poisoned

**Signature:**

```c
const char** kreuzberg_list_post_processors();
```

**Example:**

```c
const char** result = kreuzberg_list_post_processors();
```

**Returns:** `const char**`

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_clear_post_processors()

Remove all registered post-processors.

**Signature:**

```c
void kreuzberg_clear_post_processors();
```

**Example:**

```c
kreuzberg_clear_post_processors();
```

**Returns:** No return value.

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_list_renderers()

List names of all registered renderers.

**Errors:**

Returns an error if the registry lock is poisoned.

**Signature:**

```c
const char** kreuzberg_list_renderers();
```

**Example:**

```c
const char** result = kreuzberg_list_renderers();
```

**Returns:** `const char**`

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_clear_renderers()

Clear all renderers from the global registry.

Removes every renderer, including the built-in defaults (markdown, html,
djot, plain). After calling this no renderers are registered; re-register
as needed.

**Errors:**

Returns an error if the registry lock is poisoned.

**Signature:**

```c
void kreuzberg_clear_renderers();
```

**Example:**

```c
kreuzberg_clear_renderers();
```

**Returns:** No return value.

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_clear_reranker_backends()

Clear all reranker backends from the global registry.

Calls `shutdown()` on every registered backend, then empties the registry.

**Errors:**

- Any error returned by a backend's `shutdown()` method. The first error
  encountered stops processing of remaining backends.

Since v5.0.

**Signature:**

```c
void kreuzberg_clear_reranker_backends();
```

**Example:**

```c
kreuzberg_clear_reranker_backends();
```

**Returns:** No return value.

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_list_reranker_backends()

List the names of all registered reranker backends.

Used by `kreuzberg-cli`, the api/mcp endpoints, and generated language
bindings.

Since v5.0.

**Signature:**

```c
const char** kreuzberg_list_reranker_backends();
```

**Example:**

```c
const char** result = kreuzberg_list_reranker_backends();
```

**Returns:** `const char**`

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_list_validators()

List names of all registered validators.

**Signature:**

```c
const char** kreuzberg_list_validators();
```

**Example:**

```c
const char** result = kreuzberg_list_validators();
```

**Returns:** `const char**`

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_clear_validators()

Remove all registered validators.

**Signature:**

```c
void kreuzberg_clear_validators();
```

**Example:**

```c
kreuzberg_clear_validators();
```

**Returns:** No return value.

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_classify_pages()

Run page classification against an extraction result.

Mutates `result.page_classifications` with one entry per non-empty page and
appends every LLM call's usage to `result.llm_usage`.

**Errors:**

Returns the first error encountered when rendering the prompt or calling the
LLM. Partially produced classifications are discarded so callers do not see
a half-populated vector.

**Signature:**

```c
void kreuzberg_classify_pages(KreuzbergExtractionResult result, KreuzbergPageClassificationConfig config);
```

**Example:**

```c
kreuzberg_classify_pages(NULL, NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `result` | `KreuzbergExtractionResult` | Yes | The extraction result |
| `config` | `KreuzbergPageClassificationConfig` | Yes | The configuration options |

**Returns:** No return value.

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_classify_text()

Classify a single piece of text without requiring an `ExtractionResult`.

Use this when the caller already has plain text (e.g. a RAG ingest pipeline
receiving documents off a queue) and wants a label list back without
manufacturing extractor-side metadata.

**Errors:**

Same as `classify_pages`: a validation error when `config.labels` is empty,
or any error returned by prompt rendering or the underlying LLM call.

**Signature:**

```c
KreuzbergClassificationLabel* kreuzberg_classify_text(const char* text, KreuzbergPageClassificationConfig config);
```

**Example:**

```c
KreuzbergClassificationLabel* result = kreuzberg_classify_text("value", NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `const char*` | Yes | The text |
| `config` | `KreuzbergPageClassificationConfig` | Yes | The configuration options |

**Returns:** `KreuzbergClassificationLabel*`

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_classify_document()

Classify a single document (as multiple pages or a single text block).

Aggregates classifications across all pages in the provided text, returning
a combined label set that represents the document as a whole.

  using the configured LLM, and results are aggregated.

- `config` - Classification configuration including labels and LLM settings.

**Returns:**

A vector of `ClassificationLabel` entries representing the document's overall classification.

**Errors:**

Returns an error if `config.labels` is empty or if LLM calls fail.

**Signature:**

```c
KreuzbergClassificationLabel* kreuzberg_classify_document(const char** pages, KreuzbergPageClassificationConfig config);
```

**Example:**

```c
KreuzbergClassificationLabel* result = kreuzberg_classify_document(NULL, NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `pages` | `const char**` | Yes | Slice of page texts to classify. Each page is classified independently |
| `config` | `KreuzbergPageClassificationConfig` | Yes | Classification configuration including labels and LLM settings. |

**Returns:** `KreuzbergClassificationLabel*`

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_download_model()

Eagerly download a NER model into the kreuzberg cache.

`name` is a HuggingFace repo id (e.g. `urchade/gliner_multi-v2.1`). The
CLI flag `kreuzberg warm --ner` delegates here.

**Signature:**

```c
const char* kreuzberg_download_model(const char* name, const char* cache_dir);
```

**Example:**

```c
const char* result = kreuzberg_download_model("value", "value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `name` | `const char*` | Yes | The name |
| `cache_dir` | `const char**` | No | The cache dir |

**Returns:** `const char*`

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_download_model()

**Signature:**

```c
const char* kreuzberg_download_model(const char* name, const char* cache_dir);
```

**Example:**

```c
const char* result = kreuzberg_download_model("value", "value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `name` | `const char*` | Yes | The  name |
| `cache_dir` | `const char**` | No | The  cache dir |

**Returns:** `const char*`

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_default_model_name()

Pinned default NER model identifier.

**Signature:**

```c
const char* kreuzberg_default_model_name();
```

**Example:**

```c
const char *result = kreuzberg_default_model_name();
```

**Returns:** `const char*`

---

#### kreuzberg_default_model_name()

**Signature:**

```c
const char* kreuzberg_default_model_name();
```

**Example:**

```c
const char *result = kreuzberg_default_model_name();
```

**Returns:** `const char*`

---

#### kreuzberg_known_models()

All NER models kreuzberg knows about (used by `--all-ner-models`).

**Signature:**

```c
const char** kreuzberg_known_models();
```

**Example:**

```c
const char** result = kreuzberg_known_models();
```

**Returns:** `const char**`

---

#### kreuzberg_known_models()

**Signature:**

```c
const char** kreuzberg_known_models();
```

**Example:**

```c
const char** result = kreuzberg_known_models();
```

**Returns:** `const char**`

---

#### kreuzberg_download_model()

Download a NER model into the kreuzberg cache.

**Signature:**

```c
const char* kreuzberg_download_model(const char* name, const char* cache_dir);
```

**Example:**

```c
const char* result = kreuzberg_download_model("value", "value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `name` | `const char*` | Yes | The  name |
| `cache_dir` | `const char**` | No | The  cache dir |

**Returns:** `const char*`

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_default_model_name()

Default NER model identifier.

**Signature:**

```c
const char* kreuzberg_default_model_name();
```

**Example:**

```c
const char *result = kreuzberg_default_model_name();
```

**Returns:** `const char*`

---

#### kreuzberg_known_models()

All NER models kreuzberg knows about.

**Signature:**

```c
const char** kreuzberg_known_models();
```

**Example:**

```c
const char** result = kreuzberg_known_models();
```

**Returns:** `const char**`

---

#### kreuzberg_redact()

Run pattern redaction (and optional NER-driven redaction) over `result` and
rewrite every textual field. Populates `result.redaction_report`.

**Signature:**

```c
void kreuzberg_redact(KreuzbergExtractionResult result, KreuzbergRedactionConfig config);
```

**Example:**

```c
kreuzberg_redact(NULL, NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `result` | `KreuzbergExtractionResult` | Yes | The extraction result |
| `config` | `KreuzbergRedactionConfig` | Yes | The configuration options |

**Returns:** No return value.

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_summarize()

Score and return the top-N sentences from `text`, joined in original order.

`language` is an ISO 639 (or locale) code used to pick a stopword list;
pass `NULL` (or an unknown code) to fall back to English.
`max_tokens` bounds the summary length by whitespace-separated tokens;
`NULL` falls back to `DEFAULT_MAX_TOKENS`.

**Signature:**

```c
const char** kreuzberg_summarize(const char* text, const char* language, uint32_t max_tokens);
```

**Example:**

```c
const char** result = kreuzberg_summarize("value", "value", 42);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `const char*` | Yes | The text |
| `language` | `const char**` | No | The language |
| `max_tokens` | `uint32_t*` | No | The max tokens |

**Returns:** `const char**`

---

#### kreuzberg_token_count()

Count whitespace-separated tokens (used for token-budget bookkeeping by
callers).

**Signature:**

```c
uint32_t kreuzberg_token_count(const char* text);
```

**Example:**

```c
uint32_t result = kreuzberg_token_count("value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `const char*` | Yes | The text |

**Returns:** `uint32_t`

---

#### kreuzberg_translate_result()

Translate the extraction result in place.

Populates `result.translation` with the translated `content`, optionally the
translated `formatted_content` (when `preserve_markup = true`), and rewrites
every chunk's `content` field. Every LLM call's usage is appended to
`result.llm_usage`.

**Signature:**

```c
void kreuzberg_translate_result(KreuzbergExtractionResult result, KreuzbergTranslationConfig config);
```

**Example:**

```c
kreuzberg_translate_result(NULL, NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `result` | `KreuzbergExtractionResult` | Yes | The extraction result |
| `config` | `KreuzbergTranslationConfig` | Yes | The configuration options |

**Returns:** No return value.

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_chunk_for_rag()

Chunk text for RAG retrieval, ensuring every chunk carries a `heading_path`.

Delegates to `chunk_text` using the caller's config (defaulting to
`ChunkerType.Markdown` when the config uses the default `Text` type, so that
heading hierarchy is resolved).  After chunking, derives
`ChunkMetadata.heading_path` from each chunk's `heading_context`.

  underlying splitter; use `ChunkerType.Markdown` for documents with ATX
  headings.

**Returns:**

A `ChunkingResult` where every chunk's `heading_path` is populated from its
`heading_context` (empty when the chunk is not under any heading).

**Errors:**

Propagates any error from the underlying chunker (e.g. invalid overlap).

**Signature:**

```c
KreuzbergChunkingResult* kreuzberg_chunk_for_rag(const char* text, KreuzbergChunkingConfig config);
```

**Example:**

```c
KreuzbergChunkingResult *result = kreuzberg_chunk_for_rag("value", NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `const char*` | Yes | The text |
| `config` | `KreuzbergChunkingConfig` | Yes | The configuration options |

**Returns:** `KreuzbergChunkingResult`

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_compare()

Compare two extraction results and return a structured diff.

The comparison is purely structural — no I/O, no side effects. All fields
of `ExtractionDiff` are populated according to the provided `DiffOptions`.

**Signature:**

```c
KreuzbergExtractionDiff* kreuzberg_compare(KreuzbergExtractionResult a, KreuzbergExtractionResult b, KreuzbergDiffOptions opts);
```

**Example:**

```c
KreuzbergExtractionDiff *result = kreuzberg_compare(NULL, NULL, NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `a` | `KreuzbergExtractionResult` | Yes | The extraction result |
| `b` | `KreuzbergExtractionResult` | Yes | The extraction result |
| `opts` | `KreuzbergDiffOptions` | Yes | The options to use |

**Returns:** `KreuzbergExtractionDiff`

---

#### kreuzberg_extract_region_with_vlm()

Extract content from a pre-cropped image region using a VLM.

The caller is responsible for cropping the page image to the region's bounding
box before calling this function. The `image_bytes` parameter must contain the
raw bytes of the **cropped** region image (JPEG, PNG, WebP, etc.).

**Returns:**

Extracted Markdown text from the VLM, or an error if the VLM call fails.

**Errors:**

- `Ocr` if the VLM call fails or returns no content.
- `MissingDependency` if the liter-llm client cannot
  be initialised.

**Signature:**

```c
const char* kreuzberg_extract_region_with_vlm(const uint8_t* image_bytes, const char* image_mime, KreuzbergRegionKind region_kind, KreuzbergLlmConfig llm_config, const char* custom_prompt);
```

**Example:**

```c
const char *result = kreuzberg_extract_region_with_vlm((const uint8_t *)"data", "value", (KreuzbergRegionKind){0}, NULL, "value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `image_bytes` | `const uint8_t*` | Yes | The image bytes |
| `image_mime` | `const char*` | Yes | The image mime |
| `region_kind` | `KreuzbergRegionKind` | Yes | The region kind |
| `llm_config` | `KreuzbergLlmConfig` | Yes | The llm config |
| `custom_prompt` | `const char**` | No | The custom prompt |

**Returns:** `const char*`

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_rerank_async()

Rerank documents asynchronously.

Async counterpart to `rerank`. Offloads blocking ONNX inference to a
dedicated blocking thread pool via Tokio's `spawn_blocking`, keeping the
async executor free.

Since v5.0.

**Signature:**

```c
KreuzbergRerankedDocument* kreuzberg_rerank_async(const char* query, const char** documents, KreuzbergRerankerConfig config);
```

**Example:**

```c
KreuzbergRerankedDocument* result = kreuzberg_rerank_async("value", NULL, NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `query` | `const char*` | Yes | The query |
| `documents` | `const char**` | Yes | The documents |
| `config` | `KreuzbergRerankerConfig` | Yes | The configuration options |

**Returns:** `KreuzbergRerankedDocument*`

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_extract_keywords()

Extract keywords from text using the specified algorithm.

This is the unified entry point for keyword extraction. The algorithm
used is determined by `config.algorithm`.

**Returns:**

A vector of keywords sorted by relevance (highest score first).

**Errors:**

Returns an error if:

- The specified algorithm feature is not enabled
- Keyword extraction fails

**Signature:**

```c
KreuzbergKeyword* kreuzberg_extract_keywords(const char* text, KreuzbergKeywordConfig config);
```

**Example:**

```c
KreuzbergKeyword* result = kreuzberg_extract_keywords("value", NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `const char*` | Yes | The text to extract keywords from |
| `config` | `KreuzbergKeywordConfig` | Yes | Keyword extraction configuration |

**Returns:** `KreuzbergKeyword*`

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_analyze_document()

Analyze a document and determine the optimal chunking strategy.

Decision logic (in priority order):

1. If user provides `disable_chunking` → no chunking
2. If user provides page_ranges → use user overrides
3. If chunking is not enabled → no chunking
4. If format doesn't support chunking → no chunking
5. If file is small (below both thresholds) and not force_chunking → no chunking
6. If PDF has a substantial text layer AND !force_ocr → no chunking
   *(only when `heuristics-pdf` feature is enabled; otherwise skipped)*

7. Otherwise → chunk the document

**Errors:**

Returns an error only when the `heuristics-pdf` feature is active and
the PDF text-layer analysis itself returns a hard error.  In all other
cases the function returns a `ChunkingDecision`.

**Signature:**

```c
KreuzbergChunkingDecision* kreuzberg_analyze_document(KreuzbergDocumentMetadata metadata, KreuzbergHeuristicsConfig config, const uint8_t* document_bytes);
```

**Example:**

```c
KreuzbergChunkingDecision *result = kreuzberg_analyze_document(NULL, NULL, (const uint8_t *)"data");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `metadata` | `KreuzbergDocumentMetadata` | Yes | The document metadata |
| `config` | `KreuzbergHeuristicsConfig` | Yes | The configuration options |
| `document_bytes` | `const uint8_t**` | No | The document bytes |

**Returns:** `KreuzbergChunkingDecision`

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_analyze_with_user_chunks()

Analyze a document with user-specified chunk ranges.

Creates a chunk plan based on user-provided page ranges.

**Signature:**

```c
KreuzbergChunkingDecision* kreuzberg_analyze_with_user_chunks(KreuzbergPageRange* user_ranges, uint32_t total_pages, uint64_t size_bytes, KreuzbergHeuristicsConfig config);
```

**Example:**

```c
KreuzbergChunkingDecision *result = kreuzberg_analyze_with_user_chunks(NULL, 42, 42, NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `user_ranges` | `KreuzbergPageRange*` | Yes | The user ranges |
| `total_pages` | `uint32_t` | Yes | The total pages |
| `size_bytes` | `uint64_t` | Yes | The size bytes |
| `config` | `KreuzbergHeuristicsConfig` | Yes | The configuration options |

**Returns:** `KreuzbergChunkingDecision`

---

#### kreuzberg_score_confidence()

Score a `ConfidenceSignals` triple into an `ExtractionConfidence` using
the supplied weights.

When `signals.ocr_aggregate` is `NULL`, the OCR weight folds into
`text_coverage` so the weighted sum still totals 1.0.

**Signature:**

```c
KreuzbergExtractionConfidence* kreuzberg_score_confidence(KreuzbergConfidenceSignals signals, KreuzbergConfidenceWeights weights);
```

**Example:**

```c
KreuzbergExtractionConfidence *result = kreuzberg_score_confidence((KreuzbergConfidenceSignals){0}, (KreuzbergConfidenceWeights){0});
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `signals` | `KreuzbergConfidenceSignals` | Yes | The confidence signals |
| `weights` | `KreuzbergConfidenceWeights` | Yes | The confidence weights |

**Returns:** `KreuzbergExtractionConfidence`

---

#### kreuzberg_check_format_limits()

Decision returned for pre-extraction rejection based on XLSX/PPTX-specific
resource bounds. Returns `Some(reason)` to reject; `NULL` to proceed.

Callers must provide counts from a pre-extraction peek (e.g. parsing
`xl/workbook.xml` for sheet count).

**Signature:**

```c
const char** kreuzberg_check_format_limits(const char* mime_type, uint32_t sheet_count, uint64_t workbook_cells, uint32_t embedded_count, KreuzbergHeuristicsConfig config);
```

**Example:**

```c
const char** result = kreuzberg_check_format_limits("value", 42, 42, 42, NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `mime_type` | `const char*` | Yes | The mime type |
| `sheet_count` | `uint32_t*` | No | The sheet count |
| `workbook_cells` | `uint64_t*` | No | The workbook cells |
| `embedded_count` | `uint32_t*` | No | The embedded count |
| `config` | `KreuzbergHeuristicsConfig` | Yes | The configuration options |

**Returns:** `const char**`

---

#### kreuzberg_boundaries_from_extraction_result()

Derive document boundaries from an already-produced `ExtractionResult`.

Builds a `MultidocInput` from `result.pages` (one `PageSignals` per
`PageContent` entry), then delegates to `detect_boundaries`.

### Fallback behaviour

- If `result.pages` is `NULL` or empty the whole document is treated as a
  single document: returns `[Start(1), End(1)]`, matching the contract of
  `detect_boundaries` for a one-page input.

### Text density

`PageContent` does not carry a pre-computed density score.
This function approximates density as
`non_whitespace_chars / total_chars` (clamped to `[0.0, 1.0]`), which is a
reasonable proxy for how text-dense a page is relative to itself.  Pass a
custom `MultidocInput` to `detect_boundaries` directly when you need a
higher-fidelity density measurement (e.g. chars-per-pt² from a PDF extractor).

**Signature:**

```c
KreuzbergDocumentBoundary* kreuzberg_boundaries_from_extraction_result(KreuzbergExtractionResult result, KreuzbergMultidocThresholds thresholds);
```

**Example:**

```c
KreuzbergDocumentBoundary* result = kreuzberg_boundaries_from_extraction_result(NULL, NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `result` | `KreuzbergExtractionResult` | Yes | The extraction result |
| `thresholds` | `KreuzbergMultidocThresholds` | Yes | The multidoc thresholds |

**Returns:** `KreuzbergDocumentBoundary*`

---

#### kreuzberg_detect_boundaries()

Detect document boundaries in a multi-document PDF.

Returns a list of detected boundaries, always including implicit boundaries
at start (page 1) and end (page_count).  Boundaries are returned in ascending
order of `start_page`.

**Returns:**

Ordered list of document boundaries.

**Signature:**

```c
KreuzbergDocumentBoundary* kreuzberg_detect_boundaries(KreuzbergMultidocInput input, KreuzbergMultidocThresholds thresholds);
```

**Example:**

```c
KreuzbergDocumentBoundary* result = kreuzberg_detect_boundaries(NULL, NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `input` | `KreuzbergMultidocInput` | Yes | Page signals for the PDF |
| `thresholds` | `KreuzbergMultidocThresholds` | Yes | Detection thresholds |

**Returns:** `KreuzbergDocumentBoundary*`

---

#### kreuzberg_choose_call_mode()

Decide which call mode best fits this document.

Rules applied in order:

1. `image/*` → `StructuredCallMode.VisionOnly` (no text layer to start from).
2. `application/pdf` → `StructuredCallMode.TextOnly` regardless of
   `text_coverage` or embedded image count.  Kreuzberg's OCR + text-layer
   extraction produces text for scanned PDFs; the orchestrator's
   post-call confidence gate handles any vision escalation actually needed.

3. DOCX / `text/html` / `text/*` / `application/json` / `application/xml` /
   `application/rtf` with `avg_chars_per_page > docx_text_min_density`
   → `StructuredCallMode.TextOnly`.

4. Anything else → `StructuredCallMode.Skip`.

After rule selection two post-rule promotions apply (in order):

- `user_force_vision` promotes `TextOnly` → `TextPlusVision`
  (`Skip` stays `Skip` — caller meant to opt out).

- `enable_vision_fallback` promotes `TextOnly` →
  `TextOnlyWithVisionFallback` (does **not** upgrade `TextPlusVision` or
  `Skip`).

**Signature:**

```c
KreuzbergStructuredCallMode* kreuzberg_choose_call_mode(KreuzbergStructuredInput input, KreuzbergStructuredThresholds t);
```

**Example:**

```c
KreuzbergStructuredCallMode *result = kreuzberg_choose_call_mode(NULL, NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `input` | `KreuzbergStructuredInput` | Yes | The input data |
| `t` | `KreuzbergStructuredThresholds` | Yes | The structured thresholds |

**Returns:** `KreuzbergStructuredCallMode`

---

#### kreuzberg_calculate_chunk_plan()

Calculate a chunking plan for a document.

**Returns:**

A `ChunkPlan` with optimal chunk boundaries.

**Signature:**

```c
KreuzbergChunkPlan* kreuzberg_calculate_chunk_plan(uint32_t page_count, uint64_t size_bytes, bool needs_ocr, KreuzbergHeuristicsConfig config);
```

**Example:**

```c
KreuzbergChunkPlan *result = kreuzberg_calculate_chunk_plan(42, 42, true, NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `page_count` | `uint32_t` | Yes | Total number of pages in the document |
| `size_bytes` | `uint64_t` | Yes | File size in bytes |
| `needs_ocr` | `bool` | Yes | Whether OCR will be required |
| `config` | `KreuzbergHeuristicsConfig` | Yes | Heuristics configuration |

**Returns:** `KreuzbergChunkPlan`

---

#### kreuzberg_calculate_plan_from_overrides()

Calculate a chunk plan from user-specified page ranges.

Validates and processes user overrides into a proper chunk plan.

**Signature:**

```c
KreuzbergChunkPlan* kreuzberg_calculate_plan_from_overrides(KreuzbergPageRange* user_chunks, uint32_t total_pages, uint64_t size_bytes, KreuzbergHeuristicsConfig config);
```

**Example:**

```c
KreuzbergChunkPlan *result = kreuzberg_calculate_plan_from_overrides(NULL, 42, 42, NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `user_chunks` | `KreuzbergPageRange*` | Yes | The user chunks |
| `total_pages` | `uint32_t` | Yes | The total pages |
| `size_bytes` | `uint64_t` | Yes | The size bytes |
| `config` | `KreuzbergHeuristicsConfig` | Yes | The configuration options |

**Returns:** `KreuzbergChunkPlan`

---

#### kreuzberg_fingerprint()

Stable sha256 fingerprint of `raw`, formatted as `sha256:<hex>`.

**Signature:**

```c
const char* kreuzberg_fingerprint(const uint8_t* raw);
```

**Example:**

```c
const char *result = kreuzberg_fingerprint((const uint8_t *)"data");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `raw` | `const uint8_t*` | Yes | The raw |

**Returns:** `const char*`

---

#### kreuzberg_resolve()

Resolve `(preset, custom_schema_override, context)` into a `ResolvedPreset`.

- `custom_schema` overrides `preset.schema` when set.
- `context` substitutes `{{key}}` tokens in `preset.context_template`; the
  rendered string is appended to `system_prompt` so the model sees it.

**Signature:**

```c
KreuzbergResolvedPreset* kreuzberg_resolve(KreuzbergPreset preset, void* custom_schema, void* context);
```

**Example:**

```c
KreuzbergResolvedPreset *result = kreuzberg_resolve(NULL, NULL, NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `preset` | `KreuzbergPreset` | Yes | The preset |
| `custom_schema` | `void**` | No | The custom schema |
| `context` | `void*` | Yes | The context |

**Returns:** `KreuzbergResolvedPreset`

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_extract_structured_json()

Extract structured JSON from a document using JSON-encoded preset spec and options.

This is the synchronous JSON-in / JSON-out entry point suitable for FFI and
language-binding call paths.

  `cache`).  Pass `"{}"` to use all defaults.

**Returns:**

JSON-serialised `StructuredOutput` on success.

**Errors:**

Returns `Validation` when either JSON argument is
malformed.  All other failures from the underlying
`extract_structured_sync` call are mapped onto `KreuzbergError`
via `From<StructuredError>`.

**Signature:**

```c
const char* kreuzberg_extract_structured_json(const uint8_t* bytes, const char* mime, const char* preset_spec_json, const char* options_json);
```

**Example:**

```c
const char *result = kreuzberg_extract_structured_json((const uint8_t *)"data", "value", "value", "value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `bytes` | `const uint8_t*` | Yes | The bytes |
| `mime` | `const char*` | Yes | The mime |
| `preset_spec_json` | `const char*` | Yes | The preset spec json |
| `options_json` | `const char*` | Yes | The options json |

**Returns:** `const char*`

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_split_and_extract_json()

Split a multi-document PDF and extract structured JSON from each segment,
returning a JSON array of `StructuredOutput` objects.

Non-PDF documents are passed through as a single-element array.

Same as `extract_structured_json`.

**Returns:**

JSON-serialised `const ``StructuredOutput``*` (a JSON array) on success.

**Errors:**

Returns `Validation` when either JSON argument is
malformed.  All other failures from the underlying
`split_and_extract_sync` call are mapped onto `KreuzbergError`
via `From<StructuredError>`.

**Signature:**

```c
const char* kreuzberg_split_and_extract_json(const uint8_t* bytes, const char* mime, const char* preset_spec_json, const char* options_json);
```

**Example:**

```c
const char *result = kreuzberg_split_and_extract_json((const uint8_t *)"data", "value", "value", "value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `bytes` | `const uint8_t*` | Yes | The bytes |
| `mime` | `const char*` | Yes | The mime |
| `preset_spec_json` | `const char*` | Yes | The preset spec json |
| `options_json` | `const char*` | Yes | The options json |

**Returns:** `const char*`

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_render_pdf_page_to_png()

Render a single PDF page to PNG bytes.

Returns raw PNG-encoded bytes for the specified page at the given DPI.
Uses pdf_oxide with tiny-skia for pure-Rust rendering.

For pages with extreme dimensions (very wide vector diagrams, etc.) the
effective DPI may be automatically reduced to avoid rasterizer failure.
A warning is logged when this happens.

**Errors:**

Returns `KreuzbergError.Parsing` if the PDF cannot be opened, authenticated,
or rendered, or if `page_index` is out of range.

**Signature:**

```c
const uint8_t* kreuzberg_render_pdf_page_to_png(const uint8_t* pdf_bytes, uintptr_t page_index, int32_t dpi, const char* password);
```

**Example:**

```c
const uint8_t *result = kreuzberg_render_pdf_page_to_png((const uint8_t *)"data", 42, 42, "value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `pdf_bytes` | `const uint8_t*` | Yes | Raw PDF file bytes |
| `page_index` | `uintptr_t` | Yes | Zero-based page index |
| `dpi` | `int32_t*` | No | Resolution in dots per inch (default: 150) |
| `password` | `const char**` | No | Optional password for encrypted PDFs |

**Returns:** `const uint8_t*`

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_caption_image()

Caption a single image from bytes.

  `RegionKind.Caption` prompt when `NULL`.

**Returns:**

The generated caption text.

**Errors:**

Returns an error if the VLM call fails or if image format detection fails.

**Signature:**

```c
const char* kreuzberg_caption_image(const uint8_t* image_bytes, KreuzbergLlmConfig llm_config, const char* custom_prompt);
```

**Example:**

```c
const char *result = kreuzberg_caption_image((const uint8_t *)"data", NULL, "value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `image_bytes` | `const uint8_t*` | Yes | The image data. |
| `llm_config` | `KreuzbergLlmConfig` | Yes | LLM configuration for the VLM call. |
| `custom_prompt` | `const char**` | No | Optional custom caption prompt. Uses the default |

**Returns:** `const char*`

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_caption_image_file()

Caption a single image from a file path.

  `RegionKind.Caption` prompt when `NULL`.

**Returns:**

The generated caption text.

**Errors:**

Returns an error if the file cannot be read, if image format detection fails,
or if the VLM call fails.

**Signature:**

```c
const char* kreuzberg_caption_image_file(const char* path, KreuzbergLlmConfig llm_config, const char* custom_prompt);
```

**Example:**

```c
const char *result = kreuzberg_caption_image_file("value", NULL, "value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `path` | `const char*` | Yes | Path to the image file. |
| `llm_config` | `KreuzbergLlmConfig` | Yes | LLM configuration for the VLM call. |
| `custom_prompt` | `const char**` | No | Optional custom caption prompt. Uses the default |

**Returns:** `const char*`

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_detect_mime_type()

Detect the MIME type of a file at the given path.

Uses the file extension and optionally the file content to determine the MIME type.
Set `check_exists` to `true` to verify the file exists before detection.

**Signature:**

```c
const char* kreuzberg_detect_mime_type(const char* path, bool check_exists);
```

**Example:**

```c
const char *result = kreuzberg_detect_mime_type("value", true);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `path` | `const char*` | Yes | Path to the file |
| `check_exists` | `bool` | Yes | The check exists |

**Returns:** `const char*`

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_embed_texts_async()

**Signature:**

```c
float** kreuzberg_embed_texts_async(const char** texts, KreuzbergEmbeddingConfig config);
```

**Example:**

```c
float** result = kreuzberg_embed_texts_async(NULL, NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `texts` | `const char**` | Yes | The  texts |
| `config` | `KreuzbergEmbeddingConfig` | Yes | The embedding config |

**Returns:** `float**`

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_get_embedding_preset()

Get an embedding preset by name.

Returns `NULL` if no preset with the given name exists. Returns an owned
clone so the value is safe to pass across FFI boundaries.

**Signature:**

```c
KreuzbergEmbeddingPreset* kreuzberg_get_embedding_preset(const char* name);
```

**Example:**

```c
KreuzbergEmbeddingPreset* result = kreuzberg_get_embedding_preset("value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `name` | `const char*` | Yes | The name |

**Returns:** `KreuzbergEmbeddingPreset*`

---

#### kreuzberg_list_embedding_presets()

List the names of all available embedding presets.

Returns owned `String`s so the values are safe to pass across FFI boundaries.

**Signature:**

```c
const char** kreuzberg_list_embedding_presets();
```

**Example:**

```c
const char** result = kreuzberg_list_embedding_presets();
```

**Returns:** `const char**`

---

#### kreuzberg_get_embedding_preset()

Returns `NULL` for builds without the `embedding-presets` feature.

**Signature:**

```c
KreuzbergEmbeddingPreset* kreuzberg_get_embedding_preset(const char* name);
```

**Example:**

```c
KreuzbergEmbeddingPreset* result = kreuzberg_get_embedding_preset("value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `name` | `const char*` | Yes | The  name |

**Returns:** `KreuzbergEmbeddingPreset*`

---

#### kreuzberg_list_embedding_presets()

Returns an empty list for builds without the `embedding-presets` feature.

**Signature:**

```c
const char** kreuzberg_list_embedding_presets();
```

**Example:**

```c
const char** result = kreuzberg_list_embedding_presets();
```

**Returns:** `const char**`

---

#### kreuzberg_rerank()

Rerank a list of documents by relevance to a query.

Returns documents sorted descending by score. Applies `top_k` truncation if
configured.

**Errors:**

- `KreuzbergError.Validation` if `query` is empty or blank.
- `KreuzbergError.MissingDependency` if ONNX Runtime is not installed (ONNX path).
- `KreuzbergError.Reranking` if the preset is unknown or model download fails.

Since v5.0.

**Signature:**

```c
KreuzbergRerankedDocument* kreuzberg_rerank(const char* query, const char** documents, KreuzbergRerankerConfig config);
```

**Example:**

```c
KreuzbergRerankedDocument* result = kreuzberg_rerank("value", NULL, NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `query` | `const char*` | Yes | The query |
| `documents` | `const char**` | Yes | The documents |
| `config` | `KreuzbergRerankerConfig` | Yes | The configuration options |

**Returns:** `KreuzbergRerankedDocument*`

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_rerank()

Stub for builds without the `reranker` feature — keeps the symbol available
on no-ORT targets (Android x86_64 emulator, WASM) so language bindings compile.

Since v5.0.

**Signature:**

```c
KreuzbergRerankedDocument* kreuzberg_rerank(const char* query, const char** documents, KreuzbergRerankerConfig config);
```

**Example:**

```c
KreuzbergRerankedDocument* result = kreuzberg_rerank("value", NULL, NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `query` | `const char*` | Yes | The  query |
| `documents` | `const char**` | Yes | The  documents |
| `config` | `KreuzbergRerankerConfig` | Yes | The reranker config |

**Returns:** `KreuzbergRerankedDocument*`

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_rerank_async()

Stub for builds without the `reranker` feature.

Since v5.0.

**Signature:**

```c
KreuzbergRerankedDocument* kreuzberg_rerank_async(const char* query, const char** documents, KreuzbergRerankerConfig config);
```

**Example:**

```c
KreuzbergRerankedDocument* result = kreuzberg_rerank_async("value", NULL, NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `query` | `const char*` | Yes | The  query |
| `documents` | `const char**` | Yes | The  documents |
| `config` | `KreuzbergRerankerConfig` | Yes | The reranker config |

**Returns:** `KreuzbergRerankedDocument*`

**Errors:** Returns `NULL` on error.

---

#### kreuzberg_get_reranker_preset()

Get a reranker preset by name.

Returns `NULL` if no preset with the given name exists. Returns an owned
clone so the value is safe to pass across FFI boundaries.

Since v5.0.

**Signature:**

```c
KreuzbergRerankerPreset* kreuzberg_get_reranker_preset(const char* name);
```

**Example:**

```c
KreuzbergRerankerPreset* result = kreuzberg_get_reranker_preset("value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `name` | `const char*` | Yes | The name |

**Returns:** `KreuzbergRerankerPreset*`

---

#### kreuzberg_list_reranker_presets()

List the names of all available reranker presets.

Returns owned `String`s so the values are safe to pass across FFI boundaries.

Since v5.0.

**Signature:**

```c
const char** kreuzberg_list_reranker_presets();
```

**Example:**

```c
const char** result = kreuzberg_list_reranker_presets();
```

**Returns:** `const char**`

---

#### kreuzberg_get_reranker_preset()

Returns `NULL` for builds without the `reranker-presets` feature.

Since v5.0.

**Signature:**

```c
KreuzbergRerankerPreset* kreuzberg_get_reranker_preset(const char* name);
```

**Example:**

```c
KreuzbergRerankerPreset* result = kreuzberg_get_reranker_preset("value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `name` | `const char*` | Yes | The  name |

**Returns:** `KreuzbergRerankerPreset*`

---

#### kreuzberg_list_reranker_presets()

Returns an empty list for builds without the `reranker-presets` feature.

Since v5.0.

**Signature:**

```c
const char** kreuzberg_list_reranker_presets();
```

**Example:**

```c
const char** result = kreuzberg_list_reranker_presets();
```

**Returns:** `const char**`

---

#### kreuzberg_embed_texts_async()

**Signature:**

```c
float** kreuzberg_embed_texts_async(const char** texts, KreuzbergEmbeddingConfig config);
```

**Example:**

```c
float** result = kreuzberg_embed_texts_async(NULL, NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `texts` | `const char**` | Yes | The  texts |
| `config` | `KreuzbergEmbeddingConfig` | Yes | The embedding config |

**Returns:** `float**`

**Errors:** Returns `NULL` on error.

---

### Types

#### KreuzbergAccelerationConfig

Hardware acceleration configuration for ONNX Runtime models.

Controls which execution provider (CPU, CoreML, CUDA, TensorRT) is used
for inference in layout detection and embedding generation.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `provider` | `KreuzbergExecutionProviderType` | `KREUZBERG_KREUZBERG_AUTO` | Execution provider to use for ONNX inference. |
| `device_id` | `uint32_t` | — | GPU device ID (for CUDA/TensorRT). Ignored for CPU/CoreML/Auto. |

---

#### KreuzbergArchiveEntry

A single file extracted from an archive.

When archives (ZIP, TAR, 7Z, GZIP) are extracted with recursive extraction
enabled, each processable file produces its own full `ExtractionResult`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `path` | `const char*` | — | Archive-relative file path (e.g. "folder/document.pdf"). |
| `mime_type` | `const char*` | — | Detected MIME type of the file. |
| `result` | `KreuzbergExtractionResult` | — | Full extraction result for this file. |

---

#### KreuzbergArchiveMetadata

Archive (ZIP/TAR/7Z) metadata.

Extracted from compressed archive files containing file lists and size information.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `format` | `const char*` | — | Archive format ("ZIP", "TAR", "7Z", etc.) |
| `file_count` | `uint32_t` | — | Total number of files in the archive |
| `file_list` | `const char**` | `NULL` | List of file paths within the archive |
| `total_size` | `uint64_t` | — | Total uncompressed size in bytes |
| `compressed_size` | `uint64_t*` | `NULL` | Compressed size in bytes (if available) |

---

#### KreuzbergAudioMetadata

Audio/video file metadata.

Populated from container tags (ID3v2, MP4 atoms, Vorbis comments, etc.) and
PCM decode properties. Available when the `transcription-types` feature is enabled.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `duration_ms` | `uint64_t*` | `NULL` | Duration in milliseconds derived from the decoded audio stream. |
| `codec` | `const char**` | `NULL` | Audio codec (e.g. "mp3", "aac", "opus", "flac"). |
| `container` | `const char**` | `NULL` | Container format (e.g. "mpeg", "mp4", "ogg", "wav"). |
| `sample_rate_hz` | `uint32_t*` | `NULL` | Sample rate in Hz after decode (always 16000 when resampled for Whisper). |
| `channels` | `uint16_t*` | `NULL` | Number of audio channels (1 = mono, 2 = stereo). |
| `bitrate` | `uint32_t*` | `NULL` | Audio bitrate in kbps from the source file tags/properties. |

---

#### KreuzbergBBox

Bounding box in original image coordinates (x1, y1) top-left, (x2, y2) bottom-right.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `x1` | `float` | — | Left edge (x-coordinate of the top-left corner). |
| `y1` | `float` | — | Top edge (y-coordinate of the top-left corner). |
| `x2` | `float` | — | Right edge (x-coordinate of the bottom-right corner). |
| `y2` | `float` | — | Bottom edge (y-coordinate of the bottom-right corner). |

---

#### KreuzbergBatchBytesItem

Batch item for byte array extraction.

Used with `batch_extract_bytes` and `batch_extract_bytes_sync`
to represent a single item in a batch extraction job.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `const uint8_t*` | — | The content bytes to extract from |
| `mime_type` | `const char*` | — | MIME type of the content (e.g., "application/pdf", "text/html") |
| `config` | `KreuzbergFileExtractionConfig*` | `NULL` | Per-item configuration overrides (None uses batch-level defaults) |

---

#### KreuzbergBatchFileItem

Batch item for file extraction.

Used with `batch_extract_files` and `batch_extract_files_sync`
to represent a single file in a batch extraction job.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `path` | `const char*` | — | Path to the file to extract from |
| `config` | `KreuzbergFileExtractionConfig*` | `NULL` | Per-file configuration overrides (None uses batch-level defaults) |

---

#### KreuzbergBibtexMetadata

BibTeX bibliography metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `entry_count` | `uintptr_t` | — | Number of entries in the bibliography. |
| `citation_keys` | `const char**` | `NULL` | BibTeX citation keys (e.g. `"knuth1984"`) for all entries. |
| `authors` | `const char**` | `NULL` | Author names collected across all bibliography entries. |
| `year_range` | `KreuzbergYearRange*` | `NULL` | Earliest and latest publication years found in the bibliography. |
| `entry_types` | `void**` | `NULL` | Count of entries grouped by BibTeX entry type (e.g. `"article"` → 5). |

---

#### KreuzbergBoundingBox

Bounding box coordinates for element positioning.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `x0` | `double` | — | Left x-coordinate |
| `y0` | `double` | — | Bottom y-coordinate |
| `x1` | `double` | — | Right x-coordinate |
| `y1` | `double` | — | Top y-coordinate |

---

#### KreuzbergCacheStats

Aggregate statistics for a kreuzberg cache directory.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `total_files` | `uintptr_t` | — | Total number of files currently in the cache directory. |
| `total_size_mb` | `double` | — | Combined size of all cache files in megabytes. |
| `available_space_mb` | `double` | — | Free disk space available on the cache volume, in megabytes. |
| `oldest_file_age_days` | `double` | — | Age of the oldest cache file in days (0.0 if the cache is empty). |
| `newest_file_age_days` | `double` | — | Age of the most recently written cache file in days (0.0 if the cache is empty). |

---

#### KreuzbergCaptioningConfig

**Since:** `v5.0`

Configuration for the VLM captioning post-processor.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `llm` | `KreuzbergLlmConfig` | — | LLM configuration used for the VLM call. |
| `prompt` | `const char**` | `NULL` | Optional custom caption prompt. `NULL` uses the default `RegionKind.Caption` prompt that ships with `crate.llm.region_extractor`. |
| `min_image_area` | `uint32_t` | `/* serde(default) */` | Skip images whose `width * height` is below this threshold (in pixels). Default `1_000` filters out icons and decorations. |

---

#### KreuzbergCaptioningEnrichmentConfig

Captioning enrichment knob: which LLM to use for image captions.

The enrichment stage calls `caption_image` for every
image in `ExtractionResult.images` that has non-empty `data`. Images with
empty byte data (e.g. reference-only images populated via `source_path`) are
skipped rather than forwarded to the VLM.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `config` | `KreuzbergLlmConfig` | — | LLM / VLM configuration forwarded verbatim to each `caption_image` call. |
| `custom_prompt` | `const char**` | `NULL` | Optional custom prompt override forwarded to every `caption_image` call. `NULL` uses the default `RegionKind.Caption` prompt. |

---

#### KreuzbergCellChange

A single changed cell within a table.

Defined here (rather than only in `crate.diff`) so `RevisionDelta` can
reference it unconditionally, without requiring the `diff` Cargo feature.
`crate.diff` re-exports this type verbatim.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `row` | `uintptr_t` | — | Zero-based row index. |
| `col` | `uintptr_t` | — | Zero-based column index. |
| `from` | `const char*` | — | Value before the change. |
| `to` | `const char*` | — | Value after the change. |

---

#### KreuzbergChunk

A text chunk with optional embedding and metadata.

Chunks are created when chunking is enabled in `ExtractionConfig`. Each chunk
contains the text content, optional embedding vector (if embedding generation
is configured), and metadata about its position in the document.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `const char*` | — | The text content of this chunk. |
| `chunk_type` | `KreuzbergChunkType` | `/* serde(default) */` | Semantic structural classification of this chunk. Assigned by the heuristic classifier based on content patterns and heading context. Defaults to `ChunkType.Unknown` when no rule matches. |
| `embedding` | `float**` | `NULL` | Optional embedding vector for this chunk. Only populated when `EmbeddingConfig` is provided in chunking configuration. The dimensionality depends on the chosen embedding model. |
| `metadata` | `KreuzbergChunkMetadata` | — | Metadata about this chunk's position and properties. |

---

#### KreuzbergChunkInfo

Information about a single chunk.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `index` | `uint32_t` | — | Zero-based chunk index. |
| `pages` | `KreuzbergPageRange` | — | Page range for this chunk. |
| `estimated_time_ms` | `uint64_t` | — | Estimated processing time for this chunk in milliseconds. |

---

#### KreuzbergChunkMetadata

Metadata about a chunk's position in the original document.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `byte_start` | `uintptr_t` | — | Byte offset where this chunk starts in the original text (UTF-8 valid boundary). |
| `byte_end` | `uintptr_t` | — | Byte offset where this chunk ends in the original text (UTF-8 valid boundary). |
| `token_count` | `uintptr_t*` | `NULL` | Number of tokens in this chunk (if available). This is calculated by the embedding model's tokenizer if embeddings are enabled. |
| `chunk_index` | `uintptr_t` | — | Zero-based index of this chunk in the document. |
| `total_chunks` | `uintptr_t` | — | Total number of chunks in the document. |
| `first_page` | `uint32_t*` | `NULL` | First page number this chunk spans (1-indexed). Only populated when page tracking is enabled in extraction configuration. |
| `last_page` | `uint32_t*` | `NULL` | Last page number this chunk spans (1-indexed, equal to first_page for single-page chunks). Only populated when page tracking is enabled in extraction configuration. |
| `heading_context` | `KreuzbergHeadingContext*` | `/* serde(default) */` | Heading context when using Markdown chunker. Contains the heading hierarchy this chunk falls under. Only populated when `ChunkerType.Markdown` is used. |
| `heading_path` | `const char**` | `/* serde(default) */` | Flattened heading trail from document root to this chunk's section. Each element is a heading's text, outermost first. Derived from `heading_context` when present; empty otherwise. Provides a binding-friendly, RAG-shaped breadcrumb without requiring callers to walk the nested `HeadingContext` structure. |
| `image_indices` | `uint32_t*` | `/* serde(default) */` | Indices into `ExtractionResult.images` for images on pages covered by this chunk. Contains zero-based indices into the top-level `images` collection for every image whose `page_number` falls within `\[first_page, last_page\]`. Empty when image extraction is disabled or the chunk spans no pages with images. |

---

#### KreuzbergChunkPlan

Complete chunking plan for a document.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `total_chunks` | `uint32_t` | — | Total number of chunks. |
| `chunks` | `KreuzbergChunkInfo*` | — | Individual chunk information. |
| `total_estimated_time_ms` | `uint64_t` | — | Estimated total processing time in milliseconds. |
| `use_disk_processing` | `bool` | — | Whether to use disk-based processing for large files. |
| `reason` | `KreuzbergChunkingReason` | — | Reason for chunking. |

##### Methods

###### kreuzberg_total_pages()

Get the total number of pages across all chunks.

**Signature:**

```c
uint32_t kreuzberg_total_pages();
```

**Example:**

```c
uint32_t result = kreuzberg_total_pages(instance);
```

**Returns:** `uint32_t`

---

#### KreuzbergChunkingConfig

Chunking configuration.

Configures text chunking for document content, including chunk size,
overlap, trimming behavior, and optional embeddings.

Use `..the default constructor` when constructing to allow for future field additions:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_characters` | `uintptr_t` | `1000` | Maximum size per chunk (in units determined by `sizing`). When `sizing` is `Characters` (default), this is the max character count. When using token-based sizing, this is the max token count. Default: 1000 |
| `overlap` | `uintptr_t` | `200` | Overlap between chunks (in units determined by `sizing`). Default: 200 |
| `trim` | `bool` | `true` | Whether to trim whitespace from chunk boundaries. Default: true |
| `chunker_type` | `KreuzbergChunkerType` | `KREUZBERG_KREUZBERG_TEXT` | Type of chunker to use (Text or Markdown). Default: Text |
| `embedding` | `KreuzbergEmbeddingConfig*` | `NULL` | Optional embedding configuration for chunk embeddings. |
| `preset` | `const char**` | `NULL` | Use a preset configuration (overrides individual settings if provided). |
| `sizing` | `KreuzbergChunkSizing` | `KREUZBERG_KREUZBERG_CHARACTERS` | How to measure chunk size. Default: `Characters` (Unicode character count). Enable `chunking-tiktoken` or `chunking-tokenizers` features for token-based sizing. |
| `prepend_heading_context` | `bool` | `false` | When `true` and `chunker_type` is `Markdown`, prepend the heading hierarchy path (e.g. `"# Title > ## Section\n\n"`) to each chunk's content string. This is useful for RAG pipelines where each chunk needs self-contained context about its position in the document structure. Default: `false` |
| `topic_threshold` | `float*` | `NULL` | Optional cosine similarity threshold for semantic topic boundary detection. Only used when `chunker_type` is `Semantic` and an `EmbeddingConfig` is provided. You almost never need to set this. When omitted, defaults to `0.75` which works well for most documents. Lower values detect more topic boundaries (more, smaller chunks); higher values detect fewer. Range: `0.0..=1.0`. |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergChunkingConfig kreuzberg_default();
```

**Example:**

```c
KreuzbergChunkingConfig *result = kreuzberg_default();
```

**Returns:** `KreuzbergChunkingConfig`

---

#### KreuzbergChunkingResult

Result of a text chunking operation.

Contains the generated chunks and metadata about the chunking.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `chunks` | `KreuzbergChunk*` | — | List of text chunks |
| `chunk_count` | `uintptr_t` | — | Total number of chunks generated |

---

#### KreuzbergCitationMetadata

Citation file metadata (RIS, PubMed, EndNote).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `citation_count` | `uintptr_t` | — | Total number of citation records in the file. |
| `format` | `const char**` | `NULL` | Detected citation file format (e.g. `"ris"`, `"pubmed"`, `"endnote"`). |
| `authors` | `const char**` | `NULL` | Author names collected across all citation records. |
| `year_range` | `KreuzbergYearRange*` | `NULL` | Earliest and latest publication years found in the file. |
| `dois` | `const char**` | `NULL` | DOI identifiers found in the citation records. |
| `keywords` | `const char**` | `NULL` | Keywords collected from all citation records. |

---

#### KreuzbergClassificationEnrichmentConfig

Classification enrichment knob: how to label the document.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `config` | `KreuzbergPageClassificationConfig` | — | Label set and LLM settings for the classification stage. |

---

#### KreuzbergClassificationLabel

A single label + confidence pair.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `label` | `const char*` | — | Label name as configured in `PageClassificationConfig.labels`. |
| `confidence` | `float*` | `NULL` | Backend-reported confidence in `\[0.0, 1.0\]`. `NULL` when the backend (e.g. an LLM prompt without explicit confidence schema) did not report one. |

---

#### KreuzbergConfidenceSignals

Input signals for confidence scoring.

Caller fills these from the extraction result and the LLM response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text_coverage` | `float` | — | Fraction of pages with usable text in `\[0, 1\]`. |
| `ocr_aggregate` | `float*` | `NULL` | Mean OCR per-element recognition confidence; `NULL` when OCR did not run. |
| `schema_compliance` | `KreuzbergSchemaCompliance` | — | Schema-validation result of the merged output. |

##### Methods

###### kreuzberg_from_extraction_result()

Build `ConfidenceSignals` from an `ExtractionResult`.

- `result` — The extraction result whose `ocr_elements` are inspected.
- `schema_compliance` — Caller-supplied schema validation outcome.
- `text_coverage` — Caller-supplied fraction of pages with usable text
  (e.g. 1.0 for native text formats, value from PDF analysis for PDFs).

The `ocr_aggregate` is computed as the arithmetic mean of all
`ocr_elements[].confidence.recognition` values.  When `ocr_elements` is
`NULL` or empty the field is set to `NULL`.

**Signature:**

```c
KreuzbergConfidenceSignals kreuzberg_from_extraction_result(KreuzbergExtractionResult result, KreuzbergSchemaCompliance schema_compliance, float text_coverage);
```

**Example:**

```c
KreuzbergConfidenceSignals *result = kreuzberg_from_extraction_result(NULL, (KreuzbergSchemaCompliance){0}, 0.5);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `result` | `KreuzbergExtractionResult` | Yes | The extraction result |
| `schema_compliance` | `KreuzbergSchemaCompliance` | Yes | The schema compliance |
| `text_coverage` | `float` | Yes | The text coverage |

**Returns:** `KreuzbergConfidenceSignals`

---

#### KreuzbergConfidenceWeights

Tunable weights for the confidence scoring formula.

Defaults picked by inspection; callers tune them via config.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text_coverage` | `float` | `0.3` | Weight assigned to `text_coverage`. Default 0.30. |
| `ocr_aggregate` | `float` | `0.3` | Weight assigned to `ocr_aggregate` when OCR ran. Default 0.30 — folds into `text_coverage` weight when OCR did not run. |
| `schema_compliance` | `float` | `0.4` | Weight assigned to `schema_compliance`. Default 0.40. |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergConfidenceWeights kreuzberg_default();
```

**Example:**

```c
KreuzbergConfidenceWeights *result = kreuzberg_default();
```

**Returns:** `KreuzbergConfidenceWeights`

###### kreuzberg_is_normalized()

Validate that weights sum to approximately 1.0.

**Signature:**

```c
bool kreuzberg_is_normalized();
```

**Example:**

```c
bool result = kreuzberg_is_normalized(instance);
```

**Returns:** `bool`

---

#### KreuzbergContentFilterConfig

Cross-extractor content filtering configuration.

Controls whether "furniture" content (headers, footers, page numbers,
watermarks, repeating text) is included in or stripped from extraction
results. Applies across all extractors (PDF, DOCX, RTF, ODT, HTML, etc.)
with format-specific implementation.

When `NULL` on `ExtractionConfig`, each extractor uses its current
default behavior unchanged.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `include_headers` | `bool` | `false` | Include running headers in extraction output. - PDF: Disables top-margin furniture stripping and prevents the layout model from treating `PageHeader`-classified regions as furniture. - DOCX: Includes document headers in text output. - RTF/ODT: Headers already included; this is a no-op when true. - HTML/EPUB: Keeps `<header>` element content. Default: `false` (headers are stripped or excluded). |
| `include_footers` | `bool` | `false` | Include running footers in extraction output. - PDF: Disables bottom-margin furniture stripping and prevents the layout model from treating `PageFooter`-classified regions as furniture. - DOCX: Includes document footers in text output. - RTF/ODT: Footers already included; this is a no-op when true. - HTML/EPUB: Keeps `<footer>` element content. Default: `false` (footers are stripped or excluded). |
| `strip_repeating_text` | `bool` | `true` | Enable the heuristic cross-page repeating text detector. When `true` (default), text that repeats verbatim across a supermajority of pages is classified as furniture and stripped.  Disable this if brand names or repeated headings are being incorrectly removed by the heuristic. Note: when a layout-detection model is active, the model may independently classify page-header / page-footer regions as furniture on a per-page basis. To preserve those regions, set `include_headers = true`, `include_footers = true`, or both, in addition to disabling this flag. Primarily affects PDF extraction. Default: `true`. |
| `include_watermarks` | `bool` | `false` | Include watermark text in extraction output. - PDF: Keeps watermark artifacts and arXiv identifiers. - Other formats: No effect currently. Default: `false` (watermarks are stripped). |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergContentFilterConfig kreuzberg_default();
```

**Example:**

```c
KreuzbergContentFilterConfig *result = kreuzberg_default();
```

**Returns:** `KreuzbergContentFilterConfig`

---

#### KreuzbergContributorRole

JATS contributor with role.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `const char*` | — | Contributor display name. |
| `role` | `const char**` | `NULL` | Contributor role (e.g. `"author"`, `"editor"`). |

---

#### KreuzbergCoreProperties

Dublin Core metadata from docProps/core.xml

Contains standard metadata fields defined by the Dublin Core standard
and Office-specific extensions.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `title` | `const char**` | `NULL` | Document title |
| `subject` | `const char**` | `NULL` | Document subject/topic |
| `creator` | `const char**` | `NULL` | Document creator/author |
| `keywords` | `const char**` | `NULL` | Keywords or tags |
| `description` | `const char**` | `NULL` | Document description/abstract |
| `last_modified_by` | `const char**` | `NULL` | User who last modified the document |
| `revision` | `const char**` | `NULL` | Revision number |
| `created` | `const char**` | `NULL` | Creation timestamp (ISO 8601) |
| `modified` | `const char**` | `NULL` | Last modification timestamp (ISO 8601) |
| `category` | `const char**` | `NULL` | Document category |
| `content_status` | `const char**` | `NULL` | Content status (Draft, Final, etc.) |
| `language` | `const char**` | `NULL` | Document language |
| `identifier` | `const char**` | `NULL` | Unique identifier |
| `version` | `const char**` | `NULL` | Document version |
| `last_printed` | `const char**` | `NULL` | Last print timestamp (ISO 8601) |

---

#### KreuzbergCsvMetadata

CSV/TSV file metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `row_count` | `uint32_t` | — | Total number of data rows (excluding the header row if present). |
| `column_count` | `uint32_t` | — | Number of columns detected. |
| `delimiter` | `const char**` | `NULL` | Field delimiter character (e.g. `","` or `"\t"`). |
| `has_header` | `bool` | — | Whether the first row was treated as a header. |
| `column_types` | `const char***` | `NULL` | Inferred data type for each column (e.g. `"string"`, `"integer"`, `"float"`). |

---

#### KreuzbergDbfFieldInfo

dBASE field information.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `const char*` | — | Field (column) name. |
| `field_type` | `const char*` | — | dBASE field type character (e.g. `"C"` for character, `"N"` for numeric). |

---

#### KreuzbergDbfMetadata

dBASE (DBF) file metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `record_count` | `uintptr_t` | — | Total number of data records in the DBF file. |
| `field_count` | `uintptr_t` | — | Number of field (column) definitions. |
| `fields` | `KreuzbergDbfFieldInfo*` | `NULL` | Descriptor for each field in the table schema. |

---

#### KreuzbergDetectResponse

MIME type detection response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `mime_type` | `const char*` | — | Detected MIME type |
| `filename` | `const char**` | `NULL` | Original filename (if provided) |

---

#### KreuzbergDetectionResult

Page-level detection result containing all detections and page metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `page_width` | `uint32_t` | — | Page width in pixels (as seen by the model). |
| `page_height` | `uint32_t` | — | Page height in pixels (as seen by the model). |
| `detections` | `KreuzbergLayoutDetection*` | — | All layout detections on this page after postprocessing. |

---

#### KreuzbergDiffHunk

A single contiguous hunk in a unified diff.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `from_line` | `uintptr_t` | — | Starting line number in the old content (0-indexed). |
| `from_count` | `uintptr_t` | — | Number of lines from the old content in this hunk. |
| `to_line` | `uintptr_t` | — | Starting line number in the new content (0-indexed). |
| `to_count` | `uintptr_t` | — | Number of lines from the new content in this hunk. |
| `lines` | `KreuzbergDiffLine*` | — | Lines that make up this hunk. |

---

#### KreuzbergDiffOptions

Options controlling how two `ExtractionResult` values are compared.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `include_metadata` | `bool` | `true` | Include metadata changes in the diff. Default: `true`. |
| `include_embedded` | `bool` | `true` | Include embedded-children changes in the diff. Default: `true`. |
| `max_content_chars` | `uintptr_t*` | `NULL` | Truncate content to this many characters before diffing. Useful for very large documents where only the first N characters matter. `NULL` means no truncation. |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergDiffOptions kreuzberg_default();
```

**Example:**

```c
KreuzbergDiffOptions *result = kreuzberg_default();
```

**Returns:** `KreuzbergDiffOptions`

---

#### KreuzbergDjotContent

Comprehensive Djot document structure with semantic preservation.

This type captures the full richness of Djot markup, including:

- Block-level structures (headings, lists, blockquotes, code blocks, etc.)
- Inline formatting (emphasis, strong, highlight, subscript, superscript, etc.)
- Attributes (classes, IDs, key-value pairs)
- Links, images, footnotes
- Math expressions (inline and display)
- Tables with full structure

Available when the `djot` feature is enabled.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `plain_text` | `const char*` | — | Plain text representation for backwards compatibility |
| `blocks` | `KreuzbergFormattedBlock*` | — | Structured block-level content |
| `metadata` | `KreuzbergMetadata` | — | Metadata from YAML frontmatter |
| `tables` | `KreuzbergTable*` | — | Extracted tables as structured data |
| `images` | `KreuzbergDjotImage*` | — | Extracted images with metadata |
| `links` | `KreuzbergDjotLink*` | — | Extracted links with URLs |
| `footnotes` | `KreuzbergFootnote*` | — | Footnote definitions |

---

#### KreuzbergDjotImage

Image element in Djot.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `src` | `const char*` | — | Image source URL or path |
| `alt` | `const char*` | — | Alternative text |
| `title` | `const char**` | `NULL` | Optional title |

---

#### KreuzbergDjotLink

Link element in Djot.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | `const char*` | — | Link URL |
| `text` | `const char*` | — | Link text content |
| `title` | `const char**` | `NULL` | Optional title |

---

#### KreuzbergDocumentBoundary

Detected document boundary within a PDF.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `start_page` | `uint32_t` | — | 1-indexed start page (inclusive). |
| `end_page` | `uint32_t` | — | 1-indexed end page (inclusive). |
| `confidence` | `float` | — | Confidence in this boundary, `\[0.0, 1.0\]`. |
| `reason` | `KreuzbergBoundaryReason` | — | Reason for the boundary detection. |

---

#### KreuzbergDocumentExtractor

Trait for document extractor plugins.

Implement this trait to add support for new document formats or to override
built-in extraction behavior with custom logic.

##### Return Type

Extractors return `InternalDocument`, a flat intermediate representation.
The pipeline converts this into the public `ExtractionResult` via the
derivation step.

##### Priority System

When multiple extractors support the same MIME type, the registry selects
the extractor with the highest priority value. Use this to:

- Override built-in extractors (priority > 50)
- Provide fallback extractors (priority < 50)
- Implement specialized extractors for specific use cases

Default priority is 50.

##### Thread Safety

Extractors must be thread-safe (`Send + Sync`) to support concurrent extraction.

##### Methods

###### kreuzberg_extract_bytes()

Extract content from a byte array.

This is the core extraction method that processes in-memory document data.

**Returns:**

An `InternalDocument` containing the extracted elements, metadata, and tables.
The pipeline will convert this into the public `ExtractionResult`.

**Errors:**

- `KreuzbergError.Parsing` - Document parsing failed
- `KreuzbergError.Validation` - Invalid document structure
- `KreuzbergError.Io` - I/O errors (these always bubble up)
- `KreuzbergError.MissingDependency` - Required dependency not available

**Signature:**

```c
KreuzbergInternalDocument kreuzberg_extract_bytes(const uint8_t* content, const char* mime_type, KreuzbergExtractionConfig config);
```

**Example:**

```c
KreuzbergInternalDocument *result = kreuzberg_extract_bytes(instance, (const uint8_t *)"data", "value", NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `content` | `const uint8_t*` | Yes | Raw document bytes |
| `mime_type` | `const char*` | Yes | MIME type of the document (already validated) |
| `config` | `KreuzbergExtractionConfig` | Yes | Extraction configuration |

**Returns:** `KreuzbergInternalDocument`

**Errors:** Returns `NULL` on error.

###### kreuzberg_extract_file()

Extract content from a file.

Default implementation reads the file and calls `extract_bytes`.
Override for custom file handling, streaming, or memory optimizations.

**Returns:**

An `InternalDocument` containing the extracted elements, metadata, and tables.

**Errors:**

Same as `extract_bytes`, plus file I/O errors.

**Signature:**

```c
KreuzbergInternalDocument kreuzberg_extract_file(const char* path, const char* mime_type, KreuzbergExtractionConfig config);
```

**Example:**

```c
KreuzbergInternalDocument *result = kreuzberg_extract_file(instance, "value", "value", NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `path` | `const char*` | Yes | Path to the document file |
| `mime_type` | `const char*` | Yes | MIME type of the document (already validated) |
| `config` | `KreuzbergExtractionConfig` | Yes | Extraction configuration |

**Returns:** `KreuzbergInternalDocument`

**Errors:** Returns `NULL` on error.

###### kreuzberg_supported_mime_types()

Get the list of MIME types supported by this extractor.

Can include exact MIME types and prefix patterns:

- Exact: `"application/pdf"`, `"text/plain"`
- Prefix: `"image/*"` (matches any image type)

**Returns:**

A slice of MIME type strings.

**Signature:**

```c
const char** kreuzberg_supported_mime_types();
```

**Example:**

```c
const char** result = kreuzberg_supported_mime_types(instance);
```

**Returns:** `const char**`

###### kreuzberg_priority()

Get the priority of this extractor.

Higher priority extractors are preferred when multiple extractors
support the same MIME type.

##### Priority Guidelines

- **0-25**: Fallback/low-quality extractors
- **26-49**: Alternative extractors
- **50**: Default priority (built-in extractors)
- **51-75**: Premium/enhanced extractors
- **76-100**: Specialized/high-priority extractors

**Returns:**

Priority value (default: 50)

**Signature:**

```c
int32_t kreuzberg_priority();
```

**Example:**

```c
int32_t result = kreuzberg_priority(instance);
```

**Returns:** `int32_t`

###### kreuzberg_can_handle()

Optional: Check if this extractor can handle a specific file.

Allows for more sophisticated detection beyond MIME types.
Defaults to `true` (rely on MIME type matching).

**Returns:**

`true` if the extractor can handle this file, `false` otherwise.

**Signature:**

```c
bool kreuzberg_can_handle(const char* path, const char* mime_type);
```

**Example:**

```c
bool result = kreuzberg_can_handle(instance, "value", "value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `path` | `const char*` | Yes | The  path |
| `mime_type` | `const char*` | Yes | The  mime type |

**Returns:** `bool`

---

#### KreuzbergDocumentMetadata

Metadata about a document for analysis.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `mime_type` | `const char*` | — | MIME type of the document. |
| `size_bytes` | `uint64_t` | — | File size in bytes. |
| `page_count` | `uint32_t*` | `NULL` | Page count (if known, e.g., from previous analysis). |
| `force_ocr` | `bool` | — | Whether OCR is forced regardless of text layer. |
| `user_chunk_config` | `KreuzbergUserChunkConfig*` | `NULL` | User-provided chunk configuration overrides. |
| `chunking_enabled` | `bool` | — | Whether chunking is enabled for this job. |

---

#### KreuzbergDocumentNode

A single node in the document tree.

Each node has deterministic `id`, typed `content`, optional `parent`/`children`
for tree structure, and metadata like page number, bounding box, and content layer.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `KreuzbergNodeContent` | — | Node content — tagged enum, type-specific data only. |
| `parent` | `uint32_t*` | `NULL` | Parent node index (`NULL` = root-level node). |
| `children` | `uint32_t*` | `/* serde(default) */` | Child node indices in reading order. |
| `content_layer` | `KreuzbergContentLayer` | `/* serde(default) */` | Content layer classification. Always serialised — Kotlin-Android (and any other typed binding) treats the field as non-nullable, so omitting it from the JSON wire would break consumer deserialisation.  `#\[serde(default)\]` covers the missing-field case on inbound JSON. |
| `page` | `uint32_t*` | `NULL` | Page number where this node starts (1-indexed). |
| `page_end` | `uint32_t*` | `NULL` | Page number where this node ends (for multi-page tables/sections). |
| `bbox` | `KreuzbergBoundingBox*` | `NULL` | Bounding box in document coordinates. |
| `annotations` | `KreuzbergTextAnnotation*` | `/* serde(default) */` | Inline annotations (formatting, links) on this node's text content. Only meaningful for text-carrying nodes; empty for containers. |
| `attributes` | `void**` | `NULL` | Format-specific key-value attributes. Extensible bag for miscellaneous data without a dedicated typed field: CSS classes, LaTeX environment names, Excel cell formulas, slide layout names, etc. |

---

#### KreuzbergDocumentRelationship

A resolved relationship between two nodes in the document tree.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `source` | `uint32_t` | — | Source node index (the referencing node). |
| `target` | `uint32_t` | — | Target node index (the referenced node). |
| `kind` | `KreuzbergRelationshipKind` | — | Semantic kind of the relationship. |

---

#### KreuzbergDocumentRevision

A single tracked change embedded in a document.

Populated by per-format extractors that understand change-tracking metadata
(DOCX `w:ins`/`w:del`/`w:rPrChange`, ODT `text:change-*`, …). Every
extractor defaults to `ExtractionResult.revisions = None` until a
format-specific implementation is added.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `revision_id` | `const char*` | — | Format-specific revision identifier. For DOCX this is the `w:id` attribute value on the change element (e.g. `"42"`). When the attribute is absent a synthetic fallback is generated (`"docx-ins-0"`, `"docx-del-3"`, …). |
| `author` | `const char**` | `NULL` | Display name of the author who made this change, when available. |
| `timestamp` | `const char**` | `NULL` | ISO-8601 timestamp of the change, when available. Stored as a plain string so this type remains FFI-friendly and unconditionally available without the `chrono` optional dep. DOCX populates this from the `w:date` attribute (e.g. `"2024-03-15T10:30:00Z"`). |
| `kind` | `KreuzbergRevisionKind` | — | Semantic kind of this revision. |
| `anchor` | `KreuzbergRevisionAnchor*` | `NULL` | Best-effort document location for this revision. Resolution is format-dependent and may be `NULL` when the location cannot be determined (e.g. changes inside table cells before table-cell anchor support is added). |
| `delta` | `KreuzbergRevisionDelta` | — | The content changes that make up this revision. |

---

#### KreuzbergDocumentStructure

Top-level structured document representation.

A flat array of nodes with index-based parent/child references forming a tree.
Root-level nodes have `parent: None`. Use `body_roots()` and `furniture_roots()`
to iterate over top-level content by layer.

##### Validation

Call `validate()` after construction to verify all node indices are in bounds
and parent-child relationships are bidirectionally consistent.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `nodes` | `KreuzbergDocumentNode*` | `NULL` | All nodes in document/reading order. |
| `source_format` | `const char**` | `NULL` | Origin format identifier (e.g. "docx", "pptx", "html", "pdf"). Allows renderers to apply format-aware heuristics when converting the document tree to output formats. |
| `relationships` | `KreuzbergDocumentRelationship*` | `NULL` | Resolved relationships between nodes (footnote refs, citations, anchor links, etc.). Populated during derivation from the internal document representation. Empty when no relationships are detected. |
| `node_types` | `const char**` | `NULL` | Sorted, deduplicated list of node type names present in this document. Each value is the snake_case `node_type` tag of the corresponding `NodeContent` variant (e.g. `"paragraph"`, `"heading"`, `"table"`, …). Computed from `nodes` via `DocumentStructure.finalize_node_types`. Empty until that method is called (internal construction paths call it at the end of derivation). |

##### Methods

###### kreuzberg_finalize_node_types()

Compute and populate the `node_types` field from the current `nodes`.

Call this after all nodes have been added to the structure. Internal
construction paths (builder, derivation) call this automatically.

**Signature:**

```c
void kreuzberg_finalize_node_types();
```

**Example:**

```c
kreuzberg_finalize_node_types(instance);
```

**Returns:** No return value.

###### kreuzberg_is_empty()

Check if the document structure is empty.

**Signature:**

```c
bool kreuzberg_is_empty();
```

**Example:**

```c
bool result = kreuzberg_is_empty(instance);
```

**Returns:** `bool`

###### kreuzberg_default()

**Signature:**

```c
KreuzbergDocumentStructure kreuzberg_default();
```

**Example:**

```c
KreuzbergDocumentStructure *result = kreuzberg_default();
```

**Returns:** `KreuzbergDocumentStructure`

---

#### KreuzbergDocumentSummary

Summary of an extracted document.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text` | `const char*` | — | Summary text (plain prose). |
| `strategy` | `KreuzbergSummaryStrategy` | — | Strategy that produced this summary. |
| `token_count` | `uint32_t*` | `NULL` | Approximate token count of the summary, when known. |

---

#### KreuzbergDocxAppProperties

Application properties from docProps/app.xml for DOCX

Contains Word-specific document statistics and metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `application` | `const char**` | `NULL` | Application name (e.g., "Microsoft Office Word") |
| `app_version` | `const char**` | `NULL` | Application version |
| `template` | `const char**` | `NULL` | Template filename |
| `total_time` | `int32_t*` | `NULL` | Total editing time in minutes |
| `pages` | `int32_t*` | `NULL` | Number of pages |
| `words` | `int32_t*` | `NULL` | Number of words |
| `characters` | `int32_t*` | `NULL` | Number of characters (excluding spaces) |
| `characters_with_spaces` | `int32_t*` | `NULL` | Number of characters (including spaces) |
| `lines` | `int32_t*` | `NULL` | Number of lines |
| `paragraphs` | `int32_t*` | `NULL` | Number of paragraphs |
| `company` | `const char**` | `NULL` | Company name |
| `doc_security` | `int32_t*` | `NULL` | Document security level |
| `scale_crop` | `bool*` | `NULL` | Scale crop flag |
| `links_up_to_date` | `bool*` | `NULL` | Links up to date flag |
| `shared_doc` | `bool*` | `NULL` | Shared document flag |
| `hyperlinks_changed` | `bool*` | `NULL` | Hyperlinks changed flag |

---

#### KreuzbergDocxMetadata

Word document metadata.

Extracted from DOCX files using shared Office Open XML metadata extraction.
Integrates with `office_metadata` module for core/app/custom properties.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `core_properties` | `KreuzbergCoreProperties*` | `NULL` | Core properties from docProps/core.xml (Dublin Core metadata) Contains title, creator, subject, keywords, dates, etc. Shared format across DOCX/PPTX/XLSX documents. |
| `app_properties` | `KreuzbergDocxAppProperties*` | `NULL` | Application properties from docProps/app.xml (Word-specific statistics) Contains word count, page count, paragraph count, editing time, etc. DOCX-specific variant of Office application properties. |
| `custom_properties` | `void**` | `NULL` | Custom properties from docProps/custom.xml (user-defined properties) Contains key-value pairs defined by users or applications. Values can be strings, numbers, booleans, or dates. |

---

#### KreuzbergElement

Semantic element extracted from document.

Represents a logical unit of content with semantic classification,
unique identifier, and metadata for tracking origin and position.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `element_type` | `KreuzbergElementType` | — | Semantic type of this element |
| `text` | `const char*` | — | Text content of the element |
| `metadata` | `KreuzbergElementMetadata` | — | Metadata about the element |

---

#### KreuzbergElementMetadata

Metadata for a semantic element.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `page_number` | `uint32_t*` | `NULL` | Page number (1-indexed) |
| `filename` | `const char**` | `NULL` | Source filename or document name |
| `coordinates` | `KreuzbergBoundingBox*` | `NULL` | Bounding box coordinates if available |
| `element_index` | `uintptr_t*` | `NULL` | Position index in the element sequence |
| `additional` | `void*` | — | Additional custom metadata |

---

#### KreuzbergEmailAttachment

Email attachment representation.

Contains metadata and optionally the content of an email attachment.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `const char**` | `NULL` | Attachment name (from Content-Disposition header) |
| `filename` | `const char**` | `NULL` | Filename of the attachment |
| `mime_type` | `const char**` | `NULL` | MIME type of the attachment |
| `size` | `uintptr_t*` | `NULL` | Size in bytes |
| `is_image` | `bool` | — | Whether this attachment is an image |
| `data` | `const uint8_t**` | `NULL` | Attachment data (if extracted). Uses `bytes.Bytes` for cheap cloning of large buffers. |

---

#### KreuzbergEmailConfig

Configuration for email extraction.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `msg_fallback_codepage` | `uint32_t*` | `NULL` | Windows codepage number to use when an MSG file contains no codepage property. Defaults to `NULL`, which falls back to windows-1252. If an unrecognized or invalid codepage number is supplied (including 0), the behavior silently falls back to windows-1252 — the same as when the MSG file itself contains an unrecognized codepage. No error or warning is emitted. Users should verify output when supplying unusual values. Common values: - 1250: Central European (Polish, Czech, Hungarian, etc.) - 1251: Cyrillic (Russian, Ukrainian, Bulgarian, etc.) - 1252: Western European (default) - 1253: Greek - 1254: Turkish - 1255: Hebrew - 1256: Arabic - 932:  Japanese (Shift-JIS) - 936:  Simplified Chinese (GBK) |

---

#### KreuzbergEmailExtractionResult

Email extraction result.

Complete representation of an extracted email message (.eml or .msg)
including headers, body content, and attachments.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `subject` | `const char**` | `NULL` | Email subject line |
| `from_email` | `const char**` | `NULL` | Sender email address |
| `to_emails` | `const char**` | — | Primary recipient email addresses |
| `cc_emails` | `const char**` | — | CC recipient email addresses |
| `bcc_emails` | `const char**` | — | BCC recipient email addresses |
| `date` | `const char**` | `NULL` | Email date/timestamp |
| `message_id` | `const char**` | `NULL` | Message-ID header value |
| `plain_text` | `const char**` | `NULL` | Plain text version of the email body |
| `html_content` | `const char**` | `NULL` | HTML version of the email body |
| `content` | `const char*` | — | Cleaned/processed text content. Aliased as `cleaned_text` for back-compat. |
| `attachments` | `KreuzbergEmailAttachment*` | — | List of email attachments |
| `metadata` | `void*` | — | Additional email headers and metadata |

---

#### KreuzbergEmailMetadata

Email metadata extracted from .eml and .msg files.

Includes sender/recipient information, message ID, and attachment list.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `from_email` | `const char**` | `NULL` | Sender's email address |
| `from_name` | `const char**` | `NULL` | Sender's display name |
| `to_emails` | `const char**` | `NULL` | Primary recipients |
| `cc_emails` | `const char**` | `NULL` | CC recipients |
| `bcc_emails` | `const char**` | `NULL` | BCC recipients |
| `message_id` | `const char**` | `NULL` | Message-ID header value |
| `attachments` | `const char**` | `NULL` | List of attachment filenames |

---

#### KreuzbergEmbeddedChanges

Changes to embedded archive children between two results.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `added` | `KreuzbergArchiveEntry*` | — | Children present in `b` but not in `a` (matched by `path`). |
| `removed` | `KreuzbergArchiveEntry*` | — | Children present in `a` but not in `b` (matched by `path`). |
| `changed` | `KreuzbergEmbeddedDiff*` | — | Children present in both but with differing content (matched by `path`). Each entry holds the diff of the nested `ExtractionResult`. |

---

#### KreuzbergEmbeddedDiff

Diff for a single embedded archive entry that appears in both results.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `path` | `const char*` | — | Archive-relative path identifying this entry. |
| `diff` | `KreuzbergExtractionDiff` | — | The recursive diff of the entry's extraction result. |

---

#### KreuzbergEmbeddedFile

Embedded file descriptor extracted from the PDF name tree.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `const char*` | — | The filename as stored in the PDF name tree. |
| `data` | `const uint8_t*` | — | Raw file bytes from the embedded stream (already decompressed by lopdf). |
| `compressed_size` | `uintptr_t` | — | Compressed byte count of the original stream (before decompression). Used by callers to compute the decompression ratio and detect zip-bomb-style attacks that embed a tiny compressed stream expanding to gigabytes of data. |
| `mime_type` | `const char**` | `NULL` | MIME type if specified in the filespec, otherwise `NULL`. |

---

#### KreuzbergEmbeddingBackend

Trait for in-process embedding backend plugins.

Async to match the convention used by `OcrBackend`,
`DocumentExtractor`, and `PostProcessor`.
Host-language bridges (PyO3, napi-rs, Rustler, extendr, magnus, ext-php-rs,
C FFI, etc.) wrap their synchronous host callables in `spawn_blocking` or the
equivalent to satisfy the async signature.

##### Thread safety

Backends must be `Send + Sync + 'static`. They are stored in
`Arc<dyn EmbeddingBackend>` and called concurrently from kreuzberg's chunking
pipeline. If the backend's underlying model isn't thread-safe, the backend
itself must serialize access internally (e.g. via `Mutex<Inner>`).

##### Contract

- `embed(texts)` MUST return exactly `texts.len()` vectors, each of length
  `self.dimensions()`. The dispatcher in `crate.embeddings.embed_texts`
  validates this before returning to downstream consumers; a non-conforming
  backend surfaces as a `KreuzbergError.Validation`, not a panic.

- `embed` may be called from any thread. Its future must be `Send`
  (enforced by `async_trait` when `#[async_trait]` is used on non-WASM targets).

- `dimensions()` is called exactly once at registration, immediately after
  `initialize()` succeeds. The returned value is cached by the registry and
  used for all subsequent shape validation. Lazy-loading implementations can
  defer model loading into `initialize()` and report the real dimension
  afterwards. Later mutations of the backend's reported dimension are not
  observed by kreuzberg — implementations that need to change dimension
  must unregister and re-register.

- `shutdown()` (inherited from `Plugin`) may be invoked
  concurrently with an in-flight `embed()` call. Implementations must
  tolerate this — e.g. by letting in-flight calls finish using resources
  held via the `Arc<dyn EmbeddingBackend>` reference, and only releasing
  shared state that isn't needed by `embed`.

##### Runtime

The synchronous `embed_texts` entry uses
`tokio.task.block_in_place` to await the trait's async `embed`, which
requires a multi-thread tokio runtime. Callers running inside a
`current_thread` runtime (e.g. `#[tokio.test]` without `flavor = "multi_thread"`,
or `tokio.runtime.Builder.new_current_thread()`) must use
`embed_texts_async` instead, which awaits directly without `block_in_place`.

##### Methods

###### kreuzberg_dimensions()

Embedding vector dimension. Must be `> 0` and must match the length of
every vector returned by `embed`.

**Signature:**

```c
uintptr_t kreuzberg_dimensions();
```

**Example:**

```c
uintptr_t result = kreuzberg_dimensions(instance);
```

**Returns:** `uintptr_t`

###### kreuzberg_embed()

Embed a batch of texts, returning one vector per input in order.

**Errors:**

Implementations should return `Plugin` for
backend-specific failures. The dispatcher layers its own validation
(length, per-vector dimension) on top.

**Signature:**

```c
float** kreuzberg_embed(const char** texts);
```

**Example:**

```c
float** result = kreuzberg_embed(instance, NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `texts` | `const char**` | Yes | The texts |

**Returns:** `float**`

**Errors:** Returns `NULL` on error.

---

#### KreuzbergEmbeddingConfig

Embedding configuration for text chunks.

Configures embedding generation using ONNX models via the vendored embedding engine.
Requires the `embeddings` feature to be enabled.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `KreuzbergEmbeddingModelType` | `KREUZBERG_KREUZBERG_PRESET` | The embedding model to use (defaults to "balanced" preset if not specified) |
| `normalize` | `bool` | `true` | Whether to normalize embedding vectors (recommended for cosine similarity) |
| `batch_size` | `uintptr_t` | `32` | Batch size for embedding generation |
| `show_download_progress` | `bool` | `false` | Show model download progress |
| `cache_dir` | `const char**` | `NULL` | Custom cache directory for model files Defaults to `~/.cache/kreuzberg/embeddings/` if not specified. Allows full customization of model download location. |
| `acceleration` | `KreuzbergAccelerationConfig*` | `NULL` | Hardware acceleration for the embedding ONNX model. When set, controls which execution provider (CPU, CUDA, CoreML, TensorRT) is used for inference. Defaults to `NULL` (auto-select per platform). |
| `max_embed_duration_secs` | `uint64_t*` | `NULL` | Maximum wall-clock duration (in seconds) for a single `embed()` call when using `EmbeddingModelType.Plugin`. Applies only to the in-process plugin path — protects against hung host-language backends (e.g. a Python callback deadlocked on the GIL, a model stuck on CUDA OOM retries, etc.). On timeout, the dispatcher returns `Plugin` instead of blocking forever. `NULL` disables the timeout. The default (60 seconds) is conservative for common in-process inference; increase for large batches on slow hardware. |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergEmbeddingConfig kreuzberg_default();
```

**Example:**

```c
KreuzbergEmbeddingConfig *result = kreuzberg_default();
```

**Returns:** `KreuzbergEmbeddingConfig`

---

#### KreuzbergEmbeddingPreset

Preset configurations for common RAG use cases.

Each preset combines chunk size, overlap, and embedding model
to provide an optimized configuration for specific scenarios.

All string fields are owned `String` for FFI compatibility — instances
are safe to clone and pass across language boundaries.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `const char*` | — | Short identifier for this preset (e.g. `"balanced"`, `"fast"`, `"quality"`). |
| `chunk_size` | `uintptr_t` | — | Target chunk size in characters. |
| `overlap` | `uintptr_t` | — | Overlap between consecutive chunks in characters. |
| `model_repo` | `const char*` | — | HuggingFace repository name for the model. |
| `pooling` | `const char*` | — | Pooling strategy: "cls" or "mean". |
| `model_file` | `const char*` | — | Path to the ONNX model file within the repo. |
| `dimensions` | `uintptr_t` | — | Embedding vector dimension produced by this model. |
| `description` | `const char*` | — | Human-readable description of the preset's intended use case. |

---

#### KreuzbergEnrichOptions

Which enrichment passes to run on a piece of text.

All fields default to `false` / empty so callers can opt in precisely.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `keywords` | `bool` | — | Run keyword extraction on the input text. When `true`, the enrichment backend identifies the most salient terms and returns them in `EnrichResult.keywords`. |
| `entities` | `bool` | — | Run named-entity recognition (NER) on the input text. When `true`, the enrichment backend identifies named entities (persons, organisations, locations, etc.) and returns them in `EnrichResult.entities`. |
| `labels` | `const char**` | `NULL` | Custom labels to pass through to the result without modification. These are caller-supplied tags that the enrichment pipeline propagates verbatim into `EnrichResult.labels`. Useful for attaching project- or document-level metadata to every enrichment result. |

---

#### KreuzbergEnrichResult

Structured output produced by a completed enrichment pass.

Fields are populated only when the corresponding `EnrichOptions` flag was set.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `keywords` | `const char**` | `NULL` | Salient terms extracted from the text. Populated when `EnrichOptions.keywords` was `true`. The ordering is backend-defined (typically by descending relevance score). |
| `entities` | `KreuzbergEntity*` | `NULL` | Named entities found in the text. Populated when `EnrichOptions.entities` was `true`. Uses the shared OSS entity schema (`Entity` / `EntityCategory`) so consumers can pattern-match on entity categories without JSON gymnastics. |
| `labels` | `const char**` | `NULL` | Caller-supplied labels echoed from `EnrichOptions.labels`. |

---

#### KreuzbergEntity

A single named entity detected in the extracted text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `category` | `KreuzbergEntityCategory` | — | Canonical category the entity belongs to (PERSON, ORG, LOCATION, etc.). |
| `text` | `const char*` | — | Raw mention text exactly as it appeared in the source. |
| `start` | `uint32_t` | — | Byte-offset span in `ExtractionResult.content` where the mention starts. |
| `end` | `uint32_t` | — | Byte-offset span in `ExtractionResult.content` where the mention ends (exclusive). |
| `confidence` | `float*` | `NULL` | Backend-reported confidence in `\[0.0, 1.0\]`. `NULL` when the backend does not expose confidence scores. |

---

#### KreuzbergEpubMetadata

EPUB metadata (Dublin Core extensions).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `coverage` | `const char**` | `NULL` | Dublin Core `coverage` field (geographic or temporal scope). |
| `dc_format` | `const char**` | `NULL` | Dublin Core `format` field (media type of the resource). |
| `relation` | `const char**` | `NULL` | Dublin Core `relation` field (related resource identifier). |
| `source` | `const char**` | `NULL` | Dublin Core `source` field (origin resource identifier). |
| `dc_type` | `const char**` | `NULL` | Dublin Core `type` field (nature or genre of the resource). |
| `cover_image` | `const char**` | `NULL` | Path or identifier of the cover image within the EPUB container. |

---

#### KreuzbergErrorMetadata

Error metadata (for batch operations).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `error_type` | `const char*` | — | Machine-readable error type identifier (e.g. "UnsupportedFormat"). |
| `message` | `const char*` | — | Human-readable error description. |

---

#### KreuzbergExcelMetadata

Excel/spreadsheet format metadata.

Identifies the document as a spreadsheet source via the `FormatMetadata.Excel`
discriminant. Sheet count and sheet names are stored inside this struct.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `sheet_count` | `uint32_t*` | `NULL` | Number of sheets in the workbook. |
| `sheet_names` | `const char***` | `NULL` | Names of all sheets in the workbook. |

---

#### KreuzbergExcelSheet

Single Excel worksheet.

Represents one sheet from an Excel workbook with its content
converted to Markdown format and dimensional statistics.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `const char*` | — | Sheet name as it appears in Excel |
| `markdown` | `const char*` | — | Sheet content converted to Markdown tables |
| `row_count` | `uintptr_t` | — | Number of rows |
| `col_count` | `uintptr_t` | — | Number of columns |
| `cell_count` | `uintptr_t` | — | Total number of non-empty cells |
| `table_cells` | `const char****` | `NULL` | Pre-extracted table cells (2D vector of cell values) Populated during markdown generation to avoid re-parsing markdown. None for empty sheets. |

---

#### KreuzbergExcelWorkbook

Excel workbook representation.

Contains all sheets from an Excel file (.xlsx, .xls, etc.) with
extracted content and metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `sheets` | `KreuzbergExcelSheet*` | — | All sheets in the workbook |
| `metadata` | `void*` | — | Workbook-level metadata (author, creation date, etc.) |
| `revisions` | `KreuzbergDocumentRevision**` | `/* serde(default) */` | Collaborative-edit revision headers from `xl/revisions/revisionHeaders.xml`. Populated for legacy shared-workbook `.xlsx` files that contain the `xl/revisions/` directory. Each `<header>` element maps to one `DocumentRevision { kind: FormatChange }` carrying the header's `guid` (→ `revision_id`), `userName` (→ `author`), and `dateTime` (→ `timestamp`). `anchor` and `delta` are `NULL`/empty for v1 (per-cell log parsing is a follow-up). `NULL` when `xl/revisions/revisionHeaders.xml` is absent. |

---

#### KreuzbergExtractedImage

Extracted image from a document.

Contains raw image data, metadata, and optional nested OCR results.
Raw bytes allow cross-language compatibility - users can convert to
PIL.Image (Python), Sharp (Node.js), or other formats as needed.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `data` | `const uint8_t*` | — | Raw image data (PNG, JPEG, WebP, etc. bytes). Uses `bytes.Bytes` for cheap cloning of large buffers. |
| `format` | `const char*` | — | Image format (e.g., "jpeg", "png", "webp") Uses Cow<'static, str> to avoid allocation for static literals. |
| `image_index` | `uint32_t` | — | Zero-indexed position of this image in the document/page |
| `page_number` | `uint32_t*` | `NULL` | Page/slide number where image was found (1-indexed) |
| `width` | `uint32_t*` | `NULL` | Image width in pixels |
| `height` | `uint32_t*` | `NULL` | Image height in pixels |
| `colorspace` | `const char**` | `NULL` | Colorspace information (e.g., "RGB", "CMYK", "Gray") |
| `bits_per_component` | `uint32_t*` | `NULL` | Bits per color component (e.g., 8, 16) |
| `is_mask` | `bool` | — | Whether this image is a mask image |
| `description` | `const char**` | `NULL` | Optional description of the image |
| `ocr_result` | `KreuzbergExtractionResult*` | `NULL` | Nested OCR extraction result (if image was OCRed) When OCR is performed on this image, the result is embedded here rather than in a separate collection, making the relationship explicit. |
| `bounding_box` | `KreuzbergBoundingBox*` | `NULL` | Bounding box of the image on the page (PDF coordinates: x0=left, y0=bottom, x1=right, y1=top). Only populated for PDF-extracted images when position data is available from the PDF extractor. |
| `source_path` | `const char**` | `NULL` | Original source path of the image within the document archive (e.g., "media/image1.png" in DOCX). Used for rendering image references when the binary data is not extracted. |
| `image_kind` | `KreuzbergImageKind*` | `NULL` | Heuristic classification of what this image likely depicts. `NULL` if classification was disabled or inconclusive. |
| `kind_confidence` | `float*` | `NULL` | Confidence score for `image_kind`, in the range 0.0 to 1.0. |
| `cluster_id` | `uint32_t*` | `NULL` | Identifier shared across images that form a single logical figure (e.g. all raster tiles of one technical drawing). `NULL` for singletons. |
| `caption` | `const char**` | `NULL` | VLM-generated caption describing the image, when captioning is configured. Populated by the captioning post-processor (`crates/kreuzberg/src/plugins/processor/builtin/captioning.rs`), which routes each image through `crate.llm.region_extractor.extract_region_with_vlm` in caption mode. `NULL` when captioning is disabled or the VLM declined to caption. |
| `qr_codes` | `KreuzbergQrCode**` | `NULL` | QR codes decoded from this image, when QR detection is enabled. Populated by the QR post-processor (`crates/kreuzberg/src/extractors/qr.rs`) via the pure-Rust `rqrr` decoder. `NULL` when QR detection is disabled; an empty `Some(\[\])` when detection ran but found nothing. |

---

#### KreuzbergExtractedUri

A URI extracted from a document.

Represents any link, reference, or resource pointer found during extraction.
The `kind` field classifies the URI semantically, while `label` carries
optional human-readable display text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | `const char*` | — | The URL or path string. |
| `label` | `const char**` | `NULL` | Optional display text / label for the link. |
| `page` | `uint32_t*` | `NULL` | Optional page number where the URI was found (1-indexed). |
| `kind` | `KreuzbergUriKind` | — | Semantic classification of the URI. |

---

#### KreuzbergExtractionConfidence

Combined confidence on `[0, 1]`.

When OCR did not run, the `ocr_aggregate` weight folds into `text_coverage`
so the weighted sum still totals 1.0.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text_coverage` | `float` | — | Fraction of pages with a usable text layer. |
| `ocr_aggregate` | `float*` | `NULL` | Mean OCR per-element recognition confidence when OCR ran; `NULL` when it did not. |
| `schema_compliance` | `KreuzbergSchemaCompliance` | — | Whether the merged output validates against the preset schema. |
| `combined` | `float` | — | Weighted blend in `\[0, 1\]`.  The value compared against the fallback threshold. |

---

#### KreuzbergExtractionConfig

Main extraction configuration.

This struct contains all configuration options for the extraction process.
It can be loaded from TOML, YAML, or JSON files, or created programmatically.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `use_cache` | `bool` | `true` | Enable caching of extraction results |
| `enable_quality_processing` | `bool` | `true` | Enable quality post-processing |
| `ocr` | `KreuzbergOcrConfig*` | `NULL` | OCR configuration (None = OCR disabled) |
| `force_ocr` | `bool` | `false` | Force OCR even for searchable PDFs |
| `force_ocr_pages` | `uint32_t**` | `NULL` | Force OCR on specific pages only (1-indexed page numbers, must be >= 1). When set, only the listed pages are OCR'd regardless of text layer quality. Unlisted pages use native text extraction. Ignored when `force_ocr` is `true`. Only applies to PDF documents. Duplicates are automatically deduplicated. An `ocr` config is recommended for backend/language selection; defaults are used if absent. |
| `disable_ocr` | `bool` | `false` | Disable OCR entirely, even for images. When `true`, OCR is skipped for all document types. Images return metadata only (dimensions, format, EXIF) without text extraction. PDFs use only native text extraction without OCR fallback. Cannot be `true` simultaneously with `force_ocr`. *Added in v4.7.0.* |
| `chunking` | `KreuzbergChunkingConfig*` | `NULL` | Text chunking configuration (None = chunking disabled) |
| `content_filter` | `KreuzbergContentFilterConfig*` | `NULL` | Content filtering configuration (None = use extractor defaults). Controls whether document "furniture" (headers, footers, watermarks, repeating text) is included in or stripped from extraction results. See `ContentFilterConfig` for per-field documentation. |
| `images` | `KreuzbergImageExtractionConfig*` | `NULL` | Image extraction configuration (None = no image extraction) |
| `pdf_options` | `KreuzbergPdfConfig*` | `NULL` | PDF-specific options (None = use defaults) |
| `token_reduction` | `KreuzbergTokenReductionOptions*` | `NULL` | Token reduction configuration (None = no token reduction) |
| `language_detection` | `KreuzbergLanguageDetectionConfig*` | `NULL` | Language detection configuration (None = no language detection) |
| `pages` | `KreuzbergPageConfig*` | `NULL` | Page extraction configuration (None = no page tracking) |
| `keywords` | `KreuzbergKeywordConfig*` | `NULL` | Keyword extraction configuration (None = no keyword extraction) |
| `postprocessor` | `KreuzbergPostProcessorConfig*` | `NULL` | Post-processor configuration (None = use defaults) |
| `html_output` | `KreuzbergHtmlOutputConfig*` | `NULL` | Styled HTML output configuration. When set alongside `output_format = OutputFormat.Html`, the extraction pipeline uses `StyledHtmlRenderer` which emits stable `kb-*` CSS class hooks on every structural element and optionally embeds theme CSS or user-supplied CSS in a `<style>` block. When `NULL`, the existing plain comrak-based HTML renderer is used. |
| `extraction_timeout_secs` | `uint64_t*` | `NULL` | Default per-file timeout in seconds for batch extraction. When set, each file in a batch will be canceled after this duration unless overridden by `FileExtractionConfig.timeout_secs`. Defaults to `Some(60)` to prevent pathological files (e.g. deeply nested archives, documents with millions of cells) from running indefinitely and exhausting caller resources. Set to `NULL` to disable the timeout for trusted input or long-running workloads. |
| `max_concurrent_extractions` | `uintptr_t*` | `NULL` | Maximum concurrent extractions in batch operations (None = (num_cpus × 1.5).ceil()). Limits parallelism to prevent resource exhaustion when processing large batches. Defaults to (num_cpus × 1.5).ceil() when not set. |
| `result_format` | `KreuzbergResultFormat` | `KREUZBERG_KREUZBERG_UNIFIED` | Result structure format Controls whether results are returned in unified format (default) with all content in the `content` field, or element-based format with semantic elements (for Unstructured-compatible output). |
| `security_limits` | `KreuzbergSecurityLimits*` | `NULL` | Security limits for archive extraction. Controls maximum archive size, compression ratio, file count, and other security thresholds to prevent decompression bomb attacks. Also caps nesting depth, iteration count, entity / token length, total content size, and table cell count for every extraction path that ingests user-controlled bytes. When `NULL`, default limits are used. |
| `max_embedded_file_bytes` | `uint64_t*` | `NULL` | Maximum uncompressed size in bytes for a single embedded file before recursive extraction is attempted (default: 50 MiB). Applies to embedded objects inside OOXML containers (DOCX, PPTX) and to email attachments processed via recursive extraction. Files that exceed this limit are skipped with a `ProcessingWarning` rather than passed to the extraction pipeline, preventing a single oversized embedded object from consuming unbounded memory or time. Set to `NULL` to disable the per-embedded-file cap (falls back to `security_limits.max_archive_size` as the only guard). |
| `output_format` | `KreuzbergOutputFormat` | `KREUZBERG_KREUZBERG_PLAIN` | Content text format (default: Plain). Controls the format of the extracted content: - `Plain`: Raw extracted text (default) - `Markdown`: Markdown formatted output - `Djot`: Djot markup format (requires djot feature) - `Html`: HTML formatted output When set to a structured format, extraction results will include formatted output. The `formatted_content` field may be populated when format conversion is applied. |
| `layout` | `KreuzbergLayoutDetectionConfig*` | `NULL` | Layout detection configuration (None = layout detection disabled). When set, PDF pages and images are analyzed for document structure (headings, code, formulas, tables, figures, etc.) using RT-DETR models via ONNX Runtime. For PDFs, layout hints override paragraph classification in the markdown pipeline. For images, per-region OCR is performed with markdown formatting based on detected layout classes. Requires the `layout-detection` feature to run inference; the field is present whenever the `layout-types` feature is active (which includes `layout-detection` as well as the no-ORT target groups). |
| `transcription` | `KreuzbergTranscriptionConfig*` | `NULL` | Transcription (speech-to-text) configuration for audio/video files. When set and `enabled`, files with audio/video MIME types (mp3, mp4, m4a, wav, webm, etc.) are routed to the Whisper-based transcription pipeline. The actual heavy dependencies are only active under the `transcription` feature; the field is visible under `transcription-types` (including on WASM and Android targets that use the no-ORT preset). Default: `NULL` (transcription disabled). This is an additive, non-breaking change. |
| `use_layout_for_markdown` | `bool` | `false` | Run layout detection on the non-OCR PDF markdown path. When `true` and `layout` is `Some(_)`, layout regions inform heading, table, list, and figure detection in the structure pipeline that would otherwise rely on font-clustering heuristics alone. Significantly improves SF1 (structural F1) at the cost of inference latency (~150-300ms/page CPU, ~20-50ms/page GPU). Default: `false`. Requires the `layout-detection` feature. |
| `include_document_structure` | `bool` | `false` | Enable structured document tree output. When true, populates the `document` field on `ExtractionResult` with a hierarchical `DocumentStructure` containing heading-driven section nesting, table grids, content layer classification, and inline annotations. Independent of `result_format` — can be combined with Unified or ElementBased. |
| `acceleration` | `KreuzbergAccelerationConfig*` | `NULL` | Hardware acceleration configuration for ONNX Runtime models. Controls execution provider selection for layout detection and embedding models. When `NULL`, uses platform defaults (CoreML on macOS, CUDA on Linux, CPU on Windows). |
| `cache_namespace` | `const char**` | `NULL` | Cache namespace for tenant isolation. When set, cache entries are stored under `{cache_dir}/{namespace}/`. Must be alphanumeric, hyphens, or underscores only (max 64 chars). Different namespaces have isolated cache spaces on the same filesystem. |
| `cache_ttl_secs` | `uint64_t*` | `NULL` | Per-request cache TTL in seconds. Overrides the global `max_age_days` for this specific extraction. When `0`, caching is completely skipped (no read or write). When `NULL`, the global TTL applies. |
| `email` | `KreuzbergEmailConfig*` | `NULL` | Email extraction configuration (None = use defaults). Currently supports configuring the fallback codepage for MSG files that do not specify one. See `EmailConfig` for details. |
| `max_archive_depth` | `uintptr_t` | — | Maximum recursion depth for archive extraction (default: 3). Set to 0 to disable recursive extraction (legacy behavior). |
| `tree_sitter` | `KreuzbergTreeSitterConfig*` | `NULL` | Tree-sitter language pack configuration (None = tree-sitter disabled). When set, enables code file extraction using tree-sitter parsers. Controls grammar download behavior and code analysis options. |
| `structured_extraction` | `KreuzbergStructuredExtractionConfig*` | `NULL` | Structured extraction via LLM (None = disabled). When set, the extracted document content is sent to an LLM with the provided JSON schema. The structured response is stored in `ExtractionResult.structured_output`. |
| `ner` | `KreuzbergNerConfig*` | `NULL` | Named-entity recognition configuration. When set, the NER post-processor runs at the Middle stage and populates `ExtractionResult.entities`. |
| `redaction` | `KreuzbergRedactionConfig*` | `NULL` | Redaction / anonymisation configuration. When set, the redaction post-processor runs at the Late stage and rewrites every textual field in `ExtractionResult`, emitting an audit trail in `ExtractionResult.redaction_report`. |
| `summarization` | `KreuzbergSummarizationConfig*` | `NULL` | Summarisation configuration. When set, the summarisation post-processor runs at the Middle stage and populates `ExtractionResult.summary`. |
| `translation` | `KreuzbergTranslationConfig*` | `NULL` | Translation configuration. When set, the translation post-processor runs at the Middle stage and populates `ExtractionResult.translation`. |
| `page_classification` | `KreuzbergPageClassificationConfig*` | `NULL` | Per-page classification configuration. When set, the classification post-processor runs at the Middle stage and populates `ExtractionResult.page_classifications`. |
| `captioning` | `KreuzbergCaptioningConfig*` | `NULL` | VLM captioning configuration for extracted images. When set, the captioning post-processor runs at the Middle stage and writes a caption into each `ExtractedImage.caption`. |
| `qr_codes` | `bool*` | `NULL` | Enable QR-code detection in extracted images. When `true`, the QR post-processor runs at the Middle stage and populates `ExtractedImage.qr_codes`. |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergExtractionConfig kreuzberg_default();
```

**Example:**

```c
KreuzbergExtractionConfig *result = kreuzberg_default();
```

**Returns:** `KreuzbergExtractionConfig`

###### kreuzberg_needs_image_data()

Check if image processing is needed by examining OCR and image extraction settings.

Returns `true` if either OCR is enabled or image extraction is configured,
indicating that image decompression and processing should occur.
Returns `false` if both are disabled, allowing optimization to skip unnecessary
image decompression for text-only extraction workflows.

##### Optimization Impact
For text-only extractions (no OCR, no image extraction), skipping image
decompression can improve CPU utilization by 5-10% by avoiding wasteful
image I/O and processing when results won't be used.
Returns `true` when image binary data should be extracted.

True when `config.images.extract_images` is set **or** when captioning is
configured — captioning requires image bytes regardless of whether the caller
also requested `images` extraction.

**Signature:**

```c
bool kreuzberg_needs_image_data();
```

**Example:**

```c
bool result = kreuzberg_needs_image_data(instance);
```

**Returns:** `bool`

###### kreuzberg_needs_image_processing()

Returns `true` when any image processing is needed during extraction.

##### Optimization Impact

For text-only extractions (no OCR, no image extraction, no captioning), skipping
image decompression can improve CPU utilization by 5-10% by avoiding wasteful
image I/O and processing when results won't be used.

**Signature:**

```c
bool kreuzberg_needs_image_processing();
```

**Example:**

```c
bool result = kreuzberg_needs_image_processing(instance);
```

**Returns:** `bool`

---

#### KreuzbergExtractionDiff

The complete diff between two `ExtractionResult` values.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content_diff` | `KreuzbergDiffHunk*` | — | Unified-diff hunks for the `content` field. Empty when the content is identical. |
| `tables_added` | `KreuzbergTable*` | — | Tables present in `b` but not in `a` (by index position, excess right-side tables). |
| `tables_removed` | `KreuzbergTable*` | — | Tables present in `a` but not in `b` (by index position, excess left-side tables). |
| `tables_changed` | `KreuzbergTableDiff*` | — | Cell-level changes for table pairs that share the same index and dimensions. |
| `metadata_changed` | `void*` | — | Metadata difference, encoded as a JSON object with three top-level keys: `added` (keys present in `b` but not `a`), `removed` (keys present in `a` but not `b`), and `changed` (keys whose values differ — each entry is `{ "from": <value-in-a>, "to": <value-in-b> }`). This is NOT RFC 6902 JSON Patch — we deliberately chose a flatter shape to avoid pulling in a json-patch crate. If you need RFC 6902 semantics (with JSON Pointer paths) feed `a.metadata` and `b.metadata` to your preferred json-patch impl directly. |
| `embedded_changes` | `KreuzbergEmbeddedChanges` | — | Changes to embedded archive children. |

---

#### KreuzbergExtractionResult

General extraction result used by the core extraction API.

This is the main result type returned by all extraction functions.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `const char*` | — | Plain-text representation of the extracted document content. |
| `mime_type` | `const char*` | — | MIME type of the source document (e.g. `"application/pdf"`). |
| `metadata` | `KreuzbergMetadata` | — | Document-level metadata (author, title, dates, format-specific fields). |
| `extraction_method` | `KreuzbergExtractionMethod*` | `NULL` | Extraction strategy used to produce the returned text. Populated when the extractor can reliably distinguish native text extraction, OCR-only extraction, or mixed native/OCR output. |
| `tables` | `KreuzbergTable*` | `NULL` | Tables extracted from the document, each with structured cell data. |
| `detected_languages` | `const char***` | `NULL` | ISO 639-1 language codes detected in the document content. |
| `chunks` | `KreuzbergChunk**` | `NULL` | Text chunks when chunking is enabled. When chunking configuration is provided, the content is split into overlapping chunks for efficient processing. Each chunk contains the text, optional embeddings (if enabled), and metadata about its position. |
| `images` | `KreuzbergExtractedImage**` | `NULL` | Extracted images from the document. When image extraction is enabled via `ImageExtractionConfig`, this field contains all images found in the document with their raw data and metadata. Each image may optionally contain a nested `ocr_result` if OCR was performed. |
| `pages` | `KreuzbergPageContent**` | `NULL` | Per-page content when page extraction is enabled. When page extraction is configured, the document is split into per-page content with tables and images mapped to their respective pages. |
| `elements` | `KreuzbergElement**` | `NULL` | Semantic elements when element-based result format is enabled. When result_format is set to ElementBased, this field contains semantic elements with type classification, unique identifiers, and metadata for Unstructured-compatible element-based processing. |
| `djot_content` | `KreuzbergDjotContent*` | `NULL` | Rich Djot content structure (when extracting Djot documents). When extracting Djot documents with structured extraction enabled, this field contains the full semantic structure including: - Block-level elements with nesting - Inline formatting with attributes - Links, images, footnotes - Math expressions - Complete attribute information The `content` field still contains plain text for backward compatibility. Always `NULL` for non-Djot documents. |
| `ocr_elements` | `KreuzbergOcrElement**` | `NULL` | OCR elements with full spatial and confidence metadata. When OCR is performed with element extraction enabled, this field contains the structured representation of detected text including: - Bounding geometry (rectangles or quadrilaterals) - Confidence scores (detection and recognition) - Rotation information - Hierarchical relationships (Tesseract only) This field preserves all metadata that would otherwise be lost when converting to plain text or markdown output formats. Only populated when `OcrElementConfig.include_elements` is true. |
| `document` | `KreuzbergDocumentStructure*` | `NULL` | Structured document tree (when document structure extraction is enabled). When `include_document_structure` is true in `ExtractionConfig`, this field contains the full hierarchical representation of the document including: - Heading-driven section nesting - Table grids with cell-level metadata - Content layer classification (body, header, footer, footnote) - Inline text annotations (formatting, links) - Bounding boxes and page numbers Independent of `result_format` — can be combined with Unified or ElementBased. |
| `extracted_keywords` | `KreuzbergKeyword**` | `NULL` | Extracted keywords when keyword extraction is enabled. When keyword extraction (RAKE or YAKE) is configured, this field contains the extracted keywords with scores, algorithm info, and position data. Previously stored in `metadata.additional\["keywords"\]`. |
| `quality_score` | `double*` | `NULL` | Document quality score from quality analysis. A value between 0.0 and 1.0 indicating the overall text quality. Previously stored in `metadata.additional\["quality_score"\]`. |
| `processing_warnings` | `KreuzbergProcessingWarning*` | `NULL` | Non-fatal warnings collected during processing pipeline stages. Captures errors from optional pipeline features (embedding, chunking, language detection, output formatting) that don't prevent extraction but may indicate degraded results. Previously stored as individual keys in `metadata.additional`. |
| `annotations` | `KreuzbergPdfAnnotation**` | `NULL` | PDF annotations extracted from the document. When annotation extraction is enabled via `PdfConfig.extract_annotations`, this field contains text notes, highlights, links, stamps, and other annotations found in PDF documents. |
| `children` | `KreuzbergArchiveEntry**` | `NULL` | Nested extraction results from archive contents. When extracting archives, each processable file inside produces its own full extraction result. Set to `NULL` for non-archive formats. Use `max_archive_depth` in config to control recursion depth. |
| `uris` | `KreuzbergExtractedUri**` | `NULL` | URIs/links discovered during document extraction. Contains hyperlinks, image references, citations, email addresses, and other URI-like references found in the document. Always extracted when present in the source document. |
| `revisions` | `KreuzbergDocumentRevision**` | `NULL` | Tracked changes embedded in the source document. Populated by per-format extractors that understand change-tracking metadata (DOCX `w:ins`/`w:del`/`w:rPrChange`, ODT `text:change-*`, …). Every extractor defaults to `NULL` until its format-specific implementation is added. Extractors that do populate this field follow the "accepted-changes" convention: inserted text is present in `content`, deleted text is absent — the revision list is the separate audit trail. |
| `structured_output` | `void**` | `NULL` | Structured extraction output from LLM-based JSON schema extraction. When `structured_extraction` is configured in `ExtractionConfig`, the extracted document content is sent to a VLM with the provided JSON schema. The response is parsed and stored here as a JSON value matching the schema. |
| `code_intelligence` | `void**` | `NULL` | Code intelligence results from tree-sitter analysis. Populated when extracting source code files with the `tree-sitter` feature. Contains metrics, structural analysis, imports/exports, comments, docstrings, symbols, diagnostics, and optionally chunked code segments. Stored as an opaque JSON value so that all language bindings (Go, Java, C#, …) can deserialize it as a raw JSON object rather than a typed struct. The underlying type is `tree_sitter_language_pack.ProcessResult`. |
| `llm_usage` | `KreuzbergLlmUsage**` | `NULL` | LLM token usage and cost data for all LLM calls made during this extraction. Contains one entry per LLM call. Multiple entries are produced when VLM OCR, structured extraction, or LLM embeddings run during the same extraction. `NULL` when no LLM was used. |
| `entities` | `KreuzbergEntity**` | `NULL` | Named entities detected in `content` by the NER post-processor. `NULL` when no NER backend is configured. Populated by the gline-rs ONNX backend or the LLM-driven backend (see `crates/kreuzberg/src/text/ner/`). |
| `summary` | `KreuzbergDocumentSummary*` | `NULL` | Summary of `content` produced by the summarisation post-processor. `NULL` when summarisation is not configured. Populated by the TextRank extractive backend (deterministic, no external service) or by the liter-llm-driven abstractive backend. |
| `translation` | `KreuzbergTranslation*` | `NULL` | Translation of `content` produced by the translation post-processor. `NULL` when translation is not configured. |
| `page_classifications` | `KreuzbergPageClassification**` | `NULL` | Per-page classifications produced by the page-classification post-processor. `NULL` when classification is not configured. |
| `redaction_report` | `KreuzbergRedactionReport*` | `NULL` | Audit report of redactions applied by the redaction post-processor. The redaction processor rewrites `content`, `formatted_content`, every chunk's text, and the textual fields of `entities` / `summary` / `translation` / `page_classifications` in place. This report describes what was found and how it was replaced. `NULL` when redaction is not configured. |
| `formulas` | `KreuzbergFormula*` | `NULL` | Mathematical formulas recognized in the document. Populated by the layout-guided formula pipeline when the `layout-detection` feature is enabled and the document contains regions classified as formulas. Empty otherwise. |
| `form_fields` | `KreuzbergPdfFormField*` | `NULL` | Form fields extracted from a PDF's AcroForm or XFA structure. Populated by the PDF extractor when `PdfConfig.extract_form_fields` is enabled (default) and the document is a fillable form. Empty otherwise. |
| `formatted_content` | `const char**` | `NULL` | Pre-rendered content in the requested output format. Populated during `derive_extraction_result` before tree derivation consumes element data. `apply_output_format` swaps this into `content` at the end of the pipeline, after post-processors have operated on plain text. |

##### Methods

###### kreuzberg_from_ocr()

Convert from an OCR result.

**Signature:**

```c
KreuzbergExtractionResult kreuzberg_from_ocr(KreuzbergOcrExtractionResult ocr);
```

**Example:**

```c
KreuzbergExtractionResult *result = kreuzberg_from_ocr((KreuzbergOcrExtractionResult){0});
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `ocr` | `KreuzbergOcrExtractionResult` | Yes | The ocr extraction result |

**Returns:** `KreuzbergExtractionResult`

---

#### KreuzbergFictionBookMetadata

FictionBook (FB2) metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `genres` | `const char**` | `NULL` | Genre tags as declared in the FB2 `<genre>` elements. |
| `sequences` | `const char**` | `NULL` | Book series (sequence) names, if any. |
| `annotation` | `const char**` | `NULL` | Short annotation / summary from the FB2 `<annotation>` element. |

---

#### KreuzbergFileExtractionConfig

Per-file extraction configuration overrides for batch processing.

All fields are `Option<T>` — `NULL` means "use the batch-level default."
This type is used with `batch_extract_files` and
`batch_extract_bytes` to allow heterogeneous
extraction settings within a single batch.

##### Excluded Fields

The following `ExtractionConfig` fields are batch-level only and
cannot be overridden per file:

- `max_concurrent_extractions` — controls batch parallelism
- `use_cache` — global caching policy
- `acceleration` — shared ONNX execution provider
- `security_limits` — global archive security policy

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enable_quality_processing` | `bool*` | `NULL` | Override quality post-processing for this file. |
| `ocr` | `KreuzbergOcrConfig*` | `NULL` | Override OCR configuration for this file (None in the Option = use batch default). |
| `force_ocr` | `bool*` | `NULL` | Override force OCR for this file. |
| `force_ocr_pages` | `uint32_t**` | `NULL` | Override force OCR pages for this file (1-indexed page numbers). |
| `disable_ocr` | `bool*` | `NULL` | Override disable OCR for this file. |
| `chunking` | `KreuzbergChunkingConfig*` | `NULL` | Override chunking configuration for this file. |
| `content_filter` | `KreuzbergContentFilterConfig*` | `NULL` | Override content filtering configuration for this file. |
| `images` | `KreuzbergImageExtractionConfig*` | `NULL` | Override image extraction configuration for this file. |
| `pdf_options` | `KreuzbergPdfConfig*` | `NULL` | Override PDF options for this file. |
| `token_reduction` | `KreuzbergTokenReductionOptions*` | `NULL` | Override token reduction for this file. |
| `language_detection` | `KreuzbergLanguageDetectionConfig*` | `NULL` | Override language detection for this file. |
| `pages` | `KreuzbergPageConfig*` | `NULL` | Override page extraction for this file. |
| `keywords` | `KreuzbergKeywordConfig*` | `NULL` | Override keyword extraction for this file. |
| `postprocessor` | `KreuzbergPostProcessorConfig*` | `NULL` | Override post-processor for this file. |
| `result_format` | `KreuzbergResultFormat*` | `NULL` | Override result format for this file. |
| `output_format` | `KreuzbergOutputFormat*` | `NULL` | Override output content format for this file. |
| `include_document_structure` | `bool*` | `NULL` | Override document structure output for this file. |
| `layout` | `KreuzbergLayoutDetectionConfig*` | `NULL` | Override layout detection for this file. |
| `transcription` | `KreuzbergTranscriptionConfig*` | `NULL` | Transcription configuration (see ExtractionConfig for docs). |
| `timeout_secs` | `uint64_t*` | `NULL` | Override per-file extraction timeout in seconds. When set, the extraction for this file will be canceled after the specified duration. A timed-out file produces an error result without affecting other files in the batch. |
| `tree_sitter` | `KreuzbergTreeSitterConfig*` | `NULL` | Override tree-sitter configuration for this file. |
| `structured_extraction` | `KreuzbergStructuredExtractionConfig*` | `NULL` | Override structured extraction configuration for this file. When set, enables LLM-based structured extraction with a JSON schema for this specific file. The extracted content is sent to a VLM/LLM and the response is parsed according to the provided schema. |

---

#### KreuzbergFootnote

Footnote in Djot.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `label` | `const char*` | — | Footnote label |
| `content` | `KreuzbergFormattedBlock*` | — | Footnote content blocks |

---

#### KreuzbergFormattedBlock

Block-level element in a Djot document.

Represents structural elements like headings, paragraphs, lists, code blocks, etc.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `block_type` | `KreuzbergBlockType` | — | Type of block element |
| `level` | `uintptr_t*` | `NULL` | Heading level (1-6) for headings, or nesting level for lists |
| `inline_content` | `KreuzbergInlineElement*` | — | Inline content within the block |
| `language` | `const char**` | `NULL` | Language identifier for code blocks |
| `code` | `const char**` | `NULL` | Raw code content for code blocks |
| `children` | `KreuzbergFormattedBlock*` | `/* serde(default) */` | Nested blocks for containers (blockquotes, list items, divs) |

---

#### KreuzbergFormula

A mathematical formula detected and recognized in a document.

Populated by the layout-guided formula pipeline: regions classified as
`LayoutClass.Formula` are routed to the formula OCR task, which returns the
LaTeX source for the region. The field is always present on
`ExtractionResult` but only populated
when the `layout-detection` feature is active and the document contains
formula regions.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `latex` | `const char*` | — | LaTeX source of the recognized formula, without surrounding `$$` delimiters. This field contains the raw LaTeX code as produced by the OCR backend. To render the formula in Markdown or other formats, wrap with `$$..$$` delimiters as needed. |
| `bbox` | `KreuzbergBoundingBox` | — | Bounding box of the formula region on its page, in rendered-image pixel coordinates. The coordinates are in the space of the OCR-rendered page image at the OCR DPI (typically 300 DPI). These coordinates are NOT comparable to bounding boxes from native PDF text extraction, which use PDF point coordinates. |
| `page` | `uint32_t` | — | 1-indexed page number the formula appears on in the document. This is set by the extraction pipeline based on which page the formula was found on. |

---

#### KreuzbergGridCell

Individual grid cell with position and span metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `const char*` | — | Cell text content. |
| `row` | `uint32_t` | — | Zero-indexed row position. |
| `col` | `uint32_t` | — | Zero-indexed column position. |
| `row_span` | `uint32_t` | `/* serde(default) */` | Number of rows this cell spans. |
| `col_span` | `uint32_t` | `/* serde(default) */` | Number of columns this cell spans. |
| `is_header` | `bool` | `/* serde(default) */` | Whether this is a header cell. |
| `bbox` | `KreuzbergBoundingBox*` | `NULL` | Bounding box for this cell (if available). |

---

#### KreuzbergHeaderMetadata

Header/heading element metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `level` | `uint8_t` | — | Header level: 1 (h1) through 6 (h6) |
| `text` | `const char*` | — | Normalized text content of the header |
| `id` | `const char**` | `NULL` | HTML id attribute if present |
| `depth` | `uint32_t` | — | Document tree depth at the header element |
| `html_offset` | `uint32_t` | — | Byte offset in original HTML document |

---

#### KreuzbergHeadingContext

Heading context for a chunk within a Markdown document.

Contains the heading hierarchy from document root to this chunk's section.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `headings` | `KreuzbergHeadingLevel*` | — | The heading hierarchy from document root to this chunk's section. Index 0 is the outermost (h1), last element is the most specific. |

---

#### KreuzbergHeadingLevel

A single heading in the hierarchy.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `level` | `uint8_t` | — | Heading depth (1 = h1, 2 = h2, etc.) |
| `text` | `const char*` | — | The text content of the heading. |

---

#### KreuzbergHeuristicsConfig

Configuration for document chunking and analysis heuristics.

Every threshold is a public field so callers can override any subset via
struct-update syntax: `HeuristicsConfig { text_layer_threshold: 0.5, ..the default constructor }`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enable_pdf_text_heuristics` | `bool` | `true` | Enable PDF text-layer detection heuristics. When `true`, PDFs with a substantial text layer will skip chunking. Default: `true`. |
| `text_layer_threshold` | `float` | `0.7` | Minimum fraction of pages that must have text to skip chunking. Range `0.0..=1.0`. Default: `0.7` (70 % of pages). |
| `file_size_threshold_bytes` | `uint64_t` | `10485760` | File size threshold in bytes for considering chunking. Files smaller than this are processed without chunking. Default: 10 MiB (10 × 1 024 × 1 024). |
| `page_count_threshold` | `uint32_t` | `50` | Page count threshold for considering chunking. Documents with fewer pages are processed without chunking. Default: 50. |
| `target_pages_per_chunk` | `uint32_t` | `10` | Target number of pages per chunk for optimal parallel processing. Default: 10. |
| `max_pages_per_chunk` | `uint32_t` | `25` | Hard cap on pages per chunk. No chunk will exceed this limit. Must be ≥ `target_pages_per_chunk`. Default: 25. |
| `disk_processing_threshold_bytes` | `uint64_t` | `52428800` | File size threshold for disk-based processing. Files larger than this are buffered to disk to prevent OOM. Default: 50 MiB (50 × 1 024 × 1 024). |
| `min_chars_per_page` | `uint32_t` | `50` | Minimum characters per page to consider a page as having text. Default: 50. |
| `max_xlsx_sheet_count` | `uint32_t` | `200` | Maximum sheet count allowed in an XLSX workbook. Workbooks beyond this are rejected pre-extraction to avoid OOM / abusive billing inflation. Default: 200. |
| `max_xlsx_workbook_cells` | `uint64_t` | `5000000` | Maximum cell count (sheets × rows × columns approximation) in an XLSX workbook. Default: 5 000 000 (≈ 200 sheets × 25 k cells). |
| `max_pptx_embedded_count` | `uint32_t` | `50` | Maximum number of OLE-embedded objects extractable from a single PPTX or DOCX. Protects against zip-bomb-style nested-document abuse. Default: 50. |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergHeuristicsConfig kreuzberg_default();
```

**Example:**

```c
KreuzbergHeuristicsConfig *result = kreuzberg_default();
```

**Returns:** `KreuzbergHeuristicsConfig`

###### kreuzberg_validate()

Validate the configuration.

**Errors:**

Returns `HeuristicsError.ConfigError` when:

- `target_pages_per_chunk` is 0
- `max_pages_per_chunk` < `target_pages_per_chunk`
- `file_size_threshold_bytes` is 0

**Signature:**

```c
void kreuzberg_validate();
```

**Example:**

```c
kreuzberg_validate(instance);
```

**Returns:** No return value.

**Errors:** Returns `NULL` on error.

###### kreuzberg_test_config()

Create a configuration suitable for unit tests (smaller thresholds).

**Signature:**

```c
KreuzbergHeuristicsConfig kreuzberg_test_config();
```

**Example:**

```c
KreuzbergHeuristicsConfig *result = kreuzberg_test_config();
```

**Returns:** `KreuzbergHeuristicsConfig`

---

#### KreuzbergHierarchicalBlock

A text block with hierarchy level assignment.

Represents a block of text with semantic heading information extracted from
font size clustering and hierarchical analysis.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text` | `const char*` | — | The text content of this block |
| `font_size` | `float` | — | The font size of the text in this block |
| `level` | `const char*` | — | The hierarchy level of this block (H1-H6 or Body) Levels correspond to HTML heading tags: - "h1": Top-level heading - "h2": Secondary heading - "h3": Tertiary heading - "h4": Quaternary heading - "h5": Quinary heading - "h6": Senary heading - "body": Body text (no heading level) |

---

#### KreuzbergHierarchyConfig

Hierarchy extraction configuration for PDF text structure analysis.

Enables extraction of document hierarchy levels (H1-H6) based on font size
clustering and semantic analysis. When enabled, hierarchical blocks are
included in page content.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `bool` | `true` | Enable hierarchy extraction |
| `k_clusters` | `uintptr_t` | `3` | Number of font size clusters to use for hierarchy levels (1-7) Default: 6, which provides H1-H6 heading levels with body text. Larger values create more fine-grained hierarchy levels. |
| `include_bbox` | `bool` | `true` | Include bounding box information in hierarchy blocks |
| `ocr_coverage_threshold` | `float*` | `NULL` | OCR coverage threshold for smart OCR triggering (0.0-1.0) Determines when OCR should be triggered based on text block coverage. OCR is triggered when text blocks cover less than this fraction of the page. Default: 0.5 (trigger OCR if less than 50% of page has text) |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergHierarchyConfig kreuzberg_default();
```

**Example:**

```c
KreuzbergHierarchyConfig *result = kreuzberg_default();
```

**Returns:** `KreuzbergHierarchyConfig`

---

#### KreuzbergHtmlMetadata

HTML metadata extracted from HTML documents.

Includes document-level metadata, Open Graph data, Twitter Card metadata,
and extracted structural elements (headers, links, images, structured data).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `title` | `const char**` | `NULL` | Document title from `<title>` tag |
| `description` | `const char**` | `NULL` | Document description from `<meta name="description">` tag |
| `keywords` | `const char**` | `NULL` | Document keywords from `<meta name="keywords">` tag, split on commas |
| `author` | `const char**` | `NULL` | Document author from `<meta name="author">` tag |
| `canonical_url` | `const char**` | `NULL` | Canonical URL from `<link rel="canonical">` tag |
| `base_href` | `const char**` | `NULL` | Base URL from `<base href="">` tag for resolving relative URLs |
| `language` | `const char**` | `NULL` | Document language from `lang` attribute |
| `text_direction` | `KreuzbergTextDirection*` | `NULL` | Document text direction from `dir` attribute |
| `open_graph` | `void*` | `NULL` | Open Graph metadata (og:* properties) for social media Keys like "title", "description", "image", "url", etc. |
| `twitter_card` | `void*` | `NULL` | Twitter Card metadata (twitter:* properties) Keys like "card", "site", "creator", "title", "description", "image", etc. |
| `meta_tags` | `void*` | `NULL` | Additional meta tags not covered by specific fields Keys are meta name/property attributes, values are content |
| `headers` | `KreuzbergHeaderMetadata*` | `NULL` | Extracted header elements with hierarchy |
| `links` | `KreuzbergLinkMetadata*` | `NULL` | Extracted hyperlinks with type classification |
| `images` | `KreuzbergImageMetadataType*` | `NULL` | Extracted images with source and dimensions |
| `structured_data` | `KreuzbergStructuredData*` | `NULL` | Extracted structured data blocks |

---

#### KreuzbergHtmlOutputConfig

Configuration for styled HTML output.

When set on `html_output` alongside
`output_format = OutputFormat.Html`, the pipeline builds a
`StyledHtmlRenderer` instead of
the plain comrak-based renderer.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `css` | `const char**` | `NULL` | Inline CSS string injected into the output after the theme stylesheet. Concatenated after `css_file` content when both are set. |
| `css_file` | `const char**` | `NULL` | Path to a CSS file loaded once at renderer construction time. Concatenated before `css` when both are set. |
| `theme` | `KreuzbergHtmlTheme` | `KREUZBERG_KREUZBERG_UNSTYLED` | Built-in colour/typography theme. Default: `HtmlTheme.Unstyled`. |
| `class_prefix` | `const char*` | — | CSS class prefix applied to every emitted class name. Default: `"kb-"`. Change this if your host application already uses classes that start with `kb-`. |
| `embed_css` | `bool` | `true` | When `true` (default), write the resolved CSS into a `<style>` block immediately after the opening `<div class="{prefix}doc">`. Set to `false` to emit only the structural markup and wire up your own stylesheet targeting the `kb-*` class names. |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergHtmlOutputConfig kreuzberg_default();
```

**Example:**

```c
KreuzbergHtmlOutputConfig *result = kreuzberg_default();
```

**Returns:** `KreuzbergHtmlOutputConfig`

---

#### KreuzbergImageExtractionConfig

Image extraction configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `extract_images` | `bool` | `true` | Extract images from documents |
| `target_dpi` | `int32_t` | `300` | Target DPI for image normalization |
| `max_image_dimension` | `int32_t` | `4096` | Maximum dimension for images (width or height) |
| `inject_placeholders` | `bool` | `true` | Whether to inject image reference placeholders into markdown output. When `true` (default), image references like `!\[Image 1\](embedded:p1_i0)` are appended to the markdown. Set to `false` to extract images as data without polluting the markdown output. |
| `auto_adjust_dpi` | `bool` | `true` | Automatically adjust DPI based on image content |
| `min_dpi` | `int32_t` | `72` | Minimum DPI threshold |
| `max_dpi` | `int32_t` | `600` | Maximum DPI threshold |
| `max_images_per_page` | `uint32_t*` | `NULL` | Maximum number of image objects to extract per PDF page. Some PDFs (e.g. technical diagrams stored as thousands of raster fragments) can trigger extremely long or indefinite extraction times when every image object on a dense page is decoded individually via the PDF extractor. Setting this limit causes kreuzberg to stop collecting individual images once the count per page reaches the cap and emit a warning instead. `NULL` (default) means no limit — all images are extracted. |
| `classify` | `bool` | `false` | When `true`, extracted images are classified by kind and grouped into clusters where they appear to belong to one figure. Defaults to `false` — opt in explicitly to avoid unexpected ML overhead. |
| `include_page_rasters` | `bool` | `false` | When `true`, full-page renders produced during OCR preprocessing are captured and returned as `ImageKind.PageRaster` entries in `ExtractionResult.images`. **PDF + OCR only.** No rasters are captured for non-PDF inputs or when the document-level OCR bypass is active (whole-document backend). When OCR is enabled and this flag is set but the active backend skips per-page rendering, a `ProcessingWarning` is emitted in `ExtractionResult.processing_warnings`. Defaults to `false`. Enable when downstream consumers need page thumbnails (e.g. citation previews, visual grounding). |
| `run_ocr_on_images` | `bool` | `true` | Run OCR on extracted images and include the recognized text in the document content. When `true` (default) and `ExtractionConfig.ocr` is configured, extracted images are processed with the configured OCR backend. Set to `false` to extract images without OCR processing, even when OCR is enabled. |
| `ocr_text_only` | `bool` | `false` | When `true`, image OCR results are rendered as plain text without the `!\[...\](...)` markdown placeholder. Only takes effect when `run_ocr_on_images` is also `true`. |
| `append_ocr_text` | `bool` | `false` | When `true` and `ocr_text_only` is `false`, append the OCR text after the image placeholder in the rendered output. |
| `output_format` | `KreuzbergImageOutputFormat` | `KREUZBERG_KREUZBERG_NATIVE` | Target format for re-encoding extracted images. When set to anything other than `Native`, each extracted image is re-encoded to the requested format before being returned. This lets callers receive uniform output without duplicating encode logic downstream. Defaults to `Native` — no re-encode pass is performed and `ExtractedImage.format` reflects the source extractor's output. |
| `svg` | `KreuzbergSvgOptions` | — | SVG-specific knobs for the image-encode pipeline. Controls sanitization and rasterization DPI when the source or output format is SVG.  Only available when the `svg` feature is active. |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergImageExtractionConfig kreuzberg_default();
```

**Example:**

```c
KreuzbergImageExtractionConfig *result = kreuzberg_default();
```

**Returns:** `KreuzbergImageExtractionConfig`

---

#### KreuzbergImageMetadata

Image metadata extracted from image files.

Includes dimensions, format, and EXIF data.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `width` | `uint32_t` | — | Image width in pixels |
| `height` | `uint32_t` | — | Image height in pixels |
| `format` | `const char*` | — | Image format (e.g., "PNG", "JPEG", "TIFF") |
| `exif` | `void*` | `NULL` | EXIF metadata tags |

---

#### KreuzbergImageMetadataType

Image element metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `src` | `const char*` | — | Image source (URL, data URI, or SVG content) |
| `alt` | `const char**` | `NULL` | Alternative text from alt attribute |
| `title` | `const char**` | `NULL` | Title attribute |
| `image_type` | `KreuzbergImageType` | — | Image type classification |

---

#### KreuzbergImagePreprocessingConfig

Image preprocessing configuration for OCR.

These settings control how images are preprocessed before OCR to improve
text recognition quality. Different preprocessing strategies work better
for different document types.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `target_dpi` | `int32_t` | `300` | Target DPI for the image (300 is standard, 600 for small text). |
| `auto_rotate` | `bool` | `false` | Auto-detect and correct image rotation. |
| `deskew` | `bool` | `true` | Correct skew (tilted images). |
| `denoise` | `bool` | `false` | Remove noise from the image. |
| `contrast_enhance` | `bool` | `false` | Enhance contrast for better text visibility. |
| `binarization_method` | `const char*` | `"otsu"` | Binarization method: "otsu", "sauvola", "adaptive". |
| `invert_colors` | `bool` | `false` | Invert colors (white text on black → black on white). |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergImagePreprocessingConfig kreuzberg_default();
```

**Example:**

```c
KreuzbergImagePreprocessingConfig *result = kreuzberg_default();
```

**Returns:** `KreuzbergImagePreprocessingConfig`

---

#### KreuzbergImagePreprocessingMetadata

Image preprocessing metadata.

Tracks the transformations applied to an image during OCR preprocessing,
including DPI normalization, resizing, and resampling.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `target_dpi` | `int32_t` | — | Target DPI from configuration |
| `scale_factor` | `double` | — | Scaling factor applied to the image |
| `auto_adjusted` | `bool` | — | Whether DPI was auto-adjusted based on content |
| `final_dpi` | `int32_t` | — | Final DPI after processing |
| `resample_method` | `const char*` | — | Resampling algorithm used ("LANCZOS3", "CATMULLROM", etc.) |
| `dimension_clamped` | `bool` | — | Whether dimensions were clamped to max_image_dimension |
| `calculated_dpi` | `int32_t*` | `NULL` | Calculated optimal DPI (if auto_adjust_dpi enabled) |
| `skipped_resize` | `bool` | — | Whether resize was skipped (dimensions already optimal) |
| `resize_error` | `const char**` | `NULL` | Error message if resize failed |

---

#### KreuzbergInlineElement

Inline element within a block.

Represents text with formatting, links, images, etc.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `element_type` | `KreuzbergInlineType` | — | Type of inline element |
| `content` | `const char*` | — | Text content |
| `metadata` | `void**` | `NULL` | Additional metadata (e.g., href for links, src/alt for images) |

---

#### KreuzbergJatsMetadata

JATS (Journal Article Tag Suite) metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `copyright` | `const char**` | `NULL` | Copyright statement from the article's `<permissions>` element. |
| `license` | `const char**` | `NULL` | Open-access license URI from the article's `<license>` element. |
| `history_dates` | `void*` | `NULL` | Publication history dates keyed by event type (e.g. `"received"`, `"accepted"`). |
| `contributor_roles` | `KreuzbergContributorRole*` | `NULL` | Authors and contributors with their stated roles. |

---

#### KreuzbergKeyword

Extracted keyword with metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text` | `const char*` | — | The keyword text. |
| `score` | `float` | — | Relevance score (higher is better, algorithm-specific range). |
| `algorithm` | `KreuzbergKeywordAlgorithm` | — | Algorithm that extracted this keyword. |
| `positions` | `uintptr_t**` | `NULL` | Optional positions where keyword appears in text (character offsets). |

---

#### KreuzbergKeywordConfig

Keyword extraction configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `algorithm` | `KreuzbergKeywordAlgorithm` | `KREUZBERG_KREUZBERG_YAKE` | Algorithm to use for extraction. |
| `max_keywords` | `uintptr_t` | `10` | Maximum number of keywords to extract (default: 10). |
| `min_score` | `float` | `0` | Minimum score threshold (0.0-1.0, default: 0.0). Keywords with scores below this threshold are filtered out. Note: Score ranges differ between algorithms. |
| `language` | `const char**` | `NULL` | Language code for stopword filtering (e.g., "en", "de", "fr"). If None, no stopword filtering is applied. |
| `yake_params` | `KreuzbergYakeParams*` | `NULL` | YAKE-specific tuning parameters. |
| `rake_params` | `KreuzbergRakeParams*` | `NULL` | RAKE-specific tuning parameters. |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergKeywordConfig kreuzberg_default();
```

**Example:**

```c
KreuzbergKeywordConfig *result = kreuzberg_default();
```

**Returns:** `KreuzbergKeywordConfig`

---

#### KreuzbergLanguageDetectionConfig

Language detection configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `bool` | `true` | Enable language detection |
| `min_confidence` | `double` | `0.8` | Minimum confidence threshold (0.0-1.0) |
| `detect_multiple` | `bool` | `false` | Detect multiple languages in the document |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergLanguageDetectionConfig kreuzberg_default();
```

**Example:**

```c
KreuzbergLanguageDetectionConfig *result = kreuzberg_default();
```

**Returns:** `KreuzbergLanguageDetectionConfig`

---

#### KreuzbergLayoutDetection

A single layout detection result.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `class_name` | `KreuzbergLayoutClass` | — | Detected layout class (e.g. `Table`, `Text`, `Title`). |
| `confidence` | `float` | — | Detection confidence score in `\[0.0, 1.0\]`. |
| `bbox` | `KreuzbergBBox` | — | Bounding box in image pixel coordinates. |

---

#### KreuzbergLayoutDetectionConfig

Layout detection configuration.

Controls layout detection behavior in the extraction pipeline.
When set on `ExtractionConfig`, layout detection
is enabled for PDF extraction.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `confidence_threshold` | `float*` | `NULL` | Confidence threshold override (None = use model default). |
| `apply_heuristics` | `bool` | `true` | Whether to apply postprocessing heuristics (default: true). |
| `table_model` | `KreuzbergTableModel` | `KREUZBERG_KREUZBERG_TATR` | Table structure recognition model. Controls which model is used for table cell detection within layout-detected table regions. Defaults to `TableModel.Tatr`. |
| `acceleration` | `KreuzbergAccelerationConfig*` | `NULL` | Hardware acceleration for ONNX models (layout detection + table structure). When set, controls which execution provider (CPU, CUDA, CoreML, TensorRT) is used for inference. Defaults to `NULL` (auto-select per platform). |
| `enable_chart_understanding` | `bool` | `false` | Route regions classified as charts to the chart-understanding OCR task. When `true`, layout regions detected as charts are sent to the VLM chart task (data-series/axis recovery) instead of being treated as generic image regions. Defaults to `false` — chart understanding is opt-in and has no effect on standard text/table extraction scores. |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergLayoutDetectionConfig kreuzberg_default();
```

**Example:**

```c
KreuzbergLayoutDetectionConfig *result = kreuzberg_default();
```

**Returns:** `KreuzbergLayoutDetectionConfig`

---

#### KreuzbergLayoutRegion

A detected layout region on a page.

When layout detection is enabled, each page may have layout regions
identifying different content types (text, pictures, tables, etc.)
with confidence scores and spatial positions.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `class_name` | `const char*` | — | Layout class name (e.g. "picture", "table", "text", "section_header"). |
| `confidence` | `double` | — | Confidence score from the layout detection model (0.0 to 1.0). |
| `bounding_box` | `KreuzbergBoundingBox` | — | Bounding box in document coordinate space. |
| `area_fraction` | `double` | — | Fraction of the page area covered by this region (0.0 to 1.0). |

---

#### KreuzbergLinkMetadata

Link element metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `href` | `const char*` | — | The href URL value |
| `text` | `const char*` | — | Link text content (normalized) |
| `title` | `const char**` | `NULL` | Optional title attribute |
| `link_type` | `KreuzbergLinkType` | — | Link type classification |
| `rel` | `const char**` | — | Rel attribute values |

---

#### KreuzbergLlmBackend

liter-llm-backed NER backend.

##### Methods

###### kreuzberg_new()

Create a new LLM-backed NER backend with the given LLM configuration.

**Signature:**

```c
KreuzbergLlmBackend kreuzberg_new(KreuzbergLlmConfig config);
```

**Example:**

```c
KreuzbergLlmBackend *result = kreuzberg_new((KreuzbergLlmConfig){0});
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `config` | `KreuzbergLlmConfig` | Yes | The configuration options |

**Returns:** `KreuzbergLlmBackend`

###### kreuzberg_detect()

**Signature:**

```c
KreuzbergEntity* kreuzberg_detect(const char* text, KreuzbergEntityCategory* categories);
```

**Example:**

```c
KreuzbergEntity* result = kreuzberg_detect(instance, "value", NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `const char*` | Yes | The text |
| `categories` | `KreuzbergEntityCategory*` | Yes | The categories |

**Returns:** `KreuzbergEntity*`

**Errors:** Returns `NULL` on error.

###### kreuzberg_detect_with_custom()

**Signature:**

```c
KreuzbergEntity* kreuzberg_detect_with_custom(const char* text, KreuzbergEntityCategory* categories, const char** custom_labels);
```

**Example:**

```c
KreuzbergEntity* result = kreuzberg_detect_with_custom(instance, "value", NULL, NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `const char*` | Yes | The text |
| `categories` | `KreuzbergEntityCategory*` | Yes | The categories |
| `custom_labels` | `const char**` | Yes | The custom labels |

**Returns:** `KreuzbergEntity*`

**Errors:** Returns `NULL` on error.

---

#### KreuzbergLlmConfig

Configuration for an LLM provider/model via liter-llm.

Each feature (VLM OCR, VLM embeddings, structured extraction) carries
its own `LlmConfig`, allowing different providers per feature.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `const char*` | — | Provider/model string using liter-llm routing format. Examples: `"openai/gpt-4o"`, `"anthropic/claude-sonnet-4-20250514"`, `"groq/llama-3.1-70b-versatile"`. |
| `api_key` | `const char**` | `NULL` | API key for the provider. When `NULL`, liter-llm falls back to the provider's standard environment variable (e.g., `OPENAI_API_KEY`). |
| `base_url` | `const char**` | `NULL` | Custom base URL override for the provider endpoint. |
| `timeout_secs` | `uint64_t*` | `NULL` | Request timeout in seconds (default: 60). |
| `max_retries` | `uint32_t*` | `NULL` | Maximum retry attempts (default: 3). |
| `temperature` | `double*` | `NULL` | Sampling temperature for generation tasks. |
| `max_tokens` | `uint64_t*` | `NULL` | Maximum tokens to generate. |

---

#### KreuzbergLlmUsage

Token usage and cost data for a single LLM call made during extraction.

Populated when VLM OCR, structured extraction, or LLM-based embeddings
are used. Multiple entries may be present when multiple LLM calls occur
within one extraction (e.g. VLM OCR + structured extraction).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `const char*` | — | The LLM model identifier (e.g. "openai/gpt-4o", "anthropic/claude-sonnet-4-20250514"). |
| `source` | `const char*` | — | The pipeline stage that triggered this LLM call (e.g. "vlm_ocr", "structured_extraction", "embeddings"). |
| `input_tokens` | `uint64_t*` | `NULL` | Number of input/prompt tokens consumed. |
| `output_tokens` | `uint64_t*` | `NULL` | Number of output/completion tokens generated. |
| `total_tokens` | `uint64_t*` | `NULL` | Total tokens (input + output). |
| `estimated_cost` | `double*` | `NULL` | Estimated cost in USD based on the provider's published pricing. |
| `finish_reason` | `const char**` | `NULL` | Why the model stopped generating (e.g. "stop", "length", "content_filter"). |

---

#### KreuzbergMetaSchema

Compiled meta-schema validator over `preset.schema.json`.

##### Methods

###### kreuzberg_compile()

Compile the given JSON text as a Draft 2020-12 meta-schema.

**Signature:**

```c
KreuzbergMetaSchema kreuzberg_compile(const char* meta_schema_json);
```

**Example:**

```c
KreuzbergMetaSchema *result = kreuzberg_compile("value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `meta_schema_json` | `const char*` | Yes | The meta schema json |

**Returns:** `KreuzbergMetaSchema`

**Errors:** Returns `NULL` on error.

###### kreuzberg_parse_preset()

Validate `raw` against the meta-schema and deserialize into a `Preset`,
stamping the fingerprint over the canonical file bytes.

**Signature:**

```c
KreuzbergPreset kreuzberg_parse_preset(const char* path, const uint8_t* raw);
```

**Example:**

```c
KreuzbergPreset *result = kreuzberg_parse_preset(instance, "value", (const uint8_t *)"data");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `path` | `const char*` | Yes | Path to the file |
| `raw` | `const uint8_t*` | Yes | The raw |

**Returns:** `KreuzbergPreset`

**Errors:** Returns `NULL` on error.

---

#### KreuzbergMetadata

Extraction result metadata.

Contains common fields applicable to all formats, format-specific metadata
via a discriminated union, and additional custom fields from postprocessors.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `title` | `const char**` | `NULL` | Document title |
| `subject` | `const char**` | `NULL` | Document subject or description |
| `authors` | `const char***` | `NULL` | Primary author(s) - always Vec for consistency |
| `keywords` | `const char***` | `NULL` | Keywords/tags - always Vec for consistency |
| `language` | `const char**` | `NULL` | Primary language (ISO 639 code) |
| `created_at` | `const char**` | `NULL` | Creation timestamp (ISO 8601 format) |
| `modified_at` | `const char**` | `NULL` | Last modification timestamp (ISO 8601 format) |
| `created_by` | `const char**` | `NULL` | User who created the document |
| `modified_by` | `const char**` | `NULL` | User who last modified the document |
| `pages` | `KreuzbergPageStructure*` | `NULL` | Page/slide/sheet structure with boundaries |
| `format` | `KreuzbergFormatMetadata*` | `NULL` | Format-specific metadata (discriminated union) Contains detailed metadata specific to the document format. Serialized as a nested `"format"` object with a `format_type` discriminator field. |
| `image_preprocessing` | `KreuzbergImagePreprocessingMetadata*` | `NULL` | Image preprocessing metadata (when OCR preprocessing was applied) |
| `json_schema` | `void**` | `NULL` | JSON schema (for structured data extraction) |
| `error` | `KreuzbergErrorMetadata*` | `NULL` | Error metadata (for batch operations) |
| `extraction_duration_ms` | `uint64_t*` | `NULL` | Extraction duration in milliseconds (for benchmarking). This field is populated by batch extraction to provide per-file timing information. It's `NULL` for single-file extraction (which uses external timing). |
| `category` | `const char**` | `NULL` | Document category (from frontmatter or classification). |
| `tags` | `const char***` | `NULL` | Document tags (from frontmatter). |
| `document_version` | `const char**` | `NULL` | Document version string (from frontmatter). |
| `abstract_text` | `const char**` | `NULL` | Abstract or summary text (from frontmatter). |
| `output_format` | `const char**` | `NULL` | Output format identifier (e.g., "markdown", "html", "text"). Set by the output format pipeline stage when format conversion is applied. Previously stored in `metadata.additional\["output_format"\]`. |
| `ocr_used` | `bool` | — | Whether OCR was used during extraction. Set to `true` whenever the extraction pipeline ran an OCR backend (Tesseract, PaddleOCR, VLM, etc.) and used that output as the primary or fallback text. `false` means native text extraction was used exclusively. |
| `additional` | `void*` | `NULL` | Additional custom fields from postprocessors. Serialized as a nested `"additional"` object (not flattened at root level). Uses `Cow<'static, str>` keys so static string keys avoid allocation. |

##### Methods

###### kreuzberg_is_empty()

Returns `true` when no metadata fields, format-specific metadata, or
additional postprocessor fields are populated.

**Signature:**

```c
bool kreuzberg_is_empty();
```

**Example:**

```c
bool result = kreuzberg_is_empty(instance);
```

**Returns:** `bool`

---

#### KreuzbergModelPaths

Combined paths to all models needed for OCR (backward compatibility).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `det_model` | `const char*` | — | Path to the detection model directory. |
| `cls_model` | `const char*` | — | Path to the classification model directory. |
| `rec_model` | `const char*` | — | Path to the recognition model directory. |
| `dict_file` | `const char*` | — | Path to the character dictionary file. |

---

#### KreuzbergMultidocInput

Input signals for multi-document boundary detection.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `page_count` | `uint32_t` | — | Total number of pages in the PDF. |
| `pages` | `KreuzbergPageSignals*` | — | Per-page signals extracted from the PDF. |

---

#### KreuzbergMultidocThresholds

Thresholds for multi-document boundary detection.

All fields are public; callers override any subset via struct-update syntax.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `density_shift_threshold` | `float` | `0.3` | Text density difference threshold for `DensityShift` detection. Default: 0.3. |
| `bigram_overlap_min` | `float` | `0.1` | Minimum bigram-overlap ratio below which a density shift is promoted to a `DensityShift` boundary.  Default: 0.1 (10 % overlap). |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergMultidocThresholds kreuzberg_default();
```

**Example:**

```c
KreuzbergMultidocThresholds *result = kreuzberg_default();
```

**Returns:** `KreuzbergMultidocThresholds`

---

#### KreuzbergNerConfig

**Since:** `v5.0`

Configuration for the NER post-processor.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `backend` | `KreuzbergNerBackendKind` | `KREUZBERG_KREUZBERG_ONNX` | Backend that runs the entity detection. |
| `categories` | `KreuzbergEntityCategory*` | `NULL` | Entity categories to detect. Defaults to a sensible PERSON/ORG/LOCATION/EMAIL set when empty. |
| `model` | `const char**` | `NULL` | Override the default model — only used by `NerBackendKind.Onnx`. `NULL` lets the backend pick its pinned default (`urchade/gliner_multi-v2.1` for gline-rs). |
| `llm` | `KreuzbergLlmConfig*` | `NULL` | Optional LLM configuration — only used by `NerBackendKind.Llm`. Token usage for LLM backends is recorded in `ExtractionResult.llm_usage`. |
| `custom_labels` | `const char**` | `NULL` | Arbitrary user-supplied entity labels for zero-shot detection. gline-rs natively supports zero-shot inference over caller-supplied labels — this is the primary value of GLiNER. The LLM backend also honours these labels by including them in the structured-output schema. Custom labels surface as `EntityCategory.Custom` in the resulting `Entity` stream. Use this when you need domain-specific entity types (e.g. `"Treatment"`, `"Product"`, `"Vessel"`) without forking GLiNER's taxonomy. |

---

#### KreuzbergOcrBackend

Trait for OCR backend plugins.

Implement this trait to add custom OCR capabilities. OCR backends can be:

- Native Rust implementations (like Tesseract)
- FFI bridges to Python libraries (like EasyOCR, PaddleOCR)
- Cloud-based OCR services (Google Vision, AWS Textract, etc.)

##### Thread Safety

OCR backends must be thread-safe (`Send + Sync`) to support concurrent processing.

##### Methods

###### kreuzberg_process_image()

Process an image and extract text via OCR.

**Returns:**

An `ExtractionResult` containing the extracted text and metadata.

**Errors:**

- `KreuzbergError.Ocr` - OCR processing failed
- `KreuzbergError.Validation` - Invalid image format or configuration
- `KreuzbergError.Io` - I/O errors (these always bubble up)

##### Reading `backend_options`

Backends that support runtime tuning can read `config.backend_options` and
deserialize only the keys they care about. Unknown keys are silently ignored,
so multiple backends can coexist in a pipeline without key conflicts.

**Signature:**

```c
KreuzbergExtractionResult kreuzberg_process_image(const uint8_t* image_bytes, KreuzbergOcrConfig config);
```

**Example:**

```c
KreuzbergExtractionResult *result = kreuzberg_process_image(instance, (const uint8_t *)"data", NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `image_bytes` | `const uint8_t*` | Yes | Raw image data (JPEG, PNG, TIFF, etc.) |
| `config` | `KreuzbergOcrConfig` | Yes | OCR configuration (language, PSM mode, etc.) |

**Returns:** `KreuzbergExtractionResult`

**Errors:** Returns `NULL` on error.

###### kreuzberg_process_image_file()

Process a file and extract text via OCR.

Default implementation reads the file and calls `process_image`.
Override for custom file handling or optimizations.

**Errors:**

Same as `process_image`, plus file I/O errors.

**Signature:**

```c
KreuzbergExtractionResult kreuzberg_process_image_file(const char* path, KreuzbergOcrConfig config);
```

**Example:**

```c
KreuzbergExtractionResult *result = kreuzberg_process_image_file(instance, "value", NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `path` | `const char*` | Yes | Path to the image file |
| `config` | `KreuzbergOcrConfig` | Yes | OCR configuration |

**Returns:** `KreuzbergExtractionResult`

**Errors:** Returns `NULL` on error.

###### kreuzberg_supports_language()

Check if this backend supports a given language code.

**Returns:**

`true` if the language is supported, `false` otherwise.

**Signature:**

```c
bool kreuzberg_supports_language(const char* lang);
```

**Example:**

```c
bool result = kreuzberg_supports_language(instance, "value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `lang` | `const char*` | Yes | ISO 639-2/3 language code (e.g., "eng", "deu", "fra") |

**Returns:** `bool`

###### kreuzberg_backend_type()

Get the backend type identifier.

**Returns:**

The backend type enum value.

**Signature:**

```c
KreuzbergOcrBackendType kreuzberg_backend_type();
```

**Example:**

```c
KreuzbergOcrBackendType *result = kreuzberg_backend_type(instance);
```

**Returns:** `KreuzbergOcrBackendType`

###### kreuzberg_supported_languages()

Optional: Get a list of all supported languages.

Defaults to empty list. Override to provide comprehensive language support info.

**Signature:**

```c
const char** kreuzberg_supported_languages();
```

**Example:**

```c
const char** result = kreuzberg_supported_languages(instance);
```

**Returns:** `const char**`

###### kreuzberg_supports_table_detection()

Optional: Check if the backend supports table detection.

Defaults to `false`. Override if your backend can detect and extract tables.

**Signature:**

```c
bool kreuzberg_supports_table_detection();
```

**Example:**

```c
bool result = kreuzberg_supports_table_detection(instance);
```

**Returns:** `bool`

###### kreuzberg_supports_document_processing()

Check if the backend supports direct document-level processing (e.g. for PDFs).

Defaults to `false`. Override if the backend has optimized document processing.

**Signature:**

```c
bool kreuzberg_supports_document_processing();
```

**Example:**

```c
bool result = kreuzberg_supports_document_processing(instance);
```

**Returns:** `bool`

###### kreuzberg_emits_structured_markdown()

Declare that this backend emits structured markdown directly (tables, headings, lists)
and downstream layout reconstruction should be skipped.

Defaults to `false` — classical OCR backends (Tesseract, PaddleOCR classical) return
plain text per detected region. End-to-end VLM backends (PaddleOCR-VL, GOT-OCR 2.0)
emit markdown in one forward pass and should override this to `true`.

**Signature:**

```c
bool kreuzberg_emits_structured_markdown();
```

**Example:**

```c
bool result = kreuzberg_emits_structured_markdown(instance);
```

**Returns:** `bool`

###### kreuzberg_process_document()

Process a document file directly via OCR.

Only called if `supports_document_processing` returns `true`.

**Signature:**

```c
KreuzbergExtractionResult kreuzberg_process_document(const char* path, KreuzbergOcrConfig config);
```

**Example:**

```c
KreuzbergExtractionResult *result = kreuzberg_process_document(instance, "value", NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `path` | `const char*` | Yes | The  path |
| `config` | `KreuzbergOcrConfig` | Yes | The ocr config |

**Returns:** `KreuzbergExtractionResult`

**Errors:** Returns `NULL` on error.

---

#### KreuzbergOcrConfidence

Confidence scores for an OCR element.

Separates detection confidence (how confident that text exists at this location)
from recognition confidence (how confident about the actual text content).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `detection` | `double*` | `NULL` | Detection confidence: how confident the OCR engine is that text exists here. PaddleOCR provides this as `box_score`, Tesseract doesn't have a direct equivalent. Range: 0.0 to 1.0 (or None if not available). |
| `recognition` | `double` | — | Recognition confidence: how confident about the text content. Range: 0.0 to 1.0. |

---

#### KreuzbergOcrConfig

OCR configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `bool` | `true` | Whether OCR is enabled. Setting `enabled: false` is a shorthand for `disable_ocr: true` on the parent `ExtractionConfig`. Images return metadata only; PDFs use native text extraction without OCR fallback. Defaults to `true`. When `false`, all other OCR settings are ignored. |
| `backend` | `const char*` | — | OCR backend: tesseract, easyocr, paddleocr |
| `language` | `const char*` | — | Language code (e.g., "eng", "deu") |
| `tesseract_config` | `KreuzbergTesseractConfig*` | `NULL` | Tesseract-specific configuration (optional) |
| `output_format` | `KreuzbergOutputFormat*` | `NULL` | Output format for OCR results (optional, for format conversion) |
| `paddle_ocr_config` | `void**` | `NULL` | PaddleOCR-specific configuration (optional, JSON passthrough) |
| `backend_options` | `void**` | `NULL` | Arbitrary per-call options passed through to the backend unchanged. Custom OCR backends and built-in backends that support runtime tuning can read this value and deserialize the keys they care about. Keys unknown to the backend are silently ignored. This is the recommended extension point for per-call parameters that are not covered by the typed fields above (e.g. mode switching, preprocessing flags, inference batch size). **Scope:** when `pipeline` is `NULL`, this value is propagated to the primary stage of the auto-constructed pipeline. When `pipeline` is explicitly set, this field has **no effect** — the caller must set `OcrPipelineStage.backend_options` directly on the relevant stage(s) instead. Example: ```json { "mode": "fast", "enable_layout": true, "timeout_ms": 5000 } ``` |
| `element_config` | `KreuzbergOcrElementConfig*` | `NULL` | OCR element extraction configuration |
| `quality_thresholds` | `KreuzbergOcrQualityThresholds*` | `NULL` | Quality thresholds for the native-text-to-OCR fallback decision. When None, uses compiled defaults (matching previous hardcoded behavior). |
| `pipeline` | `KreuzbergOcrPipelineConfig*` | `NULL` | Multi-backend OCR pipeline configuration. When set, enables weighted fallback across multiple OCR backends based on output quality. When None, uses the single `backend` field (same as today). |
| `auto_rotate` | `bool` | `false` | Enable automatic page rotation based on orientation detection. When enabled, uses Tesseract's `DetectOrientationScript()` to detect page orientation (0/90/180/270 degrees) before OCR. If the page is rotated with high confidence, the image is corrected before recognition. This is critical for handling rotated scanned documents. |
| `vlm_fallback` | `KreuzbergVlmFallbackPolicy` | `KREUZBERG_KREUZBERG_DISABLED` | Ergonomic VLM fallback policy. When set to anything other than `VlmFallbackPolicy.Disabled` and `OcrConfig.pipeline` is `NULL`, a multi-stage pipeline is synthesised automatically: - `VlmFallbackPolicy.OnLowQuality` → `\[classical_stage, vlm_stage\]` with the `quality_threshold` mapped onto `OcrQualityThresholds.pipeline_min_quality`. - `VlmFallbackPolicy.Always` → `\[vlm_stage\]` only. Requires `OcrConfig.vlm_config` to be `Some` when not `Disabled`. When `OcrConfig.pipeline` is explicitly set, this field is ignored. |
| `vlm_config` | `KreuzbergLlmConfig*` | `NULL` | VLM (Vision Language Model) OCR configuration. Required when `backend` is `"vlm"` or when `vlm_fallback` is not `VlmFallbackPolicy.Disabled`. Uses liter-llm to send page images to a vision model for text extraction. |
| `vlm_prompt` | `const char**` | `NULL` | Custom Jinja2 prompt template for VLM OCR. When `NULL`, uses the default template. Available variables: - `{{ language }}` — The document language code (e.g., "eng", "deu"). |
| `acceleration` | `KreuzbergAccelerationConfig*` | `NULL` | Hardware acceleration for ONNX Runtime models (e.g. PaddleOCR, layout detection). Not user-configurable via config files — injected at runtime from `ExtractionConfig.acceleration` before each `process_image` call. |
| `tessdata_bytes` | `void**` | `NULL` | Caller-supplied Tesseract `traineddata` bytes per language code. Primary use case is the WASM build, which has no filesystem and cannot download tessdata at runtime. Native builds typically rely on `TessdataManager` and ignore this field. When present, the WASM Tesseract backend prefers these bytes over its compile-time-bundled English data. Skipped by serde to keep config files small — supply via the typed API at runtime. |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergOcrConfig kreuzberg_default();
```

**Example:**

```c
KreuzbergOcrConfig *result = kreuzberg_default();
```

**Returns:** `KreuzbergOcrConfig`

---

#### KreuzbergOcrElement

A unified OCR element representing detected text with full metadata.

This is the primary type for structured OCR output, preserving all information
from both Tesseract and PaddleOCR backends.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text` | `const char*` | — | The recognized text content. |
| `geometry` | `KreuzbergOcrBoundingGeometry` | `KREUZBERG_KREUZBERG_RECTANGLE` | Bounding geometry (rectangle or quadrilateral). |
| `confidence` | `KreuzbergOcrConfidence` | — | Confidence scores for detection and recognition. |
| `level` | `KreuzbergOcrElementLevel` | `KREUZBERG_KREUZBERG_LINE` | Hierarchical level (word, line, block, page). |
| `rotation` | `KreuzbergOcrRotation*` | `NULL` | Rotation information (if detected). |
| `page_number` | `uint32_t` | — | Page number (1-indexed). |
| `parent_id` | `const char**` | `NULL` | Parent element ID for hierarchical relationships. Only used for Tesseract output which has word -> line -> block hierarchy. |
| `backend_metadata` | `void*` | `NULL` | Backend-specific metadata that doesn't fit the unified schema. |

---

#### KreuzbergOcrElementConfig

Configuration for OCR element extraction.

Controls how OCR elements are extracted and filtered.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `include_elements` | `bool` | — | Whether to include OCR elements in the extraction result. When true, the `ocr_elements` field in `ExtractionResult` will be populated. |
| `min_level` | `KreuzbergOcrElementLevel` | `KREUZBERG_KREUZBERG_LINE` | Minimum hierarchical level to include. Elements below this level (e.g., words when min_level is Line) will be excluded. |
| `min_confidence` | `double` | — | Minimum recognition confidence threshold (0.0-1.0). Elements with confidence below this threshold will be filtered out. |
| `build_hierarchy` | `bool` | — | Whether to build hierarchical relationships between elements. When true, `parent_id` fields will be populated based on spatial containment. Only meaningful for Tesseract output. |

---

#### KreuzbergOcrExtractionResult

OCR extraction result.

Result of performing OCR on an image or scanned document,
including recognized text and detected tables.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `const char*` | — | Recognized text content |
| `mime_type` | `const char*` | — | Original MIME type of the processed image |
| `metadata` | `void*` | — | OCR processing metadata (confidence scores, language, etc.) |
| `tables` | `KreuzbergOcrTable*` | — | Tables detected and extracted via OCR |
| `ocr_elements` | `KreuzbergOcrElement**` | `/* serde(default) */` | Structured OCR elements with bounding boxes and confidence scores. Available when TSV output is requested or table detection is enabled. |

---

#### KreuzbergOcrMetadata

OCR processing metadata.

Captures information about OCR processing configuration and results.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `language` | `const char*` | — | OCR language code(s) used |
| `psm` | `int32_t` | — | Tesseract Page Segmentation Mode (PSM) |
| `output_format` | `const char*` | — | Output format (e.g., "text", "hocr") |
| `table_count` | `uint32_t` | — | Number of tables detected |
| `table_rows` | `uint32_t*` | `NULL` | Number of rows in the detected table (if a single table was found). |
| `table_cols` | `uint32_t*` | `NULL` | Number of columns in the detected table (if a single table was found). |

---

#### KreuzbergOcrPipelineConfig

Multi-backend OCR pipeline with quality-based fallback.

Backends are tried in priority order (highest first). After each backend
produces output, quality is evaluated. If it meets `quality_thresholds.pipeline_min_quality`,
the result is accepted. Otherwise the next backend is tried.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `stages` | `KreuzbergOcrPipelineStage*` | — | Ordered list of backends to try. Sorted by priority (descending) at runtime. |
| `quality_thresholds` | `KreuzbergOcrQualityThresholds` | `/* serde(default) */` | Quality thresholds for deciding whether to accept a result or try the next backend. |

---

#### KreuzbergOcrPipelineStage

A single backend stage in the OCR pipeline.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `backend` | `const char*` | — | Backend name: "tesseract", "paddleocr", "easyocr", or a custom registered name. |
| `priority` | `uint32_t` | `/* serde(default) */` | Priority weight (higher = tried first). Stages are sorted by priority descending. |
| `language` | `const char**` | `/* serde(default) */` | Language override for this stage (None = use parent OcrConfig.language). |
| `tesseract_config` | `KreuzbergTesseractConfig*` | `/* serde(default) */` | Tesseract-specific config override for this stage. |
| `paddle_ocr_config` | `void**` | `/* serde(default) */` | PaddleOCR-specific config for this stage. |
| `vlm_config` | `KreuzbergLlmConfig*` | `/* serde(default) */` | VLM config override for this pipeline stage. |
| `backend_options` | `void**` | `/* serde(default) */` | Arbitrary per-call options passed through to the backend unchanged. Backends that support runtime tuning (mode switching, preprocessing flags, inference parameters, etc.) read this value and deserialize the keys they care about. Keys unknown to the backend are silently ignored, so options from different backends can coexist in the same config without conflict. Example (custom backend): ```json { "mode": "fast", "enable_layout": true } ``` |

---

#### KreuzbergOcrQualityThresholds

Quality thresholds for OCR fallback decisions and pipeline quality gating.

All fields default to the values that match the previous hardcoded behavior,
so `OcrQualityThresholds.default()` preserves existing semantics exactly.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `min_total_non_whitespace` | `uintptr_t` | `64` | Minimum total non-whitespace characters to consider text substantive. |
| `min_non_whitespace_per_page` | `double` | `32` | Minimum non-whitespace characters per page on average. |
| `min_meaningful_word_len` | `uintptr_t` | `4` | Minimum character count for a word to be "meaningful". |
| `min_meaningful_words` | `uintptr_t` | `3` | Minimum count of meaningful words before text is accepted. |
| `min_alnum_ratio` | `double` | `0.3` | Minimum alphanumeric ratio (non-whitespace chars that are alphanumeric). |
| `min_garbage_chars` | `uintptr_t` | `5` | Minimum Unicode replacement characters (U+FFFD) to trigger OCR fallback. |
| `max_fragmented_word_ratio` | `double` | `0.6` | Maximum fraction of short (1-2 char) words before text is considered fragmented. |
| `critical_fragmented_word_ratio` | `double` | `0.8` | Critical fragmentation threshold — triggers OCR regardless of meaningful words. Normal English text has ~20-30% short words. 80%+ is definitive garbage. |
| `min_avg_word_length` | `double` | `2` | Minimum average word length. Below this with enough words indicates garbled extraction. |
| `min_words_for_avg_length_check` | `uintptr_t` | `50` | Minimum word count before average word length check applies. |
| `min_consecutive_repeat_ratio` | `double` | `0.08` | Minimum consecutive word repetition ratio to detect column scrambling. |
| `min_words_for_repeat_check` | `uintptr_t` | `50` | Minimum word count before consecutive repetition check is applied. |
| `substantive_min_chars` | `uintptr_t` | `100` | Minimum character count for "substantive markdown" OCR skip gate. |
| `non_text_min_chars` | `uintptr_t` | `20` | Minimum character count for "non-text content" OCR skip gate. |
| `alnum_ws_ratio_threshold` | `double` | `0.4` | Alphanumeric+whitespace ratio threshold for skip decisions. |
| `pipeline_min_quality` | `double` | `0.5` | Minimum quality score (0.0-1.0) for a pipeline stage result to be accepted. If the result from a backend scores below this, try the next backend. |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergOcrQualityThresholds kreuzberg_default();
```

**Example:**

```c
KreuzbergOcrQualityThresholds *result = kreuzberg_default();
```

**Returns:** `KreuzbergOcrQualityThresholds`

---

#### KreuzbergOcrRotation

Rotation information for an OCR element.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `angle_degrees` | `double` | — | Rotation angle in degrees (0, 90, 180, 270 for PaddleOCR). |
| `confidence` | `double*` | `NULL` | Confidence score for the rotation detection. |

---

#### KreuzbergOcrTable

Table detected via OCR.

Represents a table structure recognized during OCR processing.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `cells` | `const char***` | — | Table cells as a 2D vector (rows × columns) |
| `markdown` | `const char*` | — | Markdown representation of the table |
| `page_number` | `uint32_t` | — | Page number where the table was found (1-indexed) |
| `bounding_box` | `KreuzbergOcrTableBoundingBox*` | `/* serde(default) */` | Bounding box of the table in pixel coordinates (from OCR word positions). |

---

#### KreuzbergOcrTableBoundingBox

Bounding box for an OCR-detected table in pixel coordinates.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `left` | `uint32_t` | — | Left x-coordinate (pixels) |
| `top` | `uint32_t` | — | Top y-coordinate (pixels) |
| `right` | `uint32_t` | — | Right x-coordinate (pixels) |
| `bottom` | `uint32_t` | — | Bottom y-coordinate (pixels) |

---

#### KreuzbergOrientationResult

Document orientation detection result.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `degrees` | `uint32_t` | — | Detected orientation in degrees (0, 90, 180, or 270). |
| `confidence` | `float` | — | Confidence score (0.0-1.0). |

---

#### KreuzbergPaddleOcrConfig

Configuration for PaddleOCR backend.

Configures PaddleOCR text detection and recognition with multi-language support.
Uses a builder pattern for convenient configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `language` | `const char*` | — | Language code (e.g., "en", "ch", "jpn", "kor", "deu", "fra") |
| `cache_dir` | `const char**` | `NULL` | Optional custom cache directory for model files |
| `use_angle_cls` | `bool` | — | Enable angle classification for rotated text (default: false). Can misfire on short text regions, rotating crops incorrectly before recognition. |
| `enable_table_detection` | `bool` | — | Enable table structure detection (default: false) |
| `det_db_thresh` | `float` | — | Database threshold for text detection (default: 0.3) Range: 0.0-1.0, higher values require more confident detections |
| `det_db_box_thresh` | `float` | — | Box threshold for text bounding box refinement (default: 0.5) Range: 0.0-1.0 |
| `det_db_unclip_ratio` | `float` | — | Unclip ratio for expanding text bounding boxes (default: 1.6) Controls the expansion of detected text regions |
| `det_limit_side_len` | `uint32_t` | — | Maximum side length for detection image (default: 960) Larger images may be resized to this limit for faster inference |
| `rec_batch_num` | `uint32_t` | — | Batch size for recognition inference (default: 6) Number of text regions to process simultaneously |
| `padding` | `uint32_t` | — | Padding in pixels added around the image before detection (default: 10). Large values can include surrounding content like table gridlines. |
| `drop_score` | `float` | — | Minimum recognition confidence score for text lines (default: 0.5). Text regions with recognition confidence below this threshold are discarded. Matches PaddleOCR Python's `drop_score` parameter. Range: 0.0-1.0 |
| `model_tier` | `const char*` | — | Model tier controlling detection/recognition model size and accuracy trade-off. - `"mobile"` (default): Lightweight models (~4.5MB detection, ~16.5MB recognition), fast download and inference - `"server"`: Large, high-accuracy models (~88MB detection, ~84MB recognition), best for GPU or complex documents |

##### Methods

###### kreuzberg_with_cache_dir()

Sets a custom cache directory for model files.

**Signature:**

```c
KreuzbergPaddleOcrConfig kreuzberg_with_cache_dir(const char* path);
```

**Example:**

```c
KreuzbergPaddleOcrConfig *result = kreuzberg_with_cache_dir(instance, "value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `path` | `const char*` | Yes | Path to cache directory |

**Returns:** `KreuzbergPaddleOcrConfig`

###### kreuzberg_with_table_detection()

Enables or disables table structure detection.

**Signature:**

```c
KreuzbergPaddleOcrConfig kreuzberg_with_table_detection(bool enable);
```

**Example:**

```c
KreuzbergPaddleOcrConfig *result = kreuzberg_with_table_detection(instance, true);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `enable` | `bool` | Yes | Whether to enable table detection |

**Returns:** `KreuzbergPaddleOcrConfig`

###### kreuzberg_with_angle_cls()

Enables or disables angle classification for rotated text.

**Signature:**

```c
KreuzbergPaddleOcrConfig kreuzberg_with_angle_cls(bool enable);
```

**Example:**

```c
KreuzbergPaddleOcrConfig *result = kreuzberg_with_angle_cls(instance, true);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `enable` | `bool` | Yes | Whether to enable angle classification |

**Returns:** `KreuzbergPaddleOcrConfig`

###### kreuzberg_with_det_db_thresh()

Sets the database threshold for text detection.

**Signature:**

```c
KreuzbergPaddleOcrConfig kreuzberg_with_det_db_thresh(float threshold);
```

**Example:**

```c
KreuzbergPaddleOcrConfig *result = kreuzberg_with_det_db_thresh(instance, 0.5);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `threshold` | `float` | Yes | Detection threshold (0.0-1.0) |

**Returns:** `KreuzbergPaddleOcrConfig`

###### kreuzberg_with_det_db_box_thresh()

Sets the box threshold for text bounding box refinement.

**Signature:**

```c
KreuzbergPaddleOcrConfig kreuzberg_with_det_db_box_thresh(float threshold);
```

**Example:**

```c
KreuzbergPaddleOcrConfig *result = kreuzberg_with_det_db_box_thresh(instance, 0.5);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `threshold` | `float` | Yes | Box threshold (0.0-1.0) |

**Returns:** `KreuzbergPaddleOcrConfig`

###### kreuzberg_with_det_db_unclip_ratio()

Sets the unclip ratio for expanding text bounding boxes.

**Signature:**

```c
KreuzbergPaddleOcrConfig kreuzberg_with_det_db_unclip_ratio(float ratio);
```

**Example:**

```c
KreuzbergPaddleOcrConfig *result = kreuzberg_with_det_db_unclip_ratio(instance, 0.5);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `ratio` | `float` | Yes | Unclip ratio (typically 1.5-2.0) |

**Returns:** `KreuzbergPaddleOcrConfig`

###### kreuzberg_with_det_limit_side_len()

Sets the maximum side length for detection images.

**Signature:**

```c
KreuzbergPaddleOcrConfig kreuzberg_with_det_limit_side_len(uint32_t length);
```

**Example:**

```c
KreuzbergPaddleOcrConfig *result = kreuzberg_with_det_limit_side_len(instance, 42);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `length` | `uint32_t` | Yes | Maximum side length in pixels |

**Returns:** `KreuzbergPaddleOcrConfig`

###### kreuzberg_with_rec_batch_num()

Sets the batch size for recognition inference.

**Signature:**

```c
KreuzbergPaddleOcrConfig kreuzberg_with_rec_batch_num(uint32_t batch_size);
```

**Example:**

```c
KreuzbergPaddleOcrConfig *result = kreuzberg_with_rec_batch_num(instance, 42);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `batch_size` | `uint32_t` | Yes | Number of text regions to process simultaneously |

**Returns:** `KreuzbergPaddleOcrConfig`

###### kreuzberg_with_drop_score()

Sets the minimum recognition confidence threshold.

**Signature:**

```c
KreuzbergPaddleOcrConfig kreuzberg_with_drop_score(float score);
```

**Example:**

```c
KreuzbergPaddleOcrConfig *result = kreuzberg_with_drop_score(instance, 0.5);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `score` | `float` | Yes | Minimum confidence (0.0-1.0), text below this is dropped |

**Returns:** `KreuzbergPaddleOcrConfig`

###### kreuzberg_with_padding()

Sets padding in pixels added around images before detection.

**Signature:**

```c
KreuzbergPaddleOcrConfig kreuzberg_with_padding(uint32_t padding);
```

**Example:**

```c
KreuzbergPaddleOcrConfig *result = kreuzberg_with_padding(instance, 42);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `padding` | `uint32_t` | Yes | Padding in pixels (0-100) |

**Returns:** `KreuzbergPaddleOcrConfig`

###### kreuzberg_with_model_tier()

Sets the model tier controlling detection/recognition model size.

**Signature:**

```c
KreuzbergPaddleOcrConfig kreuzberg_with_model_tier(const char* tier);
```

**Example:**

```c
KreuzbergPaddleOcrConfig *result = kreuzberg_with_model_tier(instance, "value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `tier` | `const char*` | Yes | `"mobile"` (default, lightweight, faster) or `"server"` (high accuracy, GPU/complex documents) |

**Returns:** `KreuzbergPaddleOcrConfig`

###### kreuzberg_default()

Creates a default configuration with English language support.

**Signature:**

```c
KreuzbergPaddleOcrConfig kreuzberg_default();
```

**Example:**

```c
KreuzbergPaddleOcrConfig *result = kreuzberg_default();
```

**Returns:** `KreuzbergPaddleOcrConfig`

---

#### KreuzbergPageBoundary

Byte offset boundary for a page.

Tracks where a specific page's content starts and ends in the main content string,
enabling mapping from byte positions to page numbers. Offsets are guaranteed to be
at valid UTF-8 character boundaries when using standard String methods (push_str, push, etc.).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `byte_start` | `uintptr_t` | — | Byte offset where this page starts in the content string (UTF-8 valid boundary, inclusive) |
| `byte_end` | `uintptr_t` | — | Byte offset where this page ends in the content string (UTF-8 valid boundary, exclusive) |
| `page_number` | `uint32_t` | — | Page number (1-indexed) |

---

#### KreuzbergPageClassification

Classification result for a single page.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `page_number` | `uint32_t` | — | 1-indexed page number this classification belongs to. |
| `labels` | `KreuzbergClassificationLabel*` | — | Labels assigned to the page. Single-label classification yields exactly one entry; multi-label classification yields any subset of the configured label set. |

---

#### KreuzbergPageClassificationConfig

**Since:** `v5.0`

Configuration for the page-classification post-processor.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `prompt_template` | `const char**` | `NULL` | Minijinja prompt template. Receives `{{ labels }}` (joined list), `{{ page_text }}` and `{{ multi_label }}` variables. `NULL` lets the backend pick a sensible default. |
| `labels` | `const char**` | — | The set of labels the classifier may emit. Must contain at least one entry. |
| `multi_label` | `bool` | `/* serde(default) */` | Allow multiple labels per page. Single-label mode returns at most one label. |
| `llm` | `KreuzbergLlmConfig` | — | LLM configuration used for classification. |

---

#### KreuzbergPageConfig

Page extraction and tracking configuration.

Controls how pages are extracted, tracked, and represented in the extraction results.
When `NULL`, page tracking is disabled.

Page range tracking in chunk metadata (first_page/last_page) is automatically enabled
when page boundaries are available and chunking is configured.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `extract_pages` | `bool` | `false` | Extract pages as separate array (ExtractionResult.pages) |
| `insert_page_markers` | `bool` | `false` | Insert page markers in main content string |
| `marker_format` | `const char*` | `"<!-- PAGE {page_num} -->"` | Page marker format (use {page_num} placeholder) Default: "\n\n<!-- PAGE {page_num} -->\n\n" |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergPageConfig kreuzberg_default();
```

**Example:**

```c
KreuzbergPageConfig *result = kreuzberg_default();
```

**Returns:** `KreuzbergPageConfig`

---

#### KreuzbergPageContent

Content for a single page/slide.

When page extraction is enabled, documents are split into per-page content
with associated tables and images mapped to each page.

##### Performance

Uses shared tables and images for memory efficiency:

- `const Table*` enables zero-copy sharing of table data
- `const ExtractedImage*` enables zero-copy sharing of image data
- Maintains exact JSON compatibility via custom Serialize/Deserialize

This reduces memory overhead for documents with shared tables/images
by avoiding redundant copies during serialization.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `page_number` | `uint32_t` | — | Page number (1-indexed) |
| `content` | `const char*` | — | Text content for this page |
| `tables` | `KreuzbergTable*` | `/* serde(default) */` | Tables found on this page (uses Arc for memory efficiency) Serializes as const Table* for JSON compatibility while maintaining shared in-memory ownership for zero-copy sharing. |
| `image_indices` | `uint32_t*` | `/* serde(default) */` | Indices into `ExtractionResult.images` for images found on this page. Each value is a zero-based index into the top-level `images` collection. Only populated when `extract_images = true` in the extraction config. |
| `hierarchy` | `KreuzbergPageHierarchy*` | `NULL` | Hierarchy information for the page (when hierarchy extraction is enabled) Contains text hierarchy levels (H1-H6) extracted from the page content. |
| `is_blank` | `bool*` | `NULL` | Whether this page is blank (no meaningful text content) Determined during extraction based on text content analysis. A page is blank if it has fewer than 3 non-whitespace characters and contains no tables or images. |
| `layout_regions` | `KreuzbergLayoutRegion**` | `NULL` | Layout detection regions for this page (when layout detection is enabled). Contains detected layout regions with class, confidence, bounding box, and area fraction. Only populated when layout detection is configured. |
| `speaker_notes` | `const char**` | `NULL` | Speaker notes for this slide (PPTX only). Contains the text from the slide's notes pane (`ppt/notesSlides/notesSlide{N}.xml`). Only populated when the source is a PPTX file and notes are present. |
| `section_name` | `const char**` | `NULL` | Section name this slide belongs to (PPTX only). PowerPoint sections group slides into logical chapters (`<p:sectionLst>` in `ppt/presentation.xml`). Only populated when the source is a PPTX file and the slide belongs to a named section. |
| `sheet_name` | `const char**` | `NULL` | Sheet name for this page (XLSX/ODS only). Each spreadsheet sheet maps to one `PageContent` entry. This field carries the sheet's display name as it appears in the workbook. `NULL` for all non-spreadsheet formats and for sheets with an empty name. |

---

#### KreuzbergPageHierarchy

Page hierarchy structure containing heading levels and block information.

Used when PDF text hierarchy extraction is enabled. Contains hierarchical
blocks with heading levels (H1-H6) for semantic document structure.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `block_count` | `uint32_t` | — | Number of hierarchy blocks on this page |
| `blocks` | `KreuzbergHierarchicalBlock*` | `/* serde(default) */` | Hierarchical blocks with heading levels |

---

#### KreuzbergPageInfo

Metadata for individual page/slide/sheet.

Captures per-page information including dimensions, content counts,
and visibility state (for presentations).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `number` | `uint32_t` | — | Page number (1-indexed) |
| `title` | `const char**` | `NULL` | Page title (usually for presentations) |
| `image_count` | `uint32_t*` | `NULL` | Number of images on this page |
| `table_count` | `uint32_t*` | `NULL` | Number of tables on this page |
| `hidden` | `bool*` | `NULL` | Whether this page is hidden (e.g., in presentations) |
| `is_blank` | `bool*` | `NULL` | Whether this page is blank (no meaningful text, no images, no tables) A page is considered blank if it has fewer than 3 non-whitespace characters and contains no tables or images. This is useful for filtering out empty pages in scanned documents or PDFs with blank separator pages. |
| `has_vector_graphics` | `bool` | `/* serde(default) */` | Whether this page contains non-trivial vector graphics (paths, shapes, curves) Indicates the presence of vector-drawn content such as charts, diagrams, or geometric shapes (e.g., from Adobe InDesign, LaTeX TikZ). These are invisible to `ExtractionResult.images` since they are not embedded as raster XObjects. Set to `true` when path count exceeds a heuristic threshold, signaling that downstream consumers may want to rasterize the page to capture this content. Only populated for PDFs; `NULL` for other document types. |

---

#### KreuzbergPageRange

Page range for a chunk (0-indexed, inclusive).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `start` | `uint32_t` | — | Start page (0-indexed, inclusive). |
| `end` | `uint32_t` | — | End page (0-indexed, inclusive). |

##### Methods

###### kreuzberg_page_count()

Get the number of pages in this range.

**Signature:**

```c
uint32_t kreuzberg_page_count();
```

**Example:**

```c
uint32_t result = kreuzberg_page_count(instance);
```

**Returns:** `uint32_t`

---

#### KreuzbergPageSignals

Per-page signals extracted from PDF content.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `page_number` | `uint32_t` | — | 1-indexed page number. |
| `text_excerpt` | `const char*` | — | First ~500 characters of extracted text. |
| `starts_with_letterhead_like` | `bool` | — | `true` if page starts with letterhead-like content (ALL CAPS line in first 5 lines or a logo-image bbox at top). |
| `has_page_number_one_marker` | `bool` | — | `true` if text contains "Page 1" or "1 of N" pattern. |
| `has_signature_block` | `bool` | — | `true` if text contains signature indicators ("Sincerely", "Signed") or a signature image bbox. |
| `layout_text_density` | `float` | — | Text density: characters per page area, normalised to `\[0.0, 1.0\]`. |

##### Methods

###### kreuzberg_from_page_text()

Derive signals from raw page text.

Callers that already have structured per-page data (e.g. from a PDF extractor)
can set individual fields directly.  This constructor is for callers that only
have the plain-text content of a page (e.g. from `PageContent`).

  when unknown (disables density-shift detection for this page).

##### Heuristics

All signal derivations are *conservative starting points*.  Each is documented
inline.  They err on the side of fewer false positives; tune thresholds via
`MultidocThresholds` rather than by changing these heuristics.

**Signature:**

```c
KreuzbergPageSignals kreuzberg_from_page_text(uint32_t page_number, const char* text, float layout_text_density);
```

**Example:**

```c
KreuzbergPageSignals *result = kreuzberg_from_page_text(42, "value", 0.5);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `page_number` | `uint32_t` | Yes | The page number |
| `text` | `const char*` | Yes | The text |
| `layout_text_density` | `float` | Yes | The layout text density |

**Returns:** `KreuzbergPageSignals`

---

#### KreuzbergPageStructure

Unified page structure for documents.

Supports different page types (PDF pages, PPTX slides, Excel sheets)
with character offset boundaries for chunk-to-page mapping.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `total_count` | `uint32_t` | — | Total number of pages/slides/sheets |
| `unit_type` | `KreuzbergPageUnitType` | — | Type of paginated unit |
| `boundaries` | `KreuzbergPageBoundary**` | `NULL` | Character offset boundaries for each page Maps character ranges in the extracted content to page numbers. Used for chunk page range calculation. |
| `pages` | `KreuzbergPageInfo**` | `NULL` | Detailed per-page metadata (optional, only when needed) |

---

#### KreuzbergPatternMatch

One detected PII span in the input text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `start` | `uintptr_t` | — | Inclusive byte-offset start of the match in the source text. |
| `end` | `uintptr_t` | — | Exclusive byte-offset end of the match. |
| `category` | `KreuzbergPiiCategory` | — | Category the match belongs to. |
| `text` | `const char*` | — | Matched substring (owned copy — pattern engine returns owned data so the caller can free the original text if needed before replacement). |

---

#### KreuzbergPdfAnnotation

A PDF annotation extracted from a document page.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `annotation_type` | `KreuzbergPdfAnnotationType` | — | The type of annotation. |
| `content` | `const char**` | `NULL` | Text content of the annotation (e.g., comment text, link URL). |
| `page_number` | `uint32_t` | — | Page number where the annotation appears (1-indexed). |
| `bounding_box` | `KreuzbergBoundingBox*` | `NULL` | Bounding box of the annotation on the page. |

---

#### KreuzbergPdfConfig

PDF-specific configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `extract_images` | `bool` | `false` | Extract images from PDF |
| `extract_tables` | `bool` | `true` | Extract tables from PDF. When `true` (default), runs pdf_oxide's native grid detector and, if it finds nothing, falls back to the heuristic text-layer reconstruction in `pdf.oxide.table.extract_tables_heuristic`. Set to `false` to skip both passes — `tables` will then be empty in the result. |
| `passwords` | `const char***` | `NULL` | List of passwords to try when opening encrypted PDFs |
| `extract_metadata` | `bool` | `true` | Extract PDF metadata |
| `hierarchy` | `KreuzbergHierarchyConfig*` | `NULL` | Hierarchy extraction configuration (None = hierarchy extraction disabled) |
| `extract_annotations` | `bool` | `false` | Extract PDF annotations (text notes, highlights, links, stamps). Default: false |
| `top_margin_fraction` | `float*` | `NULL` | Top margin fraction (0.0–1.0) of page height to exclude headers/running heads. Default: 0.06 (6%) |
| `bottom_margin_fraction` | `float*` | `NULL` | Bottom margin fraction (0.0–1.0) of page height to exclude footers/page numbers. Default: 0.05 (5%) |
| `allow_single_column_tables` | `bool` | `false` | Allow single-column pseudo tables in extraction results. By default, tables with fewer than 2 columns (layout-guided) or 3 columns (heuristic) are rejected. When `true`, the minimum column count is relaxed to 1, allowing single-column structured data (glossaries, itemized lists) to be emitted as tables. Other quality filters (density, sparsity, prose detection) still apply. |
| `ocr_inline_images` | `bool` | `false` | Perform OCR on inline images extracted from PDF pages and attach the recognized text to each `ExtractedImage.ocr_result`. Requires Tesseract to be available; if `ExtractionConfig.ocr` is `NULL` the extractor falls back to `TesseractConfig.default()`. Per-image failures degrade gracefully (the image is returned without OCR text rather than failing the whole extraction). Default: `false`. |
| `extract_form_fields` | `bool` | `true` | Extract AcroForm and XFA form fields into `ExtractionResult.form_fields`. When `true` (default), reads the document's interactive form structure (field names, types, values, widget geometry). Cheap and strictly additive — non-form PDFs simply yield an empty list. Set to `false` to skip the form pass entirely. |
| `reading_order` | `bool` | `false` | Reorder extracted text by layout-detected reading order. When `true`, projects text spans onto layout-detected regions, performs column detection, and emits spans in natural reading order (important for multi-column academic PDFs). Requires the `layout-detection` feature; has no effect without it. Defaults to `false`. |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergPdfConfig kreuzberg_default();
```

**Example:**

```c
KreuzbergPdfConfig *result = kreuzberg_default();
```

**Returns:** `KreuzbergPdfConfig`

---

#### KreuzbergPdfFormField

A form field extracted from a PDF's AcroForm or XFA structure.

Populated by the PDF extractor when `PdfConfig.extract_form_fields` is
enabled and the document is a fillable form. Supports both AcroForm (standard)
and XFA (XML Forms Architecture) layers. When both are present, AcroForm fields
take priority (canonical fallback per PDF spec), and XFA-only fields are appended.
The collection is empty for non-form PDFs and for non-PDF formats.

`PdfConfig.extract_form_fields`: crate.core.config.PdfConfig.extract_form_fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `const char*` | — | Partial field name (the leaf name within the field hierarchy). |
| `full_name` | `const char*` | — | Fully-qualified field name (dotted path from the form root). |
| `field_type` | `KreuzbergFormFieldType` | — | Classified field type. |
| `value` | `const char**` | `/* serde(default) */` | Current field value, if any. |
| `default_value` | `const char**` | `/* serde(default) */` | Default field value, if any. |
| `flags` | `uint32_t` | `/* serde(default) */` | Raw field-flags bitmask (read-only, required, multiline, …). |
| `page` | `uint32_t*` | `/* serde(default) */` | 1-indexed page the field's widget appears on. Currently always `NULL` for AcroForm fields; page assignment is a deferred enhancement requiring spatial analysis of widget annotations per page. |
| `bbox` | `KreuzbergBoundingBox*` | `/* serde(default) */` | Widget bounding box on its page, if known. |
| `max_length` | `uint32_t*` | `/* serde(default) */` | Maximum input length for text fields, if specified. |
| `tooltip` | `const char**` | `/* serde(default) */` | Tooltip / alternate field description, if present. |

---

#### KreuzbergPdfMetadata

PDF-specific metadata.

Contains metadata fields specific to PDF documents that are not in the common
`Metadata` structure. Common fields like title, authors, keywords, and dates
are at the `Metadata` level.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `pdf_version` | `const char**` | `NULL` | PDF version (e.g., "1.7", "2.0") |
| `producer` | `const char**` | `NULL` | PDF producer (application that created the PDF) |
| `is_encrypted` | `bool*` | `NULL` | Whether the PDF is encrypted/password-protected |
| `width` | `int64_t*` | `NULL` | First page width in points (1/72 inch) |
| `height` | `int64_t*` | `NULL` | First page height in points (1/72 inch) |
| `page_count` | `uint32_t*` | `NULL` | Total number of pages in the PDF document |

---

#### KreuzbergPlugin

Base trait that all plugins must implement.

This trait provides common functionality for plugin lifecycle management,
identification, and metadata.

##### Thread Safety

All plugins must be `Send + Sync` to support concurrent usage across threads.

##### Methods

###### kreuzberg_name()

Returns the unique name/identifier for this plugin.

The name should be:

- Unique across all plugins
- Lowercase with hyphens (e.g., "my-custom-plugin")
- URL-safe characters only

**Signature:**

```c
const char* kreuzberg_name();
```

**Example:**

```c
const char *result = kreuzberg_name(instance);
```

**Returns:** `const char*`

###### kreuzberg_version()

Returns the semantic version of this plugin.

Should follow semver format: `MAJOR.MINOR.PATCH`

Defaults to the kreuzberg crate version.

**Signature:**

```c
const char* kreuzberg_version();
```

**Example:**

```c
const char *result = kreuzberg_version(instance);
```

**Returns:** `const char*`

###### kreuzberg_initialize()

Initialize the plugin.

Called once when the plugin is registered. Use this to:

- Load configuration
- Initialize resources (connections, caches, etc.)
- Validate dependencies

##### Thread Safety

This method takes `&self` instead of `&mut self` to work with `Arc<dyn Plugin>`.
Plugins needing mutable state during initialization should use interior mutability
patterns (Mutex, RwLock, OnceCell, etc.).

**Errors:**

Should return an error if initialization fails. The plugin will not be
registered if this method returns an error.

Defaults to a no-op for stateless plugins.

**Signature:**

```c
void kreuzberg_initialize();
```

**Example:**

```c
kreuzberg_initialize(instance);
```

**Returns:** No return value.

**Errors:** Returns `NULL` on error.

###### kreuzberg_shutdown()

Shutdown the plugin.

Called when the plugin is being unregistered or the application is shutting down.
Use this to:

- Close connections
- Flush caches
- Release resources

##### Thread Safety

This method takes `&self` instead of `&mut self` to work with `Arc<dyn Plugin>`.
Plugins needing mutable state during shutdown should use interior mutability
patterns (Mutex, RwLock, etc.).

**Errors:**

Errors during shutdown are logged but don't prevent the shutdown process.

Defaults to a no-op for stateless plugins.

**Signature:**

```c
void kreuzberg_shutdown();
```

**Example:**

```c
kreuzberg_shutdown(instance);
```

**Returns:** No return value.

**Errors:** Returns `NULL` on error.

###### kreuzberg_description()

Optional plugin description for debugging and logging.

Defaults to empty string if not overridden.

**Signature:**

```c
const char* kreuzberg_description();
```

**Example:**

```c
const char *result = kreuzberg_description(instance);
```

**Returns:** `const char*`

###### kreuzberg_author()

Optional plugin author information.

Defaults to empty string if not overridden.

**Signature:**

```c
const char* kreuzberg_author();
```

**Example:**

```c
const char *result = kreuzberg_author(instance);
```

**Returns:** `const char*`

---

#### KreuzbergPostProcessor

Trait for post-processor plugins.

Post-processors transform or enrich extraction results after the initial
extraction is complete. They can:

- Clean and normalize text
- Add metadata (language, keywords, entities)
- Split content into chunks
- Score quality
- Apply custom transformations

##### Processing Order

Post-processors are executed in stage order:

1. **Early** - Language detection, entity extraction
2. **Middle** - Keyword extraction, token reduction
3. **Late** - Custom hooks, final validation

Within each stage, processors are executed in registration order.

##### Error Handling

Post-processor errors are non-fatal by default - they're captured in metadata
and execution continues. To make errors fatal, return an error from `process()`.

##### Thread Safety

Post-processors must be thread-safe (`Send + Sync`).

##### Methods

###### kreuzberg_process()

Process an extraction result.

Transform or enrich the extraction result. Can modify:

- `content` - The extracted text
- `metadata` - Add or update metadata fields
- `tables` - Modify or enhance table data

**Returns:**

`Ok(())` if processing succeeded, `Err(...)` for fatal failures.

**Errors:**

Return errors for fatal processing failures. Non-fatal errors should be
captured in metadata directly on the result.

##### Performance

This signature avoids unnecessary cloning of large extraction results by
taking a mutable reference instead of ownership. Processors modify the
result in place.

##### Example - Language Detection

##### Example - Text Cleaning

**Signature:**

```c
void kreuzberg_process(KreuzbergExtractionResult result, KreuzbergExtractionConfig config);
```

**Example:**

```c
kreuzberg_process(instance, NULL, NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `result` | `KreuzbergExtractionResult` | Yes | Mutable reference to the extraction result to process |
| `config` | `KreuzbergExtractionConfig` | Yes | Extraction configuration |

**Returns:** No return value.

**Errors:** Returns `NULL` on error.

###### kreuzberg_processing_stage()

Get the processing stage for this post-processor.

Determines when this processor runs in the pipeline.

**Returns:**

The `ProcessingStage` (Early, Middle, or Late).

**Signature:**

```c
KreuzbergProcessingStage kreuzberg_processing_stage();
```

**Example:**

```c
KreuzbergProcessingStage *result = kreuzberg_processing_stage(instance);
```

**Returns:** `KreuzbergProcessingStage`

###### kreuzberg_should_process()

Optional: Check if this processor should run for a given result.

Allows conditional processing based on MIME type, metadata, or content.
Defaults to `true` (always run).

**Returns:**

`true` if the processor should run, `false` to skip.

**Signature:**

```c
bool kreuzberg_should_process(KreuzbergExtractionResult result, KreuzbergExtractionConfig config);
```

**Example:**

```c
bool result = kreuzberg_should_process(instance, NULL, NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `result` | `KreuzbergExtractionResult` | Yes | The extraction result |
| `config` | `KreuzbergExtractionConfig` | Yes | The extraction config |

**Returns:** `bool`

###### kreuzberg_estimated_duration_ms()

Optional: Estimate processing time in milliseconds.

Used for logging and debugging. Defaults to 0 (unknown).

**Returns:**

Estimated processing time in milliseconds.

**Signature:**

```c
uint64_t kreuzberg_estimated_duration_ms(KreuzbergExtractionResult result);
```

**Example:**

```c
uint64_t result = kreuzberg_estimated_duration_ms(instance, NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `result` | `KreuzbergExtractionResult` | Yes | The extraction result |

**Returns:** `uint64_t`

###### kreuzberg_priority()

Execution priority within the processing stage.

Higher values run first within the same `ProcessingStage`. Defaults to 50.
Use 0-49 for fallback processors, 50 for normal processors, and 51-255
for high-priority processors that should run early in their stage.

**Signature:**

```c
int32_t kreuzberg_priority();
```

**Example:**

```c
int32_t result = kreuzberg_priority(instance);
```

**Returns:** `int32_t`

---

#### KreuzbergPostProcessorConfig

Post-processor configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `bool` | `true` | Enable post-processors |
| `enabled_processors` | `const char***` | `NULL` | Whitelist of processor names to run (None = all enabled) |
| `disabled_processors` | `const char***` | `NULL` | Blacklist of processor names to skip (None = none disabled) |
| `enabled_set` | `const char***` | `NULL` | Pre-computed AHashSet for O(1) enabled processor lookup |
| `disabled_set` | `const char***` | `NULL` | Pre-computed AHashSet for O(1) disabled processor lookup |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergPostProcessorConfig kreuzberg_default();
```

**Example:**

```c
KreuzbergPostProcessorConfig *result = kreuzberg_default();
```

**Returns:** `KreuzbergPostProcessorConfig`

---

#### KreuzbergPptxAppProperties

Application properties from docProps/app.xml for PPTX

Contains PowerPoint-specific document metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `application` | `const char**` | `NULL` | Application name (e.g., "Microsoft Office PowerPoint") |
| `app_version` | `const char**` | `NULL` | Application version |
| `total_time` | `int32_t*` | `NULL` | Total editing time in minutes |
| `company` | `const char**` | `NULL` | Company name |
| `doc_security` | `int32_t*` | `NULL` | Document security level |
| `scale_crop` | `bool*` | `NULL` | Scale crop flag |
| `links_up_to_date` | `bool*` | `NULL` | Links up to date flag |
| `shared_doc` | `bool*` | `NULL` | Shared document flag |
| `hyperlinks_changed` | `bool*` | `NULL` | Hyperlinks changed flag |
| `slides` | `int32_t*` | `NULL` | Number of slides |
| `notes` | `int32_t*` | `NULL` | Number of notes |
| `hidden_slides` | `int32_t*` | `NULL` | Number of hidden slides |
| `multimedia_clips` | `int32_t*` | `NULL` | Number of multimedia clips |
| `presentation_format` | `const char**` | `NULL` | Presentation format (e.g., "Widescreen", "Standard") |
| `slide_titles` | `const char**` | `NULL` | Slide titles |

---

#### KreuzbergPptxExtractionResult

PowerPoint (PPTX) extraction result.

Contains extracted slide content, metadata, and embedded images/tables.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `const char*` | — | Extracted text content from all slides |
| `metadata` | `KreuzbergPptxMetadata` | — | Presentation metadata |
| `slide_count` | `uintptr_t` | — | Total number of slides |
| `image_count` | `uintptr_t` | — | Total number of embedded images |
| `table_count` | `uintptr_t` | — | Total number of tables |
| `images` | `KreuzbergExtractedImage*` | — | Extracted images from the presentation |
| `page_structure` | `KreuzbergPageStructure*` | `NULL` | Slide structure with boundaries (when page tracking is enabled) |
| `page_contents` | `KreuzbergPageContent**` | `NULL` | Per-slide content (when page tracking is enabled) |
| `document` | `KreuzbergDocumentStructure*` | `NULL` | Structured document representation |
| `office_metadata` | `void*` | `/* serde(default) */` | Office metadata extracted from docProps/core.xml and docProps/app.xml. Contains keys like "title", "author", "created_by", "subject", "keywords", "modified_by", "created_at", "modified_at", etc. |
| `revisions` | `KreuzbergDocumentRevision**` | `/* serde(default) */` | Slide comments as revisions. Each `<p:cm>` element in `ppt/comments/comment{N}.xml` becomes a `DocumentRevision { kind: Comment }` with author (resolved from `ppt/commentAuthors.xml`), ISO-8601 timestamp, and `RevisionAnchor.Slide { index }`. `NULL` when no comment XML parts exist. |

---

#### KreuzbergPptxMetadata

PowerPoint presentation metadata.

Extracted from PPTX files containing slide counts and presentation details.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `slide_count` | `uint32_t` | — | Total number of slides in the presentation |
| `slide_names` | `const char**` | `NULL` | Names of slides (if available) |
| `image_count` | `uint32_t*` | `NULL` | Number of embedded images |
| `table_count` | `uint32_t*` | `NULL` | Number of tables |

---

#### KreuzbergPreset

A curated structured-extraction preset loaded from the embedded library.

Each preset is a JSON file under `src/presets/library/<id>/v1.json` that
validates against the meta-schema in `src/presets/preset.schema.json`.

The curated catalog is downstream (kreuzberg-cloud) and injects presets via
`extend_from_dir`. The embedded OSS library
ships only the `generic_document` toy preset.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `const char*` | — | Stable, URL-safe preset identifier (lowercase snake_case). |
| `version` | `const char*` | — | Monotonic version string (e.g. `v1`). |
| `schema_name` | `const char*` | — | Human-readable schema name forwarded to the LLM as the response/tool name. |
| `description` | `const char*` | — | One-line preset description shown in the registry UI. |
| `category` | `KreuzbergPresetCategory` | — | Top-level category for grouping in the playground. |
| `tags` | `const char**` | `/* serde(default) */` | Free-form tags used for search/filtering. May be empty. |
| `schema` | `void*` | — | JSON Schema (Draft 2020-12) describing the structured output shape. |
| `system_prompt` | `const char*` | — | Instruction primer sent to the model. |
| `context_template` | `const char**` | `/* serde(default) */` | Optional mustache-style template merged with caller-supplied context. |
| `merge_mode` | `KreuzbergMergeMode` | — | Strategy for merging per-batch outputs across paginated calls. |
| `preferred_call_mode` | `KreuzbergCallMode` | — | Default call mode suggested for this preset; heuristics may override. |
| `emit_citations` | `bool` | — | When true, the prompt asks the model to wrap each field as `{value, page, bbox, confidence}` for downstream citation overlays. |
| `sample` | `KreuzbergPresetSample*` | `/* serde(default) */` | Optional bundled sample (input file + reference output) for preview. |
| `fingerprint` | `const char*` | `/* serde(default) */` | Stable sha256 fingerprint of the canonical preset file contents. Populated at registry load — not present in the on-disk JSON files. Used as a cache-invalidation token by the worker pipeline. |

---

#### KreuzbergPresetSample

Pointer to a sample input + its reference output bundled with the preset.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `input_path` | `const char*` | — | Path to the sample input file, relative to the preset directory. |
| `output_path` | `const char*` | — | Path to the reference structured output, relative to the preset directory. |

---

#### KreuzbergPresetSummary

Lightweight projection of `Preset` used by the registry list endpoint
(omits the full schema and prompt to keep the payload small).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `const char*` | — | Preset identifier matching `Preset.id`. |
| `version` | `const char*` | — | Preset version matching `Preset.version`. |
| `schema_name` | `const char*` | — | Schema name matching `Preset.schema_name`. |
| `description` | `const char*` | — | One-line preset description. |
| `category` | `KreuzbergPresetCategory` | — | Top-level category. |
| `tags` | `const char**` | — | Free-form tags. |
| `preferred_call_mode` | `KreuzbergCallMode` | — | Default call mode. |
| `emit_citations` | `bool` | — | Whether the preset prompts the model for citations. |
| `fingerprint` | `const char*` | — | Stable fingerprint matching `Preset.fingerprint`. |

---

#### KreuzbergProcessingWarning

A non-fatal warning from a processing pipeline stage.

Captures errors from optional features that don't prevent extraction
but may indicate degraded results.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `source` | `const char*` | — | The pipeline stage or feature that produced this warning (e.g., "embedding", "chunking", "language_detection", "output_format"). |
| `message` | `const char*` | — | Human-readable description of what went wrong. |

---

#### KreuzbergPstMetadata

Outlook PST archive metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `message_count` | `uintptr_t` | — | Total number of email messages found in the PST archive. |

---

#### KreuzbergQrBoundingBox

Pixel-space bounding box of a QR code inside its source image.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `x` | `uint32_t` | — | Horizontal pixel offset of the bounding box top-left corner. |
| `y` | `uint32_t` | — | Vertical pixel offset of the bounding box top-left corner. |
| `width` | `uint32_t` | — | Width of the bounding box in pixels. |
| `height` | `uint32_t` | — | Height of the bounding box in pixels. |

---

#### KreuzbergQrCode

One QR code decoded from an extracted image.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `payload` | `const char*` | — | Decoded payload (text, URL, vCard string, …). |
| `confidence` | `float*` | `NULL` | Detector-reported confidence in `\[0.0, 1.0\]`. `NULL` when the decoder does not expose confidence (the default `rqrr` backend always reports `Some` because successful decode implies high confidence). |
| `bbox` | `KreuzbergQrBoundingBox*` | `NULL` | Bounding box of the QR code inside the source image, in pixel coordinates (`x`, `y` of the top-left corner; `width`, `height` of the rectangle). `NULL` if the decoder did not report a bounding box. |

---

#### KreuzbergRakeParams

RAKE-specific parameters.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `min_word_length` | `uintptr_t` | `1` | Minimum word length to consider (default: 1). |
| `max_words_per_phrase` | `uintptr_t` | `3` | Maximum words in a keyword phrase (default: 3). |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergRakeParams kreuzberg_default();
```

**Example:**

```c
KreuzbergRakeParams *result = kreuzberg_default();
```

**Returns:** `KreuzbergRakeParams`

---

#### KreuzbergRecognizedTable

Pre-computed table markdown for a table detection region.

Produced by the TATR-based table structure recognizer and surfaced as part of
layout-aware OCR results.  The struct lives here (under `layout-types`, pure-Rust)
so that consumers who do not enable `layout-detection` (ORT) can still reference
the type in their own code.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `detection_bbox` | `KreuzbergBBox` | — | Detection bbox that this table corresponds to (for matching). |
| `cells` | `const char***` | — | Table cells as a 2D vector (rows × columns). |
| `markdown` | `const char*` | — | Rendered markdown table. |

---

#### KreuzbergRedactionConfig

**Since:** `v5.0`

Configuration for the redaction post-processor.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `categories` | `KreuzbergPiiCategory*` | `NULL` | Categories to redact. Empty means "every category supported by the engine." |
| `strategy` | `KreuzbergRedactionStrategy` | `KREUZBERG_KREUZBERG_MASK` | Strategy applied to every match. |
| `ner` | `KreuzbergNerConfig*` | `NULL` | Optional NER backend — required to redact PERSON / ORGANIZATION / LOCATION categories (the pure-Rust pattern engine only covers regex-detectable PII). |
| `preserve_offsets` | `bool` | `true` | When `true`, chunk byte ranges are kept consistent with the rewritten content by adjusting `byte_start` / `byte_end` after replacement. When `false`, chunk byte ranges still refer to the *original* content offsets — useful when downstream consumers want to map findings back to the original document. |
| `custom_terms` | `KreuzbergRedactionTerm*` | `NULL` | Arbitrary user-supplied literal terms to redact. Each term is treated as a regex hit against the document, surfacing as `PiiCategory.Custom(label)` in `RedactionFinding` where `label` is the per-term label (defaulting to the literal value itself). Case-insensitive by default; set `RedactionTerm.case_sensitive` for exact match. Use this when you need to redact tenant-specific tokens (employee IDs, project codes, internal product names) without writing a custom plugin. |
| `custom_patterns` | `KreuzbergRedactionPattern*` | `NULL` | Arbitrary user-supplied regex patterns to redact. Same surfacing semantics as `custom_terms`: each hit becomes a `PiiCategory.Custom(label)` finding. Patterns are validated at config-construction time via `RedactionConfig.validate`. |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergRedactionConfig kreuzberg_default();
```

**Example:**

```c
KreuzbergRedactionConfig *result = kreuzberg_default();
```

**Returns:** `KreuzbergRedactionConfig`

###### kreuzberg_validate()

Validate user-supplied terms and patterns at config-construction time.

Compiles every `RedactionPattern.pattern` (with the case-insensitive
inline flag where applicable) and returns the first compilation error so
the caller can reject the config before the redaction pipeline runs.
Pure terms (regex-escaped) cannot fail to compile, but the function
still rejects empty values to avoid degenerate zero-length matches.

**Signature:**

```c
void kreuzberg_validate();
```

**Example:**

```c
kreuzberg_validate(instance);
```

**Returns:** No return value.

**Errors:** Returns `NULL` on error.

---

#### KreuzbergRedactionFinding

One redaction event: which span was rewritten, why, and with what.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `start` | `uint32_t` | — | Byte-offset start in the original (pre-redaction) `ExtractionResult.content`. |
| `end` | `uint32_t` | — | Byte-offset end (exclusive) in the original `ExtractionResult.content`. |
| `category` | `KreuzbergPiiCategory` | — | PII category that fired this redaction. |
| `strategy` | `KreuzbergRedactionStrategy` | — | Strategy applied to this finding (mask, hash, token-replace, drop). |
| `replacement_token` | `const char*` | — | String that replaced the original mention. Always present; for `Drop` the replacement is the empty string. |

---

#### KreuzbergRedactionPattern

One user-supplied regex pattern to redact.

The pattern is compiled with the Rust `regex` crate (no look-around). Case
sensitivity is encoded in the pattern via the `(?i)` inline flag when
`Self.case_sensitive` is `false`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `label` | `const char*` | — | Custom category label surfaced in `RedactionFinding.category`. |
| `pattern` | `const char*` | — | Regex pattern (Rust `regex` crate dialect — no look-around). |
| `case_sensitive` | `bool` | `/* serde(default) */` | When `true`, match case-sensitively; otherwise prepend `(?i)` to the regex. |

##### Methods

###### kreuzberg_labeled()

Build a pattern with the given label (case-insensitive by default).

**Signature:**

```c
KreuzbergRedactionPattern kreuzberg_labeled(const char* label, const char* pattern);
```

**Example:**

```c
KreuzbergRedactionPattern *result = kreuzberg_labeled("value", "value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `label` | `const char*` | Yes | The label |
| `pattern` | `const char*` | Yes | The pattern |

**Returns:** `KreuzbergRedactionPattern`

---

#### KreuzbergRedactionReport

Audit report describing what the redaction processor found and how it replaced it.

The redactor returns this alongside the rewritten content so compliance, replay, and
audit-log consumers can see exactly what fired. Offsets are relative to the *original*
pre-redaction `content` and are intended for audit reconstruction only — the original
bytes are dropped at the end of the pipeline.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `findings` | `KreuzbergRedactionFinding*` | — | Individual redaction findings in original-source byte order. |
| `total_redacted` | `uint32_t` | — | Total number of redactions applied across the document. |

---

#### KreuzbergRedactionTerm

One user-supplied literal term to redact.

Matched as a regex-escaped substring (so callers do not need to escape
metacharacters themselves). Case-insensitive by default — set
`Self.case_sensitive` to `true` for exact byte-match semantics.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `label` | `const char*` | — | Custom category label surfaced in `RedactionFinding.category`. |
| `value` | `const char*` | — | Literal value to match. Regex metacharacters are escaped automatically. |
| `case_sensitive` | `bool` | `/* serde(default) */` | When `true`, match the value as-is; otherwise match ASCII-case-insensitively. |

##### Methods

###### kreuzberg_literal()

Build a term whose label is the literal value itself (case-insensitive).

**Signature:**

```c
KreuzbergRedactionTerm kreuzberg_literal(const char* value);
```

**Example:**

```c
KreuzbergRedactionTerm *result = kreuzberg_literal("value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `value` | `const char*` | Yes | The value |

**Returns:** `KreuzbergRedactionTerm`

###### kreuzberg_labeled()

Build a term with a custom label.

**Signature:**

```c
KreuzbergRedactionTerm kreuzberg_labeled(const char* label, const char* value);
```

**Example:**

```c
KreuzbergRedactionTerm *result = kreuzberg_labeled("value", "value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `label` | `const char*` | Yes | The label |
| `value` | `const char*` | Yes | The value |

**Returns:** `KreuzbergRedactionTerm`

---

#### KreuzbergRegistry

Sorted map of preset id → `Preset`.

##### Methods

###### kreuzberg_load_embedded()

Build the registry from preset files embedded at compile time under
`src/presets/library/`. Validates every file against the meta-schema.

**Signature:**

```c
KreuzbergRegistry kreuzberg_load_embedded();
```

**Example:**

```c
KreuzbergRegistry *result = kreuzberg_load_embedded();
```

**Returns:** `KreuzbergRegistry`

**Errors:** Returns `NULL` on error.

###### kreuzberg_global()

Return the global registry, loading it on first access.

**Panics:**

Panics if any embedded preset is malformed. The build-time validation
test ensures this cannot happen for the embedded presets; a panic here
indicates a build artifact problem, not a runtime error.

**Signature:**

```c
KreuzbergRegistry kreuzberg_global();
```

**Example:**

```c
KreuzbergRegistry *result = kreuzberg_global();
```

**Returns:** `KreuzbergRegistry`

###### kreuzberg_get()

Look up a preset by its identifier.

**Signature:**

```c
KreuzbergPreset* kreuzberg_get(const char* id);
```

**Example:**

```c
KreuzbergPreset* result = kreuzberg_get(instance, "value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `id` | `const char*` | Yes | The id |

**Returns:** `KreuzbergPreset*`

###### kreuzberg_summaries()

Materialize a `PresetSummary` list for the public registry endpoint.

**Signature:**

```c
KreuzbergPresetSummary* kreuzberg_summaries();
```

**Example:**

```c
KreuzbergPresetSummary* result = kreuzberg_summaries(instance);
```

**Returns:** `KreuzbergPresetSummary*`

###### kreuzberg_len()

Number of presets currently loaded.

**Signature:**

```c
uintptr_t kreuzberg_len();
```

**Example:**

```c
uintptr_t result = kreuzberg_len(instance);
```

**Returns:** `uintptr_t`

###### kreuzberg_is_empty()

Whether the registry contains zero presets.

**Signature:**

```c
bool kreuzberg_is_empty();
```

**Example:**

```c
bool result = kreuzberg_is_empty(instance);
```

**Returns:** `bool`

###### kreuzberg_sample_bytes()

Read raw sample bytes for `<preset_id>` from
`library/<id>/samples/<name>`. Returns `NULL` when the file is absent.

**Signature:**

```c
const uint8_t** kreuzberg_sample_bytes(const char* preset_id, const char* name);
```

**Example:**

```c
const uint8_t** result = kreuzberg_sample_bytes(instance, "value", "value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `preset_id` | `const char*` | Yes | The preset id |
| `name` | `const char*` | Yes | The name |

**Returns:** `const uint8_t**`

###### kreuzberg_extend_from_dir()

Load additional preset files from a runtime directory and insert them
into this registry.

Reads every `*.json` file directly under `dir` (non-recursive),
validates each against the meta-schema, and inserts it. Files that fail
validation are rejected — the error is returned immediately and the
registry is left in a partially-updated state. Existing entries with the
same id are overwritten.

Returns the number of presets successfully loaded from `dir`.

##### Use case

This is the injection point for downstream catalogs: kreuzberg-cloud
calls this once at startup to add its 20+ curated presets on top of the
single embedded OSS preset.

**Signature:**

```c
uintptr_t kreuzberg_extend_from_dir(const char* dir);
```

**Example:**

```c
uintptr_t result = kreuzberg_extend_from_dir(instance, "value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `dir` | `const char*` | Yes | The dir |

**Returns:** `uintptr_t`

**Errors:** Returns `NULL` on error.

---

#### KreuzbergRenderer

Trait for document renderers that convert `InternalDocument` to output strings.

Renderers are typically stateless converters that transform the internal
document representation into a specific output format (Markdown, HTML,
Djot, plain text, etc.). They participate in the standard `Plugin`
lifecycle so custom renderers can be registered from any supported binding
language.

The format name is exposed via `Plugin.name`. For stateless renderers
the `Plugin` lifecycle methods (`version`, `initialize`, `shutdown`) all
take no-op defaults and need not be overridden.

##### Thread Safety

Renderers must be `Send + Sync` (inherited from `Plugin`).

##### Methods

###### kreuzberg_render()

Render an `InternalDocument` to the output format.

**Returns:**

The rendered output as a string.

**Errors:**

Returns an error if rendering fails.

**Signature:**

```c
const char* kreuzberg_render(KreuzbergInternalDocument doc);
```

**Example:**

```c
const char *result = kreuzberg_render(instance, NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `doc` | `KreuzbergInternalDocument` | Yes | The internal document to render |

**Returns:** `const char*`

**Errors:** Returns `NULL` on error.

---

#### KreuzbergRerankedDocument

A single document returned by the reranker, with its position in the input and score.

`index` maps back to the caller's original document list, so metadata arrays
(e.g. IDs, paths) can be reordered without passing them through the reranker.

Since v5.0.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `index` | `uintptr_t` | — | Position of this document in the original input `documents` slice. |
| `score` | `float` | — | Relevance score in `\[0, 1\]`. Higher means more relevant to the query. |
| `document` | `const char*` | — | The document text. |

---

#### KreuzbergRerankerBackend

Trait for in-process reranker backend plugins.

Cross-encoders score `(query, document)` pairs jointly and return a
raw logit per document. The dispatcher in `rerank` applies
sigmoid to convert logits to `[0, 1]` scores, sorts descending by score,
and truncates to `top_k`.

Async to match the convention used by `EmbeddingBackend`
and other plugin traits. Host-language bridges wrap their synchronous
host callables in `spawn_blocking` or the equivalent.

##### Thread safety

Backends must be `Send + Sync + 'static`. They are stored in
`Arc<dyn RerankerBackend>` and may be called concurrently from kreuzberg's
dispatcher. If the backend's underlying model is not thread-safe, the
backend itself must serialize access internally (e.g. via `Mutex<Inner>`).

##### Contract

- `rerank(query, documents)` MUST return exactly `documents.len()` scores.
  The dispatcher validates this before sorting and returning to callers;
  a non-conforming backend surfaces as a `KreuzbergError.Validation`, not
  a panic.

- Scores are raw logits in any range — callers must NOT assume `[0, 1]`.
  The dispatcher applies sigmoid before sorting.

- `rerank` may be called from any thread. Its future must be `Send`
  (enforced by `async_trait` when `#[async_trait]` is used on non-WASM
  targets).

- `shutdown()` (inherited from `Plugin`) may be invoked
  concurrently with an in-flight `rerank()` call. Implementations must
  tolerate this — letting in-flight calls finish via the `Arc` reference
  and only releasing shared state that isn't needed by `rerank`.

##### Runtime

The synchronous `rerank` entry uses
`tokio.task.block_in_place` to await the trait's async `rerank`, which
requires a multi-thread tokio runtime. Callers running inside a
`current_thread` runtime must use `rerank_async` instead.

Since v5.0.

##### Methods

###### kreuzberg_rerank()

Score a list of documents against a query.

Returns one raw logit per document in the same order as the input.
The dispatcher applies sigmoid to convert to `[0, 1]` scores.

**Errors:**

Implementations should return `Plugin` for
backend-specific failures. The dispatcher validates the returned length
against `documents.len()` before sorting.

**Signature:**

```c
float* kreuzberg_rerank(const char* query, const char** documents);
```

**Example:**

```c
float* result = kreuzberg_rerank(instance, "value", NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `query` | `const char*` | Yes | The query |
| `documents` | `const char**` | Yes | The documents |

**Returns:** `float*`

**Errors:** Returns `NULL` on error.

---

#### KreuzbergRerankerConfig

Configuration for the reranking pipeline.

Controls which model to use, how many results to return, and download/cache
behavior for local ONNX models.

Since v5.0.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `KreuzbergRerankerModelType` | `KREUZBERG_KREUZBERG_PRESET` | The reranker model to use (defaults to "balanced" preset if not specified). |
| `top_k` | `uintptr_t*` | `NULL` | Return at most this many documents. `NULL` returns all. Applied after sorting by score, so the highest-scoring documents are kept. |
| `batch_size` | `uintptr_t` | `32` | Batch size for local ONNX cross-encoder inference. |
| `show_download_progress` | `bool` | `false` | Show model download progress (local ONNX path only). |
| `cache_dir` | `const char**` | `NULL` | Custom cache directory for model files. Defaults to `~/.cache/kreuzberg/rerankers/` if not specified. |
| `acceleration` | `KreuzbergAccelerationConfig*` | `NULL` | Hardware acceleration for the reranker ONNX model. Controls which execution provider (CPU, CUDA, CoreML, TensorRT) is used for local inference. Defaults to `NULL` (auto-select per platform). |
| `max_rerank_duration_secs` | `uint64_t*` | `NULL` | Maximum wall-clock duration (in seconds) for a single `rerank()` call when using `RerankerModelType.Plugin`. Applies only to the in-process plugin path — protects against hung host-language backends. On timeout, the dispatcher returns `Plugin` instead of blocking forever. `NULL` disables the timeout. The default (60 seconds) is conservative for common in-process inference; increase for large document sets on slow hardware. |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergRerankerConfig kreuzberg_default();
```

**Example:**

```c
KreuzbergRerankerConfig *result = kreuzberg_default();
```

**Returns:** `KreuzbergRerankerConfig`

---

#### KreuzbergRerankerPreset

Metadata for a bundled reranker preset.

All string fields are owned `String` for FFI compatibility — instances are
safe to clone and pass across language boundaries.

Since v5.0.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `const char*` | — | Short identifier (catalog name, e.g. `"bge-reranker-base"`). |
| `model_repo` | `const char*` | — | HuggingFace repository name for the model. |
| `model_file` | `const char*` | — | Path to the ONNX model file within the repo. |
| `additional_files` | `const char**` | `/* serde(default) */` | Sibling files that must be downloaded alongside `model_file`. Empty for most presets. Used by repos that split the weight blob — e.g. `rozgo/bge-reranker-v2-m3` ships the model in `model.onnx` plus a co-located `model.onnx.data` payload. |
| `max_length` | `uintptr_t` | — | Maximum token sequence length the model supports. |
| `description` | `const char*` | — | Human-readable description of the preset's intended use case. |

---

#### KreuzbergResolvedPreset

A preset merged with caller-supplied overrides (custom schema, prompt suffix,
context map). Output is what the pipeline orchestrator consumes.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `const char*` | — | Source preset identifier. |
| `version` | `const char*` | — | Source preset version. |
| `fingerprint` | `const char*` | — | Fingerprint of the source preset file, used as a cache token. |
| `schema_name` | `const char*` | — | Schema name forwarded to the LLM. |
| `schema` | `void*` | — | Effective JSON Schema (caller override or the preset's own). |
| `system_prompt` | `const char*` | — | System prompt with rendered context appended. |
| `merge_mode` | `KreuzbergMergeMode` | — | Merge strategy for paginated outputs. |
| `preferred_call_mode` | `KreuzbergCallMode` | — | Preferred call mode. |
| `emit_citations` | `bool` | — | Whether the prompt asks for per-field citations. |

---

#### KreuzbergRevisionDelta

The content changes that make up a single revision.

For insertions and deletions the `content` field carries the added/removed
lines as `DiffLine.Added` / `DiffLine.Removed` entries. For format
changes, `content` is empty — the property diff is left as a TODO for a
later enrichment pass.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `KreuzbergDiffLine*` | `NULL` | Line-level content changes for this revision. |
| `table_changes` | `KreuzbergCellChange*` | `NULL` | Cell-level table changes for this revision. |

---

#### KreuzbergSecurityLimits

Configuration for security limits across extractors.

All limits are intentionally conservative to prevent DoS attacks
while still supporting legitimate documents.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_archive_size` | `uintptr_t` | `524288000` | Maximum uncompressed size for archives (500 MB) |
| `max_compression_ratio` | `uintptr_t` | `100` | Maximum compression ratio before flagging as potential bomb (100:1) |
| `max_files_in_archive` | `uintptr_t` | `10000` | Maximum number of files in archive (10,000) |
| `max_nesting_depth` | `uintptr_t` | `1024` | Maximum nesting depth for structures (100) |
| `max_entity_length` | `uintptr_t` | `1048576` | Maximum length of any single XML entity / attribute / token (1 MiB). This is a per-token cap, NOT a total cap — billion-laughs class attacks where a single entity expands to hundreds of MB are caught here, while normal long text content (a paragraph, a CDATA block) is caught by `max_content_size` instead. |
| `max_content_size` | `uintptr_t` | `104857600` | Maximum string growth per document (100 MB) |
| `max_iterations` | `uintptr_t` | `10000000` | Maximum iterations per operation |
| `max_xml_depth` | `uintptr_t` | `1024` | Maximum XML depth (100 levels) |
| `max_table_cells` | `uintptr_t` | `100000` | Maximum cells per table (100,000) |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergSecurityLimits kreuzberg_default();
```

**Example:**

```c
KreuzbergSecurityLimits *result = kreuzberg_default();
```

**Returns:** `KreuzbergSecurityLimits`

---

#### KreuzbergServerConfig

API server configuration.

This struct holds all configuration options for the Kreuzberg API server,
including host/port settings, CORS configuration, and upload limits.

##### Defaults

- `host`: "127.0.0.1" (localhost only)
- `port`: 8000
- `cors_origins`: empty listtor (allows all origins)
- `max_request_body_bytes`: 104_857_600 (100 MB)
- `max_multipart_field_bytes`: 104_857_600 (100 MB)

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `host` | `const char*` | — | Server host address (e.g., "127.0.0.1", "0.0.0.0") |
| `port` | `uint16_t` | — | Server port number |
| `cors_origins` | `const char**` | `NULL` | CORS allowed origins. Empty vector means allow all origins. If this is an empty listtor, the server will accept requests from any origin. If populated with specific origins (e.g., `"<https://example.com"`>), only those origins will be allowed. |
| `max_request_body_bytes` | `uintptr_t` | — | Maximum size of request body in bytes (default: 100 MB) |
| `max_multipart_field_bytes` | `uintptr_t` | — | Maximum size of multipart fields in bytes (default: 100 MB) |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergServerConfig kreuzberg_default();
```

**Example:**

```c
KreuzbergServerConfig *result = kreuzberg_default();
```

**Returns:** `KreuzbergServerConfig`

###### kreuzberg_listen_addr()

Get the server listen address (host:port).

**Signature:**

```c
const char* kreuzberg_listen_addr();
```

**Example:**

```c
const char *result = kreuzberg_listen_addr(instance);
```

**Returns:** `const char*`

###### kreuzberg_cors_allows_all()

Check if CORS allows all origins.

Returns `true` if the `cors_origins` vector is empty, meaning all origins
are allowed. Returns `false` if specific origins are configured.

**Signature:**

```c
bool kreuzberg_cors_allows_all();
```

**Example:**

```c
bool result = kreuzberg_cors_allows_all(instance);
```

**Returns:** `bool`

###### kreuzberg_is_origin_allowed()

Check if a given origin is allowed by CORS configuration.

Returns `true` if:

- CORS allows all origins (empty origins list), or
- The given origin is in the allowed origins list

**Signature:**

```c
bool kreuzberg_is_origin_allowed(const char* origin);
```

**Example:**

```c
bool result = kreuzberg_is_origin_allowed(instance, "value");
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `origin` | `const char*` | Yes | The origin to check (e.g., "<https://example.com">) |

**Returns:** `bool`

###### kreuzberg_max_request_body_mb()

Get maximum request body size in megabytes (rounded up).

**Signature:**

```c
uintptr_t kreuzberg_max_request_body_mb();
```

**Example:**

```c
uintptr_t result = kreuzberg_max_request_body_mb(instance);
```

**Returns:** `uintptr_t`

###### kreuzberg_max_multipart_field_mb()

Get maximum multipart field size in megabytes (rounded up).

**Signature:**

```c
uintptr_t kreuzberg_max_multipart_field_mb();
```

**Example:**

```c
uintptr_t result = kreuzberg_max_multipart_field_mb(instance);
```

**Returns:** `uintptr_t`

---

#### KreuzbergStructuredData

Structured data (Schema.org, microdata, RDFa) block.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `data_type` | `KreuzbergStructuredDataType` | — | Type of structured data |
| `raw_json` | `const char*` | — | Raw JSON string representation |
| `schema_type` | `const char**` | `NULL` | Schema type if detectable (e.g., "Article", "Event", "Product") |

---

#### KreuzbergStructuredDataResult

Result of parsing a structured data file (JSON, JSONL, YAML, or TOML).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `const char*` | — | The extracted text content, formatted for readability. |
| `format` | `const char*` | — | The source format identifier (e.g. `"json"`, `"yaml"`, `"toml"`). |
| `metadata` | `void*` | — | Key-value metadata extracted from recognized text fields. |
| `text_fields` | `const char**` | — | JSON paths of fields that were classified as text-bearing. |

---

#### KreuzbergStructuredExtractionConfig

Configuration for LLM-based structured data extraction.

Sends extracted document content to a VLM with a JSON schema,
returning structured data that conforms to the schema.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `schema` | `void*` | — | JSON Schema defining the desired output structure. |
| `schema_name` | `const char*` | `/* serde(default) */` | Schema name passed to the LLM's structured output mode. |
| `schema_description` | `const char**` | `/* serde(default) */` | Optional schema description for the LLM. |
| `strict` | `bool` | `/* serde(default) */` | Enable strict mode — output must exactly match the schema. |
| `prompt` | `const char**` | `/* serde(default) */` | Custom Jinja2 extraction prompt template. When `NULL`, a default template is used. Available template variables: - `{{ content }}` — The extracted document text. - `{{ schema }}` — The JSON schema as a formatted string. - `{{ schema_name }}` — The schema name. - `{{ schema_description }}` — The schema description (may be empty). |
| `llm` | `KreuzbergLlmConfig` | — | LLM configuration for the extraction. |

---

#### KreuzbergStructuredInput

Signals consumed by the call-mode heuristic.

All fields derive from a prior kreuzberg extraction — no double-work.
This is a plain DTO; it intentionally has no dependency on internal
kreuzberg extraction types so it can be constructed from any source.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `mime_type` | `const char*` | — | MIME type, canonicalised to lowercase by the caller. |
| `page_count` | `uint32_t` | — | Number of pages in the document. |
| `text_coverage` | `double` | — | Fraction of pages with a real text layer (0.0..=1.0). |
| `avg_chars_per_page` | `double` | — | Average extracted characters per page. |
| `embedded_image_count` | `uint32_t` | — | Count of embedded images (figures, photos, signatures) discovered. |
| `user_force_vision` | `bool` | — | When `true`, promote the result to at least `StructuredCallMode.TextPlusVision`. |

---

#### KreuzbergStructuredThresholds

Thresholds for the structured-extraction call-mode heuristic.

All defaults are **conservative starting points**.  Deployments should
measure their own document corpus and override via their own config;
these values are chosen to be safe-by-default, not to be optimal for
any particular workload.

Construct custom thresholds with struct-update syntax:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `scan_max_coverage` | `double` | `0.1` | PDFs with `text_coverage` strictly below this are treated as scanned. **Conservative default: 0.10** — deployments override via their own config after measuring their document corpus. |
| `digital_min_coverage` | `double` | `0.9` | PDFs with `text_coverage` at or above this AND zero embedded images route to `StructuredCallMode.TextOnly`. **Conservative default: 0.90** — deployments override via their own config after measuring their document corpus. |
| `docx_text_min_density` | `double` | `200` | DOCX / HTML / text documents with `avg_chars_per_page` above this route to `StructuredCallMode.TextOnly`. **Conservative default: 200.0** — deployments override via their own config after measuring their document corpus. |
| `enable_vision_fallback` | `bool` | `false` | When `true`, emit `StructuredCallMode.TextOnlyWithVisionFallback` instead of `StructuredCallMode.TextOnly` so the orchestrator can escalate to vision on low confidence. **Conservative default: `false`** — must be explicitly enabled per deployment after bench validation; deployments override via their own config. |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergStructuredThresholds kreuzberg_default();
```

**Example:**

```c
KreuzbergStructuredThresholds *result = kreuzberg_default();
```

**Returns:** `KreuzbergStructuredThresholds`

---

#### KreuzbergSummarizationConfig

**Since:** `v5.0`

Configuration for the summarisation post-processor.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `strategy` | `KreuzbergSummaryStrategy` | `KREUZBERG_KREUZBERG_EXTRACTIVE` | Summarisation strategy. |
| `max_tokens` | `uint32_t*` | `NULL` | Maximum summary length in tokens. `NULL` lets the backend pick a default. |
| `llm` | `KreuzbergLlmConfig*` | `NULL` | LLM configuration for the abstractive backend. Ignored when `strategy = Extractive`. Required when `strategy = Abstractive`. |

---

#### KreuzbergSupportedFormat

A supported document format entry.

Represents a file extension and its corresponding MIME type that Kreuzberg can process.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `extension` | `const char*` | — | File extension (without leading dot), e.g., "pdf", "docx" |
| `mime_type` | `const char*` | — | MIME type string, e.g., "application/pdf" |

---

#### KreuzbergSvgOptions

SVG-specific configuration for the image-encode pipeline.

Applies when the source image is SVG or when the output format is set to
`ImageOutputFormat.Svg`.  Available when the `svg` feature is active.

Used via `ImageExtractionConfig.svg`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `sanitize` | `bool` | `true` | Run SVG bytes through `usvg` sanitization (strips external `href` attributes, JavaScript event handlers, and `foreignObject` elements) even when the output format is `Native`.  Defaults to `true`. |
| `render_dpi` | `float` | `96` | Target DPI when rasterizing SVG to a pixel-based format (PNG, JPEG, WebP, HEIF).  The tree's viewBox is scaled by `render_dpi / 96.0` before the pixel buffer is allocated.  Defaults to `96.0` (1× CSS pixel density). |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergSvgOptions kreuzberg_default();
```

**Example:**

```c
KreuzbergSvgOptions *result = kreuzberg_default();
```

**Returns:** `KreuzbergSvgOptions`

---

#### KreuzbergTable

Extracted table structure.

Represents a table detected and extracted from a document (PDF, image, etc.).
Tables are converted to both structured cell data and Markdown format.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `cells` | `const char***` | `NULL` | Table cells as a 2D vector (rows × columns) |
| `markdown` | `const char*` | — | Markdown representation of the table |
| `page_number` | `uint32_t` | — | Page number where the table was found (1-indexed) |
| `bounding_box` | `KreuzbergBoundingBox*` | `NULL` | Bounding box of the table on the page (PDF coordinates: x0=left, y0=bottom, x1=right, y1=top). Only populated for PDF-extracted tables when position data is available. |

---

#### KreuzbergTableCell

Individual table cell with content and optional styling.

Future extension point for rich table support with cell-level metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `const char*` | — | Cell content as text |
| `row_span` | `uint32_t` | — | Row span (number of rows this cell spans) |
| `col_span` | `uint32_t` | — | Column span (number of columns this cell spans) |
| `is_header` | `bool` | — | Whether this is a header cell |

---

#### KreuzbergTableDiff

Cell-level changes for a pair of tables that share the same index.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `from_index` | `uintptr_t` | — | Zero-based index of the table in both `a.tables` and `b.tables`. |
| `to_index` | `uintptr_t` | — | Zero-based index in `b.tables` (equal to `from_index` for same-dimension tables). |
| `cell_changes` | `KreuzbergCellChange*` | — | Cell-level changes within the table. |

---

#### KreuzbergTableGrid

Structured table grid with cell-level metadata.

Stores row/column dimensions and a flat list of cells with position info.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `rows` | `uint32_t` | — | Number of rows in the table. |
| `cols` | `uint32_t` | — | Number of columns in the table. |
| `cells` | `KreuzbergGridCell*` | `NULL` | All cells in row-major order. |

---

#### KreuzbergTesseractConfig

Tesseract OCR configuration.

Provides fine-grained control over Tesseract OCR engine parameters.
Most users can use the defaults, but these settings allow optimization
for specific document types (invoices, handwriting, etc.).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `language` | `const char*` | `"eng"` | Language code (e.g., "eng", "deu", "fra") |
| `psm` | `int32_t` | `3` | Page Segmentation Mode (0-13). Common values: - 3: Fully automatic page segmentation (native default) - 6: Assume a single uniform block of text (WASM default — avoids layout-analysis hang) - 11: Sparse text with no particular order |
| `output_format` | `const char*` | `"markdown"` | Output format ("text" or "markdown") |
| `oem` | `int32_t` | `3` | OCR Engine Mode (0-3). - 0: Legacy engine only - 1: Neural nets (LSTM) only (usually best) - 2: Legacy + LSTM - 3: Default (based on what's available) |
| `min_confidence` | `double` | `0` | Minimum confidence threshold (0.0-100.0). Words with confidence below this threshold may be rejected or flagged. |
| `preprocessing` | `KreuzbergImagePreprocessingConfig*` | `NULL` | Image preprocessing configuration. Controls how images are preprocessed before OCR. Can significantly improve quality for scanned documents or low-quality images. |
| `enable_table_detection` | `bool` | `true` | Enable automatic table detection and reconstruction |
| `table_min_confidence` | `double` | `0` | Minimum confidence threshold for table detection (0.0-1.0) |
| `table_column_threshold` | `int32_t` | `50` | Column threshold for table detection (pixels) |
| `table_row_threshold_ratio` | `double` | `0.5` | Row threshold ratio for table detection (0.0-1.0) |
| `use_cache` | `bool` | `true` | Enable OCR result caching |
| `classify_use_pre_adapted_templates` | `bool` | `true` | Use pre-adapted templates for character classification |
| `language_model_ngram_on` | `bool` | `false` | Enable N-gram language model |
| `tessedit_dont_blkrej_good_wds` | `bool` | `true` | Don't reject good words during block-level processing |
| `tessedit_dont_rowrej_good_wds` | `bool` | `true` | Don't reject good words during row-level processing |
| `tessedit_enable_dict_correction` | `bool` | `true` | Enable dictionary correction |
| `tessedit_char_whitelist` | `const char*` | `""` | Whitelist of allowed characters (empty = all allowed) |
| `tessedit_char_blacklist` | `const char*` | `""` | Blacklist of forbidden characters (empty = none forbidden) |
| `tessedit_use_primary_params_model` | `bool` | `true` | Use primary language params model |
| `textord_space_size_is_variable` | `bool` | `true` | Variable-width space detection |
| `thresholding_method` | `bool` | `false` | Use adaptive thresholding method |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergTesseractConfig kreuzberg_default();
```

**Example:**

```c
KreuzbergTesseractConfig *result = kreuzberg_default();
```

**Returns:** `KreuzbergTesseractConfig`

---

#### KreuzbergTextAnnotation

Inline text annotation — byte-range based formatting and links.

Annotations reference byte offsets into the node's text content,
enabling precise identification of formatted regions.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `start` | `uint32_t` | — | Start byte offset in the node's text content (inclusive). |
| `end` | `uint32_t` | — | End byte offset in the node's text content (exclusive). |
| `kind` | `KreuzbergAnnotationKind` | — | Annotation type. |

---

#### KreuzbergTextExtractionResult

Plain text and Markdown extraction result.

Contains the extracted text along with statistics and,
for Markdown files, structural elements like headers and links.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `const char*` | — | Extracted text content |
| `line_count` | `uintptr_t` | — | Number of lines |
| `word_count` | `uintptr_t` | — | Number of words |
| `character_count` | `uintptr_t` | — | Number of characters |
| `headers` | `const char***` | `NULL` | Markdown headers (text only, Markdown files only) |

---

#### KreuzbergTextMetadata

Text/Markdown metadata.

Extracted from plain text and Markdown files. Includes word counts and,
for Markdown, structural elements like headers and links.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `line_count` | `uint32_t` | — | Number of lines in the document |
| `word_count` | `uint32_t` | — | Number of words |
| `character_count` | `uint32_t` | — | Number of characters |
| `headers` | `const char***` | `NULL` | Markdown headers (headings text only, for Markdown files) |

---

#### KreuzbergTokenCounter

Per-category running counter for `RedactionStrategy.TokenReplace`.

##### Methods

###### kreuzberg_new()

Create a fresh counter with no previous state.

**Signature:**

```c
KreuzbergTokenCounter kreuzberg_new();
```

**Example:**

```c
KreuzbergTokenCounter *result = kreuzberg_new();
```

**Returns:** `KreuzbergTokenCounter`

---

#### KreuzbergTokenReductionConfig

Configuration for the token-reduction pipeline.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `level` | `KreuzbergReductionLevel` | `KREUZBERG_KREUZBERG_MODERATE` | Reduction intensity level. |
| `language_hint` | `const char**` | `NULL` | ISO 639-1 language code hint for stopword selection (e.g. `"en"`, `"de"`). |
| `preserve_markdown` | `bool` | `false` | Preserve Markdown formatting tokens during reduction. |
| `preserve_code` | `bool` | `true` | Preserve code block contents unchanged. |
| `semantic_threshold` | `float` | `0.3` | Cosine similarity threshold below which sentences are considered dissimilar. |
| `enable_parallel` | `bool` | `true` | Use Rayon parallel iterators for multi-core processing. |
| `use_simd` | `bool` | `true` | Use SIMD-optimized text scanning where available. |
| `custom_stopwords` | `void**` | `NULL` | Per-language custom stopword lists (`language_code → stopword_list`). |
| `preserve_patterns` | `const char**` | `NULL` | Regex patterns whose matched text is always preserved unchanged. |
| `target_reduction` | `float*` | `NULL` | Target fraction of text to retain (0.0–1.0); `NULL` = no fixed target. |
| `enable_semantic_clustering` | `bool` | `false` | Group semantically similar sentences and emit only one per cluster. |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergTokenReductionConfig kreuzberg_default();
```

**Example:**

```c
KreuzbergTokenReductionConfig *result = kreuzberg_default();
```

**Returns:** `KreuzbergTokenReductionConfig`

---

#### KreuzbergTokenReductionOptions

Token reduction configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `mode` | `const char*` | — | Reduction mode: "off", "light", "moderate", "aggressive", "maximum" |
| `preserve_important_words` | `bool` | `true` | Preserve important words (capitalized, technical terms) |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergTokenReductionOptions kreuzberg_default();
```

**Example:**

```c
KreuzbergTokenReductionOptions *result = kreuzberg_default();
```

**Returns:** `KreuzbergTokenReductionOptions`

---

#### KreuzbergTranscriptionConfig

Configuration for audio/video transcription (speech-to-text).

When present and `enabled`, Kreuzberg will route audio and video files
(mp3, mp4, m4a, wav, webm, etc.) through the transcription pipeline.

The heavy dependencies (ORT, hf-hub, symphonia) are only pulled when the
`transcription` feature is enabled. The config struct itself is available
under `transcription-types` so that `ExtractionConfig` round-trips on all
targets.

All fields have sensible defaults. The recommended starting point is:

```toml
[extraction.transcription]
enabled = true
model = "tiny"
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `bool` | `true` | Master switch. When false the block is ignored and audio files fall back to the normal "unsupported format" path. |
| `model` | `KreuzbergWhisperModel` | `KREUZBERG_KREUZBERG_TINY` | Whisper model size to use. Smaller = faster + lower memory. `tiny` is the pragmatic default for first-time users and CI. |
| `language` | `const char**` | `NULL` | Optional language hint (ISO-639-1 code, e.g. "en", "de"). When `NULL` (default), the current engine falls back to English. For deterministic production output, always set this explicitly. |
| `timestamps` | `bool` | `false` | Whether to request segment-level timestamps. Accepted for forward compatibility. The current engine always uses `<\|notimestamps\|>` and does not emit segment metadata yet. |
| `max_duration_ms` | `uint64_t*` | `NULL` | Hard safety limit on input duration (milliseconds). Files longer than this are rejected after decode, before model work. Default: 30 minutes. Set to `NULL` to disable (not recommended for untrusted input). |
| `max_bytes` | `uint64_t*` | `NULL` | Hard safety limit on input size (bytes). Default: 512 MiB. Protects against pathological or malicious uploads. |
| `timeout_ms` | `uint64_t*` | `NULL` | Wall-clock timeout for the entire transcription operation (ms). Default: 10 minutes. Reserved for timeout enforcement; the current extractor does not enforce this field yet. |
| `model_cache_dir` | `const char**` | `NULL` | Override the directory used for Whisper model cache. When `NULL`, uses the centralized resolver: `KREUZBERG_CACHE_DIR/whisper` or the platform default (`~/.cache/kreuzberg/whisper` on Linux, etc.). |
| `allow_network` | `bool` | `true` | Allow network access to download models from Hugging Face Hub. When `false`, only previously cached models may be used. Useful for air-gapped or fully offline deployments. |
| `verify_hash` | `bool` | `true` | Request SHA256 verification of downloaded model files. Reserved for the checksum table follow-up. The current resolver logs a warning and treats this as a no-op. |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergTranscriptionConfig kreuzberg_default();
```

**Example:**

```c
KreuzbergTranscriptionConfig *result = kreuzberg_default();
```

**Returns:** `KreuzbergTranscriptionConfig`

---

#### KreuzbergTranslation

Translation of the extracted content.

Holds the translated rendition of `ExtractionResult.content` and (when
`preserve_markup` was requested) the translated `formatted_content`. Chunks
are translated in place inside `ExtractionResult.chunks[*].content` rather
than duplicated here.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `target_lang` | `const char*` | — | BCP-47 language tag the translation was produced into (e.g. `"de"`, `"fr-CA"`). |
| `source_lang` | `const char**` | `NULL` | BCP-47 source language. `NULL` when the translation backend was asked to detect. |
| `content` | `const char*` | — | Translated plain-text body. Matches the shape of `ExtractionResult.content`. |
| `formatted_content` | `const char**` | `NULL` | Translated markup body (Markdown / HTML / etc.) when `preserve_markup` was enabled on the config. `NULL` otherwise. |

---

#### KreuzbergTranslationConfig

**Since:** `v5.0`

Configuration for the translation post-processor.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `target_lang` | `const char*` | — | BCP-47 language tag for the target language (e.g. `"de"`, `"fr-CA"`). |
| `source_lang` | `const char**` | `NULL` | Optional explicit source language. `NULL` asks the backend to auto-detect. |
| `preserve_markup` | `bool` | `/* serde(default) */` | Translate the formatted (Markdown/HTML) rendition alongside plain text when `formatted_content` is present. |
| `llm` | `KreuzbergLlmConfig` | — | LLM configuration used for translation. |

---

#### KreuzbergTreeSitterConfig

Configuration for tree-sitter language pack integration.

Controls grammar download behavior and code analysis options.

##### Example (TOML)

```toml
[tree_sitter]
languages = ["python", "rust"]
groups = ["web"]

[tree_sitter.process]
structure = true
comments = true
docstrings = true
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `bool` | `true` | Enable code intelligence processing (default: true). When `false`, tree-sitter analysis is completely skipped even if the config section is present. |
| `cache_dir` | `const char**` | `NULL` | Custom cache directory for downloaded grammars. When `NULL`, uses the default: `~/.cache/tree-sitter-language-pack/v{version}/libs/`. |
| `languages` | `const char***` | `NULL` | Languages to pre-download on init (e.g., `\["python", "rust"\]`). |
| `groups` | `const char***` | `NULL` | Language groups to pre-download (e.g., `\["web", "systems", "scripting"\]`). |
| `process` | `KreuzbergTreeSitterProcessConfig` | — | Processing options for code analysis. |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergTreeSitterConfig kreuzberg_default();
```

**Example:**

```c
KreuzbergTreeSitterConfig *result = kreuzberg_default();
```

**Returns:** `KreuzbergTreeSitterConfig`

---

#### KreuzbergTreeSitterProcessConfig

Processing options for tree-sitter code analysis.

Controls which analysis features are enabled when extracting code files.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `structure` | `bool` | `true` | Extract structural items (functions, classes, structs, etc.). Default: true. |
| `imports` | `bool` | `true` | Extract import statements. Default: true. |
| `exports` | `bool` | `true` | Extract export statements. Default: true. |
| `comments` | `bool` | `false` | Extract comments. Default: false. |
| `docstrings` | `bool` | `false` | Extract docstrings. Default: false. |
| `symbols` | `bool` | `false` | Extract symbol definitions. Default: false. |
| `diagnostics` | `bool` | `false` | Include parse diagnostics. Default: false. |
| `chunk_max_size` | `uintptr_t*` | `NULL` | Maximum chunk size in bytes. `NULL` disables chunking. |
| `content_mode` | `KreuzbergCodeContentMode` | `KREUZBERG_KREUZBERG_CHUNKS` | Content rendering mode for code extraction. |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergTreeSitterProcessConfig kreuzberg_default();
```

**Example:**

```c
KreuzbergTreeSitterProcessConfig *result = kreuzberg_default();
```

**Returns:** `KreuzbergTreeSitterProcessConfig`

---

#### KreuzbergUserChunkConfig

User-provided chunk configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `page_ranges` | `KreuzbergPageRange**` | `NULL` | User-specified page ranges (overrides automatic chunking). |
| `pages_per_chunk` | `uint32_t*` | `NULL` | User-specified pages per chunk (overrides automatic calculation). |
| `force_chunking` | `bool` | — | Force chunking even for small documents. |
| `disable_chunking` | `bool` | — | Disable chunking even for large documents. |

---

#### KreuzbergValidator

Trait for validator plugins.

Validators check extraction results for quality, completeness, or correctness.
Unlike post-processors, validator errors **fail fast** - if a validator returns
an error, the extraction fails immediately.

##### Use Cases

- **Quality Gates**: Ensure extracted content meets minimum quality standards
- **Compliance**: Verify content meets regulatory requirements
- **Content Filtering**: Reject documents containing unwanted content
- **Format Validation**: Verify extracted content structure
- **Security Checks**: Scan for malicious content

##### Error Handling

Validator errors are **fatal** - they cause the extraction to fail and bubble up
to the caller. Use validators for hard requirements that must be met.

For non-fatal checks, use post-processors instead.

##### Thread Safety

Validators must be thread-safe (`Send + Sync`).

##### Methods

###### kreuzberg_validate()

Validate an extraction result.

Check the extraction result and return `Ok(())` if valid, or an error
if validation fails.

**Returns:**

- `Ok(())` if validation passes
- `Err(...)` if validation fails (extraction will fail)

**Errors:**

- `KreuzbergError.Validation` - Validation failed
- Any other error type appropriate for the failure

##### Example - Content Length Validation

##### Example - Quality Score Validation

##### Example - Security Validation

**Signature:**

```c
void kreuzberg_validate(KreuzbergExtractionResult result, KreuzbergExtractionConfig config);
```

**Example:**

```c
kreuzberg_validate(instance, NULL, NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `result` | `KreuzbergExtractionResult` | Yes | The extraction result to validate |
| `config` | `KreuzbergExtractionConfig` | Yes | Extraction configuration |

**Returns:** No return value.

**Errors:** Returns `NULL` on error.

###### kreuzberg_should_validate()

Optional: Check if this validator should run for a given result.

Allows conditional validation based on MIME type, metadata, or content.
Defaults to `true` (always run).

**Returns:**

`true` if the validator should run, `false` to skip.

**Signature:**

```c
bool kreuzberg_should_validate(KreuzbergExtractionResult result, KreuzbergExtractionConfig config);
```

**Example:**

```c
bool result = kreuzberg_should_validate(instance, NULL, NULL);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `result` | `KreuzbergExtractionResult` | Yes | The extraction result |
| `config` | `KreuzbergExtractionConfig` | Yes | The extraction config |

**Returns:** `bool`

###### kreuzberg_priority()

Optional: Get the validation priority.

Higher priority validators run first. Useful for ordering validation checks
(e.g., run cheap validations before expensive ones).

Default priority is 50.

**Returns:**

Priority value (higher = runs earlier).

**Signature:**

```c
int32_t kreuzberg_priority();
```

**Example:**

```c
int32_t result = kreuzberg_priority(instance);
```

**Returns:** `int32_t`

---

#### KreuzbergXlsxAppProperties

Application properties from docProps/app.xml for XLSX

Contains Excel-specific document metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `application` | `const char**` | `NULL` | Application name (e.g., "Microsoft Excel") |
| `app_version` | `const char**` | `NULL` | Application version |
| `doc_security` | `int32_t*` | `NULL` | Document security level |
| `scale_crop` | `bool*` | `NULL` | Scale crop flag |
| `links_up_to_date` | `bool*` | `NULL` | Links up to date flag |
| `shared_doc` | `bool*` | `NULL` | Shared document flag |
| `hyperlinks_changed` | `bool*` | `NULL` | Hyperlinks changed flag |
| `company` | `const char**` | `NULL` | Company name |
| `worksheet_names` | `const char**` | `NULL` | Worksheet names |

---

#### KreuzbergXmlExtractionResult

XML extraction result.

Contains extracted text content from XML files along with
structural statistics about the XML document.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `const char*` | — | Extracted text content (XML structure filtered out) |
| `element_count` | `uintptr_t` | — | Total number of XML elements processed |
| `unique_elements` | `const char**` | — | List of unique element names found (sorted) |

---

#### KreuzbergXmlMetadata

XML metadata extracted during XML parsing.

Provides statistics about XML document structure.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `element_count` | `uint32_t` | — | Total number of XML elements processed |
| `unique_elements` | `const char**` | `NULL` | List of unique element tag names (sorted) |

---

#### KreuzbergYakeParams

YAKE-specific parameters.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `window_size` | `uintptr_t` | `2` | Window size for co-occurrence analysis (default: 2). Controls the context window for computing co-occurrence statistics. |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergYakeParams kreuzberg_default();
```

**Example:**

```c
KreuzbergYakeParams *result = kreuzberg_default();
```

**Returns:** `KreuzbergYakeParams`

---

#### KreuzbergYearRange

Year range for bibliographic metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `min` | `uint32_t*` | `NULL` | Earliest (minimum) year in the range. |
| `max` | `uint32_t*` | `NULL` | Latest (maximum) year in the range. |
| `years` | `uint32_t*` | `/* serde(default) */` | All individual years present in the collection. |

---

### Enums

#### KreuzbergExecutionProviderType

ONNX Runtime execution provider type.

Determines which hardware backend is used for model inference.
`Auto` (default) selects the best available provider per platform.

| Value | Description |
|-------|-------------|
| `KREUZBERG_AUTO` | Auto-select: CoreML on macOS, CUDA on Linux, CPU elsewhere. |
| `KREUZBERG_CPU` | CPU execution provider (always available). |
| `KREUZBERG_CORE_ML` | Apple CoreML (macOS/iOS Neural Engine + GPU). |
| `KREUZBERG_CUDA` | NVIDIA CUDA GPU acceleration. |
| `KREUZBERG_TENSOR_RT` | NVIDIA TensorRT (optimized CUDA inference). |

---

#### KreuzbergImageOutputFormat

Target format for re-encoding extracted images.

Controls whether and how extracted images are normalised to a uniform
container format before being returned in `ExtractionResult.images`.
The default (`Native`) preserves the format produced by each extractor
without any additional encode pass.

Callers that need uniform output — e.g. cloud pipelines that always store
WebP thumbnails — set this once on `ImageExtractionConfig.output_format`
rather than re-encoding downstream.

### Serde shape

Uses a tagged enum: `{"type": "native"}`, `{"type": "png"}`,
`{"type": "jpeg", "quality": 90}`, etc.

| Value | Description |
|-------|-------------|
| `KREUZBERG_NATIVE` | Preserve whatever format the extractor produced (default). No re-encode pass is performed. `ExtractedImage.format` reflects the source format: JPEG for embedded PDF images, PNG for rasterised content, or the native container format from office documents. |
| `KREUZBERG_PNG` | Re-encode all extracted images as PNG (lossless). |
| `KREUZBERG_JPEG` | Re-encode all extracted images as JPEG at the given quality level. `quality` must be in `1..=100`. Values outside this range are clamped and a warning is emitted. Higher values produce larger files with less artefacting; 85 is a reasonable default. — Fields: `quality`: `uint8_t` |
| `KREUZBERG_WEBP` | Re-encode all extracted images as WebP at the given quality level. `quality` must be in `1..=100`. Values outside this range are clamped and a warning is emitted. 80 is a reasonable default. — Fields: `quality`: `uint8_t` |
| `KREUZBERG_HEIF` | Re-encode all extracted images as HEIF/HEIC at the given quality level. Requires the `heic` feature. `quality` must be in `1..=100`. Values outside this range are clamped and a warning is emitted. 80 is a reasonable default. — Fields: `quality`: `uint8_t` |
| `KREUZBERG_SVG` | Output pure-vector SVG. Lossless. Raster sources are not re-encoded (a warning is emitted and the image bytes are left untouched). When the source is already SVG, the bytes are passed through the `usvg` sanitizer (strips external hrefs, JS event handlers, and `foreignObject` elements) when `SvgOptions.sanitize` is `true`. Requires the `svg` feature. |

---

#### KreuzbergOutputFormat

Output format for extraction results.

Controls the format of the `content` field in `ExtractionResult`.
When set to `Markdown`, `Djot`, or `Html`, the output uses that format.
`Plain` returns the raw extracted text.
`Structured` returns JSON with full OCR element data including bounding
boxes and confidence scores.

| Value | Description |
|-------|-------------|
| `KREUZBERG_PLAIN` | Plain text content only (default) |
| `KREUZBERG_MARKDOWN` | Markdown format |
| `KREUZBERG_DJOT` | Djot markup format |
| `KREUZBERG_HTML` | HTML format |
| `KREUZBERG_JSON` | JSON tree format with heading-driven sections. |
| `KREUZBERG_STRUCTURED` | Structured JSON format with full OCR element metadata. |
| `KREUZBERG_CUSTOM` | Custom renderer registered via the RendererRegistry. The string is the renderer name (e.g., "docx", "latex"). — Fields: `0`: `const char*` |

---

#### KreuzbergHtmlTheme

Built-in HTML theme selection.

| Value | Description |
|-------|-------------|
| `KREUZBERG_DEFAULT` | Sensible defaults: system font stack, neutral colours, readable line measure. CSS custom properties (`--kb-*`) are all defined so user CSS can override individual values. |
| `KREUZBERG_GIT_HUB` | GitHub Markdown-inspired palette and spacing. |
| `KREUZBERG_DARK` | Dark background, light text. |
| `KREUZBERG_LIGHT` | Minimal light theme with generous whitespace. |
| `KREUZBERG_UNSTYLED` | No built-in stylesheet emitted. CSS custom properties are still defined on `:root` so user stylesheets can reference `var(--kb-*)` tokens. |

---

#### KreuzbergTableModel

Which table structure recognition model to use.

Controls the model used for table cell detection within layout-detected
table regions. Wire format is snake_case in all serializers (JSON, TOML,
YAML).

| Value | Description |
|-------|-------------|
| `KREUZBERG_TATR` | TATR (Table Transformer) -- default, 30MB, DETR-based row/column detection. |
| `KREUZBERG_SLANET_WIRED` | SLANeXT wired variant -- 365MB, optimized for bordered tables. |
| `KREUZBERG_SLANET_WIRELESS` | SLANeXT wireless variant -- 365MB, optimized for borderless tables. |
| `KREUZBERG_SLANET_PLUS` | SLANet-plus -- 7.78MB, lightweight general-purpose. |
| `KREUZBERG_SLANET_AUTO` | Classifier-routed SLANeXT: auto-select wired/wireless per table. Uses PP-LCNet classifier (6.78MB) + both SLANeXT variants (730MB total). |
| `KREUZBERG_DISABLED` | Disable table structure model inference entirely; use heuristic path only. |

---

#### KreuzbergCallMode

How a structured-extraction preset is dispatched to the model.

This is the preset-facing call mode (the `preferred_call_mode` field of a
`Preset`). The richer runtime decision enum used by the
structured pipeline — which adds `Skip` and `TextOnlyWithVisionFallback` —
lives in `crate.heuristics.structured.StructuredCallMode`; this 3-variant
type is the stable, serializable surface presets and bindings depend on.

| Value | Description |
|-------|-------------|
| `KREUZBERG_TEXT_ONLY` | Use the extracted text only. |
| `KREUZBERG_VISION_ONLY` | Use rasterized page images only. |
| `KREUZBERG_TEXT_PLUS_VISION` | Provide both extracted text and page images to the model. |

---

#### KreuzbergMergeMode

How partial results from multiple model calls (e.g. per page batch) are combined.

Canonical home for the merge strategy referenced by presets and by the
structured pipeline's post-processing. There is intentionally only one merge
type across the crate — do not introduce a second.

| Value | Description |
|-------|-------------|
| `KREUZBERG_OBJECT_MERGE` | Deep-merge JSON objects field by field (later calls fill missing fields). |
| `KREUZBERG_ARRAY_CONCAT` | Concatenate top-level arrays across calls. |
| `KREUZBERG_OBJECT_FIRST` | Keep the first non-empty result; ignore subsequent calls. |

---

#### KreuzbergNerBackendKind

NER backend selector.

| Value | Description |
|-------|-------------|
| `KREUZBERG_ONNX` | gline-rs ONNX inference. Requires `ner-onnx` feature. Models download lazily from HuggingFace via `model_download.hf_download`. |
| `KREUZBERG_LLM` | liter-llm zero-shot NER via structured-output prompts. Requires `ner-llm` feature. Useful when domain-specific categories outstrip the ONNX taxonomy. |

---

#### KreuzbergVlmFallbackPolicy

Policy controlling when VLM (Vision Language Model) OCR is used as a fallback.

This knob is syntactic sugar over the explicit `OcrPipelineConfig` stage
ordering. When `vlm_fallback` is set and `pipeline` is `NULL`, an equivalent
pipeline is synthesised at extraction time:

- `VlmFallbackPolicy.Disabled` — no synthesis; single-backend mode (default).
- `VlmFallbackPolicy.OnLowQuality` — tries the classical backend first; if the
  result scores below `quality_threshold`, tries VLM.

- `VlmFallbackPolicy.Always` — skips the classical backend and sends every page
  to the VLM.

When `OcrConfig.pipeline` is explicitly set, `vlm_fallback` is ignored — the
explicit pipeline takes precedence.

**Errors:**

Both `OnLowQuality` and `Always` require `OcrConfig.vlm_config` to be `Some`.
Constructing an `OcrConfig` with one of these policies but no `vlm_config` is
detected by `OcrConfig.validate` and will surface as a
`Validation` error at extraction time, not a panic.

| Value | Description |
|-------|-------------|
| `KREUZBERG_DISABLED` | No VLM fallback (default). Behaves identically to the pre-policy single-backend mode. |
| `KREUZBERG_ON_LOW_QUALITY` | Try the classical OCR backend first. If the quality score is below `quality_threshold`, send the page to the VLM. `quality_threshold` is in the `\[0.0, 1.0\]` range produced by `calculate_quality_score`. A value of `0.5` is a reasonable starting point; calibrate with the Stage 0 benchmark harness. — Fields: `quality_threshold`: `double` |
| `KREUZBERG_ALWAYS` | Skip the classical OCR backend entirely. Every page is sent to the VLM. |

---

#### KreuzbergChunkerType

Type of text chunker to use.

### Variants

- `Text` - Generic text splitter, splits on whitespace and punctuation
- `Markdown` - Markdown-aware splitter, preserves formatting and structure
- `Yaml` - YAML-aware splitter, creates one chunk per top-level key
- `Semantic` - Topic-aware chunker. With an `EmbeddingConfig`, splits at
  embedding-based topic shifts tuned by `topic_threshold` (default 0.75,
  lower = more splits). Without an embedding, falls back to a
  structural-boundary heuristic (ALL-CAPS headers, numbered sections,
  blank-line paragraphs) and merges groups into chunks capped at
  `max_characters` (default 1000). `topic_threshold` has no effect in the
  fallback path. For best results, pair with an embedding model.

| Value | Description |
|-------|-------------|
| `KREUZBERG_TEXT` | Generic whitespace- and punctuation-aware text splitter (default). |
| `KREUZBERG_MARKDOWN` | Markdown-aware splitter that preserves heading and code-block boundaries. |
| `KREUZBERG_YAML` | YAML-aware splitter that creates one chunk per top-level key. |
| `KREUZBERG_SEMANTIC` | Topic-aware chunker that splits at embedding-based topic shifts. |

---

#### KreuzbergChunkSizing

How chunk size is measured.

Defaults to `Characters` (Unicode character count). When using token-based sizing,
chunks are sized by token count according to the specified tokenizer.

Token-based sizing uses HuggingFace tokenizers loaded at runtime. Any tokenizer
available on HuggingFace Hub can be used, including OpenAI-compatible tokenizers
(e.g., `Xenova/gpt-4o`, `Xenova/cl100k_base`).

| Value | Description |
|-------|-------------|
| `KREUZBERG_CHARACTERS` | Size measured in Unicode characters (default). |
| `KREUZBERG_TOKENIZER` | Size measured in tokens from a HuggingFace tokenizer. — Fields: `model`: `const char*`, `cache_dir`: `const char*` |

---

#### KreuzbergEmbeddingModelType

Embedding model types supported by Kreuzberg.

| Value | Description |
|-------|-------------|
| `KREUZBERG_PRESET` | Use a preset model configuration (recommended) — Fields: `name`: `const char*` |
| `KREUZBERG_CUSTOM` | Use a custom ONNX model from HuggingFace — Fields: `model_id`: `const char*`, `dimensions`: `uintptr_t` |
| `KREUZBERG_LLM` | Provider-hosted embedding model via liter-llm. Uses the model specified in the nested `LlmConfig` (e.g., `"openai/text-embedding-3-small"`). — Fields: `llm`: `KreuzbergLlmConfig` |
| `KREUZBERG_PLUGIN` | In-process embedding backend registered via the plugin system. The caller registers an `EmbeddingBackend` once (e.g. a wrapper around an already-loaded `llama-cpp-python`, `sentence-transformers`, or tuned ONNX model), then references it by name in config. Kreuzberg calls back into the registered backend during chunking and standalone embed requests — no HuggingFace download, no ONNX Runtime requirement, no HTTP sidecar. When this variant is selected, only the following `EmbeddingConfig` fields apply: `normalize` (post-call L2 normalization) and `max_embed_duration_secs` (dispatcher timeout). Model-loading fields (`batch_size`, `cache_dir`, `show_download_progress`, `acceleration`) are ignored — the host owns the model lifecycle. Semantic chunking falls back to `ChunkingConfig.max_characters` when this variant is used, since there is no preset to look a chunk-size ceiling up against — size your context window via `max_characters` directly. See `register_embedding_backend`. — Fields: `name`: `const char*` |

---

#### KreuzbergRerankerModelType

Reranker model types supported by Kreuzberg.

Since v5.0.

| Value | Description |
|-------|-------------|
| `KREUZBERG_PRESET` | Use a preset cross-encoder model (recommended). — Fields: `name`: `const char*` |
| `KREUZBERG_CUSTOM` | Use a custom ONNX cross-encoder from HuggingFace. — Fields: `model_id`: `const char*`, `model_file`: `const char*`, `additional_files`: `const char**`, `max_length`: `int64_t` |
| `KREUZBERG_LLM` | Provider-hosted reranker via liter-llm (e.g. Cohere, Jina, Voyage). The model in the nested `LlmConfig` must be a rerank-capable model ID (e.g. `"cohere/rerank-english-v3.0"`). — Fields: `llm`: `KreuzbergLlmConfig` |
| `KREUZBERG_PLUGIN` | In-process reranker registered via the plugin system. The caller registers a `RerankerBackend` once (e.g. a wrapper around a `sentence-transformers` cross-encoder or a provider client), then references it by name in config. Kreuzberg calls back into the registered backend — no HuggingFace download, no ONNX Runtime requirement. When this variant is selected, only `max_rerank_duration_secs` applies. Model-loading fields (`batch_size`, `cache_dir`, `show_download_progress`, `acceleration`) are ignored — the host owns the model lifecycle. See `register_reranker_backend`. — Fields: `name`: `const char*` |

---

#### KreuzbergWhisperModel

Supported Whisper model sizes.

These map to published ONNX exports on Hugging Face (onnx-community or
similar orgs). The actual filenames and repos are resolved inside the
transcription engine.

| Value | Description |
|-------|-------------|
| `KREUZBERG_TINY` | Smallest, fastest, lowest quality. Good default for development and CI. |
| `KREUZBERG_BASE` | Reasonable quality/speed tradeoff. |
| `KREUZBERG_SMALL` | Better accuracy with higher memory and cache use. |
| `KREUZBERG_MEDIUM` | High quality; slower and more memory-intensive. |
| `KREUZBERG_LARGE_V3` | Best quality (large-v3). Use only when latency and memory use are acceptable. |

---

#### KreuzbergCodeContentMode

Content rendering mode for code extraction.

Controls how extracted code content is represented in the `content` field
of `ExtractionResult`.

| Value | Description |
|-------|-------------|
| `KREUZBERG_CHUNKS` | Use TSLP semantic chunks as content (default). |
| `KREUZBERG_RAW` | Use raw source code as content. |
| `KREUZBERG_STRUCTURE` | Emit function/class headings + docstrings (no code bodies). |

---

#### KreuzbergListType

Type of list detection.

| Value | Description |
|-------|-------------|
| `KREUZBERG_BULLET` | Bullet points (-, *, •, etc.) |
| `KREUZBERG_NUMBERED` | Numbered lists (1., 2., etc.) |
| `KREUZBERG_LETTERED` | Lettered lists (a., b., A., B., etc.) |
| `KREUZBERG_INDENTED` | Indented items |

---

#### KreuzbergOcrBackendType

OCR backend types.

| Value | Description |
|-------|-------------|
| `KREUZBERG_TESSERACT` | Tesseract OCR (native Rust binding) |
| `KREUZBERG_EASY_OCR` | EasyOCR (Python-based, via FFI) |
| `KREUZBERG_PADDLE_OCR` | PaddleOCR (Python-based, via FFI) |
| `KREUZBERG_CANDLE` | Candle-based VLM OCR (TrOCR, PaddleOCR-VL). |
| `KREUZBERG_CUSTOM` | Custom/third-party OCR backend |

---

#### KreuzbergProcessingStage

Processing stages for post-processors.

Post-processors are executed in stage order (Early → Middle → Late).
Use stages to control the order of post-processing operations.

| Value | Description |
|-------|-------------|
| `KREUZBERG_EARLY` | Early stage - foundational processing. Use for: - Language detection - Character encoding normalization - Entity extraction (NER) - Text quality scoring |
| `KREUZBERG_MIDDLE` | Middle stage - content transformation. Use for: - Keyword extraction - Token reduction - Text summarization - Semantic analysis |
| `KREUZBERG_LATE` | Late stage - final enrichment. Use for: - Custom user hooks - Analytics/logging - Final validation - Output formatting |

---

#### KreuzbergReductionLevel

Intensity level for the token-reduction pipeline.

| Value | Description |
|-------|-------------|
| `KREUZBERG_OFF` | No reduction applied; text is returned as-is. |
| `KREUZBERG_LIGHT` | Remove only the most common stopwords. |
| `KREUZBERG_MODERATE` | Balanced stopword removal and redundancy filtering. |
| `KREUZBERG_AGGRESSIVE` | Aggressive filtering; may remove less common content words. |
| `KREUZBERG_MAXIMUM` | Maximum compression; prioritizes brevity over completeness. |

---

#### KreuzbergPdfAnnotationType

Type of PDF annotation.

| Value | Description |
|-------|-------------|
| `KREUZBERG_TEXT` | Sticky note / text annotation |
| `KREUZBERG_HIGHLIGHT` | Highlighted text region |
| `KREUZBERG_LINK` | Hyperlink annotation |
| `KREUZBERG_STAMP` | Rubber stamp annotation |
| `KREUZBERG_UNDERLINE` | Underline text markup |
| `KREUZBERG_STRIKE_OUT` | Strikeout text markup |
| `KREUZBERG_OTHER` | Any other annotation type |

---

#### KreuzbergBlockType

Types of block-level elements in Djot.

| Value | Description |
|-------|-------------|
| `KREUZBERG_PARAGRAPH` | Standard prose paragraph. |
| `KREUZBERG_HEADING` | Section heading (level stored in `FormattedBlock.level`). |
| `KREUZBERG_BLOCKQUOTE` | Block quotation container. |
| `KREUZBERG_CODE_BLOCK` | Fenced or indented code block. |
| `KREUZBERG_LIST_ITEM` | Individual item within a list. |
| `KREUZBERG_ORDERED_LIST` | Numbered (ordered) list container. |
| `KREUZBERG_BULLET_LIST` | Unnumbered (bullet) list container. |
| `KREUZBERG_TASK_LIST` | Task / checkbox list container. |
| `KREUZBERG_DEFINITION_LIST` | Definition list container. |
| `KREUZBERG_DEFINITION_TERM` | Term part of a definition list entry. |
| `KREUZBERG_DEFINITION_DESCRIPTION` | Description / definition part of a definition list entry. |
| `KREUZBERG_DIV` | Generic `div` container with optional attributes. |
| `KREUZBERG_SECTION` | Logical section container, often associated with a heading. |
| `KREUZBERG_THEMATIC_BREAK` | Horizontal rule / thematic break. |
| `KREUZBERG_RAW_BLOCK` | Raw content block in a specified format (e.g. HTML, LaTeX). |
| `KREUZBERG_MATH_DISPLAY` | Display-mode mathematical expression. |

---

#### KreuzbergInlineType

Types of inline elements in Djot.

| Value | Description |
|-------|-------------|
| `KREUZBERG_TEXT` | Plain text run. |
| `KREUZBERG_STRONG` | Bold / strong emphasis. |
| `KREUZBERG_EMPHASIS` | Italic / regular emphasis. |
| `KREUZBERG_HIGHLIGHT` | Highlighted text (marker pen). |
| `KREUZBERG_SUBSCRIPT` | Subscript text. |
| `KREUZBERG_SUPERSCRIPT` | Superscript text. |
| `KREUZBERG_INSERT` | Inserted text (tracked change). |
| `KREUZBERG_DELETE` | Deleted text (tracked change). |
| `KREUZBERG_CODE` | Inline code span. |
| `KREUZBERG_LINK` | Hyperlink with URL. |
| `KREUZBERG_IMAGE` | Inline image reference. |
| `KREUZBERG_SPAN` | Generic inline span with optional attributes. |
| `KREUZBERG_MATH` | Inline mathematical expression. |
| `KREUZBERG_RAW_INLINE` | Raw inline content in a specified format. |
| `KREUZBERG_FOOTNOTE_REF` | Footnote reference marker. |
| `KREUZBERG_SYMBOL` | Named symbol or emoji shortcode. |

---

#### KreuzbergRelationshipKind

Semantic kind of a relationship between document elements.

| Value | Description |
|-------|-------------|
| `KREUZBERG_FOOTNOTE_REFERENCE` | Footnote marker -> footnote definition. |
| `KREUZBERG_CITATION_REFERENCE` | Citation marker -> bibliography entry. |
| `KREUZBERG_INTERNAL_LINK` | Internal anchor link (`#id`) -> target heading/element. |
| `KREUZBERG_CAPTION` | Caption paragraph -> figure/table it describes. |
| `KREUZBERG_LABEL` | Label -> labeled element (HTML `<label for>`, LaTeX `\label{}`). |
| `KREUZBERG_TOC_ENTRY` | TOC entry -> target section. |
| `KREUZBERG_CROSS_REFERENCE` | Cross-reference (LaTeX `\ref{}`, DOCX cross-reference field). |

---

#### KreuzbergContentLayer

Content layer classification for document nodes.

Replaces separate body/furniture arrays with per-node granularity.

| Value | Description |
|-------|-------------|
| `KREUZBERG_BODY` | Main document body content. |
| `KREUZBERG_HEADER` | Page/section header (running header). |
| `KREUZBERG_FOOTER` | Page/section footer (running footer). |
| `KREUZBERG_FOOTNOTE` | Footnote content. |

---

#### KreuzbergNodeContent

Tagged enum for node content. Each variant carries only type-specific data.

Uses `#[serde(tag = "node_type")]` to avoid "type" keyword collision in
Go/Java/TypeScript bindings.

| Value | Description |
|-------|-------------|
| `KREUZBERG_TITLE` | Document title. — Fields: `text`: `const char*` |
| `KREUZBERG_HEADING` | Section heading with level (1-6). — Fields: `level`: `uint8_t`, `text`: `const char*` |
| `KREUZBERG_PARAGRAPH` | Body text paragraph. — Fields: `text`: `const char*` |
| `KREUZBERG_LIST` | List container — children are `ListItem` nodes. — Fields: `ordered`: `bool` |
| `KREUZBERG_LIST_ITEM` | Individual list item. — Fields: `text`: `const char*` |
| `KREUZBERG_TABLE` | Table with structured cell grid. — Fields: `grid`: `KreuzbergTableGrid` |
| `KREUZBERG_IMAGE` | Image reference. — Fields: `description`: `const char*`, `image_index`: `uint32_t`, `src`: `const char*` |
| `KREUZBERG_CODE` | Code block. — Fields: `text`: `const char*`, `language`: `const char*` |
| `KREUZBERG_QUOTE` | Block quote — container, children carry the quoted content. |
| `KREUZBERG_FORMULA` | Mathematical formula / equation. — Fields: `text`: `const char*` |
| `KREUZBERG_FOOTNOTE` | Footnote reference content. — Fields: `text`: `const char*` |
| `KREUZBERG_GROUP` | Logical grouping container (section, key-value area). `heading_level` + `heading_text` capture the section heading directly rather than relying on a first-child positional convention. — Fields: `label`: `const char*`, `heading_level`: `uint8_t`, `heading_text`: `const char*` |
| `KREUZBERG_PAGE_BREAK` | Page break marker. |
| `KREUZBERG_SLIDE` | Presentation slide container — children are the slide's content nodes. — Fields: `number`: `uint32_t`, `title`: `const char*` |
| `KREUZBERG_DEFINITION_LIST` | Definition list container — children are `DefinitionItem` nodes. |
| `KREUZBERG_DEFINITION_ITEM` | Individual definition list entry with term and definition. — Fields: `term`: `const char*`, `definition`: `const char*` |
| `KREUZBERG_CITATION` | Citation or bibliographic reference. — Fields: `key`: `const char*`, `text`: `const char*` |
| `KREUZBERG_ADMONITION` | Admonition / callout container (note, warning, tip, etc.). Children carry the admonition body content. — Fields: `kind`: `const char*`, `title`: `const char*` |
| `KREUZBERG_RAW_BLOCK` | Raw block preserved verbatim from the source format. Used for content that cannot be mapped to a semantic node type (e.g. JSX in MDX, raw LaTeX in markdown, embedded HTML). — Fields: `format`: `const char*`, `content`: `const char*` |
| `KREUZBERG_METADATA_BLOCK` | Structured metadata block (email headers, YAML frontmatter, etc.). |

---

#### KreuzbergAnnotationKind

Types of inline text annotations.

| Value | Description |
|-------|-------------|
| `KREUZBERG_BOLD` | Bold (strong) text formatting. |
| `KREUZBERG_ITALIC` | Italic (emphasis) text formatting. |
| `KREUZBERG_UNDERLINE` | Underlined text. |
| `KREUZBERG_STRIKETHROUGH` | Strikethrough text. |
| `KREUZBERG_CODE` | Inline code span. |
| `KREUZBERG_SUBSCRIPT` | Subscript text. |
| `KREUZBERG_SUPERSCRIPT` | Superscript text. |
| `KREUZBERG_LINK` | Hyperlink annotation. — Fields: `url`: `const char*`, `title`: `const char*` |
| `KREUZBERG_HIGHLIGHT` | Highlighted text (PDF highlights, HTML `<mark>`). |
| `KREUZBERG_COLOR` | Text color (CSS-compatible value, e.g. "#ff0000", "red"). — Fields: `value`: `const char*` |
| `KREUZBERG_FONT_SIZE` | Font size with units (e.g. "12pt", "1.2em", "16px"). — Fields: `value`: `const char*` |
| `KREUZBERG_CUSTOM` | Extensible annotation for format-specific styling. — Fields: `name`: `const char*`, `value`: `const char*` |

---

#### KreuzbergEntityCategory

Standard entity categories produced by built-in NER backends.

The `Custom(String)` variant lets caller-supplied categories (e.g. LLM
schemas) flow through without losing fidelity to the consumer.

| Value | Description |
|-------|-------------|
| `KREUZBERG_PERSON` | A person's name. |
| `KREUZBERG_ORGANIZATION` | A company, institution, or organisation name. |
| `KREUZBERG_LOCATION` | A geographic location (city, country, address). |
| `KREUZBERG_DATE` | A calendar date. |
| `KREUZBERG_TIME` | A time of day or duration. |
| `KREUZBERG_MONEY` | A monetary amount with optional currency. |
| `KREUZBERG_PERCENT` | A percentage value. |
| `KREUZBERG_EMAIL` | An email address. |
| `KREUZBERG_PHONE` | A phone number. |
| `KREUZBERG_URL` | A URL or URI. |
| `KREUZBERG_CUSTOM` | A caller-supplied custom category label. — Fields: `0`: `const char*` |

---

#### KreuzbergExtractionMethod

How the extracted text was produced.

| Value | Description |
|-------|-------------|
| `KREUZBERG_NATIVE` | Text extracted directly from the document's native format (no OCR). |
| `KREUZBERG_OCR` | All text was obtained via OCR (e.g. scanned image-only PDF). |
| `KREUZBERG_MIXED` | Text came from a combination of native extraction and OCR. |

---

#### KreuzbergChunkType

Semantic structural classification of a text chunk.

Assigned by the heuristic classifier in `chunking.classifier`.
Defaults to `Unknown` when no rule matches.
Designed to be extended in future versions without breaking changes.

| Value | Description |
|-------|-------------|
| `KREUZBERG_HEADING` | Section heading or document title. |
| `KREUZBERG_PARTY_LIST` | Party list: names, addresses, and signatories. |
| `KREUZBERG_DEFINITIONS` | Definition clause ("X means…", "X shall mean…"). |
| `KREUZBERG_OPERATIVE_CLAUSE` | Operative clause containing legal/contractual action verbs. |
| `KREUZBERG_SIGNATURE_BLOCK` | Signature block with signatures, names, and dates. |
| `KREUZBERG_SCHEDULE` | Schedule, annex, appendix, or exhibit section. |
| `KREUZBERG_TABLE_LIKE` | Table-like content with aligned columns or repeated patterns. |
| `KREUZBERG_FORMULA` | Mathematical formula or equation. |
| `KREUZBERG_CODE_BLOCK` | Code block or preformatted content. |
| `KREUZBERG_IMAGE` | Embedded or referenced image content. |
| `KREUZBERG_ORG_CHART` | Organizational chart or hierarchy diagram. |
| `KREUZBERG_DIAGRAM` | Diagram, figure, or visual illustration. |
| `KREUZBERG_UNKNOWN` | Unclassified or mixed content. |

---

#### KreuzbergImageKind

Heuristic classification of what an image likely depicts.

| Value | Description |
|-------|-------------|
| `KREUZBERG_PHOTOGRAPH` | Photographic image (natural scene, photograph) |
| `KREUZBERG_DIAGRAM` | Technical or schematic diagram |
| `KREUZBERG_CHART` | Chart, graph, or plot |
| `KREUZBERG_DRAWING` | Freehand or technical drawing |
| `KREUZBERG_TEXT_BLOCK` | Text-heavy image (scanned text, document) |
| `KREUZBERG_DECORATION` | Decorative element or border |
| `KREUZBERG_LOGO` | Logo or brand mark |
| `KREUZBERG_ICON` | Small icon |
| `KREUZBERG_TILE_FRAGMENT` | Fragment of a larger tiled image (tile of a technical drawing) |
| `KREUZBERG_MASK` | Mask or transparency map |
| `KREUZBERG_PAGE_RASTER` | Full-page render produced during OCR preprocessing; used as a citation thumbnail. |
| `KREUZBERG_UNKNOWN` | Could not classify with reasonable confidence |

---

#### KreuzbergResultFormat

Result-shape selection for extraction results.

Distinct from `OutputFormat` (which controls rendering — Plain, Markdown,
HTML, etc.). `ResultFormat` controls the *shape* of the result: a unified content
blob vs. an element-based decomposition.

| Value | Description |
|-------|-------------|
| `KREUZBERG_UNIFIED` | Unified format with all content in `content` field |
| `KREUZBERG_ELEMENT_BASED` | Element-based format with semantic element extraction |

---

#### KreuzbergElementType

Semantic element type classification.

Categorizes text content into semantic units for downstream processing.
Supports the element types commonly found in Unstructured documents.

| Value | Description |
|-------|-------------|
| `KREUZBERG_TITLE` | Document title |
| `KREUZBERG_NARRATIVE_TEXT` | Main narrative text body |
| `KREUZBERG_HEADING` | Section heading |
| `KREUZBERG_LIST_ITEM` | List item (bullet, numbered, etc.) |
| `KREUZBERG_TABLE` | Table element |
| `KREUZBERG_IMAGE` | Image element |
| `KREUZBERG_PAGE_BREAK` | Page break marker |
| `KREUZBERG_CODE_BLOCK` | Code block |
| `KREUZBERG_BLOCK_QUOTE` | Block quote |
| `KREUZBERG_FOOTER` | Footer text |
| `KREUZBERG_HEADER` | Header text |

---

#### KreuzbergFormFieldType

Kind of a PDF form field.

Mirrors `pdf_oxide`'s widget field taxonomy without leaking the upstream
type across the binding surface.

| Value | Description |
|-------|-------------|
| `KREUZBERG_TEXT` | Single- or multi-line text input. |
| `KREUZBERG_CHECKBOX` | Checkbox (on/off toggle). |
| `KREUZBERG_RADIO` | Radio-button group member. |
| `KREUZBERG_CHOICE` | Choice field (dropdown or list box). |
| `KREUZBERG_SIGNATURE` | Digital-signature field. |
| `KREUZBERG_BUTTON` | Push button. |
| `KREUZBERG_UNKNOWN` | Field type that could not be classified. |

---

#### KreuzbergFormatMetadata

Format-specific metadata (discriminated union).

Only one format type can exist per extraction result. This provides
type-safe, clean metadata without nested optionals.

| Value | Description |
|-------|-------------|
| `KREUZBERG_PDF` | Metadata extracted from a PDF document. — Fields: `0`: `KreuzbergPdfMetadata` |
| `KREUZBERG_DOCX` | Metadata extracted from a DOCX Word document. — Fields: `0`: `KreuzbergDocxMetadata` |
| `KREUZBERG_EXCEL` | Metadata extracted from an Excel spreadsheet. — Fields: `0`: `KreuzbergExcelMetadata` |
| `KREUZBERG_EMAIL` | Metadata extracted from an email message (EML/MSG). — Fields: `0`: `KreuzbergEmailMetadata` |
| `KREUZBERG_PPTX` | Metadata extracted from a PowerPoint presentation. — Fields: `0`: `KreuzbergPptxMetadata` |
| `KREUZBERG_ARCHIVE` | Metadata extracted from an archive (ZIP, TAR, 7Z, etc.). — Fields: `0`: `KreuzbergArchiveMetadata` |
| `KREUZBERG_IMAGE` | Metadata extracted from a raster or vector image. — Fields: `0`: `KreuzbergImageMetadata` |
| `KREUZBERG_XML` | Metadata extracted from an XML document. — Fields: `0`: `KreuzbergXmlMetadata` |
| `KREUZBERG_TEXT` | Metadata extracted from a plain-text file. — Fields: `0`: `KreuzbergTextMetadata` |
| `KREUZBERG_HTML` | Metadata extracted from an HTML document. — Fields: `0`: `KreuzbergHtmlMetadata` |
| `KREUZBERG_OCR` | Metadata produced by an OCR pipeline. — Fields: `0`: `KreuzbergOcrMetadata` |
| `KREUZBERG_CSV` | Metadata extracted from a CSV or TSV file. — Fields: `0`: `KreuzbergCsvMetadata` |
| `KREUZBERG_BIBTEX` | Metadata extracted from a BibTeX bibliography file. — Fields: `0`: `KreuzbergBibtexMetadata` |
| `KREUZBERG_CITATION` | Metadata extracted from a citation file (RIS, PubMed, EndNote). — Fields: `0`: `KreuzbergCitationMetadata` |
| `KREUZBERG_FICTION_BOOK` | Metadata extracted from a FictionBook (FB2) e-book. — Fields: `0`: `KreuzbergFictionBookMetadata` |
| `KREUZBERG_DBF` | Metadata extracted from a dBASE (DBF) database file. — Fields: `0`: `KreuzbergDbfMetadata` |
| `KREUZBERG_JATS` | Metadata extracted from a JATS (Journal Article Tag Suite) XML file. — Fields: `0`: `KreuzbergJatsMetadata` |
| `KREUZBERG_EPUB` | Metadata extracted from an EPUB e-book. — Fields: `0`: `KreuzbergEpubMetadata` |
| `KREUZBERG_PST` | Metadata extracted from an Outlook PST archive. — Fields: `0`: `KreuzbergPstMetadata` |
| `KREUZBERG_AUDIO` | Metadata extracted from an audio or video file. — Fields: `0`: `KreuzbergAudioMetadata` |
| `KREUZBERG_CODE` | Code (tree-sitter analyzable source). The structured analysis result is exposed via `ExtractionResult.code_intelligence`; this variant only tags the format. |

---

#### KreuzbergTextDirection

Text direction enumeration for HTML documents.

| Value | Description |
|-------|-------------|
| `KREUZBERG_LEFT_TO_RIGHT` | Left-to-right text direction |
| `KREUZBERG_RIGHT_TO_LEFT` | Right-to-left text direction |
| `KREUZBERG_AUTO` | Automatic text direction detection |

---

#### KreuzbergLinkType

Link type classification.

| Value | Description |
|-------|-------------|
| `KREUZBERG_ANCHOR` | Anchor link (#section) |
| `KREUZBERG_INTERNAL` | Internal link (same domain) |
| `KREUZBERG_EXTERNAL` | External link (different domain) |
| `KREUZBERG_EMAIL` | Email link (mailto:) |
| `KREUZBERG_PHONE` | Phone link (tel:) |
| `KREUZBERG_OTHER` | Other link type |

---

#### KreuzbergImageType

Image type classification.

| Value | Description |
|-------|-------------|
| `KREUZBERG_DATA_URI` | Data URI image |
| `KREUZBERG_INLINE_SVG` | Inline SVG |
| `KREUZBERG_EXTERNAL` | External image URL |
| `KREUZBERG_RELATIVE` | Relative path image |

---

#### KreuzbergStructuredDataType

Structured data type classification.

| Value | Description |
|-------|-------------|
| `KREUZBERG_JSON_LD` | JSON-LD structured data |
| `KREUZBERG_MICRODATA` | Microdata |
| `KREUZBERG_RDFA` | RDFa |

---

#### KreuzbergOcrBoundingGeometry

Bounding geometry for an OCR element.

Supports both axis-aligned rectangles (from Tesseract) and 4-point quadrilaterals
(from PaddleOCR and rotated text detection).

| Value | Description |
|-------|-------------|
| `KREUZBERG_RECTANGLE` | Axis-aligned bounding box (typical for Tesseract output). — Fields: `left`: `uint32_t`, `top`: `uint32_t`, `width`: `uint32_t`, `height`: `uint32_t` |
| `KREUZBERG_QUADRILATERAL` | 4-point quadrilateral for rotated/skewed text (PaddleOCR). Points are in clockwise order starting from top-left: `\[top_left, top_right, bottom_right, bottom_left\]` |

---

#### KreuzbergOcrElementLevel

Hierarchical level of an OCR element.

Maps to Tesseract's page segmentation hierarchy and provides
equivalent semantics for PaddleOCR.

| Value | Description |
|-------|-------------|
| `KREUZBERG_WORD` | Individual word |
| `KREUZBERG_LINE` | Line of text (default for PaddleOCR) |
| `KREUZBERG_BLOCK` | Paragraph or text block |
| `KREUZBERG_PAGE` | Page-level element |

---

#### KreuzbergPageUnitType

Type of paginated unit in a document.

Distinguishes between different types of "pages" (PDF pages, presentation slides, spreadsheet sheets).

| Value | Description |
|-------|-------------|
| `KREUZBERG_PAGE` | Standard document pages (PDF, DOCX, images) |
| `KREUZBERG_SLIDE` | Presentation slides (PPTX, ODP) |
| `KREUZBERG_SHEET` | Spreadsheet sheets (XLSX, ODS) |

---

#### KreuzbergRedactionStrategy

Strategy applied when a PII match is rewritten.

| Value | Description |
|-------|-------------|
| `KREUZBERG_MASK` | Replace the matched span with a fixed mask token (default `"\[REDACTED\]"`). |
| `KREUZBERG_HASH` | Replace with a SHA-256 hash of the original value (truncated to 16 hex chars). Lets downstream consumers do equality joins without recovering the source. |
| `KREUZBERG_TOKEN_REPLACE` | Replace with a per-category running token (`"\[PERSON_1\]"`, `"\[PERSON_2\]"`, …) so the same person referenced twice gets the same token within the document. |
| `KREUZBERG_DROP` | Delete the matched span entirely. |

---

#### KreuzbergPiiCategory

PII categories the pattern engine recognises.

| Value | Description |
|-------|-------------|
| `KREUZBERG_EMAIL` | Email address (e.g. `user@example.com`). |
| `KREUZBERG_PHONE` | Phone number in any common format. |
| `KREUZBERG_SSN` | US Social Security Number. |
| `KREUZBERG_CREDIT_CARD` | Payment card number (Visa, Mastercard, Amex, etc.). |
| `KREUZBERG_POSTAL_CODE` | Postal / ZIP code. |
| `KREUZBERG_IP_ADDRESS` | IPv4 or IPv6 address. |
| `KREUZBERG_IBAN` | International Bank Account Number. |
| `KREUZBERG_SWIFT_BIC` | SWIFT / BIC bank identifier code. |
| `KREUZBERG_DATE_OF_BIRTH` | Date of birth. |
| `KREUZBERG_PERSON` | Person name, surfaced by the optional NER backend. |
| `KREUZBERG_ORGANIZATION` | Organization name, surfaced by the optional NER backend. |
| `KREUZBERG_LOCATION` | Location, surfaced by the optional NER backend. |
| `KREUZBERG_CUSTOM` | Caller-supplied custom category (e.g. internal employee IDs). Surfaced by the redaction engine when a hit comes from `RedactionConfig.custom_terms` or `RedactionConfig.custom_patterns`. The string is the label passed alongside the term/pattern. Use those fields rather than constructing `Custom` directly via the `categories` filter — the pattern engine cannot detect arbitrary text from a category name alone. — Fields: `0`: `const char*` |

---

#### KreuzbergDiffLine

A single line in a unified-diff hunk.

Defined here (rather than only in `crate.diff`) so `RevisionDelta` can
reference it unconditionally, without requiring the `diff` Cargo feature.
`crate.diff` re-exports this type verbatim.

| Value | Description |
|-------|-------------|
| `KREUZBERG_CONTEXT` | Unchanged context line. — Fields: `0`: `const char*` |
| `KREUZBERG_ADDED` | Line added in the "after" version. — Fields: `0`: `const char*` |
| `KREUZBERG_REMOVED` | Line removed from the "before" version. — Fields: `0`: `const char*` |

---

#### KreuzbergRevisionKind

Semantic classification of a tracked change.

| Value | Description |
|-------|-------------|
| `KREUZBERG_INSERTION` | Text or content was inserted. |
| `KREUZBERG_DELETION` | Text or content was deleted. |
| `KREUZBERG_FORMAT_CHANGE` | Run-level formatting (font, size, colour, …) was changed. |
| `KREUZBERG_COMMENT` | A reviewer comment or annotation. |

---

#### KreuzbergRevisionAnchor

Best-effort document location for a revision.

| Value | Description |
|-------|-------------|
| `KREUZBERG_PARAGRAPH` | Body paragraph, identified by its zero-based index in the document flow. — Fields: `index`: `uintptr_t` |
| `KREUZBERG_TABLE_CELL` | Cell inside a table. — Fields: `row`: `uintptr_t`, `col`: `uintptr_t`, `table_index`: `uintptr_t` |
| `KREUZBERG_PAGE` | Page, identified by its zero-based index. — Fields: `index`: `uintptr_t` |
| `KREUZBERG_SLIDE` | Presentation slide, identified by its zero-based index. — Fields: `index`: `uintptr_t` |
| `KREUZBERG_SHEET` | Spreadsheet cell or range, identified by sheet index and optional name. — Fields: `index`: `uintptr_t`, `name`: `const char*` |

---

#### KreuzbergSummaryStrategy

Summarisation strategy.

| Value | Description |
|-------|-------------|
| `KREUZBERG_EXTRACTIVE` | Pure-Rust extractive summary (TextRank over the chunk graph). Deterministic, fast, no external service required. |
| `KREUZBERG_ABSTRACTIVE` | Abstractive summary produced by liter-llm. Requires `liter-llm` feature and a configured `LlmConfig`. Token usage is captured in `ExtractionResult.llm_usage`. |

---

#### KreuzbergUriKind

Semantic classification of an extracted URI.

| Value | Description |
|-------|-------------|
| `KREUZBERG_HYPERLINK` | A clickable hyperlink (web URL, file link). |
| `KREUZBERG_IMAGE` | An image or media resource reference. |
| `KREUZBERG_ANCHOR` | An internal anchor or cross-reference target. |
| `KREUZBERG_CITATION` | A citation or bibliographic reference (DOI, academic ref). |
| `KREUZBERG_REFERENCE` | A general reference (e.g. `\ref{}` in LaTeX, `:ref:` in RST). |
| `KREUZBERG_EMAIL` | An email address (`mailto:` link or bare email). |

---

#### KreuzbergRegionKind

Classification of a detected layout region that warrants VLM extraction.

Each variant maps to a specific prompt optimised for that content type.
The mapping is intentionally narrow — only region kinds for which VLM
extraction provides a clear quality benefit over classical suppression.

| Value | Description |
|-------|-------------|
| `KREUZBERG_FIGURE` | A figure, diagram, chart, or image region. VLM prompt: describe the diagram / chart, including axis labels, legend entries, and any embedded text. |
| `KREUZBERG_DENSE_TABLE` | A densely formatted or complex table that classical extraction garbles. VLM prompt: extract the table as GitHub-Flavoured Markdown. |
| `KREUZBERG_COMPLEX_LAYOUT` | A region whose layout the classical pipeline cannot handle (multi-column insets, heavily annotated forms, mixed text+diagram). VLM prompt: extract all text and structure as markdown, preserving reading order. |
| `KREUZBERG_CAPTION` | A standalone image to be captioned (not extracted as figure markdown). VLM prompt: produce a single-sentence alt-text-style caption suitable for accessibility tooling and downstream indexing. Used by the captioning post-processor to populate `ExtractedImage.caption`. |

---

#### KreuzbergKeywordAlgorithm

Keyword algorithm selection.

| Value | Description |
|-------|-------------|
| `KREUZBERG_YAKE` | YAKE (Yet Another Keyword Extractor) - statistical approach |
| `KREUZBERG_RAKE` | RAKE (Rapid Automatic Keyword Extraction) - co-occurrence based |

---

#### KreuzbergEnrichStatus

Async lifecycle status for an enrichment job.

Intended for use with any polling or event-driven pipeline that needs
to track whether enrichment has completed, succeeded, or failed.

### Serialisation

Uses an internally-tagged `"status"` field with `snake_case` variants:

```json
{ "status": "pending" }
{ "status": "completed", "result": { ... } }
{ "status": "failed", "error": "text too large" }
```

| Value | Description |
|-------|-------------|
| `KREUZBERG_PENDING` | Job submitted; processing has not yet started or is in progress. |
| `KREUZBERG_COMPLETED` | Processing completed successfully. — Fields: `result`: `KreuzbergEnrichResult` |
| `KREUZBERG_FAILED` | Processing failed. — Fields: `error`: `const char*` |

---

#### KreuzbergSchemaCompliance

Schema-validation outcome surfaced as one of three buckets.

Fold into the combined confidence score without leaking internal validation
error types.

| Value | Description |
|-------|-------------|
| `KREUZBERG_ALL_VALID` | Every batch validated against the schema. |
| `KREUZBERG_PARTIAL_VALID` | At least one batch validated; at least one did not. |
| `KREUZBERG_ALL_INVALID` | No batch validated. |

---

#### KreuzbergChunkingDecision

The chunking decision made by the analyzer.

| Value | Description |
|-------|-------------|
| `KREUZBERG_NO_CHUNKING` | Process without chunking (small file, text layer detected, etc.) — Fields: `reason`: `KreuzbergNoChunkingReason` |
| `KREUZBERG_CHUNK` | Chunk according to plan. — Fields: `0`: `KreuzbergChunkPlan` |
| `KREUZBERG_USE_OVERRIDES` | Use user-provided chunk overrides. — Fields: `user_chunks`: `KreuzbergPageRange*` |

---

#### KreuzbergNoChunkingReason

Reason for not chunking a document.

| Value | Description |
|-------|-------------|
| `KREUZBERG_SMALL_FILE` | File is below size threshold. — Fields: `size_bytes`: `uint64_t`, `threshold_bytes`: `uint64_t` |
| `KREUZBERG_FEW_PAGES` | Document has fewer pages than threshold. — Fields: `page_count`: `uint32_t`, `threshold`: `uint32_t` |
| `KREUZBERG_TEXT_LAYER_DETECTED` | PDF has substantial text layer (OCR not needed). — Fields: `text_coverage`: `float`, `avg_chars_per_page`: `uint32_t` |
| `KREUZBERG_FORMAT_NOT_CHUNKABLE` | Document format does not support chunking. — Fields: `mime_type`: `const char*` |
| `KREUZBERG_CHUNKING_DISABLED` | Chunking is disabled by configuration. |
| `KREUZBERG_FAST_TEXT_EXTRACTION` | Force OCR is disabled and text extraction is fast. |

---

#### KreuzbergChunkingReason

Reason for chunking a document.

| Value | Description |
|-------|-------------|
| `KREUZBERG_LARGE_FILE` | File exceeds size threshold. — Fields: `size_bytes`: `uint64_t`, `threshold_bytes`: `uint64_t` |
| `KREUZBERG_MANY_PAGES` | Document has many pages. — Fields: `page_count`: `uint32_t`, `threshold`: `uint32_t` |
| `KREUZBERG_OCR_REQUIRED` | PDF requires OCR and is large. — Fields: `page_count`: `uint32_t`, `force_ocr`: `bool` |
| `KREUZBERG_LARGE_AND_MANY_PAGES` | Both size and page count exceed thresholds. — Fields: `size_bytes`: `uint64_t`, `page_count`: `uint32_t` |

---

#### KreuzbergBoundaryReason

Reason for boundary detection.

| Value | Description |
|-------|-------------|
| `KREUZBERG_START` | Start of PDF. |
| `KREUZBERG_PAGE_ONE_MARKER` | Page-one marker ("Page 1", "1 of N") detected. |
| `KREUZBERG_LETTERHEAD_RESET` | Letterhead reset after signature block. |
| `KREUZBERG_DENSITY_SHIFT` | Text density shift with low bigram overlap. |
| `KREUZBERG_END` | End of PDF. |

---

#### KreuzbergStructuredCallMode

Outcome of the structured-extraction call-mode heuristic.

**Distinct from `crate.core.config.CallMode`** which has three variants
and governs extraction-engine behaviour.  This enum governs whether and how
an already-extracted document is sent to an LLM structured-extraction
pipeline.

| Value | Description |
|-------|-------------|
| `KREUZBERG_SKIP` | Document is unsupported or not worth invoking the pipeline. |
| `KREUZBERG_TEXT_ONLY` | Send extracted text only; no vision model call. |
| `KREUZBERG_VISION_ONLY` | Send page rasters only; no extracted text payload. |
| `KREUZBERG_TEXT_PLUS_VISION` | Fuse extracted text with page rasters in a single multimodal call. |
| `KREUZBERG_TEXT_ONLY_WITH_VISION_FALLBACK` | Try text-only first; escalate to vision on low confidence score. |

---

#### KreuzbergPresetCategory

High-level category used to group presets in the registry UI.

| Value | Description |
|-------|-------------|
| `KREUZBERG_FINANCE` | Invoices, receipts, statements, purchase orders, W-9. |
| `KREUZBERG_IDENTITY` | Passports, drivers licenses, insurance cards. |
| `KREUZBERG_LEGAL` | Contracts, NDAs, agreements. |
| `KREUZBERG_LOGISTICS` | Bills of lading, customs declarations, packing lists. |
| `KREUZBERG_MEDICAL` | Clinical records, lab reports. |
| `KREUZBERG_HR` | Pay stubs, resumes, employment offers. |
| `KREUZBERG_OTHER` | Catch-all for documents that don't fit the other categories. |

---

#### KreuzbergPsmMode

Page Segmentation Mode for Tesseract OCR.

| Value | Description |
|-------|-------------|
| `KREUZBERG_OSD_ONLY` | Orientation and script detection only. |
| `KREUZBERG_AUTO_OSD` | Automatic page segmentation with OSD. |
| `KREUZBERG_AUTO_ONLY` | Automatic page segmentation without OSD or OCR. |
| `KREUZBERG_AUTO` | Fully automatic page segmentation with no OSD (default). |
| `KREUZBERG_SINGLE_COLUMN` | Assume a single column of text of variable sizes. |
| `KREUZBERG_SINGLE_BLOCK_VERTICAL` | Assume a single uniform block of vertically aligned text. |
| `KREUZBERG_SINGLE_BLOCK` | Assume a single uniform block of text. |
| `KREUZBERG_SINGLE_LINE` | Treat the image as a single text line. |
| `KREUZBERG_SINGLE_WORD` | Treat the image as a single word. |
| `KREUZBERG_CIRCLE_WORD` | Treat the image as a single word in a circle. |
| `KREUZBERG_SINGLE_CHAR` | Treat the image as a single character. |

---

#### KreuzbergPaddleLanguage

Supported languages in PaddleOCR.

Maps user-friendly language codes to paddle-ocr-rs language identifiers.

| Value | Description |
|-------|-------------|
| `KREUZBERG_ENGLISH` | English |
| `KREUZBERG_CHINESE` | Simplified Chinese |
| `KREUZBERG_JAPANESE` | Japanese |
| `KREUZBERG_KOREAN` | Korean |
| `KREUZBERG_GERMAN` | German |
| `KREUZBERG_FRENCH` | French |
| `KREUZBERG_LATIN` | Latin script (covers most European languages) |
| `KREUZBERG_CYRILLIC` | Cyrillic (Russian and related) |
| `KREUZBERG_TRADITIONAL_CHINESE` | Traditional Chinese |
| `KREUZBERG_THAI` | Thai |
| `KREUZBERG_GREEK` | Greek |
| `KREUZBERG_EAST_SLAVIC` | East Slavic (Russian, Ukrainian, Belarusian) |
| `KREUZBERG_ARABIC` | Arabic (Arabic, Persian, Urdu) |
| `KREUZBERG_DEVANAGARI` | Devanagari (Hindi, Marathi, Sanskrit, Nepali) |
| `KREUZBERG_TAMIL` | Tamil |
| `KREUZBERG_TELUGU` | Telugu |

---

#### KreuzbergLayoutClass

The 18 canonical document layout classes.

All model backends (RT-DETR, YOLO, etc.) map their native class IDs
to this shared set. Models with fewer classes (DocLayNet: 11, PubLayNet: 5)
map to the closest equivalent.

Wire format is snake_case in all serializers (JSON, TOML, YAML).

| Value | Description |
|-------|-------------|
| `KREUZBERG_CAPTION` | Figure or table caption text. |
| `KREUZBERG_CHART` | Chart or graph visualization. |
| `KREUZBERG_FOOTNOTE` | Footnote or endnote text. |
| `KREUZBERG_FORMULA` | Mathematical formula or equation. |
| `KREUZBERG_LIST_ITEM` | A single item in a bulleted or numbered list. |
| `KREUZBERG_PAGE_FOOTER` | Running footer at the bottom of a page. |
| `KREUZBERG_PAGE_HEADER` | Running header at the top of a page. |
| `KREUZBERG_PICTURE` | Image, chart, or other graphical element. |
| `KREUZBERG_SECTION_HEADER` | Section heading. |
| `KREUZBERG_TABLE` | Data table. |
| `KREUZBERG_TEXT` | Body text paragraph. |
| `KREUZBERG_TITLE` | Document or chapter title. |
| `KREUZBERG_DOCUMENT_INDEX` | Table of contents or index. |
| `KREUZBERG_CODE` | Source code block. |
| `KREUZBERG_CHECKBOX_SELECTED` | Checkbox in selected state. |
| `KREUZBERG_CHECKBOX_UNSELECTED` | Checkbox in unselected state. |
| `KREUZBERG_FORM` | Form field or form element. |
| `KREUZBERG_KEY_VALUE_REGION` | Key-value pair region (e.g. label + value in a form). |

---

### Errors

#### KreuzbergKreuzbergError

Main error type for all Kreuzberg operations.

All errors in Kreuzberg use this enum, which preserves error chains
and provides context for debugging.

### Variants

- `Io` - File system and I/O errors (always bubble up)
- `Parsing` - Document parsing errors (corrupt files, unsupported features)
- `Ocr` - OCR processing errors
- `Validation` - Input validation errors (invalid paths, config, parameters)
- `Cache` - Cache operation errors (non-fatal, can be ignored)
- `ImageProcessing` - Image manipulation errors
- `Serialization` - JSON/MessagePack serialization errors
- `MissingDependency` - Missing optional dependencies (tesseract, etc.)
- `Plugin` - Plugin-specific errors
- `LockPoisoned` - Mutex/RwLock poisoning (should not happen in normal operation)
- `UnsupportedFormat` - Unsupported MIME type or file format
- `Other` - Catch-all for uncommon errors

| Variant | Description |
|---------|-------------|
| `KREUZBERG_IO` | A file system or I/O operation failed. These errors always bubble up unchanged. |
| `KREUZBERG_PARSING` | Document parsing failed (e.g. corrupt file, unsupported format feature). |
| `KREUZBERG_OCR` | An OCR engine returned an error or produced unusable output. |
| `KREUZBERG_VALIDATION` | Invalid configuration or input parameters were supplied. |
| `KREUZBERG_CACHE` | A cache read or write operation failed. |
| `KREUZBERG_IMAGE_PROCESSING` | An image manipulation operation (resize, decode, DPI conversion) failed. |
| `KREUZBERG_SERIALIZATION` | JSON or MessagePack serialization/deserialization failed. |
| `KREUZBERG_MISSING_DEPENDENCY` | A required optional system dependency (e.g. `tesseract`) was not found. |
| `KREUZBERG_PLUGIN` | A registered plugin returned an error during extraction. |
| `KREUZBERG_LOCK_POISONED` | An internal `Mutex` or `RwLock` was found in a poisoned state. |
| `KREUZBERG_UNSUPPORTED_FORMAT` | The document's MIME type is not supported by any registered extractor. |
| `KREUZBERG_EMBEDDING` | The embedding model or embedding pipeline returned an error. |
| `KREUZBERG_RERANKING` | The reranker model or reranking pipeline returned an error. Since v5.0. |
| `KREUZBERG_TRANSCRIPTION` | Audio/video transcription failed. |
| `KREUZBERG_TIMEOUT` | The extraction operation exceeded the configured time limit. |
| `KREUZBERG_CANCELLED` | The extraction was cancelled via a `CancellationToken`. |
| `KREUZBERG_SECURITY` | A security policy was violated (e.g. zip bomb, oversized archive). |
| `KREUZBERG_OTHER` | A catch-all for uncommon errors that do not fit another variant. |

---

#### KreuzbergHeuristicsError

Errors that can occur during heuristics analysis.

| Variant | Description |
|---------|-------------|
| `KREUZBERG_CONFIG_ERROR` | Invalid configuration value. |
| `KREUZBERG_PDF_ANALYSIS_ERROR` | PDF analysis step failed (only when `heuristics-pdf` feature is active). |

---

#### KreuzbergLoadError

Errors produced while loading or validating a preset file.

| Variant | Description |
|---------|-------------|
| `KREUZBERG_PARSE` | The file is not valid JSON. |
| `KREUZBERG_SCHEMA_VALIDATION` | The file parses as JSON but does not validate against the meta-schema. |
| `KREUZBERG_DESERIALIZE` | The file validates but cannot be deserialized into `Preset`. |
| `KREUZBERG_ID_MISMATCH` | The preset's declared `id` does not match its file-system location. |
| `KREUZBERG_BAD_META_SCHEMA` | The meta-schema itself failed to compile. |
| `KREUZBERG_IO` | A filesystem I/O error occurred while reading a preset directory. |

---

#### KreuzbergResolveError

Errors produced while resolving a preset against caller overrides.

| Variant | Description |
|---------|-------------|
| `KREUZBERG_SCHEMA_NOT_OBJECT` | A custom schema override was supplied but is not a JSON object. |

---

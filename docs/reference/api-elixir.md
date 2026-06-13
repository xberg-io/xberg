---
title: "Elixir API Reference"
---

## Elixir API Reference <span class="version-badge">v5.0.0-rc.11</span>

### Functions

#### extract_bytes()

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

```elixir
@spec extract_bytes(content, mime_type, config) :: {:ok, term()} | {:error, term()}
def extract_bytes(content, mime_type, config)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `content` | `binary()` | Yes | The byte array to extract |
| `mime_type` | `String.t()` | Yes | MIME type of the content |
| `config` | `ExtractionConfig` | Yes | Extraction configuration |

**Returns:** `ExtractionResult`
**Errors:** Returns `{:error, reason}`

---

#### extract_file()

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

```elixir
@spec extract_file(path, mime_type, config) :: {:ok, term()} | {:error, term()}
def extract_file(path, mime_type, config)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `path` | `String.t()` | Yes | Path to the file to extract |
| `mime_type` | `String.t() \| nil` | No | Optional MIME type override. If None, will be auto-detected |
| `config` | `ExtractionConfig` | Yes | Extraction configuration |

**Returns:** `ExtractionResult`
**Errors:** Returns `{:error, reason}`

---

#### extract_file_sync()

Synchronous wrapper for `extract_file`.

This is a convenience function that blocks the current thread until extraction completes.
For async code, use `extract_file` directly.

Uses the global Tokio runtime for 100x+ performance improvement over creating
a new runtime per call. Always uses the global runtime to avoid nested runtime issues.

This function is only available with the `tokio-runtime` feature. For WASM targets,
use a truly synchronous extraction approach instead.

**Signature:**

```elixir
@spec extract_file_sync(path, mime_type, config) :: {:ok, term()} | {:error, term()}
def extract_file_sync(path, mime_type, config)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `path` | `String.t()` | Yes | Path to the file |
| `mime_type` | `String.t() \| nil` | No | The mime type |
| `config` | `ExtractionConfig` | Yes | The configuration options |

**Returns:** `ExtractionResult`
**Errors:** Returns `{:error, reason}`

---

#### extract_bytes_sync()

Synchronous wrapper for `extract_bytes`.

Uses the global Tokio runtime for 100x+ performance improvement over creating
a new runtime per call.

With the `tokio-runtime` feature, this blocks the current thread using the global
Tokio runtime. Without it (WASM), this calls a truly synchronous implementation.

**Signature:**

```elixir
@spec extract_bytes_sync(content, mime_type, config) :: {:ok, term()} | {:error, term()}
def extract_bytes_sync(content, mime_type, config)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `content` | `binary()` | Yes | The content to process |
| `mime_type` | `String.t()` | Yes | The mime type |
| `config` | `ExtractionConfig` | Yes | The configuration options |

**Returns:** `ExtractionResult`
**Errors:** Returns `{:error, reason}`

---

#### batch_extract_files_sync()

Synchronous wrapper for `batch_extract_files`.

Uses the global Tokio runtime for optimal performance.
Only available with `tokio-runtime` (WASM has no filesystem).

**Signature:**

```elixir
@spec batch_extract_files_sync(items, config) :: {:ok, term()} | {:error, term()}
def batch_extract_files_sync(items, config)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `items` | `list(BatchFileItem)` | Yes | The items |
| `config` | `ExtractionConfig` | Yes | The configuration options |

**Returns:** `list(ExtractionResult)`
**Errors:** Returns `{:error, reason}`

---

#### batch_extract_bytes_sync()

Synchronous wrapper for `batch_extract_bytes`.

Uses the global Tokio runtime for optimal performance.
With the `tokio-runtime` feature, this blocks the current thread using the global
Tokio runtime. Without it (WASM), this calls a truly synchronous implementation
that iterates through items and calls `extract_bytes_sync()`.

**Signature:**

```elixir
@spec batch_extract_bytes_sync(items, config) :: {:ok, term()} | {:error, term()}
def batch_extract_bytes_sync(items, config)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `items` | `list(BatchBytesItem)` | Yes | The items |
| `config` | `ExtractionConfig` | Yes | The configuration options |

**Returns:** `list(ExtractionResult)`
**Errors:** Returns `{:error, reason}`

---

#### batch_extract_files()

Extract content from multiple files concurrently.

This function processes multiple files in parallel, automatically managing
concurrency to prevent resource exhaustion. The concurrency limit can be
configured via `ExtractionConfig.max_concurrent_extractions` or defaults
to `(num_cpus * 1.5).ceil()`.

Each file can optionally specify a `FileExtractionConfig` that overrides specific
fields from the batch-level `config`. Pass `nil` for a file to use the batch defaults.
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

```elixir
@spec batch_extract_files(items, config) :: {:ok, term()} | {:error, term()}
def batch_extract_files(items, config)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `items` | `list(BatchFileItem)` | Yes | Vector of `BatchFileItem` structs, each containing a path and optional |
| `config` | `ExtractionConfig` | Yes | Batch-level extraction configuration (provides defaults and batch settings) |

**Returns:** `list(ExtractionResult)`
**Errors:** Returns `{:error, reason}`

---

#### batch_extract_bytes()

Extract content from multiple byte arrays concurrently.

This function processes multiple byte arrays in parallel, automatically managing
concurrency to prevent resource exhaustion. The concurrency limit can be
configured via `ExtractionConfig.max_concurrent_extractions` or defaults
to `(num_cpus * 1.5).ceil()`.

Each item can optionally specify a `FileExtractionConfig` that overrides specific
fields from the batch-level `config`. Pass `nil` as the config to use
the batch-level defaults for that item.

  MIME type, and optional per-item configuration overrides.

- `config` - Batch-level extraction configuration

**Returns:**

A vector of `ExtractionResult` in the same order as the input items.

Simple usage with no per-item overrides:

Per-item configuration overrides:

**Signature:**

```elixir
@spec batch_extract_bytes(items, config) :: {:ok, term()} | {:error, term()}
def batch_extract_bytes(items, config)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `items` | `list(BatchBytesItem)` | Yes | Vector of `BatchBytesItem` structs, each containing content bytes, |
| `config` | `ExtractionConfig` | Yes | Batch-level extraction configuration |

**Returns:** `list(ExtractionResult)`
**Errors:** Returns `{:error, reason}`

---

#### detect_mime_type_from_bytes()

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

```elixir
@spec detect_mime_type_from_bytes(content) :: {:ok, term()} | {:error, term()}
def detect_mime_type_from_bytes(content)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `content` | `binary()` | Yes | Raw file bytes |

**Returns:** `String.t()`
**Errors:** Returns `{:error, reason}`

---

#### get_extensions_for_mime()

Get file extensions for a given MIME type.

Returns all known file extensions that map to the specified MIME type.

**Returns:**

A vector of file extensions (without leading dot) for the MIME type.

**Signature:**

```elixir
@spec get_extensions_for_mime(mime_type) :: {:ok, term()} | {:error, term()}
def get_extensions_for_mime(mime_type)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `mime_type` | `String.t()` | Yes | The MIME type to look up |

**Returns:** `list(String.t())`
**Errors:** Returns `{:error, reason}`

---

#### list_supported_formats()

List all supported document formats.

Returns every file extension Kreuzberg recognizes together with its
corresponding MIME type, derived from the central format registry.
Formats that have no registered file extension (such as source code,
which is detected dynamically) are not included.

The list is sorted alphabetically by file extension.

**Returns:**

A vector of `SupportedFormat` entries sorted by extension.

**Signature:**

```elixir
@spec list_supported_formats() :: {:ok, term()} | {:error, term()}
def list_supported_formats()
```

**Returns:** `list(SupportedFormat)`

---

#### detect_qr_codes()

Detect QR codes in the bytes of an `ExtractedImage`.

`format_hint` is currently unused — the `image` crate auto-detects the
container format from magic bytes — but the parameter is retained so future
backends (e.g. a WebP-via-`webp-decoder` variant) can use it without an API
break.

Returns an empty vector on any of:

- Empty input.
- Image-decode failure.
- No QR grids detected.
- All detected grids fail to decode.

Successfully decoded QR codes carry their payload, a confidence of `1.0`
(rqrr does not expose per-grid confidence; a successful decode is treated
as high-confidence by convention), and the pixel-space bounding box derived
from the four corner points of the grid.

**Signature:**

```elixir
@spec detect_qr_codes(image_bytes, format_hint) :: {:ok, term()} | {:error, term()}
def detect_qr_codes(image_bytes, format_hint)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `image_bytes` | `binary()` | Yes | The image bytes |
| `format_hint` | `String.t() \| nil` | No | The  format hint |

**Returns:** `list(QrCode)`

---

#### clear_embedding_backends()

Clear all embedding backends from the global registry.

Calls `shutdown()` on every registered backend, then empties the registry.

**Errors:**

- Any error returned by a backend's `shutdown()` method. The first error
  encountered stops processing of remaining backends.

**Signature:**

```elixir
@spec clear_embedding_backends() :: {:ok, term()} | {:error, term()}
def clear_embedding_backends()
```

**Returns:** `:ok`
**Errors:** Returns `{:error, reason}`

---

#### list_embedding_backends()

List the names of all registered embedding backends.

Used by `kreuzberg-cli`, the api/mcp endpoints, and generated language
bindings.

**Signature:**

```elixir
@spec list_embedding_backends() :: {:ok, term()} | {:error, term()}
def list_embedding_backends()
```

**Returns:** `list(String.t())`
**Errors:** Returns `{:error, reason}`

---

#### list_document_extractors()

List names of all registered document extractors.

**Signature:**

```elixir
@spec list_document_extractors() :: {:ok, term()} | {:error, term()}
def list_document_extractors()
```

**Returns:** `list(String.t())`
**Errors:** Returns `{:error, reason}`

---

#### clear_document_extractors()

Clear all document extractors from the global registry.

Calls `shutdown()` on every registered extractor, then empties the registry.

**Errors:**

- Any error returned by an extractor's `shutdown()` method. The first error
  encountered stops processing of remaining extractors.

**Signature:**

```elixir
@spec clear_document_extractors() :: {:ok, term()} | {:error, term()}
def clear_document_extractors()
```

**Returns:** `:ok`
**Errors:** Returns `{:error, reason}`

---

#### list_ocr_backends()

List all registered OCR backends.

Returns the names of all OCR backends currently registered in the global registry.

**Returns:**

A vector of OCR backend names.

**Signature:**

```elixir
@spec list_ocr_backends() :: {:ok, term()} | {:error, term()}
def list_ocr_backends()
```

**Returns:** `list(String.t())`
**Errors:** Returns `{:error, reason}`

---

#### clear_ocr_backends()

Clear all OCR backends from the global registry.

Removes all OCR backends and calls their `shutdown()` methods.

**Returns:**

- `Ok(())` if all backends were cleared successfully
- `Err(...)` if any shutdown method failed

**Signature:**

```elixir
@spec clear_ocr_backends() :: {:ok, term()} | {:error, term()}
def clear_ocr_backends()
```

**Returns:** `:ok`
**Errors:** Returns `{:error, reason}`

---

#### register_builtin()

Register every built-in post-processor enabled by the active feature set.

This is the single entry point that callers (including
`register_default_post_processors`) use to populate the global
post-processor registry with the in-tree built-ins. Each submodule's own
`register` function is gated by its feature flag so this aggregate stays
safe to call on any target.

**Signature:**

```elixir
@spec register_builtin() :: {:ok, term()} | {:error, term()}
def register_builtin()
```

**Returns:** `:ok`
**Errors:** Returns `{:error, reason}`

---

#### list_post_processors()

List all registered post-processor names.

Returns a vector of all post-processor names currently registered in the
global registry.

**Returns:**

- `Ok(Vec<String>)` - Vector of post-processor names
- `Err(...)` if the registry lock is poisoned

**Signature:**

```elixir
@spec list_post_processors() :: {:ok, term()} | {:error, term()}
def list_post_processors()
```

**Returns:** `list(String.t())`
**Errors:** Returns `{:error, reason}`

---

#### clear_post_processors()

Remove all registered post-processors.

**Signature:**

```elixir
@spec clear_post_processors() :: {:ok, term()} | {:error, term()}
def clear_post_processors()
```

**Returns:** `:ok`
**Errors:** Returns `{:error, reason}`

---

#### list_renderers()

List names of all registered renderers.

**Errors:**

Returns an error if the registry lock is poisoned.

**Signature:**

```elixir
@spec list_renderers() :: {:ok, term()} | {:error, term()}
def list_renderers()
```

**Returns:** `list(String.t())`
**Errors:** Returns `{:error, reason}`

---

#### clear_renderers()

Clear all renderers from the global registry.

Removes every renderer, including the built-in defaults (markdown, html,
djot, plain). After calling this no renderers are registered; re-register
as needed.

**Errors:**

Returns an error if the registry lock is poisoned.

**Signature:**

```elixir
@spec clear_renderers() :: {:ok, term()} | {:error, term()}
def clear_renderers()
```

**Returns:** `:ok`
**Errors:** Returns `{:error, reason}`

---

#### clear_reranker_backends()

Clear all reranker backends from the global registry.

Calls `shutdown()` on every registered backend, then empties the registry.

**Errors:**

- Any error returned by a backend's `shutdown()` method. The first error
  encountered stops processing of remaining backends.

Since v5.0.0.

**Signature:**

```elixir
@spec clear_reranker_backends() :: {:ok, term()} | {:error, term()}
def clear_reranker_backends()
```

**Returns:** `:ok`
**Errors:** Returns `{:error, reason}`

---

#### list_reranker_backends()

List the names of all registered reranker backends.

Used by `kreuzberg-cli`, the api/mcp endpoints, and generated language
bindings.

Since v5.0.0.

**Signature:**

```elixir
@spec list_reranker_backends() :: {:ok, term()} | {:error, term()}
def list_reranker_backends()
```

**Returns:** `list(String.t())`
**Errors:** Returns `{:error, reason}`

---

#### list_validators()

List names of all registered validators.

**Signature:**

```elixir
@spec list_validators() :: {:ok, term()} | {:error, term()}
def list_validators()
```

**Returns:** `list(String.t())`
**Errors:** Returns `{:error, reason}`

---

#### clear_validators()

Remove all registered validators.

**Signature:**

```elixir
@spec clear_validators() :: {:ok, term()} | {:error, term()}
def clear_validators()
```

**Returns:** `:ok`
**Errors:** Returns `{:error, reason}`

---

#### classify_pages()

Run page classification against an extraction result.

Mutates `result.page_classifications` with one entry per non-empty page and
appends every LLM call's usage to `result.llm_usage`.

**Errors:**

Returns the first error encountered when rendering the prompt or calling the
LLM. Partially produced classifications are discarded so callers do not see
a half-populated vector.

**Signature:**

```elixir
@spec classify_pages(result, config) :: {:ok, term()} | {:error, term()}
def classify_pages(result, config)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `result` | `ExtractionResult` | Yes | The extraction result |
| `config` | `PageClassificationConfig` | Yes | The configuration options |

**Returns:** `:ok`
**Errors:** Returns `{:error, reason}`

---

#### classify_text()

Classify a single piece of text without requiring an `ExtractionResult`.

Use this when the caller already has plain text (e.g. a RAG ingest pipeline
receiving documents off a queue) and wants a label list back without
manufacturing extractor-side metadata.

**Errors:**

Same as `classify_pages`: a validation error when `config.labels` is empty,
or any error returned by prompt rendering or the underlying LLM call.

**Signature:**

```elixir
@spec classify_text(text, config) :: {:ok, term()} | {:error, term()}
def classify_text(text, config)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `String.t()` | Yes | The text |
| `config` | `PageClassificationConfig` | Yes | The configuration options |

**Returns:** `list(ClassificationLabel)`
**Errors:** Returns `{:error, reason}`

---

#### download_model()

Eagerly download a NER model into the kreuzberg cache.

`name` is a HuggingFace repo id (e.g. `urchade/gliner_multi-v2.1`). The
CLI flag `kreuzberg warm --ner` delegates here.

**Signature:**

```elixir
@spec download_model(name, cache_dir) :: {:ok, term()} | {:error, term()}
def download_model(name, cache_dir)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `name` | `String.t()` | Yes | The name |
| `cache_dir` | `String.t() \| nil` | No | The cache dir |

**Returns:** `String.t()`
**Errors:** Returns `{:error, reason}`

---

#### default_model_name()

Pinned default NER model identifier.

**Signature:**

```elixir
@spec default_model_name() :: {:ok, term()} | {:error, term()}
def default_model_name()
```

**Returns:** `String.t()`

---

#### known_models()

All NER models kreuzberg knows about (used by `--all-ner-models`).

**Signature:**

```elixir
@spec known_models() :: {:ok, term()} | {:error, term()}
def known_models()
```

**Returns:** `list(String.t())`

---

#### redact()

Run pattern redaction (and optional NER-driven redaction) over `result` and
rewrite every textual field. Populates `result.redaction_report`.

**Signature:**

```elixir
@spec redact(result, config) :: {:ok, term()} | {:error, term()}
def redact(result, config)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `result` | `ExtractionResult` | Yes | The extraction result |
| `config` | `RedactionConfig` | Yes | The configuration options |

**Returns:** `:ok`
**Errors:** Returns `{:error, reason}`

---

#### find_all()

Find all US Social Security Number spans in `text` (format: NNN-NN-NNNN).

**Signature:**

```elixir
@spec find_all(text) :: {:ok, term()} | {:error, term()}
def find_all(text)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `String.t()` | Yes | The text |

**Returns:** `list(PatternMatch)`

---

#### scan_text()

Scan `text` for every PII category in `categories` and return all matches
in source-byte order.

When `categories` is empty every supported regex-detectable category fires.
Person / Organization / Location are *not* covered by the pattern engine —
they must be supplied by a NER backend through the redaction engine.

**Signature:**

```elixir
@spec scan_text(text, categories) :: {:ok, term()} | {:error, term()}
def scan_text(text, categories)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `String.t()` | Yes | The text |
| `categories` | `list(PiiCategory)` | Yes | The categories |

**Returns:** `list(PatternMatch)`

---

#### summarize()

Score and return the top-N sentences from `text`, joined in original order.

`language` is an ISO 639 (or locale) code used to pick a stopword list;
pass `nil` (or an unknown code) to fall back to English.
`max_tokens` bounds the summary length by whitespace-separated tokens;
`nil` falls back to `DEFAULT_MAX_TOKENS`.

**Signature:**

```elixir
@spec summarize(text, language, max_tokens) :: {:ok, term()} | {:error, term()}
def summarize(text, language, max_tokens)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `String.t()` | Yes | The text |
| `language` | `String.t() \| nil` | No | The language |
| `max_tokens` | `integer() \| nil` | No | The max tokens |

**Returns:** `String.t() | nil`

---

#### token_count()

Count whitespace-separated tokens (used for token-budget bookkeeping by
callers).

**Signature:**

```elixir
@spec token_count(text) :: {:ok, term()} | {:error, term()}
def token_count(text)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `String.t()` | Yes | The text |

**Returns:** `integer()`

---

#### translate_result()

Translate the extraction result in place.

Populates `result.translation` with the translated `content`, optionally the
translated `formatted_content` (when `preserve_markup = true`), and rewrites
every chunk's `content` field. Every LLM call's usage is appended to
`result.llm_usage`.

**Signature:**

```elixir
@spec translate_result(result, config) :: {:ok, term()} | {:error, term()}
def translate_result(result, config)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `result` | `ExtractionResult` | Yes | The extraction result |
| `config` | `TranslationConfig` | Yes | The configuration options |

**Returns:** `:ok`
**Errors:** Returns `{:error, reason}`

---

#### compare()

Compare two extraction results and return a structured diff.

The comparison is purely structural — no I/O, no side effects. All fields
of `ExtractionDiff` are populated according to the provided `DiffOptions`.

**Signature:**

```elixir
@spec compare(a, b, opts) :: {:ok, term()} | {:error, term()}
def compare(a, b, opts)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `a` | `ExtractionResult` | Yes | The extraction result |
| `b` | `ExtractionResult` | Yes | The extraction result |
| `opts` | `DiffOptions` | Yes | The options to use |

**Returns:** `ExtractionDiff`

---

#### extract_region_with_vlm()

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

```elixir
@spec extract_region_with_vlm(image_bytes, image_mime, region_kind, llm_config, custom_prompt) :: {:ok, term()} | {:error, term()}
def extract_region_with_vlm(image_bytes, image_mime, region_kind, llm_config, custom_prompt)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `image_bytes` | `binary()` | Yes | The image bytes |
| `image_mime` | `String.t()` | Yes | The image mime |
| `region_kind` | `RegionKind` | Yes | The region kind |
| `llm_config` | `LlmConfig` | Yes | The llm config |
| `custom_prompt` | `String.t() \| nil` | No | The custom prompt |

**Returns:** `String.t()`
**Errors:** Returns `{:error, reason}`

---

#### extract_keywords()

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

```elixir
@spec extract_keywords(text, config) :: {:ok, term()} | {:error, term()}
def extract_keywords(text, config)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `String.t()` | Yes | The text to extract keywords from |
| `config` | `KeywordConfig` | Yes | Keyword extraction configuration |

**Returns:** `list(Keyword)`
**Errors:** Returns `{:error, reason}`

---

#### render_pdf_page_to_png()

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

```elixir
@spec render_pdf_page_to_png(pdf_bytes, page_index, dpi, password) :: {:ok, term()} | {:error, term()}
def render_pdf_page_to_png(pdf_bytes, page_index, dpi, password)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `pdf_bytes` | `binary()` | Yes | Raw PDF file bytes |
| `page_index` | `integer()` | Yes | Zero-based page index |
| `dpi` | `integer() \| nil` | No | Resolution in dots per inch (default: 150) |
| `password` | `String.t() \| nil` | No | Optional password for encrypted PDFs |

**Returns:** `binary()`
**Errors:** Returns `{:error, reason}`

---

#### detect_mime_type()

Detect the MIME type of a file at the given path.

Uses the file extension and optionally the file content to determine the MIME type.
Set `check_exists` to `true` to verify the file exists before detection.

**Signature:**

```elixir
@spec detect_mime_type(path, check_exists) :: {:ok, term()} | {:error, term()}
def detect_mime_type(path, check_exists)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `path` | `String.t()` | Yes | Path to the file |
| `check_exists` | `boolean()` | Yes | The check exists |

**Returns:** `String.t()`
**Errors:** Returns `{:error, reason}`

---

#### embed_texts_async()

**Signature:**

```elixir
@spec embed_texts_async(texts, config) :: {:ok, term()} | {:error, term()}
def embed_texts_async(texts, config)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `texts` | `list(String.t())` | Yes | The  texts |
| `config` | `EmbeddingConfig` | Yes | The embedding config |

**Returns:** `list(list(float()))`
**Errors:** Returns `{:error, reason}`

---

#### get_embedding_preset()

Get an embedding preset by name.

Returns `nil` if no preset with the given name exists. Returns an owned
clone so the value is safe to pass across FFI boundaries.

**Signature:**

```elixir
@spec get_embedding_preset(name) :: {:ok, term()} | {:error, term()}
def get_embedding_preset(name)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `name` | `String.t()` | Yes | The name |

**Returns:** `EmbeddingPreset | nil`

---

#### list_embedding_presets()

List the names of all available embedding presets.

Returns owned `String`s so the values are safe to pass across FFI boundaries.

**Signature:**

```elixir
@spec list_embedding_presets() :: {:ok, term()} | {:error, term()}
def list_embedding_presets()
```

**Returns:** `list(String.t())`

---

#### rerank()

Rerank a list of documents by relevance to a query.

Returns documents sorted descending by score. Applies `top_k` truncation if
configured.

**Errors:**

- `KreuzbergError.Validation` if `query` is empty or blank.
- `KreuzbergError.MissingDependency` if ONNX Runtime is not installed (ONNX path).
- `KreuzbergError.Reranking` if the preset is unknown or model download fails.

Since v5.0.0.

**Signature:**

```elixir
@spec rerank(query, documents, config) :: {:ok, term()} | {:error, term()}
def rerank(query, documents, config)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `query` | `String.t()` | Yes | The query |
| `documents` | `list(String.t())` | Yes | The documents |
| `config` | `RerankerConfig` | Yes | The configuration options |

**Returns:** `list(RerankedDocument)`
**Errors:** Returns `{:error, reason}`

---

#### rerank_async()

Stub for builds without the `reranker` feature.

Since v5.0.0.

**Signature:**

```elixir
@spec rerank_async(query, documents, config) :: {:ok, term()} | {:error, term()}
def rerank_async(query, documents, config)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `query` | `String.t()` | Yes | The  query |
| `documents` | `list(String.t())` | Yes | The  documents |
| `config` | `RerankerConfig` | Yes | The reranker config |

**Returns:** `list(RerankedDocument)`
**Errors:** Returns `{:error, reason}`

---

#### get_reranker_preset()

Get a reranker preset by name.

Returns `nil` if no preset with the given name exists. Returns an owned
clone so the value is safe to pass across FFI boundaries.

Since v5.0.0.

**Signature:**

```elixir
@spec get_reranker_preset(name) :: {:ok, term()} | {:error, term()}
def get_reranker_preset(name)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `name` | `String.t()` | Yes | The name |

**Returns:** `RerankerPreset | nil`

---

#### list_reranker_presets()

List the names of all available reranker presets.

Returns owned `String`s so the values are safe to pass across FFI boundaries.

Since v5.0.0.

**Signature:**

```elixir
@spec list_reranker_presets() :: {:ok, term()} | {:error, term()}
def list_reranker_presets()
```

**Returns:** `list(String.t())`

---

### Types

#### AccelerationConfig

Hardware acceleration configuration for ONNX Runtime models.

Controls which execution provider (CPU, CoreML, CUDA, TensorRT) is used
for inference in layout detection and embedding generation.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `provider` | `ExecutionProviderType` | `:auto` | Execution provider to use for ONNX inference. |
| `device_id` | `integer()` | — | GPU device ID (for CUDA/TensorRT). Ignored for CPU/CoreML/Auto. |

---

#### ArchiveEntry

A single file extracted from an archive.

When archives (ZIP, TAR, 7Z, GZIP) are extracted with recursive extraction
enabled, each processable file produces its own full `ExtractionResult`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `path` | `String.t()` | — | Archive-relative file path (e.g. "folder/document.pdf"). |
| `mime_type` | `String.t()` | — | Detected MIME type of the file. |
| `result` | `ExtractionResult` | — | Full extraction result for this file. |

---

#### ArchiveMetadata

Archive (ZIP/TAR/7Z) metadata.

Extracted from compressed archive files containing file lists and size information.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `format` | `String.t()` | — | Archive format ("ZIP", "TAR", "7Z", etc.) |
| `file_count` | `integer()` | — | Total number of files in the archive |
| `file_list` | `list(String.t())` | `[]` | List of file paths within the archive |
| `total_size` | `integer()` | — | Total uncompressed size in bytes |
| `compressed_size` | `integer() \| nil` | `nil` | Compressed size in bytes (if available) |

---

#### AudioMetadata

Audio/video file metadata.

Populated from container tags (ID3v2, MP4 atoms, Vorbis comments, etc.) and
PCM decode properties. Available when the `transcription-types` feature is enabled.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `duration_ms` | `integer() \| nil` | `nil` | Duration in milliseconds derived from the decoded audio stream. |
| `codec` | `String.t() \| nil` | `nil` | Audio codec (e.g. "mp3", "aac", "opus", "flac"). |
| `container` | `String.t() \| nil` | `nil` | Container format (e.g. "mpeg", "mp4", "ogg", "wav"). |
| `sample_rate_hz` | `integer() \| nil` | `nil` | Sample rate in Hz after decode (always 16000 when resampled for Whisper). |
| `channels` | `integer() \| nil` | `nil` | Number of audio channels (1 = mono, 2 = stereo). |
| `bitrate` | `integer() \| nil` | `nil` | Audio bitrate in kbps from the source file tags/properties. |

---

#### BBox

Bounding box in original image coordinates (x1, y1) top-left, (x2, y2) bottom-right.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `x1` | `float()` | — | Left edge (x-coordinate of the top-left corner). |
| `y1` | `float()` | — | Top edge (y-coordinate of the top-left corner). |
| `x2` | `float()` | — | Right edge (x-coordinate of the bottom-right corner). |
| `y2` | `float()` | — | Bottom edge (y-coordinate of the bottom-right corner). |

---

#### BatchBytesItem

Batch item for byte array extraction.

Used with `batch_extract_bytes` and `batch_extract_bytes_sync`
to represent a single item in a batch extraction job.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `binary()` | — | The content bytes to extract from |
| `mime_type` | `String.t()` | — | MIME type of the content (e.g., "application/pdf", "text/html") |
| `config` | `FileExtractionConfig \| nil` | `nil` | Per-item configuration overrides (None uses batch-level defaults) |

---

#### BatchFileItem

Batch item for file extraction.

Used with `batch_extract_files` and `batch_extract_files_sync`
to represent a single file in a batch extraction job.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `path` | `String.t()` | — | Path to the file to extract from |
| `config` | `FileExtractionConfig \| nil` | `nil` | Per-file configuration overrides (None uses batch-level defaults) |

---

#### BibtexMetadata

BibTeX bibliography metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `entry_count` | `integer()` | — | Number of entries in the bibliography. |
| `citation_keys` | `list(String.t())` | `[]` | BibTeX citation keys (e.g. `"knuth1984"`) for all entries. |
| `authors` | `list(String.t())` | `[]` | Author names collected across all bibliography entries. |
| `year_range` | `YearRange \| nil` | `nil` | Earliest and latest publication years found in the bibliography. |
| `entry_types` | `map() \| nil` | `%{}` | Count of entries grouped by BibTeX entry type (e.g. `"article"` → 5). |

---

#### BoundingBox

Bounding box coordinates for element positioning.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `x0` | `float()` | — | Left x-coordinate |
| `y0` | `float()` | — | Bottom y-coordinate |
| `x1` | `float()` | — | Right x-coordinate |
| `y1` | `float()` | — | Top y-coordinate |

---

#### CacheStats

Aggregate statistics for a kreuzberg cache directory.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `total_files` | `integer()` | — | Total number of files currently in the cache directory. |
| `total_size_mb` | `float()` | — | Combined size of all cache files in megabytes. |
| `available_space_mb` | `float()` | — | Free disk space available on the cache volume, in megabytes. |
| `oldest_file_age_days` | `float()` | — | Age of the oldest cache file in days (0.0 if the cache is empty). |
| `newest_file_age_days` | `float()` | — | Age of the most recently written cache file in days (0.0 if the cache is empty). |

---

#### CaptioningConfig

**Since:** `v5.0.0`

Configuration for the VLM captioning post-processor.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `llm` | `LlmConfig` | — | LLM configuration used for the VLM call. |
| `prompt` | `String.t() \| nil` | `nil` | Optional custom caption prompt. `nil` uses the default `RegionKind.Caption` prompt that ships with `crate.llm.region_extractor`. |
| `min_image_area` | `integer()` | `/* serde(default) */` | Skip images whose `width * height` is below this threshold (in pixels). Default `1_000` filters out icons and decorations. |

---

#### CellChange

A single changed cell within a table.

Defined here (rather than only in `crate.diff`) so `RevisionDelta` can
reference it unconditionally, without requiring the `diff` Cargo feature.
`crate.diff` re-exports this type verbatim.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `row` | `integer()` | — | Zero-based row index. |
| `col` | `integer()` | — | Zero-based column index. |
| `from` | `String.t()` | — | Value before the change. |
| `to` | `String.t()` | — | Value after the change. |

---

#### Chunk

A text chunk with optional embedding and metadata.

Chunks are created when chunking is enabled in `ExtractionConfig`. Each chunk
contains the text content, optional embedding vector (if embedding generation
is configured), and metadata about its position in the document.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String.t()` | — | The text content of this chunk. |
| `chunk_type` | `ChunkType` | `/* serde(default) */` | Semantic structural classification of this chunk. Assigned by the heuristic classifier based on content patterns and heading context. Defaults to `ChunkType.Unknown` when no rule matches. |
| `embedding` | `list(float()) \| nil` | `nil` | Optional embedding vector for this chunk. Only populated when `EmbeddingConfig` is provided in chunking configuration. The dimensionality depends on the chosen embedding model. |
| `metadata` | `ChunkMetadata` | — | Metadata about this chunk's position and properties. |

---

#### ChunkMetadata

Metadata about a chunk's position in the original document.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `byte_start` | `integer()` | — | Byte offset where this chunk starts in the original text (UTF-8 valid boundary). |
| `byte_end` | `integer()` | — | Byte offset where this chunk ends in the original text (UTF-8 valid boundary). |
| `token_count` | `integer() \| nil` | `nil` | Number of tokens in this chunk (if available). This is calculated by the embedding model's tokenizer if embeddings are enabled. |
| `chunk_index` | `integer()` | — | Zero-based index of this chunk in the document. |
| `total_chunks` | `integer()` | — | Total number of chunks in the document. |
| `first_page` | `integer() \| nil` | `nil` | First page number this chunk spans (1-indexed). Only populated when page tracking is enabled in extraction configuration. |
| `last_page` | `integer() \| nil` | `nil` | Last page number this chunk spans (1-indexed, equal to first_page for single-page chunks). Only populated when page tracking is enabled in extraction configuration. |
| `heading_context` | `HeadingContext \| nil` | `/* serde(default) */` | Heading context when using Markdown chunker. Contains the heading hierarchy this chunk falls under. Only populated when `ChunkerType.Markdown` is used. |
| `image_indices` | `list(integer())` | `/* serde(default) */` | Indices into `ExtractionResult.images` for images on pages covered by this chunk. Contains zero-based indices into the top-level `images` collection for every image whose `page_number` falls within `[first_page, last_page]`. Empty when image extraction is disabled or the chunk spans no pages with images. |

---

#### ChunkingConfig

Chunking configuration.

Configures text chunking for document content, including chunk size,
overlap, trimming behavior, and optional embeddings.

Use `..the default constructor` when constructing to allow for future field additions:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_characters` | `integer()` | `1000` | Maximum size per chunk (in units determined by `sizing`). When `sizing` is `Characters` (default), this is the max character count. When using token-based sizing, this is the max token count. Default: 1000 |
| `overlap` | `integer()` | `200` | Overlap between chunks (in units determined by `sizing`). Default: 200 |
| `trim` | `boolean()` | `true` | Whether to trim whitespace from chunk boundaries. Default: true |
| `chunker_type` | `ChunkerType` | `:text` | Type of chunker to use (Text or Markdown). Default: Text |
| `embedding` | `EmbeddingConfig \| nil` | `nil` | Optional embedding configuration for chunk embeddings. |
| `preset` | `String.t() \| nil` | `nil` | Use a preset configuration (overrides individual settings if provided). |
| `sizing` | `ChunkSizing` | `:characters` | How to measure chunk size. Default: `Characters` (Unicode character count). Enable `chunking-tiktoken` or `chunking-tokenizers` features for token-based sizing. |
| `prepend_heading_context` | `boolean()` | `false` | When `true` and `chunker_type` is `Markdown`, prepend the heading hierarchy path (e.g. `"# Title > ## Section\n\n"`) to each chunk's content string. This is useful for RAG pipelines where each chunk needs self-contained context about its position in the document structure. Default: `false` |
| `topic_threshold` | `float() \| nil` | `nil` | Optional cosine similarity threshold for semantic topic boundary detection. Only used when `chunker_type` is `Semantic` and an `EmbeddingConfig` is provided. You almost never need to set this. When omitted, defaults to `0.75` which works well for most documents. Lower values detect more topic boundaries (more, smaller chunks); higher values detect fewer. Range: `0.0..=1.0`. |

### Functions

#### default()

**Signature:**

```elixir
def default()
```

---

#### CitationMetadata

Citation file metadata (RIS, PubMed, EndNote).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `citation_count` | `integer()` | — | Total number of citation records in the file. |
| `format` | `String.t() \| nil` | `nil` | Detected citation file format (e.g. `"ris"`, `"pubmed"`, `"endnote"`). |
| `authors` | `list(String.t())` | `[]` | Author names collected across all citation records. |
| `year_range` | `YearRange \| nil` | `nil` | Earliest and latest publication years found in the file. |
| `dois` | `list(String.t())` | `[]` | DOI identifiers found in the citation records. |
| `keywords` | `list(String.t())` | `[]` | Keywords collected from all citation records. |

---

#### ClassificationLabel

A single label + confidence pair.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `label` | `String.t()` | — | Label name as configured in `PageClassificationConfig.labels`. |
| `confidence` | `float() \| nil` | `nil` | Backend-reported confidence in `[0.0, 1.0]`. `nil` when the backend (e.g. an LLM prompt without explicit confidence schema) did not report one. |

---

#### ContentFilterConfig

Cross-extractor content filtering configuration.

Controls whether "furniture" content (headers, footers, page numbers,
watermarks, repeating text) is included in or stripped from extraction
results. Applies across all extractors (PDF, DOCX, RTF, ODT, HTML, etc.)
with format-specific implementation.

When `nil` on `ExtractionConfig`, each extractor uses its current
default behavior unchanged.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `include_headers` | `boolean()` | `false` | Include running headers in extraction output. - PDF: Disables top-margin furniture stripping and prevents the layout model from treating `PageHeader`-classified regions as furniture. - DOCX: Includes document headers in text output. - RTF/ODT: Headers already included; this is a no-op when true. - HTML/EPUB: Keeps `<header>` element content. Default: `false` (headers are stripped or excluded). |
| `include_footers` | `boolean()` | `false` | Include running footers in extraction output. - PDF: Disables bottom-margin furniture stripping and prevents the layout model from treating `PageFooter`-classified regions as furniture. - DOCX: Includes document footers in text output. - RTF/ODT: Footers already included; this is a no-op when true. - HTML/EPUB: Keeps `<footer>` element content. Default: `false` (footers are stripped or excluded). |
| `strip_repeating_text` | `boolean()` | `true` | Enable the heuristic cross-page repeating text detector. When `true` (default), text that repeats verbatim across a supermajority of pages is classified as furniture and stripped.  Disable this if brand names or repeated headings are being incorrectly removed by the heuristic. Note: when a layout-detection model is active, the model may independently classify page-header / page-footer regions as furniture on a per-page basis. To preserve those regions, set `include_headers = true`, `include_footers = true`, or both, in addition to disabling this flag. Primarily affects PDF extraction. Default: `true`. |
| `include_watermarks` | `boolean()` | `false` | Include watermark text in extraction output. - PDF: Keeps watermark artifacts and arXiv identifiers. - Other formats: No effect currently. Default: `false` (watermarks are stripped). |

### Functions

#### default()

**Signature:**

```elixir
def default()
```

---

#### ContributorRole

JATS contributor with role.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String.t()` | — | Contributor display name. |
| `role` | `String.t() \| nil` | `nil` | Contributor role (e.g. `"author"`, `"editor"`). |

---

#### CoreProperties

Dublin Core metadata from docProps/core.xml

Contains standard metadata fields defined by the Dublin Core standard
and Office-specific extensions.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `title` | `String.t() \| nil` | `nil` | Document title |
| `subject` | `String.t() \| nil` | `nil` | Document subject/topic |
| `creator` | `String.t() \| nil` | `nil` | Document creator/author |
| `keywords` | `String.t() \| nil` | `nil` | Keywords or tags |
| `description` | `String.t() \| nil` | `nil` | Document description/abstract |
| `last_modified_by` | `String.t() \| nil` | `nil` | User who last modified the document |
| `revision` | `String.t() \| nil` | `nil` | Revision number |
| `created` | `String.t() \| nil` | `nil` | Creation timestamp (ISO 8601) |
| `modified` | `String.t() \| nil` | `nil` | Last modification timestamp (ISO 8601) |
| `category` | `String.t() \| nil` | `nil` | Document category |
| `content_status` | `String.t() \| nil` | `nil` | Content status (Draft, Final, etc.) |
| `language` | `String.t() \| nil` | `nil` | Document language |
| `identifier` | `String.t() \| nil` | `nil` | Unique identifier |
| `version` | `String.t() \| nil` | `nil` | Document version |
| `last_printed` | `String.t() \| nil` | `nil` | Last print timestamp (ISO 8601) |

---

#### CsvMetadata

CSV/TSV file metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `row_count` | `integer()` | — | Total number of data rows (excluding the header row if present). |
| `column_count` | `integer()` | — | Number of columns detected. |
| `delimiter` | `String.t() \| nil` | `nil` | Field delimiter character (e.g. `","` or `"\t"`). |
| `has_header` | `boolean()` | — | Whether the first row was treated as a header. |
| `column_types` | `list(String.t()) \| nil` | `[]` | Inferred data type for each column (e.g. `"string"`, `"integer"`, `"float"`). |

---

#### DbfFieldInfo

dBASE field information.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String.t()` | — | Field (column) name. |
| `field_type` | `String.t()` | — | dBASE field type character (e.g. `"C"` for character, `"N"` for numeric). |

---

#### DbfMetadata

dBASE (DBF) file metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `record_count` | `integer()` | — | Total number of data records in the DBF file. |
| `field_count` | `integer()` | — | Number of field (column) definitions. |
| `fields` | `list(DbfFieldInfo)` | `[]` | Descriptor for each field in the table schema. |

---

#### DetectResponse

MIME type detection response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `mime_type` | `String.t()` | — | Detected MIME type |
| `filename` | `String.t() \| nil` | `nil` | Original filename (if provided) |

---

#### DetectionResult

Page-level detection result containing all detections and page metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `page_width` | `integer()` | — | Page width in pixels (as seen by the model). |
| `page_height` | `integer()` | — | Page height in pixels (as seen by the model). |
| `detections` | `list(LayoutDetection)` | — | All layout detections on this page after postprocessing. |

---

#### DiffHunk

A single contiguous hunk in a unified diff.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `from_line` | `integer()` | — | Starting line number in the old content (0-indexed). |
| `from_count` | `integer()` | — | Number of lines from the old content in this hunk. |
| `to_line` | `integer()` | — | Starting line number in the new content (0-indexed). |
| `to_count` | `integer()` | — | Number of lines from the new content in this hunk. |
| `lines` | `list(DiffLine)` | — | Lines that make up this hunk. |

---

#### DiffOptions

Options controlling how two `ExtractionResult` values are compared.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `include_metadata` | `boolean()` | `true` | Include metadata changes in the diff. Default: `true`. |
| `include_embedded` | `boolean()` | `true` | Include embedded-children changes in the diff. Default: `true`. |
| `max_content_chars` | `integer() \| nil` | `nil` | Truncate content to this many characters before diffing. Useful for very large documents where only the first N characters matter. `nil` means no truncation. |

### Functions

#### default()

**Signature:**

```elixir
def default()
```

---

#### DjotContent

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
| `plain_text` | `String.t()` | — | Plain text representation for backwards compatibility |
| `blocks` | `list(FormattedBlock)` | — | Structured block-level content |
| `metadata` | `Metadata` | — | Metadata from YAML frontmatter |
| `tables` | `list(Table)` | — | Extracted tables as structured data |
| `images` | `list(DjotImage)` | — | Extracted images with metadata |
| `links` | `list(DjotLink)` | — | Extracted links with URLs |
| `footnotes` | `list(Footnote)` | — | Footnote definitions |
| `attributes` | `list(String.t())` | `/* serde(default) */` | Attributes mapped by element identifier (if present) |

---

#### DjotImage

Image element in Djot.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `src` | `String.t()` | — | Image source URL or path |
| `alt` | `String.t()` | — | Alternative text |
| `title` | `String.t() \| nil` | `nil` | Optional title |
| `attributes` | `String.t() \| nil` | `nil` | Element attributes |

---

#### DjotLink

Link element in Djot.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | `String.t()` | — | Link URL |
| `text` | `String.t()` | — | Link text content |
| `title` | `String.t() \| nil` | `nil` | Optional title |
| `attributes` | `String.t() \| nil` | `nil` | Element attributes |

---

#### DocumentExtractor

Trait for document extractor plugins.

Implement this trait to add support for new document formats or to override
built-in extraction behavior with custom logic.

### Return Type

Extractors return `InternalDocument`, a flat intermediate representation.
The pipeline converts this into the public `ExtractionResult` via the
derivation step.

### Priority System

When multiple extractors support the same MIME type, the registry selects
the extractor with the highest priority value. Use this to:

- Override built-in extractors (priority > 50)
- Provide fallback extractors (priority < 50)
- Implement specialized extractors for specific use cases

Default priority is 50.

### Thread Safety

Extractors must be thread-safe (`Send + Sync`) to support concurrent extraction.

### Functions

#### extract_bytes()

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

```elixir
def extract_bytes(content, mime_type, config)
```

#### extract_file()

Extract content from a file.

Default implementation reads the file and calls `extract_bytes`.
Override for custom file handling, streaming, or memory optimizations.

**Returns:**

An `InternalDocument` containing the extracted elements, metadata, and tables.

**Errors:**

Same as `extract_bytes`, plus file I/O errors.

**Signature:**

```elixir
def extract_file(path, mime_type, config)
```

#### supported_mime_types()

Get the list of MIME types supported by this extractor.

Can include exact MIME types and prefix patterns:

- Exact: `"application/pdf"`, `"text/plain"`
- Prefix: `"image/*"` (matches any image type)

**Returns:**

A slice of MIME type strings.

**Signature:**

```elixir
def supported_mime_types()
```

#### priority()

Get the priority of this extractor.

Higher priority extractors are preferred when multiple extractors
support the same MIME type.

### Priority Guidelines

- **0-25**: Fallback/low-quality extractors
- **26-49**: Alternative extractors
- **50**: Default priority (built-in extractors)
- **51-75**: Premium/enhanced extractors
- **76-100**: Specialized/high-priority extractors

**Returns:**

Priority value (default: 50)

**Signature:**

```elixir
def priority()
```

#### can_handle()

Optional: Check if this extractor can handle a specific file.

Allows for more sophisticated detection beyond MIME types.
Defaults to `true` (rely on MIME type matching).

**Returns:**

`true` if the extractor can handle this file, `false` otherwise.

**Signature:**

```elixir
def can_handle(path, mime_type)
```

---

#### DocumentNode

A single node in the document tree.

Each node has deterministic `id`, typed `content`, optional `parent`/`children`
for tree structure, and metadata like page number, bounding box, and content layer.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String.t()` | — | Deterministic identifier (hash of content + position). |
| `content` | `NodeContent` | — | Node content — tagged enum, type-specific data only. |
| `parent` | `integer() \| nil` | `nil` | Parent node index (`nil` = root-level node). |
| `children` | `list(integer())` | `/* serde(default) */` | Child node indices in reading order. |
| `content_layer` | `ContentLayer` | `/* serde(default) */` | Content layer classification. Always serialised — Kotlin-Android (and any other typed binding) treats the field as non-nullable, so omitting it from the JSON wire would break consumer deserialisation.  `#[serde(default)]` covers the missing-field case on inbound JSON. |
| `page` | `integer() \| nil` | `nil` | Page number where this node starts (1-indexed). |
| `page_end` | `integer() \| nil` | `nil` | Page number where this node ends (for multi-page tables/sections). |
| `bbox` | `BoundingBox \| nil` | `nil` | Bounding box in document coordinates. |
| `annotations` | `list(TextAnnotation)` | `/* serde(default) */` | Inline annotations (formatting, links) on this node's text content. Only meaningful for text-carrying nodes; empty for containers. |
| `attributes` | `map() \| nil` | `nil` | Format-specific key-value attributes. Extensible bag for miscellaneous data without a dedicated typed field: CSS classes, LaTeX environment names, Excel cell formulas, slide layout names, etc. |

---

#### DocumentRelationship

A resolved relationship between two nodes in the document tree.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `source` | `integer()` | — | Source node index (the referencing node). |
| `target` | `integer()` | — | Target node index (the referenced node). |
| `kind` | `RelationshipKind` | — | Semantic kind of the relationship. |

---

#### DocumentRevision

A single tracked change embedded in a document.

Populated by per-format extractors that understand change-tracking metadata
(DOCX `w:ins`/`w:del`/`w:rPrChange`, ODT `text:change-*`, …). Every
extractor defaults to `ExtractionResult.revisions = None` until a
format-specific implementation is added.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `revision_id` | `String.t()` | — | Format-specific revision identifier. For DOCX this is the `w:id` attribute value on the change element (e.g. `"42"`). When the attribute is absent a synthetic fallback is generated (`"docx-ins-0"`, `"docx-del-3"`, …). |
| `author` | `String.t() \| nil` | `nil` | Display name of the author who made this change, when available. |
| `timestamp` | `String.t() \| nil` | `nil` | ISO-8601 timestamp of the change, when available. Stored as a plain string so this type remains FFI-friendly and unconditionally available without the `chrono` optional dep. DOCX populates this from the `w:date` attribute (e.g. `"2024-03-15T10:30:00Z"`). |
| `kind` | `RevisionKind` | — | Semantic kind of this revision. |
| `anchor` | `RevisionAnchor \| nil` | `nil` | Best-effort document location for this revision. Resolution is format-dependent and may be `nil` when the location cannot be determined (e.g. changes inside table cells before table-cell anchor support is added). |
| `delta` | `RevisionDelta` | — | The content changes that make up this revision. |

---

#### DocumentStructure

Top-level structured document representation.

A flat array of nodes with index-based parent/child references forming a tree.
Root-level nodes have `parent: None`. Use `body_roots()` and `furniture_roots()`
to iterate over top-level content by layer.

### Validation

Call `validate()` after construction to verify all node indices are in bounds
and parent-child relationships are bidirectionally consistent.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `nodes` | `list(DocumentNode)` | `[]` | All nodes in document/reading order. |
| `source_format` | `String.t() \| nil` | `nil` | Origin format identifier (e.g. "docx", "pptx", "html", "pdf"). Allows renderers to apply format-aware heuristics when converting the document tree to output formats. |
| `relationships` | `list(DocumentRelationship)` | `[]` | Resolved relationships between nodes (footnote refs, citations, anchor links, etc.). Populated during derivation from the internal document representation. Empty when no relationships are detected. |
| `node_types` | `list(String.t())` | `[]` | Sorted, deduplicated list of node type names present in this document. Each value is the snake_case `node_type` tag of the corresponding `NodeContent` variant (e.g. `"paragraph"`, `"heading"`, `"table"`, …). Computed from `nodes` via `DocumentStructure.finalize_node_types`. Empty until that method is called (internal construction paths call it at the end of derivation). |

### Functions

#### finalize_node_types()

Compute and populate the `node_types` field from the current `nodes`.

Call this after all nodes have been added to the structure. Internal
construction paths (builder, derivation) call this automatically.

**Signature:**

```elixir
def finalize_node_types()
```

#### is_empty()

Check if the document structure is empty.

**Signature:**

```elixir
def is_empty()
```

#### default()

**Signature:**

```elixir
def default()
```

---

#### DocumentSummary

Summary of an extracted document.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text` | `String.t()` | — | Summary text (plain prose). |
| `strategy` | `SummaryStrategy` | — | Strategy that produced this summary. |
| `token_count` | `integer() \| nil` | `nil` | Approximate token count of the summary, when known. |

---

#### DocxAppProperties

Application properties from docProps/app.xml for DOCX

Contains Word-specific document statistics and metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `application` | `String.t() \| nil` | `nil` | Application name (e.g., "Microsoft Office Word") |
| `app_version` | `String.t() \| nil` | `nil` | Application version |
| `template` | `String.t() \| nil` | `nil` | Template filename |
| `total_time` | `integer() \| nil` | `nil` | Total editing time in minutes |
| `pages` | `integer() \| nil` | `nil` | Number of pages |
| `words` | `integer() \| nil` | `nil` | Number of words |
| `characters` | `integer() \| nil` | `nil` | Number of characters (excluding spaces) |
| `characters_with_spaces` | `integer() \| nil` | `nil` | Number of characters (including spaces) |
| `lines` | `integer() \| nil` | `nil` | Number of lines |
| `paragraphs` | `integer() \| nil` | `nil` | Number of paragraphs |
| `company` | `String.t() \| nil` | `nil` | Company name |
| `doc_security` | `integer() \| nil` | `nil` | Document security level |
| `scale_crop` | `boolean() \| nil` | `nil` | Scale crop flag |
| `links_up_to_date` | `boolean() \| nil` | `nil` | Links up to date flag |
| `shared_doc` | `boolean() \| nil` | `nil` | Shared document flag |
| `hyperlinks_changed` | `boolean() \| nil` | `nil` | Hyperlinks changed flag |

---

#### DocxMetadata

Word document metadata.

Extracted from DOCX files using shared Office Open XML metadata extraction.
Integrates with `office_metadata` module for core/app/custom properties.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `core_properties` | `CoreProperties \| nil` | `nil` | Core properties from docProps/core.xml (Dublin Core metadata) Contains title, creator, subject, keywords, dates, etc. Shared format across DOCX/PPTX/XLSX documents. |
| `app_properties` | `DocxAppProperties \| nil` | `nil` | Application properties from docProps/app.xml (Word-specific statistics) Contains word count, page count, paragraph count, editing time, etc. DOCX-specific variant of Office application properties. |
| `custom_properties` | `map() \| nil` | `%{}` | Custom properties from docProps/custom.xml (user-defined properties) Contains key-value pairs defined by users or applications. Values can be strings, numbers, booleans, or dates. |

---

#### Element

Semantic element extracted from document.

Represents a logical unit of content with semantic classification,
unique identifier, and metadata for tracking origin and position.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `element_id` | `String.t()` | — | Unique element identifier |
| `element_type` | `ElementType` | — | Semantic type of this element |
| `text` | `String.t()` | — | Text content of the element |
| `metadata` | `ElementMetadata` | — | Metadata about the element |

---

#### ElementMetadata

Metadata for a semantic element.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `page_number` | `integer() \| nil` | `nil` | Page number (1-indexed) |
| `filename` | `String.t() \| nil` | `nil` | Source filename or document name |
| `coordinates` | `BoundingBox \| nil` | `nil` | Bounding box coordinates if available |
| `element_index` | `integer() \| nil` | `nil` | Position index in the element sequence |
| `additional` | `map()` | — | Additional custom metadata |

---

#### EmailAttachment

Email attachment representation.

Contains metadata and optionally the content of an email attachment.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String.t() \| nil` | `nil` | Attachment name (from Content-Disposition header) |
| `filename` | `String.t() \| nil` | `nil` | Filename of the attachment |
| `mime_type` | `String.t() \| nil` | `nil` | MIME type of the attachment |
| `size` | `integer() \| nil` | `nil` | Size in bytes |
| `is_image` | `boolean()` | — | Whether this attachment is an image |
| `data` | `binary() \| nil` | `nil` | Attachment data (if extracted). Uses `bytes.Bytes` for cheap cloning of large buffers. |

---

#### EmailConfig

Configuration for email extraction.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `msg_fallback_codepage` | `integer() \| nil` | `nil` | Windows codepage number to use when an MSG file contains no codepage property. Defaults to `nil`, which falls back to windows-1252. If an unrecognized or invalid codepage number is supplied (including 0), the behavior silently falls back to windows-1252 — the same as when the MSG file itself contains an unrecognized codepage. No error or warning is emitted. Users should verify output when supplying unusual values. Common values: - 1250: Central European (Polish, Czech, Hungarian, etc.) - 1251: Cyrillic (Russian, Ukrainian, Bulgarian, etc.) - 1252: Western European (default) - 1253: Greek - 1254: Turkish - 1255: Hebrew - 1256: Arabic - 932:  Japanese (Shift-JIS) - 936:  Simplified Chinese (GBK) |

---

#### EmailExtractionResult

Email extraction result.

Complete representation of an extracted email message (.eml or .msg)
including headers, body content, and attachments.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `subject` | `String.t() \| nil` | `nil` | Email subject line |
| `from_email` | `String.t() \| nil` | `nil` | Sender email address |
| `to_emails` | `list(String.t())` | — | Primary recipient email addresses |
| `cc_emails` | `list(String.t())` | — | CC recipient email addresses |
| `bcc_emails` | `list(String.t())` | — | BCC recipient email addresses |
| `date` | `String.t() \| nil` | `nil` | Email date/timestamp |
| `message_id` | `String.t() \| nil` | `nil` | Message-ID header value |
| `plain_text` | `String.t() \| nil` | `nil` | Plain text version of the email body |
| `html_content` | `String.t() \| nil` | `nil` | HTML version of the email body |
| `content` | `String.t()` | — | Cleaned/processed text content. Aliased as `cleaned_text` for back-compat. |
| `attachments` | `list(EmailAttachment)` | — | List of email attachments |
| `metadata` | `map()` | — | Additional email headers and metadata |

---

#### EmailMetadata

Email metadata extracted from .eml and .msg files.

Includes sender/recipient information, message ID, and attachment list.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `from_email` | `String.t() \| nil` | `nil` | Sender's email address |
| `from_name` | `String.t() \| nil` | `nil` | Sender's display name |
| `to_emails` | `list(String.t())` | `[]` | Primary recipients |
| `cc_emails` | `list(String.t())` | `[]` | CC recipients |
| `bcc_emails` | `list(String.t())` | `[]` | BCC recipients |
| `message_id` | `String.t() \| nil` | `nil` | Message-ID header value |
| `attachments` | `list(String.t())` | `[]` | List of attachment filenames |

---

#### EmbeddedChanges

Changes to embedded archive children between two results.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `added` | `list(ArchiveEntry)` | — | Children present in `b` but not in `a` (matched by `path`). |
| `removed` | `list(ArchiveEntry)` | — | Children present in `a` but not in `b` (matched by `path`). |
| `changed` | `list(EmbeddedDiff)` | — | Children present in both but with differing content (matched by `path`). Each entry holds the diff of the nested `ExtractionResult`. |

---

#### EmbeddedDiff

Diff for a single embedded archive entry that appears in both results.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `path` | `String.t()` | — | Archive-relative path identifying this entry. |
| `diff` | `ExtractionDiff` | — | The recursive diff of the entry's extraction result. |

---

#### EmbeddedFile

Embedded file descriptor extracted from the PDF name tree.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String.t()` | — | The filename as stored in the PDF name tree. |
| `data` | `binary()` | — | Raw file bytes from the embedded stream (already decompressed by lopdf). |
| `compressed_size` | `integer()` | — | Compressed byte count of the original stream (before decompression). Used by callers to compute the decompression ratio and detect zip-bomb-style attacks that embed a tiny compressed stream expanding to gigabytes of data. |
| `mime_type` | `String.t() \| nil` | `nil` | MIME type if specified in the filespec, otherwise `nil`. |

---

#### EmbeddingBackend

Trait for in-process embedding backend plugins.

Async to match the convention used by `OcrBackend`,
`DocumentExtractor`, and `PostProcessor`.
Host-language bridges (PyO3, napi-rs, Rustler, extendr, magnus, ext-php-rs,
C FFI, etc.) wrap their synchronous host callables in `spawn_blocking` or the
equivalent to satisfy the async signature.

### Thread safety

Backends must be `Send + Sync + 'static`. They are stored in
`Arc<dyn EmbeddingBackend>` and called concurrently from kreuzberg's chunking
pipeline. If the backend's underlying model isn't thread-safe, the backend
itself must serialize access internally (e.g. via `Mutex<Inner>`).

### Contract

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

### Runtime

The synchronous `embed_texts` entry uses
`tokio.task.block_in_place` to await the trait's async `embed`, which
requires a multi-thread tokio runtime. Callers running inside a
`current_thread` runtime (e.g. `#[tokio.test]` without `flavor = "multi_thread"`,
or `tokio.runtime.Builder.new_current_thread()`) must use
`embed_texts_async` instead, which awaits directly without `block_in_place`.

### Functions

#### dimensions()

Embedding vector dimension. Must be `> 0` and must match the length of
every vector returned by `embed`.

**Signature:**

```elixir
def dimensions()
```

#### embed()

Embed a batch of texts, returning one vector per input in order.

**Errors:**

Implementations should return `Plugin` for
backend-specific failures. The dispatcher layers its own validation
(length, per-vector dimension) on top.

**Signature:**

```elixir
def embed(texts)
```

---

#### EmbeddingConfig

Embedding configuration for text chunks.

Configures embedding generation using ONNX models via the vendored embedding engine.
Requires the `embeddings` feature to be enabled.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `EmbeddingModelType` | `:preset` | The embedding model to use (defaults to "balanced" preset if not specified) |
| `normalize` | `boolean()` | `true` | Whether to normalize embedding vectors (recommended for cosine similarity) |
| `batch_size` | `integer()` | `32` | Batch size for embedding generation |
| `show_download_progress` | `boolean()` | `false` | Show model download progress |
| `cache_dir` | `String.t() \| nil` | `nil` | Custom cache directory for model files Defaults to `~/.cache/kreuzberg/embeddings/` if not specified. Allows full customization of model download location. |
| `acceleration` | `AccelerationConfig \| nil` | `nil` | Hardware acceleration for the embedding ONNX model. When set, controls which execution provider (CPU, CUDA, CoreML, TensorRT) is used for inference. Defaults to `nil` (auto-select per platform). |
| `max_embed_duration_secs` | `integer() \| nil` | `nil` | Maximum wall-clock duration (in seconds) for a single `embed()` call when using `EmbeddingModelType.Plugin`. Applies only to the in-process plugin path — protects against hung host-language backends (e.g. a Python callback deadlocked on the GIL, a model stuck on CUDA OOM retries, etc.). On timeout, the dispatcher returns `Plugin` instead of blocking forever. `nil` disables the timeout. The default (60 seconds) is conservative for common in-process inference; increase for large batches on slow hardware. |

### Functions

#### default()

**Signature:**

```elixir
def default()
```

---

#### EmbeddingPreset

Preset configurations for common RAG use cases.

Each preset combines chunk size, overlap, and embedding model
to provide an optimized configuration for specific scenarios.

All string fields are owned `String` for FFI compatibility — instances
are safe to clone and pass across language boundaries.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String.t()` | — | Short identifier for this preset (e.g. `"balanced"`, `"fast"`, `"quality"`). |
| `chunk_size` | `integer()` | — | Target chunk size in characters. |
| `overlap` | `integer()` | — | Overlap between consecutive chunks in characters. |
| `model_repo` | `String.t()` | — | HuggingFace repository name for the model. |
| `pooling` | `String.t()` | — | Pooling strategy: "cls" or "mean". |
| `model_file` | `String.t()` | — | Path to the ONNX model file within the repo. |
| `dimensions` | `integer()` | — | Embedding vector dimension produced by this model. |
| `description` | `String.t()` | — | Human-readable description of the preset's intended use case. |

---

#### Entity

A single named entity detected in the extracted text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `category` | `EntityCategory` | — | Canonical category the entity belongs to (PERSON, ORG, LOCATION, etc.). |
| `text` | `String.t()` | — | Raw mention text exactly as it appeared in the source. |
| `start` | `integer()` | — | Byte-offset span in `ExtractionResult.content` where the mention starts. |
| `end` | `integer()` | — | Byte-offset span in `ExtractionResult.content` where the mention ends (exclusive). |
| `confidence` | `float() \| nil` | `nil` | Backend-reported confidence in `[0.0, 1.0]`. `nil` when the backend does not expose confidence scores. |

---

#### EpubMetadata

EPUB metadata (Dublin Core extensions).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `coverage` | `String.t() \| nil` | `nil` | Dublin Core `coverage` field (geographic or temporal scope). |
| `dc_format` | `String.t() \| nil` | `nil` | Dublin Core `format` field (media type of the resource). |
| `relation` | `String.t() \| nil` | `nil` | Dublin Core `relation` field (related resource identifier). |
| `source` | `String.t() \| nil` | `nil` | Dublin Core `source` field (origin resource identifier). |
| `dc_type` | `String.t() \| nil` | `nil` | Dublin Core `type` field (nature or genre of the resource). |
| `cover_image` | `String.t() \| nil` | `nil` | Path or identifier of the cover image within the EPUB container. |

---

#### ErrorMetadata

Error metadata (for batch operations).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `error_type` | `String.t()` | — | Machine-readable error type identifier (e.g. "UnsupportedFormat"). |
| `message` | `String.t()` | — | Human-readable error description. |

---

#### ExcelMetadata

Excel/spreadsheet format metadata.

Identifies the document as a spreadsheet source via the `FormatMetadata.Excel`
discriminant. Sheet count and sheet names are stored inside this struct.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `sheet_count` | `integer() \| nil` | `nil` | Number of sheets in the workbook. |
| `sheet_names` | `list(String.t()) \| nil` | `[]` | Names of all sheets in the workbook. |

---

#### ExcelSheet

Single Excel worksheet.

Represents one sheet from an Excel workbook with its content
converted to Markdown format and dimensional statistics.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String.t()` | — | Sheet name as it appears in Excel |
| `markdown` | `String.t()` | — | Sheet content converted to Markdown tables |
| `row_count` | `integer()` | — | Number of rows |
| `col_count` | `integer()` | — | Number of columns |
| `cell_count` | `integer()` | — | Total number of non-empty cells |
| `table_cells` | `list(list(String.t())) \| nil` | `nil` | Pre-extracted table cells (2D vector of cell values) Populated during markdown generation to avoid re-parsing markdown. None for empty sheets. |

---

#### ExcelWorkbook

Excel workbook representation.

Contains all sheets from an Excel file (.xlsx, .xls, etc.) with
extracted content and metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `sheets` | `list(ExcelSheet)` | — | All sheets in the workbook |
| `metadata` | `map()` | — | Workbook-level metadata (author, creation date, etc.) |
| `revisions` | `list(DocumentRevision) \| nil` | `/* serde(default) */` | Collaborative-edit revision headers from `xl/revisions/revisionHeaders.xml`. Populated for legacy shared-workbook `.xlsx` files that contain the `xl/revisions/` directory. Each `<header>` element maps to one `DocumentRevision { kind: FormatChange }` carrying the header's `guid` (→ `revision_id`), `userName` (→ `author`), and `dateTime` (→ `timestamp`). `anchor` and `delta` are `nil`/empty for v1 (per-cell log parsing is a follow-up). `nil` when `xl/revisions/revisionHeaders.xml` is absent. |

---

#### ExtractedImage

Extracted image from a document.

Contains raw image data, metadata, and optional nested OCR results.
Raw bytes allow cross-language compatibility - users can convert to
PIL.Image (Python), Sharp (Node.js), or other formats as needed.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `data` | `binary()` | — | Raw image data (PNG, JPEG, WebP, etc. bytes). Uses `bytes.Bytes` for cheap cloning of large buffers. |
| `format` | `String.t()` | — | Image format (e.g., "jpeg", "png", "webp") Uses Cow<'static, str> to avoid allocation for static literals. |
| `image_index` | `integer()` | — | Zero-indexed position of this image in the document/page |
| `page_number` | `integer() \| nil` | `nil` | Page/slide number where image was found (1-indexed) |
| `width` | `integer() \| nil` | `nil` | Image width in pixels |
| `height` | `integer() \| nil` | `nil` | Image height in pixels |
| `colorspace` | `String.t() \| nil` | `nil` | Colorspace information (e.g., "RGB", "CMYK", "Gray") |
| `bits_per_component` | `integer() \| nil` | `nil` | Bits per color component (e.g., 8, 16) |
| `is_mask` | `boolean()` | — | Whether this image is a mask image |
| `description` | `String.t() \| nil` | `nil` | Optional description of the image |
| `ocr_result` | `ExtractionResult \| nil` | `nil` | Nested OCR extraction result (if image was OCRed) When OCR is performed on this image, the result is embedded here rather than in a separate collection, making the relationship explicit. |
| `bounding_box` | `BoundingBox \| nil` | `nil` | Bounding box of the image on the page (PDF coordinates: x0=left, y0=bottom, x1=right, y1=top). Only populated for PDF-extracted images when position data is available from the PDF extractor. |
| `source_path` | `String.t() \| nil` | `nil` | Original source path of the image within the document archive (e.g., "media/image1.png" in DOCX). Used for rendering image references when the binary data is not extracted. |
| `image_kind` | `ImageKind \| nil` | `nil` | Heuristic classification of what this image likely depicts. `nil` if classification was disabled or inconclusive. |
| `kind_confidence` | `float() \| nil` | `nil` | Confidence score for `image_kind`, in the range 0.0 to 1.0. |
| `cluster_id` | `integer() \| nil` | `nil` | Identifier shared across images that form a single logical figure (e.g. all raster tiles of one technical drawing). `nil` for singletons. |
| `caption` | `String.t() \| nil` | `nil` | VLM-generated caption describing the image, when captioning is configured. Populated by the captioning post-processor (`crates/kreuzberg/src/plugins/processor/builtin/captioning.rs`), which routes each image through `crate.llm.region_extractor.extract_region_with_vlm` in caption mode. `nil` when captioning is disabled or the VLM declined to caption. |
| `qr_codes` | `list(QrCode) \| nil` | `[]` | QR codes decoded from this image, when QR detection is enabled. Populated by the QR post-processor (`crates/kreuzberg/src/extractors/qr.rs`) via the pure-Rust `rqrr` decoder. `nil` when QR detection is disabled; an empty `Some(vec![])` when detection ran but found nothing. |

---

#### ExtractedUri

A URI extracted from a document.

Represents any link, reference, or resource pointer found during extraction.
The `kind` field classifies the URI semantically, while `label` carries
optional human-readable display text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | `String.t()` | — | The URL or path string. |
| `label` | `String.t() \| nil` | `nil` | Optional display text / label for the link. |
| `page` | `integer() \| nil` | `nil` | Optional page number where the URI was found (1-indexed). |
| `kind` | `UriKind` | — | Semantic classification of the URI. |

---

#### ExtractionConfig

Main extraction configuration.

This struct contains all configuration options for the extraction process.
It can be loaded from TOML, YAML, or JSON files, or created programmatically.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `use_cache` | `boolean()` | `true` | Enable caching of extraction results |
| `enable_quality_processing` | `boolean()` | `true` | Enable quality post-processing |
| `ocr` | `OcrConfig \| nil` | `nil` | OCR configuration (None = OCR disabled) |
| `force_ocr` | `boolean()` | `false` | Force OCR even for searchable PDFs |
| `force_ocr_pages` | `list(integer()) \| nil` | `nil` | Force OCR on specific pages only (1-indexed page numbers, must be >= 1). When set, only the listed pages are OCR'd regardless of text layer quality. Unlisted pages use native text extraction. Ignored when `force_ocr` is `true`. Only applies to PDF documents. Duplicates are automatically deduplicated. An `ocr` config is recommended for backend/language selection; defaults are used if absent. |
| `disable_ocr` | `boolean()` | `false` | Disable OCR entirely, even for images. When `true`, OCR is skipped for all document types. Images return metadata only (dimensions, format, EXIF) without text extraction. PDFs use only native text extraction without OCR fallback. Cannot be `true` simultaneously with `force_ocr`. *Added in v4.7.0.* |
| `chunking` | `ChunkingConfig \| nil` | `nil` | Text chunking configuration (None = chunking disabled) |
| `content_filter` | `ContentFilterConfig \| nil` | `nil` | Content filtering configuration (None = use extractor defaults). Controls whether document "furniture" (headers, footers, watermarks, repeating text) is included in or stripped from extraction results. See `ContentFilterConfig` for per-field documentation. |
| `images` | `ImageExtractionConfig \| nil` | `nil` | Image extraction configuration (None = no image extraction) |
| `pdf_options` | `PdfConfig \| nil` | `nil` | PDF-specific options (None = use defaults) |
| `token_reduction` | `TokenReductionOptions \| nil` | `nil` | Token reduction configuration (None = no token reduction) |
| `language_detection` | `LanguageDetectionConfig \| nil` | `nil` | Language detection configuration (None = no language detection) |
| `pages` | `PageConfig \| nil` | `nil` | Page extraction configuration (None = no page tracking) |
| `keywords` | `KeywordConfig \| nil` | `nil` | Keyword extraction configuration (None = no keyword extraction) |
| `postprocessor` | `PostProcessorConfig \| nil` | `nil` | Post-processor configuration (None = use defaults) |
| `html_options` | `String.t() \| nil` | `nil` | HTML to Markdown conversion options (None = use defaults) Configure how HTML documents are converted to Markdown, including heading styles, list formatting, code block styles, and preprocessing options. |
| `html_output` | `HtmlOutputConfig \| nil` | `nil` | Styled HTML output configuration. When set alongside `output_format = OutputFormat.Html`, the extraction pipeline uses `StyledHtmlRenderer` which emits stable `kb-*` CSS class hooks on every structural element and optionally embeds theme CSS or user-supplied CSS in a `<style>` block. When `nil`, the existing plain comrak-based HTML renderer is used. |
| `extraction_timeout_secs` | `integer() \| nil` | `nil` | Default per-file timeout in seconds for batch extraction. When set, each file in a batch will be canceled after this duration unless overridden by `FileExtractionConfig.timeout_secs`. Defaults to `Some(60)` to prevent pathological files (e.g. deeply nested archives, documents with millions of cells) from running indefinitely and exhausting caller resources. Set to `nil` to disable the timeout for trusted input or long-running workloads. |
| `max_concurrent_extractions` | `integer() \| nil` | `nil` | Maximum concurrent extractions in batch operations (None = (num_cpus × 1.5).ceil()). Limits parallelism to prevent resource exhaustion when processing large batches. Defaults to (num_cpus × 1.5).ceil() when not set. |
| `result_format` | `ResultFormat` | `:unified` | Result structure format Controls whether results are returned in unified format (default) with all content in the `content` field, or element-based format with semantic elements (for Unstructured-compatible output). |
| `security_limits` | `SecurityLimits \| nil` | `nil` | Security limits for archive extraction. Controls maximum archive size, compression ratio, file count, and other security thresholds to prevent decompression bomb attacks. Also caps nesting depth, iteration count, entity / token length, total content size, and table cell count for every extraction path that ingests user-controlled bytes. When `nil`, default limits are used. |
| `max_embedded_file_bytes` | `integer() \| nil` | `nil` | Maximum uncompressed size in bytes for a single embedded file before recursive extraction is attempted (default: 50 MiB). Applies to embedded objects inside OOXML containers (DOCX, PPTX) and to email attachments processed via recursive extraction. Files that exceed this limit are skipped with a `ProcessingWarning` rather than passed to the extraction pipeline, preventing a single oversized embedded object from consuming unbounded memory or time. Set to `nil` to disable the per-embedded-file cap (falls back to `security_limits.max_archive_size` as the only guard). |
| `output_format` | `OutputFormat` | `:plain` | Content text format (default: Plain). Controls the format of the extracted content: - `Plain`: Raw extracted text (default) - `Markdown`: Markdown formatted output - `Djot`: Djot markup format (requires djot feature) - `Html`: HTML formatted output When set to a structured format, extraction results will include formatted output. The `formatted_content` field may be populated when format conversion is applied. |
| `layout` | `LayoutDetectionConfig \| nil` | `nil` | Layout detection configuration (None = layout detection disabled). When set, PDF pages and images are analyzed for document structure (headings, code, formulas, tables, figures, etc.) using RT-DETR models via ONNX Runtime. For PDFs, layout hints override paragraph classification in the markdown pipeline. For images, per-region OCR is performed with markdown formatting based on detected layout classes. Requires the `layout-detection` feature to run inference; the field is present whenever the `layout-types` feature is active (which includes `layout-detection` as well as the no-ORT target groups). |
| `use_layout_for_markdown` | `boolean()` | `false` | Run layout detection on the non-OCR PDF markdown path. When `true` and `layout` is `Some(_)`, layout regions inform heading, table, list, and figure detection in the structure pipeline that would otherwise rely on font-clustering heuristics alone. Significantly improves SF1 (structural F1) at the cost of inference latency (~150-300ms/page CPU, ~20-50ms/page GPU). Default: `false`. Requires the `layout-detection` feature. |
| `include_document_structure` | `boolean()` | `false` | Enable structured document tree output. When true, populates the `document` field on `ExtractionResult` with a hierarchical `DocumentStructure` containing heading-driven section nesting, table grids, content layer classification, and inline annotations. Independent of `result_format` — can be combined with Unified or ElementBased. |
| `acceleration` | `AccelerationConfig \| nil` | `nil` | Hardware acceleration configuration for ONNX Runtime models. Controls execution provider selection for layout detection and embedding models. When `nil`, uses platform defaults (CoreML on macOS, CUDA on Linux, CPU on Windows). |
| `cache_namespace` | `String.t() \| nil` | `nil` | Cache namespace for tenant isolation. When set, cache entries are stored under `{cache_dir}/{namespace}/`. Must be alphanumeric, hyphens, or underscores only (max 64 chars). Different namespaces have isolated cache spaces on the same filesystem. |
| `cache_ttl_secs` | `integer() \| nil` | `nil` | Per-request cache TTL in seconds. Overrides the global `max_age_days` for this specific extraction. When `0`, caching is completely skipped (no read or write). When `nil`, the global TTL applies. |
| `email` | `EmailConfig \| nil` | `nil` | Email extraction configuration (None = use defaults). Currently supports configuring the fallback codepage for MSG files that do not specify one. See `EmailConfig` for details. |
| `concurrency` | `String.t() \| nil` | `nil` | Concurrency limits for constrained environments (None = use defaults). Controls Rayon thread pool size, ONNX Runtime intra-op threads, and (when `max_concurrent_extractions` is unset) the batch concurrency semaphore. See `ConcurrencyConfig` for details. |
| `max_archive_depth` | `integer()` | — | Maximum recursion depth for archive extraction (default: 3). Set to 0 to disable recursive extraction (legacy behavior). |
| `tree_sitter` | `TreeSitterConfig \| nil` | `nil` | Tree-sitter language pack configuration (None = tree-sitter disabled). When set, enables code file extraction using tree-sitter parsers. Controls grammar download behavior and code analysis options. |
| `structured_extraction` | `StructuredExtractionConfig \| nil` | `nil` | Structured extraction via LLM (None = disabled). When set, the extracted document content is sent to an LLM with the provided JSON schema. The structured response is stored in `ExtractionResult.structured_output`. |
| `ner` | `NerConfig \| nil` | `nil` | Named-entity recognition configuration. When set, the NER post-processor runs at the Middle stage and populates `ExtractionResult.entities`. |
| `redaction` | `RedactionConfig \| nil` | `nil` | Redaction / anonymisation configuration. When set, the redaction post-processor runs at the Late stage and rewrites every textual field in `ExtractionResult`, emitting an audit trail in `ExtractionResult.redaction_report`. |
| `summarization` | `SummarizationConfig \| nil` | `nil` | Summarisation configuration. When set, the summarisation post-processor runs at the Middle stage and populates `ExtractionResult.summary`. |
| `translation` | `TranslationConfig \| nil` | `nil` | Translation configuration. When set, the translation post-processor runs at the Middle stage and populates `ExtractionResult.translation`. |
| `page_classification` | `PageClassificationConfig \| nil` | `nil` | Per-page classification configuration. When set, the classification post-processor runs at the Middle stage and populates `ExtractionResult.page_classifications`. |
| `captioning` | `CaptioningConfig \| nil` | `nil` | VLM captioning configuration for extracted images. When set, the captioning post-processor runs at the Middle stage and writes a caption into each `ExtractedImage.caption`. |
| `qr_codes` | `boolean() \| nil` | `nil` | Enable QR-code detection in extracted images. When `true`, the QR post-processor runs at the Middle stage and populates `ExtractedImage.qr_codes`. |
| `cancel_token` | `String.t() \| nil` | `nil` | Cancellation token for this extraction (None = no external cancellation). Pass a `CancellationToken` clone here and call its `cancel()` from another thread / task to abort the extraction in progress. The extractor checks the token at safe checkpoints (before lock acquisition, between pages, between batch items) and returns `Cancelled` when set. The field is excluded from serialization because `CancellationToken` is a runtime handle, not a configuration value. |

### Functions

#### default()

**Signature:**

```elixir
def default()
```

#### needs_image_data()

Check if image processing is needed by examining OCR and image extraction settings.

Returns `true` if either OCR is enabled or image extraction is configured,
indicating that image decompression and processing should occur.
Returns `false` if both are disabled, allowing optimization to skip unnecessary
image decompression for text-only extraction workflows.

### Optimization Impact
For text-only extractions (no OCR, no image extraction), skipping image
decompression can improve CPU utilization by 5-10% by avoiding wasteful
image I/O and processing when results won't be used.
Returns `true` when image binary data should be extracted.

True when `config.images.extract_images` is set **or** when captioning is
configured — captioning requires image bytes regardless of whether the caller
also requested `images` extraction.

**Signature:**

```elixir
def needs_image_data()
```

#### needs_image_processing()

Returns `true` when any image processing is needed during extraction.

### Optimization Impact

For text-only extractions (no OCR, no image extraction, no captioning), skipping
image decompression can improve CPU utilization by 5-10% by avoiding wasteful
image I/O and processing when results won't be used.

**Signature:**

```elixir
def needs_image_processing()
```

---

#### ExtractionDiff

The complete diff between two `ExtractionResult` values.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content_diff` | `list(DiffHunk)` | — | Unified-diff hunks for the `content` field. Empty when the content is identical. |
| `tables_added` | `list(Table)` | — | Tables present in `b` but not in `a` (by index position, excess right-side tables). |
| `tables_removed` | `list(Table)` | — | Tables present in `a` but not in `b` (by index position, excess left-side tables). |
| `tables_changed` | `list(TableDiff)` | — | Cell-level changes for table pairs that share the same index and dimensions. |
| `metadata_changed` | `term()` | — | Metadata difference, encoded as a JSON object with three top-level keys: `added` (keys present in `b` but not `a`), `removed` (keys present in `a` but not `b`), and `changed` (keys whose values differ — each entry is `{ "from": <value-in-a>, "to": <value-in-b> }`). This is NOT RFC 6902 JSON Patch — we deliberately chose a flatter shape to avoid pulling in a json-patch crate. If you need RFC 6902 semantics (with JSON Pointer paths) feed `a.metadata` and `b.metadata` to your preferred json-patch impl directly. |
| `embedded_changes` | `EmbeddedChanges` | — | Changes to embedded archive children. |

---

#### ExtractionResult

General extraction result used by the core extraction API.

This is the main result type returned by all extraction functions.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String.t()` | — | Plain-text representation of the extracted document content. |
| `mime_type` | `String.t()` | — | MIME type of the source document (e.g. `"application/pdf"`). |
| `metadata` | `Metadata` | — | Document-level metadata (author, title, dates, format-specific fields). |
| `extraction_method` | `ExtractionMethod \| nil` | `nil` | Extraction strategy used to produce the returned text. Populated when the extractor can reliably distinguish native text extraction, OCR-only extraction, or mixed native/OCR output. |
| `tables` | `list(Table)` | `[]` | Tables extracted from the document, each with structured cell data. |
| `detected_languages` | `list(String.t()) \| nil` | `[]` | ISO 639-1 language codes detected in the document content. |
| `chunks` | `list(Chunk) \| nil` | `[]` | Text chunks when chunking is enabled. When chunking configuration is provided, the content is split into overlapping chunks for efficient processing. Each chunk contains the text, optional embeddings (if enabled), and metadata about its position. |
| `images` | `list(ExtractedImage) \| nil` | `[]` | Extracted images from the document. When image extraction is enabled via `ImageExtractionConfig`, this field contains all images found in the document with their raw data and metadata. Each image may optionally contain a nested `ocr_result` if OCR was performed. |
| `pages` | `list(PageContent) \| nil` | `[]` | Per-page content when page extraction is enabled. When page extraction is configured, the document is split into per-page content with tables and images mapped to their respective pages. |
| `elements` | `list(Element) \| nil` | `[]` | Semantic elements when element-based result format is enabled. When result_format is set to ElementBased, this field contains semantic elements with type classification, unique identifiers, and metadata for Unstructured-compatible element-based processing. |
| `djot_content` | `DjotContent \| nil` | `nil` | Rich Djot content structure (when extracting Djot documents). When extracting Djot documents with structured extraction enabled, this field contains the full semantic structure including: - Block-level elements with nesting - Inline formatting with attributes - Links, images, footnotes - Math expressions - Complete attribute information The `content` field still contains plain text for backward compatibility. Always `nil` for non-Djot documents. |
| `ocr_elements` | `list(OcrElement) \| nil` | `[]` | OCR elements with full spatial and confidence metadata. When OCR is performed with element extraction enabled, this field contains the structured representation of detected text including: - Bounding geometry (rectangles or quadrilaterals) - Confidence scores (detection and recognition) - Rotation information - Hierarchical relationships (Tesseract only) This field preserves all metadata that would otherwise be lost when converting to plain text or markdown output formats. Only populated when `OcrElementConfig.include_elements` is true. |
| `document` | `DocumentStructure \| nil` | `nil` | Structured document tree (when document structure extraction is enabled). When `include_document_structure` is true in `ExtractionConfig`, this field contains the full hierarchical representation of the document including: - Heading-driven section nesting - Table grids with cell-level metadata - Content layer classification (body, header, footer, footnote) - Inline text annotations (formatting, links) - Bounding boxes and page numbers Independent of `result_format` — can be combined with Unified or ElementBased. |
| `extracted_keywords` | `list(Keyword) \| nil` | `[]` | Extracted keywords when keyword extraction is enabled. When keyword extraction (RAKE or YAKE) is configured, this field contains the extracted keywords with scores, algorithm info, and position data. Previously stored in `metadata.additional["keywords"]`. |
| `quality_score` | `float() \| nil` | `nil` | Document quality score from quality analysis. A value between 0.0 and 1.0 indicating the overall text quality. Previously stored in `metadata.additional["quality_score"]`. |
| `processing_warnings` | `list(ProcessingWarning)` | `[]` | Non-fatal warnings collected during processing pipeline stages. Captures errors from optional pipeline features (embedding, chunking, language detection, output formatting) that don't prevent extraction but may indicate degraded results. Previously stored as individual keys in `metadata.additional`. |
| `annotations` | `list(PdfAnnotation) \| nil` | `[]` | PDF annotations extracted from the document. When annotation extraction is enabled via `PdfConfig.extract_annotations`, this field contains text notes, highlights, links, stamps, and other annotations found in PDF documents. |
| `children` | `list(ArchiveEntry) \| nil` | `[]` | Nested extraction results from archive contents. When extracting archives, each processable file inside produces its own full extraction result. Set to `nil` for non-archive formats. Use `max_archive_depth` in config to control recursion depth. |
| `uris` | `list(ExtractedUri) \| nil` | `[]` | URIs/links discovered during document extraction. Contains hyperlinks, image references, citations, email addresses, and other URI-like references found in the document. Always extracted when present in the source document. |
| `revisions` | `list(DocumentRevision) \| nil` | `[]` | Tracked changes embedded in the source document. Populated by per-format extractors that understand change-tracking metadata (DOCX `w:ins`/`w:del`/`w:rPrChange`, ODT `text:change-*`, …). Every extractor defaults to `nil` until its format-specific implementation is added. Extractors that do populate this field follow the "accepted-changes" convention: inserted text is present in `content`, deleted text is absent — the revision list is the separate audit trail. |
| `structured_output` | `term() \| nil` | `nil` | Structured extraction output from LLM-based JSON schema extraction. When `structured_extraction` is configured in `ExtractionConfig`, the extracted document content is sent to a VLM with the provided JSON schema. The response is parsed and stored here as a JSON value matching the schema. |
| `code_intelligence` | `term() \| nil` | `nil` | Code intelligence results from tree-sitter analysis. Populated when extracting source code files with the `tree-sitter` feature. Contains metrics, structural analysis, imports/exports, comments, docstrings, symbols, diagnostics, and optionally chunked code segments. Stored as an opaque JSON value so that all language bindings (Go, Java, C#, …) can deserialize it as a raw JSON object rather than a typed struct. The underlying type is `tree_sitter_language_pack.ProcessResult`. |
| `llm_usage` | `list(LlmUsage) \| nil` | `[]` | LLM token usage and cost data for all LLM calls made during this extraction. Contains one entry per LLM call. Multiple entries are produced when VLM OCR, structured extraction, or LLM embeddings run during the same extraction. `nil` when no LLM was used. |
| `entities` | `list(Entity) \| nil` | `[]` | Named entities detected in `content` by the NER post-processor. `nil` when no NER backend is configured. Populated by the gline-rs ONNX backend or the LLM-driven backend (see `crates/kreuzberg/src/text/ner/`). |
| `summary` | `DocumentSummary \| nil` | `nil` | Summary of `content` produced by the summarisation post-processor. `nil` when summarisation is not configured. Populated by the TextRank extractive backend (deterministic, no external service) or by the liter-llm-driven abstractive backend. |
| `translation` | `Translation \| nil` | `nil` | Translation of `content` produced by the translation post-processor. `nil` when translation is not configured. |
| `page_classifications` | `list(PageClassification) \| nil` | `[]` | Per-page classifications produced by the page-classification post-processor. `nil` when classification is not configured. |
| `redaction_report` | `RedactionReport \| nil` | `nil` | Audit report of redactions applied by the redaction post-processor. The redaction processor rewrites `content`, `formatted_content`, every chunk's text, and the textual fields of `entities` / `summary` / `translation` / `page_classifications` in place. This report describes what was found and how it was replaced. `nil` when redaction is not configured. |
| `formatted_content` | `String.t() \| nil` | `nil` | Pre-rendered content in the requested output format. Populated during `derive_extraction_result` before tree derivation consumes element data. `apply_output_format` swaps this into `content` at the end of the pipeline, after post-processors have operated on plain text. |
| `ocr_internal_document` | `String.t() \| nil` | `nil` | Structured hOCR document for the OCR+layout pipeline. When tesseract produces hOCR output, the parsed `InternalDocument` carries paragraph structure with bounding boxes and confidence scores. The layout classification step enriches these elements before final rendering. |

### Functions

#### from_ocr()

Convert from an OCR result.

**Signature:**

```elixir
def from_ocr(ocr)
```

---

#### FictionBookMetadata

FictionBook (FB2) metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `genres` | `list(String.t())` | `[]` | Genre tags as declared in the FB2 `<genre>` elements. |
| `sequences` | `list(String.t())` | `[]` | Book series (sequence) names, if any. |
| `annotation` | `String.t() \| nil` | `nil` | Short annotation / summary from the FB2 `<annotation>` element. |

---

#### FileExtractionConfig

Per-file extraction configuration overrides for batch processing.

All fields are `Option<T>` — `nil` means "use the batch-level default."
This type is used with `batch_extract_files` and
`batch_extract_bytes` to allow heterogeneous
extraction settings within a single batch.

### Excluded Fields

The following `ExtractionConfig` fields are batch-level only and
cannot be overridden per file:

- `max_concurrent_extractions` — controls batch parallelism
- `use_cache` — global caching policy
- `acceleration` — shared ONNX execution provider
- `security_limits` — global archive security policy

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enable_quality_processing` | `boolean() \| nil` | `nil` | Override quality post-processing for this file. |
| `ocr` | `OcrConfig \| nil` | `nil` | Override OCR configuration for this file (None in the Option = use batch default). |
| `force_ocr` | `boolean() \| nil` | `nil` | Override force OCR for this file. |
| `force_ocr_pages` | `list(integer()) \| nil` | `[]` | Override force OCR pages for this file (1-indexed page numbers). |
| `disable_ocr` | `boolean() \| nil` | `nil` | Override disable OCR for this file. |
| `chunking` | `ChunkingConfig \| nil` | `nil` | Override chunking configuration for this file. |
| `content_filter` | `ContentFilterConfig \| nil` | `nil` | Override content filtering configuration for this file. |
| `images` | `ImageExtractionConfig \| nil` | `nil` | Override image extraction configuration for this file. |
| `pdf_options` | `PdfConfig \| nil` | `nil` | Override PDF options for this file. |
| `token_reduction` | `TokenReductionOptions \| nil` | `nil` | Override token reduction for this file. |
| `language_detection` | `LanguageDetectionConfig \| nil` | `nil` | Override language detection for this file. |
| `pages` | `PageConfig \| nil` | `nil` | Override page extraction for this file. |
| `keywords` | `KeywordConfig \| nil` | `nil` | Override keyword extraction for this file. |
| `postprocessor` | `PostProcessorConfig \| nil` | `nil` | Override post-processor for this file. |
| `html_options` | `String.t() \| nil` | `nil` | Override HTML conversion options for this file. |
| `result_format` | `ResultFormat \| nil` | `nil` | Override result format for this file. |
| `output_format` | `OutputFormat \| nil` | `nil` | Override output content format for this file. |
| `include_document_structure` | `boolean() \| nil` | `nil` | Override document structure output for this file. |
| `layout` | `LayoutDetectionConfig \| nil` | `nil` | Override layout detection for this file. |
| `timeout_secs` | `integer() \| nil` | `nil` | Override per-file extraction timeout in seconds. When set, the extraction for this file will be canceled after the specified duration. A timed-out file produces an error result without affecting other files in the batch. |
| `tree_sitter` | `TreeSitterConfig \| nil` | `nil` | Override tree-sitter configuration for this file. |
| `structured_extraction` | `StructuredExtractionConfig \| nil` | `nil` | Override structured extraction configuration for this file. When set, enables LLM-based structured extraction with a JSON schema for this specific file. The extracted content is sent to a VLM/LLM and the response is parsed according to the provided schema. |

---

#### Footnote

Footnote in Djot.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `label` | `String.t()` | — | Footnote label |
| `content` | `list(FormattedBlock)` | — | Footnote content blocks |

---

#### FormattedBlock

Block-level element in a Djot document.

Represents structural elements like headings, paragraphs, lists, code blocks, etc.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `block_type` | `BlockType` | — | Type of block element |
| `level` | `integer() \| nil` | `nil` | Heading level (1-6) for headings, or nesting level for lists |
| `inline_content` | `list(InlineElement)` | — | Inline content within the block |
| `attributes` | `String.t() \| nil` | `nil` | Element attributes (classes, IDs, key-value pairs) |
| `language` | `String.t() \| nil` | `nil` | Language identifier for code blocks |
| `code` | `String.t() \| nil` | `nil` | Raw code content for code blocks |
| `children` | `list(FormattedBlock)` | `/* serde(default) */` | Nested blocks for containers (blockquotes, list items, divs) |

---

#### GridCell

Individual grid cell with position and span metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String.t()` | — | Cell text content. |
| `row` | `integer()` | — | Zero-indexed row position. |
| `col` | `integer()` | — | Zero-indexed column position. |
| `row_span` | `integer()` | `/* serde(default) */` | Number of rows this cell spans. |
| `col_span` | `integer()` | `/* serde(default) */` | Number of columns this cell spans. |
| `is_header` | `boolean()` | `/* serde(default) */` | Whether this is a header cell. |
| `bbox` | `BoundingBox \| nil` | `nil` | Bounding box for this cell (if available). |

---

#### HeaderMetadata

Header/heading element metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `level` | `integer()` | — | Header level: 1 (h1) through 6 (h6) |
| `text` | `String.t()` | — | Normalized text content of the header |
| `id` | `String.t() \| nil` | `nil` | HTML id attribute if present |
| `depth` | `integer()` | — | Document tree depth at the header element |
| `html_offset` | `integer()` | — | Byte offset in original HTML document |

---

#### HeadingContext

Heading context for a chunk within a Markdown document.

Contains the heading hierarchy from document root to this chunk's section.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `headings` | `list(HeadingLevel)` | — | The heading hierarchy from document root to this chunk's section. Index 0 is the outermost (h1), last element is the most specific. |

---

#### HeadingLevel

A single heading in the hierarchy.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `level` | `integer()` | — | Heading depth (1 = h1, 2 = h2, etc.) |
| `text` | `String.t()` | — | The text content of the heading. |

---

#### HierarchicalBlock

A text block with hierarchy level assignment.

Represents a block of text with semantic heading information extracted from
font size clustering and hierarchical analysis.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text` | `String.t()` | — | The text content of this block |
| `font_size` | `float()` | — | The font size of the text in this block |
| `level` | `String.t()` | — | The hierarchy level of this block (H1-H6 or Body) Levels correspond to HTML heading tags: - "h1": Top-level heading - "h2": Secondary heading - "h3": Tertiary heading - "h4": Quaternary heading - "h5": Quinary heading - "h6": Senary heading - "body": Body text (no heading level) |
| `bbox` | `list(float()) \| nil` | `nil` | Bounding box information for the block Contains coordinates as (left, top, right, bottom) in PDF units. |

---

#### HierarchyConfig

Hierarchy extraction configuration for PDF text structure analysis.

Enables extraction of document hierarchy levels (H1-H6) based on font size
clustering and semantic analysis. When enabled, hierarchical blocks are
included in page content.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `boolean()` | `true` | Enable hierarchy extraction |
| `k_clusters` | `integer()` | `3` | Number of font size clusters to use for hierarchy levels (1-7) Default: 6, which provides H1-H6 heading levels with body text. Larger values create more fine-grained hierarchy levels. |
| `include_bbox` | `boolean()` | `true` | Include bounding box information in hierarchy blocks |
| `ocr_coverage_threshold` | `float() \| nil` | `nil` | OCR coverage threshold for smart OCR triggering (0.0-1.0) Determines when OCR should be triggered based on text block coverage. OCR is triggered when text blocks cover less than this fraction of the page. Default: 0.5 (trigger OCR if less than 50% of page has text) |

### Functions

#### default()

**Signature:**

```elixir
def default()
```

---

#### HtmlMetadata

HTML metadata extracted from HTML documents.

Includes document-level metadata, Open Graph data, Twitter Card metadata,
and extracted structural elements (headers, links, images, structured data).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `title` | `String.t() \| nil` | `nil` | Document title from `<title>` tag |
| `description` | `String.t() \| nil` | `nil` | Document description from `<meta name="description">` tag |
| `keywords` | `list(String.t())` | `[]` | Document keywords from `<meta name="keywords">` tag, split on commas |
| `author` | `String.t() \| nil` | `nil` | Document author from `<meta name="author">` tag |
| `canonical_url` | `String.t() \| nil` | `nil` | Canonical URL from `<link rel="canonical">` tag |
| `base_href` | `String.t() \| nil` | `nil` | Base URL from `<base href="">` tag for resolving relative URLs |
| `language` | `String.t() \| nil` | `nil` | Document language from `lang` attribute |
| `text_direction` | `TextDirection \| nil` | `nil` | Document text direction from `dir` attribute |
| `open_graph` | `map()` | `%{}` | Open Graph metadata (og:* properties) for social media Keys like "title", "description", "image", "url", etc. |
| `twitter_card` | `map()` | `%{}` | Twitter Card metadata (twitter:* properties) Keys like "card", "site", "creator", "title", "description", "image", etc. |
| `meta_tags` | `map()` | `%{}` | Additional meta tags not covered by specific fields Keys are meta name/property attributes, values are content |
| `headers` | `list(HeaderMetadata)` | `[]` | Extracted header elements with hierarchy |
| `links` | `list(LinkMetadata)` | `[]` | Extracted hyperlinks with type classification |
| `images` | `list(ImageMetadataType)` | `[]` | Extracted images with source and dimensions |
| `structured_data` | `list(StructuredData)` | `[]` | Extracted structured data blocks |

---

#### HtmlOutputConfig

Configuration for styled HTML output.

When set on `html_output` alongside
`output_format = OutputFormat.Html`, the pipeline builds a
`StyledHtmlRenderer` instead of
the plain comrak-based renderer.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `css` | `String.t() \| nil` | `nil` | Inline CSS string injected into the output after the theme stylesheet. Concatenated after `css_file` content when both are set. |
| `css_file` | `String.t() \| nil` | `nil` | Path to a CSS file loaded once at renderer construction time. Concatenated before `css` when both are set. |
| `theme` | `HtmlTheme` | `:unstyled` | Built-in colour/typography theme. Default: `HtmlTheme.Unstyled`. |
| `class_prefix` | `String.t()` | — | CSS class prefix applied to every emitted class name. Default: `"kb-"`. Change this if your host application already uses classes that start with `kb-`. |
| `embed_css` | `boolean()` | `true` | When `true` (default), write the resolved CSS into a `<style>` block immediately after the opening `<div class="{prefix}doc">`. Set to `false` to emit only the structural markup and wire up your own stylesheet targeting the `kb-*` class names. |

### Functions

#### default()

**Signature:**

```elixir
def default()
```

---

#### ImageExtractionConfig

Image extraction configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `extract_images` | `boolean()` | `true` | Extract images from documents |
| `target_dpi` | `integer()` | `300` | Target DPI for image normalization |
| `max_image_dimension` | `integer()` | `4096` | Maximum dimension for images (width or height) |
| `inject_placeholders` | `boolean()` | `true` | Whether to inject image reference placeholders into markdown output. When `true` (default), image references like `![Image 1](embedded:p1_i0)` are appended to the markdown. Set to `false` to extract images as data without polluting the markdown output. |
| `auto_adjust_dpi` | `boolean()` | `true` | Automatically adjust DPI based on image content |
| `min_dpi` | `integer()` | `72` | Minimum DPI threshold |
| `max_dpi` | `integer()` | `600` | Maximum DPI threshold |
| `max_images_per_page` | `integer() \| nil` | `nil` | Maximum number of image objects to extract per PDF page. Some PDFs (e.g. technical diagrams stored as thousands of raster fragments) can trigger extremely long or indefinite extraction times when every image object on a dense page is decoded individually via the PDF extractor. Setting this limit causes kreuzberg to stop collecting individual images once the count per page reaches the cap and emit a warning instead. `nil` (default) means no limit — all images are extracted. |
| `classify` | `boolean()` | `true` | When `true` (default), extracted images are classified by kind and grouped into clusters where they appear to belong to one figure. |
| `include_page_rasters` | `boolean()` | `false` | When `true`, full-page renders produced during OCR preprocessing are captured and returned as `ImageKind.PageRaster` entries in `ExtractionResult.images`. **PDF + OCR only.** No rasters are captured for non-PDF inputs or when the document-level OCR bypass is active (whole-document backend). When OCR is enabled and this flag is set but the active backend skips per-page rendering, a `ProcessingWarning` is emitted in `ExtractionResult.processing_warnings`. Defaults to `false`. Enable when downstream consumers need page thumbnails (e.g. citation previews, visual grounding). |
| `run_ocr_on_images` | `boolean()` | `true` | Run OCR on extracted images and include the recognized text in the document content. When `true` (default) and `ExtractionConfig.ocr` is configured, extracted images are processed with the configured OCR backend. Set to `false` to extract images without OCR processing, even when OCR is enabled. |
| `ocr_text_only` | `boolean()` | `false` | When `true`, image OCR results are rendered as plain text without the `![...](...)` markdown placeholder. Only takes effect when `run_ocr_on_images` is also `true`. |
| `append_ocr_text` | `boolean()` | `false` | When `true` and `ocr_text_only` is `false`, append the OCR text after the image placeholder in the rendered output. |

### Functions

#### default()

**Signature:**

```elixir
def default()
```

---

#### ImageMetadata

Image metadata extracted from image files.

Includes dimensions, format, and EXIF data.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `width` | `integer()` | — | Image width in pixels |
| `height` | `integer()` | — | Image height in pixels |
| `format` | `String.t()` | — | Image format (e.g., "PNG", "JPEG", "TIFF") |
| `exif` | `map()` | `%{}` | EXIF metadata tags |

---

#### ImageMetadataType

Image element metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `src` | `String.t()` | — | Image source (URL, data URI, or SVG content) |
| `alt` | `String.t() \| nil` | `nil` | Alternative text from alt attribute |
| `title` | `String.t() \| nil` | `nil` | Title attribute |
| `dimensions` | `list(integer()) \| nil` | `nil` | Image dimensions as (width, height) if available |
| `image_type` | `ImageType` | — | Image type classification |
| `attributes` | `list(list(String.t()))` | — | Additional attributes as key-value pairs |

---

#### ImagePreprocessingConfig

Image preprocessing configuration for OCR.

These settings control how images are preprocessed before OCR to improve
text recognition quality. Different preprocessing strategies work better
for different document types.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `target_dpi` | `integer()` | `300` | Target DPI for the image (300 is standard, 600 for small text). |
| `auto_rotate` | `boolean()` | `true` | Auto-detect and correct image rotation. |
| `deskew` | `boolean()` | `true` | Correct skew (tilted images). |
| `denoise` | `boolean()` | `false` | Remove noise from the image. |
| `contrast_enhance` | `boolean()` | `false` | Enhance contrast for better text visibility. |
| `binarization_method` | `String.t()` | `"otsu"` | Binarization method: "otsu", "sauvola", "adaptive". |
| `invert_colors` | `boolean()` | `false` | Invert colors (white text on black → black on white). |

### Functions

#### default()

**Signature:**

```elixir
def default()
```

---

#### ImagePreprocessingMetadata

Image preprocessing metadata.

Tracks the transformations applied to an image during OCR preprocessing,
including DPI normalization, resizing, and resampling.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `original_dimensions` | `list(integer())` | — | Original image dimensions (width, height) in pixels |
| `original_dpi` | `list(float())` | — | Original image DPI (horizontal, vertical) |
| `target_dpi` | `integer()` | — | Target DPI from configuration |
| `scale_factor` | `float()` | — | Scaling factor applied to the image |
| `auto_adjusted` | `boolean()` | — | Whether DPI was auto-adjusted based on content |
| `final_dpi` | `integer()` | — | Final DPI after processing |
| `new_dimensions` | `list(integer()) \| nil` | `nil` | New dimensions after resizing (if resized) |
| `resample_method` | `String.t()` | — | Resampling algorithm used ("LANCZOS3", "CATMULLROM", etc.) |
| `dimension_clamped` | `boolean()` | — | Whether dimensions were clamped to max_image_dimension |
| `calculated_dpi` | `integer() \| nil` | `nil` | Calculated optimal DPI (if auto_adjust_dpi enabled) |
| `skipped_resize` | `boolean()` | — | Whether resize was skipped (dimensions already optimal) |
| `resize_error` | `String.t() \| nil` | `nil` | Error message if resize failed |

---

#### InlineElement

Inline element within a block.

Represents text with formatting, links, images, etc.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `element_type` | `InlineType` | — | Type of inline element |
| `content` | `String.t()` | — | Text content |
| `attributes` | `String.t() \| nil` | `nil` | Element attributes |
| `metadata` | `map() \| nil` | `nil` | Additional metadata (e.g., href for links, src/alt for images) |

---

#### JatsMetadata

JATS (Journal Article Tag Suite) metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `copyright` | `String.t() \| nil` | `nil` | Copyright statement from the article's `<permissions>` element. |
| `license` | `String.t() \| nil` | `nil` | Open-access license URI from the article's `<license>` element. |
| `history_dates` | `map()` | `%{}` | Publication history dates keyed by event type (e.g. `"received"`, `"accepted"`). |
| `contributor_roles` | `list(ContributorRole)` | `[]` | Authors and contributors with their stated roles. |

---

#### Keyword

Extracted keyword with metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text` | `String.t()` | — | The keyword text. |
| `score` | `float()` | — | Relevance score (higher is better, algorithm-specific range). |
| `algorithm` | `KeywordAlgorithm` | — | Algorithm that extracted this keyword. |
| `positions` | `list(integer()) \| nil` | `nil` | Optional positions where keyword appears in text (character offsets). |

---

#### KeywordConfig

Keyword extraction configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `algorithm` | `KeywordAlgorithm` | `:yake` | Algorithm to use for extraction. |
| `max_keywords` | `integer()` | `10` | Maximum number of keywords to extract (default: 10). |
| `min_score` | `float()` | `0` | Minimum score threshold (0.0-1.0, default: 0.0). Keywords with scores below this threshold are filtered out. Note: Score ranges differ between algorithms. |
| `ngram_range` | `list(integer())` | `[]` | N-gram range for keyword extraction (min, max). (1, 1) = unigrams only (1, 2) = unigrams and bigrams (1, 3) = unigrams, bigrams, and trigrams (default) |
| `language` | `String.t() \| nil` | `nil` | Language code for stopword filtering (e.g., "en", "de", "fr"). If None, no stopword filtering is applied. |
| `yake_params` | `YakeParams \| nil` | `nil` | YAKE-specific tuning parameters. |
| `rake_params` | `RakeParams \| nil` | `nil` | RAKE-specific tuning parameters. |

### Functions

#### default()

**Signature:**

```elixir
def default()
```

---

#### LanguageDetectionConfig

Language detection configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `boolean()` | `true` | Enable language detection |
| `min_confidence` | `float()` | `0.8` | Minimum confidence threshold (0.0-1.0) |
| `detect_multiple` | `boolean()` | `false` | Detect multiple languages in the document |

### Functions

#### default()

**Signature:**

```elixir
def default()
```

---

#### LayoutDetection

A single layout detection result.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `class_name` | `LayoutClass` | — | Detected layout class (e.g. `Table`, `Text`, `Title`). |
| `confidence` | `float()` | — | Detection confidence score in `[0.0, 1.0]`. |
| `bbox` | `BBox` | — | Bounding box in image pixel coordinates. |

---

#### LayoutDetectionConfig

Layout detection configuration.

Controls layout detection behavior in the extraction pipeline.
When set on `ExtractionConfig`, layout detection
is enabled for PDF extraction.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `confidence_threshold` | `float() \| nil` | `nil` | Confidence threshold override (None = use model default). |
| `apply_heuristics` | `boolean()` | `true` | Whether to apply postprocessing heuristics (default: true). |
| `table_model` | `TableModel` | `:tatr` | Table structure recognition model. Controls which model is used for table cell detection within layout-detected table regions. Defaults to `TableModel.Tatr`. |
| `acceleration` | `AccelerationConfig \| nil` | `nil` | Hardware acceleration for ONNX models (layout detection + table structure). When set, controls which execution provider (CPU, CUDA, CoreML, TensorRT) is used for inference. Defaults to `nil` (auto-select per platform). |

### Functions

#### default()

**Signature:**

```elixir
def default()
```

---

#### LayoutRegion

A detected layout region on a page.

When layout detection is enabled, each page may have layout regions
identifying different content types (text, pictures, tables, etc.)
with confidence scores and spatial positions.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `class_name` | `String.t()` | — | Layout class name (e.g. "picture", "table", "text", "section_header"). |
| `confidence` | `float()` | — | Confidence score from the layout detection model (0.0 to 1.0). |
| `bounding_box` | `BoundingBox` | — | Bounding box in document coordinate space. |
| `area_fraction` | `float()` | — | Fraction of the page area covered by this region (0.0 to 1.0). |

---

#### LinkMetadata

Link element metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `href` | `String.t()` | — | The href URL value |
| `text` | `String.t()` | — | Link text content (normalized) |
| `title` | `String.t() \| nil` | `nil` | Optional title attribute |
| `link_type` | `LinkType` | — | Link type classification |
| `rel` | `list(String.t())` | — | Rel attribute values |
| `attributes` | `list(list(String.t()))` | — | Additional attributes as key-value pairs |

---

#### LlmBackend

liter-llm-backed NER backend.

### Functions

#### new()

Create a new LLM-backed NER backend with the given LLM configuration.

**Signature:**

```elixir
def new(config)
```

#### detect()

**Signature:**

```elixir
def detect(text, categories)
```

#### detect_with_custom()

**Signature:**

```elixir
def detect_with_custom(text, categories, custom_labels)
```

---

#### LlmConfig

Configuration for an LLM provider/model via liter-llm.

Each feature (VLM OCR, VLM embeddings, structured extraction) carries
its own `LlmConfig`, allowing different providers per feature.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String.t()` | — | Provider/model string using liter-llm routing format. Examples: `"openai/gpt-4o"`, `"anthropic/claude-sonnet-4-20250514"`, `"groq/llama-3.1-70b-versatile"`. |
| `api_key` | `String.t() \| nil` | `nil` | API key for the provider. When `nil`, liter-llm falls back to the provider's standard environment variable (e.g., `OPENAI_API_KEY`). |
| `base_url` | `String.t() \| nil` | `nil` | Custom base URL override for the provider endpoint. |
| `timeout_secs` | `integer() \| nil` | `nil` | Request timeout in seconds (default: 60). |
| `max_retries` | `integer() \| nil` | `nil` | Maximum retry attempts (default: 3). |
| `temperature` | `float() \| nil` | `nil` | Sampling temperature for generation tasks. |
| `max_tokens` | `integer() \| nil` | `nil` | Maximum tokens to generate. |

---

#### LlmUsage

Token usage and cost data for a single LLM call made during extraction.

Populated when VLM OCR, structured extraction, or LLM-based embeddings
are used. Multiple entries may be present when multiple LLM calls occur
within one extraction (e.g. VLM OCR + structured extraction).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String.t()` | — | The LLM model identifier (e.g. "openai/gpt-4o", "anthropic/claude-sonnet-4-20250514"). |
| `source` | `String.t()` | — | The pipeline stage that triggered this LLM call (e.g. "vlm_ocr", "structured_extraction", "embeddings"). |
| `input_tokens` | `integer() \| nil` | `nil` | Number of input/prompt tokens consumed. |
| `output_tokens` | `integer() \| nil` | `nil` | Number of output/completion tokens generated. |
| `total_tokens` | `integer() \| nil` | `nil` | Total tokens (input + output). |
| `estimated_cost` | `float() \| nil` | `nil` | Estimated cost in USD based on the provider's published pricing. |
| `finish_reason` | `String.t() \| nil` | `nil` | Why the model stopped generating (e.g. "stop", "length", "content_filter"). |

---

#### Metadata

Extraction result metadata.

Contains common fields applicable to all formats, format-specific metadata
via a discriminated union, and additional custom fields from postprocessors.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `title` | `String.t() \| nil` | `nil` | Document title |
| `subject` | `String.t() \| nil` | `nil` | Document subject or description |
| `authors` | `list(String.t()) \| nil` | `[]` | Primary author(s) - always Vec for consistency |
| `keywords` | `list(String.t()) \| nil` | `[]` | Keywords/tags - always Vec for consistency |
| `language` | `String.t() \| nil` | `nil` | Primary language (ISO 639 code) |
| `created_at` | `String.t() \| nil` | `nil` | Creation timestamp (ISO 8601 format) |
| `modified_at` | `String.t() \| nil` | `nil` | Last modification timestamp (ISO 8601 format) |
| `created_by` | `String.t() \| nil` | `nil` | User who created the document |
| `modified_by` | `String.t() \| nil` | `nil` | User who last modified the document |
| `pages` | `PageStructure \| nil` | `nil` | Page/slide/sheet structure with boundaries |
| `format` | `FormatMetadata \| nil` | `nil` | Format-specific metadata (discriminated union) Contains detailed metadata specific to the document format. Serialized as a nested `"format"` object with a `format_type` discriminator field. |
| `image_preprocessing` | `ImagePreprocessingMetadata \| nil` | `nil` | Image preprocessing metadata (when OCR preprocessing was applied) |
| `json_schema` | `term() \| nil` | `nil` | JSON schema (for structured data extraction) |
| `error` | `ErrorMetadata \| nil` | `nil` | Error metadata (for batch operations) |
| `extraction_duration_ms` | `integer() \| nil` | `nil` | Extraction duration in milliseconds (for benchmarking). This field is populated by batch extraction to provide per-file timing information. It's `nil` for single-file extraction (which uses external timing). |
| `category` | `String.t() \| nil` | `nil` | Document category (from frontmatter or classification). |
| `tags` | `list(String.t()) \| nil` | `[]` | Document tags (from frontmatter). |
| `document_version` | `String.t() \| nil` | `nil` | Document version string (from frontmatter). |
| `abstract_text` | `String.t() \| nil` | `nil` | Abstract or summary text (from frontmatter). |
| `output_format` | `String.t() \| nil` | `nil` | Output format identifier (e.g., "markdown", "html", "text"). Set by the output format pipeline stage when format conversion is applied. Previously stored in `metadata.additional["output_format"]`. |
| `ocr_used` | `boolean()` | — | Whether OCR was used during extraction. Set to `true` whenever the extraction pipeline ran an OCR backend (Tesseract, PaddleOCR, VLM, etc.) and used that output as the primary or fallback text. `false` means native text extraction was used exclusively. |
| `additional` | `map()` | `%{}` | Additional custom fields from postprocessors. Serialized as a nested `"additional"` object (not flattened at root level). Uses `Cow<'static, str>` keys so static string keys avoid allocation. |

### Functions

#### is_empty()

Returns `true` when no metadata fields, format-specific metadata, or
additional postprocessor fields are populated.

**Signature:**

```elixir
def is_empty()
```

---

#### ModelPaths

Combined paths to all models needed for OCR (backward compatibility).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `det_model` | `String.t()` | — | Path to the detection model directory. |
| `cls_model` | `String.t()` | — | Path to the classification model directory. |
| `rec_model` | `String.t()` | — | Path to the recognition model directory. |
| `dict_file` | `String.t()` | — | Path to the character dictionary file. |

---

#### NerBackend

NER backend trait (stub for Android x86_64).

---

#### NerConfig

**Since:** `v5.0.0`

Configuration for the NER post-processor.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `backend` | `NerBackendKind` | `:onnx` | Backend that runs the entity detection. |
| `categories` | `list(EntityCategory)` | `[]` | Entity categories to detect. Defaults to a sensible PERSON/ORG/LOCATION/EMAIL set when empty. |
| `model` | `String.t() \| nil` | `nil` | Override the default model — only used by `NerBackendKind.Onnx`. `nil` lets the backend pick its pinned default (`urchade/gliner_multi-v2.1` for gline-rs). |
| `llm` | `LlmConfig \| nil` | `nil` | Optional LLM configuration — only used by `NerBackendKind.Llm`. Token usage for LLM backends is recorded in `ExtractionResult.llm_usage`. |
| `custom_labels` | `list(String.t())` | `[]` | Arbitrary user-supplied entity labels for zero-shot detection. gline-rs natively supports zero-shot inference over caller-supplied labels — this is the primary value of GLiNER. The LLM backend also honours these labels by including them in the structured-output schema. Custom labels surface as `EntityCategory.Custom` in the resulting `Entity` stream. Use this when you need domain-specific entity types (e.g. `"Treatment"`, `"Product"`, `"Vessel"`) without forking GLiNER's taxonomy. |

---

#### OcrBackend

Trait for OCR backend plugins.

Implement this trait to add custom OCR capabilities. OCR backends can be:

- Native Rust implementations (like Tesseract)
- FFI bridges to Python libraries (like EasyOCR, PaddleOCR)
- Cloud-based OCR services (Google Vision, AWS Textract, etc.)

### Thread Safety

OCR backends must be thread-safe (`Send + Sync`) to support concurrent processing.

### Functions

#### process_image()

Process an image and extract text via OCR.

**Returns:**

An `ExtractionResult` containing the extracted text and metadata.

**Errors:**

- `KreuzbergError.Ocr` - OCR processing failed
- `KreuzbergError.Validation` - Invalid image format or configuration
- `KreuzbergError.Io` - I/O errors (these always bubble up)

### Reading `backend_options`

Backends that support runtime tuning can read `config.backend_options` and
deserialize only the keys they care about. Unknown keys are silently ignored,
so multiple backends can coexist in a pipeline without key conflicts.

**Signature:**

```elixir
def process_image(image_bytes, config)
```

#### process_image_file()

Process a file and extract text via OCR.

Default implementation reads the file and calls `process_image`.
Override for custom file handling or optimizations.

**Errors:**

Same as `process_image`, plus file I/O errors.

**Signature:**

```elixir
def process_image_file(path, config)
```

#### supports_language()

Check if this backend supports a given language code.

**Returns:**

`true` if the language is supported, `false` otherwise.

**Signature:**

```elixir
def supports_language(lang)
```

#### backend_type()

Get the backend type identifier.

**Returns:**

The backend type enum value.

**Signature:**

```elixir
def backend_type()
```

#### supported_languages()

Optional: Get a list of all supported languages.

Defaults to empty list. Override to provide comprehensive language support info.

**Signature:**

```elixir
def supported_languages()
```

#### supports_table_detection()

Optional: Check if the backend supports table detection.

Defaults to `false`. Override if your backend can detect and extract tables.

**Signature:**

```elixir
def supports_table_detection()
```

#### supports_document_processing()

Check if the backend supports direct document-level processing (e.g. for PDFs).

Defaults to `false`. Override if the backend has optimized document processing.

**Signature:**

```elixir
def supports_document_processing()
```

#### process_document()

Process a document file directly via OCR.

Only called if `supports_document_processing` returns `true`.

**Signature:**

```elixir
def process_document(path, config)
```

---

#### OcrConfidence

Confidence scores for an OCR element.

Separates detection confidence (how confident that text exists at this location)
from recognition confidence (how confident about the actual text content).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `detection` | `float() \| nil` | `nil` | Detection confidence: how confident the OCR engine is that text exists here. PaddleOCR provides this as `box_score`, Tesseract doesn't have a direct equivalent. Range: 0.0 to 1.0 (or None if not available). |
| `recognition` | `float()` | — | Recognition confidence: how confident about the text content. Range: 0.0 to 1.0. |

---

#### OcrConfig

OCR configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `boolean()` | `true` | Whether OCR is enabled. Setting `enabled: false` is a shorthand for `disable_ocr: true` on the parent `ExtractionConfig`. Images return metadata only; PDFs use native text extraction without OCR fallback. Defaults to `true`. When `false`, all other OCR settings are ignored. |
| `backend` | `String.t()` | — | OCR backend: tesseract, easyocr, paddleocr |
| `language` | `String.t()` | — | Language code (e.g., "eng", "deu") |
| `tesseract_config` | `TesseractConfig \| nil` | `nil` | Tesseract-specific configuration (optional) |
| `output_format` | `OutputFormat \| nil` | `nil` | Output format for OCR results (optional, for format conversion) |
| `paddle_ocr_config` | `term() \| nil` | `nil` | PaddleOCR-specific configuration (optional, JSON passthrough) |
| `backend_options` | `term() \| nil` | `nil` | Arbitrary per-call options passed through to the backend unchanged. Custom OCR backends and built-in backends that support runtime tuning can read this value and deserialize the keys they care about. Keys unknown to the backend are silently ignored. This is the recommended extension point for per-call parameters that are not covered by the typed fields above (e.g. mode switching, preprocessing flags, inference batch size). **Scope:** when `pipeline` is `nil`, this value is propagated to the primary stage of the auto-constructed pipeline. When `pipeline` is explicitly set, this field has **no effect** — the caller must set `OcrPipelineStage.backend_options` directly on the relevant stage(s) instead. Example: ```json { "mode": "fast", "enable_layout": true, "timeout_ms": 5000 } ``` |
| `element_config` | `OcrElementConfig \| nil` | `nil` | OCR element extraction configuration |
| `quality_thresholds` | `OcrQualityThresholds \| nil` | `nil` | Quality thresholds for the native-text-to-OCR fallback decision. When None, uses compiled defaults (matching previous hardcoded behavior). |
| `pipeline` | `OcrPipelineConfig \| nil` | `nil` | Multi-backend OCR pipeline configuration. When set, enables weighted fallback across multiple OCR backends based on output quality. When None, uses the single `backend` field (same as today). |
| `auto_rotate` | `boolean()` | `false` | Enable automatic page rotation based on orientation detection. When enabled, uses Tesseract's `DetectOrientationScript()` to detect page orientation (0/90/180/270 degrees) before OCR. If the page is rotated with high confidence, the image is corrected before recognition. This is critical for handling rotated scanned documents. |
| `vlm_fallback` | `VlmFallbackPolicy` | `:disabled` | Ergonomic VLM fallback policy. When set to anything other than `VlmFallbackPolicy.Disabled` and `OcrConfig.pipeline` is `nil`, a multi-stage pipeline is synthesised automatically: - `VlmFallbackPolicy.OnLowQuality` → `[classical_stage, vlm_stage]` with the `quality_threshold` mapped onto `OcrQualityThresholds.pipeline_min_quality`. - `VlmFallbackPolicy.Always` → `[vlm_stage]` only. Requires `OcrConfig.vlm_config` to be `Some` when not `Disabled`. When `OcrConfig.pipeline` is explicitly set, this field is ignored. |
| `vlm_config` | `LlmConfig \| nil` | `nil` | VLM (Vision Language Model) OCR configuration. Required when `backend` is `"vlm"` or when `vlm_fallback` is not `VlmFallbackPolicy.Disabled`. Uses liter-llm to send page images to a vision model for text extraction. |
| `vlm_prompt` | `String.t() \| nil` | `nil` | Custom Jinja2 prompt template for VLM OCR. When `nil`, uses the default template. Available variables: - `{{ language }}` — The document language code (e.g., "eng", "deu"). |
| `acceleration` | `AccelerationConfig \| nil` | `nil` | Hardware acceleration for ONNX Runtime models (e.g. PaddleOCR, layout detection). Not user-configurable via config files — injected at runtime from `ExtractionConfig.acceleration` before each `process_image` call. |
| `tessdata_bytes` | `map() \| nil` | `nil` | Caller-supplied Tesseract `traineddata` bytes per language code. Primary use case is the WASM build, which has no filesystem and cannot download tessdata at runtime. Native builds typically rely on `TessdataManager` and ignore this field. When present, the WASM Tesseract backend prefers these bytes over its compile-time-bundled English data. Skipped by serde to keep config files small — supply via the typed API at runtime. |

### Functions

#### default()

**Signature:**

```elixir
def default()
```

---

#### OcrElement

A unified OCR element representing detected text with full metadata.

This is the primary type for structured OCR output, preserving all information
from both Tesseract and PaddleOCR backends.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text` | `String.t()` | — | The recognized text content. |
| `geometry` | `OcrBoundingGeometry` | `:rectangle` | Bounding geometry (rectangle or quadrilateral). |
| `confidence` | `OcrConfidence` | — | Confidence scores for detection and recognition. |
| `level` | `OcrElementLevel` | `:line` | Hierarchical level (word, line, block, page). |
| `rotation` | `OcrRotation \| nil` | `nil` | Rotation information (if detected). |
| `page_number` | `integer()` | — | Page number (1-indexed). |
| `parent_id` | `String.t() \| nil` | `nil` | Parent element ID for hierarchical relationships. Only used for Tesseract output which has word -> line -> block hierarchy. |
| `backend_metadata` | `map()` | `%{}` | Backend-specific metadata that doesn't fit the unified schema. |

---

#### OcrElementConfig

Configuration for OCR element extraction.

Controls how OCR elements are extracted and filtered.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `include_elements` | `boolean()` | — | Whether to include OCR elements in the extraction result. When true, the `ocr_elements` field in `ExtractionResult` will be populated. |
| `min_level` | `OcrElementLevel` | `:line` | Minimum hierarchical level to include. Elements below this level (e.g., words when min_level is Line) will be excluded. |
| `min_confidence` | `float()` | — | Minimum recognition confidence threshold (0.0-1.0). Elements with confidence below this threshold will be filtered out. |
| `build_hierarchy` | `boolean()` | — | Whether to build hierarchical relationships between elements. When true, `parent_id` fields will be populated based on spatial containment. Only meaningful for Tesseract output. |

---

#### OcrExtractionResult

OCR extraction result.

Result of performing OCR on an image or scanned document,
including recognized text and detected tables.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String.t()` | — | Recognized text content |
| `mime_type` | `String.t()` | — | Original MIME type of the processed image |
| `metadata` | `map()` | — | OCR processing metadata (confidence scores, language, etc.) |
| `tables` | `list(OcrTable)` | — | Tables detected and extracted via OCR |
| `ocr_elements` | `list(OcrElement) \| nil` | `/* serde(default) */` | Structured OCR elements with bounding boxes and confidence scores. Available when TSV output is requested or table detection is enabled. |
| `internal_document` | `String.t() \| nil` | `nil` | Structured document produced from hOCR parsing. Carries paragraph structure, bounding boxes, and confidence scores that the flattened `content` string discards. |

---

#### OcrMetadata

OCR processing metadata.

Captures information about OCR processing configuration and results.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `language` | `String.t()` | — | OCR language code(s) used |
| `psm` | `integer()` | — | Tesseract Page Segmentation Mode (PSM) |
| `output_format` | `String.t()` | — | Output format (e.g., "text", "hocr") |
| `table_count` | `integer()` | — | Number of tables detected |
| `table_rows` | `integer() \| nil` | `nil` | Number of rows in the detected table (if a single table was found). |
| `table_cols` | `integer() \| nil` | `nil` | Number of columns in the detected table (if a single table was found). |

---

#### OcrPipelineConfig

Multi-backend OCR pipeline with quality-based fallback.

Backends are tried in priority order (highest first). After each backend
produces output, quality is evaluated. If it meets `quality_thresholds.pipeline_min_quality`,
the result is accepted. Otherwise the next backend is tried.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `stages` | `list(OcrPipelineStage)` | — | Ordered list of backends to try. Sorted by priority (descending) at runtime. |
| `quality_thresholds` | `OcrQualityThresholds` | `/* serde(default) */` | Quality thresholds for deciding whether to accept a result or try the next backend. |

---

#### OcrPipelineStage

A single backend stage in the OCR pipeline.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `backend` | `String.t()` | — | Backend name: "tesseract", "paddleocr", "easyocr", or a custom registered name. |
| `priority` | `integer()` | `/* serde(default) */` | Priority weight (higher = tried first). Stages are sorted by priority descending. |
| `language` | `String.t() \| nil` | `/* serde(default) */` | Language override for this stage (None = use parent OcrConfig.language). |
| `tesseract_config` | `TesseractConfig \| nil` | `/* serde(default) */` | Tesseract-specific config override for this stage. |
| `paddle_ocr_config` | `term() \| nil` | `/* serde(default) */` | PaddleOCR-specific config for this stage. |
| `vlm_config` | `LlmConfig \| nil` | `/* serde(default) */` | VLM config override for this pipeline stage. |
| `backend_options` | `term() \| nil` | `/* serde(default) */` | Arbitrary per-call options passed through to the backend unchanged. Backends that support runtime tuning (mode switching, preprocessing flags, inference parameters, etc.) read this value and deserialize the keys they care about. Keys unknown to the backend are silently ignored, so options from different backends can coexist in the same config without conflict. Example (custom backend): ```json { "mode": "fast", "enable_layout": true } ``` |

---

#### OcrQualityThresholds

Quality thresholds for OCR fallback decisions and pipeline quality gating.

All fields default to the values that match the previous hardcoded behavior,
so `OcrQualityThresholds.default()` preserves existing semantics exactly.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `min_total_non_whitespace` | `integer()` | `64` | Minimum total non-whitespace characters to consider text substantive. |
| `min_non_whitespace_per_page` | `float()` | `32` | Minimum non-whitespace characters per page on average. |
| `min_meaningful_word_len` | `integer()` | `4` | Minimum character count for a word to be "meaningful". |
| `min_meaningful_words` | `integer()` | `3` | Minimum count of meaningful words before text is accepted. |
| `min_alnum_ratio` | `float()` | `0.3` | Minimum alphanumeric ratio (non-whitespace chars that are alphanumeric). |
| `min_garbage_chars` | `integer()` | `5` | Minimum Unicode replacement characters (U+FFFD) to trigger OCR fallback. |
| `max_fragmented_word_ratio` | `float()` | `0.6` | Maximum fraction of short (1-2 char) words before text is considered fragmented. |
| `critical_fragmented_word_ratio` | `float()` | `0.8` | Critical fragmentation threshold — triggers OCR regardless of meaningful words. Normal English text has ~20-30% short words. 80%+ is definitive garbage. |
| `min_avg_word_length` | `float()` | `2` | Minimum average word length. Below this with enough words indicates garbled extraction. |
| `min_words_for_avg_length_check` | `integer()` | `50` | Minimum word count before average word length check applies. |
| `min_consecutive_repeat_ratio` | `float()` | `0.08` | Minimum consecutive word repetition ratio to detect column scrambling. |
| `min_words_for_repeat_check` | `integer()` | `50` | Minimum word count before consecutive repetition check is applied. |
| `substantive_min_chars` | `integer()` | `100` | Minimum character count for "substantive markdown" OCR skip gate. |
| `non_text_min_chars` | `integer()` | `20` | Minimum character count for "non-text content" OCR skip gate. |
| `alnum_ws_ratio_threshold` | `float()` | `0.4` | Alphanumeric+whitespace ratio threshold for skip decisions. |
| `pipeline_min_quality` | `float()` | `0.5` | Minimum quality score (0.0-1.0) for a pipeline stage result to be accepted. If the result from a backend scores below this, try the next backend. |

### Functions

#### default()

**Signature:**

```elixir
def default()
```

---

#### OcrRotation

Rotation information for an OCR element.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `angle_degrees` | `float()` | — | Rotation angle in degrees (0, 90, 180, 270 for PaddleOCR). |
| `confidence` | `float() \| nil` | `nil` | Confidence score for the rotation detection. |

---

#### OcrTable

Table detected via OCR.

Represents a table structure recognized during OCR processing.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `cells` | `list(list(String.t()))` | — | Table cells as a 2D vector (rows × columns) |
| `markdown` | `String.t()` | — | Markdown representation of the table |
| `page_number` | `integer()` | — | Page number where the table was found (1-indexed) |
| `bounding_box` | `OcrTableBoundingBox \| nil` | `/* serde(default) */` | Bounding box of the table in pixel coordinates (from OCR word positions). |

---

#### OcrTableBoundingBox

Bounding box for an OCR-detected table in pixel coordinates.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `left` | `integer()` | — | Left x-coordinate (pixels) |
| `top` | `integer()` | — | Top y-coordinate (pixels) |
| `right` | `integer()` | — | Right x-coordinate (pixels) |
| `bottom` | `integer()` | — | Bottom y-coordinate (pixels) |

---

#### OrientationResult

Document orientation detection result.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `degrees` | `integer()` | — | Detected orientation in degrees (0, 90, 180, or 270). |
| `confidence` | `float()` | — | Confidence score (0.0-1.0). |

---

#### PaddleOcrConfig

Configuration for PaddleOCR backend.

Configures PaddleOCR text detection and recognition with multi-language support.
Uses a builder pattern for convenient configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `language` | `String.t()` | — | Language code (e.g., "en", "ch", "jpn", "kor", "deu", "fra") |
| `cache_dir` | `String.t() \| nil` | `nil` | Optional custom cache directory for model files |
| `use_angle_cls` | `boolean()` | — | Enable angle classification for rotated text (default: false). Can misfire on short text regions, rotating crops incorrectly before recognition. |
| `enable_table_detection` | `boolean()` | — | Enable table structure detection (default: false) |
| `det_db_thresh` | `float()` | — | Database threshold for text detection (default: 0.3) Range: 0.0-1.0, higher values require more confident detections |
| `det_db_box_thresh` | `float()` | — | Box threshold for text bounding box refinement (default: 0.5) Range: 0.0-1.0 |
| `det_db_unclip_ratio` | `float()` | — | Unclip ratio for expanding text bounding boxes (default: 1.6) Controls the expansion of detected text regions |
| `det_limit_side_len` | `integer()` | — | Maximum side length for detection image (default: 960) Larger images may be resized to this limit for faster inference |
| `rec_batch_num` | `integer()` | — | Batch size for recognition inference (default: 6) Number of text regions to process simultaneously |
| `padding` | `integer()` | — | Padding in pixels added around the image before detection (default: 10). Large values can include surrounding content like table gridlines. |
| `drop_score` | `float()` | — | Minimum recognition confidence score for text lines (default: 0.5). Text regions with recognition confidence below this threshold are discarded. Matches PaddleOCR Python's `drop_score` parameter. Range: 0.0-1.0 |
| `model_tier` | `String.t()` | — | Model tier controlling detection/recognition model size and accuracy trade-off. - `"mobile"` (default): Lightweight models (~4.5MB detection, ~16.5MB recognition), fast download and inference - `"server"`: Large, high-accuracy models (~88MB detection, ~84MB recognition), best for GPU or complex documents |

### Functions

#### with_cache_dir()

Sets a custom cache directory for model files.

**Signature:**

```elixir
def with_cache_dir(path)
```

#### with_table_detection()

Enables or disables table structure detection.

**Signature:**

```elixir
def with_table_detection(enable)
```

#### with_angle_cls()

Enables or disables angle classification for rotated text.

**Signature:**

```elixir
def with_angle_cls(enable)
```

#### with_det_db_thresh()

Sets the database threshold for text detection.

**Signature:**

```elixir
def with_det_db_thresh(threshold)
```

#### with_det_db_box_thresh()

Sets the box threshold for text bounding box refinement.

**Signature:**

```elixir
def with_det_db_box_thresh(threshold)
```

#### with_det_db_unclip_ratio()

Sets the unclip ratio for expanding text bounding boxes.

**Signature:**

```elixir
def with_det_db_unclip_ratio(ratio)
```

#### with_det_limit_side_len()

Sets the maximum side length for detection images.

**Signature:**

```elixir
def with_det_limit_side_len(length)
```

#### with_rec_batch_num()

Sets the batch size for recognition inference.

**Signature:**

```elixir
def with_rec_batch_num(batch_size)
```

#### with_drop_score()

Sets the minimum recognition confidence threshold.

**Signature:**

```elixir
def with_drop_score(score)
```

#### with_padding()

Sets padding in pixels added around images before detection.

**Signature:**

```elixir
def with_padding(padding)
```

#### with_model_tier()

Sets the model tier controlling detection/recognition model size.

**Signature:**

```elixir
def with_model_tier(tier)
```

#### default()

Creates a default configuration with English language support.

**Signature:**

```elixir
def default()
```

---

#### PageBoundary

Byte offset boundary for a page.

Tracks where a specific page's content starts and ends in the main content string,
enabling mapping from byte positions to page numbers. Offsets are guaranteed to be
at valid UTF-8 character boundaries when using standard String methods (push_str, push, etc.).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `byte_start` | `integer()` | — | Byte offset where this page starts in the content string (UTF-8 valid boundary, inclusive) |
| `byte_end` | `integer()` | — | Byte offset where this page ends in the content string (UTF-8 valid boundary, exclusive) |
| `page_number` | `integer()` | — | Page number (1-indexed) |

---

#### PageClassification

Classification result for a single page.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `page_number` | `integer()` | — | 1-indexed page number this classification belongs to. |
| `labels` | `list(ClassificationLabel)` | — | Labels assigned to the page. Single-label classification yields exactly one entry; multi-label classification yields any subset of the configured label set. |

---

#### PageClassificationConfig

**Since:** `v5.0.0`

Configuration for the page-classification post-processor.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `prompt_template` | `String.t() \| nil` | `nil` | Minijinja prompt template. Receives `{{ labels }}` (joined list), `{{ page_text }}` and `{{ multi_label }}` variables. `nil` lets the backend pick a sensible default. |
| `labels` | `list(String.t())` | — | The set of labels the classifier may emit. Must contain at least one entry. |
| `multi_label` | `boolean()` | `/* serde(default) */` | Allow multiple labels per page. Single-label mode returns at most one label. |
| `llm` | `LlmConfig` | — | LLM configuration used for classification. |

---

#### PageConfig

Page extraction and tracking configuration.

Controls how pages are extracted, tracked, and represented in the extraction results.
When `nil`, page tracking is disabled.

Page range tracking in chunk metadata (first_page/last_page) is automatically enabled
when page boundaries are available and chunking is configured.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `extract_pages` | `boolean()` | `false` | Extract pages as separate array (ExtractionResult.pages) |
| `insert_page_markers` | `boolean()` | `false` | Insert page markers in main content string |
| `marker_format` | `String.t()` | `"<!-- PAGE {page_num} -->"` | Page marker format (use {page_num} placeholder) Default: "\n\n<!-- PAGE {page_num} -->\n\n" |

### Functions

#### default()

**Signature:**

```elixir
def default()
```

---

#### PageContent

Content for a single page/slide.

When page extraction is enabled, documents are split into per-page content
with associated tables and images mapped to each page.

### Performance

Uses Arc-wrapped tables and images for memory efficiency:

- `Vec<Arc<Table>>` enables zero-copy sharing of table data
- `Vec<Arc<ExtractedImage>>` enables zero-copy sharing of image data
- Maintains exact JSON compatibility via custom Serialize/Deserialize

This reduces memory overhead for documents with shared tables/images
by avoiding redundant copies during serialization.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `page_number` | `integer()` | — | Page number (1-indexed) |
| `content` | `String.t()` | — | Text content for this page |
| `tables` | `list(Table)` | `/* serde(default) */` | Tables found on this page (uses Arc for memory efficiency) Serializes as Vec<Table> for JSON compatibility while maintaining Arc semantics in-memory for zero-copy sharing. |
| `image_indices` | `list(integer())` | `/* serde(default) */` | Indices into `ExtractionResult.images` for images found on this page. Each value is a zero-based index into the top-level `images` collection. Only populated when `extract_images = true` in the extraction config. |
| `hierarchy` | `PageHierarchy \| nil` | `nil` | Hierarchy information for the page (when hierarchy extraction is enabled) Contains text hierarchy levels (H1-H6) extracted from the page content. |
| `is_blank` | `boolean() \| nil` | `nil` | Whether this page is blank (no meaningful text content) Determined during extraction based on text content analysis. A page is blank if it has fewer than 3 non-whitespace characters and contains no tables or images. |
| `layout_regions` | `list(LayoutRegion) \| nil` | `nil` | Layout detection regions for this page (when layout detection is enabled). Contains detected layout regions with class, confidence, bounding box, and area fraction. Only populated when layout detection is configured. |
| `speaker_notes` | `String.t() \| nil` | `nil` | Speaker notes for this slide (PPTX only). Contains the text from the slide's notes pane (`ppt/notesSlides/notesSlide{N}.xml`). Only populated when the source is a PPTX file and notes are present. |
| `section_name` | `String.t() \| nil` | `nil` | Section name this slide belongs to (PPTX only). PowerPoint sections group slides into logical chapters (`<p:sectionLst>` in `ppt/presentation.xml`). Only populated when the source is a PPTX file and the slide belongs to a named section. |
| `sheet_name` | `String.t() \| nil` | `nil` | Sheet name for this page (XLSX/ODS only). Each spreadsheet sheet maps to one `PageContent` entry. This field carries the sheet's display name as it appears in the workbook. `nil` for all non-spreadsheet formats and for sheets with an empty name. |

---

#### PageHierarchy

Page hierarchy structure containing heading levels and block information.

Used when PDF text hierarchy extraction is enabled. Contains hierarchical
blocks with heading levels (H1-H6) for semantic document structure.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `block_count` | `integer()` | — | Number of hierarchy blocks on this page |
| `blocks` | `list(HierarchicalBlock)` | `/* serde(default) */` | Hierarchical blocks with heading levels |

---

#### PageInfo

Metadata for individual page/slide/sheet.

Captures per-page information including dimensions, content counts,
and visibility state (for presentations).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `number` | `integer()` | — | Page number (1-indexed) |
| `title` | `String.t() \| nil` | `nil` | Page title (usually for presentations) |
| `dimensions` | `list(float()) \| nil` | `nil` | Dimensions in points (PDF) or pixels (images): (width, height) |
| `image_count` | `integer() \| nil` | `nil` | Number of images on this page |
| `table_count` | `integer() \| nil` | `nil` | Number of tables on this page |
| `hidden` | `boolean() \| nil` | `nil` | Whether this page is hidden (e.g., in presentations) |
| `is_blank` | `boolean() \| nil` | `nil` | Whether this page is blank (no meaningful text, no images, no tables) A page is considered blank if it has fewer than 3 non-whitespace characters and contains no tables or images. This is useful for filtering out empty pages in scanned documents or PDFs with blank separator pages. |
| `has_vector_graphics` | `boolean()` | `/* serde(default) */` | Whether this page contains non-trivial vector graphics (paths, shapes, curves) Indicates the presence of vector-drawn content such as charts, diagrams, or geometric shapes (e.g., from Adobe InDesign, LaTeX TikZ). These are invisible to `ExtractionResult.images` since they are not embedded as raster XObjects. Set to `true` when path count exceeds a heuristic threshold, signaling that downstream consumers may want to rasterize the page to capture this content. Only populated for PDFs; `nil` for other document types. |

---

#### PageStructure

Unified page structure for documents.

Supports different page types (PDF pages, PPTX slides, Excel sheets)
with character offset boundaries for chunk-to-page mapping.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `total_count` | `integer()` | — | Total number of pages/slides/sheets |
| `unit_type` | `PageUnitType` | — | Type of paginated unit |
| `boundaries` | `list(PageBoundary) \| nil` | `nil` | Character offset boundaries for each page Maps character ranges in the extracted content to page numbers. Used for chunk page range calculation. |
| `pages` | `list(PageInfo) \| nil` | `nil` | Detailed per-page metadata (optional, only when needed) |

---

#### PatternMatch

One detected PII span in the input text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `start` | `integer()` | — | Inclusive byte-offset start of the match in the source text. |
| `end` | `integer()` | — | Exclusive byte-offset end of the match. |
| `category` | `PiiCategory` | — | Category the match belongs to. |
| `text` | `String.t()` | — | Matched substring (owned copy — pattern engine returns owned data so the caller can free the original text if needed before replacement). |

---

#### PdfAnnotation

A PDF annotation extracted from a document page.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `annotation_type` | `PdfAnnotationType` | — | The type of annotation. |
| `content` | `String.t() \| nil` | `nil` | Text content of the annotation (e.g., comment text, link URL). |
| `page_number` | `integer()` | — | Page number where the annotation appears (1-indexed). |
| `bounding_box` | `BoundingBox \| nil` | `nil` | Bounding box of the annotation on the page. |

---

#### PdfConfig

PDF-specific configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `extract_images` | `boolean()` | `false` | Extract images from PDF |
| `extract_tables` | `boolean()` | `true` | Extract tables from PDF. When `true` (default), runs pdf_oxide's native grid detector and, if it finds nothing, falls back to the heuristic text-layer reconstruction in `pdf.oxide.table.extract_tables_heuristic`. Set to `false` to skip both passes — `tables` will then be empty in the result. |
| `passwords` | `list(String.t()) \| nil` | `nil` | List of passwords to try when opening encrypted PDFs |
| `extract_metadata` | `boolean()` | `true` | Extract PDF metadata |
| `hierarchy` | `HierarchyConfig \| nil` | `nil` | Hierarchy extraction configuration (None = hierarchy extraction disabled) |
| `extract_annotations` | `boolean()` | `false` | Extract PDF annotations (text notes, highlights, links, stamps). Default: false |
| `top_margin_fraction` | `float() \| nil` | `nil` | Top margin fraction (0.0–1.0) of page height to exclude headers/running heads. Default: 0.06 (6%) |
| `bottom_margin_fraction` | `float() \| nil` | `nil` | Bottom margin fraction (0.0–1.0) of page height to exclude footers/page numbers. Default: 0.05 (5%) |
| `allow_single_column_tables` | `boolean()` | `false` | Allow single-column pseudo tables in extraction results. By default, tables with fewer than 2 columns (layout-guided) or 3 columns (heuristic) are rejected. When `true`, the minimum column count is relaxed to 1, allowing single-column structured data (glossaries, itemized lists) to be emitted as tables. Other quality filters (density, sparsity, prose detection) still apply. |
| `ocr_inline_images` | `boolean()` | `false` | Perform OCR on inline images extracted from PDF pages and attach the recognized text to each `ExtractedImage.ocr_result`. Requires Tesseract to be available; if `ExtractionConfig.ocr` is `nil` the extractor falls back to `TesseractConfig.default()`. Per-image failures degrade gracefully (the image is returned without OCR text rather than failing the whole extraction). Default: `false`. |

### Functions

#### default()

**Signature:**

```elixir
def default()
```

---

#### PdfMetadata

PDF-specific metadata.

Contains metadata fields specific to PDF documents that are not in the common
`Metadata` structure. Common fields like title, authors, keywords, and dates
are at the `Metadata` level.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `pdf_version` | `String.t() \| nil` | `nil` | PDF version (e.g., "1.7", "2.0") |
| `producer` | `String.t() \| nil` | `nil` | PDF producer (application that created the PDF) |
| `is_encrypted` | `boolean() \| nil` | `nil` | Whether the PDF is encrypted/password-protected |
| `width` | `integer() \| nil` | `nil` | First page width in points (1/72 inch) |
| `height` | `integer() \| nil` | `nil` | First page height in points (1/72 inch) |
| `page_count` | `integer() \| nil` | `nil` | Total number of pages in the PDF document |

---

#### Plugin

Base trait that all plugins must implement.

This trait provides common functionality for plugin lifecycle management,
identification, and metadata.

### Thread Safety

All plugins must be `Send + Sync` to support concurrent usage across threads.

### Functions

#### name()

Returns the unique name/identifier for this plugin.

The name should be:

- Unique across all plugins
- Lowercase with hyphens (e.g., "my-custom-plugin")
- URL-safe characters only

**Signature:**

```elixir
def name()
```

#### version()

Returns the semantic version of this plugin.

Should follow semver format: `MAJOR.MINOR.PATCH`

Defaults to the kreuzberg crate version.

**Signature:**

```elixir
def version()
```

#### initialize()

Initialize the plugin.

Called once when the plugin is registered. Use this to:

- Load configuration
- Initialize resources (connections, caches, etc.)
- Validate dependencies

### Thread Safety

This method takes `&self` instead of `&mut self` to work with `Arc<dyn Plugin>`.
Plugins needing mutable state during initialization should use interior mutability
patterns (Mutex, RwLock, OnceCell, etc.).

**Errors:**

Should return an error if initialization fails. The plugin will not be
registered if this method returns an error.

Defaults to a no-op for stateless plugins.

**Signature:**

```elixir
def initialize()
```

#### shutdown()

Shutdown the plugin.

Called when the plugin is being unregistered or the application is shutting down.
Use this to:

- Close connections
- Flush caches
- Release resources

### Thread Safety

This method takes `&self` instead of `&mut self` to work with `Arc<dyn Plugin>`.
Plugins needing mutable state during shutdown should use interior mutability
patterns (Mutex, RwLock, etc.).

**Errors:**

Errors during shutdown are logged but don't prevent the shutdown process.

Defaults to a no-op for stateless plugins.

**Signature:**

```elixir
def shutdown()
```

#### description()

Optional plugin description for debugging and logging.

Defaults to empty string if not overridden.

**Signature:**

```elixir
def description()
```

#### author()

Optional plugin author information.

Defaults to empty string if not overridden.

**Signature:**

```elixir
def author()
```

---

#### PostProcessor

Trait for post-processor plugins.

Post-processors transform or enrich extraction results after the initial
extraction is complete. They can:

- Clean and normalize text
- Add metadata (language, keywords, entities)
- Split content into chunks
- Score quality
- Apply custom transformations

### Processing Order

Post-processors are executed in stage order:

1. **Early** - Language detection, entity extraction
2. **Middle** - Keyword extraction, token reduction
3. **Late** - Custom hooks, final validation

Within each stage, processors are executed in registration order.

### Error Handling

Post-processor errors are non-fatal by default - they're captured in metadata
and execution continues. To make errors fatal, return an error from `process()`.

### Thread Safety

Post-processors must be thread-safe (`Send + Sync`).

### Functions

#### process()

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

### Performance

This signature avoids unnecessary cloning of large extraction results by
taking a mutable reference instead of ownership. Processors modify the
result in place.

### Example - Language Detection

### Example - Text Cleaning

```rust
async fn process(&self, result: &mut ExtractionResult, config: &ExtractionConfig)
    -> Result<()> {
    // Remove excessive whitespace
    result.content = result
        .content
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    Ok(())
}
```

**Signature:**

```elixir
def process(result, config)
```

#### processing_stage()

Get the processing stage for this post-processor.

Determines when this processor runs in the pipeline.

**Returns:**

The `ProcessingStage` (Early, Middle, or Late).

**Signature:**

```elixir
def processing_stage()
```

#### should_process()

Optional: Check if this processor should run for a given result.

Allows conditional processing based on MIME type, metadata, or content.
Defaults to `true` (always run).

**Returns:**

`true` if the processor should run, `false` to skip.

**Signature:**

```elixir
def should_process(result, config)
```

#### estimated_duration_ms()

Optional: Estimate processing time in milliseconds.

Used for logging and debugging. Defaults to 0 (unknown).

**Returns:**

Estimated processing time in milliseconds.

**Signature:**

```elixir
def estimated_duration_ms(result)
```

#### priority()

Execution priority within the processing stage.

Higher values run first within the same `ProcessingStage`. Defaults to 50.
Use 0-49 for fallback processors, 50 for normal processors, and 51-255
for high-priority processors that should run early in their stage.

**Signature:**

```elixir
def priority()
```

---

#### PostProcessorConfig

Post-processor configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `boolean()` | `true` | Enable post-processors |
| `enabled_processors` | `list(String.t()) \| nil` | `nil` | Whitelist of processor names to run (None = all enabled) |
| `disabled_processors` | `list(String.t()) \| nil` | `nil` | Blacklist of processor names to skip (None = none disabled) |
| `enabled_set` | `list(String.t()) \| nil` | `nil` | Pre-computed AHashSet for O(1) enabled processor lookup |
| `disabled_set` | `list(String.t()) \| nil` | `nil` | Pre-computed AHashSet for O(1) disabled processor lookup |

### Functions

#### default()

**Signature:**

```elixir
def default()
```

---

#### PptxAppProperties

Application properties from docProps/app.xml for PPTX

Contains PowerPoint-specific document metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `application` | `String.t() \| nil` | `nil` | Application name (e.g., "Microsoft Office PowerPoint") |
| `app_version` | `String.t() \| nil` | `nil` | Application version |
| `total_time` | `integer() \| nil` | `nil` | Total editing time in minutes |
| `company` | `String.t() \| nil` | `nil` | Company name |
| `doc_security` | `integer() \| nil` | `nil` | Document security level |
| `scale_crop` | `boolean() \| nil` | `nil` | Scale crop flag |
| `links_up_to_date` | `boolean() \| nil` | `nil` | Links up to date flag |
| `shared_doc` | `boolean() \| nil` | `nil` | Shared document flag |
| `hyperlinks_changed` | `boolean() \| nil` | `nil` | Hyperlinks changed flag |
| `slides` | `integer() \| nil` | `nil` | Number of slides |
| `notes` | `integer() \| nil` | `nil` | Number of notes |
| `hidden_slides` | `integer() \| nil` | `nil` | Number of hidden slides |
| `multimedia_clips` | `integer() \| nil` | `nil` | Number of multimedia clips |
| `presentation_format` | `String.t() \| nil` | `nil` | Presentation format (e.g., "Widescreen", "Standard") |
| `slide_titles` | `list(String.t())` | `[]` | Slide titles |

---

#### PptxExtractionResult

PowerPoint (PPTX) extraction result.

Contains extracted slide content, metadata, and embedded images/tables.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String.t()` | — | Extracted text content from all slides |
| `metadata` | `PptxMetadata` | — | Presentation metadata |
| `slide_count` | `integer()` | — | Total number of slides |
| `image_count` | `integer()` | — | Total number of embedded images |
| `table_count` | `integer()` | — | Total number of tables |
| `images` | `list(ExtractedImage)` | — | Extracted images from the presentation |
| `page_structure` | `PageStructure \| nil` | `nil` | Slide structure with boundaries (when page tracking is enabled) |
| `page_contents` | `list(PageContent) \| nil` | `nil` | Per-slide content (when page tracking is enabled) |
| `document` | `DocumentStructure \| nil` | `nil` | Structured document representation |
| `hyperlinks` | `list(String.t())` | `/* serde(default) */` | Hyperlinks discovered in slides as (url, optional_label) pairs. |
| `office_metadata` | `map()` | `/* serde(default) */` | Office metadata extracted from docProps/core.xml and docProps/app.xml. Contains keys like "title", "author", "created_by", "subject", "keywords", "modified_by", "created_at", "modified_at", etc. |
| `revisions` | `list(DocumentRevision) \| nil` | `/* serde(default) */` | Slide comments as revisions. Each `<p:cm>` element in `ppt/comments/comment{N}.xml` becomes a `DocumentRevision { kind: Comment }` with author (resolved from `ppt/commentAuthors.xml`), ISO-8601 timestamp, and `RevisionAnchor.Slide { index }`. `nil` when no comment XML parts exist. |

---

#### PptxMetadata

PowerPoint presentation metadata.

Extracted from PPTX files containing slide counts and presentation details.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `slide_count` | `integer()` | — | Total number of slides in the presentation |
| `slide_names` | `list(String.t())` | `[]` | Names of slides (if available) |
| `image_count` | `integer() \| nil` | `nil` | Number of embedded images |
| `table_count` | `integer() \| nil` | `nil` | Number of tables |

---

#### ProcessingWarning

A non-fatal warning from a processing pipeline stage.

Captures errors from optional features that don't prevent extraction
but may indicate degraded results.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `source` | `String.t()` | — | The pipeline stage or feature that produced this warning (e.g., "embedding", "chunking", "language_detection", "output_format"). |
| `message` | `String.t()` | — | Human-readable description of what went wrong. |

---

#### PstMetadata

Outlook PST archive metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `message_count` | `integer()` | — | Total number of email messages found in the PST archive. |

---

#### QrBoundingBox

Pixel-space bounding box of a QR code inside its source image.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `x` | `integer()` | — | Horizontal pixel offset of the bounding box top-left corner. |
| `y` | `integer()` | — | Vertical pixel offset of the bounding box top-left corner. |
| `width` | `integer()` | — | Width of the bounding box in pixels. |
| `height` | `integer()` | — | Height of the bounding box in pixels. |

---

#### QrCode

One QR code decoded from an extracted image.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `payload` | `String.t()` | — | Decoded payload (text, URL, vCard string, …). |
| `confidence` | `float() \| nil` | `nil` | Detector-reported confidence in `[0.0, 1.0]`. `nil` when the decoder does not expose confidence (the default `rqrr` backend always reports `Some` because successful decode implies high confidence). |
| `bbox` | `QrBoundingBox \| nil` | `nil` | Bounding box of the QR code inside the source image, in pixel coordinates (`x`, `y` of the top-left corner; `width`, `height` of the rectangle). `nil` if the decoder did not report a bounding box. |

---

#### RakeParams

RAKE-specific parameters.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `min_word_length` | `integer()` | `1` | Minimum word length to consider (default: 1). |
| `max_words_per_phrase` | `integer()` | `3` | Maximum words in a keyword phrase (default: 3). |

### Functions

#### default()

**Signature:**

```elixir
def default()
```

---

#### RecognizedTable

Pre-computed table markdown for a table detection region.

Produced by the TATR-based table structure recognizer and surfaced as part of
layout-aware OCR results.  The struct lives here (under `layout-types`, pure-Rust)
so that consumers who do not enable `layout-detection` (ORT) can still reference
the type in their own code.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `detection_bbox` | `BBox` | — | Detection bbox that this table corresponds to (for matching). |
| `cells` | `list(list(String.t()))` | — | Table cells as a 2D vector (rows × columns). |
| `markdown` | `String.t()` | — | Rendered markdown table. |

---

#### RedactionConfig

**Since:** `v5.0.0`

Configuration for the redaction post-processor.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `categories` | `list(PiiCategory)` | `[]` | Categories to redact. Empty means "every category supported by the engine." |
| `strategy` | `RedactionStrategy` | `:mask` | Strategy applied to every match. |
| `ner` | `NerConfig \| nil` | `nil` | Optional NER backend — required to redact PERSON / ORGANIZATION / LOCATION categories (the pure-Rust pattern engine only covers regex-detectable PII). |
| `preserve_offsets` | `boolean()` | `true` | When `true`, chunk byte ranges are kept consistent with the rewritten content by adjusting `byte_start` / `byte_end` after replacement. When `false`, chunk byte ranges still refer to the *original* content offsets — useful when downstream consumers want to map findings back to the original document. |
| `custom_terms` | `list(RedactionTerm)` | `[]` | Arbitrary user-supplied literal terms to redact. Each term is treated as a regex hit against the document, surfacing as `PiiCategory.Custom(label)` in `RedactionFinding` where `label` is the per-term label (defaulting to the literal value itself). Case-insensitive by default; set `RedactionTerm.case_sensitive` for exact match. Use this when you need to redact tenant-specific tokens (employee IDs, project codes, internal product names) without writing a custom plugin. |
| `custom_patterns` | `list(RedactionPattern)` | `[]` | Arbitrary user-supplied regex patterns to redact. Same surfacing semantics as `custom_terms`: each hit becomes a `PiiCategory.Custom(label)` finding. Patterns are validated at config-construction time via `RedactionConfig.validate`. |

### Functions

#### default()

**Signature:**

```elixir
def default()
```

#### validate()

Validate user-supplied terms and patterns at config-construction time.

Compiles every `RedactionPattern.pattern` (with the case-insensitive
inline flag where applicable) and returns the first compilation error so
the caller can reject the config before the redaction pipeline runs.
Pure terms (regex-escaped) cannot fail to compile, but the function
still rejects empty values to avoid degenerate zero-length matches.

**Signature:**

```elixir
def validate()
```

---

#### RedactionFinding

One redaction event: which span was rewritten, why, and with what.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `start` | `integer()` | — | Byte-offset start in the original (pre-redaction) `ExtractionResult.content`. |
| `end` | `integer()` | — | Byte-offset end (exclusive) in the original `ExtractionResult.content`. |
| `category` | `PiiCategory` | — | PII category that fired this redaction. |
| `strategy` | `RedactionStrategy` | — | Strategy applied to this finding (mask, hash, token-replace, drop). |
| `replacement_token` | `String.t()` | — | String that replaced the original mention. Always present; for `Drop` the replacement is the empty string. |

---

#### RedactionPattern

One user-supplied regex pattern to redact.

The pattern is compiled with the Rust `regex` crate (no look-around). Case
sensitivity is encoded in the pattern via the `(?i)` inline flag when
`Self.case_sensitive` is `false`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `label` | `String.t()` | — | Custom category label surfaced in `RedactionFinding.category`. |
| `pattern` | `String.t()` | — | Regex pattern (Rust `regex` crate dialect — no look-around). |
| `case_sensitive` | `boolean()` | `/* serde(default) */` | When `true`, match case-sensitively; otherwise prepend `(?i)` to the regex. |

### Functions

#### labeled()

Build a pattern with the given label (case-insensitive by default).

**Signature:**

```elixir
def labeled(label, pattern)
```

---

#### RedactionReport

Audit report describing what the redaction processor found and how it replaced it.

The redactor returns this alongside the rewritten content so compliance, replay, and
audit-log consumers can see exactly what fired. Offsets are relative to the *original*
pre-redaction `content` and are intended for audit reconstruction only — the original
bytes are dropped at the end of the pipeline.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `findings` | `list(RedactionFinding)` | — | Individual redaction findings in original-source byte order. |
| `total_redacted` | `integer()` | — | Total number of redactions applied across the document. |

---

#### RedactionTerm

One user-supplied literal term to redact.

Matched as a regex-escaped substring (so callers do not need to escape
metacharacters themselves). Case-insensitive by default — set
`Self.case_sensitive` to `true` for exact byte-match semantics.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `label` | `String.t()` | — | Custom category label surfaced in `RedactionFinding.category`. |
| `value` | `String.t()` | — | Literal value to match. Regex metacharacters are escaped automatically. |
| `case_sensitive` | `boolean()` | `/* serde(default) */` | When `true`, match the value as-is; otherwise match ASCII-case-insensitively. |

### Functions

#### literal()

Build a term whose label is the literal value itself (case-insensitive).

**Signature:**

```elixir
def literal(value)
```

#### labeled()

Build a term with a custom label.

**Signature:**

```elixir
def labeled(label, value)
```

---

#### Renderer

Trait for document renderers that convert `InternalDocument` to output strings.

Renderers are typically stateless converters that transform the internal
document representation into a specific output format (Markdown, HTML,
Djot, plain text, etc.). They participate in the standard `Plugin`
lifecycle so custom renderers can be registered from any supported binding
language.

The format name is exposed via `Plugin.name`. For stateless renderers
the `Plugin` lifecycle methods (`version`, `initialize`, `shutdown`) all
take no-op defaults and need not be overridden.

### Thread Safety

Renderers must be `Send + Sync` (inherited from `Plugin`).

### Functions

#### render()

Render an `InternalDocument` to the output format.

**Returns:**

The rendered output as a string.

**Errors:**

Returns an error if rendering fails.

**Signature:**

```elixir
def render(doc)
```

---

#### RerankedDocument

A single document returned by the reranker, with its position in the input and score.

`index` maps back to the caller's original document list, so metadata arrays
(e.g. IDs, paths) can be reordered without passing them through the reranker.

Since v5.0.0.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `index` | `integer()` | — | Position of this document in the original input `documents` slice. |
| `score` | `float()` | — | Relevance score in `[0, 1]`. Higher means more relevant to the query. |
| `document` | `String.t()` | — | The document text. |

---

#### RerankerBackend

Trait for in-process reranker backend plugins.

Cross-encoders score `(query, document)` pairs jointly and return a
raw logit per document. The dispatcher in `rerank` applies
sigmoid to convert logits to `[0, 1]` scores, sorts descending by score,
and truncates to `top_k`.

Async to match the convention used by `EmbeddingBackend`
and other plugin traits. Host-language bridges wrap their synchronous
host callables in `spawn_blocking` or the equivalent.

### Thread safety

Backends must be `Send + Sync + 'static`. They are stored in
`Arc<dyn RerankerBackend>` and may be called concurrently from kreuzberg's
dispatcher. If the backend's underlying model is not thread-safe, the
backend itself must serialize access internally (e.g. via `Mutex<Inner>`).

### Contract

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

### Runtime

The synchronous `rerank` entry uses
`tokio.task.block_in_place` to await the trait's async `rerank`, which
requires a multi-thread tokio runtime. Callers running inside a
`current_thread` runtime must use `rerank_async` instead.

Since v5.0.0.

### Functions

#### rerank()

Score a list of documents against a query.

Returns one raw logit per document in the same order as the input.
The dispatcher applies sigmoid to convert to `[0, 1]` scores.

**Errors:**

Implementations should return `Plugin` for
backend-specific failures. The dispatcher validates the returned length
against `documents.len()` before sorting.

**Signature:**

```elixir
def rerank(query, documents)
```

---

#### RerankerConfig

Configuration for the reranking pipeline.

Controls which model to use, how many results to return, and download/cache
behavior for local ONNX models.

Since v5.0.0.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `RerankerModelType` | `:preset` | The reranker model to use (defaults to "balanced" preset if not specified). |
| `top_k` | `integer() \| nil` | `nil` | Return at most this many documents. `nil` returns all. Applied after sorting by score, so the highest-scoring documents are kept. |
| `batch_size` | `integer()` | `32` | Batch size for local ONNX cross-encoder inference. |
| `show_download_progress` | `boolean()` | `false` | Show model download progress (local ONNX path only). |
| `cache_dir` | `String.t() \| nil` | `nil` | Custom cache directory for model files. Defaults to `~/.cache/kreuzberg/rerankers/` if not specified. |
| `acceleration` | `AccelerationConfig \| nil` | `nil` | Hardware acceleration for the reranker ONNX model. Controls which execution provider (CPU, CUDA, CoreML, TensorRT) is used for local inference. Defaults to `nil` (auto-select per platform). |
| `max_rerank_duration_secs` | `integer() \| nil` | `nil` | Maximum wall-clock duration (in seconds) for a single `rerank()` call when using `RerankerModelType.Plugin`. Applies only to the in-process plugin path — protects against hung host-language backends. On timeout, the dispatcher returns `Plugin` instead of blocking forever. `nil` disables the timeout. The default (60 seconds) is conservative for common in-process inference; increase for large document sets on slow hardware. |

### Functions

#### default()

**Signature:**

```elixir
def default()
```

---

#### RerankerPreset

Metadata for a bundled reranker preset.

All string fields are owned `String` for FFI compatibility — instances are
safe to clone and pass across language boundaries.

Since v5.0.0.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String.t()` | — | Short identifier (catalog name, e.g. `"bge-reranker-base"`). |
| `model_repo` | `String.t()` | — | HuggingFace repository name for the model. |
| `model_file` | `String.t()` | — | Path to the ONNX model file within the repo. |
| `additional_files` | `list(String.t())` | `/* serde(default) */` | Sibling files that must be downloaded alongside `model_file`. Empty for most presets. Used by repos that split the weight blob — e.g. `rozgo/bge-reranker-v2-m3` ships the model in `model.onnx` plus a co-located `model.onnx.data` payload. |
| `max_length` | `integer()` | — | Maximum token sequence length the model supports. |
| `description` | `String.t()` | — | Human-readable description of the preset's intended use case. |

---

#### RevisionDelta

The content changes that make up a single revision.

For insertions and deletions the `content` field carries the added/removed
lines as `DiffLine.Added` / `DiffLine.Removed` entries. For format
changes, `content` is empty — the property diff is left as a TODO for a
later enrichment pass.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `list(DiffLine)` | `[]` | Line-level content changes for this revision. |
| `table_changes` | `list(CellChange)` | `[]` | Cell-level table changes for this revision. |

---

#### SecurityLimits

Configuration for security limits across extractors.

All limits are intentionally conservative to prevent DoS attacks
while still supporting legitimate documents.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_archive_size` | `integer()` | `524288000` | Maximum uncompressed size for archives (500 MB) |
| `max_compression_ratio` | `integer()` | `100` | Maximum compression ratio before flagging as potential bomb (100:1) |
| `max_files_in_archive` | `integer()` | `10000` | Maximum number of files in archive (10,000) |
| `max_nesting_depth` | `integer()` | `1024` | Maximum nesting depth for structures (100) |
| `max_entity_length` | `integer()` | `1048576` | Maximum length of any single XML entity / attribute / token (1 MiB). This is a per-token cap, NOT a total cap — billion-laughs class attacks where a single entity expands to hundreds of MB are caught here, while normal long text content (a paragraph, a CDATA block) is caught by `max_content_size` instead. |
| `max_content_size` | `integer()` | `104857600` | Maximum string growth per document (100 MB) |
| `max_iterations` | `integer()` | `10000000` | Maximum iterations per operation |
| `max_xml_depth` | `integer()` | `1024` | Maximum XML depth (100 levels) |
| `max_table_cells` | `integer()` | `100000` | Maximum cells per table (100,000) |

### Functions

#### default()

**Signature:**

```elixir
def default()
```

---

#### ServerConfig

API server configuration.

This struct holds all configuration options for the Kreuzberg API server,
including host/port settings, CORS configuration, and upload limits.

### Defaults

- `host`: "127.0.0.1" (localhost only)
- `port`: 8000
- `cors_origins`: empty vector (allows all origins)
- `max_request_body_bytes`: 104_857_600 (100 MB)
- `max_multipart_field_bytes`: 104_857_600 (100 MB)

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `host` | `String.t()` | — | Server host address (e.g., "127.0.0.1", "0.0.0.0") |
| `port` | `integer()` | — | Server port number |
| `cors_origins` | `list(String.t())` | `[]` | CORS allowed origins. Empty vector means allow all origins. If this is an empty vector, the server will accept requests from any origin. If populated with specific origins (e.g., `"<https://example.com"`>), only those origins will be allowed. |
| `max_request_body_bytes` | `integer()` | — | Maximum size of request body in bytes (default: 100 MB) |
| `max_multipart_field_bytes` | `integer()` | — | Maximum size of multipart fields in bytes (default: 100 MB) |

### Functions

#### default()

**Signature:**

```elixir
def default()
```

#### listen_addr()

Get the server listen address (host:port).

**Signature:**

```elixir
def listen_addr()
```

#### cors_allows_all()

Check if CORS allows all origins.

Returns `true` if the `cors_origins` vector is empty, meaning all origins
are allowed. Returns `false` if specific origins are configured.

**Signature:**

```elixir
def cors_allows_all()
```

#### is_origin_allowed()

Check if a given origin is allowed by CORS configuration.

Returns `true` if:

- CORS allows all origins (empty origins list), or
- The given origin is in the allowed origins list

**Signature:**

```elixir
def is_origin_allowed(origin)
```

#### max_request_body_mb()

Get maximum request body size in megabytes (rounded up).

**Signature:**

```elixir
def max_request_body_mb()
```

#### max_multipart_field_mb()

Get maximum multipart field size in megabytes (rounded up).

**Signature:**

```elixir
def max_multipart_field_mb()
```

---

#### StructuredData

Structured data (Schema.org, microdata, RDFa) block.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `data_type` | `StructuredDataType` | — | Type of structured data |
| `raw_json` | `String.t()` | — | Raw JSON string representation |
| `schema_type` | `String.t() \| nil` | `nil` | Schema type if detectable (e.g., "Article", "Event", "Product") |

---

#### StructuredDataResult

Result of parsing a structured data file (JSON, JSONL, YAML, or TOML).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String.t()` | — | The extracted text content, formatted for readability. |
| `format` | `String.t()` | — | The source format identifier (e.g. `"json"`, `"yaml"`, `"toml"`). |
| `metadata` | `map()` | — | Key-value metadata extracted from recognized text fields. |
| `text_fields` | `list(String.t())` | — | JSON paths of fields that were classified as text-bearing. |

---

#### StructuredExtractionConfig

Configuration for LLM-based structured data extraction.

Sends extracted document content to a VLM with a JSON schema,
returning structured data that conforms to the schema.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `schema` | `term()` | — | JSON Schema defining the desired output structure. |
| `schema_name` | `String.t()` | `/* serde(default) */` | Schema name passed to the LLM's structured output mode. |
| `schema_description` | `String.t() \| nil` | `/* serde(default) */` | Optional schema description for the LLM. |
| `strict` | `boolean()` | `/* serde(default) */` | Enable strict mode — output must exactly match the schema. |
| `prompt` | `String.t() \| nil` | `/* serde(default) */` | Custom Jinja2 extraction prompt template. When `nil`, a default template is used. Available template variables: - `{{ content }}` — The extracted document text. - `{{ schema }}` — The JSON schema as a formatted string. - `{{ schema_name }}` — The schema name. - `{{ schema_description }}` — The schema description (may be empty). |
| `llm` | `LlmConfig` | — | LLM configuration for the extraction. |

---

#### SummarizationConfig

**Since:** `v5.0.0`

Configuration for the summarisation post-processor.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `strategy` | `SummaryStrategy` | `:extractive` | Summarisation strategy. |
| `max_tokens` | `integer() \| nil` | `nil` | Maximum summary length in tokens. `nil` lets the backend pick a default. |
| `llm` | `LlmConfig \| nil` | `nil` | LLM configuration for the abstractive backend. Ignored when `strategy = Extractive`. Required when `strategy = Abstractive`. |

---

#### SupportedFormat

A supported document format entry.

Represents a file extension and its corresponding MIME type that Kreuzberg can process.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `extension` | `String.t()` | — | File extension (without leading dot), e.g., "pdf", "docx" |
| `mime_type` | `String.t()` | — | MIME type string, e.g., "application/pdf" |

---

#### Table

Extracted table structure.

Represents a table detected and extracted from a document (PDF, image, etc.).
Tables are converted to both structured cell data and Markdown format.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `cells` | `list(list(String.t()))` | `[]` | Table cells as a 2D vector (rows × columns) |
| `markdown` | `String.t()` | — | Markdown representation of the table |
| `page_number` | `integer()` | — | Page number where the table was found (1-indexed) |
| `bounding_box` | `BoundingBox \| nil` | `nil` | Bounding box of the table on the page (PDF coordinates: x0=left, y0=bottom, x1=right, y1=top). Only populated for PDF-extracted tables when position data is available. |

---

#### TableCell

Individual table cell with content and optional styling.

Future extension point for rich table support with cell-level metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String.t()` | — | Cell content as text |
| `row_span` | `integer()` | — | Row span (number of rows this cell spans) |
| `col_span` | `integer()` | — | Column span (number of columns this cell spans) |
| `is_header` | `boolean()` | — | Whether this is a header cell |

---

#### TableDiff

Cell-level changes for a pair of tables that share the same index.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `from_index` | `integer()` | — | Zero-based index of the table in both `a.tables` and `b.tables`. |
| `to_index` | `integer()` | — | Zero-based index in `b.tables` (equal to `from_index` for same-dimension tables). |
| `cell_changes` | `list(CellChange)` | — | Cell-level changes within the table. |

---

#### TableGrid

Structured table grid with cell-level metadata.

Stores row/column dimensions and a flat list of cells with position info.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `rows` | `integer()` | — | Number of rows in the table. |
| `cols` | `integer()` | — | Number of columns in the table. |
| `cells` | `list(GridCell)` | `[]` | All cells in row-major order. |

---

#### TesseractConfig

Tesseract OCR configuration.

Provides fine-grained control over Tesseract OCR engine parameters.
Most users can use the defaults, but these settings allow optimization
for specific document types (invoices, handwriting, etc.).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `language` | `String.t()` | `"eng"` | Language code (e.g., "eng", "deu", "fra") |
| `psm` | `integer()` | `3` | Page Segmentation Mode (0-13). Common values: - 3: Fully automatic page segmentation (native default) - 6: Assume a single uniform block of text (WASM default — avoids layout-analysis hang) - 11: Sparse text with no particular order |
| `output_format` | `String.t()` | `"markdown"` | Output format ("text" or "markdown") |
| `oem` | `integer()` | `3` | OCR Engine Mode (0-3). - 0: Legacy engine only - 1: Neural nets (LSTM) only (usually best) - 2: Legacy + LSTM - 3: Default (based on what's available) |
| `min_confidence` | `float()` | `0` | Minimum confidence threshold (0.0-100.0). Words with confidence below this threshold may be rejected or flagged. |
| `preprocessing` | `ImagePreprocessingConfig \| nil` | `nil` | Image preprocessing configuration. Controls how images are preprocessed before OCR. Can significantly improve quality for scanned documents or low-quality images. |
| `enable_table_detection` | `boolean()` | `true` | Enable automatic table detection and reconstruction |
| `table_min_confidence` | `float()` | `0` | Minimum confidence threshold for table detection (0.0-1.0) |
| `table_column_threshold` | `integer()` | `50` | Column threshold for table detection (pixels) |
| `table_row_threshold_ratio` | `float()` | `0.5` | Row threshold ratio for table detection (0.0-1.0) |
| `use_cache` | `boolean()` | `true` | Enable OCR result caching |
| `classify_use_pre_adapted_templates` | `boolean()` | `true` | Use pre-adapted templates for character classification |
| `language_model_ngram_on` | `boolean()` | `false` | Enable N-gram language model |
| `tessedit_dont_blkrej_good_wds` | `boolean()` | `true` | Don't reject good words during block-level processing |
| `tessedit_dont_rowrej_good_wds` | `boolean()` | `true` | Don't reject good words during row-level processing |
| `tessedit_enable_dict_correction` | `boolean()` | `true` | Enable dictionary correction |
| `tessedit_char_whitelist` | `String.t()` | `""` | Whitelist of allowed characters (empty = all allowed) |
| `tessedit_char_blacklist` | `String.t()` | `""` | Blacklist of forbidden characters (empty = none forbidden) |
| `tessedit_use_primary_params_model` | `boolean()` | `true` | Use primary language params model |
| `textord_space_size_is_variable` | `boolean()` | `true` | Variable-width space detection |
| `thresholding_method` | `boolean()` | `false` | Use adaptive thresholding method |

### Functions

#### default()

**Signature:**

```elixir
def default()
```

---

#### TextAnnotation

Inline text annotation — byte-range based formatting and links.

Annotations reference byte offsets into the node's text content,
enabling precise identification of formatted regions.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `start` | `integer()` | — | Start byte offset in the node's text content (inclusive). |
| `end` | `integer()` | — | End byte offset in the node's text content (exclusive). |
| `kind` | `AnnotationKind` | — | Annotation type. |

---

#### TextExtractionResult

Plain text and Markdown extraction result.

Contains the extracted text along with statistics and,
for Markdown files, structural elements like headers and links.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String.t()` | — | Extracted text content |
| `line_count` | `integer()` | — | Number of lines |
| `word_count` | `integer()` | — | Number of words |
| `character_count` | `integer()` | — | Number of characters |
| `headers` | `list(String.t()) \| nil` | `nil` | Markdown headers (text only, Markdown files only) |
| `links` | `list(list(String.t())) \| nil` | `nil` | Markdown links as (text, URL) tuples (Markdown files only) |
| `code_blocks` | `list(list(String.t())) \| nil` | `nil` | Code blocks as (language, code) tuples (Markdown files only) |

---

#### TextMetadata

Text/Markdown metadata.

Extracted from plain text and Markdown files. Includes word counts and,
for Markdown, structural elements like headers and links.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `line_count` | `integer()` | — | Number of lines in the document |
| `word_count` | `integer()` | — | Number of words |
| `character_count` | `integer()` | — | Number of characters |
| `headers` | `list(String.t()) \| nil` | `[]` | Markdown headers (headings text only, for Markdown files) |
| `links` | `list(list(String.t())) \| nil` | `[]` | Markdown links as (text, url) tuples (for Markdown files) |
| `code_blocks` | `list(list(String.t())) \| nil` | `[]` | Code blocks as (language, code) tuples (for Markdown files) |

---

#### TokenCounter

Per-category running counter for `RedactionStrategy.TokenReplace`.

### Functions

#### new()

Create a fresh counter with no previous state.

**Signature:**

```elixir
def new()
```

---

#### TokenReductionConfig

Configuration for the token-reduction pipeline.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `level` | `ReductionLevel` | `:moderate` | Reduction intensity level. |
| `language_hint` | `String.t() \| nil` | `nil` | ISO 639-1 language code hint for stopword selection (e.g. `"en"`, `"de"`). |
| `preserve_markdown` | `boolean()` | `false` | Preserve Markdown formatting tokens during reduction. |
| `preserve_code` | `boolean()` | `true` | Preserve code block contents unchanged. |
| `semantic_threshold` | `float()` | `0.3` | Cosine similarity threshold below which sentences are considered dissimilar. |
| `enable_parallel` | `boolean()` | `true` | Use Rayon parallel iterators for multi-core processing. |
| `use_simd` | `boolean()` | `true` | Use SIMD-optimized text scanning where available. |
| `custom_stopwords` | `map() \| nil` | `nil` | Per-language custom stopword lists (`language_code → stopword_list`). |
| `preserve_patterns` | `list(String.t())` | `[]` | Regex patterns whose matched text is always preserved unchanged. |
| `target_reduction` | `float() \| nil` | `nil` | Target fraction of text to retain (0.0–1.0); `nil` = no fixed target. |
| `enable_semantic_clustering` | `boolean()` | `false` | Group semantically similar sentences and emit only one per cluster. |

### Functions

#### default()

**Signature:**

```elixir
def default()
```

---

#### TokenReductionOptions

Token reduction configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `mode` | `String.t()` | — | Reduction mode: "off", "light", "moderate", "aggressive", "maximum" |
| `preserve_important_words` | `boolean()` | `true` | Preserve important words (capitalized, technical terms) |

### Functions

#### default()

**Signature:**

```elixir
def default()
```

---

#### TranscriptionConfig

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
| `enabled` | `boolean()` | `true` | Master switch. When false the block is ignored and audio files fall back to the normal "unsupported format" path. |
| `model` | `WhisperModel` | `:tiny` | Whisper model size to use. Smaller = faster + lower memory. `tiny` is the pragmatic default for first-time users and CI. |
| `language` | `String.t() \| nil` | `nil` | Optional language hint (ISO-639-1 code, e.g. "en", "de"). When `nil` (default) the engine may attempt auto-detection if supported. For deterministic production output, always set this explicitly. |
| `timestamps` | `boolean()` | `false` | Whether to emit segment-level timestamps in the result metadata. When true, `metadata["transcription.segments"]` will contain an array of `{start_ms, end_ms, text}` objects (if the engine supports it). |
| `max_duration_ms` | `integer() \| nil` | `nil` | Hard safety limit on input duration (milliseconds). Files longer than this are rejected *before* any decode or model work. Default: 30 minutes. Set to `nil` to disable (not recommended for untrusted input). |
| `max_bytes` | `integer() \| nil` | `nil` | Hard safety limit on input size (bytes). Default: 512 MiB. Protects against pathological or malicious uploads. |
| `timeout_ms` | `integer() \| nil` | `nil` | Wall-clock timeout for the entire transcription operation (ms). Includes model download (first time), decode, and inference. Default: 10 minutes. Uses `tokio.select!` so the async runtime is never blocked. |
| `model_cache_dir` | `String.t() \| nil` | `nil` | Override the directory used for Whisper model cache. When `nil`, uses the centralized resolver: `KREUZBERG_CACHE_DIR/transcription/whisper` or the platform default (`~/.cache/kreuzberg/transcription/whisper` on Linux, etc.). |
| `allow_network` | `boolean()` | `true` | Allow network access to download models from Hugging Face Hub. When `false`, only previously cached models may be used. Useful for air-gapped or fully offline deployments. |
| `verify_hash` | `boolean()` | `true` | Verify SHA256 checksums of downloaded model files (when known). Strongly recommended; disable only for debugging. |

### Functions

#### default()

**Signature:**

```elixir
def default()
```

---

#### Translation

Translation of the extracted content.

Holds the translated rendition of `ExtractionResult.content` and (when
`preserve_markup` was requested) the translated `formatted_content`. Chunks
are translated in place inside `ExtractionResult.chunks[*].content` rather
than duplicated here.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `target_lang` | `String.t()` | — | BCP-47 language tag the translation was produced into (e.g. `"de"`, `"fr-CA"`). |
| `source_lang` | `String.t() \| nil` | `nil` | BCP-47 source language. `nil` when the translation backend was asked to detect. |
| `content` | `String.t()` | — | Translated plain-text body. Matches the shape of `ExtractionResult.content`. |
| `formatted_content` | `String.t() \| nil` | `nil` | Translated markup body (Markdown / HTML / etc.) when `preserve_markup` was enabled on the config. `nil` otherwise. |

---

#### TranslationConfig

**Since:** `v5.0.0`

Configuration for the translation post-processor.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `target_lang` | `String.t()` | — | BCP-47 language tag for the target language (e.g. `"de"`, `"fr-CA"`). |
| `source_lang` | `String.t() \| nil` | `nil` | Optional explicit source language. `nil` asks the backend to auto-detect. |
| `preserve_markup` | `boolean()` | `/* serde(default) */` | Translate the formatted (Markdown/HTML) rendition alongside plain text when `formatted_content` is present. |
| `llm` | `LlmConfig` | — | LLM configuration used for translation. |

---

#### TreeSitterConfig

Configuration for tree-sitter language pack integration.

Controls grammar download behavior and code analysis options.

### Example (TOML)

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
| `enabled` | `boolean()` | `true` | Enable code intelligence processing (default: true). When `false`, tree-sitter analysis is completely skipped even if the config section is present. |
| `cache_dir` | `String.t() \| nil` | `nil` | Custom cache directory for downloaded grammars. When `nil`, uses the default: `~/.cache/tree-sitter-language-pack/v{version}/libs/`. |
| `languages` | `list(String.t()) \| nil` | `nil` | Languages to pre-download on init (e.g., `["python", "rust"]`). |
| `groups` | `list(String.t()) \| nil` | `nil` | Language groups to pre-download (e.g., `["web", "systems", "scripting"]`). |
| `process` | `TreeSitterProcessConfig` | — | Processing options for code analysis. |

### Functions

#### default()

**Signature:**

```elixir
def default()
```

---

#### TreeSitterProcessConfig

Processing options for tree-sitter code analysis.

Controls which analysis features are enabled when extracting code files.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `structure` | `boolean()` | `true` | Extract structural items (functions, classes, structs, etc.). Default: true. |
| `imports` | `boolean()` | `true` | Extract import statements. Default: true. |
| `exports` | `boolean()` | `true` | Extract export statements. Default: true. |
| `comments` | `boolean()` | `false` | Extract comments. Default: false. |
| `docstrings` | `boolean()` | `false` | Extract docstrings. Default: false. |
| `symbols` | `boolean()` | `false` | Extract symbol definitions. Default: false. |
| `diagnostics` | `boolean()` | `false` | Include parse diagnostics. Default: false. |
| `chunk_max_size` | `integer() \| nil` | `nil` | Maximum chunk size in bytes. `nil` disables chunking. |
| `content_mode` | `CodeContentMode` | `:chunks` | Content rendering mode for code extraction. |

### Functions

#### default()

**Signature:**

```elixir
def default()
```

---

#### Validator

Trait for validator plugins.

Validators check extraction results for quality, completeness, or correctness.
Unlike post-processors, validator errors **fail fast** - if a validator returns
an error, the extraction fails immediately.

### Use Cases

- **Quality Gates**: Ensure extracted content meets minimum quality standards
- **Compliance**: Verify content meets regulatory requirements
- **Content Filtering**: Reject documents containing unwanted content
- **Format Validation**: Verify extracted content structure
- **Security Checks**: Scan for malicious content

### Error Handling

Validator errors are **fatal** - they cause the extraction to fail and bubble up
to the caller. Use validators for hard requirements that must be met.

For non-fatal checks, use post-processors instead.

### Thread Safety

Validators must be thread-safe (`Send + Sync`).

### Functions

#### validate()

Validate an extraction result.

Check the extraction result and return `Ok(())` if valid, or an error
if validation fails.

**Returns:**

- `Ok(())` if validation passes
- `Err(...)` if validation fails (extraction will fail)

**Errors:**

- `KreuzbergError.Validation` - Validation failed
- Any other error type appropriate for the failure

### Example - Content Length Validation

```rust
async fn validate(&self, result: &ExtractionResult, config: &ExtractionConfig)
    -> Result<()> {
    let length = result.content.len();

    if length < self.min {
        return Err(KreuzbergError::validation(format!(
            "Content too short: {} < {} characters",
            length, self.min
        )));
    }

    if length > self.max {
        return Err(KreuzbergError::validation(format!(
            "Content too long: {} > {} characters",
            length, self.max
        )));
    }

    Ok(())
}
```

### Example - Quality Score Validation

```rust
async fn validate(&self, result: &ExtractionResult, config: &ExtractionConfig)
    -> Result<()> {
    // Check if quality_score exists in metadata
    let score = result.metadata
        .additional
        .get("quality_score")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    if score < self.min_score {
        return Err(KreuzbergError::validation(format!(
            "Quality score too low: {} < {}",
            score, self.min_score
        )));
    }

    Ok(())
}
```

### Example - Security Validation

```rust
async fn validate(&self, result: &ExtractionResult, config: &ExtractionConfig)
    -> Result<()> {
    // Check for blocked patterns
    for pattern in &self.blocked_patterns {
        if result.content.contains(pattern) {
            return Err(KreuzbergError::validation(format!(
                "Content contains blocked pattern: {}",
                pattern
            )));
        }
    }

    Ok(())
}
```

**Signature:**

```elixir
def validate(result, config)
```

#### should_validate()

Optional: Check if this validator should run for a given result.

Allows conditional validation based on MIME type, metadata, or content.
Defaults to `true` (always run).

**Returns:**

`true` if the validator should run, `false` to skip.

**Signature:**

```elixir
def should_validate(result, config)
```

#### priority()

Optional: Get the validation priority.

Higher priority validators run first. Useful for ordering validation checks
(e.g., run cheap validations before expensive ones).

Default priority is 50.

**Returns:**

Priority value (higher = runs earlier).

**Signature:**

```elixir
def priority()
```

---

#### XlsxAppProperties

Application properties from docProps/app.xml for XLSX

Contains Excel-specific document metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `application` | `String.t() \| nil` | `nil` | Application name (e.g., "Microsoft Excel") |
| `app_version` | `String.t() \| nil` | `nil` | Application version |
| `doc_security` | `integer() \| nil` | `nil` | Document security level |
| `scale_crop` | `boolean() \| nil` | `nil` | Scale crop flag |
| `links_up_to_date` | `boolean() \| nil` | `nil` | Links up to date flag |
| `shared_doc` | `boolean() \| nil` | `nil` | Shared document flag |
| `hyperlinks_changed` | `boolean() \| nil` | `nil` | Hyperlinks changed flag |
| `company` | `String.t() \| nil` | `nil` | Company name |
| `worksheet_names` | `list(String.t())` | `[]` | Worksheet names |

---

#### XmlExtractionResult

XML extraction result.

Contains extracted text content from XML files along with
structural statistics about the XML document.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String.t()` | — | Extracted text content (XML structure filtered out) |
| `element_count` | `integer()` | — | Total number of XML elements processed |
| `unique_elements` | `list(String.t())` | — | List of unique element names found (sorted) |

---

#### XmlMetadata

XML metadata extracted during XML parsing.

Provides statistics about XML document structure.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `element_count` | `integer()` | — | Total number of XML elements processed |
| `unique_elements` | `list(String.t())` | `[]` | List of unique element tag names (sorted) |

---

#### YakeParams

YAKE-specific parameters.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `window_size` | `integer()` | `2` | Window size for co-occurrence analysis (default: 2). Controls the context window for computing co-occurrence statistics. |

### Functions

#### default()

**Signature:**

```elixir
def default()
```

---

#### YearRange

Year range for bibliographic metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `min` | `integer() \| nil` | `nil` | Earliest (minimum) year in the range. |
| `max` | `integer() \| nil` | `nil` | Latest (maximum) year in the range. |
| `years` | `list(integer())` | `/* serde(default) */` | All individual years present in the collection. |

---

### Enums

#### ExecutionProviderType

ONNX Runtime execution provider type.

Determines which hardware backend is used for model inference.
`Auto` (default) selects the best available provider per platform.

| Value | Description |
|-------|-------------|
| `auto` | Auto-select: CoreML on macOS, CUDA on Linux, CPU elsewhere. |
| `cpu` | CPU execution provider (always available). |
| `core_ml` | Apple CoreML (macOS/iOS Neural Engine + GPU). |
| `cuda` | NVIDIA CUDA GPU acceleration. |
| `tensor_rt` | NVIDIA TensorRT (optimized CUDA inference). |

---

#### OutputFormat

Output format for extraction results.

Controls the format of the `content` field in `ExtractionResult`.
When set to `Markdown`, `Djot`, or `Html`, the output uses that format.
`Plain` returns the raw extracted text.
`Structured` returns JSON with full OCR element data including bounding
boxes and confidence scores.

| Value | Description |
|-------|-------------|
| `plain` | Plain text content only (default) |
| `markdown` | Markdown format |
| `djot` | Djot markup format |
| `html` | HTML format |
| `json` | JSON tree format with heading-driven sections. |
| `structured` | Structured JSON format with full OCR element metadata. |
| `custom` | Custom renderer registered via the RendererRegistry. The string is the renderer name (e.g., "docx", "latex"). — Fields: `0`: `String.t()` |

---

#### HtmlTheme

Built-in HTML theme selection.

| Value | Description |
|-------|-------------|
| `default` | Sensible defaults: system font stack, neutral colours, readable line measure. CSS custom properties (`--kb-*`) are all defined so user CSS can override individual values. |
| `git_hub` | GitHub Markdown-inspired palette and spacing. |
| `dark` | Dark background, light text. |
| `light` | Minimal light theme with generous whitespace. |
| `unstyled` | No built-in stylesheet emitted. CSS custom properties are still defined on `:root` so user stylesheets can reference `var(--kb-*)` tokens. |

---

#### TableModel

Which table structure recognition model to use.

Controls the model used for table cell detection within layout-detected
table regions. Wire format is snake_case in all serializers (JSON, TOML,
YAML).

| Value | Description |
|-------|-------------|
| `tatr` | TATR (Table Transformer) -- default, 30MB, DETR-based row/column detection. |
| `slanet_wired` | SLANeXT wired variant -- 365MB, optimized for bordered tables. |
| `slanet_wireless` | SLANeXT wireless variant -- 365MB, optimized for borderless tables. |
| `slanet_plus` | SLANet-plus -- 7.78MB, lightweight general-purpose. |
| `slanet_auto` | Classifier-routed SLANeXT: auto-select wired/wireless per table. Uses PP-LCNet classifier (6.78MB) + both SLANeXT variants (730MB total). |
| `disabled` | Disable table structure model inference entirely; use heuristic path only. |

---

#### NerBackendKind

NER backend selector.

| Value | Description |
|-------|-------------|
| `onnx` | gline-rs ONNX inference. Requires `ner-onnx` feature. Models download lazily from HuggingFace via `model_download.hf_download`. |
| `llm` | liter-llm zero-shot NER via structured-output prompts. Requires `ner-llm` feature. Useful when domain-specific categories outstrip the ONNX taxonomy. |

---

#### VlmFallbackPolicy

Policy controlling when VLM (Vision Language Model) OCR is used as a fallback.

This knob is syntactic sugar over the explicit `OcrPipelineConfig` stage
ordering. When `vlm_fallback` is set and `pipeline` is `nil`, an equivalent
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
| `disabled` | No VLM fallback (default). Behaves identically to the pre-policy single-backend mode. |
| `on_low_quality` | Try the classical OCR backend first. If the quality score is below `quality_threshold`, send the page to the VLM. `quality_threshold` is in the `[0.0, 1.0]` range produced by `calculate_quality_score`. A value of `0.5` is a reasonable starting point; calibrate with the Stage 0 benchmark harness. — Fields: `quality_threshold`: `float()` |
| `always` | Skip the classical OCR backend entirely. Every page is sent to the VLM. |

---

#### ChunkerType

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
| `text` | Generic whitespace- and punctuation-aware text splitter (default). |
| `markdown` | Markdown-aware splitter that preserves heading and code-block boundaries. |
| `yaml` | YAML-aware splitter that creates one chunk per top-level key. |
| `semantic` | Topic-aware chunker that splits at embedding-based topic shifts. |

---

#### ChunkSizing

How chunk size is measured.

Defaults to `Characters` (Unicode character count). When using token-based sizing,
chunks are sized by token count according to the specified tokenizer.

Token-based sizing uses HuggingFace tokenizers loaded at runtime. Any tokenizer
available on HuggingFace Hub can be used, including OpenAI-compatible tokenizers
(e.g., `Xenova/gpt-4o`, `Xenova/cl100k_base`).

| Value | Description |
|-------|-------------|
| `characters` | Size measured in Unicode characters (default). |
| `tokenizer` | Size measured in tokens from a HuggingFace tokenizer. — Fields: `model`: `String.t()`, `cache_dir`: `String.t()` |

---

#### EmbeddingModelType

Embedding model types supported by Kreuzberg.

| Value | Description |
|-------|-------------|
| `preset` | Use a preset model configuration (recommended) — Fields: `name`: `String.t()` |
| `custom` | Use a custom ONNX model from HuggingFace — Fields: `model_id`: `String.t()`, `dimensions`: `integer()` |
| `llm` | Provider-hosted embedding model via liter-llm. Uses the model specified in the nested `LlmConfig` (e.g., `"openai/text-embedding-3-small"`). — Fields: `llm`: `LlmConfig` |
| `plugin` | In-process embedding backend registered via the plugin system. The caller registers an `EmbeddingBackend` once (e.g. a wrapper around an already-loaded `llama-cpp-python`, `sentence-transformers`, or tuned ONNX model), then references it by name in config. Kreuzberg calls back into the registered backend during chunking and standalone embed requests — no HuggingFace download, no ONNX Runtime requirement, no HTTP sidecar. When this variant is selected, only the following `EmbeddingConfig` fields apply: `normalize` (post-call L2 normalization) and `max_embed_duration_secs` (dispatcher timeout). Model-loading fields (`batch_size`, `cache_dir`, `show_download_progress`, `acceleration`) are ignored — the host owns the model lifecycle. Semantic chunking falls back to `ChunkingConfig.max_characters` when this variant is used, since there is no preset to look a chunk-size ceiling up against — size your context window via `max_characters` directly. See `register_embedding_backend`. — Fields: `name`: `String.t()` |

---

#### RerankerModelType

Reranker model types supported by Kreuzberg.

Since v5.0.0.

| Value | Description |
|-------|-------------|
| `preset` | Use a preset cross-encoder model (recommended). — Fields: `name`: `String.t()` |
| `custom` | Use a custom ONNX cross-encoder from HuggingFace. — Fields: `model_id`: `String.t()`, `model_file`: `String.t()`, `additional_files`: `list(String.t())`, `max_length`: `integer()` |
| `llm` | Provider-hosted reranker via liter-llm (e.g. Cohere, Jina, Voyage). The model in the nested `LlmConfig` must be a rerank-capable model ID (e.g. `"cohere/rerank-english-v3.0"`). — Fields: `llm`: `LlmConfig` |
| `plugin` | In-process reranker registered via the plugin system. The caller registers a `RerankerBackend` once (e.g. a wrapper around a `sentence-transformers` cross-encoder or a provider client), then references it by name in config. Kreuzberg calls back into the registered backend — no HuggingFace download, no ONNX Runtime requirement. When this variant is selected, only `max_rerank_duration_secs` applies. Model-loading fields (`batch_size`, `cache_dir`, `show_download_progress`, `acceleration`) are ignored — the host owns the model lifecycle. See `register_reranker_backend`. — Fields: `name`: `String.t()` |

---

#### WhisperModel

Supported Whisper model sizes.

These map to published ONNX exports on Hugging Face (onnx-community or
similar orgs). The actual filenames and repos are resolved inside the
transcription engine.

| Value | Description |
|-------|-------------|
| `tiny` | ~39 MB, fastest, lowest quality. Good default for development and CI. |
| `base` | ~74 MB, reasonable quality/speed tradeoff. |
| `small` | ~244 MB, better accuracy. |
| `medium` | ~769 MB, high quality (slower, more memory). |
| `large_v3` | ~1550 MB, best quality (large-v3). Use only when latency is acceptable. |

---

#### CodeContentMode

Content rendering mode for code extraction.

Controls how extracted code content is represented in the `content` field
of `ExtractionResult`.

| Value | Description |
|-------|-------------|
| `chunks` | Use TSLP semantic chunks as content (default). |
| `raw` | Use raw source code as content. |
| `structure` | Emit function/class headings + docstrings (no code bodies). |

---

#### ListType

Type of list detection.

| Value | Description |
|-------|-------------|
| `bullet` | Bullet points (-, *, •, etc.) |
| `numbered` | Numbered lists (1., 2., etc.) |
| `lettered` | Lettered lists (a., b., A., B., etc.) |
| `indented` | Indented items |

---

#### OcrBackendType

OCR backend types.

| Value | Description |
|-------|-------------|
| `tesseract` | Tesseract OCR (native Rust binding) |
| `easy_ocr` | EasyOCR (Python-based, via FFI) |
| `paddle_ocr` | PaddleOCR (Python-based, via FFI) |
| `custom` | Custom/third-party OCR backend |

---

#### ProcessingStage

Processing stages for post-processors.

Post-processors are executed in stage order (Early → Middle → Late).
Use stages to control the order of post-processing operations.

| Value | Description |
|-------|-------------|
| `early` | Early stage - foundational processing. Use for: - Language detection - Character encoding normalization - Entity extraction (NER) - Text quality scoring |
| `middle` | Middle stage - content transformation. Use for: - Keyword extraction - Token reduction - Text summarization - Semantic analysis |
| `late` | Late stage - final enrichment. Use for: - Custom user hooks - Analytics/logging - Final validation - Output formatting |

---

#### ReductionLevel

Intensity level for the token-reduction pipeline.

| Value | Description |
|-------|-------------|
| `off` | No reduction applied; text is returned as-is. |
| `light` | Remove only the most common stopwords. |
| `moderate` | Balanced stopword removal and redundancy filtering. |
| `aggressive` | Aggressive filtering; may remove less common content words. |
| `maximum` | Maximum compression; prioritizes brevity over completeness. |

---

#### PdfAnnotationType

Type of PDF annotation.

| Value | Description |
|-------|-------------|
| `text` | Sticky note / text annotation |
| `highlight` | Highlighted text region |
| `link` | Hyperlink annotation |
| `stamp` | Rubber stamp annotation |
| `underline` | Underline text markup |
| `strike_out` | Strikeout text markup |
| `other` | Any other annotation type |

---

#### BlockType

Types of block-level elements in Djot.

| Value | Description |
|-------|-------------|
| `paragraph` | Standard prose paragraph. |
| `heading` | Section heading (level stored in `FormattedBlock.level`). |
| `blockquote` | Block quotation container. |
| `code_block` | Fenced or indented code block. |
| `list_item` | Individual item within a list. |
| `ordered_list` | Numbered (ordered) list container. |
| `bullet_list` | Unnumbered (bullet) list container. |
| `task_list` | Task / checkbox list container. |
| `definition_list` | Definition list container. |
| `definition_term` | Term part of a definition list entry. |
| `definition_description` | Description / definition part of a definition list entry. |
| `div` | Generic `div` container with optional attributes. |
| `section` | Logical section container, often associated with a heading. |
| `thematic_break` | Horizontal rule / thematic break. |
| `raw_block` | Raw content block in a specified format (e.g. HTML, LaTeX). |
| `math_display` | Display-mode mathematical expression. |

---

#### InlineType

Types of inline elements in Djot.

| Value | Description |
|-------|-------------|
| `text` | Plain text run. |
| `strong` | Bold / strong emphasis. |
| `emphasis` | Italic / regular emphasis. |
| `highlight` | Highlighted text (marker pen). |
| `subscript` | Subscript text. |
| `superscript` | Superscript text. |
| `insert` | Inserted text (tracked change). |
| `delete` | Deleted text (tracked change). |
| `code` | Inline code span. |
| `link` | Hyperlink with URL. |
| `image` | Inline image reference. |
| `span` | Generic inline span with optional attributes. |
| `math` | Inline mathematical expression. |
| `raw_inline` | Raw inline content in a specified format. |
| `footnote_ref` | Footnote reference marker. |
| `symbol` | Named symbol or emoji shortcode. |

---

#### RelationshipKind

Semantic kind of a relationship between document elements.

| Value | Description |
|-------|-------------|
| `footnote_reference` | Footnote marker -> footnote definition. |
| `citation_reference` | Citation marker -> bibliography entry. |
| `internal_link` | Internal anchor link (`#id`) -> target heading/element. |
| `caption` | Caption paragraph -> figure/table it describes. |
| `label` | Label -> labeled element (HTML `<label for>`, LaTeX `\label{}`). |
| `toc_entry` | TOC entry -> target section. |
| `cross_reference` | Cross-reference (LaTeX `\ref{}`, DOCX cross-reference field). |

---

#### ContentLayer

Content layer classification for document nodes.

Replaces separate body/furniture arrays with per-node granularity.

| Value | Description |
|-------|-------------|
| `body` | Main document body content. |
| `header` | Page/section header (running header). |
| `footer` | Page/section footer (running footer). |
| `footnote` | Footnote content. |

---

#### NodeContent

Tagged enum for node content. Each variant carries only type-specific data.

Uses `#[serde(tag = "node_type")]` to avoid "type" keyword collision in
Go/Java/TypeScript bindings.

| Value | Description |
|-------|-------------|
| `title` | Document title. — Fields: `text`: `String.t()` |
| `heading` | Section heading with level (1-6). — Fields: `level`: `integer()`, `text`: `String.t()` |
| `paragraph` | Body text paragraph. — Fields: `text`: `String.t()` |
| `list` | List container — children are `ListItem` nodes. — Fields: `ordered`: `boolean()` |
| `list_item` | Individual list item. — Fields: `text`: `String.t()` |
| `table` | Table with structured cell grid. — Fields: `grid`: `TableGrid` |
| `image` | Image reference. — Fields: `description`: `String.t()`, `image_index`: `integer()`, `src`: `String.t()` |
| `code` | Code block. — Fields: `text`: `String.t()`, `language`: `String.t()` |
| `quote` | Block quote — container, children carry the quoted content. |
| `formula` | Mathematical formula / equation. — Fields: `text`: `String.t()` |
| `footnote` | Footnote reference content. — Fields: `text`: `String.t()` |
| `group` | Logical grouping container (section, key-value area). `heading_level` + `heading_text` capture the section heading directly rather than relying on a first-child positional convention. — Fields: `label`: `String.t()`, `heading_level`: `integer()`, `heading_text`: `String.t()` |
| `page_break` | Page break marker. |
| `slide` | Presentation slide container — children are the slide's content nodes. — Fields: `number`: `integer()`, `title`: `String.t()` |
| `definition_list` | Definition list container — children are `DefinitionItem` nodes. |
| `definition_item` | Individual definition list entry with term and definition. — Fields: `term`: `String.t()`, `definition`: `String.t()` |
| `citation` | Citation or bibliographic reference. — Fields: `key`: `String.t()`, `text`: `String.t()` |
| `admonition` | Admonition / callout container (note, warning, tip, etc.). Children carry the admonition body content. — Fields: `kind`: `String.t()`, `title`: `String.t()` |
| `raw_block` | Raw block preserved verbatim from the source format. Used for content that cannot be mapped to a semantic node type (e.g. JSX in MDX, raw LaTeX in markdown, embedded HTML). — Fields: `format`: `String.t()`, `content`: `String.t()` |
| `metadata_block` | Structured metadata block (email headers, YAML frontmatter, etc.). — Fields: `entries`: `list(list(String.t()))` |

---

#### AnnotationKind

Types of inline text annotations.

| Value | Description |
|-------|-------------|
| `bold` | Bold (strong) text formatting. |
| `italic` | Italic (emphasis) text formatting. |
| `underline` | Underlined text. |
| `strikethrough` | Strikethrough text. |
| `code` | Inline code span. |
| `subscript` | Subscript text. |
| `superscript` | Superscript text. |
| `link` | Hyperlink annotation. — Fields: `url`: `String.t()`, `title`: `String.t()` |
| `highlight` | Highlighted text (PDF highlights, HTML `<mark>`). |
| `color` | Text color (CSS-compatible value, e.g. "#ff0000", "red"). — Fields: `value`: `String.t()` |
| `font_size` | Font size with units (e.g. "12pt", "1.2em", "16px"). — Fields: `value`: `String.t()` |
| `custom` | Extensible annotation for format-specific styling. — Fields: `name`: `String.t()`, `value`: `String.t()` |

---

#### EntityCategory

Standard entity categories produced by built-in NER backends.

The `Custom(String)` variant lets caller-supplied categories (e.g. LLM
schemas) flow through without losing fidelity to the consumer.

| Value | Description |
|-------|-------------|
| `person` | A person's name. |
| `organization` | A company, institution, or organisation name. |
| `location` | A geographic location (city, country, address). |
| `date` | A calendar date. |
| `time` | A time of day or duration. |
| `money` | A monetary amount with optional currency. |
| `percent` | A percentage value. |
| `email` | An email address. |
| `phone` | A phone number. |
| `url` | A URL or URI. |
| `custom` | A caller-supplied custom category label. — Fields: `0`: `String.t()` |

---

#### ExtractionMethod

How the extracted text was produced.

| Value | Description |
|-------|-------------|
| `native` | Text extracted directly from the document's native format (no OCR). |
| `ocr` | All text was obtained via OCR (e.g. scanned image-only PDF). |
| `mixed` | Text came from a combination of native extraction and OCR. |

---

#### ChunkType

Semantic structural classification of a text chunk.

Assigned by the heuristic classifier in `chunking.classifier`.
Defaults to `Unknown` when no rule matches.
Designed to be extended in future versions without breaking changes.

| Value | Description |
|-------|-------------|
| `heading` | Section heading or document title. |
| `party_list` | Party list: names, addresses, and signatories. |
| `definitions` | Definition clause ("X means…", "X shall mean…"). |
| `operative_clause` | Operative clause containing legal/contractual action verbs. |
| `signature_block` | Signature block with signatures, names, and dates. |
| `schedule` | Schedule, annex, appendix, or exhibit section. |
| `table_like` | Table-like content with aligned columns or repeated patterns. |
| `formula` | Mathematical formula or equation. |
| `code_block` | Code block or preformatted content. |
| `image` | Embedded or referenced image content. |
| `org_chart` | Organizational chart or hierarchy diagram. |
| `diagram` | Diagram, figure, or visual illustration. |
| `unknown` | Unclassified or mixed content. |

---

#### ImageKind

Heuristic classification of what an image likely depicts.

| Value | Description |
|-------|-------------|
| `photograph` | Photographic image (natural scene, photograph) |
| `diagram` | Technical or schematic diagram |
| `chart` | Chart, graph, or plot |
| `drawing` | Freehand or technical drawing |
| `text_block` | Text-heavy image (scanned text, document) |
| `decoration` | Decorative element or border |
| `logo` | Logo or brand mark |
| `icon` | Small icon |
| `tile_fragment` | Fragment of a larger tiled image (tile of a technical drawing) |
| `mask` | Mask or transparency map |
| `page_raster` | Full-page render produced during OCR preprocessing; used as a citation thumbnail. |
| `unknown` | Could not classify with reasonable confidence |

---

#### ResultFormat

Result-shape selection for extraction results.

Distinct from `OutputFormat` (which controls rendering — Plain, Markdown,
HTML, etc.). `ResultFormat` controls the *shape* of the result: a unified content
blob vs. an element-based decomposition.

| Value | Description |
|-------|-------------|
| `unified` | Unified format with all content in `content` field |
| `element_based` | Element-based format with semantic element extraction |

---

#### ElementType

Semantic element type classification.

Categorizes text content into semantic units for downstream processing.
Supports the element types commonly found in Unstructured documents.

| Value | Description |
|-------|-------------|
| `title` | Document title |
| `narrative_text` | Main narrative text body |
| `heading` | Section heading |
| `list_item` | List item (bullet, numbered, etc.) |
| `table` | Table element |
| `image` | Image element |
| `page_break` | Page break marker |
| `code_block` | Code block |
| `block_quote` | Block quote |
| `footer` | Footer text |
| `header` | Header text |

---

#### FormatMetadata

Format-specific metadata (discriminated union).

Only one format type can exist per extraction result. This provides
type-safe, clean metadata without nested optionals.

| Value | Description |
|-------|-------------|
| `pdf` | Metadata extracted from a PDF document. — Fields: `0`: `PdfMetadata` |
| `docx` | Metadata extracted from a DOCX Word document. — Fields: `0`: `DocxMetadata` |
| `excel` | Metadata extracted from an Excel spreadsheet. — Fields: `0`: `ExcelMetadata` |
| `email` | Metadata extracted from an email message (EML/MSG). — Fields: `0`: `EmailMetadata` |
| `pptx` | Metadata extracted from a PowerPoint presentation. — Fields: `0`: `PptxMetadata` |
| `archive` | Metadata extracted from an archive (ZIP, TAR, 7Z, etc.). — Fields: `0`: `ArchiveMetadata` |
| `image` | Metadata extracted from a raster or vector image. — Fields: `0`: `ImageMetadata` |
| `xml` | Metadata extracted from an XML document. — Fields: `0`: `XmlMetadata` |
| `text` | Metadata extracted from a plain-text file. — Fields: `0`: `TextMetadata` |
| `html` | Metadata extracted from an HTML document. — Fields: `0`: `HtmlMetadata` |
| `ocr` | Metadata produced by an OCR pipeline. — Fields: `0`: `OcrMetadata` |
| `csv` | Metadata extracted from a CSV or TSV file. — Fields: `0`: `CsvMetadata` |
| `bibtex` | Metadata extracted from a BibTeX bibliography file. — Fields: `0`: `BibtexMetadata` |
| `citation` | Metadata extracted from a citation file (RIS, PubMed, EndNote). — Fields: `0`: `CitationMetadata` |
| `fiction_book` | Metadata extracted from a FictionBook (FB2) e-book. — Fields: `0`: `FictionBookMetadata` |
| `dbf` | Metadata extracted from a dBASE (DBF) database file. — Fields: `0`: `DbfMetadata` |
| `jats` | Metadata extracted from a JATS (Journal Article Tag Suite) XML file. — Fields: `0`: `JatsMetadata` |
| `epub` | Metadata extracted from an EPUB e-book. — Fields: `0`: `EpubMetadata` |
| `pst` | Metadata extracted from an Outlook PST archive. — Fields: `0`: `PstMetadata` |
| `audio` | Metadata extracted from an audio or video file. — Fields: `0`: `AudioMetadata` |
| `code` | Code (tree-sitter analyzable source). The structured analysis result is exposed via `ExtractionResult.code_intelligence`; this variant only tags the format. |

---

#### TextDirection

Text direction enumeration for HTML documents.

| Value | Description |
|-------|-------------|
| `left_to_right` | Left-to-right text direction |
| `right_to_left` | Right-to-left text direction |
| `auto` | Automatic text direction detection |

---

#### LinkType

Link type classification.

| Value | Description |
|-------|-------------|
| `anchor` | Anchor link (#section) |
| `internal` | Internal link (same domain) |
| `external` | External link (different domain) |
| `email` | Email link (mailto:) |
| `phone` | Phone link (tel:) |
| `other` | Other link type |

---

#### ImageType

Image type classification.

| Value | Description |
|-------|-------------|
| `data_uri` | Data URI image |
| `inline_svg` | Inline SVG |
| `external` | External image URL |
| `relative` | Relative path image |

---

#### StructuredDataType

Structured data type classification.

| Value | Description |
|-------|-------------|
| `json_ld` | JSON-LD structured data |
| `microdata` | Microdata |
| `rdfa` | RDFa |

---

#### OcrBoundingGeometry

Bounding geometry for an OCR element.

Supports both axis-aligned rectangles (from Tesseract) and 4-point quadrilaterals
(from PaddleOCR and rotated text detection).

| Value | Description |
|-------|-------------|
| `rectangle` | Axis-aligned bounding box (typical for Tesseract output). — Fields: `left`: `integer()`, `top`: `integer()`, `width`: `integer()`, `height`: `integer()` |
| `quadrilateral` | 4-point quadrilateral for rotated/skewed text (PaddleOCR). Points are in clockwise order starting from top-left: `[top_left, top_right, bottom_right, bottom_left]` — Fields: `points`: `String.t()` |

---

#### OcrElementLevel

Hierarchical level of an OCR element.

Maps to Tesseract's page segmentation hierarchy and provides
equivalent semantics for PaddleOCR.

| Value | Description |
|-------|-------------|
| `word` | Individual word |
| `line` | Line of text (default for PaddleOCR) |
| `block` | Paragraph or text block |
| `page` | Page-level element |

---

#### PageUnitType

Type of paginated unit in a document.

Distinguishes between different types of "pages" (PDF pages, presentation slides, spreadsheet sheets).

| Value | Description |
|-------|-------------|
| `page` | Standard document pages (PDF, DOCX, images) |
| `slide` | Presentation slides (PPTX, ODP) |
| `sheet` | Spreadsheet sheets (XLSX, ODS) |

---

#### RedactionStrategy

Strategy applied when a PII match is rewritten.

| Value | Description |
|-------|-------------|
| `mask` | Replace the matched span with a fixed mask token (default `"[REDACTED]"`). |
| `hash` | Replace with a SHA-256 hash of the original value (truncated to 16 hex chars). Lets downstream consumers do equality joins without recovering the source. |
| `token_replace` | Replace with a per-category running token (`"[PERSON_1]"`, `"[PERSON_2]"`, …) so the same person referenced twice gets the same token within the document. |
| `drop` | Delete the matched span entirely. |

---

#### PiiCategory

PII categories the pattern engine recognises.

| Value | Description |
|-------|-------------|
| `email` | Email address (e.g. `user@example.com`). |
| `phone` | Phone number in any common format. |
| `ssn` | US Social Security Number. |
| `credit_card` | Payment card number (Visa, Mastercard, Amex, etc.). |
| `postal_code` | Postal / ZIP code. |
| `ip_address` | IPv4 or IPv6 address. |
| `iban` | International Bank Account Number. |
| `swift_bic` | SWIFT / BIC bank identifier code. |
| `date_of_birth` | Date of birth. |
| `person` | Person name, surfaced by the optional NER backend. |
| `organization` | Organization name, surfaced by the optional NER backend. |
| `location` | Location, surfaced by the optional NER backend. |
| `custom` | Caller-supplied custom category (e.g. internal employee IDs). Surfaced by the redaction engine when a hit comes from `RedactionConfig.custom_terms` or `RedactionConfig.custom_patterns`. The string is the label passed alongside the term/pattern. Use those fields rather than constructing `Custom` directly via the `categories` filter — the pattern engine cannot detect arbitrary text from a category name alone. — Fields: `0`: `String.t()` |

---

#### DiffLine

A single line in a unified-diff hunk.

Defined here (rather than only in `crate.diff`) so `RevisionDelta` can
reference it unconditionally, without requiring the `diff` Cargo feature.
`crate.diff` re-exports this type verbatim.

| Value | Description |
|-------|-------------|
| `context` | Unchanged context line. — Fields: `0`: `String.t()` |
| `added` | Line added in the "after" version. — Fields: `0`: `String.t()` |
| `removed` | Line removed from the "before" version. — Fields: `0`: `String.t()` |

---

#### RevisionKind

Semantic classification of a tracked change.

| Value | Description |
|-------|-------------|
| `insertion` | Text or content was inserted. |
| `deletion` | Text or content was deleted. |
| `format_change` | Run-level formatting (font, size, colour, …) was changed. |
| `comment` | A reviewer comment or annotation. |

---

#### RevisionAnchor

Best-effort document location for a revision.

| Value | Description |
|-------|-------------|
| `paragraph` | Body paragraph, identified by its zero-based index in the document flow. — Fields: `index`: `integer()` |
| `table_cell` | Cell inside a table. — Fields: `row`: `integer()`, `col`: `integer()`, `table_index`: `integer()` |
| `page` | Page, identified by its zero-based index. — Fields: `index`: `integer()` |
| `slide` | Presentation slide, identified by its zero-based index. — Fields: `index`: `integer()` |
| `sheet` | Spreadsheet cell or range, identified by sheet index and optional name. — Fields: `index`: `integer()`, `name`: `String.t()` |

---

#### SummaryStrategy

Summarisation strategy.

| Value | Description |
|-------|-------------|
| `extractive` | Pure-Rust extractive summary (TextRank over the chunk graph). Deterministic, fast, no external service required. |
| `abstractive` | Abstractive summary produced by liter-llm. Requires `liter-llm` feature and a configured `LlmConfig`. Token usage is captured in `ExtractionResult.llm_usage`. |

---

#### UriKind

Semantic classification of an extracted URI.

| Value | Description |
|-------|-------------|
| `hyperlink` | A clickable hyperlink (web URL, file link). |
| `image` | An image or media resource reference. |
| `anchor` | An internal anchor or cross-reference target. |
| `citation` | A citation or bibliographic reference (DOI, academic ref). |
| `reference` | A general reference (e.g. `\ref{}` in LaTeX, `:ref:` in RST). |
| `email` | An email address (`mailto:` link or bare email). |

---

#### RegionKind

Classification of a detected layout region that warrants VLM extraction.

Each variant maps to a specific prompt optimised for that content type.
The mapping is intentionally narrow — only region kinds for which VLM
extraction provides a clear quality benefit over classical suppression.

| Value | Description |
|-------|-------------|
| `figure` | A figure, diagram, chart, or image region. VLM prompt: describe the diagram / chart, including axis labels, legend entries, and any embedded text. |
| `dense_table` | A densely formatted or complex table that classical extraction garbles. VLM prompt: extract the table as GitHub-Flavoured Markdown. |
| `complex_layout` | A region whose layout the classical pipeline cannot handle (multi-column insets, heavily annotated forms, mixed text+diagram). VLM prompt: extract all text and structure as markdown, preserving reading order. |
| `caption` | A standalone image to be captioned (not extracted as figure markdown). VLM prompt: produce a single-sentence alt-text-style caption suitable for accessibility tooling and downstream indexing. Used by the captioning post-processor to populate `ExtractedImage.caption`. |

---

#### KeywordAlgorithm

Keyword algorithm selection.

| Value | Description |
|-------|-------------|
| `yake` | YAKE (Yet Another Keyword Extractor) - statistical approach |
| `rake` | RAKE (Rapid Automatic Keyword Extraction) - co-occurrence based |

---

#### PsmMode

Page Segmentation Mode for Tesseract OCR.

| Value | Description |
|-------|-------------|
| `osd_only` | Orientation and script detection only. |
| `auto_osd` | Automatic page segmentation with OSD. |
| `auto_only` | Automatic page segmentation without OSD or OCR. |
| `auto` | Fully automatic page segmentation with no OSD (default). |
| `single_column` | Assume a single column of text of variable sizes. |
| `single_block_vertical` | Assume a single uniform block of vertically aligned text. |
| `single_block` | Assume a single uniform block of text. |
| `single_line` | Treat the image as a single text line. |
| `single_word` | Treat the image as a single word. |
| `circle_word` | Treat the image as a single word in a circle. |
| `single_char` | Treat the image as a single character. |

---

#### PaddleLanguage

Supported languages in PaddleOCR.

Maps user-friendly language codes to paddle-ocr-rs language identifiers.

| Value | Description |
|-------|-------------|
| `english` | English |
| `chinese` | Simplified Chinese |
| `japanese` | Japanese |
| `korean` | Korean |
| `german` | German |
| `french` | French |
| `latin` | Latin script (covers most European languages) |
| `cyrillic` | Cyrillic (Russian and related) |
| `traditional_chinese` | Traditional Chinese |
| `thai` | Thai |
| `greek` | Greek |
| `east_slavic` | East Slavic (Russian, Ukrainian, Belarusian) |
| `arabic` | Arabic (Arabic, Persian, Urdu) |
| `devanagari` | Devanagari (Hindi, Marathi, Sanskrit, Nepali) |
| `tamil` | Tamil |
| `telugu` | Telugu |

---

#### LayoutClass

The 17 canonical document layout classes.

All model backends (RT-DETR, YOLO, etc.) map their native class IDs
to this shared set. Models with fewer classes (DocLayNet: 11, PubLayNet: 5)
map to the closest equivalent.

Wire format is snake_case in all serializers (JSON, TOML, YAML).

| Value | Description |
|-------|-------------|
| `caption` | Figure or table caption text. |
| `footnote` | Footnote or endnote text. |
| `formula` | Mathematical formula or equation. |
| `list_item` | A single item in a bulleted or numbered list. |
| `page_footer` | Running footer at the bottom of a page. |
| `page_header` | Running header at the top of a page. |
| `picture` | Image, chart, or other graphical element. |
| `section_header` | Section heading. |
| `table` | Data table. |
| `text` | Body text paragraph. |
| `title` | Document or chapter title. |
| `document_index` | Table of contents or index. |
| `code` | Source code block. |
| `checkbox_selected` | Checkbox in selected state. |
| `checkbox_unselected` | Checkbox in unselected state. |
| `form` | Form field or form element. |
| `key_value_region` | Key-value pair region (e.g. label + value in a form). |

---

### Errors

#### KreuzbergError

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
| `io` | A file system or I/O operation failed. These errors always bubble up unchanged. |
| `parsing` | Document parsing failed (e.g. corrupt file, unsupported format feature). |
| `ocr` | An OCR engine returned an error or produced unusable output. |
| `validation` | Invalid configuration or input parameters were supplied. |
| `cache` | A cache read or write operation failed. |
| `image_processing` | An image manipulation operation (resize, decode, DPI conversion) failed. |
| `serialization` | JSON or MessagePack serialization/deserialization failed. |
| `missing_dependency` | A required optional system dependency (e.g. `tesseract`) was not found. |
| `plugin` | A registered plugin returned an error during extraction. |
| `lock_poisoned` | An internal `Mutex` or `RwLock` was found in a poisoned state. |
| `unsupported_format` | The document's MIME type is not supported by any registered extractor. |
| `embedding` | The embedding model or embedding pipeline returned an error. |
| `reranking` | The reranker model or reranking pipeline returned an error. Since v5.0.0. |
| `transcription` | Audio/video transcription failed. |
| `timeout` | The extraction operation exceeded the configured time limit. |
| `cancelled` | The extraction was cancelled via a `CancellationToken`. |
| `security` | A security policy was violated (e.g. zip bomb, oversized archive). |
| `other` | A catch-all for uncommon errors that do not fit another variant. |

---

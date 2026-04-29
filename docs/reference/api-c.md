---
title: "C API Reference"
---

## C API Reference <span class="version-badge">v4.10.0-rc.8</span>

### Functions

#### kreuzberg_blake3_hash_bytes()

Hash arbitrary bytes with blake3, returning a 32-char hex string.

**Signature:**

```c
const char* kreuzberg_blake3_hash_bytes(const uint8_t* data);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `data` | `const uint8_t*` | Yes | The data |

**Returns:** `const char*`


---

#### kreuzberg_blake3_hash_file()

Hash a file's content with blake3 using streaming 64 KiB reads.

Returns a 32-char hex string (128 bits of blake3 output).

**Signature:**

```c
const char* kreuzberg_blake3_hash_file(const char* path);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `path` | `const char*` | Yes | Path to the file |

**Returns:** `const char*`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_fast_hash()

**Signature:**

```c
uint64_t kreuzberg_fast_hash(const uint8_t* data);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `data` | `const uint8_t*` | Yes | The data |

**Returns:** `uint64_t`


---

#### kreuzberg_validate_cache_key()

**Signature:**

```c
bool kreuzberg_validate_cache_key(const char* key);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `key` | `const char*` | Yes | The key |

**Returns:** `bool`


---

#### kreuzberg_validate_port()

Validate a port number for server configuration.

Port must be in the range 1-65535. While ports 1-1023 are privileged and may require
special permissions on some systems, they are still valid port numbers.

**Returns:**

`Ok(())` if the port is valid, or a `ValidationError` with details about valid ranges.

**Signature:**

```c
void kreuzberg_validate_port(uint16_t port);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `port` | `uint16_t` | Yes | The port number to validate |

**Returns:** `void`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_validate_host()

Validate a host/IP address string for server configuration.

Accepts valid IPv4 addresses (e.g., "127.0.0.1", "0.0.0.0"), valid IPv6 addresses
(e.g., ".1", "."), and hostnames (e.g., "localhost", "example.com").

**Returns:**

`Ok(())` if the host is valid, or a `ValidationError` with details about valid formats.

**Signature:**

```c
void kreuzberg_validate_host(const char* host);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `host` | `const char*` | Yes | The host/IP address string to validate |

**Returns:** `void`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_validate_cors_origin()

Validate a CORS (Cross-Origin Resource Sharing) origin URL.

Accepts valid HTTP/HTTPS URLs (e.g., "<https://example.com">) or the wildcard "*"
to allow all origins. URLs must start with "<http://"> or "<https://",> or be exactly "*".

**Returns:**

`Ok(())` if the origin is valid, or a `ValidationError` with details about valid formats.

**Signature:**

```c
void kreuzberg_validate_cors_origin(const char* origin);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `origin` | `const char*` | Yes | The CORS origin URL to validate |

**Returns:** `void`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_validate_upload_size()

Validate an upload size limit for server configuration.

Upload size must be greater than 0 (measured in bytes).

**Returns:**

`Ok(())` if the size is valid, or a `ValidationError` with details about constraints.

**Signature:**

```c
void kreuzberg_validate_upload_size(uintptr_t size);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `size` | `uintptr_t` | Yes | The maximum upload size in bytes to validate |

**Returns:** `void`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_validate_binarization_method()

Validate a binarization method string.

**Returns:**

`Ok(())` if the method is valid, or a `ValidationError` with details about valid options.

**Signature:**

```c
void kreuzberg_validate_binarization_method(const char* method);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `method` | `const char*` | Yes | The binarization method to validate (e.g., "otsu", "adaptive", "sauvola") |

**Returns:** `void`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_validate_token_reduction_level()

Validate a token reduction level string.

**Returns:**

`Ok(())` if the level is valid, or a `ValidationError` with details about valid options.

**Signature:**

```c
void kreuzberg_validate_token_reduction_level(const char* level);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `level` | `const char*` | Yes | The token reduction level to validate (e.g., "off", "light", "moderate") |

**Returns:** `void`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_validate_ocr_backend()

Validate an OCR backend string.

**Returns:**

`Ok(())` if the backend is valid, or a `ValidationError` with details about valid options.

**Signature:**

```c
void kreuzberg_validate_ocr_backend(const char* backend);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `backend` | `const char*` | Yes | The OCR backend to validate (e.g., "tesseract", "easyocr", "paddleocr") |

**Returns:** `void`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_validate_language_code()

Validate a language code (ISO 639-1 or 639-3 format).

Accepts both 2-letter ISO 639-1 codes (e.g., "en", "de") and
3-letter ISO 639-3 codes (e.g., "eng", "deu") for broader compatibility.

**Returns:**

`Ok(())` if the code is valid, or a `ValidationError` indicating an invalid language code.

**Signature:**

```c
void kreuzberg_validate_language_code(const char* code);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `code` | `const char*` | Yes | The language code to validate |

**Returns:** `void`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_validate_tesseract_psm()

Validate a tesseract Page Segmentation Mode (PSM).

**Returns:**

`Ok(())` if the PSM is valid, or a `ValidationError` with details about valid ranges.

**Signature:**

```c
void kreuzberg_validate_tesseract_psm(int32_t psm);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `psm` | `int32_t` | Yes | The PSM value to validate (0-13) |

**Returns:** `void`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_validate_tesseract_oem()

Validate a tesseract OCR Engine Mode (OEM).

**Returns:**

`Ok(())` if the OEM is valid, or a `ValidationError` with details about valid options.

**Signature:**

```c
void kreuzberg_validate_tesseract_oem(int32_t oem);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `oem` | `int32_t` | Yes | The OEM value to validate (0-3) |

**Returns:** `void`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_validate_output_format()

Validate a document extraction output format.

Accepts the following formats and aliases:
- "plain" or "text" for plain text output
- "markdown" or "md" for Markdown output
- "djot" for Djot markup format
- "html" for HTML output

**Returns:**

`Ok(())` if the format is valid, or a `ValidationError` with details about valid options.

**Signature:**

```c
void kreuzberg_validate_output_format(const char* format);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `format` | `const char*` | Yes | The output format to validate |

**Returns:** `void`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_validate_confidence()

Validate a confidence threshold value.

Confidence thresholds should be between 0.0 and 1.0 inclusive.

**Returns:**

`Ok(())` if the confidence is valid, or a `ValidationError` with details about valid ranges.

**Signature:**

```c
void kreuzberg_validate_confidence(double confidence);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `confidence` | `double` | Yes | The confidence threshold to validate |

**Returns:** `void`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_validate_dpi()

Validate a DPI (dots per inch) value.

DPI should be a positive integer, typically 72-600.

**Returns:**

`Ok(())` if the DPI is valid, or a `ValidationError` with details about valid ranges.

**Signature:**

```c
void kreuzberg_validate_dpi(int32_t dpi);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `dpi` | `int32_t` | Yes | The DPI value to validate |

**Returns:** `void`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_validate_chunking_params()

Validate chunk size parameters.

Checks that max_chars > 0 and max_overlap < max_chars.

**Returns:**

`Ok(())` if the parameters are valid, or a `ValidationError` with details about constraints.

**Signature:**

```c
void kreuzberg_validate_chunking_params(uintptr_t max_chars, uintptr_t max_overlap);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `max_chars` | `uintptr_t` | Yes | The maximum characters per chunk |
| `max_overlap` | `uintptr_t` | Yes | The maximum overlap between chunks |

**Returns:** `void`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_validate_llm_config_model()

Validate that an `LlmConfig` has a non-empty model string.

**Returns:**

`Ok(())` if the model is non-empty, or a `ValidationError` otherwise.

**Signature:**

```c
void kreuzberg_validate_llm_config_model(const char* model);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `model` | `const char*` | Yes | The model string to validate |

**Returns:** `void`

**Errors:** Returns `NULL` on error.


---

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

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `content` | `const uint8_t*` | Yes | The content to process |
| `mime_type` | `const char*` | Yes | The mime type |
| `config` | `KreuzbergExtractionConfig` | Yes | The configuration options |

**Returns:** `KreuzbergExtractionResult`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_batch_extract_file_sync()

Synchronous wrapper for `batch_extract_file`.

Uses the global Tokio runtime for optimal performance.
Only available with `tokio-runtime` (WASM has no filesystem).

**Signature:**

```c
KreuzbergExtractionResult* kreuzberg_batch_extract_file_sync(const char** items, KreuzbergExtractionConfig config);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `items` | `const char**` | Yes | The items |
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
KreuzbergExtractionResult* kreuzberg_batch_extract_bytes_sync(const char** items, KreuzbergExtractionConfig config);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `items` | `const char**` | Yes | The items |
| `config` | `KreuzbergExtractionConfig` | Yes | The configuration options |

**Returns:** `KreuzbergExtractionResult*`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_batch_extract_file()

Extract content from multiple files concurrently.

This function processes multiple files in parallel, automatically managing
concurrency to prevent resource exhaustion. The concurrency limit can be
configured via `ExtractionConfig.max_concurrent_extractions` or defaults
to `(num_cpus * 1.5).ceil()`.

Each file can optionally specify a `FileExtractionConfig` that overrides specific
fields from the batch-level `config`. Pass `NULL` for a file to use the batch defaults.
Batch-level settings like `max_concurrent_extractions` and `use_cache` are always
taken from the batch-level `config`.

  config to use the batch-level defaults for that file.
* `config` - Batch-level extraction configuration (provides defaults and batch settings)

**Returns:**

A vector of `ExtractionResult` in the same order as the input items.

**Errors:**

Individual file errors are captured in the result metadata. System errors
(IO, RuntimeError equivalents) will bubble up and fail the entire batch.

Simple usage with no per-file overrides:


Per-file configuration overrides:

**Signature:**

```c
KreuzbergExtractionResult* kreuzberg_batch_extract_file(const char** items, KreuzbergExtractionConfig config);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `items` | `const char**` | Yes | Vector of `(path, optional_file_config)` tuples. Pass `None` as the |
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

**Returns:**

A vector of `ExtractionResult` in the same order as the input items.

Simple usage with no per-item overrides:


Per-item configuration overrides:

**Signature:**

```c
KreuzbergExtractionResult* kreuzberg_batch_extract_bytes(const char** items, KreuzbergExtractionConfig config);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `items` | `const char**` | Yes | Vector of `(bytes, mime_type, optional_file_config)` tuples |
| `config` | `KreuzbergExtractionConfig` | Yes | Batch-level extraction configuration |

**Returns:** `KreuzbergExtractionResult*`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_is_valid_format_field()

Validates whether a field name is in the known formats registry.

This uses a pre-built hash set for O(1) lookups instead of linear search,
providing significant performance improvements for repeated validations.

**Returns:**

`true` if the field is in KNOWN_FORMATS, `false` otherwise.

**Signature:**

```c
bool kreuzberg_is_valid_format_field(const char* field);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `field` | `const char*` | Yes | The field name to validate |

**Returns:** `bool`


---

#### kreuzberg_validate_mime_type()

Validate that a MIME type is supported.

**Returns:**

The validated MIME type (may be normalized).

**Errors:**

Returns `KreuzbergError.UnsupportedFormat` if not supported.

**Signature:**

```c
const char* kreuzberg_validate_mime_type(const char* mime_type);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `mime_type` | `const char*` | Yes | The MIME type to validate |

**Returns:** `const char*`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_detect_or_validate()

Detect or validate MIME type.

If `mime_type` is provided, validates it. Otherwise, detects from `path`.

**Returns:**

The validated MIME type string.

**Signature:**

```c
const char* kreuzberg_detect_or_validate(const char* path, const char* mime_type);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `path` | `const char**` | No | Optional path to detect MIME type from |
| `mime_type` | `const char**` | No | Optional explicit MIME type to validate |

**Returns:** `const char*`

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

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `mime_type` | `const char*` | Yes | The MIME type to look up |

**Returns:** `const char**`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_list_supported_formats()

List all supported document formats.

Returns a list of all file extensions and their corresponding MIME types
that Kreuzberg can process. Derived from the centralized `FORMATS` registry.

The list is sorted alphabetically by file extension.

**Signature:**

```c
KreuzbergSupportedFormat* kreuzberg_list_supported_formats();
```

**Returns:** `KreuzbergSupportedFormat*`


---

#### kreuzberg_clear_processor_cache()

Clear the processor cache (primarily for testing when registry changes).

**Signature:**

```c
void kreuzberg_clear_processor_cache();
```

**Returns:** `void`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_transform_extraction_result_to_elements()

Transform an extraction result into semantic elements.

This function takes a reference to an ExtractionResult and generates
a vector of Element structs representing semantic blocks in the document.
It detects content sections, list items, page breaks, and other structural
elements to create an Unstructured-compatible element-based output.

Handles:
- PDF hierarchy → Title/Heading elements
- Multi-page documents with correct page numbers
- Table and Image extraction
- PageBreak interleaving
- Bounding box coordinates
- Paragraph detection for NarrativeText

**Returns:**

A vector of Elements with proper semantic types and metadata.

**Signature:**

```c
KreuzbergElement* kreuzberg_transform_extraction_result_to_elements(KreuzbergExtractionResult result);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `result` | `KreuzbergExtractionResult` | Yes | Reference to the ExtractionResult to transform |

**Returns:** `KreuzbergElement*`


---

#### kreuzberg_extract_email_content()

Extract email content from either .eml or .msg format

**Signature:**

```c
KreuzbergEmailExtractionResult* kreuzberg_extract_email_content(const uint8_t* data, const char* mime_type, uint32_t fallback_codepage);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `data` | `const uint8_t*` | Yes | The data |
| `mime_type` | `const char*` | Yes | The mime type |
| `fallback_codepage` | `uint32_t*` | No | The fallback codepage |

**Returns:** `KreuzbergEmailExtractionResult`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_cells_to_text()

Converts a 2D vector of cell strings into a GitHub-Flavored Markdown table.

# Behavior

- The first row is treated as the header row
- A separator row is inserted after the header
- Pipe characters (`|`) in cell content are automatically escaped with backslash
- Irregular tables (rows with varying column counts) are padded with empty cells to match the header
- Returns an empty string for empty input

**Returns:**

A `String` containing the GFM markdown table representation

Converts a 2D vector of cell strings into plain text with tab-separated columns.

# Behavior

- Rows are separated by newlines
- Cells within a row are separated by tab characters
- No pipe delimiters or separator rows (unlike markdown tables)
- Returns an empty string for empty input

**Returns:**

A `String` containing the plain text table representation

**Signature:**

```c
const char* kreuzberg_cells_to_text(const char*** cells);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `cells` | `const char***` | Yes | A slice of vectors representing table rows, where each inner vector contains cell values |

**Returns:** `const char*`


---

#### kreuzberg_cells_to_markdown()

**Signature:**

```c
const char* kreuzberg_cells_to_markdown(const char*** cells);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `cells` | `const char***` | Yes | The cells |

**Returns:** `const char*`


---

#### kreuzberg_djot_to_html()

Render djot content to HTML.

This function takes djot source text and renders it to HTML using jotdown's
built-in HTML renderer.

**Returns:**

A `Result` containing the rendered HTML string

**Signature:**

```c
const char* kreuzberg_djot_to_html(const char* djot_source);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `djot_source` | `const char*` | Yes | The djot markup text to render |

**Returns:** `const char*`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_dedup_text()

Deduplicate a list of text strings while preserving order.
Adjacent duplicates and near-duplicates are removed.

**Signature:**

```c
const char** kreuzberg_dedup_text(const char** texts);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `texts` | `const char**` | Yes | The texts |

**Returns:** `const char**`


---

#### kreuzberg_normalize_whitespace()

Normalize whitespace in a string.

- Collapses multiple consecutive spaces/tabs into a single space
- Preserves single newlines (paragraph breaks from \par)
- Collapses multiple consecutive newlines into a double newline
- Trims leading/trailing whitespace from each line
- Trims leading/trailing blank lines

**Signature:**

```c
const char* kreuzberg_normalize_whitespace(const char* s);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `s` | `const char*` | Yes | The s |

**Returns:** `const char*`


---

#### kreuzberg_register_default_extractors()

Register all built-in extractors with the global registry.

This function should be called once at application startup to register
the default extractors (PlainText, Markdown, XML, etc.).

**Note:** This is called automatically on first extraction operation.
Explicit calling is optional.

**Signature:**

```c
void kreuzberg_register_default_extractors();
```

**Returns:** `void`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_unregister_extractor()

Unregister a document extractor by name.

**Signature:**

```c
void kreuzberg_unregister_extractor(const char* name);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `name` | `const char*` | Yes | The name |

**Returns:** `void`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_list_extractors()

List names of all registered document extractors.

**Signature:**

```c
const char** kreuzberg_list_extractors();
```

**Returns:** `const char**`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_clear_extractors()

Remove all registered document extractors.

**Signature:**

```c
void kreuzberg_clear_extractors();
```

**Returns:** `void`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_unregister_ocr_backend()

Unregister an OCR backend by name.

Removes the OCR backend from the global registry and calls its `shutdown()` method.

**Returns:**

- `Ok(())` if the backend was unregistered or didn't exist
- `Err(...)` if the shutdown method failed

**Signature:**

```c
void kreuzberg_unregister_ocr_backend(const char* name);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `name` | `const char*` | Yes | Name of the OCR backend to unregister |

**Returns:** `void`

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

**Returns:** `void`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_list_post_processors()

List all registered post-processor names.

Returns a vector of all post-processor names currently registered in the
global registry.

**Returns:**

- `Ok(Vec<String>)` - Vector of post-processor names
- `Err(...)` if the registry lock is poisoned

**Signature:**

```c
const char** kreuzberg_list_post_processors();
```

**Returns:** `const char**`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_unregister_renderer()

Unregister a renderer by name.

**Signature:**

```c
void kreuzberg_unregister_renderer(const char* name);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `name` | `const char*` | Yes | The name |

**Returns:** `void`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_list_renderers()

List names of all registered renderers.

**Signature:**

```c
const char** kreuzberg_list_renderers();
```

**Returns:** `const char**`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_clear_renderers()

Remove all registered renderers.

**Signature:**

```c
void kreuzberg_clear_renderers();
```

**Returns:** `void`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_list_validators()

List names of all registered validators.

**Signature:**

```c
const char** kreuzberg_list_validators();
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

**Returns:** `void`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_sanitize_filename()

Sanitize a file path to return only the filename (no directory).

Prevents PII from appearing in traces.

**Signature:**

```c
const char* kreuzberg_sanitize_filename(const char* path);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `path` | `const char*` | Yes | Path to the file |

**Returns:** `const char*`


---

#### kreuzberg_sanitize_path()

Sanitize a file path to return only the filename.

Prevents PII (personally identifiable information) from appearing in
traces by only recording filenames instead of full paths.

**Signature:**

```c
const char* kreuzberg_sanitize_path(const char* path);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `path` | `const char*` | Yes | Path to the file |

**Returns:** `const char*`


---

#### kreuzberg_is_valid_utf8()

Validates bytes as UTF-8 without conversion to string slice.

Returns `true` if the bytes represent valid UTF-8, `false` otherwise.
This is useful when you only need to check validity without constructing a string.

**Returns:**

`true` if valid UTF-8, `false` otherwise.

# Performance

This function is optimized for early exit on invalid sequences.

**Signature:**

```c
bool kreuzberg_is_valid_utf8(const uint8_t* bytes);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `bytes` | `const uint8_t*` | Yes | The byte slice to validate |

**Returns:** `bool`


---

#### kreuzberg_clean_extracted_text()

**Signature:**

```c
const char* kreuzberg_clean_extracted_text(const char* text);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `const char*` | Yes | The text |

**Returns:** `const char*`


---

#### kreuzberg_reduce_tokens()

Reduces token count in text while preserving meaning and structure.

This function removes stopwords, redundancy, and applies compression techniques
based on the specified reduction level. Supports 64 languages with automatic
stopword removal and optional semantic clustering.

**Returns:**

Returns the reduced text with preserved structure (markdown, code blocks).

**Errors:**

Returns an error if the language hint is invalid or stopwords cannot be loaded.

**Signature:**

```c
const char* kreuzberg_reduce_tokens(const char* text, KreuzbergTokenReductionConfig config, const char* language_hint);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `const char*` | Yes | The input text to reduce |
| `config` | `KreuzbergTokenReductionConfig` | Yes | Configuration specifying reduction level and options |
| `language_hint` | `const char**` | No | Optional ISO 639-3 language code (e.g., "eng", "spa") |

**Returns:** `const char*`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_batch_reduce_tokens()

Reduces token count for multiple texts efficiently using parallel processing.

This function processes multiple texts in parallel using Rayon, providing
significant performance improvements for batch operations. All texts use the
same configuration and language hint for consistency.

**Returns:**

Returns a vector of reduced texts in the same order as the input.

**Errors:**

Returns an error if the language hint is invalid or stopwords cannot be loaded.

**Signature:**

```c
const char** kreuzberg_batch_reduce_tokens(const char** texts, KreuzbergTokenReductionConfig config, const char* language_hint);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `texts` | `const char**` | Yes | Slice of text references to reduce |
| `config` | `KreuzbergTokenReductionConfig` | Yes | Configuration specifying reduction level and options |
| `language_hint` | `const char**` | No | Optional ISO 639-3 language code (e.g., "eng", "spa") |

**Returns:** `const char**`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_bold()

Create a bold annotation for the given byte range.

**Signature:**

```c
KreuzbergTextAnnotation* kreuzberg_bold(uint32_t start, uint32_t end);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `start` | `uint32_t` | Yes | The start |
| `end` | `uint32_t` | Yes | The end |

**Returns:** `KreuzbergTextAnnotation`


---

#### kreuzberg_italic()

Create an italic annotation for the given byte range.

**Signature:**

```c
KreuzbergTextAnnotation* kreuzberg_italic(uint32_t start, uint32_t end);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `start` | `uint32_t` | Yes | The start |
| `end` | `uint32_t` | Yes | The end |

**Returns:** `KreuzbergTextAnnotation`


---

#### kreuzberg_underline()

Create an underline annotation for the given byte range.

**Signature:**

```c
KreuzbergTextAnnotation* kreuzberg_underline(uint32_t start, uint32_t end);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `start` | `uint32_t` | Yes | The start |
| `end` | `uint32_t` | Yes | The end |

**Returns:** `KreuzbergTextAnnotation`


---

#### kreuzberg_link()

Create a link annotation for the given byte range.

**Signature:**

```c
KreuzbergTextAnnotation* kreuzberg_link(uint32_t start, uint32_t end, const char* url, const char* title);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `start` | `uint32_t` | Yes | The start |
| `end` | `uint32_t` | Yes | The end |
| `url` | `const char*` | Yes | The URL to fetch |
| `title` | `const char**` | No | The title |

**Returns:** `KreuzbergTextAnnotation`


---

#### kreuzberg_code()

Create a code (inline) annotation for the given byte range.

**Signature:**

```c
KreuzbergTextAnnotation* kreuzberg_code(uint32_t start, uint32_t end);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `start` | `uint32_t` | Yes | The start |
| `end` | `uint32_t` | Yes | The end |

**Returns:** `KreuzbergTextAnnotation`


---

#### kreuzberg_strikethrough()

Create a strikethrough annotation for the given byte range.

**Signature:**

```c
KreuzbergTextAnnotation* kreuzberg_strikethrough(uint32_t start, uint32_t end);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `start` | `uint32_t` | Yes | The start |
| `end` | `uint32_t` | Yes | The end |

**Returns:** `KreuzbergTextAnnotation`


---

#### kreuzberg_subscript()

Create a subscript annotation for the given byte range.

**Signature:**

```c
KreuzbergTextAnnotation* kreuzberg_subscript(uint32_t start, uint32_t end);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `start` | `uint32_t` | Yes | The start |
| `end` | `uint32_t` | Yes | The end |

**Returns:** `KreuzbergTextAnnotation`


---

#### kreuzberg_superscript()

Create a superscript annotation for the given byte range.

**Signature:**

```c
KreuzbergTextAnnotation* kreuzberg_superscript(uint32_t start, uint32_t end);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `start` | `uint32_t` | Yes | The start |
| `end` | `uint32_t` | Yes | The end |

**Returns:** `KreuzbergTextAnnotation`


---

#### kreuzberg_font_size()

Create a font size annotation for the given byte range.

**Signature:**

```c
KreuzbergTextAnnotation* kreuzberg_font_size(uint32_t start, uint32_t end, const char* value);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `start` | `uint32_t` | Yes | The start |
| `end` | `uint32_t` | Yes | The end |
| `value` | `const char*` | Yes | The value |

**Returns:** `KreuzbergTextAnnotation`


---

#### kreuzberg_color()

Create a color annotation for the given byte range.

**Signature:**

```c
KreuzbergTextAnnotation* kreuzberg_color(uint32_t start, uint32_t end, const char* value);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `start` | `uint32_t` | Yes | The start |
| `end` | `uint32_t` | Yes | The end |
| `value` | `const char*` | Yes | The value |

**Returns:** `KreuzbergTextAnnotation`


---

#### kreuzberg_highlight()

Create a highlight annotation for the given byte range.

**Signature:**

```c
KreuzbergTextAnnotation* kreuzberg_highlight(uint32_t start, uint32_t end);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `start` | `uint32_t` | Yes | The start |
| `end` | `uint32_t` | Yes | The end |

**Returns:** `KreuzbergTextAnnotation`


---

#### kreuzberg_classify_uri()

Classify a URL string into the appropriate `UriKind`.

- `mailto:` → `Email`
- `#` prefix → `Anchor`
- everything else → `Hyperlink`

**Signature:**

```c
KreuzbergUriKind* kreuzberg_classify_uri(const char* url);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `url` | `const char*` | Yes | The URL to fetch |

**Returns:** `KreuzbergUriKind`


---

#### kreuzberg_safe_decode()

Decode raw bytes into UTF-8, using heuristics and fallback encodings when necessary.

The function prefers an explicit `encoding`, falls back to the cached guess, probes
an encoding detector, and finally tries a small curated list before returning a
mojibake-cleaned string.

**Signature:**

```c
const char* kreuzberg_safe_decode(const uint8_t* byte_data, const char* encoding);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `byte_data` | `const uint8_t*` | Yes | The byte data |
| `encoding` | `const char**` | No | The encoding |

**Returns:** `const char*`


---

#### kreuzberg_calculate_text_confidence()

Estimate how trustworthy a decoded string is on a 0.0–1.0 scale.

Scores close to 1.0 indicate mostly printable characters, whereas lower scores
point to mojibake, control characters, or suspicious character mixes.

**Signature:**

```c
double kreuzberg_calculate_text_confidence(const char* text);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `const char*` | Yes | The text |

**Returns:** `double`


---

#### kreuzberg_create_string_buffer_pool()

Create a pre-configured string buffer pool for batch processing.

**Returns:**

A pool configured for text accumulation with reasonable defaults.

**Signature:**

```c
KreuzbergStringBufferPool* kreuzberg_create_string_buffer_pool(uintptr_t pool_size, uintptr_t buffer_capacity);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `pool_size` | `uintptr_t` | Yes | Maximum number of buffers to keep in the pool |
| `buffer_capacity` | `uintptr_t` | Yes | Initial capacity for each buffer in bytes |

**Returns:** `KreuzbergStringBufferPool`


---

#### kreuzberg_create_byte_buffer_pool()

Create a pre-configured byte buffer pool for batch processing.

**Returns:**

A pool configured for binary data handling with reasonable defaults.

**Signature:**

```c
KreuzbergByteBufferPool* kreuzberg_create_byte_buffer_pool(uintptr_t pool_size, uintptr_t buffer_capacity);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `pool_size` | `uintptr_t` | Yes | Maximum number of buffers to keep in the pool |
| `buffer_capacity` | `uintptr_t` | Yes | Initial capacity for each buffer in bytes |

**Returns:** `KreuzbergByteBufferPool`


---

#### kreuzberg_openapi_json()

Generate OpenAPI JSON schema.

Returns the complete OpenAPI 3.1 specification as a JSON string.

**Signature:**

```c
const char* kreuzberg_openapi_json();
```

**Returns:** `const char*`


---

#### kreuzberg_serve_default()

Start the API server with default host and port.

Defaults: host = "127.0.0.1", port = 8000

Uses config file discovery (searches current/parent directories for kreuzberg.toml/yaml/json).
Validates plugins at startup to help diagnose configuration issues.

**Signature:**

```c
void kreuzberg_serve_default();
```

**Returns:** `void`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_chunk_text()

Split text into chunks with optional page boundary tracking.

This is the primary API function for chunking text. It supports both plain text
and Markdown with configurable chunk size, overlap, and page boundary mapping.

**Returns:**

A ChunkingResult containing all chunks and their metadata.

**Signature:**

```c
KreuzbergChunkingResult* kreuzberg_chunk_text(const char* text, KreuzbergChunkingConfig config, KreuzbergPageBoundary* page_boundaries);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `const char*` | Yes | The text to split into chunks |
| `config` | `KreuzbergChunkingConfig` | Yes | Chunking configuration (max size, overlap, type) |
| `page_boundaries` | `KreuzbergPageBoundary**` | No | Optional page boundary markers for mapping chunks to pages |

**Returns:** `KreuzbergChunkingResult`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_chunk_text_with_heading_source()

Chunk text with an optional separate markdown source for heading context resolution.

When `heading_source` is provided, it is used instead of `text` for building the
heading map. This is needed when `text` is plain text (no markdown headings) but
the original document had headings that were stripped during rendering.

**Signature:**

```c
KreuzbergChunkingResult* kreuzberg_chunk_text_with_heading_source(const char* text, KreuzbergChunkingConfig config, KreuzbergPageBoundary* page_boundaries, const char* heading_source);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `const char*` | Yes | The text |
| `config` | `KreuzbergChunkingConfig` | Yes | The configuration options |
| `page_boundaries` | `KreuzbergPageBoundary**` | No | The page boundaries |
| `heading_source` | `const char**` | No | The heading source |

**Returns:** `KreuzbergChunkingResult`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_chunk_texts_batch()

Batch process multiple texts with the same configuration.

This convenience function applies the same chunking configuration to multiple
texts in sequence.

**Returns:**

A vector of ChunkingResult objects, one per input text.

**Errors:**

Returns an error if chunking any individual text fails.

**Signature:**

```c
KreuzbergChunkingResult* kreuzberg_chunk_texts_batch(const char** texts, KreuzbergChunkingConfig config);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `texts` | `const char**` | Yes | Slice of text strings to chunk |
| `config` | `KreuzbergChunkingConfig` | Yes | Chunking configuration to apply to all texts |

**Returns:** `KreuzbergChunkingResult*`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_chunk_semantic()

Split text into semantically coherent chunks.

Splits text into fine-grained segments, detects structural (and optionally
embedding-based) topic boundaries, then merges segments into chunks that
respect those boundaries and the configured size budget.

**Signature:**

```c
KreuzbergChunkingResult* kreuzberg_chunk_semantic(const char* text, KreuzbergChunkingConfig config, KreuzbergPageBoundary* page_boundaries);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `const char*` | Yes | The text |
| `config` | `KreuzbergChunkingConfig` | Yes | The configuration options |
| `page_boundaries` | `KreuzbergPageBoundary**` | No | The page boundaries |

**Returns:** `KreuzbergChunkingResult`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_normalize()

L2-normalize a vector.

**Signature:**

```c
float* kreuzberg_normalize(float* v);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `v` | `float*` | Yes | The v |

**Returns:** `float*`


---

#### kreuzberg_get_preset()

Get a preset by name.

**Signature:**

```c
const char** kreuzberg_get_preset(const char* name);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `name` | `const char*` | Yes | The name |

**Returns:** `const char**`


---

#### kreuzberg_list_presets()

List all available preset names.

**Signature:**

```c
const char** kreuzberg_list_presets();
```

**Returns:** `const char**`


---

#### kreuzberg_warm_model()

Eagerly download and cache an embedding model without returning the handle.

This triggers the same download and initialization as `get_or_init_engine`
but discards the result, making it suitable for cache-warming scenarios
where the caller doesn't need to use the model immediately.

**Note**: This function downloads AND initializes the ONNX model, which
requires ONNX Runtime and uses significant memory. For download-only
scenarios (e.g., init containers), use `download_model` instead.

**Signature:**

```c
void kreuzberg_warm_model(KreuzbergEmbeddingModelType model_type, const char* cache_dir);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `model_type` | `KreuzbergEmbeddingModelType` | Yes | The embedding model type |
| `cache_dir` | `const char**` | No | The cache dir |

**Returns:** `void`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_download_model()

Download an embedding model's files without initializing ONNX Runtime.

Downloads the model files (ONNX model, tokenizer, config) from HuggingFace
to the cache directory. Subsequent calls to `warm_model` or
`get_or_init_engine` will find the files cached and skip the download step.

This is ideal for init containers or CI environments where you want to
pre-populate the cache without loading models into memory.

**Signature:**

```c
void kreuzberg_download_model(KreuzbergEmbeddingModelType model_type, const char* cache_dir);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `model_type` | `KreuzbergEmbeddingModelType` | Yes | The embedding model type |
| `cache_dir` | `const char**` | No | The cache dir |

**Returns:** `void`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_calculate_optimal_dpi()

Calculate optimal DPI with min/max constraints

**Signature:**

```c
int32_t kreuzberg_calculate_optimal_dpi(double page_width, double page_height, int32_t target_dpi, int32_t max_dimension, int32_t min_dpi, int32_t max_dpi);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `page_width` | `double` | Yes | The page width |
| `page_height` | `double` | Yes | The page height |
| `target_dpi` | `int32_t` | Yes | The target dpi |
| `max_dimension` | `int32_t` | Yes | The max dimension |
| `min_dpi` | `int32_t` | Yes | The min dpi |
| `max_dpi` | `int32_t` | Yes | The max dpi |

**Returns:** `int32_t`


---

#### kreuzberg_detect_languages()

Detect languages in text using whatlang.

Returns a list of detected language codes (ISO 639-3 format).
Returns `NULL` if no languages could be detected with sufficient confidence.

**Signature:**

```c
const char*** kreuzberg_detect_languages(const char* text, KreuzbergLanguageDetectionConfig config);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `const char*` | Yes | The text to analyze for language detection |
| `config` | `KreuzbergLanguageDetectionConfig` | Yes | Optional configuration for language detection |

**Returns:** `const char***`

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

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `const char*` | Yes | The text to extract keywords from |
| `config` | `KreuzbergKeywordConfig` | Yes | Keyword extraction configuration |

**Returns:** `KreuzbergKeyword*`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_compute_hash()

Compute a blake3 hash string from input data.

Returns a 32-character hex string (128 bits of blake3 output).

**Signature:**

```c
const char* kreuzberg_compute_hash(const char* data);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `data` | `const char*` | Yes | The data |

**Returns:** `const char*`


---

#### kreuzberg_render_pdf_page_to_png()

Render a single PDF page to a PNG-encoded byte buffer.

**Errors:**

Returns an error if the PDF is invalid, the page index is out of bounds,
or if the page fails to render.

**Signature:**

```c
const uint8_t* kreuzberg_render_pdf_page_to_png(const uint8_t* pdf_bytes, uintptr_t page_index, int32_t dpi, const char* password);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `pdf_bytes` | `const uint8_t*` | Yes | The pdf bytes |
| `page_index` | `uintptr_t` | Yes | The page index |
| `dpi` | `int32_t*` | No | The dpi |
| `password` | `const char**` | No | The password |

**Returns:** `const uint8_t*`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_extract_text_from_pdf()

**Signature:**

```c
const char* kreuzberg_extract_text_from_pdf(const uint8_t* pdf_bytes);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `pdf_bytes` | `const uint8_t*` | Yes | The pdf bytes |

**Returns:** `const char*`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_serialize_to_toon()

Serialize an `ExtractionResult` to TOON (Token-Oriented Object Notation).

TOON is a token-efficient alternative to JSON for LLM prompts.
Losslessly convertible to/from JSON but uses fewer tokens.

**Signature:**

```c
const char* kreuzberg_serialize_to_toon(KreuzbergExtractionResult result);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `result` | `KreuzbergExtractionResult` | Yes | The extraction result |

**Returns:** `const char*`

**Errors:** Returns `NULL` on error.


---

#### kreuzberg_serialize_to_json()

Serialize an `ExtractionResult` to pretty-printed JSON.

**Signature:**

```c
const char* kreuzberg_serialize_to_json(KreuzbergExtractionResult result);
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `result` | `KreuzbergExtractionResult` | Yes | The extraction result |

**Returns:** `const char*`

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

#### KreuzbergAnchorProperties

Properties for anchored drawings.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `behind_doc` | `bool` | — | Behind doc |
| `layout_in_cell` | `bool` | — | Layout in cell |
| `relative_height` | `int64_t*` | `NULL` | Relative height |
| `position_h` | `const char**` | `NULL` | Position h |
| `position_v` | `const char**` | `NULL` | Position v |
| `wrap_type` | `const char*` | — | Wrap type |


---

#### KreuzbergApiDoc

OpenAPI documentation structure.

Defines all endpoints, request/response schemas, and examples
for the Kreuzberg document extraction API.


---

#### KreuzbergApiState

API server state.

Holds the default extraction configuration loaded from config file
(via discovery or explicit path). Per-request configs override these defaults.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `default_config` | `KreuzbergExtractionConfig` | — | Default extraction configuration |
| `extraction_service` | `const char*` | — | Tower service for extraction requests. Wrapped in `Arc<Mutex>` because `BoxCloneService` is `Send` but not `Sync`, while `ApiState` must be `Clone + Sync` for Axum's state requirement. The lock is held only long enough to clone the service. |


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
| `file_count` | `uintptr_t` | — | Total number of files in the archive |
| `file_list` | `const char**` | `NULL` | List of file paths within the archive |
| `total_size` | `uintptr_t` | — | Total uncompressed size in bytes |
| `compressed_size` | `uintptr_t*` | `NULL` | Compressed size in bytes (if available) |


---

#### KreuzbergBBox

Bounding box in original image coordinates (x1, y1) top-left, (x2, y2) bottom-right.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `x1` | `float` | — | X1 |
| `y1` | `float` | — | Y1 |
| `x2` | `float` | — | X2 |
| `y2` | `float` | — | Y2 |


---

#### KreuzbergBatchExtractFilesParams

Request parameters for batch file extraction.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `paths` | `const char**` | — | Paths to files to extract |
| `config` | `void**` | `NULL` | Extraction configuration (JSON object) |
| `pdf_password` | `const char**` | `NULL` | Password for encrypted PDFs |
| `file_configs` | `void****` | `NULL` | Per-file extraction configuration overrides (parallel array to paths). Each entry is either null (use default) or a FileExtractionConfig JSON object. |
| `response_format` | `const char**` | `NULL` | Wire format for the response: "json" (default) or "toon" |


---

#### KreuzbergBibtexMetadata

BibTeX bibliography metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `entry_count` | `uintptr_t` | — | Number of entries in the bibliography. |
| `citation_keys` | `const char**` | `NULL` | Citation keys |
| `authors` | `const char**` | `NULL` | Authors |
| `year_range` | `KreuzbergYearRange*` | `NULL` | Year range (year range) |
| `entry_types` | `void**` | `NULL` | Entry types |


---

#### KreuzbergByteBufferPool

Convenience type alias for a pooled Vec<u8>.


---

#### KreuzbergCacheClearResponse

Cache clear response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `directory` | `const char*` | — | Cache directory path |
| `removed_files` | `uintptr_t` | — | Number of files removed |
| `freed_mb` | `double` | — | Space freed in MB |


---

#### KreuzbergCacheStatsResponse

Cache statistics response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `directory` | `const char*` | — | Cache directory path |
| `total_files` | `uintptr_t` | — | Total number of cache files |
| `total_size_mb` | `double` | — | Total cache size in MB |
| `available_space_mb` | `double` | — | Available disk space in MB |
| `oldest_file_age_days` | `double` | — | Age of oldest file in days |
| `newest_file_age_days` | `double` | — | Age of newest file in days |


---

#### KreuzbergCacheWarmParams

Request parameters for cache warm (model download).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `all_embeddings` | `bool` | — | Download all embedding model presets |
| `embedding_model` | `const char**` | `NULL` | Specific embedding preset name to download (e.g. "balanced", "speed", "quality") |


---

#### KreuzbergChunk

A text chunk with optional embedding and metadata.

Chunks are created when chunking is enabled in `ExtractionConfig`. Each chunk
contains the text content, optional embedding vector (if embedding generation
is configured), and metadata about its position in the document.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `const char*` | — | The text content of this chunk. |
| `chunk_type` | `KreuzbergChunkType` | — | Semantic structural classification of this chunk. Assigned by the heuristic classifier based on content patterns and heading context. Defaults to `ChunkType.Unknown` when no rule matches. |
| `embedding` | `float**` | `NULL` | Optional embedding vector for this chunk. Only populated when `EmbeddingConfig` is provided in chunking configuration. The dimensionality depends on the chosen embedding model. |
| `metadata` | `KreuzbergChunkMetadata` | — | Metadata about this chunk's position and properties. |


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
| `first_page` | `uintptr_t*` | `NULL` | First page number this chunk spans (1-indexed). Only populated when page tracking is enabled in extraction configuration. |
| `last_page` | `uintptr_t*` | `NULL` | Last page number this chunk spans (1-indexed, equal to first_page for single-page chunks). Only populated when page tracking is enabled in extraction configuration. |
| `heading_context` | `KreuzbergHeadingContext*` | `NULL` | Heading context when using Markdown chunker. Contains the heading hierarchy this chunk falls under. Only populated when `ChunkerType.Markdown` is used. |


---

#### KreuzbergChunkRequest

Chunk request with text and configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text` | `const char*` | — | Text to chunk (must not be empty) |
| `config` | `const char**` | `NULL` | Optional chunking configuration |
| `chunker_type` | `const char*` | — | Chunker type (text, markdown, yaml, or semantic) |


---

#### KreuzbergChunkResponse

Chunk response with chunks and metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `chunks` | `const char**` | — | List of chunks |
| `chunk_count` | `uintptr_t` | — | Total number of chunks |
| `config` | `const char*` | — | Configuration used for chunking |
| `input_size_bytes` | `uintptr_t` | — | Input text size in bytes |
| `chunker_type` | `const char*` | — | Chunker type used for chunking |


---

#### KreuzbergChunkTextParams

Request parameters for text chunking.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text` | `const char*` | — | Text content to split into chunks |
| `max_characters` | `uintptr_t*` | `NULL` | Maximum characters per chunk (default: 2000) |
| `overlap` | `uintptr_t*` | `NULL` | Number of overlapping characters between chunks (default: 100) |
| `chunker_type` | `const char**` | `NULL` | Chunker type: "text", "markdown", "yaml", or "semantic" (default: "text") |
| `topic_threshold` | `float*` | `NULL` | Topic threshold for semantic chunking (0.0-1.0, default: 0.75) |


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
| `citation_count` | `uintptr_t` | — | Number of citations |
| `format` | `const char**` | `NULL` | Format |
| `authors` | `const char**` | `NULL` | Authors |
| `year_range` | `KreuzbergYearRange*` | `NULL` | Year range (year range) |
| `dois` | `const char**` | `NULL` | Dois |
| `keywords` | `const char**` | `NULL` | Keywords |


---

#### KreuzbergCommonPdfMetadata

Common metadata fields extracted from a PDF.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `title` | `const char**` | `NULL` | Title |
| `subject` | `const char**` | `NULL` | Subject |
| `authors` | `const char***` | `NULL` | Authors |
| `keywords` | `const char***` | `NULL` | Keywords |
| `created_at` | `const char**` | `NULL` | Created at |
| `modified_at` | `const char**` | `NULL` | Modified at |
| `created_by` | `const char**` | `NULL` | Created by |


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
| `strip_repeating_text` | `bool` | `true` | Enable the heuristic cross-page repeating text detector. When `true` (default), text that repeats verbatim across a supermajority of pages is classified as furniture and stripped.  Disable this if brand names or repeated headings are being incorrectly removed by the heuristic. Note: when a layout-detection model is active, the model may independently classify page-header / page-footer regions as furniture on a per-page basis. To preserve those regions, set `include_headers = true` and/or `include_footers = true` in addition to disabling this flag. Primarily affects PDF extraction. Default: `true`. |
| `include_watermarks` | `bool` | `false` | Include watermark text in extraction output. - PDF: Keeps watermark artifacts and arXiv identifiers. - Other formats: No effect currently. Default: `false` (watermarks are stripped). |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergContentFilterConfig kreuzberg_default();
```


---

#### KreuzbergContributorRole

JATS contributor with role.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `const char*` | — | The name |
| `role` | `const char**` | `NULL` | Role |


---

#### KreuzbergCsvMetadata

CSV/TSV file metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `row_count` | `uintptr_t` | — | Number of rows |
| `column_count` | `uintptr_t` | — | Number of columns |
| `delimiter` | `const char**` | `NULL` | Delimiter |
| `has_header` | `bool` | — | Whether header |
| `column_types` | `const char***` | `NULL` | Column types |


---

#### KreuzbergCustomProperties

Custom properties from docProps/custom.xml

Maps property names to their values. Values are converted to JSON types
based on the VT (Variant Type) specified in the XML.


---

#### KreuzbergDbfFieldInfo

dBASE field information.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `const char*` | — | The name |
| `field_type` | `const char*` | — | Field type |


---

#### KreuzbergDbfMetadata

dBASE (DBF) file metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `record_count` | `uintptr_t` | — | Number of records |
| `field_count` | `uintptr_t` | — | Number of fields |
| `fields` | `KreuzbergDbfFieldInfo*` | `NULL` | Fields |


---

#### KreuzbergDetectMimeTypeParams

Request parameters for MIME type detection.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `path` | `const char*` | — | Path to the file |
| `use_content` | `bool` | — | Use content-based detection (default: true) |


---

#### KreuzbergDetectResponse

MIME type detection response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `mime_type` | `const char*` | — | Detected MIME type |
| `filename` | `const char**` | `NULL` | Original filename (if provided) |


---

#### KreuzbergDetectedBoundary

A detected structural boundary in the text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `byte_offset` | `uintptr_t` | — | Byte offset of the start of the line in the original text. |
| `is_header` | `bool` | — | Whether this boundary looks like a header/section title. |


---

#### KreuzbergDetectionResult

Page-level detection result containing all detections and page metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `page_width` | `uint32_t` | — | Page width |
| `page_height` | `uint32_t` | — | Page height |
| `detections` | `KreuzbergLayoutDetection*` | — | Detections |


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
| `tables` | `const char**` | — | Extracted tables as structured data |
| `images` | `KreuzbergDjotImage*` | — | Extracted images with metadata |
| `links` | `KreuzbergDjotLink*` | — | Extracted links with URLs |
| `footnotes` | `KreuzbergFootnote*` | — | Footnote definitions |
| `attributes` | `const char**` | — | Attributes mapped by element identifier (if present) |


---

#### KreuzbergDjotImage

Image element in Djot.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `src` | `const char*` | — | Image source URL or path |
| `alt` | `const char*` | — | Alternative text |
| `title` | `const char**` | `NULL` | Optional title |
| `attributes` | `const char**` | `NULL` | Element attributes |


---

#### KreuzbergDjotLink

Link element in Djot.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | `const char*` | — | Link URL |
| `text` | `const char*` | — | Link text content |
| `title` | `const char**` | `NULL` | Optional title |
| `attributes` | `const char**` | `NULL` | Element attributes |


---

#### KreuzbergDoclingCompatResponse

OpenWebUI "Docling" engine response format.

Returned by `POST /v1/convert/file` for docling-serve compatibility.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `document` | `const char*` | — | Converted document content |
| `status` | `const char*` | — | Processing status |


---

#### KreuzbergDocumentExtractor

Trait for document extractor plugins.

Implement this trait to add support for new document formats or to override
built-in extraction behavior with custom logic.

# Return Type

Extractors return `InternalDocument`, a flat intermediate representation.
The pipeline converts this into the public `ExtractionResult` via the
derivation step.

# Priority System

When multiple extractors support the same MIME type, the registry selects
the extractor with the highest priority value. Use this to:
- Override built-in extractors (priority > 50)
- Provide fallback extractors (priority < 50)
- Implement specialized extractors for specific use cases

Default priority is 50.

# Thread Safety

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
const char* kreuzberg_extract_bytes(const uint8_t* content, const char* mime_type, KreuzbergExtractionConfig config);
```

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
const char* kreuzberg_extract_file(const char* path, const char* mime_type, KreuzbergExtractionConfig config);
```

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

###### kreuzberg_priority()

Get the priority of this extractor.

Higher priority extractors are preferred when multiple extractors
support the same MIME type.

# Priority Guidelines

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

###### kreuzberg_as_sync_extractor()

Attempt to get a reference to this extractor as a SyncExtractor.

Returns None if the extractor doesn't support synchronous extraction.
This is used for WASM and other sync-only environments.

**Signature:**

```c
KreuzbergSyncExtractor* kreuzberg_as_sync_extractor();
```


---

#### KreuzbergDocumentNode

A single node in the document tree.

Each node has deterministic `id`, typed `content`, optional `parent`/`children`
for tree structure, and metadata like page number, bounding box, and content layer.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `const char*` | — | Deterministic identifier (hash of content + position). |
| `content` | `KreuzbergNodeContent` | — | Node content — tagged enum, type-specific data only. |
| `parent` | `uint32_t*` | `NULL` | Parent node index (`NULL` = root-level node). |
| `children` | `uint32_t*` | — | Child node indices in reading order. |
| `content_layer` | `KreuzbergContentLayer` | — | Content layer classification. |
| `page` | `uint32_t*` | `NULL` | Page number where this node starts (1-indexed). |
| `page_end` | `uint32_t*` | `NULL` | Page number where this node ends (for multi-page tables/sections). |
| `bbox` | `const char**` | `NULL` | Bounding box in document coordinates. |
| `annotations` | `KreuzbergTextAnnotation*` | — | Inline annotations (formatting, links) on this node's text content. Only meaningful for text-carrying nodes; empty for containers. |
| `attributes` | `void**` | `NULL` | Format-specific key-value attributes. Extensible bag for data that doesn't warrant a typed field: CSS classes, LaTeX environment names, Excel cell formulas, slide layout names, etc. |


---

#### KreuzbergDocumentRelationship

A resolved relationship between two nodes in the document tree.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `source` | `uint32_t` | — | Source node index (the referencing node). |
| `target` | `uint32_t` | — | Target node index (the referenced node). |
| `kind` | `KreuzbergRelationshipKind` | — | Semantic kind of the relationship. |


---

#### KreuzbergDocumentStructure

Top-level structured document representation.

A flat array of nodes with index-based parent/child references forming a tree.
Root-level nodes have `parent: None`. Use `body_roots()` and `furniture_roots()`
to iterate over top-level content by layer.

# Validation

Call `validate()` after construction to verify all node indices are in bounds
and parent-child relationships are bidirectionally consistent.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `nodes` | `KreuzbergDocumentNode*` | `NULL` | All nodes in document/reading order. |
| `source_format` | `const char**` | `NULL` | Origin format identifier (e.g. "docx", "pptx", "html", "pdf"). Allows renderers to apply format-aware heuristics when converting the document tree to output formats. |
| `relationships` | `KreuzbergDocumentRelationship*` | `NULL` | Resolved relationships between nodes (footnote refs, citations, anchor links, etc.). Populated during derivation from the internal document representation. Empty when no relationships are detected. |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergDocumentStructure kreuzberg_default();
```


---

#### KreuzbergDocxMetadata

Word document metadata.

Extracted from DOCX files using shared Office Open XML metadata extraction.
Integrates with `office_metadata` module for core/app/custom properties.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `core_properties` | `const char**` | `NULL` | Core properties from docProps/core.xml (Dublin Core metadata) Contains title, creator, subject, keywords, dates, etc. Shared format across DOCX/PPTX/XLSX documents. |
| `app_properties` | `const char**` | `NULL` | Application properties from docProps/app.xml (Word-specific statistics) Contains word count, page count, paragraph count, editing time, etc. DOCX-specific variant of Office application properties. |
| `custom_properties` | `void**` | `NULL` | Custom properties from docProps/custom.xml (user-defined properties) Contains key-value pairs defined by users or applications. Values can be strings, numbers, booleans, or dates. |


---

#### KreuzbergDrawing

A drawing object extracted from `<w:drawing>`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `drawing_type` | `const char*` | — | Drawing type |
| `extent` | `const char**` | `NULL` | Extent |
| `doc_properties` | `const char**` | `NULL` | Doc properties |
| `image_ref` | `const char**` | `NULL` | Image ref |


---

#### KreuzbergElement

Semantic element extracted from document.

Represents a logical unit of content with semantic classification,
unique identifier, and metadata for tracking origin and position.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `element_id` | `const char*` | — | Unique element identifier |
| `element_type` | `KreuzbergElementType` | — | Semantic type of this element |
| `text` | `const char*` | — | Text content of the element |
| `metadata` | `KreuzbergElementMetadata` | — | Metadata about the element |


---

#### KreuzbergElementMetadata

Metadata for a semantic element.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `page_number` | `uintptr_t*` | `NULL` | Page number (1-indexed) |
| `filename` | `const char**` | `NULL` | Source filename or document name |
| `coordinates` | `const char**` | `NULL` | Bounding box coordinates if available |
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
| `cleaned_text` | `const char*` | — | Cleaned/processed text content |
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

#### KreuzbergEmbedRequest

Embedding request for generating embeddings from text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `texts` | `const char**` | — | Text strings to generate embeddings for (at least one non-empty string required) |
| `config` | `KreuzbergEmbeddingConfig*` | `NULL` | Optional embedding configuration (model, batch size, etc.) |


---

#### KreuzbergEmbedResponse

Embedding response containing generated embeddings.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `embeddings` | `float**` | — | Generated embeddings (one per input text) |
| `model` | `const char*` | — | Model used for embedding generation |
| `dimensions` | `uintptr_t` | — | Dimensionality of the embeddings |
| `count` | `uintptr_t` | — | Number of embeddings generated |


---

#### KreuzbergEmbedTextParams

Request parameters for embedding generation.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `texts` | `const char**` | — | List of text strings to generate embeddings for |
| `preset` | `const char**` | `NULL` | Embedding preset name (default: "balanced"). Available: "speed", "balanced", "quality" |
| `model` | `const char**` | `NULL` | LLM model for provider-hosted embeddings (e.g., "openai/text-embedding-3-small"). When set, overrides preset and uses liter-llm for embedding generation. |
| `api_key` | `const char**` | `NULL` | API key for the LLM provider (optional, falls back to env). |
| `embedding_plugin` | `const char**` | `NULL` | Name of a pre-registered in-process embedding plugin backend. When set, overrides both preset and model and dispatches to the registered callback. Requires a prior call to `kreuzberg.plugins.register_embedding_backend`. |


---

#### KreuzbergEmbeddedFile

Embedded file descriptor extracted from the PDF name tree.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `const char*` | — | The filename as stored in the PDF name tree. |
| `data` | `const uint8_t*` | — | Raw file bytes from the embedded stream. |
| `mime_type` | `const char**` | `NULL` | MIME type if specified in the filespec, otherwise `NULL`. |


---

#### KreuzbergEmbeddingBackend

Trait for in-process embedding backend plugins.

Async to match the convention used by `crate.plugins.OcrBackend`,
`crate.plugins.DocumentExtractor`, and `crate.plugins.PostProcessor`.
Host-language bridges (PyO3, napi-rs, Rustler, extendr, magnus, ext-php-rs,
C FFI, etc.) wrap their synchronous host callables in `spawn_blocking` or the
equivalent to satisfy the async signature.

# Thread safety

Backends must be `Send + Sync + 'static`. They are stored in
`Arc<dyn EmbeddingBackend>` and called concurrently from kreuzberg's chunking
pipeline. If the backend's underlying model isn't thread-safe, the backend
itself must serialize access internally (e.g. via `Mutex<Inner>`).

# Contract

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
- `shutdown()` (inherited from `crate.plugins.Plugin`) may be invoked
  concurrently with an in-flight `embed()` call. Implementations must
  tolerate this — e.g. by letting in-flight calls finish using resources
  held via the `Arc<dyn EmbeddingBackend>` reference, and only releasing
  shared state that isn't needed by `embed`.

# Runtime

The synchronous `crate.embed_texts` entry uses
`tokio.task.block_in_place` to await the trait's async `embed`, which
requires a multi-thread tokio runtime. Callers running inside a
`current_thread` runtime (e.g. `#[tokio.test]` without `flavor = "multi_thread"`,
or `tokio.runtime.Builder.new_current_thread()`) must use
`crate.embed_texts_async` instead, which awaits directly without
`block_in_place`.

##### Methods

###### kreuzberg_dimensions()

Embedding vector dimension. Must be `> 0` and must match the length of
every vector returned by `embed`.

**Signature:**

```c
uintptr_t kreuzberg_dimensions();
```

###### kreuzberg_embed()

Embed a batch of texts, returning one vector per input in order.

**Errors:**

Implementations should return `crate.KreuzbergError.Plugin` for
backend-specific failures. The dispatcher layers its own validation
(length, per-vector dimension) on top.

**Signature:**

```c
float** kreuzberg_embed(const char** texts);
```


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
| `max_embed_duration_secs` | `uint64_t*` | `NULL` | Maximum wall-clock duration (in seconds) for a single `embed()` call when using `EmbeddingModelType.Plugin`. Applies only to the in-process plugin path — protects against hung host-language backends (e.g. a Python callback deadlocked on the GIL, a model stuck on CUDA OOM retries, etc.). On timeout, the dispatcher returns `crate.KreuzbergError.Plugin` instead of blocking forever. `NULL` disables the timeout. The default (60 seconds) is conservative for common in-process inference; increase for large batches on slow hardware. |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergEmbeddingConfig kreuzberg_default();
```


---

#### KreuzbergEpubMetadata

EPUB metadata (Dublin Core extensions).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `coverage` | `const char**` | `NULL` | Coverage |
| `dc_format` | `const char**` | `NULL` | Dc format |
| `relation` | `const char**` | `NULL` | Relation |
| `source` | `const char**` | `NULL` | Source |
| `dc_type` | `const char**` | `NULL` | Dc type |
| `cover_image` | `const char**` | `NULL` | Cover image |


---

#### KreuzbergErrorMetadata

Error metadata (for batch operations).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `error_type` | `const char*` | — | Error type |
| `message` | `const char*` | — | Message |


---

#### KreuzbergExcelMetadata

Excel/spreadsheet metadata.

Contains information about sheets in Excel, OpenDocument Calc, and other
spreadsheet formats (.xlsx, .xls, .ods, etc.).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `sheet_count` | `uintptr_t` | — | Total number of sheets in the workbook |
| `sheet_names` | `const char**` | `NULL` | Names of all sheets in order |


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


---

#### KreuzbergExtractBytesParams

Request parameters for bytes extraction.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `data` | `const char*` | — | Base64-encoded file content |
| `mime_type` | `const char**` | `NULL` | Optional MIME type hint (auto-detected if not provided) |
| `config` | `void**` | `NULL` | Extraction configuration (JSON object) |
| `pdf_password` | `const char**` | `NULL` | Password for encrypted PDFs |
| `response_format` | `const char**` | `NULL` | Wire format for the response: "json" (default) or "toon" |


---

#### KreuzbergExtractFileParams

Request parameters for file extraction.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `path` | `const char*` | — | Path to the file to extract |
| `mime_type` | `const char**` | `NULL` | Optional MIME type hint (auto-detected if not provided) |
| `config` | `void**` | `NULL` | Extraction configuration (JSON object) |
| `pdf_password` | `const char**` | `NULL` | Password for encrypted PDFs |
| `response_format` | `const char**` | `NULL` | Wire format for the response: "json" (default) or "toon" |


---

#### KreuzbergExtractResponse

Extraction response (list of results).


---

#### KreuzbergExtractStructuredParams

Request parameters for LLM-based structured extraction.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `path` | `const char*` | — | File path to extract from |
| `schema` | `void*` | — | JSON schema for structured output |
| `model` | `const char*` | — | LLM model (e.g., "openai/gpt-4o") |
| `schema_name` | `const char*` | — | Schema name (default: "extraction") |
| `schema_description` | `const char**` | `NULL` | Schema description for the LLM |
| `prompt` | `const char**` | `NULL` | Custom Jinja2 prompt template |
| `api_key` | `const char**` | `NULL` | API key (optional, falls back to env) |
| `strict` | `bool` | — | Enable strict mode |


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
| `image_index` | `uintptr_t` | — | Zero-indexed position of this image in the document/page |
| `page_number` | `uintptr_t*` | `NULL` | Page/slide number where image was found (1-indexed) |
| `width` | `uint32_t*` | `NULL` | Image width in pixels |
| `height` | `uint32_t*` | `NULL` | Image height in pixels |
| `colorspace` | `const char**` | `NULL` | Colorspace information (e.g., "RGB", "CMYK", "Gray") |
| `bits_per_component` | `uint32_t*` | `NULL` | Bits per color component (e.g., 8, 16) |
| `is_mask` | `bool` | — | Whether this image is a mask image |
| `description` | `const char**` | `NULL` | Optional description of the image |
| `ocr_result` | `KreuzbergExtractionResult*` | `NULL` | Nested OCR extraction result (if image was OCRed) When OCR is performed on this image, the result is embedded here rather than in a separate collection, making the relationship explicit. |
| `bounding_box` | `const char**` | `NULL` | Bounding box of the image on the page (PDF coordinates: x0=left, y0=bottom, x1=right, y1=top). Only populated for PDF-extracted images when position data is available from pdfium. |
| `source_path` | `const char**` | `NULL` | Original source path of the image within the document archive (e.g., "media/image1.png" in DOCX). Used for rendering image references when the binary data is not extracted. |


---

#### KreuzbergExtractedInlineImage

Extracted inline image with metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `data` | `const uint8_t*` | — | Uses `bytes.Bytes` for cheap cloning of large buffers. |
| `format` | `const char*` | — | Format |
| `filename` | `const char**` | `NULL` | Filename |
| `description` | `const char**` | `NULL` | Human-readable description |
| `dimensions` | `uint32_t**` | `NULL` | Dimensions |
| `attributes` | `const char**` | — | Attributes |


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
| `force_ocr_pages` | `uintptr_t**` | `NULL` | Force OCR on specific pages only (1-indexed page numbers, must be >= 1). When set, only the listed pages are OCR'd regardless of text layer quality. Unlisted pages use native text extraction. Ignored when `force_ocr` is `true`. Only applies to PDF documents. Duplicates are automatically deduplicated. An `ocr` config is recommended for backend/language selection; defaults are used if absent. |
| `disable_ocr` | `bool` | `false` | Disable OCR entirely, even for images. When `true`, OCR is skipped for all document types. Images return metadata only (dimensions, format, EXIF) without text extraction. PDFs use only native text extraction without OCR fallback. Cannot be `true` simultaneously with `force_ocr`. *Added in v4.7.0.* |
| `chunking` | `KreuzbergChunkingConfig*` | `NULL` | Text chunking configuration (None = chunking disabled) |
| `content_filter` | `KreuzbergContentFilterConfig*` | `NULL` | Content filtering configuration (None = use extractor defaults). Controls whether document "furniture" (headers, footers, watermarks, repeating text) is included in or stripped from extraction results. See `ContentFilterConfig` for per-field documentation. |
| `images` | `KreuzbergImageExtractionConfig*` | `NULL` | Image extraction configuration (None = no image extraction) |
| `pdf_options` | `KreuzbergPdfConfig*` | `NULL` | PDF-specific options (None = use defaults) |
| `token_reduction` | `KreuzbergTokenReductionOptions*` | `NULL` | Token reduction configuration (None = no token reduction) |
| `language_detection` | `KreuzbergLanguageDetectionConfig*` | `NULL` | Language detection configuration (None = no language detection) |
| `pages` | `KreuzbergPageConfig*` | `NULL` | Page extraction configuration (None = no page tracking) |
| `postprocessor` | `KreuzbergPostProcessorConfig*` | `NULL` | Post-processor configuration (None = use defaults) |
| `html_options` | `const char**` | `NULL` | HTML to Markdown conversion options (None = use defaults) Configure how HTML documents are converted to Markdown, including heading styles, list formatting, code block styles, and preprocessing options. |
| `html_output` | `KreuzbergHtmlOutputConfig*` | `NULL` | Styled HTML output configuration. When set alongside `output_format = OutputFormat.Html`, the extraction pipeline uses `StyledHtmlRenderer` which emits stable `kb-*` CSS class hooks on every structural element and optionally embeds theme CSS or user-supplied CSS in a `<style>` block. When `NULL`, the existing plain comrak-based HTML renderer is used. |
| `extraction_timeout_secs` | `uint64_t*` | `NULL` | Default per-file timeout in seconds for batch extraction. When set, each file in a batch will be canceled after this duration unless overridden by `FileExtractionConfig.timeout_secs`. `NULL` means no timeout (unbounded extraction time). |
| `max_concurrent_extractions` | `uintptr_t*` | `NULL` | Maximum concurrent extractions in batch operations (None = (num_cpus × 1.5).ceil()). Limits parallelism to prevent resource exhaustion when processing large batches. Defaults to (num_cpus × 1.5).ceil() when not set. |
| `result_format` | `const char*` | — | Result structure format Controls whether results are returned in unified format (default) with all content in the `content` field, or element-based format with semantic elements (for Unstructured-compatible output). |
| `security_limits` | `const char**` | `NULL` | Security limits for archive extraction. Controls maximum archive size, compression ratio, file count, and other security thresholds to prevent decompression bomb attacks. Also caps nesting depth, iteration count, entity / token length, cumulative content size, and table cell count for every extraction path that ingests user-controlled bytes. When `NULL`, default limits are used. |
| `output_format` | `const char*` | `Plain` | Content text format (default: Plain). Controls the format of the extracted content: - `Plain`: Raw extracted text (default) - `Markdown`: Markdown formatted output - `Djot`: Djot markup format (requires djot feature) - `Html`: HTML formatted output When set to a structured format, extraction results will include formatted output. The `formatted_content` field may be populated when format conversion is applied. |
| `layout` | `KreuzbergLayoutDetectionConfig*` | `NULL` | Layout detection configuration (None = layout detection disabled). When set, PDF pages and images are analyzed for document structure (headings, code, formulas, tables, figures, etc.) using RT-DETR models via ONNX Runtime. For PDFs, layout hints override paragraph classification in the markdown pipeline. For images, per-region OCR is performed with markdown formatting based on detected layout classes. Requires the `layout-detection` feature. |
| `include_document_structure` | `bool` | `false` | Enable structured document tree output. When true, populates the `document` field on `ExtractionResult` with a hierarchical `DocumentStructure` containing heading-driven section nesting, table grids, content layer classification, and inline annotations. Independent of `result_format` — can be combined with Unified or ElementBased. |
| `acceleration` | `KreuzbergAccelerationConfig*` | `NULL` | Hardware acceleration configuration for ONNX Runtime models. Controls execution provider selection for layout detection and embedding models. When `NULL`, uses platform defaults (CoreML on macOS, CUDA on Linux, CPU on Windows). |
| `cache_namespace` | `const char**` | `NULL` | Cache namespace for tenant isolation. When set, cache entries are stored under `{cache_dir}/{namespace}/`. Must be alphanumeric, hyphens, or underscores only (max 64 chars). Different namespaces have isolated cache spaces on the same filesystem. |
| `cache_ttl_secs` | `uint64_t*` | `NULL` | Per-request cache TTL in seconds. Overrides the global `max_age_days` for this specific extraction. When `0`, caching is completely skipped (no read or write). When `NULL`, the global TTL applies. |
| `email` | `KreuzbergEmailConfig*` | `NULL` | Email extraction configuration (None = use defaults). Currently supports configuring the fallback codepage for MSG files that do not specify one. See `crate.core.config.EmailConfig` for details. |
| `concurrency` | `const char**` | `NULL` | Concurrency limits for constrained environments (None = use defaults). Controls Rayon thread pool size, ONNX Runtime intra-op threads, and (when `max_concurrent_extractions` is unset) the batch concurrency semaphore. See `crate.core.config.ConcurrencyConfig` for details. |
| `max_archive_depth` | `uintptr_t` | — | Maximum recursion depth for archive extraction (default: 3). Set to 0 to disable recursive extraction (legacy behavior). |
| `tree_sitter` | `KreuzbergTreeSitterConfig*` | `NULL` | Tree-sitter language pack configuration (None = tree-sitter disabled). When set, enables code file extraction using tree-sitter parsers. Controls grammar download behavior and code analysis options. |
| `structured_extraction` | `KreuzbergStructuredExtractionConfig*` | `NULL` | Structured extraction via LLM (None = disabled). When set, the extracted document content is sent to an LLM with the provided JSON schema. The structured response is stored in `ExtractionResult.structured_output`. |
| `cancel_token` | `const char**` | `NULL` | Cancellation token for this extraction (None = no external cancellation). Pass a `CancellationToken` clone here and call `CancellationToken.cancel` from another thread / task to abort the extraction in progress. The extractor checks the token at safe checkpoints (before lock acquisition, between pages, between batch items) and returns `KreuzbergError.Cancelled` when set. The field is excluded from serialization because `CancellationToken` is a runtime handle, not a configuration value. |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergExtractionConfig kreuzberg_default();
```

###### kreuzberg_needs_image_processing()

Check if image processing is needed by examining OCR and image extraction settings.

Returns `true` if either OCR is enabled or image extraction is configured,
indicating that image decompression and processing should occur.
Returns `false` if both are disabled, allowing optimization to skip unnecessary
image decompression for text-only extraction workflows.

# Optimization Impact
For text-only extractions (no OCR, no image extraction), skipping image
decompression can improve CPU utilization by 5-10% by avoiding wasteful
image I/O and processing when results won't be used.

**Signature:**

```c
bool kreuzberg_needs_image_processing();
```


---

#### KreuzbergExtractionResult

General extraction result used by the core extraction API.

This is the main result type returned by all extraction functions.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `const char*` | — | The extracted text content |
| `mime_type` | `const char*` | — | The detected MIME type |
| `metadata` | `KreuzbergMetadata` | — | Document metadata |
| `extraction_method` | `KreuzbergExtractionMethod*` | `NULL` | Extraction strategy used to produce the returned text. Populated when the extractor can reliably distinguish native text extraction, OCR-only extraction, or mixed native/OCR output. |
| `tables` | `const char**` | `NULL` | Tables extracted from the document |
| `detected_languages` | `const char***` | `NULL` | Detected languages |
| `chunks` | `KreuzbergChunk**` | `NULL` | Text chunks when chunking is enabled. When chunking configuration is provided, the content is split into overlapping chunks for efficient processing. Each chunk contains the text, optional embeddings (if enabled), and metadata about its position. |
| `images` | `KreuzbergExtractedImage**` | `NULL` | Extracted images from the document. When image extraction is enabled via `ImageExtractionConfig`, this field contains all images found in the document with their raw data and metadata. Each image may optionally contain a nested `ocr_result` if OCR was performed. |
| `pages` | `KreuzbergPageContent**` | `NULL` | Per-page content when page extraction is enabled. When page extraction is configured, the document is split into per-page content with tables and images mapped to their respective pages. |
| `elements` | `KreuzbergElement**` | `NULL` | Semantic elements when element-based result format is enabled. When result_format is set to ElementBased, this field contains semantic elements with type classification, unique identifiers, and metadata for Unstructured-compatible element-based processing. |
| `djot_content` | `KreuzbergDjotContent*` | `NULL` | Rich Djot content structure (when extracting Djot documents). When extracting Djot documents with structured extraction enabled, this field contains the full semantic structure including: - Block-level elements with nesting - Inline formatting with attributes - Links, images, footnotes - Math expressions - Complete attribute information The `content` field still contains plain text for backward compatibility. Always `NULL` for non-Djot documents. |
| `ocr_elements` | `KreuzbergOcrElement**` | `NULL` | OCR elements with full spatial and confidence metadata. When OCR is performed with element extraction enabled, this field contains the structured representation of detected text including: - Bounding geometry (rectangles or quadrilaterals) - Confidence scores (detection and recognition) - Rotation information - Hierarchical relationships (Tesseract only) This field preserves all metadata that would otherwise be lost when converting to plain text or markdown output formats. Only populated when `OcrElementConfig.include_elements` is true. |
| `document` | `KreuzbergDocumentStructure*` | `NULL` | Structured document tree (when document structure extraction is enabled). When `include_document_structure` is true in `ExtractionConfig`, this field contains the full hierarchical representation of the document including: - Heading-driven section nesting - Table grids with cell-level metadata - Content layer classification (body, header, footer, footnote) - Inline text annotations (formatting, links) - Bounding boxes and page numbers Independent of `result_format` — can be combined with Unified or ElementBased. |
| `quality_score` | `double*` | `NULL` | Document quality score from quality analysis. A value between 0.0 and 1.0 indicating the overall text quality. Previously stored in `metadata.additional["quality_score"]`. |
| `processing_warnings` | `KreuzbergProcessingWarning*` | `NULL` | Non-fatal warnings collected during processing pipeline stages. Captures errors from optional pipeline features (embedding, chunking, language detection, output formatting) that don't prevent extraction but may indicate degraded results. Previously stored as individual keys in `metadata.additional`. |
| `annotations` | `KreuzbergPdfAnnotation**` | `NULL` | PDF annotations extracted from the document. When annotation extraction is enabled via `PdfConfig.extract_annotations`, this field contains text notes, highlights, links, stamps, and other annotations found in PDF documents. |
| `children` | `KreuzbergArchiveEntry**` | `NULL` | Nested extraction results from archive contents. When extracting archives, each processable file inside produces its own full extraction result. Set to `NULL` for non-archive formats. Use `max_archive_depth` in config to control recursion depth. |
| `uris` | `KreuzbergUri**` | `NULL` | URIs/links discovered during document extraction. Contains hyperlinks, image references, citations, email addresses, and other URI-like references found in the document. Always extracted when present in the source document. |
| `structured_output` | `void**` | `NULL` | Structured extraction output from LLM-based JSON schema extraction. When `structured_extraction` is configured in `ExtractionConfig`, the extracted document content is sent to a VLM with the provided JSON schema. The response is parsed and stored here as a JSON value matching the schema. |
| `code_intelligence` | `const char**` | `NULL` | Code intelligence results from tree-sitter analysis. Populated when extracting source code files with the `tree-sitter` feature. Contains metrics, structural analysis, imports/exports, comments, docstrings, symbols, diagnostics, and optionally chunked code segments. |
| `llm_usage` | `KreuzbergLlmUsage**` | `NULL` | LLM token usage and cost data for all LLM calls made during this extraction. Contains one entry per LLM call. Multiple entries are produced when VLM OCR, structured extraction, and/or LLM embeddings all run during the same extraction. `NULL` when no LLM was used. |
| `formatted_content` | `const char**` | `NULL` | Pre-rendered content in the requested output format. Populated during `derive_extraction_result` before tree derivation consumes element data. `apply_output_format` swaps this into `content` at the end of the pipeline, after post-processors have operated on plain text. |
| `ocr_internal_document` | `const char**` | `NULL` | Structured hOCR document for the OCR+layout pipeline. When tesseract produces hOCR output, the parsed `InternalDocument` carries paragraph structure with bounding boxes and confidence scores. The layout classification step enriches these elements before final rendering. |


---

#### KreuzbergFictionBookMetadata

FictionBook (FB2) metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `genres` | `const char**` | `NULL` | Genres |
| `sequences` | `const char**` | `NULL` | Sequences |
| `annotation` | `const char**` | `NULL` | Annotation |


---

#### KreuzbergFileExtractionConfig

Per-file extraction configuration overrides for batch processing.

All fields are `Option<T>` — `NULL` means "use the batch-level default."
This type is used with `crate.batch_extract_file` and
`crate.batch_extract_bytes` to allow heterogeneous
extraction settings within a single batch.

# Excluded Fields

The following `super.ExtractionConfig` fields are batch-level only and
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
| `force_ocr_pages` | `uintptr_t**` | `NULL` | Override force OCR pages for this file (1-indexed page numbers). |
| `disable_ocr` | `bool*` | `NULL` | Override disable OCR for this file. |
| `chunking` | `KreuzbergChunkingConfig*` | `NULL` | Override chunking configuration for this file. |
| `content_filter` | `KreuzbergContentFilterConfig*` | `NULL` | Override content filtering configuration for this file. |
| `images` | `KreuzbergImageExtractionConfig*` | `NULL` | Override image extraction configuration for this file. |
| `pdf_options` | `KreuzbergPdfConfig*` | `NULL` | Override PDF options for this file. |
| `token_reduction` | `KreuzbergTokenReductionOptions*` | `NULL` | Override token reduction for this file. |
| `language_detection` | `KreuzbergLanguageDetectionConfig*` | `NULL` | Override language detection for this file. |
| `pages` | `KreuzbergPageConfig*` | `NULL` | Override page extraction for this file. |
| `postprocessor` | `KreuzbergPostProcessorConfig*` | `NULL` | Override post-processor for this file. |
| `html_options` | `const char**` | `NULL` | Override HTML conversion options for this file. |
| `result_format` | `const char**` | `NULL` | Override result format for this file. |
| `output_format` | `const char**` | `NULL` | Override output content format for this file. |
| `include_document_structure` | `bool*` | `NULL` | Override document structure output for this file. |
| `layout` | `KreuzbergLayoutDetectionConfig*` | `NULL` | Override layout detection for this file. |
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
| `attributes` | `const char**` | `NULL` | Element attributes (classes, IDs, key-value pairs) |
| `language` | `const char**` | `NULL` | Language identifier for code blocks |
| `code` | `const char**` | `NULL` | Raw code content for code blocks |
| `children` | `KreuzbergFormattedBlock*` | — | Nested blocks for containers (blockquotes, list items, divs) |


---

#### KreuzbergGridCell

Individual grid cell with position and span metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `const char*` | — | Cell text content. |
| `row` | `uint32_t` | — | Zero-indexed row position. |
| `col` | `uint32_t` | — | Zero-indexed column position. |
| `row_span` | `uint32_t` | — | Number of rows this cell spans. |
| `col_span` | `uint32_t` | — | Number of columns this cell spans. |
| `is_header` | `bool` | — | Whether this is a header cell. |
| `bbox` | `const char**` | `NULL` | Bounding box for this cell (if available). |


---

#### KreuzbergHeaderFooter

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `paragraphs` | `const char**` | `NULL` | Paragraphs |
| `tables` | `const char**` | `NULL` | Tables extracted from the document |
| `header_type` | `const char*` | — | Header type |


---

#### KreuzbergHeaderMetadata

Header/heading element metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `level` | `uint8_t` | — | Header level: 1 (h1) through 6 (h6) |
| `text` | `const char*` | — | Normalized text content of the header |
| `id` | `const char**` | `NULL` | HTML id attribute if present |
| `depth` | `uintptr_t` | — | Document tree depth at the header element |
| `html_offset` | `uintptr_t` | — | Byte offset in original HTML document |


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

#### KreuzbergHealthResponse

Health check response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `status` | `const char*` | — | Health status |
| `version` | `const char*` | — | API version |
| `plugins` | `const char**` | `NULL` | Plugin status (optional) |


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
| `bbox` | `float**` | `NULL` | Bounding box information for the block Contains coordinates as (left, top, right, bottom) in PDF units. |


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


---

#### KreuzbergHtmlExtractionResult

Result of HTML extraction with optional images and warnings.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `markdown` | `const char*` | — | Markdown |
| `images` | `KreuzbergExtractedInlineImage*` | — | Images extracted from the document |
| `warnings` | `const char**` | — | Warnings |


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

##### Methods

###### kreuzberg_from()

**Signature:**

```c
KreuzbergHtmlMetadata kreuzberg_from(KreuzbergHtmlMetadata metadata);
```


---

#### KreuzbergHtmlOutputConfig

Configuration for styled HTML output.

When set on `ExtractionConfig.html_output` alongside
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


---

#### KreuzbergImageExtractionConfig

Image extraction configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `extract_images` | `bool` | `true` | Extract images from documents |
| `target_dpi` | `int32_t` | `300` | Target DPI for image normalization |
| `max_image_dimension` | `int32_t` | `4096` | Maximum dimension for images (width or height) |
| `inject_placeholders` | `bool` | `true` | Whether to inject image reference placeholders into markdown output. When `true` (default), image references like `![Image 1](embedded:p1_i0)` are appended to the markdown. Set to `false` to extract images as data without polluting the markdown output. |
| `auto_adjust_dpi` | `bool` | `true` | Automatically adjust DPI based on image content |
| `min_dpi` | `int32_t` | `72` | Minimum DPI threshold |
| `max_dpi` | `int32_t` | `600` | Maximum DPI threshold |
| `max_images_per_page` | `uint32_t*` | `NULL` | Maximum number of image objects to extract per PDF page. Some PDFs (e.g. technical diagrams stored as thousands of raster fragments) can trigger extremely long or indefinite extraction times when every image object on a dense page is decoded individually via pdfium FFI. Setting this limit causes kreuzberg to stop collecting individual images once the count per page reaches the cap and emit a warning instead. `NULL` (default) means no limit — all images are extracted. |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergImageExtractionConfig kreuzberg_default();
```


---

#### KreuzbergImageMetadataType

Image element metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `src` | `const char*` | — | Image source (URL, data URI, or SVG content) |
| `alt` | `const char**` | `NULL` | Alternative text from alt attribute |
| `title` | `const char**` | `NULL` | Title attribute |
| `dimensions` | `uint32_t**` | `NULL` | Image dimensions as (width, height) if available |
| `image_type` | `KreuzbergImageType` | — | Image type classification |
| `attributes` | `const char**` | — | Additional attributes as key-value pairs |


---

#### KreuzbergImageOcrResult

Result of OCR extraction from an image with optional page tracking.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `const char*` | — | Extracted text content |
| `boundaries` | `KreuzbergPageBoundary**` | `NULL` | Character byte boundaries per frame (for multi-frame TIFFs) |
| `page_contents` | `KreuzbergPageContent**` | `NULL` | Per-frame content information |


---

#### KreuzbergImagePreprocessingConfig

Image preprocessing configuration for OCR.

These settings control how images are preprocessed before OCR to improve
text recognition quality. Different preprocessing strategies work better
for different document types.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `target_dpi` | `int32_t` | `300` | Target DPI for the image (300 is standard, 600 for small text). |
| `auto_rotate` | `bool` | `true` | Auto-detect and correct image rotation. |
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


---

#### KreuzbergImagePreprocessingMetadata

Image preprocessing metadata.

Tracks the transformations applied to an image during OCR preprocessing,
including DPI normalization, resizing, and resampling.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `original_dimensions` | `uintptr_t*` | — | Original image dimensions (width, height) in pixels |
| `original_dpi` | `double*` | — | Original image DPI (horizontal, vertical) |
| `target_dpi` | `int32_t` | — | Target DPI from configuration |
| `scale_factor` | `double` | — | Scaling factor applied to the image |
| `auto_adjusted` | `bool` | — | Whether DPI was auto-adjusted based on content |
| `final_dpi` | `int32_t` | — | Final DPI after processing |
| `new_dimensions` | `uintptr_t**` | `NULL` | New dimensions after resizing (if resized) |
| `resample_method` | `const char*` | — | Resampling algorithm used ("LANCZOS3", "CATMULLROM", etc.) |
| `dimension_clamped` | `bool` | — | Whether dimensions were clamped to max_image_dimension |
| `calculated_dpi` | `int32_t*` | `NULL` | Calculated optimal DPI (if auto_adjust_dpi enabled) |
| `skipped_resize` | `bool` | — | Whether resize was skipped (dimensions already optimal) |
| `resize_error` | `const char**` | `NULL` | Error message if resize failed |


---

#### KreuzbergInfoResponse

Server information response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `version` | `const char*` | — | API version |
| `rust_backend` | `bool` | — | Whether using Rust backend |


---

#### KreuzbergInlineElement

Inline element within a block.

Represents text with formatting, links, images, etc.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `element_type` | `KreuzbergInlineType` | — | Type of inline element |
| `content` | `const char*` | — | Text content |
| `attributes` | `const char**` | `NULL` | Element attributes |
| `metadata` | `void**` | `NULL` | Additional metadata (e.g., href for links, src/alt for images) |


---

#### KreuzbergJatsMetadata

JATS (Journal Article Tag Suite) metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `copyright` | `const char**` | `NULL` | Copyright |
| `license` | `const char**` | `NULL` | License |
| `history_dates` | `void*` | `NULL` | History dates |
| `contributor_roles` | `KreuzbergContributorRole*` | `NULL` | Contributor roles |


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
| `ngram_range` | `uintptr_t*` | `NULL` | N-gram range for keyword extraction (min, max). (1, 1) = unigrams only (1, 2) = unigrams and bigrams (1, 3) = unigrams, bigrams, and trigrams (default) |
| `language` | `const char**` | `NULL` | Language code for stopword filtering (e.g., "en", "de", "fr"). If None, no stopword filtering is applied. |
| `yake_params` | `KreuzbergYakeParams*` | `NULL` | YAKE-specific tuning parameters. |
| `rake_params` | `KreuzbergRakeParams*` | `NULL` | RAKE-specific tuning parameters. |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergKeywordConfig kreuzberg_default();
```


---

#### KreuzbergLanguageDetectionConfig

Language detection configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `bool` | — | Enable language detection |
| `min_confidence` | `double` | — | Minimum confidence threshold (0.0-1.0) |
| `detect_multiple` | `bool` | — | Detect multiple languages in the document |


---

#### KreuzbergLayoutDetection

A single layout detection result.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `class_name` | `KreuzbergLayoutClass` | — | Class name (layout class) |
| `confidence` | `float` | — | Confidence |
| `bbox` | `KreuzbergBBox` | — | Bbox (b box) |


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

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergLayoutDetectionConfig kreuzberg_default();
```


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
| `bounding_box` | `const char*` | — | Bounding box in document coordinate space. |
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
| `attributes` | `const char**` | — | Additional attributes as key-value pairs |


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

#### KreuzbergManifestEntryResponse

Model manifest entry for cache management.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `relative_path` | `const char*` | — | Relative path within the cache directory |
| `sha256` | `const char*` | — | SHA256 checksum of the model file |
| `size_bytes` | `uint64_t` | — | Expected file size in bytes |
| `source_url` | `const char*` | — | HuggingFace source URL for downloading |


---

#### KreuzbergManifestResponse

Model manifest response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `kreuzberg_version` | `const char*` | — | Kreuzberg version |
| `total_size_bytes` | `uint64_t` | — | Total size of all models in bytes |
| `model_count` | `uintptr_t` | — | Number of models in the manifest |
| `models` | `KreuzbergManifestEntryResponse*` | — | Individual model entries |


---

#### KreuzbergMergedChunk

A merged chunk produced by `merge_segments`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text` | `const char*` | — | Text |
| `byte_start` | `uintptr_t` | — | Byte start |
| `byte_end` | `uintptr_t` | — | Byte end |


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
| `format` | `KreuzbergFormatMetadata*` | `NULL` | Format-specific metadata (discriminated union) Contains detailed metadata specific to the document format. Serializes with a `format_type` discriminator field. |
| `image_preprocessing` | `KreuzbergImagePreprocessingMetadata*` | `NULL` | Image preprocessing metadata (when OCR preprocessing was applied) |
| `json_schema` | `void**` | `NULL` | JSON schema (for structured data extraction) |
| `error` | `KreuzbergErrorMetadata*` | `NULL` | Error metadata (for batch operations) |
| `extraction_duration_ms` | `uint64_t*` | `NULL` | Extraction duration in milliseconds (for benchmarking). This field is populated by batch extraction to provide per-file timing information. It's `NULL` for single-file extraction (which uses external timing). |
| `category` | `const char**` | `NULL` | Document category (from frontmatter or classification). |
| `tags` | `const char***` | `NULL` | Document tags (from frontmatter). |
| `document_version` | `const char**` | `NULL` | Document version string (from frontmatter). |
| `abstract_text` | `const char**` | `NULL` | Abstract or summary text (from frontmatter). |
| `output_format` | `const char**` | `NULL` | Output format identifier (e.g., "markdown", "html", "text"). Set by the output format pipeline stage when format conversion is applied. Previously stored in `metadata.additional["output_format"]`. |
| `additional` | `const char*` | — | Additional custom fields from postprocessors. **Deprecated**: Prefer using typed fields on `ExtractionResult` and `Metadata` instead of inserting into this map. Typed fields provide better cross-language compatibility and type safety. This field will be removed in a future major version. This flattened map allows Python/TypeScript postprocessors to add arbitrary fields (entity extraction, keyword extraction, etc.). Fields are merged at the root level during serialization. Uses `Cow<'static, str>` keys so static string keys avoid allocation. |


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

#### KreuzbergNote

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `const char*` | — | Unique identifier |
| `note_type` | `const char*` | — | Note type |
| `paragraphs` | `const char**` | — | Paragraphs |


---

#### KreuzbergOcrBackend

Trait for OCR backend plugins.

Implement this trait to add custom OCR capabilities. OCR backends can be:
- Native Rust implementations (like Tesseract)
- FFI bridges to Python libraries (like EasyOCR, PaddleOCR)
- Cloud-based OCR services (Google Vision, AWS Textract, etc.)

# Thread Safety

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

**Signature:**

```c
KreuzbergExtractionResult kreuzberg_process_image(const uint8_t* image_bytes, KreuzbergOcrConfig config);
```

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

###### kreuzberg_supports_language()

Check if this backend supports a given language code.

**Returns:**

`true` if the language is supported, `false` otherwise.

**Signature:**

```c
bool kreuzberg_supports_language(const char* lang);
```

###### kreuzberg_backend_type()

Get the backend type identifier.

**Returns:**

The backend type enum value.

**Signature:**

```c
KreuzbergOcrBackendType kreuzberg_backend_type();
```

###### kreuzberg_supported_languages()

Optional: Get a list of all supported languages.

Defaults to empty list. Override to provide comprehensive language support info.

**Signature:**

```c
const char** kreuzberg_supported_languages();
```

###### kreuzberg_supports_table_detection()

Optional: Check if the backend supports table detection.

Defaults to `false`. Override if your backend can detect and extract tables.

**Signature:**

```c
bool kreuzberg_supports_table_detection();
```

###### kreuzberg_supports_document_processing()

Check if the backend supports direct document-level processing (e.g. for PDFs).

Defaults to `false`. Override if the backend has optimized document processing.

**Signature:**

```c
bool kreuzberg_supports_document_processing();
```

###### kreuzberg_process_document()

Process a document file directly via OCR.

Only called if `supports_document_processing` returns `true`.

**Signature:**

```c
KreuzbergExtractionResult kreuzberg_process_document(const char* path, KreuzbergOcrConfig config);
```


---

#### KreuzbergOcrCacheStats

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `total_files` | `uintptr_t` | — | Total files |
| `total_size_mb` | `double` | — | Total size mb |


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
| `output_format` | `const char**` | `NULL` | Output format for OCR results (optional, for format conversion) |
| `paddle_ocr_config` | `void**` | `NULL` | PaddleOCR-specific configuration (optional, JSON passthrough) |
| `element_config` | `KreuzbergOcrElementConfig*` | `NULL` | OCR element extraction configuration |
| `quality_thresholds` | `KreuzbergOcrQualityThresholds*` | `NULL` | Quality thresholds for the native-text-to-OCR fallback decision. When None, uses compiled defaults (matching previous hardcoded behavior). |
| `pipeline` | `KreuzbergOcrPipelineConfig*` | `NULL` | Multi-backend OCR pipeline configuration. When set, enables weighted fallback across multiple OCR backends based on output quality. When None, uses the single `backend` field (same as today). |
| `auto_rotate` | `bool` | `false` | Enable automatic page rotation based on orientation detection. When enabled, uses Tesseract's `DetectOrientationScript()` to detect page orientation (0/90/180/270 degrees) before OCR. If the page is rotated with high confidence, the image is corrected before recognition. This is critical for handling rotated scanned documents. |
| `vlm_config` | `KreuzbergLlmConfig*` | `NULL` | VLM (Vision Language Model) OCR configuration. Required when `backend` is `"vlm"`. Uses liter-llm to send page images to a vision model for text extraction. |
| `vlm_prompt` | `const char**` | `NULL` | Custom Jinja2 prompt template for VLM OCR. When `NULL`, uses the default template. Available variables: - `{{ language }}` — The document language code (e.g., "eng", "deu"). |
| `acceleration` | `KreuzbergAccelerationConfig*` | `NULL` | Hardware acceleration for ONNX Runtime models (e.g. PaddleOCR, layout detection). Not user-configurable via config files — injected at runtime from `ExtractionConfig.acceleration` before each `process_image` call. |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergOcrConfig kreuzberg_default();
```


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
| `page_number` | `uintptr_t` | — | Page number (1-indexed). |
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
| `ocr_elements` | `KreuzbergOcrElement**` | `NULL` | Structured OCR elements with bounding boxes and confidence scores. Available when TSV output is requested or table detection is enabled. |
| `internal_document` | `const char**` | `NULL` | Structured document produced from hOCR parsing. Carries paragraph structure, bounding boxes, and confidence scores that the flattened `content` string discards. |


---

#### KreuzbergOcrMetadata

OCR processing metadata.

Captures information about OCR processing configuration and results.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `language` | `const char*` | — | OCR language code(s) used |
| `psm` | `int32_t` | — | Tesseract Page Segmentation Mode (PSM) |
| `output_format` | `const char*` | — | Output format (e.g., "text", "hocr") |
| `table_count` | `uintptr_t` | — | Number of tables detected |
| `table_rows` | `uintptr_t*` | `NULL` | Table rows |
| `table_cols` | `uintptr_t*` | `NULL` | Table cols |


---

#### KreuzbergOcrPipelineConfig

Multi-backend OCR pipeline with quality-based fallback.

Backends are tried in priority order (highest first). After each backend
produces output, quality is evaluated. If it meets `quality_thresholds.pipeline_min_quality`,
the result is accepted. Otherwise the next backend is tried.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `stages` | `KreuzbergOcrPipelineStage*` | — | Ordered list of backends to try. Sorted by priority (descending) at runtime. |
| `quality_thresholds` | `KreuzbergOcrQualityThresholds` | — | Quality thresholds for deciding whether to accept a result or try the next backend. |


---

#### KreuzbergOcrPipelineStage

A single backend stage in the OCR pipeline.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `backend` | `const char*` | — | Backend name: "tesseract", "paddleocr", "easyocr", or a custom registered name. |
| `priority` | `uint32_t` | — | Priority weight (higher = tried first). Stages are sorted by priority descending. |
| `language` | `const char**` | `NULL` | Language override for this stage (None = use parent OcrConfig.language). |
| `tesseract_config` | `KreuzbergTesseractConfig*` | `NULL` | Tesseract-specific config override for this stage. |
| `paddle_ocr_config` | `void**` | `NULL` | PaddleOCR-specific config for this stage. |
| `vlm_config` | `KreuzbergLlmConfig*` | `NULL` | VLM config override for this pipeline stage. |


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
| `page_number` | `uintptr_t` | — | Page number where the table was found (1-indexed) |
| `bounding_box` | `KreuzbergOcrTableBoundingBox*` | `NULL` | Bounding box of the table in pixel coordinates (from OCR word positions). |


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

#### KreuzbergOdtProperties

OpenDocument metadata from meta.xml

Contains metadata fields defined by the OASIS OpenDocument Format standard.
Uses Dublin Core elements (dc:) and OpenDocument meta elements (meta:).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `title` | `const char**` | `NULL` | Document title (dc:title) |
| `subject` | `const char**` | `NULL` | Document subject/topic (dc:subject) |
| `creator` | `const char**` | `NULL` | Current document creator/author (dc:creator) |
| `initial_creator` | `const char**` | `NULL` | Initial creator of the document (meta:initial-creator) |
| `keywords` | `const char**` | `NULL` | Keywords or tags (meta:keyword) |
| `description` | `const char**` | `NULL` | Document description (dc:description) |
| `date` | `const char**` | `NULL` | Current modification date (dc:date) |
| `creation_date` | `const char**` | `NULL` | Initial creation date (meta:creation-date) |
| `language` | `const char**` | `NULL` | Document language (dc:language) |
| `generator` | `const char**` | `NULL` | Generator/application that created the document (meta:generator) |
| `editing_duration` | `const char**` | `NULL` | Editing duration in ISO 8601 format (meta:editing-duration) |
| `editing_cycles` | `const char**` | `NULL` | Number of edits/revisions (meta:editing-cycles) |
| `page_count` | `int32_t*` | `NULL` | Document statistics - page count (meta:page-count) |
| `word_count` | `int32_t*` | `NULL` | Document statistics - word count (meta:word-count) |
| `character_count` | `int32_t*` | `NULL` | Document statistics - character count (meta:character-count) |
| `paragraph_count` | `int32_t*` | `NULL` | Document statistics - paragraph count (meta:paragraph-count) |
| `table_count` | `int32_t*` | `NULL` | Document statistics - table count (meta:table-count) |
| `image_count` | `int32_t*` | `NULL` | Document statistics - image count (meta:image-count) |


---

#### KreuzbergOpenWebDocumentResponse

OpenWebUI "External" engine response format.

Returned by `PUT /process` for the OpenWebUI external document loader.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `page_content` | `const char*` | — | Extracted text content |
| `metadata` | `const char*` | — | Document metadata |


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

###### kreuzberg_with_table_detection()

Enables or disables table structure detection.

**Signature:**

```c
KreuzbergPaddleOcrConfig kreuzberg_with_table_detection(bool enable);
```

###### kreuzberg_with_angle_cls()

Enables or disables angle classification for rotated text.

**Signature:**

```c
KreuzbergPaddleOcrConfig kreuzberg_with_angle_cls(bool enable);
```

###### kreuzberg_with_det_db_thresh()

Sets the database threshold for text detection.

**Signature:**

```c
KreuzbergPaddleOcrConfig kreuzberg_with_det_db_thresh(float threshold);
```

###### kreuzberg_with_det_db_box_thresh()

Sets the box threshold for text bounding box refinement.

**Signature:**

```c
KreuzbergPaddleOcrConfig kreuzberg_with_det_db_box_thresh(float threshold);
```

###### kreuzberg_with_det_db_unclip_ratio()

Sets the unclip ratio for expanding text bounding boxes.

**Signature:**

```c
KreuzbergPaddleOcrConfig kreuzberg_with_det_db_unclip_ratio(float ratio);
```

###### kreuzberg_with_det_limit_side_len()

Sets the maximum side length for detection images.

**Signature:**

```c
KreuzbergPaddleOcrConfig kreuzberg_with_det_limit_side_len(uint32_t length);
```

###### kreuzberg_with_rec_batch_num()

Sets the batch size for recognition inference.

**Signature:**

```c
KreuzbergPaddleOcrConfig kreuzberg_with_rec_batch_num(uint32_t batch_size);
```

###### kreuzberg_with_drop_score()

Sets the minimum recognition confidence threshold.

**Signature:**

```c
KreuzbergPaddleOcrConfig kreuzberg_with_drop_score(float score);
```

###### kreuzberg_with_padding()

Sets padding in pixels added around images before detection.

**Signature:**

```c
KreuzbergPaddleOcrConfig kreuzberg_with_padding(uint32_t padding);
```

###### kreuzberg_with_model_tier()

Sets the model tier controlling detection/recognition model size.

**Signature:**

```c
KreuzbergPaddleOcrConfig kreuzberg_with_model_tier(const char* tier);
```

###### kreuzberg_default()

Creates a default configuration with English language support.

**Signature:**

```c
KreuzbergPaddleOcrConfig kreuzberg_default();
```


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
| `page_number` | `uintptr_t` | — | Page number (1-indexed) |


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
| `marker_format` | `const char*` | `"

<!-- PAGE {page_num} -->

"` | Page marker format (use {page_num} placeholder) Default: "\n\n<!-- PAGE {page_num} -->\n\n" |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergPageConfig kreuzberg_default();
```


---

#### KreuzbergPageContent

Content for a single page/slide.

When page extraction is enabled, documents are split into per-page content
with associated tables and images mapped to each page.

# Performance

Uses Arc-wrapped tables and images for memory efficiency:
- `Vec<Arc<Table>>` enables zero-copy sharing of table data
- `Vec<Arc<ExtractedImage>>` enables zero-copy sharing of image data
- Maintains exact JSON compatibility via custom Serialize/Deserialize

This reduces memory overhead for documents with shared tables/images
by avoiding redundant copies during serialization.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `page_number` | `uintptr_t` | — | Page number (1-indexed) |
| `content` | `const char*` | — | Text content for this page |
| `tables` | `const char**` | — | Tables found on this page (uses Arc for memory efficiency) Serializes as Vec<Table> for JSON compatibility while maintaining Arc semantics in-memory for zero-copy sharing. |
| `images` | `KreuzbergExtractedImage*` | — | Images found on this page (uses Arc for memory efficiency) Serializes as Vec<ExtractedImage> for JSON compatibility while maintaining Arc semantics in-memory for zero-copy sharing. |
| `hierarchy` | `KreuzbergPageHierarchy*` | `NULL` | Hierarchy information for the page (when hierarchy extraction is enabled) Contains text hierarchy levels (H1-H6) extracted from the page content. |
| `is_blank` | `bool*` | `NULL` | Whether this page is blank (no meaningful text content) Determined during extraction based on text content analysis. A page is blank if it has fewer than 3 non-whitespace characters and contains no tables or images. |
| `layout_regions` | `KreuzbergLayoutRegion**` | `NULL` | Layout detection regions for this page (when layout detection is enabled). Contains detected layout regions with class, confidence, bounding box, and area fraction. Only populated when layout detection is configured. |


---

#### KreuzbergPageHierarchy

Page hierarchy structure containing heading levels and block information.

Used when PDF text hierarchy extraction is enabled. Contains hierarchical
blocks with heading levels (H1-H6) for semantic document structure.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `block_count` | `uintptr_t` | — | Number of hierarchy blocks on this page |
| `blocks` | `KreuzbergHierarchicalBlock*` | — | Hierarchical blocks with heading levels |


---

#### KreuzbergPageInfo

Metadata for individual page/slide/sheet.

Captures per-page information including dimensions, content counts,
and visibility state (for presentations).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `number` | `uintptr_t` | — | Page number (1-indexed) |
| `title` | `const char**` | `NULL` | Page title (usually for presentations) |
| `dimensions` | `double**` | `NULL` | Dimensions in points (PDF) or pixels (images): (width, height) |
| `image_count` | `uintptr_t*` | `NULL` | Number of images on this page |
| `table_count` | `uintptr_t*` | `NULL` | Number of tables on this page |
| `hidden` | `bool*` | `NULL` | Whether this page is hidden (e.g., in presentations) |
| `is_blank` | `bool*` | `NULL` | Whether this page is blank (no meaningful text, no images, no tables) A page is considered blank if it has fewer than 3 non-whitespace characters and contains no tables or images. This is useful for filtering out empty pages in scanned documents or PDFs with blank separator pages. |


---

#### KreuzbergPageLayoutResult

Layout detection results for a single page.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `page_index` | `uintptr_t` | — | Page index |
| `regions` | `const char**` | — | Regions |
| `page_width_pts` | `float` | — | Page width pts |
| `page_height_pts` | `float` | — | Page height pts |
| `render_width_px` | `uint32_t` | — | Width of the rendered image used for layout detection (pixels). |
| `render_height_px` | `uint32_t` | — | Height of the rendered image used for layout detection (pixels). |


---

#### KreuzbergPageMarginsPoints

Page margins converted to points (1/72 inch).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `top` | `double*` | `NULL` | Top |
| `right` | `double*` | `NULL` | Right |
| `bottom` | `double*` | `NULL` | Bottom |
| `left` | `double*` | `NULL` | Left |
| `header` | `double*` | `NULL` | Header |
| `footer` | `double*` | `NULL` | Footer |
| `gutter` | `double*` | `NULL` | Gutter |


---

#### KreuzbergPageStructure

Unified page structure for documents.

Supports different page types (PDF pages, PPTX slides, Excel sheets)
with character offset boundaries for chunk-to-page mapping.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `total_count` | `uintptr_t` | — | Total number of pages/slides/sheets |
| `unit_type` | `KreuzbergPageUnitType` | — | Type of paginated unit |
| `boundaries` | `KreuzbergPageBoundary**` | `NULL` | Character offset boundaries for each page Maps character ranges in the extracted content to page numbers. Used for chunk page range calculation. |
| `pages` | `KreuzbergPageInfo**` | `NULL` | Detailed per-page metadata (optional, only when needed) |


---

#### KreuzbergPageTiming

Timing breakdown for a single page.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `render_ms` | `double` | — | Time to render the PDF page to a raster image (amortized from batch render). |
| `preprocess_ms` | `double` | — | Time spent in image preprocessing (resize, normalize, tensor construction). |
| `onnx_ms` | `double` | — | Time for the ONNX model session.run() call (actual neural network inference). |
| `inference_ms` | `double` | — | Total model inference time (preprocess + onnx), as measured by the engine. |
| `postprocess_ms` | `double` | — | Time spent in postprocessing (confidence filtering, overlap resolution). |
| `mapping_ms` | `double` | — | Time to map pixel-space bounding boxes to PDF coordinate space. |


---

#### KreuzbergPdfAnnotation

A PDF annotation extracted from a document page.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `annotation_type` | `KreuzbergPdfAnnotationType` | — | The type of annotation. |
| `content` | `const char**` | `NULL` | Text content of the annotation (e.g., comment text, link URL). |
| `page_number` | `uintptr_t` | — | Page number where the annotation appears (1-indexed). |
| `bounding_box` | `const char**` | `NULL` | Bounding box of the annotation on the page. |


---

#### KreuzbergPdfConfig

PDF-specific configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `backend` | `KreuzbergPdfBackend` | `KREUZBERG_KREUZBERG_PDFIUM` | PDF extraction backend. Default: `Pdfium`. |
| `extract_images` | `bool` | `false` | Extract images from PDF |
| `passwords` | `const char***` | `NULL` | List of passwords to try when opening encrypted PDFs |
| `extract_metadata` | `bool` | `true` | Extract PDF metadata |
| `hierarchy` | `KreuzbergHierarchyConfig*` | `NULL` | Hierarchy extraction configuration (None = hierarchy extraction disabled) |
| `extract_annotations` | `bool` | `false` | Extract PDF annotations (text notes, highlights, links, stamps). Default: false |
| `top_margin_fraction` | `float*` | `NULL` | Top margin fraction (0.0–1.0) of page height to exclude headers/running heads. Default: 0.06 (6%) |
| `bottom_margin_fraction` | `float*` | `NULL` | Bottom margin fraction (0.0–1.0) of page height to exclude footers/page numbers. Default: 0.05 (5%) |
| `allow_single_column_tables` | `bool` | `false` | Allow single-column pseudo tables in extraction results. By default, tables with fewer than 2 columns (layout-guided) or 3 columns (heuristic) are rejected. When `true`, the minimum column count is relaxed to 1, allowing single-column structured data (glossaries, itemized lists) to be emitted as tables. Other quality filters (density, sparsity, prose detection) still apply. |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergPdfConfig kreuzberg_default();
```


---

#### KreuzbergPdfImage

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `page_number` | `uintptr_t` | — | Page number |
| `image_index` | `uintptr_t` | — | Image index |
| `width` | `int64_t` | — | Width |
| `height` | `int64_t` | — | Height |
| `color_space` | `const char**` | `NULL` | Color space |
| `bits_per_component` | `int64_t*` | `NULL` | Bits per component |
| `filters` | `const char**` | — | Original PDF stream filters (e.g. `["FlateDecode"]`, `["DCTDecode"]`). |
| `data` | `const uint8_t*` | — | The decoded image bytes in a standard format (JPEG, PNG, etc.). |
| `decoded_format` | `const char*` | — | The format of `data` after decoding: `"jpeg"`, `"png"`, `"jpeg2000"`, `"ccitt"`, or `"raw"`. |


---

#### KreuzbergPdfUnifiedExtractionResult

Result type for unified PDF text and metadata extraction.

Contains text, optional page boundaries, optional per-page content, and metadata.


---

#### KreuzbergPlugin

Base trait that all plugins must implement.

This trait provides common functionality for plugin lifecycle management,
identification, and metadata.

# Thread Safety

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

###### kreuzberg_version()

Returns the semantic version of this plugin.

Should follow semver format: `MAJOR.MINOR.PATCH`

**Signature:**

```c
const char* kreuzberg_version();
```

###### kreuzberg_initialize()

Initialize the plugin.

Called once when the plugin is registered. Use this to:
- Load configuration
- Initialize resources (connections, caches, etc.)
- Validate dependencies

# Thread Safety

This method takes `&self` instead of `&mut self` to work with `Arc<dyn Plugin>`.
Plugins needing mutable state during initialization should use interior mutability
patterns (Mutex, RwLock, OnceCell, etc.).

**Errors:**

Should return an error if initialization fails. The plugin will not be
registered if this method returns an error.

**Signature:**

```c
void kreuzberg_initialize();
```

###### kreuzberg_shutdown()

Shutdown the plugin.

Called when the plugin is being unregistered or the application is shutting down.
Use this to:
- Close connections
- Flush caches
- Release resources

# Thread Safety

This method takes `&self` instead of `&mut self` to work with `Arc<dyn Plugin>`.
Plugins needing mutable state during shutdown should use interior mutability
patterns (Mutex, RwLock, etc.).

**Errors:**

Errors during shutdown are logged but don't prevent the shutdown process.

**Signature:**

```c
void kreuzberg_shutdown();
```

###### kreuzberg_description()

Optional plugin description for debugging and logging.

Defaults to empty string if not overridden.

**Signature:**

```c
const char* kreuzberg_description();
```

###### kreuzberg_author()

Optional plugin author information.

Defaults to empty string if not overridden.

**Signature:**

```c
const char* kreuzberg_author();
```


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

# Processing Order

Post-processors are executed in stage order:
1. **Early** - Language detection, entity extraction
2. **Middle** - Keyword extraction, token reduction
3. **Late** - Custom hooks, final validation

Within each stage, processors are executed in registration order.

# Error Handling

Post-processor errors are non-fatal by default - they're captured in metadata
and execution continues. To make errors fatal, return an error from `process()`.

# Thread Safety

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

# Performance

This signature avoids unnecessary cloning of large extraction results by
taking a mutable reference instead of ownership. Processors modify the
result in place.

# Example - Language Detection


# Example - Text Cleaning

**Signature:**

```c
void kreuzberg_process(KreuzbergExtractionResult result, KreuzbergExtractionConfig config);
```

###### kreuzberg_processing_stage()

Get the processing stage for this post-processor.

Determines when this processor runs in the pipeline.

**Returns:**

The `ProcessingStage` (Early, Middle, or Late).

**Signature:**

```c
KreuzbergProcessingStage kreuzberg_processing_stage();
```

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

###### kreuzberg_estimated_duration_ms()

Optional: Estimate processing time in milliseconds.

Used for logging and debugging. Defaults to 0 (unknown).

**Returns:**

Estimated processing time in milliseconds.

**Signature:**

```c
uint64_t kreuzberg_estimated_duration_ms(KreuzbergExtractionResult result);
```


---

#### KreuzbergPostProcessorConfig

Post-processor configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `bool` | `true` | Enable post-processors |
| `enabled_processors` | `const char***` | `NULL` | Whitelist of processor names to run (None = all enabled) |
| `disabled_processors` | `const char***` | `NULL` | Blacklist of processor names to skip (None = none disabled) |
| `enabled_set` | `const char**` | `NULL` | Pre-computed AHashSet for O(1) enabled processor lookup |
| `disabled_set` | `const char**` | `NULL` | Pre-computed AHashSet for O(1) disabled processor lookup |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergPostProcessorConfig kreuzberg_default();
```


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
| `hyperlinks` | `const char**` | — | Hyperlinks discovered in slides as (url, optional_label) pairs. |
| `office_metadata` | `void*` | — | Office metadata extracted from docProps/core.xml and docProps/app.xml. Contains keys like "title", "author", "created_by", "subject", "keywords", "modified_by", "created_at", "modified_at", etc. |


---

#### KreuzbergPptxMetadata

PowerPoint presentation metadata.

Extracted from PPTX files containing slide counts and presentation details.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `slide_count` | `uintptr_t` | — | Total number of slides in the presentation |
| `slide_names` | `const char**` | `NULL` | Names of slides (if available) |
| `image_count` | `uintptr_t*` | `NULL` | Number of embedded images |
| `table_count` | `uintptr_t*` | `NULL` | Number of tables |


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
| `message_count` | `uintptr_t` | — | Number of messages |


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


---

#### KreuzbergRecognizedTable

Pre-computed table markdown for a table detection region.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `detection_bbox` | `KreuzbergBBox` | — | Detection bbox that this table corresponds to (for matching). |
| `cells` | `const char***` | — | Table cells as a 2D vector (rows x columns). |
| `markdown` | `const char*` | — | Rendered markdown table. |


---

#### KreuzbergRecyclable

Trait for types that can be pooled and reused.

Implementing this trait allows a type to be used with `Pool<T>`.
The `reset()` method should clear the object's state for reuse.

##### Methods

###### kreuzberg_reset()

Reset the object to a reusable state.

This is called when returning an object to the pool.
Should clear any internal data while preserving capacity.

**Signature:**

```c
void kreuzberg_reset();
```


---

#### KreuzbergResolvedStyle

Fully resolved (flattened) style after walking the inheritance chain.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `paragraph_properties` | `const char*` | — | Paragraph properties |
| `run_properties` | `const char*` | — | Run properties |


---

#### KreuzbergServerConfig

API server configuration.

This struct holds all configuration options for the Kreuzberg API server,
including host/port settings, CORS configuration, and upload limits.

# Defaults

- `host`: "127.0.0.1" (localhost only)
- `port`: 8000
- `cors_origins`: empty vector (allows all origins)
- `max_request_body_bytes`: 104_857_600 (100 MB)
- `max_multipart_field_bytes`: 104_857_600 (100 MB)

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `host` | `const char*` | — | Server host address (e.g., "127.0.0.1", "0.0.0.0") |
| `port` | `uint16_t` | — | Server port number |
| `cors_origins` | `const char**` | `NULL` | CORS allowed origins. Empty vector means allow all origins. If this is an empty vector, the server will accept requests from any origin. If populated with specific origins (e.g., ["<https://example.com">]), only those origins will be allowed. |
| `max_request_body_bytes` | `uintptr_t` | — | Maximum size of request body in bytes (default: 100 MB) |
| `max_multipart_field_bytes` | `uintptr_t` | — | Maximum size of multipart fields in bytes (default: 100 MB) |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergServerConfig kreuzberg_default();
```

###### kreuzberg_listen_addr()

Get the server listen address (host:port).

**Signature:**

```c
const char* kreuzberg_listen_addr();
```

###### kreuzberg_cors_allows_all()

Check if CORS allows all origins.

Returns `true` if the `cors_origins` vector is empty, meaning all origins
are allowed. Returns `false` if specific origins are configured.

**Signature:**

```c
bool kreuzberg_cors_allows_all();
```

###### kreuzberg_is_origin_allowed()

Check if a given origin is allowed by CORS configuration.

Returns `true` if:
- CORS allows all origins (empty origins list), or
- The given origin is in the allowed origins list

**Signature:**

```c
bool kreuzberg_is_origin_allowed(const char* origin);
```

###### kreuzberg_max_request_body_mb()

Get maximum request body size in megabytes (rounded up).

**Signature:**

```c
uintptr_t kreuzberg_max_request_body_mb();
```

###### kreuzberg_max_multipart_field_mb()

Get maximum multipart field size in megabytes (rounded up).

**Signature:**

```c
uintptr_t kreuzberg_max_multipart_field_mb();
```


---

#### KreuzbergStreamReader


---

#### KreuzbergStringBufferPool

Convenience type alias for a pooled String.


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

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `const char*` | — | The extracted text content |
| `format` | `const char*` | — | Format |
| `metadata` | `void*` | — | Document metadata |
| `text_fields` | `const char**` | — | Text fields |


---

#### KreuzbergStructuredExtractionConfig

Configuration for LLM-based structured data extraction.

Sends extracted document content to a VLM with a JSON schema,
returning structured data that conforms to the schema.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `schema` | `void*` | — | JSON Schema defining the desired output structure. |
| `schema_name` | `const char*` | — | Schema name passed to the LLM's structured output mode. |
| `schema_description` | `const char**` | `NULL` | Optional schema description for the LLM. |
| `strict` | `bool` | — | Enable strict mode — output must exactly match the schema. |
| `prompt` | `const char**` | `NULL` | Custom Jinja2 extraction prompt template. When `NULL`, a default template is used. Available template variables: - `{{ content }}` — The extracted document text. - `{{ schema }}` — The JSON schema as a formatted string. - `{{ schema_name }}` — The schema name. - `{{ schema_description }}` — The schema description (may be empty). |
| `llm` | `KreuzbergLlmConfig` | — | LLM configuration for the extraction. |


---

#### KreuzbergStructuredExtractionResponse

Response from structured extraction endpoint.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `structured_output` | `void*` | — | Structured data conforming to the provided JSON schema |
| `content` | `const char*` | — | Extracted document text content |
| `mime_type` | `const char*` | — | Detected MIME type of the input file |


---

#### KreuzbergStyleDefinition

A single style definition parsed from `<w:style>` in `word/styles.xml`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `const char*` | — | The style ID (`w:styleId` attribute). |
| `name` | `const char**` | `NULL` | Human-readable name (`<w:name w:val="..."/>`). |
| `style_type` | `const char*` | — | Style type: paragraph, character, table, or numbering. |
| `based_on` | `const char**` | `NULL` | ID of the parent style (`<w:basedOn w:val="..."/>`). |
| `next_style` | `const char**` | `NULL` | ID of the style to apply to the next paragraph (`<w:next w:val="..."/>`). |
| `is_default` | `bool` | — | Whether this is the default style for its type. |
| `paragraph_properties` | `const char*` | — | Paragraph properties defined directly on this style. |
| `run_properties` | `const char*` | — | Run properties defined directly on this style. |


---

#### KreuzbergSupportedFormat

A supported document format entry.

Represents a file extension and its corresponding MIME type that Kreuzberg can process.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `extension` | `const char*` | — | File extension (without leading dot), e.g., "pdf", "docx" |
| `mime_type` | `const char*` | — | MIME type string, e.g., "application/pdf" |


---

#### KreuzbergSyncExtractor

Trait for extractors that can work synchronously (WASM-compatible).

This trait defines the synchronous extraction interface for WASM targets and other
environments where async/tokio runtimes are not available or desirable.

# Implementation

Extractors that need to support WASM should implement this trait in addition to
the async `DocumentExtractor` trait. This allows the same extractor to work in both
environments by delegating to the sync implementation.

# MIME Type Validation

The `mime_type` parameter is guaranteed to be already validated.

##### Methods

###### kreuzberg_extract_sync()

Extract content from a byte array synchronously.

This method performs extraction without requiring an async runtime.
It is called by `extract_bytes_sync()` when the `tokio-runtime` feature is disabled.

**Returns:**

An `InternalDocument` containing the extracted elements, metadata, and tables.

**Signature:**

```c
const char* kreuzberg_extract_sync(const uint8_t* content, const char* mime_type, KreuzbergExtractionConfig config);
```


---

#### KreuzbergTableProperties

Table-level properties from `<w:tblPr>`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `style_id` | `const char**` | `NULL` | Style id |
| `width` | `const char**` | `NULL` | Width |
| `alignment` | `const char**` | `NULL` | Alignment |
| `layout` | `const char**` | `NULL` | Layout |
| `look` | `const char**` | `NULL` | Look |
| `borders` | `const char**` | `NULL` | Borders |
| `cell_margins` | `const char**` | `NULL` | Cell margins |
| `indent` | `const char**` | `NULL` | Indent |
| `caption` | `const char**` | `NULL` | Caption |


---

#### KreuzbergTessdataManager

Manages tessdata file downloading, caching, and manifest generation.

##### Methods

###### kreuzberg_cache_dir()

Get the cache directory path.

**Signature:**

```c
const char* kreuzberg_cache_dir();
```

###### kreuzberg_is_language_cached()

Check if a specific language traineddata file is cached.

**Signature:**

```c
bool kreuzberg_is_language_cached(const char* lang);
```

###### kreuzberg_ensure_all_languages()

Downloads all tessdata_fast traineddata files to the cache directory.

Skips files that already exist. Returns the count of newly downloaded files.

Requires the `paddle-ocr` feature for HTTP download support (ureq).

**Signature:**

```c
uintptr_t kreuzberg_ensure_all_languages();
```


---

#### KreuzbergTesseractConfig

Tesseract OCR configuration.

Provides fine-grained control over Tesseract OCR engine parameters.
Most users can use the defaults, but these settings allow optimization
for specific document types (invoices, handwriting, etc.).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `language` | `const char*` | `"eng"` | Language code (e.g., "eng", "deu", "fra") |
| `psm` | `int32_t` | `3` | Page Segmentation Mode (0-13). Common values: - 3: Fully automatic page segmentation (default) - 6: Assume a single uniform block of text - 11: Sparse text with no particular order |
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
| `links` | `const char***` | `NULL` | Markdown links as (text, URL) tuples (Markdown files only) |
| `code_blocks` | `const char***` | `NULL` | Code blocks as (language, code) tuples (Markdown files only) |


---

#### KreuzbergTextMetadata

Text/Markdown metadata.

Extracted from plain text and Markdown files. Includes word counts and,
for Markdown, structural elements like headers and links.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `line_count` | `uintptr_t` | — | Number of lines in the document |
| `word_count` | `uintptr_t` | — | Number of words |
| `character_count` | `uintptr_t` | — | Number of characters |
| `headers` | `const char***` | `NULL` | Markdown headers (headings text only, for Markdown files) |
| `links` | `const char***` | `NULL` | Markdown links as (text, url) tuples (for Markdown files) |
| `code_blocks` | `const char***` | `NULL` | Code blocks as (language, code) tuples (for Markdown files) |


---

#### KreuzbergTokenReductionConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `level` | `KreuzbergReductionLevel` | `KREUZBERG_KREUZBERG_MODERATE` | Level (reduction level) |
| `language_hint` | `const char**` | `NULL` | Language hint |
| `preserve_markdown` | `bool` | `false` | Preserve markdown |
| `preserve_code` | `bool` | `true` | Preserve code |
| `semantic_threshold` | `float` | `0.3` | Semantic threshold |
| `enable_parallel` | `bool` | `true` | Enable parallel |
| `use_simd` | `bool` | `true` | Use simd |
| `custom_stopwords` | `void**` | `NULL` | Custom stopwords |
| `preserve_patterns` | `const char**` | `NULL` | Preserve patterns |
| `target_reduction` | `float*` | `NULL` | Target reduction |
| `enable_semantic_clustering` | `bool` | `false` | Enable semantic clustering |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergTokenReductionConfig kreuzberg_default();
```


---

#### KreuzbergTokenReductionOptions

Token reduction configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `mode` | `const char*` | — | Reduction mode: "off", "light", "moderate", "aggressive", "maximum" |
| `preserve_important_words` | `bool` | — | Preserve important words (capitalized, technical terms) |


---

#### KreuzbergTracingLayer

A `tower.Layer` that wraps each extraction in a semantic tracing span.


---

#### KreuzbergTreeSitterConfig

Configuration for tree-sitter language pack integration.

Controls grammar download behavior and code analysis options.

# Example (TOML)

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
| `languages` | `const char***` | `NULL` | Languages to pre-download on init (e.g., `["python", "rust"]`). |
| `groups` | `const char***` | `NULL` | Language groups to pre-download (e.g., `["web", "systems", "scripting"]`). |
| `process` | `KreuzbergTreeSitterProcessConfig` | — | Processing options for code analysis. |

##### Methods

###### kreuzberg_default()

**Signature:**

```c
KreuzbergTreeSitterConfig kreuzberg_default();
```


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


---

#### KreuzbergUri

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

#### KreuzbergValidator

Trait for validator plugins.

Validators check extraction results for quality, completeness, or correctness.
Unlike post-processors, validator errors **fail fast** - if a validator returns
an error, the extraction fails immediately.

# Use Cases

- **Quality Gates**: Ensure extracted content meets minimum quality standards
- **Compliance**: Verify content meets regulatory requirements
- **Content Filtering**: Reject documents containing unwanted content
- **Format Validation**: Verify extracted content structure
- **Security Checks**: Scan for malicious content

# Error Handling

Validator errors are **fatal** - they cause the extraction to fail and bubble up
to the caller. Use validators for hard requirements that must be met.

For non-fatal checks, use post-processors instead.

# Thread Safety

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

# Example - Content Length Validation


# Example - Quality Score Validation


# Example - Security Validation

**Signature:**

```c
void kreuzberg_validate(KreuzbergExtractionResult result, KreuzbergExtractionConfig config);
```

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


---

#### KreuzbergVersionResponse

Version response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `version` | `const char*` | — | Kreuzberg version string |


---

#### KreuzbergWarmRequest

Cache warm request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `all_embeddings` | `bool` | — | Download all embedding model presets |
| `embedding_model` | `const char**` | `NULL` | Specific embedding model preset to download |


---

#### KreuzbergWarmResponse

Cache warm response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `cache_dir` | `const char*` | — | Cache directory used |
| `downloaded` | `const char**` | — | Models that were downloaded |
| `already_cached` | `const char**` | — | Models that were already cached |


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
| `element_count` | `uintptr_t` | — | Total number of XML elements processed |
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


---

#### KreuzbergYearRange

Year range for bibliographic metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `min` | `uint32_t*` | `NULL` | Min |
| `max` | `uint32_t*` | `NULL` | Max |
| `years` | `uint32_t*` | — | Years |


---

#### KreuzbergZipBombValidator

Helper struct for validating ZIP archives for security issues.


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
table regions.

| Value | Description |
|-------|-------------|
| `KREUZBERG_TATR` | TATR (Table Transformer) -- default, 30MB, DETR-based row/column detection. |
| `KREUZBERG_SLANET_WIRED` | SLANeXT wired variant -- 365MB, optimized for bordered tables. |
| `KREUZBERG_SLANET_WIRELESS` | SLANeXT wireless variant -- 365MB, optimized for borderless tables. |
| `KREUZBERG_SLANET_PLUS` | SLANet-plus -- 7.78MB, lightweight general-purpose. |
| `KREUZBERG_SLANET_AUTO` | Classifier-routed SLANeXT: auto-select wired/wireless per table. Uses PP-LCNet classifier (6.78MB) + both SLANeXT variants (730MB total). |
| `KREUZBERG_DISABLED` | Disable table structure model inference entirely; use heuristic path only. |


---

#### KreuzbergPdfBackend

PDF extraction backend selection.

Controls which PDF library is used for text extraction:
- `Pdfium`: pdfium-render (default, C++ based, mature)
- `PdfOxide`: pdf_oxide (pure Rust, faster, requires `pdf-oxide` feature)
- `Auto`: automatically select based on available features

| Value | Description |
|-------|-------------|
| `KREUZBERG_PDFIUM` | Use pdfium-render backend (default). |
| `KREUZBERG_PDF_OXIDE` | Use pdf_oxide backend (pure Rust). Requires `pdf-oxide` feature. |
| `KREUZBERG_AUTO` | Automatically select the best available backend. |


---

#### KreuzbergChunkerType

Type of text chunker to use.

# Variants

* `Text` - Generic text splitter, splits on whitespace and punctuation
* `Markdown` - Markdown-aware splitter, preserves formatting and structure
* `Yaml` - YAML-aware splitter, creates one chunk per top-level key
* `Semantic` - Topic-aware chunker. With an `EmbeddingConfig`, splits at
  embedding-based topic shifts tuned by `topic_threshold` (default 0.75,
  lower = more splits). Without an embedding, falls back to a
  structural-boundary heuristic (ALL-CAPS headers, numbered sections,
  blank-line paragraphs) and merges groups into chunks capped at
  `max_characters` (default 1000). `topic_threshold` has no effect in the
  fallback path. For best results, pair with an embedding model.

| Value | Description |
|-------|-------------|
| `KREUZBERG_TEXT` | Text format |
| `KREUZBERG_MARKDOWN` | Markdown format |
| `KREUZBERG_YAML` | Yaml format |
| `KREUZBERG_SEMANTIC` | Semantic |


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
| `KREUZBERG_PLUGIN` | In-process embedding backend registered via the plugin system. The caller registers an `EmbeddingBackend` once (e.g. a wrapper around an already-loaded `llama-cpp-python`, `sentence-transformers`, or tuned ONNX model), then references it by name in config. Kreuzberg calls back into the registered backend during chunking and standalone embed requests — no HuggingFace download, no ONNX Runtime requirement, no HTTP sidecar. When this variant is selected, only the following `EmbeddingConfig` fields apply: `normalize` (post-call L2 normalization) and `max_embed_duration_secs` (dispatcher timeout). Model-loading fields (`batch_size`, `cache_dir`, `show_download_progress`, `acceleration`) are ignored — the host owns the model lifecycle. Semantic chunking falls back to `ChunkingConfig.max_characters` when this variant is used, since there is no preset to look a chunk-size ceiling up against — size your context window via `max_characters` directly. See `crate.plugins.register_embedding_backend`. — Fields: `name`: `const char*` |


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

#### KreuzbergFracType

| Value | Description |
|-------|-------------|
| `KREUZBERG_BAR` | Bar |
| `KREUZBERG_NO_BAR` | No bar |
| `KREUZBERG_LINEAR` | Linear |
| `KREUZBERG_SKEWED` | Skewed |


---

#### KreuzbergOcrBackendType

OCR backend types.

| Value | Description |
|-------|-------------|
| `KREUZBERG_TESSERACT` | Tesseract OCR (native Rust binding) |
| `KREUZBERG_EASY_OCR` | EasyOCR (Python-based, via FFI) |
| `KREUZBERG_PADDLE_OCR` | PaddleOCR (Python-based, via FFI) |
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

| Value | Description |
|-------|-------------|
| `KREUZBERG_OFF` | Off |
| `KREUZBERG_LIGHT` | Light |
| `KREUZBERG_MODERATE` | Moderate |
| `KREUZBERG_AGGRESSIVE` | Aggressive |
| `KREUZBERG_MAXIMUM` | Maximum |


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
| `KREUZBERG_PARAGRAPH` | Paragraph element |
| `KREUZBERG_HEADING` | Heading element |
| `KREUZBERG_BLOCKQUOTE` | Blockquote element |
| `KREUZBERG_CODE_BLOCK` | Code block |
| `KREUZBERG_LIST_ITEM` | List item |
| `KREUZBERG_ORDERED_LIST` | Ordered list |
| `KREUZBERG_BULLET_LIST` | Bullet list |
| `KREUZBERG_TASK_LIST` | Task list |
| `KREUZBERG_DEFINITION_LIST` | Definition list |
| `KREUZBERG_DEFINITION_TERM` | Definition term |
| `KREUZBERG_DEFINITION_DESCRIPTION` | Definition description |
| `KREUZBERG_DIV` | Div |
| `KREUZBERG_SECTION` | Section element |
| `KREUZBERG_THEMATIC_BREAK` | Thematic break |
| `KREUZBERG_RAW_BLOCK` | Raw block |
| `KREUZBERG_MATH_DISPLAY` | Math display |


---

#### KreuzbergInlineType

Types of inline elements in Djot.

| Value | Description |
|-------|-------------|
| `KREUZBERG_TEXT` | Text format |
| `KREUZBERG_STRONG` | Strong |
| `KREUZBERG_EMPHASIS` | Emphasis |
| `KREUZBERG_HIGHLIGHT` | Highlight |
| `KREUZBERG_SUBSCRIPT` | Subscript |
| `KREUZBERG_SUPERSCRIPT` | Superscript |
| `KREUZBERG_INSERT` | Insert |
| `KREUZBERG_DELETE` | Delete |
| `KREUZBERG_CODE` | Code |
| `KREUZBERG_LINK` | Link |
| `KREUZBERG_IMAGE` | Image element |
| `KREUZBERG_SPAN` | Span |
| `KREUZBERG_MATH` | Math |
| `KREUZBERG_RAW_INLINE` | Raw inline |
| `KREUZBERG_FOOTNOTE_REF` | Footnote ref |
| `KREUZBERG_SYMBOL` | Symbol |


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
| `KREUZBERG_TABLE` | Table with structured cell grid. — Fields: `grid`: `const char*` |
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
| `KREUZBERG_METADATA_BLOCK` | Structured metadata block (email headers, YAML frontmatter, etc.). — Fields: `entries`: `const char**` |


---

#### KreuzbergAnnotationKind

Types of inline text annotations.

| Value | Description |
|-------|-------------|
| `KREUZBERG_BOLD` | Bold |
| `KREUZBERG_ITALIC` | Italic |
| `KREUZBERG_UNDERLINE` | Underline |
| `KREUZBERG_STRIKETHROUGH` | Strikethrough |
| `KREUZBERG_CODE` | Code |
| `KREUZBERG_SUBSCRIPT` | Subscript |
| `KREUZBERG_SUPERSCRIPT` | Superscript |
| `KREUZBERG_LINK` | Link — Fields: `url`: `const char*`, `title`: `const char*` |
| `KREUZBERG_HIGHLIGHT` | Highlighted text (PDF highlights, HTML `<mark>`). |
| `KREUZBERG_COLOR` | Text color (CSS-compatible value, e.g. "#ff0000", "red"). — Fields: `value`: `const char*` |
| `KREUZBERG_FONT_SIZE` | Font size with units (e.g. "12pt", "1.2em", "16px"). — Fields: `value`: `const char*` |
| `KREUZBERG_CUSTOM` | Extensible annotation for format-specific styling. — Fields: `name`: `const char*`, `value`: `const char*` |


---

#### KreuzbergExtractionMethod

How the extracted text was produced.

| Value | Description |
|-------|-------------|
| `KREUZBERG_NATIVE` | Native |
| `KREUZBERG_OCR` | Ocr |
| `KREUZBERG_MIXED` | Mixed |


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

#### KreuzbergFormatMetadata

Format-specific metadata (discriminated union).

Only one format type can exist per extraction result. This provides
type-safe, clean metadata without nested optionals.

| Value | Description |
|-------|-------------|
| `KREUZBERG_PDF` | Pdf format — Fields: `0`: `const char*` |
| `KREUZBERG_DOCX` | Docx format — Fields: `0`: `KreuzbergDocxMetadata` |
| `KREUZBERG_EXCEL` | Excel — Fields: `0`: `KreuzbergExcelMetadata` |
| `KREUZBERG_EMAIL` | Email — Fields: `0`: `KreuzbergEmailMetadata` |
| `KREUZBERG_PPTX` | Pptx format — Fields: `0`: `KreuzbergPptxMetadata` |
| `KREUZBERG_ARCHIVE` | Archive — Fields: `0`: `KreuzbergArchiveMetadata` |
| `KREUZBERG_IMAGE` | Image element — Fields: `0`: `const char*` |
| `KREUZBERG_XML` | Xml format — Fields: `0`: `KreuzbergXmlMetadata` |
| `KREUZBERG_TEXT` | Text format — Fields: `0`: `KreuzbergTextMetadata` |
| `KREUZBERG_HTML` | Preserve as HTML `<mark>` tags — Fields: `0`: `KreuzbergHtmlMetadata` |
| `KREUZBERG_OCR` | Ocr — Fields: `0`: `KreuzbergOcrMetadata` |
| `KREUZBERG_CSV` | Csv format — Fields: `0`: `KreuzbergCsvMetadata` |
| `KREUZBERG_BIBTEX` | Bibtex — Fields: `0`: `KreuzbergBibtexMetadata` |
| `KREUZBERG_CITATION` | Citation — Fields: `0`: `KreuzbergCitationMetadata` |
| `KREUZBERG_FICTION_BOOK` | Fiction book — Fields: `0`: `KreuzbergFictionBookMetadata` |
| `KREUZBERG_DBF` | Dbf — Fields: `0`: `KreuzbergDbfMetadata` |
| `KREUZBERG_JATS` | Jats — Fields: `0`: `KreuzbergJatsMetadata` |
| `KREUZBERG_EPUB` | Epub format — Fields: `0`: `KreuzbergEpubMetadata` |
| `KREUZBERG_PST` | Pst — Fields: `0`: `KreuzbergPstMetadata` |
| `KREUZBERG_CODE` | Code — Fields: `0`: `const char*` |


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
| `KREUZBERG_QUADRILATERAL` | 4-point quadrilateral for rotated/skewed text (PaddleOCR). Points are in clockwise order starting from top-left: `[top_left, top_right, bottom_right, bottom_left]` — Fields: `points`: `const char*` |


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

#### KreuzbergPoolError

Error type for pool operations.

| Value | Description |
|-------|-------------|
| `KREUZBERG_LOCK_POISONED` | The pool's internal mutex was poisoned. This indicates a panic occurred while holding the lock. The pool is in a locked state and cannot be recovered. |


---

#### KreuzbergKeywordAlgorithm

Keyword algorithm selection.

| Value | Description |
|-------|-------------|
| `KREUZBERG_YAKE` | YAKE (Yet Another Keyword Extractor) - statistical approach |
| `KREUZBERG_RAKE` | RAKE (Rapid Automatic Keyword Extraction) - co-occurrence based |


---

#### KreuzbergPsmMode

Page Segmentation Mode for Tesseract OCR

| Value | Description |
|-------|-------------|
| `KREUZBERG_OSD_ONLY` | Osd only |
| `KREUZBERG_AUTO_OSD` | Auto osd |
| `KREUZBERG_AUTO_ONLY` | Auto only |
| `KREUZBERG_AUTO` | Auto |
| `KREUZBERG_SINGLE_COLUMN` | Single column |
| `KREUZBERG_SINGLE_BLOCK_VERTICAL` | Single block vertical |
| `KREUZBERG_SINGLE_BLOCK` | Single block |
| `KREUZBERG_SINGLE_LINE` | Single line |
| `KREUZBERG_SINGLE_WORD` | Single word |
| `KREUZBERG_CIRCLE_WORD` | Circle word |
| `KREUZBERG_SINGLE_CHAR` | Single char |


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

The 17 canonical document layout classes.

All model backends (RT-DETR, YOLO, etc.) map their native class IDs
to this shared set. Models with fewer classes (DocLayNet: 11, PubLayNet: 5)
map to the closest equivalent.

| Value | Description |
|-------|-------------|
| `KREUZBERG_CAPTION` | Caption element |
| `KREUZBERG_FOOTNOTE` | Footnote element |
| `KREUZBERG_FORMULA` | Formula |
| `KREUZBERG_LIST_ITEM` | List item |
| `KREUZBERG_PAGE_FOOTER` | Page footer |
| `KREUZBERG_PAGE_HEADER` | Page header |
| `KREUZBERG_PICTURE` | Picture |
| `KREUZBERG_SECTION_HEADER` | Section header |
| `KREUZBERG_TABLE` | Table element |
| `KREUZBERG_TEXT` | Text format |
| `KREUZBERG_TITLE` | Title element |
| `KREUZBERG_DOCUMENT_INDEX` | Document index |
| `KREUZBERG_CODE` | Code |
| `KREUZBERG_CHECKBOX_SELECTED` | Checkbox selected |
| `KREUZBERG_CHECKBOX_UNSELECTED` | Checkbox unselected |
| `KREUZBERG_FORM` | Form |
| `KREUZBERG_KEY_VALUE_REGION` | Key value region |


---

### Errors

#### KreuzbergKreuzbergError

Main error type for all Kreuzberg operations.

All errors in Kreuzberg use this enum, which preserves error chains
and provides context for debugging.

# Variants

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
| `KREUZBERG_IO` | IO error: {0} |
| `KREUZBERG_PARSING` | Parsing error: {message} |
| `KREUZBERG_OCR` | OCR error: {message} |
| `KREUZBERG_VALIDATION` | Validation error: {message} |
| `KREUZBERG_CACHE` | Cache error: {message} |
| `KREUZBERG_IMAGE_PROCESSING` | Image processing error: {message} |
| `KREUZBERG_SERIALIZATION` | Serialization error: {message} |
| `KREUZBERG_MISSING_DEPENDENCY` | Missing dependency: {0} |
| `KREUZBERG_PLUGIN` | Plugin error in '{plugin_name}': {message} |
| `KREUZBERG_LOCK_POISONED` | Lock poisoned: {0} |
| `KREUZBERG_UNSUPPORTED_FORMAT` | Unsupported format: {0} |
| `KREUZBERG_EMBEDDING` | Embedding error: {message} |
| `KREUZBERG_TIMEOUT` | Extraction timed out after {elapsed_ms}ms (limit: {limit_ms}ms) |
| `KREUZBERG_CANCELLED` | Extraction cancelled |
| `KREUZBERG_SECURITY` | Security violation: {message} |
| `KREUZBERG_OTHER` | {0} |


---

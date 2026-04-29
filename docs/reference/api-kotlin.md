---
title: "Kotlin API Reference"
---

## Kotlin API Reference <span class="version-badge">v4.10.0-rc.8</span>

### Functions

#### blake3HashBytes()

Hash arbitrary bytes with blake3, returning a 32-char hex string.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `data` | `ByteArray` | Yes | The data |

**Returns:** `String`


---

#### blake3HashFile()

Hash a file's content with blake3 using streaming 64 KiB reads.

Returns a 32-char hex string (128 bits of blake3 output).

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `path` | `Path` | Yes | Path to the file |

**Returns:** `String`

**Errors:** Throws `Error`.


---

#### fastHash()

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `data` | `ByteArray` | Yes | The data |

**Returns:** `Long`


---

#### validateCacheKey()

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `key` | `String` | Yes | The key |

**Returns:** `Boolean`


---

#### validatePort()

Validate a port number for server configuration.

Port must be in the range 1-65535. While ports 1-1023 are privileged and may require
special permissions on some systems, they are still valid port numbers.

**Returns:**

`Ok(())` if the port is valid, or a `ValidationError` with details about valid ranges.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `port` | `Short` | Yes | The port number to validate |

**Returns:** `Unit`

**Errors:** Throws `Error`.


---

#### validateHost()

Validate a host/IP address string for server configuration.

Accepts valid IPv4 addresses (e.g., "127.0.0.1", "0.0.0.0"), valid IPv6 addresses
(e.g., ".1", "."), and hostnames (e.g., "localhost", "example.com").

**Returns:**

`Ok(())` if the host is valid, or a `ValidationError` with details about valid formats.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `host` | `String` | Yes | The host/IP address string to validate |

**Returns:** `Unit`

**Errors:** Throws `Error`.


---

#### validateCorsOrigin()

Validate a CORS (Cross-Origin Resource Sharing) origin URL.

Accepts valid HTTP/HTTPS URLs (e.g., "<https://example.com">) or the wildcard "*"
to allow all origins. URLs must start with "<http://"> or "<https://",> or be exactly "*".

**Returns:**

`Ok(())` if the origin is valid, or a `ValidationError` with details about valid formats.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `origin` | `String` | Yes | The CORS origin URL to validate |

**Returns:** `Unit`

**Errors:** Throws `Error`.


---

#### validateUploadSize()

Validate an upload size limit for server configuration.

Upload size must be greater than 0 (measured in bytes).

**Returns:**

`Ok(())` if the size is valid, or a `ValidationError` with details about constraints.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `size` | `Long` | Yes | The maximum upload size in bytes to validate |

**Returns:** `Unit`

**Errors:** Throws `Error`.


---

#### validateBinarizationMethod()

Validate a binarization method string.

**Returns:**

`Ok(())` if the method is valid, or a `ValidationError` with details about valid options.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `method` | `String` | Yes | The binarization method to validate (e.g., "otsu", "adaptive", "sauvola") |

**Returns:** `Unit`

**Errors:** Throws `Error`.


---

#### validateTokenReductionLevel()

Validate a token reduction level string.

**Returns:**

`Ok(())` if the level is valid, or a `ValidationError` with details about valid options.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `level` | `String` | Yes | The token reduction level to validate (e.g., "off", "light", "moderate") |

**Returns:** `Unit`

**Errors:** Throws `Error`.


---

#### validateOcrBackend()

Validate an OCR backend string.

**Returns:**

`Ok(())` if the backend is valid, or a `ValidationError` with details about valid options.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `backend` | `String` | Yes | The OCR backend to validate (e.g., "tesseract", "easyocr", "paddleocr") |

**Returns:** `Unit`

**Errors:** Throws `Error`.


---

#### validateLanguageCode()

Validate a language code (ISO 639-1 or 639-3 format).

Accepts both 2-letter ISO 639-1 codes (e.g., "en", "de") and
3-letter ISO 639-3 codes (e.g., "eng", "deu") for broader compatibility.

**Returns:**

`Ok(())` if the code is valid, or a `ValidationError` indicating an invalid language code.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `code` | `String` | Yes | The language code to validate |

**Returns:** `Unit`

**Errors:** Throws `Error`.


---

#### validateTesseractPsm()

Validate a tesseract Page Segmentation Mode (PSM).

**Returns:**

`Ok(())` if the PSM is valid, or a `ValidationError` with details about valid ranges.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `psm` | `Int` | Yes | The PSM value to validate (0-13) |

**Returns:** `Unit`

**Errors:** Throws `Error`.


---

#### validateTesseractOem()

Validate a tesseract OCR Engine Mode (OEM).

**Returns:**

`Ok(())` if the OEM is valid, or a `ValidationError` with details about valid options.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `oem` | `Int` | Yes | The OEM value to validate (0-3) |

**Returns:** `Unit`

**Errors:** Throws `Error`.


---

#### validateOutputFormat()

Validate a document extraction output format.

Accepts the following formats and aliases:
- "plain" or "text" for plain text output
- "markdown" or "md" for Markdown output
- "djot" for Djot markup format
- "html" for HTML output

**Returns:**

`Ok(())` if the format is valid, or a `ValidationError` with details about valid options.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `format` | `String` | Yes | The output format to validate |

**Returns:** `Unit`

**Errors:** Throws `Error`.


---

#### validateConfidence()

Validate a confidence threshold value.

Confidence thresholds should be between 0.0 and 1.0 inclusive.

**Returns:**

`Ok(())` if the confidence is valid, or a `ValidationError` with details about valid ranges.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `confidence` | `Double` | Yes | The confidence threshold to validate |

**Returns:** `Unit`

**Errors:** Throws `Error`.


---

#### validateDpi()

Validate a DPI (dots per inch) value.

DPI should be a positive integer, typically 72-600.

**Returns:**

`Ok(())` if the DPI is valid, or a `ValidationError` with details about valid ranges.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `dpi` | `Int` | Yes | The DPI value to validate |

**Returns:** `Unit`

**Errors:** Throws `Error`.


---

#### validateChunkingParams()

Validate chunk size parameters.

Checks that max_chars > 0 and max_overlap < max_chars.

**Returns:**

`Ok(())` if the parameters are valid, or a `ValidationError` with details about constraints.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `maxChars` | `Long` | Yes | The maximum characters per chunk |
| `maxOverlap` | `Long` | Yes | The maximum overlap between chunks |

**Returns:** `Unit`

**Errors:** Throws `Error`.


---

#### validateLlmConfigModel()

Validate that an `LlmConfig` has a non-empty model string.

**Returns:**

`Ok(())` if the model is non-empty, or a `ValidationError` otherwise.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `model` | `String` | Yes | The model string to validate |

**Returns:** `Unit`

**Errors:** Throws `Error`.


---

#### extractBytes()

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

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `content` | `ByteArray` | Yes | The byte array to extract |
| `mimeType` | `String` | Yes | MIME type of the content |
| `config` | `ExtractionConfig` | Yes | Extraction configuration |

**Returns:** `ExtractionResult`

**Errors:** Throws `Error`.


---

#### extractFile()

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

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `path` | `Path` | Yes | Path to the file to extract |
| `mimeType` | `String?` | No | Optional MIME type override. If None, will be auto-detected |
| `config` | `ExtractionConfig` | Yes | Extraction configuration |

**Returns:** `ExtractionResult`

**Errors:** Throws `Error`.


---

#### extractFileSync()

Synchronous wrapper for `extract_file`.

This is a convenience function that blocks the current thread until extraction completes.
For async code, use `extract_file` directly.

Uses the global Tokio runtime for 100x+ performance improvement over creating
a new runtime per call. Always uses the global runtime to avoid nested runtime issues.

This function is only available with the `tokio-runtime` feature. For WASM targets,
use a truly synchronous extraction approach instead.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `path` | `Path` | Yes | Path to the file |
| `mimeType` | `String?` | No | The mime type |
| `config` | `ExtractionConfig` | Yes | The configuration options |

**Returns:** `ExtractionResult`

**Errors:** Throws `Error`.


---

#### extractBytesSync()

Synchronous wrapper for `extract_bytes`.

Uses the global Tokio runtime for 100x+ performance improvement over creating
a new runtime per call.

With the `tokio-runtime` feature, this blocks the current thread using the global
Tokio runtime. Without it (WASM), this calls a truly synchronous implementation.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `content` | `ByteArray` | Yes | The content to process |
| `mimeType` | `String` | Yes | The mime type |
| `config` | `ExtractionConfig` | Yes | The configuration options |

**Returns:** `ExtractionResult`

**Errors:** Throws `Error`.


---

#### batchExtractFileSync()

Synchronous wrapper for `batch_extract_file`.

Uses the global Tokio runtime for optimal performance.
Only available with `tokio-runtime` (WASM has no filesystem).

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `items` | `List<String>` | Yes | The items |
| `config` | `ExtractionConfig` | Yes | The configuration options |

**Returns:** `List<ExtractionResult>`

**Errors:** Throws `Error`.


---

#### batchExtractBytesSync()

Synchronous wrapper for `batch_extract_bytes`.

Uses the global Tokio runtime for optimal performance.
With the `tokio-runtime` feature, this blocks the current thread using the global
Tokio runtime. Without it (WASM), this calls a truly synchronous implementation
that iterates through items and calls `extract_bytes_sync()`.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `items` | `List<String>` | Yes | The items |
| `config` | `ExtractionConfig` | Yes | The configuration options |

**Returns:** `List<ExtractionResult>`

**Errors:** Throws `Error`.


---

#### batchExtractFile()

Extract content from multiple files concurrently.

This function processes multiple files in parallel, automatically managing
concurrency to prevent resource exhaustion. The concurrency limit can be
configured via `ExtractionConfig.max_concurrent_extractions` or defaults
to `(num_cpus * 1.5).ceil()`.

Each file can optionally specify a `FileExtractionConfig` that overrides specific
fields from the batch-level `config`. Pass `null` for a file to use the batch defaults.
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

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `items` | `List<String>` | Yes | Vector of `(path, optional_file_config)` tuples. Pass `None` as the |
| `config` | `ExtractionConfig` | Yes | Batch-level extraction configuration (provides defaults and batch settings) |

**Returns:** `List<ExtractionResult>`

**Errors:** Throws `Error`.


---

#### batchExtractBytes()

Extract content from multiple byte arrays concurrently.

This function processes multiple byte arrays in parallel, automatically managing
concurrency to prevent resource exhaustion. The concurrency limit can be
configured via `ExtractionConfig.max_concurrent_extractions` or defaults
to `(num_cpus * 1.5).ceil()`.

Each item can optionally specify a `FileExtractionConfig` that overrides specific
fields from the batch-level `config`. Pass `null` as the config to use
the batch-level defaults for that item.

**Returns:**

A vector of `ExtractionResult` in the same order as the input items.

Simple usage with no per-item overrides:


Per-item configuration overrides:

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `items` | `List<String>` | Yes | Vector of `(bytes, mime_type, optional_file_config)` tuples |
| `config` | `ExtractionConfig` | Yes | Batch-level extraction configuration |

**Returns:** `List<ExtractionResult>`

**Errors:** Throws `Error`.


---

#### isValidFormatField()

Validates whether a field name is in the known formats registry.

This uses a pre-built hash set for O(1) lookups instead of linear search,
providing significant performance improvements for repeated validations.

**Returns:**

`true` if the field is in KNOWN_FORMATS, `false` otherwise.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `field` | `String` | Yes | The field name to validate |

**Returns:** `Boolean`


---

#### validateMimeType()

Validate that a MIME type is supported.

**Returns:**

The validated MIME type (may be normalized).

**Errors:**

Returns `KreuzbergError.UnsupportedFormat` if not supported.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `mimeType` | `String` | Yes | The MIME type to validate |

**Returns:** `String`

**Errors:** Throws `Error`.


---

#### detectOrValidate()

Detect or validate MIME type.

If `mime_type` is provided, validates it. Otherwise, detects from `path`.

**Returns:**

The validated MIME type string.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `path` | `String?` | No | Optional path to detect MIME type from |
| `mimeType` | `String?` | No | Optional explicit MIME type to validate |

**Returns:** `String`

**Errors:** Throws `Error`.


---

#### detectMimeTypeFromBytes()

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

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `content` | `ByteArray` | Yes | Raw file bytes |

**Returns:** `String`

**Errors:** Throws `Error`.


---

#### getExtensionsForMime()

Get file extensions for a given MIME type.

Returns all known file extensions that map to the specified MIME type.

**Returns:**

A vector of file extensions (without leading dot) for the MIME type.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `mimeType` | `String` | Yes | The MIME type to look up |

**Returns:** `List<String>`

**Errors:** Throws `Error`.


---

#### listSupportedFormats()

List all supported document formats.

Returns a list of all file extensions and their corresponding MIME types
that Kreuzberg can process. Derived from the centralized `FORMATS` registry.

The list is sorted alphabetically by file extension.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Returns:** `List<SupportedFormat>`


---

#### clearProcessorCache()

Clear the processor cache (primarily for testing when registry changes).

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Returns:** `Unit`

**Errors:** Throws `Error`.


---

#### transformExtractionResultToElements()

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

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `result` | `ExtractionResult` | Yes | Reference to the ExtractionResult to transform |

**Returns:** `List<Element>`


---

#### extractEmailContent()

Extract email content from either .eml or .msg format

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `data` | `ByteArray` | Yes | The data |
| `mimeType` | `String` | Yes | The mime type |
| `fallbackCodepage` | `Int?` | No | The fallback codepage |

**Returns:** `EmailExtractionResult`

**Errors:** Throws `Error`.


---

#### cellsToText()

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

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `cells` | `List<List<String>>` | Yes | A slice of vectors representing table rows, where each inner vector contains cell values |

**Returns:** `String`


---

#### cellsToMarkdown()

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `cells` | `List<List<String>>` | Yes | The cells |

**Returns:** `String`


---

#### djotToHtml()

Render djot content to HTML.

This function takes djot source text and renders it to HTML using jotdown's
built-in HTML renderer.

**Returns:**

A `Result` containing the rendered HTML string

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `djotSource` | `String` | Yes | The djot markup text to render |

**Returns:** `String`

**Errors:** Throws `Error`.


---

#### dedupText()

Deduplicate a list of text strings while preserving order.
Adjacent duplicates and near-duplicates are removed.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `texts` | `List<String>` | Yes | The texts |

**Returns:** `List<String>`


---

#### normalizeWhitespace()

Normalize whitespace in a string.

- Collapses multiple consecutive spaces/tabs into a single space
- Preserves single newlines (paragraph breaks from \par)
- Collapses multiple consecutive newlines into a double newline
- Trims leading/trailing whitespace from each line
- Trims leading/trailing blank lines

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `s` | `String` | Yes | The s |

**Returns:** `String`


---

#### registerDefaultExtractors()

Register all built-in extractors with the global registry.

This function should be called once at application startup to register
the default extractors (PlainText, Markdown, XML, etc.).

**Note:** This is called automatically on first extraction operation.
Explicit calling is optional.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Returns:** `Unit`

**Errors:** Throws `Error`.


---

#### unregisterExtractor()

Unregister a document extractor by name.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `name` | `String` | Yes | The name |

**Returns:** `Unit`

**Errors:** Throws `Error`.


---

#### listExtractors()

List names of all registered document extractors.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Returns:** `List<String>`

**Errors:** Throws `Error`.


---

#### clearExtractors()

Remove all registered document extractors.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Returns:** `Unit`

**Errors:** Throws `Error`.


---

#### unregisterOcrBackend()

Unregister an OCR backend by name.

Removes the OCR backend from the global registry and calls its `shutdown()` method.

**Returns:**

- `Ok(())` if the backend was unregistered or didn't exist
- `Err(...)` if the shutdown method failed

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `name` | `String` | Yes | Name of the OCR backend to unregister |

**Returns:** `Unit`

**Errors:** Throws `Error`.


---

#### listOcrBackends()

List all registered OCR backends.

Returns the names of all OCR backends currently registered in the global registry.

**Returns:**

A vector of OCR backend names.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Returns:** `List<String>`

**Errors:** Throws `Error`.


---

#### clearOcrBackends()

Clear all OCR backends from the global registry.

Removes all OCR backends and calls their `shutdown()` methods.

**Returns:**

- `Ok(())` if all backends were cleared successfully
- `Err(...)` if any shutdown method failed

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Returns:** `Unit`

**Errors:** Throws `Error`.


---

#### listPostProcessors()

List all registered post-processor names.

Returns a vector of all post-processor names currently registered in the
global registry.

**Returns:**

- `Ok(Vec<String>)` - Vector of post-processor names
- `Err(...)` if the registry lock is poisoned

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Returns:** `List<String>`

**Errors:** Throws `Error`.


---

#### unregisterRenderer()

Unregister a renderer by name.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `name` | `String` | Yes | The name |

**Returns:** `Unit`

**Errors:** Throws `Error`.


---

#### listRenderers()

List names of all registered renderers.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Returns:** `List<String>`

**Errors:** Throws `Error`.


---

#### clearRenderers()

Remove all registered renderers.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Returns:** `Unit`

**Errors:** Throws `Error`.


---

#### listValidators()

List names of all registered validators.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Returns:** `List<String>`

**Errors:** Throws `Error`.


---

#### clearValidators()

Remove all registered validators.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Returns:** `Unit`

**Errors:** Throws `Error`.


---

#### sanitizeFilename()

Sanitize a file path to return only the filename (no directory).

Prevents PII from appearing in traces.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `path` | `Path` | Yes | Path to the file |

**Returns:** `String`


---

#### sanitizePath()

Sanitize a file path to return only the filename.

Prevents PII (personally identifiable information) from appearing in
traces by only recording filenames instead of full paths.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `path` | `Path` | Yes | Path to the file |

**Returns:** `String`


---

#### isValidUtf8()

Validates bytes as UTF-8 without conversion to string slice.

Returns `true` if the bytes represent valid UTF-8, `false` otherwise.
This is useful when you only need to check validity without constructing a string.

**Returns:**

`true` if valid UTF-8, `false` otherwise.

# Performance

This function is optimized for early exit on invalid sequences.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `bytes` | `ByteArray` | Yes | The byte slice to validate |

**Returns:** `Boolean`


---

#### cleanExtractedText()

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `String` | Yes | The text |

**Returns:** `String`


---

#### reduceTokens()

Reduces token count in text while preserving meaning and structure.

This function removes stopwords, redundancy, and applies compression techniques
based on the specified reduction level. Supports 64 languages with automatic
stopword removal and optional semantic clustering.

**Returns:**

Returns the reduced text with preserved structure (markdown, code blocks).

**Errors:**

Returns an error if the language hint is invalid or stopwords cannot be loaded.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `String` | Yes | The input text to reduce |
| `config` | `TokenReductionConfig` | Yes | Configuration specifying reduction level and options |
| `languageHint` | `String?` | No | Optional ISO 639-3 language code (e.g., "eng", "spa") |

**Returns:** `String`

**Errors:** Throws `Error`.


---

#### batchReduceTokens()

Reduces token count for multiple texts efficiently using parallel processing.

This function processes multiple texts in parallel using Rayon, providing
significant performance improvements for batch operations. All texts use the
same configuration and language hint for consistency.

**Returns:**

Returns a vector of reduced texts in the same order as the input.

**Errors:**

Returns an error if the language hint is invalid or stopwords cannot be loaded.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `texts` | `List<String>` | Yes | Slice of text references to reduce |
| `config` | `TokenReductionConfig` | Yes | Configuration specifying reduction level and options |
| `languageHint` | `String?` | No | Optional ISO 639-3 language code (e.g., "eng", "spa") |

**Returns:** `List<String>`

**Errors:** Throws `Error`.


---

#### bold()

Create a bold annotation for the given byte range.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `start` | `Int` | Yes | The start |
| `end` | `Int` | Yes | The end |

**Returns:** `TextAnnotation`


---

#### italic()

Create an italic annotation for the given byte range.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `start` | `Int` | Yes | The start |
| `end` | `Int` | Yes | The end |

**Returns:** `TextAnnotation`


---

#### underline()

Create an underline annotation for the given byte range.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `start` | `Int` | Yes | The start |
| `end` | `Int` | Yes | The end |

**Returns:** `TextAnnotation`


---

#### link()

Create a link annotation for the given byte range.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `start` | `Int` | Yes | The start |
| `end` | `Int` | Yes | The end |
| `url` | `String` | Yes | The URL to fetch |
| `title` | `String?` | No | The title |

**Returns:** `TextAnnotation`


---

#### code()

Create a code (inline) annotation for the given byte range.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `start` | `Int` | Yes | The start |
| `end` | `Int` | Yes | The end |

**Returns:** `TextAnnotation`


---

#### strikethrough()

Create a strikethrough annotation for the given byte range.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `start` | `Int` | Yes | The start |
| `end` | `Int` | Yes | The end |

**Returns:** `TextAnnotation`


---

#### subscript()

Create a subscript annotation for the given byte range.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `start` | `Int` | Yes | The start |
| `end` | `Int` | Yes | The end |

**Returns:** `TextAnnotation`


---

#### superscript()

Create a superscript annotation for the given byte range.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `start` | `Int` | Yes | The start |
| `end` | `Int` | Yes | The end |

**Returns:** `TextAnnotation`


---

#### fontSize()

Create a font size annotation for the given byte range.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `start` | `Int` | Yes | The start |
| `end` | `Int` | Yes | The end |
| `value` | `String` | Yes | The value |

**Returns:** `TextAnnotation`


---

#### color()

Create a color annotation for the given byte range.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `start` | `Int` | Yes | The start |
| `end` | `Int` | Yes | The end |
| `value` | `String` | Yes | The value |

**Returns:** `TextAnnotation`


---

#### highlight()

Create a highlight annotation for the given byte range.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `start` | `Int` | Yes | The start |
| `end` | `Int` | Yes | The end |

**Returns:** `TextAnnotation`


---

#### classifyUri()

Classify a URL string into the appropriate `UriKind`.

- `mailto:` → `Email`
- `#` prefix → `Anchor`
- everything else → `Hyperlink`

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `url` | `String` | Yes | The URL to fetch |

**Returns:** `UriKind`


---

#### safeDecode()

Decode raw bytes into UTF-8, using heuristics and fallback encodings when necessary.

The function prefers an explicit `encoding`, falls back to the cached guess, probes
an encoding detector, and finally tries a small curated list before returning a
mojibake-cleaned string.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `byteData` | `ByteArray` | Yes | The byte data |
| `encoding` | `String?` | No | The encoding |

**Returns:** `String`


---

#### calculateTextConfidence()

Estimate how trustworthy a decoded string is on a 0.0–1.0 scale.

Scores close to 1.0 indicate mostly printable characters, whereas lower scores
point to mojibake, control characters, or suspicious character mixes.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `String` | Yes | The text |

**Returns:** `Double`


---

#### createStringBufferPool()

Create a pre-configured string buffer pool for batch processing.

**Returns:**

A pool configured for text accumulation with reasonable defaults.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `poolSize` | `Long` | Yes | Maximum number of buffers to keep in the pool |
| `bufferCapacity` | `Long` | Yes | Initial capacity for each buffer in bytes |

**Returns:** `StringBufferPool`


---

#### createByteBufferPool()

Create a pre-configured byte buffer pool for batch processing.

**Returns:**

A pool configured for binary data handling with reasonable defaults.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `poolSize` | `Long` | Yes | Maximum number of buffers to keep in the pool |
| `bufferCapacity` | `Long` | Yes | Initial capacity for each buffer in bytes |

**Returns:** `ByteBufferPool`


---

#### openapiJson()

Generate OpenAPI JSON schema.

Returns the complete OpenAPI 3.1 specification as a JSON string.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Returns:** `String`


---

#### serveDefault()

Start the API server with default host and port.

Defaults: host = "127.0.0.1", port = 8000

Uses config file discovery (searches current/parent directories for kreuzberg.toml/yaml/json).
Validates plugins at startup to help diagnose configuration issues.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Returns:** `Unit`

**Errors:** Throws `Error`.


---

#### chunkText()

Split text into chunks with optional page boundary tracking.

This is the primary API function for chunking text. It supports both plain text
and Markdown with configurable chunk size, overlap, and page boundary mapping.

**Returns:**

A ChunkingResult containing all chunks and their metadata.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `String` | Yes | The text to split into chunks |
| `config` | `ChunkingConfig` | Yes | Chunking configuration (max size, overlap, type) |
| `pageBoundaries` | `List<PageBoundary>?` | No | Optional page boundary markers for mapping chunks to pages |

**Returns:** `ChunkingResult`

**Errors:** Throws `Error`.


---

#### chunkTextWithHeadingSource()

Chunk text with an optional separate markdown source for heading context resolution.

When `heading_source` is provided, it is used instead of `text` for building the
heading map. This is needed when `text` is plain text (no markdown headings) but
the original document had headings that were stripped during rendering.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `String` | Yes | The text |
| `config` | `ChunkingConfig` | Yes | The configuration options |
| `pageBoundaries` | `List<PageBoundary>?` | No | The page boundaries |
| `headingSource` | `String?` | No | The heading source |

**Returns:** `ChunkingResult`

**Errors:** Throws `Error`.


---

#### chunkTextsBatch()

Batch process multiple texts with the same configuration.

This convenience function applies the same chunking configuration to multiple
texts in sequence.

**Returns:**

A vector of ChunkingResult objects, one per input text.

**Errors:**

Returns an error if chunking any individual text fails.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `texts` | `List<String>` | Yes | Slice of text strings to chunk |
| `config` | `ChunkingConfig` | Yes | Chunking configuration to apply to all texts |

**Returns:** `List<ChunkingResult>`

**Errors:** Throws `Error`.


---

#### chunkSemantic()

Split text into semantically coherent chunks.

Splits text into fine-grained segments, detects structural (and optionally
embedding-based) topic boundaries, then merges segments into chunks that
respect those boundaries and the configured size budget.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `String` | Yes | The text |
| `config` | `ChunkingConfig` | Yes | The configuration options |
| `pageBoundaries` | `List<PageBoundary>?` | No | The page boundaries |

**Returns:** `ChunkingResult`

**Errors:** Throws `Error`.


---

#### normalize()

L2-normalize a vector.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `v` | `List<Float>` | Yes | The v |

**Returns:** `List<Float>`


---

#### getPreset()

Get a preset by name.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `name` | `String` | Yes | The name |

**Returns:** `String?`


---

#### listPresets()

List all available preset names.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Returns:** `List<String>`


---

#### warmModel()

Eagerly download and cache an embedding model without returning the handle.

This triggers the same download and initialization as `get_or_init_engine`
but discards the result, making it suitable for cache-warming scenarios
where the caller doesn't need to use the model immediately.

**Note**: This function downloads AND initializes the ONNX model, which
requires ONNX Runtime and uses significant memory. For download-only
scenarios (e.g., init containers), use `download_model` instead.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `modelType` | `EmbeddingModelType` | Yes | The embedding model type |
| `cacheDir` | `Path?` | No | The cache dir |

**Returns:** `Unit`

**Errors:** Throws `Error`.


---

#### downloadModel()

Download an embedding model's files without initializing ONNX Runtime.

Downloads the model files (ONNX model, tokenizer, config) from HuggingFace
to the cache directory. Subsequent calls to `warm_model` or
`get_or_init_engine` will find the files cached and skip the download step.

This is ideal for init containers or CI environments where you want to
pre-populate the cache without loading models into memory.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `modelType` | `EmbeddingModelType` | Yes | The embedding model type |
| `cacheDir` | `Path?` | No | The cache dir |

**Returns:** `Unit`

**Errors:** Throws `Error`.


---

#### calculateOptimalDpi()

Calculate optimal DPI with min/max constraints

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `pageWidth` | `Double` | Yes | The page width |
| `pageHeight` | `Double` | Yes | The page height |
| `targetDpi` | `Int` | Yes | The target dpi |
| `maxDimension` | `Int` | Yes | The max dimension |
| `minDpi` | `Int` | Yes | The min dpi |
| `maxDpi` | `Int` | Yes | The max dpi |

**Returns:** `Int`


---

#### detectLanguages()

Detect languages in text using whatlang.

Returns a list of detected language codes (ISO 639-3 format).
Returns `null` if no languages could be detected with sufficient confidence.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `String` | Yes | The text to analyze for language detection |
| `config` | `LanguageDetectionConfig` | Yes | Optional configuration for language detection |

**Returns:** `List<String>?`

**Errors:** Throws `Error`.


---

#### extractKeywords()

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

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `String` | Yes | The text to extract keywords from |
| `config` | `KeywordConfig` | Yes | Keyword extraction configuration |

**Returns:** `List<Keyword>`

**Errors:** Throws `Error`.


---

#### computeHash()

Compute a blake3 hash string from input data.

Returns a 32-character hex string (128 bits of blake3 output).

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `data` | `String` | Yes | The data |

**Returns:** `String`


---

#### renderPdfPageToPng()

Render a single PDF page to a PNG-encoded byte buffer.

**Errors:**

Returns an error if the PDF is invalid, the page index is out of bounds,
or if the page fails to render.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `pdfBytes` | `ByteArray` | Yes | The pdf bytes |
| `pageIndex` | `Long` | Yes | The page index |
| `dpi` | `Int?` | No | The dpi |
| `password` | `String?` | No | The password |

**Returns:** `ByteArray`

**Errors:** Throws `Error`.


---

#### extractTextFromPdf()

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `pdfBytes` | `ByteArray` | Yes | The pdf bytes |

**Returns:** `String`

**Errors:** Throws `Error`.


---

#### serializeToToon()

Serialize an `ExtractionResult` to TOON (Token-Oriented Object Notation).

TOON is a token-efficient alternative to JSON for LLM prompts.
Losslessly convertible to/from JSON but uses fewer tokens.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `result` | `ExtractionResult` | Yes | The extraction result |

**Returns:** `String`

**Errors:** Throws `Error`.


---

#### serializeToJson()

Serialize an `ExtractionResult` to pretty-printed JSON.

**Signature:**

```kotlin
// Phase 1: kotlin backend signature generation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `result` | `ExtractionResult` | Yes | The extraction result |

**Returns:** `String`

**Errors:** Throws `Error`.


---

### Types

#### AccelerationConfig

Hardware acceleration configuration for ONNX Runtime models.

Controls which execution provider (CPU, CoreML, CUDA, TensorRT) is used
for inference in layout detection and embedding generation.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `provider` | `ExecutionProviderType` | `ExecutionProviderType.Auto` | Execution provider to use for ONNX inference. |
| `deviceId` | `Int` | — | GPU device ID (for CUDA/TensorRT). Ignored for CPU/CoreML/Auto. |


---

#### AnchorProperties

Properties for anchored drawings.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `behindDoc` | `Boolean` | — | Behind doc |
| `layoutInCell` | `Boolean` | — | Layout in cell |
| `relativeHeight` | `Long?` | `null` | Relative height |
| `positionH` | `String?` | `null` | Position h |
| `positionV` | `String?` | `null` | Position v |
| `wrapType` | `String` | — | Wrap type |


---

#### ApiDoc

OpenAPI documentation structure.

Defines all endpoints, request/response schemas, and examples
for the Kreuzberg document extraction API.


---

#### ApiState

API server state.

Holds the default extraction configuration loaded from config file
(via discovery or explicit path). Per-request configs override these defaults.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `defaultConfig` | `ExtractionConfig` | — | Default extraction configuration |
| `extractionService` | `String` | — | Tower service for extraction requests. Wrapped in `Arc<Mutex>` because `BoxCloneService` is `Send` but not `Sync`, while `ApiState` must be `Clone + Sync` for Axum's state requirement. The lock is held only long enough to clone the service. |


---

#### ArchiveEntry

A single file extracted from an archive.

When archives (ZIP, TAR, 7Z, GZIP) are extracted with recursive extraction
enabled, each processable file produces its own full `ExtractionResult`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `path` | `String` | — | Archive-relative file path (e.g. "folder/document.pdf"). |
| `mimeType` | `String` | — | Detected MIME type of the file. |
| `result` | `ExtractionResult` | — | Full extraction result for this file. |


---

#### ArchiveMetadata

Archive (ZIP/TAR/7Z) metadata.

Extracted from compressed archive files containing file lists and size information.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `format` | `String` | — | Archive format ("ZIP", "TAR", "7Z", etc.) |
| `fileCount` | `Long` | — | Total number of files in the archive |
| `fileList` | `List<String>` | `[]` | List of file paths within the archive |
| `totalSize` | `Long` | — | Total uncompressed size in bytes |
| `compressedSize` | `Long?` | `null` | Compressed size in bytes (if available) |


---

#### BBox

Bounding box in original image coordinates (x1, y1) top-left, (x2, y2) bottom-right.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `x1` | `Float` | — | X1 |
| `y1` | `Float` | — | Y1 |
| `x2` | `Float` | — | X2 |
| `y2` | `Float` | — | Y2 |


---

#### BatchExtractFilesParams

Request parameters for batch file extraction.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `paths` | `List<String>` | — | Paths to files to extract |
| `config` | `Any?` | `null` | Extraction configuration (JSON object) |
| `pdfPassword` | `String?` | `null` | Password for encrypted PDFs |
| `fileConfigs` | `List<Any?>?` | `null` | Per-file extraction configuration overrides (parallel array to paths). Each entry is either null (use default) or a FileExtractionConfig JSON object. |
| `responseFormat` | `String?` | `null` | Wire format for the response: "json" (default) or "toon" |


---

#### BibtexMetadata

BibTeX bibliography metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `entryCount` | `Long` | — | Number of entries in the bibliography. |
| `citationKeys` | `List<String>` | `[]` | Citation keys |
| `authors` | `List<String>` | `[]` | Authors |
| `yearRange` | `YearRange?` | `null` | Year range (year range) |
| `entryTypes` | `Map<String, Long>?` | `{}` | Entry types |


---

#### ByteBufferPool

Convenience type alias for a pooled Vec<u8>.


---

#### CacheClearResponse

Cache clear response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `directory` | `String` | — | Cache directory path |
| `removedFiles` | `Long` | — | Number of files removed |
| `freedMb` | `Double` | — | Space freed in MB |


---

#### CacheStatsResponse

Cache statistics response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `directory` | `String` | — | Cache directory path |
| `totalFiles` | `Long` | — | Total number of cache files |
| `totalSizeMb` | `Double` | — | Total cache size in MB |
| `availableSpaceMb` | `Double` | — | Available disk space in MB |
| `oldestFileAgeDays` | `Double` | — | Age of oldest file in days |
| `newestFileAgeDays` | `Double` | — | Age of newest file in days |


---

#### CacheWarmParams

Request parameters for cache warm (model download).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `allEmbeddings` | `Boolean` | — | Download all embedding model presets |
| `embeddingModel` | `String?` | `null` | Specific embedding preset name to download (e.g. "balanced", "speed", "quality") |


---

#### Chunk

A text chunk with optional embedding and metadata.

Chunks are created when chunking is enabled in `ExtractionConfig`. Each chunk
contains the text content, optional embedding vector (if embedding generation
is configured), and metadata about its position in the document.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String` | — | The text content of this chunk. |
| `chunkType` | `ChunkType` | — | Semantic structural classification of this chunk. Assigned by the heuristic classifier based on content patterns and heading context. Defaults to `ChunkType.Unknown` when no rule matches. |
| `embedding` | `List<Float>?` | `null` | Optional embedding vector for this chunk. Only populated when `EmbeddingConfig` is provided in chunking configuration. The dimensionality depends on the chosen embedding model. |
| `metadata` | `ChunkMetadata` | — | Metadata about this chunk's position and properties. |


---

#### ChunkMetadata

Metadata about a chunk's position in the original document.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `byteStart` | `Long` | — | Byte offset where this chunk starts in the original text (UTF-8 valid boundary). |
| `byteEnd` | `Long` | — | Byte offset where this chunk ends in the original text (UTF-8 valid boundary). |
| `tokenCount` | `Long?` | `null` | Number of tokens in this chunk (if available). This is calculated by the embedding model's tokenizer if embeddings are enabled. |
| `chunkIndex` | `Long` | — | Zero-based index of this chunk in the document. |
| `totalChunks` | `Long` | — | Total number of chunks in the document. |
| `firstPage` | `Long?` | `null` | First page number this chunk spans (1-indexed). Only populated when page tracking is enabled in extraction configuration. |
| `lastPage` | `Long?` | `null` | Last page number this chunk spans (1-indexed, equal to first_page for single-page chunks). Only populated when page tracking is enabled in extraction configuration. |
| `headingContext` | `HeadingContext?` | `null` | Heading context when using Markdown chunker. Contains the heading hierarchy this chunk falls under. Only populated when `ChunkerType.Markdown` is used. |


---

#### ChunkRequest

Chunk request with text and configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text` | `String` | — | Text to chunk (must not be empty) |
| `config` | `String?` | `null` | Optional chunking configuration |
| `chunkerType` | `String` | — | Chunker type (text, markdown, yaml, or semantic) |


---

#### ChunkResponse

Chunk response with chunks and metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `chunks` | `List<String>` | — | List of chunks |
| `chunkCount` | `Long` | — | Total number of chunks |
| `config` | `String` | — | Configuration used for chunking |
| `inputSizeBytes` | `Long` | — | Input text size in bytes |
| `chunkerType` | `String` | — | Chunker type used for chunking |


---

#### ChunkTextParams

Request parameters for text chunking.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text` | `String` | — | Text content to split into chunks |
| `maxCharacters` | `Long?` | `null` | Maximum characters per chunk (default: 2000) |
| `overlap` | `Long?` | `null` | Number of overlapping characters between chunks (default: 100) |
| `chunkerType` | `String?` | `null` | Chunker type: "text", "markdown", "yaml", or "semantic" (default: "text") |
| `topicThreshold` | `Float?` | `null` | Topic threshold for semantic chunking (0.0-1.0, default: 0.75) |


---

#### ChunkingConfig

Chunking configuration.

Configures text chunking for document content, including chunk size,
overlap, trimming behavior, and optional embeddings.

Use `..the default constructor` when constructing to allow for future field additions:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `maxCharacters` | `Long` | `1000` | Maximum size per chunk (in units determined by `sizing`). When `sizing` is `Characters` (default), this is the max character count. When using token-based sizing, this is the max token count. Default: 1000 |
| `overlap` | `Long` | `200` | Overlap between chunks (in units determined by `sizing`). Default: 200 |
| `trim` | `Boolean` | `true` | Whether to trim whitespace from chunk boundaries. Default: true |
| `chunkerType` | `ChunkerType` | `ChunkerType.Text` | Type of chunker to use (Text or Markdown). Default: Text |
| `embedding` | `EmbeddingConfig?` | `null` | Optional embedding configuration for chunk embeddings. |
| `preset` | `String?` | `null` | Use a preset configuration (overrides individual settings if provided). |
| `sizing` | `ChunkSizing` | `ChunkSizing.Characters` | How to measure chunk size. Default: `Characters` (Unicode character count). Enable `chunking-tiktoken` or `chunking-tokenizers` features for token-based sizing. |
| `prependHeadingContext` | `Boolean` | `false` | When `true` and `chunker_type` is `Markdown`, prepend the heading hierarchy path (e.g. `"# Title > ## Section\n\n"`) to each chunk's content string. This is useful for RAG pipelines where each chunk needs self-contained context about its position in the document structure. Default: `false` |
| `topicThreshold` | `Float?` | `null` | Optional cosine similarity threshold for semantic topic boundary detection. Only used when `chunker_type` is `Semantic` and an `EmbeddingConfig` is provided. You almost never need to set this. When omitted, defaults to `0.75` which works well for most documents. Lower values detect more topic boundaries (more, smaller chunks); higher values detect fewer. Range: `0.0..=1.0`. |

##### Methods

###### default()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```


---

#### ChunkingResult

Result of a text chunking operation.

Contains the generated chunks and metadata about the chunking.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `chunks` | `List<Chunk>` | — | List of text chunks |
| `chunkCount` | `Long` | — | Total number of chunks generated |


---

#### CitationMetadata

Citation file metadata (RIS, PubMed, EndNote).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `citationCount` | `Long` | — | Number of citations |
| `format` | `String?` | `null` | Format |
| `authors` | `List<String>` | `[]` | Authors |
| `yearRange` | `YearRange?` | `null` | Year range (year range) |
| `dois` | `List<String>` | `[]` | Dois |
| `keywords` | `List<String>` | `[]` | Keywords |


---

#### CommonPdfMetadata

Common metadata fields extracted from a PDF.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `title` | `String?` | `null` | Title |
| `subject` | `String?` | `null` | Subject |
| `authors` | `List<String>?` | `null` | Authors |
| `keywords` | `List<String>?` | `null` | Keywords |
| `createdAt` | `String?` | `null` | Created at |
| `modifiedAt` | `String?` | `null` | Modified at |
| `createdBy` | `String?` | `null` | Created by |


---

#### ContentFilterConfig

Cross-extractor content filtering configuration.

Controls whether "furniture" content (headers, footers, page numbers,
watermarks, repeating text) is included in or stripped from extraction
results. Applies across all extractors (PDF, DOCX, RTF, ODT, HTML, etc.)
with format-specific implementation.

When `null` on `ExtractionConfig`, each extractor uses its current
default behavior unchanged.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `includeHeaders` | `Boolean` | `false` | Include running headers in extraction output. - PDF: Disables top-margin furniture stripping and prevents the layout model from treating `PageHeader`-classified regions as furniture. - DOCX: Includes document headers in text output. - RTF/ODT: Headers already included; this is a no-op when true. - HTML/EPUB: Keeps `<header>` element content. Default: `false` (headers are stripped or excluded). |
| `includeFooters` | `Boolean` | `false` | Include running footers in extraction output. - PDF: Disables bottom-margin furniture stripping and prevents the layout model from treating `PageFooter`-classified regions as furniture. - DOCX: Includes document footers in text output. - RTF/ODT: Footers already included; this is a no-op when true. - HTML/EPUB: Keeps `<footer>` element content. Default: `false` (footers are stripped or excluded). |
| `stripRepeatingText` | `Boolean` | `true` | Enable the heuristic cross-page repeating text detector. When `true` (default), text that repeats verbatim across a supermajority of pages is classified as furniture and stripped.  Disable this if brand names or repeated headings are being incorrectly removed by the heuristic. Note: when a layout-detection model is active, the model may independently classify page-header / page-footer regions as furniture on a per-page basis. To preserve those regions, set `include_headers = true` and/or `include_footers = true` in addition to disabling this flag. Primarily affects PDF extraction. Default: `true`. |
| `includeWatermarks` | `Boolean` | `false` | Include watermark text in extraction output. - PDF: Keeps watermark artifacts and arXiv identifiers. - Other formats: No effect currently. Default: `false` (watermarks are stripped). |

##### Methods

###### default()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```


---

#### ContributorRole

JATS contributor with role.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String` | — | The name |
| `role` | `String?` | `null` | Role |


---

#### CsvMetadata

CSV/TSV file metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `rowCount` | `Long` | — | Number of rows |
| `columnCount` | `Long` | — | Number of columns |
| `delimiter` | `String?` | `null` | Delimiter |
| `hasHeader` | `Boolean` | — | Whether header |
| `columnTypes` | `List<String>?` | `[]` | Column types |


---

#### CustomProperties

Custom properties from docProps/custom.xml

Maps property names to their values. Values are converted to JSON types
based on the VT (Variant Type) specified in the XML.


---

#### DbfFieldInfo

dBASE field information.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String` | — | The name |
| `fieldType` | `String` | — | Field type |


---

#### DbfMetadata

dBASE (DBF) file metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `recordCount` | `Long` | — | Number of records |
| `fieldCount` | `Long` | — | Number of fields |
| `fields` | `List<DbfFieldInfo>` | `[]` | Fields |


---

#### DetectMimeTypeParams

Request parameters for MIME type detection.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `path` | `String` | — | Path to the file |
| `useContent` | `Boolean` | — | Use content-based detection (default: true) |


---

#### DetectResponse

MIME type detection response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `mimeType` | `String` | — | Detected MIME type |
| `filename` | `String?` | `null` | Original filename (if provided) |


---

#### DetectedBoundary

A detected structural boundary in the text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `byteOffset` | `Long` | — | Byte offset of the start of the line in the original text. |
| `isHeader` | `Boolean` | — | Whether this boundary looks like a header/section title. |


---

#### DetectionResult

Page-level detection result containing all detections and page metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `pageWidth` | `Int` | — | Page width |
| `pageHeight` | `Int` | — | Page height |
| `detections` | `List<LayoutDetection>` | — | Detections |


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
| `plainText` | `String` | — | Plain text representation for backwards compatibility |
| `blocks` | `List<FormattedBlock>` | — | Structured block-level content |
| `metadata` | `Metadata` | — | Metadata from YAML frontmatter |
| `tables` | `List<String>` | — | Extracted tables as structured data |
| `images` | `List<DjotImage>` | — | Extracted images with metadata |
| `links` | `List<DjotLink>` | — | Extracted links with URLs |
| `footnotes` | `List<Footnote>` | — | Footnote definitions |
| `attributes` | `List<String>` | — | Attributes mapped by element identifier (if present) |


---

#### DjotImage

Image element in Djot.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `src` | `String` | — | Image source URL or path |
| `alt` | `String` | — | Alternative text |
| `title` | `String?` | `null` | Optional title |
| `attributes` | `String?` | `null` | Element attributes |


---

#### DjotLink

Link element in Djot.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | `String` | — | Link URL |
| `text` | `String` | — | Link text content |
| `title` | `String?` | `null` | Optional title |
| `attributes` | `String?` | `null` | Element attributes |


---

#### DoclingCompatResponse

OpenWebUI "Docling" engine response format.

Returned by `POST /v1/convert/file` for docling-serve compatibility.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `document` | `String` | — | Converted document content |
| `status` | `String` | — | Processing status |


---

#### DocumentExtractor

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

###### extractBytes()

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

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### extractFile()

Extract content from a file.

Default implementation reads the file and calls `extract_bytes`.
Override for custom file handling, streaming, or memory optimizations.

**Returns:**

An `InternalDocument` containing the extracted elements, metadata, and tables.

**Errors:**

Same as `extract_bytes`, plus file I/O errors.

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### supportedMimeTypes()

Get the list of MIME types supported by this extractor.

Can include exact MIME types and prefix patterns:
- Exact: `"application/pdf"`, `"text/plain"`
- Prefix: `"image/*"` (matches any image type)

**Returns:**

A slice of MIME type strings.

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### priority()

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

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### canHandle()

Optional: Check if this extractor can handle a specific file.

Allows for more sophisticated detection beyond MIME types.
Defaults to `true` (rely on MIME type matching).

**Returns:**

`true` if the extractor can handle this file, `false` otherwise.

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### asSyncExtractor()

Attempt to get a reference to this extractor as a SyncExtractor.

Returns None if the extractor doesn't support synchronous extraction.
This is used for WASM and other sync-only environments.

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```


---

#### DocumentNode

A single node in the document tree.

Each node has deterministic `id`, typed `content`, optional `parent`/`children`
for tree structure, and metadata like page number, bounding box, and content layer.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String` | — | Deterministic identifier (hash of content + position). |
| `content` | `NodeContent` | — | Node content — tagged enum, type-specific data only. |
| `parent` | `Int?` | `null` | Parent node index (`null` = root-level node). |
| `children` | `List<Int>` | — | Child node indices in reading order. |
| `contentLayer` | `ContentLayer` | — | Content layer classification. |
| `page` | `Int?` | `null` | Page number where this node starts (1-indexed). |
| `pageEnd` | `Int?` | `null` | Page number where this node ends (for multi-page tables/sections). |
| `bbox` | `String?` | `null` | Bounding box in document coordinates. |
| `annotations` | `List<TextAnnotation>` | — | Inline annotations (formatting, links) on this node's text content. Only meaningful for text-carrying nodes; empty for containers. |
| `attributes` | `Map<String, String>?` | `null` | Format-specific key-value attributes. Extensible bag for data that doesn't warrant a typed field: CSS classes, LaTeX environment names, Excel cell formulas, slide layout names, etc. |


---

#### DocumentRelationship

A resolved relationship between two nodes in the document tree.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `source` | `Int` | — | Source node index (the referencing node). |
| `target` | `Int` | — | Target node index (the referenced node). |
| `kind` | `RelationshipKind` | — | Semantic kind of the relationship. |


---

#### DocumentStructure

Top-level structured document representation.

A flat array of nodes with index-based parent/child references forming a tree.
Root-level nodes have `parent: None`. Use `body_roots()` and `furniture_roots()`
to iterate over top-level content by layer.

# Validation

Call `validate()` after construction to verify all node indices are in bounds
and parent-child relationships are bidirectionally consistent.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `nodes` | `List<DocumentNode>` | `[]` | All nodes in document/reading order. |
| `sourceFormat` | `String?` | `null` | Origin format identifier (e.g. "docx", "pptx", "html", "pdf"). Allows renderers to apply format-aware heuristics when converting the document tree to output formats. |
| `relationships` | `List<DocumentRelationship>` | `[]` | Resolved relationships between nodes (footnote refs, citations, anchor links, etc.). Populated during derivation from the internal document representation. Empty when no relationships are detected. |

##### Methods

###### default()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```


---

#### DocxMetadata

Word document metadata.

Extracted from DOCX files using shared Office Open XML metadata extraction.
Integrates with `office_metadata` module for core/app/custom properties.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `coreProperties` | `String?` | `null` | Core properties from docProps/core.xml (Dublin Core metadata) Contains title, creator, subject, keywords, dates, etc. Shared format across DOCX/PPTX/XLSX documents. |
| `appProperties` | `String?` | `null` | Application properties from docProps/app.xml (Word-specific statistics) Contains word count, page count, paragraph count, editing time, etc. DOCX-specific variant of Office application properties. |
| `customProperties` | `Map<String, Any>?` | `{}` | Custom properties from docProps/custom.xml (user-defined properties) Contains key-value pairs defined by users or applications. Values can be strings, numbers, booleans, or dates. |


---

#### Drawing

A drawing object extracted from `<w:drawing>`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `drawingType` | `String` | — | Drawing type |
| `extent` | `String?` | `null` | Extent |
| `docProperties` | `String?` | `null` | Doc properties |
| `imageRef` | `String?` | `null` | Image ref |


---

#### Element

Semantic element extracted from document.

Represents a logical unit of content with semantic classification,
unique identifier, and metadata for tracking origin and position.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `elementId` | `String` | — | Unique element identifier |
| `elementType` | `ElementType` | — | Semantic type of this element |
| `text` | `String` | — | Text content of the element |
| `metadata` | `ElementMetadata` | — | Metadata about the element |


---

#### ElementMetadata

Metadata for a semantic element.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `pageNumber` | `Long?` | `null` | Page number (1-indexed) |
| `filename` | `String?` | `null` | Source filename or document name |
| `coordinates` | `String?` | `null` | Bounding box coordinates if available |
| `elementIndex` | `Long?` | `null` | Position index in the element sequence |
| `additional` | `Map<String, String>` | — | Additional custom metadata |


---

#### EmailAttachment

Email attachment representation.

Contains metadata and optionally the content of an email attachment.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String?` | `null` | Attachment name (from Content-Disposition header) |
| `filename` | `String?` | `null` | Filename of the attachment |
| `mimeType` | `String?` | `null` | MIME type of the attachment |
| `size` | `Long?` | `null` | Size in bytes |
| `isImage` | `Boolean` | — | Whether this attachment is an image |
| `data` | `ByteArray?` | `null` | Attachment data (if extracted). Uses `bytes.Bytes` for cheap cloning of large buffers. |


---

#### EmailConfig

Configuration for email extraction.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `msgFallbackCodepage` | `Int?` | `null` | Windows codepage number to use when an MSG file contains no codepage property. Defaults to `null`, which falls back to windows-1252. If an unrecognized or invalid codepage number is supplied (including 0), the behavior silently falls back to windows-1252 — the same as when the MSG file itself contains an unrecognized codepage. No error or warning is emitted. Users should verify output when supplying unusual values. Common values: - 1250: Central European (Polish, Czech, Hungarian, etc.) - 1251: Cyrillic (Russian, Ukrainian, Bulgarian, etc.) - 1252: Western European (default) - 1253: Greek - 1254: Turkish - 1255: Hebrew - 1256: Arabic - 932:  Japanese (Shift-JIS) - 936:  Simplified Chinese (GBK) |


---

#### EmailExtractionResult

Email extraction result.

Complete representation of an extracted email message (.eml or .msg)
including headers, body content, and attachments.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `subject` | `String?` | `null` | Email subject line |
| `fromEmail` | `String?` | `null` | Sender email address |
| `toEmails` | `List<String>` | — | Primary recipient email addresses |
| `ccEmails` | `List<String>` | — | CC recipient email addresses |
| `bccEmails` | `List<String>` | — | BCC recipient email addresses |
| `date` | `String?` | `null` | Email date/timestamp |
| `messageId` | `String?` | `null` | Message-ID header value |
| `plainText` | `String?` | `null` | Plain text version of the email body |
| `htmlContent` | `String?` | `null` | HTML version of the email body |
| `cleanedText` | `String` | — | Cleaned/processed text content |
| `attachments` | `List<EmailAttachment>` | — | List of email attachments |
| `metadata` | `Map<String, String>` | — | Additional email headers and metadata |


---

#### EmailMetadata

Email metadata extracted from .eml and .msg files.

Includes sender/recipient information, message ID, and attachment list.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `fromEmail` | `String?` | `null` | Sender's email address |
| `fromName` | `String?` | `null` | Sender's display name |
| `toEmails` | `List<String>` | `[]` | Primary recipients |
| `ccEmails` | `List<String>` | `[]` | CC recipients |
| `bccEmails` | `List<String>` | `[]` | BCC recipients |
| `messageId` | `String?` | `null` | Message-ID header value |
| `attachments` | `List<String>` | `[]` | List of attachment filenames |


---

#### EmbedRequest

Embedding request for generating embeddings from text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `texts` | `List<String>` | — | Text strings to generate embeddings for (at least one non-empty string required) |
| `config` | `EmbeddingConfig?` | `null` | Optional embedding configuration (model, batch size, etc.) |


---

#### EmbedResponse

Embedding response containing generated embeddings.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `embeddings` | `List<List<Float>>` | — | Generated embeddings (one per input text) |
| `model` | `String` | — | Model used for embedding generation |
| `dimensions` | `Long` | — | Dimensionality of the embeddings |
| `count` | `Long` | — | Number of embeddings generated |


---

#### EmbedTextParams

Request parameters for embedding generation.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `texts` | `List<String>` | — | List of text strings to generate embeddings for |
| `preset` | `String?` | `null` | Embedding preset name (default: "balanced"). Available: "speed", "balanced", "quality" |
| `model` | `String?` | `null` | LLM model for provider-hosted embeddings (e.g., "openai/text-embedding-3-small"). When set, overrides preset and uses liter-llm for embedding generation. |
| `apiKey` | `String?` | `null` | API key for the LLM provider (optional, falls back to env). |
| `embeddingPlugin` | `String?` | `null` | Name of a pre-registered in-process embedding plugin backend. When set, overrides both preset and model and dispatches to the registered callback. Requires a prior call to `kreuzberg.plugins.register_embedding_backend`. |


---

#### EmbeddedFile

Embedded file descriptor extracted from the PDF name tree.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String` | — | The filename as stored in the PDF name tree. |
| `data` | `ByteArray` | — | Raw file bytes from the embedded stream. |
| `mimeType` | `String?` | `null` | MIME type if specified in the filespec, otherwise `null`. |


---

#### EmbeddingBackend

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

###### dimensions()

Embedding vector dimension. Must be `> 0` and must match the length of
every vector returned by `embed`.

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### embed()

Embed a batch of texts, returning one vector per input in order.

**Errors:**

Implementations should return `crate.KreuzbergError.Plugin` for
backend-specific failures. The dispatcher layers its own validation
(length, per-vector dimension) on top.

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```


---

#### EmbeddingConfig

Embedding configuration for text chunks.

Configures embedding generation using ONNX models via the vendored embedding engine.
Requires the `embeddings` feature to be enabled.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `EmbeddingModelType` | `EmbeddingModelType.Preset` | The embedding model to use (defaults to "balanced" preset if not specified) |
| `normalize` | `Boolean` | `true` | Whether to normalize embedding vectors (recommended for cosine similarity) |
| `batchSize` | `Long` | `32` | Batch size for embedding generation |
| `showDownloadProgress` | `Boolean` | `false` | Show model download progress |
| `cacheDir` | `Path?` | `null` | Custom cache directory for model files Defaults to `~/.cache/kreuzberg/embeddings/` if not specified. Allows full customization of model download location. |
| `acceleration` | `AccelerationConfig?` | `null` | Hardware acceleration for the embedding ONNX model. When set, controls which execution provider (CPU, CUDA, CoreML, TensorRT) is used for inference. Defaults to `null` (auto-select per platform). |
| `maxEmbedDurationSecs` | `Long?` | `null` | Maximum wall-clock duration (in seconds) for a single `embed()` call when using `EmbeddingModelType.Plugin`. Applies only to the in-process plugin path — protects against hung host-language backends (e.g. a Python callback deadlocked on the GIL, a model stuck on CUDA OOM retries, etc.). On timeout, the dispatcher returns `crate.KreuzbergError.Plugin` instead of blocking forever. `null` disables the timeout. The default (60 seconds) is conservative for common in-process inference; increase for large batches on slow hardware. |

##### Methods

###### default()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```


---

#### EpubMetadata

EPUB metadata (Dublin Core extensions).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `coverage` | `String?` | `null` | Coverage |
| `dcFormat` | `String?` | `null` | Dc format |
| `relation` | `String?` | `null` | Relation |
| `source` | `String?` | `null` | Source |
| `dcType` | `String?` | `null` | Dc type |
| `coverImage` | `String?` | `null` | Cover image |


---

#### ErrorMetadata

Error metadata (for batch operations).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `errorType` | `String` | — | Error type |
| `message` | `String` | — | Message |


---

#### ExcelMetadata

Excel/spreadsheet metadata.

Contains information about sheets in Excel, OpenDocument Calc, and other
spreadsheet formats (.xlsx, .xls, .ods, etc.).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `sheetCount` | `Long` | — | Total number of sheets in the workbook |
| `sheetNames` | `List<String>` | `[]` | Names of all sheets in order |


---

#### ExcelSheet

Single Excel worksheet.

Represents one sheet from an Excel workbook with its content
converted to Markdown format and dimensional statistics.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String` | — | Sheet name as it appears in Excel |
| `markdown` | `String` | — | Sheet content converted to Markdown tables |
| `rowCount` | `Long` | — | Number of rows |
| `colCount` | `Long` | — | Number of columns |
| `cellCount` | `Long` | — | Total number of non-empty cells |
| `tableCells` | `List<List<String>>?` | `null` | Pre-extracted table cells (2D vector of cell values) Populated during markdown generation to avoid re-parsing markdown. None for empty sheets. |


---

#### ExcelWorkbook

Excel workbook representation.

Contains all sheets from an Excel file (.xlsx, .xls, etc.) with
extracted content and metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `sheets` | `List<ExcelSheet>` | — | All sheets in the workbook |
| `metadata` | `Map<String, String>` | — | Workbook-level metadata (author, creation date, etc.) |


---

#### ExtractBytesParams

Request parameters for bytes extraction.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `data` | `String` | — | Base64-encoded file content |
| `mimeType` | `String?` | `null` | Optional MIME type hint (auto-detected if not provided) |
| `config` | `Any?` | `null` | Extraction configuration (JSON object) |
| `pdfPassword` | `String?` | `null` | Password for encrypted PDFs |
| `responseFormat` | `String?` | `null` | Wire format for the response: "json" (default) or "toon" |


---

#### ExtractFileParams

Request parameters for file extraction.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `path` | `String` | — | Path to the file to extract |
| `mimeType` | `String?` | `null` | Optional MIME type hint (auto-detected if not provided) |
| `config` | `Any?` | `null` | Extraction configuration (JSON object) |
| `pdfPassword` | `String?` | `null` | Password for encrypted PDFs |
| `responseFormat` | `String?` | `null` | Wire format for the response: "json" (default) or "toon" |


---

#### ExtractResponse

Extraction response (list of results).


---

#### ExtractStructuredParams

Request parameters for LLM-based structured extraction.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `path` | `String` | — | File path to extract from |
| `schema` | `Any` | — | JSON schema for structured output |
| `model` | `String` | — | LLM model (e.g., "openai/gpt-4o") |
| `schemaName` | `String` | — | Schema name (default: "extraction") |
| `schemaDescription` | `String?` | `null` | Schema description for the LLM |
| `prompt` | `String?` | `null` | Custom Jinja2 prompt template |
| `apiKey` | `String?` | `null` | API key (optional, falls back to env) |
| `strict` | `Boolean` | — | Enable strict mode |


---

#### ExtractedImage

Extracted image from a document.

Contains raw image data, metadata, and optional nested OCR results.
Raw bytes allow cross-language compatibility - users can convert to
PIL.Image (Python), Sharp (Node.js), or other formats as needed.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `data` | `ByteArray` | — | Raw image data (PNG, JPEG, WebP, etc. bytes). Uses `bytes.Bytes` for cheap cloning of large buffers. |
| `format` | `String` | — | Image format (e.g., "jpeg", "png", "webp") Uses Cow<'static, str> to avoid allocation for static literals. |
| `imageIndex` | `Long` | — | Zero-indexed position of this image in the document/page |
| `pageNumber` | `Long?` | `null` | Page/slide number where image was found (1-indexed) |
| `width` | `Int?` | `null` | Image width in pixels |
| `height` | `Int?` | `null` | Image height in pixels |
| `colorspace` | `String?` | `null` | Colorspace information (e.g., "RGB", "CMYK", "Gray") |
| `bitsPerComponent` | `Int?` | `null` | Bits per color component (e.g., 8, 16) |
| `isMask` | `Boolean` | — | Whether this image is a mask image |
| `description` | `String?` | `null` | Optional description of the image |
| `ocrResult` | `ExtractionResult?` | `null` | Nested OCR extraction result (if image was OCRed) When OCR is performed on this image, the result is embedded here rather than in a separate collection, making the relationship explicit. |
| `boundingBox` | `String?` | `null` | Bounding box of the image on the page (PDF coordinates: x0=left, y0=bottom, x1=right, y1=top). Only populated for PDF-extracted images when position data is available from pdfium. |
| `sourcePath` | `String?` | `null` | Original source path of the image within the document archive (e.g., "media/image1.png" in DOCX). Used for rendering image references when the binary data is not extracted. |


---

#### ExtractedInlineImage

Extracted inline image with metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `data` | `ByteArray` | — | Uses `bytes.Bytes` for cheap cloning of large buffers. |
| `format` | `String` | — | Format |
| `filename` | `String?` | `null` | Filename |
| `description` | `String?` | `null` | Human-readable description |
| `dimensions` | `List<Int>?` | `null` | Dimensions |
| `attributes` | `List<String>` | — | Attributes |


---

#### ExtractionConfig

Main extraction configuration.

This struct contains all configuration options for the extraction process.
It can be loaded from TOML, YAML, or JSON files, or created programmatically.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `useCache` | `Boolean` | `true` | Enable caching of extraction results |
| `enableQualityProcessing` | `Boolean` | `true` | Enable quality post-processing |
| `ocr` | `OcrConfig?` | `null` | OCR configuration (None = OCR disabled) |
| `forceOcr` | `Boolean` | `false` | Force OCR even for searchable PDFs |
| `forceOcrPages` | `List<Long>?` | `null` | Force OCR on specific pages only (1-indexed page numbers, must be >= 1). When set, only the listed pages are OCR'd regardless of text layer quality. Unlisted pages use native text extraction. Ignored when `force_ocr` is `true`. Only applies to PDF documents. Duplicates are automatically deduplicated. An `ocr` config is recommended for backend/language selection; defaults are used if absent. |
| `disableOcr` | `Boolean` | `false` | Disable OCR entirely, even for images. When `true`, OCR is skipped for all document types. Images return metadata only (dimensions, format, EXIF) without text extraction. PDFs use only native text extraction without OCR fallback. Cannot be `true` simultaneously with `force_ocr`. *Added in v4.7.0.* |
| `chunking` | `ChunkingConfig?` | `null` | Text chunking configuration (None = chunking disabled) |
| `contentFilter` | `ContentFilterConfig?` | `null` | Content filtering configuration (None = use extractor defaults). Controls whether document "furniture" (headers, footers, watermarks, repeating text) is included in or stripped from extraction results. See `ContentFilterConfig` for per-field documentation. |
| `images` | `ImageExtractionConfig?` | `null` | Image extraction configuration (None = no image extraction) |
| `pdfOptions` | `PdfConfig?` | `null` | PDF-specific options (None = use defaults) |
| `tokenReduction` | `TokenReductionOptions?` | `null` | Token reduction configuration (None = no token reduction) |
| `languageDetection` | `LanguageDetectionConfig?` | `null` | Language detection configuration (None = no language detection) |
| `pages` | `PageConfig?` | `null` | Page extraction configuration (None = no page tracking) |
| `postprocessor` | `PostProcessorConfig?` | `null` | Post-processor configuration (None = use defaults) |
| `htmlOptions` | `String?` | `null` | HTML to Markdown conversion options (None = use defaults) Configure how HTML documents are converted to Markdown, including heading styles, list formatting, code block styles, and preprocessing options. |
| `htmlOutput` | `HtmlOutputConfig?` | `null` | Styled HTML output configuration. When set alongside `output_format = OutputFormat.Html`, the extraction pipeline uses `StyledHtmlRenderer` which emits stable `kb-*` CSS class hooks on every structural element and optionally embeds theme CSS or user-supplied CSS in a `<style>` block. When `null`, the existing plain comrak-based HTML renderer is used. |
| `extractionTimeoutSecs` | `Long?` | `null` | Default per-file timeout in seconds for batch extraction. When set, each file in a batch will be canceled after this duration unless overridden by `FileExtractionConfig.timeout_secs`. `null` means no timeout (unbounded extraction time). |
| `maxConcurrentExtractions` | `Long?` | `null` | Maximum concurrent extractions in batch operations (None = (num_cpus × 1.5).ceil()). Limits parallelism to prevent resource exhaustion when processing large batches. Defaults to (num_cpus × 1.5).ceil() when not set. |
| `resultFormat` | `String` | — | Result structure format Controls whether results are returned in unified format (default) with all content in the `content` field, or element-based format with semantic elements (for Unstructured-compatible output). |
| `securityLimits` | `String?` | `null` | Security limits for archive extraction. Controls maximum archive size, compression ratio, file count, and other security thresholds to prevent decompression bomb attacks. Also caps nesting depth, iteration count, entity / token length, cumulative content size, and table cell count for every extraction path that ingests user-controlled bytes. When `null`, default limits are used. |
| `outputFormat` | `String` | `Plain` | Content text format (default: Plain). Controls the format of the extracted content: - `Plain`: Raw extracted text (default) - `Markdown`: Markdown formatted output - `Djot`: Djot markup format (requires djot feature) - `Html`: HTML formatted output When set to a structured format, extraction results will include formatted output. The `formatted_content` field may be populated when format conversion is applied. |
| `layout` | `LayoutDetectionConfig?` | `null` | Layout detection configuration (None = layout detection disabled). When set, PDF pages and images are analyzed for document structure (headings, code, formulas, tables, figures, etc.) using RT-DETR models via ONNX Runtime. For PDFs, layout hints override paragraph classification in the markdown pipeline. For images, per-region OCR is performed with markdown formatting based on detected layout classes. Requires the `layout-detection` feature. |
| `includeDocumentStructure` | `Boolean` | `false` | Enable structured document tree output. When true, populates the `document` field on `ExtractionResult` with a hierarchical `DocumentStructure` containing heading-driven section nesting, table grids, content layer classification, and inline annotations. Independent of `result_format` — can be combined with Unified or ElementBased. |
| `acceleration` | `AccelerationConfig?` | `null` | Hardware acceleration configuration for ONNX Runtime models. Controls execution provider selection for layout detection and embedding models. When `null`, uses platform defaults (CoreML on macOS, CUDA on Linux, CPU on Windows). |
| `cacheNamespace` | `String?` | `null` | Cache namespace for tenant isolation. When set, cache entries are stored under `{cache_dir}/{namespace}/`. Must be alphanumeric, hyphens, or underscores only (max 64 chars). Different namespaces have isolated cache spaces on the same filesystem. |
| `cacheTtlSecs` | `Long?` | `null` | Per-request cache TTL in seconds. Overrides the global `max_age_days` for this specific extraction. When `0`, caching is completely skipped (no read or write). When `null`, the global TTL applies. |
| `email` | `EmailConfig?` | `null` | Email extraction configuration (None = use defaults). Currently supports configuring the fallback codepage for MSG files that do not specify one. See `crate.core.config.EmailConfig` for details. |
| `concurrency` | `String?` | `null` | Concurrency limits for constrained environments (None = use defaults). Controls Rayon thread pool size, ONNX Runtime intra-op threads, and (when `max_concurrent_extractions` is unset) the batch concurrency semaphore. See `crate.core.config.ConcurrencyConfig` for details. |
| `maxArchiveDepth` | `Long` | — | Maximum recursion depth for archive extraction (default: 3). Set to 0 to disable recursive extraction (legacy behavior). |
| `treeSitter` | `TreeSitterConfig?` | `null` | Tree-sitter language pack configuration (None = tree-sitter disabled). When set, enables code file extraction using tree-sitter parsers. Controls grammar download behavior and code analysis options. |
| `structuredExtraction` | `StructuredExtractionConfig?` | `null` | Structured extraction via LLM (None = disabled). When set, the extracted document content is sent to an LLM with the provided JSON schema. The structured response is stored in `ExtractionResult.structured_output`. |
| `cancelToken` | `String?` | `null` | Cancellation token for this extraction (None = no external cancellation). Pass a `CancellationToken` clone here and call `CancellationToken.cancel` from another thread / task to abort the extraction in progress. The extractor checks the token at safe checkpoints (before lock acquisition, between pages, between batch items) and returns `KreuzbergError.Cancelled` when set. The field is excluded from serialization because `CancellationToken` is a runtime handle, not a configuration value. |

##### Methods

###### default()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### needsImageProcessing()

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

```kotlin
// Phase 1: kotlin backend method signature generation
```


---

#### ExtractionResult

General extraction result used by the core extraction API.

This is the main result type returned by all extraction functions.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String` | — | The extracted text content |
| `mimeType` | `String` | — | The detected MIME type |
| `metadata` | `Metadata` | — | Document metadata |
| `extractionMethod` | `ExtractionMethod?` | `null` | Extraction strategy used to produce the returned text. Populated when the extractor can reliably distinguish native text extraction, OCR-only extraction, or mixed native/OCR output. |
| `tables` | `List<String>` | `[]` | Tables extracted from the document |
| `detectedLanguages` | `List<String>?` | `[]` | Detected languages |
| `chunks` | `List<Chunk>?` | `[]` | Text chunks when chunking is enabled. When chunking configuration is provided, the content is split into overlapping chunks for efficient processing. Each chunk contains the text, optional embeddings (if enabled), and metadata about its position. |
| `images` | `List<ExtractedImage>?` | `[]` | Extracted images from the document. When image extraction is enabled via `ImageExtractionConfig`, this field contains all images found in the document with their raw data and metadata. Each image may optionally contain a nested `ocr_result` if OCR was performed. |
| `pages` | `List<PageContent>?` | `[]` | Per-page content when page extraction is enabled. When page extraction is configured, the document is split into per-page content with tables and images mapped to their respective pages. |
| `elements` | `List<Element>?` | `[]` | Semantic elements when element-based result format is enabled. When result_format is set to ElementBased, this field contains semantic elements with type classification, unique identifiers, and metadata for Unstructured-compatible element-based processing. |
| `djotContent` | `DjotContent?` | `null` | Rich Djot content structure (when extracting Djot documents). When extracting Djot documents with structured extraction enabled, this field contains the full semantic structure including: - Block-level elements with nesting - Inline formatting with attributes - Links, images, footnotes - Math expressions - Complete attribute information The `content` field still contains plain text for backward compatibility. Always `null` for non-Djot documents. |
| `ocrElements` | `List<OcrElement>?` | `[]` | OCR elements with full spatial and confidence metadata. When OCR is performed with element extraction enabled, this field contains the structured representation of detected text including: - Bounding geometry (rectangles or quadrilaterals) - Confidence scores (detection and recognition) - Rotation information - Hierarchical relationships (Tesseract only) This field preserves all metadata that would otherwise be lost when converting to plain text or markdown output formats. Only populated when `OcrElementConfig.include_elements` is true. |
| `document` | `DocumentStructure?` | `null` | Structured document tree (when document structure extraction is enabled). When `include_document_structure` is true in `ExtractionConfig`, this field contains the full hierarchical representation of the document including: - Heading-driven section nesting - Table grids with cell-level metadata - Content layer classification (body, header, footer, footnote) - Inline text annotations (formatting, links) - Bounding boxes and page numbers Independent of `result_format` — can be combined with Unified or ElementBased. |
| `qualityScore` | `Double?` | `null` | Document quality score from quality analysis. A value between 0.0 and 1.0 indicating the overall text quality. Previously stored in `metadata.additional["quality_score"]`. |
| `processingWarnings` | `List<ProcessingWarning>` | `[]` | Non-fatal warnings collected during processing pipeline stages. Captures errors from optional pipeline features (embedding, chunking, language detection, output formatting) that don't prevent extraction but may indicate degraded results. Previously stored as individual keys in `metadata.additional`. |
| `annotations` | `List<PdfAnnotation>?` | `[]` | PDF annotations extracted from the document. When annotation extraction is enabled via `PdfConfig.extract_annotations`, this field contains text notes, highlights, links, stamps, and other annotations found in PDF documents. |
| `children` | `List<ArchiveEntry>?` | `[]` | Nested extraction results from archive contents. When extracting archives, each processable file inside produces its own full extraction result. Set to `null` for non-archive formats. Use `max_archive_depth` in config to control recursion depth. |
| `uris` | `List<Uri>?` | `[]` | URIs/links discovered during document extraction. Contains hyperlinks, image references, citations, email addresses, and other URI-like references found in the document. Always extracted when present in the source document. |
| `structuredOutput` | `Any?` | `null` | Structured extraction output from LLM-based JSON schema extraction. When `structured_extraction` is configured in `ExtractionConfig`, the extracted document content is sent to a VLM with the provided JSON schema. The response is parsed and stored here as a JSON value matching the schema. |
| `codeIntelligence` | `String?` | `null` | Code intelligence results from tree-sitter analysis. Populated when extracting source code files with the `tree-sitter` feature. Contains metrics, structural analysis, imports/exports, comments, docstrings, symbols, diagnostics, and optionally chunked code segments. |
| `llmUsage` | `List<LlmUsage>?` | `[]` | LLM token usage and cost data for all LLM calls made during this extraction. Contains one entry per LLM call. Multiple entries are produced when VLM OCR, structured extraction, and/or LLM embeddings all run during the same extraction. `null` when no LLM was used. |
| `formattedContent` | `String?` | `null` | Pre-rendered content in the requested output format. Populated during `derive_extraction_result` before tree derivation consumes element data. `apply_output_format` swaps this into `content` at the end of the pipeline, after post-processors have operated on plain text. |
| `ocrInternalDocument` | `String?` | `null` | Structured hOCR document for the OCR+layout pipeline. When tesseract produces hOCR output, the parsed `InternalDocument` carries paragraph structure with bounding boxes and confidence scores. The layout classification step enriches these elements before final rendering. |


---

#### FictionBookMetadata

FictionBook (FB2) metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `genres` | `List<String>` | `[]` | Genres |
| `sequences` | `List<String>` | `[]` | Sequences |
| `annotation` | `String?` | `null` | Annotation |


---

#### FileExtractionConfig

Per-file extraction configuration overrides for batch processing.

All fields are `Option<T>` — `null` means "use the batch-level default."
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
| `enableQualityProcessing` | `Boolean?` | `null` | Override quality post-processing for this file. |
| `ocr` | `OcrConfig?` | `null` | Override OCR configuration for this file (None in the Option = use batch default). |
| `forceOcr` | `Boolean?` | `null` | Override force OCR for this file. |
| `forceOcrPages` | `List<Long>?` | `[]` | Override force OCR pages for this file (1-indexed page numbers). |
| `disableOcr` | `Boolean?` | `null` | Override disable OCR for this file. |
| `chunking` | `ChunkingConfig?` | `null` | Override chunking configuration for this file. |
| `contentFilter` | `ContentFilterConfig?` | `null` | Override content filtering configuration for this file. |
| `images` | `ImageExtractionConfig?` | `null` | Override image extraction configuration for this file. |
| `pdfOptions` | `PdfConfig?` | `null` | Override PDF options for this file. |
| `tokenReduction` | `TokenReductionOptions?` | `null` | Override token reduction for this file. |
| `languageDetection` | `LanguageDetectionConfig?` | `null` | Override language detection for this file. |
| `pages` | `PageConfig?` | `null` | Override page extraction for this file. |
| `postprocessor` | `PostProcessorConfig?` | `null` | Override post-processor for this file. |
| `htmlOptions` | `String?` | `null` | Override HTML conversion options for this file. |
| `resultFormat` | `String?` | `null` | Override result format for this file. |
| `outputFormat` | `String?` | `null` | Override output content format for this file. |
| `includeDocumentStructure` | `Boolean?` | `null` | Override document structure output for this file. |
| `layout` | `LayoutDetectionConfig?` | `null` | Override layout detection for this file. |
| `timeoutSecs` | `Long?` | `null` | Override per-file extraction timeout in seconds. When set, the extraction for this file will be canceled after the specified duration. A timed-out file produces an error result without affecting other files in the batch. |
| `treeSitter` | `TreeSitterConfig?` | `null` | Override tree-sitter configuration for this file. |
| `structuredExtraction` | `StructuredExtractionConfig?` | `null` | Override structured extraction configuration for this file. When set, enables LLM-based structured extraction with a JSON schema for this specific file. The extracted content is sent to a VLM/LLM and the response is parsed according to the provided schema. |


---

#### Footnote

Footnote in Djot.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `label` | `String` | — | Footnote label |
| `content` | `List<FormattedBlock>` | — | Footnote content blocks |


---

#### FormattedBlock

Block-level element in a Djot document.

Represents structural elements like headings, paragraphs, lists, code blocks, etc.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `blockType` | `BlockType` | — | Type of block element |
| `level` | `Long?` | `null` | Heading level (1-6) for headings, or nesting level for lists |
| `inlineContent` | `List<InlineElement>` | — | Inline content within the block |
| `attributes` | `String?` | `null` | Element attributes (classes, IDs, key-value pairs) |
| `language` | `String?` | `null` | Language identifier for code blocks |
| `code` | `String?` | `null` | Raw code content for code blocks |
| `children` | `List<FormattedBlock>` | — | Nested blocks for containers (blockquotes, list items, divs) |


---

#### GridCell

Individual grid cell with position and span metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String` | — | Cell text content. |
| `row` | `Int` | — | Zero-indexed row position. |
| `col` | `Int` | — | Zero-indexed column position. |
| `rowSpan` | `Int` | — | Number of rows this cell spans. |
| `colSpan` | `Int` | — | Number of columns this cell spans. |
| `isHeader` | `Boolean` | — | Whether this is a header cell. |
| `bbox` | `String?` | `null` | Bounding box for this cell (if available). |


---

#### HeaderFooter

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `paragraphs` | `List<String>` | `[]` | Paragraphs |
| `tables` | `List<String>` | `[]` | Tables extracted from the document |
| `headerType` | `String` | — | Header type |


---

#### HeaderMetadata

Header/heading element metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `level` | `Byte` | — | Header level: 1 (h1) through 6 (h6) |
| `text` | `String` | — | Normalized text content of the header |
| `id` | `String?` | `null` | HTML id attribute if present |
| `depth` | `Long` | — | Document tree depth at the header element |
| `htmlOffset` | `Long` | — | Byte offset in original HTML document |


---

#### HeadingContext

Heading context for a chunk within a Markdown document.

Contains the heading hierarchy from document root to this chunk's section.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `headings` | `List<HeadingLevel>` | — | The heading hierarchy from document root to this chunk's section. Index 0 is the outermost (h1), last element is the most specific. |


---

#### HeadingLevel

A single heading in the hierarchy.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `level` | `Byte` | — | Heading depth (1 = h1, 2 = h2, etc.) |
| `text` | `String` | — | The text content of the heading. |


---

#### HealthResponse

Health check response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `status` | `String` | — | Health status |
| `version` | `String` | — | API version |
| `plugins` | `String?` | `null` | Plugin status (optional) |


---

#### HierarchicalBlock

A text block with hierarchy level assignment.

Represents a block of text with semantic heading information extracted from
font size clustering and hierarchical analysis.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text` | `String` | — | The text content of this block |
| `fontSize` | `Float` | — | The font size of the text in this block |
| `level` | `String` | — | The hierarchy level of this block (H1-H6 or Body) Levels correspond to HTML heading tags: - "h1": Top-level heading - "h2": Secondary heading - "h3": Tertiary heading - "h4": Quaternary heading - "h5": Quinary heading - "h6": Senary heading - "body": Body text (no heading level) |
| `bbox` | `List<Float>?` | `null` | Bounding box information for the block Contains coordinates as (left, top, right, bottom) in PDF units. |


---

#### HierarchyConfig

Hierarchy extraction configuration for PDF text structure analysis.

Enables extraction of document hierarchy levels (H1-H6) based on font size
clustering and semantic analysis. When enabled, hierarchical blocks are
included in page content.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `Boolean` | `true` | Enable hierarchy extraction |
| `kClusters` | `Long` | `3` | Number of font size clusters to use for hierarchy levels (1-7) Default: 6, which provides H1-H6 heading levels with body text. Larger values create more fine-grained hierarchy levels. |
| `includeBbox` | `Boolean` | `true` | Include bounding box information in hierarchy blocks |
| `ocrCoverageThreshold` | `Float?` | `null` | OCR coverage threshold for smart OCR triggering (0.0-1.0) Determines when OCR should be triggered based on text block coverage. OCR is triggered when text blocks cover less than this fraction of the page. Default: 0.5 (trigger OCR if less than 50% of page has text) |

##### Methods

###### default()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```


---

#### HtmlExtractionResult

Result of HTML extraction with optional images and warnings.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `markdown` | `String` | — | Markdown |
| `images` | `List<ExtractedInlineImage>` | — | Images extracted from the document |
| `warnings` | `List<String>` | — | Warnings |


---

#### HtmlMetadata

HTML metadata extracted from HTML documents.

Includes document-level metadata, Open Graph data, Twitter Card metadata,
and extracted structural elements (headers, links, images, structured data).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `title` | `String?` | `null` | Document title from `<title>` tag |
| `description` | `String?` | `null` | Document description from `<meta name="description">` tag |
| `keywords` | `List<String>` | `[]` | Document keywords from `<meta name="keywords">` tag, split on commas |
| `author` | `String?` | `null` | Document author from `<meta name="author">` tag |
| `canonicalUrl` | `String?` | `null` | Canonical URL from `<link rel="canonical">` tag |
| `baseHref` | `String?` | `null` | Base URL from `<base href="">` tag for resolving relative URLs |
| `language` | `String?` | `null` | Document language from `lang` attribute |
| `textDirection` | `TextDirection?` | `null` | Document text direction from `dir` attribute |
| `openGraph` | `Map<String, String>` | `{}` | Open Graph metadata (og:* properties) for social media Keys like "title", "description", "image", "url", etc. |
| `twitterCard` | `Map<String, String>` | `{}` | Twitter Card metadata (twitter:* properties) Keys like "card", "site", "creator", "title", "description", "image", etc. |
| `metaTags` | `Map<String, String>` | `{}` | Additional meta tags not covered by specific fields Keys are meta name/property attributes, values are content |
| `headers` | `List<HeaderMetadata>` | `[]` | Extracted header elements with hierarchy |
| `links` | `List<LinkMetadata>` | `[]` | Extracted hyperlinks with type classification |
| `images` | `List<ImageMetadataType>` | `[]` | Extracted images with source and dimensions |
| `structuredData` | `List<StructuredData>` | `[]` | Extracted structured data blocks |

##### Methods

###### from()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```


---

#### HtmlOutputConfig

Configuration for styled HTML output.

When set on `ExtractionConfig.html_output` alongside
`output_format = OutputFormat.Html`, the pipeline builds a
`StyledHtmlRenderer` instead of
the plain comrak-based renderer.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `css` | `String?` | `null` | Inline CSS string injected into the output after the theme stylesheet. Concatenated after `css_file` content when both are set. |
| `cssFile` | `Path?` | `null` | Path to a CSS file loaded once at renderer construction time. Concatenated before `css` when both are set. |
| `theme` | `HtmlTheme` | `HtmlTheme.Unstyled` | Built-in colour/typography theme. Default: `HtmlTheme.Unstyled`. |
| `classPrefix` | `String` | — | CSS class prefix applied to every emitted class name. Default: `"kb-"`. Change this if your host application already uses classes that start with `kb-`. |
| `embedCss` | `Boolean` | `true` | When `true` (default), write the resolved CSS into a `<style>` block immediately after the opening `<div class="{prefix}doc">`. Set to `false` to emit only the structural markup and wire up your own stylesheet targeting the `kb-*` class names. |

##### Methods

###### default()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```


---

#### ImageExtractionConfig

Image extraction configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `extractImages` | `Boolean` | `true` | Extract images from documents |
| `targetDpi` | `Int` | `300` | Target DPI for image normalization |
| `maxImageDimension` | `Int` | `4096` | Maximum dimension for images (width or height) |
| `injectPlaceholders` | `Boolean` | `true` | Whether to inject image reference placeholders into markdown output. When `true` (default), image references like `![Image 1](embedded:p1_i0)` are appended to the markdown. Set to `false` to extract images as data without polluting the markdown output. |
| `autoAdjustDpi` | `Boolean` | `true` | Automatically adjust DPI based on image content |
| `minDpi` | `Int` | `72` | Minimum DPI threshold |
| `maxDpi` | `Int` | `600` | Maximum DPI threshold |
| `maxImagesPerPage` | `Int?` | `null` | Maximum number of image objects to extract per PDF page. Some PDFs (e.g. technical diagrams stored as thousands of raster fragments) can trigger extremely long or indefinite extraction times when every image object on a dense page is decoded individually via pdfium FFI. Setting this limit causes kreuzberg to stop collecting individual images once the count per page reaches the cap and emit a warning instead. `null` (default) means no limit — all images are extracted. |

##### Methods

###### default()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```


---

#### ImageMetadataType

Image element metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `src` | `String` | — | Image source (URL, data URI, or SVG content) |
| `alt` | `String?` | `null` | Alternative text from alt attribute |
| `title` | `String?` | `null` | Title attribute |
| `dimensions` | `List<Int>?` | `null` | Image dimensions as (width, height) if available |
| `imageType` | `ImageType` | — | Image type classification |
| `attributes` | `List<String>` | — | Additional attributes as key-value pairs |


---

#### ImageOcrResult

Result of OCR extraction from an image with optional page tracking.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String` | — | Extracted text content |
| `boundaries` | `List<PageBoundary>?` | `null` | Character byte boundaries per frame (for multi-frame TIFFs) |
| `pageContents` | `List<PageContent>?` | `null` | Per-frame content information |


---

#### ImagePreprocessingConfig

Image preprocessing configuration for OCR.

These settings control how images are preprocessed before OCR to improve
text recognition quality. Different preprocessing strategies work better
for different document types.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `targetDpi` | `Int` | `300` | Target DPI for the image (300 is standard, 600 for small text). |
| `autoRotate` | `Boolean` | `true` | Auto-detect and correct image rotation. |
| `deskew` | `Boolean` | `true` | Correct skew (tilted images). |
| `denoise` | `Boolean` | `false` | Remove noise from the image. |
| `contrastEnhance` | `Boolean` | `false` | Enhance contrast for better text visibility. |
| `binarizationMethod` | `String` | `"otsu"` | Binarization method: "otsu", "sauvola", "adaptive". |
| `invertColors` | `Boolean` | `false` | Invert colors (white text on black → black on white). |

##### Methods

###### default()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```


---

#### ImagePreprocessingMetadata

Image preprocessing metadata.

Tracks the transformations applied to an image during OCR preprocessing,
including DPI normalization, resizing, and resampling.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `originalDimensions` | `List<Long>` | — | Original image dimensions (width, height) in pixels |
| `originalDpi` | `List<Double>` | — | Original image DPI (horizontal, vertical) |
| `targetDpi` | `Int` | — | Target DPI from configuration |
| `scaleFactor` | `Double` | — | Scaling factor applied to the image |
| `autoAdjusted` | `Boolean` | — | Whether DPI was auto-adjusted based on content |
| `finalDpi` | `Int` | — | Final DPI after processing |
| `newDimensions` | `List<Long>?` | `null` | New dimensions after resizing (if resized) |
| `resampleMethod` | `String` | — | Resampling algorithm used ("LANCZOS3", "CATMULLROM", etc.) |
| `dimensionClamped` | `Boolean` | — | Whether dimensions were clamped to max_image_dimension |
| `calculatedDpi` | `Int?` | `null` | Calculated optimal DPI (if auto_adjust_dpi enabled) |
| `skippedResize` | `Boolean` | — | Whether resize was skipped (dimensions already optimal) |
| `resizeError` | `String?` | `null` | Error message if resize failed |


---

#### InfoResponse

Server information response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `version` | `String` | — | API version |
| `rustBackend` | `Boolean` | — | Whether using Rust backend |


---

#### InlineElement

Inline element within a block.

Represents text with formatting, links, images, etc.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `elementType` | `InlineType` | — | Type of inline element |
| `content` | `String` | — | Text content |
| `attributes` | `String?` | `null` | Element attributes |
| `metadata` | `Map<String, String>?` | `null` | Additional metadata (e.g., href for links, src/alt for images) |


---

#### JatsMetadata

JATS (Journal Article Tag Suite) metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `copyright` | `String?` | `null` | Copyright |
| `license` | `String?` | `null` | License |
| `historyDates` | `Map<String, String>` | `{}` | History dates |
| `contributorRoles` | `List<ContributorRole>` | `[]` | Contributor roles |


---

#### Keyword

Extracted keyword with metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text` | `String` | — | The keyword text. |
| `score` | `Float` | — | Relevance score (higher is better, algorithm-specific range). |
| `algorithm` | `KeywordAlgorithm` | — | Algorithm that extracted this keyword. |
| `positions` | `List<Long>?` | `null` | Optional positions where keyword appears in text (character offsets). |


---

#### KeywordConfig

Keyword extraction configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `algorithm` | `KeywordAlgorithm` | `KeywordAlgorithm.Yake` | Algorithm to use for extraction. |
| `maxKeywords` | `Long` | `10` | Maximum number of keywords to extract (default: 10). |
| `minScore` | `Float` | `0` | Minimum score threshold (0.0-1.0, default: 0.0). Keywords with scores below this threshold are filtered out. Note: Score ranges differ between algorithms. |
| `ngramRange` | `List<Long>` | `[]` | N-gram range for keyword extraction (min, max). (1, 1) = unigrams only (1, 2) = unigrams and bigrams (1, 3) = unigrams, bigrams, and trigrams (default) |
| `language` | `String?` | `null` | Language code for stopword filtering (e.g., "en", "de", "fr"). If None, no stopword filtering is applied. |
| `yakeParams` | `YakeParams?` | `null` | YAKE-specific tuning parameters. |
| `rakeParams` | `RakeParams?` | `null` | RAKE-specific tuning parameters. |

##### Methods

###### default()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```


---

#### LanguageDetectionConfig

Language detection configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `Boolean` | — | Enable language detection |
| `minConfidence` | `Double` | — | Minimum confidence threshold (0.0-1.0) |
| `detectMultiple` | `Boolean` | — | Detect multiple languages in the document |


---

#### LayoutDetection

A single layout detection result.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `className` | `LayoutClass` | — | Class name (layout class) |
| `confidence` | `Float` | — | Confidence |
| `bbox` | `BBox` | — | Bbox (b box) |


---

#### LayoutDetectionConfig

Layout detection configuration.

Controls layout detection behavior in the extraction pipeline.
When set on `ExtractionConfig`, layout detection
is enabled for PDF extraction.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `confidenceThreshold` | `Float?` | `null` | Confidence threshold override (None = use model default). |
| `applyHeuristics` | `Boolean` | `true` | Whether to apply postprocessing heuristics (default: true). |
| `tableModel` | `TableModel` | `TableModel.Tatr` | Table structure recognition model. Controls which model is used for table cell detection within layout-detected table regions. Defaults to `TableModel.Tatr`. |
| `acceleration` | `AccelerationConfig?` | `null` | Hardware acceleration for ONNX models (layout detection + table structure). When set, controls which execution provider (CPU, CUDA, CoreML, TensorRT) is used for inference. Defaults to `null` (auto-select per platform). |

##### Methods

###### default()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```


---

#### LayoutRegion

A detected layout region on a page.

When layout detection is enabled, each page may have layout regions
identifying different content types (text, pictures, tables, etc.)
with confidence scores and spatial positions.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `className` | `String` | — | Layout class name (e.g. "picture", "table", "text", "section_header"). |
| `confidence` | `Double` | — | Confidence score from the layout detection model (0.0 to 1.0). |
| `boundingBox` | `String` | — | Bounding box in document coordinate space. |
| `areaFraction` | `Double` | — | Fraction of the page area covered by this region (0.0 to 1.0). |


---

#### LinkMetadata

Link element metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `href` | `String` | — | The href URL value |
| `text` | `String` | — | Link text content (normalized) |
| `title` | `String?` | `null` | Optional title attribute |
| `linkType` | `LinkType` | — | Link type classification |
| `rel` | `List<String>` | — | Rel attribute values |
| `attributes` | `List<String>` | — | Additional attributes as key-value pairs |


---

#### LlmConfig

Configuration for an LLM provider/model via liter-llm.

Each feature (VLM OCR, VLM embeddings, structured extraction) carries
its own `LlmConfig`, allowing different providers per feature.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String` | — | Provider/model string using liter-llm routing format. Examples: `"openai/gpt-4o"`, `"anthropic/claude-sonnet-4-20250514"`, `"groq/llama-3.1-70b-versatile"`. |
| `apiKey` | `String?` | `null` | API key for the provider. When `null`, liter-llm falls back to the provider's standard environment variable (e.g., `OPENAI_API_KEY`). |
| `baseUrl` | `String?` | `null` | Custom base URL override for the provider endpoint. |
| `timeoutSecs` | `Long?` | `null` | Request timeout in seconds (default: 60). |
| `maxRetries` | `Int?` | `null` | Maximum retry attempts (default: 3). |
| `temperature` | `Double?` | `null` | Sampling temperature for generation tasks. |
| `maxTokens` | `Long?` | `null` | Maximum tokens to generate. |


---

#### LlmUsage

Token usage and cost data for a single LLM call made during extraction.

Populated when VLM OCR, structured extraction, or LLM-based embeddings
are used. Multiple entries may be present when multiple LLM calls occur
within one extraction (e.g. VLM OCR + structured extraction).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String` | — | The LLM model identifier (e.g. "openai/gpt-4o", "anthropic/claude-sonnet-4-20250514"). |
| `source` | `String` | — | The pipeline stage that triggered this LLM call (e.g. "vlm_ocr", "structured_extraction", "embeddings"). |
| `inputTokens` | `Long?` | `null` | Number of input/prompt tokens consumed. |
| `outputTokens` | `Long?` | `null` | Number of output/completion tokens generated. |
| `totalTokens` | `Long?` | `null` | Total tokens (input + output). |
| `estimatedCost` | `Double?` | `null` | Estimated cost in USD based on the provider's published pricing. |
| `finishReason` | `String?` | `null` | Why the model stopped generating (e.g. "stop", "length", "content_filter"). |


---

#### ManifestEntryResponse

Model manifest entry for cache management.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `relativePath` | `String` | — | Relative path within the cache directory |
| `sha256` | `String` | — | SHA256 checksum of the model file |
| `sizeBytes` | `Long` | — | Expected file size in bytes |
| `sourceUrl` | `String` | — | HuggingFace source URL for downloading |


---

#### ManifestResponse

Model manifest response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `kreuzbergVersion` | `String` | — | Kreuzberg version |
| `totalSizeBytes` | `Long` | — | Total size of all models in bytes |
| `modelCount` | `Long` | — | Number of models in the manifest |
| `models` | `List<ManifestEntryResponse>` | — | Individual model entries |


---

#### MergedChunk

A merged chunk produced by `merge_segments`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text` | `String` | — | Text |
| `byteStart` | `Long` | — | Byte start |
| `byteEnd` | `Long` | — | Byte end |


---

#### Metadata

Extraction result metadata.

Contains common fields applicable to all formats, format-specific metadata
via a discriminated union, and additional custom fields from postprocessors.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `title` | `String?` | `null` | Document title |
| `subject` | `String?` | `null` | Document subject or description |
| `authors` | `List<String>?` | `[]` | Primary author(s) - always Vec for consistency |
| `keywords` | `List<String>?` | `[]` | Keywords/tags - always Vec for consistency |
| `language` | `String?` | `null` | Primary language (ISO 639 code) |
| `createdAt` | `String?` | `null` | Creation timestamp (ISO 8601 format) |
| `modifiedAt` | `String?` | `null` | Last modification timestamp (ISO 8601 format) |
| `createdBy` | `String?` | `null` | User who created the document |
| `modifiedBy` | `String?` | `null` | User who last modified the document |
| `pages` | `PageStructure?` | `null` | Page/slide/sheet structure with boundaries |
| `format` | `FormatMetadata?` | `null` | Format-specific metadata (discriminated union) Contains detailed metadata specific to the document format. Serializes with a `format_type` discriminator field. |
| `imagePreprocessing` | `ImagePreprocessingMetadata?` | `null` | Image preprocessing metadata (when OCR preprocessing was applied) |
| `jsonSchema` | `Any?` | `null` | JSON schema (for structured data extraction) |
| `error` | `ErrorMetadata?` | `null` | Error metadata (for batch operations) |
| `extractionDurationMs` | `Long?` | `null` | Extraction duration in milliseconds (for benchmarking). This field is populated by batch extraction to provide per-file timing information. It's `null` for single-file extraction (which uses external timing). |
| `category` | `String?` | `null` | Document category (from frontmatter or classification). |
| `tags` | `List<String>?` | `[]` | Document tags (from frontmatter). |
| `documentVersion` | `String?` | `null` | Document version string (from frontmatter). |
| `abstractText` | `String?` | `null` | Abstract or summary text (from frontmatter). |
| `outputFormat` | `String?` | `null` | Output format identifier (e.g., "markdown", "html", "text"). Set by the output format pipeline stage when format conversion is applied. Previously stored in `metadata.additional["output_format"]`. |
| `additional` | `String` | — | Additional custom fields from postprocessors. **Deprecated**: Prefer using typed fields on `ExtractionResult` and `Metadata` instead of inserting into this map. Typed fields provide better cross-language compatibility and type safety. This field will be removed in a future major version. This flattened map allows Python/TypeScript postprocessors to add arbitrary fields (entity extraction, keyword extraction, etc.). Fields are merged at the root level during serialization. Uses `Cow<'static, str>` keys so static string keys avoid allocation. |


---

#### ModelPaths

Combined paths to all models needed for OCR (backward compatibility).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `detModel` | `Path` | — | Path to the detection model directory. |
| `clsModel` | `Path` | — | Path to the classification model directory. |
| `recModel` | `Path` | — | Path to the recognition model directory. |
| `dictFile` | `Path` | — | Path to the character dictionary file. |


---

#### Note

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String` | — | Unique identifier |
| `noteType` | `String` | — | Note type |
| `paragraphs` | `List<String>` | — | Paragraphs |


---

#### OcrBackend

Trait for OCR backend plugins.

Implement this trait to add custom OCR capabilities. OCR backends can be:
- Native Rust implementations (like Tesseract)
- FFI bridges to Python libraries (like EasyOCR, PaddleOCR)
- Cloud-based OCR services (Google Vision, AWS Textract, etc.)

# Thread Safety

OCR backends must be thread-safe (`Send + Sync`) to support concurrent processing.

##### Methods

###### processImage()

Process an image and extract text via OCR.

**Returns:**

An `ExtractionResult` containing the extracted text and metadata.

**Errors:**

- `KreuzbergError.Ocr` - OCR processing failed
- `KreuzbergError.Validation` - Invalid image format or configuration
- `KreuzbergError.Io` - I/O errors (these always bubble up)

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### processImageFile()

Process a file and extract text via OCR.

Default implementation reads the file and calls `process_image`.
Override for custom file handling or optimizations.

**Errors:**

Same as `process_image`, plus file I/O errors.

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### supportsLanguage()

Check if this backend supports a given language code.

**Returns:**

`true` if the language is supported, `false` otherwise.

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### backendType()

Get the backend type identifier.

**Returns:**

The backend type enum value.

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### supportedLanguages()

Optional: Get a list of all supported languages.

Defaults to empty list. Override to provide comprehensive language support info.

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### supportsTableDetection()

Optional: Check if the backend supports table detection.

Defaults to `false`. Override if your backend can detect and extract tables.

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### supportsDocumentProcessing()

Check if the backend supports direct document-level processing (e.g. for PDFs).

Defaults to `false`. Override if the backend has optimized document processing.

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### processDocument()

Process a document file directly via OCR.

Only called if `supports_document_processing` returns `true`.

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```


---

#### OcrCacheStats

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `totalFiles` | `Long` | — | Total files |
| `totalSizeMb` | `Double` | — | Total size mb |


---

#### OcrConfidence

Confidence scores for an OCR element.

Separates detection confidence (how confident that text exists at this location)
from recognition confidence (how confident about the actual text content).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `detection` | `Double?` | `null` | Detection confidence: how confident the OCR engine is that text exists here. PaddleOCR provides this as `box_score`, Tesseract doesn't have a direct equivalent. Range: 0.0 to 1.0 (or None if not available). |
| `recognition` | `Double` | — | Recognition confidence: how confident about the text content. Range: 0.0 to 1.0. |


---

#### OcrConfig

OCR configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `Boolean` | `true` | Whether OCR is enabled. Setting `enabled: false` is a shorthand for `disable_ocr: true` on the parent `ExtractionConfig`. Images return metadata only; PDFs use native text extraction without OCR fallback. Defaults to `true`. When `false`, all other OCR settings are ignored. |
| `backend` | `String` | — | OCR backend: tesseract, easyocr, paddleocr |
| `language` | `String` | — | Language code (e.g., "eng", "deu") |
| `tesseractConfig` | `TesseractConfig?` | `null` | Tesseract-specific configuration (optional) |
| `outputFormat` | `String?` | `null` | Output format for OCR results (optional, for format conversion) |
| `paddleOcrConfig` | `Any?` | `null` | PaddleOCR-specific configuration (optional, JSON passthrough) |
| `elementConfig` | `OcrElementConfig?` | `null` | OCR element extraction configuration |
| `qualityThresholds` | `OcrQualityThresholds?` | `null` | Quality thresholds for the native-text-to-OCR fallback decision. When None, uses compiled defaults (matching previous hardcoded behavior). |
| `pipeline` | `OcrPipelineConfig?` | `null` | Multi-backend OCR pipeline configuration. When set, enables weighted fallback across multiple OCR backends based on output quality. When None, uses the single `backend` field (same as today). |
| `autoRotate` | `Boolean` | `false` | Enable automatic page rotation based on orientation detection. When enabled, uses Tesseract's `DetectOrientationScript()` to detect page orientation (0/90/180/270 degrees) before OCR. If the page is rotated with high confidence, the image is corrected before recognition. This is critical for handling rotated scanned documents. |
| `vlmConfig` | `LlmConfig?` | `null` | VLM (Vision Language Model) OCR configuration. Required when `backend` is `"vlm"`. Uses liter-llm to send page images to a vision model for text extraction. |
| `vlmPrompt` | `String?` | `null` | Custom Jinja2 prompt template for VLM OCR. When `null`, uses the default template. Available variables: - `{{ language }}` — The document language code (e.g., "eng", "deu"). |
| `acceleration` | `AccelerationConfig?` | `null` | Hardware acceleration for ONNX Runtime models (e.g. PaddleOCR, layout detection). Not user-configurable via config files — injected at runtime from `ExtractionConfig.acceleration` before each `process_image` call. |

##### Methods

###### default()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```


---

#### OcrElement

A unified OCR element representing detected text with full metadata.

This is the primary type for structured OCR output, preserving all information
from both Tesseract and PaddleOCR backends.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text` | `String` | — | The recognized text content. |
| `geometry` | `OcrBoundingGeometry` | `OcrBoundingGeometry.Rectangle` | Bounding geometry (rectangle or quadrilateral). |
| `confidence` | `OcrConfidence` | — | Confidence scores for detection and recognition. |
| `level` | `OcrElementLevel` | `OcrElementLevel.Line` | Hierarchical level (word, line, block, page). |
| `rotation` | `OcrRotation?` | `null` | Rotation information (if detected). |
| `pageNumber` | `Long` | — | Page number (1-indexed). |
| `parentId` | `String?` | `null` | Parent element ID for hierarchical relationships. Only used for Tesseract output which has word -> line -> block hierarchy. |
| `backendMetadata` | `Map<String, Any>` | `{}` | Backend-specific metadata that doesn't fit the unified schema. |


---

#### OcrElementConfig

Configuration for OCR element extraction.

Controls how OCR elements are extracted and filtered.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `includeElements` | `Boolean` | — | Whether to include OCR elements in the extraction result. When true, the `ocr_elements` field in `ExtractionResult` will be populated. |
| `minLevel` | `OcrElementLevel` | `OcrElementLevel.Line` | Minimum hierarchical level to include. Elements below this level (e.g., words when min_level is Line) will be excluded. |
| `minConfidence` | `Double` | — | Minimum recognition confidence threshold (0.0-1.0). Elements with confidence below this threshold will be filtered out. |
| `buildHierarchy` | `Boolean` | — | Whether to build hierarchical relationships between elements. When true, `parent_id` fields will be populated based on spatial containment. Only meaningful for Tesseract output. |


---

#### OcrExtractionResult

OCR extraction result.

Result of performing OCR on an image or scanned document,
including recognized text and detected tables.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String` | — | Recognized text content |
| `mimeType` | `String` | — | Original MIME type of the processed image |
| `metadata` | `Map<String, Any>` | — | OCR processing metadata (confidence scores, language, etc.) |
| `tables` | `List<OcrTable>` | — | Tables detected and extracted via OCR |
| `ocrElements` | `List<OcrElement>?` | `null` | Structured OCR elements with bounding boxes and confidence scores. Available when TSV output is requested or table detection is enabled. |
| `internalDocument` | `String?` | `null` | Structured document produced from hOCR parsing. Carries paragraph structure, bounding boxes, and confidence scores that the flattened `content` string discards. |


---

#### OcrMetadata

OCR processing metadata.

Captures information about OCR processing configuration and results.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `language` | `String` | — | OCR language code(s) used |
| `psm` | `Int` | — | Tesseract Page Segmentation Mode (PSM) |
| `outputFormat` | `String` | — | Output format (e.g., "text", "hocr") |
| `tableCount` | `Long` | — | Number of tables detected |
| `tableRows` | `Long?` | `null` | Table rows |
| `tableCols` | `Long?` | `null` | Table cols |


---

#### OcrPipelineConfig

Multi-backend OCR pipeline with quality-based fallback.

Backends are tried in priority order (highest first). After each backend
produces output, quality is evaluated. If it meets `quality_thresholds.pipeline_min_quality`,
the result is accepted. Otherwise the next backend is tried.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `stages` | `List<OcrPipelineStage>` | — | Ordered list of backends to try. Sorted by priority (descending) at runtime. |
| `qualityThresholds` | `OcrQualityThresholds` | — | Quality thresholds for deciding whether to accept a result or try the next backend. |


---

#### OcrPipelineStage

A single backend stage in the OCR pipeline.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `backend` | `String` | — | Backend name: "tesseract", "paddleocr", "easyocr", or a custom registered name. |
| `priority` | `Int` | — | Priority weight (higher = tried first). Stages are sorted by priority descending. |
| `language` | `String?` | `null` | Language override for this stage (None = use parent OcrConfig.language). |
| `tesseractConfig` | `TesseractConfig?` | `null` | Tesseract-specific config override for this stage. |
| `paddleOcrConfig` | `Any?` | `null` | PaddleOCR-specific config for this stage. |
| `vlmConfig` | `LlmConfig?` | `null` | VLM config override for this pipeline stage. |


---

#### OcrQualityThresholds

Quality thresholds for OCR fallback decisions and pipeline quality gating.

All fields default to the values that match the previous hardcoded behavior,
so `OcrQualityThresholds.default()` preserves existing semantics exactly.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `minTotalNonWhitespace` | `Long` | `64` | Minimum total non-whitespace characters to consider text substantive. |
| `minNonWhitespacePerPage` | `Double` | `32` | Minimum non-whitespace characters per page on average. |
| `minMeaningfulWordLen` | `Long` | `4` | Minimum character count for a word to be "meaningful". |
| `minMeaningfulWords` | `Long` | `3` | Minimum count of meaningful words before text is accepted. |
| `minAlnumRatio` | `Double` | `0.3` | Minimum alphanumeric ratio (non-whitespace chars that are alphanumeric). |
| `minGarbageChars` | `Long` | `5` | Minimum Unicode replacement characters (U+FFFD) to trigger OCR fallback. |
| `maxFragmentedWordRatio` | `Double` | `0.6` | Maximum fraction of short (1-2 char) words before text is considered fragmented. |
| `criticalFragmentedWordRatio` | `Double` | `0.8` | Critical fragmentation threshold — triggers OCR regardless of meaningful words. Normal English text has ~20-30% short words. 80%+ is definitive garbage. |
| `minAvgWordLength` | `Double` | `2` | Minimum average word length. Below this with enough words indicates garbled extraction. |
| `minWordsForAvgLengthCheck` | `Long` | `50` | Minimum word count before average word length check applies. |
| `minConsecutiveRepeatRatio` | `Double` | `0.08` | Minimum consecutive word repetition ratio to detect column scrambling. |
| `minWordsForRepeatCheck` | `Long` | `50` | Minimum word count before consecutive repetition check is applied. |
| `substantiveMinChars` | `Long` | `100` | Minimum character count for "substantive markdown" OCR skip gate. |
| `nonTextMinChars` | `Long` | `20` | Minimum character count for "non-text content" OCR skip gate. |
| `alnumWsRatioThreshold` | `Double` | `0.4` | Alphanumeric+whitespace ratio threshold for skip decisions. |
| `pipelineMinQuality` | `Double` | `0.5` | Minimum quality score (0.0-1.0) for a pipeline stage result to be accepted. If the result from a backend scores below this, try the next backend. |

##### Methods

###### default()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```


---

#### OcrRotation

Rotation information for an OCR element.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `angleDegrees` | `Double` | — | Rotation angle in degrees (0, 90, 180, 270 for PaddleOCR). |
| `confidence` | `Double?` | `null` | Confidence score for the rotation detection. |


---

#### OcrTable

Table detected via OCR.

Represents a table structure recognized during OCR processing.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `cells` | `List<List<String>>` | — | Table cells as a 2D vector (rows × columns) |
| `markdown` | `String` | — | Markdown representation of the table |
| `pageNumber` | `Long` | — | Page number where the table was found (1-indexed) |
| `boundingBox` | `OcrTableBoundingBox?` | `null` | Bounding box of the table in pixel coordinates (from OCR word positions). |


---

#### OcrTableBoundingBox

Bounding box for an OCR-detected table in pixel coordinates.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `left` | `Int` | — | Left x-coordinate (pixels) |
| `top` | `Int` | — | Top y-coordinate (pixels) |
| `right` | `Int` | — | Right x-coordinate (pixels) |
| `bottom` | `Int` | — | Bottom y-coordinate (pixels) |


---

#### OdtProperties

OpenDocument metadata from meta.xml

Contains metadata fields defined by the OASIS OpenDocument Format standard.
Uses Dublin Core elements (dc:) and OpenDocument meta elements (meta:).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `title` | `String?` | `null` | Document title (dc:title) |
| `subject` | `String?` | `null` | Document subject/topic (dc:subject) |
| `creator` | `String?` | `null` | Current document creator/author (dc:creator) |
| `initialCreator` | `String?` | `null` | Initial creator of the document (meta:initial-creator) |
| `keywords` | `String?` | `null` | Keywords or tags (meta:keyword) |
| `description` | `String?` | `null` | Document description (dc:description) |
| `date` | `String?` | `null` | Current modification date (dc:date) |
| `creationDate` | `String?` | `null` | Initial creation date (meta:creation-date) |
| `language` | `String?` | `null` | Document language (dc:language) |
| `generator` | `String?` | `null` | Generator/application that created the document (meta:generator) |
| `editingDuration` | `String?` | `null` | Editing duration in ISO 8601 format (meta:editing-duration) |
| `editingCycles` | `String?` | `null` | Number of edits/revisions (meta:editing-cycles) |
| `pageCount` | `Int?` | `null` | Document statistics - page count (meta:page-count) |
| `wordCount` | `Int?` | `null` | Document statistics - word count (meta:word-count) |
| `characterCount` | `Int?` | `null` | Document statistics - character count (meta:character-count) |
| `paragraphCount` | `Int?` | `null` | Document statistics - paragraph count (meta:paragraph-count) |
| `tableCount` | `Int?` | `null` | Document statistics - table count (meta:table-count) |
| `imageCount` | `Int?` | `null` | Document statistics - image count (meta:image-count) |


---

#### OpenWebDocumentResponse

OpenWebUI "External" engine response format.

Returned by `PUT /process` for the OpenWebUI external document loader.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `pageContent` | `String` | — | Extracted text content |
| `metadata` | `String` | — | Document metadata |


---

#### OrientationResult

Document orientation detection result.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `degrees` | `Int` | — | Detected orientation in degrees (0, 90, 180, or 270). |
| `confidence` | `Float` | — | Confidence score (0.0-1.0). |


---

#### PaddleOcrConfig

Configuration for PaddleOCR backend.

Configures PaddleOCR text detection and recognition with multi-language support.
Uses a builder pattern for convenient configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `language` | `String` | — | Language code (e.g., "en", "ch", "jpn", "kor", "deu", "fra") |
| `cacheDir` | `Path?` | `null` | Optional custom cache directory for model files |
| `useAngleCls` | `Boolean` | — | Enable angle classification for rotated text (default: false). Can misfire on short text regions, rotating crops incorrectly before recognition. |
| `enableTableDetection` | `Boolean` | — | Enable table structure detection (default: false) |
| `detDbThresh` | `Float` | — | Database threshold for text detection (default: 0.3) Range: 0.0-1.0, higher values require more confident detections |
| `detDbBoxThresh` | `Float` | — | Box threshold for text bounding box refinement (default: 0.5) Range: 0.0-1.0 |
| `detDbUnclipRatio` | `Float` | — | Unclip ratio for expanding text bounding boxes (default: 1.6) Controls the expansion of detected text regions |
| `detLimitSideLen` | `Int` | — | Maximum side length for detection image (default: 960) Larger images may be resized to this limit for faster inference |
| `recBatchNum` | `Int` | — | Batch size for recognition inference (default: 6) Number of text regions to process simultaneously |
| `padding` | `Int` | — | Padding in pixels added around the image before detection (default: 10). Large values can include surrounding content like table gridlines. |
| `dropScore` | `Float` | — | Minimum recognition confidence score for text lines (default: 0.5). Text regions with recognition confidence below this threshold are discarded. Matches PaddleOCR Python's `drop_score` parameter. Range: 0.0-1.0 |
| `modelTier` | `String` | — | Model tier controlling detection/recognition model size and accuracy trade-off. - `"mobile"` (default): Lightweight models (~4.5MB detection, ~16.5MB recognition), fast download and inference - `"server"`: Large, high-accuracy models (~88MB detection, ~84MB recognition), best for GPU or complex documents |

##### Methods

###### withCacheDir()

Sets a custom cache directory for model files.

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### withTableDetection()

Enables or disables table structure detection.

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### withAngleCls()

Enables or disables angle classification for rotated text.

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### withDetDbThresh()

Sets the database threshold for text detection.

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### withDetDbBoxThresh()

Sets the box threshold for text bounding box refinement.

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### withDetDbUnclipRatio()

Sets the unclip ratio for expanding text bounding boxes.

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### withDetLimitSideLen()

Sets the maximum side length for detection images.

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### withRecBatchNum()

Sets the batch size for recognition inference.

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### withDropScore()

Sets the minimum recognition confidence threshold.

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### withPadding()

Sets padding in pixels added around images before detection.

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### withModelTier()

Sets the model tier controlling detection/recognition model size.

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### default()

Creates a default configuration with English language support.

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```


---

#### PageBoundary

Byte offset boundary for a page.

Tracks where a specific page's content starts and ends in the main content string,
enabling mapping from byte positions to page numbers. Offsets are guaranteed to be
at valid UTF-8 character boundaries when using standard String methods (push_str, push, etc.).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `byteStart` | `Long` | — | Byte offset where this page starts in the content string (UTF-8 valid boundary, inclusive) |
| `byteEnd` | `Long` | — | Byte offset where this page ends in the content string (UTF-8 valid boundary, exclusive) |
| `pageNumber` | `Long` | — | Page number (1-indexed) |


---

#### PageConfig

Page extraction and tracking configuration.

Controls how pages are extracted, tracked, and represented in the extraction results.
When `null`, page tracking is disabled.

Page range tracking in chunk metadata (first_page/last_page) is automatically enabled
when page boundaries are available and chunking is configured.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `extractPages` | `Boolean` | `false` | Extract pages as separate array (ExtractionResult.pages) |
| `insertPageMarkers` | `Boolean` | `false` | Insert page markers in main content string |
| `markerFormat` | `String` | `"

<!-- PAGE {page_num} -->

"` | Page marker format (use {page_num} placeholder) Default: "\n\n<!-- PAGE {page_num} -->\n\n" |

##### Methods

###### default()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```


---

#### PageContent

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
| `pageNumber` | `Long` | — | Page number (1-indexed) |
| `content` | `String` | — | Text content for this page |
| `tables` | `List<String>` | — | Tables found on this page (uses Arc for memory efficiency) Serializes as Vec<Table> for JSON compatibility while maintaining Arc semantics in-memory for zero-copy sharing. |
| `images` | `List<ExtractedImage>` | — | Images found on this page (uses Arc for memory efficiency) Serializes as Vec<ExtractedImage> for JSON compatibility while maintaining Arc semantics in-memory for zero-copy sharing. |
| `hierarchy` | `PageHierarchy?` | `null` | Hierarchy information for the page (when hierarchy extraction is enabled) Contains text hierarchy levels (H1-H6) extracted from the page content. |
| `isBlank` | `Boolean?` | `null` | Whether this page is blank (no meaningful text content) Determined during extraction based on text content analysis. A page is blank if it has fewer than 3 non-whitespace characters and contains no tables or images. |
| `layoutRegions` | `List<LayoutRegion>?` | `null` | Layout detection regions for this page (when layout detection is enabled). Contains detected layout regions with class, confidence, bounding box, and area fraction. Only populated when layout detection is configured. |


---

#### PageHierarchy

Page hierarchy structure containing heading levels and block information.

Used when PDF text hierarchy extraction is enabled. Contains hierarchical
blocks with heading levels (H1-H6) for semantic document structure.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `blockCount` | `Long` | — | Number of hierarchy blocks on this page |
| `blocks` | `List<HierarchicalBlock>` | — | Hierarchical blocks with heading levels |


---

#### PageInfo

Metadata for individual page/slide/sheet.

Captures per-page information including dimensions, content counts,
and visibility state (for presentations).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `number` | `Long` | — | Page number (1-indexed) |
| `title` | `String?` | `null` | Page title (usually for presentations) |
| `dimensions` | `List<Double>?` | `null` | Dimensions in points (PDF) or pixels (images): (width, height) |
| `imageCount` | `Long?` | `null` | Number of images on this page |
| `tableCount` | `Long?` | `null` | Number of tables on this page |
| `hidden` | `Boolean?` | `null` | Whether this page is hidden (e.g., in presentations) |
| `isBlank` | `Boolean?` | `null` | Whether this page is blank (no meaningful text, no images, no tables) A page is considered blank if it has fewer than 3 non-whitespace characters and contains no tables or images. This is useful for filtering out empty pages in scanned documents or PDFs with blank separator pages. |


---

#### PageLayoutResult

Layout detection results for a single page.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `pageIndex` | `Long` | — | Page index |
| `regions` | `List<String>` | — | Regions |
| `pageWidthPts` | `Float` | — | Page width pts |
| `pageHeightPts` | `Float` | — | Page height pts |
| `renderWidthPx` | `Int` | — | Width of the rendered image used for layout detection (pixels). |
| `renderHeightPx` | `Int` | — | Height of the rendered image used for layout detection (pixels). |


---

#### PageMarginsPoints

Page margins converted to points (1/72 inch).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `top` | `Double?` | `null` | Top |
| `right` | `Double?` | `null` | Right |
| `bottom` | `Double?` | `null` | Bottom |
| `left` | `Double?` | `null` | Left |
| `header` | `Double?` | `null` | Header |
| `footer` | `Double?` | `null` | Footer |
| `gutter` | `Double?` | `null` | Gutter |


---

#### PageStructure

Unified page structure for documents.

Supports different page types (PDF pages, PPTX slides, Excel sheets)
with character offset boundaries for chunk-to-page mapping.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `totalCount` | `Long` | — | Total number of pages/slides/sheets |
| `unitType` | `PageUnitType` | — | Type of paginated unit |
| `boundaries` | `List<PageBoundary>?` | `null` | Character offset boundaries for each page Maps character ranges in the extracted content to page numbers. Used for chunk page range calculation. |
| `pages` | `List<PageInfo>?` | `null` | Detailed per-page metadata (optional, only when needed) |


---

#### PageTiming

Timing breakdown for a single page.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `renderMs` | `Double` | — | Time to render the PDF page to a raster image (amortized from batch render). |
| `preprocessMs` | `Double` | — | Time spent in image preprocessing (resize, normalize, tensor construction). |
| `onnxMs` | `Double` | — | Time for the ONNX model session.run() call (actual neural network inference). |
| `inferenceMs` | `Double` | — | Total model inference time (preprocess + onnx), as measured by the engine. |
| `postprocessMs` | `Double` | — | Time spent in postprocessing (confidence filtering, overlap resolution). |
| `mappingMs` | `Double` | — | Time to map pixel-space bounding boxes to PDF coordinate space. |


---

#### PdfAnnotation

A PDF annotation extracted from a document page.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `annotationType` | `PdfAnnotationType` | — | The type of annotation. |
| `content` | `String?` | `null` | Text content of the annotation (e.g., comment text, link URL). |
| `pageNumber` | `Long` | — | Page number where the annotation appears (1-indexed). |
| `boundingBox` | `String?` | `null` | Bounding box of the annotation on the page. |


---

#### PdfConfig

PDF-specific configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `backend` | `PdfBackend` | `PdfBackend.Pdfium` | PDF extraction backend. Default: `Pdfium`. |
| `extractImages` | `Boolean` | `false` | Extract images from PDF |
| `passwords` | `List<String>?` | `null` | List of passwords to try when opening encrypted PDFs |
| `extractMetadata` | `Boolean` | `true` | Extract PDF metadata |
| `hierarchy` | `HierarchyConfig?` | `null` | Hierarchy extraction configuration (None = hierarchy extraction disabled) |
| `extractAnnotations` | `Boolean` | `false` | Extract PDF annotations (text notes, highlights, links, stamps). Default: false |
| `topMarginFraction` | `Float?` | `null` | Top margin fraction (0.0–1.0) of page height to exclude headers/running heads. Default: 0.06 (6%) |
| `bottomMarginFraction` | `Float?` | `null` | Bottom margin fraction (0.0–1.0) of page height to exclude footers/page numbers. Default: 0.05 (5%) |
| `allowSingleColumnTables` | `Boolean` | `false` | Allow single-column pseudo tables in extraction results. By default, tables with fewer than 2 columns (layout-guided) or 3 columns (heuristic) are rejected. When `true`, the minimum column count is relaxed to 1, allowing single-column structured data (glossaries, itemized lists) to be emitted as tables. Other quality filters (density, sparsity, prose detection) still apply. |

##### Methods

###### default()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```


---

#### PdfImage

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `pageNumber` | `Long` | — | Page number |
| `imageIndex` | `Long` | — | Image index |
| `width` | `Long` | — | Width |
| `height` | `Long` | — | Height |
| `colorSpace` | `String?` | `null` | Color space |
| `bitsPerComponent` | `Long?` | `null` | Bits per component |
| `filters` | `List<String>` | — | Original PDF stream filters (e.g. `["FlateDecode"]`, `["DCTDecode"]`). |
| `data` | `ByteArray` | — | The decoded image bytes in a standard format (JPEG, PNG, etc.). |
| `decodedFormat` | `String` | — | The format of `data` after decoding: `"jpeg"`, `"png"`, `"jpeg2000"`, `"ccitt"`, or `"raw"`. |


---

#### PdfUnifiedExtractionResult

Result type for unified PDF text and metadata extraction.

Contains text, optional page boundaries, optional per-page content, and metadata.


---

#### Plugin

Base trait that all plugins must implement.

This trait provides common functionality for plugin lifecycle management,
identification, and metadata.

# Thread Safety

All plugins must be `Send + Sync` to support concurrent usage across threads.

##### Methods

###### name()

Returns the unique name/identifier for this plugin.

The name should be:
- Unique across all plugins
- Lowercase with hyphens (e.g., "my-custom-plugin")
- URL-safe characters only

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### version()

Returns the semantic version of this plugin.

Should follow semver format: `MAJOR.MINOR.PATCH`

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### initialize()

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

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### shutdown()

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

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### description()

Optional plugin description for debugging and logging.

Defaults to empty string if not overridden.

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### author()

Optional plugin author information.

Defaults to empty string if not overridden.

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
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

###### process()

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

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### processingStage()

Get the processing stage for this post-processor.

Determines when this processor runs in the pipeline.

**Returns:**

The `ProcessingStage` (Early, Middle, or Late).

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### shouldProcess()

Optional: Check if this processor should run for a given result.

Allows conditional processing based on MIME type, metadata, or content.
Defaults to `true` (always run).

**Returns:**

`true` if the processor should run, `false` to skip.

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### estimatedDurationMs()

Optional: Estimate processing time in milliseconds.

Used for logging and debugging. Defaults to 0 (unknown).

**Returns:**

Estimated processing time in milliseconds.

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```


---

#### PostProcessorConfig

Post-processor configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `Boolean` | `true` | Enable post-processors |
| `enabledProcessors` | `List<String>?` | `null` | Whitelist of processor names to run (None = all enabled) |
| `disabledProcessors` | `List<String>?` | `null` | Blacklist of processor names to skip (None = none disabled) |
| `enabledSet` | `String?` | `null` | Pre-computed AHashSet for O(1) enabled processor lookup |
| `disabledSet` | `String?` | `null` | Pre-computed AHashSet for O(1) disabled processor lookup |

##### Methods

###### default()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```


---

#### PptxAppProperties

Application properties from docProps/app.xml for PPTX

Contains PowerPoint-specific document metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `application` | `String?` | `null` | Application name (e.g., "Microsoft Office PowerPoint") |
| `appVersion` | `String?` | `null` | Application version |
| `totalTime` | `Int?` | `null` | Total editing time in minutes |
| `company` | `String?` | `null` | Company name |
| `docSecurity` | `Int?` | `null` | Document security level |
| `scaleCrop` | `Boolean?` | `null` | Scale crop flag |
| `linksUpToDate` | `Boolean?` | `null` | Links up to date flag |
| `sharedDoc` | `Boolean?` | `null` | Shared document flag |
| `hyperlinksChanged` | `Boolean?` | `null` | Hyperlinks changed flag |
| `slides` | `Int?` | `null` | Number of slides |
| `notes` | `Int?` | `null` | Number of notes |
| `hiddenSlides` | `Int?` | `null` | Number of hidden slides |
| `multimediaClips` | `Int?` | `null` | Number of multimedia clips |
| `presentationFormat` | `String?` | `null` | Presentation format (e.g., "Widescreen", "Standard") |
| `slideTitles` | `List<String>` | `[]` | Slide titles |


---

#### PptxExtractionResult

PowerPoint (PPTX) extraction result.

Contains extracted slide content, metadata, and embedded images/tables.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String` | — | Extracted text content from all slides |
| `metadata` | `PptxMetadata` | — | Presentation metadata |
| `slideCount` | `Long` | — | Total number of slides |
| `imageCount` | `Long` | — | Total number of embedded images |
| `tableCount` | `Long` | — | Total number of tables |
| `images` | `List<ExtractedImage>` | — | Extracted images from the presentation |
| `pageStructure` | `PageStructure?` | `null` | Slide structure with boundaries (when page tracking is enabled) |
| `pageContents` | `List<PageContent>?` | `null` | Per-slide content (when page tracking is enabled) |
| `document` | `DocumentStructure?` | `null` | Structured document representation |
| `hyperlinks` | `List<String>` | — | Hyperlinks discovered in slides as (url, optional_label) pairs. |
| `officeMetadata` | `Map<String, String>` | — | Office metadata extracted from docProps/core.xml and docProps/app.xml. Contains keys like "title", "author", "created_by", "subject", "keywords", "modified_by", "created_at", "modified_at", etc. |


---

#### PptxMetadata

PowerPoint presentation metadata.

Extracted from PPTX files containing slide counts and presentation details.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `slideCount` | `Long` | — | Total number of slides in the presentation |
| `slideNames` | `List<String>` | `[]` | Names of slides (if available) |
| `imageCount` | `Long?` | `null` | Number of embedded images |
| `tableCount` | `Long?` | `null` | Number of tables |


---

#### ProcessingWarning

A non-fatal warning from a processing pipeline stage.

Captures errors from optional features that don't prevent extraction
but may indicate degraded results.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `source` | `String` | — | The pipeline stage or feature that produced this warning (e.g., "embedding", "chunking", "language_detection", "output_format"). |
| `message` | `String` | — | Human-readable description of what went wrong. |


---

#### PstMetadata

Outlook PST archive metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `messageCount` | `Long` | — | Number of messages |


---

#### RakeParams

RAKE-specific parameters.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `minWordLength` | `Long` | `1` | Minimum word length to consider (default: 1). |
| `maxWordsPerPhrase` | `Long` | `3` | Maximum words in a keyword phrase (default: 3). |

##### Methods

###### default()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```


---

#### RecognizedTable

Pre-computed table markdown for a table detection region.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `detectionBbox` | `BBox` | — | Detection bbox that this table corresponds to (for matching). |
| `cells` | `List<List<String>>` | — | Table cells as a 2D vector (rows x columns). |
| `markdown` | `String` | — | Rendered markdown table. |


---

#### Recyclable

Trait for types that can be pooled and reused.

Implementing this trait allows a type to be used with `Pool<T>`.
The `reset()` method should clear the object's state for reuse.

##### Methods

###### reset()

Reset the object to a reusable state.

This is called when returning an object to the pool.
Should clear any internal data while preserving capacity.

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```


---

#### ResolvedStyle

Fully resolved (flattened) style after walking the inheritance chain.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `paragraphProperties` | `String` | — | Paragraph properties |
| `runProperties` | `String` | — | Run properties |


---

#### ServerConfig

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
| `host` | `String` | — | Server host address (e.g., "127.0.0.1", "0.0.0.0") |
| `port` | `Short` | — | Server port number |
| `corsOrigins` | `List<String>` | `[]` | CORS allowed origins. Empty vector means allow all origins. If this is an empty vector, the server will accept requests from any origin. If populated with specific origins (e.g., ["<https://example.com">]), only those origins will be allowed. |
| `maxRequestBodyBytes` | `Long` | — | Maximum size of request body in bytes (default: 100 MB) |
| `maxMultipartFieldBytes` | `Long` | — | Maximum size of multipart fields in bytes (default: 100 MB) |

##### Methods

###### default()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### listenAddr()

Get the server listen address (host:port).

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### corsAllowsAll()

Check if CORS allows all origins.

Returns `true` if the `cors_origins` vector is empty, meaning all origins
are allowed. Returns `false` if specific origins are configured.

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### isOriginAllowed()

Check if a given origin is allowed by CORS configuration.

Returns `true` if:
- CORS allows all origins (empty origins list), or
- The given origin is in the allowed origins list

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### maxRequestBodyMb()

Get maximum request body size in megabytes (rounded up).

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### maxMultipartFieldMb()

Get maximum multipart field size in megabytes (rounded up).

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```


---

#### StreamReader


---

#### StringBufferPool

Convenience type alias for a pooled String.


---

#### StructuredData

Structured data (Schema.org, microdata, RDFa) block.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `dataType` | `StructuredDataType` | — | Type of structured data |
| `rawJson` | `String` | — | Raw JSON string representation |
| `schemaType` | `String?` | `null` | Schema type if detectable (e.g., "Article", "Event", "Product") |


---

#### StructuredDataResult

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String` | — | The extracted text content |
| `format` | `String` | — | Format |
| `metadata` | `Map<String, String>` | — | Document metadata |
| `textFields` | `List<String>` | — | Text fields |


---

#### StructuredExtractionConfig

Configuration for LLM-based structured data extraction.

Sends extracted document content to a VLM with a JSON schema,
returning structured data that conforms to the schema.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `schema` | `Any` | — | JSON Schema defining the desired output structure. |
| `schemaName` | `String` | — | Schema name passed to the LLM's structured output mode. |
| `schemaDescription` | `String?` | `null` | Optional schema description for the LLM. |
| `strict` | `Boolean` | — | Enable strict mode — output must exactly match the schema. |
| `prompt` | `String?` | `null` | Custom Jinja2 extraction prompt template. When `null`, a default template is used. Available template variables: - `{{ content }}` — The extracted document text. - `{{ schema }}` — The JSON schema as a formatted string. - `{{ schema_name }}` — The schema name. - `{{ schema_description }}` — The schema description (may be empty). |
| `llm` | `LlmConfig` | — | LLM configuration for the extraction. |


---

#### StructuredExtractionResponse

Response from structured extraction endpoint.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `structuredOutput` | `Any` | — | Structured data conforming to the provided JSON schema |
| `content` | `String` | — | Extracted document text content |
| `mimeType` | `String` | — | Detected MIME type of the input file |


---

#### StyleDefinition

A single style definition parsed from `<w:style>` in `word/styles.xml`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String` | — | The style ID (`w:styleId` attribute). |
| `name` | `String?` | `null` | Human-readable name (`<w:name w:val="..."/>`). |
| `styleType` | `String` | — | Style type: paragraph, character, table, or numbering. |
| `basedOn` | `String?` | `null` | ID of the parent style (`<w:basedOn w:val="..."/>`). |
| `nextStyle` | `String?` | `null` | ID of the style to apply to the next paragraph (`<w:next w:val="..."/>`). |
| `isDefault` | `Boolean` | — | Whether this is the default style for its type. |
| `paragraphProperties` | `String` | — | Paragraph properties defined directly on this style. |
| `runProperties` | `String` | — | Run properties defined directly on this style. |


---

#### SupportedFormat

A supported document format entry.

Represents a file extension and its corresponding MIME type that Kreuzberg can process.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `extension` | `String` | — | File extension (without leading dot), e.g., "pdf", "docx" |
| `mimeType` | `String` | — | MIME type string, e.g., "application/pdf" |


---

#### SyncExtractor

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

###### extractSync()

Extract content from a byte array synchronously.

This method performs extraction without requiring an async runtime.
It is called by `extract_bytes_sync()` when the `tokio-runtime` feature is disabled.

**Returns:**

An `InternalDocument` containing the extracted elements, metadata, and tables.

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```


---

#### TableProperties

Table-level properties from `<w:tblPr>`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `styleId` | `String?` | `null` | Style id |
| `width` | `String?` | `null` | Width |
| `alignment` | `String?` | `null` | Alignment |
| `layout` | `String?` | `null` | Layout |
| `look` | `String?` | `null` | Look |
| `borders` | `String?` | `null` | Borders |
| `cellMargins` | `String?` | `null` | Cell margins |
| `indent` | `String?` | `null` | Indent |
| `caption` | `String?` | `null` | Caption |


---

#### TessdataManager

Manages tessdata file downloading, caching, and manifest generation.

##### Methods

###### cacheDir()

Get the cache directory path.

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### isLanguageCached()

Check if a specific language traineddata file is cached.

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### ensureAllLanguages()

Downloads all tessdata_fast traineddata files to the cache directory.

Skips files that already exist. Returns the count of newly downloaded files.

Requires the `paddle-ocr` feature for HTTP download support (ureq).

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```


---

#### TesseractConfig

Tesseract OCR configuration.

Provides fine-grained control over Tesseract OCR engine parameters.
Most users can use the defaults, but these settings allow optimization
for specific document types (invoices, handwriting, etc.).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `language` | `String` | `"eng"` | Language code (e.g., "eng", "deu", "fra") |
| `psm` | `Int` | `3` | Page Segmentation Mode (0-13). Common values: - 3: Fully automatic page segmentation (default) - 6: Assume a single uniform block of text - 11: Sparse text with no particular order |
| `outputFormat` | `String` | `"markdown"` | Output format ("text" or "markdown") |
| `oem` | `Int` | `3` | OCR Engine Mode (0-3). - 0: Legacy engine only - 1: Neural nets (LSTM) only (usually best) - 2: Legacy + LSTM - 3: Default (based on what's available) |
| `minConfidence` | `Double` | `0` | Minimum confidence threshold (0.0-100.0). Words with confidence below this threshold may be rejected or flagged. |
| `preprocessing` | `ImagePreprocessingConfig?` | `null` | Image preprocessing configuration. Controls how images are preprocessed before OCR. Can significantly improve quality for scanned documents or low-quality images. |
| `enableTableDetection` | `Boolean` | `true` | Enable automatic table detection and reconstruction |
| `tableMinConfidence` | `Double` | `0` | Minimum confidence threshold for table detection (0.0-1.0) |
| `tableColumnThreshold` | `Int` | `50` | Column threshold for table detection (pixels) |
| `tableRowThresholdRatio` | `Double` | `0.5` | Row threshold ratio for table detection (0.0-1.0) |
| `useCache` | `Boolean` | `true` | Enable OCR result caching |
| `classifyUsePreAdaptedTemplates` | `Boolean` | `true` | Use pre-adapted templates for character classification |
| `languageModelNgramOn` | `Boolean` | `false` | Enable N-gram language model |
| `tesseditDontBlkrejGoodWds` | `Boolean` | `true` | Don't reject good words during block-level processing |
| `tesseditDontRowrejGoodWds` | `Boolean` | `true` | Don't reject good words during row-level processing |
| `tesseditEnableDictCorrection` | `Boolean` | `true` | Enable dictionary correction |
| `tesseditCharWhitelist` | `String` | `""` | Whitelist of allowed characters (empty = all allowed) |
| `tesseditCharBlacklist` | `String` | `""` | Blacklist of forbidden characters (empty = none forbidden) |
| `tesseditUsePrimaryParamsModel` | `Boolean` | `true` | Use primary language params model |
| `textordSpaceSizeIsVariable` | `Boolean` | `true` | Variable-width space detection |
| `thresholdingMethod` | `Boolean` | `false` | Use adaptive thresholding method |

##### Methods

###### default()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```


---

#### TextAnnotation

Inline text annotation — byte-range based formatting and links.

Annotations reference byte offsets into the node's text content,
enabling precise identification of formatted regions.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `start` | `Int` | — | Start byte offset in the node's text content (inclusive). |
| `end` | `Int` | — | End byte offset in the node's text content (exclusive). |
| `kind` | `AnnotationKind` | — | Annotation type. |


---

#### TextExtractionResult

Plain text and Markdown extraction result.

Contains the extracted text along with statistics and,
for Markdown files, structural elements like headers and links.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String` | — | Extracted text content |
| `lineCount` | `Long` | — | Number of lines |
| `wordCount` | `Long` | — | Number of words |
| `characterCount` | `Long` | — | Number of characters |
| `headers` | `List<String>?` | `null` | Markdown headers (text only, Markdown files only) |
| `links` | `List<String>?` | `null` | Markdown links as (text, URL) tuples (Markdown files only) |
| `codeBlocks` | `List<String>?` | `null` | Code blocks as (language, code) tuples (Markdown files only) |


---

#### TextMetadata

Text/Markdown metadata.

Extracted from plain text and Markdown files. Includes word counts and,
for Markdown, structural elements like headers and links.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `lineCount` | `Long` | — | Number of lines in the document |
| `wordCount` | `Long` | — | Number of words |
| `characterCount` | `Long` | — | Number of characters |
| `headers` | `List<String>?` | `[]` | Markdown headers (headings text only, for Markdown files) |
| `links` | `List<String>?` | `[]` | Markdown links as (text, url) tuples (for Markdown files) |
| `codeBlocks` | `List<String>?` | `[]` | Code blocks as (language, code) tuples (for Markdown files) |


---

#### TokenReductionConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `level` | `ReductionLevel` | `ReductionLevel.Moderate` | Level (reduction level) |
| `languageHint` | `String?` | `null` | Language hint |
| `preserveMarkdown` | `Boolean` | `false` | Preserve markdown |
| `preserveCode` | `Boolean` | `true` | Preserve code |
| `semanticThreshold` | `Float` | `0.3` | Semantic threshold |
| `enableParallel` | `Boolean` | `true` | Enable parallel |
| `useSimd` | `Boolean` | `true` | Use simd |
| `customStopwords` | `Map<String, List<String>>?` | `null` | Custom stopwords |
| `preservePatterns` | `List<String>` | `[]` | Preserve patterns |
| `targetReduction` | `Float?` | `null` | Target reduction |
| `enableSemanticClustering` | `Boolean` | `false` | Enable semantic clustering |

##### Methods

###### default()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```


---

#### TokenReductionOptions

Token reduction configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `mode` | `String` | — | Reduction mode: "off", "light", "moderate", "aggressive", "maximum" |
| `preserveImportantWords` | `Boolean` | — | Preserve important words (capitalized, technical terms) |


---

#### TracingLayer

A `tower.Layer` that wraps each extraction in a semantic tracing span.


---

#### TreeSitterConfig

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
| `enabled` | `Boolean` | `true` | Enable code intelligence processing (default: true). When `false`, tree-sitter analysis is completely skipped even if the config section is present. |
| `cacheDir` | `Path?` | `null` | Custom cache directory for downloaded grammars. When `null`, uses the default: `~/.cache/tree-sitter-language-pack/v{version}/libs/`. |
| `languages` | `List<String>?` | `null` | Languages to pre-download on init (e.g., `["python", "rust"]`). |
| `groups` | `List<String>?` | `null` | Language groups to pre-download (e.g., `["web", "systems", "scripting"]`). |
| `process` | `TreeSitterProcessConfig` | — | Processing options for code analysis. |

##### Methods

###### default()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```


---

#### TreeSitterProcessConfig

Processing options for tree-sitter code analysis.

Controls which analysis features are enabled when extracting code files.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `structure` | `Boolean` | `true` | Extract structural items (functions, classes, structs, etc.). Default: true. |
| `imports` | `Boolean` | `true` | Extract import statements. Default: true. |
| `exports` | `Boolean` | `true` | Extract export statements. Default: true. |
| `comments` | `Boolean` | `false` | Extract comments. Default: false. |
| `docstrings` | `Boolean` | `false` | Extract docstrings. Default: false. |
| `symbols` | `Boolean` | `false` | Extract symbol definitions. Default: false. |
| `diagnostics` | `Boolean` | `false` | Include parse diagnostics. Default: false. |
| `chunkMaxSize` | `Long?` | `null` | Maximum chunk size in bytes. `null` disables chunking. |
| `contentMode` | `CodeContentMode` | `CodeContentMode.Chunks` | Content rendering mode for code extraction. |

##### Methods

###### default()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```


---

#### Uri

A URI extracted from a document.

Represents any link, reference, or resource pointer found during extraction.
The `kind` field classifies the URI semantically, while `label` carries
optional human-readable display text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | `String` | — | The URL or path string. |
| `label` | `String?` | `null` | Optional display text / label for the link. |
| `page` | `Int?` | `null` | Optional page number where the URI was found (1-indexed). |
| `kind` | `UriKind` | — | Semantic classification of the URI. |


---

#### Validator

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

###### validate()

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

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### shouldValidate()

Optional: Check if this validator should run for a given result.

Allows conditional validation based on MIME type, metadata, or content.
Defaults to `true` (always run).

**Returns:**

`true` if the validator should run, `false` to skip.

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```

###### priority()

Optional: Get the validation priority.

Higher priority validators run first. Useful for ordering validation checks
(e.g., run cheap validations before expensive ones).

Default priority is 50.

**Returns:**

Priority value (higher = runs earlier).

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```


---

#### VersionResponse

Version response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `version` | `String` | — | Kreuzberg version string |


---

#### WarmRequest

Cache warm request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `allEmbeddings` | `Boolean` | — | Download all embedding model presets |
| `embeddingModel` | `String?` | `null` | Specific embedding model preset to download |


---

#### WarmResponse

Cache warm response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `cacheDir` | `String` | — | Cache directory used |
| `downloaded` | `List<String>` | — | Models that were downloaded |
| `alreadyCached` | `List<String>` | — | Models that were already cached |


---

#### XlsxAppProperties

Application properties from docProps/app.xml for XLSX

Contains Excel-specific document metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `application` | `String?` | `null` | Application name (e.g., "Microsoft Excel") |
| `appVersion` | `String?` | `null` | Application version |
| `docSecurity` | `Int?` | `null` | Document security level |
| `scaleCrop` | `Boolean?` | `null` | Scale crop flag |
| `linksUpToDate` | `Boolean?` | `null` | Links up to date flag |
| `sharedDoc` | `Boolean?` | `null` | Shared document flag |
| `hyperlinksChanged` | `Boolean?` | `null` | Hyperlinks changed flag |
| `company` | `String?` | `null` | Company name |
| `worksheetNames` | `List<String>` | `[]` | Worksheet names |


---

#### XmlExtractionResult

XML extraction result.

Contains extracted text content from XML files along with
structural statistics about the XML document.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String` | — | Extracted text content (XML structure filtered out) |
| `elementCount` | `Long` | — | Total number of XML elements processed |
| `uniqueElements` | `List<String>` | — | List of unique element names found (sorted) |


---

#### XmlMetadata

XML metadata extracted during XML parsing.

Provides statistics about XML document structure.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `elementCount` | `Long` | — | Total number of XML elements processed |
| `uniqueElements` | `List<String>` | `[]` | List of unique element tag names (sorted) |


---

#### YakeParams

YAKE-specific parameters.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `windowSize` | `Long` | `2` | Window size for co-occurrence analysis (default: 2). Controls the context window for computing co-occurrence statistics. |

##### Methods

###### default()

**Signature:**

```kotlin
// Phase 1: kotlin backend method signature generation
```


---

#### YearRange

Year range for bibliographic metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `min` | `Int?` | `null` | Min |
| `max` | `Int?` | `null` | Max |
| `years` | `List<Int>` | — | Years |


---

#### ZipBombValidator

Helper struct for validating ZIP archives for security issues.


---

### Enums

#### ExecutionProviderType

ONNX Runtime execution provider type.

Determines which hardware backend is used for model inference.
`Auto` (default) selects the best available provider per platform.

| Value | Description |
|-------|-------------|
| `Auto` | Auto-select: CoreML on macOS, CUDA on Linux, CPU elsewhere. |
| `Cpu` | CPU execution provider (always available). |
| `CoreMl` | Apple CoreML (macOS/iOS Neural Engine + GPU). |
| `Cuda` | NVIDIA CUDA GPU acceleration. |
| `TensorRt` | NVIDIA TensorRT (optimized CUDA inference). |


---

#### HtmlTheme

Built-in HTML theme selection.

| Value | Description |
|-------|-------------|
| `Default` | Sensible defaults: system font stack, neutral colours, readable line measure. CSS custom properties (`--kb-*`) are all defined so user CSS can override individual values. |
| `GitHub` | GitHub Markdown-inspired palette and spacing. |
| `Dark` | Dark background, light text. |
| `Light` | Minimal light theme with generous whitespace. |
| `Unstyled` | No built-in stylesheet emitted. CSS custom properties are still defined on `:root` so user stylesheets can reference `var(--kb-*)` tokens. |


---

#### TableModel

Which table structure recognition model to use.

Controls the model used for table cell detection within layout-detected
table regions.

| Value | Description |
|-------|-------------|
| `Tatr` | TATR (Table Transformer) -- default, 30MB, DETR-based row/column detection. |
| `SlanetWired` | SLANeXT wired variant -- 365MB, optimized for bordered tables. |
| `SlanetWireless` | SLANeXT wireless variant -- 365MB, optimized for borderless tables. |
| `SlanetPlus` | SLANet-plus -- 7.78MB, lightweight general-purpose. |
| `SlanetAuto` | Classifier-routed SLANeXT: auto-select wired/wireless per table. Uses PP-LCNet classifier (6.78MB) + both SLANeXT variants (730MB total). |
| `Disabled` | Disable table structure model inference entirely; use heuristic path only. |


---

#### PdfBackend

PDF extraction backend selection.

Controls which PDF library is used for text extraction:
- `Pdfium`: pdfium-render (default, C++ based, mature)
- `PdfOxide`: pdf_oxide (pure Rust, faster, requires `pdf-oxide` feature)
- `Auto`: automatically select based on available features

| Value | Description |
|-------|-------------|
| `Pdfium` | Use pdfium-render backend (default). |
| `PdfOxide` | Use pdf_oxide backend (pure Rust). Requires `pdf-oxide` feature. |
| `Auto` | Automatically select the best available backend. |


---

#### ChunkerType

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
| `Text` | Text format |
| `Markdown` | Markdown format |
| `Yaml` | Yaml format |
| `Semantic` | Semantic |


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
| `Characters` | Size measured in Unicode characters (default). |
| `Tokenizer` | Size measured in tokens from a HuggingFace tokenizer. — Fields: `model`: `String`, `cacheDir`: `Path` |


---

#### EmbeddingModelType

Embedding model types supported by Kreuzberg.

| Value | Description |
|-------|-------------|
| `Preset` | Use a preset model configuration (recommended) — Fields: `name`: `String` |
| `Custom` | Use a custom ONNX model from HuggingFace — Fields: `modelId`: `String`, `dimensions`: `Long` |
| `Llm` | Provider-hosted embedding model via liter-llm. Uses the model specified in the nested `LlmConfig` (e.g., `"openai/text-embedding-3-small"`). — Fields: `llm`: `LlmConfig` |
| `Plugin` | In-process embedding backend registered via the plugin system. The caller registers an `EmbeddingBackend` once (e.g. a wrapper around an already-loaded `llama-cpp-python`, `sentence-transformers`, or tuned ONNX model), then references it by name in config. Kreuzberg calls back into the registered backend during chunking and standalone embed requests — no HuggingFace download, no ONNX Runtime requirement, no HTTP sidecar. When this variant is selected, only the following `EmbeddingConfig` fields apply: `normalize` (post-call L2 normalization) and `max_embed_duration_secs` (dispatcher timeout). Model-loading fields (`batch_size`, `cache_dir`, `show_download_progress`, `acceleration`) are ignored — the host owns the model lifecycle. Semantic chunking falls back to `ChunkingConfig.max_characters` when this variant is used, since there is no preset to look a chunk-size ceiling up against — size your context window via `max_characters` directly. See `crate.plugins.register_embedding_backend`. — Fields: `name`: `String` |


---

#### CodeContentMode

Content rendering mode for code extraction.

Controls how extracted code content is represented in the `content` field
of `ExtractionResult`.

| Value | Description |
|-------|-------------|
| `Chunks` | Use TSLP semantic chunks as content (default). |
| `Raw` | Use raw source code as content. |
| `Structure` | Emit function/class headings + docstrings (no code bodies). |


---

#### FracType

| Value | Description |
|-------|-------------|
| `Bar` | Bar |
| `NoBar` | No bar |
| `Linear` | Linear |
| `Skewed` | Skewed |


---

#### OcrBackendType

OCR backend types.

| Value | Description |
|-------|-------------|
| `Tesseract` | Tesseract OCR (native Rust binding) |
| `EasyOcr` | EasyOCR (Python-based, via FFI) |
| `PaddleOcr` | PaddleOCR (Python-based, via FFI) |
| `Custom` | Custom/third-party OCR backend |


---

#### ProcessingStage

Processing stages for post-processors.

Post-processors are executed in stage order (Early → Middle → Late).
Use stages to control the order of post-processing operations.

| Value | Description |
|-------|-------------|
| `Early` | Early stage - foundational processing. Use for: - Language detection - Character encoding normalization - Entity extraction (NER) - Text quality scoring |
| `Middle` | Middle stage - content transformation. Use for: - Keyword extraction - Token reduction - Text summarization - Semantic analysis |
| `Late` | Late stage - final enrichment. Use for: - Custom user hooks - Analytics/logging - Final validation - Output formatting |


---

#### ReductionLevel

| Value | Description |
|-------|-------------|
| `Off` | Off |
| `Light` | Light |
| `Moderate` | Moderate |
| `Aggressive` | Aggressive |
| `Maximum` | Maximum |


---

#### PdfAnnotationType

Type of PDF annotation.

| Value | Description |
|-------|-------------|
| `Text` | Sticky note / text annotation |
| `Highlight` | Highlighted text region |
| `Link` | Hyperlink annotation |
| `Stamp` | Rubber stamp annotation |
| `Underline` | Underline text markup |
| `StrikeOut` | Strikeout text markup |
| `Other` | Any other annotation type |


---

#### BlockType

Types of block-level elements in Djot.

| Value | Description |
|-------|-------------|
| `Paragraph` | Paragraph element |
| `Heading` | Heading element |
| `Blockquote` | Blockquote element |
| `CodeBlock` | Code block |
| `ListItem` | List item |
| `OrderedList` | Ordered list |
| `BulletList` | Bullet list |
| `TaskList` | Task list |
| `DefinitionList` | Definition list |
| `DefinitionTerm` | Definition term |
| `DefinitionDescription` | Definition description |
| `Div` | Div |
| `Section` | Section element |
| `ThematicBreak` | Thematic break |
| `RawBlock` | Raw block |
| `MathDisplay` | Math display |


---

#### InlineType

Types of inline elements in Djot.

| Value | Description |
|-------|-------------|
| `Text` | Text format |
| `Strong` | Strong |
| `Emphasis` | Emphasis |
| `Highlight` | Highlight |
| `Subscript` | Subscript |
| `Superscript` | Superscript |
| `Insert` | Insert |
| `Delete` | Delete |
| `Code` | Code |
| `Link` | Link |
| `Image` | Image element |
| `Span` | Span |
| `Math` | Math |
| `RawInline` | Raw inline |
| `FootnoteRef` | Footnote ref |
| `Symbol` | Symbol |


---

#### RelationshipKind

Semantic kind of a relationship between document elements.

| Value | Description |
|-------|-------------|
| `FootnoteReference` | Footnote marker -> footnote definition. |
| `CitationReference` | Citation marker -> bibliography entry. |
| `InternalLink` | Internal anchor link (`#id`) -> target heading/element. |
| `Caption` | Caption paragraph -> figure/table it describes. |
| `Label` | Label -> labeled element (HTML `<label for>`, LaTeX `\label{}`). |
| `TocEntry` | TOC entry -> target section. |
| `CrossReference` | Cross-reference (LaTeX `\ref{}`, DOCX cross-reference field). |


---

#### ContentLayer

Content layer classification for document nodes.

Replaces separate body/furniture arrays with per-node granularity.

| Value | Description |
|-------|-------------|
| `Body` | Main document body content. |
| `Header` | Page/section header (running header). |
| `Footer` | Page/section footer (running footer). |
| `Footnote` | Footnote content. |


---

#### NodeContent

Tagged enum for node content. Each variant carries only type-specific data.

Uses `#[serde(tag = "node_type")]` to avoid "type" keyword collision in
Go/Java/TypeScript bindings.

| Value | Description |
|-------|-------------|
| `Title` | Document title. — Fields: `text`: `String` |
| `Heading` | Section heading with level (1-6). — Fields: `level`: `Byte`, `text`: `String` |
| `Paragraph` | Body text paragraph. — Fields: `text`: `String` |
| `List` | List container — children are `ListItem` nodes. — Fields: `ordered`: `Boolean` |
| `ListItem` | Individual list item. — Fields: `text`: `String` |
| `Table` | Table with structured cell grid. — Fields: `grid`: `String` |
| `Image` | Image reference. — Fields: `description`: `String`, `imageIndex`: `Int`, `src`: `String` |
| `Code` | Code block. — Fields: `text`: `String`, `language`: `String` |
| `Quote` | Block quote — container, children carry the quoted content. |
| `Formula` | Mathematical formula / equation. — Fields: `text`: `String` |
| `Footnote` | Footnote reference content. — Fields: `text`: `String` |
| `Group` | Logical grouping container (section, key-value area). `heading_level` + `heading_text` capture the section heading directly rather than relying on a first-child positional convention. — Fields: `label`: `String`, `headingLevel`: `Byte`, `headingText`: `String` |
| `PageBreak` | Page break marker. |
| `Slide` | Presentation slide container — children are the slide's content nodes. — Fields: `number`: `Int`, `title`: `String` |
| `DefinitionList` | Definition list container — children are `DefinitionItem` nodes. |
| `DefinitionItem` | Individual definition list entry with term and definition. — Fields: `term`: `String`, `definition`: `String` |
| `Citation` | Citation or bibliographic reference. — Fields: `key`: `String`, `text`: `String` |
| `Admonition` | Admonition / callout container (note, warning, tip, etc.). Children carry the admonition body content. — Fields: `kind`: `String`, `title`: `String` |
| `RawBlock` | Raw block preserved verbatim from the source format. Used for content that cannot be mapped to a semantic node type (e.g. JSX in MDX, raw LaTeX in markdown, embedded HTML). — Fields: `format`: `String`, `content`: `String` |
| `MetadataBlock` | Structured metadata block (email headers, YAML frontmatter, etc.). — Fields: `entries`: `List<String>` |


---

#### AnnotationKind

Types of inline text annotations.

| Value | Description |
|-------|-------------|
| `Bold` | Bold |
| `Italic` | Italic |
| `Underline` | Underline |
| `Strikethrough` | Strikethrough |
| `Code` | Code |
| `Subscript` | Subscript |
| `Superscript` | Superscript |
| `Link` | Link — Fields: `url`: `String`, `title`: `String` |
| `Highlight` | Highlighted text (PDF highlights, HTML `<mark>`). |
| `Color` | Text color (CSS-compatible value, e.g. "#ff0000", "red"). — Fields: `value`: `String` |
| `FontSize` | Font size with units (e.g. "12pt", "1.2em", "16px"). — Fields: `value`: `String` |
| `Custom` | Extensible annotation for format-specific styling. — Fields: `name`: `String`, `value`: `String` |


---

#### ExtractionMethod

How the extracted text was produced.

| Value | Description |
|-------|-------------|
| `Native` | Native |
| `Ocr` | Ocr |
| `Mixed` | Mixed |


---

#### ChunkType

Semantic structural classification of a text chunk.

Assigned by the heuristic classifier in `chunking.classifier`.
Defaults to `Unknown` when no rule matches.
Designed to be extended in future versions without breaking changes.

| Value | Description |
|-------|-------------|
| `Heading` | Section heading or document title. |
| `PartyList` | Party list: names, addresses, and signatories. |
| `Definitions` | Definition clause ("X means…", "X shall mean…"). |
| `OperativeClause` | Operative clause containing legal/contractual action verbs. |
| `SignatureBlock` | Signature block with signatures, names, and dates. |
| `Schedule` | Schedule, annex, appendix, or exhibit section. |
| `TableLike` | Table-like content with aligned columns or repeated patterns. |
| `Formula` | Mathematical formula or equation. |
| `CodeBlock` | Code block or preformatted content. |
| `Image` | Embedded or referenced image content. |
| `OrgChart` | Organizational chart or hierarchy diagram. |
| `Diagram` | Diagram, figure, or visual illustration. |
| `Unknown` | Unclassified or mixed content. |


---

#### ElementType

Semantic element type classification.

Categorizes text content into semantic units for downstream processing.
Supports the element types commonly found in Unstructured documents.

| Value | Description |
|-------|-------------|
| `Title` | Document title |
| `NarrativeText` | Main narrative text body |
| `Heading` | Section heading |
| `ListItem` | List item (bullet, numbered, etc.) |
| `Table` | Table element |
| `Image` | Image element |
| `PageBreak` | Page break marker |
| `CodeBlock` | Code block |
| `BlockQuote` | Block quote |
| `Footer` | Footer text |
| `Header` | Header text |


---

#### FormatMetadata

Format-specific metadata (discriminated union).

Only one format type can exist per extraction result. This provides
type-safe, clean metadata without nested optionals.

| Value | Description |
|-------|-------------|
| `Pdf` | Pdf format — Fields: `0`: `String` |
| `Docx` | Docx format — Fields: `0`: `DocxMetadata` |
| `Excel` | Excel — Fields: `0`: `ExcelMetadata` |
| `Email` | Email — Fields: `0`: `EmailMetadata` |
| `Pptx` | Pptx format — Fields: `0`: `PptxMetadata` |
| `Archive` | Archive — Fields: `0`: `ArchiveMetadata` |
| `Image` | Image element — Fields: `0`: `String` |
| `Xml` | Xml format — Fields: `0`: `XmlMetadata` |
| `Text` | Text format — Fields: `0`: `TextMetadata` |
| `Html` | Preserve as HTML `<mark>` tags — Fields: `0`: `HtmlMetadata` |
| `Ocr` | Ocr — Fields: `0`: `OcrMetadata` |
| `Csv` | Csv format — Fields: `0`: `CsvMetadata` |
| `Bibtex` | Bibtex — Fields: `0`: `BibtexMetadata` |
| `Citation` | Citation — Fields: `0`: `CitationMetadata` |
| `FictionBook` | Fiction book — Fields: `0`: `FictionBookMetadata` |
| `Dbf` | Dbf — Fields: `0`: `DbfMetadata` |
| `Jats` | Jats — Fields: `0`: `JatsMetadata` |
| `Epub` | Epub format — Fields: `0`: `EpubMetadata` |
| `Pst` | Pst — Fields: `0`: `PstMetadata` |
| `Code` | Code — Fields: `0`: `String` |


---

#### TextDirection

Text direction enumeration for HTML documents.

| Value | Description |
|-------|-------------|
| `LeftToRight` | Left-to-right text direction |
| `RightToLeft` | Right-to-left text direction |
| `Auto` | Automatic text direction detection |


---

#### LinkType

Link type classification.

| Value | Description |
|-------|-------------|
| `Anchor` | Anchor link (#section) |
| `Internal` | Internal link (same domain) |
| `External` | External link (different domain) |
| `Email` | Email link (mailto:) |
| `Phone` | Phone link (tel:) |
| `Other` | Other link type |


---

#### ImageType

Image type classification.

| Value | Description |
|-------|-------------|
| `DataUri` | Data URI image |
| `InlineSvg` | Inline SVG |
| `External` | External image URL |
| `Relative` | Relative path image |


---

#### StructuredDataType

Structured data type classification.

| Value | Description |
|-------|-------------|
| `JsonLd` | JSON-LD structured data |
| `Microdata` | Microdata |
| `RDFa` | RDFa |


---

#### OcrBoundingGeometry

Bounding geometry for an OCR element.

Supports both axis-aligned rectangles (from Tesseract) and 4-point quadrilaterals
(from PaddleOCR and rotated text detection).

| Value | Description |
|-------|-------------|
| `Rectangle` | Axis-aligned bounding box (typical for Tesseract output). — Fields: `left`: `Int`, `top`: `Int`, `width`: `Int`, `height`: `Int` |
| `Quadrilateral` | 4-point quadrilateral for rotated/skewed text (PaddleOCR). Points are in clockwise order starting from top-left: `[top_left, top_right, bottom_right, bottom_left]` — Fields: `points`: `String` |


---

#### OcrElementLevel

Hierarchical level of an OCR element.

Maps to Tesseract's page segmentation hierarchy and provides
equivalent semantics for PaddleOCR.

| Value | Description |
|-------|-------------|
| `Word` | Individual word |
| `Line` | Line of text (default for PaddleOCR) |
| `Block` | Paragraph or text block |
| `Page` | Page-level element |


---

#### PageUnitType

Type of paginated unit in a document.

Distinguishes between different types of "pages" (PDF pages, presentation slides, spreadsheet sheets).

| Value | Description |
|-------|-------------|
| `Page` | Standard document pages (PDF, DOCX, images) |
| `Slide` | Presentation slides (PPTX, ODP) |
| `Sheet` | Spreadsheet sheets (XLSX, ODS) |


---

#### UriKind

Semantic classification of an extracted URI.

| Value | Description |
|-------|-------------|
| `Hyperlink` | A clickable hyperlink (web URL, file link). |
| `Image` | An image or media resource reference. |
| `Anchor` | An internal anchor or cross-reference target. |
| `Citation` | A citation or bibliographic reference (DOI, academic ref). |
| `Reference` | A general reference (e.g. `\ref{}` in LaTeX, `:ref:` in RST). |
| `Email` | An email address (`mailto:` link or bare email). |


---

#### PoolError

Error type for pool operations.

| Value | Description |
|-------|-------------|
| `LockPoisoned` | The pool's internal mutex was poisoned. This indicates a panic occurred while holding the lock. The pool is in a locked state and cannot be recovered. |


---

#### KeywordAlgorithm

Keyword algorithm selection.

| Value | Description |
|-------|-------------|
| `Yake` | YAKE (Yet Another Keyword Extractor) - statistical approach |
| `Rake` | RAKE (Rapid Automatic Keyword Extraction) - co-occurrence based |


---

#### PsmMode

Page Segmentation Mode for Tesseract OCR

| Value | Description |
|-------|-------------|
| `OsdOnly` | Osd only |
| `AutoOsd` | Auto osd |
| `AutoOnly` | Auto only |
| `Auto` | Auto |
| `SingleColumn` | Single column |
| `SingleBlockVertical` | Single block vertical |
| `SingleBlock` | Single block |
| `SingleLine` | Single line |
| `SingleWord` | Single word |
| `CircleWord` | Circle word |
| `SingleChar` | Single char |


---

#### PaddleLanguage

Supported languages in PaddleOCR.

Maps user-friendly language codes to paddle-ocr-rs language identifiers.

| Value | Description |
|-------|-------------|
| `English` | English |
| `Chinese` | Simplified Chinese |
| `Japanese` | Japanese |
| `Korean` | Korean |
| `German` | German |
| `French` | French |
| `Latin` | Latin script (covers most European languages) |
| `Cyrillic` | Cyrillic (Russian and related) |
| `TraditionalChinese` | Traditional Chinese |
| `Thai` | Thai |
| `Greek` | Greek |
| `EastSlavic` | East Slavic (Russian, Ukrainian, Belarusian) |
| `Arabic` | Arabic (Arabic, Persian, Urdu) |
| `Devanagari` | Devanagari (Hindi, Marathi, Sanskrit, Nepali) |
| `Tamil` | Tamil |
| `Telugu` | Telugu |


---

#### LayoutClass

The 17 canonical document layout classes.

All model backends (RT-DETR, YOLO, etc.) map their native class IDs
to this shared set. Models with fewer classes (DocLayNet: 11, PubLayNet: 5)
map to the closest equivalent.

| Value | Description |
|-------|-------------|
| `Caption` | Caption element |
| `Footnote` | Footnote element |
| `Formula` | Formula |
| `ListItem` | List item |
| `PageFooter` | Page footer |
| `PageHeader` | Page header |
| `Picture` | Picture |
| `SectionHeader` | Section header |
| `Table` | Table element |
| `Text` | Text format |
| `Title` | Title element |
| `DocumentIndex` | Document index |
| `Code` | Code |
| `CheckboxSelected` | Checkbox selected |
| `CheckboxUnselected` | Checkbox unselected |
| `Form` | Form |
| `KeyValueRegion` | Key value region |


---

### Errors

#### KreuzbergError

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
| `Io` | IO error: {0} |
| `Parsing` | Parsing error: {message} |
| `Ocr` | OCR error: {message} |
| `Validation` | Validation error: {message} |
| `Cache` | Cache error: {message} |
| `ImageProcessing` | Image processing error: {message} |
| `Serialization` | Serialization error: {message} |
| `MissingDependency` | Missing dependency: {0} |
| `Plugin` | Plugin error in '{plugin_name}': {message} |
| `LockPoisoned` | Lock poisoned: {0} |
| `UnsupportedFormat` | Unsupported format: {0} |
| `Embedding` | Embedding error: {message} |
| `Timeout` | Extraction timed out after {elapsed_ms}ms (limit: {limit_ms}ms) |
| `Cancelled` | Extraction cancelled |
| `Security` | Security violation: {message} |
| `Other` | {0} |


---

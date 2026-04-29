---
title: "Go API Reference"
---

## Go API Reference <span class="version-badge">v4.10.0-rc.8</span>

### Functions

#### Blake3HashBytes()

Hash arbitrary bytes with blake3, returning a 32-char hex string.

**Signature:**

```go
func Blake3HashBytes(data []byte) string
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Data` | `[]byte` | Yes | The data |

**Returns:** `string`


---

#### Blake3HashFile()

Hash a file's content with blake3 using streaming 64 KiB reads.

Returns a 32-char hex string (128 bits of blake3 output).

**Signature:**

```go
func Blake3HashFile(path string) (string, error)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Path` | `string` | Yes | Path to the file |

**Returns:** `string`

**Errors:** Returns `error`.


---

#### FastHash()

**Signature:**

```go
func FastHash(data []byte) uint64
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Data` | `[]byte` | Yes | The data |

**Returns:** `uint64`


---

#### ValidateCacheKey()

**Signature:**

```go
func ValidateCacheKey(key string) bool
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Key` | `string` | Yes | The key |

**Returns:** `bool`


---

#### ValidatePort()

Validate a port number for server configuration.

Port must be in the range 1-65535. While ports 1-1023 are privileged and may require
special permissions on some systems, they are still valid port numbers.

**Returns:**

`Ok(())` if the port is valid, or a `ValidationError` with details about valid ranges.

**Signature:**

```go
func ValidatePort(port uint16) error
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Port` | `uint16` | Yes | The port number to validate |

**Returns:** ``

**Errors:** Returns `error`.


---

#### ValidateHost()

Validate a host/IP address string for server configuration.

Accepts valid IPv4 addresses (e.g., "127.0.0.1", "0.0.0.0"), valid IPv6 addresses
(e.g., ".1", "."), and hostnames (e.g., "localhost", "example.com").

**Returns:**

`Ok(())` if the host is valid, or a `ValidationError` with details about valid formats.

**Signature:**

```go
func ValidateHost(host string) error
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Host` | `string` | Yes | The host/IP address string to validate |

**Returns:** ``

**Errors:** Returns `error`.


---

#### ValidateCorsOrigin()

Validate a CORS (Cross-Origin Resource Sharing) origin URL.

Accepts valid HTTP/HTTPS URLs (e.g., "<https://example.com">) or the wildcard "*"
to allow all origins. URLs must start with "<http://"> or "<https://",> or be exactly "*".

**Returns:**

`Ok(())` if the origin is valid, or a `ValidationError` with details about valid formats.

**Signature:**

```go
func ValidateCorsOrigin(origin string) error
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Origin` | `string` | Yes | The CORS origin URL to validate |

**Returns:** ``

**Errors:** Returns `error`.


---

#### ValidateUploadSize()

Validate an upload size limit for server configuration.

Upload size must be greater than 0 (measured in bytes).

**Returns:**

`Ok(())` if the size is valid, or a `ValidationError` with details about constraints.

**Signature:**

```go
func ValidateUploadSize(size int) error
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Size` | `int` | Yes | The maximum upload size in bytes to validate |

**Returns:** ``

**Errors:** Returns `error`.


---

#### ValidateBinarizationMethod()

Validate a binarization method string.

**Returns:**

`Ok(())` if the method is valid, or a `ValidationError` with details about valid options.

**Signature:**

```go
func ValidateBinarizationMethod(method string) error
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Method` | `string` | Yes | The binarization method to validate (e.g., "otsu", "adaptive", "sauvola") |

**Returns:** ``

**Errors:** Returns `error`.


---

#### ValidateTokenReductionLevel()

Validate a token reduction level string.

**Returns:**

`Ok(())` if the level is valid, or a `ValidationError` with details about valid options.

**Signature:**

```go
func ValidateTokenReductionLevel(level string) error
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Level` | `string` | Yes | The token reduction level to validate (e.g., "off", "light", "moderate") |

**Returns:** ``

**Errors:** Returns `error`.


---

#### ValidateOcrBackend()

Validate an OCR backend string.

**Returns:**

`Ok(())` if the backend is valid, or a `ValidationError` with details about valid options.

**Signature:**

```go
func ValidateOcrBackend(backend string) error
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Backend` | `string` | Yes | The OCR backend to validate (e.g., "tesseract", "easyocr", "paddleocr") |

**Returns:** ``

**Errors:** Returns `error`.


---

#### ValidateLanguageCode()

Validate a language code (ISO 639-1 or 639-3 format).

Accepts both 2-letter ISO 639-1 codes (e.g., "en", "de") and
3-letter ISO 639-3 codes (e.g., "eng", "deu") for broader compatibility.

**Returns:**

`Ok(())` if the code is valid, or a `ValidationError` indicating an invalid language code.

**Signature:**

```go
func ValidateLanguageCode(code string) error
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Code` | `string` | Yes | The language code to validate |

**Returns:** ``

**Errors:** Returns `error`.


---

#### ValidateTesseractPsm()

Validate a tesseract Page Segmentation Mode (PSM).

**Returns:**

`Ok(())` if the PSM is valid, or a `ValidationError` with details about valid ranges.

**Signature:**

```go
func ValidateTesseractPsm(psm int32) error
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Psm` | `int32` | Yes | The PSM value to validate (0-13) |

**Returns:** ``

**Errors:** Returns `error`.


---

#### ValidateTesseractOem()

Validate a tesseract OCR Engine Mode (OEM).

**Returns:**

`Ok(())` if the OEM is valid, or a `ValidationError` with details about valid options.

**Signature:**

```go
func ValidateTesseractOem(oem int32) error
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Oem` | `int32` | Yes | The OEM value to validate (0-3) |

**Returns:** ``

**Errors:** Returns `error`.


---

#### ValidateOutputFormat()

Validate a document extraction output format.

Accepts the following formats and aliases:
- "plain" or "text" for plain text output
- "markdown" or "md" for Markdown output
- "djot" for Djot markup format
- "html" for HTML output

**Returns:**

`Ok(())` if the format is valid, or a `ValidationError` with details about valid options.

**Signature:**

```go
func ValidateOutputFormat(format string) error
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Format` | `string` | Yes | The output format to validate |

**Returns:** ``

**Errors:** Returns `error`.


---

#### ValidateConfidence()

Validate a confidence threshold value.

Confidence thresholds should be between 0.0 and 1.0 inclusive.

**Returns:**

`Ok(())` if the confidence is valid, or a `ValidationError` with details about valid ranges.

**Signature:**

```go
func ValidateConfidence(confidence float64) error
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Confidence` | `float64` | Yes | The confidence threshold to validate |

**Returns:** ``

**Errors:** Returns `error`.


---

#### ValidateDpi()

Validate a DPI (dots per inch) value.

DPI should be a positive integer, typically 72-600.

**Returns:**

`Ok(())` if the DPI is valid, or a `ValidationError` with details about valid ranges.

**Signature:**

```go
func ValidateDpi(dpi int32) error
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Dpi` | `int32` | Yes | The DPI value to validate |

**Returns:** ``

**Errors:** Returns `error`.


---

#### ValidateChunkingParams()

Validate chunk size parameters.

Checks that max_chars > 0 and max_overlap < max_chars.

**Returns:**

`Ok(())` if the parameters are valid, or a `ValidationError` with details about constraints.

**Signature:**

```go
func ValidateChunkingParams(maxChars int, maxOverlap int) error
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `MaxChars` | `int` | Yes | The maximum characters per chunk |
| `MaxOverlap` | `int` | Yes | The maximum overlap between chunks |

**Returns:** ``

**Errors:** Returns `error`.


---

#### ValidateLlmConfigModel()

Validate that an `LlmConfig` has a non-empty model string.

**Returns:**

`Ok(())` if the model is non-empty, or a `ValidationError` otherwise.

**Signature:**

```go
func ValidateLlmConfigModel(model string) error
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Model` | `string` | Yes | The model string to validate |

**Returns:** ``

**Errors:** Returns `error`.


---

#### ExtractBytes()

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

```go
func ExtractBytes(content []byte, mimeType string, config ExtractionConfig) (ExtractionResult, error)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Content` | `[]byte` | Yes | The byte array to extract |
| `MimeType` | `string` | Yes | MIME type of the content |
| `Config` | `ExtractionConfig` | Yes | Extraction configuration |

**Returns:** `ExtractionResult`

**Errors:** Returns `error`.


---

#### ExtractFile()

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

```go
func ExtractFile(path string, mimeType string, config ExtractionConfig) (ExtractionResult, error)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Path` | `string` | Yes | Path to the file to extract |
| `MimeType` | `*string` | No | Optional MIME type override. If None, will be auto-detected |
| `Config` | `ExtractionConfig` | Yes | Extraction configuration |

**Returns:** `ExtractionResult`

**Errors:** Returns `error`.


---

#### ExtractFileSync()

Synchronous wrapper for `extract_file`.

This is a convenience function that blocks the current thread until extraction completes.
For async code, use `extract_file` directly.

Uses the global Tokio runtime for 100x+ performance improvement over creating
a new runtime per call. Always uses the global runtime to avoid nested runtime issues.

This function is only available with the `tokio-runtime` feature. For WASM targets,
use a truly synchronous extraction approach instead.

**Signature:**

```go
func ExtractFileSync(path string, mimeType string, config ExtractionConfig) (ExtractionResult, error)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Path` | `string` | Yes | Path to the file |
| `MimeType` | `*string` | No | The mime type |
| `Config` | `ExtractionConfig` | Yes | The configuration options |

**Returns:** `ExtractionResult`

**Errors:** Returns `error`.


---

#### ExtractBytesSync()

Synchronous wrapper for `extract_bytes`.

Uses the global Tokio runtime for 100x+ performance improvement over creating
a new runtime per call.

With the `tokio-runtime` feature, this blocks the current thread using the global
Tokio runtime. Without it (WASM), this calls a truly synchronous implementation.

**Signature:**

```go
func ExtractBytesSync(content []byte, mimeType string, config ExtractionConfig) (ExtractionResult, error)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Content` | `[]byte` | Yes | The content to process |
| `MimeType` | `string` | Yes | The mime type |
| `Config` | `ExtractionConfig` | Yes | The configuration options |

**Returns:** `ExtractionResult`

**Errors:** Returns `error`.


---

#### BatchExtractFileSync()

Synchronous wrapper for `batch_extract_file`.

Uses the global Tokio runtime for optimal performance.
Only available with `tokio-runtime` (WASM has no filesystem).

**Signature:**

```go
func BatchExtractFileSync(items []string, config ExtractionConfig) ([]ExtractionResult, error)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Items` | `[]string` | Yes | The items |
| `Config` | `ExtractionConfig` | Yes | The configuration options |

**Returns:** `[]ExtractionResult`

**Errors:** Returns `error`.


---

#### BatchExtractBytesSync()

Synchronous wrapper for `batch_extract_bytes`.

Uses the global Tokio runtime for optimal performance.
With the `tokio-runtime` feature, this blocks the current thread using the global
Tokio runtime. Without it (WASM), this calls a truly synchronous implementation
that iterates through items and calls `extract_bytes_sync()`.

**Signature:**

```go
func BatchExtractBytesSync(items []string, config ExtractionConfig) ([]ExtractionResult, error)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Items` | `[]string` | Yes | The items |
| `Config` | `ExtractionConfig` | Yes | The configuration options |

**Returns:** `[]ExtractionResult`

**Errors:** Returns `error`.


---

#### BatchExtractFile()

Extract content from multiple files concurrently.

This function processes multiple files in parallel, automatically managing
concurrency to prevent resource exhaustion. The concurrency limit can be
configured via `ExtractionConfig.max_concurrent_extractions` or defaults
to `(num_cpus * 1.5).ceil()`.

Each file can optionally specify a `FileExtractionConfig` that overrides specific
fields from the batch-level `config`. Pass `nil` for a file to use the batch defaults.
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

```go
func BatchExtractFile(items []string, config ExtractionConfig) ([]ExtractionResult, error)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Items` | `[]string` | Yes | Vector of `(path, optional_file_config)` tuples. Pass `None` as the |
| `Config` | `ExtractionConfig` | Yes | Batch-level extraction configuration (provides defaults and batch settings) |

**Returns:** `[]ExtractionResult`

**Errors:** Returns `error`.


---

#### BatchExtractBytes()

Extract content from multiple byte arrays concurrently.

This function processes multiple byte arrays in parallel, automatically managing
concurrency to prevent resource exhaustion. The concurrency limit can be
configured via `ExtractionConfig.max_concurrent_extractions` or defaults
to `(num_cpus * 1.5).ceil()`.

Each item can optionally specify a `FileExtractionConfig` that overrides specific
fields from the batch-level `config`. Pass `nil` as the config to use
the batch-level defaults for that item.

**Returns:**

A vector of `ExtractionResult` in the same order as the input items.

Simple usage with no per-item overrides:


Per-item configuration overrides:

**Signature:**

```go
func BatchExtractBytes(items []string, config ExtractionConfig) ([]ExtractionResult, error)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Items` | `[]string` | Yes | Vector of `(bytes, mime_type, optional_file_config)` tuples |
| `Config` | `ExtractionConfig` | Yes | Batch-level extraction configuration |

**Returns:** `[]ExtractionResult`

**Errors:** Returns `error`.


---

#### IsValidFormatField()

Validates whether a field name is in the known formats registry.

This uses a pre-built hash set for O(1) lookups instead of linear search,
providing significant performance improvements for repeated validations.

**Returns:**

`true` if the field is in KNOWN_FORMATS, `false` otherwise.

**Signature:**

```go
func IsValidFormatField(field string) bool
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Field` | `string` | Yes | The field name to validate |

**Returns:** `bool`


---

#### ValidateMimeType()

Validate that a MIME type is supported.

**Returns:**

The validated MIME type (may be normalized).

**Errors:**

Returns `KreuzbergError.UnsupportedFormat` if not supported.

**Signature:**

```go
func ValidateMimeType(mimeType string) (string, error)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `MimeType` | `string` | Yes | The MIME type to validate |

**Returns:** `string`

**Errors:** Returns `error`.


---

#### DetectOrValidate()

Detect or validate MIME type.

If `mime_type` is provided, validates it. Otherwise, detects from `path`.

**Returns:**

The validated MIME type string.

**Signature:**

```go
func DetectOrValidate(path string, mimeType string) (string, error)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Path` | `*string` | No | Optional path to detect MIME type from |
| `MimeType` | `*string` | No | Optional explicit MIME type to validate |

**Returns:** `string`

**Errors:** Returns `error`.


---

#### DetectMimeTypeFromBytes()

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

```go
func DetectMimeTypeFromBytes(content []byte) (string, error)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Content` | `[]byte` | Yes | Raw file bytes |

**Returns:** `string`

**Errors:** Returns `error`.


---

#### GetExtensionsForMime()

Get file extensions for a given MIME type.

Returns all known file extensions that map to the specified MIME type.

**Returns:**

A vector of file extensions (without leading dot) for the MIME type.

**Signature:**

```go
func GetExtensionsForMime(mimeType string) ([]string, error)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `MimeType` | `string` | Yes | The MIME type to look up |

**Returns:** `[]string`

**Errors:** Returns `error`.


---

#### ListSupportedFormats()

List all supported document formats.

Returns a list of all file extensions and their corresponding MIME types
that Kreuzberg can process. Derived from the centralized `FORMATS` registry.

The list is sorted alphabetically by file extension.

**Signature:**

```go
func ListSupportedFormats() []SupportedFormat
```

**Returns:** `[]SupportedFormat`


---

#### ClearProcessorCache()

Clear the processor cache (primarily for testing when registry changes).

**Signature:**

```go
func ClearProcessorCache() error
```

**Returns:** ``

**Errors:** Returns `error`.


---

#### TransformExtractionResultToElements()

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

```go
func TransformExtractionResultToElements(result ExtractionResult) []Element
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Result` | `ExtractionResult` | Yes | Reference to the ExtractionResult to transform |

**Returns:** `[]Element`


---

#### ExtractEmailContent()

Extract email content from either .eml or .msg format

**Signature:**

```go
func ExtractEmailContent(data []byte, mimeType string, fallbackCodepage uint32) (EmailExtractionResult, error)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Data` | `[]byte` | Yes | The data |
| `MimeType` | `string` | Yes | The mime type |
| `FallbackCodepage` | `*uint32` | No | The fallback codepage |

**Returns:** `EmailExtractionResult`

**Errors:** Returns `error`.


---

#### CellsToText()

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

```go
func CellsToText(cells [][]string) string
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Cells` | `[][]string` | Yes | A slice of vectors representing table rows, where each inner vector contains cell values |

**Returns:** `string`


---

#### CellsToMarkdown()

**Signature:**

```go
func CellsToMarkdown(cells [][]string) string
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Cells` | `[][]string` | Yes | The cells |

**Returns:** `string`


---

#### DjotToHtml()

Render djot content to HTML.

This function takes djot source text and renders it to HTML using jotdown's
built-in HTML renderer.

**Returns:**

A `Result` containing the rendered HTML string

**Signature:**

```go
func DjotToHtml(djotSource string) (string, error)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `DjotSource` | `string` | Yes | The djot markup text to render |

**Returns:** `string`

**Errors:** Returns `error`.


---

#### DedupText()

Deduplicate a list of text strings while preserving order.
Adjacent duplicates and near-duplicates are removed.

**Signature:**

```go
func DedupText(texts []string) []string
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Texts` | `[]string` | Yes | The texts |

**Returns:** `[]string`


---

#### NormalizeWhitespace()

Normalize whitespace in a string.

- Collapses multiple consecutive spaces/tabs into a single space
- Preserves single newlines (paragraph breaks from \par)
- Collapses multiple consecutive newlines into a double newline
- Trims leading/trailing whitespace from each line
- Trims leading/trailing blank lines

**Signature:**

```go
func NormalizeWhitespace(s string) string
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `S` | `string` | Yes | The s |

**Returns:** `string`


---

#### RegisterDefaultExtractors()

Register all built-in extractors with the global registry.

This function should be called once at application startup to register
the default extractors (PlainText, Markdown, XML, etc.).

**Note:** This is called automatically on first extraction operation.
Explicit calling is optional.

**Signature:**

```go
func RegisterDefaultExtractors() error
```

**Returns:** ``

**Errors:** Returns `error`.


---

#### UnregisterExtractor()

Unregister a document extractor by name.

**Signature:**

```go
func UnregisterExtractor(name string) error
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Name` | `string` | Yes | The name |

**Returns:** ``

**Errors:** Returns `error`.


---

#### ListExtractors()

List names of all registered document extractors.

**Signature:**

```go
func ListExtractors() ([]string, error)
```

**Returns:** `[]string`

**Errors:** Returns `error`.


---

#### ClearExtractors()

Remove all registered document extractors.

**Signature:**

```go
func ClearExtractors() error
```

**Returns:** ``

**Errors:** Returns `error`.


---

#### UnregisterOcrBackend()

Unregister an OCR backend by name.

Removes the OCR backend from the global registry and calls its `shutdown()` method.

**Returns:**

- `Ok(())` if the backend was unregistered or didn't exist
- `Err(...)` if the shutdown method failed

**Signature:**

```go
func UnregisterOcrBackend(name string) error
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Name` | `string` | Yes | Name of the OCR backend to unregister |

**Returns:** ``

**Errors:** Returns `error`.


---

#### ListOcrBackends()

List all registered OCR backends.

Returns the names of all OCR backends currently registered in the global registry.

**Returns:**

A vector of OCR backend names.

**Signature:**

```go
func ListOcrBackends() ([]string, error)
```

**Returns:** `[]string`

**Errors:** Returns `error`.


---

#### ClearOcrBackends()

Clear all OCR backends from the global registry.

Removes all OCR backends and calls their `shutdown()` methods.

**Returns:**

- `Ok(())` if all backends were cleared successfully
- `Err(...)` if any shutdown method failed

**Signature:**

```go
func ClearOcrBackends() error
```

**Returns:** ``

**Errors:** Returns `error`.


---

#### ListPostProcessors()

List all registered post-processor names.

Returns a vector of all post-processor names currently registered in the
global registry.

**Returns:**

- `Ok(Vec<String>)` - Vector of post-processor names
- `Err(...)` if the registry lock is poisoned

**Signature:**

```go
func ListPostProcessors() ([]string, error)
```

**Returns:** `[]string`

**Errors:** Returns `error`.


---

#### UnregisterRenderer()

Unregister a renderer by name.

**Signature:**

```go
func UnregisterRenderer(name string) error
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Name` | `string` | Yes | The name |

**Returns:** ``

**Errors:** Returns `error`.


---

#### ListRenderers()

List names of all registered renderers.

**Signature:**

```go
func ListRenderers() ([]string, error)
```

**Returns:** `[]string`

**Errors:** Returns `error`.


---

#### ClearRenderers()

Remove all registered renderers.

**Signature:**

```go
func ClearRenderers() error
```

**Returns:** ``

**Errors:** Returns `error`.


---

#### ListValidators()

List names of all registered validators.

**Signature:**

```go
func ListValidators() ([]string, error)
```

**Returns:** `[]string`

**Errors:** Returns `error`.


---

#### ClearValidators()

Remove all registered validators.

**Signature:**

```go
func ClearValidators() error
```

**Returns:** ``

**Errors:** Returns `error`.


---

#### SanitizeFilename()

Sanitize a file path to return only the filename (no directory).

Prevents PII from appearing in traces.

**Signature:**

```go
func SanitizeFilename(path string) string
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Path` | `string` | Yes | Path to the file |

**Returns:** `string`


---

#### SanitizePath()

Sanitize a file path to return only the filename.

Prevents PII (personally identifiable information) from appearing in
traces by only recording filenames instead of full paths.

**Signature:**

```go
func SanitizePath(path string) string
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Path` | `string` | Yes | Path to the file |

**Returns:** `string`


---

#### IsValidUtf8()

Validates bytes as UTF-8 without conversion to string slice.

Returns `true` if the bytes represent valid UTF-8, `false` otherwise.
This is useful when you only need to check validity without constructing a string.

**Returns:**

`true` if valid UTF-8, `false` otherwise.

# Performance

This function is optimized for early exit on invalid sequences.

**Signature:**

```go
func IsValidUtf8(bytes []byte) bool
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Bytes` | `[]byte` | Yes | The byte slice to validate |

**Returns:** `bool`


---

#### CleanExtractedText()

**Signature:**

```go
func CleanExtractedText(text string) string
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Text` | `string` | Yes | The text |

**Returns:** `string`


---

#### ReduceTokens()

Reduces token count in text while preserving meaning and structure.

This function removes stopwords, redundancy, and applies compression techniques
based on the specified reduction level. Supports 64 languages with automatic
stopword removal and optional semantic clustering.

**Returns:**

Returns the reduced text with preserved structure (markdown, code blocks).

**Errors:**

Returns an error if the language hint is invalid or stopwords cannot be loaded.

**Signature:**

```go
func ReduceTokens(text string, config TokenReductionConfig, languageHint string) (string, error)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Text` | `string` | Yes | The input text to reduce |
| `Config` | `TokenReductionConfig` | Yes | Configuration specifying reduction level and options |
| `LanguageHint` | `*string` | No | Optional ISO 639-3 language code (e.g., "eng", "spa") |

**Returns:** `string`

**Errors:** Returns `error`.


---

#### BatchReduceTokens()

Reduces token count for multiple texts efficiently using parallel processing.

This function processes multiple texts in parallel using Rayon, providing
significant performance improvements for batch operations. All texts use the
same configuration and language hint for consistency.

**Returns:**

Returns a vector of reduced texts in the same order as the input.

**Errors:**

Returns an error if the language hint is invalid or stopwords cannot be loaded.

**Signature:**

```go
func BatchReduceTokens(texts []string, config TokenReductionConfig, languageHint string) ([]string, error)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Texts` | `[]string` | Yes | Slice of text references to reduce |
| `Config` | `TokenReductionConfig` | Yes | Configuration specifying reduction level and options |
| `LanguageHint` | `*string` | No | Optional ISO 639-3 language code (e.g., "eng", "spa") |

**Returns:** `[]string`

**Errors:** Returns `error`.


---

#### Bold()

Create a bold annotation for the given byte range.

**Signature:**

```go
func Bold(start uint32, end uint32) TextAnnotation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Start` | `uint32` | Yes | The start |
| `End` | `uint32` | Yes | The end |

**Returns:** `TextAnnotation`


---

#### Italic()

Create an italic annotation for the given byte range.

**Signature:**

```go
func Italic(start uint32, end uint32) TextAnnotation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Start` | `uint32` | Yes | The start |
| `End` | `uint32` | Yes | The end |

**Returns:** `TextAnnotation`


---

#### Underline()

Create an underline annotation for the given byte range.

**Signature:**

```go
func Underline(start uint32, end uint32) TextAnnotation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Start` | `uint32` | Yes | The start |
| `End` | `uint32` | Yes | The end |

**Returns:** `TextAnnotation`


---

#### Link()

Create a link annotation for the given byte range.

**Signature:**

```go
func Link(start uint32, end uint32, url string, title string) TextAnnotation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Start` | `uint32` | Yes | The start |
| `End` | `uint32` | Yes | The end |
| `Url` | `string` | Yes | The URL to fetch |
| `Title` | `*string` | No | The title |

**Returns:** `TextAnnotation`


---

#### Code()

Create a code (inline) annotation for the given byte range.

**Signature:**

```go
func Code(start uint32, end uint32) TextAnnotation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Start` | `uint32` | Yes | The start |
| `End` | `uint32` | Yes | The end |

**Returns:** `TextAnnotation`


---

#### Strikethrough()

Create a strikethrough annotation for the given byte range.

**Signature:**

```go
func Strikethrough(start uint32, end uint32) TextAnnotation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Start` | `uint32` | Yes | The start |
| `End` | `uint32` | Yes | The end |

**Returns:** `TextAnnotation`


---

#### Subscript()

Create a subscript annotation for the given byte range.

**Signature:**

```go
func Subscript(start uint32, end uint32) TextAnnotation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Start` | `uint32` | Yes | The start |
| `End` | `uint32` | Yes | The end |

**Returns:** `TextAnnotation`


---

#### Superscript()

Create a superscript annotation for the given byte range.

**Signature:**

```go
func Superscript(start uint32, end uint32) TextAnnotation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Start` | `uint32` | Yes | The start |
| `End` | `uint32` | Yes | The end |

**Returns:** `TextAnnotation`


---

#### FontSize()

Create a font size annotation for the given byte range.

**Signature:**

```go
func FontSize(start uint32, end uint32, value string) TextAnnotation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Start` | `uint32` | Yes | The start |
| `End` | `uint32` | Yes | The end |
| `Value` | `string` | Yes | The value |

**Returns:** `TextAnnotation`


---

#### Color()

Create a color annotation for the given byte range.

**Signature:**

```go
func Color(start uint32, end uint32, value string) TextAnnotation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Start` | `uint32` | Yes | The start |
| `End` | `uint32` | Yes | The end |
| `Value` | `string` | Yes | The value |

**Returns:** `TextAnnotation`


---

#### Highlight()

Create a highlight annotation for the given byte range.

**Signature:**

```go
func Highlight(start uint32, end uint32) TextAnnotation
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Start` | `uint32` | Yes | The start |
| `End` | `uint32` | Yes | The end |

**Returns:** `TextAnnotation`


---

#### ClassifyUri()

Classify a URL string into the appropriate `UriKind`.

- `mailto:` → `Email`
- `#` prefix → `Anchor`
- everything else → `Hyperlink`

**Signature:**

```go
func ClassifyUri(url string) UriKind
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Url` | `string` | Yes | The URL to fetch |

**Returns:** `UriKind`


---

#### SafeDecode()

Decode raw bytes into UTF-8, using heuristics and fallback encodings when necessary.

The function prefers an explicit `encoding`, falls back to the cached guess, probes
an encoding detector, and finally tries a small curated list before returning a
mojibake-cleaned string.

**Signature:**

```go
func SafeDecode(byteData []byte, encoding string) string
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `ByteData` | `[]byte` | Yes | The byte data |
| `Encoding` | `*string` | No | The encoding |

**Returns:** `string`


---

#### CalculateTextConfidence()

Estimate how trustworthy a decoded string is on a 0.0–1.0 scale.

Scores close to 1.0 indicate mostly printable characters, whereas lower scores
point to mojibake, control characters, or suspicious character mixes.

**Signature:**

```go
func CalculateTextConfidence(text string) float64
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Text` | `string` | Yes | The text |

**Returns:** `float64`


---

#### CreateStringBufferPool()

Create a pre-configured string buffer pool for batch processing.

**Returns:**

A pool configured for text accumulation with reasonable defaults.

**Signature:**

```go
func CreateStringBufferPool(poolSize int, bufferCapacity int) StringBufferPool
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `PoolSize` | `int` | Yes | Maximum number of buffers to keep in the pool |
| `BufferCapacity` | `int` | Yes | Initial capacity for each buffer in bytes |

**Returns:** `StringBufferPool`


---

#### CreateByteBufferPool()

Create a pre-configured byte buffer pool for batch processing.

**Returns:**

A pool configured for binary data handling with reasonable defaults.

**Signature:**

```go
func CreateByteBufferPool(poolSize int, bufferCapacity int) ByteBufferPool
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `PoolSize` | `int` | Yes | Maximum number of buffers to keep in the pool |
| `BufferCapacity` | `int` | Yes | Initial capacity for each buffer in bytes |

**Returns:** `ByteBufferPool`


---

#### OpenapiJson()

Generate OpenAPI JSON schema.

Returns the complete OpenAPI 3.1 specification as a JSON string.

**Signature:**

```go
func OpenapiJson() string
```

**Returns:** `string`


---

#### ServeDefault()

Start the API server with default host and port.

Defaults: host = "127.0.0.1", port = 8000

Uses config file discovery (searches current/parent directories for kreuzberg.toml/yaml/json).
Validates plugins at startup to help diagnose configuration issues.

**Signature:**

```go
func ServeDefault() error
```

**Returns:** ``

**Errors:** Returns `error`.


---

#### ChunkText()

Split text into chunks with optional page boundary tracking.

This is the primary API function for chunking text. It supports both plain text
and Markdown with configurable chunk size, overlap, and page boundary mapping.

**Returns:**

A ChunkingResult containing all chunks and their metadata.

**Signature:**

```go
func ChunkText(text string, config ChunkingConfig, pageBoundaries []PageBoundary) (ChunkingResult, error)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Text` | `string` | Yes | The text to split into chunks |
| `Config` | `ChunkingConfig` | Yes | Chunking configuration (max size, overlap, type) |
| `PageBoundaries` | `*[]PageBoundary` | No | Optional page boundary markers for mapping chunks to pages |

**Returns:** `ChunkingResult`

**Errors:** Returns `error`.


---

#### ChunkTextWithHeadingSource()

Chunk text with an optional separate markdown source for heading context resolution.

When `heading_source` is provided, it is used instead of `text` for building the
heading map. This is needed when `text` is plain text (no markdown headings) but
the original document had headings that were stripped during rendering.

**Signature:**

```go
func ChunkTextWithHeadingSource(text string, config ChunkingConfig, pageBoundaries []PageBoundary, headingSource string) (ChunkingResult, error)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Text` | `string` | Yes | The text |
| `Config` | `ChunkingConfig` | Yes | The configuration options |
| `PageBoundaries` | `*[]PageBoundary` | No | The page boundaries |
| `HeadingSource` | `*string` | No | The heading source |

**Returns:** `ChunkingResult`

**Errors:** Returns `error`.


---

#### ChunkTextsBatch()

Batch process multiple texts with the same configuration.

This convenience function applies the same chunking configuration to multiple
texts in sequence.

**Returns:**

A vector of ChunkingResult objects, one per input text.

**Errors:**

Returns an error if chunking any individual text fails.

**Signature:**

```go
func ChunkTextsBatch(texts []string, config ChunkingConfig) ([]ChunkingResult, error)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Texts` | `[]string` | Yes | Slice of text strings to chunk |
| `Config` | `ChunkingConfig` | Yes | Chunking configuration to apply to all texts |

**Returns:** `[]ChunkingResult`

**Errors:** Returns `error`.


---

#### ChunkSemantic()

Split text into semantically coherent chunks.

Splits text into fine-grained segments, detects structural (and optionally
embedding-based) topic boundaries, then merges segments into chunks that
respect those boundaries and the configured size budget.

**Signature:**

```go
func ChunkSemantic(text string, config ChunkingConfig, pageBoundaries []PageBoundary) (ChunkingResult, error)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Text` | `string` | Yes | The text |
| `Config` | `ChunkingConfig` | Yes | The configuration options |
| `PageBoundaries` | `*[]PageBoundary` | No | The page boundaries |

**Returns:** `ChunkingResult`

**Errors:** Returns `error`.


---

#### Normalize()

L2-normalize a vector.

**Signature:**

```go
func Normalize(v []float32) []float32
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `V` | `[]float32` | Yes | The v |

**Returns:** `[]float32`


---

#### GetPreset()

Get a preset by name.

**Signature:**

```go
func GetPreset(name string) *string
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Name` | `string` | Yes | The name |

**Returns:** `*string`


---

#### ListPresets()

List all available preset names.

**Signature:**

```go
func ListPresets() []string
```

**Returns:** `[]string`


---

#### WarmModel()

Eagerly download and cache an embedding model without returning the handle.

This triggers the same download and initialization as `get_or_init_engine`
but discards the result, making it suitable for cache-warming scenarios
where the caller doesn't need to use the model immediately.

**Note**: This function downloads AND initializes the ONNX model, which
requires ONNX Runtime and uses significant memory. For download-only
scenarios (e.g., init containers), use `download_model` instead.

**Signature:**

```go
func WarmModel(modelType EmbeddingModelType, cacheDir string) error
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `ModelType` | `EmbeddingModelType` | Yes | The embedding model type |
| `CacheDir` | `*string` | No | The cache dir |

**Returns:** ``

**Errors:** Returns `error`.


---

#### DownloadModel()

Download an embedding model's files without initializing ONNX Runtime.

Downloads the model files (ONNX model, tokenizer, config) from HuggingFace
to the cache directory. Subsequent calls to `warm_model` or
`get_or_init_engine` will find the files cached and skip the download step.

This is ideal for init containers or CI environments where you want to
pre-populate the cache without loading models into memory.

**Signature:**

```go
func DownloadModel(modelType EmbeddingModelType, cacheDir string) error
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `ModelType` | `EmbeddingModelType` | Yes | The embedding model type |
| `CacheDir` | `*string` | No | The cache dir |

**Returns:** ``

**Errors:** Returns `error`.


---

#### CalculateOptimalDpi()

Calculate optimal DPI with min/max constraints

**Signature:**

```go
func CalculateOptimalDpi(pageWidth float64, pageHeight float64, targetDpi int32, maxDimension int32, minDpi int32, maxDpi int32) int32
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `PageWidth` | `float64` | Yes | The page width |
| `PageHeight` | `float64` | Yes | The page height |
| `TargetDpi` | `int32` | Yes | The target dpi |
| `MaxDimension` | `int32` | Yes | The max dimension |
| `MinDpi` | `int32` | Yes | The min dpi |
| `MaxDpi` | `int32` | Yes | The max dpi |

**Returns:** `int32`


---

#### DetectLanguages()

Detect languages in text using whatlang.

Returns a list of detected language codes (ISO 639-3 format).
Returns `nil` if no languages could be detected with sufficient confidence.

**Signature:**

```go
func DetectLanguages(text string, config LanguageDetectionConfig) (*[]string, error)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Text` | `string` | Yes | The text to analyze for language detection |
| `Config` | `LanguageDetectionConfig` | Yes | Optional configuration for language detection |

**Returns:** `*[]string`

**Errors:** Returns `error`.


---

#### ExtractKeywords()

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

```go
func ExtractKeywords(text string, config KeywordConfig) ([]Keyword, error)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Text` | `string` | Yes | The text to extract keywords from |
| `Config` | `KeywordConfig` | Yes | Keyword extraction configuration |

**Returns:** `[]Keyword`

**Errors:** Returns `error`.


---

#### ComputeHash()

Compute a blake3 hash string from input data.

Returns a 32-character hex string (128 bits of blake3 output).

**Signature:**

```go
func ComputeHash(data string) string
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Data` | `string` | Yes | The data |

**Returns:** `string`


---

#### RenderPdfPageToPng()

Render a single PDF page to a PNG-encoded byte buffer.

**Errors:**

Returns an error if the PDF is invalid, the page index is out of bounds,
or if the page fails to render.

**Signature:**

```go
func RenderPdfPageToPng(pdfBytes []byte, pageIndex int, dpi int32, password string) ([]byte, error)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `PdfBytes` | `[]byte` | Yes | The pdf bytes |
| `PageIndex` | `int` | Yes | The page index |
| `Dpi` | `*int32` | No | The dpi |
| `Password` | `*string` | No | The password |

**Returns:** `[]byte`

**Errors:** Returns `error`.


---

#### ExtractTextFromPdf()

**Signature:**

```go
func ExtractTextFromPdf(pdfBytes []byte) (string, error)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `PdfBytes` | `[]byte` | Yes | The pdf bytes |

**Returns:** `string`

**Errors:** Returns `error`.


---

#### SerializeToToon()

Serialize an `ExtractionResult` to TOON (Token-Oriented Object Notation).

TOON is a token-efficient alternative to JSON for LLM prompts.
Losslessly convertible to/from JSON but uses fewer tokens.

**Signature:**

```go
func SerializeToToon(result ExtractionResult) (string, error)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Result` | `ExtractionResult` | Yes | The extraction result |

**Returns:** `string`

**Errors:** Returns `error`.


---

#### SerializeToJson()

Serialize an `ExtractionResult` to pretty-printed JSON.

**Signature:**

```go
func SerializeToJson(result ExtractionResult) (string, error)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Result` | `ExtractionResult` | Yes | The extraction result |

**Returns:** `string`

**Errors:** Returns `error`.


---

### Types

#### AccelerationConfig

Hardware acceleration configuration for ONNX Runtime models.

Controls which execution provider (CPU, CoreML, CUDA, TensorRT) is used
for inference in layout detection and embedding generation.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Provider` | `ExecutionProviderType` | `ExecutionProviderType.Auto` | Execution provider to use for ONNX inference. |
| `DeviceId` | `uint32` | — | GPU device ID (for CUDA/TensorRT). Ignored for CPU/CoreML/Auto. |


---

#### AnchorProperties

Properties for anchored drawings.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `BehindDoc` | `bool` | — | Behind doc |
| `LayoutInCell` | `bool` | — | Layout in cell |
| `RelativeHeight` | `*int64` | `nil` | Relative height |
| `PositionH` | `*string` | `nil` | Position h |
| `PositionV` | `*string` | `nil` | Position v |
| `WrapType` | `string` | — | Wrap type |


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
| `DefaultConfig` | `ExtractionConfig` | — | Default extraction configuration |
| `ExtractionService` | `string` | — | Tower service for extraction requests. Wrapped in `Arc<Mutex>` because `BoxCloneService` is `Send` but not `Sync`, while `ApiState` must be `Clone + Sync` for Axum's state requirement. The lock is held only long enough to clone the service. |


---

#### ArchiveEntry

A single file extracted from an archive.

When archives (ZIP, TAR, 7Z, GZIP) are extracted with recursive extraction
enabled, each processable file produces its own full `ExtractionResult`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Path` | `string` | — | Archive-relative file path (e.g. "folder/document.pdf"). |
| `MimeType` | `string` | — | Detected MIME type of the file. |
| `Result` | `ExtractionResult` | — | Full extraction result for this file. |


---

#### ArchiveMetadata

Archive (ZIP/TAR/7Z) metadata.

Extracted from compressed archive files containing file lists and size information.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Format` | `string` | — | Archive format ("ZIP", "TAR", "7Z", etc.) |
| `FileCount` | `int` | — | Total number of files in the archive |
| `FileList` | `[]string` | `nil` | List of file paths within the archive |
| `TotalSize` | `int` | — | Total uncompressed size in bytes |
| `CompressedSize` | `*int` | `nil` | Compressed size in bytes (if available) |


---

#### BBox

Bounding box in original image coordinates (x1, y1) top-left, (x2, y2) bottom-right.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `X1` | `float32` | — | X1 |
| `Y1` | `float32` | — | Y1 |
| `X2` | `float32` | — | X2 |
| `Y2` | `float32` | — | Y2 |


---

#### BatchExtractFilesParams

Request parameters for batch file extraction.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Paths` | `[]string` | — | Paths to files to extract |
| `Config` | `*interface{}` | `nil` | Extraction configuration (JSON object) |
| `PdfPassword` | `*string` | `nil` | Password for encrypted PDFs |
| `FileConfigs` | `*[]*interface{}` | `nil` | Per-file extraction configuration overrides (parallel array to paths). Each entry is either null (use default) or a FileExtractionConfig JSON object. |
| `ResponseFormat` | `*string` | `nil` | Wire format for the response: "json" (default) or "toon" |


---

#### BibtexMetadata

BibTeX bibliography metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `EntryCount` | `int` | — | Number of entries in the bibliography. |
| `CitationKeys` | `[]string` | `nil` | Citation keys |
| `Authors` | `[]string` | `nil` | Authors |
| `YearRange` | `*YearRange` | `nil` | Year range (year range) |
| `EntryTypes` | `*map[string]int` | `nil` | Entry types |


---

#### ByteBufferPool

Convenience type alias for a pooled Vec<u8>.


---

#### CacheClearResponse

Cache clear response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Directory` | `string` | — | Cache directory path |
| `RemovedFiles` | `int` | — | Number of files removed |
| `FreedMb` | `float64` | — | Space freed in MB |


---

#### CacheStatsResponse

Cache statistics response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Directory` | `string` | — | Cache directory path |
| `TotalFiles` | `int` | — | Total number of cache files |
| `TotalSizeMb` | `float64` | — | Total cache size in MB |
| `AvailableSpaceMb` | `float64` | — | Available disk space in MB |
| `OldestFileAgeDays` | `float64` | — | Age of oldest file in days |
| `NewestFileAgeDays` | `float64` | — | Age of newest file in days |


---

#### CacheWarmParams

Request parameters for cache warm (model download).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `AllEmbeddings` | `bool` | — | Download all embedding model presets |
| `EmbeddingModel` | `*string` | `nil` | Specific embedding preset name to download (e.g. "balanced", "speed", "quality") |


---

#### Chunk

A text chunk with optional embedding and metadata.

Chunks are created when chunking is enabled in `ExtractionConfig`. Each chunk
contains the text content, optional embedding vector (if embedding generation
is configured), and metadata about its position in the document.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Content` | `string` | — | The text content of this chunk. |
| `ChunkType` | `ChunkType` | — | Semantic structural classification of this chunk. Assigned by the heuristic classifier based on content patterns and heading context. Defaults to `ChunkType.Unknown` when no rule matches. |
| `Embedding` | `*[]float32` | `nil` | Optional embedding vector for this chunk. Only populated when `EmbeddingConfig` is provided in chunking configuration. The dimensionality depends on the chosen embedding model. |
| `Metadata` | `ChunkMetadata` | — | Metadata about this chunk's position and properties. |


---

#### ChunkMetadata

Metadata about a chunk's position in the original document.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `ByteStart` | `int` | — | Byte offset where this chunk starts in the original text (UTF-8 valid boundary). |
| `ByteEnd` | `int` | — | Byte offset where this chunk ends in the original text (UTF-8 valid boundary). |
| `TokenCount` | `*int` | `nil` | Number of tokens in this chunk (if available). This is calculated by the embedding model's tokenizer if embeddings are enabled. |
| `ChunkIndex` | `int` | — | Zero-based index of this chunk in the document. |
| `TotalChunks` | `int` | — | Total number of chunks in the document. |
| `FirstPage` | `*int` | `nil` | First page number this chunk spans (1-indexed). Only populated when page tracking is enabled in extraction configuration. |
| `LastPage` | `*int` | `nil` | Last page number this chunk spans (1-indexed, equal to first_page for single-page chunks). Only populated when page tracking is enabled in extraction configuration. |
| `HeadingContext` | `*HeadingContext` | `nil` | Heading context when using Markdown chunker. Contains the heading hierarchy this chunk falls under. Only populated when `ChunkerType.Markdown` is used. |


---

#### ChunkRequest

Chunk request with text and configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Text` | `string` | — | Text to chunk (must not be empty) |
| `Config` | `*string` | `nil` | Optional chunking configuration |
| `ChunkerType` | `string` | — | Chunker type (text, markdown, yaml, or semantic) |


---

#### ChunkResponse

Chunk response with chunks and metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Chunks` | `[]string` | — | List of chunks |
| `ChunkCount` | `int` | — | Total number of chunks |
| `Config` | `string` | — | Configuration used for chunking |
| `InputSizeBytes` | `int` | — | Input text size in bytes |
| `ChunkerType` | `string` | — | Chunker type used for chunking |


---

#### ChunkTextParams

Request parameters for text chunking.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Text` | `string` | — | Text content to split into chunks |
| `MaxCharacters` | `*int` | `nil` | Maximum characters per chunk (default: 2000) |
| `Overlap` | `*int` | `nil` | Number of overlapping characters between chunks (default: 100) |
| `ChunkerType` | `*string` | `nil` | Chunker type: "text", "markdown", "yaml", or "semantic" (default: "text") |
| `TopicThreshold` | `*float32` | `nil` | Topic threshold for semantic chunking (0.0-1.0, default: 0.75) |


---

#### ChunkingConfig

Chunking configuration.

Configures text chunking for document content, including chunk size,
overlap, trimming behavior, and optional embeddings.

Use `..the default constructor` when constructing to allow for future field additions:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `MaxCharacters` | `int` | `1000` | Maximum size per chunk (in units determined by `sizing`). When `sizing` is `Characters` (default), this is the max character count. When using token-based sizing, this is the max token count. Default: 1000 |
| `Overlap` | `int` | `200` | Overlap between chunks (in units determined by `sizing`). Default: 200 |
| `Trim` | `bool` | `true` | Whether to trim whitespace from chunk boundaries. Default: true |
| `ChunkerType` | `ChunkerType` | `ChunkerType.Text` | Type of chunker to use (Text or Markdown). Default: Text |
| `Embedding` | `*EmbeddingConfig` | `nil` | Optional embedding configuration for chunk embeddings. |
| `Preset` | `*string` | `nil` | Use a preset configuration (overrides individual settings if provided). |
| `Sizing` | `ChunkSizing` | `ChunkSizing.Characters` | How to measure chunk size. Default: `Characters` (Unicode character count). Enable `chunking-tiktoken` or `chunking-tokenizers` features for token-based sizing. |
| `PrependHeadingContext` | `bool` | `false` | When `true` and `chunker_type` is `Markdown`, prepend the heading hierarchy path (e.g. `"# Title > ## Section\n\n"`) to each chunk's content string. This is useful for RAG pipelines where each chunk needs self-contained context about its position in the document structure. Default: `false` |
| `TopicThreshold` | `*float32` | `nil` | Optional cosine similarity threshold for semantic topic boundary detection. Only used when `chunker_type` is `Semantic` and an `EmbeddingConfig` is provided. You almost never need to set this. When omitted, defaults to `0.75` which works well for most documents. Lower values detect more topic boundaries (more, smaller chunks); higher values detect fewer. Range: `0.0..=1.0`. |

##### Methods

###### Default()

**Signature:**

```go
func (o *ChunkingConfig) Default() ChunkingConfig
```


---

#### ChunkingResult

Result of a text chunking operation.

Contains the generated chunks and metadata about the chunking.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Chunks` | `[]Chunk` | — | List of text chunks |
| `ChunkCount` | `int` | — | Total number of chunks generated |


---

#### CitationMetadata

Citation file metadata (RIS, PubMed, EndNote).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `CitationCount` | `int` | — | Number of citations |
| `Format` | `*string` | `nil` | Format |
| `Authors` | `[]string` | `nil` | Authors |
| `YearRange` | `*YearRange` | `nil` | Year range (year range) |
| `Dois` | `[]string` | `nil` | Dois |
| `Keywords` | `[]string` | `nil` | Keywords |


---

#### CommonPdfMetadata

Common metadata fields extracted from a PDF.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Title` | `*string` | `nil` | Title |
| `Subject` | `*string` | `nil` | Subject |
| `Authors` | `*[]string` | `nil` | Authors |
| `Keywords` | `*[]string` | `nil` | Keywords |
| `CreatedAt` | `*string` | `nil` | Created at |
| `ModifiedAt` | `*string` | `nil` | Modified at |
| `CreatedBy` | `*string` | `nil` | Created by |


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
| `IncludeHeaders` | `bool` | `false` | Include running headers in extraction output. - PDF: Disables top-margin furniture stripping and prevents the layout model from treating `PageHeader`-classified regions as furniture. - DOCX: Includes document headers in text output. - RTF/ODT: Headers already included; this is a no-op when true. - HTML/EPUB: Keeps `<header>` element content. Default: `false` (headers are stripped or excluded). |
| `IncludeFooters` | `bool` | `false` | Include running footers in extraction output. - PDF: Disables bottom-margin furniture stripping and prevents the layout model from treating `PageFooter`-classified regions as furniture. - DOCX: Includes document footers in text output. - RTF/ODT: Footers already included; this is a no-op when true. - HTML/EPUB: Keeps `<footer>` element content. Default: `false` (footers are stripped or excluded). |
| `StripRepeatingText` | `bool` | `true` | Enable the heuristic cross-page repeating text detector. When `true` (default), text that repeats verbatim across a supermajority of pages is classified as furniture and stripped.  Disable this if brand names or repeated headings are being incorrectly removed by the heuristic. Note: when a layout-detection model is active, the model may independently classify page-header / page-footer regions as furniture on a per-page basis. To preserve those regions, set `include_headers = true` and/or `include_footers = true` in addition to disabling this flag. Primarily affects PDF extraction. Default: `true`. |
| `IncludeWatermarks` | `bool` | `false` | Include watermark text in extraction output. - PDF: Keeps watermark artifacts and arXiv identifiers. - Other formats: No effect currently. Default: `false` (watermarks are stripped). |

##### Methods

###### Default()

**Signature:**

```go
func (o *ContentFilterConfig) Default() ContentFilterConfig
```


---

#### ContributorRole

JATS contributor with role.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Name` | `string` | — | The name |
| `Role` | `*string` | `nil` | Role |


---

#### CsvMetadata

CSV/TSV file metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `RowCount` | `int` | — | Number of rows |
| `ColumnCount` | `int` | — | Number of columns |
| `Delimiter` | `*string` | `nil` | Delimiter |
| `HasHeader` | `bool` | — | Whether header |
| `ColumnTypes` | `*[]string` | `nil` | Column types |


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
| `Name` | `string` | — | The name |
| `FieldType` | `string` | — | Field type |


---

#### DbfMetadata

dBASE (DBF) file metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `RecordCount` | `int` | — | Number of records |
| `FieldCount` | `int` | — | Number of fields |
| `Fields` | `[]DbfFieldInfo` | `nil` | Fields |


---

#### DetectMimeTypeParams

Request parameters for MIME type detection.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Path` | `string` | — | Path to the file |
| `UseContent` | `bool` | — | Use content-based detection (default: true) |


---

#### DetectResponse

MIME type detection response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `MimeType` | `string` | — | Detected MIME type |
| `Filename` | `*string` | `nil` | Original filename (if provided) |


---

#### DetectedBoundary

A detected structural boundary in the text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `ByteOffset` | `int` | — | Byte offset of the start of the line in the original text. |
| `IsHeader` | `bool` | — | Whether this boundary looks like a header/section title. |


---

#### DetectionResult

Page-level detection result containing all detections and page metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `PageWidth` | `uint32` | — | Page width |
| `PageHeight` | `uint32` | — | Page height |
| `Detections` | `[]LayoutDetection` | — | Detections |


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
| `PlainText` | `string` | — | Plain text representation for backwards compatibility |
| `Blocks` | `[]FormattedBlock` | — | Structured block-level content |
| `Metadata` | `Metadata` | — | Metadata from YAML frontmatter |
| `Tables` | `[]string` | — | Extracted tables as structured data |
| `Images` | `[]DjotImage` | — | Extracted images with metadata |
| `Links` | `[]DjotLink` | — | Extracted links with URLs |
| `Footnotes` | `[]Footnote` | — | Footnote definitions |
| `Attributes` | `[]string` | — | Attributes mapped by element identifier (if present) |


---

#### DjotImage

Image element in Djot.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Src` | `string` | — | Image source URL or path |
| `Alt` | `string` | — | Alternative text |
| `Title` | `*string` | `nil` | Optional title |
| `Attributes` | `*string` | `nil` | Element attributes |


---

#### DjotLink

Link element in Djot.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Url` | `string` | — | Link URL |
| `Text` | `string` | — | Link text content |
| `Title` | `*string` | `nil` | Optional title |
| `Attributes` | `*string` | `nil` | Element attributes |


---

#### DoclingCompatResponse

OpenWebUI "Docling" engine response format.

Returned by `POST /v1/convert/file` for docling-serve compatibility.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Document` | `string` | — | Converted document content |
| `Status` | `string` | — | Processing status |


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

###### ExtractBytes()

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

```go
func (o *DocumentExtractor) ExtractBytes(content []byte, mimeType string, config ExtractionConfig) (string, error)
```

###### ExtractFile()

Extract content from a file.

Default implementation reads the file and calls `extract_bytes`.
Override for custom file handling, streaming, or memory optimizations.

**Returns:**

An `InternalDocument` containing the extracted elements, metadata, and tables.

**Errors:**

Same as `extract_bytes`, plus file I/O errors.

**Signature:**

```go
func (o *DocumentExtractor) ExtractFile(path string, mimeType string, config ExtractionConfig) (string, error)
```

###### SupportedMimeTypes()

Get the list of MIME types supported by this extractor.

Can include exact MIME types and prefix patterns:
- Exact: `"application/pdf"`, `"text/plain"`
- Prefix: `"image/*"` (matches any image type)

**Returns:**

A slice of MIME type strings.

**Signature:**

```go
func (o *DocumentExtractor) SupportedMimeTypes() []string
```

###### Priority()

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

```go
func (o *DocumentExtractor) Priority() int32
```

###### CanHandle()

Optional: Check if this extractor can handle a specific file.

Allows for more sophisticated detection beyond MIME types.
Defaults to `true` (rely on MIME type matching).

**Returns:**

`true` if the extractor can handle this file, `false` otherwise.

**Signature:**

```go
func (o *DocumentExtractor) CanHandle(path string, mimeType string) bool
```

###### AsSyncExtractor()

Attempt to get a reference to this extractor as a SyncExtractor.

Returns None if the extractor doesn't support synchronous extraction.
This is used for WASM and other sync-only environments.

**Signature:**

```go
func (o *DocumentExtractor) AsSyncExtractor() *SyncExtractor
```


---

#### DocumentNode

A single node in the document tree.

Each node has deterministic `id`, typed `content`, optional `parent`/`children`
for tree structure, and metadata like page number, bounding box, and content layer.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Id` | `string` | — | Deterministic identifier (hash of content + position). |
| `Content` | `NodeContent` | — | Node content — tagged enum, type-specific data only. |
| `Parent` | `*uint32` | `nil` | Parent node index (`nil` = root-level node). |
| `Children` | `[]uint32` | — | Child node indices in reading order. |
| `ContentLayer` | `ContentLayer` | — | Content layer classification. |
| `Page` | `*uint32` | `nil` | Page number where this node starts (1-indexed). |
| `PageEnd` | `*uint32` | `nil` | Page number where this node ends (for multi-page tables/sections). |
| `Bbox` | `*string` | `nil` | Bounding box in document coordinates. |
| `Annotations` | `[]TextAnnotation` | — | Inline annotations (formatting, links) on this node's text content. Only meaningful for text-carrying nodes; empty for containers. |
| `Attributes` | `*map[string]string` | `nil` | Format-specific key-value attributes. Extensible bag for data that doesn't warrant a typed field: CSS classes, LaTeX environment names, Excel cell formulas, slide layout names, etc. |


---

#### DocumentRelationship

A resolved relationship between two nodes in the document tree.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Source` | `uint32` | — | Source node index (the referencing node). |
| `Target` | `uint32` | — | Target node index (the referenced node). |
| `Kind` | `RelationshipKind` | — | Semantic kind of the relationship. |


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
| `Nodes` | `[]DocumentNode` | `nil` | All nodes in document/reading order. |
| `SourceFormat` | `*string` | `nil` | Origin format identifier (e.g. "docx", "pptx", "html", "pdf"). Allows renderers to apply format-aware heuristics when converting the document tree to output formats. |
| `Relationships` | `[]DocumentRelationship` | `nil` | Resolved relationships between nodes (footnote refs, citations, anchor links, etc.). Populated during derivation from the internal document representation. Empty when no relationships are detected. |

##### Methods

###### Default()

**Signature:**

```go
func (o *DocumentStructure) Default() DocumentStructure
```


---

#### DocxMetadata

Word document metadata.

Extracted from DOCX files using shared Office Open XML metadata extraction.
Integrates with `office_metadata` module for core/app/custom properties.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `CoreProperties` | `*string` | `nil` | Core properties from docProps/core.xml (Dublin Core metadata) Contains title, creator, subject, keywords, dates, etc. Shared format across DOCX/PPTX/XLSX documents. |
| `AppProperties` | `*string` | `nil` | Application properties from docProps/app.xml (Word-specific statistics) Contains word count, page count, paragraph count, editing time, etc. DOCX-specific variant of Office application properties. |
| `CustomProperties` | `*map[string]interface{}` | `nil` | Custom properties from docProps/custom.xml (user-defined properties) Contains key-value pairs defined by users or applications. Values can be strings, numbers, booleans, or dates. |


---

#### Drawing

A drawing object extracted from `<w:drawing>`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `DrawingType` | `string` | — | Drawing type |
| `Extent` | `*string` | `nil` | Extent |
| `DocProperties` | `*string` | `nil` | Doc properties |
| `ImageRef` | `*string` | `nil` | Image ref |


---

#### Element

Semantic element extracted from document.

Represents a logical unit of content with semantic classification,
unique identifier, and metadata for tracking origin and position.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `ElementId` | `string` | — | Unique element identifier |
| `ElementType` | `ElementType` | — | Semantic type of this element |
| `Text` | `string` | — | Text content of the element |
| `Metadata` | `ElementMetadata` | — | Metadata about the element |


---

#### ElementMetadata

Metadata for a semantic element.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `PageNumber` | `*int` | `nil` | Page number (1-indexed) |
| `Filename` | `*string` | `nil` | Source filename or document name |
| `Coordinates` | `*string` | `nil` | Bounding box coordinates if available |
| `ElementIndex` | `*int` | `nil` | Position index in the element sequence |
| `Additional` | `map[string]string` | — | Additional custom metadata |


---

#### EmailAttachment

Email attachment representation.

Contains metadata and optionally the content of an email attachment.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Name` | `*string` | `nil` | Attachment name (from Content-Disposition header) |
| `Filename` | `*string` | `nil` | Filename of the attachment |
| `MimeType` | `*string` | `nil` | MIME type of the attachment |
| `Size` | `*int` | `nil` | Size in bytes |
| `IsImage` | `bool` | — | Whether this attachment is an image |
| `Data` | `*[]byte` | `nil` | Attachment data (if extracted). Uses `bytes.Bytes` for cheap cloning of large buffers. |


---

#### EmailConfig

Configuration for email extraction.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `MsgFallbackCodepage` | `*uint32` | `nil` | Windows codepage number to use when an MSG file contains no codepage property. Defaults to `nil`, which falls back to windows-1252. If an unrecognized or invalid codepage number is supplied (including 0), the behavior silently falls back to windows-1252 — the same as when the MSG file itself contains an unrecognized codepage. No error or warning is emitted. Users should verify output when supplying unusual values. Common values: - 1250: Central European (Polish, Czech, Hungarian, etc.) - 1251: Cyrillic (Russian, Ukrainian, Bulgarian, etc.) - 1252: Western European (default) - 1253: Greek - 1254: Turkish - 1255: Hebrew - 1256: Arabic - 932:  Japanese (Shift-JIS) - 936:  Simplified Chinese (GBK) |


---

#### EmailExtractionResult

Email extraction result.

Complete representation of an extracted email message (.eml or .msg)
including headers, body content, and attachments.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Subject` | `*string` | `nil` | Email subject line |
| `FromEmail` | `*string` | `nil` | Sender email address |
| `ToEmails` | `[]string` | — | Primary recipient email addresses |
| `CcEmails` | `[]string` | — | CC recipient email addresses |
| `BccEmails` | `[]string` | — | BCC recipient email addresses |
| `Date` | `*string` | `nil` | Email date/timestamp |
| `MessageId` | `*string` | `nil` | Message-ID header value |
| `PlainText` | `*string` | `nil` | Plain text version of the email body |
| `HtmlContent` | `*string` | `nil` | HTML version of the email body |
| `CleanedText` | `string` | — | Cleaned/processed text content |
| `Attachments` | `[]EmailAttachment` | — | List of email attachments |
| `Metadata` | `map[string]string` | — | Additional email headers and metadata |


---

#### EmailMetadata

Email metadata extracted from .eml and .msg files.

Includes sender/recipient information, message ID, and attachment list.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `FromEmail` | `*string` | `nil` | Sender's email address |
| `FromName` | `*string` | `nil` | Sender's display name |
| `ToEmails` | `[]string` | `nil` | Primary recipients |
| `CcEmails` | `[]string` | `nil` | CC recipients |
| `BccEmails` | `[]string` | `nil` | BCC recipients |
| `MessageId` | `*string` | `nil` | Message-ID header value |
| `Attachments` | `[]string` | `nil` | List of attachment filenames |


---

#### EmbedRequest

Embedding request for generating embeddings from text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Texts` | `[]string` | — | Text strings to generate embeddings for (at least one non-empty string required) |
| `Config` | `*EmbeddingConfig` | `nil` | Optional embedding configuration (model, batch size, etc.) |


---

#### EmbedResponse

Embedding response containing generated embeddings.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Embeddings` | `[][]float32` | — | Generated embeddings (one per input text) |
| `Model` | `string` | — | Model used for embedding generation |
| `Dimensions` | `int` | — | Dimensionality of the embeddings |
| `Count` | `int` | — | Number of embeddings generated |


---

#### EmbedTextParams

Request parameters for embedding generation.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Texts` | `[]string` | — | List of text strings to generate embeddings for |
| `Preset` | `*string` | `nil` | Embedding preset name (default: "balanced"). Available: "speed", "balanced", "quality" |
| `Model` | `*string` | `nil` | LLM model for provider-hosted embeddings (e.g., "openai/text-embedding-3-small"). When set, overrides preset and uses liter-llm for embedding generation. |
| `ApiKey` | `*string` | `nil` | API key for the LLM provider (optional, falls back to env). |
| `EmbeddingPlugin` | `*string` | `nil` | Name of a pre-registered in-process embedding plugin backend. When set, overrides both preset and model and dispatches to the registered callback. Requires a prior call to `kreuzberg.plugins.register_embedding_backend`. |


---

#### EmbeddedFile

Embedded file descriptor extracted from the PDF name tree.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Name` | `string` | — | The filename as stored in the PDF name tree. |
| `Data` | `[]byte` | — | Raw file bytes from the embedded stream. |
| `MimeType` | `*string` | `nil` | MIME type if specified in the filespec, otherwise `nil`. |


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

###### Dimensions()

Embedding vector dimension. Must be `> 0` and must match the length of
every vector returned by `embed`.

**Signature:**

```go
func (o *EmbeddingBackend) Dimensions() int
```

###### Embed()

Embed a batch of texts, returning one vector per input in order.

**Errors:**

Implementations should return `crate.KreuzbergError.Plugin` for
backend-specific failures. The dispatcher layers its own validation
(length, per-vector dimension) on top.

**Signature:**

```go
func (o *EmbeddingBackend) Embed(texts []string) ([][]float32, error)
```


---

#### EmbeddingConfig

Embedding configuration for text chunks.

Configures embedding generation using ONNX models via the vendored embedding engine.
Requires the `embeddings` feature to be enabled.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Model` | `EmbeddingModelType` | `EmbeddingModelType.Preset` | The embedding model to use (defaults to "balanced" preset if not specified) |
| `Normalize` | `bool` | `true` | Whether to normalize embedding vectors (recommended for cosine similarity) |
| `BatchSize` | `int` | `32` | Batch size for embedding generation |
| `ShowDownloadProgress` | `bool` | `false` | Show model download progress |
| `CacheDir` | `*string` | `nil` | Custom cache directory for model files Defaults to `~/.cache/kreuzberg/embeddings/` if not specified. Allows full customization of model download location. |
| `Acceleration` | `*AccelerationConfig` | `nil` | Hardware acceleration for the embedding ONNX model. When set, controls which execution provider (CPU, CUDA, CoreML, TensorRT) is used for inference. Defaults to `nil` (auto-select per platform). |
| `MaxEmbedDurationSecs` | `*uint64` | `nil` | Maximum wall-clock duration (in seconds) for a single `embed()` call when using `EmbeddingModelType.Plugin`. Applies only to the in-process plugin path — protects against hung host-language backends (e.g. a Python callback deadlocked on the GIL, a model stuck on CUDA OOM retries, etc.). On timeout, the dispatcher returns `crate.KreuzbergError.Plugin` instead of blocking forever. `nil` disables the timeout. The default (60 seconds) is conservative for common in-process inference; increase for large batches on slow hardware. |

##### Methods

###### Default()

**Signature:**

```go
func (o *EmbeddingConfig) Default() EmbeddingConfig
```


---

#### EpubMetadata

EPUB metadata (Dublin Core extensions).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Coverage` | `*string` | `nil` | Coverage |
| `DcFormat` | `*string` | `nil` | Dc format |
| `Relation` | `*string` | `nil` | Relation |
| `Source` | `*string` | `nil` | Source |
| `DcType` | `*string` | `nil` | Dc type |
| `CoverImage` | `*string` | `nil` | Cover image |


---

#### ErrorMetadata

Error metadata (for batch operations).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `ErrorType` | `string` | — | Error type |
| `Message` | `string` | — | Message |


---

#### ExcelMetadata

Excel/spreadsheet metadata.

Contains information about sheets in Excel, OpenDocument Calc, and other
spreadsheet formats (.xlsx, .xls, .ods, etc.).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `SheetCount` | `int` | — | Total number of sheets in the workbook |
| `SheetNames` | `[]string` | `nil` | Names of all sheets in order |


---

#### ExcelSheet

Single Excel worksheet.

Represents one sheet from an Excel workbook with its content
converted to Markdown format and dimensional statistics.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Name` | `string` | — | Sheet name as it appears in Excel |
| `Markdown` | `string` | — | Sheet content converted to Markdown tables |
| `RowCount` | `int` | — | Number of rows |
| `ColCount` | `int` | — | Number of columns |
| `CellCount` | `int` | — | Total number of non-empty cells |
| `TableCells` | `*[][]string` | `nil` | Pre-extracted table cells (2D vector of cell values) Populated during markdown generation to avoid re-parsing markdown. None for empty sheets. |


---

#### ExcelWorkbook

Excel workbook representation.

Contains all sheets from an Excel file (.xlsx, .xls, etc.) with
extracted content and metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Sheets` | `[]ExcelSheet` | — | All sheets in the workbook |
| `Metadata` | `map[string]string` | — | Workbook-level metadata (author, creation date, etc.) |


---

#### ExtractBytesParams

Request parameters for bytes extraction.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Data` | `string` | — | Base64-encoded file content |
| `MimeType` | `*string` | `nil` | Optional MIME type hint (auto-detected if not provided) |
| `Config` | `*interface{}` | `nil` | Extraction configuration (JSON object) |
| `PdfPassword` | `*string` | `nil` | Password for encrypted PDFs |
| `ResponseFormat` | `*string` | `nil` | Wire format for the response: "json" (default) or "toon" |


---

#### ExtractFileParams

Request parameters for file extraction.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Path` | `string` | — | Path to the file to extract |
| `MimeType` | `*string` | `nil` | Optional MIME type hint (auto-detected if not provided) |
| `Config` | `*interface{}` | `nil` | Extraction configuration (JSON object) |
| `PdfPassword` | `*string` | `nil` | Password for encrypted PDFs |
| `ResponseFormat` | `*string` | `nil` | Wire format for the response: "json" (default) or "toon" |


---

#### ExtractResponse

Extraction response (list of results).


---

#### ExtractStructuredParams

Request parameters for LLM-based structured extraction.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Path` | `string` | — | File path to extract from |
| `Schema` | `interface{}` | — | JSON schema for structured output |
| `Model` | `string` | — | LLM model (e.g., "openai/gpt-4o") |
| `SchemaName` | `string` | — | Schema name (default: "extraction") |
| `SchemaDescription` | `*string` | `nil` | Schema description for the LLM |
| `Prompt` | `*string` | `nil` | Custom Jinja2 prompt template |
| `ApiKey` | `*string` | `nil` | API key (optional, falls back to env) |
| `Strict` | `bool` | — | Enable strict mode |


---

#### ExtractedImage

Extracted image from a document.

Contains raw image data, metadata, and optional nested OCR results.
Raw bytes allow cross-language compatibility - users can convert to
PIL.Image (Python), Sharp (Node.js), or other formats as needed.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Data` | `[]byte` | — | Raw image data (PNG, JPEG, WebP, etc. bytes). Uses `bytes.Bytes` for cheap cloning of large buffers. |
| `Format` | `string` | — | Image format (e.g., "jpeg", "png", "webp") Uses Cow<'static, str> to avoid allocation for static literals. |
| `ImageIndex` | `int` | — | Zero-indexed position of this image in the document/page |
| `PageNumber` | `*int` | `nil` | Page/slide number where image was found (1-indexed) |
| `Width` | `*uint32` | `nil` | Image width in pixels |
| `Height` | `*uint32` | `nil` | Image height in pixels |
| `Colorspace` | `*string` | `nil` | Colorspace information (e.g., "RGB", "CMYK", "Gray") |
| `BitsPerComponent` | `*uint32` | `nil` | Bits per color component (e.g., 8, 16) |
| `IsMask` | `bool` | — | Whether this image is a mask image |
| `Description` | `*string` | `nil` | Optional description of the image |
| `OcrResult` | `*ExtractionResult` | `nil` | Nested OCR extraction result (if image was OCRed) When OCR is performed on this image, the result is embedded here rather than in a separate collection, making the relationship explicit. |
| `BoundingBox` | `*string` | `nil` | Bounding box of the image on the page (PDF coordinates: x0=left, y0=bottom, x1=right, y1=top). Only populated for PDF-extracted images when position data is available from pdfium. |
| `SourcePath` | `*string` | `nil` | Original source path of the image within the document archive (e.g., "media/image1.png" in DOCX). Used for rendering image references when the binary data is not extracted. |


---

#### ExtractedInlineImage

Extracted inline image with metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Data` | `[]byte` | — | Uses `bytes.Bytes` for cheap cloning of large buffers. |
| `Format` | `string` | — | Format |
| `Filename` | `*string` | `nil` | Filename |
| `Description` | `*string` | `nil` | Human-readable description |
| `Dimensions` | `*[]uint32` | `nil` | Dimensions |
| `Attributes` | `[]string` | — | Attributes |


---

#### ExtractionConfig

Main extraction configuration.

This struct contains all configuration options for the extraction process.
It can be loaded from TOML, YAML, or JSON files, or created programmatically.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `UseCache` | `bool` | `true` | Enable caching of extraction results |
| `EnableQualityProcessing` | `bool` | `true` | Enable quality post-processing |
| `Ocr` | `*OcrConfig` | `nil` | OCR configuration (None = OCR disabled) |
| `ForceOcr` | `bool` | `false` | Force OCR even for searchable PDFs |
| `ForceOcrPages` | `*[]int` | `nil` | Force OCR on specific pages only (1-indexed page numbers, must be >= 1). When set, only the listed pages are OCR'd regardless of text layer quality. Unlisted pages use native text extraction. Ignored when `force_ocr` is `true`. Only applies to PDF documents. Duplicates are automatically deduplicated. An `ocr` config is recommended for backend/language selection; defaults are used if absent. |
| `DisableOcr` | `bool` | `false` | Disable OCR entirely, even for images. When `true`, OCR is skipped for all document types. Images return metadata only (dimensions, format, EXIF) without text extraction. PDFs use only native text extraction without OCR fallback. Cannot be `true` simultaneously with `force_ocr`. *Added in v4.7.0.* |
| `Chunking` | `*ChunkingConfig` | `nil` | Text chunking configuration (None = chunking disabled) |
| `ContentFilter` | `*ContentFilterConfig` | `nil` | Content filtering configuration (None = use extractor defaults). Controls whether document "furniture" (headers, footers, watermarks, repeating text) is included in or stripped from extraction results. See `ContentFilterConfig` for per-field documentation. |
| `Images` | `*ImageExtractionConfig` | `nil` | Image extraction configuration (None = no image extraction) |
| `PdfOptions` | `*PdfConfig` | `nil` | PDF-specific options (None = use defaults) |
| `TokenReduction` | `*TokenReductionOptions` | `nil` | Token reduction configuration (None = no token reduction) |
| `LanguageDetection` | `*LanguageDetectionConfig` | `nil` | Language detection configuration (None = no language detection) |
| `Pages` | `*PageConfig` | `nil` | Page extraction configuration (None = no page tracking) |
| `Postprocessor` | `*PostProcessorConfig` | `nil` | Post-processor configuration (None = use defaults) |
| `HtmlOptions` | `*string` | `nil` | HTML to Markdown conversion options (None = use defaults) Configure how HTML documents are converted to Markdown, including heading styles, list formatting, code block styles, and preprocessing options. |
| `HtmlOutput` | `*HtmlOutputConfig` | `nil` | Styled HTML output configuration. When set alongside `output_format = OutputFormat.Html`, the extraction pipeline uses `StyledHtmlRenderer` which emits stable `kb-*` CSS class hooks on every structural element and optionally embeds theme CSS or user-supplied CSS in a `<style>` block. When `nil`, the existing plain comrak-based HTML renderer is used. |
| `ExtractionTimeoutSecs` | `*uint64` | `nil` | Default per-file timeout in seconds for batch extraction. When set, each file in a batch will be canceled after this duration unless overridden by `FileExtractionConfig.timeout_secs`. `nil` means no timeout (unbounded extraction time). |
| `MaxConcurrentExtractions` | `*int` | `nil` | Maximum concurrent extractions in batch operations (None = (num_cpus × 1.5).ceil()). Limits parallelism to prevent resource exhaustion when processing large batches. Defaults to (num_cpus × 1.5).ceil() when not set. |
| `ResultFormat` | `string` | — | Result structure format Controls whether results are returned in unified format (default) with all content in the `content` field, or element-based format with semantic elements (for Unstructured-compatible output). |
| `SecurityLimits` | `*string` | `nil` | Security limits for archive extraction. Controls maximum archive size, compression ratio, file count, and other security thresholds to prevent decompression bomb attacks. Also caps nesting depth, iteration count, entity / token length, cumulative content size, and table cell count for every extraction path that ingests user-controlled bytes. When `nil`, default limits are used. |
| `OutputFormat` | `string` | `Plain` | Content text format (default: Plain). Controls the format of the extracted content: - `Plain`: Raw extracted text (default) - `Markdown`: Markdown formatted output - `Djot`: Djot markup format (requires djot feature) - `Html`: HTML formatted output When set to a structured format, extraction results will include formatted output. The `formatted_content` field may be populated when format conversion is applied. |
| `Layout` | `*LayoutDetectionConfig` | `nil` | Layout detection configuration (None = layout detection disabled). When set, PDF pages and images are analyzed for document structure (headings, code, formulas, tables, figures, etc.) using RT-DETR models via ONNX Runtime. For PDFs, layout hints override paragraph classification in the markdown pipeline. For images, per-region OCR is performed with markdown formatting based on detected layout classes. Requires the `layout-detection` feature. |
| `IncludeDocumentStructure` | `bool` | `false` | Enable structured document tree output. When true, populates the `document` field on `ExtractionResult` with a hierarchical `DocumentStructure` containing heading-driven section nesting, table grids, content layer classification, and inline annotations. Independent of `result_format` — can be combined with Unified or ElementBased. |
| `Acceleration` | `*AccelerationConfig` | `nil` | Hardware acceleration configuration for ONNX Runtime models. Controls execution provider selection for layout detection and embedding models. When `nil`, uses platform defaults (CoreML on macOS, CUDA on Linux, CPU on Windows). |
| `CacheNamespace` | `*string` | `nil` | Cache namespace for tenant isolation. When set, cache entries are stored under `{cache_dir}/{namespace}/`. Must be alphanumeric, hyphens, or underscores only (max 64 chars). Different namespaces have isolated cache spaces on the same filesystem. |
| `CacheTtlSecs` | `*uint64` | `nil` | Per-request cache TTL in seconds. Overrides the global `max_age_days` for this specific extraction. When `0`, caching is completely skipped (no read or write). When `nil`, the global TTL applies. |
| `Email` | `*EmailConfig` | `nil` | Email extraction configuration (None = use defaults). Currently supports configuring the fallback codepage for MSG files that do not specify one. See `crate.core.config.EmailConfig` for details. |
| `Concurrency` | `*string` | `nil` | Concurrency limits for constrained environments (None = use defaults). Controls Rayon thread pool size, ONNX Runtime intra-op threads, and (when `max_concurrent_extractions` is unset) the batch concurrency semaphore. See `crate.core.config.ConcurrencyConfig` for details. |
| `MaxArchiveDepth` | `int` | — | Maximum recursion depth for archive extraction (default: 3). Set to 0 to disable recursive extraction (legacy behavior). |
| `TreeSitter` | `*TreeSitterConfig` | `nil` | Tree-sitter language pack configuration (None = tree-sitter disabled). When set, enables code file extraction using tree-sitter parsers. Controls grammar download behavior and code analysis options. |
| `StructuredExtraction` | `*StructuredExtractionConfig` | `nil` | Structured extraction via LLM (None = disabled). When set, the extracted document content is sent to an LLM with the provided JSON schema. The structured response is stored in `ExtractionResult.structured_output`. |
| `CancelToken` | `*string` | `nil` | Cancellation token for this extraction (None = no external cancellation). Pass a `CancellationToken` clone here and call `CancellationToken.cancel` from another thread / task to abort the extraction in progress. The extractor checks the token at safe checkpoints (before lock acquisition, between pages, between batch items) and returns `KreuzbergError.Cancelled` when set. The field is excluded from serialization because `CancellationToken` is a runtime handle, not a configuration value. |

##### Methods

###### Default()

**Signature:**

```go
func (o *ExtractionConfig) Default() ExtractionConfig
```

###### NeedsImageProcessing()

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

```go
func (o *ExtractionConfig) NeedsImageProcessing() bool
```


---

#### ExtractionResult

General extraction result used by the core extraction API.

This is the main result type returned by all extraction functions.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Content` | `string` | — | The extracted text content |
| `MimeType` | `string` | — | The detected MIME type |
| `Metadata` | `Metadata` | — | Document metadata |
| `ExtractionMethod` | `*ExtractionMethod` | `nil` | Extraction strategy used to produce the returned text. Populated when the extractor can reliably distinguish native text extraction, OCR-only extraction, or mixed native/OCR output. |
| `Tables` | `[]string` | `nil` | Tables extracted from the document |
| `DetectedLanguages` | `*[]string` | `nil` | Detected languages |
| `Chunks` | `*[]Chunk` | `nil` | Text chunks when chunking is enabled. When chunking configuration is provided, the content is split into overlapping chunks for efficient processing. Each chunk contains the text, optional embeddings (if enabled), and metadata about its position. |
| `Images` | `*[]ExtractedImage` | `nil` | Extracted images from the document. When image extraction is enabled via `ImageExtractionConfig`, this field contains all images found in the document with their raw data and metadata. Each image may optionally contain a nested `ocr_result` if OCR was performed. |
| `Pages` | `*[]PageContent` | `nil` | Per-page content when page extraction is enabled. When page extraction is configured, the document is split into per-page content with tables and images mapped to their respective pages. |
| `Elements` | `*[]Element` | `nil` | Semantic elements when element-based result format is enabled. When result_format is set to ElementBased, this field contains semantic elements with type classification, unique identifiers, and metadata for Unstructured-compatible element-based processing. |
| `DjotContent` | `*DjotContent` | `nil` | Rich Djot content structure (when extracting Djot documents). When extracting Djot documents with structured extraction enabled, this field contains the full semantic structure including: - Block-level elements with nesting - Inline formatting with attributes - Links, images, footnotes - Math expressions - Complete attribute information The `content` field still contains plain text for backward compatibility. Always `nil` for non-Djot documents. |
| `OcrElements` | `*[]OcrElement` | `nil` | OCR elements with full spatial and confidence metadata. When OCR is performed with element extraction enabled, this field contains the structured representation of detected text including: - Bounding geometry (rectangles or quadrilaterals) - Confidence scores (detection and recognition) - Rotation information - Hierarchical relationships (Tesseract only) This field preserves all metadata that would otherwise be lost when converting to plain text or markdown output formats. Only populated when `OcrElementConfig.include_elements` is true. |
| `Document` | `*DocumentStructure` | `nil` | Structured document tree (when document structure extraction is enabled). When `include_document_structure` is true in `ExtractionConfig`, this field contains the full hierarchical representation of the document including: - Heading-driven section nesting - Table grids with cell-level metadata - Content layer classification (body, header, footer, footnote) - Inline text annotations (formatting, links) - Bounding boxes and page numbers Independent of `result_format` — can be combined with Unified or ElementBased. |
| `QualityScore` | `*float64` | `nil` | Document quality score from quality analysis. A value between 0.0 and 1.0 indicating the overall text quality. Previously stored in `metadata.additional["quality_score"]`. |
| `ProcessingWarnings` | `[]ProcessingWarning` | `nil` | Non-fatal warnings collected during processing pipeline stages. Captures errors from optional pipeline features (embedding, chunking, language detection, output formatting) that don't prevent extraction but may indicate degraded results. Previously stored as individual keys in `metadata.additional`. |
| `Annotations` | `*[]PdfAnnotation` | `nil` | PDF annotations extracted from the document. When annotation extraction is enabled via `PdfConfig.extract_annotations`, this field contains text notes, highlights, links, stamps, and other annotations found in PDF documents. |
| `Children` | `*[]ArchiveEntry` | `nil` | Nested extraction results from archive contents. When extracting archives, each processable file inside produces its own full extraction result. Set to `nil` for non-archive formats. Use `max_archive_depth` in config to control recursion depth. |
| `Uris` | `*[]Uri` | `nil` | URIs/links discovered during document extraction. Contains hyperlinks, image references, citations, email addresses, and other URI-like references found in the document. Always extracted when present in the source document. |
| `StructuredOutput` | `*interface{}` | `nil` | Structured extraction output from LLM-based JSON schema extraction. When `structured_extraction` is configured in `ExtractionConfig`, the extracted document content is sent to a VLM with the provided JSON schema. The response is parsed and stored here as a JSON value matching the schema. |
| `CodeIntelligence` | `*string` | `nil` | Code intelligence results from tree-sitter analysis. Populated when extracting source code files with the `tree-sitter` feature. Contains metrics, structural analysis, imports/exports, comments, docstrings, symbols, diagnostics, and optionally chunked code segments. |
| `LlmUsage` | `*[]LlmUsage` | `nil` | LLM token usage and cost data for all LLM calls made during this extraction. Contains one entry per LLM call. Multiple entries are produced when VLM OCR, structured extraction, and/or LLM embeddings all run during the same extraction. `nil` when no LLM was used. |
| `FormattedContent` | `*string` | `nil` | Pre-rendered content in the requested output format. Populated during `derive_extraction_result` before tree derivation consumes element data. `apply_output_format` swaps this into `content` at the end of the pipeline, after post-processors have operated on plain text. |
| `OcrInternalDocument` | `*string` | `nil` | Structured hOCR document for the OCR+layout pipeline. When tesseract produces hOCR output, the parsed `InternalDocument` carries paragraph structure with bounding boxes and confidence scores. The layout classification step enriches these elements before final rendering. |


---

#### FictionBookMetadata

FictionBook (FB2) metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Genres` | `[]string` | `nil` | Genres |
| `Sequences` | `[]string` | `nil` | Sequences |
| `Annotation` | `*string` | `nil` | Annotation |


---

#### FileExtractionConfig

Per-file extraction configuration overrides for batch processing.

All fields are `Option<T>` — `nil` means "use the batch-level default."
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
| `EnableQualityProcessing` | `*bool` | `nil` | Override quality post-processing for this file. |
| `Ocr` | `*OcrConfig` | `nil` | Override OCR configuration for this file (None in the Option = use batch default). |
| `ForceOcr` | `*bool` | `nil` | Override force OCR for this file. |
| `ForceOcrPages` | `*[]int` | `nil` | Override force OCR pages for this file (1-indexed page numbers). |
| `DisableOcr` | `*bool` | `nil` | Override disable OCR for this file. |
| `Chunking` | `*ChunkingConfig` | `nil` | Override chunking configuration for this file. |
| `ContentFilter` | `*ContentFilterConfig` | `nil` | Override content filtering configuration for this file. |
| `Images` | `*ImageExtractionConfig` | `nil` | Override image extraction configuration for this file. |
| `PdfOptions` | `*PdfConfig` | `nil` | Override PDF options for this file. |
| `TokenReduction` | `*TokenReductionOptions` | `nil` | Override token reduction for this file. |
| `LanguageDetection` | `*LanguageDetectionConfig` | `nil` | Override language detection for this file. |
| `Pages` | `*PageConfig` | `nil` | Override page extraction for this file. |
| `Postprocessor` | `*PostProcessorConfig` | `nil` | Override post-processor for this file. |
| `HtmlOptions` | `*string` | `nil` | Override HTML conversion options for this file. |
| `ResultFormat` | `*string` | `nil` | Override result format for this file. |
| `OutputFormat` | `*string` | `nil` | Override output content format for this file. |
| `IncludeDocumentStructure` | `*bool` | `nil` | Override document structure output for this file. |
| `Layout` | `*LayoutDetectionConfig` | `nil` | Override layout detection for this file. |
| `TimeoutSecs` | `*uint64` | `nil` | Override per-file extraction timeout in seconds. When set, the extraction for this file will be canceled after the specified duration. A timed-out file produces an error result without affecting other files in the batch. |
| `TreeSitter` | `*TreeSitterConfig` | `nil` | Override tree-sitter configuration for this file. |
| `StructuredExtraction` | `*StructuredExtractionConfig` | `nil` | Override structured extraction configuration for this file. When set, enables LLM-based structured extraction with a JSON schema for this specific file. The extracted content is sent to a VLM/LLM and the response is parsed according to the provided schema. |


---

#### Footnote

Footnote in Djot.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Label` | `string` | — | Footnote label |
| `Content` | `[]FormattedBlock` | — | Footnote content blocks |


---

#### FormattedBlock

Block-level element in a Djot document.

Represents structural elements like headings, paragraphs, lists, code blocks, etc.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `BlockType` | `BlockType` | — | Type of block element |
| `Level` | `*int` | `nil` | Heading level (1-6) for headings, or nesting level for lists |
| `InlineContent` | `[]InlineElement` | — | Inline content within the block |
| `Attributes` | `*string` | `nil` | Element attributes (classes, IDs, key-value pairs) |
| `Language` | `*string` | `nil` | Language identifier for code blocks |
| `Code` | `*string` | `nil` | Raw code content for code blocks |
| `Children` | `[]FormattedBlock` | — | Nested blocks for containers (blockquotes, list items, divs) |


---

#### GridCell

Individual grid cell with position and span metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Content` | `string` | — | Cell text content. |
| `Row` | `uint32` | — | Zero-indexed row position. |
| `Col` | `uint32` | — | Zero-indexed column position. |
| `RowSpan` | `uint32` | — | Number of rows this cell spans. |
| `ColSpan` | `uint32` | — | Number of columns this cell spans. |
| `IsHeader` | `bool` | — | Whether this is a header cell. |
| `Bbox` | `*string` | `nil` | Bounding box for this cell (if available). |


---

#### HeaderFooter

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Paragraphs` | `[]string` | `nil` | Paragraphs |
| `Tables` | `[]string` | `nil` | Tables extracted from the document |
| `HeaderType` | `string` | — | Header type |


---

#### HeaderMetadata

Header/heading element metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Level` | `uint8` | — | Header level: 1 (h1) through 6 (h6) |
| `Text` | `string` | — | Normalized text content of the header |
| `Id` | `*string` | `nil` | HTML id attribute if present |
| `Depth` | `int` | — | Document tree depth at the header element |
| `HtmlOffset` | `int` | — | Byte offset in original HTML document |


---

#### HeadingContext

Heading context for a chunk within a Markdown document.

Contains the heading hierarchy from document root to this chunk's section.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Headings` | `[]HeadingLevel` | — | The heading hierarchy from document root to this chunk's section. Index 0 is the outermost (h1), last element is the most specific. |


---

#### HeadingLevel

A single heading in the hierarchy.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Level` | `uint8` | — | Heading depth (1 = h1, 2 = h2, etc.) |
| `Text` | `string` | — | The text content of the heading. |


---

#### HealthResponse

Health check response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Status` | `string` | — | Health status |
| `Version` | `string` | — | API version |
| `Plugins` | `*string` | `nil` | Plugin status (optional) |


---

#### HierarchicalBlock

A text block with hierarchy level assignment.

Represents a block of text with semantic heading information extracted from
font size clustering and hierarchical analysis.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Text` | `string` | — | The text content of this block |
| `FontSize` | `float32` | — | The font size of the text in this block |
| `Level` | `string` | — | The hierarchy level of this block (H1-H6 or Body) Levels correspond to HTML heading tags: - "h1": Top-level heading - "h2": Secondary heading - "h3": Tertiary heading - "h4": Quaternary heading - "h5": Quinary heading - "h6": Senary heading - "body": Body text (no heading level) |
| `Bbox` | `*[]float32` | `nil` | Bounding box information for the block Contains coordinates as (left, top, right, bottom) in PDF units. |


---

#### HierarchyConfig

Hierarchy extraction configuration for PDF text structure analysis.

Enables extraction of document hierarchy levels (H1-H6) based on font size
clustering and semantic analysis. When enabled, hierarchical blocks are
included in page content.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Enabled` | `bool` | `true` | Enable hierarchy extraction |
| `KClusters` | `int` | `3` | Number of font size clusters to use for hierarchy levels (1-7) Default: 6, which provides H1-H6 heading levels with body text. Larger values create more fine-grained hierarchy levels. |
| `IncludeBbox` | `bool` | `true` | Include bounding box information in hierarchy blocks |
| `OcrCoverageThreshold` | `*float32` | `nil` | OCR coverage threshold for smart OCR triggering (0.0-1.0) Determines when OCR should be triggered based on text block coverage. OCR is triggered when text blocks cover less than this fraction of the page. Default: 0.5 (trigger OCR if less than 50% of page has text) |

##### Methods

###### Default()

**Signature:**

```go
func (o *HierarchyConfig) Default() HierarchyConfig
```


---

#### HtmlExtractionResult

Result of HTML extraction with optional images and warnings.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Markdown` | `string` | — | Markdown |
| `Images` | `[]ExtractedInlineImage` | — | Images extracted from the document |
| `Warnings` | `[]string` | — | Warnings |


---

#### HtmlMetadata

HTML metadata extracted from HTML documents.

Includes document-level metadata, Open Graph data, Twitter Card metadata,
and extracted structural elements (headers, links, images, structured data).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Title` | `*string` | `nil` | Document title from `<title>` tag |
| `Description` | `*string` | `nil` | Document description from `<meta name="description">` tag |
| `Keywords` | `[]string` | `nil` | Document keywords from `<meta name="keywords">` tag, split on commas |
| `Author` | `*string` | `nil` | Document author from `<meta name="author">` tag |
| `CanonicalUrl` | `*string` | `nil` | Canonical URL from `<link rel="canonical">` tag |
| `BaseHref` | `*string` | `nil` | Base URL from `<base href="">` tag for resolving relative URLs |
| `Language` | `*string` | `nil` | Document language from `lang` attribute |
| `TextDirection` | `*TextDirection` | `nil` | Document text direction from `dir` attribute |
| `OpenGraph` | `map[string]string` | `nil` | Open Graph metadata (og:* properties) for social media Keys like "title", "description", "image", "url", etc. |
| `TwitterCard` | `map[string]string` | `nil` | Twitter Card metadata (twitter:* properties) Keys like "card", "site", "creator", "title", "description", "image", etc. |
| `MetaTags` | `map[string]string` | `nil` | Additional meta tags not covered by specific fields Keys are meta name/property attributes, values are content |
| `Headers` | `[]HeaderMetadata` | `nil` | Extracted header elements with hierarchy |
| `Links` | `[]LinkMetadata` | `nil` | Extracted hyperlinks with type classification |
| `Images` | `[]ImageMetadataType` | `nil` | Extracted images with source and dimensions |
| `StructuredData` | `[]StructuredData` | `nil` | Extracted structured data blocks |

##### Methods

###### From()

**Signature:**

```go
func (o *HtmlMetadata) From(metadata HtmlMetadata) HtmlMetadata
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
| `Css` | `*string` | `nil` | Inline CSS string injected into the output after the theme stylesheet. Concatenated after `css_file` content when both are set. |
| `CssFile` | `*string` | `nil` | Path to a CSS file loaded once at renderer construction time. Concatenated before `css` when both are set. |
| `Theme` | `HtmlTheme` | `HtmlTheme.Unstyled` | Built-in colour/typography theme. Default: `HtmlTheme.Unstyled`. |
| `ClassPrefix` | `string` | — | CSS class prefix applied to every emitted class name. Default: `"kb-"`. Change this if your host application already uses classes that start with `kb-`. |
| `EmbedCss` | `bool` | `true` | When `true` (default), write the resolved CSS into a `<style>` block immediately after the opening `<div class="{prefix}doc">`. Set to `false` to emit only the structural markup and wire up your own stylesheet targeting the `kb-*` class names. |

##### Methods

###### Default()

**Signature:**

```go
func (o *HtmlOutputConfig) Default() HtmlOutputConfig
```


---

#### ImageExtractionConfig

Image extraction configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `ExtractImages` | `bool` | `true` | Extract images from documents |
| `TargetDpi` | `int32` | `300` | Target DPI for image normalization |
| `MaxImageDimension` | `int32` | `4096` | Maximum dimension for images (width or height) |
| `InjectPlaceholders` | `bool` | `true` | Whether to inject image reference placeholders into markdown output. When `true` (default), image references like `![Image 1](embedded:p1_i0)` are appended to the markdown. Set to `false` to extract images as data without polluting the markdown output. |
| `AutoAdjustDpi` | `bool` | `true` | Automatically adjust DPI based on image content |
| `MinDpi` | `int32` | `72` | Minimum DPI threshold |
| `MaxDpi` | `int32` | `600` | Maximum DPI threshold |
| `MaxImagesPerPage` | `*uint32` | `nil` | Maximum number of image objects to extract per PDF page. Some PDFs (e.g. technical diagrams stored as thousands of raster fragments) can trigger extremely long or indefinite extraction times when every image object on a dense page is decoded individually via pdfium FFI. Setting this limit causes kreuzberg to stop collecting individual images once the count per page reaches the cap and emit a warning instead. `nil` (default) means no limit — all images are extracted. |

##### Methods

###### Default()

**Signature:**

```go
func (o *ImageExtractionConfig) Default() ImageExtractionConfig
```


---

#### ImageMetadataType

Image element metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Src` | `string` | — | Image source (URL, data URI, or SVG content) |
| `Alt` | `*string` | `nil` | Alternative text from alt attribute |
| `Title` | `*string` | `nil` | Title attribute |
| `Dimensions` | `*[]uint32` | `nil` | Image dimensions as (width, height) if available |
| `ImageType` | `ImageType` | — | Image type classification |
| `Attributes` | `[]string` | — | Additional attributes as key-value pairs |


---

#### ImageOcrResult

Result of OCR extraction from an image with optional page tracking.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Content` | `string` | — | Extracted text content |
| `Boundaries` | `*[]PageBoundary` | `nil` | Character byte boundaries per frame (for multi-frame TIFFs) |
| `PageContents` | `*[]PageContent` | `nil` | Per-frame content information |


---

#### ImagePreprocessingConfig

Image preprocessing configuration for OCR.

These settings control how images are preprocessed before OCR to improve
text recognition quality. Different preprocessing strategies work better
for different document types.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `TargetDpi` | `int32` | `300` | Target DPI for the image (300 is standard, 600 for small text). |
| `AutoRotate` | `bool` | `true` | Auto-detect and correct image rotation. |
| `Deskew` | `bool` | `true` | Correct skew (tilted images). |
| `Denoise` | `bool` | `false` | Remove noise from the image. |
| `ContrastEnhance` | `bool` | `false` | Enhance contrast for better text visibility. |
| `BinarizationMethod` | `string` | `"otsu"` | Binarization method: "otsu", "sauvola", "adaptive". |
| `InvertColors` | `bool` | `false` | Invert colors (white text on black → black on white). |

##### Methods

###### Default()

**Signature:**

```go
func (o *ImagePreprocessingConfig) Default() ImagePreprocessingConfig
```


---

#### ImagePreprocessingMetadata

Image preprocessing metadata.

Tracks the transformations applied to an image during OCR preprocessing,
including DPI normalization, resizing, and resampling.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `OriginalDimensions` | `[]int` | — | Original image dimensions (width, height) in pixels |
| `OriginalDpi` | `[]float64` | — | Original image DPI (horizontal, vertical) |
| `TargetDpi` | `int32` | — | Target DPI from configuration |
| `ScaleFactor` | `float64` | — | Scaling factor applied to the image |
| `AutoAdjusted` | `bool` | — | Whether DPI was auto-adjusted based on content |
| `FinalDpi` | `int32` | — | Final DPI after processing |
| `NewDimensions` | `*[]int` | `nil` | New dimensions after resizing (if resized) |
| `ResampleMethod` | `string` | — | Resampling algorithm used ("LANCZOS3", "CATMULLROM", etc.) |
| `DimensionClamped` | `bool` | — | Whether dimensions were clamped to max_image_dimension |
| `CalculatedDpi` | `*int32` | `nil` | Calculated optimal DPI (if auto_adjust_dpi enabled) |
| `SkippedResize` | `bool` | — | Whether resize was skipped (dimensions already optimal) |
| `ResizeError` | `*string` | `nil` | Error message if resize failed |


---

#### InfoResponse

Server information response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Version` | `string` | — | API version |
| `RustBackend` | `bool` | — | Whether using Rust backend |


---

#### InlineElement

Inline element within a block.

Represents text with formatting, links, images, etc.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `ElementType` | `InlineType` | — | Type of inline element |
| `Content` | `string` | — | Text content |
| `Attributes` | `*string` | `nil` | Element attributes |
| `Metadata` | `*map[string]string` | `nil` | Additional metadata (e.g., href for links, src/alt for images) |


---

#### JatsMetadata

JATS (Journal Article Tag Suite) metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Copyright` | `*string` | `nil` | Copyright |
| `License` | `*string` | `nil` | License |
| `HistoryDates` | `map[string]string` | `nil` | History dates |
| `ContributorRoles` | `[]ContributorRole` | `nil` | Contributor roles |


---

#### Keyword

Extracted keyword with metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Text` | `string` | — | The keyword text. |
| `Score` | `float32` | — | Relevance score (higher is better, algorithm-specific range). |
| `Algorithm` | `KeywordAlgorithm` | — | Algorithm that extracted this keyword. |
| `Positions` | `*[]int` | `nil` | Optional positions where keyword appears in text (character offsets). |


---

#### KeywordConfig

Keyword extraction configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Algorithm` | `KeywordAlgorithm` | `KeywordAlgorithm.Yake` | Algorithm to use for extraction. |
| `MaxKeywords` | `int` | `10` | Maximum number of keywords to extract (default: 10). |
| `MinScore` | `float32` | `0` | Minimum score threshold (0.0-1.0, default: 0.0). Keywords with scores below this threshold are filtered out. Note: Score ranges differ between algorithms. |
| `NgramRange` | `[]int` | `nil` | N-gram range for keyword extraction (min, max). (1, 1) = unigrams only (1, 2) = unigrams and bigrams (1, 3) = unigrams, bigrams, and trigrams (default) |
| `Language` | `*string` | `nil` | Language code for stopword filtering (e.g., "en", "de", "fr"). If None, no stopword filtering is applied. |
| `YakeParams` | `*YakeParams` | `nil` | YAKE-specific tuning parameters. |
| `RakeParams` | `*RakeParams` | `nil` | RAKE-specific tuning parameters. |

##### Methods

###### Default()

**Signature:**

```go
func (o *KeywordConfig) Default() KeywordConfig
```


---

#### LanguageDetectionConfig

Language detection configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Enabled` | `bool` | — | Enable language detection |
| `MinConfidence` | `float64` | — | Minimum confidence threshold (0.0-1.0) |
| `DetectMultiple` | `bool` | — | Detect multiple languages in the document |


---

#### LayoutDetection

A single layout detection result.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `ClassName` | `LayoutClass` | — | Class name (layout class) |
| `Confidence` | `float32` | — | Confidence |
| `Bbox` | `BBox` | — | Bbox (b box) |


---

#### LayoutDetectionConfig

Layout detection configuration.

Controls layout detection behavior in the extraction pipeline.
When set on `ExtractionConfig`, layout detection
is enabled for PDF extraction.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `ConfidenceThreshold` | `*float32` | `nil` | Confidence threshold override (None = use model default). |
| `ApplyHeuristics` | `bool` | `true` | Whether to apply postprocessing heuristics (default: true). |
| `TableModel` | `TableModel` | `TableModel.Tatr` | Table structure recognition model. Controls which model is used for table cell detection within layout-detected table regions. Defaults to `TableModel.Tatr`. |
| `Acceleration` | `*AccelerationConfig` | `nil` | Hardware acceleration for ONNX models (layout detection + table structure). When set, controls which execution provider (CPU, CUDA, CoreML, TensorRT) is used for inference. Defaults to `nil` (auto-select per platform). |

##### Methods

###### Default()

**Signature:**

```go
func (o *LayoutDetectionConfig) Default() LayoutDetectionConfig
```


---

#### LayoutRegion

A detected layout region on a page.

When layout detection is enabled, each page may have layout regions
identifying different content types (text, pictures, tables, etc.)
with confidence scores and spatial positions.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `ClassName` | `string` | — | Layout class name (e.g. "picture", "table", "text", "section_header"). |
| `Confidence` | `float64` | — | Confidence score from the layout detection model (0.0 to 1.0). |
| `BoundingBox` | `string` | — | Bounding box in document coordinate space. |
| `AreaFraction` | `float64` | — | Fraction of the page area covered by this region (0.0 to 1.0). |


---

#### LinkMetadata

Link element metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Href` | `string` | — | The href URL value |
| `Text` | `string` | — | Link text content (normalized) |
| `Title` | `*string` | `nil` | Optional title attribute |
| `LinkType` | `LinkType` | — | Link type classification |
| `Rel` | `[]string` | — | Rel attribute values |
| `Attributes` | `[]string` | — | Additional attributes as key-value pairs |


---

#### LlmConfig

Configuration for an LLM provider/model via liter-llm.

Each feature (VLM OCR, VLM embeddings, structured extraction) carries
its own `LlmConfig`, allowing different providers per feature.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Model` | `string` | — | Provider/model string using liter-llm routing format. Examples: `"openai/gpt-4o"`, `"anthropic/claude-sonnet-4-20250514"`, `"groq/llama-3.1-70b-versatile"`. |
| `ApiKey` | `*string` | `nil` | API key for the provider. When `nil`, liter-llm falls back to the provider's standard environment variable (e.g., `OPENAI_API_KEY`). |
| `BaseUrl` | `*string` | `nil` | Custom base URL override for the provider endpoint. |
| `TimeoutSecs` | `*uint64` | `nil` | Request timeout in seconds (default: 60). |
| `MaxRetries` | `*uint32` | `nil` | Maximum retry attempts (default: 3). |
| `Temperature` | `*float64` | `nil` | Sampling temperature for generation tasks. |
| `MaxTokens` | `*uint64` | `nil` | Maximum tokens to generate. |


---

#### LlmUsage

Token usage and cost data for a single LLM call made during extraction.

Populated when VLM OCR, structured extraction, or LLM-based embeddings
are used. Multiple entries may be present when multiple LLM calls occur
within one extraction (e.g. VLM OCR + structured extraction).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Model` | `string` | — | The LLM model identifier (e.g. "openai/gpt-4o", "anthropic/claude-sonnet-4-20250514"). |
| `Source` | `string` | — | The pipeline stage that triggered this LLM call (e.g. "vlm_ocr", "structured_extraction", "embeddings"). |
| `InputTokens` | `*uint64` | `nil` | Number of input/prompt tokens consumed. |
| `OutputTokens` | `*uint64` | `nil` | Number of output/completion tokens generated. |
| `TotalTokens` | `*uint64` | `nil` | Total tokens (input + output). |
| `EstimatedCost` | `*float64` | `nil` | Estimated cost in USD based on the provider's published pricing. |
| `FinishReason` | `*string` | `nil` | Why the model stopped generating (e.g. "stop", "length", "content_filter"). |


---

#### ManifestEntryResponse

Model manifest entry for cache management.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `RelativePath` | `string` | — | Relative path within the cache directory |
| `Sha256` | `string` | — | SHA256 checksum of the model file |
| `SizeBytes` | `uint64` | — | Expected file size in bytes |
| `SourceUrl` | `string` | — | HuggingFace source URL for downloading |


---

#### ManifestResponse

Model manifest response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `KreuzbergVersion` | `string` | — | Kreuzberg version |
| `TotalSizeBytes` | `uint64` | — | Total size of all models in bytes |
| `ModelCount` | `int` | — | Number of models in the manifest |
| `Models` | `[]ManifestEntryResponse` | — | Individual model entries |


---

#### MergedChunk

A merged chunk produced by `merge_segments`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Text` | `string` | — | Text |
| `ByteStart` | `int` | — | Byte start |
| `ByteEnd` | `int` | — | Byte end |


---

#### Metadata

Extraction result metadata.

Contains common fields applicable to all formats, format-specific metadata
via a discriminated union, and additional custom fields from postprocessors.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Title` | `*string` | `nil` | Document title |
| `Subject` | `*string` | `nil` | Document subject or description |
| `Authors` | `*[]string` | `nil` | Primary author(s) - always Vec for consistency |
| `Keywords` | `*[]string` | `nil` | Keywords/tags - always Vec for consistency |
| `Language` | `*string` | `nil` | Primary language (ISO 639 code) |
| `CreatedAt` | `*string` | `nil` | Creation timestamp (ISO 8601 format) |
| `ModifiedAt` | `*string` | `nil` | Last modification timestamp (ISO 8601 format) |
| `CreatedBy` | `*string` | `nil` | User who created the document |
| `ModifiedBy` | `*string` | `nil` | User who last modified the document |
| `Pages` | `*PageStructure` | `nil` | Page/slide/sheet structure with boundaries |
| `Format` | `*FormatMetadata` | `nil` | Format-specific metadata (discriminated union) Contains detailed metadata specific to the document format. Serializes with a `format_type` discriminator field. |
| `ImagePreprocessing` | `*ImagePreprocessingMetadata` | `nil` | Image preprocessing metadata (when OCR preprocessing was applied) |
| `JsonSchema` | `*interface{}` | `nil` | JSON schema (for structured data extraction) |
| `Error` | `*ErrorMetadata` | `nil` | Error metadata (for batch operations) |
| `ExtractionDurationMs` | `*uint64` | `nil` | Extraction duration in milliseconds (for benchmarking). This field is populated by batch extraction to provide per-file timing information. It's `nil` for single-file extraction (which uses external timing). |
| `Category` | `*string` | `nil` | Document category (from frontmatter or classification). |
| `Tags` | `*[]string` | `nil` | Document tags (from frontmatter). |
| `DocumentVersion` | `*string` | `nil` | Document version string (from frontmatter). |
| `AbstractText` | `*string` | `nil` | Abstract or summary text (from frontmatter). |
| `OutputFormat` | `*string` | `nil` | Output format identifier (e.g., "markdown", "html", "text"). Set by the output format pipeline stage when format conversion is applied. Previously stored in `metadata.additional["output_format"]`. |
| `Additional` | `string` | — | Additional custom fields from postprocessors. **Deprecated**: Prefer using typed fields on `ExtractionResult` and `Metadata` instead of inserting into this map. Typed fields provide better cross-language compatibility and type safety. This field will be removed in a future major version. This flattened map allows Python/TypeScript postprocessors to add arbitrary fields (entity extraction, keyword extraction, etc.). Fields are merged at the root level during serialization. Uses `Cow<'static, str>` keys so static string keys avoid allocation. |


---

#### ModelPaths

Combined paths to all models needed for OCR (backward compatibility).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `DetModel` | `string` | — | Path to the detection model directory. |
| `ClsModel` | `string` | — | Path to the classification model directory. |
| `RecModel` | `string` | — | Path to the recognition model directory. |
| `DictFile` | `string` | — | Path to the character dictionary file. |


---

#### Note

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Id` | `string` | — | Unique identifier |
| `NoteType` | `string` | — | Note type |
| `Paragraphs` | `[]string` | — | Paragraphs |


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

###### ProcessImage()

Process an image and extract text via OCR.

**Returns:**

An `ExtractionResult` containing the extracted text and metadata.

**Errors:**

- `KreuzbergError.Ocr` - OCR processing failed
- `KreuzbergError.Validation` - Invalid image format or configuration
- `KreuzbergError.Io` - I/O errors (these always bubble up)

**Signature:**

```go
func (o *OcrBackend) ProcessImage(imageBytes []byte, config OcrConfig) (ExtractionResult, error)
```

###### ProcessImageFile()

Process a file and extract text via OCR.

Default implementation reads the file and calls `process_image`.
Override for custom file handling or optimizations.

**Errors:**

Same as `process_image`, plus file I/O errors.

**Signature:**

```go
func (o *OcrBackend) ProcessImageFile(path string, config OcrConfig) (ExtractionResult, error)
```

###### SupportsLanguage()

Check if this backend supports a given language code.

**Returns:**

`true` if the language is supported, `false` otherwise.

**Signature:**

```go
func (o *OcrBackend) SupportsLanguage(lang string) bool
```

###### BackendType()

Get the backend type identifier.

**Returns:**

The backend type enum value.

**Signature:**

```go
func (o *OcrBackend) BackendType() OcrBackendType
```

###### SupportedLanguages()

Optional: Get a list of all supported languages.

Defaults to empty list. Override to provide comprehensive language support info.

**Signature:**

```go
func (o *OcrBackend) SupportedLanguages() []string
```

###### SupportsTableDetection()

Optional: Check if the backend supports table detection.

Defaults to `false`. Override if your backend can detect and extract tables.

**Signature:**

```go
func (o *OcrBackend) SupportsTableDetection() bool
```

###### SupportsDocumentProcessing()

Check if the backend supports direct document-level processing (e.g. for PDFs).

Defaults to `false`. Override if the backend has optimized document processing.

**Signature:**

```go
func (o *OcrBackend) SupportsDocumentProcessing() bool
```

###### ProcessDocument()

Process a document file directly via OCR.

Only called if `supports_document_processing` returns `true`.

**Signature:**

```go
func (o *OcrBackend) ProcessDocument(path string, config OcrConfig) (ExtractionResult, error)
```


---

#### OcrCacheStats

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `TotalFiles` | `int` | — | Total files |
| `TotalSizeMb` | `float64` | — | Total size mb |


---

#### OcrConfidence

Confidence scores for an OCR element.

Separates detection confidence (how confident that text exists at this location)
from recognition confidence (how confident about the actual text content).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Detection` | `*float64` | `nil` | Detection confidence: how confident the OCR engine is that text exists here. PaddleOCR provides this as `box_score`, Tesseract doesn't have a direct equivalent. Range: 0.0 to 1.0 (or None if not available). |
| `Recognition` | `float64` | — | Recognition confidence: how confident about the text content. Range: 0.0 to 1.0. |


---

#### OcrConfig

OCR configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Enabled` | `bool` | `true` | Whether OCR is enabled. Setting `enabled: false` is a shorthand for `disable_ocr: true` on the parent `ExtractionConfig`. Images return metadata only; PDFs use native text extraction without OCR fallback. Defaults to `true`. When `false`, all other OCR settings are ignored. |
| `Backend` | `string` | — | OCR backend: tesseract, easyocr, paddleocr |
| `Language` | `string` | — | Language code (e.g., "eng", "deu") |
| `TesseractConfig` | `*TesseractConfig` | `nil` | Tesseract-specific configuration (optional) |
| `OutputFormat` | `*string` | `nil` | Output format for OCR results (optional, for format conversion) |
| `PaddleOcrConfig` | `*interface{}` | `nil` | PaddleOCR-specific configuration (optional, JSON passthrough) |
| `ElementConfig` | `*OcrElementConfig` | `nil` | OCR element extraction configuration |
| `QualityThresholds` | `*OcrQualityThresholds` | `nil` | Quality thresholds for the native-text-to-OCR fallback decision. When None, uses compiled defaults (matching previous hardcoded behavior). |
| `Pipeline` | `*OcrPipelineConfig` | `nil` | Multi-backend OCR pipeline configuration. When set, enables weighted fallback across multiple OCR backends based on output quality. When None, uses the single `backend` field (same as today). |
| `AutoRotate` | `bool` | `false` | Enable automatic page rotation based on orientation detection. When enabled, uses Tesseract's `DetectOrientationScript()` to detect page orientation (0/90/180/270 degrees) before OCR. If the page is rotated with high confidence, the image is corrected before recognition. This is critical for handling rotated scanned documents. |
| `VlmConfig` | `*LlmConfig` | `nil` | VLM (Vision Language Model) OCR configuration. Required when `backend` is `"vlm"`. Uses liter-llm to send page images to a vision model for text extraction. |
| `VlmPrompt` | `*string` | `nil` | Custom Jinja2 prompt template for VLM OCR. When `nil`, uses the default template. Available variables: - `{{ language }}` — The document language code (e.g., "eng", "deu"). |
| `Acceleration` | `*AccelerationConfig` | `nil` | Hardware acceleration for ONNX Runtime models (e.g. PaddleOCR, layout detection). Not user-configurable via config files — injected at runtime from `ExtractionConfig.acceleration` before each `process_image` call. |

##### Methods

###### Default()

**Signature:**

```go
func (o *OcrConfig) Default() OcrConfig
```


---

#### OcrElement

A unified OCR element representing detected text with full metadata.

This is the primary type for structured OCR output, preserving all information
from both Tesseract and PaddleOCR backends.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Text` | `string` | — | The recognized text content. |
| `Geometry` | `OcrBoundingGeometry` | `OcrBoundingGeometry.Rectangle` | Bounding geometry (rectangle or quadrilateral). |
| `Confidence` | `OcrConfidence` | — | Confidence scores for detection and recognition. |
| `Level` | `OcrElementLevel` | `OcrElementLevel.Line` | Hierarchical level (word, line, block, page). |
| `Rotation` | `*OcrRotation` | `nil` | Rotation information (if detected). |
| `PageNumber` | `int` | — | Page number (1-indexed). |
| `ParentId` | `*string` | `nil` | Parent element ID for hierarchical relationships. Only used for Tesseract output which has word -> line -> block hierarchy. |
| `BackendMetadata` | `map[string]interface{}` | `nil` | Backend-specific metadata that doesn't fit the unified schema. |


---

#### OcrElementConfig

Configuration for OCR element extraction.

Controls how OCR elements are extracted and filtered.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `IncludeElements` | `bool` | — | Whether to include OCR elements in the extraction result. When true, the `ocr_elements` field in `ExtractionResult` will be populated. |
| `MinLevel` | `OcrElementLevel` | `OcrElementLevel.Line` | Minimum hierarchical level to include. Elements below this level (e.g., words when min_level is Line) will be excluded. |
| `MinConfidence` | `float64` | — | Minimum recognition confidence threshold (0.0-1.0). Elements with confidence below this threshold will be filtered out. |
| `BuildHierarchy` | `bool` | — | Whether to build hierarchical relationships between elements. When true, `parent_id` fields will be populated based on spatial containment. Only meaningful for Tesseract output. |


---

#### OcrExtractionResult

OCR extraction result.

Result of performing OCR on an image or scanned document,
including recognized text and detected tables.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Content` | `string` | — | Recognized text content |
| `MimeType` | `string` | — | Original MIME type of the processed image |
| `Metadata` | `map[string]interface{}` | — | OCR processing metadata (confidence scores, language, etc.) |
| `Tables` | `[]OcrTable` | — | Tables detected and extracted via OCR |
| `OcrElements` | `*[]OcrElement` | `nil` | Structured OCR elements with bounding boxes and confidence scores. Available when TSV output is requested or table detection is enabled. |
| `InternalDocument` | `*string` | `nil` | Structured document produced from hOCR parsing. Carries paragraph structure, bounding boxes, and confidence scores that the flattened `content` string discards. |


---

#### OcrMetadata

OCR processing metadata.

Captures information about OCR processing configuration and results.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Language` | `string` | — | OCR language code(s) used |
| `Psm` | `int32` | — | Tesseract Page Segmentation Mode (PSM) |
| `OutputFormat` | `string` | — | Output format (e.g., "text", "hocr") |
| `TableCount` | `int` | — | Number of tables detected |
| `TableRows` | `*int` | `nil` | Table rows |
| `TableCols` | `*int` | `nil` | Table cols |


---

#### OcrPipelineConfig

Multi-backend OCR pipeline with quality-based fallback.

Backends are tried in priority order (highest first). After each backend
produces output, quality is evaluated. If it meets `quality_thresholds.pipeline_min_quality`,
the result is accepted. Otherwise the next backend is tried.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Stages` | `[]OcrPipelineStage` | — | Ordered list of backends to try. Sorted by priority (descending) at runtime. |
| `QualityThresholds` | `OcrQualityThresholds` | — | Quality thresholds for deciding whether to accept a result or try the next backend. |


---

#### OcrPipelineStage

A single backend stage in the OCR pipeline.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Backend` | `string` | — | Backend name: "tesseract", "paddleocr", "easyocr", or a custom registered name. |
| `Priority` | `uint32` | — | Priority weight (higher = tried first). Stages are sorted by priority descending. |
| `Language` | `*string` | `nil` | Language override for this stage (None = use parent OcrConfig.language). |
| `TesseractConfig` | `*TesseractConfig` | `nil` | Tesseract-specific config override for this stage. |
| `PaddleOcrConfig` | `*interface{}` | `nil` | PaddleOCR-specific config for this stage. |
| `VlmConfig` | `*LlmConfig` | `nil` | VLM config override for this pipeline stage. |


---

#### OcrQualityThresholds

Quality thresholds for OCR fallback decisions and pipeline quality gating.

All fields default to the values that match the previous hardcoded behavior,
so `OcrQualityThresholds.default()` preserves existing semantics exactly.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `MinTotalNonWhitespace` | `int` | `64` | Minimum total non-whitespace characters to consider text substantive. |
| `MinNonWhitespacePerPage` | `float64` | `32` | Minimum non-whitespace characters per page on average. |
| `MinMeaningfulWordLen` | `int` | `4` | Minimum character count for a word to be "meaningful". |
| `MinMeaningfulWords` | `int` | `3` | Minimum count of meaningful words before text is accepted. |
| `MinAlnumRatio` | `float64` | `0.3` | Minimum alphanumeric ratio (non-whitespace chars that are alphanumeric). |
| `MinGarbageChars` | `int` | `5` | Minimum Unicode replacement characters (U+FFFD) to trigger OCR fallback. |
| `MaxFragmentedWordRatio` | `float64` | `0.6` | Maximum fraction of short (1-2 char) words before text is considered fragmented. |
| `CriticalFragmentedWordRatio` | `float64` | `0.8` | Critical fragmentation threshold — triggers OCR regardless of meaningful words. Normal English text has ~20-30% short words. 80%+ is definitive garbage. |
| `MinAvgWordLength` | `float64` | `2` | Minimum average word length. Below this with enough words indicates garbled extraction. |
| `MinWordsForAvgLengthCheck` | `int` | `50` | Minimum word count before average word length check applies. |
| `MinConsecutiveRepeatRatio` | `float64` | `0.08` | Minimum consecutive word repetition ratio to detect column scrambling. |
| `MinWordsForRepeatCheck` | `int` | `50` | Minimum word count before consecutive repetition check is applied. |
| `SubstantiveMinChars` | `int` | `100` | Minimum character count for "substantive markdown" OCR skip gate. |
| `NonTextMinChars` | `int` | `20` | Minimum character count for "non-text content" OCR skip gate. |
| `AlnumWsRatioThreshold` | `float64` | `0.4` | Alphanumeric+whitespace ratio threshold for skip decisions. |
| `PipelineMinQuality` | `float64` | `0.5` | Minimum quality score (0.0-1.0) for a pipeline stage result to be accepted. If the result from a backend scores below this, try the next backend. |

##### Methods

###### Default()

**Signature:**

```go
func (o *OcrQualityThresholds) Default() OcrQualityThresholds
```


---

#### OcrRotation

Rotation information for an OCR element.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `AngleDegrees` | `float64` | — | Rotation angle in degrees (0, 90, 180, 270 for PaddleOCR). |
| `Confidence` | `*float64` | `nil` | Confidence score for the rotation detection. |


---

#### OcrTable

Table detected via OCR.

Represents a table structure recognized during OCR processing.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Cells` | `[][]string` | — | Table cells as a 2D vector (rows × columns) |
| `Markdown` | `string` | — | Markdown representation of the table |
| `PageNumber` | `int` | — | Page number where the table was found (1-indexed) |
| `BoundingBox` | `*OcrTableBoundingBox` | `nil` | Bounding box of the table in pixel coordinates (from OCR word positions). |


---

#### OcrTableBoundingBox

Bounding box for an OCR-detected table in pixel coordinates.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Left` | `uint32` | — | Left x-coordinate (pixels) |
| `Top` | `uint32` | — | Top y-coordinate (pixels) |
| `Right` | `uint32` | — | Right x-coordinate (pixels) |
| `Bottom` | `uint32` | — | Bottom y-coordinate (pixels) |


---

#### OdtProperties

OpenDocument metadata from meta.xml

Contains metadata fields defined by the OASIS OpenDocument Format standard.
Uses Dublin Core elements (dc:) and OpenDocument meta elements (meta:).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Title` | `*string` | `nil` | Document title (dc:title) |
| `Subject` | `*string` | `nil` | Document subject/topic (dc:subject) |
| `Creator` | `*string` | `nil` | Current document creator/author (dc:creator) |
| `InitialCreator` | `*string` | `nil` | Initial creator of the document (meta:initial-creator) |
| `Keywords` | `*string` | `nil` | Keywords or tags (meta:keyword) |
| `Description` | `*string` | `nil` | Document description (dc:description) |
| `Date` | `*string` | `nil` | Current modification date (dc:date) |
| `CreationDate` | `*string` | `nil` | Initial creation date (meta:creation-date) |
| `Language` | `*string` | `nil` | Document language (dc:language) |
| `Generator` | `*string` | `nil` | Generator/application that created the document (meta:generator) |
| `EditingDuration` | `*string` | `nil` | Editing duration in ISO 8601 format (meta:editing-duration) |
| `EditingCycles` | `*string` | `nil` | Number of edits/revisions (meta:editing-cycles) |
| `PageCount` | `*int32` | `nil` | Document statistics - page count (meta:page-count) |
| `WordCount` | `*int32` | `nil` | Document statistics - word count (meta:word-count) |
| `CharacterCount` | `*int32` | `nil` | Document statistics - character count (meta:character-count) |
| `ParagraphCount` | `*int32` | `nil` | Document statistics - paragraph count (meta:paragraph-count) |
| `TableCount` | `*int32` | `nil` | Document statistics - table count (meta:table-count) |
| `ImageCount` | `*int32` | `nil` | Document statistics - image count (meta:image-count) |


---

#### OpenWebDocumentResponse

OpenWebUI "External" engine response format.

Returned by `PUT /process` for the OpenWebUI external document loader.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `PageContent` | `string` | — | Extracted text content |
| `Metadata` | `string` | — | Document metadata |


---

#### OrientationResult

Document orientation detection result.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Degrees` | `uint32` | — | Detected orientation in degrees (0, 90, 180, or 270). |
| `Confidence` | `float32` | — | Confidence score (0.0-1.0). |


---

#### PaddleOcrConfig

Configuration for PaddleOCR backend.

Configures PaddleOCR text detection and recognition with multi-language support.
Uses a builder pattern for convenient configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Language` | `string` | — | Language code (e.g., "en", "ch", "jpn", "kor", "deu", "fra") |
| `CacheDir` | `*string` | `nil` | Optional custom cache directory for model files |
| `UseAngleCls` | `bool` | — | Enable angle classification for rotated text (default: false). Can misfire on short text regions, rotating crops incorrectly before recognition. |
| `EnableTableDetection` | `bool` | — | Enable table structure detection (default: false) |
| `DetDbThresh` | `float32` | — | Database threshold for text detection (default: 0.3) Range: 0.0-1.0, higher values require more confident detections |
| `DetDbBoxThresh` | `float32` | — | Box threshold for text bounding box refinement (default: 0.5) Range: 0.0-1.0 |
| `DetDbUnclipRatio` | `float32` | — | Unclip ratio for expanding text bounding boxes (default: 1.6) Controls the expansion of detected text regions |
| `DetLimitSideLen` | `uint32` | — | Maximum side length for detection image (default: 960) Larger images may be resized to this limit for faster inference |
| `RecBatchNum` | `uint32` | — | Batch size for recognition inference (default: 6) Number of text regions to process simultaneously |
| `Padding` | `uint32` | — | Padding in pixels added around the image before detection (default: 10). Large values can include surrounding content like table gridlines. |
| `DropScore` | `float32` | — | Minimum recognition confidence score for text lines (default: 0.5). Text regions with recognition confidence below this threshold are discarded. Matches PaddleOCR Python's `drop_score` parameter. Range: 0.0-1.0 |
| `ModelTier` | `string` | — | Model tier controlling detection/recognition model size and accuracy trade-off. - `"mobile"` (default): Lightweight models (~4.5MB detection, ~16.5MB recognition), fast download and inference - `"server"`: Large, high-accuracy models (~88MB detection, ~84MB recognition), best for GPU or complex documents |

##### Methods

###### WithCacheDir()

Sets a custom cache directory for model files.

**Signature:**

```go
func (o *PaddleOcrConfig) WithCacheDir(path string) PaddleOcrConfig
```

###### WithTableDetection()

Enables or disables table structure detection.

**Signature:**

```go
func (o *PaddleOcrConfig) WithTableDetection(enable bool) PaddleOcrConfig
```

###### WithAngleCls()

Enables or disables angle classification for rotated text.

**Signature:**

```go
func (o *PaddleOcrConfig) WithAngleCls(enable bool) PaddleOcrConfig
```

###### WithDetDbThresh()

Sets the database threshold for text detection.

**Signature:**

```go
func (o *PaddleOcrConfig) WithDetDbThresh(threshold float32) PaddleOcrConfig
```

###### WithDetDbBoxThresh()

Sets the box threshold for text bounding box refinement.

**Signature:**

```go
func (o *PaddleOcrConfig) WithDetDbBoxThresh(threshold float32) PaddleOcrConfig
```

###### WithDetDbUnclipRatio()

Sets the unclip ratio for expanding text bounding boxes.

**Signature:**

```go
func (o *PaddleOcrConfig) WithDetDbUnclipRatio(ratio float32) PaddleOcrConfig
```

###### WithDetLimitSideLen()

Sets the maximum side length for detection images.

**Signature:**

```go
func (o *PaddleOcrConfig) WithDetLimitSideLen(length uint32) PaddleOcrConfig
```

###### WithRecBatchNum()

Sets the batch size for recognition inference.

**Signature:**

```go
func (o *PaddleOcrConfig) WithRecBatchNum(batchSize uint32) PaddleOcrConfig
```

###### WithDropScore()

Sets the minimum recognition confidence threshold.

**Signature:**

```go
func (o *PaddleOcrConfig) WithDropScore(score float32) PaddleOcrConfig
```

###### WithPadding()

Sets padding in pixels added around images before detection.

**Signature:**

```go
func (o *PaddleOcrConfig) WithPadding(padding uint32) PaddleOcrConfig
```

###### WithModelTier()

Sets the model tier controlling detection/recognition model size.

**Signature:**

```go
func (o *PaddleOcrConfig) WithModelTier(tier string) PaddleOcrConfig
```

###### Default()

Creates a default configuration with English language support.

**Signature:**

```go
func (o *PaddleOcrConfig) Default() PaddleOcrConfig
```


---

#### PageBoundary

Byte offset boundary for a page.

Tracks where a specific page's content starts and ends in the main content string,
enabling mapping from byte positions to page numbers. Offsets are guaranteed to be
at valid UTF-8 character boundaries when using standard String methods (push_str, push, etc.).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `ByteStart` | `int` | — | Byte offset where this page starts in the content string (UTF-8 valid boundary, inclusive) |
| `ByteEnd` | `int` | — | Byte offset where this page ends in the content string (UTF-8 valid boundary, exclusive) |
| `PageNumber` | `int` | — | Page number (1-indexed) |


---

#### PageConfig

Page extraction and tracking configuration.

Controls how pages are extracted, tracked, and represented in the extraction results.
When `nil`, page tracking is disabled.

Page range tracking in chunk metadata (first_page/last_page) is automatically enabled
when page boundaries are available and chunking is configured.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `ExtractPages` | `bool` | `false` | Extract pages as separate array (ExtractionResult.pages) |
| `InsertPageMarkers` | `bool` | `false` | Insert page markers in main content string |
| `MarkerFormat` | `string` | `"

<!-- PAGE {page_num} -->

"` | Page marker format (use {page_num} placeholder) Default: "\n\n<!-- PAGE {page_num} -->\n\n" |

##### Methods

###### Default()

**Signature:**

```go
func (o *PageConfig) Default() PageConfig
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
| `PageNumber` | `int` | — | Page number (1-indexed) |
| `Content` | `string` | — | Text content for this page |
| `Tables` | `[]string` | — | Tables found on this page (uses Arc for memory efficiency) Serializes as Vec<Table> for JSON compatibility while maintaining Arc semantics in-memory for zero-copy sharing. |
| `Images` | `[]ExtractedImage` | — | Images found on this page (uses Arc for memory efficiency) Serializes as Vec<ExtractedImage> for JSON compatibility while maintaining Arc semantics in-memory for zero-copy sharing. |
| `Hierarchy` | `*PageHierarchy` | `nil` | Hierarchy information for the page (when hierarchy extraction is enabled) Contains text hierarchy levels (H1-H6) extracted from the page content. |
| `IsBlank` | `*bool` | `nil` | Whether this page is blank (no meaningful text content) Determined during extraction based on text content analysis. A page is blank if it has fewer than 3 non-whitespace characters and contains no tables or images. |
| `LayoutRegions` | `*[]LayoutRegion` | `nil` | Layout detection regions for this page (when layout detection is enabled). Contains detected layout regions with class, confidence, bounding box, and area fraction. Only populated when layout detection is configured. |


---

#### PageHierarchy

Page hierarchy structure containing heading levels and block information.

Used when PDF text hierarchy extraction is enabled. Contains hierarchical
blocks with heading levels (H1-H6) for semantic document structure.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `BlockCount` | `int` | — | Number of hierarchy blocks on this page |
| `Blocks` | `[]HierarchicalBlock` | — | Hierarchical blocks with heading levels |


---

#### PageInfo

Metadata for individual page/slide/sheet.

Captures per-page information including dimensions, content counts,
and visibility state (for presentations).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Number` | `int` | — | Page number (1-indexed) |
| `Title` | `*string` | `nil` | Page title (usually for presentations) |
| `Dimensions` | `*[]float64` | `nil` | Dimensions in points (PDF) or pixels (images): (width, height) |
| `ImageCount` | `*int` | `nil` | Number of images on this page |
| `TableCount` | `*int` | `nil` | Number of tables on this page |
| `Hidden` | `*bool` | `nil` | Whether this page is hidden (e.g., in presentations) |
| `IsBlank` | `*bool` | `nil` | Whether this page is blank (no meaningful text, no images, no tables) A page is considered blank if it has fewer than 3 non-whitespace characters and contains no tables or images. This is useful for filtering out empty pages in scanned documents or PDFs with blank separator pages. |


---

#### PageLayoutResult

Layout detection results for a single page.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `PageIndex` | `int` | — | Page index |
| `Regions` | `[]string` | — | Regions |
| `PageWidthPts` | `float32` | — | Page width pts |
| `PageHeightPts` | `float32` | — | Page height pts |
| `RenderWidthPx` | `uint32` | — | Width of the rendered image used for layout detection (pixels). |
| `RenderHeightPx` | `uint32` | — | Height of the rendered image used for layout detection (pixels). |


---

#### PageMarginsPoints

Page margins converted to points (1/72 inch).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Top` | `*float64` | `nil` | Top |
| `Right` | `*float64` | `nil` | Right |
| `Bottom` | `*float64` | `nil` | Bottom |
| `Left` | `*float64` | `nil` | Left |
| `Header` | `*float64` | `nil` | Header |
| `Footer` | `*float64` | `nil` | Footer |
| `Gutter` | `*float64` | `nil` | Gutter |


---

#### PageStructure

Unified page structure for documents.

Supports different page types (PDF pages, PPTX slides, Excel sheets)
with character offset boundaries for chunk-to-page mapping.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `TotalCount` | `int` | — | Total number of pages/slides/sheets |
| `UnitType` | `PageUnitType` | — | Type of paginated unit |
| `Boundaries` | `*[]PageBoundary` | `nil` | Character offset boundaries for each page Maps character ranges in the extracted content to page numbers. Used for chunk page range calculation. |
| `Pages` | `*[]PageInfo` | `nil` | Detailed per-page metadata (optional, only when needed) |


---

#### PageTiming

Timing breakdown for a single page.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `RenderMs` | `float64` | — | Time to render the PDF page to a raster image (amortized from batch render). |
| `PreprocessMs` | `float64` | — | Time spent in image preprocessing (resize, normalize, tensor construction). |
| `OnnxMs` | `float64` | — | Time for the ONNX model session.run() call (actual neural network inference). |
| `InferenceMs` | `float64` | — | Total model inference time (preprocess + onnx), as measured by the engine. |
| `PostprocessMs` | `float64` | — | Time spent in postprocessing (confidence filtering, overlap resolution). |
| `MappingMs` | `float64` | — | Time to map pixel-space bounding boxes to PDF coordinate space. |


---

#### PdfAnnotation

A PDF annotation extracted from a document page.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `AnnotationType` | `PdfAnnotationType` | — | The type of annotation. |
| `Content` | `*string` | `nil` | Text content of the annotation (e.g., comment text, link URL). |
| `PageNumber` | `int` | — | Page number where the annotation appears (1-indexed). |
| `BoundingBox` | `*string` | `nil` | Bounding box of the annotation on the page. |


---

#### PdfConfig

PDF-specific configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Backend` | `PdfBackend` | `PdfBackend.Pdfium` | PDF extraction backend. Default: `Pdfium`. |
| `ExtractImages` | `bool` | `false` | Extract images from PDF |
| `Passwords` | `*[]string` | `nil` | List of passwords to try when opening encrypted PDFs |
| `ExtractMetadata` | `bool` | `true` | Extract PDF metadata |
| `Hierarchy` | `*HierarchyConfig` | `nil` | Hierarchy extraction configuration (None = hierarchy extraction disabled) |
| `ExtractAnnotations` | `bool` | `false` | Extract PDF annotations (text notes, highlights, links, stamps). Default: false |
| `TopMarginFraction` | `*float32` | `nil` | Top margin fraction (0.0–1.0) of page height to exclude headers/running heads. Default: 0.06 (6%) |
| `BottomMarginFraction` | `*float32` | `nil` | Bottom margin fraction (0.0–1.0) of page height to exclude footers/page numbers. Default: 0.05 (5%) |
| `AllowSingleColumnTables` | `bool` | `false` | Allow single-column pseudo tables in extraction results. By default, tables with fewer than 2 columns (layout-guided) or 3 columns (heuristic) are rejected. When `true`, the minimum column count is relaxed to 1, allowing single-column structured data (glossaries, itemized lists) to be emitted as tables. Other quality filters (density, sparsity, prose detection) still apply. |

##### Methods

###### Default()

**Signature:**

```go
func (o *PdfConfig) Default() PdfConfig
```


---

#### PdfImage

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `PageNumber` | `int` | — | Page number |
| `ImageIndex` | `int` | — | Image index |
| `Width` | `int64` | — | Width |
| `Height` | `int64` | — | Height |
| `ColorSpace` | `*string` | `nil` | Color space |
| `BitsPerComponent` | `*int64` | `nil` | Bits per component |
| `Filters` | `[]string` | — | Original PDF stream filters (e.g. `["FlateDecode"]`, `["DCTDecode"]`). |
| `Data` | `[]byte` | — | The decoded image bytes in a standard format (JPEG, PNG, etc.). |
| `DecodedFormat` | `string` | — | The format of `data` after decoding: `"jpeg"`, `"png"`, `"jpeg2000"`, `"ccitt"`, or `"raw"`. |


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

###### Name()

Returns the unique name/identifier for this plugin.

The name should be:
- Unique across all plugins
- Lowercase with hyphens (e.g., "my-custom-plugin")
- URL-safe characters only

**Signature:**

```go
func (o *Plugin) Name() string
```

###### Version()

Returns the semantic version of this plugin.

Should follow semver format: `MAJOR.MINOR.PATCH`

**Signature:**

```go
func (o *Plugin) Version() string
```

###### Initialize()

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

```go
func (o *Plugin) Initialize() error
```

###### Shutdown()

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

```go
func (o *Plugin) Shutdown() error
```

###### Description()

Optional plugin description for debugging and logging.

Defaults to empty string if not overridden.

**Signature:**

```go
func (o *Plugin) Description() string
```

###### Author()

Optional plugin author information.

Defaults to empty string if not overridden.

**Signature:**

```go
func (o *Plugin) Author() string
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

###### Process()

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

```go
func (o *PostProcessor) Process(result ExtractionResult, config ExtractionConfig) error
```

###### ProcessingStage()

Get the processing stage for this post-processor.

Determines when this processor runs in the pipeline.

**Returns:**

The `ProcessingStage` (Early, Middle, or Late).

**Signature:**

```go
func (o *PostProcessor) ProcessingStage() ProcessingStage
```

###### ShouldProcess()

Optional: Check if this processor should run for a given result.

Allows conditional processing based on MIME type, metadata, or content.
Defaults to `true` (always run).

**Returns:**

`true` if the processor should run, `false` to skip.

**Signature:**

```go
func (o *PostProcessor) ShouldProcess(result ExtractionResult, config ExtractionConfig) bool
```

###### EstimatedDurationMs()

Optional: Estimate processing time in milliseconds.

Used for logging and debugging. Defaults to 0 (unknown).

**Returns:**

Estimated processing time in milliseconds.

**Signature:**

```go
func (o *PostProcessor) EstimatedDurationMs(result ExtractionResult) uint64
```


---

#### PostProcessorConfig

Post-processor configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Enabled` | `bool` | `true` | Enable post-processors |
| `EnabledProcessors` | `*[]string` | `nil` | Whitelist of processor names to run (None = all enabled) |
| `DisabledProcessors` | `*[]string` | `nil` | Blacklist of processor names to skip (None = none disabled) |
| `EnabledSet` | `*string` | `nil` | Pre-computed AHashSet for O(1) enabled processor lookup |
| `DisabledSet` | `*string` | `nil` | Pre-computed AHashSet for O(1) disabled processor lookup |

##### Methods

###### Default()

**Signature:**

```go
func (o *PostProcessorConfig) Default() PostProcessorConfig
```


---

#### PptxAppProperties

Application properties from docProps/app.xml for PPTX

Contains PowerPoint-specific document metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Application` | `*string` | `nil` | Application name (e.g., "Microsoft Office PowerPoint") |
| `AppVersion` | `*string` | `nil` | Application version |
| `TotalTime` | `*int32` | `nil` | Total editing time in minutes |
| `Company` | `*string` | `nil` | Company name |
| `DocSecurity` | `*int32` | `nil` | Document security level |
| `ScaleCrop` | `*bool` | `nil` | Scale crop flag |
| `LinksUpToDate` | `*bool` | `nil` | Links up to date flag |
| `SharedDoc` | `*bool` | `nil` | Shared document flag |
| `HyperlinksChanged` | `*bool` | `nil` | Hyperlinks changed flag |
| `Slides` | `*int32` | `nil` | Number of slides |
| `Notes` | `*int32` | `nil` | Number of notes |
| `HiddenSlides` | `*int32` | `nil` | Number of hidden slides |
| `MultimediaClips` | `*int32` | `nil` | Number of multimedia clips |
| `PresentationFormat` | `*string` | `nil` | Presentation format (e.g., "Widescreen", "Standard") |
| `SlideTitles` | `[]string` | `nil` | Slide titles |


---

#### PptxExtractionResult

PowerPoint (PPTX) extraction result.

Contains extracted slide content, metadata, and embedded images/tables.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Content` | `string` | — | Extracted text content from all slides |
| `Metadata` | `PptxMetadata` | — | Presentation metadata |
| `SlideCount` | `int` | — | Total number of slides |
| `ImageCount` | `int` | — | Total number of embedded images |
| `TableCount` | `int` | — | Total number of tables |
| `Images` | `[]ExtractedImage` | — | Extracted images from the presentation |
| `PageStructure` | `*PageStructure` | `nil` | Slide structure with boundaries (when page tracking is enabled) |
| `PageContents` | `*[]PageContent` | `nil` | Per-slide content (when page tracking is enabled) |
| `Document` | `*DocumentStructure` | `nil` | Structured document representation |
| `Hyperlinks` | `[]string` | — | Hyperlinks discovered in slides as (url, optional_label) pairs. |
| `OfficeMetadata` | `map[string]string` | — | Office metadata extracted from docProps/core.xml and docProps/app.xml. Contains keys like "title", "author", "created_by", "subject", "keywords", "modified_by", "created_at", "modified_at", etc. |


---

#### PptxMetadata

PowerPoint presentation metadata.

Extracted from PPTX files containing slide counts and presentation details.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `SlideCount` | `int` | — | Total number of slides in the presentation |
| `SlideNames` | `[]string` | `nil` | Names of slides (if available) |
| `ImageCount` | `*int` | `nil` | Number of embedded images |
| `TableCount` | `*int` | `nil` | Number of tables |


---

#### ProcessingWarning

A non-fatal warning from a processing pipeline stage.

Captures errors from optional features that don't prevent extraction
but may indicate degraded results.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Source` | `string` | — | The pipeline stage or feature that produced this warning (e.g., "embedding", "chunking", "language_detection", "output_format"). |
| `Message` | `string` | — | Human-readable description of what went wrong. |


---

#### PstMetadata

Outlook PST archive metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `MessageCount` | `int` | — | Number of messages |


---

#### RakeParams

RAKE-specific parameters.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `MinWordLength` | `int` | `1` | Minimum word length to consider (default: 1). |
| `MaxWordsPerPhrase` | `int` | `3` | Maximum words in a keyword phrase (default: 3). |

##### Methods

###### Default()

**Signature:**

```go
func (o *RakeParams) Default() RakeParams
```


---

#### RecognizedTable

Pre-computed table markdown for a table detection region.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `DetectionBbox` | `BBox` | — | Detection bbox that this table corresponds to (for matching). |
| `Cells` | `[][]string` | — | Table cells as a 2D vector (rows x columns). |
| `Markdown` | `string` | — | Rendered markdown table. |


---

#### Recyclable

Trait for types that can be pooled and reused.

Implementing this trait allows a type to be used with `Pool<T>`.
The `reset()` method should clear the object's state for reuse.

##### Methods

###### Reset()

Reset the object to a reusable state.

This is called when returning an object to the pool.
Should clear any internal data while preserving capacity.

**Signature:**

```go
func (o *Recyclable) Reset()
```


---

#### ResolvedStyle

Fully resolved (flattened) style after walking the inheritance chain.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `ParagraphProperties` | `string` | — | Paragraph properties |
| `RunProperties` | `string` | — | Run properties |


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
| `Host` | `string` | — | Server host address (e.g., "127.0.0.1", "0.0.0.0") |
| `Port` | `uint16` | — | Server port number |
| `CorsOrigins` | `[]string` | `nil` | CORS allowed origins. Empty vector means allow all origins. If this is an empty vector, the server will accept requests from any origin. If populated with specific origins (e.g., ["<https://example.com">]), only those origins will be allowed. |
| `MaxRequestBodyBytes` | `int` | — | Maximum size of request body in bytes (default: 100 MB) |
| `MaxMultipartFieldBytes` | `int` | — | Maximum size of multipart fields in bytes (default: 100 MB) |

##### Methods

###### Default()

**Signature:**

```go
func (o *ServerConfig) Default() ServerConfig
```

###### ListenAddr()

Get the server listen address (host:port).

**Signature:**

```go
func (o *ServerConfig) ListenAddr() string
```

###### CorsAllowsAll()

Check if CORS allows all origins.

Returns `true` if the `cors_origins` vector is empty, meaning all origins
are allowed. Returns `false` if specific origins are configured.

**Signature:**

```go
func (o *ServerConfig) CorsAllowsAll() bool
```

###### IsOriginAllowed()

Check if a given origin is allowed by CORS configuration.

Returns `true` if:
- CORS allows all origins (empty origins list), or
- The given origin is in the allowed origins list

**Signature:**

```go
func (o *ServerConfig) IsOriginAllowed(origin string) bool
```

###### MaxRequestBodyMb()

Get maximum request body size in megabytes (rounded up).

**Signature:**

```go
func (o *ServerConfig) MaxRequestBodyMb() int
```

###### MaxMultipartFieldMb()

Get maximum multipart field size in megabytes (rounded up).

**Signature:**

```go
func (o *ServerConfig) MaxMultipartFieldMb() int
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
| `DataType` | `StructuredDataType` | — | Type of structured data |
| `RawJson` | `string` | — | Raw JSON string representation |
| `SchemaType` | `*string` | `nil` | Schema type if detectable (e.g., "Article", "Event", "Product") |


---

#### StructuredDataResult

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Content` | `string` | — | The extracted text content |
| `Format` | `string` | — | Format |
| `Metadata` | `map[string]string` | — | Document metadata |
| `TextFields` | `[]string` | — | Text fields |


---

#### StructuredExtractionConfig

Configuration for LLM-based structured data extraction.

Sends extracted document content to a VLM with a JSON schema,
returning structured data that conforms to the schema.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Schema` | `interface{}` | — | JSON Schema defining the desired output structure. |
| `SchemaName` | `string` | — | Schema name passed to the LLM's structured output mode. |
| `SchemaDescription` | `*string` | `nil` | Optional schema description for the LLM. |
| `Strict` | `bool` | — | Enable strict mode — output must exactly match the schema. |
| `Prompt` | `*string` | `nil` | Custom Jinja2 extraction prompt template. When `nil`, a default template is used. Available template variables: - `{{ content }}` — The extracted document text. - `{{ schema }}` — The JSON schema as a formatted string. - `{{ schema_name }}` — The schema name. - `{{ schema_description }}` — The schema description (may be empty). |
| `Llm` | `LlmConfig` | — | LLM configuration for the extraction. |


---

#### StructuredExtractionResponse

Response from structured extraction endpoint.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `StructuredOutput` | `interface{}` | — | Structured data conforming to the provided JSON schema |
| `Content` | `string` | — | Extracted document text content |
| `MimeType` | `string` | — | Detected MIME type of the input file |


---

#### StyleDefinition

A single style definition parsed from `<w:style>` in `word/styles.xml`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Id` | `string` | — | The style ID (`w:styleId` attribute). |
| `Name` | `*string` | `nil` | Human-readable name (`<w:name w:val="..."/>`). |
| `StyleType` | `string` | — | Style type: paragraph, character, table, or numbering. |
| `BasedOn` | `*string` | `nil` | ID of the parent style (`<w:basedOn w:val="..."/>`). |
| `NextStyle` | `*string` | `nil` | ID of the style to apply to the next paragraph (`<w:next w:val="..."/>`). |
| `IsDefault` | `bool` | — | Whether this is the default style for its type. |
| `ParagraphProperties` | `string` | — | Paragraph properties defined directly on this style. |
| `RunProperties` | `string` | — | Run properties defined directly on this style. |


---

#### SupportedFormat

A supported document format entry.

Represents a file extension and its corresponding MIME type that Kreuzberg can process.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Extension` | `string` | — | File extension (without leading dot), e.g., "pdf", "docx" |
| `MimeType` | `string` | — | MIME type string, e.g., "application/pdf" |


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

###### ExtractSync()

Extract content from a byte array synchronously.

This method performs extraction without requiring an async runtime.
It is called by `extract_bytes_sync()` when the `tokio-runtime` feature is disabled.

**Returns:**

An `InternalDocument` containing the extracted elements, metadata, and tables.

**Signature:**

```go
func (o *SyncExtractor) ExtractSync(content []byte, mimeType string, config ExtractionConfig) (string, error)
```


---

#### TableProperties

Table-level properties from `<w:tblPr>`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `StyleId` | `*string` | `nil` | Style id |
| `Width` | `*string` | `nil` | Width |
| `Alignment` | `*string` | `nil` | Alignment |
| `Layout` | `*string` | `nil` | Layout |
| `Look` | `*string` | `nil` | Look |
| `Borders` | `*string` | `nil` | Borders |
| `CellMargins` | `*string` | `nil` | Cell margins |
| `Indent` | `*string` | `nil` | Indent |
| `Caption` | `*string` | `nil` | Caption |


---

#### TessdataManager

Manages tessdata file downloading, caching, and manifest generation.

##### Methods

###### CacheDir()

Get the cache directory path.

**Signature:**

```go
func (o *TessdataManager) CacheDir() string
```

###### IsLanguageCached()

Check if a specific language traineddata file is cached.

**Signature:**

```go
func (o *TessdataManager) IsLanguageCached(lang string) bool
```

###### EnsureAllLanguages()

Downloads all tessdata_fast traineddata files to the cache directory.

Skips files that already exist. Returns the count of newly downloaded files.

Requires the `paddle-ocr` feature for HTTP download support (ureq).

**Signature:**

```go
func (o *TessdataManager) EnsureAllLanguages() (int, error)
```


---

#### TesseractConfig

Tesseract OCR configuration.

Provides fine-grained control over Tesseract OCR engine parameters.
Most users can use the defaults, but these settings allow optimization
for specific document types (invoices, handwriting, etc.).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Language` | `string` | `"eng"` | Language code (e.g., "eng", "deu", "fra") |
| `Psm` | `int32` | `3` | Page Segmentation Mode (0-13). Common values: - 3: Fully automatic page segmentation (default) - 6: Assume a single uniform block of text - 11: Sparse text with no particular order |
| `OutputFormat` | `string` | `"markdown"` | Output format ("text" or "markdown") |
| `Oem` | `int32` | `3` | OCR Engine Mode (0-3). - 0: Legacy engine only - 1: Neural nets (LSTM) only (usually best) - 2: Legacy + LSTM - 3: Default (based on what's available) |
| `MinConfidence` | `float64` | `0` | Minimum confidence threshold (0.0-100.0). Words with confidence below this threshold may be rejected or flagged. |
| `Preprocessing` | `*ImagePreprocessingConfig` | `nil` | Image preprocessing configuration. Controls how images are preprocessed before OCR. Can significantly improve quality for scanned documents or low-quality images. |
| `EnableTableDetection` | `bool` | `true` | Enable automatic table detection and reconstruction |
| `TableMinConfidence` | `float64` | `0` | Minimum confidence threshold for table detection (0.0-1.0) |
| `TableColumnThreshold` | `int32` | `50` | Column threshold for table detection (pixels) |
| `TableRowThresholdRatio` | `float64` | `0.5` | Row threshold ratio for table detection (0.0-1.0) |
| `UseCache` | `bool` | `true` | Enable OCR result caching |
| `ClassifyUsePreAdaptedTemplates` | `bool` | `true` | Use pre-adapted templates for character classification |
| `LanguageModelNgramOn` | `bool` | `false` | Enable N-gram language model |
| `TesseditDontBlkrejGoodWds` | `bool` | `true` | Don't reject good words during block-level processing |
| `TesseditDontRowrejGoodWds` | `bool` | `true` | Don't reject good words during row-level processing |
| `TesseditEnableDictCorrection` | `bool` | `true` | Enable dictionary correction |
| `TesseditCharWhitelist` | `string` | `""` | Whitelist of allowed characters (empty = all allowed) |
| `TesseditCharBlacklist` | `string` | `""` | Blacklist of forbidden characters (empty = none forbidden) |
| `TesseditUsePrimaryParamsModel` | `bool` | `true` | Use primary language params model |
| `TextordSpaceSizeIsVariable` | `bool` | `true` | Variable-width space detection |
| `ThresholdingMethod` | `bool` | `false` | Use adaptive thresholding method |

##### Methods

###### Default()

**Signature:**

```go
func (o *TesseractConfig) Default() TesseractConfig
```


---

#### TextAnnotation

Inline text annotation — byte-range based formatting and links.

Annotations reference byte offsets into the node's text content,
enabling precise identification of formatted regions.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Start` | `uint32` | — | Start byte offset in the node's text content (inclusive). |
| `End` | `uint32` | — | End byte offset in the node's text content (exclusive). |
| `Kind` | `AnnotationKind` | — | Annotation type. |


---

#### TextExtractionResult

Plain text and Markdown extraction result.

Contains the extracted text along with statistics and,
for Markdown files, structural elements like headers and links.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Content` | `string` | — | Extracted text content |
| `LineCount` | `int` | — | Number of lines |
| `WordCount` | `int` | — | Number of words |
| `CharacterCount` | `int` | — | Number of characters |
| `Headers` | `*[]string` | `nil` | Markdown headers (text only, Markdown files only) |
| `Links` | `*[]string` | `nil` | Markdown links as (text, URL) tuples (Markdown files only) |
| `CodeBlocks` | `*[]string` | `nil` | Code blocks as (language, code) tuples (Markdown files only) |


---

#### TextMetadata

Text/Markdown metadata.

Extracted from plain text and Markdown files. Includes word counts and,
for Markdown, structural elements like headers and links.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `LineCount` | `int` | — | Number of lines in the document |
| `WordCount` | `int` | — | Number of words |
| `CharacterCount` | `int` | — | Number of characters |
| `Headers` | `*[]string` | `nil` | Markdown headers (headings text only, for Markdown files) |
| `Links` | `*[]string` | `nil` | Markdown links as (text, url) tuples (for Markdown files) |
| `CodeBlocks` | `*[]string` | `nil` | Code blocks as (language, code) tuples (for Markdown files) |


---

#### TokenReductionConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Level` | `ReductionLevel` | `ReductionLevel.Moderate` | Level (reduction level) |
| `LanguageHint` | `*string` | `nil` | Language hint |
| `PreserveMarkdown` | `bool` | `false` | Preserve markdown |
| `PreserveCode` | `bool` | `true` | Preserve code |
| `SemanticThreshold` | `float32` | `0.3` | Semantic threshold |
| `EnableParallel` | `bool` | `true` | Enable parallel |
| `UseSimd` | `bool` | `true` | Use simd |
| `CustomStopwords` | `*map[string][]string` | `nil` | Custom stopwords |
| `PreservePatterns` | `[]string` | `nil` | Preserve patterns |
| `TargetReduction` | `*float32` | `nil` | Target reduction |
| `EnableSemanticClustering` | `bool` | `false` | Enable semantic clustering |

##### Methods

###### Default()

**Signature:**

```go
func (o *TokenReductionConfig) Default() TokenReductionConfig
```


---

#### TokenReductionOptions

Token reduction configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Mode` | `string` | — | Reduction mode: "off", "light", "moderate", "aggressive", "maximum" |
| `PreserveImportantWords` | `bool` | — | Preserve important words (capitalized, technical terms) |


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
| `Enabled` | `bool` | `true` | Enable code intelligence processing (default: true). When `false`, tree-sitter analysis is completely skipped even if the config section is present. |
| `CacheDir` | `*string` | `nil` | Custom cache directory for downloaded grammars. When `nil`, uses the default: `~/.cache/tree-sitter-language-pack/v{version}/libs/`. |
| `Languages` | `*[]string` | `nil` | Languages to pre-download on init (e.g., `["python", "rust"]`). |
| `Groups` | `*[]string` | `nil` | Language groups to pre-download (e.g., `["web", "systems", "scripting"]`). |
| `Process` | `TreeSitterProcessConfig` | — | Processing options for code analysis. |

##### Methods

###### Default()

**Signature:**

```go
func (o *TreeSitterConfig) Default() TreeSitterConfig
```


---

#### TreeSitterProcessConfig

Processing options for tree-sitter code analysis.

Controls which analysis features are enabled when extracting code files.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Structure` | `bool` | `true` | Extract structural items (functions, classes, structs, etc.). Default: true. |
| `Imports` | `bool` | `true` | Extract import statements. Default: true. |
| `Exports` | `bool` | `true` | Extract export statements. Default: true. |
| `Comments` | `bool` | `false` | Extract comments. Default: false. |
| `Docstrings` | `bool` | `false` | Extract docstrings. Default: false. |
| `Symbols` | `bool` | `false` | Extract symbol definitions. Default: false. |
| `Diagnostics` | `bool` | `false` | Include parse diagnostics. Default: false. |
| `ChunkMaxSize` | `*int` | `nil` | Maximum chunk size in bytes. `nil` disables chunking. |
| `ContentMode` | `CodeContentMode` | `CodeContentMode.Chunks` | Content rendering mode for code extraction. |

##### Methods

###### Default()

**Signature:**

```go
func (o *TreeSitterProcessConfig) Default() TreeSitterProcessConfig
```


---

#### Uri

A URI extracted from a document.

Represents any link, reference, or resource pointer found during extraction.
The `kind` field classifies the URI semantically, while `label` carries
optional human-readable display text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Url` | `string` | — | The URL or path string. |
| `Label` | `*string` | `nil` | Optional display text / label for the link. |
| `Page` | `*uint32` | `nil` | Optional page number where the URI was found (1-indexed). |
| `Kind` | `UriKind` | — | Semantic classification of the URI. |


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

###### Validate()

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

```go
func (o *Validator) Validate(result ExtractionResult, config ExtractionConfig) error
```

###### ShouldValidate()

Optional: Check if this validator should run for a given result.

Allows conditional validation based on MIME type, metadata, or content.
Defaults to `true` (always run).

**Returns:**

`true` if the validator should run, `false` to skip.

**Signature:**

```go
func (o *Validator) ShouldValidate(result ExtractionResult, config ExtractionConfig) bool
```

###### Priority()

Optional: Get the validation priority.

Higher priority validators run first. Useful for ordering validation checks
(e.g., run cheap validations before expensive ones).

Default priority is 50.

**Returns:**

Priority value (higher = runs earlier).

**Signature:**

```go
func (o *Validator) Priority() int32
```


---

#### VersionResponse

Version response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Version` | `string` | — | Kreuzberg version string |


---

#### WarmRequest

Cache warm request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `AllEmbeddings` | `bool` | — | Download all embedding model presets |
| `EmbeddingModel` | `*string` | `nil` | Specific embedding model preset to download |


---

#### WarmResponse

Cache warm response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `CacheDir` | `string` | — | Cache directory used |
| `Downloaded` | `[]string` | — | Models that were downloaded |
| `AlreadyCached` | `[]string` | — | Models that were already cached |


---

#### XlsxAppProperties

Application properties from docProps/app.xml for XLSX

Contains Excel-specific document metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Application` | `*string` | `nil` | Application name (e.g., "Microsoft Excel") |
| `AppVersion` | `*string` | `nil` | Application version |
| `DocSecurity` | `*int32` | `nil` | Document security level |
| `ScaleCrop` | `*bool` | `nil` | Scale crop flag |
| `LinksUpToDate` | `*bool` | `nil` | Links up to date flag |
| `SharedDoc` | `*bool` | `nil` | Shared document flag |
| `HyperlinksChanged` | `*bool` | `nil` | Hyperlinks changed flag |
| `Company` | `*string` | `nil` | Company name |
| `WorksheetNames` | `[]string` | `nil` | Worksheet names |


---

#### XmlExtractionResult

XML extraction result.

Contains extracted text content from XML files along with
structural statistics about the XML document.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Content` | `string` | — | Extracted text content (XML structure filtered out) |
| `ElementCount` | `int` | — | Total number of XML elements processed |
| `UniqueElements` | `[]string` | — | List of unique element names found (sorted) |


---

#### XmlMetadata

XML metadata extracted during XML parsing.

Provides statistics about XML document structure.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `ElementCount` | `int` | — | Total number of XML elements processed |
| `UniqueElements` | `[]string` | `nil` | List of unique element tag names (sorted) |


---

#### YakeParams

YAKE-specific parameters.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `WindowSize` | `int` | `2` | Window size for co-occurrence analysis (default: 2). Controls the context window for computing co-occurrence statistics. |

##### Methods

###### Default()

**Signature:**

```go
func (o *YakeParams) Default() YakeParams
```


---

#### YearRange

Year range for bibliographic metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Min` | `*uint32` | `nil` | Min |
| `Max` | `*uint32` | `nil` | Max |
| `Years` | `[]uint32` | — | Years |


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
| `Tokenizer` | Size measured in tokens from a HuggingFace tokenizer. — Fields: `Model`: `string`, `CacheDir`: `string` |


---

#### EmbeddingModelType

Embedding model types supported by Kreuzberg.

| Value | Description |
|-------|-------------|
| `Preset` | Use a preset model configuration (recommended) — Fields: `Name`: `string` |
| `Custom` | Use a custom ONNX model from HuggingFace — Fields: `ModelId`: `string`, `Dimensions`: `int` |
| `Llm` | Provider-hosted embedding model via liter-llm. Uses the model specified in the nested `LlmConfig` (e.g., `"openai/text-embedding-3-small"`). — Fields: `Llm`: `LlmConfig` |
| `Plugin` | In-process embedding backend registered via the plugin system. The caller registers an `EmbeddingBackend` once (e.g. a wrapper around an already-loaded `llama-cpp-python`, `sentence-transformers`, or tuned ONNX model), then references it by name in config. Kreuzberg calls back into the registered backend during chunking and standalone embed requests — no HuggingFace download, no ONNX Runtime requirement, no HTTP sidecar. When this variant is selected, only the following `EmbeddingConfig` fields apply: `normalize` (post-call L2 normalization) and `max_embed_duration_secs` (dispatcher timeout). Model-loading fields (`batch_size`, `cache_dir`, `show_download_progress`, `acceleration`) are ignored — the host owns the model lifecycle. Semantic chunking falls back to `ChunkingConfig.max_characters` when this variant is used, since there is no preset to look a chunk-size ceiling up against — size your context window via `max_characters` directly. See `crate.plugins.register_embedding_backend`. — Fields: `Name`: `string` |


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
| `Title` | Document title. — Fields: `Text`: `string` |
| `Heading` | Section heading with level (1-6). — Fields: `Level`: `uint8`, `Text`: `string` |
| `Paragraph` | Body text paragraph. — Fields: `Text`: `string` |
| `List` | List container — children are `ListItem` nodes. — Fields: `Ordered`: `bool` |
| `ListItem` | Individual list item. — Fields: `Text`: `string` |
| `Table` | Table with structured cell grid. — Fields: `Grid`: `string` |
| `Image` | Image reference. — Fields: `Description`: `string`, `ImageIndex`: `uint32`, `Src`: `string` |
| `Code` | Code block. — Fields: `Text`: `string`, `Language`: `string` |
| `Quote` | Block quote — container, children carry the quoted content. |
| `Formula` | Mathematical formula / equation. — Fields: `Text`: `string` |
| `Footnote` | Footnote reference content. — Fields: `Text`: `string` |
| `Group` | Logical grouping container (section, key-value area). `heading_level` + `heading_text` capture the section heading directly rather than relying on a first-child positional convention. — Fields: `Label`: `string`, `HeadingLevel`: `uint8`, `HeadingText`: `string` |
| `PageBreak` | Page break marker. |
| `Slide` | Presentation slide container — children are the slide's content nodes. — Fields: `Number`: `uint32`, `Title`: `string` |
| `DefinitionList` | Definition list container — children are `DefinitionItem` nodes. |
| `DefinitionItem` | Individual definition list entry with term and definition. — Fields: `Term`: `string`, `Definition`: `string` |
| `Citation` | Citation or bibliographic reference. — Fields: `Key`: `string`, `Text`: `string` |
| `Admonition` | Admonition / callout container (note, warning, tip, etc.). Children carry the admonition body content. — Fields: `Kind`: `string`, `Title`: `string` |
| `RawBlock` | Raw block preserved verbatim from the source format. Used for content that cannot be mapped to a semantic node type (e.g. JSX in MDX, raw LaTeX in markdown, embedded HTML). — Fields: `Format`: `string`, `Content`: `string` |
| `MetadataBlock` | Structured metadata block (email headers, YAML frontmatter, etc.). — Fields: `Entries`: `[]string` |


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
| `Link` | Link — Fields: `Url`: `string`, `Title`: `string` |
| `Highlight` | Highlighted text (PDF highlights, HTML `<mark>`). |
| `Color` | Text color (CSS-compatible value, e.g. "#ff0000", "red"). — Fields: `Value`: `string` |
| `FontSize` | Font size with units (e.g. "12pt", "1.2em", "16px"). — Fields: `Value`: `string` |
| `Custom` | Extensible annotation for format-specific styling. — Fields: `Name`: `string`, `Value`: `string` |


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
| `Pdf` | Pdf format — Fields: `0`: `string` |
| `Docx` | Docx format — Fields: `0`: `DocxMetadata` |
| `Excel` | Excel — Fields: `0`: `ExcelMetadata` |
| `Email` | Email — Fields: `0`: `EmailMetadata` |
| `Pptx` | Pptx format — Fields: `0`: `PptxMetadata` |
| `Archive` | Archive — Fields: `0`: `ArchiveMetadata` |
| `Image` | Image element — Fields: `0`: `string` |
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
| `Code` | Code — Fields: `0`: `string` |


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
| `Rectangle` | Axis-aligned bounding box (typical for Tesseract output). — Fields: `Left`: `uint32`, `Top`: `uint32`, `Width`: `uint32`, `Height`: `uint32` |
| `Quadrilateral` | 4-point quadrilateral for rotated/skewed text (PaddleOCR). Points are in clockwise order starting from top-left: `[top_left, top_right, bottom_right, bottom_left]` — Fields: `Points`: `string` |


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

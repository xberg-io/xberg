---
title: "TypeScript API Reference"
---

## TypeScript API Reference <span class="version-badge">v5.0.0-rc.10</span>

### Functions

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

```typescript
function extractBytes(content: Buffer, mimeType: string, config: ExtractionConfig): Promise<ExtractionResult>
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `content` | `Buffer` | Yes | The byte array to extract |
| `mimeType` | `string` | Yes | MIME type of the content |
| `config` | `ExtractionConfig` | Yes | Extraction configuration |

**Returns:** `ExtractionResult`
**Errors:** Throws `Error` with a descriptive message.

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

```typescript
function extractFile(path: string, mimeType?: string, config: ExtractionConfig): Promise<ExtractionResult>
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `path` | `string` | Yes | Path to the file to extract |
| `mimeType` | `string \| null` | No | Optional MIME type override. If None, will be auto-detected |
| `config` | `ExtractionConfig` | Yes | Extraction configuration |

**Returns:** `ExtractionResult`
**Errors:** Throws `Error` with a descriptive message.

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

```typescript
function extractFileSync(path: string, mimeType?: string, config: ExtractionConfig): ExtractionResult
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `path` | `string` | Yes | Path to the file |
| `mimeType` | `string \| null` | No | The mime type |
| `config` | `ExtractionConfig` | Yes | The configuration options |

**Returns:** `ExtractionResult`
**Errors:** Throws `Error` with a descriptive message.

---

#### extractBytesSync()

Synchronous wrapper for `extract_bytes`.

Uses the global Tokio runtime for 100x+ performance improvement over creating
a new runtime per call.

With the `tokio-runtime` feature, this blocks the current thread using the global
Tokio runtime. Without it (WASM), this calls a truly synchronous implementation.

**Signature:**

```typescript
function extractBytesSync(content: Buffer, mimeType: string, config: ExtractionConfig): ExtractionResult
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `content` | `Buffer` | Yes | The content to process |
| `mimeType` | `string` | Yes | The mime type |
| `config` | `ExtractionConfig` | Yes | The configuration options |

**Returns:** `ExtractionResult`
**Errors:** Throws `Error` with a descriptive message.

---

#### batchExtractFilesSync()

Synchronous wrapper for `batch_extract_files`.

Uses the global Tokio runtime for optimal performance.
Only available with `tokio-runtime` (WASM has no filesystem).

**Signature:**

```typescript
function batchExtractFilesSync(items: Array<BatchFileItem>, config: ExtractionConfig): Array<ExtractionResult>
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `items` | `Array<BatchFileItem>` | Yes | The items |
| `config` | `ExtractionConfig` | Yes | The configuration options |

**Returns:** `Array<ExtractionResult>`
**Errors:** Throws `Error` with a descriptive message.

---

#### batchExtractBytesSync()

Synchronous wrapper for `batch_extract_bytes`.

Uses the global Tokio runtime for optimal performance.
With the `tokio-runtime` feature, this blocks the current thread using the global
Tokio runtime. Without it (WASM), this calls a truly synchronous implementation
that iterates through items and calls `extract_bytes_sync()`.

**Signature:**

```typescript
function batchExtractBytesSync(items: Array<BatchBytesItem>, config: ExtractionConfig): Array<ExtractionResult>
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `items` | `Array<BatchBytesItem>` | Yes | The items |
| `config` | `ExtractionConfig` | Yes | The configuration options |

**Returns:** `Array<ExtractionResult>`
**Errors:** Throws `Error` with a descriptive message.

---

#### batchExtractFiles()

Extract content from multiple files concurrently.

This function processes multiple files in parallel, automatically managing
concurrency to prevent resource exhaustion. The concurrency limit can be
configured via `ExtractionConfig.max_concurrent_extractions` or defaults
to `(num_cpus * 1.5).ceil()`.

Each file can optionally specify a `FileExtractionConfig` that overrides specific
fields from the batch-level `config`. Pass `null` for a file to use the batch defaults.
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

```typescript
function batchExtractFiles(items: Array<BatchFileItem>, config: ExtractionConfig): Promise<Array<ExtractionResult>>
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `items` | `Array<BatchFileItem>` | Yes | Vector of `BatchFileItem` structs, each containing a path and optional |
| `config` | `ExtractionConfig` | Yes | Batch-level extraction configuration (provides defaults and batch settings) |

**Returns:** `Array<ExtractionResult>`
**Errors:** Throws `Error` with a descriptive message.

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

  MIME type, and optional per-item configuration overrides.

- `config` - Batch-level extraction configuration

**Returns:**

A vector of `ExtractionResult` in the same order as the input items.

Simple usage with no per-item overrides:

Per-item configuration overrides:

**Signature:**

```typescript
function batchExtractBytes(items: Array<BatchBytesItem>, config: ExtractionConfig): Promise<Array<ExtractionResult>>
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `items` | `Array<BatchBytesItem>` | Yes | Vector of `BatchBytesItem` structs, each containing content bytes, |
| `config` | `ExtractionConfig` | Yes | Batch-level extraction configuration |

**Returns:** `Array<ExtractionResult>`
**Errors:** Throws `Error` with a descriptive message.

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

```typescript
function detectMimeTypeFromBytes(content: Buffer): string
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `content` | `Buffer` | Yes | Raw file bytes |

**Returns:** `string`
**Errors:** Throws `Error` with a descriptive message.

---

#### getExtensionsForMime()

Get file extensions for a given MIME type.

Returns all known file extensions that map to the specified MIME type.

**Returns:**

A vector of file extensions (without leading dot) for the MIME type.

**Signature:**

```typescript
function getExtensionsForMime(mimeType: string): Array<string>
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `mimeType` | `string` | Yes | The MIME type to look up |

**Returns:** `Array<string>`
**Errors:** Throws `Error` with a descriptive message.

---

#### detectQrCodes()

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

```typescript
function detectQrCodes(imageBytes: Buffer, formatHint?: string): Array<QrCode>
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `imageBytes` | `Buffer` | Yes | The image bytes |
| `formatHint` | `string \| null` | No | The  format hint |

**Returns:** `Array<QrCode>`

---

#### clearEmbeddingBackends()

Clear all embedding backends from the global registry.

Calls `shutdown()` on every registered backend, then empties the registry.

**Errors:**

- Any error returned by a backend's `shutdown()` method. The first error
  encountered stops processing of remaining backends.

**Signature:**

```typescript
function clearEmbeddingBackends(): void
```

**Returns:** `void`
**Errors:** Throws `Error` with a descriptive message.

---

#### listEmbeddingBackends()

List the names of all registered embedding backends.

Used by `kreuzberg-cli`, the api/mcp endpoints, and generated language
bindings.

**Signature:**

```typescript
function listEmbeddingBackends(): Array<string>
```

**Returns:** `Array<string>`
**Errors:** Throws `Error` with a descriptive message.

---

#### listDocumentExtractors()

List names of all registered document extractors.

**Signature:**

```typescript
function listDocumentExtractors(): Array<string>
```

**Returns:** `Array<string>`
**Errors:** Throws `Error` with a descriptive message.

---

#### clearDocumentExtractors()

Clear all document extractors from the global registry.

Calls `shutdown()` on every registered extractor, then empties the registry.

**Errors:**

- Any error returned by an extractor's `shutdown()` method. The first error
  encountered stops processing of remaining extractors.

**Signature:**

```typescript
function clearDocumentExtractors(): void
```

**Returns:** `void`
**Errors:** Throws `Error` with a descriptive message.

---

#### listOcrBackends()

List all registered OCR backends.

Returns the names of all OCR backends currently registered in the global registry.

**Returns:**

A vector of OCR backend names.

**Signature:**

```typescript
function listOcrBackends(): Array<string>
```

**Returns:** `Array<string>`
**Errors:** Throws `Error` with a descriptive message.

---

#### clearOcrBackends()

Clear all OCR backends from the global registry.

Removes all OCR backends and calls their `shutdown()` methods.

**Returns:**

- `Ok(())` if all backends were cleared successfully
- `Err(...)` if any shutdown method failed

**Signature:**

```typescript
function clearOcrBackends(): void
```

**Returns:** `void`
**Errors:** Throws `Error` with a descriptive message.

---

#### registerBuiltin()

Register every built-in post-processor enabled by the active feature set.

This is the single entry point that callers (including
`register_default_post_processors`) use to populate the global
post-processor registry with the in-tree built-ins. Each submodule's own
`register` function is gated by its feature flag so this aggregate stays
safe to call on any target.

**Signature:**

```typescript
function registerBuiltin(): void
```

**Returns:** `void`
**Errors:** Throws `Error` with a descriptive message.

---

#### listPostProcessors()

List all registered post-processor names.

Returns a vector of all post-processor names currently registered in the
global registry.

**Returns:**

- `Ok(Vec<String>)` - Vector of post-processor names
- `Err(...)` if the registry lock is poisoned

**Signature:**

```typescript
function listPostProcessors(): Array<string>
```

**Returns:** `Array<string>`
**Errors:** Throws `Error` with a descriptive message.

---

#### clearPostProcessors()

Remove all registered post-processors.

**Signature:**

```typescript
function clearPostProcessors(): void
```

**Returns:** `void`
**Errors:** Throws `Error` with a descriptive message.

---

#### listRenderers()

List names of all registered renderers.

**Errors:**

Returns an error if the registry lock is poisoned.

**Signature:**

```typescript
function listRenderers(): Array<string>
```

**Returns:** `Array<string>`
**Errors:** Throws `Error` with a descriptive message.

---

#### clearRenderers()

Clear all renderers from the global registry.

Removes every renderer, including the built-in defaults (markdown, html,
djot, plain). After calling this no renderers are registered; re-register
as needed.

**Errors:**

Returns an error if the registry lock is poisoned.

**Signature:**

```typescript
function clearRenderers(): void
```

**Returns:** `void`
**Errors:** Throws `Error` with a descriptive message.

---

#### listValidators()

List names of all registered validators.

**Signature:**

```typescript
function listValidators(): Array<string>
```

**Returns:** `Array<string>`
**Errors:** Throws `Error` with a descriptive message.

---

#### clearValidators()

Remove all registered validators.

**Signature:**

```typescript
function clearValidators(): void
```

**Returns:** `void`
**Errors:** Throws `Error` with a descriptive message.

---

#### classifyPages()

Run page classification against an extraction result.

Mutates `result.page_classifications` with one entry per non-empty page and
appends every LLM call's usage to `result.llm_usage`.

**Errors:**

Returns the first error encountered when rendering the prompt or calling the
LLM. Partially produced classifications are discarded so callers do not see
a half-populated vector.

**Signature:**

```typescript
function classifyPages(result: ExtractionResult, config: PageClassificationConfig): Promise<void>
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `result` | `ExtractionResult` | Yes | The extraction result |
| `config` | `PageClassificationConfig` | Yes | The configuration options |

**Returns:** `void`
**Errors:** Throws `Error` with a descriptive message.

---

#### downloadModel()

Eagerly download a NER model into the kreuzberg cache.

`name` is a HuggingFace repo id (e.g. `urchade/gliner_multi-v2.1`). The
CLI flag `kreuzberg warm --ner` delegates here.

**Signature:**

```typescript
function downloadModel(name: string, cacheDir?: string): string
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `name` | `string` | Yes | The name |
| `cacheDir` | `string \| null` | No | The cache dir |

**Returns:** `string`
**Errors:** Throws `Error` with a descriptive message.

---

#### defaultModelName()

Pinned default NER model identifier.

**Signature:**

```typescript
function defaultModelName(): string
```

**Returns:** `string`

---

#### knownModels()

All NER models kreuzberg knows about (used by `--all-ner-models`).

**Signature:**

```typescript
function knownModels(): Array<string>
```

**Returns:** `Array<string>`

---

#### redact()

Run pattern redaction (and optional NER-driven redaction) over `result` and
rewrite every textual field. Populates `result.redaction_report`.

**Signature:**

```typescript
function redact(result: ExtractionResult, config: RedactionConfig): Promise<void>
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `result` | `ExtractionResult` | Yes | The extraction result |
| `config` | `RedactionConfig` | Yes | The configuration options |

**Returns:** `void`
**Errors:** Throws `Error` with a descriptive message.

---

#### findAll()

Find all US Social Security Number spans in `text` (format: NNN-NN-NNNN).

**Signature:**

```typescript
function findAll(text: string): Array<PatternMatch>
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `string` | Yes | The text |

**Returns:** `Array<PatternMatch>`

---

#### scanText()

Scan `text` for every PII category in `categories` and return all matches
in source-byte order.

When `categories` is empty every supported regex-detectable category fires.
Person / Organization / Location are *not* covered by the pattern engine —
they must be supplied by a NER backend through the redaction engine.

**Signature:**

```typescript
function scanText(text: string, categories: Array<PiiCategory>): Array<PatternMatch>
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `string` | Yes | The text |
| `categories` | `Array<PiiCategory>` | Yes | The categories |

**Returns:** `Array<PatternMatch>`

---

#### summarize()

Score and return the top-N sentences from `text`, joined in original order.

`language` is an ISO 639 (or locale) code used to pick a stopword list;
pass `null` (or an unknown code) to fall back to English.
`max_tokens` bounds the summary length by whitespace-separated tokens;
`null` falls back to `DEFAULT_MAX_TOKENS`.

**Signature:**

```typescript
function summarize(text: string, language?: string, maxTokens?: number): string | null
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `string` | Yes | The text |
| `language` | `string \| null` | No | The language |
| `maxTokens` | `number \| null` | No | The max tokens |

**Returns:** `string | null`

---

#### tokenCount()

Count whitespace-separated tokens (used for token-budget bookkeeping by
callers).

**Signature:**

```typescript
function tokenCount(text: string): number
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `string` | Yes | The text |

**Returns:** `number`

---

#### translateResult()

Translate the extraction result in place.

Populates `result.translation` with the translated `content`, optionally the
translated `formatted_content` (when `preserve_markup = true`), and rewrites
every chunk's `content` field. Every LLM call's usage is appended to
`result.llm_usage`.

**Signature:**

```typescript
function translateResult(result: ExtractionResult, config: TranslationConfig): Promise<void>
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `result` | `ExtractionResult` | Yes | The extraction result |
| `config` | `TranslationConfig` | Yes | The configuration options |

**Returns:** `void`
**Errors:** Throws `Error` with a descriptive message.

---

#### compare()

Compare two extraction results and return a structured diff.

The comparison is purely structural — no I/O, no side effects. All fields
of `ExtractionDiff` are populated according to the provided `DiffOptions`.

**Signature:**

```typescript
function compare(a: ExtractionResult, b: ExtractionResult, opts: DiffOptions): ExtractionDiff
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `a` | `ExtractionResult` | Yes | The extraction result |
| `b` | `ExtractionResult` | Yes | The extraction result |
| `opts` | `DiffOptions` | Yes | The options to use |

**Returns:** `ExtractionDiff`

---

#### extractRegionWithVlm()

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

```typescript
function extractRegionWithVlm(imageBytes: Buffer, imageMime: string, regionKind: RegionKind, llmConfig: LlmConfig, customPrompt?: string): Promise<string>
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `imageBytes` | `Buffer` | Yes | The image bytes |
| `imageMime` | `string` | Yes | The image mime |
| `regionKind` | `RegionKind` | Yes | The region kind |
| `llmConfig` | `LlmConfig` | Yes | The llm config |
| `customPrompt` | `string \| null` | No | The custom prompt |

**Returns:** `string`
**Errors:** Throws `Error` with a descriptive message.

---

#### renderPdfPageToPng()

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

```typescript
function renderPdfPageToPng(pdfBytes: Buffer, pageIndex: number, dpi?: number, password?: string): Buffer
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `pdfBytes` | `Buffer` | Yes | Raw PDF file bytes |
| `pageIndex` | `number` | Yes | Zero-based page index |
| `dpi` | `number \| null` | No | Resolution in dots per inch (default: 150) |
| `password` | `string \| null` | No | Optional password for encrypted PDFs |

**Returns:** `Buffer`
**Errors:** Throws `Error` with a descriptive message.

---

#### detectMimeType()

Detect the MIME type of a file at the given path.

Uses the file extension and optionally the file content to determine the MIME type.
Set `check_exists` to `true` to verify the file exists before detection.

**Signature:**

```typescript
function detectMimeType(path: string, checkExists: boolean): string
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `path` | `string` | Yes | Path to the file |
| `checkExists` | `boolean` | Yes | The check exists |

**Returns:** `string`
**Errors:** Throws `Error` with a descriptive message.

---

#### embedTextsAsync()

**Signature:**

```typescript
function embedTextsAsync(texts: Array<string>, config: EmbeddingConfig): Promise<Array<Array<number>>>
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `texts` | `Array<string>` | Yes | The  texts |
| `config` | `EmbeddingConfig` | Yes | The embedding config |

**Returns:** `Array<Array<number>>`
**Errors:** Throws `Error` with a descriptive message.

---

#### getEmbeddingPreset()

Get an embedding preset by name.

Returns `null` if no preset with the given name exists. Returns an owned
clone so the value is safe to pass across FFI boundaries.

**Signature:**

```typescript
function getEmbeddingPreset(name: string): EmbeddingPreset | null
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `name` | `string` | Yes | The name |

**Returns:** `EmbeddingPreset | null`

---

#### listEmbeddingPresets()

List the names of all available embedding presets.

Returns owned `String`s so the values are safe to pass across FFI boundaries.

**Signature:**

```typescript
function listEmbeddingPresets(): Array<string>
```

**Returns:** `Array<string>`

---

### Types

#### AccelerationConfig

Hardware acceleration configuration for ONNX Runtime models.

Controls which execution provider (CPU, CoreML, CUDA, TensorRT) is used
for inference in layout detection and embedding generation.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `provider` | `ExecutionProviderType` | `ExecutionProviderType.Auto` | Execution provider to use for ONNX inference. |
| `deviceId` | `number` | — | GPU device ID (for CUDA/TensorRT). Ignored for CPU/CoreML/Auto. |

---

#### ArchiveEntry

A single file extracted from an archive.

When archives (ZIP, TAR, 7Z, GZIP) are extracted with recursive extraction
enabled, each processable file produces its own full `ExtractionResult`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `path` | `string` | — | Archive-relative file path (e.g. "folder/document.pdf"). |
| `mimeType` | `string` | — | Detected MIME type of the file. |
| `result` | `ExtractionResult` | — | Full extraction result for this file. |

---

#### ArchiveMetadata

Archive (ZIP/TAR/7Z) metadata.

Extracted from compressed archive files containing file lists and size information.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `format` | `string` | — | Archive format ("ZIP", "TAR", "7Z", etc.) |
| `fileCount` | `number` | — | Total number of files in the archive |
| `fileList` | `Array<string>` | `[]` | List of file paths within the archive |
| `totalSize` | `number` | — | Total uncompressed size in bytes |
| `compressedSize` | `number \| null` | `null` | Compressed size in bytes (if available) |

---

#### AudioMetadata

Audio/video file metadata.

Populated from container tags (ID3v2, MP4 atoms, Vorbis comments, etc.) and
PCM decode properties. Available when the `transcription-types` feature is enabled.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `durationMs` | `number \| null` | `null` | Duration in milliseconds derived from the decoded audio stream. |
| `codec` | `string \| null` | `null` | Audio codec (e.g. "mp3", "aac", "opus", "flac"). |
| `container` | `string \| null` | `null` | Container format (e.g. "mpeg", "mp4", "ogg", "wav"). |
| `sampleRateHz` | `number \| null` | `null` | Sample rate in Hz after decode (always 16000 when resampled for Whisper). |
| `channels` | `number \| null` | `null` | Number of audio channels (1 = mono, 2 = stereo). |
| `bitrate` | `number \| null` | `null` | Audio bitrate in kbps from the source file tags/properties. |

---

#### BBox

Bounding box in original image coordinates (x1, y1) top-left, (x2, y2) bottom-right.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `x1` | `number` | — | Left edge (x-coordinate of the top-left corner). |
| `y1` | `number` | — | Top edge (y-coordinate of the top-left corner). |
| `x2` | `number` | — | Right edge (x-coordinate of the bottom-right corner). |
| `y2` | `number` | — | Bottom edge (y-coordinate of the bottom-right corner). |

---

#### BatchBytesItem

Batch item for byte array extraction.

Used with `batch_extract_bytes` and `batch_extract_bytes_sync`
to represent a single item in a batch extraction job.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `Buffer` | — | The content bytes to extract from |
| `mimeType` | `string` | — | MIME type of the content (e.g., "application/pdf", "text/html") |
| `config` | `FileExtractionConfig \| null` | `null` | Per-item configuration overrides (None uses batch-level defaults) |

---

#### BatchFileItem

Batch item for file extraction.

Used with `batch_extract_files` and `batch_extract_files_sync`
to represent a single file in a batch extraction job.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `path` | `string` | — | Path to the file to extract from |
| `config` | `FileExtractionConfig \| null` | `null` | Per-file configuration overrides (None uses batch-level defaults) |

---

#### BibtexMetadata

BibTeX bibliography metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `entryCount` | `number` | — | Number of entries in the bibliography. |
| `citationKeys` | `Array<string>` | `[]` | BibTeX citation keys (e.g. `"knuth1984"`) for all entries. |
| `authors` | `Array<string>` | `[]` | Author names collected across all bibliography entries. |
| `yearRange` | `YearRange \| null` | `null` | Earliest and latest publication years found in the bibliography. |
| `entryTypes` | `Record<string, number> \| null` | `{}` | Count of entries grouped by BibTeX entry type (e.g. `"article"` → 5). |

---

#### BoundingBox

Bounding box coordinates for element positioning.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `x0` | `number` | — | Left x-coordinate |
| `y0` | `number` | — | Bottom y-coordinate |
| `x1` | `number` | — | Right x-coordinate |
| `y1` | `number` | — | Top y-coordinate |

---

#### CacheStats

Aggregate statistics for a kreuzberg cache directory.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `totalFiles` | `number` | — | Total number of files currently in the cache directory. |
| `totalSizeMb` | `number` | — | Combined size of all cache files in megabytes. |
| `availableSpaceMb` | `number` | — | Free disk space available on the cache volume, in megabytes. |
| `oldestFileAgeDays` | `number` | — | Age of the oldest cache file in days (0.0 if the cache is empty). |
| `newestFileAgeDays` | `number` | — | Age of the most recently written cache file in days (0.0 if the cache is empty). |

---

#### CaptioningConfig

**Since:** `v5.0.0`

Configuration for the VLM captioning post-processor.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `llm` | `LlmConfig` | — | LLM configuration used for the VLM call. |
| `prompt` | `string \| null` | `null` | Optional custom caption prompt. `null` uses the default `RegionKind.Caption` prompt that ships with `crate.llm.region_extractor`. |
| `minImageArea` | `number` | `/* serde(default) */` | Skip images whose `width * height` is below this threshold (in pixels). Default `1_000` filters out icons and decorations. |

---

#### CellChange

A single changed cell within a table.

Defined here (rather than only in `crate.diff`) so `RevisionDelta` can
reference it unconditionally, without requiring the `diff` Cargo feature.
`crate.diff` re-exports this type verbatim.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `row` | `number` | — | Zero-based row index. |
| `col` | `number` | — | Zero-based column index. |
| `from` | `string` | — | Value before the change. |
| `to` | `string` | — | Value after the change. |

---

#### Chunk

A text chunk with optional embedding and metadata.

Chunks are created when chunking is enabled in `ExtractionConfig`. Each chunk
contains the text content, optional embedding vector (if embedding generation
is configured), and metadata about its position in the document.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `string` | — | The text content of this chunk. |
| `chunkType` | `ChunkType` | `/* serde(default) */` | Semantic structural classification of this chunk. Assigned by the heuristic classifier based on content patterns and heading context. Defaults to `ChunkType.Unknown` when no rule matches. |
| `embedding` | `Array<number> \| null` | `null` | Optional embedding vector for this chunk. Only populated when `EmbeddingConfig` is provided in chunking configuration. The dimensionality depends on the chosen embedding model. |
| `metadata` | `ChunkMetadata` | — | Metadata about this chunk's position and properties. |

---

#### ChunkMetadata

Metadata about a chunk's position in the original document.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `byteStart` | `number` | — | Byte offset where this chunk starts in the original text (UTF-8 valid boundary). |
| `byteEnd` | `number` | — | Byte offset where this chunk ends in the original text (UTF-8 valid boundary). |
| `tokenCount` | `number \| null` | `null` | Number of tokens in this chunk (if available). This is calculated by the embedding model's tokenizer if embeddings are enabled. |
| `chunkIndex` | `number` | — | Zero-based index of this chunk in the document. |
| `totalChunks` | `number` | — | Total number of chunks in the document. |
| `firstPage` | `number \| null` | `null` | First page number this chunk spans (1-indexed). Only populated when page tracking is enabled in extraction configuration. |
| `lastPage` | `number \| null` | `null` | Last page number this chunk spans (1-indexed, equal to first_page for single-page chunks). Only populated when page tracking is enabled in extraction configuration. |
| `headingContext` | `HeadingContext \| null` | `/* serde(default) */` | Heading context when using Markdown chunker. Contains the heading hierarchy this chunk falls under. Only populated when `ChunkerType.Markdown` is used. |
| `imageIndices` | `Array<number>` | `/* serde(default) */` | Indices into `ExtractionResult.images` for images on pages covered by this chunk. Contains zero-based indices into the top-level `images` collection for every image whose `page_number` falls within `[first_page, last_page]`. Empty when image extraction is disabled or the chunk spans no pages with images. |

---

#### ChunkingConfig

Chunking configuration.

Configures text chunking for document content, including chunk size,
overlap, trimming behavior, and optional embeddings.

Use `..the default constructor` when constructing to allow for future field additions:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `maxCharacters` | `number` | `1000` | Maximum size per chunk (in units determined by `sizing`). When `sizing` is `Characters` (default), this is the max character count. When using token-based sizing, this is the max token count. Default: 1000 |
| `overlap` | `number` | `200` | Overlap between chunks (in units determined by `sizing`). Default: 200 |
| `trim` | `boolean` | `true` | Whether to trim whitespace from chunk boundaries. Default: true |
| `chunkerType` | `ChunkerType` | `ChunkerType.Text` | Type of chunker to use (Text or Markdown). Default: Text |
| `embedding` | `EmbeddingConfig \| null` | `null` | Optional embedding configuration for chunk embeddings. |
| `preset` | `string \| null` | `null` | Use a preset configuration (overrides individual settings if provided). |
| `sizing` | `ChunkSizing` | `ChunkSizing.Characters` | How to measure chunk size. Default: `Characters` (Unicode character count). Enable `chunking-tiktoken` or `chunking-tokenizers` features for token-based sizing. |
| `prependHeadingContext` | `boolean` | `false` | When `true` and `chunker_type` is `Markdown`, prepend the heading hierarchy path (e.g. `"# Title > ## Section\n\n"`) to each chunk's content string. This is useful for RAG pipelines where each chunk needs self-contained context about its position in the document structure. Default: `false` |
| `topicThreshold` | `number \| null` | `null` | Optional cosine similarity threshold for semantic topic boundary detection. Only used when `chunker_type` is `Semantic` and an `EmbeddingConfig` is provided. You almost never need to set this. When omitted, defaults to `0.75` which works well for most documents. Lower values detect more topic boundaries (more, smaller chunks); higher values detect fewer. Range: `0.0..=1.0`. |

### Methods

#### default()

**Signature:**

```typescript
static default(): ChunkingConfig
```

---

#### CitationMetadata

Citation file metadata (RIS, PubMed, EndNote).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `citationCount` | `number` | — | Total number of citation records in the file. |
| `format` | `string \| null` | `null` | Detected citation file format (e.g. `"ris"`, `"pubmed"`, `"endnote"`). |
| `authors` | `Array<string>` | `[]` | Author names collected across all citation records. |
| `yearRange` | `YearRange \| null` | `null` | Earliest and latest publication years found in the file. |
| `dois` | `Array<string>` | `[]` | DOI identifiers found in the citation records. |
| `keywords` | `Array<string>` | `[]` | Keywords collected from all citation records. |

---

#### ClassificationLabel

A single label + confidence pair.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `label` | `string` | — | Label name as configured in `PageClassificationConfig.labels`. |
| `confidence` | `number \| null` | `null` | Backend-reported confidence in `[0.0, 1.0]`. `null` when the backend (e.g. an LLM prompt without explicit confidence schema) did not report one. |

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
| `includeHeaders` | `boolean` | `false` | Include running headers in extraction output. - PDF: Disables top-margin furniture stripping and prevents the layout model from treating `PageHeader`-classified regions as furniture. - DOCX: Includes document headers in text output. - RTF/ODT: Headers already included; this is a no-op when true. - HTML/EPUB: Keeps `<header>` element content. Default: `false` (headers are stripped or excluded). |
| `includeFooters` | `boolean` | `false` | Include running footers in extraction output. - PDF: Disables bottom-margin furniture stripping and prevents the layout model from treating `PageFooter`-classified regions as furniture. - DOCX: Includes document footers in text output. - RTF/ODT: Footers already included; this is a no-op when true. - HTML/EPUB: Keeps `<footer>` element content. Default: `false` (footers are stripped or excluded). |
| `stripRepeatingText` | `boolean` | `true` | Enable the heuristic cross-page repeating text detector. When `true` (default), text that repeats verbatim across a supermajority of pages is classified as furniture and stripped.  Disable this if brand names or repeated headings are being incorrectly removed by the heuristic. Note: when a layout-detection model is active, the model may independently classify page-header / page-footer regions as furniture on a per-page basis. To preserve those regions, set `include_headers = true`, `include_footers = true`, or both, in addition to disabling this flag. Primarily affects PDF extraction. Default: `true`. |
| `includeWatermarks` | `boolean` | `false` | Include watermark text in extraction output. - PDF: Keeps watermark artifacts and arXiv identifiers. - Other formats: No effect currently. Default: `false` (watermarks are stripped). |

### Methods

#### default()

**Signature:**

```typescript
static default(): ContentFilterConfig
```

---

#### ContributorRole

JATS contributor with role.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `string` | — | Contributor display name. |
| `role` | `string \| null` | `null` | Contributor role (e.g. `"author"`, `"editor"`). |

---

#### CoreProperties

Dublin Core metadata from docProps/core.xml

Contains standard metadata fields defined by the Dublin Core standard
and Office-specific extensions.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `title` | `string \| null` | `null` | Document title |
| `subject` | `string \| null` | `null` | Document subject/topic |
| `creator` | `string \| null` | `null` | Document creator/author |
| `keywords` | `string \| null` | `null` | Keywords or tags |
| `description` | `string \| null` | `null` | Document description/abstract |
| `lastModifiedBy` | `string \| null` | `null` | User who last modified the document |
| `revision` | `string \| null` | `null` | Revision number |
| `created` | `string \| null` | `null` | Creation timestamp (ISO 8601) |
| `modified` | `string \| null` | `null` | Last modification timestamp (ISO 8601) |
| `category` | `string \| null` | `null` | Document category |
| `contentStatus` | `string \| null` | `null` | Content status (Draft, Final, etc.) |
| `language` | `string \| null` | `null` | Document language |
| `identifier` | `string \| null` | `null` | Unique identifier |
| `version` | `string \| null` | `null` | Document version |
| `lastPrinted` | `string \| null` | `null` | Last print timestamp (ISO 8601) |

---

#### CsvMetadata

CSV/TSV file metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `rowCount` | `number` | — | Total number of data rows (excluding the header row if present). |
| `columnCount` | `number` | — | Number of columns detected. |
| `delimiter` | `string \| null` | `null` | Field delimiter character (e.g. `","` or `"\t"`). |
| `hasHeader` | `boolean` | — | Whether the first row was treated as a header. |
| `columnTypes` | `Array<string> \| null` | `[]` | Inferred data type for each column (e.g. `"string"`, `"integer"`, `"float"`). |

---

#### DbfFieldInfo

dBASE field information.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `string` | — | Field (column) name. |
| `fieldType` | `string` | — | dBASE field type character (e.g. `"C"` for character, `"N"` for numeric). |

---

#### DbfMetadata

dBASE (DBF) file metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `recordCount` | `number` | — | Total number of data records in the DBF file. |
| `fieldCount` | `number` | — | Number of field (column) definitions. |
| `fields` | `Array<DbfFieldInfo>` | `[]` | Descriptor for each field in the table schema. |

---

#### DetectResponse

MIME type detection response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `mimeType` | `string` | — | Detected MIME type |
| `filename` | `string \| null` | `null` | Original filename (if provided) |

---

#### DetectionResult

Page-level detection result containing all detections and page metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `pageWidth` | `number` | — | Page width in pixels (as seen by the model). |
| `pageHeight` | `number` | — | Page height in pixels (as seen by the model). |
| `detections` | `Array<LayoutDetection>` | — | All layout detections on this page after postprocessing. |

---

#### DiffHunk

A single contiguous hunk in a unified diff.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `fromLine` | `number` | — | Starting line number in the old content (0-indexed). |
| `fromCount` | `number` | — | Number of lines from the old content in this hunk. |
| `toLine` | `number` | — | Starting line number in the new content (0-indexed). |
| `toCount` | `number` | — | Number of lines from the new content in this hunk. |
| `lines` | `Array<DiffLine>` | — | Lines that make up this hunk. |

---

#### DiffOptions

Options controlling how two `ExtractionResult` values are compared.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `includeMetadata` | `boolean` | `true` | Include metadata changes in the diff. Default: `true`. |
| `includeEmbedded` | `boolean` | `true` | Include embedded-children changes in the diff. Default: `true`. |
| `maxContentChars` | `number \| null` | `null` | Truncate content to this many characters before diffing. Useful for very large documents where only the first N characters matter. `null` means no truncation. |

### Methods

#### default()

**Signature:**

```typescript
static default(): DiffOptions
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
| `plainText` | `string` | — | Plain text representation for backwards compatibility |
| `blocks` | `Array<FormattedBlock>` | — | Structured block-level content |
| `metadata` | `Metadata` | — | Metadata from YAML frontmatter |
| `tables` | `Array<Table>` | — | Extracted tables as structured data |
| `images` | `Array<DjotImage>` | — | Extracted images with metadata |
| `links` | `Array<DjotLink>` | — | Extracted links with URLs |
| `footnotes` | `Array<Footnote>` | — | Footnote definitions |
| `attributes` | `Array<string>` | `/* serde(default) */` | Attributes mapped by element identifier (if present) |

---

#### DjotImage

Image element in Djot.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `src` | `string` | — | Image source URL or path |
| `alt` | `string` | — | Alternative text |
| `title` | `string \| null` | `null` | Optional title |
| `attributes` | `string \| null` | `null` | Element attributes |

---

#### DjotLink

Link element in Djot.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | `string` | — | Link URL |
| `text` | `string` | — | Link text content |
| `title` | `string \| null` | `null` | Optional title |
| `attributes` | `string \| null` | `null` | Element attributes |

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

### Methods

#### extractBytes()

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

```typescript
extractBytes(content: Buffer, mimeType: string, config: ExtractionConfig): InternalDocument
```

#### extractFile()

Extract content from a file.

Default implementation reads the file and calls `extract_bytes`.
Override for custom file handling, streaming, or memory optimizations.

**Returns:**

An `InternalDocument` containing the extracted elements, metadata, and tables.

**Errors:**

Same as `extract_bytes`, plus file I/O errors.

**Signature:**

```typescript
extractFile(path: string, mimeType: string, config: ExtractionConfig): InternalDocument
```

#### supportedMimeTypes()

Get the list of MIME types supported by this extractor.

Can include exact MIME types and prefix patterns:

- Exact: `"application/pdf"`, `"text/plain"`
- Prefix: `"image/*"` (matches any image type)

**Returns:**

A slice of MIME type strings.

**Signature:**

```typescript
supportedMimeTypes(): Array<string>
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

```typescript
priority(): number
```

#### canHandle()

Optional: Check if this extractor can handle a specific file.

Allows for more sophisticated detection beyond MIME types.
Defaults to `true` (rely on MIME type matching).

**Returns:**

`true` if the extractor can handle this file, `false` otherwise.

**Signature:**

```typescript
canHandle(path: string, mimeType: string): boolean
```

---

#### DocumentNode

A single node in the document tree.

Each node has deterministic `id`, typed `content`, optional `parent`/`children`
for tree structure, and metadata like page number, bounding box, and content layer.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `string` | — | Deterministic identifier (hash of content + position). |
| `content` | `NodeContent` | — | Node content — tagged enum, type-specific data only. |
| `parent` | `number \| null` | `null` | Parent node index (`null` = root-level node). |
| `children` | `Array<number>` | `/* serde(default) */` | Child node indices in reading order. |
| `contentLayer` | `ContentLayer` | `/* serde(default) */` | Content layer classification. |
| `page` | `number \| null` | `null` | Page number where this node starts (1-indexed). |
| `pageEnd` | `number \| null` | `null` | Page number where this node ends (for multi-page tables/sections). |
| `bbox` | `BoundingBox \| null` | `null` | Bounding box in document coordinates. |
| `annotations` | `Array<TextAnnotation>` | `/* serde(default) */` | Inline annotations (formatting, links) on this node's text content. Only meaningful for text-carrying nodes; empty for containers. |
| `attributes` | `Record<string, string> \| null` | `null` | Format-specific key-value attributes. Extensible bag for miscellaneous data without a dedicated typed field: CSS classes, LaTeX environment names, Excel cell formulas, slide layout names, etc. |

---

#### DocumentRelationship

A resolved relationship between two nodes in the document tree.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `source` | `number` | — | Source node index (the referencing node). |
| `target` | `number` | — | Target node index (the referenced node). |
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
| `revisionId` | `string` | — | Format-specific revision identifier. For DOCX this is the `w:id` attribute value on the change element (e.g. `"42"`). When the attribute is absent a synthetic fallback is generated (`"docx-ins-0"`, `"docx-del-3"`, …). |
| `author` | `string \| null` | `null` | Display name of the author who made this change, when available. |
| `timestamp` | `string \| null` | `null` | ISO-8601 timestamp of the change, when available. Stored as a plain string so this type remains FFI-friendly and unconditionally available without the `chrono` optional dep. DOCX populates this from the `w:date` attribute (e.g. `"2024-03-15T10:30:00Z"`). |
| `kind` | `RevisionKind` | — | Semantic kind of this revision. |
| `anchor` | `RevisionAnchor \| null` | `null` | Best-effort document location for this revision. Resolution is format-dependent and may be `null` when the location cannot be determined (e.g. changes inside table cells before table-cell anchor support is added). |
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
| `nodes` | `Array<DocumentNode>` | `[]` | All nodes in document/reading order. |
| `sourceFormat` | `string \| null` | `null` | Origin format identifier (e.g. "docx", "pptx", "html", "pdf"). Allows renderers to apply format-aware heuristics when converting the document tree to output formats. |
| `relationships` | `Array<DocumentRelationship>` | `[]` | Resolved relationships between nodes (footnote refs, citations, anchor links, etc.). Populated during derivation from the internal document representation. Empty when no relationships are detected. |
| `nodeTypes` | `Array<string>` | `[]` | Sorted, deduplicated list of node type names present in this document. Each value is the snake_case `node_type` tag of the corresponding `NodeContent` variant (e.g. `"paragraph"`, `"heading"`, `"table"`, …). Computed from `nodes` via `DocumentStructure.finalize_node_types`. Empty until that method is called (internal construction paths call it at the end of derivation). |

### Methods

#### finalizeNodeTypes()

Compute and populate the `node_types` field from the current `nodes`.

Call this after all nodes have been added to the structure. Internal
construction paths (builder, derivation) call this automatically.

**Signature:**

```typescript
finalizeNodeTypes(): void
```

#### isEmpty()

Check if the document structure is empty.

**Signature:**

```typescript
isEmpty(): boolean
```

#### default()

**Signature:**

```typescript
static default(): DocumentStructure
```

---

#### DocumentSummary

Summary of an extracted document.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text` | `string` | — | Summary text (plain prose). |
| `strategy` | `SummaryStrategy` | — | Strategy that produced this summary. |
| `tokenCount` | `number \| null` | `null` | Approximate token count of the summary, when known. |

---

#### DocxAppProperties

Application properties from docProps/app.xml for DOCX

Contains Word-specific document statistics and metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `application` | `string \| null` | `null` | Application name (e.g., "Microsoft Office Word") |
| `appVersion` | `string \| null` | `null` | Application version |
| `template` | `string \| null` | `null` | Template filename |
| `totalTime` | `number \| null` | `null` | Total editing time in minutes |
| `pages` | `number \| null` | `null` | Number of pages |
| `words` | `number \| null` | `null` | Number of words |
| `characters` | `number \| null` | `null` | Number of characters (excluding spaces) |
| `charactersWithSpaces` | `number \| null` | `null` | Number of characters (including spaces) |
| `lines` | `number \| null` | `null` | Number of lines |
| `paragraphs` | `number \| null` | `null` | Number of paragraphs |
| `company` | `string \| null` | `null` | Company name |
| `docSecurity` | `number \| null` | `null` | Document security level |
| `scaleCrop` | `boolean \| null` | `null` | Scale crop flag |
| `linksUpToDate` | `boolean \| null` | `null` | Links up to date flag |
| `sharedDoc` | `boolean \| null` | `null` | Shared document flag |
| `hyperlinksChanged` | `boolean \| null` | `null` | Hyperlinks changed flag |

---

#### DocxMetadata

Word document metadata.

Extracted from DOCX files using shared Office Open XML metadata extraction.
Integrates with `office_metadata` module for core/app/custom properties.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `coreProperties` | `CoreProperties \| null` | `null` | Core properties from docProps/core.xml (Dublin Core metadata) Contains title, creator, subject, keywords, dates, etc. Shared format across DOCX/PPTX/XLSX documents. |
| `appProperties` | `DocxAppProperties \| null` | `null` | Application properties from docProps/app.xml (Word-specific statistics) Contains word count, page count, paragraph count, editing time, etc. DOCX-specific variant of Office application properties. |
| `customProperties` | `Record<string, unknown> \| null` | `{}` | Custom properties from docProps/custom.xml (user-defined properties) Contains key-value pairs defined by users or applications. Values can be strings, numbers, booleans, or dates. |

---

#### Element

Semantic element extracted from document.

Represents a logical unit of content with semantic classification,
unique identifier, and metadata for tracking origin and position.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `elementId` | `string` | — | Unique element identifier |
| `elementType` | `ElementType` | — | Semantic type of this element |
| `text` | `string` | — | Text content of the element |
| `metadata` | `ElementMetadata` | — | Metadata about the element |

---

#### ElementMetadata

Metadata for a semantic element.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `pageNumber` | `number \| null` | `null` | Page number (1-indexed) |
| `filename` | `string \| null` | `null` | Source filename or document name |
| `coordinates` | `BoundingBox \| null` | `null` | Bounding box coordinates if available |
| `elementIndex` | `number \| null` | `null` | Position index in the element sequence |
| `additional` | `Record<string, string>` | — | Additional custom metadata |

---

#### EmailAttachment

Email attachment representation.

Contains metadata and optionally the content of an email attachment.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `string \| null` | `null` | Attachment name (from Content-Disposition header) |
| `filename` | `string \| null` | `null` | Filename of the attachment |
| `mimeType` | `string \| null` | `null` | MIME type of the attachment |
| `size` | `number \| null` | `null` | Size in bytes |
| `isImage` | `boolean` | — | Whether this attachment is an image |
| `data` | `Buffer \| null` | `null` | Attachment data (if extracted). Uses `bytes.Bytes` for cheap cloning of large buffers. |

---

#### EmailConfig

Configuration for email extraction.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `msgFallbackCodepage` | `number \| null` | `null` | Windows codepage number to use when an MSG file contains no codepage property. Defaults to `null`, which falls back to windows-1252. If an unrecognized or invalid codepage number is supplied (including 0), the behavior silently falls back to windows-1252 — the same as when the MSG file itself contains an unrecognized codepage. No error or warning is emitted. Users should verify output when supplying unusual values. Common values: - 1250: Central European (Polish, Czech, Hungarian, etc.) - 1251: Cyrillic (Russian, Ukrainian, Bulgarian, etc.) - 1252: Western European (default) - 1253: Greek - 1254: Turkish - 1255: Hebrew - 1256: Arabic - 932:  Japanese (Shift-JIS) - 936:  Simplified Chinese (GBK) |

---

#### EmailExtractionResult

Email extraction result.

Complete representation of an extracted email message (.eml or .msg)
including headers, body content, and attachments.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `subject` | `string \| null` | `null` | Email subject line |
| `fromEmail` | `string \| null` | `null` | Sender email address |
| `toEmails` | `Array<string>` | — | Primary recipient email addresses |
| `ccEmails` | `Array<string>` | — | CC recipient email addresses |
| `bccEmails` | `Array<string>` | — | BCC recipient email addresses |
| `date` | `string \| null` | `null` | Email date/timestamp |
| `messageId` | `string \| null` | `null` | Message-ID header value |
| `plainText` | `string \| null` | `null` | Plain text version of the email body |
| `htmlContent` | `string \| null` | `null` | HTML version of the email body |
| `content` | `string` | — | Cleaned/processed text content. Aliased as `cleaned_text` for back-compat. |
| `attachments` | `Array<EmailAttachment>` | — | List of email attachments |
| `metadata` | `Record<string, string>` | — | Additional email headers and metadata |

---

#### EmailMetadata

Email metadata extracted from .eml and .msg files.

Includes sender/recipient information, message ID, and attachment list.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `fromEmail` | `string \| null` | `null` | Sender's email address |
| `fromName` | `string \| null` | `null` | Sender's display name |
| `toEmails` | `Array<string>` | `[]` | Primary recipients |
| `ccEmails` | `Array<string>` | `[]` | CC recipients |
| `bccEmails` | `Array<string>` | `[]` | BCC recipients |
| `messageId` | `string \| null` | `null` | Message-ID header value |
| `attachments` | `Array<string>` | `[]` | List of attachment filenames |

---

#### EmbeddedChanges

Changes to embedded archive children between two results.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `added` | `Array<ArchiveEntry>` | — | Children present in `b` but not in `a` (matched by `path`). |
| `removed` | `Array<ArchiveEntry>` | — | Children present in `a` but not in `b` (matched by `path`). |
| `changed` | `Array<EmbeddedDiff>` | — | Children present in both but with differing content (matched by `path`). Each entry holds the diff of the nested `ExtractionResult`. |

---

#### EmbeddedDiff

Diff for a single embedded archive entry that appears in both results.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `path` | `string` | — | Archive-relative path identifying this entry. |
| `diff` | `ExtractionDiff` | — | The recursive diff of the entry's extraction result. |

---

#### EmbeddedFile

Embedded file descriptor extracted from the PDF name tree.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `string` | — | The filename as stored in the PDF name tree. |
| `data` | `Buffer` | — | Raw file bytes from the embedded stream (already decompressed by lopdf). |
| `compressedSize` | `number` | — | Compressed byte count of the original stream (before decompression). Used by callers to compute the decompression ratio and detect zip-bomb-style attacks that embed a tiny compressed stream expanding to gigabytes of data. |
| `mimeType` | `string \| null` | `null` | MIME type if specified in the filespec, otherwise `null`. |

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

### Methods

#### dimensions()

Embedding vector dimension. Must be `> 0` and must match the length of
every vector returned by `embed`.

**Signature:**

```typescript
dimensions(): number
```

#### embed()

Embed a batch of texts, returning one vector per input in order.

**Errors:**

Implementations should return `Plugin` for
backend-specific failures. The dispatcher layers its own validation
(length, per-vector dimension) on top.

**Signature:**

```typescript
embed(texts: Array<string>): Array<Array<number>>
```

---

#### EmbeddingConfig

Embedding configuration for text chunks.

Configures embedding generation using ONNX models via the vendored embedding engine.
Requires the `embeddings` feature to be enabled.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `EmbeddingModelType` | `EmbeddingModelType.Preset` | The embedding model to use (defaults to "balanced" preset if not specified) |
| `normalize` | `boolean` | `true` | Whether to normalize embedding vectors (recommended for cosine similarity) |
| `batchSize` | `number` | `32` | Batch size for embedding generation |
| `showDownloadProgress` | `boolean` | `false` | Show model download progress |
| `cacheDir` | `string \| null` | `null` | Custom cache directory for model files Defaults to `~/.cache/kreuzberg/embeddings/` if not specified. Allows full customization of model download location. |
| `acceleration` | `AccelerationConfig \| null` | `null` | Hardware acceleration for the embedding ONNX model. When set, controls which execution provider (CPU, CUDA, CoreML, TensorRT) is used for inference. Defaults to `null` (auto-select per platform). |
| `maxEmbedDurationSecs` | `number \| null` | `null` | Maximum wall-clock duration (in seconds) for a single `embed()` call when using `EmbeddingModelType.Plugin`. Applies only to the in-process plugin path — protects against hung host-language backends (e.g. a Python callback deadlocked on the GIL, a model stuck on CUDA OOM retries, etc.). On timeout, the dispatcher returns `Plugin` instead of blocking forever. `null` disables the timeout. The default (60 seconds) is conservative for common in-process inference; increase for large batches on slow hardware. |

### Methods

#### default()

**Signature:**

```typescript
static default(): EmbeddingConfig
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
| `name` | `string` | — | Short identifier for this preset (e.g. `"balanced"`, `"fast"`, `"quality"`). |
| `chunkSize` | `number` | — | Target chunk size in characters. |
| `overlap` | `number` | — | Overlap between consecutive chunks in characters. |
| `modelRepo` | `string` | — | HuggingFace repository name for the model. |
| `pooling` | `string` | — | Pooling strategy: "cls" or "mean". |
| `modelFile` | `string` | — | Path to the ONNX model file within the repo. |
| `dimensions` | `number` | — | Embedding vector dimension produced by this model. |
| `description` | `string` | — | Human-readable description of the preset's intended use case. |

---

#### Entity

A single named entity detected in the extracted text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `category` | `EntityCategory` | — | Canonical category the entity belongs to (PERSON, ORG, LOCATION, etc.). |
| `text` | `string` | — | Raw mention text exactly as it appeared in the source. |
| `start` | `number` | — | Byte-offset span in `ExtractionResult.content` where the mention starts. |
| `end` | `number` | — | Byte-offset span in `ExtractionResult.content` where the mention ends (exclusive). |
| `confidence` | `number \| null` | `null` | Backend-reported confidence in `[0.0, 1.0]`. `null` when the backend does not expose confidence scores. |

---

#### EpubMetadata

EPUB metadata (Dublin Core extensions).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `coverage` | `string \| null` | `null` | Dublin Core `coverage` field (geographic or temporal scope). |
| `dcFormat` | `string \| null` | `null` | Dublin Core `format` field (media type of the resource). |
| `relation` | `string \| null` | `null` | Dublin Core `relation` field (related resource identifier). |
| `source` | `string \| null` | `null` | Dublin Core `source` field (origin resource identifier). |
| `dcType` | `string \| null` | `null` | Dublin Core `type` field (nature or genre of the resource). |
| `coverImage` | `string \| null` | `null` | Path or identifier of the cover image within the EPUB container. |

---

#### ErrorMetadata

Error metadata (for batch operations).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `errorType` | `string` | — | Machine-readable error type identifier (e.g. "UnsupportedFormat"). |
| `message` | `string` | — | Human-readable error description. |

---

#### ExcelMetadata

Excel/spreadsheet format metadata.

Identifies the document as a spreadsheet source via the `FormatMetadata.Excel`
discriminant. Sheet count and sheet names are stored inside this struct.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `sheetCount` | `number \| null` | `null` | Number of sheets in the workbook. |
| `sheetNames` | `Array<string> \| null` | `[]` | Names of all sheets in the workbook. |

---

#### ExcelSheet

Single Excel worksheet.

Represents one sheet from an Excel workbook with its content
converted to Markdown format and dimensional statistics.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `string` | — | Sheet name as it appears in Excel |
| `markdown` | `string` | — | Sheet content converted to Markdown tables |
| `rowCount` | `number` | — | Number of rows |
| `colCount` | `number` | — | Number of columns |
| `cellCount` | `number` | — | Total number of non-empty cells |
| `tableCells` | `Array<Array<string>> \| null` | `null` | Pre-extracted table cells (2D vector of cell values) Populated during markdown generation to avoid re-parsing markdown. None for empty sheets. |

---

#### ExcelWorkbook

Excel workbook representation.

Contains all sheets from an Excel file (.xlsx, .xls, etc.) with
extracted content and metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `sheets` | `Array<ExcelSheet>` | — | All sheets in the workbook |
| `metadata` | `Record<string, string>` | — | Workbook-level metadata (author, creation date, etc.) |
| `revisions` | `Array<DocumentRevision> \| null` | `/* serde(default) */` | Collaborative-edit revision headers from `xl/revisions/revisionHeaders.xml`. Populated for legacy shared-workbook `.xlsx` files that contain the `xl/revisions/` directory. Each `<header>` element maps to one `DocumentRevision { kind: FormatChange }` carrying the header's `guid` (→ `revision_id`), `userName` (→ `author`), and `dateTime` (→ `timestamp`). `anchor` and `delta` are `null`/empty for v1 (per-cell log parsing is a follow-up). `null` when `xl/revisions/revisionHeaders.xml` is absent. |

---

#### ExtractedImage

Extracted image from a document.

Contains raw image data, metadata, and optional nested OCR results.
Raw bytes allow cross-language compatibility - users can convert to
PIL.Image (Python), Sharp (Node.js), or other formats as needed.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `data` | `Buffer` | — | Raw image data (PNG, JPEG, WebP, etc. bytes). Uses `bytes.Bytes` for cheap cloning of large buffers. |
| `format` | `string` | — | Image format (e.g., "jpeg", "png", "webp") Uses Cow<'static, str> to avoid allocation for static literals. |
| `imageIndex` | `number` | — | Zero-indexed position of this image in the document/page |
| `pageNumber` | `number \| null` | `null` | Page/slide number where image was found (1-indexed) |
| `width` | `number \| null` | `null` | Image width in pixels |
| `height` | `number \| null` | `null` | Image height in pixels |
| `colorspace` | `string \| null` | `null` | Colorspace information (e.g., "RGB", "CMYK", "Gray") |
| `bitsPerComponent` | `number \| null` | `null` | Bits per color component (e.g., 8, 16) |
| `isMask` | `boolean` | — | Whether this image is a mask image |
| `description` | `string \| null` | `null` | Optional description of the image |
| `ocrResult` | `ExtractionResult \| null` | `null` | Nested OCR extraction result (if image was OCRed) When OCR is performed on this image, the result is embedded here rather than in a separate collection, making the relationship explicit. |
| `boundingBox` | `BoundingBox \| null` | `null` | Bounding box of the image on the page (PDF coordinates: x0=left, y0=bottom, x1=right, y1=top). Only populated for PDF-extracted images when position data is available from the PDF extractor. |
| `sourcePath` | `string \| null` | `null` | Original source path of the image within the document archive (e.g., "media/image1.png" in DOCX). Used for rendering image references when the binary data is not extracted. |
| `imageKind` | `ImageKind \| null` | `null` | Heuristic classification of what this image likely depicts. `null` if classification was disabled or inconclusive. |
| `kindConfidence` | `number \| null` | `null` | Confidence score for `image_kind`, in the range 0.0 to 1.0. |
| `clusterId` | `number \| null` | `null` | Identifier shared across images that form a single logical figure (e.g. all raster tiles of one technical drawing). `null` for singletons. |
| `caption` | `string \| null` | `null` | VLM-generated caption describing the image, when captioning is configured. Populated by the captioning post-processor (`crates/kreuzberg/src/plugins/processor/builtin/captioning.rs`), which routes each image through `crate.llm.region_extractor.extract_region_with_vlm` in caption mode. `null` when captioning is disabled or the VLM declined to caption. |
| `qrCodes` | `Array<QrCode> \| null` | `[]` | QR codes decoded from this image, when QR detection is enabled. Populated by the QR post-processor (`crates/kreuzberg/src/extractors/qr.rs`) via the pure-Rust `rqrr` decoder. `null` when QR detection is disabled; an empty `Some(vec![])` when detection ran but found nothing. |

---

#### ExtractedUri

A URI extracted from a document.

Represents any link, reference, or resource pointer found during extraction.
The `kind` field classifies the URI semantically, while `label` carries
optional human-readable display text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | `string` | — | The URL or path string. |
| `label` | `string \| null` | `null` | Optional display text / label for the link. |
| `page` | `number \| null` | `null` | Optional page number where the URI was found (1-indexed). |
| `kind` | `UriKind` | — | Semantic classification of the URI. |

---

#### ExtractionConfig

Main extraction configuration.

This struct contains all configuration options for the extraction process.
It can be loaded from TOML, YAML, or JSON files, or created programmatically.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `useCache` | `boolean` | `true` | Enable caching of extraction results |
| `enableQualityProcessing` | `boolean` | `true` | Enable quality post-processing |
| `ocr` | `OcrConfig \| null` | `null` | OCR configuration (None = OCR disabled) |
| `forceOcr` | `boolean` | `false` | Force OCR even for searchable PDFs |
| `forceOcrPages` | `Array<number> \| null` | `null` | Force OCR on specific pages only (1-indexed page numbers, must be >= 1). When set, only the listed pages are OCR'd regardless of text layer quality. Unlisted pages use native text extraction. Ignored when `force_ocr` is `true`. Only applies to PDF documents. Duplicates are automatically deduplicated. An `ocr` config is recommended for backend/language selection; defaults are used if absent. |
| `disableOcr` | `boolean` | `false` | Disable OCR entirely, even for images. When `true`, OCR is skipped for all document types. Images return metadata only (dimensions, format, EXIF) without text extraction. PDFs use only native text extraction without OCR fallback. Cannot be `true` simultaneously with `force_ocr`. *Added in v4.7.0.* |
| `chunking` | `ChunkingConfig \| null` | `null` | Text chunking configuration (None = chunking disabled) |
| `contentFilter` | `ContentFilterConfig \| null` | `null` | Content filtering configuration (None = use extractor defaults). Controls whether document "furniture" (headers, footers, watermarks, repeating text) is included in or stripped from extraction results. See `ContentFilterConfig` for per-field documentation. |
| `images` | `ImageExtractionConfig \| null` | `null` | Image extraction configuration (None = no image extraction) |
| `pdfOptions` | `PdfConfig \| null` | `null` | PDF-specific options (None = use defaults) |
| `tokenReduction` | `TokenReductionOptions \| null` | `null` | Token reduction configuration (None = no token reduction) |
| `languageDetection` | `LanguageDetectionConfig \| null` | `null` | Language detection configuration (None = no language detection) |
| `pages` | `PageConfig \| null` | `null` | Page extraction configuration (None = no page tracking) |
| `keywords` | `KeywordConfig \| null` | `null` | Keyword extraction configuration (None = no keyword extraction) |
| `postprocessor` | `PostProcessorConfig \| null` | `null` | Post-processor configuration (None = use defaults) |
| `htmlOptions` | `string \| null` | `null` | HTML to Markdown conversion options (None = use defaults) Configure how HTML documents are converted to Markdown, including heading styles, list formatting, code block styles, and preprocessing options. |
| `htmlOutput` | `HtmlOutputConfig \| null` | `null` | Styled HTML output configuration. When set alongside `output_format = OutputFormat.Html`, the extraction pipeline uses `StyledHtmlRenderer` which emits stable `kb-*` CSS class hooks on every structural element and optionally embeds theme CSS or user-supplied CSS in a `<style>` block. When `null`, the existing plain comrak-based HTML renderer is used. |
| `extractionTimeoutSecs` | `number \| null` | `null` | Default per-file timeout in seconds for batch extraction. When set, each file in a batch will be canceled after this duration unless overridden by `FileExtractionConfig.timeout_secs`. Defaults to `Some(60)` to prevent pathological files (e.g. deeply nested archives, documents with millions of cells) from running indefinitely and exhausting caller resources. Set to `null` to disable the timeout for trusted input or long-running workloads. |
| `maxConcurrentExtractions` | `number \| null` | `null` | Maximum concurrent extractions in batch operations (None = (num_cpus × 1.5).ceil()). Limits parallelism to prevent resource exhaustion when processing large batches. Defaults to (num_cpus × 1.5).ceil() when not set. |
| `resultFormat` | `ResultFormat` | `ResultFormat.Unified` | Result structure format Controls whether results are returned in unified format (default) with all content in the `content` field, or element-based format with semantic elements (for Unstructured-compatible output). |
| `securityLimits` | `SecurityLimits \| null` | `null` | Security limits for archive extraction. Controls maximum archive size, compression ratio, file count, and other security thresholds to prevent decompression bomb attacks. Also caps nesting depth, iteration count, entity / token length, total content size, and table cell count for every extraction path that ingests user-controlled bytes. When `null`, default limits are used. |
| `maxEmbeddedFileBytes` | `number \| null` | `null` | Maximum uncompressed size in bytes for a single embedded file before recursive extraction is attempted (default: 50 MiB). Applies to embedded objects inside OOXML containers (DOCX, PPTX) and to email attachments processed via recursive extraction. Files that exceed this limit are skipped with a `ProcessingWarning` rather than passed to the extraction pipeline, preventing a single oversized embedded object from consuming unbounded memory or time. Set to `null` to disable the per-embedded-file cap (falls back to `security_limits.max_archive_size` as the only guard). |
| `outputFormat` | `OutputFormat` | `OutputFormat.Plain` | Content text format (default: Plain). Controls the format of the extracted content: - `Plain`: Raw extracted text (default) - `Markdown`: Markdown formatted output - `Djot`: Djot markup format (requires djot feature) - `Html`: HTML formatted output When set to a structured format, extraction results will include formatted output. The `formatted_content` field may be populated when format conversion is applied. |
| `layout` | `LayoutDetectionConfig \| null` | `null` | Layout detection configuration (None = layout detection disabled). When set, PDF pages and images are analyzed for document structure (headings, code, formulas, tables, figures, etc.) using RT-DETR models via ONNX Runtime. For PDFs, layout hints override paragraph classification in the markdown pipeline. For images, per-region OCR is performed with markdown formatting based on detected layout classes. Requires the `layout-detection` feature to run inference; the field is present whenever the `layout-types` feature is active (which includes `layout-detection` as well as the no-ORT target groups). |
| `useLayoutForMarkdown` | `boolean` | `false` | Run layout detection on the non-OCR PDF markdown path. When `true` and `layout` is `Some(_)`, layout regions inform heading, table, list, and figure detection in the structure pipeline that would otherwise rely on font-clustering heuristics alone. Significantly improves SF1 (structural F1) at the cost of inference latency (~150-300ms/page CPU, ~20-50ms/page GPU). Default: `false`. Requires the `layout-detection` feature. |
| `includeDocumentStructure` | `boolean` | `false` | Enable structured document tree output. When true, populates the `document` field on `ExtractionResult` with a hierarchical `DocumentStructure` containing heading-driven section nesting, table grids, content layer classification, and inline annotations. Independent of `result_format` — can be combined with Unified or ElementBased. |
| `acceleration` | `AccelerationConfig \| null` | `null` | Hardware acceleration configuration for ONNX Runtime models. Controls execution provider selection for layout detection and embedding models. When `null`, uses platform defaults (CoreML on macOS, CUDA on Linux, CPU on Windows). |
| `cacheNamespace` | `string \| null` | `null` | Cache namespace for tenant isolation. When set, cache entries are stored under `{cache_dir}/{namespace}/`. Must be alphanumeric, hyphens, or underscores only (max 64 chars). Different namespaces have isolated cache spaces on the same filesystem. |
| `cacheTtlSecs` | `number \| null` | `null` | Per-request cache TTL in seconds. Overrides the global `max_age_days` for this specific extraction. When `0`, caching is completely skipped (no read or write). When `null`, the global TTL applies. |
| `email` | `EmailConfig \| null` | `null` | Email extraction configuration (None = use defaults). Currently supports configuring the fallback codepage for MSG files that do not specify one. See `EmailConfig` for details. |
| `concurrency` | `string \| null` | `null` | Concurrency limits for constrained environments (None = use defaults). Controls Rayon thread pool size, ONNX Runtime intra-op threads, and (when `max_concurrent_extractions` is unset) the batch concurrency semaphore. See `ConcurrencyConfig` for details. |
| `maxArchiveDepth` | `number` | — | Maximum recursion depth for archive extraction (default: 3). Set to 0 to disable recursive extraction (legacy behavior). |
| `treeSitter` | `TreeSitterConfig \| null` | `null` | Tree-sitter language pack configuration (None = tree-sitter disabled). When set, enables code file extraction using tree-sitter parsers. Controls grammar download behavior and code analysis options. |
| `structuredExtraction` | `StructuredExtractionConfig \| null` | `null` | Structured extraction via LLM (None = disabled). When set, the extracted document content is sent to an LLM with the provided JSON schema. The structured response is stored in `ExtractionResult.structured_output`. |
| `ner` | `NerConfig \| null` | `null` | Named-entity recognition configuration. When set, the NER post-processor runs at the Middle stage and populates `ExtractionResult.entities`. |
| `redaction` | `RedactionConfig \| null` | `null` | Redaction / anonymisation configuration. When set, the redaction post-processor runs at the Late stage and rewrites every textual field in `ExtractionResult`, emitting an audit trail in `ExtractionResult.redaction_report`. |
| `summarization` | `SummarizationConfig \| null` | `null` | Summarisation configuration. When set, the summarisation post-processor runs at the Middle stage and populates `ExtractionResult.summary`. |
| `translation` | `TranslationConfig \| null` | `null` | Translation configuration. When set, the translation post-processor runs at the Middle stage and populates `ExtractionResult.translation`. |
| `pageClassification` | `PageClassificationConfig \| null` | `null` | Per-page classification configuration. When set, the classification post-processor runs at the Middle stage and populates `ExtractionResult.page_classifications`. |
| `captioning` | `CaptioningConfig \| null` | `null` | VLM captioning configuration for extracted images. When set, the captioning post-processor runs at the Middle stage and writes a caption into each `ExtractedImage.caption`. |
| `qrCodes` | `boolean \| null` | `null` | Enable QR-code detection in extracted images. When `true`, the QR post-processor runs at the Middle stage and populates `ExtractedImage.qr_codes`. |
| `cancelToken` | `string \| null` | `null` | Cancellation token for this extraction (None = no external cancellation). Pass a `CancellationToken` clone here and call its `cancel()` from another thread / task to abort the extraction in progress. The extractor checks the token at safe checkpoints (before lock acquisition, between pages, between batch items) and returns `Cancelled` when set. The field is excluded from serialization because `CancellationToken` is a runtime handle, not a configuration value. |

### Methods

#### default()

**Signature:**

```typescript
static default(): ExtractionConfig
```

#### needsImageProcessing()

Check if image processing is needed by examining OCR and image extraction settings.

Returns `true` if either OCR is enabled or image extraction is configured,
indicating that image decompression and processing should occur.
Returns `false` if both are disabled, allowing optimization to skip unnecessary
image decompression for text-only extraction workflows.

### Optimization Impact
For text-only extractions (no OCR, no image extraction), skipping image
decompression can improve CPU utilization by 5-10% by avoiding wasteful
image I/O and processing when results won't be used.

**Signature:**

```typescript
needsImageProcessing(): boolean
```

---

#### ExtractionDiff

The complete diff between two `ExtractionResult` values.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `contentDiff` | `Array<DiffHunk>` | — | Unified-diff hunks for the `content` field. Empty when the content is identical. |
| `tablesAdded` | `Array<Table>` | — | Tables present in `b` but not in `a` (by index position, excess right-side tables). |
| `tablesRemoved` | `Array<Table>` | — | Tables present in `a` but not in `b` (by index position, excess left-side tables). |
| `tablesChanged` | `Array<TableDiff>` | — | Cell-level changes for table pairs that share the same index and dimensions. |
| `metadataChanged` | `unknown` | — | Metadata difference, encoded as a JSON object with three top-level keys: `added` (keys present in `b` but not `a`), `removed` (keys present in `a` but not `b`), and `changed` (keys whose values differ — each entry is `{ "from": <value-in-a>, "to": <value-in-b> }`). This is NOT RFC 6902 JSON Patch — we deliberately chose a flatter shape to avoid pulling in a json-patch crate. If you need RFC 6902 semantics (with JSON Pointer paths) feed `a.metadata` and `b.metadata` to your preferred json-patch impl directly. |
| `embeddedChanges` | `EmbeddedChanges` | — | Changes to embedded archive children. |

---

#### ExtractionResult

General extraction result used by the core extraction API.

This is the main result type returned by all extraction functions.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `string` | — | Plain-text representation of the extracted document content. |
| `mimeType` | `string` | — | MIME type of the source document (e.g. `"application/pdf"`). |
| `metadata` | `Metadata` | — | Document-level metadata (author, title, dates, format-specific fields). |
| `extractionMethod` | `ExtractionMethod \| null` | `null` | Extraction strategy used to produce the returned text. Populated when the extractor can reliably distinguish native text extraction, OCR-only extraction, or mixed native/OCR output. |
| `tables` | `Array<Table>` | `[]` | Tables extracted from the document, each with structured cell data. |
| `detectedLanguages` | `Array<string> \| null` | `[]` | ISO 639-1 language codes detected in the document content. |
| `chunks` | `Array<Chunk> \| null` | `[]` | Text chunks when chunking is enabled. When chunking configuration is provided, the content is split into overlapping chunks for efficient processing. Each chunk contains the text, optional embeddings (if enabled), and metadata about its position. |
| `images` | `Array<ExtractedImage> \| null` | `[]` | Extracted images from the document. When image extraction is enabled via `ImageExtractionConfig`, this field contains all images found in the document with their raw data and metadata. Each image may optionally contain a nested `ocr_result` if OCR was performed. |
| `pages` | `Array<PageContent> \| null` | `[]` | Per-page content when page extraction is enabled. When page extraction is configured, the document is split into per-page content with tables and images mapped to their respective pages. |
| `elements` | `Array<Element> \| null` | `[]` | Semantic elements when element-based result format is enabled. When result_format is set to ElementBased, this field contains semantic elements with type classification, unique identifiers, and metadata for Unstructured-compatible element-based processing. |
| `djotContent` | `DjotContent \| null` | `null` | Rich Djot content structure (when extracting Djot documents). When extracting Djot documents with structured extraction enabled, this field contains the full semantic structure including: - Block-level elements with nesting - Inline formatting with attributes - Links, images, footnotes - Math expressions - Complete attribute information The `content` field still contains plain text for backward compatibility. Always `null` for non-Djot documents. |
| `ocrElements` | `Array<OcrElement> \| null` | `[]` | OCR elements with full spatial and confidence metadata. When OCR is performed with element extraction enabled, this field contains the structured representation of detected text including: - Bounding geometry (rectangles or quadrilaterals) - Confidence scores (detection and recognition) - Rotation information - Hierarchical relationships (Tesseract only) This field preserves all metadata that would otherwise be lost when converting to plain text or markdown output formats. Only populated when `OcrElementConfig.include_elements` is true. |
| `document` | `DocumentStructure \| null` | `null` | Structured document tree (when document structure extraction is enabled). When `include_document_structure` is true in `ExtractionConfig`, this field contains the full hierarchical representation of the document including: - Heading-driven section nesting - Table grids with cell-level metadata - Content layer classification (body, header, footer, footnote) - Inline text annotations (formatting, links) - Bounding boxes and page numbers Independent of `result_format` — can be combined with Unified or ElementBased. |
| `extractedKeywords` | `Array<Keyword> \| null` | `[]` | Extracted keywords when keyword extraction is enabled. When keyword extraction (RAKE or YAKE) is configured, this field contains the extracted keywords with scores, algorithm info, and position data. Previously stored in `metadata.additional["keywords"]`. |
| `qualityScore` | `number \| null` | `null` | Document quality score from quality analysis. A value between 0.0 and 1.0 indicating the overall text quality. Previously stored in `metadata.additional["quality_score"]`. |
| `processingWarnings` | `Array<ProcessingWarning>` | `[]` | Non-fatal warnings collected during processing pipeline stages. Captures errors from optional pipeline features (embedding, chunking, language detection, output formatting) that don't prevent extraction but may indicate degraded results. Previously stored as individual keys in `metadata.additional`. |
| `annotations` | `Array<PdfAnnotation> \| null` | `[]` | PDF annotations extracted from the document. When annotation extraction is enabled via `PdfConfig.extract_annotations`, this field contains text notes, highlights, links, stamps, and other annotations found in PDF documents. |
| `children` | `Array<ArchiveEntry> \| null` | `[]` | Nested extraction results from archive contents. When extracting archives, each processable file inside produces its own full extraction result. Set to `null` for non-archive formats. Use `max_archive_depth` in config to control recursion depth. |
| `uris` | `Array<ExtractedUri> \| null` | `[]` | URIs/links discovered during document extraction. Contains hyperlinks, image references, citations, email addresses, and other URI-like references found in the document. Always extracted when present in the source document. |
| `revisions` | `Array<DocumentRevision> \| null` | `[]` | Tracked changes embedded in the source document. Populated by per-format extractors that understand change-tracking metadata (DOCX `w:ins`/`w:del`/`w:rPrChange`, ODT `text:change-*`, …). Every extractor defaults to `null` until its format-specific implementation is added. Extractors that do populate this field follow the "accepted-changes" convention: inserted text is present in `content`, deleted text is absent — the revision list is the separate audit trail. |
| `structuredOutput` | `unknown \| null` | `null` | Structured extraction output from LLM-based JSON schema extraction. When `structured_extraction` is configured in `ExtractionConfig`, the extracted document content is sent to a VLM with the provided JSON schema. The response is parsed and stored here as a JSON value matching the schema. |
| `codeIntelligence` | `unknown \| null` | `null` | Code intelligence results from tree-sitter analysis. Populated when extracting source code files with the `tree-sitter` feature. Contains metrics, structural analysis, imports/exports, comments, docstrings, symbols, diagnostics, and optionally chunked code segments. Stored as an opaque JSON value so that all language bindings (Go, Java, C#, …) can deserialize it as a raw JSON object rather than a typed struct. The underlying type is `tree_sitter_language_pack.ProcessResult`. |
| `llmUsage` | `Array<LlmUsage> \| null` | `[]` | LLM token usage and cost data for all LLM calls made during this extraction. Contains one entry per LLM call. Multiple entries are produced when VLM OCR, structured extraction, or LLM embeddings run during the same extraction. `null` when no LLM was used. |
| `entities` | `Array<Entity> \| null` | `[]` | Named entities detected in `content` by the NER post-processor. `null` when no NER backend is configured. Populated by the gline-rs ONNX backend or the LLM-driven backend (see `crates/kreuzberg/src/text/ner/`). |
| `summary` | `DocumentSummary \| null` | `null` | Summary of `content` produced by the summarisation post-processor. `null` when summarisation is not configured. Populated by the TextRank extractive backend (deterministic, no external service) or by the liter-llm-driven abstractive backend. |
| `translation` | `Translation \| null` | `null` | Translation of `content` produced by the translation post-processor. `null` when translation is not configured. |
| `pageClassifications` | `Array<PageClassification> \| null` | `[]` | Per-page classifications produced by the page-classification post-processor. `null` when classification is not configured. |
| `redactionReport` | `RedactionReport \| null` | `null` | Audit report of redactions applied by the redaction post-processor. The redaction processor rewrites `content`, `formatted_content`, every chunk's text, and the textual fields of `entities` / `summary` / `translation` / `page_classifications` in place. This report describes what was found and how it was replaced. `null` when redaction is not configured. |
| `formattedContent` | `string \| null` | `null` | Pre-rendered content in the requested output format. Populated during `derive_extraction_result` before tree derivation consumes element data. `apply_output_format` swaps this into `content` at the end of the pipeline, after post-processors have operated on plain text. |
| `ocrInternalDocument` | `string \| null` | `null` | Structured hOCR document for the OCR+layout pipeline. When tesseract produces hOCR output, the parsed `InternalDocument` carries paragraph structure with bounding boxes and confidence scores. The layout classification step enriches these elements before final rendering. |

### Methods

#### fromOcr()

Convert from an OCR result.

**Signature:**

```typescript
static fromOcr(ocr: OcrExtractionResult): ExtractionResult
```

---

#### FictionBookMetadata

FictionBook (FB2) metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `genres` | `Array<string>` | `[]` | Genre tags as declared in the FB2 `<genre>` elements. |
| `sequences` | `Array<string>` | `[]` | Book series (sequence) names, if any. |
| `annotation` | `string \| null` | `null` | Short annotation / summary from the FB2 `<annotation>` element. |

---

#### FileExtractionConfig

Per-file extraction configuration overrides for batch processing.

All fields are `Option<T>` — `null` means "use the batch-level default."
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
| `enableQualityProcessing` | `boolean \| null` | `null` | Override quality post-processing for this file. |
| `ocr` | `OcrConfig \| null` | `null` | Override OCR configuration for this file (None in the Option = use batch default). |
| `forceOcr` | `boolean \| null` | `null` | Override force OCR for this file. |
| `forceOcrPages` | `Array<number> \| null` | `[]` | Override force OCR pages for this file (1-indexed page numbers). |
| `disableOcr` | `boolean \| null` | `null` | Override disable OCR for this file. |
| `chunking` | `ChunkingConfig \| null` | `null` | Override chunking configuration for this file. |
| `contentFilter` | `ContentFilterConfig \| null` | `null` | Override content filtering configuration for this file. |
| `images` | `ImageExtractionConfig \| null` | `null` | Override image extraction configuration for this file. |
| `pdfOptions` | `PdfConfig \| null` | `null` | Override PDF options for this file. |
| `tokenReduction` | `TokenReductionOptions \| null` | `null` | Override token reduction for this file. |
| `languageDetection` | `LanguageDetectionConfig \| null` | `null` | Override language detection for this file. |
| `pages` | `PageConfig \| null` | `null` | Override page extraction for this file. |
| `keywords` | `KeywordConfig \| null` | `null` | Override keyword extraction for this file. |
| `postprocessor` | `PostProcessorConfig \| null` | `null` | Override post-processor for this file. |
| `htmlOptions` | `string \| null` | `null` | Override HTML conversion options for this file. |
| `resultFormat` | `ResultFormat \| null` | `null` | Override result format for this file. |
| `outputFormat` | `OutputFormat \| null` | `null` | Override output content format for this file. |
| `includeDocumentStructure` | `boolean \| null` | `null` | Override document structure output for this file. |
| `layout` | `LayoutDetectionConfig \| null` | `null` | Override layout detection for this file. |
| `timeoutSecs` | `number \| null` | `null` | Override per-file extraction timeout in seconds. When set, the extraction for this file will be canceled after the specified duration. A timed-out file produces an error result without affecting other files in the batch. |
| `treeSitter` | `TreeSitterConfig \| null` | `null` | Override tree-sitter configuration for this file. |
| `structuredExtraction` | `StructuredExtractionConfig \| null` | `null` | Override structured extraction configuration for this file. When set, enables LLM-based structured extraction with a JSON schema for this specific file. The extracted content is sent to a VLM/LLM and the response is parsed according to the provided schema. |

---

#### Footnote

Footnote in Djot.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `label` | `string` | — | Footnote label |
| `content` | `Array<FormattedBlock>` | — | Footnote content blocks |

---

#### FormattedBlock

Block-level element in a Djot document.

Represents structural elements like headings, paragraphs, lists, code blocks, etc.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `blockType` | `BlockType` | — | Type of block element |
| `level` | `number \| null` | `null` | Heading level (1-6) for headings, or nesting level for lists |
| `inlineContent` | `Array<InlineElement>` | — | Inline content within the block |
| `attributes` | `string \| null` | `null` | Element attributes (classes, IDs, key-value pairs) |
| `language` | `string \| null` | `null` | Language identifier for code blocks |
| `code` | `string \| null` | `null` | Raw code content for code blocks |
| `children` | `Array<FormattedBlock>` | `/* serde(default) */` | Nested blocks for containers (blockquotes, list items, divs) |

---

#### GridCell

Individual grid cell with position and span metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `string` | — | Cell text content. |
| `row` | `number` | — | Zero-indexed row position. |
| `col` | `number` | — | Zero-indexed column position. |
| `rowSpan` | `number` | `/* serde(default) */` | Number of rows this cell spans. |
| `colSpan` | `number` | `/* serde(default) */` | Number of columns this cell spans. |
| `isHeader` | `boolean` | `/* serde(default) */` | Whether this is a header cell. |
| `bbox` | `BoundingBox \| null` | `null` | Bounding box for this cell (if available). |

---

#### HeaderMetadata

Header/heading element metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `level` | `number` | — | Header level: 1 (h1) through 6 (h6) |
| `text` | `string` | — | Normalized text content of the header |
| `id` | `string \| null` | `null` | HTML id attribute if present |
| `depth` | `number` | — | Document tree depth at the header element |
| `htmlOffset` | `number` | — | Byte offset in original HTML document |

---

#### HeadingContext

Heading context for a chunk within a Markdown document.

Contains the heading hierarchy from document root to this chunk's section.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `headings` | `Array<HeadingLevel>` | — | The heading hierarchy from document root to this chunk's section. Index 0 is the outermost (h1), last element is the most specific. |

---

#### HeadingLevel

A single heading in the hierarchy.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `level` | `number` | — | Heading depth (1 = h1, 2 = h2, etc.) |
| `text` | `string` | — | The text content of the heading. |

---

#### HierarchicalBlock

A text block with hierarchy level assignment.

Represents a block of text with semantic heading information extracted from
font size clustering and hierarchical analysis.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text` | `string` | — | The text content of this block |
| `fontSize` | `number` | — | The font size of the text in this block |
| `level` | `string` | — | The hierarchy level of this block (H1-H6 or Body) Levels correspond to HTML heading tags: - "h1": Top-level heading - "h2": Secondary heading - "h3": Tertiary heading - "h4": Quaternary heading - "h5": Quinary heading - "h6": Senary heading - "body": Body text (no heading level) |
| `bbox` | `Array<number> \| null` | `null` | Bounding box information for the block Contains coordinates as (left, top, right, bottom) in PDF units. |

---

#### HierarchyConfig

Hierarchy extraction configuration for PDF text structure analysis.

Enables extraction of document hierarchy levels (H1-H6) based on font size
clustering and semantic analysis. When enabled, hierarchical blocks are
included in page content.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `boolean` | `true` | Enable hierarchy extraction |
| `kClusters` | `number` | `3` | Number of font size clusters to use for hierarchy levels (1-7) Default: 6, which provides H1-H6 heading levels with body text. Larger values create more fine-grained hierarchy levels. |
| `includeBbox` | `boolean` | `true` | Include bounding box information in hierarchy blocks |
| `ocrCoverageThreshold` | `number \| null` | `null` | OCR coverage threshold for smart OCR triggering (0.0-1.0) Determines when OCR should be triggered based on text block coverage. OCR is triggered when text blocks cover less than this fraction of the page. Default: 0.5 (trigger OCR if less than 50% of page has text) |

### Methods

#### default()

**Signature:**

```typescript
static default(): HierarchyConfig
```

---

#### HtmlMetadata

HTML metadata extracted from HTML documents.

Includes document-level metadata, Open Graph data, Twitter Card metadata,
and extracted structural elements (headers, links, images, structured data).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `title` | `string \| null` | `null` | Document title from `<title>` tag |
| `description` | `string \| null` | `null` | Document description from `<meta name="description">` tag |
| `keywords` | `Array<string>` | `[]` | Document keywords from `<meta name="keywords">` tag, split on commas |
| `author` | `string \| null` | `null` | Document author from `<meta name="author">` tag |
| `canonicalUrl` | `string \| null` | `null` | Canonical URL from `<link rel="canonical">` tag |
| `baseHref` | `string \| null` | `null` | Base URL from `<base href="">` tag for resolving relative URLs |
| `language` | `string \| null` | `null` | Document language from `lang` attribute |
| `textDirection` | `TextDirection \| null` | `null` | Document text direction from `dir` attribute |
| `openGraph` | `Record<string, string>` | `{}` | Open Graph metadata (og:* properties) for social media Keys like "title", "description", "image", "url", etc. |
| `twitterCard` | `Record<string, string>` | `{}` | Twitter Card metadata (twitter:* properties) Keys like "card", "site", "creator", "title", "description", "image", etc. |
| `metaTags` | `Record<string, string>` | `{}` | Additional meta tags not covered by specific fields Keys are meta name/property attributes, values are content |
| `headers` | `Array<HeaderMetadata>` | `[]` | Extracted header elements with hierarchy |
| `links` | `Array<LinkMetadata>` | `[]` | Extracted hyperlinks with type classification |
| `images` | `Array<ImageMetadataType>` | `[]` | Extracted images with source and dimensions |
| `structuredData` | `Array<StructuredData>` | `[]` | Extracted structured data blocks |

---

#### HtmlOutputConfig

Configuration for styled HTML output.

When set on `html_output` alongside
`output_format = OutputFormat.Html`, the pipeline builds a
`StyledHtmlRenderer` instead of
the plain comrak-based renderer.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `css` | `string \| null` | `null` | Inline CSS string injected into the output after the theme stylesheet. Concatenated after `css_file` content when both are set. |
| `cssFile` | `string \| null` | `null` | Path to a CSS file loaded once at renderer construction time. Concatenated before `css` when both are set. |
| `theme` | `HtmlTheme` | `HtmlTheme.Unstyled` | Built-in colour/typography theme. Default: `HtmlTheme.Unstyled`. |
| `classPrefix` | `string` | — | CSS class prefix applied to every emitted class name. Default: `"kb-"`. Change this if your host application already uses classes that start with `kb-`. |
| `embedCss` | `boolean` | `true` | When `true` (default), write the resolved CSS into a `<style>` block immediately after the opening `<div class="{prefix}doc">`. Set to `false` to emit only the structural markup and wire up your own stylesheet targeting the `kb-*` class names. |

### Methods

#### default()

**Signature:**

```typescript
static default(): HtmlOutputConfig
```

---

#### ImageExtractionConfig

Image extraction configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `extractImages` | `boolean` | `true` | Extract images from documents |
| `targetDpi` | `number` | `300` | Target DPI for image normalization |
| `maxImageDimension` | `number` | `4096` | Maximum dimension for images (width or height) |
| `injectPlaceholders` | `boolean` | `true` | Whether to inject image reference placeholders into markdown output. When `true` (default), image references like `![Image 1](embedded:p1_i0)` are appended to the markdown. Set to `false` to extract images as data without polluting the markdown output. |
| `autoAdjustDpi` | `boolean` | `true` | Automatically adjust DPI based on image content |
| `minDpi` | `number` | `72` | Minimum DPI threshold |
| `maxDpi` | `number` | `600` | Maximum DPI threshold |
| `maxImagesPerPage` | `number \| null` | `null` | Maximum number of image objects to extract per PDF page. Some PDFs (e.g. technical diagrams stored as thousands of raster fragments) can trigger extremely long or indefinite extraction times when every image object on a dense page is decoded individually via the PDF extractor. Setting this limit causes kreuzberg to stop collecting individual images once the count per page reaches the cap and emit a warning instead. `null` (default) means no limit — all images are extracted. |
| `classify` | `boolean` | `true` | When `true` (default), extracted images are classified by kind and grouped into clusters where they appear to belong to one figure. |
| `includePageRasters` | `boolean` | `false` | When `true`, full-page renders produced during OCR preprocessing are captured and returned as `ImageKind.PageRaster` entries in `ExtractionResult.images`. **PDF + OCR only.** No rasters are captured for non-PDF inputs or when the document-level OCR bypass is active (whole-document backend). When OCR is enabled and this flag is set but the active backend skips per-page rendering, a `ProcessingWarning` is emitted in `ExtractionResult.processing_warnings`. Defaults to `false`. Enable when downstream consumers need page thumbnails (e.g. citation previews, visual grounding). |
| `runOcrOnImages` | `boolean` | `true` | Run OCR on extracted images and include the recognized text in the document content. When `true` (default) and `ExtractionConfig.ocr` is configured, extracted images are processed with the configured OCR backend. Set to `false` to extract images without OCR processing, even when OCR is enabled. |
| `ocrTextOnly` | `boolean` | `false` | When `true`, image OCR results are rendered as plain text without the `![...](...)` markdown placeholder. Only takes effect when `run_ocr_on_images` is also `true`. |
| `appendOcrText` | `boolean` | `false` | When `true` and `ocr_text_only` is `false`, append the OCR text after the image placeholder in the rendered output. |

### Methods

#### default()

**Signature:**

```typescript
static default(): ImageExtractionConfig
```

---

#### ImageMetadata

Image metadata extracted from image files.

Includes dimensions, format, and EXIF data.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `width` | `number` | — | Image width in pixels |
| `height` | `number` | — | Image height in pixels |
| `format` | `string` | — | Image format (e.g., "PNG", "JPEG", "TIFF") |
| `exif` | `Record<string, string>` | `{}` | EXIF metadata tags |

---

#### ImageMetadataType

Image element metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `src` | `string` | — | Image source (URL, data URI, or SVG content) |
| `alt` | `string \| null` | `null` | Alternative text from alt attribute |
| `title` | `string \| null` | `null` | Title attribute |
| `dimensions` | `Array<number> \| null` | `null` | Image dimensions as (width, height) if available |
| `imageType` | `ImageType` | — | Image type classification |
| `attributes` | `Array<Array<string>>` | — | Additional attributes as key-value pairs |

---

#### ImagePreprocessingConfig

Image preprocessing configuration for OCR.

These settings control how images are preprocessed before OCR to improve
text recognition quality. Different preprocessing strategies work better
for different document types.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `targetDpi` | `number` | `300` | Target DPI for the image (300 is standard, 600 for small text). |
| `autoRotate` | `boolean` | `true` | Auto-detect and correct image rotation. |
| `deskew` | `boolean` | `true` | Correct skew (tilted images). |
| `denoise` | `boolean` | `false` | Remove noise from the image. |
| `contrastEnhance` | `boolean` | `false` | Enhance contrast for better text visibility. |
| `binarizationMethod` | `string` | `"otsu"` | Binarization method: "otsu", "sauvola", "adaptive". |
| `invertColors` | `boolean` | `false` | Invert colors (white text on black → black on white). |

### Methods

#### default()

**Signature:**

```typescript
static default(): ImagePreprocessingConfig
```

---

#### ImagePreprocessingMetadata

Image preprocessing metadata.

Tracks the transformations applied to an image during OCR preprocessing,
including DPI normalization, resizing, and resampling.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `originalDimensions` | `Array<number>` | — | Original image dimensions (width, height) in pixels |
| `originalDpi` | `Array<number>` | — | Original image DPI (horizontal, vertical) |
| `targetDpi` | `number` | — | Target DPI from configuration |
| `scaleFactor` | `number` | — | Scaling factor applied to the image |
| `autoAdjusted` | `boolean` | — | Whether DPI was auto-adjusted based on content |
| `finalDpi` | `number` | — | Final DPI after processing |
| `newDimensions` | `Array<number> \| null` | `null` | New dimensions after resizing (if resized) |
| `resampleMethod` | `string` | — | Resampling algorithm used ("LANCZOS3", "CATMULLROM", etc.) |
| `dimensionClamped` | `boolean` | — | Whether dimensions were clamped to max_image_dimension |
| `calculatedDpi` | `number \| null` | `null` | Calculated optimal DPI (if auto_adjust_dpi enabled) |
| `skippedResize` | `boolean` | — | Whether resize was skipped (dimensions already optimal) |
| `resizeError` | `string \| null` | `null` | Error message if resize failed |

---

#### InlineElement

Inline element within a block.

Represents text with formatting, links, images, etc.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `elementType` | `InlineType` | — | Type of inline element |
| `content` | `string` | — | Text content |
| `attributes` | `string \| null` | `null` | Element attributes |
| `metadata` | `Record<string, string> \| null` | `null` | Additional metadata (e.g., href for links, src/alt for images) |

---

#### JatsMetadata

JATS (Journal Article Tag Suite) metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `copyright` | `string \| null` | `null` | Copyright statement from the article's `<permissions>` element. |
| `license` | `string \| null` | `null` | Open-access license URI from the article's `<license>` element. |
| `historyDates` | `Record<string, string>` | `{}` | Publication history dates keyed by event type (e.g. `"received"`, `"accepted"`). |
| `contributorRoles` | `Array<ContributorRole>` | `[]` | Authors and contributors with their stated roles. |

---

#### Keyword

Extracted keyword with metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text` | `string` | — | The keyword text. |
| `score` | `number` | — | Relevance score (higher is better, algorithm-specific range). |
| `algorithm` | `KeywordAlgorithm` | — | Algorithm that extracted this keyword. |
| `positions` | `Array<number> \| null` | `null` | Optional positions where keyword appears in text (character offsets). |

---

#### KeywordConfig

Keyword extraction configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `algorithm` | `KeywordAlgorithm` | `KeywordAlgorithm.Yake` | Algorithm to use for extraction. |
| `maxKeywords` | `number` | `10` | Maximum number of keywords to extract (default: 10). |
| `minScore` | `number` | `0` | Minimum score threshold (0.0-1.0, default: 0.0). Keywords with scores below this threshold are filtered out. Note: Score ranges differ between algorithms. |
| `ngramRange` | `Array<number>` | `[]` | N-gram range for keyword extraction (min, max). (1, 1) = unigrams only (1, 2) = unigrams and bigrams (1, 3) = unigrams, bigrams, and trigrams (default) |
| `language` | `string \| null` | `null` | Language code for stopword filtering (e.g., "en", "de", "fr"). If None, no stopword filtering is applied. |
| `yakeParams` | `YakeParams \| null` | `null` | YAKE-specific tuning parameters. |
| `rakeParams` | `RakeParams \| null` | `null` | RAKE-specific tuning parameters. |

### Methods

#### default()

**Signature:**

```typescript
static default(): KeywordConfig
```

---

#### LanguageDetectionConfig

Language detection configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `boolean` | `true` | Enable language detection |
| `minConfidence` | `number` | `0.8` | Minimum confidence threshold (0.0-1.0) |
| `detectMultiple` | `boolean` | `false` | Detect multiple languages in the document |

### Methods

#### default()

**Signature:**

```typescript
static default(): LanguageDetectionConfig
```

---

#### LayoutDetection

A single layout detection result.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `className` | `LayoutClass` | — | Detected layout class (e.g. `Table`, `Text`, `Title`). |
| `confidence` | `number` | — | Detection confidence score in `[0.0, 1.0]`. |
| `bbox` | `BBox` | — | Bounding box in image pixel coordinates. |

---

#### LayoutDetectionConfig

Layout detection configuration.

Controls layout detection behavior in the extraction pipeline.
When set on `ExtractionConfig`, layout detection
is enabled for PDF extraction.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `confidenceThreshold` | `number \| null` | `null` | Confidence threshold override (None = use model default). |
| `applyHeuristics` | `boolean` | `true` | Whether to apply postprocessing heuristics (default: true). |
| `tableModel` | `TableModel` | `TableModel.Tatr` | Table structure recognition model. Controls which model is used for table cell detection within layout-detected table regions. Defaults to `TableModel.Tatr`. |
| `acceleration` | `AccelerationConfig \| null` | `null` | Hardware acceleration for ONNX models (layout detection + table structure). When set, controls which execution provider (CPU, CUDA, CoreML, TensorRT) is used for inference. Defaults to `null` (auto-select per platform). |

### Methods

#### default()

**Signature:**

```typescript
static default(): LayoutDetectionConfig
```

---

#### LayoutRegion

A detected layout region on a page.

When layout detection is enabled, each page may have layout regions
identifying different content types (text, pictures, tables, etc.)
with confidence scores and spatial positions.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `className` | `string` | — | Layout class name (e.g. "picture", "table", "text", "section_header"). |
| `confidence` | `number` | — | Confidence score from the layout detection model (0.0 to 1.0). |
| `boundingBox` | `BoundingBox` | — | Bounding box in document coordinate space. |
| `areaFraction` | `number` | — | Fraction of the page area covered by this region (0.0 to 1.0). |

---

#### LinkMetadata

Link element metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `href` | `string` | — | The href URL value |
| `text` | `string` | — | Link text content (normalized) |
| `title` | `string \| null` | `null` | Optional title attribute |
| `linkType` | `LinkType` | — | Link type classification |
| `rel` | `Array<string>` | — | Rel attribute values |
| `attributes` | `Array<Array<string>>` | — | Additional attributes as key-value pairs |

---

#### LlmBackend

liter-llm-backed NER backend.

### Methods

#### new()

Create a new LLM-backed NER backend with the given LLM configuration.

**Signature:**

```typescript
static new(config: LlmConfig): LlmBackend
```

#### detect()

**Signature:**

```typescript
detect(text: string, categories: Array<EntityCategory>): Array<Entity>
```

#### detectWithCustom()

**Signature:**

```typescript
detectWithCustom(text: string, categories: Array<EntityCategory>, customLabels: Array<string>): Array<Entity>
```

---

#### LlmConfig

Configuration for an LLM provider/model via liter-llm.

Each feature (VLM OCR, VLM embeddings, structured extraction) carries
its own `LlmConfig`, allowing different providers per feature.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `string` | — | Provider/model string using liter-llm routing format. Examples: `"openai/gpt-4o"`, `"anthropic/claude-sonnet-4-20250514"`, `"groq/llama-3.1-70b-versatile"`. |
| `apiKey` | `string \| null` | `null` | API key for the provider. When `null`, liter-llm falls back to the provider's standard environment variable (e.g., `OPENAI_API_KEY`). |
| `baseUrl` | `string \| null` | `null` | Custom base URL override for the provider endpoint. |
| `timeoutSecs` | `number \| null` | `null` | Request timeout in seconds (default: 60). |
| `maxRetries` | `number \| null` | `null` | Maximum retry attempts (default: 3). |
| `temperature` | `number \| null` | `null` | Sampling temperature for generation tasks. |
| `maxTokens` | `number \| null` | `null` | Maximum tokens to generate. |

---

#### LlmUsage

Token usage and cost data for a single LLM call made during extraction.

Populated when VLM OCR, structured extraction, or LLM-based embeddings
are used. Multiple entries may be present when multiple LLM calls occur
within one extraction (e.g. VLM OCR + structured extraction).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `string` | — | The LLM model identifier (e.g. "openai/gpt-4o", "anthropic/claude-sonnet-4-20250514"). |
| `source` | `string` | — | The pipeline stage that triggered this LLM call (e.g. "vlm_ocr", "structured_extraction", "embeddings"). |
| `inputTokens` | `number \| null` | `null` | Number of input/prompt tokens consumed. |
| `outputTokens` | `number \| null` | `null` | Number of output/completion tokens generated. |
| `totalTokens` | `number \| null` | `null` | Total tokens (input + output). |
| `estimatedCost` | `number \| null` | `null` | Estimated cost in USD based on the provider's published pricing. |
| `finishReason` | `string \| null` | `null` | Why the model stopped generating (e.g. "stop", "length", "content_filter"). |

---

#### Metadata

Extraction result metadata.

Contains common fields applicable to all formats, format-specific metadata
via a discriminated union, and additional custom fields from postprocessors.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `title` | `string \| null` | `null` | Document title |
| `subject` | `string \| null` | `null` | Document subject or description |
| `authors` | `Array<string> \| null` | `[]` | Primary author(s) - always Vec for consistency |
| `keywords` | `Array<string> \| null` | `[]` | Keywords/tags - always Vec for consistency |
| `language` | `string \| null` | `null` | Primary language (ISO 639 code) |
| `createdAt` | `string \| null` | `null` | Creation timestamp (ISO 8601 format) |
| `modifiedAt` | `string \| null` | `null` | Last modification timestamp (ISO 8601 format) |
| `createdBy` | `string \| null` | `null` | User who created the document |
| `modifiedBy` | `string \| null` | `null` | User who last modified the document |
| `pages` | `PageStructure \| null` | `null` | Page/slide/sheet structure with boundaries |
| `format` | `FormatMetadata \| null` | `null` | Format-specific metadata (discriminated union) Contains detailed metadata specific to the document format. Serialized as a nested `"format"` object with a `format_type` discriminator field. |
| `imagePreprocessing` | `ImagePreprocessingMetadata \| null` | `null` | Image preprocessing metadata (when OCR preprocessing was applied) |
| `jsonSchema` | `unknown \| null` | `null` | JSON schema (for structured data extraction) |
| `error` | `ErrorMetadata \| null` | `null` | Error metadata (for batch operations) |
| `extractionDurationMs` | `number \| null` | `null` | Extraction duration in milliseconds (for benchmarking). This field is populated by batch extraction to provide per-file timing information. It's `null` for single-file extraction (which uses external timing). |
| `category` | `string \| null` | `null` | Document category (from frontmatter or classification). |
| `tags` | `Array<string> \| null` | `[]` | Document tags (from frontmatter). |
| `documentVersion` | `string \| null` | `null` | Document version string (from frontmatter). |
| `abstractText` | `string \| null` | `null` | Abstract or summary text (from frontmatter). |
| `outputFormat` | `string \| null` | `null` | Output format identifier (e.g., "markdown", "html", "text"). Set by the output format pipeline stage when format conversion is applied. Previously stored in `metadata.additional["output_format"]`. |
| `ocrUsed` | `boolean` | — | Whether OCR was used during extraction. Set to `true` whenever the extraction pipeline ran an OCR backend (Tesseract, PaddleOCR, VLM, etc.) and used that output as the primary or fallback text. `false` means native text extraction was used exclusively. |
| `additional` | `Record<string, unknown>` | `{}` | Additional custom fields from postprocessors. Serialized as a nested `"additional"` object (not flattened at root level). Uses `Cow<'static, str>` keys so static string keys avoid allocation. |

### Methods

#### isEmpty()

Returns `true` when no metadata fields, format-specific metadata, or
additional postprocessor fields are populated.

**Signature:**

```typescript
isEmpty(): boolean
```

---

#### ModelPaths

Combined paths to all models needed for OCR (backward compatibility).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `detModel` | `string` | — | Path to the detection model directory. |
| `clsModel` | `string` | — | Path to the classification model directory. |
| `recModel` | `string` | — | Path to the recognition model directory. |
| `dictFile` | `string` | — | Path to the character dictionary file. |

---

#### NerBackend

NER backend trait (stub for Android x86_64).

---

#### NerConfig

**Since:** `v5.0.0`

Configuration for the NER post-processor.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `backend` | `NerBackendKind` | `NerBackendKind.Onnx` | Backend that runs the entity detection. |
| `categories` | `Array<EntityCategory>` | `[]` | Entity categories to detect. Defaults to a sensible PERSON/ORG/LOCATION/EMAIL set when empty. |
| `model` | `string \| null` | `null` | Override the default model — only used by `NerBackendKind.Onnx`. `null` lets the backend pick its pinned default (`urchade/gliner_multi-v2.1` for gline-rs). |
| `llm` | `LlmConfig \| null` | `null` | Optional LLM configuration — only used by `NerBackendKind.Llm`. Token usage for LLM backends is recorded in `ExtractionResult.llm_usage`. |
| `customLabels` | `Array<string>` | `[]` | Arbitrary user-supplied entity labels for zero-shot detection. gline-rs natively supports zero-shot inference over caller-supplied labels — this is the primary value of GLiNER. The LLM backend also honours these labels by including them in the structured-output schema. Custom labels surface as `EntityCategory.Custom` in the resulting `Entity` stream. Use this when you need domain-specific entity types (e.g. `"Treatment"`, `"Product"`, `"Vessel"`) without forking GLiNER's taxonomy. |

---

#### OcrBackend

Trait for OCR backend plugins.

Implement this trait to add custom OCR capabilities. OCR backends can be:

- Native Rust implementations (like Tesseract)
- FFI bridges to Python libraries (like EasyOCR, PaddleOCR)
- Cloud-based OCR services (Google Vision, AWS Textract, etc.)

### Thread Safety

OCR backends must be thread-safe (`Send + Sync`) to support concurrent processing.

### Methods

#### processImage()

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

```typescript
processImage(imageBytes: Buffer, config: OcrConfig): ExtractionResult
```

#### processImageFile()

Process a file and extract text via OCR.

Default implementation reads the file and calls `process_image`.
Override for custom file handling or optimizations.

**Errors:**

Same as `process_image`, plus file I/O errors.

**Signature:**

```typescript
processImageFile(path: string, config: OcrConfig): ExtractionResult
```

#### supportsLanguage()

Check if this backend supports a given language code.

**Returns:**

`true` if the language is supported, `false` otherwise.

**Signature:**

```typescript
supportsLanguage(lang: string): boolean
```

#### backendType()

Get the backend type identifier.

**Returns:**

The backend type enum value.

**Signature:**

```typescript
backendType(): OcrBackendType
```

#### supportedLanguages()

Optional: Get a list of all supported languages.

Defaults to empty list. Override to provide comprehensive language support info.

**Signature:**

```typescript
supportedLanguages(): Array<string>
```

#### supportsTableDetection()

Optional: Check if the backend supports table detection.

Defaults to `false`. Override if your backend can detect and extract tables.

**Signature:**

```typescript
supportsTableDetection(): boolean
```

#### supportsDocumentProcessing()

Check if the backend supports direct document-level processing (e.g. for PDFs).

Defaults to `false`. Override if the backend has optimized document processing.

**Signature:**

```typescript
supportsDocumentProcessing(): boolean
```

#### processDocument()

Process a document file directly via OCR.

Only called if `supports_document_processing` returns `true`.

**Signature:**

```typescript
processDocument(path: string, config: OcrConfig): ExtractionResult
```

---

#### OcrConfidence

Confidence scores for an OCR element.

Separates detection confidence (how confident that text exists at this location)
from recognition confidence (how confident about the actual text content).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `detection` | `number \| null` | `null` | Detection confidence: how confident the OCR engine is that text exists here. PaddleOCR provides this as `box_score`, Tesseract doesn't have a direct equivalent. Range: 0.0 to 1.0 (or None if not available). |
| `recognition` | `number` | — | Recognition confidence: how confident about the text content. Range: 0.0 to 1.0. |

---

#### OcrConfig

OCR configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `boolean` | `true` | Whether OCR is enabled. Setting `enabled: false` is a shorthand for `disable_ocr: true` on the parent `ExtractionConfig`. Images return metadata only; PDFs use native text extraction without OCR fallback. Defaults to `true`. When `false`, all other OCR settings are ignored. |
| `backend` | `string` | — | OCR backend: tesseract, easyocr, paddleocr |
| `language` | `string` | — | Language code (e.g., "eng", "deu") |
| `tesseractConfig` | `TesseractConfig \| null` | `null` | Tesseract-specific configuration (optional) |
| `outputFormat` | `OutputFormat \| null` | `null` | Output format for OCR results (optional, for format conversion) |
| `paddleOcrConfig` | `unknown \| null` | `null` | PaddleOCR-specific configuration (optional, JSON passthrough) |
| `backendOptions` | `unknown \| null` | `null` | Arbitrary per-call options passed through to the backend unchanged. Custom OCR backends and built-in backends that support runtime tuning can read this value and deserialize the keys they care about. Keys unknown to the backend are silently ignored. This is the recommended extension point for per-call parameters that are not covered by the typed fields above (e.g. mode switching, preprocessing flags, inference batch size). **Scope:** when `pipeline` is `null`, this value is propagated to the primary stage of the auto-constructed pipeline. When `pipeline` is explicitly set, this field has **no effect** — the caller must set `OcrPipelineStage.backend_options` directly on the relevant stage(s) instead. Example: ```json { "mode": "fast", "enable_layout": true, "timeout_ms": 5000 } ``` |
| `elementConfig` | `OcrElementConfig \| null` | `null` | OCR element extraction configuration |
| `qualityThresholds` | `OcrQualityThresholds \| null` | `null` | Quality thresholds for the native-text-to-OCR fallback decision. When None, uses compiled defaults (matching previous hardcoded behavior). |
| `pipeline` | `OcrPipelineConfig \| null` | `null` | Multi-backend OCR pipeline configuration. When set, enables weighted fallback across multiple OCR backends based on output quality. When None, uses the single `backend` field (same as today). |
| `autoRotate` | `boolean` | `false` | Enable automatic page rotation based on orientation detection. When enabled, uses Tesseract's `DetectOrientationScript()` to detect page orientation (0/90/180/270 degrees) before OCR. If the page is rotated with high confidence, the image is corrected before recognition. This is critical for handling rotated scanned documents. |
| `vlmFallback` | `VlmFallbackPolicy` | `VlmFallbackPolicy.Disabled` | Ergonomic VLM fallback policy. When set to anything other than `VlmFallbackPolicy.Disabled` and `OcrConfig.pipeline` is `null`, a multi-stage pipeline is synthesised automatically: - `VlmFallbackPolicy.OnLowQuality` → `[classical_stage, vlm_stage]` with the `quality_threshold` mapped onto `OcrQualityThresholds.pipeline_min_quality`. - `VlmFallbackPolicy.Always` → `[vlm_stage]` only. Requires `OcrConfig.vlm_config` to be `Some` when not `Disabled`. When `OcrConfig.pipeline` is explicitly set, this field is ignored. |
| `vlmConfig` | `LlmConfig \| null` | `null` | VLM (Vision Language Model) OCR configuration. Required when `backend` is `"vlm"` or when `vlm_fallback` is not `VlmFallbackPolicy.Disabled`. Uses liter-llm to send page images to a vision model for text extraction. |
| `vlmPrompt` | `string \| null` | `null` | Custom Jinja2 prompt template for VLM OCR. When `null`, uses the default template. Available variables: - `{{ language }}` — The document language code (e.g., "eng", "deu"). |
| `acceleration` | `AccelerationConfig \| null` | `null` | Hardware acceleration for ONNX Runtime models (e.g. PaddleOCR, layout detection). Not user-configurable via config files — injected at runtime from `ExtractionConfig.acceleration` before each `process_image` call. |
| `tessdataBytes` | `Record<string, Buffer> \| null` | `null` | Caller-supplied Tesseract `traineddata` bytes per language code. Primary use case is the WASM build, which has no filesystem and cannot download tessdata at runtime. Native builds typically rely on `TessdataManager` and ignore this field. When present, the WASM Tesseract backend prefers these bytes over its compile-time-bundled English data. Skipped by serde to keep config files small — supply via the typed API at runtime. |

### Methods

#### default()

**Signature:**

```typescript
static default(): OcrConfig
```

---

#### OcrElement

A unified OCR element representing detected text with full metadata.

This is the primary type for structured OCR output, preserving all information
from both Tesseract and PaddleOCR backends.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text` | `string` | — | The recognized text content. |
| `geometry` | `OcrBoundingGeometry` | `OcrBoundingGeometry.Rectangle` | Bounding geometry (rectangle or quadrilateral). |
| `confidence` | `OcrConfidence` | — | Confidence scores for detection and recognition. |
| `level` | `OcrElementLevel` | `OcrElementLevel.Line` | Hierarchical level (word, line, block, page). |
| `rotation` | `OcrRotation \| null` | `null` | Rotation information (if detected). |
| `pageNumber` | `number` | — | Page number (1-indexed). |
| `parentId` | `string \| null` | `null` | Parent element ID for hierarchical relationships. Only used for Tesseract output which has word -> line -> block hierarchy. |
| `backendMetadata` | `Record<string, unknown>` | `{}` | Backend-specific metadata that doesn't fit the unified schema. |

---

#### OcrElementConfig

Configuration for OCR element extraction.

Controls how OCR elements are extracted and filtered.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `includeElements` | `boolean` | — | Whether to include OCR elements in the extraction result. When true, the `ocr_elements` field in `ExtractionResult` will be populated. |
| `minLevel` | `OcrElementLevel` | `OcrElementLevel.Line` | Minimum hierarchical level to include. Elements below this level (e.g., words when min_level is Line) will be excluded. |
| `minConfidence` | `number` | — | Minimum recognition confidence threshold (0.0-1.0). Elements with confidence below this threshold will be filtered out. |
| `buildHierarchy` | `boolean` | — | Whether to build hierarchical relationships between elements. When true, `parent_id` fields will be populated based on spatial containment. Only meaningful for Tesseract output. |

---

#### OcrExtractionResult

OCR extraction result.

Result of performing OCR on an image or scanned document,
including recognized text and detected tables.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `string` | — | Recognized text content |
| `mimeType` | `string` | — | Original MIME type of the processed image |
| `metadata` | `Record<string, unknown>` | — | OCR processing metadata (confidence scores, language, etc.) |
| `tables` | `Array<OcrTable>` | — | Tables detected and extracted via OCR |
| `ocrElements` | `Array<OcrElement> \| null` | `/* serde(default) */` | Structured OCR elements with bounding boxes and confidence scores. Available when TSV output is requested or table detection is enabled. |
| `internalDocument` | `string \| null` | `null` | Structured document produced from hOCR parsing. Carries paragraph structure, bounding boxes, and confidence scores that the flattened `content` string discards. |

---

#### OcrMetadata

OCR processing metadata.

Captures information about OCR processing configuration and results.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `language` | `string` | — | OCR language code(s) used |
| `psm` | `number` | — | Tesseract Page Segmentation Mode (PSM) |
| `outputFormat` | `string` | — | Output format (e.g., "text", "hocr") |
| `tableCount` | `number` | — | Number of tables detected |
| `tableRows` | `number \| null` | `null` | Number of rows in the detected table (if a single table was found). |
| `tableCols` | `number \| null` | `null` | Number of columns in the detected table (if a single table was found). |

---

#### OcrPipelineConfig

Multi-backend OCR pipeline with quality-based fallback.

Backends are tried in priority order (highest first). After each backend
produces output, quality is evaluated. If it meets `quality_thresholds.pipeline_min_quality`,
the result is accepted. Otherwise the next backend is tried.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `stages` | `Array<OcrPipelineStage>` | — | Ordered list of backends to try. Sorted by priority (descending) at runtime. |
| `qualityThresholds` | `OcrQualityThresholds` | `/* serde(default) */` | Quality thresholds for deciding whether to accept a result or try the next backend. |

---

#### OcrPipelineStage

A single backend stage in the OCR pipeline.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `backend` | `string` | — | Backend name: "tesseract", "paddleocr", "easyocr", or a custom registered name. |
| `priority` | `number` | `/* serde(default) */` | Priority weight (higher = tried first). Stages are sorted by priority descending. |
| `language` | `string \| null` | `/* serde(default) */` | Language override for this stage (None = use parent OcrConfig.language). |
| `tesseractConfig` | `TesseractConfig \| null` | `/* serde(default) */` | Tesseract-specific config override for this stage. |
| `paddleOcrConfig` | `unknown \| null` | `/* serde(default) */` | PaddleOCR-specific config for this stage. |
| `vlmConfig` | `LlmConfig \| null` | `/* serde(default) */` | VLM config override for this pipeline stage. |
| `backendOptions` | `unknown \| null` | `/* serde(default) */` | Arbitrary per-call options passed through to the backend unchanged. Backends that support runtime tuning (mode switching, preprocessing flags, inference parameters, etc.) read this value and deserialize the keys they care about. Keys unknown to the backend are silently ignored, so options from different backends can coexist in the same config without conflict. Example (custom backend): ```json { "mode": "fast", "enable_layout": true } ``` |

---

#### OcrQualityThresholds

Quality thresholds for OCR fallback decisions and pipeline quality gating.

All fields default to the values that match the previous hardcoded behavior,
so `OcrQualityThresholds.default()` preserves existing semantics exactly.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `minTotalNonWhitespace` | `number` | `64` | Minimum total non-whitespace characters to consider text substantive. |
| `minNonWhitespacePerPage` | `number` | `32` | Minimum non-whitespace characters per page on average. |
| `minMeaningfulWordLen` | `number` | `4` | Minimum character count for a word to be "meaningful". |
| `minMeaningfulWords` | `number` | `3` | Minimum count of meaningful words before text is accepted. |
| `minAlnumRatio` | `number` | `0.3` | Minimum alphanumeric ratio (non-whitespace chars that are alphanumeric). |
| `minGarbageChars` | `number` | `5` | Minimum Unicode replacement characters (U+FFFD) to trigger OCR fallback. |
| `maxFragmentedWordRatio` | `number` | `0.6` | Maximum fraction of short (1-2 char) words before text is considered fragmented. |
| `criticalFragmentedWordRatio` | `number` | `0.8` | Critical fragmentation threshold — triggers OCR regardless of meaningful words. Normal English text has ~20-30% short words. 80%+ is definitive garbage. |
| `minAvgWordLength` | `number` | `2` | Minimum average word length. Below this with enough words indicates garbled extraction. |
| `minWordsForAvgLengthCheck` | `number` | `50` | Minimum word count before average word length check applies. |
| `minConsecutiveRepeatRatio` | `number` | `0.08` | Minimum consecutive word repetition ratio to detect column scrambling. |
| `minWordsForRepeatCheck` | `number` | `50` | Minimum word count before consecutive repetition check is applied. |
| `substantiveMinChars` | `number` | `100` | Minimum character count for "substantive markdown" OCR skip gate. |
| `nonTextMinChars` | `number` | `20` | Minimum character count for "non-text content" OCR skip gate. |
| `alnumWsRatioThreshold` | `number` | `0.4` | Alphanumeric+whitespace ratio threshold for skip decisions. |
| `pipelineMinQuality` | `number` | `0.5` | Minimum quality score (0.0-1.0) for a pipeline stage result to be accepted. If the result from a backend scores below this, try the next backend. |

### Methods

#### default()

**Signature:**

```typescript
static default(): OcrQualityThresholds
```

---

#### OcrRotation

Rotation information for an OCR element.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `angleDegrees` | `number` | — | Rotation angle in degrees (0, 90, 180, 270 for PaddleOCR). |
| `confidence` | `number \| null` | `null` | Confidence score for the rotation detection. |

---

#### OcrTable

Table detected via OCR.

Represents a table structure recognized during OCR processing.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `cells` | `Array<Array<string>>` | — | Table cells as a 2D vector (rows × columns) |
| `markdown` | `string` | — | Markdown representation of the table |
| `pageNumber` | `number` | — | Page number where the table was found (1-indexed) |
| `boundingBox` | `OcrTableBoundingBox \| null` | `/* serde(default) */` | Bounding box of the table in pixel coordinates (from OCR word positions). |

---

#### OcrTableBoundingBox

Bounding box for an OCR-detected table in pixel coordinates.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `left` | `number` | — | Left x-coordinate (pixels) |
| `top` | `number` | — | Top y-coordinate (pixels) |
| `right` | `number` | — | Right x-coordinate (pixels) |
| `bottom` | `number` | — | Bottom y-coordinate (pixels) |

---

#### OrientationResult

Document orientation detection result.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `degrees` | `number` | — | Detected orientation in degrees (0, 90, 180, or 270). |
| `confidence` | `number` | — | Confidence score (0.0-1.0). |

---

#### PaddleOcrConfig

Configuration for PaddleOCR backend.

Configures PaddleOCR text detection and recognition with multi-language support.
Uses a builder pattern for convenient configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `language` | `string` | — | Language code (e.g., "en", "ch", "jpn", "kor", "deu", "fra") |
| `cacheDir` | `string \| null` | `null` | Optional custom cache directory for model files |
| `useAngleCls` | `boolean` | — | Enable angle classification for rotated text (default: false). Can misfire on short text regions, rotating crops incorrectly before recognition. |
| `enableTableDetection` | `boolean` | — | Enable table structure detection (default: false) |
| `detDbThresh` | `number` | — | Database threshold for text detection (default: 0.3) Range: 0.0-1.0, higher values require more confident detections |
| `detDbBoxThresh` | `number` | — | Box threshold for text bounding box refinement (default: 0.5) Range: 0.0-1.0 |
| `detDbUnclipRatio` | `number` | — | Unclip ratio for expanding text bounding boxes (default: 1.6) Controls the expansion of detected text regions |
| `detLimitSideLen` | `number` | — | Maximum side length for detection image (default: 960) Larger images may be resized to this limit for faster inference |
| `recBatchNum` | `number` | — | Batch size for recognition inference (default: 6) Number of text regions to process simultaneously |
| `padding` | `number` | — | Padding in pixels added around the image before detection (default: 10). Large values can include surrounding content like table gridlines. |
| `dropScore` | `number` | — | Minimum recognition confidence score for text lines (default: 0.5). Text regions with recognition confidence below this threshold are discarded. Matches PaddleOCR Python's `drop_score` parameter. Range: 0.0-1.0 |
| `modelTier` | `string` | — | Model tier controlling detection/recognition model size and accuracy trade-off. - `"mobile"` (default): Lightweight models (~4.5MB detection, ~16.5MB recognition), fast download and inference - `"server"`: Large, high-accuracy models (~88MB detection, ~84MB recognition), best for GPU or complex documents |

### Methods

#### withCacheDir()

Sets a custom cache directory for model files.

**Signature:**

```typescript
withCacheDir(path: string): PaddleOcrConfig
```

#### withTableDetection()

Enables or disables table structure detection.

**Signature:**

```typescript
withTableDetection(enable: boolean): PaddleOcrConfig
```

#### withAngleCls()

Enables or disables angle classification for rotated text.

**Signature:**

```typescript
withAngleCls(enable: boolean): PaddleOcrConfig
```

#### withDetDbThresh()

Sets the database threshold for text detection.

**Signature:**

```typescript
withDetDbThresh(threshold: number): PaddleOcrConfig
```

#### withDetDbBoxThresh()

Sets the box threshold for text bounding box refinement.

**Signature:**

```typescript
withDetDbBoxThresh(threshold: number): PaddleOcrConfig
```

#### withDetDbUnclipRatio()

Sets the unclip ratio for expanding text bounding boxes.

**Signature:**

```typescript
withDetDbUnclipRatio(ratio: number): PaddleOcrConfig
```

#### withDetLimitSideLen()

Sets the maximum side length for detection images.

**Signature:**

```typescript
withDetLimitSideLen(length: number): PaddleOcrConfig
```

#### withRecBatchNum()

Sets the batch size for recognition inference.

**Signature:**

```typescript
withRecBatchNum(batchSize: number): PaddleOcrConfig
```

#### withDropScore()

Sets the minimum recognition confidence threshold.

**Signature:**

```typescript
withDropScore(score: number): PaddleOcrConfig
```

#### withPadding()

Sets padding in pixels added around images before detection.

**Signature:**

```typescript
withPadding(padding: number): PaddleOcrConfig
```

#### withModelTier()

Sets the model tier controlling detection/recognition model size.

**Signature:**

```typescript
withModelTier(tier: string): PaddleOcrConfig
```

#### default()

Creates a default configuration with English language support.

**Signature:**

```typescript
static default(): PaddleOcrConfig
```

---

#### PageBoundary

Byte offset boundary for a page.

Tracks where a specific page's content starts and ends in the main content string,
enabling mapping from byte positions to page numbers. Offsets are guaranteed to be
at valid UTF-8 character boundaries when using standard String methods (push_str, push, etc.).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `byteStart` | `number` | — | Byte offset where this page starts in the content string (UTF-8 valid boundary, inclusive) |
| `byteEnd` | `number` | — | Byte offset where this page ends in the content string (UTF-8 valid boundary, exclusive) |
| `pageNumber` | `number` | — | Page number (1-indexed) |

---

#### PageClassification

Classification result for a single page.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `pageNumber` | `number` | — | 1-indexed page number this classification belongs to. |
| `labels` | `Array<ClassificationLabel>` | — | Labels assigned to the page. Single-label classification yields exactly one entry; multi-label classification yields any subset of the configured label set. |

---

#### PageClassificationConfig

**Since:** `v5.0.0`

Configuration for the page-classification post-processor.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `promptTemplate` | `string \| null` | `null` | Minijinja prompt template. Receives `{{ labels }}` (joined list), `{{ page_text }}` and `{{ multi_label }}` variables. `null` lets the backend pick a sensible default. |
| `labels` | `Array<string>` | — | The set of labels the classifier may emit. Must contain at least one entry. |
| `multiLabel` | `boolean` | `/* serde(default) */` | Allow multiple labels per page. Single-label mode returns at most one label. |
| `llm` | `LlmConfig` | — | LLM configuration used for classification. |

---

#### PageConfig

Page extraction and tracking configuration.

Controls how pages are extracted, tracked, and represented in the extraction results.
When `null`, page tracking is disabled.

Page range tracking in chunk metadata (first_page/last_page) is automatically enabled
when page boundaries are available and chunking is configured.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `extractPages` | `boolean` | `false` | Extract pages as separate array (ExtractionResult.pages) |
| `insertPageMarkers` | `boolean` | `false` | Insert page markers in main content string |
| `markerFormat` | `string` | `"<!-- PAGE {page_num} -->"` | Page marker format (use {page_num} placeholder) Default: "\n\n<!-- PAGE {page_num} -->\n\n" |

### Methods

#### default()

**Signature:**

```typescript
static default(): PageConfig
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
| `pageNumber` | `number` | — | Page number (1-indexed) |
| `content` | `string` | — | Text content for this page |
| `tables` | `Array<Table>` | `/* serde(default) */` | Tables found on this page (uses Arc for memory efficiency) Serializes as Vec<Table> for JSON compatibility while maintaining Arc semantics in-memory for zero-copy sharing. |
| `imageIndices` | `Array<number>` | `/* serde(default) */` | Indices into `ExtractionResult.images` for images found on this page. Each value is a zero-based index into the top-level `images` collection. Only populated when `extract_images = true` in the extraction config. |
| `hierarchy` | `PageHierarchy \| null` | `null` | Hierarchy information for the page (when hierarchy extraction is enabled) Contains text hierarchy levels (H1-H6) extracted from the page content. |
| `isBlank` | `boolean \| null` | `null` | Whether this page is blank (no meaningful text content) Determined during extraction based on text content analysis. A page is blank if it has fewer than 3 non-whitespace characters and contains no tables or images. |
| `layoutRegions` | `Array<LayoutRegion> \| null` | `null` | Layout detection regions for this page (when layout detection is enabled). Contains detected layout regions with class, confidence, bounding box, and area fraction. Only populated when layout detection is configured. |
| `speakerNotes` | `string \| null` | `null` | Speaker notes for this slide (PPTX only). Contains the text from the slide's notes pane (`ppt/notesSlides/notesSlide{N}.xml`). Only populated when the source is a PPTX file and notes are present. |
| `sectionName` | `string \| null` | `null` | Section name this slide belongs to (PPTX only). PowerPoint sections group slides into logical chapters (`<p:sectionLst>` in `ppt/presentation.xml`). Only populated when the source is a PPTX file and the slide belongs to a named section. |
| `sheetName` | `string \| null` | `null` | Sheet name for this page (XLSX/ODS only). Each spreadsheet sheet maps to one `PageContent` entry. This field carries the sheet's display name as it appears in the workbook. `null` for all non-spreadsheet formats and for sheets with an empty name. |

---

#### PageHierarchy

Page hierarchy structure containing heading levels and block information.

Used when PDF text hierarchy extraction is enabled. Contains hierarchical
blocks with heading levels (H1-H6) for semantic document structure.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `blockCount` | `number` | — | Number of hierarchy blocks on this page |
| `blocks` | `Array<HierarchicalBlock>` | `/* serde(default) */` | Hierarchical blocks with heading levels |

---

#### PageInfo

Metadata for individual page/slide/sheet.

Captures per-page information including dimensions, content counts,
and visibility state (for presentations).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `number` | `number` | — | Page number (1-indexed) |
| `title` | `string \| null` | `null` | Page title (usually for presentations) |
| `dimensions` | `Array<number> \| null` | `null` | Dimensions in points (PDF) or pixels (images): (width, height) |
| `imageCount` | `number \| null` | `null` | Number of images on this page |
| `tableCount` | `number \| null` | `null` | Number of tables on this page |
| `hidden` | `boolean \| null` | `null` | Whether this page is hidden (e.g., in presentations) |
| `isBlank` | `boolean \| null` | `null` | Whether this page is blank (no meaningful text, no images, no tables) A page is considered blank if it has fewer than 3 non-whitespace characters and contains no tables or images. This is useful for filtering out empty pages in scanned documents or PDFs with blank separator pages. |
| `hasVectorGraphics` | `boolean` | `/* serde(default) */` | Whether this page contains non-trivial vector graphics (paths, shapes, curves) Indicates the presence of vector-drawn content such as charts, diagrams, or geometric shapes (e.g., from Adobe InDesign, LaTeX TikZ). These are invisible to `ExtractionResult.images` since they are not embedded as raster XObjects. Set to `true` when path count exceeds a heuristic threshold, signaling that downstream consumers may want to rasterize the page to capture this content. Only populated for PDFs; `null` for other document types. |

---

#### PageStructure

Unified page structure for documents.

Supports different page types (PDF pages, PPTX slides, Excel sheets)
with character offset boundaries for chunk-to-page mapping.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `totalCount` | `number` | — | Total number of pages/slides/sheets |
| `unitType` | `PageUnitType` | — | Type of paginated unit |
| `boundaries` | `Array<PageBoundary> \| null` | `null` | Character offset boundaries for each page Maps character ranges in the extracted content to page numbers. Used for chunk page range calculation. |
| `pages` | `Array<PageInfo> \| null` | `null` | Detailed per-page metadata (optional, only when needed) |

---

#### PatternMatch

One detected PII span in the input text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `start` | `number` | — | Inclusive byte-offset start of the match in the source text. |
| `end` | `number` | — | Exclusive byte-offset end of the match. |
| `category` | `PiiCategory` | — | Category the match belongs to. |
| `text` | `string` | — | Matched substring (owned copy — pattern engine returns owned data so the caller can free the original text if needed before replacement). |

---

#### PdfAnnotation

A PDF annotation extracted from a document page.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `annotationType` | `PdfAnnotationType` | — | The type of annotation. |
| `content` | `string \| null` | `null` | Text content of the annotation (e.g., comment text, link URL). |
| `pageNumber` | `number` | — | Page number where the annotation appears (1-indexed). |
| `boundingBox` | `BoundingBox \| null` | `null` | Bounding box of the annotation on the page. |

---

#### PdfConfig

PDF-specific configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `extractImages` | `boolean` | `false` | Extract images from PDF |
| `extractTables` | `boolean` | `true` | Extract tables from PDF. When `true` (default), runs pdf_oxide's native grid detector and, if it finds nothing, falls back to the heuristic text-layer reconstruction in `pdf.oxide.table.extract_tables_heuristic`. Set to `false` to skip both passes — `tables` will then be empty in the result. |
| `passwords` | `Array<string> \| null` | `null` | List of passwords to try when opening encrypted PDFs |
| `extractMetadata` | `boolean` | `true` | Extract PDF metadata |
| `hierarchy` | `HierarchyConfig \| null` | `null` | Hierarchy extraction configuration (None = hierarchy extraction disabled) |
| `extractAnnotations` | `boolean` | `false` | Extract PDF annotations (text notes, highlights, links, stamps). Default: false |
| `topMarginFraction` | `number \| null` | `null` | Top margin fraction (0.0–1.0) of page height to exclude headers/running heads. Default: 0.06 (6%) |
| `bottomMarginFraction` | `number \| null` | `null` | Bottom margin fraction (0.0–1.0) of page height to exclude footers/page numbers. Default: 0.05 (5%) |
| `allowSingleColumnTables` | `boolean` | `false` | Allow single-column pseudo tables in extraction results. By default, tables with fewer than 2 columns (layout-guided) or 3 columns (heuristic) are rejected. When `true`, the minimum column count is relaxed to 1, allowing single-column structured data (glossaries, itemized lists) to be emitted as tables. Other quality filters (density, sparsity, prose detection) still apply. |
| `ocrInlineImages` | `boolean` | `false` | Perform OCR on inline images extracted from PDF pages and attach the recognized text to each `ExtractedImage.ocr_result`. Requires Tesseract to be available; if `ExtractionConfig.ocr` is `null` the extractor falls back to `TesseractConfig.default()`. Per-image failures degrade gracefully (the image is returned without OCR text rather than failing the whole extraction). Default: `false`. |

### Methods

#### default()

**Signature:**

```typescript
static default(): PdfConfig
```

---

#### PdfMetadata

PDF-specific metadata.

Contains metadata fields specific to PDF documents that are not in the common
`Metadata` structure. Common fields like title, authors, keywords, and dates
are at the `Metadata` level.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `pdfVersion` | `string \| null` | `null` | PDF version (e.g., "1.7", "2.0") |
| `producer` | `string \| null` | `null` | PDF producer (application that created the PDF) |
| `isEncrypted` | `boolean \| null` | `null` | Whether the PDF is encrypted/password-protected |
| `width` | `number \| null` | `null` | First page width in points (1/72 inch) |
| `height` | `number \| null` | `null` | First page height in points (1/72 inch) |
| `pageCount` | `number \| null` | `null` | Total number of pages in the PDF document |

---

#### Plugin

Base trait that all plugins must implement.

This trait provides common functionality for plugin lifecycle management,
identification, and metadata.

### Thread Safety

All plugins must be `Send + Sync` to support concurrent usage across threads.

### Methods

#### name()

Returns the unique name/identifier for this plugin.

The name should be:

- Unique across all plugins
- Lowercase with hyphens (e.g., "my-custom-plugin")
- URL-safe characters only

**Signature:**

```typescript
name(): string
```

#### version()

Returns the semantic version of this plugin.

Should follow semver format: `MAJOR.MINOR.PATCH`

Defaults to the kreuzberg crate version.

**Signature:**

```typescript
version(): string
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

```typescript
initialize(): void
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

```typescript
shutdown(): void
```

#### description()

Optional plugin description for debugging and logging.

Defaults to empty string if not overridden.

**Signature:**

```typescript
description(): string
```

#### author()

Optional plugin author information.

Defaults to empty string if not overridden.

**Signature:**

```typescript
author(): string
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

### Methods

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

```typescript
process(result: ExtractionResult, config: ExtractionConfig): void
```

#### processingStage()

Get the processing stage for this post-processor.

Determines when this processor runs in the pipeline.

**Returns:**

The `ProcessingStage` (Early, Middle, or Late).

**Signature:**

```typescript
processingStage(): ProcessingStage
```

#### shouldProcess()

Optional: Check if this processor should run for a given result.

Allows conditional processing based on MIME type, metadata, or content.
Defaults to `true` (always run).

**Returns:**

`true` if the processor should run, `false` to skip.

**Signature:**

```typescript
shouldProcess(result: ExtractionResult, config: ExtractionConfig): boolean
```

#### estimatedDurationMs()

Optional: Estimate processing time in milliseconds.

Used for logging and debugging. Defaults to 0 (unknown).

**Returns:**

Estimated processing time in milliseconds.

**Signature:**

```typescript
estimatedDurationMs(result: ExtractionResult): number
```

#### priority()

Execution priority within the processing stage.

Higher values run first within the same `ProcessingStage`. Defaults to 50.
Use 0-49 for fallback processors, 50 for normal processors, and 51-255
for high-priority processors that should run early in their stage.

**Signature:**

```typescript
priority(): number
```

---

#### PostProcessorConfig

Post-processor configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `boolean` | `true` | Enable post-processors |
| `enabledProcessors` | `Array<string> \| null` | `null` | Whitelist of processor names to run (None = all enabled) |
| `disabledProcessors` | `Array<string> \| null` | `null` | Blacklist of processor names to skip (None = none disabled) |
| `enabledSet` | `Array<string> \| null` | `null` | Pre-computed AHashSet for O(1) enabled processor lookup |
| `disabledSet` | `Array<string> \| null` | `null` | Pre-computed AHashSet for O(1) disabled processor lookup |

### Methods

#### default()

**Signature:**

```typescript
static default(): PostProcessorConfig
```

---

#### PptxAppProperties

Application properties from docProps/app.xml for PPTX

Contains PowerPoint-specific document metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `application` | `string \| null` | `null` | Application name (e.g., "Microsoft Office PowerPoint") |
| `appVersion` | `string \| null` | `null` | Application version |
| `totalTime` | `number \| null` | `null` | Total editing time in minutes |
| `company` | `string \| null` | `null` | Company name |
| `docSecurity` | `number \| null` | `null` | Document security level |
| `scaleCrop` | `boolean \| null` | `null` | Scale crop flag |
| `linksUpToDate` | `boolean \| null` | `null` | Links up to date flag |
| `sharedDoc` | `boolean \| null` | `null` | Shared document flag |
| `hyperlinksChanged` | `boolean \| null` | `null` | Hyperlinks changed flag |
| `slides` | `number \| null` | `null` | Number of slides |
| `notes` | `number \| null` | `null` | Number of notes |
| `hiddenSlides` | `number \| null` | `null` | Number of hidden slides |
| `multimediaClips` | `number \| null` | `null` | Number of multimedia clips |
| `presentationFormat` | `string \| null` | `null` | Presentation format (e.g., "Widescreen", "Standard") |
| `slideTitles` | `Array<string>` | `[]` | Slide titles |

---

#### PptxExtractionResult

PowerPoint (PPTX) extraction result.

Contains extracted slide content, metadata, and embedded images/tables.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `string` | — | Extracted text content from all slides |
| `metadata` | `PptxMetadata` | — | Presentation metadata |
| `slideCount` | `number` | — | Total number of slides |
| `imageCount` | `number` | — | Total number of embedded images |
| `tableCount` | `number` | — | Total number of tables |
| `images` | `Array<ExtractedImage>` | — | Extracted images from the presentation |
| `pageStructure` | `PageStructure \| null` | `null` | Slide structure with boundaries (when page tracking is enabled) |
| `pageContents` | `Array<PageContent> \| null` | `null` | Per-slide content (when page tracking is enabled) |
| `document` | `DocumentStructure \| null` | `null` | Structured document representation |
| `hyperlinks` | `Array<string>` | `/* serde(default) */` | Hyperlinks discovered in slides as (url, optional_label) pairs. |
| `officeMetadata` | `Record<string, string>` | `/* serde(default) */` | Office metadata extracted from docProps/core.xml and docProps/app.xml. Contains keys like "title", "author", "created_by", "subject", "keywords", "modified_by", "created_at", "modified_at", etc. |
| `revisions` | `Array<DocumentRevision> \| null` | `/* serde(default) */` | Slide comments as revisions. Each `<p:cm>` element in `ppt/comments/comment{N}.xml` becomes a `DocumentRevision { kind: Comment }` with author (resolved from `ppt/commentAuthors.xml`), ISO-8601 timestamp, and `RevisionAnchor.Slide { index }`. `null` when no comment XML parts exist. |

---

#### PptxMetadata

PowerPoint presentation metadata.

Extracted from PPTX files containing slide counts and presentation details.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `slideCount` | `number` | — | Total number of slides in the presentation |
| `slideNames` | `Array<string>` | `[]` | Names of slides (if available) |
| `imageCount` | `number \| null` | `null` | Number of embedded images |
| `tableCount` | `number \| null` | `null` | Number of tables |

---

#### ProcessingWarning

A non-fatal warning from a processing pipeline stage.

Captures errors from optional features that don't prevent extraction
but may indicate degraded results.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `source` | `string` | — | The pipeline stage or feature that produced this warning (e.g., "embedding", "chunking", "language_detection", "output_format"). |
| `message` | `string` | — | Human-readable description of what went wrong. |

---

#### PstMetadata

Outlook PST archive metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `messageCount` | `number` | — | Total number of email messages found in the PST archive. |

---

#### QrBoundingBox

Pixel-space bounding box of a QR code inside its source image.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `x` | `number` | — | Horizontal pixel offset of the bounding box top-left corner. |
| `y` | `number` | — | Vertical pixel offset of the bounding box top-left corner. |
| `width` | `number` | — | Width of the bounding box in pixels. |
| `height` | `number` | — | Height of the bounding box in pixels. |

---

#### QrCode

One QR code decoded from an extracted image.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `payload` | `string` | — | Decoded payload (text, URL, vCard string, …). |
| `confidence` | `number \| null` | `null` | Detector-reported confidence in `[0.0, 1.0]`. `null` when the decoder does not expose confidence (the default `rqrr` backend always reports `Some` because successful decode implies high confidence). |
| `bbox` | `QrBoundingBox \| null` | `null` | Bounding box of the QR code inside the source image, in pixel coordinates (`x`, `y` of the top-left corner; `width`, `height` of the rectangle). `null` if the decoder did not report a bounding box. |

---

#### RakeParams

RAKE-specific parameters.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `minWordLength` | `number` | `1` | Minimum word length to consider (default: 1). |
| `maxWordsPerPhrase` | `number` | `3` | Maximum words in a keyword phrase (default: 3). |

### Methods

#### default()

**Signature:**

```typescript
static default(): RakeParams
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
| `detectionBbox` | `BBox` | — | Detection bbox that this table corresponds to (for matching). |
| `cells` | `Array<Array<string>>` | — | Table cells as a 2D vector (rows × columns). |
| `markdown` | `string` | — | Rendered markdown table. |

---

#### RedactionConfig

**Since:** `v5.0.0`

Configuration for the redaction post-processor.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `categories` | `Array<PiiCategory>` | `[]` | Categories to redact. Empty means "every category supported by the engine." |
| `strategy` | `RedactionStrategy` | `RedactionStrategy.Mask` | Strategy applied to every match. |
| `ner` | `NerConfig \| null` | `null` | Optional NER backend — required to redact PERSON / ORGANIZATION / LOCATION categories (the pure-Rust pattern engine only covers regex-detectable PII). |
| `preserveOffsets` | `boolean` | `true` | When `true`, chunk byte ranges are kept consistent with the rewritten content by adjusting `byte_start` / `byte_end` after replacement. When `false`, chunk byte ranges still refer to the *original* content offsets — useful when downstream consumers want to map findings back to the original document. |
| `customTerms` | `Array<RedactionTerm>` | `[]` | Arbitrary user-supplied literal terms to redact. Each term is treated as a regex hit against the document, surfacing as `PiiCategory.Custom(label)` in `RedactionFinding` where `label` is the per-term label (defaulting to the literal value itself). Case-insensitive by default; set `RedactionTerm.case_sensitive` for exact match. Use this when you need to redact tenant-specific tokens (employee IDs, project codes, internal product names) without writing a custom plugin. |
| `customPatterns` | `Array<RedactionPattern>` | `[]` | Arbitrary user-supplied regex patterns to redact. Same surfacing semantics as `custom_terms`: each hit becomes a `PiiCategory.Custom(label)` finding. Patterns are validated at config-construction time via `RedactionConfig.validate`. |

### Methods

#### default()

**Signature:**

```typescript
static default(): RedactionConfig
```

#### validate()

Validate user-supplied terms and patterns at config-construction time.

Compiles every `RedactionPattern.pattern` (with the case-insensitive
inline flag where applicable) and returns the first compilation error so
the caller can reject the config before the redaction pipeline runs.
Pure terms (regex-escaped) cannot fail to compile, but the function
still rejects empty values to avoid degenerate zero-length matches.

**Signature:**

```typescript
validate(): void
```

---

#### RedactionFinding

One redaction event: which span was rewritten, why, and with what.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `start` | `number` | — | Byte-offset start in the original (pre-redaction) `ExtractionResult.content`. |
| `end` | `number` | — | Byte-offset end (exclusive) in the original `ExtractionResult.content`. |
| `category` | `PiiCategory` | — | PII category that fired this redaction. |
| `strategy` | `RedactionStrategy` | — | Strategy applied to this finding (mask, hash, token-replace, drop). |
| `replacementToken` | `string` | — | String that replaced the original mention. Always present; for `Drop` the replacement is the empty string. |

---

#### RedactionPattern

One user-supplied regex pattern to redact.

The pattern is compiled with the Rust `regex` crate (no look-around). Case
sensitivity is encoded in the pattern via the `(?i)` inline flag when
`Self.case_sensitive` is `false`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `label` | `string` | — | Custom category label surfaced in `RedactionFinding.category`. |
| `pattern` | `string` | — | Regex pattern (Rust `regex` crate dialect — no look-around). |
| `caseSensitive` | `boolean` | `/* serde(default) */` | When `true`, match case-sensitively; otherwise prepend `(?i)` to the regex. |

### Methods

#### labeled()

Build a pattern with the given label (case-insensitive by default).

**Signature:**

```typescript
static labeled(label: string, pattern: string): RedactionPattern
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
| `findings` | `Array<RedactionFinding>` | — | Individual redaction findings in original-source byte order. |
| `totalRedacted` | `number` | — | Total number of redactions applied across the document. |

---

#### RedactionTerm

One user-supplied literal term to redact.

Matched as a regex-escaped substring (so callers do not need to escape
metacharacters themselves). Case-insensitive by default — set
`Self.case_sensitive` to `true` for exact byte-match semantics.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `label` | `string` | — | Custom category label surfaced in `RedactionFinding.category`. |
| `value` | `string` | — | Literal value to match. Regex metacharacters are escaped automatically. |
| `caseSensitive` | `boolean` | `/* serde(default) */` | When `true`, match the value as-is; otherwise match ASCII-case-insensitively. |

### Methods

#### literal()

Build a term whose label is the literal value itself (case-insensitive).

**Signature:**

```typescript
static literal(value: string): RedactionTerm
```

#### labeled()

Build a term with a custom label.

**Signature:**

```typescript
static labeled(label: string, value: string): RedactionTerm
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

### Methods

#### render()

Render an `InternalDocument` to the output format.

**Returns:**

The rendered output as a string.

**Errors:**

Returns an error if rendering fails.

**Signature:**

```typescript
render(doc: InternalDocument): string
```

---

#### RevisionDelta

The content changes that make up a single revision.

For insertions and deletions the `content` field carries the added/removed
lines as `DiffLine.Added` / `DiffLine.Removed` entries. For format
changes, `content` is empty — the property diff is left as a TODO for a
later enrichment pass.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `Array<DiffLine>` | `[]` | Line-level content changes for this revision. |
| `tableChanges` | `Array<CellChange>` | `[]` | Cell-level table changes for this revision. |

---

#### SecurityLimits

Configuration for security limits across extractors.

All limits are intentionally conservative to prevent DoS attacks
while still supporting legitimate documents.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `maxArchiveSize` | `number` | `524288000` | Maximum uncompressed size for archives (500 MB) |
| `maxCompressionRatio` | `number` | `100` | Maximum compression ratio before flagging as potential bomb (100:1) |
| `maxFilesInArchive` | `number` | `10000` | Maximum number of files in archive (10,000) |
| `maxNestingDepth` | `number` | `1024` | Maximum nesting depth for structures (100) |
| `maxEntityLength` | `number` | `1048576` | Maximum length of any single XML entity / attribute / token (1 MiB). This is a per-token cap, NOT a total cap — billion-laughs class attacks where a single entity expands to hundreds of MB are caught here, while normal long text content (a paragraph, a CDATA block) is caught by `max_content_size` instead. |
| `maxContentSize` | `number` | `104857600` | Maximum string growth per document (100 MB) |
| `maxIterations` | `number` | `10000000` | Maximum iterations per operation |
| `maxXmlDepth` | `number` | `1024` | Maximum XML depth (100 levels) |
| `maxTableCells` | `number` | `100000` | Maximum cells per table (100,000) |

### Methods

#### default()

**Signature:**

```typescript
static default(): SecurityLimits
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
| `host` | `string` | — | Server host address (e.g., "127.0.0.1", "0.0.0.0") |
| `port` | `number` | — | Server port number |
| `corsOrigins` | `Array<string>` | `[]` | CORS allowed origins. Empty vector means allow all origins. If this is an empty vector, the server will accept requests from any origin. If populated with specific origins (e.g., `"<https://example.com"`>), only those origins will be allowed. |
| `maxRequestBodyBytes` | `number` | — | Maximum size of request body in bytes (default: 100 MB) |
| `maxMultipartFieldBytes` | `number` | — | Maximum size of multipart fields in bytes (default: 100 MB) |

### Methods

#### default()

**Signature:**

```typescript
static default(): ServerConfig
```

#### listenAddr()

Get the server listen address (host:port).

**Signature:**

```typescript
listenAddr(): string
```

#### corsAllowsAll()

Check if CORS allows all origins.

Returns `true` if the `cors_origins` vector is empty, meaning all origins
are allowed. Returns `false` if specific origins are configured.

**Signature:**

```typescript
corsAllowsAll(): boolean
```

#### isOriginAllowed()

Check if a given origin is allowed by CORS configuration.

Returns `true` if:

- CORS allows all origins (empty origins list), or
- The given origin is in the allowed origins list

**Signature:**

```typescript
isOriginAllowed(origin: string): boolean
```

#### maxRequestBodyMb()

Get maximum request body size in megabytes (rounded up).

**Signature:**

```typescript
maxRequestBodyMb(): number
```

#### maxMultipartFieldMb()

Get maximum multipart field size in megabytes (rounded up).

**Signature:**

```typescript
maxMultipartFieldMb(): number
```

---

#### StructuredData

Structured data (Schema.org, microdata, RDFa) block.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `dataType` | `StructuredDataType` | — | Type of structured data |
| `rawJson` | `string` | — | Raw JSON string representation |
| `schemaType` | `string \| null` | `null` | Schema type if detectable (e.g., "Article", "Event", "Product") |

---

#### StructuredDataResult

Result of parsing a structured data file (JSON, JSONL, YAML, or TOML).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `string` | — | The extracted text content, formatted for readability. |
| `format` | `string` | — | The source format identifier (e.g. `"json"`, `"yaml"`, `"toml"`). |
| `metadata` | `Record<string, string>` | — | Key-value metadata extracted from recognized text fields. |
| `textFields` | `Array<string>` | — | JSON paths of fields that were classified as text-bearing. |

---

#### StructuredExtractionConfig

Configuration for LLM-based structured data extraction.

Sends extracted document content to a VLM with a JSON schema,
returning structured data that conforms to the schema.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `schema` | `unknown` | — | JSON Schema defining the desired output structure. |
| `schemaName` | `string` | `/* serde(default) */` | Schema name passed to the LLM's structured output mode. |
| `schemaDescription` | `string \| null` | `/* serde(default) */` | Optional schema description for the LLM. |
| `strict` | `boolean` | `/* serde(default) */` | Enable strict mode — output must exactly match the schema. |
| `prompt` | `string \| null` | `/* serde(default) */` | Custom Jinja2 extraction prompt template. When `null`, a default template is used. Available template variables: - `{{ content }}` — The extracted document text. - `{{ schema }}` — The JSON schema as a formatted string. - `{{ schema_name }}` — The schema name. - `{{ schema_description }}` — The schema description (may be empty). |
| `llm` | `LlmConfig` | — | LLM configuration for the extraction. |

---

#### SummarizationConfig

**Since:** `v5.0.0`

Configuration for the summarisation post-processor.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `strategy` | `SummaryStrategy` | `SummaryStrategy.Extractive` | Summarisation strategy. |
| `maxTokens` | `number \| null` | `null` | Maximum summary length in tokens. `null` lets the backend pick a default. |
| `llm` | `LlmConfig \| null` | `null` | LLM configuration for the abstractive backend. Ignored when `strategy = Extractive`. Required when `strategy = Abstractive`. |

---

#### SupportedFormat

A supported document format entry.

Represents a file extension and its corresponding MIME type that Kreuzberg can process.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `extension` | `string` | — | File extension (without leading dot), e.g., "pdf", "docx" |
| `mimeType` | `string` | — | MIME type string, e.g., "application/pdf" |

---

#### Table

Extracted table structure.

Represents a table detected and extracted from a document (PDF, image, etc.).
Tables are converted to both structured cell data and Markdown format.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `cells` | `Array<Array<string>>` | `[]` | Table cells as a 2D vector (rows × columns) |
| `markdown` | `string` | — | Markdown representation of the table |
| `pageNumber` | `number` | — | Page number where the table was found (1-indexed) |
| `boundingBox` | `BoundingBox \| null` | `null` | Bounding box of the table on the page (PDF coordinates: x0=left, y0=bottom, x1=right, y1=top). Only populated for PDF-extracted tables when position data is available. |

---

#### TableCell

Individual table cell with content and optional styling.

Future extension point for rich table support with cell-level metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `string` | — | Cell content as text |
| `rowSpan` | `number` | — | Row span (number of rows this cell spans) |
| `colSpan` | `number` | — | Column span (number of columns this cell spans) |
| `isHeader` | `boolean` | — | Whether this is a header cell |

---

#### TableDiff

Cell-level changes for a pair of tables that share the same index.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `fromIndex` | `number` | — | Zero-based index of the table in both `a.tables` and `b.tables`. |
| `toIndex` | `number` | — | Zero-based index in `b.tables` (equal to `from_index` for same-dimension tables). |
| `cellChanges` | `Array<CellChange>` | — | Cell-level changes within the table. |

---

#### TableGrid

Structured table grid with cell-level metadata.

Stores row/column dimensions and a flat list of cells with position info.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `rows` | `number` | — | Number of rows in the table. |
| `cols` | `number` | — | Number of columns in the table. |
| `cells` | `Array<GridCell>` | `[]` | All cells in row-major order. |

---

#### TesseractConfig

Tesseract OCR configuration.

Provides fine-grained control over Tesseract OCR engine parameters.
Most users can use the defaults, but these settings allow optimization
for specific document types (invoices, handwriting, etc.).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `language` | `string` | `"eng"` | Language code (e.g., "eng", "deu", "fra") |
| `psm` | `number` | `3` | Page Segmentation Mode (0-13). Common values: - 3: Fully automatic page segmentation (native default) - 6: Assume a single uniform block of text (WASM default — avoids layout-analysis hang) - 11: Sparse text with no particular order |
| `outputFormat` | `string` | `"markdown"` | Output format ("text" or "markdown") |
| `oem` | `number` | `3` | OCR Engine Mode (0-3). - 0: Legacy engine only - 1: Neural nets (LSTM) only (usually best) - 2: Legacy + LSTM - 3: Default (based on what's available) |
| `minConfidence` | `number` | `0` | Minimum confidence threshold (0.0-100.0). Words with confidence below this threshold may be rejected or flagged. |
| `preprocessing` | `ImagePreprocessingConfig \| null` | `null` | Image preprocessing configuration. Controls how images are preprocessed before OCR. Can significantly improve quality for scanned documents or low-quality images. |
| `enableTableDetection` | `boolean` | `true` | Enable automatic table detection and reconstruction |
| `tableMinConfidence` | `number` | `0` | Minimum confidence threshold for table detection (0.0-1.0) |
| `tableColumnThreshold` | `number` | `50` | Column threshold for table detection (pixels) |
| `tableRowThresholdRatio` | `number` | `0.5` | Row threshold ratio for table detection (0.0-1.0) |
| `useCache` | `boolean` | `true` | Enable OCR result caching |
| `classifyUsePreAdaptedTemplates` | `boolean` | `true` | Use pre-adapted templates for character classification |
| `languageModelNgramOn` | `boolean` | `false` | Enable N-gram language model |
| `tesseditDontBlkrejGoodWds` | `boolean` | `true` | Don't reject good words during block-level processing |
| `tesseditDontRowrejGoodWds` | `boolean` | `true` | Don't reject good words during row-level processing |
| `tesseditEnableDictCorrection` | `boolean` | `true` | Enable dictionary correction |
| `tesseditCharWhitelist` | `string` | `""` | Whitelist of allowed characters (empty = all allowed) |
| `tesseditCharBlacklist` | `string` | `""` | Blacklist of forbidden characters (empty = none forbidden) |
| `tesseditUsePrimaryParamsModel` | `boolean` | `true` | Use primary language params model |
| `textordSpaceSizeIsVariable` | `boolean` | `true` | Variable-width space detection |
| `thresholdingMethod` | `boolean` | `false` | Use adaptive thresholding method |

### Methods

#### default()

**Signature:**

```typescript
static default(): TesseractConfig
```

---

#### TextAnnotation

Inline text annotation — byte-range based formatting and links.

Annotations reference byte offsets into the node's text content,
enabling precise identification of formatted regions.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `start` | `number` | — | Start byte offset in the node's text content (inclusive). |
| `end` | `number` | — | End byte offset in the node's text content (exclusive). |
| `kind` | `AnnotationKind` | — | Annotation type. |

---

#### TextExtractionResult

Plain text and Markdown extraction result.

Contains the extracted text along with statistics and,
for Markdown files, structural elements like headers and links.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `string` | — | Extracted text content |
| `lineCount` | `number` | — | Number of lines |
| `wordCount` | `number` | — | Number of words |
| `characterCount` | `number` | — | Number of characters |
| `headers` | `Array<string> \| null` | `null` | Markdown headers (text only, Markdown files only) |
| `links` | `Array<Array<string>> \| null` | `null` | Markdown links as (text, URL) tuples (Markdown files only) |
| `codeBlocks` | `Array<Array<string>> \| null` | `null` | Code blocks as (language, code) tuples (Markdown files only) |

---

#### TextMetadata

Text/Markdown metadata.

Extracted from plain text and Markdown files. Includes word counts and,
for Markdown, structural elements like headers and links.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `lineCount` | `number` | — | Number of lines in the document |
| `wordCount` | `number` | — | Number of words |
| `characterCount` | `number` | — | Number of characters |
| `headers` | `Array<string> \| null` | `[]` | Markdown headers (headings text only, for Markdown files) |
| `links` | `Array<Array<string>> \| null` | `[]` | Markdown links as (text, url) tuples (for Markdown files) |
| `codeBlocks` | `Array<Array<string>> \| null` | `[]` | Code blocks as (language, code) tuples (for Markdown files) |

---

#### TokenCounter

Per-category running counter for `RedactionStrategy.TokenReplace`.

### Methods

#### new()

Create a fresh counter with no previous state.

**Signature:**

```typescript
static new(): TokenCounter
```

---

#### TokenReductionConfig

Configuration for the token-reduction pipeline.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `level` | `ReductionLevel` | `ReductionLevel.Moderate` | Reduction intensity level. |
| `languageHint` | `string \| null` | `null` | ISO 639-1 language code hint for stopword selection (e.g. `"en"`, `"de"`). |
| `preserveMarkdown` | `boolean` | `false` | Preserve Markdown formatting tokens during reduction. |
| `preserveCode` | `boolean` | `true` | Preserve code block contents unchanged. |
| `semanticThreshold` | `number` | `0.3` | Cosine similarity threshold below which sentences are considered dissimilar. |
| `enableParallel` | `boolean` | `true` | Use Rayon parallel iterators for multi-core processing. |
| `useSimd` | `boolean` | `true` | Use SIMD-optimized text scanning where available. |
| `customStopwords` | `Record<string, Array<string>> \| null` | `null` | Per-language custom stopword lists (`language_code → stopword_list`). |
| `preservePatterns` | `Array<string>` | `[]` | Regex patterns whose matched text is always preserved unchanged. |
| `targetReduction` | `number \| null` | `null` | Target fraction of text to retain (0.0–1.0); `null` = no fixed target. |
| `enableSemanticClustering` | `boolean` | `false` | Group semantically similar sentences and emit only one per cluster. |

### Methods

#### default()

**Signature:**

```typescript
static default(): TokenReductionConfig
```

---

#### TokenReductionOptions

Token reduction configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `mode` | `string` | — | Reduction mode: "off", "light", "moderate", "aggressive", "maximum" |
| `preserveImportantWords` | `boolean` | `true` | Preserve important words (capitalized, technical terms) |

### Methods

#### default()

**Signature:**

```typescript
static default(): TokenReductionOptions
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
| `enabled` | `boolean` | `true` | Master switch. When false the block is ignored and audio files fall back to the normal "unsupported format" path. |
| `model` | `WhisperModel` | `WhisperModel.Tiny` | Whisper model size to use. Smaller = faster + lower memory. `tiny` is the pragmatic default for first-time users and CI. |
| `language` | `string \| null` | `null` | Optional language hint (ISO-639-1 code, e.g. "en", "de"). When `null` (default) the engine may attempt auto-detection if supported. For deterministic production output, always set this explicitly. |
| `timestamps` | `boolean` | `false` | Whether to emit segment-level timestamps in the result metadata. When true, `metadata["transcription.segments"]` will contain an array of `{start_ms, end_ms, text}` objects (if the engine supports it). |
| `maxDurationMs` | `number \| null` | `null` | Hard safety limit on input duration (milliseconds). Files longer than this are rejected *before* any decode or model work. Default: 30 minutes. Set to `null` to disable (not recommended for untrusted input). |
| `maxBytes` | `number \| null` | `null` | Hard safety limit on input size (bytes). Default: 512 MiB. Protects against pathological or malicious uploads. |
| `timeoutMs` | `number \| null` | `null` | Wall-clock timeout for the entire transcription operation (ms). Includes model download (first time), decode, and inference. Default: 10 minutes. Uses `tokio.select!` so the async runtime is never blocked. |
| `modelCacheDir` | `string \| null` | `null` | Override the directory used for Whisper model cache. When `null`, uses the centralized resolver: `KREUZBERG_CACHE_DIR/transcription/whisper` or the platform default (`~/.cache/kreuzberg/transcription/whisper` on Linux, etc.). |
| `allowNetwork` | `boolean` | `true` | Allow network access to download models from Hugging Face Hub. When `false`, only previously cached models may be used. Useful for air-gapped or fully offline deployments. |
| `verifyHash` | `boolean` | `true` | Verify SHA256 checksums of downloaded model files (when known). Strongly recommended; disable only for debugging. |

### Methods

#### default()

**Signature:**

```typescript
static default(): TranscriptionConfig
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
| `targetLang` | `string` | — | BCP-47 language tag the translation was produced into (e.g. `"de"`, `"fr-CA"`). |
| `sourceLang` | `string \| null` | `null` | BCP-47 source language. `null` when the translation backend was asked to detect. |
| `content` | `string` | — | Translated plain-text body. Matches the shape of `ExtractionResult.content`. |
| `formattedContent` | `string \| null` | `null` | Translated markup body (Markdown / HTML / etc.) when `preserve_markup` was enabled on the config. `null` otherwise. |

---

#### TranslationConfig

**Since:** `v5.0.0`

Configuration for the translation post-processor.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `targetLang` | `string` | — | BCP-47 language tag for the target language (e.g. `"de"`, `"fr-CA"`). |
| `sourceLang` | `string \| null` | `null` | Optional explicit source language. `null` asks the backend to auto-detect. |
| `preserveMarkup` | `boolean` | `/* serde(default) */` | Translate the formatted (Markdown/HTML) rendition alongside plain text when `formatted_content` is present. |
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
| `enabled` | `boolean` | `true` | Enable code intelligence processing (default: true). When `false`, tree-sitter analysis is completely skipped even if the config section is present. |
| `cacheDir` | `string \| null` | `null` | Custom cache directory for downloaded grammars. When `null`, uses the default: `~/.cache/tree-sitter-language-pack/v{version}/libs/`. |
| `languages` | `Array<string> \| null` | `null` | Languages to pre-download on init (e.g., `["python", "rust"]`). |
| `groups` | `Array<string> \| null` | `null` | Language groups to pre-download (e.g., `["web", "systems", "scripting"]`). |
| `process` | `TreeSitterProcessConfig` | — | Processing options for code analysis. |

### Methods

#### default()

**Signature:**

```typescript
static default(): TreeSitterConfig
```

---

#### TreeSitterProcessConfig

Processing options for tree-sitter code analysis.

Controls which analysis features are enabled when extracting code files.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `structure` | `boolean` | `true` | Extract structural items (functions, classes, structs, etc.). Default: true. |
| `imports` | `boolean` | `true` | Extract import statements. Default: true. |
| `exports` | `boolean` | `true` | Extract export statements. Default: true. |
| `comments` | `boolean` | `false` | Extract comments. Default: false. |
| `docstrings` | `boolean` | `false` | Extract docstrings. Default: false. |
| `symbols` | `boolean` | `false` | Extract symbol definitions. Default: false. |
| `diagnostics` | `boolean` | `false` | Include parse diagnostics. Default: false. |
| `chunkMaxSize` | `number \| null` | `null` | Maximum chunk size in bytes. `null` disables chunking. |
| `contentMode` | `CodeContentMode` | `CodeContentMode.Chunks` | Content rendering mode for code extraction. |

### Methods

#### default()

**Signature:**

```typescript
static default(): TreeSitterProcessConfig
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

### Methods

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

```typescript
validate(result: ExtractionResult, config: ExtractionConfig): void
```

#### shouldValidate()

Optional: Check if this validator should run for a given result.

Allows conditional validation based on MIME type, metadata, or content.
Defaults to `true` (always run).

**Returns:**

`true` if the validator should run, `false` to skip.

**Signature:**

```typescript
shouldValidate(result: ExtractionResult, config: ExtractionConfig): boolean
```

#### priority()

Optional: Get the validation priority.

Higher priority validators run first. Useful for ordering validation checks
(e.g., run cheap validations before expensive ones).

Default priority is 50.

**Returns:**

Priority value (higher = runs earlier).

**Signature:**

```typescript
priority(): number
```

---

#### XlsxAppProperties

Application properties from docProps/app.xml for XLSX

Contains Excel-specific document metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `application` | `string \| null` | `null` | Application name (e.g., "Microsoft Excel") |
| `appVersion` | `string \| null` | `null` | Application version |
| `docSecurity` | `number \| null` | `null` | Document security level |
| `scaleCrop` | `boolean \| null` | `null` | Scale crop flag |
| `linksUpToDate` | `boolean \| null` | `null` | Links up to date flag |
| `sharedDoc` | `boolean \| null` | `null` | Shared document flag |
| `hyperlinksChanged` | `boolean \| null` | `null` | Hyperlinks changed flag |
| `company` | `string \| null` | `null` | Company name |
| `worksheetNames` | `Array<string>` | `[]` | Worksheet names |

---

#### XmlExtractionResult

XML extraction result.

Contains extracted text content from XML files along with
structural statistics about the XML document.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `string` | — | Extracted text content (XML structure filtered out) |
| `elementCount` | `number` | — | Total number of XML elements processed |
| `uniqueElements` | `Array<string>` | — | List of unique element names found (sorted) |

---

#### XmlMetadata

XML metadata extracted during XML parsing.

Provides statistics about XML document structure.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `elementCount` | `number` | — | Total number of XML elements processed |
| `uniqueElements` | `Array<string>` | `[]` | List of unique element tag names (sorted) |

---

#### YakeParams

YAKE-specific parameters.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `windowSize` | `number` | `2` | Window size for co-occurrence analysis (default: 2). Controls the context window for computing co-occurrence statistics. |

### Methods

#### default()

**Signature:**

```typescript
static default(): YakeParams
```

---

#### YearRange

Year range for bibliographic metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `min` | `number \| null` | `null` | Earliest (minimum) year in the range. |
| `max` | `number \| null` | `null` | Latest (maximum) year in the range. |
| `years` | `Array<number>` | `/* serde(default) */` | All individual years present in the collection. |

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

#### OutputFormat

Output format for extraction results.

Controls the format of the `content` field in `ExtractionResult`.
When set to `Markdown`, `Djot`, or `Html`, the output uses that format.
`Plain` returns the raw extracted text.
`Structured` returns JSON with full OCR element data including bounding
boxes and confidence scores.

| Value | Description |
|-------|-------------|
| `Plain` | Plain text content only (default) |
| `Markdown` | Markdown format |
| `Djot` | Djot markup format |
| `Html` | HTML format |
| `Json` | JSON tree format with heading-driven sections. |
| `Structured` | Structured JSON format with full OCR element metadata. |
| `Custom` | Custom renderer registered via the RendererRegistry. The string is the renderer name (e.g., "docx", "latex"). — Fields: `0`: `string` |

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
table regions. Wire format is snake_case in all serializers (JSON, TOML,
YAML).

| Value | Description |
|-------|-------------|
| `Tatr` | TATR (Table Transformer) -- default, 30MB, DETR-based row/column detection. |
| `SlanetWired` | SLANeXT wired variant -- 365MB, optimized for bordered tables. |
| `SlanetWireless` | SLANeXT wireless variant -- 365MB, optimized for borderless tables. |
| `SlanetPlus` | SLANet-plus -- 7.78MB, lightweight general-purpose. |
| `SlanetAuto` | Classifier-routed SLANeXT: auto-select wired/wireless per table. Uses PP-LCNet classifier (6.78MB) + both SLANeXT variants (730MB total). |
| `Disabled` | Disable table structure model inference entirely; use heuristic path only. |

---

#### NerBackendKind

NER backend selector.

| Value | Description |
|-------|-------------|
| `Onnx` | gline-rs ONNX inference. Requires `ner-onnx` feature. Models download lazily from HuggingFace via `model_download.hf_download`. |
| `Llm` | liter-llm zero-shot NER via structured-output prompts. Requires `ner-llm` feature. Useful when domain-specific categories outstrip the ONNX taxonomy. |

---

#### VlmFallbackPolicy

Policy controlling when VLM (Vision Language Model) OCR is used as a fallback.

This knob is syntactic sugar over the explicit `OcrPipelineConfig` stage
ordering. When `vlm_fallback` is set and `pipeline` is `null`, an equivalent
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
| `Disabled` | No VLM fallback (default). Behaves identically to the pre-policy single-backend mode. |
| `OnLowQuality` | Try the classical OCR backend first. If the quality score is below `quality_threshold`, send the page to the VLM. `quality_threshold` is in the `[0.0, 1.0]` range produced by `calculate_quality_score`. A value of `0.5` is a reasonable starting point; calibrate with the Stage 0 benchmark harness. — Fields: `qualityThreshold`: `number` |
| `Always` | Skip the classical OCR backend entirely. Every page is sent to the VLM. |

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
| `Text` | Generic whitespace- and punctuation-aware text splitter (default). |
| `Markdown` | Markdown-aware splitter that preserves heading and code-block boundaries. |
| `Yaml` | YAML-aware splitter that creates one chunk per top-level key. |
| `Semantic` | Topic-aware chunker that splits at embedding-based topic shifts. |

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
| `Tokenizer` | Size measured in tokens from a HuggingFace tokenizer. — Fields: `model`: `string`, `cacheDir`: `string` |

---

#### EmbeddingModelType

Embedding model types supported by Kreuzberg.

| Value | Description |
|-------|-------------|
| `Preset` | Use a preset model configuration (recommended) — Fields: `name`: `string` |
| `Custom` | Use a custom ONNX model from HuggingFace — Fields: `modelId`: `string`, `dimensions`: `number` |
| `Llm` | Provider-hosted embedding model via liter-llm. Uses the model specified in the nested `LlmConfig` (e.g., `"openai/text-embedding-3-small"`). — Fields: `llm`: `LlmConfig` |
| `Plugin` | In-process embedding backend registered via the plugin system. The caller registers an `EmbeddingBackend` once (e.g. a wrapper around an already-loaded `llama-cpp-python`, `sentence-transformers`, or tuned ONNX model), then references it by name in config. Kreuzberg calls back into the registered backend during chunking and standalone embed requests — no HuggingFace download, no ONNX Runtime requirement, no HTTP sidecar. When this variant is selected, only the following `EmbeddingConfig` fields apply: `normalize` (post-call L2 normalization) and `max_embed_duration_secs` (dispatcher timeout). Model-loading fields (`batch_size`, `cache_dir`, `show_download_progress`, `acceleration`) are ignored — the host owns the model lifecycle. Semantic chunking falls back to `ChunkingConfig.max_characters` when this variant is used, since there is no preset to look a chunk-size ceiling up against — size your context window via `max_characters` directly. See `register_embedding_backend`. — Fields: `name`: `string` |

---

#### WhisperModel

Supported Whisper model sizes.

These map to published ONNX exports on Hugging Face (onnx-community or
similar orgs). The actual filenames and repos are resolved inside the
transcription engine.

| Value | Description |
|-------|-------------|
| `Tiny` | ~39 MB, fastest, lowest quality. Good default for development and CI. |
| `Base` | ~74 MB, reasonable quality/speed tradeoff. |
| `Small` | ~244 MB, better accuracy. |
| `Medium` | ~769 MB, high quality (slower, more memory). |
| `LargeV3` | ~1550 MB, best quality (large-v3). Use only when latency is acceptable. |

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

#### ListType

Type of list detection.

| Value | Description |
|-------|-------------|
| `Bullet` | Bullet points (-, *, •, etc.) |
| `Numbered` | Numbered lists (1., 2., etc.) |
| `Lettered` | Lettered lists (a., b., A., B., etc.) |
| `Indented` | Indented items |

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

Intensity level for the token-reduction pipeline.

| Value | Description |
|-------|-------------|
| `Off` | No reduction applied; text is returned as-is. |
| `Light` | Remove only the most common stopwords. |
| `Moderate` | Balanced stopword removal and redundancy filtering. |
| `Aggressive` | Aggressive filtering; may remove less common content words. |
| `Maximum` | Maximum compression; prioritizes brevity over completeness. |

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
| `Paragraph` | Standard prose paragraph. |
| `Heading` | Section heading (level stored in `FormattedBlock.level`). |
| `Blockquote` | Block quotation container. |
| `CodeBlock` | Fenced or indented code block. |
| `ListItem` | Individual item within a list. |
| `OrderedList` | Numbered (ordered) list container. |
| `BulletList` | Unnumbered (bullet) list container. |
| `TaskList` | Task / checkbox list container. |
| `DefinitionList` | Definition list container. |
| `DefinitionTerm` | Term part of a definition list entry. |
| `DefinitionDescription` | Description / definition part of a definition list entry. |
| `Div` | Generic `div` container with optional attributes. |
| `Section` | Logical section container, often associated with a heading. |
| `ThematicBreak` | Horizontal rule / thematic break. |
| `RawBlock` | Raw content block in a specified format (e.g. HTML, LaTeX). |
| `MathDisplay` | Display-mode mathematical expression. |

---

#### InlineType

Types of inline elements in Djot.

| Value | Description |
|-------|-------------|
| `Text` | Plain text run. |
| `Strong` | Bold / strong emphasis. |
| `Emphasis` | Italic / regular emphasis. |
| `Highlight` | Highlighted text (marker pen). |
| `Subscript` | Subscript text. |
| `Superscript` | Superscript text. |
| `Insert` | Inserted text (tracked change). |
| `Delete` | Deleted text (tracked change). |
| `Code` | Inline code span. |
| `Link` | Hyperlink with URL. |
| `Image` | Inline image reference. |
| `Span` | Generic inline span with optional attributes. |
| `Math` | Inline mathematical expression. |
| `RawInline` | Raw inline content in a specified format. |
| `FootnoteRef` | Footnote reference marker. |
| `Symbol` | Named symbol or emoji shortcode. |

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
| `Title` | Document title. — Fields: `text`: `string` |
| `Heading` | Section heading with level (1-6). — Fields: `level`: `number`, `text`: `string` |
| `Paragraph` | Body text paragraph. — Fields: `text`: `string` |
| `List` | List container — children are `ListItem` nodes. — Fields: `ordered`: `boolean` |
| `ListItem` | Individual list item. — Fields: `text`: `string` |
| `Table` | Table with structured cell grid. — Fields: `grid`: `TableGrid` |
| `Image` | Image reference. — Fields: `description`: `string`, `imageIndex`: `number`, `src`: `string` |
| `Code` | Code block. — Fields: `text`: `string`, `language`: `string` |
| `Quote` | Block quote — container, children carry the quoted content. |
| `Formula` | Mathematical formula / equation. — Fields: `text`: `string` |
| `Footnote` | Footnote reference content. — Fields: `text`: `string` |
| `Group` | Logical grouping container (section, key-value area). `heading_level` + `heading_text` capture the section heading directly rather than relying on a first-child positional convention. — Fields: `label`: `string`, `headingLevel`: `number`, `headingText`: `string` |
| `PageBreak` | Page break marker. |
| `Slide` | Presentation slide container — children are the slide's content nodes. — Fields: `number`: `number`, `title`: `string` |
| `DefinitionList` | Definition list container — children are `DefinitionItem` nodes. |
| `DefinitionItem` | Individual definition list entry with term and definition. — Fields: `term`: `string`, `definition`: `string` |
| `Citation` | Citation or bibliographic reference. — Fields: `key`: `string`, `text`: `string` |
| `Admonition` | Admonition / callout container (note, warning, tip, etc.). Children carry the admonition body content. — Fields: `kind`: `string`, `title`: `string` |
| `RawBlock` | Raw block preserved verbatim from the source format. Used for content that cannot be mapped to a semantic node type (e.g. JSX in MDX, raw LaTeX in markdown, embedded HTML). — Fields: `format`: `string`, `content`: `string` |
| `MetadataBlock` | Structured metadata block (email headers, YAML frontmatter, etc.). — Fields: `entries`: `Array<Array<string>>` |

---

#### AnnotationKind

Types of inline text annotations.

| Value | Description |
|-------|-------------|
| `Bold` | Bold (strong) text formatting. |
| `Italic` | Italic (emphasis) text formatting. |
| `Underline` | Underlined text. |
| `Strikethrough` | Strikethrough text. |
| `Code` | Inline code span. |
| `Subscript` | Subscript text. |
| `Superscript` | Superscript text. |
| `Link` | Hyperlink annotation. — Fields: `url`: `string`, `title`: `string` |
| `Highlight` | Highlighted text (PDF highlights, HTML `<mark>`). |
| `Color` | Text color (CSS-compatible value, e.g. "#ff0000", "red"). — Fields: `value`: `string` |
| `FontSize` | Font size with units (e.g. "12pt", "1.2em", "16px"). — Fields: `value`: `string` |
| `Custom` | Extensible annotation for format-specific styling. — Fields: `name`: `string`, `value`: `string` |

---

#### EntityCategory

Standard entity categories produced by built-in NER backends.

The `Custom(String)` variant lets caller-supplied categories (e.g. LLM
schemas) flow through without losing fidelity to the consumer.

| Value | Description |
|-------|-------------|
| `Person` | A person's name. |
| `Organization` | A company, institution, or organisation name. |
| `Location` | A geographic location (city, country, address). |
| `Date` | A calendar date. |
| `Time` | A time of day or duration. |
| `Money` | A monetary amount with optional currency. |
| `Percent` | A percentage value. |
| `Email` | An email address. |
| `Phone` | A phone number. |
| `Url` | A URL or URI. |
| `Custom` | A caller-supplied custom category label. — Fields: `0`: `string` |

---

#### ExtractionMethod

How the extracted text was produced.

| Value | Description |
|-------|-------------|
| `Native` | Text extracted directly from the document's native format (no OCR). |
| `Ocr` | All text was obtained via OCR (e.g. scanned image-only PDF). |
| `Mixed` | Text came from a combination of native extraction and OCR. |

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

#### ImageKind

Heuristic classification of what an image likely depicts.

| Value | Description |
|-------|-------------|
| `Photograph` | Photographic image (natural scene, photograph) |
| `Diagram` | Technical or schematic diagram |
| `Chart` | Chart, graph, or plot |
| `Drawing` | Freehand or technical drawing |
| `TextBlock` | Text-heavy image (scanned text, document) |
| `Decoration` | Decorative element or border |
| `Logo` | Logo or brand mark |
| `Icon` | Small icon |
| `TileFragment` | Fragment of a larger tiled image (tile of a technical drawing) |
| `Mask` | Mask or transparency map |
| `PageRaster` | Full-page render produced during OCR preprocessing; used as a citation thumbnail. |
| `Unknown` | Could not classify with reasonable confidence |

---

#### ResultFormat

Result-shape selection for extraction results.

Distinct from `OutputFormat` (which controls rendering — Plain, Markdown,
HTML, etc.). `ResultFormat` controls the *shape* of the result: a unified content
blob vs. an element-based decomposition.

| Value | Description |
|-------|-------------|
| `Unified` | Unified format with all content in `content` field |
| `ElementBased` | Element-based format with semantic element extraction |

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
| `Pdf` | Metadata extracted from a PDF document. — Fields: `0`: `PdfMetadata` |
| `Docx` | Metadata extracted from a DOCX Word document. — Fields: `0`: `DocxMetadata` |
| `Excel` | Metadata extracted from an Excel spreadsheet. — Fields: `0`: `ExcelMetadata` |
| `Email` | Metadata extracted from an email message (EML/MSG). — Fields: `0`: `EmailMetadata` |
| `Pptx` | Metadata extracted from a PowerPoint presentation. — Fields: `0`: `PptxMetadata` |
| `Archive` | Metadata extracted from an archive (ZIP, TAR, 7Z, etc.). — Fields: `0`: `ArchiveMetadata` |
| `Image` | Metadata extracted from a raster or vector image. — Fields: `0`: `ImageMetadata` |
| `Xml` | Metadata extracted from an XML document. — Fields: `0`: `XmlMetadata` |
| `Text` | Metadata extracted from a plain-text file. — Fields: `0`: `TextMetadata` |
| `Html` | Metadata extracted from an HTML document. — Fields: `0`: `HtmlMetadata` |
| `Ocr` | Metadata produced by an OCR pipeline. — Fields: `0`: `OcrMetadata` |
| `Csv` | Metadata extracted from a CSV or TSV file. — Fields: `0`: `CsvMetadata` |
| `Bibtex` | Metadata extracted from a BibTeX bibliography file. — Fields: `0`: `BibtexMetadata` |
| `Citation` | Metadata extracted from a citation file (RIS, PubMed, EndNote). — Fields: `0`: `CitationMetadata` |
| `FictionBook` | Metadata extracted from a FictionBook (FB2) e-book. — Fields: `0`: `FictionBookMetadata` |
| `Dbf` | Metadata extracted from a dBASE (DBF) database file. — Fields: `0`: `DbfMetadata` |
| `Jats` | Metadata extracted from a JATS (Journal Article Tag Suite) XML file. — Fields: `0`: `JatsMetadata` |
| `Epub` | Metadata extracted from an EPUB e-book. — Fields: `0`: `EpubMetadata` |
| `Pst` | Metadata extracted from an Outlook PST archive. — Fields: `0`: `PstMetadata` |
| `Audio` | Metadata extracted from an audio or video file. — Fields: `0`: `AudioMetadata` |

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
| `Rectangle` | Axis-aligned bounding box (typical for Tesseract output). — Fields: `left`: `number`, `top`: `number`, `width`: `number`, `height`: `number` |
| `Quadrilateral` | 4-point quadrilateral for rotated/skewed text (PaddleOCR). Points are in clockwise order starting from top-left: `[top_left, top_right, bottom_right, bottom_left]` — Fields: `points`: `string` |

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

#### RedactionStrategy

Strategy applied when a PII match is rewritten.

| Value | Description |
|-------|-------------|
| `Mask` | Replace the matched span with a fixed mask token (default `"[REDACTED]"`). |
| `Hash` | Replace with a SHA-256 hash of the original value (truncated to 16 hex chars). Lets downstream consumers do equality joins without recovering the source. |
| `TokenReplace` | Replace with a per-category running token (`"[PERSON_1]"`, `"[PERSON_2]"`, …) so the same person referenced twice gets the same token within the document. |
| `Drop` | Delete the matched span entirely. |

---

#### PiiCategory

PII categories the pattern engine recognises.

| Value | Description |
|-------|-------------|
| `Email` | Email address (e.g. `user@example.com`). |
| `Phone` | Phone number in any common format. |
| `Ssn` | US Social Security Number. |
| `CreditCard` | Payment card number (Visa, Mastercard, Amex, etc.). |
| `PostalCode` | Postal / ZIP code. |
| `IpAddress` | IPv4 or IPv6 address. |
| `Iban` | International Bank Account Number. |
| `SwiftBic` | SWIFT / BIC bank identifier code. |
| `DateOfBirth` | Date of birth. |
| `Person` | Person name, surfaced by the optional NER backend. |
| `Organization` | Organization name, surfaced by the optional NER backend. |
| `Location` | Location, surfaced by the optional NER backend. |
| `Custom` | Caller-supplied custom category (e.g. internal employee IDs). Surfaced by the redaction engine when a hit comes from `RedactionConfig.custom_terms` or `RedactionConfig.custom_patterns`. The string is the label passed alongside the term/pattern. Use those fields rather than constructing `Custom` directly via the `categories` filter — the pattern engine cannot detect arbitrary text from a category name alone. — Fields: `0`: `string` |

---

#### DiffLine

A single line in a unified-diff hunk.

Defined here (rather than only in `crate.diff`) so `RevisionDelta` can
reference it unconditionally, without requiring the `diff` Cargo feature.
`crate.diff` re-exports this type verbatim.

| Value | Description |
|-------|-------------|
| `Context` | Unchanged context line. — Fields: `0`: `string` |
| `Added` | Line added in the "after" version. — Fields: `0`: `string` |
| `Removed` | Line removed from the "before" version. — Fields: `0`: `string` |

---

#### RevisionKind

Semantic classification of a tracked change.

| Value | Description |
|-------|-------------|
| `Insertion` | Text or content was inserted. |
| `Deletion` | Text or content was deleted. |
| `FormatChange` | Run-level formatting (font, size, colour, …) was changed. |
| `Comment` | A reviewer comment or annotation. |

---

#### RevisionAnchor

Best-effort document location for a revision.

| Value | Description |
|-------|-------------|
| `Paragraph` | Body paragraph, identified by its zero-based index in the document flow. — Fields: `index`: `number` |
| `TableCell` | Cell inside a table. — Fields: `row`: `number`, `col`: `number`, `tableIndex`: `number` |
| `Page` | Page, identified by its zero-based index. — Fields: `index`: `number` |
| `Slide` | Presentation slide, identified by its zero-based index. — Fields: `index`: `number` |
| `Sheet` | Spreadsheet cell or range, identified by sheet index and optional name. — Fields: `index`: `number`, `name`: `string` |

---

#### SummaryStrategy

Summarisation strategy.

| Value | Description |
|-------|-------------|
| `Extractive` | Pure-Rust extractive summary (TextRank over the chunk graph). Deterministic, fast, no external service required. |
| `Abstractive` | Abstractive summary produced by liter-llm. Requires `liter-llm` feature and a configured `LlmConfig`. Token usage is captured in `ExtractionResult.llm_usage`. |

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

#### RegionKind

Classification of a detected layout region that warrants VLM extraction.

Each variant maps to a specific prompt optimised for that content type.
The mapping is intentionally narrow — only region kinds for which VLM
extraction provides a clear quality benefit over classical suppression.

| Value | Description |
|-------|-------------|
| `Figure` | A figure, diagram, chart, or image region. VLM prompt: describe the diagram / chart, including axis labels, legend entries, and any embedded text. |
| `DenseTable` | A densely formatted or complex table that classical extraction garbles. VLM prompt: extract the table as GitHub-Flavoured Markdown. |
| `ComplexLayout` | A region whose layout the classical pipeline cannot handle (multi-column insets, heavily annotated forms, mixed text+diagram). VLM prompt: extract all text and structure as markdown, preserving reading order. |
| `Caption` | A standalone image to be captioned (not extracted as figure markdown). VLM prompt: produce a single-sentence alt-text-style caption suitable for accessibility tooling and downstream indexing. Used by the captioning post-processor to populate `ExtractedImage.caption`. |

---

#### KeywordAlgorithm

Keyword algorithm selection.

| Value | Description |
|-------|-------------|
| `Yake` | YAKE (Yet Another Keyword Extractor) - statistical approach |
| `Rake` | RAKE (Rapid Automatic Keyword Extraction) - co-occurrence based |

---

#### PsmMode

Page Segmentation Mode for Tesseract OCR.

| Value | Description |
|-------|-------------|
| `OsdOnly` | Orientation and script detection only. |
| `AutoOsd` | Automatic page segmentation with OSD. |
| `AutoOnly` | Automatic page segmentation without OSD or OCR. |
| `Auto` | Fully automatic page segmentation with no OSD (default). |
| `SingleColumn` | Assume a single column of text of variable sizes. |
| `SingleBlockVertical` | Assume a single uniform block of vertically aligned text. |
| `SingleBlock` | Assume a single uniform block of text. |
| `SingleLine` | Treat the image as a single text line. |
| `SingleWord` | Treat the image as a single word. |
| `CircleWord` | Treat the image as a single word in a circle. |
| `SingleChar` | Treat the image as a single character. |

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

Wire format is snake_case in all serializers (JSON, TOML, YAML).

| Value | Description |
|-------|-------------|
| `Caption` | Figure or table caption text. |
| `Footnote` | Footnote or endnote text. |
| `Formula` | Mathematical formula or equation. |
| `ListItem` | A single item in a bulleted or numbered list. |
| `PageFooter` | Running footer at the bottom of a page. |
| `PageHeader` | Running header at the top of a page. |
| `Picture` | Image, chart, or other graphical element. |
| `SectionHeader` | Section heading. |
| `Table` | Data table. |
| `Text` | Body text paragraph. |
| `Title` | Document or chapter title. |
| `DocumentIndex` | Table of contents or index. |
| `Code` | Source code block. |
| `CheckboxSelected` | Checkbox in selected state. |
| `CheckboxUnselected` | Checkbox in unselected state. |
| `Form` | Form field or form element. |
| `KeyValueRegion` | Key-value pair region (e.g. label + value in a form). |

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

Errors are thrown as plain `Error` objects with descriptive messages.

| Variant | Description |
|---------|-------------|
| `Io` | A file system or I/O operation failed. These errors always bubble up unchanged. |
| `Parsing` | Document parsing failed (e.g. corrupt file, unsupported format feature). |
| `Ocr` | An OCR engine returned an error or produced unusable output. |
| `Validation` | Invalid configuration or input parameters were supplied. |
| `Cache` | A cache read or write operation failed. |
| `ImageProcessing` | An image manipulation operation (resize, decode, DPI conversion) failed. |
| `Serialization` | JSON or MessagePack serialization/deserialization failed. |
| `MissingDependency` | A required optional system dependency (e.g. `tesseract`) was not found. |
| `Plugin` | A registered plugin returned an error during extraction. |
| `LockPoisoned` | An internal `Mutex` or `RwLock` was found in a poisoned state. |
| `UnsupportedFormat` | The document's MIME type is not supported by any registered extractor. |
| `Embedding` | The embedding model or embedding pipeline returned an error. |
| `Transcription` | Audio/video transcription failed. |
| `Timeout` | The extraction operation exceeded the configured time limit. |
| `Cancelled` | The extraction was cancelled via a `CancellationToken`. |
| `Security` | A security policy was violated (e.g. zip bomb, oversized archive). |
| `Other` | A catch-all for uncommon errors that do not fit another variant. |

---

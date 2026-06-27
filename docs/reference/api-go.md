---
title: "Go API Reference"
---

## Go API Reference <span class="version-badge">v1.0.0-rc.1</span>

### Functions

#### Extract()

Extract content from a single bytes or URI input.

**Signature:**

```go
func Extract(input ExtractInput, config ExtractionConfig) (ExtractionOutput, error)
```

**Example:**

```go
result, err := Extract(ExtractInput{}, ExtractionConfig{})
if err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Input` | `ExtractInput` | Yes | The input data |
| `Config` | `ExtractionConfig` | Yes | The configuration options |

**Returns:** `ExtractionOutput`

**Errors:** Returns `error`.

---

#### ExtractBatch()

Extract content from multiple bytes or URI inputs.

**Signature:**

```go
func ExtractBatch(inputs []ExtractInput, config ExtractionConfig) (ExtractionOutput, error)
```

**Example:**

```go
result, err := ExtractBatch(nil, ExtractionConfig{})
if err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Inputs` | `\[\]ExtractInput` | Yes | The inputs |
| `Config` | `ExtractionConfig` | Yes | The configuration options |

**Returns:** `ExtractionOutput`

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

Returns `XbergError.UnsupportedFormat` if MIME type cannot be determined.

**Signature:**

```go
func DetectMimeTypeFromBytes(content []byte) (string, error)
```

**Example:**

```go
result, err := DetectMimeTypeFromBytes([]byte("data"))
if err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Content` | `\[\]byte` | Yes | Raw file bytes |

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

**Example:**

```go
result, err := GetExtensionsForMime("value")
if err != nil {
    return err
}
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

Returns every file extension Xberg recognizes together with its
corresponding MIME type, derived from the central format registry.
Formats that have no registered file extension (such as source code,
which is detected dynamically) are not included.

The list is sorted alphabetically by file extension.

**Returns:**

A vector of `SupportedFormat` entries sorted by extension.

**Signature:**

```go
func ListSupportedFormats() []SupportedFormat
```

**Example:**

```go
result := ListSupportedFormats()
```

**Returns:** `[]SupportedFormat`

---

#### DetectQrCodes()

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

```go
func DetectQrCodes(imageBytes []byte, formatHint string) []QrCode
```

**Example:**

```go
result := DetectQrCodes([]byte("data"), "value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `ImageBytes` | `\[\]byte` | Yes | The image bytes |
| `FormatHint` | `*string` | No | The  format hint |

**Returns:** `[]QrCode`

---

#### ClearEmbeddingBackends()

Clear all embedding backends from the global registry.

Calls `shutdown()` on every registered backend, then empties the registry.

**Errors:**

- Any error returned by a backend's `shutdown()` method. The first error
  encountered stops processing of remaining backends.

**Signature:**

```go
func ClearEmbeddingBackends() error
```

**Example:**

```go
if err := ClearEmbeddingBackends(); err != nil {
    return err
}
```

**Returns:** No return value.

**Errors:** Returns `error`.

---

#### ListEmbeddingBackends()

List the names of all registered embedding backends.

Used by `xberg-cli`, the api/mcp endpoints, and generated language
bindings.

**Signature:**

```go
func ListEmbeddingBackends() ([]string, error)
```

**Example:**

```go
result, err := ListEmbeddingBackends()
if err != nil {
    return err
}
```

**Returns:** `[]string`

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

**Example:**

```go
result, err := ListOcrBackends()
if err != nil {
    return err
}
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

**Example:**

```go
if err := ClearOcrBackends(); err != nil {
    return err
}
```

**Returns:** No return value.

**Errors:** Returns `error`.

---

#### RegisterBuiltin()

Register every built-in post-processor enabled by the active feature set.

This is the single entry point that callers (including
`register_default_post_processors`) use to populate the global
post-processor registry with the in-tree built-ins. Each submodule's own
`register` function is gated by its feature flag so this aggregate stays
safe to call on any target.

**Signature:**

```go
func RegisterBuiltin() error
```

**Example:**

```go
if err := RegisterBuiltin(); err != nil {
    return err
}
```

**Returns:** No return value.

**Errors:** Returns `error`.

---

#### ListPostProcessors()

List all registered post-processor names.

Returns a vector of all post-processor names currently registered in the
global registry.

**Returns:**

- `Ok([]string)` - Vector of post-processor names
- `Err(...)` if the registry lock is poisoned

**Signature:**

```go
func ListPostProcessors() ([]string, error)
```

**Example:**

```go
result, err := ListPostProcessors()
if err != nil {
    return err
}
```

**Returns:** `[]string`

**Errors:** Returns `error`.

---

#### ClearPostProcessors()

Remove all registered post-processors.

**Signature:**

```go
func ClearPostProcessors() error
```

**Example:**

```go
if err := ClearPostProcessors(); err != nil {
    return err
}
```

**Returns:** No return value.

**Errors:** Returns `error`.

---

#### ListRenderers()

List names of all registered renderers.

**Errors:**

Returns an error if the registry lock is poisoned.

**Signature:**

```go
func ListRenderers() ([]string, error)
```

**Example:**

```go
result, err := ListRenderers()
if err != nil {
    return err
}
```

**Returns:** `[]string`

**Errors:** Returns `error`.

---

#### ClearRenderers()

Clear all renderers from the global registry.

Removes every renderer, including the built-in defaults (markdown, html,
djot, plain). After calling this no renderers are registered; re-register
as needed.

**Errors:**

Returns an error if the registry lock is poisoned.

**Signature:**

```go
func ClearRenderers() error
```

**Example:**

```go
if err := ClearRenderers(); err != nil {
    return err
}
```

**Returns:** No return value.

**Errors:** Returns `error`.

---

#### ClearRerankerBackends()

Clear all reranker backends from the global registry.

Calls `shutdown()` on every registered backend, then empties the registry.

**Errors:**

- Any error returned by a backend's `shutdown()` method. The first error
  encountered stops processing of remaining backends.

Since v5.0.

**Signature:**

```go
func ClearRerankerBackends() error
```

**Example:**

```go
if err := ClearRerankerBackends(); err != nil {
    return err
}
```

**Returns:** No return value.

**Errors:** Returns `error`.

---

#### ListRerankerBackends()

List the names of all registered reranker backends.

Used by `xberg-cli`, the api/mcp endpoints, and generated language
bindings.

Since v5.0.

**Signature:**

```go
func ListRerankerBackends() ([]string, error)
```

**Example:**

```go
result, err := ListRerankerBackends()
if err != nil {
    return err
}
```

**Returns:** `[]string`

**Errors:** Returns `error`.

---

#### ListValidators()

List names of all registered validators.

**Signature:**

```go
func ListValidators() ([]string, error)
```

**Example:**

```go
result, err := ListValidators()
if err != nil {
    return err
}
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

**Example:**

```go
if err := ClearValidators(); err != nil {
    return err
}
```

**Returns:** No return value.

**Errors:** Returns `error`.

---

#### ClassifyPages()

Run page classification against an extraction result.

Mutates `result.page_classifications` with one entry per non-empty page and
appends every LLM call's usage to `result.llm_usage`.

**Errors:**

Returns the first error encountered when rendering the prompt or calling the
LLM. Partially produced classifications are discarded so callers do not see
a half-populated vector.

**Signature:**

```go
func ClassifyPages(result ExtractionResult, config PageClassificationConfig) error
```

**Example:**

```go
if err := ClassifyPages(ExtractionResult{}, PageClassificationConfig{}); err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Result` | `ExtractionResult` | Yes | The extraction result |
| `Config` | `PageClassificationConfig` | Yes | The configuration options |

**Returns:** No return value.

**Errors:** Returns `error`.

---

#### ClassifyText()

Classify a single piece of text without requiring an `ExtractionResult`.

Use this when the caller already has plain text (e.g. a RAG ingest pipeline
receiving documents off a queue) and wants a label list back without
manufacturing extractor-side metadata.

**Errors:**

Same as `classify_pages`: a validation error when `config.labels` is empty,
or any error returned by prompt rendering or the underlying LLM call.

**Signature:**

```go
func ClassifyText(text string, config PageClassificationConfig) ([]ClassificationLabel, error)
```

**Example:**

```go
result, err := ClassifyText("value", PageClassificationConfig{})
if err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Text` | `string` | Yes | The text |
| `Config` | `PageClassificationConfig` | Yes | The configuration options |

**Returns:** `[]ClassificationLabel`

**Errors:** Returns `error`.

---

#### ClassifyDocument()

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

```go
func ClassifyDocument(pages []string, config PageClassificationConfig) ([]ClassificationLabel, error)
```

**Example:**

```go
result, err := ClassifyDocument(nil, PageClassificationConfig{})
if err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Pages` | `\[\]string` | Yes | Slice of page texts to classify. Each page is classified independently |
| `Config` | `PageClassificationConfig` | Yes | Classification configuration including labels and LLM settings. |

**Returns:** `[]ClassificationLabel`

**Errors:** Returns `error`.

---

#### DownloadModel()

Eagerly download a NER model into the xberg cache.

`name` is a supported xberg GLiNER alias or catalog id. The CLI flag
`xberg cache warm --ner` delegates here.

**Signature:**

```go
func DownloadModel(name string, cacheDir string) (string, error)
```

**Example:**

```go
result, err := DownloadModel("value", "value")
if err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Name` | `string` | Yes | The name |
| `CacheDir` | `*string` | No | The cache dir |

**Returns:** `string`

**Errors:** Returns `error`.

---

#### DownloadModel()

**Signature:**

```go
func DownloadModel(name string, cacheDir string) (string, error)
```

**Example:**

```go
result, err := DownloadModel("value", "value")
if err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Name` | `string` | Yes | The  name |
| `CacheDir` | `*string` | No | The  cache dir |

**Returns:** `string`

**Errors:** Returns `error`.

---

#### DefaultModelName()

Pinned default NER model identifier.

**Signature:**

```go
func DefaultModelName() string
```

**Example:**

```go
result := DefaultModelName()
```

**Returns:** `string`

---

#### DefaultModelName()

**Signature:**

```go
func DefaultModelName() string
```

**Example:**

```go
result := DefaultModelName()
```

**Returns:** `string`

---

#### KnownModels()

All NER models xberg knows about (used by `--all-ner-models`).

**Signature:**

```go
func KnownModels() []string
```

**Example:**

```go
result := KnownModels()
```

**Returns:** `[]string`

---

#### KnownModels()

**Signature:**

```go
func KnownModels() []string
```

**Example:**

```go
result := KnownModels()
```

**Returns:** `[]string`

---

#### DownloadModel()

Download a NER model into the xberg cache.

**Signature:**

```go
func DownloadModel(name string, cacheDir string) (string, error)
```

**Example:**

```go
result, err := DownloadModel("value", "value")
if err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Name` | `string` | Yes | The  name |
| `CacheDir` | `*string` | No | The  cache dir |

**Returns:** `string`

**Errors:** Returns `error`.

---

#### DefaultModelName()

Default NER model identifier.

**Signature:**

```go
func DefaultModelName() string
```

**Example:**

```go
result := DefaultModelName()
```

**Returns:** `string`

---

#### KnownModels()

All NER models xberg knows about.

**Signature:**

```go
func KnownModels() []string
```

**Example:**

```go
result := KnownModels()
```

**Returns:** `[]string`

---

#### Redact()

Run pattern redaction (and optional NER-driven redaction) over `result` and
rewrite every textual field. Populates `result.redaction_report`.

**Signature:**

```go
func Redact(result ExtractionResult, config RedactionConfig) error
```

**Example:**

```go
if err := Redact(ExtractionResult{}, RedactionConfig{}); err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Result` | `ExtractionResult` | Yes | The extraction result |
| `Config` | `RedactionConfig` | Yes | The configuration options |

**Returns:** No return value.

**Errors:** Returns `error`.

---

#### Summarize()

Score and return the top-N sentences from `text`, joined in original order.

`language` is an ISO 639 (or locale) code used to pick a stopword list;
pass `nil` (or an unknown code) to fall back to English.
`max_tokens` bounds the summary length by whitespace-separated tokens;
`nil` falls back to `DEFAULT_MAX_TOKENS`.

**Signature:**

```go
func Summarize(text string, language string, maxTokens uint32) *string
```

**Example:**

```go
result := Summarize("value", "value", 42)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Text` | `string` | Yes | The text |
| `Language` | `*string` | No | The language |
| `MaxTokens` | `*uint32` | No | The max tokens |

**Returns:** `*string`

---

#### TokenCount()

Count whitespace-separated tokens (used for token-budget bookkeeping by
callers).

**Signature:**

```go
func TokenCount(text string) uint32
```

**Example:**

```go
result := TokenCount("value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Text` | `string` | Yes | The text |

**Returns:** `uint32`

---

#### TranslateResult()

Translate the extraction result in place.

Populates `result.translation` with the translated `content`, optionally the
translated `formatted_content` (when `preserve_markup = true`), and rewrites
every chunk's `content` field. Every LLM call's usage is appended to
`result.llm_usage`.

**Signature:**

```go
func TranslateResult(result ExtractionResult, config TranslationConfig) error
```

**Example:**

```go
if err := TranslateResult(ExtractionResult{}, TranslationConfig{}); err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Result` | `ExtractionResult` | Yes | The extraction result |
| `Config` | `TranslationConfig` | Yes | The configuration options |

**Returns:** No return value.

**Errors:** Returns `error`.

---

#### FindFootnoteAnchors()

Find all footnote anchor references in markdown text.

Returns a vector of footnote anchors (`[^label]` use-sites), including byte offsets.
Footnote definitions (`[^label]: ...`) are NOT included in the results.

**Returns:**

A vector of `FootnoteAnchor` entries, each with the label and byte offset.

**Signature:**

```go
func FindFootnoteAnchors(markdown string) []FootnoteAnchor
```

**Example:**

```go
result := FindFootnoteAnchors("value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Markdown` | `string` | Yes | The markdown text to search |

**Returns:** `[]FootnoteAnchor`

---

#### ParseFootnoteDefinitions()

Parse footnote definitions from markdown text.

Returns a vector of footnote definitions found in the markdown.
Handles multi-line definitions with continuation/indented lines (CommonMark format).

**Returns:**

A vector of `FootnoteDefinition` entries, each with label, content, and byte offset.

**Signature:**

```go
func ParseFootnoteDefinitions(markdown string) []FootnoteDefinition
```

**Example:**

```go
result := ParseFootnoteDefinitions("value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Markdown` | `string` | Yes | The markdown text to search |

**Returns:** `[]FootnoteDefinition`

---

#### FindInferenceMarkers()

Find inference markers in markdown text.

Returns byte offsets of every `[*inference*]` marker found in the text.

**Returns:**

A vector of byte offsets where inference markers appear.

**Signature:**

```go
func FindInferenceMarkers(markdown string) []int
```

**Example:**

```go
result := FindInferenceMarkers("value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Markdown` | `string` | Yes | The markdown text to search |

**Returns:** `[]int`

---

#### FindUnmarkedClaims()

Find unmarked claims in markdown text.

Returns lines that assert a claim but carry neither a footnote citation anchor (`[^...]`)
nor an inference marker (`[*inference*]`).

The heuristic is simple: a line that contains alphabetic words, ends with sentence punctuation,
and is not a heading, blank line, or markup-only line is considered a claim.
Exclude lines that appear in the citation block (after `---` + `<!-- citations ... -->`).

**Returns:**

A vector of trimmed line text strings for unmarked claims.

**Signature:**

```go
func FindUnmarkedClaims(markdown string) []string
```

**Example:**

```go
result := FindUnmarkedClaims("value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Markdown` | `string` | Yes | The markdown text to search |

**Returns:** `[]string`

---

#### ParseCitations()

Parse the structured citation block from markdown.

Extracts citations from the block after a `---` thematic break followed by
`<!-- citations ... -->` comment. Parses each entry as:
`[^srcN]: <source>, <optional-locator>, excerpt: "<text>"`

Returns parsed citations with source, optional locator, and optional excerpt.

**Returns:**

A vector of `Citation` entries parsed from the citation block.

**Signature:**

```go
func ParseCitations(markdown string) []Citation
```

**Example:**

```go
result := ParseCitations("value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Markdown` | `string` | Yes | The markdown text to search |

**Returns:** `[]Citation`

---

#### VerifyExcerpt()

Verify that an excerpt appears verbatim in source text.

Performs exact matching by default. Also tries whitespace-normalized matching
(collapsing runs of whitespace on both sides) since PDF-extracted text often
has irregular spacing.

**Returns:**

`true` if the excerpt appears (exactly or with normalized whitespace), `false` otherwise.

**Signature:**

```go
func VerifyExcerpt(excerpt string, sourceText string) bool
```

**Example:**

```go
result := VerifyExcerpt("value", "value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Excerpt` | `string` | Yes | The text snippet to find |
| `SourceText` | `string` | Yes | The full source text to search |

**Returns:** `bool`

---

#### ChunkForRag()

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

```go
func ChunkForRag(text string, config ChunkingConfig) (ChunkingResult, error)
```

**Example:**

```go
result, err := ChunkForRag("value", ChunkingConfig{})
if err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Text` | `string` | Yes | The text |
| `Config` | `ChunkingConfig` | Yes | The configuration options |

**Returns:** `ChunkingResult`

**Errors:** Returns `error`.

---

#### Compare()

Compare two extraction results and return a structured diff.

The comparison is purely structural — no I/O, no side effects. All fields
of `ExtractionDiff` are populated according to the provided `DiffOptions`.

**Signature:**

```go
func Compare(a ExtractionResult, b ExtractionResult, opts DiffOptions) ExtractionDiff
```

**Example:**

```go
result := Compare(ExtractionResult{}, ExtractionResult{}, DiffOptions{})
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `A` | `ExtractionResult` | Yes | The extraction result |
| `B` | `ExtractionResult` | Yes | The extraction result |
| `Opts` | `DiffOptions` | Yes | The options to use |

**Returns:** `ExtractionDiff`

---

#### ExtractRegionWithVlm()

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

```go
func ExtractRegionWithVlm(imageBytes []byte, imageMime string, regionKind RegionKind, llmConfig LlmConfig, customPrompt string) (string, error)
```

**Example:**

```go
result, err := ExtractRegionWithVlm([]byte("data"), "value", RegionKind{}, LlmConfig{}, "value")
if err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `ImageBytes` | `\[\]byte` | Yes | The image bytes |
| `ImageMime` | `string` | Yes | The image mime |
| `RegionKind` | `RegionKind` | Yes | The region kind |
| `LlmConfig` | `LlmConfig` | Yes | The llm config |
| `CustomPrompt` | `*string` | No | The custom prompt |

**Returns:** `string`

**Errors:** Returns `error`.

---

#### RerankAsync()

Rerank documents asynchronously.

Async counterpart to `rerank`. Offloads blocking ONNX inference to a
dedicated blocking thread pool via Tokio's `spawn_blocking`, keeping the
async executor free.

Since v5.0.

**Signature:**

```go
func RerankAsync(query string, documents []string, config RerankerConfig) ([]RerankedDocument, error)
```

**Example:**

```go
result, err := RerankAsync("value", nil, RerankerConfig{})
if err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Query` | `string` | Yes | The query |
| `Documents` | `\[\]string` | Yes | The documents |
| `Config` | `RerankerConfig` | Yes | The configuration options |

**Returns:** `[]RerankedDocument`

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

**Example:**

```go
result, err := ExtractKeywords("value", KeywordConfig{})
if err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Text` | `string` | Yes | The text to extract keywords from |
| `Config` | `KeywordConfig` | Yes | Keyword extraction configuration |

**Returns:** `[]Keyword`

**Errors:** Returns `error`.

---

#### AnalyzeDocument()

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

```go
func AnalyzeDocument(metadata DocumentMetadata, config HeuristicsConfig, documentBytes []byte) (ChunkingDecision, error)
```

**Example:**

```go
result, err := AnalyzeDocument(DocumentMetadata{}, HeuristicsConfig{}, []byte("data"))
if err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Metadata` | `DocumentMetadata` | Yes | The document metadata |
| `Config` | `HeuristicsConfig` | Yes | The configuration options |
| `DocumentBytes` | `*\[\]byte` | No | The document bytes |

**Returns:** `ChunkingDecision`

**Errors:** Returns `error`.

---

#### AnalyzeWithUserChunks()

Analyze a document with user-specified chunk ranges.

Creates a chunk plan based on user-provided page ranges.

**Signature:**

```go
func AnalyzeWithUserChunks(userRanges []PageRange, totalPages uint32, sizeBytes uint64, config HeuristicsConfig) ChunkingDecision
```

**Example:**

```go
result := AnalyzeWithUserChunks(nil, 42, 42, HeuristicsConfig{})
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `UserRanges` | `\[\]PageRange` | Yes | The user ranges |
| `TotalPages` | `uint32` | Yes | The total pages |
| `SizeBytes` | `uint64` | Yes | The size bytes |
| `Config` | `HeuristicsConfig` | Yes | The configuration options |

**Returns:** `ChunkingDecision`

---

#### ScoreConfidence()

Score a `ConfidenceSignals` triple into an `ExtractionConfidence` using
the supplied weights.

When `signals.ocr_aggregate` is `nil`, the OCR weight folds into
`text_coverage` so the weighted sum still totals 1.0.

**Signature:**

```go
func ScoreConfidence(signals ConfidenceSignals, weights ConfidenceWeights) ExtractionConfidence
```

**Example:**

```go
result := ScoreConfidence(ConfidenceSignals{}, ConfidenceWeights{})
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Signals` | `ConfidenceSignals` | Yes | The confidence signals |
| `Weights` | `ConfidenceWeights` | Yes | The confidence weights |

**Returns:** `ExtractionConfidence`

---

#### CheckFormatLimits()

Decision returned for pre-extraction rejection based on XLSX/PPTX-specific
resource bounds. Returns `Some(reason)` to reject; `nil` to proceed.

Callers must provide counts from a pre-extraction peek (e.g. parsing
`xl/workbook.xml` for sheet count).

**Signature:**

```go
func CheckFormatLimits(mimeType string, sheetCount uint32, workbookCells uint64, embeddedCount uint32, config HeuristicsConfig) *string
```

**Example:**

```go
result := CheckFormatLimits("value", 42, 42, 42, HeuristicsConfig{})
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `MimeType` | `string` | Yes | The mime type |
| `SheetCount` | `*uint32` | No | The sheet count |
| `WorkbookCells` | `*uint64` | No | The workbook cells |
| `EmbeddedCount` | `*uint32` | No | The embedded count |
| `Config` | `HeuristicsConfig` | Yes | The configuration options |

**Returns:** `*string`

---

#### BoundariesFromExtractionResult()

Derive document boundaries from an already-produced `ExtractionResult`.

Builds a `MultidocInput` from `result.pages` (one `PageSignals` per
`PageContent` entry), then delegates to `detect_boundaries`.

### Fallback behaviour

- If `result.pages` is `nil` or empty the whole document is treated as a
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

```go
func BoundariesFromExtractionResult(result ExtractionResult, thresholds MultidocThresholds) []DocumentBoundary
```

**Example:**

```go
result := BoundariesFromExtractionResult(ExtractionResult{}, MultidocThresholds{})
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Result` | `ExtractionResult` | Yes | The extraction result |
| `Thresholds` | `MultidocThresholds` | Yes | The multidoc thresholds |

**Returns:** `[]DocumentBoundary`

---

#### DetectBoundaries()

Detect document boundaries in a multi-document PDF.

Returns a list of detected boundaries, always including implicit boundaries
at start (page 1) and end (page_count).  Boundaries are returned in ascending
order of `start_page`.

**Returns:**

Ordered list of document boundaries.

**Signature:**

```go
func DetectBoundaries(input MultidocInput, thresholds MultidocThresholds) []DocumentBoundary
```

**Example:**

```go
result := DetectBoundaries(MultidocInput{}, MultidocThresholds{})
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Input` | `MultidocInput` | Yes | Page signals for the PDF |
| `Thresholds` | `MultidocThresholds` | Yes | Detection thresholds |

**Returns:** `[]DocumentBoundary`

---

#### ChooseCallMode()

Decide which call mode best fits this document.

Rules applied in order:

1. `image/*` → `StructuredCallMode.VisionOnly` (no text layer to start from).
2. `application/pdf` → `StructuredCallMode.TextOnly` regardless of
   `text_coverage` or embedded image count.  Xberg's OCR + text-layer
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

```go
func ChooseCallMode(input StructuredInput, t StructuredThresholds) StructuredCallMode
```

**Example:**

```go
result := ChooseCallMode(StructuredInput{}, StructuredThresholds{})
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Input` | `StructuredInput` | Yes | The input data |
| `T` | `StructuredThresholds` | Yes | The structured thresholds |

**Returns:** `StructuredCallMode`

---

#### CalculateChunkPlan()

Calculate a chunking plan for a document.

**Returns:**

A `ChunkPlan` with optimal chunk boundaries.

**Signature:**

```go
func CalculateChunkPlan(pageCount uint32, sizeBytes uint64, needsOcr bool, config HeuristicsConfig) ChunkPlan
```

**Example:**

```go
result := CalculateChunkPlan(42, 42, true, HeuristicsConfig{})
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `PageCount` | `uint32` | Yes | Total number of pages in the document |
| `SizeBytes` | `uint64` | Yes | File size in bytes |
| `NeedsOcr` | `bool` | Yes | Whether OCR will be required |
| `Config` | `HeuristicsConfig` | Yes | Heuristics configuration |

**Returns:** `ChunkPlan`

---

#### CalculatePlanFromOverrides()

Calculate a chunk plan from user-specified page ranges.

Validates and processes user overrides into a proper chunk plan.

**Signature:**

```go
func CalculatePlanFromOverrides(userChunks []PageRange, totalPages uint32, sizeBytes uint64, config HeuristicsConfig) ChunkPlan
```

**Example:**

```go
result := CalculatePlanFromOverrides(nil, 42, 42, HeuristicsConfig{})
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `UserChunks` | `\[\]PageRange` | Yes | The user chunks |
| `TotalPages` | `uint32` | Yes | The total pages |
| `SizeBytes` | `uint64` | Yes | The size bytes |
| `Config` | `HeuristicsConfig` | Yes | The configuration options |

**Returns:** `ChunkPlan`

---

#### Fingerprint()

Stable sha256 fingerprint of `raw`, formatted as `sha256:<hex>`.

**Signature:**

```go
func Fingerprint(raw []byte) string
```

**Example:**

```go
result := Fingerprint([]byte("data"))
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Raw` | `\[\]byte` | Yes | The raw |

**Returns:** `string`

---

#### Resolve()

Resolve `(preset, custom_schema_override, context)` into a `ResolvedPreset`.

- `custom_schema` overrides `preset.schema` when set.
- `context` substitutes `{{key}}` tokens in `preset.context_template`; the
  rendered string is appended to `system_prompt` so the model sees it.

**Signature:**

```go
func Resolve(preset Preset, customSchema interface{}, context map[string]string) (ResolvedPreset, error)
```

**Example:**

```go
result, err := Resolve(Preset{}, nil, nil)
if err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Preset` | `Preset` | Yes | The preset |
| `CustomSchema` | `*interface{}` | No | The custom schema |
| `Context` | `map\[string\]string` | Yes | The context |

**Returns:** `ResolvedPreset`

**Errors:** Returns `error`.

---

#### ExtractStructuredJson()

Extract structured JSON from a document using JSON-encoded preset spec and options.

This is the synchronous JSON-in / JSON-out entry point suitable for FFI and
language-binding call paths.

  `cache`).  Pass `"{}"` to use all defaults.

**Returns:**

JSON-serialised `StructuredOutput` on success.

**Errors:**

Returns `Validation` when either JSON argument is
malformed.  All other failures from the underlying
`extract_structured_sync` call are mapped onto `XbergError`
via `From<StructuredError>`.

**Signature:**

```go
func ExtractStructuredJson(bytes []byte, mime string, presetSpecJson string, optionsJson string) (string, error)
```

**Example:**

```go
result, err := ExtractStructuredJson([]byte("data"), "value", "value", "value")
if err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Bytes` | `\[\]byte` | Yes | The bytes |
| `Mime` | `string` | Yes | The mime |
| `PresetSpecJson` | `string` | Yes | The preset spec json |
| `OptionsJson` | `string` | Yes | The options json |

**Returns:** `string`

**Errors:** Returns `error`.

---

#### SplitAndExtractJson()

Split a multi-document PDF and extract structured JSON from each segment,
returning a JSON array of `StructuredOutput` objects.

Non-PDF documents are passed through as a single-element array.

Same as `extract_structured_json`.

**Returns:**

JSON-serialised `[]StructuredOutput` (a JSON array) on success.

**Errors:**

Returns `Validation` when either JSON argument is
malformed.  All other failures from the underlying
`split_and_extract_sync` call are mapped onto `XbergError`
via `From<StructuredError>`.

**Signature:**

```go
func SplitAndExtractJson(bytes []byte, mime string, presetSpecJson string, optionsJson string) (string, error)
```

**Example:**

```go
result, err := SplitAndExtractJson([]byte("data"), "value", "value", "value")
if err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Bytes` | `\[\]byte` | Yes | The bytes |
| `Mime` | `string` | Yes | The mime |
| `PresetSpecJson` | `string` | Yes | The preset spec json |
| `OptionsJson` | `string` | Yes | The options json |

**Returns:** `string`

**Errors:** Returns `error`.

---

#### RenderPdfPageToPng()

Render a single PDF page to PNG bytes.

Returns raw PNG-encoded bytes for the specified page at the given DPI.
Uses pdf_oxide with tiny-skia for pure-Rust rendering.

For pages with extreme dimensions (very wide vector diagrams, etc.) the
effective DPI may be automatically reduced to avoid rasterizer failure.
A warning is logged when this happens.

**Errors:**

Returns `XbergError.Parsing` if the PDF cannot be opened, authenticated,
or rendered, or if `page_index` is out of range.

**Signature:**

```go
func RenderPdfPageToPng(pdfBytes []byte, pageIndex int, dpi int32, password string) ([]byte, error)
```

**Example:**

```go
result, err := RenderPdfPageToPng([]byte("data"), 42, 42, "value")
if err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `PdfBytes` | `\[\]byte` | Yes | Raw PDF file bytes |
| `PageIndex` | `int` | Yes | Zero-based page index |
| `Dpi` | `*int32` | No | Resolution in dots per inch (default: 150) |
| `Password` | `*string` | No | Optional password for encrypted PDFs |

**Returns:** `[]byte`

**Errors:** Returns `error`.

---

#### PdfPageCount()

Count the pages in a PDF without rendering any of them.

Opens the document and returns its page count from the PDF structure. No page
is rasterized, so this is cheap relative to `render_pdf_page_to_png` — use it
when you only need the count (e.g. to drive a render loop over the pages).

**Errors:**

Returns `XbergError.Parsing` if the PDF cannot be opened, authenticated,
or its page count read.

**Signature:**

```go
func PdfPageCount(pdfBytes []byte, password string) (int, error)
```

**Example:**

```go
result, err := PdfPageCount([]byte("data"), "value")
if err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `PdfBytes` | `\[\]byte` | Yes | Raw PDF file bytes |
| `Password` | `*string` | No | Optional password for encrypted PDFs |

**Returns:** `int`

**Errors:** Returns `error`.

---

#### CaptionImage()

Caption a single image from bytes.

  `RegionKind.Caption` prompt when `nil`.

**Returns:**

The generated caption text.

**Errors:**

Returns an error if the VLM call fails or if image format detection fails.

**Signature:**

```go
func CaptionImage(imageBytes []byte, llmConfig LlmConfig, customPrompt string) (string, error)
```

**Example:**

```go
result, err := CaptionImage([]byte("data"), LlmConfig{}, "value")
if err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `ImageBytes` | `\[\]byte` | Yes | The image data. |
| `LlmConfig` | `LlmConfig` | Yes | LLM configuration for the VLM call. |
| `CustomPrompt` | `*string` | No | Optional custom caption prompt. Uses the default |

**Returns:** `string`

**Errors:** Returns `error`.

---

#### CaptionImageFile()

Caption a single image from a file path.

  `RegionKind.Caption` prompt when `nil`.

**Returns:**

The generated caption text.

**Errors:**

Returns an error if the file cannot be read, if image format detection fails,
or if the VLM call fails.

**Signature:**

```go
func CaptionImageFile(path string, llmConfig LlmConfig, customPrompt string) (string, error)
```

**Example:**

```go
result, err := CaptionImageFile("value", LlmConfig{}, "value")
if err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Path` | `string` | Yes | Path to the image file. |
| `LlmConfig` | `LlmConfig` | Yes | LLM configuration for the VLM call. |
| `CustomPrompt` | `*string` | No | Optional custom caption prompt. Uses the default |

**Returns:** `string`

**Errors:** Returns `error`.

---

#### DetectMimeType()

Detect the MIME type of a file at the given path.

Uses the file extension and optionally the file content to determine the MIME type.
Set `check_exists` to `true` to verify the file exists before detection.

**Signature:**

```go
func DetectMimeType(path string, checkExists bool) (string, error)
```

**Example:**

```go
result, err := DetectMimeType("value", true)
if err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Path` | `string` | Yes | Path to the file |
| `CheckExists` | `bool` | Yes | The check exists |

**Returns:** `string`

**Errors:** Returns `error`.

---

#### EmbedTextsAsync()

**Signature:**

```go
func EmbedTextsAsync(texts []string, config EmbeddingConfig) ([][]float32, error)
```

**Example:**

```go
result, err := EmbedTextsAsync(nil, EmbeddingConfig{})
if err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Texts` | `\[\]string` | Yes | The  texts |
| `Config` | `EmbeddingConfig` | Yes | The embedding config |

**Returns:** `[][]float32`

**Errors:** Returns `error`.

---

#### GetEmbeddingPreset()

Get an embedding preset by name.

Returns `nil` if no preset with the given name exists. Returns an owned
clone so the value is safe to pass across FFI boundaries.

**Signature:**

```go
func GetEmbeddingPreset(name string) *EmbeddingPreset
```

**Example:**

```go
result := GetEmbeddingPreset("value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Name` | `string` | Yes | The name |

**Returns:** `*EmbeddingPreset`

---

#### ListEmbeddingPresets()

List the names of all available embedding presets.

Returns owned `String`s so the values are safe to pass across FFI boundaries.

**Signature:**

```go
func ListEmbeddingPresets() []string
```

**Example:**

```go
result := ListEmbeddingPresets()
```

**Returns:** `[]string`

---

#### GetEmbeddingPreset()

Returns `nil` for builds without the `embedding-presets` feature.

**Signature:**

```go
func GetEmbeddingPreset(name string) *EmbeddingPreset
```

**Example:**

```go
result := GetEmbeddingPreset("value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Name` | `string` | Yes | The  name |

**Returns:** `*EmbeddingPreset`

---

#### ListEmbeddingPresets()

Returns an empty list for builds without the `embedding-presets` feature.

**Signature:**

```go
func ListEmbeddingPresets() []string
```

**Example:**

```go
result := ListEmbeddingPresets()
```

**Returns:** `[]string`

---

#### Rerank()

Rerank a list of documents by relevance to a query.

Returns documents sorted descending by score. Applies `top_k` truncation if
configured.

**Errors:**

- `XbergError.Validation` if `query` is empty or blank.
- `XbergError.MissingDependency` if ONNX Runtime is not installed (ONNX path).
- `XbergError.Reranking` if the preset is unknown or model download fails.

Since v5.0.

**Signature:**

```go
func Rerank(query string, documents []string, config RerankerConfig) ([]RerankedDocument, error)
```

**Example:**

```go
result, err := Rerank("value", nil, RerankerConfig{})
if err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Query` | `string` | Yes | The query |
| `Documents` | `\[\]string` | Yes | The documents |
| `Config` | `RerankerConfig` | Yes | The configuration options |

**Returns:** `[]RerankedDocument`

**Errors:** Returns `error`.

---

#### Rerank()

Stub for builds without the `reranker` feature — keeps the symbol available
on no-ORT targets (Android x86_64 emulator, WASM) so language bindings compile.

Since v5.0.

**Signature:**

```go
func Rerank(query string, documents []string, config RerankerConfig) ([]RerankedDocument, error)
```

**Example:**

```go
result, err := Rerank("value", nil, RerankerConfig{})
if err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Query` | `string` | Yes | The  query |
| `Documents` | `\[\]string` | Yes | The  documents |
| `Config` | `RerankerConfig` | Yes | The reranker config |

**Returns:** `[]RerankedDocument`

**Errors:** Returns `error`.

---

#### RerankAsync()

Stub for builds without the `reranker` feature.

Since v5.0.

**Signature:**

```go
func RerankAsync(query string, documents []string, config RerankerConfig) ([]RerankedDocument, error)
```

**Example:**

```go
result, err := RerankAsync("value", nil, RerankerConfig{})
if err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Query` | `string` | Yes | The  query |
| `Documents` | `\[\]string` | Yes | The  documents |
| `Config` | `RerankerConfig` | Yes | The reranker config |

**Returns:** `[]RerankedDocument`

**Errors:** Returns `error`.

---

#### GetRerankerPreset()

Get a reranker preset by name.

Returns `nil` if no preset with the given name exists. Returns an owned
clone so the value is safe to pass across FFI boundaries.

Since v5.0.

**Signature:**

```go
func GetRerankerPreset(name string) *RerankerPreset
```

**Example:**

```go
result := GetRerankerPreset("value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Name` | `string` | Yes | The name |

**Returns:** `*RerankerPreset`

---

#### ListRerankerPresets()

List the names of all available reranker presets.

Returns owned `String`s so the values are safe to pass across FFI boundaries.

Since v5.0.

**Signature:**

```go
func ListRerankerPresets() []string
```

**Example:**

```go
result := ListRerankerPresets()
```

**Returns:** `[]string`

---

#### GetRerankerPreset()

Returns `nil` for builds without the `reranker-presets` feature.

Since v5.0.

**Signature:**

```go
func GetRerankerPreset(name string) *RerankerPreset
```

**Example:**

```go
result := GetRerankerPreset("value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Name` | `string` | Yes | The  name |

**Returns:** `*RerankerPreset`

---

#### ListRerankerPresets()

Returns an empty list for builds without the `reranker-presets` feature.

Since v5.0.

**Signature:**

```go
func ListRerankerPresets() []string
```

**Example:**

```go
result := ListRerankerPresets()
```

**Returns:** `[]string`

---

#### EmbedTextsAsync()

**Signature:**

```go
func EmbedTextsAsync(texts []string, config EmbeddingConfig) ([][]float32, error)
```

**Example:**

```go
result, err := EmbedTextsAsync(nil, EmbeddingConfig{})
if err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Texts` | `\[\]string` | Yes | The  texts |
| `Config` | `EmbeddingConfig` | Yes | The embedding config |

**Returns:** `[][]float32`

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
| `FileCount` | `uint32` | — | Total number of files in the archive |
| `FileList` | `\[\]string` | `nil` | List of file paths within the archive |
| `TotalSize` | `uint64` | — | Total uncompressed size in bytes |
| `CompressedSize` | `*uint64` | `nil` | Compressed size in bytes (if available) |

---

#### AudioMetadata

Audio/video file metadata.

Populated from container tags (ID3v2, MP4 atoms, Vorbis comments, etc.) and
PCM decode properties. Available when the `transcription-types` feature is enabled.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `DurationMs` | `*uint64` | `nil` | Duration in milliseconds derived from the decoded audio stream. |
| `Codec` | `*string` | `nil` | Audio codec (e.g. "mp3", "aac", "opus", "flac"). |
| `Container` | `*string` | `nil` | Container format (e.g. "mpeg", "mp4", "ogg", "wav"). |
| `SampleRateHz` | `*uint32` | `nil` | Sample rate in Hz after decode (always 16000 when resampled for Whisper). |
| `Channels` | `*uint16` | `nil` | Number of audio channels (1 = mono, 2 = stereo). |
| `Bitrate` | `*uint32` | `nil` | Audio bitrate in kbps from the source file tags/properties. |

---

#### BBox

Bounding box in original image coordinates (x1, y1) top-left, (x2, y2) bottom-right.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `X1` | `float32` | — | Left edge (x-coordinate of the top-left corner). |
| `Y1` | `float32` | — | Top edge (y-coordinate of the top-left corner). |
| `X2` | `float32` | — | Right edge (x-coordinate of the bottom-right corner). |
| `Y2` | `float32` | — | Bottom edge (y-coordinate of the bottom-right corner). |

---

#### BibtexMetadata

BibTeX bibliography metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `EntryCount` | `int` | — | Number of entries in the bibliography. |
| `CitationKeys` | `\[\]string` | `nil` | BibTeX citation keys (e.g. `"knuth1984"`) for all entries. |
| `Authors` | `\[\]string` | `nil` | Author names collected across all bibliography entries. |
| `YearRange` | `*YearRange` | `nil` | Earliest and latest publication years found in the bibliography. |
| `EntryTypes` | `*map\[string\]int` | `nil` | Count of entries grouped by BibTeX entry type (e.g. `"article"` → 5). |

---

#### BoundingBox

Bounding box coordinates for element positioning.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `X0` | `float64` | — | Left x-coordinate |
| `Y0` | `float64` | — | Bottom y-coordinate |
| `X1` | `float64` | — | Right x-coordinate |
| `Y1` | `float64` | — | Top y-coordinate |

---

#### CacheStats

Aggregate statistics for a xberg cache directory.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `TotalFiles` | `int` | — | Total number of files currently in the cache directory. |
| `TotalSizeMb` | `float64` | — | Combined size of all cache files in megabytes. |
| `AvailableSpaceMb` | `float64` | — | Free disk space available on the cache volume, in megabytes. |
| `OldestFileAgeDays` | `float64` | — | Age of the oldest cache file in days (0.0 if the cache is empty). |
| `NewestFileAgeDays` | `float64` | — | Age of the most recently written cache file in days (0.0 if the cache is empty). |

---

#### CaptioningConfig

**Since:** `v5.0`

Configuration for the VLM captioning post-processor.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Llm` | `LlmConfig` | — | LLM configuration used for the VLM call. |
| `Prompt` | `*string` | `nil` | Optional custom caption prompt. `nil` uses the default `RegionKind.Caption` prompt that ships with `crate.llm.region_extractor`. |
| `MinImageArea` | `uint32` | `serde(default = "default_min_image_area")` | Skip images whose `width * height` is below this threshold (in pixels). Default `1_000` filters out icons and decorations. |

---

#### CaptioningEnrichmentConfig

Captioning enrichment knob: which LLM to use for image captions.

The enrichment stage calls `caption_image` for every
image in `ExtractionResult.images` that has non-empty `data`. Images with
empty byte data (e.g. reference-only images populated via `source_path`) are
skipped rather than forwarded to the VLM.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Config` | `LlmConfig` | — | LLM / VLM configuration forwarded verbatim to each `caption_image` call. |
| `CustomPrompt` | `*string` | `nil` | Optional custom prompt override forwarded to every `caption_image` call. `nil` uses the default `RegionKind.Caption` prompt. |

---

#### CellChange

A single changed cell within a table.

Defined here (rather than only in `crate.diff`) so `RevisionDelta` can
reference it unconditionally, without requiring the `diff` Cargo feature.
`crate.diff` re-exports this type verbatim.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Row` | `int` | — | Zero-based row index. |
| `Col` | `int` | — | Zero-based column index. |
| `From` | `string` | — | Value before the change. |
| `To` | `string` | — | Value after the change. |

---

#### Chunk

A text chunk with optional embedding and metadata.

Chunks are created when chunking is enabled in `ExtractionConfig`. Each chunk
contains the text content, optional embedding vector (if embedding generation
is configured), and metadata about its position in the document.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Content` | `string` | — | The text content of this chunk. |
| `ChunkType` | `ChunkType` | `/* serde(default) */` | Semantic structural classification of this chunk. Assigned by the heuristic classifier based on content patterns and heading context. Defaults to `ChunkType.Unknown` when no rule matches. |
| `Embedding` | `*\[\]float32` | `nil` | Optional embedding vector for this chunk. Only populated when `EmbeddingConfig` is provided in chunking configuration. The dimensionality depends on the chosen embedding model. |
| `Metadata` | `ChunkMetadata` | — | Metadata about this chunk's position and properties. |

---

#### ChunkInfo

Information about a single chunk.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Index` | `uint32` | — | Zero-based chunk index. |
| `Pages` | `PageRange` | — | Page range for this chunk. |
| `EstimatedTimeMs` | `uint64` | — | Estimated processing time for this chunk in milliseconds. |

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
| `FirstPage` | `*uint32` | `nil` | First page number this chunk spans (1-indexed). Only populated when page tracking is enabled in extraction configuration. |
| `LastPage` | `*uint32` | `nil` | Last page number this chunk spans (1-indexed, equal to first_page for single-page chunks). Only populated when page tracking is enabled in extraction configuration. |
| `HeadingContext` | `*HeadingContext` | `/* serde(default) */` | Heading context when using Markdown chunker. Contains the heading hierarchy this chunk falls under. Only populated when `ChunkerType.Markdown` is used. |
| `HeadingPath` | `\[\]string` | `/* serde(default) */` | Flattened heading trail from document root to this chunk's section. Each element is a heading's text, outermost first. Derived from `heading_context` when present; empty otherwise. Provides a binding-friendly, RAG-shaped breadcrumb without requiring callers to walk the nested `HeadingContext` structure. |
| `ImageIndices` | `\[\]uint32` | `/* serde(default) */` | Indices into `ExtractionResult.images` for images on pages covered by this chunk. Contains zero-based indices into the top-level `images` collection for every image whose `page_number` falls within `\[first_page, last_page\]`. Empty when image extraction is disabled or the chunk spans no pages with images. |

---

#### ChunkPlan

Complete chunking plan for a document.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `TotalChunks` | `uint32` | `0` | Total number of chunks. |
| `Chunks` | `\[\]ChunkInfo` | `nil` | Individual chunk information. |
| `TotalEstimatedTimeMs` | `uint64` | `0` | Estimated total processing time in milliseconds. |
| `UseDiskProcessing` | `bool` | `false` | Whether to use disk-based processing for large files. |
| `Reason` | `ChunkingReason` | `ChunkingReason.LargeFile` | Reason for chunking. |

##### Methods

###### Default()

An empty plan (no chunks). The `reason` is a placeholder since an empty plan
has no chunking rationale; callers always overwrite it when a real plan is built.

**Signature:**

```go
func (o *ChunkPlan) Default() ChunkPlan
```

**Example:**

```go
result := ChunkPlan.Default()
```

**Returns:** `ChunkPlan`

###### TotalPages()

Get the total number of pages across all chunks.

**Signature:**

```go
func (o *ChunkPlan) TotalPages() uint32
```

**Example:**

```go
result := instance.TotalPages()
```

**Returns:** `uint32`

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
| `TableChunking` | `TableChunkingMode` | `TableChunkingMode.Split` | How to handle markdown tables that exceed the chunk size limit. Only applies when `chunker_type` is `Markdown`. - `Split` (default) — tables are split at row boundaries; continuation chunks do not repeat the header. - `RepeatHeader` — the table header row and separator are prepended to every continuation chunk so each chunk is self-contained. Default: `Split` |

##### Methods

###### Default()

**Signature:**

```go
func (o *ChunkingConfig) Default() ChunkingConfig
```

**Example:**

```go
result := ChunkingConfig.Default()
```

**Returns:** `ChunkingConfig`

---

#### ChunkingResult

Result of a text chunking operation.

Contains the generated chunks and metadata about the chunking.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Chunks` | `\[\]Chunk` | — | List of text chunks |
| `ChunkCount` | `int` | — | Total number of chunks generated |

---

#### Citation

A structured citation from a citation block.

Parsed from entries like:
`[^srcN]: source, locator, excerpt: "text"`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Label` | `string` | — | The label of the citation (e.g., "src1" in `\[^src1\]: ...`). |
| `Source` | `string` | — | The source reference (path, URL, or identifier). |
| `Locator` | `*string` | `nil` | Optional locator within the source (e.g., "page 3" or "section 2.1"). |
| `Excerpt` | `*string` | `nil` | Optional excerpt — quoted text from the source. |

---

#### CitationMetadata

Citation file metadata (RIS, PubMed, EndNote).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `CitationCount` | `int` | — | Total number of citation records in the file. |
| `Format` | `*string` | `nil` | Detected citation file format (e.g. `"ris"`, `"pubmed"`, `"endnote"`). |
| `Authors` | `\[\]string` | `nil` | Author names collected across all citation records. |
| `YearRange` | `*YearRange` | `nil` | Earliest and latest publication years found in the file. |
| `Dois` | `\[\]string` | `nil` | DOI identifiers found in the citation records. |
| `Keywords` | `\[\]string` | `nil` | Keywords collected from all citation records. |

---

#### ClassificationEnrichmentConfig

Classification enrichment knob: how to label the document.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Config` | `PageClassificationConfig` | — | Label set and LLM settings for the classification stage. |

---

#### ClassificationLabel

A single label + confidence pair.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Label` | `string` | — | Label name as configured in `PageClassificationConfig.labels`. |
| `Confidence` | `*float32` | `nil` | Backend-reported confidence in `\[0.0, 1.0\]`. `nil` when the backend (e.g. an LLM prompt without explicit confidence schema) did not report one. |

---

#### ConfidenceSignals

Input signals for confidence scoring.

Caller fills these from the extraction result and the LLM response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `TextCoverage` | `float32` | — | Fraction of pages with usable text in `\[0, 1\]`. |
| `OcrAggregate` | `*float32` | `nil` | Mean OCR per-element recognition confidence; `nil` when OCR did not run. |
| `SchemaCompliance` | `SchemaCompliance` | — | Schema-validation result of the merged output. |

##### Methods

###### FromExtractionResult()

Build `ConfidenceSignals` from an `ExtractionResult`.

- `result` — The extraction result whose `ocr_elements` are inspected.
- `schema_compliance` — Caller-supplied schema validation outcome.
- `text_coverage` — Caller-supplied fraction of pages with usable text
  (e.g. 1.0 for native text formats, value from PDF analysis for PDFs).

The `ocr_aggregate` is computed as the arithmetic mean of all
`ocr_elements[].confidence.recognition` values.  When `ocr_elements` is
`nil` or empty the field is set to `nil`.

**Signature:**

```go
func (o *ConfidenceSignals) FromExtractionResult(result ExtractionResult, schemaCompliance SchemaCompliance, textCoverage float32) ConfidenceSignals
```

**Example:**

```go
result := ConfidenceSignals.FromExtractionResult(ExtractionResult{}, SchemaCompliance{}, 0.5)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Result` | `ExtractionResult` | Yes | The extraction result |
| `SchemaCompliance` | `SchemaCompliance` | Yes | The schema compliance |
| `TextCoverage` | `float32` | Yes | The text coverage |

**Returns:** `ConfidenceSignals`

---

#### ConfidenceWeights

Tunable weights for the confidence scoring formula.

Defaults picked by inspection; callers tune them via config.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `TextCoverage` | `float32` | `0.3` | Weight assigned to `text_coverage`. Default 0.30. |
| `OcrAggregate` | `float32` | `0.3` | Weight assigned to `ocr_aggregate` when OCR ran. Default 0.30 — folds into `text_coverage` weight when OCR did not run. |
| `SchemaCompliance` | `float32` | `0.4` | Weight assigned to `schema_compliance`. Default 0.40. |

##### Methods

###### Default()

**Signature:**

```go
func (o *ConfidenceWeights) Default() ConfidenceWeights
```

**Example:**

```go
result := ConfidenceWeights.Default()
```

**Returns:** `ConfidenceWeights`

###### IsNormalized()

Validate that weights sum to approximately 1.0.

**Signature:**

```go
func (o *ConfidenceWeights) IsNormalized() bool
```

**Example:**

```go
result := instance.IsNormalized()
```

**Returns:** `bool`

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
| `StripRepeatingText` | `bool` | `true` | Enable the heuristic cross-page repeating text detector. When `true` (default), text that repeats verbatim across a supermajority of pages is classified as furniture and stripped.  Disable this if brand names or repeated headings are being incorrectly removed by the heuristic. Note: when a layout-detection model is active, the model may independently classify page-header / page-footer regions as furniture on a per-page basis. To preserve those regions, set `include_headers = true`, `include_footers = true`, or both, in addition to disabling this flag. Primarily affects PDF extraction. Default: `true`. |
| `IncludeWatermarks` | `bool` | `false` | Include watermark text in extraction output. - PDF: Keeps watermark artifacts and arXiv identifiers. - Other formats: No effect currently. Default: `false` (watermarks are stripped). |

##### Methods

###### Default()

**Signature:**

```go
func (o *ContentFilterConfig) Default() ContentFilterConfig
```

**Example:**

```go
result := ContentFilterConfig.Default()
```

**Returns:** `ContentFilterConfig`

---

#### ContributorRole

JATS contributor with role.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Name` | `string` | — | Contributor display name. |
| `Role` | `*string` | `nil` | Contributor role (e.g. `"author"`, `"editor"`). |

---

#### CoreProperties

Dublin Core metadata from docProps/core.xml

Contains standard metadata fields defined by the Dublin Core standard
and Office-specific extensions.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Title` | `*string` | `nil` | Document title |
| `Subject` | `*string` | `nil` | Document subject/topic |
| `Creator` | `*string` | `nil` | Document creator/author |
| `Keywords` | `*string` | `nil` | Keywords or tags |
| `Description` | `*string` | `nil` | Document description/abstract |
| `LastModifiedBy` | `*string` | `nil` | User who last modified the document |
| `Revision` | `*string` | `nil` | Revision number |
| `Created` | `*string` | `nil` | Creation timestamp (ISO 8601) |
| `Modified` | `*string` | `nil` | Last modification timestamp (ISO 8601) |
| `Category` | `*string` | `nil` | Document category |
| `ContentStatus` | `*string` | `nil` | Content status (Draft, Final, etc.) |
| `Language` | `*string` | `nil` | Document language |
| `Identifier` | `*string` | `nil` | Unique identifier |
| `Version` | `*string` | `nil` | Document version |
| `LastPrinted` | `*string` | `nil` | Last print timestamp (ISO 8601) |

---

#### CsvMetadata

CSV/TSV file metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `RowCount` | `uint32` | — | Total number of data rows (excluding the header row if present). |
| `ColumnCount` | `uint32` | — | Number of columns detected. |
| `Delimiter` | `*string` | `nil` | Field delimiter character (e.g. `","` or `"\t"`). |
| `HasHeader` | `bool` | — | Whether the first row was treated as a header. |
| `ColumnTypes` | `*\[\]string` | `nil` | Inferred data type for each column (e.g. `"string"`, `"integer"`, `"float"`). |

---

#### DbfFieldInfo

dBASE field information.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Name` | `string` | — | Field (column) name. |
| `FieldType` | `string` | — | dBASE field type character (e.g. `"C"` for character, `"N"` for numeric). |

---

#### DbfMetadata

dBASE (DBF) file metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `RecordCount` | `int` | — | Total number of data records in the DBF file. |
| `FieldCount` | `int` | — | Number of field (column) definitions. |
| `Fields` | `\[\]DbfFieldInfo` | `nil` | Descriptor for each field in the table schema. |

---

#### DetectResponse

MIME type detection response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `MimeType` | `string` | — | Detected MIME type |
| `Filename` | `*string` | `nil` | Original filename (if provided) |

---

#### DetectionResult

Page-level detection result containing all detections and page metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `PageWidth` | `uint32` | — | Page width in pixels (as seen by the model). |
| `PageHeight` | `uint32` | — | Page height in pixels (as seen by the model). |
| `Detections` | `\[\]LayoutDetection` | — | All layout detections on this page after postprocessing. |

---

#### DiffHunk

A single contiguous hunk in a unified diff.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `FromLine` | `int` | — | Starting line number in the old content (0-indexed). |
| `FromCount` | `int` | — | Number of lines from the old content in this hunk. |
| `ToLine` | `int` | — | Starting line number in the new content (0-indexed). |
| `ToCount` | `int` | — | Number of lines from the new content in this hunk. |
| `Lines` | `\[\]DiffLine` | — | Lines that make up this hunk. |

---

#### DiffOptions

Options controlling how two `ExtractionResult` values are compared.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `IncludeMetadata` | `bool` | `true` | Include metadata changes in the diff. Default: `true`. |
| `IncludeEmbedded` | `bool` | `true` | Include embedded-children changes in the diff. Default: `true`. |
| `MaxContentChars` | `*int` | `nil` | Truncate content to this many characters before diffing. Useful for very large documents where only the first N characters matter. `nil` means no truncation. |

##### Methods

###### Default()

**Signature:**

```go
func (o *DiffOptions) Default() DiffOptions
```

**Example:**

```go
result := DiffOptions.Default()
```

**Returns:** `DiffOptions`

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
| `Blocks` | `\[\]FormattedBlock` | — | Structured block-level content |
| `Metadata` | `Metadata` | — | Metadata from YAML frontmatter |
| `Tables` | `\[\]Table` | — | Extracted tables as structured data |
| `Images` | `\[\]DjotImage` | — | Extracted images with metadata |
| `Links` | `\[\]DjotLink` | — | Extracted links with URLs |
| `Footnotes` | `\[\]Footnote` | — | Footnote definitions |

---

#### DjotImage

Image element in Djot.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Src` | `string` | — | Image source URL or path |
| `Alt` | `string` | — | Alternative text |
| `Title` | `*string` | `nil` | Optional title |

---

#### DjotLink

Link element in Djot.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Url` | `string` | — | Link URL |
| `Text` | `string` | — | Link text content |
| `Title` | `*string` | `nil` | Optional title |

---

#### DocumentBoundary

Detected document boundary within a PDF.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `StartPage` | `uint32` | — | 1-indexed start page (inclusive). |
| `EndPage` | `uint32` | — | 1-indexed end page (inclusive). |
| `Confidence` | `float32` | — | Confidence in this boundary, `\[0.0, 1.0\]`. |
| `Reason` | `BoundaryReason` | — | Reason for the boundary detection. |

---

#### DocumentMetadata

Metadata about a document for analysis.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `MimeType` | `string` | — | MIME type of the document. |
| `SizeBytes` | `uint64` | — | File size in bytes. |
| `PageCount` | `*uint32` | `nil` | Page count (if known, e.g., from previous analysis). |
| `ForceOcr` | `bool` | — | Whether OCR is forced regardless of text layer. |
| `UserChunkConfig` | `*UserChunkConfig` | `nil` | User-provided chunk configuration overrides. |
| `ChunkingEnabled` | `bool` | — | Whether chunking is enabled for this job. |

---

#### DocumentNode

A single node in the document tree.

Each node has deterministic `id`, typed `content`, optional `parent`/`children`
for tree structure, and metadata like page number, bounding box, and content layer.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Content` | `NodeContent` | — | Node content — tagged enum, type-specific data only. |
| `Parent` | `*uint32` | `nil` | Parent node index (`nil` = root-level node). |
| `Children` | `\[\]uint32` | `/* serde(default) */` | Child node indices in reading order. |
| `ContentLayer` | `ContentLayer` | `/* serde(default) */` | Content layer classification. Always serialised — Kotlin-Android (and any other typed binding) treats the field as non-nullable, so omitting it from the JSON wire would break consumer deserialisation.  `#\[serde(default)\]` covers the missing-field case on inbound JSON. |
| `Page` | `*uint32` | `nil` | Page number where this node starts (1-indexed). |
| `PageEnd` | `*uint32` | `nil` | Page number where this node ends (for multi-page tables/sections). |
| `Bbox` | `*BoundingBox` | `nil` | Bounding box in document coordinates. |
| `Annotations` | `\[\]TextAnnotation` | `/* serde(default) */` | Inline annotations (formatting, links) on this node's text content. Only meaningful for text-carrying nodes; empty for containers. |
| `Attributes` | `*map\[string\]string` | `nil` | Format-specific key-value attributes. Extensible bag for miscellaneous data without a dedicated typed field: CSS classes, LaTeX environment names, Excel cell formulas, slide layout names, etc. |

---

#### DocumentRelationship

A resolved relationship between two nodes in the document tree.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Source` | `uint32` | — | Source node index (the referencing node). |
| `Target` | `uint32` | — | Target node index (the referenced node). |
| `Kind` | `RelationshipKind` | — | Semantic kind of the relationship. |

---

#### DocumentRevision

A single tracked change embedded in a document.

Populated by per-format extractors that understand change-tracking metadata
(DOCX `w:ins`/`w:del`/`w:rPrChange`, ODT `text:change-*`, …). Every
extractor defaults to `ExtractionResult.revisions = None` until a
format-specific implementation is added.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `RevisionId` | `string` | — | Format-specific revision identifier. For DOCX this is the `w:id` attribute value on the change element (e.g. `"42"`). When the attribute is absent a synthetic fallback is generated (`"docx-ins-0"`, `"docx-del-3"`, …). |
| `Author` | `*string` | `nil` | Display name of the author who made this change, when available. |
| `Timestamp` | `*string` | `nil` | ISO-8601 timestamp of the change, when available. Stored as a plain string so this type remains FFI-friendly and unconditionally available without the `chrono` optional dep. DOCX populates this from the `w:date` attribute (e.g. `"2024-03-15T10:30:00Z"`). |
| `Kind` | `RevisionKind` | — | Semantic kind of this revision. |
| `Anchor` | `*RevisionAnchor` | `nil` | Best-effort document location for this revision. Resolution is format-dependent and may be `nil` when the location cannot be determined (e.g. changes inside table cells before table-cell anchor support is added). |
| `Delta` | `RevisionDelta` | — | The content changes that make up this revision. |

---

#### DocumentStructure

Top-level structured document representation.

A flat array of nodes with index-based parent/child references forming a tree.
Root-level nodes have `parent: None`. Use `body_roots()` and `furniture_roots()`
to iterate over top-level content by layer.

##### Validation

Call `validate()` after construction to verify all node indices are in bounds
and parent-child relationships are bidirectionally consistent.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Nodes` | `\[\]DocumentNode` | `nil` | All nodes in document/reading order. |
| `SourceFormat` | `*string` | `nil` | Origin format identifier (e.g. "docx", "pptx", "html", "pdf"). Allows renderers to apply format-aware heuristics when converting the document tree to output formats. |
| `Relationships` | `\[\]DocumentRelationship` | `nil` | Resolved relationships between nodes (footnote refs, citations, anchor links, etc.). Populated during derivation from the internal document representation. Empty when no relationships are detected. |
| `NodeTypes` | `\[\]string` | `nil` | Sorted, deduplicated list of node type names present in this document. Each value is the snake_case `node_type` tag of the corresponding `NodeContent` variant (e.g. `"paragraph"`, `"heading"`, `"table"`, …). Computed from `nodes` via `DocumentStructure.finalize_node_types`. Empty until that method is called (internal construction paths call it at the end of derivation). |

##### Methods

###### FinalizeNodeTypes()

Compute and populate the `node_types` field from the current `nodes`.

Call this after all nodes have been added to the structure. Internal
construction paths (builder, derivation) call this automatically.

**Signature:**

```go
func (o *DocumentStructure) FinalizeNodeTypes()
```

**Example:**

```go
instance.FinalizeNodeTypes()
```

**Returns:** No return value.

###### IsEmpty()

Check if the document structure is empty.

**Signature:**

```go
func (o *DocumentStructure) IsEmpty() bool
```

**Example:**

```go
result := instance.IsEmpty()
```

**Returns:** `bool`

###### Default()

**Signature:**

```go
func (o *DocumentStructure) Default() DocumentStructure
```

**Example:**

```go
result := DocumentStructure.Default()
```

**Returns:** `DocumentStructure`

---

#### DocumentSummary

Summary of an extracted document.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Text` | `string` | — | Summary text (plain prose). |
| `Strategy` | `SummaryStrategy` | — | Strategy that produced this summary. |
| `TokenCount` | `*uint32` | `nil` | Approximate token count of the summary, when known. |

---

#### DocxAppProperties

Application properties from docProps/app.xml for DOCX

Contains Word-specific document statistics and metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Application` | `*string` | `nil` | Application name (e.g., "Microsoft Office Word") |
| `AppVersion` | `*string` | `nil` | Application version |
| `Template` | `*string` | `nil` | Template filename |
| `TotalTime` | `*int32` | `nil` | Total editing time in minutes |
| `Pages` | `*int32` | `nil` | Number of pages |
| `Words` | `*int32` | `nil` | Number of words |
| `Characters` | `*int32` | `nil` | Number of characters (excluding spaces) |
| `CharactersWithSpaces` | `*int32` | `nil` | Number of characters (including spaces) |
| `Lines` | `*int32` | `nil` | Number of lines |
| `Paragraphs` | `*int32` | `nil` | Number of paragraphs |
| `Company` | `*string` | `nil` | Company name |
| `DocSecurity` | `*int32` | `nil` | Document security level |
| `ScaleCrop` | `*bool` | `nil` | Scale crop flag |
| `LinksUpToDate` | `*bool` | `nil` | Links up to date flag |
| `SharedDoc` | `*bool` | `nil` | Shared document flag |
| `HyperlinksChanged` | `*bool` | `nil` | Hyperlinks changed flag |

---

#### DocxMetadata

Word document metadata.

Extracted from DOCX files using shared Office Open XML metadata extraction.
Integrates with `office_metadata` module for core/app/custom properties.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `CoreProperties` | `*CoreProperties` | `nil` | Core properties from docProps/core.xml (Dublin Core metadata) Contains title, creator, subject, keywords, dates, etc. Shared format across DOCX/PPTX/XLSX documents. |
| `AppProperties` | `*DocxAppProperties` | `nil` | Application properties from docProps/app.xml (Word-specific statistics) Contains word count, page count, paragraph count, editing time, etc. DOCX-specific variant of Office application properties. |
| `CustomProperties` | `*map\[string\]interface{}` | `nil` | Custom properties from docProps/custom.xml (user-defined properties) Contains key-value pairs defined by users or applications. Values can be strings, numbers, booleans, or dates. |

---

#### Element

Semantic element extracted from document.

Represents a logical unit of content with semantic classification,
unique identifier, and metadata for tracking origin and position.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `ElementType` | `ElementType` | — | Semantic type of this element |
| `Text` | `string` | — | Text content of the element |
| `Metadata` | `ElementMetadata` | — | Metadata about the element |

---

#### ElementMetadata

Metadata for a semantic element.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `PageNumber` | `*uint32` | `nil` | Page number (1-indexed) |
| `Filename` | `*string` | `nil` | Source filename or document name |
| `Coordinates` | `*BoundingBox` | `nil` | Bounding box coordinates if available |
| `ElementIndex` | `*int` | `nil` | Position index in the element sequence |
| `Additional` | `map\[string\]string` | — | Additional custom metadata |

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
| `Data` | `*\[\]byte` | `nil` | Attachment data (if extracted). Uses `bytes.Bytes` for cheap cloning of large buffers. |

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
| `ToEmails` | `\[\]string` | — | Primary recipient email addresses |
| `CcEmails` | `\[\]string` | — | CC recipient email addresses |
| `BccEmails` | `\[\]string` | — | BCC recipient email addresses |
| `Date` | `*string` | `nil` | Email date/timestamp |
| `MessageId` | `*string` | `nil` | Message-ID header value |
| `PlainText` | `*string` | `nil` | Plain text version of the email body |
| `HtmlContent` | `*string` | `nil` | HTML version of the email body |
| `Content` | `string` | — | Cleaned/processed text content. Aliased as `cleaned_text` for back-compat. |
| `Attachments` | `\[\]EmailAttachment` | — | List of email attachments |
| `Metadata` | `map\[string\]string` | — | Additional email headers and metadata |

---

#### EmailMetadata

Email metadata extracted from .eml and .msg files.

Includes sender/recipient information, message ID, and attachment list.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `FromEmail` | `*string` | `nil` | Sender's email address |
| `FromName` | `*string` | `nil` | Sender's display name |
| `ToEmails` | `\[\]string` | `nil` | Primary recipients |
| `CcEmails` | `\[\]string` | `nil` | CC recipients |
| `BccEmails` | `\[\]string` | `nil` | BCC recipients |
| `MessageId` | `*string` | `nil` | Message-ID header value |
| `Attachments` | `\[\]string` | `nil` | List of attachment filenames |

---

#### EmbeddedChanges

Changes to embedded archive children between two results.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Added` | `\[\]ArchiveEntry` | `nil` | Children present in `b` but not in `a` (matched by `path`). |
| `Removed` | `\[\]ArchiveEntry` | `nil` | Children present in `a` but not in `b` (matched by `path`). |
| `Changed` | `\[\]EmbeddedDiff` | `nil` | Children present in both but with differing content (matched by `path`). Each entry holds the diff of the nested `ExtractionResult`. |

---

#### EmbeddedDiff

Diff for a single embedded archive entry that appears in both results.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Path` | `string` | — | Archive-relative path identifying this entry. |
| `Diff` | `ExtractionDiff` | — | The recursive diff of the entry's extraction result. |

---

#### EmbeddedFile

Embedded file descriptor extracted from the PDF name tree.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Name` | `string` | — | The filename as stored in the PDF name tree. |
| `Data` | `\[\]byte` | — | Raw file bytes from the embedded stream (already decompressed by lopdf). |
| `CompressedSize` | `int` | — | Compressed byte count of the original stream (before decompression). Used by callers to compute the decompression ratio and detect zip-bomb-style attacks that embed a tiny compressed stream expanding to gigabytes of data. |
| `MimeType` | `*string` | `nil` | MIME type if specified in the filespec, otherwise `nil`. |

---

#### EmbeddingBackend

Trait for in-process embedding backend plugins.

Async to match the convention used by `OcrBackend`,
`DocumentExtractor`, and `PostProcessor`.
Host-language bridges (PyO3, napi-rs, Rustler, extendr, magnus, ext-php-rs,
C FFI, etc.) wrap their synchronous host callables in `spawn_blocking` or the
equivalent to satisfy the async signature.

##### Thread safety

Backends must be `Send + Sync + 'static`. They are stored in
`Arc<dyn EmbeddingBackend>` and called concurrently from xberg's chunking
pipeline. If the backend's underlying model isn't thread-safe, the backend
itself must serialize access internally (e.g. via `Mutex<Inner>`).

##### Contract

- `embed(texts)` MUST return exactly `texts.len()` vectors, each of length
  `self.dimensions()`. The dispatcher in `crate.embeddings.embed_texts`
  validates this before returning to downstream consumers; a non-conforming
  backend surfaces as a `XbergError.Validation`, not a panic.

- `embed` may be called from any thread. Its future must be `Send`
  (enforced by `async_trait` when `#[async_trait]` is used on non-WASM targets).

- `dimensions()` is called exactly once at registration, immediately after
  `initialize()` succeeds. The returned value is cached by the registry and
  used for all subsequent shape validation. Lazy-loading implementations can
  defer model loading into `initialize()` and report the real dimension
  afterwards. Later mutations of the backend's reported dimension are not
  observed by xberg — implementations that need to change dimension
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

###### Dimensions()

Embedding vector dimension. Must be `> 0` and must match the length of
every vector returned by `embed`.

**Signature:**

```go
func (o *EmbeddingBackend) Dimensions() int
```

**Example:**

```go
result := instance.Dimensions()
```

**Returns:** `int`

###### Embed()

Embed a batch of texts, returning one vector per input in order.

**Errors:**

Implementations should return `Plugin` for
backend-specific failures. The dispatcher layers its own validation
(length, per-vector dimension) on top.

**Signature:**

```go
func (o *EmbeddingBackend) Embed(texts []string) ([][]float32, error)
```

**Example:**

```go
result, err := instance.Embed(nil)
if err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Texts` | `\[\]string` | Yes | The texts |

**Returns:** `[][]float32`

**Errors:** Returns `error`.

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
| `CacheDir` | `*string` | `nil` | Custom cache directory for model files Defaults to `~/.cache/xberg/embeddings/` if not specified. Allows full customization of model download location. |
| `Acceleration` | `*AccelerationConfig` | `nil` | Hardware acceleration for the embedding ONNX model. When set, controls which execution provider (CPU, CUDA, CoreML, TensorRT) is used for inference. Defaults to `nil` (auto-select per platform). |
| `MaxEmbedDurationSecs` | `*uint64` | `nil` | Maximum wall-clock duration (in seconds) for a single `embed()` call when using `EmbeddingModelType.Plugin`. Applies only to the in-process plugin path — protects against hung host-language backends (e.g. a Python callback deadlocked on the GIL, a model stuck on CUDA OOM retries, etc.). On timeout, the dispatcher returns `Plugin` instead of blocking forever. `nil` disables the timeout. The default (60 seconds) is conservative for common in-process inference; increase for large batches on slow hardware. |

##### Methods

###### Default()

**Signature:**

```go
func (o *EmbeddingConfig) Default() EmbeddingConfig
```

**Example:**

```go
result := EmbeddingConfig.Default()
```

**Returns:** `EmbeddingConfig`

---

#### EmbeddingPreset

Preset configurations for common RAG use cases.

Each preset combines chunk size, overlap, and embedding model
to provide an optimized configuration for specific scenarios.

All string fields are owned `String` for FFI compatibility — instances
are safe to clone and pass across language boundaries.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Name` | `string` | — | Short identifier for this preset (e.g. `"balanced"`, `"fast"`, `"quality"`). |
| `ChunkSize` | `int` | — | Target chunk size in characters. |
| `Overlap` | `int` | — | Overlap between consecutive chunks in characters. |
| `ModelRepo` | `string` | — | HuggingFace repository name for the model. |
| `Pooling` | `string` | — | Pooling strategy: "cls" or "mean". |
| `ModelFile` | `string` | — | Path to the ONNX model file within the repo. |
| `Dimensions` | `int` | — | Embedding vector dimension produced by this model. |
| `Description` | `string` | — | Human-readable description of the preset's intended use case. |

---

#### EnrichOptions

Which enrichment passes to run on a piece of text.

All fields default to `false` / empty so callers can opt in precisely.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Keywords` | `bool` | — | Run keyword extraction on the input text. When `true`, the enrichment backend identifies the most salient terms and returns them in `EnrichResult.keywords`. |
| `Entities` | `bool` | — | Run named-entity recognition (NER) on the input text. When `true`, the enrichment backend identifies named entities (persons, organisations, locations, etc.) and returns them in `EnrichResult.entities`. |
| `Labels` | `\[\]string` | `nil` | Custom labels to pass through to the result without modification. These are caller-supplied tags that the enrichment pipeline propagates verbatim into `EnrichResult.labels`. Useful for attaching project- or document-level metadata to every enrichment result. |

---

#### EnrichResult

Structured output produced by a completed enrichment pass.

Fields are populated only when the corresponding `EnrichOptions` flag was set.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Keywords` | `\[\]string` | `nil` | Salient terms extracted from the text. Populated when `EnrichOptions.keywords` was `true`. The ordering is backend-defined (typically by descending relevance score). |
| `Entities` | `\[\]Entity` | `nil` | Named entities found in the text. Populated when `EnrichOptions.entities` was `true`. Uses the shared OSS entity schema (`Entity` / `EntityCategory`) so consumers can pattern-match on entity categories without JSON gymnastics. |
| `Labels` | `\[\]string` | `nil` | Caller-supplied labels echoed from `EnrichOptions.labels`. |

---

#### Entity

A single named entity detected in the extracted text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Category` | `EntityCategory` | — | Canonical category the entity belongs to (PERSON, ORG, LOCATION, etc.). |
| `Text` | `string` | — | Raw mention text exactly as it appeared in the source. |
| `Start` | `uint32` | — | Byte-offset span in `ExtractionResult.content` where the mention starts. |
| `End` | `uint32` | — | Byte-offset span in `ExtractionResult.content` where the mention ends (exclusive). |
| `Confidence` | `*float32` | `nil` | Backend-reported confidence in `\[0.0, 1.0\]`. `nil` when the backend does not expose confidence scores. |

---

#### EpubMetadata

EPUB metadata (Dublin Core extensions).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Coverage` | `*string` | `nil` | Dublin Core `coverage` field (geographic or temporal scope). |
| `DcFormat` | `*string` | `nil` | Dublin Core `format` field (media type of the resource). |
| `Relation` | `*string` | `nil` | Dublin Core `relation` field (related resource identifier). |
| `Source` | `*string` | `nil` | Dublin Core `source` field (origin resource identifier). |
| `DcType` | `*string` | `nil` | Dublin Core `type` field (nature or genre of the resource). |
| `CoverImage` | `*string` | `nil` | Path or identifier of the cover image within the EPUB container. |

---

#### ErrorMetadata

Error metadata (for batch operations).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `ErrorType` | `string` | — | Machine-readable error type identifier (e.g. "UnsupportedFormat"). |
| `Message` | `string` | — | Human-readable error description. |

---

#### ExcelMetadata

Excel/spreadsheet format metadata.

Identifies the document as a spreadsheet source via the `FormatMetadata.Excel`
discriminant. Sheet count and sheet names are stored inside this struct.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `SheetCount` | `*uint32` | `nil` | Number of sheets in the workbook. |
| `SheetNames` | `*\[\]string` | `nil` | Names of all sheets in the workbook. |

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
| `TableCells` | `*\[\]\[\]string` | `nil` | Pre-extracted table cells (2D vector of cell values) Populated during markdown generation to avoid re-parsing markdown. None for empty sheets. |

---

#### ExcelWorkbook

Excel workbook representation.

Contains all sheets from an Excel file (.xlsx, .xls, etc.) with
extracted content and metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Sheets` | `\[\]ExcelSheet` | — | All sheets in the workbook |
| `Metadata` | `map\[string\]string` | — | Workbook-level metadata (author, creation date, etc.) |
| `Revisions` | `*\[\]DocumentRevision` | `/* serde(default) */` | Collaborative-edit revision headers from `xl/revisions/revisionHeaders.xml`. Populated for legacy shared-workbook `.xlsx` files that contain the `xl/revisions/` directory. Each `<header>` element maps to one `DocumentRevision { kind: FormatChange }` carrying the header's `guid` (→ `revision_id`), `userName` (→ `author`), and `dateTime` (→ `timestamp`). `anchor` and `delta` are `nil`/empty for v1 (per-cell log parsing is a follow-up). `nil` when `xl/revisions/revisionHeaders.xml` is absent. |

---

#### ExtractInput

Unified extraction input for all public extraction entry points.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Kind` | `ExtractInputKind` | `ExtractInputKind.Uri` | Source kind. `bytes` requires `bytes`; `uri` requires `uri`. |
| `Bytes` | `*\[\]byte` | `nil` | Raw bytes for `kind = "bytes"`. |
| `Uri` | `*string` | `nil` | Local path, `file://` URI, or HTTP(S) URL for `kind = "uri"`. |
| `MimeType` | `*string` | `nil` | MIME type hint. |
| `Filename` | `*string` | `nil` | Filename hint used for MIME detection and metadata. |
| `Config` | `*FileExtractionConfig` | `nil` | Per-input extraction overrides. |

##### Methods

###### Default()

**Signature:**

```go
func (o *ExtractInput) Default() ExtractInput
```

**Example:**

```go
result := ExtractInput.Default()
```

**Returns:** `ExtractInput`

###### Bytes()

Build a bytes input with a MIME type and optional filename hint.

**Signature:**

```go
func (o *ExtractInput) Bytes(bytes []byte, mimeType string, filename string) ExtractInput
```

**Example:**

```go
result := ExtractInput.Bytes([]byte("data"), "value", "value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Bytes` | `\[\]byte` | Yes | The bytes |
| `MimeType` | `string` | Yes | The mime type |
| `Filename` | `*string` | No | The filename |

**Returns:** `ExtractInput`

###### Uri()

Build a URI input from a local path, `file://` URI, or HTTP(S) URL.

**Signature:**

```go
func (o *ExtractInput) Uri(uri string) ExtractInput
```

**Example:**

```go
result := ExtractInput.Uri("value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Uri` | `string` | Yes | The uri |

**Returns:** `ExtractInput`

---

#### ExtractedImage

Extracted image from a document.

Contains raw image data, metadata, and optional nested OCR results.
Raw bytes allow cross-language compatibility - users can convert to
PIL.Image (Python), Sharp (Node.js), or other formats as needed.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Data` | `\[\]byte` | — | Raw image data (PNG, JPEG, WebP, etc. bytes). Uses `bytes.Bytes` for cheap cloning of large buffers. |
| `Format` | `string` | — | Image format (e.g., "jpeg", "png", "webp") Uses Cow<'static, str> to avoid allocation for static literals. |
| `ImageIndex` | `uint32` | — | Zero-indexed position of this image in the document/page |
| `PageNumber` | `*uint32` | `nil` | Page/slide number where image was found (1-indexed) |
| `Width` | `*uint32` | `nil` | Image width in pixels |
| `Height` | `*uint32` | `nil` | Image height in pixels |
| `Colorspace` | `*string` | `nil` | Colorspace information (e.g., "RGB", "CMYK", "Gray") |
| `BitsPerComponent` | `*uint32` | `nil` | Bits per color component (e.g., 8, 16) |
| `IsMask` | `bool` | — | Whether this image is a mask image |
| `Description` | `*string` | `nil` | Optional description of the image |
| `OcrResult` | `*ExtractionResult` | `nil` | Nested OCR extraction result (if image was OCRed) When OCR is performed on this image, the result is embedded here rather than in a separate collection, making the relationship explicit. |
| `BoundingBox` | `*BoundingBox` | `nil` | Bounding box of the image on the page (PDF coordinates: x0=left, y0=bottom, x1=right, y1=top). Only populated for PDF-extracted images when position data is available from the PDF extractor. |
| `SourcePath` | `*string` | `nil` | Original source path of the image within the document archive (e.g., "media/image1.png" in DOCX). Used for rendering image references when the binary data is not extracted. |
| `ImageKind` | `*ImageKind` | `nil` | Heuristic classification of what this image likely depicts. `nil` if classification was disabled or inconclusive. |
| `KindConfidence` | `*float32` | `nil` | Confidence score for `image_kind`, in the range 0.0 to 1.0. |
| `ClusterId` | `*uint32` | `nil` | Identifier shared across images that form a single logical figure (e.g. all raster tiles of one technical drawing). `nil` for singletons. |
| `Caption` | `*string` | `nil` | VLM-generated caption describing the image, when captioning is configured. Populated by the captioning post-processor (`crates/xberg/src/plugins/processor/builtin/captioning.rs`), which routes each image through `crate.llm.region_extractor.extract_region_with_vlm` in caption mode. `nil` when captioning is disabled or the VLM declined to caption. |
| `QrCodes` | `*\[\]QrCode` | `nil` | QR codes decoded from this image, when QR detection is enabled. Populated by the QR post-processor (`crates/xberg/src/extractors/qr.rs`) via the pure-Rust `rqrr` decoder. `nil` when QR detection is disabled; an empty `Some(\[\])` when detection ran but found nothing. |
| `DataBase64` | `*string` | `nil` | Base64-encoded copy of `data`; populated when `ImageExtractionConfig.include_data_base64` is `true`. Omitted from JSON by default; use instead of `data` in JSON-only clients. |

---

#### ExtractedUri

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

#### ExtractionConfidence

Combined confidence on `[0, 1]`.

When OCR did not run, the `ocr_aggregate` weight folds into `text_coverage`
so the weighted sum still totals 1.0.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `TextCoverage` | `float32` | — | Fraction of pages with a usable text layer. |
| `OcrAggregate` | `*float32` | `nil` | Mean OCR per-element recognition confidence when OCR ran; `nil` when it did not. |
| `SchemaCompliance` | `SchemaCompliance` | — | Whether the merged output validates against the preset schema. |
| `Combined` | `float32` | — | Weighted blend in `\[0, 1\]`.  The value compared against the fallback threshold. |

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
| `ForceOcrPages` | `*\[\]uint32` | `nil` | Force OCR on specific pages only (1-indexed page numbers, must be >= 1). When set, only the listed pages are OCR'd regardless of text layer quality. Unlisted pages use native text extraction. Ignored when `force_ocr` is `true`. Only applies to PDF documents. Duplicates are automatically deduplicated. An `ocr` config is recommended for backend/language selection; defaults are used if absent. |
| `DisableOcr` | `bool` | `false` | Disable OCR entirely, even for images. When `true`, OCR is skipped for all document types. Images return metadata only (dimensions, format, EXIF) without text extraction. PDFs use only native text extraction without OCR fallback. Cannot be `true` simultaneously with `force_ocr`. *Added in v4.7.0.* |
| `Chunking` | `*ChunkingConfig` | `nil` | Text chunking configuration (None = chunking disabled) |
| `ContentFilter` | `*ContentFilterConfig` | `nil` | Content filtering configuration (None = use extractor defaults). Controls whether document "furniture" (headers, footers, watermarks, repeating text) is included in or stripped from extraction results. See `ContentFilterConfig` for per-field documentation. |
| `Images` | `*ImageExtractionConfig` | `nil` | Image extraction configuration (None = no image extraction) |
| `PdfOptions` | `*PdfConfig` | `nil` | PDF-specific options (None = use defaults) |
| `TokenReduction` | `*TokenReductionOptions` | `nil` | Token reduction configuration (None = no token reduction) |
| `LanguageDetection` | `*LanguageDetectionConfig` | `nil` | Language detection configuration (None = no language detection) |
| `Pages` | `*PageConfig` | `nil` | Page extraction configuration (None = no page tracking) |
| `Keywords` | `*KeywordConfig` | `nil` | Keyword extraction configuration (None = no keyword extraction) |
| `Postprocessor` | `*PostProcessorConfig` | `nil` | Post-processor configuration (None = use defaults) |
| `HtmlOutput` | `*HtmlOutputConfig` | `nil` | Styled HTML output configuration. When set alongside `output_format = OutputFormat.Html`, the extraction pipeline uses `StyledHtmlRenderer` which emits stable `kb-*` CSS class hooks on every structural element and optionally embeds theme CSS or user-supplied CSS in a `<style>` block. When `nil`, the existing plain comrak-based HTML renderer is used. |
| `ExtractionTimeoutSecs` | `*uint64` | `nil` | Default per-file timeout in seconds for batch extraction. When set, each file in a batch will be canceled after this duration unless overridden by `FileExtractionConfig.timeout_secs`. Defaults to `Some(60)` to prevent pathological files (e.g. deeply nested archives, documents with millions of cells) from running indefinitely and exhausting caller resources. Set to `nil` to disable the timeout for trusted input or long-running workloads. |
| `MaxConcurrentExtractions` | `*int` | `nil` | Maximum concurrent extractions in batch operations (None = (num_cpus × 1.5).ceil()). Limits parallelism to prevent resource exhaustion when processing large batches. Defaults to (num_cpus × 1.5).ceil() when not set. |
| `ResultFormat` | `ResultFormat` | `ResultFormat.Unified` | Result structure format Controls whether results are returned in unified format (default) with all content in the `content` field, or element-based format with semantic elements (for Unstructured-compatible output). |
| `SecurityLimits` | `*SecurityLimits` | `nil` | Security limits for archive extraction. Controls maximum archive size, compression ratio, file count, and other security thresholds to prevent decompression bomb attacks. Also caps nesting depth, iteration count, entity / token length, total content size, and table cell count for every extraction path that ingests user-controlled bytes. When `nil`, default limits are used. |
| `MaxEmbeddedFileBytes` | `*uint64` | `nil` | Maximum uncompressed size in bytes for a single embedded file before recursive extraction is attempted (default: 50 MiB). Applies to embedded objects inside OOXML containers (DOCX, PPTX) and to email attachments processed via recursive extraction. Files that exceed this limit are skipped with a `ProcessingWarning` rather than passed to the extraction pipeline, preventing a single oversized embedded object from consuming unbounded memory or time. Set to `nil` to disable the per-embedded-file cap (falls back to `security_limits.max_archive_size` as the only guard). |
| `OutputFormat` | `OutputFormat` | `OutputFormat.Plain` | Content text format (default: Plain). Controls the format of the extracted content: - `Plain`: Raw extracted text (default) - `Markdown`: Markdown formatted output - `Djot`: Djot markup format (requires djot feature) - `Html`: HTML formatted output When set to a structured format, extraction results will include formatted output. The `formatted_content` field may be populated when format conversion is applied. |
| `Layout` | `*LayoutDetectionConfig` | `nil` | Layout detection configuration (None = layout detection disabled). When set, PDF pages and images are analyzed for document structure (headings, code, formulas, tables, figures, etc.) using RT-DETR models via ONNX Runtime. For PDFs, layout hints override paragraph classification in the markdown pipeline. For images, per-region OCR is performed with markdown formatting based on detected layout classes. Requires the `layout-detection` feature to run inference; the field is present whenever the `layout-types` feature is active (which includes `layout-detection` as well as the no-ORT target groups). |
| `Transcription` | `*TranscriptionConfig` | `nil` | Transcription (speech-to-text) configuration for audio/video files. When set and `enabled`, files with audio/video MIME types (mp3, mp4, m4a, wav, webm, etc.) are routed to the Whisper-based transcription pipeline. The actual heavy dependencies are only active under the `transcription` feature; the field is visible under `transcription-types` (including on WASM and Android targets that use the no-ORT preset). Default: `nil` (transcription disabled). This is an additive, non-breaking change. |
| `UseLayoutForMarkdown` | `bool` | `false` | Run layout detection on the non-OCR PDF markdown path. When `true` and `layout` is `Some(_)`, layout regions inform heading, table, list, and figure detection in the structure pipeline that would otherwise rely on font-clustering heuristics alone. Significantly improves SF1 (structural F1) at the cost of inference latency (~150-300ms/page CPU, ~20-50ms/page GPU). Default: `false`. Requires the `layout-detection` feature. |
| `IncludeDocumentStructure` | `bool` | `false` | Enable structured document tree output. When true, populates the `document` field on `ExtractionResult` with a hierarchical `DocumentStructure` containing heading-driven section nesting, table grids, content layer classification, and inline annotations. Independent of `result_format` — can be combined with Unified or ElementBased. |
| `Acceleration` | `*AccelerationConfig` | `nil` | Hardware acceleration configuration for ONNX Runtime models. Controls execution provider selection for layout detection and embedding models. When `nil`, uses platform defaults (CoreML on macOS, CUDA on Linux, CPU on Windows). |
| `CacheNamespace` | `*string` | `nil` | Cache namespace for tenant isolation. When set, cache entries are stored under `{cache_dir}/{namespace}/`. Must be alphanumeric, hyphens, or underscores only (max 64 chars). Different namespaces have isolated cache spaces on the same filesystem. |
| `CacheTtlSecs` | `*uint64` | `nil` | Per-request cache TTL in seconds. Overrides the global `max_age_days` for this specific extraction. When `0`, caching is completely skipped (no read or write). When `nil`, the global TTL applies. |
| `Email` | `*EmailConfig` | `nil` | Email extraction configuration (None = use defaults). Currently supports configuring the fallback codepage for MSG files that do not specify one. See `EmailConfig` for details. |
| `Url` | `UrlExtractionConfig` | — | URL ingestion and crawl configuration. |
| `MaxArchiveDepth` | `int` | — | Maximum recursion depth for archive extraction (default: 3). Set to 0 to disable recursive extraction (legacy behavior). |
| `TreeSitter` | `*TreeSitterConfig` | `nil` | Tree-sitter language pack configuration (None = tree-sitter disabled). When set, enables code file extraction using tree-sitter parsers. Controls grammar download behavior and code analysis options. |
| `StructuredExtraction` | `*StructuredExtractionConfig` | `nil` | Structured extraction via LLM (None = disabled). When set, the extracted document content is sent to an LLM with the provided JSON schema. The structured response is stored in `ExtractionResult.structured_output`. |
| `Ner` | `*NerConfig` | `nil` | Named-entity recognition configuration. When set, the NER post-processor runs at the Middle stage and populates `ExtractionResult.entities`. |
| `Redaction` | `*RedactionConfig` | `nil` | Redaction / anonymisation configuration. When set, the redaction post-processor runs at the Late stage and rewrites every textual field in `ExtractionResult`, emitting an audit trail in `ExtractionResult.redaction_report`. |
| `Summarization` | `*SummarizationConfig` | `nil` | Summarisation configuration. When set, the summarisation post-processor runs at the Middle stage and populates `ExtractionResult.summary`. |
| `Translation` | `*TranslationConfig` | `nil` | Translation configuration. When set, the translation post-processor runs at the Middle stage and populates `ExtractionResult.translation`. |
| `PageClassification` | `*PageClassificationConfig` | `nil` | Per-page classification configuration. When set, the classification post-processor runs at the Middle stage and populates `ExtractionResult.page_classifications`. |
| `Captioning` | `*CaptioningConfig` | `nil` | VLM captioning configuration for extracted images. When set, the captioning post-processor runs at the Middle stage and writes a caption into each `ExtractedImage.caption`. |
| `QrCodes` | `*bool` | `nil` | Enable QR-code detection in extracted images. When `true`, the QR post-processor runs at the Middle stage and populates `ExtractedImage.qr_codes`. |

##### Methods

###### Default()

**Signature:**

```go
func (o *ExtractionConfig) Default() ExtractionConfig
```

**Example:**

```go
result := ExtractionConfig.Default()
```

**Returns:** `ExtractionConfig`

###### NeedsImageData()

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

```go
func (o *ExtractionConfig) NeedsImageData() bool
```

**Example:**

```go
result := instance.NeedsImageData()
```

**Returns:** `bool`

###### NeedsImageProcessing()

Returns `true` when any image processing is needed during extraction.

##### Optimization Impact

For text-only extractions (no OCR, no image extraction, no captioning), skipping
image decompression can improve CPU utilization by 5-10% by avoiding wasteful
image I/O and processing when results won't be used.

**Signature:**

```go
func (o *ExtractionConfig) NeedsImageProcessing() bool
```

**Example:**

```go
result := instance.NeedsImageProcessing()
```

**Returns:** `bool`

---

#### ExtractionDiff

The complete diff between two `ExtractionResult` values.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `ContentDiff` | `\[\]DiffHunk` | `nil` | Unified-diff hunks for the `content` field. Empty when the content is identical. |
| `TablesAdded` | `\[\]Table` | `nil` | Tables present in `b` but not in `a` (by index position, excess right-side tables). |
| `TablesRemoved` | `\[\]Table` | `nil` | Tables present in `a` but not in `b` (by index position, excess left-side tables). |
| `TablesChanged` | `\[\]TableDiff` | `nil` | Cell-level changes for table pairs that share the same index and dimensions. |
| `MetadataChanged` | `interface{}` | — | Metadata difference, encoded as a JSON object with three top-level keys: `added` (keys present in `b` but not `a`), `removed` (keys present in `a` but not `b`), and `changed` (keys whose values differ — each entry is `{ "from": <value-in-a>, "to": <value-in-b> }`). This is NOT RFC 6902 JSON Patch — we deliberately chose a flatter shape to avoid pulling in a json-patch crate. If you need RFC 6902 semantics (with JSON Pointer paths) feed `a.metadata` and `b.metadata` to your preferred json-patch impl directly. |
| `EmbeddedChanges` | `EmbeddedChanges` | — | Changes to embedded archive children. |

---

#### ExtractionErrorItem

Non-fatal per-input extraction error captured by `ExtractionOutput`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Index` | `int` | — | Input index in the original request. |
| `Code` | `uint32` | — | Stable numeric error code. |
| `ErrorType` | `string` | — | Stable snake_case error kind. |
| `Source` | `string` | — | Best-effort source identifier. |
| `Message` | `string` | — | Error message. |

---

#### ExtractionOutput

Unified extraction output envelope.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Results` | `\[\]ExtractionResult` | `nil` | Extraction results in discovery order. |
| `Errors` | `\[\]ExtractionErrorItem` | `nil` | Non-fatal per-input errors. |
| `Summary` | `ExtractionSummary` | — | Aggregate counts for the operation. |
| `CrawlFinalUrls` | `\[\]string` | `nil` | Final URLs reached after redirects during URL ingestion. |
| `CrawlRedirectCount` | `int` | — | Total redirects followed while fetching or crawling URLs. |
| `CrawlUniqueNormalizedUrls` | `\[\]string` | `nil` | Unique normalized URLs discovered by crawls. |

##### Methods

###### Single()

Build an output containing one successful result.

**Signature:**

```go
func (o *ExtractionOutput) Single(result ExtractionResult) ExtractionOutput
```

**Example:**

```go
result := ExtractionOutput.Single(ExtractionResult{})
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Result` | `ExtractionResult` | Yes | The extraction result |

**Returns:** `ExtractionOutput`

---

#### ExtractionResult

General extraction result used by the core extraction API.

This is the main result type returned by all extraction functions.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Content` | `string` | — | Plain-text representation of the extracted document content. |
| `MimeType` | `string` | — | MIME type of the source document (e.g. `"application/pdf"`). |
| `Metadata` | `Metadata` | — | Document-level metadata (author, title, dates, format-specific fields). |
| `ExtractionMethod` | `*ExtractionMethod` | `nil` | Extraction strategy used to produce the returned text. Populated when the extractor can reliably distinguish native text extraction, OCR-only extraction, or mixed native/OCR output. |
| `Tables` | `\[\]Table` | `nil` | Tables extracted from the document, each with structured cell data. |
| `DetectedLanguages` | `*\[\]string` | `nil` | ISO 639-1 language codes detected in the document content. |
| `Chunks` | `*\[\]Chunk` | `nil` | Text chunks when chunking is enabled. When chunking configuration is provided, the content is split into overlapping chunks for efficient processing. Each chunk contains the text, optional embeddings (if enabled), and metadata about its position. |
| `Images` | `*\[\]ExtractedImage` | `nil` | Extracted images from the document. When image extraction is enabled via `ImageExtractionConfig`, this field contains all images found in the document with their raw data and metadata. Each image may optionally contain a nested `ocr_result` if OCR was performed. |
| `Pages` | `*\[\]PageContent` | `nil` | Per-page content when page extraction is enabled. When page extraction is configured, the document is split into per-page content with tables and images mapped to their respective pages. |
| `Elements` | `*\[\]Element` | `nil` | Semantic elements when element-based result format is enabled. When result_format is set to ElementBased, this field contains semantic elements with type classification, unique identifiers, and metadata for Unstructured-compatible element-based processing. |
| `DjotContent` | `*DjotContent` | `nil` | Rich Djot content structure (when extracting Djot documents). When extracting Djot documents with structured extraction enabled, this field contains the full semantic structure including: - Block-level elements with nesting - Inline formatting with attributes - Links, images, footnotes - Math expressions - Complete attribute information The `content` field still contains plain text for backward compatibility. Always `nil` for non-Djot documents. |
| `OcrElements` | `*\[\]OcrElement` | `nil` | OCR elements with full spatial and confidence metadata. When OCR is performed with element extraction enabled, this field contains the structured representation of detected text including: - Bounding geometry (rectangles or quadrilaterals) - Confidence scores (detection and recognition) - Rotation information - Hierarchical relationships (Tesseract only) This field preserves all metadata that would otherwise be lost when converting to plain text or markdown output formats. Only populated when `OcrElementConfig.include_elements` is true. |
| `Document` | `*DocumentStructure` | `nil` | Structured document tree (when document structure extraction is enabled). When `include_document_structure` is true in `ExtractionConfig`, this field contains the full hierarchical representation of the document including: - Heading-driven section nesting - Table grids with cell-level metadata - Content layer classification (body, header, footer, footnote) - Inline text annotations (formatting, links) - Bounding boxes and page numbers Independent of `result_format` — can be combined with Unified or ElementBased. |
| `ExtractedKeywords` | `*\[\]Keyword` | `nil` | Extracted keywords when keyword extraction is enabled. When keyword extraction (RAKE or YAKE) is configured, this field contains the extracted keywords with scores, algorithm info, and position data. Previously stored in `metadata.additional\["keywords"\]`. |
| `QualityScore` | `*float64` | `nil` | Document quality score from quality analysis. A value between 0.0 and 1.0 indicating the overall text quality. Previously stored in `metadata.additional\["quality_score"\]`. |
| `ProcessingWarnings` | `\[\]ProcessingWarning` | `nil` | Non-fatal warnings collected during processing pipeline stages. Captures errors from optional pipeline features (embedding, chunking, language detection, output formatting) that don't prevent extraction but may indicate degraded results. Previously stored as individual keys in `metadata.additional`. |
| `Annotations` | `*\[\]PdfAnnotation` | `nil` | PDF annotations extracted from the document. When annotation extraction is enabled via `PdfConfig.extract_annotations`, this field contains text notes, highlights, links, stamps, and other annotations found in PDF documents. |
| `Children` | `*\[\]ArchiveEntry` | `nil` | Nested extraction results from archive contents. When extracting archives, each processable file inside produces its own full extraction result. Set to `nil` for non-archive formats. Use `max_archive_depth` in config to control recursion depth. |
| `Uris` | `*\[\]ExtractedUri` | `nil` | URIs/links discovered during document extraction. Contains hyperlinks, image references, citations, email addresses, and other URI-like references found in the document. Always extracted when present in the source document. |
| `Revisions` | `*\[\]DocumentRevision` | `nil` | Tracked changes embedded in the source document. Populated by per-format extractors that understand change-tracking metadata (DOCX `w:ins`/`w:del`/`w:rPrChange`, ODT `text:change-*`, …). Every extractor defaults to `nil` until its format-specific implementation is added. Extractors that do populate this field follow the "accepted-changes" convention: inserted text is present in `content`, deleted text is absent — the revision list is the separate audit trail. |
| `StructuredOutput` | `*interface{}` | `nil` | Structured extraction output from LLM-based JSON schema extraction. When `structured_extraction` is configured in `ExtractionConfig`, the extracted document content is sent to a VLM with the provided JSON schema. The response is parsed and stored here as a JSON value matching the schema. |
| `CodeIntelligence` | `*interface{}` | `nil` | Code intelligence results from tree-sitter analysis. Populated when extracting source code files with the `tree-sitter` feature. Contains metrics, structural analysis, imports/exports, comments, docstrings, symbols, diagnostics, and optionally chunked code segments. Stored as an opaque JSON value so that all language bindings (Go, Java, C#, …) can deserialize it as a raw JSON object rather than a typed struct. The underlying type is `tree_sitter_language_pack.ProcessResult`. |
| `LlmUsage` | `*\[\]LlmUsage` | `nil` | LLM token usage and cost data for all LLM calls made during this extraction. Contains one entry per LLM call. Multiple entries are produced when VLM OCR, structured extraction, or LLM embeddings run during the same extraction. `nil` when no LLM was used. |
| `Entities` | `*\[\]Entity` | `nil` | Named entities detected in `content` by the NER post-processor. `nil` when no NER backend is configured. Populated by the `xberg-gliner` ONNX backend or the LLM-driven backend (see `crates/xberg/src/text/ner/`). |
| `Summary` | `*DocumentSummary` | `nil` | Summary of `content` produced by the summarisation post-processor. `nil` when summarisation is not configured. Populated by the TextRank extractive backend (deterministic, no external service) or by the liter-llm-driven abstractive backend. |
| `ExtractionConfidence` | `*ExtractionConfidence` | `nil` | Confidence score computed by the heuristics pipeline. Populated when the `heuristics` feature is enabled and confidence scoring has been performed.  Combines text-coverage, OCR aggregate confidence, and schema-compliance into a single `\[0, 1\]` value. `nil` when confidence scoring is not configured or the feature is absent. |
| `Translation` | `*Translation` | `nil` | Translation of `content` produced by the translation post-processor. `nil` when translation is not configured. |
| `PageClassifications` | `*\[\]PageClassification` | `nil` | Per-page classifications produced by the page-classification post-processor. `nil` when classification is not configured. |
| `RedactionReport` | `*RedactionReport` | `nil` | Audit report of redactions applied by the redaction post-processor. The redaction processor rewrites `content`, `formatted_content`, every chunk's text, and the textual fields of `entities` / `summary` / `translation` / `page_classifications` in place. This report describes what was found and how it was replaced. `nil` when redaction is not configured. |
| `Formulas` | `\[\]Formula` | `nil` | Mathematical formulas recognized in the document. Populated by the layout-guided formula pipeline when the `layout-detection` feature is enabled and the document contains regions classified as formulas. Empty otherwise. |
| `FormFields` | `\[\]PdfFormField` | `nil` | Form fields extracted from a PDF's AcroForm or XFA structure. Populated by the PDF extractor when `PdfConfig.extract_form_fields` is enabled (default) and the document is a fillable form. Empty otherwise. |
| `FormattedContent` | `*string` | `nil` | Pre-rendered content in the requested output format. Populated during `derive_extraction_result` before tree derivation consumes element data. `apply_output_format` swaps this into `content` at the end of the pipeline, after post-processors have operated on plain text. |

##### Methods

###### FromOcr()

Convert from an OCR result.

**Signature:**

```go
func (o *ExtractionResult) FromOcr(ocr OcrExtractionResult) ExtractionResult
```

**Example:**

```go
result := ExtractionResult.FromOcr(OcrExtractionResult{})
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Ocr` | `OcrExtractionResult` | Yes | The ocr extraction result |

**Returns:** `ExtractionResult`

---

#### ExtractionSummary

Summary for a unified extraction call.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Inputs` | `int` | — | Number of inputs submitted by the caller. |
| `Results` | `int` | — | Number of extraction results produced. |
| `Errors` | `int` | — | Number of per-input errors. |
| `RemoteUrls` | `int` | — | Number of URI inputs that resolved to remote HTTP(S) URLs. |
| `PagesCrawled` | `int` | — | Number of HTML pages crawled or scraped. |
| `DocumentsDownloaded` | `int` | — | Number of downloaded non-HTML documents extracted from URLs. |

---

#### FictionBookMetadata

FictionBook (FB2) metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Genres` | `\[\]string` | `nil` | Genre tags as declared in the FB2 `<genre>` elements. |
| `Sequences` | `\[\]string` | `nil` | Book series (sequence) names, if any. |
| `Annotation` | `*string` | `nil` | Short annotation / summary from the FB2 `<annotation>` element. |

---

#### FileExtractionConfig

Per-file extraction configuration overrides for batch processing.

All fields are `Option<T>` — `nil` means "use the batch-level default."
This type is used by `config` and `extract_batch`
to allow heterogeneous extraction settings within a single batch.

##### Excluded Fields

The following `ExtractionConfig` fields are batch-level only and
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
| `ForceOcrPages` | `*\[\]uint32` | `nil` | Override force OCR pages for this file (1-indexed page numbers). |
| `DisableOcr` | `*bool` | `nil` | Override disable OCR for this file. |
| `Chunking` | `*ChunkingConfig` | `nil` | Override chunking configuration for this file. |
| `ContentFilter` | `*ContentFilterConfig` | `nil` | Override content filtering configuration for this file. |
| `Images` | `*ImageExtractionConfig` | `nil` | Override image extraction configuration for this file. |
| `PdfOptions` | `*PdfConfig` | `nil` | Override PDF options for this file. |
| `TokenReduction` | `*TokenReductionOptions` | `nil` | Override token reduction for this file. |
| `LanguageDetection` | `*LanguageDetectionConfig` | `nil` | Override language detection for this file. |
| `Pages` | `*PageConfig` | `nil` | Override page extraction for this file. |
| `Keywords` | `*KeywordConfig` | `nil` | Override keyword extraction for this file. |
| `Postprocessor` | `*PostProcessorConfig` | `nil` | Override post-processor for this file. |
| `ResultFormat` | `*ResultFormat` | `nil` | Override result format for this file. |
| `OutputFormat` | `*OutputFormat` | `nil` | Override output content format for this file. |
| `IncludeDocumentStructure` | `*bool` | `nil` | Override document structure output for this file. |
| `Layout` | `*LayoutDetectionConfig` | `nil` | Override layout detection for this file. |
| `Transcription` | `*TranscriptionConfig` | `nil` | Transcription configuration (see ExtractionConfig for docs). |
| `TimeoutSecs` | `*uint64` | `nil` | Override per-file extraction timeout in seconds. When set, the extraction for this file will be canceled after the specified duration. A timed-out file produces an error result without affecting other files in the batch. |
| `TreeSitter` | `*TreeSitterConfig` | `nil` | Override tree-sitter configuration for this file. |
| `StructuredExtraction` | `*StructuredExtractionConfig` | `nil` | Override structured extraction configuration for this file. When set, enables LLM-based structured extraction with a JSON schema for this specific file. The extracted content is sent to a VLM/LLM and the response is parsed according to the provided schema. |

---

#### Footnote

Footnote in Djot.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Label` | `string` | — | Footnote label |
| `Content` | `\[\]FormattedBlock` | — | Footnote content blocks |

---

#### FootnoteAnchor

A footnote anchor reference in markdown text.

Represents a `[^label]` use-site (not a definition).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Label` | `string` | — | The label of the footnote reference (e.g., "1" in `\[^1\]`). |
| `Offset` | `int` | — | Byte offset of the anchor in the markdown text. |

---

#### FootnoteConfig

Configuration for markdown footnote and citation parsing.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `ParseCitations` | `bool` | `true` | Whether to parse the structured citation block (default: true). When enabled, the parser will look for and extract citations from the block after `---` + `<!-- citations ... -->`. |

##### Methods

###### Default()

**Signature:**

```go
func (o *FootnoteConfig) Default() FootnoteConfig
```

**Example:**

```go
result := FootnoteConfig.Default()
```

**Returns:** `FootnoteConfig`

###### WithParseCitations()

Set whether to parse the citation block.

**Signature:**

```go
func (o *FootnoteConfig) WithParseCitations(enabled bool) FootnoteConfig
```

**Example:**

```go
result := instance.WithParseCitations(true)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Enabled` | `bool` | Yes | The enabled |

**Returns:** `FootnoteConfig`

---

#### FootnoteDefinition

A footnote definition from markdown text.

Represents `[^label]: content` declarations (including multi-line continuations).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Label` | `string` | — | The label of the footnote (e.g., "1" in `\[^1\]: ...`). |
| `Content` | `string` | — | The full content of the footnote definition. |
| `Offset` | `int` | — | Byte offset of the definition line in the markdown text. |

---

#### FormattedBlock

Block-level element in a Djot document.

Represents structural elements like headings, paragraphs, lists, code blocks, etc.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `BlockType` | `BlockType` | — | Type of block element |
| `Level` | `*int` | `nil` | Heading level (1-6) for headings, or nesting level for lists |
| `InlineContent` | `\[\]InlineElement` | — | Inline content within the block |
| `Language` | `*string` | `nil` | Language identifier for code blocks |
| `Code` | `*string` | `nil` | Raw code content for code blocks |
| `Children` | `\[\]FormattedBlock` | `/* serde(default) */` | Nested blocks for containers (blockquotes, list items, divs) |

---

#### Formula

A mathematical formula detected and recognized in a document.

Populated by the layout-guided formula pipeline: regions classified as
`LayoutClass.Formula` are routed to the formula OCR task, which returns the
LaTeX source for the region. The field is always present on
`ExtractionResult` but only populated
when the `layout-detection` feature is active and the document contains
formula regions.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Latex` | `string` | — | LaTeX source of the recognized formula, without surrounding `$$` delimiters. This field contains the raw LaTeX code as produced by the OCR backend. To render the formula in Markdown or other formats, wrap with `$$..$$` delimiters as needed. |
| `Bbox` | `BoundingBox` | — | Bounding box of the formula region on its page, in rendered-image pixel coordinates. The coordinates are in the space of the OCR-rendered page image at the OCR DPI (typically 300 DPI). These coordinates are NOT comparable to bounding boxes from native PDF text extraction, which use PDF point coordinates. |
| `Page` | `uint32` | — | 1-indexed page number the formula appears on in the document. This is set by the extraction pipeline based on which page the formula was found on. |

---

#### GridCell

Individual grid cell with position and span metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Content` | `string` | — | Cell text content. |
| `Row` | `uint32` | — | Zero-indexed row position. |
| `Col` | `uint32` | — | Zero-indexed column position. |
| `RowSpan` | `uint32` | `serde(default = "default_span")` | Number of rows this cell spans. |
| `ColSpan` | `uint32` | `serde(default = "default_span")` | Number of columns this cell spans. |
| `IsHeader` | `bool` | `/* serde(default) */` | Whether this is a header cell. |
| `Bbox` | `*BoundingBox` | `nil` | Bounding box for this cell (if available). |

---

#### HeaderMetadata

Header/heading element metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Level` | `uint8` | — | Header level: 1 (h1) through 6 (h6) |
| `Text` | `string` | — | Normalized text content of the header |
| `Id` | `*string` | `nil` | HTML id attribute if present |
| `Depth` | `uint32` | — | Document tree depth at the header element |
| `HtmlOffset` | `uint32` | — | Byte offset in original HTML document |

---

#### HeadingContext

Heading context for a chunk within a Markdown document.

Contains the heading hierarchy from document root to this chunk's section.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Headings` | `\[\]HeadingLevel` | — | The heading hierarchy from document root to this chunk's section. Index 0 is the outermost (h1), last element is the most specific. |

---

#### HeadingLevel

A single heading in the hierarchy.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Level` | `uint8` | — | Heading depth (1 = h1, 2 = h2, etc.) |
| `Text` | `string` | — | The text content of the heading. |

---

#### HeuristicsConfig

Configuration for document chunking and analysis heuristics.

Every threshold is a public field so callers can override any subset via
struct-update syntax: `HeuristicsConfig { text_layer_threshold: 0.5, ..the default constructor }`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `EnablePdfTextHeuristics` | `bool` | `true` | Enable PDF text-layer detection heuristics. When `true`, PDFs with a substantial text layer will skip chunking. Default: `true`. |
| `TextLayerThreshold` | `float32` | `0.7` | Minimum fraction of pages that must have text to skip chunking. Range `0.0..=1.0`. Default: `0.7` (70 % of pages). |
| `FileSizeThresholdBytes` | `uint64` | `10485760` | File size threshold in bytes for considering chunking. Files smaller than this are processed without chunking. Default: 10 MiB (10 × 1 024 × 1 024). |
| `PageCountThreshold` | `uint32` | `50` | Page count threshold for considering chunking. Documents with fewer pages are processed without chunking. Default: 50. |
| `TargetPagesPerChunk` | `uint32` | `10` | Target number of pages per chunk for optimal parallel processing. Default: 10. |
| `MaxPagesPerChunk` | `uint32` | `25` | Hard cap on pages per chunk. No chunk will exceed this limit. Must be ≥ `target_pages_per_chunk`. Default: 25. |
| `DiskProcessingThresholdBytes` | `uint64` | `52428800` | File size threshold for disk-based processing. Files larger than this are buffered to disk to prevent OOM. Default: 50 MiB (50 × 1 024 × 1 024). |
| `MinCharsPerPage` | `uint32` | `50` | Minimum characters per page to consider a page as having text. Default: 50. |
| `MaxXlsxSheetCount` | `uint32` | `200` | Maximum sheet count allowed in an XLSX workbook. Workbooks beyond this are rejected pre-extraction to avoid OOM / abusive billing inflation. Default: 200. |
| `MaxXlsxWorkbookCells` | `uint64` | `5000000` | Maximum cell count (sheets × rows × columns approximation) in an XLSX workbook. Default: 5 000 000 (≈ 200 sheets × 25 k cells). |
| `MaxPptxEmbeddedCount` | `uint32` | `50` | Maximum number of OLE-embedded objects extractable from a single PPTX or DOCX. Protects against zip-bomb-style nested-document abuse. Default: 50. |

##### Methods

###### Default()

**Signature:**

```go
func (o *HeuristicsConfig) Default() HeuristicsConfig
```

**Example:**

```go
result := HeuristicsConfig.Default()
```

**Returns:** `HeuristicsConfig`

###### Validate()

Validate the configuration.

**Errors:**

Returns `HeuristicsError.ConfigError` when:

- `target_pages_per_chunk` is 0
- `max_pages_per_chunk` < `target_pages_per_chunk`
- `file_size_threshold_bytes` is 0

**Signature:**

```go
func (o *HeuristicsConfig) Validate() error
```

**Example:**

```go
if err := instance.Validate(); err != nil {
    return err
}
```

**Returns:** No return value.

**Errors:** Returns `error`.

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

**Example:**

```go
result := HierarchyConfig.Default()
```

**Returns:** `HierarchyConfig`

---

#### HtmlMetadata

HTML metadata extracted from HTML documents.

Includes document-level metadata, Open Graph data, Twitter Card metadata,
and extracted structural elements (headers, links, images, structured data).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Title` | `*string` | `nil` | Document title from `<title>` tag |
| `Description` | `*string` | `nil` | Document description from `<meta name="description">` tag |
| `Keywords` | `\[\]string` | `nil` | Document keywords from `<meta name="keywords">` tag, split on commas |
| `Author` | `*string` | `nil` | Document author from `<meta name="author">` tag |
| `CanonicalUrl` | `*string` | `nil` | Canonical URL from `<link rel="canonical">` tag |
| `BaseHref` | `*string` | `nil` | Base URL from `<base href="">` tag for resolving relative URLs |
| `Language` | `*string` | `nil` | Document language from `lang` attribute |
| `TextDirection` | `*TextDirection` | `nil` | Document text direction from `dir` attribute |
| `OpenGraph` | `map\[string\]string` | `nil` | Open Graph metadata (og:* properties) for social media Keys like "title", "description", "image", "url", etc. |
| `TwitterCard` | `map\[string\]string` | `nil` | Twitter Card metadata (twitter:* properties) Keys like "card", "site", "creator", "title", "description", "image", etc. |
| `MetaTags` | `map\[string\]string` | `nil` | Additional meta tags not covered by specific fields Keys are meta name/property attributes, values are content |
| `Headers` | `\[\]HeaderMetadata` | `nil` | Extracted header elements with hierarchy |
| `Links` | `\[\]LinkMetadata` | `nil` | Extracted hyperlinks with type classification |
| `Images` | `\[\]ImageMetadataType` | `nil` | Extracted images with source and dimensions |
| `StructuredData` | `\[\]StructuredData` | `nil` | Extracted structured data blocks |

---

#### HtmlOutputConfig

Configuration for styled HTML output.

When set on `html_output` alongside
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

**Example:**

```go
result := HtmlOutputConfig.Default()
```

**Returns:** `HtmlOutputConfig`

---

#### ImageExtractionConfig

Image extraction configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `ExtractImages` | `bool` | `true` | Extract images from documents |
| `TargetDpi` | `int32` | `300` | Target DPI for image normalization |
| `MaxImageDimension` | `int32` | `4096` | Maximum dimension for images (width or height) |
| `InjectPlaceholders` | `bool` | `true` | Whether to inject image reference placeholders into markdown output. When `true` (default), image references like `!\[Image 1\](embedded:p1_i0)` are appended to the markdown. Set to `false` to extract images as data without polluting the markdown output. |
| `AutoAdjustDpi` | `bool` | `true` | Automatically adjust DPI based on image content |
| `MinDpi` | `int32` | `72` | Minimum DPI threshold |
| `MaxDpi` | `int32` | `600` | Maximum DPI threshold |
| `MaxImagesPerPage` | `*uint32` | `nil` | Maximum number of image objects to extract per PDF page. Some PDFs (e.g. technical diagrams stored as thousands of raster fragments) can trigger extremely long or indefinite extraction times when every image object on a dense page is decoded individually via the PDF extractor. Setting this limit causes xberg to stop collecting individual images once the count per page reaches the cap and emit a warning instead. `nil` (default) means no limit — all images are extracted. |
| `Classify` | `bool` | `false` | When `true`, extracted images are classified by kind and grouped into clusters where they appear to belong to one figure. Defaults to `false` — opt in explicitly to avoid unexpected ML overhead. |
| `IncludePageRasters` | `bool` | `false` | When `true`, full-page renders produced during OCR preprocessing are captured and returned as `ImageKind.PageRaster` entries in `ExtractionResult.images`. **PDF + OCR only.** No rasters are captured for non-PDF inputs or when the document-level OCR bypass is active (whole-document backend). When OCR is enabled and this flag is set but the active backend skips per-page rendering, a `ProcessingWarning` is emitted in `ExtractionResult.processing_warnings`. Defaults to `false`. Enable when downstream consumers need page thumbnails (e.g. citation previews, visual grounding). |
| `RunOcrOnImages` | `bool` | `true` | Run OCR on extracted images and include the recognized text in the document content. When `true` (default) and `ExtractionConfig.ocr` is configured, extracted images are processed with the configured OCR backend. Set to `false` to extract images without OCR processing, even when OCR is enabled. |
| `OcrTextOnly` | `bool` | `false` | When `true`, image OCR results are rendered as plain text without the `!\[...\](...)` markdown placeholder. Only takes effect when `run_ocr_on_images` is also `true`. |
| `AppendOcrText` | `bool` | `false` | When `true` and `ocr_text_only` is `false`, append the OCR text after the image placeholder in the rendered output. |
| `OutputFormat` | `ImageOutputFormat` | `ImageOutputFormat.Native` | Target format for re-encoding extracted images. When set to anything other than `Native`, each extracted image is re-encoded to the requested format before being returned. This lets callers receive uniform output without duplicating encode logic downstream. Defaults to `Native` — no re-encode pass is performed and `ExtractedImage.format` reflects the source extractor's output. |
| `Svg` | `SvgOptions` | — | SVG-specific knobs for the image-encode pipeline. Controls sanitization and rasterization DPI when the source or output format is SVG.  Only available when the `svg` feature is active. |
| `IncludeDataBase64` | `bool` | `false` | When `true`, populate `ExtractedImage.data_base64` with a Base64-encoded copy of the raw image bytes. Useful for JSON-only clients that cannot efficiently parse the default integer-array serialization of `data`. Defaults to `false`; enabling it doubles the in-memory image representation for the duration of the response. |

##### Methods

###### Default()

**Signature:**

```go
func (o *ImageExtractionConfig) Default() ImageExtractionConfig
```

**Example:**

```go
result := ImageExtractionConfig.Default()
```

**Returns:** `ImageExtractionConfig`

---

#### ImageMetadata

Image metadata extracted from image files.

Includes dimensions, format, and EXIF data.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Width` | `uint32` | — | Image width in pixels |
| `Height` | `uint32` | — | Image height in pixels |
| `Format` | `string` | — | Image format (e.g., "PNG", "JPEG", "TIFF") |
| `Exif` | `map\[string\]string` | `nil` | EXIF metadata tags |

---

#### ImageMetadataType

Image element metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Src` | `string` | — | Image source (URL, data URI, or SVG content) |
| `Alt` | `*string` | `nil` | Alternative text from alt attribute |
| `Title` | `*string` | `nil` | Title attribute |
| `ImageType` | `ImageType` | — | Image type classification |

---

#### ImagePreprocessingConfig

Image preprocessing configuration for OCR.

These settings control how images are preprocessed before OCR to improve
text recognition quality. Different preprocessing strategies work better
for different document types.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `TargetDpi` | `int32` | `300` | Target DPI for the image (300 is standard, 600 for small text). |
| `AutoRotate` | `bool` | `false` | Auto-detect and correct image rotation. |
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

**Example:**

```go
result := ImagePreprocessingConfig.Default()
```

**Returns:** `ImagePreprocessingConfig`

---

#### ImagePreprocessingMetadata

Image preprocessing metadata.

Tracks the transformations applied to an image during OCR preprocessing,
including DPI normalization, resizing, and resampling.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `TargetDpi` | `int32` | — | Target DPI from configuration |
| `ScaleFactor` | `float64` | — | Scaling factor applied to the image |
| `AutoAdjusted` | `bool` | — | Whether DPI was auto-adjusted based on content |
| `FinalDpi` | `int32` | — | Final DPI after processing |
| `ResampleMethod` | `string` | — | Resampling algorithm used ("LANCZOS3", "CATMULLROM", etc.) |
| `DimensionClamped` | `bool` | — | Whether dimensions were clamped to max_image_dimension |
| `CalculatedDpi` | `*int32` | `nil` | Calculated optimal DPI (if auto_adjust_dpi enabled) |
| `SkippedResize` | `bool` | — | Whether resize was skipped (dimensions already optimal) |
| `ResizeError` | `*string` | `nil` | Error message if resize failed |

---

#### InlineElement

Inline element within a block.

Represents text with formatting, links, images, etc.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `ElementType` | `InlineType` | — | Type of inline element |
| `Content` | `string` | — | Text content |
| `Metadata` | `*map\[string\]string` | `nil` | Additional metadata (e.g., href for links, src/alt for images) |

---

#### JatsMetadata

JATS (Journal Article Tag Suite) metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Copyright` | `*string` | `nil` | Copyright statement from the article's `<permissions>` element. |
| `License` | `*string` | `nil` | Open-access license URI from the article's `<license>` element. |
| `HistoryDates` | `map\[string\]string` | `nil` | Publication history dates keyed by event type (e.g. `"received"`, `"accepted"`). |
| `ContributorRoles` | `\[\]ContributorRole` | `nil` | Authors and contributors with their stated roles. |

---

#### Keyword

Extracted keyword with metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Text` | `string` | — | The keyword text. |
| `Score` | `float32` | — | Relevance score (higher is better, algorithm-specific range). |
| `Algorithm` | `KeywordAlgorithm` | — | Algorithm that extracted this keyword. |
| `Positions` | `*\[\]int` | `nil` | Optional positions where keyword appears in text (character offsets). |

---

#### KeywordConfig

Keyword extraction configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Algorithm` | `KeywordAlgorithm` | `KeywordAlgorithm.Yake` | Algorithm to use for extraction. |
| `MaxKeywords` | `int` | `10` | Maximum number of keywords to extract (default: 10). |
| `MinScore` | `float32` | `0` | Minimum score threshold (0.0-1.0, default: 0.0). Keywords with scores below this threshold are filtered out. Note: Score ranges differ between algorithms. |
| `Language` | `*string` | `nil` | Language code for stopword filtering (e.g., "en", "de", "fr"). If None, no stopword filtering is applied. |
| `YakeParams` | `*YakeParams` | `nil` | YAKE-specific tuning parameters. |
| `RakeParams` | `*RakeParams` | `nil` | RAKE-specific tuning parameters. |

##### Methods

###### Default()

**Signature:**

```go
func (o *KeywordConfig) Default() KeywordConfig
```

**Example:**

```go
result := KeywordConfig.Default()
```

**Returns:** `KeywordConfig`

---

#### LanguageDetectionConfig

Language detection configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Enabled` | `bool` | `true` | Enable language detection |
| `MinConfidence` | `float64` | `0.8` | Minimum confidence threshold (0.0-1.0) |
| `DetectMultiple` | `bool` | `false` | Detect multiple languages in the document |

##### Methods

###### Default()

**Signature:**

```go
func (o *LanguageDetectionConfig) Default() LanguageDetectionConfig
```

**Example:**

```go
result := LanguageDetectionConfig.Default()
```

**Returns:** `LanguageDetectionConfig`

---

#### LayoutDetection

A single layout detection result.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `ClassName` | `LayoutClass` | — | Detected layout class (e.g. `Table`, `Text`, `Title`). |
| `Confidence` | `float32` | — | Detection confidence score in `\[0.0, 1.0\]`. |
| `Bbox` | `BBox` | — | Bounding box in image pixel coordinates. |

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
| `EnableChartUnderstanding` | `bool` | `false` | Route regions classified as charts to the chart-understanding OCR task. When `true`, layout regions detected as charts are sent to the VLM chart task (data-series/axis recovery) instead of being treated as generic image regions. Defaults to `false` — chart understanding is opt-in and has no effect on standard text/table extraction scores. |

##### Methods

###### Default()

**Signature:**

```go
func (o *LayoutDetectionConfig) Default() LayoutDetectionConfig
```

**Example:**

```go
result := LayoutDetectionConfig.Default()
```

**Returns:** `LayoutDetectionConfig`

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
| `BoundingBox` | `BoundingBox` | — | Bounding box in document coordinate space. |
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
| `Rel` | `\[\]string` | — | Rel attribute values |

---

#### LlmBackend

liter-llm-backed NER backend.

##### Methods

###### New()

Create a new LLM-backed NER backend with the given LLM configuration.

**Signature:**

```go
func (o *LlmBackend) New(config LlmConfig) LlmBackend
```

**Example:**

```go
result := LlmBackend.New(LlmConfig{})
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Config` | `LlmConfig` | Yes | The configuration options |

**Returns:** `LlmBackend`

###### Detect()

**Signature:**

```go
func (o *LlmBackend) Detect(text string, categories []EntityCategory) ([]Entity, error)
```

**Example:**

```go
result, err := instance.Detect("value", nil)
if err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Text` | `string` | Yes | The text |
| `Categories` | `\[\]EntityCategory` | Yes | The categories |

**Returns:** `[]Entity`

**Errors:** Returns `error`.

###### DetectWithCustom()

**Signature:**

```go
func (o *LlmBackend) DetectWithCustom(text string, categories []EntityCategory, customLabels []string) ([]Entity, error)
```

**Example:**

```go
result, err := instance.DetectWithCustom("value", nil, nil)
if err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Text` | `string` | Yes | The text |
| `Categories` | `\[\]EntityCategory` | Yes | The categories |
| `CustomLabels` | `\[\]string` | Yes | The custom labels |

**Returns:** `[]Entity`

**Errors:** Returns `error`.

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

#### MetaSchema

Compiled meta-schema validator over `preset.schema.json`.

##### Methods

###### Compile()

Compile the given JSON text as a Draft 2020-12 meta-schema.

**Signature:**

```go
func (o *MetaSchema) Compile(metaSchemaJson string) (MetaSchema, error)
```

**Example:**

```go
result, err := MetaSchema.Compile("value")
if err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `MetaSchemaJson` | `string` | Yes | The meta schema json |

**Returns:** `MetaSchema`

**Errors:** Returns `error`.

###### ParsePreset()

Validate `raw` against the meta-schema and deserialize into a `Preset`,
stamping the fingerprint over the canonical file bytes.

**Signature:**

```go
func (o *MetaSchema) ParsePreset(path string, raw []byte) (Preset, error)
```

**Example:**

```go
result, err := instance.ParsePreset("value", []byte("data"))
if err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Path` | `string` | Yes | Path to the file |
| `Raw` | `\[\]byte` | Yes | The raw |

**Returns:** `Preset`

**Errors:** Returns `error`.

---

#### Metadata

Extraction result metadata.

Contains common fields applicable to all formats, format-specific metadata
via a discriminated union, and additional custom fields from postprocessors.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Title` | `*string` | `nil` | Document title |
| `Subject` | `*string` | `nil` | Document subject or description |
| `Authors` | `*\[\]string` | `nil` | Primary author(s) - always Vec for consistency |
| `Keywords` | `*\[\]string` | `nil` | Keywords/tags - always Vec for consistency |
| `Language` | `*string` | `nil` | Primary language (ISO 639 code) |
| `CreatedAt` | `*string` | `nil` | Creation timestamp (ISO 8601 format) |
| `ModifiedAt` | `*string` | `nil` | Last modification timestamp (ISO 8601 format) |
| `CreatedBy` | `*string` | `nil` | User who created the document |
| `ModifiedBy` | `*string` | `nil` | User who last modified the document |
| `Pages` | `*PageStructure` | `nil` | Page/slide/sheet structure with boundaries |
| `Format` | `*FormatMetadata` | `nil` | Format-specific metadata (discriminated union) Contains detailed metadata specific to the document format. Serialized as a nested `"format"` object with a `format_type` discriminator field. |
| `ImagePreprocessing` | `*ImagePreprocessingMetadata` | `nil` | Image preprocessing metadata (when OCR preprocessing was applied) |
| `JsonSchema` | `*interface{}` | `nil` | JSON schema (for structured data extraction) |
| `Error` | `*ErrorMetadata` | `nil` | Error metadata (for batch operations) |
| `ExtractionDurationMs` | `*uint64` | `nil` | Extraction duration in milliseconds (for benchmarking). This field is populated by batch extraction to provide per-file timing information. It's `nil` for single-file extraction (which uses external timing). |
| `Category` | `*string` | `nil` | Document category (from frontmatter or classification). |
| `Tags` | `*\[\]string` | `nil` | Document tags (from frontmatter). |
| `DocumentVersion` | `*string` | `nil` | Document version string (from frontmatter). |
| `AbstractText` | `*string` | `nil` | Abstract or summary text (from frontmatter). |
| `OutputFormat` | `*string` | `nil` | Output format identifier (e.g., "markdown", "html", "text"). Set by the output format pipeline stage when format conversion is applied. Previously stored in `metadata.additional\["output_format"\]`. |
| `OcrUsed` | `bool` | — | Whether OCR was used during extraction. Set to `true` whenever the extraction pipeline ran an OCR backend (Tesseract, PaddleOCR, VLM, etc.) and used that output as the primary or fallback text. `false` means native text extraction was used exclusively. |
| `Additional` | `map\[string\]interface{}` | `nil` | Additional custom fields from postprocessors. Serialized as a nested `"additional"` object (not flattened at root level). Uses `Cow<'static, str>` keys so static string keys avoid allocation. |

##### Methods

###### IsEmpty()

Returns `true` when no metadata fields, format-specific metadata, or
additional postprocessor fields are populated.

**Signature:**

```go
func (o *Metadata) IsEmpty() bool
```

**Example:**

```go
result := instance.IsEmpty()
```

**Returns:** `bool`

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

#### MultidocInput

Input signals for multi-document boundary detection.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `PageCount` | `uint32` | — | Total number of pages in the PDF. |
| `Pages` | `\[\]PageSignals` | — | Per-page signals extracted from the PDF. |

---

#### MultidocThresholds

Thresholds for multi-document boundary detection.

All fields are public; callers override any subset via struct-update syntax.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `DensityShiftThreshold` | `float32` | `0.3` | Text density difference threshold for `DensityShift` detection. Default: 0.3. |
| `BigramOverlapMin` | `float32` | `0.1` | Minimum bigram-overlap ratio below which a density shift is promoted to a `DensityShift` boundary.  Default: 0.1 (10 % overlap). |

##### Methods

###### Default()

**Signature:**

```go
func (o *MultidocThresholds) Default() MultidocThresholds
```

**Example:**

```go
result := MultidocThresholds.Default()
```

**Returns:** `MultidocThresholds`

---

#### NerConfig

**Since:** `v5.0`

Configuration for the NER post-processor.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Backend` | `NerBackendKind` | `NerBackendKind.Onnx` | Backend that runs the entity detection. |
| `Categories` | `\[\]EntityCategory` | `nil` | Entity categories to detect. Defaults to a sensible PERSON/ORG/LOCATION/EMAIL set when empty. |
| `Model` | `*string` | `nil` | Override the default model — only used by `NerBackendKind.Onnx`. `nil` lets the backend pick its pinned default xberg GLiNER model alias. |
| `Llm` | `*LlmConfig` | `nil` | Optional LLM configuration — only used by `NerBackendKind.Llm`. Token usage for LLM backends is recorded in `ExtractionResult.llm_usage`. |
| `CustomLabels` | `\[\]string` | `nil` | Arbitrary user-supplied entity labels for zero-shot detection. `xberg-gliner` natively supports zero-shot inference over caller-supplied labels. The LLM backend also honours these labels by including them in the structured-output schema. Custom labels surface as `EntityCategory.Custom` in the resulting `Entity` stream. Use this when you need domain-specific entity types (e.g. `"Treatment"`, `"Product"`, `"Vessel"`) without forking GLiNER's taxonomy. |

---

#### OcrBackend

Trait for OCR backend plugins.

Implement this trait to add custom OCR capabilities. OCR backends can be:

- Native Rust implementations (like Tesseract)
- FFI bridges to Python libraries (like EasyOCR, PaddleOCR)
- Cloud-based OCR services (Google Vision, AWS Textract, etc.)

##### Thread Safety

OCR backends must be thread-safe (`Send + Sync`) to support concurrent processing.

##### Methods

###### ProcessImage()

Process an image and extract text via OCR.

**Returns:**

An `ExtractionResult` containing the extracted text and metadata.

**Errors:**

- `XbergError.Ocr` - OCR processing failed
- `XbergError.Validation` - Invalid image format or configuration
- `XbergError.Io` - I/O errors (these always bubble up)

##### Reading `backend_options`

Backends that support runtime tuning can read `config.backend_options` and
deserialize only the keys they care about. Unknown keys are silently ignored,
so multiple backends can coexist in a pipeline without key conflicts.

**Signature:**

```go
func (o *OcrBackend) ProcessImage(imageBytes []byte, config OcrConfig) (ExtractionResult, error)
```

**Example:**

```go
result, err := instance.ProcessImage([]byte("data"), OcrConfig{})
if err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `ImageBytes` | `\[\]byte` | Yes | Raw image data (JPEG, PNG, TIFF, etc.) |
| `Config` | `OcrConfig` | Yes | OCR configuration (language, PSM mode, etc.) |

**Returns:** `ExtractionResult`

**Errors:** Returns `error`.

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

**Example:**

```go
result, err := instance.ProcessImageFile("value", OcrConfig{})
if err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Path` | `string` | Yes | Path to the image file |
| `Config` | `OcrConfig` | Yes | OCR configuration |

**Returns:** `ExtractionResult`

**Errors:** Returns `error`.

###### SupportsLanguage()

Check if this backend supports a given language code.

**Returns:**

`true` if the language is supported, `false` otherwise.

**Signature:**

```go
func (o *OcrBackend) SupportsLanguage(lang string) bool
```

**Example:**

```go
result := instance.SupportsLanguage("value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Lang` | `string` | Yes | ISO 639-2/3 language code (e.g., "eng", "deu", "fra") |

**Returns:** `bool`

###### BackendType()

Get the backend type identifier.

**Returns:**

The backend type enum value.

**Signature:**

```go
func (o *OcrBackend) BackendType() OcrBackendType
```

**Example:**

```go
result := instance.BackendType()
```

**Returns:** `OcrBackendType`

###### SupportedLanguages()

Optional: Get a list of all supported languages.

Defaults to empty list. Override to provide comprehensive language support info.

**Signature:**

```go
func (o *OcrBackend) SupportedLanguages() []string
```

**Example:**

```go
result := instance.SupportedLanguages()
```

**Returns:** `[]string`

###### SupportsTableDetection()

Optional: Check if the backend supports table detection.

Defaults to `false`. Override if your backend can detect and extract tables.

**Signature:**

```go
func (o *OcrBackend) SupportsTableDetection() bool
```

**Example:**

```go
result := instance.SupportsTableDetection()
```

**Returns:** `bool`

###### SupportsDocumentProcessing()

Check if the backend supports direct document-level processing (e.g. for PDFs).

Defaults to `false`. Override if the backend has optimized document processing.

**Signature:**

```go
func (o *OcrBackend) SupportsDocumentProcessing() bool
```

**Example:**

```go
result := instance.SupportsDocumentProcessing()
```

**Returns:** `bool`

###### EmitsStructuredMarkdown()

Declare that this backend emits structured markdown directly (tables, headings, lists)
and downstream layout reconstruction should be skipped.

Defaults to `false` — classical OCR backends (Tesseract, PaddleOCR classical) return
plain text per detected region. End-to-end VLM backends (PaddleOCR-VL, GOT-OCR 2.0)
emit markdown in one forward pass and should override this to `true`.

**Signature:**

```go
func (o *OcrBackend) EmitsStructuredMarkdown() bool
```

**Example:**

```go
result := instance.EmitsStructuredMarkdown()
```

**Returns:** `bool`

###### ProcessDocument()

Process a document file directly via OCR.

Only called if `supports_document_processing` returns `true`.

**Signature:**

```go
func (o *OcrBackend) ProcessDocument(path string, config OcrConfig) (ExtractionResult, error)
```

**Example:**

```go
result, err := instance.ProcessDocument("value", OcrConfig{})
if err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Path` | `string` | Yes | The  path |
| `Config` | `OcrConfig` | Yes | The ocr config |

**Returns:** `ExtractionResult`

**Errors:** Returns `error`.

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
| `Language` | `\[\]string` | `nil` | Language code(s) for OCR recognition. Accepts either a single language code ("eng") or a list (\["eng", "deu"\]). Defaults to \["eng"\]. For Tesseract, languages are joined with "+". |
| `TesseractConfig` | `*TesseractConfig` | `nil` | Tesseract-specific configuration (optional) |
| `OutputFormat` | `*OutputFormat` | `nil` | Output format for OCR results (optional, for format conversion) |
| `PaddleOcrConfig` | `*interface{}` | `nil` | PaddleOCR-specific configuration (optional, JSON passthrough) |
| `BackendOptions` | `*interface{}` | `nil` | Arbitrary per-call options passed through to the backend unchanged. Custom OCR backends and built-in backends that support runtime tuning can read this value and deserialize the keys they care about. Keys unknown to the backend are silently ignored. This is the recommended extension point for per-call parameters that are not covered by the typed fields above (e.g. mode switching, preprocessing flags, inference batch size). **Scope:** when `pipeline` is `nil`, this value is propagated to the primary stage of the auto-constructed pipeline. When `pipeline` is explicitly set, this field has **no effect** — the caller must set `OcrPipelineStage.backend_options` directly on the relevant stage(s) instead. Example: ```json { "mode": "fast", "enable_layout": true, "timeout_ms": 5000 } ``` |
| `ElementConfig` | `*OcrElementConfig` | `nil` | OCR element extraction configuration |
| `QualityThresholds` | `*OcrQualityThresholds` | `nil` | Quality thresholds for the native-text-to-OCR fallback decision. When None, uses compiled defaults (matching previous hardcoded behavior). |
| `Pipeline` | `*OcrPipelineConfig` | `nil` | Multi-backend OCR pipeline configuration. When set, enables weighted fallback across multiple OCR backends based on output quality. When None, uses the single `backend` field (same as today). |
| `AutoRotate` | `bool` | `false` | Enable automatic page rotation based on orientation detection. When enabled, uses Tesseract's `DetectOrientationScript()` to detect page orientation (0/90/180/270 degrees) before OCR. If the page is rotated with high confidence, the image is corrected before recognition. This is critical for handling rotated scanned documents. |
| `VlmFallback` | `VlmFallbackPolicy` | `VlmFallbackPolicy.Disabled` | Ergonomic VLM fallback policy. When set to anything other than `VlmFallbackPolicy.Disabled` and `OcrConfig.pipeline` is `nil`, a multi-stage pipeline is synthesised automatically: - `VlmFallbackPolicy.OnLowQuality` → `\[classical_stage, vlm_stage\]` with the `quality_threshold` mapped onto `OcrQualityThresholds.pipeline_min_quality`. - `VlmFallbackPolicy.Always` → `\[vlm_stage\]` only. Requires `OcrConfig.vlm_config` to be `Some` when not `Disabled`. When `OcrConfig.pipeline` is explicitly set, this field is ignored. |
| `VlmConfig` | `*LlmConfig` | `nil` | VLM (Vision Language Model) OCR configuration. Required when `backend` is `"vlm"` or when `vlm_fallback` is not `VlmFallbackPolicy.Disabled`. Uses liter-llm to send page images to a vision model for text extraction. |
| `VlmPrompt` | `*string` | `nil` | Custom Jinja2 prompt template for VLM OCR. When `nil`, uses the default template. Available variables: - `{{ language }}` — The document language code (e.g., "eng", "deu"). |
| `Acceleration` | `*AccelerationConfig` | `nil` | Hardware acceleration for ONNX Runtime models (e.g. PaddleOCR, layout detection). Not user-configurable via config files — injected at runtime from `ExtractionConfig.acceleration` before each `process_image` call. |
| `TessdataBytes` | `*map\[string\]\[\]byte` | `nil` | Caller-supplied Tesseract `traineddata` bytes per language code. Primary use case is the WASM build, which has no filesystem and cannot download tessdata at runtime. Native builds typically rely on `TessdataManager` and ignore this field. When present, the WASM Tesseract backend prefers these bytes over its compile-time-bundled English data. Skipped by serde to keep config files small — supply via the typed API at runtime. |
| `TessdataPath` | `*string` | `nil` | Runtime override for tessdata directory path. When set, uses this path as the highest-priority tessdata location, bypassing environment variables and cache directories. Useful for embedding pre-installed tessdata in applications. When `nil`, uses the standard resolution chain: TESSDATA_PREFIX env, cache dir, system paths. |

##### Methods

###### Default()

**Signature:**

```go
func (o *OcrConfig) Default() OcrConfig
```

**Example:**

```go
result := OcrConfig.Default()
```

**Returns:** `OcrConfig`

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
| `PageNumber` | `uint32` | — | Page number (1-indexed). |
| `ParentId` | `*string` | `nil` | Parent element ID for hierarchical relationships. Only used for Tesseract output which has word -> line -> block hierarchy. |
| `BackendMetadata` | `map\[string\]interface{}` | `nil` | Backend-specific metadata that doesn't fit the unified schema. |

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
| `Metadata` | `map\[string\]interface{}` | — | OCR processing metadata (confidence scores, language, etc.) |
| `Tables` | `\[\]OcrTable` | — | Tables detected and extracted via OCR |
| `OcrElements` | `*\[\]OcrElement` | `/* serde(default) */` | Structured OCR elements with bounding boxes and confidence scores. Available when TSV output is requested or table detection is enabled. |

---

#### OcrMetadata

OCR processing metadata.

Captures information about OCR processing configuration and results.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Language` | `string` | — | OCR language code(s) used |
| `Psm` | `int32` | — | Tesseract Page Segmentation Mode (PSM) |
| `OutputFormat` | `string` | — | Output format (e.g., "text", "hocr") |
| `TableCount` | `uint32` | — | Number of tables detected |
| `TableRows` | `*uint32` | `nil` | Number of rows in the detected table (if a single table was found). |
| `TableCols` | `*uint32` | `nil` | Number of columns in the detected table (if a single table was found). |

---

#### OcrPipelineConfig

Multi-backend OCR pipeline with quality-based fallback.

Backends are tried in priority order (highest first). After each backend
produces output, quality is evaluated. If it meets `quality_thresholds.pipeline_min_quality`,
the result is accepted. Otherwise the next backend is tried.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Stages` | `\[\]OcrPipelineStage` | — | Ordered list of backends to try. Sorted by priority (descending) at runtime. |
| `QualityThresholds` | `OcrQualityThresholds` | `/* serde(default) */` | Quality thresholds for deciding whether to accept a result or try the next backend. |

---

#### OcrPipelineStage

A single backend stage in the OCR pipeline.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Backend` | `string` | — | Backend name: "tesseract", "paddleocr", "easyocr", or a custom registered name. |
| `Priority` | `uint32` | `serde(default = "default_priority")` | Priority weight (higher = tried first). Stages are sorted by priority descending. |
| `Language` | `*\[\]string` | `/* serde(default) */` | Language override for this stage (None = use parent OcrConfig.language). Accepts either a single language code ("eng") or a list (\["eng", "deu"\]). |
| `TesseractConfig` | `*TesseractConfig` | `/* serde(default) */` | Tesseract-specific config override for this stage. |
| `PaddleOcrConfig` | `*interface{}` | `/* serde(default) */` | PaddleOCR-specific config for this stage. |
| `VlmConfig` | `*LlmConfig` | `/* serde(default) */` | VLM config override for this pipeline stage. |
| `BackendOptions` | `*interface{}` | `/* serde(default) */` | Arbitrary per-call options passed through to the backend unchanged. Backends that support runtime tuning (mode switching, preprocessing flags, inference parameters, etc.) read this value and deserialize the keys they care about. Keys unknown to the backend are silently ignored, so options from different backends can coexist in the same config without conflict. Example (custom backend): ```json { "mode": "fast", "enable_layout": true } ``` |

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

**Example:**

```go
result := OcrQualityThresholds.Default()
```

**Returns:** `OcrQualityThresholds`

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
| `Cells` | `\[\]\[\]string` | — | Table cells as a 2D vector (rows × columns) |
| `Markdown` | `string` | — | Markdown representation of the table |
| `PageNumber` | `uint32` | — | Page number where the table was found (1-indexed) |
| `BoundingBox` | `*OcrTableBoundingBox` | `/* serde(default) */` | Bounding box of the table in pixel coordinates (from OCR word positions). |

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

**Example:**

```go
result := instance.WithCacheDir("value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Path` | `string` | Yes | Path to cache directory |

**Returns:** `PaddleOcrConfig`

###### WithTableDetection()

Enables or disables table structure detection.

**Signature:**

```go
func (o *PaddleOcrConfig) WithTableDetection(enable bool) PaddleOcrConfig
```

**Example:**

```go
result := instance.WithTableDetection(true)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Enable` | `bool` | Yes | Whether to enable table detection |

**Returns:** `PaddleOcrConfig`

###### WithAngleCls()

Enables or disables angle classification for rotated text.

**Signature:**

```go
func (o *PaddleOcrConfig) WithAngleCls(enable bool) PaddleOcrConfig
```

**Example:**

```go
result := instance.WithAngleCls(true)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Enable` | `bool` | Yes | Whether to enable angle classification |

**Returns:** `PaddleOcrConfig`

###### WithDetDbThresh()

Sets the database threshold for text detection.

**Signature:**

```go
func (o *PaddleOcrConfig) WithDetDbThresh(threshold float32) PaddleOcrConfig
```

**Example:**

```go
result := instance.WithDetDbThresh(0.5)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Threshold` | `float32` | Yes | Detection threshold (0.0-1.0) |

**Returns:** `PaddleOcrConfig`

###### WithDetDbBoxThresh()

Sets the box threshold for text bounding box refinement.

**Signature:**

```go
func (o *PaddleOcrConfig) WithDetDbBoxThresh(threshold float32) PaddleOcrConfig
```

**Example:**

```go
result := instance.WithDetDbBoxThresh(0.5)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Threshold` | `float32` | Yes | Box threshold (0.0-1.0) |

**Returns:** `PaddleOcrConfig`

###### WithDetDbUnclipRatio()

Sets the unclip ratio for expanding text bounding boxes.

**Signature:**

```go
func (o *PaddleOcrConfig) WithDetDbUnclipRatio(ratio float32) PaddleOcrConfig
```

**Example:**

```go
result := instance.WithDetDbUnclipRatio(0.5)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Ratio` | `float32` | Yes | Unclip ratio (typically 1.5-2.0) |

**Returns:** `PaddleOcrConfig`

###### WithDetLimitSideLen()

Sets the maximum side length for detection images.

**Signature:**

```go
func (o *PaddleOcrConfig) WithDetLimitSideLen(length uint32) PaddleOcrConfig
```

**Example:**

```go
result := instance.WithDetLimitSideLen(42)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Length` | `uint32` | Yes | Maximum side length in pixels |

**Returns:** `PaddleOcrConfig`

###### WithRecBatchNum()

Sets the batch size for recognition inference.

**Signature:**

```go
func (o *PaddleOcrConfig) WithRecBatchNum(batchSize uint32) PaddleOcrConfig
```

**Example:**

```go
result := instance.WithRecBatchNum(42)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `BatchSize` | `uint32` | Yes | Number of text regions to process simultaneously |

**Returns:** `PaddleOcrConfig`

###### WithDropScore()

Sets the minimum recognition confidence threshold.

**Signature:**

```go
func (o *PaddleOcrConfig) WithDropScore(score float32) PaddleOcrConfig
```

**Example:**

```go
result := instance.WithDropScore(0.5)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Score` | `float32` | Yes | Minimum confidence (0.0-1.0), text below this is dropped |

**Returns:** `PaddleOcrConfig`

###### WithPadding()

Sets padding in pixels added around images before detection.

**Signature:**

```go
func (o *PaddleOcrConfig) WithPadding(padding uint32) PaddleOcrConfig
```

**Example:**

```go
result := instance.WithPadding(42)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Padding` | `uint32` | Yes | Padding in pixels (0-100) |

**Returns:** `PaddleOcrConfig`

###### WithModelTier()

Sets the model tier controlling detection/recognition model size.

**Signature:**

```go
func (o *PaddleOcrConfig) WithModelTier(tier string) PaddleOcrConfig
```

**Example:**

```go
result := instance.WithModelTier("value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Tier` | `string` | Yes | `"mobile"` (default, lightweight, faster) or `"server"` (high accuracy, GPU/complex documents) |

**Returns:** `PaddleOcrConfig`

###### Default()

Creates a default configuration with English language support.

**Signature:**

```go
func (o *PaddleOcrConfig) Default() PaddleOcrConfig
```

**Example:**

```go
result := PaddleOcrConfig.Default()
```

**Returns:** `PaddleOcrConfig`

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
| `PageNumber` | `uint32` | — | Page number (1-indexed) |

---

#### PageClassification

Classification result for a single page.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `PageNumber` | `uint32` | — | 1-indexed page number this classification belongs to. |
| `Labels` | `\[\]ClassificationLabel` | — | Labels assigned to the page. Single-label classification yields exactly one entry; multi-label classification yields any subset of the configured label set. |

---

#### PageClassificationConfig

**Since:** `v5.0`

Configuration for the page-classification post-processor.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `PromptTemplate` | `*string` | `nil` | Minijinja prompt template. Receives `{{ labels }}` (joined list), `{{ page_text }}` and `{{ multi_label }}` variables. `nil` lets the backend pick a sensible default. |
| `Labels` | `\[\]string` | — | The set of labels the classifier may emit. Must contain at least one entry. |
| `MultiLabel` | `bool` | `/* serde(default) */` | Allow multiple labels per page. Single-label mode returns at most one label. |
| `Llm` | `LlmConfig` | — | LLM configuration used for classification. |

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
| `MarkerFormat` | `string` | `"<!-- PAGE {page_num} -->"` | Page marker format (use {page_num} placeholder) Default: "\n\n<!-- PAGE {page_num} -->\n\n" |

##### Methods

###### Default()

**Signature:**

```go
func (o *PageConfig) Default() PageConfig
```

**Example:**

```go
result := PageConfig.Default()
```

**Returns:** `PageConfig`

---

#### PageContent

Content for a single page/slide.

When page extraction is enabled, documents are split into per-page content
with associated tables and images mapped to each page.

##### Performance

Uses shared tables and images for memory efficiency:

- `[]Table` enables zero-copy sharing of table data
- `[]ExtractedImage` enables zero-copy sharing of image data
- Maintains exact JSON compatibility via custom Serialize/Deserialize

This reduces memory overhead for documents with shared tables/images
by avoiding redundant copies during serialization.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `PageNumber` | `uint32` | — | Page number (1-indexed) |
| `Content` | `string` | — | Text content for this page |
| `Tables` | `\[\]Table` | `/* serde(default) */` | Tables found on this page (uses Arc for memory efficiency) Serializes as \[\]Table for JSON compatibility while maintaining shared in-memory ownership for zero-copy sharing. |
| `ImageIndices` | `\[\]uint32` | `/* serde(default) */` | Indices into `ExtractionResult.images` for images found on this page. Each value is a zero-based index into the top-level `images` collection. Only populated when `extract_images = true` in the extraction config. |
| `Hierarchy` | `*PageHierarchy` | `nil` | Hierarchy information for the page (when hierarchy extraction is enabled) Contains text hierarchy levels (H1-H6) extracted from the page content. |
| `IsBlank` | `*bool` | `nil` | Whether this page is blank (no meaningful text content) Determined during extraction based on text content analysis. A page is blank if it has fewer than 3 non-whitespace characters and contains no tables or images. |
| `LayoutRegions` | `*\[\]LayoutRegion` | `nil` | Layout detection regions for this page (when layout detection is enabled). Contains detected layout regions with class, confidence, bounding box, and area fraction. Only populated when layout detection is configured. |
| `SpeakerNotes` | `*string` | `nil` | Speaker notes for this slide (PPTX only). Contains the text from the slide's notes pane (`ppt/notesSlides/notesSlide{N}.xml`). Only populated when the source is a PPTX file and notes are present. |
| `SectionName` | `*string` | `nil` | Section name this slide belongs to (PPTX only). PowerPoint sections group slides into logical chapters (`<p:sectionLst>` in `ppt/presentation.xml`). Only populated when the source is a PPTX file and the slide belongs to a named section. |
| `SheetName` | `*string` | `nil` | Sheet name for this page (XLSX/ODS only). Each spreadsheet sheet maps to one `PageContent` entry. This field carries the sheet's display name as it appears in the workbook. `nil` for all non-spreadsheet formats and for sheets with an empty name. |

---

#### PageHierarchy

Page hierarchy structure containing heading levels and block information.

Used when PDF text hierarchy extraction is enabled. Contains hierarchical
blocks with heading levels (H1-H6) for semantic document structure.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `BlockCount` | `uint32` | — | Number of hierarchy blocks on this page |
| `Blocks` | `\[\]HierarchicalBlock` | `/* serde(default) */` | Hierarchical blocks with heading levels |

---

#### PageInfo

Metadata for individual page/slide/sheet.

Captures per-page information including dimensions, content counts,
and visibility state (for presentations).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Number` | `uint32` | — | Page number (1-indexed) |
| `Title` | `*string` | `nil` | Page title (usually for presentations) |
| `ImageCount` | `*uint32` | `nil` | Number of images on this page |
| `TableCount` | `*uint32` | `nil` | Number of tables on this page |
| `Hidden` | `*bool` | `nil` | Whether this page is hidden (e.g., in presentations) |
| `IsBlank` | `*bool` | `nil` | Whether this page is blank (no meaningful text, no images, no tables) A page is considered blank if it has fewer than 3 non-whitespace characters and contains no tables or images. This is useful for filtering out empty pages in scanned documents or PDFs with blank separator pages. |
| `HasVectorGraphics` | `bool` | `/* serde(default) */` | Whether this page contains non-trivial vector graphics (paths, shapes, curves) Indicates the presence of vector-drawn content such as charts, diagrams, or geometric shapes (e.g., from Adobe InDesign, LaTeX TikZ). These are invisible to `ExtractionResult.images` since they are not embedded as raster XObjects. Set to `true` when path count exceeds a heuristic threshold, signaling that downstream consumers may want to rasterize the page to capture this content. Only populated for PDFs; `nil` for other document types. |

---

#### PageRange

Page range for a chunk (0-indexed, inclusive).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Start` | `uint32` | — | Start page (0-indexed, inclusive). |
| `End` | `uint32` | — | End page (0-indexed, inclusive). |

##### Methods

###### PageCount()

Get the number of pages in this range.

**Signature:**

```go
func (o *PageRange) PageCount() uint32
```

**Example:**

```go
result := instance.PageCount()
```

**Returns:** `uint32`

---

#### PageSignals

Per-page signals extracted from PDF content.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `PageNumber` | `uint32` | — | 1-indexed page number. |
| `TextExcerpt` | `string` | — | First ~500 characters of extracted text. |
| `StartsWithLetterheadLike` | `bool` | — | `true` if page starts with letterhead-like content (ALL CAPS line in first 5 lines or a logo-image bbox at top). |
| `HasPageNumberOneMarker` | `bool` | — | `true` if text contains "Page 1" or "1 of N" pattern. |
| `HasSignatureBlock` | `bool` | — | `true` if text contains signature indicators ("Sincerely", "Signed") or a signature image bbox. |
| `LayoutTextDensity` | `float32` | — | Text density: characters per page area, normalised to `\[0.0, 1.0\]`. |

##### Methods

###### FromPageText()

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

```go
func (o *PageSignals) FromPageText(pageNumber uint32, text string, layoutTextDensity float32) PageSignals
```

**Example:**

```go
result := PageSignals.FromPageText(42, "value", 0.5)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `PageNumber` | `uint32` | Yes | The page number |
| `Text` | `string` | Yes | The text |
| `LayoutTextDensity` | `float32` | Yes | The layout text density |

**Returns:** `PageSignals`

---

#### PageStructure

Unified page structure for documents.

Supports different page types (PDF pages, PPTX slides, Excel sheets)
with character offset boundaries for chunk-to-page mapping.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `TotalCount` | `uint32` | — | Total number of pages/slides/sheets |
| `UnitType` | `PageUnitType` | — | Type of paginated unit |
| `Boundaries` | `*\[\]PageBoundary` | `nil` | Character offset boundaries for each page Maps character ranges in the extracted content to page numbers. Used for chunk page range calculation. |
| `Pages` | `*\[\]PageInfo` | `nil` | Detailed per-page metadata (optional, only when needed) |

---

#### PatternMatch

One detected PII span in the input text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Start` | `int` | — | Inclusive byte-offset start of the match in the source text. |
| `End` | `int` | — | Exclusive byte-offset end of the match. |
| `Category` | `PiiCategory` | — | Category the match belongs to. |
| `Text` | `string` | — | Matched substring (owned copy — pattern engine returns owned data so the caller can free the original text if needed before replacement). |

---

#### PdfAnnotation

A PDF annotation extracted from a document page.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `AnnotationType` | `PdfAnnotationType` | — | The type of annotation. |
| `Content` | `*string` | `nil` | Text content of the annotation (e.g., comment text, link URL). |
| `PageNumber` | `uint32` | — | Page number where the annotation appears (1-indexed). |
| `BoundingBox` | `*BoundingBox` | `nil` | Bounding box of the annotation on the page. |

---

#### PdfConfig

PDF-specific configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `ExtractImages` | `bool` | `false` | Extract images from PDF |
| `ExtractTables` | `bool` | `true` | Extract tables from PDF. When `true` (default), runs pdf_oxide's native grid detector and, if it finds nothing, falls back to the heuristic text-layer reconstruction in `pdf.oxide.table.extract_tables_heuristic`. Set to `false` to skip both passes — `tables` will then be empty in the result. |
| `Passwords` | `*\[\]string` | `nil` | List of passwords to try when opening encrypted PDFs |
| `ExtractMetadata` | `bool` | `true` | Extract PDF metadata |
| `Hierarchy` | `*HierarchyConfig` | `nil` | Hierarchy extraction configuration (None = hierarchy extraction disabled) |
| `ExtractAnnotations` | `bool` | `false` | Extract PDF annotations (text notes, highlights, links, stamps). Default: false |
| `TopMarginFraction` | `*float32` | `nil` | Top margin fraction (0.0–1.0) of page height to exclude headers/running heads. Default: 0.06 (6%) |
| `BottomMarginFraction` | `*float32` | `nil` | Bottom margin fraction (0.0–1.0) of page height to exclude footers/page numbers. Default: 0.05 (5%) |
| `AllowSingleColumnTables` | `bool` | `false` | Allow single-column pseudo tables in extraction results. By default, tables with fewer than 2 columns (layout-guided) or 3 columns (heuristic) are rejected. When `true`, the minimum column count is relaxed to 1, allowing single-column structured data (glossaries, itemized lists) to be emitted as tables. Other quality filters (density, sparsity, prose detection) still apply. |
| `OcrInlineImages` | `bool` | `false` | Perform OCR on inline images extracted from PDF pages and attach the recognized text to each `ExtractedImage.ocr_result`. Requires Tesseract to be available; if `ExtractionConfig.ocr` is `nil` the extractor falls back to `TesseractConfig.default()`. Per-image failures degrade gracefully (the image is returned without OCR text rather than failing the whole extraction). Default: `false`. |
| `ExtractFormFields` | `bool` | `true` | Extract AcroForm and XFA form fields into `ExtractionResult.form_fields`. When `true` (default), reads the document's interactive form structure (field names, types, values, widget geometry). Cheap and strictly additive — non-form PDFs simply yield an empty list. Set to `false` to skip the form pass entirely. |
| `ReadingOrder` | `bool` | `false` | Reorder extracted text by layout-detected reading order. When `true`, projects text spans onto layout-detected regions, performs column detection, and emits spans in natural reading order (important for multi-column academic PDFs). Requires the `layout-detection` feature; has no effect without it. Defaults to `false`. |

##### Methods

###### Default()

**Signature:**

```go
func (o *PdfConfig) Default() PdfConfig
```

**Example:**

```go
result := PdfConfig.Default()
```

**Returns:** `PdfConfig`

---

#### PdfFormField

A form field extracted from a PDF's AcroForm or XFA structure.

Populated by the PDF extractor when `PdfConfig.extract_form_fields` is
enabled and the document is a fillable form. Supports both AcroForm (standard)
and XFA (XML Forms Architecture) layers. When both are present, AcroForm fields
take priority (canonical fallback per PDF spec), and XFA-only fields are appended.
The collection is empty for non-form PDFs and for non-PDF formats.

`PdfConfig.extract_form_fields`: crate.core.config.PdfConfig.extract_form_fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Name` | `string` | — | Partial field name (the leaf name within the field hierarchy). |
| `FullName` | `string` | — | Fully-qualified field name (dotted path from the form root). |
| `FieldType` | `FormFieldType` | — | Classified field type. |
| `Value` | `*string` | `/* serde(default) */` | Current field value, if any. |
| `DefaultValue` | `*string` | `/* serde(default) */` | Default field value, if any. |
| `Flags` | `uint32` | `/* serde(default) */` | Raw field-flags bitmask (read-only, required, multiline, …). |
| `Page` | `*uint32` | `/* serde(default) */` | 1-indexed page the field's widget appears on. Currently always `nil` for AcroForm fields; page assignment is a deferred enhancement requiring spatial analysis of widget annotations per page. |
| `Bbox` | `*BoundingBox` | `/* serde(default) */` | Widget bounding box on its page, if known. |
| `MaxLength` | `*uint32` | `/* serde(default) */` | Maximum input length for text fields, if specified. |
| `Tooltip` | `*string` | `/* serde(default) */` | Tooltip / alternate field description, if present. |

---

#### PdfMetadata

PDF-specific metadata.

Contains metadata fields specific to PDF documents that are not in the common
`Metadata` structure. Common fields like title, authors, keywords, and dates
are at the `Metadata` level.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `PdfVersion` | `*string` | `nil` | PDF version (e.g., "1.7", "2.0") |
| `Producer` | `*string` | `nil` | PDF producer (application that created the PDF) |
| `IsEncrypted` | `*bool` | `nil` | Whether the PDF is encrypted/password-protected |
| `Width` | `*int64` | `nil` | First page width in points (1/72 inch) |
| `Height` | `*int64` | `nil` | First page height in points (1/72 inch) |
| `PageCount` | `*uint32` | `nil` | Total number of pages in the PDF document |

---

#### Plugin

Base trait that all plugins must implement.

This trait provides common functionality for plugin lifecycle management,
identification, and metadata.

##### Thread Safety

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

**Example:**

```go
result := instance.Name()
```

**Returns:** `string`

###### Version()

Returns the semantic version of this plugin.

Should follow semver format: `MAJOR.MINOR.PATCH`

Defaults to the xberg crate version.

**Signature:**

```go
func (o *Plugin) Version() string
```

**Example:**

```go
result := instance.Version()
```

**Returns:** `string`

###### Initialize()

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

```go
func (o *Plugin) Initialize() error
```

**Example:**

```go
if err := instance.Initialize(); err != nil {
    return err
}
```

**Returns:** No return value.

**Errors:** Returns `error`.

###### Shutdown()

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

```go
func (o *Plugin) Shutdown() error
```

**Example:**

```go
if err := instance.Shutdown(); err != nil {
    return err
}
```

**Returns:** No return value.

**Errors:** Returns `error`.

###### Description()

Optional plugin description for debugging and logging.

Defaults to empty string if not overridden.

**Signature:**

```go
func (o *Plugin) Description() string
```

**Example:**

```go
result := instance.Description()
```

**Returns:** `string`

###### Author()

Optional plugin author information.

Defaults to empty string if not overridden.

**Signature:**

```go
func (o *Plugin) Author() string
```

**Example:**

```go
result := instance.Author()
```

**Returns:** `string`

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

##### Performance

This signature avoids unnecessary cloning of large extraction results by
taking a mutable reference instead of ownership. Processors modify the
result in place.

##### Example - Language Detection

##### Example - Text Cleaning

**Signature:**

```go
func (o *PostProcessor) Process(result ExtractionResult, config ExtractionConfig) error
```

**Example:**

```go
if err := instance.Process(ExtractionResult{}, ExtractionConfig{}); err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Result` | `ExtractionResult` | Yes | Mutable reference to the extraction result to process |
| `Config` | `ExtractionConfig` | Yes | Extraction configuration |

**Returns:** No return value.

**Errors:** Returns `error`.

###### ProcessingStage()

Get the processing stage for this post-processor.

Determines when this processor runs in the pipeline.

**Returns:**

The `ProcessingStage` (Early, Middle, or Late).

**Signature:**

```go
func (o *PostProcessor) ProcessingStage() ProcessingStage
```

**Example:**

```go
result := instance.ProcessingStage()
```

**Returns:** `ProcessingStage`

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

**Example:**

```go
result := instance.ShouldProcess(ExtractionResult{}, ExtractionConfig{})
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Result` | `ExtractionResult` | Yes | The extraction result |
| `Config` | `ExtractionConfig` | Yes | The extraction config |

**Returns:** `bool`

###### EstimatedDurationMs()

Optional: Estimate processing time in milliseconds.

Used for logging and debugging. Defaults to 0 (unknown).

**Returns:**

Estimated processing time in milliseconds.

**Signature:**

```go
func (o *PostProcessor) EstimatedDurationMs(result ExtractionResult) uint64
```

**Example:**

```go
result := instance.EstimatedDurationMs(ExtractionResult{})
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Result` | `ExtractionResult` | Yes | The extraction result |

**Returns:** `uint64`

###### Priority()

Execution priority within the processing stage.

Higher values run first within the same `ProcessingStage`. Defaults to 50.
Use 0-49 for fallback processors, 50 for normal processors, and 51-255
for high-priority processors that should run early in their stage.

**Signature:**

```go
func (o *PostProcessor) Priority() int32
```

**Example:**

```go
result := instance.Priority()
```

**Returns:** `int32`

---

#### PostProcessorConfig

Post-processor configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Enabled` | `bool` | `true` | Enable post-processors |
| `EnabledProcessors` | `*\[\]string` | `nil` | Whitelist of processor names to run (None = all enabled) |
| `DisabledProcessors` | `*\[\]string` | `nil` | Blacklist of processor names to skip (None = none disabled) |
| `EnabledSet` | `*\[\]string` | `nil` | Pre-computed AHashSet for O(1) enabled processor lookup |
| `DisabledSet` | `*\[\]string` | `nil` | Pre-computed AHashSet for O(1) disabled processor lookup |

##### Methods

###### Default()

**Signature:**

```go
func (o *PostProcessorConfig) Default() PostProcessorConfig
```

**Example:**

```go
result := PostProcessorConfig.Default()
```

**Returns:** `PostProcessorConfig`

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
| `SlideTitles` | `\[\]string` | `nil` | Slide titles |

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
| `Images` | `\[\]ExtractedImage` | — | Extracted images from the presentation |
| `PageStructure` | `*PageStructure` | `nil` | Slide structure with boundaries (when page tracking is enabled) |
| `PageContents` | `*\[\]PageContent` | `nil` | Per-slide content (when page tracking is enabled) |
| `Document` | `*DocumentStructure` | `nil` | Structured document representation |
| `OfficeMetadata` | `map\[string\]string` | `/* serde(default) */` | Office metadata extracted from docProps/core.xml and docProps/app.xml. Contains keys like "title", "author", "created_by", "subject", "keywords", "modified_by", "created_at", "modified_at", etc. |
| `Revisions` | `*\[\]DocumentRevision` | `/* serde(default) */` | Slide comments as revisions. Each `<p:cm>` element in `ppt/comments/comment{N}.xml` becomes a `DocumentRevision { kind: Comment }` with author (resolved from `ppt/commentAuthors.xml`), ISO-8601 timestamp, and `RevisionAnchor.Slide { index }`. `nil` when no comment XML parts exist. |

---

#### PptxMetadata

PowerPoint presentation metadata.

Extracted from PPTX files containing slide counts and presentation details.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `SlideCount` | `uint32` | — | Total number of slides in the presentation |
| `SlideNames` | `\[\]string` | `nil` | Names of slides (if available) |
| `ImageCount` | `*uint32` | `nil` | Number of embedded images |
| `TableCount` | `*uint32` | `nil` | Number of tables |

---

#### Preset

A curated structured-extraction preset loaded from the embedded library.

Each preset is a JSON file under `src/presets/library/<id>/v1.json` that
validates against the meta-schema in `src/presets/preset.schema.json`.

Downstream catalog consumers can inject presets via
`extend_from_dir`. The embedded OSS library
ships only the `generic_document` toy preset.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Id` | `string` | — | Stable, URL-safe preset identifier (lowercase snake_case). |
| `Version` | `string` | — | Monotonic version string (e.g. `v1`). |
| `SchemaName` | `string` | — | Human-readable schema name forwarded to the LLM as the response/tool name. |
| `Description` | `string` | — | One-line preset description shown in the registry UI. |
| `Category` | `PresetCategory` | — | Top-level category for grouping in the playground. |
| `Tags` | `\[\]string` | `/* serde(default) */` | Free-form tags used for search/filtering. May be empty. |
| `Schema` | `interface{}` | — | JSON Schema (Draft 2020-12) describing the structured output shape. |
| `SystemPrompt` | `string` | — | Instruction primer sent to the model. |
| `ContextTemplate` | `*string` | `/* serde(default) */` | Optional mustache-style template merged with caller-supplied context. |
| `MergeMode` | `MergeMode` | — | Strategy for merging per-batch outputs across paginated calls. |
| `PreferredCallMode` | `CallMode` | — | Default call mode suggested for this preset; heuristics may override. |
| `EmitCitations` | `bool` | — | When true, the prompt asks the model to wrap each field as `{value, page, bbox, confidence}` for downstream citation overlays. |
| `Sample` | `*PresetSample` | `/* serde(default) */` | Optional bundled sample (input file + reference output) for preview. |
| `Fingerprint` | `string` | `/* serde(default) */` | Stable sha256 fingerprint of the canonical preset file contents. Populated at registry load — not present in the on-disk JSON files. Used as a cache-invalidation token by the worker pipeline. |

---

#### PresetSample

Pointer to a sample input + its reference output bundled with the preset.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `InputPath` | `string` | — | Path to the sample input file, relative to the preset directory. |
| `OutputPath` | `string` | — | Path to the reference structured output, relative to the preset directory. |

---

#### PresetSummary

Lightweight projection of `Preset` used by the registry list endpoint
(omits the full schema and prompt to keep the payload small).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Id` | `string` | — | Preset identifier matching `Preset.id`. |
| `Version` | `string` | — | Preset version matching `Preset.version`. |
| `SchemaName` | `string` | — | Schema name matching `Preset.schema_name`. |
| `Description` | `string` | — | One-line preset description. |
| `Category` | `PresetCategory` | — | Top-level category. |
| `Tags` | `\[\]string` | — | Free-form tags. |
| `PreferredCallMode` | `CallMode` | — | Default call mode. |
| `EmitCitations` | `bool` | — | Whether the preset prompts the model for citations. |
| `Fingerprint` | `string` | — | Stable fingerprint matching `Preset.fingerprint`. |

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
| `MessageCount` | `int` | — | Total number of email messages found in the PST archive. |

---

#### QrBoundingBox

Pixel-space bounding box of a QR code inside its source image.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `X` | `uint32` | — | Horizontal pixel offset of the bounding box top-left corner. |
| `Y` | `uint32` | — | Vertical pixel offset of the bounding box top-left corner. |
| `Width` | `uint32` | — | Width of the bounding box in pixels. |
| `Height` | `uint32` | — | Height of the bounding box in pixels. |

---

#### QrCode

One QR code decoded from an extracted image.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Payload` | `string` | — | Decoded payload (text, URL, vCard string, …). |
| `Confidence` | `*float32` | `nil` | Detector-reported confidence in `\[0.0, 1.0\]`. `nil` when the decoder does not expose confidence (the default `rqrr` backend always reports `Some` because successful decode implies high confidence). |
| `Bbox` | `*QrBoundingBox` | `nil` | Bounding box of the QR code inside the source image, in pixel coordinates (`x`, `y` of the top-left corner; `width`, `height` of the rectangle). `nil` if the decoder did not report a bounding box. |

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

**Example:**

```go
result := RakeParams.Default()
```

**Returns:** `RakeParams`

---

#### RecognizedTable

Pre-computed table markdown for a table detection region.

Produced by the TATR-based table structure recognizer and surfaced as part of
layout-aware OCR results.  The struct lives here (under `layout-types`, pure-Rust)
so that consumers who do not enable `layout-detection` (ORT) can still reference
the type in their own code.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `DetectionBbox` | `BBox` | — | Detection bbox that this table corresponds to (for matching). |
| `Cells` | `\[\]\[\]string` | — | Table cells as a 2D vector (rows × columns). |
| `Markdown` | `string` | — | Rendered markdown table. |

---

#### RedactionConfig

**Since:** `v5.0`

Configuration for the redaction post-processor.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Categories` | `\[\]PiiCategory` | `nil` | Categories to redact. Empty means "every category supported by the engine." |
| `Strategy` | `RedactionStrategy` | `RedactionStrategy.Mask` | Strategy applied to every match. |
| `Ner` | `*NerConfig` | `nil` | Optional NER backend — required to redact PERSON / ORGANIZATION / LOCATION categories (the pure-Rust pattern engine only covers regex-detectable PII). |
| `PreserveOffsets` | `bool` | `true` | When `true`, chunk byte ranges are kept consistent with the rewritten content by adjusting `byte_start` / `byte_end` after replacement. When `false`, chunk byte ranges still refer to the *original* content offsets — useful when downstream consumers want to map findings back to the original document. |
| `CustomTerms` | `\[\]RedactionTerm` | `nil` | Arbitrary user-supplied literal terms to redact. Each term is treated as a regex hit against the document, surfacing as `PiiCategory.Custom(label)` in `RedactionFinding` where `label` is the per-term label (defaulting to the literal value itself). Case-insensitive by default; set `RedactionTerm.case_sensitive` for exact match. Use this when you need to redact tenant-specific tokens (employee IDs, project codes, internal product names) without writing a custom plugin. |
| `CustomPatterns` | `\[\]RedactionPattern` | `nil` | Arbitrary user-supplied regex patterns to redact. Same surfacing semantics as `custom_terms`: each hit becomes a `PiiCategory.Custom(label)` finding. Patterns are validated at config-construction time via `RedactionConfig.validate`. |

##### Methods

###### Default()

**Signature:**

```go
func (o *RedactionConfig) Default() RedactionConfig
```

**Example:**

```go
result := RedactionConfig.Default()
```

**Returns:** `RedactionConfig`

###### Validate()

Validate user-supplied terms and patterns at config-construction time.

Compiles every `RedactionPattern.pattern` (with the case-insensitive
inline flag where applicable) and returns the first compilation error so
the caller can reject the config before the redaction pipeline runs.
Pure terms (regex-escaped) cannot fail to compile, but the function
still rejects empty values to avoid degenerate zero-length matches.

**Signature:**

```go
func (o *RedactionConfig) Validate() error
```

**Example:**

```go
if err := instance.Validate(); err != nil {
    return err
}
```

**Returns:** No return value.

**Errors:** Returns `error`.

---

#### RedactionFinding

One redaction event: which span was rewritten, why, and with what.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Start` | `uint32` | — | Byte-offset start in the original (pre-redaction) `ExtractionResult.content`. |
| `End` | `uint32` | — | Byte-offset end (exclusive) in the original `ExtractionResult.content`. |
| `Category` | `PiiCategory` | — | PII category that fired this redaction. |
| `Strategy` | `RedactionStrategy` | — | Strategy applied to this finding (mask, hash, token-replace, drop). |
| `ReplacementToken` | `string` | — | String that replaced the original mention. Always present; for `Drop` the replacement is the empty string. |

---

#### RedactionPattern

One user-supplied regex pattern to redact.

The pattern is compiled with the Rust `regex` crate (no look-around). Case
sensitivity is encoded in the pattern via the `(?i)` inline flag when
`Self.case_sensitive` is `false`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Label` | `string` | — | Custom category label surfaced in `RedactionFinding.category`. |
| `Pattern` | `string` | — | Regex pattern (Rust `regex` crate dialect — no look-around). |
| `CaseSensitive` | `bool` | `serde(default = "default_case_sensitive")` | When `true`, match case-sensitively; otherwise prepend `(?i)` to the regex. |

##### Methods

###### Labeled()

Build a pattern with the given label (case-insensitive by default).

**Signature:**

```go
func (o *RedactionPattern) Labeled(label string, pattern string) RedactionPattern
```

**Example:**

```go
result := RedactionPattern.Labeled("value", "value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Label` | `string` | Yes | The label |
| `Pattern` | `string` | Yes | The pattern |

**Returns:** `RedactionPattern`

---

#### RedactionReport

Audit report describing what the redaction processor found and how it replaced it.

The redactor returns this alongside the rewritten content so compliance, replay, and
audit-log consumers can see exactly what fired. Offsets are relative to the *original*
pre-redaction `content` and are intended for audit reconstruction only — the original
bytes are dropped at the end of the pipeline.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Findings` | `\[\]RedactionFinding` | — | Individual redaction findings in original-source byte order. |
| `TotalRedacted` | `uint32` | — | Total number of redactions applied across the document. |

---

#### RedactionTerm

One user-supplied literal term to redact.

Matched as a regex-escaped substring (so callers do not need to escape
metacharacters themselves). Case-insensitive by default — set
`Self.case_sensitive` to `true` for exact byte-match semantics.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Label` | `string` | — | Custom category label surfaced in `RedactionFinding.category`. |
| `Value` | `string` | — | Literal value to match. Regex metacharacters are escaped automatically. |
| `CaseSensitive` | `bool` | `serde(default = "default_case_sensitive")` | When `true`, match the value as-is; otherwise match ASCII-case-insensitively. |

##### Methods

###### Literal()

Build a term whose label is the literal value itself (case-insensitive).

**Signature:**

```go
func (o *RedactionTerm) Literal(value string) RedactionTerm
```

**Example:**

```go
result := RedactionTerm.Literal("value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Value` | `string` | Yes | The value |

**Returns:** `RedactionTerm`

###### Labeled()

Build a term with a custom label.

**Signature:**

```go
func (o *RedactionTerm) Labeled(label string, value string) RedactionTerm
```

**Example:**

```go
result := RedactionTerm.Labeled("value", "value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Label` | `string` | Yes | The label |
| `Value` | `string` | Yes | The value |

**Returns:** `RedactionTerm`

---

#### Registry

Sorted map of preset id → `Preset`.

##### Methods

###### LoadEmbedded()

Build the registry from preset files embedded at compile time under
`src/presets/library/`. Validates every file against the meta-schema.

**Signature:**

```go
func (o *Registry) LoadEmbedded() (Registry, error)
```

**Example:**

```go
result, err := Registry.LoadEmbedded()
if err != nil {
    return err
}
```

**Returns:** `Registry`

**Errors:** Returns `error`.

###### Global()

Return the global registry, loading it on first access.

**Panics:**

Panics if any embedded preset is malformed. The build-time validation
test ensures this cannot happen for the embedded presets; a panic here
indicates a build artifact problem, not a runtime error.

**Signature:**

```go
func (o *Registry) Global() Registry
```

**Example:**

```go
result := Registry.Global()
```

**Returns:** `Registry`

###### Get()

Look up a preset by its identifier.

**Signature:**

```go
func (o *Registry) Get(id string) *Preset
```

**Example:**

```go
result := instance.Get("value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Id` | `string` | Yes | The id |

**Returns:** `*Preset`

###### Summaries()

Materialize a `PresetSummary` list for the public registry endpoint.

**Signature:**

```go
func (o *Registry) Summaries() []PresetSummary
```

**Example:**

```go
result := instance.Summaries()
```

**Returns:** `[]PresetSummary`

###### Len()

Number of presets currently loaded.

**Signature:**

```go
func (o *Registry) Len() int
```

**Example:**

```go
result := instance.Len()
```

**Returns:** `int`

###### IsEmpty()

Whether the registry contains zero presets.

**Signature:**

```go
func (o *Registry) IsEmpty() bool
```

**Example:**

```go
result := instance.IsEmpty()
```

**Returns:** `bool`

###### SampleBytes()

Read raw sample bytes for `<preset_id>` from
`library/<id>/samples/<name>`. Returns `nil` when the file is absent.

**Signature:**

```go
func (o *Registry) SampleBytes(presetId string, name string) *[]byte
```

**Example:**

```go
result := instance.SampleBytes("value", "value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `PresetId` | `string` | Yes | The preset id |
| `Name` | `string` | Yes | The name |

**Returns:** `*[]byte`

###### ExtendFromDir()

Load additional preset files from a runtime directory and insert them
into this registry.

Reads every `*.json` file directly under `dir` (non-recursive),
validates each against the meta-schema, and inserts it. Files that fail
validation are rejected — the error is returned immediately and the
registry is left in a partially-updated state. Existing entries with the
same id are overwritten.

Returns the number of presets successfully loaded from `dir`.

##### Use case

This is the injection point for downstream catalogs that add curated
presets on top of the single embedded OSS preset.

**Signature:**

```go
func (o *Registry) ExtendFromDir(dir string) (int, error)
```

**Example:**

```go
result, err := instance.ExtendFromDir("value")
if err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Dir` | `string` | Yes | The dir |

**Returns:** `int`

**Errors:** Returns `error`.

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

##### Thread Safety

Renderers must be `Send + Sync` (inherited from `Plugin`).

##### Methods

###### Render()

Render an `InternalDocument` to the output format.

**Returns:**

The rendered output as a string.

**Errors:**

Returns an error if rendering fails.

**Signature:**

```go
func (o *Renderer) Render(doc InternalDocument) (string, error)
```

**Example:**

```go
result, err := instance.Render(InternalDocument{})
if err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Doc` | `InternalDocument` | Yes | The internal document to render |

**Returns:** `string`

**Errors:** Returns `error`.

---

#### RerankedDocument

A single document returned by the reranker, with its position in the input and score.

`index` maps back to the caller's original document list, so metadata arrays
(e.g. IDs, paths) can be reordered without passing them through the reranker.

Since v5.0.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Index` | `int` | — | Position of this document in the original input `documents` slice. |
| `Score` | `float32` | — | Relevance score in `\[0, 1\]`. Higher means more relevant to the query. |
| `Document` | `string` | — | The document text. |

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

##### Thread safety

Backends must be `Send + Sync + 'static`. They are stored in
`Arc<dyn RerankerBackend>` and may be called concurrently from xberg's
dispatcher. If the backend's underlying model is not thread-safe, the
backend itself must serialize access internally (e.g. via `Mutex<Inner>`).

##### Contract

- `rerank(query, documents)` MUST return exactly `documents.len()` scores.
  The dispatcher validates this before sorting and returning to callers;
  a non-conforming backend surfaces as a `XbergError.Validation`, not
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

###### Rerank()

Score a list of documents against a query.

Returns one raw logit per document in the same order as the input.
The dispatcher applies sigmoid to convert to `[0, 1]` scores.

**Errors:**

Implementations should return `Plugin` for
backend-specific failures. The dispatcher validates the returned length
against `documents.len()` before sorting.

**Signature:**

```go
func (o *RerankerBackend) Rerank(query string, documents []string) ([]float32, error)
```

**Example:**

```go
result, err := instance.Rerank("value", nil)
if err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Query` | `string` | Yes | The query |
| `Documents` | `\[\]string` | Yes | The documents |

**Returns:** `[]float32`

**Errors:** Returns `error`.

---

#### RerankerConfig

Configuration for the reranking pipeline.

Controls which model to use, how many results to return, and download/cache
behavior for local ONNX models.

Since v5.0.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Model` | `RerankerModelType` | `RerankerModelType.Preset` | The reranker model to use (defaults to "balanced" preset if not specified). |
| `TopK` | `*int` | `nil` | Return at most this many documents. `nil` returns all. Applied after sorting by score, so the highest-scoring documents are kept. |
| `BatchSize` | `int` | `32` | Batch size for local ONNX cross-encoder inference. |
| `ShowDownloadProgress` | `bool` | `false` | Show model download progress (local ONNX path only). |
| `CacheDir` | `*string` | `nil` | Custom cache directory for model files. Defaults to `~/.cache/xberg/rerankers/` if not specified. |
| `Acceleration` | `*AccelerationConfig` | `nil` | Hardware acceleration for the reranker ONNX model. Controls which execution provider (CPU, CUDA, CoreML, TensorRT) is used for local inference. Defaults to `nil` (auto-select per platform). |
| `MaxRerankDurationSecs` | `*uint64` | `nil` | Maximum wall-clock duration (in seconds) for a single `rerank()` call when using `RerankerModelType.Plugin`. Applies only to the in-process plugin path — protects against hung host-language backends. On timeout, the dispatcher returns `Plugin` instead of blocking forever. `nil` disables the timeout. The default (60 seconds) is conservative for common in-process inference; increase for large document sets on slow hardware. |

##### Methods

###### Default()

**Signature:**

```go
func (o *RerankerConfig) Default() RerankerConfig
```

**Example:**

```go
result := RerankerConfig.Default()
```

**Returns:** `RerankerConfig`

---

#### RerankerPreset

Metadata for a bundled reranker preset.

All string fields are owned `String` for FFI compatibility — instances are
safe to clone and pass across language boundaries.

Since v5.0.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Name` | `string` | — | Short identifier (catalog name, e.g. `"bge-reranker-base"`). |
| `ModelRepo` | `string` | — | HuggingFace repository name for the model. |
| `ModelFile` | `string` | — | Path to the ONNX model file within the repo. |
| `AdditionalFiles` | `\[\]string` | `/* serde(default) */` | Sibling files that must be downloaded alongside `model_file`. Empty for most presets. Used by repos that split the weight blob — e.g. `rozgo/bge-reranker-v2-m3` ships the model in `model.onnx` plus a co-located `model.onnx.data` payload. |
| `MaxLength` | `int` | — | Maximum token sequence length the model supports. |
| `Description` | `string` | — | Human-readable description of the preset's intended use case. |

---

#### ResolvedPreset

A preset merged with caller-supplied overrides (custom schema, prompt suffix,
context map). Output is what the pipeline orchestrator consumes.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Id` | `string` | — | Source preset identifier. |
| `Version` | `string` | — | Source preset version. |
| `Fingerprint` | `string` | — | Fingerprint of the source preset file, used as a cache token. |
| `SchemaName` | `string` | — | Schema name forwarded to the LLM. |
| `Schema` | `interface{}` | — | Effective JSON Schema (caller override or the preset's own). |
| `SystemPrompt` | `string` | — | System prompt with rendered context appended. |
| `MergeMode` | `MergeMode` | — | Merge strategy for paginated outputs. |
| `PreferredCallMode` | `CallMode` | — | Preferred call mode. |
| `EmitCitations` | `bool` | — | Whether the prompt asks for per-field citations. |

---

#### RevisionDelta

The content changes that make up a single revision.

For insertions and deletions the `content` field carries the added/removed
lines as `DiffLine.Added` / `DiffLine.Removed` entries. For format
changes, `content` is empty — the property diff is left as a TODO for a
later enrichment pass.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Content` | `\[\]DiffLine` | `nil` | Line-level content changes for this revision. |
| `TableChanges` | `\[\]CellChange` | `nil` | Cell-level table changes for this revision. |

---

#### SecurityLimits

Configuration for security limits across extractors.

All limits are intentionally conservative to prevent DoS attacks
while still supporting legitimate documents.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `MaxArchiveSize` | `int` | `524288000` | Maximum uncompressed size for archives (500 MB) |
| `MaxCompressionRatio` | `int` | `100` | Maximum compression ratio before flagging as potential bomb (100:1) |
| `MaxFilesInArchive` | `int` | `10000` | Maximum number of files in archive (10,000) |
| `MaxNestingDepth` | `int` | `1024` | Maximum nesting depth for structures (100) |
| `MaxEntityLength` | `int` | `1048576` | Maximum length of any single XML entity / attribute / token (1 MiB). This is a per-token cap, NOT a total cap — billion-laughs class attacks where a single entity expands to hundreds of MB are caught here, while normal long text content (a paragraph, a CDATA block) is caught by `max_content_size` instead. |
| `MaxContentSize` | `int` | `104857600` | Maximum string growth per document (100 MB) |
| `MaxIterations` | `int` | `10000000` | Maximum iterations per operation |
| `MaxXmlDepth` | `int` | `1024` | Maximum XML depth (100 levels) |
| `MaxTableCells` | `int` | `100000` | Maximum cells per table (100,000) |

##### Methods

###### Default()

**Signature:**

```go
func (o *SecurityLimits) Default() SecurityLimits
```

**Example:**

```go
result := SecurityLimits.Default()
```

**Returns:** `SecurityLimits`

---

#### ServerConfig

API server configuration.

This struct holds all configuration options for the Xberg API server,
including host/port settings, CORS configuration, and upload limits.

##### Defaults

- `host`: "127.0.0.1" (localhost only)
- `port`: 8000
- `cors_origins`: empty listtor (allows all origins)
- `max_request_body_bytes`: 104_857_600 (100 MB)
- `max_multipart_field_bytes`: 104_857_600 (100 MB)

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Host` | `string` | — | Server host address (e.g., "127.0.0.1", "0.0.0.0") |
| `Port` | `uint16` | — | Server port number |
| `CorsOrigins` | `\[\]string` | `nil` | CORS allowed origins. Empty vector means allow all origins. If this is an empty listtor, the server will accept requests from any origin. If populated with specific origins (e.g., `"<https://example.com"`>), only those origins will be allowed. |
| `MaxRequestBodyBytes` | `int` | — | Maximum size of request body in bytes (default: 100 MB) |
| `MaxMultipartFieldBytes` | `int` | — | Maximum size of multipart fields in bytes (default: 100 MB) |

##### Methods

###### Default()

**Signature:**

```go
func (o *ServerConfig) Default() ServerConfig
```

**Example:**

```go
result := ServerConfig.Default()
```

**Returns:** `ServerConfig`

###### ListenAddr()

Get the server listen address (host:port).

**Signature:**

```go
func (o *ServerConfig) ListenAddr() string
```

**Example:**

```go
result := instance.ListenAddr()
```

**Returns:** `string`

###### CorsAllowsAll()

Check if CORS allows all origins.

Returns `true` if the `cors_origins` vector is empty, meaning all origins
are allowed. Returns `false` if specific origins are configured.

**Signature:**

```go
func (o *ServerConfig) CorsAllowsAll() bool
```

**Example:**

```go
result := instance.CorsAllowsAll()
```

**Returns:** `bool`

###### IsOriginAllowed()

Check if a given origin is allowed by CORS configuration.

Returns `true` if:

- CORS allows all origins (empty origins list), or
- The given origin is in the allowed origins list

**Signature:**

```go
func (o *ServerConfig) IsOriginAllowed(origin string) bool
```

**Example:**

```go
result := instance.IsOriginAllowed("value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Origin` | `string` | Yes | The origin to check (e.g., "<https://example.com">) |

**Returns:** `bool`

###### MaxRequestBodyMb()

Get maximum request body size in megabytes (rounded up).

**Signature:**

```go
func (o *ServerConfig) MaxRequestBodyMb() int
```

**Example:**

```go
result := instance.MaxRequestBodyMb()
```

**Returns:** `int`

###### MaxMultipartFieldMb()

Get maximum multipart field size in megabytes (rounded up).

**Signature:**

```go
func (o *ServerConfig) MaxMultipartFieldMb() int
```

**Example:**

```go
result := instance.MaxMultipartFieldMb()
```

**Returns:** `int`

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

Result of parsing a structured data file (JSON, JSONL, YAML, or TOML).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Content` | `string` | — | The extracted text content, formatted for readability. |
| `Format` | `string` | — | The source format identifier (e.g. `"json"`, `"yaml"`, `"toml"`). |
| `Metadata` | `map\[string\]string` | — | Key-value metadata extracted from recognized text fields. |
| `TextFields` | `\[\]string` | — | JSON paths of fields that were classified as text-bearing. |

---

#### StructuredExtractionConfig

Configuration for LLM-based structured data extraction.

Sends extracted document content to a VLM with a JSON schema,
returning structured data that conforms to the schema.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Schema` | `interface{}` | — | JSON Schema defining the desired output structure. |
| `SchemaName` | `string` | `serde(default = "default_schema_name")` | Schema name passed to the LLM's structured output mode. |
| `SchemaDescription` | `*string` | `/* serde(default) */` | Optional schema description for the LLM. |
| `Strict` | `bool` | `/* serde(default) */` | Enable strict mode — output must exactly match the schema. |
| `Prompt` | `*string` | `/* serde(default) */` | Custom Jinja2 extraction prompt template. When `nil`, a default template is used. Available template variables: - `{{ content }}` — The extracted document text. - `{{ schema }}` — The JSON schema as a formatted string. - `{{ schema_name }}` — The schema name. - `{{ schema_description }}` — The schema description (may be empty). |
| `Llm` | `LlmConfig` | — | LLM configuration for the extraction. |

---

#### StructuredInput

Signals consumed by the call-mode heuristic.

All fields derive from a prior xberg extraction — no double-work.
This is a plain DTO; it intentionally has no dependency on internal
xberg extraction types so it can be constructed from any source.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `MimeType` | `string` | — | MIME type, canonicalised to lowercase by the caller. |
| `PageCount` | `uint32` | — | Number of pages in the document. |
| `TextCoverage` | `float64` | — | Fraction of pages with a real text layer (0.0..=1.0). |
| `AvgCharsPerPage` | `float64` | — | Average extracted characters per page. |
| `EmbeddedImageCount` | `uint32` | — | Count of embedded images (figures, photos, signatures) discovered. |
| `UserForceVision` | `bool` | — | When `true`, promote the result to at least `StructuredCallMode.TextPlusVision`. |

---

#### StructuredThresholds

Thresholds for the structured-extraction call-mode heuristic.

All defaults are **conservative starting points**.  Deployments should
measure their own document corpus and override via their own config;
these values are chosen to be safe-by-default, not to be optimal for
any particular workload.

Construct custom thresholds with struct-update syntax:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `ScanMaxCoverage` | `float64` | `0.1` | PDFs with `text_coverage` strictly below this are treated as scanned. **Conservative default: 0.10** — deployments override via their own config after measuring their document corpus. |
| `DigitalMinCoverage` | `float64` | `0.9` | PDFs with `text_coverage` at or above this AND zero embedded images route to `StructuredCallMode.TextOnly`. **Conservative default: 0.90** — deployments override via their own config after measuring their document corpus. |
| `DocxTextMinDensity` | `float64` | `200` | DOCX / HTML / text documents with `avg_chars_per_page` above this route to `StructuredCallMode.TextOnly`. **Conservative default: 200.0** — deployments override via their own config after measuring their document corpus. |
| `EnableVisionFallback` | `bool` | `false` | When `true`, emit `StructuredCallMode.TextOnlyWithVisionFallback` instead of `StructuredCallMode.TextOnly` so the orchestrator can escalate to vision on low confidence. **Conservative default: `false`** — must be explicitly enabled per deployment after bench validation; deployments override via their own config. |

##### Methods

###### Default()

**Signature:**

```go
func (o *StructuredThresholds) Default() StructuredThresholds
```

**Example:**

```go
result := StructuredThresholds.Default()
```

**Returns:** `StructuredThresholds`

---

#### SummarizationConfig

**Since:** `v5.0`

Configuration for the summarisation post-processor.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Strategy` | `SummaryStrategy` | `SummaryStrategy.Extractive` | Summarisation strategy. |
| `MaxTokens` | `*uint32` | `nil` | Maximum summary length in tokens. `nil` lets the backend pick a default. |
| `Llm` | `*LlmConfig` | `nil` | LLM configuration for the abstractive backend. Ignored when `strategy = Extractive`. Required when `strategy = Abstractive`. |

---

#### SupportedFormat

A supported document format entry.

Represents a file extension and its corresponding MIME type that Xberg can process.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Extension` | `string` | — | File extension (without leading dot), e.g., "pdf", "docx" |
| `MimeType` | `string` | — | MIME type string, e.g., "application/pdf" |

---

#### SvgOptions

SVG-specific configuration for the image-encode pipeline.

Applies when the source image is SVG or when the output format is set to
`ImageOutputFormat.Svg`.  Available when the `svg` feature is active.

Used via `ImageExtractionConfig.svg`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Sanitize` | `bool` | `true` | Run SVG bytes through `usvg` sanitization (strips external `href` attributes, JavaScript event handlers, and `foreignObject` elements) even when the output format is `Native`.  Defaults to `true`. |
| `RenderDpi` | `float32` | `96` | Target DPI when rasterizing SVG to a pixel-based format (PNG, JPEG, WebP, HEIF).  The tree's viewBox is scaled by `render_dpi / 96.0` before the pixel buffer is allocated.  Defaults to `96.0` (1× CSS pixel density). |

##### Methods

###### Default()

**Signature:**

```go
func (o *SvgOptions) Default() SvgOptions
```

**Example:**

```go
result := SvgOptions.Default()
```

**Returns:** `SvgOptions`

---

#### Table

Extracted table structure.

Represents a table detected and extracted from a document (PDF, image, etc.).
Tables are converted to both structured cell data and Markdown format.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Cells` | `\[\]\[\]string` | `nil` | Table cells as a 2D vector (rows × columns) |
| `Markdown` | `string` | — | Markdown representation of the table |
| `PageNumber` | `uint32` | — | Page number where the table was found (1-indexed) |
| `BoundingBox` | `*BoundingBox` | `nil` | Bounding box of the table on the page (PDF coordinates: x0=left, y0=bottom, x1=right, y1=top). Only populated for PDF-extracted tables when position data is available. |

---

#### TableCell

Individual table cell with content and optional styling.

Future extension point for rich table support with cell-level metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Content` | `string` | — | Cell content as text |
| `RowSpan` | `uint32` | — | Row span (number of rows this cell spans) |
| `ColSpan` | `uint32` | — | Column span (number of columns this cell spans) |
| `IsHeader` | `bool` | — | Whether this is a header cell |

---

#### TableDiff

Cell-level changes for a pair of tables that share the same index.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `FromIndex` | `int` | — | Zero-based index of the table in both `a.tables` and `b.tables`. |
| `ToIndex` | `int` | — | Zero-based index in `b.tables` (equal to `from_index` for same-dimension tables). |
| `CellChanges` | `\[\]CellChange` | — | Cell-level changes within the table. |

---

#### TableGrid

Structured table grid with cell-level metadata.

Stores row/column dimensions and a flat list of cells with position info.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Rows` | `uint32` | — | Number of rows in the table. |
| `Cols` | `uint32` | — | Number of columns in the table. |
| `Cells` | `\[\]GridCell` | `nil` | All cells in row-major order. |

---

#### TesseractConfig

Tesseract OCR configuration.

Provides fine-grained control over Tesseract OCR engine parameters.
Most users can use the defaults, but these settings allow optimization
for specific document types (invoices, handwriting, etc.).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Language` | `\[\]string` | `nil` | Language code(s) for OCR recognition. Accepts either a single language code ("eng") or a list (\["eng", "deu"\]). For Tesseract backend, languages are joined with "+". |
| `Psm` | `int32` | `3` | Page Segmentation Mode (0-13). Common values: - 3: Fully automatic page segmentation (native default) - 6: Assume a single uniform block of text (WASM default — avoids layout-analysis hang) - 11: Sparse text with no particular order |
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

**Example:**

```go
result := TesseractConfig.Default()
```

**Returns:** `TesseractConfig`

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
| `Headers` | `*\[\]string` | `nil` | Markdown headers (text only, Markdown files only) |

---

#### TextMetadata

Text/Markdown metadata.

Extracted from plain text and Markdown files. Includes word counts and,
for Markdown, structural elements like headers and links.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `LineCount` | `uint32` | — | Number of lines in the document |
| `WordCount` | `uint32` | — | Number of words |
| `CharacterCount` | `uint32` | — | Number of characters |
| `Headers` | `*\[\]string` | `nil` | Markdown headers (headings text only, for Markdown files) |

---

#### TokenCounter

Per-category running counter for `RedactionStrategy.TokenReplace`.

##### Methods

###### New()

Create a fresh counter with no previous state.

**Signature:**

```go
func (o *TokenCounter) New() TokenCounter
```

**Example:**

```go
result := TokenCounter.New()
```

**Returns:** `TokenCounter`

---

#### TokenReductionConfig

Configuration for the token-reduction pipeline.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Level` | `ReductionLevel` | `ReductionLevel.Moderate` | Reduction intensity level. |
| `LanguageHint` | `*string` | `nil` | ISO 639-1 language code hint for stopword selection (e.g. `"en"`, `"de"`). |
| `PreserveMarkdown` | `bool` | `false` | Preserve Markdown formatting tokens during reduction. |
| `PreserveCode` | `bool` | `true` | Preserve code block contents unchanged. |
| `SemanticThreshold` | `float32` | `0.3` | Cosine similarity threshold below which sentences are considered dissimilar. |
| `EnableParallel` | `bool` | `true` | Use Rayon parallel iterators for multi-core processing. |
| `UseSimd` | `bool` | `true` | Use SIMD-optimized text scanning where available. |
| `CustomStopwords` | `*map\[string\]\[\]string` | `nil` | Per-language custom stopword lists (`language_code → stopword_list`). |
| `PreservePatterns` | `\[\]string` | `nil` | Regex patterns whose matched text is always preserved unchanged. |
| `TargetReduction` | `*float32` | `nil` | Target fraction of text to retain (0.0–1.0); `nil` = no fixed target. |
| `EnableSemanticClustering` | `bool` | `false` | Group semantically similar sentences and emit only one per cluster. |

##### Methods

###### Default()

**Signature:**

```go
func (o *TokenReductionConfig) Default() TokenReductionConfig
```

**Example:**

```go
result := TokenReductionConfig.Default()
```

**Returns:** `TokenReductionConfig`

---

#### TokenReductionOptions

Token reduction configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Mode` | `string` | — | Reduction mode: "off", "light", "moderate", "aggressive", "maximum" |
| `PreserveImportantWords` | `bool` | `true` | Preserve important words (capitalized, technical terms) |

##### Methods

###### Default()

**Signature:**

```go
func (o *TokenReductionOptions) Default() TokenReductionOptions
```

**Example:**

```go
result := TokenReductionOptions.Default()
```

**Returns:** `TokenReductionOptions`

---

#### TranscriptionConfig

Configuration for audio/video transcription (speech-to-text).

When present and `enabled`, Xberg will route audio and video files
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
| `Enabled` | `bool` | `true` | Master switch. When false the block is ignored and audio files fall back to the normal "unsupported format" path. |
| `Model` | `WhisperModel` | `WhisperModel.Tiny` | Whisper model size to use. Smaller = faster + lower memory. `tiny` is the pragmatic default for first-time users and CI. |
| `Language` | `*string` | `nil` | Optional language hint (ISO-639-1 code, e.g. "en", "de"). When `nil` (default), the current engine falls back to English. For deterministic production output, always set this explicitly. |
| `Timestamps` | `bool` | `false` | Whether to request segment-level timestamps. Accepted for forward compatibility. The current engine always uses `<\|notimestamps\|>` and does not emit segment metadata yet. |
| `MaxDurationMs` | `*uint64` | `nil` | Hard safety limit on input duration (milliseconds). Files longer than this are rejected after decode, before model work. Default: 30 minutes. Set to `nil` to disable (not recommended for untrusted input). |
| `MaxBytes` | `*uint64` | `nil` | Hard safety limit on input size (bytes). Default: 512 MiB. Protects against pathological or malicious uploads. |
| `TimeoutMs` | `*uint64` | `nil` | Wall-clock timeout for the entire transcription operation (ms). Default: 10 minutes. Reserved for timeout enforcement; the current extractor does not enforce this field yet. |
| `ModelCacheDir` | `*string` | `nil` | Override the directory used for Whisper model cache. When `nil`, uses the centralized resolver: `XBERG_CACHE_DIR/whisper` or the platform default (`~/.cache/xberg/whisper` on Linux, etc.). |
| `AllowNetwork` | `bool` | `true` | Allow network access to download models from Hugging Face Hub. When `false`, only previously cached models may be used. Useful for air-gapped or fully offline deployments. |
| `VerifyHash` | `bool` | `true` | Request SHA256 verification of downloaded model files. Reserved for the checksum table follow-up. The current resolver logs a warning and treats this as a no-op. |

##### Methods

###### Default()

**Signature:**

```go
func (o *TranscriptionConfig) Default() TranscriptionConfig
```

**Example:**

```go
result := TranscriptionConfig.Default()
```

**Returns:** `TranscriptionConfig`

---

#### Translation

Translation of the extracted content.

Holds the translated rendition of `ExtractionResult.content` and (when
`preserve_markup` was requested) the translated `formatted_content`. Chunks
are translated in place inside `ExtractionResult.chunks[*].content` rather
than duplicated here.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `TargetLang` | `string` | — | BCP-47 language tag the translation was produced into (e.g. `"de"`, `"fr-CA"`). |
| `SourceLang` | `*string` | `nil` | BCP-47 source language. `nil` when the translation backend was asked to detect. |
| `Content` | `string` | — | Translated plain-text body. Matches the shape of `ExtractionResult.content`. |
| `FormattedContent` | `*string` | `nil` | Translated markup body (Markdown / HTML / etc.) when `preserve_markup` was enabled on the config. `nil` otherwise. |

---

#### TranslationConfig

**Since:** `v5.0`

Configuration for the translation post-processor.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `TargetLang` | `string` | — | BCP-47 language tag for the target language (e.g. `"de"`, `"fr-CA"`). |
| `SourceLang` | `*string` | `nil` | Optional explicit source language. `nil` asks the backend to auto-detect. |
| `PreserveMarkup` | `bool` | `/* serde(default) */` | Translate the formatted (Markdown/HTML) rendition alongside plain text when `formatted_content` is present. |
| `Llm` | `LlmConfig` | — | LLM configuration used for translation. |

---

#### TreeSitterConfig

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
| `Enabled` | `bool` | `true` | Enable code intelligence processing (default: true). When `false`, tree-sitter analysis is completely skipped even if the config section is present. |
| `CacheDir` | `*string` | `nil` | Custom cache directory for downloaded grammars. When `nil`, uses the default: `~/.cache/tree-sitter-language-pack/v{version}/libs/`. |
| `Languages` | `*\[\]string` | `nil` | Languages to pre-download on init (e.g., `\["python", "rust"\]`). |
| `Groups` | `*\[\]string` | `nil` | Language groups to pre-download (e.g., `\["web", "systems", "scripting"\]`). |
| `Process` | `TreeSitterProcessConfig` | — | Processing options for code analysis. |

##### Methods

###### Default()

**Signature:**

```go
func (o *TreeSitterConfig) Default() TreeSitterConfig
```

**Example:**

```go
result := TreeSitterConfig.Default()
```

**Returns:** `TreeSitterConfig`

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

**Example:**

```go
result := TreeSitterProcessConfig.Default()
```

**Returns:** `TreeSitterProcessConfig`

---

#### UrlExtractionConfig

URL ingestion and crawl configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Mode` | `UrlExtractionMode` | `UrlExtractionMode.Auto` | URL extraction mode. |
| `DocumentUrlPattern` | `*string` | `nil` | Optional regex filter for document-discovered URLs. |
| `MaxDocumentUrlsPerResult` | `*uint32` | `nil` | Maximum URLs to follow per extraction result. |
| `MaxTotalUrls` | `*uint32` | `nil` | Maximum URLs followed across the whole extraction call. |
| `AllowLocalFileInputs` | `bool` | `true` | Allow bare local filesystem path inputs. |
| `AllowFileUris` | `bool` | `true` | Allow local `file://` URI inputs. |

##### Methods

###### Default()

**Signature:**

```go
func (o *UrlExtractionConfig) Default() UrlExtractionConfig
```

**Example:**

```go
result := UrlExtractionConfig.Default()
```

**Returns:** `UrlExtractionConfig`

---

#### UserChunkConfig

User-provided chunk configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `PageRanges` | `*\[\]PageRange` | `nil` | User-specified page ranges (overrides automatic chunking). |
| `PagesPerChunk` | `*uint32` | `nil` | User-specified pages per chunk (overrides automatic calculation). |
| `ForceChunking` | `bool` | — | Force chunking even for small documents. |
| `DisableChunking` | `bool` | — | Disable chunking even for large documents. |

---

#### Validator

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

###### Validate()

Validate an extraction result.

Check the extraction result and return `Ok(())` if valid, or an error
if validation fails.

**Returns:**

- `Ok(())` if validation passes
- `Err(...)` if validation fails (extraction will fail)

**Errors:**

- `XbergError.Validation` - Validation failed
- Any other error type appropriate for the failure

##### Example - Content Length Validation

##### Example - Quality Score Validation

##### Example - Security Validation

**Signature:**

```go
func (o *Validator) Validate(result ExtractionResult, config ExtractionConfig) error
```

**Example:**

```go
if err := instance.Validate(ExtractionResult{}, ExtractionConfig{}); err != nil {
    return err
}
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Result` | `ExtractionResult` | Yes | The extraction result to validate |
| `Config` | `ExtractionConfig` | Yes | Extraction configuration |

**Returns:** No return value.

**Errors:** Returns `error`.

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

**Example:**

```go
result := instance.ShouldValidate(ExtractionResult{}, ExtractionConfig{})
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `Result` | `ExtractionResult` | Yes | The extraction result |
| `Config` | `ExtractionConfig` | Yes | The extraction config |

**Returns:** `bool`

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

**Example:**

```go
result := instance.Priority()
```

**Returns:** `int32`

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
| `WorksheetNames` | `\[\]string` | `nil` | Worksheet names |

---

#### XmlExtractionResult

XML extraction result.

Contains extracted text content from XML files along with
structural statistics about the XML document.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Content` | `string` | — | Extracted text content (XML structure filtered out) |
| `ElementCount` | `int` | — | Total number of XML elements processed |
| `UniqueElements` | `\[\]string` | — | List of unique element names found (sorted) |

---

#### XmlMetadata

XML metadata extracted during XML parsing.

Provides statistics about XML document structure.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `ElementCount` | `uint32` | — | Total number of XML elements processed |
| `UniqueElements` | `\[\]string` | `nil` | List of unique element tag names (sorted) |

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

**Example:**

```go
result := YakeParams.Default()
```

**Returns:** `YakeParams`

---

#### YearRange

Year range for bibliographic metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `Min` | `*uint32` | `nil` | Earliest (minimum) year in the range. |
| `Max` | `*uint32` | `nil` | Latest (maximum) year in the range. |
| `Years` | `\[\]uint32` | `/* serde(default) */` | All individual years present in the collection. |

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

#### ImageOutputFormat

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
| `Native` | Preserve whatever format the extractor produced (default). No re-encode pass is performed. `ExtractedImage.format` reflects the source format: JPEG for embedded PDF images, PNG for rasterised content, or the native container format from office documents. |
| `Png` | Re-encode all extracted images as PNG (lossless). |
| `Jpeg` | Re-encode all extracted images as JPEG at the given quality level. `quality` must be in `1..=100`. Values outside this range are clamped and a warning is emitted. Higher values produce larger files with less artefacting; 85 is a reasonable default. — Fields: `Quality`: `uint8` |
| `Webp` | Re-encode all extracted images as WebP at the given quality level. `quality` must be in `1..=100`. Values outside this range are clamped and a warning is emitted. 80 is a reasonable default. — Fields: `Quality`: `uint8` |
| `Heif` | Re-encode all extracted images as HEIF/HEIC at the given quality level. Requires the `heic` feature. `quality` must be in `1..=100`. Values outside this range are clamped and a warning is emitted. 80 is a reasonable default. — Fields: `Quality`: `uint8` |
| `Svg` | Output pure-vector SVG. Lossless. Raster sources are not re-encoded (a warning is emitted and the image bytes are left untouched). When the source is already SVG, the bytes are passed through the `usvg` sanitizer (strips external hrefs, JS event handlers, and `foreignObject` elements) when `SvgOptions.sanitize` is `true`. Requires the `svg` feature. |

---

#### ExtractInputKind

Source kind for `ExtractInput`.

| Value | Description |
|-------|-------------|
| `Bytes` | Raw in-memory bytes. |
| `Uri` | A filesystem path, `file://` URI, or HTTP(S) URL. |

---

#### UrlExtractionMode

URL extraction mode.

| Value | Description |
|-------|-------------|
| `Auto` | Classify HTTP(S) resources after fetch. |
| `Document` | Treat the URI as a single remote document/page. |
| `Crawl` | Crawl from the seed URI and extract discovered pages/documents. |

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

#### CallMode

How a structured-extraction preset is dispatched to the model.

This is the preset-facing call mode (the `preferred_call_mode` field of a
`Preset`). The richer runtime decision enum used by the
structured pipeline — which adds `Skip` and `TextOnlyWithVisionFallback` —
lives in `crate.heuristics.structured.StructuredCallMode`; this 3-variant
type is the stable, serializable surface presets and bindings depend on.

| Value | Description |
|-------|-------------|
| `TextOnly` | Use the extracted text only. |
| `VisionOnly` | Use rasterized page images only. |
| `TextPlusVision` | Provide both extracted text and page images to the model. |

---

#### MergeMode

How partial results from multiple model calls (e.g. per page batch) are combined.

Canonical home for the merge strategy referenced by presets and by the
structured pipeline's post-processing. There is intentionally only one merge
type across the crate — do not introduce a second.

| Value | Description |
|-------|-------------|
| `ObjectMerge` | Deep-merge JSON objects field by field (later calls fill missing fields). |
| `ArrayConcat` | Concatenate top-level arrays across calls. |
| `ObjectFirst` | Keep the first non-empty result; ignore subsequent calls. |

---

#### NerBackendKind

NER backend selector.

| Value | Description |
|-------|-------------|
| `Onnx` | `xberg-gliner` ONNX inference. Requires `ner-onnx` feature. Models download lazily from `xberg-io/gliner-models`. |
| `Llm` | liter-llm zero-shot NER via structured-output prompts. Requires `ner-llm` feature. Useful when domain-specific categories outstrip the ONNX taxonomy. |

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
| `Disabled` | No VLM fallback (default). Behaves identically to the pre-policy single-backend mode. |
| `OnLowQuality` | Try the classical OCR backend first. If the quality score is below `quality_threshold`, send the page to the VLM. `quality_threshold` is in the `\[0.0, 1.0\]` range produced by `calculate_quality_score`. A value of `0.5` is a reasonable starting point; calibrate with the Stage 0 benchmark harness. — Fields: `QualityThreshold`: `float64` |
| `Always` | Skip the classical OCR backend entirely. Every page is sent to the VLM. |

---

#### TableChunkingMode

Controls how markdown tables are handled when they exceed the chunk size limit.

Only applies when `chunker_type` is `Markdown`.

### Variants

- `Split` - Default behavior: tables are split at row boundaries like any
  other block element. Continuation chunks contain only data rows without
  the header, which can break downstream consumers that need column context.

- `RepeatHeader` - Prepend the table header (header row + separator row) to
  every continuation chunk that contains data rows from the same table.
  Adds a small amount of duplicate text but ensures each chunk is
  self-contained for extraction, search, and LLM consumption.

| Value | Description |
|-------|-------------|
| `Split` | Split tables at row boundaries (default). Continuation chunks have no header. |
| `RepeatHeader` | Prepend the table header to every chunk that continues a split table. |

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
| `Tokenizer` | Size measured in tokens from a HuggingFace tokenizer. — Fields: `Model`: `string`, `CacheDir`: `string` |

---

#### EmbeddingModelType

Embedding model types supported by Xberg.

| Value | Description |
|-------|-------------|
| `Preset` | Use a preset model configuration (recommended) — Fields: `Name`: `string` |
| `Custom` | Use a custom ONNX model from HuggingFace — Fields: `ModelId`: `string`, `Dimensions`: `int` |
| `Llm` | Provider-hosted embedding model via liter-llm. Uses the model specified in the nested `LlmConfig` (e.g., `"openai/text-embedding-3-small"`). — Fields: `Llm`: `LlmConfig` |
| `Plugin` | In-process embedding backend registered via the plugin system. The caller registers an `EmbeddingBackend` once (e.g. a wrapper around an already-loaded `llama-cpp-python`, `sentence-transformers`, or tuned ONNX model), then references it by name in config. Xberg calls back into the registered backend during chunking and standalone embed requests — no HuggingFace download, no ONNX Runtime requirement, no HTTP sidecar. When this variant is selected, only the following `EmbeddingConfig` fields apply: `normalize` (post-call L2 normalization) and `max_embed_duration_secs` (dispatcher timeout). Model-loading fields (`batch_size`, `cache_dir`, `show_download_progress`, `acceleration`) are ignored — the host owns the model lifecycle. Semantic chunking falls back to `ChunkingConfig.max_characters` when this variant is used, since there is no preset to look a chunk-size ceiling up against — size your context window via `max_characters` directly. See `register_embedding_backend`. — Fields: `Name`: `string` |

---

#### RerankerModelType

Reranker model types supported by Xberg.

Since v5.0.

| Value | Description |
|-------|-------------|
| `Preset` | Use a preset cross-encoder model (recommended). — Fields: `Name`: `string` |
| `Custom` | Use a custom ONNX cross-encoder from HuggingFace. — Fields: `ModelId`: `string`, `ModelFile`: `string`, `AdditionalFiles`: `\[\]string`, `MaxLength`: `int64` |
| `Llm` | Provider-hosted reranker via liter-llm (e.g. Cohere, Jina, Voyage). The model in the nested `LlmConfig` must be a rerank-capable model ID (e.g. `"cohere/rerank-english-v3.0"`). — Fields: `Llm`: `LlmConfig` |
| `Plugin` | In-process reranker registered via the plugin system. The caller registers a `RerankerBackend` once (e.g. a wrapper around a `sentence-transformers` cross-encoder or a provider client), then references it by name in config. Xberg calls back into the registered backend — no HuggingFace download, no ONNX Runtime requirement. When this variant is selected, only `max_rerank_duration_secs` applies. Model-loading fields (`batch_size`, `cache_dir`, `show_download_progress`, `acceleration`) are ignored — the host owns the model lifecycle. See `register_reranker_backend`. — Fields: `Name`: `string` |

---

#### WhisperModel

Supported Whisper model sizes.

These map to published ONNX exports on Hugging Face (onnx-community or
similar orgs). The actual filenames and repos are resolved inside the
transcription engine.

| Value | Description |
|-------|-------------|
| `Tiny` | Smallest, fastest, lowest quality. Good default for development and CI. |
| `Base` | Reasonable quality/speed tradeoff. |
| `Small` | Better accuracy with higher memory and cache use. |
| `Medium` | High quality; slower and more memory-intensive. |
| `LargeV3` | Best quality (large-v3). Use only when latency and memory use are acceptable. |

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
| `Candle` | Candle-based VLM OCR (TrOCR, PaddleOCR-VL). |
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
| `Title` | Document title. — Fields: `Text`: `string` |
| `Heading` | Section heading with level (1-6). — Fields: `Level`: `uint8`, `Text`: `string` |
| `Paragraph` | Body text paragraph. — Fields: `Text`: `string` |
| `List` | List container — children are `ListItem` nodes. — Fields: `Ordered`: `bool` |
| `ListItem` | Individual list item. — Fields: `Text`: `string` |
| `Table` | Table with structured cell grid. — Fields: `Grid`: `TableGrid` |
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
| `MetadataBlock` | Structured metadata block (email headers, YAML frontmatter, etc.). |

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
| `Link` | Hyperlink annotation. — Fields: `Url`: `string`, `Title`: `string` |
| `Highlight` | Highlighted text (PDF highlights, HTML `<mark>`). |
| `Color` | Text color (CSS-compatible value, e.g. "#ff0000", "red"). — Fields: `Value`: `string` |
| `FontSize` | Font size with units (e.g. "12pt", "1.2em", "16px"). — Fields: `Value`: `string` |
| `Custom` | Extensible annotation for format-specific styling. — Fields: `Name`: `string`, `Value`: `string` |

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

#### FormFieldType

Kind of a PDF form field.

Mirrors `pdf_oxide`'s widget field taxonomy without leaking the upstream
type across the binding surface.

| Value | Description |
|-------|-------------|
| `Text` | Single- or multi-line text input. |
| `Checkbox` | Checkbox (on/off toggle). |
| `Radio` | Radio-button group member. |
| `Choice` | Choice field (dropdown or list box). |
| `Signature` | Digital-signature field. |
| `Button` | Push button. |
| `Unknown` | Field type that could not be classified. |

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
| `Code` | Code (tree-sitter analyzable source). The structured analysis result is exposed via `ExtractionResult.code_intelligence`; this variant only tags the format. |

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
| `Quadrilateral` | 4-point quadrilateral for rotated/skewed text (PaddleOCR). Points are in clockwise order starting from top-left: `\[top_left, top_right, bottom_right, bottom_left\]` |

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
| `Mask` | Replace the matched span with a fixed mask token (default `"\[REDACTED\]"`). |
| `Hash` | Replace with a SHA-256 hash of the original value (truncated to 16 hex chars). Lets downstream consumers do equality joins without recovering the source. |
| `TokenReplace` | Replace with a per-category running token (`"\[PERSON_1\]"`, `"\[PERSON_2\]"`, …) so the same person referenced twice gets the same token within the document. |
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
| `Paragraph` | Body paragraph, identified by its zero-based index in the document flow. — Fields: `Index`: `int` |
| `TableCell` | Cell inside a table. — Fields: `Row`: `int`, `Col`: `int`, `TableIndex`: `int` |
| `Page` | Page, identified by its zero-based index. — Fields: `Index`: `int` |
| `Slide` | Presentation slide, identified by its zero-based index. — Fields: `Index`: `int` |
| `Sheet` | Spreadsheet cell or range, identified by sheet index and optional name. — Fields: `Index`: `int`, `Name`: `string` |

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

#### EnrichStatus

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
| `Pending` | Job submitted; processing has not yet started or is in progress. |
| `Completed` | Processing completed successfully. — Fields: `Result`: `EnrichResult` |
| `Failed` | Processing failed. — Fields: `Error`: `string` |

---

#### SchemaCompliance

Schema-validation outcome surfaced as one of three buckets.

Fold into the combined confidence score without leaking internal validation
error types.

| Value | Description |
|-------|-------------|
| `AllValid` | Every batch validated against the schema. |
| `PartialValid` | At least one batch validated; at least one did not. |
| `AllInvalid` | No batch validated. |

---

#### ChunkingDecision

The chunking decision made by the analyzer.

| Value | Description |
|-------|-------------|
| `NoChunking` | Process without chunking (small file, text layer detected, etc.) — Fields: `Reason`: `NoChunkingReason` |
| `Chunk` | Chunk according to plan. — Fields: `0`: `ChunkPlan` |
| `UseOverrides` | Use user-provided chunk overrides. — Fields: `UserChunks`: `\[\]PageRange` |

---

#### NoChunkingReason

Reason for not chunking a document.

| Value | Description |
|-------|-------------|
| `SmallFile` | File is below size threshold. — Fields: `SizeBytes`: `uint64`, `ThresholdBytes`: `uint64` |
| `FewPages` | Document has fewer pages than threshold. — Fields: `PageCount`: `uint32`, `Threshold`: `uint32` |
| `TextLayerDetected` | PDF has substantial text layer (OCR not needed). — Fields: `TextCoverage`: `float32`, `AvgCharsPerPage`: `uint32` |
| `FormatNotChunkable` | Document format does not support chunking. — Fields: `MimeType`: `string` |
| `ChunkingDisabled` | Chunking is disabled by configuration. |
| `FastTextExtraction` | Force OCR is disabled and text extraction is fast. |

---

#### ChunkingReason

Reason for chunking a document.

| Value | Description |
|-------|-------------|
| `LargeFile` | File exceeds size threshold. — Fields: `SizeBytes`: `uint64`, `ThresholdBytes`: `uint64` |
| `ManyPages` | Document has many pages. — Fields: `PageCount`: `uint32`, `Threshold`: `uint32` |
| `OcrRequired` | PDF requires OCR and is large. — Fields: `PageCount`: `uint32`, `ForceOcr`: `bool` |
| `LargeAndManyPages` | Both size and page count exceed thresholds. — Fields: `SizeBytes`: `uint64`, `PageCount`: `uint32` |

---

#### BoundaryReason

Reason for boundary detection.

| Value | Description |
|-------|-------------|
| `Start` | Start of PDF. |
| `PageOneMarker` | Page-one marker ("Page 1", "1 of N") detected. |
| `LetterheadReset` | Letterhead reset after signature block. |
| `DensityShift` | Text density shift with low bigram overlap. |
| `End` | End of PDF. |

---

#### StructuredCallMode

Outcome of the structured-extraction call-mode heuristic.

**Distinct from `crate.core.config.CallMode`** which has three variants
and governs extraction-engine behaviour.  This enum governs whether and how
an already-extracted document is sent to an LLM structured-extraction
pipeline.

| Value | Description |
|-------|-------------|
| `Skip` | Document is unsupported or not worth invoking the pipeline. |
| `TextOnly` | Send extracted text only; no vision model call. |
| `VisionOnly` | Send page rasters only; no extracted text payload. |
| `TextPlusVision` | Fuse extracted text with page rasters in a single multimodal call. |
| `TextOnlyWithVisionFallback` | Try text-only first; escalate to vision on low confidence score. |

---

#### PresetCategory

High-level category used to group presets in the registry UI.

| Value | Description |
|-------|-------------|
| `Finance` | Invoices, receipts, statements, purchase orders, W-9. |
| `Identity` | Passports, drivers licenses, insurance cards. |
| `Legal` | Contracts, NDAs, agreements. |
| `Logistics` | Bills of lading, customs declarations, packing lists. |
| `Medical` | Clinical records, lab reports. |
| `Hr` | Pay stubs, resumes, employment offers. |
| `Other` | Catch-all for documents that don't fit the other categories. |

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

The 18 canonical document layout classes.

All model backends (RT-DETR, YOLO, etc.) map their native class IDs
to this shared set. Models with fewer classes (DocLayNet: 11, PubLayNet: 5)
map to the closest equivalent.

Wire format is snake_case in all serializers (JSON, TOML, YAML).

| Value | Description |
|-------|-------------|
| `Caption` | Figure or table caption text. |
| `Chart` | Chart or graph visualization. |
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

#### XbergError

Main error type for all Xberg operations.

All errors in Xberg use this enum, which preserves error chains
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
| `Reranking` | The reranker model or reranking pipeline returned an error. Since v5.0. |
| `Transcription` | Audio/video transcription failed. |
| `Timeout` | The extraction operation exceeded the configured time limit. |
| `Cancelled` | The extraction was cancelled via a `CancellationToken`. |
| `Security` | A security policy was violated (e.g. zip bomb, oversized archive). |
| `Other` | A catch-all for uncommon errors that do not fit another variant. |

---

#### HeuristicsError

Errors that can occur during heuristics analysis.

| Variant | Description |
|---------|-------------|
| `ConfigError` | Invalid configuration value. |
| `PdfAnalysisError` | PDF analysis step failed (only when `heuristics-pdf` feature is active). |

---

#### LoadError

Errors produced while loading or validating a preset file.

| Variant | Description |
|---------|-------------|
| `Parse` | The file is not valid JSON. |
| `SchemaValidation` | The file parses as JSON but does not validate against the meta-schema. |
| `Deserialize` | The file validates but cannot be deserialized into `Preset`. |
| `IdMismatch` | The preset's declared `id` does not match its file-system location. |
| `BadMetaSchema` | The meta-schema itself failed to compile. |
| `Io` | A filesystem I/O error occurred while reading a preset directory. |

---

#### ResolveError

Errors produced while resolving a preset against caller overrides.

| Variant | Description |
|---------|-------------|
| `SchemaNotObject` | A custom schema override was supplied but is not a JSON object. |

---

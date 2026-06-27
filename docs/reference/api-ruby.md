---
title: "Ruby API Reference"
---

## Ruby API Reference <span class="version-badge">v1.0.0-rc.1</span>

### Functions

#### extract()

Extract content from a single bytes or URI input.

**Signature:**

```ruby
def self.extract(input, config)
```

**Example:**

```ruby
result = extract(ExtractInput.new, ExtractionConfig.new)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `input` | `ExtractInput` | Yes | The input data |
| `config` | `ExtractionConfig` | Yes | The configuration options |

**Returns:** `ExtractionOutput`

**Errors:** Raises `Error`.

---

#### extract_batch()

Extract content from multiple bytes or URI inputs.

**Signature:**

```ruby
def self.extract_batch(inputs, config)
```

**Example:**

```ruby
result = extract_batch([], ExtractionConfig.new)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `inputs` | `Array<ExtractInput>` | Yes | The inputs |
| `config` | `ExtractionConfig` | Yes | The configuration options |

**Returns:** `ExtractionOutput`

**Errors:** Raises `Error`.

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

Returns `XbergError.UnsupportedFormat` if MIME type cannot be determined.

**Signature:**

```ruby
def self.detect_mime_type_from_bytes(content)
```

**Example:**

```ruby
result = detect_mime_type_from_bytes("data")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `content` | `String` | Yes | Raw file bytes |

**Returns:** `String`

**Errors:** Raises `Error`.

---

#### get_extensions_for_mime()

Get file extensions for a given MIME type.

Returns all known file extensions that map to the specified MIME type.

**Returns:**

A vector of file extensions (without leading dot) for the MIME type.

**Signature:**

```ruby
def self.get_extensions_for_mime(mime_type)
```

**Example:**

```ruby
result = get_extensions_for_mime("value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `mime_type` | `String` | Yes | The MIME type to look up |

**Returns:** `Array<String>`

**Errors:** Raises `Error`.

---

#### list_supported_formats()

List all supported document formats.

Returns every file extension Xberg recognizes together with its
corresponding MIME type, derived from the central format registry.
Formats that have no registered file extension (such as source code,
which is detected dynamically) are not included.

The list is sorted alphabetically by file extension.

**Returns:**

A vector of `SupportedFormat` entries sorted by extension.

**Signature:**

```ruby
def self.list_supported_formats()
```

**Example:**

```ruby
result = list_supported_formats()
```

**Returns:** `Array<SupportedFormat>`

---

#### detect_qr_codes()

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

```ruby
def self.detect_qr_codes(image_bytes, format_hint: nil)
```

**Example:**

```ruby
result = detect_qr_codes("data", format_hint: "value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `image_bytes` | `String` | Yes | The image bytes |
| `format_hint` | `String?` | No | The  format hint |

**Returns:** `Array<QrCode>`

---

#### clear_embedding_backends()

Clear all embedding backends from the global registry.

Calls `shutdown()` on every registered backend, then empties the registry.

**Errors:**

- Any error returned by a backend's `shutdown()` method. The first error
  encountered stops processing of remaining backends.

**Signature:**

```ruby
def self.clear_embedding_backends()
```

**Example:**

```ruby
clear_embedding_backends()
```

**Returns:** No return value.

**Errors:** Raises `Error`.

---

#### list_embedding_backends()

List the names of all registered embedding backends.

Used by `xberg-cli`, the api/mcp endpoints, and generated language
bindings.

**Signature:**

```ruby
def self.list_embedding_backends()
```

**Example:**

```ruby
result = list_embedding_backends()
```

**Returns:** `Array<String>`

**Errors:** Raises `Error`.

---

#### list_ocr_backends()

List all registered OCR backends.

Returns the names of all OCR backends currently registered in the global registry.

**Returns:**

A vector of OCR backend names.

**Signature:**

```ruby
def self.list_ocr_backends()
```

**Example:**

```ruby
result = list_ocr_backends()
```

**Returns:** `Array<String>`

**Errors:** Raises `Error`.

---

#### clear_ocr_backends()

Clear all OCR backends from the global registry.

Removes all OCR backends and calls their `shutdown()` methods.

**Returns:**

- `Ok(())` if all backends were cleared successfully
- `Err(...)` if any shutdown method failed

**Signature:**

```ruby
def self.clear_ocr_backends()
```

**Example:**

```ruby
clear_ocr_backends()
```

**Returns:** No return value.

**Errors:** Raises `Error`.

---

#### register_builtin()

Register every built-in post-processor enabled by the active feature set.

This is the single entry point that callers (including
`register_default_post_processors`) use to populate the global
post-processor registry with the in-tree built-ins. Each submodule's own
`register` function is gated by its feature flag so this aggregate stays
safe to call on any target.

**Signature:**

```ruby
def self.register_builtin()
```

**Example:**

```ruby
register_builtin()
```

**Returns:** No return value.

**Errors:** Raises `Error`.

---

#### list_post_processors()

List all registered post-processor names.

Returns a vector of all post-processor names currently registered in the
global registry.

**Returns:**

- `Ok(Array<String>)` - Vector of post-processor names
- `Err(...)` if the registry lock is poisoned

**Signature:**

```ruby
def self.list_post_processors()
```

**Example:**

```ruby
result = list_post_processors()
```

**Returns:** `Array<String>`

**Errors:** Raises `Error`.

---

#### clear_post_processors()

Remove all registered post-processors.

**Signature:**

```ruby
def self.clear_post_processors()
```

**Example:**

```ruby
clear_post_processors()
```

**Returns:** No return value.

**Errors:** Raises `Error`.

---

#### list_renderers()

List names of all registered renderers.

**Errors:**

Returns an error if the registry lock is poisoned.

**Signature:**

```ruby
def self.list_renderers()
```

**Example:**

```ruby
result = list_renderers()
```

**Returns:** `Array<String>`

**Errors:** Raises `Error`.

---

#### clear_renderers()

Clear all renderers from the global registry.

Removes every renderer, including the built-in defaults (markdown, html,
djot, plain). After calling this no renderers are registered; re-register
as needed.

**Errors:**

Returns an error if the registry lock is poisoned.

**Signature:**

```ruby
def self.clear_renderers()
```

**Example:**

```ruby
clear_renderers()
```

**Returns:** No return value.

**Errors:** Raises `Error`.

---

#### clear_reranker_backends()

Clear all reranker backends from the global registry.

Calls `shutdown()` on every registered backend, then empties the registry.

**Errors:**

- Any error returned by a backend's `shutdown()` method. The first error
  encountered stops processing of remaining backends.

Since v5.0.

**Signature:**

```ruby
def self.clear_reranker_backends()
```

**Example:**

```ruby
clear_reranker_backends()
```

**Returns:** No return value.

**Errors:** Raises `Error`.

---

#### list_reranker_backends()

List the names of all registered reranker backends.

Used by `xberg-cli`, the api/mcp endpoints, and generated language
bindings.

Since v5.0.

**Signature:**

```ruby
def self.list_reranker_backends()
```

**Example:**

```ruby
result = list_reranker_backends()
```

**Returns:** `Array<String>`

**Errors:** Raises `Error`.

---

#### list_validators()

List names of all registered validators.

**Signature:**

```ruby
def self.list_validators()
```

**Example:**

```ruby
result = list_validators()
```

**Returns:** `Array<String>`

**Errors:** Raises `Error`.

---

#### clear_validators()

Remove all registered validators.

**Signature:**

```ruby
def self.clear_validators()
```

**Example:**

```ruby
clear_validators()
```

**Returns:** No return value.

**Errors:** Raises `Error`.

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

```ruby
def self.classify_pages(result, config)
```

**Example:**

```ruby
classify_pages(ExtractionResult.new, PageClassificationConfig.new)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `result` | `ExtractionResult` | Yes | The extraction result |
| `config` | `PageClassificationConfig` | Yes | The configuration options |

**Returns:** No return value.

**Errors:** Raises `Error`.

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

```ruby
def self.classify_text(text, config)
```

**Example:**

```ruby
result = classify_text("value", PageClassificationConfig.new)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `String` | Yes | The text |
| `config` | `PageClassificationConfig` | Yes | The configuration options |

**Returns:** `Array<ClassificationLabel>`

**Errors:** Raises `Error`.

---

#### classify_document()

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

```ruby
def self.classify_document(pages, config)
```

**Example:**

```ruby
result = classify_document([], PageClassificationConfig.new)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `pages` | `Array<String>` | Yes | Slice of page texts to classify. Each page is classified independently |
| `config` | `PageClassificationConfig` | Yes | Classification configuration including labels and LLM settings. |

**Returns:** `Array<ClassificationLabel>`

**Errors:** Raises `Error`.

---

#### download_model()

Eagerly download a NER model into the xberg cache.

`name` is a supported xberg GLiNER alias or catalog id. The CLI flag
`xberg cache warm --ner` delegates here.

**Signature:**

```ruby
def self.download_model(name, cache_dir: nil)
```

**Example:**

```ruby
result = download_model("value", cache_dir: "value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `name` | `String` | Yes | The name |
| `cache_dir` | `String?` | No | The cache dir |

**Returns:** `String`

**Errors:** Raises `Error`.

---

#### download_model()

**Signature:**

```ruby
def self.download_model(name, cache_dir: nil)
```

**Example:**

```ruby
result = download_model("value", cache_dir: "value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `name` | `String` | Yes | The  name |
| `cache_dir` | `String?` | No | The  cache dir |

**Returns:** `String`

**Errors:** Raises `Error`.

---

#### default_model_name()

Pinned default NER model identifier.

**Signature:**

```ruby
def self.default_model_name()
```

**Example:**

```ruby
result = default_model_name()
```

**Returns:** `String`

---

#### default_model_name()

**Signature:**

```ruby
def self.default_model_name()
```

**Example:**

```ruby
result = default_model_name()
```

**Returns:** `String`

---

#### known_models()

All NER models xberg knows about (used by `--all-ner-models`).

**Signature:**

```ruby
def self.known_models()
```

**Example:**

```ruby
result = known_models()
```

**Returns:** `Array<String>`

---

#### known_models()

**Signature:**

```ruby
def self.known_models()
```

**Example:**

```ruby
result = known_models()
```

**Returns:** `Array<String>`

---

#### download_model()

Download a NER model into the xberg cache.

**Signature:**

```ruby
def self.download_model(name, cache_dir: nil)
```

**Example:**

```ruby
result = download_model("value", cache_dir: "value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `name` | `String` | Yes | The  name |
| `cache_dir` | `String?` | No | The  cache dir |

**Returns:** `String`

**Errors:** Raises `Error`.

---

#### default_model_name()

Default NER model identifier.

**Signature:**

```ruby
def self.default_model_name()
```

**Example:**

```ruby
result = default_model_name()
```

**Returns:** `String`

---

#### known_models()

All NER models xberg knows about.

**Signature:**

```ruby
def self.known_models()
```

**Example:**

```ruby
result = known_models()
```

**Returns:** `Array<String>`

---

#### redact()

Run pattern redaction (and optional NER-driven redaction) over `result` and
rewrite every textual field. Populates `result.redaction_report`.

**Signature:**

```ruby
def self.redact(result, config)
```

**Example:**

```ruby
redact(ExtractionResult.new, RedactionConfig.new)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `result` | `ExtractionResult` | Yes | The extraction result |
| `config` | `RedactionConfig` | Yes | The configuration options |

**Returns:** No return value.

**Errors:** Raises `Error`.

---

#### find_all()

Find all US Social Security Number spans in `text` (format: NNN-NN-NNNN).

**Signature:**

```ruby
def self.find_all(text)
```

**Example:**

```ruby
result = find_all("value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `String` | Yes | The text |

**Returns:** `Array<PatternMatch>`

---

#### scan_text()

Scan `text` for every PII category in `categories` and return all matches
in source-byte order.

When `categories` is empty every supported regex-detectable category fires.
Person / Organization / Location are *not* covered by the pattern engine —
they must be supplied by a NER backend through the redaction engine.

**Signature:**

```ruby
def self.scan_text(text, categories)
```

**Example:**

```ruby
result = scan_text("value", [])
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `String` | Yes | The text |
| `categories` | `Array<PiiCategory>` | Yes | The categories |

**Returns:** `Array<PatternMatch>`

---

#### summarize()

Score and return the top-N sentences from `text`, joined in original order.

`language` is an ISO 639 (or locale) code used to pick a stopword list;
pass `nil` (or an unknown code) to fall back to English.
`max_tokens` bounds the summary length by whitespace-separated tokens;
`nil` falls back to `DEFAULT_MAX_TOKENS`.

**Signature:**

```ruby
def self.summarize(text, language: nil, max_tokens: nil)
```

**Example:**

```ruby
result = summarize("value", language: "value", max_tokens: 42)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `String` | Yes | The text |
| `language` | `String?` | No | The language |
| `max_tokens` | `Integer?` | No | The max tokens |

**Returns:** `String?`

---

#### token_count()

Count whitespace-separated tokens (used for token-budget bookkeeping by
callers).

**Signature:**

```ruby
def self.token_count(text)
```

**Example:**

```ruby
result = token_count("value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `String` | Yes | The text |

**Returns:** `Integer`

---

#### translate_result()

Translate the extraction result in place.

Populates `result.translation` with the translated `content`, optionally the
translated `formatted_content` (when `preserve_markup = true`), and rewrites
every chunk's `content` field. Every LLM call's usage is appended to
`result.llm_usage`.

**Signature:**

```ruby
def self.translate_result(result, config)
```

**Example:**

```ruby
translate_result(ExtractionResult.new, TranslationConfig.new)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `result` | `ExtractionResult` | Yes | The extraction result |
| `config` | `TranslationConfig` | Yes | The configuration options |

**Returns:** No return value.

**Errors:** Raises `Error`.

---

#### find_footnote_anchors()

Find all footnote anchor references in markdown text.

Returns a vector of footnote anchors (`[^label]` use-sites), including byte offsets.
Footnote definitions (`[^label]: ...`) are NOT included in the results.

**Returns:**

A vector of `FootnoteAnchor` entries, each with the label and byte offset.

**Signature:**

```ruby
def self.find_footnote_anchors(markdown)
```

**Example:**

```ruby
result = find_footnote_anchors("value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `markdown` | `String` | Yes | The markdown text to search |

**Returns:** `Array<FootnoteAnchor>`

---

#### parse_footnote_definitions()

Parse footnote definitions from markdown text.

Returns a vector of footnote definitions found in the markdown.
Handles multi-line definitions with continuation/indented lines (CommonMark format).

**Returns:**

A vector of `FootnoteDefinition` entries, each with label, content, and byte offset.

**Signature:**

```ruby
def self.parse_footnote_definitions(markdown)
```

**Example:**

```ruby
result = parse_footnote_definitions("value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `markdown` | `String` | Yes | The markdown text to search |

**Returns:** `Array<FootnoteDefinition>`

---

#### find_inference_markers()

Find inference markers in markdown text.

Returns byte offsets of every `[*inference*]` marker found in the text.

**Returns:**

A vector of byte offsets where inference markers appear.

**Signature:**

```ruby
def self.find_inference_markers(markdown)
```

**Example:**

```ruby
result = find_inference_markers("value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `markdown` | `String` | Yes | The markdown text to search |

**Returns:** `Array<Integer>`

---

#### find_unmarked_claims()

Find unmarked claims in markdown text.

Returns lines that assert a claim but carry neither a footnote citation anchor (`[^...]`)
nor an inference marker (`[*inference*]`).

The heuristic is simple: a line that contains alphabetic words, ends with sentence punctuation,
and is not a heading, blank line, or markup-only line is considered a claim.
Exclude lines that appear in the citation block (after `---` + `<!-- citations ... -->`).

**Returns:**

A vector of trimmed line text strings for unmarked claims.

**Signature:**

```ruby
def self.find_unmarked_claims(markdown)
```

**Example:**

```ruby
result = find_unmarked_claims("value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `markdown` | `String` | Yes | The markdown text to search |

**Returns:** `Array<String>`

---

#### parse_citations()

Parse the structured citation block from markdown.

Extracts citations from the block after a `---` thematic break followed by
`<!-- citations ... -->` comment. Parses each entry as:
`[^srcN]: <source>, <optional-locator>, excerpt: "<text>"`

Returns parsed citations with source, optional locator, and optional excerpt.

**Returns:**

A vector of `Citation` entries parsed from the citation block.

**Signature:**

```ruby
def self.parse_citations(markdown)
```

**Example:**

```ruby
result = parse_citations("value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `markdown` | `String` | Yes | The markdown text to search |

**Returns:** `Array<Citation>`

---

#### verify_excerpt()

Verify that an excerpt appears verbatim in source text.

Performs exact matching by default. Also tries whitespace-normalized matching
(collapsing runs of whitespace on both sides) since PDF-extracted text often
has irregular spacing.

**Returns:**

`true` if the excerpt appears (exactly or with normalized whitespace), `false` otherwise.

**Signature:**

```ruby
def self.verify_excerpt(excerpt, source_text)
```

**Example:**

```ruby
result = verify_excerpt("value", "value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `excerpt` | `String` | Yes | The text snippet to find |
| `source_text` | `String` | Yes | The full source text to search |

**Returns:** `Boolean`

---

#### chunk_for_rag()

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

```ruby
def self.chunk_for_rag(text, config)
```

**Example:**

```ruby
result = chunk_for_rag("value", ChunkingConfig.new)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `String` | Yes | The text |
| `config` | `ChunkingConfig` | Yes | The configuration options |

**Returns:** `ChunkingResult`

**Errors:** Raises `Error`.

---

#### compare()

Compare two extraction results and return a structured diff.

The comparison is purely structural — no I/O, no side effects. All fields
of `ExtractionDiff` are populated according to the provided `DiffOptions`.

**Signature:**

```ruby
def self.compare(a, b, opts)
```

**Example:**

```ruby
result = compare(ExtractionResult.new, ExtractionResult.new, DiffOptions.new)
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

```ruby
def self.extract_region_with_vlm(image_bytes, image_mime, region_kind, llm_config, custom_prompt: nil)
```

**Example:**

```ruby
result = extract_region_with_vlm("data", "value", RegionKind.new, LlmConfig.new, custom_prompt: "value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `image_bytes` | `String` | Yes | The image bytes |
| `image_mime` | `String` | Yes | The image mime |
| `region_kind` | `RegionKind` | Yes | The region kind |
| `llm_config` | `LlmConfig` | Yes | The llm config |
| `custom_prompt` | `String?` | No | The custom prompt |

**Returns:** `String`

**Errors:** Raises `Error`.

---

#### rerank_async()

Rerank documents asynchronously.

Async counterpart to `rerank`. Offloads blocking ONNX inference to a
dedicated blocking thread pool via Tokio's `spawn_blocking`, keeping the
async executor free.

Since v5.0.

**Signature:**

```ruby
def self.rerank_async(query, documents, config)
```

**Example:**

```ruby
result = rerank_async("value", [], RerankerConfig.new)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `query` | `String` | Yes | The query |
| `documents` | `Array<String>` | Yes | The documents |
| `config` | `RerankerConfig` | Yes | The configuration options |

**Returns:** `Array<RerankedDocument>`

**Errors:** Raises `Error`.

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

```ruby
def self.extract_keywords(text, config)
```

**Example:**

```ruby
result = extract_keywords("value", KeywordConfig.new)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `String` | Yes | The text to extract keywords from |
| `config` | `KeywordConfig` | Yes | Keyword extraction configuration |

**Returns:** `Array<Keyword>`

**Errors:** Raises `Error`.

---

#### analyze_document()

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

```ruby
def self.analyze_document(metadata, config, document_bytes: nil)
```

**Example:**

```ruby
result = analyze_document(DocumentMetadata.new, HeuristicsConfig.new, document_bytes: "data")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `metadata` | `DocumentMetadata` | Yes | The document metadata |
| `config` | `HeuristicsConfig` | Yes | The configuration options |
| `document_bytes` | `String?` | No | The document bytes |

**Returns:** `ChunkingDecision`

**Errors:** Raises `Error`.

---

#### analyze_with_user_chunks()

Analyze a document with user-specified chunk ranges.

Creates a chunk plan based on user-provided page ranges.

**Signature:**

```ruby
def self.analyze_with_user_chunks(user_ranges, total_pages, size_bytes, config)
```

**Example:**

```ruby
result = analyze_with_user_chunks([], 42, 42, HeuristicsConfig.new)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `user_ranges` | `Array<PageRange>` | Yes | The user ranges |
| `total_pages` | `Integer` | Yes | The total pages |
| `size_bytes` | `Integer` | Yes | The size bytes |
| `config` | `HeuristicsConfig` | Yes | The configuration options |

**Returns:** `ChunkingDecision`

---

#### score_confidence()

Score a `ConfidenceSignals` triple into an `ExtractionConfidence` using
the supplied weights.

When `signals.ocr_aggregate` is `nil`, the OCR weight folds into
`text_coverage` so the weighted sum still totals 1.0.

**Signature:**

```ruby
def self.score_confidence(signals, weights)
```

**Example:**

```ruby
result = score_confidence(ConfidenceSignals.new, ConfidenceWeights.new)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `signals` | `ConfidenceSignals` | Yes | The confidence signals |
| `weights` | `ConfidenceWeights` | Yes | The confidence weights |

**Returns:** `ExtractionConfidence`

---

#### check_format_limits()

Decision returned for pre-extraction rejection based on XLSX/PPTX-specific
resource bounds. Returns `Some(reason)` to reject; `nil` to proceed.

Callers must provide counts from a pre-extraction peek (e.g. parsing
`xl/workbook.xml` for sheet count).

**Signature:**

```ruby
def self.check_format_limits(mime_type, sheet_count: nil, workbook_cells: nil, embedded_count: nil, config)
```

**Example:**

```ruby
result = check_format_limits("value", sheet_count: 42, workbook_cells: 42, embedded_count: 42, HeuristicsConfig.new)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `mime_type` | `String` | Yes | The mime type |
| `sheet_count` | `Integer?` | No | The sheet count |
| `workbook_cells` | `Integer?` | No | The workbook cells |
| `embedded_count` | `Integer?` | No | The embedded count |
| `config` | `HeuristicsConfig` | Yes | The configuration options |

**Returns:** `String?`

---

#### boundaries_from_extraction_result()

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

```ruby
def self.boundaries_from_extraction_result(result, thresholds)
```

**Example:**

```ruby
result = boundaries_from_extraction_result(ExtractionResult.new, MultidocThresholds.new)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `result` | `ExtractionResult` | Yes | The extraction result |
| `thresholds` | `MultidocThresholds` | Yes | The multidoc thresholds |

**Returns:** `Array<DocumentBoundary>`

---

#### detect_boundaries()

Detect document boundaries in a multi-document PDF.

Returns a list of detected boundaries, always including implicit boundaries
at start (page 1) and end (page_count).  Boundaries are returned in ascending
order of `start_page`.

**Returns:**

Ordered list of document boundaries.

**Signature:**

```ruby
def self.detect_boundaries(input, thresholds)
```

**Example:**

```ruby
result = detect_boundaries(MultidocInput.new, MultidocThresholds.new)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `input` | `MultidocInput` | Yes | Page signals for the PDF |
| `thresholds` | `MultidocThresholds` | Yes | Detection thresholds |

**Returns:** `Array<DocumentBoundary>`

---

#### choose_call_mode()

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

```ruby
def self.choose_call_mode(input, t)
```

**Example:**

```ruby
result = choose_call_mode(StructuredInput.new, StructuredThresholds.new)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `input` | `StructuredInput` | Yes | The input data |
| `t` | `StructuredThresholds` | Yes | The structured thresholds |

**Returns:** `StructuredCallMode`

---

#### calculate_chunk_plan()

Calculate a chunking plan for a document.

**Returns:**

A `ChunkPlan` with optimal chunk boundaries.

**Signature:**

```ruby
def self.calculate_chunk_plan(page_count, size_bytes, needs_ocr, config)
```

**Example:**

```ruby
result = calculate_chunk_plan(42, 42, true, HeuristicsConfig.new)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `page_count` | `Integer` | Yes | Total number of pages in the document |
| `size_bytes` | `Integer` | Yes | File size in bytes |
| `needs_ocr` | `Boolean` | Yes | Whether OCR will be required |
| `config` | `HeuristicsConfig` | Yes | Heuristics configuration |

**Returns:** `ChunkPlan`

---

#### calculate_plan_from_overrides()

Calculate a chunk plan from user-specified page ranges.

Validates and processes user overrides into a proper chunk plan.

**Signature:**

```ruby
def self.calculate_plan_from_overrides(user_chunks, total_pages, size_bytes, config)
```

**Example:**

```ruby
result = calculate_plan_from_overrides([], 42, 42, HeuristicsConfig.new)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `user_chunks` | `Array<PageRange>` | Yes | The user chunks |
| `total_pages` | `Integer` | Yes | The total pages |
| `size_bytes` | `Integer` | Yes | The size bytes |
| `config` | `HeuristicsConfig` | Yes | The configuration options |

**Returns:** `ChunkPlan`

---

#### fingerprint()

Stable sha256 fingerprint of `raw`, formatted as `sha256:<hex>`.

**Signature:**

```ruby
def self.fingerprint(raw)
```

**Example:**

```ruby
result = fingerprint("data")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `raw` | `String` | Yes | The raw |

**Returns:** `String`

---

#### resolve()

Resolve `(preset, custom_schema_override, context)` into a `ResolvedPreset`.

- `custom_schema` overrides `preset.schema` when set.
- `context` substitutes `{{key}}` tokens in `preset.context_template`; the
  rendered string is appended to `system_prompt` so the model sees it.

**Signature:**

```ruby
def self.resolve(preset, custom_schema: nil, context)
```

**Example:**

```ruby
result = resolve(Preset.new, custom_schema: [], [])
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `preset` | `Preset` | Yes | The preset |
| `custom_schema` | `Object?` | No | The custom schema |
| `context` | `Hash{String=>String}` | Yes | The context |

**Returns:** `ResolvedPreset`

**Errors:** Raises `ResolveError`.

---

#### extract_structured_json()

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

```ruby
def self.extract_structured_json(bytes, mime, preset_spec_json, options_json)
```

**Example:**

```ruby
result = extract_structured_json("data", "value", "value", "value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `bytes` | `String` | Yes | The bytes |
| `mime` | `String` | Yes | The mime |
| `preset_spec_json` | `String` | Yes | The preset spec json |
| `options_json` | `String` | Yes | The options json |

**Returns:** `String`

**Errors:** Raises `Error`.

---

#### split_and_extract_json()

Split a multi-document PDF and extract structured JSON from each segment,
returning a JSON array of `StructuredOutput` objects.

Non-PDF documents are passed through as a single-element array.

Same as `extract_structured_json`.

**Returns:**

JSON-serialised `Array<StructuredOutput>` (a JSON array) on success.

**Errors:**

Returns `Validation` when either JSON argument is
malformed.  All other failures from the underlying
`split_and_extract_sync` call are mapped onto `XbergError`
via `From<StructuredError>`.

**Signature:**

```ruby
def self.split_and_extract_json(bytes, mime, preset_spec_json, options_json)
```

**Example:**

```ruby
result = split_and_extract_json("data", "value", "value", "value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `bytes` | `String` | Yes | The bytes |
| `mime` | `String` | Yes | The mime |
| `preset_spec_json` | `String` | Yes | The preset spec json |
| `options_json` | `String` | Yes | The options json |

**Returns:** `String`

**Errors:** Raises `Error`.

---

#### render_pdf_page_to_png()

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

```ruby
def self.render_pdf_page_to_png(pdf_bytes, page_index, dpi: nil, password: nil)
```

**Example:**

```ruby
result = render_pdf_page_to_png("data", 42, dpi: 42, password: "value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `pdf_bytes` | `String` | Yes | Raw PDF file bytes |
| `page_index` | `Integer` | Yes | Zero-based page index |
| `dpi` | `Integer?` | No | Resolution in dots per inch (default: 150) |
| `password` | `String?` | No | Optional password for encrypted PDFs |

**Returns:** `String`

**Errors:** Raises `Error`.

---

#### pdf_page_count()

Count the pages in a PDF without rendering any of them.

Opens the document and returns its page count from the PDF structure. No page
is rasterized, so this is cheap relative to `render_pdf_page_to_png` — use it
when you only need the count (e.g. to drive a render loop over the pages).

**Errors:**

Returns `XbergError.Parsing` if the PDF cannot be opened, authenticated,
or its page count read.

**Signature:**

```ruby
def self.pdf_page_count(pdf_bytes, password: nil)
```

**Example:**

```ruby
result = pdf_page_count("data", password: "value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `pdf_bytes` | `String` | Yes | Raw PDF file bytes |
| `password` | `String?` | No | Optional password for encrypted PDFs |

**Returns:** `Integer`

**Errors:** Raises `Error`.

---

#### caption_image()

Caption a single image from bytes.

  `RegionKind.Caption` prompt when `nil`.

**Returns:**

The generated caption text.

**Errors:**

Returns an error if the VLM call fails or if image format detection fails.

**Signature:**

```ruby
def self.caption_image(image_bytes, llm_config, custom_prompt: nil)
```

**Example:**

```ruby
result = caption_image("data", LlmConfig.new, custom_prompt: "value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `image_bytes` | `String` | Yes | The image data. |
| `llm_config` | `LlmConfig` | Yes | LLM configuration for the VLM call. |
| `custom_prompt` | `String?` | No | Optional custom caption prompt. Uses the default |

**Returns:** `String`

**Errors:** Raises `Error`.

---

#### caption_image_file()

Caption a single image from a file path.

  `RegionKind.Caption` prompt when `nil`.

**Returns:**

The generated caption text.

**Errors:**

Returns an error if the file cannot be read, if image format detection fails,
or if the VLM call fails.

**Signature:**

```ruby
def self.caption_image_file(path, llm_config, custom_prompt: nil)
```

**Example:**

```ruby
result = caption_image_file("value", LlmConfig.new, custom_prompt: "value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `path` | `String` | Yes | Path to the image file. |
| `llm_config` | `LlmConfig` | Yes | LLM configuration for the VLM call. |
| `custom_prompt` | `String?` | No | Optional custom caption prompt. Uses the default |

**Returns:** `String`

**Errors:** Raises `Error`.

---

#### detect_mime_type()

Detect the MIME type of a file at the given path.

Uses the file extension and optionally the file content to determine the MIME type.
Set `check_exists` to `true` to verify the file exists before detection.

**Signature:**

```ruby
def self.detect_mime_type(path, check_exists)
```

**Example:**

```ruby
result = detect_mime_type("value", true)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `path` | `String` | Yes | Path to the file |
| `check_exists` | `Boolean` | Yes | The check exists |

**Returns:** `String`

**Errors:** Raises `Error`.

---

#### embed_texts_async()

**Signature:**

```ruby
def self.embed_texts_async(texts, config)
```

**Example:**

```ruby
result = embed_texts_async([], EmbeddingConfig.new)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `texts` | `Array<String>` | Yes | The  texts |
| `config` | `EmbeddingConfig` | Yes | The embedding config |

**Returns:** `Array<Array<Float>>`

**Errors:** Raises `Error`.

---

#### get_embedding_preset()

Get an embedding preset by name.

Returns `nil` if no preset with the given name exists. Returns an owned
clone so the value is safe to pass across FFI boundaries.

**Signature:**

```ruby
def self.get_embedding_preset(name)
```

**Example:**

```ruby
result = get_embedding_preset("value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `name` | `String` | Yes | The name |

**Returns:** `EmbeddingPreset?`

---

#### list_embedding_presets()

List the names of all available embedding presets.

Returns owned `String`s so the values are safe to pass across FFI boundaries.

**Signature:**

```ruby
def self.list_embedding_presets()
```

**Example:**

```ruby
result = list_embedding_presets()
```

**Returns:** `Array<String>`

---

#### get_embedding_preset()

Returns `nil` for builds without the `embedding-presets` feature.

**Signature:**

```ruby
def self.get_embedding_preset(name)
```

**Example:**

```ruby
result = get_embedding_preset("value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `name` | `String` | Yes | The  name |

**Returns:** `EmbeddingPreset?`

---

#### list_embedding_presets()

Returns an empty list for builds without the `embedding-presets` feature.

**Signature:**

```ruby
def self.list_embedding_presets()
```

**Example:**

```ruby
result = list_embedding_presets()
```

**Returns:** `Array<String>`

---

#### rerank()

Rerank a list of documents by relevance to a query.

Returns documents sorted descending by score. Applies `top_k` truncation if
configured.

**Errors:**

- `XbergError.Validation` if `query` is empty or blank.
- `XbergError.MissingDependency` if ONNX Runtime is not installed (ONNX path).
- `XbergError.Reranking` if the preset is unknown or model download fails.

Since v5.0.

**Signature:**

```ruby
def self.rerank(query, documents, config)
```

**Example:**

```ruby
result = rerank("value", [], RerankerConfig.new)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `query` | `String` | Yes | The query |
| `documents` | `Array<String>` | Yes | The documents |
| `config` | `RerankerConfig` | Yes | The configuration options |

**Returns:** `Array<RerankedDocument>`

**Errors:** Raises `Error`.

---

#### rerank()

Stub for builds without the `reranker` feature — keeps the symbol available
on no-ORT targets (Android x86_64 emulator, WASM) so language bindings compile.

Since v5.0.

**Signature:**

```ruby
def self.rerank(query, documents, config)
```

**Example:**

```ruby
result = rerank("value", [], RerankerConfig.new)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `query` | `String` | Yes | The  query |
| `documents` | `Array<String>` | Yes | The  documents |
| `config` | `RerankerConfig` | Yes | The reranker config |

**Returns:** `Array<RerankedDocument>`

**Errors:** Raises `Error`.

---

#### rerank_async()

Stub for builds without the `reranker` feature.

Since v5.0.

**Signature:**

```ruby
def self.rerank_async(query, documents, config)
```

**Example:**

```ruby
result = rerank_async("value", [], RerankerConfig.new)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `query` | `String` | Yes | The  query |
| `documents` | `Array<String>` | Yes | The  documents |
| `config` | `RerankerConfig` | Yes | The reranker config |

**Returns:** `Array<RerankedDocument>`

**Errors:** Raises `Error`.

---

#### get_reranker_preset()

Get a reranker preset by name.

Returns `nil` if no preset with the given name exists. Returns an owned
clone so the value is safe to pass across FFI boundaries.

Since v5.0.

**Signature:**

```ruby
def self.get_reranker_preset(name)
```

**Example:**

```ruby
result = get_reranker_preset("value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `name` | `String` | Yes | The name |

**Returns:** `RerankerPreset?`

---

#### list_reranker_presets()

List the names of all available reranker presets.

Returns owned `String`s so the values are safe to pass across FFI boundaries.

Since v5.0.

**Signature:**

```ruby
def self.list_reranker_presets()
```

**Example:**

```ruby
result = list_reranker_presets()
```

**Returns:** `Array<String>`

---

#### get_reranker_preset()

Returns `nil` for builds without the `reranker-presets` feature.

Since v5.0.

**Signature:**

```ruby
def self.get_reranker_preset(name)
```

**Example:**

```ruby
result = get_reranker_preset("value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `name` | `String` | Yes | The  name |

**Returns:** `RerankerPreset?`

---

#### list_reranker_presets()

Returns an empty list for builds without the `reranker-presets` feature.

Since v5.0.

**Signature:**

```ruby
def self.list_reranker_presets()
```

**Example:**

```ruby
result = list_reranker_presets()
```

**Returns:** `Array<String>`

---

#### embed_texts_async()

**Signature:**

```ruby
def self.embed_texts_async(texts, config)
```

**Example:**

```ruby
result = embed_texts_async([], EmbeddingConfig.new)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `texts` | `Array<String>` | Yes | The  texts |
| `config` | `EmbeddingConfig` | Yes | The embedding config |

**Returns:** `Array<Array<Float>>`

**Errors:** Raises `Error`.

---

### Types

#### AccelerationConfig

Hardware acceleration configuration for ONNX Runtime models.

Controls which execution provider (CPU, CoreML, CUDA, TensorRT) is used
for inference in layout detection and embedding generation.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `provider` | `ExecutionProviderType` | `:auto` | Execution provider to use for ONNX inference. |
| `device_id` | `Integer` | — | GPU device ID (for CUDA/TensorRT). Ignored for CPU/CoreML/Auto. |

---

#### ArchiveEntry

A single file extracted from an archive.

When archives (ZIP, TAR, 7Z, GZIP) are extracted with recursive extraction
enabled, each processable file produces its own full `ExtractionResult`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `path` | `String` | — | Archive-relative file path (e.g. "folder/document.pdf"). |
| `mime_type` | `String` | — | Detected MIME type of the file. |
| `result` | `ExtractionResult` | — | Full extraction result for this file. |

---

#### ArchiveMetadata

Archive (ZIP/TAR/7Z) metadata.

Extracted from compressed archive files containing file lists and size information.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `format` | `String` | — | Archive format ("ZIP", "TAR", "7Z", etc.) |
| `file_count` | `Integer` | — | Total number of files in the archive |
| `file_list` | `Array<String>` | `\[\]` | List of file paths within the archive |
| `total_size` | `Integer` | — | Total uncompressed size in bytes |
| `compressed_size` | `Integer?` | `nil` | Compressed size in bytes (if available) |

---

#### AudioMetadata

Audio/video file metadata.

Populated from container tags (ID3v2, MP4 atoms, Vorbis comments, etc.) and
PCM decode properties. Available when the `transcription-types` feature is enabled.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `duration_ms` | `Integer?` | `nil` | Duration in milliseconds derived from the decoded audio stream. |
| `codec` | `String?` | `nil` | Audio codec (e.g. "mp3", "aac", "opus", "flac"). |
| `container` | `String?` | `nil` | Container format (e.g. "mpeg", "mp4", "ogg", "wav"). |
| `sample_rate_hz` | `Integer?` | `nil` | Sample rate in Hz after decode (always 16000 when resampled for Whisper). |
| `channels` | `Integer?` | `nil` | Number of audio channels (1 = mono, 2 = stereo). |
| `bitrate` | `Integer?` | `nil` | Audio bitrate in kbps from the source file tags/properties. |

---

#### BBox

Bounding box in original image coordinates (x1, y1) top-left, (x2, y2) bottom-right.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `x1` | `Float` | — | Left edge (x-coordinate of the top-left corner). |
| `y1` | `Float` | — | Top edge (y-coordinate of the top-left corner). |
| `x2` | `Float` | — | Right edge (x-coordinate of the bottom-right corner). |
| `y2` | `Float` | — | Bottom edge (y-coordinate of the bottom-right corner). |

---

#### BibtexMetadata

BibTeX bibliography metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `entry_count` | `Integer` | — | Number of entries in the bibliography. |
| `citation_keys` | `Array<String>` | `\[\]` | BibTeX citation keys (e.g. `"knuth1984"`) for all entries. |
| `authors` | `Array<String>` | `\[\]` | Author names collected across all bibliography entries. |
| `year_range` | `YearRange?` | `nil` | Earliest and latest publication years found in the bibliography. |
| `entry_types` | `Hash{String=>Integer}?` | `{}` | Count of entries grouped by BibTeX entry type (e.g. `"article"` → 5). |

---

#### BoundingBox

Bounding box coordinates for element positioning.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `x0` | `Float` | — | Left x-coordinate |
| `y0` | `Float` | — | Bottom y-coordinate |
| `x1` | `Float` | — | Right x-coordinate |
| `y1` | `Float` | — | Top y-coordinate |

---

#### CacheStats

Aggregate statistics for a xberg cache directory.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `total_files` | `Integer` | — | Total number of files currently in the cache directory. |
| `total_size_mb` | `Float` | — | Combined size of all cache files in megabytes. |
| `available_space_mb` | `Float` | — | Free disk space available on the cache volume, in megabytes. |
| `oldest_file_age_days` | `Float` | — | Age of the oldest cache file in days (0.0 if the cache is empty). |
| `newest_file_age_days` | `Float` | — | Age of the most recently written cache file in days (0.0 if the cache is empty). |

---

#### CaptioningConfig

**Since:** `v5.0`

Configuration for the VLM captioning post-processor.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `llm` | `LlmConfig` | — | LLM configuration used for the VLM call. |
| `prompt` | `String?` | `nil` | Optional custom caption prompt. `nil` uses the default `RegionKind.Caption` prompt that ships with `crate.llm.region_extractor`. |
| `min_image_area` | `Integer` | `serde(default = "default_min_image_area")` | Skip images whose `width * height` is below this threshold (in pixels). Default `1_000` filters out icons and decorations. |

---

#### CaptioningEnrichmentConfig

Captioning enrichment knob: which LLM to use for image captions.

The enrichment stage calls `caption_image` for every
image in `ExtractionResult.images` that has non-empty `data`. Images with
empty byte data (e.g. reference-only images populated via `source_path`) are
skipped rather than forwarded to the VLM.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `config` | `LlmConfig` | — | LLM / VLM configuration forwarded verbatim to each `caption_image` call. |
| `custom_prompt` | `String?` | `nil` | Optional custom prompt override forwarded to every `caption_image` call. `nil` uses the default `RegionKind.Caption` prompt. |

---

#### CellChange

A single changed cell within a table.

Defined here (rather than only in `crate.diff`) so `RevisionDelta` can
reference it unconditionally, without requiring the `diff` Cargo feature.
`crate.diff` re-exports this type verbatim.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `row` | `Integer` | — | Zero-based row index. |
| `col` | `Integer` | — | Zero-based column index. |
| `from` | `String` | — | Value before the change. |
| `to` | `String` | — | Value after the change. |

---

#### Chunk

A text chunk with optional embedding and metadata.

Chunks are created when chunking is enabled in `ExtractionConfig`. Each chunk
contains the text content, optional embedding vector (if embedding generation
is configured), and metadata about its position in the document.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String` | — | The text content of this chunk. |
| `chunk_type` | `ChunkType` | `/* serde(default) */` | Semantic structural classification of this chunk. Assigned by the heuristic classifier based on content patterns and heading context. Defaults to `ChunkType.Unknown` when no rule matches. |
| `embedding` | `Array<Float>?` | `nil` | Optional embedding vector for this chunk. Only populated when `EmbeddingConfig` is provided in chunking configuration. The dimensionality depends on the chosen embedding model. |
| `metadata` | `ChunkMetadata` | — | Metadata about this chunk's position and properties. |

---

#### ChunkInfo

Information about a single chunk.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `index` | `Integer` | — | Zero-based chunk index. |
| `pages` | `PageRange` | — | Page range for this chunk. |
| `estimated_time_ms` | `Integer` | — | Estimated processing time for this chunk in milliseconds. |

---

#### ChunkMetadata

Metadata about a chunk's position in the original document.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `byte_start` | `Integer` | — | Byte offset where this chunk starts in the original text (UTF-8 valid boundary). |
| `byte_end` | `Integer` | — | Byte offset where this chunk ends in the original text (UTF-8 valid boundary). |
| `token_count` | `Integer?` | `nil` | Number of tokens in this chunk (if available). This is calculated by the embedding model's tokenizer if embeddings are enabled. |
| `chunk_index` | `Integer` | — | Zero-based index of this chunk in the document. |
| `total_chunks` | `Integer` | — | Total number of chunks in the document. |
| `first_page` | `Integer?` | `nil` | First page number this chunk spans (1-indexed). Only populated when page tracking is enabled in extraction configuration. |
| `last_page` | `Integer?` | `nil` | Last page number this chunk spans (1-indexed, equal to first_page for single-page chunks). Only populated when page tracking is enabled in extraction configuration. |
| `heading_context` | `HeadingContext?` | `/* serde(default) */` | Heading context when using Markdown chunker. Contains the heading hierarchy this chunk falls under. Only populated when `ChunkerType.Markdown` is used. |
| `heading_path` | `Array<String>` | `/* serde(default) */` | Flattened heading trail from document root to this chunk's section. Each element is a heading's text, outermost first. Derived from `heading_context` when present; empty otherwise. Provides a binding-friendly, RAG-shaped breadcrumb without requiring callers to walk the nested `HeadingContext` structure. |
| `image_indices` | `Array<Integer>` | `/* serde(default) */` | Indices into `ExtractionResult.images` for images on pages covered by this chunk. Contains zero-based indices into the top-level `images` collection for every image whose `page_number` falls within `\[first_page, last_page\]`. Empty when image extraction is disabled or the chunk spans no pages with images. |

---

#### ChunkPlan

Complete chunking plan for a document.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `total_chunks` | `Integer` | `0` | Total number of chunks. |
| `chunks` | `Array<ChunkInfo>` | `\[\]` | Individual chunk information. |
| `total_estimated_time_ms` | `Integer` | `0` | Estimated total processing time in milliseconds. |
| `use_disk_processing` | `Boolean` | `false` | Whether to use disk-based processing for large files. |
| `reason` | `ChunkingReason` | `:large_file` | Reason for chunking. |

##### Methods

###### default()

An empty plan (no chunks). The `reason` is a placeholder since an empty plan
has no chunking rationale; callers always overwrite it when a real plan is built.

**Signature:**

```ruby
def self.default()
```

**Example:**

```ruby
result = ChunkPlan.default()
```

**Returns:** `ChunkPlan`

###### total_pages()

Get the total number of pages across all chunks.

**Signature:**

```ruby
def total_pages()
```

**Example:**

```ruby
result = instance.total_pages()
```

**Returns:** `Integer`

---

#### ChunkingConfig

Chunking configuration.

Configures text chunking for document content, including chunk size,
overlap, trimming behavior, and optional embeddings.

Use `..the default constructor` when constructing to allow for future field additions:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_characters` | `Integer` | `1000` | Maximum size per chunk (in units determined by `sizing`). When `sizing` is `Characters` (default), this is the max character count. When using token-based sizing, this is the max token count. Default: 1000 |
| `overlap` | `Integer` | `200` | Overlap between chunks (in units determined by `sizing`). Default: 200 |
| `trim` | `Boolean` | `true` | Whether to trim whitespace from chunk boundaries. Default: true |
| `chunker_type` | `ChunkerType` | `:text` | Type of chunker to use (Text or Markdown). Default: Text |
| `embedding` | `EmbeddingConfig?` | `nil` | Optional embedding configuration for chunk embeddings. |
| `preset` | `String?` | `nil` | Use a preset configuration (overrides individual settings if provided). |
| `sizing` | `ChunkSizing` | `:characters` | How to measure chunk size. Default: `Characters` (Unicode character count). Enable `chunking-tiktoken` or `chunking-tokenizers` features for token-based sizing. |
| `prepend_heading_context` | `Boolean` | `false` | When `true` and `chunker_type` is `Markdown`, prepend the heading hierarchy path (e.g. `"# Title > ## Section\n\n"`) to each chunk's content string. This is useful for RAG pipelines where each chunk needs self-contained context about its position in the document structure. Default: `false` |
| `topic_threshold` | `Float?` | `nil` | Optional cosine similarity threshold for semantic topic boundary detection. Only used when `chunker_type` is `Semantic` and an `EmbeddingConfig` is provided. You almost never need to set this. When omitted, defaults to `0.75` which works well for most documents. Lower values detect more topic boundaries (more, smaller chunks); higher values detect fewer. Range: `0.0..=1.0`. |
| `table_chunking` | `TableChunkingMode` | `:split` | How to handle markdown tables that exceed the chunk size limit. Only applies when `chunker_type` is `Markdown`. - `Split` (default) — tables are split at row boundaries; continuation chunks do not repeat the header. - `RepeatHeader` — the table header row and separator are prepended to every continuation chunk so each chunk is self-contained. Default: `Split` |

##### Methods

###### default()

**Signature:**

```ruby
def self.default()
```

**Example:**

```ruby
result = ChunkingConfig.default()
```

**Returns:** `ChunkingConfig`

---

#### ChunkingResult

Result of a text chunking operation.

Contains the generated chunks and metadata about the chunking.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `chunks` | `Array<Chunk>` | — | List of text chunks |
| `chunk_count` | `Integer` | — | Total number of chunks generated |

---

#### Citation

A structured citation from a citation block.

Parsed from entries like:
`[^srcN]: source, locator, excerpt: "text"`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `label` | `String` | — | The label of the citation (e.g., "src1" in `\[^src1\]: ...`). |
| `source` | `String` | — | The source reference (path, URL, or identifier). |
| `locator` | `String?` | `nil` | Optional locator within the source (e.g., "page 3" or "section 2.1"). |
| `excerpt` | `String?` | `nil` | Optional excerpt — quoted text from the source. |

---

#### CitationMetadata

Citation file metadata (RIS, PubMed, EndNote).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `citation_count` | `Integer` | — | Total number of citation records in the file. |
| `format` | `String?` | `nil` | Detected citation file format (e.g. `"ris"`, `"pubmed"`, `"endnote"`). |
| `authors` | `Array<String>` | `\[\]` | Author names collected across all citation records. |
| `year_range` | `YearRange?` | `nil` | Earliest and latest publication years found in the file. |
| `dois` | `Array<String>` | `\[\]` | DOI identifiers found in the citation records. |
| `keywords` | `Array<String>` | `\[\]` | Keywords collected from all citation records. |

---

#### ClassificationEnrichmentConfig

Classification enrichment knob: how to label the document.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `config` | `PageClassificationConfig` | — | Label set and LLM settings for the classification stage. |

---

#### ClassificationLabel

A single label + confidence pair.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `label` | `String` | — | Label name as configured in `PageClassificationConfig.labels`. |
| `confidence` | `Float?` | `nil` | Backend-reported confidence in `\[0.0, 1.0\]`. `nil` when the backend (e.g. an LLM prompt without explicit confidence schema) did not report one. |

---

#### ConfidenceSignals

Input signals for confidence scoring.

Caller fills these from the extraction result and the LLM response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text_coverage` | `Float` | — | Fraction of pages with usable text in `\[0, 1\]`. |
| `ocr_aggregate` | `Float?` | `nil` | Mean OCR per-element recognition confidence; `nil` when OCR did not run. |
| `schema_compliance` | `SchemaCompliance` | — | Schema-validation result of the merged output. |

##### Methods

###### from_extraction_result()

Build `ConfidenceSignals` from an `ExtractionResult`.

- `result` — The extraction result whose `ocr_elements` are inspected.
- `schema_compliance` — Caller-supplied schema validation outcome.
- `text_coverage` — Caller-supplied fraction of pages with usable text
  (e.g. 1.0 for native text formats, value from PDF analysis for PDFs).

The `ocr_aggregate` is computed as the arithmetic mean of all
`ocr_elements[].confidence.recognition` values.  When `ocr_elements` is
`nil` or empty the field is set to `nil`.

**Signature:**

```ruby
def self.from_extraction_result(result, schema_compliance, text_coverage)
```

**Example:**

```ruby
result = ConfidenceSignals.from_extraction_result(ExtractionResult.new, SchemaCompliance.new, 0.5)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `result` | `ExtractionResult` | Yes | The extraction result |
| `schema_compliance` | `SchemaCompliance` | Yes | The schema compliance |
| `text_coverage` | `Float` | Yes | The text coverage |

**Returns:** `ConfidenceSignals`

---

#### ConfidenceWeights

Tunable weights for the confidence scoring formula.

Defaults picked by inspection; callers tune them via config.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text_coverage` | `Float` | `0.3` | Weight assigned to `text_coverage`. Default 0.30. |
| `ocr_aggregate` | `Float` | `0.3` | Weight assigned to `ocr_aggregate` when OCR ran. Default 0.30 — folds into `text_coverage` weight when OCR did not run. |
| `schema_compliance` | `Float` | `0.4` | Weight assigned to `schema_compliance`. Default 0.40. |

##### Methods

###### default()

**Signature:**

```ruby
def self.default()
```

**Example:**

```ruby
result = ConfidenceWeights.default()
```

**Returns:** `ConfidenceWeights`

###### is_normalized()

Validate that weights sum to approximately 1.0.

**Signature:**

```ruby
def is_normalized()
```

**Example:**

```ruby
result = instance.is_normalized()
```

**Returns:** `Boolean`

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
| `include_headers` | `Boolean` | `false` | Include running headers in extraction output. - PDF: Disables top-margin furniture stripping and prevents the layout model from treating `PageHeader`-classified regions as furniture. - DOCX: Includes document headers in text output. - RTF/ODT: Headers already included; this is a no-op when true. - HTML/EPUB: Keeps `<header>` element content. Default: `false` (headers are stripped or excluded). |
| `include_footers` | `Boolean` | `false` | Include running footers in extraction output. - PDF: Disables bottom-margin furniture stripping and prevents the layout model from treating `PageFooter`-classified regions as furniture. - DOCX: Includes document footers in text output. - RTF/ODT: Footers already included; this is a no-op when true. - HTML/EPUB: Keeps `<footer>` element content. Default: `false` (footers are stripped or excluded). |
| `strip_repeating_text` | `Boolean` | `true` | Enable the heuristic cross-page repeating text detector. When `true` (default), text that repeats verbatim across a supermajority of pages is classified as furniture and stripped.  Disable this if brand names or repeated headings are being incorrectly removed by the heuristic. Note: when a layout-detection model is active, the model may independently classify page-header / page-footer regions as furniture on a per-page basis. To preserve those regions, set `include_headers = true`, `include_footers = true`, or both, in addition to disabling this flag. Primarily affects PDF extraction. Default: `true`. |
| `include_watermarks` | `Boolean` | `false` | Include watermark text in extraction output. - PDF: Keeps watermark artifacts and arXiv identifiers. - Other formats: No effect currently. Default: `false` (watermarks are stripped). |

##### Methods

###### default()

**Signature:**

```ruby
def self.default()
```

**Example:**

```ruby
result = ContentFilterConfig.default()
```

**Returns:** `ContentFilterConfig`

---

#### ContributorRole

JATS contributor with role.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String` | — | Contributor display name. |
| `role` | `String?` | `nil` | Contributor role (e.g. `"author"`, `"editor"`). |

---

#### CoreProperties

Dublin Core metadata from docProps/core.xml

Contains standard metadata fields defined by the Dublin Core standard
and Office-specific extensions.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `title` | `String?` | `nil` | Document title |
| `subject` | `String?` | `nil` | Document subject/topic |
| `creator` | `String?` | `nil` | Document creator/author |
| `keywords` | `String?` | `nil` | Keywords or tags |
| `description` | `String?` | `nil` | Document description/abstract |
| `last_modified_by` | `String?` | `nil` | User who last modified the document |
| `revision` | `String?` | `nil` | Revision number |
| `created` | `String?` | `nil` | Creation timestamp (ISO 8601) |
| `modified` | `String?` | `nil` | Last modification timestamp (ISO 8601) |
| `category` | `String?` | `nil` | Document category |
| `content_status` | `String?` | `nil` | Content status (Draft, Final, etc.) |
| `language` | `String?` | `nil` | Document language |
| `identifier` | `String?` | `nil` | Unique identifier |
| `version` | `String?` | `nil` | Document version |
| `last_printed` | `String?` | `nil` | Last print timestamp (ISO 8601) |

---

#### CsvMetadata

CSV/TSV file metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `row_count` | `Integer` | — | Total number of data rows (excluding the header row if present). |
| `column_count` | `Integer` | — | Number of columns detected. |
| `delimiter` | `String?` | `nil` | Field delimiter character (e.g. `","` or `"\t"`). |
| `has_header` | `Boolean` | — | Whether the first row was treated as a header. |
| `column_types` | `Array<String>?` | `\[\]` | Inferred data type for each column (e.g. `"string"`, `"integer"`, `"float"`). |

---

#### DbfFieldInfo

dBASE field information.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String` | — | Field (column) name. |
| `field_type` | `String` | — | dBASE field type character (e.g. `"C"` for character, `"N"` for numeric). |

---

#### DbfMetadata

dBASE (DBF) file metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `record_count` | `Integer` | — | Total number of data records in the DBF file. |
| `field_count` | `Integer` | — | Number of field (column) definitions. |
| `fields` | `Array<DbfFieldInfo>` | `\[\]` | Descriptor for each field in the table schema. |

---

#### DetectResponse

MIME type detection response.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `mime_type` | `String` | — | Detected MIME type |
| `filename` | `String?` | `nil` | Original filename (if provided) |

---

#### DetectionResult

Page-level detection result containing all detections and page metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `page_width` | `Integer` | — | Page width in pixels (as seen by the model). |
| `page_height` | `Integer` | — | Page height in pixels (as seen by the model). |
| `detections` | `Array<LayoutDetection>` | — | All layout detections on this page after postprocessing. |

---

#### DiffHunk

A single contiguous hunk in a unified diff.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `from_line` | `Integer` | — | Starting line number in the old content (0-indexed). |
| `from_count` | `Integer` | — | Number of lines from the old content in this hunk. |
| `to_line` | `Integer` | — | Starting line number in the new content (0-indexed). |
| `to_count` | `Integer` | — | Number of lines from the new content in this hunk. |
| `lines` | `Array<DiffLine>` | — | Lines that make up this hunk. |

---

#### DiffOptions

Options controlling how two `ExtractionResult` values are compared.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `include_metadata` | `Boolean` | `true` | Include metadata changes in the diff. Default: `true`. |
| `include_embedded` | `Boolean` | `true` | Include embedded-children changes in the diff. Default: `true`. |
| `max_content_chars` | `Integer?` | `nil` | Truncate content to this many characters before diffing. Useful for very large documents where only the first N characters matter. `nil` means no truncation. |

##### Methods

###### default()

**Signature:**

```ruby
def self.default()
```

**Example:**

```ruby
result = DiffOptions.default()
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
| `plain_text` | `String` | — | Plain text representation for backwards compatibility |
| `blocks` | `Array<FormattedBlock>` | — | Structured block-level content |
| `metadata` | `Metadata` | — | Metadata from YAML frontmatter |
| `tables` | `Array<Table>` | — | Extracted tables as structured data |
| `images` | `Array<DjotImage>` | — | Extracted images with metadata |
| `links` | `Array<DjotLink>` | — | Extracted links with URLs |
| `footnotes` | `Array<Footnote>` | — | Footnote definitions |

---

#### DjotImage

Image element in Djot.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `src` | `String` | — | Image source URL or path |
| `alt` | `String` | — | Alternative text |
| `title` | `String?` | `nil` | Optional title |

---

#### DjotLink

Link element in Djot.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | `String` | — | Link URL |
| `text` | `String` | — | Link text content |
| `title` | `String?` | `nil` | Optional title |

---

#### DocumentBoundary

Detected document boundary within a PDF.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `start_page` | `Integer` | — | 1-indexed start page (inclusive). |
| `end_page` | `Integer` | — | 1-indexed end page (inclusive). |
| `confidence` | `Float` | — | Confidence in this boundary, `\[0.0, 1.0\]`. |
| `reason` | `BoundaryReason` | — | Reason for the boundary detection. |

---

#### DocumentMetadata

Metadata about a document for analysis.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `mime_type` | `String` | — | MIME type of the document. |
| `size_bytes` | `Integer` | — | File size in bytes. |
| `page_count` | `Integer?` | `nil` | Page count (if known, e.g., from previous analysis). |
| `force_ocr` | `Boolean` | — | Whether OCR is forced regardless of text layer. |
| `user_chunk_config` | `UserChunkConfig?` | `nil` | User-provided chunk configuration overrides. |
| `chunking_enabled` | `Boolean` | — | Whether chunking is enabled for this job. |

---

#### DocumentNode

A single node in the document tree.

Each node has deterministic `id`, typed `content`, optional `parent`/`children`
for tree structure, and metadata like page number, bounding box, and content layer.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `NodeContent` | — | Node content — tagged enum, type-specific data only. |
| `parent` | `Integer?` | `nil` | Parent node index (`nil` = root-level node). |
| `children` | `Array<Integer>` | `/* serde(default) */` | Child node indices in reading order. |
| `content_layer` | `ContentLayer` | `/* serde(default) */` | Content layer classification. Always serialised — Kotlin-Android (and any other typed binding) treats the field as non-nullable, so omitting it from the JSON wire would break consumer deserialisation.  `#\[serde(default)\]` covers the missing-field case on inbound JSON. |
| `page` | `Integer?` | `nil` | Page number where this node starts (1-indexed). |
| `page_end` | `Integer?` | `nil` | Page number where this node ends (for multi-page tables/sections). |
| `bbox` | `BoundingBox?` | `nil` | Bounding box in document coordinates. |
| `annotations` | `Array<TextAnnotation>` | `/* serde(default) */` | Inline annotations (formatting, links) on this node's text content. Only meaningful for text-carrying nodes; empty for containers. |
| `attributes` | `Hash{String=>String}?` | `nil` | Format-specific key-value attributes. Extensible bag for miscellaneous data without a dedicated typed field: CSS classes, LaTeX environment names, Excel cell formulas, slide layout names, etc. |

---

#### DocumentRelationship

A resolved relationship between two nodes in the document tree.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `source` | `Integer` | — | Source node index (the referencing node). |
| `target` | `Integer` | — | Target node index (the referenced node). |
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
| `revision_id` | `String` | — | Format-specific revision identifier. For DOCX this is the `w:id` attribute value on the change element (e.g. `"42"`). When the attribute is absent a synthetic fallback is generated (`"docx-ins-0"`, `"docx-del-3"`, …). |
| `author` | `String?` | `nil` | Display name of the author who made this change, when available. |
| `timestamp` | `String?` | `nil` | ISO-8601 timestamp of the change, when available. Stored as a plain string so this type remains FFI-friendly and unconditionally available without the `chrono` optional dep. DOCX populates this from the `w:date` attribute (e.g. `"2024-03-15T10:30:00Z"`). |
| `kind` | `RevisionKind` | — | Semantic kind of this revision. |
| `anchor` | `RevisionAnchor?` | `nil` | Best-effort document location for this revision. Resolution is format-dependent and may be `nil` when the location cannot be determined (e.g. changes inside table cells before table-cell anchor support is added). |
| `delta` | `RevisionDelta` | — | The content changes that make up this revision. |

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
| `nodes` | `Array<DocumentNode>` | `\[\]` | All nodes in document/reading order. |
| `source_format` | `String?` | `nil` | Origin format identifier (e.g. "docx", "pptx", "html", "pdf"). Allows renderers to apply format-aware heuristics when converting the document tree to output formats. |
| `relationships` | `Array<DocumentRelationship>` | `\[\]` | Resolved relationships between nodes (footnote refs, citations, anchor links, etc.). Populated during derivation from the internal document representation. Empty when no relationships are detected. |
| `node_types` | `Array<String>` | `\[\]` | Sorted, deduplicated list of node type names present in this document. Each value is the snake_case `node_type` tag of the corresponding `NodeContent` variant (e.g. `"paragraph"`, `"heading"`, `"table"`, …). Computed from `nodes` via `DocumentStructure.finalize_node_types`. Empty until that method is called (internal construction paths call it at the end of derivation). |

##### Methods

###### finalize_node_types()

Compute and populate the `node_types` field from the current `nodes`.

Call this after all nodes have been added to the structure. Internal
construction paths (builder, derivation) call this automatically.

**Signature:**

```ruby
def finalize_node_types()
```

**Example:**

```ruby
instance.finalize_node_types()
```

**Returns:** No return value.

###### is_empty()

Check if the document structure is empty.

**Signature:**

```ruby
def is_empty()
```

**Example:**

```ruby
result = instance.is_empty()
```

**Returns:** `Boolean`

###### default()

**Signature:**

```ruby
def self.default()
```

**Example:**

```ruby
result = DocumentStructure.default()
```

**Returns:** `DocumentStructure`

---

#### DocumentSummary

Summary of an extracted document.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text` | `String` | — | Summary text (plain prose). |
| `strategy` | `SummaryStrategy` | — | Strategy that produced this summary. |
| `token_count` | `Integer?` | `nil` | Approximate token count of the summary, when known. |

---

#### DocxAppProperties

Application properties from docProps/app.xml for DOCX

Contains Word-specific document statistics and metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `application` | `String?` | `nil` | Application name (e.g., "Microsoft Office Word") |
| `app_version` | `String?` | `nil` | Application version |
| `template` | `String?` | `nil` | Template filename |
| `total_time` | `Integer?` | `nil` | Total editing time in minutes |
| `pages` | `Integer?` | `nil` | Number of pages |
| `words` | `Integer?` | `nil` | Number of words |
| `characters` | `Integer?` | `nil` | Number of characters (excluding spaces) |
| `characters_with_spaces` | `Integer?` | `nil` | Number of characters (including spaces) |
| `lines` | `Integer?` | `nil` | Number of lines |
| `paragraphs` | `Integer?` | `nil` | Number of paragraphs |
| `company` | `String?` | `nil` | Company name |
| `doc_security` | `Integer?` | `nil` | Document security level |
| `scale_crop` | `Boolean?` | `nil` | Scale crop flag |
| `links_up_to_date` | `Boolean?` | `nil` | Links up to date flag |
| `shared_doc` | `Boolean?` | `nil` | Shared document flag |
| `hyperlinks_changed` | `Boolean?` | `nil` | Hyperlinks changed flag |

---

#### DocxMetadata

Word document metadata.

Extracted from DOCX files using shared Office Open XML metadata extraction.
Integrates with `office_metadata` module for core/app/custom properties.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `core_properties` | `CoreProperties?` | `nil` | Core properties from docProps/core.xml (Dublin Core metadata) Contains title, creator, subject, keywords, dates, etc. Shared format across DOCX/PPTX/XLSX documents. |
| `app_properties` | `DocxAppProperties?` | `nil` | Application properties from docProps/app.xml (Word-specific statistics) Contains word count, page count, paragraph count, editing time, etc. DOCX-specific variant of Office application properties. |
| `custom_properties` | `Hash{String=>Object}?` | `{}` | Custom properties from docProps/custom.xml (user-defined properties) Contains key-value pairs defined by users or applications. Values can be strings, numbers, booleans, or dates. |

---

#### Element

Semantic element extracted from document.

Represents a logical unit of content with semantic classification,
unique identifier, and metadata for tracking origin and position.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `element_type` | `ElementType` | — | Semantic type of this element |
| `text` | `String` | — | Text content of the element |
| `metadata` | `ElementMetadata` | — | Metadata about the element |

---

#### ElementMetadata

Metadata for a semantic element.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `page_number` | `Integer?` | `nil` | Page number (1-indexed) |
| `filename` | `String?` | `nil` | Source filename or document name |
| `coordinates` | `BoundingBox?` | `nil` | Bounding box coordinates if available |
| `element_index` | `Integer?` | `nil` | Position index in the element sequence |
| `additional` | `Hash{String=>String}` | — | Additional custom metadata |

---

#### EmailAttachment

Email attachment representation.

Contains metadata and optionally the content of an email attachment.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String?` | `nil` | Attachment name (from Content-Disposition header) |
| `filename` | `String?` | `nil` | Filename of the attachment |
| `mime_type` | `String?` | `nil` | MIME type of the attachment |
| `size` | `Integer?` | `nil` | Size in bytes |
| `is_image` | `Boolean` | — | Whether this attachment is an image |
| `data` | `String?` | `nil` | Attachment data (if extracted). Uses `bytes.Bytes` for cheap cloning of large buffers. |

---

#### EmailConfig

Configuration for email extraction.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `msg_fallback_codepage` | `Integer?` | `nil` | Windows codepage number to use when an MSG file contains no codepage property. Defaults to `nil`, which falls back to windows-1252. If an unrecognized or invalid codepage number is supplied (including 0), the behavior silently falls back to windows-1252 — the same as when the MSG file itself contains an unrecognized codepage. No error or warning is emitted. Users should verify output when supplying unusual values. Common values: - 1250: Central European (Polish, Czech, Hungarian, etc.) - 1251: Cyrillic (Russian, Ukrainian, Bulgarian, etc.) - 1252: Western European (default) - 1253: Greek - 1254: Turkish - 1255: Hebrew - 1256: Arabic - 932:  Japanese (Shift-JIS) - 936:  Simplified Chinese (GBK) |

---

#### EmailExtractionResult

Email extraction result.

Complete representation of an extracted email message (.eml or .msg)
including headers, body content, and attachments.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `subject` | `String?` | `nil` | Email subject line |
| `from_email` | `String?` | `nil` | Sender email address |
| `to_emails` | `Array<String>` | — | Primary recipient email addresses |
| `cc_emails` | `Array<String>` | — | CC recipient email addresses |
| `bcc_emails` | `Array<String>` | — | BCC recipient email addresses |
| `date` | `String?` | `nil` | Email date/timestamp |
| `message_id` | `String?` | `nil` | Message-ID header value |
| `plain_text` | `String?` | `nil` | Plain text version of the email body |
| `html_content` | `String?` | `nil` | HTML version of the email body |
| `content` | `String` | — | Cleaned/processed text content. Aliased as `cleaned_text` for back-compat. |
| `attachments` | `Array<EmailAttachment>` | — | List of email attachments |
| `metadata` | `Hash{String=>String}` | — | Additional email headers and metadata |

---

#### EmailMetadata

Email metadata extracted from .eml and .msg files.

Includes sender/recipient information, message ID, and attachment list.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `from_email` | `String?` | `nil` | Sender's email address |
| `from_name` | `String?` | `nil` | Sender's display name |
| `to_emails` | `Array<String>` | `\[\]` | Primary recipients |
| `cc_emails` | `Array<String>` | `\[\]` | CC recipients |
| `bcc_emails` | `Array<String>` | `\[\]` | BCC recipients |
| `message_id` | `String?` | `nil` | Message-ID header value |
| `attachments` | `Array<String>` | `\[\]` | List of attachment filenames |

---

#### EmbeddedChanges

Changes to embedded archive children between two results.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `added` | `Array<ArchiveEntry>` | `\[\]` | Children present in `b` but not in `a` (matched by `path`). |
| `removed` | `Array<ArchiveEntry>` | `\[\]` | Children present in `a` but not in `b` (matched by `path`). |
| `changed` | `Array<EmbeddedDiff>` | `\[\]` | Children present in both but with differing content (matched by `path`). Each entry holds the diff of the nested `ExtractionResult`. |

---

#### EmbeddedDiff

Diff for a single embedded archive entry that appears in both results.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `path` | `String` | — | Archive-relative path identifying this entry. |
| `diff` | `ExtractionDiff` | — | The recursive diff of the entry's extraction result. |

---

#### EmbeddedFile

Embedded file descriptor extracted from the PDF name tree.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String` | — | The filename as stored in the PDF name tree. |
| `data` | `String` | — | Raw file bytes from the embedded stream (already decompressed by lopdf). |
| `compressed_size` | `Integer` | — | Compressed byte count of the original stream (before decompression). Used by callers to compute the decompression ratio and detect zip-bomb-style attacks that embed a tiny compressed stream expanding to gigabytes of data. |
| `mime_type` | `String?` | `nil` | MIME type if specified in the filespec, otherwise `nil`. |

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

###### dimensions()

Embedding vector dimension. Must be `> 0` and must match the length of
every vector returned by `embed`.

**Signature:**

```ruby
def dimensions()
```

**Example:**

```ruby
result = instance.dimensions()
```

**Returns:** `Integer`

###### embed()

Embed a batch of texts, returning one vector per input in order.

**Errors:**

Implementations should return `Plugin` for
backend-specific failures. The dispatcher layers its own validation
(length, per-vector dimension) on top.

**Signature:**

```ruby
def embed(texts)
```

**Example:**

```ruby
result = instance.embed([])
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `texts` | `Array<String>` | Yes | The texts |

**Returns:** `Array<Array<Float>>`

**Errors:** Raises `Error`.

---

#### EmbeddingConfig

Embedding configuration for text chunks.

Configures embedding generation using ONNX models via the vendored embedding engine.
Requires the `embeddings` feature to be enabled.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `EmbeddingModelType` | `:preset` | The embedding model to use (defaults to "balanced" preset if not specified) |
| `normalize` | `Boolean` | `true` | Whether to normalize embedding vectors (recommended for cosine similarity) |
| `batch_size` | `Integer` | `32` | Batch size for embedding generation |
| `show_download_progress` | `Boolean` | `false` | Show model download progress |
| `cache_dir` | `String?` | `nil` | Custom cache directory for model files Defaults to `~/.cache/xberg/embeddings/` if not specified. Allows full customization of model download location. |
| `acceleration` | `AccelerationConfig?` | `nil` | Hardware acceleration for the embedding ONNX model. When set, controls which execution provider (CPU, CUDA, CoreML, TensorRT) is used for inference. Defaults to `nil` (auto-select per platform). |
| `max_embed_duration_secs` | `Integer?` | `nil` | Maximum wall-clock duration (in seconds) for a single `embed()` call when using `EmbeddingModelType.Plugin`. Applies only to the in-process plugin path — protects against hung host-language backends (e.g. a Python callback deadlocked on the GIL, a model stuck on CUDA OOM retries, etc.). On timeout, the dispatcher returns `Plugin` instead of blocking forever. `nil` disables the timeout. The default (60 seconds) is conservative for common in-process inference; increase for large batches on slow hardware. |

##### Methods

###### default()

**Signature:**

```ruby
def self.default()
```

**Example:**

```ruby
result = EmbeddingConfig.default()
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
| `name` | `String` | — | Short identifier for this preset (e.g. `"balanced"`, `"fast"`, `"quality"`). |
| `chunk_size` | `Integer` | — | Target chunk size in characters. |
| `overlap` | `Integer` | — | Overlap between consecutive chunks in characters. |
| `model_repo` | `String` | — | HuggingFace repository name for the model. |
| `pooling` | `String` | — | Pooling strategy: "cls" or "mean". |
| `model_file` | `String` | — | Path to the ONNX model file within the repo. |
| `dimensions` | `Integer` | — | Embedding vector dimension produced by this model. |
| `description` | `String` | — | Human-readable description of the preset's intended use case. |

---

#### EnrichOptions

Which enrichment passes to run on a piece of text.

All fields default to `false` / empty so callers can opt in precisely.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `keywords` | `Boolean` | — | Run keyword extraction on the input text. When `true`, the enrichment backend identifies the most salient terms and returns them in `EnrichResult.keywords`. |
| `entities` | `Boolean` | — | Run named-entity recognition (NER) on the input text. When `true`, the enrichment backend identifies named entities (persons, organisations, locations, etc.) and returns them in `EnrichResult.entities`. |
| `labels` | `Array<String>` | `\[\]` | Custom labels to pass through to the result without modification. These are caller-supplied tags that the enrichment pipeline propagates verbatim into `EnrichResult.labels`. Useful for attaching project- or document-level metadata to every enrichment result. |

---

#### EnrichResult

Structured output produced by a completed enrichment pass.

Fields are populated only when the corresponding `EnrichOptions` flag was set.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `keywords` | `Array<String>` | `\[\]` | Salient terms extracted from the text. Populated when `EnrichOptions.keywords` was `true`. The ordering is backend-defined (typically by descending relevance score). |
| `entities` | `Array<Entity>` | `\[\]` | Named entities found in the text. Populated when `EnrichOptions.entities` was `true`. Uses the shared OSS entity schema (`Entity` / `EntityCategory`) so consumers can pattern-match on entity categories without JSON gymnastics. |
| `labels` | `Array<String>` | `\[\]` | Caller-supplied labels echoed from `EnrichOptions.labels`. |

---

#### Entity

A single named entity detected in the extracted text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `category` | `EntityCategory` | — | Canonical category the entity belongs to (PERSON, ORG, LOCATION, etc.). |
| `text` | `String` | — | Raw mention text exactly as it appeared in the source. |
| `start` | `Integer` | — | Byte-offset span in `ExtractionResult.content` where the mention starts. |
| `end` | `Integer` | — | Byte-offset span in `ExtractionResult.content` where the mention ends (exclusive). |
| `confidence` | `Float?` | `nil` | Backend-reported confidence in `\[0.0, 1.0\]`. `nil` when the backend does not expose confidence scores. |

---

#### EpubMetadata

EPUB metadata (Dublin Core extensions).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `coverage` | `String?` | `nil` | Dublin Core `coverage` field (geographic or temporal scope). |
| `dc_format` | `String?` | `nil` | Dublin Core `format` field (media type of the resource). |
| `relation` | `String?` | `nil` | Dublin Core `relation` field (related resource identifier). |
| `source` | `String?` | `nil` | Dublin Core `source` field (origin resource identifier). |
| `dc_type` | `String?` | `nil` | Dublin Core `type` field (nature or genre of the resource). |
| `cover_image` | `String?` | `nil` | Path or identifier of the cover image within the EPUB container. |

---

#### ErrorMetadata

Error metadata (for batch operations).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `error_type` | `String` | — | Machine-readable error type identifier (e.g. "UnsupportedFormat"). |
| `message` | `String` | — | Human-readable error description. |

---

#### ExcelMetadata

Excel/spreadsheet format metadata.

Identifies the document as a spreadsheet source via the `FormatMetadata.Excel`
discriminant. Sheet count and sheet names are stored inside this struct.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `sheet_count` | `Integer?` | `nil` | Number of sheets in the workbook. |
| `sheet_names` | `Array<String>?` | `\[\]` | Names of all sheets in the workbook. |

---

#### ExcelSheet

Single Excel worksheet.

Represents one sheet from an Excel workbook with its content
converted to Markdown format and dimensional statistics.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String` | — | Sheet name as it appears in Excel |
| `markdown` | `String` | — | Sheet content converted to Markdown tables |
| `row_count` | `Integer` | — | Number of rows |
| `col_count` | `Integer` | — | Number of columns |
| `cell_count` | `Integer` | — | Total number of non-empty cells |
| `table_cells` | `Array<Array<String>>?` | `nil` | Pre-extracted table cells (2D vector of cell values) Populated during markdown generation to avoid re-parsing markdown. None for empty sheets. |

---

#### ExcelWorkbook

Excel workbook representation.

Contains all sheets from an Excel file (.xlsx, .xls, etc.) with
extracted content and metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `sheets` | `Array<ExcelSheet>` | — | All sheets in the workbook |
| `metadata` | `Hash{String=>String}` | — | Workbook-level metadata (author, creation date, etc.) |
| `revisions` | `Array<DocumentRevision>?` | `/* serde(default) */` | Collaborative-edit revision headers from `xl/revisions/revisionHeaders.xml`. Populated for legacy shared-workbook `.xlsx` files that contain the `xl/revisions/` directory. Each `<header>` element maps to one `DocumentRevision { kind: FormatChange }` carrying the header's `guid` (→ `revision_id`), `userName` (→ `author`), and `dateTime` (→ `timestamp`). `anchor` and `delta` are `nil`/empty for v1 (per-cell log parsing is a follow-up). `nil` when `xl/revisions/revisionHeaders.xml` is absent. |

---

#### ExtractInput

Unified extraction input for all public extraction entry points.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `kind` | `ExtractInputKind` | `:uri` | Source kind. `bytes` requires `bytes`; `uri` requires `uri`. |
| `bytes` | `String?` | `nil` | Raw bytes for `kind = "bytes"`. |
| `uri` | `String?` | `nil` | Local path, `file://` URI, or HTTP(S) URL for `kind = "uri"`. |
| `mime_type` | `String?` | `nil` | MIME type hint. |
| `filename` | `String?` | `nil` | Filename hint used for MIME detection and metadata. |
| `config` | `FileExtractionConfig?` | `nil` | Per-input extraction overrides. |

##### Methods

###### default()

**Signature:**

```ruby
def self.default()
```

**Example:**

```ruby
result = ExtractInput.default()
```

**Returns:** `ExtractInput`

###### bytes()

Build a bytes input with a MIME type and optional filename hint.

**Signature:**

```ruby
def self.bytes(bytes, mime_type, filename)
```

**Example:**

```ruby
result = ExtractInput.bytes("data", "value", filename: "value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `bytes` | `String` | Yes | The bytes |
| `mime_type` | `String` | Yes | The mime type |
| `filename` | `String?` | No | The filename |

**Returns:** `ExtractInput`

###### uri()

Build a URI input from a local path, `file://` URI, or HTTP(S) URL.

**Signature:**

```ruby
def self.uri(uri)
```

**Example:**

```ruby
result = ExtractInput.uri("value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `uri` | `String` | Yes | The uri |

**Returns:** `ExtractInput`

---

#### ExtractedImage

Extracted image from a document.

Contains raw image data, metadata, and optional nested OCR results.
Raw bytes allow cross-language compatibility - users can convert to
PIL.Image (Python), Sharp (Node.js), or other formats as needed.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `data` | `String` | — | Raw image data (PNG, JPEG, WebP, etc. bytes). Uses `bytes.Bytes` for cheap cloning of large buffers. |
| `format` | `String` | — | Image format (e.g., "jpeg", "png", "webp") Uses Cow<'static, str> to avoid allocation for static literals. |
| `image_index` | `Integer` | — | Zero-indexed position of this image in the document/page |
| `page_number` | `Integer?` | `nil` | Page/slide number where image was found (1-indexed) |
| `width` | `Integer?` | `nil` | Image width in pixels |
| `height` | `Integer?` | `nil` | Image height in pixels |
| `colorspace` | `String?` | `nil` | Colorspace information (e.g., "RGB", "CMYK", "Gray") |
| `bits_per_component` | `Integer?` | `nil` | Bits per color component (e.g., 8, 16) |
| `is_mask` | `Boolean` | — | Whether this image is a mask image |
| `description` | `String?` | `nil` | Optional description of the image |
| `ocr_result` | `ExtractionResult?` | `nil` | Nested OCR extraction result (if image was OCRed) When OCR is performed on this image, the result is embedded here rather than in a separate collection, making the relationship explicit. |
| `bounding_box` | `BoundingBox?` | `nil` | Bounding box of the image on the page (PDF coordinates: x0=left, y0=bottom, x1=right, y1=top). Only populated for PDF-extracted images when position data is available from the PDF extractor. |
| `source_path` | `String?` | `nil` | Original source path of the image within the document archive (e.g., "media/image1.png" in DOCX). Used for rendering image references when the binary data is not extracted. |
| `image_kind` | `ImageKind?` | `nil` | Heuristic classification of what this image likely depicts. `nil` if classification was disabled or inconclusive. |
| `kind_confidence` | `Float?` | `nil` | Confidence score for `image_kind`, in the range 0.0 to 1.0. |
| `cluster_id` | `Integer?` | `nil` | Identifier shared across images that form a single logical figure (e.g. all raster tiles of one technical drawing). `nil` for singletons. |
| `caption` | `String?` | `nil` | VLM-generated caption describing the image, when captioning is configured. Populated by the captioning post-processor (`crates/xberg/src/plugins/processor/builtin/captioning.rs`), which routes each image through `crate.llm.region_extractor.extract_region_with_vlm` in caption mode. `nil` when captioning is disabled or the VLM declined to caption. |
| `qr_codes` | `Array<QrCode>?` | `\[\]` | QR codes decoded from this image, when QR detection is enabled. Populated by the QR post-processor (`crates/xberg/src/extractors/qr.rs`) via the pure-Rust `rqrr` decoder. `nil` when QR detection is disabled; an empty `Some(\[\])` when detection ran but found nothing. |
| `data_base64` | `String?` | `nil` | Base64-encoded copy of `data`; populated when `ImageExtractionConfig.include_data_base64` is `true`. Omitted from JSON by default; use instead of `data` in JSON-only clients. |

---

#### ExtractedUri

A URI extracted from a document.

Represents any link, reference, or resource pointer found during extraction.
The `kind` field classifies the URI semantically, while `label` carries
optional human-readable display text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | `String` | — | The URL or path string. |
| `label` | `String?` | `nil` | Optional display text / label for the link. |
| `page` | `Integer?` | `nil` | Optional page number where the URI was found (1-indexed). |
| `kind` | `UriKind` | — | Semantic classification of the URI. |

---

#### ExtractionConfidence

Combined confidence on `[0, 1]`.

When OCR did not run, the `ocr_aggregate` weight folds into `text_coverage`
so the weighted sum still totals 1.0.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text_coverage` | `Float` | — | Fraction of pages with a usable text layer. |
| `ocr_aggregate` | `Float?` | `nil` | Mean OCR per-element recognition confidence when OCR ran; `nil` when it did not. |
| `schema_compliance` | `SchemaCompliance` | — | Whether the merged output validates against the preset schema. |
| `combined` | `Float` | — | Weighted blend in `\[0, 1\]`.  The value compared against the fallback threshold. |

---

#### ExtractionConfig

Main extraction configuration.

This struct contains all configuration options for the extraction process.
It can be loaded from TOML, YAML, or JSON files, or created programmatically.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `use_cache` | `Boolean` | `true` | Enable caching of extraction results |
| `enable_quality_processing` | `Boolean` | `true` | Enable quality post-processing |
| `ocr` | `OcrConfig?` | `nil` | OCR configuration (None = OCR disabled) |
| `force_ocr` | `Boolean` | `false` | Force OCR even for searchable PDFs |
| `force_ocr_pages` | `Array<Integer>?` | `nil` | Force OCR on specific pages only (1-indexed page numbers, must be >= 1). When set, only the listed pages are OCR'd regardless of text layer quality. Unlisted pages use native text extraction. Ignored when `force_ocr` is `true`. Only applies to PDF documents. Duplicates are automatically deduplicated. An `ocr` config is recommended for backend/language selection; defaults are used if absent. |
| `disable_ocr` | `Boolean` | `false` | Disable OCR entirely, even for images. When `true`, OCR is skipped for all document types. Images return metadata only (dimensions, format, EXIF) without text extraction. PDFs use only native text extraction without OCR fallback. Cannot be `true` simultaneously with `force_ocr`. *Added in v4.7.0.* |
| `chunking` | `ChunkingConfig?` | `nil` | Text chunking configuration (None = chunking disabled) |
| `content_filter` | `ContentFilterConfig?` | `nil` | Content filtering configuration (None = use extractor defaults). Controls whether document "furniture" (headers, footers, watermarks, repeating text) is included in or stripped from extraction results. See `ContentFilterConfig` for per-field documentation. |
| `images` | `ImageExtractionConfig?` | `nil` | Image extraction configuration (None = no image extraction) |
| `pdf_options` | `PdfConfig?` | `nil` | PDF-specific options (None = use defaults) |
| `token_reduction` | `TokenReductionOptions?` | `nil` | Token reduction configuration (None = no token reduction) |
| `language_detection` | `LanguageDetectionConfig?` | `nil` | Language detection configuration (None = no language detection) |
| `pages` | `PageConfig?` | `nil` | Page extraction configuration (None = no page tracking) |
| `keywords` | `KeywordConfig?` | `nil` | Keyword extraction configuration (None = no keyword extraction) |
| `postprocessor` | `PostProcessorConfig?` | `nil` | Post-processor configuration (None = use defaults) |
| `html_output` | `HtmlOutputConfig?` | `nil` | Styled HTML output configuration. When set alongside `output_format = OutputFormat.Html`, the extraction pipeline uses `StyledHtmlRenderer` which emits stable `kb-*` CSS class hooks on every structural element and optionally embeds theme CSS or user-supplied CSS in a `<style>` block. When `nil`, the existing plain comrak-based HTML renderer is used. |
| `extraction_timeout_secs` | `Integer?` | `nil` | Default per-file timeout in seconds for batch extraction. When set, each file in a batch will be canceled after this duration unless overridden by `FileExtractionConfig.timeout_secs`. Defaults to `Some(60)` to prevent pathological files (e.g. deeply nested archives, documents with millions of cells) from running indefinitely and exhausting caller resources. Set to `nil` to disable the timeout for trusted input or long-running workloads. |
| `max_concurrent_extractions` | `Integer?` | `nil` | Maximum concurrent extractions in batch operations (None = (num_cpus × 1.5).ceil()). Limits parallelism to prevent resource exhaustion when processing large batches. Defaults to (num_cpus × 1.5).ceil() when not set. |
| `result_format` | `ResultFormat` | `:unified` | Result structure format Controls whether results are returned in unified format (default) with all content in the `content` field, or element-based format with semantic elements (for Unstructured-compatible output). |
| `security_limits` | `SecurityLimits?` | `nil` | Security limits for archive extraction. Controls maximum archive size, compression ratio, file count, and other security thresholds to prevent decompression bomb attacks. Also caps nesting depth, iteration count, entity / token length, total content size, and table cell count for every extraction path that ingests user-controlled bytes. When `nil`, default limits are used. |
| `max_embedded_file_bytes` | `Integer?` | `nil` | Maximum uncompressed size in bytes for a single embedded file before recursive extraction is attempted (default: 50 MiB). Applies to embedded objects inside OOXML containers (DOCX, PPTX) and to email attachments processed via recursive extraction. Files that exceed this limit are skipped with a `ProcessingWarning` rather than passed to the extraction pipeline, preventing a single oversized embedded object from consuming unbounded memory or time. Set to `nil` to disable the per-embedded-file cap (falls back to `security_limits.max_archive_size` as the only guard). |
| `output_format` | `OutputFormat` | `:plain` | Content text format (default: Plain). Controls the format of the extracted content: - `Plain`: Raw extracted text (default) - `Markdown`: Markdown formatted output - `Djot`: Djot markup format (requires djot feature) - `Html`: HTML formatted output When set to a structured format, extraction results will include formatted output. The `formatted_content` field may be populated when format conversion is applied. |
| `layout` | `LayoutDetectionConfig?` | `nil` | Layout detection configuration (None = layout detection disabled). When set, PDF pages and images are analyzed for document structure (headings, code, formulas, tables, figures, etc.) using RT-DETR models via ONNX Runtime. For PDFs, layout hints override paragraph classification in the markdown pipeline. For images, per-region OCR is performed with markdown formatting based on detected layout classes. Requires the `layout-detection` feature to run inference; the field is present whenever the `layout-types` feature is active (which includes `layout-detection` as well as the no-ORT target groups). |
| `transcription` | `TranscriptionConfig?` | `nil` | Transcription (speech-to-text) configuration for audio/video files. When set and `enabled`, files with audio/video MIME types (mp3, mp4, m4a, wav, webm, etc.) are routed to the Whisper-based transcription pipeline. The actual heavy dependencies are only active under the `transcription` feature; the field is visible under `transcription-types` (including on WASM and Android targets that use the no-ORT preset). Default: `nil` (transcription disabled). This is an additive, non-breaking change. |
| `use_layout_for_markdown` | `Boolean` | `false` | Run layout detection on the non-OCR PDF markdown path. When `true` and `layout` is `Some(_)`, layout regions inform heading, table, list, and figure detection in the structure pipeline that would otherwise rely on font-clustering heuristics alone. Significantly improves SF1 (structural F1) at the cost of inference latency (~150-300ms/page CPU, ~20-50ms/page GPU). Default: `false`. Requires the `layout-detection` feature. |
| `include_document_structure` | `Boolean` | `false` | Enable structured document tree output. When true, populates the `document` field on `ExtractionResult` with a hierarchical `DocumentStructure` containing heading-driven section nesting, table grids, content layer classification, and inline annotations. Independent of `result_format` — can be combined with Unified or ElementBased. |
| `acceleration` | `AccelerationConfig?` | `nil` | Hardware acceleration configuration for ONNX Runtime models. Controls execution provider selection for layout detection and embedding models. When `nil`, uses platform defaults (CoreML on macOS, CUDA on Linux, CPU on Windows). |
| `cache_namespace` | `String?` | `nil` | Cache namespace for tenant isolation. When set, cache entries are stored under `{cache_dir}/{namespace}/`. Must be alphanumeric, hyphens, or underscores only (max 64 chars). Different namespaces have isolated cache spaces on the same filesystem. |
| `cache_ttl_secs` | `Integer?` | `nil` | Per-request cache TTL in seconds. Overrides the global `max_age_days` for this specific extraction. When `0`, caching is completely skipped (no read or write). When `nil`, the global TTL applies. |
| `email` | `EmailConfig?` | `nil` | Email extraction configuration (None = use defaults). Currently supports configuring the fallback codepage for MSG files that do not specify one. See `EmailConfig` for details. |
| `url` | `UrlExtractionConfig` | — | URL ingestion and crawl configuration. |
| `max_archive_depth` | `Integer` | — | Maximum recursion depth for archive extraction (default: 3). Set to 0 to disable recursive extraction (legacy behavior). |
| `tree_sitter` | `TreeSitterConfig?` | `nil` | Tree-sitter language pack configuration (None = tree-sitter disabled). When set, enables code file extraction using tree-sitter parsers. Controls grammar download behavior and code analysis options. |
| `structured_extraction` | `StructuredExtractionConfig?` | `nil` | Structured extraction via LLM (None = disabled). When set, the extracted document content is sent to an LLM with the provided JSON schema. The structured response is stored in `ExtractionResult.structured_output`. |
| `ner` | `NerConfig?` | `nil` | Named-entity recognition configuration. When set, the NER post-processor runs at the Middle stage and populates `ExtractionResult.entities`. |
| `redaction` | `RedactionConfig?` | `nil` | Redaction / anonymisation configuration. When set, the redaction post-processor runs at the Late stage and rewrites every textual field in `ExtractionResult`, emitting an audit trail in `ExtractionResult.redaction_report`. |
| `summarization` | `SummarizationConfig?` | `nil` | Summarisation configuration. When set, the summarisation post-processor runs at the Middle stage and populates `ExtractionResult.summary`. |
| `translation` | `TranslationConfig?` | `nil` | Translation configuration. When set, the translation post-processor runs at the Middle stage and populates `ExtractionResult.translation`. |
| `page_classification` | `PageClassificationConfig?` | `nil` | Per-page classification configuration. When set, the classification post-processor runs at the Middle stage and populates `ExtractionResult.page_classifications`. |
| `captioning` | `CaptioningConfig?` | `nil` | VLM captioning configuration for extracted images. When set, the captioning post-processor runs at the Middle stage and writes a caption into each `ExtractedImage.caption`. |
| `qr_codes` | `Boolean?` | `nil` | Enable QR-code detection in extracted images. When `true`, the QR post-processor runs at the Middle stage and populates `ExtractedImage.qr_codes`. |

##### Methods

###### default()

**Signature:**

```ruby
def self.default()
```

**Example:**

```ruby
result = ExtractionConfig.default()
```

**Returns:** `ExtractionConfig`

###### needs_image_data()

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

```ruby
def needs_image_data()
```

**Example:**

```ruby
result = instance.needs_image_data()
```

**Returns:** `Boolean`

###### needs_image_processing()

Returns `true` when any image processing is needed during extraction.

##### Optimization Impact

For text-only extractions (no OCR, no image extraction, no captioning), skipping
image decompression can improve CPU utilization by 5-10% by avoiding wasteful
image I/O and processing when results won't be used.

**Signature:**

```ruby
def needs_image_processing()
```

**Example:**

```ruby
result = instance.needs_image_processing()
```

**Returns:** `Boolean`

---

#### ExtractionDiff

The complete diff between two `ExtractionResult` values.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content_diff` | `Array<DiffHunk>` | `\[\]` | Unified-diff hunks for the `content` field. Empty when the content is identical. |
| `tables_added` | `Array<Table>` | `\[\]` | Tables present in `b` but not in `a` (by index position, excess right-side tables). |
| `tables_removed` | `Array<Table>` | `\[\]` | Tables present in `a` but not in `b` (by index position, excess left-side tables). |
| `tables_changed` | `Array<TableDiff>` | `\[\]` | Cell-level changes for table pairs that share the same index and dimensions. |
| `metadata_changed` | `Object` | — | Metadata difference, encoded as a JSON object with three top-level keys: `added` (keys present in `b` but not `a`), `removed` (keys present in `a` but not `b`), and `changed` (keys whose values differ — each entry is `{ "from": <value-in-a>, "to": <value-in-b> }`). This is NOT RFC 6902 JSON Patch — we deliberately chose a flatter shape to avoid pulling in a json-patch crate. If you need RFC 6902 semantics (with JSON Pointer paths) feed `a.metadata` and `b.metadata` to your preferred json-patch impl directly. |
| `embedded_changes` | `EmbeddedChanges` | — | Changes to embedded archive children. |

---

#### ExtractionErrorItem

Non-fatal per-input extraction error captured by `ExtractionOutput`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `index` | `Integer` | — | Input index in the original request. |
| `code` | `Integer` | — | Stable numeric error code. |
| `error_type` | `String` | — | Stable snake_case error kind. |
| `source` | `String` | — | Best-effort source identifier. |
| `message` | `String` | — | Error message. |

---

#### ExtractionOutput

Unified extraction output envelope.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `results` | `Array<ExtractionResult>` | `\[\]` | Extraction results in discovery order. |
| `errors` | `Array<ExtractionErrorItem>` | `\[\]` | Non-fatal per-input errors. |
| `summary` | `ExtractionSummary` | — | Aggregate counts for the operation. |
| `crawl_final_urls` | `Array<String>` | `\[\]` | Final URLs reached after redirects during URL ingestion. |
| `crawl_redirect_count` | `Integer` | — | Total redirects followed while fetching or crawling URLs. |
| `crawl_unique_normalized_urls` | `Array<String>` | `\[\]` | Unique normalized URLs discovered by crawls. |

##### Methods

###### single()

Build an output containing one successful result.

**Signature:**

```ruby
def self.single(result)
```

**Example:**

```ruby
result = ExtractionOutput.single(ExtractionResult.new)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `result` | `ExtractionResult` | Yes | The extraction result |

**Returns:** `ExtractionOutput`

---

#### ExtractionResult

General extraction result used by the core extraction API.

This is the main result type returned by all extraction functions.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String` | — | Plain-text representation of the extracted document content. |
| `mime_type` | `String` | — | MIME type of the source document (e.g. `"application/pdf"`). |
| `metadata` | `Metadata` | — | Document-level metadata (author, title, dates, format-specific fields). |
| `extraction_method` | `ExtractionMethod?` | `nil` | Extraction strategy used to produce the returned text. Populated when the extractor can reliably distinguish native text extraction, OCR-only extraction, or mixed native/OCR output. |
| `tables` | `Array<Table>` | `\[\]` | Tables extracted from the document, each with structured cell data. |
| `detected_languages` | `Array<String>?` | `\[\]` | ISO 639-1 language codes detected in the document content. |
| `chunks` | `Array<Chunk>?` | `\[\]` | Text chunks when chunking is enabled. When chunking configuration is provided, the content is split into overlapping chunks for efficient processing. Each chunk contains the text, optional embeddings (if enabled), and metadata about its position. |
| `images` | `Array<ExtractedImage>?` | `\[\]` | Extracted images from the document. When image extraction is enabled via `ImageExtractionConfig`, this field contains all images found in the document with their raw data and metadata. Each image may optionally contain a nested `ocr_result` if OCR was performed. |
| `pages` | `Array<PageContent>?` | `\[\]` | Per-page content when page extraction is enabled. When page extraction is configured, the document is split into per-page content with tables and images mapped to their respective pages. |
| `elements` | `Array<Element>?` | `\[\]` | Semantic elements when element-based result format is enabled. When result_format is set to ElementBased, this field contains semantic elements with type classification, unique identifiers, and metadata for Unstructured-compatible element-based processing. |
| `djot_content` | `DjotContent?` | `nil` | Rich Djot content structure (when extracting Djot documents). When extracting Djot documents with structured extraction enabled, this field contains the full semantic structure including: - Block-level elements with nesting - Inline formatting with attributes - Links, images, footnotes - Math expressions - Complete attribute information The `content` field still contains plain text for backward compatibility. Always `nil` for non-Djot documents. |
| `ocr_elements` | `Array<OcrElement>?` | `\[\]` | OCR elements with full spatial and confidence metadata. When OCR is performed with element extraction enabled, this field contains the structured representation of detected text including: - Bounding geometry (rectangles or quadrilaterals) - Confidence scores (detection and recognition) - Rotation information - Hierarchical relationships (Tesseract only) This field preserves all metadata that would otherwise be lost when converting to plain text or markdown output formats. Only populated when `OcrElementConfig.include_elements` is true. |
| `document` | `DocumentStructure?` | `nil` | Structured document tree (when document structure extraction is enabled). When `include_document_structure` is true in `ExtractionConfig`, this field contains the full hierarchical representation of the document including: - Heading-driven section nesting - Table grids with cell-level metadata - Content layer classification (body, header, footer, footnote) - Inline text annotations (formatting, links) - Bounding boxes and page numbers Independent of `result_format` — can be combined with Unified or ElementBased. |
| `extracted_keywords` | `Array<Keyword>?` | `\[\]` | Extracted keywords when keyword extraction is enabled. When keyword extraction (RAKE or YAKE) is configured, this field contains the extracted keywords with scores, algorithm info, and position data. Previously stored in `metadata.additional\["keywords"\]`. |
| `quality_score` | `Float?` | `nil` | Document quality score from quality analysis. A value between 0.0 and 1.0 indicating the overall text quality. Previously stored in `metadata.additional\["quality_score"\]`. |
| `processing_warnings` | `Array<ProcessingWarning>` | `\[\]` | Non-fatal warnings collected during processing pipeline stages. Captures errors from optional pipeline features (embedding, chunking, language detection, output formatting) that don't prevent extraction but may indicate degraded results. Previously stored as individual keys in `metadata.additional`. |
| `annotations` | `Array<PdfAnnotation>?` | `\[\]` | PDF annotations extracted from the document. When annotation extraction is enabled via `PdfConfig.extract_annotations`, this field contains text notes, highlights, links, stamps, and other annotations found in PDF documents. |
| `children` | `Array<ArchiveEntry>?` | `\[\]` | Nested extraction results from archive contents. When extracting archives, each processable file inside produces its own full extraction result. Set to `nil` for non-archive formats. Use `max_archive_depth` in config to control recursion depth. |
| `uris` | `Array<ExtractedUri>?` | `\[\]` | URIs/links discovered during document extraction. Contains hyperlinks, image references, citations, email addresses, and other URI-like references found in the document. Always extracted when present in the source document. |
| `revisions` | `Array<DocumentRevision>?` | `\[\]` | Tracked changes embedded in the source document. Populated by per-format extractors that understand change-tracking metadata (DOCX `w:ins`/`w:del`/`w:rPrChange`, ODT `text:change-*`, …). Every extractor defaults to `nil` until its format-specific implementation is added. Extractors that do populate this field follow the "accepted-changes" convention: inserted text is present in `content`, deleted text is absent — the revision list is the separate audit trail. |
| `structured_output` | `Object?` | `nil` | Structured extraction output from LLM-based JSON schema extraction. When `structured_extraction` is configured in `ExtractionConfig`, the extracted document content is sent to a VLM with the provided JSON schema. The response is parsed and stored here as a JSON value matching the schema. |
| `code_intelligence` | `Object?` | `nil` | Code intelligence results from tree-sitter analysis. Populated when extracting source code files with the `tree-sitter` feature. Contains metrics, structural analysis, imports/exports, comments, docstrings, symbols, diagnostics, and optionally chunked code segments. Stored as an opaque JSON value so that all language bindings (Go, Java, C#, …) can deserialize it as a raw JSON object rather than a typed struct. The underlying type is `tree_sitter_language_pack.ProcessResult`. |
| `llm_usage` | `Array<LlmUsage>?` | `\[\]` | LLM token usage and cost data for all LLM calls made during this extraction. Contains one entry per LLM call. Multiple entries are produced when VLM OCR, structured extraction, or LLM embeddings run during the same extraction. `nil` when no LLM was used. |
| `entities` | `Array<Entity>?` | `\[\]` | Named entities detected in `content` by the NER post-processor. `nil` when no NER backend is configured. Populated by the `xberg-gliner` ONNX backend or the LLM-driven backend (see `crates/xberg/src/text/ner/`). |
| `summary` | `DocumentSummary?` | `nil` | Summary of `content` produced by the summarisation post-processor. `nil` when summarisation is not configured. Populated by the TextRank extractive backend (deterministic, no external service) or by the liter-llm-driven abstractive backend. |
| `extraction_confidence` | `ExtractionConfidence?` | `nil` | Confidence score computed by the heuristics pipeline. Populated when the `heuristics` feature is enabled and confidence scoring has been performed.  Combines text-coverage, OCR aggregate confidence, and schema-compliance into a single `\[0, 1\]` value. `nil` when confidence scoring is not configured or the feature is absent. |
| `translation` | `Translation?` | `nil` | Translation of `content` produced by the translation post-processor. `nil` when translation is not configured. |
| `page_classifications` | `Array<PageClassification>?` | `\[\]` | Per-page classifications produced by the page-classification post-processor. `nil` when classification is not configured. |
| `redaction_report` | `RedactionReport?` | `nil` | Audit report of redactions applied by the redaction post-processor. The redaction processor rewrites `content`, `formatted_content`, every chunk's text, and the textual fields of `entities` / `summary` / `translation` / `page_classifications` in place. This report describes what was found and how it was replaced. `nil` when redaction is not configured. |
| `formulas` | `Array<Formula>` | `\[\]` | Mathematical formulas recognized in the document. Populated by the layout-guided formula pipeline when the `layout-detection` feature is enabled and the document contains regions classified as formulas. Empty otherwise. |
| `form_fields` | `Array<PdfFormField>` | `\[\]` | Form fields extracted from a PDF's AcroForm or XFA structure. Populated by the PDF extractor when `PdfConfig.extract_form_fields` is enabled (default) and the document is a fillable form. Empty otherwise. |
| `formatted_content` | `String?` | `nil` | Pre-rendered content in the requested output format. Populated during `derive_extraction_result` before tree derivation consumes element data. `apply_output_format` swaps this into `content` at the end of the pipeline, after post-processors have operated on plain text. |

##### Methods

###### from_ocr()

Convert from an OCR result.

**Signature:**

```ruby
def self.from_ocr(ocr)
```

**Example:**

```ruby
result = ExtractionResult.from_ocr(OcrExtractionResult.new)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `ocr` | `OcrExtractionResult` | Yes | The ocr extraction result |

**Returns:** `ExtractionResult`

---

#### ExtractionSummary

Summary for a unified extraction call.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `inputs` | `Integer` | — | Number of inputs submitted by the caller. |
| `results` | `Integer` | — | Number of extraction results produced. |
| `errors` | `Integer` | — | Number of per-input errors. |
| `remote_urls` | `Integer` | — | Number of URI inputs that resolved to remote HTTP(S) URLs. |
| `pages_crawled` | `Integer` | — | Number of HTML pages crawled or scraped. |
| `documents_downloaded` | `Integer` | — | Number of downloaded non-HTML documents extracted from URLs. |

---

#### FictionBookMetadata

FictionBook (FB2) metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `genres` | `Array<String>` | `\[\]` | Genre tags as declared in the FB2 `<genre>` elements. |
| `sequences` | `Array<String>` | `\[\]` | Book series (sequence) names, if any. |
| `annotation` | `String?` | `nil` | Short annotation / summary from the FB2 `<annotation>` element. |

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
| `enable_quality_processing` | `Boolean?` | `nil` | Override quality post-processing for this file. |
| `ocr` | `OcrConfig?` | `nil` | Override OCR configuration for this file (None in the Option = use batch default). |
| `force_ocr` | `Boolean?` | `nil` | Override force OCR for this file. |
| `force_ocr_pages` | `Array<Integer>?` | `\[\]` | Override force OCR pages for this file (1-indexed page numbers). |
| `disable_ocr` | `Boolean?` | `nil` | Override disable OCR for this file. |
| `chunking` | `ChunkingConfig?` | `nil` | Override chunking configuration for this file. |
| `content_filter` | `ContentFilterConfig?` | `nil` | Override content filtering configuration for this file. |
| `images` | `ImageExtractionConfig?` | `nil` | Override image extraction configuration for this file. |
| `pdf_options` | `PdfConfig?` | `nil` | Override PDF options for this file. |
| `token_reduction` | `TokenReductionOptions?` | `nil` | Override token reduction for this file. |
| `language_detection` | `LanguageDetectionConfig?` | `nil` | Override language detection for this file. |
| `pages` | `PageConfig?` | `nil` | Override page extraction for this file. |
| `keywords` | `KeywordConfig?` | `nil` | Override keyword extraction for this file. |
| `postprocessor` | `PostProcessorConfig?` | `nil` | Override post-processor for this file. |
| `result_format` | `ResultFormat?` | `nil` | Override result format for this file. |
| `output_format` | `OutputFormat?` | `nil` | Override output content format for this file. |
| `include_document_structure` | `Boolean?` | `nil` | Override document structure output for this file. |
| `layout` | `LayoutDetectionConfig?` | `nil` | Override layout detection for this file. |
| `transcription` | `TranscriptionConfig?` | `nil` | Transcription configuration (see ExtractionConfig for docs). |
| `timeout_secs` | `Integer?` | `nil` | Override per-file extraction timeout in seconds. When set, the extraction for this file will be canceled after the specified duration. A timed-out file produces an error result without affecting other files in the batch. |
| `tree_sitter` | `TreeSitterConfig?` | `nil` | Override tree-sitter configuration for this file. |
| `structured_extraction` | `StructuredExtractionConfig?` | `nil` | Override structured extraction configuration for this file. When set, enables LLM-based structured extraction with a JSON schema for this specific file. The extracted content is sent to a VLM/LLM and the response is parsed according to the provided schema. |

---

#### Footnote

Footnote in Djot.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `label` | `String` | — | Footnote label |
| `content` | `Array<FormattedBlock>` | — | Footnote content blocks |

---

#### FootnoteAnchor

A footnote anchor reference in markdown text.

Represents a `[^label]` use-site (not a definition).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `label` | `String` | — | The label of the footnote reference (e.g., "1" in `\[^1\]`). |
| `offset` | `Integer` | — | Byte offset of the anchor in the markdown text. |

---

#### FootnoteConfig

Configuration for markdown footnote and citation parsing.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `parse_citations` | `Boolean` | `true` | Whether to parse the structured citation block (default: true). When enabled, the parser will look for and extract citations from the block after `---` + `<!-- citations ... -->`. |

##### Methods

###### default()

**Signature:**

```ruby
def self.default()
```

**Example:**

```ruby
result = FootnoteConfig.default()
```

**Returns:** `FootnoteConfig`

###### with_parse_citations()

Set whether to parse the citation block.

**Signature:**

```ruby
def with_parse_citations(enabled)
```

**Example:**

```ruby
result = instance.with_parse_citations(true)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `enabled` | `Boolean` | Yes | The enabled |

**Returns:** `FootnoteConfig`

---

#### FootnoteDefinition

A footnote definition from markdown text.

Represents `[^label]: content` declarations (including multi-line continuations).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `label` | `String` | — | The label of the footnote (e.g., "1" in `\[^1\]: ...`). |
| `content` | `String` | — | The full content of the footnote definition. |
| `offset` | `Integer` | — | Byte offset of the definition line in the markdown text. |

---

#### FormattedBlock

Block-level element in a Djot document.

Represents structural elements like headings, paragraphs, lists, code blocks, etc.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `block_type` | `BlockType` | — | Type of block element |
| `level` | `Integer?` | `nil` | Heading level (1-6) for headings, or nesting level for lists |
| `inline_content` | `Array<InlineElement>` | — | Inline content within the block |
| `language` | `String?` | `nil` | Language identifier for code blocks |
| `code` | `String?` | `nil` | Raw code content for code blocks |
| `children` | `Array<FormattedBlock>` | `/* serde(default) */` | Nested blocks for containers (blockquotes, list items, divs) |

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
| `latex` | `String` | — | LaTeX source of the recognized formula, without surrounding `$$` delimiters. This field contains the raw LaTeX code as produced by the OCR backend. To render the formula in Markdown or other formats, wrap with `$$..$$` delimiters as needed. |
| `bbox` | `BoundingBox` | — | Bounding box of the formula region on its page, in rendered-image pixel coordinates. The coordinates are in the space of the OCR-rendered page image at the OCR DPI (typically 300 DPI). These coordinates are NOT comparable to bounding boxes from native PDF text extraction, which use PDF point coordinates. |
| `page` | `Integer` | — | 1-indexed page number the formula appears on in the document. This is set by the extraction pipeline based on which page the formula was found on. |

---

#### GridCell

Individual grid cell with position and span metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String` | — | Cell text content. |
| `row` | `Integer` | — | Zero-indexed row position. |
| `col` | `Integer` | — | Zero-indexed column position. |
| `row_span` | `Integer` | `serde(default = "default_span")` | Number of rows this cell spans. |
| `col_span` | `Integer` | `serde(default = "default_span")` | Number of columns this cell spans. |
| `is_header` | `Boolean` | `/* serde(default) */` | Whether this is a header cell. |
| `bbox` | `BoundingBox?` | `nil` | Bounding box for this cell (if available). |

---

#### HeaderMetadata

Header/heading element metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `level` | `Integer` | — | Header level: 1 (h1) through 6 (h6) |
| `text` | `String` | — | Normalized text content of the header |
| `id` | `String?` | `nil` | HTML id attribute if present |
| `depth` | `Integer` | — | Document tree depth at the header element |
| `html_offset` | `Integer` | — | Byte offset in original HTML document |

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
| `level` | `Integer` | — | Heading depth (1 = h1, 2 = h2, etc.) |
| `text` | `String` | — | The text content of the heading. |

---

#### HeuristicsConfig

Configuration for document chunking and analysis heuristics.

Every threshold is a public field so callers can override any subset via
struct-update syntax: `HeuristicsConfig { text_layer_threshold: 0.5, ..the default constructor }`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enable_pdf_text_heuristics` | `Boolean` | `true` | Enable PDF text-layer detection heuristics. When `true`, PDFs with a substantial text layer will skip chunking. Default: `true`. |
| `text_layer_threshold` | `Float` | `0.7` | Minimum fraction of pages that must have text to skip chunking. Range `0.0..=1.0`. Default: `0.7` (70 % of pages). |
| `file_size_threshold_bytes` | `Integer` | `10485760` | File size threshold in bytes for considering chunking. Files smaller than this are processed without chunking. Default: 10 MiB (10 × 1 024 × 1 024). |
| `page_count_threshold` | `Integer` | `50` | Page count threshold for considering chunking. Documents with fewer pages are processed without chunking. Default: 50. |
| `target_pages_per_chunk` | `Integer` | `10` | Target number of pages per chunk for optimal parallel processing. Default: 10. |
| `max_pages_per_chunk` | `Integer` | `25` | Hard cap on pages per chunk. No chunk will exceed this limit. Must be ≥ `target_pages_per_chunk`. Default: 25. |
| `disk_processing_threshold_bytes` | `Integer` | `52428800` | File size threshold for disk-based processing. Files larger than this are buffered to disk to prevent OOM. Default: 50 MiB (50 × 1 024 × 1 024). |
| `min_chars_per_page` | `Integer` | `50` | Minimum characters per page to consider a page as having text. Default: 50. |
| `max_xlsx_sheet_count` | `Integer` | `200` | Maximum sheet count allowed in an XLSX workbook. Workbooks beyond this are rejected pre-extraction to avoid OOM / abusive billing inflation. Default: 200. |
| `max_xlsx_workbook_cells` | `Integer` | `5000000` | Maximum cell count (sheets × rows × columns approximation) in an XLSX workbook. Default: 5 000 000 (≈ 200 sheets × 25 k cells). |
| `max_pptx_embedded_count` | `Integer` | `50` | Maximum number of OLE-embedded objects extractable from a single PPTX or DOCX. Protects against zip-bomb-style nested-document abuse. Default: 50. |

##### Methods

###### default()

**Signature:**

```ruby
def self.default()
```

**Example:**

```ruby
result = HeuristicsConfig.default()
```

**Returns:** `HeuristicsConfig`

###### validate()

Validate the configuration.

**Errors:**

Returns `HeuristicsError.ConfigError` when:

- `target_pages_per_chunk` is 0
- `max_pages_per_chunk` < `target_pages_per_chunk`
- `file_size_threshold_bytes` is 0

**Signature:**

```ruby
def validate()
```

**Example:**

```ruby
instance.validate()
```

**Returns:** No return value.

**Errors:** Raises `Error`.

---

#### HierarchicalBlock

A text block with hierarchy level assignment.

Represents a block of text with semantic heading information extracted from
font size clustering and hierarchical analysis.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text` | `String` | — | The text content of this block |
| `font_size` | `Float` | — | The font size of the text in this block |
| `level` | `String` | — | The hierarchy level of this block (H1-H6 or Body) Levels correspond to HTML heading tags: - "h1": Top-level heading - "h2": Secondary heading - "h3": Tertiary heading - "h4": Quaternary heading - "h5": Quinary heading - "h6": Senary heading - "body": Body text (no heading level) |

---

#### HierarchyConfig

Hierarchy extraction configuration for PDF text structure analysis.

Enables extraction of document hierarchy levels (H1-H6) based on font size
clustering and semantic analysis. When enabled, hierarchical blocks are
included in page content.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `Boolean` | `true` | Enable hierarchy extraction |
| `k_clusters` | `Integer` | `3` | Number of font size clusters to use for hierarchy levels (1-7) Default: 6, which provides H1-H6 heading levels with body text. Larger values create more fine-grained hierarchy levels. |
| `include_bbox` | `Boolean` | `true` | Include bounding box information in hierarchy blocks |
| `ocr_coverage_threshold` | `Float?` | `nil` | OCR coverage threshold for smart OCR triggering (0.0-1.0) Determines when OCR should be triggered based on text block coverage. OCR is triggered when text blocks cover less than this fraction of the page. Default: 0.5 (trigger OCR if less than 50% of page has text) |

##### Methods

###### default()

**Signature:**

```ruby
def self.default()
```

**Example:**

```ruby
result = HierarchyConfig.default()
```

**Returns:** `HierarchyConfig`

---

#### HtmlMetadata

HTML metadata extracted from HTML documents.

Includes document-level metadata, Open Graph data, Twitter Card metadata,
and extracted structural elements (headers, links, images, structured data).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `title` | `String?` | `nil` | Document title from `<title>` tag |
| `description` | `String?` | `nil` | Document description from `<meta name="description">` tag |
| `keywords` | `Array<String>` | `\[\]` | Document keywords from `<meta name="keywords">` tag, split on commas |
| `author` | `String?` | `nil` | Document author from `<meta name="author">` tag |
| `canonical_url` | `String?` | `nil` | Canonical URL from `<link rel="canonical">` tag |
| `base_href` | `String?` | `nil` | Base URL from `<base href="">` tag for resolving relative URLs |
| `language` | `String?` | `nil` | Document language from `lang` attribute |
| `text_direction` | `TextDirection?` | `nil` | Document text direction from `dir` attribute |
| `open_graph` | `Hash{String=>String}` | `{}` | Open Graph metadata (og:* properties) for social media Keys like "title", "description", "image", "url", etc. |
| `twitter_card` | `Hash{String=>String}` | `{}` | Twitter Card metadata (twitter:* properties) Keys like "card", "site", "creator", "title", "description", "image", etc. |
| `meta_tags` | `Hash{String=>String}` | `{}` | Additional meta tags not covered by specific fields Keys are meta name/property attributes, values are content |
| `headers` | `Array<HeaderMetadata>` | `\[\]` | Extracted header elements with hierarchy |
| `links` | `Array<LinkMetadata>` | `\[\]` | Extracted hyperlinks with type classification |
| `images` | `Array<ImageMetadataType>` | `\[\]` | Extracted images with source and dimensions |
| `structured_data` | `Array<StructuredData>` | `\[\]` | Extracted structured data blocks |

---

#### HtmlOutputConfig

Configuration for styled HTML output.

When set on `html_output` alongside
`output_format = OutputFormat.Html`, the pipeline builds a
`StyledHtmlRenderer` instead of
the plain comrak-based renderer.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `css` | `String?` | `nil` | Inline CSS string injected into the output after the theme stylesheet. Concatenated after `css_file` content when both are set. |
| `css_file` | `String?` | `nil` | Path to a CSS file loaded once at renderer construction time. Concatenated before `css` when both are set. |
| `theme` | `HtmlTheme` | `:unstyled` | Built-in colour/typography theme. Default: `HtmlTheme.Unstyled`. |
| `class_prefix` | `String` | — | CSS class prefix applied to every emitted class name. Default: `"kb-"`. Change this if your host application already uses classes that start with `kb-`. |
| `embed_css` | `Boolean` | `true` | When `true` (default), write the resolved CSS into a `<style>` block immediately after the opening `<div class="{prefix}doc">`. Set to `false` to emit only the structural markup and wire up your own stylesheet targeting the `kb-*` class names. |

##### Methods

###### default()

**Signature:**

```ruby
def self.default()
```

**Example:**

```ruby
result = HtmlOutputConfig.default()
```

**Returns:** `HtmlOutputConfig`

---

#### ImageExtractionConfig

Image extraction configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `extract_images` | `Boolean` | `true` | Extract images from documents |
| `target_dpi` | `Integer` | `300` | Target DPI for image normalization |
| `max_image_dimension` | `Integer` | `4096` | Maximum dimension for images (width or height) |
| `inject_placeholders` | `Boolean` | `true` | Whether to inject image reference placeholders into markdown output. When `true` (default), image references like `!\[Image 1\](embedded:p1_i0)` are appended to the markdown. Set to `false` to extract images as data without polluting the markdown output. |
| `auto_adjust_dpi` | `Boolean` | `true` | Automatically adjust DPI based on image content |
| `min_dpi` | `Integer` | `72` | Minimum DPI threshold |
| `max_dpi` | `Integer` | `600` | Maximum DPI threshold |
| `max_images_per_page` | `Integer?` | `nil` | Maximum number of image objects to extract per PDF page. Some PDFs (e.g. technical diagrams stored as thousands of raster fragments) can trigger extremely long or indefinite extraction times when every image object on a dense page is decoded individually via the PDF extractor. Setting this limit causes xberg to stop collecting individual images once the count per page reaches the cap and emit a warning instead. `nil` (default) means no limit — all images are extracted. |
| `classify` | `Boolean` | `false` | When `true`, extracted images are classified by kind and grouped into clusters where they appear to belong to one figure. Defaults to `false` — opt in explicitly to avoid unexpected ML overhead. |
| `include_page_rasters` | `Boolean` | `false` | When `true`, full-page renders produced during OCR preprocessing are captured and returned as `ImageKind.PageRaster` entries in `ExtractionResult.images`. **PDF + OCR only.** No rasters are captured for non-PDF inputs or when the document-level OCR bypass is active (whole-document backend). When OCR is enabled and this flag is set but the active backend skips per-page rendering, a `ProcessingWarning` is emitted in `ExtractionResult.processing_warnings`. Defaults to `false`. Enable when downstream consumers need page thumbnails (e.g. citation previews, visual grounding). |
| `run_ocr_on_images` | `Boolean` | `true` | Run OCR on extracted images and include the recognized text in the document content. When `true` (default) and `ExtractionConfig.ocr` is configured, extracted images are processed with the configured OCR backend. Set to `false` to extract images without OCR processing, even when OCR is enabled. |
| `ocr_text_only` | `Boolean` | `false` | When `true`, image OCR results are rendered as plain text without the `!\[...\](...)` markdown placeholder. Only takes effect when `run_ocr_on_images` is also `true`. |
| `append_ocr_text` | `Boolean` | `false` | When `true` and `ocr_text_only` is `false`, append the OCR text after the image placeholder in the rendered output. |
| `output_format` | `ImageOutputFormat` | `:native` | Target format for re-encoding extracted images. When set to anything other than `Native`, each extracted image is re-encoded to the requested format before being returned. This lets callers receive uniform output without duplicating encode logic downstream. Defaults to `Native` — no re-encode pass is performed and `ExtractedImage.format` reflects the source extractor's output. |
| `svg` | `SvgOptions` | — | SVG-specific knobs for the image-encode pipeline. Controls sanitization and rasterization DPI when the source or output format is SVG.  Only available when the `svg` feature is active. |
| `include_data_base64` | `Boolean` | `false` | When `true`, populate `ExtractedImage.data_base64` with a Base64-encoded copy of the raw image bytes. Useful for JSON-only clients that cannot efficiently parse the default integer-array serialization of `data`. Defaults to `false`; enabling it doubles the in-memory image representation for the duration of the response. |

##### Methods

###### default()

**Signature:**

```ruby
def self.default()
```

**Example:**

```ruby
result = ImageExtractionConfig.default()
```

**Returns:** `ImageExtractionConfig`

---

#### ImageMetadata

Image metadata extracted from image files.

Includes dimensions, format, and EXIF data.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `width` | `Integer` | — | Image width in pixels |
| `height` | `Integer` | — | Image height in pixels |
| `format` | `String` | — | Image format (e.g., "PNG", "JPEG", "TIFF") |
| `exif` | `Hash{String=>String}` | `{}` | EXIF metadata tags |

---

#### ImageMetadataType

Image element metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `src` | `String` | — | Image source (URL, data URI, or SVG content) |
| `alt` | `String?` | `nil` | Alternative text from alt attribute |
| `title` | `String?` | `nil` | Title attribute |
| `image_type` | `ImageType` | — | Image type classification |

---

#### ImagePreprocessingConfig

Image preprocessing configuration for OCR.

These settings control how images are preprocessed before OCR to improve
text recognition quality. Different preprocessing strategies work better
for different document types.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `target_dpi` | `Integer` | `300` | Target DPI for the image (300 is standard, 600 for small text). |
| `auto_rotate` | `Boolean` | `false` | Auto-detect and correct image rotation. |
| `deskew` | `Boolean` | `true` | Correct skew (tilted images). |
| `denoise` | `Boolean` | `false` | Remove noise from the image. |
| `contrast_enhance` | `Boolean` | `false` | Enhance contrast for better text visibility. |
| `binarization_method` | `String` | `"otsu"` | Binarization method: "otsu", "sauvola", "adaptive". |
| `invert_colors` | `Boolean` | `false` | Invert colors (white text on black → black on white). |

##### Methods

###### default()

**Signature:**

```ruby
def self.default()
```

**Example:**

```ruby
result = ImagePreprocessingConfig.default()
```

**Returns:** `ImagePreprocessingConfig`

---

#### ImagePreprocessingMetadata

Image preprocessing metadata.

Tracks the transformations applied to an image during OCR preprocessing,
including DPI normalization, resizing, and resampling.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `target_dpi` | `Integer` | — | Target DPI from configuration |
| `scale_factor` | `Float` | — | Scaling factor applied to the image |
| `auto_adjusted` | `Boolean` | — | Whether DPI was auto-adjusted based on content |
| `final_dpi` | `Integer` | — | Final DPI after processing |
| `resample_method` | `String` | — | Resampling algorithm used ("LANCZOS3", "CATMULLROM", etc.) |
| `dimension_clamped` | `Boolean` | — | Whether dimensions were clamped to max_image_dimension |
| `calculated_dpi` | `Integer?` | `nil` | Calculated optimal DPI (if auto_adjust_dpi enabled) |
| `skipped_resize` | `Boolean` | — | Whether resize was skipped (dimensions already optimal) |
| `resize_error` | `String?` | `nil` | Error message if resize failed |

---

#### InlineElement

Inline element within a block.

Represents text with formatting, links, images, etc.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `element_type` | `InlineType` | — | Type of inline element |
| `content` | `String` | — | Text content |
| `metadata` | `Hash{String=>String}?` | `nil` | Additional metadata (e.g., href for links, src/alt for images) |

---

#### JatsMetadata

JATS (Journal Article Tag Suite) metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `copyright` | `String?` | `nil` | Copyright statement from the article's `<permissions>` element. |
| `license` | `String?` | `nil` | Open-access license URI from the article's `<license>` element. |
| `history_dates` | `Hash{String=>String}` | `{}` | Publication history dates keyed by event type (e.g. `"received"`, `"accepted"`). |
| `contributor_roles` | `Array<ContributorRole>` | `\[\]` | Authors and contributors with their stated roles. |

---

#### Keyword

Extracted keyword with metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text` | `String` | — | The keyword text. |
| `score` | `Float` | — | Relevance score (higher is better, algorithm-specific range). |
| `algorithm` | `KeywordAlgorithm` | — | Algorithm that extracted this keyword. |
| `positions` | `Array<Integer>?` | `nil` | Optional positions where keyword appears in text (character offsets). |

---

#### KeywordConfig

Keyword extraction configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `algorithm` | `KeywordAlgorithm` | `:yake` | Algorithm to use for extraction. |
| `max_keywords` | `Integer` | `10` | Maximum number of keywords to extract (default: 10). |
| `min_score` | `Float` | `0` | Minimum score threshold (0.0-1.0, default: 0.0). Keywords with scores below this threshold are filtered out. Note: Score ranges differ between algorithms. |
| `language` | `String?` | `nil` | Language code for stopword filtering (e.g., "en", "de", "fr"). If None, no stopword filtering is applied. |
| `yake_params` | `YakeParams?` | `nil` | YAKE-specific tuning parameters. |
| `rake_params` | `RakeParams?` | `nil` | RAKE-specific tuning parameters. |

##### Methods

###### default()

**Signature:**

```ruby
def self.default()
```

**Example:**

```ruby
result = KeywordConfig.default()
```

**Returns:** `KeywordConfig`

---

#### LanguageDetectionConfig

Language detection configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `Boolean` | `true` | Enable language detection |
| `min_confidence` | `Float` | `0.8` | Minimum confidence threshold (0.0-1.0) |
| `detect_multiple` | `Boolean` | `false` | Detect multiple languages in the document |

##### Methods

###### default()

**Signature:**

```ruby
def self.default()
```

**Example:**

```ruby
result = LanguageDetectionConfig.default()
```

**Returns:** `LanguageDetectionConfig`

---

#### LayoutDetection

A single layout detection result.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `class_name` | `LayoutClass` | — | Detected layout class (e.g. `Table`, `Text`, `Title`). |
| `confidence` | `Float` | — | Detection confidence score in `\[0.0, 1.0\]`. |
| `bbox` | `BBox` | — | Bounding box in image pixel coordinates. |

---

#### LayoutDetectionConfig

Layout detection configuration.

Controls layout detection behavior in the extraction pipeline.
When set on `ExtractionConfig`, layout detection
is enabled for PDF extraction.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `confidence_threshold` | `Float?` | `nil` | Confidence threshold override (None = use model default). |
| `apply_heuristics` | `Boolean` | `true` | Whether to apply postprocessing heuristics (default: true). |
| `table_model` | `TableModel` | `:tatr` | Table structure recognition model. Controls which model is used for table cell detection within layout-detected table regions. Defaults to `TableModel.Tatr`. |
| `acceleration` | `AccelerationConfig?` | `nil` | Hardware acceleration for ONNX models (layout detection + table structure). When set, controls which execution provider (CPU, CUDA, CoreML, TensorRT) is used for inference. Defaults to `nil` (auto-select per platform). |
| `enable_chart_understanding` | `Boolean` | `false` | Route regions classified as charts to the chart-understanding OCR task. When `true`, layout regions detected as charts are sent to the VLM chart task (data-series/axis recovery) instead of being treated as generic image regions. Defaults to `false` — chart understanding is opt-in and has no effect on standard text/table extraction scores. |

##### Methods

###### default()

**Signature:**

```ruby
def self.default()
```

**Example:**

```ruby
result = LayoutDetectionConfig.default()
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
| `class_name` | `String` | — | Layout class name (e.g. "picture", "table", "text", "section_header"). |
| `confidence` | `Float` | — | Confidence score from the layout detection model (0.0 to 1.0). |
| `bounding_box` | `BoundingBox` | — | Bounding box in document coordinate space. |
| `area_fraction` | `Float` | — | Fraction of the page area covered by this region (0.0 to 1.0). |

---

#### LinkMetadata

Link element metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `href` | `String` | — | The href URL value |
| `text` | `String` | — | Link text content (normalized) |
| `title` | `String?` | `nil` | Optional title attribute |
| `link_type` | `LinkType` | — | Link type classification |
| `rel` | `Array<String>` | — | Rel attribute values |

---

#### LlmBackend

liter-llm-backed NER backend.

##### Methods

###### new()

Create a new LLM-backed NER backend with the given LLM configuration.

**Signature:**

```ruby
def self.new(config)
```

**Example:**

```ruby
result = LlmBackend.new(LlmConfig.new)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `config` | `LlmConfig` | Yes | The configuration options |

**Returns:** `LlmBackend`

###### detect()

**Signature:**

```ruby
def detect(text, categories)
```

**Example:**

```ruby
result = instance.detect("value", [])
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `String` | Yes | The text |
| `categories` | `Array<EntityCategory>` | Yes | The categories |

**Returns:** `Array<Entity>`

**Errors:** Raises `Error`.

###### detect_with_custom()

**Signature:**

```ruby
def detect_with_custom(text, categories, custom_labels)
```

**Example:**

```ruby
result = instance.detect_with_custom("value", [], [])
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `String` | Yes | The text |
| `categories` | `Array<EntityCategory>` | Yes | The categories |
| `custom_labels` | `Array<String>` | Yes | The custom labels |

**Returns:** `Array<Entity>`

**Errors:** Raises `Error`.

---

#### LlmConfig

Configuration for an LLM provider/model via liter-llm.

Each feature (VLM OCR, VLM embeddings, structured extraction) carries
its own `LlmConfig`, allowing different providers per feature.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String` | — | Provider/model string using liter-llm routing format. Examples: `"openai/gpt-4o"`, `"anthropic/claude-sonnet-4-20250514"`, `"groq/llama-3.1-70b-versatile"`. |
| `api_key` | `String?` | `nil` | API key for the provider. When `nil`, liter-llm falls back to the provider's standard environment variable (e.g., `OPENAI_API_KEY`). |
| `base_url` | `String?` | `nil` | Custom base URL override for the provider endpoint. |
| `timeout_secs` | `Integer?` | `nil` | Request timeout in seconds (default: 60). |
| `max_retries` | `Integer?` | `nil` | Maximum retry attempts (default: 3). |
| `temperature` | `Float?` | `nil` | Sampling temperature for generation tasks. |
| `max_tokens` | `Integer?` | `nil` | Maximum tokens to generate. |

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
| `input_tokens` | `Integer?` | `nil` | Number of input/prompt tokens consumed. |
| `output_tokens` | `Integer?` | `nil` | Number of output/completion tokens generated. |
| `total_tokens` | `Integer?` | `nil` | Total tokens (input + output). |
| `estimated_cost` | `Float?` | `nil` | Estimated cost in USD based on the provider's published pricing. |
| `finish_reason` | `String?` | `nil` | Why the model stopped generating (e.g. "stop", "length", "content_filter"). |

---

#### MetaSchema

Compiled meta-schema validator over `preset.schema.json`.

##### Methods

###### compile()

Compile the given JSON text as a Draft 2020-12 meta-schema.

**Signature:**

```ruby
def self.compile(meta_schema_json)
```

**Example:**

```ruby
result = MetaSchema.compile("value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `meta_schema_json` | `String` | Yes | The meta schema json |

**Returns:** `MetaSchema`

**Errors:** Raises `LoadError`.

###### parse_preset()

Validate `raw` against the meta-schema and deserialize into a `Preset`,
stamping the fingerprint over the canonical file bytes.

**Signature:**

```ruby
def parse_preset(path, raw)
```

**Example:**

```ruby
result = instance.parse_preset("value", "data")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `path` | `String` | Yes | Path to the file |
| `raw` | `String` | Yes | The raw |

**Returns:** `Preset`

**Errors:** Raises `LoadError`.

---

#### Metadata

Extraction result metadata.

Contains common fields applicable to all formats, format-specific metadata
via a discriminated union, and additional custom fields from postprocessors.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `title` | `String?` | `nil` | Document title |
| `subject` | `String?` | `nil` | Document subject or description |
| `authors` | `Array<String>?` | `\[\]` | Primary author(s) - always Vec for consistency |
| `keywords` | `Array<String>?` | `\[\]` | Keywords/tags - always Vec for consistency |
| `language` | `String?` | `nil` | Primary language (ISO 639 code) |
| `created_at` | `String?` | `nil` | Creation timestamp (ISO 8601 format) |
| `modified_at` | `String?` | `nil` | Last modification timestamp (ISO 8601 format) |
| `created_by` | `String?` | `nil` | User who created the document |
| `modified_by` | `String?` | `nil` | User who last modified the document |
| `pages` | `PageStructure?` | `nil` | Page/slide/sheet structure with boundaries |
| `format` | `FormatMetadata?` | `nil` | Format-specific metadata (discriminated union) Contains detailed metadata specific to the document format. Serialized as a nested `"format"` object with a `format_type` discriminator field. |
| `image_preprocessing` | `ImagePreprocessingMetadata?` | `nil` | Image preprocessing metadata (when OCR preprocessing was applied) |
| `json_schema` | `Object?` | `nil` | JSON schema (for structured data extraction) |
| `error` | `ErrorMetadata?` | `nil` | Error metadata (for batch operations) |
| `extraction_duration_ms` | `Integer?` | `nil` | Extraction duration in milliseconds (for benchmarking). This field is populated by batch extraction to provide per-file timing information. It's `nil` for single-file extraction (which uses external timing). |
| `category` | `String?` | `nil` | Document category (from frontmatter or classification). |
| `tags` | `Array<String>?` | `\[\]` | Document tags (from frontmatter). |
| `document_version` | `String?` | `nil` | Document version string (from frontmatter). |
| `abstract_text` | `String?` | `nil` | Abstract or summary text (from frontmatter). |
| `output_format` | `String?` | `nil` | Output format identifier (e.g., "markdown", "html", "text"). Set by the output format pipeline stage when format conversion is applied. Previously stored in `metadata.additional\["output_format"\]`. |
| `ocr_used` | `Boolean` | — | Whether OCR was used during extraction. Set to `true` whenever the extraction pipeline ran an OCR backend (Tesseract, PaddleOCR, VLM, etc.) and used that output as the primary or fallback text. `false` means native text extraction was used exclusively. |
| `additional` | `Hash{String=>Object}` | `{}` | Additional custom fields from postprocessors. Serialized as a nested `"additional"` object (not flattened at root level). Uses `Cow<'static, str>` keys so static string keys avoid allocation. |

##### Methods

###### is_empty()

Returns `true` when no metadata fields, format-specific metadata, or
additional postprocessor fields are populated.

**Signature:**

```ruby
def is_empty()
```

**Example:**

```ruby
result = instance.is_empty()
```

**Returns:** `Boolean`

---

#### ModelPaths

Combined paths to all models needed for OCR (backward compatibility).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `det_model` | `String` | — | Path to the detection model directory. |
| `cls_model` | `String` | — | Path to the classification model directory. |
| `rec_model` | `String` | — | Path to the recognition model directory. |
| `dict_file` | `String` | — | Path to the character dictionary file. |

---

#### MultidocInput

Input signals for multi-document boundary detection.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `page_count` | `Integer` | — | Total number of pages in the PDF. |
| `pages` | `Array<PageSignals>` | — | Per-page signals extracted from the PDF. |

---

#### MultidocThresholds

Thresholds for multi-document boundary detection.

All fields are public; callers override any subset via struct-update syntax.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `density_shift_threshold` | `Float` | `0.3` | Text density difference threshold for `DensityShift` detection. Default: 0.3. |
| `bigram_overlap_min` | `Float` | `0.1` | Minimum bigram-overlap ratio below which a density shift is promoted to a `DensityShift` boundary.  Default: 0.1 (10 % overlap). |

##### Methods

###### default()

**Signature:**

```ruby
def self.default()
```

**Example:**

```ruby
result = MultidocThresholds.default()
```

**Returns:** `MultidocThresholds`

---

#### NerConfig

**Since:** `v5.0`

Configuration for the NER post-processor.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `backend` | `NerBackendKind` | `:onnx` | Backend that runs the entity detection. |
| `categories` | `Array<EntityCategory>` | `\[\]` | Entity categories to detect. Defaults to a sensible PERSON/ORG/LOCATION/EMAIL set when empty. |
| `model` | `String?` | `nil` | Override the default model — only used by `NerBackendKind.Onnx`. `nil` lets the backend pick its pinned default xberg GLiNER model alias. |
| `llm` | `LlmConfig?` | `nil` | Optional LLM configuration — only used by `NerBackendKind.Llm`. Token usage for LLM backends is recorded in `ExtractionResult.llm_usage`. |
| `custom_labels` | `Array<String>` | `\[\]` | Arbitrary user-supplied entity labels for zero-shot detection. `xberg-gliner` natively supports zero-shot inference over caller-supplied labels. The LLM backend also honours these labels by including them in the structured-output schema. Custom labels surface as `EntityCategory.Custom` in the resulting `Entity` stream. Use this when you need domain-specific entity types (e.g. `"Treatment"`, `"Product"`, `"Vessel"`) without forking GLiNER's taxonomy. |

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

###### process_image()

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

```ruby
def process_image(image_bytes, config)
```

**Example:**

```ruby
result = instance.process_image("data", OcrConfig.new)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `image_bytes` | `String` | Yes | Raw image data (JPEG, PNG, TIFF, etc.) |
| `config` | `OcrConfig` | Yes | OCR configuration (language, PSM mode, etc.) |

**Returns:** `ExtractionResult`

**Errors:** Raises `Error`.

###### process_image_file()

Process a file and extract text via OCR.

Default implementation reads the file and calls `process_image`.
Override for custom file handling or optimizations.

**Errors:**

Same as `process_image`, plus file I/O errors.

**Signature:**

```ruby
def process_image_file(path, config)
```

**Example:**

```ruby
result = instance.process_image_file("value", OcrConfig.new)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `path` | `String` | Yes | Path to the image file |
| `config` | `OcrConfig` | Yes | OCR configuration |

**Returns:** `ExtractionResult`

**Errors:** Raises `Error`.

###### supports_language()

Check if this backend supports a given language code.

**Returns:**

`true` if the language is supported, `false` otherwise.

**Signature:**

```ruby
def supports_language(lang)
```

**Example:**

```ruby
result = instance.supports_language("value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `lang` | `String` | Yes | ISO 639-2/3 language code (e.g., "eng", "deu", "fra") |

**Returns:** `Boolean`

###### backend_type()

Get the backend type identifier.

**Returns:**

The backend type enum value.

**Signature:**

```ruby
def backend_type()
```

**Example:**

```ruby
result = instance.backend_type()
```

**Returns:** `OcrBackendType`

###### supported_languages()

Optional: Get a list of all supported languages.

Defaults to empty list. Override to provide comprehensive language support info.

**Signature:**

```ruby
def supported_languages()
```

**Example:**

```ruby
result = instance.supported_languages()
```

**Returns:** `Array<String>`

###### supports_table_detection()

Optional: Check if the backend supports table detection.

Defaults to `false`. Override if your backend can detect and extract tables.

**Signature:**

```ruby
def supports_table_detection()
```

**Example:**

```ruby
result = instance.supports_table_detection()
```

**Returns:** `Boolean`

###### supports_document_processing()

Check if the backend supports direct document-level processing (e.g. for PDFs).

Defaults to `false`. Override if the backend has optimized document processing.

**Signature:**

```ruby
def supports_document_processing()
```

**Example:**

```ruby
result = instance.supports_document_processing()
```

**Returns:** `Boolean`

###### emits_structured_markdown()

Declare that this backend emits structured markdown directly (tables, headings, lists)
and downstream layout reconstruction should be skipped.

Defaults to `false` — classical OCR backends (Tesseract, PaddleOCR classical) return
plain text per detected region. End-to-end VLM backends (PaddleOCR-VL, GOT-OCR 2.0)
emit markdown in one forward pass and should override this to `true`.

**Signature:**

```ruby
def emits_structured_markdown()
```

**Example:**

```ruby
result = instance.emits_structured_markdown()
```

**Returns:** `Boolean`

###### process_document()

Process a document file directly via OCR.

Only called if `supports_document_processing` returns `true`.

**Signature:**

```ruby
def process_document(path, config)
```

**Example:**

```ruby
result = instance.process_document("value", OcrConfig.new)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `path` | `String` | Yes | The  path |
| `config` | `OcrConfig` | Yes | The ocr config |

**Returns:** `ExtractionResult`

**Errors:** Raises `Error`.

---

#### OcrConfidence

Confidence scores for an OCR element.

Separates detection confidence (how confident that text exists at this location)
from recognition confidence (how confident about the actual text content).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `detection` | `Float?` | `nil` | Detection confidence: how confident the OCR engine is that text exists here. PaddleOCR provides this as `box_score`, Tesseract doesn't have a direct equivalent. Range: 0.0 to 1.0 (or None if not available). |
| `recognition` | `Float` | — | Recognition confidence: how confident about the text content. Range: 0.0 to 1.0. |

---

#### OcrConfig

OCR configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `Boolean` | `true` | Whether OCR is enabled. Setting `enabled: false` is a shorthand for `disable_ocr: true` on the parent `ExtractionConfig`. Images return metadata only; PDFs use native text extraction without OCR fallback. Defaults to `true`. When `false`, all other OCR settings are ignored. |
| `backend` | `String` | — | OCR backend: tesseract, easyocr, paddleocr |
| `language` | `Array<String>` | `\[\]` | Language code(s) for OCR recognition. Accepts either a single language code ("eng") or a list (\["eng", "deu"\]). Defaults to \["eng"\]. For Tesseract, languages are joined with "+". |
| `tesseract_config` | `TesseractConfig?` | `nil` | Tesseract-specific configuration (optional) |
| `output_format` | `OutputFormat?` | `nil` | Output format for OCR results (optional, for format conversion) |
| `paddle_ocr_config` | `Object?` | `nil` | PaddleOCR-specific configuration (optional, JSON passthrough) |
| `backend_options` | `Object?` | `nil` | Arbitrary per-call options passed through to the backend unchanged. Custom OCR backends and built-in backends that support runtime tuning can read this value and deserialize the keys they care about. Keys unknown to the backend are silently ignored. This is the recommended extension point for per-call parameters that are not covered by the typed fields above (e.g. mode switching, preprocessing flags, inference batch size). **Scope:** when `pipeline` is `nil`, this value is propagated to the primary stage of the auto-constructed pipeline. When `pipeline` is explicitly set, this field has **no effect** — the caller must set `OcrPipelineStage.backend_options` directly on the relevant stage(s) instead. Example: ```json { "mode": "fast", "enable_layout": true, "timeout_ms": 5000 } ``` |
| `element_config` | `OcrElementConfig?` | `nil` | OCR element extraction configuration |
| `quality_thresholds` | `OcrQualityThresholds?` | `nil` | Quality thresholds for the native-text-to-OCR fallback decision. When None, uses compiled defaults (matching previous hardcoded behavior). |
| `pipeline` | `OcrPipelineConfig?` | `nil` | Multi-backend OCR pipeline configuration. When set, enables weighted fallback across multiple OCR backends based on output quality. When None, uses the single `backend` field (same as today). |
| `auto_rotate` | `Boolean` | `false` | Enable automatic page rotation based on orientation detection. When enabled, uses Tesseract's `DetectOrientationScript()` to detect page orientation (0/90/180/270 degrees) before OCR. If the page is rotated with high confidence, the image is corrected before recognition. This is critical for handling rotated scanned documents. |
| `vlm_fallback` | `VlmFallbackPolicy` | `:disabled` | Ergonomic VLM fallback policy. When set to anything other than `VlmFallbackPolicy.Disabled` and `OcrConfig.pipeline` is `nil`, a multi-stage pipeline is synthesised automatically: - `VlmFallbackPolicy.OnLowQuality` → `\[classical_stage, vlm_stage\]` with the `quality_threshold` mapped onto `OcrQualityThresholds.pipeline_min_quality`. - `VlmFallbackPolicy.Always` → `\[vlm_stage\]` only. Requires `OcrConfig.vlm_config` to be `Some` when not `Disabled`. When `OcrConfig.pipeline` is explicitly set, this field is ignored. |
| `vlm_config` | `LlmConfig?` | `nil` | VLM (Vision Language Model) OCR configuration. Required when `backend` is `"vlm"` or when `vlm_fallback` is not `VlmFallbackPolicy.Disabled`. Uses liter-llm to send page images to a vision model for text extraction. |
| `vlm_prompt` | `String?` | `nil` | Custom Jinja2 prompt template for VLM OCR. When `nil`, uses the default template. Available variables: - `{{ language }}` — The document language code (e.g., "eng", "deu"). |
| `acceleration` | `AccelerationConfig?` | `nil` | Hardware acceleration for ONNX Runtime models (e.g. PaddleOCR, layout detection). Not user-configurable via config files — injected at runtime from `ExtractionConfig.acceleration` before each `process_image` call. |
| `tessdata_bytes` | `Hash{String=>String}?` | `nil` | Caller-supplied Tesseract `traineddata` bytes per language code. Primary use case is the WASM build, which has no filesystem and cannot download tessdata at runtime. Native builds typically rely on `TessdataManager` and ignore this field. When present, the WASM Tesseract backend prefers these bytes over its compile-time-bundled English data. Skipped by serde to keep config files small — supply via the typed API at runtime. |
| `tessdata_path` | `String?` | `nil` | Runtime override for tessdata directory path. When set, uses this path as the highest-priority tessdata location, bypassing environment variables and cache directories. Useful for embedding pre-installed tessdata in applications. When `nil`, uses the standard resolution chain: TESSDATA_PREFIX env, cache dir, system paths. |

##### Methods

###### default()

**Signature:**

```ruby
def self.default()
```

**Example:**

```ruby
result = OcrConfig.default()
```

**Returns:** `OcrConfig`

---

#### OcrElement

A unified OCR element representing detected text with full metadata.

This is the primary type for structured OCR output, preserving all information
from both Tesseract and PaddleOCR backends.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text` | `String` | — | The recognized text content. |
| `geometry` | `OcrBoundingGeometry` | `:rectangle` | Bounding geometry (rectangle or quadrilateral). |
| `confidence` | `OcrConfidence` | — | Confidence scores for detection and recognition. |
| `level` | `OcrElementLevel` | `:line` | Hierarchical level (word, line, block, page). |
| `rotation` | `OcrRotation?` | `nil` | Rotation information (if detected). |
| `page_number` | `Integer` | — | Page number (1-indexed). |
| `parent_id` | `String?` | `nil` | Parent element ID for hierarchical relationships. Only used for Tesseract output which has word -> line -> block hierarchy. |
| `backend_metadata` | `Hash{String=>Object}` | `{}` | Backend-specific metadata that doesn't fit the unified schema. |

---

#### OcrElementConfig

Configuration for OCR element extraction.

Controls how OCR elements are extracted and filtered.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `include_elements` | `Boolean` | — | Whether to include OCR elements in the extraction result. When true, the `ocr_elements` field in `ExtractionResult` will be populated. |
| `min_level` | `OcrElementLevel` | `:line` | Minimum hierarchical level to include. Elements below this level (e.g., words when min_level is Line) will be excluded. |
| `min_confidence` | `Float` | — | Minimum recognition confidence threshold (0.0-1.0). Elements with confidence below this threshold will be filtered out. |
| `build_hierarchy` | `Boolean` | — | Whether to build hierarchical relationships between elements. When true, `parent_id` fields will be populated based on spatial containment. Only meaningful for Tesseract output. |

---

#### OcrExtractionResult

OCR extraction result.

Result of performing OCR on an image or scanned document,
including recognized text and detected tables.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String` | — | Recognized text content |
| `mime_type` | `String` | — | Original MIME type of the processed image |
| `metadata` | `Hash{String=>Object}` | — | OCR processing metadata (confidence scores, language, etc.) |
| `tables` | `Array<OcrTable>` | — | Tables detected and extracted via OCR |
| `ocr_elements` | `Array<OcrElement>?` | `/* serde(default) */` | Structured OCR elements with bounding boxes and confidence scores. Available when TSV output is requested or table detection is enabled. |

---

#### OcrMetadata

OCR processing metadata.

Captures information about OCR processing configuration and results.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `language` | `String` | — | OCR language code(s) used |
| `psm` | `Integer` | — | Tesseract Page Segmentation Mode (PSM) |
| `output_format` | `String` | — | Output format (e.g., "text", "hocr") |
| `table_count` | `Integer` | — | Number of tables detected |
| `table_rows` | `Integer?` | `nil` | Number of rows in the detected table (if a single table was found). |
| `table_cols` | `Integer?` | `nil` | Number of columns in the detected table (if a single table was found). |

---

#### OcrPipelineConfig

Multi-backend OCR pipeline with quality-based fallback.

Backends are tried in priority order (highest first). After each backend
produces output, quality is evaluated. If it meets `quality_thresholds.pipeline_min_quality`,
the result is accepted. Otherwise the next backend is tried.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `stages` | `Array<OcrPipelineStage>` | — | Ordered list of backends to try. Sorted by priority (descending) at runtime. |
| `quality_thresholds` | `OcrQualityThresholds` | `/* serde(default) */` | Quality thresholds for deciding whether to accept a result or try the next backend. |

---

#### OcrPipelineStage

A single backend stage in the OCR pipeline.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `backend` | `String` | — | Backend name: "tesseract", "paddleocr", "easyocr", or a custom registered name. |
| `priority` | `Integer` | `serde(default = "default_priority")` | Priority weight (higher = tried first). Stages are sorted by priority descending. |
| `language` | `Array<String>?` | `/* serde(default) */` | Language override for this stage (None = use parent OcrConfig.language). Accepts either a single language code ("eng") or a list (\["eng", "deu"\]). |
| `tesseract_config` | `TesseractConfig?` | `/* serde(default) */` | Tesseract-specific config override for this stage. |
| `paddle_ocr_config` | `Object?` | `/* serde(default) */` | PaddleOCR-specific config for this stage. |
| `vlm_config` | `LlmConfig?` | `/* serde(default) */` | VLM config override for this pipeline stage. |
| `backend_options` | `Object?` | `/* serde(default) */` | Arbitrary per-call options passed through to the backend unchanged. Backends that support runtime tuning (mode switching, preprocessing flags, inference parameters, etc.) read this value and deserialize the keys they care about. Keys unknown to the backend are silently ignored, so options from different backends can coexist in the same config without conflict. Example (custom backend): ```json { "mode": "fast", "enable_layout": true } ``` |

---

#### OcrQualityThresholds

Quality thresholds for OCR fallback decisions and pipeline quality gating.

All fields default to the values that match the previous hardcoded behavior,
so `OcrQualityThresholds.default()` preserves existing semantics exactly.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `min_total_non_whitespace` | `Integer` | `64` | Minimum total non-whitespace characters to consider text substantive. |
| `min_non_whitespace_per_page` | `Float` | `32` | Minimum non-whitespace characters per page on average. |
| `min_meaningful_word_len` | `Integer` | `4` | Minimum character count for a word to be "meaningful". |
| `min_meaningful_words` | `Integer` | `3` | Minimum count of meaningful words before text is accepted. |
| `min_alnum_ratio` | `Float` | `0.3` | Minimum alphanumeric ratio (non-whitespace chars that are alphanumeric). |
| `min_garbage_chars` | `Integer` | `5` | Minimum Unicode replacement characters (U+FFFD) to trigger OCR fallback. |
| `max_fragmented_word_ratio` | `Float` | `0.6` | Maximum fraction of short (1-2 char) words before text is considered fragmented. |
| `critical_fragmented_word_ratio` | `Float` | `0.8` | Critical fragmentation threshold — triggers OCR regardless of meaningful words. Normal English text has ~20-30% short words. 80%+ is definitive garbage. |
| `min_avg_word_length` | `Float` | `2` | Minimum average word length. Below this with enough words indicates garbled extraction. |
| `min_words_for_avg_length_check` | `Integer` | `50` | Minimum word count before average word length check applies. |
| `min_consecutive_repeat_ratio` | `Float` | `0.08` | Minimum consecutive word repetition ratio to detect column scrambling. |
| `min_words_for_repeat_check` | `Integer` | `50` | Minimum word count before consecutive repetition check is applied. |
| `substantive_min_chars` | `Integer` | `100` | Minimum character count for "substantive markdown" OCR skip gate. |
| `non_text_min_chars` | `Integer` | `20` | Minimum character count for "non-text content" OCR skip gate. |
| `alnum_ws_ratio_threshold` | `Float` | `0.4` | Alphanumeric+whitespace ratio threshold for skip decisions. |
| `pipeline_min_quality` | `Float` | `0.5` | Minimum quality score (0.0-1.0) for a pipeline stage result to be accepted. If the result from a backend scores below this, try the next backend. |

##### Methods

###### default()

**Signature:**

```ruby
def self.default()
```

**Example:**

```ruby
result = OcrQualityThresholds.default()
```

**Returns:** `OcrQualityThresholds`

---

#### OcrRotation

Rotation information for an OCR element.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `angle_degrees` | `Float` | — | Rotation angle in degrees (0, 90, 180, 270 for PaddleOCR). |
| `confidence` | `Float?` | `nil` | Confidence score for the rotation detection. |

---

#### OcrTable

Table detected via OCR.

Represents a table structure recognized during OCR processing.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `cells` | `Array<Array<String>>` | — | Table cells as a 2D vector (rows × columns) |
| `markdown` | `String` | — | Markdown representation of the table |
| `page_number` | `Integer` | — | Page number where the table was found (1-indexed) |
| `bounding_box` | `OcrTableBoundingBox?` | `/* serde(default) */` | Bounding box of the table in pixel coordinates (from OCR word positions). |

---

#### OcrTableBoundingBox

Bounding box for an OCR-detected table in pixel coordinates.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `left` | `Integer` | — | Left x-coordinate (pixels) |
| `top` | `Integer` | — | Top y-coordinate (pixels) |
| `right` | `Integer` | — | Right x-coordinate (pixels) |
| `bottom` | `Integer` | — | Bottom y-coordinate (pixels) |

---

#### OrientationResult

Document orientation detection result.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `degrees` | `Integer` | — | Detected orientation in degrees (0, 90, 180, or 270). |
| `confidence` | `Float` | — | Confidence score (0.0-1.0). |

---

#### PaddleOcrConfig

Configuration for PaddleOCR backend.

Configures PaddleOCR text detection and recognition with multi-language support.
Uses a builder pattern for convenient configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `language` | `String` | — | Language code (e.g., "en", "ch", "jpn", "kor", "deu", "fra") |
| `cache_dir` | `String?` | `nil` | Optional custom cache directory for model files |
| `use_angle_cls` | `Boolean` | — | Enable angle classification for rotated text (default: false). Can misfire on short text regions, rotating crops incorrectly before recognition. |
| `enable_table_detection` | `Boolean` | — | Enable table structure detection (default: false) |
| `det_db_thresh` | `Float` | — | Database threshold for text detection (default: 0.3) Range: 0.0-1.0, higher values require more confident detections |
| `det_db_box_thresh` | `Float` | — | Box threshold for text bounding box refinement (default: 0.5) Range: 0.0-1.0 |
| `det_db_unclip_ratio` | `Float` | — | Unclip ratio for expanding text bounding boxes (default: 1.6) Controls the expansion of detected text regions |
| `det_limit_side_len` | `Integer` | — | Maximum side length for detection image (default: 960) Larger images may be resized to this limit for faster inference |
| `rec_batch_num` | `Integer` | — | Batch size for recognition inference (default: 6) Number of text regions to process simultaneously |
| `padding` | `Integer` | — | Padding in pixels added around the image before detection (default: 10). Large values can include surrounding content like table gridlines. |
| `drop_score` | `Float` | — | Minimum recognition confidence score for text lines (default: 0.5). Text regions with recognition confidence below this threshold are discarded. Matches PaddleOCR Python's `drop_score` parameter. Range: 0.0-1.0 |
| `model_tier` | `String` | — | Model tier controlling detection/recognition model size and accuracy trade-off. - `"mobile"` (default): Lightweight models (~4.5MB detection, ~16.5MB recognition), fast download and inference - `"server"`: Large, high-accuracy models (~88MB detection, ~84MB recognition), best for GPU or complex documents |

##### Methods

###### with_cache_dir()

Sets a custom cache directory for model files.

**Signature:**

```ruby
def with_cache_dir(path)
```

**Example:**

```ruby
result = instance.with_cache_dir("value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `path` | `String` | Yes | Path to cache directory |

**Returns:** `PaddleOcrConfig`

###### with_table_detection()

Enables or disables table structure detection.

**Signature:**

```ruby
def with_table_detection(enable)
```

**Example:**

```ruby
result = instance.with_table_detection(true)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `enable` | `Boolean` | Yes | Whether to enable table detection |

**Returns:** `PaddleOcrConfig`

###### with_angle_cls()

Enables or disables angle classification for rotated text.

**Signature:**

```ruby
def with_angle_cls(enable)
```

**Example:**

```ruby
result = instance.with_angle_cls(true)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `enable` | `Boolean` | Yes | Whether to enable angle classification |

**Returns:** `PaddleOcrConfig`

###### with_det_db_thresh()

Sets the database threshold for text detection.

**Signature:**

```ruby
def with_det_db_thresh(threshold)
```

**Example:**

```ruby
result = instance.with_det_db_thresh(0.5)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `threshold` | `Float` | Yes | Detection threshold (0.0-1.0) |

**Returns:** `PaddleOcrConfig`

###### with_det_db_box_thresh()

Sets the box threshold for text bounding box refinement.

**Signature:**

```ruby
def with_det_db_box_thresh(threshold)
```

**Example:**

```ruby
result = instance.with_det_db_box_thresh(0.5)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `threshold` | `Float` | Yes | Box threshold (0.0-1.0) |

**Returns:** `PaddleOcrConfig`

###### with_det_db_unclip_ratio()

Sets the unclip ratio for expanding text bounding boxes.

**Signature:**

```ruby
def with_det_db_unclip_ratio(ratio)
```

**Example:**

```ruby
result = instance.with_det_db_unclip_ratio(0.5)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `ratio` | `Float` | Yes | Unclip ratio (typically 1.5-2.0) |

**Returns:** `PaddleOcrConfig`

###### with_det_limit_side_len()

Sets the maximum side length for detection images.

**Signature:**

```ruby
def with_det_limit_side_len(length)
```

**Example:**

```ruby
result = instance.with_det_limit_side_len(42)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `length` | `Integer` | Yes | Maximum side length in pixels |

**Returns:** `PaddleOcrConfig`

###### with_rec_batch_num()

Sets the batch size for recognition inference.

**Signature:**

```ruby
def with_rec_batch_num(batch_size)
```

**Example:**

```ruby
result = instance.with_rec_batch_num(42)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `batch_size` | `Integer` | Yes | Number of text regions to process simultaneously |

**Returns:** `PaddleOcrConfig`

###### with_drop_score()

Sets the minimum recognition confidence threshold.

**Signature:**

```ruby
def with_drop_score(score)
```

**Example:**

```ruby
result = instance.with_drop_score(0.5)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `score` | `Float` | Yes | Minimum confidence (0.0-1.0), text below this is dropped |

**Returns:** `PaddleOcrConfig`

###### with_padding()

Sets padding in pixels added around images before detection.

**Signature:**

```ruby
def with_padding(padding)
```

**Example:**

```ruby
result = instance.with_padding(42)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `padding` | `Integer` | Yes | Padding in pixels (0-100) |

**Returns:** `PaddleOcrConfig`

###### with_model_tier()

Sets the model tier controlling detection/recognition model size.

**Signature:**

```ruby
def with_model_tier(tier)
```

**Example:**

```ruby
result = instance.with_model_tier("value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `tier` | `String` | Yes | `"mobile"` (default, lightweight, faster) or `"server"` (high accuracy, GPU/complex documents) |

**Returns:** `PaddleOcrConfig`

###### default()

Creates a default configuration with English language support.

**Signature:**

```ruby
def self.default()
```

**Example:**

```ruby
result = PaddleOcrConfig.default()
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
| `byte_start` | `Integer` | — | Byte offset where this page starts in the content string (UTF-8 valid boundary, inclusive) |
| `byte_end` | `Integer` | — | Byte offset where this page ends in the content string (UTF-8 valid boundary, exclusive) |
| `page_number` | `Integer` | — | Page number (1-indexed) |

---

#### PageClassification

Classification result for a single page.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `page_number` | `Integer` | — | 1-indexed page number this classification belongs to. |
| `labels` | `Array<ClassificationLabel>` | — | Labels assigned to the page. Single-label classification yields exactly one entry; multi-label classification yields any subset of the configured label set. |

---

#### PageClassificationConfig

**Since:** `v5.0`

Configuration for the page-classification post-processor.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `prompt_template` | `String?` | `nil` | Minijinja prompt template. Receives `{{ labels }}` (joined list), `{{ page_text }}` and `{{ multi_label }}` variables. `nil` lets the backend pick a sensible default. |
| `labels` | `Array<String>` | — | The set of labels the classifier may emit. Must contain at least one entry. |
| `multi_label` | `Boolean` | `/* serde(default) */` | Allow multiple labels per page. Single-label mode returns at most one label. |
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
| `extract_pages` | `Boolean` | `false` | Extract pages as separate array (ExtractionResult.pages) |
| `insert_page_markers` | `Boolean` | `false` | Insert page markers in main content string |
| `marker_format` | `String` | `"<!-- PAGE {page_num} -->"` | Page marker format (use {page_num} placeholder) Default: "\n\n<!-- PAGE {page_num} -->\n\n" |

##### Methods

###### default()

**Signature:**

```ruby
def self.default()
```

**Example:**

```ruby
result = PageConfig.default()
```

**Returns:** `PageConfig`

---

#### PageContent

Content for a single page/slide.

When page extraction is enabled, documents are split into per-page content
with associated tables and images mapped to each page.

##### Performance

Uses shared tables and images for memory efficiency:

- `Array<Table>` enables zero-copy sharing of table data
- `Array<ExtractedImage>` enables zero-copy sharing of image data
- Maintains exact JSON compatibility via custom Serialize/Deserialize

This reduces memory overhead for documents with shared tables/images
by avoiding redundant copies during serialization.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `page_number` | `Integer` | — | Page number (1-indexed) |
| `content` | `String` | — | Text content for this page |
| `tables` | `Array<Table>` | `/* serde(default) */` | Tables found on this page (uses Arc for memory efficiency) Serializes as Array<Table> for JSON compatibility while maintaining shared in-memory ownership for zero-copy sharing. |
| `image_indices` | `Array<Integer>` | `/* serde(default) */` | Indices into `ExtractionResult.images` for images found on this page. Each value is a zero-based index into the top-level `images` collection. Only populated when `extract_images = true` in the extraction config. |
| `hierarchy` | `PageHierarchy?` | `nil` | Hierarchy information for the page (when hierarchy extraction is enabled) Contains text hierarchy levels (H1-H6) extracted from the page content. |
| `is_blank` | `Boolean?` | `nil` | Whether this page is blank (no meaningful text content) Determined during extraction based on text content analysis. A page is blank if it has fewer than 3 non-whitespace characters and contains no tables or images. |
| `layout_regions` | `Array<LayoutRegion>?` | `nil` | Layout detection regions for this page (when layout detection is enabled). Contains detected layout regions with class, confidence, bounding box, and area fraction. Only populated when layout detection is configured. |
| `speaker_notes` | `String?` | `nil` | Speaker notes for this slide (PPTX only). Contains the text from the slide's notes pane (`ppt/notesSlides/notesSlide{N}.xml`). Only populated when the source is a PPTX file and notes are present. |
| `section_name` | `String?` | `nil` | Section name this slide belongs to (PPTX only). PowerPoint sections group slides into logical chapters (`<p:sectionLst>` in `ppt/presentation.xml`). Only populated when the source is a PPTX file and the slide belongs to a named section. |
| `sheet_name` | `String?` | `nil` | Sheet name for this page (XLSX/ODS only). Each spreadsheet sheet maps to one `PageContent` entry. This field carries the sheet's display name as it appears in the workbook. `nil` for all non-spreadsheet formats and for sheets with an empty name. |

---

#### PageHierarchy

Page hierarchy structure containing heading levels and block information.

Used when PDF text hierarchy extraction is enabled. Contains hierarchical
blocks with heading levels (H1-H6) for semantic document structure.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `block_count` | `Integer` | — | Number of hierarchy blocks on this page |
| `blocks` | `Array<HierarchicalBlock>` | `/* serde(default) */` | Hierarchical blocks with heading levels |

---

#### PageInfo

Metadata for individual page/slide/sheet.

Captures per-page information including dimensions, content counts,
and visibility state (for presentations).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `number` | `Integer` | — | Page number (1-indexed) |
| `title` | `String?` | `nil` | Page title (usually for presentations) |
| `image_count` | `Integer?` | `nil` | Number of images on this page |
| `table_count` | `Integer?` | `nil` | Number of tables on this page |
| `hidden` | `Boolean?` | `nil` | Whether this page is hidden (e.g., in presentations) |
| `is_blank` | `Boolean?` | `nil` | Whether this page is blank (no meaningful text, no images, no tables) A page is considered blank if it has fewer than 3 non-whitespace characters and contains no tables or images. This is useful for filtering out empty pages in scanned documents or PDFs with blank separator pages. |
| `has_vector_graphics` | `Boolean` | `/* serde(default) */` | Whether this page contains non-trivial vector graphics (paths, shapes, curves) Indicates the presence of vector-drawn content such as charts, diagrams, or geometric shapes (e.g., from Adobe InDesign, LaTeX TikZ). These are invisible to `ExtractionResult.images` since they are not embedded as raster XObjects. Set to `true` when path count exceeds a heuristic threshold, signaling that downstream consumers may want to rasterize the page to capture this content. Only populated for PDFs; `nil` for other document types. |

---

#### PageRange

Page range for a chunk (0-indexed, inclusive).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `start` | `Integer` | — | Start page (0-indexed, inclusive). |
| `end` | `Integer` | — | End page (0-indexed, inclusive). |

##### Methods

###### page_count()

Get the number of pages in this range.

**Signature:**

```ruby
def page_count()
```

**Example:**

```ruby
result = instance.page_count()
```

**Returns:** `Integer`

---

#### PageSignals

Per-page signals extracted from PDF content.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `page_number` | `Integer` | — | 1-indexed page number. |
| `text_excerpt` | `String` | — | First ~500 characters of extracted text. |
| `starts_with_letterhead_like` | `Boolean` | — | `true` if page starts with letterhead-like content (ALL CAPS line in first 5 lines or a logo-image bbox at top). |
| `has_page_number_one_marker` | `Boolean` | — | `true` if text contains "Page 1" or "1 of N" pattern. |
| `has_signature_block` | `Boolean` | — | `true` if text contains signature indicators ("Sincerely", "Signed") or a signature image bbox. |
| `layout_text_density` | `Float` | — | Text density: characters per page area, normalised to `\[0.0, 1.0\]`. |

##### Methods

###### from_page_text()

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

```ruby
def self.from_page_text(page_number, text, layout_text_density)
```

**Example:**

```ruby
result = PageSignals.from_page_text(42, "value", 0.5)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `page_number` | `Integer` | Yes | The page number |
| `text` | `String` | Yes | The text |
| `layout_text_density` | `Float` | Yes | The layout text density |

**Returns:** `PageSignals`

---

#### PageStructure

Unified page structure for documents.

Supports different page types (PDF pages, PPTX slides, Excel sheets)
with character offset boundaries for chunk-to-page mapping.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `total_count` | `Integer` | — | Total number of pages/slides/sheets |
| `unit_type` | `PageUnitType` | — | Type of paginated unit |
| `boundaries` | `Array<PageBoundary>?` | `nil` | Character offset boundaries for each page Maps character ranges in the extracted content to page numbers. Used for chunk page range calculation. |
| `pages` | `Array<PageInfo>?` | `nil` | Detailed per-page metadata (optional, only when needed) |

---

#### PatternMatch

One detected PII span in the input text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `start` | `Integer` | — | Inclusive byte-offset start of the match in the source text. |
| `end` | `Integer` | — | Exclusive byte-offset end of the match. |
| `category` | `PiiCategory` | — | Category the match belongs to. |
| `text` | `String` | — | Matched substring (owned copy — pattern engine returns owned data so the caller can free the original text if needed before replacement). |

---

#### PdfAnnotation

A PDF annotation extracted from a document page.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `annotation_type` | `PdfAnnotationType` | — | The type of annotation. |
| `content` | `String?` | `nil` | Text content of the annotation (e.g., comment text, link URL). |
| `page_number` | `Integer` | — | Page number where the annotation appears (1-indexed). |
| `bounding_box` | `BoundingBox?` | `nil` | Bounding box of the annotation on the page. |

---

#### PdfConfig

PDF-specific configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `extract_images` | `Boolean` | `false` | Extract images from PDF |
| `extract_tables` | `Boolean` | `true` | Extract tables from PDF. When `true` (default), runs pdf_oxide's native grid detector and, if it finds nothing, falls back to the heuristic text-layer reconstruction in `pdf.oxide.table.extract_tables_heuristic`. Set to `false` to skip both passes — `tables` will then be empty in the result. |
| `passwords` | `Array<String>?` | `nil` | List of passwords to try when opening encrypted PDFs |
| `extract_metadata` | `Boolean` | `true` | Extract PDF metadata |
| `hierarchy` | `HierarchyConfig?` | `nil` | Hierarchy extraction configuration (None = hierarchy extraction disabled) |
| `extract_annotations` | `Boolean` | `false` | Extract PDF annotations (text notes, highlights, links, stamps). Default: false |
| `top_margin_fraction` | `Float?` | `nil` | Top margin fraction (0.0–1.0) of page height to exclude headers/running heads. Default: 0.06 (6%) |
| `bottom_margin_fraction` | `Float?` | `nil` | Bottom margin fraction (0.0–1.0) of page height to exclude footers/page numbers. Default: 0.05 (5%) |
| `allow_single_column_tables` | `Boolean` | `false` | Allow single-column pseudo tables in extraction results. By default, tables with fewer than 2 columns (layout-guided) or 3 columns (heuristic) are rejected. When `true`, the minimum column count is relaxed to 1, allowing single-column structured data (glossaries, itemized lists) to be emitted as tables. Other quality filters (density, sparsity, prose detection) still apply. |
| `ocr_inline_images` | `Boolean` | `false` | Perform OCR on inline images extracted from PDF pages and attach the recognized text to each `ExtractedImage.ocr_result`. Requires Tesseract to be available; if `ExtractionConfig.ocr` is `nil` the extractor falls back to `TesseractConfig.default()`. Per-image failures degrade gracefully (the image is returned without OCR text rather than failing the whole extraction). Default: `false`. |
| `extract_form_fields` | `Boolean` | `true` | Extract AcroForm and XFA form fields into `ExtractionResult.form_fields`. When `true` (default), reads the document's interactive form structure (field names, types, values, widget geometry). Cheap and strictly additive — non-form PDFs simply yield an empty list. Set to `false` to skip the form pass entirely. |
| `reading_order` | `Boolean` | `false` | Reorder extracted text by layout-detected reading order. When `true`, projects text spans onto layout-detected regions, performs column detection, and emits spans in natural reading order (important for multi-column academic PDFs). Requires the `layout-detection` feature; has no effect without it. Defaults to `false`. |

##### Methods

###### default()

**Signature:**

```ruby
def self.default()
```

**Example:**

```ruby
result = PdfConfig.default()
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
| `name` | `String` | — | Partial field name (the leaf name within the field hierarchy). |
| `full_name` | `String` | — | Fully-qualified field name (dotted path from the form root). |
| `field_type` | `FormFieldType` | — | Classified field type. |
| `value` | `String?` | `/* serde(default) */` | Current field value, if any. |
| `default_value` | `String?` | `/* serde(default) */` | Default field value, if any. |
| `flags` | `Integer` | `/* serde(default) */` | Raw field-flags bitmask (read-only, required, multiline, …). |
| `page` | `Integer?` | `/* serde(default) */` | 1-indexed page the field's widget appears on. Currently always `nil` for AcroForm fields; page assignment is a deferred enhancement requiring spatial analysis of widget annotations per page. |
| `bbox` | `BoundingBox?` | `/* serde(default) */` | Widget bounding box on its page, if known. |
| `max_length` | `Integer?` | `/* serde(default) */` | Maximum input length for text fields, if specified. |
| `tooltip` | `String?` | `/* serde(default) */` | Tooltip / alternate field description, if present. |

---

#### PdfMetadata

PDF-specific metadata.

Contains metadata fields specific to PDF documents that are not in the common
`Metadata` structure. Common fields like title, authors, keywords, and dates
are at the `Metadata` level.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `pdf_version` | `String?` | `nil` | PDF version (e.g., "1.7", "2.0") |
| `producer` | `String?` | `nil` | PDF producer (application that created the PDF) |
| `is_encrypted` | `Boolean?` | `nil` | Whether the PDF is encrypted/password-protected |
| `width` | `Integer?` | `nil` | First page width in points (1/72 inch) |
| `height` | `Integer?` | `nil` | First page height in points (1/72 inch) |
| `page_count` | `Integer?` | `nil` | Total number of pages in the PDF document |

---

#### Plugin

Base trait that all plugins must implement.

This trait provides common functionality for plugin lifecycle management,
identification, and metadata.

##### Thread Safety

All plugins must be `Send + Sync` to support concurrent usage across threads.

##### Methods

###### name()

Returns the unique name/identifier for this plugin.

The name should be:

- Unique across all plugins
- Lowercase with hyphens (e.g., "my-custom-plugin")
- URL-safe characters only

**Signature:**

```ruby
def name()
```

**Example:**

```ruby
result = instance.name()
```

**Returns:** `String`

###### version()

Returns the semantic version of this plugin.

Should follow semver format: `MAJOR.MINOR.PATCH`

Defaults to the xberg crate version.

**Signature:**

```ruby
def version()
```

**Example:**

```ruby
result = instance.version()
```

**Returns:** `String`

###### initialize()

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

```ruby
def initialize()
```

**Example:**

```ruby
instance.initialize()
```

**Returns:** No return value.

**Errors:** Raises `Error`.

###### shutdown()

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

```ruby
def shutdown()
```

**Example:**

```ruby
instance.shutdown()
```

**Returns:** No return value.

**Errors:** Raises `Error`.

###### description()

Optional plugin description for debugging and logging.

Defaults to empty string if not overridden.

**Signature:**

```ruby
def description()
```

**Example:**

```ruby
result = instance.description()
```

**Returns:** `String`

###### author()

Optional plugin author information.

Defaults to empty string if not overridden.

**Signature:**

```ruby
def author()
```

**Example:**

```ruby
result = instance.author()
```

**Returns:** `String`

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

##### Performance

This signature avoids unnecessary cloning of large extraction results by
taking a mutable reference instead of ownership. Processors modify the
result in place.

##### Example - Language Detection

##### Example - Text Cleaning

**Signature:**

```ruby
def process(result, config)
```

**Example:**

```ruby
instance.process(ExtractionResult.new, ExtractionConfig.new)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `result` | `ExtractionResult` | Yes | Mutable reference to the extraction result to process |
| `config` | `ExtractionConfig` | Yes | Extraction configuration |

**Returns:** No return value.

**Errors:** Raises `Error`.

###### processing_stage()

Get the processing stage for this post-processor.

Determines when this processor runs in the pipeline.

**Returns:**

The `ProcessingStage` (Early, Middle, or Late).

**Signature:**

```ruby
def processing_stage()
```

**Example:**

```ruby
result = instance.processing_stage()
```

**Returns:** `ProcessingStage`

###### should_process()

Optional: Check if this processor should run for a given result.

Allows conditional processing based on MIME type, metadata, or content.
Defaults to `true` (always run).

**Returns:**

`true` if the processor should run, `false` to skip.

**Signature:**

```ruby
def should_process(result, config)
```

**Example:**

```ruby
result = instance.should_process(ExtractionResult.new, ExtractionConfig.new)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `result` | `ExtractionResult` | Yes | The extraction result |
| `config` | `ExtractionConfig` | Yes | The extraction config |

**Returns:** `Boolean`

###### estimated_duration_ms()

Optional: Estimate processing time in milliseconds.

Used for logging and debugging. Defaults to 0 (unknown).

**Returns:**

Estimated processing time in milliseconds.

**Signature:**

```ruby
def estimated_duration_ms(result)
```

**Example:**

```ruby
result = instance.estimated_duration_ms(ExtractionResult.new)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `result` | `ExtractionResult` | Yes | The extraction result |

**Returns:** `Integer`

###### priority()

Execution priority within the processing stage.

Higher values run first within the same `ProcessingStage`. Defaults to 50.
Use 0-49 for fallback processors, 50 for normal processors, and 51-255
for high-priority processors that should run early in their stage.

**Signature:**

```ruby
def priority()
```

**Example:**

```ruby
result = instance.priority()
```

**Returns:** `Integer`

---

#### PostProcessorConfig

Post-processor configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `Boolean` | `true` | Enable post-processors |
| `enabled_processors` | `Array<String>?` | `nil` | Whitelist of processor names to run (None = all enabled) |
| `disabled_processors` | `Array<String>?` | `nil` | Blacklist of processor names to skip (None = none disabled) |
| `enabled_set` | `Array<String>?` | `nil` | Pre-computed AHashSet for O(1) enabled processor lookup |
| `disabled_set` | `Array<String>?` | `nil` | Pre-computed AHashSet for O(1) disabled processor lookup |

##### Methods

###### default()

**Signature:**

```ruby
def self.default()
```

**Example:**

```ruby
result = PostProcessorConfig.default()
```

**Returns:** `PostProcessorConfig`

---

#### PptxAppProperties

Application properties from docProps/app.xml for PPTX

Contains PowerPoint-specific document metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `application` | `String?` | `nil` | Application name (e.g., "Microsoft Office PowerPoint") |
| `app_version` | `String?` | `nil` | Application version |
| `total_time` | `Integer?` | `nil` | Total editing time in minutes |
| `company` | `String?` | `nil` | Company name |
| `doc_security` | `Integer?` | `nil` | Document security level |
| `scale_crop` | `Boolean?` | `nil` | Scale crop flag |
| `links_up_to_date` | `Boolean?` | `nil` | Links up to date flag |
| `shared_doc` | `Boolean?` | `nil` | Shared document flag |
| `hyperlinks_changed` | `Boolean?` | `nil` | Hyperlinks changed flag |
| `slides` | `Integer?` | `nil` | Number of slides |
| `notes` | `Integer?` | `nil` | Number of notes |
| `hidden_slides` | `Integer?` | `nil` | Number of hidden slides |
| `multimedia_clips` | `Integer?` | `nil` | Number of multimedia clips |
| `presentation_format` | `String?` | `nil` | Presentation format (e.g., "Widescreen", "Standard") |
| `slide_titles` | `Array<String>` | `\[\]` | Slide titles |

---

#### PptxExtractionResult

PowerPoint (PPTX) extraction result.

Contains extracted slide content, metadata, and embedded images/tables.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String` | — | Extracted text content from all slides |
| `metadata` | `PptxMetadata` | — | Presentation metadata |
| `slide_count` | `Integer` | — | Total number of slides |
| `image_count` | `Integer` | — | Total number of embedded images |
| `table_count` | `Integer` | — | Total number of tables |
| `images` | `Array<ExtractedImage>` | — | Extracted images from the presentation |
| `page_structure` | `PageStructure?` | `nil` | Slide structure with boundaries (when page tracking is enabled) |
| `page_contents` | `Array<PageContent>?` | `nil` | Per-slide content (when page tracking is enabled) |
| `document` | `DocumentStructure?` | `nil` | Structured document representation |
| `office_metadata` | `Hash{String=>String}` | `/* serde(default) */` | Office metadata extracted from docProps/core.xml and docProps/app.xml. Contains keys like "title", "author", "created_by", "subject", "keywords", "modified_by", "created_at", "modified_at", etc. |
| `revisions` | `Array<DocumentRevision>?` | `/* serde(default) */` | Slide comments as revisions. Each `<p:cm>` element in `ppt/comments/comment{N}.xml` becomes a `DocumentRevision { kind: Comment }` with author (resolved from `ppt/commentAuthors.xml`), ISO-8601 timestamp, and `RevisionAnchor.Slide { index }`. `nil` when no comment XML parts exist. |

---

#### PptxMetadata

PowerPoint presentation metadata.

Extracted from PPTX files containing slide counts and presentation details.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `slide_count` | `Integer` | — | Total number of slides in the presentation |
| `slide_names` | `Array<String>` | `\[\]` | Names of slides (if available) |
| `image_count` | `Integer?` | `nil` | Number of embedded images |
| `table_count` | `Integer?` | `nil` | Number of tables |

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
| `id` | `String` | — | Stable, URL-safe preset identifier (lowercase snake_case). |
| `version` | `String` | — | Monotonic version string (e.g. `v1`). |
| `schema_name` | `String` | — | Human-readable schema name forwarded to the LLM as the response/tool name. |
| `description` | `String` | — | One-line preset description shown in the registry UI. |
| `category` | `PresetCategory` | — | Top-level category for grouping in the playground. |
| `tags` | `Array<String>` | `/* serde(default) */` | Free-form tags used for search/filtering. May be empty. |
| `schema` | `Object` | — | JSON Schema (Draft 2020-12) describing the structured output shape. |
| `system_prompt` | `String` | — | Instruction primer sent to the model. |
| `context_template` | `String?` | `/* serde(default) */` | Optional mustache-style template merged with caller-supplied context. |
| `merge_mode` | `MergeMode` | — | Strategy for merging per-batch outputs across paginated calls. |
| `preferred_call_mode` | `CallMode` | — | Default call mode suggested for this preset; heuristics may override. |
| `emit_citations` | `Boolean` | — | When true, the prompt asks the model to wrap each field as `{value, page, bbox, confidence}` for downstream citation overlays. |
| `sample` | `PresetSample?` | `/* serde(default) */` | Optional bundled sample (input file + reference output) for preview. |
| `fingerprint` | `String` | `/* serde(default) */` | Stable sha256 fingerprint of the canonical preset file contents. Populated at registry load — not present in the on-disk JSON files. Used as a cache-invalidation token by the worker pipeline. |

---

#### PresetSample

Pointer to a sample input + its reference output bundled with the preset.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `input_path` | `String` | — | Path to the sample input file, relative to the preset directory. |
| `output_path` | `String` | — | Path to the reference structured output, relative to the preset directory. |

---

#### PresetSummary

Lightweight projection of `Preset` used by the registry list endpoint
(omits the full schema and prompt to keep the payload small).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String` | — | Preset identifier matching `Preset.id`. |
| `version` | `String` | — | Preset version matching `Preset.version`. |
| `schema_name` | `String` | — | Schema name matching `Preset.schema_name`. |
| `description` | `String` | — | One-line preset description. |
| `category` | `PresetCategory` | — | Top-level category. |
| `tags` | `Array<String>` | — | Free-form tags. |
| `preferred_call_mode` | `CallMode` | — | Default call mode. |
| `emit_citations` | `Boolean` | — | Whether the preset prompts the model for citations. |
| `fingerprint` | `String` | — | Stable fingerprint matching `Preset.fingerprint`. |

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
| `message_count` | `Integer` | — | Total number of email messages found in the PST archive. |

---

#### QrBoundingBox

Pixel-space bounding box of a QR code inside its source image.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `x` | `Integer` | — | Horizontal pixel offset of the bounding box top-left corner. |
| `y` | `Integer` | — | Vertical pixel offset of the bounding box top-left corner. |
| `width` | `Integer` | — | Width of the bounding box in pixels. |
| `height` | `Integer` | — | Height of the bounding box in pixels. |

---

#### QrCode

One QR code decoded from an extracted image.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `payload` | `String` | — | Decoded payload (text, URL, vCard string, …). |
| `confidence` | `Float?` | `nil` | Detector-reported confidence in `\[0.0, 1.0\]`. `nil` when the decoder does not expose confidence (the default `rqrr` backend always reports `Some` because successful decode implies high confidence). |
| `bbox` | `QrBoundingBox?` | `nil` | Bounding box of the QR code inside the source image, in pixel coordinates (`x`, `y` of the top-left corner; `width`, `height` of the rectangle). `nil` if the decoder did not report a bounding box. |

---

#### RakeParams

RAKE-specific parameters.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `min_word_length` | `Integer` | `1` | Minimum word length to consider (default: 1). |
| `max_words_per_phrase` | `Integer` | `3` | Maximum words in a keyword phrase (default: 3). |

##### Methods

###### default()

**Signature:**

```ruby
def self.default()
```

**Example:**

```ruby
result = RakeParams.default()
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
| `detection_bbox` | `BBox` | — | Detection bbox that this table corresponds to (for matching). |
| `cells` | `Array<Array<String>>` | — | Table cells as a 2D vector (rows × columns). |
| `markdown` | `String` | — | Rendered markdown table. |

---

#### RedactionConfig

**Since:** `v5.0`

Configuration for the redaction post-processor.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `categories` | `Array<PiiCategory>` | `\[\]` | Categories to redact. Empty means "every category supported by the engine." |
| `strategy` | `RedactionStrategy` | `:mask` | Strategy applied to every match. |
| `ner` | `NerConfig?` | `nil` | Optional NER backend — required to redact PERSON / ORGANIZATION / LOCATION categories (the pure-Rust pattern engine only covers regex-detectable PII). |
| `preserve_offsets` | `Boolean` | `true` | When `true`, chunk byte ranges are kept consistent with the rewritten content by adjusting `byte_start` / `byte_end` after replacement. When `false`, chunk byte ranges still refer to the *original* content offsets — useful when downstream consumers want to map findings back to the original document. |
| `custom_terms` | `Array<RedactionTerm>` | `\[\]` | Arbitrary user-supplied literal terms to redact. Each term is treated as a regex hit against the document, surfacing as `PiiCategory.Custom(label)` in `RedactionFinding` where `label` is the per-term label (defaulting to the literal value itself). Case-insensitive by default; set `RedactionTerm.case_sensitive` for exact match. Use this when you need to redact tenant-specific tokens (employee IDs, project codes, internal product names) without writing a custom plugin. |
| `custom_patterns` | `Array<RedactionPattern>` | `\[\]` | Arbitrary user-supplied regex patterns to redact. Same surfacing semantics as `custom_terms`: each hit becomes a `PiiCategory.Custom(label)` finding. Patterns are validated at config-construction time via `RedactionConfig.validate`. |

##### Methods

###### default()

**Signature:**

```ruby
def self.default()
```

**Example:**

```ruby
result = RedactionConfig.default()
```

**Returns:** `RedactionConfig`

###### validate()

Validate user-supplied terms and patterns at config-construction time.

Compiles every `RedactionPattern.pattern` (with the case-insensitive
inline flag where applicable) and returns the first compilation error so
the caller can reject the config before the redaction pipeline runs.
Pure terms (regex-escaped) cannot fail to compile, but the function
still rejects empty values to avoid degenerate zero-length matches.

**Signature:**

```ruby
def validate()
```

**Example:**

```ruby
instance.validate()
```

**Returns:** No return value.

**Errors:** Raises `Error`.

---

#### RedactionFinding

One redaction event: which span was rewritten, why, and with what.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `start` | `Integer` | — | Byte-offset start in the original (pre-redaction) `ExtractionResult.content`. |
| `end` | `Integer` | — | Byte-offset end (exclusive) in the original `ExtractionResult.content`. |
| `category` | `PiiCategory` | — | PII category that fired this redaction. |
| `strategy` | `RedactionStrategy` | — | Strategy applied to this finding (mask, hash, token-replace, drop). |
| `replacement_token` | `String` | — | String that replaced the original mention. Always present; for `Drop` the replacement is the empty string. |

---

#### RedactionPattern

One user-supplied regex pattern to redact.

The pattern is compiled with the Rust `regex` crate (no look-around). Case
sensitivity is encoded in the pattern via the `(?i)` inline flag when
`Self.case_sensitive` is `false`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `label` | `String` | — | Custom category label surfaced in `RedactionFinding.category`. |
| `pattern` | `String` | — | Regex pattern (Rust `regex` crate dialect — no look-around). |
| `case_sensitive` | `Boolean` | `serde(default = "default_case_sensitive")` | When `true`, match case-sensitively; otherwise prepend `(?i)` to the regex. |

##### Methods

###### labeled()

Build a pattern with the given label (case-insensitive by default).

**Signature:**

```ruby
def self.labeled(label, pattern)
```

**Example:**

```ruby
result = RedactionPattern.labeled("value", "value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `label` | `String` | Yes | The label |
| `pattern` | `String` | Yes | The pattern |

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
| `findings` | `Array<RedactionFinding>` | — | Individual redaction findings in original-source byte order. |
| `total_redacted` | `Integer` | — | Total number of redactions applied across the document. |

---

#### RedactionTerm

One user-supplied literal term to redact.

Matched as a regex-escaped substring (so callers do not need to escape
metacharacters themselves). Case-insensitive by default — set
`Self.case_sensitive` to `true` for exact byte-match semantics.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `label` | `String` | — | Custom category label surfaced in `RedactionFinding.category`. |
| `value` | `String` | — | Literal value to match. Regex metacharacters are escaped automatically. |
| `case_sensitive` | `Boolean` | `serde(default = "default_case_sensitive")` | When `true`, match the value as-is; otherwise match ASCII-case-insensitively. |

##### Methods

###### literal()

Build a term whose label is the literal value itself (case-insensitive).

**Signature:**

```ruby
def self.literal(value)
```

**Example:**

```ruby
result = RedactionTerm.literal("value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `value` | `String` | Yes | The value |

**Returns:** `RedactionTerm`

###### labeled()

Build a term with a custom label.

**Signature:**

```ruby
def self.labeled(label, value)
```

**Example:**

```ruby
result = RedactionTerm.labeled("value", "value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `label` | `String` | Yes | The label |
| `value` | `String` | Yes | The value |

**Returns:** `RedactionTerm`

---

#### Registry

Sorted map of preset id → `Preset`.

##### Methods

###### load_embedded()

Build the registry from preset files embedded at compile time under
`src/presets/library/`. Validates every file against the meta-schema.

**Signature:**

```ruby
def self.load_embedded()
```

**Example:**

```ruby
result = Registry.load_embedded()
```

**Returns:** `Registry`

**Errors:** Raises `LoadError`.

###### global()

Return the global registry, loading it on first access.

**Panics:**

Panics if any embedded preset is malformed. The build-time validation
test ensures this cannot happen for the embedded presets; a panic here
indicates a build artifact problem, not a runtime error.

**Signature:**

```ruby
def self.global()
```

**Example:**

```ruby
result = Registry.global()
```

**Returns:** `Registry`

###### get()

Look up a preset by its identifier.

**Signature:**

```ruby
def get(id)
```

**Example:**

```ruby
result = instance.get("value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `id` | `String` | Yes | The id |

**Returns:** `Preset?`

###### summaries()

Materialize a `PresetSummary` list for the public registry endpoint.

**Signature:**

```ruby
def summaries()
```

**Example:**

```ruby
result = instance.summaries()
```

**Returns:** `Array<PresetSummary>`

###### len()

Number of presets currently loaded.

**Signature:**

```ruby
def len()
```

**Example:**

```ruby
result = instance.len()
```

**Returns:** `Integer`

###### is_empty()

Whether the registry contains zero presets.

**Signature:**

```ruby
def is_empty()
```

**Example:**

```ruby
result = instance.is_empty()
```

**Returns:** `Boolean`

###### sample_bytes()

Read raw sample bytes for `<preset_id>` from
`library/<id>/samples/<name>`. Returns `nil` when the file is absent.

**Signature:**

```ruby
def sample_bytes(preset_id, name)
```

**Example:**

```ruby
result = instance.sample_bytes("value", "value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `preset_id` | `String` | Yes | The preset id |
| `name` | `String` | Yes | The name |

**Returns:** `String?`

###### extend_from_dir()

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

```ruby
def extend_from_dir(dir)
```

**Example:**

```ruby
result = instance.extend_from_dir("value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `dir` | `String` | Yes | The dir |

**Returns:** `Integer`

**Errors:** Raises `LoadError`.

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

###### render()

Render an `InternalDocument` to the output format.

**Returns:**

The rendered output as a string.

**Errors:**

Returns an error if rendering fails.

**Signature:**

```ruby
def render(doc)
```

**Example:**

```ruby
result = instance.render(InternalDocument.new)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `doc` | `InternalDocument` | Yes | The internal document to render |

**Returns:** `String`

**Errors:** Raises `Error`.

---

#### RerankedDocument

A single document returned by the reranker, with its position in the input and score.

`index` maps back to the caller's original document list, so metadata arrays
(e.g. IDs, paths) can be reordered without passing them through the reranker.

Since v5.0.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `index` | `Integer` | — | Position of this document in the original input `documents` slice. |
| `score` | `Float` | — | Relevance score in `\[0, 1\]`. Higher means more relevant to the query. |
| `document` | `String` | — | The document text. |

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

###### rerank()

Score a list of documents against a query.

Returns one raw logit per document in the same order as the input.
The dispatcher applies sigmoid to convert to `[0, 1]` scores.

**Errors:**

Implementations should return `Plugin` for
backend-specific failures. The dispatcher validates the returned length
against `documents.len()` before sorting.

**Signature:**

```ruby
def rerank(query, documents)
```

**Example:**

```ruby
result = instance.rerank("value", [])
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `query` | `String` | Yes | The query |
| `documents` | `Array<String>` | Yes | The documents |

**Returns:** `Array<Float>`

**Errors:** Raises `Error`.

---

#### RerankerConfig

Configuration for the reranking pipeline.

Controls which model to use, how many results to return, and download/cache
behavior for local ONNX models.

Since v5.0.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `RerankerModelType` | `:preset` | The reranker model to use (defaults to "balanced" preset if not specified). |
| `top_k` | `Integer?` | `nil` | Return at most this many documents. `nil` returns all. Applied after sorting by score, so the highest-scoring documents are kept. |
| `batch_size` | `Integer` | `32` | Batch size for local ONNX cross-encoder inference. |
| `show_download_progress` | `Boolean` | `false` | Show model download progress (local ONNX path only). |
| `cache_dir` | `String?` | `nil` | Custom cache directory for model files. Defaults to `~/.cache/xberg/rerankers/` if not specified. |
| `acceleration` | `AccelerationConfig?` | `nil` | Hardware acceleration for the reranker ONNX model. Controls which execution provider (CPU, CUDA, CoreML, TensorRT) is used for local inference. Defaults to `nil` (auto-select per platform). |
| `max_rerank_duration_secs` | `Integer?` | `nil` | Maximum wall-clock duration (in seconds) for a single `rerank()` call when using `RerankerModelType.Plugin`. Applies only to the in-process plugin path — protects against hung host-language backends. On timeout, the dispatcher returns `Plugin` instead of blocking forever. `nil` disables the timeout. The default (60 seconds) is conservative for common in-process inference; increase for large document sets on slow hardware. |

##### Methods

###### default()

**Signature:**

```ruby
def self.default()
```

**Example:**

```ruby
result = RerankerConfig.default()
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
| `name` | `String` | — | Short identifier (catalog name, e.g. `"bge-reranker-base"`). |
| `model_repo` | `String` | — | HuggingFace repository name for the model. |
| `model_file` | `String` | — | Path to the ONNX model file within the repo. |
| `additional_files` | `Array<String>` | `/* serde(default) */` | Sibling files that must be downloaded alongside `model_file`. Empty for most presets. Used by repos that split the weight blob — e.g. `rozgo/bge-reranker-v2-m3` ships the model in `model.onnx` plus a co-located `model.onnx.data` payload. |
| `max_length` | `Integer` | — | Maximum token sequence length the model supports. |
| `description` | `String` | — | Human-readable description of the preset's intended use case. |

---

#### ResolvedPreset

A preset merged with caller-supplied overrides (custom schema, prompt suffix,
context map). Output is what the pipeline orchestrator consumes.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | `String` | — | Source preset identifier. |
| `version` | `String` | — | Source preset version. |
| `fingerprint` | `String` | — | Fingerprint of the source preset file, used as a cache token. |
| `schema_name` | `String` | — | Schema name forwarded to the LLM. |
| `schema` | `Object` | — | Effective JSON Schema (caller override or the preset's own). |
| `system_prompt` | `String` | — | System prompt with rendered context appended. |
| `merge_mode` | `MergeMode` | — | Merge strategy for paginated outputs. |
| `preferred_call_mode` | `CallMode` | — | Preferred call mode. |
| `emit_citations` | `Boolean` | — | Whether the prompt asks for per-field citations. |

---

#### RevisionDelta

The content changes that make up a single revision.

For insertions and deletions the `content` field carries the added/removed
lines as `DiffLine.Added` / `DiffLine.Removed` entries. For format
changes, `content` is empty — the property diff is left as a TODO for a
later enrichment pass.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `Array<DiffLine>` | `\[\]` | Line-level content changes for this revision. |
| `table_changes` | `Array<CellChange>` | `\[\]` | Cell-level table changes for this revision. |

---

#### SecurityLimits

Configuration for security limits across extractors.

All limits are intentionally conservative to prevent DoS attacks
while still supporting legitimate documents.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_archive_size` | `Integer` | `524288000` | Maximum uncompressed size for archives (500 MB) |
| `max_compression_ratio` | `Integer` | `100` | Maximum compression ratio before flagging as potential bomb (100:1) |
| `max_files_in_archive` | `Integer` | `10000` | Maximum number of files in archive (10,000) |
| `max_nesting_depth` | `Integer` | `1024` | Maximum nesting depth for structures (100) |
| `max_entity_length` | `Integer` | `1048576` | Maximum length of any single XML entity / attribute / token (1 MiB). This is a per-token cap, NOT a total cap — billion-laughs class attacks where a single entity expands to hundreds of MB are caught here, while normal long text content (a paragraph, a CDATA block) is caught by `max_content_size` instead. |
| `max_content_size` | `Integer` | `104857600` | Maximum string growth per document (100 MB) |
| `max_iterations` | `Integer` | `10000000` | Maximum iterations per operation |
| `max_xml_depth` | `Integer` | `1024` | Maximum XML depth (100 levels) |
| `max_table_cells` | `Integer` | `100000` | Maximum cells per table (100,000) |

##### Methods

###### default()

**Signature:**

```ruby
def self.default()
```

**Example:**

```ruby
result = SecurityLimits.default()
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
| `host` | `String` | — | Server host address (e.g., "127.0.0.1", "0.0.0.0") |
| `port` | `Integer` | — | Server port number |
| `cors_origins` | `Array<String>` | `\[\]` | CORS allowed origins. Empty vector means allow all origins. If this is an empty listtor, the server will accept requests from any origin. If populated with specific origins (e.g., `"<https://example.com"`>), only those origins will be allowed. |
| `max_request_body_bytes` | `Integer` | — | Maximum size of request body in bytes (default: 100 MB) |
| `max_multipart_field_bytes` | `Integer` | — | Maximum size of multipart fields in bytes (default: 100 MB) |

##### Methods

###### default()

**Signature:**

```ruby
def self.default()
```

**Example:**

```ruby
result = ServerConfig.default()
```

**Returns:** `ServerConfig`

###### listen_addr()

Get the server listen address (host:port).

**Signature:**

```ruby
def listen_addr()
```

**Example:**

```ruby
result = instance.listen_addr()
```

**Returns:** `String`

###### cors_allows_all()

Check if CORS allows all origins.

Returns `true` if the `cors_origins` vector is empty, meaning all origins
are allowed. Returns `false` if specific origins are configured.

**Signature:**

```ruby
def cors_allows_all()
```

**Example:**

```ruby
result = instance.cors_allows_all()
```

**Returns:** `Boolean`

###### is_origin_allowed()

Check if a given origin is allowed by CORS configuration.

Returns `true` if:

- CORS allows all origins (empty origins list), or
- The given origin is in the allowed origins list

**Signature:**

```ruby
def is_origin_allowed(origin)
```

**Example:**

```ruby
result = instance.is_origin_allowed("value")
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `origin` | `String` | Yes | The origin to check (e.g., "<https://example.com">) |

**Returns:** `Boolean`

###### max_request_body_mb()

Get maximum request body size in megabytes (rounded up).

**Signature:**

```ruby
def max_request_body_mb()
```

**Example:**

```ruby
result = instance.max_request_body_mb()
```

**Returns:** `Integer`

###### max_multipart_field_mb()

Get maximum multipart field size in megabytes (rounded up).

**Signature:**

```ruby
def max_multipart_field_mb()
```

**Example:**

```ruby
result = instance.max_multipart_field_mb()
```

**Returns:** `Integer`

---

#### StructuredData

Structured data (Schema.org, microdata, RDFa) block.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `data_type` | `StructuredDataType` | — | Type of structured data |
| `raw_json` | `String` | — | Raw JSON string representation |
| `schema_type` | `String?` | `nil` | Schema type if detectable (e.g., "Article", "Event", "Product") |

---

#### StructuredDataResult

Result of parsing a structured data file (JSON, JSONL, YAML, or TOML).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String` | — | The extracted text content, formatted for readability. |
| `format` | `String` | — | The source format identifier (e.g. `"json"`, `"yaml"`, `"toml"`). |
| `metadata` | `Hash{String=>String}` | — | Key-value metadata extracted from recognized text fields. |
| `text_fields` | `Array<String>` | — | JSON paths of fields that were classified as text-bearing. |

---

#### StructuredExtractionConfig

Configuration for LLM-based structured data extraction.

Sends extracted document content to a VLM with a JSON schema,
returning structured data that conforms to the schema.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `schema` | `Object` | — | JSON Schema defining the desired output structure. |
| `schema_name` | `String` | `serde(default = "default_schema_name")` | Schema name passed to the LLM's structured output mode. |
| `schema_description` | `String?` | `/* serde(default) */` | Optional schema description for the LLM. |
| `strict` | `Boolean` | `/* serde(default) */` | Enable strict mode — output must exactly match the schema. |
| `prompt` | `String?` | `/* serde(default) */` | Custom Jinja2 extraction prompt template. When `nil`, a default template is used. Available template variables: - `{{ content }}` — The extracted document text. - `{{ schema }}` — The JSON schema as a formatted string. - `{{ schema_name }}` — The schema name. - `{{ schema_description }}` — The schema description (may be empty). |
| `llm` | `LlmConfig` | — | LLM configuration for the extraction. |

---

#### StructuredInput

Signals consumed by the call-mode heuristic.

All fields derive from a prior xberg extraction — no double-work.
This is a plain DTO; it intentionally has no dependency on internal
xberg extraction types so it can be constructed from any source.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `mime_type` | `String` | — | MIME type, canonicalised to lowercase by the caller. |
| `page_count` | `Integer` | — | Number of pages in the document. |
| `text_coverage` | `Float` | — | Fraction of pages with a real text layer (0.0..=1.0). |
| `avg_chars_per_page` | `Float` | — | Average extracted characters per page. |
| `embedded_image_count` | `Integer` | — | Count of embedded images (figures, photos, signatures) discovered. |
| `user_force_vision` | `Boolean` | — | When `true`, promote the result to at least `StructuredCallMode.TextPlusVision`. |

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
| `scan_max_coverage` | `Float` | `0.1` | PDFs with `text_coverage` strictly below this are treated as scanned. **Conservative default: 0.10** — deployments override via their own config after measuring their document corpus. |
| `digital_min_coverage` | `Float` | `0.9` | PDFs with `text_coverage` at or above this AND zero embedded images route to `StructuredCallMode.TextOnly`. **Conservative default: 0.90** — deployments override via their own config after measuring their document corpus. |
| `docx_text_min_density` | `Float` | `200` | DOCX / HTML / text documents with `avg_chars_per_page` above this route to `StructuredCallMode.TextOnly`. **Conservative default: 200.0** — deployments override via their own config after measuring their document corpus. |
| `enable_vision_fallback` | `Boolean` | `false` | When `true`, emit `StructuredCallMode.TextOnlyWithVisionFallback` instead of `StructuredCallMode.TextOnly` so the orchestrator can escalate to vision on low confidence. **Conservative default: `false`** — must be explicitly enabled per deployment after bench validation; deployments override via their own config. |

##### Methods

###### default()

**Signature:**

```ruby
def self.default()
```

**Example:**

```ruby
result = StructuredThresholds.default()
```

**Returns:** `StructuredThresholds`

---

#### SummarizationConfig

**Since:** `v5.0`

Configuration for the summarisation post-processor.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `strategy` | `SummaryStrategy` | `:extractive` | Summarisation strategy. |
| `max_tokens` | `Integer?` | `nil` | Maximum summary length in tokens. `nil` lets the backend pick a default. |
| `llm` | `LlmConfig?` | `nil` | LLM configuration for the abstractive backend. Ignored when `strategy = Extractive`. Required when `strategy = Abstractive`. |

---

#### SupportedFormat

A supported document format entry.

Represents a file extension and its corresponding MIME type that Xberg can process.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `extension` | `String` | — | File extension (without leading dot), e.g., "pdf", "docx" |
| `mime_type` | `String` | — | MIME type string, e.g., "application/pdf" |

---

#### SvgOptions

SVG-specific configuration for the image-encode pipeline.

Applies when the source image is SVG or when the output format is set to
`ImageOutputFormat.Svg`.  Available when the `svg` feature is active.

Used via `ImageExtractionConfig.svg`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `sanitize` | `Boolean` | `true` | Run SVG bytes through `usvg` sanitization (strips external `href` attributes, JavaScript event handlers, and `foreignObject` elements) even when the output format is `Native`.  Defaults to `true`. |
| `render_dpi` | `Float` | `96` | Target DPI when rasterizing SVG to a pixel-based format (PNG, JPEG, WebP, HEIF).  The tree's viewBox is scaled by `render_dpi / 96.0` before the pixel buffer is allocated.  Defaults to `96.0` (1× CSS pixel density). |

##### Methods

###### default()

**Signature:**

```ruby
def self.default()
```

**Example:**

```ruby
result = SvgOptions.default()
```

**Returns:** `SvgOptions`

---

#### Table

Extracted table structure.

Represents a table detected and extracted from a document (PDF, image, etc.).
Tables are converted to both structured cell data and Markdown format.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `cells` | `Array<Array<String>>` | `\[\]` | Table cells as a 2D vector (rows × columns) |
| `markdown` | `String` | — | Markdown representation of the table |
| `page_number` | `Integer` | — | Page number where the table was found (1-indexed) |
| `bounding_box` | `BoundingBox?` | `nil` | Bounding box of the table on the page (PDF coordinates: x0=left, y0=bottom, x1=right, y1=top). Only populated for PDF-extracted tables when position data is available. |

---

#### TableCell

Individual table cell with content and optional styling.

Future extension point for rich table support with cell-level metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String` | — | Cell content as text |
| `row_span` | `Integer` | — | Row span (number of rows this cell spans) |
| `col_span` | `Integer` | — | Column span (number of columns this cell spans) |
| `is_header` | `Boolean` | — | Whether this is a header cell |

---

#### TableDiff

Cell-level changes for a pair of tables that share the same index.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `from_index` | `Integer` | — | Zero-based index of the table in both `a.tables` and `b.tables`. |
| `to_index` | `Integer` | — | Zero-based index in `b.tables` (equal to `from_index` for same-dimension tables). |
| `cell_changes` | `Array<CellChange>` | — | Cell-level changes within the table. |

---

#### TableGrid

Structured table grid with cell-level metadata.

Stores row/column dimensions and a flat list of cells with position info.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `rows` | `Integer` | — | Number of rows in the table. |
| `cols` | `Integer` | — | Number of columns in the table. |
| `cells` | `Array<GridCell>` | `\[\]` | All cells in row-major order. |

---

#### TesseractConfig

Tesseract OCR configuration.

Provides fine-grained control over Tesseract OCR engine parameters.
Most users can use the defaults, but these settings allow optimization
for specific document types (invoices, handwriting, etc.).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `language` | `Array<String>` | `\[\]` | Language code(s) for OCR recognition. Accepts either a single language code ("eng") or a list (\["eng", "deu"\]). For Tesseract backend, languages are joined with "+". |
| `psm` | `Integer` | `3` | Page Segmentation Mode (0-13). Common values: - 3: Fully automatic page segmentation (native default) - 6: Assume a single uniform block of text (WASM default — avoids layout-analysis hang) - 11: Sparse text with no particular order |
| `output_format` | `String` | `"markdown"` | Output format ("text" or "markdown") |
| `oem` | `Integer` | `3` | OCR Engine Mode (0-3). - 0: Legacy engine only - 1: Neural nets (LSTM) only (usually best) - 2: Legacy + LSTM - 3: Default (based on what's available) |
| `min_confidence` | `Float` | `0` | Minimum confidence threshold (0.0-100.0). Words with confidence below this threshold may be rejected or flagged. |
| `preprocessing` | `ImagePreprocessingConfig?` | `nil` | Image preprocessing configuration. Controls how images are preprocessed before OCR. Can significantly improve quality for scanned documents or low-quality images. |
| `enable_table_detection` | `Boolean` | `true` | Enable automatic table detection and reconstruction |
| `table_min_confidence` | `Float` | `0` | Minimum confidence threshold for table detection (0.0-1.0) |
| `table_column_threshold` | `Integer` | `50` | Column threshold for table detection (pixels) |
| `table_row_threshold_ratio` | `Float` | `0.5` | Row threshold ratio for table detection (0.0-1.0) |
| `use_cache` | `Boolean` | `true` | Enable OCR result caching |
| `classify_use_pre_adapted_templates` | `Boolean` | `true` | Use pre-adapted templates for character classification |
| `language_model_ngram_on` | `Boolean` | `false` | Enable N-gram language model |
| `tessedit_dont_blkrej_good_wds` | `Boolean` | `true` | Don't reject good words during block-level processing |
| `tessedit_dont_rowrej_good_wds` | `Boolean` | `true` | Don't reject good words during row-level processing |
| `tessedit_enable_dict_correction` | `Boolean` | `true` | Enable dictionary correction |
| `tessedit_char_whitelist` | `String` | `""` | Whitelist of allowed characters (empty = all allowed) |
| `tessedit_char_blacklist` | `String` | `""` | Blacklist of forbidden characters (empty = none forbidden) |
| `tessedit_use_primary_params_model` | `Boolean` | `true` | Use primary language params model |
| `textord_space_size_is_variable` | `Boolean` | `true` | Variable-width space detection |
| `thresholding_method` | `Boolean` | `false` | Use adaptive thresholding method |

##### Methods

###### default()

**Signature:**

```ruby
def self.default()
```

**Example:**

```ruby
result = TesseractConfig.default()
```

**Returns:** `TesseractConfig`

---

#### TextAnnotation

Inline text annotation — byte-range based formatting and links.

Annotations reference byte offsets into the node's text content,
enabling precise identification of formatted regions.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `start` | `Integer` | — | Start byte offset in the node's text content (inclusive). |
| `end` | `Integer` | — | End byte offset in the node's text content (exclusive). |
| `kind` | `AnnotationKind` | — | Annotation type. |

---

#### TextExtractionResult

Plain text and Markdown extraction result.

Contains the extracted text along with statistics and,
for Markdown files, structural elements like headers and links.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String` | — | Extracted text content |
| `line_count` | `Integer` | — | Number of lines |
| `word_count` | `Integer` | — | Number of words |
| `character_count` | `Integer` | — | Number of characters |
| `headers` | `Array<String>?` | `nil` | Markdown headers (text only, Markdown files only) |

---

#### TextMetadata

Text/Markdown metadata.

Extracted from plain text and Markdown files. Includes word counts and,
for Markdown, structural elements like headers and links.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `line_count` | `Integer` | — | Number of lines in the document |
| `word_count` | `Integer` | — | Number of words |
| `character_count` | `Integer` | — | Number of characters |
| `headers` | `Array<String>?` | `\[\]` | Markdown headers (headings text only, for Markdown files) |

---

#### TokenCounter

Per-category running counter for `RedactionStrategy.TokenReplace`.

##### Methods

###### new()

Create a fresh counter with no previous state.

**Signature:**

```ruby
def self.new()
```

**Example:**

```ruby
result = TokenCounter.new()
```

**Returns:** `TokenCounter`

---

#### TokenReductionConfig

Configuration for the token-reduction pipeline.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `level` | `ReductionLevel` | `:moderate` | Reduction intensity level. |
| `language_hint` | `String?` | `nil` | ISO 639-1 language code hint for stopword selection (e.g. `"en"`, `"de"`). |
| `preserve_markdown` | `Boolean` | `false` | Preserve Markdown formatting tokens during reduction. |
| `preserve_code` | `Boolean` | `true` | Preserve code block contents unchanged. |
| `semantic_threshold` | `Float` | `0.3` | Cosine similarity threshold below which sentences are considered dissimilar. |
| `enable_parallel` | `Boolean` | `true` | Use Rayon parallel iterators for multi-core processing. |
| `use_simd` | `Boolean` | `true` | Use SIMD-optimized text scanning where available. |
| `custom_stopwords` | `Hash{String=>Array<String>}?` | `nil` | Per-language custom stopword lists (`language_code → stopword_list`). |
| `preserve_patterns` | `Array<String>` | `\[\]` | Regex patterns whose matched text is always preserved unchanged. |
| `target_reduction` | `Float?` | `nil` | Target fraction of text to retain (0.0–1.0); `nil` = no fixed target. |
| `enable_semantic_clustering` | `Boolean` | `false` | Group semantically similar sentences and emit only one per cluster. |

##### Methods

###### default()

**Signature:**

```ruby
def self.default()
```

**Example:**

```ruby
result = TokenReductionConfig.default()
```

**Returns:** `TokenReductionConfig`

---

#### TokenReductionOptions

Token reduction configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `mode` | `String` | — | Reduction mode: "off", "light", "moderate", "aggressive", "maximum" |
| `preserve_important_words` | `Boolean` | `true` | Preserve important words (capitalized, technical terms) |

##### Methods

###### default()

**Signature:**

```ruby
def self.default()
```

**Example:**

```ruby
result = TokenReductionOptions.default()
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
| `enabled` | `Boolean` | `true` | Master switch. When false the block is ignored and audio files fall back to the normal "unsupported format" path. |
| `model` | `WhisperModel` | `:tiny` | Whisper model size to use. Smaller = faster + lower memory. `tiny` is the pragmatic default for first-time users and CI. |
| `language` | `String?` | `nil` | Optional language hint (ISO-639-1 code, e.g. "en", "de"). When `nil` (default), the current engine falls back to English. For deterministic production output, always set this explicitly. |
| `timestamps` | `Boolean` | `false` | Whether to request segment-level timestamps. Accepted for forward compatibility. The current engine always uses `<\|notimestamps\|>` and does not emit segment metadata yet. |
| `max_duration_ms` | `Integer?` | `nil` | Hard safety limit on input duration (milliseconds). Files longer than this are rejected after decode, before model work. Default: 30 minutes. Set to `nil` to disable (not recommended for untrusted input). |
| `max_bytes` | `Integer?` | `nil` | Hard safety limit on input size (bytes). Default: 512 MiB. Protects against pathological or malicious uploads. |
| `timeout_ms` | `Integer?` | `nil` | Wall-clock timeout for the entire transcription operation (ms). Default: 10 minutes. Reserved for timeout enforcement; the current extractor does not enforce this field yet. |
| `model_cache_dir` | `String?` | `nil` | Override the directory used for Whisper model cache. When `nil`, uses the centralized resolver: `XBERG_CACHE_DIR/whisper` or the platform default (`~/.cache/xberg/whisper` on Linux, etc.). |
| `allow_network` | `Boolean` | `true` | Allow network access to download models from Hugging Face Hub. When `false`, only previously cached models may be used. Useful for air-gapped or fully offline deployments. |
| `verify_hash` | `Boolean` | `true` | Request SHA256 verification of downloaded model files. Reserved for the checksum table follow-up. The current resolver logs a warning and treats this as a no-op. |

##### Methods

###### default()

**Signature:**

```ruby
def self.default()
```

**Example:**

```ruby
result = TranscriptionConfig.default()
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
| `target_lang` | `String` | — | BCP-47 language tag the translation was produced into (e.g. `"de"`, `"fr-CA"`). |
| `source_lang` | `String?` | `nil` | BCP-47 source language. `nil` when the translation backend was asked to detect. |
| `content` | `String` | — | Translated plain-text body. Matches the shape of `ExtractionResult.content`. |
| `formatted_content` | `String?` | `nil` | Translated markup body (Markdown / HTML / etc.) when `preserve_markup` was enabled on the config. `nil` otherwise. |

---

#### TranslationConfig

**Since:** `v5.0`

Configuration for the translation post-processor.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `target_lang` | `String` | — | BCP-47 language tag for the target language (e.g. `"de"`, `"fr-CA"`). |
| `source_lang` | `String?` | `nil` | Optional explicit source language. `nil` asks the backend to auto-detect. |
| `preserve_markup` | `Boolean` | `/* serde(default) */` | Translate the formatted (Markdown/HTML) rendition alongside plain text when `formatted_content` is present. |
| `llm` | `LlmConfig` | — | LLM configuration used for translation. |

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
| `enabled` | `Boolean` | `true` | Enable code intelligence processing (default: true). When `false`, tree-sitter analysis is completely skipped even if the config section is present. |
| `cache_dir` | `String?` | `nil` | Custom cache directory for downloaded grammars. When `nil`, uses the default: `~/.cache/tree-sitter-language-pack/v{version}/libs/`. |
| `languages` | `Array<String>?` | `nil` | Languages to pre-download on init (e.g., `\["python", "rust"\]`). |
| `groups` | `Array<String>?` | `nil` | Language groups to pre-download (e.g., `\["web", "systems", "scripting"\]`). |
| `process` | `TreeSitterProcessConfig` | — | Processing options for code analysis. |

##### Methods

###### default()

**Signature:**

```ruby
def self.default()
```

**Example:**

```ruby
result = TreeSitterConfig.default()
```

**Returns:** `TreeSitterConfig`

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
| `chunk_max_size` | `Integer?` | `nil` | Maximum chunk size in bytes. `nil` disables chunking. |
| `content_mode` | `CodeContentMode` | `:chunks` | Content rendering mode for code extraction. |

##### Methods

###### default()

**Signature:**

```ruby
def self.default()
```

**Example:**

```ruby
result = TreeSitterProcessConfig.default()
```

**Returns:** `TreeSitterProcessConfig`

---

#### UrlExtractionConfig

URL ingestion and crawl configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `mode` | `UrlExtractionMode` | `:auto` | URL extraction mode. |
| `document_url_pattern` | `String?` | `nil` | Optional regex filter for document-discovered URLs. |
| `max_document_urls_per_result` | `Integer?` | `nil` | Maximum URLs to follow per extraction result. |
| `max_total_urls` | `Integer?` | `nil` | Maximum URLs followed across the whole extraction call. |
| `allow_local_file_inputs` | `Boolean` | `true` | Allow bare local filesystem path inputs. |
| `allow_file_uris` | `Boolean` | `true` | Allow local `file://` URI inputs. |

##### Methods

###### default()

**Signature:**

```ruby
def self.default()
```

**Example:**

```ruby
result = UrlExtractionConfig.default()
```

**Returns:** `UrlExtractionConfig`

---

#### UserChunkConfig

User-provided chunk configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `page_ranges` | `Array<PageRange>?` | `\[\]` | User-specified page ranges (overrides automatic chunking). |
| `pages_per_chunk` | `Integer?` | `nil` | User-specified pages per chunk (overrides automatic calculation). |
| `force_chunking` | `Boolean` | — | Force chunking even for small documents. |
| `disable_chunking` | `Boolean` | — | Disable chunking even for large documents. |

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

###### validate()

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

```ruby
def validate(result, config)
```

**Example:**

```ruby
instance.validate(ExtractionResult.new, ExtractionConfig.new)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `result` | `ExtractionResult` | Yes | The extraction result to validate |
| `config` | `ExtractionConfig` | Yes | Extraction configuration |

**Returns:** No return value.

**Errors:** Raises `Error`.

###### should_validate()

Optional: Check if this validator should run for a given result.

Allows conditional validation based on MIME type, metadata, or content.
Defaults to `true` (always run).

**Returns:**

`true` if the validator should run, `false` to skip.

**Signature:**

```ruby
def should_validate(result, config)
```

**Example:**

```ruby
result = instance.should_validate(ExtractionResult.new, ExtractionConfig.new)
```

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `result` | `ExtractionResult` | Yes | The extraction result |
| `config` | `ExtractionConfig` | Yes | The extraction config |

**Returns:** `Boolean`

###### priority()

Optional: Get the validation priority.

Higher priority validators run first. Useful for ordering validation checks
(e.g., run cheap validations before expensive ones).

Default priority is 50.

**Returns:**

Priority value (higher = runs earlier).

**Signature:**

```ruby
def priority()
```

**Example:**

```ruby
result = instance.priority()
```

**Returns:** `Integer`

---

#### XlsxAppProperties

Application properties from docProps/app.xml for XLSX

Contains Excel-specific document metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `application` | `String?` | `nil` | Application name (e.g., "Microsoft Excel") |
| `app_version` | `String?` | `nil` | Application version |
| `doc_security` | `Integer?` | `nil` | Document security level |
| `scale_crop` | `Boolean?` | `nil` | Scale crop flag |
| `links_up_to_date` | `Boolean?` | `nil` | Links up to date flag |
| `shared_doc` | `Boolean?` | `nil` | Shared document flag |
| `hyperlinks_changed` | `Boolean?` | `nil` | Hyperlinks changed flag |
| `company` | `String?` | `nil` | Company name |
| `worksheet_names` | `Array<String>` | `\[\]` | Worksheet names |

---

#### XmlExtractionResult

XML extraction result.

Contains extracted text content from XML files along with
structural statistics about the XML document.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `String` | — | Extracted text content (XML structure filtered out) |
| `element_count` | `Integer` | — | Total number of XML elements processed |
| `unique_elements` | `Array<String>` | — | List of unique element names found (sorted) |

---

#### XmlMetadata

XML metadata extracted during XML parsing.

Provides statistics about XML document structure.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `element_count` | `Integer` | — | Total number of XML elements processed |
| `unique_elements` | `Array<String>` | `\[\]` | List of unique element tag names (sorted) |

---

#### YakeParams

YAKE-specific parameters.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `window_size` | `Integer` | `2` | Window size for co-occurrence analysis (default: 2). Controls the context window for computing co-occurrence statistics. |

##### Methods

###### default()

**Signature:**

```ruby
def self.default()
```

**Example:**

```ruby
result = YakeParams.default()
```

**Returns:** `YakeParams`

---

#### YearRange

Year range for bibliographic metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `min` | `Integer?` | `nil` | Earliest (minimum) year in the range. |
| `max` | `Integer?` | `nil` | Latest (maximum) year in the range. |
| `years` | `Array<Integer>` | `/* serde(default) */` | All individual years present in the collection. |

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
| `native` | Preserve whatever format the extractor produced (default). No re-encode pass is performed. `ExtractedImage.format` reflects the source format: JPEG for embedded PDF images, PNG for rasterised content, or the native container format from office documents. |
| `png` | Re-encode all extracted images as PNG (lossless). |
| `jpeg` | Re-encode all extracted images as JPEG at the given quality level. `quality` must be in `1..=100`. Values outside this range are clamped and a warning is emitted. Higher values produce larger files with less artefacting; 85 is a reasonable default. — Fields: `quality`: `Integer` |
| `webp` | Re-encode all extracted images as WebP at the given quality level. `quality` must be in `1..=100`. Values outside this range are clamped and a warning is emitted. 80 is a reasonable default. — Fields: `quality`: `Integer` |
| `heif` | Re-encode all extracted images as HEIF/HEIC at the given quality level. Requires the `heic` feature. `quality` must be in `1..=100`. Values outside this range are clamped and a warning is emitted. 80 is a reasonable default. — Fields: `quality`: `Integer` |
| `svg` | Output pure-vector SVG. Lossless. Raster sources are not re-encoded (a warning is emitted and the image bytes are left untouched). When the source is already SVG, the bytes are passed through the `usvg` sanitizer (strips external hrefs, JS event handlers, and `foreignObject` elements) when `SvgOptions.sanitize` is `true`. Requires the `svg` feature. |

---

#### ExtractInputKind

Source kind for `ExtractInput`.

| Value | Description |
|-------|-------------|
| `bytes` | Raw in-memory bytes. |
| `uri` | A filesystem path, `file://` URI, or HTTP(S) URL. |

---

#### UrlExtractionMode

URL extraction mode.

| Value | Description |
|-------|-------------|
| `auto` | Classify HTTP(S) resources after fetch. |
| `document` | Treat the URI as a single remote document/page. |
| `crawl` | Crawl from the seed URI and extract discovered pages/documents. |

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
| `custom` | Custom renderer registered via the RendererRegistry. The string is the renderer name (e.g., "docx", "latex"). — Fields: `0`: `String` |

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

#### CallMode

How a structured-extraction preset is dispatched to the model.

This is the preset-facing call mode (the `preferred_call_mode` field of a
`Preset`). The richer runtime decision enum used by the
structured pipeline — which adds `Skip` and `TextOnlyWithVisionFallback` —
lives in `crate.heuristics.structured.StructuredCallMode`; this 3-variant
type is the stable, serializable surface presets and bindings depend on.

| Value | Description |
|-------|-------------|
| `text_only` | Use the extracted text only. |
| `vision_only` | Use rasterized page images only. |
| `text_plus_vision` | Provide both extracted text and page images to the model. |

---

#### MergeMode

How partial results from multiple model calls (e.g. per page batch) are combined.

Canonical home for the merge strategy referenced by presets and by the
structured pipeline's post-processing. There is intentionally only one merge
type across the crate — do not introduce a second.

| Value | Description |
|-------|-------------|
| `object_merge` | Deep-merge JSON objects field by field (later calls fill missing fields). |
| `array_concat` | Concatenate top-level arrays across calls. |
| `object_first` | Keep the first non-empty result; ignore subsequent calls. |

---

#### NerBackendKind

NER backend selector.

| Value | Description |
|-------|-------------|
| `onnx` | `xberg-gliner` ONNX inference. Requires `ner-onnx` feature. Models download lazily from `xberg-io/gliner-models`. |
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
| `on_low_quality` | Try the classical OCR backend first. If the quality score is below `quality_threshold`, send the page to the VLM. `quality_threshold` is in the `\[0.0, 1.0\]` range produced by `calculate_quality_score`. A value of `0.5` is a reasonable starting point; calibrate with the Stage 0 benchmark harness. — Fields: `quality_threshold`: `Float` |
| `always` | Skip the classical OCR backend entirely. Every page is sent to the VLM. |

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
| `split` | Split tables at row boundaries (default). Continuation chunks have no header. |
| `repeat_header` | Prepend the table header to every chunk that continues a split table. |

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
| `tokenizer` | Size measured in tokens from a HuggingFace tokenizer. — Fields: `model`: `String`, `cache_dir`: `String` |

---

#### EmbeddingModelType

Embedding model types supported by Xberg.

| Value | Description |
|-------|-------------|
| `preset` | Use a preset model configuration (recommended) — Fields: `name`: `String` |
| `custom` | Use a custom ONNX model from HuggingFace — Fields: `model_id`: `String`, `dimensions`: `Integer` |
| `llm` | Provider-hosted embedding model via liter-llm. Uses the model specified in the nested `LlmConfig` (e.g., `"openai/text-embedding-3-small"`). — Fields: `llm`: `LlmConfig` |
| `plugin` | In-process embedding backend registered via the plugin system. The caller registers an `EmbeddingBackend` once (e.g. a wrapper around an already-loaded `llama-cpp-python`, `sentence-transformers`, or tuned ONNX model), then references it by name in config. Xberg calls back into the registered backend during chunking and standalone embed requests — no HuggingFace download, no ONNX Runtime requirement, no HTTP sidecar. When this variant is selected, only the following `EmbeddingConfig` fields apply: `normalize` (post-call L2 normalization) and `max_embed_duration_secs` (dispatcher timeout). Model-loading fields (`batch_size`, `cache_dir`, `show_download_progress`, `acceleration`) are ignored — the host owns the model lifecycle. Semantic chunking falls back to `ChunkingConfig.max_characters` when this variant is used, since there is no preset to look a chunk-size ceiling up against — size your context window via `max_characters` directly. See `register_embedding_backend`. — Fields: `name`: `String` |

---

#### RerankerModelType

Reranker model types supported by Xberg.

Since v5.0.

| Value | Description |
|-------|-------------|
| `preset` | Use a preset cross-encoder model (recommended). — Fields: `name`: `String` |
| `custom` | Use a custom ONNX cross-encoder from HuggingFace. — Fields: `model_id`: `String`, `model_file`: `String`, `additional_files`: `Array<String>`, `max_length`: `Integer` |
| `llm` | Provider-hosted reranker via liter-llm (e.g. Cohere, Jina, Voyage). The model in the nested `LlmConfig` must be a rerank-capable model ID (e.g. `"cohere/rerank-english-v3.0"`). — Fields: `llm`: `LlmConfig` |
| `plugin` | In-process reranker registered via the plugin system. The caller registers a `RerankerBackend` once (e.g. a wrapper around a `sentence-transformers` cross-encoder or a provider client), then references it by name in config. Xberg calls back into the registered backend — no HuggingFace download, no ONNX Runtime requirement. When this variant is selected, only `max_rerank_duration_secs` applies. Model-loading fields (`batch_size`, `cache_dir`, `show_download_progress`, `acceleration`) are ignored — the host owns the model lifecycle. See `register_reranker_backend`. — Fields: `name`: `String` |

---

#### WhisperModel

Supported Whisper model sizes.

These map to published ONNX exports on Hugging Face (onnx-community or
similar orgs). The actual filenames and repos are resolved inside the
transcription engine.

| Value | Description |
|-------|-------------|
| `tiny` | Smallest, fastest, lowest quality. Good default for development and CI. |
| `base` | Reasonable quality/speed tradeoff. |
| `small` | Better accuracy with higher memory and cache use. |
| `medium` | High quality; slower and more memory-intensive. |
| `large_v3` | Best quality (large-v3). Use only when latency and memory use are acceptable. |

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
| `candle` | Candle-based VLM OCR (TrOCR, PaddleOCR-VL). |
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
| `title` | Document title. — Fields: `text`: `String` |
| `heading` | Section heading with level (1-6). — Fields: `level`: `Integer`, `text`: `String` |
| `paragraph` | Body text paragraph. — Fields: `text`: `String` |
| `list` | List container — children are `ListItem` nodes. — Fields: `ordered`: `Boolean` |
| `list_item` | Individual list item. — Fields: `text`: `String` |
| `table` | Table with structured cell grid. — Fields: `grid`: `TableGrid` |
| `image` | Image reference. — Fields: `description`: `String`, `image_index`: `Integer`, `src`: `String` |
| `code` | Code block. — Fields: `text`: `String`, `language`: `String` |
| `quote` | Block quote — container, children carry the quoted content. |
| `formula` | Mathematical formula / equation. — Fields: `text`: `String` |
| `footnote` | Footnote reference content. — Fields: `text`: `String` |
| `group` | Logical grouping container (section, key-value area). `heading_level` + `heading_text` capture the section heading directly rather than relying on a first-child positional convention. — Fields: `label`: `String`, `heading_level`: `Integer`, `heading_text`: `String` |
| `page_break` | Page break marker. |
| `slide` | Presentation slide container — children are the slide's content nodes. — Fields: `number`: `Integer`, `title`: `String` |
| `definition_list` | Definition list container — children are `DefinitionItem` nodes. |
| `definition_item` | Individual definition list entry with term and definition. — Fields: `term`: `String`, `definition`: `String` |
| `citation` | Citation or bibliographic reference. — Fields: `key`: `String`, `text`: `String` |
| `admonition` | Admonition / callout container (note, warning, tip, etc.). Children carry the admonition body content. — Fields: `kind`: `String`, `title`: `String` |
| `raw_block` | Raw block preserved verbatim from the source format. Used for content that cannot be mapped to a semantic node type (e.g. JSX in MDX, raw LaTeX in markdown, embedded HTML). — Fields: `format`: `String`, `content`: `String` |
| `metadata_block` | Structured metadata block (email headers, YAML frontmatter, etc.). |

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
| `link` | Hyperlink annotation. — Fields: `url`: `String`, `title`: `String` |
| `highlight` | Highlighted text (PDF highlights, HTML `<mark>`). |
| `color` | Text color (CSS-compatible value, e.g. "#ff0000", "red"). — Fields: `value`: `String` |
| `font_size` | Font size with units (e.g. "12pt", "1.2em", "16px"). — Fields: `value`: `String` |
| `custom` | Extensible annotation for format-specific styling. — Fields: `name`: `String`, `value`: `String` |

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
| `custom` | A caller-supplied custom category label. — Fields: `0`: `String` |

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

#### FormFieldType

Kind of a PDF form field.

Mirrors `pdf_oxide`'s widget field taxonomy without leaking the upstream
type across the binding surface.

| Value | Description |
|-------|-------------|
| `text` | Single- or multi-line text input. |
| `checkbox` | Checkbox (on/off toggle). |
| `radio` | Radio-button group member. |
| `choice` | Choice field (dropdown or list box). |
| `signature` | Digital-signature field. |
| `button` | Push button. |
| `unknown` | Field type that could not be classified. |

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
| `rectangle` | Axis-aligned bounding box (typical for Tesseract output). — Fields: `left`: `Integer`, `top`: `Integer`, `width`: `Integer`, `height`: `Integer` |
| `quadrilateral` | 4-point quadrilateral for rotated/skewed text (PaddleOCR). Points are in clockwise order starting from top-left: `\[top_left, top_right, bottom_right, bottom_left\]` |

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
| `mask` | Replace the matched span with a fixed mask token (default `"\[REDACTED\]"`). |
| `hash` | Replace with a SHA-256 hash of the original value (truncated to 16 hex chars). Lets downstream consumers do equality joins without recovering the source. |
| `token_replace` | Replace with a per-category running token (`"\[PERSON_1\]"`, `"\[PERSON_2\]"`, …) so the same person referenced twice gets the same token within the document. |
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
| `custom` | Caller-supplied custom category (e.g. internal employee IDs). Surfaced by the redaction engine when a hit comes from `RedactionConfig.custom_terms` or `RedactionConfig.custom_patterns`. The string is the label passed alongside the term/pattern. Use those fields rather than constructing `Custom` directly via the `categories` filter — the pattern engine cannot detect arbitrary text from a category name alone. — Fields: `0`: `String` |

---

#### DiffLine

A single line in a unified-diff hunk.

Defined here (rather than only in `crate.diff`) so `RevisionDelta` can
reference it unconditionally, without requiring the `diff` Cargo feature.
`crate.diff` re-exports this type verbatim.

| Value | Description |
|-------|-------------|
| `context` | Unchanged context line. — Fields: `0`: `String` |
| `added` | Line added in the "after" version. — Fields: `0`: `String` |
| `removed` | Line removed from the "before" version. — Fields: `0`: `String` |

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
| `paragraph` | Body paragraph, identified by its zero-based index in the document flow. — Fields: `index`: `Integer` |
| `table_cell` | Cell inside a table. — Fields: `row`: `Integer`, `col`: `Integer`, `table_index`: `Integer` |
| `page` | Page, identified by its zero-based index. — Fields: `index`: `Integer` |
| `slide` | Presentation slide, identified by its zero-based index. — Fields: `index`: `Integer` |
| `sheet` | Spreadsheet cell or range, identified by sheet index and optional name. — Fields: `index`: `Integer`, `name`: `String` |

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
| `pending` | Job submitted; processing has not yet started or is in progress. |
| `completed` | Processing completed successfully. — Fields: `result`: `EnrichResult` |
| `failed` | Processing failed. — Fields: `error`: `String` |

---

#### SchemaCompliance

Schema-validation outcome surfaced as one of three buckets.

Fold into the combined confidence score without leaking internal validation
error types.

| Value | Description |
|-------|-------------|
| `all_valid` | Every batch validated against the schema. |
| `partial_valid` | At least one batch validated; at least one did not. |
| `all_invalid` | No batch validated. |

---

#### ChunkingDecision

The chunking decision made by the analyzer.

| Value | Description |
|-------|-------------|
| `no_chunking` | Process without chunking (small file, text layer detected, etc.) — Fields: `reason`: `NoChunkingReason` |
| `chunk` | Chunk according to plan. — Fields: `0`: `ChunkPlan` |
| `use_overrides` | Use user-provided chunk overrides. — Fields: `user_chunks`: `Array<PageRange>` |

---

#### NoChunkingReason

Reason for not chunking a document.

| Value | Description |
|-------|-------------|
| `small_file` | File is below size threshold. — Fields: `size_bytes`: `Integer`, `threshold_bytes`: `Integer` |
| `few_pages` | Document has fewer pages than threshold. — Fields: `page_count`: `Integer`, `threshold`: `Integer` |
| `text_layer_detected` | PDF has substantial text layer (OCR not needed). — Fields: `text_coverage`: `Float`, `avg_chars_per_page`: `Integer` |
| `format_not_chunkable` | Document format does not support chunking. — Fields: `mime_type`: `String` |
| `chunking_disabled` | Chunking is disabled by configuration. |
| `fast_text_extraction` | Force OCR is disabled and text extraction is fast. |

---

#### ChunkingReason

Reason for chunking a document.

| Value | Description |
|-------|-------------|
| `large_file` | File exceeds size threshold. — Fields: `size_bytes`: `Integer`, `threshold_bytes`: `Integer` |
| `many_pages` | Document has many pages. — Fields: `page_count`: `Integer`, `threshold`: `Integer` |
| `ocr_required` | PDF requires OCR and is large. — Fields: `page_count`: `Integer`, `force_ocr`: `Boolean` |
| `large_and_many_pages` | Both size and page count exceed thresholds. — Fields: `size_bytes`: `Integer`, `page_count`: `Integer` |

---

#### BoundaryReason

Reason for boundary detection.

| Value | Description |
|-------|-------------|
| `start` | Start of PDF. |
| `page_one_marker` | Page-one marker ("Page 1", "1 of N") detected. |
| `letterhead_reset` | Letterhead reset after signature block. |
| `density_shift` | Text density shift with low bigram overlap. |
| `end` | End of PDF. |

---

#### StructuredCallMode

Outcome of the structured-extraction call-mode heuristic.

**Distinct from `crate.core.config.CallMode`** which has three variants
and governs extraction-engine behaviour.  This enum governs whether and how
an already-extracted document is sent to an LLM structured-extraction
pipeline.

| Value | Description |
|-------|-------------|
| `skip` | Document is unsupported or not worth invoking the pipeline. |
| `text_only` | Send extracted text only; no vision model call. |
| `vision_only` | Send page rasters only; no extracted text payload. |
| `text_plus_vision` | Fuse extracted text with page rasters in a single multimodal call. |
| `text_only_with_vision_fallback` | Try text-only first; escalate to vision on low confidence score. |

---

#### PresetCategory

High-level category used to group presets in the registry UI.

| Value | Description |
|-------|-------------|
| `finance` | Invoices, receipts, statements, purchase orders, W-9. |
| `identity` | Passports, drivers licenses, insurance cards. |
| `legal` | Contracts, NDAs, agreements. |
| `logistics` | Bills of lading, customs declarations, packing lists. |
| `medical` | Clinical records, lab reports. |
| `hr` | Pay stubs, resumes, employment offers. |
| `other` | Catch-all for documents that don't fit the other categories. |

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

The 18 canonical document layout classes.

All model backends (RT-DETR, YOLO, etc.) map their native class IDs
to this shared set. Models with fewer classes (DocLayNet: 11, PubLayNet: 5)
map to the closest equivalent.

Wire format is snake_case in all serializers (JSON, TOML, YAML).

| Value | Description |
|-------|-------------|
| `caption` | Figure or table caption text. |
| `chart` | Chart or graph visualization. |
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
| `reranking` | The reranker model or reranking pipeline returned an error. Since v5.0. |
| `transcription` | Audio/video transcription failed. |
| `timeout` | The extraction operation exceeded the configured time limit. |
| `cancelled` | The extraction was cancelled via a `CancellationToken`. |
| `security` | A security policy was violated (e.g. zip bomb, oversized archive). |
| `other` | A catch-all for uncommon errors that do not fit another variant. |

---

#### HeuristicsError

Errors that can occur during heuristics analysis.

| Variant | Description |
|---------|-------------|
| `config_error` | Invalid configuration value. |
| `pdf_analysis_error` | PDF analysis step failed (only when `heuristics-pdf` feature is active). |

---

#### LoadError

Errors produced while loading or validating a preset file.

| Variant | Description |
|---------|-------------|
| `parse` | The file is not valid JSON. |
| `schema_validation` | The file parses as JSON but does not validate against the meta-schema. |
| `deserialize` | The file validates but cannot be deserialized into `Preset`. |
| `id_mismatch` | The preset's declared `id` does not match its file-system location. |
| `bad_meta_schema` | The meta-schema itself failed to compile. |
| `io` | A filesystem I/O error occurred while reading a preset directory. |

---

#### ResolveError

Errors produced while resolving a preset against caller overrides.

| Variant | Description |
|---------|-------------|
| `schema_not_object` | A custom schema override was supplied but is not a JSON object. |

---

---
title: "Configuration Reference"
---

## Configuration Reference

This page documents all configuration types and their defaults across all languages.

### AccelerationConfig

Hardware acceleration configuration for ONNX Runtime models.

Controls which execution provider (CPU, CoreML, CUDA, TensorRT) is used
for inference in layout detection and embedding generation.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `provider` | `ExecutionProviderType` | `ExecutionProviderType.AUTO` | Execution provider to use for ONNX inference. |
| `device_id` | `int` | — | GPU device ID (for CUDA/TensorRT). Ignored for CPU/CoreML/Auto. |

---

### CaptioningConfig

Configuration for the VLM captioning post-processor.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `llm` | `LlmConfig` | — | LLM configuration used for the VLM call. |
| `prompt` | `str \| None` | `None` | Optional custom caption prompt. `None` uses the default `RegionKind.Caption` prompt that ships with `crate.llm.region_extractor`. |
| `min_image_area` | `int` | `/* serde(default) */` | Skip images whose `width * height` is below this threshold (in pixels). Default `1_000` filters out icons and decorations. |

---

### PageClassificationConfig

Configuration for the page-classification post-processor.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `prompt_template` | `str \| None` | `None` | Minijinja prompt template. Receives `{{ labels }}` (joined list), `{{ page_text }}` and `{{ multi_label }}` variables. `None` lets the backend pick a sensible default. |
| `labels` | `list[str]` | — | The set of labels the classifier may emit. Must contain at least one entry. |
| `multi_label` | `bool` | `/* serde(default) */` | Allow multiple labels per page. Single-label mode returns at most one label. |
| `llm` | `LlmConfig` | — | LLM configuration used for classification. |

---

### ContentFilterConfig

Cross-extractor content filtering configuration.

Controls whether "furniture" content (headers, footers, page numbers,
watermarks, repeating text) is included in or stripped from extraction
results. Applies across all extractors (PDF, DOCX, RTF, ODT, HTML, etc.)
with format-specific implementation.

When `None` on `ExtractionConfig`, each extractor uses its current
default behavior unchanged.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `include_headers` | `bool` | `False` | Include running headers in extraction output. - PDF: Disables top-margin furniture stripping and prevents the layout model from treating `PageHeader`-classified regions as furniture. - DOCX: Includes document headers in text output. - RTF/ODT: Headers already included; this is a no-op when true. - HTML/EPUB: Keeps `<header>` element content. Default: `False` (headers are stripped or excluded). |
| `include_footers` | `bool` | `False` | Include running footers in extraction output. - PDF: Disables bottom-margin furniture stripping and prevents the layout model from treating `PageFooter`-classified regions as furniture. - DOCX: Includes document footers in text output. - RTF/ODT: Footers already included; this is a no-op when true. - HTML/EPUB: Keeps `<footer>` element content. Default: `False` (footers are stripped or excluded). |
| `strip_repeating_text` | `bool` | `True` | Enable the heuristic cross-page repeating text detector. When `True` (default), text that repeats verbatim across a supermajority of pages is classified as furniture and stripped.  Disable this if brand names or repeated headings are being incorrectly removed by the heuristic. Note: when a layout-detection model is active, the model may independently classify page-header / page-footer regions as furniture on a per-page basis. To preserve those regions, set `include_headers = true`, `include_footers = true`, or both, in addition to disabling this flag. Primarily affects PDF extraction. Default: `True`. |
| `include_watermarks` | `bool` | `False` | Include watermark text in extraction output. - PDF: Keeps watermark artifacts and arXiv identifiers. - Other formats: No effect currently. Default: `False` (watermarks are stripped). |

---

### EmailConfig

Configuration for email extraction.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `msg_fallback_codepage` | `int \| None` | `None` | Windows codepage number to use when an MSG file contains no codepage property. Defaults to `None`, which falls back to windows-1252. If an unrecognized or invalid codepage number is supplied (including 0), the behavior silently falls back to windows-1252 — the same as when the MSG file itself contains an unrecognized codepage. No error or warning is emitted. Users should verify output when supplying unusual values. Common values: - 1250: Central European (Polish, Czech, Hungarian, etc.) - 1251: Cyrillic (Russian, Ukrainian, Bulgarian, etc.) - 1252: Western European (default) - 1253: Greek - 1254: Turkish - 1255: Hebrew - 1256: Arabic - 932:  Japanese (Shift-JIS) - 936:  Simplified Chinese (GBK) |

---

### ExtractionConfig

Main extraction configuration.

This struct contains all configuration options for the extraction process.
It can be loaded from TOML, YAML, or JSON files, or created programmatically.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `use_cache` | `bool` | `True` | Enable caching of extraction results |
| `enable_quality_processing` | `bool` | `True` | Enable quality post-processing |
| `ocr` | `OcrConfig \| None` | `None` | OCR configuration (None = OCR disabled) |
| `force_ocr` | `bool` | `False` | Force OCR even for searchable PDFs |
| `force_ocr_pages` | `list[int] \| None` | `None` | Force OCR on specific pages only (1-indexed page numbers, must be >= 1). When set, only the listed pages are OCR'd regardless of text layer quality. Unlisted pages use native text extraction. Ignored when `force_ocr` is `True`. Only applies to PDF documents. Duplicates are automatically deduplicated. An `ocr` config is recommended for backend/language selection; defaults are used if absent. |
| `disable_ocr` | `bool` | `False` | Disable OCR entirely, even for images. When `True`, OCR is skipped for all document types. Images return metadata only (dimensions, format, EXIF) without text extraction. PDFs use only native text extraction without OCR fallback. Cannot be `True` simultaneously with `force_ocr`. *Added in v4.7.0.* |
| `chunking` | `ChunkingConfig \| None` | `None` | Text chunking configuration (None = chunking disabled) |
| `content_filter` | `ContentFilterConfig \| None` | `None` | Content filtering configuration (None = use extractor defaults). Controls whether document "furniture" (headers, footers, watermarks, repeating text) is included in or stripped from extraction results. See `ContentFilterConfig` for per-field documentation. |
| `images` | `ImageExtractionConfig \| None` | `None` | Image extraction configuration (None = no image extraction) |
| `pdf_options` | `PdfConfig \| None` | `None` | PDF-specific options (None = use defaults) |
| `token_reduction` | `TokenReductionOptions \| None` | `None` | Token reduction configuration (None = no token reduction) |
| `language_detection` | `LanguageDetectionConfig \| None` | `None` | Language detection configuration (None = no language detection) |
| `pages` | `PageConfig \| None` | `None` | Page extraction configuration (None = no page tracking) |
| `keywords` | `KeywordConfig \| None` | `None` | Keyword extraction configuration (None = no keyword extraction) |
| `postprocessor` | `PostProcessorConfig \| None` | `None` | Post-processor configuration (None = use defaults) |
| `html_options` | `str \| None` | `None` | HTML to Markdown conversion options (None = use defaults) Configure how HTML documents are converted to Markdown, including heading styles, list formatting, code block styles, and preprocessing options. |
| `html_output` | `HtmlOutputConfig \| None` | `None` | Styled HTML output configuration. When set alongside `output_format = OutputFormat.Html`, the extraction pipeline uses `StyledHtmlRenderer` which emits stable `kb-*` CSS class hooks on every structural element and optionally embeds theme CSS or user-supplied CSS in a `<style>` block. When `None`, the existing plain comrak-based HTML renderer is used. |
| `extraction_timeout_secs` | `int \| None` | `None` | Default per-file timeout in seconds for batch extraction. When set, each file in a batch will be canceled after this duration unless overridden by `FileExtractionConfig.timeout_secs`. Defaults to `Some(60)` to prevent pathological files (e.g. deeply nested archives, documents with millions of cells) from running indefinitely and exhausting caller resources. Set to `None` to disable the timeout for trusted input or long-running workloads. |
| `max_concurrent_extractions` | `int \| None` | `None` | Maximum concurrent extractions in batch operations (None = (num_cpus × 1.5).ceil()). Limits parallelism to prevent resource exhaustion when processing large batches. Defaults to (num_cpus × 1.5).ceil() when not set. |
| `result_format` | `ResultFormat` | `ResultFormat.UNIFIED` | Result structure format Controls whether results are returned in unified format (default) with all content in the `content` field, or element-based format with semantic elements (for Unstructured-compatible output). |
| `security_limits` | `SecurityLimits \| None` | `None` | Security limits for archive extraction. Controls maximum archive size, compression ratio, file count, and other security thresholds to prevent decompression bomb attacks. Also caps nesting depth, iteration count, entity / token length, total content size, and table cell count for every extraction path that ingests user-controlled bytes. When `None`, default limits are used. |
| `max_embedded_file_bytes` | `int \| None` | `None` | Maximum uncompressed size in bytes for a single embedded file before recursive extraction is attempted (default: 50 MiB). Applies to embedded objects inside OOXML containers (DOCX, PPTX) and to email attachments processed via recursive extraction. Files that exceed this limit are skipped with a `ProcessingWarning` rather than passed to the extraction pipeline, preventing a single oversized embedded object from consuming unbounded memory or time. Set to `None` to disable the per-embedded-file cap (falls back to `security_limits.max_archive_size` as the only guard). |
| `output_format` | `OutputFormat` | `OutputFormat.PLAIN` | Content text format (default: Plain). Controls the format of the extracted content: - `Plain`: Raw extracted text (default) - `Markdown`: Markdown formatted output - `Djot`: Djot markup format (requires djot feature) - `Html`: HTML formatted output When set to a structured format, extraction results will include formatted output. The `formatted_content` field may be populated when format conversion is applied. |
| `layout` | `LayoutDetectionConfig \| None` | `None` | Layout detection configuration (None = layout detection disabled). When set, PDF pages and images are analyzed for document structure (headings, code, formulas, tables, figures, etc.) using RT-DETR models via ONNX Runtime. For PDFs, layout hints override paragraph classification in the markdown pipeline. For images, per-region OCR is performed with markdown formatting based on detected layout classes. Requires the `layout-detection` feature to run inference; the field is present whenever the `layout-types` feature is active (which includes `layout-detection` as well as the no-ORT target groups). |
| `use_layout_for_markdown` | `bool` | `False` | Run layout detection on the non-OCR PDF markdown path. When `True` and `layout` is `Some(_)`, layout regions inform heading, table, list, and figure detection in the structure pipeline that would otherwise rely on font-clustering heuristics alone. Significantly improves SF1 (structural F1) at the cost of inference latency (~150-300ms/page CPU, ~20-50ms/page GPU). Default: `False`. Requires the `layout-detection` feature. |
| `include_document_structure` | `bool` | `False` | Enable structured document tree output. When true, populates the `document` field on `ExtractionResult` with a hierarchical `DocumentStructure` containing heading-driven section nesting, table grids, content layer classification, and inline annotations. Independent of `result_format` — can be combined with Unified or ElementBased. |
| `acceleration` | `AccelerationConfig \| None` | `None` | Hardware acceleration configuration for ONNX Runtime models. Controls execution provider selection for layout detection and embedding models. When `None`, uses platform defaults (CoreML on macOS, CUDA on Linux, CPU on Windows). |
| `cache_namespace` | `str \| None` | `None` | Cache namespace for tenant isolation. When set, cache entries are stored under `{cache_dir}/{namespace}/`. Must be alphanumeric, hyphens, or underscores only (max 64 chars). Different namespaces have isolated cache spaces on the same filesystem. |
| `cache_ttl_secs` | `int \| None` | `None` | Per-request cache TTL in seconds. Overrides the global `max_age_days` for this specific extraction. When `0`, caching is completely skipped (no read or write). When `None`, the global TTL applies. |
| `email` | `EmailConfig \| None` | `None` | Email extraction configuration (None = use defaults). Currently supports configuring the fallback codepage for MSG files that do not specify one. See `EmailConfig` for details. |
| `concurrency` | `str \| None` | `None` | Concurrency limits for constrained environments (None = use defaults). Controls Rayon thread pool size, ONNX Runtime intra-op threads, and (when `max_concurrent_extractions` is unset) the batch concurrency semaphore. See `ConcurrencyConfig` for details. |
| `max_archive_depth` | `int` | — | Maximum recursion depth for archive extraction (default: 3). Set to 0 to disable recursive extraction (legacy behavior). |
| `tree_sitter` | `TreeSitterConfig \| None` | `None` | Tree-sitter language pack configuration (None = tree-sitter disabled). When set, enables code file extraction using tree-sitter parsers. Controls grammar download behavior and code analysis options. |
| `structured_extraction` | `StructuredExtractionConfig \| None` | `None` | Structured extraction via LLM (None = disabled). When set, the extracted document content is sent to an LLM with the provided JSON schema. The structured response is stored in `ExtractionResult.structured_output`. |
| `ner` | `NerConfig \| None` | `None` | Named-entity recognition configuration. When set, the NER post-processor runs at the Middle stage and populates `ExtractionResult.entities`. |
| `redaction` | `RedactionConfig \| None` | `None` | Redaction / anonymisation configuration. When set, the redaction post-processor runs at the Late stage and rewrites every textual field in `ExtractionResult`, emitting an audit trail in `ExtractionResult.redaction_report`. |
| `summarization` | `SummarizationConfig \| None` | `None` | Summarisation configuration. When set, the summarisation post-processor runs at the Middle stage and populates `ExtractionResult.summary`. |
| `translation` | `TranslationConfig \| None` | `None` | Translation configuration. When set, the translation post-processor runs at the Middle stage and populates `ExtractionResult.translation`. |
| `page_classification` | `PageClassificationConfig \| None` | `None` | Per-page classification configuration. When set, the classification post-processor runs at the Middle stage and populates `ExtractionResult.page_classifications`. |
| `captioning` | `CaptioningConfig \| None` | `None` | VLM captioning configuration for extracted images. When set, the captioning post-processor runs at the Middle stage and writes a caption into each `ExtractedImage.caption`. |
| `qr_codes` | `bool \| None` | `None` | Enable QR-code detection in extracted images. When `True`, the QR post-processor runs at the Middle stage and populates `ExtractedImage.qr_codes`. |
| `cancel_token` | `str \| None` | `None` | Cancellation token for this extraction (None = no external cancellation). Pass a `CancellationToken` clone here and call `CancellationToken.cancel` from another thread / task to abort the extraction in progress. The extractor checks the token at safe checkpoints (before lock acquisition, between pages, between batch items) and returns `KreuzbergError.Cancelled` when set. The field is excluded from serialization because `CancellationToken` is a runtime handle, not a configuration value. |

---

### FileExtractionConfig

Per-file extraction configuration overrides for batch processing.

All fields are `Option<T>` — `None` means "use the batch-level default."
This type is used with `batch_extract_files` and
`batch_extract_bytes` to allow heterogeneous
extraction settings within a single batch.

## Excluded Fields

The following `ExtractionConfig` fields are batch-level only and
cannot be overridden per file:

- `max_concurrent_extractions` — controls batch parallelism
- `use_cache` — global caching policy
- `acceleration` — shared ONNX execution provider
- `security_limits` — global archive security policy

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enable_quality_processing` | `bool \| None` | `None` | Override quality post-processing for this file. |
| `ocr` | `OcrConfig \| None` | `None` | Override OCR configuration for this file (None in the Option = use batch default). |
| `force_ocr` | `bool \| None` | `None` | Override force OCR for this file. |
| `force_ocr_pages` | `list[int] \| None` | `[]` | Override force OCR pages for this file (1-indexed page numbers). |
| `disable_ocr` | `bool \| None` | `None` | Override disable OCR for this file. |
| `chunking` | `ChunkingConfig \| None` | `None` | Override chunking configuration for this file. |
| `content_filter` | `ContentFilterConfig \| None` | `None` | Override content filtering configuration for this file. |
| `images` | `ImageExtractionConfig \| None` | `None` | Override image extraction configuration for this file. |
| `pdf_options` | `PdfConfig \| None` | `None` | Override PDF options for this file. |
| `token_reduction` | `TokenReductionOptions \| None` | `None` | Override token reduction for this file. |
| `language_detection` | `LanguageDetectionConfig \| None` | `None` | Override language detection for this file. |
| `pages` | `PageConfig \| None` | `None` | Override page extraction for this file. |
| `keywords` | `KeywordConfig \| None` | `None` | Override keyword extraction for this file. |
| `postprocessor` | `PostProcessorConfig \| None` | `None` | Override post-processor for this file. |
| `html_options` | `str \| None` | `None` | Override HTML conversion options for this file. |
| `result_format` | `ResultFormat \| None` | `None` | Override result format for this file. |
| `output_format` | `OutputFormat \| None` | `None` | Override output content format for this file. |
| `include_document_structure` | `bool \| None` | `None` | Override document structure output for this file. |
| `layout` | `LayoutDetectionConfig \| None` | `None` | Override layout detection for this file. |
| `timeout_secs` | `int \| None` | `None` | Override per-file extraction timeout in seconds. When set, the extraction for this file will be canceled after the specified duration. A timed-out file produces an error result without affecting other files in the batch. |
| `tree_sitter` | `TreeSitterConfig \| None` | `None` | Override tree-sitter configuration for this file. |
| `structured_extraction` | `StructuredExtractionConfig \| None` | `None` | Override structured extraction configuration for this file. When set, enables LLM-based structured extraction with a JSON schema for this specific file. The extracted content is sent to a VLM/LLM and the response is parsed according to the provided schema. |

---

### ImageExtractionConfig

Image extraction configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `extract_images` | `bool` | `True` | Extract images from documents |
| `target_dpi` | `int` | `300` | Target DPI for image normalization |
| `max_image_dimension` | `int` | `4096` | Maximum dimension for images (width or height) |
| `inject_placeholders` | `bool` | `True` | Whether to inject image reference placeholders into markdown output. When `True` (default), image references like `![Image 1](embedded:p1_i0)` are appended to the markdown. Set to `False` to extract images as data without polluting the markdown output. |
| `auto_adjust_dpi` | `bool` | `True` | Automatically adjust DPI based on image content |
| `min_dpi` | `int` | `72` | Minimum DPI threshold |
| `max_dpi` | `int` | `600` | Maximum DPI threshold |
| `max_images_per_page` | `int \| None` | `None` | Maximum number of image objects to extract per PDF page. Some PDFs (e.g. technical diagrams stored as thousands of raster fragments) can trigger extremely long or indefinite extraction times when every image object on a dense page is decoded individually via the PDF extractor. Setting this limit causes kreuzberg to stop collecting individual images once the count per page reaches the cap and emit a warning instead. `None` (default) means no limit — all images are extracted. |
| `classify` | `bool` | `True` | When `True` (default), extracted images are classified by kind and grouped into clusters where they appear to belong to one figure. |
| `include_page_rasters` | `bool` | `False` | When `True`, full-page renders produced during OCR preprocessing are captured and returned as `ImageKind.PageRaster` entries in `ExtractionResult.images`. **PDF + OCR only.** No rasters are captured for non-PDF inputs or when the document-level OCR bypass is active (whole-document backend). When OCR is enabled and this flag is set but the active backend skips per-page rendering, a `ProcessingWarning` is emitted in `ExtractionResult.processing_warnings`. Defaults to `False`. Enable when downstream consumers need page thumbnails (e.g. citation previews, visual grounding). |
| `run_ocr_on_images` | `bool` | `True` | Run OCR on extracted images and include the recognized text in the document content. When `True` (default) and `ExtractionConfig.ocr` is configured, extracted images are processed with the configured OCR backend. Set to `False` to extract images without OCR processing, even when OCR is enabled. |
| `ocr_text_only` | `bool` | `False` | When `True`, image OCR results are rendered as plain text without the `![...](...)` markdown placeholder. Only takes effect when `run_ocr_on_images` is also `True`. |
| `append_ocr_text` | `bool` | `False` | When `True` and `ocr_text_only` is `False`, append the OCR text after the image placeholder in the rendered output. |

---

### TokenReductionOptions

Token reduction configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `mode` | `str` | — | Reduction mode: "off", "light", "moderate", "aggressive", "maximum" |
| `preserve_important_words` | `bool` | `True` | Preserve important words (capitalized, technical terms) |

---

### LanguageDetectionConfig

Language detection configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `bool` | `True` | Enable language detection |
| `min_confidence` | `float` | `0.8` | Minimum confidence threshold (0.0-1.0) |
| `detect_multiple` | `bool` | `False` | Detect multiple languages in the document |

---

### HtmlOutputConfig

Configuration for styled HTML output.

When set on `ExtractionConfig.html_output` alongside
`output_format = OutputFormat.Html`, the pipeline builds a
`StyledHtmlRenderer` instead of
the plain comrak-based renderer.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `css` | `str \| None` | `None` | Inline CSS string injected into the output after the theme stylesheet. Concatenated after `css_file` content when both are set. |
| `css_file` | `str \| None` | `None` | Path to a CSS file loaded once at renderer construction time. Concatenated before `css` when both are set. |
| `theme` | `HtmlTheme` | `HtmlTheme.UNSTYLED` | Built-in colour/typography theme. Default: `HtmlTheme.Unstyled`. |
| `class_prefix` | `str` | — | CSS class prefix applied to every emitted class name. Default: `"kb-"`. Change this if your host application already uses classes that start with `kb-`. |
| `embed_css` | `bool` | `True` | When `True` (default), write the resolved CSS into a `<style>` block immediately after the opening `<div class="{prefix}doc">`. Set to `False` to emit only the structural markup and wire up your own stylesheet targeting the `kb-*` class names. |

---

### LayoutDetectionConfig

Layout detection configuration.

Controls layout detection behavior in the extraction pipeline.
When set on `ExtractionConfig`, layout detection
is enabled for PDF extraction.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `confidence_threshold` | `float \| None` | `None` | Confidence threshold override (None = use model default). |
| `apply_heuristics` | `bool` | `True` | Whether to apply postprocessing heuristics (default: true). |
| `table_model` | `TableModel` | `TableModel.TATR` | Table structure recognition model. Controls which model is used for table cell detection within layout-detected table regions. Defaults to `TableModel.Tatr`. |
| `acceleration` | `AccelerationConfig \| None` | `None` | Hardware acceleration for ONNX models (layout detection + table structure). When set, controls which execution provider (CPU, CUDA, CoreML, TensorRT) is used for inference. Defaults to `None` (auto-select per platform). |

---

### LlmConfig

Configuration for an LLM provider/model via liter-llm.

Each feature (VLM OCR, VLM embeddings, structured extraction) carries
its own `LlmConfig`, allowing different providers per feature.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `str` | — | Provider/model string using liter-llm routing format. Examples: `"openai/gpt-4o"`, `"anthropic/claude-sonnet-4-20250514"`, `"groq/llama-3.1-70b-versatile"`. |
| `api_key` | `str \| None` | `None` | API key for the provider. When `None`, liter-llm falls back to the provider's standard environment variable (e.g., `OPENAI_API_KEY`). |
| `base_url` | `str \| None` | `None` | Custom base URL override for the provider endpoint. |
| `timeout_secs` | `int \| None` | `None` | Request timeout in seconds (default: 60). |
| `max_retries` | `int \| None` | `None` | Maximum retry attempts (default: 3). |
| `temperature` | `float \| None` | `None` | Sampling temperature for generation tasks. |
| `max_tokens` | `int \| None` | `None` | Maximum tokens to generate. |

---

### StructuredExtractionConfig

Configuration for LLM-based structured data extraction.

Sends extracted document content to a VLM with a JSON schema,
returning structured data that conforms to the schema.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `schema` | `dict[str, Any]` | — | JSON Schema defining the desired output structure. |
| `schema_name` | `str` | `/* serde(default) */` | Schema name passed to the LLM's structured output mode. |
| `schema_description` | `str \| None` | `/* serde(default) */` | Optional schema description for the LLM. |
| `strict` | `bool` | `/* serde(default) */` | Enable strict mode — output must exactly match the schema. |
| `prompt` | `str \| None` | `/* serde(default) */` | Custom Jinja2 extraction prompt template. When `None`, a default template is used. Available template variables: - `{{ content }}` — The extracted document text. - `{{ schema }}` — The JSON schema as a formatted string. - `{{ schema_name }}` — The schema name. - `{{ schema_description }}` — The schema description (may be empty). |
| `llm` | `LlmConfig` | — | LLM configuration for the extraction. |

---

### NerConfig

Configuration for the NER post-processor.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `backend` | `NerBackendKind` | `NerBackendKind.ONNX` | Backend that runs the entity detection. |
| `categories` | `list[EntityCategory]` | `[]` | Entity categories to detect. Defaults to a sensible PERSON/ORG/LOCATION/EMAIL set when empty. |
| `model` | `str \| None` | `None` | Override the default model — only used by `NerBackendKind.Onnx`. `None` lets the backend pick its pinned default (`urchade/gliner_multi-v2.1` for gline-rs). |
| `llm` | `LlmConfig \| None` | `None` | Optional LLM configuration — only used by `NerBackendKind.Llm`. Token usage for LLM backends is recorded in `ExtractionResult.llm_usage`. |
| `custom_labels` | `list[str]` | `[]` | Arbitrary user-supplied entity labels for zero-shot detection. gline-rs natively supports zero-shot inference over caller-supplied labels — this is the primary value of GLiNER. The LLM backend also honours these labels by including them in the structured-output schema. Custom labels surface as `EntityCategory.Custom` in the resulting `Entity` stream. Use this when you need domain-specific entity types (e.g. `"Treatment"`, `"Product"`, `"Vessel"`) without forking GLiNER's taxonomy. |

---

### OcrQualityThresholds

Quality thresholds for OCR fallback decisions and pipeline quality gating.

All fields default to the values that match the previous hardcoded behavior,
so `OcrQualityThresholds.default()` preserves existing semantics exactly.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `min_total_non_whitespace` | `int` | `64` | Minimum total non-whitespace characters to consider text substantive. |
| `min_non_whitespace_per_page` | `float` | `32` | Minimum non-whitespace characters per page on average. |
| `min_meaningful_word_len` | `int` | `4` | Minimum character count for a word to be "meaningful". |
| `min_meaningful_words` | `int` | `3` | Minimum count of meaningful words before text is accepted. |
| `min_alnum_ratio` | `float` | `0.3` | Minimum alphanumeric ratio (non-whitespace chars that are alphanumeric). |
| `min_garbage_chars` | `int` | `5` | Minimum Unicode replacement characters (U+FFFD) to trigger OCR fallback. |
| `max_fragmented_word_ratio` | `float` | `0.6` | Maximum fraction of short (1-2 char) words before text is considered fragmented. |
| `critical_fragmented_word_ratio` | `float` | `0.8` | Critical fragmentation threshold — triggers OCR regardless of meaningful words. Normal English text has ~20-30% short words. 80%+ is definitive garbage. |
| `min_avg_word_length` | `float` | `2` | Minimum average word length. Below this with enough words indicates garbled extraction. |
| `min_words_for_avg_length_check` | `int` | `50` | Minimum word count before average word length check applies. |
| `min_consecutive_repeat_ratio` | `float` | `0.08` | Minimum consecutive word repetition ratio to detect column scrambling. |
| `min_words_for_repeat_check` | `int` | `50` | Minimum word count before consecutive repetition check is applied. |
| `substantive_min_chars` | `int` | `100` | Minimum character count for "substantive markdown" OCR skip gate. |
| `non_text_min_chars` | `int` | `20` | Minimum character count for "non-text content" OCR skip gate. |
| `alnum_ws_ratio_threshold` | `float` | `0.4` | Alphanumeric+whitespace ratio threshold for skip decisions. |
| `pipeline_min_quality` | `float` | `0.5` | Minimum quality score (0.0-1.0) for a pipeline stage result to be accepted. If the result from a backend scores below this, try the next backend. |

---

### OcrPipelineConfig

Multi-backend OCR pipeline with quality-based fallback.

Backends are tried in priority order (highest first). After each backend
produces output, quality is evaluated. If it meets `quality_thresholds.pipeline_min_quality`,
the result is accepted. Otherwise the next backend is tried.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `stages` | `list[OcrPipelineStage]` | — | Ordered list of backends to try. Sorted by priority (descending) at runtime. |
| `quality_thresholds` | `OcrQualityThresholds` | `/* serde(default) */` | Quality thresholds for deciding whether to accept a result or try the next backend. |

---

### OcrConfig

OCR configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `bool` | `True` | Whether OCR is enabled. Setting `enabled: false` is a shorthand for `disable_ocr: true` on the parent `ExtractionConfig`. Images return metadata only; PDFs use native text extraction without OCR fallback. Defaults to `True`. When `False`, all other OCR settings are ignored. |
| `backend` | `str` | — | OCR backend: tesseract, easyocr, paddleocr |
| `language` | `str` | — | Language code (e.g., "eng", "deu") |
| `tesseract_config` | `TesseractConfig \| None` | `None` | Tesseract-specific configuration (optional) |
| `output_format` | `OutputFormat \| None` | `None` | Output format for OCR results (optional, for format conversion) |
| `paddle_ocr_config` | `dict[str, Any] \| None` | `None` | PaddleOCR-specific configuration (optional, JSON passthrough) |
| `backend_options` | `dict[str, Any] \| None` | `None` | Arbitrary per-call options passed through to the backend unchanged. Custom OCR backends and built-in backends that support runtime tuning can read this value and deserialize the keys they care about. Keys unknown to the backend are silently ignored. This is the recommended extension point for per-call parameters that are not covered by the typed fields above (e.g. mode switching, preprocessing flags, inference batch size). **Scope:** when `pipeline` is `None`, this value is propagated to the primary stage of the auto-constructed pipeline. When `pipeline` is explicitly set, this field has **no effect** — the caller must set `OcrPipelineStage.backend_options` directly on the relevant stage(s) instead. Example: ```json { "mode": "fast", "enable_layout": true, "timeout_ms": 5000 }``` |
| `element_config` | `OcrElementConfig \| None` | `None` | OCR element extraction configuration |
| `quality_thresholds` | `OcrQualityThresholds \| None` | `None` | Quality thresholds for the native-text-to-OCR fallback decision. When None, uses compiled defaults (matching previous hardcoded behavior). |
| `pipeline` | `OcrPipelineConfig \| None` | `None` | Multi-backend OCR pipeline configuration. When set, enables weighted fallback across multiple OCR backends based on output quality. When None, uses the single `backend` field (same as today). |
| `auto_rotate` | `bool` | `False` | Enable automatic page rotation based on orientation detection. When enabled, uses Tesseract's `DetectOrientationScript()` to detect page orientation (0/90/180/270 degrees) before OCR. If the page is rotated with high confidence, the image is corrected before recognition. This is critical for handling rotated scanned documents. |
| `vlm_fallback` | `VlmFallbackPolicy` | `VlmFallbackPolicy.DISABLED` | Ergonomic VLM fallback policy. When set to anything other than `VlmFallbackPolicy.Disabled` and `OcrConfig.pipeline` is `None`, a multi-stage pipeline is synthesised automatically: - `VlmFallbackPolicy.OnLowQuality` → `[classical_stage, vlm_stage]` with the `quality_threshold` mapped onto `OcrQualityThresholds.pipeline_min_quality`. - `VlmFallbackPolicy.Always` → `[vlm_stage]` only. Requires `OcrConfig.vlm_config` to be `Some` when not `Disabled`. When `OcrConfig.pipeline` is explicitly set, this field is ignored. |
| `vlm_config` | `LlmConfig \| None` | `None` | VLM (Vision Language Model) OCR configuration. Required when `backend` is `"vlm"` or when `vlm_fallback` is not `VlmFallbackPolicy.Disabled`. Uses liter-llm to send page images to a vision model for text extraction. |
| `vlm_prompt` | `str \| None` | `None` | Custom Jinja2 prompt template for VLM OCR. When `None`, uses the default template. Available variables: - `{{ language }}` — The document language code (e.g., "eng", "deu"). |
| `acceleration` | `AccelerationConfig \| None` | `None` | Hardware acceleration for ONNX Runtime models (e.g. PaddleOCR, layout detection). Not user-configurable via config files — injected at runtime from `ExtractionConfig.acceleration` before each `process_image` call. |
| `tessdata_bytes` | `dict[str, bytes] \| None` | `None` | Caller-supplied Tesseract `traineddata` bytes per language code. Primary use case is the WASM build, which has no filesystem and cannot download tessdata at runtime. Native builds typically rely on `TessdataManager` and ignore this field. When present, the WASM Tesseract backend prefers these bytes over its compile-time-bundled English data. Skipped by serde to keep config files small — supply via the typed API at runtime. |

---

### PageConfig

Page extraction and tracking configuration.

Controls how pages are extracted, tracked, and represented in the extraction results.
When `None`, page tracking is disabled.

Page range tracking in chunk metadata (first_page/last_page) is automatically enabled
when page boundaries are available and chunking is configured.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `extract_pages` | `bool` | `False` | Extract pages as separate array (ExtractionResult.pages) |
| `insert_page_markers` | `bool` | `False` | Insert page markers in main content string |
| `marker_format` | `str` | `"<!-- PAGE {page_num} -->"` | Page marker format (use {page_num} placeholder) Default: "\n\n<!-- PAGE {page_num} -->\n\n" |

---

### PdfConfig

PDF-specific configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `extract_images` | `bool` | `False` | Extract images from PDF |
| `extract_tables` | `bool` | `True` | Extract tables from PDF. When `True` (default), runs pdf_oxide's native grid detector and, if it finds nothing, falls back to the heuristic text-layer reconstruction in `pdf.oxide.table.extract_tables_heuristic`. Set to `False` to skip both passes — `tables` will then be empty in the result. |
| `passwords` | `list[str] \| None` | `None` | List of passwords to try when opening encrypted PDFs |
| `extract_metadata` | `bool` | `True` | Extract PDF metadata |
| `hierarchy` | `HierarchyConfig \| None` | `None` | Hierarchy extraction configuration (None = hierarchy extraction disabled) |
| `extract_annotations` | `bool` | `False` | Extract PDF annotations (text notes, highlights, links, stamps). Default: false |
| `top_margin_fraction` | `float \| None` | `None` | Top margin fraction (0.0–1.0) of page height to exclude headers/running heads. Default: 0.06 (6%) |
| `bottom_margin_fraction` | `float \| None` | `None` | Bottom margin fraction (0.0–1.0) of page height to exclude footers/page numbers. Default: 0.05 (5%) |
| `allow_single_column_tables` | `bool` | `False` | Allow single-column pseudo tables in extraction results. By default, tables with fewer than 2 columns (layout-guided) or 3 columns (heuristic) are rejected. When `True`, the minimum column count is relaxed to 1, allowing single-column structured data (glossaries, itemized lists) to be emitted as tables. Other quality filters (density, sparsity, prose detection) still apply. |
| `ocr_inline_images` | `bool` | `False` | Perform OCR on inline images extracted from PDF pages and attach the recognized text to each `ExtractedImage.ocr_result`. Requires Tesseract to be available; if `ExtractionConfig.ocr` is `None` the extractor falls back to `TesseractConfig.default()`. Per-image failures degrade gracefully (the image is returned without OCR text rather than failing the whole extraction). Default: `False`. |

---

### HierarchyConfig

Hierarchy extraction configuration for PDF text structure analysis.

Enables extraction of document hierarchy levels (H1-H6) based on font size
clustering and semantic analysis. When enabled, hierarchical blocks are
included in page content.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `bool` | `True` | Enable hierarchy extraction |
| `k_clusters` | `int` | `3` | Number of font size clusters to use for hierarchy levels (1-7) Default: 6, which provides H1-H6 heading levels with body text. Larger values create more fine-grained hierarchy levels. |
| `include_bbox` | `bool` | `True` | Include bounding box information in hierarchy blocks |
| `ocr_coverage_threshold` | `float \| None` | `None` | OCR coverage threshold for smart OCR triggering (0.0-1.0) Determines when OCR should be triggered based on text block coverage. OCR is triggered when text blocks cover less than this fraction of the page. Default: 0.5 (trigger OCR if less than 50% of page has text) |

---

### PostProcessorConfig

Post-processor configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `bool` | `True` | Enable post-processors |
| `enabled_processors` | `list[str] \| None` | `None` | Whitelist of processor names to run (None = all enabled) |
| `disabled_processors` | `list[str] \| None` | `None` | Blacklist of processor names to skip (None = none disabled) |
| `enabled_set` | `list[str] \| None` | `None` | Pre-computed AHashSet for O(1) enabled processor lookup |
| `disabled_set` | `list[str] \| None` | `None` | Pre-computed AHashSet for O(1) disabled processor lookup |

---

### ChunkingConfig

Chunking configuration.

Configures text chunking for document content, including chunk size,
overlap, trimming behavior, and optional embeddings.

Use `..the default constructor` when constructing to allow for future field additions:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_characters` | `int` | `1000` | Maximum size per chunk (in units determined by `sizing`). When `sizing` is `Characters` (default), this is the max character count. When using token-based sizing, this is the max token count. Default: 1000 |
| `overlap` | `int` | `200` | Overlap between chunks (in units determined by `sizing`). Default: 200 |
| `trim` | `bool` | `True` | Whether to trim whitespace from chunk boundaries. Default: true |
| `chunker_type` | `ChunkerType` | `ChunkerType.TEXT` | Type of chunker to use (Text or Markdown). Default: Text |
| `embedding` | `EmbeddingConfig \| None` | `None` | Optional embedding configuration for chunk embeddings. |
| `preset` | `str \| None` | `None` | Use a preset configuration (overrides individual settings if provided). |
| `sizing` | `ChunkSizing` | `ChunkSizing.CHARACTERS` | How to measure chunk size. Default: `Characters` (Unicode character count). Enable `chunking-tiktoken` or `chunking-tokenizers` features for token-based sizing. |
| `prepend_heading_context` | `bool` | `False` | When `True` and `chunker_type` is `Markdown`, prepend the heading hierarchy path (e.g. `"# Title > ## Section\n\n"`) to each chunk's content string. This is useful for RAG pipelines where each chunk needs self-contained context about its position in the document structure. Default: `False` |
| `topic_threshold` | `float \| None` | `None` | Optional cosine similarity threshold for semantic topic boundary detection. Only used when `chunker_type` is `Semantic` and an `EmbeddingConfig` is provided. You almost never need to set this. When omitted, defaults to `0.75` which works well for most documents. Lower values detect more topic boundaries (more, smaller chunks); higher values detect fewer. Range: `0.0..=1.0`. |

---

### EmbeddingConfig

Embedding configuration for text chunks.

Configures embedding generation using ONNX models via the vendored embedding engine.
Requires the `embeddings` feature to be enabled.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `EmbeddingModelType` | `EmbeddingModelType.PRESET` | The embedding model to use (defaults to "balanced" preset if not specified) |
| `normalize` | `bool` | `True` | Whether to normalize embedding vectors (recommended for cosine similarity) |
| `batch_size` | `int` | `32` | Batch size for embedding generation |
| `show_download_progress` | `bool` | `False` | Show model download progress |
| `cache_dir` | `str \| None` | `None` | Custom cache directory for model files Defaults to `~/.cache/kreuzberg/embeddings/` if not specified. Allows full customization of model download location. |
| `acceleration` | `AccelerationConfig \| None` | `None` | Hardware acceleration for the embedding ONNX model. When set, controls which execution provider (CPU, CUDA, CoreML, TensorRT) is used for inference. Defaults to `None` (auto-select per platform). |
| `max_embed_duration_secs` | `int \| None` | `None` | Maximum wall-clock duration (in seconds) for a single `embed()` call when using `EmbeddingModelType.Plugin`. Applies only to the in-process plugin path — protects against hung host-language backends (e.g. a Python callback deadlocked on the GIL, a model stuck on CUDA OOM retries, etc.). On timeout, the dispatcher returns `Plugin` instead of blocking forever. `None` disables the timeout. The default (60 seconds) is conservative for common in-process inference; increase for large batches on slow hardware. |

---

### RedactionConfig

Configuration for the redaction post-processor.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `categories` | `list[PiiCategory]` | `[]` | Categories to redact. Empty means "every category supported by the engine." |
| `strategy` | `RedactionStrategy` | `RedactionStrategy.MASK` | Strategy applied to every match. |
| `ner` | `NerConfig \| None` | `None` | Optional NER backend — required to redact PERSON / ORGANIZATION / LOCATION categories (the pure-Rust pattern engine only covers regex-detectable PII). |
| `preserve_offsets` | `bool` | `True` | When `True`, chunk byte ranges are kept consistent with the rewritten content by adjusting `byte_start` / `byte_end` after replacement. When `False`, chunk byte ranges still refer to the *original* content offsets — useful when downstream consumers want to map findings back to the original document. |
| `custom_terms` | `list[RedactionTerm]` | `[]` | Arbitrary user-supplied literal terms to redact. Each term is treated as a regex hit against the document, surfacing as `PiiCategory.Custom(label)` in `RedactionFinding` where `label` is the per-term label (defaulting to the literal value itself). Case-insensitive by default; set `RedactionTerm.case_sensitive` for exact match. Use this when you need to redact tenant-specific tokens (employee IDs, project codes, internal product names) without writing a custom plugin. |
| `custom_patterns` | `list[RedactionPattern]` | `[]` | Arbitrary user-supplied regex patterns to redact. Same surfacing semantics as `custom_terms`: each hit becomes a `PiiCategory.Custom(label)` finding. Patterns are validated at config-construction time via `RedactionConfig.validate`. |

---

### SummarizationConfig

Configuration for the summarisation post-processor.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `strategy` | `SummaryStrategy` | `SummaryStrategy.EXTRACTIVE` | Summarisation strategy. |
| `max_tokens` | `int \| None` | `None` | Maximum summary length in tokens. `None` lets the backend pick a default. |
| `llm` | `LlmConfig \| None` | `None` | LLM configuration for the abstractive backend. Ignored when `strategy = Extractive`. Required when `strategy = Abstractive`. |

---

### TranslationConfig

Configuration for the translation post-processor.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `target_lang` | `str` | — | BCP-47 language tag for the target language (e.g. `"de"`, `"fr-CA"`). |
| `source_lang` | `str \| None` | `None` | Optional explicit source language. `None` asks the backend to auto-detect. |
| `preserve_markup` | `bool` | `/* serde(default) */` | Translate the formatted (Markdown/HTML) rendition alongside plain text when `formatted_content` is present. |
| `llm` | `LlmConfig` | — | LLM configuration used for translation. |

---

### TreeSitterConfig

Configuration for tree-sitter language pack integration.

Controls grammar download behavior and code analysis options.

## Example (TOML)

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
| `enabled` | `bool` | `True` | Enable code intelligence processing (default: true). When `False`, tree-sitter analysis is completely skipped even if the config section is present. |
| `cache_dir` | `str \| None` | `None` | Custom cache directory for downloaded grammars. When `None`, uses the default: `~/.cache/tree-sitter-language-pack/v{version}/libs/`. |
| `languages` | `list[str] \| None` | `None` | Languages to pre-download on init (e.g., `["python", "rust"]`). |
| `groups` | `list[str] \| None` | `None` | Language groups to pre-download (e.g., `["web", "systems", "scripting"]`). |
| `process` | `TreeSitterProcessConfig` | — | Processing options for code analysis. |

---

### TreeSitterProcessConfig

Processing options for tree-sitter code analysis.

Controls which analysis features are enabled when extracting code files.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `structure` | `bool` | `True` | Extract structural items (functions, classes, structs, etc.). Default: true. |
| `imports` | `bool` | `True` | Extract import statements. Default: true. |
| `exports` | `bool` | `True` | Extract export statements. Default: true. |
| `comments` | `bool` | `False` | Extract comments. Default: false. |
| `docstrings` | `bool` | `False` | Extract docstrings. Default: false. |
| `symbols` | `bool` | `False` | Extract symbol definitions. Default: false. |
| `diagnostics` | `bool` | `False` | Include parse diagnostics. Default: false. |
| `chunk_max_size` | `int \| None` | `None` | Maximum chunk size in bytes. `None` disables chunking. |
| `content_mode` | `CodeContentMode` | `CodeContentMode.CHUNKS` | Content rendering mode for code extraction. |

---

### ServerConfig

API server configuration.

This struct holds all configuration options for the Kreuzberg API server,
including host/port settings, CORS configuration, and upload limits.

## Defaults

- `host`: "127.0.0.1" (localhost only)
- `port`: 8000
- `cors_origins`: empty vector (allows all origins)
- `max_request_body_bytes`: 104_857_600 (100 MB)
- `max_multipart_field_bytes`: 104_857_600 (100 MB)

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `host` | `str` | — | Server host address (e.g., "127.0.0.1", "0.0.0.0") |
| `port` | `int` | — | Server port number |
| `cors_origins` | `list[str]` | `[]` | CORS allowed origins. Empty vector means allow all origins. If this is an empty vector, the server will accept requests from any origin. If populated with specific origins (e.g., `"<https://example.com"`>), only those origins will be allowed. |
| `max_request_body_bytes` | `int` | — | Maximum size of request body in bytes (default: 100 MB) |
| `max_multipart_field_bytes` | `int` | — | Maximum size of multipart fields in bytes (default: 100 MB) |

---

### DocxAppProperties

Application properties from docProps/app.xml for DOCX

Contains Word-specific document statistics and metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `application` | `str \| None` | `None` | Application name (e.g., "Microsoft Office Word") |
| `app_version` | `str \| None` | `None` | Application version |
| `template` | `str \| None` | `None` | Template filename |
| `total_time` | `int \| None` | `None` | Total editing time in minutes |
| `pages` | `int \| None` | `None` | Number of pages |
| `words` | `int \| None` | `None` | Number of words |
| `characters` | `int \| None` | `None` | Number of characters (excluding spaces) |
| `characters_with_spaces` | `int \| None` | `None` | Number of characters (including spaces) |
| `lines` | `int \| None` | `None` | Number of lines |
| `paragraphs` | `int \| None` | `None` | Number of paragraphs |
| `company` | `str \| None` | `None` | Company name |
| `doc_security` | `int \| None` | `None` | Document security level |
| `scale_crop` | `bool \| None` | `None` | Scale crop flag |
| `links_up_to_date` | `bool \| None` | `None` | Links up to date flag |
| `shared_doc` | `bool \| None` | `None` | Shared document flag |
| `hyperlinks_changed` | `bool \| None` | `None` | Hyperlinks changed flag |

---

### XlsxAppProperties

Application properties from docProps/app.xml for XLSX

Contains Excel-specific document metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `application` | `str \| None` | `None` | Application name (e.g., "Microsoft Excel") |
| `app_version` | `str \| None` | `None` | Application version |
| `doc_security` | `int \| None` | `None` | Document security level |
| `scale_crop` | `bool \| None` | `None` | Scale crop flag |
| `links_up_to_date` | `bool \| None` | `None` | Links up to date flag |
| `shared_doc` | `bool \| None` | `None` | Shared document flag |
| `hyperlinks_changed` | `bool \| None` | `None` | Hyperlinks changed flag |
| `company` | `str \| None` | `None` | Company name |
| `worksheet_names` | `list[str]` | `[]` | Worksheet names |

---

### PptxAppProperties

Application properties from docProps/app.xml for PPTX

Contains PowerPoint-specific document metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `application` | `str \| None` | `None` | Application name (e.g., "Microsoft Office PowerPoint") |
| `app_version` | `str \| None` | `None` | Application version |
| `total_time` | `int \| None` | `None` | Total editing time in minutes |
| `company` | `str \| None` | `None` | Company name |
| `doc_security` | `int \| None` | `None` | Document security level |
| `scale_crop` | `bool \| None` | `None` | Scale crop flag |
| `links_up_to_date` | `bool \| None` | `None` | Links up to date flag |
| `shared_doc` | `bool \| None` | `None` | Shared document flag |
| `hyperlinks_changed` | `bool \| None` | `None` | Hyperlinks changed flag |
| `slides` | `int \| None` | `None` | Number of slides |
| `notes` | `int \| None` | `None` | Number of notes |
| `hidden_slides` | `int \| None` | `None` | Number of hidden slides |
| `multimedia_clips` | `int \| None` | `None` | Number of multimedia clips |
| `presentation_format` | `str \| None` | `None` | Presentation format (e.g., "Widescreen", "Standard") |
| `slide_titles` | `list[str]` | `[]` | Slide titles |

---

### CoreProperties

Dublin Core metadata from docProps/core.xml

Contains standard metadata fields defined by the Dublin Core standard
and Office-specific extensions.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `title` | `str \| None` | `None` | Document title |
| `subject` | `str \| None` | `None` | Document subject/topic |
| `creator` | `str \| None` | `None` | Document creator/author |
| `keywords` | `str \| None` | `None` | Keywords or tags |
| `description` | `str \| None` | `None` | Document description/abstract |
| `last_modified_by` | `str \| None` | `None` | User who last modified the document |
| `revision` | `str \| None` | `None` | Revision number |
| `created` | `str \| None` | `None` | Creation timestamp (ISO 8601) |
| `modified` | `str \| None` | `None` | Last modification timestamp (ISO 8601) |
| `category` | `str \| None` | `None` | Document category |
| `content_status` | `str \| None` | `None` | Content status (Draft, Final, etc.) |
| `language` | `str \| None` | `None` | Document language |
| `identifier` | `str \| None` | `None` | Unique identifier |
| `version` | `str \| None` | `None` | Document version |
| `last_printed` | `str \| None` | `None` | Last print timestamp (ISO 8601) |

---

### SecurityLimits

Configuration for security limits across extractors.

All limits are intentionally conservative to prevent DoS attacks
while still supporting legitimate documents.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_archive_size` | `int` | `524288000` | Maximum uncompressed size for archives (500 MB) |
| `max_compression_ratio` | `int` | `100` | Maximum compression ratio before flagging as potential bomb (100:1) |
| `max_files_in_archive` | `int` | `10000` | Maximum number of files in archive (10,000) |
| `max_nesting_depth` | `int` | `1024` | Maximum nesting depth for structures (100) |
| `max_entity_length` | `int` | `1048576` | Maximum length of any single XML entity / attribute / token (1 MiB). This is a per-token cap, NOT a total cap — billion-laughs class attacks where a single entity expands to hundreds of MB are caught here, while normal long text content (a paragraph, a CDATA block) is caught by `max_content_size` instead. |
| `max_content_size` | `int` | `104857600` | Maximum string growth per document (100 MB) |
| `max_iterations` | `int` | `10000000` | Maximum iterations per operation |
| `max_xml_depth` | `int` | `1024` | Maximum XML depth (100 levels) |
| `max_table_cells` | `int` | `100000` | Maximum cells per table (100,000) |

---

### TokenReductionConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `level` | `ReductionLevel` | `ReductionLevel.MODERATE` | Level (reduction level) |
| `language_hint` | `str \| None` | `None` | Language hint |
| `preserve_markdown` | `bool` | `False` | Preserve markdown |
| `preserve_code` | `bool` | `True` | Preserve code |
| `semantic_threshold` | `float` | `0.3` | Semantic threshold |
| `enable_parallel` | `bool` | `True` | Enable parallel |
| `use_simd` | `bool` | `True` | Use simd |
| `custom_stopwords` | `dict[str, list[str]] \| None` | `None` | Custom stopwords |
| `preserve_patterns` | `list[str]` | `[]` | Preserve patterns |
| `target_reduction` | `float \| None` | `None` | Target reduction |
| `enable_semantic_clustering` | `bool` | `False` | Enable semantic clustering |

---

### DocumentStructure

Top-level structured document representation.

A flat array of nodes with index-based parent/child references forming a tree.
Root-level nodes have `parent: None`. Use `body_roots()` and `furniture_roots()`
to iterate over top-level content by layer.

## Validation

Call `validate()` after construction to verify all node indices are in bounds
and parent-child relationships are bidirectionally consistent.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `nodes` | `list[DocumentNode]` | `[]` | All nodes in document/reading order. |
| `source_format` | `str \| None` | `None` | Origin format identifier (e.g. "docx", "pptx", "html", "pdf"). Allows renderers to apply format-aware heuristics when converting the document tree to output formats. |
| `relationships` | `list[DocumentRelationship]` | `[]` | Resolved relationships between nodes (footnote refs, citations, anchor links, etc.). Populated during derivation from the internal document representation. Empty when no relationships are detected. |
| `node_types` | `list[str]` | `[]` | Sorted, deduplicated list of node type names present in this document. Each value is the snake_case `node_type` tag of the corresponding `NodeContent` variant (e.g. `"paragraph"`, `"heading"`, `"table"`, …). Computed from `nodes` via `DocumentStructure.finalize_node_types`. Empty until that method is called (internal construction paths call it at the end of derivation). |

---

### TableGrid

Structured table grid with cell-level metadata.

Stores row/column dimensions and a flat list of cells with position info.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `rows` | `int` | — | Number of rows in the table. |
| `cols` | `int` | — | Number of columns in the table. |
| `cells` | `list[GridCell]` | `[]` | All cells in row-major order. |

---

### ExtractionResult

General extraction result used by the core extraction API.

This is the main result type returned by all extraction functions.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `str` | — | The extracted text content |
| `mime_type` | `str` | — | The detected MIME type |
| `metadata` | `Metadata` | — | Document metadata |
| `extraction_method` | `ExtractionMethod \| None` | `None` | Extraction strategy used to produce the returned text. Populated when the extractor can reliably distinguish native text extraction, OCR-only extraction, or mixed native/OCR output. |
| `tables` | `list[Table]` | `[]` | Tables extracted from the document |
| `detected_languages` | `list[str] \| None` | `[]` | Detected languages |
| `chunks` | `list[Chunk] \| None` | `[]` | Text chunks when chunking is enabled. When chunking configuration is provided, the content is split into overlapping chunks for efficient processing. Each chunk contains the text, optional embeddings (if enabled), and metadata about its position. |
| `images` | `list[ExtractedImage] \| None` | `[]` | Extracted images from the document. When image extraction is enabled via `ImageExtractionConfig`, this field contains all images found in the document with their raw data and metadata. Each image may optionally contain a nested `ocr_result` if OCR was performed. |
| `pages` | `list[PageContent] \| None` | `[]` | Per-page content when page extraction is enabled. When page extraction is configured, the document is split into per-page content with tables and images mapped to their respective pages. |
| `elements` | `list[Element] \| None` | `[]` | Semantic elements when element-based result format is enabled. When result_format is set to ElementBased, this field contains semantic elements with type classification, unique identifiers, and metadata for Unstructured-compatible element-based processing. |
| `djot_content` | `DjotContent \| None` | `None` | Rich Djot content structure (when extracting Djot documents). When extracting Djot documents with structured extraction enabled, this field contains the full semantic structure including: - Block-level elements with nesting - Inline formatting with attributes - Links, images, footnotes - Math expressions - Complete attribute information The `content` field still contains plain text for backward compatibility. Always `None` for non-Djot documents. |
| `ocr_elements` | `list[OcrElement] \| None` | `[]` | OCR elements with full spatial and confidence metadata. When OCR is performed with element extraction enabled, this field contains the structured representation of detected text including: - Bounding geometry (rectangles or quadrilaterals) - Confidence scores (detection and recognition) - Rotation information - Hierarchical relationships (Tesseract only) This field preserves all metadata that would otherwise be lost when converting to plain text or markdown output formats. Only populated when `OcrElementConfig.include_elements` is true. |
| `document` | `DocumentStructure \| None` | `None` | Structured document tree (when document structure extraction is enabled). When `include_document_structure` is true in `ExtractionConfig`, this field contains the full hierarchical representation of the document including: - Heading-driven section nesting - Table grids with cell-level metadata - Content layer classification (body, header, footer, footnote) - Inline text annotations (formatting, links) - Bounding boxes and page numbers Independent of `result_format` — can be combined with Unified or ElementBased. |
| `extracted_keywords` | `list[Keyword] \| None` | `[]` | Extracted keywords when keyword extraction is enabled. When keyword extraction (RAKE or YAKE) is configured, this field contains the extracted keywords with scores, algorithm info, and position data. Previously stored in `metadata.additional["keywords"]`. |
| `quality_score` | `float \| None` | `None` | Document quality score from quality analysis. A value between 0.0 and 1.0 indicating the overall text quality. Previously stored in `metadata.additional["quality_score"]`. |
| `processing_warnings` | `list[ProcessingWarning]` | `[]` | Non-fatal warnings collected during processing pipeline stages. Captures errors from optional pipeline features (embedding, chunking, language detection, output formatting) that don't prevent extraction but may indicate degraded results. Previously stored as individual keys in `metadata.additional`. |
| `annotations` | `list[PdfAnnotation] \| None` | `[]` | PDF annotations extracted from the document. When annotation extraction is enabled via `PdfConfig.extract_annotations`, this field contains text notes, highlights, links, stamps, and other annotations found in PDF documents. |
| `children` | `list[ArchiveEntry] \| None` | `[]` | Nested extraction results from archive contents. When extracting archives, each processable file inside produces its own full extraction result. Set to `None` for non-archive formats. Use `max_archive_depth` in config to control recursion depth. |
| `uris` | `list[ExtractedUri] \| None` | `[]` | URIs/links discovered during document extraction. Contains hyperlinks, image references, citations, email addresses, and other URI-like references found in the document. Always extracted when present in the source document. |
| `revisions` | `list[DocumentRevision] \| None` | `[]` | Tracked changes embedded in the source document. Populated by per-format extractors that understand change-tracking metadata (DOCX `w:ins`/`w:del`/`w:rPrChange`, ODT `text:change-*`, …). Every extractor defaults to `None` until its format-specific implementation is added. Extractors that do populate this field follow the "accepted-changes" convention: inserted text is present in `content`, deleted text is absent — the revision list is the separate audit trail. |
| `structured_output` | `dict[str, Any] \| None` | `None` | Structured extraction output from LLM-based JSON schema extraction. When `structured_extraction` is configured in `ExtractionConfig`, the extracted document content is sent to a VLM with the provided JSON schema. The response is parsed and stored here as a JSON value matching the schema. |
| `code_intelligence` | `dict[str, Any] \| None` | `None` | Code intelligence results from tree-sitter analysis. Populated when extracting source code files with the `tree-sitter` feature. Contains metrics, structural analysis, imports/exports, comments, docstrings, symbols, diagnostics, and optionally chunked code segments. Stored as an opaque JSON value so that all language bindings (Go, Java, C#, …) can deserialize it as a raw JSON object rather than a typed struct. The underlying type is `tree_sitter_language_pack.ProcessResult`. |
| `llm_usage` | `list[LlmUsage] \| None` | `[]` | LLM token usage and cost data for all LLM calls made during this extraction. Contains one entry per LLM call. Multiple entries are produced when VLM OCR, structured extraction, or LLM embeddings run during the same extraction. `None` when no LLM was used. |
| `entities` | `list[Entity] \| None` | `[]` | Named entities detected in `content` by the NER post-processor. `None` when no NER backend is configured. Populated by the gline-rs ONNX backend or the LLM-driven backend (see `crates/kreuzberg/src/text/ner/`). |
| `summary` | `DocumentSummary \| None` | `None` | Summary of `content` produced by the summarisation post-processor. `None` when summarisation is not configured. Populated by the TextRank extractive backend (deterministic, no external service) or by the liter-llm-driven abstractive backend. |
| `translation` | `Translation \| None` | `None` | Translation of `content` produced by the translation post-processor. `None` when translation is not configured. |
| `page_classifications` | `list[PageClassification] \| None` | `[]` | Per-page classifications produced by the page-classification post-processor. `None` when classification is not configured. |
| `redaction_report` | `RedactionReport \| None` | `None` | Audit report of redactions applied by the redaction post-processor. The redaction processor rewrites `content`, `formatted_content`, every chunk's text, and the textual fields of `entities` / `summary` / `translation` / `page_classifications` in place. This report describes what was found and how it was replaced. `None` when redaction is not configured. |
| `formatted_content` | `str \| None` | `None` | Pre-rendered content in the requested output format. Populated during `derive_extraction_result` before tree derivation consumes element data. `apply_output_format` swaps this into `content` at the end of the pipeline, after post-processors have operated on plain text. |
| `ocr_internal_document` | `str \| None` | `None` | Structured hOCR document for the OCR+layout pipeline. When tesseract produces hOCR output, the parsed `InternalDocument` carries paragraph structure with bounding boxes and confidence scores. The layout classification step enriches these elements before final rendering. |

---

### LlmUsage

Token usage and cost data for a single LLM call made during extraction.

Populated when VLM OCR, structured extraction, or LLM-based embeddings
are used. Multiple entries may be present when multiple LLM calls occur
within one extraction (e.g. VLM OCR + structured extraction).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `str` | — | The LLM model identifier (e.g. "openai/gpt-4o", "anthropic/claude-sonnet-4-20250514"). |
| `source` | `str` | — | The pipeline stage that triggered this LLM call (e.g. "vlm_ocr", "structured_extraction", "embeddings"). |
| `input_tokens` | `int \| None` | `None` | Number of input/prompt tokens consumed. |
| `output_tokens` | `int \| None` | `None` | Number of output/completion tokens generated. |
| `total_tokens` | `int \| None` | `None` | Total tokens (input + output). |
| `estimated_cost` | `float \| None` | `None` | Estimated cost in USD based on the provider's published pricing. |
| `finish_reason` | `str \| None` | `None` | Why the model stopped generating (e.g. "stop", "length", "content_filter"). |

---

### ExtractedImage

Extracted image from a document.

Contains raw image data, metadata, and optional nested OCR results.
Raw bytes allow cross-language compatibility - users can convert to
PIL.Image (Python), Sharp (Node.js), or other formats as needed.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `data` | `bytes` | — | Raw image data (PNG, JPEG, WebP, etc. bytes). Uses `bytes.Bytes` for cheap cloning of large buffers. |
| `format` | `str` | — | Image format (e.g., "jpeg", "png", "webp") Uses Cow<'static, str> to avoid allocation for static literals. |
| `image_index` | `int` | — | Zero-indexed position of this image in the document/page |
| `page_number` | `int \| None` | `None` | Page/slide number where image was found (1-indexed) |
| `width` | `int \| None` | `None` | Image width in pixels |
| `height` | `int \| None` | `None` | Image height in pixels |
| `colorspace` | `str \| None` | `None` | Colorspace information (e.g., "RGB", "CMYK", "Gray") |
| `bits_per_component` | `int \| None` | `None` | Bits per color component (e.g., 8, 16) |
| `is_mask` | `bool` | — | Whether this image is a mask image |
| `description` | `str \| None` | `None` | Optional description of the image |
| `ocr_result` | `ExtractionResult \| None` | `None` | Nested OCR extraction result (if image was OCRed) When OCR is performed on this image, the result is embedded here rather than in a separate collection, making the relationship explicit. |
| `bounding_box` | `BoundingBox \| None` | `None` | Bounding box of the image on the page (PDF coordinates: x0=left, y0=bottom, x1=right, y1=top). Only populated for PDF-extracted images when position data is available from the PDF extractor. |
| `source_path` | `str \| None` | `None` | Original source path of the image within the document archive (e.g., "media/image1.png" in DOCX). Used for rendering image references when the binary data is not extracted. |
| `image_kind` | `ImageKind \| None` | `None` | Heuristic classification of what this image likely depicts. `None` if classification was disabled or inconclusive. |
| `kind_confidence` | `float \| None` | `None` | Confidence score for `image_kind`, in the range 0.0 to 1.0. |
| `cluster_id` | `int \| None` | `None` | Identifier shared across images that form a single logical figure (e.g. all raster tiles of one technical drawing). `None` for singletons. |
| `caption` | `str \| None` | `None` | VLM-generated caption describing the image, when captioning is configured. Populated by the captioning post-processor (`crates/kreuzberg/src/plugins/processor/builtin/captioning.rs`), which routes each image through `crate.llm.region_extractor.extract_region_with_vlm` in caption mode. `None` when captioning is disabled or the VLM declined to caption. |
| `qr_codes` | `list[QrCode] \| None` | `[]` | QR codes decoded from this image, when QR detection is enabled. Populated by the QR post-processor (`crates/kreuzberg/src/extractors/qr.rs`) via the pure-Rust `rqrr` decoder. `None` when QR detection is disabled; an empty `Some(vec![])` when detection ran but found nothing. |

---

### BoundingBox

Bounding box coordinates for element positioning.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `x0` | `float` | — | Left x-coordinate |
| `y0` | `float` | — | Bottom y-coordinate |
| `x1` | `float` | — | Right x-coordinate |
| `y1` | `float` | — | Top y-coordinate |

---

### ImagePreprocessingConfig

Image preprocessing configuration for OCR.

These settings control how images are preprocessed before OCR to improve
text recognition quality. Different preprocessing strategies work better
for different document types.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `target_dpi` | `int` | `300` | Target DPI for the image (300 is standard, 600 for small text). |
| `auto_rotate` | `bool` | `True` | Auto-detect and correct image rotation. |
| `deskew` | `bool` | `True` | Correct skew (tilted images). |
| `denoise` | `bool` | `False` | Remove noise from the image. |
| `contrast_enhance` | `bool` | `False` | Enhance contrast for better text visibility. |
| `binarization_method` | `str` | `"otsu"` | Binarization method: "otsu", "sauvola", "adaptive". |
| `invert_colors` | `bool` | `False` | Invert colors (white text on black → black on white). |

---

### TesseractConfig

Tesseract OCR configuration.

Provides fine-grained control over Tesseract OCR engine parameters.
Most users can use the defaults, but these settings allow optimization
for specific document types (invoices, handwriting, etc.).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `language` | `str` | `"eng"` | Language code (e.g., "eng", "deu", "fra") |
| `psm` | `int` | `3` | Page Segmentation Mode (0-13). Common values: - 3: Fully automatic page segmentation (native default) - 6: Assume a single uniform block of text (WASM default — avoids layout-analysis hang) - 11: Sparse text with no particular order |
| `output_format` | `str` | `"markdown"` | Output format ("text" or "markdown") |
| `oem` | `int` | `3` | OCR Engine Mode (0-3). - 0: Legacy engine only - 1: Neural nets (LSTM) only (usually best) - 2: Legacy + LSTM - 3: Default (based on what's available) |
| `min_confidence` | `float` | `0` | Minimum confidence threshold (0.0-100.0). Words with confidence below this threshold may be rejected or flagged. |
| `preprocessing` | `ImagePreprocessingConfig \| None` | `None` | Image preprocessing configuration. Controls how images are preprocessed before OCR. Can significantly improve quality for scanned documents or low-quality images. |
| `enable_table_detection` | `bool` | `True` | Enable automatic table detection and reconstruction |
| `table_min_confidence` | `float` | `0` | Minimum confidence threshold for table detection (0.0-1.0) |
| `table_column_threshold` | `int` | `50` | Column threshold for table detection (pixels) |
| `table_row_threshold_ratio` | `float` | `0.5` | Row threshold ratio for table detection (0.0-1.0) |
| `use_cache` | `bool` | `True` | Enable OCR result caching |
| `classify_use_pre_adapted_templates` | `bool` | `True` | Use pre-adapted templates for character classification |
| `language_model_ngram_on` | `bool` | `False` | Enable N-gram language model |
| `tessedit_dont_blkrej_good_wds` | `bool` | `True` | Don't reject good words during block-level processing |
| `tessedit_dont_rowrej_good_wds` | `bool` | `True` | Don't reject good words during row-level processing |
| `tessedit_enable_dict_correction` | `bool` | `True` | Enable dictionary correction |
| `tessedit_char_whitelist` | `str` | `""` | Whitelist of allowed characters (empty = all allowed) |
| `tessedit_char_blacklist` | `str` | `""` | Blacklist of forbidden characters (empty = none forbidden) |
| `tessedit_use_primary_params_model` | `bool` | `True` | Use primary language params model |
| `textord_space_size_is_variable` | `bool` | `True` | Variable-width space detection |
| `thresholding_method` | `bool` | `False` | Use adaptive thresholding method |

---

### Metadata

Extraction result metadata.

Contains common fields applicable to all formats, format-specific metadata
via a discriminated union, and additional custom fields from postprocessors.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `title` | `str \| None` | `None` | Document title |
| `subject` | `str \| None` | `None` | Document subject or description |
| `authors` | `list[str] \| None` | `[]` | Primary author(s) - always Vec for consistency |
| `keywords` | `list[str] \| None` | `[]` | Keywords/tags - always Vec for consistency |
| `language` | `str \| None` | `None` | Primary language (ISO 639 code) |
| `created_at` | `str \| None` | `None` | Creation timestamp (ISO 8601 format) |
| `modified_at` | `str \| None` | `None` | Last modification timestamp (ISO 8601 format) |
| `created_by` | `str \| None` | `None` | User who created the document |
| `modified_by` | `str \| None` | `None` | User who last modified the document |
| `pages` | `PageStructure \| None` | `None` | Page/slide/sheet structure with boundaries |
| `format` | `FormatMetadata \| None` | `None` | Format-specific metadata (discriminated union) Contains detailed metadata specific to the document format. Serialized as a nested `"format"` object with a `format_type` discriminator field. |
| `image_preprocessing` | `ImagePreprocessingMetadata \| None` | `None` | Image preprocessing metadata (when OCR preprocessing was applied) |
| `json_schema` | `dict[str, Any] \| None` | `None` | JSON schema (for structured data extraction) |
| `error` | `ErrorMetadata \| None` | `None` | Error metadata (for batch operations) |
| `extraction_duration_ms` | `int \| None` | `None` | Extraction duration in milliseconds (for benchmarking). This field is populated by batch extraction to provide per-file timing information. It's `None` for single-file extraction (which uses external timing). |
| `category` | `str \| None` | `None` | Document category (from frontmatter or classification). |
| `tags` | `list[str] \| None` | `[]` | Document tags (from frontmatter). |
| `document_version` | `str \| None` | `None` | Document version string (from frontmatter). |
| `abstract_text` | `str \| None` | `None` | Abstract or summary text (from frontmatter). |
| `output_format` | `str \| None` | `None` | Output format identifier (e.g., "markdown", "html", "text"). Set by the output format pipeline stage when format conversion is applied. Previously stored in `metadata.additional["output_format"]`. |
| `ocr_used` | `bool` | — | Whether OCR was used during extraction. Set to `True` whenever the extraction pipeline ran an OCR backend (Tesseract, PaddleOCR, VLM, etc.) and used that output as the primary or fallback text. `False` means native text extraction was used exclusively. |
| `additional` | `dict[str, dict[str, Any]]` | `{}` | Additional custom fields from postprocessors. Serialized as a nested `"additional"` object (not flattened at root level). Uses `Cow<'static, str>` keys so static string keys avoid allocation. |

---

### ExcelMetadata

Excel/spreadsheet format metadata.

Identifies the document as a spreadsheet source via the `FormatMetadata.Excel`
discriminant. Sheet count and sheet names are stored inside this struct.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `sheet_count` | `int \| None` | `None` | Number of sheets in the workbook. |
| `sheet_names` | `list[str] \| None` | `[]` | Names of all sheets in the workbook. |

---

### EmailMetadata

Email metadata extracted from .eml and .msg files.

Includes sender/recipient information, message ID, and attachment list.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `from_email` | `str \| None` | `None` | Sender's email address |
| `from_name` | `str \| None` | `None` | Sender's display name |
| `to_emails` | `list[str]` | `[]` | Primary recipients |
| `cc_emails` | `list[str]` | `[]` | CC recipients |
| `bcc_emails` | `list[str]` | `[]` | BCC recipients |
| `message_id` | `str \| None` | `None` | Message-ID header value |
| `attachments` | `list[str]` | `[]` | List of attachment filenames |

---

### ArchiveMetadata

Archive (ZIP/TAR/7Z) metadata.

Extracted from compressed archive files containing file lists and size information.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `format` | `str` | — | Archive format ("ZIP", "TAR", "7Z", etc.) |
| `file_count` | `int` | — | Total number of files in the archive |
| `file_list` | `list[str]` | `[]` | List of file paths within the archive |
| `total_size` | `int` | — | Total uncompressed size in bytes |
| `compressed_size` | `int \| None` | `None` | Compressed size in bytes (if available) |

---

### ImageMetadata

Image metadata extracted from image files.

Includes dimensions, format, and EXIF data.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `width` | `int` | — | Image width in pixels |
| `height` | `int` | — | Image height in pixels |
| `format` | `str` | — | Image format (e.g., "PNG", "JPEG", "TIFF") |
| `exif` | `dict[str, str]` | `{}` | EXIF metadata tags |

---

### XmlMetadata

XML metadata extracted during XML parsing.

Provides statistics about XML document structure.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `element_count` | `int` | — | Total number of XML elements processed |
| `unique_elements` | `list[str]` | `[]` | List of unique element tag names (sorted) |

---

### TextMetadata

Text/Markdown metadata.

Extracted from plain text and Markdown files. Includes word counts and,
for Markdown, structural elements like headers and links.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `line_count` | `int` | — | Number of lines in the document |
| `word_count` | `int` | — | Number of words |
| `character_count` | `int` | — | Number of characters |
| `headers` | `list[str] \| None` | `[]` | Markdown headers (headings text only, for Markdown files) |
| `links` | `list[list[str]] \| None` | `[]` | Markdown links as (text, url) tuples (for Markdown files) |
| `code_blocks` | `list[list[str]] \| None` | `[]` | Code blocks as (language, code) tuples (for Markdown files) |

---

### HtmlMetadata

HTML metadata extracted from HTML documents.

Includes document-level metadata, Open Graph data, Twitter Card metadata,
and extracted structural elements (headers, links, images, structured data).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `title` | `str \| None` | `None` | Document title from `<title>` tag |
| `description` | `str \| None` | `None` | Document description from `<meta name="description">` tag |
| `keywords` | `list[str]` | `[]` | Document keywords from `<meta name="keywords">` tag, split on commas |
| `author` | `str \| None` | `None` | Document author from `<meta name="author">` tag |
| `canonical_url` | `str \| None` | `None` | Canonical URL from `<link rel="canonical">` tag |
| `base_href` | `str \| None` | `None` | Base URL from `<base href="">` tag for resolving relative URLs |
| `language` | `str \| None` | `None` | Document language from `lang` attribute |
| `text_direction` | `TextDirection \| None` | `None` | Document text direction from `dir` attribute |
| `open_graph` | `dict[str, str]` | `{}` | Open Graph metadata (og:* properties) for social media Keys like "title", "description", "image", "url", etc. |
| `twitter_card` | `dict[str, str]` | `{}` | Twitter Card metadata (twitter:* properties) Keys like "card", "site", "creator", "title", "description", "image", etc. |
| `meta_tags` | `dict[str, str]` | `{}` | Additional meta tags not covered by specific fields Keys are meta name/property attributes, values are content |
| `headers` | `list[HeaderMetadata]` | `[]` | Extracted header elements with hierarchy |
| `links` | `list[LinkMetadata]` | `[]` | Extracted hyperlinks with type classification |
| `images` | `list[ImageMetadataType]` | `[]` | Extracted images with source and dimensions |
| `structured_data` | `list[StructuredData]` | `[]` | Extracted structured data blocks |

---

### OcrMetadata

OCR processing metadata.

Captures information about OCR processing configuration and results.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `language` | `str` | — | OCR language code(s) used |
| `psm` | `int` | — | Tesseract Page Segmentation Mode (PSM) |
| `output_format` | `str` | — | Output format (e.g., "text", "hocr") |
| `table_count` | `int` | — | Number of tables detected |
| `table_rows` | `int \| None` | `None` | Table rows |
| `table_cols` | `int \| None` | `None` | Table cols |

---

### PptxMetadata

PowerPoint presentation metadata.

Extracted from PPTX files containing slide counts and presentation details.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `slide_count` | `int` | — | Total number of slides in the presentation |
| `slide_names` | `list[str]` | `[]` | Names of slides (if available) |
| `image_count` | `int \| None` | `None` | Number of embedded images |
| `table_count` | `int \| None` | `None` | Number of tables |

---

### DocxMetadata

Word document metadata.

Extracted from DOCX files using shared Office Open XML metadata extraction.
Integrates with `office_metadata` module for core/app/custom properties.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `core_properties` | `CoreProperties \| None` | `None` | Core properties from docProps/core.xml (Dublin Core metadata) Contains title, creator, subject, keywords, dates, etc. Shared format across DOCX/PPTX/XLSX documents. |
| `app_properties` | `DocxAppProperties \| None` | `None` | Application properties from docProps/app.xml (Word-specific statistics) Contains word count, page count, paragraph count, editing time, etc. DOCX-specific variant of Office application properties. |
| `custom_properties` | `dict[str, dict[str, Any]] \| None` | `{}` | Custom properties from docProps/custom.xml (user-defined properties) Contains key-value pairs defined by users or applications. Values can be strings, numbers, booleans, or dates. |

---

### CsvMetadata

CSV/TSV file metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `row_count` | `int` | — | Number of rows |
| `column_count` | `int` | — | Number of columns |
| `delimiter` | `str \| None` | `None` | Delimiter |
| `has_header` | `bool` | — | Whether header |
| `column_types` | `list[str] \| None` | `[]` | Column types |

---

### BibtexMetadata

BibTeX bibliography metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `entry_count` | `int` | — | Number of entries in the bibliography. |
| `citation_keys` | `list[str]` | `[]` | Citation keys |
| `authors` | `list[str]` | `[]` | Authors |
| `year_range` | `YearRange \| None` | `None` | Year range (year range) |
| `entry_types` | `dict[str, int] \| None` | `{}` | Entry types |

---

### CitationMetadata

Citation file metadata (RIS, PubMed, EndNote).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `citation_count` | `int` | — | Number of citations |
| `format` | `str \| None` | `None` | Format |
| `authors` | `list[str]` | `[]` | Authors |
| `year_range` | `YearRange \| None` | `None` | Year range (year range) |
| `dois` | `list[str]` | `[]` | Dois |
| `keywords` | `list[str]` | `[]` | Keywords |

---

### FictionBookMetadata

FictionBook (FB2) metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `genres` | `list[str]` | `[]` | Genres |
| `sequences` | `list[str]` | `[]` | Sequences |
| `annotation` | `str \| None` | `None` | Annotation |

---

### DbfMetadata

dBASE (DBF) file metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `record_count` | `int` | — | Number of records |
| `field_count` | `int` | — | Number of fields |
| `fields` | `list[DbfFieldInfo]` | `[]` | Fields |

---

### JatsMetadata

JATS (Journal Article Tag Suite) metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `copyright` | `str \| None` | `None` | Copyright |
| `license` | `str \| None` | `None` | License |
| `history_dates` | `dict[str, str]` | `{}` | History dates |
| `contributor_roles` | `list[ContributorRole]` | `[]` | Contributor roles |

---

### EpubMetadata

EPUB metadata (Dublin Core extensions).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `coverage` | `str \| None` | `None` | Coverage |
| `dc_format` | `str \| None` | `None` | Dc format |
| `relation` | `str \| None` | `None` | Relation |
| `source` | `str \| None` | `None` | Source |
| `dc_type` | `str \| None` | `None` | Dc type |
| `cover_image` | `str \| None` | `None` | Cover image |

---

### PstMetadata

Outlook PST archive metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `message_count` | `int` | — | Number of messages |

---

### OcrConfidence

Confidence scores for an OCR element.

Separates detection confidence (how confident that text exists at this location)
from recognition confidence (how confident about the actual text content).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `detection` | `float \| None` | `None` | Detection confidence: how confident the OCR engine is that text exists here. PaddleOCR provides this as `box_score`, Tesseract doesn't have a direct equivalent. Range: 0.0 to 1.0 (or None if not available). |
| `recognition` | `float` | — | Recognition confidence: how confident about the text content. Range: 0.0 to 1.0. |

---

### OcrElement

A unified OCR element representing detected text with full metadata.

This is the primary type for structured OCR output, preserving all information
from both Tesseract and PaddleOCR backends.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `text` | `str` | — | The recognized text content. |
| `geometry` | `OcrBoundingGeometry` | `OcrBoundingGeometry.RECTANGLE` | Bounding geometry (rectangle or quadrilateral). |
| `confidence` | `OcrConfidence` | — | Confidence scores for detection and recognition. |
| `level` | `OcrElementLevel` | `OcrElementLevel.LINE` | Hierarchical level (word, line, block, page). |
| `rotation` | `OcrRotation \| None` | `None` | Rotation information (if detected). |
| `page_number` | `int` | — | Page number (1-indexed). |
| `parent_id` | `str \| None` | `None` | Parent element ID for hierarchical relationships. Only used for Tesseract output which has word -> line -> block hierarchy. |
| `backend_metadata` | `dict[str, dict[str, Any]]` | `{}` | Backend-specific metadata that doesn't fit the unified schema. |

---

### OcrElementConfig

Configuration for OCR element extraction.

Controls how OCR elements are extracted and filtered.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `include_elements` | `bool` | — | Whether to include OCR elements in the extraction result. When true, the `ocr_elements` field in `ExtractionResult` will be populated. |
| `min_level` | `OcrElementLevel` | `OcrElementLevel.LINE` | Minimum hierarchical level to include. Elements below this level (e.g., words when min_level is Line) will be excluded. |
| `min_confidence` | `float` | — | Minimum recognition confidence threshold (0.0-1.0). Elements with confidence below this threshold will be filtered out. |
| `build_hierarchy` | `bool` | — | Whether to build hierarchical relationships between elements. When true, `parent_id` fields will be populated based on spatial containment. Only meaningful for Tesseract output. |

---

### LayoutRegion

A detected layout region on a page.

When layout detection is enabled, each page may have layout regions
identifying different content types (text, pictures, tables, etc.)
with confidence scores and spatial positions.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `class_name` | `str` | — | Layout class name (e.g. "picture", "table", "text", "section_header"). |
| `confidence` | `float` | — | Confidence score from the layout detection model (0.0 to 1.0). |
| `bounding_box` | `BoundingBox` | — | Bounding box in document coordinate space. |
| `area_fraction` | `float` | — | Fraction of the page area covered by this region (0.0 to 1.0). |

---

### RevisionDelta

The content changes that make up a single revision.

For insertions and deletions the `content` field carries the added/removed
lines as `DiffLine.Added` / `DiffLine.Removed` entries. For format
changes, `content` is empty — the property diff is left as a TODO for a
later enrichment pass.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `list[DiffLine]` | `[]` | Line-level content changes for this revision. |
| `table_changes` | `list[CellChange]` | `[]` | Cell-level table changes for this revision. |

---

### Table

Extracted table structure.

Represents a table detected and extracted from a document (PDF, image, etc.).
Tables are converted to both structured cell data and Markdown format.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `cells` | `list[list[str]]` | `[]` | Table cells as a 2D vector (rows × columns) |
| `markdown` | `str` | — | Markdown representation of the table |
| `page_number` | `int` | — | Page number where the table was found (1-indexed) |
| `bounding_box` | `BoundingBox \| None` | `None` | Bounding box of the table on the page (PDF coordinates: x0=left, y0=bottom, x1=right, y1=top). Only populated for PDF-extracted tables when position data is available. |

---

### TableCell

Individual table cell with content and optional styling.

Future extension point for rich table support with cell-level metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `str` | — | Cell content as text |
| `row_span` | `int` | — | Row span (number of rows this cell spans) |
| `col_span` | `int` | — | Column span (number of columns this cell spans) |
| `is_header` | `bool` | — | Whether this is a header cell |

---

### DiffOptions

Options controlling how two `ExtractionResult` values are compared.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `include_metadata` | `bool` | `True` | Include metadata changes in the diff. Default: `True`. |
| `include_embedded` | `bool` | `True` | Include embedded-children changes in the diff. Default: `True`. |
| `max_content_chars` | `int \| None` | `None` | Truncate content to this many characters before diffing. Useful for very large documents where only the first N characters matter. `None` means no truncation. |

---

### YakeParams

YAKE-specific parameters.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `window_size` | `int` | `2` | Window size for co-occurrence analysis (default: 2). Controls the context window for computing co-occurrence statistics. |

---

### RakeParams

RAKE-specific parameters.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `min_word_length` | `int` | `1` | Minimum word length to consider (default: 1). |
| `max_words_per_phrase` | `int` | `3` | Maximum words in a keyword phrase (default: 3). |

---

### KeywordConfig

Keyword extraction configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `algorithm` | `KeywordAlgorithm` | `KeywordAlgorithm.YAKE` | Algorithm to use for extraction. |
| `max_keywords` | `int` | `10` | Maximum number of keywords to extract (default: 10). |
| `min_score` | `float` | `0` | Minimum score threshold (0.0-1.0, default: 0.0). Keywords with scores below this threshold are filtered out. Note: Score ranges differ between algorithms. |
| `ngram_range` | `list[int]` | `[]` | N-gram range for keyword extraction (min, max). (1, 1) = unigrams only (1, 2) = unigrams and bigrams (1, 3) = unigrams, bigrams, and trigrams (default) |
| `language` | `str \| None` | `None` | Language code for stopword filtering (e.g., "en", "de", "fr"). If None, no stopword filtering is applied. |
| `yake_params` | `YakeParams \| None` | `None` | YAKE-specific tuning parameters. |
| `rake_params` | `RakeParams \| None` | `None` | RAKE-specific tuning parameters. |

---

### PaddleOcrConfig

Configuration for PaddleOCR backend.

Configures PaddleOCR text detection and recognition with multi-language support.
Uses a builder pattern for convenient configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `language` | `str` | — | Language code (e.g., "en", "ch", "jpn", "kor", "deu", "fra") |
| `cache_dir` | `str \| None` | `None` | Optional custom cache directory for model files |
| `use_angle_cls` | `bool` | — | Enable angle classification for rotated text (default: false). Can misfire on short text regions, rotating crops incorrectly before recognition. |
| `enable_table_detection` | `bool` | — | Enable table structure detection (default: false) |
| `det_db_thresh` | `float` | — | Database threshold for text detection (default: 0.3) Range: 0.0-1.0, higher values require more confident detections |
| `det_db_box_thresh` | `float` | — | Box threshold for text bounding box refinement (default: 0.5) Range: 0.0-1.0 |
| `det_db_unclip_ratio` | `float` | — | Unclip ratio for expanding text bounding boxes (default: 1.6) Controls the expansion of detected text regions |
| `det_limit_side_len` | `int` | — | Maximum side length for detection image (default: 960) Larger images may be resized to this limit for faster inference |
| `rec_batch_num` | `int` | — | Batch size for recognition inference (default: 6) Number of text regions to process simultaneously |
| `padding` | `int` | — | Padding in pixels added around the image before detection (default: 10). Large values can include surrounding content like table gridlines. |
| `drop_score` | `float` | — | Minimum recognition confidence score for text lines (default: 0.5). Text regions with recognition confidence below this threshold are discarded. Matches PaddleOCR Python's `drop_score` parameter. Range: 0.0-1.0 |
| `model_tier` | `str` | — | Model tier controlling detection/recognition model size and accuracy trade-off. - `"mobile"` (default): Lightweight models (~4.5MB detection, ~16.5MB recognition), fast download and inference - `"server"`: Large, high-accuracy models (~88MB detection, ~84MB recognition), best for GPU or complex documents |

---

### PdfMetadata

PDF-specific metadata.

Contains metadata fields specific to PDF documents that are not in the common
`Metadata` structure. Common fields like title, authors, keywords, and dates
are at the `Metadata` level.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `pdf_version` | `str \| None` | `None` | PDF version (e.g., "1.7", "2.0") |
| `producer` | `str \| None` | `None` | PDF producer (application that created the PDF) |
| `is_encrypted` | `bool \| None` | `None` | Whether the PDF is encrypted/password-protected |
| `width` | `int \| None` | `None` | First page width in points (1/72 inch) |
| `height` | `int \| None` | `None` | First page height in points (1/72 inch) |
| `page_count` | `int \| None` | `None` | Total number of pages in the PDF document |

---

### Enums

#### ChunkSizing

How chunk size is measured.

Defaults to `Characters` (Unicode character count). When using token-based sizing,
chunks are sized by token count according to the specified tokenizer.

Token-based sizing uses HuggingFace tokenizers loaded at runtime. Any tokenizer
available on HuggingFace Hub can be used, including OpenAI-compatible tokenizers
(e.g., `Xenova/gpt-4o`, `Xenova/cl100k_base`).

| Variant | Wire value | Description |
|---------|------------|-------------|
| `Characters` | `characters` | Size measured in Unicode characters (default). |
| `Tokenizer` | `tokenizer` | Size measured in tokens from a HuggingFace tokenizer. — Fields: `model`: `String`, `cache_dir`: `PathBuf` |

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

| Variant | Wire value | Description |
|---------|------------|-------------|
| `Text` | `text` | Text format |
| `Markdown` | `markdown` | Markdown format |
| `Yaml` | `yaml` | Yaml format |
| `Semantic` | `semantic` | Semantic |

---

#### CodeContentMode

Content rendering mode for code extraction.

Controls how extracted code content is represented in the `content` field
of `ExtractionResult`.

| Variant | Wire value | Description |
|---------|------------|-------------|
| `Chunks` | `chunks` | Use TSLP semantic chunks as content (default). |
| `Raw` | `raw` | Use raw source code as content. |
| `Structure` | `structure` | Emit function/class headings + docstrings (no code bodies). |

---

#### DiffLine

A single line in a unified-diff hunk.

Defined here (rather than only in `crate.diff`) so `RevisionDelta` can
reference it unconditionally, without requiring the `diff` Cargo feature.
`crate.diff` re-exports this type verbatim.

| Variant | Wire value | Description |
|---------|------------|-------------|
| `Context` | `context` | Unchanged context line. — Fields: `_0`: `String` |
| `Added` | `added` | Line added in the "after" version. — Fields: `_0`: `String` |
| `Removed` | `removed` | Line removed from the "before" version. — Fields: `_0`: `String` |

---

#### EmbeddingModelType

Embedding model types supported by Kreuzberg.

| Variant | Wire value | Description |
|---------|------------|-------------|
| `Preset` | `preset` | Use a preset model configuration (recommended) — Fields: `name`: `String` |
| `Custom` | `custom` | Use a custom ONNX model from HuggingFace — Fields: `model_id`: `String`, `dimensions`: `usize` |
| `Llm` | `llm` | Provider-hosted embedding model via liter-llm. Uses the model specified in the nested `LlmConfig` (e.g., `"openai/text-embedding-3-small"`). — Fields: `llm`: `LlmConfig` |
| `Plugin` | `plugin` | In-process embedding backend registered via the plugin system. The caller registers an `EmbeddingBackend` once (e.g. a wrapper around an already-loaded `llama-cpp-python`, `sentence-transformers`, or tuned ONNX model), then references it by name in config. Kreuzberg calls back into the registered backend during chunking and standalone embed requests — no HuggingFace download, no ONNX Runtime requirement, no HTTP sidecar. When this variant is selected, only the following `EmbeddingConfig` fields apply: `normalize` (post-call L2 normalization) and `max_embed_duration_secs` (dispatcher timeout). Model-loading fields (`batch_size`, `cache_dir`, `show_download_progress`, `acceleration`) are ignored — the host owns the model lifecycle. Semantic chunking falls back to `ChunkingConfig.max_characters` when this variant is used, since there is no preset to look a chunk-size ceiling up against — size your context window via `max_characters` directly. See `register_embedding_backend`. — Fields: `name`: `String` |

---

#### EntityCategory

Standard entity categories produced by built-in NER backends.

The `Custom(String)` variant lets caller-supplied categories (e.g. LLM
schemas) flow through without losing fidelity to the consumer.

| Variant | Wire value | Description |
|---------|------------|-------------|
| `Person` | `person` | Person |
| `Organization` | `organization` | Organization |
| `Location` | `location` | Location |
| `Date` | `date` | Date |
| `Time` | `time` | Time |
| `Money` | `money` | Money |
| `Percent` | `percent` | Percent |
| `Email` | `email` | Email |
| `Phone` | `phone` | Phone |
| `Url` | `url` | Url |
| `Custom` | `custom` | Custom — Fields: `_0`: `String` |

---

#### ExecutionProviderType

ONNX Runtime execution provider type.

Determines which hardware backend is used for model inference.
`Auto` (default) selects the best available provider per platform.

| Variant | Wire value | Description |
|---------|------------|-------------|
| `Auto` | `auto` | Auto-select: CoreML on macOS, CUDA on Linux, CPU elsewhere. |
| `Cpu` | `cpu` | CPU execution provider (always available). |
| `CoreMl` | `coreml` | Apple CoreML (macOS/iOS Neural Engine + GPU). |
| `Cuda` | `cuda` | NVIDIA CUDA GPU acceleration. |
| `TensorRt` | `tensorrt` | NVIDIA TensorRT (optimized CUDA inference). |

---

#### ExtractionMethod

How the extracted text was produced.

| Variant | Wire value | Description |
|---------|------------|-------------|
| `Native` | `native` | Native |
| `Ocr` | `ocr` | Ocr |
| `Mixed` | `mixed` | Mixed |

---

#### FormatMetadata

Format-specific metadata (discriminated union).

Only one format type can exist per extraction result. This provides
type-safe, clean metadata without nested optionals.

| Variant | Wire value | Description |
|---------|------------|-------------|
| `Pdf` | `pdf` | Pdf format — Fields: `_0`: `PdfMetadata` |
| `Docx` | `docx` | Docx format — Fields: `_0`: `DocxMetadata` |
| `Excel` | `excel` | Excel — Fields: `_0`: `ExcelMetadata` |
| `Email` | `email` | Email — Fields: `_0`: `EmailMetadata` |
| `Pptx` | `pptx` | Pptx format — Fields: `_0`: `PptxMetadata` |
| `Archive` | `archive` | Archive — Fields: `_0`: `ArchiveMetadata` |
| `Image` | `image` | Image element — Fields: `_0`: `ImageMetadata` |
| `Xml` | `xml` | Xml format — Fields: `_0`: `XmlMetadata` |
| `Text` | `text` | Text format — Fields: `_0`: `TextMetadata` |
| `Html` | `html` | Preserve as HTML `<mark>` tags — Fields: `_0`: `HtmlMetadata` |
| `Ocr` | `ocr` | Ocr — Fields: `_0`: `OcrMetadata` |
| `Csv` | `csv` | Csv format — Fields: `_0`: `CsvMetadata` |
| `Bibtex` | `bibtex` | Bibtex — Fields: `_0`: `BibtexMetadata` |
| `Citation` | `citation` | Citation — Fields: `_0`: `CitationMetadata` |
| `FictionBook` | `fiction_book` | Fiction book — Fields: `_0`: `FictionBookMetadata` |
| `Dbf` | `dbf` | Dbf — Fields: `_0`: `DbfMetadata` |
| `Jats` | `jats` | Jats — Fields: `_0`: `JatsMetadata` |
| `Epub` | `epub` | Epub format — Fields: `_0`: `EpubMetadata` |
| `Pst` | `pst` | Pst — Fields: `_0`: `PstMetadata` |
| `Code` | `code` | Code — Fields: `_0`: `String` |

---

#### HtmlTheme

Built-in HTML theme selection.

| Variant | Wire value | Description |
|---------|------------|-------------|
| `Default` | `default` | Sensible defaults: system font stack, neutral colours, readable line measure. CSS custom properties (`--kb-*`) are all defined so user CSS can override individual values. |
| `GitHub` | `github` | GitHub Markdown-inspired palette and spacing. |
| `Dark` | `dark` | Dark background, light text. |
| `Light` | `light` | Minimal light theme with generous whitespace. |
| `Unstyled` | `unstyled` | No built-in stylesheet emitted. CSS custom properties are still defined on `:root` so user stylesheets can reference `var(--kb-*)` tokens. |

---

#### ImageKind

Heuristic classification of what an image likely depicts.

| Variant | Wire value | Description |
|---------|------------|-------------|
| `Photograph` | `photograph` | Photographic image (natural scene, photograph) |
| `Diagram` | `diagram` | Technical or schematic diagram |
| `Chart` | `chart` | Chart, graph, or plot |
| `Drawing` | `drawing` | Freehand or technical drawing |
| `TextBlock` | `text_block` | Text-heavy image (scanned text, document) |
| `Decoration` | `decoration` | Decorative element or border |
| `Logo` | `logo` | Logo or brand mark |
| `Icon` | `icon` | Small icon |
| `TileFragment` | `tile_fragment` | Fragment of a larger tiled image (tile of a technical drawing) |
| `Mask` | `mask` | Mask or transparency map |
| `PageRaster` | `page_raster` | Full-page render produced during OCR preprocessing; used as a citation thumbnail. |
| `Unknown` | `unknown` | Could not classify with reasonable confidence |

---

#### KeywordAlgorithm

Keyword algorithm selection.

| Variant | Wire value | Description |
|---------|------------|-------------|
| `Yake` | `yake` | YAKE (Yet Another Keyword Extractor) - statistical approach |
| `Rake` | `rake` | RAKE (Rapid Automatic Keyword Extraction) - co-occurrence based |

---

#### NerBackendKind

NER backend selector.

| Variant | Wire value | Description |
|---------|------------|-------------|
| `Onnx` | `onnx` | gline-rs ONNX inference. Requires `ner-onnx` feature. Models download lazily from HuggingFace via `model_download.hf_download`. |
| `Llm` | `llm` | liter-llm zero-shot NER via structured-output prompts. Requires `ner-llm` feature. Useful when domain-specific categories outstrip the ONNX taxonomy. |

---

#### OcrBoundingGeometry

Bounding geometry for an OCR element.

Supports both axis-aligned rectangles (from Tesseract) and 4-point quadrilaterals
(from PaddleOCR and rotated text detection).

| Variant | Wire value | Description |
|---------|------------|-------------|
| `Rectangle` | `rectangle` | Axis-aligned bounding box (typical for Tesseract output). — Fields: `left`: `u32`, `top`: `u32`, `width`: `u32`, `height`: `u32` |
| `Quadrilateral` | `quadrilateral` | 4-point quadrilateral for rotated/skewed text (PaddleOCR). Points are in clockwise order starting from top-left: `[top_left, top_right, bottom_right, bottom_left]` — Fields: `points`: `String` |

---

#### OcrElementLevel

Hierarchical level of an OCR element.

Maps to Tesseract's page segmentation hierarchy and provides
equivalent semantics for PaddleOCR.

| Variant | Wire value | Description |
|---------|------------|-------------|
| `Word` | `word` | Individual word |
| `Line` | `line` | Line of text (default for PaddleOCR) |
| `Block` | `block` | Paragraph or text block |
| `Page` | `page` | Page-level element |

---

#### OutputFormat

Output format for extraction results.

Controls the format of the `content` field in `ExtractionResult`.
When set to `Markdown`, `Djot`, or `Html`, the output uses that format.
`Plain` returns the raw extracted text.
`Structured` returns JSON with full OCR element data including bounding
boxes and confidence scores.

| Variant | Wire value | Description |
|---------|------------|-------------|
| `Plain` | `plain` | Plain text content only (default) |
| `Markdown` | `markdown` | Markdown format |
| `Djot` | `djot` | Djot markup format |
| `Html` | `html` | HTML format |
| `Json` | `json` | JSON tree format with heading-driven sections. |
| `Structured` | `structured` | Structured JSON format with full OCR element metadata. |
| `Custom` | `custom` | Custom renderer registered via the RendererRegistry. The string is the renderer name (e.g., "docx", "latex"). — Fields: `_0`: `String` |

---

#### PiiCategory

PII categories the pattern engine recognises.

| Variant | Wire value | Description |
|---------|------------|-------------|
| `Email` | `email` | Email |
| `Phone` | `phone` | Phone |
| `Ssn` | `ssn` | Ssn |
| `CreditCard` | `credit_card` | Credit card |
| `PostalCode` | `postal_code` | Postal code |
| `IpAddress` | `ip_address` | Ip address |
| `Iban` | `iban` | Iban |
| `SwiftBic` | `swift_bic` | Swift bic |
| `DateOfBirth` | `date_of_birth` | Date of birth |
| `Person` | `person` | Person name, surfaced by the optional NER backend. |
| `Organization` | `organization` | Organization name, surfaced by the optional NER backend. |
| `Location` | `location` | Location, surfaced by the optional NER backend. |
| `Custom` | `custom` | Caller-supplied custom category (e.g. internal employee IDs). Surfaced by the redaction engine when a hit comes from `RedactionConfig.custom_terms` or `RedactionConfig.custom_patterns`. The string is the label passed alongside the term/pattern. Use those fields rather than constructing `Custom` directly via the `categories` filter — the pattern engine cannot detect arbitrary text from a category name alone. — Fields: `_0`: `String` |

---

#### RedactionStrategy

Strategy applied when a PII match is rewritten.

| Variant | Wire value | Description |
|---------|------------|-------------|
| `Mask` | `mask` | Replace the matched span with a fixed mask token (default `"[REDACTED]"`). |
| `Hash` | `hash` | Replace with a SHA-256 hash of the original value (truncated to 16 hex chars). Lets downstream consumers do equality joins without recovering the source. |
| `TokenReplace` | `token_replace` | Replace with a per-category running token (`"[PERSON_1]"`, `"[PERSON_2]"`, …) so the same person referenced twice gets the same token within the document. |
| `Drop` | `drop` | Delete the matched span entirely. |

---

#### ReductionLevel

| Variant | Description |
|---------|-------------|
| `Off` | Off |
| `Light` | Light |
| `Moderate` | Moderate |
| `Aggressive` | Aggressive |
| `Maximum` | Maximum |

---

#### ResultFormat

Result-shape selection for extraction results.

Distinct from `OutputFormat` (which controls rendering — Plain, Markdown,
HTML, etc.). `ResultFormat` controls the *shape* of the result: a unified content
blob vs. an element-based decomposition.

| Variant | Wire value | Description |
|---------|------------|-------------|
| `Unified` | `unified` | Unified format with all content in `content` field |
| `ElementBased` | `element_based` | Element-based format with semantic element extraction |

---

#### SummaryStrategy

Summarisation strategy.

| Variant | Wire value | Description |
|---------|------------|-------------|
| `Extractive` | `extractive` | Pure-Rust extractive summary (TextRank over the chunk graph). Deterministic, fast, no external service required. |
| `Abstractive` | `abstractive` | Abstractive summary produced by liter-llm. Requires `liter-llm` feature and a configured `LlmConfig`. Token usage is captured in `ExtractionResult.llm_usage`. |

---

#### TableModel

Which table structure recognition model to use.

Controls the model used for table cell detection within layout-detected
table regions. Wire format is snake_case in all serializers (JSON, TOML,
YAML).

| Variant | Wire value | Description |
|---------|------------|-------------|
| `Tatr` | `tatr` | TATR (Table Transformer) -- default, 30MB, DETR-based row/column detection. |
| `SlanetWired` | `slanet_wired` | SLANeXT wired variant -- 365MB, optimized for bordered tables. |
| `SlanetWireless` | `slanet_wireless` | SLANeXT wireless variant -- 365MB, optimized for borderless tables. |
| `SlanetPlus` | `slanet_plus` | SLANet-plus -- 7.78MB, lightweight general-purpose. |
| `SlanetAuto` | `slanet_auto` | Classifier-routed SLANeXT: auto-select wired/wireless per table. Uses PP-LCNet classifier (6.78MB) + both SLANeXT variants (730MB total). |
| `Disabled` | `disabled` | Disable table structure model inference entirely; use heuristic path only. |

---

#### TextDirection

Text direction enumeration for HTML documents.

| Variant | Wire value | Description |
|---------|------------|-------------|
| `LeftToRight` | `ltr` | Left-to-right text direction |
| `RightToLeft` | `rtl` | Right-to-left text direction |
| `Auto` | `auto` | Automatic text direction detection |

---

#### VlmFallbackPolicy

Policy controlling when VLM (Vision Language Model) OCR is used as a fallback.

This knob is syntactic sugar over the explicit `OcrPipelineConfig` stage
ordering. When `vlm_fallback` is set and `pipeline` is `None`, an equivalent
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

| Variant | Wire value | Description |
|---------|------------|-------------|
| `Disabled` | `disabled` | No VLM fallback (default). Behaves identically to the pre-policy single-backend mode. |
| `OnLowQuality` | `on_low_quality` | Try the classical OCR backend first. If the quality score is below `quality_threshold`, send the page to the VLM. `quality_threshold` is in the `[0.0, 1.0]` range produced by `calculate_quality_score`. A value of `0.5` is a reasonable starting point; calibrate with the Stage 0 benchmark harness. — Fields: `quality_threshold`: `f64` |
| `Always` | `always` | Skip the classical OCR backend entirely. Every page is sent to the VLM. |

---

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
| `strip_repeating_text` | `bool` | `True` | Enable the heuristic cross-page repeating text detector. When `True` (default), text that repeats verbatim across a supermajority of pages is classified as furniture and stripped.  Disable this if brand names or repeated headings are being incorrectly removed by the heuristic. Note: when a layout-detection model is active, the model may independently classify page-header / page-footer regions as furniture on a per-page basis. To preserve those regions, set `include_headers = true` and/or `include_footers = true` in addition to disabling this flag. Primarily affects PDF extraction. Default: `True`. |
| `include_watermarks` | `bool` | `False` | Include watermark text in extraction output. - PDF: Keeps watermark artifacts and arXiv identifiers. - Other formats: No effect currently. Default: `False` (watermarks are stripped). |

---

### EmailConfig

Configuration for email extraction.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `msg_fallback_codepage` | `int | None` | `None` | Windows codepage number to use when an MSG file contains no codepage property. Defaults to `None`, which falls back to windows-1252. If an unrecognized or invalid codepage number is supplied (including 0), the behavior silently falls back to windows-1252 — the same as when the MSG file itself contains an unrecognized codepage. No error or warning is emitted. Users should verify output when supplying unusual values. Common values: - 1250: Central European (Polish, Czech, Hungarian, etc.) - 1251: Cyrillic (Russian, Ukrainian, Bulgarian, etc.) - 1252: Western European (default) - 1253: Greek - 1254: Turkish - 1255: Hebrew - 1256: Arabic - 932:  Japanese (Shift-JIS) - 936:  Simplified Chinese (GBK) |

---

### ExtractionConfig

Main extraction configuration.

This struct contains all configuration options for the extraction process.
It can be loaded from TOML, YAML, or JSON files, or created programmatically.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `use_cache` | `bool` | `True` | Enable caching of extraction results |
| `enable_quality_processing` | `bool` | `True` | Enable quality post-processing |
| `ocr` | `OcrConfig | None` | `None` | OCR configuration (None = OCR disabled) |
| `force_ocr` | `bool` | `False` | Force OCR even for searchable PDFs |
| `force_ocr_pages` | `list[int] | None` | `None` | Force OCR on specific pages only (1-indexed page numbers, must be >= 1). When set, only the listed pages are OCR'd regardless of text layer quality. Unlisted pages use native text extraction. Ignored when `force_ocr` is `True`. Only applies to PDF documents. Duplicates are automatically deduplicated. An `ocr` config is recommended for backend/language selection; defaults are used if absent. |
| `disable_ocr` | `bool` | `False` | Disable OCR entirely, even for images. When `True`, OCR is skipped for all document types. Images return metadata only (dimensions, format, EXIF) without text extraction. PDFs use only native text extraction without OCR fallback. Cannot be `True` simultaneously with `force_ocr`. *Added in v4.7.0.* |
| `chunking` | `ChunkingConfig | None` | `None` | Text chunking configuration (None = chunking disabled) |
| `content_filter` | `ContentFilterConfig | None` | `None` | Content filtering configuration (None = use extractor defaults). Controls whether document "furniture" (headers, footers, watermarks, repeating text) is included in or stripped from extraction results. See `ContentFilterConfig` for per-field documentation. |
| `images` | `ImageExtractionConfig | None` | `None` | Image extraction configuration (None = no image extraction) |
| `pdf_options` | `PdfConfig | None` | `None` | PDF-specific options (None = use defaults) |
| `token_reduction` | `TokenReductionOptions | None` | `None` | Token reduction configuration (None = no token reduction) |
| `language_detection` | `LanguageDetectionConfig | None` | `None` | Language detection configuration (None = no language detection) |
| `pages` | `PageConfig | None` | `None` | Page extraction configuration (None = no page tracking) |
| `postprocessor` | `PostProcessorConfig | None` | `None` | Post-processor configuration (None = use defaults) |
| `html_options` | `str | None` | `None` | HTML to Markdown conversion options (None = use defaults) Configure how HTML documents are converted to Markdown, including heading styles, list formatting, code block styles, and preprocessing options. |
| `html_output` | `HtmlOutputConfig | None` | `None` | Styled HTML output configuration. When set alongside `output_format = OutputFormat.Html`, the extraction pipeline uses `StyledHtmlRenderer` which emits stable `kb-*` CSS class hooks on every structural element and optionally embeds theme CSS or user-supplied CSS in a `<style>` block. When `None`, the existing plain comrak-based HTML renderer is used. |
| `extraction_timeout_secs` | `int | None` | `None` | Default per-file timeout in seconds for batch extraction. When set, each file in a batch will be canceled after this duration unless overridden by `FileExtractionConfig.timeout_secs`. `None` means no timeout (unbounded extraction time). |
| `max_concurrent_extractions` | `int | None` | `None` | Maximum concurrent extractions in batch operations (None = (num_cpus × 1.5).ceil()). Limits parallelism to prevent resource exhaustion when processing large batches. Defaults to (num_cpus × 1.5).ceil() when not set. |
| `result_format` | `str` | — | Result structure format Controls whether results are returned in unified format (default) with all content in the `content` field, or element-based format with semantic elements (for Unstructured-compatible output). |
| `security_limits` | `str | None` | `None` | Security limits for archive extraction. Controls maximum archive size, compression ratio, file count, and other security thresholds to prevent decompression bomb attacks. When `None`, default limits are used (500MB archive, 100:1 ratio, 10K files). |
| `output_format` | `str` | `Plain` | Content text format (default: Plain). Controls the format of the extracted content: - `Plain`: Raw extracted text (default) - `Markdown`: Markdown formatted output - `Djot`: Djot markup format (requires djot feature) - `Html`: HTML formatted output When set to a structured format, extraction results will include formatted output. The `formatted_content` field may be populated when format conversion is applied. |
| `layout` | `LayoutDetectionConfig | None` | `None` | Layout detection configuration (None = layout detection disabled). When set, PDF pages and images are analyzed for document structure (headings, code, formulas, tables, figures, etc.) using RT-DETR models via ONNX Runtime. For PDFs, layout hints override paragraph classification in the markdown pipeline. For images, per-region OCR is performed with markdown formatting based on detected layout classes. Requires the `layout-detection` feature. |
| `include_document_structure` | `bool` | `False` | Enable structured document tree output. When true, populates the `document` field on `ExtractionResult` with a hierarchical `DocumentStructure` containing heading-driven section nesting, table grids, content layer classification, and inline annotations. Independent of `result_format` — can be combined with Unified or ElementBased. |
| `acceleration` | `AccelerationConfig | None` | `None` | Hardware acceleration configuration for ONNX Runtime models. Controls execution provider selection for layout detection and embedding models. When `None`, uses platform defaults (CoreML on macOS, CUDA on Linux, CPU on Windows). |
| `cache_namespace` | `str | None` | `None` | Cache namespace for tenant isolation. When set, cache entries are stored under `{cache_dir}/{namespace}/`. Must be alphanumeric, hyphens, or underscores only (max 64 chars). Different namespaces have isolated cache spaces on the same filesystem. |
| `cache_ttl_secs` | `int | None` | `None` | Per-request cache TTL in seconds. Overrides the global `max_age_days` for this specific extraction. When `0`, caching is completely skipped (no read or write). When `None`, the global TTL applies. |
| `email` | `EmailConfig | None` | `None` | Email extraction configuration (None = use defaults). Currently supports configuring the fallback codepage for MSG files that do not specify one. See `crate.core.config.EmailConfig` for details. |
| `concurrency` | `str | None` | `None` | Concurrency limits for constrained environments (None = use defaults). Controls Rayon thread pool size, ONNX Runtime intra-op threads, and (when `max_concurrent_extractions` is unset) the batch concurrency semaphore. See `crate.core.config.ConcurrencyConfig` for details. |
| `max_archive_depth` | `int` | — | Maximum recursion depth for archive extraction (default: 3). Set to 0 to disable recursive extraction (legacy behavior). |
| `tree_sitter` | `TreeSitterConfig | None` | `None` | Tree-sitter language pack configuration (None = tree-sitter disabled). When set, enables code file extraction using tree-sitter parsers. Controls grammar download behavior and code analysis options. |
| `structured_extraction` | `StructuredExtractionConfig | None` | `None` | Structured extraction via LLM (None = disabled). When set, the extracted document content is sent to an LLM with the provided JSON schema. The structured response is stored in `ExtractionResult.structured_output`. |
| `cancel_token` | `str | None` | `None` | Cancellation token for this extraction (None = no external cancellation). Pass a `CancellationToken` clone here and call `CancellationToken.cancel` from another thread / task to abort the extraction in progress. The extractor checks the token at safe checkpoints (before lock acquisition, between pages, between batch items) and returns `KreuzbergError.Cancelled` when set. The field is excluded from serialization because `CancellationToken` is a runtime handle, not a configuration value. |

---

### FileExtractionConfig

Per-file extraction configuration overrides for batch processing.

All fields are `Option<T>` — `None` means "use the batch-level default."
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
| `enable_quality_processing` | `bool | None` | `None` | Override quality post-processing for this file. |
| `ocr` | `OcrConfig | None` | `None` | Override OCR configuration for this file (None in the Option = use batch default). |
| `force_ocr` | `bool | None` | `None` | Override force OCR for this file. |
| `force_ocr_pages` | `list[int] | None` | `[]` | Override force OCR pages for this file (1-indexed page numbers). |
| `disable_ocr` | `bool | None` | `None` | Override disable OCR for this file. |
| `chunking` | `ChunkingConfig | None` | `None` | Override chunking configuration for this file. |
| `content_filter` | `ContentFilterConfig | None` | `None` | Override content filtering configuration for this file. |
| `images` | `ImageExtractionConfig | None` | `None` | Override image extraction configuration for this file. |
| `pdf_options` | `PdfConfig | None` | `None` | Override PDF options for this file. |
| `token_reduction` | `TokenReductionOptions | None` | `None` | Override token reduction for this file. |
| `language_detection` | `LanguageDetectionConfig | None` | `None` | Override language detection for this file. |
| `pages` | `PageConfig | None` | `None` | Override page extraction for this file. |
| `postprocessor` | `PostProcessorConfig | None` | `None` | Override post-processor for this file. |
| `html_options` | `str | None` | `None` | Override HTML conversion options for this file. |
| `result_format` | `str | None` | `None` | Override result format for this file. |
| `output_format` | `str | None` | `None` | Override output content format for this file. |
| `include_document_structure` | `bool | None` | `None` | Override document structure output for this file. |
| `layout` | `LayoutDetectionConfig | None` | `None` | Override layout detection for this file. |
| `timeout_secs` | `int | None` | `None` | Override per-file extraction timeout in seconds. When set, the extraction for this file will be canceled after the specified duration. A timed-out file produces an error result without affecting other files in the batch. |
| `tree_sitter` | `TreeSitterConfig | None` | `None` | Override tree-sitter configuration for this file. |
| `structured_extraction` | `StructuredExtractionConfig | None` | `None` | Override structured extraction configuration for this file. When set, enables LLM-based structured extraction with a JSON schema for this specific file. The extracted content is sent to a VLM/LLM and the response is parsed according to the provided schema. |

---

### ImageExtractionConfig

Image extraction configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `extract_images` | `bool` | — | Extract images from documents |
| `target_dpi` | `int` | — | Target DPI for image normalization |
| `max_image_dimension` | `int` | — | Maximum dimension for images (width or height) |
| `inject_placeholders` | `bool` | — | Whether to inject image reference placeholders into markdown output. When `True` (default), image references like `![Image 1](embedded:p1_i0)` are appended to the markdown. Set to `False` to extract images as data without polluting the markdown output. |
| `auto_adjust_dpi` | `bool` | — | Automatically adjust DPI based on image content |
| `min_dpi` | `int` | — | Minimum DPI threshold |
| `max_dpi` | `int` | — | Maximum DPI threshold |
| `max_images_per_page` | `int | None` | `None` | Maximum number of image objects to extract per PDF page. Some PDFs (e.g. technical diagrams stored as thousands of raster fragments) can trigger extremely long or indefinite extraction times when every image object on a dense page is decoded individually via pdfium FFI. Setting this limit causes kreuzberg to stop collecting individual images once the count per page reaches the cap and emit a warning instead. `None` (default) means no limit — all images are extracted. |

---

### TokenReductionOptions

Token reduction configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `mode` | `str` | — | Reduction mode: "off", "light", "moderate", "aggressive", "maximum" |
| `preserve_important_words` | `bool` | — | Preserve important words (capitalized, technical terms) |

---

### LanguageDetectionConfig

Language detection configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `bool` | — | Enable language detection |
| `min_confidence` | `float` | — | Minimum confidence threshold (0.0-1.0) |
| `detect_multiple` | `bool` | — | Detect multiple languages in the document |

---

### HtmlOutputConfig

Configuration for styled HTML output.

When set on `ExtractionConfig.html_output` alongside
`output_format = OutputFormat.Html`, the pipeline builds a
`StyledHtmlRenderer` instead of
the plain comrak-based renderer.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `css` | `str | None` | `None` | Inline CSS string injected into the output after the theme stylesheet. Concatenated after `css_file` content when both are set. |
| `css_file` | `str | None` | `None` | Path to a CSS file loaded once at renderer construction time. Concatenated before `css` when both are set. |
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
| `confidence_threshold` | `float | None` | `None` | Confidence threshold override (None = use model default). |
| `apply_heuristics` | `bool` | `True` | Whether to apply postprocessing heuristics (default: true). |
| `table_model` | `TableModel` | `TableModel.TATR` | Table structure recognition model. Controls which model is used for table cell detection within layout-detected table regions. Defaults to `TableModel.Tatr`. |
| `acceleration` | `AccelerationConfig | None` | `None` | Hardware acceleration for ONNX models (layout detection + table structure). When set, controls which execution provider (CPU, CUDA, CoreML, TensorRT) is used for inference. Defaults to `None` (auto-select per platform). |

---

### LlmConfig

Configuration for an LLM provider/model via liter-llm.

Each feature (VLM OCR, VLM embeddings, structured extraction) carries
its own `LlmConfig`, allowing different providers per feature.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `str` | — | Provider/model string using liter-llm routing format. Examples: `"openai/gpt-4o"`, `"anthropic/claude-sonnet-4-20250514"`, `"groq/llama-3.1-70b-versatile"`. |
| `api_key` | `str | None` | `None` | API key for the provider. When `None`, liter-llm falls back to the provider's standard environment variable (e.g., `OPENAI_API_KEY`). |
| `base_url` | `str | None` | `None` | Custom base URL override for the provider endpoint. |
| `timeout_secs` | `int | None` | `None` | Request timeout in seconds (default: 60). |
| `max_retries` | `int | None` | `None` | Maximum retry attempts (default: 3). |
| `temperature` | `float | None` | `None` | Sampling temperature for generation tasks. |
| `max_tokens` | `int | None` | `None` | Maximum tokens to generate. |

---

### StructuredExtractionConfig

Configuration for LLM-based structured data extraction.

Sends extracted document content to a VLM with a JSON schema,
returning structured data that conforms to the schema.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `schema` | `dict[str, Any]` | — | JSON Schema defining the desired output structure. |
| `schema_name` | `str` | — | Schema name passed to the LLM's structured output mode. |
| `schema_description` | `str | None` | `None` | Optional schema description for the LLM. |
| `strict` | `bool` | — | Enable strict mode — output must exactly match the schema. |
| `prompt` | `str | None` | `None` | Custom Jinja2 extraction prompt template. When `None`, a default template is used. Available template variables: - `{{ content }}` — The extracted document text. - `{{ schema }}` — The JSON schema as a formatted string. - `{{ schema_name }}` — The schema name. - `{{ schema_description }}` — The schema description (may be empty). |
| `llm` | `LlmConfig` | — | LLM configuration for the extraction. |

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
| `quality_thresholds` | `OcrQualityThresholds` | — | Quality thresholds for deciding whether to accept a result or try the next backend. |

---

### OcrConfig

OCR configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `bool` | `True` | Whether OCR is enabled. Setting `enabled: false` is a shorthand for `disable_ocr: true` on the parent `ExtractionConfig`. Images return metadata only; PDFs use native text extraction without OCR fallback. Defaults to `True`. When `False`, all other OCR settings are ignored. |
| `backend` | `str` | — | OCR backend: tesseract, easyocr, paddleocr |
| `language` | `str` | — | Language code (e.g., "eng", "deu") |
| `tesseract_config` | `TesseractConfig | None` | `None` | Tesseract-specific configuration (optional) |
| `output_format` | `str | None` | `None` | Output format for OCR results (optional, for format conversion) |
| `paddle_ocr_config` | `dict[str, Any] | None` | `None` | PaddleOCR-specific configuration (optional, JSON passthrough) |
| `element_config` | `OcrElementConfig | None` | `None` | OCR element extraction configuration |
| `quality_thresholds` | `OcrQualityThresholds | None` | `None` | Quality thresholds for the native-text-to-OCR fallback decision. When None, uses compiled defaults (matching previous hardcoded behavior). |
| `pipeline` | `OcrPipelineConfig | None` | `None` | Multi-backend OCR pipeline configuration. When set, enables weighted fallback across multiple OCR backends based on output quality. When None, uses the single `backend` field (same as today). |
| `auto_rotate` | `bool` | `False` | Enable automatic page rotation based on orientation detection. When enabled, uses Tesseract's `DetectOrientationScript()` to detect page orientation (0/90/180/270 degrees) before OCR. If the page is rotated with high confidence, the image is corrected before recognition. This is critical for handling rotated scanned documents. |
| `vlm_config` | `LlmConfig | None` | `None` | VLM (Vision Language Model) OCR configuration. Required when `backend` is `"vlm"`. Uses liter-llm to send page images to a vision model for text extraction. |
| `vlm_prompt` | `str | None` | `None` | Custom Jinja2 prompt template for VLM OCR. When `None`, uses the default template. Available variables: - `{{ language }}` — The document language code (e.g., "eng", "deu"). |
| `acceleration` | `AccelerationConfig | None` | `None` | Hardware acceleration for ONNX Runtime models (e.g. PaddleOCR, layout detection). Not user-configurable via config files — injected at runtime from `ExtractionConfig.acceleration` before each `process_image` call. |

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
| `marker_format` | `str` | `"

<!-- PAGE {page_num} -->

"` | Page marker format (use {page_num} placeholder) Default: "\n\n<!-- PAGE {page_num} -->\n\n" |

---

### PdfConfig

PDF-specific configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `backend` | `PdfBackend` | `PdfBackend.PDFIUM` | PDF extraction backend. Default: `Pdfium`. |
| `extract_images` | `bool` | `False` | Extract images from PDF |
| `passwords` | `list[str] | None` | `None` | List of passwords to try when opening encrypted PDFs |
| `extract_metadata` | `bool` | `True` | Extract PDF metadata |
| `hierarchy` | `HierarchyConfig | None` | `None` | Hierarchy extraction configuration (None = hierarchy extraction disabled) |
| `extract_annotations` | `bool` | `False` | Extract PDF annotations (text notes, highlights, links, stamps). Default: false |
| `top_margin_fraction` | `float | None` | `None` | Top margin fraction (0.0–1.0) of page height to exclude headers/running heads. Default: 0.06 (6%) |
| `bottom_margin_fraction` | `float | None` | `None` | Bottom margin fraction (0.0–1.0) of page height to exclude footers/page numbers. Default: 0.05 (5%) |
| `allow_single_column_tables` | `bool` | `False` | Allow single-column pseudo tables in extraction results. By default, tables with fewer than 2 columns (layout-guided) or 3 columns (heuristic) are rejected. When `True`, the minimum column count is relaxed to 1, allowing single-column structured data (glossaries, itemized lists) to be emitted as tables. Other quality filters (density, sparsity, prose detection) still apply. |

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
| `ocr_coverage_threshold` | `float | None` | `None` | OCR coverage threshold for smart OCR triggering (0.0-1.0) Determines when OCR should be triggered based on text block coverage. OCR is triggered when text blocks cover less than this fraction of the page. Default: 0.5 (trigger OCR if less than 50% of page has text) |

---

### PostProcessorConfig

Post-processor configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `bool` | `True` | Enable post-processors |
| `enabled_processors` | `list[str] | None` | `None` | Whitelist of processor names to run (None = all enabled) |
| `disabled_processors` | `list[str] | None` | `None` | Blacklist of processor names to skip (None = none disabled) |
| `enabled_set` | `str | None` | `None` | Pre-computed AHashSet for O(1) enabled processor lookup |
| `disabled_set` | `str | None` | `None` | Pre-computed AHashSet for O(1) disabled processor lookup |

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
| `embedding` | `EmbeddingConfig | None` | `None` | Optional embedding configuration for chunk embeddings. |
| `preset` | `str | None` | `None` | Use a preset configuration (overrides individual settings if provided). |
| `sizing` | `ChunkSizing` | `ChunkSizing.CHARACTERS` | How to measure chunk size. Default: `Characters` (Unicode character count). Enable `chunking-tiktoken` or `chunking-tokenizers` features for token-based sizing. |
| `prepend_heading_context` | `bool` | `False` | When `True` and `chunker_type` is `Markdown`, prepend the heading hierarchy path (e.g. `"# Title > ## Section\n\n"`) to each chunk's content string. This is useful for RAG pipelines where each chunk needs self-contained context about its position in the document structure. Default: `False` |
| `topic_threshold` | `float | None` | `None` | Optional cosine similarity threshold for semantic topic boundary detection. Only used when `chunker_type` is `Semantic` and an `EmbeddingConfig` is provided. You almost never need to set this. When omitted, defaults to `0.75` which works well for most documents. Lower values detect more topic boundaries (more, smaller chunks); higher values detect fewer. Range: `0.0..=1.0`. |

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
| `cache_dir` | `str | None` | `None` | Custom cache directory for model files Defaults to `~/.cache/kreuzberg/embeddings/` if not specified. Allows full customization of model download location. |
| `acceleration` | `AccelerationConfig | None` | `None` | Hardware acceleration for the embedding ONNX model. When set, controls which execution provider (CPU, CUDA, CoreML, TensorRT) is used for inference. Defaults to `None` (auto-select per platform). |
| `max_embed_duration_secs` | `int | None` | `None` | Maximum wall-clock duration (in seconds) for a single `embed()` call when using `EmbeddingModelType.Plugin`. Applies only to the in-process plugin path — protects against hung host-language backends (e.g. a Python callback deadlocked on the GIL, a model stuck on CUDA OOM retries, etc.). On timeout, the dispatcher returns `crate.KreuzbergError.Plugin` instead of blocking forever. `None` disables the timeout. The default (60 seconds) is conservative for common in-process inference; increase for large batches on slow hardware. |

---

### TreeSitterConfig

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
| `enabled` | `bool` | `True` | Enable code intelligence processing (default: true). When `False`, tree-sitter analysis is completely skipped even if the config section is present. |
| `cache_dir` | `str | None` | `None` | Custom cache directory for downloaded grammars. When `None`, uses the default: `~/.cache/tree-sitter-language-pack/v{version}/libs/`. |
| `languages` | `list[str] | None` | `None` | Languages to pre-download on init (e.g., `["python", "rust"]`). |
| `groups` | `list[str] | None` | `None` | Language groups to pre-download (e.g., `["web", "systems", "scripting"]`). |
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
| `chunk_max_size` | `int | None` | `None` | Maximum chunk size in bytes. `None` disables chunking. |
| `content_mode` | `CodeContentMode` | `CodeContentMode.CHUNKS` | Content rendering mode for code extraction. |

---

### ServerConfig

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
| `host` | `str` | — | Server host address (e.g., "127.0.0.1", "0.0.0.0") |
| `port` | `int` | — | Server port number |
| `cors_origins` | `list[str]` | `[]` | CORS allowed origins. Empty vector means allow all origins. If this is an empty vector, the server will accept requests from any origin. If populated with specific origins (e.g., ["<https://example.com">]), only those origins will be allowed. |
| `max_request_body_bytes` | `int` | — | Maximum size of request body in bytes (default: 100 MB) |
| `max_multipart_field_bytes` | `int` | — | Maximum size of multipart fields in bytes (default: 100 MB) |

---

### Drawing

A drawing object extracted from `<w:drawing>`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `drawing_type` | `str` | — | Drawing type |
| `extent` | `str | None` | `None` | Extent |
| `doc_properties` | `str | None` | `None` | Doc properties |
| `image_ref` | `str | None` | `None` | Image ref |

---

### AnchorProperties

Properties for anchored drawings.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `behind_doc` | `bool` | — | Behind doc |
| `layout_in_cell` | `bool` | — | Layout in cell |
| `relative_height` | `int | None` | `None` | Relative height |
| `position_h` | `str | None` | `None` | Position h |
| `position_v` | `str | None` | `None` | Position v |
| `wrap_type` | `str` | — | Wrap type |

---

### HeaderFooter

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `paragraphs` | `list[str]` | `[]` | Paragraphs |
| `tables` | `list[str]` | `[]` | Tables extracted from the document |
| `header_type` | `str` | — | Header type |

---

### PageMarginsPoints

Page margins converted to points (1/72 inch).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `top` | `float | None` | `None` | Top |
| `right` | `float | None` | `None` | Right |
| `bottom` | `float | None` | `None` | Bottom |
| `left` | `float | None` | `None` | Left |
| `header` | `float | None` | `None` | Header |
| `footer` | `float | None` | `None` | Footer |
| `gutter` | `float | None` | `None` | Gutter |

---

### ResolvedStyle

Fully resolved (flattened) style after walking the inheritance chain.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `paragraph_properties` | `str` | — | Paragraph properties |
| `run_properties` | `str` | — | Run properties |

---

### TableProperties

Table-level properties from `<w:tblPr>`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `style_id` | `str | None` | `None` | Style id |
| `width` | `str | None` | `None` | Width |
| `alignment` | `str | None` | `None` | Alignment |
| `layout` | `str | None` | `None` | Layout |
| `look` | `str | None` | `None` | Look |
| `borders` | `str | None` | `None` | Borders |
| `cell_margins` | `str | None` | `None` | Cell margins |
| `indent` | `str | None` | `None` | Indent |
| `caption` | `str | None` | `None` | Caption |

---

### XlsxAppProperties

Application properties from docProps/app.xml for XLSX

Contains Excel-specific document metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `application` | `str | None` | `None` | Application name (e.g., "Microsoft Excel") |
| `app_version` | `str | None` | `None` | Application version |
| `doc_security` | `int | None` | `None` | Document security level |
| `scale_crop` | `bool | None` | `None` | Scale crop flag |
| `links_up_to_date` | `bool | None` | `None` | Links up to date flag |
| `shared_doc` | `bool | None` | `None` | Shared document flag |
| `hyperlinks_changed` | `bool | None` | `None` | Hyperlinks changed flag |
| `company` | `str | None` | `None` | Company name |
| `worksheet_names` | `list[str]` | `[]` | Worksheet names |

---

### PptxAppProperties

Application properties from docProps/app.xml for PPTX

Contains PowerPoint-specific document metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `application` | `str | None` | `None` | Application name (e.g., "Microsoft Office PowerPoint") |
| `app_version` | `str | None` | `None` | Application version |
| `total_time` | `int | None` | `None` | Total editing time in minutes |
| `company` | `str | None` | `None` | Company name |
| `doc_security` | `int | None` | `None` | Document security level |
| `scale_crop` | `bool | None` | `None` | Scale crop flag |
| `links_up_to_date` | `bool | None` | `None` | Links up to date flag |
| `shared_doc` | `bool | None` | `None` | Shared document flag |
| `hyperlinks_changed` | `bool | None` | `None` | Hyperlinks changed flag |
| `slides` | `int | None` | `None` | Number of slides |
| `notes` | `int | None` | `None` | Number of notes |
| `hidden_slides` | `int | None` | `None` | Number of hidden slides |
| `multimedia_clips` | `int | None` | `None` | Number of multimedia clips |
| `presentation_format` | `str | None` | `None` | Presentation format (e.g., "Widescreen", "Standard") |
| `slide_titles` | `list[str]` | `[]` | Slide titles |

---

### OdtProperties

OpenDocument metadata from meta.xml

Contains metadata fields defined by the OASIS OpenDocument Format standard.
Uses Dublin Core elements (dc:) and OpenDocument meta elements (meta:).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `title` | `str | None` | `None` | Document title (dc:title) |
| `subject` | `str | None` | `None` | Document subject/topic (dc:subject) |
| `creator` | `str | None` | `None` | Current document creator/author (dc:creator) |
| `initial_creator` | `str | None` | `None` | Initial creator of the document (meta:initial-creator) |
| `keywords` | `str | None` | `None` | Keywords or tags (meta:keyword) |
| `description` | `str | None` | `None` | Document description (dc:description) |
| `date` | `str | None` | `None` | Current modification date (dc:date) |
| `creation_date` | `str | None` | `None` | Initial creation date (meta:creation-date) |
| `language` | `str | None` | `None` | Document language (dc:language) |
| `generator` | `str | None` | `None` | Generator/application that created the document (meta:generator) |
| `editing_duration` | `str | None` | `None` | Editing duration in ISO 8601 format (meta:editing-duration) |
| `editing_cycles` | `str | None` | `None` | Number of edits/revisions (meta:editing-cycles) |
| `page_count` | `int | None` | `None` | Document statistics - page count (meta:page-count) |
| `word_count` | `int | None` | `None` | Document statistics - word count (meta:word-count) |
| `character_count` | `int | None` | `None` | Document statistics - character count (meta:character-count) |
| `paragraph_count` | `int | None` | `None` | Document statistics - paragraph count (meta:paragraph-count) |
| `table_count` | `int | None` | `None` | Document statistics - table count (meta:table-count) |
| `image_count` | `int | None` | `None` | Document statistics - image count (meta:image-count) |

---

### TokenReductionConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `level` | `ReductionLevel` | `ReductionLevel.MODERATE` | Level (reduction level) |
| `language_hint` | `str | None` | `None` | Language hint |
| `preserve_markdown` | `bool` | `False` | Preserve markdown |
| `preserve_code` | `bool` | `True` | Preserve code |
| `semantic_threshold` | `float` | `0.3` | Semantic threshold |
| `enable_parallel` | `bool` | `True` | Enable parallel |
| `use_simd` | `bool` | `True` | Use simd |
| `custom_stopwords` | `dict[str, list[str]] | None` | `None` | Custom stopwords |
| `preserve_patterns` | `list[str]` | `[]` | Preserve patterns |
| `target_reduction` | `float | None` | `None` | Target reduction |
| `enable_semantic_clustering` | `bool` | `False` | Enable semantic clustering |

---

### DocumentStructure

Top-level structured document representation.

A flat array of nodes with index-based parent/child references forming a tree.
Root-level nodes have `parent: None`. Use `body_roots()` and `furniture_roots()`
to iterate over top-level content by layer.

# Validation

Call `validate()` after construction to verify all node indices are in bounds
and parent-child relationships are bidirectionally consistent.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `nodes` | `list[DocumentNode]` | `[]` | All nodes in document/reading order. |
| `source_format` | `str | None` | `None` | Origin format identifier (e.g. "docx", "pptx", "html", "pdf"). Allows renderers to apply format-aware heuristics when converting the document tree to output formats. |
| `relationships` | `list[DocumentRelationship]` | `[]` | Resolved relationships between nodes (footnote refs, citations, anchor links, etc.). Populated during derivation from the internal document representation. Empty when no relationships are detected. |

---

### ExtractionResult

General extraction result used by the core extraction API.

This is the main result type returned by all extraction functions.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content` | `str` | — | The extracted text content |
| `mime_type` | `str` | — | The detected MIME type |
| `metadata` | `Metadata` | — | Document metadata |
| `tables` | `list[str]` | `[]` | Tables extracted from the document |
| `detected_languages` | `list[str] | None` | `[]` | Detected languages |
| `chunks` | `list[Chunk] | None` | `[]` | Text chunks when chunking is enabled. When chunking configuration is provided, the content is split into overlapping chunks for efficient processing. Each chunk contains the text, optional embeddings (if enabled), and metadata about its position. |
| `images` | `list[ExtractedImage] | None` | `[]` | Extracted images from the document. When image extraction is enabled via `ImageExtractionConfig`, this field contains all images found in the document with their raw data and metadata. Each image may optionally contain a nested `ocr_result` if OCR was performed. |
| `pages` | `list[PageContent] | None` | `[]` | Per-page content when page extraction is enabled. When page extraction is configured, the document is split into per-page content with tables and images mapped to their respective pages. |
| `elements` | `list[Element] | None` | `[]` | Semantic elements when element-based result format is enabled. When result_format is set to ElementBased, this field contains semantic elements with type classification, unique identifiers, and metadata for Unstructured-compatible element-based processing. |
| `djot_content` | `DjotContent | None` | `None` | Rich Djot content structure (when extracting Djot documents). When extracting Djot documents with structured extraction enabled, this field contains the full semantic structure including: - Block-level elements with nesting - Inline formatting with attributes - Links, images, footnotes - Math expressions - Complete attribute information The `content` field still contains plain text for backward compatibility. Always `None` for non-Djot documents. |
| `ocr_elements` | `list[OcrElement] | None` | `[]` | OCR elements with full spatial and confidence metadata. When OCR is performed with element extraction enabled, this field contains the structured representation of detected text including: - Bounding geometry (rectangles or quadrilaterals) - Confidence scores (detection and recognition) - Rotation information - Hierarchical relationships (Tesseract only) This field preserves all metadata that would otherwise be lost when converting to plain text or markdown output formats. Only populated when `OcrElementConfig.include_elements` is true. |
| `document` | `DocumentStructure | None` | `None` | Structured document tree (when document structure extraction is enabled). When `include_document_structure` is true in `ExtractionConfig`, this field contains the full hierarchical representation of the document including: - Heading-driven section nesting - Table grids with cell-level metadata - Content layer classification (body, header, footer, footnote) - Inline text annotations (formatting, links) - Bounding boxes and page numbers Independent of `result_format` — can be combined with Unified or ElementBased. |
| `quality_score` | `float | None` | `None` | Document quality score from quality analysis. A value between 0.0 and 1.0 indicating the overall text quality. Previously stored in `metadata.additional["quality_score"]`. |
| `processing_warnings` | `list[ProcessingWarning]` | `[]` | Non-fatal warnings collected during processing pipeline stages. Captures errors from optional pipeline features (embedding, chunking, language detection, output formatting) that don't prevent extraction but may indicate degraded results. Previously stored as individual keys in `metadata.additional`. |
| `annotations` | `list[PdfAnnotation] | None` | `[]` | PDF annotations extracted from the document. When annotation extraction is enabled via `PdfConfig.extract_annotations`, this field contains text notes, highlights, links, stamps, and other annotations found in PDF documents. |
| `children` | `list[ArchiveEntry] | None` | `[]` | Nested extraction results from archive contents. When extracting archives, each processable file inside produces its own full extraction result. Set to `None` for non-archive formats. Use `max_archive_depth` in config to control recursion depth. |
| `uris` | `list[Uri] | None` | `[]` | URIs/links discovered during document extraction. Contains hyperlinks, image references, citations, email addresses, and other URI-like references found in the document. Always extracted when present in the source document. |
| `structured_output` | `dict[str, Any] | None` | `None` | Structured extraction output from LLM-based JSON schema extraction. When `structured_extraction` is configured in `ExtractionConfig`, the extracted document content is sent to a VLM with the provided JSON schema. The response is parsed and stored here as a JSON value matching the schema. |
| `code_intelligence` | `str | None` | `None` | Code intelligence results from tree-sitter analysis. Populated when extracting source code files with the `tree-sitter` feature. Contains metrics, structural analysis, imports/exports, comments, docstrings, symbols, diagnostics, and optionally chunked code segments. |
| `llm_usage` | `list[LlmUsage] | None` | `[]` | LLM token usage and cost data for all LLM calls made during this extraction. Contains one entry per LLM call. Multiple entries are produced when VLM OCR, structured extraction, and/or LLM embeddings all run during the same extraction. `None` when no LLM was used. |
| `formatted_content` | `str | None` | `None` | Pre-rendered content in the requested output format. Populated during `derive_extraction_result` before tree derivation consumes element data. `apply_output_format` swaps this into `content` at the end of the pipeline, after post-processors have operated on plain text. |
| `ocr_internal_document` | `str | None` | `None` | Structured hOCR document for the OCR+layout pipeline. When tesseract produces hOCR output, the parsed `InternalDocument` carries paragraph structure with bounding boxes and confidence scores. The layout classification step enriches these elements before final rendering. |

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
| `input_tokens` | `int | None` | `None` | Number of input/prompt tokens consumed. |
| `output_tokens` | `int | None` | `None` | Number of output/completion tokens generated. |
| `total_tokens` | `int | None` | `None` | Total tokens (input + output). |
| `estimated_cost` | `float | None` | `None` | Estimated cost in USD based on the provider's published pricing. |
| `finish_reason` | `str | None` | `None` | Why the model stopped generating (e.g. "stop", "length", "content_filter"). |

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
| `psm` | `int` | `3` | Page Segmentation Mode (0-13). Common values: - 3: Fully automatic page segmentation (default) - 6: Assume a single uniform block of text - 11: Sparse text with no particular order |
| `output_format` | `str` | `"markdown"` | Output format ("text" or "markdown") |
| `oem` | `int` | `3` | OCR Engine Mode (0-3). - 0: Legacy engine only - 1: Neural nets (LSTM) only (usually best) - 2: Legacy + LSTM - 3: Default (based on what's available) |
| `min_confidence` | `float` | `0` | Minimum confidence threshold (0.0-100.0). Words with confidence below this threshold may be rejected or flagged. |
| `preprocessing` | `ImagePreprocessingConfig | None` | `None` | Image preprocessing configuration. Controls how images are preprocessed before OCR. Can significantly improve quality for scanned documents or low-quality images. |
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
| `title` | `str | None` | `None` | Document title |
| `subject` | `str | None` | `None` | Document subject or description |
| `authors` | `list[str] | None` | `[]` | Primary author(s) - always Vec for consistency |
| `keywords` | `list[str] | None` | `[]` | Keywords/tags - always Vec for consistency |
| `language` | `str | None` | `None` | Primary language (ISO 639 code) |
| `created_at` | `str | None` | `None` | Creation timestamp (ISO 8601 format) |
| `modified_at` | `str | None` | `None` | Last modification timestamp (ISO 8601 format) |
| `created_by` | `str | None` | `None` | User who created the document |
| `modified_by` | `str | None` | `None` | User who last modified the document |
| `pages` | `PageStructure | None` | `None` | Page/slide/sheet structure with boundaries |
| `format` | `FormatMetadata | None` | `None` | Format-specific metadata (discriminated union) Contains detailed metadata specific to the document format. Serializes with a `format_type` discriminator field. |
| `image_preprocessing` | `ImagePreprocessingMetadata | None` | `None` | Image preprocessing metadata (when OCR preprocessing was applied) |
| `json_schema` | `dict[str, Any] | None` | `None` | JSON schema (for structured data extraction) |
| `error` | `ErrorMetadata | None` | `None` | Error metadata (for batch operations) |
| `extraction_duration_ms` | `int | None` | `None` | Extraction duration in milliseconds (for benchmarking). This field is populated by batch extraction to provide per-file timing information. It's `None` for single-file extraction (which uses external timing). |
| `category` | `str | None` | `None` | Document category (from frontmatter or classification). |
| `tags` | `list[str] | None` | `[]` | Document tags (from frontmatter). |
| `document_version` | `str | None` | `None` | Document version string (from frontmatter). |
| `abstract_text` | `str | None` | `None` | Abstract or summary text (from frontmatter). |
| `output_format` | `str | None` | `None` | Output format identifier (e.g., "markdown", "html", "text"). Set by the output format pipeline stage when format conversion is applied. Previously stored in `metadata.additional["output_format"]`. |
| `additional` | `str` | — | Additional custom fields from postprocessors. **Deprecated**: Prefer using typed fields on `ExtractionResult` and `Metadata` instead of inserting into this map. Typed fields provide better cross-language compatibility and type safety. This field will be removed in a future major version. This flattened map allows Python/TypeScript postprocessors to add arbitrary fields (entity extraction, keyword extraction, etc.). Fields are merged at the root level during serialization. Uses `Cow<'static, str>` keys so static string keys avoid allocation. |

---

### ExcelMetadata

Excel/spreadsheet metadata.

Contains information about sheets in Excel, OpenDocument Calc, and other
spreadsheet formats (.xlsx, .xls, .ods, etc.).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `sheet_count` | `int` | — | Total number of sheets in the workbook |
| `sheet_names` | `list[str]` | `[]` | Names of all sheets in order |

---

### EmailMetadata

Email metadata extracted from .eml and .msg files.

Includes sender/recipient information, message ID, and attachment list.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `from_email` | `str | None` | `None` | Sender's email address |
| `from_name` | `str | None` | `None` | Sender's display name |
| `to_emails` | `list[str]` | `[]` | Primary recipients |
| `cc_emails` | `list[str]` | `[]` | CC recipients |
| `bcc_emails` | `list[str]` | `[]` | BCC recipients |
| `message_id` | `str | None` | `None` | Message-ID header value |
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
| `compressed_size` | `int | None` | `None` | Compressed size in bytes (if available) |

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
| `headers` | `list[str] | None` | `[]` | Markdown headers (headings text only, for Markdown files) |
| `links` | `list[str] | None` | `[]` | Markdown links as (text, url) tuples (for Markdown files) |
| `code_blocks` | `list[str] | None` | `[]` | Code blocks as (language, code) tuples (for Markdown files) |

---

### HtmlMetadata

HTML metadata extracted from HTML documents.

Includes document-level metadata, Open Graph data, Twitter Card metadata,
and extracted structural elements (headers, links, images, structured data).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `title` | `str | None` | `None` | Document title from `<title>` tag |
| `description` | `str | None` | `None` | Document description from `<meta name="description">` tag |
| `keywords` | `list[str]` | `[]` | Document keywords from `<meta name="keywords">` tag, split on commas |
| `author` | `str | None` | `None` | Document author from `<meta name="author">` tag |
| `canonical_url` | `str | None` | `None` | Canonical URL from `<link rel="canonical">` tag |
| `base_href` | `str | None` | `None` | Base URL from `<base href="">` tag for resolving relative URLs |
| `language` | `str | None` | `None` | Document language from `lang` attribute |
| `text_direction` | `TextDirection | None` | `None` | Document text direction from `dir` attribute |
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
| `table_rows` | `int | None` | `None` | Table rows |
| `table_cols` | `int | None` | `None` | Table cols |

---

### PptxMetadata

PowerPoint presentation metadata.

Extracted from PPTX files containing slide counts and presentation details.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `slide_count` | `int` | — | Total number of slides in the presentation |
| `slide_names` | `list[str]` | `[]` | Names of slides (if available) |
| `image_count` | `int | None` | `None` | Number of embedded images |
| `table_count` | `int | None` | `None` | Number of tables |

---

### DocxMetadata

Word document metadata.

Extracted from DOCX files using shared Office Open XML metadata extraction.
Integrates with `office_metadata` module for core/app/custom properties.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `core_properties` | `str | None` | `None` | Core properties from docProps/core.xml (Dublin Core metadata) Contains title, creator, subject, keywords, dates, etc. Shared format across DOCX/PPTX/XLSX documents. |
| `app_properties` | `str | None` | `None` | Application properties from docProps/app.xml (Word-specific statistics) Contains word count, page count, paragraph count, editing time, etc. DOCX-specific variant of Office application properties. |
| `custom_properties` | `dict[str, dict[str, Any]] | None` | `{}` | Custom properties from docProps/custom.xml (user-defined properties) Contains key-value pairs defined by users or applications. Values can be strings, numbers, booleans, or dates. |

---

### CsvMetadata

CSV/TSV file metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `row_count` | `int` | — | Number of rows |
| `column_count` | `int` | — | Number of columns |
| `delimiter` | `str | None` | `None` | Delimiter |
| `has_header` | `bool` | — | Whether header |
| `column_types` | `list[str] | None` | `[]` | Column types |

---

### BibtexMetadata

BibTeX bibliography metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `entry_count` | `int` | — | Number of entries in the bibliography. |
| `citation_keys` | `list[str]` | `[]` | Citation keys |
| `authors` | `list[str]` | `[]` | Authors |
| `year_range` | `YearRange | None` | `None` | Year range (year range) |
| `entry_types` | `dict[str, int] | None` | `{}` | Entry types |

---

### CitationMetadata

Citation file metadata (RIS, PubMed, EndNote).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `citation_count` | `int` | — | Number of citations |
| `format` | `str | None` | `None` | Format |
| `authors` | `list[str]` | `[]` | Authors |
| `year_range` | `YearRange | None` | `None` | Year range (year range) |
| `dois` | `list[str]` | `[]` | Dois |
| `keywords` | `list[str]` | `[]` | Keywords |

---

### FictionBookMetadata

FictionBook (FB2) metadata.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `genres` | `list[str]` | `[]` | Genres |
| `sequences` | `list[str]` | `[]` | Sequences |
| `annotation` | `str | None` | `None` | Annotation |

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
| `copyright` | `str | None` | `None` | Copyright |
| `license` | `str | None` | `None` | License |
| `history_dates` | `dict[str, str]` | `{}` | History dates |
| `contributor_roles` | `list[ContributorRole]` | `[]` | Contributor roles |

---

### EpubMetadata

EPUB metadata (Dublin Core extensions).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `coverage` | `str | None` | `None` | Coverage |
| `dc_format` | `str | None` | `None` | Dc format |
| `relation` | `str | None` | `None` | Relation |
| `source` | `str | None` | `None` | Source |
| `dc_type` | `str | None` | `None` | Dc type |
| `cover_image` | `str | None` | `None` | Cover image |

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
| `detection` | `float | None` | `None` | Detection confidence: how confident the OCR engine is that text exists here. PaddleOCR provides this as `box_score`, Tesseract doesn't have a direct equivalent. Range: 0.0 to 1.0 (or None if not available). |
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
| `rotation` | `OcrRotation | None` | `None` | Rotation information (if detected). |
| `page_number` | `int` | — | Page number (1-indexed). |
| `parent_id` | `str | None` | `None` | Parent element ID for hierarchical relationships. Only used for Tesseract output which has word -> line -> block hierarchy. |
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
| `bounding_box` | `str` | — | Bounding box in document coordinate space. |
| `area_fraction` | `float` | — | Fraction of the page area covered by this region (0.0 to 1.0). |

---

### WarmRequest

Cache warm request.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `all_embeddings` | `bool` | — | Download all embedding model presets |
| `embedding_model` | `str | None` | `None` | Specific embedding model preset to download |

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
| `language` | `str | None` | `None` | Language code for stopword filtering (e.g., "en", "de", "fr"). If None, no stopword filtering is applied. |
| `yake_params` | `YakeParams | None` | `None` | YAKE-specific tuning parameters. |
| `rake_params` | `RakeParams | None` | `None` | RAKE-specific tuning parameters. |

---

### OcrCacheStats

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `total_files` | `int` | — | Total files |
| `total_size_mb` | `float` | — | Total size mb |

---

### PaddleOcrConfig

Configuration for PaddleOCR backend.

Configures PaddleOCR text detection and recognition with multi-language support.
Uses a builder pattern for convenient configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `language` | `str` | — | Language code (e.g., "en", "ch", "jpn", "kor", "deu", "fra") |
| `cache_dir` | `str | None` | `None` | Optional custom cache directory for model files |
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

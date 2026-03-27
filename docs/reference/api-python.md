# Python API Reference

Complete reference for the Kreuzberg Python API.

## Installation

```bash title="Terminal"
pip install kreuzberg
```

**With EasyOCR:**

```bash title="Terminal"
pip install "kreuzberg[easyocr]"
```

**With API server:**

```bash title="Terminal"
pip install "kreuzberg[api]"
```

**With all features:**

```bash title="Terminal"
pip install "kreuzberg[all]"
```

## Core Functions

### batch_extract_bytes()

Extract content from multiple byte arrays in parallel (asynchronous).

**Signature:**

```python title="Python"
async def batch_extract_bytes(
    data_list: list[bytes | bytearray],
    mime_types: list[str],
    config: ExtractionConfig | None = None,
    *,
    easyocr_kwargs: dict[str, Any] | None = None,
) -> list[ExtractionResult]
```

**Parameters:**

Same as [`batch_extract_bytes_sync()`](#batch_extract_bytes_sync).

**Returns:**

- `list[ExtractionResult]`: List of extraction results (one per data item)

---

### batch_extract_bytes_sync()

Extract content from multiple byte arrays in parallel (synchronous).

**Signature:**

```python title="Python"
def batch_extract_bytes_sync(
    data_list: list[bytes | bytearray],
    mime_types: list[str],
    config: ExtractionConfig | None = None,
    *,
    easyocr_kwargs: dict[str, Any] | None = None,
) -> list[ExtractionResult]
```

**Parameters:**

- `data_list` (list[bytes | bytearray]): List of file contents as bytes/bytearray
- `mime_types` (list[str]): List of MIME types (one per data item, same length as data_list)
- `config` (ExtractionConfig | None): Extraction configuration applied to all items
- `easyocr_kwargs` (dict | None): EasyOCR initialization options


**Returns:**

- `list[ExtractionResult]`: List of extraction results (one per data item)

---

### batch_extract_files()

Extract content from multiple files in parallel (asynchronous).

**Signature:**

```python title="Python"
async def batch_extract_files(
    paths: list[str | Path],
    config: ExtractionConfig | None = None,
    *,
    easyocr_kwargs: dict[str, Any] | None = None,
) -> list[ExtractionResult]
```

**Parameters:**

Same as [`batch_extract_files_sync()`](#batch_extract_files_sync).

**Returns:**

- `list[ExtractionResult]`: List of extraction results (one per file)

---

### batch_extract_files_sync()

Extract content from multiple files in parallel (synchronous).

**Signature:**

```python title="Python"
def batch_extract_files_sync(
    paths: list[str | Path],
    config: ExtractionConfig | None = None,
    *,
    easyocr_kwargs: dict[str, Any] | None = None,
) -> list[ExtractionResult]
```

**Parameters:**

- `paths` (list[str | Path]): List of file paths to extract
- `config` (ExtractionConfig | None): Extraction configuration applied to all files
- `easyocr_kwargs` (dict | None): EasyOCR initialization options


**Returns:**

- `list[ExtractionResult]`: List of extraction results (one per file)

**Examples:**

```python title="basic_extraction.py"
from kreuzberg import batch_extract_files_sync

paths = ["doc1.pdf", "doc2.docx", "doc3.xlsx"]
results = batch_extract_files_sync(paths)

for path, result in zip(paths, results):
    print(f"{path}: {len(result.content)} characters")
```

---

### extract_bytes()

Extract content from bytes (asynchronous).

**Signature:**

```python title="Python"
async def extract_bytes(
    data: bytes | bytearray,
    mime_type: str,
    config: ExtractionConfig | None = None,
    *,
    easyocr_kwargs: dict[str, Any] | None = None,
) -> ExtractionResult
```

**Parameters:**

Same as [`extract_bytes_sync()`](#extract_bytes_sync).

**Returns:**

- `ExtractionResult`: Extraction result containing content, metadata, and tables

---

### extract_bytes_sync()

Extract content from bytes (synchronous).

**Signature:**

```python title="Python"
def extract_bytes_sync(
    data: bytes | bytearray,
    mime_type: str,
    config: ExtractionConfig | None = None,
    *,
    easyocr_kwargs: dict[str, Any] | None = None,
) -> ExtractionResult
```

**Parameters:**

- `data` (bytes | bytearray): File content as bytes or bytearray
- `mime_type` (str): MIME type of the data (required for format detection)
- `config` (ExtractionConfig | None): Extraction configuration. Uses defaults if None
- `easyocr_kwargs` (dict | None): EasyOCR initialization options


**Returns:**

- `ExtractionResult`: Extraction result containing content, metadata, and tables

**Examples:**

```python title="basic_extraction.py"
from kreuzberg import extract_bytes_sync

with open("document.pdf", "rb") as f:
    data = f.read()

result = extract_bytes_sync(data, "application/pdf")
print(result.content)
```

---

### extract_file()

Extract content from a file (asynchronous).

**Signature:**

```python title="Python"
async def extract_file(
    file_path: str | Path,
    mime_type: str | None = None,
    config: ExtractionConfig | None = None,
    *,
    easyocr_kwargs: dict[str, Any] | None = None,
) -> ExtractionResult
```

**Parameters:**

Same as [`extract_file_sync()`](#extract_file_sync).

**Returns:**

- `ExtractionResult`: Extraction result containing content, metadata, and tables

**Examples:**

```python title="basic_extraction.py"
import asyncio
from kreuzberg import extract_file

async def main():
    result = await extract_file("document.pdf")
    print(result.content)

async def main():
    result = await extract_file("document.pdf")
    print(result.content)

asyncio.run(main())
```

---

### extract_file_sync()

Extract content from a file (synchronous).

**Signature:**

```python title="Python"
def extract_file_sync(
    file_path: str | Path,
    mime_type: str | None = None,
    config: ExtractionConfig | None = None,
    *,
    easyocr_kwargs: dict[str, Any] | None = None,
) -> ExtractionResult
```

**Parameters:**

- `file_path` (str | Path): Path to the file to extract
- `mime_type` (str | None): Optional MIME type hint. If None, MIME type is auto-detected from file extension and content
- `config` (ExtractionConfig | None): Extraction configuration. Uses defaults if None
- `easyocr_kwargs` (dict | None): EasyOCR initialization options (languages, use_gpu, beam_width, etc.)


**Returns:**

- `ExtractionResult`: Extraction result containing content, metadata, and tables

**Raises:**

- `KreuzbergError`: Base exception for all extraction errors
- `ValidationError`: Invalid configuration or file path
- `ParsingError`: Document parsing failure
- `OCRError`: OCR processing failure
- `MissingDependencyError`: Required system dependency not found

**Example - Basic usage:**

```python title="basic_extraction.py"
from kreuzberg import extract_file_sync

result = extract_file_sync("document.pdf")
print(result.content)
print(f"Pages: {result.metadata['page_count']}")
```

**Example - With OCR:**

```python title="with_ocr.py"
from kreuzberg import extract_file_sync, ExtractionConfig, OcrConfig

config = ExtractionConfig(
    ocr=OcrConfig(backend="tesseract", language="eng")
)
result = extract_file_sync("scanned.pdf", config=config)
```

**Example - With EasyOCR custom options:**

```python title="easyocr_custom.py"
from kreuzberg import extract_file_sync, ExtractionConfig, OcrConfig

config = ExtractionConfig(
    ocr=OcrConfig(backend="easyocr", language="eng")
)
result = extract_file_sync(
    "scanned.pdf",
    config=config,
    easyocr_kwargs={"use_gpu": True, "beam_width": 10}
)
```

---

## Configuration

### ExtractionConfig

!!! warning "Deprecated API"
The `force_ocr` parameter has been deprecated in favor of the new `ocr` configuration object.

    **Old pattern (no longer supported):**
    ```python
    config = ExtractionConfig(force_ocr=True)
    ```

    **New pattern:**
    ```python
    config = ExtractionConfig(
        ocr=OcrConfig(backend="tesseract")
    )
    ```

    The new approach provides more granular control over OCR behavior through the `OcrConfig` object.

Main configuration class for extraction operations.

**Fields:**

- `chunking` (`ChunkingConfig | None`): Text chunking configuration. Default: `None`
- `concurrency` (`ConcurrencyConfig | None`) <span class="version-badge">v4.5.0</span>: Concurrency configuration. Default: `None`
- `enable_quality_processing` (`bool`): Enable quality post-processing. Default: `True`
- `force_ocr` (`bool`): Force OCR processing even for searchable documents. Default: `False`
- `html_options` (`HtmlConversionOptions | None`): HTML-specific conversion options. Default: `None`
- `images` (`ImageExtractionConfig | None`): Image extraction configuration. Default: `None`
- `include_document_structure` (`bool`): Include hierarchical document structure in the result. Default: `False`
- `language_detection` (`LanguageDetectionConfig | None`): Language detection settings. Default: `None`
- `layout` (`LayoutDetectionConfig | None`): Layout detection configuration. Default: `None`
- `max_concurrent_extractions` (`int | None`): Max concurrent batch extractions. Default: `None`
- `ocr` (`OcrConfig | None`): OCR configuration. Default: `None`
- `output_format` (`str`): Output content format (plain, markdown, djot, html). Default: `"plain"`
- `pages` (`PageConfig | None`): Page extraction settings. Default: `None`
- `pdf_options` (`PdfConfig | None`): PDF-specific options. Default: `None`
- `postprocessor` (`PostProcessorConfig | None`): Post-processing settings. Default: `None`
- `result_format` (`str`): Result layout (unified, element_based). Default: `"unified"`
- `token_reduction` (`TokenReductionConfig | None`): Token reduction settings. Default: `None`
- `use_cache` (`bool`): Enable result caching. Default: `True`

**Example:**

```python title="config.py"
from kreuzberg import ExtractionConfig, OcrConfig, PdfConfig

config = ExtractionConfig(
    ocr=OcrConfig(backend="tesseract", language="eng"),
    pdf_options=PdfConfig(
        passwords=["password1", "password2"],
        extract_images=True
    )
)

result = extract_file_sync("document.pdf", config=config)
```

**Configuration loading:**

- `ExtractionConfig.from_file(path: str | Path)` → `ExtractionConfig`: Load configuration from a file (`.toml`, `.yaml`, or `.json` by extension).
- `ExtractionConfig.discover()` → `ExtractionConfig`: Discover config from `KREUZBERG_CONFIG_PATH` or search for `kreuzberg.toml` / `kreuzberg.yaml` / `kreuzberg.json` in current and parent directories (raises if not found).

Module-level:

- `load_extraction_config_from_file(path)` → `ExtractionConfig`
- `discover_extraction_config()` → `ExtractionConfig | None` (returns None if no config file found)

---

### FileExtractionConfig <span class="version-badge">v4.5.0</span>

Per-file extraction configuration overrides for batch operations. All fields are optional — `None` means "use the batch-level default."

**Fields:**

- `enable_quality_processing` (bool | None): Override quality post-processing
- `ocr` (OcrConfig | None): Override OCR configuration
- `force_ocr` (bool | None): Override force OCR
- `chunking` (ChunkingConfig | None): Override chunking
- `images` (ImageExtractionConfig | None): Override image extraction
- `pdf_options` (PdfConfig | None): Override PDF options
- `token_reduction` (TokenReductionConfig | None): Override token reduction
- `language_detection` (LanguageDetectionConfig | None): Override language detection
- `pages` (PageConfig | None): Override page extraction
- `keywords` (KeywordConfig | None): Override keyword extraction
- `postprocessor` (PostProcessorConfig | None): Override post-processing
- `html_options` (HtmlConversionOptions | None): Override HTML conversion
- `result_format` (str | None): Override result format
- `output_format` (str | None): Override output format
- `include_document_structure` (bool | None): Override document structure
- `layout` (LayoutDetectionConfig | None): Override layout detection

**Example:**

```python title="file_extraction_config.py"
from kreuzberg import FileExtractionConfig, OcrConfig

# Override only OCR for a specific file
per_file = FileExtractionConfig(
    force_ocr=True,
    ocr=OcrConfig(backend="tesseract", language="deu"),
)
```

See [Configuration Reference](configuration.md#fileextractionconfig) for full details on merge semantics.

---

### OcrConfig

OCR processing configuration.

**Fields:**

- `backend` (str): OCR backend to use. Options: "tesseract", "easyocr", "paddleocr". Default: "tesseract"
- `language` (str): Language code for OCR (ISO 639-3). Default: "eng"
- `tesseract_config` (TesseractConfig | None): Tesseract-specific configuration. Default: None
- `model_tier` (str | None): <span class="version-badge">v4.5.0</span> PaddleOCR model tier: "mobile" (lightweight, ~21MB total, fast) or "server" (high accuracy, ~172MB, best with GPU). Default: "mobile"
- `padding` (int | None): <span class="version-badge">v4.5.0</span> Padding in pixels (0-100) added around the image before PaddleOCR detection. Default: 10

**Example - Basic OCR:**

```python title="with_ocr.py"
from kreuzberg import OcrConfig

ocr_config = OcrConfig(backend="tesseract", language="eng")
```

**Example - With EasyOCR:**

```python title="with_ocr.py"
from kreuzberg import OcrConfig

ocr_config = OcrConfig(backend="easyocr", language="en")
```

---

### TesseractConfig

Tesseract OCR backend configuration.

**Fields (common):**

- `psm` (int): Page segmentation mode (0-13). Default: 3 (auto)
- `oem` (int): OCR engine mode (0-3). Default: 3 (Auto - Tesseract chooses based on build)
- `enable_table_detection` (bool): Enable table detection and extraction. Default: True
- `tessedit_char_whitelist` (str): Character whitelist (e.g., "0123456789" for digits only). Empty string = all characters. Default: ""
- `tessedit_char_blacklist` (str): Character blacklist. Empty string = none. Default: ""
- `language` (str): OCR language (ISO 639-3). Default: "eng"
- `min_confidence` (float): Minimum confidence (0.0-1.0) for accepting OCR results. Default: 0.0
- `preprocessing` (ImagePreprocessingConfig | None): Image preprocessing before OCR. Default: None
- `output_format` (str): OCR output format. Default: "markdown"

Additional fields (table thresholds, cache, tessedit options, etc.) are available; see the type stub for the full list.

**Example:**

```python title="basic_extraction.py"
from kreuzberg import OcrConfig, TesseractConfig

config = ExtractionConfig(
    ocr=OcrConfig(
        backend="tesseract",
        language="eng",
        tesseract_config=TesseractConfig(
            psm=6,
            enable_table_detection=True,
            tessedit_char_whitelist="0123456789"
        )
    )
)
```

---

### PdfConfig

PDF-specific configuration.

**Fields:**

- `allow_single_column_tables` (`bool`) <span class="version-badge">v4.5.0</span>: Allow extraction of single-column tables. Default: `False`
- `extract_images` (`bool`): Extract images from PDF documents.
  Default: `False`
- `passwords` (`list[str] | None`): List of passwords to try when opening
  encrypted PDFs. Try each password in order until one succeeds.
  Default: None
- `extract_metadata` (`bool`): Extract PDF metadata (title, author, creation date,
  etc.). Default: `True`
- `hierarchy` (`HierarchyConfig | None`): Document hierarchy detection configuration
  for detecting document structure and organization. `None` = no hierarchy detection.
  Default: `None`

**Example:**

```python title="basic_extraction.py"
from kreuzberg import PdfConfig

pdf_config = PdfConfig(
    passwords=["password1", "password2"],
    extract_images=True,
    extract_metadata=True
)
```

---

### ConcurrencyConfig <span class="version-badge">v4.5.0</span>

Concurrency configuration for controlling parallel extraction.

**Fields:**

- `max_threads` (`int | None`): Maximum number of concurrent threads. Default: `None` (use system default)

**Example:**

```python title="concurrency_config.py"
from kreuzberg import ConcurrencyConfig, ExtractionConfig

config = ExtractionConfig(
    concurrency=ConcurrencyConfig(max_threads=4)
)
```

---

---

### HierarchyConfig

Document hierarchy detection configuration (used with `PdfConfig.hierarchy`).

**Fields:**

- `enabled` (bool): Enable hierarchy detection. Default: True
- `k_clusters` (int): Number of clusters for k-means clustering. Default: 6
- `include_bbox` (bool): Include bounding box information in hierarchy output. Default: True
- `ocr_coverage_threshold` (float | None): Optional threshold for OCR coverage before enabling hierarchy detection. Default: None

---

### LayoutDetectionConfig <span class="version-badge">v4.5.0</span>

Layout detection configuration (requires `layout-detection` feature).

**Fields:**

- `preset` (str): Model selection preset. `"fast"` (YOLOv8) or `"accurate"` (RT-DETR). Default: `"fast"`
- `confidence_threshold` (float | None): Confidence threshold for layout detection (0.0-1.0). Default: `None`
- `apply_heuristics` (bool): Apply post-processing heuristics to improve layout grouping. Default: `True`

---

---

### PageConfig

Page extraction and tracking configuration.

**Fields:**

- `extract_pages` (bool): Enable page tracking and per-page extraction. Default: False
- `insert_page_markers` (bool): Insert page markers into `content`. Default: False
- `marker_format` (str): Marker template containing `{page_num}`. Default: `"\n\n<!-- PAGE {page_num} -->\n\n"`

---

### ChunkingConfig

Text chunking configuration for splitting long documents.

**Fields:**

- `max_chars` (int): Maximum characters per chunk. Default: 1000
- `max_overlap` (int): Overlap between chunks in characters. Default: 200
- `embedding` (EmbeddingConfig | None): Embedding configuration for generating embeddings. Default: None
- `preset` (str | None): Chunking preset to use (e.g. from `list_embedding_presets()`). Default: None
- `sizing_type` (str | None): How chunk size is measured. Options: `"characters"` (default) or `"tokenizer"` (use a HuggingFace tokenizer). Default: None (characters)
- `sizing_model` (str | None): HuggingFace model ID for tokenizer-based sizing (e.g. `"bert-base-uncased"`). Required when `sizing_type="tokenizer"`. Default: None
- `sizing_cache_dir` (str | None): Optional directory to cache downloaded tokenizer files. Default: None
- `chunker_type` (str | None): Type of chunker to use. Options: `"text"` (default), `"markdown"`, `"yaml"`. Default: None (text)
- `prepend_heading_context` (bool | None): When True, prepends heading hierarchy path to each chunk's content. Most useful with `chunker_type="markdown"`. Default: None (False)

**Example:**

```python title="basic_extraction.py"
from kreuzberg import ChunkingConfig

chunking_config = ChunkingConfig(
    max_chars=1000,
    max_overlap=200
)
```

---

### LanguageDetectionConfig

Language detection configuration.

**Fields:**

- `enabled` (bool): Enable language detection. Default: True
- `min_confidence` (float): Minimum confidence threshold (0.0-1.0). Default: 0.8
- `detect_multiple` (bool): Detect multiple languages in the document. When False, only the most confident language is returned. Default: False

**Example:**

```python title="basic_extraction.py"
from kreuzberg import LanguageDetectionConfig

lang_config = LanguageDetectionConfig(
    enabled=True,
    min_confidence=0.7
)
```

---

### KeywordConfig

Keyword extraction configuration (used with `ExtractionConfig.keywords`).

**Fields:**

- `algorithm` (KeywordAlgorithm): Algorithm to use. Values: `KeywordAlgorithm.Yake`, `KeywordAlgorithm.Rake`. Default: Yake
- `max_keywords` (int): Maximum number of keywords to extract. Default: 10
- `min_score` (float): Minimum score threshold. Default: 0.0
- `ngram_range` (tuple[int, int]): N-gram range (min, max). Default: (1, 3)
- `language` (str | None): Optional language hint. Default: "en"
- `yake_params` (YakeParams | None): YAKE-specific tuning (e.g. `window_size`). Default: None
- `rake_params` (RakeParams | None): RAKE-specific tuning (`min_word_length`, `max_words_per_phrase`). Default: None

---

### ImageExtractionConfig

Image extraction configuration.

**Fields:**

- `extract_images` (bool): Enable image extraction from documents. Default: True
- `target_dpi` (int): Target DPI for image normalization. Default: 300
- `max_image_dimension` (int): Maximum width or height for extracted images. Default: 4096
- `auto_adjust_dpi` (bool): Automatically adjust DPI based on image content. Default: True
- `min_dpi` (int): Minimum DPI threshold. Default: 72
- `max_dpi` (int): Maximum DPI threshold. Default: 600

---

### TokenReductionConfig

Token reduction configuration for compressing extracted text.

**Fields:**

- `mode` (str): Token reduction mode. Options: `"off"`, `"light"`, `"moderate"`, `"aggressive"`, `"maximum"`. Default: `"off"`
  - `"off"`: No token reduction
  - `"light"`: Remove extra whitespace and redundant punctuation
  - `"moderate"`: Also remove common filler words and some formatting
  - `"aggressive"`: Also remove longer stopwords and collapse similar phrases
  - `"maximum"`: Maximum reduction while preserving semantic content
- `preserve_important_words` (bool): Preserve important words (capitalized, technical terms) even in aggressive reduction modes. Default: True

---

### PostProcessorConfig

Post-processing configuration.

**Fields:**

- `enabled` (`bool`): Enable post-processors in the extraction pipeline. Default: True
- `enabled_processors` (`list[str] | None`): Whitelist of processor names to run. If specified, only these processors are executed. None = run all enabled. Default: None
- `disabled_processors` (`list[str] | None`): Blacklist of processor names to skip. If specified, these processors are not executed. None = none disabled. Default: None

---

### ImagePreprocessingConfig

Image preprocessing configuration for OCR (used with `TesseractConfig.preprocessing`).

**Fields:**

- `target_dpi` (int): Target DPI for image preprocessing. Default: 300
- `auto_rotate` (bool): Auto-rotate images based on orientation. Default: True
- `deskew` (bool): Correct skewed images. Default: True
- `denoise` (bool): Apply denoising filter. Default: False
- `contrast_enhance` (bool): Enhance contrast. Default: False
- `binarization_method` (str): Binarization method (e.g., "otsu"). Default: "otsu"
- `invert_colors` (bool): Invert colors (e.g., white text on black). Default: False

---

## Results & Types

### ExtractionResult

Result object returned by all extraction functions.

**Type Definition:**

```python title="Python"
class ExtractionResult:
    annotations: list[PdfAnnotation] | None
    chunks: list[Chunk] | None
    content: str
    detected_languages: list[str] | None
    djot_content: DjotContent | None
    document: DocumentStructure | None
    elements: list[Element] | None
    extracted_keywords: list[ExtractedKeyword] | None
    images: list[ExtractedImage] | None
    metadata: Metadata
    metadata_json: str
    mime_type: str
    ocr_elements: list[OcrElement] | None
    output_format: str | None
    pages: list[PageContent] | None
    processing_warnings: list[ProcessingWarning]
    quality_score: float | None
    result_format: str | None
    tables: list[ExtractedTable]
    def get_page_count(self) -> int: ...
    def get_chunk_count(self) -> int: ...
    def get_detected_language(self) -> str | None: ...
    def get_metadata_field(self, field_name: str) -> Any | None: ...
```

**Fields:**

- `annotations` (list[PdfAnnotation] | None): Extracted PDF annotations and highlights
- `chunks` (list[Chunk] | None): Text chunks when chunking is configured
- `content` (str): Extracted text content
- `detected_languages` (list[str] | None): Detected language codes (ISO 639-1)
- `djot_content` (DjotContent | None): Structured djot content when `output_format="djot"`
- `document` (DocumentStructure | None): Hierarchical document strucure when `include_document_structure=True`
- `elements` (list[Element] | None): Semantic elements when using element-based layout
- `extracted_keywords` (list[ExtractedKeyword] | None): Keywords extracted with RAKE/YAKE
- `images` (list[ExtractedImage] | None): Extracted images
- `metadata` (Metadata): Document metadata (format-specific fields)
- `metadata_json` (str): Raw JSON string of all metadata
- `mime_type` (str): MIME type of the document
- `ocr_elements` (list[OcrElement] | None): Granular OCR blocks with bounding boxes
- `output_format` (str | None): Effective output format
- `pages` (list[PageContent] | None): Per-page content when enabled
- `processing_warnings` (list[ProcessingWarning]): Non-fatal warnings during extraction
- `quality_score` (float | None): Document quality score
- `result_format` (str | None): Layout format (unified or element_based)
- `tables` (list[ExtractedTable]): List of extracted tables

**Methods:**

- `get_page_count()` → int: Number of pages (from metadata when available)
- `get_chunk_count()` → int: Number of chunks (0 if chunking disabled)
- `get_detected_language()` → str | None: Primary detected language code
- `get_metadata_field(field_name: str)` → Any | None: Get a metadata field by name

**Example:**

```python title="basic_extraction.py"
result = extract_file_sync("document.pdf")

print(f"Content: {result.content}")
print(f"MIME type: {result.mime_type}")
print(f"Page count: {result.metadata.get('page_count')}")
print(f"Tables: {len(result.tables)}")

if result.detected_languages:
    print(f"Languages: {', '.join(result.detected_languages)}")
```

#### pages

**Type**: `list[PageContent] | None`

Per-page extracted content when page extraction is enabled via `PageConfig.extract_pages = true`.

Each page contains:

- Page number (1-indexed)
- Text content for that page
- Tables on that page
- Images on that page

**Example:**

```python title="page_extraction.py"
from kreuzberg import extract_file_sync, ExtractionConfig, PageConfig

config = ExtractionConfig(
    pages=PageConfig(extract_pages=True)
)

result = extract_file_sync("document.pdf", config=config)

if result.pages:
    for page in result.pages:
        print(f"Page {page.page_number}:")
        print(f"  Content: {len(page.content)} chars")
        print(f"  Tables: {len(page.tables)}")
        print(f"  Images: {len(page.images)}")
```

---

### Accessing Per-Page Content

When page extraction is enabled, access individual pages and iterate over them:

```python title="iterate_pages.py"
from kreuzberg import extract_file_sync, ExtractionConfig, PageConfig

config = ExtractionConfig(
    pages=PageConfig(
        extract_pages=True,
        insert_page_markers=True,
        marker_format="\n\n--- Page {page_num} ---\n\n"
    )
)

result = extract_file_sync("document.pdf", config=config)

# Access combined content with page markers
print("Combined content with markers:")
print(result.content[:500])
print()

# Access per-page content
if result.pages:
    for page in result.pages:
        print(f"Page {page.page_number}:")
        print(f"  {page.content[:100]}...")
        if page.tables:
            print(f"  Found {len(page.tables)} table(s)")
        if page.images:
            print(f"  Found {len(page.images)} image(s)")
```

---

### Metadata

Strongly-typed metadata dictionary. Fields vary by document format.

**Standard 13 Fields:**

- `authors` (list[str]): Primary author(s)
- `created_at` (str): Creation timestamp (ISO 8601)
- `created_by` (str): User/agent who created the document
- `custom` (dict[str, Any]): Custom metadata fields (replaces the deprecated `additional`)
- `date` (str): Document date string
- `format_type` (str): Document format type (e.g., "pdf", "docx")
- `keywords` (list[str]): Document keywords
- `language` (str): Primary document language (ISO 639-1 code)
- `modified_at` (str): Last modification timestamp
- `modified_by` (str): User who last modified the document
- `page_count` (int): Total number of pages
- `producer` (str): Document producer/generator
- `subject` (str): Document subject/description
- `title` (str): Document title


**Excel-Specific Fields** (when `format_type == "excel"`):

- `sheet_count` (int): Number of sheets
- `sheet_names` (list[str]): List of sheet names

**Email-Specific Fields** (when `format_type == "email"`):

- `from_email` (str): Sender email address
- `from_name` (str): Sender name
- `to_emails` (list[str]): Recipient email addresses
- `cc_emails` (list[str]): CC email addresses
- `bcc_emails` (list[str]): BCC email addresses
- `message_id` (str): Email message ID
- `attachments` (list[str]): List of attachment filenames

**Example:**

```python title="basic_extraction.py"
result = extract_file_sync("document.pdf")
metadata = result.metadata

if metadata.get("format_type") == "pdf":
    print(f"Title: {metadata.get('title')}")
    print(f"Authors: {metadata.get('authors')}")
    print(f"Pages: {metadata.get('page_count')}")
```

See the Types Reference for complete metadata field documentation.

---

### ExtractedTable

Extracted table structure. The API type is **`ExtractedTable`** (same shape as below).

**Type Definition:**

```python title="Python"
class ExtractedTable:
    cells: list[list[str]]
    markdown: str
    page_number: int
```

**Fields:**

- `cells` (list[list[str]]): 2D array of table cells (rows x columns)
- `markdown` (str): Table rendered as markdown
- `page_number` (int): Page number where table was found

**Example:**

```python title="basic_extraction.py"
result = extract_file_sync("invoice.pdf")

for table in result.tables:
    print(f"Table on page {table.page_number}:")
    print(table.markdown)
    print()
```

---

### ChunkMetadata

Metadata for a single text chunk.

**Type Definition:**

```python title="Python"
class ChunkMetadata(TypedDict, total=False):
    byte_start: int
    byte_end: int
    chunk_index: int
    total_chunks: int
    token_count: int | None
    first_page: int
    last_page: int
    heading_context: HeadingContext | None
```

**Fields:**

- `byte_start` (int): UTF-8 byte offset in content (inclusive)
- `byte_end` (int): UTF-8 byte offset in content (exclusive)
- `chunk_index` (int): Zero-based index of this chunk in the document
- `total_chunks` (int): Total number of chunks for the document
- `token_count` (int | None): Estimated token count (if configured)
- `first_page` (int): First page this chunk appears on (1-indexed, only when page boundaries available)
- `last_page` (int): Last page this chunk appears on (1-indexed, only when page boundaries available)
- `heading_context` (HeadingContext | None): Heading hierarchy when using Markdown chunker. Only populated when chunker_type is set to markdown.

**Page tracking:** When `PageStructure.boundaries` is available and chunking is enabled, `first_page` and `last_page` are automatically calculated based on byte offsets.

**Example:**

```python title="chunk_metadata.py"
from kreuzberg import extract_file_sync, ExtractionConfig, ChunkingConfig, PageConfig

config = ExtractionConfig(
    chunking=ChunkingConfig(max_chars=500, max_overlap=50),
    pages=PageConfig(extract_pages=True)
)

result = extract_file_sync("document.pdf", config=config)

if result.chunks:
    for chunk in result.chunks:
        meta = chunk.metadata
        page_info = ""
        if meta.get('first_page'):
            if meta['first_page'] == meta.get('last_page'):
                page_info = f" (page {meta['first_page']})"
            else:
                page_info = f" (pages {meta['first_page']}-{meta.get('last_page')})"

        print(f"Chunk [{meta['byte_start']}:{meta['byte_end']}]: {len(chunk.content)} chars{page_info}")
```

---

---

## Extensibility

### Custom Post-Processors

Create custom post-processors to add processing logic to the extraction pipeline.

**Protocol:**

```python title="Python"
from kreuzberg import PostProcessorProtocol, ExtractionResult

class PostProcessorProtocol:
    def name(self) -> str:
        """Return unique processor name"""
        ...

    def process(self, result: ExtractionResult) -> ExtractionResult:
        """Process extraction result and return modified result"""
        ...

    def processing_stage(self) -> str:
        """Return processing stage: 'early', 'middle', or 'late'"""
        ...
```

Optional lifecycle methods: `initialize()` (called when registered), `shutdown()` (called when unregistered).

**Example:**

```python title="basic_extraction.py"
from kreuzberg import (
    PostProcessorProtocol,
    ExtractionResult,
    register_post_processor
)

class CustomProcessor:
    def name(self) -> str:
        return "custom_processor"

    def process(self, result: ExtractionResult) -> ExtractionResult:
        # Add custom field to metadata
        result.metadata["custom_field"] = "custom_value"
        return result

    def processing_stage(self) -> str:
        return "middle"

# Register the processor
register_post_processor(CustomProcessor())

# Now all extractions will use this processor
result = extract_file_sync("document.pdf")
print(result.metadata["custom_field"])  # "custom_value"
```

**Managing Processors:**

```python title="basic_extraction.py"
from kreuzberg import (
    register_post_processor,
    unregister_post_processor,
    clear_post_processors
)

# Register
register_post_processor(CustomProcessor())

# Unregister by name
unregister_post_processor("custom_processor")

# Clear all processors
clear_post_processors()
```

---

### Custom Validators

Create custom validators to validate extraction results.

**ValidatorProtocol:** Implement:

- `name() -> str`
- `validate(result: ExtractionResult) -> None` (raise to fail)
- Optional: `priority() -> int` (default 50, higher runs first)
- Optional: `should_validate(result: ExtractionResult) -> bool` (default True)
- Optional lifecycle: `initialize()`, `shutdown()`

**Functions:**

```python title="custom_validator.py"
from kreuzberg import register_validator, unregister_validator, clear_validators

# Register a validator
register_validator(validator)

# Unregister by name
unregister_validator("validator_name")

# Clear all validators
clear_validators()
```

---

## Error Handling

All errors inherit from **`KreuzbergError`**. See [Error Handling Reference](errors.md) for complete documentation.

**Exception Hierarchy:**

- **`KreuzbergError`** — Base exception for all extraction errors
  - `ValidationError` — Invalid configuration or input
  - `ParsingError` — Document parsing failure
  - `OCRError` — OCR processing failure
  - `MissingDependencyError` — Missing optional dependency
  - `CacheError` — Cache read/write failure
  - `ImageProcessingError` — Image processing failure
  - `PluginError` — Plugin (post-processor, validator, OCR backend) failure

**Example:**

```python title="error_handling.py"
from kreuzberg import (
    extract_file_sync,
    KreuzbergError,
    ValidationError,
    ParsingError,
    MissingDependencyError
)

try:
    result = extract_file_sync("document.pdf")
except ValidationError as e:
    print(f"Invalid input: {e}")
except ParsingError as e:
    print(f"Failed to parse document: {e}")
except MissingDependencyError as e:
    print(f"Missing dependency: {e}")
    print(f"Install with: {e.install_command}")
except KreuzbergError as e:
    print(f"Extraction failed: {e}")
```

**Error inspection:**

- `get_last_error_code()` → int | None
- `get_error_details()` → dict (message, error_code, error_type, source_file, source_line, is_panic, etc.)
- `classify_error(message: str)` → int
- `error_code_name(code: int)` → str

See [Error Handling Reference](errors.md) for detailed error documentation and best practices.

---

## PDF Rendering

!!! info "Added in v4.6.2"

### render_pdf_page()

Render a single PDF page as a PNG image.

**Signature:**

```python title="Python"
def render_pdf_page(
    file_path: str | Path,
    page_index: int,
    *,
    dpi: int = 150,
) -> bytes
```

**Parameters:**

- `file_path` (str | Path): Path to the PDF file
- `page_index` (int): Zero-based page index to render
- `dpi` (int): Resolution for rendering (default 150)

**Returns:**

- `bytes`: PNG-encoded bytes for the requested page

**Example:**

```python title="render_single_page.py"
from kreuzberg import render_pdf_page

png_bytes = render_pdf_page("document.pdf", 0)
with open("first_page.png", "wb") as f:
    f.write(png_bytes)
```

---

## Utilities

- **`detect_mime_type(data: bytes | bytearray)`** → str: Detect MIME type from file bytes (e.g. for `extract_bytes_sync`).
- **`detect_mime_type_from_path(path: str | Path)`** → str: Detect MIME type from file path (reads file).
- **`get_extensions_for_mime(mime_type: str)`** → list[str]: Return file extensions associated with a MIME type.

---

## Version Information

```python title="basic_extraction.py"
import kreuzberg

print(kreuzberg.__version__)
```

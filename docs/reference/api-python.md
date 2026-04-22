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

### Hardware Acceleration

To enable GPU acceleration (CUDA) for PaddleOCR, layout detection, and embeddings, install the GPU version of ONNX Runtime:

```bash title="Terminal"
pip install onnxruntime-gpu
```

Kreuzberg will automatically detect and use the GPU-enabled library from your Python environment.

## Core Functions

### Batch_extract_bytes()

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

### Batch_extract_bytes_sync()

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

### Batch_extract_files()

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

### Batch_extract_files_sync()

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

### Extract_bytes()

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

### Extract_bytes_sync()

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

### Extract_file()

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

### Extract_file_sync()

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

!!! Warning "Deprecated API"
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
- `content_filter` (`ContentFilterConfig | None`) <span class="version-badge">v4.8.0</span>: Header, footer, watermark, and repeating-text filtering. Default: `None` (each extractor uses its built-in defaults). See [ContentFilterConfig](configuration.md#contentfilterconfig).
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
- `content_filter` (ContentFilterConfig | None) <span class="version-badge">v4.8.0</span>: Override header/footer/watermark/repeating-text filtering. See [ContentFilterConfig](configuration.md#contentfilterconfig).
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
- `paddle_ocr_config` (PaddleOcrConfig | None) <span class="version-badge">v4.9.3</span>: PaddleOCR-specific configuration. Default: None
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

### PaddleOcrConfig <span class="version-badge">v4.9.3</span>

PaddleOCR-specific configuration.

**Fields:**

- `language` (str | None): Language code for OCR. Default: None
- `cache_dir` (str | None): Directory for caching model files. Default: None
- `use_angle_cls` (bool | None): Use angle classifier for orientation correction. Default: None
- `enable_table_detection` (bool | None): Enable table detection. Default: None
- `model_tier` (str | None): Model tier ("mobile" or "server"). Default: None
- `padding` (int | None): Padding around detection boxes. Default: None
- `det_db_thresh` (float | None): Detection DB threshold. Default: None
- `det_db_box_thresh` (float | None): Detection DB box threshold. Default: None
- `det_db_unclip_ratio` (float | None): Detection DB unclip ratio. Default: None
- `det_limit_side_len` (int | None): Detection side length limit. Default: None
- `rec_batch_num` (int | None): Recognition batch size. Default: None

**Example:**

```python title="paddle_config.py"
from kreuzberg import OcrConfig, PaddleOcrConfig

config = OcrConfig(
    backend="paddleocr",
    paddle_ocr_config=PaddleOcrConfig(
        model_tier="server",
        language="chi_sim"
    )
)
```

---

### TesseractConfig

Tesseract OCR backend configuration.

**Fields (common):**

- `psm` (int): Page segmentation mode (0-13). Default: 3 (auto)
- `oem` (int): OCR engine mode (0-3). Default: 3 (Auto - Tesseract chooses based on build)
- `enable_table_detection` (bool): Enable table detection and extraction. Default: True
- `tessedit_char_whitelist` (str): Character whitelist (for example, "0123456789" for digits only). Empty string = all characters. Default: ""
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
- `preset` (str | None): Chunking preset to use (for example from `list_embedding_presets()`). Default: None
- `sizing_type` (str | None): How chunk size is measured. Options: `"characters"` (default) or `"tokenizer"` (use a HuggingFace tokenizer). Default: None (characters)
- `sizing_model` (str | None): HuggingFace model ID for tokenizer-based sizing (for example `"bert-base-uncased"`). Required when `sizing_type="tokenizer"`. Default: None
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
- `yake_params` (YakeParams | None): YAKE-specific tuning (for example `window_size`). Default: None
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
- `binarization_method` (str): Binarization method (for example, "otsu"). Default: "otsu"
- `invert_colors` (bool): Invert colors (for example, white text on black). Default: False

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
- `document` (DocumentStructure | None): Hierarchical document structure when `include_document_structure=True`
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

#### Pages

**Type**: `list[PageContent] | None`

Per-page extracted content when page extraction is enabled via `PageConfig.extract_pages = true`.

Each page contains:

- Page number (1-indexed)
- Text content for that page
- Tables on that page
- Images on that page
- Layout regions when layout detection is enabled, each with `class` (string), `confidence` (float, 0–1), `bounding_box`, and `area_fraction` (float, 0–1)

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
- `format_type` (str): Document format type (for example, "pdf", "docx")
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

Kreuzberg's plugin system lets you register custom OCR backends, post-processors, validators, and document extractors. Once registered, they're available to the Rust CLI, API server, and MCP server — not just the Python API.

### OCR Backends

Swap in a cloud OCR service, a custom engine, or a fine-tuned model. Any Python object that implements the required methods can be registered.

#### OcrBackendProtocol

Defined in `kreuzberg.ocr.protocol`. Your backend needs three methods; everything else is optional.

**Required:**

| Method | Returns | Purpose |
|--------|---------|---------|
| `name()` | `str` | Unique backend name (lowercase, no spaces) |
| `supported_languages()` | `list[str]` | ISO 639 language codes this backend handles |
| `process_image(image_bytes, language)` | `dict` | The core OCR method — takes raw image bytes, returns extracted content |

**Optional:**

| Method | Purpose |
|--------|---------|
| `process_image_file(path, language)` | Optimized path-based processing (avoids loading entire file into memory) |
| `supports_document_processing()` | Return `True` if `process_document()` is implemented |
| `process_document(path, language)` | Native multi-page processing (PDFs, multi-page TIFFs) |
| `initialize()` | Called on registration — load models, warm up GPU |
| `shutdown()` | Called on unregistration — release resources |
| `version()` | Version string (defaults to `"1.0.0"`) |

The return dict from `process_image()` and `process_document()` must include `"content"` (extracted text). `"metadata"` and `"tables"` are optional:

```python title="Python"
{
    "content": "extracted text",
    "metadata": {"width": 800, "height": 600, "confidence": 0.95},
    "tables": [
        {
            "cells": [["Header1", "Header2"], ["Cell1", "Cell2"]],
            "markdown": "| Header1 | Header2 |\n| --- | --- |\n| Cell1 | Cell2 |",
            "page_number": 1
        }
    ]
}
```

#### EasyOCRBackend

The built-in backend wrapping [EasyOCR](https://github.com/JaidedAI/EasyOCR). Supports 80+ languages, optional GPU acceleration, and multi-page document processing. Available from `kreuzberg.ocr.easyocr`.

```python title="Python"
from kreuzberg.ocr.easyocr import EasyOCRBackend

backend = EasyOCRBackend(
    languages=["en", "de"],
    use_gpu=True,
    model_storage_directory="/tmp/easyocr_models",
    beam_width=10,
)
```

| Parameter | Type | Default | Notes |
|-----------|------|---------|-------|
| `languages` | `list[str] \| None` | `None` | EasyOCR language codes; defaults to `["en"]` internally when `None` |
| `use_gpu` | `bool \| None` | `None` | `None` auto-detects CUDA availability |
| `model_storage_directory` | `str \| None` | `None` | Custom model cache path; uses EasyOCR's default when `None` |
| `beam_width` | `int` | `5` | Higher = slower but more accurate |

You usually don't need to instantiate this directly. When you set `backend="easyocr"` in `OcrConfig`, Kreuzberg auto-registers it:

```python title="Python"
from kreuzberg import extract_file_sync, ExtractionConfig, OcrConfig

config = ExtractionConfig(ocr=OcrConfig(backend="easyocr", language="en"))
result = extract_file_sync("scanned.pdf", config=config, easyocr_kwargs={"use_gpu": True})
```

#### Register_ocr_backend()

```python title="Python"
def register_ocr_backend(backend: Any) -> None
```

Validates the backend object, wraps it for Rust interop, and registers it globally. Raises `TypeError` if required methods are missing, `ValueError` if the name collides with an existing backend.

```python title="register_ocr.py"
from kreuzberg import register_ocr_backend
import httpx

class CloudOcrBackend:
    def name(self) -> str:
        return "cloud-ocr"

    def supported_languages(self) -> list[str]:
        return ["eng", "deu", "fra"]

    def process_image(self, image_bytes: bytes, language: str) -> dict:
        with httpx.Client() as client:
            resp = client.post(
                "https://api.example.com/ocr",
                files={"image": image_bytes},
                json={"language": language},
            )
            return {"content": resp.json()["text"], "metadata": {}, "tables": []}

    def initialize(self) -> None:
        pass

    def shutdown(self) -> None:
        pass

register_ocr_backend(CloudOcrBackend())
```

#### Unregister_ocr_backend()

```python title="Python"
def unregister_ocr_backend(name: str) -> None
```

Removes the backend and calls its `shutdown()` method.

#### Managing OCR Backends

```python title="manage_ocr.py"
from kreuzberg import (
    register_ocr_backend,
    unregister_ocr_backend,
    list_ocr_backends,
    clear_ocr_backends,
)

register_ocr_backend(my_backend)
print(list_ocr_backends())
unregister_ocr_backend("cloud-ocr")
clear_ocr_backends()
```

---

### Custom Post-Processors

Post-processors run after extraction to transform or enrich results. They execute in three stages: **early** (language detection, normalization), **middle** (keyword extraction, summarization), **late** (analytics, output formatting).

**Protocol** — implement these three methods:

```python title="Python"
class PostProcessorProtocol:
    def name(self) -> str: ...
    def process(self, result: ExtractionResult) -> ExtractionResult: ...
    def processing_stage(self) -> str: ...   # "early", "middle", or "late"
```

Optional: `initialize()`, `shutdown()`, `version()`.

```python title="word_count_processor.py"
from kreuzberg import register_post_processor, ExtractionResult

class WordCountProcessor:
    def name(self) -> str:
        return "word-count"

    def process(self, result: ExtractionResult) -> ExtractionResult:
        result.metadata["word_count"] = len(result.content.split())
        return result

    def processing_stage(self) -> str:
        return "late"

register_post_processor(WordCountProcessor())
```

**Managing processors:** `register_post_processor()`, `unregister_post_processor(name)`, `list_post_processors()`, `clear_post_processors()`.

---

### Custom Validators

Validators run after extraction and post-processing. If a validator raises an exception, the extraction fails. Use them for hard quality gates — minimum content length, confidence thresholds, required metadata fields.

**Required:** `name() -> str`, `validate(result) -> None` (raise to reject).

**Optional:** `priority() -> int` (default 50, higher runs first), `should_validate(result) -> bool`, `initialize()`, `shutdown()`, `version()`.

```python title="custom_validator.py"
from kreuzberg import register_validator, ExtractionResult, ValidationError

class MinLengthValidator:
    def name(self) -> str:
        return "min_length"

    def priority(self) -> int:
        return 100

    def validate(self, result: ExtractionResult) -> None:
        if len(result.content) < 50:
            raise ValidationError(f"Content too short: {len(result.content)}")

    def should_validate(self, result: ExtractionResult) -> bool:
        return True

register_validator(MinLengthValidator())
```

**Managing validators:** `register_validator()`, `unregister_validator(name)`, `list_validators()`, `clear_validators()`.

---

### Document Extractors

Document extractors are registered per-MIME type with a priority system — 0–100, with built-ins at 50. A higher priority wins; lower is used as fallback.

!!! Note "Rust-only registration"
    `register_document_extractor()` is not exposed to Python. Extractor *implementation and registration* must be done in Rust. See the [Creating Plugins Guide](../guides/plugins.md#document-extractors) for the Rust API.

The Python API covers the management side only — listing, removing, and clearing extractors that were registered from Rust:

**`list_document_extractors() -> list[str]`** — names of all currently registered extractors.

**`unregister_document_extractor(name: str) -> None`** — remove a registered extractor by name.

**`clear_document_extractors() -> None`** — remove all custom extractors.

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

### Error Introspection

When something goes wrong in the Rust core, these functions let you dig into what happened — the error code, a structured details dict, and (if a Rust panic occurred) the exact file and line in the source.

#### Get_last_error_code()

```python title="Python"
def get_last_error_code() -> int | None
```

Returns the numeric error code from the most recent FFI operation, or `None` if nothing has failed. Match against `ErrorCode` for readable comparisons:

```python title="error_introspection.py"
from kreuzberg import get_last_error_code, ErrorCode

code = get_last_error_code()
if code == ErrorCode.PANIC:
    print("A panic occurred in the Rust core")
elif code == ErrorCode.OCR_ERROR:
    print("OCR processing failed")
```

| Code | Name | Meaning |
|------|------|---------|
| 0 | `SUCCESS` | No error |
| 1 | `GENERIC_ERROR` | Unspecified error |
| 2 | `PANIC` | Rust core panic |
| 3 | `INVALID_ARGUMENT` | Invalid argument |
| 4 | `IO_ERROR` | I/O operation failed |
| 5 | `PARSING_ERROR` | Document parsing failed |
| 6 | `OCR_ERROR` | OCR processing failed |
| 7 | `MISSING_DEPENDENCY` | Required dependency unavailable |
| 8 | `EMBEDDING` | Embedding operation failed |

---

#### Get_error_details()

```python title="Python"
def get_error_details() -> dict[str, Any]
```

Returns a structured dict from the FFI layer's thread-local error storage. More useful than the error code alone — you get the message, the source location, and whether a panic was involved:

```python title="error_details.py"
from kreuzberg import extract_file_sync, get_error_details, KreuzbergError

try:
    result = extract_file_sync("corrupt.pdf")
except KreuzbergError:
    details = get_error_details()
    print(f"Error: {details['message']}")
    print(f"Type: {details['error_type']}")
    if details['is_panic']:
        print(f"Panic at {details['source_file']}:{details['source_line']}")
```

**Keys:** `message` (str), `error_code` (int), `error_type` (str), `source_file` (str | None), `source_function` (str | None), `source_line` (int), `context_info` (str | None), `is_panic` (bool).

---

#### Classify_error()

```python title="Python"
def classify_error(message: str) -> int
```

Takes a raw error message string — from an external library, a system call, wherever — and classifies it into a Kreuzberg error category. Useful for error routing in custom pipelines:

```python title="classify.py"
from kreuzberg import classify_error, error_code_name

code = classify_error("Failed to open file: permission denied")
print(f"Category: {error_code_name(code)}")  # "io"
```

Categories: 0 = Validation, 1 = Parsing, 2 = OCR, 3 = Missing dependency, 4 = I/O, 5 = Plugin, 6 = Unsupported format, 7 = Internal.

!!! Warning "Different integer space from `ErrorCode`"
    The integers returned by `classify_error()` are **not** the same as `ErrorCode` values — do not compare them directly or substitute one for the other. `ErrorCode` represents FFI-layer panic shield codes (e.g. `PANIC = 2`, `OCR_ERROR = 6`); `classify_error` returns message-based category codes with a completely different mapping (e.g. `2 = OCR`, `4 = I/O`). Use `error_code_name(code)` to get the string label rather than comparing raw integers.

---

#### Error_code_name()

```python title="Python"
def error_code_name(code: int) -> str
```

Converts a numeric error code to its human-readable name (`"validation"`, `"ocr"`, etc.). Returns `"unknown"` for out-of-range values.

---

### ErrorCode

`IntEnum` mapping the FFI panic shield error codes. Use it for readable comparisons instead of raw integers:

```python title="Python"
from kreuzberg import ErrorCode

ErrorCode.SUCCESS           # 0
ErrorCode.PANIC             # 2
ErrorCode.OCR_ERROR         # 6
ErrorCode.MISSING_DEPENDENCY # 7
ErrorCode.EMBEDDING         # 8
```

### PanicContext

When the Rust core panics, `get_last_panic_context()` returns a JSON string you can parse into a `PanicContext` dataclass. This gives you the exact source file, line number, and function where the panic happened — invaluable for bug reports.

```python title="Python"
def get_last_panic_context() -> str | None
```

Returns `None` when no panic has occurred in the current thread. Always guard against `None` before parsing:

```python title="panic_debugging.py"
from kreuzberg.exceptions import PanicContext
from kreuzberg import get_last_panic_context

context_json = get_last_panic_context()
if context_json is not None:
    ctx = PanicContext.from_json(context_json)
    print(f"Panic at {ctx.file}:{ctx.line} in {ctx.function}")
    print(f"Message: {ctx.message}")
```

**Fields:** `file`, `line`, `function`, `message`, `timestamp_secs`.

See [Error Handling Reference](errors.md) for the complete error documentation.

---

## Validation Helpers

These functions let you validate configuration values before passing them to extraction. All return `bool` (except `validate_mime_type` which returns the normalized string). All importable from `kreuzberg`.

Useful for building UIs, CLI argument validation, or pre-flight checks in pipelines.

| Function | Validates |
|----------|-----------|
| `validate_dpi(dpi: int)` | DPI within allowed range |
| `validate_language_code(code: str)` | Valid language code string |
| `validate_mime_type(mime_type: str) -> str` | Valid MIME type (returns normalized form) |
| `validate_confidence(confidence: float)` | Confidence in 0.0–1.0 range |
| `validate_ocr_backend(backend: str)` | Known OCR backend identifier |
| `validate_output_format(output_format: str)` | Valid output format string |
| `validate_tesseract_psm(psm: int)` | Valid Tesseract page segmentation mode |
| `validate_tesseract_oem(oem: int)` | Valid Tesseract OCR engine mode |
| `validate_chunking_params(max_chars: int, max_overlap: int)` | Chunk size/overlap constraints |
| `validate_binarization_method(method: str)` | Valid binarization method name |
| `validate_token_reduction_level(level: str)` | Valid token reduction level |

To get the full list of valid values for any of these, use the corresponding discovery helper:

```python title="discovery_helpers.py"
from kreuzberg import (
    get_valid_binarization_methods,
    get_valid_language_codes,
    get_valid_ocr_backends,
    get_valid_token_reduction_levels,
)

print(get_valid_language_codes())          # All valid language codes
print(get_valid_ocr_backends())           # Registered OCR backend names
print(get_valid_binarization_methods())    # Valid binarization methods
print(get_valid_token_reduction_levels())  # Valid reduction levels
```

---

## Configuration Utilities

Three helpers for working with `ExtractionConfig` objects programmatically — serializing, inspecting, and merging configs.

### Config_to_json()

```python title="Python"
def config_to_json(config: ExtractionConfig) -> str
```

Serialize a config to JSON. Useful for logging, debugging, or sending configs over the wire:

```python title="config_json.py"
from kreuzberg import ExtractionConfig, OcrConfig, config_to_json

config = ExtractionConfig(ocr=OcrConfig(backend="tesseract", language="eng"))
print(config_to_json(config))
```

### Config_get_field()

```python title="Python"
def config_get_field(config: ExtractionConfig, field_name: str) -> Any | None
```

Look up a config field by name. Returns `None` if the field doesn't exist or isn't set:

```python title="config_field.py"
from kreuzberg import ExtractionConfig, OcrConfig, config_get_field

config = ExtractionConfig(ocr=OcrConfig(backend="tesseract"))
print(config_get_field(config, "ocr"))       # OcrConfig(...)
print(config_get_field(config, "chunking"))  # None
```

### Config_merge()

```python title="Python"
def config_merge(base: ExtractionConfig, override: ExtractionConfig) -> None
```

Merge `override` into `base` in place. Fields set on `override` win; unset fields leave `base` unchanged. This is how you layer environment defaults with per-request overrides:

```python title="config_merge.py"
from kreuzberg import ExtractionConfig, OcrConfig, ChunkingConfig, config_merge

base = ExtractionConfig(ocr=OcrConfig(backend="tesseract", language="eng"))
override = ExtractionConfig(chunking=ChunkingConfig(max_chars=1000))

config_merge(base, override)
```

---

## Configuration Discovery

### Discover_extraction_config()

!!! Warning "Deprecated since v4.2.0"
    Use `load_extraction_config_from_file()` with an explicit path instead.

```python title="Python"
def discover_extraction_config() -> ExtractionConfig | None
```

Searches for a config file automatically: first checks `KREUZBERG_CONFIG_PATH`, then walks up from the current directory looking for `kreuzberg.toml`, `kreuzberg.yaml`, or `kreuzberg.json`. Returns `None` if nothing is found.

### Load_extraction_config_from_file()

```python title="Python"
def load_extraction_config_from_file(path: str | Path) -> ExtractionConfig
```

Load a config from a specific file. The format is determined by extension (`.toml`, `.yaml`, `.json`). Raises `FileNotFoundError`, `RuntimeError` (invalid content), or `ValueError` (unsupported format).

```python title="load_config.py"
from kreuzberg import load_extraction_config_from_file, extract_file_sync

config = load_extraction_config_from_file("kreuzberg.toml")
result = extract_file_sync("document.pdf", config=config)
```

---

## Embedding Presets

Kreuzberg ships with named embedding presets that bundle a model, chunk size, and overlap into a single selection. Use `list_embedding_presets()` to see what's available and `get_embedding_preset()` to inspect details.

### List_embedding_presets()

```python title="Python"
def list_embedding_presets() -> list[str]
```

### Get_embedding_preset()

```python title="Python"
def get_embedding_preset(name: str) -> EmbeddingPreset | None
```

Returns `None` if the name doesn't match a known preset.

### EmbeddingPreset

Describes a preset's model and recommended chunking parameters:

| Field | Type | Example |
|-------|------|---------|
| `name` | `str` | `"balanced"` |
| `model_name` | `str` | ONNX model identifier |
| `dimensions` | `int` | Embedding vector size |
| `chunk_size` | `int` | Recommended chunk size in characters |
| `overlap` | `int` | Recommended overlap between chunks |
| `description` | `str` | What this preset optimizes for |

```python title="preset_info.py"
from kreuzberg import get_embedding_preset, list_embedding_presets

for name in list_embedding_presets():
    preset = get_embedding_preset(name)
    print(f"{preset.name}: {preset.dimensions}d, chunk={preset.chunk_size}")
```

---

## Types and Enums

### OutputFormat

Controls the text format of extraction results. Pass to `ExtractionConfig.output_format`:

```python title="Python"
from kreuzberg import OutputFormat

OutputFormat.PLAIN       # "plain" — raw text
OutputFormat.MARKDOWN    # "markdown" — Markdown with headings, lists, tables
OutputFormat.DJOT        # "djot" — Djot markup
OutputFormat.HTML        # "html" — HTML
OutputFormat.STRUCTURED  # "structured" — element-based structured output
```

```python title="output_format.py"
from kreuzberg import ExtractionConfig, OutputFormat, extract_file_sync

config = ExtractionConfig(output_format=OutputFormat.MARKDOWN)
result = extract_file_sync("document.pdf", config=config)
```

### ResultFormat

Controls the shape of the result — a single unified string, or a list of structural elements:

```python title="Python"
from kreuzberg import ResultFormat

ResultFormat.UNIFIED        # "unified" — one content string
ResultFormat.ELEMENT_BASED  # "element_based" — list of typed elements
```

---

## PDF Rendering

!!! Info "Added in v4.6.2"

### Render_pdf_page()

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

### PdfPageIterator

For rendering every page of a PDF without loading them all into memory at once. Yields `(page_index, png_bytes)` tuples — zero-based index paired with the PNG-encoded image bytes.

```python title="Python"
# Constructor signature:
# PdfPageIterator(path: str, dpi: int | None = None)
```

Works as a context manager, supports `len()`, and has a `page_count` property:

```python title="iterate_pdf_pages.py"
from kreuzberg import PdfPageIterator

with PdfPageIterator("document.pdf", dpi=200) as pages:
    print(f"Total pages: {pages.page_count}")

    for page_index, png_bytes in pages:
        with open(f"page_{page_index}.png", "wb") as f:
            f.write(png_bytes)
        print(f"Page {page_index}: {len(png_bytes)} bytes")
```

```python title="quick_page_count.py"
from kreuzberg import PdfPageIterator

pages = PdfPageIterator("document.pdf")
print(f"Document has {len(pages)} pages")
pages.close()
```

---

## Embeddings

### Embed_sync()

Generate embeddings for a list of texts synchronously.

**Signature:**

```python
def embed_sync(
    texts: list[str],
    config: EmbeddingConfig = EmbeddingConfig(),
) -> list[list[float]]
```

**Parameters:**

- `texts` (list[str]): List of strings to embed.
- `config` (EmbeddingConfig): Embedding configuration. Defaults to the "balanced" preset.

**Returns:** `list[list[float]]` — one embedding vector per input text.

**Raises:** `MissingDependencyError` if the `embeddings` feature is not enabled.

**Example:**

--8<-- "snippets/python/utils/standalone_embed.md"

---

### Embed()

Async variant of `embed_sync()`.

**Signature:**

```python
async def embed(
    texts: list[str],
    config: EmbeddingConfig = EmbeddingConfig(),
) -> list[list[float]]
```

Same parameters and return type as `embed_sync()`.

---

## Utilities

- **`detect_mime_type(data: bytes | bytearray)`** → str: Detect MIME type from file bytes (for example for `extract_bytes_sync`).
- **`detect_mime_type_from_path(path: str | Path)`** → str: Detect MIME type from file path (reads file).
- **`get_extensions_for_mime(mime_type: str)`** → list[str]: Return file extensions associated with a MIME type.

---

## LLM Integration

Kreuzberg integrates with LLMs via the `liter-llm` crate for structured extraction and VLM-based OCR. See the [LLM Integration Guide](../guides/llm-integration.md) for full details.

### Structured Extraction

Use `StructuredExtractionConfig` to extract structured data from documents using an LLM:

--8<-- "snippets/python/llm/structured_extraction.md"

The `structured_output` field on `ExtractionResult` contains the JSON string conforming to the provided schema:

```python title="access_structured_output.py"
result = await extract_file("paper.pdf", config=config)

if result.structured_output:
    import json
    data = json.loads(result.structured_output)
    print(data["title"])
```

### VLM OCR

Use a vision-language model as an OCR backend by setting `backend="vlm"` with a `vlm_config`:

--8<-- "snippets/python/llm/vlm_ocr.md"

### LLM Embeddings

Generate embeddings using an LLM provider instead of local ONNX models:

```python title="llm_embeddings.py"
from kreuzberg import EmbeddingConfig

config = EmbeddingConfig(
    model_type="llm",
    llm=LlmConfig(model="openai/text-embedding-3-small"),
)
vectors = embed_sync(["hello world"], config=config)
```

For configuration details including API keys, model selection, and provider setup, see the [LLM Integration Guide](../guides/llm-integration.md).

---

## Code Intelligence

Kreuzberg uses [tree-sitter-language-pack](https://docs.tree-sitter-language-pack.kreuzberg.dev) to parse and analyze source code files across 248 programming languages. When extracting code files, the result metadata includes structural analysis, imports, exports, symbols, diagnostics, and semantic code chunks.

Code intelligence data is available in `result.metadata["format"]` when `format_type` is `"code"`.

```python title="code_intelligence.py"
import kreuzberg

config = kreuzberg.ExtractionConfig(
    tree_sitter={
        "process": {
            "structure": True,
            "imports": True,
            "exports": True,
            "comments": True,
            "docstrings": True,
        }
    }
)

result = kreuzberg.extract_file_sync("app.py", config=config)

# Access code intelligence from format metadata
fmt = result.metadata.get("format")
if fmt and fmt.get("format_type") == "code":
    print(f"Language: {fmt['language']}")
    print(f"Functions/classes: {len(fmt['structure'])}")
    print(f"Imports: {len(fmt['imports'])}")

    for item in fmt["structure"]:
        print(f"  {item['kind']}: {item.get('name')} at line {item['span']['start_line']}")

    for chunk in fmt.get("chunks", []):
        print(f"Chunk: {chunk['content'][:50]}...")
```

For configuration details, see the [Code Intelligence Guide](../guides/code-intelligence.md).

---

## Version Information

```python title="basic_extraction.py"
import kreuzberg

print(kreuzberg.__version__)
```

# Kreuzberg Python API Reference

Comprehensive documentation for the Kreuzberg Python API. All extraction logic and heavy lifting is implemented in high-performance Rust, with Python adding OCR backends (EasyOCR, PaddleOCR) and custom post-processor support.

## Extraction Functions

### Synchronous File Extraction

```python
def extract_file_sync(
    file_path: str | Path,
    mime_type: str | None = None,
    config: ExtractionConfig | None = None,
    *,
    easyocr_kwargs: dict[str, Any] | None = None,
    paddleocr_kwargs: dict[str, Any] | None = None,
) -> ExtractionResult
```

Extract content from a file (synchronous).

**Parameters:**
- `file_path` (str | Path): Path to the file
- `mime_type` (str | None): Optional MIME type hint (auto-detected if None)
- `config` (ExtractionConfig | None): Extraction configuration (uses defaults if None)
- `easyocr_kwargs` (dict | None): EasyOCR initialization options (languages, use_gpu, beam_width, etc.)
- `paddleocr_kwargs` (dict | None): PaddleOCR initialization options (lang, use_angle_cls, show_log, etc.)

**Returns:** ExtractionResult with content, metadata, and tables

**Example:**

```python
from kreuzberg import extract_file_sync, ExtractionConfig, OcrConfig, TesseractConfig

# Basic usage
result = extract_file_sync("document.pdf")

# With Tesseract configuration
config = ExtractionConfig(
    ocr=OcrConfig(
        backend="tesseract",
        language="eng",
        tesseract_config=TesseractConfig(psm=6, enable_table_detection=True),
    )
)
result = extract_file_sync("invoice.pdf", config=config)

# With EasyOCR custom options
config = ExtractionConfig(ocr=OcrConfig(backend="easyocr", language="eng"))
result = extract_file_sync("scanned.pdf", config=config, easyocr_kwargs={"use_gpu": True})
```

### Asynchronous File Extraction

```python
async def extract_file(
    file_path: str | Path,
    mime_type: str | None = None,
    config: ExtractionConfig | None = None,
    *,
    easyocr_kwargs: dict[str, Any] | None = None,
    paddleocr_kwargs: dict[str, Any] | None = None,
) -> ExtractionResult
```

Extract content from a file (asynchronous). Same parameters and behavior as `extract_file_sync`.

### Synchronous Bytes Extraction

```python
def extract_bytes_sync(
    data: bytes | bytearray,
    mime_type: str,
    config: ExtractionConfig | None = None,
    *,
    easyocr_kwargs: dict[str, Any] | None = None,
    paddleocr_kwargs: dict[str, Any] | None = None,
) -> ExtractionResult
```

Extract content from bytes (synchronous).

**Parameters:**
- `data` (bytes | bytearray): File content as bytes or bytearray
- `mime_type` (str): MIME type of the data (required for format detection)
- `config` (ExtractionConfig | None): Extraction configuration
- `easyocr_kwargs` (dict | None): EasyOCR initialization options
- `paddleocr_kwargs` (dict | None): PaddleOCR initialization options

**Returns:** ExtractionResult with content, metadata, and tables

### Asynchronous Bytes Extraction

```python
async def extract_bytes(
    data: bytes | bytearray,
    mime_type: str,
    config: ExtractionConfig | None = None,
    *,
    easyocr_kwargs: dict[str, Any] | None = None,
    paddleocr_kwargs: dict[str, Any] | None = None,
) -> ExtractionResult
```

Extract content from bytes (asynchronous). Same parameters and behavior as `extract_bytes_sync`.

### Batch File Extraction

```python
async def batch_extract_files(
    paths: list[str | Path],
    config: ExtractionConfig | None = None,
    *,
    easyocr_kwargs: dict[str, Any] | None = None,
    paddleocr_kwargs: dict[str, Any] | None = None,
) -> list[ExtractionResult]
```

Extract content from multiple files in parallel (asynchronous).

**Parameters:**
- `paths` (list[str | Path]): List of file paths
- `config` (ExtractionConfig | None): Extraction configuration
- `easyocr_kwargs` (dict | None): EasyOCR initialization options
- `paddleocr_kwargs` (dict | None): PaddleOCR initialization options

**Returns:** List of ExtractionResults (one per file)

### Batch File Extraction (Synchronous)

```python
def batch_extract_files_sync(
    paths: list[str | Path],
    config: ExtractionConfig | None = None,
    *,
    easyocr_kwargs: dict[str, Any] | None = None,
    paddleocr_kwargs: dict[str, Any] | None = None,
) -> list[ExtractionResult]
```

Extract content from multiple files in parallel (synchronous).

### Batch Bytes Extraction

```python
async def batch_extract_bytes(
    data_list: list[bytes | bytearray],
    mime_types: list[str],
    config: ExtractionConfig | None = None,
    *,
    easyocr_kwargs: dict[str, Any] | None = None,
    paddleocr_kwargs: dict[str, Any] | None = None,
) -> list[ExtractionResult]
```

Extract content from multiple byte arrays in parallel (asynchronous).

**Parameters:**
- `data_list` (list[bytes | bytearray]): List of file contents as bytes/bytearray
- `mime_types` (list[str]): List of MIME types (one per data item)
- `config` (ExtractionConfig | None): Extraction configuration
- `easyocr_kwargs` (dict | None): EasyOCR initialization options
- `paddleocr_kwargs` (dict | None): PaddleOCR initialization options

**Returns:** List of ExtractionResults (one per data item)

### Batch Bytes Extraction (Synchronous)

```python
def batch_extract_bytes_sync(
    data_list: list[bytes | bytearray],
    mime_types: list[str],
    config: ExtractionConfig | None = None,
    *,
    easyocr_kwargs: dict[str, Any] | None = None,
    paddleocr_kwargs: dict[str, Any] | None = None,
) -> list[ExtractionResult]
```

Extract content from multiple byte arrays in parallel (synchronous).

## Configuration Classes

### ExtractionConfig

Main extraction configuration for document processing. All attributes are optional and use sensible defaults when not specified.

**Attributes:**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `use_cache` | bool | True | Enable caching of extraction results to improve performance on repeated extractions |
| `enable_quality_processing` | bool | True | Enable quality post-processing to clean and normalize extracted text |
| `ocr` | OcrConfig \| None | None | OCR configuration for extracting text from images. None = OCR disabled |
| `force_ocr` | bool | False | Force OCR processing even for searchable PDFs that contain extractable text |
| `chunking` | ChunkingConfig \| None | None | Text chunking configuration for dividing content into manageable chunks. None = disabled |
| `images` | ImageExtractionConfig \| None | None | Image extraction configuration for extracting images FROM documents. None = no extraction |
| `pdf_options` | PdfConfig \| None | None | PDF-specific options like password handling and metadata extraction |
| `token_reduction` | TokenReductionConfig \| None | None | Token reduction configuration for reducing token count in extracted content |
| `language_detection` | LanguageDetectionConfig \| None | None | Language detection configuration for identifying document language(s) |
| `keywords` | KeywordConfig \| None | None | Keyword extraction configuration for identifying important terms and phrases |
| `postprocessor` | PostProcessorConfig \| None | None | Post-processor configuration for custom text processing |
| `max_concurrent_extractions` | int \| None | num_cpus * 2 | Maximum concurrent extractions in batch operations |
| `html_options` | HtmlConversionOptions \| None | None | HTML conversion options for converting documents to markdown |
| `pages` | PageConfig \| None | None | Page extraction configuration for tracking page boundaries |
| `security_limits` | dict[str, int] \| None | None | Security limits configuration |
| `result_format` | str | "unified" | Result format: "unified" or "element_based" |
| `output_format` | str | "plain" | Output content format: "plain", "markdown", "djot", or "html" |

**Example:**

```python
from kreuzberg import ExtractionConfig, ChunkingConfig, OcrConfig

# Basic extraction with defaults
config = ExtractionConfig()

# Enable chunking with 512-char chunks and 100-char overlap
config = ExtractionConfig(chunking=ChunkingConfig(max_chars=512, max_overlap=100))

# Enable OCR with Tesseract
config = ExtractionConfig(ocr=OcrConfig(backend="tesseract", language="eng"))

# Multiple options
config = ExtractionConfig(
    use_cache=True,
    enable_quality_processing=True,
    output_format="markdown",
    result_format="unified"
)
```

### OcrConfig

OCR configuration for extracting text from images.

**Attributes:**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `backend` | str | "tesseract" | OCR backend: "tesseract", "easyocr", or "paddleocr" |
| `language` | str | "eng" | Language code (ISO 639-3 three-letter: "eng", "deu", "fra" or ISO 639-1 two-letter: "en", "de", "fr") |
| `tesseract_config` | TesseractConfig \| None | None | Tesseract-specific configuration (only used when backend="tesseract") |

**Example:**

```python
from kreuzberg import OcrConfig

# Tesseract with German language
config = OcrConfig(backend="tesseract", language="deu")

# EasyOCR for faster recognition
config = OcrConfig(backend="easyocr", language="eng")

# PaddleOCR for production deployments
config = OcrConfig(backend="paddleocr", language="chi_sim")
```

### TesseractConfig

Detailed Tesseract OCR configuration for advanced tuning. Fine-tune Tesseract OCR behavior for specific document types and quality levels.

**Attributes:**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `language` | str | "eng" | OCR language (ISO 639-3 three-letter code) |
| `psm` | int | 3 | Page Segmentation Mode: 0 (detection only), 3 (auto), 6 (uniform block), 11 (sparse text) |
| `output_format` | str | "markdown" | Output format for OCR results |
| `oem` | int | 3 | OCR Engine Mode: 0 (legacy), 1 (LSTM), 2 (both), 3 (auto) |
| `min_confidence` | float | 0.0 | Minimum confidence threshold (0.0-1.0) for accepting OCR results |
| `preprocessing` | ImagePreprocessingConfig \| None | None | Image preprocessing configuration before OCR |
| `enable_table_detection` | bool | True | Enable automatic table detection and extraction |
| `table_min_confidence` | float | 0.0 | Minimum confidence for table detection (0.0-1.0) |
| `table_column_threshold` | int | 50 | Minimum pixel width between columns |
| `table_row_threshold_ratio` | float | 0.5 | Minimum row height ratio |
| `use_cache` | bool | True | Cache OCR results for improved performance |
| `classify_use_pre_adapted_templates` | bool | True | Use pre-adapted character templates |
| `language_model_ngram_on` | bool | False | Enable language model n-gram processing |
| `tessedit_dont_blkrej_good_wds` | bool | True | Don't block-reject good words |
| `tessedit_dont_rowrej_good_wds` | bool | True | Don't row-reject good words |
| `tessedit_enable_dict_correction` | bool | True | Enable dictionary-based spelling correction |
| `tessedit_char_whitelist` | str | "" | Whitelist of characters to recognize (empty = all) |
| `tessedit_char_blacklist` | str | "" | Blacklist of characters to ignore |
| `tessedit_use_primary_params_model` | bool | True | Use primary parameters model |
| `textord_space_size_is_variable` | bool | True | Allow variable space sizes |
| `thresholding_method` | bool | False | Thresholding method for binarization |

**Example:**

```python
from kreuzberg import TesseractConfig, ImagePreprocessingConfig

# General document OCR
config = TesseractConfig(psm=3, oem=3)

# Invoice/form OCR with table detection
config = TesseractConfig(psm=6, oem=2, enable_table_detection=True, min_confidence=0.6)

# High-precision technical document OCR
config = TesseractConfig(
    psm=3,
    oem=2,
    preprocessing=ImagePreprocessingConfig(denoise=True, contrast_enhance=True, auto_rotate=True),
    min_confidence=0.7,
    tessedit_enable_dict_correction=True,
)

# Numeric-only OCR (for receipts, barcodes)
config = TesseractConfig(psm=6, tessedit_char_whitelist="0123456789.-,", min_confidence=0.8)

# Multiple language document
config = TesseractConfig(language="eng+fra+deu", psm=3, oem=2)
```

### ChunkingConfig

Text chunking configuration for dividing content into chunks. Chunking is useful for preparing content for embedding, indexing, or processing with length-limited systems (like LLM context windows).

**Attributes:**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_chars` | int | 1000 | Maximum number of characters per chunk. Chunks larger than this will be split intelligently at sentence/paragraph boundaries |
| `max_overlap` | int | 200 | Overlap between consecutive chunks in characters. Creates context bridges between chunks for smoother processing |
| `embedding` | EmbeddingConfig \| None | None | Configuration for generating embeddings for each chunk using ONNX models. None = no embeddings |
| `preset` | str \| None | None | Use a preset chunking configuration (overrides individual settings if provided). Use list_embedding_presets() to see available presets |

**IMPORTANT:** The fields are `max_chars` and `max_overlap` (NOT `max_characters` or `overlap`).

**Example:**

```python
from kreuzberg import ExtractionConfig, ChunkingConfig, EmbeddingConfig, EmbeddingModelType

# Basic chunking with defaults
config = ExtractionConfig(chunking=ChunkingConfig())

# Custom chunk size with overlap
config = ExtractionConfig(chunking=ChunkingConfig(max_chars=512, max_overlap=100))

# Chunking with embeddings
config = ExtractionConfig(
    chunking=ChunkingConfig(
        max_chars=512,
        embedding=EmbeddingConfig(model=EmbeddingModelType.preset("balanced"))
    )
)

# Using preset configuration
config = ExtractionConfig(chunking=ChunkingConfig(preset="semantic"))
```

### PdfConfig

PDF-specific extraction configuration.

**Attributes:**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `extract_images` | bool | False | Extract images from PDF documents |
| `passwords` | list[str] \| None | None | List of passwords to try when opening encrypted PDFs. Try each password in order until one succeeds |
| `extract_metadata` | bool | True | Extract PDF metadata (title, author, creation date, etc.) |
| `hierarchy` | HierarchyConfig \| None | None | Document hierarchy detection configuration. None = no hierarchy detection |

**Example:**

```python
from kreuzberg import ExtractionConfig, PdfConfig, HierarchyConfig

# Basic PDF configuration
config = ExtractionConfig(pdf_options=PdfConfig())

# Extract metadata and images from PDF
config = ExtractionConfig(pdf_options=PdfConfig(extract_images=True, extract_metadata=True))

# Handle encrypted PDFs
config = ExtractionConfig(pdf_options=PdfConfig(passwords=["password123", "fallback_password"]))

# Enable hierarchy detection
config = ExtractionConfig(pdf_options=PdfConfig(hierarchy=HierarchyConfig(k_clusters=6)))
```

### ImageExtractionConfig

Configuration for extracting images FROM documents. This is NOT for preprocessing images before OCR.

**Attributes:**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `extract_images` | bool | True | Enable image extraction from documents |
| `target_dpi` | int | 300 | Target DPI for image normalization. Images are resampled to this DPI for consistency |
| `max_image_dimension` | int | 4096 | Maximum width or height for extracted images. Larger images are downscaled to fit |
| `auto_adjust_dpi` | bool | True | Automatically adjust DPI based on image content quality |
| `min_dpi` | int | 72 | Minimum DPI threshold. Images with lower DPI are upscaled |
| `max_dpi` | int | 600 | Maximum DPI threshold. Images with higher DPI are downscaled |

**Example:**

```python
from kreuzberg import ExtractionConfig, ImageExtractionConfig

# Basic image extraction
config = ExtractionConfig(images=ImageExtractionConfig())

# Extract images with custom DPI settings
config = ExtractionConfig(
    images=ImageExtractionConfig(target_dpi=150, max_image_dimension=2048, auto_adjust_dpi=False)
)
```

### EmbeddingConfig

Embedding generation configuration for text chunks. Configures embedding generation using ONNX models via fastembed-rs.

**Attributes:**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | EmbeddingModelType | Preset "balanced" | The embedding model to use (preset, fastembed, or custom) |
| `normalize` | bool | True | Whether to normalize embedding vectors to unit length (recommended for cosine similarity) |
| `batch_size` | int | 32 | Number of texts to process simultaneously. Higher values use more memory but may be faster |
| `show_download_progress` | bool | False | Display progress during embedding model download |
| `cache_dir` | str \| None | None | Custom directory for caching downloaded models (defaults to ~/.cache/kreuzberg/embeddings/) |

**Example:**

```python
from kreuzberg import EmbeddingConfig, EmbeddingModelType

# Basic preset embedding (recommended)
config = EmbeddingConfig()

# Specific preset with settings
config = EmbeddingConfig(
    model=EmbeddingModelType.preset("balanced"),
    normalize=True,
    batch_size=64
)

# Custom ONNX model
config = EmbeddingConfig(
    model=EmbeddingModelType.custom(model_id="sentence-transformers/all-MiniLM-L6-v2", dimensions=384)
)

# With custom cache directory
config = EmbeddingConfig(cache_dir="/path/to/model/cache")
```

### EmbeddingModelType

Embedding model type selector with multiple configurations.

**Static Methods:**

```python
@staticmethod
def preset(name: str) -> EmbeddingModelType
```
Use a preset configuration (recommended for most use cases). Available presets: balanced, compact, large.

```python
@staticmethod
def fastembed(model: str, dimensions: int) -> EmbeddingModelType
```
Use a specific fastembed model by name.

```python
@staticmethod
def custom(model_id: str, dimensions: int) -> EmbeddingModelType
```
Use a custom ONNX model from HuggingFace (e.g., sentence-transformers/*).

**Example:**

```python
from kreuzberg import EmbeddingModelType, list_embedding_presets

# Using the balanced preset (recommended for general use)
model = EmbeddingModelType.preset("balanced")

# Using a specific fast embedding model
model = EmbeddingModelType.fastembed(model="BAAI/bge-small-en-v1.5", dimensions=384)

# Using a custom HuggingFace model
model = EmbeddingModelType.custom(
    model_id="sentence-transformers/all-MiniLM-L6-v2",
    dimensions=384
)

# Listing available presets
presets = list_embedding_presets()
print(f"Available presets: {presets}")
```

### TokenReductionConfig

Configuration for reducing token count in extracted content. Reduces token count to lower costs when working with LLM APIs.

**Attributes:**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `mode` | str | "off" | Token reduction mode: "off", "light", "moderate", "aggressive", or "maximum" |
| `preserve_important_words` | bool | True | Preserve capitalized words, technical terms, and proper nouns even in aggressive reduction modes |

**Example:**

```python
from kreuzberg import ExtractionConfig, TokenReductionConfig

# Moderate token reduction
config = ExtractionConfig(
    token_reduction=TokenReductionConfig(mode="moderate", preserve_important_words=True)
)

# Maximum reduction for large batches
config = ExtractionConfig(
    token_reduction=TokenReductionConfig(mode="maximum", preserve_important_words=True)
)

# No reduction (default)
config = ExtractionConfig(
    token_reduction=TokenReductionConfig(mode="off")
)
```

### LanguageDetectionConfig

Configuration for detecting document language(s).

**Attributes:**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | True | Enable language detection for extracted content |
| `min_confidence` | float | 0.8 | Minimum confidence threshold (0.0-1.0) for language detection |
| `detect_multiple` | bool | False | Detect multiple languages in the document. When False, only the most confident language is returned |

**Example:**

```python
from kreuzberg import ExtractionConfig, LanguageDetectionConfig, extract_file_sync

# Basic language detection
config = ExtractionConfig(language_detection=LanguageDetectionConfig())

# Detect multiple languages with lower confidence threshold
config = ExtractionConfig(
    language_detection=LanguageDetectionConfig(detect_multiple=True, min_confidence=0.6)
)

# Access detected languages in result
result = extract_file_sync("multilingual.pdf", config=config)
print(f"Languages: {result.detected_languages}")
```

### KeywordConfig

Keyword extraction configuration.

**Attributes:**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `algorithm` | KeywordAlgorithm | - | Keyword extraction algorithm (KeywordAlgorithm.Yake or KeywordAlgorithm.Rake) |
| `max_keywords` | int | 10 | Maximum number of keywords to extract |
| `min_score` | float | 0.0 | Minimum score threshold |
| `ngram_range` | tuple[int, int] | (1, 3) | N-gram range for keyword extraction |
| `language` | str \| None | "en" | Optional language hint |
| `yake_params` | YakeParams \| None | None | YAKE-specific tuning parameters |
| `rake_params` | RakeParams \| None | None | RAKE-specific tuning parameters |

### PageConfig

Page extraction and tracking configuration.

**Attributes:**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `extract_pages` | bool | False | Enable page tracking and per-page extraction |
| `insert_page_markers` | bool | False | Insert page markers into content |
| `marker_format` | str | "\\n\\n<!-- PAGE {page_num} -->\\n\\n" | Marker template containing {page_num} |

**Example:**

```python
from kreuzberg import ExtractionConfig, PageConfig

config = ExtractionConfig(pages=PageConfig(extract_pages=True))
```

### PostProcessorConfig

Configuration for post-processors in the extraction pipeline.

**Attributes:**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | True | Enable post-processors in the extraction pipeline |
| `enabled_processors` | list[str] \| None | None | Whitelist of processor names to run. None = run all enabled |
| `disabled_processors` | list[str] \| None | None | Blacklist of processor names to skip. None = none disabled |

**Example:**

```python
from kreuzberg import ExtractionConfig, PostProcessorConfig

# Basic post-processing with defaults
config = ExtractionConfig(postprocessor=PostProcessorConfig())

# Enable only specific processors
config = ExtractionConfig(
    postprocessor=PostProcessorConfig(
        enabled=True,
        enabled_processors=["normalize_whitespace", "fix_encoding"]
    )
)

# Disable specific processors
config = ExtractionConfig(
    postprocessor=PostProcessorConfig(
        enabled=True,
        disabled_processors=["experimental_cleanup"]
    )
)

# Disable all post-processing
config = ExtractionConfig(postprocessor=PostProcessorConfig(enabled=False))
```

### ImagePreprocessingConfig

Configuration for preprocessing images before OCR. This is NOT for extracting images from documents.

**Attributes:**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `target_dpi` | int | 300 | Target DPI for image normalization before OCR |
| `auto_rotate` | bool | True | Automatically detect and correct image rotation |
| `deskew` | bool | True | Correct skewed images to improve OCR accuracy |
| `denoise` | bool | False | Apply denoising filters to reduce noise in images |
| `contrast_enhance` | bool | False | Enhance contrast to improve text readability |
| `binarization_method` | str | "otsu" | Method for converting images to black and white |
| `invert_colors` | bool | False | Invert colors (white text on black background) |

**Example:**

```python
from kreuzberg import TesseractConfig, ImagePreprocessingConfig

# Basic preprocessing for OCR
config = TesseractConfig(preprocessing=ImagePreprocessingConfig())

# Aggressive preprocessing for low-quality scans
config = TesseractConfig(
    preprocessing=ImagePreprocessingConfig(
        target_dpi=300,
        denoise=True,
        contrast_enhance=True,
        auto_rotate=True,
        deskew=True
    )
)
```

## ExtractionResult

Result object returned by extraction functions.

**Attributes:**

| Field | Type | Description |
|-------|------|-------------|
| `content` | str | Main extracted text content in the specified output_format |
| `mime_type` | str | MIME type of the processed document |
| `metadata` | Metadata | Extracted document metadata (title, author, created_at, format_type, etc.) |
| `tables` | list[ExtractedTable] | Extracted tables from the document |
| `detected_languages` | list[str] \| None | Detected language codes (e.g., ["en", "de"]) if language detection is enabled |
| `chunks` | list[Chunk] \| None | Text chunks if chunking is enabled (each chunk has content, embedding, metadata) |
| `images` | list[ExtractedImage] \| None | Extracted images if image extraction is enabled |
| `pages` | list[PageContent] \| None | Per-page content and metadata if page extraction is enabled |
| `elements` | list[Element] \| None | Semantic elements if result_format="element_based" |
| `output_format` | str \| None | Format of the content field (plain, markdown, djot, html) |
| `result_format` | str \| None | Result format used (unified or element_based) |

**Methods:**

```python
def get_page_count(self) -> int
```
Get the total number of pages in the document.

```python
def get_chunk_count(self) -> int
```
Get the total number of chunks if chunking is enabled.

```python
def get_detected_language(self) -> str | None
```
Get the most confident detected language code.

```python
def get_metadata_field(self, field_name: str) -> Any | None
```
Get a specific metadata field by name.

**Example:**

```python
from kreuzberg import extract_file_sync, ExtractionConfig, ChunkingConfig

config = ExtractionConfig(
    chunking=ChunkingConfig(max_chars=512),
    output_format="markdown"
)
result = extract_file_sync("document.pdf", config=config)

print(f"Content preview: {result.content[:200]}")
print(f"MIME type: {result.mime_type}")
print(f"Page count: {result.get_page_count()}")
print(f"Chunk count: {result.get_chunk_count()}")
print(f"Detected language: {result.get_detected_language()}")

if result.tables:
    print(f"Found {len(result.tables)} tables")

if result.chunks:
    first_chunk = result.chunks[0]
    print(f"First chunk: {first_chunk.content[:100]}")
    if first_chunk.embedding:
        print(f"Embedding dimensions: {len(first_chunk.embedding)}")
```

## Error Classes

All exceptions inherit from `KreuzbergError`, the base exception class.

### KreuzbergError

Base exception class for all Kreuzberg errors.

```python
class KreuzbergError(Exception):
    """Base exception for all Kreuzberg errors."""
```

### ParsingError

Raised when document parsing fails.

```python
class ParsingError(KreuzbergError):
    """Document parsing failed (corrupt, malformed, etc.)."""
```

### OCRError

Raised when OCR processing fails.

```python
class OCRError(KreuzbergError):
    """OCR operation failed."""
```

### ValidationError

Raised when validation fails.

```python
class ValidationError(KreuzbergError):
    """Validation failed (invalid parameters, constraints, format mismatches)."""
```

### MissingDependencyError

Raised when required dependencies are not available.

```python
class MissingDependencyError(KreuzbergError):
    """Required dependency not available (easyocr, paddleocr, tesseract, etc.)."""

    @staticmethod
    def create_for_package(dependency_group: str, functionality: str, package_name: str) -> MissingDependencyError
```

**Example:**

```python
from kreuzberg import extract_file_sync, MissingDependencyError, OCRError, ParsingError

try:
    result = extract_file_sync("document.pdf")
except ParsingError as e:
    print(f"Failed to parse document: {e}")
except OCRError as e:
    print(f"OCR failed: {e}")
except MissingDependencyError as e:
    print(f"Missing dependency: {e}")
```

## Utility Functions

### MIME Type Detection

```python
def detect_mime_type(data: bytes | bytearray) -> str
```

Detect MIME type from file bytes using magic number detection.

**Parameters:**
- `data` (bytes | bytearray): File content as bytes or bytearray

**Returns:** Detected MIME type (e.g., "application/pdf", "image/png")

```python
def detect_mime_type_from_path(path: str | Path) -> str
```

Detect MIME type from file path by reading the file and detecting its MIME type.

**Parameters:**
- `path` (str | Path): Path to the file

**Returns:** Detected MIME type

**Raises:**
- `OSError`: If file cannot be read (file not found, permission denied, etc.)
- `RuntimeError`: If MIME type detection fails

**Example:**

```python
from kreuzberg import detect_mime_type, detect_mime_type_from_path

# From bytes
pdf_bytes = b"%PDF-1.4\n"
mime_type = detect_mime_type(pdf_bytes)

# From path
mime_type = detect_mime_type_from_path("document.pdf")
```

### MIME Type Validation

```python
def validate_mime_type(mime_type: str) -> str
```

Validate a MIME type string and return the canonical form.

```python
def get_extensions_for_mime(mime_type: str) -> list[str]
```

Get file extensions associated with a MIME type.

**Example:**

```python
from kreuzberg import validate_mime_type, get_extensions_for_mime

canonical = validate_mime_type("application/pdf")
extensions = get_extensions_for_mime("application/pdf")  # Returns ["pdf"]
```

### Configuration Loading

```python
def load_extraction_config_from_file(path: str | Path) -> ExtractionConfig
```

Load extraction configuration from a specific file.

**Parameters:**
- `path` (str | Path): Path to the configuration file (.toml, .yaml, or .json)

**Returns:** ExtractionConfig parsed from the file

**Raises:**
- `FileNotFoundError`: If the configuration file does not exist
- `RuntimeError`: If the file cannot be read or parsed
- `ValueError`: If the file format is invalid or unsupported

```python
def discover_extraction_config() -> ExtractionConfig | None
```

Discover extraction configuration from the environment (deprecated).

Attempts to locate a Kreuzberg configuration file using:
1. KREUZBERG_CONFIG_PATH environment variable
2. Search for kreuzberg.toml, kreuzberg.yaml, or kreuzberg.json in current and parent directories

**Returns:** ExtractionConfig if found, None otherwise

**Note:** Deprecated in favor of `load_extraction_config_from_file` for more predictable behavior.

**Example:**

```python
from kreuzberg import load_extraction_config_from_file, extract_file_sync

# Load from specific file
config = load_extraction_config_from_file("kreuzberg.toml")
result = extract_file_sync("document.pdf", config=config)

# Auto-discover configuration
import os
os.environ["KREUZBERG_CONFIG_PATH"] = "config/kreuzberg.yaml"
# Then extraction will use the discovered config
```

## Plugin System

### Registering Post-Processors

```python
def register_post_processor(processor: Any) -> None
```

Register a Python PostProcessor with the Rust core. Once registered, the processor will be called automatically after extraction to enrich results.

**Required Methods:**
- `name() -> str`: Return processor name (must be non-empty)
- `process(result: ExtractionResult) -> ExtractionResult`: Process and enrich the extraction result
- `processing_stage() -> str`: Return "early", "middle", or "late"

**Optional Methods:**
- `initialize() -> None`: Called when processor is registered
- `shutdown() -> None`: Called when processor is unregistered

**Example:**

```python
from kreuzberg import register_post_processor, ExtractionResult

class EntityExtractor:
    def name(self) -> str:
        return "entity_extraction"

    def processing_stage(self) -> str:
        return "early"

    def process(self, result: ExtractionResult) -> ExtractionResult:
        entities = {"PERSON": ["John Doe"], "ORG": ["Microsoft"]}
        result.metadata["entities"] = entities
        return result

register_post_processor(EntityExtractor())
```

### Registering OCR Backends

```python
def register_ocr_backend(backend: Any) -> None
```

Register a Python OCR backend with the Rust core.

**Required Methods:**
- `name() -> str`: Return backend name (must be non-empty)
- `supported_languages() -> list[str]`: Return list of supported language codes
- `process_image(image_bytes: bytes, language: str) -> OcrResult`: Process image and return OCR result
- `process_file(path: str, language: str) -> OcrResult`: Process file and return OCR result
- `initialize() -> None`: Called when backend is registered
- `shutdown() -> None`: Called when backend is unregistered
- `version() -> str`: Return backend version string

**Example:**

```python
from kreuzberg import register_ocr_backend

class MyOcrBackend:
    def name(self) -> str:
        return "my-ocr"

    def supported_languages(self) -> list[str]:
        return ["eng", "deu", "fra"]

    def process_image(self, image_bytes: bytes, language: str) -> dict:
        return {
            "content": "extracted text",
            "metadata": {"confidence": 0.95},
            "tables": []
        }

register_ocr_backend(MyOcrBackend())
```

### Registering Validators

```python
def register_validator(validator: Any) -> None
```

Register a Python Validator with the Rust core. Validators are called automatically after extraction to validate results.

**Required Methods:**
- `name() -> str`: Return validator name (must be non-empty)
- `validate(result: ExtractionResult) -> None`: Validate the extraction result (raise error to fail)

**Optional Methods:**
- `should_validate(result: ExtractionResult) -> bool`: Check if validator should run (defaults to True)
- `priority() -> int`: Return priority (defaults to 50, higher runs first)

**Example:**

```python
from kreuzberg import register_validator, ValidationError, ExtractionResult

class MinLengthValidator:
    def name(self) -> str:
        return "min_length_validator"

    def priority(self) -> int:
        return 100

    def validate(self, result: ExtractionResult) -> None:
        if len(result.content) < 100:
            raise ValidationError("Content too short")

register_validator(MinLengthValidator())
```

### Plugin Management Functions

```python
def list_post_processors() -> list[str]
```
List names of all registered post-processors.

```python
def list_validators() -> list[str]
```
List names of all registered validators.

```python
def list_ocr_backends() -> list[str]
```
List names of all available OCR backends.

```python
def unregister_post_processor(name: str) -> None
```
Unregister a post-processor by name.

```python
def unregister_validator(name: str) -> None
```
Unregister a validator by name.

```python
def unregister_ocr_backend(name: str) -> None
```
Unregister an OCR backend by name.

```python
def clear_post_processors() -> None
```
Clear all registered post-processors.

```python
def clear_validators() -> None
```
Clear all registered validators.

```python
def clear_ocr_backends() -> None
```
Clear all registered OCR backends.

## Format Enums

### OutputFormat

Output format for extraction results.

```python
class OutputFormat(str, Enum):
    PLAIN = "plain"         # Plain text format
    MARKDOWN = "markdown"   # Markdown format
    DJOT = "djot"          # Djot lightweight markup format
    HTML = "html"          # HTML format
```

### ResultFormat

Result format controlling extraction output structure.

```python
class ResultFormat(str, Enum):
    UNIFIED = "unified"                # All content in `content` field
    ELEMENT_BASED = "element_based"   # Unstructured-compatible output with semantic elements
```

## Error Handling

### Error Code Functions

```python
def get_last_error_code() -> int
```

Get the last error code from the FFI layer.

**Returns:**
- 0 (SUCCESS): No error occurred
- 1 (GENERIC_ERROR): Generic unspecified error
- 2 (PANIC): A panic occurred in the Rust core
- 3 (INVALID_ARGUMENT): Invalid argument provided
- 4 (IO_ERROR): I/O operation failed
- 5 (PARSING_ERROR): Document parsing failed
- 6 (OCR_ERROR): OCR operation failed
- 7 (MISSING_DEPENDENCY): Required dependency not available

```python
def get_error_details() -> dict[str, Any]
```

Get detailed error information from the FFI layer.

**Returns:** dict with keys:
- `message` (str): Human-readable error message
- `error_code` (int): Numeric error code (0-7)
- `error_type` (str): Error type name (e.g., "validation", "ocr")
- `source_file` (str | None): Source file path if available
- `source_function` (str | None): Function name if available
- `source_line` (int): Line number (0 if unknown)
- `context_info` (str | None): Additional context if available
- `is_panic` (bool): Whether error came from a panic

```python
def classify_error(message: str) -> int
```

Classify an error message into a Kreuzberg error code.

**Parameters:**
- `message` (str): The error message to classify

**Returns:** int error code (0-7) representing the classification

```python
def error_code_name(code: int) -> str
```

Get the human-readable name of an error code.

**Parameters:**
- `code` (int): Numeric error code (0-7)

**Returns:** Human-readable error code name (e.g., "validation", "ocr")

**Example:**

```python
from kreuzberg import get_error_details, get_last_error_code, error_code_name, classify_error

try:
    result = extract_file_sync("document.pdf")
except Exception as e:
    code = get_last_error_code()
    if code:
        print(f"Error code: {code} ({error_code_name(code)})")

    details = get_error_details()
    print(f"Error: {details['message']}")
    print(f"Type: {details['error_type']}")

    classified = classify_error(str(e))
    print(f"Classified as: {error_code_name(classified)}")
```

## Validation Functions

### Parameter Validation

```python
def validate_chunking_params(max_chars: int, max_overlap: int) -> bool
```
Validate chunking parameters.

```python
def validate_confidence(confidence: float) -> bool
```
Validate confidence value (0.0-1.0).

```python
def validate_dpi(dpi: int) -> bool
```
Validate DPI value.

```python
def validate_tesseract_psm(psm: int) -> bool
```
Validate Tesseract Page Segmentation Mode.

```python
def validate_tesseract_oem(oem: int) -> bool
```
Validate Tesseract OCR Engine Mode.

```python
def validate_ocr_backend(backend: str) -> bool
```
Validate OCR backend name.

```python
def validate_language_code(code: str) -> bool
```
Validate language code format.

```python
def validate_token_reduction_level(level: str) -> bool
```
Validate token reduction level.

```python
def validate_output_format(output_format: str) -> bool
```
Validate output format string.

```python
def validate_binarization_method(method: str) -> bool
```
Validate binarization method for image preprocessing.

### Getting Valid Values

```python
def get_valid_binarization_methods() -> list[str]
```
Get list of valid binarization methods.

```python
def get_valid_language_codes() -> list[str]
```
Get list of valid language codes.

```python
def get_valid_ocr_backends() -> list[str]
```
Get list of valid OCR backend names.

```python
def get_valid_token_reduction_levels() -> list[str]
```
Get list of valid token reduction levels.

```python
def list_embedding_presets() -> list[str]
```
List available embedding presets.

```python
def get_embedding_preset(name: str) -> EmbeddingPreset | None
```
Get details about a specific embedding preset.

**Example:**

```python
from kreuzberg import (
    validate_dpi,
    get_valid_binarization_methods,
    list_embedding_presets,
    get_embedding_preset
)

# Validate parameters
if not validate_dpi(300):
    print("Invalid DPI")

# List valid values
binarization_methods = get_valid_binarization_methods()
presets = list_embedding_presets()

# Get preset details
preset = get_embedding_preset("balanced")
if preset:
    print(f"Balanced preset: {preset.description}")
    print(f"Dimensions: {preset.dimensions}")
    print(f"Recommended chunk size: {preset.chunk_size}")
```

## Configuration Utilities

### Config Manipulation

```python
def config_to_json(config: ExtractionConfig) -> str
```
Convert ExtractionConfig to JSON string.

```python
def config_get_field(config: ExtractionConfig, field_name: str) -> Any | None
```
Get a specific field value from ExtractionConfig.

```python
def config_merge(base: ExtractionConfig, override: ExtractionConfig) -> None
```
Merge override config into base config (mutates base).

**Example:**

```python
from kreuzberg import ExtractionConfig, config_to_json, config_get_field, config_merge

config = ExtractionConfig(use_cache=True, enable_quality_processing=False)

# Convert to JSON
json_str = config_to_json(config)
print(json_str)

# Get field
use_cache = config_get_field(config, "use_cache")
print(f"use_cache: {use_cache}")

# Merge configs
override = ExtractionConfig(use_cache=False)
config_merge(config, override)
```

## Version Information

```python
__version__: str
```

Current version of the kreuzberg package.

**Example:**

```python
from kreuzberg import __version__

print(f"Kreuzberg version: {__version__}")
```

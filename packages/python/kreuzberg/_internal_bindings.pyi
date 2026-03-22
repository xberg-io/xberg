from collections.abc import Awaitable
from enum import StrEnum
from pathlib import Path
from typing import Any, Literal, Protocol, TypeAlias, TypedDict, overload

class OutputFormat(StrEnum):
    PLAIN = "plain"
    MARKDOWN = "markdown"
    DJOT = "djot"
    HTML = "html"
    STRUCTURED = "structured"

class ResultFormat(StrEnum):
    UNIFIED = "unified"
    ELEMENT_BASED = "element_based"

__all__ = [
    "AccelerationConfig",
    "AnnotationType",
    "Attributes",
    "BoundingBox",
    "Chunk",
    "ChunkMetadata",
    "ChunkingConfig",
    "ConcurrencyConfig",
    "ContentLayer",
    "DjotContent",
    "DjotImage",
    "DjotLink",
    "DjotTable",
    "DocumentNode",
    "DocumentStructure",
    "Element",
    "ElementMetadata",
    "ElementType",
    "EmailConfig",
    "EmbeddingConfig",
    "EmbeddingModelType",
    "EmbeddingPreset",
    "ErrorDetails",
    "ErrorMetadata",
    "ExtractedImage",
    "ExtractedKeyword",
    "ExtractedTable",
    "ExtractionConfig",
    "ExtractionResult",
    "FileExtractionConfig",
    "Footnote",
    "FormattedBlock",
    "GridCell",
    "HeaderMetadata",
    "HierarchyConfig",
    "HtmlConversionOptions",
    "HtmlImageMetadata",
    "HtmlMetadata",
    "HtmlPreprocessingOptions",
    "ImageExtractionConfig",
    "ImagePreprocessingConfig",
    "ImagePreprocessingMetadata",
    "InlineElement",
    "KeywordAlgorithm",
    "KeywordConfig",
    "LanguageDetectionConfig",
    "LayoutDetectionConfig",
    "LinkMetadata",
    "Metadata",
    "MissingDependencyError",
    "NodeContent",
    "NodeContentType",
    "OCRError",
    "OcrBackendProtocol",
    "OcrBoundingGeometry",
    "OcrBoundingGeometryQuadrilateral",
    "OcrBoundingGeometryRectangle",
    "OcrConfidence",
    "OcrConfig",
    "OcrElement",
    "OcrElementConfig",
    "OcrElementLevel",
    "OcrResult",
    "OcrResultTable",
    "OcrRotation",
    "OutputFormat",
    "PaddleOcrConfig",
    "PageBoundary",
    "PageConfig",
    "PageContent",
    "PageInfo",
    "PageStructure",
    "ParsingError",
    "PdfAnnotation",
    "PdfAnnotationType",
    "PdfConfig",
    "PostProcessorConfig",
    "PostProcessorProtocol",
    "ProcessingWarning",
    "RakeParams",
    "ResultFormat",
    "StructuredData",
    "TableGrid",
    "TesseractConfig",
    "TextAnnotation",
    "TokenReductionConfig",
    "ValidationError",
    "ValidatorProtocol",
    "YakeParams",
    "_discover_extraction_config_impl",
    "_load_extraction_config_from_file_impl",
    "batch_extract_bytes",
    "batch_extract_bytes_sync",
    "batch_extract_files",
    "batch_extract_files_sync",
    "classify_error",
    "clear_document_extractors",
    "clear_ocr_backends",
    "clear_post_processors",
    "clear_validators",
    "config_get_field",
    "config_merge",
    "config_to_json",
    "detect_mime_type_from_bytes",
    "detect_mime_type_from_path",
    "error_code_name",
    "extract_bytes",
    "extract_bytes_sync",
    "extract_file",
    "extract_file_sync",
    "get_embedding_preset",
    "get_error_details",
    "get_extensions_for_mime",
    "get_last_error_code",
    "get_last_panic_context",
    "get_valid_binarization_methods",
    "get_valid_language_codes",
    "get_valid_ocr_backends",
    "get_valid_token_reduction_levels",
    "init_async_runtime",
    "list_document_extractors",
    "list_embedding_presets",
    "list_ocr_backends",
    "list_post_processors",
    "list_validators",
    "register_ocr_backend",
    "register_post_processor",
    "register_validator",
    "unregister_document_extractor",
    "unregister_ocr_backend",
    "unregister_post_processor",
    "unregister_validator",
    "validate_binarization_method",
    "validate_chunking_params",
    "validate_confidence",
    "validate_dpi",
    "validate_language_code",
    "validate_mime_type",
    "validate_ocr_backend",
    "validate_output_format",
    "validate_tesseract_oem",
    "validate_tesseract_psm",
    "validate_token_reduction_level",
]

class ValidationError(Exception): ...
class ParsingError(Exception): ...
class OCRError(Exception): ...
class MissingDependencyError(Exception): ...

def init_async_runtime() -> None: ...

class ErrorDetails(TypedDict):
    message: str
    error_code: int
    error_type: str
    source_file: str | None
    source_function: str | None
    source_line: int
    context_info: str | None
    is_panic: bool

class HtmlPreprocessingOptions(TypedDict, total=False):
    enabled: bool
    preset: str
    remove_navigation: bool
    remove_forms: bool

class HtmlConversionOptions(TypedDict, total=False):
    heading_style: str
    list_indent_type: str
    list_indent_width: int
    bullets: str
    strong_em_symbol: str
    escape_asterisks: bool
    escape_underscores: bool
    escape_misc: bool
    escape_ascii: bool
    code_language: str
    autolinks: bool
    default_title: bool
    br_in_tables: bool
    hocr_spatial_tables: bool
    highlight_style: str
    extract_metadata: bool
    whitespace_mode: str
    strip_newlines: bool
    wrap: bool
    wrap_width: int
    convert_as_inline: bool
    sub_symbol: str
    sup_symbol: str
    newline_style: str
    code_block_style: str
    keep_inline_images_in: str
    encoding: str
    debug: bool
    strip_tags: bool
    preserve_tags: bool
    preprocessing: HtmlPreprocessingOptions

class OcrResultTable(TypedDict):
    cells: list[list[str]]
    markdown: str
    page_number: int

class OcrResult(TypedDict, total=False):
    content: str
    metadata: dict[str, Any]
    tables: list[OcrResultTable]

class OcrBackendProtocol(Protocol):
    def name(self) -> str: ...
    def supported_languages(self) -> list[str]: ...
    def process_image(self, image_bytes: bytes, language: str) -> OcrResult: ...
    def process_file(self, path: str, language: str) -> OcrResult: ...
    def initialize(self) -> None: ...
    def shutdown(self) -> None: ...
    def version(self) -> str: ...

OcrElementLevel: TypeAlias = Literal["word", "line", "block", "page"]

class OcrBoundingGeometryRectangle(TypedDict):
    type: str
    left: float
    top: float
    width: float
    height: float

class OcrBoundingGeometryQuadrilateral(TypedDict):
    type: str
    points: list[list[float]]

OcrBoundingGeometry: TypeAlias = OcrBoundingGeometryRectangle | OcrBoundingGeometryQuadrilateral

class OcrConfidence(TypedDict, total=False):
    detection: float
    recognition: float

class OcrRotation(TypedDict, total=False):
    angle_degrees: float
    confidence: float

class OcrElement(TypedDict, total=False):
    text: str
    geometry: OcrBoundingGeometry | None
    confidence: OcrConfidence | None
    level: OcrElementLevel | None
    rotation: OcrRotation | None
    page_number: int | None
    parent_id: str | None
    backend_metadata: dict[str, Any] | None

class OcrElementConfig:
    include_elements: bool
    min_level: str | None
    min_confidence: float | None
    build_hierarchy: bool
    def __init__(
        self,
        *,
        include_elements: bool = ...,
        min_level: str | None = ...,
        min_confidence: float | None = ...,
        build_hierarchy: bool = ...,
    ) -> None: ...

class PaddleOcrConfig:
    language: str | None
    cache_dir: str | None
    use_angle_cls: bool | None
    enable_table_detection: bool | None
    det_db_thresh: float | None
    det_db_box_thresh: float | None
    det_db_unclip_ratio: float | None
    det_limit_side_len: int | None
    rec_batch_num: int | None
    padding: int | None
    model_tier: str | None
    def __init__(
        self,
        *,
        language: str | None = ...,
        cache_dir: str | None = ...,
        use_angle_cls: bool | None = ...,
        enable_table_detection: bool | None = ...,
        det_db_thresh: float | None = ...,
        det_db_box_thresh: float | None = ...,
        det_db_unclip_ratio: float | None = ...,
        det_limit_side_len: int | None = ...,
        rec_batch_num: int | None = ...,
        padding: int | None = ...,
        model_tier: str | None = ...,
    ) -> None: ...

class PostProcessorProtocol(Protocol):
    def name(self) -> str: ...
    def process(self, result: ExtractionResult) -> ExtractionResult: ...
    def processing_stage(self) -> Literal["early", "middle", "late"]: ...
    def initialize(self) -> None: ...
    def shutdown(self) -> None: ...

class ValidatorProtocol(Protocol):
    def name(self) -> str: ...
    def validate(self, result: ExtractionResult) -> None: ...
    def priority(self) -> int: ...
    def should_validate(self, result: ExtractionResult) -> bool: ...

class LayoutDetectionConfig:
    """Layout detection configuration for PDF extraction.

    Controls layout detection behavior using ONNX-based document layout models
    (YOLO or RT-DETR) to detect document structure elements like tables, figures,
    headers, and code blocks.

    Requires the ``layout-detection`` feature to be compiled.

    Attributes:
        preset (str): Model preset for layout detection. ``"fast"`` uses YOLO
            (DocLayNet, 11 classes), ``"accurate"`` uses RT-DETR (17 classes).
            Default: ``"fast"``

        confidence_threshold (float | None): Override the model's default
            confidence threshold for detections. None uses the model default.
            Default: None

        apply_heuristics (bool): Whether to apply postprocessing heuristics
            to improve detection quality (e.g., merging overlapping regions).
            Default: True

    Example:
        Enable fast layout detection:
            >>> from kreuzberg import LayoutDetectionConfig, ExtractionConfig
            >>> config = ExtractionConfig(layout=LayoutDetectionConfig())

        Use accurate model with custom threshold:
            >>> layout = LayoutDetectionConfig(preset="accurate", confidence_threshold=0.5)
            >>> config = ExtractionConfig(layout=layout)
    """

    preset: str
    confidence_threshold: float | None
    apply_heuristics: bool

    def __init__(
        self,
        *,
        preset: str | None = None,
        confidence_threshold: float | None = None,
        apply_heuristics: bool | None = None,
    ) -> None: ...

class AccelerationConfig:
    """Hardware acceleration configuration for ONNX Runtime.

    Controls which execution provider is used for ONNX model inference
    (e.g., for layout detection or embedding models).

    Attributes:
        provider (str): Execution provider. One of ``"auto"``, ``"cpu"``,
            ``"coreml"``, ``"cuda"``, ``"tensorrt"``. Default: ``"auto"``
        device_id (int): GPU device ID for CUDA/TensorRT providers. Default: 0

    Example:
        Auto-select provider:
            >>> from kreuzberg import AccelerationConfig
            >>> config = AccelerationConfig()

        Force CPU:
            >>> config = AccelerationConfig(provider="cpu")

        Use CUDA on device 1:
            >>> config = AccelerationConfig(provider="cuda", device_id=1)
    """

    provider: str
    device_id: int

    def __init__(
        self,
        *,
        provider: str | None = None,
        device_id: int | None = None,
    ) -> None: ...

class EmailConfig:
    """Email extraction configuration.

    Controls behavior specific to MSG email extraction.

    Attributes:
        msg_fallback_codepage (int | None): Windows codepage number to use when an
            MSG file contains no codepage property. Defaults to None, which falls
            back to windows-1252. Common values: 1250 (Central European),
            1251 (Cyrillic), 1252 (Western European), 1253 (Greek), 1254 (Turkish),
            1255 (Hebrew), 1256 (Arabic), 932 (Japanese), 936 (Simplified Chinese).
            Default: None

    Example:
        Use default (windows-1252 fallback):
            >>> from kreuzberg import EmailConfig
            >>> config = EmailConfig()

        Force Cyrillic codepage for Russian MSG files:
            >>> config = EmailConfig(msg_fallback_codepage=1251)
    """

    msg_fallback_codepage: int | None

    def __init__(
        self,
        *,
        msg_fallback_codepage: int | None = None,
    ) -> None: ...

class ConcurrencyConfig:
    """Concurrency configuration for controlling thread usage.

    Controls thread usage for constrained environments by capping all internal
    thread pools (Rayon, ONNX Runtime intra-op) and batch concurrency.

    Attributes:
        max_threads (int | None): Maximum number of threads for all internal
            thread pools. None = system defaults. Default: None

    Example:
        Limit to 2 threads:
            >>> from kreuzberg import ConcurrencyConfig
            >>> config = ConcurrencyConfig(max_threads=2)
    """

    max_threads: int | None

    def __init__(
        self,
        *,
        max_threads: int | None = None,
    ) -> None: ...

class ExtractionConfig:
    """Main extraction configuration for document processing.

    This class contains all configuration options for the extraction process.
    All attributes are optional and use sensible defaults when not specified.

    Attributes:
        use_cache (bool): Enable caching of extraction results to improve performance
            on repeated extractions. Default: True

        enable_quality_processing (bool): Enable quality post-processing to clean
            and normalize extracted text. Default: True

        ocr (OcrConfig | None): OCR configuration for extracting text from images
            and scanned documents. None = OCR disabled. Default: None

        force_ocr (bool): Force OCR processing even for searchable PDFs that contain
            extractable text. Useful for ensuring consistent formatting. Default: False

        chunking (ChunkingConfig | None): Text chunking configuration for dividing
            content into manageable chunks. None = chunking disabled. Default: None

        images (ImageExtractionConfig | None): Image extraction configuration for
            extracting images FROM documents (not for OCR preprocessing).
            None = no image extraction. Default: None

        pdf_options (PdfConfig | None): PDF-specific options like password handling
            and metadata extraction. None = use defaults. Default: None

        token_reduction (TokenReductionConfig | None): Token reduction configuration
            for reducing token count in extracted content (useful for LLM APIs).
            None = no token reduction. Default: None

        language_detection (LanguageDetectionConfig | None): Language detection
            configuration for identifying the language(s) in documents.
            None = no language detection. Default: None

        pages (PageConfig | None): Page extraction configuration for tracking and
            extracting page boundaries. None = no page tracking. Default: None

        keywords (KeywordConfig | None): Keyword extraction configuration for
            identifying important terms and phrases in content.
            None = no keyword extraction. Default: None

        postprocessor (PostProcessorConfig | None): Post-processor configuration
            for custom text processing. None = use defaults. Default: None

        max_concurrent_extractions (int | None): Maximum concurrent extractions
            in batch operations. None = num_cpus * 2. Default: None

        html_options (HtmlConversionOptions | None): HTML conversion options for
            converting documents to markdown. Default: None

        result_format (str): Result format for extraction output.
            Specifies whether results use unified format (all content in `content` field)
            or element-based format (with semantic elements for Unstructured-compatible output).
            Values: "unified" (default), "element_based". Default: "unified"

        output_format (str): Output content format.
            Controls the format of the extracted content.
            Values: "plain" (default), "markdown", "djot", "html". Default: "plain"

    Example:
        Basic extraction with defaults:
            >>> from kreuzberg import ExtractionConfig, extract_file_sync
            >>> config = ExtractionConfig()
            >>> result = extract_file_sync("document.pdf", config=config)

        Enable chunking with 512-char chunks and 100-char overlap:
            >>> from kreuzberg import ExtractionConfig, ChunkingConfig
            >>> config = ExtractionConfig(chunking=ChunkingConfig(max_chars=512, max_overlap=100))

        Enable OCR with Tesseract for non-searchable PDFs:
            >>> from kreuzberg import ExtractionConfig, OcrConfig, TesseractConfig
            >>> config = ExtractionConfig(ocr=OcrConfig(backend="tesseract", language="eng", tesseract_config=TesseractConfig(psm=6)))
    """

    use_cache: bool
    enable_quality_processing: bool
    ocr: OcrConfig | None
    force_ocr: bool
    chunking: ChunkingConfig | None
    images: ImageExtractionConfig | None
    pdf_options: PdfConfig | None
    token_reduction: TokenReductionConfig | None
    language_detection: LanguageDetectionConfig | None
    keywords: KeywordConfig | None
    postprocessor: PostProcessorConfig | None
    max_concurrent_extractions: int | None
    html_options: HtmlConversionOptions | None
    pages: PageConfig | None
    security_limits: dict[str, int] | None
    result_format: str
    output_format: str
    include_document_structure: bool
    layout: LayoutDetectionConfig | None
    acceleration: AccelerationConfig | None
    email: EmailConfig | None
    concurrency: ConcurrencyConfig | None
    cache_namespace: str | None
    cache_ttl_secs: int | None

    def __init__(
        self,
        *,
        use_cache: bool | None = None,
        enable_quality_processing: bool | None = None,
        ocr: OcrConfig | None = None,
        force_ocr: bool | None = None,
        chunking: ChunkingConfig | None = None,
        images: ImageExtractionConfig | None = None,
        pdf_options: PdfConfig | None = None,
        token_reduction: TokenReductionConfig | None = None,
        language_detection: LanguageDetectionConfig | None = None,
        keywords: KeywordConfig | None = None,
        postprocessor: PostProcessorConfig | None = None,
        max_concurrent_extractions: int | None = None,
        html_options: HtmlConversionOptions | None = None,
        pages: PageConfig | None = None,
        security_limits: dict[str, int] | None = None,
        result_format: str | None = None,
        output_format: str | None = None,
        include_document_structure: bool | None = None,
        layout: LayoutDetectionConfig | None = None,
        acceleration: AccelerationConfig | None = None,
        email: EmailConfig | None = None,
        concurrency: ConcurrencyConfig | None = None,
        cache_namespace: str | None = ...,
        cache_ttl_secs: int | None = ...,
    ) -> None: ...
    @staticmethod
    def from_file(path: str | Path) -> ExtractionConfig: ...
    @staticmethod
    def discover() -> ExtractionConfig: ...

class FileExtractionConfig:
    """Per-file extraction configuration overrides for batch processing.

    All fields are optional — None means "use the batch-level default."
    Used with batch_extract_files and batch_extract_bytes via the file_configs parameter
    to allow heterogeneous extraction settings within a single batch.
    """

    enable_quality_processing: bool | None
    ocr: OcrConfig | None
    force_ocr: bool | None
    chunking: ChunkingConfig | None
    images: ImageExtractionConfig | None
    pdf_options: PdfConfig | None
    token_reduction: TokenReductionConfig | None
    language_detection: LanguageDetectionConfig | None
    pages: PageConfig | None
    keywords: KeywordConfig | None
    postprocessor: PostProcessorConfig | None
    html_options: HtmlConversionOptions | None
    result_format: str | None
    output_format: str | None
    include_document_structure: bool | None
    layout: LayoutDetectionConfig | None

    def __init__(
        self,
        *,
        enable_quality_processing: bool | None = None,
        ocr: OcrConfig | None = None,
        force_ocr: bool | None = None,
        chunking: ChunkingConfig | None = None,
        images: ImageExtractionConfig | None = None,
        pdf_options: PdfConfig | None = None,
        token_reduction: TokenReductionConfig | None = None,
        language_detection: LanguageDetectionConfig | None = None,
        pages: PageConfig | None = None,
        keywords: KeywordConfig | None = None,
        postprocessor: PostProcessorConfig | None = None,
        html_options: HtmlConversionOptions | None = None,
        result_format: str | None = None,
        output_format: str | None = None,
        include_document_structure: bool | None = None,
        layout: LayoutDetectionConfig | None = None,
    ) -> None: ...

class OcrConfig:
    """OCR configuration for extracting text from images.

    Attributes:
        backend (str): OCR backend to use. Options: "tesseract", "easyocr", "paddleocr".
            Default: "tesseract"

        language (str): Language code (ISO 639-3 three-letter code or ISO 639-1
            two-letter code). Examples: "eng", "deu", "fra", "en", "de", "fr".
            Default: "eng"

        tesseract_config (TesseractConfig | None): Tesseract-specific configuration
            for fine-tuning OCR behavior. Only used when backend="tesseract".
            Default: None

        paddle_ocr_config (PaddleOcrConfig | None): PaddleOCR-specific configuration
            for fine-tuning OCR behavior. Only used when backend="paddleocr".
            Default: None

        element_config (OcrElementConfig | None): OCR element configuration for
            extracting individual OCR elements (words, lines, etc.).
            Default: None

    Example:
        Using Tesseract with German language:
            >>> from kreuzberg import OcrConfig
            >>> config = OcrConfig(backend="tesseract", language="deu")

        Using EasyOCR for faster recognition:
            >>> config = OcrConfig(backend="easyocr", language="eng")

        Using PaddleOCR for production deployments:
            >>> config = OcrConfig(backend="paddleocr", language="chi_sim")
    """

    backend: str
    language: str
    tesseract_config: TesseractConfig | None
    paddle_ocr_config: PaddleOcrConfig | None
    element_config: OcrElementConfig | None

    def __init__(
        self,
        *,
        backend: str | None = None,
        language: str | None = None,
        tesseract_config: TesseractConfig | None = None,
        paddle_ocr_config: PaddleOcrConfig | None = None,
        element_config: OcrElementConfig | None = None,
    ) -> None: ...

class EmbeddingModelType:
    """Embedding model type selector with multiple configurations.

    Choose from preset configurations (recommended), specific fastembed models,
    or custom ONNX models for different embedding tasks.

    Static Methods:
        preset(name: str) -> EmbeddingModelType: Use a preset configuration.
            Recommended for most use cases. Available presets: balanced, compact,
            large. Call list_embedding_presets() to see all available presets.

        fastembed(model: str, dimensions: int) -> EmbeddingModelType: Use a specific
            fastembed model by name. Requires embeddings feature.

        custom(model_id: str, dimensions: int) -> EmbeddingModelType: Use a custom
            ONNX model from HuggingFace (e.g., sentence-transformers/*).

    Example:
        Using the balanced preset (recommended for general use):
            >>> from kreuzberg import EmbeddingModelType
            >>> model = EmbeddingModelType.preset("balanced")

        Using a specific fast embedding model:
            >>> model = EmbeddingModelType.fastembed(model="BAAI/bge-small-en-v1.5", dimensions=384)

        Using a custom HuggingFace model:
            >>> model = EmbeddingModelType.custom(model_id="sentence-transformers/all-MiniLM-L6-v2", dimensions=384)

        Listing available presets:
            >>> from kreuzberg import list_embedding_presets
            >>> presets = list_embedding_presets()
            >>> print(f"Available presets: {presets}")
    """
    @staticmethod
    def preset(name: str) -> EmbeddingModelType: ...
    @staticmethod
    def fastembed(model: str, dimensions: int) -> EmbeddingModelType: ...
    @staticmethod
    def custom(model_id: str, dimensions: int) -> EmbeddingModelType: ...

class EmbeddingConfig:
    """Embedding generation configuration for text chunks.

    Configures embedding generation using ONNX models via fastembed-rs.
    Embeddings are useful for semantic search, clustering, and similarity operations.

    Requires the embeddings feature to be enabled in the Rust core.

    Attributes:
        model (EmbeddingModelType): The embedding model to use. Can be a preset
            (recommended), specific fastembed model, or custom ONNX model.
            Default: Preset "balanced"

        normalize (bool): Whether to normalize embedding vectors to unit length.
            Recommended for cosine similarity calculations. Default: True

        batch_size (int): Number of texts to process simultaneously. Higher values
            use more memory but may be faster. Default: 32

        show_download_progress (bool): Display progress during embedding model
            download. Useful for large models on slow connections. Default: False

        cache_dir (str | None): Custom directory for caching downloaded models.
            Defaults to ~/.cache/kreuzberg/embeddings/ if not specified.
            Default: None

    Example:
        Basic preset embedding (recommended):
            >>> from kreuzberg import EmbeddingConfig, EmbeddingModelType
            >>> config = EmbeddingConfig()

        Specific preset with settings:
            >>> from kreuzberg import EmbeddingConfig, EmbeddingModelType
            >>> config = EmbeddingConfig(model=EmbeddingModelType.preset("balanced"), normalize=True, batch_size=64)

        Custom ONNX model:
            >>> config = EmbeddingConfig(
            ...     model=EmbeddingModelType.custom(model_id="sentence-transformers/all-MiniLM-L6-v2", dimensions=384)
            ... )

        With custom cache directory:
            >>> config = EmbeddingConfig(cache_dir="/path/to/model/cache")
    """

    normalize: bool
    batch_size: int

    def __init__(
        self,
        *,
        model: EmbeddingModelType | None = None,
        normalize: bool | None = None,
        batch_size: int | None = None,
        show_download_progress: bool | None = None,
        cache_dir: str | None = None,
    ) -> None: ...

class EmbeddingPreset:
    """Embedding preset configuration metadata.

    Provides information about a predefined embedding configuration, including
    recommended chunking parameters and model details.

    Attributes:
        name (str): Preset name (e.g., "balanced", "compact", "large")

        chunk_size (int): Recommended chunk size in characters for this preset.
            Optimized for the underlying model's strengths.

        overlap (int): Recommended chunk overlap in characters.

        model_name (str): Name of the underlying embedding model used.

        dimensions (int): Vector dimensions produced by the model.

        description (str): Human-readable description of the preset and its use cases.

    Example:
        List all available embedding presets::

            from kreuzberg import list_embedding_presets, get_embedding_preset

            presets = list_embedding_presets()
            for preset_name in presets:
                preset = get_embedding_preset(preset_name)
                print(f"{preset.name}: {preset.description}")
                print(f"  Dimensions: {preset.dimensions}")
                print(f"  Chunk size: {preset.chunk_size}")

        Get a specific preset::

            balanced = get_embedding_preset("balanced")
            print(f"Model: {balanced.model_name}")
            print(f"Dimensions: {balanced.dimensions}")
    """

    name: str
    chunk_size: int
    overlap: int
    model_name: str
    dimensions: int
    description: str

class ChunkingConfig:
    """Text chunking configuration for dividing content into chunks.

    Chunking is useful for preparing content for embedding, indexing, or processing
    with length-limited systems (like LLM context windows).

    Attributes:
        max_chars (int): Maximum number of characters per chunk. Chunks larger than
            this will be split intelligently at sentence/paragraph boundaries.
            Default: 1000

        max_overlap (int): Overlap between consecutive chunks in characters. Creates
            context bridges between chunks for smoother processing.
            Default: 200

        embedding (EmbeddingConfig | None): Configuration for generating embeddings
            for each chunk using ONNX models. None = no embeddings. Default: None

        preset (str | None): Use a preset chunking configuration (overrides individual
            settings if provided). Use list_embedding_presets() to see available presets.
            Default: None

    Example:
        Basic chunking with defaults:
            >>> from kreuzberg import ExtractionConfig, ChunkingConfig
            >>> config = ExtractionConfig(chunking=ChunkingConfig())

        Custom chunk size with overlap:
            >>> config = ExtractionConfig(chunking=ChunkingConfig(max_chars=512, max_overlap=100))

        Chunking with embeddings:
            >>> from kreuzberg import EmbeddingConfig, EmbeddingModelType
            >>> config = ExtractionConfig(
            ...     chunking=ChunkingConfig(max_chars=512, embedding=EmbeddingConfig(model=EmbeddingModelType.preset("balanced")))
            ... )

        Using preset configuration:
            >>> config = ExtractionConfig(chunking=ChunkingConfig(preset="semantic"))
    """

    max_chars: int
    max_overlap: int
    embedding: EmbeddingConfig | None
    preset: str | None
    chunker_type: str | None
    sizing_type: str
    sizing_model: str | None

    def __init__(
        self,
        *,
        max_chars: int | None = None,
        max_overlap: int | None = None,
        embedding: EmbeddingConfig | None = None,
        preset: str | None = None,
        chunker_type: str | None = None,
        sizing_type: str | None = None,
        sizing_model: str | None = None,
        sizing_cache_dir: str | None = None,
    ) -> None: ...

class ImageExtractionConfig:
    """Configuration for extracting images FROM documents.

    This configuration controls image extraction from documents like PDFs and
    presentations. It is NOT for preprocessing images before OCR.
    (See ImagePreprocessingConfig for OCR preprocessing.)

    Attributes:
        extract_images (bool): Enable image extraction from documents.
            Default: True

        target_dpi (int): Target DPI for image normalization. Images are resampled
            to this DPI for consistency. Default: 300

        max_image_dimension (int): Maximum width or height for extracted images.
            Larger images are downscaled to fit. Default: 4096

        auto_adjust_dpi (bool): Automatically adjust DPI based on image content
            quality. May override target_dpi for better results. Default: True

        min_dpi (int): Minimum DPI threshold. Images with lower DPI are upscaled.
            Default: 72

        max_dpi (int): Maximum DPI threshold. Images with higher DPI are downscaled.
            Default: 600

    Example:
        Basic image extraction:
            >>> from kreuzberg import ExtractionConfig, ImageExtractionConfig
            >>> config = ExtractionConfig(images=ImageExtractionConfig())

        Extract images with custom DPI settings:
            >>> config = ExtractionConfig(images=ImageExtractionConfig(target_dpi=150, max_image_dimension=2048, auto_adjust_dpi=False))

        Note: For OCR image preprocessing (not image extraction from documents):
            >>> from kreuzberg import TesseractConfig, ImagePreprocessingConfig
            >>> config = TesseractConfig(preprocessing=ImagePreprocessingConfig(target_dpi=300, denoise=True))
    """

    extract_images: bool
    target_dpi: int
    max_image_dimension: int
    auto_adjust_dpi: bool
    min_dpi: int
    max_dpi: int

    def __init__(
        self,
        *,
        extract_images: bool | None = None,
        target_dpi: int | None = None,
        max_image_dimension: int | None = None,
        auto_adjust_dpi: bool | None = None,
        min_dpi: int | None = None,
        max_dpi: int | None = None,
    ) -> None: ...

class PdfConfig:
    """PDF-specific extraction configuration.

    Attributes:
        extract_images (bool): Extract images from PDF documents.
            Default: False

        passwords (list[str] | None): List of passwords to try when opening
            encrypted PDFs. Try each password in order until one succeeds.
            Default: None

        extract_metadata (bool): Extract PDF metadata (title, author, creation date,
            etc.). Default: True

        hierarchy (HierarchyConfig | None): Document hierarchy detection configuration
            for detecting document structure and organization. None = no hierarchy detection.
            Default: None

    Example:
        Basic PDF configuration:
            >>> from kreuzberg import ExtractionConfig, PdfConfig
            >>> config = ExtractionConfig(pdf_options=PdfConfig())

        Extract metadata and images from PDF:
            >>> config = ExtractionConfig(pdf_options=PdfConfig(extract_images=True, extract_metadata=True))

        Handle encrypted PDFs:
            >>> config = ExtractionConfig(pdf_options=PdfConfig(passwords=["password123", "fallback_password"]))

        Enable hierarchy detection:
            >>> config = ExtractionConfig(pdf_options=PdfConfig(hierarchy=HierarchyConfig(k_clusters=6)))
    """

    extract_images: bool
    passwords: list[str] | None
    extract_metadata: bool
    hierarchy: HierarchyConfig | None
    extract_annotations: bool
    top_margin_fraction: float | None
    bottom_margin_fraction: float | None
    allow_single_column_tables: bool

    def __init__(
        self,
        *,
        extract_images: bool | None = None,
        passwords: list[str] | None = None,
        extract_metadata: bool | None = None,
        hierarchy: HierarchyConfig | None = None,
        extract_annotations: bool | None = None,
        top_margin_fraction: float | None = None,
        bottom_margin_fraction: float | None = None,
        allow_single_column_tables: bool | None = None,
    ) -> None: ...

class HierarchyConfig:
    """Document hierarchy detection configuration.

    Controls detection of document structure and hierarchy using clustering algorithms.

    Attributes:
        enabled (bool): Enable hierarchy detection. Default: True

        k_clusters (int): Number of clusters for k-means clustering.
            Default: 6

        include_bbox (bool): Include bounding box information in hierarchy output.
            Default: True

        ocr_coverage_threshold (float | None): Optional threshold for OCR coverage
            before enabling hierarchy detection. Default: None

    Example:
        Basic hierarchy detection:
            >>> from kreuzberg import ExtractionConfig, HierarchyConfig
            >>> config = ExtractionConfig(hierarchy=HierarchyConfig())

        Customize clustering parameters:
            >>> config = ExtractionConfig(hierarchy=HierarchyConfig(k_clusters=8, include_bbox=False))

        Conditional hierarchy detection with OCR:
            >>> config = ExtractionConfig(hierarchy=HierarchyConfig(ocr_coverage_threshold=0.5))
    """

    enabled: bool
    k_clusters: int
    include_bbox: bool
    ocr_coverage_threshold: float | None

    def __init__(
        self,
        *,
        enabled: bool | None = None,
        k_clusters: int | None = None,
        include_bbox: bool | None = None,
        ocr_coverage_threshold: float | None = None,
    ) -> None: ...

class PageConfig:
    r"""Page extraction and tracking configuration.

    Controls whether Kreuzberg tracks page boundaries and optionally inserts page markers
    into the extracted `content`.

    Attributes:
        extract_pages (bool): Enable page tracking and per-page extraction. Default: False
        insert_page_markers (bool): Insert page markers into `content`. Default: False
        marker_format (str): Marker template containing `{page_num}`. Default: "\\n\\n<!-- PAGE {page_num} -->\\n\\n"

    Example:
        >>> from kreuzberg import ExtractionConfig, PageConfig
        >>> config = ExtractionConfig(pages=PageConfig(extract_pages=True))
    """

    extract_pages: bool
    insert_page_markers: bool
    marker_format: str

    def __init__(
        self,
        *,
        extract_pages: bool | None = None,
        insert_page_markers: bool | None = None,
        marker_format: str | None = None,
    ) -> None: ...

class KeywordAlgorithm:
    Yake: KeywordAlgorithm
    Rake: KeywordAlgorithm

class YakeParams:
    """YAKE-specific parameters.

    Attributes:
        window_size (int): Context window size. Default: 2
    """

    window_size: int

    def __init__(self, *, window_size: int | None = None) -> None: ...

class RakeParams:
    """RAKE-specific parameters.

    Attributes:
        min_word_length (int): Minimum word length. Default: 1
        max_words_per_phrase (int): Maximum words per phrase. Default: 3
    """

    min_word_length: int
    max_words_per_phrase: int

    def __init__(
        self,
        *,
        min_word_length: int | None = None,
        max_words_per_phrase: int | None = None,
    ) -> None: ...

class KeywordConfig:
    """Keyword extraction configuration.

    Attributes:
        algorithm (KeywordAlgorithm): Keyword extraction algorithm.
        max_keywords (int): Maximum number of keywords to extract. Default: 10
        min_score (float): Minimum score threshold. Default: 0.0
        ngram_range (tuple[int, int]): N-gram range. Default: (1, 3)
        language (str | None): Optional language hint. Default: "en"
        yake_params (YakeParams | None): YAKE-specific tuning. Default: None
        rake_params (RakeParams | None): RAKE-specific tuning. Default: None
    """

    algorithm: KeywordAlgorithm
    max_keywords: int
    min_score: float
    ngram_range: tuple[int, int]
    language: str | None
    yake_params: YakeParams | None
    rake_params: RakeParams | None

    def __init__(
        self,
        *,
        algorithm: KeywordAlgorithm | None = None,
        max_keywords: int | None = None,
        min_score: float | None = None,
        ngram_range: tuple[int, int] | None = None,
        language: str | None = None,
        yake_params: YakeParams | None = None,
        rake_params: RakeParams | None = None,
    ) -> None: ...

class TokenReductionConfig:
    """Configuration for reducing token count in extracted content.

    Reduces token count to lower costs when working with LLM APIs. Higher modes
    are more aggressive but may lose more information.

    Attributes:
        mode (str): Token reduction mode. Options: "off", "light", "moderate",
            "aggressive", "maximum". Default: "off"
            - "off": No token reduction
            - "light": Remove extra whitespace and redundant punctuation
            - "moderate": Also remove common filler words and some formatting
            - "aggressive": Also remove longer stopwords and collapse similar phrases
            - "maximum": Maximum reduction while preserving semantic content

        preserve_important_words (bool): Preserve capitalized words, technical terms,
            and proper nouns even in aggressive reduction modes. Default: True

    Example:
        Moderate token reduction:
            >>> from kreuzberg import ExtractionConfig, TokenReductionConfig
            >>> config = ExtractionConfig(token_reduction=TokenReductionConfig(mode="moderate", preserve_important_words=True))

        Maximum reduction for large batches:
            >>> config = ExtractionConfig(token_reduction=TokenReductionConfig(mode="maximum", preserve_important_words=True))

        No reduction (default):
            >>> config = ExtractionConfig(token_reduction=TokenReductionConfig(mode="off"))
    """

    mode: str
    preserve_important_words: bool

    def __init__(
        self,
        *,
        mode: Literal["off", "light", "moderate", "aggressive", "maximum"] | None = None,
        preserve_important_words: bool | None = None,
    ) -> None: ...

class LanguageDetectionConfig:
    """Configuration for detecting document language(s).

    Attributes:
        enabled (bool): Enable language detection for extracted content.
            Default: True

        min_confidence (float): Minimum confidence threshold (0.0-1.0) for language
            detection. Results below this threshold are discarded. Default: 0.8

        detect_multiple (bool): Detect multiple languages in the document. When False,
            only the most confident language is returned. Default: False

    Example:
        Basic language detection:
            >>> from kreuzberg import ExtractionConfig, LanguageDetectionConfig
            >>> config = ExtractionConfig(language_detection=LanguageDetectionConfig())

        Detect multiple languages with lower confidence threshold:
            >>> config = ExtractionConfig(language_detection=LanguageDetectionConfig(detect_multiple=True, min_confidence=0.6))

        Access detected languages in result:
            >>> from kreuzberg import extract_file_sync
            >>> result = extract_file_sync("multilingual.pdf", config=config)
            >>> print(f"Languages: {result.detected_languages}")
    """

    enabled: bool
    min_confidence: float
    detect_multiple: bool

    def __init__(
        self,
        *,
        enabled: bool | None = None,
        min_confidence: float | None = None,
        detect_multiple: bool | None = None,
    ) -> None: ...

class PostProcessorConfig:
    """Configuration for post-processors in the extraction pipeline.

    Post-processors allow custom text processing after extraction. They can be used
    to normalize text, fix formatting issues, or apply domain-specific transformations.

    Attributes:
        enabled (bool): Enable post-processors in the extraction pipeline.
            Default: True

        enabled_processors (list[str] | None): Whitelist of processor names to run.
            If specified, only these processors are executed. None = run all enabled.
            Default: None

        disabled_processors (list[str] | None): Blacklist of processor names to skip.
            If specified, these processors are not executed. None = none disabled.
            Default: None

    Example:
        Basic post-processing with defaults:
            >>> from kreuzberg import ExtractionConfig, PostProcessorConfig
            >>> config = ExtractionConfig(postprocessor=PostProcessorConfig())

        Enable only specific processors:
            >>> config = ExtractionConfig(
            ...     postprocessor=PostProcessorConfig(enabled=True, enabled_processors=["normalize_whitespace", "fix_encoding"])
            ... )

        Disable specific processors:
            >>> config = ExtractionConfig(postprocessor=PostProcessorConfig(enabled=True, disabled_processors=["experimental_cleanup"]))

        Disable all post-processing:
            >>> config = ExtractionConfig(postprocessor=PostProcessorConfig(enabled=False))

        Register custom post-processor:
            >>> from kreuzberg import register_post_processor
            >>> class MyProcessor:
            ...     def name(self) -> str:
            ...         return "my_processor"
            ...     def process(self, result) -> Any:
            ...         return result
            ...     def processing_stage(self) -> str:
            ...         return "middle"
            >>> register_post_processor(MyProcessor())
    """

    enabled: bool
    enabled_processors: list[str] | None
    disabled_processors: list[str] | None

    def __init__(
        self,
        *,
        enabled: bool | None = None,
        enabled_processors: list[str] | None = None,
        disabled_processors: list[str] | None = None,
    ) -> None: ...

class ImagePreprocessingConfig:
    """Configuration for preprocessing images before OCR.

    This configuration controls image preprocessing for OCR operations. It is NOT for
    extracting images from documents. (See ImageExtractionConfig for image extraction.)

    Attributes:
        target_dpi (int): Target DPI for image normalization before OCR.
            Default: 300

        auto_rotate (bool): Automatically detect and correct image rotation.
            Default: True

        deskew (bool): Correct skewed images to improve OCR accuracy.
            Default: True

        denoise (bool): Apply denoising filters to reduce noise in images.
            Improves OCR accuracy on low-quality scans. Default: False

        contrast_enhance (bool): Enhance contrast to improve text readability.
            Default: False

        binarization_method (str): Method for converting images to black and white.
            Options depend on the OCR backend. Default: "otsu"

        invert_colors (bool): Invert colors (white text on black background).
            Useful for certain document types. Default: False

    Example:
        Basic preprocessing for OCR:
            >>> from kreuzberg import TesseractConfig, ImagePreprocessingConfig
            >>> config = TesseractConfig(preprocessing=ImagePreprocessingConfig())

        Aggressive preprocessing for low-quality scans:
            >>> config = TesseractConfig(
            ...     preprocessing=ImagePreprocessingConfig(
            ...         target_dpi=300, denoise=True, contrast_enhance=True, auto_rotate=True, deskew=True
            ...     )
            ... )
    """

    target_dpi: int
    auto_rotate: bool
    deskew: bool
    denoise: bool
    contrast_enhance: bool
    binarization_method: str
    invert_colors: bool

    def __init__(
        self,
        *,
        target_dpi: int | None = None,
        auto_rotate: bool | None = None,
        deskew: bool | None = None,
        denoise: bool | None = None,
        contrast_enhance: bool | None = None,
        binarization_method: str | None = None,
        invert_colors: bool | None = None,
    ) -> None: ...

class TesseractConfig:
    """Detailed Tesseract OCR configuration for advanced tuning.

    Fine-tune Tesseract OCR behavior for specific document types and quality levels.
    Most documents work well with defaults; adjust these settings for specialized cases.

    Attributes:
        language (str): OCR language (ISO 639-3 three-letter code).
            Default: "eng"

        psm (int): Page Segmentation Mode - how to analyze page layout.
            Default: 3 (Fully automatic page segmentation)
            - 0: Orientation and script detection only (no OCR)
            - 3: Fully automatic page segmentation (default)
            - 6: Uniform block of text
            - 11: Sparse text - find as much text as possible
            Examples: psm=3 for general documents, psm=6 for simple uniform text.
            See Tesseract documentation for all modes.

        output_format (str): Output format for OCR results.
            Default: "markdown"

        oem (int): OCR Engine Mode - which OCR engine to use.
            Default: 3 (Auto - tesseract default)
            - 0: Legacy Tesseract only (older, fast)
            - 1: Neural Nets LSTM only (newer, accurate)
            - 2: Both legacy and LSTM (best accuracy)
            - 3: Default - Tesseract chooses based on build

        min_confidence (float): Minimum confidence threshold (0.0-1.0) for accepting
            OCR results. Default: 0.0 (accept all results)

        preprocessing (ImagePreprocessingConfig | None): Image preprocessing configuration
            for cleaning up images before OCR. Default: None

        enable_table_detection (bool): Enable automatic table detection and extraction.
            Default: True

        table_min_confidence (float): Minimum confidence for table detection (0.0-1.0).
            Default: 0.0

        table_column_threshold (int): Minimum pixel width between columns.
            Default: 50

        table_row_threshold_ratio (float): Minimum row height ratio.
            Default: 0.5

        use_cache (bool): Cache OCR results for improved performance.
            Default: True

        classify_use_pre_adapted_templates (bool): Use pre-adapted character templates.
            Default: True

        language_model_ngram_on (bool): Enable language model n-gram processing.
            Default: False

        tessedit_dont_blkrej_good_wds (bool): Don't block-reject good words.
            Default: True

        tessedit_dont_rowrej_good_wds (bool): Don't row-reject good words.
            Default: True

        tessedit_enable_dict_correction (bool): Enable dictionary-based spelling correction.
            Default: True

        tessedit_char_whitelist (str): Whitelist of characters to recognize.
            Only these characters will be recognized. Empty = all characters.
            Default: ""

        tessedit_char_blacklist (str): Blacklist of characters to ignore.
            These characters will be skipped. Default: ""

        tessedit_use_primary_params_model (bool): Use primary parameters model.
            Default: True

        textord_space_size_is_variable (bool): Allow variable space sizes.
            Default: True

        thresholding_method (bool): Thresholding method for binarization.
            Default: False

    Example:
        General document OCR:
            >>> from kreuzberg import TesseractConfig
            >>> config = TesseractConfig(psm=3, oem=3)

        Invoice/form OCR with table detection:
            >>> config = TesseractConfig(psm=6, oem=2, enable_table_detection=True, min_confidence=0.6)

        High-precision technical document OCR:
            >>> from kreuzberg import ImagePreprocessingConfig
            >>> config = TesseractConfig(
            ...     psm=3,
            ...     oem=2,
            ...     preprocessing=ImagePreprocessingConfig(denoise=True, contrast_enhance=True, auto_rotate=True),
            ...     min_confidence=0.7,
            ...     tessedit_enable_dict_correction=True,
            ... )

        Numeric-only OCR (for receipts, barcodes):
            >>> config = TesseractConfig(psm=6, tessedit_char_whitelist="0123456789.-,", min_confidence=0.8)

        Multiple language document:
            >>> config = TesseractConfig(language="eng+fra+deu", psm=3, oem=2)
    """

    language: str
    psm: int
    output_format: str
    oem: int
    min_confidence: float
    preprocessing: ImagePreprocessingConfig | None
    enable_table_detection: bool
    table_min_confidence: float
    table_column_threshold: int
    table_row_threshold_ratio: float
    use_cache: bool
    classify_use_pre_adapted_templates: bool
    language_model_ngram_on: bool
    tessedit_dont_blkrej_good_wds: bool
    tessedit_dont_rowrej_good_wds: bool
    tessedit_enable_dict_correction: bool
    tessedit_char_whitelist: str
    tessedit_char_blacklist: str
    tessedit_use_primary_params_model: bool
    textord_space_size_is_variable: bool
    thresholding_method: bool

    def __init__(
        self,
        *,
        language: str | None = None,
        psm: int | None = None,
        output_format: str | None = None,
        oem: int | None = None,
        min_confidence: float | None = None,
        preprocessing: ImagePreprocessingConfig | None = None,
        enable_table_detection: bool | None = None,
        table_min_confidence: float | None = None,
        table_column_threshold: int | None = None,
        table_row_threshold_ratio: float | None = None,
        use_cache: bool | None = None,
        classify_use_pre_adapted_templates: bool | None = None,
        language_model_ngram_on: bool | None = None,
        tessedit_dont_blkrej_good_wds: bool | None = None,
        tessedit_dont_rowrej_good_wds: bool | None = None,
        tessedit_enable_dict_correction: bool | None = None,
        tessedit_char_whitelist: str | None = None,
        tessedit_char_blacklist: str | None = None,
        tessedit_use_primary_params_model: bool | None = None,
        textord_space_size_is_variable: bool | None = None,
        thresholding_method: bool | None = None,
    ) -> None: ...

class PdfMetadata(TypedDict, total=False):
    pdf_version: str
    producer: str
    is_encrypted: bool
    width: int
    height: int
    page_count: int

class ExcelMetadata(TypedDict, total=False):
    sheet_count: int
    sheet_names: list[str]

class EmailMetadata(TypedDict, total=False):
    from_email: str
    from_name: str
    to_emails: list[str]
    cc_emails: list[str]
    bcc_emails: list[str]
    message_id: str
    attachments: list[str]

class PptxMetadata(TypedDict, total=False):
    slide_count: int
    slide_names: list[str]

class ArchiveMetadata(TypedDict, total=False):
    format: str
    file_count: int
    file_list: list[str]
    total_size: int
    compressed_size: int

class ImageMetadata(TypedDict, total=False):
    width: int
    height: int
    format: str
    exif: dict[str, str]

class XmlMetadata(TypedDict, total=False):
    element_count: int
    unique_elements: list[str]

class TextMetadata(TypedDict, total=False):
    line_count: int
    word_count: int
    character_count: int
    headers: list[str] | None
    links: list[tuple[str, str]] | None
    code_blocks: list[tuple[str, str]] | None

class HeaderMetadata(TypedDict):
    level: int
    text: str
    id: str | None
    depth: int
    html_offset: int

class LinkMetadata(TypedDict):
    href: str
    text: str
    title: str | None
    link_type: Literal["anchor", "internal", "external", "email", "phone", "other"]
    rel: list[str]
    attributes: dict[str, str]

class HtmlImageMetadata(TypedDict):
    src: str
    alt: str | None
    title: str | None
    dimensions: tuple[int, int] | None
    image_type: Literal["data-uri", "inline-svg", "external", "relative"]
    attributes: dict[str, str]

class StructuredData(TypedDict):
    data_type: Literal["json-ld", "microdata", "rdfa"]
    raw_json: str
    schema_type: str | None

class HtmlMetadata(TypedDict, total=False):
    title: str | None
    description: str | None
    keywords: list[str]
    author: str | None
    canonical_url: str | None
    base_href: str | None
    language: str | None
    text_direction: Literal["ltr", "rtl", "auto"] | None
    open_graph: dict[str, str]
    twitter_card: dict[str, str]
    meta_tags: dict[str, str]
    headers: list[HeaderMetadata]
    links: list[LinkMetadata]
    images: list[HtmlImageMetadata]
    structured_data: list[StructuredData]

class OcrMetadata(TypedDict, total=False):
    language: str
    psm: int
    output_format: str
    table_count: int
    table_rows: int | None
    table_cols: int | None

class ImagePreprocessingMetadata(TypedDict, total=False):
    original_dimensions: tuple[int, int]
    original_dpi: tuple[float, float]
    target_dpi: int
    scale_factor: float
    auto_adjusted: bool
    final_dpi: int
    new_dimensions: tuple[int, int]
    resample_method: str
    dimension_clamped: bool
    calculated_dpi: int
    skipped_resize: bool
    resize_error: str

class ErrorMetadata(TypedDict, total=False):
    error_type: str
    message: str

class PageBoundary(TypedDict):
    byte_start: int
    byte_end: int
    page_number: int

class PageInfo(TypedDict, total=False):
    number: int
    title: str | None
    dimensions: tuple[float, float] | None
    image_count: int | None
    table_count: int | None
    hidden: bool | None
    is_blank: bool | None

class PageStructure(TypedDict, total=False):
    total_count: int
    unit_type: Literal["page", "slide", "sheet"]
    boundaries: list[PageBoundary] | None
    pages: list[PageInfo] | None

class Metadata(TypedDict, total=False):
    # Common fields (set directly on all extractions)
    title: str
    subject: str
    authors: list[str]
    keywords: list[str]
    language: str
    created_at: str
    modified_at: str
    created_by: str
    modified_by: str
    pages: PageStructure

    # Format discriminator (from serde tag)
    format_type: Literal["pdf", "excel", "email", "pptx", "archive", "image", "xml", "text", "html", "ocr"]

    # PDF-specific (flattened from PdfMetadata)
    pdf_version: str | None
    producer: str | None
    is_encrypted: bool | None
    width: int | None
    height: int | None
    page_count: int | None

    # Excel-specific (flattened from ExcelMetadata)
    sheet_count: int
    sheet_names: list[str]

    # Email-specific (flattened from EmailMetadata)
    from_email: str | None
    from_name: str | None
    to_emails: list[str]
    cc_emails: list[str]
    bcc_emails: list[str]
    message_id: str | None
    attachments: list[str]

    # PPTX-specific (flattened from PptxMetadata)
    slide_count: int
    slide_names: list[str]

    # Archive-specific (flattened from ArchiveMetadata)
    format: str
    file_count: int
    file_list: list[str]
    total_size: int
    compressed_size: int | None

    # Image-specific (flattened from ImageMetadata)
    # Note: 'width', 'height', 'format' overlap with other fields
    exif: dict[str, str]

    # XML-specific (flattened from XmlMetadata)
    element_count: int
    unique_elements: list[str]

    # Text-specific (flattened from TextMetadata)
    line_count: int
    word_count: int
    character_count: int
    # Note: 'headers' is list[str] for text, list[HeaderMetadata] for html
    headers: list[str] | list[HeaderMetadata] | None
    # Note: 'links' is list[tuple[str, str]] for text, list[LinkMetadata] for html
    links: list[tuple[str, str]] | list[LinkMetadata] | None
    code_blocks: list[tuple[str, str]] | None

    # HTML-specific (flattened from HtmlMetadata)
    # Note: 'title', 'description', 'keywords', 'language' overlap with common fields
    author: str | None
    description: str | None
    canonical_url: str | None
    base_href: str | None
    text_direction: str | None
    open_graph: dict[str, str]
    twitter_card: dict[str, str]
    meta_tags: dict[str, str]
    images: list[HtmlImageMetadata]
    structured_data: list[StructuredData]

    # OCR-specific (flattened from OcrMetadata)
    # Note: 'language' overlaps with common field above
    psm: int
    output_format: str | None
    table_count: int
    table_rows: int | None
    table_cols: int | None

    # Additional metadata fields
    category: str | None
    tags: list[str] | None
    document_version: str | None
    abstract_text: str | None

    # Processing metadata
    extraction_duration_ms: int | None
    image_preprocessing: ImagePreprocessingMetadata
    json_schema: Any
    error: ErrorMetadata

class ExtractedKeyword:
    text: str
    score: float
    algorithm: str
    positions: list[int] | None

class ProcessingWarning:
    source: str
    message: str

PdfAnnotationType: TypeAlias = Literal[
    "text",
    "highlight",
    "link",
    "stamp",
    "underline",
    "strike_out",
    "other",
]

class PdfAnnotation:
    """A PDF annotation extracted from a document page.

    Attributes:
        annotation_type (str): Type of annotation ("text", "highlight", "link",
            "stamp", "underline", "strike_out", "other")
        content (str | None): Text content of the annotation
        page_number (int): Page number where the annotation appears (1-indexed)
        bounding_box (BoundingBox | None): Bounding box coordinates
    """

    annotation_type: str
    content: str | None
    page_number: int
    bounding_box: BoundingBox | None

class ExtractedImage(TypedDict, total=False):
    data: bytes
    format: str
    image_index: int
    page_number: int
    width: int
    height: int
    colorspace: str
    bits_per_component: int
    is_mask: bool
    description: str
    bounding_box: BoundingBox
    ocr_result: ExtractionResult

class HeadingLevel(TypedDict):
    level: int
    text: str

class HeadingContext(TypedDict):
    headings: list[HeadingLevel]

class ChunkMetadata(TypedDict, total=False):
    byte_start: int
    byte_end: int
    chunk_index: int
    total_chunks: int
    token_count: int | None
    first_page: int
    last_page: int
    heading_context: HeadingContext | None

class Chunk:
    content: str
    embedding: list[float] | None
    metadata: ChunkMetadata

class DjotTable(TypedDict, total=False):
    cells: list[list[str]]
    markdown: str
    page_number: int

class DjotContent(TypedDict, total=False):
    plain_text: str
    blocks: list[FormattedBlock]
    metadata: Metadata
    tables: list[DjotTable]
    images: list[DjotImage]
    links: list[DjotLink]
    footnotes: list[Footnote]
    attributes: dict[str, Attributes]

class InlineElement(TypedDict, total=False):
    element_type: str
    content: str
    attributes: Attributes | None
    metadata: dict[str, str] | None

class Attributes(TypedDict, total=False):
    id: str | None
    classes: list[str]
    key_values: dict[str, str]

class FormattedBlock(TypedDict, total=False):
    block_type: str
    level: int | None
    inline_content: list[InlineElement]
    attributes: Attributes | None
    language: str | None
    code: str | None
    children: list[FormattedBlock]

class DjotImage(TypedDict, total=False):
    src: str
    alt: str
    title: str | None
    attributes: Attributes | None

class DjotLink(TypedDict, total=False):
    url: str
    text: str
    title: str | None
    attributes: Attributes | None

class Footnote(TypedDict, total=False):
    label: str
    content: list[FormattedBlock]

class BoundingBox(TypedDict):
    x0: float
    y0: float
    x1: float
    y1: float

ElementType: TypeAlias = Literal[
    "title",
    "narrative_text",
    "heading",
    "list_item",
    "table",
    "image",
    "page_break",
    "code_block",
    "block_quote",
    "footer",
    "header",
]

class ElementMetadata(TypedDict, total=False):
    page_number: int | None
    filename: str | None
    coordinates: BoundingBox | None
    element_index: int | None
    additional: dict[str, str]

class Element(TypedDict):
    element_id: str
    element_type: ElementType
    text: str
    metadata: ElementMetadata

NodeContentType: TypeAlias = Literal[
    "title",
    "heading",
    "paragraph",
    "list",
    "list_item",
    "table",
    "image",
    "code",
    "quote",
    "formula",
    "footnote",
    "group",
    "page_break",
]

ContentLayer: TypeAlias = Literal["body", "header", "footer", "footnote"]

AnnotationType: TypeAlias = Literal[
    "bold",
    "italic",
    "underline",
    "strikethrough",
    "code",
    "subscript",
    "superscript",
    "link",
]

class GridCell(TypedDict, total=False):
    """Individual cell in a table grid."""

    content: str
    row: int
    col: int
    row_span: int
    col_span: int
    is_header: bool
    bbox: BoundingBox | None

class TableGrid(TypedDict, total=False):
    """Structured table grid with cell-level metadata."""

    rows: int
    cols: int
    cells: list[GridCell]

class TextAnnotation(TypedDict, total=False):
    """Inline text annotation with byte-range formatting.

    Annotations reference byte offsets into a node's text content.
    """

    start: int
    end: int
    annotation_type: AnnotationType
    url: str | None
    title: str | None

class NodeContent(TypedDict, total=False):
    """Tagged node content. The node_type field discriminates the variant.

    Common fields by node_type:
        title: node_type, text
        heading: node_type, text, level
        paragraph: node_type, text
        list: node_type, ordered
        list_item: node_type, text
        table: node_type, grid
        image: node_type, description, image_index
        code: node_type, text, language
        quote: node_type
        formula: node_type, text
        footnote: node_type, text
        group: node_type, label, heading_level, heading_text
        page_break: node_type
    """

    node_type: NodeContentType
    text: str
    level: int
    ordered: bool
    grid: TableGrid
    description: str | None
    image_index: int | None
    language: str | None
    label: str | None
    heading_level: int | None
    heading_text: str | None

class DocumentNode(TypedDict, total=False):
    """A node in the hierarchical document structure.

    Attributes:
        id: Deterministic node identifier generated from content hash.
        content: Node content — tagged dict with node_type discriminant.
        parent: Index of parent node, or None for root nodes.
        children: Indices of child nodes in reading order.
        content_layer: Content layer classification (body, header, footer, footnote).
        page: Page number where node starts (1-indexed), or None.
        page_end: Page number where node ends, or None.
        bbox: Bounding box of the node on the page.
        annotations: Inline text annotations (formatting, links).
    """

    id: str
    content: NodeContent
    parent: int | None
    children: list[int]
    content_layer: ContentLayer
    page: int | None
    page_end: int | None
    bbox: BoundingBox | None
    annotations: list[TextAnnotation]

class DocumentStructure(TypedDict):
    """Hierarchical document structure.

    Provides a tree-based representation of document content with nodes, parent-child
    relationships, and semantic information. Enable with ExtractionConfig(include_document_structure=True).

    Attributes:
        nodes (list[DocumentNode]): Flat array of document nodes in reading order.
            Each node contains content, position information, and relationships to other nodes.
            Parent-child relationships use index-based references into this array.

    Example:
        Access document structure after extraction:
            >>> from kreuzberg import extract_file_sync, ExtractionConfig
            >>> config = ExtractionConfig(include_document_structure=True)
            >>> result = extract_file_sync("document.pdf", None, config)
            >>> if result.document:
            ...     nodes = result.document["nodes"]
            ...     print(f"Document has {len(nodes)} nodes")
            ...     root_node = nodes[0]
            ...     print(f"Root node: {root_node['id']}")
            ...     children = [nodes[i] for i in root_node.get("children", [])]
            ...     print(f"Root has {len(children)} children")
    """

    nodes: list[DocumentNode]

class ExtractionResult:
    content: str
    mime_type: str
    metadata: Metadata
    tables: list[ExtractedTable]
    detected_languages: list[str] | None
    chunks: list[Chunk] | None
    images: list[ExtractedImage] | None
    pages: list[PageContent] | None
    elements: list[Element] | None
    document: DocumentStructure | None
    ocr_elements: list[OcrElement] | None
    djot_content: DjotContent | None
    output_format: str | None
    result_format: str | None
    extracted_keywords: list[ExtractedKeyword] | None
    quality_score: float | None
    processing_warnings: list[ProcessingWarning]
    annotations: list[PdfAnnotation] | None
    def get_page_count(self) -> int: ...
    def get_chunk_count(self) -> int: ...
    def get_detected_language(self) -> str | None: ...
    def get_metadata_field(self, field_name: str) -> Any | None: ...

class PageContent(TypedDict):
    page_number: int
    content: str
    tables: list[ExtractedTable]
    images: list[ExtractedImage]
    is_blank: bool | None

class ExtractedTable:
    cells: list[list[str]]
    markdown: str
    page_number: int
    bounding_box: BoundingBox | None

@overload
def extract_file_sync(
    path: str | Path | bytes,
    mime_type: None = None,
    config: ExtractionConfig = ...,
) -> ExtractionResult: ...
@overload
def extract_file_sync(
    path: str | Path | bytes,
    mime_type: str,
    config: ExtractionConfig = ...,
) -> ExtractionResult: ...
def extract_bytes_sync(
    data: bytes | bytearray,
    mime_type: str,
    config: ExtractionConfig = ...,
) -> ExtractionResult: ...
def batch_extract_files_sync(
    paths: list[str | Path | bytes],
    config: ExtractionConfig = ...,
    file_configs: list[FileExtractionConfig | None] | None = None,
) -> list[ExtractionResult]: ...
def batch_extract_bytes_sync(
    data_list: list[bytes | bytearray],
    mime_types: list[str],
    config: ExtractionConfig = ...,
    file_configs: list[FileExtractionConfig | None] | None = None,
) -> list[ExtractionResult]: ...
@overload
async def extract_file(
    path: str | Path | bytes,
    mime_type: None = None,
    config: ExtractionConfig = ...,
) -> ExtractionResult: ...
@overload
async def extract_file(
    path: str | Path | bytes,
    mime_type: str,
    config: ExtractionConfig = ...,
) -> ExtractionResult: ...
def extract_bytes(
    data: bytes | bytearray,
    mime_type: str,
    config: ExtractionConfig = ...,
) -> Awaitable[ExtractionResult]: ...
def batch_extract_files(
    paths: list[str | Path | bytes],
    config: ExtractionConfig = ...,
    file_configs: list[FileExtractionConfig | None] | None = None,
) -> Awaitable[list[ExtractionResult]]: ...
def batch_extract_bytes(
    data_list: list[bytes | bytearray],
    mime_types: list[str],
    config: ExtractionConfig = ...,
    file_configs: list[FileExtractionConfig | None] | None = None,
) -> Awaitable[list[ExtractionResult]]: ...
def register_ocr_backend(backend: OcrBackendProtocol) -> None: ...
def register_post_processor(processor: PostProcessorProtocol) -> None: ...
def clear_post_processors() -> None: ...
def unregister_post_processor(name: str) -> None: ...
def register_validator(validator: ValidatorProtocol) -> None: ...
def clear_validators() -> None: ...
def unregister_validator(name: str) -> None: ...
def list_embedding_presets() -> list[str]: ...
def get_embedding_preset(name: str) -> EmbeddingPreset | None: ...
def clear_document_extractors() -> None: ...
def clear_ocr_backends() -> None: ...
def detect_mime_type_from_bytes(data: bytes) -> str: ...
def detect_mime_type_from_path(path: str | Path) -> str: ...
def validate_mime_type(mime_type: str) -> str: ...
def get_extensions_for_mime(mime_type: str) -> list[str]: ...
def list_document_extractors() -> list[str]: ...
def list_ocr_backends() -> list[str]: ...
def list_post_processors() -> list[str]: ...
def list_validators() -> list[str]: ...
def unregister_document_extractor(name: str) -> None: ...
def unregister_ocr_backend(name: str) -> None: ...
def get_last_error_code() -> int: ...
def get_last_panic_context() -> str | None: ...
def validate_binarization_method(method: str) -> bool: ...
def validate_ocr_backend(backend: str) -> bool: ...
def validate_language_code(code: str) -> bool: ...
def validate_token_reduction_level(level: str) -> bool: ...
def validate_tesseract_psm(psm: int) -> bool: ...
def validate_tesseract_oem(oem: int) -> bool: ...
def validate_output_format(output_format: str) -> bool: ...
def validate_confidence(confidence: float) -> bool: ...
def validate_dpi(dpi: int) -> bool: ...
def validate_chunking_params(max_chars: int, max_overlap: int) -> bool: ...
def get_valid_binarization_methods() -> list[str]: ...
def get_valid_language_codes() -> list[str]: ...
def get_valid_ocr_backends() -> list[str]: ...
def get_valid_token_reduction_levels() -> list[str]: ...
def config_to_json(config: ExtractionConfig) -> str: ...
def config_get_field(config: ExtractionConfig, field_name: str) -> Any | None: ...
def config_merge(base: ExtractionConfig, override: ExtractionConfig) -> None: ...
def get_error_details() -> ErrorDetails: ...
def classify_error(message: str) -> int: ...
def error_code_name(code: int) -> str: ...
def _discover_extraction_config_impl() -> ExtractionConfig | None: ...
def _load_extraction_config_from_file_impl(path: str) -> ExtractionConfig: ...

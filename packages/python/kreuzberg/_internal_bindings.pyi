from collections.abc import Awaitable
from enum import StrEnum
from pathlib import Path
from typing import Any, Literal, Protocol, TypeAlias, TypedDict, overload

class OutputFormat(StrEnum):
    PLAIN = "plain"
    MARKDOWN = "markdown"
    DJOT = "djot"
    HTML = "html"

class ResultFormat(StrEnum):
    UNIFIED = "unified"
    ELEMENT_BASED = "element_based"

__all__ = [
    "ChunkingConfig",
    "EmbeddingConfig",
    "EmbeddingModelType",
    "EmbeddingPreset",
    "ExtractedTable",
    "ExtractionConfig",
    "ExtractionResult",
    "HierarchyConfig",
    "ImageExtractionConfig",
    "ImagePreprocessingConfig",
    "KeywordAlgorithm",
    "KeywordConfig",
    "LanguageDetectionConfig",
    "OcrBackendProtocol",
    "OcrConfig",
    "OcrResult",
    "PageConfig",
    "PageContent",
    "PdfConfig",
    "PostProcessorConfig",
    "PostProcessorProtocol",
    "RakeParams",
    "TesseractConfig",
    "TokenReductionConfig",
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

        html_options (dict[str, Any] | None): HTML conversion options for
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
    html_options: dict[str, Any] | None
    pages: PageConfig | None
    result_format: str
    output_format: str

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
        html_options: dict[str, Any] | None = None,
        pages: PageConfig | None = None,
        result_format: str | None = None,
        output_format: str | None = None,
    ) -> None: ...
    @staticmethod
    def from_file(path: str | Path) -> ExtractionConfig: ...

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

    def __init__(
        self,
        *,
        backend: str | None = None,
        language: str | None = None,
        tesseract_config: TesseractConfig | None = None,
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

    model: EmbeddingModelType
    normalize: bool
    batch_size: int
    show_download_progress: bool
    cache_dir: str | None

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
        List all available embedding presets:
            >>> from kreuzberg import list_embedding_presets, get_embedding_preset
            >>> presets = list_embedding_presets()
            >>> for preset_name in presets:
            ...     preset = get_embedding_preset(preset_name)
            ...     print(f"{preset.name}: {preset.description}")
            ...     print(f"  Dimensions: {preset.dimensions}")
            ...     print(f"  Chunk size: {preset.chunk_size}")

        Get a specific preset:
            >>> balanced = get_embedding_preset("balanced")
            >>> print(f"Model: {balanced.model_name}")
            >>> print(f"Dimensions: {balanced.dimensions}")
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

    def __init__(
        self,
        *,
        max_chars: int | None = None,
        max_overlap: int | None = None,
        embedding: EmbeddingConfig | None = None,
        preset: str | None = None,
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

    def __init__(
        self,
        *,
        extract_images: bool | None = None,
        passwords: list[str] | None = None,
        extract_metadata: bool | None = None,
        hierarchy: HierarchyConfig | None = None,
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
        mode: Literal["off", "moderate", "aggressive"] | None = None,
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
            Default: False

        deskew (bool): Correct skewed images to improve OCR accuracy.
            Default: False

        denoise (bool): Apply denoising filters to reduce noise in images.
            Improves OCR accuracy on low-quality scans. Default: False

        contrast_enhance (bool): Enhance contrast to improve text readability.
            Default: False

        binarization_method (str): Method for converting images to black and white.
            Options depend on the OCR backend. Default: "auto"

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
            Default: "plaintext"

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
            Default: False

        table_min_confidence (float): Minimum confidence for table detection (0.0-1.0).
            Default: 0.5

        table_column_threshold (int): Minimum pixel width between columns.
            Default: 10

        table_row_threshold_ratio (float): Minimum row height ratio.
            Default: 0.5

        use_cache (bool): Cache OCR results for improved performance.
            Default: True

        classify_use_pre_adapted_templates (bool): Use pre-adapted character templates.
            Default: False

        language_model_ngram_on (bool): Enable language model n-gram processing.
            Default: True

        tessedit_dont_blkrej_good_wds (bool): Don't block-reject good words.
            Default: False

        tessedit_dont_rowrej_good_wds (bool): Don't row-reject good words.
            Default: False

        tessedit_enable_dict_correction (bool): Enable dictionary-based spelling correction.
            Default: False

        tessedit_char_whitelist (str): Whitelist of characters to recognize.
            Only these characters will be recognized. Empty = all characters.
            Default: ""

        tessedit_char_blacklist (str): Blacklist of characters to ignore.
            These characters will be skipped. Default: ""

        tessedit_use_primary_params_model (bool): Use primary parameters model.
            Default: False

        textord_space_size_is_variable (bool): Allow variable space sizes.
            Default: False

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
    title: str
    subject: str
    authors: list[str]
    keywords: list[str]
    created_at: str
    modified_at: str
    created_by: str
    producer: str
    page_count: int
    pdf_version: str
    is_encrypted: bool
    width: int
    height: int
    summary: str

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
    title: str
    author: str
    description: str
    summary: str
    fonts: list[str]

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
    headers: list[str]
    links: list[tuple[str, str]]
    code_blocks: list[tuple[str, str]]

class HtmlMetadata(TypedDict, total=False):
    title: str
    description: str
    keywords: str
    author: str
    canonical: str
    base_href: str
    og_title: str
    og_description: str
    og_image: str
    og_url: str
    og_type: str
    og_site_name: str
    twitter_card: str
    twitter_title: str
    twitter_description: str
    twitter_image: str
    twitter_site: str
    twitter_creator: str
    link_author: str
    link_license: str
    link_alternate: str

class OcrMetadata(TypedDict, total=False):
    language: str
    psm: int
    output_format: str
    table_count: int
    table_rows: int
    table_cols: int

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

class Metadata(TypedDict, total=False):
    language: str
    date: str
    subject: str

    format_type: Literal["pdf", "excel", "email", "pptx", "archive", "image", "xml", "text", "html", "ocr"]

    title: str
    authors: list[str]
    keywords: list[str]
    created_at: str
    modified_at: str
    created_by: str
    producer: str
    page_count: int
    pdf_version: str
    is_encrypted: bool
    width: int
    height: int
    summary: str

    sheet_count: int
    sheet_names: list[str]

    from_email: str
    from_name: str
    to_emails: list[str]
    cc_emails: list[str]
    bcc_emails: list[str]
    message_id: str
    attachments: list[str]

    author: str
    description: str
    fonts: list[str]

    format: str
    file_count: int
    file_list: list[str]
    total_size: int
    compressed_size: int

    exif: dict[str, str]

    element_count: int
    unique_elements: list[str]

    line_count: int
    word_count: int
    character_count: int
    headers: list[str]
    links: list[tuple[str, str]]
    code_blocks: list[tuple[str, str]]

    canonical: str
    base_href: str
    og_title: str
    og_description: str
    og_image: str
    og_url: str
    og_type: str
    og_site_name: str
    twitter_card: str
    twitter_title: str
    twitter_description: str
    twitter_image: str
    twitter_site: str
    twitter_creator: str
    link_author: str
    link_license: str
    link_alternate: str

    psm: int
    output_format: str
    table_count: int
    table_rows: int
    table_cols: int

    image_preprocessing: ImagePreprocessingMetadata
    json_schema: dict[str, Any]
    error: ErrorMetadata

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
    ocr_result: ExtractionResult

class Chunk(TypedDict, total=False):
    content: str
    embedding: list[float] | None
    metadata: dict[str, Any]

class DjotContent(TypedDict, total=False):
    plain_text: str
    blocks: list[FormattedBlock]
    metadata: Metadata
    tables: list[ExtractedTable]
    images: list[DjotImage]
    links: list[DjotLink]
    footnotes: list[Footnote]
    attributes: dict[str, Any]

class FormattedBlock(TypedDict, total=False):
    block_type: str
    level: int | None
    content: str | None
    children: list[FormattedBlock] | None
    attributes: dict[str, Any] | None

class DjotImage(TypedDict, total=False):
    url: str
    alt: str | None
    title: str | None
    attributes: dict[str, Any] | None

class DjotLink(TypedDict, total=False):
    url: str
    text: str
    title: str | None
    link_type: str | None

class Footnote(TypedDict, total=False):
    label: str
    content: str

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

class HierarchicalBlock(TypedDict, total=False):
    text: str
    font_size: float
    level: str
    bbox: tuple[float, float, float, float] | None

class PageHierarchy(TypedDict, total=False):
    block_count: int
    blocks: list[HierarchicalBlock]

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
    djot_content: DjotContent | None
    def get_page_count(self) -> int: ...
    def get_chunk_count(self) -> int: ...
    def get_detected_language(self) -> str | None: ...
    def get_metadata_field(self, field_name: str) -> Any | None: ...

class PageContent(TypedDict):
    page_number: int
    content: str
    tables: list[ExtractedTable]
    images: list[ExtractedImage]
    hierarchy: PageHierarchy | None

class ExtractedTable:
    cells: list[list[str]]
    markdown: str
    page_number: int

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
) -> list[ExtractionResult]: ...
def batch_extract_bytes_sync(
    data_list: list[bytes | bytearray],
    mime_types: list[str],
    config: ExtractionConfig = ...,
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
) -> Awaitable[list[ExtractionResult]]: ...
def batch_extract_bytes(
    data_list: list[bytes | bytearray],
    mime_types: list[str],
    config: ExtractionConfig = ...,
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
def get_last_error_code() -> int | None: ...
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
def get_error_details() -> dict[str, Any]: ...
def classify_error(message: str) -> int: ...
def error_code_name(code: int) -> str: ...
def _discover_extraction_config_impl() -> ExtractionConfig | None: ...
def _load_extraction_config_from_file_impl(path: str) -> ExtractionConfig: ...

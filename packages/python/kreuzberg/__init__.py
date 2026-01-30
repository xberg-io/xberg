"""Kreuzberg - Multi-language document intelligence framework.

This is a thin Python wrapper around a high-performance Rust core.
All extraction logic, chunking, quality processing, and language detection
are implemented in Rust for maximum performance.

Python-specific features:
- OCR backends: EasyOCR, PaddleOCR (Python-based OCR engines)
- Custom PostProcessors: Register your own Python processing logic

Architecture:
- Rust handles: Extraction, parsing, chunking, quality, language detection, NLP (keyword extraction), API server, MCP server, CLI
- Python adds: OCR backends (EasyOCR, PaddleOCR), custom postprocessors

Creating Custom PostProcessors:
    >>> from kreuzberg import PostProcessorProtocol, register_post_processor, ExtractionResult
    >>>
    >>> class MyProcessor:
    ...     def name(self) -> str:
    ...         return "my_processor"
    ...
    ...     def process(self, result: ExtractionResult) -> ExtractionResult:
    ...         result.metadata["custom_field"] = "custom_value"
    ...         return result
    ...
    ...     def processing_stage(self) -> str:
    ...         return "middle"
    >>>
    >>> register_post_processor(MyProcessor())
"""

from __future__ import annotations

import hashlib
import json
import threading
from importlib.metadata import version
from typing import TYPE_CHECKING, Any

# ~keep: This must be imported FIRST before any Rust bindings
# ~keep: It sets up dynamic library paths for bundled native libraries (pdfium, etc.)
from kreuzberg import _setup_lib_path  # noqa: F401
from kreuzberg._deprecation import deprecated
from kreuzberg._internal_bindings import (
    ChunkingConfig,
    EmbeddingConfig,
    EmbeddingModelType,
    EmbeddingPreset,
    ExtractedTable,
    ExtractionConfig,
    ExtractionResult,
    HierarchyConfig,
    ImageExtractionConfig,
    ImagePreprocessingConfig,
    KeywordAlgorithm,
    KeywordConfig,
    LanguageDetectionConfig,
    OcrConfig,
    PageConfig,
    PdfConfig,
    PostProcessorConfig,
    RakeParams,
    TesseractConfig,
    TokenReductionConfig,
    YakeParams,
    _discover_extraction_config_impl,
    _load_extraction_config_from_file_impl,
    clear_document_extractors,
    clear_ocr_backends,
    clear_post_processors,
    clear_validators,
    config_get_field,
    config_merge,
    config_to_json,
    detect_mime_type_from_bytes,
    get_embedding_preset,
    get_extensions_for_mime,
    get_last_panic_context,
    get_valid_binarization_methods,
    get_valid_language_codes,
    get_valid_ocr_backends,
    get_valid_token_reduction_levels,
    list_document_extractors,
    list_embedding_presets,
    list_ocr_backends,
    list_post_processors,
    list_validators,
    unregister_document_extractor,
    unregister_ocr_backend,
    unregister_post_processor,
    unregister_validator,
    validate_binarization_method,
    validate_chunking_params,
    validate_confidence,
    validate_dpi,
    validate_language_code,
    validate_mime_type,
    validate_ocr_backend,
    validate_output_format,
    validate_tesseract_oem,
    validate_tesseract_psm,
    validate_token_reduction_level,
)
from kreuzberg._internal_bindings import (
    batch_extract_bytes as batch_extract_bytes_impl,
)
from kreuzberg._internal_bindings import (
    batch_extract_bytes_sync as batch_extract_bytes_sync_impl,
)
from kreuzberg._internal_bindings import (
    batch_extract_files as batch_extract_files_impl,
)
from kreuzberg._internal_bindings import (
    batch_extract_files_sync as batch_extract_files_sync_impl,
)
from kreuzberg._internal_bindings import (
    classify_error as _classify_error_impl,
)
from kreuzberg._internal_bindings import (
    detect_mime_type_from_path as _detect_mime_type_from_path_impl,
)
from kreuzberg._internal_bindings import (
    error_code_name as _error_code_name_impl,
)
from kreuzberg._internal_bindings import (
    extract_bytes as extract_bytes_impl,
)
from kreuzberg._internal_bindings import (
    extract_bytes_sync as extract_bytes_sync_impl,
)
from kreuzberg._internal_bindings import (
    extract_file as extract_file_impl,
)
from kreuzberg._internal_bindings import (
    extract_file_sync as extract_file_sync_impl,
)
from kreuzberg._internal_bindings import (
    get_error_details as _get_error_details_impl,
)
from kreuzberg._internal_bindings import (
    get_last_error_code as _get_last_error_code_impl,
)
from kreuzberg._internal_bindings import (
    register_ocr_backend as _register_ocr_backend_impl,
)
from kreuzberg._internal_bindings import (
    register_post_processor as _register_post_processor_impl,
)
from kreuzberg._internal_bindings import (
    register_validator as _register_validator_impl,
)
from kreuzberg.exceptions import (
    CacheError,
    ErrorCode,
    ImageProcessingError,
    KreuzbergError,
    MissingDependencyError,
    OCRError,
    PanicContext,
    ParsingError,
    PluginError,
    ValidationError,
)
from kreuzberg.postprocessors.protocol import PostProcessorProtocol
from kreuzberg.types import Chunk, ChunkMetadata, ExtractedImage, Metadata, OutputFormat, ResultFormat

if TYPE_CHECKING:
    from pathlib import Path

    from kreuzberg.ocr.easyocr import EasyOCRBackend  # noqa: F401
    from kreuzberg.ocr.paddleocr import PaddleOCRBackend  # noqa: F401

__version__ = version("kreuzberg")

__all__ = [
    "CacheError",
    "Chunk",
    "ChunkMetadata",
    "ChunkingConfig",
    "EmbeddingConfig",
    "EmbeddingModelType",
    "EmbeddingPreset",
    "ErrorCode",
    "ExtractedImage",
    "ExtractedTable",
    "ExtractionConfig",
    "ExtractionResult",
    "HierarchyConfig",
    "ImageExtractionConfig",
    "ImagePreprocessingConfig",
    "ImageProcessingError",
    "KeywordAlgorithm",
    "KeywordConfig",
    "KreuzbergError",
    "LanguageDetectionConfig",
    "Metadata",
    "MissingDependencyError",
    "OCRError",
    "OcrConfig",
    "OutputFormat",
    "PageConfig",
    "PanicContext",
    "ParsingError",
    "PdfConfig",
    "PluginError",
    "PostProcessorConfig",
    "PostProcessorProtocol",
    "RakeParams",
    "ResultFormat",
    "TesseractConfig",
    "TokenReductionConfig",
    "ValidationError",
    "YakeParams",
    "__version__",
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
    "deprecated",
    "detect_mime_type",
    "detect_mime_type_from_path",
    "discover_extraction_config",
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
    "load_extraction_config_from_file",
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


_REGISTERED_OCR_BACKENDS: dict[tuple[str, str], Any] = {}

_OCR_CACHE_LOCK = threading.Lock()

_MAX_CACHE_SIZE = 10


def _hash_kwargs(kwargs: dict[str, Any]) -> str:
    try:
        serialized = json.dumps(kwargs, sort_keys=True, default=str)
        return hashlib.md5(serialized.encode()).hexdigest()  # noqa: S324
    except (TypeError, ValueError):
        return hashlib.md5(repr(kwargs).encode()).hexdigest()  # noqa: S324


def _ensure_ocr_backend_registered(
    config: ExtractionConfig,
    easyocr_kwargs: dict[str, Any] | None,
    paddleocr_kwargs: dict[str, Any] | None,
) -> None:
    if config.ocr is None:
        return

    backend_name = config.ocr.backend

    if backend_name == "tesseract":
        return

    kwargs_map = {
        "easyocr": easyocr_kwargs or {},
        "paddleocr": paddleocr_kwargs or {},
    }
    kwargs = kwargs_map.get(backend_name, {})

    with _OCR_CACHE_LOCK:
        cache_key = (backend_name, _hash_kwargs(kwargs))

        if cache_key in _REGISTERED_OCR_BACKENDS:
            return

        if len(_REGISTERED_OCR_BACKENDS) >= _MAX_CACHE_SIZE:
            oldest_key = next(iter(_REGISTERED_OCR_BACKENDS))
            del _REGISTERED_OCR_BACKENDS[oldest_key]

        backend: Any
        if backend_name == "easyocr":
            try:
                from kreuzberg.ocr.easyocr import EasyOCRBackend  # noqa: PLC0415

                if "languages" not in kwargs:
                    kwargs["languages"] = [config.ocr.language]

                backend = EasyOCRBackend(**kwargs)
            except ImportError as e:
                raise MissingDependencyError.create_for_package(
                    dependency_group="easyocr",
                    functionality="EasyOCR backend",
                    package_name="easyocr",
                ) from e
        elif backend_name == "paddleocr":
            try:
                from kreuzberg.ocr.paddleocr import PaddleOCRBackend  # noqa: PLC0415

                if "lang" not in kwargs:
                    kwargs["lang"] = config.ocr.language

                backend = PaddleOCRBackend(**kwargs)
            except ImportError as e:
                raise MissingDependencyError.create_for_package(
                    dependency_group="paddleocr",
                    functionality="PaddleOCR backend",
                    package_name="paddleocr",
                ) from e
        else:
            return

        register_ocr_backend(backend)
        _REGISTERED_OCR_BACKENDS[cache_key] = backend


def extract_file_sync(
    file_path: str | Path,
    mime_type: str | None = None,
    config: ExtractionConfig | None = None,
    *,
    easyocr_kwargs: dict[str, Any] | None = None,
    paddleocr_kwargs: dict[str, Any] | None = None,
) -> ExtractionResult:
    """Extract content from a file (synchronous).

    Args:
        file_path: Path to the file (str or pathlib.Path)
        mime_type: Optional MIME type hint (auto-detected if None)
        config: Extraction configuration (uses defaults if None)
        easyocr_kwargs: EasyOCR initialization options (languages, use_gpu, beam_width, etc.)
        paddleocr_kwargs: PaddleOCR initialization options (lang, use_angle_cls, show_log, etc.)

    Returns:
        ExtractionResult with content, metadata, and tables

    Example:
        >>> from kreuzberg import extract_file_sync, ExtractionConfig, OcrConfig, TesseractConfig
        >>> # Basic usage
        >>> result = extract_file_sync("document.pdf")
        >>>
        >>> # With Tesseract configuration
        >>> config = ExtractionConfig(
        ...     ocr=OcrConfig(
        ...         backend="tesseract",
        ...         language="eng",
        ...         tesseract_config=TesseractConfig(
        ...             psm=6,
        ...             enable_table_detection=True,
        ...             tessedit_char_whitelist="0123456789",
        ...         ),
        ...     )
        ... )
        >>> result = extract_file_sync("invoice.pdf", config=config)
        >>>
        >>> # With EasyOCR custom options
        >>> config = ExtractionConfig(ocr=OcrConfig(backend="easyocr", language="eng"))
        >>> result = extract_file_sync("scanned.pdf", config=config, easyocr_kwargs={"use_gpu": True, "beam_width": 10})
    """
    if config is None:
        config = ExtractionConfig()

    _ensure_ocr_backend_registered(config, easyocr_kwargs, paddleocr_kwargs)

    return extract_file_sync_impl(str(file_path), mime_type, config)


def extract_bytes_sync(
    data: bytes | bytearray,
    mime_type: str,
    config: ExtractionConfig | None = None,
    *,
    easyocr_kwargs: dict[str, Any] | None = None,
    paddleocr_kwargs: dict[str, Any] | None = None,
) -> ExtractionResult:
    """Extract content from bytes (synchronous).

    Args:
        data: File content as bytes or bytearray
        mime_type: MIME type of the data (required for format detection)
        config: Extraction configuration (uses defaults if None)
        easyocr_kwargs: EasyOCR initialization options
        paddleocr_kwargs: PaddleOCR initialization options

    Returns:
        ExtractionResult with content, metadata, and tables
    """
    if config is None:
        config = ExtractionConfig()

    _ensure_ocr_backend_registered(config, easyocr_kwargs, paddleocr_kwargs)

    return extract_bytes_sync_impl(bytes(data), mime_type, config)


def batch_extract_files_sync(
    paths: list[str | Path],
    config: ExtractionConfig | None = None,
    *,
    easyocr_kwargs: dict[str, Any] | None = None,
    paddleocr_kwargs: dict[str, Any] | None = None,
) -> list[ExtractionResult]:
    """Extract content from multiple files in parallel (synchronous).

    Args:
        paths: List of file paths
        config: Extraction configuration (uses defaults if None)
        easyocr_kwargs: EasyOCR initialization options
        paddleocr_kwargs: PaddleOCR initialization options

    Returns:
        List of ExtractionResults (one per file)
    """
    if config is None:
        config = ExtractionConfig()

    _ensure_ocr_backend_registered(config, easyocr_kwargs, paddleocr_kwargs)

    return batch_extract_files_sync_impl([str(p) for p in paths], config)


def batch_extract_bytes_sync(
    data_list: list[bytes | bytearray],
    mime_types: list[str],
    config: ExtractionConfig | None = None,
    *,
    easyocr_kwargs: dict[str, Any] | None = None,
    paddleocr_kwargs: dict[str, Any] | None = None,
) -> list[ExtractionResult]:
    """Extract content from multiple byte arrays in parallel (synchronous).

    Args:
        data_list: List of file contents as bytes/bytearray
        mime_types: List of MIME types (one per data item)
        config: Extraction configuration (uses defaults if None)
        easyocr_kwargs: EasyOCR initialization options
        paddleocr_kwargs: PaddleOCR initialization options

    Returns:
        List of ExtractionResults (one per data item)
    """
    if config is None:
        config = ExtractionConfig()

    _ensure_ocr_backend_registered(config, easyocr_kwargs, paddleocr_kwargs)

    return batch_extract_bytes_sync_impl([bytes(d) for d in data_list], mime_types, config)


async def extract_file(
    file_path: str | Path,
    mime_type: str | None = None,
    config: ExtractionConfig | None = None,
    *,
    easyocr_kwargs: dict[str, Any] | None = None,
    paddleocr_kwargs: dict[str, Any] | None = None,
) -> ExtractionResult:
    """Extract content from a file (asynchronous).

    Args:
        file_path: Path to the file (str or pathlib.Path)
        mime_type: Optional MIME type hint (auto-detected if None)
        config: Extraction configuration (uses defaults if None)
        easyocr_kwargs: EasyOCR initialization options
        paddleocr_kwargs: PaddleOCR initialization options

    Returns:
        ExtractionResult with content, metadata, and tables
    """
    if config is None:
        config = ExtractionConfig()

    _ensure_ocr_backend_registered(config, easyocr_kwargs, paddleocr_kwargs)

    return await extract_file_impl(str(file_path), mime_type, config)


async def extract_bytes(
    data: bytes | bytearray,
    mime_type: str,
    config: ExtractionConfig | None = None,
    *,
    easyocr_kwargs: dict[str, Any] | None = None,
    paddleocr_kwargs: dict[str, Any] | None = None,
) -> ExtractionResult:
    """Extract content from bytes (asynchronous).

    Args:
        data: File content as bytes or bytearray
        mime_type: MIME type of the data (required for format detection)
        config: Extraction configuration (uses defaults if None)
        easyocr_kwargs: EasyOCR initialization options
        paddleocr_kwargs: PaddleOCR initialization options

    Returns:
        ExtractionResult with content, metadata, and tables
    """
    if config is None:
        config = ExtractionConfig()

    _ensure_ocr_backend_registered(config, easyocr_kwargs, paddleocr_kwargs)

    return await extract_bytes_impl(bytes(data), mime_type, config)


async def batch_extract_files(
    paths: list[str | Path],
    config: ExtractionConfig | None = None,
    *,
    easyocr_kwargs: dict[str, Any] | None = None,
    paddleocr_kwargs: dict[str, Any] | None = None,
) -> list[ExtractionResult]:
    """Extract content from multiple files in parallel (asynchronous).

    Args:
        paths: List of file paths
        config: Extraction configuration (uses defaults if None)
        easyocr_kwargs: EasyOCR initialization options
        paddleocr_kwargs: PaddleOCR initialization options

    Returns:
        List of ExtractionResults (one per file)
    """
    if config is None:
        config = ExtractionConfig()

    _ensure_ocr_backend_registered(config, easyocr_kwargs, paddleocr_kwargs)

    return await batch_extract_files_impl([str(p) for p in paths], config)


async def batch_extract_bytes(
    data_list: list[bytes | bytearray],
    mime_types: list[str],
    config: ExtractionConfig | None = None,
    *,
    easyocr_kwargs: dict[str, Any] | None = None,
    paddleocr_kwargs: dict[str, Any] | None = None,
) -> list[ExtractionResult]:
    """Extract content from multiple byte arrays in parallel (asynchronous).

    Args:
        data_list: List of file contents as bytes/bytearray
        mime_types: List of MIME types (one per data item)
        config: Extraction configuration (uses defaults if None)
        easyocr_kwargs: EasyOCR initialization options
        paddleocr_kwargs: PaddleOCR initialization options

    Returns:
        List of ExtractionResults (one per data item)
    """
    if config is None:
        config = ExtractionConfig()

    _ensure_ocr_backend_registered(config, easyocr_kwargs, paddleocr_kwargs)

    return await batch_extract_bytes_impl([bytes(d) for d in data_list], mime_types, config)


def detect_mime_type(data: bytes | bytearray) -> str:
    r"""Detect MIME type from file bytes.

    Args:
        data: File content as bytes or bytearray

    Returns:
        Detected MIME type (e.g., "application/pdf", "image/png")

    Example:
        >>> from kreuzberg import detect_mime_type
        >>> pdf_bytes = b"%PDF-1.4\\n"
        >>> mime_type = detect_mime_type(pdf_bytes)
        >>> assert "pdf" in mime_type.lower()
    """
    return detect_mime_type_from_bytes(bytes(data))


def detect_mime_type_from_path(path: str | Path) -> str:
    """Detect MIME type from file path.

    Reads the file at the given path and detects its MIME type using magic number detection.

    Args:
        path: Path to the file (str or pathlib.Path)

    Returns:
        Detected MIME type (e.g., "application/pdf", "text/plain")

    Raises:
        OSError: If file cannot be read (file not found, permission denied, etc.)
        RuntimeError: If MIME type detection fails

    Example:
        >>> from kreuzberg import detect_mime_type_from_path
        >>> mime_type = detect_mime_type_from_path("document.pdf")
        >>> assert "pdf" in mime_type.lower()
    """
    return _detect_mime_type_from_path_impl(str(path))


def discover_extraction_config() -> ExtractionConfig | None:
    """Discover extraction configuration from the environment.

    .. deprecated:: 4.2.0
        Consider using :func:`load_extraction_config_from_file` with explicit
        configuration paths for more predictable behavior.

    Attempts to locate a Kreuzberg configuration file using the following strategy:
    1. If KREUZBERG_CONFIG_PATH environment variable is set, load from that path
    2. Otherwise, search for kreuzberg.toml, kreuzberg.yaml, or kreuzberg.json
       in the current directory and parent directories (walking up the tree)
    3. Return None if no configuration file is found

    The search order for auto-discovery (when env var is not set):
    - kreuzberg.toml (highest priority)
    - kreuzberg.yaml
    - kreuzberg.json (lowest priority)

    Returns:
        ExtractionConfig if a configuration file is found and valid, None otherwise

    Raises:
        RuntimeError: If the discovered config file is invalid or cannot be parsed
        IOError: If there's an error reading the config file

    Example:
        >>> from kreuzberg import discover_extraction_config
        >>> config = discover_extraction_config()
        >>> if config:
        ...     print(f"Loaded config with use_cache={config.use_cache}")
        ... else:
        ...     print("No config found, using defaults")
    """
    return _discover_extraction_config_impl()


def load_extraction_config_from_file(path: str | Path) -> ExtractionConfig:
    """Load extraction configuration from a specific file.

    Loads an ExtractionConfig from the specified file path. The file format
    is determined by the file extension (.toml, .yaml, or .json).

    Args:
        path: Path to the configuration file (str or pathlib.Path).
              Supports absolute and relative paths.

    Returns:
        ExtractionConfig parsed from the file

    Raises:
        FileNotFoundError: If the configuration file does not exist
        RuntimeError: If the file cannot be read or parsed
        ValueError: If the file format is invalid or unsupported

    Example:
        >>> from kreuzberg import load_extraction_config_from_file
        >>> config = load_extraction_config_from_file("kreuzberg.toml")
        >>> result = extract_file_sync("document.pdf", config=config)
    """
    return _load_extraction_config_from_file_impl(str(path))


def register_ocr_backend(backend: Any) -> None:
    """Register a Python OCR backend with the Rust core.

    This function validates the Python backend object, wraps it in a Rust OcrBackend
    implementation, and registers it with the global OCR backend registry. Once registered,
    the backend can be used by the Rust CLI, API, and MCP server.

    Args:
        backend: Python object implementing the OCR backend protocol

    Required methods on the backend object:
        - name() -> str: Return backend name (must be non-empty)
        - supported_languages() -> list[str]: Return list of supported language codes
        - process_image(image_bytes: bytes, language: str) -> dict: Process image and return result dict

    Optional methods:
        - process_file(path: str, language: str) -> dict: Custom file processing
        - initialize(): Called when backend is registered
        - shutdown(): Called when backend is unregistered
        - version() -> str: Backend version (defaults to "1.0.0")

    Raises:
        TypeError: If backend is missing required methods (name, supported_languages, process_image)
        ValueError: If backend name is empty or already registered
        RuntimeError: If registration with the Rust registry fails

    Example:
        >>> from kreuzberg import register_ocr_backend
        >>> class MyOcrBackend:
        ...     def name(self) -> str:
        ...         return "my-ocr"
        ...
        ...     def supported_languages(self) -> list[str]:
        ...         return ["eng", "deu", "fra"]
        ...
        ...     def process_image(self, image_bytes: bytes, language: str) -> dict:
        ...         return {"content": "extracted text", "metadata": {"confidence": 0.95}, "tables": []}
        >>> register_ocr_backend(MyOcrBackend())
    """
    return _register_ocr_backend_impl(backend)


def register_post_processor(processor: Any) -> None:
    """Register a Python PostProcessor with the Rust core.

    This function validates the Python processor object, wraps it in a Rust PostProcessor
    implementation, and registers it with the global PostProcessor registry. Once registered,
    the processor will be called automatically after extraction to enrich results.

    Args:
        processor: Python object implementing the PostProcessor protocol

    Required methods on the processor object:
        - name() -> str: Return processor name (must be non-empty)
        - process(result: dict) -> dict: Process and enrich the extraction result
        - processing_stage() -> str: Return "early", "middle", or "late" (REQUIRED, not optional)

    Optional methods:
        - initialize(): Called when processor is registered
        - shutdown(): Called when processor is unregistered
        - version() -> str: Processor version (defaults to "1.0.0")

    Raises:
        TypeError: If processor is missing required methods (name, process, processing_stage)
        ValueError: If processor name is empty or already registered
        RuntimeError: If registration with the Rust registry fails

    Example:
        >>> from kreuzberg import register_post_processor
        >>> class EntityExtractor:
        ...     def name(self) -> str:
        ...         return "entity_extraction"
        ...
        ...     def processing_stage(self) -> str:
        ...         return "early"
        ...
        ...     def process(self, result: dict) -> dict:
        ...         entities = {"PERSON": ["John Doe"], "ORG": ["Microsoft"]}
        ...         result["metadata"]["entities"] = entities
        ...         return result
        >>> register_post_processor(EntityExtractor())
    """
    return _register_post_processor_impl(processor)


def register_validator(validator: Any) -> None:
    """Register a Python Validator with the Rust core.

    This function validates the Python validator object, wraps it in a Rust Validator
    implementation, and registers it with the global Validator registry. Once registered,
    the validator will be called automatically after extraction to validate results.

    Args:
        validator: Python object implementing the Validator protocol

    Required methods on the validator object:
        - name() -> str: Return validator name (must be non-empty)
        - validate(result: dict) -> None: Validate the extraction result (raise error to fail)

    Optional methods:
        - should_validate(result: dict) -> bool: Check if validator should run (defaults to True)
        - priority() -> int: Return priority (defaults to 50, higher runs first)
        - initialize(): Called when validator is registered
        - shutdown(): Called when validator is unregistered
        - version() -> str: Validator version (defaults to "1.0.0")

    Raises:
        TypeError: If validator is missing required methods (name, validate)
        ValueError: If validator name is empty or already registered
        RuntimeError: If registration with the Rust registry fails

    Example:
        >>> from kreuzberg import register_validator
        >>> from kreuzberg.exceptions import ValidationError
        >>> class MinLengthValidator:
        ...     def name(self) -> str:
        ...         return "min_length_validator"
        ...
        ...     def priority(self) -> int:
        ...         return 100
        ...
        ...     def validate(self, result: dict) -> None:
        ...         if len(result["content"]) < 100:
        ...             raise ValidationError(f"Content too short")
        >>> register_validator(MinLengthValidator())
    """
    return _register_validator_impl(validator)


def get_last_error_code() -> int | None:
    """Get the last error code from the FFI layer.

    Returns the error code from the most recent operation. Useful for debugging
    and understanding what went wrong when an operation fails.

    Error codes:
        - 0 (SUCCESS): No error occurred
        - 1 (GENERIC_ERROR): Generic unspecified error
        - 2 (PANIC): A panic occurred in the Rust core
        - 3 (INVALID_ARGUMENT): Invalid argument provided
        - 4 (IO_ERROR): I/O operation failed
        - 5 (PARSING_ERROR): Document parsing failed
        - 6 (OCR_ERROR): OCR operation failed
        - 7 (MISSING_DEPENDENCY): Required dependency not available

    Returns:
        int: The error code (0 if no error has occurred)

    Example:
        >>> from kreuzberg import get_last_error_code, ErrorCode
        >>> code = get_last_error_code()
        >>> if code == ErrorCode.SUCCESS:
        ...     print("No errors")
        >>> elif code == ErrorCode.OCR_ERROR:
        ...     print("OCR operation failed")
        >>> elif code == 2:
        ...     print("A panic occurred")
    """
    return _get_last_error_code_impl()


def get_error_details() -> dict[str, Any]:
    """Get detailed error information from the FFI layer.

    Retrieves structured error information from the thread-local error storage
    in the FFI layer. Returns comprehensive details about the most recent error
    including message, code, type, and source location if available.

    Returns:
        dict: Structured error details with keys:
            - "message" (str): Human-readable error message
            - "error_code" (int): Numeric error code (0-7)
            - "error_type" (str): Error type name (e.g., "validation", "ocr")
            - "source_file" (str | None): Source file path if available
            - "source_function" (str | None): Function name if available
            - "source_line" (int): Line number (0 if unknown)
            - "context_info" (str | None): Additional context if available
            - "is_panic" (bool): Whether error came from a panic

    Example:
        >>> from kreuzberg import get_error_details
        >>> details = get_error_details()
        >>> print(f"Error: {details['message']} (code={details['error_code']})")
        >>> if details["source_file"]:
        ...     print(f"  at {details['source_file']}:{details['source_line']}")
    """
    return _get_error_details_impl()


def classify_error(message: str) -> int:
    """Classify an error message into a Kreuzberg error code.

    Analyzes an error message and returns the most likely Kreuzberg error code
    (0-7). Useful for categorizing error messages from external libraries or
    system calls into standard Kreuzberg error categories.

    Args:
        message: The error message to classify

    Returns:
        int: Error code (0-7) representing the classification:
            - 0 (Validation): Invalid parameters, constraints, format mismatches
            - 1 (Parsing): Parse errors, corrupt data, malformed content
            - 2 (OCR): OCR processing failures
            - 3 (MissingDependency): Missing libraries or system dependencies
            - 4 (Io): File I/O, permissions, disk errors
            - 5 (Plugin): Plugin loading or registry errors
            - 6 (UnsupportedFormat): Unsupported MIME types or formats
            - 7 (Internal): Unknown or internal errors

    Example:
        >>> from kreuzberg import classify_error
        >>> code = classify_error("Failed to open file: permission denied")
        >>> if code == 4:
        ...     print("This is an I/O error")
        >>> code = classify_error("OCR processing failed")
        >>> if code == 2:
        ...     print("This is an OCR error")
    """
    return _classify_error_impl(message)


def error_code_name(code: int) -> str:
    """Get the human-readable name of an error code.

    Args:
        code: Numeric error code (0-7)

    Returns:
        str: Human-readable error code name (e.g., "validation", "ocr")
             Returns "unknown" for codes outside the valid range.

    Example:
        >>> from kreuzberg import error_code_name
        >>> name = error_code_name(0)
        >>> print(name)  # output: "validation"
        >>> name = error_code_name(2)
        >>> print(name)  # output: "ocr"
        >>> name = error_code_name(99)
        >>> print(name)  # output: "unknown"
    """
    return _error_code_name_impl(code)

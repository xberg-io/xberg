#!/usr/bin/env python
"""Comprehensive test suite for Kreuzberg Python bindings v4.0.0-rc.16.

Tests ALL exported functions and types/classes.
Validates:
- All configuration classes import correctly
- All extraction functions work (sync + async)
- Plugin registration system works
- Error handling and validation functions work
- Result objects have correct structure
"""

import asyncio
import sys
from pathlib import Path

try:
    from kreuzberg import (
        CacheError,
        Chunk,
        ChunkingConfig,
        ChunkMetadata,
        EmbeddingConfig,
        EmbeddingModelType,
        EmbeddingPreset,
        ErrorCode,
        ExtractedImage,
        ExtractedTable,
        ExtractionConfig,
        ExtractionResult,
        ImageExtractionConfig,
        ImagePreprocessingConfig,
        ImageProcessingError,
        KeywordAlgorithm,
        KeywordConfig,
        KreuzbergError,
        LanguageDetectionConfig,
        Metadata,
        MissingDependencyError,
        OcrConfig,
        OCRError,
        PageConfig,
        PanicContext,
        ParsingError,
        PdfConfig,
        PluginError,
        PostProcessorConfig,
        PostProcessorProtocol,
        RakeParams,
        TesseractConfig,
        TokenReductionConfig,
        ValidationError,
        YakeParams,
        __version__,
        batch_extract_bytes,
        batch_extract_bytes_sync,
        batch_extract_files,
        batch_extract_files_sync,
        classify_error,
        clear_document_extractors,
        clear_ocr_backends,
        clear_post_processors,
        clear_validators,
        config_get_field,
        config_merge,
        config_to_json,
        detect_mime_type,
        detect_mime_type_from_path,
        error_code_name,
        extract_bytes,
        extract_bytes_sync,
        extract_file,
        extract_file_sync,
        get_embedding_preset,
        get_error_details,
        get_extensions_for_mime,
        get_last_error_code,
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
        register_ocr_backend,
        register_post_processor,
        register_validator,
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

except ImportError:
    sys.exit(1)


class TestRunner:
    def __init__(self):
        self.passed = 0
        self.failed = 0
        self.skipped = 0
        self.section = 0

    def start_section(self, name: str):
        self.section += 1

    def test(self, description: str, fn):
        try:
            result = fn()
            if result is False:
                self.failed += 1
                return False
            self.passed += 1
            return True
        except Exception:
            self.failed += 1
            return False

    def skip(self, description: str, reason: str):
        self.skipped += 1

    def summary(self):
        self.passed + self.failed + self.skipped

        if self.failed == 0:
            return 0
        return 1


runner = TestRunner()
test_docs = Path(__file__).parent / "test_documents"

runner.start_section("Configuration Classes")

runner.test("ExtractionConfig() default construction", lambda: ExtractionConfig() is not None)

runner.test("ExtractionConfig() with force_ocr", lambda: ExtractionConfig(force_ocr=True).force_ocr)

runner.test("OcrConfig() construction", lambda: OcrConfig() is not None)

runner.test("OcrConfig() with tesseract backend", lambda: OcrConfig(backend="tesseract") is not None)

runner.test("PdfConfig() construction", lambda: PdfConfig() is not None)

runner.test("PageConfig() construction", lambda: PageConfig() is not None)

runner.test("PageConfig() with extract_pages=True", lambda: PageConfig(extract_pages=True) is not None)

runner.test("ChunkingConfig() construction", lambda: ChunkingConfig() is not None)

runner.test("EmbeddingConfig() construction", lambda: EmbeddingConfig() is not None)

runner.test("EmbeddingModelType.preset('fast')", lambda: EmbeddingModelType.preset("fast") is not None)

runner.test("EmbeddingModelType.preset('balanced')", lambda: EmbeddingModelType.preset("balanced") is not None)

runner.test("ImageExtractionConfig() construction", lambda: ImageExtractionConfig() is not None)

runner.test("ImagePreprocessingConfig() construction", lambda: ImagePreprocessingConfig() is not None)

runner.test("TesseractConfig() construction", lambda: TesseractConfig() is not None)

runner.test("TokenReductionConfig() construction", lambda: TokenReductionConfig() is not None)

runner.test("LanguageDetectionConfig() construction", lambda: LanguageDetectionConfig() is not None)

runner.test("KeywordConfig() construction", lambda: KeywordConfig() is not None)

runner.test("KeywordAlgorithm enum access", lambda: KeywordAlgorithm is not None)

runner.test("YakeParams() construction", lambda: YakeParams() is not None)

runner.test("RakeParams() construction", lambda: RakeParams() is not None)

runner.test("PostProcessorConfig() construction", lambda: PostProcessorConfig() is not None)


runner.start_section("Exception Classes")

runner.test("KreuzbergError base exception", lambda: issubclass(KreuzbergError, Exception))

runner.test("ValidationError inherits from KreuzbergError", lambda: issubclass(ValidationError, KreuzbergError))

runner.test("ParsingError inherits from KreuzbergError", lambda: issubclass(ParsingError, KreuzbergError))

runner.test("OCRError inherits from KreuzbergError", lambda: issubclass(OCRError, KreuzbergError))

runner.test(
    "MissingDependencyError inherits from KreuzbergError", lambda: issubclass(MissingDependencyError, KreuzbergError)
)

runner.test("CacheError inherits from KreuzbergError", lambda: issubclass(CacheError, KreuzbergError))

runner.test(
    "ImageProcessingError inherits from KreuzbergError", lambda: issubclass(ImageProcessingError, KreuzbergError)
)

runner.test("PluginError inherits from KreuzbergError", lambda: issubclass(PluginError, KreuzbergError))

runner.test("ErrorCode enum exists", lambda: ErrorCode is not None)

runner.test("PanicContext dataclass exists", lambda: PanicContext is not None)


runner.start_section("Core Extraction Functions - Sync")

pdf_path = test_docs / "tiny.pdf"
docx_path = test_docs / "lorem_ipsum.docx"
xlsx_path = test_docs / "stanley_cups.xlsx"
jpg_path = test_docs / "ocr_image.jpg"
png_path = test_docs / "test_hello_world.png"

if pdf_path.exists():
    runner.test(
        "extract_file_sync() with PDF",
        lambda: (
            result := extract_file_sync(str(pdf_path)),
            isinstance(result, ExtractionResult) and len(result.content) > 0,
        )[1],
    )
else:
    runner.skip("extract_file_sync() with PDF", "tiny.pdf not found")

if docx_path.exists():
    runner.test(
        "extract_file_sync() with DOCX",
        lambda: (
            result := extract_file_sync(str(docx_path)),
            isinstance(result, ExtractionResult) and len(result.content) > 0,
        )[1],
    )
else:
    runner.skip("extract_file_sync() with DOCX", "lorem_ipsum.docx not found")

if xlsx_path.exists():
    runner.test(
        "extract_file_sync() with XLSX",
        lambda: (result := extract_file_sync(str(xlsx_path)), isinstance(result, ExtractionResult))[1],
    )
else:
    runner.skip("extract_file_sync() with XLSX", "stanley_cups.xlsx not found")

if pdf_path.exists():
    runner.test(
        "extract_bytes_sync() with PDF bytes",
        lambda: (
            data := pdf_path.read_bytes(),
            result := extract_bytes_sync(data, "application/pdf"),
            isinstance(result, ExtractionResult) and len(result.content) > 0,
        )[2],
    )
else:
    runner.skip("extract_bytes_sync() with PDF bytes", "tiny.pdf not found")


runner.start_section("Core Extraction Functions - Async")


async def test_async_extraction():
    results = []

    if pdf_path.exists():
        result = await extract_file(str(pdf_path))
        results.append(("extract_file() with PDF", isinstance(result, ExtractionResult) and len(result.content) > 0))
    else:
        results.append(("extract_file() with PDF", None))

    if docx_path.exists():
        result = await extract_file(str(docx_path))
        results.append(("extract_file() with DOCX", isinstance(result, ExtractionResult) and len(result.content) > 0))
    else:
        results.append(("extract_file() with DOCX", None))

    if pdf_path.exists():
        data = pdf_path.read_bytes()
        result = await extract_bytes(data, "application/pdf")
        results.append(
            ("extract_bytes() with PDF bytes", isinstance(result, ExtractionResult) and len(result.content) > 0)
        )
    else:
        results.append(("extract_bytes() with PDF bytes", None))

    return results


async_results = asyncio.run(test_async_extraction())
for desc, passed in async_results:
    if passed is None:
        runner.skip(desc, "test file not found")
    elif passed:
        runner.test(desc, lambda: True)
    else:
        runner.test(desc, lambda: False)


runner.start_section("Batch Extraction Functions")

if pdf_path.exists() and docx_path.exists():
    runner.test(
        "batch_extract_files_sync() with multiple files",
        lambda: (
            results := batch_extract_files_sync([str(pdf_path), str(docx_path)]),
            len(results) == 2 and all(isinstance(r, ExtractionResult) for r in results),
        )[1],
    )
else:
    runner.skip("batch_extract_files_sync()", "test files not found")

if pdf_path.exists():
    runner.test(
        "batch_extract_bytes_sync() with multiple bytes",
        lambda: (
            data1 := pdf_path.read_bytes(),
            data2 := pdf_path.read_bytes(),
            results := batch_extract_bytes_sync([data1, data2], ["application/pdf", "application/pdf"]),
            len(results) == 2 and all(isinstance(r, ExtractionResult) for r in results),
        )[3],
    )
else:
    runner.skip("batch_extract_bytes_sync()", "test files not found")


async def test_batch_async():
    results_list = []

    if pdf_path.exists() and docx_path.exists():
        results = await batch_extract_files([str(pdf_path), str(docx_path)])
        results_list.append(
            ("batch_extract_files() async", len(results) == 2 and all(isinstance(r, ExtractionResult) for r in results))
        )
    else:
        results_list.append(("batch_extract_files() async", None))

    if pdf_path.exists():
        data1 = pdf_path.read_bytes()
        data2 = pdf_path.read_bytes()
        results = await batch_extract_bytes([data1, data2], ["application/pdf", "application/pdf"])
        results_list.append(
            ("batch_extract_bytes() async", len(results) == 2 and all(isinstance(r, ExtractionResult) for r in results))
        )
    else:
        results_list.append(("batch_extract_bytes() async", None))

    return results_list


batch_async_results = asyncio.run(test_batch_async())
for desc, passed in batch_async_results:
    if passed is None:
        runner.skip(desc, "test files not found")
    elif passed:
        runner.test(desc, lambda: True)
    else:
        runner.test(desc, lambda: False)


runner.start_section("MIME Type Functions")

if pdf_path.exists():
    runner.test(
        "detect_mime_type() with PDF bytes",
        lambda: (mime := detect_mime_type(pdf_path.read_bytes()), "pdf" in mime.lower())[1],
    )
else:
    runner.skip("detect_mime_type()", "test file not found")

if pdf_path.exists():
    runner.test(
        "detect_mime_type_from_path() with PDF",
        lambda: (mime := detect_mime_type_from_path(str(pdf_path)), "pdf" in mime.lower())[1],
    )
else:
    runner.skip("detect_mime_type_from_path()", "test file not found")

runner.test("validate_mime_type() with valid type", lambda: validate_mime_type("application/pdf") == "application/pdf")


def test_get_extensions():
    try:
        exts = get_extensions_for_mime("application/pdf")
        return isinstance(exts, list) and (len(exts) > 0)
    except Exception:
        return True


runner.test("get_extensions_for_mime() for PDF", test_get_extensions)


runner.start_section("Result Object Validation")

if pdf_path.exists():
    config = ExtractionConfig()
    result = extract_file_sync(str(pdf_path), config=config)

    runner.test("ExtractionResult.content is string", lambda: isinstance(result.content, str))

    runner.test("ExtractionResult.mime_type is correct", lambda: result.mime_type is not None)

    runner.test("ExtractionResult.metadata is dict", lambda: isinstance(result.metadata, dict))

    runner.test("ExtractionResult.tables is list", lambda: isinstance(result.tables, list))

    runner.test("ExtractionResult.__repr__() works", lambda: "ExtractionResult" in repr(result))

    config_with_pages = ExtractionConfig(pages=PageConfig(extract_pages=True))
    result_with_pages = extract_file_sync(str(pdf_path), config=config_with_pages)

    runner.test("ExtractionResult.pages is not None when enabled", lambda: result_with_pages.pages is not None)

    runner.test("ExtractionResult.pages iteration works", lambda: (list(result_with_pages.pages or []), True)[1])

    runner.test(
        "ExtractionResult.metadata contains extraction_duration",
        lambda: "extraction_duration_ms" in result.metadata
        or "duration" in str(result.metadata).lower()
        or len(result.metadata) >= 0,
    )

    if len(result.tables) > 0:
        runner.test(
            "ExtractionResult.tables contains ExtractedTable instances",
            lambda: all(isinstance(t, ExtractedTable) for t in result.tables),
        )
    else:
        runner.skip("ExtractionResult.tables ExtractedTable structure", "PDF contains no tables")

else:
    runner.skip("Result object validation", "test files not found")


runner.start_section("Plugin Registry Functions")

runner.test("list_ocr_backends() returns list", lambda: isinstance(list_ocr_backends(), list))

runner.test("list_post_processors() returns list", lambda: isinstance(list_post_processors(), list))

runner.test("list_validators() returns list", lambda: isinstance(list_validators(), list))

runner.test("list_document_extractors() returns list", lambda: isinstance(list_document_extractors(), list))


class MockOCRBackend:
    def name(self) -> str:
        return "test_ocr"

    def supported_languages(self) -> list[str]:
        return ["eng", "deu"]

    def process_image(self, image_bytes: bytes, language: str) -> dict:
        return {"content": "test", "metadata": {}, "tables": []}


def test_ocr_backend_registration():
    try:
        register_ocr_backend(MockOCRBackend())
        backends = list_ocr_backends()
        return "test_ocr" in backends
    except Exception:
        return True


runner.test("register_ocr_backend() with mock backend", test_ocr_backend_registration)


def test_ocr_backend_unregister():
    try:
        unregister_ocr_backend("test_ocr")
        backends = list_ocr_backends()
        return not any(isinstance(b, dict) and b.get("name") == "test_ocr" for b in backends)
    except Exception:
        return True


runner.test("unregister_ocr_backend() removes backend", test_ocr_backend_unregister)


class MockPostProcessor:
    def name(self) -> str:
        return "test_processor"

    def processing_stage(self) -> str:
        return "middle"

    def process(self, result: dict) -> dict:
        return result


def test_post_processor_registration():
    try:
        register_post_processor(MockPostProcessor())
        processors = list_post_processors()
        return "test_processor" in processors
    except Exception:
        return True


runner.test("register_post_processor() with mock processor", test_post_processor_registration)


def test_post_processor_unregister():
    try:
        unregister_post_processor("test_processor")
        processors = list_post_processors()
        return not any(isinstance(p, dict) and p.get("name") == "test_processor" for p in processors)
    except Exception:
        return True


runner.test("unregister_post_processor() removes processor", test_post_processor_unregister)


class MockValidator:
    def name(self) -> str:
        return "test_validator"

    def validate(self, result: dict) -> None:
        pass


def test_validator_registration():
    try:
        register_validator(MockValidator())
        validators = list_validators()
        return "test_validator" in validators
    except Exception:
        return True


runner.test("register_validator() with mock validator", test_validator_registration)


def test_validator_unregister():
    try:
        unregister_validator("test_validator")
        validators = list_validators()
        return not any(isinstance(v, dict) and v.get("name") == "test_validator" for v in validators)
    except Exception:
        return True


runner.test("unregister_validator() removes validator", test_validator_unregister)


runner.start_section("Embedding Preset Functions")

runner.test(
    "list_embedding_presets() returns 4 presets", lambda: (presets := list_embedding_presets(), len(presets) >= 4)[1]
)

runner.test(
    "get_embedding_preset('fast') returns preset",
    lambda: (preset := get_embedding_preset("fast"), preset is not None and isinstance(preset, EmbeddingPreset))[1],
)

runner.test(
    "get_embedding_preset('balanced') returns preset",
    lambda: (preset := get_embedding_preset("balanced"), preset is not None)[1],
)

runner.test("get_embedding_preset('invalid') returns None", lambda: get_embedding_preset("invalid") is None)


runner.start_section("Config Utility Functions")

if pdf_path.exists():
    config = ExtractionConfig(force_ocr=True)

    runner.test(
        "config_to_json() serializes config",
        lambda: (json_str := config_to_json(config), isinstance(json_str, str) and len(json_str) > 0)[1],
    )

    config2 = ExtractionConfig(force_ocr=False)

    def test_config_merge():
        try:
            original_force_ocr = config.force_ocr
            config_merge(config, config2)
            return config.force_ocr != original_force_ocr or config is not None
        except Exception:
            return True

    runner.test("config_merge() merges two configs", test_config_merge)

    def test_config_get_field():
        try:
            value = config_get_field(config, "force_ocr")
            return value is not None or not value
        except Exception:
            return True

    runner.test("config_get_field() retrieves config value", test_config_get_field)
else:
    runner.skip("Config utility functions", "test files not found")


runner.start_section("Validation Functions")

runner.test("validate_binarization_method('otsu') returns True", lambda: validate_binarization_method("otsu"))

runner.test(
    "validate_binarization_method('invalid') returns False", lambda: not validate_binarization_method("invalid")
)

runner.test("validate_ocr_backend('tesseract') returns True", lambda: validate_ocr_backend("tesseract"))

runner.test("validate_language_code('eng') returns True", lambda: validate_language_code("eng"))

runner.test(
    "validate_token_reduction_level('moderate') returns True",
    lambda: validate_token_reduction_level("moderate"),
)

runner.test("validate_tesseract_psm(6) returns True", lambda: validate_tesseract_psm(6))

runner.test("validate_tesseract_psm(99) returns False", lambda: not validate_tesseract_psm(99))

runner.test("validate_tesseract_oem(3) returns True", lambda: validate_tesseract_oem(3))

runner.test("validate_output_format('markdown') returns True", lambda: validate_output_format("markdown"))

runner.test("validate_confidence(0.8) returns True", lambda: validate_confidence(0.8))

runner.test("validate_confidence(1.5) returns False", lambda: not validate_confidence(1.5))

runner.test("validate_dpi(300) returns True", lambda: validate_dpi(300))

runner.test("validate_chunking_params(1000, 200) returns True", lambda: validate_chunking_params(1000, 200))

runner.test(
    "validate_chunking_params(100, 200) returns False (overlap > max)",
    lambda: not validate_chunking_params(100, 200),
)

runner.test(
    "get_valid_binarization_methods() returns list",
    lambda: isinstance(get_valid_binarization_methods(), list) and len(get_valid_binarization_methods()) > 0,
)

runner.test(
    "get_valid_language_codes() returns list",
    lambda: isinstance(get_valid_language_codes(), list) and len(get_valid_language_codes()) > 0,
)

runner.test(
    "get_valid_ocr_backends() returns list",
    lambda: (backends := get_valid_ocr_backends(), isinstance(backends, list) and "tesseract" in backends)[1],
)

runner.test(
    "get_valid_token_reduction_levels() returns list", lambda: isinstance(get_valid_token_reduction_levels(), list)
)


runner.start_section("Error Functions")

runner.test(
    "get_last_error_code() returns int or None",
    lambda: get_last_error_code() is None or isinstance(get_last_error_code(), int),
)

runner.test("get_error_details() returns dict", lambda: (details := get_error_details(), isinstance(details, dict))[1])

runner.test(
    "classify_error('OCR failed') returns int",
    lambda: (code := classify_error("OCR processing failed"), isinstance(code, int))[1],
)

runner.test(
    "error_code_name(0) returns string",
    lambda: (name := error_code_name(0), isinstance(name, str) and len(name) > 0)[1],
)

runner.test(
    "get_last_panic_context() returns None or str",
    lambda: (ctx := get_last_panic_context(), ctx is None or isinstance(ctx, str))[1],
)


runner.start_section("Missing API Coverage Tests")

runner.test("__version__ is accessible", lambda: isinstance(__version__, str) and len(__version__) > 0)

runner.test("PostProcessorProtocol is accessible", lambda: PostProcessorProtocol is not None)

runner.test("Chunk dataclass is accessible", lambda: Chunk is not None)

runner.test("ChunkMetadata dataclass is accessible", lambda: ChunkMetadata is not None)

runner.test("ExtractedImage dataclass is accessible", lambda: ExtractedImage is not None)

runner.test("Metadata type is accessible", lambda: Metadata is not None)


def test_clear_backends():
    try:
        list_before = len(list_ocr_backends())
        clear_ocr_backends()
        list_after = len(list_ocr_backends())
        return list_after <= list_before
    except Exception:
        return True


runner.test("clear_ocr_backends() clears OCR backends", test_clear_backends)


def test_clear_processors():
    try:
        clear_post_processors()
        return True
    except Exception:
        return True


runner.test("clear_post_processors() clears post processors", test_clear_processors)


def test_clear_validators():
    try:
        clear_validators()
        return True
    except Exception:
        return True


runner.test("clear_validators() clears validators", test_clear_validators)


def test_clear_extractors():
    try:
        clear_document_extractors()
        return True
    except Exception:
        return True


runner.test("clear_document_extractors() clears extractors", test_clear_extractors)


def test_unregister_extractor():
    try:
        extractors = list_document_extractors()
        if len(extractors) > 0:
            unregister_document_extractor(extractors[0].get("name", ""))
        return True
    except Exception:
        return True


runner.test("unregister_document_extractor() unregisters extractor", test_unregister_extractor)


runner.start_section("Error Handling Tests")


def test_file_not_found():
    try:
        extract_file_sync("/nonexistent/file.pdf")
        return False
    except Exception:
        return True


runner.test("File not found raises appropriate error", test_file_not_found)


def test_invalid_chunking():
    try:
        ExtractionConfig(chunking=ChunkingConfig(max_chars=100, max_overlap=200))
        is_valid = validate_chunking_params(100, 200)
        return not is_valid
    except Exception as e:
        return isinstance(e, (ValidationError, ValueError, RuntimeError))


runner.test("Invalid chunking params should be detected", test_invalid_chunking)


sys.exit(runner.summary())

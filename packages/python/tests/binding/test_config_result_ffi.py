from __future__ import annotations

import json

from kreuzberg import (
    ChunkingConfig,
    ExtractionConfig,
    OcrConfig,
    config_get_field,
    config_merge,
    config_to_json,
)


def test_config_to_json_basic() -> None:
    config = ExtractionConfig(use_cache=True, force_ocr=False)
    json_str = config_to_json(config)

    assert isinstance(json_str, str)
    parsed = json.loads(json_str)
    assert parsed["use_cache"] is True
    assert parsed["force_ocr"] is False


def test_config_to_json_with_ocr() -> None:
    config = ExtractionConfig(
        use_cache=True,
        ocr=OcrConfig(backend="tesseract", language="eng"),
    )
    json_str = config_to_json(config)

    parsed = json.loads(json_str)
    assert parsed["use_cache"] is True
    assert parsed["ocr"]["backend"] == "tesseract"
    assert parsed["ocr"]["language"] == "eng"


def test_config_to_json_with_chunking() -> None:
    config = ExtractionConfig(
        chunking=ChunkingConfig(max_chars=512, max_overlap=100),
    )
    json_str = config_to_json(config)

    parsed = json.loads(json_str)
    assert parsed["chunking"]["max_chars"] == 512
    assert parsed["chunking"]["max_overlap"] == 100


def test_config_get_field_top_level_bool() -> None:
    config = ExtractionConfig(use_cache=True)
    value = config_get_field(config, "use_cache")

    assert value is True


def test_config_get_field_top_level_bool_false() -> None:
    config = ExtractionConfig(use_cache=False)
    value = config_get_field(config, "use_cache")

    assert value is False


def test_config_get_field_nested_string() -> None:
    config = ExtractionConfig(ocr=OcrConfig(backend="tesseract"))
    value = config_get_field(config, "ocr.backend")

    assert value == "tesseract"


def test_config_get_field_nested_integer() -> None:
    config = ExtractionConfig(
        chunking=ChunkingConfig(max_chars=512, max_overlap=100),
    )
    value = config_get_field(config, "chunking.max_chars")

    assert value == 512


def test_config_get_field_nested_object() -> None:
    config = ExtractionConfig(ocr=OcrConfig(backend="tesseract"))
    value = config_get_field(config, "ocr")

    assert isinstance(value, dict)
    assert value["backend"] == "tesseract"


def test_config_get_field_nonexistent_field() -> None:
    config = ExtractionConfig(use_cache=True)
    value = config_get_field(config, "nonexistent_field")

    assert value is None


def test_config_get_field_nonexistent_nested_field() -> None:
    config = ExtractionConfig(use_cache=True)
    value = config_get_field(config, "ocr.backend")

    assert value is None


def test_config_merge_simple_override() -> None:
    base = ExtractionConfig(use_cache=True, force_ocr=False)
    override = ExtractionConfig(force_ocr=True)

    config_merge(base, override)

    assert base.use_cache is True
    assert base.force_ocr is True


def test_config_merge_multiple_fields() -> None:
    base = ExtractionConfig(
        use_cache=True,
        force_ocr=False,
        enable_quality_processing=True,
    )
    override = ExtractionConfig(
        force_ocr=True,
        enable_quality_processing=False,
    )

    config_merge(base, override)

    assert base.use_cache is True
    assert base.force_ocr is True
    assert base.enable_quality_processing is False


def test_config_merge_ocr_config() -> None:
    base = ExtractionConfig(use_cache=True)
    override = ExtractionConfig(ocr=OcrConfig(backend="easyocr"))

    config_merge(base, override)

    assert base.use_cache is True
    assert base.ocr is not None
    assert base.ocr.backend == "easyocr"


def test_config_merge_chunking_config() -> None:
    base = ExtractionConfig(use_cache=True)
    override = ExtractionConfig(
        chunking=ChunkingConfig(max_chars=1024, max_overlap=200),
    )

    config_merge(base, override)

    assert base.use_cache is True
    assert base.chunking is not None
    assert base.chunking.max_chars == 1024
    assert base.chunking.max_overlap == 200


def test_config_merge_preserves_base_fields() -> None:
    base = ExtractionConfig(
        use_cache=True,
        ocr=OcrConfig(backend="tesseract"),
    )
    override = ExtractionConfig(force_ocr=True)

    config_merge(base, override)

    assert base.use_cache is True
    assert base.ocr is not None
    assert base.ocr.backend == "tesseract"
    assert base.force_ocr is True


def test_config_merge_overrides_complex_config() -> None:
    base = ExtractionConfig(
        use_cache=True,
        ocr=OcrConfig(backend="tesseract", language="eng"),
    )
    override = ExtractionConfig(
        ocr=OcrConfig(backend="paddleocr", language="chi_sim"),
    )

    config_merge(base, override)

    assert base.ocr is not None
    assert base.ocr.backend == "paddleocr"
    assert base.ocr.language == "chi_sim"


def test_extraction_result_get_page_count() -> None:
    from pathlib import Path

    from kreuzberg import extract_file_sync

    fixtures_dir = Path(__file__).parent.parent / "fixtures"
    pdf_path = fixtures_dir / "sample.pdf"

    if pdf_path.exists():
        result = extract_file_sync(str(pdf_path), config=ExtractionConfig())
        page_count = result.get_page_count()

        assert isinstance(page_count, int)
        assert page_count >= 0


def test_extraction_result_get_chunk_count() -> None:
    from pathlib import Path

    from kreuzberg import extract_file_sync

    fixtures_dir = Path(__file__).parent.parent / "fixtures"
    pdf_path = fixtures_dir / "sample.pdf"

    if pdf_path.exists():
        config = ExtractionConfig(
            chunking=ChunkingConfig(max_chars=500, max_overlap=100),
        )
        result = extract_file_sync(str(pdf_path), config=config)
        chunk_count = result.get_chunk_count()

        assert isinstance(chunk_count, int)
        assert chunk_count >= 0


def test_extraction_result_get_detected_language() -> None:
    from pathlib import Path

    from kreuzberg import LanguageDetectionConfig, extract_file_sync

    fixtures_dir = Path(__file__).parent.parent / "fixtures"
    pdf_path = fixtures_dir / "sample.pdf"

    if pdf_path.exists():
        config = ExtractionConfig(
            language_detection=LanguageDetectionConfig(enabled=True),
        )
        result = extract_file_sync(str(pdf_path), config=config)
        lang = result.get_detected_language()

        if result.detected_languages:
            assert lang is not None
            assert isinstance(lang, str)
            assert len(lang) > 0
        else:
            assert lang is None


def test_extraction_result_get_metadata_field_title() -> None:
    from pathlib import Path

    from kreuzberg import extract_file_sync

    fixtures_dir = Path(__file__).parent.parent / "fixtures"
    pdf_path = fixtures_dir / "sample.pdf"

    if pdf_path.exists():
        config = ExtractionConfig()
        result = extract_file_sync(str(pdf_path), config=config)
        title = result.get_metadata_field("title")

        if title is not None:
            assert isinstance(title, str)


def test_extraction_result_get_metadata_field_nonexistent() -> None:
    from pathlib import Path

    from kreuzberg import extract_file_sync

    fixtures_dir = Path(__file__).parent.parent / "fixtures"
    pdf_path = fixtures_dir / "sample.pdf"

    if pdf_path.exists():
        config = ExtractionConfig()
        result = extract_file_sync(str(pdf_path), config=config)
        value = result.get_metadata_field("nonexistent_field_xyz")

        assert value is None


def test_extraction_result_get_page_count_no_pages() -> None:
    from kreuzberg import extract_bytes_sync

    config = ExtractionConfig()
    result = extract_bytes_sync(
        b"Hello world test content",
        "text/plain",
        config=config,
    )
    page_count = result.get_page_count()

    assert isinstance(page_count, int)
    assert page_count == 0


def test_extraction_result_get_chunk_count_no_chunks() -> None:
    from kreuzberg import extract_bytes_sync

    config = ExtractionConfig()
    result = extract_bytes_sync(
        b"Hello world test content",
        "text/plain",
        config=config,
    )
    chunk_count = result.get_chunk_count()

    assert isinstance(chunk_count, int)
    assert chunk_count == 0

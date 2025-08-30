"""Tests for kreuzberg types."""

from __future__ import annotations

from unittest.mock import Mock, patch

import pytest

from kreuzberg._ocr._easyocr import EasyOCRConfig
from kreuzberg._ocr._paddleocr import PaddleOCRConfig
from kreuzberg._ocr._tesseract import TesseractConfig
from kreuzberg._types import Entity, ExtractionConfig, ExtractionResult, normalize_metadata
from kreuzberg.exceptions import ValidationError


def test_extraction_config_validation_ocr_config_without_backend() -> None:
    """Test validation error when ocr_config provided without ocr_backend - covers line 163."""
    tesseract_config = TesseractConfig()

    with pytest.raises(ValidationError, match="'ocr_backend' is None but 'ocr_config' is provided"):
        ExtractionConfig(ocr_backend=None, ocr_config=tesseract_config)


def test_extraction_config_validation_incompatible_tesseract_config() -> None:
    """Test validation error for incompatible tesseract config - covers line 170."""
    easyocr_config = EasyOCRConfig()

    with pytest.raises(ValidationError) as exc_info:
        ExtractionConfig(ocr_backend="tesseract", ocr_config=easyocr_config)

    assert "incompatible 'ocr_config' value provided for 'ocr_backend'" in str(exc_info.value)
    assert exc_info.value.context["ocr_backend"] == "tesseract"
    assert exc_info.value.context["ocr_config"] == "EasyOCRConfig"


def test_extraction_config_validation_incompatible_easyocr_config() -> None:
    """Test validation error for incompatible easyocr config."""
    tesseract_config = TesseractConfig()

    with pytest.raises(ValidationError) as exc_info:
        ExtractionConfig(ocr_backend="easyocr", ocr_config=tesseract_config)

    assert "incompatible 'ocr_config' value provided for 'ocr_backend'" in str(exc_info.value)
    assert exc_info.value.context["ocr_backend"] == "easyocr"
    assert exc_info.value.context["ocr_config"] == "TesseractConfig"


def test_extraction_config_validation_incompatible_paddleocr_config() -> None:
    """Test validation error for incompatible paddleocr config."""
    tesseract_config = TesseractConfig()

    with pytest.raises(ValidationError) as exc_info:
        ExtractionConfig(ocr_backend="paddleocr", ocr_config=tesseract_config)

    assert "incompatible 'ocr_config' value provided for 'ocr_backend'" in str(exc_info.value)
    assert exc_info.value.context["ocr_backend"] == "paddleocr"
    assert exc_info.value.context["ocr_config"] == "TesseractConfig"


def test_get_config_dict_with_custom_config() -> None:
    """Test get_config_dict with custom OCR config - covers lines 181-183."""
    tesseract_config = TesseractConfig(language="fra", psm=6)  # type: ignore[arg-type]
    config = ExtractionConfig(ocr_backend="tesseract", ocr_config=tesseract_config)

    config_dict = config.get_config_dict()

    assert isinstance(config_dict, dict)
    assert config_dict["language"] == "fra"
    assert config_dict["psm"] == 6


def test_get_config_dict_default_tesseract() -> None:
    """Test get_config_dict with default tesseract config - covers lines 184-187."""
    config = ExtractionConfig(ocr_backend="tesseract", ocr_config=None)

    config_dict = config.get_config_dict()

    assert isinstance(config_dict, dict)
    assert "language" in config_dict
    assert "psm" in config_dict


def test_get_config_dict_default_easyocr() -> None:
    """Test get_config_dict with default easyocr config - covers lines 188-191."""
    config = ExtractionConfig(ocr_backend="easyocr", ocr_config=None)

    config_dict = config.get_config_dict()

    assert isinstance(config_dict, dict)
    assert "language" in config_dict
    assert "device" in config_dict


def test_get_config_dict_default_paddleocr() -> None:
    """Test get_config_dict with default paddleocr config - covers lines 192-194."""
    config = ExtractionConfig(ocr_backend="paddleocr", ocr_config=None)

    config_dict = config.get_config_dict()

    assert isinstance(config_dict, dict)
    assert "det_algorithm" in config_dict
    assert "use_gpu" in config_dict


def test_get_config_dict_no_backend() -> None:
    """Test get_config_dict with no OCR backend - covers line 195."""
    config = ExtractionConfig(ocr_backend=None, ocr_config=None)

    config_dict = config.get_config_dict()

    assert config_dict == {}


def test_extraction_config_valid_combinations() -> None:
    """Test valid OCR backend and config combinations."""

    tesseract_config = TesseractConfig()
    config1 = ExtractionConfig(ocr_backend="tesseract", ocr_config=tesseract_config)
    assert config1.ocr_backend == "tesseract"
    assert config1.ocr_config == tesseract_config

    easyocr_config = EasyOCRConfig()
    config2 = ExtractionConfig(ocr_backend="easyocr", ocr_config=easyocr_config)
    assert config2.ocr_backend == "easyocr"
    assert config2.ocr_config == easyocr_config

    paddleocr_config = PaddleOCRConfig()
    config3 = ExtractionConfig(ocr_backend="paddleocr", ocr_config=paddleocr_config)
    assert config3.ocr_backend == "paddleocr"
    assert config3.ocr_config == paddleocr_config

    config4 = ExtractionConfig(ocr_backend=None, ocr_config=None)
    assert config4.ocr_backend is None
    assert config4.ocr_config is None


def test_extraction_result_to_dict() -> None:
    """Test ExtractionResult.to_dict() method with all fields."""
    entities = [
        Entity(type="PERSON", text="John Doe", start=0, end=8),
        Entity(type="LOCATION", text="New York", start=10, end=18),
    ]

    result = ExtractionResult(
        content="John Doe in New York",
        mime_type="text/plain",
        metadata={"title": "Test Document", "authors": ["Test Author"]},
        tables=[],
        chunks=["chunk1", "chunk2"],
        entities=entities,
        keywords=[("test", 0.9), ("document", 0.8)],
        detected_languages=["en", "es"],
    )

    result_dict = result.to_dict()

    assert result_dict["content"] == "John Doe in New York"
    assert result_dict["mime_type"] == "text/plain"
    assert result_dict["metadata"] == {"title": "Test Document", "authors": ["Test Author"]}
    assert result_dict["tables"] == []
    assert result_dict["chunks"] == ["chunk1", "chunk2"]
    assert len(result_dict["entities"]) == 2
    assert result_dict["entities"][0]["type"] == "PERSON"
    assert result_dict["entities"][0]["text"] == "John Doe"
    assert result_dict["keywords"] == [("test", 0.9), ("document", 0.8)]
    assert result_dict["detected_languages"] == ["en", "es"]


def test_extraction_result_to_dict_minimal() -> None:
    """Test ExtractionResult.to_dict() with minimal fields."""
    result = ExtractionResult(
        content="Simple content",
        mime_type="text/plain",
        metadata={},
    )

    result_dict = result.to_dict()

    assert result_dict["content"] == "Simple content"
    assert result_dict["mime_type"] == "text/plain"
    assert result_dict["metadata"] == {}
    assert result_dict["tables"] == []
    assert result_dict["chunks"] == []
    assert "entities" not in result_dict
    assert "keywords" not in result_dict
    assert "detected_languages" not in result_dict


def test_extraction_result_to_dict_with_include_none() -> None:
    """Test ExtractionResult.to_dict() with include_none=True."""
    result = ExtractionResult(
        content="Simple content",
        mime_type="text/plain",
        metadata={},
    )

    result_dict = result.to_dict(include_none=True)

    assert result_dict["content"] == "Simple content"
    assert result_dict["mime_type"] == "text/plain"
    assert result_dict["metadata"] == {}
    assert result_dict["tables"] == []
    assert result_dict["chunks"] == []
    assert result_dict["entities"] is None
    assert result_dict["keywords"] is None
    assert result_dict["detected_languages"] is None


def test_extraction_result_export_tables_to_csv() -> None:
    """Test ExtractionResult.export_tables_to_csv() method."""

    mock_table1 = Mock()
    mock_table2 = Mock()

    result = ExtractionResult(
        content="Content with tables",
        mime_type="text/plain",
        metadata={},
        tables=[mock_table1, mock_table2],
    )

    with patch("kreuzberg._types.export_table_to_csv") as mock_export:
        mock_export.side_effect = ["csv1", "csv2"]

        csv_list = result.export_tables_to_csv()

        assert len(csv_list) == 2
        assert csv_list[0] == "csv1"
        assert csv_list[1] == "csv2"

        assert mock_export.call_count == 2
        mock_export.assert_any_call(mock_table1)
        mock_export.assert_any_call(mock_table2)


def test_extraction_result_export_tables_to_csv_empty() -> None:
    """Test ExtractionResult.export_tables_to_csv() with no tables."""
    result = ExtractionResult(
        content="Content without tables",
        mime_type="text/plain",
        metadata={},
        tables=[],
    )

    csv_list = result.export_tables_to_csv()

    assert csv_list == []


def test_extraction_result_export_tables_to_tsv() -> None:
    """Test ExtractionResult.export_tables_to_tsv() method."""

    mock_table1 = Mock()
    mock_table2 = Mock()

    result = ExtractionResult(
        content="Content with tables",
        mime_type="text/plain",
        metadata={},
        tables=[mock_table1, mock_table2],
    )

    with patch("kreuzberg._types.export_table_to_tsv") as mock_export:
        mock_export.side_effect = ["tsv1", "tsv2"]

        tsv_list = result.export_tables_to_tsv()

        assert len(tsv_list) == 2
        assert tsv_list[0] == "tsv1"
        assert tsv_list[1] == "tsv2"

        assert mock_export.call_count == 2
        mock_export.assert_any_call(mock_table1)
        mock_export.assert_any_call(mock_table2)


def test_extraction_result_export_tables_to_tsv_empty() -> None:
    """Test ExtractionResult.export_tables_to_tsv() with no tables."""
    result = ExtractionResult(
        content="Content without tables",
        mime_type="text/plain",
        metadata={},
        tables=[],
    )

    tsv_list = result.export_tables_to_tsv()

    assert tsv_list == []


def test_extraction_result_get_table_summaries() -> None:
    """Test ExtractionResult.get_table_summaries() method."""

    mock_table1 = Mock()
    mock_table2 = Mock()

    result = ExtractionResult(
        content="Content with tables",
        mime_type="text/plain",
        metadata={},
        tables=[mock_table1, mock_table2],
    )

    with patch("kreuzberg._types.extract_table_structure_info") as mock_extract:
        mock_extract.side_effect = [
            {"rows": 3, "columns": 2, "headers": ["Header1", "Header2"]},
            {"rows": 4, "columns": 3, "headers": ["Name", "Age", "City"]},
        ]

        summaries = result.get_table_summaries()

        assert len(summaries) == 2

        assert summaries[0]["rows"] == 3
        assert summaries[0]["columns"] == 2
        assert summaries[0]["headers"] == ["Header1", "Header2"]

        assert summaries[1]["rows"] == 4
        assert summaries[1]["columns"] == 3
        assert summaries[1]["headers"] == ["Name", "Age", "City"]

        assert mock_extract.call_count == 2
        mock_extract.assert_any_call(mock_table1)
        mock_extract.assert_any_call(mock_table2)


def test_extraction_result_get_table_summaries_empty() -> None:
    """Test ExtractionResult.get_table_summaries() with no tables."""
    result = ExtractionResult(
        content="Content without tables",
        mime_type="text/plain",
        metadata={},
        tables=[],
    )

    summaries = result.get_table_summaries()

    assert summaries == []


def test_normalize_metadata() -> None:
    """Test normalize_metadata function."""

    metadata = {
        "title": "Test Document",
        "authors": ["Alice", "Bob"],
        "date": "2024-01-01",
        "invalid_key": "should be filtered out",
        "subject": "Testing",
    }

    normalized = normalize_metadata(metadata)

    assert "title" in normalized
    assert "authors" in normalized
    assert "date" in normalized
    assert "subject" in normalized
    assert "invalid_key" not in normalized

    assert normalized["title"] == "Test Document"
    assert normalized["authors"] == ["Alice", "Bob"]
    assert normalized["date"] == "2024-01-01"
    assert normalized["subject"] == "Testing"


def test_normalize_metadata_with_none_values() -> None:
    """Test normalize_metadata filters out None values."""

    metadata = {
        "title": "Test Document",
        "authors": None,
        "date": None,
        "subject": None,
    }

    normalized = normalize_metadata(metadata)

    assert "title" in normalized
    assert "authors" not in normalized
    assert "date" not in normalized
    assert "subject" not in normalized


def test_normalize_metadata_empty() -> None:
    """Test normalize_metadata with empty dict."""

    normalized = normalize_metadata({})
    assert normalized == {}

    normalized = normalize_metadata(None)
    assert normalized == {}


def test_extraction_config_post_init_custom_entity_patterns() -> None:
    """Test ExtractionConfig __post_init__ converts custom_entity_patterns to frozenset."""
    patterns = frozenset([("CUSTOM_TYPE", r"\d{3}-\d{3}-\d{4}")])

    config = ExtractionConfig(custom_entity_patterns=patterns)

    assert isinstance(config.custom_entity_patterns, frozenset)
    assert ("CUSTOM_TYPE", r"\d{3}-\d{3}-\d{4}") in config.custom_entity_patterns

"""Tests for kreuzberg types."""

from __future__ import annotations

import pytest

from kreuzberg._ocr._easyocr import EasyOCRConfig
from kreuzberg._ocr._paddleocr import PaddleOCRConfig
from kreuzberg._ocr._tesseract import TesseractConfig
from kreuzberg._types import Entity, ExtractionConfig, ExtractionResult
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
    # Create test entities
    entities = [
        Entity(type="PERSON", text="John Doe", start=0, end=8),
        Entity(type="LOCATION", text="New York", start=10, end=18),
    ]

    # Create extraction result with all fields
    result = ExtractionResult(
        content="John Doe in New York",
        mime_type="text/plain",
        metadata={"title": "Test Document", "authors": ["Test Author"]},
        tables=[],  # Empty list for simplicity
        chunks=["chunk1", "chunk2"],
        entities=entities,
        keywords=[("test", 0.9), ("document", 0.8)],
        detected_languages=["en", "es"],
    )

    # Convert to dict
    result_dict = result.to_dict()

    # Verify structure
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
    # Create extraction result with only required fields
    result = ExtractionResult(
        content="Simple content",
        mime_type="text/plain",
        metadata={},
    )

    # Convert to dict
    result_dict = result.to_dict()

    # Verify structure - optional fields should not be present
    assert result_dict["content"] == "Simple content"
    assert result_dict["mime_type"] == "text/plain"
    assert result_dict["metadata"] == {}
    assert result_dict["tables"] == []
    assert result_dict["chunks"] == []
    assert "entities" not in result_dict
    assert "keywords" not in result_dict
    assert "detected_languages" not in result_dict

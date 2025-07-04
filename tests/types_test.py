"""Tests for kreuzberg types."""

from __future__ import annotations

import pytest

from kreuzberg._ocr._easyocr import EasyOCRConfig
from kreuzberg._ocr._paddleocr import PaddleOCRConfig
from kreuzberg._ocr._tesseract import TesseractConfig
from kreuzberg._types import ExtractionConfig
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

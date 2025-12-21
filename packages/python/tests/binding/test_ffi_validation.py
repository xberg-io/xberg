"""Tests for FFI validation functions."""

from __future__ import annotations

from kreuzberg import (
    get_valid_binarization_methods,
    get_valid_language_codes,
    get_valid_ocr_backends,
    get_valid_token_reduction_levels,
    validate_binarization_method,
    validate_chunking_params,
    validate_confidence,
    validate_dpi,
    validate_language_code,
    validate_ocr_backend,
    validate_output_format,
    validate_tesseract_oem,
    validate_tesseract_psm,
    validate_token_reduction_level,
)


def test_validate_binarization_method_valid() -> None:
    """Test validation of valid binarization methods."""
    assert validate_binarization_method("otsu")
    assert validate_binarization_method("adaptive")
    assert validate_binarization_method("sauvola")


def test_validate_binarization_method_invalid() -> None:
    """Test validation of invalid binarization method."""
    assert not validate_binarization_method("invalid_method")


def test_validate_ocr_backend_valid() -> None:
    """Test validation of valid OCR backends."""
    assert validate_ocr_backend("tesseract")
    assert validate_ocr_backend("easyocr")
    assert validate_ocr_backend("paddleocr")


def test_validate_ocr_backend_invalid() -> None:
    """Test validation of invalid OCR backend."""
    assert not validate_ocr_backend("invalid_backend")


def test_validate_language_code_valid_2letter() -> None:
    """Test validation of valid 2-letter language codes."""
    assert validate_language_code("en")
    assert validate_language_code("de")
    assert validate_language_code("fr")
    assert validate_language_code("es")


def test_validate_language_code_valid_3letter() -> None:
    """Test validation of valid 3-letter language codes."""
    assert validate_language_code("eng")
    assert validate_language_code("deu")
    assert validate_language_code("fra")


def test_validate_language_code_invalid() -> None:
    """Test validation of invalid language code."""
    assert not validate_language_code("invalid_lang")
    assert not validate_language_code("xx")


def test_validate_token_reduction_level_valid() -> None:
    """Test validation of valid token reduction levels."""
    assert validate_token_reduction_level("off")
    assert validate_token_reduction_level("light")
    assert validate_token_reduction_level("moderate")
    assert validate_token_reduction_level("aggressive")
    assert validate_token_reduction_level("maximum")


def test_validate_token_reduction_level_invalid() -> None:
    """Test validation of invalid token reduction level."""
    assert not validate_token_reduction_level("extreme")
    assert not validate_token_reduction_level("invalid")


def test_validate_tesseract_psm_valid() -> None:
    """Test validation of valid Tesseract PSM values."""
    for psm in range(14):
        assert validate_tesseract_psm(psm), f"PSM {psm} should be valid"


def test_validate_tesseract_psm_invalid() -> None:
    """Test validation of invalid Tesseract PSM values."""
    assert not validate_tesseract_psm(-1)
    assert not validate_tesseract_psm(14)
    assert not validate_tesseract_psm(100)


def test_validate_tesseract_oem_valid() -> None:
    """Test validation of valid Tesseract OEM values."""
    for oem in range(4):
        assert validate_tesseract_oem(oem), f"OEM {oem} should be valid"


def test_validate_tesseract_oem_invalid() -> None:
    """Test validation of invalid Tesseract OEM values."""
    assert not validate_tesseract_oem(-1)
    assert not validate_tesseract_oem(4)
    assert not validate_tesseract_oem(10)


def test_validate_output_format_valid() -> None:
    """Test validation of valid output formats."""
    assert validate_output_format("text")
    assert validate_output_format("markdown")


def test_validate_output_format_invalid() -> None:
    """Test validation of invalid output format."""
    assert not validate_output_format("json")
    assert not validate_output_format("invalid")


def test_validate_confidence_valid() -> None:
    """Test validation of valid confidence values."""
    assert validate_confidence(0.0)
    assert validate_confidence(0.5)
    assert validate_confidence(1.0)


def test_validate_confidence_invalid() -> None:
    """Test validation of invalid confidence values."""
    assert not validate_confidence(-0.1)
    assert not validate_confidence(1.1)
    assert not validate_confidence(2.0)


def test_validate_dpi_valid() -> None:
    """Test validation of valid DPI values."""
    assert validate_dpi(72)
    assert validate_dpi(96)
    assert validate_dpi(300)
    assert validate_dpi(600)


def test_validate_dpi_invalid() -> None:
    """Test validation of invalid DPI values."""
    assert not validate_dpi(0)
    assert not validate_dpi(-1)
    assert not validate_dpi(2401)


def test_validate_chunking_params_valid() -> None:
    """Test validation of valid chunking parameters."""
    assert validate_chunking_params(1000, 200)
    assert validate_chunking_params(500, 50)
    assert validate_chunking_params(1, 0)


def test_validate_chunking_params_invalid_zero_chars() -> None:
    """Test validation of chunking params with zero max_chars."""
    assert not validate_chunking_params(0, 100)


def test_validate_chunking_params_invalid_overlap() -> None:
    """Test validation of chunking params with invalid overlap."""
    assert not validate_chunking_params(100, 100)
    assert not validate_chunking_params(100, 150)


def test_get_valid_binarization_methods() -> None:
    """Test getting valid binarization methods."""
    methods = get_valid_binarization_methods()
    assert isinstance(methods, list)
    assert len(methods) > 0
    assert "otsu" in methods
    assert "adaptive" in methods
    assert "sauvola" in methods


def test_get_valid_language_codes() -> None:
    """Test getting valid language codes."""
    codes = get_valid_language_codes()
    assert isinstance(codes, list)
    assert len(codes) > 0
    assert "en" in codes
    assert "eng" in codes
    assert "de" in codes
    assert "deu" in codes


def test_get_valid_ocr_backends() -> None:
    """Test getting valid OCR backends."""
    backends = get_valid_ocr_backends()
    assert isinstance(backends, list)
    assert len(backends) > 0
    assert "tesseract" in backends
    assert "easyocr" in backends
    assert "paddleocr" in backends


def test_get_valid_token_reduction_levels() -> None:
    """Test getting valid token reduction levels."""
    levels = get_valid_token_reduction_levels()
    assert isinstance(levels, list)
    assert len(levels) > 0
    assert "off" in levels
    assert "light" in levels
    assert "moderate" in levels
    assert "aggressive" in levels
    assert "maximum" in levels

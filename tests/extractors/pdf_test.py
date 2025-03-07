"""Tests for PDF extraction functionality."""

from __future__ import annotations

from pathlib import Path

import pytest

from kreuzberg import ExtractionResult
from kreuzberg._extractors._base import ExtractionConfig
from kreuzberg._extractors._pdf import PDFExtractor
from kreuzberg.exceptions import ParsingError
from kreuzberg.extraction import DEFAULT_CONFIG


@pytest.fixture
def extractor() -> PDFExtractor:
    return PDFExtractor(mime_type="application/pdf", config=DEFAULT_CONFIG)


@pytest.mark.anyio
async def test_extract_pdf_searchable_text(extractor: PDFExtractor, searchable_pdf: Path) -> None:
    """Test extracting text from a searchable PDF."""
    result = await extractor._extract_pdf_searchable_text(searchable_pdf)
    assert isinstance(result, str)
    assert result.strip()


@pytest.mark.anyio
async def test_extract_pdf_searchable_not_fallback_to_ocr(test_contract: Path) -> None:
    extractor = PDFExtractor(mime_type="application/pdf", config=ExtractionConfig(force_ocr=False))
    result = await extractor.extract_path_async(test_contract)
    assert result.content.startswith(
        "Page 1 Sample Contract Contract No.___________ PROFESSIONAL SERVICES AGREEMENT THIS AGREEMENT made and entered into this"
    )


@pytest.mark.anyio
async def test_extract_pdf_text_with_ocr(extractor: PDFExtractor, scanned_pdf: Path) -> None:
    """Test extracting text from a scanned PDF using OCR."""
    result = await extractor._extract_pdf_text_with_ocr(scanned_pdf)
    assert isinstance(result, ExtractionResult)
    assert result.content.strip()


@pytest.mark.anyio
async def test_extract_pdf_file(extractor: PDFExtractor, searchable_pdf: Path) -> None:
    """Test extracting text from a PDF file."""
    result = await extractor.extract_path_async(searchable_pdf)
    assert isinstance(result.content, str)
    assert result.content.strip()
    assert result.mime_type == "text/plain"


@pytest.mark.anyio
async def test_extract_pdf_file_non_searchable(extractor: PDFExtractor, non_searchable_pdf: Path) -> None:
    """Test extracting text from a non-searchable PDF file."""
    result = await extractor.extract_path_async(non_searchable_pdf)
    assert isinstance(result.content, str)
    assert result.content.strip()
    assert result.mime_type == "text/plain"


@pytest.mark.anyio
async def test_extract_pdf_file_invalid(extractor: PDFExtractor) -> None:
    """Test that attempting to extract from an invalid PDF raises an error."""
    with pytest.raises(FileNotFoundError):
        await extractor.extract_path_async(Path("/invalid/path.pdf"))


@pytest.mark.anyio
async def test_convert_pdf_to_images_raises_parsing_error(extractor: PDFExtractor, tmp_path: Path) -> None:
    """Test that attempting to convert an invalid PDF to images raises a ParsingError."""
    pdf_path = tmp_path / "invalid.pdf"
    pdf_path.write_text("invalid pdf content")

    with pytest.raises(ParsingError) as exc_info:
        await extractor._convert_pdf_to_images(pdf_path)

    assert "Could not convert PDF to images" in str(exc_info.value)
    assert str(pdf_path) in str(exc_info.value.context["file_path"])


@pytest.mark.anyio
async def test_extract_pdf_searchable_text_raises_parsing_error(extractor: PDFExtractor, tmp_path: Path) -> None:
    """Test that attempting to extract text from an invalid PDF raises a ParsingError."""
    pdf_path = tmp_path / "invalid.pdf"
    pdf_path.write_text("invalid pdf content")

    with pytest.raises(ParsingError) as exc_info:
        await extractor._extract_pdf_searchable_text(pdf_path)

    assert "Could not extract text from PDF file" in str(exc_info.value)
    assert str(pdf_path) in str(exc_info.value.context["file_path"])


def test_validate_empty_text(extractor: PDFExtractor) -> None:
    """Test that empty text is considered invalid."""
    assert not extractor._validate_extracted_text("")
    assert not extractor._validate_extracted_text("   ")
    assert not extractor._validate_extracted_text("\n\n")


def test_validate_normal_text(extractor: PDFExtractor) -> None:
    """Test that normal text passes validation."""
    assert extractor._validate_extracted_text("Hello World!")
    assert extractor._validate_extracted_text("Line 1\nLine 2")
    assert extractor._validate_extracted_text(" 2024 Company")
    assert extractor._validate_extracted_text("Special chars: !@#$%^&*()")
    assert extractor._validate_extracted_text("""
        This is a normal paragraph of text that should pass validation.
        It contains normal punctuation, numbers (123), and symbols (!@#$%).
        Even with multiple paragraphs and line breaks, it should be fine.
    """)


def test_validate_short_corrupted_text(extractor: PDFExtractor) -> None:
    """Test validation of short text with corruption matches."""
    # Test text shorter than SHORT_TEXT_THRESHOLD with corruption matches
    assert not extractor._validate_extracted_text("\x00\x00\x00")  # Three null bytes (> MINIMUM_CORRUPTED_RESULTS)
    assert extractor._validate_extracted_text("Hi\x00\x00")  # Two null bytes (= MINIMUM_CORRUPTED_RESULTS)
    assert extractor._validate_extracted_text("Hi\x00")  # One null byte (< MINIMUM_CORRUPTED_RESULTS)
    assert extractor._validate_extracted_text("Short \ufffd")  # One replacement char (< MINIMUM_CORRUPTED_RESULTS)


def test_validate_long_corrupted_text(extractor: PDFExtractor) -> None:
    """Test validation of long text with corruption threshold."""
    # Create a long text with varying levels of corruption
    base_text = "A" * 1000  # Long text to exceed SHORT_TEXT_THRESHOLD

    # Test with corruption below threshold (5%)
    text_low_corruption = base_text + ("\x00" * 40)  # 4% corruption
    assert extractor._validate_extracted_text(text_low_corruption)

    # Test with corruption above threshold (5%)
    text_high_corruption = base_text + ("\x00" * 60)  # 6% corruption
    assert not extractor._validate_extracted_text(text_high_corruption)


def test_validate_custom_corruption_threshold(extractor: PDFExtractor) -> None:
    """Test validation with custom corruption threshold."""
    base_text = "A" * 1000
    corrupted_chars = "\x00" * 100  # 10% corruption
    text = base_text + corrupted_chars

    # Should fail with default threshold (5%)
    assert not extractor._validate_extracted_text(text)

    # Should pass with higher threshold (15%)
    assert extractor._validate_extracted_text(text, corruption_threshold=0.15)

    # Should fail with lower threshold (3%)
    assert not extractor._validate_extracted_text(text, corruption_threshold=0.03)

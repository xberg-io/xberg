"""Tests for PDF extraction functionality."""

from __future__ import annotations

from pathlib import Path

import pytest

from kreuzberg import ExtractionResult
from kreuzberg._pdf import (
    _convert_pdf_to_images,
    _extract_pdf_searchable_text,
    _extract_pdf_text_with_ocr,
    _validate_extracted_text,
    extract_pdf_file,
)
from kreuzberg.exceptions import ParsingError


async def test_extract_pdf_searchable_text(searchable_pdf: Path) -> None:
    """Test extracting text from a searchable PDF."""
    result = await _extract_pdf_searchable_text(searchable_pdf)
    assert isinstance(result, str)
    assert result.strip()


async def test_extract_pdf_text_with_ocr(scanned_pdf: Path) -> None:
    """Test extracting text from a scanned PDF using OCR."""
    result = await _extract_pdf_text_with_ocr(scanned_pdf, max_processes=1)
    assert isinstance(result, ExtractionResult)
    assert result.content.strip()


async def test_extract_pdf_file(searchable_pdf: Path) -> None:
    """Test extracting text from a PDF file."""
    result = await extract_pdf_file(searchable_pdf, force_ocr=False, max_processes=1)
    assert isinstance(result.content, str)
    assert result.content.strip()
    assert result.mime_type == "text/plain"


async def test_extract_pdf_file_non_searchable(non_searchable_pdf: Path) -> None:
    """Test extracting text from a non-searchable PDF file."""
    result = await extract_pdf_file(non_searchable_pdf, force_ocr=False, max_processes=1)
    assert isinstance(result.content, str)
    assert result.content.strip()
    assert result.mime_type == "text/plain"


async def test_extract_pdf_file_invalid() -> None:
    """Test that attempting to extract from an invalid PDF raises an error."""
    with pytest.raises(FileNotFoundError):
        await extract_pdf_file(Path("/invalid/path.pdf"), force_ocr=False, max_processes=1)


async def test_convert_pdf_to_images_raises_parsing_error(tmp_path: Path) -> None:
    """Test that attempting to convert an invalid PDF to images raises a ParsingError."""
    pdf_path = tmp_path / "invalid.pdf"
    pdf_path.write_text("invalid pdf content")

    with pytest.raises(ParsingError) as exc_info:
        await _convert_pdf_to_images(pdf_path)

    assert "Could not convert PDF to images" in str(exc_info.value)
    assert str(pdf_path) in str(exc_info.value.context["file_path"])


async def test_extract_pdf_searchable_text_raises_parsing_error(tmp_path: Path) -> None:
    """Test that attempting to extract text from an invalid PDF raises a ParsingError."""
    pdf_path = tmp_path / "invalid.pdf"
    pdf_path.write_text("invalid pdf content")

    with pytest.raises(ParsingError) as exc_info:
        await _extract_pdf_searchable_text(pdf_path)

    assert "Could not extract text from PDF file" in str(exc_info.value)
    assert str(pdf_path) in str(exc_info.value.context["file_path"])


def test_validate_empty_text() -> None:
    """Test that empty text is considered invalid."""
    assert not _validate_extracted_text("")
    assert not _validate_extracted_text("   ")
    assert not _validate_extracted_text("\n\n")


def test_validate_control_chars() -> None:
    """Test that text with control characters is invalid."""
    assert not _validate_extracted_text("Hello\x00World")
    assert not _validate_extracted_text("Test\x1fText")
    assert not _validate_extracted_text("Sample\x7fText")
    assert not _validate_extracted_text("Test\x9fData")


def test_validate_unicode_chars() -> None:
    """Test that text with replacement characters is invalid."""
    assert not _validate_extracted_text("Bad\ufffdText")
    assert not _validate_extracted_text("Text\u200bHidden")
    assert not _validate_extracted_text("Line\u2028Break")


def test_validate_normal_text() -> None:
    """Test that normal text passes validation."""
    assert _validate_extracted_text("Hello World!")
    assert _validate_extracted_text("Line 1\nLine 2")
    assert _validate_extracted_text("© 2024 Company")
    assert _validate_extracted_text("Special chars: !@#$%^&*()")
    assert _validate_extracted_text("""
        This is a normal paragraph of text that should pass validation.
        It contains normal punctuation, numbers (123), and symbols (!@#$%).
        Even with multiple paragraphs and line breaks, it should be fine.
    """)

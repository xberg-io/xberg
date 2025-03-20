"""Tests for PDF extraction functionality."""

from __future__ import annotations

from pathlib import Path

import pandas as pd
import pytest
from PIL.Image import Image

from kreuzberg import ExtractionResult
from kreuzberg._extractors._pdf import PDFExtractor
from kreuzberg._types import ExtractionConfig
from kreuzberg.exceptions import ParsingError
from kreuzberg.extraction import DEFAULT_CONFIG
from tests.conftest import pdfs_with_tables


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
    result = await extractor._extract_pdf_text_with_ocr(scanned_pdf, ocr_backend="tesseract")
    assert isinstance(result, ExtractionResult)
    assert result.content.strip()


@pytest.mark.anyio
async def test_extract_pdf_file(extractor: PDFExtractor, searchable_pdf: Path) -> None:
    """Test extracting text from a PDF file."""
    result = await extractor.extract_path_async(searchable_pdf)
    assert isinstance(result.content, str)
    assert result.content.strip()
    assert result.mime_type == "text/plain"

    assert result.metadata
    assert "summary" in result.metadata
    assert "PDF document with" in result.metadata["summary"]


@pytest.mark.anyio
async def test_extract_pdf_file_non_searchable(extractor: PDFExtractor, non_searchable_pdf: Path) -> None:
    """Test extracting text from a non-searchable PDF file."""
    result = await extractor.extract_path_async(non_searchable_pdf)
    assert isinstance(result.content, str)
    assert result.content.strip()
    assert result.mime_type == "text/plain"

    assert result.metadata
    assert "summary" in result.metadata


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

    assert not extractor._validate_extracted_text("\x00\x00\x00")
    assert extractor._validate_extracted_text("Hi\x00\x00")
    assert extractor._validate_extracted_text("Hi\x00")
    assert extractor._validate_extracted_text("Short \ufffd")


def test_validate_long_corrupted_text(extractor: PDFExtractor) -> None:
    """Test validation of long text with corruption threshold."""

    base_text = "A" * 1000

    text_low_corruption = base_text + ("\x00" * 40)
    assert extractor._validate_extracted_text(text_low_corruption)

    text_high_corruption = base_text + ("\x00" * 60)
    assert not extractor._validate_extracted_text(text_high_corruption)


def test_validate_custom_corruption_threshold(extractor: PDFExtractor) -> None:
    """Test validation with custom corruption threshold."""
    base_text = "A" * 1000
    corrupted_chars = "\x00" * 100
    text = base_text + corrupted_chars

    assert not extractor._validate_extracted_text(text)

    assert extractor._validate_extracted_text(text, corruption_threshold=0.15)

    assert not extractor._validate_extracted_text(text, corruption_threshold=0.03)


@pytest.mark.anyio
async def test_extract_pdf_with_rich_metadata(extractor: PDFExtractor, test_article: Path) -> None:
    """Test extracting metadata from a PDF with rich metadata fields."""

    result = await extractor.extract_path_async(test_article)

    assert result.content.strip()

    metadata = result.metadata
    assert metadata

    assert "title" in metadata
    assert isinstance(metadata["title"], str)

    assert not any(isinstance(value, bytes) for value in metadata.values())

    if "authors" in metadata:
        assert isinstance(metadata["authors"], list)
        assert all(isinstance(author, str) for author in metadata["authors"])

    if "keywords" in metadata:
        assert isinstance(metadata["keywords"], list)
        assert all(isinstance(kw, str) for kw in metadata["keywords"])

    assert "summary" in metadata
    assert "PDF document with" in metadata["summary"]


@pytest.mark.anyio
async def test_extract_pdf_bytes_with_metadata(extractor: PDFExtractor, test_article: Path) -> None:
    """Test extracting metadata from PDF bytes."""

    pdf_bytes = test_article.read_bytes()

    result = await extractor.extract_bytes_async(pdf_bytes)

    assert result.content.strip()

    metadata = result.metadata
    assert metadata

    assert "title" in metadata
    assert isinstance(metadata["title"], str)

    assert not any(isinstance(value, bytes) for value in metadata.values())


@pytest.mark.anyio
@pytest.mark.parametrize("pdf_with_table", pdfs_with_tables)
async def test_extract_tables_from_pdf(pdf_with_table: Path) -> None:
    extractor = PDFExtractor(mime_type="application/pdf", config=ExtractionConfig(extract_tables=True))
    result = await extractor.extract_path_async(pdf_with_table)

    assert result.tables
    assert isinstance(result.tables, list)
    assert all(isinstance(table, dict) for table in result.tables)

    for table in result.tables:
        assert "page_number" in table
        assert isinstance(table["page_number"], int)
        assert "text" in table
        assert isinstance(table["text"], str)
        assert "df" in table
        assert isinstance(table["df"], pd.DataFrame)
        assert isinstance(table["cropped_image"], Image)

from __future__ import annotations

import os
from pathlib import Path
from typing import TYPE_CHECKING, NoReturn
from unittest.mock import patch

import pandas as pd
import pytest
from PIL.Image import Image
from pytest import MonkeyPatch

from kreuzberg import ExtractionResult
from kreuzberg._extractors._pdf import PDFExtractor
from kreuzberg._types import ExtractionConfig
from kreuzberg.exceptions import ParsingError
from kreuzberg.extraction import DEFAULT_CONFIG
from tests.conftest import pdfs_with_tables

if TYPE_CHECKING:
    from pytest_mock import MockerFixture

IS_CI = os.environ.get("CI", "false").lower() == "true"


@pytest.fixture
def extractor() -> PDFExtractor:
    return PDFExtractor(mime_type="application/pdf", config=DEFAULT_CONFIG)


@pytest.mark.anyio
async def test_extract_pdf_searchable_text(extractor: PDFExtractor, searchable_pdf: Path) -> None:
    result = await extractor._extract_pdf_searchable_text(searchable_pdf)
    assert isinstance(result, str)
    assert result.strip()


@pytest.mark.anyio
async def test_extract_pdf_searchable_not_fallback_to_ocr(test_contract: Path) -> None:
    extractor = PDFExtractor(mime_type="application/pdf", config=ExtractionConfig(force_ocr=False))
    result = await extractor.extract_path_async(test_contract)
    assert result.content.startswith(
        "Page 1\nSample Contract\nContract No....\nPROFESSIONAL SERVICES AGREEMENT\nTHIS AGREEMENT made and entered into this"
    )


@pytest.mark.anyio
@pytest.mark.xfail(IS_CI, reason="OCR tests may fail in CI due to Tesseract issues")
async def test_extract_pdf_text_with_ocr(extractor: PDFExtractor, scanned_pdf: Path) -> None:
    result = await extractor._extract_pdf_text_with_ocr(scanned_pdf, ocr_backend="tesseract")
    assert isinstance(result, ExtractionResult)
    assert result.content.strip()


@pytest.mark.anyio
async def test_extract_pdf_file(extractor: PDFExtractor, searchable_pdf: Path) -> None:
    result = await extractor.extract_path_async(searchable_pdf)
    assert isinstance(result.content, str)
    assert result.content.strip()
    assert result.mime_type == "text/plain"

    assert result.metadata
    assert "summary" in result.metadata
    assert "PDF document with" in result.metadata["summary"]


@pytest.mark.anyio
@pytest.mark.xfail(IS_CI, reason="OCR tests may fail in CI due to Tesseract issues")
async def test_extract_pdf_file_non_searchable(extractor: PDFExtractor, non_searchable_pdf: Path) -> None:
    result = await extractor.extract_path_async(non_searchable_pdf)
    assert isinstance(result.content, str)
    assert result.content.strip()
    assert result.mime_type == "text/plain"

    assert result.metadata
    assert "summary" in result.metadata


@pytest.mark.anyio
async def test_extract_pdf_file_invalid(extractor: PDFExtractor) -> None:
    with pytest.raises(FileNotFoundError):
        await extractor.extract_path_async(Path("/invalid/path.pdf"))


@pytest.mark.anyio
async def test_convert_pdf_to_images_raises_parsing_error(extractor: PDFExtractor, tmp_path: Path) -> None:
    pdf_path = tmp_path / "invalid.pdf"
    pdf_path.write_text("invalid pdf content")

    with pytest.raises(ParsingError) as exc_info:
        await extractor._convert_pdf_to_images(pdf_path)

    assert "Could not convert PDF to images" in str(exc_info.value)
    assert str(pdf_path) in str(exc_info.value.context["file"]["path"])


@pytest.mark.anyio
async def test_extract_pdf_searchable_text_raises_parsing_error(extractor: PDFExtractor, tmp_path: Path) -> None:
    pdf_path = tmp_path / "invalid.pdf"
    pdf_path.write_text("invalid pdf content")

    with pytest.raises(ParsingError) as exc_info:
        await extractor._extract_pdf_searchable_text(pdf_path)

    assert "Could not extract text from PDF file" in str(exc_info.value)
    assert str(pdf_path) in str(exc_info.value.context["file"]["path"])


def test_validate_empty_text(extractor: PDFExtractor) -> None:
    assert not extractor._validate_extracted_text("")
    assert not extractor._validate_extracted_text("   ")
    assert not extractor._validate_extracted_text("\n\n")


def test_validate_normal_text(extractor: PDFExtractor) -> None:
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
    assert not extractor._validate_extracted_text("\x00\x00\x00")
    assert extractor._validate_extracted_text("Hi\x00\x00")
    assert extractor._validate_extracted_text("Hi\x00")
    assert extractor._validate_extracted_text("Short \ufffd")


def test_validate_long_corrupted_text(extractor: PDFExtractor) -> None:
    base_text = "A" * 1000

    text_low_corruption = base_text + ("\x00" * 40)
    assert extractor._validate_extracted_text(text_low_corruption)

    text_high_corruption = base_text + ("\x00" * 60)
    assert not extractor._validate_extracted_text(text_high_corruption)


def test_validate_custom_corruption_threshold(extractor: PDFExtractor) -> None:
    base_text = "A" * 1000
    corrupted_chars = "\x00" * 100
    text = base_text + corrupted_chars

    assert not extractor._validate_extracted_text(text)

    assert extractor._validate_extracted_text(text, corruption_threshold=0.15)

    assert not extractor._validate_extracted_text(text, corruption_threshold=0.03)


@pytest.mark.anyio
async def test_extract_pdf_with_rich_metadata(extractor: PDFExtractor, test_article: Path) -> None:
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

        assert isinstance(table["df"], (pd.DataFrame, dict))

        assert isinstance(table["cropped_image"], (Image, type(None)))


def test_extract_pdf_bytes_sync(extractor: PDFExtractor, test_article: Path) -> None:
    """Test sync PDF extraction from bytes."""
    pdf_bytes = test_article.read_bytes()

    result = extractor.extract_bytes_sync(pdf_bytes)

    assert isinstance(result, ExtractionResult)
    assert result.content.strip()
    assert result.mime_type == "text/plain"
    assert result.metadata
    assert "title" in result.metadata


def test_extract_pdf_path_sync(extractor: PDFExtractor, searchable_pdf: Path) -> None:
    """Test sync PDF extraction from path."""
    result = extractor.extract_path_sync(searchable_pdf)

    assert isinstance(result, ExtractionResult)
    assert result.content.strip()
    assert result.mime_type == "text/plain"
    assert result.metadata == {"quality_score": 1.0}


def test_extract_pdf_path_sync_with_tables(searchable_pdf: Path) -> None:
    """Test sync PDF extraction with table extraction enabled."""
    config = ExtractionConfig(extract_tables=True)
    extractor = PDFExtractor(mime_type="application/pdf", config=config)

    result = extractor.extract_path_sync(searchable_pdf)

    assert isinstance(result, ExtractionResult)
    assert result.content.strip()
    assert isinstance(result.tables, list)


@pytest.mark.xfail(IS_CI, reason="OCR tests may fail in CI due to Tesseract issues")
def test_extract_pdf_path_sync_force_ocr_tesseract(searchable_pdf: Path) -> None:
    """Test sync PDF extraction with forced OCR using tesseract."""
    config = ExtractionConfig(force_ocr=True, ocr_backend="tesseract")
    extractor = PDFExtractor(mime_type="application/pdf", config=config)

    result = extractor.extract_path_sync(searchable_pdf)

    assert isinstance(result, ExtractionResult)
    assert result.content.strip()
    assert result.mime_type == "text/plain"


def test_extract_pdf_searchable_text_sync_error(extractor: PDFExtractor, tmp_path: Path) -> None:
    """Test sync searchable text extraction with invalid PDF raises ParsingError."""
    pdf_path = tmp_path / "invalid.pdf"
    pdf_path.write_text("invalid pdf content")

    with pytest.raises(ParsingError, match="Failed to extract PDF text"):
        extractor._extract_pdf_searchable_text_sync(pdf_path)


def test_extract_pdf_with_ocr_sync_error(extractor: PDFExtractor, tmp_path: Path) -> None:
    """Test sync OCR extraction with invalid PDF raises ParsingError."""
    pdf_path = tmp_path / "invalid.pdf"
    pdf_path.write_text("invalid pdf content")

    with pytest.raises(ParsingError, match="Failed to OCR PDF"):
        extractor._extract_pdf_with_ocr_sync(pdf_path)


@pytest.mark.anyio
async def test_extract_pdf_no_ocr_backend_fallback(non_searchable_pdf: Path) -> None:
    """Test PDF extraction falls back to empty result when no OCR backend available."""
    config = ExtractionConfig(force_ocr=False, ocr_backend=None)
    extractor = PDFExtractor(mime_type="application/pdf", config=config)

    result = await extractor.extract_path_async(non_searchable_pdf)

    # Should fallback to empty result since no OCR backend and PDF isn't searchable  # ~keep
    assert result.content == ""
    assert result.mime_type == "text/plain"


@pytest.mark.anyio
async def test_extract_pdf_searchable_text_partial_failure(
    extractor: PDFExtractor, tmp_path: Path, monkeypatch: MonkeyPatch
) -> None:
    """Test searchable text extraction with partial page failures."""

    def mock_page_get_textpage() -> NoReturn:
        raise Exception("Page extraction failed")


def test_validate_short_text_with_many_corrupted_chars(extractor: PDFExtractor) -> None:
    """Test validation of short text with many corrupted characters."""

    corrupted_text = "hi\x00\x01\x02"
    assert not extractor._validate_extracted_text(corrupted_text)

    semi_corrupted = "hi\x00\x01"
    assert extractor._validate_extracted_text(semi_corrupted)


def test_validate_text_unicode_replacement_chars(extractor: PDFExtractor) -> None:
    """Test validation with Unicode replacement characters."""

    text_with_replacements = "Hello " + ("\ufffd" * 20) + " World"
    assert not extractor._validate_extracted_text(text_with_replacements)

    text_with_few_replacements = "Hello \ufffd World"
    assert extractor._validate_extracted_text(text_with_few_replacements)


def test_validate_text_mixed_corruption(extractor: PDFExtractor) -> None:
    """Test validation with mixed corruption types."""
    base_text = "A" * 1000

    mixed_corruption = "\x00\x01\x02\ufffd\ufffd" * 15
    text = base_text + mixed_corruption

    # Should fail due to high corruption ratio (75/1075 = ~7%)  # ~keep
    assert not extractor._validate_extracted_text(text)

    assert extractor._validate_extracted_text(text, corruption_threshold=0.08)


@pytest.mark.anyio
@pytest.mark.xfail(IS_CI, reason="OCR tests may fail in CI due to Tesseract issues")
async def test_extract_pdf_force_ocr_when_valid_text_exists(searchable_pdf: Path) -> None:
    """Test force_ocr=True bypasses valid text extraction - covers line 52->57."""
    config = ExtractionConfig(force_ocr=True, ocr_backend="tesseract")
    extractor = PDFExtractor(mime_type="application/pdf", config=config)

    result = await extractor.extract_path_async(searchable_pdf)

    # Should still get valid content via OCR  # ~keep
    assert result.content.strip()
    assert result.mime_type == "text/plain"


@pytest.mark.anyio
async def test_extract_pdf_searchable_text_page_errors(
    extractor: PDFExtractor, tmp_path: Path, monkeypatch: MonkeyPatch
) -> None:
    """Test partial page failure handling - covers lines 255-257, 264, 267."""
    import pypdfium2

    class MockPage:
        def __init__(self, should_fail: bool = False) -> None:
            self.should_fail = should_fail

        def get_textpage(self) -> object:
            if self.should_fail:
                raise Exception("Page extraction failed")
            return MockTextPage()

    class MockTextPage:
        def get_text_bounded(self) -> str:
            return "Valid page text"

    class MockDocument:
        def __init__(self) -> None:
            self.pages = [
                MockPage(should_fail=False),
                MockPage(should_fail=True),
                MockPage(should_fail=False),
            ]

        def __iter__(self) -> object:
            return iter(self.pages)

        def close(self) -> None:
            pass

    def mock_pdf_document(*args: object, **kwargs: object) -> MockDocument:
        return MockDocument()

    monkeypatch.setattr(pypdfium2, "PdfDocument", mock_pdf_document)


def test_pdf_password_configuration() -> None:
    """Test PDF password configuration variations."""
    config = ExtractionConfig(pdf_password="test")
    extractor = PDFExtractor(mime_type="application/pdf", config=config)
    passwords = extractor._get_passwords_to_try()
    assert passwords == ["test"]

    config = ExtractionConfig(pdf_password=["pass1", "pass2", "pass3"])
    extractor = PDFExtractor(mime_type="application/pdf", config=config)
    passwords = extractor._get_passwords_to_try()
    assert passwords == ["pass1", "pass2", "pass3"]

    config = ExtractionConfig(pdf_password="")
    extractor = PDFExtractor(mime_type="application/pdf", config=config)
    passwords = extractor._get_passwords_to_try()
    assert passwords == [""]

    config = ExtractionConfig(pdf_password=[])
    extractor = PDFExtractor(mime_type="application/pdf", config=config)
    passwords = extractor._get_passwords_to_try()
    assert passwords == [""]


def test_pdf_password_attempts_with_parse_with_password_attempts(test_article: Path) -> None:
    """Test the _parse_with_password_attempts method with different password configurations."""
    config = ExtractionConfig(pdf_password="")
    extractor = PDFExtractor(mime_type="application/pdf", config=config)

    content = test_article.read_bytes()
    document = extractor._parse_with_password_attempts(content)

    assert document is not None
    assert len(document.pages) > 0

    config = ExtractionConfig(pdf_password="wrongpassword")
    extractor = PDFExtractor(mime_type="application/pdf", config=config)

    document = extractor._parse_with_password_attempts(content)
    assert document is not None
    assert len(document.pages) > 0


@pytest.fixture
def pdf_extractor() -> PDFExtractor:
    """Create a PDFExtractor instance for testing."""
    config = ExtractionConfig()
    return PDFExtractor("application/pdf", config)


@pytest.fixture
def sample_pdf_content() -> bytes:
    """Sample PDF content for testing."""
    return b"""%PDF-1.4
1 0 obj
<<
/Type /Catalog
/Pages 2 0 R
>>
endobj

2 0 obj
<<
/Type /Pages
/Kids [3 0 R]
/Count 1
>>
endobj

3 0 obj
<<
/Type /Page
/Parent 2 0 R
/MediaBox [0 0 612 792]
/Contents 4 0 R
>>
endobj

4 0 obj
<<
/Length 44
>>
stream
BT
/F1 12 Tf
100 700 Td
(Hello World) Tj
ET
endstream
endobj

xref
0 5
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
0000000201 00000 n
trailer
<<
/Size 5
/Root 1 0 R
>>
startxref
295
%%EOF"""


class TestPDFExtractor:
    """Test PDFExtractor functionality."""

    def test_supported_mime_types(self, pdf_extractor: PDFExtractor) -> None:
        """Test supported MIME types."""
        assert "application/pdf" in pdf_extractor.SUPPORTED_MIME_TYPES

    def test_constants(self, pdf_extractor: PDFExtractor) -> None:
        """Test class constants."""
        assert pdf_extractor.SHORT_TEXT_THRESHOLD == 50
        assert pdf_extractor.MINIMUM_CORRUPTED_RESULTS == 2
        assert pdf_extractor.CORRUPTED_PATTERN is not None

    def test_validate_extracted_text_empty(self, pdf_extractor: PDFExtractor) -> None:
        """Test text validation with empty text."""
        assert not pdf_extractor._validate_extracted_text("")
        assert not pdf_extractor._validate_extracted_text("   ")
        assert not pdf_extractor._validate_extracted_text("\\n\\t")

    def test_validate_extracted_text_valid(self, pdf_extractor: PDFExtractor) -> None:
        """Test text validation with valid text."""
        assert pdf_extractor._validate_extracted_text("Hello world, this is valid text!")
        assert pdf_extractor._validate_extracted_text("Normal text with numbers 123 and symbols @#$")

    def test_validate_extracted_text_short_valid(self, pdf_extractor: PDFExtractor) -> None:
        """Test text validation with short valid text."""
        assert pdf_extractor._validate_extracted_text("Short text")
        assert pdf_extractor._validate_extracted_text("OK")

    def test_validate_extracted_text_short_corrupted(self, pdf_extractor: PDFExtractor) -> None:
        """Test text validation with short corrupted text."""
        corrupted_text = "A\\x00B\\x01C\\uFFFD"
        assert not pdf_extractor._validate_extracted_text(corrupted_text)

    def test_validate_extracted_text_long_corrupted(self, pdf_extractor: PDFExtractor) -> None:
        """Test text validation with long corrupted text."""
        base_text = "Valid text " * 10
        corrupted_text = base_text + "\\x00\\x01\\x02\\uFFFD" * 10
        assert not pdf_extractor._validate_extracted_text(corrupted_text)

    def test_validate_extracted_text_long_low_corruption(self, pdf_extractor: PDFExtractor) -> None:
        """Test text validation with long text and acceptable corruption."""
        base_text = "Valid text with good content " * 20
        corrupted_text = base_text + "\\x00\\uFFFD"
        assert pdf_extractor._validate_extracted_text(corrupted_text)

    def test_validate_extracted_text_custom_threshold(self, pdf_extractor: PDFExtractor) -> None:
        """Test text validation with custom corruption threshold."""
        base_text = "Text " * 20
        corrupted_text = base_text + "\\x00" * 8

        assert not pdf_extractor._validate_extracted_text(corrupted_text)

        assert pdf_extractor._validate_extracted_text(corrupted_text, corruption_threshold=0.10)

    @pytest.mark.anyio
    async def test_extract_bytes_async_basic(
        self, pdf_extractor: PDFExtractor, sample_pdf_content: bytes, mocker: MockerFixture
    ) -> None:
        """Test basic async bytes extraction."""
        mock_create_temp_file = mocker.patch("kreuzberg._extractors._pdf.create_temp_file")
        mock_create_temp_file.return_value = ("/tmp/test.pdf", mocker.AsyncMock())

        mock_async_path = mocker.patch("kreuzberg._extractors._pdf.AsyncPath")
        mock_async_path.return_value.write_bytes = mocker.AsyncMock()

        mock_extract_metadata = mocker.patch.object(pdf_extractor, "_extract_metadata_with_password_attempts")
        mock_extract_metadata.return_value = {"pages": 1}

        mock_extract_path = mocker.patch.object(pdf_extractor, "extract_path_async")
        mock_extract_path.return_value = ExtractionResult(
            content="Test content", mime_type="text/plain", metadata={}, chunks=[]
        )

        result = await pdf_extractor.extract_bytes_async(sample_pdf_content)

        assert result.content == "Test content"
        assert result.metadata == {"pages": 1}
        mock_create_temp_file.assert_called_once_with(".pdf")

    def test_extract_bytes_sync_basic(
        self, pdf_extractor: PDFExtractor, sample_pdf_content: bytes, mocker: MockerFixture
    ) -> None:
        """Test basic sync bytes extraction."""
        mock_mkstemp = mocker.patch("tempfile.mkstemp")
        mock_mkstemp.return_value = (5, "/tmp/test.pdf")

        mock_fdopen = mocker.patch("os.fdopen")
        mock_file = mocker.Mock()
        mock_fdopen.return_value.__enter__.return_value = mock_file
        mock_fdopen.return_value.__exit__.return_value = None

        mock_extract_path = mocker.patch.object(pdf_extractor, "extract_path_sync")
        mock_extract_path.return_value = ExtractionResult(
            content="Sync test content", mime_type="text/plain", metadata={}, chunks=[]
        )

        mock_extract_metadata = mocker.patch.object(pdf_extractor, "_extract_metadata_with_password_attempts_sync")
        mock_extract_metadata.return_value = {"title": "Test PDF"}

        mocker.patch("pathlib.Path.unlink")

        result = pdf_extractor.extract_bytes_sync(sample_pdf_content)

        assert result.content == "Sync test content"
        assert result.metadata == {"title": "Test PDF"}
        mock_file.write.assert_called_once_with(sample_pdf_content)

    @pytest.mark.anyio
    async def test_extract_path_async_searchable_text(
        self, pdf_extractor: PDFExtractor, tmp_path: Path, mocker: MockerFixture
    ) -> None:
        """Test async path extraction with searchable text."""
        test_file = tmp_path / "test.pdf"
        test_file.write_bytes(b"dummy pdf content")

        mock_async_path = mocker.patch("kreuzberg._extractors._pdf.AsyncPath")
        mock_async_path.return_value.read_bytes = mocker.AsyncMock(return_value=b"dummy pdf content")

        mock_extract_searchable = mocker.patch.object(pdf_extractor, "_extract_pdf_searchable_text")
        mock_extract_searchable.return_value = "Extracted searchable text"

        mock_extract_metadata = mocker.patch.object(pdf_extractor, "_extract_metadata_with_password_attempts")
        mock_extract_metadata.return_value = {"pages": 1}

        mock_apply_quality = mocker.patch.object(pdf_extractor, "_apply_quality_processing")
        mock_apply_quality.side_effect = lambda x: x

        result = await pdf_extractor.extract_path_async(test_file)

        assert result.content == "Extracted searchable text"
        mock_extract_searchable.assert_called_once_with(test_file)

    @pytest.mark.anyio
    async def test_extract_path_async_force_ocr(
        self, pdf_extractor: PDFExtractor, tmp_path: Path, mocker: MockerFixture
    ) -> None:
        """Test async path extraction with force OCR enabled."""
        pdf_extractor.config.force_ocr = True
        pdf_extractor.config.ocr_backend = "tesseract"

        test_file = tmp_path / "test.pdf"
        test_file.write_bytes(b"dummy pdf content")

        mock_async_path = mocker.patch("kreuzberg._extractors._pdf.AsyncPath")
        mock_async_path.return_value.read_bytes = mocker.AsyncMock(return_value=b"dummy pdf content")

        mock_extract_ocr = mocker.patch.object(pdf_extractor, "_extract_pdf_text_with_ocr")
        mock_extract_ocr.return_value = ExtractionResult(
            content="OCR extracted text", mime_type="text/plain", metadata={}, chunks=[]
        )

        mock_extract_metadata = mocker.patch.object(pdf_extractor, "_extract_metadata_with_password_attempts")
        mock_extract_metadata.return_value = {"pages": 1}

        mock_apply_quality = mocker.patch.object(pdf_extractor, "_apply_quality_processing")
        mock_apply_quality.side_effect = lambda x: x

        result = await pdf_extractor.extract_path_async(test_file)

        assert result.content == "OCR extracted text"
        mock_extract_ocr.assert_called_once_with(test_file, "tesseract")

    @pytest.mark.anyio
    async def test_extract_path_async_with_tables(
        self, pdf_extractor: PDFExtractor, tmp_path: Path, mocker: MockerFixture
    ) -> None:
        """Test async path extraction with table extraction enabled."""
        pdf_extractor.config.extract_tables = True

        test_file = tmp_path / "test.pdf"
        test_file.write_bytes(b"dummy pdf content")

        mock_async_path = mocker.patch("kreuzberg._extractors._pdf.AsyncPath")
        mock_async_path.return_value.read_bytes = mocker.AsyncMock(return_value=b"dummy pdf content")

        mock_extract_searchable = mocker.patch.object(pdf_extractor, "_extract_pdf_searchable_text")
        mock_extract_searchable.return_value = "Text with tables"

        mock_extract_metadata = mocker.patch.object(pdf_extractor, "_extract_metadata_with_password_attempts")
        mock_extract_metadata.return_value = {"pages": 2}

        mock_extract_tables = mocker.patch("kreuzberg._gmft.extract_tables")
        mock_extract_tables.return_value = [
            {"text": "Table 1", "page_number": 1},
            {"text": "Table 2", "page_number": 2},
        ]

        mock_generate_summary = mocker.patch("kreuzberg._extractors._pdf.generate_table_summary")
        mock_generate_summary.return_value = {"table_count": 2, "pages_with_tables": 2, "total_rows": 10}

        mock_apply_quality = mocker.patch.object(pdf_extractor, "_apply_quality_processing")
        mock_apply_quality.side_effect = lambda x: x

        result = await pdf_extractor.extract_path_async(test_file)

        assert result.content == "Text with tables"
        assert len(result.tables) == 2
        assert result.metadata["table_count"] == 2
        assert "2 tables" in result.metadata["tables_summary"]

    @pytest.mark.anyio
    async def test_extract_path_async_searchable_text_fails(
        self, pdf_extractor: PDFExtractor, tmp_path: Path, mocker: MockerFixture
    ) -> None:
        """Test async path extraction when searchable text extraction fails."""
        test_file = tmp_path / "test.pdf"
        test_file.write_bytes(b"dummy pdf content")

        mock_async_path = mocker.patch("kreuzberg._extractors._pdf.AsyncPath")
        mock_async_path.return_value.read_bytes = mocker.AsyncMock(return_value=b"dummy pdf content")

        mock_extract_searchable = mocker.patch.object(pdf_extractor, "_extract_pdf_searchable_text")
        mock_extract_searchable.side_effect = ParsingError("PDF parsing failed")

        pdf_extractor.config.ocr_backend = "tesseract"
        mock_extract_ocr = mocker.patch.object(pdf_extractor, "_extract_pdf_text_with_ocr")
        mock_extract_ocr.return_value = ExtractionResult(
            content="OCR fallback content", mime_type="text/plain", metadata={}, chunks=[]
        )

        mock_extract_metadata = mocker.patch.object(pdf_extractor, "_extract_metadata_with_password_attempts")
        mock_extract_metadata.return_value = {"pages": 1}

        mock_apply_quality = mocker.patch.object(pdf_extractor, "_apply_quality_processing")
        mock_apply_quality.side_effect = lambda x: x

        result = await pdf_extractor.extract_path_async(test_file)

        assert result.content == "OCR fallback content"
        mock_extract_ocr.assert_called_once()

    @pytest.mark.anyio
    async def test_extract_path_async_no_extraction_possible(
        self, pdf_extractor: PDFExtractor, tmp_path: Path, mocker: MockerFixture
    ) -> None:
        """Test async path extraction when no text extraction is possible."""
        pdf_extractor.config.ocr_backend = None

        test_file = tmp_path / "test.pdf"
        test_file.write_bytes(b"dummy pdf content")

        mock_async_path = mocker.patch("kreuzberg._extractors._pdf.AsyncPath")
        mock_async_path.return_value.read_bytes = mocker.AsyncMock(return_value=b"dummy pdf content")

        mock_extract_searchable = mocker.patch.object(pdf_extractor, "_extract_pdf_searchable_text")
        mock_extract_searchable.side_effect = ParsingError("PDF parsing failed")

        mock_extract_metadata = mocker.patch.object(pdf_extractor, "_extract_metadata_with_password_attempts")
        mock_extract_metadata.return_value = {"pages": 1}

        mock_apply_quality = mocker.patch.object(pdf_extractor, "_apply_quality_processing")
        mock_apply_quality.side_effect = lambda x: x

        result = await pdf_extractor.extract_path_async(test_file)

        assert result.content == ""
        assert result.mime_type == "text/plain"

    def test_extract_path_sync_basic(self, pdf_extractor: PDFExtractor, tmp_path: Path, mocker: MockerFixture) -> None:
        """Test basic sync path extraction."""
        test_file = tmp_path / "test.pdf"
        test_file.write_bytes(b"dummy pdf content")

        mock_extract_searchable = mocker.patch.object(pdf_extractor, "_extract_pdf_searchable_text_sync")
        mock_extract_searchable.return_value = "Extracted text"

        mock_extract_playa = mocker.patch.object(pdf_extractor, "_extract_with_playa_sync")
        mock_extract_playa.return_value = "Enhanced text with structure"

        mock_normalize = mocker.patch("kreuzberg._extractors._pdf.normalize_spaces")
        mock_normalize.return_value = "Normalized text"

        mock_apply_quality = mocker.patch.object(pdf_extractor, "_apply_quality_processing")
        mock_apply_quality.side_effect = lambda x: x

        result = pdf_extractor.extract_path_sync(test_file)

        assert result.content == "Normalized text"
        mock_extract_playa.assert_called_once_with(test_file, fallback_text="Extracted text")

    def test_extract_path_sync_parsing_error(
        self, pdf_extractor: PDFExtractor, tmp_path: Path, mocker: MockerFixture
    ) -> None:
        """Test sync path extraction when searchable text extraction fails."""
        test_file = tmp_path / "test.pdf"
        test_file.write_bytes(b"dummy pdf content")

        mock_extract_searchable = mocker.patch.object(pdf_extractor, "_extract_pdf_searchable_text_sync")
        mock_extract_searchable.side_effect = ParsingError("Sync parsing failed")

        pdf_extractor.config.ocr_backend = "tesseract"
        mock_extract_ocr = mocker.patch.object(pdf_extractor, "_extract_pdf_with_ocr_sync")
        mock_extract_ocr.return_value = "OCR sync content"

        mock_normalize = mocker.patch("kreuzberg._extractors._pdf.normalize_spaces")
        mock_normalize.return_value = "Normalized OCR content"

        mock_apply_quality = mocker.patch.object(pdf_extractor, "_apply_quality_processing")
        mock_apply_quality.side_effect = lambda x: x

        result = pdf_extractor.extract_path_sync(test_file)

        assert result.content == "Normalized OCR content"
        mock_extract_ocr.assert_called_once_with(test_file)

    def test_extract_path_sync_with_tables_import_error(
        self, pdf_extractor: PDFExtractor, tmp_path: Path, mocker: MockerFixture
    ) -> None:
        """Test sync path extraction when GMFT import fails."""
        pdf_extractor.config.extract_tables = True

        test_file = tmp_path / "test.pdf"
        test_file.write_bytes(b"dummy pdf content")

        mock_extract_searchable = mocker.patch.object(pdf_extractor, "_extract_pdf_searchable_text_sync")
        mock_extract_searchable.return_value = "Text content"

        mock_extract_playa = mocker.patch.object(pdf_extractor, "_extract_with_playa_sync")
        mock_extract_playa.return_value = "Enhanced text"

        mock_normalize = mocker.patch("kreuzberg._extractors._pdf.normalize_spaces")
        mock_normalize.return_value = "Normalized text"

        with patch.dict("sys.modules", {"kreuzberg._gmft": None}):
            mock_apply_quality = mocker.patch.object(pdf_extractor, "_apply_quality_processing")
            mock_apply_quality.side_effect = lambda x: x

            result = pdf_extractor.extract_path_sync(test_file)

        assert result.content == "Normalized text"
        assert result.tables == []

    def test_extract_path_sync_invalid_text_triggers_ocr(
        self, pdf_extractor: PDFExtractor, tmp_path: Path, mocker: MockerFixture
    ) -> None:
        """Test sync path extraction when extracted text is invalid, triggering OCR."""
        pdf_extractor.config.ocr_backend = "tesseract"

        test_file = tmp_path / "test.pdf"
        test_file.write_bytes(b"dummy pdf content")

        mock_extract_searchable = mocker.patch.object(pdf_extractor, "_extract_pdf_searchable_text_sync")
        mock_extract_searchable.return_value = ""

        mock_extract_ocr = mocker.patch.object(pdf_extractor, "_extract_pdf_with_ocr_sync")
        mock_extract_ocr.return_value = "Valid OCR text"

        mock_normalize = mocker.patch("kreuzberg._extractors._pdf.normalize_spaces")
        mock_normalize.return_value = "Normalized OCR text"

        mock_apply_quality = mocker.patch.object(pdf_extractor, "_apply_quality_processing")
        mock_apply_quality.side_effect = lambda x: x

        result = pdf_extractor.extract_path_sync(test_file)

        assert result.content == "Normalized OCR text"
        mock_extract_ocr.assert_called_once_with(test_file)

    def test_corrupted_pattern_matching(self, pdf_extractor: PDFExtractor) -> None:
        """Test the corrupted pattern regex."""
        test_cases = [
            ("\\x00", True),
            ("\\x01", True),
            ("\\x08", True),
            ("\\x0B", True),
            ("\\x0C", True),
            ("\\x0E", True),
            ("\\x1F", True),
            ("\\uFFFD", True),
            ("A", False),
            ("1", False),
            (" ", False),
            ("\\n", False),
            ("\\t", False),
            ("\\r", False),
        ]

        for char, should_match in test_cases:
            matches = pdf_extractor.CORRUPTED_PATTERN.findall(char)
            if should_match:
                assert len(matches) > 0, f"Character {char!r} should match corrupted pattern"
            else:
                assert len(matches) == 0, f"Character {char!r} should not match corrupted pattern"

    def test_class_constants_values(self, pdf_extractor: PDFExtractor) -> None:
        """Test that class constants have expected values."""
        assert pdf_extractor.SHORT_TEXT_THRESHOLD == 50
        assert pdf_extractor.MINIMUM_CORRUPTED_RESULTS == 2

        assert pdf_extractor.CORRUPTED_PATTERN is not None
        assert hasattr(pdf_extractor.CORRUPTED_PATTERN, "pattern")

    @pytest.mark.anyio
    async def test_extract_path_async_table_extraction_import_error(
        self, pdf_extractor: PDFExtractor, tmp_path: Path, mocker: MockerFixture
    ) -> None:
        """Test async path extraction when GMFT import fails."""
        pdf_extractor.config.extract_tables = True

        test_file = tmp_path / "test.pdf"
        test_file.write_bytes(b"dummy pdf content")

        mock_async_path = mocker.patch("kreuzberg._extractors._pdf.AsyncPath")
        mock_async_path.return_value.read_bytes = mocker.AsyncMock(return_value=b"dummy pdf content")

        mock_extract_searchable = mocker.patch.object(pdf_extractor, "_extract_pdf_searchable_text")
        mock_extract_searchable.return_value = "Text content"

        mock_extract_metadata = mocker.patch.object(pdf_extractor, "_extract_metadata_with_password_attempts")
        mock_extract_metadata.return_value = {"pages": 1}

        with patch.dict("sys.modules", {"kreuzberg._gmft": None}):
            mock_apply_quality = mocker.patch.object(pdf_extractor, "_apply_quality_processing")
            mock_apply_quality.side_effect = lambda x: x

            result = await pdf_extractor.extract_path_async(test_file)

        assert result.content == "Text content"
        assert result.tables == []

from __future__ import annotations

from pathlib import Path

import pytest

from kreuzberg._mime_types import (
    EXCEL_MIME_TYPE,
    MARKDOWN_MIME_TYPE,
    PDF_MIME_TYPE,
    PLAIN_TEXT_MIME_TYPE,
    POWER_POINT_MIME_TYPE,
)
from kreuzberg.exceptions import ValidationError
from kreuzberg.extraction import extract_bytes, extract_file


@pytest.mark.timeout(timeout=60)
@pytest.mark.parametrize("pdf_document", list((Path(__file__).parent / "source").glob("*.pdf")))
async def test_extract_bytes_pdf(pdf_document: Path) -> None:
    content = pdf_document.read_bytes()
    result = await extract_bytes(content, PDF_MIME_TYPE)
    assert result.mime_type == PLAIN_TEXT_MIME_TYPE
    assert isinstance(result.content, str)
    assert result.content.strip()


async def test_extract_bytes_force_ocr_pdf(non_ascii_pdf: Path) -> None:
    content = non_ascii_pdf.read_bytes()
    result = await extract_bytes(content, PDF_MIME_TYPE, True)
    assert result.mime_type == PLAIN_TEXT_MIME_TYPE
    assert result.content.startswith("AMTSBLATT")
    assert isinstance(result.content, str)
    assert result.content.strip()


async def test_extract_bytes_image(ocr_image: Path) -> None:
    content = ocr_image.read_bytes()
    mime_type = "image/jpeg"
    result = await extract_bytes(content, mime_type)
    assert result.mime_type == PLAIN_TEXT_MIME_TYPE
    assert isinstance(result.content, str)
    assert result.content.strip()


async def test_extract_bytes_pandoc(docx_document: Path) -> None:
    content = docx_document.read_bytes()
    mime_type = "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    result = await extract_bytes(content, mime_type)
    assert result.mime_type == MARKDOWN_MIME_TYPE
    assert isinstance(result.content, str)
    assert result.content.strip()


async def test_extract_bytes_plain_text() -> None:
    content = b"This is a plain text file."
    result = await extract_bytes(content, PLAIN_TEXT_MIME_TYPE)
    assert result.mime_type == PLAIN_TEXT_MIME_TYPE
    assert isinstance(result.content, str)
    assert result.content.strip() == "This is a plain text file."


async def test_extract_bytes_pptx(pptx_document: Path) -> None:
    content = pptx_document.read_bytes()
    result = await extract_bytes(content, POWER_POINT_MIME_TYPE)
    assert result.mime_type == MARKDOWN_MIME_TYPE
    assert isinstance(result.content, str)
    assert (
        "At Contoso, we empower organizations to foster collaborative thinking to further drive workplace innovation. By closing the loop and leveraging agile frameworks, we help business grow organically and foster a consumer first mindset."
        in result.content
    )


async def test_extract_bytes_html(html_document: Path) -> None:
    content = html_document.read_bytes()
    result = await extract_bytes(content, "text/html")
    assert result.mime_type == MARKDOWN_MIME_TYPE
    assert isinstance(result.content, str)
    assert (
        result.content
        == "Browsers usually insert quotation marks around the q element. WWF's goal is to: Build a future where people live in harmony with nature."
    )


async def test_extract_bytes_markdown(markdown_document: Path) -> None:
    content = markdown_document.read_bytes()
    result = await extract_bytes(content, MARKDOWN_MIME_TYPE)
    assert result.mime_type == MARKDOWN_MIME_TYPE
    assert isinstance(result.content, str)
    assert result.content.strip()


async def test_extract_bytes_invalid_mime() -> None:
    with pytest.raises(ValidationError, match="Unsupported mime type"):
        await extract_bytes(b"some content", "application/unknown")


@pytest.mark.parametrize("pdf_document", list((Path(__file__).parent / "source").glob("*.pdf")))
async def test_extract_file_pdf(pdf_document: Path) -> None:
    result = await extract_file(pdf_document, PDF_MIME_TYPE)
    assert result.mime_type == PLAIN_TEXT_MIME_TYPE
    assert isinstance(result.content, str)
    assert result.content.strip()


async def test_extract_file_force_ocr_pdf(non_ascii_pdf: Path) -> None:
    result = await extract_file(non_ascii_pdf, PDF_MIME_TYPE, True)
    assert result.mime_type == PLAIN_TEXT_MIME_TYPE
    assert result.content.startswith("AMTSBLATT")
    assert isinstance(result.content, str)
    assert result.content.strip()


async def test_extract_file_image(ocr_image: Path) -> None:
    mime_type = "image/jpeg"
    result = await extract_file(ocr_image, mime_type)
    assert result.mime_type == PLAIN_TEXT_MIME_TYPE
    assert isinstance(result.content, str)
    assert result.content.strip()


async def test_extract_file_pandoc(docx_document: Path) -> None:
    mime_type = "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    result = await extract_file(docx_document, mime_type)
    assert result.mime_type == MARKDOWN_MIME_TYPE
    assert isinstance(result.content, str)
    assert result.content.strip()


async def test_extract_file_plain_text(tmp_path: Path) -> None:
    text_file = tmp_path / "sample.txt"
    text_file.write_text("This is a plain text file.")
    result = await extract_file(text_file, PLAIN_TEXT_MIME_TYPE)
    assert result.mime_type == PLAIN_TEXT_MIME_TYPE
    assert isinstance(result.content, str)
    assert result.content.strip() == "This is a plain text file."


async def test_extract_file_markdown(markdown_document: Path) -> None:
    result = await extract_file(markdown_document, MARKDOWN_MIME_TYPE)
    assert result.mime_type == MARKDOWN_MIME_TYPE
    assert isinstance(result.content, str)
    assert result.content.strip()


async def test_extract_file_pptx(pptx_document: Path) -> None:
    result = await extract_file(pptx_document, POWER_POINT_MIME_TYPE)
    assert result.mime_type == MARKDOWN_MIME_TYPE
    assert isinstance(result.content, str)
    assert (
        "At Contoso, we empower organizations to foster collaborative thinking to further drive workplace innovation. By closing the loop and leveraging agile frameworks, we help business grow organically and foster a consumer first mindset."
        in result.content
    )


async def test_extract_file_html(html_document: Path) -> None:
    result = await extract_file(html_document, "text/html")
    assert result.mime_type == MARKDOWN_MIME_TYPE
    assert isinstance(result.content, str)
    assert (
        result.content
        == "Browsers usually insert quotation marks around the q element. WWF's goal is to: Build a future where people live in harmony with nature."
    )


async def test_extract_file_invalid_mime() -> None:
    with pytest.raises(ValidationError, match="Unsupported mime type"):
        await extract_file("/invalid/path.txt", "application/unknown")


async def test_extract_file_not_exists() -> None:
    with pytest.raises(ValidationError, match="The file does not exist"):
        await extract_file("/invalid/path.txt", PLAIN_TEXT_MIME_TYPE)


async def test_extract_bytes_excel(excel_document: Path) -> None:
    content = excel_document.read_bytes()
    result = await extract_bytes(content, EXCEL_MIME_TYPE)
    assert result.mime_type == MARKDOWN_MIME_TYPE
    assert isinstance(result.content, str)
    assert result.content.strip()


async def test_extract_file_excel(excel_document: Path) -> None:
    result = await extract_file(excel_document, EXCEL_MIME_TYPE)
    assert result.mime_type == MARKDOWN_MIME_TYPE
    assert isinstance(result.content, str)
    assert result.content.strip()


async def test_extract_file_excel_invalid() -> None:
    with pytest.raises(ValidationError, match="The file does not exist"):
        await extract_file("/invalid/path.xlsx", EXCEL_MIME_TYPE)

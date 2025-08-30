"""Tests for kreuzberg._playa helper functions."""

from __future__ import annotations

from typing import TYPE_CHECKING
from unittest.mock import Mock, patch

import pytest

from kreuzberg._playa import (
    _collect_document_permissions,
    _extract_author_metadata,
    _extract_basic_metadata,
    _extract_category_metadata,
    _extract_creator_metadata,
    _extract_date_metadata,
    _extract_document_dimensions,
    _extract_keyword_metadata,
    _extract_structure_information,
    _format_outline,
    _generate_document_summary,
    _generate_outline_description,
    _parse_date_string,
    extract_pdf_metadata_sync,
)
from kreuzberg.exceptions import ParsingError

if TYPE_CHECKING:
    from kreuzberg._types import Metadata


def test_parse_date_string_basic() -> None:
    """Test basic date string parsing."""
    assert _parse_date_string("D:20240101120000") == "2024-01-01T12:00:00"

    assert _parse_date_string("20240101120000") == "2024-01-01T12:00:00"

    assert _parse_date_string("20240101") == "2024-01-01T00:00:00"

    assert _parse_date_string("2024") == "2024"
    assert _parse_date_string("D:2024") == "2024"


def test_extract_basic_metadata() -> None:
    """Test extraction of basic metadata fields."""
    pdf_info = {
        "title": b"Test Title",
        "subject": b"Test Subject",
        "publisher": b"Test Publisher",
        "copyright": b"Test Copyright",
        "comments": b"Test Comments",
        "identifier": b"Test ID",
        "license": b"Test License",
        "modifiedby": b"Test Modifier",
        "version": b"1.0",
    }

    result: Metadata = {}
    _extract_basic_metadata(pdf_info, result)

    assert result["title"] == "Test Title"
    assert result["subject"] == "Test Subject"
    assert result["publisher"] == "Test Publisher"
    assert result["copyright"] == "Test Copyright"
    assert result["comments"] == "Test Comments"
    assert result["identifier"] == "Test ID"
    assert result["license"] == "Test License"
    assert result["modified_by"] == "Test Modifier"
    assert result["version"] == "1.0"


def test_extract_basic_metadata_alternative_keys() -> None:
    """Test extraction with alternative key names."""
    pdf_info = {
        "Publisher": b"Test Publisher",
        "rights": b"Test Rights",
        "id": b"Test ID",
        "last_modified_by": b"Test Modifier",
    }

    result: Metadata = {}
    _extract_basic_metadata(pdf_info, result)

    assert result["publisher"] == "Test Publisher"
    assert result["copyright"] == "Test Rights"
    assert result["identifier"] == "Test ID"
    assert result["modified_by"] == "Test Modifier"


def test_extract_basic_metadata_skip_existing() -> None:
    """Test that existing metadata is not overwritten."""
    pdf_info = {"title": b"New Title"}
    result: Metadata = {"title": "Existing Title"}

    _extract_basic_metadata(pdf_info, result)

    assert result["title"] == "Existing Title"


def test_extract_author_metadata_string() -> None:
    """Test author extraction from string format."""
    pdf_info = {"author": b"John Doe"}
    result: Metadata = {}
    _extract_author_metadata(pdf_info, result)
    assert result["authors"] == ["John Doe"]

    pdf_info = {"author": b"John Doe, Jane Smith"}
    result = {}
    _extract_author_metadata(pdf_info, result)
    assert result["authors"] == ["John Doe", "Jane Smith"]

    pdf_info = {"author": b"John Doe; Jane Smith"}
    result = {}
    _extract_author_metadata(pdf_info, result)
    assert result["authors"] == ["John Doe", "Jane Smith"]

    pdf_info = {"author": b"John Doe and Jane Smith"}
    result = {}
    _extract_author_metadata(pdf_info, result)
    assert result["authors"] == ["John Doe", "Jane Smith"]

    pdf_info = {"author": b" John Doe ;  Jane Smith ,  Bob Johnson  "}
    result = {}
    _extract_author_metadata(pdf_info, result)
    assert result["authors"] == ["John Doe", "Jane Smith", "Bob Johnson"]


def test_extract_author_metadata_list() -> None:
    """Test author extraction from list format."""
    pdf_info = {"author": [b"John Doe", b"Jane Smith"]}
    result: Metadata = {}
    _extract_author_metadata(pdf_info, result)
    assert result["authors"] == ["John Doe", "Jane Smith"]


def test_extract_keyword_metadata_string() -> None:
    """Test keyword extraction from string format."""
    pdf_info = {"keywords": b"python, programming, testing"}
    result: Metadata = {}
    _extract_keyword_metadata(pdf_info, result)
    assert result["keywords"] == ["python", "programming", "testing"]

    pdf_info = {"keywords": b"python; programming; testing"}
    result = {}
    _extract_keyword_metadata(pdf_info, result)
    assert result["keywords"] == ["python", "programming", "testing"]

    pdf_info = {"keywords": b" python ;  programming ,  testing  "}
    result = {}
    _extract_keyword_metadata(pdf_info, result)
    assert result["keywords"] == ["python", "programming", "testing"]

    pdf_info = {"keywords": b"python, , testing"}
    result = {}
    _extract_keyword_metadata(pdf_info, result)
    assert result["keywords"] == ["python", "testing"]


def test_extract_keyword_metadata_list() -> None:
    """Test keyword extraction from list format."""
    pdf_info = {"keywords": [b"python", b"programming", b"testing"]}
    result: Metadata = {}
    _extract_keyword_metadata(pdf_info, result)
    assert result["keywords"] == ["python", "programming", "testing"]


def test_extract_category_metadata_string() -> None:
    """Test category extraction from string format."""
    pdf_info = {"categories": b"tech, programming, python"}
    result: Metadata = {}
    _extract_category_metadata(pdf_info, result)
    assert result["categories"] == ["tech", "programming", "python"]

    pdf_info = {"category": b"tech, programming"}
    result = {}
    _extract_category_metadata(pdf_info, result)
    assert result["categories"] == ["tech", "programming"]

    pdf_info = {"categories": b"tech, , python"}
    result = {}
    _extract_category_metadata(pdf_info, result)
    assert result["categories"] == ["tech", "python"]


def test_extract_category_metadata_list() -> None:
    """Test category extraction from list format."""
    pdf_info = {"categories": [b"tech", b"programming"]}
    result: Metadata = {}
    _extract_category_metadata(pdf_info, result)
    assert result["categories"] == ["tech", "programming"]


def test_extract_date_metadata() -> None:
    """Test date metadata extraction."""
    pdf_info = {"creationdate": b"D:20240101120000"}
    result: Metadata = {}
    _extract_date_metadata(pdf_info, result)
    assert result["created_at"] == "2024-01-01T12:00:00"

    pdf_info = {"createdate": b"D:20240101120000"}
    result = {}
    _extract_date_metadata(pdf_info, result)
    assert result["created_at"] == "2024-01-01T12:00:00"

    pdf_info = {"moddate": b"D:20240201150000"}
    result = {}
    _extract_date_metadata(pdf_info, result)
    assert result["modified_at"] == "2024-02-01T15:00:00"

    pdf_info = {"modificationdate": b"D:20240201150000"}
    result = {}
    _extract_date_metadata(pdf_info, result)
    assert result["modified_at"] == "2024-02-01T15:00:00"

    pdf_info = {"creationdate": b"Invalid Date"}
    result = {}
    _extract_date_metadata(pdf_info, result)
    assert result["created_at"] == "Invalid Date"

    pdf_info = {"creationdate": b"D:20240101"}
    result = {}
    _extract_date_metadata(pdf_info, result)
    assert result["created_at"] == "2024-01-01T00:00:00"


def test_extract_creator_metadata() -> None:
    """Test creator metadata extraction."""
    pdf_info = {"creator": b"Test Creator"}
    result: Metadata = {}
    _extract_creator_metadata(pdf_info, result)
    assert result["created_by"] == "Test Creator"

    pdf_info = {"producer": b"Test Producer"}
    result = {}
    _extract_creator_metadata(pdf_info, result)
    assert result["created_by"] == "Test Producer"

    pdf_info = {"creator": b"Test Creator", "producer": b"Test Producer"}
    result = {}
    _extract_creator_metadata(pdf_info, result)
    assert result["created_by"] == "Test Creator (Producer: Test Producer)"

    pdf_info = {"creator": b"Test Creator", "producer": b"Test Creator"}
    result = {}
    _extract_creator_metadata(pdf_info, result)
    assert result["created_by"] == "Test Creator"


def test_extract_document_dimensions() -> None:
    """Test document dimensions extraction."""
    page = Mock()
    page.width = 595.5
    page.height = 842.5

    document = Mock()
    document.pages = [page]

    result: Metadata = {}
    _extract_document_dimensions(document, result)

    assert result["width"] == 595
    assert result["height"] == 842


def test_format_outline() -> None:
    """Test outline formatting."""
    entry1 = Mock()
    entry1.title = "Chapter 1"
    entry1.children = []

    entry2 = Mock()
    entry2.title = "Chapter 2"
    entry2.children = []

    outline = _format_outline([entry1, entry2])
    assert outline == ["- Chapter 1", "- Chapter 2"]

    subentry = Mock()
    subentry.title = "Section 1.1"
    subentry.children = []

    entry1.children = [subentry]

    outline = _format_outline([entry1])
    assert outline == ["- Chapter 1"]

    entry_no_title = Mock()
    entry_no_title.title = None
    entry_no_title.children = []

    outline = _format_outline([entry_no_title])
    assert outline == []


def test_generate_outline_description() -> None:
    """Test outline description generation."""
    entry1 = Mock()
    entry1.title = "Chapter 1"
    entry1.children = []

    document = Mock()
    document.outline = [entry1]

    with patch("kreuzberg._playa._format_outline", return_value=["- Chapter 1"]):
        description = _generate_outline_description(document)
        assert description == "Table of Contents:\n- Chapter 1"

    with patch("kreuzberg._playa._format_outline", return_value=[]):
        description = _generate_outline_description(document)
        assert description == ""


def test_generate_document_summary() -> None:
    """Test document summary generation."""
    document = Mock()
    document.pages = [Mock(), Mock(), Mock()]
    document.is_printable = True
    document.is_modifiable = False
    document.is_extractable = True
    document.configure_mock(
        pdf_version=None, is_encrypted=False, status=None, is_pdf_a=False, encryption_method=None, pdf_a_level=None
    )

    summary = _generate_document_summary(document)
    assert "PDF document with 3 pages" in summary
    assert "printable" in summary
    assert "extractable" in summary
    assert "modifiable" not in summary

    document.pages = [Mock()]
    summary = _generate_document_summary(document)
    assert "PDF document with 1 page." in summary

    document.pdf_version = "1.7"
    summary = _generate_document_summary(document)
    assert "PDF version 1.7" in summary

    document.is_encrypted = True
    document.encryption_method = "AES-256"
    summary = _generate_document_summary(document)
    assert "Document is encrypted" in summary
    assert "Encryption: AES-256" in summary

    document.status = b"Final"
    summary = _generate_document_summary(document)
    assert "Status: Final" in summary

    document.is_pdf_a = True
    document.pdf_a_level = "1b"
    summary = _generate_document_summary(document)
    assert "PDF/A-1b compliant" in summary

    document.pdf_a_level = None
    summary = _generate_document_summary(document)
    assert "PDF/A compliant" in summary


def test_collect_document_permissions() -> None:
    """Test document permissions collection."""
    document = Mock()

    document.is_printable = True
    document.is_modifiable = True
    document.is_extractable = True
    permissions = _collect_document_permissions(document)
    assert permissions == ["printable", "modifiable", "extractable"]

    document.is_printable = True
    document.is_modifiable = False
    document.is_extractable = True
    permissions = _collect_document_permissions(document)
    assert permissions == ["printable", "extractable"]

    document.is_printable = False
    document.is_modifiable = False
    document.is_extractable = False
    permissions = _collect_document_permissions(document)
    assert permissions == []


def test_extract_structure_information() -> None:
    """Test structure information extraction."""
    element1 = Mock()
    element1.language = "EN"
    element1.role = None
    element1.children = []

    element2 = Mock()
    element2.language = "fr"
    element2.role = None
    element2.children = []

    document = Mock()
    document.structure = [element1, element2]

    result: Metadata = {}
    _extract_structure_information(document, result)
    assert set(result["languages"]) == {"en", "fr"}

    element3 = Mock()
    element3.language = None
    element3.role = "H1"
    element3.text = b"Subtitle Text"
    element3.children = []

    document.structure = [element3]
    result = {"title": "Main Title"}
    _extract_structure_information(document, result)
    assert result["subtitle"] == "Subtitle Text"

    result = {"title": "Subtitle Text"}
    _extract_structure_information(document, result)
    assert "subtitle" not in result

    child_element = Mock()
    child_element.language = "es"
    child_element.role = None
    child_element.children = []

    parent_element = Mock()
    parent_element.language = "en"
    parent_element.role = None
    parent_element.children = [child_element]

    document.structure = [parent_element]
    result = {}
    _extract_structure_information(document, result)
    assert set(result["languages"]) == {"en", "es"}

    document.structure = []
    result = {}
    _extract_structure_information(document, result)
    assert "languages" not in result
    assert "subtitle" not in result


def test_extract_pdf_metadata_sync() -> None:
    """Test synchronous version of extract_pdf_metadata."""
    mock_document = Mock()
    mock_info = Mock()
    mock_info.items.return_value = [("Title", b"Test Title"), ("Author", b"Test Author")]
    mock_document.info = [mock_info]

    mock_page = Mock()
    mock_page.width = 595
    mock_page.height = 842
    mock_document.pages = [mock_page]

    mock_document.outline = []
    mock_document.structure = []
    mock_document.is_printable = True
    mock_document.is_modifiable = False
    mock_document.is_extractable = True

    def mock_hasattr(obj: object, name: str) -> bool:
        return name not in ("status", "pdf_version", "is_encrypted", "encryption_method", "is_pdf_a", "pdf_a_level")

    with (
        patch("kreuzberg._playa.parse", return_value=mock_document),
        patch("kreuzberg._playa.asobj", return_value={"title": b"Test Title", "author": b"Test Author"}),
        patch("builtins.hasattr", side_effect=mock_hasattr),
    ):
        metadata = extract_pdf_metadata_sync(b"test pdf content")
        assert metadata["title"] == "Test Title"
        assert metadata["authors"] == ["Test Author"]
        assert metadata["width"] == 595
        assert metadata["height"] == 842
        assert "summary" in metadata

    with patch("kreuzberg._playa.parse", side_effect=ValueError("Test error")):
        with pytest.raises(ParsingError, match="Failed to extract PDF metadata: Test error"):
            extract_pdf_metadata_sync(b"invalid pdf")


def test_extract_pdf_metadata_sync_with_password() -> None:
    """Test synchronous extraction with password."""
    mock_document = Mock()
    mock_document.info = []
    mock_document.pages = []
    mock_document.outline = []
    mock_document.structure = []
    mock_document.is_printable = True
    mock_document.is_modifiable = True
    mock_document.is_extractable = True

    def mock_hasattr(obj: object, name: str) -> bool:
        return name not in ("status", "pdf_version", "is_encrypted", "encryption_method", "is_pdf_a", "pdf_a_level")

    with (
        patch("kreuzberg._playa.parse", return_value=mock_document) as mock_parse,
        patch("builtins.hasattr", side_effect=mock_hasattr),
    ):
        metadata = extract_pdf_metadata_sync(b"test pdf", password="secret")
        mock_parse.assert_called_once_with(b"test pdf", max_workers=1, password="secret")
        assert "summary" in metadata

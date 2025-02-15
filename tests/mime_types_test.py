"""Tests for MIME type validation and detection."""

from pathlib import Path

import pytest

from kreuzberg._mime_types import (
    EXT_TO_MIME_TYPE,
    HTML_MIME_TYPE,
    IMAGE_MIME_TYPES,
    MARKDOWN_MIME_TYPE,
    PDF_MIME_TYPE,
    PLAIN_TEXT_MIME_TYPE,
    POWER_POINT_MIME_TYPE,
    SUPPORTED_MIME_TYPES,
    validate_mime_type,
)
from kreuzberg.exceptions import ValidationError


def test_validate_mime_type_with_explicit_mime_type() -> None:
    """Test that explicit MIME type validation works correctly."""
    # Test with exact MIME type matches
    assert validate_mime_type("test.txt", PLAIN_TEXT_MIME_TYPE) == PLAIN_TEXT_MIME_TYPE
    assert validate_mime_type("test.pdf", PDF_MIME_TYPE) == PDF_MIME_TYPE
    assert validate_mime_type("test.html", HTML_MIME_TYPE) == HTML_MIME_TYPE

    # Test with MIME type prefixes
    assert validate_mime_type("test.txt", "text/plain; charset=utf-8") == PLAIN_TEXT_MIME_TYPE
    assert validate_mime_type("test.pdf", "application/pdf; version=1.7") == PDF_MIME_TYPE
    assert validate_mime_type("test.html", "text/html; charset=utf-8") == HTML_MIME_TYPE

    # Test with invalid MIME type
    with pytest.raises(ValidationError) as exc_info:
        validate_mime_type("test.txt", "application/invalid")
    assert "Unsupported mime type" in str(exc_info.value)


def test_validate_mime_type_extension_detection() -> None:
    """Test MIME type detection from file extensions."""
    # Test common file extensions
    assert validate_mime_type("document.txt") == PLAIN_TEXT_MIME_TYPE
    assert validate_mime_type("document.md") == MARKDOWN_MIME_TYPE
    assert validate_mime_type("presentation.pptx") == POWER_POINT_MIME_TYPE
    assert validate_mime_type("document.pdf") == PDF_MIME_TYPE

    # Test case insensitivity
    assert validate_mime_type("image.PNG") == "image/png"
    assert validate_mime_type("document.PDF") == PDF_MIME_TYPE
    assert validate_mime_type("page.HTML") == HTML_MIME_TYPE

    # Test with Path object
    assert validate_mime_type(Path("document.txt")) == PLAIN_TEXT_MIME_TYPE

    # Test with system-detected MIME types that include parameters
    assert validate_mime_type("document.txt", "text/plain; charset=utf-8") == PLAIN_TEXT_MIME_TYPE
    assert validate_mime_type("document.html", "text/html; charset=utf-8") == HTML_MIME_TYPE


def test_validate_mime_type_image_extensions() -> None:
    """Test MIME type detection for various image formats."""
    image_files = {
        "photo.jpg": "image/jpeg",
        "photo.jpeg": "image/jpeg",
        "icon.png": "image/png",
        "picture.gif": "image/gif",
        "scan.tiff": "image/tiff",
        "graphic.webp": "image/webp",
        "image.bmp": "image/bmp",
    }

    for filename, expected_mime in image_files.items():
        # Test basic extension detection
        assert validate_mime_type(filename) == expected_mime
        assert expected_mime in IMAGE_MIME_TYPES

        # Test with MIME type parameters
        parameterized_mime = f"{expected_mime}; charset=binary"
        assert validate_mime_type(filename, parameterized_mime) == expected_mime


def test_validate_mime_type_unknown_extension() -> None:
    """Test behavior with unknown file extensions."""
    # Test with unknown extension
    with pytest.raises(ValidationError) as exc_info:
        validate_mime_type("file.unknown")
    assert "Could not determine the mime type" in str(exc_info.value)
    assert "extension" in exc_info.value.context
    assert exc_info.value.context["extension"] == ".unknown"


def test_ext_to_mime_type_mapping_consistency() -> None:
    """Test that all mapped MIME types are in SUPPORTED_MIME_TYPES."""
    for mime_type in EXT_TO_MIME_TYPE.values():
        # Test the MIME type is supported
        result = validate_mime_type("test.txt", mime_type)
        assert result in SUPPORTED_MIME_TYPES

        # Test with parameters
        parameterized = f"{mime_type}; charset=utf-8"
        result = validate_mime_type("test.txt", parameterized)
        assert result in SUPPORTED_MIME_TYPES


def test_validate_mime_type_with_path_variants() -> None:
    """Test MIME type validation with different path formats."""
    # Test with string paths and exact MIME types
    assert validate_mime_type("./document.txt") == PLAIN_TEXT_MIME_TYPE
    assert validate_mime_type("/path/to/document.pdf") == PDF_MIME_TYPE
    assert validate_mime_type("relative/path/page.html") == HTML_MIME_TYPE

    # Test with Path objects and MIME type parameters
    assert validate_mime_type(Path("document.txt"), "text/plain; charset=utf-8") == PLAIN_TEXT_MIME_TYPE
    assert validate_mime_type(Path("/absolute/path/document.pdf"), "application/pdf; version=1.7") == PDF_MIME_TYPE
    assert validate_mime_type(Path("./relative/path/page.html"), "text/html; charset=utf-8") == HTML_MIME_TYPE

    # Test with system-detected MIME types
    assert validate_mime_type("./document.txt", "text/plain; charset=us-ascii") == PLAIN_TEXT_MIME_TYPE
    assert validate_mime_type("/path/to/document.pdf", "application/pdf; version=1.5") == PDF_MIME_TYPE


def test_validate_mime_type_with_dots_in_name() -> None:
    """Test MIME type validation with filenames containing multiple dots."""
    # Test files with multiple dots
    assert validate_mime_type("my.backup.txt") == PLAIN_TEXT_MIME_TYPE
    assert validate_mime_type("version.1.2.pdf") == PDF_MIME_TYPE
    assert validate_mime_type("index.min.html") == HTML_MIME_TYPE

    # Test with version numbers
    assert validate_mime_type("readme.v2.md") == MARKDOWN_MIME_TYPE
    assert validate_mime_type("document.2023.02.14.pdf") == PDF_MIME_TYPE

    # Test with MIME type parameters
    assert validate_mime_type("my.backup.txt", "text/plain; charset=utf-8") == PLAIN_TEXT_MIME_TYPE
    assert validate_mime_type("index.min.html", "text/html; charset=utf-8") == HTML_MIME_TYPE

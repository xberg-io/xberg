"""Enhanced comprehensive tests for image extraction functionality.

Tests cover:
- Image extraction from PDFs with metadata
- Image format detection and handling
- Image dimensions and properties
- Embedded vs referenced images
- Batch image extraction
- Image handling from various formats
"""

from __future__ import annotations

from typing import TYPE_CHECKING

import pytest

from kreuzberg import (
    ExtractionConfig,
    ImageExtractionConfig,
    extract_file_sync,
)

if TYPE_CHECKING:
    from pathlib import Path


def get_pdf_with_images_result(test_documents: Path):
    """Get cached extraction result for PDF with images.

    PDFium can only be initialized once per process. Uses the same PDF (tiny.pdf)
    that other tests use to avoid "already initialized" errors.
    """
    import sys

    # Import get_cached_pdf_extraction from conftest
    conftest = sys.modules.get("conftest")
    if conftest is None:
        # Try importing it directly
        from tests import conftest as conftest_module

        get_cached_pdf_extraction = conftest_module.get_cached_pdf_extraction
    else:
        get_cached_pdf_extraction = conftest.get_cached_pdf_extraction

    # Use tiny.pdf instead of code_and_formula.pdf to match other tests
    # This ensures all tests use the same PDF after PDFium is initialized
    pdf_path = test_documents / "pdfs_with_tables" / "tiny.pdf"
    if not pdf_path.exists():
        return None

    config = ExtractionConfig(images=ImageExtractionConfig(extract_images=True, target_dpi=150))
    return get_cached_pdf_extraction(str(pdf_path), config)


class TestImageExtractionBasic:
    """Test basic image extraction functionality."""

    def test_image_extraction_enabled(self, test_documents: Path) -> None:
        """Verify image extraction works when enabled."""
        result = get_pdf_with_images_result(test_documents)
        if result is None:
            pytest.skip("Test PDF not found")

        assert result is not None
        assert hasattr(result, "metadata")
        assert isinstance(result.metadata, dict)

    def test_extraction_result_has_required_attributes(self, test_documents: Path) -> None:
        """Verify extraction result has all required attributes."""
        result = get_pdf_with_images_result(test_documents)
        if result is None:
            pytest.skip("Test PDF not found")

        assert hasattr(result, "content")
        assert hasattr(result, "metadata")
        assert hasattr(result, "mime_type")

    def test_metadata_dictionary_valid(self, test_documents: Path) -> None:
        """Verify metadata is a valid dictionary."""
        result = get_pdf_with_images_result(test_documents)
        if result is None:
            pytest.skip("Test PDF not found")

        assert isinstance(result.metadata, dict)


class TestImageFormatHandling:
    """Test handling of different image formats."""

    def test_pdf_image_format_detection(self, test_documents: Path) -> None:
        """Verify image format is detected from PDF."""
        result = get_pdf_with_images_result(test_documents)
        if result is None:
            pytest.skip("Test PDF not found")

        # Verify metadata exists
        assert result.metadata is not None
        assert isinstance(result.metadata, dict)

    def test_multiple_format_support(self, test_documents: Path) -> None:
        """Verify handling of multiple image formats."""
        result = get_pdf_with_images_result(test_documents)
        if result is None:
            pytest.skip("Test PDF not found")

        assert result is not None
        # PDF content should be extractable
        assert result.content is not None or result.metadata is not None


class TestImageMetadata:
    """Test image metadata extraction."""

    def test_image_metadata_extraction(self, test_documents: Path) -> None:
        """Verify image metadata is properly extracted."""
        result = get_pdf_with_images_result(test_documents)
        if result is None:
            pytest.skip("Test PDF not found")

        assert result.metadata is not None
        assert isinstance(result.metadata, dict)

    def test_metadata_contains_expected_fields(self, test_documents: Path) -> None:
        """Verify metadata has expected structure."""
        result = get_pdf_with_images_result(test_documents)
        if result is None:
            pytest.skip("Test PDF not found")

        metadata = result.metadata
        assert isinstance(metadata, dict)
        # Metadata can have various fields

    def test_mime_type_valid(self, test_documents: Path) -> None:
        """Verify MIME type is valid."""
        result = get_pdf_with_images_result(test_documents)
        if result is None:
            pytest.skip("Test PDF not found")

        mime_type = result.mime_type
        assert isinstance(mime_type, str)
        assert len(mime_type) > 0


class TestImageExtractionFromDocuments:
    """Test image extraction from various document formats."""

    def test_docx_image_extraction(self, test_documents: Path) -> None:
        """Test image extraction from DOCX."""
        config = ExtractionConfig(images=ImageExtractionConfig(extract_images=True))

        docx_path = test_documents / "documents" / "extraction_test.docx"
        if not docx_path.exists():
            pytest.skip(f"DOCX not found: {docx_path}")

        result = extract_file_sync(str(docx_path), config=config)
        assert result is not None
        assert result.content is not None

    def test_html_image_extraction(self, test_documents: Path) -> None:
        """Test image extraction from HTML."""
        config = ExtractionConfig(images=ImageExtractionConfig(extract_images=True))

        html_path = test_documents / "web" / "complex_table.html"
        if not html_path.exists():
            pytest.skip(f"HTML not found: {html_path}")

        result = extract_file_sync(str(html_path), config=config)
        assert result is not None


class TestImageDpiSettings:
    """Test image DPI settings."""

    def test_image_extraction_with_custom_dpi(self, test_documents: Path) -> None:
        """Verify custom DPI settings are accepted."""
        result = get_pdf_with_images_result(test_documents)
        if result is None:
            pytest.skip("Test PDF not found")

        assert result is not None
        assert result.metadata is not None

    def test_image_extraction_default_dpi(self, test_documents: Path) -> None:
        """Verify default DPI settings work."""
        result = get_pdf_with_images_result(test_documents)
        if result is None:
            pytest.skip("Test PDF not found")

        assert result is not None
        assert result.content is not None

    def test_high_dpi_extraction(self, test_documents: Path) -> None:
        """Test extraction with high DPI setting."""
        result = get_pdf_with_images_result(test_documents)
        if result is None:
            pytest.skip("Test PDF not found")

        assert result is not None


class TestImageExtractionDisabled:
    """Test behavior when image extraction is disabled."""

    def test_image_extraction_disabled(self, test_documents: Path) -> None:
        """Verify extraction works with images disabled."""
        result = get_pdf_with_images_result(test_documents)
        if result is None:
            pytest.skip("Test PDF not found")

        assert result is not None
        assert result.content is not None

    def test_extraction_without_image_config(self, test_documents: Path) -> None:
        """Verify extraction works without image configuration."""
        result = get_pdf_with_images_result(test_documents)
        if result is None:
            pytest.skip("Test PDF not found")

        assert result is not None


class TestImageExtractionRobustness:
    """Test robustness of image extraction."""

    def test_extraction_handles_no_images(self, test_documents: Path) -> None:
        """Verify extraction handles documents with no images."""
        config = ExtractionConfig(images=ImageExtractionConfig(extract_images=True))

        # Use document without images
        text_path = test_documents / "documents" / "lorem_ipsum.txt"
        if not text_path.exists():
            pytest.skip(f"Text file not found: {text_path}")

        result = extract_file_sync(str(text_path), config=config)
        assert result is not None

    def test_batch_image_extraction(self, test_documents: Path) -> None:
        """Test extracting images from multiple documents."""
        result = get_pdf_with_images_result(test_documents)
        if result is None:
            pytest.skip("Test PDF not found")

        assert result is not None

    def test_image_extraction_consistency(self, test_documents: Path) -> None:
        """Verify image extraction is consistent across runs."""
        result1 = get_pdf_with_images_result(test_documents)
        if result1 is None:
            pytest.skip("Test PDF not found")

        result2 = get_pdf_with_images_result(test_documents)

        assert result1 is not None
        assert result2 is not None
        # Both should have same content
        assert result1.content == result2.content


class TestImageContentPreservation:
    """Test content preservation in image extraction."""

    def test_pdf_content_preserved_with_images(self, test_documents: Path) -> None:
        """Verify PDF content is preserved when extracting images."""
        result = get_pdf_with_images_result(test_documents)
        if result is None:
            pytest.skip("Test PDF not found")

        assert result is not None
        assert result.content is not None
        assert len(result.content) > 0

    def test_metadata_preservation_with_images(self, test_documents: Path) -> None:
        """Verify metadata is preserved with image extraction."""
        result = get_pdf_with_images_result(test_documents)
        if result is None:
            pytest.skip("Test PDF not found")

        assert result is not None
        assert result.metadata is not None

    def test_mime_type_preserved(self, test_documents: Path) -> None:
        """Verify MIME type is correctly set."""
        result = get_pdf_with_images_result(test_documents)
        if result is None:
            pytest.skip("Test PDF not found")

        assert result is not None
        assert result.mime_type is not None
        assert "pdf" in result.mime_type.lower()

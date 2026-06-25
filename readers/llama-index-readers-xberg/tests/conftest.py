"""Shared test fixtures for XbergReader tests."""

from unittest.mock import MagicMock

from xberg import ExtractedImage, ExtractedTable, ExtractionResult, Metadata
from llama_index.readers.xberg._types import Annotation, Keyword, PageContent, ProcessingWarning


def make_extraction_result(
    content: str = "Hello world",
    mime_type: str = "application/pdf",
    metadata: Metadata | None = None,
    pages: list[PageContent] | None = None,
    elements: list[dict] | None = None,
    tables: list[ExtractedTable] | None = None,
    images: list[ExtractedImage] | None = None,
    quality_score: float | None = 0.95,
    detected_languages: list[str] | None = None,
    extracted_keywords: list[Keyword] | None = None,
    processing_warnings: list[ProcessingWarning] | None = None,
    annotations: list[Annotation] | None = None,
    page_count: int = 1,
) -> MagicMock:
    """Create a mock ExtractionResult with sensible defaults."""
    result = MagicMock(spec=ExtractionResult)
    result.content = content
    result.mime_type = mime_type
    result.metadata = metadata or {
        "title": "Test Document",
        "subject": None,
        "authors": ["Author One"],
        "keywords": ["test"],
        "language": "eng",
        "created_at": "2026-01-01T00:00:00Z",
        "modified_at": None,
        "created_by": "TestApp",
        "modified_by": None,
        "format_type": "pdf",
    }
    result.tables = tables or []
    result.pages = pages
    result.elements = elements
    result.images = images
    result.quality_score = quality_score
    result.detected_languages = detected_languages or ["eng"]
    result.extracted_keywords = extracted_keywords or []
    result.processing_warnings = processing_warnings or []
    result.annotations = annotations
    result.output_format = "plain"
    result.result_format = "unified"
    result.get_page_count.return_value = page_count
    return result


def make_page_content(
    page_number: int = 1,
    content: str = "Page content",
    tables: list[ExtractedTable] | None = None,
    images: list[ExtractedImage] | None = None,
    *,
    is_blank: bool = False,
) -> PageContent:
    """Create a PageContent dict matching xberg's runtime format."""
    return {
        "page_number": page_number,
        "content": content,
        "tables": tables or [],
        "images": images or [],
        "is_blank": is_blank,
    }

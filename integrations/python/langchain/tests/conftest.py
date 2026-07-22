"""Shared test fixtures for langchain-xberg."""

from pathlib import Path
from typing import Any
from unittest.mock import MagicMock

import pytest


@pytest.fixture
def sample_txt_path() -> Path:
    """Path to a small test text file."""
    return Path(__file__).parent / "fixtures" / "sample.txt"


@pytest.fixture
def sample_pdf_path() -> Path:
    """Path to a small test PDF."""
    return Path(__file__).parent / "fixtures" / "sample.pdf"


@pytest.fixture
def sample_docx_path() -> Path:
    """Path to a small test DOCX file."""
    return Path(__file__).parent / "fixtures" / "sample.docx"


@pytest.fixture
def sample_html_path() -> Path:
    """Path to a small test HTML file."""
    return Path(__file__).parent / "fixtures" / "sample.html"


@pytest.fixture
def sample_bytes() -> bytes:
    """Sample text bytes for bytes-mode testing."""
    return b"Hello, this is sample text content for testing."


@pytest.fixture
def tmp_dir_with_files(tmp_path: Path) -> Path:
    """Temporary directory with mixed file types for glob testing."""
    (tmp_path / "file1.txt").write_text("Text file 1")
    (tmp_path / "file2.txt").write_text("Text file 2")
    (tmp_path / "subdir").mkdir()
    (tmp_path / "subdir" / "file3.txt").write_text("Text file 3")
    return tmp_path


def make_mock_result(
    content: str = "Extracted text content",
    mime_type: str = "text/plain",
    *,
    metadata: dict[str, Any] | None = None,
    tables: list[Any] | None = None,
    pages: list[dict[str, Any]] | None = None,
    quality_score: float | None = 1.0,
    detected_languages: list[str] | None = None,
    extracted_keywords: list[Any] | None = None,
    processing_warnings: list[Any] | None = None,
    output_format: str | None = "markdown",
    page_count: int = 1,
) -> MagicMock:
    """Create a mock ExtractionResult with sensible defaults."""
    result = MagicMock()
    result.content = content
    result.mime_type = mime_type
    result.metadata = metadata if metadata is not None else {"format_type": "text"}
    result.tables = tables or []
    result.pages = pages
    result.quality_score = quality_score
    result.detected_languages = detected_languages
    result.extracted_keywords = extracted_keywords
    if processing_warnings is not None:
        warnings = []
        for w in processing_warnings:
            warning = MagicMock()
            if isinstance(w, str):
                warning.source = "extraction"
                warning.message = w
            else:
                warning.source = w.get("source", "extraction")
                warning.message = w.get("message", "")
            warnings.append(warning)
        result.processing_warnings = warnings
    else:
        result.processing_warnings = []
    result.output_format = output_format
    result.get_page_count.return_value = page_count
    return result


def make_mock_table(
    cells: list[list[str]] | None = None,
    markdown: str = "| A | B |\n|---|---|\n| 1 | 2 |",
    page_number: int = 1,
) -> MagicMock:
    """Create a mock ExtractedTable."""
    table = MagicMock()
    table.cells = cells or [["A", "B"], ["1", "2"]]
    table.markdown = markdown
    table.page_number = page_number
    return table


def make_mock_keyword(
    text: str = "python",
    score: float = 0.95,
    algorithm: str = "yake",
) -> MagicMock:
    """Create a mock ExtractedKeyword."""
    kw = MagicMock()
    kw.text = text
    kw.score = score
    kw.algorithm = algorithm
    return kw

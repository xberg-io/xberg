from __future__ import annotations

import os
from pathlib import Path

import pytest

os.environ["OMP_THREAD_LIMIT"] = "1"


@pytest.fixture(scope="session")
def searchable_pdf() -> Path:
    return Path(__file__).parent / "source" / "searchable.pdf"


@pytest.fixture(scope="session")
def scanned_pdf() -> Path:
    return Path(__file__).parent / "source" / "scanned.pdf"


@pytest.fixture(scope="session")
def german_pdf() -> Path:
    return Path(__file__).parent / "source" / "german.pdf"


@pytest.fixture(scope="session")
def non_searchable_pdf() -> Path:
    return Path(__file__).parent / "source" / "non-searchable.pdf"


@pytest.fixture(scope="session")
def non_ascii_pdf() -> Path:
    return Path(__file__).parent / "source" / "non-ascii-text.pdf"


@pytest.fixture(scope="session")
def ocr_image() -> Path:
    return Path(__file__).parent / "source" / "ocr-image.jpg"


@pytest.fixture(scope="session")
def docx_document() -> Path:
    return Path(__file__).parent / "source" / "document.docx"


@pytest.fixture(scope="session")
def markdown_document() -> Path:
    return Path(__file__).parent / "source" / "markdown.md"


@pytest.fixture(scope="session")
def pptx_document() -> Path:
    return Path(__file__).parent / "source" / "pitch-deck-presentation.pptx"


@pytest.fixture(scope="session")
def html_document() -> Path:
    return Path(__file__).parent / "source" / "html.html"


@pytest.fixture(scope="session")
def excel_document() -> Path:
    return Path(__file__).parent / "source" / "excel.xlsx"

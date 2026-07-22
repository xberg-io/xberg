"""Shared test fixtures for xberg-txtai."""

from pathlib import Path

import pytest

FIXTURES_DIR = Path(__file__).parent / "fixtures"


@pytest.fixture
def sample_txt_path() -> Path:
    """Path to a small test text file."""
    return FIXTURES_DIR / "sample.txt"


@pytest.fixture
def sample_pdf_path() -> Path:
    """Path to a small test PDF."""
    return FIXTURES_DIR / "sample.pdf"


@pytest.fixture
def sample_docx_path() -> Path:
    """Path to a small test DOCX file."""
    return FIXTURES_DIR / "sample.docx"


@pytest.fixture
def sample_html_path() -> Path:
    """Path to a small test HTML file."""
    return FIXTURES_DIR / "sample.html"

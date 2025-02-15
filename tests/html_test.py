"""Tests for HTML extraction functionality."""

from __future__ import annotations

from typing import TYPE_CHECKING

from kreuzberg._html import extract_html_string

if TYPE_CHECKING:
    from pathlib import Path


async def test_extract_html_string(html_document: Path) -> None:
    """Test extracting text from an HTML string."""
    result = await extract_html_string(html_document)
    assert isinstance(result.content, str)
    assert result.content.strip()
    assert result.mime_type == "text/markdown"


async def test_extract_html_string_bytes() -> None:
    """Test extracting text from HTML bytes."""
    html_content = b"<html><body><h1>Test</h1><p>This is a test.</p></body></html>"
    result = await extract_html_string(html_content)
    assert isinstance(result.content, str)
    assert result.content.strip()
    assert result.mime_type == "text/markdown"
    assert "Test" in result.content
    assert "This is a test." in result.content

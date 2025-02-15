"""Tests for PowerPoint extraction functionality."""

from __future__ import annotations

from typing import TYPE_CHECKING

from kreuzberg._pptx import extract_pptx_file_content

if TYPE_CHECKING:
    from pytest_mock import MockerFixture


async def test_extract_pptx_with_notes(mocker: MockerFixture) -> None:
    """Test extracting text from a PowerPoint file with notes."""
    mock_presentation = mocker.MagicMock()
    mock_slide = mocker.MagicMock()
    mock_notes_slide = mocker.MagicMock()
    mock_text_frame = mocker.MagicMock()

    mock_presentation.slides = [mock_slide]
    mock_slide.has_notes_slide = True
    mock_slide.notes_slide = mock_notes_slide
    mock_notes_slide.notes_text_frame = mock_text_frame
    mock_text_frame.text = "Test note content"

    mocker.patch("pptx.Presentation", return_value=mock_presentation)

    result = await extract_pptx_file_content(b"mock pptx content")

    assert "Test note content" in result.content
    assert result.mime_type == "text/markdown"

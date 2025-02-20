"""Tests for Excel extraction functionality."""

from __future__ import annotations

import sys
from typing import TYPE_CHECKING

import pytest

from kreuzberg import ExtractionResult, ParsingError
from kreuzberg._mime_types import MARKDOWN_MIME_TYPE
from kreuzberg._xlsx import extract_xlsx_file

if TYPE_CHECKING:
    from pathlib import Path

    from pytest_mock import MockerFixture


if sys.version_info < (3, 11):  # pragma: no cover
    from exceptiongroup import ExceptionGroup  # type: ignore[import-not-found]


@pytest.mark.anyio
async def test_extract_xlsx_file(excel_document: Path) -> None:
    """Test extracting text from an Excel file."""
    result = await extract_xlsx_file(excel_document)
    assert isinstance(result.content, str)
    assert result.content.strip()
    assert result.mime_type == "text/markdown"


@pytest.mark.anyio
async def test_extract_xlsx_multi_sheet_file(excel_multi_sheet_document: Path) -> None:
    """Test extracting text from an Excel file with multiple sheets."""
    result = await extract_xlsx_file(excel_multi_sheet_document)
    assert isinstance(result, ExtractionResult)
    assert result.mime_type == MARKDOWN_MIME_TYPE

    # Split content into sheets and their content
    sheets = result.content.split("\n\n")
    assert len(sheets) == 4  # Two pairs of sheet headers and content

    # Verify first sheet
    assert sheets[0] == "## first_sheet"
    first_sheet_content = sheets[1]
    assert "Column 1" in first_sheet_content
    assert "Column 2" in first_sheet_content
    assert "a" in first_sheet_content
    assert "1.0" in first_sheet_content
    assert "b" in first_sheet_content
    assert "2.0" in first_sheet_content
    assert "c" in first_sheet_content
    assert "3.0" in first_sheet_content

    # Verify second sheet
    assert sheets[2] == "## second_sheet"
    second_sheet_content = sheets[3]
    assert "Product" in second_sheet_content
    assert "Value" in second_sheet_content
    assert "Tomato" in second_sheet_content
    assert "Potato" in second_sheet_content
    assert "Beetroot" in second_sheet_content
    assert "1.0" in second_sheet_content
    assert "2.0" in second_sheet_content


@pytest.mark.anyio
async def test_extract_xlsx_file_exception_group(mocker: MockerFixture, excel_multi_sheet_document: Path) -> None:
    # Mock openpyxl to raise multiple exceptions
    mock_load = mocker.patch("kreuzberg._xlsx.run_taskgroup")
    exceptions = [ValueError("Error 1"), ValueError("Error 2")]
    mock_load.side_effect = ExceptionGroup("test group", exceptions)

    with pytest.raises(ParsingError) as exc_info:
        await extract_xlsx_file(excel_multi_sheet_document)

    assert "Failed to extract file data" in str(exc_info.value)
    assert len(exc_info.value.context["errors"]) == 2

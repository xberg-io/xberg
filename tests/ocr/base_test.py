"""Tests for OCR base classes."""

from __future__ import annotations

from typing import TYPE_CHECKING
from unittest.mock import Mock

import pytest
from PIL import Image

from kreuzberg._ocr._base import OCRBackend
from kreuzberg._types import ExtractionResult

if TYPE_CHECKING:
    from pathlib import Path


class TestOCRBackend(OCRBackend[dict[str, object]]):
    """Test implementation of OCRBackend."""

    async def process_image(self, image: Image.Image, **kwargs: dict[str, object]) -> ExtractionResult:
        """Test implementation of process_image."""
        return ExtractionResult(content="Test OCR result", mime_type="text/plain", metadata={}, chunks=[])

    async def process_file(self, path: Path, **kwargs: dict[str, object]) -> ExtractionResult:
        """Test implementation of process_file."""
        return ExtractionResult(content="Test file OCR result", mime_type="text/plain", metadata={}, chunks=[])


def test_ocr_backend_hash() -> None:
    """Test OCR backend hash method."""
    backend1 = TestOCRBackend()
    backend2 = TestOCRBackend()

    assert hash(backend1) == hash(backend2)
    assert hash(backend1) == hash("TestOCRBackend")


def test_ocr_backend_different_types_different_hash() -> None:
    """Test that different OCR backend types have different hashes."""

    class AnotherTestBackend(OCRBackend[dict[str, object]]):
        async def process_image(self, image: Image.Image, **kwargs: dict[str, object]) -> ExtractionResult:
            return ExtractionResult(content="", mime_type="text/plain", metadata={}, chunks=[])

        async def process_file(self, path: Path, **kwargs: dict[str, object]) -> ExtractionResult:
            return ExtractionResult(content="", mime_type="text/plain", metadata={}, chunks=[])

    backend1 = TestOCRBackend()
    backend2 = AnotherTestBackend()

    assert hash(backend1) != hash(backend2)


@pytest.mark.anyio
async def test_ocr_backend_process_image() -> None:
    """Test OCR backend process_image method."""
    backend = TestOCRBackend()
    image = Mock(spec=Image.Image)

    result = await backend.process_image(image)

    assert isinstance(result, ExtractionResult)
    assert result.content == "Test OCR result"
    assert result.mime_type == "text/plain"


@pytest.mark.anyio
async def test_ocr_backend_process_file(tmp_path: Path) -> None:
    """Test OCR backend process_file method."""
    backend = TestOCRBackend()
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")

    result = await backend.process_file(test_file)

    assert isinstance(result, ExtractionResult)
    assert result.content == "Test file OCR result"
    assert result.mime_type == "text/plain"

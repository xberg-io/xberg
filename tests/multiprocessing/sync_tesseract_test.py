"""Tests for synchronous Tesseract processing."""

from __future__ import annotations

import io
from pathlib import Path
from typing import Any
from unittest.mock import patch

import pytest
from PIL import Image

from kreuzberg._multiprocessing.sync_tesseract import (
    process_batch_images_process_pool,
    process_batch_images_sync_pure,
    process_batch_images_threaded,
    process_image_bytes_sync_pure,
    process_image_sync_pure,
)
from kreuzberg._ocr._tesseract import TesseractConfig
from kreuzberg._types import ExtractionResult
from kreuzberg.exceptions import OCRError


@pytest.fixture
def test_image() -> Image.Image:
    """Create a test image."""
    return Image.new("RGB", (100, 100), color="white")


@pytest.fixture
def test_image_path(tmp_path: Path) -> Path:
    """Create a test image file."""
    img_path = tmp_path / "test_image.png"
    img = Image.new("RGB", (100, 100), color="white")
    img.save(img_path)
    return img_path


@pytest.fixture
def test_image_paths(tmp_path: Path) -> list[Path]:
    """Create multiple test image files."""
    paths = []
    for i in range(3):
        img_path = tmp_path / f"test_image_{i}.png"
        img = Image.new("RGB", (50, 50), color="white")
        img.save(img_path)
        paths.append(img_path)
    return paths


@pytest.fixture
def tesseract_config() -> TesseractConfig:
    """Create a test Tesseract configuration."""
    return TesseractConfig(language="eng", psm=3)  # type: ignore[arg-type]


class _MockSubprocessResult:
    """Simple mock for subprocess result that can be pickled."""

    def __init__(self, returncode: int, stdout: str = "", stderr: str = ""):
        self.returncode = returncode
        self.stdout = stdout
        self.stderr = stderr


# Module-level functions for process pool tests (must be picklable)
def _mock_process_image_success(path: str, config_dict: dict[str, Any]) -> dict[str, Any]:
    """Mock successful image processing for process pool tests."""
    # Extract index from path to return appropriate result
    for i in range(3):
        if f"test_image_{i}" in path:
            return {
                "success": True,
                "text": f"Image {i} text",
                "mime_type": "text/plain",
                "metadata": {},
            }
    return {
        "success": True,
        "text": "Image 0 text",
        "mime_type": "text/plain",
        "metadata": {},
    }


def _mock_process_image_with_error(path: str, config_dict: dict[str, Any]) -> dict[str, Any]:
    """Mock image processing with error for process pool tests."""
    if "image_1" in str(path):
        return {"success": False, "error": "OCR failed"}
    return {"success": True, "text": "Success", "mime_type": "text/plain", "metadata": {}}


def _mock_process_image_with_exception(path: str, config_dict: dict[str, Any]) -> dict[str, Any]:
    """Mock image processing with exception for process pool tests."""
    if "image_1" in str(path):
        raise RuntimeError("Unexpected error")
    return {"success": True, "text": "Success", "mime_type": "text/plain", "metadata": {}}


def test_process_image_sync_pure_success(test_image_path: Path, tesseract_config: TesseractConfig) -> None:
    """Test successful synchronous image processing."""
    with patch("subprocess.run") as mock_run:
        mock_run.return_value = _MockSubprocessResult(returncode=0, stdout="", stderr="")

        with patch("pathlib.Path.open") as mock_open:
            mock_file = mock_open.return_value.__enter__.return_value
            mock_file.read.return_value = "Test OCR output"

            result = process_image_sync_pure(test_image_path, tesseract_config)

            assert isinstance(result, ExtractionResult)
            assert result.content == "Test OCR output"
            assert result.mime_type == "text/plain"


def test_process_image_sync_pure_error(test_image_path: Path, tesseract_config: TesseractConfig) -> None:
    """Test synchronous image processing with error."""
    with patch("subprocess.run") as mock_run:
        mock_run.return_value = _MockSubprocessResult(returncode=1, stderr="Tesseract error")

        with pytest.raises(OCRError) as exc_info:
            process_image_sync_pure(test_image_path, tesseract_config)

        assert "Tesseract failed" in str(exc_info.value)


def test_process_image_sync_pure_with_options(test_image_path: Path) -> None:
    """Test synchronous image processing with custom options."""
    config = TesseractConfig(
        language="fra",
        psm=6,  # type: ignore[arg-type]
        tessedit_enable_dict_correction=True,
        tessedit_use_primary_params_model=False,
    )

    with patch("subprocess.run") as mock_run:
        mock_run.return_value = _MockSubprocessResult(returncode=0)

        with patch("pathlib.Path.open") as mock_open:
            mock_file = mock_open.return_value.__enter__.return_value
            mock_file.read.return_value = "French text"

            process_image_sync_pure(test_image_path, config)

            args = mock_run.call_args[0][0]
            assert "-l" in args
            assert "fra" in args
            assert "--psm" in args
            assert "6" in args

            assert "-c" in args
            assert "tessedit_enable_dict_correction=1" in args
            assert "tessedit_use_primary_params_model=0" in args


def test_process_image_sync_pure_cleanup(test_image_path: Path, tesseract_config: TesseractConfig) -> None:
    """Test that temporary files are cleaned up."""
    with patch("subprocess.run") as mock_run:
        mock_run.return_value = _MockSubprocessResult(returncode=0)

        temp_files = []

        def mock_unlink(self: Any) -> None:
            temp_files.append(str(self))

        with patch.object(Path, "unlink", mock_unlink), patch("pathlib.Path.open") as mock_open:
            mock_file = mock_open.return_value.__enter__.return_value
            mock_file.read.return_value = "Text"

            process_image_sync_pure(test_image_path, tesseract_config)

        assert any(".txt" in f for f in temp_files)


def test_process_image_bytes_sync_pure(test_image: Image.Image, tesseract_config: TesseractConfig) -> None:
    """Test processing image bytes."""

    img_bytes = io.BytesIO()
    test_image.save(img_bytes, format="PNG")
    img_bytes_value = img_bytes.getvalue()

    with patch("kreuzberg._multiprocessing.sync_tesseract.process_image_sync_pure") as mock_process:
        mock_process.return_value = ExtractionResult(
            content="Bytes OCR output", mime_type="text/plain", metadata={}, chunks=[]
        )

        result = process_image_bytes_sync_pure(img_bytes_value, tesseract_config)

        assert result.content == "Bytes OCR output"
        mock_process.assert_called_once()

        call_args = mock_process.call_args[0]
        assert str(call_args[0]).endswith(".png")


def test_process_batch_images_sync_pure(test_image_paths: list[Path], tesseract_config: TesseractConfig) -> None:
    """Test batch processing with pure sync."""
    with patch("kreuzberg._multiprocessing.sync_tesseract.process_image_sync_pure") as mock_process:
        mock_results = [
            ExtractionResult(content=f"Image {i} text", mime_type="text/plain", metadata={}, chunks=[])
            for i in range(3)
        ]
        mock_process.side_effect = mock_results

        results = process_batch_images_sync_pure(test_image_paths, tesseract_config)  # type: ignore[arg-type]

        assert len(results) == 3
        for i, result in enumerate(results):
            assert result.content == f"Image {i} text"

        assert mock_process.call_count == 3


def test_process_batch_images_threaded(test_image_paths: list[Path], tesseract_config: TesseractConfig) -> None:
    """Test batch processing with threading."""
    with patch("kreuzberg._multiprocessing.sync_tesseract.process_image_sync_pure") as mock_process:
        mock_results = [
            ExtractionResult(content=f"Image {i} text", mime_type="text/plain", metadata={}, chunks=[])
            for i in range(3)
        ]

        def side_effect(path: Path, config: TesseractConfig) -> ExtractionResult:
            for i, test_path in enumerate(test_image_paths):
                if str(path) == str(test_path):
                    return mock_results[i]
            return mock_results[0]

        mock_process.side_effect = side_effect

        results = process_batch_images_threaded(test_image_paths, tesseract_config, max_workers=2)  # type: ignore[arg-type]

        assert len(results) == 3

        for i, result in enumerate(results):
            assert result.content == f"Image {i} text"


def test_process_batch_images_threaded_error_handling(
    test_image_paths: list[Path], tesseract_config: TesseractConfig
) -> None:
    """Test batch processing with threading handles errors."""

    def side_effect(path: Path, config: TesseractConfig) -> ExtractionResult:
        if "image_1" in str(path):
            raise RuntimeError("Processing failed")
        return ExtractionResult(content="Success", mime_type="text/plain", metadata={}, chunks=[])

    with patch("kreuzberg._multiprocessing.sync_tesseract.process_image_sync_pure", side_effect=side_effect):
        results = process_batch_images_threaded(test_image_paths, tesseract_config)  # type: ignore[arg-type]

        assert len(results) == 3
        assert results[0].content == "Success"
        assert "Error: Processing failed" in results[1].content
        assert results[2].content == "Success"


def test_process_batch_images_process_pool(test_image_paths: list[Path], tesseract_config: TesseractConfig) -> None:
    """Test batch processing with process pool."""
    with patch("kreuzberg._multiprocessing.tesseract_pool._process_image_with_tesseract", _mock_process_image_success):
        results = process_batch_images_process_pool(test_image_paths, tesseract_config, max_workers=2)  # type: ignore[arg-type]

        assert len(results) == 3
        for i, result in enumerate(results):
            assert result.content == f"Image {i} text"


def test_process_batch_images_process_pool_error_handling(
    test_image_paths: list[Path], tesseract_config: TesseractConfig
) -> None:
    """Test process pool batch processing handles errors."""
    with patch(
        "kreuzberg._multiprocessing.tesseract_pool._process_image_with_tesseract", _mock_process_image_with_error
    ):
        results = process_batch_images_process_pool(test_image_paths, tesseract_config)  # type: ignore[arg-type]

        assert len(results) == 3
        assert results[0].content == "Success"
        assert "Error: OCR failed" in results[1].content
        assert results[2].content == "Success"


def test_process_batch_images_process_pool_exception_handling(
    test_image_paths: list[Path], tesseract_config: TesseractConfig
) -> None:
    """Test process pool batch processing handles exceptions."""
    with patch(
        "kreuzberg._multiprocessing.tesseract_pool._process_image_with_tesseract", _mock_process_image_with_exception
    ):
        results = process_batch_images_process_pool(test_image_paths, tesseract_config)  # type: ignore[arg-type]

        assert len(results) == 3
        assert results[0].content == "Success"
        assert "Error: Unexpected error" in results[1].content
        assert results[2].content == "Success"


def test_process_image_sync_pure_unicode(test_image_path: Path, tesseract_config: TesseractConfig) -> None:
    """Test processing with unicode text output."""
    with patch("subprocess.run") as mock_run:
        mock_run.return_value = _MockSubprocessResult(returncode=0)

        unicode_text = "Hello 世界! Привет мир! مرحبا بالعالم"
        with patch("pathlib.Path.open") as mock_open:
            mock_file = mock_open.return_value.__enter__.return_value
            mock_file.read.return_value = unicode_text

            result = process_image_sync_pure(test_image_path, tesseract_config)

            assert result.content == unicode_text


def test_process_image_sync_pure_empty_output(test_image_path: Path, tesseract_config: TesseractConfig) -> None:
    """Test processing with empty text output."""
    with patch("subprocess.run") as mock_run:
        mock_run.return_value = _MockSubprocessResult(returncode=0)

        with patch("pathlib.Path.open") as mock_open:
            mock_file = mock_open.return_value.__enter__.return_value
            mock_file.read.return_value = ""

            result = process_image_sync_pure(test_image_path, tesseract_config)

            assert result.content == ""
            assert result.mime_type == "text/plain"


def test_process_image_sync_pure_default_config(test_image_path: Path) -> None:
    """Test processing with default config."""
    with patch("subprocess.run") as mock_run:
        mock_run.return_value = _MockSubprocessResult(returncode=0)

        with patch("pathlib.Path.open") as mock_open:
            mock_file = mock_open.return_value.__enter__.return_value
            mock_file.read.return_value = "Default config output"

            result = process_image_sync_pure(test_image_path)

            assert result.content == "Default config output"

            args = mock_run.call_args[0][0]
            assert "-l" in args
            assert "eng" in args


def test_config_dict_conversion_in_process_pool() -> None:
    """Test that config is properly converted to dict for process pool."""
    from enum import Enum

    class TestEnum(Enum):
        VALUE = 3

    config = TesseractConfig()

    config_dict = {}
    for field_name in config.__dataclass_fields__:
        value = getattr(config, field_name)
        if hasattr(value, "value") and hasattr(value, "__class__") and issubclass(value.__class__, Enum):
            config_dict[field_name] = value.value
        else:
            config_dict[field_name] = value

    assert isinstance(config_dict["psm"], int)
    # The psm value should be either the enum value (3) or direct int (3)
    expected_psm: Any = 3
    expected_psm = config.psm.value if hasattr(config.psm, "value") else config.psm
    assert config_dict["psm"] == expected_psm

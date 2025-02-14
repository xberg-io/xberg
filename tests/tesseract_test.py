from __future__ import annotations

from pathlib import Path
from typing import TYPE_CHECKING, Any, cast
from unittest.mock import Mock

import pytest
from PIL import Image

from kreuzberg import ExtractionResult
from kreuzberg._tesseract import (
    PSMMode,
    batch_process_images,
    process_file,
    process_image,
    process_image_with_tesseract,
    validate_tesseract_version,
)
from kreuzberg.exceptions import MissingDependencyError, OCRError, ParsingError

if TYPE_CHECKING:
    from os import PathLike

    from pytest_mock import MockerFixture


@pytest.fixture
def mock_subprocess_run(mocker: MockerFixture) -> Mock:
    def run_sync(*args: list[Any], **kwargs: dict[str, Any]) -> Mock:
        result = Mock()
        result.stdout = b"tesseract 5.0.0"
        result.returncode = 0
        result.stderr = b""

        if isinstance(args[0], list) and "--version" in args[0]:
            return result

        # Handle error test cases
        if "test_process_file_error" in str(kwargs.get("cwd")):
            result.returncode = 1
            result.stderr = b"Error processing file"
            raise RuntimeError("Error processing file")

        if "test_process_file_runtime_error" in str(kwargs.get("cwd")):
            raise RuntimeError("Command failed")

        # Normal case
        if isinstance(args[0], list) and len(args[0]) >= 3:
            output_file = args[0][2]
            if "test_process_image_with_tesseract_invalid_input" in str(kwargs.get("cwd")):
                result.returncode = 1
                result.stderr = b"Error processing file"
                raise RuntimeError("Error processing file")
            Path(f"{output_file}.txt").write_text("Sample OCR text")
            result.returncode = 0
            return result

        return result

    # Mock both subprocess.run and anyio.to_process.run_sync
    mock = mocker.patch("subprocess.run", side_effect=run_sync)
    mocker.patch("anyio.to_process.run_sync", side_effect=lambda func, *args, **kwargs: func(*args, **kwargs))
    return mock


@pytest.fixture
def mock_subprocess_run_invalid(mocker: MockerFixture) -> Mock:
    mock = mocker.patch("subprocess.run")
    mock.return_value.stdout = b"tesseract 4.0.0"
    mock.return_value.returncode = 0
    return mock


@pytest.fixture
def mock_subprocess_run_error(mocker: MockerFixture) -> Mock:
    mock = mocker.patch("subprocess.run")
    mock.side_effect = FileNotFoundError()
    return mock


async def test_validate_tesseract_version(mock_subprocess_run: Mock) -> None:
    await validate_tesseract_version()
    mock_subprocess_run.assert_called_with(["tesseract", "--version"], capture_output=True)


@pytest.fixture(autouse=True)
def reset_version_ref(mocker: MockerFixture) -> None:
    mocker.patch("kreuzberg._tesseract.version_ref", {"checked": False})


async def test_validate_tesseract_version_invalid(mock_subprocess_run_invalid: Mock) -> None:
    with pytest.raises(MissingDependencyError, match="Tesseract version 5 or above is required"):
        await validate_tesseract_version()


async def test_validate_tesseract_version_missing(mock_subprocess_run_error: Mock) -> None:
    with pytest.raises(MissingDependencyError, match="Tesseract is not installed"):
        await validate_tesseract_version()


async def test_process_file(mock_subprocess_run: Mock, ocr_image: Path) -> None:
    result = await process_file(ocr_image, language="eng", psm=PSMMode.AUTO)
    assert isinstance(result, ExtractionResult)
    assert result.content.strip() == "Sample OCR text"


async def test_process_file_with_options(mock_subprocess_run: Mock, ocr_image: Path) -> None:
    result = await process_file(ocr_image, language="eng", psm=PSMMode.AUTO)
    assert isinstance(result, ExtractionResult)
    assert result.content.strip() == "Sample OCR text"


async def test_process_file_error(mock_subprocess_run: Mock, ocr_image: Path) -> None:
    mock_subprocess_run.return_value.returncode = 1
    mock_subprocess_run.side_effect = None
    with pytest.raises(OCRError, match="OCR failed with a non-0 return code"):
        await process_file(ocr_image, language="eng", psm=PSMMode.AUTO)


async def test_process_file_runtime_error(mock_subprocess_run: Mock, ocr_image: Path) -> None:
    mock_subprocess_run.side_effect = RuntimeError()
    with pytest.raises(OCRError, match="Failed to OCR using tesseract"):
        await process_file(ocr_image, language="eng", psm=PSMMode.AUTO)


async def test_process_image(mock_subprocess_run: Mock) -> None:
    image = Image.new("RGB", (100, 100))
    result = await process_image(image, language="eng", psm=PSMMode.AUTO)
    assert isinstance(result, ExtractionResult)
    assert result.content.strip() == "Sample OCR text"


async def test_process_image_with_tesseract_pillow(mock_subprocess_run: Mock) -> None:
    image = Image.new("RGB", (100, 100))
    result = await process_image_with_tesseract(image)
    assert isinstance(result, ExtractionResult)
    assert result.content.strip() == "Sample OCR text"


async def test_process_image_with_tesseract_path(mock_subprocess_run: Mock, ocr_image: Path) -> None:
    result = await process_image_with_tesseract(ocr_image)
    assert isinstance(result, ExtractionResult)
    assert result.content.strip() == "Sample OCR text"


async def test_process_image_with_tesseract_invalid_input() -> None:
    with pytest.raises(ValueError, match="Input must be one of: str, Pathlike or Pillow Image"):
        await process_image_with_tesseract([])  # type: ignore


async def test_batch_process_images_pillow(mock_subprocess_run: Mock) -> None:
    images = [Image.new("RGB", (100, 100)) for _ in range(3)]
    results = await batch_process_images(images, language="eng", psm=PSMMode.AUTO, max_tesseract_concurrency=1)
    assert isinstance(results, list)
    assert all(isinstance(result, ExtractionResult) for result in results)
    assert all(result.content.strip() == "Sample OCR text" for result in results)


async def test_batch_process_images_paths(mock_subprocess_run: Mock, ocr_image: Path) -> None:
    images = [str(ocr_image)] * 3
    results = await batch_process_images(images, language="eng", psm=PSMMode.AUTO, max_tesseract_concurrency=1)
    assert isinstance(results, list)
    assert all(isinstance(result, ExtractionResult) for result in results)
    assert all(result.content.strip() == "Sample OCR text" for result in results)


async def test_batch_process_images_mixed(mock_subprocess_run: Mock, ocr_image: Path) -> None:
    images: list[Image.Image | PathLike[str] | str] = [
        Image.new("RGB", (100, 100)),
        str(ocr_image),
        str(ocr_image),
    ]
    results = await batch_process_images(images, language="eng", psm=PSMMode.AUTO, max_tesseract_concurrency=1)
    assert isinstance(results, list)
    assert all(isinstance(result, ExtractionResult) for result in results)
    assert all(result.content.strip() == "Sample OCR text" for result in results)


async def test_integration_validate_tesseract_version() -> None:
    await validate_tesseract_version()


async def test_integration_process_file(ocr_image: Path) -> None:
    result = await process_file(ocr_image, language="eng", psm=PSMMode.AUTO)
    assert isinstance(result, ExtractionResult)
    assert result.content.strip()


async def test_integration_process_file_with_options(ocr_image: Path) -> None:
    result = await process_file(ocr_image, language="eng", psm=PSMMode.AUTO)
    assert isinstance(result, ExtractionResult)
    assert result.content.strip()


async def test_integration_process_image(ocr_image: Path) -> None:
    image = Image.open(ocr_image)
    with image:
        result = await process_image(image, language="eng", psm=PSMMode.AUTO)
        assert isinstance(result, ExtractionResult)
        assert result.content.strip()


async def test_integration_process_image_with_tesseract_pillow(ocr_image: Path) -> None:
    image = Image.open(ocr_image)
    with image:
        result = await process_image_with_tesseract(image)
        assert isinstance(result, ExtractionResult)
        assert result.content.strip()


async def test_integration_process_image_with_tesseract_path(ocr_image: Path) -> None:
    result = await process_image_with_tesseract(ocr_image)
    assert isinstance(result, ExtractionResult)
    assert result.content.strip()


async def test_integration_batch_process_images_pillow(ocr_image: Path) -> None:
    image = Image.open(ocr_image)
    with image:
        images = [image.copy() for _ in range(3)]
        results = await batch_process_images(images, language="eng", psm=PSMMode.AUTO, max_tesseract_concurrency=1)
        assert isinstance(results, list)
        assert len(results) == 3
        assert all(isinstance(result, ExtractionResult) for result in results)
        assert all(result.content.strip() for result in results)


async def test_integration_batch_process_images_paths(ocr_image: Path) -> None:
    images = [str(ocr_image)] * 3
    results = await batch_process_images(images, language="eng", psm=PSMMode.AUTO, max_tesseract_concurrency=1)
    assert isinstance(results, list)
    assert len(results) == 3
    assert all(isinstance(result, ExtractionResult) for result in results)
    assert all(result.content.strip() for result in results)


async def test_integration_batch_process_images_mixed(ocr_image: Path) -> None:
    image = Image.open(ocr_image)
    with image:
        images: list[Image.Image | PathLike[str] | str] = [image.copy(), ocr_image, str(ocr_image)]
        results = await batch_process_images(images, language="eng", psm=PSMMode.AUTO, max_tesseract_concurrency=1)
        assert isinstance(results, list)
        assert len(results) == 3
        assert all(isinstance(result, ExtractionResult) for result in results)
        assert all(result.content.strip() for result in results)


async def test_batch_process_images_exception_group(mock_subprocess_run: Mock) -> None:
    """Test that batch_process_images handles ExceptionGroup correctly."""

    def side_effect(*args: list[Any], **kwargs: dict[str, Any]) -> Mock:
        if args[0][0] == "tesseract" and "--version" in args[0]:
            mock_subprocess_run.return_value.stdout = b"tesseract 5.0.0"
            return cast(Mock, mock_subprocess_run.return_value)
        raise RuntimeError("Tesseract error")

    mock_subprocess_run.side_effect = side_effect
    image = Image.new("RGB", (100, 100))

    with pytest.raises(ParsingError, match="Failed to process images with Tesseract"):
        await batch_process_images([image], language="eng", psm=PSMMode.AUTO, max_tesseract_concurrency=1)

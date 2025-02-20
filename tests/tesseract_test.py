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
def mock_run_process(mocker: MockerFixture) -> Mock:
    def run_sync(command: list[str], **kwargs: Any) -> Mock:
        result = Mock()
        result.stdout = b"tesseract 5.0.0"
        result.returncode = 0
        result.stderr = b""

        if "--version" in command and command[0].endswith("tesseract"):
            return result

        # Handle error test cases
        if "test_process_file_error" in str(kwargs.get("cwd")):
            result.returncode = 1
            result.stderr = b"Error processing file"
            raise OCRError("Error processing file")

        if "test_process_file_runtime_error" in str(kwargs.get("cwd")):
            raise RuntimeError("Command failed")

        # Normal case
        if len(command) >= 3 and command[0].endswith("tesseract"):
            output_file = command[2]
            if "test_process_image_with_tesseract_invalid_input" in str(kwargs.get("cwd")):
                result.returncode = 1
                result.stderr = b"Error processing file"
                raise OCRError("Error processing file")

            # Verify required tesseract arguments
            if not all(arg in command for arg in ["--oem", "1", "--loglevel", "OFF", "-c", "thresholding_method=1"]):
                result.returncode = 1
                result.stderr = b"Missing required tesseract arguments"
                return result

            Path(f"{output_file}.txt").write_text("Sample OCR text")
            result.returncode = 0
            return result

        return result

    return mocker.patch("kreuzberg._tesseract.run_process", side_effect=run_sync)


@pytest.fixture
def mock_run_process_invalid(mocker: MockerFixture) -> Mock:
    def run_sync(command: list[str], **kwargs: Any) -> Mock:
        result = Mock()
        result.stdout = b"tesseract 4.0.0"
        result.returncode = 0
        result.stderr = b""
        return result

    return mocker.patch("kreuzberg._tesseract.run_process", side_effect=run_sync)


@pytest.fixture
def mock_run_process_error(mocker: MockerFixture) -> Mock:
    def run_sync(command: list[str], **kwargs: Any) -> Mock:
        raise FileNotFoundError

    return mocker.patch("kreuzberg._tesseract.run_process", side_effect=run_sync)


@pytest.mark.anyio
async def test_validate_tesseract_version(mock_run_process: Mock) -> None:
    await validate_tesseract_version()
    mock_run_process.assert_called_with(["tesseract", "--version"])


@pytest.fixture(autouse=True)
def reset_version_ref(mocker: MockerFixture) -> None:
    mocker.patch("kreuzberg._tesseract.version_ref", {"checked": False})


@pytest.mark.anyio
async def test_validate_tesseract_version_invalid(mock_run_process_invalid: Mock, reset_version_ref: None) -> None:
    with pytest.raises(MissingDependencyError, match="Tesseract version 5 or above is required"):
        await validate_tesseract_version()


@pytest.mark.anyio
async def test_validate_tesseract_version_missing(mock_run_process_error: Mock, reset_version_ref: None) -> None:
    with pytest.raises(MissingDependencyError, match="Tesseract is not installed"):
        await validate_tesseract_version()


@pytest.mark.anyio
async def test_process_file(mock_run_process: Mock, ocr_image: Path) -> None:
    result = await process_file(ocr_image, language="eng", psm=PSMMode.AUTO)
    assert isinstance(result, ExtractionResult)
    assert result.content.strip() == "Sample OCR text"


@pytest.mark.anyio
async def test_process_file_with_options(mock_run_process: Mock, ocr_image: Path) -> None:
    result = await process_file(ocr_image, language="eng", psm=PSMMode.AUTO)
    assert isinstance(result, ExtractionResult)
    assert result.content.strip() == "Sample OCR text"


@pytest.mark.anyio
async def test_process_file_error(mock_run_process: Mock, ocr_image: Path) -> None:
    mock_run_process.return_value.returncode = 1
    mock_run_process.return_value.stderr = b"Error processing file"
    mock_run_process.side_effect = None
    with pytest.raises(OCRError, match="OCR failed with a non-0 return code"):
        await process_file(ocr_image, language="eng", psm=PSMMode.AUTO)


@pytest.mark.anyio
async def test_process_file_runtime_error(mock_run_process: Mock, ocr_image: Path) -> None:
    mock_run_process.side_effect = RuntimeError()
    with pytest.raises(OCRError, match="Failed to OCR using tesseract"):
        await process_file(ocr_image, language="eng", psm=PSMMode.AUTO)


@pytest.mark.anyio
async def test_process_image(mock_run_process: Mock) -> None:
    image = Image.new("RGB", (100, 100))
    result = await process_image(image, language="eng", psm=PSMMode.AUTO)
    assert isinstance(result, ExtractionResult)
    assert result.content.strip() == "Sample OCR text"


@pytest.mark.anyio
async def test_process_image_with_tesseract_pillow(mock_run_process: Mock) -> None:
    image = Image.new("RGB", (100, 100))
    result = await process_image_with_tesseract(image)
    assert isinstance(result, ExtractionResult)
    assert result.content.strip() == "Sample OCR text"


@pytest.mark.anyio
async def test_process_image_with_tesseract_path(mock_run_process: Mock, ocr_image: Path) -> None:
    result = await process_image_with_tesseract(ocr_image)
    assert isinstance(result, ExtractionResult)
    assert result.content.strip() == "Sample OCR text"


@pytest.mark.anyio
async def test_process_image_with_tesseract_invalid_input() -> None:
    with pytest.raises(ValueError, match="Input must be one of: str, Pathlike or Pillow Image"):
        await process_image_with_tesseract([])  # type: ignore


@pytest.mark.anyio
async def test_batch_process_images_pillow(mock_run_process: Mock) -> None:
    images = [Image.new("RGB", (100, 100)) for _ in range(3)]
    results = await batch_process_images(images, language="eng", psm=PSMMode.AUTO, max_processes=1)
    assert isinstance(results, list)
    assert all(isinstance(result, ExtractionResult) for result in results)
    assert all(result.content.strip() == "Sample OCR text" for result in results)


@pytest.mark.anyio
async def test_batch_process_images_paths(mock_run_process: Mock, ocr_image: Path) -> None:
    images = [str(ocr_image)] * 3
    results = await batch_process_images(images, language="eng", psm=PSMMode.AUTO, max_processes=1)
    assert isinstance(results, list)
    assert all(isinstance(result, ExtractionResult) for result in results)
    assert all(result.content.strip() == "Sample OCR text" for result in results)


@pytest.mark.anyio
async def test_batch_process_images_mixed(mock_run_process: Mock, ocr_image: Path) -> None:
    images: list[Image.Image | PathLike[str] | str] = [
        Image.new("RGB", (100, 100)),
        str(ocr_image),
        str(ocr_image),
    ]
    results = await batch_process_images(images, language="eng", psm=PSMMode.AUTO, max_processes=1)
    assert isinstance(results, list)
    assert all(isinstance(result, ExtractionResult) for result in results)
    assert all(result.content.strip() == "Sample OCR text" for result in results)


@pytest.mark.anyio
async def test_integration_validate_tesseract_version() -> None:
    await validate_tesseract_version()


@pytest.mark.anyio
async def test_integration_process_file(ocr_image: Path) -> None:
    result = await process_file(ocr_image, language="eng", psm=PSMMode.AUTO)
    assert isinstance(result, ExtractionResult)
    assert result.content.strip()


@pytest.mark.anyio
async def test_integration_process_file_with_options(ocr_image: Path) -> None:
    result = await process_file(ocr_image, language="eng", psm=PSMMode.AUTO)
    assert isinstance(result, ExtractionResult)
    assert result.content.strip()


@pytest.mark.anyio
async def test_integration_process_image(ocr_image: Path) -> None:
    image = Image.open(ocr_image)
    with image:
        result = await process_image(image, language="eng", psm=PSMMode.AUTO)
        assert isinstance(result, ExtractionResult)
        assert result.content.strip()


@pytest.mark.anyio
async def test_integration_process_image_with_tesseract_pillow(ocr_image: Path) -> None:
    image = Image.open(ocr_image)
    with image:
        result = await process_image_with_tesseract(image)
        assert isinstance(result, ExtractionResult)
        assert result.content.strip()


@pytest.mark.anyio
async def test_integration_process_image_with_tesseract_path(ocr_image: Path) -> None:
    result = await process_image_with_tesseract(ocr_image)
    assert isinstance(result, ExtractionResult)
    assert result.content.strip()


@pytest.mark.anyio
async def test_integration_batch_process_images_pillow(ocr_image: Path) -> None:
    image = Image.open(ocr_image)
    with image:
        images = [image.copy() for _ in range(3)]
        results = await batch_process_images(images, language="eng", psm=PSMMode.AUTO, max_processes=1)
        assert isinstance(results, list)
        assert len(results) == 3
        assert all(isinstance(result, ExtractionResult) for result in results)
        assert all(result.content.strip() for result in results)


@pytest.mark.anyio
async def test_integration_batch_process_images_paths(ocr_image: Path) -> None:
    images = [str(ocr_image)] * 3
    results = await batch_process_images(images, language="eng", psm=PSMMode.AUTO, max_processes=1)
    assert isinstance(results, list)
    assert len(results) == 3
    assert all(isinstance(result, ExtractionResult) for result in results)
    assert all(result.content.strip() for result in results)


@pytest.mark.anyio
async def test_integration_batch_process_images_mixed(ocr_image: Path) -> None:
    image = Image.open(ocr_image)
    with image:
        images: list[Image.Image | PathLike[str] | str] = [image.copy(), ocr_image, str(ocr_image)]
        results = await batch_process_images(images, language="eng", psm=PSMMode.AUTO, max_processes=1)
        assert isinstance(results, list)
        assert len(results) == 3
        assert all(isinstance(result, ExtractionResult) for result in results)
        assert all(result.content.strip() for result in results)


@pytest.mark.anyio
async def test_batch_process_images_exception_group(mock_run_process: Mock) -> None:
    def side_effect(*args: list[Any], **kwargs: dict[str, Any]) -> Mock:
        if args[0][0] == "tesseract" and "--version" in args[0]:
            mock_run_process.return_value.stdout = b"tesseract 5.0.0"
            return cast(Mock, mock_run_process.return_value)
        raise RuntimeError("Tesseract error")

    mock_run_process.side_effect = side_effect
    image = Image.new("RGB", (100, 100))

    with pytest.raises(ParsingError, match="Failed to process images with Tesseract"):
        await batch_process_images([image], language="eng", psm=PSMMode.AUTO, max_processes=1)


@pytest.mark.anyio
async def test_process_file_linux(mocker: MockerFixture) -> None:
    # Mock sys.platform to simulate Linux
    mocker.patch("sys.platform", "linux")

    mock_run = mocker.patch("kreuzberg._tesseract.run_process")
    mock_run.return_value.returncode = 0
    mock_run.return_value.stdout = b"test output"

    await process_file("test.png", language="eng", psm=PSMMode.AUTO)

    # Verify that OMP_THREAD_LIMIT was set for Linux
    mock_run.assert_called_once()
    assert mock_run.call_args[1]["env"] == {"OMP_THREAD_LIMIT": "1"}

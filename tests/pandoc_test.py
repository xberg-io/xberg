from __future__ import annotations

from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from collections.abc import Coroutine
    from unittest.mock import Mock

import pytest

from kreuzberg import ExtractionResult
from kreuzberg._pandoc import (
    MIMETYPE_TO_PANDOC_TYPE_MAPPING,
    _extract_inline_text,
    _extract_inlines,
    _extract_meta_value,
    _get_pandoc_type_from_mime_type,
    _handle_extract_file,
    _handle_extract_metadata,
    _validate_pandoc_version,
    process_content_with_pandoc,
    process_file_with_pandoc,
)
from kreuzberg.exceptions import MissingDependencyError, ParsingError, ValidationError

if TYPE_CHECKING:
    from collections.abc import Callable

    from pytest_mock import MockerFixture

SAMPLE_PANDOC_JSON = {
    "pandoc-api-version": [1, 23, 1],
    "meta": {"title": {"t": "MetaString", "c": "Test Document"}, "author": {"t": "MetaString", "c": "Test Author"}},
    "blocks": [],
}


@pytest.fixture
def mock_run_process(mocker: MockerFixture) -> Mock:
    return mocker.patch("kreuzberg._pandoc.run_process")


@pytest.fixture
def mock_version_check(mocker: MockerFixture) -> None:
    mocker.patch("kreuzberg._pandoc.version_ref", {"checked": True})


@pytest.fixture
def mock_run_taskgroup(mocker: MockerFixture) -> Mock:
    return mocker.patch("kreuzberg._pandoc.run_taskgroup")


@pytest.mark.anyio
@pytest.mark.parametrize(
    "major_version, should_raise",
    [
        (1, True),
        (2, False),
        (3, False),
    ],
)
async def test_validate_pandoc_version(
    mocker: MockerFixture, mock_run_process: Mock, major_version: int, should_raise: bool
) -> None:
    mocker.patch("kreuzberg._pandoc.version_ref", {"checked": False})

    mock_run_process.return_value.returncode = 0
    mock_run_process.return_value.stderr = b""
    mock_run_process.return_value.stdout = f"pandoc {major_version}.1.0".encode()

    if should_raise:
        with pytest.raises(MissingDependencyError):
            await _validate_pandoc_version()
    else:
        await _validate_pandoc_version()

    mock_run_process.assert_called_with(["pandoc", "--version"])


@pytest.mark.parametrize(
    "node, expected_output",
    [
        ({"t": "Str", "c": "Hello"}, "Hello"),
        ({"t": "Space", "c": " "}, " "),
        ({"t": "Emph", "c": [{"t": "Str", "c": "Emphasized"}]}, "Emphasized"),
    ],
)
def test_extract_inline_text(node: dict[str, Any], expected_output: str) -> None:
    assert _extract_inline_text(node) == expected_output


@pytest.mark.parametrize(
    "nodes, expected_output",
    [
        ([{"t": "Str", "c": "Hello"}, {"t": "Space", "c": " "}, {"t": "Str", "c": "World"}], "Hello World"),
        ([{"t": "Emph", "c": [{"t": "Str", "c": "Emphasized"}]}], "Emphasized"),
    ],
)
def test_extract_inlines(nodes: list[dict[str, Any]], expected_output: str) -> None:
    assert _extract_inlines(nodes) == expected_output


@pytest.mark.parametrize(
    "node, expected_output",
    [
        ({"t": "MetaString", "c": "Test String"}, "Test String"),
        ({"t": "MetaInlines", "c": [{"t": "Str", "c": "Inline String"}]}, "Inline String"),
        ({"t": "MetaList", "c": [{"t": "MetaString", "c": "List Item"}]}, ["List Item"]),
    ],
)
def test_extract_meta_value(node: Any, expected_output: Any) -> None:
    assert _extract_meta_value(node) == expected_output


@pytest.mark.parametrize("mime_type, expected_type", MIMETYPE_TO_PANDOC_TYPE_MAPPING.items())
def test_get_pandoc_type_from_mime_type_all_mappings(mime_type: str, expected_type: str) -> None:
    assert _get_pandoc_type_from_mime_type(mime_type) == expected_type


# Mock the Pandoc version check
@pytest.fixture(autouse=True)
def mock_pandoc_version(mocker: MockerFixture) -> None:
    # Mock the version_ref to avoid version check
    mocker.patch("kreuzberg._pandoc.version_ref", {"checked": True})


@pytest.fixture
def mock_temp_file(mocker: MockerFixture) -> None:
    # Mock create_temp_file to return a predictable path
    async def mock_create(_: Any) -> tuple[str, Callable[[], Coroutine[None, None, None]]]:
        async def mock_unlink() -> None:
            pass

        return "/tmp/test", mock_unlink

    mocker.patch("kreuzberg._pandoc.create_temp_file", side_effect=mock_create)


@pytest.fixture
def mock_async_path(mocker: MockerFixture) -> None:
    # Mock AsyncPath operations
    mock_path = mocker.patch("kreuzberg._pandoc.AsyncPath")
    mock_path.return_value.read_text = mocker.AsyncMock(return_value="Test content")
    mock_path.return_value.write_bytes = mocker.AsyncMock()


@pytest.mark.anyio
async def test_handle_extract_file(mock_run_process: Mock) -> None:
    mock_run_process.return_value.returncode = 0
    mock_run_process.return_value.stdout = b"Test content"

    result = await _handle_extract_file("dummy_file", mime_type="application/csl+json")
    assert isinstance(result, str)


@pytest.mark.anyio
async def test_process_file_with_pandoc(mock_version_check: Mock, mock_run_taskgroup: Mock) -> None:
    mock_run_taskgroup.return_value = ({"title": "Test Document"}, "Test Content")

    result = await process_file_with_pandoc("dummy_file", mime_type="application/csl+json")
    assert isinstance(result, ExtractionResult)
    assert result.metadata["title"] == "Test Document"
    assert result.content == "Test Content"


@pytest.mark.anyio
async def test_process_content_with_pandoc(mock_version_check: Mock, mock_run_taskgroup: Mock) -> None:
    mock_run_taskgroup.return_value = ({"title": "Test Document"}, "Test Content")
    result = await process_content_with_pandoc(b"Test Content", mime_type="application/csl+json")
    assert isinstance(result, ExtractionResult)
    assert result.metadata["title"] == "Test Document"
    assert result.content == "Test Content"


@pytest.mark.anyio
async def test_validate_pandoc_version_file_not_found(mocker: MockerFixture) -> None:
    mocker.patch("kreuzberg._pandoc.version_ref", {"checked": False})
    mock_run = mocker.patch("kreuzberg._pandoc.run_process")
    mock_run.side_effect = FileNotFoundError()

    with pytest.raises(MissingDependencyError, match="Pandoc is not installed"):
        await _validate_pandoc_version()


@pytest.mark.anyio
async def test_validate_pandoc_version_invalid_output(mocker: MockerFixture) -> None:
    mocker.patch("kreuzberg._pandoc.version_ref", {"checked": False})
    mock_run = mocker.patch("kreuzberg._pandoc.run_process")
    mock_run.return_value.stdout = b"invalid version output"

    with pytest.raises(MissingDependencyError, match="MissingDependencyError: Pandoc version 2 or above is required"):
        await _validate_pandoc_version()


@pytest.mark.anyio
async def test_validate_pandoc_version_parse_error(mocker: MockerFixture) -> None:
    mocker.patch("kreuzberg._pandoc.version_ref", {"checked": False})
    mock_run = mocker.patch("kreuzberg._pandoc.run_process")
    mock_run.return_value.stdout = b"pandoc abc"

    with pytest.raises(MissingDependencyError, match="Pandoc version 2 or above is required"):
        await _validate_pandoc_version()


@pytest.mark.anyio
async def test_handle_extract_metadata_runtime_error(
    mock_run_process: Mock, mock_temp_file: None, mock_async_path: None
) -> None:
    mock_run_process.side_effect = RuntimeError("Runtime error")

    with pytest.raises(ParsingError, match="Failed to extract file data"):
        await _handle_extract_metadata("dummy_file", mime_type="application/csl+json")


@pytest.mark.anyio
async def test_handle_extract_file_runtime_error(
    mock_run_process: Mock, mock_temp_file: None, mock_async_path: None
) -> None:
    mock_run_process.side_effect = RuntimeError("Runtime error")

    with pytest.raises(ParsingError, match="Failed to extract file data"):
        await _handle_extract_file("dummy_file", mime_type="application/csl+json")


@pytest.mark.anyio
async def test_process_content_with_pandoc_runtime_error(
    mock_version_check: None, mock_temp_file: None, mock_async_path: None, mock_run_process: Mock
) -> None:
    mock_run_process.side_effect = RuntimeError("Runtime error")

    with pytest.raises(ParsingError, match="Failed to process file"):
        await process_content_with_pandoc(b"Test content", mime_type="application/csl+json")


def test_get_pandoc_type_unsupported_mime() -> None:
    with pytest.raises(ValidationError, match="Unsupported mime type: invalid/type"):
        _get_pandoc_type_from_mime_type("invalid/type")


def test_get_pandoc_type_prefix_match() -> None:
    assert _get_pandoc_type_from_mime_type("application/csl") == "csljson"


@pytest.mark.anyio
async def test_handle_extract_metadata_error(
    mock_run_process: Mock, mock_temp_file: None, mock_async_path: None
) -> None:
    mock_run_process.return_value.returncode = 1
    mock_run_process.return_value.stderr = b"Error processing file"

    with pytest.raises(ParsingError, match="Failed to extract file data"):
        await _handle_extract_metadata("dummy_file", mime_type="application/csl+json")


@pytest.mark.anyio
async def test_handle_extract_file_error(mock_run_process: Mock, mock_temp_file: None, mock_async_path: None) -> None:
    mock_run_process.return_value.returncode = 1
    mock_run_process.return_value.stderr = b"Error processing file"

    with pytest.raises(ParsingError, match="Failed to extract file data"):
        await _handle_extract_file("dummy_file", mime_type="application/csl+json")


@pytest.mark.anyio
async def test_process_content_with_pandoc_error(
    mock_version_check: None, mock_temp_file: None, mock_async_path: None, mock_run_process: Mock
) -> None:
    mock_run_process.return_value.returncode = 1
    mock_run_process.return_value.stderr = b"Error processing file"
    with pytest.raises(ValidationError, match="Unsupported mime type: invalid/type"):
        await process_content_with_pandoc(b"Invalid content", mime_type="invalid/type")

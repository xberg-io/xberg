from __future__ import annotations

import json
from pathlib import Path
from typing import TYPE_CHECKING, Any, cast
from unittest.mock import Mock

import pytest

from kreuzberg._pandoc import (
    PANDOC_MIMETYPE_TO_FORMAT_MAPPING,
    PandocResult,
    _get_extension_from_mime_type,
    extract_metadata,
    process_content,
    process_file,
    validate_pandoc_version,
)
from kreuzberg.exceptions import MissingDependencyError, ParsingError, ValidationError

if TYPE_CHECKING:
    from pytest_mock import MockerFixture

SAMPLE_PANDOC_JSON = {
    "pandoc-api-version": [1, 23, 1],
    "meta": {"title": {"t": "MetaString", "c": "Test Document"}, "author": {"t": "MetaString", "c": "Test Author"}},
    "blocks": [],
}


@pytest.fixture
def mock_subprocess_run(mocker: MockerFixture) -> Mock:
    mock = mocker.patch("subprocess.run")
    mock.return_value.stdout = b"pandoc 3.1.0"
    mock.return_value.returncode = 0
    mock.return_value.stderr = b""

    def side_effect(*args: list[Any], **_: Any) -> Mock:
        if args[0][0] == "pandoc" and "--version" in args[0]:
            return cast(Mock, mock.return_value)

        output_file = next((arg for arg in args[0] if arg.endswith((".md", ".json"))), "")
        if output_file:
            content = json.dumps(SAMPLE_PANDOC_JSON) if output_file.endswith(".json") else "Sample processed content"
            Path(output_file).write_text(content)
        return cast(Mock, mock.return_value)

    mock.side_effect = side_effect
    return mock


@pytest.fixture
def mock_subprocess_run_invalid(mocker: MockerFixture) -> Mock:
    mock = mocker.patch("subprocess.run")
    mock.return_value.stdout = b"pandoc 2.0.0"
    mock.return_value.returncode = 0
    return mock


@pytest.fixture
def mock_subprocess_run_error(mocker: MockerFixture) -> Mock:
    mock = mocker.patch("subprocess.run")
    mock.side_effect = FileNotFoundError()
    return mock


@pytest.fixture(autouse=True)
def reset_version_ref(mocker: MockerFixture) -> None:
    mocker.patch("kreuzberg._pandoc.version_ref", {"checked": False})


async def test_validate_pandoc_version(mock_subprocess_run: Mock) -> None:
    await validate_pandoc_version()
    mock_subprocess_run.assert_called_with(["pandoc", "--version"], capture_output=True)


async def test_validate_pandoc_version_invalid(mock_subprocess_run_invalid: Mock) -> None:
    with pytest.raises(MissingDependencyError, match="Pandoc version 3 or above is required"):
        await validate_pandoc_version()


async def test_validate_pandoc_version_missing(mock_subprocess_run_error: Mock) -> None:
    with pytest.raises(MissingDependencyError, match="Pandoc is not installed"):
        await validate_pandoc_version()


async def test_get_extension_from_mime_type_valid() -> None:
    for mime_type in PANDOC_MIMETYPE_TO_FORMAT_MAPPING:
        extension = _get_extension_from_mime_type(mime_type)
        assert isinstance(extension, str)
        assert extension


async def test_get_extension_from_mime_type_invalid() -> None:
    with pytest.raises(ValidationError, match="Unsupported mime type"):
        _get_extension_from_mime_type("invalid/mime-type")


async def test_process_file_success(mock_subprocess_run: Mock, docx_document: Path) -> None:
    result = await process_file(
        docx_document, mime_type="application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    )
    assert isinstance(result, PandocResult)
    assert result.content.strip() == "Sample processed content"


async def test_process_file_with_extra_args(mock_subprocess_run: Mock, docx_document: Path) -> None:
    result = await process_file(
        docx_document,
        mime_type="application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        extra_args=["--strip-comments"],
    )
    assert isinstance(result, PandocResult)
    assert result.content.strip() == "Sample processed content"
    assert "--strip-comments" in mock_subprocess_run.call_args[0][0]


async def test_process_file_error(mock_subprocess_run: Mock, docx_document: Path) -> None:
    mock_subprocess_run.return_value.returncode = 1
    mock_subprocess_run.return_value.stderr = b"Error processing file"

    with pytest.raises(ParsingError, match="Failed to extract file data"):
        await process_file(
            docx_document, mime_type="application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        )


async def test_process_content_success(mock_subprocess_run: Mock) -> None:
    result = await process_content(
        b"test content", mime_type="application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    )
    assert isinstance(result, PandocResult)
    assert result.content.strip() == "Sample processed content"


async def test_process_content_with_extra_args(mock_subprocess_run: Mock) -> None:
    result = await process_content(
        b"test content",
        mime_type="application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        extra_args=["--strip-comments"],
    )
    assert isinstance(result, PandocResult)
    assert result.content.strip() == "Sample processed content"
    assert "--strip-comments" in mock_subprocess_run.call_args[0][0]


async def test_extract_metadata_error(mock_subprocess_run: Mock, docx_document: Path) -> None:
    mock_subprocess_run.return_value.returncode = 1
    mock_subprocess_run.return_value.stderr = b"Error extracting metadata"

    with pytest.raises(ParsingError, match="Failed to extract file data"):
        await extract_metadata(
            docx_document, mime_type="application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        )


async def test_extract_metadata_runtime_error(mock_subprocess_run: Mock, docx_document: Path) -> None:
    mock_subprocess_run.side_effect = RuntimeError("Command failed")

    with pytest.raises(ParsingError, match="Failed to extract file data"):
        await extract_metadata(
            docx_document, mime_type="application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        )


async def test_integration_validate_pandoc_version() -> None:
    await validate_pandoc_version()


async def test_integration_process_file(markdown_document: Path) -> None:
    result = await process_file(markdown_document, mime_type="text/x-markdown")
    assert isinstance(result, PandocResult)
    assert isinstance(result.content, str)
    assert result.content.strip()


async def test_integration_process_content() -> None:
    content = b"# Test\nThis is a test file."
    result = await process_content(content, mime_type="text/x-markdown")
    assert isinstance(result, PandocResult)
    assert isinstance(result.content, str)
    assert result.content.strip()


async def test_integration_extract_metadata(markdown_document: Path) -> None:
    result = await extract_metadata(markdown_document, mime_type="text/x-markdown")
    assert isinstance(result, dict)


async def test_process_content_runtime_error(mock_subprocess_run: Mock) -> None:
    def side_effect(*args: list[Any], **_: Any) -> Mock:
        if args[0][0] == "pandoc" and "--version" in args[0]:
            mock_subprocess_run.return_value.stdout = b"pandoc 3.1.0"
            return cast(Mock, mock_subprocess_run.return_value)
        raise RuntimeError("Pandoc error")

    mock_subprocess_run.side_effect = side_effect
    with pytest.raises(ParsingError, match="Failed to extract file data"):
        await process_content(
            b"content", mime_type="application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        )


async def test_process_file_runtime_error(mock_subprocess_run: Mock, docx_document: Path) -> None:
    def side_effect(*args: list[Any], **_: Any) -> Mock:
        if args[0][0] == "pandoc" and "--version" in args[0]:
            mock_subprocess_run.return_value.stdout = b"pandoc 3.1.0"
            return cast(Mock, mock_subprocess_run.return_value)
        raise RuntimeError("Pandoc error")

    mock_subprocess_run.side_effect = side_effect
    with pytest.raises(ParsingError, match="Failed to extract file data"):
        await process_file(
            docx_document, mime_type="application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        )


async def test_process_content_empty_result(mock_subprocess_run: Mock) -> None:
    def side_effect(*args: list[Any], **_: Any) -> Mock:
        if args[0][0] == "pandoc" and "--version" in args[0]:
            mock_subprocess_run.return_value.stdout = b"pandoc 3.1.0"
            return cast(Mock, mock_subprocess_run.return_value)
        output_file = next((arg for arg in args[0] if arg.endswith((".md", ".json"))), "")
        if output_file:
            if output_file.endswith(".json"):
                Path(output_file).write_text('{"pandoc-api-version":[1,22,2,1],"meta":{},"blocks":[]}')
            else:
                Path(output_file).write_text("")
        return cast(Mock, mock_subprocess_run.return_value)

    mock_subprocess_run.side_effect = side_effect
    result = await process_content(
        b"content", mime_type="application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    )
    assert isinstance(result, PandocResult)
    assert result.content == ""
    assert result.metadata == {}


async def test_process_file_invalid_mime_type(mock_subprocess_run: Mock, docx_document: Path) -> None:
    with pytest.raises(ValidationError, match="Unsupported mime type"):
        await process_file(docx_document, mime_type="invalid/mime-type")


async def test_process_content_invalid_mime_type(mock_subprocess_run: Mock) -> None:
    with pytest.raises(ValidationError, match="Unsupported mime type"):
        await process_content(b"content", mime_type="invalid/mime-type")

from __future__ import annotations

from pathlib import Path
from typing import TYPE_CHECKING

import pytest

from kreuzberg._tmp import create_temp_file

if TYPE_CHECKING:
    from pytest_mock import MockerFixture


@pytest.mark.anyio
async def test_create_temp_file() -> None:
    # Test creating an empty temp file
    temp_file_path, cleanup = await create_temp_file(".txt")
    assert isinstance(temp_file_path, Path)
    assert temp_file_path.suffix == ".txt"
    assert temp_file_path.exists()
    await cleanup()
    assert not temp_file_path.exists()


@pytest.mark.anyio
async def test_create_temp_file_with_content() -> None:
    content = b"test content"
    temp_file_path, cleanup = await create_temp_file(".txt", content)
    assert isinstance(temp_file_path, Path)
    assert temp_file_path.suffix == ".txt"
    assert temp_file_path.exists()
    # Use sync read_bytes since we're testing a real file
    assert temp_file_path.read_bytes() == content
    await cleanup()
    assert not temp_file_path.exists()


@pytest.mark.anyio
async def test_create_temp_file_cleanup_error(mocker: MockerFixture) -> None:
    # Test that errors during file cleanup are properly suppressed
    temp_file_path, cleanup = await create_temp_file(".txt")
    assert temp_file_path.exists()

    # Mock the unlink operation to raise an error
    mock_path = mocker.Mock()
    mock_path.unlink = mocker.Mock(side_effect=PermissionError("Mock permission error"))

    mocker.patch("kreuzberg._tmp.AsyncPath", return_value=mock_path)
    # This should not raise an error even though the unlink fails
    temp_file_path2, cleanup2 = await create_temp_file(".txt")
    await cleanup2()  # This should not raise an error

    # Clean up our actual temp file
    await cleanup()
    # Note: We don't check if the file exists here since the cleanup is async
    # and the file might still exist briefly after the cleanup call

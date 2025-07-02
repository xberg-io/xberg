"""Tests for PDF lock utilities."""

from __future__ import annotations

import hashlib
import threading
import time
from pathlib import Path

import pytest

from kreuzberg._utils._pdf_lock import (
    _get_file_key,
    _get_file_lock,
    pypdfium_file_lock,
    pypdfium_lock,
    with_pypdfium_lock,
)


def test_get_file_key_string_path() -> None:
    """Test _get_file_key with string path."""
    path = "/tmp/test.pdf"
    key = _get_file_key(path)

    resolved_path = str(Path(path).resolve())
    expected_hash = hashlib.md5(resolved_path.encode()).hexdigest()
    assert key == expected_hash


def test_get_file_key_path_object() -> None:
    """Test _get_file_key with Path object."""
    path = Path("/tmp/test.pdf")
    key = _get_file_key(path)

    resolved_path = str(path.resolve())
    expected_hash = hashlib.md5(resolved_path.encode()).hexdigest()
    assert key == expected_hash


def test_get_file_key_consistent() -> None:
    """Test that _get_file_key returns consistent results."""
    path1 = "/tmp/test.pdf"
    path2 = Path("/tmp/test.pdf")

    key1 = _get_file_key(path1)
    key2 = _get_file_key(path2)

    assert key1 == key2


def test_get_file_lock_creates_new_lock() -> None:
    """Test _get_file_lock creates new lock for new file."""
    path = "/tmp/unique_test_file.pdf"
    lock = _get_file_lock(path)

    assert type(lock).__name__ == "RLock"


def test_get_file_lock_reuses_existing_lock() -> None:
    """Test _get_file_lock reuses existing lock for same file."""
    path = "/tmp/test_reuse.pdf"

    lock1 = _get_file_lock(path)
    lock2 = _get_file_lock(path)

    assert lock1 is lock2


def test_get_file_lock_different_files() -> None:
    """Test _get_file_lock creates different locks for different files."""
    path1 = "/tmp/file1.pdf"
    path2 = "/tmp/file2.pdf"

    lock1 = _get_file_lock(path1)
    lock2 = _get_file_lock(path2)

    assert lock1 is not lock2


def test_pypdfium_lock_context_manager() -> None:
    """Test pypdfium_lock context manager."""

    execution_order = []

    def thread_func(thread_id: int) -> None:
        with pypdfium_lock():
            execution_order.append(f"start_{thread_id}")
            time.sleep(0.01)  # Small delay to ensure sequential execution  # ~keep
            execution_order.append(f"end_{thread_id}")

    threads = []
    for i in range(3):
        thread = threading.Thread(target=thread_func, args=(i,))
        threads.append(thread)
        thread.start()

    for thread in threads:
        thread.join()

    # Due to the lock, execution should be sequential  # ~keep
    # Each thread should complete before the next starts  # ~keep
    assert len(execution_order) == 6

    for i in range(0, len(execution_order), 2):
        start = execution_order[i]
        end = execution_order[i + 1]
        thread_id = start.split("_")[1]
        assert start == f"start_{thread_id}"
        assert end == f"end_{thread_id}"


def test_pypdfium_file_lock_context_manager(tmp_path: Path) -> None:
    """Test pypdfium_file_lock context manager."""
    test_file = tmp_path / "test.pdf"
    test_file.write_bytes(b"fake pdf content")

    execution_order = []

    def thread_func(thread_id: int) -> None:
        with pypdfium_file_lock(test_file):
            execution_order.append(f"start_{thread_id}")
            time.sleep(0.01)
            execution_order.append(f"end_{thread_id}")

    threads = []
    for i in range(2):
        thread = threading.Thread(target=thread_func, args=(i,))
        threads.append(thread)
        thread.start()

    for thread in threads:
        thread.join()

    # Should be sequential due to file lock  # ~keep
    assert len(execution_order) == 4


def test_pypdfium_file_lock_different_files(tmp_path: Path) -> None:
    """Test pypdfium_file_lock allows concurrent access to different files."""
    file1 = tmp_path / "file1.pdf"
    file2 = tmp_path / "file2.pdf"
    file1.write_bytes(b"content1")
    file2.write_bytes(b"content2")

    execution_times = {}

    def thread_func(file_path: Path, thread_id: str) -> None:
        start_time = time.time()
        with pypdfium_file_lock(file_path):
            time.sleep(0.05)
        end_time = time.time()
        execution_times[thread_id] = (start_time, end_time)

    thread1 = threading.Thread(target=thread_func, args=(file1, "thread1"))
    thread2 = threading.Thread(target=thread_func, args=(file2, "thread2"))

    thread1.start()
    thread2.start()

    thread1.join()
    thread2.join()

    assert "thread1" in execution_times
    assert "thread2" in execution_times


def test_with_pypdfium_lock_decorator() -> None:
    """Test with_pypdfium_lock decorator."""
    execution_order = []

    @with_pypdfium_lock
    def test_function(thread_id: int) -> str:
        execution_order.append(f"start_{thread_id}")
        time.sleep(0.01)
        execution_order.append(f"end_{thread_id}")
        return f"result_{thread_id}"

    def thread_func(thread_id: int, results: list[str]) -> None:
        result = test_function(thread_id)
        results.append(result)

    results: list[str] = []
    threads = []
    for i in range(2):
        thread = threading.Thread(target=thread_func, args=(i, results))
        threads.append(thread)
        thread.start()

    for thread in threads:
        thread.join()

    assert len(results) == 2
    assert "result_0" in results
    assert "result_1" in results

    assert len(execution_order) == 4


def test_with_pypdfium_lock_preserves_return_value() -> None:
    """Test that decorator preserves function return value."""

    @with_pypdfium_lock
    def test_function(x: int, y: int) -> int:
        return x + y

    result = test_function(5, 3)
    assert result == 8


def test_with_pypdfium_lock_preserves_exceptions() -> None:
    """Test that decorator preserves exceptions."""

    @with_pypdfium_lock
    def test_function() -> None:
        raise ValueError("Test error")

    with pytest.raises(ValueError, match="Test error"):
        test_function()


def test_file_lock_cache_cleanup() -> None:
    """Test that file lock cache can be cleaned up by GC."""
    from kreuzberg._utils._pdf_lock import _FILE_LOCKS_CACHE

    initial_size = len(_FILE_LOCKS_CACHE)

    path = "/tmp/temp_file_for_gc_test.pdf"
    lock = _get_file_lock(path)

    assert len(_FILE_LOCKS_CACHE) == initial_size + 1

    del lock

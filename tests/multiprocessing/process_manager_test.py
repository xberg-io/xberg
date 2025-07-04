"""Tests for process pool manager."""

from __future__ import annotations

import multiprocessing as mp
from typing import Any
from unittest.mock import patch

import pytest

from kreuzberg._multiprocessing.process_manager import ProcessPoolManager


def simple_function(x: int) -> int:
    """Simple function for testing."""
    return x * 2


def add_function(x: int, y: int) -> int:
    """Function that adds two numbers."""
    return x + y


def error_function() -> None:
    """Function that raises an error."""
    raise ValueError("Test error")


class TestProcessPoolManager:
    """Tests for ProcessPoolManager class."""

    def test_init_default(self) -> None:
        """Test initialization with default parameters."""
        manager = ProcessPoolManager()
        expected_processes = mp.cpu_count()
        assert manager.max_processes == expected_processes
        assert manager._executor is None
        assert manager._active_tasks == 0

    def test_init_custom_processes(self) -> None:
        """Test initialization with custom max_processes."""
        manager = ProcessPoolManager(max_processes=4)
        assert manager.max_processes == 4

    def test_init_custom_memory_limit(self) -> None:
        """Test initialization with custom memory limit."""
        manager = ProcessPoolManager(memory_limit_gb=2.0)
        expected_bytes = int(2.0 * 1024**3)
        assert manager.memory_limit_bytes == expected_bytes

    def test_init_default_memory_limit(self) -> None:
        """Test initialization with default memory limit (75% of available)."""
        with patch("psutil.virtual_memory") as mock_memory:
            mock_memory.return_value.available = 8 * 1024**3
            manager = ProcessPoolManager()
            expected_bytes = int(8 * 1024**3 * 0.75)
            assert manager.memory_limit_bytes == expected_bytes

    def test_get_optimal_workers_memory_constrained(self) -> None:
        """Test optimal workers calculation when memory is the constraint."""
        manager = ProcessPoolManager(max_processes=8, memory_limit_gb=1.0)

        workers = manager.get_optimal_workers(task_memory_mb=500)
        assert workers == 2

    def test_get_optimal_workers_cpu_constrained(self) -> None:
        """Test optimal workers calculation when CPU is the constraint."""
        manager = ProcessPoolManager(max_processes=2, memory_limit_gb=10.0)

        workers = manager.get_optimal_workers(task_memory_mb=100)
        assert workers == 2

    def test_get_optimal_workers_minimum_one(self) -> None:
        """Test optimal workers returns at least 1."""
        manager = ProcessPoolManager(max_processes=1, memory_limit_gb=0.001)
        workers = manager.get_optimal_workers(task_memory_mb=1000)
        assert workers == 1

    def test_ensure_executor_creation(self) -> None:
        """Test that _ensure_executor creates ProcessPoolExecutor."""
        manager = ProcessPoolManager(max_processes=2)
        executor = manager._ensure_executor()

        assert manager._executor is not None
        assert executor is manager._executor

    def test_ensure_executor_reuse(self) -> None:
        """Test that _ensure_executor reuses existing executor with same workers."""
        manager = ProcessPoolManager(max_processes=2)
        executor1 = manager._ensure_executor(max_workers=2)
        executor2 = manager._ensure_executor(max_workers=2)

        assert executor1 is executor2

    def test_ensure_executor_recreation(self) -> None:
        """Test that _ensure_executor recreates executor with different workers."""
        manager = ProcessPoolManager(max_processes=4)

        class _MockExecutor:
            def __init__(self) -> None:
                self._max_workers = 2
                self.shutdown_called = False

            def shutdown(self, wait: bool = True) -> None:
                self.shutdown_called = True

        mock_executor = _MockExecutor()
        manager._executor = mock_executor  # type: ignore[assignment]

        new_executor = manager._ensure_executor(max_workers=4)

        assert mock_executor.shutdown_called
        assert new_executor is not mock_executor  # type: ignore[comparison-overlap]

    @pytest.mark.anyio
    async def test_submit_task_success(self) -> None:
        """Test successful task submission."""
        manager = ProcessPoolManager(max_processes=2)

        result = await manager.submit_task(simple_function, 5)

        assert result == 10
        assert manager._active_tasks == 0

    @pytest.mark.anyio
    async def test_submit_task_with_memory_constraint(self) -> None:
        """Test task submission with memory constraint."""
        manager = ProcessPoolManager(max_processes=8, memory_limit_gb=1.0)

        result = await manager.submit_task(simple_function, 3, task_memory_mb=500)

        assert result == 6
        assert manager._active_tasks == 0

    @pytest.mark.anyio
    async def test_submit_task_tracks_active_tasks(self) -> None:
        """Test that active tasks are tracked during execution."""
        manager = ProcessPoolManager(max_processes=2)

        result = await manager.submit_task(simple_function, 21)

        assert result == 42
        assert manager._active_tasks == 0

    @pytest.mark.anyio
    async def test_submit_batch_success(self) -> None:
        """Test successful batch submission."""
        manager = ProcessPoolManager(max_processes=2)

        arg_batches = [(1,), (2,), (3,), (4,)]
        results = await manager.submit_batch(simple_function, arg_batches)

        assert results == [2, 4, 6, 8]
        assert manager._active_tasks == 0

    @pytest.mark.anyio
    async def test_submit_batch_empty(self) -> None:
        """Test batch submission with empty list."""
        manager = ProcessPoolManager(max_processes=2)

        results = await manager.submit_batch(simple_function, [])

        assert results == []

    @pytest.mark.anyio
    async def test_submit_batch_with_concurrency_limit(self) -> None:
        """Test batch submission with concurrency limit."""
        manager = ProcessPoolManager(max_processes=4)

        arg_batches = [(1,), (2,), (3,), (4,), (5,)]
        results = await manager.submit_batch(simple_function, arg_batches, max_concurrent=2)

        assert results == [2, 4, 6, 8, 10]
        assert manager._active_tasks == 0

    @pytest.mark.anyio
    async def test_submit_batch_with_memory_constraint(self) -> None:
        """Test batch submission with memory constraint."""
        manager = ProcessPoolManager(max_processes=8, memory_limit_gb=1.0)

        arg_batches = [(1,), (2,)]
        results = await manager.submit_batch(
            simple_function,
            arg_batches,
            task_memory_mb=500,
        )

        assert results == [2, 4]

    def test_get_system_info(self) -> None:
        """Test system info retrieval."""
        manager = ProcessPoolManager(max_processes=4, memory_limit_gb=2.0)

        with patch("psutil.virtual_memory") as mock_memory, patch("psutil.cpu_percent") as mock_cpu:
            mock_memory.return_value.total = 16 * 1024**3
            mock_memory.return_value.available = 8 * 1024**3
            mock_memory.return_value.percent = 50.0
            mock_cpu.return_value = 25.5

            info = manager.get_system_info()

            assert info["cpu_count"] == mp.cpu_count()
            assert info["cpu_percent"] == 25.5
            assert info["memory_total"] == 16 * 1024**3
            assert info["memory_available"] == 8 * 1024**3
            assert info["memory_percent"] == 50.0
            assert info["active_tasks"] == 0
            assert info["max_processes"] == 4
            assert info["memory_limit"] == int(2.0 * 1024**3)

    def test_shutdown_with_executor(self) -> None:
        """Test shutdown when executor exists."""
        manager = ProcessPoolManager(max_processes=2)
        manager._ensure_executor()

        class _MockExecutor:
            def __init__(self) -> None:
                self.shutdown_called_with: bool | None = None

            def shutdown(self, wait: bool = True) -> None:
                self.shutdown_called_with = wait

        mock_executor = _MockExecutor()
        manager._executor = mock_executor  # type: ignore[assignment]

        manager.shutdown(wait=True)

        assert mock_executor.shutdown_called_with is True
        assert manager._executor is None

    def test_shutdown_without_executor(self) -> None:
        """Test shutdown when no executor exists."""
        manager = ProcessPoolManager(max_processes=2)

        manager.shutdown()
        assert manager._executor is None

    def test_context_manager_sync(self) -> None:
        """Test synchronous context manager."""
        manager = ProcessPoolManager(max_processes=2)

        shutdown_called = False
        original_shutdown = manager.shutdown

        def mock_shutdown(*args: Any, **kwargs: Any) -> None:
            nonlocal shutdown_called
            shutdown_called = True
            return original_shutdown(*args, **kwargs)

        manager.shutdown = mock_shutdown  # type: ignore[method-assign]

        with manager:
            pass

        assert shutdown_called

    @pytest.mark.anyio
    async def test_context_manager_async(self) -> None:
        """Test asynchronous context manager."""
        manager = ProcessPoolManager(max_processes=2)

        shutdown_called = False
        original_shutdown = manager.shutdown

        def mock_shutdown(*args: Any, **kwargs: Any) -> None:
            nonlocal shutdown_called
            shutdown_called = True
            return original_shutdown(*args, **kwargs)

        manager.shutdown = mock_shutdown  # type: ignore[method-assign]

        async with manager:
            pass

        assert shutdown_called

    @pytest.mark.anyio
    async def test_context_manager_with_task(self) -> None:
        """Test context manager with actual task execution."""
        async with ProcessPoolManager(max_processes=2) as manager:
            result = await manager.submit_task(simple_function, 7)
            assert result == 14

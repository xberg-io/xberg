from __future__ import annotations

from pathlib import Path
from unittest.mock import Mock, patch

from kreuzberg._utils._errors import (
    BatchExtractionResult,
    create_error_context,
    is_resource_error,
    is_transient_error,
    should_retry,
)
from kreuzberg.exceptions import ValidationError


def test_create_error_context_basic() -> None:
    context = create_error_context(operation="test_operation")

    assert context["operation"] == "test_operation"
    assert "timestamp" in context
    assert context["timestamp"].endswith("+00:00") or context["timestamp"].endswith("Z")


def test_create_error_context_with_file_path() -> None:
    test_file = Path(__file__)

    context = create_error_context(operation="test_op", file_path=test_file)

    assert context["file"]["path"] == str(test_file)
    assert context["file"]["name"] == test_file.name
    assert context["file"]["exists"] is True
    assert context["file"]["size"] > 0


def test_create_error_context_with_string_path() -> None:
    test_path = str(Path(__file__))

    context = create_error_context(operation="test_op", file_path=test_path)

    assert context["file"]["path"] == test_path
    assert context["file"]["exists"] is True


def test_create_error_context_with_nonexistent_file() -> None:
    fake_path = Path("/nonexistent/file.txt")

    context = create_error_context(operation="test_op", file_path=fake_path)

    assert context["file"]["path"] == str(fake_path)
    assert context["file"]["exists"] is False
    assert context["file"]["size"] is None


def test_create_error_context_with_error() -> None:
    test_error = ValueError("Test error message")

    context = create_error_context(operation="test_op", error=test_error)

    assert context["error"]["type"] == "ValueError"
    assert context["error"]["message"] == "Test error message"
    assert isinstance(context["error"]["traceback"], list)


def test_create_error_context_with_system_error() -> None:
    memory_error = OSError("Out of memory error")

    with (
        patch("psutil.virtual_memory") as mock_mem,
        patch("psutil.cpu_percent") as mock_cpu,
        patch("psutil.pids") as mock_pids,
        patch("platform.platform") as mock_platform,
    ):
        mock_mem.return_value = Mock(available=1024 * 1024 * 1024, percent=75.0)
        mock_cpu.return_value = 50.0
        mock_pids.return_value = [1, 2, 3, 4, 5]
        mock_platform.return_value = "Test Platform"

        context = create_error_context(operation="test_op", error=memory_error)

        assert "system" in context
        assert context["system"]["memory_available_mb"] == 1024.0
        assert context["system"]["memory_percent"] == 75.0
        assert context["system"]["cpu_percent"] == 50.0
        assert context["system"]["process_count"] == 5
        assert context["system"]["platform"] == "Test Platform"


def test_create_error_context_system_info_exception() -> None:
    memory_error = OSError("memory allocation failed")

    with patch("psutil.virtual_memory", side_effect=Exception("psutil failed")):
        context = create_error_context(operation="test_op", error=memory_error)

        assert "system" not in context
        assert context["error"]["type"] == "OSError"


def test_create_error_context_with_extra_args() -> None:
    context = create_error_context(
        operation="test_op", custom_field="custom_value", another_field=42, nested_data={"key": "value"}
    )

    assert context["custom_field"] == "custom_value"
    assert context["another_field"] == 42
    assert context["nested_data"] == {"key": "value"}


def test_is_transient_error_with_transient_types() -> None:
    assert is_transient_error(OSError("test")) is True
    assert is_transient_error(PermissionError("access denied")) is True
    assert is_transient_error(TimeoutError("timeout")) is True
    assert is_transient_error(ConnectionError("connection failed")) is True
    assert is_transient_error(BrokenPipeError("broken pipe")) is True


def test_is_transient_error_with_patterns() -> None:
    assert is_transient_error(Exception("temporary failure")) is True
    assert is_transient_error(Exception("file is locked")) is True
    assert is_transient_error(Exception("resource in use")) is True
    assert is_transient_error(Exception("access denied")) is True
    assert is_transient_error(Exception("permission denied")) is True
    assert is_transient_error(Exception("connection timeout")) is True
    assert is_transient_error(Exception("network error")) is True
    assert is_transient_error(Exception("too many open files")) is True
    assert is_transient_error(Exception("cannot allocate memory")) is True
    assert is_transient_error(Exception("resource temporarily unavailable")) is True
    assert is_transient_error(Exception("broken pipe")) is True
    assert is_transient_error(Exception("subprocess failed")) is True
    assert is_transient_error(Exception("signal interrupted")) is True


def test_is_transient_error_non_transient() -> None:
    assert is_transient_error(ValueError("invalid value")) is False
    assert is_transient_error(TypeError("wrong type")) is False
    assert is_transient_error(Exception("some other error")) is False


def test_is_resource_error() -> None:
    assert is_resource_error(Exception("memory error")) is True
    assert is_resource_error(Exception("out of memory")) is True
    assert is_resource_error(Exception("cannot allocate")) is True
    assert is_resource_error(Exception("too many open files")) is True
    assert is_resource_error(Exception("file descriptor limit")) is True
    assert is_resource_error(Exception("resource exhausted")) is True
    assert is_resource_error(Exception("cpu limit exceeded")) is True
    assert is_resource_error(Exception("thread limit reached")) is True
    assert is_resource_error(Exception("process limit")) is True

    assert is_resource_error(Exception("invalid input")) is False
    assert is_resource_error(ValueError("bad value")) is False


def test_should_retry_max_attempts_reached() -> None:
    error = OSError("temporary failure")

    assert should_retry(error, attempt=1, max_attempts=3) is True
    assert should_retry(error, attempt=2, max_attempts=3) is True
    assert should_retry(error, attempt=3, max_attempts=3) is False
    assert should_retry(error, attempt=4, max_attempts=3) is False


def test_should_retry_validation_error() -> None:
    validation_error = ValidationError("invalid input")

    assert should_retry(validation_error, attempt=1, max_attempts=3) is False
    assert should_retry(validation_error, attempt=1, max_attempts=10) is False


def test_should_retry_transient_vs_non_transient() -> None:
    transient_error = OSError("temporary failure")
    non_transient_error = ValueError("invalid value")

    assert should_retry(transient_error, attempt=1, max_attempts=3) is True
    assert should_retry(non_transient_error, attempt=1, max_attempts=3) is False


class TestBatchExtractionResult:
    def test_init(self) -> None:
        result = BatchExtractionResult()

        assert result.successful == []
        assert result.failed == []
        assert result.total_count == 0

    def test_add_success(self) -> None:
        result = BatchExtractionResult()

        result.add_success(0, "result1")
        result.add_success(2, "result3")

        assert len(result.successful) == 2
        assert result.successful[0] == (0, "result1")
        assert result.successful[1] == (2, "result3")

    def test_add_failure(self) -> None:
        result = BatchExtractionResult()
        error = ValueError("test error")
        context = {"file": "test.pdf"}

        result.add_failure(1, error, context)

        assert len(result.failed) == 1
        index, error_info = result.failed[0]
        assert index == 1
        assert error_info["error"]["type"] == "ValueError"
        assert error_info["error"]["message"] == "test error"
        assert error_info["context"] == context

    def test_properties(self) -> None:
        result = BatchExtractionResult()
        result.total_count = 5

        result.add_success(0, "result1")
        result.add_success(2, "result3")
        result.add_failure(1, ValueError("error1"), {})
        result.add_failure(4, OSError("error2"), {})

        assert result.success_count == 2
        assert result.failure_count == 2
        assert result.success_rate == 40.0

    def test_success_rate_zero_total(self) -> None:
        result = BatchExtractionResult()
        assert result.success_rate == 0.0

    def test_get_ordered_results(self) -> None:
        result = BatchExtractionResult()
        result.total_count = 4

        result.add_success(0, "result1")
        result.add_success(2, "result3")

        ordered = result.get_ordered_results()

        assert len(ordered) == 4
        assert ordered[0] == "result1"
        assert ordered[1] is None
        assert ordered[2] == "result3"
        assert ordered[3] is None

    def test_get_summary(self) -> None:
        result = BatchExtractionResult()
        result.total_count = 3

        result.add_success(0, "success1")
        result.add_failure(1, ValueError("error1"), {"file": "test1.pdf"})
        result.add_failure(2, OSError("error2"), {"file": "test2.pdf"})

        summary = result.get_summary()

        assert summary["total"] == 3
        assert summary["successful"] == 1
        assert summary["failed"] == 2
        assert summary["success_rate"] == "33.3%"
        assert len(summary["failures"]) == 2

        failure1 = summary["failures"][0]
        assert failure1["index"] == 1
        assert failure1["error"] == "ValueError"
        assert failure1["message"] == "error1"

        failure2 = summary["failures"][1]
        assert failure2["index"] == 2
        assert failure2["error"] == "OSError"
        assert failure2["message"] == "error2"

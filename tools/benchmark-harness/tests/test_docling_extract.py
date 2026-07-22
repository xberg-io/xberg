"""Conformance tests for the isolated Docling benchmark wrapper."""

from __future__ import annotations

import importlib.util
import sys
import types
import unittest
from collections.abc import Iterator
from pathlib import Path
from unittest import mock


def _load_wrapper() -> types.ModuleType:
    docling = types.ModuleType("docling")
    converter_module = types.ModuleType("docling.document_converter")
    converter_module.DocumentConverter = object
    script = Path(__file__).parents[1] / "scripts" / "docling_extract.py"
    spec = importlib.util.spec_from_file_location("docling_extract_under_test", script)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    with mock.patch.dict(
        sys.modules,
        {"docling": docling, "docling.document_converter": converter_module},
    ):
        spec.loader.exec_module(module)
    return module


class _Document:
    def __init__(self, path: str, events: list[str]) -> None:
        self.path = path
        self.events = events

    def export_to_markdown(self) -> str:
        self.events.append(f"render:{self.path}")
        return f"markdown:{self.path}"


class _Result:
    def __init__(self, path: str, events: list[str], *, success: bool) -> None:
        self.status = types.SimpleNamespace(name="SUCCESS" if success else "FAILURE")
        self.document = _Document(path, events)
        self.errors = [] if success else [f"failed:{path}"]


class _Converter:
    def __init__(self, events: list[str]) -> None:
        self.events = events
        self.calls = 0

    def convert_all(self, paths: list[str], *, raises_on_error: bool) -> Iterator[_Result]:
        self.calls += 1
        self.events.append("convert_all")
        assert raises_on_error is False

        def results() -> Iterator[_Result]:
            for index, path in enumerate(paths):
                self.events.append(f"yield:{path}")
                yield _Result(path, self.events, success=index == 0)

        return results()


class DoclingBatchConformanceTest(unittest.TestCase):
    """Validate the wrapper contract without importing the real Docling package."""

    def test_converter_configuration_fails_closed(self) -> None:
        """Reject a run when the requested OCR mode cannot be configured."""
        wrapper = _load_wrapper()
        error: RuntimeError | None = None

        try:
            wrapper.create_converter(ocr_enabled=True)
        except RuntimeError as caught:
            error = caught
        else:
            self.fail("converter configuration unexpectedly succeeded")

        assert error is not None
        assert str(error) == "failed to configure Docling with OCR explicitly enabled"
        assert isinstance(error.__cause__, ImportError)

    def test_convert_all_is_single_lazy_ordered_timed_batch(self) -> None:
        """The lazy iterator must be consumed once, in order, inside the timer."""
        wrapper = _load_wrapper()
        events: list[str] = []
        converter = _Converter(events)

        def perf_counter() -> float:
            events.append("clock")
            return 10.0 if events.count("clock") == 1 else 12.0

        with (
            mock.patch.object(wrapper.time, "perf_counter", side_effect=perf_counter),
            mock.patch.object(wrapper, "_get_peak_memory_bytes", return_value=123),
        ):
            payload = wrapper.extract_batch(["first.pdf", "second.pdf"], converter)

        assert converter.calls == 1
        assert events[0:2] == ["clock", "convert_all"]
        assert events[-1] == "clock"
        assert events.index("yield:first.pdf") < events.index("render:first.pdf")
        assert events.index("render:first.pdf") < events.index("yield:second.pdf")
        assert len(payload["results"]) == 2
        assert payload["results"][0]["content"] == "markdown:first.pdf"
        assert payload["results"][1]["error"] == "['failed:second.pdf']"
        assert payload["total_ms"] == 2000.0
        assert payload["per_file_ms"] == [None, None]
        assert payload["metadata"]["benchmark_timing_scope"] == "cold_end_to_end_subprocess"


if __name__ == "__main__":
    unittest.main()

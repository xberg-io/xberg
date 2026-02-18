"""Tests for kreuzberg/ocr/rapidocr.py."""

from __future__ import annotations

import sys
import types
from typing import Any
from unittest.mock import patch

import pytest

from kreuzberg.exceptions import OCRError, ValidationError


def _make_mock_rapidocr_module() -> Any:
    module: Any = types.ModuleType("rapidocr")

    class LangCls:
        CH = "ch"

    def lang_det(value: str) -> str:
        return value

    def lang_rec(value: str) -> str:
        return value

    class Result:
        txts = ("hello", "world")
        scores = (0.9, 0.8)
        img = type("Img", (), {"shape": (120, 320, 3)})()

    class RapidOCR:
        def __init__(self, config_path: str | None = None, params: dict[str, Any] | None = None) -> None:
            self.config_path = config_path
            self.params = params or {}

        def __call__(self, _image_bytes: bytes) -> Result:
            return Result()

    module.RapidOCR = RapidOCR
    module.LangDet = lang_det
    module.LangRec = lang_rec
    module.LangCls = LangCls
    return module


def test_rapidocr_import_error() -> None:
    """RapidOCRBackend should raise ImportError when rapidocr is unavailable."""
    from kreuzberg.ocr.rapidocr import RapidOCRBackend

    with patch.dict(sys.modules, {"rapidocr": None}):
        with pytest.raises(ImportError) as exc_info:
            RapidOCRBackend()

    assert "kreuzberg[rapidocr]" in str(exc_info.value)


def test_rapidocr_backend_process_image_success() -> None:
    """RapidOCRBackend should convert RapidOCR output into Kreuzberg result shape."""
    from kreuzberg.ocr.rapidocr import RapidOCRBackend

    mock_rapidocr = _make_mock_rapidocr_module()
    with patch.dict(sys.modules, {"rapidocr": mock_rapidocr}):
        backend = RapidOCRBackend(language="eng")
        result = backend.process_image(b"fake-image", "eng")

    assert result["content"] == "hello\nworld"
    assert result["metadata"]["backend"] == "rapid-ocr"
    assert result["metadata"]["text_regions"] == 2
    assert result["metadata"]["width"] == 320
    assert result["metadata"]["height"] == 120


def test_rapidocr_backend_unsupported_language() -> None:
    """RapidOCRBackend should reject unsupported language codes."""
    from kreuzberg.ocr.rapidocr import RapidOCRBackend

    mock_rapidocr = _make_mock_rapidocr_module()
    with patch.dict(sys.modules, {"rapidocr": mock_rapidocr}):
        backend = RapidOCRBackend(language="eng")
        with pytest.raises(ValidationError):
            backend.process_image(b"fake-image", "unsupported")


def test_rapidocr_backend_init_failure_raises_ocr_error() -> None:
    """RapidOCRBackend should wrap engine init errors with OCRError."""
    from kreuzberg.ocr.rapidocr import RapidOCRBackend

    mock_rapidocr = _make_mock_rapidocr_module()

    class FailingRapidOCR:
        def __init__(self, *_args: Any, **_kwargs: Any) -> None:
            msg = "boom"
            raise RuntimeError(msg)

    mock_rapidocr.RapidOCR = FailingRapidOCR

    with patch.dict(sys.modules, {"rapidocr": mock_rapidocr}):
        backend = RapidOCRBackend(language="eng")
        with pytest.raises(OCRError):
            backend.initialize()

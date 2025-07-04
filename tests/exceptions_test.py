from __future__ import annotations

from kreuzberg.exceptions import KreuzbergError, MissingDependencyError, OCRError, ParsingError, ValidationError


def test_kreuzberg_error_serialize_context_with_bytes() -> None:
    error = KreuzbergError("Test error", context={"data": b"test bytes"})
    serialized = error._serialize_context(error.context)
    assert serialized == {"data": "test bytes"}


def test_kreuzberg_error_serialize_context_with_list() -> None:
    """Test serialization of list context."""
    error = KreuzbergError("Test error")
    serialized = error._serialize_context([b"bytes", "string", 123])
    assert serialized == ["bytes", "string", 123]


def test_kreuzberg_error_serialize_context_with_tuple() -> None:
    """Test serialization of tuple context."""
    error = KreuzbergError("Test error")
    serialized = error._serialize_context((b"bytes", "string", 123))
    assert serialized == ["bytes", "string", 123]


def test_kreuzberg_error_serialize_context_with_exception() -> None:
    """Test serialization of exception context."""
    error = KreuzbergError("Test error")
    inner_exception = ValueError("Inner error")
    serialized = error._serialize_context(inner_exception)
    assert serialized == {
        "type": "ValueError",
        "message": "Inner error",
    }


def test_kreuzberg_error_serialize_context_nested() -> None:
    """Test serialization of nested context structures."""
    error = KreuzbergError("Test error")
    context = {
        "list": [b"bytes", Exception("test")],
        "nested_dict": {"exception": RuntimeError("runtime error")},
        "tuple": (b"tuple bytes", 123),
    }
    serialized = error._serialize_context(context)
    expected = {
        "list": ["bytes", {"type": "Exception", "message": "test"}],
        "nested_dict": {"exception": {"type": "RuntimeError", "message": "runtime error"}},
        "tuple": ["tuple bytes", 123],
    }
    assert serialized == expected


def test_kreuzberg_error_str_with_context() -> None:
    """Test string representation with context."""
    context = {"error_code": 500, "details": "Server error"}
    error = KreuzbergError("Something went wrong", context=context)
    error_str = str(error)
    assert "KreuzbergError: Something went wrong" in error_str
    assert "Context:" in error_str
    assert '"error_code": 500' in error_str
    assert '"details": "Server error"' in error_str


def test_kreuzberg_error_str_without_context() -> None:
    """Test string representation without context."""
    error = KreuzbergError("Something went wrong")
    error_str = str(error)
    assert error_str == "KreuzbergError: Something went wrong"
    assert "Context:" not in error_str


def test_parsing_error() -> None:
    """Test ParsingError creation."""
    error = ParsingError("Parse failed", context={"line": 10})
    assert str(error).startswith("ParsingError: Parse failed")
    assert error.context == {"line": 10}


def test_validation_error() -> None:
    """Test ValidationError creation."""
    error = ValidationError("Validation failed", context={"field": "email"})
    assert str(error).startswith("ValidationError: Validation failed")
    assert error.context == {"field": "email"}


def test_ocr_error() -> None:
    """Test OCRError creation."""
    error = OCRError("OCR failed", context={"confidence": 0.1})
    assert str(error).startswith("OCRError: OCR failed")
    assert error.context == {"confidence": 0.1}


def test_missing_dependency_error_create_for_package() -> None:
    """Test MissingDependencyError creation."""
    error = MissingDependencyError.create_for_package(
        dependency_group="ocr", functionality="optical character recognition", package_name="tesseract"
    )
    assert "tesseract" in str(error)
    assert "optical character recognition" in str(error)
    assert "kreuzberg['ocr']" in str(error)

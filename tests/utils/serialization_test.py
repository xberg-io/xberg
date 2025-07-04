"""Tests for serialization utilities."""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from typing import Any

import pytest

from kreuzberg._utils._serialization import (
    deserialize,
    encode_hook,
    serialize,
)


class Color(Enum):
    """Test enum."""

    RED = "red"
    GREEN = "green"
    BLUE = "blue"


@dataclass
class SampleDataclass:
    """Test dataclass."""

    name: str
    value: int
    color: Color


class SampleError(Exception):
    """Test exception class."""


def test_encode_hook_callable() -> None:
    """Test encoding callable objects returns None."""

    def test_func() -> None:
        pass

    assert encode_hook(test_func) is None
    assert encode_hook(lambda x: x) is None
    assert encode_hook(print) is None


def test_encode_hook_exception() -> None:
    """Test encoding exceptions."""
    exc = ValueError("Test error message")
    result = encode_hook(exc)

    assert result == {"message": "Test error message", "type": "ValueError"}

    custom_exc = SampleError("Custom error")
    result = encode_hook(custom_exc)

    assert result == {"message": "Custom error", "type": "SampleError"}


def test_encode_hook_dataclass() -> None:
    """Test encoding dataclasses."""
    obj = SampleDataclass(name="test", value=42, color=Color.RED)
    result = encode_hook(obj)

    assert result == {
        "name": "test",
        "value": 42,
        "color": "red",
    }


def test_encode_hook_dataclass_type() -> None:
    """Test that dataclass types return None."""
    result = encode_hook(SampleDataclass)
    assert result is None


def test_encode_hook_dict_methods() -> None:
    """Test encoding objects with dict methods."""

    class MockClass:
        def to_dict(self) -> dict[str, str]:
            return {"key": "value"}

    obj = MockClass()
    assert encode_hook(obj) == {"key": "value"}

    class MockClass2:
        def as_dict(self) -> dict[str, str]:
            return {"key2": "value2"}

    obj2: MockClass = MockClass2()  # type: ignore[assignment]
    assert encode_hook(obj2) == {"key2": "value2"}

    class MockClass3:
        def dict(self) -> dict[str, str]:
            return {"key3": "value3"}

    obj3: MockClass = MockClass3()  # type: ignore[assignment]
    assert encode_hook(obj3) == {"key3": "value3"}

    class MockClass4:
        def model_dump(self) -> dict[str, str]:
            return {"key4": "value4"}

    obj4: MockClass = MockClass4()  # type: ignore[assignment]
    assert encode_hook(obj4) == {"key4": "value4"}


def test_encode_hook_list_methods() -> None:
    """Test encoding objects with list methods."""

    class MockClass1:
        def to_list(self) -> list[int]:
            return [1, 2, 3]

    obj = MockClass1()
    assert encode_hook(obj) == [1, 2, 3]

    class MockClass2:
        def tolist(self) -> list[int]:
            return [4, 5, 6]

    obj2: MockClass1 = MockClass2()  # type: ignore[assignment]
    assert encode_hook(obj2) == [4, 5, 6]


def test_encode_hook_pil_image() -> None:
    """Test encoding PIL images returns None."""

    class MockImage:
        def save(self, *args: object, **kwargs: object) -> None:
            pass

        format = "PNG"

    mock_image = MockImage()
    assert encode_hook(mock_image) is None


def test_encode_hook_to_dict() -> None:
    """Test encoding objects with to_dict method."""

    class MockDataFrame:
        def to_dict(self) -> dict[str, list[int]]:
            return {"col1": [1, 2], "col2": [3, 4]}

    mock_df = MockDataFrame()
    result = encode_hook(mock_df)
    assert result == {"col1": [1, 2], "col2": [3, 4]}


def test_encode_hook_unsupported() -> None:
    """Test encoding unsupported types raises TypeError."""

    class UnsupportedType:
        pass

    obj = UnsupportedType()

    with pytest.raises(TypeError, match="Unsupported type.*UnsupportedType"):
        encode_hook(obj)


def test_serialize_simple() -> None:
    """Test serializing simple types."""

    result = serialize("hello")
    assert isinstance(result, bytes)

    result = serialize(42)
    assert isinstance(result, bytes)

    result = serialize([1, 2, 3])
    assert isinstance(result, bytes)

    result = serialize({"key": "value"})
    assert isinstance(result, bytes)


def test_serialize_with_kwargs() -> None:
    """Test serializing dict with additional kwargs."""
    base = {"key1": "value1"}
    result = serialize(base, key2="value2", key3=123)

    from msgspec import msgpack

    decoded = msgpack.decode(result)

    assert decoded == {"key1": "value1", "key2": "value2", "key3": 123}


def test_serialize_complex_object() -> None:
    """Test serializing complex objects."""
    obj = SampleDataclass(name="test", value=42, color=Color.GREEN)
    result = serialize(obj)

    assert isinstance(result, bytes)

    from msgspec import msgpack

    decoded = msgpack.decode(result)
    assert decoded["name"] == "test"
    assert decoded["value"] == 42
    assert decoded["color"] == "green"


def test_serialize_error() -> None:
    """Test serialization error handling."""

    class BadObject:
        def __init__(self) -> None:
            self.circular = self

    obj = BadObject()

    with pytest.raises(ValueError, match="Failed to serialize"):
        serialize(obj)


def test_deserialize_simple() -> None:
    """Test deserializing simple types."""

    data = serialize("hello")
    result: str = deserialize(data, str)
    assert result == "hello"

    data = serialize(42)
    result_int: int = deserialize(data, int)
    assert result_int == 42

    data = serialize([1, 2, 3])
    result_list: list[int] = deserialize(data, list[int])
    assert result_list == [1, 2, 3]


def test_deserialize_dict() -> None:
    """Test deserializing dictionaries."""
    data = serialize({"key": "value", "num": 123})
    result = deserialize(data, dict[str, Any])

    assert result == {"key": "value", "num": 123}


def test_deserialize_error() -> None:
    """Test deserialization error handling."""
    data = serialize("not a number")

    with pytest.raises(ValueError, match="Failed to deserialize to int"):
        deserialize(data, int)


def test_roundtrip_complex() -> None:
    """Test roundtrip serialization of complex objects."""
    original = {
        "name": "test",
        "items": [1, 2, 3],
        "metadata": {
            "created": "2024-01-01",
            "tags": ["a", "b", "c"],
        },
        "count": 42,
    }

    serialized = serialize(original)
    result = deserialize(serialized, dict[str, Any])

    assert result == original


def test_serialize_none_values() -> None:
    """Test serializing None values."""
    data = {"key": None, "value": 123}
    result = serialize(data)

    from msgspec import msgpack

    decoded = msgpack.decode(result)

    assert decoded["key"] is None
    assert decoded["value"] == 123


def test_encode_hook_method_priority() -> None:
    """Test method priority in encode_hook."""

    class MultiMethodObject:
        def to_dict(self) -> dict[str, str]:
            return {"from": "to_dict"}

        def as_dict(self) -> dict[str, str]:
            return {"from": "as_dict"}

        def dict(self) -> dict[str, str]:
            return {"from": "dict"}

    obj = MultiMethodObject()
    result = encode_hook(obj)
    assert result == {"from": "to_dict"}


def test_encode_hook_json_method() -> None:
    """Test encoding objects with json method."""

    class JsonObject:
        def json(self) -> str:
            return '{"key": "json_value"}'

    obj = JsonObject()
    result = encode_hook(obj)
    assert result == '{"key": "json_value"}'


def test_serialize_bytes_input() -> None:
    """Test that serialize handles bytes properly."""
    data = b"binary data"
    result = serialize(data)

    assert isinstance(result, bytes)

    from msgspec import msgpack

    decoded = msgpack.decode(result)
    assert decoded == data


def test_deserialize_with_bytes_input() -> None:
    """Test deserialize with bytes input."""
    original = {"test": "data"}
    serialized = serialize(original)

    result = deserialize(serialized, dict[str, str])
    assert result == original

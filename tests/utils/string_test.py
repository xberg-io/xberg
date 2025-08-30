"""Tests for string utility functions."""

from __future__ import annotations

from kreuzberg._utils._string import (
    _calculate_text_confidence,
    _fix_mojibake,
    _get_encoding_cache_key,
    normalize_spaces,
    safe_decode,
)


def test_safe_decode_empty_bytes() -> None:
    """Test safe_decode with empty bytes."""
    assert safe_decode(b"") == ""


def test_safe_decode_with_encoding() -> None:
    """Test safe_decode with provided encoding."""
    text = "Hello, World! שלום עולם"
    utf8_bytes = text.encode("utf-8")
    assert safe_decode(utf8_bytes, "utf-8") == text


def test_safe_decode_with_invalid_encoding() -> None:
    """Test safe_decode with invalid encoding falls back to detection."""
    text = "Hello, World!"
    utf8_bytes = text.encode("utf-8")
    result = safe_decode(utf8_bytes, "invalid-encoding")
    assert result == text


def test_safe_decode_with_windows_1252() -> None:
    """Test safe_decode with Windows-1252 encoding."""
    text = "café résumé"
    windows_bytes = text.encode("windows-1252")
    result = safe_decode(windows_bytes)
    assert result == text


def test_safe_decode_with_hebrew() -> None:
    """Test safe_decode with Hebrew Windows-1255 encoding."""
    hebrew_text = "שלום עולם"
    hebrew_bytes = hebrew_text.encode("windows-1255")
    result = safe_decode(hebrew_bytes)
    assert "שלום" in result or "עולם" in result


def test_safe_decode_cache_functionality() -> None:
    """Test that safe_decode uses cache for repeated content."""
    text = "Hello, repeated content!"
    utf8_bytes = text.encode("utf-8")

    result1 = safe_decode(utf8_bytes)
    assert result1 == text

    result2 = safe_decode(utf8_bytes)
    assert result2 == text
    assert result1 == result2


def test_safe_decode_fallback_to_latin1() -> None:
    """Test safe_decode fallback to latin-1 with replacement."""
    problematic_bytes = b"\xff\xfe\x00\x00\x01\x02\x03"
    result = safe_decode(problematic_bytes)
    assert isinstance(result, str)


def test_safe_decode_confidence_scoring() -> None:
    """Test that safe_decode uses confidence scoring for encoding selection."""
    good_text = "This is normal English text with good characters."
    utf8_bytes = good_text.encode("utf-8")
    result = safe_decode(utf8_bytes)
    assert result == good_text


def test_calculate_text_confidence_empty() -> None:
    """Test _calculate_text_confidence with empty text."""
    assert _calculate_text_confidence("") == 0.0


def test_calculate_text_confidence_normal_text() -> None:
    """Test _calculate_text_confidence with normal text."""
    text = "This is normal English text."
    confidence = _calculate_text_confidence(text)
    assert confidence > 0.8


def test_calculate_text_confidence_with_replacement_chars() -> None:
    """Test _calculate_text_confidence with replacement characters."""
    text = "Text with replacement \ufffd characters"
    confidence = _calculate_text_confidence(text)
    assert confidence < 1.0


def test_calculate_text_confidence_with_control_chars() -> None:
    """Test _calculate_text_confidence with control characters."""
    text = "Text with\x00control\x01chars"
    confidence = _calculate_text_confidence(text)
    assert confidence < 0.8


def test_calculate_text_confidence_cyrillic_penalty() -> None:
    """Test _calculate_text_confidence with suspicious Cyrillic."""
    suspicious_text = "аваыврдвфгхькол" * 5
    confidence = _calculate_text_confidence(suspicious_text)
    assert confidence <= 0.7


def test_fix_mojibake_empty() -> None:
    """Test _fix_mojibake with empty text."""
    assert _fix_mojibake("") == ""


def test_fix_mojibake_control_chars() -> None:
    """Test _fix_mojibake removes control characters."""
    text = "Text with\x00control\x01chars\x7f"
    cleaned = _fix_mojibake(text)
    assert "\x00" not in cleaned
    assert "\x01" not in cleaned
    assert "\x7f" not in cleaned
    assert cleaned == "Text withcontrolchars"


def test_fix_mojibake_replacement_chars() -> None:
    """Test _fix_mojibake removes replacement characters."""
    text = "Text with\ufffd\ufffdreplacement chars"
    cleaned = _fix_mojibake(text)
    assert "\ufffd" not in cleaned
    assert cleaned == "Text withreplacement chars"


def test_fix_mojibake_isolated_combining() -> None:
    """Test _fix_mojibake removes isolated combining marks."""
    text = "Text with\u0300\u0301isolated combining"
    cleaned = _fix_mojibake(text)
    assert "\u0300" not in cleaned
    assert len(cleaned) < len(text)


def test_fix_mojibake_cyrillic_detection() -> None:
    """Test _fix_mojibake detects but preserves Cyrillic patterns."""
    cyrillic_text = "аваыврдвфгхькол"
    result = _fix_mojibake(cyrillic_text)
    assert len(result) > 0


def test_normalize_spaces_empty() -> None:
    """Test normalize_spaces with empty text."""
    assert normalize_spaces("") == ""
    assert normalize_spaces("   ") == ""
    assert normalize_spaces("\n\n") == ""


def test_normalize_spaces_basic() -> None:
    """Test normalize_spaces with basic text."""
    text = "This   is    some  text"
    expected = "This is some text"
    assert normalize_spaces(text) == expected


def test_normalize_spaces_preserve_paragraphs() -> None:
    """Test normalize_spaces preserves paragraph breaks."""
    text = "First paragraph.\n\nSecond paragraph."
    expected = "First paragraph.\n\nSecond paragraph."
    assert normalize_spaces(text) == expected


def test_normalize_spaces_multiple_whitespace() -> None:
    """Test normalize_spaces handles multiple types of whitespace."""
    text = "Text\twith\fvarious\v\rwhitespace\xa0types"
    expected = "Text with various whitespace types"
    assert normalize_spaces(text) == expected


def test_normalize_spaces_preserve_single_newlines() -> None:
    """Test normalize_spaces preserves single newlines within paragraphs."""
    text = "Line 1\nLine 2\nLine 3"
    expected = "Line 1\nLine 2\nLine 3"
    assert normalize_spaces(text) == expected


def test_normalize_spaces_clean_multiple_newlines() -> None:
    """Test normalize_spaces cleans up multiple newlines within paragraphs."""
    text = "Line 1\n\n\nLine 2"
    expected = "Line 1\n\nLine 2"
    assert normalize_spaces(text) == expected


def test_normalize_spaces_remove_empty_lines() -> None:
    """Test normalize_spaces removes empty lines."""
    text = "Good line\n   \nAnother good line"
    expected = "Good line\nAnother good line"
    assert normalize_spaces(text) == expected


def test_normalize_spaces_complex_example() -> None:
    """Test normalize_spaces with complex mixed whitespace."""
    text = """
    First   paragraph with    extra spaces.



    Second paragraph\t\twith\ttabs.

    Third\n\n\nparagraph  with  newlines.
    """
    result = normalize_spaces(text)

    assert "First paragraph with extra spaces." in result
    assert "Second paragraph with tabs." in result
    assert "Third" in result
    assert "paragraph with newlines." in result

    paragraphs = result.split("\n\n")
    assert len(paragraphs) >= 2

    assert "   " not in result
    assert "\t\t" not in result


def test_get_encoding_cache_key() -> None:
    """Test _get_encoding_cache_key generates consistent keys."""
    hash1 = "abcdef123456"
    size1 = 1024
    key1 = _get_encoding_cache_key(hash1, size1)
    key2 = _get_encoding_cache_key(hash1, size1)

    assert key1 == key2
    assert hash1 in key1
    assert str(size1) in key1


def test_get_encoding_cache_key_different_inputs() -> None:
    """Test _get_encoding_cache_key generates different keys for different inputs."""
    key1 = _get_encoding_cache_key("hash1", 100)
    key2 = _get_encoding_cache_key("hash2", 100)
    key3 = _get_encoding_cache_key("hash1", 200)

    assert key1 != key2
    assert key1 != key3
    assert key2 != key3


def test_safe_decode_encoding_tries_fallback_encodings() -> None:
    """Test that safe_decode tries fallback encodings when detection fails."""
    text = "Simple ASCII text"
    ascii_bytes = text.encode("ascii")

    result = safe_decode(ascii_bytes)
    assert result == text


def test_safe_decode_caches_successful_detections() -> None:
    """Test that safe_decode caches successful encoding detections."""
    import kreuzberg._utils._string as string_module

    string_module._encoding_cache.clear()

    text = "Test caching functionality"
    utf8_bytes = text.encode("utf-8")

    result1 = safe_decode(utf8_bytes)
    assert result1 == text

    assert len(string_module._encoding_cache) > 0

    result2 = safe_decode(utf8_bytes)
    assert result2 == text


def test_safe_decode_cache_size_limit() -> None:
    """Test that safe_decode respects cache size limits."""
    import kreuzberg._utils._string as string_module

    string_module._encoding_cache.clear()

    for i in range(1005):
        unique_text = f"Unique text {i}"
        unique_bytes = unique_text.encode("utf-8")
        safe_decode(unique_bytes)

    assert len(string_module._encoding_cache) <= 1000

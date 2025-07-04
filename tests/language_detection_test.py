"""
Tests for language detection functionality.
"""

from __future__ import annotations

from typing import TYPE_CHECKING
from unittest.mock import MagicMock, patch

import pytest

from kreuzberg._language_detection import LanguageDetectionConfig, detect_languages
from kreuzberg.exceptions import MissingDependencyError

if TYPE_CHECKING:
    from pathlib import Path


@patch("kreuzberg._language_detection.HAS_FAST_LANGDETECT", True)
@patch("kreuzberg._language_detection.detect_multilingual")
@patch("kreuzberg._language_detection.detect")
def test_detect_languages_single_success(mock_detect: MagicMock, mock_detect_multi: MagicMock) -> None:
    """Test successful single language detection."""
    mock_result = {"lang": "EN", "score": 0.99}
    mock_detect.return_value = mock_result
    result = detect_languages("This is English text.")
    assert result == ["en"]
    mock_detect.assert_called_once_with("This is English text.", low_memory=True)


@patch("kreuzberg._language_detection.HAS_FAST_LANGDETECT", True)
@patch("kreuzberg._language_detection.detect_multilingual")
@patch("kreuzberg._language_detection.detect")
def test_detect_languages_multilingual_success(mock_detect: MagicMock, mock_detect_multi: MagicMock) -> None:
    """Test successful multilingual detection."""
    mock_results = [{"lang": "EN", "score": 0.5}, {"lang": "DE", "score": 0.3}, {"lang": "FR", "score": 0.2}]
    config = LanguageDetectionConfig(multilingual=True, top_k=3)
    mock_detect_multi.return_value = mock_results
    result = detect_languages("This is English text with some German words.", config)
    assert result == ["en", "de", "fr"]
    mock_detect_multi.assert_called_once_with("This is English text with some German words.", low_memory=True, k=3)


@patch("kreuzberg._language_detection.HAS_FAST_LANGDETECT", True)
@patch("kreuzberg._language_detection.detect_multilingual")
@patch("kreuzberg._language_detection.detect")
def test_detect_languages_with_high_accuracy_mode(mock_detect: MagicMock, mock_detect_multi: MagicMock) -> None:
    """Test language detection with high accuracy mode (low_memory=False)."""
    mock_result = {"lang": "JA", "score": 0.99}
    config = LanguageDetectionConfig(low_memory=False)
    mock_detect.return_value = mock_result
    result = detect_languages("これは日本語のテキストです。", config)
    assert result == ["ja"]
    mock_detect.assert_called_once_with("これは日本語のテキストです。", low_memory=False)


@patch("kreuzberg._language_detection.HAS_FAST_LANGDETECT", True)
@patch("kreuzberg._language_detection.detect_multilingual")
@patch("kreuzberg._language_detection.detect")
def test_detect_languages_exception_handling(mock_detect: MagicMock, mock_detect_multi: MagicMock) -> None:
    """Test that exceptions in language detection are handled gracefully."""
    mock_detect.side_effect = Exception("Detection failed")
    result = detect_languages("Some text")
    assert result is None


def test_detect_languages_missing_dependency() -> None:
    """Test that MissingDependencyError is raised when fast-langdetect is not available."""
    detect_languages.cache_clear()
    with (
        patch("kreuzberg._language_detection.HAS_FAST_LANGDETECT", False),
        pytest.raises(MissingDependencyError, match="fast-langdetect"),
    ):
        detect_languages("Some text")


@patch("kreuzberg._language_detection.HAS_FAST_LANGDETECT", True)
@patch("kreuzberg._language_detection.detect_multilingual")
@patch("kreuzberg._language_detection.detect")
def test_detect_languages_caching(mock_detect: MagicMock, mock_detect_multi: MagicMock) -> None:
    """Test that language detection results are cached."""
    mock_result = {"lang": "EN", "score": 0.99}
    config = LanguageDetectionConfig()
    mock_detect.return_value = mock_result

    result1 = detect_languages("This is English text.", config)

    result2 = detect_languages("This is English text.", config)

    assert result1 == result2

    assert mock_detect.call_count == 1


@patch("kreuzberg._language_detection.HAS_FAST_LANGDETECT", True)
@patch("kreuzberg._language_detection.detect_multilingual")
@patch("kreuzberg._language_detection.detect")
def test_detect_languages_different_texts_not_cached(mock_detect: MagicMock, mock_detect_multi: MagicMock) -> None:
    """Test that different texts are not cached together."""
    detect_languages.cache_clear()
    mock_result1 = {"lang": "EN", "score": 0.99}
    mock_result2 = {"lang": "DE", "score": 0.99}
    mock_detect.side_effect = [mock_result1, mock_result2]
    result1 = detect_languages("This is English text 1.")
    result2 = detect_languages("Das ist deutscher Text 2.")
    assert result1 == ["en"]
    assert result2 == ["de"]
    assert mock_detect.call_count == 2


@patch("kreuzberg._language_detection.HAS_FAST_LANGDETECT", True)
@patch("kreuzberg._language_detection.detect_multilingual")
@patch("kreuzberg._language_detection.detect")
def test_detect_languages_with_custom_config(mock_detect: MagicMock, mock_detect_multi: MagicMock) -> None:
    """Test language detection with custom configuration."""
    mock_results = [{"lang": "ZH", "score": 0.6}, {"lang": "JA", "score": 0.4}]
    config = LanguageDetectionConfig(
        multilingual=True, top_k=2, low_memory=False, cache_dir="/custom/cache", allow_fallback=False
    )
    mock_detect_multi.return_value = mock_results
    result = detect_languages("混合的中文和日本語テキスト", config)
    assert result == ["zh", "ja"]
    mock_detect_multi.assert_called_once_with("混合的中文和日本語テキスト", low_memory=False, k=2)


@patch("kreuzberg._language_detection.HAS_FAST_LANGDETECT", True)
@patch("kreuzberg._language_detection.detect_multilingual")
@patch("kreuzberg._language_detection.detect")
def test_detect_languages_empty_result(mock_detect: MagicMock, mock_detect_multi: MagicMock) -> None:
    """Test handling of empty detection results."""

    mock_detect.return_value = {"score": 0.1}
    result = detect_languages("Unknown text")
    assert result is None

    mock_detect_multi.return_value = []
    config = LanguageDetectionConfig(multilingual=True)
    result = detect_languages("Unknown text", config)
    assert result == []


@pytest.mark.anyio
async def test_extract_file_with_language_detection(tmp_path: Path) -> None:
    """Test that language detection works with extract_file."""
    from kreuzberg import ExtractionConfig, extract_file

    test_file = tmp_path / "test.txt"
    test_file.write_text("This is English text for testing language detection.")
    with (
        patch("kreuzberg._language_detection.HAS_FAST_LANGDETECT", True),
        patch("kreuzberg._language_detection.detect") as mock_detect,
        patch("kreuzberg._language_detection.detect_multilingual"),
    ):
        mock_detect.return_value = {"lang": "EN", "score": 0.99}
        config = ExtractionConfig(auto_detect_language=True)
        result = await extract_file(test_file, config=config)
        assert result.detected_languages == ["en"]


@pytest.mark.anyio
async def test_extract_file_without_language_detection(tmp_path: Path) -> None:
    """Test that language detection is not performed when disabled."""
    from kreuzberg import ExtractionConfig, extract_file

    test_file = tmp_path / "test.txt"
    test_file.write_text("This is English text.")

    with (
        patch("kreuzberg._language_detection.HAS_FAST_LANGDETECT", True),
        patch("kreuzberg._language_detection.detect") as mock_detect,
        patch("kreuzberg._language_detection.detect_multilingual") as mock_detect_multi,
    ):
        config = ExtractionConfig(auto_detect_language=False)
        result = await extract_file(test_file, config=config)

        assert result.detected_languages is None
        mock_detect.assert_not_called()
        mock_detect_multi.assert_not_called()


@pytest.mark.anyio
async def test_extract_file_with_custom_language_config(tmp_path: Path) -> None:
    """Test extraction with custom language detection configuration."""
    from kreuzberg import ExtractionConfig, extract_file

    test_file = tmp_path / "test.txt"
    test_file.write_text("This is a multilingual text with multiple languages.")

    mock_results = [{"lang": "EN", "score": 0.7}, {"lang": "ES", "score": 0.3}]

    detect_languages.cache_clear()

    with (
        patch("kreuzberg._language_detection.HAS_FAST_LANGDETECT", True),
        patch("kreuzberg._language_detection.detect"),
        patch("kreuzberg._language_detection.detect_multilingual") as mock_detect_multi,
    ):
        mock_detect_multi.return_value = mock_results

        lang_config = LanguageDetectionConfig(multilingual=True, top_k=2, low_memory=False)
        config = ExtractionConfig(auto_detect_language=True, language_detection_config=lang_config)
        result = await extract_file(test_file, config=config)

        assert result.detected_languages == ["en", "es"]
        mock_detect_multi.assert_called_once()


@pytest.mark.anyio
async def test_extract_file_language_detection_missing_dependency(tmp_path: Path) -> None:
    """Test that extraction fails when language detection is enabled but dependency is missing."""
    from kreuzberg import ExtractionConfig, extract_file

    test_file = tmp_path / "test.txt"
    test_content = "This is test content."
    test_file.write_text(test_content)

    with patch("kreuzberg._language_detection.HAS_FAST_LANGDETECT", False):
        config = ExtractionConfig(auto_detect_language=True)
        with pytest.raises(MissingDependencyError, match="fast-langdetect"):
            await extract_file(test_file, config=config)


def test_extract_file_sync_with_language_detection(tmp_path: Path) -> None:
    """Test that language detection works with extract_file_sync."""
    from kreuzberg import ExtractionConfig, extract_file_sync

    test_file = tmp_path / "test.txt"
    test_file.write_text("This is English text for testing language detection.")
    with (
        patch("kreuzberg._language_detection.HAS_FAST_LANGDETECT", True),
        patch("kreuzberg._language_detection.detect") as mock_detect,
        patch("kreuzberg._language_detection.detect_multilingual"),
    ):
        mock_detect.return_value = {"lang": "EN", "score": 0.99}
        config = ExtractionConfig(auto_detect_language=True)
        result = extract_file_sync(test_file, config=config)
        assert result.detected_languages == ["en"]

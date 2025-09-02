from __future__ import annotations

from unittest.mock import MagicMock

import pytest

from kreuzberg import MissingDependencyError
from kreuzberg._language_detection import LanguageDetectionConfig, _create_fast_langdetect_config, detect_languages


def test_language_detection_config_defaults() -> None:
    config = LanguageDetectionConfig()
    assert config.low_memory is True
    assert config.top_k == 3
    assert config.multilingual is False
    assert config.cache_dir is None
    assert config.allow_fallback is True


def test_language_detection_config_custom_values() -> None:
    config = LanguageDetectionConfig(
        low_memory=False, top_k=5, multilingual=True, cache_dir="/tmp/cache", allow_fallback=False
    )
    assert config.low_memory is False
    assert config.top_k == 5
    assert config.multilingual is True
    assert config.cache_dir == "/tmp/cache"
    assert config.allow_fallback is False


def test_detect_languages_missing_dependency(monkeypatch: pytest.MonkeyPatch) -> None:
    import kreuzberg._language_detection as ld

    monkeypatch.setattr(ld, "HAS_FAST_LANGDETECT", False)
    monkeypatch.setattr(ld, "detect", None)
    monkeypatch.setattr(ld, "detect_multilingual", None)

    with pytest.raises(MissingDependencyError, match="fast-langdetect"):
        detect_languages("Hello world")


def test_detect_languages_with_results(monkeypatch: pytest.MonkeyPatch) -> None:
    def mock_detect(text: str, low_memory: bool = True) -> dict[str, str | float]:
        return {"lang": "en", "confidence": 0.95}

    import kreuzberg._language_detection as ld

    monkeypatch.setattr(ld, "HAS_FAST_LANGDETECT", True)
    monkeypatch.setattr(ld, "detect", mock_detect)
    monkeypatch.setattr(ld, "detect_multilingual", mock_detect)

    result = detect_languages("Hello world", LanguageDetectionConfig())
    assert result == ["en"]


def test_detect_languages_empty_input(monkeypatch: pytest.MonkeyPatch) -> None:
    def mock_detect(text: str, low_memory: bool = True) -> dict[str, str | float] | None:
        return None

    import kreuzberg._language_detection as ld

    monkeypatch.setattr(ld, "HAS_FAST_LANGDETECT", True)
    monkeypatch.setattr(ld, "detect", mock_detect)
    monkeypatch.setattr(ld, "detect_multilingual", mock_detect)

    result = detect_languages("", LanguageDetectionConfig())
    assert result is None


def test_detect_languages_multilingual(monkeypatch: pytest.MonkeyPatch) -> None:
    def mock_detect_multilingual(text: str, low_memory: bool = True, k: int = 3) -> list[dict[str, str | float]]:
        return [
            {"lang": "en", "confidence": 0.8},
            {"lang": "fr", "confidence": 0.2},
        ]

    import kreuzberg._language_detection as ld

    monkeypatch.setattr(ld, "HAS_FAST_LANGDETECT", True)
    monkeypatch.setattr(ld, "detect", lambda *_args, **_kwargs: {"lang": "en"})
    monkeypatch.setattr(ld, "detect_multilingual", mock_detect_multilingual)

    result = detect_languages("text", LanguageDetectionConfig(multilingual=True, top_k=2))
    assert result == ["en", "fr"]


def test_detect_languages_exception_handling(monkeypatch: pytest.MonkeyPatch) -> None:
    def mock_detect_error(text: str, low_memory: bool = True) -> None:
        raise RuntimeError("Detection failed")

    import kreuzberg._language_detection as ld

    monkeypatch.setattr(ld, "HAS_FAST_LANGDETECT", True)
    monkeypatch.setattr(ld, "detect", mock_detect_error)
    monkeypatch.setattr(ld, "detect_multilingual", mock_detect_error)

    result = detect_languages("text", LanguageDetectionConfig())
    assert result is None


def test_detect_languages_with_none_config(monkeypatch: pytest.MonkeyPatch) -> None:
    def mock_detect(text: str, low_memory: bool = True) -> dict[str, str | float]:
        return {"lang": "de", "confidence": 0.9}

    import kreuzberg._language_detection as ld

    monkeypatch.setattr(ld, "HAS_FAST_LANGDETECT", True)
    monkeypatch.setattr(ld, "detect", mock_detect)
    monkeypatch.setattr(ld, "detect_multilingual", mock_detect)

    result = detect_languages("Hallo Welt")
    assert result == ["de"]


def test_create_fast_langdetect_config_missing_dependency(monkeypatch: pytest.MonkeyPatch) -> None:
    import kreuzberg._language_detection as ld

    monkeypatch.setattr(ld, "HAS_FAST_LANGDETECT", False)
    monkeypatch.setattr(ld, "FastLangDetectConfig", None)

    config = LanguageDetectionConfig()
    result = _create_fast_langdetect_config(config)
    assert result is None


def test_create_fast_langdetect_config_with_cache_dir(monkeypatch: pytest.MonkeyPatch) -> None:
    mock_config_class = MagicMock()

    import kreuzberg._language_detection as ld

    monkeypatch.setattr(ld, "HAS_FAST_LANGDETECT", True)
    monkeypatch.setattr(ld, "FastLangDetectConfig", mock_config_class)

    config = LanguageDetectionConfig(cache_dir="/tmp/test", allow_fallback=False)
    _create_fast_langdetect_config(config)

    mock_config_class.assert_called_once_with(allow_fallback=False, cache_dir="/tmp/test")


def test_create_fast_langdetect_config_without_cache_dir(monkeypatch: pytest.MonkeyPatch) -> None:
    mock_config_class = MagicMock()

    import kreuzberg._language_detection as ld

    monkeypatch.setattr(ld, "HAS_FAST_LANGDETECT", True)
    monkeypatch.setattr(ld, "FastLangDetectConfig", mock_config_class)

    config = LanguageDetectionConfig(cache_dir=None, allow_fallback=True)
    _create_fast_langdetect_config(config)

    mock_config_class.assert_called_once_with(allow_fallback=True)

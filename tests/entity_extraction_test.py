from __future__ import annotations

import sys
from typing import Any
from unittest.mock import MagicMock

import pytest

from kreuzberg import MissingDependencyError
from kreuzberg import _entity_extraction as ee
from kreuzberg._types import Entity, SpacyEntityExtractionConfig

SAMPLE_TEXT = "John Doe visited Berlin on 2023-01-01. Contact: john@example.com or +49-123-4567."


@pytest.mark.parametrize(
    "custom_patterns,expected",
    [
        (None, []),
        (frozenset([("INVOICE_ID", r"INV-\d+")]), []),
        (
            frozenset([("EMAIL", r"[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+")]),
            [Entity(type="EMAIL", text="john@example.com", start=48, end=64)],
        ),
    ],
)
def test_custom_entity_patterns(
    custom_patterns: frozenset[tuple[str, str]] | None, expected: list[Entity], monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.setitem(sys.modules, "spacy", MagicMock())
    entities = ee.extract_entities(SAMPLE_TEXT, entity_types=(), custom_patterns=custom_patterns)
    assert all(isinstance(e, Entity) for e in entities)
    if expected:
        assert any(e.type == "EMAIL" and e.text == "john@example.com" for e in entities)
    else:
        assert entities == expected


def test_extract_entities_with_spacy(monkeypatch: pytest.MonkeyPatch) -> None:
    class DummyEnt:
        def __init__(self, label: str, text: str, start_char: int, end_char: int):
            self.label_ = label
            self.text = text
            self.start_char = start_char
            self.end_char = end_char

    class DummyDoc:
        def __init__(self, text: str):
            self.ents = [
                DummyEnt("PERSON", "John Doe", 0, 8),
                DummyEnt("GPE", "Berlin", 18, 24),
            ]

    class DummyNLP:
        max_length = 1000000

        def __call__(self, text: str) -> DummyDoc:
            return DummyDoc(text)

    def mock_load(_model_name: str) -> DummyNLP:
        return DummyNLP()

    mock_spacy = MagicMock()
    mock_spacy.load = mock_load
    monkeypatch.setitem(sys.modules, "spacy", mock_spacy)

    def mock_load_spacy_model(model_name: str, spacy_config: Any) -> DummyNLP:
        return DummyNLP()

    monkeypatch.setattr(ee, "_load_spacy_model", mock_load_spacy_model)

    result = ee.extract_entities(SAMPLE_TEXT, entity_types=["PERSON", "GPE"], languages=["en"])
    assert any(e.type == "PERSON" and e.text == "John Doe" for e in result)
    assert any(e.type == "GPE" and e.text == "Berlin" for e in result)
    assert all(isinstance(e, Entity) for e in result)


def test_extract_keywords_with_keybert(monkeypatch: pytest.MonkeyPatch) -> None:
    class DummyModel:
        def extract_keywords(self, _text: str, top_n: int = 10) -> list[tuple[str, float]]:
            if top_n == 2:
                return [("Berlin", 0.9), ("John Doe", 0.8)]
            return [("keyword", 0.5)] * top_n

    mock_keybert = MagicMock()
    mock_keybert.KeyBERT = DummyModel
    monkeypatch.setitem(sys.modules, "keybert", mock_keybert)

    result = ee.extract_keywords(SAMPLE_TEXT, keyword_count=2)
    assert result == [("Berlin", 0.9), ("John Doe", 0.8)]


def test_extract_entities_missing_spacy(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setitem(sys.modules, "spacy", None)
    with pytest.raises(MissingDependencyError):
        ee.extract_entities(SAMPLE_TEXT, entity_types=["PERSON"])


def test_extract_keywords_missing_keybert(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setitem(sys.modules, "keybert", None)
    with pytest.raises(MissingDependencyError):
        ee.extract_keywords(SAMPLE_TEXT, keyword_count=5)


def test_spacy_entity_extraction_config_defaults() -> None:
    config = SpacyEntityExtractionConfig()
    assert config.language_models is not None
    assert isinstance(config.language_models, tuple)
    assert config.fallback_to_multilingual is True
    assert config.max_doc_length == 1000000
    assert config.batch_size == 1000


def test_spacy_entity_extraction_config_custom_models() -> None:
    custom_models = {"en": "en_core_web_lg", "fr": "fr_core_news_sm"}
    config = SpacyEntityExtractionConfig(language_models=custom_models)
    assert isinstance(config.language_models, tuple)
    assert dict(config.language_models) == custom_models


def test_spacy_entity_extraction_config_get_model_for_language() -> None:
    config = SpacyEntityExtractionConfig()

    assert config.get_model_for_language("en") == "en_core_web_sm"
    assert config.get_model_for_language("de") == "de_core_news_sm"

    assert config.get_model_for_language("en-US") == "en_core_web_sm"
    assert config.get_model_for_language("de-DE") == "de_core_news_sm"

    assert config.get_model_for_language("xx") is None
    assert config.get_model_for_language("nonexistent") is None


def test_spacy_entity_extraction_config_get_fallback_model() -> None:
    config_with_fallback = SpacyEntityExtractionConfig(fallback_to_multilingual=True)
    assert config_with_fallback.get_fallback_model() == "xx_ent_wiki_sm"

    config_without_fallback = SpacyEntityExtractionConfig(fallback_to_multilingual=False)
    assert config_without_fallback.get_fallback_model() is None


def test_spacy_entity_extraction_config_empty_models() -> None:
    config = SpacyEntityExtractionConfig(language_models={})
    assert config.get_model_for_language("en") is None


def test_spacy_entity_extraction_config_model_cache_dir() -> None:
    import tempfile

    with tempfile.TemporaryDirectory() as temp_dir:
        config = SpacyEntityExtractionConfig(model_cache_dir=temp_dir)
        assert str(config.model_cache_dir) == temp_dir


def test_select_spacy_model_fallback() -> None:
    config = SpacyEntityExtractionConfig(language_models={"en": "en_core_web_sm"}, fallback_to_multilingual=True)

    model = ee._select_spacy_model(["en"], config)
    assert model == "en_core_web_sm"

    model = ee._select_spacy_model(["nonexistent"], config)
    assert model == "xx_ent_wiki_sm"

    config_no_fallback = SpacyEntityExtractionConfig(
        language_models={"en": "en_core_web_sm"}, fallback_to_multilingual=False
    )
    model = ee._select_spacy_model(["nonexistent"], config_no_fallback)
    assert model is None


def test_extract_entities_empty_input() -> None:
    result = ee.extract_entities("", entity_types=["PERSON"])
    assert result == []


def test_extract_entities_no_entities_types() -> None:
    result = ee.extract_entities(SAMPLE_TEXT, entity_types=())
    assert isinstance(result, list)


def test_extract_keywords_with_default_count(monkeypatch: pytest.MonkeyPatch) -> None:
    class DummyModel:
        def extract_keywords(self, _text: str, top_n: int = 10) -> list[tuple[str, float]]:
            return [("keyword", 0.5)] * min(top_n, 3)

    mock_keybert = MagicMock()
    mock_keybert.KeyBERT = DummyModel
    monkeypatch.setitem(sys.modules, "keybert", mock_keybert)

    result = ee.extract_keywords(SAMPLE_TEXT)
    assert len(result) == 3


def test_extract_keywords_empty_input(monkeypatch: pytest.MonkeyPatch) -> None:
    class DummyModel:
        def extract_keywords(self, _text: str, top_n: int = 10) -> list[tuple[str, float]]:
            return []

    mock_keybert = MagicMock()
    mock_keybert.KeyBERT = DummyModel
    monkeypatch.setitem(sys.modules, "keybert", mock_keybert)

    result = ee.extract_keywords("")
    assert result == []


def test_spacy_entity_extraction_config_comprehensive_tuple_language_models() -> None:
    models_tuple = (("en", "en_core_web_sm"), ("fr", "fr_core_news_sm"))
    config = SpacyEntityExtractionConfig(language_models=models_tuple)

    assert isinstance(config.language_models, tuple)
    assert config.language_models == models_tuple


def test_spacy_entity_extraction_config_comprehensive_default_language_models() -> None:
    config = SpacyEntityExtractionConfig()
    models_dict = dict(config.language_models) if isinstance(config.language_models, tuple) else config.language_models

    expected_languages = ["en", "de", "fr", "es", "pt", "it", "nl", "zh", "ja", "ko", "ru"]
    assert models_dict is not None
    for lang in expected_languages:
        assert lang in models_dict
        assert models_dict[lang].endswith("_sm")


def test_spacy_entity_extraction_config_comprehensive_get_model_complex_language_codes() -> None:
    config = SpacyEntityExtractionConfig()

    assert config.get_model_for_language("en-US-POSIX") == "en_core_web_sm"
    assert config.get_model_for_language("de-DE-1996") == "de_core_news_sm"

    assert config.get_model_for_language("EN") == "en_core_web_sm"
    assert config.get_model_for_language("En-Us") == "en_core_web_sm"


def test_spacy_entity_extraction_config_comprehensive_no_language_models() -> None:
    config = SpacyEntityExtractionConfig()
    object.__setattr__(config, "language_models", None)

    assert config.get_model_for_language("en") is None
    assert config.get_model_for_language("any") is None


def test_spacy_entity_extraction_config_comprehensive_pathlib_model_cache_dir() -> None:
    import tempfile
    from pathlib import Path

    with tempfile.TemporaryDirectory() as temp_dir:
        path_dir = Path(temp_dir)
        config = SpacyEntityExtractionConfig(model_cache_dir=path_dir)
        assert config.model_cache_dir == path_dir


def test_entity_extraction_edge_cases_very_long_text(monkeypatch: pytest.MonkeyPatch) -> None:
    class DummyDoc:
        def __init__(self, text: str):
            self.ents: list[Any] = []

    class DummyNLP:
        max_length = 1000000

        def __call__(self, text: str) -> DummyDoc:
            return DummyDoc(text)

    def mock_load_spacy_model(model_name: str, spacy_config: Any) -> DummyNLP:
        return DummyNLP()

    mock_spacy = MagicMock()
    monkeypatch.setitem(sys.modules, "spacy", mock_spacy)
    monkeypatch.setattr(ee, "_load_spacy_model", mock_load_spacy_model)

    long_text = "a" * 1500000
    config = SpacyEntityExtractionConfig(max_doc_length=1000000)

    result = ee.extract_entities(long_text, entity_types=["PERSON"], spacy_config=config)
    assert isinstance(result, list)


def test_entity_extraction_edge_cases_custom_max_doc_length(monkeypatch: pytest.MonkeyPatch) -> None:
    class DummyDoc:
        def __init__(self, text: str):
            self.ents: list[Any] = []

    class DummyNLP:
        max_length = 100

        def __call__(self, text: str) -> DummyDoc:
            assert len(text) <= 100
            return DummyDoc(text)

    def mock_load_spacy_model(model_name: str, spacy_config: Any) -> DummyNLP:
        return DummyNLP()

    mock_spacy = MagicMock()
    monkeypatch.setitem(sys.modules, "spacy", mock_spacy)
    monkeypatch.setattr(ee, "_load_spacy_model", mock_load_spacy_model)

    long_text = "This is a long text. " * 10
    config = SpacyEntityExtractionConfig(max_doc_length=100)

    result = ee.extract_entities(long_text, entity_types=["PERSON"], spacy_config=config)
    assert isinstance(result, list)


def test_entity_extraction_edge_cases_multiple_custom_patterns(monkeypatch: pytest.MonkeyPatch) -> None:
    mock_spacy = MagicMock()
    monkeypatch.setitem(sys.modules, "spacy", mock_spacy)

    text = "Invoice INV-001 dated 2023-01-01, email: test@example.com, phone: +1-555-123-4567"
    custom_patterns = frozenset(
        [
            ("INVOICE_ID", r"INV-\d+"),
            ("DATE", r"\d{4}-\d{2}-\d{2}"),
            ("EMAIL", r"[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+"),
            ("PHONE", r"\+\d{1,3}-\d{3}-\d{3}-\d{4}"),
        ]
    )

    result = ee.extract_entities(text, entity_types=(), custom_patterns=custom_patterns)

    assert len(result) == 4
    entity_types = {e.type for e in result}
    assert entity_types == {"INVOICE_ID", "DATE", "EMAIL", "PHONE"}


def test_entity_extraction_edge_cases_overlapping_patterns(monkeypatch: pytest.MonkeyPatch) -> None:
    mock_spacy = MagicMock()
    monkeypatch.setitem(sys.modules, "spacy", mock_spacy)

    text = "test@example.com"
    custom_patterns = frozenset(
        [
            ("EMAIL", r"[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+"),
            ("DOMAIN", r"@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+"),
            ("TLD", r"\.[a-zA-Z0-9-.]+$"),
        ]
    )

    result = ee.extract_entities(text, entity_types=(), custom_patterns=custom_patterns)

    assert len(result) == 3
    entity_types = {e.type for e in result}
    assert entity_types == {"EMAIL", "DOMAIN", "TLD"}


def test_entity_extraction_edge_cases_spacy_model_loading_failure(monkeypatch: pytest.MonkeyPatch) -> None:
    mock_spacy = MagicMock()
    monkeypatch.setitem(sys.modules, "spacy", mock_spacy)

    def mock_load_spacy_model_fail(model_name: str, spacy_config: Any) -> None:
        return None

    monkeypatch.setattr(ee, "_load_spacy_model", mock_load_spacy_model_fail)

    result = ee.extract_entities(SAMPLE_TEXT, entity_types=["PERSON"], languages=["en"])
    assert result == []


def test_entity_extraction_edge_cases_no_model_selected(monkeypatch: pytest.MonkeyPatch) -> None:
    mock_spacy = MagicMock()
    monkeypatch.setitem(sys.modules, "spacy", mock_spacy)

    def mock_select_spacy_model_none(languages: list[str] | None, spacy_config: Any) -> None:
        return None

    monkeypatch.setattr(ee, "_select_spacy_model", mock_select_spacy_model_none)

    result = ee.extract_entities(SAMPLE_TEXT, entity_types=["PERSON"], languages=["unknown"])
    assert result == []


def test_entity_extraction_edge_cases_case_insensitive_entity_types(monkeypatch: pytest.MonkeyPatch) -> None:
    class DummyEnt:
        def __init__(self, label: str, text: str, start_char: int, end_char: int):
            self.label_ = label
            self.text = text
            self.start_char = start_char
            self.end_char = end_char

    class DummyDoc:
        def __init__(self, text: str):
            self.ents = [
                DummyEnt("person", "John Doe", 0, 8),
                DummyEnt("GPE", "Berlin", 18, 24),
                DummyEnt("Date", "2023-01-01", 28, 38),
            ]

    class DummyNLP:
        max_length = 1000000

        def __call__(self, text: str) -> DummyDoc:
            return DummyDoc(text)

    def mock_load_spacy_model(model_name: str, spacy_config: Any) -> DummyNLP:
        return DummyNLP()

    mock_spacy = MagicMock()
    monkeypatch.setitem(sys.modules, "spacy", mock_spacy)
    monkeypatch.setattr(ee, "_load_spacy_model", mock_load_spacy_model)

    result = ee.extract_entities(SAMPLE_TEXT, entity_types=["PERSON", "gpe", "date"], languages=["en"])

    entity_types = {e.type for e in result}
    assert entity_types == {"person", "GPE", "Date"}


def test_spacy_model_selection_comprehensive_empty_languages() -> None:
    config = SpacyEntityExtractionConfig()

    model = ee._select_spacy_model([], config)
    assert model == "en_core_web_sm"


def test_spacy_model_selection_comprehensive_multiple_languages_first_match() -> None:
    config = SpacyEntityExtractionConfig()

    model = ee._select_spacy_model(["unknown", "de", "fr"], config)
    assert model == "de_core_news_sm"


def test_spacy_model_selection_comprehensive_base_language_fallback() -> None:
    config = SpacyEntityExtractionConfig()

    model = ee._select_spacy_model(["de-AT"], config)
    assert model == "de_core_news_sm"


def test_spacy_model_selection_comprehensive_no_fallback_when_disabled() -> None:
    config = SpacyEntityExtractionConfig(fallback_to_multilingual=False)

    model = ee._select_spacy_model(["unknown"], config)
    assert model is None


def test_spacy_model_loading_comprehensive_with_cache_dir(monkeypatch: pytest.MonkeyPatch) -> None:
    import os
    import tempfile

    class DummyNLP:
        max_length = 1000000

    def mock_spacy_load(model_name: str) -> DummyNLP:
        return DummyNLP()

    mock_spacy = MagicMock()
    mock_spacy.load = mock_spacy_load
    monkeypatch.setitem(sys.modules, "spacy", mock_spacy)

    with tempfile.TemporaryDirectory() as temp_dir:
        config = SpacyEntityExtractionConfig(model_cache_dir=temp_dir, max_doc_length=500000)

        original_env = os.environ.get("SPACY_DATA")
        try:
            result = ee._load_spacy_model("en_core_web_sm", config)

            assert os.environ.get("SPACY_DATA") == temp_dir
            assert result is not None
            assert result.max_length == 500000
        finally:
            if original_env:
                os.environ["SPACY_DATA"] = original_env
            elif "SPACY_DATA" in os.environ:
                del os.environ["SPACY_DATA"]


def test_spacy_model_loading_comprehensive_os_error(monkeypatch: pytest.MonkeyPatch) -> None:
    def mock_spacy_load_error(model_name: str) -> None:
        raise OSError("Model not found")

    mock_spacy = MagicMock()
    mock_spacy.load = mock_spacy_load_error
    monkeypatch.setitem(sys.modules, "spacy", mock_spacy)

    config = SpacyEntityExtractionConfig()
    result = ee._load_spacy_model("nonexistent_model", config)
    assert result is None


def test_spacy_model_loading_comprehensive_import_error(monkeypatch: pytest.MonkeyPatch) -> None:
    def mock_spacy_load_error(model_name: str) -> None:
        raise ImportError("spaCy import error")

    mock_spacy = MagicMock()
    mock_spacy.load = mock_spacy_load_error
    monkeypatch.setitem(sys.modules, "spacy", mock_spacy)

    config = SpacyEntityExtractionConfig()
    result = ee._load_spacy_model("problematic_model", config)
    assert result is None


def test_keyword_extraction_comprehensive_runtime_error(monkeypatch: pytest.MonkeyPatch) -> None:
    class DummyModel:
        def extract_keywords(self, _text: str, top_n: int = 10) -> None:
            raise RuntimeError("KeyBERT processing error")

    mock_keybert = MagicMock()
    mock_keybert.KeyBERT = DummyModel
    monkeypatch.setitem(sys.modules, "keybert", mock_keybert)

    result = ee.extract_keywords(SAMPLE_TEXT, keyword_count=5)
    assert result == []


def test_keyword_extraction_comprehensive_os_error(monkeypatch: pytest.MonkeyPatch) -> None:
    class DummyModel:
        def extract_keywords(self, _text: str, top_n: int = 10) -> None:
            raise OSError("Model file not found")

    mock_keybert = MagicMock()
    mock_keybert.KeyBERT = DummyModel
    monkeypatch.setitem(sys.modules, "keybert", mock_keybert)

    result = ee.extract_keywords(SAMPLE_TEXT, keyword_count=5)
    assert result == []


def test_keyword_extraction_comprehensive_value_error(monkeypatch: pytest.MonkeyPatch) -> None:
    class DummyModel:
        def extract_keywords(self, _text: str, top_n: int = 10) -> None:
            raise ValueError("Invalid input parameters")

    mock_keybert = MagicMock()
    mock_keybert.KeyBERT = DummyModel
    monkeypatch.setitem(sys.modules, "keybert", mock_keybert)

    result = ee.extract_keywords(SAMPLE_TEXT, keyword_count=5)
    assert result == []


def test_keyword_extraction_comprehensive_zero_count(monkeypatch: pytest.MonkeyPatch) -> None:
    class DummyModel:
        def extract_keywords(self, _text: str, top_n: int = 10) -> list[tuple[str, float]]:
            return [] if top_n == 0 else [("keyword", 0.5)]

    mock_keybert = MagicMock()
    mock_keybert.KeyBERT = DummyModel
    monkeypatch.setitem(sys.modules, "keybert", mock_keybert)

    result = ee.extract_keywords(SAMPLE_TEXT, keyword_count=0)
    assert result == []


def test_keyword_extraction_comprehensive_float_conversion(monkeypatch: pytest.MonkeyPatch) -> None:
    class DummyModel:
        def extract_keywords(self, _text: str, top_n: int = 10) -> list[tuple[str, Any]]:
            return [
                ("keyword1", 0.95),
                ("keyword2", 85),
                ("keyword3", "0.75"),
            ]

    mock_keybert = MagicMock()
    mock_keybert.KeyBERT = DummyModel
    monkeypatch.setitem(sys.modules, "keybert", mock_keybert)

    result = ee.extract_keywords(SAMPLE_TEXT, keyword_count=3)

    assert len(result) == 3
    assert all(isinstance(score, float) for _, score in result)
    assert result == [("keyword1", 0.95), ("keyword2", 85.0), ("keyword3", 0.75)]


def test_entity_extraction_configuration_edge_cases_post_init_dict_conversion() -> None:
    models_dict = {"fr": "fr_core_news_sm", "en": "en_core_web_sm", "de": "de_core_news_sm"}
    config = SpacyEntityExtractionConfig(language_models=models_dict)

    assert isinstance(config.language_models, tuple)
    expected_tuple = tuple(sorted(models_dict.items()))
    assert config.language_models == expected_tuple


def test_entity_extraction_configuration_edge_cases_post_init_none_to_defaults() -> None:
    config = SpacyEntityExtractionConfig(language_models=None)

    assert config.language_models is not None
    assert isinstance(config.language_models, tuple)
    assert len(config.language_models) > 0


def test_entity_extraction_configuration_edge_cases_immutability() -> None:
    config = SpacyEntityExtractionConfig()

    with pytest.raises(AttributeError):
        config.max_doc_length = 500000  # type: ignore[misc]

    with pytest.raises(AttributeError):
        config.fallback_to_multilingual = False  # type: ignore[misc]

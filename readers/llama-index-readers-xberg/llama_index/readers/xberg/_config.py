"""ExtractionConfig dict <-> object reconstruction for pipeline persistence.

Xberg's ExtractionConfig is a PyO3 class with no from_dict() method.
These helpers reconstruct typed config objects from serialized dicts,
enabling lossless to_dict() / from_dict() round-trips.

PyO3 classes do not expose constructor type hints via get_type_hints(), so
sub-sub-config field mappings are declared as static dicts rather than
derived from annotations.
"""

import functools
import inspect
from typing import Any

import xberg as _xberg
from xberg import (
    ChunkingConfig,
    EmbeddingConfig,
    ExtractionConfig,
    HierarchyConfig,
    ImageExtractionConfig,
    ImagePreprocessingConfig,
    KeywordConfig,
    LanguageDetectionConfig,
    OcrConfig,
    PageConfig,
    PdfConfig,
    PostProcessorConfig,
    RakeParams,
    TesseractConfig,
    TokenReductionConfig,
    YakeParams,
)

# Top-level sub-config fields on ExtractionConfig: field_name -> config class.
# Only fields that hold config objects (not scalars or lists) are listed here.
_TOP_LEVEL_CONFIGS: dict[str, type] = {
    "chunking": ChunkingConfig,
    "images": ImageExtractionConfig,
    "keywords": KeywordConfig,
    "language_detection": LanguageDetectionConfig,
    "ocr": OcrConfig,
    "pages": PageConfig,
    "pdf_options": PdfConfig,
    "postprocessor": PostProcessorConfig,
    "token_reduction": TokenReductionConfig,
}

_OPTIONAL_TOP_LEVEL: list[tuple[str, str]] = [
    ("acceleration", "AccelerationConfig"),
    ("concurrency", "ConcurrencyConfig"),
    ("content_filter", "ContentFilterConfig"),
    ("email", "EmailConfig"),
    ("html_output", "HtmlOutputConfig"),
    ("layout", "LayoutDetectionConfig"),
    ("tree_sitter", "TreeSitterConfig"),
]

for _field, _cls_name in _OPTIONAL_TOP_LEVEL:
    _cls = getattr(_xberg, _cls_name, None)
    if _cls is not None:
        _TOP_LEVEL_CONFIGS[_field] = _cls

# Nested config fields on sub-config classes: (class, field_name) -> inner class.
# Required because PyO3 classes reject raw dicts for typed constructor arguments.
_NESTED_FIELD_MAP: dict[tuple[type, str], type] = {
    (OcrConfig, "tesseract_config"): TesseractConfig,
    (TesseractConfig, "preprocessing"): ImagePreprocessingConfig,
    (PdfConfig, "hierarchy"): HierarchyConfig,
    (ChunkingConfig, "embedding"): EmbeddingConfig,
    (KeywordConfig, "rake_params"): RakeParams,
    (KeywordConfig, "yake_params"): YakeParams,
}

_tsc = getattr(_xberg, "TreeSitterConfig", None)
_tspc = getattr(_xberg, "TreeSitterProcessConfig", None)
if _tsc is not None and _tspc is not None:
    _NESTED_FIELD_MAP[(_tsc, "process")] = _tspc


@functools.lru_cache(maxsize=32)
def _known_fields(cls: type) -> frozenset[str]:
    """Return the set of field names accepted by a PyO3 config class constructor.

    Uses inspect.signature() to get actual constructor parameters. PyO3 classes
    may expose read-only attributes (e.g. computed properties) in dir() that are
    not valid constructor kwargs, so dir() is not a reliable proxy.
    """
    try:
        return frozenset(inspect.signature(cls).parameters)
    except (ValueError, TypeError):
        return frozenset()


def _reconstruct(cls: type, d: dict[str, Any]) -> Any:
    """Reconstruct a PyO3 config class from a dict.

    Filters out fields not accepted by the constructor (config_to_json can
    serialize fields that aren't valid constructor arguments), then handles
    sub-sub-configs by looking up each (cls, field_name) pair in the static
    _NESTED_FIELD_MAP and recursing when the value is a non-None dict.
    """
    accepted = _known_fields(cls)
    kwargs: dict[str, Any] = {}
    for key, value in d.items():
        if accepted and key not in accepted:
            continue
        if isinstance(value, dict):
            inner_cls = _NESTED_FIELD_MAP.get((cls, key))
            if inner_cls is not None:
                kwargs[key] = _reconstruct(inner_cls, value)
                continue
        kwargs[key] = value
    return cls(**kwargs)


def dict_to_config(d: dict[str, Any]) -> ExtractionConfig:
    """Reconstruct an ExtractionConfig from a serialized dict.

    Converts nested dicts to their typed PyO3 counterparts, then constructs
    ExtractionConfig. Fields present in the dict that are not recognised by
    the installed xberg version are silently ignored so that configs
    serialized with a newer xberg can still be loaded.

    Args:
        d: A dict produced by ``json.loads(config_to_json(config))``, or any
           partial dict containing ExtractionConfig field values.

    Returns:
        A fully typed ExtractionConfig instance.

    """
    accepted = _known_fields(ExtractionConfig)
    kwargs: dict[str, Any] = {}
    for key, value in d.items():
        # Skip fields unknown to the installed xberg version.
        if accepted and key not in accepted:
            continue
        if isinstance(value, dict) and key in _TOP_LEVEL_CONFIGS:
            kwargs[key] = _reconstruct(_TOP_LEVEL_CONFIGS[key], value)
        else:
            kwargs[key] = value
    return ExtractionConfig(**kwargs)

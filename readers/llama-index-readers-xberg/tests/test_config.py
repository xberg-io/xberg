"""Tests for ExtractionConfig dict <-> object reconstruction."""

import json

from xberg import (
    ExtractionConfig,
    HierarchyConfig,
    ImagePreprocessingConfig,
    OcrConfig,
    PageConfig,
    PdfConfig,
    TesseractConfig,
    config_to_json,
)
from llama_index.readers.xberg._config import dict_to_config

# --- dict_to_config reconstruction ---


def test_empty_dict_returns_default_config() -> None:
    config = dict_to_config({})
    assert isinstance(config, ExtractionConfig)


def test_flat_fields_reconstructed() -> None:
    config = dict_to_config({"force_ocr": True, "output_format": "markdown"})
    assert config.force_ocr is True
    assert config.output_format == "markdown"


def test_nested_ocr_config_reconstructed() -> None:
    config = dict_to_config(
        {
            "ocr": {"backend": "paddleocr", "language": "deu"},
        }
    )
    assert isinstance(config.ocr, OcrConfig)
    assert config.ocr.backend == "paddleocr"
    assert config.ocr.language == "deu"


def test_unknown_fields_silently_ignored() -> None:
    config = dict_to_config(
        {
            "output_format": "markdown",
            "some_future_field": "value_from_newer_xberg",
            "ocr": {
                "backend": "tesseract",
                "future_ocr_field": 42,
            },
        }
    )
    assert isinstance(config, ExtractionConfig)
    assert config.output_format == "markdown"
    assert config.ocr.backend == "tesseract"


# --- Nested reconstruction ---


def test_pdf_with_hierarchy_config() -> None:
    config = dict_to_config(
        {
            "pdf_options": {
                "extract_images": True,
                "hierarchy": {"enabled": True, "k_clusters": 4},
            },
        }
    )
    assert isinstance(config.pdf_options, PdfConfig)
    assert isinstance(config.pdf_options.hierarchy, HierarchyConfig)
    assert config.pdf_options.hierarchy.k_clusters == 4


def test_ocr_with_tesseract_config() -> None:
    config = dict_to_config(
        {
            "ocr": {
                "backend": "tesseract",
                "tesseract_config": {
                    "psm": 6,
                    "oem": 1,
                },
            },
        }
    )
    assert isinstance(config.ocr.tesseract_config, TesseractConfig)
    assert config.ocr.tesseract_config.psm == 6


def test_page_config_reconstructed() -> None:
    config = dict_to_config(
        {
            "pages": {"extract_pages": True, "insert_page_markers": True},
        }
    )
    assert isinstance(config.pages, PageConfig)
    assert config.pages.extract_pages is True


# --- Round-trip via config_to_json ---


def test_config_round_trip() -> None:
    original = ExtractionConfig(
        output_format="markdown",
        force_ocr=True,
        ocr=OcrConfig(backend="tesseract", language="deu"),
        pages=PageConfig(extract_pages=True),
    )
    serialized = json.loads(config_to_json(original))
    reconstructed = dict_to_config(serialized)

    assert reconstructed.output_format == "markdown"
    assert reconstructed.force_ocr is True
    assert reconstructed.ocr.backend == "tesseract"
    assert reconstructed.ocr.language == "deu"
    assert reconstructed.pages.extract_pages is True


def test_default_config_round_trip() -> None:
    original = ExtractionConfig()
    serialized = json.loads(config_to_json(original))
    reconstructed = dict_to_config(serialized)
    assert isinstance(reconstructed, ExtractionConfig)


def test_tesseract_with_preprocessing_reconstructed() -> None:
    config = dict_to_config(
        {
            "ocr": {
                "backend": "tesseract",
                "tesseract_config": {
                    "psm": 6,
                    "preprocessing": {
                        "target_dpi": 600,
                        "deskew": False,
                        "denoise": True,
                    },
                },
            },
        }
    )
    assert isinstance(config.ocr.tesseract_config, TesseractConfig)
    assert isinstance(config.ocr.tesseract_config.preprocessing, ImagePreprocessingConfig)
    assert config.ocr.tesseract_config.preprocessing.target_dpi == 600
    assert config.ocr.tesseract_config.preprocessing.deskew is False
    assert config.ocr.tesseract_config.preprocessing.denoise is True


def test_none_sub_config_stays_none() -> None:
    config = dict_to_config({"ocr": None, "pages": None, "pdf_options": None})
    assert config.ocr is None
    assert config.pages is None
    assert config.pdf_options is None


def test_deeply_nested_round_trip() -> None:
    original = ExtractionConfig(
        ocr=OcrConfig(
            backend="tesseract",
            language="deu",
            tesseract_config=TesseractConfig(
                psm=6,
                oem=1,
                preprocessing=ImagePreprocessingConfig(
                    target_dpi=600,
                    deskew=False,
                    denoise=True,
                    binarization_method="sauvola",
                ),
            ),
        ),
    )
    serialized = json.loads(config_to_json(original))
    reconstructed = dict_to_config(serialized)

    assert reconstructed.ocr.backend == "tesseract"
    assert reconstructed.ocr.language == "deu"
    assert reconstructed.ocr.tesseract_config.psm == 6
    assert reconstructed.ocr.tesseract_config.oem == 1
    assert reconstructed.ocr.tesseract_config.preprocessing.target_dpi == 600
    assert reconstructed.ocr.tesseract_config.preprocessing.deskew is False
    assert reconstructed.ocr.tesseract_config.preprocessing.denoise is True
    assert reconstructed.ocr.tesseract_config.preprocessing.binarization_method == "sauvola"

from __future__ import annotations

import tempfile
from pathlib import Path
from typing import Any
from unittest.mock import patch

import pytest

from kreuzberg._config import (
    _build_ocr_config_from_cli,
    _configure_gmft,
    _configure_ocr_backend,
    _merge_cli_args,
    _merge_file_config,
    build_extraction_config,
    build_extraction_config_from_dict,
    discover_and_load_config,
    discover_config,
    find_config_file,
    find_default_config,
    load_config_from_file,
    load_config_from_path,
    load_default_config,
    merge_configs,
    parse_ocr_backend_config,
)
from kreuzberg._types import (
    EasyOCRConfig,
    ExtractionConfig,
    GMFTConfig,
    HTMLToMarkdownConfig,
    PaddleOCRConfig,
    PSMMode,
    TesseractConfig,
)
from kreuzberg.exceptions import ValidationError


def test_config_file_loading_load_kreuzberg_toml(tmp_path: Path) -> None:
    config_file = tmp_path / "kreuzberg.toml"
    config_file.write_text("""
force_ocr = true
chunk_content = false
extract_tables = true
max_chars = 1000
""")

    result = load_config_from_file(config_file)
    assert result == {
        "force_ocr": True,
        "chunk_content": False,
        "extract_tables": True,
        "max_chars": 1000,
    }


def test_config_file_loading_load_pyproject_toml(tmp_path: Path) -> None:
    config_file = tmp_path / "pyproject.toml"
    config_file.write_text("""
[build-system]
requires = ["hatchling"]

[tool.kreuzberg]
force_ocr = true
ocr_backend = "tesseract"
""")

    result = load_config_from_file(config_file)
    assert result == {"force_ocr": True, "ocr_backend": "tesseract"}


def test_config_file_loading_load_pyproject_toml_no_kreuzberg_section(tmp_path: Path) -> None:
    config_file = tmp_path / "pyproject.toml"
    config_file.write_text("""
[build-system]
requires = ["hatchling"]
""")

    result = load_config_from_file(config_file)
    assert result == {}


def test_config_file_loading_load_missing_file(tmp_path: Path) -> None:
    config_file = tmp_path / "nonexistent.toml"

    with pytest.raises(ValidationError, match="Configuration file not found"):
        load_config_from_file(config_file)


def test_config_file_loading_load_invalid_toml(tmp_path: Path) -> None:
    config_file = tmp_path / "invalid.toml"
    config_file.write_text("invalid [ toml")

    with pytest.raises(ValidationError, match="Invalid TOML"):
        load_config_from_file(config_file)


def test_config_file_loading_load_nested_config(tmp_path: Path) -> None:
    config_path = tmp_path / "kreuzberg.toml"
    config_content = """
ocr_backend = "tesseract"
extract_tables = true

[tesseract]
language = "eng+fra"
psm = 6
config = "--oem 1"

[gmft]
verbosity = 2
detector_base_threshold = 0.7

[html_to_markdown]
heading_style = "atx"
wrap = true
wrap_width = 100
"""
    config_path.write_text(config_content)

    result = load_config_from_file(config_path)

    assert result["ocr_backend"] == "tesseract"
    assert result["extract_tables"] is True
    assert result["tesseract"]["language"] == "eng+fra"
    assert result["tesseract"]["psm"] == 6
    assert result["gmft"]["verbosity"] == 2
    assert result["html_to_markdown"]["heading_style"] == "atx"
    assert result["html_to_markdown"]["wrap"] is True
    assert result["html_to_markdown"]["wrap_width"] == 100


def test_config_discovery_find_kreuzberg_toml(tmp_path: Path) -> None:
    config_file = tmp_path / "kreuzberg.toml"
    config_file.write_text("force_ocr = true")

    result = find_config_file(tmp_path)
    assert result == config_file


def test_config_discovery_find_pyproject_toml_with_kreuzberg_section(tmp_path: Path) -> None:
    config_file = tmp_path / "pyproject.toml"
    config_file.write_text("""
[tool.kreuzberg]
force_ocr = true
""")

    result = find_config_file(tmp_path)
    assert result == config_file


def test_config_discovery_find_pyproject_toml_without_kreuzberg_section(tmp_path: Path) -> None:
    config_file = tmp_path / "pyproject.toml"
    config_file.write_text("""
[build-system]
requires = ["hatchling"]
""")

    result = find_config_file(tmp_path)
    assert result is None


def test_config_discovery_find_config_prefers_kreuzberg_toml(tmp_path: Path) -> None:
    kreuzberg_file = tmp_path / "kreuzberg.toml"
    pyproject_file = tmp_path / "pyproject.toml"

    kreuzberg_file.write_text("force_ocr = true")
    pyproject_file.write_text("""
[tool.kreuzberg]
force_ocr = false
""")

    result = find_config_file(tmp_path)
    assert result == kreuzberg_file


def test_config_discovery_find_config_searches_up_tree(tmp_path: Path) -> None:
    config_file = tmp_path / "kreuzberg.toml"
    config_file.write_text("force_ocr = true")

    subdir = tmp_path / "subdir" / "deep"
    subdir.mkdir(parents=True)

    result = find_config_file(subdir)
    assert result == config_file


def test_config_discovery_find_config_no_file_found(tmp_path: Path) -> None:
    result = find_config_file(tmp_path)
    assert result is None


def test_config_discovery_find_config_stops_at_git_root(tmp_path: Path) -> None:
    git_dir = tmp_path / ".git"
    git_dir.mkdir()

    subdir = tmp_path / "subdir"
    subdir.mkdir()

    result = find_config_file(subdir)
    assert result is None


def test_config_discovery_find_config_default_start_path() -> None:
    with patch("pathlib.Path.cwd") as mock_cwd:
        mock_cwd.return_value = Path("/fake/path")

        with patch.object(Path, "exists", return_value=False):
            result = find_config_file()
            assert result is None


def test_config_discovery_find_config_invalid_pyproject_toml(tmp_path: Path) -> None:
    config_file = tmp_path / "pyproject.toml"
    config_file.write_text("invalid [ toml")

    with pytest.raises(ValidationError) as exc_info:
        find_config_file(tmp_path)
    assert "Invalid TOML in pyproject.toml" in str(exc_info.value)


def test_config_parsing_merge_configs_simple() -> None:
    base = {"force_ocr": False, "max_chars": 1000}
    override = {"force_ocr": True, "chunk_content": True}

    result = merge_configs(base, override)
    assert result == {
        "force_ocr": True,
        "max_chars": 1000,
        "chunk_content": True,
    }


def test_config_parsing_merge_configs_nested() -> None:
    base = {"tesseract": {"lang": "eng", "psm": 3}}
    override = {"tesseract": {"psm": 6, "oem": 1}}

    result = merge_configs(base, override)
    assert result == {"tesseract": {"lang": "eng", "psm": 6, "oem": 1}}


def test_config_parsing_merge_configs_empty_base() -> None:
    base: dict[str, Any] = {}
    override = {"force_ocr": True, "max_chars": 1000}

    result = merge_configs(base, override)
    assert result == override


def test_config_parsing_merge_configs_empty_override() -> None:
    base = {"force_ocr": True, "max_chars": 1000}
    override: dict[str, Any] = {}

    result = merge_configs(base, override)
    assert result == base


def test_config_parsing_merge_configs_deep_nesting() -> None:
    base = {"level1": {"level2": {"level3": {"value1": "base", "value2": "keep"}}}}
    override = {"level1": {"level2": {"level3": {"value1": "override", "value3": "new"}}}}

    result = merge_configs(base, override)
    assert result == {"level1": {"level2": {"level3": {"value1": "override", "value2": "keep", "value3": "new"}}}}


def test_config_parsing_parse_tesseract_config() -> None:
    config_dict = {"tesseract": {"language": "eng", "psm": 6}}

    result = parse_ocr_backend_config(config_dict, "tesseract")
    assert isinstance(result, TesseractConfig)
    assert result.language == "eng"
    assert result.psm.value == 6


def test_config_parsing_parse_tesseract_config_with_all_fields() -> None:
    config_dict = {
        "tesseract": {
            "language": "eng+fra",
            "psm": 6,
            "tessedit_char_whitelist": "0123456789",
        }
    }

    result = parse_ocr_backend_config(config_dict, "tesseract")
    assert isinstance(result, TesseractConfig)
    assert result.language == "eng+fra"
    assert result.psm == PSMMode.SINGLE_BLOCK
    assert result.tessedit_char_whitelist == "0123456789"


def test_config_parsing_parse_easyocr_config() -> None:
    config_dict = {
        "easyocr": {
            "language": ["en", "fr"],
        }
    }

    result = parse_ocr_backend_config(config_dict, "easyocr")
    assert isinstance(result, EasyOCRConfig)
    assert result.language == ["en", "fr"]


def test_config_parsing_parse_paddleocr_config() -> None:
    config_dict = {
        "paddleocr": {
            "language": "en",
            "use_angle_cls": True,
            "use_gpu": False,
        }
    }

    result = parse_ocr_backend_config(config_dict, "paddleocr")
    assert isinstance(result, PaddleOCRConfig)
    assert result.language == "en"
    assert result.use_angle_cls is True
    assert result.use_gpu is False


def test_config_parsing_parse_ocr_config_missing_backend() -> None:
    config_dict = {"other_setting": "value"}

    result = parse_ocr_backend_config(config_dict, "tesseract")
    assert result is None


def test_config_parsing_parse_ocr_config_invalid_type() -> None:
    config_dict = {"tesseract": "invalid"}

    with pytest.raises(ValidationError) as exc_info:
        parse_ocr_backend_config(config_dict, "tesseract")

    assert "expected dict, got str" in str(exc_info.value)
    assert exc_info.value.context["backend"] == "tesseract"


def test_config_parsing_parse_ocr_config_invalid_backend() -> None:
    config_dict = {"tesseract": {"language": "eng"}}

    result = parse_ocr_backend_config(config_dict, "invalid_backend")  # type: ignore[arg-type]
    assert result is None


def test_merge_configs_deep_merge() -> None:
    base = {
        "force_ocr": False,
        "tesseract": {"language": "eng", "psm": 6},
        "nested": {"deep": {"value": "base"}},
    }
    override = {
        "force_ocr": True,
        "tesseract": {"psm": 3},
        "nested": {"deep": {"value": "override", "new": "added"}},
        "new_key": "new_value",
    }

    result = merge_configs(base, override)

    assert result["force_ocr"] is True
    assert result["tesseract"]["language"] == "eng"
    assert result["tesseract"]["psm"] == 3
    assert result["nested"]["deep"]["value"] == "override"
    assert result["nested"]["deep"]["new"] == "added"
    assert result["new_key"] == "new_value"


def test_build_extraction_config_from_dict_with_all_options() -> None:
    config_dict = {
        "force_ocr": True,
        "chunk_content": True,
        "max_chars": 500,
        "max_overlap": 50,
        "extract_tables": True,
        "extract_entities": True,
        "extract_keywords": True,
        "keyword_count": 15,
        "auto_detect_document_type": True,
        "document_type_confidence_threshold": 0.8,
        "auto_detect_language": True,
        "ocr_backend": "tesseract",
        "tesseract": {"language": "fra", "psm": 6},
        "gmft": {"verbosity": 2, "formatter_base_threshold": 0.4},
    }

    config = build_extraction_config_from_dict(config_dict)

    assert config.force_ocr is True
    assert config.chunk_content is True
    assert config.max_chars == 500
    assert config.max_overlap == 50
    assert config.extract_tables is True
    assert config.extract_entities is True
    assert config.extract_keywords is True
    assert config.keyword_count == 15
    assert config.auto_detect_document_type is True
    assert config.document_type_confidence_threshold == 0.8
    assert config.auto_detect_language is True
    assert config.ocr_backend == "tesseract"


def test_build_extraction_config_from_dict_invalid_ocr_backend() -> None:
    config_dict = {
        "ocr_backend": "invalid_backend",
    }

    with pytest.raises(ValidationError, match="Invalid OCR backend"):
        build_extraction_config_from_dict(config_dict)


def test_find_config_file_with_pyproject_toml(tmp_path: Path) -> None:
    pyproject_file = tmp_path / "pyproject.toml"
    pyproject_file.write_text("""
[tool.kreuzberg]
force_ocr = true
chunk_content = false
""")

    result = find_config_file(tmp_path)
    assert result == pyproject_file


def test_find_config_file_no_config(tmp_path: Path) -> None:
    result = find_config_file(tmp_path)
    assert result is None


def test_load_config_from_path_string() -> None:
    with tempfile.NamedTemporaryFile(mode="w", suffix=".toml", delete=False) as f:
        f.write("force_ocr = true\nchunk_content = false\n")
        f.flush()
        temp_path = f.name

    try:
        config = load_config_from_path(temp_path)
        assert config.force_ocr is True
        assert config.chunk_content is False
    finally:
        Path(temp_path).unlink()


def test_discover_and_load_config_with_default(tmp_path: Path) -> None:
    with pytest.raises(ValidationError, match="No configuration file found"):
        discover_and_load_config(str(tmp_path))


def test_discover_config_returns_none(tmp_path: Path) -> None:
    result = discover_config(str(tmp_path))
    assert result is None


def test_build_extraction_config_comprehensive() -> None:
    config_dict = {
        "force_ocr": True,
        "chunk_content": False,
        "extract_tables": True,
        "max_chars": 2000,
        "auto_detect_document_type": True,
    }

    config = build_extraction_config_from_dict(config_dict)

    assert config.force_ocr is True
    assert config.chunk_content is False
    assert config.extract_tables is True
    assert config.max_chars == 2000
    assert config.auto_detect_document_type is True


def test_build_from_dict_with_ocr_config() -> None:
    config_dict = {
        "force_ocr": True,
        "ocr_backend": "tesseract",
        "tesseract": {"language": "eng", "psm": 6},
    }

    result = build_extraction_config_from_dict(config_dict)
    assert result.ocr_backend == "tesseract"
    assert isinstance(result.ocr_config, TesseractConfig)
    assert result.ocr_config.language == "eng"


def test_build_from_dict_ocr_backend_none() -> None:
    config_dict = {"ocr_backend": "none"}

    result = build_extraction_config_from_dict(config_dict)
    assert result.ocr_backend is None


def test_merge_cli_args_with_boolean_flags() -> None:
    base_config = {"force_ocr": False, "chunk_content": True}
    cli_args = {"force_ocr": True, "extract_tables": True}

    _merge_cli_args(base_config, cli_args)

    assert base_config["force_ocr"] is True
    assert base_config["chunk_content"] is True
    assert base_config["extract_tables"] is True


def test_merge_file_config_with_missing_keys() -> None:
    base_config = {"force_ocr": False, "chunk_content": True}
    file_config = {"chunk_content": False}

    _merge_file_config(base_config, file_config)

    assert base_config["force_ocr"] is False
    assert base_config["chunk_content"] is False


def test_build_from_dict_with_gmft_config() -> None:
    config_dict = {
        "extract_tables": True,
        "gmft": {
            "verbosity": 2,
            "detector_base_threshold": 0.7,
        },
    }

    result = build_extraction_config_from_dict(config_dict)
    assert result.extract_tables is True
    assert isinstance(result.gmft_config, GMFTConfig)
    assert result.gmft_config.verbosity == 2


def test_build_from_dict_with_html_to_markdown_config() -> None:
    config_dict = {
        "force_ocr": False,
        "html_to_markdown": {
            "heading_style": "atx",
            "strong_em_symbol": "_",
            "wrap": True,
            "wrap_width": 120,
            "preprocessing_preset": "standard",
        },
    }

    result = build_extraction_config_from_dict(config_dict)
    assert result.force_ocr is False
    assert isinstance(result.html_to_markdown_config, HTMLToMarkdownConfig)
    assert result.html_to_markdown_config.heading_style == "atx"
    assert result.html_to_markdown_config.strong_em_symbol == "_"
    assert result.html_to_markdown_config.wrap is True
    assert result.html_to_markdown_config.wrap_width == 120
    assert result.html_to_markdown_config.preprocessing_preset == "standard"


def test_build_extraction_config_file_cli_merge() -> None:
    file_config = {
        "max_chars": 1000,
        "tesseract": {"language": "eng"},
    }
    cli_args = {
        "force_ocr": True,
        "ocr_backend": "tesseract",
    }

    result = build_extraction_config(file_config, cli_args)
    assert result.force_ocr is True
    assert result.max_chars == 1000
    assert result.ocr_backend == "tesseract"


def test_build_extraction_config_complete() -> None:
    config_dict = {
        "force_ocr": True,
        "chunk_content": True,
        "extract_tables": True,
        "max_chars": 4000,
        "max_overlap": 200,
        "language_detection_threshold": 0.8,
        "auto_detect_language": True,
        "document_classification_mode": "text",
        "document_type_confidence_threshold": 0.7,
        "ocr_backend": "tesseract",
        "tesseract": {"language": "eng+fra"},
        "gmft": {"verbosity": 1},
    }

    result = build_extraction_config_from_dict(config_dict)
    assert result.force_ocr is True
    assert result.chunk_content is True
    assert result.extract_tables is True
    assert result.max_chars == 4000
    assert result.max_overlap == 200
    assert result.auto_detect_language is True
    assert result.ocr_backend == "tesseract"


def test_high_level_api_load_config_from_path(tmp_path: Path) -> None:
    config_file = tmp_path / "kreuzberg.toml"
    config_file.write_text("""
force_ocr = true
chunk_content = false
""")

    result = load_config_from_path(config_file)
    assert isinstance(result, ExtractionConfig)
    assert result.force_ocr is True
    assert result.chunk_content is False


def test_high_level_api_discover_and_load_config(tmp_path: Path) -> None:
    config_file = tmp_path / "kreuzberg.toml"
    config_file.write_text("force_ocr = true")

    result = discover_and_load_config(tmp_path)
    assert isinstance(result, ExtractionConfig)
    assert result.force_ocr is True


def test_high_level_api_discover_and_load_config_not_found(tmp_path: Path) -> None:
    with pytest.raises(ValidationError, match="No configuration file found"):
        discover_and_load_config(tmp_path)


def test_high_level_api_discover_and_load_config_empty(tmp_path: Path) -> None:
    config_file = tmp_path / "pyproject.toml"
    config_file.write_text("""
[build-system]
requires = ["hatchling"]
""")

    with pytest.raises(ValidationError, match="No configuration file found"):
        discover_and_load_config(tmp_path)


def test_high_level_api_discover_config_success(tmp_path: Path) -> None:
    config_file = tmp_path / "kreuzberg.toml"
    config_file.write_text("force_ocr = true")

    result = discover_config(tmp_path)
    assert isinstance(result, ExtractionConfig)
    assert result.force_ocr is True


def test_high_level_api_discover_config_not_found(tmp_path: Path) -> None:
    result = discover_config(tmp_path)
    assert result is None


def test_high_level_api_discover_config_invalid(tmp_path: Path) -> None:
    config_file = tmp_path / "kreuzberg.toml"
    config_file.write_text("invalid [ toml")

    with pytest.raises(ValidationError) as exc_info:
        discover_config(tmp_path)
    assert "Invalid TOML" in str(exc_info.value)


def test_high_level_api_load_default_config(tmp_path: Path) -> None:
    config_file = tmp_path / "kreuzberg.toml"
    config_file.write_text("force_ocr = true")

    with patch("kreuzberg._config.find_config_file") as mock_find:
        mock_find.return_value = config_file

        result = load_default_config()

        assert isinstance(result, ExtractionConfig)
        assert result.force_ocr is True


def test_config_merging_merge_file_config() -> None:
    config_dict: dict[str, Any] = {"existing": "value"}
    file_config = {
        "force_ocr": True,
        "max_chars": 1000,
        "chunk_content": False,
        "unknown_field": "ignored",
    }

    _merge_file_config(config_dict, file_config)

    assert config_dict["force_ocr"] is True
    assert config_dict["chunk_content"] is False
    assert "unknown_field" not in config_dict
    assert config_dict["existing"] == "value"


def test_config_merging_merge_file_config_empty() -> None:
    config_dict: dict[str, Any] = {"existing": "value"}

    _merge_file_config(config_dict, {})

    assert config_dict == {"existing": "value"}


def test_config_merging_merge_cli_args() -> None:
    config_dict: dict[str, Any] = {}
    cli_args: dict[str, Any] = {
        "force_ocr": True,
        "chunk_content": None,
        "max_chars": 2000,
        "extract_tables": True,
        "ocr_backend": "tesseract",
    }

    _merge_cli_args(config_dict, cli_args)

    assert config_dict["force_ocr"] is True
    assert "chunk_content" not in config_dict
    assert config_dict["max_chars"] == 2000
    assert config_dict["extract_tables"] is True
    assert config_dict["ocr_backend"] == "tesseract"


def test_ocr_config_build_from_cli_tesseract() -> None:
    cli_args = {"tesseract_config": {"language": "eng", "psm": 6}}

    result = _build_ocr_config_from_cli("tesseract", cli_args)

    assert isinstance(result, TesseractConfig)
    assert result.language == "eng"
    assert result.psm == 6  # type: ignore[comparison-overlap]  # PSM value is not converted to enum in _build_ocr_config_from_cli


def test_ocr_config_build_from_cli_easyocr() -> None:
    cli_args = {"easyocr_config": {"language": ["en", "fr"]}}

    result = _build_ocr_config_from_cli("easyocr", cli_args)

    assert isinstance(result, EasyOCRConfig)
    assert result.language == ["en", "fr"]


def test_ocr_config_build_from_cli_paddleocr() -> None:
    cli_args = {"paddleocr_config": {"language": "en", "use_gpu": False}}

    result = _build_ocr_config_from_cli("paddleocr", cli_args)

    assert isinstance(result, PaddleOCRConfig)
    assert result.language == "en"
    assert result.use_gpu is False


def test_ocr_config_build_from_cli_no_config() -> None:
    cli_args: dict[str, Any] = {}

    result = _build_ocr_config_from_cli("tesseract", cli_args)

    assert result is None


def test_ocr_config_configure_backend() -> None:
    config_dict = {"ocr_backend": "tesseract"}
    file_config: dict[str, Any] = {"tesseract": {"language": "eng", "psm": 3}}
    cli_args: dict[str, Any] = {}

    _configure_ocr_backend(config_dict, file_config, cli_args)

    assert "ocr_config" in config_dict
    assert isinstance(config_dict["ocr_config"], TesseractConfig)


def test_ocr_config_configure_backend_cli_priority() -> None:
    config_dict = {"ocr_backend": "tesseract"}
    file_config: dict[str, Any] = {"tesseract": {"language": "eng"}}
    cli_args: dict[str, Any] = {"tesseract_config": {"language": "fra"}}

    _configure_ocr_backend(config_dict, file_config, cli_args)

    assert isinstance(config_dict["ocr_config"], TesseractConfig)
    assert config_dict["ocr_config"].language == "fra"


def test_ocr_config_configure_backend_none() -> None:
    config_dict = {"ocr_backend": None}
    file_config: dict[str, Any] = {}
    cli_args: dict[str, Any] = {}

    _configure_ocr_backend(config_dict, file_config, cli_args)

    assert "ocr_config" not in config_dict


def test_gmft_config_configure() -> None:
    config_dict = {"extract_tables": True}
    file_config: dict[str, Any] = {"gmft": {"verbosity": 2, "detector_base_threshold": 0.7}}
    cli_args: dict[str, Any] = {}

    _configure_gmft(config_dict, file_config, cli_args)

    assert "gmft_config" in config_dict
    assert isinstance(config_dict["gmft_config"], GMFTConfig)
    assert config_dict["gmft_config"].verbosity == 2


def test_gmft_config_configure_cli_priority() -> None:
    config_dict = {"extract_tables": True}
    file_config: dict[str, Any] = {"gmft": {"detector_base_threshold": 0.5}}
    cli_args: dict[str, Any] = {"gmft_config": {"detector_base_threshold": 0.9}}

    _configure_gmft(config_dict, file_config, cli_args)

    assert isinstance(config_dict["gmft_config"], GMFTConfig)
    assert config_dict["gmft_config"].detector_base_threshold == 0.9


def test_gmft_config_configure_no_extract_tables() -> None:
    config_dict = {"extract_tables": False}
    file_config: dict[str, Any] = {"gmft": {"verbosity": 2}}
    cli_args: dict[str, Any] = {}

    _configure_gmft(config_dict, file_config, cli_args)

    assert "gmft_config" not in config_dict


def test_config_integration_build_extraction_config() -> None:
    file_config = {
        "force_ocr": False,
        "max_chars": 1000,
        "ocr_backend": "tesseract",
        "tesseract": {"language": "eng", "psm": 3},
        "gmft": {"verbosity": 1},
    }
    cli_args = {
        "force_ocr": True,
        "extract_tables": True,
        "tesseract_config": {"language": "fra"},
    }

    result = build_extraction_config(file_config, cli_args)

    assert result.force_ocr is True
    assert result.max_chars == 1000
    assert result.ocr_backend == "tesseract"
    assert result.extract_tables is True
    assert result.ocr_config is not None
    assert isinstance(result.ocr_config, TesseractConfig)
    assert result.ocr_config.language == "fra"


def test_config_deprecated_find_default_config() -> None:
    with patch("kreuzberg._config.find_config_file") as mock_find:
        mock_find.return_value = Path("/fake/config.toml")

        result = find_default_config()

        assert result == Path("/fake/config.toml")
        mock_find.assert_called_once_with()


def test_config_integration_real_world_pyproject_toml(tmp_path: Path) -> None:
    config_file = tmp_path / "pyproject.toml"
    config_file.write_text("""
[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[project]
name = "my-project"
version = "1.0.0"

[tool.kreuzberg]
force_ocr = false
chunk_content = true
extract_tables = true
max_chars = 4000
max_overlap = 200
ocr_backend = "tesseract"
auto_detect_language = true

[tool.kreuzberg.tesseract]
language = "eng+fra"
psm = 6

[tool.kreuzberg.gmft]
verbosity = 1
""")

    config = load_config_from_path(config_file)
    assert config.force_ocr is False
    assert config.chunk_content is True
    assert config.extract_tables is True
    assert config.max_chars == 4000
    assert config.max_overlap == 200
    assert config.ocr_backend == "tesseract"
    assert config.auto_detect_language is True

    assert isinstance(config.ocr_config, TesseractConfig)
    assert config.ocr_config.language == "eng+fra"
    assert config.ocr_config.psm.value == 6


def test_config_integration_discovery_with_cwd() -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tmp_path = Path(tmp_dir)
        config_file = tmp_path / "kreuzberg.toml"
        config_file.write_text("force_ocr = true")

        import os

        original_cwd = Path.cwd()
        try:
            os.chdir(tmp_dir)
            result = discover_config()
            assert result is not None
            assert result.force_ocr is True
        finally:
            os.chdir(str(original_cwd))


def test_config_integration_priority_kreuzberg_over_pyproject(tmp_path: Path) -> None:
    kreuzberg_file = tmp_path / "kreuzberg.toml"
    kreuzberg_file.write_text("force_ocr = true")

    pyproject_file = tmp_path / "pyproject.toml"
    pyproject_file.write_text("""
[tool.kreuzberg]
force_ocr = false
""")

    config = discover_and_load_config(tmp_path)
    assert config.force_ocr is True


def test_config_integration_complex_config_merging() -> None:
    file_config = {
        "force_ocr": False,
        "chunk_content": True,
        "max_chars": 1000,
        "ocr_backend": "tesseract",
        "tesseract": {"language": "eng", "psm": 3},
        "gmft": {"verbosity": 1, "detector_base_threshold": 0.5},
    }

    cli_args = {
        "force_ocr": True,
        "extract_tables": True,
        "tesseract_config": {"language": "eng+fra"},
        "gmft_config": {"verbosity": 2},
    }

    result = build_extraction_config(file_config, cli_args)

    assert result.force_ocr is True
    assert result.extract_tables is True

    assert result.chunk_content is True
    assert result.max_chars == 1000

    assert isinstance(result.ocr_config, TesseractConfig)
    assert result.ocr_config.language == "eng+fra"
    assert result.ocr_config.psm.value == 3

    assert isinstance(result.gmft_config, GMFTConfig)
    assert result.gmft_config.verbosity == 2


def test_config_file_loading_comprehensive_arbitrary_toml(tmp_path: Path) -> None:
    config_file = tmp_path / "custom.toml"
    config_file.write_text("""
force_ocr = true
chunk_content = false
ocr_backend = "tesseract"
""")

    result: dict[str, Any] = load_config_from_file(config_file)
    assert result == {
        "force_ocr": True,
        "chunk_content": False,
        "ocr_backend": "tesseract",
    }


def test_config_file_loading_comprehensive_pyproject_tool_no_kreuzberg(tmp_path: Path) -> None:
    config_file = tmp_path / "pyproject.toml"
    config_file.write_text("""
[tool.black]
line-length = 88

[tool.mypy]
strict = true
""")

    result = load_config_from_file(config_file)
    assert result == {}


def test_config_file_loading_comprehensive_empty_file(tmp_path: Path) -> None:
    config_file = tmp_path / "kreuzberg.toml"
    config_file.write_text("")

    result = load_config_from_file(config_file)
    assert result == {}


def test_config_file_loading_comprehensive_with_comments(tmp_path: Path) -> None:
    config_file = tmp_path / "kreuzberg.toml"
    config_file.write_text("""
# Main configuration
force_ocr = true  # Force OCR processing

# OCR settings
ocr_backend = "tesseract"

[tesseract]
# Language settings
language = "eng+fra"  # English and French
psm = 6
tessedit_char_whitelist = "0123456789"
""")

    result = load_config_from_file(config_file)
    assert result["force_ocr"] is True
    assert result["ocr_backend"] == "tesseract"
    assert result["tesseract"]["language"] == "eng+fra"
    assert result["tesseract"]["tessedit_char_whitelist"] == "0123456789"


def test_config_discovery_comprehensive_handles_permission_error(tmp_path: Path) -> None:
    pyproject_file = tmp_path / "pyproject.toml"
    pyproject_file.write_text("""
[tool.kreuzberg]
force_ocr = true
""")

    with patch("pathlib.Path.open", side_effect=PermissionError("No access")):
        with pytest.raises(ValidationError) as exc_info:
            find_config_file(tmp_path)
        assert "Failed to read pyproject.toml" in str(exc_info.value)
        assert "No access" in str(exc_info.value.context["error"])


def test_config_discovery_comprehensive_handles_generic_exception(tmp_path: Path) -> None:
    pyproject_file = tmp_path / "pyproject.toml"
    pyproject_file.write_text("""
[tool.kreuzberg]
force_ocr = true
""")

    with patch("kreuzberg._config.tomllib.load", side_effect=RuntimeError("Unexpected error")):
        with pytest.raises(RuntimeError, match="Unexpected error"):
            find_config_file(tmp_path)


def test_config_discovery_comprehensive_find_at_root_directory() -> None:
    with patch("pathlib.Path.parent", new_callable=lambda: property(lambda self: self)):
        root_path = Path("/")
        result = find_config_file(root_path)
        assert result is None


def test_config_parsing_comprehensive_tesseract_psm_enum_conversion() -> None:
    config_dict = {
        "tesseract": {
            "language": "eng",
            "psm": 0,
        }
    }

    result = parse_ocr_backend_config(config_dict, "tesseract")
    assert isinstance(result, TesseractConfig)
    assert result.psm == PSMMode.OSD_ONLY


def test_config_parsing_comprehensive_tesseract_all_psm_modes() -> None:
    for psm_value in range(11):
        config_dict = {"tesseract": {"psm": psm_value}}
        result = parse_ocr_backend_config(config_dict, "tesseract")
        assert isinstance(result, TesseractConfig)
        assert result.psm.value == psm_value


def test_config_parsing_comprehensive_tesseract_boolean_fields() -> None:
    config_dict = {
        "tesseract": {
            "language": "eng",
            "tessedit_enable_dict_correction": False,
            "tessedit_use_primary_params_model": False,
            "textord_space_size_is_variable": False,
        }
    }

    result = parse_ocr_backend_config(config_dict, "tesseract")
    assert isinstance(result, TesseractConfig)
    assert result.tessedit_enable_dict_correction is False
    assert result.tessedit_use_primary_params_model is False
    assert result.textord_space_size_is_variable is False


def test_config_parsing_comprehensive_paddleocr_all_fields() -> None:
    config_dict = {
        "paddleocr": {
            "language": "ch",
            "use_gpu": True,
            "use_angle_cls": False,
            "det_db_box_thresh": 0.6,
            "det_db_thresh": 0.4,
            "det_db_unclip_ratio": 2.5,
            "max_text_length": 100,
            "use_space_char": False,
            "drop_score": 0.3,
            "enable_mkldnn": True,
            "gpu_mem": 4000,
            "rec": False,
            "table": False,
            "device": "cuda",
            "gpu_memory_limit": 4.0,
            "fallback_to_cpu": False,
        }
    }

    result = parse_ocr_backend_config(config_dict, "paddleocr")
    assert isinstance(result, PaddleOCRConfig)
    assert result.language == "ch"
    assert result.use_gpu is True
    assert result.use_angle_cls is False
    assert result.det_db_box_thresh == 0.6
    assert result.det_db_thresh == 0.4
    assert result.det_db_unclip_ratio == 2.5
    assert result.max_text_length == 100
    assert result.use_space_char is False
    assert result.drop_score == 0.3
    assert result.enable_mkldnn is True
    assert result.gpu_mem == 4000
    assert result.rec is False
    assert result.table is False
    assert result.device == "cuda"
    assert result.gpu_memory_limit == 4.0
    assert result.fallback_to_cpu is False


def test_config_parsing_comprehensive_easyocr_all_fields() -> None:
    config_dict = {
        "easyocr": {
            "language": ["en", "fr", "de"],
            "add_margin": 0.2,
            "adjust_contrast": 0.7,
            "beam_width": 10,
            "canvas_size": 3000,
            "contrast_ths": 0.2,
            "decoder": "beamsearch",
            "height_ths": 0.6,
            "link_threshold": 0.5,
            "low_text": 0.5,
            "mag_ratio": 1.5,
            "min_size": 20,
            "rotation_info": [0, 90, 180, 270],
            "slope_ths": 0.2,
            "use_gpu": False,
            "device": "cpu",
        }
    }

    result = parse_ocr_backend_config(config_dict, "easyocr")
    assert isinstance(result, EasyOCRConfig)
    assert result.language == ["en", "fr", "de"]
    assert result.add_margin == 0.2
    assert result.adjust_contrast == 0.7
    assert result.beam_width == 10
    assert result.canvas_size == 3000
    assert result.contrast_ths == 0.2
    assert result.decoder == "beamsearch"
    assert result.height_ths == 0.6
    assert result.link_threshold == 0.5
    assert result.low_text == 0.5
    assert result.mag_ratio == 1.5
    assert result.min_size == 20
    assert result.rotation_info == [0, 90, 180, 270]
    assert result.slope_ths == 0.2
    assert result.use_gpu is False
    assert result.device == "cpu"


def test_config_parsing_comprehensive_merge_configs_non_dict_override() -> None:
    base = {"nested": {"key": "value"}}
    override = {"nested": "string_value"}

    result = merge_configs(base, override)
    assert result["nested"] == "string_value"


def test_config_parsing_comprehensive_merge_configs_mixed_types() -> None:
    base = {
        "string": "base",
        "number": 100,
        "bool": False,
        "list": [1, 2, 3],
        "dict": {"inner": "base"},
    }
    override = {
        "string": "override",
        "number": 200,
        "bool": True,
        "list": [4, 5],
        "dict": {"inner": "override", "new": "value"},
    }

    result = merge_configs(base, override)
    assert result["string"] == "override"
    assert result["number"] == 200
    assert result["bool"] is True
    assert result["list"] == [4, 5]
    assert result["dict"] == {"inner": "override", "new": "value"}


def test_extraction_config_builder_comprehensive_all_basic_fields() -> None:
    config_dict = {
        "force_ocr": True,
        "chunk_content": True,
        "extract_tables": True,
        "max_chars": 3000,
        "max_overlap": 300,
        "ocr_backend": "tesseract",
        "extract_entities": True,
        "extract_keywords": True,
        "auto_detect_language": True,
        "enable_quality_processing": False,
        "auto_detect_document_type": False,
        "document_type_confidence_threshold": 0.9,
        "document_classification_mode": "vision",
        "keyword_count": 20,
    }

    result = build_extraction_config_from_dict(config_dict)

    assert result.force_ocr is True
    assert result.chunk_content is True
    assert result.extract_tables is True
    assert result.max_chars == 3000
    assert result.max_overlap == 300
    assert result.ocr_backend == "tesseract"
    assert result.extract_entities is True
    assert result.extract_keywords is True
    assert result.auto_detect_language is True
    assert result.enable_quality_processing is False
    assert result.auto_detect_document_type is False
    assert result.document_type_confidence_threshold == 0.9
    assert result.document_classification_mode == "vision"
    assert result.keyword_count == 20


def test_extraction_config_builder_comprehensive_without_ocr_backend_config() -> None:
    config_dict = {
        "ocr_backend": "tesseract",
    }

    result = build_extraction_config_from_dict(config_dict)
    assert result.ocr_backend == "tesseract"
    assert result.ocr_config is None


def test_extraction_config_builder_comprehensive_gmft_no_extract_tables() -> None:
    config_dict = {
        "extract_tables": False,
        "gmft": {
            "verbosity": 2,
            "detector_base_threshold": 0.8,
        },
    }

    result = build_extraction_config_from_dict(config_dict)
    assert result.extract_tables is False
    assert result.gmft_config is None


def test_extraction_config_builder_comprehensive_extract_tables_no_gmft() -> None:
    config_dict = {
        "extract_tables": True,
    }

    result = build_extraction_config_from_dict(config_dict)
    assert result.extract_tables is True
    assert result.gmft_config is None


def test_extraction_config_builder_comprehensive_invalid_gmft_config() -> None:
    config_dict = {
        "extract_tables": True,
        "gmft": "not_a_dict",
    }

    result = build_extraction_config_from_dict(config_dict)
    assert result.extract_tables is True
    assert result.gmft_config is None


def test_extraction_config_builder_comprehensive_minimal() -> None:
    config_dict: dict[str, Any] = {}

    result = build_extraction_config_from_dict(config_dict)
    assert result.force_ocr is False
    assert result.chunk_content is False
    assert result.ocr_backend == "tesseract"


def test_extraction_config_builder_comprehensive_validation_context() -> None:
    config_dict = {
        "ocr_backend": "invalid_ocr",
    }

    with pytest.raises(ValidationError) as exc_info:
        build_extraction_config_from_dict(config_dict)

    assert exc_info.value.context["provided"] == "invalid_ocr"
    assert "easyocr" in exc_info.value.context["valid"]
    assert "paddleocr" in exc_info.value.context["valid"]
    assert "tesseract" in exc_info.value.context["valid"]


def test_high_level_api_comprehensive_load_default_no_file() -> None:
    with patch("kreuzberg._config.find_config_file", return_value=None):
        result = load_default_config()
        assert result is None


def test_high_level_api_comprehensive_load_default_with_error(tmp_path: Path) -> None:
    config_file = tmp_path / "kreuzberg.toml"
    config_file.write_text("invalid [ toml")

    with patch("kreuzberg._config.find_config_file", return_value=config_file):
        with pytest.raises(ValidationError) as exc_info:
            load_default_config()
        assert "Invalid TOML" in str(exc_info.value)


def test_high_level_api_comprehensive_load_default_empty_dict(tmp_path: Path) -> None:
    config_file = tmp_path / "kreuzberg.toml"
    config_file.write_text("")

    with patch("kreuzberg._config.find_config_file", return_value=config_file):
        result = load_default_config()
        assert result is None


def test_high_level_api_comprehensive_discover_load_string_path(tmp_path: Path) -> None:
    config_file = tmp_path / "kreuzberg.toml"
    config_file.write_text("force_ocr = true")

    result = discover_and_load_config(str(tmp_path))
    assert result.force_ocr is True


def test_high_level_api_comprehensive_discover_load_empty_error(tmp_path: Path) -> None:
    config_file = tmp_path / "kreuzberg.toml"
    config_file.write_text("")

    with pytest.raises(ValidationError, match="contains no Kreuzberg configuration"):
        discover_and_load_config(tmp_path)


def test_high_level_api_comprehensive_try_discover_string_path(tmp_path: Path) -> None:
    config_file = tmp_path / "kreuzberg.toml"
    config_file.write_text("chunk_content = true")

    result = discover_config(str(tmp_path))
    assert result is not None
    assert result.chunk_content is True


def test_legacy_functions_comprehensive_merge_file_config_none_values() -> None:
    config_dict: dict[str, Any] = {}
    file_config = {
        "force_ocr": None,
        "chunk_content": True,
        "max_chars": 0,
    }

    _merge_file_config(config_dict, file_config)

    assert config_dict["force_ocr"] is None
    assert config_dict["chunk_content"] is True
    assert config_dict["max_chars"] == 0


def test_legacy_functions_comprehensive_merge_cli_args_empty_dict() -> None:
    config_dict: dict[str, Any] = {}
    cli_args = {
        "force_ocr": False,
        "chunk_content": None,
        "extract_tables": True,
        "unknown_field": "ignored",
    }

    _merge_cli_args(config_dict, cli_args)

    assert config_dict["force_ocr"] is False
    assert "chunk_content" not in config_dict
    assert config_dict["extract_tables"] is True
    assert "unknown_field" not in config_dict


def test_legacy_functions_comprehensive_build_ocr_config_invalid_backend() -> None:
    cli_args = {"tesseract_config": {"language": "eng"}}

    result = _build_ocr_config_from_cli("invalid_backend", cli_args)
    assert result is None


def test_legacy_functions_comprehensive_configure_ocr_backend_none_string() -> None:
    config_dict = {"ocr_backend": "none"}
    file_config: dict[str, Any] = {}
    cli_args: dict[str, Any] = {}

    _configure_ocr_backend(config_dict, file_config, cli_args)

    assert "ocr_config" not in config_dict


def test_legacy_functions_comprehensive_configure_ocr_backend_no_config() -> None:
    config_dict = {"ocr_backend": "tesseract"}
    file_config: dict[str, Any] = {}
    cli_args: dict[str, Any] = {}

    _configure_ocr_backend(config_dict, file_config, cli_args)

    assert "ocr_config" not in config_dict


def test_legacy_functions_comprehensive_configure_gmft_none_values() -> None:
    config_dict = {"extract_tables": None}
    file_config: dict[str, Any] = {"gmft": {"verbosity": 2}}
    cli_args: dict[str, Any] = {}

    _configure_gmft(config_dict, file_config, cli_args)

    assert "gmft_config" not in config_dict


def test_legacy_functions_comprehensive_build_config_none_to_null() -> None:
    file_config = {"ocr_backend": "none"}
    cli_args: dict[str, Any] = {}

    result = build_extraction_config(file_config, cli_args)

    assert result.ocr_backend is None


def test_legacy_functions_comprehensive_build_config_complex_override() -> None:
    file_config = {
        "force_ocr": True,
        "chunk_content": False,
        "max_chars": 1000,
        "ocr_backend": "tesseract",
        "tesseract": {"language": "eng", "psm": 3},
        "extract_tables": True,
        "gmft": {"verbosity": 1, "detector_base_threshold": 0.5},
    }

    cli_args = {
        "force_ocr": None,
        "chunk_content": True,
        "max_overlap": 100,
        "tesseract_config": {"psm": 6},
        "gmft_config": None,
        "unused_arg": "ignored",
    }

    result = build_extraction_config(file_config, cli_args)

    assert result.force_ocr is True
    assert result.chunk_content is True
    assert result.max_chars == 1000
    assert result.max_overlap == 100
    assert result.ocr_backend == "tesseract"
    assert isinstance(result.ocr_config, TesseractConfig)
    assert result.ocr_config.psm == 6  # type: ignore[comparison-overlap]  # CLI override
    assert result.ocr_config.language == "eng"


def test_edge_cases_load_config_from_path_pathlib(tmp_path: Path) -> None:
    config_file = tmp_path / "config.toml"
    config_file.write_text("extract_entities = true")

    result = load_config_from_path(config_file)
    assert result.extract_entities is True


def test_edge_cases_discover_config_search_path_context(tmp_path: Path) -> None:
    with pytest.raises(ValidationError) as exc_info:
        discover_and_load_config(tmp_path)

    assert "search_path" in exc_info.value.context
    assert str(tmp_path) in exc_info.value.context["search_path"]


def test_edge_cases_discover_config_file_path_context(tmp_path: Path) -> None:
    config_file = tmp_path / "kreuzberg.toml"
    config_file.write_text("")

    with pytest.raises(ValidationError) as exc_info:
        discover_and_load_config(tmp_path)

    assert "config_path" in exc_info.value.context
    assert str(config_file) in exc_info.value.context["config_path"]


def test_edge_cases_html_to_markdown_config_all_params() -> None:
    from kreuzberg._types import HTMLToMarkdownConfig

    config_dict = {
        "html_to_markdown": {
            "heading_style": "atx_closed",
            "strong_em_symbol": "_",
            "escape_asterisks": False,
            "escape_underscores": False,
            "wrap": True,
            "wrap_width": 80,
            "bullets": "-",
            "preprocessing_preset": "minimal",
            "remove_navigation": False,
            "remove_forms": False,
        },
    }

    result = build_extraction_config_from_dict(config_dict)
    assert result.html_to_markdown_config is not None
    assert isinstance(result.html_to_markdown_config, HTMLToMarkdownConfig)
    assert result.html_to_markdown_config.heading_style == "atx_closed"
    assert result.html_to_markdown_config.strong_em_symbol == "_"
    assert result.html_to_markdown_config.escape_asterisks is False
    assert result.html_to_markdown_config.escape_underscores is False
    assert result.html_to_markdown_config.wrap is True
    assert result.html_to_markdown_config.wrap_width == 80
    assert result.html_to_markdown_config.bullets == "-"
    assert result.html_to_markdown_config.preprocessing_preset == "minimal"
    assert result.html_to_markdown_config.remove_navigation is False
    assert result.html_to_markdown_config.remove_forms is False


def test_edge_cases_gmft_config_all_parameters() -> None:
    config_dict = {
        "extract_tables": True,
        "gmft": {
            "verbosity": 3,
            "detector_base_threshold": 0.6,
            "formatter_base_threshold": 0.5,
            "cell_required_confidence": {0: 0.1, 1: 0.2, 2: 0.3, 3: 0.4, 4: 0.5, 5: 0.6, 6: 0.9},
            "remove_null_rows": False,
        },
    }

    result = build_extraction_config_from_dict(config_dict)
    assert result.gmft_config is not None
    assert result.gmft_config.verbosity == 3
    assert result.gmft_config.detector_base_threshold == 0.6
    assert result.gmft_config.formatter_base_threshold == 0.5
    assert result.gmft_config.cell_required_confidence == {0: 0.1, 1: 0.2, 2: 0.3, 3: 0.4, 4: 0.5, 5: 0.6, 6: 0.9}
    assert result.gmft_config.remove_null_rows is False

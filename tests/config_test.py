"""Tests for configuration discovery and loading."""

from __future__ import annotations

import tempfile
from pathlib import Path

import pytest

from kreuzberg._config import (
    build_extraction_config,
    build_extraction_config_from_dict,
    discover_and_load_config,
    find_config_file,
    load_config_from_file,
    load_config_from_path,
    merge_configs,
    parse_ocr_backend_config,
    try_discover_config,
)
from kreuzberg._ocr._tesseract import TesseractConfig
from kreuzberg._types import ExtractionConfig
from kreuzberg.exceptions import ValidationError


class TestConfigFileLoading:
    """Test configuration file loading functionality."""

    def test_load_kreuzberg_toml(self, tmp_path: Path) -> None:
        """Test loading from kreuzberg.toml file."""
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

    def test_load_pyproject_toml(self, tmp_path: Path) -> None:
        """Test loading from pyproject.toml with [tool.kreuzberg] section."""
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

    def test_load_pyproject_toml_no_kreuzberg_section(self, tmp_path: Path) -> None:
        """Test loading from pyproject.toml without [tool.kreuzberg] section."""
        config_file = tmp_path / "pyproject.toml"
        config_file.write_text("""
[build-system]
requires = ["hatchling"]
""")

        result = load_config_from_file(config_file)
        assert result == {}

    def test_load_missing_file(self, tmp_path: Path) -> None:
        """Test error when config file doesn't exist."""
        config_file = tmp_path / "nonexistent.toml"

        with pytest.raises(ValidationError, match="Configuration file not found"):
            load_config_from_file(config_file)

    def test_load_invalid_toml(self, tmp_path: Path) -> None:
        """Test error when TOML file is invalid."""
        config_file = tmp_path / "invalid.toml"
        config_file.write_text("invalid [ toml")

        with pytest.raises(ValidationError, match="Invalid TOML"):
            load_config_from_file(config_file)


class TestConfigDiscovery:
    """Test configuration file discovery functionality."""

    def test_find_kreuzberg_toml(self, tmp_path: Path) -> None:
        """Test finding kreuzberg.toml file."""
        config_file = tmp_path / "kreuzberg.toml"
        config_file.write_text("force_ocr = true")

        result = find_config_file(tmp_path)
        assert result == config_file

    def test_find_pyproject_toml_with_kreuzberg_section(self, tmp_path: Path) -> None:
        """Test finding pyproject.toml with [tool.kreuzberg] section."""
        config_file = tmp_path / "pyproject.toml"
        config_file.write_text("""
[tool.kreuzberg]
force_ocr = true
""")

        result = find_config_file(tmp_path)
        assert result == config_file

    def test_find_pyproject_toml_without_kreuzberg_section(self, tmp_path: Path) -> None:
        """Test that pyproject.toml without [tool.kreuzberg] is ignored."""
        config_file = tmp_path / "pyproject.toml"
        config_file.write_text("""
[build-system]
requires = ["hatchling"]
""")

        result = find_config_file(tmp_path)
        assert result is None

    def test_find_config_prefers_kreuzberg_toml(self, tmp_path: Path) -> None:
        """Test that kreuzberg.toml is preferred over pyproject.toml."""
        kreuzberg_file = tmp_path / "kreuzberg.toml"
        pyproject_file = tmp_path / "pyproject.toml"

        kreuzberg_file.write_text("force_ocr = true")
        pyproject_file.write_text("""
[tool.kreuzberg]
force_ocr = false
""")

        result = find_config_file(tmp_path)
        assert result == kreuzberg_file

    def test_find_config_searches_up_tree(self, tmp_path: Path) -> None:
        """Test that config search goes up directory tree."""
        # Create config in parent directory
        config_file = tmp_path / "kreuzberg.toml"
        config_file.write_text("force_ocr = true")

        # Search from subdirectory
        subdir = tmp_path / "subdir" / "deep"
        subdir.mkdir(parents=True)

        result = find_config_file(subdir)
        assert result == config_file

    def test_find_config_no_file_found(self, tmp_path: Path) -> None:
        """Test when no config file is found."""
        result = find_config_file(tmp_path)
        assert result is None


class TestConfigParsing:
    """Test configuration parsing functionality."""

    def test_merge_configs_simple(self) -> None:
        """Test merging simple configurations."""
        base = {"force_ocr": False, "max_chars": 1000}
        override = {"force_ocr": True, "chunk_content": True}

        result = merge_configs(base, override)
        assert result == {
            "force_ocr": True,
            "max_chars": 1000,
            "chunk_content": True,
        }

    def test_merge_configs_nested(self) -> None:
        """Test merging nested configurations."""
        base = {"tesseract": {"lang": "eng", "psm": 3}}
        override = {"tesseract": {"psm": 6, "oem": 1}}

        result = merge_configs(base, override)
        assert result == {"tesseract": {"lang": "eng", "psm": 6, "oem": 1}}

    def test_parse_tesseract_config(self) -> None:
        """Test parsing Tesseract OCR configuration."""
        config_dict = {"tesseract": {"language": "eng", "psm": 6}}

        result = parse_ocr_backend_config(config_dict, "tesseract")
        assert isinstance(result, TesseractConfig)
        assert result.language == "eng"
        assert result.psm.value == 6

    def test_parse_ocr_config_missing_backend(self) -> None:
        """Test parsing when OCR backend config is missing."""
        config_dict = {"other_setting": "value"}

        result = parse_ocr_backend_config(config_dict, "tesseract")
        assert result is None

    def test_parse_ocr_config_invalid_type(self) -> None:
        """Test parsing when OCR backend config is not a dict."""
        config_dict = {"tesseract": "invalid"}

        result = parse_ocr_backend_config(config_dict, "tesseract")
        assert result is None


class TestExtractionConfigBuilder:
    """Test ExtractionConfig building functionality."""

    def test_build_from_dict_basic(self) -> None:
        """Test building ExtractionConfig from basic dictionary."""
        config_dict = {
            "force_ocr": True,
            "chunk_content": False,
            "extract_tables": True,
            "max_chars": 2000,
        }

        result = build_extraction_config_from_dict(config_dict)
        assert isinstance(result, ExtractionConfig)
        assert result.force_ocr is True
        assert result.chunk_content is False
        assert result.extract_tables is True
        assert result.max_chars == 2000

    def test_build_from_dict_with_ocr_config(self) -> None:
        """Test building ExtractionConfig with OCR configuration."""
        config_dict = {
            "force_ocr": True,
            "ocr_backend": "tesseract",
            "tesseract": {"language": "eng", "psm": 6},
        }

        result = build_extraction_config_from_dict(config_dict)
        assert result.ocr_backend == "tesseract"
        assert isinstance(result.ocr_config, TesseractConfig)
        assert result.ocr_config.language == "eng"

    def test_build_from_dict_ocr_backend_none(self) -> None:
        """Test building ExtractionConfig with OCR backend set to 'none'."""
        config_dict = {"ocr_backend": "none"}

        result = build_extraction_config_from_dict(config_dict)
        assert result.ocr_backend is None

    def test_build_extraction_config_legacy(self) -> None:
        """Test legacy build_extraction_config function."""
        file_config = {
            "force_ocr": False,
            "max_chars": 1000,
            "tesseract": {"language": "eng"},
        }
        cli_args = {
            "force_ocr": True,
            "ocr_backend": "tesseract",
        }

        result = build_extraction_config(file_config, cli_args)
        assert result.force_ocr is True  # CLI overrides file
        assert result.max_chars == 1000  # File value preserved
        assert result.ocr_backend == "tesseract"


class TestHighLevelAPI:
    """Test high-level configuration API."""

    def test_load_config_from_path(self, tmp_path: Path) -> None:
        """Test loading configuration from specific path."""
        config_file = tmp_path / "kreuzberg.toml"
        config_file.write_text("""
force_ocr = true
chunk_content = false
""")

        result = load_config_from_path(config_file)
        assert isinstance(result, ExtractionConfig)
        assert result.force_ocr is True
        assert result.chunk_content is False

    def test_discover_and_load_config(self, tmp_path: Path) -> None:
        """Test discovering and loading configuration."""
        config_file = tmp_path / "kreuzberg.toml"
        config_file.write_text("force_ocr = true")

        result = discover_and_load_config(tmp_path)
        assert isinstance(result, ExtractionConfig)
        assert result.force_ocr is True

    def test_discover_and_load_config_not_found(self, tmp_path: Path) -> None:
        """Test error when no config file is discovered."""
        with pytest.raises(ValidationError, match="No configuration file found"):
            discover_and_load_config(tmp_path)

    def test_discover_and_load_config_empty(self, tmp_path: Path) -> None:
        """Test error when config file exists but is empty."""
        config_file = tmp_path / "pyproject.toml"
        config_file.write_text("""
[build-system]
requires = ["hatchling"]
""")

        with pytest.raises(ValidationError, match="No configuration file found"):
            discover_and_load_config(tmp_path)

    def test_try_discover_config_success(self, tmp_path: Path) -> None:
        """Test try_discover_config with successful discovery."""
        config_file = tmp_path / "kreuzberg.toml"
        config_file.write_text("force_ocr = true")

        result = try_discover_config(tmp_path)
        assert isinstance(result, ExtractionConfig)
        assert result.force_ocr is True

    def test_try_discover_config_not_found(self, tmp_path: Path) -> None:
        """Test try_discover_config when no config is found."""
        result = try_discover_config(tmp_path)
        assert result is None

    def test_try_discover_config_invalid(self, tmp_path: Path) -> None:
        """Test try_discover_config with invalid config."""
        config_file = tmp_path / "kreuzberg.toml"
        config_file.write_text("invalid [ toml")

        result = try_discover_config(tmp_path)
        assert result is None


class TestConfigIntegration:
    """Test configuration system integration scenarios."""

    def test_real_world_pyproject_toml(self, tmp_path: Path) -> None:
        """Test with realistic pyproject.toml configuration."""
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

        # Check nested OCR config
        assert isinstance(config.ocr_config, TesseractConfig)
        assert config.ocr_config.language == "eng+fra"
        assert config.ocr_config.psm.value == 6

    def test_config_discovery_with_cwd(self) -> None:
        """Test config discovery using current working directory."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            tmp_path = Path(tmp_dir)
            config_file = tmp_path / "kreuzberg.toml"
            config_file.write_text("force_ocr = true")

            # Change to temp directory and test discovery
            import os

            original_cwd = Path.cwd()
            try:
                os.chdir(tmp_dir)
                result = try_discover_config()
                assert result is not None
                assert result.force_ocr is True
            finally:
                os.chdir(str(original_cwd))

    def test_config_priority_kreuzberg_over_pyproject(self, tmp_path: Path) -> None:
        """Test that kreuzberg.toml takes priority over pyproject.toml."""
        # Create both files with different settings
        kreuzberg_file = tmp_path / "kreuzberg.toml"
        kreuzberg_file.write_text("force_ocr = true")

        pyproject_file = tmp_path / "pyproject.toml"
        pyproject_file.write_text("""
[tool.kreuzberg]
force_ocr = false
""")

        config = discover_and_load_config(tmp_path)
        assert config.force_ocr is True  # Should use kreuzberg.toml value

"""Tests for the CLI module."""

from __future__ import annotations

import json
from pathlib import Path
from typing import TYPE_CHECKING
from unittest.mock import Mock

import pytest
from click.testing import CliRunner

from kreuzberg._cli_config import (
    build_extraction_config,
    load_config_from_file,
    merge_configs,
    parse_ocr_backend_config,
)
from kreuzberg._ocr._tesseract import TesseractConfig
from kreuzberg.exceptions import ValidationError

if TYPE_CHECKING:
    from pytest_mock import MockerFixture


try:
    from kreuzberg.cli import cli

    CLI_AVAILABLE = True
except ImportError:
    CLI_AVAILABLE = False

pytestmark = pytest.mark.skipif(not CLI_AVAILABLE, reason="CLI dependencies not installed")


class TestCliConfig:
    """Test configuration parsing functionality."""

    def test_load_config_from_file(self, tmp_path: Path) -> None:
        """Test loading configuration from TOML file."""
        config_file = tmp_path / "pyproject.toml"
        config_file.write_text("""
[tool.kreuzberg]
force_ocr = true
chunk_content = false
max_chars = 5000

[tool.kreuzberg.tesseract]
lang = "eng+deu"
psm = 3
""")

        config = load_config_from_file(config_file)
        assert config["force_ocr"] is True
        assert config["chunk_content"] is False
        assert config["max_chars"] == 5000
        assert config["tesseract"]["lang"] == "eng+deu"
        assert config["tesseract"]["psm"] == 3

    def test_load_config_file_not_found(self) -> None:
        """Test error when config file doesn't exist."""
        with pytest.raises(ValidationError, match="Configuration file not found"):
            load_config_from_file(Path("nonexistent.toml"))

    def test_load_config_invalid_toml(self, tmp_path: Path) -> None:
        """Test error when config file has invalid TOML."""
        config_file = tmp_path / "invalid.toml"
        config_file.write_text("invalid toml content [")

        with pytest.raises(ValidationError, match="Invalid TOML"):
            load_config_from_file(config_file)

    def test_merge_configs(self) -> None:
        """Test configuration merging."""
        base = {
            "force_ocr": False,
            "max_chars": 1000,
            "tesseract": {"lang": "eng", "psm": 3},
        }
        override = {
            "force_ocr": True,
            "tesseract": {"lang": "deu"},
        }

        result = merge_configs(base, override)
        assert result["force_ocr"] is True
        assert result["max_chars"] == 1000
        assert result["tesseract"]["lang"] == "deu"
        assert result["tesseract"]["psm"] == 3

    def test_parse_ocr_backend_config(self) -> None:
        """Test parsing OCR backend configuration."""
        config_dict = {
            "tesseract": {"language": "eng", "psm": 3},
            "easyocr": {"languages": ["en", "de"]},
        }

        tesseract_config = parse_ocr_backend_config(config_dict, "tesseract")
        assert isinstance(tesseract_config, TesseractConfig)
        assert tesseract_config.language == "eng"
        assert (tesseract_config.psm.value if hasattr(tesseract_config.psm, "value") else tesseract_config.psm) == 3

        assert parse_ocr_backend_config(config_dict, "paddleocr") is None

    def test_build_extraction_config(self) -> None:
        """Test building ExtractionConfig from file and CLI args."""
        file_config = {
            "force_ocr": True,
            "chunk_content": False,
            "max_chars": 5000,
            "tesseract": {"language": "eng"},
        }
        cli_args = {
            "chunk_content": True,
            "ocr_backend": "tesseract",
            "tesseract_config": {"psm": 6},
        }

        config = build_extraction_config(file_config, cli_args)
        assert config.force_ocr is True
        assert config.chunk_content is True
        assert config.max_chars == 5000
        assert config.ocr_backend == "tesseract"
        assert isinstance(config.ocr_config, TesseractConfig)
        assert config.ocr_config.language == "eng"
        assert (config.ocr_config.psm.value if hasattr(config.ocr_config.psm, "value") else config.ocr_config.psm) == 6

    def test_build_extraction_config_ocr_none(self) -> None:
        """Test building config with OCR disabled."""
        cli_args = {"ocr_backend": "none"}
        config = build_extraction_config({}, cli_args)
        assert config.ocr_backend is None


class TestCli:
    """Test CLI commands."""

    def test_cli_help(self) -> None:
        """Test CLI help output."""
        runner = CliRunner()
        result = runner.invoke(cli, ["--help"])
        assert result.exit_code == 0
        assert "Kreuzberg - Text extraction from documents" in result.output

    def test_cli_version(self) -> None:
        """Test CLI version output."""
        runner = CliRunner()
        result = runner.invoke(cli, ["--version"])
        assert result.exit_code == 0
        assert "kreuzberg, version" in result.output

    def test_extract_file(self, tmp_path: Path, mocker: MockerFixture) -> None:
        """Test extracting from a file."""

        test_file = tmp_path / "test.pdf"
        test_file.write_bytes(b"dummy content")

        mock_result = Mock()
        mock_result.content = "Extracted text content"
        mock_result.mime_type = "application/pdf"
        mock_result.metadata = {"pages": 1}
        mock_result.tables = None
        mock_result.chunks = None

        mocker.patch("kreuzberg.cli.extract_file_sync", return_value=mock_result)

        runner = CliRunner()
        result = runner.invoke(cli, ["extract", str(test_file)])

        assert result.exit_code == 0
        assert "Extracted text content" in result.output

    def test_extract_file_with_output(self, tmp_path: Path, mocker: MockerFixture) -> None:
        """Test extracting to an output file."""
        test_file = tmp_path / "test.pdf"
        test_file.write_bytes(b"dummy content")
        output_file = tmp_path / "output.txt"

        mock_result = Mock()
        mock_result.content = "Extracted text"
        mock_result.mime_type = "application/pdf"
        mock_result.metadata = {}
        mock_result.tables = None
        mock_result.chunks = None

        mocker.patch("kreuzberg.cli.extract_file_sync", return_value=mock_result)

        runner = CliRunner()
        result = runner.invoke(cli, ["extract", str(test_file), "-o", str(output_file)])

        assert result.exit_code == 0
        assert output_file.read_text() == "Extracted text"

    def test_extract_stdin(self, mocker: MockerFixture) -> None:
        """Test extracting from stdin."""
        mock_result = Mock()
        mock_result.content = "Text from stdin"
        mock_result.mime_type = "text/plain"
        mock_result.metadata = {}
        mock_result.tables = None
        mock_result.chunks = None

        mocker.patch("kreuzberg.cli.extract_bytes_sync", return_value=mock_result)

        runner = CliRunner()
        result = runner.invoke(cli, ["extract"], input=b"test content")

        assert result.exit_code == 0
        assert "Text from stdin" in result.output

    def test_extract_with_options(self, tmp_path: Path, mocker: MockerFixture) -> None:
        """Test extraction with various options."""
        test_file = tmp_path / "test.pdf"
        test_file.write_bytes(b"dummy")

        mock_result = Mock()
        mock_result.content = "Content"
        mock_result.mime_type = "application/pdf"
        mock_result.metadata = {"test": "value"}
        mock_result.tables = [{"data": [[1, 2], [3, 4]]}]
        mock_result.chunks = None

        extract_mock = mocker.patch("kreuzberg.cli.extract_file_sync", return_value=mock_result)

        runner = CliRunner()
        result = runner.invoke(
            cli,
            [
                "extract",
                str(test_file),
                "--force-ocr",
                "--chunk-content",
                "--extract-tables",
                "--ocr-backend",
                "easyocr",
                "--show-metadata",
                "--output-format",
                "json",
            ],
        )

        assert result.exit_code == 0

        extract_mock.assert_called_once()
        config = extract_mock.call_args[1]["config"]
        assert config.force_ocr is True
        assert config.chunk_content is True
        assert config.extract_tables is True
        assert config.ocr_backend == "easyocr"

        output_data = json.loads(result.output)
        assert output_data["content"] == "Content"
        assert output_data["metadata"] == {"test": "value"}
        assert output_data["tables"] == [{"data": [[1, 2], [3, 4]]}]

    def test_extract_with_config_file(self, tmp_path: Path, mocker: MockerFixture) -> None:
        """Test extraction with config file."""
        test_file = tmp_path / "test.pdf"
        test_file.write_bytes(b"dummy")

        config_file = tmp_path / "config.toml"
        config_file.write_text("""
[tool.kreuzberg]
force_ocr = true
max_chars = 2000
""")

        mock_result = Mock()
        mock_result.content = "Content"
        mock_result.mime_type = "application/pdf"
        mock_result.metadata = {}
        mock_result.tables = None
        mock_result.chunks = None

        extract_mock = mocker.patch("kreuzberg.cli.extract_file_sync", return_value=mock_result)

        runner = CliRunner()
        result = runner.invoke(cli, ["extract", str(test_file), "--config", str(config_file)])

        assert result.exit_code == 0
        config = extract_mock.call_args[1]["config"]
        assert config.force_ocr is True
        assert config.max_chars == 2000

    def test_config_command(self, tmp_path: Path) -> None:
        """Test config command."""
        config_file = tmp_path / "pyproject.toml"
        config_file.write_text("""
[tool.kreuzberg]
force_ocr = true
chunk_content = false
""")

        runner = CliRunner()
        with runner.isolated_filesystem(temp_dir=tmp_path):
            result = runner.invoke(cli, ["config"])

        assert result.exit_code == 0
        assert "force_ocr" in result.output

    def test_error_handling(self, tmp_path: Path, mocker: MockerFixture) -> None:
        """Test error handling."""
        from kreuzberg.exceptions import ParsingError

        test_file = tmp_path / "test.pdf"
        test_file.write_bytes(b"dummy")

        mocker.patch("kreuzberg.cli.extract_file_sync", side_effect=ParsingError("Failed to parse"))

        runner = CliRunner()
        result = runner.invoke(cli, ["extract", str(test_file)])

        assert result.exit_code == 1
        assert "Error:" in result.output
        assert "Failed to parse" in result.output

    def test_missing_dependency_error(self, tmp_path: Path, mocker: MockerFixture) -> None:
        """Test missing dependency error handling."""
        from kreuzberg.exceptions import MissingDependencyError

        test_file = tmp_path / "test.pdf"
        test_file.write_bytes(b"dummy")

        mocker.patch(
            "kreuzberg.cli.extract_file_sync",
            side_effect=MissingDependencyError.create_for_package(
                dependency_group="easyocr", functionality="OCR processing", package_name="easyocr"
            ),
        )

        runner = CliRunner()
        result = runner.invoke(cli, ["extract", str(test_file)])

        assert result.exit_code == 2
        assert "Missing dependency:" in result.output

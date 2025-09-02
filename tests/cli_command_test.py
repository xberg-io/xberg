from __future__ import annotations

import json
from pathlib import Path
from typing import TYPE_CHECKING
from unittest.mock import Mock, patch

import click
import pytest
from click.testing import CliRunner

if TYPE_CHECKING:
    from pytest_mock import MockerFixture

try:
    from kreuzberg._types import ExtractionResult, Metadata
    from kreuzberg.cli import OcrBackendParamType, cli, format_extraction_result

    CLI_AVAILABLE = True
except ImportError:
    CLI_AVAILABLE = False

pytestmark = pytest.mark.skipif(not CLI_AVAILABLE, reason="CLI dependencies not installed")


class TestOcrBackendParamType:
    def test_convert_none(self) -> None:
        param_type = OcrBackendParamType()
        result = param_type.convert(None, None, None)
        assert result is None

    def test_convert_none_string(self) -> None:
        param_type = OcrBackendParamType()
        result = param_type.convert("none", None, None)
        assert result == "none"

    def test_convert_valid_backends(self) -> None:
        param_type = OcrBackendParamType()

        assert param_type.convert("tesseract", None, None) == "tesseract"
        assert param_type.convert("TESSERACT", None, None) == "tesseract"
        assert param_type.convert("EasyOCR", None, None) == "easyocr"
        assert param_type.convert("paddleocr", None, None) == "paddleocr"

    def test_convert_invalid_backend(self) -> None:
        param_type = OcrBackendParamType()

        with pytest.raises(click.exceptions.BadParameter, match="Invalid OCR backend"):
            param_type.convert("invalid", None, None)


class TestFormatExtractionResult:
    def test_format_json_output(self) -> None:
        metadata: Metadata = {"title": "Test Document", "authors": ["Test Author"]}
        result = ExtractionResult(
            content="Test content",
            metadata=metadata,
            mime_type="text/plain",
            tables=[],
            chunks=["chunk1", "chunk2"],
        )

        output = format_extraction_result(result, show_metadata=True, output_format="json")
        data = json.loads(output)

        assert data["content"] == "Test content"
        assert data["metadata"]["title"] == "Test Document"
        assert data["mime_type"] == "text/plain"
        assert "tables" not in data
        assert len(data["chunks"]) == 2

    def test_format_text_output_no_metadata(self) -> None:
        result = ExtractionResult(content="Test content", metadata={}, mime_type="text/plain")

        output = format_extraction_result(result, show_metadata=False, output_format="text")
        assert output == "Test content"

    def test_format_text_output_with_metadata(self) -> None:
        metadata: Metadata = {"title": "Test Document"}
        result = ExtractionResult(content="Test content", metadata=metadata, mime_type="text/plain")

        output = format_extraction_result(result, show_metadata=True, output_format="text")
        assert "Test content" in output
        assert "--- METADATA ---" in output
        assert '"title": "Test Document"' in output

    def test_format_text_output_with_tables(self) -> None:
        result = ExtractionResult(
            content="Test content",
            metadata={},
            mime_type="text/plain",
            tables=[],
        )

        output = format_extraction_result(result, show_metadata=False, output_format="text")
        assert "Test content" in output
        assert "--- TABLES ---" not in output


class TestCliCommands:
    def test_cli_no_subcommand(self) -> None:
        runner = CliRunner()
        result = runner.invoke(cli, [])

        assert result.exit_code == 0
        assert "Kreuzberg - Text extraction from documents" in result.output
        assert "Commands:" in result.output

    def test_cli_version(self) -> None:
        runner = CliRunner()
        result = runner.invoke(cli, ["--version"])

        assert result.exit_code == 0
        assert "kreuzberg, version" in result.output

    @patch("kreuzberg.cli.extract_file_sync")
    def test_extract_file_basic(self, mock_extract: Mock) -> None:
        mock_result = ExtractionResult(
            content="Extracted text content", metadata={"title": "Test"}, mime_type="text/plain"
        )
        mock_extract.return_value = mock_result

        runner = CliRunner()
        with runner.isolated_filesystem():
            test_file = Path("test.txt")
            test_file.write_text("Test content")

            result = runner.invoke(cli, ["extract", str(test_file)])

        assert result.exit_code == 0
        assert "Extracted text content" in result.output
        mock_extract.assert_called_once()

    @patch("kreuzberg.cli.extract_bytes_sync")
    def test_extract_stdin(self, mock_extract: Mock) -> None:
        mock_result = ExtractionResult(content="Content from stdin", metadata={}, mime_type="text/plain")
        mock_extract.return_value = mock_result

        runner = CliRunner()
        result = runner.invoke(cli, ["extract"], input=b"Test input from stdin")

        assert result.exit_code == 0
        assert "Content from stdin" in result.output
        mock_extract.assert_called_once()
        assert mock_extract.call_args[0][0] == b"Test input from stdin"

    @patch("kreuzberg.cli.extract_file_sync")
    def test_extract_with_output_file(self, mock_extract: Mock) -> None:
        mock_result = ExtractionResult(content="Output file content", metadata={}, mime_type="text/plain")
        mock_extract.return_value = mock_result

        runner = CliRunner()
        with runner.isolated_filesystem():
            test_file = Path("input.txt")
            test_file.write_text("Input")
            output_file = Path("output.txt")

            result = runner.invoke(cli, ["extract", str(test_file), "-o", str(output_file)])

            assert result.exit_code == 0
            assert output_file.exists()
            assert output_file.read_text() == "Output file content"

    @patch("kreuzberg.cli.extract_file_sync")
    def test_extract_with_json_output(self, mock_extract: Mock) -> None:
        metadata: Metadata = {"title": "JSON Test"}
        mock_result = ExtractionResult(content="JSON content", metadata=metadata, mime_type="application/pdf")
        mock_extract.return_value = mock_result

        runner = CliRunner()
        with runner.isolated_filesystem():
            test_file = Path("test.pdf")
            test_file.write_bytes(b"PDF content")

            result = runner.invoke(cli, ["extract", str(test_file), "--output-format", "json", "--show-metadata"])

        assert result.exit_code == 0
        output_data = json.loads(result.output)
        assert output_data["content"] == "JSON content"
        assert output_data["metadata"]["title"] == "JSON Test"
        assert output_data["mime_type"] == "application/pdf"

    @patch("kreuzberg.cli.extract_file_sync")
    def test_extract_with_metadata(self, mock_extract: Mock) -> None:
        metadata: Metadata = {"authors": ["Test Author"], "created_at": "2024-01-01"}
        mock_result = ExtractionResult(content="Content with metadata", metadata=metadata, mime_type="text/plain")
        mock_extract.return_value = mock_result

        runner = CliRunner()
        with runner.isolated_filesystem():
            test_file = Path("test.txt")
            test_file.write_text("Test")

            result = runner.invoke(cli, ["extract", str(test_file), "--show-metadata"])

        assert result.exit_code == 0
        assert "Content with metadata" in result.output
        assert "--- METADATA ---" in result.output
        assert '"authors"' in result.output
        assert "Test Author" in result.output

    @patch("kreuzberg.cli.extract_file_sync")
    def test_extract_with_ocr_options(self, mock_extract: Mock) -> None:
        mock_result = ExtractionResult(content="OCR content", metadata={}, mime_type="text/plain")
        mock_extract.return_value = mock_result

        runner = CliRunner()
        with runner.isolated_filesystem():
            test_file = Path("test.png")
            test_file.write_bytes(b"Image content")

            result = runner.invoke(
                cli,
                [
                    "extract",
                    str(test_file),
                    "--force-ocr",
                    "--ocr-backend",
                    "tesseract",
                    "--tesseract-lang",
                    "eng+deu",
                    "--tesseract-psm",
                    "3",
                ],
            )

        assert result.exit_code == 0
        assert "OCR content" in result.output

        call_args = mock_extract.call_args
        assert call_args is not None

    @patch("kreuzberg.cli.extract_file_sync")
    def test_extract_with_chunking(self, mock_extract: Mock) -> None:
        mock_result = ExtractionResult(
            content="Chunked content",
            metadata={},
            mime_type="text/plain",
            chunks=["Chunk 1 content", "Chunk 2 content", "Chunk 3 content"],
        )
        mock_extract.return_value = mock_result

        runner = CliRunner()
        with runner.isolated_filesystem():
            test_file = Path("test.txt")
            test_file.write_text("Long content")

            result = runner.invoke(
                cli,
                [
                    "extract",
                    str(test_file),
                    "--chunk-content",
                    "--max-chars",
                    "1000",
                    "--max-overlap",
                    "100",
                    "--output-format",
                    "json",
                ],
            )

        assert result.exit_code == 0
        output_data = json.loads(result.output)
        assert len(output_data["chunks"]) == 3
        assert output_data["chunks"][0] == "Chunk 1 content"

    @patch("kreuzberg.cli.extract_file_sync")
    def test_extract_with_tables(self, mock_extract: Mock) -> None:
        mock_result = ExtractionResult(
            content="Document with tables",
            metadata={},
            mime_type="text/plain",
            tables=[],
        )
        mock_extract.return_value = mock_result

        runner = CliRunner()
        with runner.isolated_filesystem():
            test_file = Path("test.pdf")
            test_file.write_bytes(b"PDF")

            result = runner.invoke(cli, ["extract", str(test_file), "--extract-tables"])

        assert result.exit_code == 0
        assert "Document with tables" in result.output
        assert "--- TABLES ---" not in result.output

    @patch("kreuzberg.cli.find_config_file")
    @patch("kreuzberg.cli.load_config_from_file")
    @patch("kreuzberg.cli.extract_file_sync")
    def test_extract_with_config_file(self, mock_extract: Mock, mock_load_config: Mock, mock_find_config: Mock) -> None:
        mock_find_config.return_value = None
        mock_load_config.return_value = {"force_ocr": True, "chunk_content": True}
        mock_result = ExtractionResult(content="Config file test", metadata={}, mime_type="text/plain")
        mock_extract.return_value = mock_result

        runner = CliRunner()
        with runner.isolated_filesystem():
            test_file = Path("test.txt")
            test_file.write_text("Test")
            config_file = Path("config.toml")
            config_file.write_text("[tool.kreuzberg]\nforce_ocr = true")

            result = runner.invoke(cli, ["extract", str(test_file), "--config", str(config_file)])

        assert result.exit_code == 0
        assert "Config file test" in result.output
        mock_load_config.assert_called_once_with(config_file)

    @patch("kreuzberg.cli.extract_file_sync")
    def test_extract_verbose_mode(self, mock_extract: Mock) -> None:
        mock_result = ExtractionResult(content="Verbose output", metadata={}, mime_type="text/plain")
        mock_extract.return_value = mock_result

        runner = CliRunner()
        with runner.isolated_filesystem():
            test_file = Path("test.txt")
            test_file.write_text("Test")

            result = runner.invoke(cli, ["extract", str(test_file), "-v"])

        assert result.exit_code == 0
        assert "Verbose output" in result.output

    @patch("kreuzberg.cli.extract_file_sync")
    def test_extract_error_handling(self, mock_extract: Mock) -> None:
        mock_extract.side_effect = Exception("Extraction failed")

        runner = CliRunner()
        with runner.isolated_filesystem():
            test_file = Path("test.txt")
            test_file.write_text("Test")

            result = runner.invoke(cli, ["extract", str(test_file)])

        assert result.exit_code == 1
        assert "Error:" in result.output or "Extraction failed" in result.output

    @patch("kreuzberg.cli.extract_file_sync")
    def test_extract_missing_dependency_error(self, mock_extract: Mock) -> None:
        from kreuzberg.exceptions import MissingDependencyError

        mock_extract.side_effect = MissingDependencyError.create_for_package(
            dependency_group="ocr", functionality="text extraction", package_name="tesseract"
        )

        runner = CliRunner()
        with runner.isolated_filesystem():
            test_file = Path("test.txt")
            test_file.write_text("Test")

            result = runner.invoke(cli, ["extract", str(test_file)])

        assert result.exit_code != 0
        assert (
            "Missing dependency" in result.output or "tesseract" in result.output or "kreuzberg['ocr']" in result.output
        )

    @patch("kreuzberg.cli.extract_file_sync")
    def test_extract_all_ocr_backends(self, mock_extract: Mock) -> None:
        mock_result = ExtractionResult(content="OCR result", metadata={}, mime_type="text/plain")
        mock_extract.return_value = mock_result

        runner = CliRunner()
        with runner.isolated_filesystem():
            test_file = Path("test.png")
            test_file.write_bytes(b"Image")

            result = runner.invoke(cli, ["extract", str(test_file)])
            assert result.exit_code == 0
            assert "OCR result" in result.output

            result = runner.invoke(cli, ["extract", str(test_file), "--ocr-backend", "none"])
            assert result.exit_code == 0
            assert "OCR result" in result.output

    def test_extract_nonexistent_file(self) -> None:
        runner = CliRunner()
        result = runner.invoke(cli, ["extract", "nonexistent.txt"])

        assert result.exit_code != 0
        assert "does not exist" in result.output or "Error" in result.output

    @patch("kreuzberg.cli.extract_file_sync")
    def test_extract_progress_display(self, mock_extract: Mock, mocker: MockerFixture) -> None:
        mock_result = ExtractionResult(content="Progress test", metadata={}, mime_type="text/plain")
        mock_extract.return_value = mock_result

        mock_progress = mocker.patch("kreuzberg.cli.Progress")

        runner = CliRunner()
        with runner.isolated_filesystem():
            test_file = Path("test.txt")
            test_file.write_text("Test")

            result = runner.invoke(cli, ["extract", str(test_file)])

        assert result.exit_code == 0
        mock_progress.assert_called()


class TestConfigCommand:
    def test_config_no_config_file(self) -> None:
        with patch("kreuzberg.cli.find_config_file", return_value=None):
            runner = CliRunner()
            result = runner.invoke(cli, ["config"])

            assert result.exit_code == 0
            assert "No configuration file found" in result.output
            assert "Default configuration values:" in result.output
            assert "force_ocr: False" in result.output

    def test_config_with_config_file(self) -> None:
        runner = CliRunner()
        with runner.isolated_filesystem():
            config_file = Path("pyproject.toml")
            config_file.write_text("""
[tool.kreuzberg]
force_ocr = true
chunk_content = true
max_chars = 5000
""")

            with patch("kreuzberg.cli.find_config_file", return_value=config_file):
                result = runner.invoke(cli, ["config"])

            assert result.exit_code == 0
            assert "Configuration from:" in result.output
            assert "pyproject.toml" in result.output
            assert '"force_ocr": true' in result.output
            assert '"chunk_content": true' in result.output
            assert '"max_chars": 5000' in result.output

    def test_config_with_specified_file(self) -> None:
        runner = CliRunner()
        with runner.isolated_filesystem():
            config_file = Path("custom_config.toml")
            config_file.write_text("""
[tool.kreuzberg]
extract_tables = true
ocr_backend = "easyocr"
""")

            result = runner.invoke(cli, ["config", "--config", str(config_file)])

            assert result.exit_code == 0
            assert "custom_config.toml" in result.output
            assert '"extract_tables": true' in result.output
            assert '"ocr_backend": "easyocr"' in result.output

    def test_config_error_handling(self) -> None:
        with patch("kreuzberg.cli.load_config_from_file", side_effect=Exception("Config load error")):
            runner = CliRunner()
            with runner.isolated_filesystem():
                config_file = Path("bad_config.toml")
                config_file.write_text("invalid toml")

                result = runner.invoke(cli, ["config", "--config", str(config_file)])

            assert result.exit_code == 1
            assert "Error:" in result.output or "Config load error" in result.output

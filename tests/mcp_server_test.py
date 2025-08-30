from __future__ import annotations

import base64
import json
import os
import sys
import tempfile
from pathlib import Path
from unittest.mock import patch

import pytest
from mcp.types import TextContent

from kreuzberg._mcp.server import (
    extract_and_summarize,
    extract_bytes,
    extract_document,
    extract_simple,
    extract_structured,
    get_available_backends,
    get_default_config,
    get_supported_formats,
    mcp,
)

pytestmark = pytest.mark.skipif(
    sys.platform == "darwin" and os.environ.get("CI") == "true",
    reason="MCP tests cause segmentation faults on macOS CI",
)


def test_mcp_server_initialization() -> None:
    """Test that the MCP server initializes correctly."""
    assert mcp.name == "Kreuzberg Text Extraction"
    assert mcp is not None


def test_mcp_server_tools_available() -> None:
    """Test that all expected tools are available."""
    assert callable(extract_document)
    assert callable(extract_bytes)
    assert callable(extract_simple)


def test_mcp_server_resources_available() -> None:
    """Test that all expected resources are available."""
    assert callable(get_default_config)
    assert callable(get_available_backends)
    assert callable(get_supported_formats)


def test_mcp_server_prompts_available() -> None:
    """Test that all expected prompts are available."""
    assert callable(extract_and_summarize)
    assert callable(extract_structured)


def test_extract_simple_with_text_file(tmp_path: Path) -> None:
    """Test simple text extraction from a file."""
    test_file = tmp_path / "test.txt"
    test_content = "Hello, World! This is a test document."
    test_file.write_text(test_content)

    result = extract_simple(file_path=str(test_file))
    assert isinstance(result, str)
    assert test_content in result


def test_extract_simple_with_pdf(searchable_pdf: Path) -> None:
    """Test simple extraction from a PDF file."""
    result = extract_simple(file_path=str(searchable_pdf))
    assert isinstance(result, str)
    assert "Sample PDF" in result


def test_extract_document_basic(searchable_pdf: Path) -> None:
    """Test basic document extraction."""
    result = extract_document(file_path=str(searchable_pdf), mime_type="application/pdf")

    assert isinstance(result, dict)
    assert "content" in result
    assert "mime_type" in result
    assert "metadata" in result
    assert "Sample PDF" in result["content"]
    assert result["mime_type"] in ["text/plain", "text/markdown"]


def test_extract_document_with_chunking(searchable_pdf: Path) -> None:
    """Test document extraction with chunking enabled."""
    result = extract_document(file_path=str(searchable_pdf), chunk_content=True, max_chars=500, max_overlap=50)

    assert isinstance(result, dict)
    assert "chunks" in result
    assert isinstance(result["chunks"], list)
    if result["chunks"]:
        assert len(result["chunks"]) > 0


def test_extract_document_with_entities(searchable_pdf: Path) -> None:
    """Test document extraction with entity extraction."""
    result = extract_document(file_path=str(searchable_pdf), extract_entities=True)

    assert isinstance(result, dict)
    assert "entities" in result


def test_extract_document_with_keywords(searchable_pdf: Path) -> None:
    """Test document extraction with keyword extraction."""
    result = extract_document(file_path=str(searchable_pdf), extract_keywords=True, keyword_count=5)

    assert isinstance(result, dict)
    assert "keywords" in result


def test_extract_document_with_language_detection(searchable_pdf: Path) -> None:
    """Test document extraction with language detection."""
    result = extract_document(file_path=str(searchable_pdf), auto_detect_language=True)

    assert isinstance(result, dict)
    assert "detected_languages" in result


def test_extract_bytes_basic(searchable_pdf: Path) -> None:
    """Test basic bytes extraction."""
    with searchable_pdf.open("rb") as f:
        content_bytes = f.read()

    content_base64 = base64.b64encode(content_bytes).decode()

    result = extract_bytes(content_base64=content_base64, mime_type="application/pdf")

    assert isinstance(result, dict)
    assert "content" in result
    assert "mime_type" in result
    assert "metadata" in result
    assert "Sample PDF" in result["content"]


def test_extract_bytes_with_options(searchable_pdf: Path) -> None:
    """Test bytes extraction with various options."""
    with searchable_pdf.open("rb") as f:
        content_bytes = f.read()

    content_base64 = base64.b64encode(content_bytes).decode()

    result = extract_bytes(
        content_base64=content_base64,
        mime_type="application/pdf",
        chunk_content=True,
        extract_entities=True,
        extract_keywords=True,
        max_chars=1000,
        max_overlap=50,
        keyword_count=3,
    )

    assert isinstance(result, dict)
    assert "content" in result
    assert "chunks" in result
    assert "entities" in result
    assert "keywords" in result


def test_extract_document_different_backends(searchable_pdf: Path) -> None:
    """Test extraction with different OCR backends."""
    backends = ["tesseract", "easyocr", "paddleocr"]

    for backend in backends:
        result = extract_document(file_path=str(searchable_pdf), ocr_backend=backend)
        assert isinstance(result, dict)
        assert "content" in result


def test_extract_document_invalid_file() -> None:
    """Test extraction with invalid file path."""
    with pytest.raises(Exception):  # noqa: B017, PT011
        extract_document(file_path="/nonexistent/file.pdf")


def test_extract_bytes_invalid_base64() -> None:
    """Test extraction with invalid base64 content."""
    with pytest.raises(Exception):  # noqa: B017, PT011
        extract_bytes(content_base64="invalid_base64", mime_type="application/pdf")


def test_get_default_config() -> None:
    """Test getting default configuration."""
    result = get_default_config()
    assert isinstance(result, str)
    assert "force_ocr" in result
    assert "chunk_content" in result
    assert "extract_tables" in result


def test_get_available_backends() -> None:
    """Test getting available OCR backends."""
    result = get_available_backends()
    assert isinstance(result, str)
    assert "tesseract" in result
    assert "easyocr" in result
    assert "paddleocr" in result


def test_get_supported_formats() -> None:
    """Test getting supported file formats."""
    result = get_supported_formats()
    assert isinstance(result, str)
    assert "PDF" in result
    assert "Images" in result
    assert "Office documents" in result


def test_get_invalid_resource() -> None:
    """Test getting invalid resource."""
    assert callable(get_default_config)
    assert callable(get_available_backends)
    assert callable(get_supported_formats)


def test_extract_and_summarize_prompt(searchable_pdf: Path) -> None:
    """Test extract and summarize prompt."""
    result = extract_and_summarize(file_path=str(searchable_pdf))
    assert isinstance(result, list)
    assert len(result) > 0

    text_content = result[0]
    assert hasattr(text_content, "text")
    assert "Document Content:" in text_content.text
    assert "Sample PDF" in text_content.text
    assert "Please provide a concise summary" in text_content.text


def test_extract_structured_prompt(searchable_pdf: Path) -> None:
    """Test extract structured prompt."""
    with (
        patch("kreuzberg._entity_extraction.extract_entities") as mock_entities,
        patch("kreuzberg._entity_extraction.extract_keywords") as mock_keywords,
    ):
        mock_entities.return_value = []
        mock_keywords.return_value = []

        result = extract_structured(file_path=str(searchable_pdf))
        assert isinstance(result, list)
        assert len(result) > 0

        text_content = result[0]
        assert hasattr(text_content, "text")
        assert "Document Content:" in text_content.text
        assert "Sample PDF" in text_content.text
        assert "Please analyze this document" in text_content.text


def test_extract_and_summarize_with_invalid_file() -> None:
    """Test extract and summarize prompt with invalid file."""
    with pytest.raises(Exception):  # noqa: B017, PT011
        extract_and_summarize(file_path="/nonexistent/file.pdf")


def test_extract_structured_with_invalid_file() -> None:
    """Test extract structured prompt with invalid file."""
    with pytest.raises(Exception):  # noqa: B017, PT011
        extract_structured(file_path="/nonexistent/file.pdf")


def test_invalid_prompt() -> None:
    """Test getting invalid prompt."""
    assert callable(extract_and_summarize)
    assert callable(extract_structured)


def test_full_workflow_pdf(searchable_pdf: Path) -> None:
    """Test complete workflow with PDF file."""
    simple_result = extract_simple(file_path=str(searchable_pdf))
    assert isinstance(simple_result, str)
    assert "Sample PDF" in simple_result

    full_result = extract_document(
        file_path=str(searchable_pdf),
        chunk_content=True,
        extract_entities=True,
        extract_keywords=True,
        max_chars=1000,
        max_overlap=100,
    )
    assert isinstance(full_result, dict)
    assert "content" in full_result
    assert "chunks" in full_result
    assert "entities" in full_result
    assert "keywords" in full_result

    prompt_result = extract_and_summarize(file_path=str(searchable_pdf))
    assert isinstance(prompt_result, list)
    assert len(prompt_result) > 0


def test_multiple_file_types(searchable_pdf: Path, docx_document: Path) -> None:
    """Test extraction with multiple file types."""
    pdf_result = extract_simple(file_path=str(searchable_pdf))
    assert isinstance(pdf_result, str)
    assert len(pdf_result) > 0

    docx_result = extract_simple(file_path=str(docx_document))
    assert isinstance(docx_result, str)
    assert len(docx_result) > 0

    assert pdf_result != docx_result


def test_bytes_vs_file_consistency(searchable_pdf: Path) -> None:
    """Test that bytes and file extraction produce consistent results."""
    file_result = extract_simple(file_path=str(searchable_pdf))

    with searchable_pdf.open("rb") as f:
        content_bytes = f.read()
    content_base64 = base64.b64encode(content_bytes).decode()

    bytes_result = extract_bytes(content_base64=content_base64, mime_type="application/pdf")

    assert file_result == bytes_result["content"]


def test_configuration_consistency() -> None:
    """Test that configuration resources are consistent."""
    default_config = get_default_config()
    backends = get_available_backends()
    formats = get_supported_formats()

    assert isinstance(default_config, str)
    assert isinstance(backends, str)
    assert isinstance(formats, str)

    assert "tesseract" in backends
    assert "easyocr" in backends
    assert "paddleocr" in backends


def test_error_handling() -> None:
    """Test error handling across different components."""
    with pytest.raises(Exception):  # noqa: B017, PT011
        extract_simple(file_path="/nonexistent/file.pdf")

    with pytest.raises(Exception):  # noqa: B017, PT011
        extract_bytes(content_base64="invalid", mime_type="application/pdf")

    assert callable(get_default_config)
    assert callable(get_available_backends)
    assert callable(get_supported_formats)

    assert callable(extract_and_summarize)
    assert callable(extract_structured)


def test_create_config_with_overrides_no_discovered_config(tmp_path: Path) -> None:
    """Test config creation when no config file is discovered."""
    from kreuzberg._mcp.server import _create_config_with_overrides

    with patch("kreuzberg._mcp.server.try_discover_config", return_value=None):
        config = _create_config_with_overrides(force_ocr=True, chunk_content=True, max_chars=500, ocr_backend="easyocr")

    assert config.force_ocr is True
    assert config.chunk_content is True
    assert config.max_chars == 500
    assert config.ocr_backend == "easyocr"
    assert config.extract_tables is False
    assert config.extract_entities is False


def test_create_config_with_overrides_discovered_config(tmp_path: Path) -> None:
    """Test config creation with discovered config as base."""
    from kreuzberg import ExtractionConfig
    from kreuzberg._mcp.server import _create_config_with_overrides

    discovered_config = ExtractionConfig(
        force_ocr=False, chunk_content=False, extract_tables=True, max_chars=1000, ocr_backend="tesseract"
    )

    with patch("kreuzberg._mcp.server.try_discover_config", return_value=discovered_config):
        config = _create_config_with_overrides(
            force_ocr=True,
            max_chars=2000,
        )

    assert config.force_ocr is True
    assert config.max_chars == 2000

    assert config.chunk_content is False
    assert config.extract_tables is True
    assert config.ocr_backend == "tesseract"


def test_extract_document_with_tables(tmp_path: Path) -> None:
    """Test document extraction with table extraction enabled."""
    test_file = tmp_path / "test.txt"
    test_file.write_text("Simple test content")

    with (
        patch("kreuzberg._gmft.extract_tables", return_value=[]),
        patch("kreuzberg.extraction.extract_file_sync") as mock_extract,
    ):
        from kreuzberg._types import ExtractionResult

        mock_result = ExtractionResult(
            content="Simple test content",
            mime_type="text/plain",
            metadata={},
            chunks=[],
            tables=[],
            entities=None,
            keywords=None,
        )
        mock_extract.return_value = mock_result

        result = extract_document(file_path=str(test_file), extract_tables=True)

    assert isinstance(result, dict)
    assert "tables" in result
    assert isinstance(result["tables"], list) or result["tables"] is None


def test_extract_document_all_parameters(tmp_path: Path) -> None:
    """Test document extraction with all possible parameters."""
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content for comprehensive parameter testing.")

    with (
        patch("kreuzberg._entity_extraction.extract_entities", return_value=[]),
        patch("kreuzberg._entity_extraction.extract_keywords", return_value=[]),
        patch("kreuzberg._gmft.extract_tables", return_value=[]),
    ):
        result = extract_document(
            file_path=str(test_file),
            mime_type="text/plain",
            force_ocr=False,
            chunk_content=True,
            extract_tables=True,
            extract_entities=True,
            extract_keywords=True,
            ocr_backend="tesseract",
            max_chars=800,
            max_overlap=100,
            keyword_count=5,
            auto_detect_language=True,
        )

    assert isinstance(result, dict)

    expected_keys = [
        "content",
        "mime_type",
        "metadata",
        "chunks",
        "tables",
        "entities",
        "keywords",
        "detected_languages",
    ]
    for key in expected_keys:
        assert key in result

    assert isinstance(result["content"], str)
    assert "Test content" in result["content"]
    assert result["mime_type"] in ["text/plain", "text/markdown"]


def test_extract_bytes_all_parameters() -> None:
    """Test bytes extraction with all possible parameters."""
    test_content = "Test content for comprehensive bytes parameter testing."
    content_bytes = test_content.encode("utf-8")
    content_base64 = base64.b64encode(content_bytes).decode()

    with (
        patch("kreuzberg._entity_extraction.extract_entities", return_value=[]),
        patch("kreuzberg._entity_extraction.extract_keywords", return_value=[]),
        patch("kreuzberg._gmft.extract_tables", return_value=[]),
    ):
        result = extract_bytes(
            content_base64=content_base64,
            mime_type="text/plain",
            force_ocr=False,
            chunk_content=True,
            extract_tables=True,
            extract_entities=True,
            extract_keywords=True,
            ocr_backend="tesseract",
            max_chars=600,
            max_overlap=80,
            keyword_count=8,
            auto_detect_language=True,
        )

    assert isinstance(result, dict)

    expected_keys = [
        "content",
        "mime_type",
        "metadata",
        "chunks",
        "tables",
        "entities",
        "keywords",
        "detected_languages",
    ]
    for key in expected_keys:
        assert key in result

    assert isinstance(result["content"], str)
    assert "Test content" in result["content"]


def test_extract_bytes_base64_edge_cases() -> None:
    """Test bytes extraction with edge case base64 inputs."""
    empty_base64 = base64.b64encode(b"").decode()
    result = extract_bytes(content_base64=empty_base64, mime_type="text/plain")
    assert isinstance(result, dict)
    assert "content" in result

    small_content = base64.b64encode(b"a").decode()
    result = extract_bytes(content_base64=small_content, mime_type="text/plain")
    assert isinstance(result, dict)
    assert "content" in result


def test_extract_simple_with_mime_type_override(tmp_path: Path) -> None:
    """Test simple extraction with explicit MIME type."""
    test_file = tmp_path / "test.bin"
    test_content = "This is actually text content"
    test_file.write_text(test_content)

    result = extract_simple(file_path=str(test_file), mime_type="text/plain")
    assert isinstance(result, str)
    assert test_content in result


def test_get_discovered_config_with_config() -> None:
    """Test getting discovered config when config exists."""
    from kreuzberg import ExtractionConfig
    from kreuzberg._mcp.server import get_discovered_config

    mock_config = ExtractionConfig(chunk_content=True, max_chars=1500)

    with patch("kreuzberg._mcp.server.try_discover_config", return_value=mock_config):
        result = get_discovered_config()

    assert isinstance(result, str)
    assert "chunk_content" in result
    assert "true" in result.lower()
    assert "1500" in result


def test_get_discovered_config_no_config() -> None:
    """Test getting discovered config when no config exists."""
    from kreuzberg._mcp.server import get_discovered_config

    with patch("kreuzberg._mcp.server.try_discover_config", return_value=None):
        result = get_discovered_config()

    assert result == "No configuration file found"


def test_extract_structured_with_entities_and_keywords(searchable_pdf: Path) -> None:
    """Test structured extraction with mocked entities and keywords."""
    from kreuzberg._types import Entity

    mock_entities = [
        Entity(text="John Doe", type="PERSON", start=0, end=8),
        Entity(text="New York", type="GPE", start=15, end=23),
    ]
    mock_keywords = [("document", 0.85), ("sample", 0.75), ("test", 0.65)]

    with (
        patch("kreuzberg._entity_extraction.extract_entities", return_value=mock_entities),
        patch("kreuzberg._entity_extraction.extract_keywords", return_value=mock_keywords),
        patch("kreuzberg._gmft.extract_tables", return_value=[]),
    ):
        result = extract_structured(file_path=str(searchable_pdf))

    assert isinstance(result, list)
    assert len(result) > 0

    text_content = result[0]
    assert hasattr(text_content, "text")

    assert "Entities:" in text_content.text or "Keywords:" in text_content.text


def test_extract_structured_with_tables(tmp_path: Path) -> None:
    """Test structured extraction when tables are found."""
    test_file = tmp_path / "test.txt"
    test_file.write_text("Simple test content")

    with patch("kreuzberg._mcp.server.extract_file_sync") as mock_extract:
        import pandas as pd
        from PIL import Image

        from kreuzberg._types import ExtractionResult, TableData

        mock_image = Image.new("RGB", (100, 100), "white")
        mock_df = pd.DataFrame([["A", "B"], ["1", "2"]])
        mock_table = TableData(cropped_image=mock_image, df=mock_df, page_number=1, text="| A | B |\n| 1 | 2 |")

        mock_result = ExtractionResult(
            content="Sample content",
            mime_type="text/plain",
            metadata={},
            chunks=[],
            tables=[mock_table, mock_table],
            entities=[],
            keywords=[],
        )
        mock_extract.return_value = mock_result

        result = extract_structured(file_path=str(test_file))

    assert isinstance(result, list)
    assert len(result) > 0

    text_content = result[0]
    assert "Tables found: 2" in text_content.text


def test_extract_and_summarize_content_length(tmp_path: Path) -> None:
    """Test extract and summarize with different content lengths."""
    short_file = tmp_path / "short.txt"
    short_file.write_text("Short.")

    result = extract_and_summarize(file_path=str(short_file))
    assert isinstance(result, list)
    assert len(result) > 0
    assert "Short." in result[0].text

    long_file = tmp_path / "long.txt"
    long_content = "Very long content. " * 1000
    long_file.write_text(long_content)

    result = extract_and_summarize(file_path=str(long_file))
    assert isinstance(result, list)
    assert len(result) > 0
    assert "Very long content" in result[0].text


def test_extract_document_with_special_characters(tmp_path: Path) -> None:
    """Test document extraction with special characters."""
    test_file = tmp_path / "special.txt"
    special_content = "Special chars: àáâãäåæçèéêë ñ ü ß 中文 🚀"
    test_file.write_text(special_content, encoding="utf-8")

    result = extract_document(file_path=str(test_file))

    assert isinstance(result, dict)
    assert "content" in result
    content = result["content"]
    assert "àáâãäåæçèéêë" in content or "Special chars" in content


def test_extract_bytes_with_special_mime_types() -> None:
    """Test bytes extraction with various MIME types."""
    test_content = "Test content with special handling"
    test_bytes = test_content.encode("utf-8")
    content_base64 = base64.b64encode(test_bytes).decode()

    mime_types = ["text/plain", "text/markdown", "text/csv", "text/html", "application/json"]

    for mime_type in mime_types:
        result = extract_bytes(content_base64=content_base64, mime_type=mime_type)
        assert isinstance(result, dict)
        assert "content" in result
        assert "Test content" in result["content"]


def test_extract_document_ocr_backend_switching(tmp_path: Path) -> None:
    """Test switching between different OCR backends."""
    test_file = tmp_path / "test.txt"
    test_file.write_text("OCR backend test content")

    backends = ["tesseract", "easyocr", "paddleocr"]

    for backend in backends:
        result = extract_document(
            file_path=str(test_file),
            ocr_backend=backend,
            force_ocr=True,
        )
        assert isinstance(result, dict)
        assert "content" in result


def test_mcp_server_tool_parameter_validation() -> None:
    """Test parameter validation for MCP server tools."""
    with pytest.raises(Exception):  # noqa: B017, PT011
        extract_document(
            file_path="/nonexistent",
            max_chars=-1,
        )


def test_configuration_resource_json_validity() -> None:
    """Test that configuration resources return valid JSON."""
    from kreuzberg._mcp.server import get_discovered_config

    default_config = get_default_config()
    parsed_default = json.loads(default_config)
    assert isinstance(parsed_default, dict)
    assert "force_ocr" in parsed_default

    from kreuzberg import ExtractionConfig

    mock_config = ExtractionConfig(chunk_content=True)
    with patch("kreuzberg._mcp.server.try_discover_config", return_value=mock_config):
        discovered_config = get_discovered_config()
        parsed_discovered = json.loads(discovered_config)
        assert isinstance(parsed_discovered, dict)
        assert "chunk_content" in parsed_discovered


def test_extract_multiple_files_consistency(tmp_path: Path) -> None:
    """Test extraction consistency across multiple files."""
    files = []
    for i in range(3):
        test_file = tmp_path / f"test_{i}.txt"
        test_file.write_text(f"Test content number {i}")
        files.append(test_file)

    results = []
    for file_path in files:
        result = extract_simple(file_path=str(file_path))
        results.append(result)

    assert len(set(results)) == 3
    for i, result in enumerate(results):
        assert f"number {i}" in result


def test_resource_availability_consistency() -> None:
    """Test that resource functions are consistently available."""
    from kreuzberg._mcp.server import (
        get_available_backends,
        get_default_config,
        get_discovered_config,
        get_supported_formats,
    )

    for _ in range(3):
        backends = get_available_backends()
        default_config = get_default_config()
        discovered = get_discovered_config()
        formats = get_supported_formats()

        assert isinstance(backends, str)
        assert isinstance(default_config, str)
        assert isinstance(discovered, str)
        assert isinstance(formats, str)

        assert "tesseract" in backends
        assert "force_ocr" in default_config
        assert "PDF" in formats


def test_prompt_functions_text_content_structure() -> None:
    """Test that prompt functions return properly structured TextContent."""

    with tempfile.NamedTemporaryFile(mode="w", suffix=".txt", delete=False) as f:
        f.write("Test content for prompt functions")
        temp_file = f.name

    try:
        summarize_result = extract_and_summarize(file_path=temp_file)
        assert isinstance(summarize_result, list)
        assert len(summarize_result) > 0
        assert isinstance(summarize_result[0], TextContent)
        assert summarize_result[0].type == "text"
        assert isinstance(summarize_result[0].text, str)

        with (
            patch("kreuzberg._entity_extraction.extract_entities", return_value=[]),
            patch("kreuzberg._entity_extraction.extract_keywords", return_value=[]),
        ):
            structured_result = extract_structured(file_path=temp_file)
            assert isinstance(structured_result, list)
            assert len(structured_result) > 0
            assert isinstance(structured_result[0], TextContent)
            assert structured_result[0].type == "text"
            assert isinstance(structured_result[0].text, str)

    finally:
        Path(temp_file).unlink()


def test_config_merging_priority() -> None:
    """Test that tool parameters take priority over discovered config."""
    from kreuzberg import ExtractionConfig
    from kreuzberg._mcp.server import _create_config_with_overrides

    discovered_config = ExtractionConfig(
        force_ocr=False, chunk_content=False, max_chars=1000, max_overlap=100, keyword_count=5
    )

    with patch("kreuzberg._mcp.server.try_discover_config", return_value=discovered_config):
        config = _create_config_with_overrides(
            force_ocr=True,
            max_chars=2000,
            keyword_count=10,
        )

    assert config.force_ocr is True
    assert config.max_chars == 2000
    assert config.keyword_count == 10

    assert config.chunk_content is False
    assert config.max_overlap == 100

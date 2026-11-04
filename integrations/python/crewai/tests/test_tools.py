"""Tests for crewai-xberg tools."""

import re
from pathlib import Path
from types import SimpleNamespace

import pytest
from xberg import ExtractedDocument, ExtractionErrorItem, ExtractionResult, IoError

from crewai_xberg.tools import (
    ExtractBatchInput,
    ExtractDocumentInput,
    ExtractMetadataInput,
    XbergExtractBatchTool,
    XbergExtractMetadataTool,
    XbergExtractTool,
    _build_config,
    _format_batch,
    _format_document,
)

FIXTURES_DIR = Path(__file__).parent / "fixtures"
SAMPLE_TXT = str(FIXTURES_DIR / "sample.txt")
MISSING_FILE = "/nonexistent/path/missing.pdf"


def test_extract_input_default_output_format() -> None:
    """Default output_format is markdown."""
    schema = ExtractDocumentInput(file_path=SAMPLE_TXT)
    assert schema.output_format == "markdown"


def test_extract_input_plain_format() -> None:
    """Accepts plain output format."""
    schema = ExtractDocumentInput(file_path=SAMPLE_TXT, output_format="plain")
    assert schema.output_format == "plain"


def test_extract_input_html_format() -> None:
    """Accepts html output format."""
    schema = ExtractDocumentInput(file_path=SAMPLE_TXT, output_format="html")
    assert schema.output_format == "html"


def test_extract_input_invalid_format_rejected() -> None:
    """Invalid output_format is rejected by Pydantic."""
    with pytest.raises(Exception):  # noqa: B017, PT011
        ExtractDocumentInput(file_path=SAMPLE_TXT, output_format="djot")  # type: ignore[arg-type]


def test_extract_input_flags_default_false() -> None:
    """The extraction feature flags default to disabled."""
    schema = ExtractDocumentInput(file_path=SAMPLE_TXT)
    assert (schema.force_ocr, schema.chunk, schema.extract_keywords) == (False, False, False)
    assert (schema.extract_entities, schema.summarize) == (False, False)


def test_batch_input_requires_at_least_one_path() -> None:
    """An empty file_paths list is rejected."""
    with pytest.raises(Exception):  # noqa: B017, PT011
        ExtractBatchInput(file_paths=[])


def test_metadata_input_file_path_required() -> None:
    """file_path is required."""
    with pytest.raises(Exception):  # noqa: B017, PT011
        ExtractMetadataInput()  # type: ignore[call-arg]


def test_build_config_defaults_to_output_format_only() -> None:
    """With no flags set, only output_format is passed through."""
    config = _build_config(ExtractDocumentInput(file_path=SAMPLE_TXT))
    assert config == {"output_format": "markdown"}


def test_build_config_enables_requested_capabilities() -> None:
    """Each flag maps to the matching ExtractionConfig sub-config."""
    options = ExtractDocumentInput(
        file_path=SAMPLE_TXT,
        output_format="plain",
        force_ocr=True,
        chunk=True,
        extract_keywords=True,
        extract_entities=True,
        summarize=True,
    )
    assert _build_config(options) == {
        "output_format": "plain",
        "force_ocr": True,
        "chunking": {},
        "keywords": {},
        "ner": {},
        "summarization": {},
    }


def test_format_document_plain_returns_content_only() -> None:
    """A document with no rich results renders as its raw content."""
    document = ExtractedDocument(content="hello world")
    assert _format_document(document) == "hello world"


def test_format_document_appends_languages_and_tables() -> None:
    """Detected languages and table markdown are surfaced when present.

    The formatter is duck-typed over the extracted-document attributes, so a
    namespace stub deterministically drives the rich-result branches (the native
    result types reject Python-wrapper instances at construction time).
    """
    document = SimpleNamespace(
        content="body text",
        detected_languages=["en", "de"],
        tables=[SimpleNamespace(markdown="| a | b |")],
    )
    rendered = _format_document(document)
    assert "body text" in rendered
    assert "## Detected languages\nen, de" in rendered
    assert "| a | b |" in rendered


def test_format_document_appends_keywords_and_entities() -> None:
    """Extracted keywords and named entities are surfaced when present."""
    document = SimpleNamespace(
        content="body",
        extracted_keywords=[SimpleNamespace(text="invoice"), SimpleNamespace(text="total")],
        entities=[SimpleNamespace(text="Acme", category="ORG")],
        summary=SimpleNamespace(text="A short summary."),
    )
    rendered = _format_document(document)
    assert "## Keywords\ninvoice, total" in rendered
    assert "## Entities\nAcme (ORG)" in rendered
    assert "## Summary\nA short summary." in rendered


def test_format_batch_labels_successes_and_reports_errors() -> None:
    """Successful documents are labelled by source; failures list under Errors."""
    result = ExtractionResult(
        results=[ExtractedDocument(content="good content")],
        errors=[
            ExtractionErrorItem(index=1, code=5, error_type="io", source="bad.pdf", message="boom"),
        ],
    )
    rendered = _format_batch(result, ["good.txt", "bad.pdf"])
    assert "# good.txt\n\ngood content" in rendered
    assert "# Errors" in rendered
    assert "bad.pdf: boom (code 5)" in rendered


def test_extract_tool_name() -> None:
    """Tool has correct name."""
    tool = XbergExtractTool()
    assert tool.name == "Extract Document"


def test_extract_tool_description() -> None:
    """Tool has a description mentioning format support."""
    tool = XbergExtractTool()
    # Assert the SHAPE of the claim, not the number. The count has exactly one owner --
    # the MIME registry, propagated by scripts/sync_supported_counts.py, which rewrites
    # the "Supports N file formats" advertisement in tools.py. Repeating the digit here
    # made this test a second owner, and it drifted the moment the registry went 106 ->
    # 107: the source was updated, the assertion was not, and CI Integrations failed on
    # a number that was never wrong.
    assert re.search(r"Supports \d+ file formats", tool.description)


def test_extract_tool_args_schema() -> None:
    """Tool uses ExtractDocumentInput as args schema."""
    tool = XbergExtractTool()
    assert tool.args_schema is ExtractDocumentInput


async def test_extract_tool_default_markdown() -> None:
    """Default extraction returns markdown content via the async path."""
    tool = XbergExtractTool()
    result = await tool.arun(file_path=SAMPLE_TXT)

    assert isinstance(result, str)
    assert len(result) > 0
    assert "sample document" in result.lower()


async def test_extract_tool_plain_format() -> None:
    """Extraction with plain output format returns plain text."""
    tool = XbergExtractTool()
    result = await tool.arun(file_path=SAMPLE_TXT, output_format="plain")

    assert isinstance(result, str)
    assert "sample document" in result.lower()


async def test_extract_tool_html_format() -> None:
    """Extraction with html output format returns HTML."""
    tool = XbergExtractTool()
    result = await tool.arun(file_path=SAMPLE_TXT, output_format="html")

    assert isinstance(result, str)
    assert len(result) > 0


async def test_extract_tool_content_preserves_section_text() -> None:
    """Extracted content preserves the section headings' text."""
    tool = XbergExtractTool()
    result = await tool.arun(file_path=SAMPLE_TXT, output_format="markdown")

    assert "Section One" in result
    assert "Section Two" in result


async def test_extract_tool_keywords_flag_passes_through() -> None:
    """Requesting keywords keeps extraction working end to end."""
    tool = XbergExtractTool()
    result = await tool.arun(file_path=SAMPLE_TXT, extract_keywords=True)

    assert isinstance(result, str)
    assert "sample document" in result.lower()


def test_extract_tool_sync_run_bridge() -> None:
    """The sync `run` bridge drives the async implementation."""
    tool = XbergExtractTool()
    result = tool.run(file_path=SAMPLE_TXT, output_format="plain")

    assert isinstance(result, str)
    assert "sample document" in result.lower()


async def test_extract_tool_file_not_found() -> None:
    """A missing file surfaces as the public IoError from xberg."""
    tool = XbergExtractTool()
    with pytest.raises(IoError, match=r"^IO error:"):
        await tool.arun(file_path=MISSING_FILE)


def test_batch_tool_name() -> None:
    """Batch tool has the expected name."""
    tool = XbergExtractBatchTool()
    assert tool.name == "Extract Documents (Batch)"


def test_batch_tool_args_schema() -> None:
    """Batch tool uses ExtractBatchInput as args schema."""
    tool = XbergExtractBatchTool()
    assert tool.args_schema is ExtractBatchInput


async def test_batch_tool_extracts_multiple_files() -> None:
    """Batch extraction returns a labelled section for each input file."""
    tool = XbergExtractBatchTool()
    result = await tool.arun(file_paths=[SAMPLE_TXT, SAMPLE_TXT])

    assert result.count(f"# {SAMPLE_TXT}") == 2
    assert result.lower().count("sample document") == 2


async def test_batch_tool_reports_per_file_errors_without_aborting() -> None:
    """A bad input surfaces in the Errors section while good inputs still extract."""
    tool = XbergExtractBatchTool()
    result = await tool.arun(file_paths=[SAMPLE_TXT, MISSING_FILE])

    assert "sample document" in result.lower()
    assert "# Errors" in result
    assert "missing.pdf" in result


def test_batch_tool_sync_run_bridge() -> None:
    """The batch sync `run` bridge drives the async implementation."""
    tool = XbergExtractBatchTool()
    result = tool.run(file_paths=[SAMPLE_TXT])

    assert "sample document" in result.lower()


def test_metadata_tool_name() -> None:
    """Tool has correct name."""
    tool = XbergExtractMetadataTool()
    assert tool.name == "Extract Document Metadata"


def test_metadata_tool_description() -> None:
    """Tool has a description mentioning metadata."""
    tool = XbergExtractMetadataTool()
    assert "metadata" in tool.description.lower()


def test_metadata_tool_args_schema() -> None:
    """Tool uses ExtractMetadataInput as args schema."""
    tool = XbergExtractMetadataTool()
    assert tool.args_schema is ExtractMetadataInput


async def test_metadata_tool_returns_string() -> None:
    """Metadata extraction returns a non-empty string."""
    tool = XbergExtractMetadataTool()
    result = await tool.arun(file_path=SAMPLE_TXT)

    assert isinstance(result, str)
    assert len(result) > 0


async def test_metadata_tool_includes_format_details() -> None:
    """Metadata output includes the format details block."""
    tool = XbergExtractMetadataTool()
    result = await tool.arun(file_path=SAMPLE_TXT)

    # The native Metadata `format` field carries the format_type detail. ~keep
    assert "format_type" in result


async def test_metadata_tool_includes_counts() -> None:
    """Metadata output includes page, table, and image counts."""
    tool = XbergExtractMetadataTool()
    result = await tool.arun(file_path=SAMPLE_TXT)

    assert "pages:" in result
    assert "tables:" in result
    assert "images:" in result


async def test_metadata_tool_file_not_found() -> None:
    """A missing file surfaces as the public IoError from xberg."""
    tool = XbergExtractMetadataTool()
    with pytest.raises(IoError, match=r"^IO error:"):
        await tool.arun(file_path=MISSING_FILE)

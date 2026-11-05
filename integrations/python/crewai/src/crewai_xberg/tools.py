"""Xberg document extraction tools for CrewAI agents."""

import asyncio
from typing import Any, Literal

from crewai.tools import BaseTool
from pydantic import BaseModel, Field
from xberg import ExtractInput, ExtractionResult, extract, extract_batch

OutputFormat = Literal["plain", "markdown", "html"]

# Human-relevant scalar/list metadata fields. `ExtractedDocument.metadata` is a
# native (Rust) object at runtime — not the `xberg.options.Metadata` dataclass —
# so `dataclasses.asdict` raises `TypeError`. Fields are read via `getattr`
# against this allow-list instead. `format` carries the format-type details. ~keep
_METADATA_FIELDS: tuple[str, ...] = (
    "title",
    "subject",
    "authors",
    "keywords",
    "tags",
    "category",
    "language",
    "created_at",
    "modified_at",
    "created_by",
    "modified_by",
    "document_version",
    "output_format",
    "ocr_used",
    "format",
)


class _ExtractionOptions(BaseModel):
    """Extraction options shared by the single-file and batch tools.

    Every field is optional so an agent can call a tool with only the required
    input. Each boolean toggles an ``xberg.ExtractionConfig`` capability; passing
    an empty sub-config enables the feature with its built-in defaults.
    """

    output_format: OutputFormat = Field(
        default="markdown",
        description="Output format for the extracted text: plain, markdown, or html.",
    )
    force_ocr: bool = Field(
        default=False,
        description="Force OCR on every page, even for documents that already contain a text layer.",
    )
    chunk: bool = Field(
        default=False,
        description="Split the document into semantic chunks and report the chunk count.",
    )
    extract_keywords: bool = Field(
        default=False,
        description="Extract salient keywords from the document.",
    )
    extract_entities: bool = Field(
        default=False,
        description="Run named-entity recognition (people, organizations, locations, ...).",
    )
    summarize: bool = Field(
        default=False,
        description="Generate a short summary of the document.",
    )


class ExtractDocumentInput(_ExtractionOptions):
    """Input schema for :class:`XbergExtractTool`.

    Named to avoid a collision with :class:`xberg.ExtractInput`.
    """

    file_path: str = Field(..., description="Path or URL of the document to extract text from.")


class ExtractBatchInput(_ExtractionOptions):
    """Input schema for :class:`XbergExtractBatchTool`."""

    file_paths: list[str] = Field(
        ...,
        min_length=1,
        description="Paths or URLs of the documents to extract in one batched call.",
    )


class ExtractMetadataInput(BaseModel):
    """Input schema for :class:`XbergExtractMetadataTool`."""

    file_path: str = Field(..., description="Path or URL of the document to extract metadata from.")


def _build_config(options: _ExtractionOptions) -> dict[str, Any]:
    """Translate the agent-facing options into an xberg extraction config dict.

    The ``xberg`` Python API accepts a plain dict (nested sub-configs included)
    and coerces it to the native config, so only the requested capabilities are
    set — everything else keeps its xberg default.
    """
    config: dict[str, Any] = {"output_format": options.output_format}
    if options.force_ocr:
        config["force_ocr"] = True
    if options.chunk:
        config["chunking"] = {}
    if options.extract_keywords:
        config["keywords"] = {}
    if options.extract_entities:
        config["ner"] = {}
    if options.summarize:
        config["summarization"] = {}
    return config


def _first_document(result: ExtractionResult, file_path: str) -> Any:
    """Return the first extracted document, raising on errors or empty output.

    A single CrewAI tool call handles exactly one file, so ``extract`` (not
    ``extract_batch``) is the correct entry point and ``results``/``errors``
    each hold at most one item. A missing file raises ``xberg.IoError`` from
    ``extract`` itself; per-input failures (e.g. unsupported formats) instead
    surface in ``result.errors``, which is guarded here.
    """
    if result.errors:
        first = result.errors[0]
        msg = f"Extraction failed for {file_path!r}: {first.message} (code {first.code})"
        raise ValueError(msg)
    if not result.results:
        msg = f"Extraction produced no result for {file_path!r}"
        raise ValueError(msg)
    return result.results[0]


def _format_document(document: Any) -> str:
    """Render an extracted document as text, appending any rich results present.

    The extracted markdown/text always leads. Optional sections (languages,
    keywords, entities, summary, tables, chunk count) are appended only when the
    corresponding data is present, so a plain extraction stays plain.
    """
    parts: list[str] = [document.content]

    languages = getattr(document, "detected_languages", None)
    if languages:
        parts.append("## Detected languages\n" + ", ".join(languages))

    keywords = getattr(document, "extracted_keywords", None)
    if keywords:
        parts.append("## Keywords\n" + ", ".join(keyword.text for keyword in keywords))

    entities = getattr(document, "entities", None)
    if entities:
        seen: dict[str, None] = {}
        for entity in entities:
            seen.setdefault(f"{entity.text} ({entity.category})", None)
        parts.append("## Entities\n" + "\n".join(seen))

    summary = getattr(document, "summary", None)
    if summary is not None:
        parts.append("## Summary\n" + summary.text)

    tables = getattr(document, "tables", None)
    if tables:
        blocks = [table.markdown for table in tables if getattr(table, "markdown", None)]
        if blocks:
            parts.append("## Tables\n" + "\n\n".join(blocks))

    chunks = getattr(document, "chunks", None)
    if chunks:
        parts.append(f"## Chunks\n{len(chunks)} chunk(s) produced.")

    return "\n\n".join(parts)


def _metadata_to_lines(metadata: object) -> list[str]:
    """Flatten a native ``Metadata`` object into ``key: value`` lines.

    Reads a fixed allow-list of fields via ``getattr`` (the runtime object is a
    Rust pyclass, not a dataclass) and merges the ``additional`` provenance dict
    when present. Empty values are skipped.
    """
    lines: list[str] = []
    for name in _METADATA_FIELDS:
        value = getattr(metadata, name, None)
        if value is None or value == []:
            continue
        lines.append(f"{name}: {value}")
    additional = getattr(metadata, "additional", None)
    if isinstance(additional, dict):
        lines.extend(f"{key}: {value}" for key, value in additional.items() if value is not None)
    return lines


class XbergExtractTool(BaseTool):
    """Extract text content from a single document file or URL.

    Supports 107 file formats including PDF, DOCX, XLSX, HTML, and images with
    OCR. The agent picks the output format and can enable OCR, chunking, keyword
    extraction, named-entity recognition, and summarization per call.
    """

    name: str = "Extract Document"
    description: str = (
        "Extract text content from a document file or URL. Supports 107 file formats "
        "including PDF, DOCX, XLSX, HTML, and images with OCR. Optionally force OCR, "
        "chunk the document, or extract keywords, entities, and a summary."
    )
    args_schema: type[BaseModel] = ExtractDocumentInput

    async def _arun(self, **kwargs: Any) -> str:
        # xberg is async-only. `_arun` is CrewAI 1.15's native async hook — it is
        # awaited directly by `arun()` (and by CrewAI's async execution paths),
        # so there is no `asyncio.run` re-entrancy inside a running event loop. ~keep
        options = ExtractDocumentInput(**kwargs)
        result = await extract(ExtractInput(uri=options.file_path), config=_build_config(options))
        return _format_document(_first_document(result, options.file_path))

    def _run(self, **kwargs: Any) -> str:
        return asyncio.run(self._arun(**kwargs))


class XbergExtractBatchTool(BaseTool):
    """Extract text content from several documents in one batched call.

    Uses ``xberg.extract_batch``, which is substantially faster than issuing one
    extraction per file. Successful documents are returned in input order; any
    per-file failures are reported in a trailing ``Errors`` section instead of
    aborting the whole batch.
    """

    name: str = "Extract Documents (Batch)"
    description: str = (
        "Extract text from multiple document files or URLs in a single batched call — "
        "faster than extracting them one at a time. Supports 107 file formats and the "
        "same OCR, chunking, keyword, entity, and summary options as single extraction."
    )
    args_schema: type[BaseModel] = ExtractBatchInput

    async def _arun(self, **kwargs: Any) -> str:
        options = ExtractBatchInput(**kwargs)
        inputs = [ExtractInput(uri=path) for path in options.file_paths]
        result = await extract_batch(inputs, config=_build_config(options))
        return _format_batch(result, options.file_paths)

    def _run(self, **kwargs: Any) -> str:
        return asyncio.run(self._arun(**kwargs))


def _format_batch(result: ExtractionResult, file_paths: list[str]) -> str:
    """Render a batch result: one section per successful document, then errors.

    Batch results preserve input order minus failures, so successful documents
    are re-labelled with their source path by skipping the failed indices. If
    that alignment is ever off, documents fall back to positional labels.
    """
    error_indices = {error.index for error in result.errors}
    success_paths = [path for index, path in enumerate(file_paths) if index not in error_indices]
    aligned = len(success_paths) == len(result.results)

    sections: list[str] = []
    for position, document in enumerate(result.results):
        label = success_paths[position] if aligned else f"Document {position + 1}"
        sections.append(f"# {label}\n\n{_format_document(document)}")

    if result.errors:
        error_lines = [f"- {error.source}: {error.message} (code {error.code})" for error in result.errors]
        sections.append("# Errors\n\n" + "\n".join(error_lines))

    return "\n\n".join(sections) if sections else "Extraction produced no results."


class XbergExtractMetadataTool(BaseTool):
    """Extract metadata from a document file.

    Returns metadata such as title, authors, dates, page count, and format-specific
    details as a formatted string. Supports 107 file formats.
    """

    name: str = "Extract Document Metadata"
    description: str = (
        "Extract metadata from a document file such as title, authors, dates, "
        "page count, and format-specific details. Supports 107 file formats "
        "including PDF, DOCX, XLSX, HTML, images, and more."
    )
    args_schema: type[BaseModel] = ExtractMetadataInput

    async def _arun(self, file_path: str) -> str:
        result = await extract(ExtractInput(uri=file_path))
        document = _first_document(result, file_path)
        lines = _metadata_to_lines(document.metadata)
        lines.append(f"pages: {document.counts.pages}")
        lines.append(f"tables: {document.counts.tables}")
        lines.append(f"images: {document.counts.images}")
        return "\n".join(lines) if lines else "No metadata found."

    def _run(self, file_path: str) -> str:
        return asyncio.run(self._arun(file_path))

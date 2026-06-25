"""Xberg document loader for LangChain."""

from collections.abc import AsyncIterator, Iterator
from pathlib import Path
from typing import Any

from xberg import (
    ExtractionConfig,
    ExtractionResult,
    XbergError,
    batch_extract_files,
    batch_extract_files_sync,
    extract_bytes,
    extract_bytes_sync,
    extract_file,
    extract_file_sync,
)
from langchain_core.document_loaders import BaseLoader
from langchain_core.documents import Document


class XbergLoader(BaseLoader):
    """Load documents using Xberg, supporting 88+ file formats with true async.

    Xberg is a Rust-powered document intelligence library. This loader wraps its
    extraction API to provide LangChain-compatible Documents with rich metadata.

    Examples:
        Load a single file:
            >>> loader = XbergLoader(file_path="document.pdf")
            >>> docs = loader.load()

        Load multiple files:
            >>> loader = XbergLoader(file_path=["a.pdf", "b.docx", "c.txt"])
            >>> docs = loader.load()

        Load from bytes:
            >>> loader = XbergLoader(data=raw_bytes, mime_type="application/pdf")
            >>> docs = loader.load()

        Load a directory:
            >>> loader = XbergLoader(file_path="./docs/", glob="**/*.pdf")
            >>> docs = loader.load()

        Per-page splitting:
            >>> from xberg import ExtractionConfig, PageConfig
            >>> config = ExtractionConfig(pages=PageConfig(extract_pages=True))
            >>> loader = XbergLoader(file_path="document.pdf", config=config)
            >>> docs = loader.load()  # One Document per page

        OCR a scanned document:
            >>> from xberg import ExtractionConfig, OcrConfig
            >>> config = ExtractionConfig(
            ...     force_ocr=True,
            ...     ocr=OcrConfig(backend="tesseract", language="eng"),
            ... )
            >>> loader = XbergLoader(file_path="scan.pdf", config=config)
            >>> docs = loader.load()

        Async loading:
            >>> loader = XbergLoader(file_path="document.pdf")
            >>> docs = await loader.aload()

    """

    def __init__(
        self,
        *,
        file_path: str | Path | list[str | Path] | None = None,
        data: bytes | None = None,
        mime_type: str | None = None,
        glob: str | None = None,
        config: ExtractionConfig | None = None,
    ) -> None:
        """Initialize the XbergLoader.

        Args:
            file_path: File path, list of file paths, or directory path to load.
            data: Raw bytes to extract text from. Mutually exclusive with file_path.
            mime_type: MIME type hint. Required when using data, optional for file_path.
            glob: Glob pattern for directory mode. Defaults to None (matches all files).
            config: Xberg ExtractionConfig for controlling extraction behavior
                (output format, OCR settings, page splitting, etc.).
                Defaults to ExtractionConfig() if not provided.

        Raises:
            ValueError: If neither file_path nor data is provided.
            ValueError: If both file_path and data are provided.
            ValueError: If data is provided without mime_type.

        """
        if file_path is None and data is None:
            msg = "Either 'file_path' or 'data' must be provided."
            raise ValueError(msg)
        if file_path is not None and data is not None:
            msg = "Cannot specify both 'file_path' and 'data'. Use one or the other."
            raise ValueError(msg)
        if data is not None and mime_type is None:
            msg = "'mime_type' is required when using 'data'."
            raise ValueError(msg)

        # Normalize file_path
        if isinstance(file_path, (str, Path)):
            self._file_path: Path | list[Path] | None = Path(file_path)
        elif file_path is not None:
            self._file_path = [Path(p) for p in file_path]
        else:
            self._file_path = None

        self._data = data
        self._mime_type = mime_type
        self._glob = glob
        self._config = config or ExtractionConfig()

    @property
    def _per_page(self) -> bool:
        """Whether per-page splitting is enabled in the config."""
        return self._config.pages is not None and self._config.pages.extract_pages

    def _result_to_documents(self, result: ExtractionResult, source: str) -> Iterator[Document]:
        """Convert an ExtractionResult to one or more LangChain Documents."""
        if self._per_page and result.pages:
            yield from self._pages_to_documents(result, source)
        else:
            metadata = self._build_metadata(result, source)
            page_content = self._assemble_content(result.content, result.tables)
            yield Document(page_content=page_content, metadata=metadata)

    def _build_metadata(self, result: ExtractionResult, source: str) -> dict[str, Any]:
        """Build a flat metadata dict from an ExtractionResult."""
        metadata: dict[str, Any] = {}

        # Flatten Xberg metadata (a TypedDict / plain dict)
        if isinstance(result.metadata, dict):
            metadata.update({k: v for k, v in result.metadata.items() if v is not None})

        # Top-level enrichment
        metadata["mime_type"] = result.mime_type
        if result.quality_score is not None:
            metadata["quality_score"] = result.quality_score
        if result.detected_languages is not None:
            metadata["detected_languages"] = result.detected_languages
        if result.output_format is not None:
            metadata["output_format"] = result.output_format
        metadata["page_count"] = result.get_page_count()

        # Extracted keywords
        if result.extracted_keywords:
            metadata["extracted_keywords"] = [
                {
                    "text": kw.text,
                    "score": kw.score,
                    "algorithm": kw.algorithm,
                }
                for kw in result.extracted_keywords
            ]

        # Tables metadata
        metadata["table_count"] = len(result.tables)
        if result.tables:
            metadata["tables"] = [
                {
                    "cells": table.cells,
                    "markdown": table.markdown,
                    "page_number": table.page_number,
                }
                for table in result.tables
            ]

        # Processing warnings
        if result.processing_warnings:
            metadata["processing_warnings"] = [
                {"source": w.source, "message": w.message} for w in result.processing_warnings
            ]

        metadata["source"] = source

        return metadata

    def _pages_to_documents(
        self,
        result: ExtractionResult,
        source: str,
    ) -> Iterator[Document]:
        """Yield one Document per page from an ExtractionResult."""
        base_metadata = self._build_metadata(result, source)

        for page in result.pages:
            page_metadata = {**base_metadata}

            # Page-specific fields (Xberg uses 1-indexed, LangChain uses 0-indexed)
            page_number: int = page["page_number"]
            page_metadata["page"] = page_number - 1
            if page.get("is_blank") is not None:
                page_metadata["is_blank"] = page["is_blank"]

            # Assemble page content
            page_tables = page.get("tables", [])
            page_content = self._assemble_content(page["content"], page_tables)

            yield Document(page_content=page_content, metadata=page_metadata)

    def _assemble_content(
        self,
        content: str,
        tables: Any,
    ) -> str:
        """Combine text content with table markdown."""
        if not tables:
            return content

        table_parts = [table.markdown if hasattr(table, "markdown") else table.get("markdown", "") for table in tables]
        return "\n\n".join([content, *(m for m in table_parts if m)])

    def _resolve_file_paths(self) -> Iterator[Path]:
        """Resolve file paths for multi-file and directory modes."""
        if isinstance(self._file_path, list):
            yield from self._file_path
        elif isinstance(self._file_path, Path) and self._file_path.is_dir():
            pattern = self._glob or "**/*"
            yield from (p for p in self._file_path.glob(pattern) if p.is_file())

    def _is_single_file(self) -> bool:
        """Whether the loader targets exactly one file (not a list or directory)."""
        return isinstance(self._file_path, Path) and not self._file_path.is_dir()

    @staticmethod
    def _check_batch_result(result: ExtractionResult, path: Path) -> None:
        """Raise XbergError if a batch result represents an extraction failure.

        Xberg v4.x batch extraction embeds per-file errors as metadata["error"] dicts
        rather than raising exceptions, to allow partial batch success.
        """
        error = result.metadata.get("error") if isinstance(result.metadata, dict) else None
        if error is not None:
            error_message = error.get("message", result.content) if isinstance(error, dict) else result.content
            msg = f"Failed to extract '{path}': {error_message}"
            raise XbergError(msg)

    def lazy_load(self) -> Iterator[Document]:
        """Load documents lazily, yielding one Document at a time.

        Yields:
            Document objects with extracted text and metadata.

        """
        config = self._config

        if self._data is not None:
            mime_type: str = self._mime_type  # type: ignore[assignment]  # Validated in __init__
            result = extract_bytes_sync(self._data, mime_type, config=config)
            source = f"bytes://{mime_type}"
            yield from self._result_to_documents(result, source)
        elif self._is_single_file():
            path = self._file_path
            assert isinstance(path, Path)  # noqa: S101
            try:
                result = extract_file_sync(path, mime_type=self._mime_type, config=config)
            except XbergError as exc:
                msg = f"Failed to extract '{path}': {exc}"
                raise type(exc)(msg) from exc
            yield from self._result_to_documents(result, str(path))
        else:
            batch_paths: list[str | Path] = list(self._resolve_file_paths())
            if not batch_paths:
                return
            results = batch_extract_files_sync(batch_paths, config)
            for file_path, result in zip(batch_paths, results, strict=True):
                self._check_batch_result(result, Path(file_path))
                yield from self._result_to_documents(result, str(file_path))

    async def alazy_load(self) -> AsyncIterator[Document]:
        """Load documents asynchronously, yielding one Document at a time.

        Uses Xberg's native async extraction backed by Rust's tokio runtime.

        Yields:
            Document objects with extracted text and metadata.

        """
        config = self._config

        if self._data is not None:
            mime_type: str = self._mime_type  # type: ignore[assignment]  # Validated in __init__
            result = await extract_bytes(self._data, mime_type, config=config)
            source = f"bytes://{mime_type}"
            for doc in self._result_to_documents(result, source):
                yield doc
        elif self._is_single_file():
            path = self._file_path
            assert isinstance(path, Path)  # noqa: S101
            try:
                result = await extract_file(path, mime_type=self._mime_type, config=config)
            except XbergError as exc:
                msg = f"Failed to extract '{path}': {exc}"
                raise type(exc)(msg) from exc
            for doc in self._result_to_documents(result, str(path)):
                yield doc
        else:
            batch_paths: list[str | Path] = list(self._resolve_file_paths())
            if not batch_paths:
                return
            results = await batch_extract_files(batch_paths, config)
            for file_path, result in zip(batch_paths, results, strict=True):
                self._check_batch_result(result, Path(file_path))
                for doc in self._result_to_documents(result, str(file_path)):
                    yield doc

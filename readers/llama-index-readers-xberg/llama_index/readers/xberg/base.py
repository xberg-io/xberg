"""XbergReader — LlamaIndex reader for 88+ document formats.

Wraps xberg's Rust-core extraction engine with true async support,
maximalist metadata, and lossless pipeline persistence.
"""

import json
import logging
from collections.abc import AsyncIterator, Awaitable, Callable, Iterable
from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING, Any, Literal, cast

from llama_index.core.readers.base import BasePydanticReader
from llama_index.core.schema import Document
from llama_index.readers.xberg._config import dict_to_config

if TYPE_CHECKING:
    from llama_index.readers.xberg._types import PageContent
from llama_index.readers.xberg._utils import (
    append_tables,
    build_metadata,
    excluded_keys,
    generate_doc_id,
)
from pydantic import Field, field_serializer, field_validator

from xberg import (
    ExtractionConfig,
    ExtractionResult,
    batch_extract_bytes,
    batch_extract_bytes_sync,
    batch_extract_files,
    batch_extract_files_sync,
    config_to_json,
    extract_bytes,
    extract_bytes_sync,
    extract_file,
    extract_file_sync,
)

logger = logging.getLogger(__name__)


@dataclass(frozen=True, slots=True)
class _ExtractionTask:
    """Describes what to extract after input validation and routing.

    Built by ``_prepare_extractions``; consumed by ``_extract_sync`` and
    ``_extract_async`` which dispatch to the appropriate xberg functions.
    Single-item inputs (including one-element lists) use ``kind="file"`` or
    ``kind="bytes"``; multi-item inputs use the ``_batch`` variants.
    """

    kind: Literal["file", "file_batch", "bytes", "bytes_batch"]
    paths: tuple[Path, ...] = ()
    data_list: tuple[bytes, ...] = ()
    mime_types: tuple[str, ...] = ()


class XbergReader(BasePydanticReader):
    """Reader for 88+ document formats powered by xberg's Rust extraction engine.

    Supports file paths, raw bytes, batch input, per-page splitting,
    and true async via Rust tokio.

    Note:
        This is a local-only reader (``is_remote = False``). Remote/virtual
        filesystems (the ``fs`` parameter used by ``SimpleDirectoryReader``)
        are not supported.

    """

    is_remote: bool = False
    raise_on_error: bool = Field(
        default=False,
        description="If True, propagate xberg exceptions. If False, log warnings and skip failed files.",
    )
    extraction_config: ExtractionConfig | None = Field(
        default=None,
        description="Full xberg ExtractionConfig for controlling output format, "
        "OCR, image extraction, and all other extraction options.",
    )

    @classmethod
    def class_name(cls) -> str:
        """Return the canonical class name used for serialization."""
        return "XbergReader"

    @field_validator("extraction_config", mode="before")
    @classmethod
    def _validate_config(cls, v: ExtractionConfig | dict[str, Any] | None) -> ExtractionConfig | None:
        if v is None:
            return None
        if isinstance(v, dict):
            return dict_to_config(v)
        if isinstance(v, ExtractionConfig):
            return v
        msg = f"Expected ExtractionConfig, dict, or None, got {type(v)}"
        raise ValueError(msg)

    @field_serializer("extraction_config")
    def _serialize_config(self, v: ExtractionConfig | None) -> dict[str, Any] | None:
        if v is None:
            return None
        result: dict[str, Any] = json.loads(config_to_json(v))
        return result

    def _build_config(self) -> ExtractionConfig:
        """Return the ExtractionConfig to use for extraction."""
        return self.extraction_config or ExtractionConfig()

    @staticmethod
    def _prepare_extractions(
        *,
        file_path: str | Path | list[str] | list[Path] | None = None,
        data: bytes | list[bytes] | None = None,
        mime_type: str | list[str] | None = None,
    ) -> _ExtractionTask:
        """Validate inputs and build an extraction task descriptor."""
        if file_path is not None:
            paths = tuple(Path(p) for p in file_path) if isinstance(file_path, list) else (Path(file_path),)
            if len(paths) == 1:
                return _ExtractionTask(kind="file", paths=paths)
            return _ExtractionTask(kind="file_batch", paths=paths)

        if data is not None:
            if isinstance(data, list):
                if not isinstance(mime_type, list) or len(data) != len(mime_type):
                    msg = "data and mime_type must be parallel lists of equal length"
                    raise ValueError(msg)
                return _ExtractionTask(kind="bytes_batch", data_list=tuple(data), mime_types=tuple(mime_type))
            if mime_type is None or isinstance(mime_type, list):
                msg = "mime_type must be a string for single bytes input"
                raise ValueError(msg)
            return _ExtractionTask(kind="bytes", data_list=(data,), mime_types=(mime_type,))

        msg = "Either file_path or data must be provided"
        raise ValueError(msg)

    def load_data(  # noqa: D102
        self,
        file_path: str | Path | list[str] | list[Path] | None = None,
        extra_info: dict[str, Any] | None = None,
        *,
        data: bytes | list[bytes] | None = None,
        mime_type: str | list[str] | None = None,
    ) -> list[Document]:
        return list(
            self.lazy_load_data(
                file_path=file_path,
                extra_info=extra_info,
                data=data,
                mime_type=mime_type,
            )
        )

    def lazy_load_data(  # noqa: D102
        self,
        file_path: str | Path | list[str] | list[Path] | None = None,
        extra_info: dict[str, Any] | None = None,
        *,
        data: bytes | list[bytes] | None = None,
        mime_type: str | list[str] | None = None,
    ) -> Iterable[Document]:
        config = self._build_config()
        results_with_source = self._extract_sync(file_path=file_path, data=data, mime_type=mime_type, config=config)
        yield from self._results_to_documents(results_with_source, extra_info)

    def _extract_sync(
        self,
        *,
        file_path: str | Path | list[str] | list[Path] | None = None,
        data: bytes | list[bytes] | None = None,
        mime_type: str | list[str] | None = None,
        config: ExtractionConfig,
    ) -> list[tuple[ExtractionResult, Path | None, bytes | None]]:
        task = self._prepare_extractions(file_path=file_path, data=data, mime_type=mime_type)
        match task.kind:
            case "file":
                r = self._safe_extract(
                    lambda: extract_file_sync(task.paths[0], config=config),
                    source=str(task.paths[0]),
                )
                return [(r, task.paths[0], None)] if r else []
            case "file_batch":
                results = self._safe_batch_extract(
                    lambda: batch_extract_files_sync(list(task.paths), config=config),
                    sources=[str(p) for p in task.paths],
                )
                return [(r, p, None) for r, p in zip(results, task.paths, strict=True) if r]
            case "bytes":
                r = self._safe_extract(
                    lambda: extract_bytes_sync(task.data_list[0], task.mime_types[0], config=config),
                    source="bytes",
                )
                return [(r, None, task.data_list[0])] if r else []
            case "bytes_batch":
                results = self._safe_batch_extract(
                    lambda: batch_extract_bytes_sync(list(task.data_list), list(task.mime_types), config=config),
                    sources=[f"bytes[{i}]" for i in range(len(task.data_list))],
                )
                return [(r, None, d) for r, d in zip(results, task.data_list, strict=True) if r]
            case _ as unreachable:
                msg = f"Unexpected extraction kind: {unreachable}"
                raise AssertionError(msg)

    def _safe_extract(self, fn: Callable[[], ExtractionResult], source: str) -> ExtractionResult | None:
        try:
            return fn()
        except Exception:
            if self.raise_on_error:
                raise
            logger.warning("Failed to extract %s", source, exc_info=True)
            return None

    def _safe_batch_extract(
        self,
        fn: Callable[[], Iterable[ExtractionResult]],
        sources: list[str],
    ) -> list[ExtractionResult | None]:
        try:
            return list(fn())
        except Exception:
            if self.raise_on_error:
                raise
            logger.warning("Batch extraction failed for %s", ", ".join(sources), exc_info=True)
            return [None] * len(sources)

    @staticmethod
    def _results_to_documents(
        results_with_source: list[tuple[ExtractionResult, Path | None, bytes | None]],
        extra_info: dict[str, Any] | None = None,
    ) -> Iterable[Document]:
        """Yield Documents from extraction results, one per page when pages are present."""
        for result, file_path, data in results_with_source:
            if result.pages:
                for page in result.pages:
                    page = cast("PageContent", page)
                    page_num = page["page_number"]
                    content = append_tables(
                        page["content"],
                        page["tables"],
                    )
                    meta = build_metadata(
                        result=result,
                        file_path=file_path,
                        source="bytes" if data is not None else None,
                        extra_info=extra_info,
                        page_number=page_num,
                    )
                    excl = excluded_keys(meta)
                    yield Document(
                        text=content,
                        id_=generate_doc_id(file_path=file_path, data=data, page_number=page_num),
                        metadata=meta,
                        excluded_llm_metadata_keys=excl,
                        excluded_embed_metadata_keys=excl,
                    )
            else:
                content = append_tables(result.content, result.tables)
                meta = build_metadata(
                    result=result,
                    file_path=file_path,
                    source="bytes" if data is not None else None,
                    extra_info=extra_info,
                )
                excl = excluded_keys(meta)
                yield Document(
                    text=content,
                    id_=generate_doc_id(file_path=file_path, data=data),
                    metadata=meta,
                    excluded_llm_metadata_keys=excl,
                    excluded_embed_metadata_keys=excl,
                )

    async def aload_data(  # noqa: D102
        self,
        file_path: str | Path | list[str] | list[Path] | None = None,
        extra_info: dict[str, Any] | None = None,
        *,
        data: bytes | list[bytes] | None = None,
        mime_type: str | list[str] | None = None,
    ) -> list[Document]:
        return [
            doc
            async for doc in self.alazy_load_data(
                file_path=file_path, extra_info=extra_info, data=data, mime_type=mime_type
            )
        ]

    async def alazy_load_data(  # type: ignore[override]  # noqa: D102
        self,
        file_path: str | Path | list[str] | list[Path] | None = None,
        extra_info: dict[str, Any] | None = None,
        *,
        data: bytes | list[bytes] | None = None,
        mime_type: str | list[str] | None = None,
    ) -> AsyncIterator[Document]:
        config = self._build_config()
        results_with_source = await self._extract_async(
            file_path=file_path, data=data, mime_type=mime_type, config=config
        )
        for doc in self._results_to_documents(results_with_source, extra_info):
            yield doc

    async def _extract_async(
        self,
        *,
        file_path: str | Path | list[str] | list[Path] | None = None,
        data: bytes | list[bytes] | None = None,
        mime_type: str | list[str] | None = None,
        config: ExtractionConfig,
    ) -> list[tuple[ExtractionResult, Path | None, bytes | None]]:
        task = self._prepare_extractions(file_path=file_path, data=data, mime_type=mime_type)
        match task.kind:
            case "file":
                r = await self._safe_extract_async(
                    extract_file(task.paths[0], config=config),
                    source=str(task.paths[0]),
                )
                return [(r, task.paths[0], None)] if r else []
            case "file_batch":
                results = await self._safe_batch_extract_async(
                    batch_extract_files(list(task.paths), config=config),
                    sources=[str(p) for p in task.paths],
                )
                return [(r, p, None) for r, p in zip(results, task.paths, strict=True) if r]
            case "bytes":
                r = await self._safe_extract_async(
                    extract_bytes(task.data_list[0], task.mime_types[0], config=config),
                    source="bytes",
                )
                return [(r, None, task.data_list[0])] if r else []
            case "bytes_batch":
                results = await self._safe_batch_extract_async(
                    batch_extract_bytes(list(task.data_list), list(task.mime_types), config=config),
                    sources=[f"bytes[{i}]" for i in range(len(task.data_list))],
                )
                return [(r, None, d) for r, d in zip(results, task.data_list, strict=True) if r]
            case _ as unreachable:
                msg = f"Unexpected extraction kind: {unreachable}"
                raise AssertionError(msg)

    async def _safe_extract_async(self, coro: Awaitable[ExtractionResult], source: str) -> ExtractionResult | None:
        try:
            return await coro
        except Exception:
            if self.raise_on_error:
                raise
            logger.warning("Failed to extract %s", source, exc_info=True)
            return None

    async def _safe_batch_extract_async(
        self,
        coro: Awaitable[Iterable[ExtractionResult]],
        sources: list[str],
    ) -> list[ExtractionResult | None]:
        try:
            return list(await coro)
        except Exception:
            if self.raise_on_error:
                raise
            logger.warning("Batch extraction failed for %s", ", ".join(sources), exc_info=True)
            return [None] * len(sources)

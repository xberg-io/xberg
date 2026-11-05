"""Base ingester and shared helpers for document ingestion."""

import hashlib
import mimetypes
from abc import ABC, abstractmethod
from collections.abc import Sequence
from datetime import UTC, datetime
from pathlib import Path
from typing import Any, Protocol, runtime_checkable

from anyio import Path as AsyncPath
from surrealdb import RecordID, Value
from surrealdb.errors import ServerError
from xberg import ExtractedDocument, ExtractInput, ExtractionConfig, ExtractionResult, Metadata, extract, extract_batch

from surrealdb_xberg.exceptions import DimensionMismatchError, IngestionError, SchemaNotInitializedError
from surrealdb_xberg.types import DocumentRecord

_DEFAULT_MIME_TYPE = "application/octet-stream"


@runtime_checkable
class AsyncSurrealQueryable(Protocol):
    """Protocol for any async SurrealDB object that can execute queries.

    Satisfied by connections (AsyncWsSurrealConnection, AsyncHttpSurrealConnection,
    AsyncEmbeddedSurrealConnection), transactions (AsyncSurrealTransaction),
    and sessions returned by the AsyncSurreal factory.
    """

    async def query(self, query: str, vars: dict[str, Value] | None = None) -> Value: ...  # noqa: A002


def _content_hash(content: str) -> str:
    """Compute SHA-256 hash of content for dedup."""
    return hashlib.sha256(content.encode()).hexdigest()


def _parse_datetime(value: Any) -> datetime | None:
    """Parse a datetime value from metadata, returning None if invalid.

    Args:
        value: A datetime, ISO-format string, or None.

    Returns:
        A timezone-aware datetime, or None if the value is missing or unparseable.

    """
    if value is None:
        return None
    if isinstance(value, datetime):
        return value
    if isinstance(value, str):
        try:
            dt = datetime.fromisoformat(value)
        except ValueError:
            return None
        else:
            return dt if dt.tzinfo else dt.replace(tzinfo=UTC)
    return None


def _guess_mime_type(path: str) -> str:
    """Guess the MIME type of a file from its name, falling back to a generic type.

    Args:
        path: File path or name to inspect.

    Returns:
        The guessed MIME type, or ``application/octet-stream`` when unknown.
        Xberg performs its own content sniffing, so the fallback is safe.

    """
    mime_type, _ = mimetypes.guess_type(path)
    return mime_type or _DEFAULT_MIME_TYPE


async def _input_from_path(path: str | Path) -> ExtractInput:
    """Build an ``ExtractInput`` from a local file by reading its bytes.

    Reads the file into memory and constructs a bytes-based input. This avoids
    Xberg's local-file/``file://`` URI gating (which requires explicit allow
    flags) while remaining explicit about the source's MIME type and filename.

    Args:
        path: Path to the local file.

    Returns:
        An ``ExtractInput`` carrying the file bytes, guessed MIME type, and name.

    """
    data = await AsyncPath(path).read_bytes()
    return ExtractInput(
        kind="bytes",
        bytes=data,
        mime_type=_guess_mime_type(str(path)),
        filename=Path(path).name,
    )


def _metadata_to_dict(metadata: Metadata) -> dict[str, Any]:
    """Convert a Xberg ``Metadata`` object into a plain dict for SurrealDB.

    ``Metadata`` is an attribute-based object (not a mapping), so the document's
    ``metadata`` column is built explicitly from its useful fields. ``None``
    values are dropped to keep the stored object compact.

    Args:
        metadata: The ``Metadata`` object from an ``ExtractedDocument``.

    Returns:
        A dict of populated metadata fields.

    """
    fields: dict[str, Any] = {
        "title": metadata.title,
        "subject": metadata.subject,
        "authors": metadata.authors,
        "keywords": metadata.keywords,
        "language": metadata.language,
        "created_at": metadata.created_at,
        "modified_at": metadata.modified_at,
        "created_by": metadata.created_by,
        "modified_by": metadata.modified_by,
        "category": metadata.category,
        "tags": metadata.tags,
        "additional": metadata.additional or None,
    }
    return {key: value for key, value in fields.items() if value is not None}


def _entities_to_list(document: ExtractedDocument) -> list[dict[str, Any]]:
    """Serialize the document's named entities (NER) into plain dicts.

    Each entity's ``category`` is a native ``EntityCategory`` object, so it is
    stringified (e.g. ``"person"``) for storage. Persisting entities as a
    first-class column lets callers filter and traverse them for graph search.

    Args:
        document: The extracted document carrying the entities.

    Returns:
        A list of ``{category, text, start, end, confidence}`` dicts, empty when
        NER was not run.

    """
    return [
        {
            "category": str(entity.category),
            "text": entity.text,
            "start": entity.start,
            "end": entity.end,
            "confidence": entity.confidence,
        }
        for entity in document.entities or []
    ]


def _tables_to_list(document: ExtractedDocument) -> list[dict[str, Any]]:
    """Serialize the document's extracted tables into plain dicts.

    Args:
        document: The extracted document carrying the tables.

    Returns:
        A list of ``{markdown, page_number, cells}`` dicts, empty when the
        document has no tables.

    """
    return [
        {
            "markdown": table.markdown,
            "page_number": table.page_number,
            "cells": table.cells,
        }
        for table in document.tables or []
    ]


def _map_result_to_doc(document: ExtractedDocument, source: str, table: str) -> DocumentRecord:
    """Map an ``ExtractedDocument`` to a SurrealDB document record.

    Args:
        document: A single extracted document (``ExtractionResult.results[i]``).
        source: Identifier for the document origin (e.g. file path).
        table: SurrealDB table name, used to build the deterministic RecordID.

    Returns:
        A dict ready for INSERT into SurrealDB, keyed by ``RecordID(table, content_hash)``.

    """
    content_hash = _content_hash(document.content)
    metadata = document.metadata
    authors = metadata.authors
    keywords = document.extracted_keywords
    summary = document.summary
    return {
        "id": RecordID(table, content_hash),
        "source": source,
        "content": document.content,
        "mime_type": document.mime_type,
        "title": metadata.title,
        "authors": ", ".join(authors) if authors else None,
        "created_at": _parse_datetime(metadata.created_at),
        "metadata": _metadata_to_dict(metadata),
        "quality_score": document.quality_score,
        "content_hash": content_hash,
        "detected_languages": document.detected_languages or [],
        "keywords": [kw.text for kw in keywords] if keywords else [],
        "summary": summary.text if summary is not None else None,
        "entities": _entities_to_list(document),
        "tables": _tables_to_list(document),
    }


def _first_document(result: ExtractionResult, source: str) -> ExtractedDocument:
    """Unwrap the single document from an ``ExtractionResult`` container.

    Args:
        result: The container returned by ``extract``.
        source: Identifier for the document origin, used in error messages.

    Returns:
        The first (and only) ``ExtractedDocument``.

    Raises:
        IngestionError: If extraction produced no document.

    """
    if result.results:
        return result.results[0]
    message = result.errors[0].message if result.errors else "no document produced"
    raise IngestionError(f"extraction of {source}", message)


def _pair_documents(result: ExtractionResult, sources: Sequence[str]) -> list[tuple[ExtractedDocument, str]]:
    """Pair each extracted document with its source, accounting for failures.

    ``extract_batch`` returns only the successfully extracted documents in
    ``results`` and reports failures in ``errors`` (each carrying the input
    ``index``). This walks the inputs in order, skipping failed indices, so the
    surviving documents stay aligned with their originating sources.

    Args:
        result: The container returned by ``extract_batch``.
        sources: The source identifiers, in the same order as the batch inputs.

    Returns:
        A list of ``(document, source)`` pairs for every successful extraction.

    """
    failed_indices = {item.index for item in result.errors}
    documents = iter(result.results)
    pairs: list[tuple[ExtractedDocument, str]] = []
    for index, source in enumerate(sources):
        if index in failed_indices:
            continue
        document = next(documents, None)
        if document is None:
            break
        pairs.append((document, source))
    return pairs


def _check_insert_result(result: Value, *, context: str) -> None:
    """Check INSERT IGNORE results for silent errors and raise if found.

    SurrealDB's INSERT IGNORE swallows certain errors — returning error strings
    in the result list instead of raising exceptions. This catches dimension
    mismatches and other silent failures that would otherwise leave tables
    empty with no user-visible error.

    Args:
        result: The raw return value from ``client.query()`` for an INSERT IGNORE.
        context: A human-readable label (e.g. ``"document insertion"``) included
            in error messages.

    Raises:
        DimensionMismatchError: If the error indicates a vector dimension conflict.
        IngestionError: If the result list contains other error strings.

    """
    if not isinstance(result, list):
        return
    errors = [item for item in result if isinstance(item, str)]
    if not errors:
        return

    dim_errors = [e for e in errors if "dimension" in e.lower()]
    if dim_errors:
        raise DimensionMismatchError(context, dim_errors[0])

    raise IngestionError(context, errors[0])


def _raise_from_server_error(error: ServerError, *, context: str) -> None:
    """Translate a SurrealDB ``ServerError`` into a package-specific exception.

    SurrealDB 2.0 raises a structured ``ServerError`` when a statement returns
    ``status: "ERR"``, rather than embedding an error string in the result list.
    The message is inspected to distinguish a vector dimension conflict from a
    generic ingestion failure.

    Args:
        error: The ``ServerError`` raised by ``client.query()``.
        context: A human-readable label (e.g. ``"document insertion"``).

    Raises:
        DimensionMismatchError: If the message indicates a vector dimension conflict.
        IngestionError: For any other server error.

    """
    message = str(error)
    if "dimension" in message.lower():
        raise DimensionMismatchError(context, message) from error
    raise IngestionError(context, message) from error


async def _execute_insert(
    client: AsyncSurrealQueryable,
    query: str,
    records: list[Any],
    *,
    context: str,
) -> None:
    """Run an ``INSERT IGNORE`` and normalize both failure modes into exceptions.

    Wraps the query in a ``ServerError`` guard (SurrealDB 2.0 raises before
    returning) and, on success, still runs the defensive string-scanning fallback
    for older/embedded engines that swallow errors into the result list.

    Args:
        client: The SurrealDB async connection.
        query: The parameterized ``INSERT IGNORE`` statement.
        records: The record rows to bind as ``$records``.
        context: A human-readable label for error messages.

    Raises:
        DimensionMismatchError: On a vector dimension conflict.
        IngestionError: On any other ingestion failure.

    """
    try:
        result = await client.query(query, {"records": records})
    except ServerError as error:
        _raise_from_server_error(error, context=context)
    else:
        _check_insert_result(result, context=context)


async def _collect_files(directory: str | Path, glob: str) -> list[Path]:
    """Collect matching file paths from a directory.

    Args:
        directory: Root directory to search.
        glob: Glob pattern for file matching (e.g. ``"**/*.pdf"``).

    Returns:
        Sorted list of matching file paths.

    """
    root = await AsyncPath(directory).resolve()
    results: list[Path] = []
    async for p in root.glob(glob):
        if await p.is_file() and (await p.resolve()).is_relative_to(root):
            results.append(Path(p))  # noqa: PERF401
    return sorted(results)


class BaseIngester(ABC):
    """Abstract base for document ingestion into SurrealDB.

    Provides shared constructor, properties, batched insertion, and the four
    ``ingest_*`` entry points.  Subclasses implement ``_ingest_batch`` to
    control how extracted documents are mapped and stored. Single-document
    entry points route through ``_ingest_batch`` as one-element batches.
    """

    def __init__(
        self,
        *,
        db: AsyncSurrealQueryable,
        table: str = "documents",
        config: ExtractionConfig | None = None,
    ) -> None:
        """Initialize the ingester.

        Args:
            db: An active SurrealDB async connection.
            table: Name of the documents table.
            config: Optional Xberg ExtractionConfig for extraction tuning.

        """
        self._client = db
        self._table = table
        self._config = config
        self._schema_ready = False

    @property
    def client(self) -> AsyncSurrealQueryable:
        """The underlying SurrealDB connection."""
        return self._client

    @property
    def table(self) -> str:
        """The documents table name."""
        return self._table

    def _require_schema(self) -> None:
        """Raise if setup_schema() has not been called."""
        if not self._schema_ready:
            raise SchemaNotInitializedError

    @abstractmethod
    async def _ingest_batch(self, documents: list[tuple[ExtractedDocument, str]]) -> None:
        """Map and store a batch of extracted documents.

        Args:
            documents: ``(document, source)`` pairs to persist. Single-document
                entry points pass a one-element list.

        """

    async def ingest_file(self, path: str | Path) -> None:
        """Extract and ingest a single file.

        Args:
            path: Path to the file to extract and store.

        """
        self._require_schema()
        result = await extract(await _input_from_path(path), self._config)
        document = _first_document(result, str(path))
        await self._ingest_batch([(document, str(path))])

    async def ingest_files(self, paths: Sequence[str | Path]) -> None:
        """Extract and ingest multiple files in a single batched extraction.

        Uses one ``extract_batch`` call for all inputs (avoiding the per-file
        N+1 extraction) and lets ``_ingest_batch`` group the resulting rows.

        Args:
            paths: Sequence of file paths to extract and store.

        """
        self._require_schema()
        sources = [str(path) for path in paths]
        if not sources:
            return
        inputs = [await _input_from_path(source) for source in sources]
        result = await extract_batch(inputs, self._config)
        await self._ingest_batch(_pair_documents(result, sources))

    async def ingest_directory(self, directory: str | Path, *, glob: str = "**/*") -> None:
        """Extract and ingest all matching files in a directory.

        Args:
            directory: Root directory to search.
            glob: Glob pattern for file matching. Defaults to all files recursively.

        """
        self._require_schema()
        await self.ingest_files(await _collect_files(directory, glob))

    async def ingest_bytes(self, *, data: bytes, mime_type: str, source: str) -> None:
        """Extract and ingest from raw bytes.

        Args:
            data: Raw file content.
            mime_type: MIME type of the data (e.g. ``"application/pdf"``).
            source: Identifier for the document origin.

        """
        self._require_schema()
        result = await extract(ExtractInput(kind="bytes", bytes=data, mime_type=mime_type), self._config)
        document = _first_document(result, source)
        await self._ingest_batch([(document, source)])

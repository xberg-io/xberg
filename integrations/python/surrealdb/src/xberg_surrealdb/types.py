"""Typed record structures for SurrealDB document and chunk records."""

from datetime import datetime
from typing import Any, TypedDict

from surrealdb import RecordID


class DocumentRecord(TypedDict):
    """Shape of a document record inserted into SurrealDB."""

    id: RecordID
    source: str
    content: str
    mime_type: str
    title: str | None
    authors: str | None
    created_at: datetime | None
    metadata: dict[str, Any]
    quality_score: float | None
    content_hash: str
    detected_languages: list[str]
    keywords: list[str]


class ChunkRecord(TypedDict):
    """Shape of a chunk record inserted into SurrealDB."""

    id: RecordID
    document: RecordID
    content: str
    chunk_index: int
    embedding: list[float] | None
    word_count: int
    page_number: int | None
    char_start: int | None
    char_end: int | None
    first_page: int | None
    last_page: int | None

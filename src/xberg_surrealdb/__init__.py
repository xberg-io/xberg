"""Xberg-to-SurrealDB connector for zero-dependency RAG pipelines."""

from xberg_surrealdb._base import AsyncSurrealQueryable
from xberg_surrealdb.connector import DocumentConnector
from xberg_surrealdb.exceptions import DimensionMismatchError, IngestionError, SchemaNotInitializedError
from xberg_surrealdb.pipeline import DocumentPipeline
from xberg_surrealdb.types import ChunkRecord, DocumentRecord

__all__ = [
    "AsyncSurrealQueryable",
    "ChunkRecord",
    "DimensionMismatchError",
    "DocumentConnector",
    "DocumentPipeline",
    "DocumentRecord",
    "IngestionError",
    "SchemaNotInitializedError",
]

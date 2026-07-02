"""Chunked extraction with optional embedding for RAG pipelines."""

from typing import TYPE_CHECKING, Any, cast

from surrealdb import RecordID
from xberg import (
    ChunkingConfig,
    EmbeddingConfig,
    EmbeddingModelType,
    ExtractionConfig,
    ExtractionResult,
    extract_bytes,
    get_embedding_preset,
)

from xberg_surrealdb._base import (
    AsyncSurrealQueryable,
    BaseIngester,
    _check_insert_result,
    _content_hash,
    _map_result_to_doc,
)
from xberg_surrealdb.schema import build_pipeline_schema

if TYPE_CHECKING:
    from xberg_surrealdb.types import ChunkRecord


class DocumentPipeline(BaseIngester):
    """Chunked extraction with optional embedding for RAG pipelines."""

    def __init__(
        self,
        *,
        db: AsyncSurrealQueryable,
        table: str = "documents",
        insert_batch_size: int = 100,
        chunk_table: str = "chunks",
        config: ExtractionConfig | None = None,
        embed: bool = True,
        embedding_model: str | EmbeddingModelType = "balanced",
        embedding_dimensions: int | None = None,
    ) -> None:
        """Initialize the pipeline.

        Args:
            db: An active SurrealDB async connection.
            table: Name of the documents table.
            insert_batch_size: Max records per INSERT IGNORE batch.
            chunk_table: Name of the chunks table.
            config: Optional Xberg ExtractionConfig. If it includes a
                ChunkingConfig, the chunking parameters are preserved and only
                the embedding config is injected.
            embed: Whether to generate embeddings for vector search.
            embedding_model: Preset name (e.g. ``"balanced"``, ``"fast"``) or
                an ``EmbeddingModelType`` instance.
            embedding_dimensions: Vector dimensions. Required when passing an
                ``EmbeddingModelType`` directly; inferred from presets otherwise.

        Raises:
            ValueError: If the embedding preset is unknown or dimensions are
                missing for a custom model type.

        """
        super().__init__(db=db, table=table, config=config)
        self._insert_batch_size = insert_batch_size
        self._chunk_table = chunk_table
        self._embed = embed

        if isinstance(embedding_model, str):
            preset_info = get_embedding_preset(embedding_model)
            if preset_info is None:
                msg = f"Unknown embedding preset: {embedding_model}"
                raise ValueError(msg)
            self._embedding_dimensions: int = embedding_dimensions or preset_info.dimensions
            self._embedding_model_type: EmbeddingModelType = EmbeddingModelType.preset(embedding_model)
        else:
            if embedding_dimensions is None:
                msg = "embedding_dimensions is required when passing an EmbeddingModelType directly"
                raise ValueError(msg)
            self._embedding_dimensions = embedding_dimensions
            self._embedding_model_type = embedding_model

        self._config = self._build_extraction_config()

    @property
    def chunk_table(self) -> str:
        """The chunks table name."""
        return self._chunk_table

    @property
    def embedding_dimensions(self) -> int:
        """The vector embedding dimensions."""
        return self._embedding_dimensions

    def _build_extraction_config(self) -> ExtractionConfig:
        """Build ExtractionConfig with chunking and optional embedding.

        If the user provided an ExtractionConfig with a ChunkingConfig,
        preserve their chunking parameters (max_chars, max_overlap) and
        only inject the embedding configuration.

        Returns:
            A fully configured ExtractionConfig with chunking (and optionally
            embedding) enabled.

        """
        embedding = EmbeddingConfig(model=self._embedding_model_type) if self._embed else None

        if self._config is not None and self._config.chunking is not None:
            user_chunking = self._config.chunking
            self._config.chunking = ChunkingConfig(
                max_chars=user_chunking.max_chars,
                max_overlap=user_chunking.max_overlap,
                preset=user_chunking.preset,
                embedding=embedding,
            )
            return self._config

        chunking = ChunkingConfig(embedding=embedding)
        if self._config is not None:
            self._config.chunking = chunking
            return self._config

        return ExtractionConfig(chunking=chunking)

    async def setup_schema(
        self,
        *,
        analyzer_language: str = "english",
        bm25_k1: float = 1.2,
        bm25_b: float = 0.75,
        distance_metric: str = "COSINE",
        hnsw_efc: int = 150,
        hnsw_m: int = 12,
    ) -> None:
        """Create documents + chunks tables with BM25 and HNSW indexes.

        Args:
            analyzer_language: Snowball stemmer language for the BM25 analyzer.
            bm25_k1: BM25 term-frequency saturation parameter.
            bm25_b: BM25 document-length normalization parameter.
            distance_metric: HNSW distance function (e.g. ``"COSINE"``, ``"EUCLIDEAN"``).
            hnsw_efc: HNSW construction-time search width (higher = slower build, better recall).
            hnsw_m: HNSW max edges per node (higher = more memory, better recall).

        """
        stmts = build_pipeline_schema(
            table=self._table,
            chunk_table=self._chunk_table,
            embed=self._embed,
            embedding_dimension=self._embedding_dimensions,
            analyzer_language=analyzer_language,
            bm25_k1=bm25_k1,
            bm25_b=bm25_b,
            distance_metric=distance_metric,
            hnsw_efc=hnsw_efc,
            hnsw_m=hnsw_m,
        )
        for stmt in stmts:
            await self._client.query(stmt)
        self._schema_ready = True

    async def _ingest_result(self, result: ExtractionResult, source: str) -> None:
        """Extract, store document, then store chunks with record links.

        Both documents and chunks use deterministic record IDs and INSERT IGNORE,
        making the entire pipeline idempotent and resilient to partial failures.

        Args:
            result: The extraction result from Xberg, including chunks.
            source: Identifier for the document origin (e.g. file path).

        """
        table = self._table
        content_hash = _content_hash(result.content)
        doc = _map_result_to_doc(result, source, table)
        doc_id = doc["id"]

        res = await self._client.query(
            f"INSERT IGNORE INTO {table} $records",
            {"records": cast("Any", [doc])},
        )
        _check_insert_result(res, context="document insertion")

        chunk_records: list[ChunkRecord] = []
        for i, chunk in enumerate(result.chunks or []):
            meta = chunk.metadata
            first_page = meta.get("first_page")
            last_page = meta.get("last_page")
            chunk_rec: ChunkRecord = {
                "id": RecordID(self._chunk_table, f"{content_hash}_{i}"),
                "document": doc_id,
                "content": chunk.content,
                "chunk_index": i,
                "embedding": chunk.embedding if self._embed else None,
                "word_count": len(chunk.content.split()),
                "page_number": first_page,
                "char_start": None,
                "char_end": None,
                "first_page": first_page,
                "last_page": last_page,
            }
            chunk_records.append(chunk_rec)

        ct = self._chunk_table
        if chunk_records:
            for i in range(0, len(chunk_records), self._insert_batch_size):
                batch = chunk_records[i : i + self._insert_batch_size]
                res = await self._client.query(
                    f"INSERT IGNORE INTO {ct} $records",
                    {"records": cast("Any", batch)},
                )
                _check_insert_result(res, context="chunk insertion")

    async def embed_query(self, query: str) -> list[float]:
        """Embed a query string using xberg's extraction pipeline.

        Args:
            query: The text to embed.

        Returns:
            The embedding vector as a list of floats.

        Raises:
            RuntimeError: If Xberg returns no embedding for the query.

        """
        result = await extract_bytes(query.encode(), "text/plain", config=self._config)
        if not result.chunks or result.chunks[0].embedding is None:
            msg = "Embedding generation failed: no embedding returned for query"
            raise RuntimeError(msg)
        embedding: list[float] = result.chunks[0].embedding
        return embedding

"""Tests for DocumentPipeline."""

from unittest.mock import AsyncMock, MagicMock, patch

import pytest
from xberg import Chunk, ExtractionResult

from xberg_surrealdb._base import _check_insert_result
from xberg_surrealdb.exceptions import DimensionMismatchError, IngestionError, SchemaNotInitializedError
from xberg_surrealdb.pipeline import DocumentPipeline


def test_pipeline_defaults(mock_client: AsyncMock) -> None:
    pipeline = DocumentPipeline(db=mock_client)
    assert pipeline._embed is True
    assert pipeline.chunk_table == "chunks"
    assert pipeline.embedding_dimensions == 768


def test_pipeline_custom_embedding_preset(mock_client: AsyncMock) -> None:
    pipeline = DocumentPipeline(db=mock_client, embedding_model="fast")
    assert pipeline.embedding_dimensions == 384


def test_pipeline_invalid_embedding_preset(mock_client: AsyncMock) -> None:
    with pytest.raises(ValueError, match="Unknown embedding preset"):
        DocumentPipeline(db=mock_client, embedding_model="nonexistent")


def test_pipeline_embedding_model_type_direct(mock_client: AsyncMock) -> None:
    from xberg import EmbeddingModelType

    model = EmbeddingModelType.preset("BGEBaseENV15")
    pipeline = DocumentPipeline(db=mock_client, embedding_model=model, embedding_dimensions=768)
    assert pipeline.embedding_dimensions == 768


def test_pipeline_embedding_model_type_requires_dimensions(mock_client: AsyncMock) -> None:
    from xberg import EmbeddingModelType

    model = EmbeddingModelType.preset("BGEBaseENV15")
    with pytest.raises(ValueError, match="embedding_dimensions is required"):
        DocumentPipeline(db=mock_client, embedding_model=model)


def test_pipeline_embed_false(mock_client: AsyncMock) -> None:
    pipeline = DocumentPipeline(db=mock_client, embed=False)
    assert pipeline._embed is False


def test_pipeline_custom_chunk_table(mock_client: AsyncMock) -> None:
    pipeline = DocumentPipeline(db=mock_client, chunk_table="my_chunks")
    assert pipeline.chunk_table == "my_chunks"


def test_pipeline_extraction_config_has_chunking(mock_client: AsyncMock) -> None:
    pipeline = DocumentPipeline(db=mock_client)
    assert pipeline._config is not None
    assert pipeline._config.chunking is not None


def test_pipeline_embed_true_has_embedding(mock_client: AsyncMock) -> None:
    pipeline = DocumentPipeline(db=mock_client, embed=True)
    assert pipeline._config is not None
    assert pipeline._config.chunking.embedding is not None


def test_pipeline_embed_false_no_embedding(mock_client: AsyncMock) -> None:
    pipeline = DocumentPipeline(db=mock_client, embed=False)
    assert pipeline._config is not None
    assert pipeline._config.chunking.embedding is None


def test_pipeline_user_extraction_config_gets_chunking(mock_client: AsyncMock) -> None:
    from xberg import ExtractionConfig

    user_config = ExtractionConfig()
    pipeline = DocumentPipeline(db=mock_client, config=user_config)

    assert pipeline._config is user_config
    assert pipeline._config.chunking is not None
    assert pipeline._config.chunking.embedding is not None


def test_pipeline_preserves_user_chunking_params(mock_client: AsyncMock) -> None:
    from xberg import ChunkingConfig, ExtractionConfig

    user_config = ExtractionConfig(
        chunking=ChunkingConfig(max_chars=512, max_overlap=100),
    )
    pipeline = DocumentPipeline(db=mock_client, config=user_config)

    assert pipeline._config is user_config
    assert pipeline._config.chunking.max_chars == 512
    assert pipeline._config.chunking.max_overlap == 100
    assert pipeline._config.chunking.embedding is not None


def test_pipeline_preserves_user_chunking_params_embed_false(mock_client: AsyncMock) -> None:
    from xberg import ChunkingConfig, ExtractionConfig

    user_config = ExtractionConfig(
        chunking=ChunkingConfig(max_chars=256),
    )
    pipeline = DocumentPipeline(db=mock_client, config=user_config, embed=False)

    assert pipeline._config is not None
    assert pipeline._config.chunking is not None
    assert pipeline._config.chunking.max_chars == 256
    assert pipeline._config.chunking.embedding is None


@patch("xberg_surrealdb.pipeline.build_pipeline_schema")
async def test_pipeline_setup_schema_forwards_params(
    mock_build: MagicMock,
    mock_client: AsyncMock,
) -> None:
    """setup_schema() passes constructor and method parameters to build_pipeline_schema."""
    mock_build.return_value = ["STMT1;", "STMT2;", "STMT3;"]
    pipeline = DocumentPipeline(db=mock_client, chunk_table="my_chunks", embedding_model="fast")

    await pipeline.setup_schema(
        analyzer_language="german",
        bm25_k1=1.5,
        bm25_b=0.8,
        distance_metric="EUCLIDEAN",
        hnsw_efc=200,
        hnsw_m=16,
    )

    mock_build.assert_called_once_with(
        table="documents",
        chunk_table="my_chunks",
        embed=True,
        embedding_dimension=384,
        analyzer_language="german",
        bm25_k1=1.5,
        bm25_b=0.8,
        distance_metric="EUCLIDEAN",
        hnsw_efc=200,
        hnsw_m=16,
    )
    assert mock_client.query.call_count == 3


@patch("xberg_surrealdb._base.extract_file")
async def test_pipeline_chunk_without_metadata(
    mock_extract: MagicMock,
    pipeline: DocumentPipeline,
    mock_client: AsyncMock,
    sample_extraction_result: MagicMock,
) -> None:
    """Chunks with empty metadata should set page_number/first_page/last_page to None."""
    chunk = MagicMock(spec=Chunk)
    chunk.content = "Chunk without metadata."
    chunk.embedding = [0.1] * 768
    chunk.metadata = {}

    sample_extraction_result.chunks = [chunk]
    mock_extract.return_value = sample_extraction_result

    await pipeline.ingest_file("/tmp/test.pdf")

    chunk_call = mock_client.query.call_args_list[1]
    chunk_records = chunk_call[0][1]["records"]
    rec = chunk_records[0]
    assert rec["page_number"] is None
    assert rec["char_start"] is None
    assert rec["char_end"] is None
    assert rec["first_page"] is None
    assert rec["last_page"] is None
    assert rec["content"] == "Chunk without metadata."
    assert rec["chunk_index"] == 0


@patch("xberg_surrealdb._base.extract_file")
async def test_pipeline_ingest_file_no_chunks_skips_chunk_insert(
    mock_extract: MagicMock,
    pipeline: DocumentPipeline,
    mock_client: AsyncMock,
    sample_extraction_result: MagicMock,
) -> None:
    sample_extraction_result.chunks = []
    mock_extract.return_value = sample_extraction_result

    await pipeline.ingest_file("/tmp/test.pdf")

    assert mock_client.query.call_count == 1
    assert "INSERT IGNORE INTO documents" in mock_client.query.call_args[0][0]


@patch("xberg_surrealdb._base.extract_file")
async def test_pipeline_chunk_batch_splitting(
    mock_extract: MagicMock,
    mock_client: AsyncMock,
    sample_extraction_result: MagicMock,
    sample_chunks: list[MagicMock],
) -> None:
    """With insert_batch_size=2 and 3 chunks, chunks should be split into 2 INSERT queries."""
    sample_extraction_result.chunks = sample_chunks
    mock_extract.return_value = sample_extraction_result

    pipeline = DocumentPipeline(db=mock_client, insert_batch_size=2)
    await pipeline.setup_schema()
    mock_client.query.reset_mock()

    await pipeline.ingest_file("/tmp/test.pdf")

    assert mock_client.query.call_count == 3
    chunk_call_1 = mock_client.query.call_args_list[1]
    chunk_call_2 = mock_client.query.call_args_list[2]
    assert len(chunk_call_1[0][1]["records"]) == 2
    assert len(chunk_call_2[0][1]["records"]) == 1


def test_check_insert_result_passes_on_normal_results() -> None:
    _check_insert_result([], context="test")
    _check_insert_result([{"id": "rec:1"}], context="test")
    _check_insert_result(None, context="test")


def test_check_insert_result_raises_on_dimension_error() -> None:
    result = ["Expected a vector of 768 dimensions, but got 384"]
    with pytest.raises(DimensionMismatchError, match="Vector dimension mismatch"):
        _check_insert_result(result, context="chunk insertion")


def test_check_insert_result_raises_on_generic_string_error() -> None:
    result = ["Some unexpected SurrealDB error"]
    with pytest.raises(IngestionError, match="INSERT IGNORE failed silently"):
        _check_insert_result(result, context="test")


@patch("xberg_surrealdb._base.extract_file")
async def test_pipeline_raises_on_chunk_dimension_mismatch(
    mock_extract: MagicMock,
    pipeline: DocumentPipeline,
    mock_client: AsyncMock,
    sample_extraction_result: MagicMock,
    sample_chunks: list[MagicMock],
) -> None:
    sample_extraction_result.chunks = sample_chunks
    mock_extract.return_value = sample_extraction_result
    mock_client.query = AsyncMock(
        side_effect=[
            [],
            ["Expected a vector of 768 dimensions, but got 384"],
        ]
    )

    with pytest.raises(DimensionMismatchError, match="Vector dimension mismatch during chunk insertion"):
        await pipeline.ingest_file("/tmp/test.pdf")


@patch("xberg_surrealdb.pipeline.extract_bytes")
async def test_embed_query_raises_on_empty_chunks(mock_extract: MagicMock, mock_client: AsyncMock) -> None:
    mock_result = MagicMock(spec=ExtractionResult)
    mock_result.chunks = []
    mock_extract.return_value = mock_result

    pipeline = DocumentPipeline(db=mock_client, embed=True)

    with pytest.raises(RuntimeError, match="Embedding generation failed"):
        await pipeline.embed_query("test query")


@patch("xberg_surrealdb.pipeline.extract_bytes")
async def test_embed_query_raises_on_none_embedding(mock_extract: MagicMock, mock_client: AsyncMock) -> None:
    mock_chunk = MagicMock(spec=Chunk)
    mock_chunk.embedding = None
    mock_result = MagicMock(spec=ExtractionResult)
    mock_result.chunks = [mock_chunk]
    mock_extract.return_value = mock_result

    pipeline = DocumentPipeline(db=mock_client, embed=True)

    with pytest.raises(RuntimeError, match="Embedding generation failed"):
        await pipeline.embed_query("test query")


@patch("xberg_surrealdb.pipeline.extract_bytes")
async def test_embed_query_returns_embedding(mock_extract: MagicMock, mock_client: AsyncMock) -> None:
    """embed_query() success path returns the embedding vector from the first chunk."""
    expected = [0.1, 0.2, 0.3]
    mock_chunk = MagicMock(spec=Chunk)
    mock_chunk.embedding = expected
    mock_result = MagicMock(spec=ExtractionResult)
    mock_result.chunks = [mock_chunk]
    mock_extract.return_value = mock_result

    pipeline = DocumentPipeline(db=mock_client, embed=True)
    result = await pipeline.embed_query("test query")

    assert result == expected
    mock_extract.assert_called_once_with(b"test query", "text/plain", config=pipeline._config)


@pytest.mark.parametrize(
    ("method", "args", "kwargs"),
    [
        ("ingest_file", ["/tmp/test.pdf"], {}),
        ("ingest_files", [["/tmp/a.pdf", "/tmp/b.pdf"]], {}),
        ("ingest_directory", ["/tmp"], {}),
        ("ingest_bytes", [], {"data": b"hello", "mime_type": "text/plain", "source": "test"}),
    ],
    ids=["ingest_file", "ingest_files", "ingest_directory", "ingest_bytes"],
)
async def test_pipeline_raises_without_schema(
    mock_client: AsyncMock,
    method: str,
    args: list[object],
    kwargs: dict[str, object],
) -> None:
    pipeline = DocumentPipeline(db=mock_client, embed=False)

    with pytest.raises(SchemaNotInitializedError, match="setup_schema"):
        await getattr(pipeline, method)(*args, **kwargs)

"""Integration tests requiring a live SurrealDB v3 server.

Run with: SURREALDB_URL=ws://localhost:8000 uv run pytest tests/test_integration.py -v -m integration

Requires:
- A SurrealDB v3 server (set SURREALDB_URL env var)
- ONNX Runtime installed on the system for embedding tests
"""

import os
import uuid
from collections.abc import AsyncGenerator
from pathlib import Path
from unittest.mock import AsyncMock

import pytest
from surrealdb import AsyncSurreal

from tests.conftest import FIXTURES_DIR
from xberg_surrealdb import AsyncSurrealQueryable, DocumentConnector, DocumentPipeline
from xberg_surrealdb.exceptions import SchemaNotInitializedError

pytestmark = pytest.mark.integration

_surrealdb_url = os.environ.get("SURREALDB_URL")


@pytest.fixture
async def server_db() -> AsyncGenerator[AsyncSurrealQueryable, None]:
    """A real SurrealDB connection with a unique database per test."""
    if not _surrealdb_url:
        pytest.skip("SURREALDB_URL not set")
    db = AsyncSurreal(_surrealdb_url)
    await db.connect()
    await db.signin({"username": "root", "password": "root"})
    db_name = f"test_{uuid.uuid4().hex[:8]}"
    await db.use("test", db_name)
    yield db
    try:
        await db.query(f"REMOVE DATABASE IF EXISTS {db_name}")
    finally:
        await db.close()


@pytest.fixture
def sample_text_file(tmp_path: Path) -> Path:
    f = tmp_path / "sample.txt"
    f.write_text(
        "Machine learning is a subset of artificial intelligence. "
        "It involves training algorithms on data to make predictions. "
        "Deep learning is a subset of machine learning that uses neural networks."
    )
    return f


@pytest.fixture
def second_text_file(tmp_path: Path) -> Path:
    f = tmp_path / "second.txt"
    f.write_text(
        "SurrealDB is a multi-model database that supports SQL-like queries. "
        "It provides graph, document, and relational features in a single platform."
    )
    return f


@pytest.fixture
def ml_corpus(tmp_path: Path) -> Path:
    """Directory with multiple text files for batch/search tests."""
    texts = {
        "neural.txt": (
            "Neural networks are computing systems inspired by biological neural networks. "
            "They learn to perform tasks by considering examples without task-specific programming."
        ),
        "transformers.txt": (
            "Transformer models use self-attention mechanisms to process sequential data. "
            "They are the foundation of modern large language models like GPT and BERT."
        ),
        "reinforcement.txt": (
            "Reinforcement learning trains agents to make decisions by rewarding desired behaviors. "
            "It has been used to master games like chess and Go."
        ),
    }
    for name, content in texts.items():
        (tmp_path / name).write_text(content)
    return tmp_path


async def test_connector_full_roundtrip(server_db: AsyncSurrealQueryable, sample_text_file: Path) -> None:
    connector = DocumentConnector(db=server_db)
    await connector.setup_schema()
    await connector.ingest_file(sample_text_file)

    t = connector.table
    results = await connector.client.query(
        f"SELECT *, search::score(1) AS score FROM {t} WHERE content @1@ $query ORDER BY score DESC LIMIT $limit",
        {"query": "machine learning", "limit": 5},
    )
    assert len(results) > 0


async def test_connector_ingest_multiple_files(
    server_db: AsyncSurrealQueryable,
    sample_text_file: Path,
    second_text_file: Path,
) -> None:
    connector = DocumentConnector(db=server_db)
    await connector.setup_schema()
    await connector.ingest_files([sample_text_file, second_text_file])

    t = connector.table
    results = await connector.client.query(
        f"SELECT *, search::score(1) AS score FROM {t} WHERE content @1@ $query ORDER BY score DESC LIMIT $limit",
        {"query": "database", "limit": 10},
    )
    assert len(results) > 0


async def test_connector_dedup_same_content(server_db: AsyncSurrealQueryable, sample_text_file: Path) -> None:
    connector = DocumentConnector(db=server_db)
    await connector.setup_schema()
    await connector.ingest_file(sample_text_file)
    await connector.ingest_file(sample_text_file)

    all_docs = await connector.client.query("SELECT * FROM documents")
    assert len(all_docs) == 1


async def test_connector_ingest_bytes(server_db: AsyncSurrealQueryable) -> None:
    connector = DocumentConnector(db=server_db)
    await connector.setup_schema()
    content = b"Python is a programming language used for web development and data science."
    await connector.ingest_bytes(data=content, mime_type="text/plain", source="test://bytes")

    t = connector.table
    results = await connector.client.query(
        f"SELECT *, search::score(1) AS score FROM {t} WHERE content @1@ $query ORDER BY score DESC LIMIT $limit",
        {"query": "python programming", "limit": 5},
    )
    assert len(results) > 0


async def test_connector_ingest_directory(server_db: AsyncSurrealQueryable, tmp_path: Path) -> None:
    for i in range(3):
        (tmp_path / f"doc_{i}.txt").write_text(f"Document number {i} about testing.")

    connector = DocumentConnector(db=server_db)
    await connector.setup_schema()
    await connector.ingest_directory(tmp_path, glob="*.txt")

    all_docs = await connector.client.query("SELECT * FROM documents")
    assert len(all_docs) == 3


async def test_connector_document_metadata_stored(server_db: AsyncSurrealQueryable, sample_text_file: Path) -> None:
    connector = DocumentConnector(db=server_db)
    await connector.setup_schema()
    await connector.ingest_file(sample_text_file)

    docs = await connector.client.query("SELECT * FROM documents")
    assert len(docs) == 1
    doc = docs[0]
    assert doc["source"] == str(sample_text_file)
    assert len(doc["content"]) > 0
    assert doc["mime_type"] == "text/plain"
    assert doc["content_hash"] is not None
    assert doc["ingested_at"] is not None


async def test_connector_search_limit_respected(server_db: AsyncSurrealQueryable, ml_corpus: Path) -> None:
    connector = DocumentConnector(db=server_db)
    await connector.setup_schema()
    await connector.ingest_directory(ml_corpus, glob="*.txt")

    t = connector.table
    results_1 = await connector.client.query(
        f"SELECT *, search::score(1) AS score FROM {t} WHERE content @1@ $query ORDER BY score DESC LIMIT $limit",
        {"query": "learning", "limit": 1},
    )
    results_all = await connector.client.query(
        f"SELECT *, search::score(1) AS score FROM {t} WHERE content @1@ $query ORDER BY score DESC LIMIT $limit",
        {"query": "learning", "limit": 10},
    )
    assert len(results_1) <= 1
    assert len(results_all) >= len(results_1)


async def test_connector_custom_table_name(server_db: AsyncSurrealQueryable, sample_text_file: Path) -> None:
    connector = DocumentConnector(db=server_db, table="my_docs")
    await connector.setup_schema()
    await connector.ingest_file(sample_text_file)

    docs = await connector.client.query("SELECT * FROM my_docs")
    assert len(docs) == 1

    t = connector.table
    results = await connector.client.query(
        f"SELECT *, search::score(1) AS score FROM {t} WHERE content @1@ $query ORDER BY score DESC LIMIT $limit",
        {"query": "machine learning", "limit": 10},
    )
    assert len(results) > 0


async def test_connector_ingest_without_schema_raises(server_db: AsyncSurrealQueryable, sample_text_file: Path) -> None:
    """Ingesting into a table without calling setup_schema first should fail."""
    connector = DocumentConnector(db=server_db)

    with pytest.raises(SchemaNotInitializedError):
        await connector.ingest_file(sample_text_file)


async def test_pipeline_chunks_linked_to_document(server_db: AsyncSurrealQueryable, sample_text_file: Path) -> None:
    pipeline = DocumentPipeline(db=server_db, embed=False)
    await pipeline.setup_schema()
    await pipeline.ingest_file(sample_text_file)

    docs = await pipeline.client.query("SELECT * FROM documents")
    assert len(docs) == 1
    doc_id = docs[0]["id"]

    chunks = await pipeline.client.query("SELECT * FROM chunks")
    assert len(chunks) > 0
    for chunk in chunks:
        assert chunk["document"] == doc_id


async def test_pipeline_dedup_skips_chunks(server_db: AsyncSurrealQueryable, sample_text_file: Path) -> None:
    pipeline = DocumentPipeline(db=server_db, embed=False)
    await pipeline.setup_schema()
    await pipeline.ingest_file(sample_text_file)
    first_chunk_count = len(await pipeline.client.query("SELECT * FROM chunks"))

    await pipeline.ingest_file(sample_text_file)
    second_chunk_count = len(await pipeline.client.query("SELECT * FROM chunks"))

    assert second_chunk_count == first_chunk_count


async def test_pipeline_chunk_metadata_stored(server_db: AsyncSurrealQueryable, sample_text_file: Path) -> None:
    pipeline = DocumentPipeline(db=server_db, embed=False)
    await pipeline.setup_schema()
    await pipeline.ingest_file(sample_text_file)

    chunks = await pipeline.client.query("SELECT * FROM chunks ORDER BY chunk_index ASC")
    assert len(chunks) > 0
    for i, chunk in enumerate(chunks):
        assert chunk["chunk_index"] == i
        assert "content" in chunk
        assert len(chunk["content"]) > 0
        assert chunk.get("word_count") is not None
        assert chunk["word_count"] > 0


async def test_pipeline_embed_false_no_embeddings(server_db: AsyncSurrealQueryable, sample_text_file: Path) -> None:
    pipeline = DocumentPipeline(db=server_db, embed=False)
    await pipeline.setup_schema()
    await pipeline.ingest_file(sample_text_file)

    chunks = await pipeline.client.query("SELECT * FROM chunks")
    for chunk in chunks:
        assert chunk.get("embedding") is None


async def test_pipeline_ingest_directory(server_db: AsyncSurrealQueryable, ml_corpus: Path) -> None:
    pipeline = DocumentPipeline(db=server_db, embed=False)
    await pipeline.setup_schema()
    await pipeline.ingest_directory(ml_corpus, glob="*.txt")

    docs = await pipeline.client.query("SELECT * FROM documents")
    assert len(docs) == 3

    chunks = await pipeline.client.query("SELECT * FROM chunks")
    assert len(chunks) >= 3


async def test_pipeline_ingest_files(
    server_db: AsyncSurrealQueryable,
    sample_text_file: Path,
    second_text_file: Path,
) -> None:
    pipeline = DocumentPipeline(db=server_db, embed=False)
    await pipeline.setup_schema()
    await pipeline.ingest_files([sample_text_file, second_text_file])

    docs = await pipeline.client.query("SELECT * FROM documents")
    assert len(docs) == 2


async def test_pipeline_ingest_bytes(server_db: AsyncSurrealQueryable) -> None:
    pipeline = DocumentPipeline(db=server_db, embed=False)
    await pipeline.setup_schema()
    content = b"Kubernetes orchestrates containerized applications across clusters of machines."
    await pipeline.ingest_bytes(data=content, mime_type="text/plain", source="test://k8s")

    docs = await pipeline.client.query("SELECT * FROM documents")
    assert len(docs) == 1
    assert docs[0]["source"] == "test://k8s"


async def test_pipeline_custom_table_names(server_db: AsyncSurrealQueryable, sample_text_file: Path) -> None:
    pipeline = DocumentPipeline(db=server_db, table="my_docs", chunk_table="my_chunks", embed=False)
    await pipeline.setup_schema()
    await pipeline.ingest_file(sample_text_file)

    docs = await pipeline.client.query("SELECT * FROM my_docs")
    assert len(docs) == 1

    chunks = await pipeline.client.query("SELECT * FROM my_chunks")
    assert len(chunks) > 0

    ct = pipeline.chunk_table
    results = await pipeline.client.query(
        f"SELECT *, search::score(1) AS score FROM {ct} WHERE content @1@ $query ORDER BY score DESC LIMIT $limit",
        {"query": "machine learning", "limit": 10},
    )
    assert len(results) > 0


async def test_pipeline_embed_true_ingest_and_chunks_have_embeddings(
    server_db: AsyncSurrealQueryable,
    sample_text_file: Path,
) -> None:
    pipeline = DocumentPipeline(db=server_db, embed=True)
    await pipeline.setup_schema()
    await pipeline.ingest_file(sample_text_file)

    chunks = await pipeline.client.query("SELECT * FROM chunks")
    assert len(chunks) > 0
    for chunk in chunks:
        assert chunk.get("embedding") is not None
        assert isinstance(chunk["embedding"], list)
        assert len(chunk["embedding"]) == 768


async def test_pipeline_embed_true_dedup(server_db: AsyncSurrealQueryable, sample_text_file: Path) -> None:
    pipeline = DocumentPipeline(db=server_db, embed=True)
    await pipeline.setup_schema()
    await pipeline.ingest_file(sample_text_file)
    first_count = len(await pipeline.client.query("SELECT * FROM chunks"))

    await pipeline.ingest_file(sample_text_file)
    second_count = len(await pipeline.client.query("SELECT * FROM chunks"))

    assert second_count == first_count


async def test_pipeline_embed_true_multiple_docs_ingested(server_db: AsyncSurrealQueryable, ml_corpus: Path) -> None:
    pipeline = DocumentPipeline(db=server_db, embed=True)
    await pipeline.setup_schema()
    await pipeline.ingest_directory(ml_corpus, glob="*.txt")

    docs = await pipeline.client.query("SELECT * FROM documents")
    assert len(docs) == 3

    chunks = await pipeline.client.query("SELECT * FROM chunks")
    assert len(chunks) >= 3
    for chunk in chunks:
        assert chunk.get("embedding") is not None
        assert len(chunk["embedding"]) == 768


async def test_pipeline_bm25_via_client(server_db: AsyncSurrealQueryable, sample_text_file: Path) -> None:
    """BM25 fulltext search on chunks via pipeline.client."""
    pipeline = DocumentPipeline(db=server_db, embed=False)
    await pipeline.setup_schema()
    await pipeline.ingest_file(sample_text_file)

    ct = pipeline.chunk_table
    results = await pipeline.client.query(
        f"SELECT *, search::score(1) AS score FROM {ct} WHERE content @1@ $query ORDER BY score DESC LIMIT $limit",
        {"query": "machine learning", "limit": 5},
    )
    assert len(results) > 0


async def test_pipeline_vector_search_via_client(server_db: AsyncSurrealQueryable, sample_text_file: Path) -> None:
    """Vector KNN search on chunks via pipeline.client + pipeline.embed_query()."""
    pipeline = DocumentPipeline(db=server_db, embed=True)
    await pipeline.setup_schema()
    await pipeline.ingest_file(sample_text_file)

    embedding = await pipeline.embed_query("artificial intelligence")
    ct = pipeline.chunk_table
    results = await pipeline.client.query(
        f"SELECT *, vector::distance::knn() AS distance FROM {ct} "
        f"WHERE embedding <|5,COSINE|> $embedding ORDER BY distance",
        {"embedding": embedding},
    )
    assert len(results) > 0


async def test_pipeline_hybrid_rrf_via_client(server_db: AsyncSurrealQueryable, sample_text_file: Path) -> None:
    """Hybrid RRF search on chunks via pipeline.client + pipeline.embed_query()."""
    pipeline = DocumentPipeline(db=server_db, embed=True)
    await pipeline.setup_schema()
    await pipeline.ingest_file(sample_text_file)

    embedding = await pipeline.embed_query("machine learning algorithms")
    ct = pipeline.chunk_table
    results = await pipeline.client.query(
        f"SELECT * FROM search::rrf(["
        f"(SELECT id, content FROM {ct} WHERE embedding <|5,COSINE|> $embedding),"
        f"(SELECT id, content, search::score(1) AS score FROM {ct} "
        f"WHERE content @1@ $query ORDER BY score DESC LIMIT 5)"
        f"], 5, 60);",
        {"embedding": embedding, "query": "machine learning algorithms"},
    )
    assert len(results) > 0


async def test_fast_preset_produces_384_dim_embeddings(tmp_path: Path) -> None:
    """Verify the 'fast' preset produces 384-dim embeddings via real xberg extraction."""
    from xberg import extract_file

    mock_client = AsyncMock(spec=AsyncSurrealQueryable)
    sample = tmp_path / "sample.txt"
    sample.write_text("Machine learning is a subset of artificial intelligence.")

    pipeline = DocumentPipeline(db=mock_client, embed=True, embedding_model="fast")
    result = await extract_file(str(sample), config=pipeline._config)

    assert len(result.chunks) > 0
    for chunk in result.chunks:
        assert chunk.embedding is not None
        assert len(chunk.embedding) == 384


async def test_fast_preset_embed_query_produces_384_dim() -> None:
    """Verify the 'fast' preset can embed a query string with correct dimensions."""
    from xberg import extract_bytes

    mock_client = AsyncMock(spec=AsyncSurrealQueryable)
    pipeline = DocumentPipeline(db=mock_client, embed=True, embedding_model="fast")
    result = await extract_bytes(b"machine learning", "text/plain", config=pipeline._config)

    assert len(result.chunks) > 0
    assert result.chunks[0].embedding is not None
    assert len(result.chunks[0].embedding) == 384


FIXTURE_FILES = {
    "txt": FIXTURES_DIR / "sample.txt",
    "html": FIXTURES_DIR / "sample.html",
    "pdf": FIXTURES_DIR / "sample.pdf",
    "docx": FIXTURES_DIR / "sample.docx",
}

_MIME_FALLBACK = {
    ".docx": "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
}


def _mime_for(path: Path) -> str:
    """Derive MIME type from file extension."""
    import mimetypes

    mime, _ = mimetypes.guess_type(path.name)
    if mime is None:
        mime = _MIME_FALLBACK.get(path.suffix, "application/octet-stream")
    return mime


@pytest.mark.parametrize("fmt", ["txt", "html", "pdf", "docx"])
async def test_connector_ingest_file_fixture(server_db: AsyncSurrealQueryable, fmt: str) -> None:
    path = FIXTURE_FILES[fmt]
    connector = DocumentConnector(db=server_db)
    await connector.setup_schema()
    await connector.ingest_file(path)

    docs = await connector.client.query("SELECT * FROM documents")
    assert len(docs) == 1
    assert docs[0]["source"] == str(path)
    assert len(docs[0]["content"]) > 0
    assert docs[0]["content_hash"] is not None


@pytest.mark.parametrize("fmt", ["txt", "html", "pdf", "docx"])
async def test_connector_ingest_bytes_fixture(server_db: AsyncSurrealQueryable, fmt: str) -> None:
    path = FIXTURE_FILES[fmt]
    data = path.read_bytes()

    connector = DocumentConnector(db=server_db)
    await connector.setup_schema()
    await connector.ingest_bytes(data=data, mime_type=_mime_for(path), source=f"fixture://{fmt}")

    docs = await connector.client.query("SELECT * FROM documents")
    assert len(docs) == 1
    assert docs[0]["source"] == f"fixture://{fmt}"
    assert len(docs[0]["content"]) > 0


async def test_connector_ingest_files_all_fixtures(server_db: AsyncSurrealQueryable) -> None:
    paths = list(FIXTURE_FILES.values())
    connector = DocumentConnector(db=server_db)
    await connector.setup_schema()
    await connector.ingest_files(paths)

    docs = await connector.client.query("SELECT * FROM documents")
    assert len(docs) == len(paths)


async def test_connector_ingest_directory_fixtures(server_db: AsyncSurrealQueryable) -> None:
    connector = DocumentConnector(db=server_db)
    await connector.setup_schema()
    await connector.ingest_directory(FIXTURES_DIR, glob="*.*")

    docs = await connector.client.query("SELECT * FROM documents")
    assert len(docs) == 4


async def test_connector_search_fixture_content(server_db: AsyncSurrealQueryable) -> None:
    connector = DocumentConnector(db=server_db)
    await connector.setup_schema()
    await connector.ingest_directory(FIXTURES_DIR, glob="*.*")

    t = connector.table
    results = await connector.client.query(
        f"SELECT *, search::score(1) AS score FROM {t} WHERE content @1@ $query ORDER BY score DESC LIMIT $limit",
        {"query": "sample document testing", "limit": 10},
    )
    assert len(results) > 0


@pytest.mark.parametrize("fmt", ["txt", "html", "pdf", "docx"])
async def test_pipeline_ingest_file_fixture_embed_false(server_db: AsyncSurrealQueryable, fmt: str) -> None:
    path = FIXTURE_FILES[fmt]
    pipeline = DocumentPipeline(db=server_db, embed=False)
    await pipeline.setup_schema()
    await pipeline.ingest_file(path)

    docs = await pipeline.client.query("SELECT * FROM documents")
    assert len(docs) == 1
    assert docs[0]["source"] == str(path)

    chunks = await pipeline.client.query("SELECT * FROM chunks ORDER BY chunk_index ASC")
    assert len(chunks) > 0
    for i, chunk in enumerate(chunks):
        assert chunk["chunk_index"] == i
        assert len(chunk["content"]) > 0
        assert chunk.get("embedding") is None


@pytest.mark.parametrize("fmt", ["txt", "html", "pdf", "docx"])
async def test_pipeline_ingest_bytes_fixture_embed_false(server_db: AsyncSurrealQueryable, fmt: str) -> None:
    path = FIXTURE_FILES[fmt]
    data = path.read_bytes()

    pipeline = DocumentPipeline(db=server_db, embed=False)
    await pipeline.setup_schema()
    await pipeline.ingest_bytes(data=data, mime_type=_mime_for(path), source=f"fixture://{fmt}")

    docs = await pipeline.client.query("SELECT * FROM documents")
    assert len(docs) == 1
    assert docs[0]["source"] == f"fixture://{fmt}"

    chunks = await pipeline.client.query("SELECT * FROM chunks")
    assert len(chunks) > 0


async def test_pipeline_ingest_files_all_fixtures_embed_false(server_db: AsyncSurrealQueryable) -> None:
    paths = list(FIXTURE_FILES.values())
    pipeline = DocumentPipeline(db=server_db, embed=False)
    await pipeline.setup_schema()
    await pipeline.ingest_files(paths)

    docs = await pipeline.client.query("SELECT * FROM documents")
    assert len(docs) == len(paths)

    chunks = await pipeline.client.query("SELECT * FROM chunks")
    assert len(chunks) >= len(paths)


async def test_pipeline_ingest_directory_fixtures_embed_false(server_db: AsyncSurrealQueryable) -> None:
    pipeline = DocumentPipeline(db=server_db, embed=False)
    await pipeline.setup_schema()
    await pipeline.ingest_directory(FIXTURES_DIR, glob="*.*")

    docs = await pipeline.client.query("SELECT * FROM documents")
    assert len(docs) == 4

    chunks = await pipeline.client.query("SELECT * FROM chunks")
    assert len(chunks) >= 4


@pytest.mark.parametrize("fmt", ["txt", "html", "pdf", "docx"])
async def test_pipeline_ingest_file_fixture_embed_true(server_db: AsyncSurrealQueryable, fmt: str) -> None:
    path = FIXTURE_FILES[fmt]
    pipeline = DocumentPipeline(db=server_db, embed=True)
    await pipeline.setup_schema()
    await pipeline.ingest_file(path)

    docs = await pipeline.client.query("SELECT * FROM documents")
    assert len(docs) == 1

    chunks = await pipeline.client.query("SELECT * FROM chunks")
    assert len(chunks) > 0
    for chunk in chunks:
        assert chunk.get("embedding") is not None
        assert len(chunk["embedding"]) == 768


@pytest.mark.parametrize("fmt", ["txt", "html", "pdf", "docx"])
async def test_pipeline_ingest_bytes_fixture_embed_true(server_db: AsyncSurrealQueryable, fmt: str) -> None:
    path = FIXTURE_FILES[fmt]
    data = path.read_bytes()

    pipeline = DocumentPipeline(db=server_db, embed=True)
    await pipeline.setup_schema()
    await pipeline.ingest_bytes(data=data, mime_type=_mime_for(path), source=f"fixture://{fmt}")

    docs = await pipeline.client.query("SELECT * FROM documents")
    assert len(docs) == 1

    chunks = await pipeline.client.query("SELECT * FROM chunks")
    assert len(chunks) > 0
    for chunk in chunks:
        assert chunk.get("embedding") is not None
        assert len(chunk["embedding"]) == 768


async def test_pipeline_ingest_all_fixtures_embed_true(server_db: AsyncSurrealQueryable) -> None:
    pipeline = DocumentPipeline(db=server_db, embed=True)
    await pipeline.setup_schema()
    await pipeline.ingest_directory(FIXTURES_DIR, glob="*.*")

    docs = await pipeline.client.query("SELECT * FROM documents")
    assert len(docs) == 4

    chunks = await pipeline.client.query("SELECT * FROM chunks")
    assert len(chunks) >= 4
    for chunk in chunks:
        assert chunk.get("embedding") is not None
        assert len(chunk["embedding"]) == 768

"""Shared test fixtures."""

from pathlib import Path
from unittest.mock import AsyncMock, MagicMock

import pytest
from xberg import Chunk, ExtractionResult

from xberg_surrealdb import AsyncSurrealQueryable
from xberg_surrealdb.connector import DocumentConnector
from xberg_surrealdb.pipeline import DocumentPipeline

FIXTURES_DIR = Path(__file__).parent / "fixtures"


@pytest.fixture
def mock_client() -> AsyncMock:
    """Mock AsyncSurreal connection."""
    client = AsyncMock(spec=AsyncSurrealQueryable)
    client.query = AsyncMock(return_value=[])
    return client


@pytest.fixture
def sample_extraction_result() -> MagicMock:
    """A mock ExtractionResult with typical fields populated."""
    result = MagicMock(spec=ExtractionResult)
    result.content = "This is the extracted document content."
    result.mime_type = "text/plain"
    result.metadata = {"title": "Test Document", "authors": ["Alice", "Bob"]}
    result.quality_score = 0.95
    result.detected_languages = [{"language": "en", "confidence": 0.99}]
    kw = MagicMock()
    kw.text = "test"
    result.extracted_keywords = [kw]
    result.chunks = []
    return result


@pytest.fixture
def sample_chunks() -> list[MagicMock]:
    """Mock chunks with embeddings and metadata."""
    chunks = []
    for i in range(3):
        chunk = MagicMock(spec=Chunk)
        chunk.content = f"Chunk {i} content about testing."
        chunk.embedding = [0.1 * i] * 768
        chunk.metadata = {
            "page_number": i + 1,
            "char_start": i * 100,
            "char_end": (i + 1) * 100,
            "first_page": i + 1,
            "last_page": i + 1,
        }
        chunks.append(chunk)
    return chunks


@pytest.fixture
async def connector(mock_client: AsyncMock) -> DocumentConnector:
    """DocumentConnector with schema initialized and mock query reset."""
    conn = DocumentConnector(db=mock_client)
    await conn.setup_schema()
    mock_client.query.reset_mock()
    return conn


@pytest.fixture
async def pipeline(mock_client: AsyncMock) -> DocumentPipeline:
    """DocumentPipeline with schema initialized and mock query reset."""
    pipe = DocumentPipeline(db=mock_client)
    await pipe.setup_schema()
    mock_client.query.reset_mock()
    return pipe

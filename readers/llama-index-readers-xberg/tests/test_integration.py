"""Integration tests for XbergReader — real extraction, no mocks."""

from pathlib import Path

import pytest
from xberg import ExtractionConfig, PageConfig
from llama_index.readers.xberg import XbergReader

FIXTURES = Path(__file__).parent / "fixtures"


@pytest.fixture
def reader() -> XbergReader:
    return XbergReader()


@pytest.fixture
def fixtures_dir() -> Path:
    return FIXTURES


# --- Scenario 1: User extracts a PDF standalone ---


def test_standalone_pdf_extraction(reader: XbergReader) -> None:
    docs = reader.load_data(FIXTURES / "sample.pdf")

    assert len(docs) >= 1
    for doc in docs:
        assert doc.text, "Document text should not be empty"
        assert doc.id_, "Document should have a non-empty ID"

    meta = docs[0].metadata
    assert meta["file_name"] == "sample.pdf"
    assert "file_type" in meta
    assert "total_pages" in meta


# --- Scenario 2: User extracts from raw bytes ---


def test_bytes_extraction_matches_file(reader: XbergReader) -> None:
    file_docs = reader.load_data(FIXTURES / "sample.pdf")
    pdf_bytes = (FIXTURES / "sample.pdf").read_bytes()
    bytes_docs = reader.load_data(data=pdf_bytes, mime_type="application/pdf")

    assert len(bytes_docs) >= 1
    assert bytes_docs[0].text, "Bytes extraction should produce non-empty text"
    # Same source, same content
    assert bytes_docs[0].text == file_docs[0].text


# --- Scenario 3: User extracts multi-page PDF with per-page splitting ---


def test_per_page_splitting(fixtures_dir: Path) -> None:
    reader = XbergReader(extraction_config=ExtractionConfig(pages=PageConfig(extract_pages=True)))
    docs = reader.load_data(fixtures_dir / "sample.pdf")

    assert len(docs) == 3, f"sample.pdf has 3 pages, got {len(docs)} docs"

    page_numbers = [doc.metadata["page_number"] for doc in docs]
    assert page_numbers == [1, 2, 3]

    ids = [doc.id_ for doc in docs]
    assert len(set(ids)) == 3, "Each page should have a unique ID"

    for doc in docs:
        assert doc.text, f"Page {doc.metadata['page_number']} should have non-empty text"


# --- Scenario 4: User points SimpleDirectoryReader at mixed files ---


def test_sdr_mixed_directory(reader: XbergReader, fixtures_dir: Path) -> None:
    from llama_index.core import SimpleDirectoryReader

    sdr = SimpleDirectoryReader(
        input_dir=str(fixtures_dir),
        file_extractor={
            ".pdf": reader,
            ".txt": reader,
            ".html": reader,
            ".docx": reader,
        },
    )
    docs = sdr.load_data()

    file_names = {doc.metadata["file_name"] for doc in docs}
    expected = {"sample.pdf", "sample.txt", "sample.html", "sample.docx"}
    assert expected.issubset(file_names), f"Missing files: {expected - file_names}"

    for doc in docs:
        assert "file_name" in doc.metadata
        assert "file_path" in doc.metadata
        assert doc.text, f"{doc.metadata['file_name']} should have non-empty text"


# --- Scenario 5: User uses SDR with filename_as_id ---


def test_sdr_filename_as_id(reader: XbergReader, fixtures_dir: Path) -> None:
    from llama_index.core import SimpleDirectoryReader

    sdr = SimpleDirectoryReader(
        input_dir=str(fixtures_dir),
        file_extractor={
            ".pdf": reader,
            ".txt": reader,
            ".html": reader,
            ".docx": reader,
        },
        filename_as_id=True,
    )
    docs = sdr.load_data()

    for doc in docs:
        assert "_part_" in doc.id_, f"SDR should set ID to {{filepath}}_part_{{i}}, got {doc.id_}"
        assert doc.metadata["file_name"] in doc.id_


# --- Scenario 6: User uses async SDR extraction ---


@pytest.mark.asyncio
async def test_sdr_async_extraction(reader: XbergReader, fixtures_dir: Path) -> None:
    from llama_index.core import SimpleDirectoryReader

    sdr = SimpleDirectoryReader(
        input_dir=str(fixtures_dir),
        file_extractor={
            ".pdf": reader,
            ".txt": reader,
            ".html": reader,
            ".docx": reader,
        },
    )

    async_docs = await sdr.aload_data()

    assert len(async_docs) >= 4, "Should have at least one doc per fixture file"

    for doc in async_docs:
        assert doc.text, f"{doc.metadata.get('file_name', '?')} should have non-empty text"
        assert "file_name" in doc.metadata


# --- Scenario 7: User feeds reader output into IngestionPipeline, re-runs for dedup ---


def test_pipeline_deduplication(reader: XbergReader, fixtures_dir: Path) -> None:
    from llama_index.core.ingestion import DocstoreStrategy, IngestionPipeline
    from llama_index.core.node_parser import SentenceSplitter
    from llama_index.core.storage.docstore import SimpleDocumentStore

    docs = reader.load_data(fixtures_dir / "sample.txt")
    assert len(docs) >= 1

    docstore = SimpleDocumentStore()
    pipeline = IngestionPipeline(
        transformations=[SentenceSplitter()],
        docstore=docstore,
        docstore_strategy=DocstoreStrategy.UPSERTS,
    )

    # First run — should produce nodes
    nodes_first = pipeline.run(documents=docs)
    assert len(nodes_first) > 0, "First pipeline run should produce nodes"

    # Second run — identical docs, should deduplicate
    nodes_second = pipeline.run(documents=docs)
    assert len(nodes_second) == 0, "Second run with identical docs should produce no new nodes (dedup by hash)"


# --- Scenario 8: User re-extracts with changed metadata, pipeline re-processes ---


def test_pipeline_reprocesses_on_metadata_change(reader: XbergReader, fixtures_dir: Path) -> None:
    from llama_index.core.ingestion import DocstoreStrategy, IngestionPipeline
    from llama_index.core.node_parser import SentenceSplitter
    from llama_index.core.storage.docstore import SimpleDocumentStore

    docstore = SimpleDocumentStore()
    pipeline = IngestionPipeline(
        transformations=[SentenceSplitter()],
        docstore=docstore,
        docstore_strategy=DocstoreStrategy.UPSERTS,
    )

    # First run — baseline
    docs_v1 = reader.load_data(fixtures_dir / "sample.txt")
    nodes_v1 = pipeline.run(documents=docs_v1)
    assert len(nodes_v1) > 0

    # Second run — same file but different extra_info changes the hash
    docs_v2 = reader.load_data(fixtures_dir / "sample.txt", extra_info={"version": "2"})

    # Same file path → same doc ID, but different metadata → different hash
    assert docs_v1[0].id_ == docs_v2[0].id_, "Same file should produce same doc ID"

    nodes_v2 = pipeline.run(documents=docs_v2)
    assert len(nodes_v2) > 0, "Changed metadata should change doc hash, causing pipeline to re-process"

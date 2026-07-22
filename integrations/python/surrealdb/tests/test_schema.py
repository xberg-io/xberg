"""Tests for schema DDL generation."""

from xberg_surrealdb.schema import (
    build_connector_schema,
    build_document_schema,
    build_pipeline_schema,
)


def test_document_schema_generates_analyzer() -> None:
    stmts = build_document_schema(table="documents")
    assert any("DEFINE ANALYZER" in s and "snowball(english)" in s for s in stmts)


def test_document_schema_generates_schemafull_table() -> None:
    stmts = build_document_schema(table="documents")
    assert any("DEFINE TABLE IF NOT EXISTS documents SCHEMAFULL" in s for s in stmts)


def test_document_schema_generates_all_fields() -> None:
    stmts = build_document_schema(table="documents")
    joined = " ".join(stmts)
    for field in [
        "source",
        "content",
        "mime_type",
        "title",
        "authors",
        "created_at",
        "ingested_at",
        "metadata",
        "quality_score",
        "content_hash",
        "detected_languages",
        "keywords",
    ]:
        assert f"FIELD IF NOT EXISTS {field}" in joined


def test_document_schema_generates_unique_indexes() -> None:
    stmts = build_document_schema(table="documents")
    joined = " ".join(stmts)
    assert "idx_doc_source" in joined
    assert "idx_doc_hash" in joined
    assert "UNIQUE" in joined


def test_document_schema_custom_table_name() -> None:
    stmts = build_document_schema(table="my_docs")
    assert any("DEFINE TABLE IF NOT EXISTS my_docs" in s for s in stmts)
    assert any("ON TABLE my_docs" in s for s in stmts)


def test_document_schema_custom_analyzer_language() -> None:
    stmts = build_document_schema(table="documents", analyzer_language="german")
    assert any("snowball(german)" in s for s in stmts)


def test_connector_schema_includes_document_schema() -> None:
    stmts = build_connector_schema(table="documents")
    joined = " ".join(stmts)
    assert "DEFINE TABLE IF NOT EXISTS documents SCHEMAFULL" in joined
    assert "idx_doc_source" in joined
    assert "idx_doc_hash" in joined


def test_connector_schema_adds_bm25_index_on_content() -> None:
    stmts = build_connector_schema(table="documents")
    bm25_stmts = [s for s in stmts if "idx_doc_content" in s]
    assert len(bm25_stmts) == 1
    assert "BM25(1.2,0.75)" in bm25_stmts[0]
    assert "HIGHLIGHTS" in bm25_stmts[0]


def test_connector_schema_custom_bm25_params() -> None:
    stmts = build_connector_schema(table="documents", bm25_k1=1.5, bm25_b=0.8)
    bm25_stmts = [s for s in stmts if "idx_doc_content" in s]
    assert "BM25(1.5,0.8)" in bm25_stmts[0]


def test_pipeline_schema_includes_document_and_chunk_tables() -> None:
    stmts = build_pipeline_schema(
        table="documents",
        chunk_table="chunks",
        embed=True,
        embedding_dimension=768,
    )
    joined = " ".join(stmts)
    assert "DEFINE TABLE IF NOT EXISTS documents SCHEMAFULL" in joined
    assert "DEFINE TABLE IF NOT EXISTS chunks SCHEMAFULL" in joined


def test_pipeline_schema_chunk_fields() -> None:
    stmts = build_pipeline_schema(
        table="documents",
        chunk_table="chunks",
        embed=True,
        embedding_dimension=768,
    )
    joined = " ".join(stmts)
    for field in [
        "document",
        "content",
        "chunk_index",
        "embedding",
        "page_number",
        "char_start",
        "char_end",
        "word_count",
        "first_page",
        "last_page",
    ]:
        assert f"FIELD IF NOT EXISTS {field} ON TABLE chunks" in joined


def test_pipeline_schema_chunk_document_record_link() -> None:
    stmts = build_pipeline_schema(
        table="documents",
        chunk_table="chunks",
        embed=True,
        embedding_dimension=768,
    )
    assert any("TYPE record<documents>" in s for s in stmts)


def test_pipeline_schema_bm25_on_chunks() -> None:
    stmts = build_pipeline_schema(
        table="documents",
        chunk_table="chunks",
        embed=True,
        embedding_dimension=768,
    )
    chunk_bm25 = [s for s in stmts if "idx_chunk_content" in s]
    assert len(chunk_bm25) == 1
    assert "BM25(1.2,0.75)" in chunk_bm25[0]


def test_pipeline_schema_no_bm25_on_documents() -> None:
    stmts = build_pipeline_schema(
        table="documents",
        chunk_table="chunks",
        embed=True,
        embedding_dimension=768,
    )
    assert not any("idx_doc_content" in s for s in stmts)


def test_pipeline_schema_hnsw_index_when_embed_true() -> None:
    stmts = build_pipeline_schema(
        table="documents",
        chunk_table="chunks",
        embed=True,
        embedding_dimension=768,
    )
    hnsw = [s for s in stmts if "idx_chunk_embedding" in s]
    assert len(hnsw) == 1
    assert "HNSW DIMENSION 768" in hnsw[0]
    assert "DIST COSINE" in hnsw[0]
    assert "EFC 150" in hnsw[0]
    assert "M 12" in hnsw[0]


def test_pipeline_schema_no_hnsw_index_when_embed_false() -> None:
    stmts = build_pipeline_schema(
        table="documents",
        chunk_table="chunks",
        embed=False,
        embedding_dimension=768,
    )
    assert not any("idx_chunk_embedding" in s for s in stmts)
    assert any("idx_chunk_content" in s for s in stmts)


def test_pipeline_schema_custom_chunk_table_name() -> None:
    stmts = build_pipeline_schema(
        table="documents",
        chunk_table="my_chunks",
        embed=True,
        embedding_dimension=384,
    )
    assert any("DEFINE TABLE IF NOT EXISTS my_chunks" in s for s in stmts)
    assert any("HNSW DIMENSION 384" in s for s in stmts)


def test_pipeline_schema_custom_hnsw_params() -> None:
    stmts = build_pipeline_schema(
        table="documents",
        chunk_table="chunks",
        embed=True,
        embedding_dimension=768,
        distance_metric="EUCLIDEAN",
        hnsw_efc=200,
        hnsw_m=16,
    )
    hnsw = [s for s in stmts if "idx_chunk_embedding" in s]
    assert "DIST EUCLIDEAN" in hnsw[0]
    assert "EFC 200" in hnsw[0]
    assert "M 16" in hnsw[0]

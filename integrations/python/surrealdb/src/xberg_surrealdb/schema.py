"""SurrealDB schema definitions and DDL generation."""


def _build_analyzer(*, analyzer_language: str = "english") -> list[str]:
    """Generate the shared BM25 analyzer definition.

    Args:
        analyzer_language: Snowball stemmer language for the BM25 analyzer.

    Returns:
        A single-element list with the DEFINE ANALYZER DDL statement.

    """
    return [
        f"DEFINE ANALYZER IF NOT EXISTS doc_analyzer TOKENIZERS class FILTERS snowball({analyzer_language});",
    ]


def build_document_schema(*, table: str = "documents", analyzer_language: str = "english") -> list[str]:
    """Generate DDL for the documents table (used by both classes).

    Args:
        table: Name of the documents table.
        analyzer_language: Snowball stemmer language for the BM25 analyzer.

    Returns:
        Ordered list of SurrealQL DDL statements for the table, fields, and indexes.

    """
    stmts = _build_analyzer(analyzer_language=analyzer_language)
    stmts.extend(
        [
            f"DEFINE TABLE IF NOT EXISTS {table} SCHEMAFULL;",
            f"DEFINE FIELD IF NOT EXISTS source ON TABLE {table} TYPE string;",
            f"DEFINE FIELD IF NOT EXISTS content ON TABLE {table} TYPE string;",
            f"DEFINE FIELD IF NOT EXISTS mime_type ON TABLE {table} TYPE string;",
            f"DEFINE FIELD IF NOT EXISTS title ON TABLE {table} TYPE option<string>;",
            f"DEFINE FIELD IF NOT EXISTS authors ON TABLE {table} TYPE option<string>;",
            f"DEFINE FIELD IF NOT EXISTS created_at ON TABLE {table} TYPE option<datetime>;",
            f"DEFINE FIELD IF NOT EXISTS ingested_at ON TABLE {table} TYPE datetime DEFAULT time::now();",
            f"DEFINE FIELD IF NOT EXISTS metadata ON TABLE {table} TYPE object FLEXIBLE;",
            f"DEFINE FIELD IF NOT EXISTS quality_score ON TABLE {table} TYPE option<float>;",
            f"DEFINE FIELD IF NOT EXISTS content_hash ON TABLE {table} TYPE string;",
            f"DEFINE FIELD IF NOT EXISTS detected_languages ON TABLE {table} TYPE option<array<object>>;",
            f"DEFINE FIELD IF NOT EXISTS keywords ON TABLE {table} TYPE option<array<string>>;",
            f"DEFINE INDEX IF NOT EXISTS idx_doc_source ON TABLE {table} FIELDS source UNIQUE;",
            f"DEFINE INDEX IF NOT EXISTS idx_doc_hash ON TABLE {table} FIELDS content_hash UNIQUE;",
        ]
    )
    return stmts


def build_connector_schema(
    *,
    table: str = "documents",
    analyzer_language: str = "english",
    bm25_k1: float = 1.2,
    bm25_b: float = 0.75,
) -> list[str]:
    """Generate DDL for DocumentConnector: documents table + BM25 on documents.content.

    Args:
        table: Name of the documents table.
        analyzer_language: Snowball stemmer language for the BM25 analyzer.
        bm25_k1: BM25 term-frequency saturation parameter.
        bm25_b: BM25 document-length normalization parameter.

    Returns:
        Ordered list of SurrealQL DDL statements including the fulltext index.

    """
    stmts = build_document_schema(table=table, analyzer_language=analyzer_language)
    stmts.append(
        f"DEFINE INDEX IF NOT EXISTS idx_doc_content ON TABLE {table} "
        f"FIELDS content FULLTEXT ANALYZER doc_analyzer BM25({bm25_k1},{bm25_b}) HIGHLIGHTS;",
    )
    return stmts


def _build_chunk_schema(chunk_table: str, table: str) -> list[str]:
    """Generate DDL for the chunks table.

    Args:
        chunk_table: Name of the chunks table.
        table: Name of the parent documents table (for the record link type).

    Returns:
        Ordered list of SurrealQL DDL statements for the chunks table and fields.

    """
    return [
        f"DEFINE TABLE IF NOT EXISTS {chunk_table} SCHEMAFULL;",
        f"DEFINE FIELD IF NOT EXISTS document ON TABLE {chunk_table} TYPE record<{table}>;",
        f"DEFINE FIELD IF NOT EXISTS content ON TABLE {chunk_table} TYPE string;",
        f"DEFINE FIELD IF NOT EXISTS chunk_index ON TABLE {chunk_table} TYPE int;",
        f"DEFINE FIELD IF NOT EXISTS embedding ON TABLE {chunk_table} TYPE option<array<float>>;",
        f"DEFINE FIELD IF NOT EXISTS page_number ON TABLE {chunk_table} TYPE option<int>;",
        f"DEFINE FIELD IF NOT EXISTS char_start ON TABLE {chunk_table} TYPE option<int>;",
        f"DEFINE FIELD IF NOT EXISTS char_end ON TABLE {chunk_table} TYPE option<int>;",
        f"DEFINE FIELD IF NOT EXISTS word_count ON TABLE {chunk_table} TYPE option<int>;",
        f"DEFINE FIELD IF NOT EXISTS first_page ON TABLE {chunk_table} TYPE option<int>;",
        f"DEFINE FIELD IF NOT EXISTS last_page ON TABLE {chunk_table} TYPE option<int>;",
    ]


def build_pipeline_schema(
    *,
    table: str = "documents",
    chunk_table: str = "chunks",
    embed: bool,
    embedding_dimension: int,
    analyzer_language: str = "english",
    bm25_k1: float = 1.2,
    bm25_b: float = 0.75,
    distance_metric: str = "COSINE",
    hnsw_efc: int = 150,
    hnsw_m: int = 12,
) -> list[str]:
    """Generate DDL for DocumentPipeline: documents + chunks tables, conditional HNSW.

    Args:
        table: Name of the documents table.
        chunk_table: Name of the chunks table.
        embed: Whether to include an HNSW vector index on the chunks table.
        embedding_dimension: Vector dimension for the HNSW index.
        analyzer_language: Snowball stemmer language for the BM25 analyzer.
        bm25_k1: BM25 term-frequency saturation parameter.
        bm25_b: BM25 document-length normalization parameter.
        distance_metric: HNSW distance function (e.g. ``"COSINE"``, ``"EUCLIDEAN"``).
        hnsw_efc: HNSW construction-time search width (higher = slower build, better recall).
        hnsw_m: HNSW max edges per node (higher = more memory, better recall).

    Returns:
        Ordered list of SurrealQL DDL statements for both tables and all indexes.

    """
    stmts = build_document_schema(table=table, analyzer_language=analyzer_language)
    stmts.extend(_build_chunk_schema(chunk_table, table))
    stmts.append(
        f"DEFINE INDEX IF NOT EXISTS idx_chunk_content ON TABLE {chunk_table} "
        f"FIELDS content FULLTEXT ANALYZER doc_analyzer BM25({bm25_k1},{bm25_b}) HIGHLIGHTS;",
    )
    if embed:
        stmts.append(
            f"DEFINE INDEX IF NOT EXISTS idx_chunk_embedding ON TABLE {chunk_table} "
            f"FIELDS embedding HNSW DIMENSION {embedding_dimension} TYPE F32 "
            f"DIST {distance_metric} EFC {hnsw_efc} M {hnsw_m};",
        )
    return stmts

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://cdn.jsdelivr.net/gh/xberg-io/assets@v1/banner/readme-banner-dark.svg">
    <img alt="Xberg" width="420" src="https://cdn.jsdelivr.net/gh/xberg-io/assets@v1/banner/readme-banner-light.svg">
  </picture>
</p>

# surrealdb-xberg

Ingest documents into [SurrealDB](https://surrealdb.com/) with [Xberg](https://github.com/xberg-io/xberg)
extraction. The connector manages the schema, deduplicates by content hash, and stores the full Xberg
output — content, metadata, keywords, named entities, tables, summary, detected languages, and quality
score — plus optionally chunks with embeddings for vector and hybrid search.

## Install

```bash
pip install surrealdb-xberg
```

Requires Python 3.11+ and a running SurrealDB instance:

```bash
docker run --rm -p 8000:8000 surrealdb/surrealdb:latest start --user root --pass root
```

## Choose a class

| Class                           | Stores             | Indexes             | Use for                          |
| ------------------------------- | ------------------ | ------------------- | -------------------------------- |
| `DocumentConnector`             | Whole documents    | BM25 on documents   | Keyword search over documents    |
| `DocumentPipeline`              | Documents + chunks | BM25 + HNSW         | Semantic / hybrid search         |
| `DocumentPipeline(embed=False)` | Documents + chunks | BM25 on chunks      | Keyword search over chunks       |

Both are fully async and accept any async SurrealDB connection, session, or transaction.

## Quick start

```python
import asyncio

from surrealdb import AsyncSurreal
from surrealdb_xberg import DocumentPipeline


async def main() -> None:
    async with AsyncSurreal("ws://localhost:8000") as db:
        await db.signin({"username": "root", "password": "root"})
        await db.use("app", "docs")

        pipeline = DocumentPipeline(db=db, embed=True, embedding_model="balanced")
        await pipeline.setup_schema()  # probes the embedding dimension, then creates tables + indexes

        # One extract_batch call for the whole directory, then batched idempotent inserts.
        await pipeline.ingest_directory("./papers", glob="**/*.pdf")

        # Vector search over chunks.
        embedding = await pipeline.embed_query("retrieval augmented generation")
        hits = await pipeline.client.query(
            f"SELECT document.source AS source, content, vector::distance::knn() AS distance "
            f"FROM {pipeline.chunk_table} WHERE embedding <|5,COSINE|> $embedding ORDER BY distance",
            {"embedding": embedding},
        )
        print(hits)


asyncio.run(main())
```

## Ingestion

Every entry point extracts through Xberg, then stores idempotently (deterministic record IDs plus
`INSERT IGNORE`, so re-ingesting the same content is a no-op):

```python
await pipeline.ingest_file("report.pdf")
await pipeline.ingest_files(["a.pdf", "b.docx"])       # single batched extract_batch
await pipeline.ingest_directory("./corpus", glob="**/*.pdf")
await pipeline.ingest_bytes(data=raw, mime_type="application/pdf", source="upload://1")
```

Pass an Xberg `ExtractionConfig` to control extraction — OCR, keywords, NER, summarization, chunking:

```python
from xberg import ExtractionConfig, NerConfig, SummarizationConfig

config = ExtractionConfig(ner=NerConfig(), summarization=SummarizationConfig())
connector = DocumentConnector(db=db, config=config)
```

`DocumentPipeline` injects its embedding config into whatever `ChunkingConfig` you provide (or a default
one), preserving your `max_characters`/`overlap`/`preset`.

## Stored fields

Each document row carries `source`, `content`, `mime_type`, `title`, `authors`, `created_at`,
`metadata`, `quality_score`, `content_hash`, `detected_languages`, `keywords`, `summary`, `entities`
(NER: `category`, `text`, `start`, `end`, `confidence`), and `tables` (`markdown`, `page_number`,
`cells`). `DocumentPipeline` additionally writes a `chunks` table linked back to the parent document via
a record link, each chunk holding its `content`, `chunk_index`, page/byte offsets, and (when enabled) its
`embedding`.

## Notes

- SurrealDB v3 enforces HNSW dimensions server-globally. Use one embedding model per server, or separate
  instances; a dimension conflict raises `DimensionMismatchError`.
- Ingestion failures surface as `IngestionError` (or `DimensionMismatchError`), whether SurrealDB raises a
  `ServerError` or swallows the error into an `INSERT IGNORE` result.

See [`examples/`](./examples) for BM25, vector, hybrid (RRF) search, chunk traversal, and incremental
ingestion. Licensed under MIT.

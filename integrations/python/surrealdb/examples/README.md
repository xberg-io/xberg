# Examples

Runnable examples demonstrating xberg-surrealdb usage patterns.

## Prerequisites

Install the package with dev dependencies:

```bash
uv sync
```

Start a SurrealDB server:

```bash
docker run --rm -p 8000:8000 surrealdb/surrealdb:latest start --user root --pass root
```

## Examples

### `ingest_document.py` — Document ingestion with `DocumentConnector`

Uses `DocumentConnector` with a SurrealDB server. Demonstrates:

- Connecting to SurrealDB via the SDK
- Setting up the schema
- Ingesting a single file

```bash
uv run python examples/ingest_document.py <path-to-file>
```

### `search_patterns.py` — BM25, vector, and hybrid search

Uses `DocumentPipeline` with embeddings. Demonstrates all three search modes in an interactive loop:

- BM25 full-text search with `search::highlight()` for term highlighting
- Vector (HNSW) semantic search with cosine distances
- Hybrid RRF fusion (vector + BM25) via `search::rrf()`

```bash
uv run python examples/search_patterns.py <path-to-directory>
```

### `chunk_explorer.py` — Record link traversal and chunk navigation

Uses `DocumentPipeline` to explore chunk→document relationships. Demonstrates:

- Chunk counts per document via record link aggregation
- BM25 search with parent document metadata (`document.source`, `document.quality_score`)
- Sibling chunk navigation (all chunks from the same document)

```bash
uv run python examples/chunk_explorer.py <path-to-directory>
```

### `incremental_ingest.py` — Deduplication and incremental updates

Uses `DocumentPipeline` to demonstrate idempotent ingestion. Shows:

- First ingestion: all documents and chunks are inserted
- Re-ingestion: INSERT IGNORE skips duplicates (same content hash)
- Adding new files: only new content is added

```bash
uv run python examples/incremental_ingest.py <path-to-directory>
```

## Connection Pattern

All examples connect to a local SurrealDB server using the SDK:

```python
from surrealdb import AsyncSurreal

async with AsyncSurreal("ws://localhost:8000") as db:
    await db.signin({"username": "root", "password": "root"})
    await db.use("default", "default")

    connector = DocumentConnector(db=db)
    ...
```

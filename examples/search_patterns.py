"""Search patterns: BM25, vector, and hybrid search with DocumentPipeline.

Demonstrates all three search modes over chunked documents:
1. BM25 full-text search with highlights
2. Vector (HNSW) semantic search with distances
3. Hybrid RRF fusion (vector + BM25)

Usage:
    # Start SurrealDB first:
    docker run --rm -p 8000:8000 surrealdb/surrealdb:latest start --user root --pass root

    uv run python examples/search_patterns.py <path-to-directory>
"""

import asyncio
import sys
from pathlib import Path
from typing import Any

from surrealdb import AsyncSurreal

from xberg_surrealdb import DocumentPipeline

LIMIT = 5
SEPARATOR = "─" * 60


def _print_bm25(results: list[dict[str, Any]]) -> None:
    if not results:
        print("  (no results)")
        return
    for i, r in enumerate(results, 1):
        score = r.get("score", 0.0)
        source = Path(r.get("source", "")).name
        highlight = r.get("highlight", "")[:200]
        print(f"  [{i}] score: {score:.4f}  source: {source}")
        print(f"      {highlight}")


def _print_vector(results: list[dict[str, Any]]) -> None:
    if not results:
        print("  (no results)")
        return
    for i, r in enumerate(results, 1):
        distance = r.get("distance", 0.0)
        source = Path(r.get("source", "")).name
        content = r.get("content", "")[:150]
        print(f"  [{i}] distance: {distance:.6f}  source: {source}")
        print(f"      {content}")


def _print_hybrid(results: list[dict[str, Any]]) -> None:
    if not results:
        print("  (no results)")
        return
    for i, r in enumerate(results, 1):
        rrf_score = r.get("rrf_score", 0.0)
        source = Path(r.get("source", "")).name
        content = r.get("content", "")[:150]
        print(f"  [{i}] rrf_score: {rrf_score:.6f}  source: {source}")
        print(f"      {content}")


async def main(directory: str) -> None:
    path = Path(directory)
    if not path.is_dir():
        print(f"Not a directory: {directory}")
        sys.exit(1)

    async with AsyncSurreal("ws://localhost:8000") as db:
        await db.signin({"username": "root", "password": "root"})
        await db.use("examples", "search_patterns")

        pipeline = DocumentPipeline(db=db, embed=True, embedding_model="balanced")
        await pipeline.setup_schema()

        files = sorted(p for p in path.rglob("*") if p.is_file())
        if not files:
            print(f"No files found in {directory}")
            sys.exit(1)

        print(f"Ingesting {len(files)} file(s) from {directory}...")
        await pipeline.ingest_directory(directory)

        ct = pipeline.chunk_table
        chunks = await pipeline.client.query(f"SELECT count() FROM {ct} GROUP ALL")  # noqa: S608
        count = chunks[0]["count"] if chunks else 0
        print(f"Done. {count} chunk(s) indexed.\n")
        print("Enter a search query, or 'q' to quit.\n")

        while True:
            query = input("query> ").strip()
            if not query or query.lower() == "q":
                break

            # 1. BM25 full-text search with highlights
            print(f"\n{SEPARATOR}")
            print("1. BM25 Full-Text Search (with highlights)")
            print(SEPARATOR)
            bm25 = await pipeline.client.query(
                f"SELECT document.source AS source, search::score(1) AS score, "  # noqa: S608
                f"search::highlight('<', '>', 1) AS highlight "
                f"FROM {ct} WHERE content @1@ $query "
                f"ORDER BY score DESC LIMIT $limit",
                {"query": query, "limit": LIMIT},
            )
            _print_bm25(bm25)

            # 2. Vector semantic search (HNSW + cosine distance)
            print(f"\n{SEPARATOR}")
            print("2. Vector Semantic Search (HNSW cosine)")
            print(SEPARATOR)
            embedding = await pipeline.embed_query(query)
            vector = await pipeline.client.query(
                f"SELECT document.source AS source, content, vector::distance::knn() AS distance "  # noqa: S608
                f"FROM {ct} WHERE embedding <|{LIMIT},COSINE|> $embedding "
                f"ORDER BY distance",
                {"embedding": embedding},
            )
            _print_vector(vector)

            # 3. Hybrid search (RRF fusion of vector + BM25)
            print(f"\n{SEPARATOR}")
            print("3. Hybrid Search (RRF: vector + BM25)")
            print(SEPARATOR)
            hybrid = await pipeline.client.query(
                f"SELECT * FROM search::rrf(["  # noqa: S608
                f"(SELECT id, content, document.source AS source FROM {ct}"
                f" WHERE embedding <|{LIMIT},COSINE|> $embedding),"
                f"(SELECT id, content, document.source AS source, search::score(1) AS score FROM {ct} "
                f"WHERE content @1@ $query ORDER BY score DESC LIMIT {LIMIT})"
                f"], {LIMIT}, 60);",
                {"embedding": embedding, "query": query},
            )
            _print_hybrid(hybrid)
            print()


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: uv run python examples/search_patterns.py <path-to-directory>")
        sys.exit(1)
    asyncio.run(main(sys.argv[1]))

"""Chunk explorer: record link traversal with DocumentPipeline.

Demonstrates SurrealDB's multi-model record link features:
- Chunk -> document traversal (fetch parent metadata alongside chunks)
- Document -> chunks traversal (find all sibling chunks)
- Aggregation across record links (chunk counts per document)

Usage:
    # Start SurrealDB first:
    docker run --rm -p 8000:8000 surrealdb/surrealdb:latest start --user root --pass root

    uv run python examples/chunk_explorer.py <path-to-directory>
"""

import asyncio
import sys
from pathlib import Path
from typing import Any

from surrealdb import AsyncSurreal

from xberg_surrealdb import DocumentPipeline

SEPARATOR = "─" * 60


def _truncate(text: str, length: int = 120) -> str:
    text = text.replace("\n", " ").strip()
    return f"{text[:length]}..." if len(text) > length else text


def _print_chunks(chunks: list[dict[str, Any]]) -> None:
    if not chunks:
        print("  (no results)")
        return
    for i, c in enumerate(chunks, 1):
        idx = c.get("chunk_index", "?")
        content = _truncate(c.get("content", ""))
        print(f"  [{i}] chunk #{idx}: {content}")


def _print_search_results(results: list[dict[str, Any]]) -> None:
    if not results:
        print("  (no results)")
        return
    for i, r in enumerate(results, 1):
        source = Path(r.get("doc_source", "unknown")).name
        quality = r.get("doc_quality")
        quality_str = f" | quality: {quality:.2f}" if quality is not None else ""
        highlight = _truncate(r.get("highlight", ""), 150)
        print(f"  [{i}] {source}{quality_str}")
        print(f"      {highlight}")


async def show_overview(pipeline: DocumentPipeline) -> None:
    """Print chunk counts per document."""
    ct = pipeline.chunk_table
    print(f"\n{SEPARATOR}")
    print("Documents and chunk counts")
    print(SEPARATOR)
    stats = await pipeline.client.query(
        f"SELECT document.source AS source, count() AS chunks "  # noqa: S608
        f"FROM {ct} GROUP BY document",
    )
    for row in stats:
        raw_source = row.get("source", "unknown")
        source = raw_source[0] if isinstance(raw_source, list) else raw_source
        print(f"  {Path(source).name}: {row.get('chunks', 0)} chunk(s)")


async def search_chunks(pipeline: DocumentPipeline, query: str) -> list[dict[str, Any]]:
    """BM25 search with parent document metadata via record links."""
    ct = pipeline.chunk_table
    print(f"\n{SEPARATOR}")
    print(f'BM25 search: "{query}" (with parent document metadata)')
    print(SEPARATOR)
    results = await pipeline.client.query(
        f"SELECT *, document.source AS doc_source, "  # noqa: S608
        f"document.quality_score AS doc_quality, "
        f"search::score(1) AS score, "
        f"search::highlight('<', '>', 1) AS highlight "
        f"FROM {ct} WHERE content @1@ $query "
        f"ORDER BY score DESC LIMIT 5",
        {"query": query},
    )
    _print_search_results(results)
    return results


async def show_siblings(pipeline: DocumentPipeline, result: dict[str, Any]) -> None:
    """Show all chunks from the same document as the given result."""
    ct = pipeline.chunk_table
    doc_id = result.get("document")
    if not doc_id:
        print("No document link on that result.")
        return
    doc_name = Path(result.get("doc_source", str(doc_id))).name
    print(f"\n{SEPARATOR}")
    print(f"All chunks from document: {doc_name}")
    print(SEPARATOR)
    siblings = await pipeline.client.query(
        f"SELECT * FROM {ct} WHERE document = $doc_id "  # noqa: S608
        f"ORDER BY chunk_index",
        {"doc_id": doc_id},
    )
    _print_chunks(siblings)


async def main(directory: str) -> None:
    path = Path(directory)
    if not path.is_dir():
        print(f"Not a directory: {directory}")
        sys.exit(1)

    async with AsyncSurreal("ws://localhost:8000") as db:
        await db.signin({"username": "root", "password": "root"})
        await db.use("examples", "chunk_explorer")

        pipeline = DocumentPipeline(db=db, embed=False)
        await pipeline.setup_schema()

        files = sorted(p for p in path.rglob("*") if p.is_file())
        if not files:
            print(f"No files found in {directory}")
            sys.exit(1)

        print(f"Ingesting {len(files)} file(s) from {directory}...")
        await pipeline.ingest_directory(directory)

        await show_overview(pipeline)

        print(f"\n{SEPARATOR}")
        print("Commands:")
        print("  <query>        — BM25 search with parent document metadata")
        print("  siblings <N>   — show all chunks from the same document as result N")
        print("  q              — quit")
        print(SEPARATOR)

        last_results: list[dict[str, Any]] = []

        while True:
            cmd = input("\n> ").strip()
            if not cmd or cmd.lower() == "q":
                break

            if cmd.lower().startswith("siblings"):
                parts = cmd.split()
                if len(parts) < 2 or not parts[1].isdigit():
                    print("Usage: siblings <result-number>")
                    continue
                idx = int(parts[1]) - 1
                if idx < 0 or idx >= len(last_results):
                    print(f"No result #{idx + 1}. Run a search first.")
                    continue
                await show_siblings(pipeline, last_results[idx])
            else:
                last_results = await search_chunks(pipeline, cmd)


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: uv run python examples/chunk_explorer.py <path-to-directory>")
        sys.exit(1)
    asyncio.run(main(sys.argv[1]))

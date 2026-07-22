"""Incremental ingestion: deduplication and idempotency demo.

Demonstrates that re-ingesting the same content is a safe no-op:
- First run: all documents and chunks are inserted
- Second run: INSERT IGNORE skips duplicates (same content hash)
- New files: only new content is added

Usage:
    # Start SurrealDB first:
    docker run --rm -p 8000:8000 surrealdb/surrealdb:latest start --user root --pass root

    uv run python examples/incremental_ingest.py <path-to-directory>
"""

import asyncio
import sys
from pathlib import Path

from surrealdb import AsyncSurreal

from xberg_surrealdb import DocumentPipeline

SEPARATOR = "─" * 60


async def count(pipeline: DocumentPipeline) -> tuple[int, int]:
    """Return (doc_count, chunk_count)."""
    t = pipeline.table
    ct = pipeline.chunk_table
    docs = await pipeline.client.query(f"SELECT count() FROM {t} GROUP ALL")  # noqa: S608
    chunks = await pipeline.client.query(f"SELECT count() FROM {ct} GROUP ALL")  # noqa: S608
    return (
        docs[0]["count"] if docs else 0,
        chunks[0]["count"] if chunks else 0,
    )


async def ingest_and_report(
    pipeline: DocumentPipeline, directory: str, label: str, prev: tuple[int, int] | None = None
) -> tuple[int, int]:
    """Ingest a directory and print before/after counts."""
    print(f"\n{SEPARATOR}")
    print(label)
    print(SEPARATOR)
    await pipeline.ingest_directory(directory)
    docs, chunks = await count(pipeline)
    if prev:
        print(f"  Documents: {docs} (was {prev[0]})")
        print(f"  Chunks:    {chunks} (was {prev[1]})")
    else:
        print(f"  Documents: {docs}")
        print(f"  Chunks:    {chunks}")
    return docs, chunks


async def main(directory: str) -> None:
    path = Path(directory)
    if not path.is_dir():
        print(f"Not a directory: {directory}")
        sys.exit(1)

    files = sorted(p for p in path.rglob("*") if p.is_file())
    if not files:
        print(f"No files found in {directory}")
        sys.exit(1)

    async with AsyncSurreal("ws://localhost:8000") as db:
        await db.signin({"username": "root", "password": "root"})
        await db.use("examples", "incremental_ingest")

        pipeline = DocumentPipeline(db=db, embed=False)
        await pipeline.setup_schema()

        # Phase 1: First ingestion
        first = await ingest_and_report(pipeline, directory, f"First ingestion: {len(files)} file(s)")

        # Phase 2: Re-ingest same directory (should be a no-op)
        second = await ingest_and_report(pipeline, directory, "Re-ingesting same directory (dedup test)", prev=first)
        if second == first:
            print("  Dedup confirmed: no duplicates created.")
        else:
            print("  WARNING: counts changed — dedup may not be working.")

        # Phase 3: Add a new file and re-ingest
        new_file = path / "_incremental_test.txt"
        new_file.write_text(
            "This is a new document added after the initial ingestion. "
            "It contains unique content that does not exist in any other file."
        )
        try:
            third = await ingest_and_report(pipeline, directory, "Adding a new file and re-ingesting", prev=second)
            new_docs = third[0] - second[0]
            new_chunks = third[1] - second[1]
            print(f"  New documents: {new_docs}, new chunks: {new_chunks}")
            if new_docs == 1 and new_chunks >= 1:
                print("  Incremental ingestion confirmed: only new content was added.")
        finally:
            new_file.unlink(missing_ok=True)
            print(f"\n  Cleaned up {new_file.name}")


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: uv run python examples/incremental_ingest.py <path-to-directory>")
        sys.exit(1)
    asyncio.run(main(sys.argv[1]))

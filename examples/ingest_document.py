"""Ingest a document and query it with raw SurQL via DocumentConnector.

Demonstrates that DocumentConnector handles extraction + schema setup,
while SurrealDB's query features (BM25 scoring, highlights, filters)
are accessed directly through connector.client.

Usage:
    # Start SurrealDB first:
    docker run --rm -p 8000:8000 surrealdb/surrealdb:latest start --allow-all --user root --pass root

    uv run python examples/ingest_document.py <file>...
"""

import asyncio
import sys
from pathlib import Path
from typing import Any

from surrealdb import AsyncSurreal

from xberg_surrealdb import DocumentConnector


def _print_doc(doc: dict[str, Any]) -> None:
    """Print a summary of a stored document."""
    print(f"  id      : {doc.get('id')}")
    print(f"  source  : {doc.get('source')}")
    print(f"  type    : {doc.get('mime_type')}")
    print(f"  title   : {doc.get('title', '(none)')}")
    print(f"  quality : {doc.get('quality_score')}")
    langs = doc.get("detected_languages") or []
    if langs:
        print(f"  langs   : {', '.join(lang['language'] for lang in langs)}")
    keywords = doc.get("keywords") or []
    if keywords:
        print(f"  keywords: {', '.join(kw['keyword'] for kw in keywords[:8])}")
    content = doc.get("content", "")
    print(f"  content : {content[:120]}{'...' if len(content) > 120 else ''}")
    print()


def _print_results(results: list[dict[str, Any]]) -> None:
    """Print BM25 search results with score and snippet."""
    for i, r in enumerate(results, 1):
        score = r.get("score", 0.0)
        source = r.get("source", "unknown")
        snippet = r.get("snippet", "")[:300]
        print(f"\n--- Result {i} (score: {score:.4f}) ---")
        print(f"Source: {source}")
        print(f"Snippet: {snippet}")
    print()


async def main(file_paths: list[str]) -> None:
    paths = [Path(p) for p in file_paths]
    for path in paths:
        if not path.exists():
            print(f"File not found: {path}")
            sys.exit(1)

    async with AsyncSurreal("ws://localhost:8000") as db:
        await db.signin({"username": "root", "password": "root"})
        await db.use("examples", "connector_demo")

        connector = DocumentConnector(db=db)
        await connector.setup_schema()

        # --- Ingest ---
        print(f"Ingesting {len(paths)} file(s)...")
        await connector.ingest_files(paths)
        print("Done.\n")

        table = connector.table

        # --- Inspect stored documents ---
        docs = await connector.client.query(f"SELECT * FROM {table}")  # noqa: S608
        print(f"Stored {len(docs)} document(s).\n")
        for doc in docs:
            _print_doc(doc)

        # --- Demo: auto-search using extracted keywords ---
        if docs and (kws := docs[0].get("keywords")):
            demo_query = kws[0]["keyword"]
            print(f'--- Demo: BM25 search for "{demo_query}" ---')
            results = await connector.client.query(
                f"SELECT source, search::score(1) AS score, "  # noqa: S608
                f"search::highlight('<', '>', 1) AS snippet "
                f"FROM {table} WHERE content @1@ $query "
                f"ORDER BY score DESC LIMIT 3",
                {"query": demo_query},
            )
            for r in results:
                snippet = r.get("snippet", "")[:200]
                print(f"  score {r.get('score', 0):.4f} | {snippet}")
            print()
        elif docs:
            print("(no keywords extracted — skipping demo search)\n")

        # --- Interactive search loop ---
        print("Enter a search query to run BM25 search, or 'q' to quit.\n")
        while True:
            query = input("query> ").strip()
            if not query or query.lower() == "q":
                break

            results = await connector.client.query(
                f"SELECT *, search::score(1) AS score, "  # noqa: S608
                f"search::highlight('<', '>', 1) AS snippet "
                f"FROM {table} WHERE content @1@ $query "
                f"ORDER BY score DESC LIMIT $limit",
                {"query": query, "limit": 5},
            )
            if not results:
                print("No results.\n")
                continue

            _print_results(results)


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: uv run python examples/ingest_document.py <file>...")
        sys.exit(1)
    asyncio.run(main(sys.argv[1:]))

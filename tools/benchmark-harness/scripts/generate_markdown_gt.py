#!/usr/bin/env -S uv run --no-project --script
# /// script
# requires-python = ">=3.10"
# dependencies = ["google-genai>=1.0"]
# ///
"""Generate proper markdown ground truth from PDF documents using Gemini.

Reads benchmark fixture JSON files to locate PDFs, sends each to Gemini 2.5 Flash
via Vertex AI, and saves the extracted markdown to the ground truth directory.

Usage:
    uv run tools/benchmark-harness/scripts/generate_markdown_gt.py [OPTIONS]

Examples:
    # Generate for all nougat + pdfa documents
    uv run tools/benchmark-harness/scripts/generate_markdown_gt.py

    # Generate for a specific document
    uv run tools/benchmark-harness/scripts/generate_markdown_gt.py --filter nougat_001

    # Dry run to see what would be processed
    uv run tools/benchmark-harness/scripts/generate_markdown_gt.py --dry-run

    # Force regeneration of existing files
    uv run tools/benchmark-harness/scripts/generate_markdown_gt.py --force
"""

from __future__ import annotations

import argparse
import json
import signal
import sys
import time
from pathlib import Path

from google import genai
from google.genai.types import GenerateContentConfig, Part

EXTRACTION_PROMPT = """\
Extract the complete text content of this PDF document as clean Markdown.

Rules:
- Use proper heading hierarchy (# for document title, ## for major sections, ### for subsections)
- Render tables using markdown table syntax with | delimiters and --- separator row
- Use numbered lists (1. 2. 3.) and bullet lists (- item) where the document uses them
- Preserve emphasis: **bold** and *italic* where the original uses them
- Use ``` code blocks for code snippets, formulas, or monospace content
- Use <!-- image --> as a placeholder where figures or images appear
- Omit page numbers, running headers/footers, and watermarks
- Preserve the document's reading order
- Do NOT invent or hallucinate content — only extract what is actually in the document
- Do NOT wrap the output in a markdown code fence — return raw markdown directly
- For multi-column layouts, read left column first, then right column
- For forms with label-value pairs, use **Label:** Value format
"""


def get_repo_root() -> Path:
    current = Path(__file__).resolve().parent
    while current != current.parent:
        if (current / "Cargo.toml").exists() and (current / "test_documents").exists():
            return current
        current = current.parent
    raise RuntimeError("Could not find repository root")


def discover_fixtures(fixtures_dir: Path, name_filter: str | None = None) -> list[dict]:
    """Find PDF fixtures that need markdown ground truth."""
    results = []
    for fixture_path in sorted(fixtures_dir.rglob("*.json")):
        try:
            with open(fixture_path) as f:
                fixture = json.load(f)
        except (json.JSONDecodeError, OSError):
            continue

        if fixture.get("file_type") != "pdf":
            continue

        name = fixture_path.stem
        if name_filter and name_filter not in name:
            continue

        doc_rel = fixture.get("document", "")
        if not doc_rel:
            continue

        doc_path = (fixture_path.parent / doc_rel).resolve()
        if not doc_path.exists():
            continue

        results.append(
            {
                "name": name,
                "fixture_path": fixture_path,
                "doc_path": doc_path,
                "fixture": fixture,
            }
        )

    return results


class _Timeout(Exception):
    pass


def _timeout_handler(signum, frame):
    raise _Timeout("API call timed out")


def generate_markdown(
    client: genai.Client,
    pdf_path: Path,
    model: str,
    timeout: int = 120,
) -> str:
    """Send PDF to Gemini and get markdown extraction."""
    pdf_bytes = pdf_path.read_bytes()

    old_handler = signal.signal(signal.SIGALRM, _timeout_handler)
    signal.alarm(timeout)
    try:
        response = client.models.generate_content(
            model=model,
            contents=[
                Part.from_bytes(data=pdf_bytes, mime_type="application/pdf"),
                EXTRACTION_PROMPT,
            ],
            config=GenerateContentConfig(
                temperature=0.1,
                max_output_tokens=8192,
            ),
        )
    finally:
        signal.alarm(0)
        signal.signal(signal.SIGALRM, old_handler)

    text = response.text or ""

    # Strip markdown code fence wrapper if Gemini added one
    if text.startswith("```markdown\n"):
        text = text[len("```markdown\n") :]
        text = text.removesuffix("\n```")
    elif text.startswith("```md\n"):
        text = text[len("```md\n") :]
        text = text.removesuffix("\n```")
    elif text.startswith("```\n"):
        text = text[len("```\n") :]
        text = text.removesuffix("\n```")

    return text.strip() + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description="Generate markdown ground truth from PDFs using Gemini")
    parser.add_argument(
        "--filter", type=str, default=None, help="Only process fixtures whose name contains this string"
    )
    parser.add_argument("--dry-run", action="store_true", help="Show what would be processed without calling the API")
    parser.add_argument("--force", action="store_true", help="Regenerate even if .md file already exists")
    parser.add_argument(
        "--model", type=str, default="gemini-2.0-flash", help="Gemini model to use (default: gemini-2.0-flash)"
    )
    parser.add_argument("--project", type=str, default="boxwood-spirit-479620-r5", help="GCP project ID")
    parser.add_argument("--location", type=str, default="us-central1", help="Vertex AI location")
    parser.add_argument("--delay", type=float, default=1.0, help="Delay between API calls in seconds (rate limiting)")
    parser.add_argument("--timeout", type=int, default=120, help="Per-request timeout in seconds (default: 120)")
    parser.add_argument("--max-size", type=int, default=None, help="Skip PDFs larger than this many KB")
    args = parser.parse_args()

    repo_root = get_repo_root()
    fixtures_dir = repo_root / "tools" / "benchmark-harness" / "fixtures"
    gt_dir = repo_root / "test_documents" / "ground_truth" / "pdf"

    print(f"Repository root: {repo_root}")
    print(f"Fixtures dir:    {fixtures_dir}")
    print(f"Output dir:      {gt_dir}")
    print(f"Model:           {args.model}")
    if args.dry_run:
        print("DRY RUN MODE\n")

    fixtures = discover_fixtures(fixtures_dir, args.filter)
    print(f"Found {len(fixtures)} PDF fixtures")

    if not args.dry_run:
        client = genai.Client(
            vertexai=True,
            project=args.project,
            location=args.location,
        )

    stats = {"generated": 0, "skipped": 0, "errors": 0}

    for item in fixtures:
        name = item["name"]
        md_path = gt_dir / f"{name}.md"
        file_size_kb = item["doc_path"].stat().st_size / 1024

        if md_path.exists() and not args.force:
            stats["skipped"] += 1
            continue

        if args.max_size and file_size_kb > args.max_size:
            print(f"  Skipping {name} ({file_size_kb:.0f} KB > {args.max_size} KB)")
            stats["skipped"] += 1
            continue

        if args.dry_run:
            print(f"  [DRY] {name} ({file_size_kb:.0f} KB)")
            stats["generated"] += 1
            continue

        print(f"  Processing {name} ({file_size_kb:.0f} KB)...", end=" ", flush=True)
        try:
            start = time.time()
            markdown = generate_markdown(client, item["doc_path"], args.model, timeout=args.timeout)
            elapsed = time.time() - start

            gt_dir.mkdir(parents=True, exist_ok=True)
            md_path.write_text(markdown, encoding="utf-8")

            # Quick quality check
            lines = markdown.strip().split("\n")
            headings = sum(1 for l in lines if l.startswith("#"))
            tables = sum(1 for l in lines if "|" in l and "---" not in l)
            print(f"OK ({elapsed:.1f}s, {len(lines)} lines, {headings} headings, {tables} table rows)")
            stats["generated"] += 1

            time.sleep(args.delay)

        except _Timeout:
            print(f"TIMEOUT ({args.timeout}s)")
            stats["errors"] += 1
        except Exception as e:
            print(f"ERROR: {e}")
            stats["errors"] += 1

    print(f"\n{'=' * 50}")
    print(f"Generated: {stats['generated']}")
    print(f"Skipped:   {stats['skipped']} (already exist)")
    print(f"Errors:    {stats['errors']}")

    return 0 if stats["errors"] == 0 else 1


if __name__ == "__main__":
    sys.exit(main())

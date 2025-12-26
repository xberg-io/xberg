"""pdfplumber extraction wrapper for benchmark harness.

Supports two modes:
- sync: extract text page-by-page (sequential)
- batch: process multiple files (simulated batch using loop)
"""

from __future__ import annotations

import json
import sys
import time
from typing import Any

import pdfplumber


def extract_sync(file_path: str) -> dict[str, Any]:
    """Extract using synchronous single-file API."""
    start = time.perf_counter()

    with pdfplumber.open(file_path) as pdf:
        text_parts = []
        for page in pdf.pages:
            page_text = page.extract_text()
            if page_text:
                text_parts.append(page_text)

        markdown = "\n\n".join(text_parts)

    duration_ms = (time.perf_counter() - start) * 1000.0

    return {
        "content": markdown,
        "metadata": {"framework": "pdfplumber"},
        "_extraction_time_ms": duration_ms,
    }


def extract_batch(file_paths: list[str]) -> list[dict[str, Any]]:
    """Extract multiple files (simulated batch - pdfplumber has no native batch API)."""
    start = time.perf_counter()

    results = []
    for file_path in file_paths:
        try:
            with pdfplumber.open(file_path) as pdf:
                text_parts = []
                for page in pdf.pages:
                    page_text = page.extract_text()
                    if page_text:
                        text_parts.append(page_text)

                markdown = "\n\n".join(text_parts)
                results.append({
                    "content": markdown,
                    "metadata": {"framework": "pdfplumber"},
                })
        except Exception as e:
            results.append({
                "content": "",
                "metadata": {
                    "framework": "pdfplumber",
                    "error": str(e),
                },
            })

    total_duration_ms = (time.perf_counter() - start) * 1000.0
    per_file_duration_ms = total_duration_ms / len(file_paths) if file_paths else 0

    for result in results:
        result["_extraction_time_ms"] = per_file_duration_ms
        result["_batch_total_ms"] = total_duration_ms

    return results


def main() -> None:
    if len(sys.argv) < 3:
        print("Usage: pdfplumber_extract.py <mode> <file_path> [additional_files...]", file=sys.stderr)
        print("Modes: sync, batch", file=sys.stderr)
        sys.exit(1)

    mode = sys.argv[1]
    file_paths = sys.argv[2:]

    try:
        if mode == "sync":
            if len(file_paths) != 1:
                print("Error: sync mode requires exactly one file", file=sys.stderr)
                sys.exit(1)
            payload = extract_sync(file_paths[0])
            print(json.dumps(payload), end="")

        elif mode == "batch":
            if len(file_paths) < 1:
                print("Error: batch mode requires at least one file", file=sys.stderr)
                sys.exit(1)

            if len(file_paths) == 1:
                results = extract_batch(file_paths)
                print(json.dumps(results[0]), end="")
            else:
                results = extract_batch(file_paths)
                print(json.dumps(results), end="")

        else:
            print(f"Error: Unknown mode '{mode}'. Use sync or batch", file=sys.stderr)
            sys.exit(1)

    except Exception as e:
        print(f"Error extracting with pdfplumber: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()

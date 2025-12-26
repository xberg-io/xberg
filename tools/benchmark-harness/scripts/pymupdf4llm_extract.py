"""PyMuPDF4LLM extraction wrapper for benchmark harness."""

from __future__ import annotations

import json
import sys
import time

import pymupdf4llm


def main() -> None:
    if len(sys.argv) != 2:
        print("Usage: pymupdf4llm_extract.py <file_path>", file=sys.stderr)
        sys.exit(1)

    file_path = sys.argv[1]

    try:
        start = time.perf_counter()
        markdown = pymupdf4llm.to_markdown(file_path)
        duration_ms = (time.perf_counter() - start) * 1000.0

        payload = {
            "content": markdown,
            "metadata": {"framework": "pymupdf4llm"},
            "_extraction_time_ms": duration_ms,
        }
        print(json.dumps(payload), end="")
    except Exception as e:
        print(f"Error extracting with PyMuPDF4LLM: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()

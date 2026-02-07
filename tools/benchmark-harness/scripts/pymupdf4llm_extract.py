# /// script
# requires-python = ">=3.10"
# dependencies = [
#     "pymupdf4llm>=0.0.17",
#     "pymupdf-layout>=0.0.1",
#     "Pillow>=10.0.0",
# ]
# ///
"""PyMuPDF4LLM extraction wrapper for benchmark harness."""

from __future__ import annotations

import json
import sys
import time

# Import pymupdf.layout BEFORE pymupdf4llm to enable improved layout analysis
# and suppress the "Consider using the pymupdf_layout package" info message.
import pymupdf.layout  # noqa: F401
import pymupdf4llm

# Suppress MuPDF C-level error/warning messages that can corrupt the
# persistent server's line-based JSON protocol on stdout.
# See: https://github.com/pymupdf/PyMuPDF/issues/606
import pymupdf
pymupdf.TOOLS.mupdf_display_errors(False)


def extract_sync(file_path: str) -> dict:
    """Extract using PyMuPDF4LLM."""
    start = time.perf_counter()
    markdown = pymupdf4llm.to_markdown(file_path, show_progress=False, write_images=False)
    duration_ms = (time.perf_counter() - start) * 1000.0

    return {
        "content": markdown,
        "metadata": {"framework": "pymupdf4llm"},
        "_extraction_time_ms": duration_ms,
    }


def run_server() -> None:
    """Persistent server mode."""
    for line in sys.stdin:
        file_path = line.strip()
        if not file_path:
            continue
        try:
            payload = extract_sync(file_path)
            print(json.dumps(payload), flush=True)
        except Exception as e:
            print(json.dumps({"error": str(e), "_extraction_time_ms": 0}), flush=True)


def main() -> None:
    ocr_enabled = False
    args = []
    for arg in sys.argv[1:]:
        if arg == "--ocr":
            ocr_enabled = True
        elif arg == "--no-ocr":
            ocr_enabled = False
        else:
            args.append(arg)

    if len(args) < 1:
        print("Usage: pymupdf4llm_extract.py [--ocr|--no-ocr] <mode> <file_path>", file=sys.stderr)
        print("Modes: sync, server", file=sys.stderr)
        sys.exit(1)

    mode = args[0]
    if mode == "server":
        run_server()
    elif mode == "sync":
        if len(args) < 2:
            print("Error: sync mode requires a file path", file=sys.stderr)
            sys.exit(1)
        file_path = args[1]
        try:
            payload = extract_sync(file_path)
            print(json.dumps(payload), end="")
        except Exception as e:
            print(f"Error extracting with PyMuPDF4LLM: {e}", file=sys.stderr)
            sys.exit(1)
    else:
        # Legacy fallback for direct file path
        try:
            payload = extract_sync(args[0])
            print(json.dumps(payload), end="")
        except Exception as e:
            print(f"Error extracting with PyMuPDF4LLM: {e}", file=sys.stderr)
            sys.exit(1)


if __name__ == "__main__":
    main()

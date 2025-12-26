"""MinerU extraction wrapper for benchmark harness.

Supports two modes:
- sync: process single file using CLI
- batch: process multiple files using CLI batch capability
"""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from typing import Any


def extract_sync(file_path: str) -> dict[str, Any]:
    """Extract using MinerU CLI for single file."""
    start = time.perf_counter()

    with tempfile.TemporaryDirectory() as tmpdir:
        output_dir = Path(tmpdir) / "output"

        # Run mineru CLI
        result = subprocess.run(
            ["mineru", "-p", file_path, "-o", str(output_dir)],
            capture_output=True,
            text=True,
            check=False,
        )

        if result.returncode != 0:
            raise RuntimeError(f"MinerU extraction failed: {result.stderr}")

        # Find markdown output (MinerU creates .md files)
        md_files = list(output_dir.rglob("*.md"))
        if not md_files:
            raise RuntimeError("No markdown output found from MinerU")

        # Read first markdown file found
        markdown = md_files[0].read_text(encoding="utf-8")

    duration_ms = (time.perf_counter() - start) * 1000.0

    return {
        "content": markdown,
        "metadata": {"framework": "mineru"},
        "_extraction_time_ms": duration_ms,
    }


def extract_batch(file_paths: list[str]) -> list[dict[str, Any]]:
    """Extract multiple files using MinerU CLI in sequence."""
    start = time.perf_counter()

    results = []
    for file_path in file_paths:
        try:
            with tempfile.TemporaryDirectory() as tmpdir:
                output_dir = Path(tmpdir) / "output"

                result = subprocess.run(
                    ["mineru", "-p", file_path, "-o", str(output_dir)],
                    capture_output=True,
                    text=True,
                    check=False,
                )

                if result.returncode != 0:
                    results.append({
                        "content": "",
                        "metadata": {
                            "framework": "mineru",
                            "error": f"Extraction failed: {result.stderr}",
                        },
                    })
                    continue

                md_files = list(output_dir.rglob("*.md"))
                if not md_files:
                    results.append({
                        "content": "",
                        "metadata": {
                            "framework": "mineru",
                            "error": "No markdown output found",
                        },
                    })
                    continue

                markdown = md_files[0].read_text(encoding="utf-8")
                results.append({
                    "content": markdown,
                    "metadata": {"framework": "mineru"},
                })
        except Exception as e:
            results.append({
                "content": "",
                "metadata": {
                    "framework": "mineru",
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
        print("Usage: mineru_extract.py <mode> <file_path> [additional_files...]", file=sys.stderr)
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
        print(f"Error extracting with MinerU: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()

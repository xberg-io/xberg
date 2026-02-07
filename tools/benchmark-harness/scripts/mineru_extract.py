# /// script
# requires-python = ">=3.10"
# dependencies = [
#     "mineru[pipeline]>=2.6.7",
#     "onnxruntime",
# ]
# ///
"""MinerU extraction wrapper for benchmark harness.

Supports three modes:
- sync: process single file
- batch: process multiple files
- server: persistent mode reading paths from stdin

Attempts to use MinerU's Python API directly for better performance.
Falls back to CLI subprocess if the Python API is not available.
"""

from __future__ import annotations

import os

# Force CPU-only mode to avoid GPU discovery errors in CI
os.environ.setdefault("CUDA_VISIBLE_DEVICES", "")
os.environ.setdefault("ONNXRUNTIME_PROVIDERS", "CPUExecutionProvider")
os.environ.setdefault("MINERU_DEVICE_MODE", "cpu")

import json
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from typing import Any

# Try importing MinerU's Python API to avoid subprocess overhead.
# The API surface has changed across versions, so we attempt several known entry points.
try:
    from magic_pdf.pipe.UNIPipe import UNIPipe  # noqa: F401
    HAS_PYTHON_API = True
except ImportError:
    HAS_PYTHON_API = False


def _extract_via_cli(file_path: str, ocr_enabled: bool) -> str:
    """Extract using MinerU CLI (fallback)."""
    cmd = ["mineru", "-p", file_path, "-b", "pipeline", "-d", "cpu"]
    if not ocr_enabled:
        cmd.extend(["--method", "txt"])

    with tempfile.TemporaryDirectory() as tmpdir:
        output_dir = Path(tmpdir) / "output"
        cmd.extend(["-o", str(output_dir)])

        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            check=False,
        )

        # Check for output files first — ONNX Runtime may emit warnings to
        # stderr even when extraction succeeds.
        md_files = list(output_dir.rglob("*.md"))
        if md_files:
            return md_files[0].read_text(encoding="utf-8")

        if result.returncode != 0:
            raise RuntimeError(f"MinerU extraction failed: {result.stderr}")

        raise RuntimeError("No markdown output found from MinerU")


def _extract_via_api(file_path: str, ocr_enabled: bool) -> str:
    """Extract using MinerU Python API (preferred, avoids subprocess overhead)."""
    # NOTE: The MinerU Python API is not yet stable. This is a best-effort attempt
    # using the UNIPipe interface. If this fails at runtime, the caller should
    # fall back to CLI extraction.
    from magic_pdf.pipe.UNIPipe import UNIPipe
    from magic_pdf.rw.DiskReaderWriter import DiskReaderWriter

    pdf_bytes = Path(file_path).read_bytes()

    with tempfile.TemporaryDirectory() as tmpdir:
        writer = DiskReaderWriter(tmpdir)
        method = "ocr" if ocr_enabled else "txt"
        pipe = UNIPipe(pdf_bytes, {"_pdf_type": "", "model_list": []}, writer, method=method)
        pipe.pipe_classify()
        pipe.pipe_analyze()
        pipe.pipe_parse()
        md_content = pipe.pipe_mk_markdown(str(Path(file_path).stem), tmpdir)
        return md_content


def extract_sync(file_path: str, ocr_enabled: bool) -> dict[str, Any]:
    """Extract a single file using the best available method."""
    start = time.perf_counter()

    if HAS_PYTHON_API:
        try:
            markdown = _extract_via_api(file_path, ocr_enabled)
        except Exception:
            # Fall back to CLI if Python API fails at runtime
            markdown = _extract_via_cli(file_path, ocr_enabled)
    else:
        markdown = _extract_via_cli(file_path, ocr_enabled)

    duration_ms = (time.perf_counter() - start) * 1000.0

    return {
        "content": markdown,
        "metadata": {"framework": "mineru"},
        "_extraction_time_ms": duration_ms,
    }


def extract_batch(file_paths: list[str], ocr_enabled: bool) -> list[dict[str, Any]]:
    """Extract multiple files in sequence."""
    start = time.perf_counter()

    results = []
    for file_path in file_paths:
        try:
            payload = extract_sync(file_path, ocr_enabled)
            # Remove per-file timing; we'll replace with batch timing below
            payload.pop("_extraction_time_ms", None)
            results.append(payload)
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


def run_server(ocr_enabled: bool) -> None:
    """Persistent server mode: read paths from stdin, write JSON to stdout."""
    for line in sys.stdin:
        file_path = line.strip()
        if not file_path:
            continue
        try:
            payload = extract_sync(file_path, ocr_enabled)
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
        print("Usage: mineru_extract.py [--ocr|--no-ocr] <mode> <file_path> [additional_files...]", file=sys.stderr)
        print("Modes: sync, batch, server", file=sys.stderr)
        sys.exit(1)

    mode = args[0]
    file_paths = args[1:]

    try:
        if mode == "server":
            run_server(ocr_enabled)

        elif mode == "sync":
            if len(file_paths) != 1:
                print("Error: sync mode requires exactly one file", file=sys.stderr)
                sys.exit(1)
            payload = extract_sync(file_paths[0], ocr_enabled)
            print(json.dumps(payload), end="")

        elif mode == "batch":
            if len(file_paths) < 1:
                print("Error: batch mode requires at least one file", file=sys.stderr)
                sys.exit(1)

            if len(file_paths) == 1:
                results = extract_batch(file_paths, ocr_enabled)
                print(json.dumps(results[0]), end="")
            else:
                results = extract_batch(file_paths, ocr_enabled)
                print(json.dumps(results), end="")

        else:
            print(f"Error: Unknown mode '{mode}'. Use sync, batch, or server", file=sys.stderr)
            sys.exit(1)

    except Exception as e:
        print(f"Error extracting with MinerU: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()

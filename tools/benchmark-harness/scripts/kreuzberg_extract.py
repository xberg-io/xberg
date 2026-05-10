"""Kreuzberg Python extraction wrapper for benchmark harness.

Supports four modes:
- sync: extract_file_sync() - synchronous extraction
- async: extract_file() - asynchronous extraction
- batch: batch_extract_files_sync() - synchronous batch extraction
- server: persistent mode reading paths from stdin
"""

from __future__ import annotations

import asyncio
import json
import platform
import resource
import sys
import time
from typing import Any


def _get_peak_memory_bytes() -> int:
    """Get peak RSS memory in bytes for the current process.

    Uses resource.getrusage(RUSAGE_SELF). On Linux ru_maxrss is in KB,
    on macOS/BSD it is in bytes.
    """
    usage = resource.getrusage(resource.RUSAGE_SELF)
    if platform.system() == "Linux":
        return usage.ru_maxrss * 1024
    return usage.ru_maxrss

from kreuzberg import (
    ExtractionConfig,
    OcrConfig,
    batch_extract_files_sync,
    extract_file,
    extract_file_sync,
)


def _determine_ocr_used(metadata: dict[str, Any], ocr_enabled: bool) -> bool:
    """Determine if OCR was actually used based on extraction result metadata.

    Mirrors the native Rust adapter logic: OCR is used when format_type is "ocr",
    or when format_type is "image" or "pdf" and OCR was enabled in config.
    """
    format_type = (metadata or {}).get("format_type", "")
    if format_type == "ocr":
        return True
    if format_type in ("image", "pdf") and ocr_enabled:
        return True
    return False


def _build_config(ocr_enabled: bool, output_format: str, *, force_ocr: bool = False) -> ExtractionConfig:
    """Build ExtractionConfig with cache off and the requested output format."""
    config = ExtractionConfig(use_cache=False, output_format=output_format)
    if ocr_enabled:
        config.ocr = OcrConfig(backend="tesseract")
    if force_ocr:
        config.force_ocr = True
    return config


def extract_sync(
    file_path: str, ocr_enabled: bool, output_format: str = "markdown", *, force_ocr: bool = False
) -> dict[str, Any]:
    """Extract using synchronous API."""
    config = _build_config(ocr_enabled, output_format, force_ocr=force_ocr)

    start = time.perf_counter()
    result = extract_file_sync(file_path, config=config)
    duration_ms = (time.perf_counter() - start) * 1000.0

    metadata = result.metadata or {}
    return {
        "content": result.content,
        "metadata": metadata,
        "_extraction_time_ms": duration_ms,
        "_ocr_used": _determine_ocr_used(metadata, ocr_enabled or force_ocr),
        "_peak_memory_bytes": _get_peak_memory_bytes(),
    }


async def extract_async(
    file_path: str, ocr_enabled: bool, output_format: str = "markdown", *, force_ocr: bool = False
) -> dict[str, Any]:
    """Extract using asynchronous API."""
    config = _build_config(ocr_enabled, output_format, force_ocr=force_ocr)

    start = time.perf_counter()
    result = await extract_file(file_path, config=config)
    duration_ms = (time.perf_counter() - start) * 1000.0

    metadata = result.metadata or {}
    return {
        "content": result.content,
        "metadata": metadata,
        "_extraction_time_ms": duration_ms,
        "_ocr_used": _determine_ocr_used(metadata, ocr_enabled or force_ocr),
        "_peak_memory_bytes": _get_peak_memory_bytes(),
    }


def extract_batch_sync(file_paths: list[str], ocr_enabled: bool, output_format: str = "markdown") -> list[dict[str, Any]]:
    """Extract multiple files using batch API."""
    config = _build_config(ocr_enabled, output_format)

    start = time.perf_counter()
    results = batch_extract_files_sync(file_paths, config=config)  # type: ignore[arg-type]
    total_duration_ms = (time.perf_counter() - start) * 1000.0

    per_file_duration_ms = total_duration_ms / len(file_paths) if file_paths else 0

    output = []
    for result in results:
        metadata = result.metadata or {}
        output.append({
            "content": result.content,
            "metadata": metadata,
            "_extraction_time_ms": per_file_duration_ms,
            "_batch_total_ms": total_duration_ms,
            "_ocr_used": _determine_ocr_used(metadata, ocr_enabled),
        })
    return output


def _parse_request(line: str) -> tuple[str, bool]:
    """Parse a request line: JSON object with path+force_ocr, or plain file path."""
    stripped = line.strip()
    if stripped.startswith("{"):
        try:
            req = json.loads(stripped)
            return req.get("path", ""), req.get("force_ocr", False)
        except json.JSONDecodeError:
            pass
    return stripped, False


def run_server(ocr_enabled: bool, output_format: str) -> None:
    """Persistent server mode: read paths from stdin, write JSON to stdout."""
    # Signal readiness after Python + FFI initialization
    print("READY", flush=True)
    for line in sys.stdin:
        file_path, force_ocr = _parse_request(line)
        if not file_path:
            continue
        start = time.perf_counter()
        try:
            payload = extract_sync(file_path, ocr_enabled, output_format, force_ocr=force_ocr)
            print(json.dumps(payload), flush=True)
        except Exception as e:
            duration_ms = (time.perf_counter() - start) * 1000.0
            print(json.dumps({"error": str(e), "_extraction_time_ms": duration_ms, "_ocr_used": False}), flush=True)


def main() -> None:
    ocr_enabled = False
    output_format = "markdown"
    args = []
    for arg in sys.argv[1:]:
        if arg == "--ocr":
            ocr_enabled = True
        elif arg == "--no-ocr":
            ocr_enabled = False
        elif arg.startswith("--format="):
            output_format = arg.split("=", 1)[1]
        else:
            args.append(arg)

    if output_format not in ("markdown", "plaintext"):
        print(f"Error: --format must be 'markdown' or 'plaintext'; got '{output_format}'", file=sys.stderr)
        sys.exit(64)

    if len(args) < 1:
        print("Usage: kreuzberg_extract.py [--ocr|--no-ocr] [--format=markdown|plaintext] <mode> <file_path> [additional_files...]", file=sys.stderr)
        print("Modes: sync, async, batch, server", file=sys.stderr)
        sys.exit(1)

    mode = args[0]
    file_paths = args[1:]

    try:
        if mode == "server":
            run_server(ocr_enabled, output_format)

        elif mode == "sync":
            if len(file_paths) != 1:
                print("Error: sync mode requires exactly one file", file=sys.stderr)
                sys.exit(1)
            payload = extract_sync(file_paths[0], ocr_enabled, output_format)
            print(json.dumps(payload), end="")

        elif mode == "async":
            if len(file_paths) != 1:
                print("Error: async mode requires exactly one file", file=sys.stderr)
                sys.exit(1)
            payload = asyncio.run(extract_async(file_paths[0], ocr_enabled, output_format))
            print(json.dumps(payload), end="")

        elif mode == "batch":
            if len(file_paths) < 1:
                print("Error: batch mode requires at least one file", file=sys.stderr)
                sys.exit(1)

            if len(file_paths) == 1:
                results = extract_batch_sync(file_paths, ocr_enabled, output_format)
                print(json.dumps(results[0]), end="")
            else:
                results = extract_batch_sync(file_paths, ocr_enabled, output_format)
                print(json.dumps(results), end="")

        else:
            print(f"Error: Unknown mode '{mode}'. Use sync, async, batch, or server", file=sys.stderr)
            sys.exit(1)

    except Exception as e:
        print(f"Error extracting with Kreuzberg: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()

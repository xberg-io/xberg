"""Docling extraction wrapper for benchmark harness.

Supports two modes:
- sync: convert() - synchronous single-file extraction
- batch: convert_all() - batch extraction for multiple files
- server: persistent mode reading paths from stdin
"""

from __future__ import annotations

import json
import multiprocessing as _mp
import os
import platform
import resource
import sys
import time
from typing import Any

from docling.document_converter import DocumentConverter


def _get_peak_memory_bytes() -> int:
    """Get peak memory usage in bytes using resource module."""
    usage = resource.getrusage(resource.RUSAGE_SELF)
    if platform.system() == "Linux":
        return usage.ru_maxrss * 1024
    return usage.ru_maxrss


def create_converter(ocr_enabled: bool) -> DocumentConverter:
    """Create a DocumentConverter with appropriate settings."""
    if not ocr_enabled:
        try:
            from docling.datamodel.pipeline_options import PipelineOptions
            options = PipelineOptions(do_ocr=False)
            return DocumentConverter(pipeline_options=options)
        except (ImportError, TypeError):
            # Fallback if PipelineOptions API not available
            return DocumentConverter()
    return DocumentConverter()


def _render(document: Any, output_format: str) -> str:
    if output_format == "plaintext":
        return document.export_to_text()
    return document.export_to_markdown()


def extract_sync(file_path: str, converter: DocumentConverter, output_format: str = "markdown") -> dict[str, Any]:
    """Extract using synchronous single-file API."""
    start = time.perf_counter()
    result = converter.convert(file_path)
    content = _render(result.document, output_format)
    duration_ms = (time.perf_counter() - start) * 1000.0

    return {
        "content": content,
        "metadata": {"framework": "docling", "output_format": output_format},
        "_extraction_time_ms": duration_ms,
        "_peak_memory_bytes": _get_peak_memory_bytes(),
    }


def extract_batch(file_paths: list[str], converter: DocumentConverter, output_format: str = "markdown") -> list[dict[str, Any]]:
    """Extract multiple files using batch API."""
    start = time.perf_counter()
    results = converter.convert_all(file_paths, raises_on_error=False)
    total_duration_ms = (time.perf_counter() - start) * 1000.0

    per_file_duration_ms = total_duration_ms / len(file_paths) if file_paths else 0

    outputs = []
    for result in results:
        if result.status.name == "SUCCESS":
            content = _render(result.document, output_format)
            outputs.append(
                {
                    "content": content,
                    "metadata": {"framework": "docling", "output_format": output_format},
                    "_extraction_time_ms": per_file_duration_ms,
                    "_batch_total_ms": total_duration_ms,
                    "_peak_memory_bytes": _get_peak_memory_bytes(),
                }
            )
        else:
            outputs.append(
                {
                    "content": "",
                    "metadata": {
                        "framework": "docling",
                        "error": str(result.errors) if result.errors else "Unknown error",
                        "status": result.status.name,
                    },
                    "_extraction_time_ms": per_file_duration_ms,
                    "_batch_total_ms": total_duration_ms,
                    "_peak_memory_bytes": _get_peak_memory_bytes(),
                }
            )

    return outputs


def _worker(fn, args, conn):
    """Run extraction in a forked child process.

    Closes inherited stdin/stdout so the child cannot corrupt the
    parent's line-based JSON protocol.
    """
    try:
        sys.stdin.close()
        sys.stdout = open(os.devnull, "w")
    except Exception:
        pass
    try:
        result = fn(*args)
        conn.send(result)
    except Exception as e:
        conn.send({"error": str(e), "_extraction_time_ms": 0})
    finally:
        conn.close()


def _run_with_timeout(fn, args, timeout):
    """Execute fn(*args) in a forked child with a timeout.

    On timeout the child is killed but the parent stays alive —
    no expensive process restart is needed.
    """
    try:
        ctx = _mp.get_context("fork")
        parent_conn, child_conn = ctx.Pipe(duplex=False)
        p = ctx.Process(target=_worker, args=(fn, args, child_conn))
        p.start()
        child_conn.close()

        if parent_conn.poll(timeout=timeout):
            try:
                result = parent_conn.recv()
            except Exception:
                result = {"error": "worker process crashed", "_extraction_time_ms": 0}
        else:
            p.kill()
            result = {
                "error": f"extraction timed out after {timeout}s",
                "_extraction_time_ms": timeout * 1000.0,
            }

        p.join(timeout=5)
        if p.is_alive():
            p.kill()
            p.join()
        parent_conn.close()
        return result
    except Exception:
        # Fork not available — fall back to in-process extraction
        try:
            return fn(*args)
        except Exception as e:
            return {"error": str(e), "_extraction_time_ms": 0}


def _parse_path(line: str) -> str:
    """Parse a request line: JSON object with path field, or plain file path."""
    stripped = line.strip()
    if stripped.startswith("{"):
        try:
            return json.loads(stripped).get("path", "")
        except (json.JSONDecodeError, ValueError):
            pass
    return stripped


def run_server(converter: DocumentConverter, output_format: str, timeout=None) -> None:
    """Persistent server mode: read paths from stdin, write JSON to stdout."""
    print("READY", flush=True)
    for line in sys.stdin:
        file_path = _parse_path(line)
        if not file_path:
            continue
        if timeout is not None:
            result = _run_with_timeout(extract_sync, (file_path, converter, output_format), timeout)
        else:
            try:
                result = extract_sync(file_path, converter, output_format)
            except Exception as e:
                result = {"error": str(e), "_extraction_time_ms": 0}
        print(json.dumps(result), flush=True)


def main() -> None:
    ocr_enabled = False
    timeout = None
    output_format = "markdown"
    args = []
    for arg in sys.argv[1:]:
        if arg == "--ocr":
            ocr_enabled = True
        elif arg == "--no-ocr":
            ocr_enabled = False
        elif arg.startswith("--timeout="):
            timeout = int(arg.split("=", 1)[1])
        elif arg.startswith("--format="):
            output_format = arg.split("=", 1)[1]
        elif arg == "--format":
            # Next-arg style handled below by appending
            args.append(arg)
        else:
            args.append(arg)

    # Support `--format <value>` (space-separated)
    cleaned: list[str] = []
    i = 0
    while i < len(args):
        if args[i] == "--format" and i + 1 < len(args):
            output_format = args[i + 1]
            i += 2
            continue
        cleaned.append(args[i])
        i += 1
    args = cleaned

    if output_format not in ("markdown", "plaintext"):
        print(f"Error: --format must be 'markdown' or 'plaintext'; got '{output_format}'", file=sys.stderr)
        sys.exit(64)

    if len(args) < 1:
        print("Usage: docling_extract.py [--ocr|--no-ocr] [--timeout=SECS] [--format markdown|plaintext] <mode> <file_path> [additional_files...]", file=sys.stderr)
        print("Modes: sync, batch, server", file=sys.stderr)
        sys.exit(1)

    mode = args[0]
    file_paths = args[1:]

    # Create converter once (expensive initialization)
    converter = create_converter(ocr_enabled)

    try:
        if mode == "server":
            run_server(converter, output_format, timeout=timeout)

        elif mode == "sync":
            if len(file_paths) != 1:
                print("Error: sync mode requires exactly one file", file=sys.stderr)
                sys.exit(1)
            payload = extract_sync(file_paths[0], converter, output_format)
            print(json.dumps(payload), end="")

        elif mode == "batch":
            if len(file_paths) < 1:
                print("Error: batch mode requires at least one file", file=sys.stderr)
                sys.exit(1)

            if len(file_paths) == 1:
                results = extract_batch(file_paths, converter, output_format)
                print(json.dumps(results[0]), end="")
            else:
                results = extract_batch(file_paths, converter, output_format)
                print(json.dumps(results), end="")

        else:
            print(f"Error: Unknown mode '{mode}'. Use sync, batch, or server", file=sys.stderr)
            sys.exit(1)

    except Exception as e:
        print(f"Error extracting with Docling: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()

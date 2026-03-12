"""playa-pdf extraction wrapper for benchmark harness.

Supports three modes:
- sync: extract text page-by-page (sequential)
- batch: process multiple files (simulated batch using loop)
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

import playa


def _get_peak_memory_bytes() -> int:
    """Get peak memory usage in bytes using resource module."""
    usage = resource.getrusage(resource.RUSAGE_SELF)
    if platform.system() == "Linux":
        return usage.ru_maxrss * 1024
    return usage.ru_maxrss


def extract_sync(file_path: str) -> dict[str, Any]:
    """Extract using synchronous single-file API."""
    start = time.perf_counter()

    with playa.open(file_path) as doc:
        text_parts = []
        for page in doc.pages:
            page_text = page.extract_text()
            if page_text:
                text_parts.append(page_text)

    markdown = "\n\n".join(text_parts)

    duration_ms = (time.perf_counter() - start) * 1000.0

    return {
        "content": markdown,
        "metadata": {"framework": "playa-pdf"},
        "_extraction_time_ms": duration_ms,
        "_peak_memory_bytes": _get_peak_memory_bytes(),
    }


def extract_batch(file_paths: list[str]) -> list[dict[str, Any]]:
    """Extract multiple files (simulated batch - playa-pdf has no native batch API)."""
    start = time.perf_counter()

    results = []
    for file_path in file_paths:
        try:
            with playa.open(file_path) as doc:
                text_parts = []
                for page in doc.pages:
                    page_text = page.extract_text()
                    if page_text:
                        text_parts.append(page_text)

            markdown = "\n\n".join(text_parts)
            results.append({
                "content": markdown,
                "metadata": {"framework": "playa-pdf"},
            })
        except Exception as e:
            results.append({
                "content": "",
                "metadata": {
                    "framework": "playa-pdf",
                    "error": str(e),
                },
            })

    total_duration_ms = (time.perf_counter() - start) * 1000.0
    per_file_duration_ms = total_duration_ms / len(file_paths) if file_paths else 0

    peak_memory = _get_peak_memory_bytes()
    for result in results:
        result["_extraction_time_ms"] = per_file_duration_ms
        result["_batch_total_ms"] = total_duration_ms
        result["_peak_memory_bytes"] = peak_memory

    return results


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


def run_server(timeout=None) -> None:
    """Persistent server mode: read paths from stdin, write JSON to stdout."""
    print("READY", flush=True)
    for line in sys.stdin:
        file_path = _parse_path(line)
        if not file_path:
            continue
        if timeout is not None:
            result = _run_with_timeout(extract_sync, (file_path,), timeout)
        else:
            try:
                result = extract_sync(file_path)
            except Exception as e:
                result = {"error": str(e), "_extraction_time_ms": 0}
        print(json.dumps(result), flush=True)


def main() -> None:
    timeout = None
    args = []
    for arg in sys.argv[1:]:
        if arg in ("--ocr", "--no-ocr"):
            pass  # Accepted but ignored - playa-pdf doesn't have OCR capability
        elif arg.startswith("--timeout="):
            timeout = int(arg.split("=", 1)[1])
        else:
            args.append(arg)

    if len(args) < 1:
        print("Usage: playa_pdf_extract.py [--ocr|--no-ocr] [--timeout=SECS] <mode> <file_path> [additional_files...]", file=sys.stderr)
        print("Modes: sync, batch, server", file=sys.stderr)
        sys.exit(1)

    mode = args[0]
    file_paths = args[1:]

    try:
        if mode == "server":
            run_server(timeout=timeout)

        elif mode == "sync":
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
            print(f"Error: Unknown mode '{mode}'. Use sync, batch, or server", file=sys.stderr)
            sys.exit(1)

    except Exception as e:
        print(f"Error extracting with playa-pdf: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()

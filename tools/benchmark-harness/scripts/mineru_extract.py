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
import multiprocessing as _mp
import platform
import resource
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


def _get_peak_memory_bytes() -> int:
    """Get peak memory usage in bytes using resource module."""
    usage = resource.getrusage(resource.RUSAGE_SELF)
    if platform.system() == "Linux":
        return usage.ru_maxrss * 1024
    return usage.ru_maxrss


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


_MD_STRIP_RE = None


def _strip_markdown(text: str) -> str:
    """Best-effort markdown→plaintext pass. Drops syntax tokens; preserves text."""
    import re

    global _MD_STRIP_RE
    if _MD_STRIP_RE is None:
        _MD_STRIP_RE = [
            (re.compile(r"^#{1,6}\s+", re.MULTILINE), ""),         # ATX headings
            (re.compile(r"^\s*[-*+]\s+", re.MULTILINE), ""),        # bullet markers
            (re.compile(r"^\s*\d+\.\s+", re.MULTILINE), ""),        # ordered list markers
            (re.compile(r"^>\s?", re.MULTILINE), ""),                # blockquotes
            (re.compile(r"```[a-zA-Z0-9_-]*\n?"), ""),               # code fences
            (re.compile(r"`([^`]+)`"), r"\1"),                       # inline code
            (re.compile(r"\*\*([^*]+)\*\*"), r"\1"),                 # bold
            (re.compile(r"\*([^*]+)\*"), r"\1"),                     # italic
            (re.compile(r"!\[([^\]]*)\]\([^)]*\)"), r"\1"),          # images
            (re.compile(r"\[([^\]]+)\]\([^)]*\)"), r"\1"),           # links
            (re.compile(r"^\s*\|.*\|\s*$", re.MULTILINE), ""),       # table rows (drop)
        ]
    out = text
    for pattern, repl in _MD_STRIP_RE:
        out = pattern.sub(repl, out)
    return out


def extract_sync(file_path: str, ocr_enabled: bool, output_format: str = "markdown") -> dict[str, Any]:
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

    content = _strip_markdown(markdown) if output_format == "plaintext" else markdown
    duration_ms = (time.perf_counter() - start) * 1000.0

    return {
        "content": content,
        "metadata": {"framework": "mineru", "output_format": output_format},
        "_extraction_time_ms": duration_ms,
        "_peak_memory_bytes": _get_peak_memory_bytes(),
    }


def extract_batch(file_paths: list[str], ocr_enabled: bool, output_format: str = "markdown") -> list[dict[str, Any]]:
    """Extract multiple files in sequence."""
    start = time.perf_counter()

    results = []
    for file_path in file_paths:
        try:
            payload = extract_sync(file_path, ocr_enabled, output_format)
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


def run_server(ocr_enabled: bool, output_format: str, timeout=None) -> None:
    """Persistent server mode: read paths from stdin, write JSON to stdout."""
    print("READY", flush=True)
    for line in sys.stdin:
        file_path = _parse_path(line)
        if not file_path:
            continue
        if timeout is not None:
            result = _run_with_timeout(extract_sync, (file_path, ocr_enabled, output_format), timeout)
        else:
            try:
                result = extract_sync(file_path, ocr_enabled, output_format)
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
        else:
            args.append(arg)

    if output_format not in ("markdown", "plaintext"):
        print(f"Error: --format must be 'markdown' or 'plaintext'; got '{output_format}'", file=sys.stderr)
        sys.exit(64)

    if len(args) < 1:
        print("Usage: mineru_extract.py [--ocr|--no-ocr] [--timeout=SECS] [--format=markdown|plaintext] <mode> <file_path> [additional_files...]", file=sys.stderr)
        print("Modes: sync, batch, server", file=sys.stderr)
        sys.exit(1)

    mode = args[0]
    file_paths = args[1:]

    try:
        if mode == "server":
            run_server(ocr_enabled, output_format, timeout=timeout)

        elif mode == "sync":
            if len(file_paths) != 1:
                print("Error: sync mode requires exactly one file", file=sys.stderr)
                sys.exit(1)
            payload = extract_sync(file_paths[0], ocr_enabled, output_format)
            print(json.dumps(payload), end="")

        elif mode == "batch":
            if len(file_paths) < 1:
                print("Error: batch mode requires at least one file", file=sys.stderr)
                sys.exit(1)

            if len(file_paths) == 1:
                results = extract_batch(file_paths, ocr_enabled, output_format)
                print(json.dumps(results[0]), end="")
            else:
                results = extract_batch(file_paths, ocr_enabled, output_format)
                print(json.dumps(results), end="")

        else:
            print(f"Error: Unknown mode '{mode}'. Use sync, batch, or server", file=sys.stderr)
            sys.exit(1)

    except Exception as e:
        print(f"Error extracting with MinerU: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()

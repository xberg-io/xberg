"""Unstructured extraction wrapper for benchmark harness."""

from __future__ import annotations

import json
import multiprocessing as _mp
import os
import platform
import resource
import sys
import time

from unstructured.partition.auto import partition


def _get_peak_memory_bytes() -> int:
    """Get peak memory usage in bytes using resource module."""
    usage = resource.getrusage(resource.RUSAGE_SELF)
    if platform.system() == "Linux":
        return usage.ru_maxrss * 1024
    return usage.ru_maxrss


def _render_markdown(elements: list) -> str:
    """Render Unstructured Elements as GFM-ish markdown."""
    import re

    parts: list[str] = []
    for el in elements:
        cls = type(el).__name__
        text = (el.text or "").strip() if hasattr(el, "text") else str(el).strip()
        if not text and cls not in ("Image", "Figure"):
            continue
        if cls == "Title":
            parts.append(f"# {text}")
        elif cls == "Header":
            parts.append(f"## {text}")
        elif cls == "ListItem":
            parts.append(f"- {text}")
        elif cls in ("CodeSnippet", "Code"):
            parts.append(f"```\n{text}\n```")
        elif cls in ("Image", "Figure"):
            parts.append(f"![{text or cls}]()")
        elif cls == "Table":
            html = ""
            md = getattr(el, "metadata", None)
            if md is not None:
                html = getattr(md, "text_as_html", "") or ""
            if html:
                rows = re.findall(r"<tr[^>]*>(.*?)</tr>", html, flags=re.DOTALL | re.IGNORECASE)
                rendered: list[str] = []
                for i, row_html in enumerate(rows):
                    cells = re.findall(r"<t[dh][^>]*>(.*?)</t[dh]>", row_html, flags=re.DOTALL | re.IGNORECASE)
                    cells = [re.sub(r"<[^>]+>", "", c).strip() for c in cells]
                    if cells:
                        rendered.append("| " + " | ".join(cells) + " |")
                        if i == 0:
                            rendered.append("| " + " | ".join("---" for _ in cells) + " |")
                if rendered:
                    parts.append("\n".join(rendered))
                else:
                    parts.append(text)
            else:
                parts.append(text)
        else:
            parts.append(text)
    return "\n\n".join(parts)


def extract_sync(file_path: str, ocr_enabled: bool, output_format: str = "markdown") -> dict:
    """Extract using Unstructured partition API."""
    strategy = "hi_res" if ocr_enabled else "fast"
    start = time.perf_counter()
    elements = partition(filename=file_path, strategy=strategy, languages=["eng"])
    duration_ms = (time.perf_counter() - start) * 1000.0

    if output_format == "markdown":
        content = _render_markdown(elements)
    else:
        content = "\n\n".join(str(el) for el in elements)
    return {
        "content": content,
        "metadata": {"framework": "unstructured", "strategy": strategy, "output_format": output_format},
        "_extraction_time_ms": duration_ms,
        "_peak_memory_bytes": _get_peak_memory_bytes(),
    }


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
        print("Usage: unstructured_extract.py [--ocr|--no-ocr] [--timeout=SECS] [--format=markdown|plaintext] <mode> <file_path>", file=sys.stderr)
        print("Modes: sync, server", file=sys.stderr)
        sys.exit(1)

    mode = args[0]

    if mode == "server":
        run_server(ocr_enabled, output_format, timeout=timeout)
    elif mode == "sync":
        if len(args) < 2:
            print("Error: sync mode requires a file path", file=sys.stderr)
            sys.exit(1)
        try:
            payload = extract_sync(args[1], ocr_enabled, output_format)
            print(json.dumps(payload), end="")
        except Exception as e:
            print(f"Error extracting with Unstructured: {e}", file=sys.stderr)
            sys.exit(1)
    else:
        # Legacy mode: first arg is the file path directly
        try:
            payload = extract_sync(args[0], ocr_enabled, output_format)
            print(json.dumps(payload), end="")
        except Exception as e:
            print(f"Error extracting with Unstructured: {e}", file=sys.stderr)
            sys.exit(1)


if __name__ == "__main__":
    main()

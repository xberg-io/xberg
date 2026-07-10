#!/usr/bin/env python3
"""Extract top self-time symbols from a pprof flamegraph SVG.

Usage:
    python3 tools/perf/extract_top_symbols.py flamegraphs/<sha>/baseline.svg [N=15]

Reports the top-N functions by aggregate sample count, filtered to drop
system / dependency frames so xberg::* hotspots surface clearly.

The SVG is the standard pprof flamegraph output; each rectangle has a
`<title>fn (samples, percent%)</title>` element. We aggregate per-function
sample counts across all stack instances — that's an over-counting heuristic
(a function called from many places shows up larger than its true self-time),
but it's the right signal for "is this function on the hot path?"
"""

from __future__ import annotations

import re
import sys
from collections import Counter
from pathlib import Path

EXCLUDE_PREFIXES = (
    "__",
    "_",
    "onnx::",
    "ort_sys::",
    "tokio::",
    "std::",
    "core::",
    "alloc::",
    "_$LT$std",
    "image::",
    "pdf_oxide::",
    "memchr::",
    "regex::",
    "serde::",
    "tracing::",
    "lopdf::",
    "pprof::",
)

ADDRESS_PATTERN = re.compile(r"^\d+$|^0x[0-9a-f]+$")


def is_xberg_or_application(symbol: str) -> bool:
    """Return True if the symbol is application-level (likely xberg crate)."""
    if not symbol:
        return False
    if ADDRESS_PATTERN.match(symbol):
        return False
    return not symbol.startswith(EXCLUDE_PREFIXES)


def short(symbol: str, max_len: int = 110) -> str:
    """Trim a long symbol name for display, abbreviating generic args."""
    s = re.sub(r"<[^>]*>", "<>", symbol)
    if len(s) > max_len:
        s = "..." + s[-(max_len - 3) :]
    return s


def main() -> int:
    if len(sys.argv) < 2:
        print(f"usage: {sys.argv[0]} <flamegraph.svg> [top_n=15]", file=sys.stderr)
        return 2

    svg_path = Path(sys.argv[1])
    top_n = int(sys.argv[2]) if len(sys.argv) > 2 else 15

    if not svg_path.is_file():
        print(f"error: {svg_path} is not a file", file=sys.stderr)
        return 2

    content = svg_path.read_text()
    pattern = re.compile(r"<title>([^<]+) \(([0-9,]+) samples, ([\d.]+)%\)</title>")
    matches = pattern.findall(content)

    if not matches:
        print(f"warning: no <title> elements found in {svg_path}", file=sys.stderr)
        return 1

    samples_per_fn: Counter[str] = Counter()
    for fn, samples, _pct in matches:
        samples_per_fn[fn] += int(samples.replace(",", ""))

    total_samples = sum(samples_per_fn.values()) or 1

    app_ranked = sorted(
        ((fn, s) for fn, s in samples_per_fn.items() if is_xberg_or_application(fn)),
        key=lambda x: -x[1],
    )

    if not app_ranked:
        print(
            "warning: no application-level symbols matched filters — "
            "is the binary built with --profile profiling? a release-stripped "
            "build only contains system frames.",
            file=sys.stderr,
        )
        app_ranked = sorted(samples_per_fn.items(), key=lambda x: -x[1])

    print(f"# Top {top_n} symbols by aggregate sample count from {svg_path.name}")
    print(f"# (out of {len(samples_per_fn)} unique frames; total samples {total_samples})")
    print()
    print(f"{'rank':>4}  {'samples':>8}  {'%total':>7}  symbol")
    print("-" * 80)
    for rank, (fn, samples) in enumerate(app_ranked[:top_n], start=1):
        pct = 100.0 * samples / total_samples
        print(f"{rank:>4}  {samples:>8}  {pct:>6.2f}%  {short(fn)}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
r"""Count the executable's resident code pages at process exit, per Mach-O section (macOS).

``run`` launches the binary under lldb, stops it in ``exit()``, and has this same file, imported
into lldb, give every page of the executable's ``__TEXT`` and ``__DATA_CONST`` segments an
alternating protection so ``vmmap -noCoalesce`` prints one region per page with its resident
size. ``report`` maps those pages onto the sections ``otool -l`` lists. The resident
``__TEXT,__text`` page count is what the linker order file (ADR-0020) exists to shrink.

    python3 scripts/perf/text_residency.py run --out /tmp/rss/tiny -- target/release/xberg \\
        extract tiny.pdf --no-config-discovery --format json --content-format plain

Runs under lldb's own Python too, so the module level stays Python 3.9 compatible.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from collections import Counter
from pathlib import Path

SEGMENTS = {"__TEXT": 5, "__DATA_CONST": 3}
REGION = re.compile(r"^(__\w+)\s+([0-9a-f]+)-([0-9a-f]+)\s+\[\s*(\S+)\s+(\S+)")


def _dump(debugger: object, command: str, result: object, _internal_dict: dict) -> None:
    """Lldb command ``textresidency <prefix>``: split the segments into pages and run vmmap."""
    import lldb  # noqa: PLC0415  # only importable inside lldb

    prefix = command.strip()
    target = debugger.GetSelectedTarget()
    process = target.GetProcess()
    if process.GetState() != lldb.eStateStopped:
        result.SetError("the process is not stopped in exit(); did it crash or call _exit?")
        return
    frame = process.GetSelectedThread().GetFrameAtIndex(0)
    page = int(frame.EvaluateExpression("(int)getpagesize()").GetValueAsSigned())
    module = target.GetModuleAtIndex(0)
    segments = []
    for segment in module.section_iter():
        name = segment.GetName()
        if name not in SEGMENTS:
            continue
        start, size = segment.GetLoadAddress(target), segment.GetByteSize()
        expression = (
            "unsigned long a; int bad = 0; int i = 0; "
            f"for (a = {start}UL; a < {start + size}UL; a += {page}UL, i++) "
            f"{{ if ((int)mprotect((void *)a, {page}, (i & 1) ? {SEGMENTS[name]} : 1)) bad++; }} bad"
        )
        failures = frame.EvaluateExpression(expression).GetValueAsSigned()
        segments.append(
            {
                "name": name,
                "load": start,
                "file": segment.GetFileAddress(),
                "size": size,
                "mprotect_failures": failures,
            }
        )
    pid = process.GetProcessID()
    with open(prefix + ".segments.json", "w") as handle:
        json.dump({"binary": module.GetFileSpec().fullpath, "page": page, "segments": segments}, handle, indent=1)
    os.system(f"vmmap -wide -noCoalesce -interleaved {pid} > '{prefix}.pages.txt' 2>&1")  # noqa: S605
    os.system(f"vmmap -summary {pid} > '{prefix}.summary.txt' 2>&1")  # noqa: S605


def __lldb_init_module(debugger: object, _internal_dict: dict) -> None:
    debugger.HandleCommand(f"command script add -f {__name__}._dump textresidency")


def parse_size(text: str) -> int:
    units = {"K": 1 << 10, "M": 1 << 20, "G": 1 << 30}
    if text[-1] in units:
        return int(float(text[:-1]) * units[text[-1]])
    return int(text)


def macho_sections(binary: str) -> list[tuple[str, int, int]]:
    """Return ``(segment,section, file address, size)`` rows from ``otool -l``."""
    output = subprocess.run(["otool", "-l", binary], check=True, capture_output=True, text=True).stdout
    sections = []
    current: dict[str, str] = {}
    for line in output.splitlines():
        match = re.match(r"\s*(sectname|segname|addr|size) (\S+)", line)
        if not match:
            continue
        current[match.group(1)] = match.group(2)
        if match.group(1) == "size" and "sectname" in current:
            sections.append(
                (f"{current['segname']},{current['sectname']}", int(current["addr"], 16), int(current["size"], 16))
            )
            current = {}
    return sections


def report(prefix: str) -> dict[str, dict[str, int]]:
    meta = json.loads(Path(prefix + ".segments.json").read_text())
    failures = sum(segment["mprotect_failures"] for segment in meta["segments"])
    if failures:
        raise RuntimeError(f"{failures} pages kept their protection; vmmap coalesced them, the count is unreliable")
    page = meta["page"]
    sections = macho_sections(meta["binary"])
    resident: Counter[str] = Counter()
    total: Counter[str] = Counter()
    rows = 0
    for line in Path(prefix + ".pages.txt").read_text(errors="replace").splitlines():
        match = REGION.match(line)
        if not match:
            continue
        start, end = int(match.group(2), 16), int(match.group(3), 16)
        segment = next((s for s in meta["segments"] if s["load"] <= start < s["load"] + s["size"]), None)
        if segment is None:
            continue
        rows += 1
        file_start = start - segment["load"] + segment["file"]
        file_end = end - segment["load"] + segment["file"]
        overlaps = [
            (min(file_end, address + size) - max(file_start, address), name)
            for name, address, size in sections
            if address < file_end and file_start < address + size
        ]
        name = max(overlaps)[1] if overlaps else f"{segment['name']},(header)"
        pages = max(1, (end - start) // page)
        total[name] += pages
        resident[name] += min(pages, parse_size(match.group(5)) // page)
    if rows == 0:
        raise RuntimeError(f"{prefix}.pages.txt has no region of {meta['binary']}; vmmap failed?")
    summary = {name: {"resident": resident[name], "total": total[name]} for name in total}
    for name in sorted(summary, key=lambda key: -resident[key]):
        print(
            f"{name:32} resident {resident[name]:6d} pages ({resident[name] * page / 1048576:7.2f} MiB) "
            f"of {total[name]:6d}"
        )
    text_pages = summary.get("__TEXT,__text", {}).get("resident", 0)
    text_segment = sum(value["resident"] for key, value in summary.items() if key.startswith("__TEXT,"))
    print(f"TEXT_TEXT_RESIDENT_PAGES={text_pages}")
    print(f"TEXT_SEGMENT_RESIDENT_PAGES={text_segment}")
    print(f"PAGE_BYTES={page}")
    return summary


def run(prefix: str, command: list[str]) -> None:
    for argument in command[1:]:
        if "'" in argument:
            raise ValueError(f"lldb launch arguments cannot contain a single quote: {argument}")
    Path(prefix).parent.mkdir(parents=True, exist_ok=True)
    arguments = " ".join(f"'{argument}'" for argument in command[1:])
    lldb = [
        "lldb",
        "--batch",
        "-o",
        f"command script import {Path(__file__).resolve()}",
        "-o",
        "breakpoint set -s libsystem_c.dylib -n exit",
        "-o",
        f"process launch -o /dev/null -- {arguments}",
        "-o",
        f"textresidency {prefix}",
        "-o",
        "kill",
        "--",
        command[0],
    ]
    with open(prefix + ".lldb.log", "w") as log:
        subprocess.run(lldb, check=False, stdout=log, stderr=subprocess.STDOUT)


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    commands = parser.add_subparsers(dest="command", required=True)
    run_command = commands.add_parser("run", help="launch under lldb, dump at exit, then report")
    run_command.add_argument("--out", required=True, help="output path prefix")
    run_command.add_argument("--max-text-pages", type=int, help="fail when __TEXT,__text exceeds this")
    run_command.add_argument("binary_and_args", nargs=argparse.REMAINDER)
    report_command = commands.add_parser("report", help="report an earlier run's dump")
    report_command.add_argument("--out", required=True)
    report_command.add_argument("--max-text-pages", type=int)
    args = parser.parse_args(argv)
    prefix = str(Path(args.out).resolve())
    if args.command == "run":
        command = [argument for argument in args.binary_and_args if argument != "--"]
        if not command:
            parser.error("run needs -- <binary> [arguments...]")
        run(prefix, [str(Path(command[0]).resolve()), *command[1:]])
    summary = report(prefix)
    text_pages = summary.get("__TEXT,__text", {}).get("resident", 0)
    if args.max_text_pages is not None and text_pages > args.max_text_pages:
        print(f"__TEXT,__text resident pages {text_pages} exceed {args.max_text_pages}")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

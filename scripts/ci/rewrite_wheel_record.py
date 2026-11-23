#!/usr/bin/env python3
"""Regenerate a wheel's RECORD from an unpacked wheel tree.

vendor-native-closure.sh adds shared libraries to an unpacked wheel and patches the rpath of
the native modules, which leaves the wheel's RECORD naming stale hashes and missing the added
files. The tree is the source of truth, so RECORD is rebuilt from it.
"""

from __future__ import annotations

import base64
import csv
import hashlib
import sys
from pathlib import Path


def rewrite_record(root: Path) -> Path:
    records = sorted(root.glob("*.dist-info/RECORD"))
    if len(records) != 1:
        raise ValueError(f"expected one .dist-info/RECORD under {root}, found {len(records)}")
    record = records[0]
    rows: list[list[str]] = []
    for path in sorted(p for p in root.rglob("*") if p.is_file() and p != record):
        content = path.read_bytes()
        digest = base64.urlsafe_b64encode(hashlib.sha256(content).digest()).decode("ascii").rstrip("=")
        rows.append([path.relative_to(root).as_posix(), f"sha256={digest}", str(len(content))])
    rows.append([record.relative_to(root).as_posix(), "", ""])
    with record.open("w", newline="", encoding="utf-8") as handle:
        csv.writer(handle, lineterminator="\n").writerows(rows)
    return record


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit("usage: rewrite_wheel_record.py <unpacked-wheel-dir>")
    print(f"Rewrote {rewrite_record(Path(sys.argv[1]))}")

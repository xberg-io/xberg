#!/usr/bin/env python3
"""Verify that the default iOS bindings resolve tract without native ORT."""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path

PACKAGES = ("xberg-dart", "xberg-swift")
TARGETS = ("aarch64-apple-ios", "aarch64-apple-ios-sim")
FORBIDDEN = frozenset({"ort", "ort-sys"})
REQUIRED = frozenset({"xberg", "tract-onnx"})
PACKAGE_LINE = re.compile(r"^([A-Za-z0-9_-]+) v\d[^ ]*(?: .*)?$")


def validate_graph(output: str, package: str) -> set[str]:
    """Validate resolved packages, including positive controls against empty output."""
    names = {match[1] for line in output.splitlines() if (match := PACKAGE_LINE.fullmatch(line))}
    if not names:
        raise ValueError(f"{package}: cargo tree returned no resolved packages")
    if package not in names:
        raise ValueError(f"{package}: requested binding is absent from the resolved graph")
    forbidden = names & FORBIDDEN
    if forbidden:
        raise ValueError(f"{package}: native ORT leaked into mobile defaults: {', '.join(sorted(forbidden))}")
    missing = REQUIRED - names
    if missing:
        raise ValueError(f"{package}: required mobile packages are missing: {', '.join(sorted(missing))}")
    return names


def check_package(root: Path, target: str, package: str) -> int:
    result = subprocess.run(
        [
            "cargo",
            "tree",
            "--locked",
            "--target",
            target,
            "-p",
            package,
            "--prefix",
            "none",
            "--edges",
            "normal",
            "--format",
            "{p}",
        ],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    )
    return len(validate_graph(result.stdout, package))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", choices=TARGETS, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    checked = 0
    try:
        for package in PACKAGES:
            count = check_package(root, args.target, package)
            print(f"{package}: {count} distinct package names; tract present, native ORT absent")
            checked += 1
        if checked != len(PACKAGES):
            raise ValueError(f"expected {len(PACKAGES)} binding graphs, checked {checked}")
    except (ValueError, subprocess.CalledProcessError) as error:
        print(error.stderr if isinstance(error, subprocess.CalledProcessError) else str(error), file=sys.stderr)
        return 1
    print(f"Verified {checked} mobile binding graphs for {args.target}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

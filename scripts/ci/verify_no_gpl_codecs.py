#!/usr/bin/env python3
"""Fail when a release archive or staging tree carries the GPL-2.0 x265 encoder.

`cargo deny` audits the Rust crate graph, which stays clean whether or not a C
codec is linked: x265 reaches the shipped bytes through libheif (directly, or as
a libheif plugin), so only the artifact itself can prove it is absent. This gate
opens each archive or directory and checks the shipped files:

* a file whose name contains ``x265`` (``libx265.so.215``, ``libheif-x265.so``,
  ``libx265.215.dylib``) is reported;
* every ELF, Mach-O and PE file is scanned for the ``libx265`` load-command or
  ``DT_NEEDED`` string and for ``x265_*`` symbols.

Text files are deliberately not content-scanned: the licence inventory shipped
beside the binary names libx265 to document the Elixir encoder artifact, and a
substring of a notice is not a linked codec.

The gate fails closed: an artifact that yields no files is an error, so an empty
glob or a mistyped path cannot pass as "clean".
"""

from __future__ import annotations

import argparse
import re
import sys
import tarfile
import zipfile
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Iterator

# libheif links this codec when WITH_X265 is on (its CMake default), and a libheif
# plugin package or a stale dependency cache can add it even when the main
# library is decode-only. ~keep
X265_NAME = re.compile(r"x265", re.IGNORECASE)
X265_BYTES = re.compile(rb"libx265|x265_[a-z0-9_]+")

# Enough leading bytes to tell a scanned binary from a notice, config or source
# file: ELF, PE, thin/fat Mach-O in both endiannesses, and a static `ar` archive. ~keep
BINARY_MAGIC = (
    b"\x7fELF",
    b"MZ",
    b"!<arch>\n",
    b"\xfe\xed\xfa\xce",
    b"\xce\xfa\xed\xfe",
    b"\xfe\xed\xfa\xcf",
    b"\xcf\xfa\xed\xfe",
    b"\xca\xfe\xba\xbe",
    b"\xbe\xba\xfe\xca",
    b"\xca\xfe\xba\xbf",
    b"\xbf\xba\xfe\xca",
)

CHUNK_SIZE = 1 << 20
PREFIX_SIZE = max(len(magic) for magic in BINARY_MAGIC)


def is_binary(payload: bytes) -> bool:
    """Return whether the leading bytes identify a supported executable format."""
    return any(payload.startswith(magic) for magic in BINARY_MAGIC)


def scan_stream(label: str, stream: Iterator[bytes]) -> list[str]:
    """Return the x265 findings for one file, reading it from ``stream``."""
    findings = []
    carry = b""
    prefix = b""
    for chunk in stream:
        if not chunk:
            continue
        if len(prefix) < PREFIX_SIZE:
            prefix += chunk[: PREFIX_SIZE - len(prefix)]
        if not is_binary(prefix):
            return findings
        window = carry + chunk
        match = X265_BYTES.search(window)
        if match:
            findings.append(f"{label} references {match.group(0).decode('ascii', 'replace')}")
            return findings
        carry = window[-(len(b"libx265") - 1) :]
    return findings


def name_findings(label: str) -> list[str]:
    """Return a finding when the file name itself is an x265 artifact."""
    if X265_NAME.search(label):
        return [f"{label} is an x265-named file"]
    return []


def scan_tar(path: Path) -> tuple[list[str], int]:
    findings = []
    scanned = 0
    with tarfile.open(path, "r:*") as archive:
        for member in archive:
            for finding in name_findings(member.name):
                findings.append(f"{path.name}: {finding}")
            if member.issym() or member.islnk():
                if name_findings(member.linkname):
                    findings.append(f"{path.name}: {member.name} links to {member.linkname}")
                continue
            if not member.isfile():
                continue
            stream = archive.extractfile(member)
            if stream is None:
                findings.append(f"{path.name}: {member.name} could not be read")
                continue
            scanned += 1
            findings.extend(f"{path.name}: {finding}" for finding in scan_stream(member.name, iter(stream.read, b"")))
    return findings, scanned


def scan_zip(path: Path) -> tuple[list[str], int]:
    findings = []
    scanned = 0
    with zipfile.ZipFile(path) as archive:
        for entry in archive.infolist():
            for finding in name_findings(entry.filename):
                findings.append(f"{path.name}: {finding}")
            if entry.is_dir():
                continue
            scanned += 1
            with archive.open(entry) as stream:
                findings.extend(
                    f"{path.name}: {finding}" for finding in scan_stream(entry.filename, iter(stream.read, b""))
                )
    return findings, scanned


def scan_directory(path: Path) -> tuple[list[str], int]:
    findings = []
    scanned = 0
    for entry in sorted(path.rglob("*")):
        label = entry.relative_to(path).as_posix()
        for finding in name_findings(label):
            findings.append(f"{path.name}: {finding}")
        if entry.is_symlink() or not entry.is_file():
            continue
        scanned += 1
        with entry.open("rb") as stream:
            findings.extend(f"{path.name}: {finding}" for finding in scan_stream(label, iter(stream.read, b"")))
    return findings, scanned


def scan_artifact(path: Path) -> tuple[list[str], int]:
    """Return the findings and file count for one archive or directory."""
    if path.is_dir():
        return scan_directory(path)
    if zipfile.is_zipfile(path):
        return scan_zip(path)
    try:
        return scan_tar(path)
    except tarfile.TarError as error:
        return [f"{path.name}: not a readable tar archive ({error})"], 0


def verify(paths: list[Path], min_artifacts: int) -> list[str]:
    """Return every reason the artifacts fail the no-x265 contract."""
    problems = []
    scanned_total = 0
    examined = 0
    for path in paths:
        if not path.exists():
            problems.append(f"artifact not found: {path}")
            continue
        findings, scanned = scan_artifact(path)
        problems.extend(findings)
        if scanned == 0 and not findings:
            problems.append(f"{path} yielded no files to scan")
        examined += 1
        scanned_total += scanned
    if examined < min_artifacts:
        problems.append(f"examined {examined} artifacts, expected at least {min_artifacts}")
    if problems:
        return problems
    print(f"verified {examined} artifacts and {scanned_total} files: no x265 codec")
    return []


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("paths", nargs="+", type=Path, help="archives (.tar.*, .zip) or staged directories to audit")
    parser.add_argument(
        "--min-artifacts",
        type=int,
        default=1,
        dest="min_artifacts",
        help="fail unless at least this many artifacts were examined",
    )
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    problems = verify(args.paths, args.min_artifacts)
    if problems:
        for problem in problems:
            print(f"error: {problem}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

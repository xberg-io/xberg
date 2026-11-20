#!/usr/bin/env python3
"""Fail when a CLI archive still links or bundles a GPL/LGPL codec.

`cargo deny` audits the Rust crate graph, which stays clean whether or not the
vcpkg libheif stack is linked: libheif, libde265 and x265 are C libraries reached
through `libheif-sys` and linked into the executable, so the crate licences say
nothing about them. This gate looks at the shipped bytes instead: every
executable and library in the archive is scanned for the codecs' symbols, and
the binary's own format inventory must still list PDF and the Office formats the
no-GPL archive is meant to keep.
"""

from __future__ import annotations

import argparse
import json
import sys
import zipfile
from pathlib import Path

# Symbols and diagnostic strings emitted only by the GPL/LGPL codec stack that
# `heic` links: libheif itself, the libde265 decoder and the x265 encoder.
CODEC_MARKERS = (
    b"libheif",
    b"libde265",
    b"heif_image_create",
    b"heif_context_alloc",
    b"de265_",
    b"libx265",
    b"x265_encoder",
)

BINARY_SUFFIXES = (".exe", ".dll")


def binary_entries(archive: zipfile.ZipFile) -> list[zipfile.ZipInfo]:
    """Return the executables and libraries in the archive, in name order."""
    return sorted(
        (entry for entry in archive.infolist() if entry.filename.lower().endswith(BINARY_SUFFIXES)),
        key=lambda entry: entry.filename,
    )


def find_codec_markers(payload: bytes) -> list[str]:
    """Return the codec markers present in one binary's bytes."""
    return [marker.decode("ascii", "replace") for marker in CODEC_MARKERS if marker in payload]


def archive_findings(archive: zipfile.ZipFile) -> list[str]:
    """Return one finding per binary that carries a codec marker."""
    findings = []
    for entry in binary_entries(archive):
        markers = find_codec_markers(archive.read(entry))
        if markers:
            findings.append(f"{entry.filename} contains {', '.join(markers)}")
    return findings


def formats_by_extension(formats_json: Path) -> set[str]:
    """Read `xberg formats --format json` and return the reported extensions."""
    document = json.loads(formats_json.read_text(encoding="utf-8"))
    if isinstance(document, dict):
        document = document.get("formats", [])
    extensions = set()
    for format_entry in document:
        if isinstance(format_entry, str):
            extensions.add(format_entry)
        elif isinstance(format_entry, dict) and "extension" in format_entry:
            extensions.add(str(format_entry["extension"]))
        else:
            raise ValueError(f"unrecognized format entry: {format_entry!r}")
    return extensions


def verify(archive_path: Path, formats_json: Path, required_formats: list[str]) -> list[str]:
    """Return every reason the archive fails the no-GPL contract."""
    problems = []
    if not archive_path.is_file():
        return [f"archive not found: {archive_path}"]

    with zipfile.ZipFile(archive_path) as archive:
        names = archive.namelist()
        if not any(name.lower().endswith(".exe") for name in names):
            problems.append("the archive holds no executable")
        problems.extend(archive_findings(archive))

    reported = formats_by_extension(formats_json)
    missing = [extension for extension in required_formats if extension not in reported]
    if missing:
        problems.append(f"the binary does not report {', '.join(missing)}")
    return problems


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", required=True, type=Path, help="CLI .zip to audit")
    parser.add_argument(
        "--formats-json",
        required=True,
        type=Path,
        help="output of `xberg formats --format json` from the archived binary",
    )
    parser.add_argument(
        "--require-format",
        action="append",
        default=[],
        dest="required_formats",
        help="extension the binary must still report (repeatable)",
    )
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    problems = verify(args.archive, args.formats_json, args.required_formats)
    if problems:
        for problem in problems:
            print(f"error: {problem}", file=sys.stderr)
        return 1
    print(f"{args.archive.name} is free of GPL/LGPL codecs and keeps {', '.join(args.required_formats)}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

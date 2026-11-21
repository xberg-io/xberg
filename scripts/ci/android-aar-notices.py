#!/usr/bin/env python3
"""Inject and verify the canonical notices inside a built Android AAR."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import sys
import zipfile
from pathlib import Path, PurePosixPath
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from types import ModuleType

# Android tooling reads the AAR manifest from the archive root, so it must never move. ~keep
MANIFEST = "AndroidManifest.xml"
NOTICE_DIR = "META-INF"
ZIP_EPOCH = (1980, 1, 1, 0, 0, 0)
SIGNATURE_SUFFIXES = (".SF", ".RSA", ".DSA", ".EC")


def notice_module() -> ModuleType:
    """Load the shared canonical-notice reader beside this script.

    The canonical bytes, the generated-bundle marker and the missing/empty failure messages must
    stay identical to every other release channel, so they come from `package-notices.py` rather
    than a second implementation. ~keep
    """
    path = Path(__file__).with_name("package-notices.py")
    spec = importlib.util.spec_from_file_location("xberg_package_notices", path)
    if spec is None or spec.loader is None:
        raise ValueError(f"cannot load the notice helper at {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def archives(names: list[str], expected: int) -> list[Path]:
    """Resolve the archive set, refusing a count that would verify nothing."""
    if expected < 1:
        raise ValueError("expected at least one AAR archive")
    if len(names) != expected:
        raise ValueError(f"expected {expected} AAR archives, found {len(names)}")
    return [Path(name) for name in names]


def readable(path: Path) -> zipfile.ZipFile:
    """Open a non-empty ZIP, naming the archive in every failure."""
    if not path.is_file():
        raise ValueError(f"AAR not found: {path}")
    if path.stat().st_size == 0:
        raise ValueError(f"empty AAR: {path}")
    if not zipfile.is_zipfile(path):
        raise ValueError(f"not a ZIP archive: {path}")
    return zipfile.ZipFile(path)


def root_manifests(names: list[str]) -> int:
    return sum(1 for name in names if PurePosixPath(name) == PurePosixPath(MANIFEST))


def signed_entries(names: list[str]) -> list[str]:
    return [name for name in names if name.startswith(f"{NOTICE_DIR}/") and name.upper().endswith(SIGNATURE_SUFFIXES)]


def non_notice_entries(path: Path, targets: set[str]) -> list[tuple[str, str]]:
    """Fingerprint every entry the injection must leave byte-identical."""
    with zipfile.ZipFile(path) as archive:
        return [
            (entry.filename, hashlib.sha256(archive.read(entry)).hexdigest())
            for entry in archive.infolist()
            if entry.filename not in targets
        ]


def rewrite(path: Path, notices: dict[str, bytes]) -> None:
    """Replace the notice entries, keeping every other entry and its order intact."""
    targets = {f"{NOTICE_DIR}/{name}" for name in notices}
    preserved = []
    temporary = path.with_name(f".{path.name}.notices")
    try:
        with zipfile.ZipFile(path) as source, zipfile.ZipFile(temporary, "w") as destination:
            for entry in source.infolist():
                if entry.filename in targets:
                    continue
                content = source.read(entry)
                preserved.append((entry.filename, hashlib.sha256(content).hexdigest()))
                written = zipfile.ZipInfo(entry.filename, date_time=entry.date_time)
                written.compress_type = entry.compress_type
                written.external_attr = entry.external_attr
                written.internal_attr = entry.internal_attr
                written.create_system = entry.create_system
                written.extra = entry.extra
                written.comment = entry.comment
                destination.writestr(written, content)
            for name, content in notices.items():
                written = zipfile.ZipInfo(f"{NOTICE_DIR}/{name}", date_time=ZIP_EPOCH)
                written.compress_type = zipfile.ZIP_DEFLATED
                written.external_attr = 0o100644 << 16
                destination.writestr(written, content)
        if non_notice_entries(temporary, targets) != preserved:
            raise ValueError(f"{path}: rewriting the notices changed entries other than {sorted(targets)}")
        temporary.replace(path)
    finally:
        temporary.unlink(missing_ok=True)


def inject(path: Path, notices: dict[str, bytes]) -> None:
    """Add both canonical notices to one AAR and prove the archive survived."""
    with readable(path) as archive:
        names = archive.namelist()
        found = root_manifests(names)
        if found != 1:
            raise ValueError(f"{path}: expected one {MANIFEST} at the archive root, found {found}")
        signed = signed_entries(names)
        if signed:
            raise ValueError(f"{path}: archive is signed; adding notices would invalidate {signed[0]}")
    rewrite(path, notices)
    verify(path, notices)
    print(f"Injected {len(notices)} canonical notices into {path}")


def verify(path: Path, notices: dict[str, bytes]) -> None:
    """Compare both packaged notices with the canonical bytes, byte for byte."""
    with readable(path) as archive:
        names = archive.namelist()
        found = root_manifests(names)
        if found != 1:
            raise ValueError(f"{path}: expected one {MANIFEST} at the archive root, found {found}")
        for name, expected in notices.items():
            member = f"{NOTICE_DIR}/{name}"
            matches = [entry for entry in archive.infolist() if entry.filename == member]
            if len(matches) != 1:
                raise ValueError(f"{path}: expected one {member}, found {len(matches)}")
            content = archive.read(matches[0])
            if content != expected:
                raise ValueError(
                    f"{path}: {member} differs from canonical {name} ({len(content)} bytes, expected {len(expected)})"
                )
    print(f"Verified {len(notices)} canonical notices in {path}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    for name in ("inject", "verify"):
        command = commands.add_parser(name)
        command.add_argument(
            "--canonical-root",
            type=Path,
            default=None,
            help="directory holding the canonical LICENSE and THIRD_PARTY_LICENSES.md (default: this checkout)",
        )
        command.add_argument(
            "--expected-count",
            type=int,
            default=1,
            help="number of AARs the caller expects to process; a mismatch fails instead of passing on zero",
        )
        command.add_argument("archives", nargs="+")
    args = parser.parse_args()
    try:
        notices = notice_module().canonical_notices(args.canonical_root)
        for path in archives(args.archives, args.expected_count):
            if args.command == "inject":
                inject(path, notices)
            else:
                verify(path, notices)
    except (OSError, ValueError, EOFError, zipfile.BadZipFile) as error:
        print(f"::error::{error}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()

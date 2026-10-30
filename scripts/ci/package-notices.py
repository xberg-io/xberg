#!/usr/bin/env python3
"""Stage canonical distribution notices and verify final archive bytes."""

import argparse
import json
import shutil
import sys
import tarfile
import zipfile
from pathlib import Path, PurePosixPath

NOTICE_NAMES = ("LICENSE", "THIRD_PARTY_LICENSES.md")


def canonical_notices() -> dict[str, bytes]:
    root = Path(__file__).resolve().parents[2]
    notices = {}
    for name in NOTICE_NAMES:
        content = (root / name).read_bytes()
        if not content:
            raise ValueError(f"empty canonical notice: {name}")
        notices[name] = content
    return notices


def stage(destinations: list[str], notices: dict[str, bytes]) -> None:
    root = Path(__file__).resolve().parents[2]
    for name in destinations:
        destination = Path(name)
        destination.mkdir(parents=True, exist_ok=True)
        for notice in notices:
            shutil.copyfile(root / notice, destination / notice)
        manifest = destination / "package.json"
        if manifest.is_file():
            package = json.loads(manifest.read_text())
            package["files"] = sorted(set(package.get("files") or []) | set(notices))
            manifest.write_text(json.dumps(package, indent=2) + "\n")
    print(f"Staged {len(notices) * len(destinations)} canonical notice files")


def verify(archive_path: str, prefixes: list[str], notices: dict[str, bytes]) -> None:
    with zipfile.ZipFile(archive_path) if zipfile.is_zipfile(archive_path) else tarfile.open(archive_path) as archive:
        if isinstance(archive, zipfile.ZipFile):
            members = [(member.filename, member) for member in archive.infolist() if not member.is_dir()]
        else:
            members = [(member.name, member) for member in archive.getmembers() if member.isfile()]
        for prefix in prefixes:
            for notice, expected in notices.items():
                path = PurePosixPath(prefix) / notice
                matches = [member for name, member in members if PurePosixPath(name) == path]
                if len(matches) != 1:
                    raise ValueError(f"expected one {path}, found {len(matches)}")
                if isinstance(archive, zipfile.ZipFile):
                    content = archive.read(matches[0])
                else:
                    stream = archive.extractfile(matches[0])
                    if stream is None:
                        raise ValueError(f"unreadable archive notice: {path}")
                    with stream:
                        content = stream.read()
                if content != expected:
                    raise ValueError(f"{path} differs from canonical {notice}")
    print(f"Verified {len(notices) * len(prefixes)} canonical notice files")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("stage").add_argument("destinations", nargs="+")
    verification = commands.add_parser("verify")
    verification.add_argument("archive")
    verification.add_argument("prefixes", nargs="+")
    args = parser.parse_args()
    try:
        notices = canonical_notices()
        if args.command == "stage":
            stage(args.destinations, notices)
        else:
            verify(args.archive, args.prefixes, notices)
    except (OSError, ValueError, tarfile.TarError, zipfile.BadZipFile) as error:
        print(f"::error::{error}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()

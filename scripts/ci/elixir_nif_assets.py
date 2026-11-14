"""Normalize the shared Windows builder output and write release checksum keys."""

import argparse
import hashlib
import json
import tarfile
import tempfile
from pathlib import Path


def normalize_windows(archive: Path, member: str) -> None:
    name = f"{member}.tar.gz"
    if archive.name not in (name, f"lib{name}"):
        raise ValueError(f"unexpected Windows NIF archive name: {archive.name}")

    destination = archive.with_name(name)
    with tempfile.TemporaryDirectory(dir=archive.parent) as scratch:
        staged = Path(scratch) / name
        with tarfile.open(archive) as source, tarfile.open(staged, "w:gz") as target:
            entries = source.getmembers()
            if len(entries) != 1 or not entries[0].isfile() or entries[0].name != archive.name.removesuffix(".tar.gz"):
                raise ValueError(f"expected exactly one Windows NIF in {archive}")
            entry = entries[0]
            with source.extractfile(entry) as data:
                entry.name = member
                target.addfile(entry, data)
        staged.replace(destination)
    if archive != destination:
        archive.unlink()


def write_checksums(output: Path, archives: list[Path]) -> None:
    entries = {}
    for archive in archives:
        if archive.name in entries:
            raise ValueError(f"duplicate checksum key: {archive.name}")
        with archive.open("rb") as data:
            entries[archive.name] = f"sha256:{hashlib.file_digest(data, 'sha256').hexdigest()}"
    lines = [f"  {json.dumps(name)} => {json.dumps(entries[name])},\n" for name in sorted(entries)]
    output.write_text("%{\n" + "".join(lines) + "}\n", encoding="utf-8")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    normalize = commands.add_parser("normalize-windows")
    normalize.add_argument("archive", type=Path)
    normalize.add_argument("member")
    checksums = commands.add_parser("checksums")
    checksums.add_argument("output", type=Path)
    checksums.add_argument("archives", nargs="+", type=Path)
    args = parser.parse_args()
    if args.command == "normalize-windows":
        normalize_windows(args.archive, args.member)
    else:
        write_checksums(args.output, args.archives)


if __name__ == "__main__":
    main()

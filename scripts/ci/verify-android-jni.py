"""Validate Android JNI release inputs and the AAR's packaged libraries."""

import argparse
import shutil
import struct
import zipfile
from pathlib import Path

MACHINES = {"arm64-v8a": 183, "x86_64": 62}
LIBRARY = "libxberg_jni.so"


def verify_elf(header: bytes, abi: str, label: str) -> None:
    if len(header) != 64 or header[:7] != b"\x7fELF\x02\x01\x01":
        raise ValueError(f"{label}: expected a little-endian ELF64 header")
    kind, machine = struct.unpack_from("<HH", header, 16)
    if kind != 3 or machine != MACHINES[abi]:
        raise ValueError(
            f"{label}: expected {abi} shared library (ELF machine {MACHINES[abi]}), got {kind=}, {machine=}"
        )


def stage(source: Path, destination: Path) -> None:
    inputs = [(abi, source / abi / LIBRARY) for abi in MACHINES]
    for abi, path in inputs:
        with path.open("rb") as library:
            verify_elf(library.read(64), abi, str(path))
    for abi, path in inputs:
        target = destination / abi / LIBRARY
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(path, target)


def verify_aar(path: Path) -> None:
    with zipfile.ZipFile(path) as archive:
        for abi in MACHINES:
            name = f"jni/{abi}/{LIBRARY}"
            entries = [entry for entry in archive.infolist() if entry.filename == name]
            if len(entries) != 1:
                raise ValueError(f"{path}: expected exactly one {name}, found {len(entries)}")
            with archive.open(entries[0]) as library:
                verify_elf(library.read(64), abi, f"{path}:{name}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    subcommands = parser.add_subparsers(dest="command", required=True)
    staging = subcommands.add_parser("stage")
    staging.add_argument("source", type=Path)
    staging.add_argument("destination", type=Path)
    checking = subcommands.add_parser("verify-aar")
    checking.add_argument("aar", type=Path)
    args = parser.parse_args()
    try:
        if args.command == "stage":
            stage(args.source, args.destination)
        else:
            verify_aar(args.aar)
    except (OSError, ValueError, zipfile.BadZipFile) as error:
        parser.exit(1, f"Android JNI guard: {error}\n")


if __name__ == "__main__":
    main()
